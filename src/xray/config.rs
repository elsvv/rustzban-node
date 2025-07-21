use serde_json::{json, Map, Value};
use std::fmt;
use crate::config::Config as AppConfig;

/// Конфигурация Xray (аналог класса XRayConfig из xray.py Python версии)
/// Наследуется от serde_json::Value чтобы работать как dict в Python
#[derive(Debug, Clone)]
pub struct XrayConfig {
    /// JSON configuration Xray
    config: Value,
    /// IP адрес клиента (peer_ip в Python)
    peer_ip: String,
    /// Хост для API (аналог self.api_host в Python)
    api_host: String,
    /// Порт для API (аналог self.api_port в Python)
    api_port: u16,
    /// Путь к SSL сертификату (аналог self.ssl_cert в Python)
    ssl_cert: String,
    /// Путь к SSL ключу (аналог self.ssl_key в Python)
    ssl_key: String,
    /// Разрешенные inbounds (аналог INBOUNDS в Python)
    inbounds_filter: Vec<String>,
}

impl XrayConfig {
    /// Create new configuration Xray из JSON строки
    /// Идентично __init__ из Python версии
    pub fn new(config_json: &str, peer_ip: String, app_config: &AppConfig) -> Result<Self, XrayConfigError> {
        // Парсим JSON как в Python версии
        let config: Value = serde_json::from_str(config_json)
            .map_err(|e| XrayConfigError::JsonParseError(e.to_string()))?;
        
        let mut xray_config = Self {
            config,
            peer_ip,
            api_host: app_config.xray_api_host.clone(),
            api_port: app_config.xray_api_port,
            ssl_cert: app_config.ssl_cert_file.clone(),
            ssl_key: app_config.ssl_key_file.clone(),
            inbounds_filter: app_config.inbounds.clone(),
        };
        
        // Применяем API настройки (аналог self._apply_api() в Python)
        xray_config.apply_api()?;
        
        Ok(xray_config)
    }
    
    /// Конвертирует конфигурацию в JSON строку
    /// Идентично методу to_json из Python версии
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.config).unwrap_or_default()
    }
    
    /// Получает значение из конфигурации как объект
    pub fn as_object(&self) -> Option<&Map<String, Value>> {
        self.config.as_object()
    }
    
    /// Получает мутабельную ссылку на объект конфигурации
    pub fn as_object_mut(&mut self) -> Option<&mut Map<String, Value>> {
        self.config.as_object_mut()
    }
    
    /// Применяет настройки API к конфигурации
    /// Идентично методу _apply_api из Python версии
    fn apply_api(&mut self) -> Result<(), XrayConfigError> {
        let config_obj = self.config.as_object_mut()
            .ok_or(XrayConfigError::InvalidConfig("Config is not an object".to_string()))?;
        
        // Удаляем существующие API inbounds и фильтруем по INBOUNDS (как в Python)
        if let Some(inbounds) = config_obj.get_mut("inbounds") {
            if let Some(inbounds_array) = inbounds.as_array_mut() {
                // Создаем новый массив без API_INBOUND и с фильтрацией по INBOUNDS
                let filtered_inbounds: Vec<Value> = inbounds_array
                    .iter()
                    .filter(|inbound| {
                        // Удаляем API_INBOUND (как в Python)
                        if let Some(obj) = inbound.as_object() {
                            if obj.get("protocol").and_then(|v| v.as_str()) == Some("dokodemo-door") &&
                               obj.get("tag").and_then(|v| v.as_str()) == Some("API_INBOUND") {
                                return false;
                            }
                            
                            // Фильтруем по INBOUNDS если указано (как в Python)
                            if !self.inbounds_filter.is_empty() {
                                if let Some(tag) = obj.get("tag").and_then(|v| v.as_str()) {
                                    return self.inbounds_filter.contains(&tag.to_string());
                                }
                            }
                        }
                        true
                    })
                    .cloned()
                    .collect();
                
                *inbounds_array = filtered_inbounds;
            }
        }
        
                // Получаем API tag для фильтрации (до мутабельных операций)
        let api_tag = config_obj.get("api")
            .and_then(|api| api.as_object())
            .and_then(|api_obj| api_obj.get("tag"))
            .and_then(|tag| tag.as_str())
            .map(|s| s.to_string());
        
        // Удаляем существующие API routing rules (как в Python)
        if let Some(routing) = config_obj.get_mut("routing") {
            if let Some(routing_obj) = routing.as_object_mut() {
                if let Some(rules) = routing_obj.get_mut("rules") {
                    if let Some(rules_array) = rules.as_array_mut() {
                        if let Some(ref api_tag) = api_tag {
                            rules_array.retain(|rule| {
                                if let Some(rule_obj) = rule.as_object() {
                                    if let Some(outbound_tag) = rule_obj.get("outboundTag").and_then(|v| v.as_str()) {
                                        return outbound_tag != api_tag;
                                    }
                                }
                                true
                            });
                        }
                    }
                }
            }
        }
        
        // Добавляем API конфигурацию (идентично Python версии)
        config_obj.insert("api".to_string(), json!({
            "services": [
                "HandlerService",
                "StatsService", 
                "LoggerService"
            ],
            "tag": "API"
        }));
        
        config_obj.insert("stats".to_string(), json!({}));
        
        // Создаем API inbound (идентично Python версии)
        let api_inbound = json!({
            "listen": self.api_host,
            "port": self.api_port,
            "protocol": "dokodemo-door",
            "settings": {
                "address": "127.0.0.1"
            },
            "streamSettings": {
                "security": "tls",
                "tlsSettings": {
                    "certificates": [
                        {
                            "certificateFile": self.ssl_cert,
                            "keyFile": self.ssl_key
                        }
                    ]
                }
            },
            "tag": "API_INBOUND"
        });
        
        // Добавляем API inbound в начало массива (как в Python)
        if let Some(inbounds) = config_obj.get_mut("inbounds") {
            if let Some(inbounds_array) = inbounds.as_array_mut() {
                inbounds_array.insert(0, api_inbound);
            }
        } else {
            config_obj.insert("inbounds".to_string(), json!([api_inbound]));
        }
        
        // Создаем routing rule для API (идентично Python версии)
        let api_rule = json!({
            "inboundTag": ["API_INBOUND"],
            "source": ["127.0.0.1", self.peer_ip],
            "outboundTag": "API",
            "type": "field"
        });
        
        // Добавляем rule в начало массива (как в Python)
        if let Some(routing) = config_obj.get_mut("routing") {
            if let Some(routing_obj) = routing.as_object_mut() {
                if let Some(rules) = routing_obj.get_mut("rules") {
                    if let Some(rules_array) = rules.as_array_mut() {
                        rules_array.insert(0, api_rule);
                    }
                } else {
                    routing_obj.insert("rules".to_string(), json!([api_rule]));
                }
            }
        } else {
            config_obj.insert("routing".to_string(), json!({
                "rules": [api_rule]
            }));
        }
        
        Ok(())
    }
}

/// Ошибки конфигурации Xray
#[derive(Debug)]
pub enum XrayConfigError {
    JsonParseError(String),
    InvalidConfig(String),
}

impl fmt::Display for XrayConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            XrayConfigError::JsonParseError(msg) => write!(f, "JSON parse error: {}", msg),
            XrayConfigError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
        }
    }
}

impl std::error::Error for XrayConfigError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config as AppConfig;
    
    fn create_test_app_config() -> AppConfig {
        AppConfig {
            service_host: "0.0.0.0".to_string(),
            service_port: 62050,
            xray_api_host: "127.0.0.1".to_string(),
            xray_api_port: 62051,
            xray_executable_path: "/usr/local/bin/xray".to_string(),
            xray_assets_path: "/usr/local/share/xray".to_string(),
            ssl_cert_file: "/tmp/cert.pem".to_string(),
            ssl_key_file: "/tmp/key.pem".to_string(),
            ssl_client_cert_file: None,
            debug: false,
            service_protocol: "rest".to_string(),
            inbounds: vec![],
        }
    }
    
    #[test]
    fn test_xray_config_creation() {
        let config_json = r#"{
            "inbounds": [],
            "outbounds": [{"protocol": "freedom"}]
        }"#;
        
        let app_config = create_test_app_config();
        let xray_config = XrayConfig::new(config_json, "192.168.1.1".to_string(), &app_config);
        
        assert!(xray_config.is_ok());
        let xray_config = xray_config.unwrap();
        
        // Проверяем что API добавлен
        let config_obj = xray_config.as_object().unwrap();
        assert!(config_obj.contains_key("api"));
        assert!(config_obj.contains_key("stats"));
    }
    
    #[test]
    fn test_api_inbound_added() {
        let config_json = r#"{
            "inbounds": [{"protocol": "vmess", "tag": "vmess-in"}]
        }"#;
        
        let app_config = create_test_app_config();
        let xray_config = XrayConfig::new(config_json, "192.168.1.1".to_string(), &app_config).unwrap();
        
        let config_obj = xray_config.as_object().unwrap();
        let inbounds = config_obj.get("inbounds").unwrap().as_array().unwrap();
        
        // API_INBOUND должен быть первым
        assert_eq!(inbounds.len(), 2);
        let api_inbound = &inbounds[0];
        assert_eq!(api_inbound.get("tag").unwrap().as_str().unwrap(), "API_INBOUND");
        assert_eq!(api_inbound.get("protocol").unwrap().as_str().unwrap(), "dokodemo-door");
    }
    
    #[test]
    fn test_inbounds_filtering() {
        let config_json = r#"{
            "inbounds": [
                {"protocol": "vmess", "tag": "vmess-in"},
                {"protocol": "trojan", "tag": "trojan-in"}
            ]
        }"#;
        
        let mut app_config = create_test_app_config();
        app_config.inbounds = vec!["vmess-in".to_string()]; // Разрешаем только vmess-in
        
        let xray_config = XrayConfig::new(config_json, "192.168.1.1".to_string(), &app_config).unwrap();
        
        let config_obj = xray_config.as_object().unwrap();
        let inbounds = config_obj.get("inbounds").unwrap().as_array().unwrap();
        
        // Должно быть 2 inbound: API_INBOUND + vmess-in (trojan-in отфильтрован)
        assert_eq!(inbounds.len(), 2);
        
        let user_inbound = &inbounds[1]; // API_INBOUND всегда первый
        assert_eq!(user_inbound.get("tag").unwrap().as_str().unwrap(), "vmess-in");
    }
    
    #[test]
    fn test_to_json() {
        let config_json = r#"{"inbounds": []}"#;
        let app_config = create_test_app_config();
        let xray_config = XrayConfig::new(config_json, "192.168.1.1".to_string(), &app_config).unwrap();
        
        let json_output = xray_config.to_json();
        assert!(!json_output.is_empty());
        
        // Проверяем что JSON валидный
        let parsed: Value = serde_json::from_str(&json_output).unwrap();
        assert!(parsed.is_object());
    }
} 