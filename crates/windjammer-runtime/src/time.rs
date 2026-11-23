//! `std::time`
//!
//! `Duration`, `Instant` and `SystemTime` are the standard library's own types, so the compiled program has the
//! same API the interpreter models (`Duration::from_millis`, `as_millis`, `as_secs_f64`, `Instant::now`,
//! `elapsed`, ...) with no optional dependency. The calendar functions (`DateTime`, `now`, `format`, ...) live in
//! `calendar.rs` behind the `datetime` feature (chrono).

pub use std::time::{Duration, Instant, SystemTime};

/// Get current instant for time measurements (monotonic; `instant_now`).
pub fn instant_now() -> Instant {
    Instant::now()
}

/// Sleep for duration (seconds)
pub fn sleep(secs: u64) {
    std::thread::sleep(Duration::from_secs(secs));
}

/// Sleep for duration (milliseconds)
pub fn sleep_millis(millis: u64) {
    std::thread::sleep(Duration::from_millis(millis));
}

/// Parse a human duration string into milliseconds (`1h30m`, `5s`, `250ms`, bare number = ms).
/// Windjammer: `std::time::parse_duration_ms`.
pub fn parse_duration_ms(text: &str) -> Result<i64, String> {
    let s = text.trim();
    if s.is_empty() {
        return Err("empty duration".to_string());
    }
    if s.chars().all(|c| c.is_ascii_digit()) {
        return s
            .parse::<i64>()
            .map_err(|_| "invalid duration number".to_string());
    }
    let mut total: i64 = 0;
    let mut i = 0;
    let bytes = s.as_bytes();
    let mut saw = false;
    while i < bytes.len() {
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return Err("expected number in duration".to_string());
        }
        let n: i64 = std::str::from_utf8(&bytes[start..i])
            .ok()
            .and_then(|t| t.parse().ok())
            .ok_or_else(|| "invalid duration number".to_string())?;
        if i >= bytes.len() {
            return Err("expected unit after number".to_string());
        }
        let unit_start = i;
        while i < bytes.len() && !bytes[i].is_ascii_digit() {
            i += 1;
        }
        let unit = std::str::from_utf8(&bytes[unit_start..i])
            .unwrap_or("")
            .to_ascii_lowercase();
        let mult = match unit.as_str() {
            "ms" => 1,
            "s" => 1000,
            "m" => 60_000,
            "h" => 3_600_000,
            "d" => 86_400_000,
            _ => return Err("unknown duration unit".to_string()),
        };
        total = total.saturating_add(n.saturating_mul(mult));
        saw = true;
    }
    if !saw {
        return Err("invalid duration".to_string());
    }
    Ok(total)
}

/// Format milliseconds as a compact duration (`5ms`, `3s`, `1h30m`).
/// Windjammer: `std::time::format_duration_ms`.
pub fn format_duration_ms(ms: i64) -> String {
    if ms <= 0 {
        return "0ms".to_string();
    }
    if ms < 1000 {
        return format!("{ms}ms");
    }
    let mut rem = ms;
    let hours = rem / 3_600_000;
    rem -= hours * 3_600_000;
    let minutes = rem / 60_000;
    rem -= minutes * 60_000;
    let seconds = rem / 1000;
    rem -= seconds * 1000;
    let mut out = String::new();
    if hours > 0 {
        out.push_str(&format!("{hours}h"));
    }
    if minutes > 0 {
        out.push_str(&format!("{minutes}m"));
    }
    if seconds > 0 {
        out.push_str(&format!("{seconds}s"));
    }
    if rem > 0 {
        out.push_str(&format!("{rem}ms"));
    }
    if out.is_empty() {
        "0ms".to_string()
    } else {
        out
    }
}

#[cfg(feature = "datetime")]
mod calendar;
#[cfg(feature = "datetime")]
pub use calendar::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_ms_compound() {
        assert_eq!(parse_duration_ms("1h30m").unwrap(), 5_400_000);
        assert_eq!(parse_duration_ms("5ms").unwrap(), 5);
        assert_eq!(parse_duration_ms("250").unwrap(), 250);
        assert!(parse_duration_ms("").is_err());
        assert!(parse_duration_ms("nope").is_err());
    }

    #[test]
    fn test_format_duration_ms_compound() {
        assert_eq!(format_duration_ms(5_400_000), "1h30m");
        assert_eq!(format_duration_ms(3000), "3s");
        assert_eq!(format_duration_ms(5), "5ms");
    }
}
