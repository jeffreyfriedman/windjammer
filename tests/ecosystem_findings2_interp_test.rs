#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! Interpreter gates from the second ecosystem sweep: packages that live next to each other and link through
//! `path = "../pkg/build"`, module-qualified standard-library calls, `use` inside function bodies, glob imports of
//! standard modules, re-exports through `mod.wj`. Expected values are derived by hand or by an independent Python
//! implementation (skipped loudly when `python3` is missing), never by the interpreter.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn write_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (rel, body) in files {
        let p = dir.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }
    dir
}

fn run_interpreted(dir: &std::path::Path, entry: &str) -> (String, String, i32) {
    let o = Command::new(WJ).args(["run", "--interpret", entry]).current_dir(dir).output().unwrap();
    (String::from_utf8_lossy(&o.stdout).to_string(), String::from_utf8_lossy(&o.stderr).to_string(), o.status.code().unwrap_or(-1))
}

fn run_single(src: &str) -> (String, String, i32) {
    let dir = write_project(&[("prog.wj", src)]);
    run_interpreted(dir.path(), "prog.wj")
}

fn expect(src: &str, want: &str) {
    let (out, err, code) = run_single(src);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, want, "{err}");
}

fn py(code: &str) -> Option<String> {
    let o = Command::new("python3").arg("-I").arg("-c").arg(code).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).to_string())
}

const TOML: &str = "[package]\nname = \"p\"\nversion = \"0.1.0\"\nedition = \"2025\"\n";

// ------------------------------------------------------------------ (7) packages linked through `build/`

/// `path = "../pkg/build"` is the generated output of the Windjammer package `../pkg`: the interpreter loads the
/// package source (no Rust crate, no cargo), including the package's own dependencies and `crate::` paths.
#[test]
fn build_path_dependencies_are_interpreted_as_windjammer_packages() {
    let dir = write_project(&[
        ("app/wj.toml", &format!("{TOML}\n[dependencies]\nwj_a = {{ path = \"../a/build\" }}\n")),
        ("app/src/main.wj", "use wj_a::twice_plus_one\n\nfn main() {\n    println(\"${twice_plus_one(20)}\")\n}\n"),
        ("a/wj.toml", &format!("{TOML}\n[lib]\n\n[dependencies]\nwj_b = {{ path = \"../b/build\" }}\n")),
        (
            "a/src/lib.wj",
            "pub mod inner\n\nuse crate::inner::double\n\npub fn twice_plus_one(n: int) -> int {\n    wj_b::plus_one(double(n))\n}\n",
        ),
        ("a/src/inner.wj", "pub fn double(n: int) -> int {\n    n * 2\n}\n"),
        ("a/build/Cargo.toml", "[package]\nname = \"a\"\n"),
        ("b/wj.toml", &format!("{TOML}\n[lib]\n")),
        ("b/src/lib.wj", "pub fn plus_one(n: int) -> int {\n    n + 1\n}\n"),
        ("b/build/Cargo.toml", "[package]\nname = \"b\"\n"),
    ]);
    let (out, err, code) = run_interpreted(&dir.path().join("app"), "src/main.wj");
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, "41\n", "{err}");
    assert!(!err.contains("compiled") && !err.contains("cargo"), "fell back to the compiled path: {err}");
}

// ------------------------------------------------------------------ (9) `use` in function bodies

#[test]
fn use_inside_a_function_body_binds_for_the_rest_of_the_body() {
    let dir = write_project(&[
        ("wj.toml", &format!("{TOML}\n[dependencies]\n")),
        ("src/main.wj", "mod helpers\n\nfn main() {\n    use crate::helpers::triple\n    use std::strings\n    println(\"${triple(4)} ${strings.len(\"abc\")}\")\n}\n"),
        ("src/helpers.wj", "pub fn triple(n: int) -> int {\n    n * 3\n}\n"),
    ]);
    let (out, err, code) = run_interpreted(dir.path(), "src/main.wj");
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, "12 3\n", "{err}");
}

#[test]
fn body_uses_of_two_functions_do_not_leak_into_each_other() {
    let dir = write_project(&[
        ("wj.toml", &format!("{TOML}\n[dependencies]\n")),
        (
            "src/main.wj",
            "mod a\nmod b\n\nfn first() -> int {\n    use crate::a::val\n    val()\n}\n\nfn second() -> int {\n    use crate::b::val\n    val()\n}\n\nfn main() {\n    println(\"${first()} ${second()}\")\n}\n",
        ),
        ("src/a.wj", "pub fn val() -> int {\n    1\n}\n"),
        ("src/b.wj", "pub fn val() -> int {\n    2\n}\n"),
    ]);
    let (out, err, code) = run_interpreted(dir.path(), "src/main.wj");
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, "1 2\n", "{err}");
}

// ------------------------------------------------------------------ (10) re-export through `mod.wj`

/// The generated Rust declares `pub use self::child::*` in every `mod.rs`, so `crate::adapters::serve` names the
/// function defined in `adapters/http_server.wj`; the interpreter binds the same name.
#[test]
fn module_reexports_the_public_items_of_its_children() {
    let dir = write_project(&[
        ("wj.toml", &format!("{TOML}\n[dependencies]\n")),
        ("src/main.wj", "use crate::adapters::serve\n\nfn main() {\n    println(\"${serve(7)}\")\n}\n"),
        ("src/mod.wj", "pub mod adapters\n"),
        ("src/adapters/mod.wj", "pub mod http_server\n"),
        ("src/adapters/http_server.wj", "pub fn serve(port: int) -> int {\n    port + 1\n}\n\nfn hidden() -> int {\n    0\n}\n"),
    ]);
    let (out, err, code) = run_interpreted(dir.path(), "src/main.wj");
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, "8\n", "{err}");
}

#[test]
fn private_items_of_child_modules_are_not_reexported() {
    let dir = write_project(&[
        ("wj.toml", &format!("{TOML}\n[dependencies]\n")),
        ("src/main.wj", "use crate::adapters::hidden\n\nfn main() {\n    println(\"${hidden()}\")\n}\n"),
        ("src/mod.wj", "pub mod adapters\n"),
        ("src/adapters/mod.wj", "pub mod http_server\n"),
        ("src/adapters/http_server.wj", "fn hidden() -> int {\n    0\n}\n\npub fn serve() -> int {\n    1\n}\n"),
    ]);
    let (_, err, code) = run_interpreted(dir.path(), "src/main.wj");
    assert_ne!(code, 0, "a private child item must not be visible through the parent module");
    assert!(err.contains("hidden"), "{err}");
}

/// A module and a function of the same name, re-exported together (`pub use version::version`).
#[test]
fn name_that_is_both_a_module_and_a_function_resolves_by_use() {
    let dir = write_project(&[
        ("wj.toml", &format!("{TOML}\n[dependencies]\n")),
        (
            "src/main.wj",
            "use crate::domain::version\n\nfn main() {\n    println(\"${version()} ${version::major()}\")\n}\n",
        ),
        ("src/mod.wj", "pub mod domain\n"),
        ("src/domain/mod.wj", "pub mod version\n\npub use version::version\n"),
        ("src/domain/version.wj", "pub fn version() -> string {\n    \"1.2\"\n}\n\npub fn major() -> int {\n    1\n}\n"),
    ]);
    let (out, err, code) = run_interpreted(dir.path(), "src/main.wj");
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, "1.2 1\n", "{err}");
}

// ------------------------------------------------------------------ (11) `std::thread::spawn(..)` as a path

#[test]
fn std_thread_path_call_spawns_and_joins() {
    expect(
        "use std::sync::mpsc\n\nfn main() {\n    let pair = mpsc::channel()\n    let tx = pair.0\n    let rx = pair.1\n    std::thread::spawn(|| {\n        match tx.send(5) {\n            Ok(_) => {},\n            Err(_) => {},\n        }\n    })\n    match rx.recv() {\n        Ok(v) => println(\"${v}\"),\n        Err(_) => println(\"closed\"),\n    }\n}\n",
        "5\n",
    );
}

#[test]
fn glob_import_of_a_std_module_binds_its_functions() {
    expect("use std::math::*\n\nfn main() {\n    println(\"${sqrt(16.0)} ${abs_i64(-3)}\")\n}\n", "4 3\n");
}

#[test]
fn imported_std_function_shadows_the_intrinsic_of_the_same_name() {
    // `std::log::debug` writes a log line to stderr and prints nothing on stdout; the `debug(x)` intrinsic
    // would echo `[file:line] x = ...` and return x
    let (out, err, code) = run_single("use std::log::*\n\nfn main() {\n    debug(\"quiet\")\n    println(\"done\")\n}\n");
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, "done\n");
    assert!(!err.contains("quiet = "), "intrinsic debug used: {err}");
}

// ------------------------------------------------------------------ (8) module-qualified std calls

#[test]
fn mime_constants_read_through_a_module_qualified_name() {
    expect(
        "use std::mime\n\nfn main() {\n    println(mime.APPLICATION_JSON)\n    println(mime::TEXT_HTML)\n    println(\"${mime.is_application(mime.APPLICATION_JSON)}\")\n}\n",
        "application/json; charset=utf-8\ntext/html; charset=utf-8\ntrue\n",
    );
}

#[test]
fn encoding_functions_match_python() {
    let src = r#"use std::encoding

fn show(r: Result<string, string>) -> string {
    match r {
        Ok(v) => v,
        Err(e) => "err",
    }
}

fn main() {
    println(encoding.base64_encode_string("hello world"))
    println(show(encoding.base64_decode_string("aGVsbG8gd29ybGQ=")))
    println(encoding.hex_encode_string("Hi!"))
    println(show(encoding.hex_decode_string("486921")))
    println(encoding.url_encode("a b&c/~"))
    println(show(encoding.url_decode("a%20b%26c")))
    println(show(encoding.url_decode("100%")))
    println(encoding.form_stringify(encoding.form_parse("?q=hello+world&n=1&n=2")))
}
"#;
    let Some(want) = py(
        r#"
import base64, urllib.parse
print(base64.b64encode(b"hello world").decode())
print(base64.b64decode("aGVsbG8gd29ybGQ=").decode())
print(b"Hi!".hex())
print(bytes.fromhex("486921").decode())
print(urllib.parse.quote("a b&c/~", safe="-._~"))
print(urllib.parse.unquote("a%20b%26c"))
print("100%")
print(urllib.parse.urlencode(urllib.parse.parse_qsl("q=hello+world&n=1&n=2")))
"#,
    ) else {
        eprintln!("SKIPPED: python3 unavailable");
        return;
    };
    let (out, err, code) = run_single(src);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, want, "{err}");
}

#[test]
fn regex_find_all_and_escape_match_python() {
    let src = r#"use std::regex

fn main() {
    match regex.find_all("[0-9]+", "a1 b22 c333") {
        Ok(hits) => println("${hits.len()} ${hits[0]} ${hits[2]}"),
        Err(e) => println("${e}"),
    }
    println(regex.escape("a.b*c"))
    match regex.compile_with_flags("^ab", "i") {
        Ok(re) => println("${regex.is_match_compiled(re, "ABc")}"),
        Err(e) => println("${e}"),
    }
}
"#;
    let Some(want) = py(
        r#"
import re
hits = re.findall(r"[0-9]+", "a1 b22 c333")
print(len(hits), hits[0], hits[2])
print(re.escape("a.b*c"))
print("true" if re.match(r"(?i)^ab", "ABc") else "false")
"#,
    ) else {
        eprintln!("SKIPPED: python3 unavailable");
        return;
    };
    let (out, err, code) = run_single(src);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, want, "{err}");
}

#[test]
fn sync_graduation_vocabulary_moves_values_through_a_channel() {
    expect(
        "use std::sync\n\nfn main() {\n    let pair = sync.unbounded()\n    let tx = sync.send(pair.0, 4)\n    let tx2 = sync.send(tx, 9)\n    let got = sync.recv(pair.1)\n    match got.1 {\n        Some(v) => println(\"${v}\"),\n        None => println(\"closed\"),\n    }\n    let rx = got.0\n    let second = sync.recv(rx)\n    match second.1 {\n        Some(v) => println(\"${v}\"),\n        None => println(\"closed\"),\n    }\n    let _keep = tx2\n}\n",
        "4\n9\n",
    );
}

#[test]
fn path_functions_follow_the_runtime() {
    expect(
        "use std::path\n\nfn main() {\n    println(\"${path.has_extension(\"a/b.wj\", \"wj\")} ${path.is_absolute(\"/x\")} ${path.exists(\"/definitely/not/here\")}\")\n    println(\"${path.components(\"a/b/c\").len()}\")\n    match path.strip_prefix(\"/a/b/c\", \"/a\") {\n        Some(p) => println(p),\n        None => println(\"none\"),\n    }\n}\n",
        "true true false\n3\nb/c\n",
    );
}

#[test]
fn glob_match_keeps_a_star_inside_one_path_segment() {
    expect(
        "use std::path\n\nfn main() {\n    println(\"${path.glob_match(\"src/*\", \"src/main.wj\")} ${path.glob_match(\"src/*\", \"src/deep/main.wj\")} ${path.glob_match(\"src/**/lib.wj\", \"src/lib.wj\")} ${path.glob_match(\"src/**/lib.wj\", \"src/a/b/lib.wj\")} ${path.glob_match(\"*.md\", \"d/README.md\")}\")\n}\n",
        "true false true true false\n",
    );
}

/// Function-body `use` gives the same output in the walker, the resolved engine and the resolved engine without
/// the JIT (a body `use` switches the frame's module, so such a function must run in the walker, never as an island).
#[test]
fn body_use_agrees_across_engines() {
    let dir = write_project(&[
        ("wj.toml", &format!("{TOML}\n[dependencies]\n")),
        (
            "src/main.wj",
            "mod helpers\n\nfn hot(n: int) -> int {\n    use crate::helpers::triple\n    let mut t = 0\n    let mut i = 0\n    while i < n {\n        t = t + triple(i)\n        i = i + 1\n    }\n    t\n}\n\nfn main() {\n    use crate::helpers::triple\n    println(\"${triple(4)} ${hot(200)}\")\n}\n",
        ),
        ("src/helpers.wj", "pub fn triple(n: int) -> int {\n    n * 3\n}\n"),
    ]);
    for extra in [vec!["--engine", "walker"], vec!["--engine", "resolved"], vec!["--engine", "resolved", "--no-jit"]] {
        let mut args = vec!["run", "--interpret"];
        args.extend(extra.iter().copied());
        args.push("src/main.wj");
        let o = Command::new(WJ).args(&args).current_dir(dir.path()).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&o.stdout), "12 59700\n", "{extra:?}: {}", String::from_utf8_lossy(&o.stderr));
    }
}
