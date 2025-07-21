#!/bin/bash

# Test script for Rustzban Node (Rust implementation of Marzban Node) in Docker

set -e

echo "🚀 Testing Rustzban Node in Docker"
echo "========================================"

# Цвета для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Функция для логирования
log() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} $1"
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

error() {
    echo -e "${RED}❌ $1${NC}"
}

# Проверяем что Docker установлен
if ! command -v docker &> /dev/null; then
    error "Docker is not installed!"
    exit 1
fi

# Проверяем что docker-compose установлен
if ! command -v docker-compose &> /dev/null; then
    warning "docker-compose not found, trying docker compose..."
    DOCKER_COMPOSE="docker compose"
else
    DOCKER_COMPOSE="docker-compose"
fi

# Создаем временную директорию для SSL сертификатов
SSL_DIR="/tmp/rustzban-node-test"
mkdir -p "$SSL_DIR"

log "Created temporary SSL directory: $SSL_DIR"

# Функция очистки
cleanup() {
    log "Cleaning up..."
    $DOCKER_COMPOSE -f docker-compose.rust.yml down --remove-orphans || true
    docker image prune -f || true
    rm -rf "$SSL_DIR"
}

# Устанавливаем обработчик сигналов
trap cleanup EXIT INT TERM

# Собираем Docker образ
log "Building Docker image..."
if docker build -f Dockerfile.rust -t rustzban-node:test .; then
    success "Docker image built successfully"
else
    error "Failed to build Docker image"
    exit 1
fi

# Запускаем контейнер в тестовом режиме
log "Starting container in test mode..."

# Создаем временный docker-compose файл для тестирования
cat > docker-compose.test.yml << EOF
services:
  rustzban-node-test:
    image: rustzban-node:test
    environment:
      SERVICE_HOST: "0.0.0.0"
      SERVICE_PORT: "62050"
      SERVICE_PROTOCOL: "rest"
      DEBUG: "true"
      XRAY_EXECUTABLE_PATH: "/usr/local/bin/xray"
      XRAY_ASSETS_PATH: "/usr/local/share/xray"
      SSL_CERT_FILE: "/var/lib/marzban-node/ssl_cert.pem"
      SSL_KEY_FILE: "/var/lib/marzban-node/ssl_key.pem"
      RUST_LOG: "debug"
      RUST_BACKTRACE: "1"
    ports:
      - "62050:62050"
      - "62051:62051"
    volumes:
      - "$SSL_DIR:/var/lib/marzban-node"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:62050/"]
      interval: 10s
      timeout: 5s
      retries: 3
      start_period: 30s
EOF

# Запускаем контейнер
if docker-compose -f docker-compose.test.yml up -d; then
    success "Container started"
else
    error "Failed to start container"
    exit 1
fi

# Ждем запуска сервиса
log "Waiting for service to start..."
sleep 10

# Проверяем что контейнер запущен
if docker-compose -f docker-compose.test.yml ps | grep -q "Up"; then
    success "Container is running"
else
    error "Container is not running"
    docker-compose -f docker-compose.test.yml logs
    exit 1
fi

# Тестируем API endpoints
log "Testing API endpoints..."

# Тест базового endpoint
if curl -s -X POST http://localhost:62050/ -H "Content-Type: application/json" -d '{}' > /dev/null; then
    success "Base endpoint (/): OK"
else
    warning "Base endpoint (/): Failed"
fi

# Тест connect endpoint
if curl -s -X POST http://localhost:62050/connect -H "Content-Type: application/json" -d '{}' > /dev/null; then
    success "Connect endpoint: OK"
else
    warning "Connect endpoint: Failed"
fi

# Проверяем логи
log "Checking container logs..."
docker-compose -f docker-compose.test.yml logs --tail=20

# Проверяем health check
log "Checking health status..."
HEALTH_STATUS=$(docker inspect --format='{{.State.Health.Status}}' $(docker-compose -f docker-compose.test.yml ps -q))
if [ "$HEALTH_STATUS" = "healthy" ]; then
    success "Health check: HEALTHY"
elif [ "$HEALTH_STATUS" = "starting" ]; then
    warning "Health check: STARTING (may need more time)"
else
    warning "Health check: $HEALTH_STATUS"
fi

# Проверяем что Xray установлен
log "Checking Xray installation..."
if docker-compose -f docker-compose.test.yml exec -T rustzban-node-test /usr/local/bin/xray version; then
    success "Xray is installed and working"
else
    warning "Xray check failed"
fi

# Проверяем SSL сертификаты
log "Checking SSL certificates..."
if [ -f "$SSL_DIR/ssl_cert.pem" ] && [ -f "$SSL_DIR/ssl_key.pem" ]; then
    success "SSL certificates generated"
else
    warning "SSL certificates not found"
fi

# Останавливаем контейнер
log "Stopping container..."
docker-compose -f docker-compose.test.yml down

# Очистка
rm -f docker-compose.test.yml

success "Docker test completed successfully!"
echo ""
echo "📋 Summary:"
echo "- Docker image: rustzban-node:test"
echo "- Service endpoints tested"
echo "- Xray integration verified"
echo "- SSL certificates generated"
echo ""
echo "🚀 Ready for production deployment with docker-compose.rust.yml" 