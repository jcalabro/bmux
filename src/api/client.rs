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
    let body = serde_json::json!({
        "repo": agent.did,
        "collection": "app.bsky.feed.like",
        "record": {
            "$type": "app.bsky.feed.like",
            "subject": { "uri": uri, "cid": cid },
            "createdAt": chrono::Utc::now().to_rfc3339()
        }
    });

    let resp = xrpc_post(agent, "com.atproto.repo.createRecord", &body).await?;
    Ok(resp["uri"].as_str().unwrap_or("").to_string())
}

/// Unlike a post.
pub async fn unlike_post(agent: &AppAgent, like_uri: &str) -> Result<()> {
    let parts: Vec<&str> = like_uri.split('/').collect();
    let rkey = parts.last().ok_or_else(|| anyhow::anyhow!("Bad URI"))?;

    let body = serde_json::json!({
        "repo": agent.did,
        "collection": "app.bsky.feed.like",
        "rkey": rkey
    });

    xrpc_post(agent, "com.atproto.repo.deleteRecord", &body).await?;
    Ok(())
}

/// Repost a post.
pub async fn repost_post(agent: &AppAgent, uri: &str, cid: &str) -> Result<String> {
    let body = serde_json::json!({
        "repo": agent.did,
        "collection": "app.bsky.feed.repost",
        "record": {
            "$type": "app.bsky.feed.repost",
            "subject": { "uri": uri, "cid": cid },
            "createdAt": chrono::Utc::now().to_rfc3339()
        }
    });

    let resp = xrpc_post(agent, "com.atproto.repo.createRecord", &body).await?;
    Ok(resp["uri"].as_str().unwrap_or("").to_string())
}

/// Create a new post.
pub async fn create_post(
    agent: &AppAgent,
    text: &str,
    reply_to: Option<&ReplyRef>,
) -> Result<String> {
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

    let body = serde_json::json!({
        "repo": agent.did,
        "collection": "app.bsky.feed.post",
        "record": record
    });

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
    let body = serde_json::json!({
        "convoId": convo_id,
        "message": { "text": text }
    });
    xrpc_post(agent, "chat.bsky.convo.sendMessage", &body).await?;
    Ok(())
}

/// Follow a user.
pub async fn follow_user(agent: &AppAgent, did: &str) -> Result<String> {
    let body = serde_json::json!({
        "repo": agent.did,
        "collection": "app.bsky.graph.follow",
        "record": {
            "$type": "app.bsky.graph.follow",
            "subject": did,
            "createdAt": chrono::Utc::now().to_rfc3339()
        }
    });
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
