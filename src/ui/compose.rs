use crate::config::theme::Theme;
use crate::ui::pane::ComposePane;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

const MAX_GRAPHEMES: usize = 300;

/// Render the compose pane.
pub fn render_compose_pane(
    frame: &mut Frame,
    area: Rect,
    pane: &ComposePane,
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

    let title = if pane.reply_to.is_some() {
        " Reply "
    } else if pane.quote.is_some() {
        " Quote Post "
    } else {
        " New Post "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(title, Style::default().fg(theme.fg)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    // Layout: text area + status line.
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(inner);

    // Text area.
    let text_style = Style::default().fg(theme.fg);
    let text = if pane.text.is_empty() {
        Text::from(Line::from(Span::styled(
            "What's on your mind?",
            Style::default().fg(theme.muted),
        )))
    } else {
        let lines: Vec<Line> = pane
            .text
            .split('\n')
            .map(|line| Line::from(Span::styled(line.to_string(), text_style)))
            .collect();
        Text::from(lines)
    };

    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, chunks[0]);

    // Status line: character count + hints.
    let count = pane.grapheme_count();
    let count_color = if count > MAX_GRAPHEMES {
        theme.error
    } else if count > MAX_GRAPHEMES - 20 {
        theme.warning
    } else {
        theme.muted
    };

    let status = Line::from(vec![
        Span::styled(
            format!("{}/{}", count, MAX_GRAPHEMES),
            Style::default().fg(count_color),
        ),
        Span::styled(
            "  Enter:send  Esc:cancel  Ctrl-e:$EDITOR",
            Style::default().fg(theme.muted),
        ),
    ]);

    let status_paragraph = Paragraph::new(status);
    frame.render_widget(status_paragraph, chunks[1]);
}
