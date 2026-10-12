//! Diagnostics quality: code registry completeness, explain examples that really fire,
//! golden snapshots of the rendered messages for each case in tests/diagnostics/cases, JSON
//! schema shape, and a false-positive sweep of `wj check` over the build corpus.
//!
//! Regenerate goldens with `UPDATE_GOLDEN=1 cargo test --release --test diagnostics_suite`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use windjammer::check::check_sources;
use windjammer::diagnostics::{codes, render, Diagnostic, Report};

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn check_one(name: &str, text: &str) -> Vec<Diagnostic> {
    check_sources(Path::new("."), &[(name.to_string(), text.to_string())])
}

fn render_all(name: &str, text: &str, diags: &[Diagnostic]) -> String {
    let src = |f: &str| (f == name).then(|| text.to_string());
    diags.iter().map(|d| render::render(d, &src)).collect::<Vec<_>>().join("\n")
}

// ---------- registry ----------

#[test]
fn every_code_has_a_complete_explain_entry() {
    let mut seen = BTreeSet::new();
    for c in codes::all() {
        assert!(seen.insert(c.code), "duplicate code {}", c.code);
        for (field, v) in [
            ("title", c.title),
            ("explanation", c.explanation),
            ("cause", c.cause),
            ("fix", c.fix),
            ("example_bad", c.example_bad),
            ("example_good", c.example_good),
        ] {
            assert!(v.trim().len() > 8, "{}: empty or trivial `{field}`", c.code);
        }
        let text = codes::explain(c.code).expect("explain");
        assert!(text.contains("Example (wrong)") && text.contains("Fix") && text.contains("Cause"));
    }
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// A code literal anywhere in `src/` (outside the registry itself and tests) must be registered.
#[test]
fn no_unregistered_code_literals_in_source() {
    let mut files = Vec::new();
    collect_rs(&manifest().join("src"), &mut files);
    let mut missing = BTreeSet::new();
    for f in files {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        if name.starts_with("codes") || name == "error_codes.rs" {
            continue;
        }
        let text = fs::read_to_string(&f).unwrap();
        let body = text.split("#[cfg(test)]").next().unwrap_or(&text);
        for q in body.split('"').skip(1).step_by(2) {
            let b = q.as_bytes();
            let wj = q.len() == 6 && q.starts_with("WJ") && b[2..].iter().all(u8::is_ascii_digit);
            let w = q.len() == 5 && q.starts_with('W') && b[1..].iter().all(u8::is_ascii_digit);
            if (wj || w) && codes::lookup(q).is_none() {
                missing.insert(format!("{q} ({})", f.strip_prefix(manifest()).unwrap().display()));
            }
        }
    }
    assert!(missing.is_empty(), "codes without an explain entry: {missing:?}");
}

#[test]
fn front_end_examples_fire_their_code_and_fixes_do_not() {
    for c in codes::all().filter(|c| c.front_end_checked) {
        let bad = check_one("ex.wj", c.example_bad);
        assert!(
            bad.iter().any(|d| d.code == c.code),
            "{}: example_bad does not produce the code; got {:?}",
            c.code,
            bad.iter().map(|d| d.code.clone()).collect::<Vec<_>>()
        );
        let good = check_one("ex.wj", c.example_good);
        assert!(
            !good.iter().any(|d| d.code == c.code),
            "{}: example_good still produces the code",
            c.code
        );
    }
}

#[test]
fn rustc_codes_map_to_registered_codes() {
    for rust in ["E0425", "E0308", "E0382", "E0499", "E0502", "E0599", "E0277", "E0004", "E0384", "E0596"] {
        assert!(codes::from_rust_code(rust).is_some(), "{rust} has no WJ code");
    }
}

// ---------- cases + goldens ----------

fn cases() -> Vec<(String, PathBuf)> {
    let dir = manifest().join("tests/diagnostics/cases");
    let mut v: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "wj"))
        .map(|p| (p.file_stem().unwrap().to_string_lossy().to_string(), p))
        .collect();
    v.sort();
    v
}

fn expectation(text: &str) -> String {
    text.lines()
        .next()
        .and_then(|l| l.strip_prefix("// expect: "))
        .expect("case needs a `// expect:` header")
        .trim()
        .to_string()
}

#[test]
fn front_end_cases_report_the_expected_code_with_a_span() {
    let mut bad = Vec::new();
    for (name, path) in cases() {
        let text = fs::read_to_string(&path).unwrap();
        let expect = expectation(&text);
        if expect.starts_with("rustc:") {
            continue;
        }
        let diags = check_one(&format!("{name}.wj"), &text);
        if expect == "none" {
            if !diags.is_empty() {
                bad.push(format!("{name}: expected clean, got {:?}", diags.iter().map(|d| &d.code).collect::<Vec<_>>()));
            }
            continue;
        }
        match diags.iter().find(|d| d.code == expect) {
            None => bad.push(format!("{name}: expected {expect}, got {:?}", diags.iter().map(|d| &d.code).collect::<Vec<_>>())),
            Some(d) => {
                let Some(sp) = &d.primary_span else {
                    bad.push(format!("{name}: {expect} has no primary span"));
                    continue;
                };
                let slice = text.get(sp.byte_start..sp.byte_end).unwrap_or("<bad range>");
                if slice.is_empty() || sp.line_start < 2 {
                    bad.push(format!("{name}: {expect} span is empty or on the header line: {sp:?}"));
                }
            }
        }
    }
    assert!(bad.is_empty(), "case failures:\n{}", bad.join("\n"));
}

#[test]
fn golden_messages() {
    let update = std::env::var("UPDATE_GOLDEN").is_ok();
    let mut bad = Vec::new();
    for (name, path) in cases() {
        let text = fs::read_to_string(&path).unwrap();
        if expectation(&text).starts_with("rustc:") {
            continue;
        }
        let file = format!("{name}.wj");
        let got = render_all(&file, &text, &check_one(&file, &text));
        let golden = manifest().join(format!("tests/diagnostics/golden/{name}.txt"));
        if update {
            fs::write(&golden, &got).unwrap();
            continue;
        }
        match fs::read_to_string(&golden) {
            Ok(want) if want == got => {}
            Ok(want) => bad.push(format!("{name}:\n--- want\n{want}\n--- got\n{got}")),
            Err(_) => bad.push(format!("{name}: missing golden (run with UPDATE_GOLDEN=1)")),
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

// ---------- schema ----------

#[test]
fn json_report_has_the_documented_shape() {
    let text = "fn main() {\n    println!(\"hi {}\", 1)\n    let x: int = \"a\"\n}\n";
    let diags = check_one("s.wj", text);
    let report = Report::new("check", diags);
    let v: serde_json::Value = serde_json::from_str(&report.to_json()).unwrap();
    assert_eq!(v["schema_version"], 2);
    assert_eq!(v["command"], "check");
    let schema: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(manifest().join("docs/diagnostics-schema.json")).unwrap()).unwrap();
    let required: Vec<&str> = schema["$defs"]["diagnostic"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    for d in v["diagnostics"].as_array().unwrap() {
        for r in &required {
            assert!(d.get(r).is_some(), "diagnostic lacks required `{r}`: {d}");
        }
        assert!(d["docs"]["explain"].as_str().unwrap().starts_with("wj explain "));
    }
    let w = v["diagnostics"].as_array().unwrap().iter().find(|d| d["fixes"].as_array().is_some_and(|f| !f.is_empty())).expect("a diagnostic with a fix");
    assert_eq!(w["fixes"][0]["applicability"], "machine_applicable");
    assert!(w["fixes"][0]["edits"][0]["byte_start"].is_number());
}

#[test]
fn fixes_apply_and_leave_a_clean_file() {
    use windjammer::diagnostics::apply;
    let text = "fn main() {\n    let x = 1\n    println!(\"x = {}\", x)\n    let v = vec![1, 2]\n    println(\"${v.len()}\")\n}\n";
    let diags = check_one("f.wj", text);
    let edits = apply::collect_edits(&diags);
    let fixed = apply::apply_edits(text, &edits["f.wj"]).unwrap();
    let after = check_one("f.wj", &fixed);
    assert!(after.iter().all(|d| d.code != "W0007" && d.code != "W0008"), "left: {after:?}\n{fixed}");
    assert!(fixed.contains("println(\"x = ${x}\")"), "{fixed}");
}

// ---------- false-positive sweep ----------

fn known_failures() -> BTreeSet<String> {
    fs::read_to_string(manifest().join("tests/wj_corpus_known_failures.txt"))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with('#') && l.contains('|'))
        .map(|l| l.split('|').next().unwrap().trim().to_string())
        .collect()
}

/// Every corpus file the real build accepts must pass `wj check` without an error from the
/// semantic checks (lint/parse errors already match the build, which runs them too).
#[test]
fn check_has_no_false_positives_on_the_build_corpus() {
    let known = known_failures();
    let mut files = Vec::new();
    for dir in ["", "conformance", "conformance/basics", "conformance/control_flow", "conformance/error_handling", "conformance/stdlib", "conformance/types", "conformance/values", "windjammer_tests", "auto_clone", "dogfood", "fixtures", "fixtures/traits", "fixtures/library_multipass", "@verified"] {
        let d = if dir == "@verified" { manifest().join("examples/verified") } else { manifest().join("tests").join(dir) };
        for e in fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "wj") {
                files.push(p);
            }
        }
    }
    files.sort();
    let semantic = [
        "WJ0001", "WJ0002", "WJ0003", "WJ0004", "WJ0005", "WJ0006", "WJ0008", "WJ0009", "WJ0013",
        "WJ0014", "WJ0021",
    ];
    // Corpus files the build accepts although they contain a real error that only rustc reports.
    let true_positives = [
        ("tests/bug_vec_index_passed_to_function.wj", "WJ0009: `Node { value: 10 }` omits field `data`"),
    ];
    let mut fps = Vec::new();
    for f in &files {
        let rel = f.strip_prefix(manifest()).unwrap().display().to_string();
        if known.contains(&rel) || true_positives.iter().any(|(p, _)| *p == rel) {
            continue;
        }
        let text = fs::read_to_string(f).unwrap();
        let name = rel.clone();
        for d in check_one(&name, &text) {
            if d.is_error() && semantic.contains(&d.code.as_str()) {
                let l = d.primary_span.as_ref().map_or(0, |s| s.line_start);
                fps.push(format!("{rel}:{l}: {} {}", d.code, d.message));
            }
        }
    }
    assert!(fps.is_empty(), "{} false positives:\n{}", fps.len(), fps.join("\n"));
}
