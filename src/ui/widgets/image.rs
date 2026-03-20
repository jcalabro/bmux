use crate::config::theme::Theme;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Image rendering protocol detection.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum ImageProtocol {
    Sixel,
    Kitty,
    None,
}

/// Detect which image protocol the terminal supports.
#[allow(dead_code)]
pub fn detect_image_protocol() -> ImageProtocol {
    // Kitty graphics protocol: supported by Kitty, Ghostty, and others.
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        return ImageProtocol::Kitty;
    }

    // Check TERM_PROGRAM for known terminal capabilities.
    if let Ok(term) = std::env::var("TERM_PROGRAM") {
        match term.to_lowercase().as_str() {
            // Kitty protocol support.
            "ghostty" | "kitty" => return ImageProtocol::Kitty,
            // Sixel support.
            "wezterm" | "foot" | "contour" | "mlterm" => return ImageProtocol::Sixel,
            _ => {}
        }
    }

    // Ghostty also sets this env var.
    if std::env::var("GHOSTTY_RESOURCES_DIR").is_ok() {
        return ImageProtocol::Kitty;
    }

    ImageProtocol::None
}

/// Parse the config image_protocol setting into an ImageProtocol.
#[allow(dead_code)]
pub fn parse_image_protocol(config_value: &str) -> ImageProtocol {
    match config_value {
        "sixel" => ImageProtocol::Sixel,
        "kitty" => ImageProtocol::Kitty,
        "none" => ImageProtocol::None,
        _ => detect_image_protocol(),
    }
}

/// Render an image placeholder or alt text.
#[allow(dead_code)]
pub fn render_image_placeholder(frame: &mut Frame, area: Rect, alt_text: &str, theme: &Theme) {
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
