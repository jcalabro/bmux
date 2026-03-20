use crate::config::theme::Theme;
use crate::ui::pane::{ProfilePane, ProfileTab};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

/// Render a profile pane.
pub fn render_profile_pane(
    frame: &mut Frame,
    area: Rect,
    pane: &ProfilePane,
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

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(" Profile ", Style::default().fg(theme.fg)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(profile) = &pane.profile else {
        let paragraph = Paragraph::new(Line::from(Span::styled(
            "No profile loaded. Press p on a post to view profile.",
            Style::default().fg(theme.muted),
        )));
        frame.render_widget(paragraph, inner);
        return;
    };

    let chunks = Layout::vertical([Constraint::Length(8), Constraint::Min(0)]).split(inner);

    // Profile header.
    let display_name = profile
        .display_name
        .as_deref()
        .unwrap_or(&profile.handle);

    let mut header_lines = vec![
        Line::from(Span::styled(
            display_name,
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("@{}", profile.handle),
            Style::default().fg(theme.handle),
        )),
        Line::from(""),
    ];

    if let Some(desc) = &profile.description {
        header_lines.push(Line::from(Span::styled(desc, Style::default().fg(theme.fg))));
        header_lines.push(Line::from(""));
    }

    header_lines.push(Line::from(vec![
        Span::styled(
            format!("{} ", profile.followers_count),
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("followers  ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{} ", profile.follows_count),
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("following  ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{} ", profile.posts_count),
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("posts", Style::default().fg(theme.muted)),
    ]));

    // Follow/mute/block status.
    let mut status_spans = Vec::new();
    if profile.followed_by_me.is_some() {
        status_spans.push(Span::styled(
            " Following ",
            Style::default().fg(theme.bg).bg(theme.accent),
        ));
    }
    if profile.following_me {
        status_spans.push(Span::styled(
            " Follows you ",
            Style::default().fg(theme.muted),
        ));
    }
    if profile.muted {
        status_spans.push(Span::styled(
            " Muted ",
            Style::default().fg(theme.bg).bg(theme.warning),
        ));
    }
    if profile.blocked {
        status_spans.push(Span::styled(
            " Blocked ",
            Style::default().fg(theme.bg).bg(theme.error),
        ));
    }
    if !status_spans.is_empty() {
        header_lines.push(Line::from(status_spans));
    }

    let header = Paragraph::new(Text::from(header_lines)).wrap(Wrap { trim: false });
    frame.render_widget(header, chunks[0]);

    // Tab bar + posts area.
    if chunks[1].height < 2 {
        return;
    }

    let tab_names = vec!["Posts", "Replies", "Likes", "Media"];
    let active_idx = match pane.active_tab {
        ProfileTab::Posts => 0,
        ProfileTab::Replies => 1,
        ProfileTab::Likes => 2,
        ProfileTab::Media => 3,
    };

    let tabs_area = Rect::new(chunks[1].x, chunks[1].y, chunks[1].width, 1);
    let tab_names_owned: Vec<String> = tab_names.iter().map(|s| s.to_string()).collect();
    crate::ui::widgets::tab_bar::render_feed_tabs(frame, tabs_area, &tab_names_owned, active_idx, theme);
}
