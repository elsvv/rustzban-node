mod config;
mod ssl;
mod utils;
mod xray;
mod api;
mod session;

use config::Config;
use ssl::certificate::{generate_certificate, save_certificate_files};
use utils::logging::init_logging;
use std::fs;
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Загружаем конфигурацию
    let config = Config::load().map_err(|e| {
        eprintln!("Failed to load configuration: {}", e);
        std::process::exit(1);
    })?;
    
    // Валидируем конфигурацию
    if let Err(e) = config.validate() {
        eprintln!("Configuration validation failed: {}", e);
        std::process::exit(1);
    }
    
    // Инициализируем логирование
    init_logging(config.debug)?;
    
    info!("Starting Marzban Node (Rust version)");
    info!("Service protocol: {}", config.service_protocol);
    info!("Service host: {}:{}", config.service_host, config.service_port);
    
    // Проверяем и генерируем SSL сертификаты если необходимо
    // Идентично логике из main.py Python версии
    if !fs::metadata(&config.ssl_cert_file).is_ok() || !fs::metadata(&config.ssl_key_file).is_ok() {
        info!("SSL certificate or key file missing, generating new ones...");
        
        let cert_pair = generate_certificate().map_err(|e| {
            error!("Failed to generate SSL certificate: {}", e);
            e
        })?;
        
        save_certificate_files(&cert_pair, &config.ssl_cert_file, &config.ssl_key_file)
            .map_err(|e| {
                error!("Failed to save SSL certificate files: {}", e);
                e
            })?;
        
        info!("SSL certificate and key generated successfully");
    }
    
    // Проверяем клиентский сертификат (как в Python версии)
    if config.ssl_client_cert_file.is_none() {
        warn!("You are running node without SSL_CLIENT_CERT_FILE, be aware that everyone can connect to this node and this isn't secure!");
    }
    
    if let Some(ref client_cert_file) = config.ssl_client_cert_file {
        if !fs::metadata(client_cert_file).is_ok() {
            error!("Client's certificate file specified on SSL_CLIENT_CERT_FILE is missing");
            std::process::exit(1);
        }
    }
    
    // Check service protocol
    match config.service_protocol.as_str() {
        "rest" => {
            // For REST protocol check that client certificate is specified
            if config.ssl_client_cert_file.is_none() {
                warn!("SSL_CLIENT_CERT_FILE is not set. This is not secure for production use!");
            }
            
            info!("Starting REST server on {}:{}", config.service_host, config.service_port);
            
            // Start REST server
            crate::api::create_rest_server(std::sync::Arc::new(config)).await?;
        }
        "rpyc" => {
            info!("Node service running on :{}", config.service_port);
            
            // TODO: Start RPyC server (will be implemented in future versions)
            info!("RPyC service not implemented yet");
        }
        _ => {
            error!("SERVICE_PROTOCOL is not any of (rpyc, rest).");
            std::process::exit(1);
        }
    }
    
    Ok(())
} 