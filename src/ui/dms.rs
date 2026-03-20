use crate::config::theme::Theme;
use crate::ui::pane::DmsPane;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

/// Render the DMs pane.
pub fn render_dms_pane(
    frame: &mut Frame,
    area: Rect,
    pane: &DmsPane,
    theme: &Theme,
    is_focused: bool,
) {
    if area.height < 3 || area.width < 10 {
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
        .title(Span::styled(" DMs ", Style::default().fg(theme.fg)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 10 || inner.height < 3 {
        return;
    }

    // Split: conversation list (left) | messages (right).
    let chunks =
        Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)]).split(inner);

    // Conversation list.
    render_convo_list(frame, chunks[0], pane, theme);

    // Message view.
    render_message_view(frame, chunks[1], pane, theme);
}

fn render_convo_list(frame: &mut Frame, area: Rect, pane: &DmsPane, theme: &Theme) {
    if pane.conversations.is_empty() {
        let paragraph = Paragraph::new(Line::from(Span::styled(
            "No conversations",
            Style::default().fg(theme.muted),
        )));
        frame.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = pane
        .conversations
        .iter()
        .enumerate()
        .map(|(i, convo)| {
            let member_names: Vec<&str> = convo
                .members
                .iter()
                .map(|m| m.display_name.as_deref().unwrap_or(&m.handle))
                .collect();
            let names = member_names.join(", ");

            let is_selected = i == pane.convo_cursor;
            let is_active = pane.active_convo.as_deref() == Some(&convo.id);

            let style = if is_active {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };

            let mut line_spans = vec![Span::styled(names, style)];

            if convo.unread_count > 0 {
                line_spans.push(Span::styled(
                    format!(" ({})", convo.unread_count),
                    Style::default().fg(theme.accent),
                ));
            }

            ListItem::new(Line::from(line_spans))
        })
        .collect();

    let convo_block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(theme.border));

    let list = List::new(items).block(convo_block);
    frame.render_widget(list, area);
}

fn render_message_view(frame: &mut Frame, area: Rect, pane: &DmsPane, theme: &Theme) {
    if pane.active_convo.is_none() {
        let paragraph = Paragraph::new(Line::from(Span::styled(
            "Select a conversation",
            Style::default().fg(theme.muted),
        )));
        frame.render_widget(paragraph, area);
        return;
    }

    if area.height < 3 {
        return;
    }

    // Messages area + compose input.
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(area);

    // Messages.
    let mut lines = Vec::new();
    for msg in &pane.messages {
        let sender_name = msg
            .sender
            .display_name
            .as_deref()
            .unwrap_or(&msg.sender.handle);
        lines.push(Line::from(vec![
            Span::styled(
                format!("{}: ", sender_name),
                Style::default()
                    .fg(theme.handle)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&msg.text, Style::default().fg(theme.fg)),
        ]));
    }

    let messages = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false });
    frame.render_widget(messages, chunks[0]);

    // Compose.
    let compose_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme.border));
    let compose = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(theme.accent)),
        Span::styled(&pane.draft, Style::default().fg(theme.fg)),
    ]))
    .block(compose_block);
    frame.render_widget(compose, chunks[1]);
}
