# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
just build          # debug build
just release        # optimized release build
just test           # run all tests
just test-one NAME  # run a single test by name (with output)
just lint           # cargo clippy
just fmt            # cargo fmt
just ci             # fmt-check + lint + test + build
just run -- ARGS    # run with arguments, e.g.: just run -- -u handle -p password
```

Requires `libchafa` system package for `ratatui-image` (inline image rendering).

## Architecture

**bmux** is an actor-based async TUI Bluesky client. Six tokio tasks communicate through typed `mpsc` channels with no shared mutable state.

### Message Flow

```
Input Thread (crossterm) ──KeyEvent──> App Actor ──ApiRequest──> API Task
Poll Task ──notifications──> App Actor <──ApiResponse── API Task
Image Task <──ImageRequest── App Actor ──render──> Terminal
```

The **App Actor** (`src/app.rs`) is the central coordinator. It owns all UI state and processes three kinds of input:
- `handle_key_event(KeyEvent)` — routes through VimState → UiAction
- `handle_message(AppMessage)` — handles API responses, notifications, images, toasts
- `render(&mut Frame)` — immediate-mode rendering of the full UI each frame

### Key Type Hierarchy

`src/messages.rs` defines all inter-actor types:
- **`UiAction`** — semantic actions from vim mode (ScrollDown, Like, Reply, etc.)
- **`ApiRequest` / `ApiResponse`** — typed API call pairs
- **`AppMessage`** — envelope routing Ui/Api/NotificationPoll/ImageReady/Toast to the app actor

### Pane System

Panes use a **binary tree** (`PaneTree` in `src/ui/workspace.rs`) — Split nodes have direction + ratio + two children; Leaf nodes reference a `PaneId`. Layout is computed by `pane_tree.layout(area) -> Vec<(PaneId, Rect)>`.

Pane state lives in `App.panes: HashMap<PaneId, Pane>`. Six pane kinds: Feed, Thread, Profile, DMs, Notifications, Compose. When API data arrives, the app searches all panes and updates matching ones.

Compose panes are created by splitting the focused pane, and `pre_compose_pane` tracks which pane to restore focus to on close.

### Vim Input

`VimState` (`src/input/vim.rs`) is a state machine processing KeyEvents into `Option<UiAction>`. Three modes: Normal, Insert, Command. Multi-key sequences (e.g., `gg`) use a `pending_key` buffer. The `Ctrl-w` prefix enables pane management sub-commands.

### API Layer

`src/api/client.rs` makes direct XRPC HTTP calls with bearer JWT auth (no jacquard high-level API — the type system was too restrictive). Each API request spawns independently in the API task for parallelism. Auth tokens come from `BlueskyAgent` which wraps jacquard's `MemoryCredentialSession` for login.

### Rendering

Immediate-mode: every frame rebuilds all widgets from current state. `src/ui/mod.rs::render()` lays out workspace tabs → pane tree → status bar → toast overlay → help overlay. Each pane kind has its own renderer in `src/ui/{feed,thread,profile,dms,notifications,compose}.rs`.

Post cards (`src/ui/widgets/post_card.rs`) render reply context, rich text with facets, inline quote posts with box-drawing borders, image thumbnails via `ratatui-image`, and engagement counts.

## Adding Features

**New keybinding**: Add `UiAction` variant in `messages.rs` → add key match in `vim.rs` → add handler in `app.rs::handle_ui_action()`.

**New API endpoint**: Add `ApiRequest`/`ApiResponse` variants → implement in `api/client.rs` → add dispatch in `api/mod.rs::handle_request()` → handle response in `app.rs::handle_api_response()`.

**New pane type**: Add `PaneKind` variant + state struct in `ui/pane.rs` → add renderer in `src/ui/` → add to `render_pane()` dispatch in `ui/mod.rs`.
