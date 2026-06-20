use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::Rng;

use crate::AppState;

/// Auth token length in bytes
const TOKEN_BYTES: usize = 16;

/// State holding the generated auth token
pub struct AuthState {
    token: String,
}

impl AuthState {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..TOKEN_BYTES).map(|_| rng.gen()).collect();
        let token = STANDARD.encode(&bytes);
        // Make URL-safe
        let token = token.replace(['+', '/', '='], "_");
        Self { token }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn validate(&self, token: &str) -> bool {
        self.token == token
    }
}

/// Extract token from Authorization: Bearer <token> header or ?token= query param
fn extract_token(request: &Request) -> Option<String> {
    // Try header first
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                return Some(auth_str[7..].to_string());
            }
        }
    }

    // Try query param
    let uri = request.uri().query()?;
    for pair in uri.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            if key == "token" {
                return Some(value.to_string());
            }
        }
    }

    None
}

/// Auth middleware: validate token on all API routes except /api/health
pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Allow health check without auth
    let path = request.uri().path();
    if path == "/api/health" {
        let response = next.run(request).await;
        return Ok(response);
    }

    let token = extract_token(&request).ok_or(StatusCode::UNAUTHORIZED)?;

    if !state.auth.validate(&token) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let response = next.run(request).await;
    Ok(response)
}
