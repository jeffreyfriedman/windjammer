//! The toolchain's single gateway to the process environment (one `env::var` read, one architecture-ratchet entry).

/// The variable's value when it is set and valid unicode.
pub fn get(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// A switch: set to anything but empty or `0`.
pub fn flag(name: &str) -> bool {
    get(name).is_some_and(|v| !v.is_empty() && v != "0")
}
