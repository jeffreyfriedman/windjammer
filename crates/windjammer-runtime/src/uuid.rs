//! UUID generation — Windjammer `std::uuid`.

/// Random UUID v4 string.
pub fn v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Alias for ecosystem packages that used `new_v4`.
pub fn new_v4() -> String {
    v4()
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
}
