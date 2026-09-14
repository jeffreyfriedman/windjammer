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
    feature = "integration_tests",
))]

//! cargo-bin / cross-crate: `use owned_pkg::get as query_get` must keep Owned call-site
//! ownership even when another dependency exports a free fn named `query_get` with
//! Borrowed formals.
//!
//! Dogfood: `wj-notes-api` + `wj-querystring::get as query_get` broke after adding
//! `wj-url` (whose metadata registers Borrowed `query_get`). Product workaround:
//! alias as `qs_get` instead.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn import_alias_must_not_steal_foreign_fn_ownership() {
    let tmp = TempDir::new().expect("tempdir");

    let owned_src = tmp.path().join("owned_src");
    fs::create_dir_all(&owned_src).expect("mkdir owned_src");
    fs::write(
        owned_src.join("owned_pkg.wj"),
        r#"
pub fn get(query: string, key: string) -> Option<string> {
    if query == key {
        Some(query)
    } else {
        None
    }
}
"#,
    )
    .unwrap();

    let owned_gen = tmp.path().join("owned_gen");
    let owned_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            owned_src.to_str().unwrap(),
            "--output",
            owned_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("owned_pkg build");
    assert!(
        owned_build.status.success(),
        "owned_pkg library build failed:\n{}",
        String::from_utf8_lossy(&owned_build.stderr)
    );

    let borrowed_src = tmp.path().join("borrowed_src");
    fs::create_dir_all(&borrowed_src).expect("mkdir borrowed_src");
    // Name matches the import alias used for owned `get` — metadata collision.
    fs::write(
        borrowed_src.join("borrowed_pkg.wj"),
        r#"
pub fn query_get(query: string, key: string) -> Option<string> {
    // Read-only compares demote formals to Borrowed in library metadata.
    if query == key {
        None
    } else {
        None
    }
}
"#,
    )
    .unwrap();

    let borrowed_gen = tmp.path().join("borrowed_gen");
    let borrowed_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            borrowed_src.to_str().unwrap(),
            "--output",
            borrowed_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("borrowed_pkg build");
    assert!(
        borrowed_build.status.success(),
        "borrowed_pkg library build failed:\n{}",
        String::from_utf8_lossy(&borrowed_build.stderr)
    );

    let owned_meta = owned_gen.join("metadata.json");
    let borrowed_meta = borrowed_gen.join("metadata.json");
    assert!(owned_meta.exists(), "owned_pkg must emit metadata.json");
    assert!(borrowed_meta.exists(), "borrowed_pkg must emit metadata.json");

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(app_src.join("domain")).expect("mkdir domain");
    fs::write(
        app_src.join("wj.toml"),
        format!(
            "[package]\nname = \"app_src\"\n\n[dependencies.owned_pkg]\npath = \"{}\"\npackage = \"owned_src\"\n\n[dependencies.borrowed_pkg]\npath = \"{}\"\npackage = \"borrowed_src\"\n",
            owned_gen.display(),
            borrowed_gen.display()
        ),
    )
    .unwrap();
    fs::write(
        app_src.join("domain").join("lookup.wj"),
        r#"
use owned_pkg::get as query_get
use borrowed_pkg::query_get as borrowed_query_get

pub fn wrap(query: string) -> Option<string> {
    let query = query
    // Touch borrowed export so its metadata stays live alongside the alias.
    let _probe = borrowed_query_get("", "")
    // Must move into Owned formal of owned_pkg::get — not borrow from alias name
    // colliding with borrowed_pkg::query_get metadata.
    query_get(query, "x")
}
"#,
    )
    .unwrap();

    let app_gen = tmp.path().join("app_gen");
    let app_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            app_src.join("domain").join("lookup.wj").to_str().unwrap(),
            "--output",
            app_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!(
                "owned_pkg={},borrowed_pkg={}",
                owned_meta.display(),
                borrowed_meta.display()
            ),
        ])
        .output()
        .expect("app build");
    assert!(
        app_build.status.success(),
        "app transpile failed:\n{}",
        String::from_utf8_lossy(&app_build.stderr)
    );

    let generated = fs::read_to_string(app_gen.join("lookup.rs")).unwrap_or_else(|_| {
        fs::read_to_string(app_gen.join("domain").join("lookup.rs")).unwrap_or_default()
    });

    assert!(
        !generated.contains("query_get(&query)") && !generated.contains("query_get(&"),
        "RED: import alias ownership must follow the imported Owned fn, not a foreign fn with the same alias name.\ngenerated:\n{generated}"
    );

    let cargo_toml_path = app_gen.join("Cargo.toml");
    if cargo_toml_path.exists() {
        let cargo_toml = fs::read_to_string(&cargo_toml_path).expect("read app Cargo.toml");
        let owned_line = format!(
            "owned_pkg = {{ path = \"{}\", package = \"owned_src\" }}",
            owned_gen.display()
        );
        let borrowed_line = format!(
            "borrowed_pkg = {{ path = \"{}\", package = \"borrowed_src\" }}",
            borrowed_gen.display()
        );
        let mut lines: Vec<String> = cargo_toml
            .lines()
            .filter(|line| {
                let t = line.trim_start();
                !t.starts_with("owned_pkg") && !t.starts_with("borrowed_pkg")
            })
            .map(str::to_string)
            .collect();
        if let Some(idx) = lines.iter().position(|l| l.trim() == "[dependencies]") {
            lines.insert(idx + 1, owned_line);
            lines.insert(idx + 2, borrowed_line);
        } else {
            lines.push(String::new());
            lines.push("[dependencies]".to_string());
            lines.push(owned_line);
            lines.push(borrowed_line);
        }
        fs::write(&cargo_toml_path, format!("{}\n", lines.join("\n"))).expect("patch Cargo.toml");
    }

    let check = Command::new("cargo")
        .current_dir(&app_gen)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "RED: alias must not steal foreign Borrowed ownership.\ngenerated:\n{generated}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
