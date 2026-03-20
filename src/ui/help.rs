use crate::config::theme::Theme;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

/// Render the help overlay.
pub fn render_help(frame: &mut Frame, area: Rect, theme: &Theme) {
    // Center the popup.
    let popup_width = 70.min(area.width.saturating_sub(4));
    let popup_height = 40.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(
            " bmux — Keybinding Reference ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));

    let header_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    let key_style = Style::default()
        .fg(theme.secondary)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme.fg);

    let lines = vec![
        Line::from(Span::styled("Normal Mode", header_style)),
        Line::from(""),
        help_line("j / k", "Scroll down / up", key_style, desc_style),
        help_line("gg / G", "Jump to top / bottom", key_style, desc_style),
        help_line(
            "Ctrl-d / Ctrl-u",
            "Half-page down / up",
            key_style,
            desc_style,
        ),
        help_line("l / Enter", "Open thread", key_style, desc_style),
        help_line("h / Esc", "Go back", key_style, desc_style),
        help_line("f", "Like post", key_style, desc_style),
        help_line("b", "Repost", key_style, desc_style),
        help_line("r", "Reply", key_style, desc_style),
        help_line("t", "Quote post", key_style, desc_style),
        help_line("c", "Compose new post", key_style, desc_style),
        help_line("E", "Compose in $EDITOR", key_style, desc_style),
        help_line("p", "Open profile", key_style, desc_style),
        help_line("o", "Open in browser", key_style, desc_style),
        help_line("/", "Search", key_style, desc_style),
        help_line("n / N", "Next / prev search result", key_style, desc_style),
        help_line("q", "Quit", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("Pane Management", header_style)),
        Line::from(""),
        help_line("1-9", "Switch workspace", key_style, desc_style),
        help_line("Tab", "Cycle pane focus", key_style, desc_style),
        help_line(
            "Ctrl-w h/j/k/l",
            "Focus pane direction",
            key_style,
            desc_style,
        ),
        help_line("Ctrl-w +/-/</>", "Resize pane", key_style, desc_style),
        help_line("Ctrl-w =", "Equalize panes", key_style, desc_style),
        help_line("Ctrl-w o", "Zoom pane", key_style, desc_style),
        help_line("H / L", "Prev / next feed tab", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("Insert Mode", header_style)),
        Line::from(""),
        help_line("Esc", "Cancel / normal mode", key_style, desc_style),
        help_line("Enter", "Send post", key_style, desc_style),
        help_line("Ctrl-a", "Attach image", key_style, desc_style),
        help_line("Ctrl-e", "Switch to $EDITOR", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("Commands (:)", header_style)),
        Line::from(""),
        help_line(":q", "Quit", key_style, desc_style),
        help_line(":split / :vsplit", "Split pane", key_style, desc_style),
        help_line(":close", "Close pane", key_style, desc_style),
        help_line(":workspace name", "Switch workspace", key_style, desc_style),
        help_line(":theme name", "Switch theme", key_style, desc_style),
        help_line(":dm @handle", "Open DM", key_style, desc_style),
        help_line(":follow @handle", "Follow user", key_style, desc_style),
        help_line(":mute @handle", "Mute user", key_style, desc_style),
    ];

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().bg(theme.bg))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, popup_area);
}

fn help_line<'a>(key: &'a str, desc: &'a str, key_style: Style, desc_style: Style) -> Line<'a> {
    let padding = 20usize.saturating_sub(key.len());
    Line::from(vec![
        Span::styled(format!("  {}", key), key_style),
        Span::raw(" ".repeat(padding)),
        Span::styled(desc, desc_style),
    ])
}
