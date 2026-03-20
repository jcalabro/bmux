use crate::config::theme::Theme;
use crate::config::AppConfig;
use crate::input::vim::{self, VimState};
#[cfg(test)]
use crate::input::vim::VimMode;
use crate::messages::*;
use crate::ui::pane::*;
use crate::ui::toast::ToastManager;
use crate::ui::workspace::{PaneId, PaneTree, SplitDirection, Workspace};
use ratatui::Frame;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// The central application state and actor.
pub struct App {
    pub workspaces: Vec<Workspace>,
    pub active_workspace: usize,
    pub panes: HashMap<PaneId, Pane>,
    pub vim: VimState,
    pub theme: Theme,
    pub themes: HashMap<String, Theme>,
    pub toast_manager: ToastManager,
    pub show_help: bool,
    pub user_handle: String,
    pub unread_notifs: usize,
    pub should_quit: bool,
    pub config: AppConfig,
    next_pane_id: PaneId,
    api_tx: mpsc::Sender<ApiRequest>,
}

impl App {
    pub fn new(config: AppConfig, api_tx: mpsc::Sender<ApiRequest>, user_handle: String) -> Self {
        let themes = crate::config::theme::load_themes(&config.themes);
        let theme = themes
            .get(&config.general.theme)
            .cloned()
            .unwrap_or_else(Theme::bluesky);

        // Build feed tabs from config.
        let feed_tabs: Vec<FeedTab> = config
            .feeds
            .tabs
            .iter()
            .map(|t| FeedTab::new(&t.name, &t.uri))
            .collect();

        let mut panes = HashMap::new();
        let mut next_id = 1u64;

        // Create default workspace: feed + thread split.
        let feed_pane = Pane::new_feed(next_id, feed_tabs);
        panes.insert(next_id, feed_pane);
        let feed_id = next_id;
        next_id += 1;

        let thread_pane = Pane::new_thread(next_id);
        panes.insert(next_id, thread_pane);
        let thread_id = next_id;
        next_id += 1;

        let home_tree = PaneTree::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.65,
            first: Box::new(PaneTree::leaf(feed_id)),
            second: Box::new(PaneTree::leaf(thread_id)),
        };
        let home_ws = Workspace::new("Home", home_tree, feed_id);

        // DMs workspace.
        let dm_pane = Pane::new_dms(next_id);
        panes.insert(next_id, dm_pane);
        let dm_id = next_id;
        next_id += 1;

        let dms_tree = PaneTree::leaf(dm_id);
        let dms_ws = Workspace::new("DMs", dms_tree, dm_id);

        // Notifications workspace.
        let notif_pane = Pane::new_notifications(next_id);
        panes.insert(next_id, notif_pane);
        let notif_id = next_id;
        next_id += 1;

        let notif_tree = PaneTree::leaf(notif_id);
        let notif_ws = Workspace::new("Notifs", notif_tree, notif_id);

        let workspaces = vec![home_ws, dms_ws, notif_ws];

        Self {
            workspaces,
            active_workspace: 0,
            panes,
            vim: VimState::new(),
            theme,
            themes,
            toast_manager: ToastManager::new(),
            show_help: false,
            user_handle,
            unread_notifs: 0,
            should_quit: false,
            config,
            next_pane_id: next_id,
            api_tx,
        }
    }

    fn alloc_pane_id(&mut self) -> PaneId {
        let id = self.next_pane_id;
        self.next_pane_id += 1;
        id
    }

    fn active_ws(&self) -> &Workspace {
        &self.workspaces[self.active_workspace]
    }

    fn active_ws_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active_workspace]
    }

    fn focused_pane_id(&self) -> PaneId {
        self.active_ws().focused_pane
    }

    fn focused_pane(&self) -> Option<&Pane> {
        self.panes.get(&self.focused_pane_id())
    }

    fn focused_pane_mut(&mut self) -> Option<&mut Pane> {
        let id = self.focused_pane_id();
        self.panes.get_mut(&id)
    }

    /// Handle a raw key event — feeds it through the vim state machine.
    pub fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) {
        if let Some(action) = self.vim.handle_key(key) {
            self.handle_ui_action(action);
        }
    }

    /// Handle a message from any source.
    pub fn handle_message(&mut self, msg: AppMessage) {
        match msg {
            AppMessage::Ui(action) => self.handle_ui_action(action),
            AppMessage::Api(response) => self.handle_api_response(*response),
            AppMessage::NotificationPoll(notifs, unread) => {
                self.unread_notifs = unread;
                // Update the notifications pane if one exists.
                for pane in self.panes.values_mut() {
                    if let PaneKind::Notifications(ref mut np) = pane.kind {
                        np.notifications = notifs.clone();
                        np.unread_count = unread;
                    }
                }
            }
            AppMessage::ImageReady { url, data: _ } => {
                // TODO: store decoded image data for rendering.
                tracing::debug!("Image ready: {}", url);
            }
            AppMessage::Toast(toast) => {
                self.toast_manager.push(toast);
            }
        }
    }

    fn handle_ui_action(&mut self, action: UiAction) {
        match action {
            UiAction::Quit => {
                self.should_quit = true;
            }
            UiAction::ShowHelp => {
                self.show_help = !self.show_help;
            }
            UiAction::Tick => {}
            UiAction::Resize(_, _) => {}

            // Navigation.
            UiAction::ScrollDown => self.scroll_down(),
            UiAction::ScrollUp => self.scroll_up(),
            UiAction::HalfPageDown => {
                for _ in 0..10 {
                    self.scroll_down();
                }
            }
            UiAction::HalfPageUp => {
                for _ in 0..10 {
                    self.scroll_up();
                }
            }
            UiAction::GotoTop => self.goto_top(),
            UiAction::GotoBottom => self.goto_bottom(),

            // Post actions.
            UiAction::OpenThread => self.open_thread(),
            UiAction::GoBack => {
                self.show_help = false;
            }
            UiAction::Like => self.toggle_like(),
            UiAction::Repost => self.toggle_repost(),
            UiAction::Reply => self.start_reply(),
            UiAction::ComposeNew => self.start_compose(),
            UiAction::ComposeInEditor => self.compose_in_editor(),
            UiAction::OpenProfile => self.open_profile(),
            UiAction::OpenInBrowser => self.open_in_browser(),

            // Search.
            UiAction::SearchStart => {}
            UiAction::SearchNext => self.search_next(),
            UiAction::SearchPrev => {}

            // Feed tabs.
            UiAction::PrevFeedTab => self.prev_feed_tab(),
            UiAction::NextFeedTab => self.next_feed_tab(),

            // Workspace.
            UiAction::SwitchWorkspace(idx) => {
                if idx < self.workspaces.len() {
                    self.active_workspace = idx;
                }
            }

            // Pane management.
            UiAction::CyclePaneFocus => {
                self.active_ws_mut().cycle_focus();
            }
            UiAction::FocusPaneLeft | UiAction::FocusPaneRight
            | UiAction::FocusPaneUp | UiAction::FocusPaneDown => {
                // Simplified: just cycle for now.
                self.active_ws_mut().cycle_focus();
            }
            UiAction::ResizePaneGrow | UiAction::ResizePaneShrink
            | UiAction::ResizePaneWider | UiAction::ResizePaneNarrower => {
                let delta = match action {
                    UiAction::ResizePaneGrow | UiAction::ResizePaneWider => 0.05,
                    _ => -0.05,
                };
                let id = self.focused_pane_id();
                self.active_ws_mut().pane_tree.adjust_ratio(id, delta);
            }
            UiAction::EqualizePanes => {
                self.active_ws_mut().pane_tree.equalize();
            }
            UiAction::ZoomPane => {
                self.active_ws_mut().toggle_zoom();
            }

            // Command mode.
            UiAction::EnterCommandMode => {}
            UiAction::Command(cmd) => self.handle_command(&cmd),

            // Compose/insert actions.
            UiAction::SubmitPost => self.submit_post(),
            UiAction::CancelCompose => self.cancel_compose(),
            UiAction::AttachImage => {
                self.toast_manager.push(Toast::info("Image attach not yet implemented"));
            }
            UiAction::SwitchToEditor => self.compose_in_editor(),

            // Text input.
            UiAction::InsertChar(c) => self.insert_char(c),
            UiAction::InsertBackspace => self.insert_backspace(),
            UiAction::InsertDelete => self.insert_delete(),
            UiAction::InsertNewline => self.insert_char('\n'),
            UiAction::InsertMoveLeft | UiAction::InsertMoveRight
            | UiAction::InsertMoveHome | UiAction::InsertMoveEnd => {
                // Cursor movement in compose — handled by ComposePane.
            }
        }
    }

    fn handle_api_response(&mut self, response: ApiResponse) {
        match response {
            ApiResponse::Timeline { posts, cursor } => {
                // Find the feed pane and update its active tab.
                for pane in self.panes.values_mut() {
                    if let PaneKind::Feed(ref mut fp) = pane.kind
                        && let Some(tab) = fp.active_tab_mut()
                        && tab.uri == "following"
                    {
                        if tab.posts.is_empty() {
                            tab.posts = posts.clone();
                        } else {
                            tab.posts.extend(posts.clone());
                        }
                        tab.cursor = cursor.clone();
                        tab.loading = false;
                    }
                }
            }
            ApiResponse::Thread { uri: _, thread } => {
                for pane in self.panes.values_mut() {
                    if let PaneKind::Thread(ref mut tp) = pane.kind {
                        tp.thread = Some(*thread.clone());
                        tp.cursor = 0;
                        tp.flatten_thread();
                    }
                }
            }
            ApiResponse::Profile(profile) => {
                for pane in self.panes.values_mut() {
                    if let PaneKind::Profile(ref mut pp) = pane.kind {
                        pp.profile = Some(profile.clone());
                    }
                }
            }
            ApiResponse::PostCreated { uri: _ } => {
                self.toast_manager.push(Toast::success("Post created!"));
                self.vim.enter_normal();
                // Remove compose pane if there is one.
                self.cancel_compose();
                // Refresh timeline.
                self.request_timeline_refresh();
            }
            ApiResponse::PostLiked { post_uri, like_uri } => {
                self.update_post_like(&post_uri, Some(like_uri));
            }
            ApiResponse::PostUnliked { post_uri } => {
                self.update_post_like(&post_uri, None);
            }
            ApiResponse::PostReposted { post_uri, repost_uri } => {
                self.update_post_repost(&post_uri, Some(repost_uri));
            }
            ApiResponse::PostUnreposted { post_uri } => {
                self.update_post_repost(&post_uri, None);
            }
            ApiResponse::Notifications { notifications, cursor: _, unread_count } => {
                self.unread_notifs = unread_count;
                for pane in self.panes.values_mut() {
                    if let PaneKind::Notifications(ref mut np) = pane.kind {
                        np.notifications = notifications.clone();
                        np.unread_count = unread_count;
                    }
                }
            }
            ApiResponse::Conversations { conversations, cursor: _ } => {
                for pane in self.panes.values_mut() {
                    if let PaneKind::Dms(ref mut dp) = pane.kind {
                        dp.conversations = conversations.clone();
                    }
                }
            }
            ApiResponse::MessageSent { convo_id: _ } => {
                self.toast_manager.push(Toast::success("Message sent"));
            }
            ApiResponse::SearchResults { query: _, posts, cursor } => {
                // Put results in the feed pane.
                for pane in self.panes.values_mut() {
                    if let PaneKind::Feed(ref mut fp) = pane.kind
                        && let Some(tab) = fp.active_tab_mut()
                    {
                        tab.posts = posts.clone();
                        tab.cursor = cursor.clone();
                        tab.selected = 0;
                        tab.scroll_offset = 0;
                    }
                }
            }
            ApiResponse::Error { request_description, error } => {
                self.toast_manager.push(Toast::error(format!(
                    "Error ({}): {}",
                    request_description, error
                )));
            }
            _ => {}
        }
    }

    fn handle_command(&mut self, cmd: &str) {
        let (command, args) = vim::parse_command(cmd).unwrap_or(("", ""));

        match command {
            "q" | "quit" => {
                self.should_quit = true;
            }
            "split" => {
                self.split_pane(SplitDirection::Horizontal, args);
            }
            "vsplit" => {
                self.split_pane(SplitDirection::Vertical, args);
            }
            "close" => {
                self.close_focused_pane();
            }
            "theme" => {
                if let Some(new_theme) = self.themes.get(args) {
                    self.theme = new_theme.clone();
                    self.toast_manager
                        .push(Toast::info(format!("Theme: {}", args)));
                } else {
                    self.toast_manager
                        .push(Toast::error(format!("Unknown theme: {}", args)));
                }
            }
            "workspace" => {
                if args.is_empty() {
                    return;
                }
                // Find existing workspace or create new one.
                if let Some(idx) = self.workspaces.iter().position(|w| w.name == args) {
                    self.active_workspace = idx;
                } else {
                    let id = self.alloc_pane_id();
                    let feed_tabs = self.config.feeds.tabs
                        .iter()
                        .map(|t| FeedTab::new(&t.name, &t.uri))
                        .collect();
                    let pane = Pane::new_feed(id, feed_tabs);
                    self.panes.insert(id, pane);
                    let tree = PaneTree::leaf(id);
                    let ws = Workspace::new(args, tree, id);
                    self.workspaces.push(ws);
                    self.active_workspace = self.workspaces.len() - 1;
                }
            }
            "follow" => {
                let handle = args.trim_start_matches('@');
                if !handle.is_empty() {
                    let _ = self.api_tx.try_send(ApiRequest::FollowUser {
                        did: handle.to_string(),
                    });
                }
            }
            "mute" => {
                let handle = args.trim_start_matches('@');
                if !handle.is_empty() {
                    let _ = self.api_tx.try_send(ApiRequest::MuteUser {
                        did: handle.to_string(),
                    });
                }
            }
            "dm" => {
                self.toast_manager
                    .push(Toast::info(format!("DM to {}", args)));
            }
            "feed" => {
                if !args.is_empty() {
                    // Open a custom feed in the current pane.
                    let _ = self.api_tx.try_send(ApiRequest::FetchFeed {
                        feed_uri: args.to_string(),
                        cursor: None,
                    });
                }
            }
            _ => {
                self.toast_manager
                    .push(Toast::error(format!("Unknown command: {}", command)));
            }
        }
    }

    // ── Navigation ──────────────────────────────────────────

    fn scroll_down(&mut self) {
        if let Some(pane) = self.focused_pane_mut() {
            match &mut pane.kind {
                PaneKind::Feed(fp) => {
                    if let Some(tab) = fp.active_tab_mut()
                        && tab.selected < tab.posts.len().saturating_sub(1)
                    {
                        tab.selected += 1;
                        // Auto-scroll if needed.
                        if tab.selected >= tab.scroll_offset + 5 {
                            tab.scroll_offset = tab.selected.saturating_sub(4);
                        }
                        // Auto-load more when near the end.
                        if tab.selected >= tab.posts.len().saturating_sub(3)
                            && !tab.loading
                            && tab.cursor.is_some()
                        {
                            tab.loading = true;
                            let cursor = tab.cursor.clone();
                            let _ = self.api_tx.try_send(ApiRequest::FetchTimeline { cursor });
                        }
                    }
                }
                PaneKind::Thread(tp) => {
                    if tp.cursor < tp.flattened.len().saturating_sub(1) {
                        tp.cursor += 1;
                    }
                }
                PaneKind::Notifications(np) => {
                    if np.cursor < np.notifications.len().saturating_sub(1) {
                        np.cursor += 1;
                    }
                }
                PaneKind::Dms(dp) => {
                    if dp.convo_cursor < dp.conversations.len().saturating_sub(1) {
                        dp.convo_cursor += 1;
                    }
                }
                PaneKind::Profile(pp) => {
                    if pp.cursor < pp.posts.len().saturating_sub(1) {
                        pp.cursor += 1;
                    }
                }
                _ => {}
            }
        }
    }

    fn scroll_up(&mut self) {
        if let Some(pane) = self.focused_pane_mut() {
            match &mut pane.kind {
                PaneKind::Feed(fp) => {
                    if let Some(tab) = fp.active_tab_mut() {
                        tab.selected = tab.selected.saturating_sub(1);
                        if tab.selected < tab.scroll_offset {
                            tab.scroll_offset = tab.selected;
                        }
                    }
                }
                PaneKind::Thread(tp) => {
                    tp.cursor = tp.cursor.saturating_sub(1);
                }
                PaneKind::Notifications(np) => {
                    np.cursor = np.cursor.saturating_sub(1);
                }
                PaneKind::Dms(dp) => {
                    dp.convo_cursor = dp.convo_cursor.saturating_sub(1);
                }
                PaneKind::Profile(pp) => {
                    pp.cursor = pp.cursor.saturating_sub(1);
                }
                _ => {}
            }
        }
    }

    fn goto_top(&mut self) {
        if let Some(pane) = self.focused_pane_mut() {
            match &mut pane.kind {
                PaneKind::Feed(fp) => {
                    if let Some(tab) = fp.active_tab_mut() {
                        tab.selected = 0;
                        tab.scroll_offset = 0;
                    }
                }
                PaneKind::Thread(tp) => tp.cursor = 0,
                PaneKind::Notifications(np) => np.cursor = 0,
                PaneKind::Dms(dp) => dp.convo_cursor = 0,
                _ => {}
            }
        }
    }

    fn goto_bottom(&mut self) {
        if let Some(pane) = self.focused_pane_mut() {
            match &mut pane.kind {
                PaneKind::Feed(fp) => {
                    if let Some(tab) = fp.active_tab_mut() {
                        tab.selected = tab.posts.len().saturating_sub(1);
                        tab.scroll_offset = tab.selected.saturating_sub(4);
                    }
                }
                PaneKind::Thread(tp) => {
                    tp.cursor = tp.flattened.len().saturating_sub(1);
                }
                PaneKind::Notifications(np) => {
                    np.cursor = np.notifications.len().saturating_sub(1);
                }
                PaneKind::Dms(dp) => {
                    dp.convo_cursor = dp.conversations.len().saturating_sub(1);
                }
                _ => {}
            }
        }
    }

    // ── Post actions ────────────────────────────────────────

    fn selected_post(&self) -> Option<&Post> {
        let pane = self.focused_pane()?;
        match &pane.kind {
            PaneKind::Feed(fp) => {
                let tab = fp.active_tab()?;
                tab.posts.get(tab.selected)
            }
            PaneKind::Thread(tp) => {
                tp.flattened.get(tp.cursor).map(|e| &e.post)
            }
            _ => None,
        }
    }

    fn open_thread(&mut self) {
        if let Some(post) = self.selected_post().cloned() {
            let _ = self.api_tx.try_send(ApiRequest::FetchThread {
                uri: post.uri.clone(),
            });
        }
    }

    fn toggle_like(&mut self) {
        if let Some(post) = self.selected_post().cloned() {
            if let Some(like_uri) = &post.liked_by_me {
                // Unlike — optimistic update.
                self.update_post_like(&post.uri, None);
                let _ = self.api_tx.try_send(ApiRequest::UnlikePost {
                    like_uri: like_uri.clone(),
                });
            } else {
                // Like — optimistic update.
                self.update_post_like(&post.uri, Some("pending".to_string()));
                let _ = self.api_tx.try_send(ApiRequest::LikePost {
                    uri: post.uri.clone(),
                    cid: post.cid.clone(),
                });
            }
        }
    }

    fn toggle_repost(&mut self) {
        if let Some(post) = self.selected_post().cloned() {
            if let Some(repost_uri) = &post.reposted_by_me {
                self.update_post_repost(&post.uri, None);
                let _ = self.api_tx.try_send(ApiRequest::UnrepostPost {
                    repost_uri: repost_uri.clone(),
                });
            } else {
                self.update_post_repost(&post.uri, Some("pending".to_string()));
                let _ = self.api_tx.try_send(ApiRequest::RepostPost {
                    uri: post.uri.clone(),
                    cid: post.cid.clone(),
                });
            }
        }
    }

    fn update_post_like(&mut self, post_uri: &str, like_uri: Option<String>) {
        for pane in self.panes.values_mut() {
            if let PaneKind::Feed(fp) = &mut pane.kind {
                for tab in &mut fp.tabs {
                    for post in &mut tab.posts {
                        if post.uri == post_uri {
                            if like_uri.is_some() && post.liked_by_me.is_none() {
                                post.like_count += 1;
                            } else if like_uri.is_none() && post.liked_by_me.is_some() {
                                post.like_count = post.like_count.saturating_sub(1);
                            }
                            post.liked_by_me = like_uri.clone();
                        }
                    }
                }
            }
        }
    }

    fn update_post_repost(&mut self, post_uri: &str, repost_uri: Option<String>) {
        for pane in self.panes.values_mut() {
            if let PaneKind::Feed(fp) = &mut pane.kind {
                for tab in &mut fp.tabs {
                    for post in &mut tab.posts {
                        if post.uri == post_uri {
                            if repost_uri.is_some() && post.reposted_by_me.is_none() {
                                post.repost_count += 1;
                            } else if repost_uri.is_none() && post.reposted_by_me.is_some() {
                                post.repost_count = post.repost_count.saturating_sub(1);
                            }
                            post.reposted_by_me = repost_uri.clone();
                        }
                    }
                }
            }
        }
    }

    fn open_profile(&mut self) {
        if let Some(post) = self.selected_post().cloned() {
            let _ = self.api_tx.try_send(ApiRequest::FetchProfile {
                actor: post.author.handle.clone(),
            });
        }
    }

    fn open_in_browser(&self) {
        if let Some(post) = self.selected_post() {
            // Convert AT URI to web URL.
            let parts: Vec<&str> = post.uri.split('/').collect();
            if parts.len() >= 5 {
                let handle = &post.author.handle;
                let rkey = parts.last().unwrap_or(&"");
                let url = format!("https://bsky.app/profile/{}/post/{}", handle, rkey);
                let _ = open::that(&url);
            }
        }
    }

    // ── Compose ─────────────────────────────────────────────

    fn start_compose(&mut self) {
        let id = self.alloc_pane_id();
        let pane = Pane::new_compose(id, None);
        self.panes.insert(id, pane);

        let focused_id = self.focused_pane_id();
        self.active_ws_mut()
            .pane_tree
            .split_leaf(focused_id, id, SplitDirection::Horizontal, 0.7);
        self.active_ws_mut().focused_pane = id;
        self.vim.enter_insert();
    }

    fn start_reply(&mut self) {
        if let Some(post) = self.selected_post().cloned() {
            let reply_ref = ReplyRef {
                root_uri: post.uri.clone(),
                root_cid: post.cid.clone(),
                parent_uri: post.uri.clone(),
                parent_cid: post.cid.clone(),
            };

            let id = self.alloc_pane_id();
            let pane = Pane::new_compose(id, Some(reply_ref));
            self.panes.insert(id, pane);

            let focused_id = self.focused_pane_id();
            self.active_ws_mut()
                .pane_tree
                .split_leaf(focused_id, id, SplitDirection::Horizontal, 0.7);
            self.active_ws_mut().focused_pane = id;
            self.vim.enter_insert();
        }
    }

    fn submit_post(&mut self) {
        let id = self.focused_pane_id();
        if let Some(pane) = self.panes.get(&id)
            && let PaneKind::Compose(cp) = &pane.kind
        {
            if cp.text.trim().is_empty() {
                self.toast_manager.push(Toast::error("Post is empty"));
                return;
            }
            if cp.grapheme_count() > 300 {
                self.toast_manager.push(Toast::error("Post exceeds 300 character limit"));
                return;
            }
            let _ = self.api_tx.try_send(ApiRequest::CreatePost {
                text: cp.text.clone(),
                reply_to: cp.reply_to.clone(),
            });
        }
    }

    fn cancel_compose(&mut self) {
        let id = self.focused_pane_id();
        if let Some(pane) = self.panes.get(&id)
            && matches!(pane.kind, PaneKind::Compose(_))
        {
            self.close_focused_pane();
        }
        self.vim.enter_normal();
    }

    fn compose_in_editor(&mut self) {
        let editor = self
            .config
            .general
            .editor
            .clone()
            .or_else(|| std::env::var("EDITOR").ok())
            .unwrap_or_else(|| "vi".to_string());

        // Create a temp file, open the editor, read the result.
        let tmpdir = std::env::temp_dir();
        let tmpfile = tmpdir.join("alf_compose.txt");

        // If we're in a compose pane, write the current draft.
        if let Some(pane) = self.focused_pane()
            && let PaneKind::Compose(cp) = &pane.kind
        {
            let _ = std::fs::write(&tmpfile, &cp.text);
        }

        // Note: This blocks the event loop. In a real implementation,
        // we'd suspend the TUI, run the editor, and restore.
        // For now, this is a placeholder showing the intent.
        self.toast_manager.push(Toast::info(format!(
            "$EDITOR compose: would open {}",
            editor
        )));
    }

    fn insert_char(&mut self, c: char) {
        let id = self.focused_pane_id();
        if let Some(pane) = self.panes.get_mut(&id) {
            match &mut pane.kind {
                PaneKind::Compose(cp) => cp.insert_char(c),
                PaneKind::Dms(dp) => dp.draft.push(c),
                _ => {}
            }
        }
    }

    fn insert_backspace(&mut self) {
        let id = self.focused_pane_id();
        if let Some(pane) = self.panes.get_mut(&id) {
            match &mut pane.kind {
                PaneKind::Compose(cp) => cp.backspace(),
                PaneKind::Dms(dp) => { dp.draft.pop(); }
                _ => {}
            }
        }
    }

    fn insert_delete(&mut self) {
        let id = self.focused_pane_id();
        if let Some(pane) = self.panes.get_mut(&id)
            && let PaneKind::Compose(cp) = &mut pane.kind
        {
            cp.delete();
        }
    }

    // ── Feed tabs ───────────────────────────────────────────

    fn next_feed_tab(&mut self) {
        let id = self.focused_pane_id();
        if let Some(pane) = self.panes.get_mut(&id)
            && let PaneKind::Feed(fp) = &mut pane.kind
        {
            fp.next_tab();
            self.request_feed_data();
        }
    }

    fn prev_feed_tab(&mut self) {
        let id = self.focused_pane_id();
        if let Some(pane) = self.panes.get_mut(&id)
            && let PaneKind::Feed(fp) = &mut pane.kind
        {
            fp.prev_tab();
            self.request_feed_data();
        }
    }

    // ── Search ──────────────────────────────────────────────

    fn search_next(&mut self) {
        let query = self.vim.search_query.clone();
        if !query.is_empty() {
            let _ = self.api_tx.try_send(ApiRequest::SearchPosts {
                query,
                cursor: None,
            });
        }
    }

    // ── Pane management ─────────────────────────────────────

    fn split_pane(&mut self, direction: SplitDirection, pane_type: &str) {
        let id = self.alloc_pane_id();
        let feed_tabs = self.config.feeds.tabs
            .iter()
            .map(|t| FeedTab::new(&t.name, &t.uri))
            .collect();

        let pane = match pane_type.trim() {
            "feed" | "" => Pane::new_feed(id, feed_tabs),
            "thread" => Pane::new_thread(id),
            "dms" => Pane::new_dms(id),
            "notifs" | "notifications" => Pane::new_notifications(id),
            s if s.starts_with("profile") => {
                Pane::new_profile(id)
            }
            _ => Pane::new_feed(id, feed_tabs),
        };

        self.panes.insert(id, pane);
        let focused = self.focused_pane_id();
        self.active_ws_mut()
            .pane_tree
            .split_leaf(focused, id, direction, 0.5);
    }

    fn close_focused_pane(&mut self) {
        let id = self.focused_pane_id();
        let ws = self.active_ws_mut();

        // Don't close the last pane.
        if ws.pane_tree.leaf_count() <= 1 {
            return;
        }

        // Move focus before closing.
        if let Some(next) = ws.pane_tree.next_leaf(id) {
            ws.focused_pane = next;
        }

        ws.pane_tree.close_leaf(id);
        self.panes.remove(&id);
    }

    // ── Data requests ───────────────────────────────────────

    pub fn request_initial_data(&self) {
        // Fetch the home timeline.
        let _ = self.api_tx.try_send(ApiRequest::FetchTimeline { cursor: None });
        // Fetch notifications.
        let _ = self.api_tx.try_send(ApiRequest::FetchNotifications { cursor: None });
        // Fetch conversations.
        let _ = self.api_tx.try_send(ApiRequest::FetchConversations { cursor: None });
    }

    fn request_timeline_refresh(&self) {
        let _ = self.api_tx.try_send(ApiRequest::FetchTimeline { cursor: None });
    }

    fn request_feed_data(&self) {
        // Check which feed tab is active and request its data.
        let id = self.focused_pane_id();
        if let Some(pane) = self.panes.get(&id)
            && let PaneKind::Feed(fp) = &pane.kind
            && let Some(tab) = fp.active_tab()
            && tab.posts.is_empty()
        {
            match tab.uri.as_str() {
                "following" => {
                    let _ = self.api_tx.try_send(ApiRequest::FetchTimeline { cursor: None });
                }
                "discover" => {
                    // Discover would use a different feed.
                    let _ = self.api_tx.try_send(ApiRequest::FetchTimeline { cursor: None });
                }
                uri => {
                    let _ = self.api_tx.try_send(ApiRequest::FetchFeed {
                        feed_uri: uri.to_string(),
                        cursor: None,
                    });
                }
            }
        }
    }

    /// Advance timers (called on tick).
    pub fn tick(&mut self) {
        self.toast_manager.tick();
    }

    /// Render the entire UI.
    pub fn render(&self, frame: &mut Frame) {
        crate::ui::render(
            frame,
            &self.workspaces,
            self.active_workspace,
            &self.panes,
            self.vim.mode,
            &self.vim.command_buffer,
            &self.theme,
            &self.toast_manager,
            self.show_help,
            &self.user_handle,
            self.unread_notifs,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let config = AppConfig::default();
        let (tx, _rx) = mpsc::channel(100);
        App::new(config, tx, "test.bsky.social".to_string())
    }

    #[test]
    fn test_app_creation() {
        let app = test_app();
        assert_eq!(app.workspaces.len(), 3);
        assert_eq!(app.active_workspace, 0);
        assert!(!app.should_quit);
        assert_eq!(app.vim.mode, VimMode::Normal);
    }

    #[test]
    fn test_workspace_switching() {
        let mut app = test_app();
        app.handle_ui_action(UiAction::SwitchWorkspace(1));
        assert_eq!(app.active_workspace, 1);
        app.handle_ui_action(UiAction::SwitchWorkspace(2));
        assert_eq!(app.active_workspace, 2);
        // Out of bounds should do nothing.
        app.handle_ui_action(UiAction::SwitchWorkspace(99));
        assert_eq!(app.active_workspace, 2);
    }

    #[test]
    fn test_quit() {
        let mut app = test_app();
        assert!(!app.should_quit);
        app.handle_ui_action(UiAction::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn test_help_toggle() {
        let mut app = test_app();
        assert!(!app.show_help);
        app.handle_ui_action(UiAction::ShowHelp);
        assert!(app.show_help);
        app.handle_ui_action(UiAction::ShowHelp);
        assert!(!app.show_help);
    }

    #[test]
    fn test_theme_command() {
        let mut app = test_app();
        app.handle_command("theme hacker");
        // Theme should have changed.
        assert_eq!(app.theme.bg, Theme::hacker().bg);
    }

    #[test]
    fn test_theme_command_invalid() {
        let mut app = test_app();
        app.handle_command("theme nonexistent");
        // Should have an error toast.
        assert!(!app.toast_manager.toasts.is_empty());
    }

    #[test]
    fn test_split_command() {
        let mut app = test_app();
        let initial_count = app.panes.len();
        app.handle_command("vsplit feed");
        assert_eq!(app.panes.len(), initial_count + 1);
    }

    #[test]
    fn test_close_last_pane_prevented() {
        let mut app = test_app();
        // Switch to DMs workspace which has only one pane.
        app.active_workspace = 1;
        let initial_count = app.active_ws().pane_tree.leaf_count();
        app.close_focused_pane();
        // Should not close the last pane.
        assert_eq!(app.active_ws().pane_tree.leaf_count(), initial_count);
    }

    #[test]
    fn test_equalize_panes() {
        let mut app = test_app();
        app.handle_ui_action(UiAction::EqualizePanes);
        // Should not panic.
    }

    #[test]
    fn test_api_response_timeline() {
        let mut app = test_app();
        let posts = vec![Post {
            uri: "at://test/post/1".into(),
            cid: "cid1".into(),
            author: Author {
                did: "did:plc:test".into(),
                handle: "test.bsky.social".into(),
                display_name: None,
                avatar_url: None,
            },
            text: "hello world".into(),
            facets: vec![],
            created_at: "2024-01-01T00:00:00Z".into(),
            like_count: 5,
            repost_count: 2,
            reply_count: 1,
            liked_by_me: None,
            reposted_by_me: None,
            reply_to: None,
            embed: None,
            reposted_by: None,
        }];

        app.handle_api_response(ApiResponse::Timeline {
            posts,
            cursor: Some("next".into()),
        });

        // Check that the feed pane got the posts.
        let has_posts = app.panes.values().any(|p| {
            if let PaneKind::Feed(fp) = &p.kind {
                fp.tabs.iter().any(|t| !t.posts.is_empty())
            } else {
                false
            }
        });
        assert!(has_posts);
    }

    #[test]
    fn test_optimistic_like() {
        let mut app = test_app();

        // Add a post to the feed.
        for pane in app.panes.values_mut() {
            if let PaneKind::Feed(fp) = &mut pane.kind {
                if let Some(tab) = fp.active_tab_mut() {
                    tab.posts.push(Post {
                        uri: "at://test/post/1".into(),
                        cid: "cid1".into(),
                        author: Author {
                            did: "did:plc:test".into(),
                            handle: "test".into(),
                            display_name: None,
                            avatar_url: None,
                        },
                        text: "test".into(),
                        facets: vec![],
                        created_at: "2024-01-01T00:00:00Z".into(),
                        like_count: 5,
                        repost_count: 0,
                        reply_count: 0,
                        liked_by_me: None,
                        reposted_by_me: None,
                        reply_to: None,
                        embed: None,
                        reposted_by: None,
                    });
                }
            }
        }

        // Like it.
        app.update_post_like("at://test/post/1", Some("like_uri".into()));

        // Verify optimistic update.
        for pane in app.panes.values() {
            if let PaneKind::Feed(fp) = &pane.kind {
                for tab in &fp.tabs {
                    for post in &tab.posts {
                        if post.uri == "at://test/post/1" {
                            assert_eq!(post.like_count, 6);
                            assert!(post.liked_by_me.is_some());
                        }
                    }
                }
            }
        }
    }
}
