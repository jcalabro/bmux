use crate::config::theme::Theme;
use crate::messages::{Toast, ToastLevel};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use std::collections::VecDeque;

/// Toast notification manager.
#[derive(Debug)]
pub struct ToastManager {
    pub toasts: VecDeque<Toast>,
    pub max_visible: usize,
}

impl ToastManager {
    pub fn new() -> Self {
        Self {
            toasts: VecDeque::new(),
            max_visible: 3,
        }
    }

    pub fn push(&mut self, toast: Toast) {
        self.toasts.push_back(toast);
        // Limit total queue size.
        while self.toasts.len() > 10 {
            self.toasts.pop_front();
        }
    }

    /// Remove expired toasts.
    pub fn tick(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }

    /// Render toasts in the bottom-right corner.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let visible: Vec<&Toast> = self.toasts.iter().rev().take(self.max_visible).collect();
        if visible.is_empty() {
            return;
        }

        // Max width: 80% of the screen, capped at 80 cols, min 30.
        let max_toast_width = (area.width * 4 / 5).clamp(30, 80);

        let mut y_cursor = area.height.saturating_sub(3);

        for toast in &visible {
            let (icon, border_color) = match toast.level {
                ToastLevel::Info => ("ℹ", theme.accent),
                ToastLevel::Success => ("✓", theme.success),
                ToastLevel::Warning => ("⚠", theme.warning),
                ToastLevel::Error => ("✗", theme.error),
            };

            let prefix = format!(" {} ", icon);
            // +2 for borders, +prefix len for icon
            let content_width = max_toast_width.saturating_sub(2) as usize;
            let prefix_len = 3; // " X "
            let msg_width = content_width.saturating_sub(prefix_len);

            // Wrap the message into lines that fit (char-aware).
            let msg = &toast.message;
            let wrapped: Vec<String> = if msg.chars().count() <= msg_width {
                vec![msg.clone()]
            } else {
                let chars: Vec<char> = msg.chars().collect();
                chars
                    .chunks(msg_width.max(1))
                    .map(|chunk| chunk.iter().collect())
                    .collect()
            };

            let line_count = wrapped.len() as u16;
            let toast_height = line_count + 2; // +2 for borders
            let toast_width = max_toast_width.min(area.width.saturating_sub(2));

            if y_cursor < toast_height + area.y {
                break;
            }
            y_cursor = y_cursor.saturating_sub(toast_height);

            let x_offset = area.width.saturating_sub(toast_width + 1);
            let toast_area = Rect::new(x_offset, y_cursor, toast_width, toast_height);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color));

            let mut lines = Vec::with_capacity(wrapped.len());
            for (li, line_text) in wrapped.iter().enumerate() {
                let mut spans = Vec::new();
                if li == 0 {
                    spans.push(Span::styled(
                        prefix.clone(),
                        Style::default()
                            .fg(border_color)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    spans.push(Span::raw("   ")); // align with first line
                }
                spans.push(Span::styled(line_text.clone(), Style::default().fg(theme.fg)));
                lines.push(Line::from(spans));
            }

            frame.render_widget(Clear, toast_area);
            let paragraph = Paragraph::new(lines)
                .block(block)
                .style(Style::default().bg(theme.bg));
            frame.render_widget(paragraph, toast_area);

            // Gap between toasts.
            y_cursor = y_cursor.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_toast_manager_push() {
        let mut tm = ToastManager::new();
        tm.push(Toast::info("test"));
        assert_eq!(tm.toasts.len(), 1);
    }

    #[test]
    fn test_toast_manager_tick_removes_expired() {
        let mut tm = ToastManager::new();
        tm.push(Toast {
            message: "expired".into(),
            level: ToastLevel::Info,
            ttl_ms: 0,
            created_at: Instant::now(),
        });
        std::thread::sleep(std::time::Duration::from_millis(1));
        tm.tick();
        assert!(tm.toasts.is_empty());
    }

    #[test]
    fn test_toast_manager_max_queue() {
        let mut tm = ToastManager::new();
        for i in 0..15 {
            tm.push(Toast::info(format!("toast {}", i)));
        }
        assert!(tm.toasts.len() <= 10);
    }
}
