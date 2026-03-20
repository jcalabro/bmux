use crate::auth::AppAgent;
use crate::messages::*;
use anyhow::{bail, Result};
use serde_json::Value;

/// Make an authenticated GET request to the Bluesky XRPC API.
async fn xrpc_get(agent: &AppAgent, method: &str, params: &[(&str, &str)]) -> Result<Value> {
    let client = reqwest::Client::new();
    let url = format!("{}/xrpc/{}", agent.service, method);

    let resp = client
        .get(&url)
        .bearer_auth(&agent.access_jwt)
        .query(params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("XRPC {} failed ({}): {}", method, status, body);
    }

    Ok(resp.json().await?)
}

/// Make an authenticated POST request.
async fn xrpc_post(agent: &AppAgent, method: &str, body: &Value) -> Result<Value> {
    let client = reqwest::Client::new();
    let url = format!("{}/xrpc/{}", agent.service, method);

    let resp = client
        .post(&url)
        .bearer_auth(&agent.access_jwt)
        .json(body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("XRPC {} failed ({}): {}", method, status, body);
    }

    Ok(resp.json().await?)
}

/// Fetch the home timeline.
pub async fn fetch_timeline(
    agent: &AppAgent,
    cursor: Option<&str>,
) -> Result<(Vec<Post>, Option<String>)> {
    let mut params = vec![("limit", "50")];
    let cursor_owned;
    if let Some(c) = cursor {
        cursor_owned = c.to_string();
        params.push(("cursor", &cursor_owned));
    }

    let data = xrpc_get(agent, "app.bsky.feed.getTimeline", &params).await?;

    let posts = data["feed"]
        .as_array()
        .map(|arr| arr.iter().filter_map(parse_feed_view_post).collect())
        .unwrap_or_default();

    let next_cursor = data["cursor"].as_str().map(|s| s.to_string());
    Ok((posts, next_cursor))
}

/// Fetch a post thread.
pub async fn fetch_thread(agent: &AppAgent, uri: &str) -> Result<PostThread> {
    let data = xrpc_get(
        agent,
        "app.bsky.feed.getPostThread",
        &[("uri", uri), ("depth", "6")],
    )
    .await?;

    parse_thread(&data["thread"]).ok_or_else(|| anyhow::anyhow!("Failed to parse thread"))
}

/// Like a post.
pub async fn like_post(agent: &AppAgent, uri: &str, cid: &str) -> Result<String> {
    let body = build_like_record(&agent.did, uri, cid);
    let resp = xrpc_post(agent, "com.atproto.repo.createRecord", &body).await?;
    Ok(resp["uri"].as_str().unwrap_or("").to_string())
}

/// Unlike a post.
pub async fn unlike_post(agent: &AppAgent, like_uri: &str) -> Result<()> {
    let body = build_delete_record(&agent.did, "app.bsky.feed.like", like_uri)?;
    xrpc_post(agent, "com.atproto.repo.deleteRecord", &body).await?;
    Ok(())
}

/// Repost a post.
pub async fn repost_post(agent: &AppAgent, uri: &str, cid: &str) -> Result<String> {
    let body = build_repost_record(&agent.did, uri, cid);
    let resp = xrpc_post(agent, "com.atproto.repo.createRecord", &body).await?;
    Ok(resp["uri"].as_str().unwrap_or("").to_string())
}

/// Create a new post.
pub async fn create_post(
    agent: &AppAgent,
    text: &str,
    reply_to: Option<&ReplyRef>,
    quote: Option<&QuoteRef>,
) -> Result<String> {
    let body = build_post_record(&agent.did, text, reply_to, quote);
    let resp = xrpc_post(agent, "com.atproto.repo.createRecord", &body).await?;
    Ok(resp["uri"].as_str().unwrap_or("").to_string())
}

/// Fetch notifications.
pub async fn fetch_notifications(
    agent: &AppAgent,
    cursor: Option<&str>,
) -> Result<(Vec<Notification>, Option<String>, usize)> {
    let mut params = vec![("limit", "50")];
    let cursor_owned;
    if let Some(c) = cursor {
        cursor_owned = c.to_string();
        params.push(("cursor", &cursor_owned));
    }

    let data = xrpc_get(agent, "app.bsky.notification.listNotifications", &params).await?;

    let notifications: Vec<Notification> = data["notifications"]
        .as_array()
        .map(|arr| arr.iter().filter_map(parse_notification).collect())
        .unwrap_or_default();

    let unread = notifications.iter().filter(|n| !n.is_read).count();
    let next_cursor = data["cursor"].as_str().map(|s| s.to_string());
    Ok((notifications, next_cursor, unread))
}

/// Fetch conversations.
pub async fn fetch_conversations(
    agent: &AppAgent,
    cursor: Option<&str>,
) -> Result<(Vec<Conversation>, Option<String>)> {
    let mut params = vec![("limit", "50")];
    let cursor_owned;
    if let Some(c) = cursor {
        cursor_owned = c.to_string();
        params.push(("cursor", &cursor_owned));
    }

    let data = xrpc_get(agent, "chat.bsky.convo.listConvos", &params).await?;

    let convos: Vec<Conversation> = data["convos"]
        .as_array()
        .map(|arr| arr.iter().filter_map(parse_conversation).collect())
        .unwrap_or_default();

    let next_cursor = data["cursor"].as_str().map(|s| s.to_string());
    Ok((convos, next_cursor))
}

/// Send a DM.
pub async fn send_message(agent: &AppAgent, convo_id: &str, text: &str) -> Result<()> {
    let body = build_send_message_body(convo_id, text);
    xrpc_post(agent, "chat.bsky.convo.sendMessage", &body).await?;
    Ok(())
}

/// Follow a user.
pub async fn follow_user(agent: &AppAgent, did: &str) -> Result<String> {
    let body = build_follow_record(&agent.did, did);
    let resp = xrpc_post(agent, "com.atproto.repo.createRecord", &body).await?;
    Ok(resp["uri"].as_str().unwrap_or("").to_string())
}

/// Fetch profile.
pub async fn fetch_profile(agent: &AppAgent, actor: &str) -> Result<ProfileData> {
    let data = xrpc_get(agent, "app.bsky.actor.getProfile", &[("actor", actor)]).await?;

    Ok(ProfileData {
        did: data["did"].as_str().unwrap_or("").to_string(),
        handle: data["handle"].as_str().unwrap_or("").to_string(),
        display_name: data["displayName"].as_str().map(|s| s.to_string()),
        description: data["description"].as_str().map(|s| s.to_string()),
        avatar_url: data["avatar"].as_str().map(|s| s.to_string()),
        banner_url: data["banner"].as_str().map(|s| s.to_string()),
        followers_count: data["followersCount"].as_u64().unwrap_or(0),
        follows_count: data["followsCount"].as_u64().unwrap_or(0),
        posts_count: data["postsCount"].as_u64().unwrap_or(0),
        following_me: data["viewer"]["followedBy"].is_string(),
        followed_by_me: data["viewer"]["following"].as_str().map(|s| s.to_string()),
        muted: data["viewer"]["muted"].as_bool().unwrap_or(false),
        blocked: data["viewer"]["blocking"].is_string(),
    })
}

/// Search posts.
pub async fn search_posts(
    agent: &AppAgent,
    query: &str,
    cursor: Option<&str>,
) -> Result<(Vec<Post>, Option<String>)> {
    let mut params = vec![("q", query), ("limit", "25")];
    let cursor_owned;
    if let Some(c) = cursor {
        cursor_owned = c.to_string();
        params.push(("cursor", &cursor_owned));
    }

    let data = xrpc_get(agent, "app.bsky.feed.searchPosts", &params).await?;

    let posts = data["posts"]
        .as_array()
        .map(|arr| arr.iter().filter_map(parse_post_view).collect())
        .unwrap_or_default();

    let next_cursor = data["cursor"].as_str().map(|s| s.to_string());
    Ok((posts, next_cursor))
}

// ── JSON parsing helpers ─────────────────────────────────────

fn parse_author(v: &Value) -> Author {
    Author {
        did: v["did"].as_str().unwrap_or("").to_string(),
        handle: v["handle"].as_str().unwrap_or("").to_string(),
        display_name: v["displayName"].as_str().map(|s| s.to_string()),
        avatar_url: v["avatar"].as_str().map(|s| s.to_string()),
    }
}

fn parse_post_view(v: &Value) -> Option<Post> {
    let uri = v["uri"].as_str()?;
    let cid = v["cid"].as_str()?;

    let text = v["record"]["text"].as_str().unwrap_or("").to_string();
    let facets = parse_facets(&v["record"]["facets"]);
    let embed = parse_embed(&v["embed"]);

    // Parse the post's own reply ref from the record data.
    let reply_to = parse_reply_ref(&v["record"]["reply"]);

    Some(Post {
        uri: uri.to_string(),
        cid: cid.to_string(),
        author: parse_author(&v["author"]),
        text,
        facets,
        created_at: v["indexedAt"].as_str().unwrap_or("").to_string(),
        like_count: v["likeCount"].as_u64().unwrap_or(0),
        repost_count: v["repostCount"].as_u64().unwrap_or(0),
        reply_count: v["replyCount"].as_u64().unwrap_or(0),
        liked_by_me: v["viewer"]["like"].as_str().map(|s| s.to_string()),
        reposted_by_me: v["viewer"]["repost"].as_str().map(|s| s.to_string()),
        reply_to,
        reply_context: None,
        embed,
        reposted_by: None,
    })
}

fn parse_feed_view_post(v: &Value) -> Option<Post> {
    let mut post = parse_post_view(&v["post"])?;

    // Repost reason.
    if let Some(reason) = v["reason"].as_object()
        && reason
            .get("$type")
            .and_then(|t| t.as_str())
            .map(|t| t.contains("reasonRepost"))
            .unwrap_or(false)
    {
        post.reposted_by = Some(parse_author(&v["reason"]["by"]));
    }

    // Reply context: extract parent post info so we can show
    // "replying to @handle: <text>" inline.
    if v["reply"].is_object() {
        let parent = &v["reply"]["parent"];
        if parent.is_object() && parent["author"].is_object() {
            let parent_text = parent["record"]["text"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let root = &v["reply"]["root"];
            let root_author = if root.is_object()
                && root["author"].is_object()
                && root["uri"].as_str() != parent["uri"].as_str()
            {
                Some(parse_author(&root["author"]))
            } else {
                None
            };
            post.reply_context = Some(ReplyContext {
                parent_author: parse_author(&parent["author"]),
                parent_text,
                root_author,
            });
        }
    }

    Some(post)
}

fn parse_facets(v: &Value) -> Vec<Facet> {
    v.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|f| {
                    let start = f["index"]["byteStart"].as_u64()? as usize;
                    let end = f["index"]["byteEnd"].as_u64()? as usize;
                    let feature = f["features"].as_array()?.first()?;
                    let type_str = feature["$type"].as_str()?;

                    let kind = if type_str.contains("mention") {
                        FacetKind::Mention {
                            did: feature["did"].as_str()?.to_string(),
                        }
                    } else if type_str.contains("link") {
                        FacetKind::Link {
                            uri: feature["uri"].as_str()?.to_string(),
                        }
                    } else if type_str.contains("tag") {
                        FacetKind::Tag {
                            tag: feature["tag"].as_str()?.to_string(),
                        }
                    } else {
                        return None;
                    };

                    Some(Facet { start, end, kind })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_embed(v: &Value) -> Option<PostEmbed> {
    let type_str = v["$type"].as_str()?;

    if type_str.contains("images") {
        Some(PostEmbed::Images(parse_embed_images(v)))
    } else if type_str.contains("external") {
        Some(PostEmbed::External {
            uri: v["external"]["uri"].as_str().unwrap_or("").to_string(),
            title: v["external"]["title"].as_str().unwrap_or("").to_string(),
            description: v["external"]["description"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        })
    } else if type_str.contains("recordWithMedia") {
        // Quote post with images attached.
        let record = parse_quoted_post(&v["record"])?;
        let images = if v["media"]["$type"]
            .as_str()
            .map(|t| t.contains("images"))
            .unwrap_or(false)
        {
            parse_embed_images(&v["media"])
        } else {
            vec![]
        };
        Some(PostEmbed::RecordWithMedia { record, images })
    } else if type_str.contains("record") {
        // Quote post.
        let qp = parse_quoted_post(&v["record"])?;
        Some(PostEmbed::Record(qp))
    } else {
        None
    }
}

fn parse_embed_images(v: &Value) -> Vec<EmbedImage> {
    v["images"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|img| EmbedImage {
                    thumb_url: img["thumb"].as_str().unwrap_or("").to_string(),
                    fullsize_url: img["fullsize"].as_str().unwrap_or("").to_string(),
                    alt: img["alt"].as_str().unwrap_or("").to_string(),
                    width: img["aspectRatio"]["width"].as_u64().map(|n| n as u32),
                    height: img["aspectRatio"]["height"].as_u64().map(|n| n as u32),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_quoted_post(v: &Value) -> Option<QuotedPost> {
    // The embed record view has the post at v directly, with author, value (record), etc.
    // For app.bsky.embed.record#view, the structure is:
    //   record: { $type, uri, cid, author: {}, value: { text, ... }, ... }
    let author = if v["author"].is_object() {
        parse_author(&v["author"])
    } else {
        return None;
    };

    let text = v["value"]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Some(QuotedPost {
        uri: v["uri"].as_str().unwrap_or("").to_string(),
        author,
        text,
        created_at: v["indexedAt"].as_str().unwrap_or("").to_string(),
    })
}

fn parse_reply_ref(v: &Value) -> Option<ReplyRef> {
    if !v.is_object() {
        return None;
    }
    Some(ReplyRef {
        root_uri: v["root"]["uri"].as_str()?.to_string(),
        root_cid: v["root"]["cid"].as_str()?.to_string(),
        parent_uri: v["parent"]["uri"].as_str()?.to_string(),
        parent_cid: v["parent"]["cid"].as_str()?.to_string(),
    })
}

fn parse_thread(v: &Value) -> Option<PostThread> {
    let post = parse_post_view(&v["post"])?;

    let parent = if v["parent"].is_object() {
        parse_thread(&v["parent"]).map(Box::new)
    } else {
        None
    };

    let replies = v["replies"]
        .as_array()
        .map(|arr| arr.iter().filter_map(parse_thread).collect())
        .unwrap_or_default();

    Some(PostThread {
        post,
        parent,
        replies,
    })
}

fn parse_notification(v: &Value) -> Option<Notification> {
    let reason = match v["reason"].as_str()? {
        "like" => NotificationReason::Like,
        "repost" => NotificationReason::Repost,
        "follow" => NotificationReason::Follow,
        "mention" => NotificationReason::Mention,
        "reply" => NotificationReason::Reply,
        "quote" => NotificationReason::Quote,
        _ => NotificationReason::Like,
    };

    Some(Notification {
        uri: v["uri"].as_str()?.to_string(),
        cid: v["cid"].as_str().unwrap_or("").to_string(),
        author: parse_author(&v["author"]),
        reason,
        subject_uri: v["reasonSubject"].as_str().map(|s| s.to_string()),
        created_at: v["indexedAt"].as_str().unwrap_or("").to_string(),
        is_read: v["isRead"].as_bool().unwrap_or(false),
    })
}

fn parse_conversation(v: &Value) -> Option<Conversation> {
    let id = v["id"].as_str()?.to_string();
    let members = v["members"]
        .as_array()
        .map(|arr| arr.iter().map(parse_author).collect())
        .unwrap_or_default();

    Some(Conversation {
        id,
        members,
        last_message: None,
        unread_count: v["unreadCount"].as_u64().unwrap_or(0),
        muted: v["muted"].as_bool().unwrap_or(false),
    })
}

// ── Record builders (pure functions, testable without network) ────

/// Build a createRecord body for a like.
fn build_like_record(repo: &str, uri: &str, cid: &str) -> Value {
    serde_json::json!({
        "repo": repo,
        "collection": "app.bsky.feed.like",
        "record": {
            "$type": "app.bsky.feed.like",
            "subject": { "uri": uri, "cid": cid },
            "createdAt": chrono::Utc::now().to_rfc3339()
        }
    })
}

/// Build a createRecord body for a repost.
fn build_repost_record(repo: &str, uri: &str, cid: &str) -> Value {
    serde_json::json!({
        "repo": repo,
        "collection": "app.bsky.feed.repost",
        "record": {
            "$type": "app.bsky.feed.repost",
            "subject": { "uri": uri, "cid": cid },
            "createdAt": chrono::Utc::now().to_rfc3339()
        }
    })
}

/// Build a createRecord body for a post (with optional reply and quote).
fn build_post_record(
    repo: &str,
    text: &str,
    reply_to: Option<&ReplyRef>,
    quote: Option<&QuoteRef>,
) -> Value {
    let mut record = serde_json::json!({
        "$type": "app.bsky.feed.post",
        "text": text,
        "createdAt": chrono::Utc::now().to_rfc3339()
    });

    if let Some(reply) = reply_to {
        record["reply"] = serde_json::json!({
            "root": { "uri": reply.root_uri, "cid": reply.root_cid },
            "parent": { "uri": reply.parent_uri, "cid": reply.parent_cid }
        });
    }

    if let Some(quote) = quote {
        record["embed"] = serde_json::json!({
            "$type": "app.bsky.embed.record",
            "record": { "uri": quote.uri, "cid": quote.cid }
        });
    }

    serde_json::json!({
        "repo": repo,
        "collection": "app.bsky.feed.post",
        "record": record
    })
}

/// Build a createRecord body for a follow.
fn build_follow_record(repo: &str, subject_did: &str) -> Value {
    serde_json::json!({
        "repo": repo,
        "collection": "app.bsky.graph.follow",
        "record": {
            "$type": "app.bsky.graph.follow",
            "subject": subject_did,
            "createdAt": chrono::Utc::now().to_rfc3339()
        }
    })
}

/// Build a deleteRecord body (for unlike, unrepost, unfollow, etc.).
fn build_delete_record(repo: &str, collection: &str, record_uri: &str) -> Result<Value> {
    let parts: Vec<&str> = record_uri.split('/').collect();
    let rkey = parts.last().ok_or_else(|| anyhow::anyhow!("Invalid record URI: {}", record_uri))?;
    Ok(serde_json::json!({
        "repo": repo,
        "collection": collection,
        "rkey": rkey
    }))
}

/// Build a block record.
#[cfg(test)]
fn build_block_record(repo: &str, subject_did: &str) -> Value {
    serde_json::json!({
        "repo": repo,
        "collection": "app.bsky.graph.block",
        "record": {
            "$type": "app.bsky.graph.block",
            "subject": subject_did,
            "createdAt": chrono::Utc::now().to_rfc3339()
        }
    })
}

/// Build a mute request body.
#[cfg(test)]
fn build_mute_body(did: &str) -> Value {
    serde_json::json!({
        "actor": did
    })
}

/// Build a send message body.
fn build_send_message_body(convo_id: &str, text: &str) -> Value {
    serde_json::json!({
        "convoId": convo_id,
        "message": { "text": text }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_DID: &str = "did:plc:testuser123";
    const TEST_POST_URI: &str = "at://did:plc:author/app.bsky.feed.post/abc123";
    const TEST_POST_CID: &str = "bafyreih2fedtl3ogpepfqntmz5ky6rv";
    const TEST_LIKE_URI: &str = "at://did:plc:testuser123/app.bsky.feed.like/xyz789";

    // ── Like records ────────────────────────────────────────

    #[test]
    fn test_like_record_structure() {
        let body = build_like_record(TEST_DID, TEST_POST_URI, TEST_POST_CID);
        assert_eq!(body["repo"], TEST_DID);
        assert_eq!(body["collection"], "app.bsky.feed.like");
        assert_eq!(body["record"]["$type"], "app.bsky.feed.like");
        assert_eq!(body["record"]["subject"]["uri"], TEST_POST_URI);
        assert_eq!(body["record"]["subject"]["cid"], TEST_POST_CID);
        assert!(body["record"]["createdAt"].is_string());
    }

    #[test]
    fn test_like_record_has_iso_timestamp() {
        let body = build_like_record(TEST_DID, TEST_POST_URI, TEST_POST_CID);
        let ts = body["record"]["createdAt"].as_str().unwrap();
        assert!(chrono::DateTime::parse_from_rfc3339(ts).is_ok());
    }

    // ── Repost records ──────────────────────────────────────

    #[test]
    fn test_repost_record_structure() {
        let body = build_repost_record(TEST_DID, TEST_POST_URI, TEST_POST_CID);
        assert_eq!(body["repo"], TEST_DID);
        assert_eq!(body["collection"], "app.bsky.feed.repost");
        assert_eq!(body["record"]["$type"], "app.bsky.feed.repost");
        assert_eq!(body["record"]["subject"]["uri"], TEST_POST_URI);
        assert_eq!(body["record"]["subject"]["cid"], TEST_POST_CID);
        assert!(body["record"]["createdAt"].is_string());
    }

    // ── Post records ────────────────────────────────────────

    #[test]
    fn test_post_record_simple() {
        let body = build_post_record(TEST_DID, "hello world", None, None);
        assert_eq!(body["repo"], TEST_DID);
        assert_eq!(body["collection"], "app.bsky.feed.post");
        assert_eq!(body["record"]["$type"], "app.bsky.feed.post");
        assert_eq!(body["record"]["text"], "hello world");
        assert!(body["record"]["createdAt"].is_string());
        assert!(body["record"]["reply"].is_null());
        assert!(body["record"]["embed"].is_null());
    }

    #[test]
    fn test_post_record_with_reply() {
        let reply = ReplyRef {
            root_uri: "at://did:plc:root/app.bsky.feed.post/root1".into(),
            root_cid: "bafyroot".into(),
            parent_uri: "at://did:plc:parent/app.bsky.feed.post/parent1".into(),
            parent_cid: "bafyparent".into(),
        };
        let body = build_post_record(TEST_DID, "my reply", Some(&reply), None);

        assert_eq!(body["record"]["text"], "my reply");

        let reply_obj = &body["record"]["reply"];
        assert_eq!(reply_obj["root"]["uri"], reply.root_uri);
        assert_eq!(reply_obj["root"]["cid"], reply.root_cid);
        assert_eq!(reply_obj["parent"]["uri"], reply.parent_uri);
        assert_eq!(reply_obj["parent"]["cid"], reply.parent_cid);

        // Reply should not have an embed.
        assert!(body["record"]["embed"].is_null());
    }

    #[test]
    fn test_post_record_reply_root_differs_from_parent() {
        let reply = ReplyRef {
            root_uri: "at://did:plc:root/app.bsky.feed.post/root1".into(),
            root_cid: "bafyroot".into(),
            parent_uri: "at://did:plc:parent/app.bsky.feed.post/parent1".into(),
            parent_cid: "bafyparent".into(),
        };
        let body = build_post_record(TEST_DID, "deep reply", Some(&reply), None);

        // Root and parent must be different when replying deep in a thread.
        assert_ne!(
            body["record"]["reply"]["root"]["uri"],
            body["record"]["reply"]["parent"]["uri"]
        );
    }

    #[test]
    fn test_post_record_with_quote() {
        let quote = QuoteRef {
            uri: TEST_POST_URI.into(),
            cid: TEST_POST_CID.into(),
        };
        let body = build_post_record(TEST_DID, "check this out", None, Some(&quote));

        assert_eq!(body["record"]["text"], "check this out");
        assert_eq!(body["record"]["embed"]["$type"], "app.bsky.embed.record");
        assert_eq!(body["record"]["embed"]["record"]["uri"], TEST_POST_URI);
        assert_eq!(body["record"]["embed"]["record"]["cid"], TEST_POST_CID);

        // Quote should not have a reply.
        assert!(body["record"]["reply"].is_null());
    }

    #[test]
    fn test_post_record_with_reply_and_quote() {
        let reply = ReplyRef {
            root_uri: "at://did:plc:root/app.bsky.feed.post/root1".into(),
            root_cid: "bafyroot".into(),
            parent_uri: "at://did:plc:parent/app.bsky.feed.post/parent1".into(),
            parent_cid: "bafyparent".into(),
        };
        let quote = QuoteRef {
            uri: TEST_POST_URI.into(),
            cid: TEST_POST_CID.into(),
        };
        let body = build_post_record(TEST_DID, "reply with quote", Some(&reply), Some(&quote));

        // Both reply and embed should be present.
        assert!(body["record"]["reply"].is_object());
        assert!(body["record"]["embed"].is_object());
        assert_eq!(body["record"]["reply"]["root"]["uri"], reply.root_uri);
        assert_eq!(body["record"]["embed"]["record"]["uri"], quote.uri);
    }

    // ── Follow records ──────────────────────────────────────

    #[test]
    fn test_follow_record_structure() {
        let target = "did:plc:targetuser456";
        let body = build_follow_record(TEST_DID, target);
        assert_eq!(body["repo"], TEST_DID);
        assert_eq!(body["collection"], "app.bsky.graph.follow");
        assert_eq!(body["record"]["$type"], "app.bsky.graph.follow");
        assert_eq!(body["record"]["subject"], target);
        assert!(body["record"]["createdAt"].is_string());
    }

    #[test]
    fn test_follow_record_subject_is_did() {
        let body = build_follow_record(TEST_DID, "did:plc:someone");
        // Subject should be the raw DID string, not wrapped in an object.
        assert!(body["record"]["subject"].is_string());
    }

    // ── Block records ───────────────────────────────────────

    #[test]
    fn test_block_record_structure() {
        let target = "did:plc:baduser789";
        let body = build_block_record(TEST_DID, target);
        assert_eq!(body["repo"], TEST_DID);
        assert_eq!(body["collection"], "app.bsky.graph.block");
        assert_eq!(body["record"]["$type"], "app.bsky.graph.block");
        assert_eq!(body["record"]["subject"], target);
        assert!(body["record"]["createdAt"].is_string());
    }

    // ── Mute body ───────────────────────────────────────────

    #[test]
    fn test_mute_body_structure() {
        let body = build_mute_body("did:plc:annoying");
        assert_eq!(body["actor"], "did:plc:annoying");
        // Mute is a simple action, not a record creation.
        assert!(body["repo"].is_null());
    }

    // ── Delete records ──────────────────────────────────────

    #[test]
    fn test_delete_record_extracts_rkey() {
        let body = build_delete_record(
            TEST_DID,
            "app.bsky.feed.like",
            TEST_LIKE_URI,
        )
        .unwrap();
        assert_eq!(body["repo"], TEST_DID);
        assert_eq!(body["collection"], "app.bsky.feed.like");
        assert_eq!(body["rkey"], "xyz789");
    }

    #[test]
    fn test_delete_record_invalid_uri() {
        let result = build_delete_record(TEST_DID, "app.bsky.feed.like", "");
        // Empty string still has a "last" element (empty string), so this
        // doesn't error, but the rkey would be empty.
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_record_rkey_from_longer_uri() {
        let uri = "at://did:plc:testuser/app.bsky.feed.repost/3l7abc123";
        let body = build_delete_record(TEST_DID, "app.bsky.feed.repost", uri).unwrap();
        assert_eq!(body["rkey"], "3l7abc123");
    }

    // ── Send message body ───────────────────────────────────

    #[test]
    fn test_send_message_body() {
        let body = build_send_message_body("convo-123", "hello there");
        assert_eq!(body["convoId"], "convo-123");
        assert_eq!(body["message"]["text"], "hello there");
    }

    // ── Response parsing ────────────────────────────────────

    #[test]
    fn test_parse_post_view_minimal() {
        let json = serde_json::json!({
            "uri": "at://did:plc:x/app.bsky.feed.post/1",
            "cid": "bafycid1",
            "author": {
                "did": "did:plc:x",
                "handle": "alice.bsky.social",
                "displayName": "Alice"
            },
            "record": {
                "text": "hello world",
                "$type": "app.bsky.feed.post"
            },
            "indexedAt": "2024-01-01T00:00:00Z",
            "likeCount": 5,
            "repostCount": 2,
            "replyCount": 1
        });

        let post = parse_post_view(&json).unwrap();
        assert_eq!(post.uri, "at://did:plc:x/app.bsky.feed.post/1");
        assert_eq!(post.text, "hello world");
        assert_eq!(post.author.handle, "alice.bsky.social");
        assert_eq!(post.author.display_name, Some("Alice".into()));
        assert_eq!(post.like_count, 5);
        assert_eq!(post.repost_count, 2);
        assert_eq!(post.reply_count, 1);
    }

    #[test]
    fn test_parse_post_view_with_viewer_state() {
        let json = serde_json::json!({
            "uri": "at://did:plc:x/app.bsky.feed.post/1",
            "cid": "bafycid1",
            "author": { "did": "did:plc:x", "handle": "bob.bsky.social" },
            "record": { "text": "test", "$type": "app.bsky.feed.post" },
            "indexedAt": "2024-01-01T00:00:00Z",
            "viewer": {
                "like": "at://did:plc:me/app.bsky.feed.like/abc",
                "repost": "at://did:plc:me/app.bsky.feed.repost/def"
            }
        });

        let post = parse_post_view(&json).unwrap();
        assert!(post.liked_by_me.is_some());
        assert!(post.reposted_by_me.is_some());
    }

    #[test]
    fn test_parse_post_view_with_reply_ref() {
        let json = serde_json::json!({
            "uri": "at://did:plc:x/app.bsky.feed.post/1",
            "cid": "bafycid1",
            "author": { "did": "did:plc:x", "handle": "carol.bsky.social" },
            "record": {
                "text": "a reply",
                "$type": "app.bsky.feed.post",
                "reply": {
                    "root": { "uri": "at://did:plc:y/app.bsky.feed.post/root", "cid": "rootcid" },
                    "parent": { "uri": "at://did:plc:y/app.bsky.feed.post/parent", "cid": "parentcid" }
                }
            },
            "indexedAt": "2024-01-01T00:00:00Z"
        });

        let post = parse_post_view(&json).unwrap();
        assert!(post.reply_to.is_some());
        let rt = post.reply_to.unwrap();
        assert_eq!(rt.root_uri, "at://did:plc:y/app.bsky.feed.post/root");
        assert_eq!(rt.parent_uri, "at://did:plc:y/app.bsky.feed.post/parent");
        assert_ne!(rt.root_uri, rt.parent_uri);
    }

    #[test]
    fn test_parse_feed_view_post_repost() {
        let json = serde_json::json!({
            "post": {
                "uri": "at://did:plc:x/app.bsky.feed.post/1",
                "cid": "bafycid1",
                "author": { "did": "did:plc:x", "handle": "alice.bsky.social" },
                "record": { "text": "original post", "$type": "app.bsky.feed.post" },
                "indexedAt": "2024-01-01T00:00:00Z"
            },
            "reason": {
                "$type": "app.bsky.feed.defs#reasonRepost",
                "by": {
                    "did": "did:plc:reposter",
                    "handle": "bob.bsky.social",
                    "displayName": "Bob"
                }
            }
        });

        let post = parse_feed_view_post(&json).unwrap();
        assert!(post.reposted_by.is_some());
        assert_eq!(post.reposted_by.unwrap().handle, "bob.bsky.social");
    }

    #[test]
    fn test_parse_feed_view_post_with_reply_context() {
        let json = serde_json::json!({
            "post": {
                "uri": "at://did:plc:x/app.bsky.feed.post/1",
                "cid": "bafycid1",
                "author": { "did": "did:plc:x", "handle": "alice.bsky.social" },
                "record": { "text": "a reply", "$type": "app.bsky.feed.post" },
                "indexedAt": "2024-01-01T00:00:00Z"
            },
            "reply": {
                "parent": {
                    "uri": "at://did:plc:parent/app.bsky.feed.post/p1",
                    "cid": "parentcid",
                    "author": { "did": "did:plc:parent", "handle": "parent.bsky.social" },
                    "record": { "text": "the parent post" },
                    "indexedAt": "2024-01-01T00:00:00Z"
                },
                "root": {
                    "uri": "at://did:plc:root/app.bsky.feed.post/r1",
                    "cid": "rootcid",
                    "author": { "did": "did:plc:root", "handle": "root.bsky.social" },
                    "record": { "text": "the root post" },
                    "indexedAt": "2024-01-01T00:00:00Z"
                }
            }
        });

        let post = parse_feed_view_post(&json).unwrap();
        assert!(post.reply_context.is_some());
        let ctx = post.reply_context.unwrap();
        assert_eq!(ctx.parent_author.handle, "parent.bsky.social");
        assert_eq!(ctx.parent_text, "the parent post");
        assert!(ctx.root_author.is_some());
        assert_eq!(ctx.root_author.unwrap().handle, "root.bsky.social");
    }

    #[test]
    fn test_parse_notification() {
        let json = serde_json::json!({
            "uri": "at://did:plc:x/app.bsky.feed.like/1",
            "cid": "bafycid",
            "author": { "did": "did:plc:liker", "handle": "fan.bsky.social" },
            "reason": "like",
            "indexedAt": "2024-01-01T12:00:00Z",
            "isRead": false
        });

        let notif = parse_notification(&json).unwrap();
        assert_eq!(notif.reason, NotificationReason::Like);
        assert_eq!(notif.author.handle, "fan.bsky.social");
        assert!(!notif.is_read);
    }

    #[test]
    fn test_parse_embed_images() {
        let json = serde_json::json!({
            "$type": "app.bsky.embed.images#view",
            "images": [{
                "thumb": "https://cdn.bsky.app/thumb.jpg",
                "fullsize": "https://cdn.bsky.app/full.jpg",
                "alt": "a cool picture",
                "aspectRatio": { "width": 800, "height": 600 }
            }]
        });

        let embed = parse_embed(&json).unwrap();
        if let PostEmbed::Images(imgs) = embed {
            assert_eq!(imgs.len(), 1);
            assert_eq!(imgs[0].alt, "a cool picture");
            assert_eq!(imgs[0].width, Some(800));
        } else {
            panic!("Expected Images embed");
        }
    }

    #[test]
    fn test_parse_embed_quote_post() {
        let json = serde_json::json!({
            "$type": "app.bsky.embed.record#view",
            "record": {
                "uri": "at://did:plc:quoted/app.bsky.feed.post/q1",
                "author": { "did": "did:plc:quoted", "handle": "quotee.bsky.social" },
                "value": { "text": "the original take" },
                "indexedAt": "2024-01-01T00:00:00Z"
            }
        });

        let embed = parse_embed(&json).unwrap();
        if let PostEmbed::Record(qp) = embed {
            assert_eq!(qp.author.handle, "quotee.bsky.social");
            assert_eq!(qp.text, "the original take");
        } else {
            panic!("Expected Record embed, got {:?}", embed);
        }
    }

    #[test]
    fn test_parse_embed_external_link() {
        let json = serde_json::json!({
            "$type": "app.bsky.embed.external#view",
            "external": {
                "uri": "https://example.com",
                "title": "Example",
                "description": "An example website"
            }
        });

        let embed = parse_embed(&json).unwrap();
        if let PostEmbed::External { uri, title, description } = embed {
            assert_eq!(uri, "https://example.com");
            assert_eq!(title, "Example");
            assert_eq!(description, "An example website");
        } else {
            panic!("Expected External embed");
        }
    }
}
