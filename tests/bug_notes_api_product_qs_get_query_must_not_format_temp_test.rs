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

//! P3.671: notes-api `qs_get("${query}", …)` must not hoist identity `format!("{}", query)`.
//!
//! Product already passes bare demoted `query` into `qs_get` for `pretty`/`encoding`.
//! `list_notes_for_query` still uses `"${query}"` interpolation for reuse; tip emits
//! `{ let _temp0 = format!("{}", query); qs_get(_temp0, "q") }`. Identity string
//! interpolation on an already-owned `string` must lower to move/clone, not `format!`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn notes_api_product_qs_get_query_must_not_format_temp() {
    let app = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo parent")
        .join("windjammer-ecosystem/apps/wj-notes-api");
    if !app.join("src").exists() {
        eprintln!("skip: notes-api src not at {}", app.display());
        return;
    }

    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("out");
    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .current_dir(&app)
        .args([
            "build",
            "src",
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build notes-api");
    assert!(
        build.status.success(),
        "notes-api product transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let api_rs = fs::read_to_string(out.join("domain").join("api.rs")).unwrap_or_default();
    let qs_sites: String = api_rs
        .lines()
        .filter(|l| l.contains("qs_get"))
        .collect::<Vec<_>>()
        .join("\n");
    eprintln!("product qs_get sites:\n{qs_sites}");

    assert!(
        !api_rs.contains("format!(\"{}\", query)"),
        "P3.671 RED: identity \"${{query}}\" into qs_get must not emit format!(\"{{}}\", query):\n{qs_sites}"
    );
}

#[test]
fn identity_string_interpolation_into_owned_formal_must_not_format() {
    //! Isolate: `"${s}"` where `s: string` into owned `string` formal → no format!.
    let dir = TempDir::new().expect("tempdir");
    let src = dir.path().join("main.wj");
    fs::write(
        &src,
        r#"
fn take(owned: string) -> int {
    owned.len()
}

fn twice(s: string) -> int {
    let a = take("${s}")
    let b = take("${s}")
    a + b
}

fn main() {
    let _ = twice("hi")
}
"#,
    )
    .unwrap();

    let out = dir.path().join("out");
    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build isolate");
    assert!(
        build.status.success(),
        "isolate transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("main.rs"))
        .or_else(|_| fs::read_to_string(out.join("lib.rs")))
        .unwrap_or_else(|_| {
            // Single-file may write next to output dir name
            fs::read_dir(&out)
                .ok()
                .and_then(|mut d| {
                    d.find_map(|e| {
                        let p = e.ok()?.path();
                        if p.extension().is_some_and(|e| e == "rs") {
                            fs::read_to_string(p).ok()
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_default()
        });
    eprintln!("isolate emit:\n{rs}");

    assert!(
        !rs.contains("format!(\"{}\", s)"),
        "identity \"${{s}}\" on string must not emit format!:\n{rs}"
    );
    // Owned path: fresh clone into owned formal. Demoted path: shared borrow, no format!.
    let owned_clone = rs.contains("s.clone()");
    let demoted_borrow = rs.contains("take(&s)") || rs.contains("take(s)");
    assert!(
        owned_clone || demoted_borrow,
        "expected clone into owned take or demoted borrow, got:\n{rs}"
    );
}
