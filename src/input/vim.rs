use crate::messages::UiAction;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// The three vim modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    Normal,
    Insert,
    Command,
}

/// State machine for vim modal input handling.
#[derive(Debug)]
pub struct VimState {
    pub mode: VimMode,
    /// Buffer for multi-key sequences like "gg".
    pending_key: Option<char>,
    /// Command line buffer when in command mode.
    pub command_buffer: String,
    /// Search query.
    pub search_query: String,
}

impl VimState {
    pub fn new() -> Self {
        Self {
            mode: VimMode::Normal,
            pending_key: None,
            command_buffer: String::new(),
            search_query: String::new(),
        }
    }

    /// Process a key event and return a UI action if applicable.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<UiAction> {
        match self.mode {
            VimMode::Normal => self.handle_normal(key),
            VimMode::Insert => self.handle_insert(key),
            VimMode::Command => self.handle_command(key),
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) -> Option<UiAction> {
        // Check for pending "g" key first.
        if self.pending_key == Some('g') {
            self.pending_key = None;
            return match key.code {
                KeyCode::Char('g') => Some(UiAction::GotoTop),
                _ => None,
            };
        }

        // Ctrl-key combinations.
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('d') => Some(UiAction::HalfPageDown),
                KeyCode::Char('u') => Some(UiAction::HalfPageUp),
                KeyCode::Char('w') => {
                    self.pending_key = Some('w');
                    None
                }
                _ => None,
            };
        }

        // Handle Ctrl-w sub-commands.
        if self.pending_key == Some('w') {
            self.pending_key = None;
            return match key.code {
                KeyCode::Char('h') => Some(UiAction::FocusPaneLeft),
                KeyCode::Char('j') => Some(UiAction::FocusPaneDown),
                KeyCode::Char('k') => Some(UiAction::FocusPaneUp),
                KeyCode::Char('l') => Some(UiAction::FocusPaneRight),
                KeyCode::Char('+') => Some(UiAction::ResizePaneGrow),
                KeyCode::Char('-') => Some(UiAction::ResizePaneShrink),
                KeyCode::Char('<') => Some(UiAction::ResizePaneNarrower),
                KeyCode::Char('>') => Some(UiAction::ResizePaneWider),
                KeyCode::Char('=') => Some(UiAction::EqualizePanes),
                KeyCode::Char('o') => Some(UiAction::ZoomPane),
                _ => None,
            };
        }

        // Shift combos.
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            return match key.code {
                KeyCode::Char('G') => Some(UiAction::GotoBottom),
                KeyCode::Char('H') => Some(UiAction::PrevFeedTab),
                KeyCode::Char('L') => Some(UiAction::NextFeedTab),
                KeyCode::Char('E') => Some(UiAction::ComposeInEditor),
                KeyCode::Char('N') => Some(UiAction::SearchPrev),
                _ => None,
            };
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => Some(UiAction::ScrollDown),
            KeyCode::Char('k') | KeyCode::Up => Some(UiAction::ScrollUp),
            KeyCode::Char('g') => {
                self.pending_key = Some('g');
                None
            }
            KeyCode::Char('l') | KeyCode::Enter | KeyCode::Right => Some(UiAction::OpenThread),
            KeyCode::Char('h') | KeyCode::Left => Some(UiAction::GoBack),
            KeyCode::Esc => Some(UiAction::GoBack),
            KeyCode::Char('f') => Some(UiAction::Like),
            KeyCode::Char('b') => Some(UiAction::Repost),
            KeyCode::Char('r') => {
                self.mode = VimMode::Insert;
                Some(UiAction::Reply)
            }
            KeyCode::Char('t') => {
                self.mode = VimMode::Insert;
                Some(UiAction::QuotePost)
            }
            KeyCode::Char('c') => {
                self.mode = VimMode::Insert;
                Some(UiAction::ComposeNew)
            }
            KeyCode::Char('p') => Some(UiAction::OpenProfile),
            KeyCode::Char('o') => Some(UiAction::OpenInBrowser),
            KeyCode::Char('/') => {
                self.mode = VimMode::Command;
                self.command_buffer = "/".to_string();
                Some(UiAction::SearchStart)
            }
            KeyCode::Char('n') => Some(UiAction::SearchNext),
            KeyCode::Char(':') => {
                self.mode = VimMode::Command;
                self.command_buffer.clear();
                Some(UiAction::EnterCommandMode)
            }
            KeyCode::Char('q') => Some(UiAction::Quit),
            KeyCode::Char('?') => Some(UiAction::ShowHelp),
            KeyCode::Tab => Some(UiAction::CyclePaneFocus),
            KeyCode::Char(c @ '1'..='9') => {
                Some(UiAction::SwitchWorkspace(c.to_digit(10).unwrap() as usize - 1))
            }
            _ => None,
        }
    }

    fn handle_insert(&mut self, key: KeyEvent) -> Option<UiAction> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('a') => Some(UiAction::AttachImage),
                KeyCode::Char('e') => Some(UiAction::SwitchToEditor),
                _ => None,
            };
        }

        match key.code {
            KeyCode::Esc => {
                self.mode = VimMode::Normal;
                Some(UiAction::CancelCompose)
            }
            KeyCode::Enter => Some(UiAction::SubmitPost),
            KeyCode::Char(c) => Some(UiAction::InsertChar(c)),
            KeyCode::Backspace => Some(UiAction::InsertBackspace),
            KeyCode::Delete => Some(UiAction::InsertDelete),
            KeyCode::Left => Some(UiAction::InsertMoveLeft),
            KeyCode::Right => Some(UiAction::InsertMoveRight),
            KeyCode::Home => Some(UiAction::InsertMoveHome),
            KeyCode::End => Some(UiAction::InsertMoveEnd),
            _ => None,
        }
    }

    fn handle_command(&mut self, key: KeyEvent) -> Option<UiAction> {
        match key.code {
            KeyCode::Esc => {
                self.mode = VimMode::Normal;
                self.command_buffer.clear();
                Some(UiAction::GoBack)
            }
            KeyCode::Enter => {
                let cmd = self.command_buffer.clone();
                self.command_buffer.clear();
                self.mode = VimMode::Normal;

                if let Some(stripped) = cmd.strip_prefix('/') {
                    self.search_query = stripped.to_string();
                    Some(UiAction::SearchNext)
                } else {
                    Some(UiAction::Command(cmd))
                }
            }
            KeyCode::Backspace => {
                self.command_buffer.pop();
                if self.command_buffer.is_empty() {
                    self.mode = VimMode::Normal;
                    Some(UiAction::GoBack)
                } else {
                    None
                }
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
                None
            }
            _ => None,
        }
    }

    /// Enter insert mode (called when reply/compose is triggered).
    pub fn enter_insert(&mut self) {
        self.mode = VimMode::Insert;
    }

    /// Return to normal mode.
    pub fn enter_normal(&mut self) {
        self.mode = VimMode::Normal;
        self.pending_key = None;
    }
}

/// Parse a command string into its parts.
pub fn parse_command(cmd: &str) -> Option<(&str, &str)> {
    let cmd = cmd.trim();
    if let Some(pos) = cmd.find(' ') {
        Some((&cmd[..pos], cmd[pos + 1..].trim()))
    } else {
        Some((cmd, ""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_shift(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_normal_mode_navigation() {
        let mut state = VimState::new();
        assert_eq!(state.handle_key(key(KeyCode::Char('j'))), Some(UiAction::ScrollDown));
        assert_eq!(state.handle_key(key(KeyCode::Char('k'))), Some(UiAction::ScrollUp));
    }

    #[test]
    fn test_gg_goto_top() {
        let mut state = VimState::new();
        assert_eq!(state.handle_key(key(KeyCode::Char('g'))), None);
        assert_eq!(state.handle_key(key(KeyCode::Char('g'))), Some(UiAction::GotoTop));
    }

    #[test]
    fn test_shift_g_goto_bottom() {
        let mut state = VimState::new();
        assert_eq!(
            state.handle_key(key_shift(KeyCode::Char('G'))),
            Some(UiAction::GotoBottom)
        );
    }

    #[test]
    fn test_ctrl_d_half_page() {
        let mut state = VimState::new();
        assert_eq!(
            state.handle_key(key_ctrl(KeyCode::Char('d'))),
            Some(UiAction::HalfPageDown)
        );
        assert_eq!(
            state.handle_key(key_ctrl(KeyCode::Char('u'))),
            Some(UiAction::HalfPageUp)
        );
    }

    #[test]
    fn test_like_repost() {
        let mut state = VimState::new();
        assert_eq!(state.handle_key(key(KeyCode::Char('f'))), Some(UiAction::Like));
        assert_eq!(state.handle_key(key(KeyCode::Char('b'))), Some(UiAction::Repost));
    }

    #[test]
    fn test_compose_enters_insert_mode() {
        let mut state = VimState::new();
        assert_eq!(state.handle_key(key(KeyCode::Char('c'))), Some(UiAction::ComposeNew));
        assert_eq!(state.mode, VimMode::Insert);
    }

    #[test]
    fn test_reply_enters_insert_mode() {
        let mut state = VimState::new();
        assert_eq!(state.handle_key(key(KeyCode::Char('r'))), Some(UiAction::Reply));
        assert_eq!(state.mode, VimMode::Insert);
    }

    #[test]
    fn test_insert_mode_esc_returns_normal() {
        let mut state = VimState::new();
        state.mode = VimMode::Insert;
        assert_eq!(state.handle_key(key(KeyCode::Esc)), Some(UiAction::CancelCompose));
        assert_eq!(state.mode, VimMode::Normal);
    }

    #[test]
    fn test_insert_mode_typing() {
        let mut state = VimState::new();
        state.mode = VimMode::Insert;
        assert_eq!(
            state.handle_key(key(KeyCode::Char('a'))),
            Some(UiAction::InsertChar('a'))
        );
        assert_eq!(
            state.handle_key(key(KeyCode::Backspace)),
            Some(UiAction::InsertBackspace)
        );
    }

    #[test]
    fn test_insert_mode_enter_submits() {
        let mut state = VimState::new();
        state.mode = VimMode::Insert;
        assert_eq!(
            state.handle_key(key(KeyCode::Enter)),
            Some(UiAction::SubmitPost)
        );
    }

    #[test]
    fn test_command_mode_enter() {
        let mut state = VimState::new();
        assert_eq!(
            state.handle_key(key(KeyCode::Char(':'))),
            Some(UiAction::EnterCommandMode)
        );
        assert_eq!(state.mode, VimMode::Command);
    }

    #[test]
    fn test_command_mode_type_and_submit() {
        let mut state = VimState::new();
        state.mode = VimMode::Command;
        state.handle_key(key(KeyCode::Char('q')));
        assert_eq!(state.command_buffer, "q");
        let result = state.handle_key(key(KeyCode::Enter));
        assert_eq!(result, Some(UiAction::Command("q".to_string())));
        assert_eq!(state.mode, VimMode::Normal);
    }

    #[test]
    fn test_command_mode_esc_cancels() {
        let mut state = VimState::new();
        state.mode = VimMode::Command;
        state.command_buffer = "some".to_string();
        assert_eq!(state.handle_key(key(KeyCode::Esc)), Some(UiAction::GoBack));
        assert_eq!(state.mode, VimMode::Normal);
        assert!(state.command_buffer.is_empty());
    }

    #[test]
    fn test_search_mode() {
        let mut state = VimState::new();
        assert_eq!(
            state.handle_key(key(KeyCode::Char('/'))),
            Some(UiAction::SearchStart)
        );
        assert_eq!(state.mode, VimMode::Command);
        assert_eq!(state.command_buffer, "/");

        state.handle_key(key(KeyCode::Char('t')));
        state.handle_key(key(KeyCode::Char('e')));
        state.handle_key(key(KeyCode::Char('s')));
        state.handle_key(key(KeyCode::Char('t')));
        assert_eq!(state.command_buffer, "/test");

        let result = state.handle_key(key(KeyCode::Enter));
        assert_eq!(result, Some(UiAction::SearchNext));
        assert_eq!(state.search_query, "test");
    }

    #[test]
    fn test_workspace_switch() {
        let mut state = VimState::new();
        assert_eq!(
            state.handle_key(key(KeyCode::Char('1'))),
            Some(UiAction::SwitchWorkspace(0))
        );
        assert_eq!(
            state.handle_key(key(KeyCode::Char('3'))),
            Some(UiAction::SwitchWorkspace(2))
        );
    }

    #[test]
    fn test_ctrl_w_pane_focus() {
        let mut state = VimState::new();
        state.handle_key(key_ctrl(KeyCode::Char('w')));
        assert_eq!(
            state.handle_key(key(KeyCode::Char('h'))),
            Some(UiAction::FocusPaneLeft)
        );

        state.handle_key(key_ctrl(KeyCode::Char('w')));
        assert_eq!(
            state.handle_key(key(KeyCode::Char('o'))),
            Some(UiAction::ZoomPane)
        );
    }

    #[test]
    fn test_parse_command() {
        assert_eq!(parse_command("q"), Some(("q", "")));
        assert_eq!(parse_command("split feed"), Some(("split", "feed")));
        assert_eq!(
            parse_command("theme catppuccin"),
            Some(("theme", "catppuccin"))
        );
        assert_eq!(
            parse_command("dm @alice.bsky.social"),
            Some(("dm", "@alice.bsky.social"))
        );
    }

    #[test]
    fn test_feed_tab_navigation() {
        let mut state = VimState::new();
        assert_eq!(
            state.handle_key(key_shift(KeyCode::Char('H'))),
            Some(UiAction::PrevFeedTab)
        );
        assert_eq!(
            state.handle_key(key_shift(KeyCode::Char('L'))),
            Some(UiAction::NextFeedTab)
        );
    }
}
