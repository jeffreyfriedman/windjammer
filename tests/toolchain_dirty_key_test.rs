//! Two dirty builds of one commit must not share a build identity key (stale binaries stayed "installed").

#[path = "../build_support/identity_key.rs"]
mod identity_key;
use identity_key::{dirty_digest, identity_key};

#[test]
fn different_uncommitted_changes_give_different_keys() {
    let a = identity_key("abc", true, dirty_digest("diff --git a/x b/x\n+one\n", &[]), "release");
    let b = identity_key("abc", true, dirty_digest("diff --git a/x b/x\n+two\n", &[]), "release");
    assert_ne!(a, b);
}

#[test]
fn an_untracked_source_file_changes_the_key() {
    let none = dirty_digest("", &[]);
    let one = dirty_digest("", &[("src/new.rs".to_string(), b"fn a() {}".to_vec())]);
    let other = dirty_digest("", &[("src/new.rs".to_string(), b"fn b() {}".to_vec())]);
    assert_eq!(none, 0);
    assert_ne!(one, 0);
    assert_ne!(one, other);
}

#[test]
fn the_same_state_gives_the_same_key_and_clean_trees_share_one() {
    let d = dirty_digest("diff\n+x\n", &[("a".to_string(), vec![1]), ("b".to_string(), vec![2])]);
    let e = dirty_digest("diff\n+x\n", &[("b".to_string(), vec![2]), ("a".to_string(), vec![1])]);
    assert_eq!(d, e, "file order must not matter");
    assert_eq!(identity_key("abc", false, 0, "debug"), identity_key("abc", false, dirty_digest("", &[]), "debug"));
}
