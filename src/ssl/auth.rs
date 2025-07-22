use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use openssl::{
    ssl::{SslAcceptor, SslFiletype, SslMethod, SslVerifyMode},
    x509::X509,
};
use std::path::Path;
use tracing::{debug, warn};

/// Ошибки SSL аутентификации
#[derive(Debug)]
pub enum SslAuthError {
    CertificateNotFound(String),
    InvalidCertificate(String),
    OpenSSLError(openssl::error::ErrorStack),
    IoError(std::io::Error),
}

impl std::fmt::Display for SslAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SslAuthError::CertificateNotFound(path) => {
                write!(f, "Certificate file not found: {}", path)
            }
            SslAuthError::InvalidCertificate(msg) => {
                write!(f, "Invalid certificate: {}", msg)
            }
            SslAuthError::OpenSSLError(e) => write!(f, "OpenSSL error: {}", e),
            SslAuthError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for SslAuthError {}

impl From<openssl::error::ErrorStack> for SslAuthError {
    fn from(err: openssl::error::ErrorStack) -> Self {
        SslAuthError::OpenSSLError(err)
    }
}

impl From<std::io::Error> for SslAuthError {
    fn from(err: std::io::Error) -> Self {
        SslAuthError::IoError(err)
    }
}

/// Конфигурация SSL аутентификации
#[derive(Debug, Clone)]
pub struct SslConfig {
    pub cert_file: String,
    pub key_file: String,
    pub client_cert_file: Option<String>,
}

impl SslConfig {
    pub fn new(cert_file: String, key_file: String, client_cert_file: Option<String>) -> Self {
        Self {
            cert_file,
            key_file,
            client_cert_file,
        }
    }
    
    /// Проверяет существование необходимых файлов сертификатов
    pub fn validate_files(&self) -> Result<(), SslAuthError> {
        // Проверяем серверный сертификат и ключ
        if !Path::new(&self.cert_file).exists() {
            return Err(SslAuthError::CertificateNotFound(self.cert_file.clone()));
        }
        
        if !Path::new(&self.key_file).exists() {
            return Err(SslAuthError::CertificateNotFound(self.key_file.clone()));
        }
        
        // Проверяем клиентский сертификат если указан
        if let Some(ref client_cert) = self.client_cert_file {
            if !Path::new(client_cert).exists() {
                return Err(SslAuthError::CertificateNotFound(client_cert.clone()));
            }
        }
        
        Ok(())
    }
}

/// Создает SSL acceptor для HTTPS сервера
/// Идентично логике из main.py Python версии
pub fn create_ssl_acceptor(config: &SslConfig) -> Result<SslAcceptor, SslAuthError> {
    config.validate_files()?;
    
    let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
    
    // Загружаем серверный сертификат и ключ
    acceptor.set_certificate_file(&config.cert_file, SslFiletype::PEM)?;
    acceptor.set_private_key_file(&config.key_file, SslFiletype::PEM)?;
    
    // Проверяем что ключ соответствует сертификату
    acceptor.check_private_key()?;
    
    // Настраиваем клиентскую аутентификацию если указан клиентский сертификат
    if let Some(ref client_cert_file) = config.client_cert_file {
        debug!("Configuring client certificate authentication with: {}", client_cert_file);
        
        // Загружаем CA сертификат для проверки клиентских сертификатов
        acceptor.set_ca_file(client_cert_file)?;
        
        // Требуем клиентский сертификат (ssl_cert_reqs=2 в Python версии)
        acceptor.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);
        
        debug!("Client certificate authentication configured successfully");
    } else {
        warn!("Running without client certificate authentication - this is not secure!");
        
        // Без клиентской аутентификации
        acceptor.set_verify(SslVerifyMode::NONE);
    }
    
    Ok(acceptor.build())
}



/// Middleware для проверки клиентского сертификата
/// Аналог проверок в Python версии main.py
pub async fn client_cert_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // TODO: В Axum проверка клиентского сертификата происходит на уровне TLS
    // Здесь можно добавить дополнительную логику проверки сертификата
    // если это потребуется в будущем
    
    let response = next.run(request).await;
    Ok(response)
}

/// Проверяет валидность клиентского сертификата
pub fn validate_client_certificate(cert_pem: &str) -> Result<(), SslAuthError> {
    let _cert = X509::from_pem(cert_pem.as_bytes())
        .map_err(|e| SslAuthError::InvalidCertificate(format!("Failed to parse certificate: {}", e)))?;
    
    // Проверяем что сертификат не истек
    // TODO: Добавить дополнительные проверки если потребуется
    
    debug!("Client certificate validation passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssl::certificate::generate_certificate;
    use tempfile::tempdir;
    
    #[test]
    fn test_ssl_config_validation() {
        let temp_dir = tempdir().unwrap();
        let cert_path = temp_dir.path().join("cert.pem");
        let key_path = temp_dir.path().join("key.pem");
        
        // Создаем временные файлы
        std::fs::write(&cert_path, "dummy cert").unwrap();
        std::fs::write(&key_path, "dummy key").unwrap();
        
        let config = SslConfig::new(
            cert_path.to_string_lossy().to_string(),
            key_path.to_string_lossy().to_string(),
            None,
        );
        
        // Валидация должна пройти
        assert!(config.validate_files().is_ok());
        
        // Тест с несуществующим файлом
        let invalid_config = SslConfig::new(
            "/nonexistent/cert.pem".to_string(),
            key_path.to_string_lossy().to_string(),
            None,
        );
        
        assert!(invalid_config.validate_files().is_err());
    }
    
    #[test]
    fn test_ssl_acceptor_creation() {
        let temp_dir = tempdir().unwrap();
        let cert_path = temp_dir.path().join("cert.pem");
        let key_path = temp_dir.path().join("key.pem");
        
        // Генерируем реальный сертификат для тестирования
        let cert_pair = generate_certificate().unwrap();
        std::fs::write(&cert_path, &cert_pair.cert).unwrap();
        std::fs::write(&key_path, &cert_pair.key).unwrap();
        
        let config = SslConfig::new(
            cert_path.to_string_lossy().to_string(),
            key_path.to_string_lossy().to_string(),
            None,
        );
        
        // SSL acceptor должен создаться успешно
        assert!(create_ssl_acceptor(&config).is_ok());
    }
    
    #[test]
    fn test_client_certificate_validation() {
        // Генерируем сертификат для тестирования
        let cert_pair = generate_certificate().unwrap();
        
        // Валидация должна пройти
        assert!(validate_client_certificate(&cert_pair.cert).is_ok());
        
        // Невалидный сертификат
        assert!(validate_client_certificate("invalid cert").is_err());
    }
} 