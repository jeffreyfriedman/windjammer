#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! REGRESSION: library codegen emits `use super::*` plus a builder param that
//! shares a name with a free fn re-exported by the parent module's globs.
//! Under `#![deny(ambiguous_glob_imports)]` (wasm32 default profile) that fails.
//!
//! Contract: omit blanket `use super::*` (use precise sibling imports), so
//! `cargo check` with deny(ambiguous_glob_imports) succeeds.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn library_builder_param_must_not_collide_with_super_glob_exports() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();

    fs::write(
        src.join("renderer.wj"),
        r#"
pub fn mount(sel: string) -> string { sel }
"#,
    )
    .unwrap();
    fs::write(
        src.join("authfetch.wj"),
        r#"
pub struct AuthFetch {
    mount: string,
}

impl AuthFetch {
    pub fn new() -> AuthFetch {
        AuthFetch { mount: "root" }
    }
    pub fn mount(self, mount: string) -> AuthFetch {
        self.mount = mount
        self
    }
}
"#,
    )
    .unwrap();
    fs::write(
        src.join("mod.wj"),
        "pub mod renderer\npub mod authfetch\npub use renderer::*\npub use authfetch::*\n",
    )
    .unwrap();

    let out = tmp.path().join("out");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "--library",
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("wj");
    assert!(
        status.status.success(),
        "wj library build failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );

    let auth = fs::read_to_string(out.join("authfetch.rs")).unwrap_or_default();
    let has_super_star = auth.lines().any(|l| {
        let t = l.trim();
        t == "use super::*;" || t.ends_with("use super::*;")
    });

    assert!(
        !has_super_star,
        "module files under multi-glob parents must not emit blanket `use super::*` \
         (ambiguous_glob_imports). Got:\n{auth}"
    );

    fs::write(
        out.join("Cargo.toml"),
        r#"[package]
name = "mount_probe"
version = "0.1.0"
edition = "2021"
[workspace]
[lib]
path = "lib.rs"
"#,
    )
    .unwrap();
    let lib = if out.join("lib.rs").exists() {
        fs::read_to_string(out.join("lib.rs")).unwrap_or_default()
    } else if out.join("mod.rs").exists() {
        fs::read_to_string(out.join("mod.rs")).unwrap_or_default()
    } else {
        String::new()
    };
    let lib_body = if lib.is_empty() {
        "pub mod renderer;\npub mod authfetch;\npub use renderer::*;\npub use authfetch::*;\n"
            .to_string()
    } else {
        lib
    };
    fs::write(
        out.join("lib.rs"),
        format!("#![deny(ambiguous_glob_imports)]\n{lib_body}"),
    )
    .unwrap();

    let cargo = std::process::Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&out)
        .output()
        .expect("cargo check");
    assert!(
        cargo.status.success(),
        "library with mount builder must cargo-check under deny(ambiguous_glob_imports).\n\
         has_super_star={has_super_star}\n\
         stdout:\n{}\nstderr:\n{}\nauthfetch:\n{auth}",
        String::from_utf8_lossy(&cargo.stdout),
        String::from_utf8_lossy(&cargo.stderr)
    );
}
