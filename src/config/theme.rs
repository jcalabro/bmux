use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A complete color theme for the application.
#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub secondary: Color,
    pub border: Color,
    pub muted: Color,
    pub error: Color,
    pub warning: Color,
    pub success: Color,
    pub normal_bg: Color,
    pub insert_bg: Color,
    pub command_bg: Color,
    pub handle: Color,
    pub timestamp: Color,
    pub like: Color,
    pub repost: Color,
    pub reply: Color,
}

/// Serializable theme definition from TOML.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeDef {
    pub bg: Option<String>,
    pub fg: Option<String>,
    pub accent: Option<String>,
    pub secondary: Option<String>,
    pub border: Option<String>,
    pub muted: Option<String>,
    pub error: Option<String>,
    pub warning: Option<String>,
    pub success: Option<String>,
    pub normal_bg: Option<String>,
    pub insert_bg: Option<String>,
    pub command_bg: Option<String>,
    pub handle: Option<String>,
    pub timestamp: Option<String>,
    pub like: Option<String>,
    pub repost: Option<String>,
    pub reply: Option<String>,
}

fn parse_hex_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Color::Rgb(r, g, b);
        }
    }
    Color::White
}

impl Theme {
    /// The default Bluesky-branded theme.
    pub fn bluesky() -> Self {
        Self {
            bg: parse_hex_color("#0a0e1a"),
            fg: parse_hex_color("#e0e4ef"),
            accent: parse_hex_color("#0085ff"),
            secondary: parse_hex_color("#00c2ff"),
            border: parse_hex_color("#1e2d45"),
            muted: parse_hex_color("#5a6478"),
            error: parse_hex_color("#ff4444"),
            warning: parse_hex_color("#ffaa00"),
            success: parse_hex_color("#44cc66"),
            normal_bg: parse_hex_color("#0085ff"),
            insert_bg: parse_hex_color("#e8a040"),
            command_bg: parse_hex_color("#a0d0a0"),
            handle: parse_hex_color("#0085ff"),
            timestamp: parse_hex_color("#5a6478"),
            like: parse_hex_color("#ff6b8a"),
            repost: parse_hex_color("#44cc66"),
            reply: parse_hex_color("#00c2ff"),
        }
    }

    pub fn hacker() -> Self {
        Self {
            bg: parse_hex_color("#0a0a0a"),
            fg: parse_hex_color("#00ff00"),
            accent: parse_hex_color("#00ff00"),
            secondary: parse_hex_color("#00cc00"),
            border: parse_hex_color("#003300"),
            muted: parse_hex_color("#006600"),
            error: parse_hex_color("#ff0000"),
            warning: parse_hex_color("#ffaa00"),
            success: parse_hex_color("#00ff00"),
            normal_bg: parse_hex_color("#00ff00"),
            insert_bg: parse_hex_color("#ffaa00"),
            command_bg: parse_hex_color("#00ccff"),
            handle: parse_hex_color("#00ff00"),
            timestamp: parse_hex_color("#006600"),
            like: parse_hex_color("#ff0066"),
            repost: parse_hex_color("#00ff00"),
            reply: parse_hex_color("#00ccff"),
        }
    }

    pub fn catppuccin() -> Self {
        Self {
            bg: parse_hex_color("#1e1e2e"),
            fg: parse_hex_color("#cdd6f4"),
            accent: parse_hex_color("#89b4fa"),
            secondary: parse_hex_color("#74c7ec"),
            border: parse_hex_color("#313244"),
            muted: parse_hex_color("#6c7086"),
            error: parse_hex_color("#f38ba8"),
            warning: parse_hex_color("#f9e2af"),
            success: parse_hex_color("#a6e3a1"),
            normal_bg: parse_hex_color("#89b4fa"),
            insert_bg: parse_hex_color("#f9e2af"),
            command_bg: parse_hex_color("#a6e3a1"),
            handle: parse_hex_color("#89b4fa"),
            timestamp: parse_hex_color("#6c7086"),
            like: parse_hex_color("#f38ba8"),
            repost: parse_hex_color("#a6e3a1"),
            reply: parse_hex_color("#74c7ec"),
        }
    }

    pub fn nord() -> Self {
        Self {
            bg: parse_hex_color("#2e3440"),
            fg: parse_hex_color("#eceff4"),
            accent: parse_hex_color("#88c0d0"),
            secondary: parse_hex_color("#81a1c1"),
            border: parse_hex_color("#3b4252"),
            muted: parse_hex_color("#4c566a"),
            error: parse_hex_color("#bf616a"),
            warning: parse_hex_color("#ebcb8b"),
            success: parse_hex_color("#a3be8c"),
            normal_bg: parse_hex_color("#88c0d0"),
            insert_bg: parse_hex_color("#ebcb8b"),
            command_bg: parse_hex_color("#a3be8c"),
            handle: parse_hex_color("#88c0d0"),
            timestamp: parse_hex_color("#4c566a"),
            like: parse_hex_color("#bf616a"),
            repost: parse_hex_color("#a3be8c"),
            reply: parse_hex_color("#81a1c1"),
        }
    }

    /// Get a built-in theme by name.
    pub fn builtin(name: &str) -> Option<Self> {
        match name {
            "bluesky" => Some(Self::bluesky()),
            "hacker" => Some(Self::hacker()),
            "catppuccin" => Some(Self::catppuccin()),
            "nord" => Some(Self::nord()),
            _ => None,
        }
    }

    /// Create a theme from a ThemeDef, falling back to the bluesky theme for missing values.
    pub fn from_def(def: &ThemeDef) -> Self {
        let base = Self::bluesky();
        Self {
            bg: def.bg.as_deref().map(parse_hex_color).unwrap_or(base.bg),
            fg: def.fg.as_deref().map(parse_hex_color).unwrap_or(base.fg),
            accent: def
                .accent
                .as_deref()
                .map(parse_hex_color)
                .unwrap_or(base.accent),
            secondary: def
                .secondary
                .as_deref()
                .map(parse_hex_color)
                .unwrap_or(base.secondary),
            border: def
                .border
                .as_deref()
                .map(parse_hex_color)
                .unwrap_or(base.border),
            muted: def
                .muted
                .as_deref()
                .map(parse_hex_color)
                .unwrap_or(base.muted),
            error: def
                .error
                .as_deref()
                .map(parse_hex_color)
                .unwrap_or(base.error),
            warning: def
                .warning
                .as_deref()
                .map(parse_hex_color)
                .unwrap_or(base.warning),
            success: def
                .success
                .as_deref()
                .map(parse_hex_color)
                .unwrap_or(base.success),
            normal_bg: def
                .normal_bg
                .as_deref()
                .map(parse_hex_color)
                .unwrap_or(base.normal_bg),
            insert_bg: def
                .insert_bg
                .as_deref()
                .map(parse_hex_color)
                .unwrap_or(base.insert_bg),
            command_bg: def
                .command_bg
                .as_deref()
                .map(parse_hex_color)
                .unwrap_or(base.command_bg),
            handle: def
                .handle
                .as_deref()
                .map(parse_hex_color)
                .unwrap_or(base.handle),
            timestamp: def
                .timestamp
                .as_deref()
                .map(parse_hex_color)
                .unwrap_or(base.timestamp),
            like: def
                .like
                .as_deref()
                .map(parse_hex_color)
                .unwrap_or(base.like),
            repost: def
                .repost
                .as_deref()
                .map(parse_hex_color)
                .unwrap_or(base.repost),
            reply: def
                .reply
                .as_deref()
                .map(parse_hex_color)
                .unwrap_or(base.reply),
        }
    }
}

/// All available themes: built-ins + user-defined.
pub fn load_themes(custom: &HashMap<String, ThemeDef>) -> HashMap<String, Theme> {
    let mut themes = HashMap::new();
    themes.insert("bluesky".to_string(), Theme::bluesky());
    themes.insert("hacker".to_string(), Theme::hacker());
    themes.insert("catppuccin".to_string(), Theme::catppuccin());
    themes.insert("nord".to_string(), Theme::nord());

    for (name, def) in custom {
        themes.insert(name.clone(), Theme::from_def(def));
    }

    themes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#ff0000"), Color::Rgb(255, 0, 0));
        assert_eq!(parse_hex_color("#00ff00"), Color::Rgb(0, 255, 0));
        assert_eq!(parse_hex_color("#0000ff"), Color::Rgb(0, 0, 255));
        assert_eq!(parse_hex_color("0085ff"), Color::Rgb(0, 133, 255));
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        assert_eq!(parse_hex_color("invalid"), Color::White);
        assert_eq!(parse_hex_color("#xyz"), Color::White);
    }

    #[test]
    fn test_builtin_themes() {
        assert!(Theme::builtin("bluesky").is_some());
        assert!(Theme::builtin("hacker").is_some());
        assert!(Theme::builtin("catppuccin").is_some());
        assert!(Theme::builtin("nord").is_some());
        assert!(Theme::builtin("nonexistent").is_none());
    }

    #[test]
    fn test_theme_from_partial_def() {
        let def = ThemeDef {
            bg: Some("#ff0000".to_string()),
            ..Default::default()
        };
        let theme = Theme::from_def(&def);
        assert_eq!(theme.bg, Color::Rgb(255, 0, 0));
        // Other values should fall back to bluesky defaults.
        assert_eq!(theme.fg, Theme::bluesky().fg);
    }

    #[test]
    fn test_load_themes_includes_builtins() {
        let themes = load_themes(&HashMap::new());
        assert!(themes.contains_key("bluesky"));
        assert!(themes.contains_key("hacker"));
        assert!(themes.contains_key("catppuccin"));
        assert!(themes.contains_key("nord"));
    }

    #[test]
    fn test_load_themes_custom() {
        let mut custom = HashMap::new();
        custom.insert(
            "dracula".to_string(),
            ThemeDef {
                bg: Some("#282a36".to_string()),
                ..Default::default()
            },
        );
        let themes = load_themes(&custom);
        assert!(themes.contains_key("dracula"));
    }
}
