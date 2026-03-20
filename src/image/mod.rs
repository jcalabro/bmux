pub mod cache;
pub mod kitty;
pub mod sixel;

use crate::messages::{AppMessage, ImageData, ImageRequest};
use crate::ui::widgets::image::ImageProtocol;
use cache::{ImageCache, ImageCacheKey};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Image task: decodes images and sends them back to the app.
pub async fn run_image_task(
    mut rx: mpsc::Receiver<ImageRequest>,
    tx: mpsc::Sender<AppMessage>,
    protocol: ImageProtocol,
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
            let data = fetch_and_encode(&client, &request, protocol).await;

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

async fn fetch_and_encode(
    client: &reqwest::Client,
    request: &ImageRequest,
    protocol: ImageProtocol,
) -> ImageData {
    // Download the image.
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

    // Decode the image.
    let img = match image::load_from_memory(&bytes) {
        Ok(img) => img,
        Err(e) => {
            tracing::warn!("Failed to decode image {}: {}", request.url, e);
            return ImageData::AltText("[image: decode failed]".to_string());
        }
    };

    // Encode for the target protocol.
    match protocol {
        ImageProtocol::Sixel => {
            ImageData::Sixel(sixel::encode_sixel(&img, request.max_width, request.max_height))
        }
        ImageProtocol::Kitty => {
            ImageData::Kitty(kitty::encode_kitty(&img, request.max_width, request.max_height))
        }
        ImageProtocol::None => {
            let (w, h) = (img.width(), img.height());
            ImageData::AltText(format!("[image: {}x{}]", w, h))
        }
    }
}
