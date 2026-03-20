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
use ui::widgets::image::parse_image_protocol;

/// alf — the tmux of Bluesky
#[derive(Parser, Debug)]
#[command(name = "alf", version, about = "The tmux of Bluesky")]
struct Cli {
    /// Bluesky handle or DID
    #[arg(short = 'u', long)]
    identifier: Option<String>,

    /// App password (use env ALF_PASSWORD for security)
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
                .add_directive("alf=info".parse().unwrap()),
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
        .or_else(|| std::env::var("ALF_IDENTIFIER").ok());

    let password = cli
        .password
        .or_else(|| std::env::var("ALF_PASSWORD").ok());

    let service = app_config
        .auth
        .service
        .clone()
        .unwrap_or(cli.service.clone());

    let (identifier, password) = match (identifier, password) {
        (Some(id), Some(pw)) => (id, pw),
        (Some(id), None) => {
            eprint!("Password for {}: ", id);
            let pw = read_password()?;
            (id, pw)
        }
        _ => {
            eprint!("Bluesky handle: ");
            let mut id = String::new();
            io::stdin().read_line(&mut id)?;
            let id = id.trim().to_string();
            eprint!("Password: ");
            let pw = read_password()?;
            (id, pw)
        }
    };

    eprintln!("Logging in as {}...", identifier);
    let agent = auth::login_with_app_password(&service, &identifier, &password)
        .await
        .context("Login failed")?;

    let user_handle = agent.handle.clone();
    eprintln!("Logged in as @{}! Starting alf...", user_handle);

    // ── Set up channels ─────────────────────────────────────

    let (app_tx, mut app_rx) = mpsc::channel::<AppMessage>(256);
    let (api_tx, api_rx) = mpsc::channel::<ApiRequest>(128);
    let (_img_tx, img_rx) = mpsc::channel::<messages::ImageRequest>(64);

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

    let image_protocol = parse_image_protocol(&app_config.general.image_protocol);
    let image_cache = Arc::new(Mutex::new(ImageCache::new(100)));
    let img_app_tx = app_tx.clone();
    tokio::spawn(async move {
        crate::image::run_image_task(img_rx, img_app_tx, image_protocol, image_cache).await;
    });

    // ── Create the App ──────────────────────────────────────

    let mut app = App::new(app_config, api_tx, user_handle);
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
    std::thread::spawn(move || {
        loop {
            match event::read() {
                Ok(Event::Key(key_event)) => {
                    if key_event.kind != KeyEventKind::Press {
                        continue;
                    }
                    if key_tx.blocking_send(key_event).is_err() {
                        break;
                    }
                }
                Ok(Event::Resize(_w, _h)) => {
                    // We could send resize events too, but the terminal
                    // handles this automatically on the next draw.
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });

    // ── Main event loop ─────────────────────────────────────

    loop {
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
fn read_password() -> Result<String> {
    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    let password = password.trim().to_string();
    Ok(password)
}
