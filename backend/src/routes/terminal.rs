use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tracing::{error, info};

use crate::AppState;

pub fn terminal_routes() -> Router<AppState> {
    Router::new()
        .route("/create", post(create_session))
        .route("/list", get(list_sessions))
        .route("/:id", delete(kill_session))
        .route("/:id/resize", post(resize_session))
        .route("/:id/ws", get(ws_handler))
}

#[derive(Deserialize)]
struct CreateRequest {
    cols: Option<u16>,
    rows: Option<u16>,
    cwd: Option<String>,
}

async fn create_session(
    State(state): State<AppState>,
    Json(body): Json<CreateRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.pty.create(body.cols, body.rows, body.cwd).await {
        Ok(info) => Ok(Json(serde_json::json!({
            "id": info.id,
            "cols": info.cols,
            "rows": info.rows,
        }))),
        Err(e) => {
            error!("Failed to create PTY session: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn list_sessions(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let sessions = state.pty.list().await;
    Json(serde_json::json!({ "sessions": sessions }))
}

async fn kill_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state
        .pty
        .kill(&id)
        .await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::NOT_FOUND)
}

#[derive(Deserialize)]
struct ResizeRequest {
    cols: u16,
    rows: u16,
}

async fn resize_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ResizeRequest>,
) -> Json<serde_json::Value> {
    // Send resize via ANSI escape sequence through PTY
    let seq = format!("\x1b[8;{};{}t", body.rows, body.cols);
    let _ = state.pty.write(&id, &seq).await;
    Json(serde_json::json!({"ok": true}))
}

// WebSocket handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_terminal_socket(socket, id, state))
}

async fn handle_terminal_socket(socket: WebSocket, id: String, state: AppState) {
    info!("WebSocket connected for session {}", id);

    // Verify session exists
    if state.pty.get_info(&id).await.is_none() {
        let _ = socket.close().await;
        return;
    }

    // Subscribe to stdout
    let mut stdout_rx = match state.pty.get_stdout_tx(&id).await {
        Ok(tx) => tx.subscribe(),
        Err(_) => {
            let _ = socket.close().await;
            return;
        }
    };

    // Subscribe to exit
    let mut exit_rx = match state.pty.get_exit_tx(&id).await {
        Ok(tx) => tx.subscribe(),
        Err(_) => {
            let _ = socket.close().await;
            return;
        }
    };

    // Split socket for concurrent read/write
    let (mut sender, mut receiver) = socket.split();

    // Clone pty Arc for the stdin task
    let pty_for_stdin = state.pty.clone();

    // Task: PTY stdout -> WebSocket
    let id_stdout = id.clone();
    let mut stdout_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(data) = stdout_rx.recv() => {
                    if sender.send(Message::Text(data)).await.is_err() {
                        break;
                    }
                }
                Ok(code) = exit_rx.recv() => {
                    let msg = format!("\r\n[Process exited with code {}]\r\n", code);
                    let _ = sender.send(Message::Text(msg)).await;
                    let _ = sender.close().await;
                    break;
                }
                else => break,
            }
        }
        info!("[{}] stdout forwarding ended", id_stdout);
    });

    // Task: WebSocket -> PTY stdin
    let id_stdin = id.clone();
    let mut stdin_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Check for resize message
                    if text.starts_with('{') {
                        if let Ok(resize) = serde_json::from_str::<ResizeMsg>(&text) {
                            if resize.r#type == "resize" {
                                let seq = format!("\x1b[8;{};{}t", resize.rows, resize.cols);
                                let _ = pty_for_stdin.write(&id_stdin, &seq).await;
                                continue;
                            }
                        }
                    }
                    // Regular terminal input
                    if pty_for_stdin.write(&id_stdin, &text).await.is_err() {
                        break;
                    }
                }
                Message::Close(_) => break,
                Message::Binary(data) => {
                    if let Ok(text) = String::from_utf8(data) {
                        if pty_for_stdin.write(&id_stdin, &text).await.is_err() {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        info!("[{}] stdin forwarding ended", id_stdin);
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut stdout_task => { stdin_task.abort(); }
        _ = &mut stdin_task => { stdout_task.abort(); }
    }

    info!("WebSocket disconnected for session {}", id);
}

#[derive(serde::Deserialize)]
struct ResizeMsg {
    #[serde(rename = "type")]
    r#type: String,
    cols: u16,
    rows: u16,
}
