//! File system API endpoints

use crate::error::{WebError, WebResult};
use axum::{extract::Query, Json};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

/// File/directory entry for API response
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: i64,
}

/// Query parameters for listing files
#[derive(Debug, Deserialize)]
pub struct ListFilesQuery {
    pub path: Option<String>,
}

/// List files handler (GET with query params)
#[cfg_attr(feature = "utoipa", utoipa::path(
    get,
    path = "/api/files/list",
    params(
        ("path" = Option<String>, Query, description = "Directory path to list")
    ),
    responses(
        (status = 200, description = "List of files and directories", body = Vec<FileEntry>),
        (status = 404, description = "Path not found"),
        (status = 400, description = "Path is not a directory")
    )
))]
pub async fn list_files(
    Query(query): Query<ListFilesQuery>,
) -> WebResult<Json<Vec<FileEntry>>> {
    let path = query.path.unwrap_or_else(|| {
        #[cfg(windows)]
        {
            "C:\\".to_string()
        }
        #[cfg(not(windows))]
        {
            "/".to_string()
        }
    });

    tracing::info!("Listing files: {}", path);

    let dir_path = Path::new(&path);
    if !dir_path.exists() {
        return Err(WebError::NotFound(format!("Path not found: {}", path)));
    }

    if !dir_path.is_dir() {
        return Err(WebError::BadRequest(format!(
            "Path is not a directory: {}",
            path
        )));
    }

    let mut entries = Vec::new();

    // Add parent directory entry if not at root
    if let Some(parent) = dir_path.parent() {
        entries.push(FileEntry {
            name: "..".to_string(),
            path: parent.to_string_lossy().to_string(),
            is_dir: true,
            size: 0,
            modified: 0,
        });
    }

    match fs::read_dir(dir_path) {
        Ok(read_dir) => {
            for entry in read_dir.flatten() {
                let metadata = entry.metadata().ok();
                let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = metadata
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                entries.push(FileEntry {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry.path().to_string_lossy().to_string(),
                    is_dir,
                    size,
                    modified,
                });
            }
        }
        Err(e) => {
            return Err(WebError::Forbidden(format!(
                "Cannot read directory: {}",
                e
            )));
        }
    }

    // Sort: directories first, then by name
    entries.sort_by(|a, b| {
        if a.name == ".." {
            std::cmp::Ordering::Less
        } else if b.name == ".." {
            std::cmp::Ordering::Greater
        } else if a.is_dir && !b.is_dir {
            std::cmp::Ordering::Less
        } else if !a.is_dir && b.is_dir {
            std::cmp::Ordering::Greater
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    Ok(Json(entries))
}

/// Get system drives/roots
#[cfg(windows)]
pub async fn list_drives() -> Json<Vec<FileEntry>> {
    let mut drives = Vec::new();
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        let path = Path::new(&drive);
        if path.exists() {
            drives.push(FileEntry {
                name: drive.clone(),
                path: drive,
                is_dir: true,
                size: 0,
                modified: 0,
            });
        }
    }
    Json(drives)
}

#[cfg(not(windows))]
pub async fn list_drives() -> Json<Vec<FileEntry>> {
    Json(vec![FileEntry {
        name: "/".to_string(),
        path: "/".to_string(),
        is_dir: true,
        size: 0,
        modified: 0,
    }])
}
