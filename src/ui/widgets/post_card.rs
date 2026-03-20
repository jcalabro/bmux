use crate::config::theme::Theme;
use crate::messages::Post;
use crate::ui::widgets::rich_text::render_rich_text;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

/// Render a single post card.
pub fn render_post_card(
    frame: &mut Frame,
    area: Rect,
    post: &Post,
    theme: &Theme,
    selected: bool,
) {
    if area.height < 3 || area.width < 10 {
        return;
    }

    let border_style = if selected {
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.border)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width < 5 {
        return;
    }

    // Header line: @handle · timestamp
    let display_name = post
        .author
        .display_name
        .as_deref()
        .unwrap_or(&post.author.handle);

    let repost_prefix = if let Some(reposter) = &post.reposted_by {
        let name = reposter
            .display_name
            .as_deref()
            .unwrap_or(&reposter.handle);
        format!("↻ {} reposted\n", name)
    } else {
        String::new()
    };

    let time_str = format_relative_time(&post.created_at);

    let header = Line::from(vec![
        Span::styled(
            format!("{} ", display_name),
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("@{}", post.author.handle),
            Style::default().fg(theme.handle),
        ),
        Span::styled(" · ", Style::default().fg(theme.muted)),
        Span::styled(time_str, Style::default().fg(theme.timestamp)),
    ]);

    // Post text with facets.
    let text_line = render_rich_text(&post.text, &post.facets, theme);

    // Engagement line.
    let engagement = Line::from(vec![
        Span::styled(
            format!(
                "♥ {} ",
                post.like_count
            ),
            Style::default().fg(if post.liked_by_me.is_some() {
                theme.like
            } else {
                theme.muted
            }),
        ),
        Span::styled(
            format!(
                "↻ {} ",
                post.repost_count
            ),
            Style::default().fg(if post.reposted_by_me.is_some() {
                theme.repost
            } else {
                theme.muted
            }),
        ),
        Span::styled(
            format!("💬 {}", post.reply_count),
            Style::default().fg(theme.reply),
        ),
    ]);

    // Build lines.
    let mut lines = Vec::new();
    if !repost_prefix.is_empty() {
        lines.push(Line::from(Span::styled(
            repost_prefix.trim(),
            Style::default().fg(theme.repost),
        )));
    }
    lines.push(header);
    lines.push(Line::from(""));
    lines.push(text_line);

    // Show embed info.
    if let Some(embed) = &post.embed {
        match embed {
            crate::messages::PostEmbed::Images(images) => {
                for img in images {
                    let alt = if img.alt.is_empty() {
                        "image".to_string()
                    } else {
                        img.alt.clone()
                    };
                    lines.push(Line::from(Span::styled(
                        format!("  [📷 {}]", alt),
                        Style::default().fg(theme.muted),
                    )));
                }
            }
            crate::messages::PostEmbed::External {
                title, uri, ..
            } => {
                lines.push(Line::from(Span::styled(
                    format!("  [🔗 {} — {}]", title, uri),
                    Style::default().fg(theme.secondary),
                )));
            }
            crate::messages::PostEmbed::Record { uri } => {
                lines.push(Line::from(Span::styled(
                    format!("  [📝 quote: {}]", uri),
                    Style::default().fg(theme.muted),
                )));
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(engagement);

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text);
    frame.render_widget(paragraph, inner);
}

/// Estimate how many rows a post card will take.
pub fn post_card_height(post: &Post, width: u16) -> u16 {
    let text_lines = if width > 4 {
        let inner_width = (width - 4) as usize;
        let line_count = post.text.len() / inner_width.max(1) + 1;
        line_count as u16
    } else {
        1
    };

    let embed_lines = match &post.embed {
        Some(crate::messages::PostEmbed::Images(imgs)) => imgs.len() as u16,
        Some(crate::messages::PostEmbed::External { .. }) => 1,
        Some(crate::messages::PostEmbed::Record { .. }) => 1,
        None => 0,
    };

    let repost_line = if post.reposted_by.is_some() { 1 } else { 0 };

    // border(1) + header(1) + blank(1) + text + embed + blank(1) + engagement(1) + border(1)
    2 + 1 + 1 + text_lines + embed_lines + 1 + 1 + repost_line
}

/// Format a timestamp as a relative time string.
fn format_relative_time(timestamp: &str) -> String {
    // Simple relative time formatting.
    // In production you'd parse ISO 8601 properly.
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        let now = chrono::Utc::now();
        let diff = now.signed_duration_since(dt);

        if diff.num_seconds() < 60 {
            return format!("{}s", diff.num_seconds().max(0));
        } else if diff.num_minutes() < 60 {
            return format!("{}m", diff.num_minutes());
        } else if diff.num_hours() < 24 {
            return format!("{}h", diff.num_hours());
        } else if diff.num_days() < 7 {
            return format!("{}d", diff.num_days());
        } else {
            return dt.format("%b %d").to_string();
        }
    }
    timestamp.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_relative_time_recent() {
        let now = chrono::Utc::now();
        let ts = now.to_rfc3339();
        let result = format_relative_time(&ts);
        assert!(result.ends_with('s'));
    }

    #[test]
    fn test_format_relative_time_invalid() {
        assert_eq!(format_relative_time("not a date"), "not a date");
    }

    #[test]
    fn test_post_card_height() {
        let post = Post {
            uri: "at://test".into(),
            cid: "cid1".into(),
            author: crate::messages::Author {
                did: "did:plc:test".into(),
                handle: "test.bsky.social".into(),
                display_name: None,
                avatar_url: None,
            },
            text: "short post".into(),
            facets: vec![],
            created_at: "2024-01-01T00:00:00Z".into(),
            like_count: 0,
            repost_count: 0,
            reply_count: 0,
            liked_by_me: None,
            reposted_by_me: None,
            reply_to: None,
            embed: None,
            reposted_by: None,
        };
        let height = post_card_height(&post, 80);
        assert!(height >= 6); // minimum reasonable height
    }
}
