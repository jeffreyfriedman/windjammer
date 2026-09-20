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

//! Cross-crate path-dep calls must auto-borrow owned `String` into demoted `&str` formals
//! when dependency metadata has `param_ownership: Borrowed` / `emitted_rust_ref_params: true`.
//!
//! Ecosystem `apps/wj-find`: `glob_filter(pattern, paths)` and `walk_files(root)` emit E0308
//! (`expected &str, found String`) when callee metadata lives in `.wj-cache/lib.wj.meta`.

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

fn patch_dep(
    cargo_toml_path: &std::path::Path,
    dep_key: &str,
    pkg_gen: &std::path::Path,
    package: &str,
) {
    if !cargo_toml_path.exists() {
        return;
    }
    let cargo_toml = fs::read_to_string(cargo_toml_path).expect("read Cargo.toml");
    let dep_line = format!(
        "{dep_key} = {{ path = \"{}\", package = \"{package}\" }}",
        pkg_gen.display()
    );
    let mut lines: Vec<String> = cargo_toml
        .lines()
        .filter(|line| !line.trim_start().starts_with(dep_key))
        .map(str::to_string)
        .collect();
    if let Some(idx) = lines.iter().position(|l| l.trim() == "[dependencies]") {
        lines.insert(idx + 1, dep_line);
    } else {
        lines.push(String::new());
        lines.push("[dependencies]".to_string());
        lines.push(dep_line);
    }
    fs::write(cargo_toml_path, format!("{}\n", lines.join("\n"))).expect("patch Cargo.toml");
}

#[test]
fn cross_crate_demoted_str_owned_arg_must_auto_borrow() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let glob_src = tmp.path().join("glob_src");
    fs::create_dir_all(&glob_src).expect("mkdir glob_src");
    fs::write(
        glob_src.join("glob_pkg.wj"),
        r#"
use std::strings

pub fn is_match(pattern: string, path: string) -> bool {
    strings.contains(path, pattern)
}

pub fn filter(pattern: string, paths: Vec<string>) -> Vec<string> {
    let mut out = Vec::new()
    let mut i = 0
    while i < paths.len() {
        if is_match("${pattern}", "${paths[i]}") {
            out.push("${paths[i]}")
        }
        i = i + 1
    }
    out
}
"#,
    )
    .unwrap();

    let glob_gen = tmp.path().join("glob_gen");
    build_library(wj, &glob_src, &glob_gen);
    let glob_meta = glob_gen.join("metadata.json");
    assert!(glob_meta.exists(), "glob_pkg must emit metadata.json");

    let walk_src = tmp.path().join("walk_src");
    fs::create_dir_all(&walk_src).expect("mkdir walk_src");
    fs::write(
        walk_src.join("walk_pkg.wj"),
        r#"
pub fn walk_files(root: string) -> Result<Vec<string>, string> {
    Ok(Vec::new())
}
"#,
    )
    .unwrap();

    let walk_gen = tmp.path().join("walk_gen");
    build_library(wj, &walk_src, &walk_gen);
    let walk_meta = walk_gen.join("metadata.json");
    assert!(walk_meta.exists(), "walk_pkg must emit metadata.json");

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(app_src.join("domain")).expect("mkdir domain");
    fs::write(
        app_src.join("wj.toml"),
        format!(
            "[package]\nname = \"app_src\"\n\n[dependencies.glob_pkg]\npath = \"{}\"\npackage = \"glob_src\"\n\n[dependencies.walk_pkg]\npath = \"{}\"\npackage = \"walk_src\"\n",
            glob_gen.display(),
            walk_gen.display()
        ),
    )
    .unwrap();
    fs::write(
        app_src.join("domain").join("find.wj"),
        r#"
use walk_pkg::walk_files
use glob_pkg::filter as glob_filter

pub fn find_matching(pattern: string, paths: Vec<string>) -> Vec<string> {
    glob_filter(pattern, paths)
}

pub fn find_under(root: string, pattern: string) -> Result<Vec<string>, string> {
    match walk_files(root) {
        Ok(paths) => Ok(find_matching(pattern, paths)),
        Err(e) => Err(e),
    }
}
"#,
    )
    .unwrap();

    let app_gen = tmp.path().join("app_gen");
    let app_build = Command::new(wj)
        .args([
            "build",
            app_src.join("domain").join("find.wj").to_str().unwrap(),
            "--output",
            app_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("glob_pkg={}", glob_meta.display()),
            "--metadata",
            &format!("walk_pkg={}", walk_meta.display()),
        ])
        .output()
        .expect("app build");
    assert!(
        app_build.status.success(),
        "app transpile failed:\n{}",
        String::from_utf8_lossy(&app_build.stderr)
    );

    let generated = fs::read_to_string(app_gen.join("find.rs")).unwrap_or_else(|_| {
        fs::read_to_string(app_gen.join("domain").join("find.rs")).unwrap_or_default()
    });
    eprintln!("cross-crate demoted str emit:\n{generated}");

    assert!(
        !generated.contains("glob_filter(pattern,") && !generated.contains("glob_filter(pattern ,"),
        "RED: glob_filter must borrow owned pattern into demoted &str formal.\n{generated}"
    );
    assert!(
        generated.contains("glob_filter(&") || generated.contains("glob_filter(&pattern"),
        "expected &pattern for cross-crate demoted filter formal.\n{generated}"
    );
    assert!(
        !generated.contains("walk_files(root)") || generated.contains("walk_files(&"),
        "RED: walk_files must borrow owned root.\n{generated}"
    );

    let cargo_toml_path = app_gen.join("Cargo.toml");
    patch_dep(&cargo_toml_path, "glob_pkg", &glob_gen, "glob_src");
    patch_dep(&cargo_toml_path, "walk_pkg", &walk_gen, "walk_src");

    let check = Command::new("cargo")
        .current_dir(&app_gen)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cross-crate demoted &str must cargo-check.\ngenerated:\n{generated}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
