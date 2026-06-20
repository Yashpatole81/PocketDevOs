use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tracing::{error, warn};

use crate::AppState;

pub fn fs_routes() -> Router<AppState> {
    Router::new()
        .route("/readdir", get(readdir_handler))
        .route("/read", get(read_file_handler))
        .route("/write", post(write_file_handler))
        .route("/stat", get(stat_handler))
        .route("/create", post(create_handler))
        .route("/rename", post(rename_handler))
        .route("/delete", post(delete_handler))
}

#[derive(Deserialize)]
struct PathQuery {
    path: String,
}

#[derive(Serialize)]
struct FsEntry {
    name: String,
    path: String,
    is_directory: bool,
    is_file: bool,
    is_symlink: bool,
}

#[derive(Serialize)]
struct ReaddirResponse {
    items: Vec<FsEntry>,
}

async fn readdir_handler(
    State(state): State<AppState>,
    Query(query): Query<PathQuery>,
) -> Result<Json<ReaddirResponse>, StatusCode> {
    if let Err(e) = state.guard.validate(&query.path) {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut entries = match fs::read_dir(&query.path).await {
        Ok(mut rd) => {
            let mut items = Vec::new();
            while let Ok(Some(entry)) = rd.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                let path = entry.path().to_string_lossy().to_string();
                let ft = entry.file_type().await.ok();

                items.push(FsEntry {
                    name,
                    path,
                    is_directory: ft.map(|f| f.is_dir()).unwrap_or(false),
                    is_file: ft.map(|f| f.is_file()).unwrap_or(false),
                    is_symlink: ft.map(|f| f.is_symlink()).unwrap_or(false),
                });
            }
            items
        }
        Err(e) => {
            error!("readdir error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| {
        match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    Ok(Json(ReaddirResponse { items: entries }))
}

#[derive(Serialize)]
struct ReadFileResponse {
    content: String,
    path: String,
    name: String,
    extension: String,
    size: u64,
}

async fn read_file_handler(
    State(state): State<AppState>,
    Query(query): Query<PathQuery>,
) -> Result<Json<ReadFileResponse>, StatusCode> {
    if let Err(_) = state.guard.validate(&query.path) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check file size
    let metadata = match fs::metadata(&query.path).await {
        Ok(m) => m,
        Err(_) => return Err(StatusCode::NOT_FOUND),
    };

    // Max 5MB
    if metadata.len() > 5 * 1024 * 1024 {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    let content = match fs::read_to_string(&query.path).await {
        Ok(c) => c,
        Err(e) => {
            error!("Read file error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let path = Path::new(&query.path);
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let extension = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    Ok(Json(ReadFileResponse {
        content,
        path: query.path,
        name,
        extension: format!(".{}", extension),
        size: metadata.len(),
    }))
}

#[derive(Deserialize)]
struct WriteFileRequest {
    path: String,
    content: String,
}

async fn write_file_handler(
    State(state): State<AppState>,
    Json(body): Json<WriteFileRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Err(_) = state.guard.validate(&body.path) {
        return Err(StatusCode::FORBIDDEN);
    }

    match fs::write(&body.path, body.content).await {
        Ok(_) => Ok(Json(serde_json::json!({"ok": true, "path": body.path}))),
        Err(e) => {
            error!("Write file error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Serialize)]
struct StatResponse {
    path: String,
    name: String,
    size: u64,
    is_directory: bool,
    is_file: bool,
    modified: String,
    created: String,
}

async fn stat_handler(
    State(state): State<AppState>,
    Query(query): Query<PathQuery>,
) -> Result<Json<StatResponse>, StatusCode> {
    if let Err(_) = state.guard.validate(&query.path) {
        return Err(StatusCode::FORBIDDEN);
    }

    let metadata = match fs::metadata(&query.path).await {
        Ok(m) => m,
        Err(_) => return Err(StatusCode::NOT_FOUND),
    };

    let path = Path::new(&query.path);
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
        .flatten()
        .map(|d| d.to_rfc3339())
        .unwrap_or_default();

    let created = metadata
        .created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
        .flatten()
        .map(|d| d.to_rfc3339())
        .unwrap_or_default();

    Ok(Json(StatResponse {
        path: query.path.clone(),
        name,
        size: metadata.len(),
        is_directory: metadata.is_dir(),
        is_file: metadata.is_file(),
        modified,
        created,
    }))
}

#[derive(Deserialize)]
struct CreateRequest {
    path: String,
    #[serde(rename = "type")]
    file_type: String,
    content: Option<String>,
}

async fn create_handler(
    State(state): State<AppState>,
    Json(body): Json<CreateRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Err(_) = state.guard.validate(&body.path) {
        return Err(StatusCode::FORBIDDEN);
    }

    let result = if body.file_type == "directory" {
        fs::create_dir_all(&body.path).await
    } else {
        fs::write(&body.path, body.content.unwrap_or_default()).await
    };

    match result {
        Ok(_) => Ok(Json(serde_json::json!({"ok": true, "path": body.path}))),
        Err(e) => {
            error!("Create error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Deserialize)]
struct RenameRequest {
    from: String,
    to: String,
}

async fn rename_handler(
    State(state): State<AppState>,
    Json(body): Json<RenameRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Err(_) = state.guard.validate(&body.from) {
        return Err(StatusCode::FORBIDDEN);
    }
    if let Err(_) = state.guard.validate(&body.to) {
        return Err(StatusCode::FORBIDDEN);
    }

    match fs::rename(&body.from, &body.to).await {
        Ok(_) => Ok(Json(
            serde_json::json!({"ok": true, "from": body.from, "to": body.to}),
        )),
        Err(e) => {
            error!("Rename error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Deserialize)]
struct DeleteRequest {
    path: String,
}

async fn delete_handler(
    State(state): State<AppState>,
    Json(body): Json<DeleteRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Err(_) = state.guard.validate(&body.path) {
        return Err(StatusCode::FORBIDDEN);
    }

    match fs::remove_dir_all(&body.path).await {
        Ok(_) => Ok(Json(serde_json::json!({"ok": true, "path": body.path}))),
        Err(_) => {
            // Try as file
            match fs::remove_file(&body.path).await {
                Ok(_) => Ok(Json(serde_json::json!({"ok": true, "path": body.path}))),
                Err(e) => {
                    error!("Delete error: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
    }
}
