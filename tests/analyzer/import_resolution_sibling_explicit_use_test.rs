//! E0432: In a nested library directory, an explicit `use sibling::Type` must become
//! `use super::sibling::Type` in the generated `.rs`. A bare `use sibling::Type` is not
//! resolved as a sibling module (Rust looks for an external crate / crate root).

use std::fs;
use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_nested_sibling_explicit_use_gets_super_prefix() {
    let temp = TempDir::new().unwrap();
    let demo = temp.path().join("demo");
    fs::create_dir_all(&demo).unwrap();

    fs::write(
        demo.join("foo.wj"),
        r#"
pub struct Foo {
    pub x: i32,
}
"#,
    )
    .unwrap();

    fs::write(
        demo.join("bar.wj"),
        r#"
use foo::Foo

pub struct Bar {
    pub f: Foo,
}
"#,
    )
    .unwrap();

    let out = temp.path().join("build");
    build_project_ext(temp.path(), &out, CompilationTarget::Rust, false, true, &[])
        .expect("multipass build");

    let generated = fs::read_to_string(out.join("demo/bar.rs")).unwrap();
    assert!(
        generated.contains("use super::foo::Foo"),
        "expected `use super::foo::Foo`; bare `use foo::Foo` causes E0432. Got:\n{}",
        generated
    );
}
