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

#[cfg(feature = "datetime")]
mod calendar;
#[cfg(feature = "datetime")]
pub use calendar::*;
