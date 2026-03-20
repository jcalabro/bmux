use crate::config::theme::Theme;
use crate::messages::{Toast, ToastLevel};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
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

        let toast_width = 50.min(area.width.saturating_sub(4));
        let toast_height = 3u16;

        for (i, toast) in visible.iter().enumerate() {
            let y_offset = area.height.saturating_sub((i as u16 + 1) * (toast_height + 1) + 2);
            let x_offset = area.width.saturating_sub(toast_width + 2);

            let toast_area = Rect::new(x_offset, y_offset, toast_width, toast_height);

            if toast_area.y < area.y || toast_area.height == 0 {
                continue;
            }

            let (icon, border_color) = match toast.level {
                ToastLevel::Info => ("ℹ", theme.accent),
                ToastLevel::Success => ("✓", theme.success),
                ToastLevel::Warning => ("⚠", theme.warning),
                ToastLevel::Error => ("✗", theme.error),
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color));

            let text = Line::from(vec![
                Span::styled(
                    format!(" {} ", icon),
                    Style::default()
                        .fg(border_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&toast.message, Style::default().fg(theme.fg)),
            ]);

            frame.render_widget(Clear, toast_area);
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().bg(theme.bg));
            frame.render_widget(paragraph, toast_area);
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
