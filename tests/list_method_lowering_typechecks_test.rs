//! Every documented list/option method, written as a function body over a list parameter, lowers to Rust that
//! passes rustc's type check (`--emit=metadata`). One table; a row that fails must be in `KNOWN_GAPS`, which
//! may only shrink (a gap that now passes fails the test until its entry is removed).

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

use std::process::Command;
use tempfile::TempDir;

/// (name, return type, body) over `items: Vec<Item>`, `nums: Vec<int>`, `id: string`.
const ROWS: &[(&str, &str, &str)] = &[
    ("find_struct", "Option<Item>", "items.find(|i| i.id == id)"),
    ("find_int", "Option<int>", "nums.find(|n| n > 1)"),
    ("first_struct", "Option<Item>", "items.first()"),
    ("first_int", "Option<int>", "nums.first()"),
    ("last_struct", "Option<Item>", "items.last()"),
    ("last_int", "Option<int>", "nums.last()"),
    ("get_struct", "Option<Item>", "items.get(0)"),
    ("get_int", "Option<int>", "nums.get(0)"),
    ("position", "Option<int>", "items.position(|i| i.id == id)"),
    ("any", "bool", "items.any(|i| i.id == id)"),
    ("all", "bool", "items.all(|i| i.part > 0)"),
    ("contains", "bool", "nums.contains(2)"),
    ("filter", "Vec<Item>", "items.filter(|i| i.part > 1).collect()"),
    ("map", "Vec<int>", "items.map(|i| i.part).collect()"),
    ("count", "int", "nums.filter(|n| n > 1).count()"),
    ("len", "int", "nums.len()"),
    ("is_empty", "bool", "items.is_empty()"),
    ("sum", "int", "nums.sum()"),
    ("max", "Option<int>", "nums.max()"),
    ("min", "Option<int>", "nums.min()"),
];

/// Rows that do not lower to type-correct Rust yet. Shrink-only.
///
/// - `max` / `min`: a list has no zero-argument `max()`; the lowering calls Rust's `Ord::max(self, other)` (E0061).
///   Waiting on the std API review: give lists `max() -> Option<T>`, or take them out of the documented surface.
const KNOWN_GAPS: &[&str] = &["max", "min"];

fn source(ret: &str, body: &str) -> String {
    format!(
        "pub struct Item {{\n    pub id: string,\n    pub part: int,\n}}\n\nfn t(items: Vec<Item>, nums: Vec<int>, id: string) -> {ret} {{\n    {body}\n}}\n\nfn main() {{\n    let items = [Item {{ id: \"a\", part: 1 }}, Item {{ id: \"b\", part: 2 }}]\n    let nums = [3, 1, 2]\n    let r = t(items, nums, \"b\")\n}}\n"
    )
}

/// `None` when the lowering type-checks, else the first error line.
fn typecheck(name: &str, src: &str) -> Option<String> {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join(format!("{name}.wj"));
    std::fs::write(&file, src).unwrap();
    let out_dir = dir.path().join("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", file.to_str().unwrap(), "--target", "rust", "--no-cargo", "-o", out_dir.to_str().unwrap()])
        .env("WJ_ALLOW_TOOLCHAIN_MISMATCH", "1")
        .output()
        .unwrap();
    if !build.status.success() {
        let text = format!("{}{}", String::from_utf8_lossy(&build.stdout), String::from_utf8_lossy(&build.stderr));
        return Some(format!("wj build failed: {}", text.lines().find(|l| l.contains("error")).unwrap_or("")));
    }
    let rs = out_dir.join(format!("{name}.rs"));
    let rustc = Command::new("rustc")
        .args(["--edition", "2021", "--emit=metadata", "-o"])
        .arg(dir.path().join("x.rmeta"))
        .arg(&rs)
        .output()
        .expect("rustc on PATH");
    if rustc.status.success() {
        return None;
    }
    let err = String::from_utf8_lossy(&rustc.stderr);
    Some(err.lines().find(|l| l.starts_with("error")).unwrap_or("rustc failed").to_string())
}

#[test]
fn every_list_method_lowers_to_type_correct_rust() {
    let mut unexpected = Vec::new();
    let mut fixed = Vec::new();
    for (name, ret, body) in ROWS {
        let result = typecheck(name, &source(ret, body));
        let gap = KNOWN_GAPS.contains(name);
        match (result, gap) {
            (Some(e), false) => unexpected.push(format!("{name}: `{body}` -> {e}")),
            (None, true) => fixed.push(*name),
            _ => {}
        }
    }
    assert!(unexpected.is_empty(), "list methods that lower to Rust that does not type-check:\n{}", unexpected.join("\n"));
    assert!(fixed.is_empty(), "now passing; remove from KNOWN_GAPS: {fixed:?}");
}
