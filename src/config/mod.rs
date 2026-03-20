pub mod theme;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use theme::ThemeDef;

/// Top-level application config, loaded from TOML.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub feeds: FeedsConfig,
    #[serde(default)]
    pub workspaces: HashMap<String, WorkspaceLayout>,
    #[serde(default)]
    pub themes: HashMap<String, ThemeDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    #[serde(default = "default_toast_duration")]
    pub toast_duration_secs: u64,
    #[serde(default)]
    pub editor: Option<String>,
    #[serde(default = "default_image_protocol")]
    pub image_protocol: String,
}

fn default_theme() -> String {
    "bluesky".to_string()
}
fn default_poll_interval() -> u64 {
    30
}
fn default_toast_duration() -> u64 {
    5
}
fn default_image_protocol() -> String {
    "auto".to_string()
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            poll_interval_secs: default_poll_interval(),
            toast_duration_secs: default_toast_duration(),
            editor: None,
            image_protocol: default_image_protocol(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "default_auth_method")]
    pub method: String,
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub identifier: Option<String>,
    #[serde(default = "default_redirect_port")]
    pub redirect_port: u16,
    #[serde(default)]
    pub token_file: Option<String>,
}

fn default_auth_method() -> String {
    "app-password".to_string()
}
fn default_redirect_port() -> u16 {
    8420
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            method: default_auth_method(),
            service: None,
            identifier: None,
            redirect_port: default_redirect_port(),
            token_file: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedsConfig {
    #[serde(default = "default_feed_tabs")]
    pub tabs: Vec<FeedTabConfig>,
}

fn default_feed_tabs() -> Vec<FeedTabConfig> {
    vec![
        FeedTabConfig {
            name: "Following".to_string(),
            uri: "following".to_string(),
        },
        FeedTabConfig {
            name: "Discover".to_string(),
            uri: "discover".to_string(),
        },
    ]
}

impl Default for FeedsConfig {
    fn default() -> Self {
        Self {
            tabs: default_feed_tabs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedTabConfig {
    pub name: String,
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLayout {
    #[serde(default = "default_layout")]
    pub layout: String,
    #[serde(default = "default_ratio")]
    pub ratio: f64,
    #[serde(default)]
    pub left: Option<PaneConfig>,
    #[serde(default)]
    pub right: Option<PaneConfig>,
    #[serde(default)]
    pub top: Option<PaneConfig>,
    #[serde(default)]
    pub bottom: Option<PaneConfig>,
}

fn default_layout() -> String {
    "vsplit".to_string()
}
fn default_ratio() -> f64 {
    0.5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneConfig {
    #[serde(default)]
    pub pane: Option<String>,
    #[serde(default)]
    pub layout: Option<String>,
    #[serde(default)]
    pub ratio: Option<f64>,
    #[serde(default)]
    pub left: Option<Box<PaneConfig>>,
    #[serde(default)]
    pub right: Option<Box<PaneConfig>>,
    #[serde(default)]
    pub top: Option<Box<PaneConfig>>,
    #[serde(default)]
    pub bottom: Option<Box<PaneConfig>>,
}

/// Get the config directory path.
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("bmux")
}

/// Load config from the default location, or return defaults if no file exists.
pub fn load_config() -> AppConfig {
    let config_path = config_dir().join("config.toml");
    load_config_from_path(&config_path)
}

/// Load config from a specific path.
pub fn load_config_from_path(path: &std::path::Path) -> AppConfig {
    match std::fs::read_to_string(path) {
        Ok(contents) => match toml::from_str(&contents) {
            Ok(config) => config,
            Err(e) => {
                tracing::warn!("Failed to parse config file: {e}. Using defaults.");
                AppConfig::default()
            }
        },
        Err(_) => AppConfig::default(),
    }
}

/// Ensure the config directory exists.
pub fn ensure_config_dir() -> std::io::Result<PathBuf> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.general.theme, "bluesky");
        assert_eq!(config.general.poll_interval_secs, 30);
        assert_eq!(config.feeds.tabs.len(), 2);
        assert_eq!(config.feeds.tabs[0].name, "Following");
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
[general]
theme = "hacker"
"#;
        let config: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.general.theme, "hacker");
        assert_eq!(config.general.poll_interval_secs, 30); // default
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r##"
[general]
theme = "nord"
poll_interval_secs = 60
toast_duration_secs = 3
image_protocol = "kitty"

[auth]
method = "app-password"
service = "https://bsky.social"
identifier = "user.bsky.social"

[feeds]
tabs = [
    { name = "Following", uri = "following" },
    { name = "News", uri = "at://did:plc:xxx/app.bsky.feed.generator/news" },
]

[workspaces.home]
layout = "vsplit"
ratio = 0.65

[themes.custom]
bg = "#000000"
fg = "#ffffff"
"##;
        let config: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.general.theme, "nord");
        assert_eq!(config.general.poll_interval_secs, 60);
        assert_eq!(config.auth.method, "app-password");
        assert_eq!(config.feeds.tabs.len(), 2);
        assert!(config.workspaces.contains_key("home"));
        assert!(config.themes.contains_key("custom"));
    }

    #[test]
    fn test_load_nonexistent_config() {
        let config = load_config_from_path(std::path::Path::new("/nonexistent/config.toml"));
        assert_eq!(config.general.theme, "bluesky");
    }

    #[test]
    fn test_config_dir() {
        let dir = config_dir();
        assert!(dir.ends_with("bmux"));
    }

    #[test]
    fn test_default_auth_config() {
        let config = AppConfig::default();
        assert_eq!(config.auth.method, "app-password");
        assert_eq!(config.auth.redirect_port, 8420);
        assert!(config.auth.service.is_none());
        assert!(config.auth.identifier.is_none());
        assert!(config.auth.token_file.is_none());
    }

    #[test]
    fn test_parse_oauth_auth_config() {
        let toml = r#"
[auth]
method = "oauth"
service = "https://bsky.social"
identifier = "alice.bsky.social"
redirect_port = 9000
token_file = "/custom/path/tokens.json"
"#;
        let config: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.auth.method, "oauth");
        assert_eq!(config.auth.redirect_port, 9000);
        assert_eq!(
            config.auth.token_file.as_deref(),
            Some("/custom/path/tokens.json")
        );
        assert_eq!(config.auth.identifier.as_deref(), Some("alice.bsky.social"));
    }

    #[test]
    fn test_parse_auth_config_defaults_preserved() {
        let toml = r#"
[auth]
identifier = "bob.bsky.social"
"#;
        let config: AppConfig = toml::from_str(toml).unwrap();
        // Unspecified fields should have defaults.
        assert_eq!(config.auth.method, "app-password");
        assert_eq!(config.auth.redirect_port, 8420);
        assert!(config.auth.token_file.is_none());
    }
}
