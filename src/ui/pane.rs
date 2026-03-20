use crate::config::theme::Theme;
use crate::messages::*;
use crate::ui::workspace::PaneId;
use ratatui::Frame;
use ratatui::layout::Rect;

/// The different kinds of pane content.
#[derive(Debug, Clone)]
pub enum PaneKind {
    Feed(FeedPane),
    Thread(ThreadPane),
    Profile(ProfilePane),
    Dms(DmsPane),
    Notifications(NotificationsPane),
    Compose(ComposePane),
}

/// A pane with an ID and its content.
#[derive(Debug, Clone)]
pub struct Pane {
    pub id: PaneId,
    pub kind: PaneKind,
}

impl Pane {
    pub fn new_feed(id: PaneId, tabs: Vec<FeedTab>) -> Self {
        Self {
            id,
            kind: PaneKind::Feed(FeedPane {
                tabs,
                active_tab: 0,
            }),
        }
    }

    pub fn new_thread(id: PaneId) -> Self {
        Self {
            id,
            kind: PaneKind::Thread(ThreadPane {
                thread: None,
                cursor: 0,
                flattened: Vec::new(),
            }),
        }
    }

    pub fn new_profile(id: PaneId) -> Self {
        Self {
            id,
            kind: PaneKind::Profile(ProfilePane {
                profile: None,
                posts: Vec::new(),
                active_tab: ProfileTab::Posts,
                cursor: 0,
            }),
        }
    }

    pub fn new_dms(id: PaneId) -> Self {
        Self {
            id,
            kind: PaneKind::Dms(DmsPane {
                conversations: Vec::new(),
                active_convo: None,
                messages: Vec::new(),
                draft: String::new(),
                convo_cursor: 0,
                message_scroll: 0,
            }),
        }
    }

    pub fn new_notifications(id: PaneId) -> Self {
        Self {
            id,
            kind: PaneKind::Notifications(NotificationsPane {
                notifications: Vec::new(),
                cursor: 0,
                unread_count: 0,
            }),
        }
    }

    pub fn new_compose(id: PaneId, reply_to: Option<ReplyRef>) -> Self {
        Self {
            id,
            kind: PaneKind::Compose(ComposePane {
                text: String::new(),
                reply_to,
                cursor_pos: 0,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FeedPane {
    pub tabs: Vec<FeedTab>,
    pub active_tab: usize,
}

impl FeedPane {
    pub fn active_tab(&self) -> Option<&FeedTab> {
        self.tabs.get(self.active_tab)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut FeedTab> {
        self.tabs.get_mut(self.active_tab)
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = if self.active_tab == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab - 1
            };
        }
    }
}

/// A flattened thread entry for display.
#[derive(Debug, Clone)]
pub struct FlattenedThreadEntry {
    pub post: Post,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct ThreadPane {
    pub thread: Option<PostThread>,
    pub cursor: usize,
    pub flattened: Vec<FlattenedThreadEntry>,
}

impl ThreadPane {
    /// Flatten the thread tree into a displayable list.
    pub fn flatten_thread(&mut self) {
        self.flattened.clear();
        if let Some(thread) = self.thread.clone() {
            Self::flatten_into(&mut self.flattened, &thread, 0, true);
        }
    }

    fn flatten_into(
        out: &mut Vec<FlattenedThreadEntry>,
        thread: &PostThread,
        depth: usize,
        is_root: bool,
    ) {
        if is_root {
            if let Some(parent) = &thread.parent {
                Self::flatten_parent_chain(out, parent, 0);
            }
        }

        out.push(FlattenedThreadEntry {
            post: thread.post.clone(),
            depth,
        });

        for reply in &thread.replies {
            Self::flatten_into(out, reply, depth + 1, false);
        }
    }

    fn flatten_parent_chain(
        out: &mut Vec<FlattenedThreadEntry>,
        thread: &PostThread,
        depth: usize,
    ) {
        if let Some(parent) = &thread.parent {
            Self::flatten_parent_chain(out, parent, depth);
        }
        out.push(FlattenedThreadEntry {
            post: thread.post.clone(),
            depth,
        });
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProfileTab {
    Posts,
    Replies,
    Likes,
    Media,
}

#[derive(Debug, Clone)]
pub struct ProfilePane {
    pub profile: Option<ProfileData>,
    pub posts: Vec<Post>,
    pub active_tab: ProfileTab,
    pub cursor: usize,
}

#[derive(Debug, Clone)]
pub struct DmsPane {
    pub conversations: Vec<Conversation>,
    pub active_convo: Option<String>,
    pub messages: Vec<DirectMessage>,
    pub draft: String,
    pub convo_cursor: usize,
    pub message_scroll: usize,
}

#[derive(Debug, Clone)]
pub struct NotificationsPane {
    pub notifications: Vec<Notification>,
    pub cursor: usize,
    pub unread_count: usize,
}

#[derive(Debug, Clone)]
pub struct ComposePane {
    pub text: String,
    pub reply_to: Option<ReplyRef>,
    pub cursor_pos: usize,
}

impl ComposePane {
    pub fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            // Find the previous character boundary.
            let prev = self.text[..self.cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.text.remove(prev);
            self.cursor_pos = prev;
        }
    }

    pub fn delete(&mut self) {
        if self.cursor_pos < self.text.len() {
            self.text.remove(self.cursor_pos);
        }
    }

    pub fn grapheme_count(&self) -> usize {
        use unicode_segmentation::UnicodeSegmentation;
        self.text.graphemes(true).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feed_pane_tab_navigation() {
        let mut pane = FeedPane {
            tabs: vec![
                FeedTab::new("Tab 1", "uri1"),
                FeedTab::new("Tab 2", "uri2"),
                FeedTab::new("Tab 3", "uri3"),
            ],
            active_tab: 0,
        };

        pane.next_tab();
        assert_eq!(pane.active_tab, 1);
        pane.next_tab();
        assert_eq!(pane.active_tab, 2);
        pane.next_tab();
        assert_eq!(pane.active_tab, 0); // wraps

        pane.prev_tab();
        assert_eq!(pane.active_tab, 2); // wraps back
    }

    #[test]
    fn test_compose_insert_char() {
        let mut compose = ComposePane {
            text: String::new(),
            reply_to: None,
            cursor_pos: 0,
        };
        compose.insert_char('h');
        compose.insert_char('i');
        assert_eq!(compose.text, "hi");
        assert_eq!(compose.cursor_pos, 2);
    }

    #[test]
    fn test_compose_backspace() {
        let mut compose = ComposePane {
            text: "hello".into(),
            reply_to: None,
            cursor_pos: 5,
        };
        compose.backspace();
        assert_eq!(compose.text, "hell");
        assert_eq!(compose.cursor_pos, 4);
    }

    #[test]
    fn test_compose_backspace_at_start() {
        let mut compose = ComposePane {
            text: "hello".into(),
            reply_to: None,
            cursor_pos: 0,
        };
        compose.backspace(); // should do nothing
        assert_eq!(compose.text, "hello");
    }

    #[test]
    fn test_compose_delete() {
        let mut compose = ComposePane {
            text: "hello".into(),
            reply_to: None,
            cursor_pos: 0,
        };
        compose.delete();
        assert_eq!(compose.text, "ello");
    }

    #[test]
    fn test_compose_grapheme_count() {
        let mut compose = ComposePane {
            text: "hello".into(),
            reply_to: None,
            cursor_pos: 0,
        };
        assert_eq!(compose.grapheme_count(), 5);

        // Emoji is one grapheme cluster.
        compose.text = "hi 👋".into();
        assert_eq!(compose.grapheme_count(), 4);
    }

    #[test]
    fn test_thread_flatten_empty() {
        let mut pane = ThreadPane {
            thread: None,
            cursor: 0,
            flattened: Vec::new(),
        };
        pane.flatten_thread();
        assert!(pane.flattened.is_empty());
    }

    #[test]
    fn test_thread_flatten_single() {
        let post = Post {
            uri: "at://test".into(),
            cid: "cid1".into(),
            author: Author {
                did: "did:plc:test".into(),
                handle: "test.bsky.social".into(),
                display_name: Some("Test".into()),
                avatar_url: None,
            },
            text: "hello".into(),
            facets: vec![],
            created_at: "2024-01-01T00:00:00Z".into(),
            like_count: 0,
            repost_count: 0,
            reply_count: 0,
            liked_by_me: None,
            reposted_by_me: None,
            reply_to: None,
            embed: None,
            reposted_by: None,
        };
        let mut pane = ThreadPane {
            thread: Some(PostThread {
                post: post.clone(),
                parent: None,
                replies: vec![],
            }),
            cursor: 0,
            flattened: Vec::new(),
        };
        pane.flatten_thread();
        assert_eq!(pane.flattened.len(), 1);
        assert_eq!(pane.flattened[0].depth, 0);
    }
}
