//! Decoding of PostgreSQL binary cell values into the text form `std::db` rows carry.
//!
//! `sqlx` only decodes an integer into the Rust width that matches the column (`INT2` is `i16`,
//! `INT4` is `i32`, `INT8` is `i64`), and has no decoder for `NUMERIC` without a decimal crate, so the
//! runtime decodes the wire format itself. Every function is pure: tests feed recorded wire bytes.

/// Decode one non-null cell. `type_name` is the PostgreSQL type name (`INT4`, `NUMERIC`, `TEXT[]`, ...).
/// Returns `None` when the bytes do not fit the type (the caller reports that as an error).
pub fn decode_binary(type_name: &str, b: &[u8]) -> Option<String> {
    if let Some(elem) = type_name.strip_suffix("[]") {
        let _ = elem;
        return decode_array(b);
    }
    decode_scalar(type_name, b)
}

fn decode_scalar(type_name: &str, b: &[u8]) -> Option<String> {
    match type_name {
        "INT2" => Some(i16::from_be_bytes(b.try_into().ok()?).to_string()),
        "INT4" => Some(i32::from_be_bytes(b.try_into().ok()?).to_string()),
        "INT8" => Some(i64::from_be_bytes(b.try_into().ok()?).to_string()),
        "OID" => Some(u32::from_be_bytes(b.try_into().ok()?).to_string()),
        "FLOAT4" => Some(f32::from_be_bytes(b.try_into().ok()?).to_string()),
        "FLOAT8" => Some(f64::from_be_bytes(b.try_into().ok()?).to_string()),
        "BOOL" => Some((*b.first()? != 0).to_string()),
        "NUMERIC" => numeric(b),
        "UUID" => uuid(b),
        "BYTEA" => Some(format!("\\x{}", hex(b))),
        "DATE" => Some(date(i32::from_be_bytes(b.try_into().ok()?))),
        "TIME" => Some(time(i64::from_be_bytes(b.try_into().ok()?))),
        "TIMESTAMP" => Some(timestamp(i64::from_be_bytes(b.try_into().ok()?))),
        "TIMESTAMPTZ" => Some(format!("{}+00", timestamp(i64::from_be_bytes(b.try_into().ok()?)))),
        "JSONB" => {
            // first byte is the jsonb format version
            let (version, rest) = b.split_first()?;
            if *version != 1 {
                return None;
            }
            String::from_utf8(rest.to_vec()).ok()
        }
        // text-like and everything else the wire sends as UTF-8 (TEXT, VARCHAR, BPCHAR, NAME, JSON, ...)
        _ => String::from_utf8(b.to_vec()).ok(),
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn uuid(b: &[u8]) -> Option<String> {
    if b.len() != 16 {
        return None;
    }
    let h = hex(b);
    Some(format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32]))
}

/// NUMERIC wire format: ndigits i16, weight i16, sign u16, dscale i16, then ndigits base-10000 digits.
fn numeric(b: &[u8]) -> Option<String> {
    if b.len() < 8 {
        return None;
    }
    let ndigits = i16::from_be_bytes([b[0], b[1]]) as usize;
    let weight = i16::from_be_bytes([b[2], b[3]]) as i32;
    let sign = u16::from_be_bytes([b[4], b[5]]);
    let dscale = i16::from_be_bytes([b[6], b[7]]).max(0) as usize;
    if b.len() < 8 + ndigits * 2 {
        return None;
    }
    match sign {
        0xC000 => return Some("NaN".to_string()),
        0xD000 => return Some("Infinity".to_string()),
        0xF000 => return Some("-Infinity".to_string()),
        _ => {}
    }
    let digit = |i: i32| -> u32 {
        if i < 0 || i as usize >= ndigits {
            0
        } else {
            let o = 8 + i as usize * 2;
            u16::from_be_bytes([b[o], b[o + 1]]) as u32
        }
    };
    let mut out = String::new();
    if sign == 0x4000 {
        out.push('-');
    }
    if weight >= 0 {
        for i in 0..=weight {
            if i == 0 {
                out.push_str(&digit(i).to_string());
            } else {
                out.push_str(&format!("{:04}", digit(i)));
            }
        }
    } else {
        out.push('0');
    }
    if dscale > 0 {
        let mut frac = String::new();
        let mut idx = weight + 1;
        while frac.len() < dscale {
            frac.push_str(&format!("{:04}", digit(idx)));
            idx += 1;
        }
        frac.truncate(dscale);
        out.push('.');
        out.push_str(&frac);
    }
    Some(out)
}

/// Days since 1970-01-01 to (year, month, day) (proleptic Gregorian).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Days from 1970-01-01 to 2000-01-01, the PostgreSQL epoch.
const PG_EPOCH_DAYS: i64 = 10_957;

fn date(days_since_2000: i32) -> String {
    let (y, m, d) = civil_from_days(days_since_2000 as i64 + PG_EPOCH_DAYS);
    format!("{y:04}-{m:02}-{d:02}")
}

fn time(micros: i64) -> String {
    let secs = micros.div_euclid(1_000_000);
    let frac = micros.rem_euclid(1_000_000);
    let base = format!("{:02}:{:02}:{:02}", secs / 3600, (secs / 60) % 60, secs % 60);
    if frac == 0 {
        base
    } else {
        format!("{base}.{}", format!("{frac:06}").trim_end_matches('0'))
    }
}

fn timestamp(micros_since_2000: i64) -> String {
    let days = micros_since_2000.div_euclid(86_400_000_000);
    let in_day = micros_since_2000.rem_euclid(86_400_000_000);
    format!("{} {}", date(days as i32), time(in_day))
}

fn elem_type_name(oid: u32) -> &'static str {
    match oid {
        16 => "BOOL",
        17 => "BYTEA",
        20 => "INT8",
        21 => "INT2",
        23 => "INT4",
        26 => "OID",
        114 => "JSON",
        700 => "FLOAT4",
        701 => "FLOAT8",
        1082 => "DATE",
        1083 => "TIME",
        1114 => "TIMESTAMP",
        1184 => "TIMESTAMPTZ",
        1700 => "NUMERIC",
        2950 => "UUID",
        3802 => "JSONB",
        _ => "TEXT",
    }
}

/// One-dimensional array wire format -> `{a,b,NULL}` (PostgreSQL's text form).
fn decode_array(b: &[u8]) -> Option<String> {
    let i32_at = |o: usize| -> Option<i32> { Some(i32::from_be_bytes(b.get(o..o + 4)?.try_into().ok()?)) };
    let ndim = i32_at(0)?;
    let elem_oid = i32_at(8)? as u32;
    if ndim == 0 {
        return Some("{}".to_string());
    }
    if ndim != 1 {
        return None;
    }
    let len = i32_at(12)? as usize;
    let mut off = 20;
    let name = elem_type_name(elem_oid);
    let textual = name == "TEXT" || name == "JSON" || name == "JSONB" || name == "BYTEA" || name == "UUID"
        || name == "DATE" || name == "TIME" || name == "TIMESTAMP" || name == "TIMESTAMPTZ";
    let mut parts = Vec::with_capacity(len);
    for _ in 0..len {
        let n = i32_at(off)?;
        off += 4;
        if n < 0 {
            parts.push("NULL".to_string());
            continue;
        }
        let n = n as usize;
        let v = decode_scalar(name, b.get(off..off + n)?)?;
        off += n;
        let needs_quote = textual
            && (v.is_empty() || v.eq_ignore_ascii_case("null") || v.contains(|c: char| ", {}\"\\".contains(c)) || v.contains(char::is_whitespace));
        if needs_quote {
            parts.push(format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\"")));
        } else {
            parts.push(v);
        }
    }
    Some(format!("{{{}}}", parts.join(",")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(ndigits: i16, weight: i16, sign: u16, dscale: i16, digits: &[u16]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&ndigits.to_be_bytes());
        v.extend_from_slice(&weight.to_be_bytes());
        v.extend_from_slice(&sign.to_be_bytes());
        v.extend_from_slice(&dscale.to_be_bytes());
        for d in digits {
            v.extend_from_slice(&d.to_be_bytes());
        }
        v
    }

    #[test]
    fn every_integer_width_decodes() {
        assert_eq!(decode_binary("INT2", &7i16.to_be_bytes()).as_deref(), Some("7"));
        assert_eq!(decode_binary("INT4", &(-70_000i32).to_be_bytes()).as_deref(), Some("-70000"));
        assert_eq!(decode_binary("INT8", &4_294_967_296i64.to_be_bytes()).as_deref(), Some("4294967296"));
        assert_eq!(decode_binary("INT4", &[0, 1]), None, "wrong width is an error, not zero");
    }

    #[test]
    fn numeric_cases() {
        // SUM(7::bigint) -> numeric 7
        assert_eq!(decode_binary("NUMERIC", &num(1, 0, 0, 0, &[7])).as_deref(), Some("7"));
        // 1234567.89
        assert_eq!(decode_binary("NUMERIC", &num(3, 1, 0, 2, &[123, 4567, 8900])).as_deref(), Some("1234567.89"));
        // -0.05
        assert_eq!(decode_binary("NUMERIC", &num(1, -1, 0x4000, 2, &[500])).as_deref(), Some("-0.05"));
        // 100000000 (digits omitted past the last non-zero group)
        assert_eq!(decode_binary("NUMERIC", &num(1, 2, 0, 0, &[1])).as_deref(), Some("100000000"));
        // 0.0001
        assert_eq!(decode_binary("NUMERIC", &num(1, -1, 0, 4, &[1])).as_deref(), Some("0.0001"));
        // 12.50 keeps its scale
        assert_eq!(decode_binary("NUMERIC", &num(2, 0, 0, 2, &[12, 5000])).as_deref(), Some("12.50"));
        // zero
        assert_eq!(decode_binary("NUMERIC", &num(0, 0, 0, 0, &[])).as_deref(), Some("0"));
        assert_eq!(decode_binary("NUMERIC", &num(0, 0, 0xC000, 0, &[])).as_deref(), Some("NaN"));
    }

    #[test]
    fn scalars() {
        assert_eq!(decode_binary("BOOL", &[1]).as_deref(), Some("true"));
        assert_eq!(decode_binary("BOOL", &[0]).as_deref(), Some("false"));
        assert_eq!(decode_binary("FLOAT4", &1.5f32.to_be_bytes()).as_deref(), Some("1.5"));
        assert_eq!(decode_binary("FLOAT8", &(-0.25f64).to_be_bytes()).as_deref(), Some("-0.25"));
        assert_eq!(decode_binary("TEXT", b"hello").as_deref(), Some("hello"));
        assert_eq!(decode_binary("BYTEA", &[0xde, 0xad]).as_deref(), Some("\\xdead"));
        let u = [0x55, 0x0e, 0x84, 0x00, 0xe2, 0x9b, 0x41, 0xd4, 0xa7, 0x16, 0x44, 0x66, 0x55, 0x44, 0x00, 0x00];
        assert_eq!(decode_binary("UUID", &u).as_deref(), Some("550e8400-e29b-41d4-a716-446655440000"));
        assert_eq!(decode_binary("JSONB", b"\x01{\"a\":1}").as_deref(), Some("{\"a\":1}"));
        assert_eq!(decode_binary("JSON", b"[1,2]").as_deref(), Some("[1,2]"));
    }

    #[test]
    fn dates_and_times() {
        assert_eq!(decode_binary("DATE", &0i32.to_be_bytes()).as_deref(), Some("2000-01-01"));
        assert_eq!(decode_binary("DATE", &(-1i32).to_be_bytes()).as_deref(), Some("1999-12-31"));
        // 2024-02-29 is 8825 days after 2000-01-01
        assert_eq!(decode_binary("DATE", &8825i32.to_be_bytes()).as_deref(), Some("2024-02-29"));
        let us = 8825i64 * 86_400_000_000 + (13 * 3600 + 5 * 60 + 9) * 1_000_000 + 250_000;
        assert_eq!(decode_binary("TIMESTAMP", &us.to_be_bytes()).as_deref(), Some("2024-02-29 13:05:09.25"));
        assert_eq!(decode_binary("TIMESTAMPTZ", &0i64.to_be_bytes()).as_deref(), Some("2000-01-01 00:00:00+00"));
        assert_eq!(decode_binary("TIME", &((3600 + 2 * 60 + 3) * 1_000_000i64).to_be_bytes()).as_deref(), Some("01:02:03"));
    }

    fn array(elem_oid: i32, elems: &[Option<&[u8]>]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&1i32.to_be_bytes());
        v.extend_from_slice(&0i32.to_be_bytes());
        v.extend_from_slice(&elem_oid.to_be_bytes());
        v.extend_from_slice(&(elems.len() as i32).to_be_bytes());
        v.extend_from_slice(&1i32.to_be_bytes());
        for e in elems {
            match e {
                Some(bytes) => {
                    v.extend_from_slice(&(bytes.len() as i32).to_be_bytes());
                    v.extend_from_slice(bytes);
                }
                None => v.extend_from_slice(&(-1i32).to_be_bytes()),
            }
        }
        v
    }

    #[test]
    fn arrays() {
        let ints = array(23, &[Some(&1i32.to_be_bytes()), None, Some(&3i32.to_be_bytes())]);
        assert_eq!(decode_binary("INT4[]", &ints).as_deref(), Some("{1,NULL,3}"));
        let texts = array(25, &[Some(b"a"), Some(b"b c"), Some(b"")]);
        assert_eq!(decode_binary("TEXT[]", &texts).as_deref(), Some("{a,\"b c\",\"\"}"));
        let empty = [0u8; 12];
        assert_eq!(decode_binary("INT8[]", &empty).as_deref(), Some("{}"));
    }
}
