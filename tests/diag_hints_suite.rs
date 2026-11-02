//! Golden tests for fix-oriented diagnostics: every common rustc type error is rewritten into
//! Windjammer terms with a one-line "what to write" hint and, where mechanical, a fix.
//!
//! Oracle: the rustc side of each case is REAL rustc `--error-format=json` output produced at test time
//! from a small Rust program (never hand-written strings), so the hint logic is checked against what
//! rustc really says. Run: `cargo test --release --test diag_hints_suite`.

use windjammer::diagnostics::rustc_map::{self, SourceCache};
use windjammer::diagnostics::{Applicability, Diagnostic};
use windjammer::error_mapper::{DiagnosticLevel, ErrorMapper};
use windjammer::source_map::SourceMap;

/// Compile `rs` with rustc and return the cargo-style JSON lines for its diagnostics.
fn rustc_json(rs: &str) -> String {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!("wj-hints-{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("main.rs");
    std::fs::write(&f, rs).unwrap();
    let out = std::process::Command::new("rustc")
        .args(["--edition", "2021", "--error-format=json", "--emit=metadata", "--crate-type=bin", "-A", "warnings", "-o"])
        .arg(dir.join("out.rmeta"))
        .arg("main.rs")
        .current_dir(&dir)
        .output()
        .expect("rustc is required for this suite");
    let _ = std::fs::remove_dir_all(&dir);
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter(|l| l.starts_with('{'))
        .map(|l| format!("{{\"reason\":\"compiler-message\",\"message\":{l}}}\n"))
        .collect()
}

/// Translate real rustc errors for `rs` into shared-schema diagnostics (no `.wj` file on disk).
fn diagnose(rs: &str) -> Vec<Diagnostic> {
    let json = rustc_json(rs);
    let mapped = ErrorMapper::new(SourceMap::new()).map_rustc_output(&json);
    let mut cache = SourceCache::default();
    mapped
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Error)
        .map(|d| rustc_map::from_windjammer(d, &mut cache))
        .collect()
}

fn first(rs: &str) -> Diagnostic {
    let mut d = diagnose(rs);
    assert!(!d.is_empty(), "rustc produced no error for:\n{rs}");
    d.remove(0)
}

fn all_text(d: &Diagnostic) -> String {
    format!("{} | {} | {}", d.message, d.help.join(" | "), d.notes.join(" | "))
}

#[test]
fn string_literal_where_owned_string_needed() {
    let d = first("fn take(s: String) {}\nfn main() { let a = \"x\"; take(a); }\n");
    assert_eq!(d.code, "WJ0003");
    let t = all_text(&d);
    assert!(t.contains(".to_string()"), "{t}");
    assert!(!t.contains("E0308"), "{t}");
}

#[test]
fn owned_string_where_borrowed_needed() {
    let d = first("fn show(s: &str) {}\nfn main() { let a = String::new(); show(a); }\n");
    assert_eq!(d.code, "WJ0003");
    let t = all_text(&d);
    assert!(t.contains("borrows for you"), "{t}");
    assert!(!t.contains("`&x`"), "{t}");
}

#[test]
fn len_into_int_says_cast() {
    let d = first("fn f(n: i64) {}\nfn main() { let v = vec![1]; f(v.len()); }\n");
    assert_eq!(d.code, "WJ0003");
    let t = all_text(&d);
    assert!(t.contains("len()") && t.contains("as int"), "{t}");
}

#[test]
fn int_into_usize_says_cast() {
    let d = first("fn f(n: usize) {}\nfn main() { let k: i64 = 3; f(k); }\n");
    let t = all_text(&d);
    assert!(t.contains("as uint"), "{t}");
}

#[test]
fn value_where_option_needed_says_some() {
    let d = first("fn f(n: Option<i64>) {}\nfn main() { let k: i64 = 3; f(k); }\n");
    let t = all_text(&d);
    assert!(t.contains("Some("), "{t}");
}

#[test]
fn option_where_value_needed_says_default() {
    let d = first("fn f(n: i64) {}\nfn main() { let k: Option<i64> = None; f(k); }\n");
    let t = all_text(&d);
    assert!(t.contains("?? default"), "{t}");
}

#[test]
fn float_width_says_cast() {
    let d = first("fn f(n: f32) {}\nfn main() { let k: f64 = 3.0; f(k); }\n");
    let t = all_text(&d);
    assert!(t.contains("as f32"), "{t}");
}

#[test]
fn int_into_float_says_cast() {
    let d = first("fn f(n: f64) {}\nfn main() { let k: i64 = 3; f(k); }\n");
    let t = all_text(&d);
    assert!(t.contains("as float"), "{t}");
}

#[test]
fn collect_of_references_says_cloned() {
    let d = first("fn main() { let v: Vec<String> = vec![]; let w: Vec<String> = v.iter().collect(); }\n");
    assert_eq!(d.code, "WJ0029");
    let t = all_text(&d);
    assert!(t.contains(".cloned()"), "{t}");
    assert!(t.contains("before `.collect()`"), "{t}");
}

#[test]
fn collect_of_str_refs_says_map_to_string() {
    let d = first("fn main() { let v: Vec<&str> = vec![]; let w: Vec<String> = v.into_iter().collect(); }\n");
    assert_eq!(d.code, "WJ0029");
    let t = all_text(&d);
    assert!(t.contains(".map(|s| s.to_string())"), "{t}");
}

#[test]
fn deref_of_plain_value_says_remove_star() {
    let d = first("fn main() { let x: i64 = 3; let y = *x; }\n");
    assert_eq!(d.code, "WJ0031");
    let t = all_text(&d);
    assert!(t.contains("remove the `*`"), "{t}");
}

#[test]
fn compound_assign_mismatch_is_classified() {
    let d = first("fn main() { let mut t: f64 = 0.0; let n: i64 = 2; t += n; }\n");
    assert_ne!(d.code, "WJ0099", "{}", all_text(&d));
    assert!(all_text(&d).contains("as float"), "{}", all_text(&d));
}

#[test]
fn every_translated_error_has_a_stable_code_and_a_fix_line() {
    let progs = [
        "fn f(n: i64) {}\nfn main() { f(\"a\"); }\n",
        "fn main() { let x = 1; x = 2; }\n",
        "fn main() { let v = vec![1]; v.nope(); }\n",
        "fn main() { let s = String::new(); let t = s; println!(\"{}\", s); }\n",
        "fn main() { let a: i64 = 1; let b: f64 = 2.0; let c = a + b; }\n",
        "fn main() { let x = undefined_name; }\n",
        "trait T {}\nfn f(x: T) {}\nfn main() {}\n",
    ];
    for p in progs {
        for d in diagnose(p) {
            assert_ne!(d.code, "WJ0099", "untranslated: {} for\n{p}", all_text(&d));
            assert!(!d.help.is_empty() || !d.fixes.is_empty(), "no fix line for {} in\n{p}", all_text(&d));
            let t = all_text(&d);
            assert!(!regex_like_rustc_code(&t), "rustc code leaked: {t}");
        }
    }
}

fn regex_like_rustc_code(t: &str) -> bool {
    t.as_bytes().windows(5).any(|w| w[0] == b'E' && w[1..].iter().all(u8::is_ascii_digit))
}

/// The fix is attached only when its target text is found exactly once on the mapped `.wj` line.
#[test]
fn fix_is_machine_checkable_when_the_wj_line_contains_the_expression() {
    let dir = std::env::temp_dir().join(format!("wj-hints-fix-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let wj = dir.join("main.wj");
    std::fs::write(&wj, "fn take(s: string) {}\nfn main() {\n    let a = \"x\"\n    take(a)\n}\n").unwrap();
    let rs = "fn take(s: String) {}\nfn main() {\n    let a = \"x\";\n    take(a);\n}\n";
    let json = rustc_json(rs);
    let mut sm = SourceMap::new();
    for line in 1..=5 {
        sm.add_mapping(std::path::Path::new("main.rs"), line, 1, &wj, line, 1);
    }
    let mapped = ErrorMapper::new(sm).map_rustc_output(&json);
    let mut cache = SourceCache::default();
    let d = rustc_map::from_windjammer(mapped.iter().find(|d| d.level == DiagnosticLevel::Error).unwrap(), &mut cache);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(d.fixes.len(), 1, "{d:?}");
    let fix = &d.fixes[0];
    assert_eq!(fix.applicability, Applicability::MaybeIncorrect);
    assert_eq!(fix.edits.len(), 1);
    let e = &fix.edits[0];
    assert_eq!(e.replacement, ".to_string()");
    assert_eq!(e.span.byte_start, e.span.byte_end, "an insertion");
    // Oracle: applying the edit gives the line the user should have written.
    let src = "fn take(s: string) {}\nfn main() {\n    let a = \"x\"\n    take(a)\n}\n";
    let fixed = format!("{}{}{}", &src[..e.span.byte_start], e.replacement, &src[e.span.byte_end..]);
    assert!(fixed.contains("take(a.to_string())"), "{fixed}");
}

// ---------- rung 3: Rust-leakage lints say exactly what to write ----------

fn lint_diags(src: &str) -> Vec<Diagnostic> {
    windjammer::check::check_sources(std::path::Path::new("."), &[("main.wj".to_string(), src.to_string())])
}

#[test]
fn leakage_lints_say_what_to_write_instead() {
    let cases = [
        ("W0001", "fn total(items: &Vec<int>) -> int {\n    0\n}\nfn main() {\n    println(\"${total([1])}\")\n}\n", "instead of"),
        ("W0002", "fn main() {\n    let v: Option<int> = Some(1)\n    println(\"${v.unwrap()}\")\n}\n", "x ?? default"),
        ("W0003", "fn main() {\n    let v = [1, 2]\n    for x in v.iter() {\n        println(\"${x}\")\n    }\n}\n", "for x in items"),
        ("W0004", "fn show(s: string) {\n    println(s)\n}\nfn main() {\n    let s = \"a\"\n    show(&s)\n}\n", "f(x)"),
        ("W0006", "fn main() {\n    let s = \"hello\".to_string()\n    println(s)\n}\n", "\"text\""),
    ];
    for (code, src, needle) in cases {
        let d = lint_diags(src);
        let hit = d.iter().find(|d| d.code == code).unwrap_or_else(|| panic!("{code} not reported: {d:?}"));
        let t = all_text(hit);
        assert!(t.contains("instead of"), "{code}: no replacement in {t}");
        assert!(t.contains(needle), "{code}: expected `{needle}` in {t}");
        assert!(!t.contains("Rustism") && !t.contains("Rust-specific"), "{code}: {t}");
    }
}

#[test]
fn every_code_has_an_explain_entry_with_a_fix() {
    use windjammer::diagnostics::codes;
    for info in codes::all() {
        let text = codes::explain(info.code).unwrap_or_else(|| panic!("{} has no explain", info.code));
        assert!(text.contains("Fix\n"), "{}", info.code);
        assert!(text.contains("Example (corrected)"), "{}", info.code);
    }
    for rc in ["E0308", "E0614", "E0277", "E0382", "E0599", "E0368", "E0606", "E0782", "E0369", "E0433", "E0425", "E0432", "E0384", "E0596", "E0507"] {
        assert!(codes::explain(rc).is_some(), "rustc {rc} has no Windjammer explain entry");
    }
}

// ---------- a hint must never lead the agent into another diagnostic ----------

/// Apply each Expr fix the hint table can produce to a clean program and require that `wj check`
/// reports nothing (in particular no lint such as W0004/W0006/W0012 contradicting the advice).
#[test]
fn applying_a_hint_fix_introduces_no_new_diagnostic() {
    use windjammer::diagnostics::hints::{mismatch, FixAnchor};
    let pairs = [
        ("String", "&str"), ("i64", "usize"), ("usize", "i64"), ("Option<i64>", "i64"), ("f32", "f64"),
        ("f64", "i64"), ("i64", "f64"), ("i32", "i64"), ("String", "char"), ("String", "i64"),
    ];
    for (e, f) in pairs {
        let Some(h) = mismatch(e, f) else { panic!("no hint for {e}/{f}") };
        let Some(fix) = h.fix else { continue };
        let FixAnchor::Expr { prefix, suffix } = fix.anchor else { continue };
        let src = format!("fn take(v: int) {{\n    println(\"${{v}}\")\n}}\nfn main() {{\n    let x = 3\n    take({prefix}x{suffix})\n}}\n");
        let before = lint_diags(&src.replace(&format!("{prefix}x{suffix}"), "x"));
        let after = lint_diags(&src);
        let new: Vec<_> = after.iter().filter(|d| !before.iter().any(|b| b.code == d.code)).collect();
        assert!(new.is_empty(), "{e}/{f}: fix `{prefix}x{suffix}` introduces {:?}", new.iter().map(|d| (&d.code, &d.message)).collect::<Vec<_>>());
    }
}

/// Macro-form messages carry the exact replacement and a code.
#[test]
fn macro_forms_say_what_to_write() {
    let d = lint_diags("fn main() {\n    println!(\"hi\")\n    let v = vec![1, 2]\n    println(\"${v.len()}\")\n}\n");
    let macros: Vec<_> = d.iter().filter(|d| d.code == "W0007").collect();
    assert!(macros.len() >= 2, "{d:?}");
    for m in macros {
        let t = all_text(m);
        assert!(t.contains("replace `") || t.contains("use "), "{t}");
        assert!(m.fixes.iter().any(|f| !f.edits.is_empty()), "no machine fix: {t}");
    }
}
