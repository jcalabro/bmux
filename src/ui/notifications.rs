use crate::config::theme::Theme;
use crate::messages::NotificationReason;
use crate::ui::pane::NotificationsPane;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

/// Render the notifications pane.
pub fn render_notifications_pane(
    frame: &mut Frame,
    area: Rect,
    pane: &NotificationsPane,
    theme: &Theme,
    is_focused: bool,
) {
    if area.height < 3 || area.width < 5 {
        return;
    }

    let border_style = if is_focused {
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.border)
    };

    let title = if pane.unread_count > 0 {
        format!(" Notifications ({}) ", pane.unread_count)
    } else {
        " Notifications ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(title, Style::default().fg(theme.fg)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if pane.notifications.is_empty() {
        let paragraph = Paragraph::new(Line::from(Span::styled(
            "No notifications",
            Style::default().fg(theme.muted),
        )));
        frame.render_widget(paragraph, inner);
        return;
    }

    let mut lines = Vec::new();

    for (i, notif) in pane.notifications.iter().enumerate() {
        let is_selected = i == pane.cursor;
        let display_name = notif
            .author
            .display_name
            .as_deref()
            .unwrap_or(&notif.author.handle);

        let (icon, color, action) = match notif.reason {
            NotificationReason::Like => ("♥", theme.like, "liked your post"),
            NotificationReason::Repost => ("↻", theme.repost, "reposted your post"),
            NotificationReason::Follow => ("👤", theme.accent, "followed you"),
            NotificationReason::Mention => ("@", theme.accent, "mentioned you"),
            NotificationReason::Reply => ("💬", theme.reply, "replied to your post"),
            NotificationReason::Quote => ("📝", theme.secondary, "quoted your post"),
        };

        let _bg_style = if is_selected {
            Style::default().bg(theme.border)
        } else {
            Style::default()
        };

        let unread_marker = if !notif.is_read { "● " } else { "  " };

        lines.push(Line::from(vec![
            Span::styled(
                unread_marker,
                Style::default().fg(theme.accent),
            ),
            Span::styled(
                format!("{} ", icon),
                Style::default().fg(color),
            ),
            Span::styled(
                display_name,
                Style::default()
                    .fg(theme.fg)
                    .add_modifier(if is_selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
            Span::styled(
                format!(" {}", action),
                Style::default().fg(theme.muted),
            ),
        ]));
    }

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}
