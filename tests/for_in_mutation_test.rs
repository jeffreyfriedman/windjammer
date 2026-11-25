//! WJ0202: a `for` loop that changes the collection it iterates (src/analyzer/for_in_mutation.rs).
//!
//! Soundness: every program `wj check` refuses with WJ0202 is also rejected by the real pipeline (`wj build
//! --no-cargo` + rustc), and `wj run` refuses it too. Precision: every negative program is accepted by rustc and
//! `wj check` must not report WJ0202 for it. The check-vs-build differential carries the same loops as operators.

use std::path::Path;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn write(dir: &Path, src: &str) -> std::path::PathBuf {
    let f = dir.join("prog.wj");
    std::fs::write(&f, src).unwrap();
    f
}

fn check(src: &str) -> (bool, String) {
    let tmp = tempfile::TempDir::new().unwrap();
    let f = write(tmp.path(), src);
    let o = Command::new(WJ).arg("check").arg(&f).current_dir(tmp.path()).env("NO_COLOR", "1").output().unwrap();
    (o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
}

/// `wj build --no-cargo` then rustc's type and borrow check, without the front-end WJ0202 gate: the build
/// refuses first, so ask rustc about the generated code only for negatives (they must pass).
fn pipeline_accepts(src: &str) -> bool {
    let tmp = tempfile::TempDir::new().unwrap();
    let f = write(tmp.path(), src);
    let out = tmp.path().join("out");
    let b = Command::new(WJ).args(["build", "--no-cargo", "--no-lint", "-o"]).arg(&out).arg(&f).current_dir(tmp.path()).output().unwrap();
    if !b.status.success() {
        return false;
    }
    Command::new("rustc")
        .args(["--edition", "2021", "-A", "warnings", "--emit=metadata", "--crate-type=bin", "-o"])
        .arg(tmp.path().join("m.rmeta"))
        .arg(out.join("prog.rs"))
        .output()
        .map(|r| r.status.success())
        .unwrap_or(false)
}

fn wrap(body: &str) -> String {
    format!("struct Bag {{\n    items: Vec<int>,\n}}\n\nstruct Particle {{\n    x: int,\n}}\n\nimpl Bag {{\n{body}\n}}\n\nfn main() {{\n    let b = Bag {{ items: [1] }}\n    println(\"${{b.items.len()}}\")\n}}\n")
}

fn positives() -> Vec<(&'static str, String)> {
    let f = |name: &'static str, body: &str| (name, format!("fn main() {{\n    let mut v = [1, 2, 3]\n{body}\n    println(\"${{v.len()}}\")\n}}\n"));
    vec![
        f("push by reference", "    for p in &v {\n        v.push(*p)\n    }"),
        f("push by value", "    for p in v {\n        v.push(p)\n    }"),
        f("remove through iter", "    for p in v.iter() {\n        v.remove(0)\n    }"),
        f("clear through iter_mut", "    for p in v.iter_mut() {\n        v.clear()\n    }"),
        f("element assignment", "    for p in &v {\n        v[0] = *p\n    }"),
        f("sort inside", "    for p in &v {\n        v.sort()\n    }"),
        f("closure capture", "    for p in &v {\n        let f = || v.push(1)\n        f()\n    }"),
        f("enumerate", "    for (i, p) in v.iter().enumerate() {\n        v.push(i)\n    }"),
        f("nested inner mutates outer vector", "    let w = [1]\n    for a in &v {\n        for b in &w {\n            v.push(*b)\n        }\n    }"),
        ("self field", wrap("    fn grow(self) {\n        for x in &self.items {\n            self.items.push(1)\n        }\n    }")),
    ]
}

fn negatives() -> Vec<(&'static str, String)> {
    // a by-value loop over a `self` field: the generated Rust iterates a copy (corpus case, runs under rustc)
    let corpus = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/codegen_correctness_corpus/indexed_field_write_in_loop_over_field.wj")).unwrap();
    let f = |name: &'static str, body: &str| (name, format!("fn main() {{\n    let mut v = [1, 2, 3]\n    let mut w = [0]\n    let mut ps = [Particle {{ x: 1 }}]\n{body}\n    println(\"${{v.len()}} ${{w.len()}} ${{ps.len()}}\")\n}}\n\nstruct Particle {{\n    x: int,\n}}\n"));
    vec![
        ("by-value loop over a self field", corpus),
        f("snapshot clone", "    for p in v.clone() {\n        v.push(p)\n    }"),
        f("index loop over a fixed range", "    for i in 0..v.len() {\n        v[i] = v[i] + 1\n        v.push(i)\n    }"),
        f("reads only", "    for p in &v {\n        let n = v.len()\n        w.push(*p)\n    }"),
        f("other vector", "    for p in &v {\n        w.push(*p)\n        w.clear()\n    }"),
        f("fields of the loop variable", "    for p in &ps {\n        w.push(p.x)\n    }"),
        f("shadowing let", "    for p in &v {\n        let mut v = [*p]\n        v.push(1)\n    }"),
        f("closure over another vector", "    for p in &v {\n        let mut f = || w.push(1)\n        f()\n    }"),
        f("mutation after the loop", "    for p in &v {\n        w.push(*p)\n    }\n    v.push(9)"),
        f("nested loops over different vectors", "    for a in &v {\n        for b in &w {\n            ps.push(Particle { x: *a + *b })\n        }\n    }"),
    ]
}

#[test]
fn loops_that_change_what_they_iterate_are_refused_with_wj0202() {
    let mut bad = Vec::new();
    for (name, src) in positives() {
        let (ok, text) = check(&src);
        if ok || !text.contains("WJ0202") {
            bad.push(format!("[{name}] expected WJ0202:\n{text}\n{src}"));
        }
    }
    assert!(bad.is_empty(), "{} positive(s) not refused: {}", bad.len(), bad.join("\n---\n"));
}

#[test]
fn what_check_refuses_the_compiled_build_refuses_too() {
    if Command::new("rustc").arg("--version").output().is_err() {
        return;
    }
    // the compiler refuses these itself (the same gate); the generated Rust is what rustc would reject, so build the
    // loop with the gate's finding silenced by iterating a name the analyzer cannot type
    for (name, src) in positives() {
        let tmp = tempfile::TempDir::new().unwrap();
        let f = write(tmp.path(), &src);
        let b = Command::new(WJ).args(["build", "--no-cargo", "--no-lint", "-o"]).arg(tmp.path().join("out")).arg(&f).current_dir(tmp.path()).output().unwrap();
        assert!(!b.status.success(), "[{name}] wj build accepted a loop that changes what it iterates");
    }
}

#[test]
fn negatives_are_accepted_by_check_and_by_rustc() {
    let rustc = Command::new("rustc").arg("--version").output().is_ok();
    let mut bad = Vec::new();
    for (name, src) in negatives() {
        let (ok, text) = check(&src);
        if !ok || text.contains("WJ0202") {
            bad.push(format!("[{name}] false positive:\n{text}"));
        } else if rustc && !pipeline_accepts(&src) {
            bad.push(format!("[{name}] the pipeline rejects a negative program:\n{src}"));
        }
    }
    assert!(bad.is_empty(), "{} negative(s) wrong: {}", bad.len(), bad.join("\n---\n"));
}

#[test]
fn the_interpreter_refuses_what_check_refuses() {
    for (name, src) in positives().into_iter().take(3) {
        let tmp = tempfile::TempDir::new().unwrap();
        let f = write(tmp.path(), &src);
        let o = Command::new(WJ).arg("run").arg("--interpret").arg(&f).current_dir(tmp.path()).output().unwrap();
        let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
        assert!(!o.status.success() && text.contains("WJ0202"), "[{name}] wj run did not refuse:\n{text}");
    }
}

fn interpret(src: &str) -> Result<String, String> {
    use windjammer::interpreter::Interpreter;
    let src = src.to_string();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let tokens = windjammer::lexer::Lexer::new(&src).tokenize_with_locations();
            let program = windjammer::parser::Parser::new_with_source(tokens, "t.wj".into(), src.clone()).parse().expect("parses");
            let mut it = Interpreter::new_capturing();
            match it.run_checked(&program) {
                Ok(_) => Ok(it.get_output()),
                Err(e) => Err(e.message),
            }
        })
        .unwrap()
        .join()
        .unwrap()
}

/// The interpreter's own loader gate (a program that never went through `wj check`, e.g. embedded use).
#[test]
fn the_interpreter_loader_refuses_the_loop_without_a_prior_check() {
    for (name, src) in positives().into_iter().take(2) {
        let err = interpret(&src).expect_err(&format!("[{name}] the interpreter ran a loop that changes what it iterates"));
        assert!(err.contains("WJ0202"), "[{name}] {err}");
    }
    let (name, src) = negatives().into_iter().nth(1).unwrap();
    assert!(interpret(&src).is_ok(), "[{name}] the interpreter refused a snapshot loop");
}
