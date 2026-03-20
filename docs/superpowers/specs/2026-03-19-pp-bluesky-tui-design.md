# alf — Design Spec

> The tmux of Bluesky. A maximalist terminal user interface for Bluesky built in Rust.

## Overview

`alf` is a terminal-based Bluesky client with tmux-style pane management, modal vim keybindings, inline image rendering, and full theming support. It connects to the Bluesky API via the `jacquard` crate and renders with `ratatui`.

The binary is called `alf`.

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Framework | Rust + ratatui + jacquard | ratatui is the standard Rust TUI lib; jacquard has full AT Protocol coverage including OAuth |
| Architecture | Actor-based (tokio tasks + mpsc channels) | Clean separation, no shared mutable state, naturally async |
| Auth | OAuth with DPoP only | The AT Protocol's recommended auth flow; no app passwords |
| Layout | Freely arrangeable panes + named workspace layouts | tmux-style splits with saveable/restorable workspace configurations |
| Vim | Modal — Normal/Insert/Command modes | Real modes with proper motions, not just vim-flavored hotkeys |
| Compose | Inline panel + $EDITOR handoff | Quick inline for short posts, $EDITOR for longer ones |
| Images | Sixel/Kitty inline rendering with text fallback | Inline images in supported terminals, alt text otherwise |
| Notifications | Polling (30s) with toast alerts | Simpler than firehose, still feels alive |
| Theming | Fully themeable via TOML, Bluesky blue default | 4 built-in themes, custom themes via config or theme files |

## Architecture

Six tokio tasks communicate through typed `mpsc` channels. No shared mutable state.

```
┌──────────┐    ┌──────────────────────────────────────────┐
│  Input   │    │            App Actor                     │
│  Task    │───>│                                          │
│          │    │  ┌────────────────────────────────────┐   │
│ crossterm │    │  │       Workspace Manager            │   │
│ events    │    │  │                                    │   │
└──────────┘    │  │  ┌──────┐ ┌──────┐ ┌──────┐       │   │
                │  │  │Pane 1│ │Pane 2│ │Pane 3│ ...   │   │
┌──────────┐    │  │  │ Feed │ │Thread│ │ DMs  │       │   │
│  API     │    │  │  └──────┘ └──────┘ └──────┘       │   │
│  Task    │<──>│  └────────────────────────────────────┘   │
│          │    │                                          │
│ jacquard  │    │  ┌────────────────────────────────────┐   │
│ agent     │    │  │         Render Engine              │   │
└──────────┘    │  │   ratatui immediate-mode draw      │   │
                │  └────────────────────────────────────┘   │
┌──────────┐    │                                          │
│  Poll    │    │  ┌────────────────────────────────────┐   │
│  Task    │───>│  │         Toast Manager               │   │
│          │    │  └────────────────────────────────────┘   │
│ 30s timer │    └──────────────────────────────────────────┘
└──────────┘
                ┌──────────────────────────────────────────┐
┌──────────┐    │            Config / Theme                │
│  Image   │    │   TOML config -> Theme struct             │
│  Task    │    │   Sixel/Kitty detection                   │
│          │    └──────────────────────────────────────────┘
│ decode + │
│ render   │    ┌──────────────────────────────────────────┐
└──────────┘    │            Auth / Session                 │
                │   OAuth DPoP flow via jacquard            │
                │   Token persistence (FileTokenStore)      │
                └──────────────────────────────────────────┘
```

### Actors

**App Actor** — Central coordinator. Owns all UI state: workspaces, pane trees, vim mode, toast queue, command buffer, theme, and session. Receives messages from all other tasks. Drives the render loop.

**Input Task** — Reads crossterm events. Translates raw key events into semantic `UiAction` messages, respecting the current vim mode (Normal/Insert/Command). Sends actions to App Actor.

**API Task** — Wraps the jacquard `Agent<OAuthSession>`. Receives `ApiRequest` messages, executes them asynchronously, returns `ApiResponse` messages. Handles rate limiting and retries.

**Poll Task** — Periodic timer (configurable, default 30s). Checks the notifications API and sends `Notification` messages with unread counts and new notification data to App Actor.

**Image Task** — Receives image URLs to decode. Downloads, decodes with the `image` crate, encodes to Sixel or Kitty graphics protocol sequences. Returns `ImageReady` messages. Maintains an in-memory LRU cache keyed by URL + dimensions.

**Auth/Session** — Runs at startup only. Performs the OAuth DPoP login flow via jacquard's `OAuthClient`. Starts a localhost HTTP server for the redirect callback. Persists tokens to disk via `FileTokenStore`. If valid tokens exist, refreshes them silently.

## UI Layout

The screen has three layers:

```
┌─[ Home ]──[ Feed ]──[ DMs ]──[ Notifs ]─────── alf v0.1 │ @user │ 🔔 3 ─┐
│                                                                           │
│  ┌─────────────── Feed (2/3) ──────────────┬──── Thread (1/3) ──────────┐ │
│  │                                         │                            │ │
│  │  [Following] [Discover] [News]          │   Thread view with         │ │
│  │                                         │   indented replies         │ │
│  │  Post cards with author, text,          │                            │ │
│  │  engagement counts, images              │                            │ │
│  │                                         │                            │ │
│  └─────────────────────────────────────────┴────────────────────────────┘ │
│                                                                           │
├─ NORMAL │ j/k:scroll  l:thread  f:like  r:repost  c:compose  /:search ───┤
└───────────────────────────────────────────────────────────────────────────┘
```

### Top Bar

Workspace tabs on the left (Home, Feed, DMs, Notifs — user-configurable). App name, version, user handle, and notification badge on the right. Number keys (1-9) switch workspaces.

### Main Area

tmux-style panes arranged as a binary tree of splits. Default workspace: 2/3 feed + 1/3 detail. Resizable with `Ctrl-w +/-/</>`. Create/close with `:split`, `:vsplit`, `:close`. Zoom a pane with `Ctrl-w o`.

### Status Bar

Bottom bar shows current vim mode (NORMAL/INSERT/COMMAND) with color-coded indicator, plus contextual keybinding hints that change based on the focused pane type.

### Pane Types

**Feed** — Timeline, author feed, custom feed, list feed, or search results. Post cards with author handle, text (with rich text facets rendered), engagement counts, and inline images. Sub-tabs for different feeds (Following, Discover, custom). Each tab maintains independent scroll position and cursor.

**Thread** — Threaded reply view with indentation showing the conversation tree. Shows the parent chain above and replies below the focused post. Navigate with j/k, collapse branches with h.

**Profile** — User profile view: avatar (if image protocol supported), display name, handle, bio, follower/following counts, follow/mute/block buttons. Sub-tabs for the user's posts, replies, likes, and media.

**DMs** — Split internally: conversation list on the left, active chat on the right. Messages displayed chronologically with timestamps. Compose area at the bottom of the chat sub-pane.

**Notifications** — Grouped by type: likes, reposts, follows, mentions, replies. Each entry shows the actor and the target post. Mark as read, jump to the source post with Enter.

**Compose** — Inline post composer. Character count (300 grapheme limit), mention autocomplete triggered by `@`, rich text preview. Press `E` or `Ctrl-e` to switch to `$EDITOR` for the current draft.

## Pane Tree Data Structure

Panes are stored as a binary tree — exactly how tmux and vim do it.

```rust
enum PaneTree {
    Split {
        direction: SplitDirection, // Horizontal or Vertical
        ratio: f64,                // 0.0..1.0
        first: Box<PaneTree>,
        second: Box<PaneTree>,
    },
    Leaf(Pane),
}
```

`:split` replaces the focused leaf with a `Split` node containing the old pane and a new one. `:close` removes the leaf and promotes its sibling. Resize adjusts the ratio. Zoom temporarily replaces the tree root with the focused leaf (restoring on un-zoom).

## Vim Keybinding System

Three modes with a state machine in the Input Task.

### Normal Mode (default)

| Key | Action |
|-----|--------|
| `j` / `k` | Move down / up through posts or items |
| `gg` / `G` | Jump to top / bottom of feed |
| `Ctrl-d` / `Ctrl-u` | Half-page scroll down / up |
| `l` / `Enter` | Open thread / expand selected post |
| `h` / `Esc` | Go back / close detail pane |
| `f` | Like (favorite) selected post |
| `b` | Repost (boost) selected post |
| `r` | Reply to selected post (opens inline compose) |
| `c` | Compose new post (inline) |
| `E` | Compose in $EDITOR |
| `p` | Open author profile for selected post |
| `o` | Open post/link in browser |
| `/` | Search (enters command mode with search prefix) |
| `n` / `N` | Next / previous search result |
| `q` | Quit (with confirmation) |
| `?` | Show help overlay |

### Workspace & Pane Management

| Key | Action |
|-----|--------|
| `1`-`9` | Switch to workspace 1-9 |
| `Tab` | Cycle focus between panes |
| `Ctrl-w h/j/k/l` | Move focus to left/down/up/right pane |
| `Ctrl-w +/-` | Resize current pane taller / shorter |
| `Ctrl-w </>` | Resize current pane narrower / wider |
| `Ctrl-w =` | Equalize all pane sizes |
| `Ctrl-w o` | Zoom/unzoom current pane (fill screen) |
| `Shift-h` / `Shift-l` | Previous / next feed tab within a feed pane |

### Insert Mode

Active when composing posts or DMs.

| Key | Action |
|-----|--------|
| `Esc` | Return to Normal mode (cancel compose) |
| `Ctrl-Enter` | Send post / message |
| `@` | Trigger mention autocomplete popup |
| `Ctrl-a` | Attach image (opens file picker dialog) |
| `Ctrl-e` | Switch to $EDITOR for current draft |

### Command Mode

Activated with `:` (colon).

| Command | Action |
|---------|--------|
| `:q` | Quit alf |
| `:split {type}` | Horizontal split (feed, dms, notifs, profile @handle) |
| `:vsplit {type}` | Vertical split |
| `:close` | Close current pane |
| `:workspace {name}` | Switch to or create named workspace |
| `:follow @handle` | Follow a user |
| `:mute @handle` | Mute a user |
| `:block @handle` | Block a user |
| `:theme {name}` | Switch theme |
| `:feed {uri}` | Open a custom feed by URI in current pane |
| `:dm @handle` | Open DM conversation with user |

## Data Flow

### Message Types

```
Inbound (to App Actor):
  UiAction        — from Input Task: ScrollDown, Like, Compose, SwitchWorkspace, Command(String), ...
  ApiResponse     — from API Task: Timeline(Vec<Post>), Thread(PostThread), PostCreated(Uri), Error(ApiError), ...
  Notification    — from Poll Task: NewNotifs(Vec<Notif>, unread_count)
  ImageReady      — from Image Task: Decoded(url, SixelData | AltText)

Outbound (from App Actor):
  ApiRequest      — to API Task: FetchTimeline { cursor }, CreatePost { text, facets }, LikePost { uri, cid }, ...
  ImageRequest    — to Image Task: Decode { url, max_width, max_height }
```

### State Ownership

Each pane owns its own state:

- **Feed pane**: feed tabs, active tab index, posts vec, cursor position, loading flag, pagination cursor
- **Thread pane**: root post, reply tree, collapsed set, cursor position
- **DMs pane**: conversation list, active conversation ID, messages vec, draft text
- **Profile pane**: profile data, active sub-tab, posts for that tab
- **Notifications pane**: notification list, unread count
- **Compose pane**: draft text, reply-to reference, attached images, autocomplete state

### Pagination

Feeds use cursor-based pagination. When the user scrolls near the bottom of a feed, App Actor sends a `FetchTimeline` with the next cursor. New posts are appended to the existing list. Scroll position is preserved. Each feed tab tracks its own pagination cursor independently.

### Optimistic Updates

Likes and reposts update the UI immediately. If the API call fails, the action is rolled back and a toast shows the error. This makes the app feel snappy.

### Render Cycle

```rust
loop {
    // 1. Drain all pending messages (non-blocking)
    while let Ok(msg) = rx.try_recv() {
        app.handle_message(msg);
    }

    // 2. Render current state
    terminal.draw(|frame| app.render(frame))?;

    // 3. Wait for next event (with timeout for smooth toasts)
    tokio::select! {
        msg = rx.recv() => app.handle_message(msg),
        _ = tokio::time::sleep(Duration::from_millis(100)) => {
            app.tick(); // advance toast timers, loading spinners
        }
    }
}
```

### Image Pipeline

1. Post rendering encounters an image URL
2. App Actor sends `ImageRequest::Decode { url, max_width, max_height }` to Image Task
3. Post renders with placeholder text (`[loading image...]`)
4. Image Task checks LRU cache — if hit, returns immediately
5. On miss: downloads image, decodes with `image` crate, encodes to Sixel or Kitty protocol
6. Returns `ImageReady::Decoded(url, data)` to App Actor
7. Next render cycle picks up the decoded image and renders it inline

Terminal image protocol is auto-detected at startup (check for Kitty first, then Sixel, then fall back to text-only).

## Config & Theming

### Config File

Located at `~/.config/alf/config.toml`.

```toml
[general]
theme = "bluesky"              # built-in: bluesky, hacker, catppuccin, nord
poll_interval_secs = 30
toast_duration_secs = 5
editor = "$EDITOR"             # override for compose handoff
image_protocol = "auto"        # auto, sixel, kitty, none

[oauth]
client_id = "..."
redirect_port = 8420
token_file = "~/.config/alf/tokens.json"

[feeds]
tabs = [
  { name = "Following", uri = "following" },
  { name = "Discover", uri = "discover" },
  { name = "News", uri = "at://did:plc:.../app.bsky.feed.generator/news" },
]

[workspaces.home]
layout = "vsplit"
ratio = 0.65
left = { pane = "feed" }
right = { pane = "thread" }

[workspaces.dms]
layout = "vsplit"
ratio = 0.35
left = { pane = "dm-list" }
right = { pane = "dm-chat" }
```

### Theme Definition

Themes are color tables with 16 named color slots:

```toml
[themes.bluesky]
bg         = "#0a0e1a"
fg         = "#e0e4ef"
accent     = "#0085ff"
secondary  = "#00c2ff"
border     = "#1e2d45"
muted      = "#5a6478"
error      = "#ff4444"
warning    = "#ffaa00"
success    = "#44cc66"
normal_bg  = "#0085ff"
insert_bg  = "#e8a040"
command_bg = "#a0d0a0"
handle     = "#0085ff"
timestamp  = "#5a6478"
like       = "#ff6b8a"
repost     = "#44cc66"
reply      = "#00c2ff"
```

Custom themes inherit missing values from the default (bluesky). Users can define themes inline in `config.toml` or drop `.toml` files in `~/.config/alf/themes/`. Runtime switching with `:theme {name}`.

Four built-in themes: **bluesky** (default — blues and whites), **hacker** (green/amber on black), **catppuccin** (soft pastels), **nord** (cool blues and grays).

### File Layout

```
~/.config/alf/
├── config.toml          # main config
├── tokens.json          # OAuth token persistence (auto-managed)
└── themes/              # optional: extra theme files
    └── my-theme.toml
```

## Project Structure

Single crate. Module boundaries follow the actor boundaries.

```
power-poaster/
├── Cargo.toml
├── LICENSE
├── README.md
└── src/
    ├── main.rs                  # entry: arg parsing, tokio runtime, auth, launch
    ├── app.rs                   # App actor: message loop, state coordination
    ├── auth/
    │   ├── mod.rs               # OAuth DPoP flow, token persistence
    │   └── oauth.rs             # OAuthClient setup, localhost callback server
    ├── api/
    │   ├── mod.rs               # API task: request dispatch, rate limiting
    │   ├── types.rs             # ApiRequest / ApiResponse enums
    │   └── client.rs            # jacquard agent wrapper, retry logic
    ├── input/
    │   ├── mod.rs               # Input task: crossterm event reader
    │   ├── vim.rs               # Vim mode state machine (Normal/Insert/Command)
    │   └── action.rs            # UiAction enum
    ├── ui/
    │   ├── mod.rs               # top-level render: status bar, workspace tabs, pane tree
    │   ├── workspace.rs         # Workspace struct, pane tree layout algorithm
    │   ├── pane.rs              # Pane trait, PaneKind dispatch
    │   ├── feed.rs              # Feed pane
    │   ├── thread.rs            # Thread pane
    │   ├── profile.rs           # Profile pane
    │   ├── dms.rs               # DMs pane
    │   ├── notifications.rs     # Notifications pane
    │   ├── compose.rs           # Compose pane
    │   ├── toast.rs             # Toast notification overlay
    │   ├── help.rs              # Help overlay (? key)
    │   └── widgets/
    │       ├── mod.rs
    │       ├── post_card.rs     # Post card widget
    │       ├── rich_text.rs     # Facet -> ratatui Span renderer
    │       ├── image.rs         # Sixel/Kitty image widget
    │       ├── status_bar.rs    # Bottom bar
    │       ├── tab_bar.rs       # Workspace tabs / feed tabs
    │       └── autocomplete.rs  # Mention autocomplete popup
    ├── image/
    │   ├── mod.rs               # Image task: decode, cache, protocol detection
    │   ├── sixel.rs             # Sixel encoder
    │   ├── kitty.rs             # Kitty graphics protocol encoder
    │   └── cache.rs             # In-memory LRU image cache
    ├── poll.rs                  # Poll task: periodic notification checker
    ├── config/
    │   ├── mod.rs               # Config loading, defaults, validation
    │   └── theme.rs             # Theme struct, built-in themes, TOML parsing
    └── messages.rs              # All message types
```

## Dependencies

### Core
| Crate | Purpose |
|-------|---------|
| `ratatui` | TUI framework |
| `crossterm` | Terminal backend + events |
| `jacquard` | AT Protocol / Bluesky API client |
| `tokio` | Async runtime |

### Supporting
| Crate | Purpose |
|-------|---------|
| `serde` / `toml` | Config and theme parsing |
| `image` | Image decoding |
| `tui-textarea` | Text input widget for compose |
| `clap` | CLI argument parsing |
| `dirs` | XDG config directory paths |
| `tracing` | Structured logging |

## Testing Strategy

**Unit tests** — Vim mode state machine transitions, rich text facet-to-Span rendering, pane tree split/close/resize operations, config parsing and theme inheritance, message type serialization.

**Integration tests** — Message flow (send UiAction, verify correct ApiRequest is emitted), workspace save/restore round-trip, config file loading with partial themes.

**Manual testing** — TUI rendering across terminal emulators, OAuth flow end-to-end, image protocol detection and rendering (Sixel, Kitty, fallback), compose with $EDITOR handoff.
