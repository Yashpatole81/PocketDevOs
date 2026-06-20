use axum::{
    body::Body,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::{
    ai::{client::AiClient, tools::AiTools},
    AppState,
};

// Active sessions stored as lazy static
use std::sync::OnceLock;
static ACTIVE_SESSIONS: OnceLock<Arc<Mutex<HashMap<String, tokio::sync::mpsc::Sender<()>>>>> =
    OnceLock::new();

fn active_sessions() -> Arc<Mutex<HashMap<String, tokio::sync::mpsc::Sender<()>>>> {
    ACTIVE_SESSIONS
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

pub fn ai_routes() -> Router<AppState> {
    Router::new()
        .route("/config", get(get_config).post(set_config))
        .route("/chat", post(chat_handler))
        .route("/approve", post(approve_handler))
        .route("/reject", post(reject_handler))
        .route("/stop", post(stop_handler))
}

// -- AI Config --

#[derive(Serialize)]
struct ConfigResponse {
    providers: Vec<ProviderInfo>,
    models: Vec<ModelInfo>,
    current: CurrentConfig,
}

#[derive(Serialize)]
struct ProviderInfo {
    id: String,
    name: String,
    base_url: String,
    requires_key: bool,
    description: String,
}

#[derive(Serialize)]
struct ModelInfo {
    id: String,
    provider: String,
    label: String,
    description: String,
}

#[derive(Serialize)]
struct CurrentConfig {
    provider: String,
    model: String,
    base_url: String,
    has_key: bool,
}

async fn get_config(State(_state): State<AppState>) -> Json<ConfigResponse> {
    Json(ConfigResponse {
        providers: vec![
            ProviderInfo {
                id: "nvidia".to_string(),
                name: "NVIDIA Build".to_string(),
                base_url: "https://integrate.api.nvidia.com/v1".to_string(),
                requires_key: true,
                description: "NVIDIA Build API".to_string(),
            },
            ProviderInfo {
                id: "ollama".to_string(),
                name: "Ollama (Local)".to_string(),
                base_url: "http://localhost:11434/v1".to_string(),
                requires_key: false,
                description: "Local models via Ollama".to_string(),
            },
            ProviderInfo {
                id: "custom".to_string(),
                name: "Custom Endpoint".to_string(),
                base_url: String::new(),
                requires_key: false,
                description: "Any OpenAI-compatible endpoint".to_string(),
            },
        ],
        models: vec![
            ModelInfo {
                id: "nvidia/llama-3.3-nemotron-super-49b-v1".to_string(),
                provider: "nvidia".to_string(),
                label: "Llama 3.3 Nemotron Super 49B".to_string(),
                description: "NVIDIA's top-tier reasoning model".to_string(),
            },
            ModelInfo {
                id: "google/gemma-3-27b-it".to_string(),
                provider: "nvidia".to_string(),
                label: "Gemma 3 27B IT".to_string(),
                description: "Google Gemma 3 via NVIDIA".to_string(),
            },
            ModelInfo {
                id: "meta/llama-3.1-70b-instruct".to_string(),
                provider: "nvidia".to_string(),
                label: "Llama 3.1 70B Instruct".to_string(),
                description: "Meta Llama 3.1 via NVIDIA".to_string(),
            },
            ModelInfo {
                id: "gemma4:12b".to_string(),
                provider: "ollama".to_string(),
                label: "Gemma 4 12B".to_string(),
                description: "Google Gemma 4 local".to_string(),
            },
            ModelInfo {
                id: "qwen2.5-coder:7b".to_string(),
                provider: "ollama".to_string(),
                label: "Qwen 2.5 Coder 7B".to_string(),
                description: "Coding-focused local model".to_string(),
            },
        ],
        current: CurrentConfig {
            provider: "ollama".to_string(),
            model: "qwen2.5-coder:7b".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            has_key: false,
        },
    })
}

#[derive(Deserialize)]
struct SetConfigRequest {
    #[allow(dead_code)]
    provider: Option<String>,
    #[allow(dead_code)]
    model: Option<String>,
    #[allow(dead_code)]
    base_url: Option<String>,
    #[allow(dead_code)]
    api_key: Option<String>,
}

async fn set_config(
    State(_state): State<AppState>,
    Json(_body): Json<SetConfigRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": true}))
}

// -- AI Chat with SSE Streaming --

#[derive(Deserialize)]
struct ChatRequest {
    messages: Vec<ChatMessage>,
    session_id: String,
}

#[derive(Deserialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct AgentEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    args: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

async fn chat_handler(
    State(app_state): State<AppState>,
    Json(body): Json<ChatRequest>,
) -> Result<Response, StatusCode> {
    let session_id = body.session_id.clone();
    let active = active_sessions();

    // Abort any existing session with same ID
    {
        let mut sessions = active.lock().await;
        if let Some(tx) = sessions.remove(&session_id) {
            let _ = tx.send(()).await;
        }
    }

    // Create abort channel
    let (abort_tx, mut abort_rx) = tokio::sync::mpsc::channel::<()>(1);
    {
        let mut sessions = active.lock().await;
        sessions.insert(session_id.clone(), abort_tx);
    }

    let messages = body.messages;
    let guard = app_state.guard.clone();

    // Build SSE stream
    let stream = async_stream::stream! {
        let client = AiClient::default();
        let tools = AiTools::new(guard);

        let system_prompt = "You are PocketDevOS, an AI coding assistant running inside a terminal on an Android device. You have access to the filesystem and can run shell commands.

Rules:
- Execute, don't echo. When asked to create/edit a file, use the tool directly.
- Chain actions: read → understand → change → verify.
- Be concise. No filler.
- Bare filenames resolve to the current working directory.
- For write_file and run_command, the user must approve before execution.";

        let mut openai_messages: Vec<serde_json::Value> = vec![
            serde_json::json!({"role": "system", "content": system_prompt})
        ];

        for msg in &messages {
            openai_messages.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }));
        }

        let mut stream = match client.stream_chat(openai_messages, &tools).await {
            Ok(s) => s,
            Err(e) => {
                yield Ok::<_, std::convert::Infallible>(
                    format!("data: {}\n\n", serde_json::json!({"type": "error", "message": e.to_string()}))
                );
                return;
            }
        };

        use futures_util::StreamExt;

        while let Some(chunk_result) = stream.next().await {
            if abort_rx.try_recv().is_ok() {
                yield Ok::<_, std::convert::Infallible>(
                    format!("data: {}\n\n", serde_json::json!({"type": "done"}))
                );
                return;
            }

            match chunk_result {
                Ok(chunk) => {
                    let event = AgentEvent {
                        event_type: "text".to_string(),
                        content: Some(chunk),
                        id: None,
                        name: None,
                        args: None,
                        result: None,
                        tool: None,
                        message: None,
                    };
                    let data = format!("data: {}\n\n", serde_json::to_string(&event).unwrap_or_default());
                    yield Ok::<_, std::convert::Infallible>(data);
                }
                Err(e) => {
                    let event = AgentEvent {
                        event_type: "error".to_string(),
                        content: None,
                        id: None,
                        name: None,
                        args: None,
                        result: None,
                        tool: None,
                        message: Some(e.to_string()),
                    };
                    let data = format!("data: {}\n\n", serde_json::to_string(&event).unwrap_or_default());
                    yield Ok::<_, std::convert::Infallible>(data);
                    return;
                }
            }
        }

        let done_event = AgentEvent {
            event_type: "done".to_string(),
            content: None,
            id: None,
            name: None,
            args: None,
            result: None,
            tool: None,
            message: None,
        };
        yield Ok::<_, std::convert::Infallible>(
            format!("data: {}\n\n", serde_json::to_string(&done_event).unwrap_or_default())
        );

        let mut sessions = active.lock().await;
        sessions.remove(&session_id);
    };

    let body = Body::from_stream(stream);
    let response = Response::builder()
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("X-Accel-Buffering", "no")
        .body(body)
        .unwrap();

    Ok(response)
}

#[derive(Deserialize)]
struct SessionRequest {
    session_id: String,
}

async fn approve_handler(
    State(_app): State<AppState>,
    Json(_body): Json<SessionRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": true}))
}

async fn reject_handler(
    State(_app): State<AppState>,
    Json(_body): Json<SessionRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": true}))
}

async fn stop_handler(
    State(_app): State<AppState>,
    Json(body): Json<SessionRequest>,
) -> Json<serde_json::Value> {
    let active = active_sessions();
    let mut sessions = active.lock().await;
    let stopped = if let Some(tx) = sessions.remove(&body.session_id) {
        let _ = tx.send(()).await;
        true
    } else {
        false
    };

    Json(serde_json::json!({"ok": true, "stopped": stopped}))
}
