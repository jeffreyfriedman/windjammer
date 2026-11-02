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

//! Typed JSON conformance (ADR-0027). Every `tests/json_conformance/*.wj` program runs interpreted and
//! compiled; both must print exactly its `// expect:` lines. The corpus is the behavioural oracle shared by the
//! two engines. Lines starting `enc ` are also wire-format goldens: Python's `json` module must parse them and
//! re-print them byte for byte (compact separators, key order kept), an encoder independent of serde.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn corpus() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/json_conformance");
    let mut v: Vec<PathBuf> = fs::read_dir(dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "wj")).collect();
    v.sort();
    v
}

fn expected(src: &str) -> String {
    src.lines().filter_map(|l| l.strip_prefix("// expect:")).map(|r| format!("{}\n", r.strip_prefix(' ').unwrap_or(r))).collect()
}

fn run(mode: &str, path: &Path) -> (String, String, i32) {
    let o = Command::new(WJ).args(["run", mode, "--no-lint"]).arg(path).env("CARGO_BUILD_JOBS", "2").output().expect("spawn wj");
    (String::from_utf8_lossy(&o.stdout).into(), String::from_utf8_lossy(&o.stderr).into(), o.status.code().unwrap_or(-1))
}

fn check_engine(mode: &str) {
    let files = corpus();
    assert!(files.len() >= 2, "json conformance corpus is empty");
    let mut bad = Vec::new();
    for f in &files {
        let want = expected(&fs::read_to_string(f).unwrap());
        assert!(!want.is_empty(), "{}: no `// expect:` lines", f.display());
        let (got, err, code) = run(mode, f);
        if got != want || code != 0 {
            bad.push(format!("== {} ({mode}, exit {code})\n--- want\n{want}--- got\n{got}--- stderr (tail)\n{}", f.display(), tail(&err)));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

fn tail(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();
    lines[lines.len().saturating_sub(15)..].join("\n")
}

#[test]
fn interpreted_engine_matches_the_json_corpus() {
    check_engine("--interpret");
}

#[test]
fn compiled_engine_matches_the_json_corpus() {
    if Command::new("cargo").arg("--version").output().is_err() {
        eprintln!("SKIPPED: cargo unavailable");
        return;
    }
    check_engine("--compiled");
}

#[test]
fn encoded_lines_are_canonical_json_for_python() {
    let goldens: Vec<String> = corpus()
        .iter()
        .flat_map(|f| expected(&fs::read_to_string(f).unwrap()).lines().filter_map(|l| l.strip_prefix("enc ").map(String::from)).collect::<Vec<_>>())
        .collect();
    assert!(goldens.len() >= 3, "too few wire-format goldens");
    let script = "import json,sys\nfor line in sys.stdin.read().splitlines():\n    print(json.dumps(json.loads(line), separators=(',', ':'), ensure_ascii=False))\n";
    let Ok(mut child) = Command::new("python3").args(["-I", "-c", script]).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).spawn() else {
        eprintln!("SKIPPED: python3 unavailable");
        return;
    };
    use std::io::Write;
    child.stdin.take().unwrap().write_all(goldens.join("\n").as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "python rejected a golden: {}", String::from_utf8_lossy(&out.stderr));
    let got: Vec<String> = String::from_utf8_lossy(&out.stdout).lines().map(String::from).collect();
    assert_eq!(got, goldens, "an encoding is not canonical compact JSON");
}

/// The gate can fail: a corpus whose expectation is wrong is reported (mutation self-test of the harness).
#[test]
fn harness_reports_a_wrong_expectation() {
    let d = tempfile::TempDir::new().unwrap();
    let f = d.path().join("m.wj");
    fs::write(&f, "// expect: enc {\"x\":2}\nuse std::json\nstruct P { x: int }\nfn main() {\n    match json.to_string(P { x: 1 }) {\n        Ok(s) => println(\"enc ${s}\"),\n        Err(e) => println(e),\n    }\n}\n").unwrap();
    let (got, _, _) = run("--interpret", &f);
    assert_eq!(got, "enc {\"x\":1}\n");
    assert_ne!(got, expected(&fs::read_to_string(&f).unwrap()));
}

/// ADR-0027: encoding and decoding are pure. A `@pure` function may call them; the control (`println` under
/// `@pure`) proves the check is live.
#[test]
fn typed_json_is_pure() {
    let d = tempfile::TempDir::new().unwrap();
    let check = |body: &str| {
        let f = d.path().join("p.wj");
        let src = format!("use std::json\n\nstruct P {{ x: int }}\n\n@pure\nfn f(text: string) -> Result<P, string> {{\n{body}\n}}\n\nfn main() {{\n    match f(\"{{\\\"x\\\":1}}\") {{\n        Ok(p) => println(\"${{p.x}}\"),\n        Err(e) => println(e),\n    }}\n}}\n");
        fs::write(&f, src).unwrap();
        let o = Command::new(WJ).arg("check").arg(&f).output().unwrap();
        String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr)
    };
    let pure = check("    let p: P = json.parse_string(text)?\n    let back = json.to_string(p)?\n    json.parse_string(back)");
    assert!(pure.contains("0 error(s)"), "{pure}");
    let noisy = check("    println(text)\n    json.parse_string(text)");
    assert!(noisy.contains("WJ0042"), "the @pure check must reject console output:\n{noisy}");
}
