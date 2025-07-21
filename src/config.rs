use config::{Config as ConfigBuilder, ConfigError, Environment};
use serde::{Deserialize, Serialize};
use std::env;

/// Application configuration, identical to config.py from Python version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// SERVICE_HOST - service host (default: "0.0.0.0")
    pub service_host: String,
    
    /// SERVICE_PORT - service port (default: 62050)
    pub service_port: u16,
    
    /// XRAY_API_HOST - Xray API host (default: "0.0.0.0")
    pub xray_api_host: String,
    
    /// XRAY_API_PORT - Xray API port (default: 62051)
    pub xray_api_port: u16,
    
    /// XRAY_EXECUTABLE_PATH - path to Xray executable (default: "/usr/local/bin/xray")
    pub xray_executable_path: String,
    
    /// XRAY_ASSETS_PATH - path to Xray assets (default: "/usr/local/share/xray")
    pub xray_assets_path: String,
    
    /// SSL_CERT_FILE - SSL certificate path (default: "/var/lib/rustzban-node/ssl_cert.pem")
    pub ssl_cert_file: String,
    
    /// SSL_KEY_FILE - SSL private key path (default: "/var/lib/rustzban-node/ssl_key.pem")
    pub ssl_key_file: String,
    
    /// SSL_CLIENT_CERT_FILE - client certificate path (optional)
    pub ssl_client_cert_file: Option<String>,
    
    /// DEBUG - debug mode (default: false)
    pub debug: bool,
    
    /// SERVICE_PROTOCOL - service protocol (default: "rest")
    pub service_protocol: String,
    
    /// INBOUNDS - list of allowed inbounds (comma-separated)
    pub inbounds: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            service_host: "0.0.0.0".to_string(),
            service_port: 62050,
            xray_api_host: "0.0.0.0".to_string(),
            xray_api_port: 62051,
            xray_executable_path: "/usr/local/bin/xray".to_string(),
            xray_assets_path: "/usr/local/share/xray".to_string(),
            ssl_cert_file: "/var/lib/rustzban-node/ssl_cert.pem".to_string(),
            ssl_key_file: "/var/lib/rustzban-node/ssl_key.pem".to_string(),
            ssl_client_cert_file: None,
            debug: false,
            service_protocol: "rest".to_string(),
            inbounds: Vec::new(),
        }
    }
}

impl Config {
    /// Load configuration from environment variables
    /// Identical to logic from config.py Python version
    pub fn load() -> Result<Self, ConfigError> {
        // Load .env file if exists (like in Python version)
        dotenv::dotenv().ok();
        
        let config = ConfigBuilder::builder()
            // Set default values
            .set_default("service_host", "0.0.0.0")?
            .set_default("service_port", 62050)?
            .set_default("xray_api_host", "0.0.0.0")?
            .set_default("xray_api_port", 62051)?
            .set_default("xray_executable_path", "/usr/local/bin/xray")?
            .set_default("xray_assets_path", "/usr/local/share/xray")?
            .set_default("ssl_cert_file", "/var/lib/rustzban-node/ssl_cert.pem")?
            .set_default("ssl_key_file", "/var/lib/rustzban-node/ssl_key.pem")?
            .set_default("debug", false)?
            .set_default("service_protocol", "rest")?
            .set_default("inbounds", Vec::<String>::new())?
            // Load environment variables (like decouple.config in Python)
            .add_source(Environment::default())
            .build()?;
        
        let mut settings: Config = config.try_deserialize()?;
        
        // Handle SSL_CLIENT_CERT_FILE (can be empty string)
        if let Ok(client_cert) = env::var("SSL_CLIENT_CERT_FILE") {
            settings.ssl_client_cert_file = if client_cert.is_empty() {
                None
            } else {
                Some(client_cert)
            };
        }
        
        // Handle INBOUNDS (comma-separated list like in Python)
        if let Ok(inbounds_str) = env::var("INBOUNDS") {
            if !inbounds_str.is_empty() {
                settings.inbounds = inbounds_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            } else {
                settings.inbounds = Vec::new();
            }
        } else {
            settings.inbounds = Vec::new();
        }
        
        Ok(settings)
    }
    
    /// Configuration validation
    pub fn validate(&self) -> Result<(), String> {
        // Check that SERVICE_PROTOCOL is one of supported
        if !matches!(self.service_protocol.as_str(), "rest" | "rpyc") {
            return Err(format!(
                "SERVICE_PROTOCOL must be 'rest' or 'rpyc', got: {}",
                self.service_protocol
            ));
        }
        
        // Check port validity
        if self.service_port == 0 {
            return Err("SERVICE_PORT must be greater than 0".to_string());
        }
        
        if self.xray_api_port == 0 {
            return Err("XRAY_API_PORT must be greater than 0".to_string());
        }
        
        // Check that ports are different
        if self.service_port == self.xray_api_port {
            return Err("SERVICE_PORT and XRAY_API_PORT must be different".to_string());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.service_host, "0.0.0.0");
        assert_eq!(config.service_port, 62050);
        assert_eq!(config.xray_api_host, "0.0.0.0");
        assert_eq!(config.xray_api_port, 62051);
        assert_eq!(config.service_protocol, "rest");
        assert!(!config.debug);
        assert!(config.inbounds.is_empty());
        assert!(config.ssl_client_cert_file.is_none());
    }
    
    #[test]
    fn test_inbounds_parsing() {
        // Test INBOUNDS parsing like in Python version
        unsafe {
            env::set_var("INBOUNDS", "inbound1,inbound2, inbound3 ");
        }
        
        let config = Config::load().unwrap();
        assert_eq!(config.inbounds, vec!["inbound1", "inbound2", "inbound3"]);
        
        unsafe {
            env::remove_var("INBOUNDS");
        }
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        
        // Valid configuration
        assert!(config.validate().is_ok());
        
        // Invalid protocol
        config.service_protocol = "invalid".to_string();
        assert!(config.validate().is_err());
        
        // Same ports
        config.service_protocol = "rest".to_string();
        config.xray_api_port = config.service_port;
        assert!(config.validate().is_err());
    }
} 