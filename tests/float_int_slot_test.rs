//! A float where an int is declared (and an int where a float is declared) is WJ0003, decided by declarations:
//! annotated bindings, parameters, declared returns and fields, `as` casts and arithmetic over them. The three
//! engines agree: `wj check` and `wj run --interpret` report it, and the compiled backend never inserts an `as`
//! conversion between int and float (it used to emit `half(x as i64)` and silently truncate).

use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn wj(args: &[&str], src: &str) -> (bool, String, std::path::PathBuf, tempfile::TempDir) {
    let tmp = tempfile::TempDir::new().unwrap();
    let f = tmp.path().join("main.wj");
    std::fs::write(&f, src).unwrap();
    let o = Command::new(WJ).args(args).arg(&f).env("NO_COLOR", "1").current_dir(tmp.path()).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    (o.status.success(), text, f, tmp)
}

const FN_HALF: &str = "fn half(v: int) -> int {\n    v / 2\n}\n";

/// (name, source, the message each error must carry)
fn rejected() -> Vec<(&'static str, String, &'static str)> {
    let int_found_float = "expected `int`, found `float`";
    let float_found_int = "expected `float`, found `int`";
    vec![
        ("let expression", "fn main() {\n    let x: f64 = 2.5\n    let m: int = x * 3.0\n    println(\"${m}\")\n}\n".into(), int_found_float),
        ("let variable", "fn main() {\n    let x: float = 2.5\n    let m: int = x\n    println(\"${m}\")\n}\n".into(), int_found_float),
        ("argument", format!("{FN_HALF}fn main() {{\n    let x: f64 = 9.0\n    println(\"${{half(x)}}\")\n}}\n"), int_found_float),
        ("argument expression", format!("{FN_HALF}fn main() {{\n    let x: f64 = 9.0\n    println(\"${{half(x + 1.0)}}\")\n}}\n"), int_found_float),
        ("method argument", "struct S {\n    k: int,\n}\nimpl S {\n    fn add(self, d: int) -> int {\n        self.k + d\n    }\n}\nfn main() {\n    let s = S { k: 1 }\n    let d: float = 1.5\n    println(\"${s.add(d)}\")\n}\n".into(), int_found_float),
        ("associated fn argument", "struct S {\n    k: int,\n}\nimpl S {\n    fn make(k: int) -> S {\n        S { k: k }\n    }\n}\nfn main() {\n    let d: float = 1.5\n    let s = S::make(d)\n    println(\"${s.k}\")\n}\n".into(), int_found_float),
        ("std function argument", "use std::math\nfn main() {\n    let n: int = 4\n    println(\"${math.sqrt(n)}\")\n}\n".into(), float_found_int),
        ("struct field", "struct P {\n    n: int,\n}\nfn main() {\n    let f: float = 1.5\n    let p = P { n: f * 2.0 }\n    println(\"${p.n}\")\n}\n".into(), int_found_float),
        ("float field", "struct P {\n    x: float,\n}\nfn main() {\n    let n: int = 3\n    let p = P { x: n }\n    println(\"${p.x}\")\n}\n".into(), float_found_int),
        ("tail return", "fn ratio(x: float) -> int {\n    x * 2.0\n}\nfn main() {\n    println(\"${ratio(1.5)}\")\n}\n".into(), int_found_float),
        ("early return", "fn ratio(x: float) -> int {\n    if x > 1.0 {\n        return x\n    }\n    0\n}\nfn main() {\n    println(\"${ratio(1.5)}\")\n}\n".into(), int_found_float),
        ("assignment", "fn main() {\n    let mut m: int = 0\n    let f: float = 2.5\n    m = f + 1.0\n    println(\"${m}\")\n}\n".into(), int_found_float),
        ("field assignment", "struct P {\n    n: int,\n}\nfn main() {\n    let mut p = P { n: 1 }\n    let f: float = 2.5\n    p.n = f\n    println(\"${p.n}\")\n}\n".into(), int_found_float),
        ("list literal element", "fn main() {\n    let f: float = 2.5\n    let xs: Vec<int> = [1, f]\n    println(\"${xs.len()}\")\n}\n".into(), int_found_float),
        ("list push", "fn main() {\n    let f: float = 2.5\n    let mut xs: Vec<int> = Vec::new()\n    xs.push(f * 2.0)\n    println(\"${xs.len()}\")\n}\n".into(), int_found_float),
        ("list element assignment", "fn main() {\n    let f: float = 2.5\n    let mut xs: Vec<int> = [1, 2]\n    xs[0] = f\n    println(\"${xs.len()}\")\n}\n".into(), int_found_float),
        ("declared float method", "fn main() {\n    let f: float = 2.5\n    let m: int = f.sqrt()\n    println(\"${m}\")\n}\n".into(), int_found_float),
        ("declared field", "struct P {\n    x: float,\n}\nfn main() {\n    let p = P { x: 1.5 }\n    let m: int = p.x\n    println(\"${m}\")\n}\n".into(), int_found_float),
        ("int into float let", "fn main() {\n    let n: int = 3\n    let f: float = n * 2\n    println(\"${f}\")\n}\n".into(), float_found_int),
    ]
}

#[test]
fn a_float_in_an_int_slot_and_an_int_in_a_float_slot_are_wj0003() {
    for (name, src, message) in rejected() {
        let (ok, out, _, _t) = wj(&["check"], &src);
        assert!(!ok && out.contains("error[WJ0003]") && out.contains(message), "{name}: {out}");
        assert!(out.contains("convert explicitly:"), "{name}: the hint names the explicit conversion: {out}");
        assert!(!out.contains("wj fix"), "{name}: a lossy conversion is a hint, never a machine-applicable fix: {out}");
    }
}

#[test]
fn the_hint_spells_the_whole_expression_and_the_conversion() {
    let (_, out, _, _t) = wj(&["check"], "fn main() {\n    let x: f64 = 2.5\n    let m: int = x * 3.0\n    println(\"${m}\")\n}\n");
    assert!(out.contains("`(x * 3.0) as int`") && out.contains("main.wj:3:18"), "{out}");
    let (_, out, _, _t) = wj(&["check"], &format!("{FN_HALF}fn main() {{\n    let x: f64 = 9.0\n    println(\"${{half(x)}}\")\n}}\n"));
    assert!(out.contains("`x as int`") && out.contains("or declare it `float`"), "{out}");
}

#[test]
fn the_check_has_no_machine_applicable_fix() {
    let (_, out, _, _t) = wj(&["check", "--json"], "fn main() {\n    let x: f64 = 2.5\n    let m: int = x * 3.0\n    println(\"${m}\")\n}\n");
    assert!(out.contains("WJ0003") && !out.contains("machine_applicable") && !out.contains("MachineApplicable"), "{out}");
}

#[test]
fn explicit_conversions_and_untyped_literals_are_accepted() {
    let src = "struct P {\n    x: float,\n    n: int,\n}\n\nfn half(v: int) -> int {\n    v / 2\n}\n\nfn scale(v: float) -> float {\n    v * 10.0\n}\n\nfn trunc(v: float) -> int {\n    v as int\n}\n\nfn main() {\n    let x: float = 9.7\n    let n: int = x as int\n    let back: float = n as float\n    let lit: float = 3\n    let p = P { x: 4, n: (x * 2.0) as int }\n    let mut xs: Vec<float> = [1, 2.5]\n    xs.push(7)\n    xs[0] = 2\n    let mut total = 0\n    total = total + 1\n    println(\"${half(x as int)} ${scale(2)} ${scale(n as float)} ${trunc(x)} ${back} ${lit} ${p.x} ${p.n} ${xs.len()} ${total}\")\n}\n";
    let (ok, out, _, _t) = wj(&["check"], src);
    assert!(ok && !out.contains("WJ0003"), "{out}");
}

#[test]
fn a_name_imported_from_another_crate_is_not_the_project_function_of_that_name() {
    // `median` is another crate's function here; a project function of the same name elsewhere must not judge it
    let src = "use wj_stats::median\n\nfn median_of(xs: Vec<float>) -> float {\n    median(xs)\n}\n\nfn main() {\n    println(\"${median_of([1.0])}\")\n}\n";
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("a.wj"), src).unwrap();
    std::fs::write(tmp.path().join("b.wj"), "fn median(xs: Vec<float>) -> int {\n    0\n}\n\nfn main() {\n    println(\"${median([1.0])}\")\n}\n").unwrap();
    let o = Command::new(WJ).arg("check").arg(tmp.path()).env("NO_COLOR", "1").output().unwrap();
    let out = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(!out.contains("WJ0003"), "{out}");
}

#[test]
fn the_interpreter_reports_it_before_running() {
    for (name, src, message) in rejected().into_iter().filter(|(n, ..)| ["let expression", "argument"].contains(n)) {
        let (ok, out, _, _t) = wj(&["run", "--interpret"], &src);
        assert!(!ok && out.contains("error[WJ0003]") && out.contains(message), "{name}: {out}");
        assert!(!out.contains("7.5"), "{name}: the program must not run: {out}");
    }
}

#[test]
fn the_compiled_backend_never_inserts_a_float_to_int_cast() {
    // Not reachable through `wj check` (it rejects this program first), but `wj build --no-cargo` transpiles it:
    // the emitted call is `half(x)`, so rustc reports E0308 instead of truncating 9.0 to 9 silently.
    let (_, out, f, tmp) = wj(&["build", "--no-cargo", "-o", "out"], &format!("{FN_HALF}fn main() {{\n    let x: f64 = 9.0\n    println(\"${{half(x)}}\")\n}}\n"));
    let rs = std::fs::read_to_string(tmp.path().join("out").join(format!("{}.rs", f.file_stem().unwrap().to_string_lossy()))).unwrap_or_else(|e| panic!("{e}: {out}"));
    assert!(rs.contains("half(x)") && !rs.contains("as i64") && !rs.contains("as f64"), "{rs}");
}
