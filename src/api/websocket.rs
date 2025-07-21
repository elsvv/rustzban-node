use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
};
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};
use tokio::time::sleep;
use uuid::Uuid;

use crate::api::handlers::AppState;

/// Параметры WebSocket для логов (аналог query_params в Python)
#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub session_id: String,
    pub interval: Option<f64>,
}

/// WebSocket handler для логов (аналог logs в Python rest_service.py)
pub async fn logs_websocket_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    // Парсим параметры как в Python
    let session_id_str = params.get("session_id");
    let interval_str = params.get("interval");
    
    // Проверяем session_id
    let session_id = match session_id_str {
        Some(id_str) => match Uuid::parse_str(id_str) {
            Ok(id) => id,
            Err(_) => {
                tracing::warn!("WebSocket upgrade failed: session_id should be a valid UUID");
                // Возвращаем простую заглушку
                return ws.on_upgrade(|_socket| async {
                    // WebSocket закроется автоматически
                });
            }
        },
        None => {
            tracing::warn!("WebSocket upgrade failed: session_id parameter missing");
            return ws.on_upgrade(|_socket| async {
                // WebSocket закроется автоматически
            });
        }
    };
    
    // Проверяем interval
    let interval = if let Some(interval_str) = interval_str {
        match interval_str.parse::<f64>() {
            Ok(val) => {
                if val > 10.0 {
                    tracing::warn!("WebSocket upgrade failed: Interval must be more than 0 and at most 10 seconds");
                    return ws.on_upgrade(|_socket| async {
                        // WebSocket закроется автоматически
                    });
                }
                Some(val)
            },
            Err(_) => {
                tracing::warn!("WebSocket upgrade failed: Invalid interval value");
                return ws.on_upgrade(|_socket| async {
                    // WebSocket закроется автоматически
                });
            }
        }
    } else {
        None
    };
    
    ws.on_upgrade(move |socket| logs_websocket(socket, state, session_id, interval))
}

/// Основная логика WebSocket для логов (упрощенная версия)
async fn logs_websocket(
    mut socket: WebSocket,
    state: AppState,
    session_id: Uuid,
    _interval: Option<f64>, // TODO: Реализовать interval логику позже
) {
    // Проверяем session_id как в Python
    let current_session_id = state.session_manager.get_session_id().await;
    if current_session_id != Some(session_id) {
        tracing::warn!("WebSocket connection rejected: Session ID mismatch");
        return;
    }
    
    // Получаем буфер логов
    let logs_buffer = state.session_manager.get_logs_buffer();
    
    // Упрощенная логика - отправляем логи по мере поступления
    loop {
        // Проверяем что session_id все еще актуален
        let current_session_id = state.session_manager.get_session_id().await;
        if current_session_id != Some(session_id) {
            break;
        }
        
        // Получаем новые логи
        let new_logs = {
            let buffer = logs_buffer.lock().unwrap();
            buffer.get_all()
        };
        
        // Отправляем логи
        for log in new_logs {
            if socket.send(Message::Text(log.into())).await.is_err() {
                return;
            }
        }
        
        // Ждем немного перед следующей проверкой
        sleep(Duration::from_millis(200)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_logs_query_parsing() {
        let query = LogsQuery {
            session_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            interval: Some(5.0),
        };
        
        assert_eq!(query.session_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(query.interval, Some(5.0));
    }
    
    #[test]
    fn test_uuid_parsing() {
        let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
        let parsed = Uuid::parse_str(valid_uuid);
        assert!(parsed.is_ok());
        
        let invalid_uuid = "invalid-uuid";
        let parsed = Uuid::parse_str(invalid_uuid);
        assert!(parsed.is_err());
    }
} 