pub mod oauth;

use anyhow::Result;
use std::sync::Arc;

/// Opaque agent handle. For now we store the raw session data
/// and make API calls through the jacquard agent.
pub struct BlueskyAgent {
    pub handle: String,
    pub did: String,
    pub access_jwt: String,
    pub service: String,
}

pub type AppAgent = Arc<BlueskyAgent>;

/// Login with an app password.
pub async fn login_with_app_password(
    service: &str,
    identifier: &str,
    password: &str,
) -> Result<AppAgent> {
    // Use jacquard to authenticate, then extract the session tokens.
    use jacquard::client::MemoryCredentialSession;

    let (_session, atp_session) = MemoryCredentialSession::authenticated(
        identifier.into(),
        password.into(),
        None,
        None,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Login failed: {:?}", e))?;

    Ok(Arc::new(BlueskyAgent {
        handle: atp_session.handle.to_string(),
        did: atp_session.did.to_string(),
        access_jwt: atp_session.access_jwt.to_string(),
        service: service.to_string(),
    }))
}
