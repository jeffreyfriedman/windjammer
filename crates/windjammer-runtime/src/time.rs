//! Date and time operations
//!
//! Windjammer's `std::time` module maps to these functions.

use chrono::{DateTime, Duration, Local, Utc};

/// Get current timestamp (seconds since epoch)
pub fn now() -> i64 {
    Utc::now().timestamp()
}

/// Get current timestamp (milliseconds since epoch)
pub fn now_millis() -> i64 {
    Utc::now().timestamp_millis()
}

/// Get current date/time as string (ISO 8601)
pub fn now_string() -> String {
    Utc::now().to_rfc3339()
}

/// Get local date/time as string
pub fn now_local() -> String {
    Local::now().to_rfc3339()
}

/// Parse ISO 8601 date string  
pub fn parse(s: &str) -> Result<i64, String> {
    s.parse::<DateTime<Utc>>()
        .map(|dt| dt.timestamp())
        .map_err(|e| e.to_string())
}

/// Parse date string with custom format
pub fn parse_format(s: &str, fmt: &str) -> Result<i64, String> {
    DateTime::parse_from_str(s, fmt)
        .map(|dt| dt.timestamp())
        .map_err(|e| e.to_string())
}

/// Format timestamp as string
pub fn format(timestamp: i64, format: &str) -> String {
    DateTime::from_timestamp(timestamp, 0)
        .unwrap_or_else(Utc::now)
        .format(format)
        .to_string()
}

/// Sleep for duration (seconds)
pub fn sleep(secs: u64) {
    std::thread::sleep(std::time::Duration::from_secs(secs));
}

/// Sleep for duration (milliseconds)
pub fn sleep_millis(millis: u64) {
    std::thread::sleep(std::time::Duration::from_millis(millis));
}

/// Add seconds to timestamp
pub fn add_seconds(timestamp: i64, seconds: i64) -> i64 {
    DateTime::from_timestamp(timestamp, 0)
        .and_then(|dt| dt.checked_add_signed(Duration::seconds(seconds)))
        .map(|dt| dt.timestamp())
        .unwrap_or(timestamp)
}

/// Add days to timestamp
pub fn add_days(timestamp: i64, days: i64) -> i64 {
    DateTime::from_timestamp(timestamp, 0)
        .and_then(|dt| dt.checked_add_signed(Duration::days(days)))
        .map(|dt| dt.timestamp())
        .unwrap_or(timestamp)
}

/// Calculate duration in seconds between two timestamps
pub fn duration_secs(start: i64, end: i64) -> i64 {
    end - start
}

/// Calculate duration in milliseconds between two timestamps
pub fn duration_millis(start: i64, end: i64) -> i64 {
    (end - start) * 1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now() {
        let ts = now();
        assert!(ts > 1_600_000_000); // After 2020
    }

    #[test]
    fn test_now_string() {
        let s = now_string();
        assert!(s.contains('T')); // ISO 8601 format has T separator
                                  // RFC3339 format may have +00:00 instead of Z
        assert!(s.contains('Z') || s.contains('+'));
    }

    #[test]
    fn test_parse() {
        let result = parse("2024-01-01T00:00:00Z");
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_seconds() {
        let ts = 1_700_000_000i64;
        let new_ts = add_seconds(ts, 3600);
        assert_eq!(new_ts, ts + 3600);
    }

    #[test]
    fn test_add_days() {
        let ts = 1_700_000_000i64;
        let new_ts = add_days(ts, 1);
        assert_eq!(new_ts, ts + 86400);
    }
}
