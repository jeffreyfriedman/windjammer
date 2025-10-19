//! Logging utilities
//!
//! Windjammer's `std::log` module maps to these functions.

/// Initialize logger (call once at program start)
pub fn init() {
    env_logger::init();
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
    fn test_logging() {
        // Just test that functions don't panic
        error("test error");
        warn("test warning");
        info("test info");
        debug("test debug");
        trace("test trace");
    }
}
