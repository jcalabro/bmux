pub mod client;
pub mod types;

use crate::auth::AppAgent;
use crate::messages::{ApiRequest, ApiResponse, AppMessage};
use tokio::sync::mpsc;

/// API task: receives requests, dispatches them to jacquard, sends responses back.
pub async fn run_api_task(
    agent: AppAgent,
    mut rx: mpsc::Receiver<ApiRequest>,
    tx: mpsc::Sender<AppMessage>,
) {
    while let Some(request) = rx.recv().await {
        let agent = agent.clone();
        let tx = tx.clone();

        tokio::spawn(async move {
            let response = handle_request(&agent, request).await;
            let _ = tx.send(AppMessage::Api(response)).await;
        });
    }
}

async fn handle_request(agent: &AppAgent, request: ApiRequest) -> ApiResponse {
    match request {
        ApiRequest::FetchTimeline { cursor } => {
            match client::fetch_timeline(agent, cursor.as_deref()).await {
                Ok((posts, next_cursor)) => ApiResponse::Timeline {
                    posts,
                    cursor: next_cursor,
                },
                Err(e) => ApiResponse::Error {
                    request_description: "fetch timeline".into(),
                    error: e.to_string(),
                },
            }
        }
        ApiRequest::FetchThread { uri } => {
            match client::fetch_thread(agent, &uri).await {
                Ok(thread) => ApiResponse::Thread { uri, thread },
                Err(e) => ApiResponse::Error {
                    request_description: format!("fetch thread {}", uri),
                    error: e.to_string(),
                },
            }
        }
        ApiRequest::FetchProfile { actor } => {
            match client::fetch_profile(agent, &actor).await {
                Ok(profile) => ApiResponse::Profile(profile),
                Err(e) => ApiResponse::Error {
                    request_description: format!("fetch profile {}", actor),
                    error: e.to_string(),
                },
            }
        }
        ApiRequest::CreatePost { text, reply_to } => {
            match client::create_post(agent, &text, reply_to.as_ref()).await {
                Ok(uri) => ApiResponse::PostCreated { uri },
                Err(e) => ApiResponse::Error {
                    request_description: "create post".into(),
                    error: e.to_string(),
                },
            }
        }
        ApiRequest::LikePost { uri, cid } => {
            match client::like_post(agent, &uri, &cid).await {
                Ok(like_uri) => ApiResponse::PostLiked {
                    post_uri: uri,
                    like_uri,
                },
                Err(e) => ApiResponse::Error {
                    request_description: format!("like post {}", uri),
                    error: e.to_string(),
                },
            }
        }
        ApiRequest::UnlikePost { like_uri } => {
            match client::unlike_post(agent, &like_uri).await {
                Ok(()) => ApiResponse::PostUnliked {
                    post_uri: like_uri,
                },
                Err(e) => ApiResponse::Error {
                    request_description: "unlike post".into(),
                    error: e.to_string(),
                },
            }
        }
        ApiRequest::RepostPost { uri, cid } => {
            match client::repost_post(agent, &uri, &cid).await {
                Ok(repost_uri) => ApiResponse::PostReposted {
                    post_uri: uri,
                    repost_uri,
                },
                Err(e) => ApiResponse::Error {
                    request_description: format!("repost {}", uri),
                    error: e.to_string(),
                },
            }
        }
        ApiRequest::FetchNotifications { cursor } => {
            match client::fetch_notifications(agent, cursor.as_deref()).await {
                Ok((notifications, next_cursor, unread_count)) => ApiResponse::Notifications {
                    notifications,
                    cursor: next_cursor,
                    unread_count,
                },
                Err(e) => ApiResponse::Error {
                    request_description: "fetch notifications".into(),
                    error: e.to_string(),
                },
            }
        }
        ApiRequest::FetchConversations { cursor } => {
            match client::fetch_conversations(agent, cursor.as_deref()).await {
                Ok((conversations, next_cursor)) => ApiResponse::Conversations {
                    conversations,
                    cursor: next_cursor,
                },
                Err(e) => ApiResponse::Error {
                    request_description: "fetch conversations".into(),
                    error: e.to_string(),
                },
            }
        }
        ApiRequest::SendMessage { convo_id, text } => {
            match client::send_message(agent, &convo_id, &text).await {
                Ok(()) => ApiResponse::MessageSent { convo_id },
                Err(e) => ApiResponse::Error {
                    request_description: "send message".into(),
                    error: e.to_string(),
                },
            }
        }
        ApiRequest::FollowUser { did } => {
            match client::follow_user(agent, &did).await {
                Ok(follow_uri) => ApiResponse::UserFollowed { did, follow_uri },
                Err(e) => ApiResponse::Error {
                    request_description: "follow user".into(),
                    error: e.to_string(),
                },
            }
        }
        ApiRequest::SearchPosts { query, cursor } => {
            match client::search_posts(agent, &query, cursor.as_deref()).await {
                Ok((posts, next_cursor)) => ApiResponse::SearchResults {
                    query,
                    posts,
                    cursor: next_cursor,
                },
                Err(e) => ApiResponse::Error {
                    request_description: "search posts".into(),
                    error: e.to_string(),
                },
            }
        }
        _ => ApiResponse::Error {
            request_description: "unimplemented request".into(),
            error: "Not yet implemented".into(),
        },
    }
}
