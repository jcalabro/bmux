pub mod cache;
pub mod kitty;
pub mod sixel;

use crate::messages::{AppMessage, ImageData, ImageRequest};
use cache::{ImageCache, ImageCacheKey};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Image task: downloads images and sends decoded DynamicImage data back.
pub async fn run_image_task(
    mut rx: mpsc::Receiver<ImageRequest>,
    tx: mpsc::Sender<AppMessage>,
    cache: Arc<Mutex<ImageCache>>,
) {
    let client = reqwest::Client::new();

    while let Some(request) = rx.recv().await {
        let cache_key = ImageCacheKey {
            url: request.url.clone(),
            width: request.max_width,
            height: request.max_height,
        };

        // Check cache first.
        let cached = {
            let mut c = cache.lock().unwrap();
            c.get(&cache_key).cloned()
        };
        if let Some(data) = cached {
            let _ = tx
                .send(AppMessage::ImageReady {
                    url: request.url.clone(),
                    data,
                })
                .await;
            continue;
        }

        let client = client.clone();
        let tx = tx.clone();
        let cache = cache.clone();

        tokio::spawn(async move {
            let data = fetch_image(&client, &request).await;

            // Cache the result.
            {
                let mut c = cache.lock().unwrap();
                c.put(cache_key, data.clone());
            }

            let _ = tx
                .send(AppMessage::ImageReady {
                    url: request.url,
                    data,
                })
                .await;
        });
    }
}

async fn fetch_image(client: &reqwest::Client, request: &ImageRequest) -> ImageData {
    let bytes = match client.get(&request.url).send().await {
        Ok(resp) => match resp.bytes().await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("Failed to download image {}: {}", request.url, e);
                return ImageData::AltText("[image: download failed]".to_string());
            }
        },
        Err(e) => {
            tracing::warn!("Failed to fetch image {}: {}", request.url, e);
            return ImageData::AltText("[image: fetch failed]".to_string());
        }
    };

    // Return the raw bytes -- the rendering thread will use ratatui-image's
    // Picker to create a protocol-specific encoding at render time.
    ImageData::RawBytes(bytes.to_vec())
}
