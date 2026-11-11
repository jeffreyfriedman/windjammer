//! ADR-0002 (explicit `let mut`): `WJ0051` reports a local binding the program changes without declaring it `mut`,
//! at the first change, with the declaration as a second span and a fix that inserts `mut `. Positive cases (each
//! kind of change), negative cases (bindings that are not changed, or not judged), the fix gate (applying every fix
//! leaves a program with no WJ0051 and no new diagnostic), and the same report from `wj check`, `wj build` and
//! `wj run --interpret`.

use std::process::Command;
use windjammer::check::local_mutability_sources;
use windjammer::diagnostics::{apply, Diagnostic};

const COUNTER: &str = "struct Counter {\n    n: int,\n}\nimpl Counter {\n    fn bump(self) {\n        self.n = self.n + 1\n    }\n    fn get(self) -> int {\n        self.n\n    }\n    fn with(self, n: int) -> Counter {\n        self.n = n\n        self\n    }\n}\n";

fn diags(src: &str) -> Vec<Diagnostic> {
    local_mutability_sources(&[("m.wj".to_string(), src.to_string())])
}

/// (binding, changed-at text, declared-at text) of every WJ0051, by the text under its spans.
fn found(src: &str) -> Vec<(String, String, usize, usize)> {
    diags(src)
        .iter()
        .map(|d| {
            assert_eq!(d.code, "WJ0051");
            let p = d.primary_span.as_ref().expect("primary span");
            let s = &d.secondary_spans[0];
            (src[s.byte_start..s.byte_end].to_string(), src[p.byte_start..p.byte_end].to_string(), s.line_start, p.line_start)
        })
        .collect()
}

fn names(src: &str) -> Vec<String> {
    found(src).into_iter().map(|f| f.0).collect()
}

fn with_counter(main: &str) -> String {
    format!("{COUNTER}{main}")
}

/// Applying every fix: no WJ0051 is left, the text still parses, and nothing else appears in `wj check`.
fn fixed(src: &str) -> String {
    let d = diags(src);
    let edits: Vec<_> = apply::collect_edits(&d).into_values().flatten().collect();
    let out = apply::apply_edits(src, &edits).expect("edits apply");
    assert!(diags(&out).is_empty(), "WJ0051 left after the fix:\n{out}");
    let root = std::path::Path::new("/nonexistent");
    let before: Vec<String> = windjammer::check::check_sources(root, &[("m.wj".into(), src.into())]).iter().filter(|d| d.code != "WJ0051").map(|d| d.code.clone()).collect();
    let after: Vec<String> = windjammer::check::check_sources(root, &[("m.wj".into(), out.clone())]).iter().map(|d| d.code.clone()).collect();
    assert_eq!(before, after, "the fix changed other diagnostics:\n{out}");
    out
}

#[test]
fn a_method_that_changes_its_receiver_changes_the_binding() {
    let src = with_counter("fn main() {\n    let c = Counter { n: 0 }\n    c.bump()\n    println(\"${c.get()}\")\n}\n");
    let f = found(&src);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!((f[0].0.as_str(), f[0].1.as_str()), ("c", "c"));
    assert!(f[0].3 > f[0].2, "the change is reported after the declaration: {f:?}");
    assert!(fixed(&src).contains("let mut c = Counter"));
}

#[test]
fn core_methods_declared_mut_self_change_the_binding() {
    let src = "fn main() {\n    let v = Vec::new()\n    v.push(1)\n    let s = String::new()\n    s.push_str(\"a\")\n    let m = HashMap::new()\n    m.insert(1, 2)\n    let o = Some(3)\n    let t = o.take()\n    println(\"${v.len()} ${s} ${m.len()} ${t.is_some()}\")\n}\n";
    assert_eq!(names(src), ["v", "s", "m", "o"]);
    fixed(src);
}

#[test]
fn assignments_to_the_binding_its_fields_and_elements() {
    let src = "struct P {\n    x: int,\n}\nfn main() {\n    let k = 0\n    k += 1\n    let j = 1\n    j = 2\n    let p = P { x: 1 }\n    p.x = 5\n    let v = [1, 2, 3]\n    v[0] = 9\n    println(\"${k} ${j} ${p.x} ${v[0]}\")\n}\n";
    assert_eq!(names(src), ["k", "j", "p", "v"]);
    fixed(src);
}

#[test]
fn pattern_bindings_loop_variables_and_closures() {
    let src = with_counter(
        "fn via_tuple(p: (Counter, int)) -> int {\n    let (c, k) = p\n    c.bump()\n    c.get() + k\n}\nfn main() {\n    for i in 0..3 {\n        i += 1\n        println(\"${i}\")\n    }\n    let n = 0\n    let add = |d: int| { n += d }\n    add(2)\n    println(\"${n} ${via_tuple((Counter { n: 1 }, 10))}\")\n}\n",
    );
    assert_eq!(names(&src), ["c", "i", "n"]);
    let out = fixed(&src);
    assert!(out.contains("let (mut c, k) = p") && out.contains("for mut i in 0..3"), "{out}");
}

#[test]
fn an_argument_the_callee_changes_and_a_transitive_receiver() {
    let src = "struct Gen {\n    state: int,\n}\nimpl Gen {\n    fn step(self) -> int {\n        self.state = self.state + 1\n        self.state\n    }\n    fn plus_one(self) -> int {\n        1 + self.step()\n    }\n}\nfn fill(v: Vec<int>) {\n    v.push(7)\n}\nfn main() {\n    let g = Gen { state: 0 }\n    let r = g.plus_one()\n    let xs = Vec::new()\n    fill(xs)\n    println(\"${r} ${xs.len()}\")\n}\n";
    assert_eq!(names(src), ["g", "xs"]);
    fixed(src);
}

#[test]
fn unchanged_explicit_shadowed_and_undecided_bindings_are_not_reported() {
    // read only; already `mut`; a changed inner shadow declared `mut`; a builder that consumes its receiver;
    // a scalar passed by value to a parameter the callee changes; a loop variable changed through a field;
    // a match binding; a reference taken from `get_mut`; a parameter
    let src = with_counter(
        "struct P {\n    x: int,\n}\nfn twice(x: int) -> int {\n    x += x\n    x\n}\nfn touch(c: Counter) {\n    c.bump()\n}\nfn main() {\n    let a = Counter { n: 1 }\n    println(\"${a.get()}\")\n    let mut b = Counter { n: 2 }\n    b.bump()\n    let x = 1\n    if x > 0 {\n        let mut x = 5\n        x += 1\n        println(\"${x}\")\n    }\n    let c = Counter { n: 0 }\n    let d = c.with(4)\n    let k = 3\n    let t = twice(k)\n    let ps = [P { x: 1 }]\n    for p in ps {\n        println(\"${p.x}\")\n    }\n    let o = Some(Counter { n: 0 })\n    match o {\n        Some(q) => println(\"${q.get()}\"),\n        None => println(\"none\"),\n    }\n    let mut v = [P { x: 1 }]\n    let e = v.get_mut(0).unwrap()\n    e.x = 2\n    touch(b)\n    println(\"${x} ${d.get()} ${t} ${k}\")\n}\n",
    );
    assert_eq!(names(&src), Vec::<String>::new());
}

#[test]
fn a_change_inside_interpolation_points_at_the_string() {
    let src = with_counter("fn main() {\n    let c = Counter { n: 0 }\n    println(\"${c.with(1).get()} ${c.get()}\")\n    let v = Vec::new()\n    println(\"${v.push(1)}\")\n}\n");
    let f = found(&src);
    assert_eq!(f.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["v"], "{f:?}");
}

fn wj(args: &[&str], dir: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_wj")).args(args).current_dir(dir).env("NO_COLOR", "1").output().unwrap()
}

/// The `warning[WJ0051]` block (header and span lines) of a rendered report.
fn block(text: &str) -> Vec<String> {
    let start = text.lines().position(|l| l.contains("[WJ0051]")).unwrap_or_else(|| panic!("no WJ0051 in:\n{text}"));
    text.lines().skip(start).take_while(|l| !l.trim().is_empty() && !l.starts_with("  = ")).map(str::to_string).collect()
}

#[test]
fn check_build_and_both_runs_report_the_same_warning_and_the_program_still_runs() {
    let dir = tempfile::TempDir::new().unwrap();
    let src = with_counter("fn main() {\n    let c = Counter { n: 0 }\n    c.bump()\n    println(\"${c.get()}\")\n}\n");
    std::fs::write(dir.path().join("m.wj"), &src).unwrap();
    let check = wj(&["check", "m.wj"], dir.path());
    assert!(check.status.success(), "a warning does not fail `wj check`");
    let reference = block(&String::from_utf8_lossy(&check.stdout));
    assert!(reference[0].starts_with("warning[WJ0051]"), "{reference:?}");
    let run = wj(&["run", "--interpret", "m.wj"], dir.path());
    assert_eq!(String::from_utf8_lossy(&run.stdout), "1\n", "stdout is the program's alone");
    assert_eq!(run.status.code(), Some(0));
    assert_eq!(block(&String::from_utf8_lossy(&run.stderr)), reference);
    let build = wj(&["build", "--no-cargo", "-o", "out", "m.wj"], dir.path());
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    assert_eq!(block(&String::from_utf8_lossy(&build.stderr)), reference);
    // `wj fix` inserts the `mut`; afterwards every command is silent about it
    let fix = wj(&["fix", "m.wj"], dir.path());
    assert!(fix.status.success(), "{}", String::from_utf8_lossy(&fix.stderr));
    assert!(std::fs::read_to_string(dir.path().join("m.wj")).unwrap().contains("let mut c = Counter"));
    let again = wj(&["check", "m.wj"], dir.path());
    assert!(!String::from_utf8_lossy(&again.stdout).contains("WJ0051"));
}
