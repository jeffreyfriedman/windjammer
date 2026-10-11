//! Header parsing and manifest generation for the verified example corpus
//! (`examples/verified/*.wj`). Headers are `// key: value` comment lines at the top of each file.

use anyhow::Result;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

/// Whether the compiled-Rust path is expected to reproduce the example's output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustStatus {
    /// `// rust: ok` — must compile with `wj build --no-cargo` + rustc and print the same output.
    Ok,
    /// `// rust: gap <reason>` — a known compiler gap, recorded rather than hidden.
    Gap(String),
    /// No `// rust:` header (test files).
    NotApplicable,
}

#[derive(Debug, Clone)]
pub struct ExampleHeader {
    pub name: String,
    pub tags: Vec<String>,
    pub expect: Vec<String>,
    pub exit: i32,
    pub rust: RustStatus,
    /// `// tests-passed: N` for `*_test.wj` files.
    pub tests_passed: Option<usize>,
    pub tests_failed: Option<usize>,
}

impl ExampleHeader {
    pub fn is_test(&self) -> bool {
        self.name.ends_with("_test")
    }
}

pub fn parse_header(name: &str, src: &str) -> ExampleHeader {
    let mut h = ExampleHeader {
        name: name.to_string(),
        tags: vec![],
        expect: vec![],
        exit: 0,
        rust: RustStatus::NotApplicable,
        tests_passed: None,
        tests_failed: None,
    };
    for line in src.lines() {
        let Some(rest) = line.strip_prefix("//") else { continue };
        let rest = rest.strip_prefix(' ').unwrap_or(rest);
        if let Some(v) = line.strip_prefix("// expect:") {
            h.expect.push(v.strip_prefix(' ').unwrap_or(v).to_string());
        } else if let Some(v) = rest.strip_prefix("tags:") {
            h.tags = v.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
        } else if let Some(v) = rest.strip_prefix("exit:") {
            h.exit = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = rest.strip_prefix("rust:") {
            let v = v.trim();
            h.rust = if v == "ok" {
                RustStatus::Ok
            } else {
                RustStatus::Gap(v.strip_prefix("gap").unwrap_or(v).trim().to_string())
            };
        } else if let Some(v) = rest.strip_prefix("tests-passed:") {
            h.tests_passed = v.trim().parse().ok();
        } else if let Some(v) = rest.strip_prefix("tests-failed:") {
            h.tests_failed = v.trim().parse().ok();
        }
    }
    h
}

/// All `.wj` examples in `dir`, sorted by file name.
pub fn load_examples(dir: &Path) -> Result<Vec<(ExampleHeader, String)>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    let mut paths: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("wj"))
        .collect();
    paths.sort();
    for p in paths {
        let src = fs::read_to_string(&p)?;
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        out.push((parse_header(&stem, &src), src));
    }
    Ok(out)
}

pub fn build_manifest(dir: &Path) -> Result<Value> {
    let examples = load_examples(dir)?;
    let entries: Vec<Value> = examples
        .iter()
        .map(|(h, _)| {
            let (rust, rust_note) = match &h.rust {
                RustStatus::Ok => ("ok", String::new()),
                RustStatus::Gap(r) => ("gap", r.clone()),
                RustStatus::NotApplicable => ("n/a", String::new()),
            };
            let mut v = json!({
                "file": format!("{}.wj", h.name),
                "kind": if h.is_test() { "test" } else { "program" },
                "tags": h.tags,
                "rust": rust,
                "expect_lines": h.expect.len(),
            });
            if !rust_note.is_empty() {
                v["rust_gap"] = json!(rust_note);
            }
            if h.exit != 0 {
                v["exit"] = json!(h.exit);
            }
            v
        })
        .collect();
    Ok(json!({
        "format": "each program starts with `// tags:`, `// rust:` and `// expect:` lines; see README.md",
        "count": entries.len(),
        "examples": entries,
    }))
}
