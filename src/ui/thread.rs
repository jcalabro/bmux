use crate::config::theme::Theme;
use crate::ui::pane::ThreadPane;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Render a thread pane.
pub fn render_thread_pane(
    frame: &mut Frame,
    area: Rect,
    pane: &ThreadPane,
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
        .title(Span::styled(" Thread ", Style::default().fg(theme.fg)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if pane.flattened.is_empty() {
        let msg = if pane.thread.is_some() {
            "Empty thread."
        } else {
            "Select a post and press l to view thread."
        };
        let paragraph = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(theme.muted),
        )));
        frame.render_widget(paragraph, inner);
        return;
    }

    let mut lines = Vec::new();

    for (i, entry) in pane.flattened.iter().enumerate() {
        let indent = "  ".repeat(entry.depth);
        let is_selected = i == pane.cursor;

        let display_name = entry
            .post
            .author
            .display_name
            .clone()
            .unwrap_or_else(|| entry.post.author.handle.clone());

        // Selected posts get a background highlight and selection indicator.
        let sel_bg = if is_selected {
            Style::default().bg(theme.secondary)
        } else {
            Style::default()
        };

        // Author line.
        let author_style = if is_selected {
            Style::default()
                .fg(theme.accent)
                .bg(theme.secondary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.handle)
        };

        let connector = if entry.depth > 0 { "├─ " } else { "" };
        let indicator = if is_selected { "▸ " } else { "  " };

        lines.push(Line::from(vec![
            Span::styled(indicator, sel_bg.fg(theme.accent)),
            Span::styled(
                format!("{}{}", indent, connector),
                sel_bg.fg(theme.border),
            ),
            Span::styled(display_name, author_style),
            Span::styled(
                format!(" @{}", entry.post.author.handle),
                sel_bg.fg(theme.muted),
            ),
        ]));

        // Text line.
        let text_indent = if entry.depth > 0 {
            format!("{}│  ", indent)
        } else {
            indent.clone()
        };

        let text_style = if is_selected {
            sel_bg.fg(theme.fg)
        } else {
            Style::default().fg(theme.fg)
        };

        lines.push(Line::from(vec![
            Span::styled("  ", sel_bg),
            Span::styled(text_indent.clone(), sel_bg.fg(theme.border)),
            Span::styled(entry.post.text.clone(), text_style),
        ]));

        // Engagement.
        let stats_style = if is_selected {
            sel_bg.fg(theme.muted)
        } else {
            Style::default().fg(theme.muted)
        };

        lines.push(Line::from(vec![
            Span::styled("  ", sel_bg),
            Span::styled(text_indent, sel_bg.fg(theme.border)),
            Span::styled(
                format!(
                    "♥ {}  ↻ {}  💬 {}",
                    entry.post.like_count, entry.post.repost_count, entry.post.reply_count
                ),
                stats_style,
            ),
        ]));

        lines.push(Line::from(""));
    }

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}
