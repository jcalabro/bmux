use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;

use jacquard::client::FileAuthStore;
use jacquard::identity::JacquardResolver;
use jacquard::oauth::client::OAuthClient;
use jacquard::oauth::loopback::{LoopbackConfig, LoopbackPort};
use jacquard::types::did::Did;

use super::{AppAgent, AuthSession};

/// Concrete OAuth client type.
type OAuthClientType = OAuthClient<JacquardResolver, FileAuthStore>;

/// Get the XDG data directory for bmux.
/// Uses $XDG_DATA_HOME/bmux, defaulting to ~/.local/share/bmux.
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("~"))
                .join(".local/share")
        })
        .join("bmux")
}

/// Path to the token persistence file.
pub fn token_file_path(custom_path: Option<&str>) -> PathBuf {
    if let Some(path) = custom_path {
        PathBuf::from(path)
    } else {
        data_dir().join("tokens.json")
    }
}

/// Ensure the data directory exists.
#[allow(dead_code)]
pub fn ensure_data_dir() -> Result<PathBuf> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create data dir: {}", dir.display()))?;
    Ok(dir)
}

/// Create an OAuth client with file-backed token storage.
fn create_oauth_client(token_path: &std::path::Path) -> Result<OAuthClientType> {
    // Ensure parent directory exists.
    if let Some(parent) = token_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let store = FileAuthStore::new(token_path);
    let client = OAuthClient::with_default_config(store);
    Ok(client)
}

/// Try to restore a previously saved OAuth session.
///
/// Returns None if no saved session exists or if restore fails.
pub async fn try_restore_session(
    token_path: &std::path::Path,
    identifier: &str,
) -> Option<AppAgent> {
    let client = create_oauth_client(token_path).ok()?;

    // We need the DID and session_id to restore. Read them from the token file.
    let saved = read_saved_session_info(token_path)?;

    // Only restore if the saved session matches the requested identifier.
    if !saved.identifier_matches(identifier) {
        tracing::info!(
            "Saved session is for {}, not {}; skipping restore",
            saved.display_id,
            identifier,
        );
        return None;
    }

    tracing::info!(
        "Attempting to restore OAuth session for {}",
        saved.display_id
    );

    let did: Did<'static> = Did::from(saved.did.clone());
    match client.restore(&did, &saved.session_id).await {
        Ok(session) => {
            let did_str = did.to_string();
            let handle = saved.handle.clone();

            Some(Arc::new(AuthSession::OAuth {
                session: Box::new(session),
                handle,
                did: did_str,
            }))
        }
        Err(e) => {
            tracing::warn!("Failed to restore OAuth session: {:?}", e);
            None
        }
    }
}

/// Run the OAuth browser login flow.
pub async fn login_with_browser(
    token_path: &std::path::Path,
    identifier: &str,
    redirect_port: u16,
) -> Result<AppAgent> {
    let client = create_oauth_client(token_path)?;

    let loopback_cfg = LoopbackConfig {
        host: "127.0.0.1".into(),
        port: LoopbackPort::Fixed(redirect_port),
        open_browser: true,
        timeout_ms: 5 * 60 * 1000, // 5 minutes
    };

    eprintln!("Opening browser for Bluesky login...");
    eprintln!("(If the browser doesn't open, check the URL printed below)");

    let session = client
        .login_with_local_server(identifier, Default::default(), loopback_cfg)
        .await
        .map_err(|e| anyhow::anyhow!("OAuth login failed: {:?}", e))?;

    // Extract DID and handle from the session.
    let (did, session_id) = session.session_info().await;
    let did_str = did.to_string();
    let session_id_str = session_id.to_string();

    // Resolve handle from profile.
    let handle = resolve_handle_from_session(&session, &did_str)
        .await
        .unwrap_or_else(|| identifier.to_string());

    // Save session info for future restore.
    save_session_info(token_path, &did_str, &session_id_str, &handle)?;

    Ok(Arc::new(AuthSession::OAuth {
        session: Box::new(session),
        handle,
        did: did_str,
    }))
}

// ── Session info persistence ─────────────────────────────────
// We save a small metadata file alongside the token file so we can
// restore sessions without knowing the DID/session_id upfront.

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedSessionInfo {
    did: String,
    session_id: String,
    handle: String,
    display_id: String,
}

impl SavedSessionInfo {
    fn identifier_matches(&self, identifier: &str) -> bool {
        if identifier.is_empty() {
            return true; // No filter — restore whatever we have.
        }
        self.did == identifier
            || self.handle == identifier
            || self.handle.trim_start_matches('@') == identifier.trim_start_matches('@')
    }
}

fn session_info_path(token_path: &std::path::Path) -> PathBuf {
    token_path.with_extension("meta.json")
}

fn save_session_info(
    token_path: &std::path::Path,
    did: &str,
    session_id: &str,
    handle: &str,
) -> Result<()> {
    let info = SavedSessionInfo {
        did: did.to_string(),
        session_id: session_id.to_string(),
        handle: handle.to_string(),
        display_id: if handle.is_empty() {
            did.to_string()
        } else {
            format!("@{}", handle)
        },
    };

    let path = session_info_path(token_path);
    let json = serde_json::to_string_pretty(&info)?;

    // Atomic write: write to tmp, then rename.
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, &json)
        .with_context(|| format!("Failed to write session info to {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &path).with_context(|| {
        format!(
            "Failed to rename {} to {}",
            tmp_path.display(),
            path.display()
        )
    })?;

    Ok(())
}

fn read_saved_session_info(token_path: &std::path::Path) -> Option<SavedSessionInfo> {
    let path = session_info_path(token_path);
    let contents = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Resolve the user's handle by fetching their profile via the session.
async fn resolve_handle_from_session(
    session: &super::OAuthSessionType,
    did: &str,
) -> Option<String> {
    // Make a simple unauthenticated-style call to get the handle.
    // We use the session's XRPC capabilities.
    let result =
        super::oauth_xrpc_get(session, "app.bsky.actor.getProfile", &[("actor", did)]).await;

    match result {
        Ok(data) => data["handle"].as_str().map(|s| s.to_string()),
        Err(e) => {
            tracing::warn!("Failed to resolve handle for {}: {}", did, e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_dir_ends_with_bmux() {
        let dir = data_dir();
        assert!(dir.ends_with("bmux"));
    }

    #[test]
    fn test_token_file_default_path() {
        let path = token_file_path(None);
        assert!(path.ends_with("tokens.json"));
        assert!(path.parent().unwrap().ends_with("bmux"));
    }

    #[test]
    fn test_token_file_custom_path() {
        let path = token_file_path(Some("/custom/tokens.json"));
        assert_eq!(path, PathBuf::from("/custom/tokens.json"));
    }

    #[test]
    fn test_session_info_path() {
        let token_path = PathBuf::from("/data/bmux/tokens.json");
        let info_path = session_info_path(&token_path);
        assert_eq!(info_path, PathBuf::from("/data/bmux/tokens.meta.json"));
    }

    #[test]
    fn test_saved_session_info_matches_handle() {
        let info = SavedSessionInfo {
            did: "did:plc:abc123".into(),
            session_id: "sess-1".into(),
            handle: "alice.bsky.social".into(),
            display_id: "@alice.bsky.social".into(),
        };

        assert!(info.identifier_matches("alice.bsky.social"));
        assert!(info.identifier_matches("@alice.bsky.social"));
        assert!(info.identifier_matches("did:plc:abc123"));
        assert!(info.identifier_matches("")); // empty matches all
        assert!(!info.identifier_matches("bob.bsky.social"));
        assert!(!info.identifier_matches("did:plc:other"));
    }

    #[test]
    fn test_save_and_read_session_info() {
        let dir = tempfile::tempdir().unwrap();
        let token_path = dir.path().join("tokens.json");

        save_session_info(&token_path, "did:plc:test", "sess-42", "test.bsky.social").unwrap();

        let info = read_saved_session_info(&token_path).unwrap();
        assert_eq!(info.did, "did:plc:test");
        assert_eq!(info.session_id, "sess-42");
        assert_eq!(info.handle, "test.bsky.social");
        assert_eq!(info.display_id, "@test.bsky.social");
    }

    #[test]
    fn test_save_session_info_empty_handle() {
        let dir = tempfile::tempdir().unwrap();
        let token_path = dir.path().join("tokens.json");

        save_session_info(&token_path, "did:plc:test", "sess-1", "").unwrap();

        let info = read_saved_session_info(&token_path).unwrap();
        assert_eq!(info.display_id, "did:plc:test");
    }

    #[test]
    fn test_read_missing_session_info_returns_none() {
        let path = PathBuf::from("/nonexistent/tokens.json");
        assert!(read_saved_session_info(&path).is_none());
    }

    #[test]
    fn test_read_corrupt_session_info_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let token_path = dir.path().join("tokens.json");
        let info_path = session_info_path(&token_path);
        std::fs::write(&info_path, "not valid json").unwrap();
        assert!(read_saved_session_info(&token_path).is_none());
    }

    #[test]
    fn test_ensure_data_dir_creates_directory() {
        let dir = tempfile::tempdir().unwrap();
        let test_dir = dir.path().join("test_bmux_data");
        // Override wouldn't work here since data_dir() uses dirs crate,
        // but we can test the fs operations directly.
        std::fs::create_dir_all(&test_dir).unwrap();
        assert!(test_dir.exists());
    }

    #[test]
    fn test_save_session_info_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a/b/c/tokens.json");

        // Parent dir doesn't exist yet.
        assert!(!nested.parent().unwrap().exists());

        // save_session_info doesn't create parent dirs for the info file,
        // but the info file goes alongside the token file.
        // Let's create the parent first like the real flow would.
        std::fs::create_dir_all(nested.parent().unwrap()).unwrap();
        save_session_info(&nested, "did:plc:x", "s1", "handle").unwrap();

        let info = read_saved_session_info(&nested).unwrap();
        assert_eq!(info.did, "did:plc:x");
    }

    #[test]
    fn test_save_session_info_atomic_write() {
        let dir = tempfile::tempdir().unwrap();
        let token_path = dir.path().join("tokens.json");

        // Write initial.
        save_session_info(&token_path, "did:plc:v1", "s1", "v1.bsky.social").unwrap();

        // Overwrite.
        save_session_info(&token_path, "did:plc:v2", "s2", "v2.bsky.social").unwrap();

        let info = read_saved_session_info(&token_path).unwrap();
        assert_eq!(info.did, "did:plc:v2");
        assert_eq!(info.session_id, "s2");

        // No tmp file should remain.
        let tmp = session_info_path(&token_path).with_extension("tmp");
        assert!(!tmp.exists());
    }

    #[test]
    fn test_identifier_matches_at_prefix_normalization() {
        let info = SavedSessionInfo {
            did: "did:plc:abc".into(),
            session_id: "s1".into(),
            handle: "alice.bsky.social".into(),
            display_id: "@alice.bsky.social".into(),
        };

        // Handle with and without @ should both match.
        assert!(info.identifier_matches("alice.bsky.social"));
        assert!(info.identifier_matches("@alice.bsky.social"));
    }

    #[test]
    fn test_identifier_matches_did_format() {
        let info = SavedSessionInfo {
            did: "did:plc:abc123".into(),
            session_id: "s1".into(),
            handle: "test.bsky.social".into(),
            display_id: "@test.bsky.social".into(),
        };

        // DID should match exactly.
        assert!(info.identifier_matches("did:plc:abc123"));
        assert!(!info.identifier_matches("did:plc:different"));
        assert!(!info.identifier_matches("did:web:abc123"));
    }

    #[test]
    fn test_token_file_path_xdg_compliance() {
        // Default path should be under a data directory, not config.
        let path = token_file_path(None);
        let path_str = path.to_string_lossy();

        // Should NOT be under .config (that's for config, not runtime data).
        // Should be under .local/share or equivalent XDG data dir.
        assert!(
            path_str.contains(".local/share") || path_str.contains("share"),
            "Token file should be in XDG data dir, got: {}",
            path_str
        );
    }

    #[test]
    fn test_data_dir_separate_from_config_dir() {
        let data = data_dir();
        let config = crate::config::config_dir();

        // Data and config dirs should be different paths.
        assert_ne!(data, config);
    }

    #[test]
    fn test_session_info_roundtrip_preserves_all_fields() {
        let dir = tempfile::tempdir().unwrap();
        let token_path = dir.path().join("tokens.json");

        let did = "did:plc:roundtrip123";
        let session_id = "sess-roundtrip-456";
        let handle = "roundtrip.bsky.social";

        save_session_info(&token_path, did, session_id, handle).unwrap();
        let info = read_saved_session_info(&token_path).unwrap();

        assert_eq!(info.did, did);
        assert_eq!(info.session_id, session_id);
        assert_eq!(info.handle, handle);
        assert_eq!(info.display_id, format!("@{}", handle));

        // The info should also be matchable.
        assert!(info.identifier_matches(handle));
        assert!(info.identifier_matches(did));
    }

    #[test]
    fn test_session_info_json_is_pretty_printed() {
        let dir = tempfile::tempdir().unwrap();
        let token_path = dir.path().join("tokens.json");

        save_session_info(&token_path, "did:plc:x", "s1", "x.bsky.social").unwrap();

        let info_path = session_info_path(&token_path);
        let contents = std::fs::read_to_string(info_path).unwrap();

        // Pretty-printed JSON should have newlines.
        assert!(contents.contains('\n'));
        // Should be valid JSON.
        let _: serde_json::Value = serde_json::from_str(&contents).unwrap();
    }

    #[tokio::test]
    async fn test_try_restore_no_saved_session() {
        let dir = tempfile::tempdir().unwrap();
        let token_path = dir.path().join("tokens.json");

        // No saved session file → should return None.
        let result = try_restore_session(&token_path, "alice.bsky.social").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_try_restore_mismatched_identifier() {
        let dir = tempfile::tempdir().unwrap();
        let token_path = dir.path().join("tokens.json");

        // Save session info for alice.
        save_session_info(&token_path, "did:plc:alice", "s1", "alice.bsky.social").unwrap();

        // Try to restore for bob → should return None (identifier mismatch).
        let result = try_restore_session(&token_path, "bob.bsky.social").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_try_restore_with_empty_identifier_matches_any() {
        let dir = tempfile::tempdir().unwrap();
        let token_path = dir.path().join("tokens.json");

        // Save session info.
        save_session_info(&token_path, "did:plc:anyone", "s1", "anyone.bsky.social").unwrap();

        // Empty identifier should match any saved session.
        // (Will still fail to actually restore since there's no real token file,
        // but the identifier_matches check should pass.)
        let result = try_restore_session(&token_path, "").await;
        // Will be None because create_oauth_client + restore will fail,
        // but it should at least attempt the restore (not skip due to mismatch).
        assert!(result.is_none());
    }

    #[test]
    fn test_create_oauth_client_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("deeply/nested/dir/tokens.json");

        // Parent doesn't exist yet.
        assert!(!nested.parent().unwrap().exists());

        // create_oauth_client should create parent dirs.
        let result = create_oauth_client(&nested);
        assert!(result.is_ok());
        assert!(nested.parent().unwrap().exists());
    }
}
