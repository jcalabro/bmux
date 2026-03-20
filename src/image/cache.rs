use crate::messages::ImageData;
use lru::LruCache;
use std::num::NonZeroUsize;

/// Cache key for decoded images.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ImageCacheKey {
    pub url: String,
    pub width: u16,
    pub height: u16,
}

/// LRU cache for decoded image data.
pub struct ImageCache {
    cache: LruCache<ImageCacheKey, ImageData>,
}

impl ImageCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity.max(1)).unwrap()),
        }
    }

    pub fn get(&mut self, key: &ImageCacheKey) -> Option<&ImageData> {
        self.cache.get(key)
    }

    pub fn put(&mut self, key: ImageCacheKey, data: ImageData) {
        self.cache.put(key, data);
    }

    #[allow(dead_code)]
    pub fn contains(&self, key: &ImageCacheKey) -> bool {
        self.cache.contains(key)
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_cache() {
        let mut cache = ImageCache::new(2);
        let key1 = ImageCacheKey {
            url: "http://example.com/1.jpg".into(),
            width: 100,
            height: 100,
        };
        let key2 = ImageCacheKey {
            url: "http://example.com/2.jpg".into(),
            width: 100,
            height: 100,
        };
        let key3 = ImageCacheKey {
            url: "http://example.com/3.jpg".into(),
            width: 100,
            height: 100,
        };

        cache.put(key1.clone(), ImageData::AltText("img1".into()));
        cache.put(key2.clone(), ImageData::AltText("img2".into()));
        assert_eq!(cache.len(), 2);

        // Adding third should evict first (LRU).
        cache.put(key3.clone(), ImageData::AltText("img3".into()));
        assert_eq!(cache.len(), 2);
        assert!(cache.get(&key1).is_none());
        assert!(cache.get(&key3).is_some());
    }
}
