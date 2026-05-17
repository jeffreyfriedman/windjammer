#[allow(unused_imports)]
use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
struct DateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
struct Duration;

impl DateTime {
#[inline]
pub fn timestamp(&self) -> i64 {
        0_i64
}
#[inline]
pub fn timestamp_millis(&self) -> i64 {
        0_i64
}
#[inline]
pub fn format(&self, _fmt: &str) -> String {
        "".to_string()
}
#[inline]
pub fn to_rfc3339(&self) -> String {
        "".to_string()
}
#[inline]
pub fn year(&self) -> i64 {
        0_i64
}
#[inline]
pub fn month(&self) -> i64 {
        0_i64
}
#[inline]
pub fn day(&self) -> i64 {
        0_i64
}
#[inline]
pub fn hour(&self) -> i64 {
        0_i64
}
#[inline]
pub fn minute(&self) -> i64 {
        0_i64
}
#[inline]
pub fn second(&self) -> i64 {
        0_i64
}
#[inline]
pub fn add_days(self, _days: i64) -> DateTime {
        self
}
#[inline]
pub fn add_hours(self, _hours: i64) -> DateTime {
        self
}
#[inline]
pub fn add_minutes(self, _minutes: i64) -> DateTime {
        self
}
#[inline]
pub fn add_seconds(self, _seconds: i64) -> DateTime {
        self
}
}

impl Duration {
#[inline]
pub fn as_seconds(&self) -> i64 {
        0_i64
}
#[inline]
pub fn as_millis(&self) -> i64 {
        0_i64
}
}

#[inline]
pub fn now() -> DateTime {
    DateTime {  }
}

#[inline]
pub fn utc_now() -> DateTime {
    DateTime {  }
}

#[inline]
pub fn parse(_s: &str, _format: &str) -> Result<DateTime, String> {
    Err("Time parsing requires chrono (auto-added)".to_string())
}

#[inline]
pub fn parse_rfc3339(_s: &str) -> Result<DateTime, String> {
    Err("Time parsing requires chrono (auto-added)".to_string())
}

#[inline]
pub fn duration_days(_days: i64) -> Duration {
    Duration {  }
}

#[inline]
pub fn duration_hours(_hours: i64) -> Duration {
    Duration {  }
}

#[inline]
pub fn duration_minutes(_minutes: i64) -> Duration {
    Duration {  }
}

#[inline]
pub fn duration_seconds(_seconds: i64) -> Duration {
    Duration {  }
}

