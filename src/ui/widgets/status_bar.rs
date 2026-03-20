use crate::config::theme::Theme;
use crate::input::vim::VimMode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Render the bottom status bar showing vim mode and contextual hints.
pub fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    mode: VimMode,
    command_buffer: &str,
    theme: &Theme,
    focused_pane_type: &str,
) {
    if area.height == 0 {
        return;
    }

    let (mode_str, mode_bg) = match mode {
        VimMode::Normal => (" NORMAL ", theme.normal_bg),
        VimMode::Insert => (" INSERT ", theme.insert_bg),
        VimMode::Command => (" COMMAND ", theme.command_bg),
    };

    let mode_span = Span::styled(
        mode_str,
        Style::default()
            .fg(theme.bg)
            .bg(mode_bg)
            .add_modifier(Modifier::BOLD),
    );

    let separator = Span::styled(" │ ", Style::default().fg(theme.border));

    let hints = match mode {
        VimMode::Normal => match focused_pane_type {
            "feed" => "j/k:scroll  l:thread  f:like  b:repost  r:reply  t:quote  c:compose  /:search  ?:help",
            "thread" => "j/k:scroll  h:back  f:like  b:repost  r:reply  t:quote  ?:help",
            "dms" => "j/k:scroll  l:open  c:compose  ?:help",
            "notifications" => "j/k:scroll  l:open  ?:help",
            "profile" => "j/k:scroll  H/L:tabs  ?:help",
            "compose" => "Enter:send  Esc:cancel  Ctrl-e:editor",
            _ => "?:help",
        },
        VimMode::Insert => "Esc:cancel  Enter:send  Ctrl-a:attach  Ctrl-e:editor",
        VimMode::Command => "Enter:execute  Esc:cancel",
    };

    let hints_span = Span::styled(hints, Style::default().fg(theme.muted));

    let spans = if mode == VimMode::Command && !command_buffer.is_empty() {
        vec![
            mode_span,
            separator,
            Span::styled(
                format!(":{}", command_buffer),
                Style::default().fg(theme.fg),
            ),
        ]
    } else {
        vec![mode_span, separator, hints_span]
    };

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(theme.bg));
    frame.render_widget(paragraph, area);
}
