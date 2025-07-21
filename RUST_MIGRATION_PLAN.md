# Rustzban Node - Development Plan

## 🎯 Project Goal

Create **Rustzban Node** - a high-performance Rust implementation and fork of the original Python-based Marzban Node for the Marzban ecosystem, achieving:

-   **60-80% снижения потребления памяти** (50-200MB → 10-40MB)
-   **30-70% снижения нагрузки на CPU**
-   **90-95% ускорения времени запуска** (2-10s → 0.1-0.5s)
-   **200-500% увеличения throughput** для concurrent connections
-   **40-70% снижения затрат** на серверную инфраструктуру

## 📋 Требования к совместимости

**КРИТИЧНО**: Итоговый Rust проект должен быть **полностью совместим** по внешнему API с оригинальной Python нодой:

-   ✅ Идентичные REST API endpoints
-   ✅ Идентичные JSON request/response форматы
-   ✅ Идентичное поведение WebSocket для логов
-   ✅ Идентичная SSL аутентификация
-   ✅ Идентичная работа с Xray конфигурацией
-   ✅ Идентичные переменные окружения

**TODO для будущих версий:**

-   🔄 Реализация gRPC сервиса (замена RPyC)
-   🔄 Поддержка gRPC в мастер ноде Marzban
-   🔄 Миграция с RPyC на gRPC в production

## 🛠️ Технологический стек

### Core Framework

-   **[Axum 0.7+](https://github.com/tokio-rs/axum)** - веб-фреймворк (замена FastAPI)
-   **[Tokio 1.0+](https://tokio.rs/)** - async runtime
-   **[Tower 0.4+](https://github.com/tower-rs/tower)** - middleware ecosystem

### SSL/TLS и криптография

-   **[openssl 0.10+](https://github.com/sfackler/rust-openssl)** - OpenSSL bindings (вместо rustls)
-   **[openssl-sys](https://github.com/sfackler/rust-openssl)** - системные bindings

### Дополнительные библиотеки

-   **[serde 1.0+](https://serde.rs/)** - JSON serialization
-   **[tokio-tungstenite 0.20+](https://github.com/snapview/tokio-tungstenite)** - WebSocket
-   **[tracing 0.1+](https://github.com/tokio-rs/tracing)** - structured logging
-   **[config 0.13+](https://github.com/mehcode/config-rs)** - configuration management
-   **[uuid 1.0+](https://github.com/uuid-rs/uuid)** - UUID generation
-   **[tokio-process](https://docs.rs/tokio-process/)** - async subprocess management

## 🏗️ Структура проекта

```
marzban-node-rust/
├── src/
│   ├── main.rs                 # Entry point + server setup
│   ├── config.rs               # Configuration management
│   ├── ssl/
│   │   ├── mod.rs
│   │   ├── certificate.rs      # SSL certificate generation
│   │   └── auth.rs            # Client certificate authentication
│   ├── xray/
│   │   ├── mod.rs
│   │   ├── process.rs         # Xray process management
│   │   ├── config.rs          # Xray configuration handling
│   │   └── logs.rs            # Log streaming and buffering
│   ├── api/
│   │   ├── mod.rs
│   │   ├── handlers.rs        # REST API handlers
│   │   ├── websocket.rs       # WebSocket log streaming
│   │   ├── middleware.rs      # Authentication middleware
│   │   └── models.rs          # Request/Response models
│   ├── session/
│   │   ├── mod.rs
│   │   └── manager.rs         # Session management
│   └── utils/
│       ├── mod.rs
│       └── logging.rs         # Logging setup
├── Cargo.toml                 # Dependencies
├── Dockerfile                 # Multi-stage Docker build
├── docker-compose.yml         # Docker Compose configuration
└── README.md                  # Documentation
```

---

## 📅 Поэтапный план разработки

### 🔧 Этап 1: Инфраструктура и настройка (1-2 недели)

#### 1.1 Создание проекта

```bash
# Создание нового Rust проекта
cargo new marzban-node-rust --bin
cd marzban-node-rust

# Добавление основных зависимостей
cargo add tokio --features full
cargo add axum --features ws,multipart
cargo add tower --features full
cargo add serde --features derive
cargo add serde_json
cargo add uuid --features v4,serde
cargo add tracing
cargo add tracing-subscriber
cargo add config
cargo add openssl --features vendored
cargo add tokio-tungstenite
cargo add anyhow
cargo add thiserror
```

#### 1.2 Настройка конфигурации (`src/config.rs`)

**Задачи:**

-   ✅ Загрузка переменных окружения (замена `config.py`)
-   ✅ Валидация конфигурации
-   ✅ Значения по умолчанию идентичные Python версии

**Требования совместимости:**

```rust
// Должны поддерживаться ВСЕ переменные из config.py:
// SERVICE_HOST, SERVICE_PORT, XRAY_API_HOST, XRAY_API_PORT
// XRAY_EXECUTABLE_PATH, XRAY_ASSETS_PATH
// SSL_CERT_FILE, SSL_KEY_FILE, SSL_CLIENT_CERT_FILE
// DEBUG, SERVICE_PROTOCOL, INBOUNDS
```

**Пример структуры:**

```rust
#[derive(Debug, Clone)]
pub struct Config {
    pub service_host: String,           // SERVICE_HOST
    pub service_port: u16,              // SERVICE_PORT
    pub xray_api_host: String,          // XRAY_API_HOST
    pub xray_api_port: u16,             // XRAY_API_PORT
    pub xray_executable_path: String,   // XRAY_EXECUTABLE_PATH
    pub xray_assets_path: String,       // XRAY_ASSETS_PATH
    pub ssl_cert_file: String,          // SSL_CERT_FILE
    pub ssl_key_file: String,           // SSL_KEY_FILE
    pub ssl_client_cert_file: Option<String>, // SSL_CLIENT_CERT_FILE
    pub debug: bool,                    // DEBUG
    pub inbounds: Vec<String>,          // INBOUNDS
}
```

#### 1.3 Система логирования (`src/utils/logging.rs`)

**Задачи:**

-   ✅ Настройка tracing с цветным выводом (замена `logger.py`)
-   ✅ Поддержка DEBUG режима
-   ✅ Форматирование идентичное Python версии

### 🔐 Этап 2: SSL и безопасность (1-2 недели)

#### 2.1 Генерация сертификатов (`src/ssl/certificate.rs`)

**Задачи:**

-   ✅ Генерация SSL сертификатов (замена `certificate.py`)
-   ✅ Использование OpenSSL для совместимости
-   ✅ Идентичные параметры сертификата (CN="Gozargah", RSA 4096, SHA512)

**Требования совместимости:**

```rust
// Функция должна возвращать идентичный формат:
pub fn generate_certificate() -> Result<(String, String), Error> {
    // cert_pem, key_pem - идентичные Python версии
}
```

#### 2.2 Аутентификация клиентов (`src/ssl/auth.rs`)

**Задачи:**

-   ✅ Middleware для проверки клиентских сертификатов
-   ✅ SSL/TLS termination с mutual authentication
-   ✅ Совместимость с существующими клиентами

### ⚙️ Этап 3: Xray интеграция (2-3 недели)

#### 3.1 Управление процессами (`src/xray/process.rs`)

**Задачи:**

-   ✅ Async subprocess management (замена части `xray.py`)
-   ✅ Запуск/остановка/перезапуск Xray
-   ✅ Мониторинг состояния процесса
-   ✅ Обработка сигналов и graceful shutdown

**Требования совместимости:**

```rust
pub struct XrayCore {
    // Идентичные методы Python версии:
    // start(), stop(), restart(), get_version()
    // Свойство started должно работать идентично
}
```

#### 3.2 Конфигурация Xray (`src/xray/config.rs`)

**Задачи:**

-   ✅ Парсинг и валидация JSON конфигурации (замена `XRayConfig` из `xray.py`)
-   ✅ Автоматическое добавление API inbound
-   ✅ Фильтрация inbounds по INBOUNDS переменной
-   ✅ Настройка routing rules

**КРИТИЧНО**: Логика обработки конфигурации должна быть **идентична** Python версии.

#### 3.3 Потоковая передача логов (`src/xray/logs.rs`)

**Задачи:**

-   ✅ Буферизация логов Xray (замена логики из `xray.py`)
-   ✅ Real-time streaming через WebSocket
-   ✅ Поддержка множественных подписчиков
-   ✅ Контекстные менеджеры для временных буферов

### 🌐 Этап 4: REST API (2-3 недели)

#### 4.1 Основные handlers (`src/api/handlers.rs`)

**Задачи:**

-   ✅ Реализация ВСЕХ endpoints из `rest_service.py`:
    -   `POST /` - base info
    -   `POST /ping` - ping with session validation
    -   `POST /connect` - establish connection
    -   `POST /disconnect` - disconnect
    -   `POST /start` - start Xray with config
    -   `POST /stop` - stop Xray
    -   `POST /restart` - restart Xray with new config

**КРИТИЧНО**: Request/Response форматы должны быть **идентичными** Python версии.

**Пример совместимости:**

```rust
// POST /connect response должен быть идентичен:
{
    "connected": true,
    "started": false,
    "core_version": "1.8.4",
    "session_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

#### 4.2 WebSocket для логов (`src/api/websocket.rs`)

**Задачи:**

-   ✅ WebSocket endpoint `/logs` (замена метода `logs` из `rest_service.py`)
-   ✅ Валидация session_id через query parameters
-   ✅ Поддержка interval параметра
-   ✅ Идентичная обработка ошибок и WebSocket close codes

#### 4.3 Session management (`src/session/manager.rs`)

**Задачи:**

-   ✅ UUID session tracking (замена логики из `rest_service.py`)
-   ✅ Client IP tracking
-   ✅ Session validation middleware
-   ✅ Automatic cleanup при disconnect

#### 4.4 Error handling и validation (`src/api/middleware.rs`)

**Задачи:**

-   ✅ Обработка ошибок валидации (идентично FastAPI)
-   ✅ HTTP status codes совместимые с Python версией
-   ✅ JSON error responses в том же формате

### 🧪 Этап 5: Тестирование (1-2 недели)

#### 5.1 Unit тесты

**Задачи:**

-   ✅ Тесты для каждого модуля
-   ✅ Mock Xray процессов для тестирования
-   ✅ Тесты SSL certificate generation
-   ✅ Тесты конфигурации и валидации

#### 5.2 Integration тесты

**Задачи:**

-   ✅ End-to-end тесты REST API
-   ✅ WebSocket communication тесты
-   ✅ SSL authentication тесты
-   ✅ Совместимость с существующими клиентами

#### 5.3 Performance тесты

**Задачи:**

-   ✅ Benchmarking против Python версии
-   ✅ Memory usage profiling
-   ✅ Concurrent connections testing
-   ✅ Load testing с реальными сценариями

### 🚀 Этап 6: Развертывание и документация (1 неделя)

#### 6.1 Docker конфигурация

**Задачи:**

-   ✅ Multi-stage Dockerfile для оптимального размера образа
-   ✅ Обновление docker-compose.yml
-   ✅ Совместимость с существующими volume mounts
-   ✅ Переменные окружения идентичные Python версии

**Пример Dockerfile:**

```dockerfile
# Multi-stage build для минимального размера
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/marzban-node-rust /usr/local/bin/marzban-node
# Установка Xray (идентично Python версии)
RUN curl -L https://github.com/Gozargah/Marzban-scripts/raw/master/install_latest_xray.sh | bash
CMD ["marzban-node"]
```

#### 6.2 CI/CD Pipeline

**Задачи:**

-   ✅ GitHub Actions для сборки и тестирования
-   ✅ Автоматическая сборка Docker образов
-   ✅ Cross-compilation для разных архитектур (amd64, arm64)
-   ✅ Automated testing pipeline

#### 6.3 Документация

**Задачи:**

-   ✅ README.md с инструкциями по миграции
-   ✅ API documentation (совместимость с Python версией)
-   ✅ Performance comparison benchmarks
-   ✅ Migration guide для пользователей

---

## 🎯 Критерии готовности

### Функциональная совместимость ✅

-   [ ] Все REST API endpoints работают идентично Python версии
-   [ ] WebSocket логи работают с теми же параметрами
-   [ ] SSL аутентификация совместима с существующими сертификатами
-   [ ] Xray конфигурация обрабатывается идентично
-   [ ] Переменные окружения работают как в Python версии

### Performance цели ✅

-   [ ] Потребление памяти снижено на 60%+
-   [ ] CPU usage снижен на 30%+
-   [ ] Время запуска снижено на 90%+
-   [ ] Throughput увеличен в 2+ раза
-   [ ] Docker образ уменьшен на 80%+

### Качество кода ✅

-   [ ] 80%+ покрытие тестами
-   [ ] Все integration тесты проходят
-   [ ] Performance benchmarks показывают улучшения
-   [ ] Code review пройден
-   [ ] Documentation завершена

---

## 🚨 Критические моменты

### 1. Полная совместимость API

**НЕЛЬЗЯ** изменять:

-   URL paths и HTTP methods
-   JSON request/response структуры
-   HTTP status codes
-   WebSocket протокол и параметры
-   Переменные окружения

### 2. Xray конфигурация

Логика обработки конфигурации в `src/xray/config.rs` должна **точно повторять** Python версию:

-   Добавление API inbound
-   Фильтрация по INBOUNDS
-   Routing rules
-   SSL настройки

### 3. Session management

UUID сессии и их валидация должны работать **идентично** Python версии.

### 4. Error handling

HTTP ошибки и их форматы должны **точно совпадать** с FastAPI версией.

---

## 📈 Ожидаемые результаты

### Производительность

-   **Memory**: 50-200MB → 10-40MB (-60-80%)
-   **CPU**: снижение на 30-70% при нагрузке
-   **Startup**: 2-10s → 0.1-0.5s (-90-95%)
-   **Throughput**: +200-500% concurrent connections
-   **Docker size**: 200-500MB → 20-50MB (-80-90%)

### Экономические показатели

-   **Server costs**: -40-70% при той же нагрузке
-   **Scaling**: больше пользователей на сервер
-   **Reliability**: меньше crashes и memory leaks
-   **Maintenance**: меньше runtime ошибок

### Operational benefits

-   **Single binary**: нет зависимостей Python
-   **Cross-platform**: легкая сборка для разных архитектур
-   **Memory safety**: защита от segfaults
-   **Better debugging**: structured logging и tracing

---

## 🔄 TODO для будущих версий

### gRPC Implementation (Post-MVP)

После завершения REST версии:

1. **Добавить gRPC сервис** (замена RPyC):

    - Создать `.proto` файлы на основе RPyC интерфейса
    - Реализовать gRPC сервер с Tonic
    - Поддержка streaming для логов
    - SSL/TLS authentication

2. **Обновить мастер ноду Marzban**:

    - Добавить gRPC клиент
    - Миграция с RPyC на gRPC
    - Обратная совместимость

3. **Performance optimization**:
    - HTTP/2 multiplexing
    - Binary serialization вместо JSON
    - Connection pooling

---

## 📞 Поддержка и вопросы

При возникновении вопросов во время реализации:

1. **API совместимость**: сравнивать с оригинальным `rest_service.py`
2. **Xray логика**: точно следовать `xray.py`
3. **SSL handling**: использовать OpenSSL для совместимости
4. **Performance**: профилировать и сравнивать с Python версией

**Принцип**: "Сначала совместимость, потом оптимизация"
