use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use uuid::Uuid;

use crate::{
    config::Config,
    session::{SessionError, SessionManager, SessionResponse},
};

/// Состояние приложения для handlers
#[derive(Clone)]
pub struct AppState {
    pub session_manager: Arc<SessionManager>,
    pub config: Arc<Config>,
}

/// Base API response (analog of base in Python)
pub async fn base_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let response = state.session_manager.create_response().await;
    Json(response)
}

/// Ping endpoint (analog of ping in Python)
#[derive(Debug, Deserialize)]
pub struct PingRequest {
    pub session_id: Uuid,
}

pub async fn ping_handler(
    State(state): State<AppState>,
    Json(request): Json<PingRequest>,
) -> impl IntoResponse {
    match state.session_manager.ping(request.session_id).await {
        Ok(()) => Json(serde_json::json!({})).into_response(),
        Err(e) => ApiError::Session(e).into_response(),
    }
}

/// Connect endpoint (analog of connect in Python)
pub async fn connect_handler(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let client_ip = addr.ip();
    match state.session_manager.connect(client_ip).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => ApiError::Session(e).into_response(),
    }
}

/// Disconnect endpoint (analog of disconnect in Python)
pub async fn disconnect_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.session_manager.disconnect().await {
        Ok(response) => Json(response).into_response(),
        Err(e) => ApiError::Session(e).into_response(),
    }
}

/// Start endpoint (analog of start in Python)
#[derive(Debug, Deserialize)]
pub struct StartRequest {
    pub session_id: Uuid,
    pub config: String,
}

pub async fn start_handler(
    State(state): State<AppState>,
    Json(request): Json<StartRequest>,
) -> Response {
    match state.session_manager
        .start(request.session_id, request.config, &state.config)
        .await
    {
        Ok(response) => Json(response).into_response(),
        Err(e) => ApiError::Session(e).into_response(),
    }
}

/// Stop endpoint (analog of stop in Python)
#[derive(Debug, Deserialize)]
pub struct StopRequest {
    pub session_id: Uuid,
}

pub async fn stop_handler(
    State(state): State<AppState>,
    Json(request): Json<StopRequest>,
) -> impl IntoResponse {
    match state.session_manager.stop(request.session_id).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => ApiError::Session(e).into_response(),
    }
}

/// Restart endpoint (analog of restart in Python)
#[derive(Debug, Deserialize)]
pub struct RestartRequest {
    pub session_id: Uuid,
    pub config: String,
}

pub async fn restart_handler(
    State(state): State<AppState>,
    Json(request): Json<RestartRequest>,
) -> Response {
    match state.session_manager
        .restart(request.session_id, request.config, &state.config)
        .await
    {
        Ok(response) => Json(response).into_response(),
        Err(e) => ApiError::Session(e).into_response(),
    }
}

/// Обработчик ошибок валидации (аналог validation_exception_handler в Python)
#[derive(Debug, Serialize)]
pub struct ValidationErrorResponse {
    pub detail: serde_json::Map<String, serde_json::Value>,
}

/// API ошибки (аналог HTTPException в Python)
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Session error: {0}")]
    Session(#[from] SessionError),
    
    #[error("Validation error")]
    Validation(String),
    
    #[error("Internal server error: {0}")]
    Internal(String),
}

impl From<ApiError> for (StatusCode, Json<serde_json::Value>) {
    fn from(error: ApiError) -> Self {
        match error {
            ApiError::Session(SessionError::SessionMismatch) => (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "detail": "Session ID mismatch."
                })),
            ),
            ApiError::Session(SessionError::ConfigError(msg)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "detail": {
                        "config": format!("Failed to decode config: {}", msg)
                    }
                })),
            ),
            ApiError::Session(SessionError::StartupFailed(msg)) |
            ApiError::Session(SessionError::CoreError(msg)) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "detail": msg
                })),
            ),
            ApiError::Session(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "detail": err.to_string()
                })),
            ),
            ApiError::Validation(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "detail": msg
                })),
            ),
            ApiError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "detail": msg
                })),
            ),
        }
    }
}

// Реализуем IntoResponse для ApiError
impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, json) = self.into();
        (status, json).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_api_error_conversion() {
        let error = ApiError::Session(SessionError::SessionMismatch);
        let (status, _) = error.into();
        assert_eq!(status, StatusCode::FORBIDDEN);
    }
    
    #[test]
    fn test_validation_error() {
        let error = ApiError::Validation("Invalid input".to_string());
        let (status, json) = error.into();
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
} 