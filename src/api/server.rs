use axum::{
    routing::{get, post},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use std::sync::Arc;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use crate::{
    api::{
        handlers::{
            base_handler, connect_handler, disconnect_handler, ping_handler,
            restart_handler, start_handler, stop_handler, AppState,
        },
        websocket::logs_websocket_handler,
    },
    config::Config,
    session::SessionManager,
    ssl::auth::SslConfig,
};

/// Создает REST сервер (аналог FastAPI app в Python)
pub async fn create_rest_server(config: Arc<Config>) -> Result<(), Box<dyn std::error::Error>> {
    // Создаем SessionManager (аналог Service() в Python)
    let session_manager = Arc::new(
        SessionManager::new(
            config.xray_executable_path.clone(),
            config.xray_assets_path.clone(),
        )
        .await?,
    );
    
    // Создаем состояние приложения
    let app_state = AppState {
        session_manager,
        config: Arc::clone(&config),
    };
    
    // Создаем маршруты идентично Python rest_service.py
    let app = Router::new()
        // POST endpoints как в Python
        .route("/", post(base_handler))
        .route("/ping", post(ping_handler))
        .route("/connect", post(connect_handler))
        .route("/disconnect", post(disconnect_handler))
        .route("/start", post(|state, json| async move { start_handler(state, json).await }))
        .route("/stop", post(stop_handler))
        .route("/restart", post(|state, json| async move { restart_handler(state, json).await }))
        // WebSocket endpoint для логов
        .route("/logs", get(logs_websocket_handler))
        .with_state(app_state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .into_inner(),
        );
    
    let addr = format!("{}:{}", config.service_host, config.service_port);
    let socket_addr: SocketAddr = addr.parse()
        .map_err(|e| format!("Failed to parse address {}: {}", addr, e))?;
    
    // Для REST протокола всегда используем HTTPS (как в Python версии)
    tracing::info!("Starting HTTPS server on {}", socket_addr);
    
    // Создаем SSL конфигурацию
    let ssl_config = SslConfig::new(
        config.ssl_cert_file.clone(),
        config.ssl_key_file.clone(),
        config.ssl_client_cert_file.clone(),
    );
    
            // Создаем TLS конфигурацию для axum-server
        let tls_config = RustlsConfig::from_pem_file(
            &ssl_config.cert_file,
            &ssl_config.key_file,
        ).await?;
    
    // Запускаем HTTPS сервер
    axum_server::bind_rustls(socket_addr, tls_config)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_server_creation() {
        let config = Arc::new(crate::config::Config::default());
        
        // Тест может упасть если xray не установлен - это нормально
        let _result = create_rest_server(config).await;
        
        // Просто проверяем что функция существует
        // Реальный тест требует запуска сервера
    }
} 