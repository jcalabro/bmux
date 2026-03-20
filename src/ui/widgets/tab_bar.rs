use crate::config::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Render the top workspace tab bar.
pub fn render_workspace_tabs(
    frame: &mut Frame,
    area: Rect,
    workspace_names: &[String],
    active: usize,
    theme: &Theme,
    user_handle: &str,
    unread_notifs: usize,
) {
    if area.height == 0 {
        return;
    }

    let mut spans = Vec::new();
    spans.push(Span::styled(" ", Style::default().bg(theme.bg)));

    for (i, name) in workspace_names.iter().enumerate() {
        if i == active {
            spans.push(Span::styled(
                format!(" {} ", name),
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", name),
                Style::default().fg(theme.muted),
            ));
        }
        spans.push(Span::styled(" ", Style::default()));
    }

    // Right-aligned info: app name, user handle, notification badge.
    let right_info = if unread_notifs > 0 {
        format!("bmux │ @{} │ {} ", user_handle, unread_notifs)
    } else {
        format!("bmux │ @{} ", user_handle)
    };

    // Calculate padding.
    let left_width: usize = spans.iter().map(|s| s.content.len()).sum();
    let right_width = right_info.len();
    let padding = if area.width as usize > left_width + right_width {
        area.width as usize - left_width - right_width
    } else {
        1
    };

    spans.push(Span::styled(
        " ".repeat(padding),
        Style::default().bg(theme.bg),
    ));
    spans.push(Span::styled(
        right_info,
        Style::default().fg(theme.muted),
    ));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.bg));
    frame.render_widget(paragraph, area);
}

/// Render feed sub-tabs within a feed pane.
pub fn render_feed_tabs(
    frame: &mut Frame,
    area: Rect,
    tab_names: &[String],
    active: usize,
    theme: &Theme,
) {
    if area.height == 0 {
        return;
    }

    let mut spans = Vec::new();
    spans.push(Span::styled(" ", Style::default()));

    for (i, name) in tab_names.iter().enumerate() {
        if i == active {
            spans.push(Span::styled(
                format!(" {} ", name),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", name),
                Style::default().fg(theme.muted),
            ));
        }
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}
