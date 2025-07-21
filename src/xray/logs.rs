use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Buffer for Xray logs (аналог _logs_buffer из Python версии)
/// Использует VecDeque с ограниченным размером как deque(maxlen=100) в Python
#[derive(Debug)]
pub struct LogsBuffer {
    /// Внутренний буфер с ограниченным размером
    buffer: VecDeque<String>,
    /// Максимальный размер буфера
    max_size: usize,
}

impl LogsBuffer {
    /// Создает новый буфер с указанным максимальным размером
    /// Аналогично deque(maxlen=max_size) в Python
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(max_size),
            max_size,
        }
    }
    
    /// Добавляет новую запись в буфер
    /// Автоматически удаляет старые записи если превышен размер
    pub fn push(&mut self, line: String) {
        if self.buffer.len() >= self.max_size {
            self.buffer.pop_front();
        }
        self.buffer.push_back(line);
    }
    
    /// Возвращает все логи в буфере
    pub fn get_all(&self) -> Vec<String> {
        self.buffer.iter().cloned().collect()
    }
    
    /// Возвращает количество записей в буфере
    pub fn len(&self) -> usize {
        self.buffer.len()
    }
    
    /// Проверяет пуст ли буфер
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
    
    /// Очищает буфер
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
    
    /// Создает копию буфера (для временных буферов как в Python)
    pub fn clone_buffer(&self) -> VecDeque<String> {
        self.buffer.clone()
    }
}

/// Temporary logs buffer manager
/// Аналог _temp_log_buffers из Python версии
#[derive(Debug)]
pub struct TempLogsManager {
    /// Временные буферы с уникальными ID
    temp_buffers: Arc<Mutex<std::collections::HashMap<usize, Arc<Mutex<VecDeque<String>>>>>>,
    /// Счетчик для генерации уникальных ID
    next_id: Arc<Mutex<usize>>,
}

impl TempLogsManager {
    /// Create new temporary buffer manager
    pub fn new() -> Self {
        Self {
            temp_buffers: Arc::new(Mutex::new(std::collections::HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
        }
    }
    
    /// Создает новый временный буфер (аналог get_logs context manager в Python)
    pub fn create_temp_buffer(&self) -> TempLogBuffer {
        let id = {
            let mut next_id = self.next_id.lock().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };
        
        let buffer = Arc::new(Mutex::new(VecDeque::with_capacity(100)));
        
        {
            let mut temp_buffers = self.temp_buffers.lock().unwrap();
            temp_buffers.insert(id, Arc::clone(&buffer));
        }
        
        TempLogBuffer {
            id,
            buffer,
            manager: Arc::clone(&self.temp_buffers),
        }
    }
    
    /// Добавляет строку во все временные буферы
    pub fn push_to_all(&self, line: String) {
        let temp_buffers = self.temp_buffers.lock().unwrap();
        for buffer in temp_buffers.values() {
            let mut buf = buffer.lock().unwrap();
            if buf.len() >= 100 {
                buf.pop_front();
            }
            buf.push_back(line.clone());
        }
    }
}

/// Временный буфер логов (аналог контекстного менеджера get_logs в Python)
pub struct TempLogBuffer {
    id: usize,
    buffer: Arc<Mutex<VecDeque<String>>>,
    manager: Arc<Mutex<std::collections::HashMap<usize, Arc<Mutex<VecDeque<String>>>>>>,
}

impl TempLogBuffer {
    /// Получает все логи из временного буфера
    pub fn get_logs(&self) -> Vec<String> {
        let buffer = self.buffer.lock().unwrap();
        buffer.iter().cloned().collect()
    }
    
    /// Проверяет есть ли новые логи
    pub fn has_logs(&self) -> bool {
        let buffer = self.buffer.lock().unwrap();
        !buffer.is_empty()
    }
    
    /// Извлекает один лог из буфера (аналог popleft в Python)
    pub fn pop_log(&self) -> Option<String> {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.pop_front()
    }
    
    /// Возвращает количество логов в буфере
    pub fn len(&self) -> usize {
        let buffer = self.buffer.lock().unwrap();
        buffer.len()
    }
    
    /// Проверяет пуст ли буфер
    pub fn is_empty(&self) -> bool {
        let buffer = self.buffer.lock().unwrap();
        buffer.is_empty()
    }
}

// Автоматическая очистка при удалении (аналог finally в Python context manager)
impl Drop for TempLogBuffer {
    fn drop(&mut self) {
        let mut manager = self.manager.lock().unwrap();
        manager.remove(&self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_logs_buffer_basic() {
        let mut buffer = LogsBuffer::new(3);
        
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        
        buffer.push("log1".to_string());
        buffer.push("log2".to_string());
        buffer.push("log3".to_string());
        
        assert_eq!(buffer.len(), 3);
        assert!(!buffer.is_empty());
        
        let logs = buffer.get_all();
        assert_eq!(logs, vec!["log1", "log2", "log3"]);
    }
    
    #[test]
    fn test_logs_buffer_overflow() {
        let mut buffer = LogsBuffer::new(2);
        
        buffer.push("log1".to_string());
        buffer.push("log2".to_string());
        buffer.push("log3".to_string()); // Должен вытолкнуть log1
        
        assert_eq!(buffer.len(), 2);
        
        let logs = buffer.get_all();
        assert_eq!(logs, vec!["log2", "log3"]);
    }
    
    #[test]
    fn test_temp_logs_manager() {
        let manager = TempLogsManager::new();
        
        let temp_buffer1 = manager.create_temp_buffer();
        let temp_buffer2 = manager.create_temp_buffer();
        
        assert!(temp_buffer1.is_empty());
        assert!(temp_buffer2.is_empty());
        
        // Добавляем лог во все буферы
        manager.push_to_all("test log".to_string());
        
        assert_eq!(temp_buffer1.len(), 1);
        assert_eq!(temp_buffer2.len(), 1);
        
        let log1 = temp_buffer1.pop_log().unwrap();
        let log2 = temp_buffer2.pop_log().unwrap();
        
        assert_eq!(log1, "test log");
        assert_eq!(log2, "test log");
    }
    
    #[test]
    fn test_temp_buffer_drop() {
        let manager = TempLogsManager::new();
        
        {
            let temp_buffer = manager.create_temp_buffer();
            manager.push_to_all("test".to_string());
            assert_eq!(temp_buffer.len(), 1);
        } // temp_buffer должен удалиться здесь
        
        // Создаем новый буфер - старые логи не должны попасть в него
        let new_buffer = manager.create_temp_buffer();
        assert!(new_buffer.is_empty());
    }
} 