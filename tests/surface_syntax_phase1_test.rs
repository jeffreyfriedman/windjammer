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

//! ADR-0014 Phase 1: intrinsics (print/assert/panic/todo/unreachable/debug/list literal),
//! the `!` -> plain-call migration (`wj fix`), and its deprecation lints.
//!
//! * both spellings must lower to byte-identical Rust
//! * the rewriter is idempotent and behaviour preserving (differential compile + run)

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use test_utils::*;
use windjammer::intrinsics::surface_fix::{fix_source, lint_source};

fn main_with(body: &str) -> String {
    format!("fn main() {{\n    let x = 5\n    let name = \"bob\"\n    let f = 3.14159\n{body}\n}}\n")
}

fn rust_for(body: &str) -> String {
    compile_single(&main_with(body))
}

/// Build + run generated Rust with rustc; returns (stdout, stderr, exit code).
fn run_rust(code: &str) -> (String, String, i32) {
    let tmp = tempfile::tempdir().unwrap();
    let rs = tmp.path().join("prog.rs");
    fs::write(&rs, code).unwrap();
    let bin = tmp.path().join("prog");
    let out = Command::new("rustc")
        .args(["--edition=2021", "-A", "warnings", "-o"])
        .arg(&bin)
        .arg(&rs)
        .output()
        .expect("rustc");
    assert!(
        out.status.success(),
        "generated Rust failed to compile:\n{}\n--- code ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        code
    );
    let run = Command::new(&bin).output().expect("run");
    (
        String::from_utf8_lossy(&run.stdout).to_string(),
        String::from_utf8_lossy(&run.stderr).to_string(),
        run.status.code().unwrap_or(-1),
    )
}

// ---------------------------------------------------------------------------
// 1. Both spellings produce the same code
// ---------------------------------------------------------------------------

#[test]
fn macro_and_plain_spellings_generate_identical_rust() {
    let pairs: &[(&str, &str)] = &[
        (r#"println!("x = {}", x)"#, r#"println("x = ${x}")"#),
        (r#"println!("{} and {:?}", x, name)"#, r#"println("${x} and ${name:?}")"#),
        (r#"println!("{:.2}", f)"#, r#"println("${f:.2}")"#),
        (r#"print!("no newline {}", x)"#, r#"print("no newline ${x}")"#),
        (r#"println!()"#, r#"println()"#),
        (r#"println!("{{literal}} braces")"#, r#"println("{literal} braces")"#),
        (r#"eprintln!("e {}", x)"#, r#"stderr.println("e ${x}")"#),
        (r#"eprint!("e {}", x)"#, r#"stderr.print("e ${x}")"#),
        (r#"assert!(x == 5)"#, r#"assert(x == 5)"#),
        (r#"assert!(x == 5, "x was {}", x)"#, r#"assert(x == 5, "x was ${x}")"#),
        (r#"assert_eq!(x, 5)"#, r#"assert_eq(x, 5)"#),
        (r#"assert_eq!(x, 5, "same {}", x)"#, r#"assert_eq(x, 5, "same ${x}")"#),
        (r#"assert_ne!(x, 6)"#, r#"assert_ne(x, 6)"#),
        (r#"let s = format!("v={}", x)
    println(s)"#, r#"let s = "v=${x}"
    println(s)"#),
        (r#"let v = vec![1, 2, 3]
    println("${v.len()}")"#, r#"let v = [1, 2, 3]
    println("${v.len()}")"#),
        (r#"let mut v = vec![]
    v.push(1)
    println("${v.len()}")"#, r#"let mut v = []
    v.push(1)
    println("${v.len()}")"#),
    ];
    for (macro_form, plain_form) in pairs {
        let a = rust_for(&format!("    {macro_form}"));
        let b = rust_for(&format!("    {plain_form}"));
        assert_eq!(a, b, "spellings diverge:\n  {macro_form}\n  {plain_form}");
    }
}

#[test]
fn panic_todo_unreachable_debug_both_spellings() {
    let src = |panic: &str, todo: &str, unr: &str, dbg: &str| {
        format!(
            "fn f(n: int) -> int {{\n    if n > 100 {{ {panic} }}\n    if n < 0 {{ {todo} }}\n    if n == 50 {{ {unr} }}\n    {dbg}\n}}\nfn main() {{ println(\"${{f(1)}}\") }}\n"
        )
    };
    let a = compile_single(&src(r#"panic!("big {}", n)"#, "todo!()", "unreachable!()", "dbg!(n)"));
    let b = compile_single(&src(r#"panic("big ${n}")"#, "todo()", "unreachable()", "debug(n)"));
    assert_eq!(a, b);
    assert!(a.contains("panic!(\"big {}\", n)"), "{a}");
    assert!(a.contains("todo!()"), "{a}");
    assert!(a.contains("unreachable!()"), "{a}");
    assert!(a.contains("dbg!("), "{a}");
}

// ---------------------------------------------------------------------------
// 2. Plain forms: correct Rust, correct output
// ---------------------------------------------------------------------------

#[test]
fn plain_forms_compile_and_run() {
    let code = compile_single(
        r#"
struct P { x: int, label: string }

fn main() {
    let x = 5
    let name = "bob"
    let f = 3.14159
    let p = P { x: 7, label: "pt" }
    println("x = ${x} name=${name}")
    println("${f:.2}|${name:?}|${x:>4}|${x:04}")
    print("a")
    print("b\n")
    println("{literal} braces")
    println("")
    stdout.println("out ${x}")
    stdout.print("o")
    stdout.println("")
    stderr.println("err ${x}")
    stderr.print("e")
    assert(x == 5, "x was ${x}")
    assert(x == 5)
    assert_eq(x, 5)
    assert_eq(x, 5, "same ${x}")
    assert_ne(x, 6)
    let y = debug(x)
    let z = debug(name)
    println("${y} ${z} ${name} ${p.x} ${p.label}")
    let list = [1, 2, 3]
    let mut grow = [10, 20]
    grow.push(30)
    let empty: Vec<int> = []
    println("${list.len()} ${grow.len()} ${empty.len()}")
}
"#,
    );
    let (out, err, status) = run_rust(&code);
    assert_eq!(status, 0, "stderr: {err}");
    let expected = "x = 5 name=bob\n3.14|\"bob\"|   5|0005\nab\n{literal} braces\n\nout 5\no\n5 bob bob 7 pt\n3 3 0\n";
    assert_eq!(out, expected);
    assert!(err.starts_with("err 5\ne"), "stderr was: {err}");
    assert!(err.contains("[src/") || err.contains("prog.rs"), "debug prints location: {err}");
}

#[test]
fn panic_todo_unreachable_behaviour() {
    for (body, needle) in [
        (r#"panic("boom ${x}")"#, "boom 5"),
        ("todo()", "not yet implemented"),
        (r#"todo("later ${x}")"#, "later 5"),
        ("unreachable()", "unreachable"),
        (r#"assert(x == 6, "x was ${x}")"#, "x was 5"),
        ("assert_eq(x, 6)", "left"),
        (r#"assert_eq(x, 6, "ctx ${x}")"#, "ctx 5"),
    ] {
        let code = rust_for(&format!("    {body}"));
        let (_, err, status) = run_rust(&code);
        assert_ne!(status, 0, "{body} must fail");
        assert!(err.contains(needle), "{body}: stderr {err}");
    }
}

#[test]
fn debug_returns_value_without_moving_user_data() {
    let code = compile_single(
        r#"
fn main() {
    let name = "bob".to_string()
    let again = debug(name)
    println("${name} ${again}")
}
"#,
    );
    let (out, _, status) = run_rust(&code);
    assert_eq!(status, 0);
    assert_eq!(out, "bob bob\n");
}

#[test]
fn interpolation_never_moves_user_values() {
    let code = compile_single(
        r#"
fn main() {
    let name = "bob".to_string()
    let items = [1, 2, 3]
    println("${name} ${name} ${items.len()}")
    println("${name:?}")
    let again = name
    println("${again} ${items.len()}")
}
"#,
    );
    let (out, _, status) = run_rust(&code);
    assert_eq!(status, 0);
    assert_eq!(out, "bob bob 3\n\"bob\"\nbob 3\n");
}

#[test]
fn list_literal_in_struct_field_and_nested() {
    let code = compile_single(
        r#"
struct Bag { items: Vec<int>, names: Vec<string> }
fn main() {
    let b = Bag { items: [1, 2], names: ["a", "b"] }
    let grid = [[1, 2], [3, 4]]
    println("${b.items.len()} ${b.names.len()} ${grid.len()} ${grid[1][0]}")
}
"#,
    );
    let (out, _, status) = run_rust(&code);
    assert_eq!(status, 0);
    assert_eq!(out, "2 2 2 3\n");
}

// ---------------------------------------------------------------------------
// 3. Lints
// ---------------------------------------------------------------------------

#[test]
fn deprecation_findings_have_code_span_and_replacement() {
    let src = "fn main() {\n    println!(\"a {} b {:?}\", x, y)\n    let v = vec![1, 2]\n    eprintln!(\"e\")\n    dbg!(x)\n}\n";
    let findings = lint_source(src);
    assert_eq!(findings.len(), 4, "{findings:#?}");
    assert_eq!(findings[0].code, "W0008");
    assert_eq!((findings[0].line, findings[0].col), (2, 5));
    assert_eq!(
        findings[0].replacement.as_deref(),
        Some("println(\"a ${x} b ${y:?}\")")
    );
    assert_eq!(findings[1].code, "W0007");
    assert_eq!(findings[1].replacement.as_deref(), Some("[1, 2]"));
    assert_eq!(findings[2].replacement.as_deref(), Some("stderr.println(\"e\")"));
    assert_eq!(findings[3].replacement.as_deref(), Some("debug(x)"));
}

#[test]
fn parser_emits_deprecation_warnings() {
    let source = "fn main() {\n    println!(\"{}\", 1)\n}\n";
    let tokens = windjammer::lexer::Lexer::new(source).tokenize_with_locations();
    let mut parser = windjammer::parser::Parser::new_with_source(
        tokens,
        "t.wj".to_string(),
        source.to_string(),
    );
    parser.parse().unwrap();
    let w = parser
        .warnings()
        .iter()
        .find(|w| w.message.contains("W0008"))
        .expect("W0008 warning");
    assert!(w.message.contains("suggested replacement: `println(\"${1}\")`"), "{}", w.message);
    assert_eq!((w.line, w.column), (Some(2), Some(5)));
}

#[test]
fn plain_forms_produce_no_deprecation_warnings() {
    let source = "fn main() {\n    println(\"x ${1}\")\n    let v = [1, 2]\n    stderr.println(\"e\")\n    assert(true)\n}\n";
    assert!(lint_source(source).is_empty());
}

#[test]
fn error_catalog_documents_codes() {
    let catalog = windjammer::error_catalog::ErrorCatalog::new();
    assert!(catalog.get_error("W0007").is_some());
    assert!(catalog.get_error("W0008").is_some());
}

// ---------------------------------------------------------------------------
// 4. wj fix: golden + idempotence + differential behaviour
// ---------------------------------------------------------------------------

#[test]
fn fix_golden() {
    let cases: &[(&str, &str)] = &[
        (r#"println!("a {} b {:?}", x, y)"#, r#"println("a ${x} b ${y:?}")"#),
        (r#"println!("{0} {0} {1}", a, b)"#, r#"println("${a} ${a} ${b}")"#),
        (r#"println!("{name}: {v:.1}", name = n, v = 2.5)"#, r#"println("${n}: ${2.5:.1}")"#),
        (r#"println!("{{}} {{{}}}", x)"#, r#"println("{} {${x}}")"#),
        (r#"eprintln!("bad {}", x)"#, r#"stderr.println("bad ${x}")"#),
        (r#"let s = format!("{}-{}", a, b)"#, r#"let s = "${a}-${b}""#),
        (r#"let v = vec![1, 2, 3]"#, r#"let v = [1, 2, 3]"#),
        (r#"let v = vec![0; 5]"#, r#"let v = [0; 5]"#),
        (r#"let g = vec![vec![1], vec![2]]"#, r#"let g = [[1], [2]]"#),
        ("dbg!(x + 1)", "debug(x + 1)"),
        (r#"assert!(a < b, "a {} b {}", a, b)"#, r#"assert(a < b, "a ${a} b ${b}")"#),
        (r#"assert_eq!(a, b, "ctx {}", c)"#, r#"assert_eq(a, b, "ctx ${c}")"#),
        (r#"panic!("fail: {}", why)"#, r#"panic("fail: ${why}")"#),
        ("todo!()", "todo()"),
        ("unreachable!()", "unreachable()"),
        (r#"println("old {} style", x)"#, r#"println("old ${x} style")"#),
        ("println!(\n    \"a {} b {}\",\n    foo(1, \"x\"),\n    bar,\n)", "println(\"a ${foo(1, \"x\")} b ${bar}\")"),
        (r#"println!("{}", format!("[{}]", x))"#, r#"println("${("[${x}]")}")"#),
        (r#"println!("cost $ {}", x)"#, r#"println("cost $ ${x}")"#),
        (r#"println!("{}", vec![1].len())"#, r#"println("${vec![1].len()}")"#),
    ];
    for (input, expected) in cases {
        assert_eq!(fix_source(input).text, *expected, "input: {input}");
    }
}

#[test]
fn fix_leaves_untouchable_and_lookalikes_alone() {
    for s in [
        "// println!(\"{}\", x)\n",
        "let a = \"println!(1)\"\n",
        "obj.println!(1)\n",
        "println!(\"{:w$}\", x)\n",
        "foo(vec![1, 2])\n",
        "x = vec![1]\n",
    ] {
        assert_eq!(fix_source(s).text, s, "should be untouched: {s}");
    }
}

/// Deterministic pseudo-random program generator (xorshift) for property tests.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
        xs[(self.next() % xs.len() as u64) as usize]
    }
}

fn random_program(seed: u64) -> String {
    let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let mut body = String::from("    let x = 5\n    let y = 42\n    let name = \"bob\"\n    let f = 2.71828\n");
    let fmts = ["{}", "{:?}", "{:.2}", "{:>6}", "{:<6}|", "{:04}", "{{}}", "[{}]", "${}"];
    for _ in 0..(3 + rng.next() % 6) {
        let n = 1 + (rng.next() % 3) as usize;
        let mut tmpl = String::new();
        let mut args: Vec<&str> = Vec::new();
        for _ in 0..n {
            let (spec, arg) = match rng.next() % 4 {
                0 => (rng.pick(&["{}", "{:?}", "{:>6}", "{:<6}|", "{:04}"]), "x"),
                1 => (rng.pick(&["{}", "{:?}", "{:>6}"]), "name"),
                2 => (rng.pick(&["{}", "{:.2}", "{:8.3}"]), "f"),
                _ => (rng.pick(&["{}", "{:?}"]), "(x + y)"),
            };
            tmpl.push_str(rng.pick(&["a ", "", "v=", "$ ", "{{", "}} "]));
            tmpl.push_str(spec);
            args.push(arg);
        }
        let _ = fmts;
        let call = match rng.next() % 6 {
            0 => format!("    println!(\"{tmpl}\", {})\n", args.join(", ")),
            1 => format!("    print!(\"{tmpl}\\n\", {})\n", args.join(", ")),
            2 => format!("    eprintln!(\"{tmpl}\", {})\n", args.join(", ")),
            3 => { let id = rng.next() % 1000; format!("    let s{id} = format!(\"{tmpl}\", {})\n    println!(\"{{}}\", s{id})\n", args.join(", ")) },
            4 => format!("    assert!(x == 5, \"{tmpl}\", {})\n    println(\"ok\")\n", args.join(", ")),
            _ => format!("    println(\"{tmpl}\", {})\n", args.join(", ")),
        };
        body.push_str(&call);
    }
    body.push_str("    let v = vec![x, y, 3]\n    println!(\"{:?} {}\", v, v.len())\n");
    format!("fn main() {{\n{body}}}\n")
}

#[test]
fn fix_is_idempotent_and_preserves_behaviour_on_generated_programs() {
    for seed in 1..=12u64 {
        let src = random_program(seed);
        // naming collision guard for the s{n} locals is not needed: each program is small
        let fixed = fix_source(&src).text;
        assert_eq!(fix_source(&fixed).text, fixed, "not idempotent (seed {seed}):\n{src}");
        assert!(!fixed.contains("!("), "macro left over (seed {seed}):\n{fixed}");
        let before = match compile_single_result(&src) {
            Ok(c) => c,
            Err(_) => continue, // generator produced something the old compiler rejects: skip
        };
        let after = compile_single(&fixed);
        let (o1, e1, s1) = run_rust(&before);
        let (o2, e2, s2) = run_rust(&after);
        assert_eq!((o1, e1, s1), (o2, e2, s2), "behaviour changed (seed {seed}):\n--- before\n{src}\n--- after\n{fixed}");
    }
}

#[test]
fn fix_preserves_behaviour_hand_written_programs() {
    let programs = [
        r#"
struct Point { x: int, y: int }
fn describe(p: Point) -> string {
    format!("({}, {})", p.x, p.y)
}
fn main() {
    let p = Point { x: 1, y: 2 }
    println!("{}", describe(p))
    let items = vec![1, 2, 3]
    for i in vec![10, 20] {
        println!("i = {} total = {}", i, items.len())
    }
    eprintln!("done {:?}", "ok")
    assert_eq!(items.len(), 3, "len {}", items.len())
}
"#,
        r#"
fn main() {
    let width = 8
    let pi = 3.14159265
    println!("[{:>8.3}] [{:<8}] [{:^8}]", pi, "ab", "cd")
    println!("{} {{literal}} {}", width, pi)
    let words = vec!["a", "b"]
    println!("{:?}", words)
    print!("no newline")
    println!()
    let t = format!("{}{}", words[0], words[1])
    println!("{}", t)
}
"#,
    ];
    for src in programs {
        let fixed = fix_source(src).text;
        assert_eq!(fix_source(&fixed).text, fixed);
        let (o1, e1, s1) = run_rust(&compile_single(src));
        let (o2, e2, s2) = run_rust(&compile_single(&fixed));
        assert_eq!((o1, e1, s1), (o2, e2, s2), "--- after\n{fixed}");
    }
}

#[test]
fn wj_fix_cli_rewrites_in_place_and_check_mode_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("m.wj");
    fs::write(&file, "fn main() {\n    println!(\"x {}\", 1)\n}\n").unwrap();
    let check = run_wj_command(["fix", "--check", file.to_str().unwrap()]);
    assert!(!check.status.success(), "--check must fail when changes are needed");
    assert_eq!(fs::read_to_string(&file).unwrap(), "fn main() {\n    println!(\"x {}\", 1)\n}\n");
    let fix = run_wj_command(["fix", file.to_str().unwrap()]);
    assert!(fix.status.success(), "{}", String::from_utf8_lossy(&fix.stderr));
    assert_eq!(fs::read_to_string(&file).unwrap(), "fn main() {\n    println(\"x ${1}\")\n}\n");
    let again = run_wj_command(["fix", "--check", file.to_str().unwrap()]);
    assert!(again.status.success(), "second run must be a no-op");
}

// ---------------------------------------------------------------------------
// 5. Other backends
// ---------------------------------------------------------------------------

fn build_target(target: &str, body: &str) -> String {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("p.wj");
    fs::write(&src, main_with(body)).unwrap();
    let out = tmp.path().join("out");
    let res = run_wj_command([
        "build",
        src.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--no-cargo",
        "-t",
        target,
    ]);
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let file = match target {
        "go" => "main.go",
        _ => "output.js",
    };
    fs::read_to_string(out.join(file)).unwrap()
}

#[test]
fn go_backend_lowers_intrinsics() {
    let go = build_target(
        "go",
        r#"    println("v=${f:.2} ${x}")
    stderr.println("e ${x}")
    assert(x == 5, "x was ${x}")
    assert_eq(x, 5)
    let y = debug(x)
    if y > 100 { panic("big ${y}") }
    if y > 200 { todo() }
    if y > 300 { unreachable() }"#,
    );
    assert!(go.contains("%.2f"), "{go}");
    assert!(go.contains("println(fmt.Sprintf(\"e %v\""), "{go}");
    assert!(go.contains("panic("), "{go}");
    assert!(go.contains("wjDebug"), "{go}");
    assert!(!go.contains("eprintln("), "{go}");
}

#[test]
fn js_backend_lowers_intrinsics() {
    let js = build_target(
        "javascript",
        r#"    println("v=${f:.2} ${x}")
    stderr.println("e ${x}")
    assert(x == 5, "x was ${x}")
    assert_eq(x, 5)
    let y = debug(x)
    if y > 100 { panic("big ${y}") }
    if y > 200 { todo() }"#,
    );
    assert!(js.contains("toFixed(2)"), "{js}");
    assert!(js.contains("console.error("), "{js}");
    assert!(js.contains("throw new Error"), "{js}");
    assert!(!js.contains("/* macro"), "{js}");
}

#[test]
fn interpreter_runs_plain_and_macro_forms_identically() {
    let run = |src: &str| -> String {
        let tokens = windjammer::lexer::Lexer::new(src).tokenize_with_locations();
        let mut parser = windjammer::parser::Parser::new_with_source(
            tokens,
            "t.wj".to_string(),
            src.to_string(),
        );
        let program = parser.parse().unwrap();
        let mut interp = windjammer::interpreter::Interpreter::new_capturing();
        interp.run(&program).unwrap();
        interp.get_output()
    };
    let macros = "fn main() {\n    let x = 5\n    let f = 2.0\n    println!(\"x={} f={:.2} {{ok}}\", x, f)\n    print!(\"a\")\n    assert_eq!(x, 5)\n    let v = vec![1, 2]\n    println!(\"{}\", v.len())\n    let y = dbg!(x)\n    println!(\"{}\", y)\n}\n";
    let plain = "fn main() {\n    let x = 5\n    let f = 2.0\n    println(\"x=${x} f=${f:.2} {ok}\")\n    print(\"a\")\n    assert_eq(x, 5)\n    let v = [1, 2]\n    println(\"${v.len()}\")\n    let y = debug(x)\n    println(\"${y}\")\n}\n";
    let a = run(macros);
    let b = run(plain);
    assert_eq!(a, b);
    assert_eq!(a, "x=5 f=2.00 {ok}\na2\n5\n");
}
