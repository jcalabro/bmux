use crate::config::theme::Theme;
use crate::messages::ImageData;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Image rendering protocol detection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageProtocol {
    Sixel,
    Kitty,
    None,
}

/// Detect which image protocol the terminal supports.
pub fn detect_image_protocol() -> ImageProtocol {
    // Check for Kitty first (via TERM_PROGRAM or KITTY_WINDOW_ID).
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        return ImageProtocol::Kitty;
    }

    // Check TERM_PROGRAM for known Sixel-capable terminals.
    if let Ok(term) = std::env::var("TERM_PROGRAM") {
        match term.as_str() {
            "WezTerm" | "foot" | "contour" | "mlterm" => return ImageProtocol::Sixel,
            _ => {}
        }
    }

    // Check TERM for xterm with Sixel support.
    if let Ok(term) = std::env::var("TERM") {
        if term.contains("xterm") {
            // Could do DA1 query but that's complex; default to none.
        }
    }

    ImageProtocol::None
}

/// Parse the config image_protocol setting into an ImageProtocol.
pub fn parse_image_protocol(config_value: &str) -> ImageProtocol {
    match config_value {
        "sixel" => ImageProtocol::Sixel,
        "kitty" => ImageProtocol::Kitty,
        "none" => ImageProtocol::None,
        "auto" | _ => detect_image_protocol(),
    }
}

/// Render an image placeholder or alt text.
pub fn render_image_placeholder(
    frame: &mut Frame,
    area: Rect,
    alt_text: &str,
    theme: &Theme,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let display = if alt_text.is_empty() {
        "[image]".to_string()
    } else {
        format!("[📷 {}]", alt_text)
    };

    let paragraph = Paragraph::new(Line::from(Span::styled(
        display,
        Style::default().fg(theme.muted),
    )));
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_image_protocol() {
        assert_eq!(parse_image_protocol("sixel"), ImageProtocol::Sixel);
        assert_eq!(parse_image_protocol("kitty"), ImageProtocol::Kitty);
        assert_eq!(parse_image_protocol("none"), ImageProtocol::None);
    }
}
