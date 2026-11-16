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

//! A `wj.toml` dependency `path = "../retry/build"` names a package's generated output. Its declarations are the
//! package's Windjammer sources (`../retry/src`), so the app sees `Backoff`'s field types (`initial_ms: int`)
//! exactly as a source dependency would: the literal in `Backoff { initial_ms: 100 }` is typed `i64`.

use std::fs;
use std::path::Path;
use std::process::Command;

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn wj(args: &[&str], cwd: &Path, cache: &Path) {
    let out = Command::new(env!("CARGO_BIN_EXE_wj")).args(args).current_dir(cwd).env("WJ_CARGO_TARGET_ROOT", cache).output().unwrap();
    assert!(out.status.success(), "wj {args:?} failed:\n{}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn dependency_on_generated_output_reads_the_package_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let t = tmp.path();
    write(t, "retry/wj.toml", "[package]\nname = \"retry\"\nversion = \"0.1.0\"\n");
    write(t, "retry/src/lib.wj", "pub struct Backoff {\n    pub initial_ms: int,\n}\n\npub fn first(b: Backoff) -> int {\n    b.initial_ms\n}\n");
    write(t, "app/wj.toml", "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nretry = { path = \"../retry/build\" }\n");
    write(t, "app/src/main.wj", "use retry::Backoff\nuse retry::first\n\nfn main() {\n    println(\"${first(Backoff { initial_ms: 100 })}\")\n}\n");
    wj(&["build", "src", "--library", "--no-cargo", "-o", "build"], &t.join("retry"), &t.join("cache"));
    // Like a published `build/`: generated Rust only, no metadata.json.
    let _ = fs::remove_file(t.join("retry/build/metadata.json"));
    wj(&["build", "src", "--no-cargo", "-o", "../app_gen"], &t.join("app"), &t.join("cache"));
    let main_rs = fs::read_to_string(t.join("app_gen/main.rs")).unwrap();
    assert!(main_rs.contains("initial_ms: 100_i64"), "Backoff's declared field type reaches the app:\n{main_rs}");
}
