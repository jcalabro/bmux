/// All message types flowing between actors.

/// Semantic UI actions produced by the Input Task after vim mode processing.
#[derive(Debug, Clone, PartialEq)]
pub enum UiAction {
    // Navigation
    ScrollDown,
    ScrollUp,
    HalfPageDown,
    HalfPageUp,
    GotoTop,
    GotoBottom,

    // Post actions
    OpenThread,
    GoBack,
    Like,
    Repost,
    Reply,
    ComposeNew,
    ComposeInEditor,
    OpenProfile,
    OpenInBrowser,

    // Search
    SearchStart,
    SearchNext,
    SearchPrev,

    // Feed tabs
    PrevFeedTab,
    NextFeedTab,

    // Workspace
    SwitchWorkspace(usize),

    // Pane management
    CyclePaneFocus,
    FocusPaneLeft,
    FocusPaneDown,
    FocusPaneUp,
    FocusPaneRight,
    ResizePaneGrow,
    ResizePaneShrink,
    ResizePaneWider,
    ResizePaneNarrower,
    EqualizePanes,
    ZoomPane,

    // Command mode
    EnterCommandMode,
    Command(String),

    // Compose
    SubmitPost,
    CancelCompose,
    AttachImage,
    SwitchToEditor,

    // Text input (in insert mode)
    InsertChar(char),
    InsertBackspace,
    InsertDelete,
    InsertNewline,
    InsertMoveLeft,
    InsertMoveRight,
    InsertMoveHome,
    InsertMoveEnd,

    // General
    Quit,
    ShowHelp,
    Tick,
    Resize(u16, u16),
}

/// Requests sent from App Actor to API Task.
#[derive(Debug, Clone)]
pub enum ApiRequest {
    FetchTimeline {
        cursor: Option<String>,
    },
    FetchAuthorFeed {
        actor: String,
        cursor: Option<String>,
    },
    FetchThread {
        uri: String,
    },
    FetchFeed {
        feed_uri: String,
        cursor: Option<String>,
    },
    CreatePost {
        text: String,
        reply_to: Option<ReplyRef>,
    },
    LikePost {
        uri: String,
        cid: String,
    },
    UnlikePost {
        like_uri: String,
    },
    RepostPost {
        uri: String,
        cid: String,
    },
    UnrepostPost {
        repost_uri: String,
    },
    FetchProfile {
        actor: String,
    },
    FetchNotifications {
        cursor: Option<String>,
    },
    MarkNotificationsRead,
    FetchConversations {
        cursor: Option<String>,
    },
    FetchMessages {
        convo_id: String,
        cursor: Option<String>,
    },
    SendMessage {
        convo_id: String,
        text: String,
    },
    FollowUser {
        did: String,
    },
    UnfollowUser {
        follow_uri: String,
    },
    MuteUser {
        did: String,
    },
    UnmuteUser {
        did: String,
    },
    BlockUser {
        did: String,
    },
    SearchPosts {
        query: String,
        cursor: Option<String>,
    },
    SearchUsers {
        query: String,
    },
    ResolveHandle {
        handle: String,
    },
}

/// Responses from API Task back to App Actor.
#[derive(Debug, Clone)]
pub enum ApiResponse {
    Timeline {
        posts: Vec<Post>,
        cursor: Option<String>,
    },
    AuthorFeed {
        actor: String,
        posts: Vec<Post>,
        cursor: Option<String>,
    },
    Thread {
        uri: String,
        thread: PostThread,
    },
    Feed {
        feed_uri: String,
        posts: Vec<Post>,
        cursor: Option<String>,
    },
    PostCreated {
        uri: String,
    },
    PostLiked {
        post_uri: String,
        like_uri: String,
    },
    PostUnliked {
        post_uri: String,
    },
    PostReposted {
        post_uri: String,
        repost_uri: String,
    },
    PostUnreposted {
        post_uri: String,
    },
    Profile(ProfileData),
    Notifications {
        notifications: Vec<Notification>,
        cursor: Option<String>,
        unread_count: usize,
    },
    Conversations {
        conversations: Vec<Conversation>,
        cursor: Option<String>,
    },
    Messages {
        convo_id: String,
        messages: Vec<DirectMessage>,
        cursor: Option<String>,
    },
    MessageSent {
        convo_id: String,
    },
    UserFollowed {
        did: String,
        follow_uri: String,
    },
    UserUnfollowed {
        did: String,
    },
    UserMuted {
        did: String,
    },
    UserUnmuted {
        did: String,
    },
    UserBlocked {
        did: String,
    },
    SearchResults {
        query: String,
        posts: Vec<Post>,
        cursor: Option<String>,
    },
    UserSearchResults {
        users: Vec<ProfileData>,
    },
    HandleResolved {
        handle: String,
        did: String,
    },
    Error {
        request_description: String,
        error: String,
    },
}

/// Messages from the App Actor to itself or from background tasks.
#[derive(Debug, Clone)]
pub enum AppMessage {
    Ui(UiAction),
    Api(ApiResponse),
    NotificationPoll(Vec<Notification>, usize),
    ImageReady {
        url: String,
        data: ImageData,
    },
    Toast(Toast),
}

/// A Bluesky post.
#[derive(Debug, Clone)]
pub struct Post {
    pub uri: String,
    pub cid: String,
    pub author: Author,
    pub text: String,
    pub facets: Vec<Facet>,
    pub created_at: String,
    pub like_count: u64,
    pub repost_count: u64,
    pub reply_count: u64,
    pub liked_by_me: Option<String>,    // like record URI if liked
    pub reposted_by_me: Option<String>, // repost record URI if reposted
    pub reply_to: Option<ReplyRef>,
    pub embed: Option<PostEmbed>,
    pub reposted_by: Option<Author>,
}

#[derive(Debug, Clone)]
pub struct Author {
    pub did: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Facet {
    pub start: usize,
    pub end: usize,
    pub kind: FacetKind,
}

#[derive(Debug, Clone)]
pub enum FacetKind {
    Mention { did: String },
    Link { uri: String },
    Tag { tag: String },
}

#[derive(Debug, Clone)]
pub struct ReplyRef {
    pub root_uri: String,
    pub root_cid: String,
    pub parent_uri: String,
    pub parent_cid: String,
}

#[derive(Debug, Clone)]
pub enum PostEmbed {
    Images(Vec<EmbedImage>),
    External {
        uri: String,
        title: String,
        description: String,
    },
    Record {
        uri: String,
    },
}

#[derive(Debug, Clone)]
pub struct EmbedImage {
    pub thumb_url: String,
    pub fullsize_url: String,
    pub alt: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct PostThread {
    pub post: Post,
    pub parent: Option<Box<PostThread>>,
    pub replies: Vec<PostThread>,
}

#[derive(Debug, Clone)]
pub struct ProfileData {
    pub did: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub followers_count: u64,
    pub follows_count: u64,
    pub posts_count: u64,
    pub following_me: bool,
    pub followed_by_me: Option<String>, // follow record URI
    pub muted: bool,
    pub blocked: bool,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub uri: String,
    pub cid: String,
    pub author: Author,
    pub reason: NotificationReason,
    pub subject_uri: Option<String>,
    pub created_at: String,
    pub is_read: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NotificationReason {
    Like,
    Repost,
    Follow,
    Mention,
    Reply,
    Quote,
}

#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: String,
    pub members: Vec<Author>,
    pub last_message: Option<DirectMessage>,
    pub unread_count: u64,
    pub muted: bool,
}

#[derive(Debug, Clone)]
pub struct DirectMessage {
    pub id: String,
    pub sender: Author,
    pub text: String,
    pub sent_at: String,
}

#[derive(Debug, Clone)]
pub enum ImageData {
    Sixel(String),
    Kitty(String),
    AltText(String),
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub ttl_ms: u64,
    pub created_at: std::time::Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl Toast {
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: ToastLevel::Info,
            ttl_ms: 5000,
            created_at: std::time::Instant::now(),
        }
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: ToastLevel::Success,
            ttl_ms: 5000,
            created_at: std::time::Instant::now(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: ToastLevel::Error,
            ttl_ms: 8000,
            created_at: std::time::Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_millis() as u64 >= self.ttl_ms
    }

    pub fn remaining_fraction(&self) -> f64 {
        let elapsed = self.created_at.elapsed().as_millis() as f64;
        let total = self.ttl_ms as f64;
        (1.0 - elapsed / total).max(0.0)
    }
}

/// Image request from App Actor to Image Task.
#[derive(Debug, Clone)]
pub struct ImageRequest {
    pub url: String,
    pub max_width: u16,
    pub max_height: u16,
}

/// For passing feed tab info around.
#[derive(Debug, Clone)]
pub struct FeedTab {
    pub name: String,
    pub uri: String,
    pub posts: Vec<Post>,
    pub cursor: Option<String>,
    pub scroll_offset: usize,
    pub selected: usize,
    pub loading: bool,
}

impl FeedTab {
    pub fn new(name: impl Into<String>, uri: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            uri: uri.into(),
            posts: Vec::new(),
            cursor: None,
            scroll_offset: 0,
            selected: 0,
            loading: false,
        }
    }
}

/// Metadata for displaying facets as colored spans.
#[derive(Debug, Clone)]
pub struct RichTextSegment {
    pub text: String,
    pub kind: RichTextKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RichTextKind {
    Plain,
    Mention(String),
    Link(String),
    Hashtag(String),
}

/// Convert post text + facets into rich text segments.
pub fn parse_rich_text(text: &str, facets: &[Facet]) -> Vec<RichTextSegment> {
    if facets.is_empty() {
        return vec![RichTextSegment {
            text: text.to_string(),
            kind: RichTextKind::Plain,
        }];
    }

    let bytes = text.as_bytes();
    let mut segments = Vec::new();
    let mut pos = 0;

    // Sort facets by start position.
    let mut sorted_facets = facets.to_vec();
    sorted_facets.sort_by_key(|f| f.start);

    for facet in &sorted_facets {
        let start = facet.start.min(bytes.len());
        let end = facet.end.min(bytes.len());

        if pos < start {
            if let Ok(s) = std::str::from_utf8(&bytes[pos..start]) {
                segments.push(RichTextSegment {
                    text: s.to_string(),
                    kind: RichTextKind::Plain,
                });
            }
        }

        if start < end {
            if let Ok(s) = std::str::from_utf8(&bytes[start..end]) {
                let kind = match &facet.kind {
                    FacetKind::Mention { did } => RichTextKind::Mention(did.clone()),
                    FacetKind::Link { uri } => RichTextKind::Link(uri.clone()),
                    FacetKind::Tag { tag } => RichTextKind::Hashtag(tag.clone()),
                };
                segments.push(RichTextSegment {
                    text: s.to_string(),
                    kind,
                });
            }
        }

        pos = end;
    }

    if pos < bytes.len() {
        if let Ok(s) = std::str::from_utf8(&bytes[pos..]) {
            segments.push(RichTextSegment {
                text: s.to_string(),
                kind: RichTextKind::Plain,
            });
        }
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_expiry() {
        let toast = Toast {
            message: "test".into(),
            level: ToastLevel::Info,
            ttl_ms: 0,
            created_at: std::time::Instant::now(),
        };
        // With 0 ttl, should be expired almost immediately
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(toast.is_expired());
    }

    #[test]
    fn test_toast_not_expired() {
        let toast = Toast::info("test");
        assert!(!toast.is_expired());
    }

    #[test]
    fn test_rich_text_no_facets() {
        let segments = parse_rich_text("hello world", &[]);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "hello world");
        assert_eq!(segments[0].kind, RichTextKind::Plain);
    }

    #[test]
    fn test_rich_text_with_mention() {
        let text = "hello @alice.bsky.social how are you";
        let facets = vec![Facet {
            start: 6,
            end: 24,
            kind: FacetKind::Mention {
                did: "did:plc:alice".into(),
            },
        }];
        let segments = parse_rich_text(text, &facets);
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].text, "hello ");
        assert_eq!(segments[0].kind, RichTextKind::Plain);
        assert_eq!(segments[1].text, "@alice.bsky.social");
        assert!(matches!(segments[1].kind, RichTextKind::Mention(_)));
        assert_eq!(segments[2].text, " how are you");
    }

    #[test]
    fn test_rich_text_with_link() {
        let text = "check out https://example.com for more";
        let facets = vec![Facet {
            start: 10,
            end: 29,
            kind: FacetKind::Link {
                uri: "https://example.com".into(),
            },
        }];
        let segments = parse_rich_text(text, &facets);
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[1].text, "https://example.com");
        assert!(matches!(segments[1].kind, RichTextKind::Link(_)));
    }

    #[test]
    fn test_rich_text_multiple_facets() {
        let text = "@alice hey @bob";
        let facets = vec![
            Facet {
                start: 0,
                end: 6,
                kind: FacetKind::Mention {
                    did: "did:plc:alice".into(),
                },
            },
            Facet {
                start: 11,
                end: 15,
                kind: FacetKind::Mention {
                    did: "did:plc:bob".into(),
                },
            },
        ];
        let segments = parse_rich_text(text, &facets);
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].text, "@alice");
        assert_eq!(segments[1].text, " hey ");
        assert_eq!(segments[2].text, "@bob");
    }

    #[test]
    fn test_feed_tab_new() {
        let tab = FeedTab::new("Following", "following");
        assert_eq!(tab.name, "Following");
        assert_eq!(tab.uri, "following");
        assert!(tab.posts.is_empty());
        assert_eq!(tab.selected, 0);
    }
}
