use ratatui::layout::Rect;

/// Unique identifier for a pane.
pub type PaneId = u64;

/// Split direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// Binary tree of pane splits.
#[derive(Debug, Clone)]
pub enum PaneTree {
    Split {
        direction: SplitDirection,
        ratio: f64,
        first: Box<PaneTree>,
        second: Box<PaneTree>,
    },
    Leaf {
        id: PaneId,
    },
}

impl PaneTree {
    /// Create a single leaf node.
    pub fn leaf(id: PaneId) -> Self {
        Self::Leaf { id }
    }

    /// Split a specific leaf into two panes.
    pub fn split_leaf(
        &mut self,
        target_id: PaneId,
        new_id: PaneId,
        direction: SplitDirection,
        ratio: f64,
    ) -> bool {
        match self {
            PaneTree::Leaf { id } if *id == target_id => {
                *self = PaneTree::Split {
                    direction,
                    ratio,
                    first: Box::new(PaneTree::leaf(target_id)),
                    second: Box::new(PaneTree::leaf(new_id)),
                };
                true
            }
            PaneTree::Split { first, second, .. } => {
                first.split_leaf(target_id, new_id, direction, ratio)
                    || second.split_leaf(target_id, new_id, direction, ratio)
            }
            _ => false,
        }
    }

    /// Close a leaf pane, returning the sibling tree.
    /// Returns true if the pane was found and removed.
    pub fn close_leaf(&mut self, target_id: PaneId) -> bool {
        match self {
            PaneTree::Split { first, second, .. } => {
                if let PaneTree::Leaf { id } = first.as_ref() {
                    if *id == target_id {
                        *self = *second.clone();
                        return true;
                    }
                }
                if let PaneTree::Leaf { id } = second.as_ref() {
                    if *id == target_id {
                        *self = *first.clone();
                        return true;
                    }
                }
                first.close_leaf(target_id) || second.close_leaf(target_id)
            }
            PaneTree::Leaf { .. } => false,
        }
    }

    /// Get all leaf IDs in the tree.
    pub fn leaf_ids(&self) -> Vec<PaneId> {
        match self {
            PaneTree::Leaf { id } => vec![*id],
            PaneTree::Split { first, second, .. } => {
                let mut ids = first.leaf_ids();
                ids.extend(second.leaf_ids());
                ids
            }
        }
    }

    /// Compute layout rectangles for all leaf panes given an area.
    pub fn layout(&self, area: Rect) -> Vec<(PaneId, Rect)> {
        match self {
            PaneTree::Leaf { id } => vec![(*id, area)],
            PaneTree::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_area, second_area) = split_rect(area, *direction, *ratio);
                let mut result = first.layout(first_area);
                result.extend(second.layout(second_area));
                result
            }
        }
    }

    /// Find the next leaf ID after the given one (for cycling focus).
    pub fn next_leaf(&self, current: PaneId) -> Option<PaneId> {
        let ids = self.leaf_ids();
        if ids.is_empty() {
            return None;
        }
        let pos = ids.iter().position(|id| *id == current);
        match pos {
            Some(p) => Some(ids[(p + 1) % ids.len()]),
            None => ids.first().copied(),
        }
    }

    /// Find the previous leaf ID before the given one.
    pub fn prev_leaf(&self, current: PaneId) -> Option<PaneId> {
        let ids = self.leaf_ids();
        if ids.is_empty() {
            return None;
        }
        let pos = ids.iter().position(|id| *id == current);
        match pos {
            Some(0) => ids.last().copied(),
            Some(p) => Some(ids[p - 1]),
            None => ids.first().copied(),
        }
    }

    /// Adjust the ratio of the split that contains target_id.
    pub fn adjust_ratio(&mut self, target_id: PaneId, delta: f64) -> bool {
        match self {
            PaneTree::Split {
                first,
                second,
                ratio,
                ..
            } => {
                let first_ids = first.leaf_ids();
                let second_ids = second.leaf_ids();
                if first_ids.contains(&target_id) || second_ids.contains(&target_id) {
                    // Check if the target is a direct child.
                    let is_direct = matches!(first.as_ref(), PaneTree::Leaf { id } if *id == target_id)
                        || matches!(second.as_ref(), PaneTree::Leaf { id } if *id == target_id);

                    if is_direct {
                        *ratio = (*ratio + delta).clamp(0.1, 0.9);
                        return true;
                    }
                }
                first.adjust_ratio(target_id, delta) || second.adjust_ratio(target_id, delta)
            }
            PaneTree::Leaf { .. } => false,
        }
    }

    /// Set all ratios to 0.5 (equalize).
    pub fn equalize(&mut self) {
        if let PaneTree::Split {
            ratio,
            first,
            second,
            ..
        } = self
        {
            *ratio = 0.5;
            first.equalize();
            second.equalize();
        }
    }

    /// Count the total number of leaves.
    pub fn leaf_count(&self) -> usize {
        match self {
            PaneTree::Leaf { .. } => 1,
            PaneTree::Split { first, second, .. } => first.leaf_count() + second.leaf_count(),
        }
    }
}

/// Split a rectangle into two parts.
fn split_rect(area: Rect, direction: SplitDirection, ratio: f64) -> (Rect, Rect) {
    match direction {
        SplitDirection::Vertical => {
            let first_width = (area.width as f64 * ratio) as u16;
            let second_width = area.width.saturating_sub(first_width);
            (
                Rect::new(area.x, area.y, first_width, area.height),
                Rect::new(area.x + first_width, area.y, second_width, area.height),
            )
        }
        SplitDirection::Horizontal => {
            let first_height = (area.height as f64 * ratio) as u16;
            let second_height = area.height.saturating_sub(first_height);
            (
                Rect::new(area.x, area.y, area.width, first_height),
                Rect::new(area.x, area.y + first_height, area.width, second_height),
            )
        }
    }
}

/// A workspace is a named pane layout.
#[derive(Debug, Clone)]
pub struct Workspace {
    pub name: String,
    pub pane_tree: PaneTree,
    pub focused_pane: PaneId,
    /// When a pane is zoomed, store the original tree.
    pub zoomed_tree: Option<PaneTree>,
}

impl Workspace {
    pub fn new(name: impl Into<String>, pane_tree: PaneTree, focused: PaneId) -> Self {
        Self {
            name: name.into(),
            pane_tree,
            focused_pane: focused,
            zoomed_tree: None,
        }
    }

    /// Toggle zoom on the focused pane.
    pub fn toggle_zoom(&mut self) {
        if let Some(original) = self.zoomed_tree.take() {
            self.pane_tree = original;
        } else {
            self.zoomed_tree = Some(self.pane_tree.clone());
            self.pane_tree = PaneTree::leaf(self.focused_pane);
        }
    }

    /// Cycle focus to the next pane.
    pub fn cycle_focus(&mut self) {
        if let Some(next) = self.pane_tree.next_leaf(self.focused_pane) {
            self.focused_pane = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_leaf() {
        let tree = PaneTree::leaf(1);
        assert_eq!(tree.leaf_ids(), vec![1]);
        assert_eq!(tree.leaf_count(), 1);
    }

    #[test]
    fn test_split_leaf() {
        let mut tree = PaneTree::leaf(1);
        assert!(tree.split_leaf(1, 2, SplitDirection::Vertical, 0.5));
        assert_eq!(tree.leaf_ids(), vec![1, 2]);
        assert_eq!(tree.leaf_count(), 2);
    }

    #[test]
    fn test_close_leaf() {
        let mut tree = PaneTree::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.5,
            first: Box::new(PaneTree::leaf(1)),
            second: Box::new(PaneTree::leaf(2)),
        };
        assert!(tree.close_leaf(2));
        assert_eq!(tree.leaf_ids(), vec![1]);
    }

    #[test]
    fn test_close_first_leaf() {
        let mut tree = PaneTree::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.5,
            first: Box::new(PaneTree::leaf(1)),
            second: Box::new(PaneTree::leaf(2)),
        };
        assert!(tree.close_leaf(1));
        assert_eq!(tree.leaf_ids(), vec![2]);
    }

    #[test]
    fn test_layout() {
        let tree = PaneTree::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.5,
            first: Box::new(PaneTree::leaf(1)),
            second: Box::new(PaneTree::leaf(2)),
        };
        let area = Rect::new(0, 0, 100, 50);
        let layouts = tree.layout(area);
        assert_eq!(layouts.len(), 2);
        assert_eq!(layouts[0].0, 1);
        assert_eq!(layouts[0].1, Rect::new(0, 0, 50, 50));
        assert_eq!(layouts[1].0, 2);
        assert_eq!(layouts[1].1, Rect::new(50, 0, 50, 50));
    }

    #[test]
    fn test_horizontal_layout() {
        let tree = PaneTree::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(PaneTree::leaf(1)),
            second: Box::new(PaneTree::leaf(2)),
        };
        let area = Rect::new(0, 0, 100, 50);
        let layouts = tree.layout(area);
        assert_eq!(layouts[0].1, Rect::new(0, 0, 100, 25));
        assert_eq!(layouts[1].1, Rect::new(0, 25, 100, 25));
    }

    #[test]
    fn test_next_leaf() {
        let tree = PaneTree::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.5,
            first: Box::new(PaneTree::leaf(1)),
            second: Box::new(PaneTree::leaf(2)),
        };
        assert_eq!(tree.next_leaf(1), Some(2));
        assert_eq!(tree.next_leaf(2), Some(1)); // wraps around
    }

    #[test]
    fn test_prev_leaf() {
        let tree = PaneTree::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.5,
            first: Box::new(PaneTree::leaf(1)),
            second: Box::new(PaneTree::leaf(2)),
        };
        assert_eq!(tree.prev_leaf(2), Some(1));
        assert_eq!(tree.prev_leaf(1), Some(2)); // wraps around
    }

    #[test]
    fn test_adjust_ratio() {
        let mut tree = PaneTree::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.5,
            first: Box::new(PaneTree::leaf(1)),
            second: Box::new(PaneTree::leaf(2)),
        };
        assert!(tree.adjust_ratio(1, 0.1));
        if let PaneTree::Split { ratio, .. } = &tree {
            assert!((ratio - 0.6).abs() < 0.001);
        }
    }

    #[test]
    fn test_ratio_clamped() {
        let mut tree = PaneTree::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.9,
            first: Box::new(PaneTree::leaf(1)),
            second: Box::new(PaneTree::leaf(2)),
        };
        tree.adjust_ratio(1, 0.5);
        if let PaneTree::Split { ratio, .. } = &tree {
            assert!(*ratio <= 0.9);
        }
    }

    #[test]
    fn test_equalize() {
        let mut tree = PaneTree::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.7,
            first: Box::new(PaneTree::Split {
                direction: SplitDirection::Horizontal,
                ratio: 0.8,
                first: Box::new(PaneTree::leaf(1)),
                second: Box::new(PaneTree::leaf(2)),
            }),
            second: Box::new(PaneTree::leaf(3)),
        };
        tree.equalize();
        if let PaneTree::Split { ratio, first, .. } = &tree {
            assert!((ratio - 0.5).abs() < 0.001);
            if let PaneTree::Split { ratio, .. } = first.as_ref() {
                assert!((ratio - 0.5).abs() < 0.001);
            }
        }
    }

    #[test]
    fn test_workspace_toggle_zoom() {
        let tree = PaneTree::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.5,
            first: Box::new(PaneTree::leaf(1)),
            second: Box::new(PaneTree::leaf(2)),
        };
        let mut ws = Workspace::new("test", tree, 1);
        assert_eq!(ws.pane_tree.leaf_count(), 2);

        ws.toggle_zoom();
        assert_eq!(ws.pane_tree.leaf_count(), 1);
        assert!(ws.zoomed_tree.is_some());

        ws.toggle_zoom();
        assert_eq!(ws.pane_tree.leaf_count(), 2);
        assert!(ws.zoomed_tree.is_none());
    }

    #[test]
    fn test_workspace_cycle_focus() {
        let tree = PaneTree::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.5,
            first: Box::new(PaneTree::leaf(1)),
            second: Box::new(PaneTree::leaf(2)),
        };
        let mut ws = Workspace::new("test", tree, 1);
        ws.cycle_focus();
        assert_eq!(ws.focused_pane, 2);
        ws.cycle_focus();
        assert_eq!(ws.focused_pane, 1);
    }

    #[test]
    fn test_nested_split() {
        let mut tree = PaneTree::leaf(1);
        tree.split_leaf(1, 2, SplitDirection::Vertical, 0.65);
        tree.split_leaf(2, 3, SplitDirection::Horizontal, 0.5);
        assert_eq!(tree.leaf_count(), 3);
        assert_eq!(tree.leaf_ids(), vec![1, 2, 3]);
    }
}
