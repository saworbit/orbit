/*!
 * S3 command handlers for Orbit CLI
 *
 * Contains all S3-specific subcommand implementations:
 * streaming (cat, pipe, presign) and object management (ls, head, du, rm, mv, mb, rb).
 */

use crate::cli_style::{format_bytes, print_info, section_header, Icons, Theme};
use crate::error::{OrbitError, Result};
use crate::protocol::Protocol;

fn parse_s3_uri(uri: &str, command: &str, usage: &str) -> Result<(String, String)> {
    let (protocol, key_path) = Protocol::from_uri(uri)?;
    match protocol {
        Protocol::S3 { bucket, .. } => Ok((
            bucket,
            key_path
                .to_string_lossy()
                .trim_start_matches('/')
                .to_string(),
        )),
        _ => Err(OrbitError::Config(format!(
            "{} command requires an S3 URI ({})",
            command, usage
        ))),
    }
}

fn parse_bucket_name(bucket_uri: &str, command: &str) -> Result<String> {
    let bucket_name = match Protocol::from_uri(bucket_uri) {
        Ok((Protocol::S3 { bucket, .. }, _)) => bucket,
        _ => bucket_uri
            .trim_start_matches("s3://")
            .trim_end_matches('/')
            .to_string(),
    };

    if bucket_name.is_empty() {
        return Err(OrbitError::Config(format!(
            "{} command requires a bucket name (s3://bucket-name)",
            command
        )));
    }

    Ok(bucket_name)
}

// ============================================================================
// S3 Streaming Commands (cat, pipe, presign)
// ============================================================================

pub fn handle_cat_command(uri: &str) -> Result<()> {
    use crate::protocol::s3::{S3Client, S3Config, S3Operations};
    use std::io::Write;

    let (bucket, key) = parse_s3_uri(uri, "cat", "s3://bucket/key")?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket);
        let client = S3Client::new(config)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        let data = client
            .download_bytes(&key)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to download: {}", e)))?;

        std::io::stdout()
            .write_all(&data)
            .map_err(|e| OrbitError::Other(format!("Failed to write to stdout: {}", e)))?;
        std::io::stdout()
            .flush()
            .map_err(|e| OrbitError::Other(format!("Failed to flush stdout: {}", e)))?;

        Ok(())
    })
}

pub fn handle_pipe_command(uri: &str) -> Result<()> {
    use crate::protocol::s3::{S3Client, S3Config, S3Operations};
    use bytes::Bytes;
    use std::io::Read;

    let (bucket, key) = parse_s3_uri(uri, "pipe", "s3://bucket/key")?;

    // Read all stdin into memory
    let mut buffer = Vec::new();
    std::io::stdin()
        .read_to_end(&mut buffer)
        .map_err(|e| OrbitError::Other(format!("Failed to read from stdin: {}", e)))?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket);
        let client = S3Client::new(config)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        client
            .upload_bytes(Bytes::from(buffer), &key)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to upload: {}", e)))?;

        eprintln!("Uploaded to s3://{}", key);
        Ok(())
    })
}

pub fn handle_presign_command(uri: &str, expires: u64) -> Result<()> {
    use crate::protocol::s3::{S3Client, S3Config};

    let (bucket, key) = parse_s3_uri(uri, "presign", "s3://bucket/key")?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket.clone());
        let client = S3Client::new(config)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        let url = client
            .presign_get(&key, std::time::Duration::from_secs(expires))
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to generate pre-signed URL: {}", e)))?;

        println!("{}", url);
        Ok(())
    })
}

// ============================================================================
// S3 Object Management Commands (ls, head, du, rm, mv, mb, rb)
// ============================================================================

pub fn handle_ls_command(
    uri: &str,
    show_etag: bool,
    show_storage_class: bool,
    all_versions: bool,
    show_fullpath: bool,
) -> Result<()> {
    use crate::protocol::s3::{
        has_wildcards, S3Client, S3Config, S3Operations, VersioningOperations,
    };

    let (bucket, key) = parse_s3_uri(uri, "ls", "s3://bucket/prefix")?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket.clone());
        let client = S3Client::new(config)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        if all_versions {
            let result = client
                .list_object_versions(&key)
                .await
                .map_err(|e| OrbitError::Other(format!("Failed to list versions: {}", e)))?;

            for version in &result.versions {
                let date_str = format_system_time(version.last_modified);
                let size_str = format_bytes(version.size);
                let key_display = if show_fullpath {
                    format!("s3://{}/{}", bucket, version.key)
                } else {
                    version.key.clone()
                };
                let latest_marker = if version.is_latest { " [LATEST]" } else { "" };

                print!(
                    "{}  {:>10}  {}  {}{}",
                    date_str, size_str, version.version_id, key_display, latest_marker
                );
                if let Some(ref sc) = version.storage_class {
                    if show_storage_class {
                        print!("  {}", sc);
                    }
                }
                println!();
            }

            for dm in &result.delete_markers {
                let date_str = format_system_time(dm.last_modified);
                let key_display = if show_fullpath {
                    format!("s3://{}/{}", bucket, dm.key)
                } else {
                    dm.key.clone()
                };
                println!(
                    "{}  {:>10}  {}  {} [DELETE MARKER]",
                    date_str, "(marker)", dm.version_id, key_display
                );
            }

            let total = result.versions.len() + result.delete_markers.len();
            eprintln!(
                "\n{} versions, {} delete markers",
                result.versions.len(),
                result.delete_markers.len()
            );
            if total == 0 {
                eprintln!("No objects found.");
            }
        } else {
            let mut all_objects = Vec::new();
            let use_wildcard = has_wildcards(&key);

            if use_wildcard {
                let result = client
                    .list_objects_with_wildcard(&key)
                    .await
                    .map_err(|e| OrbitError::Other(format!("Failed to list objects: {}", e)))?;
                all_objects = result.objects;
            } else {
                let mut continuation_token = None;
                loop {
                    let result = client
                        .list_objects_paginated(&key, continuation_token, None)
                        .await
                        .map_err(|e| OrbitError::Other(format!("Failed to list objects: {}", e)))?;
                    all_objects.extend(result.objects);
                    if result.is_truncated {
                        continuation_token = result.continuation_token;
                    } else {
                        break;
                    }
                }
            }

            for obj in &all_objects {
                let date_str = obj
                    .last_modified
                    .map(format_system_time)
                    .unwrap_or_else(|| "                   ".to_string());
                let size_str = format_bytes(obj.size);
                let key_display = if show_fullpath {
                    format!("s3://{}/{}", bucket, obj.key)
                } else {
                    obj.key.clone()
                };

                print!("{}  {:>10}  {}", date_str, size_str, key_display);

                if show_etag {
                    if let Some(ref etag) = obj.etag {
                        print!("  {}", etag);
                    }
                }
                if show_storage_class {
                    if let Some(ref sc) = obj.storage_class {
                        print!("  {}", sc);
                    }
                }
                println!();
            }

            if all_objects.is_empty() {
                eprintln!("No objects found.");
            } else {
                let total_size: u64 = all_objects.iter().map(|o| o.size).sum();
                eprintln!(
                    "\n{} objects, {} total",
                    all_objects.len(),
                    format_bytes(total_size)
                );
            }
        }

        Ok(())
    })
}

pub fn handle_head_command(uri: &str, version_id: Option<String>) -> Result<()> {
    use crate::protocol::s3::{S3Client, S3Config, VersioningOperations};

    let (bucket, key) = parse_s3_uri(uri, "head", "s3://bucket/key")?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket.clone());
        let client = S3Client::new(config)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        if let Some(ref vid) = version_id {
            let version = client
                .get_version_metadata(&key, vid)
                .await
                .map_err(|e| OrbitError::Other(format!("Failed to get version metadata: {}", e)))?;

            section_header(&format!("{} S3 Object Version", Icons::FILE));
            println!();
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("Key:"),
                Theme::value(&version.key)
            );
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("Version ID:"),
                Theme::value(&version.version_id)
            );
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("Size:"),
                Theme::value(format_bytes(version.size))
            );
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("Last Modified:"),
                Theme::value(format_system_time(version.last_modified))
            );
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("ETag:"),
                Theme::value(&version.etag)
            );
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("Is Latest:"),
                Theme::value(version.is_latest)
            );
            if let Some(ref sc) = version.storage_class {
                println!(
                    "  {} {} {}",
                    Icons::BULLET,
                    Theme::muted("Storage Class:"),
                    Theme::value(sc)
                );
            }
            println!();
        } else {
            let metadata = client
                .get_metadata(&key)
                .await
                .map_err(|e| OrbitError::Other(format!("Failed to get metadata: {}", e)))?;

            section_header(&format!("{} S3 Object Metadata", Icons::FILE));
            println!();
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("Key:"),
                Theme::value(&metadata.key)
            );
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("Size:"),
                Theme::value(format_bytes(metadata.size))
            );
            if let Some(ref lm) = metadata.last_modified {
                println!(
                    "  {} {} {}",
                    Icons::BULLET,
                    Theme::muted("Last Modified:"),
                    Theme::value(format_system_time(*lm))
                );
            }
            if let Some(ref etag) = metadata.etag {
                println!(
                    "  {} {} {}",
                    Icons::BULLET,
                    Theme::muted("ETag:"),
                    Theme::value(etag)
                );
            }
            if let Some(ref sc) = metadata.storage_class {
                println!(
                    "  {} {} {}",
                    Icons::BULLET,
                    Theme::muted("Storage Class:"),
                    Theme::value(sc)
                );
            }
            if let Some(ref ct) = metadata.content_type {
                println!(
                    "  {} {} {}",
                    Icons::BULLET,
                    Theme::muted("Content-Type:"),
                    Theme::value(ct)
                );
            }
            if let Some(ref ce) = metadata.content_encoding {
                println!(
                    "  {} {} {}",
                    Icons::BULLET,
                    Theme::muted("Content-Encoding:"),
                    Theme::value(ce)
                );
            }
            if let Some(ref cc) = metadata.cache_control {
                println!(
                    "  {} {} {}",
                    Icons::BULLET,
                    Theme::muted("Cache-Control:"),
                    Theme::value(cc)
                );
            }
            if let Some(ref cd) = metadata.content_disposition {
                println!(
                    "  {} {} {}",
                    Icons::BULLET,
                    Theme::muted("Content-Disposition:"),
                    Theme::value(cd)
                );
            }
            if let Some(ref vid) = metadata.version_id {
                println!(
                    "  {} {} {}",
                    Icons::BULLET,
                    Theme::muted("Version ID:"),
                    Theme::value(vid)
                );
            }
            if let Some(ref sse) = metadata.server_side_encryption {
                println!(
                    "  {} {} {:?}",
                    Icons::BULLET,
                    Theme::muted("Encryption:"),
                    sse
                );
            }
            if !metadata.metadata.is_empty() {
                println!("  {} {}", Icons::BULLET, Theme::muted("User Metadata:"));
                for (k, v) in &metadata.metadata {
                    println!("    {} = {}", Theme::muted(k), Theme::value(v));
                }
            }
            println!();
        }

        Ok(())
    })
}

pub fn handle_du_command(uri: &str, group: bool, all_versions: bool) -> Result<()> {
    use crate::protocol::s3::{S3Client, S3Config, S3Operations, VersioningOperations};
    use std::collections::HashMap;

    let (bucket, key) = parse_s3_uri(uri, "du", "s3://bucket/prefix")?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket.clone());
        let client = S3Client::new(config)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        section_header(&format!("{} S3 Storage Usage", Icons::STATS));
        println!();
        println!(
            "  {} {} s3://{}/{}",
            Icons::BULLET,
            Theme::muted("Prefix:"),
            bucket,
            key
        );
        println!();

        if all_versions {
            let result = client
                .list_object_versions(&key)
                .await
                .map_err(|e| OrbitError::Other(format!("Failed to list versions: {}", e)))?;

            let total_count = result.versions.len() as u64;
            let total_size: u64 = result.versions.iter().map(|v| v.size).sum();

            if group {
                let mut groups: HashMap<String, (u64, u64)> = HashMap::new();
                for version in &result.versions {
                    let sc = version
                        .storage_class
                        .clone()
                        .unwrap_or_else(|| "STANDARD".to_string());
                    let entry = groups.entry(sc).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += version.size;
                }
                for (class, (count, size)) in &groups {
                    println!(
                        "  {} {:>10}  {:>8} objects  {}",
                        Icons::BULLET,
                        format_bytes(*size),
                        count,
                        Theme::value(class)
                    );
                }
                println!();
            }

            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("Total objects (all versions):"),
                Theme::value(total_count)
            );
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("Total size:"),
                Theme::value(format_bytes(total_size))
            );
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("Delete markers:"),
                Theme::value(result.delete_markers.len())
            );
        } else {
            let mut all_objects = Vec::new();
            let mut continuation_token = None;
            loop {
                let result = client
                    .list_objects_paginated(&key, continuation_token, None)
                    .await
                    .map_err(|e| OrbitError::Other(format!("Failed to list objects: {}", e)))?;
                all_objects.extend(result.objects);
                if result.is_truncated {
                    continuation_token = result.continuation_token;
                } else {
                    break;
                }
            }

            let total_count = all_objects.len() as u64;
            let total_size: u64 = all_objects.iter().map(|o| o.size).sum();

            if group {
                let mut groups: HashMap<String, (u64, u64)> = HashMap::new();
                for obj in &all_objects {
                    let sc = obj
                        .storage_class
                        .as_ref()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "STANDARD".to_string());
                    let entry = groups.entry(sc).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += obj.size;
                }
                for (class, (count, size)) in &groups {
                    println!(
                        "  {} {:>10}  {:>8} objects  {}",
                        Icons::BULLET,
                        format_bytes(*size),
                        count,
                        Theme::value(class)
                    );
                }
                println!();
            }

            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("Total objects:"),
                Theme::value(total_count)
            );
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("Total size:"),
                Theme::value(format_bytes(total_size))
            );
        }

        println!();
        Ok(())
    })
}

pub fn handle_rm_command(
    uri: &str,
    all_versions: bool,
    version_id: Option<String>,
    dry_run: bool,
) -> Result<()> {
    use crate::protocol::s3::{has_wildcards, S3Client, S3Config, VersioningOperations};

    let (bucket, key) = parse_s3_uri(uri, "rm", "s3://bucket/key")?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket.clone());
        let client = S3Client::new(config)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        if let Some(ref vid) = version_id {
            if dry_run {
                println!(
                    "(dry-run) Would delete: s3://{}/{} version {}",
                    bucket, key, vid
                );
                return Ok(());
            }
            client
                .delete_object_version(&key, vid)
                .await
                .map_err(|e| OrbitError::Other(format!("Failed to delete version: {}", e)))?;
            print_info(&format!("Deleted s3://{}/{} version {}", bucket, key, vid));
            return Ok(());
        }

        if all_versions {
            let result = client
                .list_object_versions(&key)
                .await
                .map_err(|e| OrbitError::Other(format!("Failed to list versions: {}", e)))?;
            let total = result.versions.len() + result.delete_markers.len();
            if total == 0 {
                print_info("No objects or versions found.");
                return Ok(());
            }
            if dry_run {
                for v in &result.versions {
                    println!(
                        "(dry-run) Would delete: s3://{}/{} version {}",
                        bucket, v.key, v.version_id
                    );
                }
                for dm in &result.delete_markers {
                    println!(
                        "(dry-run) Would delete marker: s3://{}/{} version {}",
                        bucket, dm.key, dm.version_id
                    );
                }
                println!(
                    "\n(dry-run) Would delete {} versions, {} delete markers",
                    result.versions.len(),
                    result.delete_markers.len()
                );
                return Ok(());
            }
            for v in &result.versions {
                client
                    .delete_object_version(&v.key, &v.version_id)
                    .await
                    .map_err(|e| {
                        OrbitError::Other(format!(
                            "Failed to delete version {} of {}: {}",
                            v.version_id, v.key, e
                        ))
                    })?;
            }
            for dm in &result.delete_markers {
                client
                    .delete_object_version(&dm.key, &dm.version_id)
                    .await
                    .map_err(|e| {
                        OrbitError::Other(format!(
                            "Failed to delete marker {} of {}: {}",
                            dm.version_id, dm.key, e
                        ))
                    })?;
            }
            print_info(&format!(
                "Deleted {} versions, {} delete markers",
                result.versions.len(),
                result.delete_markers.len()
            ));
            return Ok(());
        }

        if has_wildcards(&key) {
            let result = client
                .list_objects_with_wildcard(&key)
                .await
                .map_err(|e| OrbitError::Other(format!("Failed to list objects: {}", e)))?;
            if result.objects.is_empty() {
                print_info("No objects matched the pattern.");
                return Ok(());
            }
            let keys: Vec<String> = result.objects.iter().map(|o| o.key.clone()).collect();
            if dry_run {
                for k in &keys {
                    println!("(dry-run) Would delete: s3://{}/{}", bucket, k);
                }
                println!("\n(dry-run) Would delete {} objects", keys.len());
                return Ok(());
            }
            client
                .delete_batch(&keys)
                .await
                .map_err(|e| OrbitError::Other(format!("Failed to batch delete: {}", e)))?;
            print_info(&format!("Deleted {} objects", keys.len()));
        } else {
            if dry_run {
                println!("(dry-run) Would delete: s3://{}/{}", bucket, key);
                return Ok(());
            }
            client
                .delete(&key)
                .await
                .map_err(|e| OrbitError::Other(format!("Failed to delete: {}", e)))?;
            print_info(&format!("Deleted s3://{}/{}", bucket, key));
        }

        Ok(())
    })
}

pub fn handle_mv_command(source: &str, dest: &str) -> Result<()> {
    use crate::protocol::s3::{S3Client, S3Config, S3Operations};

    let (src_bucket, src_key) = parse_s3_uri(source, "mv", "s3://bucket/key")?;
    let (dst_bucket, dst_key) = parse_s3_uri(dest, "mv", "s3://bucket/key")?;

    if src_bucket != dst_bucket {
        return Err(OrbitError::Config(
            "mv command currently only supports moves within the same bucket".to_string(),
        ));
    }

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(src_bucket.clone());
        let client = S3Client::new(config)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        client
            .copy_object(&src_key, &dst_key)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to copy object: {}", e)))?;

        client
            .delete(&src_key)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to delete source after copy: {}", e)))?;

        print_info(&format!(
            "Moved s3://{}/{} -> s3://{}/{}",
            src_bucket, src_key, dst_bucket, dst_key
        ));
        Ok(())
    })
}

pub fn handle_mb_command(bucket_uri: &str) -> Result<()> {
    use crate::protocol::s3::{S3Client, S3Config};

    let bucket_name = parse_bucket_name(bucket_uri, "mb")?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket_name.clone());
        let client = S3Client::new(config)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        client
            .create_bucket(&bucket_name)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to create bucket: {}", e)))?;

        print_info(&format!("Created bucket: s3://{}", bucket_name));
        Ok(())
    })
}

pub fn handle_rb_command(bucket_uri: &str) -> Result<()> {
    use crate::protocol::s3::{S3Client, S3Config};

    let bucket_name = parse_bucket_name(bucket_uri, "rb")?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| OrbitError::Other(format!("Failed to start async runtime: {}", e)))?;

    runtime.block_on(async {
        let config = S3Config::new(bucket_name.clone());
        let client = S3Client::new(config)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to create S3 client: {}", e)))?;

        client
            .delete_bucket(&bucket_name)
            .await
            .map_err(|e| OrbitError::Other(format!("Failed to delete bucket: {}", e)))?;

        print_info(&format!("Deleted bucket: s3://{}", bucket_name));
        Ok(())
    })
}

// ============================================================================
// Helper functions
// ============================================================================

/// Format a SystemTime as a human-readable date string
pub fn format_system_time(time: std::time::SystemTime) -> String {
    let duration = time
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;
    let (year, month, day) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since Unix epoch to (year, month, day)
pub fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn test_parse_s3_uri_accepts_object_key() {
        let (bucket, key) = parse_s3_uri("s3://demo/path/to/file.txt", "cat", "s3://bucket/key")
            .expect("expected valid S3 URI");

        assert_eq!(bucket, "demo");
        assert_eq!(key, "path/to/file.txt");
    }

    #[test]
    fn test_parse_s3_uri_rejects_non_s3_protocol() {
        let err = parse_s3_uri("C:/temp/file.txt", "cat", "s3://bucket/key").unwrap_err();

        assert!(err
            .to_string()
            .contains("cat command requires an S3 URI (s3://bucket/key)"));
    }

    #[test]
    fn test_parse_bucket_name_accepts_s3_uri_and_raw_name() {
        assert_eq!(
            parse_bucket_name("s3://demo-bucket", "mb").unwrap(),
            "demo-bucket"
        );
        assert_eq!(
            parse_bucket_name("demo-bucket", "mb").unwrap(),
            "demo-bucket"
        );
    }

    #[test]
    fn test_parse_bucket_name_rejects_empty_name() {
        let err = parse_bucket_name("", "rb").unwrap_err();
        assert!(err
            .to_string()
            .contains("rb command requires a bucket name (s3://bucket-name)"));
    }

    #[test]
    fn test_s3_handlers_reject_non_s3_uris_before_network() {
        let handlers = [
            handle_cat_command("not-a-uri"),
            handle_presign_command("not-a-uri", 60),
            handle_head_command("not-a-uri", None),
            handle_du_command("not-a-uri", false, false),
            handle_rm_command("not-a-uri", false, None, true),
            handle_mv_command("not-a-uri", "s3://bucket/key"),
        ];

        for result in handlers {
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_days_to_ymd_known_dates() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
        assert_eq!(days_to_ymd(11016), (2000, 2, 29));
    }

    #[test]
    fn test_format_system_time_unix_epoch() {
        let formatted = format_system_time(UNIX_EPOCH + Duration::from_secs(0));
        assert_eq!(formatted, "1970-01-01 00:00:00");
    }
}
