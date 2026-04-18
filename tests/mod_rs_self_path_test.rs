//! TDD: `mod.rs` must re-export child modules with `self::`, not `super::`, when a
//! same-named top-level directory exists (E0432). See `is_child_module_of_mod_rs_dir`.

use std::fs;
use tempfile::tempdir;
use windjammer::build_project_ext;
use windjammer::CompilationTarget;

#[test]
fn test_mod_rs_uses_self_for_child_module_reexports() {
    let src = tempdir().expect("tempdir");
    fs::create_dir_all(src.path().join("user")).expect("mkdir user");
    fs::write(
        src.path().join("user/user.wj"),
        "pub struct User {\n    pub name: String\n}\n",
    )
    .expect("write user.wj");
    fs::write(
        src.path().join("user/mod.wj"),
        "pub mod user\npub use user::User\n",
    )
    .expect("write mod.wj");

    let out = tempdir().expect("out");
    // Same name as the child module at the output root — triggers false `is_directory_prefix`.
    fs::create_dir_all(out.path().join("user")).expect("decoy top-level user/");

    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");

    let mod_rs = fs::read_to_string(out.path().join("user/mod.rs")).expect("read mod.rs");
    assert!(
        mod_rs.contains("pub use self::user::User"),
        "expected `pub use self::user::User` in:\n{}",
        mod_rs
    );
    assert!(
        !mod_rs.contains("pub use super::user::User"),
        "must not emit `pub use super::user::User` in:\n{}",
        mod_rs
    );
}

#[test]
fn test_mod_rs_child_reexport_not_confused_by_top_level_dir_with_other_name() {
    let src = tempdir().expect("tempdir");
    fs::create_dir_all(src.path().join("m")).expect("mkdir m");
    fs::write(
        src.path().join("m/foo.wj"),
        "pub struct Foo {\n    pub x: i32\n}\n",
    )
    .expect("write foo.wj");
    fs::write(
        src.path().join("m/mod.wj"),
        "pub mod foo\npub use foo::Foo\n",
    )
    .expect("write mod.wj");

    let out = tempdir().expect("out");
    fs::create_dir_all(out.path().join("foo")).expect("decoy top-level foo/");

    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");

    let mod_rs = fs::read_to_string(out.path().join("m/mod.rs")).expect("read mod.rs");
    assert!(
        mod_rs.contains("pub use self::foo::Foo"),
        "expected `pub use self::foo::Foo` in:\n{}",
        mod_rs
    );
    assert!(
        !mod_rs.contains("pub use super::foo::Foo"),
        "must not emit `pub use super::foo::Foo` in:\n{}",
        mod_rs
    );
}
