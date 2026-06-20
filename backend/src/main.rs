use axum::{
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod ai;
mod lib;
mod pty;
mod routes;

use lib::{auth::AuthState, security::WorkspaceGuard};
use pty::PtyManager;
use routes::{ai_routes, fs_routes, shell_routes, terminal_routes};

const DEFAULT_PORT: u16 = 3000;
const HOST: &str = "127.0.0.1";

#[derive(Clone)]
pub struct AppState {
    pub auth: Arc<AuthState>,
    pub guard: Arc<WorkspaceGuard>,
    pub pty: Arc<PtyManager>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Set up tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Parse port from env
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    // Initialize state
    let auth_state = Arc::new(AuthState::new());
    let guard = Arc::new(WorkspaceGuard::new());
    let pty = Arc::new(PtyManager::new());

    let state = AppState {
        auth: auth_state.clone(),
        guard,
        pty,
    };

    // API routes
    let api_routes = Router::new()
        .nest("/terminal", terminal_routes())
        .nest("/fs", fs_routes())
        .nest("/shell", shell_routes())
        .nest("/ai", ai_routes())
        .route("/health", get(health_handler))
        .route("/workspace", get(workspace_handler));

    // Auth middleware applied to all API routes
    let api_routes = api_routes.layer(axum::middleware::from_fn_with_state(
        state.clone(),
        lib::auth::auth_middleware,
    ));

    // Main router
    let app = Router::new()
        .nest("/api", api_routes)
        .fallback_service(ServeDir::new("frontend/dist").append_index_html_on_directories(true))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", HOST, port).parse()?;

    // Print banner
    let token = auth_state.token();
    println!("\n");
    println!("╔══════════════════════════════════════════════╗");
    println!("║        PocketDevOS v0.2.0 (Rust)             ║");
    println!("╠══════════════════════════════════════════════╣");
    println!("║  Backend: Axum + Tokio + portable-pty        ║");
    println!("║  Terminal: Native PTY (no script hack)       ║");
    println!("╠══════════════════════════════════════════════╣");
    println!("║  URL:   http://{:>15}:{:<5}              ║", HOST, port);
    println!("║  Token: {}  ║", token);
    println!("╚══════════════════════════════════════════════╝");
    println!("\n  Open the URL in your browser to start coding.\n");

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": "0.2.0",
        "backend": "rust"
    }))
}

async fn workspace_handler() -> axum::Json<serde_json::Value> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    axum::Json(serde_json::json!({ "home": home }))
}
