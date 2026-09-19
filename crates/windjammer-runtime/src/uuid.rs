//! UUID generation — Windjammer `std::uuid`.

/// Random UUID v4 string.
pub fn v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Alias for ecosystem packages that used `new_v4`.
pub fn new_v4() -> String {
    v4()
}

/// Unix-ms time-ordered UUID version 7 (RFC 9562) using the current UTC clock.
pub fn v7() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// UUID version 7 from explicit Unix millis + 10 random bytes (20 hex digits).
pub fn v7_from_timestamp(unix_millis: i64, random_hex: &str) -> Result<String, String> {
    let compact: String = random_hex
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if compact.len() != 20 {
        return Err("random must be 20 hex digits".to_string());
    }
    let decoded = crate::encoding::hex_decode(&compact)?;
    let rand: [u8; 10] = decoded
        .try_into()
        .map_err(|_| "random must be 20 hex digits".to_string())?;
    if unix_millis < 0 {
        return Err("unix_millis must be non-negative".to_string());
    }
    let millis = unix_millis as u64;
    Ok(uuid::Builder::from_unix_timestamp_millis(millis, &rand)
        .into_uuid()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v4_is_canonical_format() {
        let id = v4();
        assert_eq!(id.len(), 36);
        assert!(uuid::Uuid::parse_str(&id).is_ok());
    }

    #[test]
    fn v7_is_version_seven() {
        let id = v7();
        let u = uuid::Uuid::parse_str(&id).unwrap();
        assert_eq!(u.get_version_num(), 7);
    }

    #[test]
    fn v7_from_timestamp_rfc9562_vector() {
        let id = v7_from_timestamp(1645557742000, "0cc318c4dc0c0c07398f").unwrap();
        assert_eq!(id, "017f22e2-79b0-7cc3-98c4-dc0c0c07398f");
    }

    #[test]
    fn v7_from_timestamp_rejects_short_hex() {
        assert!(v7_from_timestamp(0, "abcd").is_err());
    }
}
