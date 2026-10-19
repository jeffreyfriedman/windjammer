use windjammer_runtime::test::*;
#[inline]
fn slugify(s: &str) -> String {
    s.trim().to_lowercase().replace(" ", "-")
}

#[test]
fn slug_basic() {
    assert_eq!(slugify("  Hello World "), "hello-world");
}

#[test]
fn slug_differs_for_different_input() {
    assert_ne!(slugify("A"), slugify("B"));
}

