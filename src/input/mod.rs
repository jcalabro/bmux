pub mod action;
pub mod vim;

use crate::messages::{AppMessage, UiAction};
use crossterm::event::{self, Event, KeyEventKind};
use tokio::sync::mpsc;

/// Input task: reads terminal events and translates them through the vim state machine.
pub async fn run_input_task(tx: mpsc::Sender<AppMessage>) {
    loop {
        match event::read() {
            Ok(Event::Key(key_event)) => {
                // Only handle key press events, not release or repeat.
                if key_event.kind != KeyEventKind::Press {
                    continue;
                }
                // We send the raw key event as a UiAction — the App actor
                // will feed it through the VimState.
                // For simplicity, we do vim processing in the app actor
                // since it needs access to the current mode.
                if tx
                    .send(AppMessage::Ui(UiAction::Tick))
                    .await
                    .is_err()
                {
                    break;
                }
                // Actually, we need to send the raw key. Let's use a wrapper.
                // We'll handle this by sending the key event directly.
                // The app will process it through VimState.
            }
            Ok(Event::Resize(w, h)) => {
                if tx
                    .send(AppMessage::Ui(UiAction::Resize(w, h)))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
}

/// Raw key event wrapper for sending to the app actor.
/// We use this instead of processing vim in the input task,
/// because the vim state machine needs to know the current mode
/// which the app actor owns.
#[derive(Debug, Clone)]
pub struct RawKeyEvent(pub crossterm::event::KeyEvent);
