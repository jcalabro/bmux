mod api;
mod app;
mod auth;
mod config;
mod image;
mod input;
mod messages;
mod poll;
mod ui;

use anyhow::{Context, Result};
use app::App;
use clap::Parser;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use image::cache::ImageCache;
use messages::{ApiRequest, AppMessage};
use ratatui::prelude::*;
use std::io;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// bmux — the tmux of Bluesky
#[derive(Parser, Debug)]
#[command(name = "bmux", version, about = "The tmux of Bluesky")]
struct Cli {
    /// Bluesky handle or DID
    #[arg(short = 'u', long)]
    identifier: Option<String>,

    /// App password (use env BMUX_PASSWORD for security)
    #[arg(short = 'p', long)]
    password: Option<String>,

    /// PDS service URL
    #[arg(short = 's', long, default_value = "https://bsky.social")]
    service: String,

    /// Config file path
    #[arg(short = 'c', long)]
    config: Option<String>,

    /// Theme override
    #[arg(short = 't', long)]
    theme: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("bmux=info".parse().unwrap()),
        )
        .with_writer(io::stderr)
        .init();

    let cli = Cli::parse();

    // Load config.
    let mut app_config = if let Some(path) = &cli.config {
        config::load_config_from_path(std::path::Path::new(path))
    } else {
        config::load_config()
    };

    // Apply CLI overrides.
    if let Some(theme) = &cli.theme {
        app_config.general.theme = theme.clone();
    }

    // Ensure config dir exists.
    let _ = config::ensure_config_dir();

    // ── Authentication ──────────────────────────────────────

    let identifier = cli
        .identifier
        .or_else(|| app_config.auth.identifier.clone())
        .or_else(|| std::env::var("BMUX_IDENTIFIER").ok());

    let password = cli.password.or_else(|| std::env::var("BMUX_PASSWORD").ok());

    let service = app_config
        .auth
        .service
        .clone()
        .unwrap_or(cli.service.clone());

    let token_path = auth::oauth::token_file_path(app_config.auth.token_file.as_deref());
    let redirect_port = app_config.auth.redirect_port;

    // Auth priority:
    // 1. Try to restore a saved OAuth session
    // 2. If --password or $BMUX_PASSWORD is provided, use app password
    // 3. Otherwise, run the OAuth browser flow
    let agent = if let Some(id) = &identifier {
        // Try restoring an existing OAuth session first.
        if let Some(agent) = auth::oauth::try_restore_session(&token_path, id).await {
            eprintln!("Restored OAuth session for @{}", agent.handle());
            agent
        } else if let Some(pw) = &password {
            // Fall back to app password.
            eprintln!("Logging in as {} with app password...", id);
            auth::login_with_app_password(&service, id, pw)
                .await
                .context("App password login failed")?
        } else {
            // No password provided — use OAuth browser flow.
            auth::oauth::login_with_browser(&token_path, id, redirect_port)
                .await
                .context("OAuth login failed")?
        }
    } else {
        // No identifier provided at all. Try restoring any saved session.
        if let Some(agent) = auth::oauth::try_restore_session(&token_path, "").await {
            eprintln!("Restored OAuth session for @{}", agent.handle());
            agent
        } else if let Some(pw) = &password {
            // Password without identifier — prompt for identifier.
            eprint!("Bluesky handle: ");
            let mut id = String::new();
            io::stdin().read_line(&mut id)?;
            let id = id.trim().to_string();
            eprintln!("Logging in as {} with app password...", id);
            auth::login_with_app_password(&service, &id, pw)
                .await
                .context("App password login failed")?
        } else {
            // No identifier, no password — prompt for handle and use OAuth.
            eprint!("Bluesky handle: ");
            let mut id = String::new();
            io::stdin().read_line(&mut id)?;
            let id = id.trim().to_string();
            auth::oauth::login_with_browser(&token_path, &id, redirect_port)
                .await
                .context("OAuth login failed")?
        }
    };

    let user_handle = agent.handle().to_string();
    eprintln!("Logged in as @{}! Starting bmux...", user_handle);

    // ── Set up channels ─────────────────────────────────────

    let (app_tx, mut app_rx) = mpsc::channel::<AppMessage>(256);
    let (api_tx, api_rx) = mpsc::channel::<ApiRequest>(128);
    let (img_tx, img_rx) = mpsc::channel::<messages::ImageRequest>(64);

    // ── Spawn background tasks ──────────────────────────────

    let api_agent = agent.clone();
    let api_app_tx = app_tx.clone();
    tokio::spawn(async move {
        api::run_api_task(api_agent, api_rx, api_app_tx).await;
    });

    let poll_agent = agent.clone();
    let poll_tx = app_tx.clone();
    let poll_interval = app_config.general.poll_interval_secs;
    tokio::spawn(async move {
        poll::run_poll_task(poll_agent, poll_tx, poll_interval).await;
    });

    let image_cache = Arc::new(Mutex::new(ImageCache::new(100)));
    let img_app_tx = app_tx.clone();
    tokio::spawn(async move {
        crate::image::run_image_task(img_rx, img_app_tx, image_cache).await;
    });

    // ── Detect image protocol ────────────────────────────────
    // Must happen before entering raw mode / alternate screen.

    let picker = ratatui_image::picker::Picker::from_query_stdio()
        .unwrap_or_else(|_| ratatui_image::picker::Picker::halfblocks());

    // ── Create the App ──────────────────────────────────────

    let mut app = App::new(app_config, api_tx, img_tx, user_handle, picker);
    app.request_initial_data();

    // ── Set up terminal ─────────────────────────────────────

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // ── Spawn input reader thread ───────────────────────────
    // crossterm::event::read() is blocking, so we run it in a dedicated thread
    // and send key events over a channel.

    let (key_tx, mut key_rx) = mpsc::channel::<KeyEvent>(64);
    let input_paused = app.input_paused.clone();
    std::thread::spawn(move || {
        loop {
            // When paused (editor is open), sleep instead of reading stdin.
            if input_paused.load(std::sync::atomic::Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }

            // Use poll + read so we can check the pause flag periodically.
            match event::poll(std::time::Duration::from_millis(10)) {
                Ok(true) => match event::read() {
                    Ok(Event::Key(key_event)) => {
                        if key_event.kind != KeyEventKind::Press {
                            continue;
                        }
                        if key_tx.blocking_send(key_event).is_err() {
                            break;
                        }
                    }
                    Ok(Event::Resize(_w, _h)) => {}
                    Ok(_) => {}
                    Err(_) => break,
                },
                Ok(false) => {} // timeout, loop back to check pause flag
                Err(_) => break,
            }
        }
    });

    // ── Main event loop ─────────────────────────────────────

    loop {
        if app.needs_full_redraw {
            app.needs_full_redraw = false;
            terminal.clear()?;
        }
        terminal.draw(|frame| app.render(frame))?;

        tokio::select! {
            key = key_rx.recv() => {
                match key {
                    Some(key_event) => {
                        app.handle_key_event(key_event);
                    }
                    None => break,
                }
            }
            msg = app_rx.recv() => {
                match msg {
                    Some(msg) => app.handle_message(msg),
                    None => break,
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                app.tick();
            }
        }

        if app.should_quit {
            break;
        }
    }

    // ── Cleanup ─────────────────────────────────────────────

    terminal::disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

/// Read a line from stdin (for password input).
#[allow(dead_code)]
fn read_password() -> Result<String> {
    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    let password = password.trim().to_string();
    Ok(password)
}
