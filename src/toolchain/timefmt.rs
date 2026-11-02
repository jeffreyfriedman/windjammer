//! Unix-seconds <-> civil-date helpers (UTC), dependency free.

/// Days since 1970-01-01 for a proleptic Gregorian date.
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// (year, month, day) for days since 1970-01-01.
pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `YYYY-MM-DD` (UTC).
pub fn iso_date(ts: i64) -> String {
    let (y, m, d) = civil_from_days(ts.div_euclid(86400));
    format!("{y:04}-{m:02}-{d:02}")
}

/// `YYYY-MM-DDTHH:MM:SSZ` (UTC).
pub fn iso_datetime(ts: i64) -> String {
    let s = ts.rem_euclid(86400);
    format!("{}T{:02}:{:02}:{:02}Z", iso_date(ts), s / 3600, s % 3600 / 60, s % 60)
}

/// `YYYYMMDDTHHMMSSZ`, for file names.
pub fn compact_datetime(ts: i64) -> String {
    iso_datetime(ts).replace(['-', ':'], "")
}

/// Midnight UTC of a `YYYY-MM-DD` date.
pub fn parse_iso_date(s: &str) -> Option<i64> {
    let mut it = s.splitn(3, '-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: i64 = it.next()?.parse().ok()?;
    let d: i64 = it.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, m, d) * 86400)
}

/// The clock. `WJ_TOOLCHAIN_NOW` (unix seconds) overrides it so tests can age a binary without waiting.
pub fn now_ts() -> i64 {
    if let Some(t) = std::env::var("WJ_TOOLCHAIN_NOW").ok().and_then(|v| v.parse().ok()) {
        return t;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dates_round_trip() {
        assert_eq!(iso_date(0), "1970-01-01");
        assert_eq!(iso_date(1_788_000_000), "2026-08-29");
        assert_eq!(iso_datetime(951_782_400 + 3661), "2000-02-29T01:01:01Z");
        assert_eq!(parse_iso_date("2026-08-29"), Some(1_787_961_600));
        for d in (-1000..30000).step_by(37) {
            let (y, m, dd) = civil_from_days(d);
            assert_eq!(days_from_civil(y, m, dd), d);
        }
    }
}
