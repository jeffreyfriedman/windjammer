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

//! Generic regression gates for the second ecosystem sweep (docs/known-issues): call-site/lowering classes
//! (imported names shadow intrinsics, channel element types, owned locals from borrowed params, runtime-signature
//! argument modes, unannotated empty collections in tests, runtime/std parity).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

/// `wj build` the single file, then `rustc` it. Returns the generated Rust and the run output.
fn build_and_run(src: &str) -> Result<String, String> {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("test.wj");
    fs::write(&f, src).unwrap();
    fs::create_dir_all(dir.path().join("build")).unwrap();
    let wj = Command::new(WJ)
        .args(["build", "--target", "rust", "--no-cargo"])
        .arg(&f)
        .current_dir(dir.path())
        .output()
        .unwrap();
    if !wj.status.success() {
        return Err(format!("wj build failed: {}{}", String::from_utf8_lossy(&wj.stdout), String::from_utf8_lossy(&wj.stderr)));
    }
    let rs = dir.path().join("build/test.rs");
    let bin = dir.path().join("bin");
    let rustc = Command::new("rustc").arg(&rs).arg("-o").arg(&bin).args(["--edition", "2021"]).output().unwrap();
    if !rustc.status.success() {
        let generated = fs::read_to_string(&rs).unwrap_or_default();
        return Err(format!("rustc failed: {}\n--- generated\n{}", String::from_utf8_lossy(&rustc.stderr), generated));
    }
    let run = Command::new(&bin).output().unwrap();
    Ok(String::from_utf8_lossy(&run.stdout).to_string())
}

fn write_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (rel, body) in files {
        let p = dir.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }
    dir
}

fn expect_output(src: &str, expect: &str) {
    let got = build_and_run(src).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(got, expect);
}

/// Generate Rust only (no rustc) for shapes that need runtime crates to link.
fn generated_rust(src: &str) -> String {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("test.wj");
    fs::write(&f, src).unwrap();
    fs::create_dir_all(dir.path().join("build")).unwrap();
    let wj = Command::new(WJ)
        .args(["build", "--target", "rust", "--no-cargo"])
        .arg(&f)
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(wj.status.success(), "wj build failed: {}{}", String::from_utf8_lossy(&wj.stdout), String::from_utf8_lossy(&wj.stderr));
    fs::read_to_string(dir.path().join("build/test.rs")).unwrap()
}

fn project_build(files: &[(&str, &str)]) -> (TempDir, String) {
    let dir = write_project(files);
    let o = Command::new(WJ).args(["build", "--no-cargo", "src"]).current_dir(dir.path()).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.success(), "wj build failed: {text}");
    (dir, text)
}

const TOML: &str = "[package]\nname = \"p\"\nversion = \"0.1.0\"\nedition = \"2025\"\n\n[dependencies]\n";

// (1) An imported name shadows a language intrinsic of the same name (ADR-0014), glob imports included.
#[test]
fn glob_imported_log_debug_is_the_module_function_not_the_intrinsic() {
    let rs = generated_rust(
        "use std::log::*\n\nfn say(level: string, line: string) {\n    match level {\n        \"debug\" => debug(line),\n        _ => info(line),\n    }\n}\n\nfn main() {\n    say(\"debug\", \"hello\")\n}\n",
    );
    assert!(!rs.contains("dbg!") && !rs.contains("__wj_dbg"), "intrinsic lowering used for an imported fn:\n{rs}");
    assert!(rs.contains("debug("), "{rs}");
}

#[test]
fn glob_imported_local_module_fn_shadows_intrinsic() {
    let (dir, _) = project_build(&[
        ("wj.toml", TOML),
        ("src/lg.wj", "pub fn debug(msg: string) {\n    println(\"D:${msg}\")\n}\n"),
        ("src/main.wj", "mod lg\nuse lg::*\n\nfn main() {\n    debug(\"x\")\n}\n"),
    ]);
    let rs = fs::read_to_string(dir.path().join("build/main.rs")).unwrap();
    assert!(!rs.contains("dbg!") && !rs.contains("__wj_dbg"), "{rs}");
}

#[test]
fn unshadowed_debug_is_still_the_intrinsic() {
    let rs = generated_rust("fn main() {\n    let x = 5\n    debug(x)\n}\n");
    assert!(rs.contains("dbg!") || rs.contains("__wj_dbg"), "{rs}");
}

// (2) The receiver's element type drives the send argument; no string conversion on arithmetic.
#[test]
fn sender_send_arithmetic_on_int_channel() {
    let rs = generated_rust(
        "use std::sync::mpsc\n\nfn send_sum(a: int, b: int) {\n    let pair = mpsc::channel()\n    let tx = pair.0\n    let rx = pair.1\n    match tx.send(a + b) {\n        Ok(_) => {},\n        Err(_) => {},\n    }\n    match tx.send(a * 2) {\n        Ok(_) => {},\n        Err(_) => {},\n    }\n    match tx.send(-1) {\n        Ok(_) => {},\n        Err(_) => {},\n    }\n    let _first = rx.recv()\n}\n\nfn main() {\n    send_sum(1, 2)\n}\n",
    );
    assert!(rs.contains("send(a + b)"), "{rs}");
    assert!(rs.contains("send(a * 2"), "{rs}");
    assert!(rs.contains("send(-1"), "{rs}");
    assert!(!rs.contains("to_string()"), "string conversion added to an int channel send:\n{rs}");
}

#[test]
fn sender_send_string_channel_still_converts_literals() {
    let rs = generated_rust(
        "use std::sync::mpsc\n\nfn main() {\n    let pair = mpsc::channel()\n    let tx = pair.0\n    let rx = pair.1\n    match tx.send(\"hi\") {\n        Ok(_) => {},\n        Err(_) => {},\n    }\n    let _r = rx.recv()\n}\n",
    );
    assert!(rs.contains("send(\"hi\".to_string())"), "{rs}");
}

// (3) A mutable local initialised from a borrowed-inferred string param is owned.
#[test]
fn mut_local_from_string_param_takes_owned_strings() {
    expect_output(
        "fn trim_bang(line: string) -> string {\n    let mut body = line\n    if body.starts_with(\"!\") {\n        body = \"${body}?\"\n    }\n    body\n}\n\nfn main() {\n    println(trim_bang(\"!a\"))\n    println(trim_bang(\"b\"))\n}\n",
        "!a?\nb\n",
    );
}

#[test]
fn mut_local_from_string_param_stored_in_struct_field() {
    expect_output(
        "struct Rule {\n    pattern: string,\n    negated: bool,\n}\n\nfn parse_rule(line: string) -> Rule {\n    let mut body = line\n    let mut negated = false\n    if body.starts_with(\"!\") {\n        negated = true\n        body = body.replace(\"!\", \"\")\n    }\n    Rule { pattern: body, negated: negated }\n}\n\nfn main() {\n    let r = parse_rule(\"!a\")\n    println(\"${r.pattern} ${r.negated}\")\n}\n",
        "a true\n",
    );
}

// (4) Call-site argument modes follow the runtime signature.
#[test]
fn http_get_and_server_serve_follow_runtime_argument_modes() {
    let (dir, _) = project_build(&[
        ("wj.toml", TOML),
        (
            "src/main.wj",
            "use std::http::*\n\nstruct Request {\n    url: string,\n}\n\nfn fetch(req: Request) -> int {\n    match get(req.url) {\n        Ok(response) => response.status_code(),\n        Err(_) => 0,\n    }\n}\n\nfn run(bind_addr: string, port: int) -> Result<(), string> {\n    server_serve(bind_addr, port, |req| ServerResponse::ok(\"hi\"))\n}\n\nfn main() {\n    println(\"${fetch(Request { url: \"u\" })}\")\n}\n",
        ),
    ]);
    let rs = fs::read_to_string(dir.path().join("build/main.rs")).unwrap();
    assert!(!rs.contains("get(req.url.clone())") && !rs.contains("get(req.url)"), "owned String to &str runtime fn:\n{rs}");
    assert!(!rs.contains("server_serve(&bind_addr"), "borrow to owned String runtime fn:\n{rs}");
}

#[test]
fn http_module_qualified_get_follows_runtime_argument_mode() {
    let rs = generated_rust(
        "use std::http\n\nstruct Request {\n    url: string,\n}\n\nfn fetch(req: Request) -> int {\n    match http.get(req.url) {\n        Ok(response) => response.status_code(),\n        Err(_) => 0,\n    }\n}\n\nfn main() {\n    println(\"${fetch(Request { url: \"u\" })}\")\n}\n",
    );
    assert!(rs.contains("http::get(&req.url)"), "{rs}");
}

// (5) A never-used empty collection in a test file compiles.
#[test]
fn unused_empty_collection_local_in_test_compiles() {
    let dir = write_project(&[
        ("wj.toml", TOML),
        ("src/lib.wj", "pub fn one() -> int { 1 }\n"),
        (
            "tests/empty_test.wj",
            "use std::collections::HashMap\n\n@test\nfn test_empty_is_empty() {\n    let _empty = Vec::new()\n    let _none = HashMap::new()\n    assert(true, \"unused collection\")\n}\n",
        ),
    ]);
    let o = Command::new(WJ).arg("test").current_dir(dir.path()).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.success(), "wj test failed:\n{text}");
}

// (6) std::process::pid exists in the runtime and agrees with the interpreter.
#[test]
fn process_pid_compiled_and_interpreted_agree() {
    let (dir, _) = project_build(&[
        ("wj.toml", TOML),
        ("src/main.wj", "use std::process\n\nfn main() {\n    println(\"${process.pid() > 0}\")\n}\n"),
    ]);
    let rs = fs::read_to_string(dir.path().join("build/main.rs")).unwrap();
    assert!(rs.contains("process::pid()"), "{rs}");
    let rt = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/crates/windjammer-runtime/src/process.rs")).unwrap();
    assert!(rt.contains("pub fn pid()"), "runtime lacks pid");
    let o = Command::new(WJ).args(["run", "--interpret", "src/main.wj"]).current_dir(dir.path()).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&o.stdout), "true\n", "{}", String::from_utf8_lossy(&o.stderr));
}

// (2b) An owned handle threaded through generic and concrete wrappers is moved, never cloned: the
// wrapper's parameter stays owned, and a `mut` parameter reassigned from a call is an owned binding.
#[test]
fn owned_handle_threaded_through_generic_wrappers_is_moved() {
    expect_output(
        "pub struct Handle<T> {\n    items: Vec<T>,\n}\n\npub fn first<T>(h: Handle<T>) -> (Handle<T>, Option<T>) {\n    match h.items.len() {\n        0 => (h, None),\n        _ => (h, None),\n    }\n}\n\npub fn first_int(h: Handle<int>) -> (Handle<int>, int) {\n    let got = first(h)\n    match got.1 {\n        Some(v) => (got.0, v),\n        None => (got.0, 7),\n    }\n}\n\npub fn drain(mut h: Handle<int>) -> int {\n    let mut total = 0\n    let mut n = 0\n    while n < 3 {\n        let got = first_int(h)\n        h = got.0\n        total = total + got.1\n        n = n + 1\n    }\n    total\n}\n\nfn main() {\n    println(\"${drain(Handle { items: Vec::new() })}\")\n}\n",
        "21\n",
    );
}

#[test]
fn owned_handle_forward_does_not_require_clone() {
    let rs = generated_rust(
        "pub struct Handle<T> {\n    items: Vec<T>,\n}\n\npub fn first<T>(h: Handle<T>) -> (Handle<T>, Option<T>) {\n    (h, None)\n}\n\npub fn first_int(h: Handle<int>) -> (Handle<int>, Option<int>) {\n    first(h)\n}\n\nfn main() {\n    let r = first_int(Handle { items: Vec::new() })\n    println(\"${r.1.is_none()}\")\n}\n",
    );
    assert!(!rs.contains("first(h.clone())"), "{rs}");
    assert!(rs.contains("fn first_int(h: Handle<i64>)"), "{rs}");
}
