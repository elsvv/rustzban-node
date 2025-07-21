use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use std::net::IpAddr;
use crate::xray::{XrayCore, XrayConfig};

/// Session manager (analog of Service class from Python rest_service.py)
/// Manages connection state, session_id and Xray core
#[derive(Debug)]
pub struct SessionManager {
    /// Connection flag (analog of self.connected in Python)
    connected: Arc<RwLock<bool>>,
    
    /// Client IP address (analog of self.client_ip in Python)
    client_ip: Arc<RwLock<Option<IpAddr>>>,
    
    /// Session ID (analog of self.session_id in Python)
    session_id: Arc<RwLock<Option<Uuid>>>,
    
    /// Xray Core (analog of self.core in Python)
    xray_core: Arc<XrayCore>,
    
    /// Xray version (analog of self.core_version in Python)
    core_version: Option<String>,
}

impl SessionManager {
    /// Create new session manager (analog of __init__ in Python)
    pub async fn new(executable_path: String, assets_path: String) -> Result<Self, Box<dyn std::error::Error>> {
        let xray_core = Arc::new(XrayCore::new(executable_path, assets_path).await?);
        let core_version = xray_core.version.clone();
        
        Ok(Self {
            connected: Arc::new(RwLock::new(false)),
            client_ip: Arc::new(RwLock::new(None)),
            session_id: Arc::new(RwLock::new(None)),
            xray_core,
            core_version,
        })
    }
    
    /// Check session_id match (analog of match_session_id in Python)
    pub async fn match_session_id(&self, session_id: Uuid) -> Result<(), SessionError> {
        let current_session = self.session_id.read().await;
        match *current_session {
            Some(current) if current == session_id => Ok(()),
            _ => Err(SessionError::SessionMismatch),
        }
    }
    
    /// Create standard response (analog of response in Python)
    pub async fn create_response(&self) -> SessionResponse {
        let connected = *self.connected.read().await;
        let started = self.xray_core.started().await;
        
        SessionResponse {
            connected,
            started,
            core_version: self.core_version.clone(),
            session_id: None, // Will be set in specific methods if needed
        }
    }
    
    /// Client connection (analog of connect in Python)
    pub async fn connect(&self, client_ip: IpAddr) -> Result<SessionResponse, SessionError> {
        let new_session_id = Uuid::new_v4();
        
        // Check if there's already a connection
        let was_connected = *self.connected.read().await;
        if was_connected {
            tracing::warn!(
                "New connection from {}, Core control access was taken away from previous client.",
                client_ip
            );
            
            // Stop core if it's running
            if self.xray_core.started().await {
                let _ = self.xray_core.stop().await;
            }
        }
        
        // Set new connection
        {
            let mut connected = self.connected.write().await;
            *connected = true;
        }
        
        {
            let mut client_ip_lock = self.client_ip.write().await;
            *client_ip_lock = Some(client_ip);
        }
        
        {
            let mut session_id_lock = self.session_id.write().await;
            *session_id_lock = Some(new_session_id);
        }
        
        tracing::info!("{} connected, Session ID = \"{}\".", client_ip, new_session_id);
        
        let mut response = self.create_response().await;
        response.session_id = Some(new_session_id);
        Ok(response)
    }
    
    /// Client disconnection (analog of disconnect in Python)
    pub async fn disconnect(&self) -> Result<SessionResponse, SessionError> {
        let client_ip = {
            let client_ip_lock = self.client_ip.read().await;
            *client_ip_lock
        };
        
        let session_id = {
            let session_id_lock = self.session_id.read().await;
            *session_id_lock
        };
        
        if *self.connected.read().await {
            if let (Some(ip), Some(id)) = (client_ip, session_id) {
                tracing::info!("{} disconnected, Session ID = \"{}\".", ip, id);
            }
        }
        
        // Reset state
        {
            let mut session_id_lock = self.session_id.write().await;
            *session_id_lock = None;
        }
        
        {
            let mut client_ip_lock = self.client_ip.write().await;
            *client_ip_lock = None;
        }
        
        {
            let mut connected = self.connected.write().await;
            *connected = false;
        }
        
        // Stop core if running
        if self.xray_core.started().await {
            let _ = self.xray_core.stop().await;
        }
        
        Ok(self.create_response().await)
    }
    
    /// Ping (analog of ping in Python)
    pub async fn ping(&self, session_id: Uuid) -> Result<(), SessionError> {
        self.match_session_id(session_id).await
    }
    
    /// Start Xray (analog of start in Python)
    pub async fn start(&self, session_id: Uuid, config_json: String, app_config: &crate::config::Config) -> Result<SessionResponse, SessionError> {
        self.match_session_id(session_id).await?;
        
        // Get client_ip for configuration
        let client_ip = {
            let client_ip_lock = self.client_ip.read().await;
            client_ip_lock.ok_or(SessionError::NoClientIp)?
        };
        
        // Parse configuration
        let xray_config = XrayConfig::new(&config_json, client_ip.to_string(), app_config)
            .map_err(|e| SessionError::ConfigError(e.to_string()))?;
        
        // Start Xray with logs like in Python
        let logs_buffer = self.xray_core.get_logs_buffer();
        
        self.xray_core.start(xray_config).await
            .map_err(|e| SessionError::CoreError(e.to_string()))?;
        
        // Wait for startup like in Python (3 seconds)
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(3);
        let mut last_log = String::new();
        
        while start_time.elapsed() < timeout {
            // Check logs for successful startup
            let logs = {
                let buffer = logs_buffer.lock().unwrap();
                buffer.get_all()
            };
            
            for log in logs.iter().rev().take(10) { // Take last 10 logs
                last_log = log.clone();
                if let Some(ref version) = self.core_version {
                    if log.contains(&format!("Xray {} started", version)) {
                        return Ok(self.create_response().await);
                    }
                }
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        // Check that core actually started
        if !self.xray_core.started().await {
            return Err(SessionError::StartupFailed(last_log));
        }
        
        Ok(self.create_response().await)
    }
    
    /// Stop Xray (analog of stop in Python)
    pub async fn stop(&self, session_id: Uuid) -> Result<SessionResponse, SessionError> {
        self.match_session_id(session_id).await?;
        
        // Stop core (ignore errors like in Python)
        let _ = self.xray_core.stop().await;
        
        Ok(self.create_response().await)
    }
    
    /// Restart Xray (analog of restart in Python)
    pub async fn restart(&self, session_id: Uuid, config_json: String, app_config: &crate::config::Config) -> Result<SessionResponse, SessionError> {
        self.match_session_id(session_id).await?;
        
        // Get client_ip for configuration
        let client_ip = {
            let client_ip_lock = self.client_ip.read().await;
            client_ip_lock.ok_or(SessionError::NoClientIp)?
        };
        
        // Parse configuration
        let xray_config = XrayConfig::new(&config_json, client_ip.to_string(), app_config)
            .map_err(|e| SessionError::ConfigError(e.to_string()))?;
        
        // Перезапускаем с логами как в Python
        let logs_buffer = self.xray_core.get_logs_buffer();
        
        self.xray_core.restart(xray_config).await
            .map_err(|e| SessionError::CoreError(e.to_string()))?;
        
        // Ждем запуска как в start
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(3);
        let mut last_log = String::new();
        
        while start_time.elapsed() < timeout {
            let logs = {
                let buffer = logs_buffer.lock().unwrap();
                buffer.get_all()
            };
            
            for log in logs.iter().rev().take(10) {
                last_log = log.clone();
                if let Some(ref version) = self.core_version {
                    if log.contains(&format!("Xray {} started", version)) {
                        return Ok(self.create_response().await);
                    }
                }
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        if !self.xray_core.started().await {
            return Err(SessionError::StartupFailed(last_log));
        }
        
        Ok(self.create_response().await)
    }
    
    /// Получает буфер логов для WebSocket (аналог get_logs в Python)
    pub fn get_logs_buffer(&self) -> Arc<std::sync::Mutex<crate::xray::logs::LogsBuffer>> {
        self.xray_core.get_logs_buffer()
    }
    
    /// Получает текущий session_id
    pub async fn get_session_id(&self) -> Option<Uuid> {
        *self.session_id.read().await
    }
    
    /// Проверяет подключен ли клиент
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }
}

/// Ответ сессии (аналог response в Python)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionResponse {
    pub connected: bool,
    pub started: bool,
    pub core_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<Uuid>,
}

/// Ошибки сессии
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session ID mismatch")]
    SessionMismatch,
    
    #[error("No client IP available")]
    NoClientIp,
    
    #[error("Config error: {0}")]
    ConfigError(String),
    
    #[error("Core error: {0}")]
    CoreError(String),
    
    #[error("Startup failed: {0}")]
    StartupFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;
    
    #[tokio::test]
    async fn test_session_manager_creation() {
        let manager = SessionManager::new(
            "/usr/local/bin/xray".to_string(),
            "/usr/local/share/xray".to_string()
        ).await;
        
        // Может упасть если xray не установлен - это нормально
        if manager.is_err() {
            return;
        }
        
        let manager = manager.unwrap();
        assert!(!manager.is_connected().await);
        assert!(manager.get_session_id().await.is_none());
    }
    
    #[tokio::test]
    async fn test_connect_disconnect() {
        let manager = SessionManager::new(
            "/usr/local/bin/xray".to_string(),
            "/usr/local/share/xray".to_string()
        ).await;
        
        if manager.is_err() {
            return;
        }
        
        let manager = manager.unwrap();
        let client_ip: IpAddr = "192.168.1.1".parse().unwrap();
        
        // Тест подключения
        let response = manager.connect(client_ip).await.unwrap();
        assert!(response.connected);
        assert!(response.session_id.is_some());
        assert!(manager.is_connected().await);
        
        let session_id = response.session_id.unwrap();
        
        // Тест ping
        assert!(manager.ping(session_id).await.is_ok());
        
        // Тест отключения
        let response = manager.disconnect().await.unwrap();
        assert!(!response.connected);
        assert!(!manager.is_connected().await);
    }
    
    #[tokio::test]
    async fn test_session_id_mismatch() {
        let manager = SessionManager::new(
            "/usr/local/bin/xray".to_string(),
            "/usr/local/share/xray".to_string()
        ).await;
        
        if manager.is_err() {
            return;
        }
        
        let manager = manager.unwrap();
        let wrong_session_id = Uuid::new_v4();
        
        // Должно вернуть ошибку для неправильного session_id
        assert!(matches!(
            manager.ping(wrong_session_id).await,
            Err(SessionError::SessionMismatch)
        ));
    }
} 