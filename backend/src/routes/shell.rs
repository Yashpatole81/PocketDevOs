use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::error;

use crate::AppState;

const MAX_OUTPUT_BYTES: usize = 256 * 1024;
const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const MAX_TIMEOUT_MS: u64 = 300_000;

pub fn shell_routes() -> Router<AppState> {
    Router::new().route("/run", post(run_command))
}

#[derive(Deserialize)]
struct RunRequest {
    command: String,
    cwd: Option<String>,
    timeout: Option<u64>,
}

#[derive(Serialize)]
struct RunResponse {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    timed_out: bool,
    truncated: bool,
}

async fn run_command(
    State(_state): State<AppState>,
    Json(body): Json<RunRequest>,
) -> Result<Json<RunResponse>, StatusCode> {
    let command = body.command.trim();
    if command.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let timeout_ms = body
        .timeout
        .unwrap_or(DEFAULT_TIMEOUT_MS)
        .min(MAX_TIMEOUT_MS);

    let cwd = body
        .cwd
        .unwrap_or_else(|| std::env::var("HOME").unwrap_or_else(|_| "/".to_string()));

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

    let result = timeout(
        Duration::from_millis(timeout_ms),
        Command::new(&shell)
            .arg("-c")
            .arg(command)
            .current_dir(&cwd)
            .env("TERM", "dumb")
            .env("POCKETDEVOS", "1")
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let truncated = stdout.len() > MAX_OUTPUT_BYTES || stderr.len() > MAX_OUTPUT_BYTES;

            Ok(Json(RunResponse {
                stdout: stdout.chars().take(MAX_OUTPUT_BYTES).collect(),
                stderr: stderr.chars().take(MAX_OUTPUT_BYTES).collect(),
                exit_code: output.status.code(),
                timed_out: false,
                truncated,
            }))
        }
        Ok(Err(e)) => {
            error!("Command execution error: {}", e);
            Ok(Json(RunResponse {
                stdout: String::new(),
                stderr: e.to_string(),
                exit_code: Some(1),
                timed_out: false,
                truncated: false,
            }))
        }
        Err(_) => {
            // Timeout
            Ok(Json(RunResponse {
                stdout: String::new(),
                stderr: format!("Command timed out after {}ms", timeout_ms),
                exit_code: None,
                timed_out: true,
                truncated: false,
            }))
        }
    }
}
