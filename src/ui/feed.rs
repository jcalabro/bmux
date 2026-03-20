use crate::config::theme::Theme;
use crate::ui::pane::FeedPane;
use crate::ui::widgets::post_card::{post_card_height, render_post_card};
use crate::ui::widgets::tab_bar::render_feed_tabs;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

/// Render a feed pane.
pub fn render_feed_pane(
    frame: &mut Frame,
    area: Rect,
    pane: &FeedPane,
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
        .title(Span::styled(" Feed ", Style::default().fg(theme.fg)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    // Layout: feed tabs (1 line) + posts.
    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(inner);

    // Feed tabs.
    let tab_names: Vec<String> = pane.tabs.iter().map(|t| t.name.clone()).collect();
    render_feed_tabs(frame, chunks[0], &tab_names, pane.active_tab, theme);

    // Posts.
    if let Some(tab) = pane.active_tab() {
        if tab.posts.is_empty() {
            let msg = if tab.loading {
                "Loading..."
            } else {
                "No posts yet. Press r to refresh."
            };
            let paragraph = Paragraph::new(Line::from(Span::styled(
                msg,
                Style::default().fg(theme.muted),
            )));
            frame.render_widget(paragraph, chunks[1]);
            return;
        }

        let post_area = chunks[1];
        let mut y = post_area.y;

        // Render posts starting from scroll_offset.
        for (i, post) in tab.posts.iter().enumerate().skip(tab.scroll_offset) {
            if y >= post_area.y + post_area.height {
                break;
            }

            let height = post_card_height(post, post_area.width).min(post_area.y + post_area.height - y);
            if height < 3 {
                break;
            }

            let card_area = Rect::new(post_area.x, y, post_area.width, height);
            let selected = i == tab.selected;
            render_post_card(frame, card_area, post, theme, selected);

            y += height;
        }
    }
}
