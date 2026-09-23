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

//! Cross-crate `require_nonempty(field: &str, value: String)` must auto-borrow owned field.
//!
//! Ecosystem `wj-todo-cli` → `wj-validate`: E0308 `expected &str, found String` on `field`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn build_library(wj: &str, src_dir: &std::path::Path, out_dir: &std::path::Path) {
    let status = Command::new(wj)
        .args([
            "build",
            src_dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("library build");
    assert!(
        status.status.success(),
        "library build failed:\n{}",
        String::from_utf8_lossy(&status.stderr)
    );
}

#[test]
fn cross_crate_validate_field_must_auto_borrow_not_value() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let val_src = tmp.path().join("validate_src");
    fs::create_dir_all(&val_src).expect("mkdir validate_src");
    fs::write(
        val_src.join("lib.wj"),
        r#"
use std::strings

pub fn require_nonempty(field: string, value: string) -> Result<string, string> {
    if strings.len(value) == 0 {
        return Err("${field} required")
    }
    Ok(value)
}

pub fn require_max_len(field: string, value: string, max: int) -> Result<string, string> {
    if strings.len(value) > max {
        return Err("${field} too long")
    }
    Ok(value)
}
"#,
    )
    .unwrap();

    let val_gen = tmp.path().join("validate_gen");
    build_library(wj, &val_src, &val_gen);
    let val_meta = val_gen.join("metadata.json");
    assert!(val_meta.exists(), "validate must emit metadata.json");

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(app_src.join("src").join("domain")).expect("mkdir src/domain");
    fs::write(
        app_src.join("wj.toml"),
        format!(
            "[package]\nname = \"todo_app\"\n\n[dependencies.validate_pkg]\npath = \"{}\"\npackage = \"validate_src\"\n",
            val_gen.display()
        ),
    )
    .unwrap();
    fs::write(
        app_src.join("src").join("mod.wj"),
        "pub mod domain\n",
    )
    .unwrap();
    fs::write(
        app_src.join("src").join("domain").join("mod.wj"),
        "pub mod todo\n",
    )
    .unwrap();
    fs::write(
        app_src.join("src").join("domain").join("todo.wj"),
        r#"
use validate_pkg::require_nonempty
use validate_pkg::require_max_len

fn own(value: string) -> string {
    value
}

fn check_nonempty(field: string, value: string) -> Result<string, string> {
    let field = own(field)
    let value = own(value)
    require_nonempty(field, value)
}

fn check_title(field: string, value: string, max: int) -> Result<string, string> {
    match check_nonempty(field, value) {
        Ok(v) => require_max_len(field, v, max),
        Err(e) => Err(e),
    }
}

pub fn add_title(title: string) -> Result<string, string> {
    check_title("title", title, 200)
}
"#,
    )
    .unwrap();

    let app_gen = tmp.path().join("app_gen");
    // Multipass `src/` (wj-todo-cli) — single-file + `--metadata` already greens.
    let app_build = Command::new(wj)
        .args([
            "build",
            app_src.join("src").to_str().unwrap(),
            "--output",
            app_gen.to_str().unwrap(),
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("app build");
    assert!(
        app_build.status.success(),
        "app transpile failed:\n{}",
        String::from_utf8_lossy(&app_build.stderr)
    );

    let generated = fs::read_to_string(app_gen.join("domain").join("todo.rs")).unwrap_or_else(|_| {
        fs::read_to_string(app_gen.join("todo.rs")).unwrap_or_default()
    });
    eprintln!("validate auto-borrow emit:\n{generated}");

    // Patch dep for cargo check
    let cargo_toml_path = app_gen.join("Cargo.toml");
    if cargo_toml_path.exists() {
        let cargo_toml = fs::read_to_string(&cargo_toml_path).unwrap();
        let dep_line = format!(
            "validate_pkg = {{ path = \"{}\", package = \"validate_src\" }}",
            val_gen.display()
        );
        let mut lines: Vec<String> = cargo_toml
            .lines()
            .filter(|line| !line.trim_start().starts_with("validate_pkg"))
            .map(str::to_string)
            .collect();
        if let Some(idx) = lines.iter().position(|l| l.trim() == "[dependencies]") {
            lines.insert(idx + 1, dep_line);
        } else {
            lines.push("[dependencies]".to_string());
            lines.push(dep_line);
        }
        fs::write(&cargo_toml_path, format!("{}\n", lines.join("\n"))).unwrap();
    }

    let check = Command::new("cargo")
        .current_dir(&app_gen)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        generated.contains("require_nonempty(&field, value)")
            || generated.contains("require_nonempty(& field, value)"),
        "field auto-borrow + owned value:\n{generated}"
    );
    assert!(
        !generated.contains("require_nonempty(&field, &value)")
            && !generated.contains("require_nonempty(& field, & value)"),
        "must not over-borrow value:\n{generated}"
    );

    assert!(
        check.status.success(),
        "cross-crate validate field must auto-borrow into &str.\ngenerated:\n{generated}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
