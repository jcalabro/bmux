use crate::auth::AppAgent;
use crate::messages::{AppMessage, Notification, Toast};
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

/// Poll task: periodically checks for new notifications.
pub async fn run_poll_task(
    agent: AppAgent,
    tx: mpsc::Sender<AppMessage>,
    interval_secs: u64,
) {
    let mut interval = time::interval(Duration::from_secs(interval_secs));
    let mut last_unread_count = 0usize;

    loop {
        interval.tick().await;

        match crate::api::client::fetch_notifications(&agent, None).await {
            Ok((notifications, _, unread_count)) => {
                // Send notification update.
                let _ = tx
                    .send(AppMessage::NotificationPoll(
                        notifications,
                        unread_count,
                    ))
                    .await;

                // Show toast if new notifications arrived.
                if unread_count > last_unread_count {
                    let new_count = unread_count - last_unread_count;
                    let msg = if new_count == 1 {
                        "1 new notification".to_string()
                    } else {
                        format!("{} new notifications", new_count)
                    };
                    let _ = tx.send(AppMessage::Toast(Toast::info(msg))).await;
                }

                last_unread_count = unread_count;
            }
            Err(e) => {
                tracing::warn!("Poll task: failed to fetch notifications: {}", e);
            }
        }
    }
}
