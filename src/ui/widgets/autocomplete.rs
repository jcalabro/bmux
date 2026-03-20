use crate::config::theme::Theme;
use crate::messages::Author;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};

/// Autocomplete popup state.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AutocompleteState {
    pub query: String,
    pub suggestions: Vec<Author>,
    pub selected: usize,
    pub visible: bool,
}

#[allow(dead_code)]
impl AutocompleteState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            suggestions: Vec::new(),
            selected: 0,
            visible: false,
        }
    }

    pub fn show(&mut self, query: &str) {
        self.query = query.to_string();
        self.visible = true;
        self.selected = 0;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.suggestions.clear();
        self.query.clear();
    }

    pub fn next(&mut self) {
        if !self.suggestions.is_empty() {
            self.selected = (self.selected + 1) % self.suggestions.len();
        }
    }

    pub fn prev(&mut self) {
        if !self.suggestions.is_empty() {
            self.selected = if self.selected == 0 {
                self.suggestions.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn selected_suggestion(&self) -> Option<&Author> {
        self.suggestions.get(self.selected)
    }
}

/// Render the autocomplete popup.
#[allow(dead_code)]
pub fn render_autocomplete(
    frame: &mut Frame,
    area: Rect,
    state: &AutocompleteState,
    theme: &Theme,
) {
    if !state.visible || state.suggestions.is_empty() {
        return;
    }

    let max_items = 5.min(state.suggestions.len());
    let popup_height = max_items as u16 + 2; // +2 for borders
    let popup_width = 40.min(area.width);

    // Position popup above the cursor area.
    let popup_area = Rect::new(
        area.x,
        area.y.saturating_sub(popup_height),
        popup_width,
        popup_height,
    );

    let items: Vec<ListItem> = state
        .suggestions
        .iter()
        .take(max_items)
        .enumerate()
        .map(|(i, author)| {
            let display = format!(
                "@{} {}",
                author.handle,
                author.display_name.as_deref().unwrap_or("")
            );
            let style = if i == state.selected {
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            ListItem::new(Line::from(Span::styled(display, style)))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(" Mentions ");

    let list = List::new(items).block(block);
    frame.render_widget(list, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autocomplete_navigation() {
        let mut state = AutocompleteState::new();
        state.suggestions = vec![
            Author {
                did: "did:1".into(),
                handle: "alice".into(),
                display_name: None,
                avatar_url: None,
            },
            Author {
                did: "did:2".into(),
                handle: "bob".into(),
                display_name: None,
                avatar_url: None,
            },
        ];
        state.visible = true;

        assert_eq!(state.selected, 0);
        state.next();
        assert_eq!(state.selected, 1);
        state.next();
        assert_eq!(state.selected, 0); // wraps

        state.prev();
        assert_eq!(state.selected, 1); // wraps back
    }

    #[test]
    fn test_autocomplete_hide() {
        let mut state = AutocompleteState::new();
        state.visible = true;
        state.query = "test".into();
        state.hide();
        assert!(!state.visible);
        assert!(state.query.is_empty());
    }
}
