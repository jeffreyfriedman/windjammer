//! Runtime types that are `Clone` through a hand-written impl rather than a derive.

/// `impl Clone for Name` / `impl<T> Clone for Name<T>` in the file: the type is Clone without a derive.
pub(super) fn implements_clone_by_hand(content: &str, name: &str) -> bool {
    let needle = format!("Clone for {name}");
    content.match_indices(&needle).any(|(at, _)| {
        let after = content[at + needle.len()..].chars().next();
        !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_hand_written_clone_impl_counts_and_a_longer_name_does_not() {
        let src = "pub struct Handle<T> { x: T }\nimpl<T> Clone for Handle<T> { fn clone(&self) -> Self { todo!() } }\n";
        assert!(super::implements_clone_by_hand(src, "Handle"));
        assert!(!super::implements_clone_by_hand("impl Clone for HandleSet {}", "Handle"));
        assert!(!super::implements_clone_by_hand("pub struct TcpStream { inner: S }", "TcpStream"));
    }
}
