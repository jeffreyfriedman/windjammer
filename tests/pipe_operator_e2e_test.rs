#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! End-to-end: compile pipe-operator programs with `wj`, build the generated
//! Rust, run it, and compare stdout with hard-coded expected values.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::path::PathBuf;

fn compile_and_run(name: &str, source: &str) -> String {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::write(dir.join(format!("{name}.wj")), source).unwrap();

    let out = std::process::Command::new(PathBuf::from(env!("CARGO_BIN_EXE_wj")))
        .args(["build", "--no-cargo", &format!("{name}.wj")])
        .current_dir(&dir)
        .output()
        .expect("run wj build");
    assert!(
        out.status.success(),
        "wj build failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );

    let target = test_utils::cargo_build_generated(&dir.join("build"));
    let run = std::process::Command::new(target.join(format!("debug/{name}")))
        .current_dir(&dir)
        .output()
        .expect("run program");
    assert!(
        run.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout).to_string()
}

#[test]
fn e2e_conformance_program_matches_declared_output() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/conformance/control_flow/pipe_operator.wj");
    let source = fs::read_to_string(&path).unwrap();
    // Expected output is the `// ` block after "EXPECTED OUTPUT:" in the header.
    let expected: Vec<String> = source
        .lines()
        .skip_while(|l| !l.starts_with("// EXPECTED OUTPUT:"))
        .skip(1)
        .take_while(|l| l.starts_with("// "))
        .map(|l| l[3..].to_string())
        .collect();
    assert_eq!(expected.len(), 28, "header should declare 28 lines");
    let stdout = compile_and_run("pipe_e2e_conformance", &source);
    let actual: Vec<String> = stdout.lines().map(|s| s.to_string()).collect();
    assert_eq!(actual, expected);
}

#[test]
fn e2e_borrowed_receivers_multiline_and_widths() {
    let source = r#"
struct Account {
    name: string,
    balance: int,
}

impl Account {
    fn new(name: string, balance: int) -> Account {
        Account { name: name, balance: balance }
    }

    fn describe(self) -> string {
        "${self.name}=${self.balance}"
    }

    fn after_fee(self, fee: int) -> int {
        self.balance - fee
    }
}

fn label(a: Account) -> string {
    "<${a.name}>"
}

fn clamp_u8(n: int) -> u8 {
    if n > 255 { 255 } else { n as u8 }
}

fn widen(b: u8) -> i64 {
    b as i64 * 1000
}

fn pad(s: string, n: int) -> string {
    let mut out = s
    let mut i = 0
    while i < n {
        out = "${out}."
        i += 1
    }
    out
}

fn trim_neg(n: int) -> Option<int> {
    if n < 0 { None } else { Some(n) }
}

fn run(name: string, bal: int, fee: int) {
    let acct = Account::new(name, bal)
    // &self method pipes must not consume the account
    let d = acct |> .describe()
    let l = acct |> label
    let rest = acct |> .after_fee(fee)
    println("${d} ${l} ${rest}")
}

fn run_chain(n: int) {
    let w = n
        |> clamp_u8        // narrow to u8
        // widen to i64
        |> widen
    let p = "x"
        |> pad(n % 4)
    println("${n}: ${w} ${p}")
}

fn run_opt(n: int) {
    match n |> trim_neg {
        Some(v) => println("some ${v |> pad_num}"),
        None => println("none"),
    }
}

fn pad_num(v: int) -> int {
    v + 1000
}

fn main() {
    run("ann", 100, 30)
    run("bo", 5, 10)
    run("cy", 0, 0)
    run_chain(7)
    run_chain(300)
    run_chain(255)
    run_opt(4)
    run_opt(-1)
    run_opt(0)
}
"#;
    let expected = "\
ann=100 <ann> 70
bo=5 <bo> -5
cy=0 <cy> 0
7: 7000 x...
300: 255000 x
255: 255000 x...
some 1004
none
some 1000
";
    assert_eq!(compile_and_run("pipe_e2e_receivers", source), expected);
}
