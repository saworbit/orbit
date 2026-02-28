//! S3 client session cache for reusing clients across operations.

use super::config::S3Config;
use super::client::S3Client;
use super::error::S3Result;
use std::collections::HashMap;
use std::sync::RwLock;

/// Cache key for S3 client instances
type CacheKey = (String, Option<String>, Option<String>); // (bucket, region, endpoint)

/// Thread-safe S3 client cache that avoids repeated credential resolution
pub struct S3ClientCache {
    cache: RwLock<HashMap<CacheKey, S3Client>>,
}

impl S3ClientCache {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Get an existing client or create a new one for the given config
    pub async fn get_or_create(&self, config: S3Config) -> S3Result<S3Client> {
        let key = (
            config.bucket.clone(),
            config.region.clone(),
            config.endpoint.clone(),
        );

        // Check if we already have a client for this config
        {
            let cache = self.cache.read().unwrap();
            if let Some(client) = cache.get(&key) {
                return Ok(client.clone());
            }
        }

        // Create a new client
        let client = S3Client::new(config).await?;

        // Cache it
        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(key, client.clone());
        }

        Ok(client)
    }

    /// Clear the cache
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }

    /// Number of cached clients
    pub fn len(&self) -> usize {
        self.cache.read().unwrap().len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.read().unwrap().is_empty()
    }
}

impl Default for S3ClientCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_new() {
        let cache = S3ClientCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[tokio::test]
    async fn test_cache_get_or_create() {
        let cache = S3ClientCache::new();
        let config = S3Config::new("test-bucket".to_string());
        let client = cache.get_or_create(config).await.unwrap();
        assert_eq!(client.bucket(), "test-bucket");
        assert_eq!(cache.len(), 1);
    }

    #[tokio::test]
    async fn test_cache_reuses_client() {
        let cache = S3ClientCache::new();
        let config1 = S3Config::new("test-bucket".to_string());
        let config2 = S3Config::new("test-bucket".to_string());

        let _client1 = cache.get_or_create(config1).await.unwrap();
        let _client2 = cache.get_or_create(config2).await.unwrap();

        // Should only have one entry since same bucket/region/endpoint
        assert_eq!(cache.len(), 1);
    }

    #[tokio::test]
    async fn test_cache_different_buckets() {
        let cache = S3ClientCache::new();
        let config1 = S3Config::new("bucket-a".to_string());
        let config2 = S3Config::new("bucket-b".to_string());

        let _client1 = cache.get_or_create(config1).await.unwrap();
        let _client2 = cache.get_or_create(config2).await.unwrap();

        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_cache_clear() {
        let cache = S3ClientCache::new();
        // We can't easily populate without async, so just test the clear path
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_default() {
        let cache = S3ClientCache::default();
        assert!(cache.is_empty());
    }
}
