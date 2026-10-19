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

//! Regression gates distilled from dogfooding a real application.
//!
//! Every test runs one small program three ways: compiled (`wj build --no-cargo` + `rustc`),
//! interpreted (`wj run --interpret`), and against an independent, hand-derived expectation.
//! Module-layout cases (`project_*`) build a small library project and require that the generated
//! crate type-checks. Open compiler bugs are listed in `.config/known-test-failures.txt` with a
//! pointer to `docs/known-issues/dogfood-findings.md`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn interpret(src: &str) -> Result<String, String> {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("prog.wj");
    fs::write(&f, src).unwrap();
    let out = Command::new(WJ).args(["run", "--interpret"]).arg(&f).output().unwrap();
    if !out.status.success() {
        return Err(format!("interpreter failed: {}", String::from_utf8_lossy(&out.stderr)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn compile_and_run(src: &str) -> Result<String, String> {
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

/// Compiled output, interpreter output and the hand-derived expectation must all agree.
fn agree(src: &str, expect: &str) {
    let interp = interpret(src).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(interp, expect, "interpreter output differs from the expectation");
    let compiled = compile_and_run(src).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(compiled, expect, "compiled output differs from the expectation");
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

/// Build `src/mod.wj` as a library project and require the generated crate to type-check.
fn project_compiles(files: &[(&str, &str)]) {
    let dir = write_project(files);
    let out = dir.path().join("gen");
    let o = Command::new(WJ)
        .arg("build")
        .arg(dir.path().join("src/mod.wj"))
        .arg("--output")
        .arg(&out)
        .args(["--library", "--no-cargo", "--module-file"])
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "wj build failed:\n{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let rustc = Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2021", "--emit", "metadata", "-o"])
        .arg(dir.path().join("out.rmeta"))
        .arg(out.join("lib.rs"))
        .output()
        .unwrap();
    assert!(rustc.status.success(), "generated library does not compile:\n{}", String::from_utf8_lossy(&rustc.stderr));
}

/// Run the entry `main.wj` of a multi-file directory interpreted, and compiled as a directory build
/// (`wj build <dir>` + `rustc main.rs`); both must print `expect`.
fn project_runs_both_ways(files: &[(&str, &str)], expect: &str) {
    let dir = write_project(files);
    let o = Command::new(WJ)
        .args(["run", "--interpret"])
        .arg(dir.path().join("main.wj"))
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(o.status.success(), "interpreted run failed:\n{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert_eq!(String::from_utf8_lossy(&o.stdout), expect, "interpreted output differs from the expectation");

    let out = dir.path().join("gen");
    let b = Command::new(WJ).arg("build").arg(dir.path()).arg("--output").arg(&out).arg("--no-cargo").output().unwrap();
    assert!(b.status.success(), "wj build failed:\n{}{}", String::from_utf8_lossy(&b.stdout), String::from_utf8_lossy(&b.stderr));
    let bin = dir.path().join("bin");
    let rustc = Command::new("rustc").arg(out.join("main.rs")).arg("-o").arg(&bin).args(["--edition", "2021"]).output().unwrap();
    assert!(rustc.status.success(), "rustc failed:\n{}", String::from_utf8_lossy(&rustc.stderr));
    let run = Command::new(&bin).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&run.stdout), expect, "compiled output differs from the expectation");
}

// ------------------------------------------------------------------ numeric width (convergence C4)

/// P0: an untyped `let mut total = 0` accumulating `int` fields must be 64-bit. Sums above 2^31
/// must not be truncated. Owned by the numeric solver (convergence step C4): do not patch codegen.
#[test]
fn untyped_int_accumulator_over_fields_is_64_bit() {
    agree(
        r#"pub struct Line {
    pub amount: int,
}

pub fn shift_by_total(lines: Vec<Line>) -> Vec<Line> {
    let mut total = 0
    for line in &lines {
        total = total + line.amount
    }
    let mut out: Vec<Line> = vec![]
    for line in lines {
        out.push(Line { amount: line.amount - total })
    }
    out
}

fn main() {
    let out = shift_by_total(vec![Line { amount: 5000000000 }, Line { amount: 1 }])
    println(out[0].amount)
    println(out[1].amount)
}
"#,
        "-1\n-5000000000\n",
    );
}

// ------------------------------------------------------------------ parser

#[test]
fn consecutive_field_doc_comments_parse() {
    agree(
        r#"pub struct Cap {
    pub a: bool,
    /// First line of the doc.
    /// Second line of the doc.
    pub limit: int,
}

fn main() {
    let c = Cap { a: true, limit: 3 }
    println(c.limit)
}
"#,
        "3\n",
    );
}

// ------------------------------------------------------------------ interpreter parity

#[test]
fn interpreter_bodiless_mod_loads_sibling_file() {
    project_runs_both_ways(
        &[
            ("money.wj", "pub fn double(x: int) -> int { x * 2 }\n"),
            ("main.wj", "mod money\nuse money::double\n\nfn main() { println(double(4)) }\n"),
        ],
        "8\n",
    );
}

#[test]
fn interpreter_bodiless_mod_with_crate_paths() {
    project_runs_both_ways(
        &[
            ("util.wj", "pub fn triple(x: int) -> int { x * 3 }\n"),
            ("helper.wj", "use crate::util::triple\n\npub fn nine(x: int) -> int { triple(triple(x)) }\n"),
            ("main.wj", "mod util\nmod helper\nuse helper::nine\n\nfn main() { println(nine(1)) }\n"),
        ],
        "9\n",
    );
}

#[test]
fn interpreter_module_and_function_with_same_name() {
    project_runs_both_ways(
        &[
            ("secret.wj", "pub fn secret() -> string { \"s3\" }\n"),
            ("main.wj", "mod secret\nuse secret::secret\n\nfn main() { println(secret()) }\n"),
        ],
        "s3\n",
    );
}

// ------------------------------------------------------------------ string constants (ownership)

#[test]
fn const_string_in_if_else_join_with_parameter() {
    agree(
        r#"pub const DEFAULT_CODE: string = "1000"

pub struct Entry {
    pub code: string,
    pub amount: int,
}

pub fn entry(code: string, amount: int) -> Entry {
    Entry { code: code, amount: amount }
}

pub fn pick(code: string, amount: int) -> Vec<Entry> {
    let chosen = if code.is_empty() { DEFAULT_CODE } else { code }
    vec![entry(chosen, amount)]
}

fn main() {
    println(pick("", 1)[0].code)
    println(pick("2000", 2)[0].code)
}
"#,
        "1000\n2000\n",
    );
}

#[test]
fn const_string_let_mut_later_assigned_owned_string() {
    agree(
        r#"pub const DEFAULT_VERSION: string = "1.0"

pub fn resolve(requested: string) -> string {
    let mut resolved = DEFAULT_VERSION
    if requested.len() > 0 {
        resolved = requested + "-x"
    }
    "v" + resolved
}

fn main() {
    println(resolve(""))
    println(resolve("2.0"))
}
"#,
        "v1.0\nv2.0-x\n",
    );
}

#[test]
fn const_string_passed_where_owned_string_is_required() {
    agree(
        r#"pub const KIND: string = "k"

pub fn tag(kind: string, n: int) -> string { kind + "-" + "x" }
pub fn pass(kind: string) -> string { kind }

fn main() {
    let v: Vec<string> = vec![KIND, KIND]
    println(tag(KIND, 1))
    println(pass(KIND))
    println(v.len())
}
"#,
        "k-x\nk\n2\n",
    );
}

// ------------------------------------------------------------------ ownership (convergence)

#[test]
fn value_moved_into_vec_macro_then_used_again_is_cloned() {
    agree(
        r#"pub fn query(args: Vec<string>) -> int { args.len() }

pub fn find(id: string) -> string {
    let n = query(vec![id])
    let got_id = id
    if n > 0 { got_id } else { "none" }
}

fn main() { println(find("a")) }
"#,
        "a\n",
    );
}

#[test]
fn concat_temp_passed_to_read_only_string_formal() {
    agree(
        r#"pub fn describe(scope: string, who: string) -> string {
    scope + "|" + who
}

pub struct Event {
    pub who: string,
}

fn main() {
    let e = Event { who: "ann" }
    let a = "x"
    println(describe(a + "", e.who + ""))
}
"#,
        "x|ann\n",
    );
}

#[test]
fn loop_var_field_move_then_second_loop_over_same_collection() {
    agree(
        r#"pub struct Item {
    pub code: string,
}

pub fn first_then_count(items: Vec<Item>) -> string {
    let mut next_code = ""
    for item in items {
        next_code = item.code
        break
    }
    let mut n = 0
    for item in items {
        n = n + item.code.len()
    }
    next_code + ":" + n.to_string()
}

fn main() {
    println(first_then_count(vec![Item { code: "ab" }, Item { code: "c" }]))
}
"#,
        "ab:3\n",
    );
}

#[test]
fn range_loop_index_over_len_used_as_counter() {
    agree(
        r#"pub fn weighted(v: Vec<int>) -> int {
    let mut sum: int = 0
    for li in 0..v.len() {
        sum = sum + v[li] * (li as int)
    }
    sum
}

fn main() {
    println(weighted(vec![5, 6, 7]))
}
"#,
        "20\n",
    );
}

// ------------------------------------------------------------------ call-site borrows

#[test]
fn crate_qualified_call_through_nested_modules_borrows_str_formals() {
    project_compiles(&[
        ("src/mod.wj", "pub mod domain\npub mod adapters\n"),
        ("src/domain/mod.wj", "pub mod text\n"),
        ("src/adapters/mod.wj", "pub mod caller\n"),
        (
            "src/domain/text.wj",
            "pub struct Event {\n    pub who: string,\n    pub what: string,\n}\n\npub fn join(scope: string, who: string, what: string) -> string {\n    scope + \"|\" + who + \"|\" + what\n}\n",
        ),
        (
            "src/adapters/caller.wj",
            "use crate::domain::text::Event\n\npub fn describe(scope: string, event: Event) -> string {\n    crate::domain::text::join(scope, event.who, event.what)\n}\n",
        ),
    ]);
}

#[test]
fn crate_qualified_call_one_level_borrows_str_formals() {
    project_compiles(&[
        ("src/mod.wj", "pub mod text\npub mod caller\n"),
        (
            "src/text.wj",
            "pub struct Event {\n    pub who: string,\n    pub what: string,\n}\n\npub fn join(scope: string, who: string, what: string) -> string {\n    scope + \"|\" + who + \"|\" + what\n}\n",
        ),
        (
            "src/caller.wj",
            "use crate::text::Event\n\npub fn describe(scope: string, event: Event) -> string {\n    crate::text::join(scope, event.who, event.what)\n}\n",
        ),
    ]);
}

// ------------------------------------------------------------------ closures

#[test]
fn closure_char_param_passed_to_str_find() {
    agree(
        r#"fn first_delim(rest: string) -> int {
    match rest.find(|c: char| c == ',' || c == '}') {
        Some(i) => i as int,
        None => -1,
    }
}

fn main() {
    println("${first_delim("12,3")}")
    println("${first_delim("abc")}")
}
"#,
        "2\n-1\n",
    );
}

// ------------------------------------------------------------------ interpreter test surface

const ASSERTIONS_PROGRAM: &str = r#"use std::json
use std::test::*

struct Payload {
    name: string,
    count: int,
}

fn parse_try(body: string) -> Result<Payload, string> {
    let payload: Payload = json.parse_string(body)?
    Ok(payload)
}

fn parse_match(body: string) -> Result<Payload, string> {
    let payload: Payload = match json.parse_string(body) {
        Ok(v) => v,
        Err(e) => return Err(e),
    }
    Ok(payload)
}

@test
fn result_assertions() {
    assert_is_ok(parse_try("{\"name\":\"a\",\"count\":2}"))
    assert_is_ok(parse_match("{\"name\":\"b\",\"count\":3}"))
    assert_is_err(parse_try("nope"))
    assert_eq(parse_match("{\"name\":\"b\",\"count\":3}").unwrap().count, 3)
}

@test
fn string_and_option_assertions() {
    let msg = parse_try("nope").unwrap_err()
    assert_contains(msg, "expected")
    assert_is_some(Some(1))
    assert_is_none(None)
}
"#;

#[test]
fn interpreter_test_assertions_and_typed_json_parse() {
    let dir = write_project(&[("tests/assertions_test.wj", ASSERTIONS_PROGRAM)]);
    let o = Command::new(WJ).args(["test", "--interpret"]).arg(dir.path()).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.success(), "interpreted tests failed:\n{text}");
    assert!(text.contains("2 passed; 0 failed"), "unexpected summary:\n{text}");
}

#[test]
fn test_discovery_agrees_between_interpreted_and_compiled_listing() {
    let dir = write_project(&[("tests/assertions_test.wj", ASSERTIONS_PROGRAM)]);
    let list = |interpret: bool| {
        let mut c = Command::new(WJ);
        c.args(["test", "--list"]);
        if interpret {
            c.arg("--interpret");
        }
        let o = c.arg(dir.path()).output().unwrap();
        assert!(o.status.success());
        let s = String::from_utf8_lossy(&o.stdout).to_string();
        s.lines().skip_while(|l| !l.contains("assertions_test")).map(|l| l.to_string()).collect::<Vec<_>>()
    };
    let (a, b) = (list(true), list(false));
    assert!(a.iter().any(|l| l.contains("result_assertions")), "{a:?}");
    assert_eq!(a, b);
}

// ------------------------------------------------------------------ stdlib: argon2id

const PASSWORD_PROGRAM: &str = r#"use std::crypto

fn show(r: Result<bool, string>) {
    match r {
        Ok(b) => println(b),
        Err(e) => println(e),
    }
}

fn main() {
    let reference = "$argon2id$v=19$m=19456,t=2,p=1$MDEyMzQ1Njc4OWFiY2RlZg$nUxirfVK2I/vOT6f2ly2wSgjwZ1oqwTLCmrcpDyjicA"
    show(crypto.verify_password("hunter2", reference))
    show(crypto.verify_password("hunter3", reference))
    let h = match crypto.hash_password_argon2id("pw") {
        Ok(v) => v,
        Err(e) => e,
    }
    println(crypto.password_hash_is_argon2id(h))
    show(crypto.verify_password("pw", h))
    show(crypto.verify_password("px", h))
    let legacy = match crypto.hash_password("pw") {
        Ok(v) => v,
        Err(e) => e,
    }
    println(crypto.password_hash_is_argon2id(legacy))
    show(crypto.verify_password("pw", legacy))
}
"#;

/// The reference hash comes from the reference C implementation; the expectation is independent of
/// the Rust argon2 crate. RFC 9106 vectors are checked in the runtime crate's unit tests.
#[test]
fn interpreter_argon2id_hash_verify_and_bcrypt_migration() {
    let out = interpret(PASSWORD_PROGRAM).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(out, "true\nfalse\ntrue\ntrue\nfalse\nfalse\ntrue\n");
}

#[test]
fn compiled_argon2id_calls_the_runtime_crypto_module() {
    let dir = write_project(&[("main.wj", PASSWORD_PROGRAM)]);
    let out = dir.path().join("gen");
    let b = Command::new(WJ).arg("build").arg(dir.path().join("main.wj")).arg("--output").arg(&out).arg("--no-cargo").output().unwrap();
    assert!(b.status.success(), "{}{}", String::from_utf8_lossy(&b.stdout), String::from_utf8_lossy(&b.stderr));
    let rs = fs::read_to_string(out.join("main.rs")).unwrap();
    for needle in ["crypto::hash_password_argon2id(", "crypto::password_hash_is_argon2id(", "crypto::verify_password("] {
        assert!(rs.contains(needle), "missing `{needle}` in:\n{rs}");
    }
}
