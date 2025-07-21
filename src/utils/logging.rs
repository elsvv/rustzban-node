use tracing::{Level, Subscriber};
use tracing_subscriber::{
    fmt::{self, format::Writer, FormatEvent, FormatFields},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Registry,
};
use std::fmt::Result as FmtResult;

/// ANSI color codes для терминала (идентично Colors из logger.py)
pub struct Colors;

impl Colors {
    pub const BLACK: &'static str = "\x1b[0;30m";
    pub const RED: &'static str = "\x1b[0;31m";
    pub const GREEN: &'static str = "\x1b[0;32m";
    pub const BROWN: &'static str = "\x1b[0;33m";
    pub const BLUE: &'static str = "\x1b[0;34m";
    pub const PURPLE: &'static str = "\x1b[0;35m";
    pub const CYAN: &'static str = "\x1b[0;36m";
    pub const LIGHT_GRAY: &'static str = "\x1b[0;37m";
    pub const DARK_GRAY: &'static str = "\x1b[1;30m";
    pub const LIGHT_RED: &'static str = "\x1b[1;31m";
    pub const LIGHT_GREEN: &'static str = "\x1b[1;32m";
    pub const YELLOW: &'static str = "\x1b[1;33m";
    pub const LIGHT_BLUE: &'static str = "\x1b[1;34m";
    pub const LIGHT_PURPLE: &'static str = "\x1b[1;35m";
    pub const LIGHT_CYAN: &'static str = "\x1b[1;36m";
    pub const LIGHT_WHITE: &'static str = "\x1b[1;37m";
    pub const BOLD: &'static str = "\x1b[1m";
    pub const FAINT: &'static str = "\x1b[2m";
    pub const ITALIC: &'static str = "\x1b[3m";
    pub const UNDERLINE: &'static str = "\x1b[4m";
    pub const BLINK: &'static str = "\x1b[5m";
    pub const NEGATIVE: &'static str = "\x1b[7m";
    pub const CROSSED: &'static str = "\x1b[9m";
    pub const END: &'static str = "\x1b[0m";
}

/// Custom formatter для tracing, идентичный LoggerFormatter из Python версии
pub struct ColoredFormatter;

impl<S, N> FormatEvent<S, N> for ColoredFormatter
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> FmtResult {
        // Получаем уровень логирования
        let level = *event.metadata().level();
        
        // Выбираем цвет в зависимости от уровня (идентично Python версии)
        let (level_color, level_name) = match level {
            Level::TRACE => (Colors::DARK_GRAY, "TRACE"),
            Level::DEBUG => (Colors::CYAN, "DEBUG"),
            Level::INFO => (Colors::BLUE, "INFO"),
            Level::WARN => (Colors::YELLOW, "WARNING"),
            Level::ERROR => (Colors::RED, "ERROR"),
        };
        
        // Проверяем поддержку цветов в терминале (как в Python версии)
        let use_colors = atty::is(atty::Stream::Stdout);
        
        if use_colors {
            // Форматируем с цветами: LEVEL_COLOR + 'LEVEL: ' + END + "message"
            write!(writer, "{}{}: {}", level_color, level_name, Colors::END)?;
        } else {
            // Без цветов
            write!(writer, "{}: ", level_name)?;
        }
        
        // Записываем сообщение
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        
        writeln!(writer)
    }
}

/// Инициализация системы логирования
/// Идентично настройке logger в Python версии
pub fn init_logging(debug: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Определяем уровень логирования (как в Python: DEBUG если debug=True, иначе INFO)
    let level = if debug { Level::DEBUG } else { Level::INFO };
    
    // Создаем filter для уровня логирования
    let env_filter = EnvFilter::from_default_env()
        .add_directive(level.into());
    
    // Настраиваем форматтер с цветным выводом
    let fmt_layer = fmt::layer()
        .event_format(ColoredFormatter);
    
    // Инициализируем tracing subscriber
    Registry::default()
        .with(env_filter)
        .with(fmt_layer)
        .init();
    
    Ok(())
}

/// Макросы для удобного логирования (аналог logger.info, logger.warning и т.д.)
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        tracing::info!($($arg)*);
    };
}

#[macro_export]
macro_rules! log_warning {
    ($($arg:tt)*) => {
        tracing::warn!($($arg)*);
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        tracing::error!($($arg)*);
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        tracing::debug!($($arg)*);
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::info;
    
    #[test]
    fn test_logging_init() {
        // Тестируем инициализацию без ошибок  
        // Может упасть если уже инициализировано - это нормально
        let _result1 = init_logging(false);
        let _result2 = init_logging(true);
        // Не проверяем результат, так как может быть уже инициализировано
    }
    
    #[test] 
    fn test_color_constants() {
        // Проверяем что цвета определены правильно
        assert_eq!(Colors::RED, "\x1b[0;31m");
        assert_eq!(Colors::BLUE, "\x1b[0;34m");
        assert_eq!(Colors::YELLOW, "\x1b[1;33m");
        assert_eq!(Colors::END, "\x1b[0m");
    }
} 