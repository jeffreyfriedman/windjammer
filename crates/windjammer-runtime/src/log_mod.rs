//! Logging utilities
//!
//! Windjammer's `std::log` module maps to these functions.

/// Log level filter (trace through error).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }

    pub fn from_str_name(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "trace" => Some(LogLevel::Trace),
            "debug" => Some(LogLevel::Debug),
            "info" => Some(LogLevel::Info),
            "warn" | "warning" => Some(LogLevel::Warn),
            "error" => Some(LogLevel::Error),
            _ => None,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Initialize logger (call once at program start)
pub fn init() {
    env_logger::init();
}

/// Initialize logger with an explicit minimum level.
pub fn init_with_level(level: LogLevel) {
    use env_logger::Env;
    env_logger::Builder::from_env(Env::default().default_filter_or(level.as_str())).init();
}

/// Log error message
pub fn error(message: &str) {
    log::error!("{}", message);
}

/// Log warning message
pub fn warn(message: &str) {
    log::warn!("{}", message);
}

/// Log info message
pub fn info(message: &str) {
    log::info!("{}", message);
}

/// Log debug message
pub fn debug(message: &str) {
    log::debug!("{}", message);
}

/// Log trace message
pub fn trace(message: &str) {
    log::trace!("{}", message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_level_from_str() {
        assert_eq!(LogLevel::from_str_name("DEBUG"), Some(LogLevel::Debug));
        assert_eq!(LogLevel::from_str_name("Warning"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::from_str_name("nope"), None);
    }

    #[test]
    fn test_logging() {
        // Just test that functions don't panic
        error("test error");
        warn("test warning");
        info("test info");
        debug("test debug");
        trace("test trace");
    }
}
