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

//! P3.537: product-shaped path-dep — `wj-cookie/build` is generated Rust with
//! no `--metadata`. After `parse_cookie_header` → `HashMap`,
//! `map.get("access_token")` must stay a borrowed key (`get("access_token")`),
//! not `get(String::from("access_token"))` / `get("…".to_string())`.
//!
//! Tip p3520 (05:14): isolate + product emit `String::from` (E0308).
//! `--metadata` library isolates false-GREEN with bare `get("access_token")`.
//! P3.532 is the product gate; P3.536 is WDB-424 (other agent).

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

fn owned_get_key(rs: &str) -> bool {
    rs.contains("get(\"access_token\".to_string())")
        || rs.contains("get(String::from(\"access_token\"))")
}

#[test]
fn cookie_build_path_dep_map_get_must_not_own_key() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let cookie_src = tmp.path().join("cookie_src");
    fs::create_dir_all(&cookie_src).unwrap();
    fs::write(
        cookie_src.join("lib.wj"),
        r#"
use std::collections::HashMap

pub fn parse_cookie_header(header: string) -> Result<HashMap<string, string>, string> {
    let mut map = HashMap::new()
    if header == "" {
        return Ok(map)
    }
    map.insert("access_token", "tok")
    Ok(map)
}
"#,
    )
    .unwrap();
    // Product shape: path = packages/wj-cookie/build (generated lib.rs, no --metadata)
    let cookie_build = tmp.path().join("cookie_build");
    build_library(wj, &cookie_src, &cookie_build);
    let _ = fs::remove_file(cookie_build.join("metadata.json"));

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(app_src.join("src").join("domain")).unwrap();
    fs::write(
        app_src.join("wj.toml"),
        format!(
            "[package]\nname = \"auth_cookie_abi\"\n\n[dependencies]\nwj_cookie = {{ path = \"{}\" }}\n",
            cookie_build.display()
        ),
    )
    .unwrap();
    fs::write(app_src.join("src").join("mod.wj"), "pub mod domain\n").unwrap();
    fs::write(
        app_src.join("src").join("domain").join("mod.wj"),
        "pub mod auth\n",
    )
    .unwrap();
    fs::write(
        app_src.join("src").join("domain").join("auth.wj"),
        r#"
use wj_cookie::parse_cookie_header

pub fn cookie_access_token(cookie: string) -> string {
    match parse_cookie_header(cookie) {
        Ok(map) => {
            match map.get("access_token") {
                Some(v) => "${v}",
                None => "",
            }
        },
        Err(_) => "",
    }
}
"#,
    )
    .unwrap();

    let app_gen = tmp.path().join("app_gen");
    let app_build = Command::new(wj)
        .current_dir(&app_src)
        .args([
            "build",
            "src",
            "--output",
            app_gen.to_str().unwrap(),
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("app build");
    assert!(
        app_build.status.success(),
        "P3.537 transpile failed:\n{}",
        String::from_utf8_lossy(&app_build.stderr)
    );

    let rs = fs::read_to_string(app_gen.join("domain").join("auth.rs")).unwrap_or_default();
    eprintln!("P3.537 build-path-dep cookie map.get emit:\n{rs}");
    assert!(
        !owned_get_key(&rs),
        "P3.537 RED: HashMap.get lit after cookie/build path-dep must stay borrowed:\n{rs}"
    );
    assert!(
        rs.contains("get(\"access_token\")"),
        "P3.537: expected bare borrowed get(\"access_token\"):\n{rs}"
    );
}
