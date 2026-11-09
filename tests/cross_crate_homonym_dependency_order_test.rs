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

//! A test crate compiles against its library's metadata (`--metadata lib=dir`) while the
//! project's `wj.toml` also names a path dependency. When both crates export a function with
//! the same bare name, the bare key must belong to the library the test imports from: the
//! dependency's homonym is reachable only as `dep::name`. Before the fix the external roots
//! were merged in hash order, so the winner (and the emitted call) changed from run to run.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn crate_metadata(name: &str, params: &[&str], ownership: &[&str], emitted: &[bool]) -> String {
    let quote = |v: &[&str]| v.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(", ");
    let flags = emitted.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(", ");
    format!(
        r#"{{"version": "0", "structs": {{}}, "functions": {{"{name}": {{
            "params": [{p}], "return_type": "Int", "is_associated": false, "parent_type": null,
            "param_ownership": [{o}], "emitted_rust_ref_params": [{flags}]}}}}}}"#,
        p = quote(params),
        o = quote(ownership),
    )
}

#[test]
fn library_bare_key_beats_dependency_homonym_on_every_run() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");
    let vec_int = "Vec(Int)";

    // The dependency: `require_pending(all: Vec<int>, applied: Vec<int>, version: int)`, owned.
    let dep = tmp.path().join("dep_build");
    fs::create_dir_all(&dep).unwrap();
    fs::write(
        dep.join("metadata.json"),
        crate_metadata("require_pending", &[vec_int, vec_int, "Int"], &["Owned", "Owned", "Owned"], &[false, false, false]),
    )
    .unwrap();

    // The library under test: `require_pending(pending: &Vec<int>)`.
    let lib = tmp.path().join("lib_build");
    fs::create_dir_all(&lib).unwrap();
    fs::write(
        lib.join("metadata.json"),
        crate_metadata("require_pending", &["Reference(Vec(Int))"], &["Borrowed"], &[true]),
    )
    .unwrap();

    let app = tmp.path().join("app");
    fs::create_dir_all(app.join("tests")).unwrap();
    fs::write(
        app.join("wj.toml"),
        format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ndep = {{ path = \"{}\" }}\n",
            dep.display()
        ),
    )
    .unwrap();
    let test_file = app.join("tests").join("pending_test.wj");
    fs::write(
        &test_file,
        "fn test_require_pending() {\n    use crate::domain::require_pending\n    assert_eq(require_pending(Vec::new()), 0)\n}\n",
    )
    .unwrap();

    let mut outputs = Vec::new();
    for run in 0..8 {
        let out = tmp.path().join(format!("out{run}"));
        let result = Command::new(wj)
            .args(["build", test_file.to_str().unwrap(), "--output", out.to_str().unwrap(), "--no-cargo"])
            .args(["--metadata", &format!("app={}", lib.display())])
            .output()
            .expect("run wj");
        assert!(result.status.success(), "build failed:\n{}", String::from_utf8_lossy(&result.stderr));
        let rust = fs::read_to_string(out.join("pending_test.rs")).expect("generated test");
        assert!(
            rust.contains("require_pending(&Vec::new())") || rust.contains("require_pending(&vec![])"),
            "run {run}: the library's `&Vec` formal must be borrowed at the call site:\n{rust}"
        );
        outputs.push(rust);
    }
    assert!(outputs.windows(2).all(|w| w[0] == w[1]), "emission must not depend on hash order");
}
