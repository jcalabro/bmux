use crate::config::theme::Theme;
use crate::messages::{Post, PostEmbed};
use crate::ui::widgets::rich_text::render_rich_text;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui_image::StatefulImage;
use ratatui_image::protocol::StatefulProtocol;
use std::collections::HashMap;

/// Render a single post card.
pub fn render_post_card(
    frame: &mut Frame,
    area: Rect,
    post: &Post,
    theme: &Theme,
    selected: bool,
    image_protos: &mut HashMap<String, StatefulProtocol>,
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

    let lines = build_post_lines(post, theme, inner.width as usize);

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text);
    frame.render_widget(paragraph, inner);

    // Render actual images if available.
    // Find where images would be positioned and render them.
    if let Some(embed) = &post.embed {
        let image_urls = get_image_urls(embed);
        if !image_urls.is_empty() {
            // Calculate approximate Y position for images (after text, before engagement).
            let text_lines_count = post.text.lines().count().max(1) as u16;
            let reply_lines = if post.reply_context.is_some() { 2 } else { 0 };
            let repost_line = if post.reposted_by.is_some() { 1 } else { 0 };
            let img_y = inner.y + repost_line + reply_lines + 1 + text_lines_count + 1;

            let img_height = 6u16; // ~6 rows per image
            for (i, url) in image_urls.iter().enumerate() {
                if let Some(proto) = image_protos.get_mut(*url) {
                    let img_area = Rect::new(
                        inner.x + 2,
                        img_y + (i as u16 * img_height),
                        inner.width.saturating_sub(4),
                        img_height.min(
                            inner
                                .height
                                .saturating_sub(img_y - inner.y + i as u16 * img_height),
                        ),
                    );
                    if img_area.height > 0 && img_area.y + img_area.height <= inner.y + inner.height
                    {
                        let image_widget = StatefulImage::default();
                        frame.render_stateful_widget(image_widget, img_area, proto);
                    }
                }
            }
        }
    }
}

/// Build all the lines for a post card.
fn build_post_lines(post: &Post, theme: &Theme, width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Repost indicator.
    if let Some(reposter) = &post.reposted_by {
        let name = reposter.display_name.as_deref().unwrap_or(&reposter.handle);
        lines.push(Line::from(Span::styled(
            format!("↻ {} reposted", name),
            Style::default().fg(theme.repost),
        )));
    }

    // Reply context: show who they're replying to with a snippet.
    if let Some(ctx) = &post.reply_context {
        let _parent_name = ctx
            .parent_author
            .display_name
            .as_deref()
            .unwrap_or(&ctx.parent_author.handle);

        // Truncate parent text to fit on one line.
        let max_preview = width.saturating_sub(20);
        let preview = truncate_text(&ctx.parent_text, max_preview);

        if let Some(root) = &ctx.root_author {
            let _root_name = root.display_name.as_deref().unwrap_or(&root.handle);
            lines.push(Line::from(vec![
                Span::styled("  ↪ ", Style::default().fg(theme.muted)),
                Span::styled(
                    format!("@{}", ctx.parent_author.handle),
                    Style::default().fg(theme.handle),
                ),
                Span::styled(
                    format!(" (thread by @{})", root.handle),
                    Style::default().fg(theme.muted),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("  ↪ ", Style::default().fg(theme.muted)),
                Span::styled(
                    format!("@{}", ctx.parent_author.handle),
                    Style::default().fg(theme.handle),
                ),
            ]));
        }

        if !preview.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  │ {}", preview),
                Style::default().fg(theme.muted),
            )));
        }
    }

    // Author header.
    let display_name = post
        .author
        .display_name
        .as_deref()
        .unwrap_or(&post.author.handle);
    let time_str = format_relative_time(&post.created_at);

    lines.push(Line::from(vec![
        Span::styled(
            format!("{} ", display_name),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("@{}", post.author.handle),
            Style::default().fg(theme.handle),
        ),
        Span::styled(" · ", Style::default().fg(theme.muted)),
        Span::styled(time_str, Style::default().fg(theme.timestamp)),
    ]));

    // Post text.
    lines.push(render_rich_text(&post.text, &post.facets, theme));

    // Embeds.
    if let Some(embed) = &post.embed {
        lines.push(Line::from(""));
        render_embed_lines(&mut lines, embed, theme, width);
    }

    // Engagement.
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            format!("♥ {} ", post.like_count),
            Style::default().fg(if post.liked_by_me.is_some() {
                theme.like
            } else {
                theme.muted
            }),
        ),
        Span::styled(
            format!("↻ {} ", post.repost_count),
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
    ]));

    lines
}

/// Render embed content as lines.
fn render_embed_lines(
    lines: &mut Vec<Line<'static>>,
    embed: &PostEmbed,
    theme: &Theme,
    width: usize,
) {
    match embed {
        PostEmbed::Images(images) => {
            for img in images {
                let alt = if img.alt.is_empty() {
                    "image".to_string()
                } else {
                    img.alt.clone()
                };
                lines.push(Line::from(Span::styled(
                    format!("  📷 {}", alt),
                    Style::default().fg(theme.muted),
                )));
            }
        }
        PostEmbed::External {
            title,
            uri,
            description,
        } => {
            lines.push(Line::from(vec![
                Span::styled("  🔗 ", Style::default().fg(theme.secondary)),
                Span::styled(
                    title.clone(),
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            if !description.is_empty() {
                let desc_preview = truncate_text(description, width.saturating_sub(6));
                lines.push(Line::from(Span::styled(
                    format!("     {}", desc_preview),
                    Style::default().fg(theme.muted),
                )));
            }
            lines.push(Line::from(Span::styled(
                format!("     {}", uri),
                Style::default().fg(theme.muted),
            )));
        }
        PostEmbed::Record(qp) => {
            render_quoted_post_lines(lines, qp, theme, width);
        }
        PostEmbed::RecordWithMedia { record, images } => {
            render_quoted_post_lines(lines, record, theme, width);
            for img in images {
                let alt = if img.alt.is_empty() {
                    "image".to_string()
                } else {
                    img.alt.clone()
                };
                lines.push(Line::from(Span::styled(
                    format!("  📷 {}", alt),
                    Style::default().fg(theme.muted),
                )));
            }
        }
    }
}

/// Render a quoted post inline with a border.
fn render_quoted_post_lines(
    lines: &mut Vec<Line<'static>>,
    qp: &crate::messages::QuotedPost,
    theme: &Theme,
    width: usize,
) {
    let qp_name = qp
        .author
        .display_name
        .as_deref()
        .unwrap_or(&qp.author.handle);
    let qp_time = format_relative_time(&qp.created_at);

    // Top border of quote box.
    let bar_width = width.saturating_sub(4);
    lines.push(Line::from(Span::styled(
        format!("  ┌{}┐", "─".repeat(bar_width)),
        Style::default().fg(theme.border),
    )));

    // Quote author.
    lines.push(Line::from(vec![
        Span::styled("  │ ", Style::default().fg(theme.border)),
        Span::styled(
            format!("{} ", qp_name),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("@{}", qp.author.handle),
            Style::default().fg(theme.handle),
        ),
        Span::styled(" · ", Style::default().fg(theme.muted)),
        Span::styled(qp_time, Style::default().fg(theme.timestamp)),
    ]));

    // Quote text -- word-wrap within the box.
    let text_width = width.saturating_sub(6);
    let wrapped = wrap_text(&qp.text, text_width);
    for wline in &wrapped {
        lines.push(Line::from(vec![
            Span::styled("  │ ", Style::default().fg(theme.border)),
            Span::styled(wline.clone(), Style::default().fg(theme.fg)),
        ]));
    }

    // Bottom border.
    lines.push(Line::from(Span::styled(
        format!("  └{}┘", "─".repeat(bar_width)),
        Style::default().fg(theme.border),
    )));
}

/// Get all image thumbnail URLs from an embed.
fn get_image_urls(embed: &PostEmbed) -> Vec<&str> {
    match embed {
        PostEmbed::Images(images) => images.iter().map(|img| img.thumb_url.as_str()).collect(),
        PostEmbed::RecordWithMedia { images, .. } => {
            images.iter().map(|img| img.thumb_url.as_str()).collect()
        }
        _ => vec![],
    }
}

/// Estimate how many rows a post card will take.
pub fn post_card_height(post: &Post, width: u16) -> u16 {
    let inner_width = if width > 4 { (width - 4) as usize } else { 1 };

    let text_lines = (post.text.len() / inner_width.max(1) + 1) as u16;

    let reply_lines = if post.reply_context.is_some() { 2 } else { 0 };

    let embed_lines: u16 = match &post.embed {
        Some(PostEmbed::Images(imgs)) => imgs.len() as u16,
        Some(PostEmbed::External { description, .. }) => {
            if description.is_empty() {
                2
            } else {
                3
            }
        }
        Some(PostEmbed::Record(qp)) => {
            // borders(2) + author(1) + text lines
            let qp_text_lines = (qp.text.len() / inner_width.max(1) + 1) as u16;
            2 + 1 + qp_text_lines
        }
        Some(PostEmbed::RecordWithMedia { record, images }) => {
            let qp_text_lines = (record.text.len() / inner_width.max(1) + 1) as u16;
            2 + 1 + qp_text_lines + images.len() as u16
        }
        None => 0,
    };

    let repost_line: u16 = if post.reposted_by.is_some() { 1 } else { 0 };

    // border(1) + repost + reply_ctx + header(1) + text + blank + embed + blank + engagement(1) + border(1)
    2 + repost_line + reply_lines + 1 + text_lines + 1 + embed_lines + 1 + 1
}

/// Simple word-wrapping.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();
    for line in text.split('\n') {
        if line.is_empty() {
            result.push(String::new());
            continue;
        }

        let mut current = String::new();
        for word in line.split(' ') {
            if current.is_empty() {
                current = word.to_string();
            } else if current.len() + 1 + word.len() > max_width {
                result.push(current);
                current = word.to_string();
            } else {
                current.push(' ');
                current.push_str(word);
            }
        }
        if !current.is_empty() {
            result.push(current);
        }
    }

    if result.is_empty() {
        result.push(String::new());
    }
    result
}

/// Truncate text to fit within a max width, adding ellipsis.
fn truncate_text(text: &str, max_len: usize) -> String {
    // Collapse newlines to spaces for inline preview.
    let flat: String = text
        .chars()
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect();
    if flat.len() <= max_len {
        flat
    } else if max_len > 3 {
        format!("{}...", &flat[..max_len - 3])
    } else {
        flat[..max_len].to_string()
    }
}

/// Format a timestamp as a relative time string.
fn format_relative_time(timestamp: &str) -> String {
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
    use crate::messages::{Author, QuotedPost, ReplyContext};

    fn test_post() -> Post {
        Post {
            uri: "at://test".into(),
            cid: "cid1".into(),
            author: Author {
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
            reply_context: None,
            embed: None,
            reposted_by: None,
        }
    }

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
    fn test_post_card_height_basic() {
        let post = test_post();
        let height = post_card_height(&post, 80);
        assert!(height >= 6);
    }

    #[test]
    fn test_post_card_height_with_reply_context() {
        let mut post = test_post();
        post.reply_context = Some(ReplyContext {
            parent_author: Author {
                did: "did:plc:parent".into(),
                handle: "parent.bsky.social".into(),
                display_name: None,
                avatar_url: None,
            },
            parent_text: "the parent post".into(),
            root_author: None,
        });
        let height = post_card_height(&post, 80);
        let base_height = post_card_height(&test_post(), 80);
        assert!(height > base_height);
    }

    #[test]
    fn test_post_card_height_with_quote() {
        let mut post = test_post();
        post.embed = Some(PostEmbed::Record(QuotedPost {
            uri: "at://quoted".into(),
            author: Author {
                did: "did:plc:quoted".into(),
                handle: "quoted.bsky.social".into(),
                display_name: Some("Quoted Person".into()),
                avatar_url: None,
            },
            text: "this is the quoted post text".into(),
            created_at: "2024-01-01T00:00:00Z".into(),
        }));
        let height = post_card_height(&post, 80);
        let base_height = post_card_height(&test_post(), 80);
        assert!(height > base_height);
    }

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("hello", 10), "hello");
        assert_eq!(truncate_text("hello world foo", 10), "hello w...");
        assert_eq!(truncate_text("a\nb", 10), "a b");
    }

    #[test]
    fn test_wrap_text() {
        let lines = wrap_text("hello world this is a test", 12);
        assert!(lines.len() >= 2);
        for line in &lines {
            assert!(line.len() <= 12);
        }
    }

    #[test]
    fn test_build_post_lines_basic() {
        let post = test_post();
        let theme = crate::config::theme::Theme::bluesky();
        let lines = build_post_lines(&post, &theme, 80);
        assert!(lines.len() >= 3); // header + text + engagement
    }

    #[test]
    fn test_build_post_lines_with_repost() {
        let mut post = test_post();
        post.reposted_by = Some(Author {
            did: "did:plc:reposter".into(),
            handle: "reposter.bsky.social".into(),
            display_name: Some("Reposter".into()),
            avatar_url: None,
        });
        let theme = crate::config::theme::Theme::bluesky();
        let lines = build_post_lines(&post, &theme, 80);
        // First line should be the repost indicator.
        let first = lines[0].spans.first().unwrap();
        assert!(first.content.contains("reposted"));
    }

    #[test]
    fn test_build_post_lines_with_reply_context() {
        let mut post = test_post();
        post.reply_context = Some(ReplyContext {
            parent_author: Author {
                did: "did:plc:parent".into(),
                handle: "parent.bsky.social".into(),
                display_name: None,
                avatar_url: None,
            },
            parent_text: "parent said this".into(),
            root_author: None,
        });
        let theme = crate::config::theme::Theme::bluesky();
        let lines = build_post_lines(&post, &theme, 80);
        // Should contain the reply indicator line.
        let has_reply = lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.content.contains("@parent")));
        assert!(has_reply);
    }

    #[test]
    fn test_build_post_lines_with_quote_post() {
        let mut post = test_post();
        post.embed = Some(PostEmbed::Record(QuotedPost {
            uri: "at://quoted".into(),
            author: Author {
                did: "did:plc:quoted".into(),
                handle: "quoted.bsky.social".into(),
                display_name: Some("Quoted".into()),
                avatar_url: None,
            },
            text: "the original hot take".into(),
            created_at: "2024-01-01T00:00:00Z".into(),
        }));
        let theme = crate::config::theme::Theme::bluesky();
        let lines = build_post_lines(&post, &theme, 80);
        // Should contain the quoted post's text somewhere.
        let has_quote = lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.content.contains("hot take")));
        assert!(has_quote);
        // Should have box-drawing characters.
        let has_border = lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.content.contains('┌')));
        assert!(has_border);
    }
}
