use crate::config::theme::Theme;
use crate::messages::{Facet, RichTextKind, parse_rich_text};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

/// Convert post text with facets into styled ratatui Spans.
pub fn render_rich_text(text: &str, facets: &[Facet], theme: &Theme) -> Line<'static> {
    let segments = parse_rich_text(text, facets);
    let spans: Vec<Span<'static>> = segments
        .into_iter()
        .map(|seg| {
            let style = match &seg.kind {
                RichTextKind::Plain => Style::default().fg(theme.fg),
                RichTextKind::Mention(_) => Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
                RichTextKind::Link(_) => Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::UNDERLINED),
                RichTextKind::Hashtag(_) => Style::default().fg(theme.accent),
            };
            Span::styled(seg.text, style)
        })
        .collect();
    Line::from(spans)
}

/// Wrap text into multiple lines respecting terminal width.
/// Returns styled lines.
#[allow(dead_code)]
pub fn wrap_rich_text<'a>(
    text: &'a str,
    facets: &[Facet],
    theme: &Theme,
    max_width: usize,
) -> Vec<Line<'a>> {
    if max_width == 0 {
        return vec![];
    }

    // Simple word-wrapping: split the text into lines first.
    let mut lines = Vec::new();
    for text_line in text.split('\n') {
        if text_line.is_empty() {
            lines.push(Line::from(""));
            continue;
        }

        // For now, do simple character-based wrapping.
        let mut current_line = String::new();
        for word in text_line.split(' ') {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() > max_width {
                // Re-render with facets for this line portion.
                lines.push(Line::from(Span::styled(
                    current_line.clone(),
                    Style::default().fg(theme.fg),
                )));
                current_line = word.to_string();
            } else {
                current_line.push(' ');
                current_line.push_str(word);
            }
        }
        if !current_line.is_empty() {
            lines.push(Line::from(Span::styled(
                current_line,
                Style::default().fg(theme.fg),
            )));
        }
    }

    if lines.is_empty() {
        lines.push(render_rich_text(text, facets, theme));
    }

    lines
}
