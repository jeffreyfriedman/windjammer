//! The machine-readable diagnostics contract (`wj check --json`, schema v2):
//!  1. every report is validated against docs/diagnostics-schema.json by an independent validator
//!     (types, required, enums, patterns, $ref, allOf/oneOf), not just "has these keys";
//!  2. the schema version is pinned: `$id`, `schema_version` and the goldens move together;
//!  3. golden JSON snapshots (tests/diagnostics/golden_json/*.json) of reports whose diagnostics carry
//!     `fixes` (span + replacement), so any change of the wire format is a reviewed diff;
//!  4. GATE: applying any single fix found over the corpus never produces a NEW diagnostic.
//!
//! Regenerate goldens: `UPDATE_GOLDEN=1 cargo test --release --test diag_schema_suite`.
//! Known fix regressions live in tests/diag_hints_ratchet/fix_new_diagnostic.txt and may only shrink.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use windjammer::check::check_sources;
use windjammer::diagnostics::{apply, Diagnostic, Report, SCHEMA_VERSION};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn check_one(name: &str, text: &str) -> Vec<Diagnostic> {
    check_sources(Path::new("."), &[(name.to_string(), text.to_string())])
}

fn schema() -> Value {
    serde_json::from_str(&fs::read_to_string(root().join("docs/diagnostics-schema.json")).unwrap()).unwrap()
}

// ---------- a small JSON-Schema validator for the subset the schema uses ----------

fn resolve<'a>(root: &'a Value, r: &str) -> &'a Value {
    let mut v = root;
    for part in r.trim_start_matches("#/").split('/') {
        v = &v[part];
    }
    v
}

fn validate(root: &Value, schema: &Value, v: &Value, path: &str, errs: &mut Vec<String>) {
    if let Some(r) = schema.get("$ref").and_then(Value::as_str) {
        return validate(root, resolve(root, r), v, path, errs);
    }
    if let Some(all) = schema.get("allOf").and_then(Value::as_array) {
        all.iter().for_each(|s| validate(root, s, v, path, errs));
    }
    if let Some(one) = schema.get("oneOf").and_then(Value::as_array) {
        let ok = one.iter().filter(|s| {
            let mut e = Vec::new();
            validate(root, s, v, path, &mut e);
            e.is_empty()
        });
        if ok.count() != 1 {
            errs.push(format!("{path}: does not match exactly one alternative"));
        }
    }
    if let Some(c) = schema.get("const") {
        if c != v {
            errs.push(format!("{path}: expected constant {c}, got {v}"));
        }
    }
    if let Some(en) = schema.get("enum").and_then(Value::as_array) {
        if !en.contains(v) {
            errs.push(format!("{path}: {v} not in enum {en:?}"));
        }
    }
    if let Some(t) = schema.get("type").and_then(Value::as_str) {
        let ok = match t {
            "object" => v.is_object(),
            "array" => v.is_array(),
            "string" => v.is_string(),
            "boolean" => v.is_boolean(),
            "integer" => v.is_i64() || v.is_u64(),
            "null" => v.is_null(),
            _ => true,
        };
        if !ok {
            errs.push(format!("{path}: expected {t}, got {v}"));
            return;
        }
    }
    if let (Some(m), Some(n)) = (schema.get("minimum").and_then(Value::as_i64), v.as_i64()) {
        if n < m {
            errs.push(format!("{path}: {n} < minimum {m}"));
        }
    }
    if let (Some(p), Some(s)) = (schema.get("pattern").and_then(Value::as_str), v.as_str()) {
        // The only pattern in the schema: ^(WJ|W)[0-9]{4}$
        assert_eq!(p, "^(WJ|W)[0-9]{4}$", "validator knows one pattern; extend it");
        let digits = s.trim_start_matches("WJ").trim_start_matches('W');
        if !(s.starts_with('W') && digits.len() == 4 && digits.chars().all(|c| c.is_ascii_digit())) {
            errs.push(format!("{path}: `{s}` does not match {p}"));
        }
    }
    if let Some(obj) = v.as_object() {
        for r in schema.get("required").and_then(Value::as_array).into_iter().flatten() {
            if !obj.contains_key(r.as_str().unwrap()) {
                errs.push(format!("{path}: missing required `{}`", r.as_str().unwrap()));
            }
        }
        for (k, s) in schema.get("properties").and_then(Value::as_object).into_iter().flatten() {
            if let Some(child) = obj.get(k) {
                validate(root, s, child, &format!("{path}.{k}"), errs);
            }
        }
    }
    if let (Some(items), Some(arr)) = (schema.get("items"), v.as_array()) {
        if let Some(min) = schema.get("minItems").and_then(Value::as_u64) {
            if (arr.len() as u64) < min {
                errs.push(format!("{path}: fewer than {min} items"));
            }
        }
        for (i, x) in arr.iter().enumerate() {
            validate(root, items, x, &format!("{path}[{i}]"), errs);
        }
    }
}

fn report_json(diags: Vec<Diagnostic>) -> Value {
    serde_json::from_str(&Report::new("check", diags).to_json()).unwrap()
}

fn assert_valid(v: &Value, what: &str) {
    let s = schema();
    let mut errs = Vec::new();
    validate(&s, &s, v, "$", &mut errs);
    assert!(errs.is_empty(), "{what} violates docs/diagnostics-schema.json:\n  {}", errs.join("\n  "));
}

// ---------- corpus ----------

fn wj_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            wj_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "wj") {
            out.push(p);
        }
    }
}

/// `(name, source)`: diagnostics cases, the annotated oracle programs, the recorded pilot sources.
fn corpus() -> Vec<(String, String)> {
    let mut files = Vec::new();
    wj_files(&root().join("tests/diagnostics/cases"), &mut files);
    wj_files(&root().join("tests/diagnostics_oracle"), &mut files);
    let mut out: Vec<(String, String)> = files
        .iter()
        .filter_map(|p| Some((p.strip_prefix(root()).unwrap().to_string_lossy().to_string(), fs::read_to_string(p).ok()?)))
        .collect();
    let results = root().join("bench/agent-dx/results");
    let mut dirs: Vec<_> = fs::read_dir(&results).map(|rd| rd.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    dirs.sort();
    let mut seen = BTreeSet::new();
    for d in dirs {
        let Ok(rd) = fs::read_dir(d.join("trials")) else { continue };
        let mut fs_: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        fs_.sort();
        for f in fs_ {
            let Ok(v) = serde_json::from_str::<Value>(&fs::read_to_string(&f).unwrap()) else { continue };
            if v["lang"] != "wj" {
                continue;
            }
            for key in ["first_source", "final_source"] {
                if let Some(s) = v[key].as_str() {
                    if seen.insert(s.to_string()) {
                        out.push((format!("pilot:{}:{key}", f.file_stem().unwrap().to_string_lossy()), s.to_string()));
                    }
                }
            }
        }
    }
    out
}

// ---------- 1 + 2: schema ----------

#[test]
fn schema_version_is_pinned_and_consistent() {
    let s = schema();
    assert_eq!(SCHEMA_VERSION, 2, "bumping the schema is a breaking change: update goldens, $id and docs together");
    assert!(s["$id"].as_str().unwrap().ends_with(&format!("/v{SCHEMA_VERSION}.json")), "$id: {}", s["$id"]);
    assert_eq!(s["properties"]["schema_version"]["const"], SCHEMA_VERSION);
}

#[test]
fn every_corpus_report_validates_against_the_schema() {
    let mut n = 0;
    let mut with_fix = 0;
    for (name, src) in corpus() {
        let v = report_json(check_one(&format!("{}.wj", name.replace([':', '/'], "_")), &src));
        assert_valid(&v, &name);
        n += 1;
        with_fix += v["diagnostics"].as_array().unwrap().iter().any(|d| d.get("fixes").is_some()) as usize;
    }
    assert!(n > 100, "corpus too small: {n}");
    assert!(with_fix >= 5, "too few reports carry a fix: {with_fix}");
    println!("schema: {n} corpus reports valid, {with_fix} carry fixes");
}

#[test]
fn validator_rejects_a_malformed_report() {
    let mut v = report_json(check_one("x.wj", "fn main() {\n    let x: int = \"a\"\n}\n"));
    v["diagnostics"][0]["code"] = Value::String("E0308".into());
    v["diagnostics"][0]["fixes"] = serde_json::json!([{ "description": "d", "applicability": "sure", "edits": [] }]);
    let s = schema();
    let mut errs = Vec::new();
    validate(&s, &s, &v, "$", &mut errs);
    let all = errs.join("\n");
    assert!(all.contains("does not match") && all.contains("not in enum") && all.contains("fewer than 1"), "{all}");
}

// ---------- 3: golden JSON ----------

const GOLDEN_CASES: &[(&str, &str)] = &[
    ("macro_forms_fix", "fn main() {\n    println!(\"hi {}\", 1)\n    let v = vec![1, 2]\n    println(\"${v.len()}\")\n}\n"),
    ("literal_mismatch_fix", "fn main() {\n    let y: string = 42\n    println(\"y = ${y}\")\n}\n"),
    ("unwrap_lint", "fn main() {\n    let v: Option<int> = Some(1)\n    println(\"${v.unwrap()}\")\n}\n"),
    ("clean", "fn main() {\n    println(\"hello\")\n}\n"),
];

fn normalise(mut v: Value) -> Value {
    v["windjammer_version"] = Value::String("VERSION".into());
    v["summary"]["elapsed_ms"] = Value::from(0);
    v
}

#[test]
fn json_goldens_are_stable() {
    let dir = root().join("tests/diagnostics/golden_json");
    let mut bad = Vec::new();
    for (name, src) in GOLDEN_CASES {
        let v = normalise(report_json(check_one(&format!("{name}.wj"), src)));
        assert_valid(&v, name);
        let got = serde_json::to_string_pretty(&v).unwrap() + "\n";
        let path = dir.join(format!("{name}.json"));
        if std::env::var("UPDATE_GOLDEN").is_ok_and(|x| x == "1") {
            fs::create_dir_all(&dir).unwrap();
            fs::write(&path, &got).unwrap();
            continue;
        }
        match fs::read_to_string(&path) {
            Ok(want) if want == got => {}
            Ok(_) => bad.push(format!("{name}: JSON differs from {} (review, then UPDATE_GOLDEN=1)", path.display())),
            Err(_) => bad.push(format!("{name}: missing golden (UPDATE_GOLDEN=1)")),
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn goldens_with_fixes_carry_span_and_replacement() {
    let v = report_json(check_one("m.wj", GOLDEN_CASES[0].1));
    let fixes: Vec<&Value> = v["diagnostics"].as_array().unwrap().iter().flat_map(|d| d["fixes"].as_array().into_iter().flatten()).collect();
    assert!(!fixes.is_empty());
    for f in fixes {
        assert_eq!(f["applicability"], "machine_applicable");
        let e = &f["edits"][0];
        assert!(e["replacement"].is_string() && e["byte_start"].is_u64() && e["byte_end"].is_u64() && e["line_start"].is_u64(), "{e}");
    }
}

// ---------- 4: applying a fix never produces a new diagnostic ----------

fn code_counts(ds: &[Diagnostic]) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for d in ds {
        *m.entry(d.code.clone()).or_default() += 1;
    }
    m
}

#[test]
fn applying_any_corpus_fix_never_produces_a_new_diagnostic() {
    let mut bad = BTreeSet::new();
    let (mut fixes, mut files_with_fixes) = (0, 0);
    for (name, src) in corpus() {
        let fname = format!("{}.wj", name.replace([':', '/'], "_"));
        let before = check_one(&fname, &src);
        let before_counts = code_counts(&before);
        let mut any = false;
        for d in &before {
            for fix in &d.fixes {
                any = true;
                fixes += 1;
                let Some(fixed) = apply::apply_edits(&src, &fix.edits) else {
                    bad.insert(format!("{name}: {} fix edits do not apply", d.code));
                    continue;
                };
                let after = code_counts(&check_one(&fname, &fixed));
                for (code, n) in &after {
                    if *n > before_counts.get(code).copied().unwrap_or(0) {
                        bad.insert(format!("{name}: fix for {} introduces {code}", d.code));
                    }
                }
            }
        }
        files_with_fixes += any as usize;
    }
    assert!(fixes >= 10, "corpus exercised too few fixes: {fixes}");
    println!("fix gate: {fixes} fixes over {files_with_fixes} files, {} violations", bad.len());
    let p = root().join("tests/diag_hints_ratchet/fix_new_diagnostic.txt");
    let allowed: BTreeSet<String> = fs::read_to_string(&p).unwrap_or_default().lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(str::to_string).collect();
    if std::env::var("DIAG_HINTS_BLESS").is_ok_and(|v| v == "1") && bad.is_subset(&allowed) {
        let body: String = bad.iter().map(|l| format!("{l}\n")).collect();
        fs::write(&p, format!("# Fixes that introduce a new diagnostic. May only shrink.\n{body}")).unwrap();
        return;
    }
    let new: Vec<_> = bad.difference(&allowed).collect();
    let stale: Vec<_> = allowed.difference(&bad).collect();
    assert!(new.is_empty(), "NEW fix regressions (fix the fix; do not add to the ratchet): {new:#?}");
    assert!(stale.is_empty(), "STALE entries (delete them): {stale:#?}");
}
