pub mod compose;
pub mod dms;
pub mod feed;
pub mod help;
pub mod notifications;
pub mod pane;
pub mod profile;
pub mod thread;
pub mod toast;
pub mod widgets;
pub mod workspace;

use crate::config::theme::Theme;
use crate::input::vim::VimMode;
use crate::ui::pane::{Pane, PaneKind};
use crate::ui::toast::ToastManager;
use crate::ui::workspace::{PaneId, Workspace};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::Frame;
use ratatui_image::protocol::StatefulProtocol;
use std::collections::HashMap;

/// Render the entire application UI.
#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut Frame,
    workspaces: &[Workspace],
    active_workspace: usize,
    panes: &HashMap<PaneId, Pane>,
    vim_mode: VimMode,
    command_buffer: &str,
    theme: &Theme,
    toast_manager: &ToastManager,
    show_help: bool,
    user_handle: &str,
    unread_notifs: usize,
    image_protos: &mut HashMap<String, StatefulProtocol>,
) {
    let area = frame.area();
    if area.height < 3 || area.width < 10 {
        return;
    }

    // Layout: top bar (1) + main area + status bar (1).
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    // Top bar — workspace tabs.
    let workspace_names: Vec<String> = workspaces.iter().map(|w| w.name.clone()).collect();
    widgets::tab_bar::render_workspace_tabs(
        frame,
        chunks[0],
        &workspace_names,
        active_workspace,
        theme,
        user_handle,
        unread_notifs,
    );

    // Main area — render panes from the active workspace.
    if let Some(ws) = workspaces.get(active_workspace) {
        let pane_layouts = ws.pane_tree.layout(chunks[1]);

        for (pane_id, pane_area) in &pane_layouts {
            if let Some(pane) = panes.get(pane_id) {
                let is_focused = *pane_id == ws.focused_pane;
                render_pane(frame, *pane_area, pane, theme, is_focused, image_protos);
            }
        }
    }

    // Status bar.
    let focused_pane_type = workspaces
        .get(active_workspace)
        .and_then(|ws| panes.get(&ws.focused_pane))
        .map(|p| match &p.kind {
            PaneKind::Feed(_) => "feed",
            PaneKind::Thread(_) => "thread",
            PaneKind::Profile(_) => "profile",
            PaneKind::Dms(_) => "dms",
            PaneKind::Notifications(_) => "notifications",
            PaneKind::Compose(_) => "compose",
        })
        .unwrap_or("unknown");

    widgets::status_bar::render_status_bar(
        frame,
        chunks[2],
        vim_mode,
        command_buffer,
        theme,
        focused_pane_type,
    );

    // Toast overlay.
    toast_manager.render(frame, area, theme);

    // Help overlay.
    if show_help {
        help::render_help(frame, area, theme);
    }
}

/// Render a single pane based on its kind.
fn render_pane(
    frame: &mut Frame,
    area: Rect,
    pane: &Pane,
    theme: &Theme,
    is_focused: bool,
    image_protos: &mut HashMap<String, StatefulProtocol>,
) {
    match &pane.kind {
        PaneKind::Feed(feed_pane) => {
            feed::render_feed_pane(frame, area, feed_pane, theme, is_focused, image_protos);
        }
        PaneKind::Thread(thread_pane) => {
            thread::render_thread_pane(frame, area, thread_pane, theme, is_focused);
        }
        PaneKind::Profile(profile_pane) => {
            profile::render_profile_pane(frame, area, profile_pane, theme, is_focused);
        }
        PaneKind::Dms(dms_pane) => {
            dms::render_dms_pane(frame, area, dms_pane, theme, is_focused);
        }
        PaneKind::Notifications(notifs_pane) => {
            notifications::render_notifications_pane(frame, area, notifs_pane, theme, is_focused);
        }
        PaneKind::Compose(compose_pane) => {
            compose::render_compose_pane(frame, area, compose_pane, theme, is_focused);
        }
    }
}
