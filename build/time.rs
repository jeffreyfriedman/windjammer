struct DateTime {
}

struct Duration {
}

impl DateTime {
#[inline]
fn timestamp(self) -> i64 {
        0
}
#[inline]
fn timestamp_millis(self) -> i64 {
        0
}
#[inline]
fn format(self, fmt: &String) -> String {
        ""
}
#[inline]
fn to_rfc3339(self) -> String {
        ""
}
#[inline]
fn year(self) -> i64 {
        0
}
#[inline]
fn month(self) -> i64 {
        0
}
#[inline]
fn day(self) -> i64 {
        0
}
#[inline]
fn hour(self) -> i64 {
        0
}
#[inline]
fn minute(self) -> i64 {
        0
}
#[inline]
fn second(self) -> i64 {
        0
}
#[inline]
fn add_days(self, days: i64) -> DateTime {
        self
}
#[inline]
fn add_hours(self, hours: i64) -> DateTime {
        self
}
#[inline]
fn add_minutes(self, minutes: i64) -> DateTime {
        self
}
#[inline]
fn add_seconds(self, seconds: i64) -> DateTime {
        self
}
}

impl Duration {
#[inline]
fn as_seconds(self) -> i64 {
        0
}
#[inline]
fn as_millis(self) -> i64 {
        0
}
}

#[inline]
fn now() -> DateTime {
    DateTime {  }
}

#[inline]
fn utc_now() -> DateTime {
    DateTime {  }
}

#[inline]
fn parse(s: &String, format: &String) -> Result<DateTime, String> {
    Err("Time parsing requires chrono (auto-added)")
}

#[inline]
fn parse_rfc3339(s: &String) -> Result<DateTime, String> {
    Err("Time parsing requires chrono (auto-added)")
}

#[inline]
fn duration_days(days: i64) -> Duration {
    Duration {  }
}

#[inline]
fn duration_hours(hours: i64) -> Duration {
    Duration {  }
}

#[inline]
fn duration_minutes(minutes: i64) -> Duration {
    Duration {  }
}

#[inline]
fn duration_seconds(seconds: i64) -> Duration {
    Duration {  }
}

