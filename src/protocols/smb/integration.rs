//! Integration helpers for Orbit's pipeline system
//!
//! Provides functions to integrate SMB transfers with Orbit's
//! manifest-based block streaming architecture.

use super::{SmbClient, SmbError};
use bytes::Bytes;

/// Pull a block from an SMB file using range reads
///
/// Aligns with Orbit's manifest block model for efficient resumable transfers.
///
/// # Arguments
///
/// * `client` - SMB client instance
/// * `rel` - Relative path to file within the share
/// * `block_ix` - Zero-based block index
/// * `block_size` - Size of each block in bytes
///
/// # Returns
///
/// Returns the requested block data as `Bytes`
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "smb-native")]
/// # {
/// use orbit::protocols::smb::{SmbTarget, SmbAuth, SmbSecurity, client_for};
/// use orbit::protocols::smb::integration::stream_pull;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let target = SmbTarget {
///     host: "server".to_string(),
///     share: "data".to_string(),
///     subpath: "".to_string(),
///     port: None,
///     auth: SmbAuth::Anonymous,
///     security: SmbSecurity::Opportunistic,
/// };
///
/// let client = client_for(&target).await?;
/// let block_size = 4 * 1024 * 1024; // 4MB blocks
/// let block_data = stream_pull(&*client, "large_file.bin", 0, block_size).await?;
/// # Ok(())
/// # }
/// # }
/// ```
pub async fn stream_pull(
    client: &dyn SmbClient,
    rel: &str,
    block_ix: u64,
    block_size: u64,
) -> Result<Bytes, SmbError> {
    let start = block_ix * block_size;
    let end = start + block_size;
    client.read_file(rel, Some(start..end)).await
}

/// Push a block to an SMB file
///
/// Currently writes the entire block. Future versions will support
/// write-at-offset for true resumability.
///
/// # Arguments
///
/// * `client` - SMB client instance
/// * `rel` - Relative path to file within the share
/// * `block_ix` - Zero-based block index (currently unused, reserved for future)
/// * `data` - Block data to write
/// * `block_size` - Size of each block in bytes (currently unused, reserved for future)
///
/// # Returns
///
/// Returns `Ok(())` on success
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "smb-native")]
/// # {
/// use orbit::protocols::smb::{SmbTarget, SmbAuth, SmbSecurity, client_for};
/// use orbit::protocols::smb::integration::stream_push;
/// use bytes::Bytes;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let target = SmbTarget {
///     host: "server".to_string(),
///     share: "data".to_string(),
///     subpath: "".to_string(),
///     port: None,
///     auth: SmbAuth::Anonymous,
///     security: SmbSecurity::Opportunistic,
/// };
///
/// let client = client_for(&target).await?;
/// let block_size = 4 * 1024 * 1024; // 4MB blocks
/// let data = Bytes::from(vec![0u8; block_size as usize]);
/// stream_push(&*client, "output_file.bin", 0, data, block_size).await?;
/// # Ok(())
/// # }
/// # }
/// ```
pub async fn stream_push(
    client: &dyn SmbClient,
    rel: &str,
    _block_ix: u64,
    data: Bytes,
    _block_size: u64,
) -> Result<(), SmbError> {
    // TODO: Future enhancement - use block_ix and block_size for write-at-offset
    // Currently writes the entire data block
    client.write_file(rel, data).await
}

/// Pull multiple blocks in parallel (future enhancement)
///
/// This is a placeholder for future multi-block parallel reads.
#[allow(dead_code)]
pub async fn stream_pull_batch(
    client: &dyn SmbClient,
    rel: &str,
    block_indices: &[u64],
    block_size: u64,
) -> Result<Vec<Bytes>, SmbError> {
    let mut results = Vec::with_capacity(block_indices.len());

    for &block_ix in block_indices {
        let data = stream_pull(client, rel, block_ix, block_size).await?;
        results.push(data);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::ops::Range;

    // Mock client for testing
    struct MockClient;

    #[async_trait]
    impl SmbClient for MockClient {
        async fn connect(&mut self, _: &super::super::SmbTarget) -> Result<(), SmbError> {
            Ok(())
        }

        async fn list_dir(&self, _: &str) -> Result<Vec<String>, SmbError> {
            Ok(vec![])
        }

        async fn read_file(&self, _: &str, range: Option<Range<u64>>) -> Result<Bytes, SmbError> {
            let range = range.unwrap_or(0..1024);
            let len = (range.end - range.start) as usize;
            Ok(Bytes::from(vec![0u8; len]))
        }

        async fn write_file(&self, _: &str, _: Bytes) -> Result<(), SmbError> {
            Ok(())
        }

        async fn mkdir(&self, _: &str) -> Result<(), SmbError> {
            Ok(())
        }

        async fn remove(&self, _: &str) -> Result<(), SmbError> {
            Ok(())
        }

        async fn rename(&self, _: &str, _: &str) -> Result<(), SmbError> {
            Ok(())
        }

        async fn metadata(&self, _: &str) -> Result<super::super::SmbMetadata, SmbError> {
            Ok(super::super::SmbMetadata {
                size: 0,
                is_dir: false,
                modified: None,
                encrypted: false,
            })
        }

        async fn disconnect(&mut self) -> Result<(), SmbError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_stream_pull() {
        let client = MockClient;
        let block_size = 4096;
        let data = stream_pull(&client, "test.bin", 0, block_size)
            .await
            .unwrap();
        assert_eq!(data.len(), block_size as usize);
    }

    #[tokio::test]
    async fn test_stream_push() {
        let client = MockClient;
        let data = Bytes::from(vec![1u8; 4096]);
        let result = stream_push(&client, "test.bin", 0, data, 4096).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stream_pull_batch() {
        let client = MockClient;
        let block_indices = vec![0, 1, 2];
        let block_size = 4096;
        let results = stream_pull_batch(&client, "test.bin", &block_indices, block_size)
            .await
            .unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].len(), block_size as usize);
    }
}
