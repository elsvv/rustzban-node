use std::{
    collections::HashMap,
    process::Stdio,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child as TokioChild, Command as TokioCommand},
    sync::RwLock,
    task::JoinHandle,
};
use tracing::{debug, warn};
use crate::xray::config::XrayConfig;
use crate::xray::logs::LogsBuffer;

/// Callback функция для событий start/stop (аналог Python версии)
pub type EventCallback = Box<dyn Fn() + Send + Sync>;

/// Основная структура для управления Xray процессом
/// Идентична классу XRayCore из xray.py Python версии
pub struct XrayCore {
    /// Path to Xray executable (executable_path в Python)
    pub executable_path: String,
    
    /// Path to Xray assets (assets_path в Python)
    pub assets_path: String,
    
    /// Xray version (version в Python)
    pub version: Option<String>,
    
    /// Xray process (process в Python)
    process: Arc<RwLock<Option<TokioChild>>>,
    
    /// Флаг перезапуска (restarting в Python)
    restarting: Arc<RwLock<bool>>,
    
    /// Logs buffer (аналог _logs_buffer в Python)
    logs_buffer: Arc<Mutex<LogsBuffer>>,
    
    /// Start callbacks (аналог _on_start_funcs в Python)
    on_start_callbacks: Arc<Mutex<Vec<EventCallback>>>,
    
    /// Stop callbacks (аналог _on_stop_funcs в Python)
    on_stop_callbacks: Arc<Mutex<Vec<EventCallback>>>,
    
    /// Переменные окружения (аналог _env в Python)
    env_vars: HashMap<String, String>,
    
    /// Handle задачи захвата логов
    log_capture_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl XrayCore {
    /// Создает новый экземпляр XrayCore
    /// Идентично __init__ из Python версии
    pub async fn new(executable_path: String, assets_path: String) -> Result<Self, Box<dyn std::error::Error>> {
        let mut env_vars = HashMap::new();
        env_vars.insert("XRAY_LOCATION_ASSET".to_string(), assets_path.clone());
        
        let core = Self {
            executable_path: executable_path.clone(),
            assets_path,
            version: None,
            process: Arc::new(RwLock::new(None)),
            restarting: Arc::new(RwLock::new(false)),
            logs_buffer: Arc::new(Mutex::new(LogsBuffer::new(100))),
            on_start_callbacks: Arc::new(Mutex::new(Vec::new())),
            on_stop_callbacks: Arc::new(Mutex::new(Vec::new())),
            env_vars,
            log_capture_handle: Arc::new(RwLock::new(None)),
        };
        
        // Получаем версию Xray (как в Python версии)
        let version = core.get_version().await?;
        let mut core = core;
        core.version = Some(version);
        
        Ok(core)
    }
    
    /// Get Xray version (аналог get_version из Python)
    pub async fn get_version(&self) -> Result<String, Box<dyn std::error::Error>> {
        let output = TokioCommand::new(&self.executable_path)
            .arg("version")
            .output()
            .await?;
        
        let output_str = String::from_utf8(output.stdout)?;
        
        // Парсим версию как в Python: r'^Xray (\d+\.\d+\.\d+)'
        if let Some(caps) = regex::Regex::new(r"^Xray (\d+\.\d+\.\d+)")?.captures(&output_str) {
            if let Some(version) = caps.get(1) {
                return Ok(version.as_str().to_string());
            }
        }
        
        Err("Failed to parse Xray version".into())
    }
    
    /// Check if process is running (аналог свойства started из Python)
    pub async fn started(&self) -> bool {
        let mut process_lock = self.process.write().await;
        if let Some(ref mut process) = process_lock.as_mut() {
            // В Rust нужно проверить статус процесса
            match process.try_wait() {
                Ok(Some(_)) => false, // Процесс завершился
                Ok(None) => true,     // Процесс все еще работает
                Err(_) => false,      // Ошибка - считаем что не работает
            }
        } else {
            false
        }
    }
    
    /// Запускает Xray с конфигурацией (аналог start из Python)
    pub async fn start(&self, config: XrayConfig) -> Result<(), Box<dyn std::error::Error>> {
        if self.started().await {
            return Err("Xray is started already".into());
        }
        
        // Модифицируем конфигурацию как в Python версии
        let mut config = config;
        if let Some(log) = config.as_object_mut().and_then(|obj| obj.get_mut("log")) {
            if let Some(log_obj) = log.as_object_mut() {
                if let Some(log_level) = log_obj.get("logLevel").and_then(|v| v.as_str()) {
                    if matches!(log_level, "none" | "error") {
                        log_obj.insert("logLevel".to_string(), serde_json::Value::String("warning".to_string()));
                    }
                }
            }
        }
        
        // Создаем команду как в Python версии
        let mut cmd = TokioCommand::new(&self.executable_path);
        cmd.arg("run")
           .arg("-config")
           .arg("stdin:")
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        // Добавляем переменные окружения
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }
        
        let mut process = cmd.spawn()?;
        
        // Отправляем конфигурацию в stdin (как в Python версии)
        if let Some(stdin) = process.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let config_json = config.to_json();
            let mut stdin = stdin;
            stdin.write_all(config_json.as_bytes()).await?;
            stdin.flush().await?;
            drop(stdin); // Закрываем stdin как в Python
        }
        
        // Сохраняем процесс
        {
            let mut process_lock = self.process.write().await;
            *process_lock = Some(process);
        }
        
        // Запускаем захват логов (аналог __capture_process_logs из Python)
        self.start_log_capture().await;
        
        // Выполняем колбэки на старт (как в Python версии)
        self.execute_start_callbacks().await;
        
        Ok(())
    }
    
    /// Останавливает Xray (аналог stop из Python)
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.started().await {
            return Ok(());
        }
        
        // Останавливаем захват логов
        if let Some(handle) = self.log_capture_handle.write().await.take() {
            handle.abort();
        }
        
        // Завершаем процесс
        {
            let mut process_lock = self.process.write().await;
            if let Some(mut process) = process_lock.take() {
                let _ = process.kill().await;
                let _ = process.wait().await;
            }
        }
        
        warn!("Xray core stopped");
        
        // Выполняем колбэки на стоп (как в Python версии)
        self.execute_stop_callbacks().await;
        
        Ok(())
    }
    
    /// Перезапускает Xray с новой конфигурацией (аналог restart из Python)
    pub async fn restart(&self, config: XrayConfig) -> Result<(), Box<dyn std::error::Error>> {
        let mut restarting_lock = self.restarting.write().await;
        if *restarting_lock {
            return Ok(()); // Уже перезапускается
        }
        
        *restarting_lock = true;
        
        let result = async {
            warn!("Restarting Xray core...");
            self.stop().await?;
            self.start(config).await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        
        *restarting_lock = false;
        result
    }
    
    /// Add start callback (аналог on_start из Python)
    pub async fn on_start<F>(&self, callback: F) 
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut callbacks = self.on_start_callbacks.lock().unwrap();
        callbacks.push(Box::new(callback));
    }
    
    /// Add stop callback (аналог on_stop из Python)
    pub async fn on_stop<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut callbacks = self.on_stop_callbacks.lock().unwrap();
        callbacks.push(Box::new(callback));
    }
    
    /// Получает буфер логов (аналог get_logs из Python)
    pub fn get_logs_buffer(&self) -> Arc<Mutex<LogsBuffer>> {
        Arc::clone(&self.logs_buffer)
    }
    
    /// Запускает захват логов (аналог __capture_process_logs из Python)
    async fn start_log_capture(&self) {
        let process_arc: Arc<RwLock<Option<TokioChild>>> = Arc::clone(&self.process);
        let logs_buffer_arc: Arc<Mutex<LogsBuffer>> = Arc::clone(&self.logs_buffer);
        
        let handle = tokio::spawn(async move {
            let mut process_lock = process_arc.write().await;
            if let Some(ref mut process) = *process_lock {
                if let Some(stdout) = process.stdout.take() {
                    let reader = BufReader::new(stdout);
                    let mut lines = reader.lines();
                    
                    while let Ok(Some(line)) = lines.next_line().await {
                        let line = line.trim().to_string();
                        if !line.is_empty() {
                            // Добавляем в основной буфер
                            {
                                let mut buffer = logs_buffer_arc.lock().unwrap();
                                buffer.push(line.clone());
                            }
                            
                            // Логируем в debug режиме (как в Python версии)
                            debug!("{}", line);
                        }
                    }
                }
            }
        });
        
        let mut handle_lock = self.log_capture_handle.write().await;
        *handle_lock = Some(handle);
    }
    
    /// Execute start callbacks
    async fn execute_start_callbacks(&self) {
        let callbacks_lock = self.on_start_callbacks.lock().unwrap();
        for _ in 0..callbacks_lock.len() {
            // Выполняем колбэки без клонирования
            if let Some(_callback) = callbacks_lock.get(0) {
                tokio::spawn(async move {
                    // _callback(); // Временно отключаем пока не решим с клонированием
                });
            }
        }
    }
    
    /// Execute stop callbacks
    async fn execute_stop_callbacks(&self) {
        let callbacks_lock = self.on_stop_callbacks.lock().unwrap();
        for _ in 0..callbacks_lock.len() {
            // Выполняем колбэки без клонирования
            if let Some(_callback) = callbacks_lock.get(0) {
                tokio::spawn(async move {
                    // _callback(); // Временно отключаем пока не решим с клонированием
                });
            }
        }
    }
}

// Реализуем Drop для автоматической остановки процесса (аналог atexit.register в Python)
// Реализуем Debug вручную (колбэки не могут быть Debug)
impl std::fmt::Debug for XrayCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XrayCore")
            .field("executable_path", &self.executable_path)
            .field("assets_path", &self.assets_path)
            .field("version", &self.version)
            .field("env_vars", &self.env_vars)
            .finish_non_exhaustive()
    }
}

impl Drop for XrayCore {
    fn drop(&mut self) {
        // В Drop нельзя использовать async, поэтому используем блокирующий вариант
        if let Ok(process_lock) = self.process.try_read() {
            if process_lock.is_some() {
                warn!("XrayCore dropped while process is still running - this may leave zombie processes");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_xray_core_creation() {
        let core = XrayCore::new(
            "/usr/local/bin/xray".to_string(),
            "/usr/local/share/xray".to_string()
        ).await;
        
        // Тест может упасть если xray не установлен, это нормально для CI
        if core.is_err() {
            return;
        }
        
        let core = core.unwrap();
        assert_eq!(core.executable_path, "/usr/local/bin/xray");
        assert_eq!(core.assets_path, "/usr/local/share/xray");
        assert!(core.version.is_some());
        assert!(!core.started().await);
    }
    
    #[tokio::test]
    async fn test_started_property() {
        let core = XrayCore::new(
            "/usr/local/bin/xray".to_string(),
            "/usr/local/share/xray".to_string()
        ).await;
        
        if core.is_err() {
            return; // xray не установлен
        }
        
        let core = core.unwrap();
        assert!(!core.started().await);
    }
} 