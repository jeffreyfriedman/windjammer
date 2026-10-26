//! Gate 5b: source maps (`*.rs.map`) must point generated Rust lines at the right `.wj` token.
//!
//! Oracle (independent of the generator): for every mapping, the Rust line exists; the `.wj`
//! (line, column) is the start of a lexed token of the `.wj` source (checked with the naive
//! lexer-stream oracle; codegen may reorder functions, so no monotonicity is assumed); and the first word of the mapped statement (`let x`, `for`, `foo(`) occurs on the
//! mapped Rust line.

use super::span_oracle::corpus::*;
use super::span_oracle::lexer_oracle::lex_all;
use super::span_oracle::ratchet;
use std::collections::BTreeSet;
use windjammer::{build_project_with_config, CompilationTarget};

#[derive(Debug, Clone)]
pub struct Map {
    pub rust_line: usize,
    pub rust_column: usize,
    pub wj_line: usize,
    pub wj_column: usize,
}

pub fn compile_with_map(wj_src: &str) -> Result<(String, Vec<Map>), String> {
    let tmp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let src_dir = tmp.path().join("src");
    let out = tmp.path().join("build");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("main.wj"), wj_src).unwrap();
    let r = std::panic::catch_unwind(|| {
        build_project_with_config(&src_dir, &out, CompilationTarget::Rust, false, false, &[], &windjammer::compiler_config::CompilerConfig::default())
    });
    match r {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(format!("compile error: {e}")),
        Err(_) => return Err("compiler panicked".into()),
    }
    let rs = std::fs::read_to_string(out.join("main.rs")).map_err(|e| format!("no main.rs: {e}"))?;
    let map_text = std::fs::read_to_string(out.join("main.rs.map")).map_err(|e| format!("no main.rs.map: {e}"))?;
    let json: serde_json::Value = serde_json::from_str(&map_text).map_err(|e| e.to_string())?;
    let maps = json["mappings"]
        .as_array()
        .ok_or("no mappings array")?
        .iter()
        .map(|m| Map {
            rust_line: m["rust_line"].as_u64().unwrap_or(0) as usize,
            rust_column: m["rust_column"].as_u64().unwrap_or(0) as usize,
            wj_line: m["wj_line"].as_u64().unwrap_or(0) as usize,
            wj_column: m["wj_column"].as_u64().unwrap_or(0) as usize,
        })
        .collect();
    Ok((rs, maps))
}

/// Verify mappings against the two texts. Returns violations.
pub fn verify(wj_src: &str, rust_src: &str, maps: &[Map]) -> Vec<String> {
    let mut v = Vec::new();
    let tokens = lex_all(wj_src).unwrap_or_default();
    let starts: BTreeSet<(usize, usize)> = tokens.iter().map(|t| (t.line, t.column)).collect();
    let rust_lines: Vec<&str> = rust_src.lines().collect();
    let wj_lines: Vec<&str> = wj_src.lines().collect();
    let mut sorted: Vec<&Map> = maps.iter().collect();
    sorted.sort_by_key(|m| m.rust_line);
    for m in &sorted {
        let at = format!("rust line {} -> wj {}:{}", m.rust_line, m.wj_line, m.wj_column);
        if m.rust_line == 0 || m.rust_line > rust_lines.len() {
            v.push(format!("{at}: rust line outside generated file ({} lines)", rust_lines.len()));
            continue;
        }
        if !starts.contains(&(m.wj_line, m.wj_column)) {
            v.push(format!("{at}: not the start of any .wj token"));
            continue;
        }
        // First word of the statement must occur on the mapped Rust line.
        let wj_line = wj_lines.get(m.wj_line - 1).copied().unwrap_or("");
        let rest: String = wj_line.chars().skip(m.wj_column - 1).collect();
        let mut words = rest.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty());
        let first = words.next().unwrap_or("");
        let key = match first {
            "let" => words.next().filter(|w| *w != "mut").or_else(|| words.next()).unwrap_or("let"),
            "if" | "while" | "for" | "match" | "loop" => first,
            "return" => continue, // a tail `return x` may be emitted without the keyword
            w => w,
        };
        let rl = rust_lines[m.rust_line - 1];
        let ok = rl.contains(key) || (first == "while" && rl.contains("loop")) || (first == "match" && rl.contains("if let"));
        if !ok {
            v.push(format!("{at}: statement starts with `{key}` but Rust line is {:?}", rl.trim()));
        }
    }
    v
}

fn sample_programs() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut files: Vec<_> = std::fs::read_dir(repo_root().join("examples/verified")).unwrap().flatten().map(|e| e.path()).collect();
    files.sort();
    for p in files {
        if p.extension().is_some_and(|e| e == "wj") {
            let src = std::fs::read_to_string(&p).unwrap();
            if src.contains("// rust: ok") && !src.contains("mod ") {
                out.push((rel(&p), src));
            }
        }
        if out.len() >= 25 {
            break;
        }
    }
    out
}

const UNICODE_CRLF: &str = "fn greet(name: string) -> string {\r\n    let msg = \"héllo 日本😀\"\r\n    msg\r\n}\r\n\r\nfn main() {\r\n    let x = greet(\"é\")\r\n    println(x)\r\n}\r\n";

#[test]
fn source_maps_point_at_the_right_wj_tokens() {
    let mut programs = sample_programs();
    programs.push(("<inline unicode+CRLF>".into(), UNICODE_CRLF.into()));
    let mut compile_failures = BTreeSet::new();
    let mut reasons = Vec::new();
    let mut bad = Vec::new();
    let mut total_maps = 0;
    for (name, src) in &programs {
        match compile_with_map(src) {
            Err(e) => {
                reasons.push(format!("{name}: {e}"));
                compile_failures.insert(name.clone());
            }
            Ok((rs, maps)) => {
                total_maps += maps.len();
                if maps.is_empty() {
                    bad.push(format!("{name}: compiled but the source map is empty"));
                }
                let v = verify(src, &rs, &maps);
                if !v.is_empty() {
                    bad.push(format!("{name}:\n    {}", v.iter().take(6).cloned().collect::<Vec<_>>().join("\n    ")));
                }
            }
        }
    }
    assert!(total_maps >= programs.len(), "suspiciously few mappings: {total_maps}; compile failures: {:#?}", reasons.iter().take(5).collect::<Vec<_>>());
    assert!(bad.is_empty(), "{}", bad.join("\n"));
    ratchet::enforce("sourcemap_compile_failures", &compile_failures);
}

// ---- mutation self-tests ----------------------------------------------------------------------

const MUT_WJ: &str = "fn main() {\n    let count = 1\n    println(count)\n}\n";
const MUT_RS: &str = "fn main() {\n    let count = 1;\n    println!(\"{}\", count);\n}\n";

fn good_maps() -> Vec<Map> {
    vec![
        Map { rust_line: 2, rust_column: 5, wj_line: 2, wj_column: 5 },
        Map { rust_line: 3, rust_column: 5, wj_line: 3, wj_column: 5 },
    ]
}

#[test]
fn verifier_accepts_correct_map() {
    assert_eq!(verify(MUT_WJ, MUT_RS, &good_maps()), Vec::<String>::new());
}
#[test]
fn verifier_rejects_wrong_rust_line() {
    let mut m = good_maps();
    m[0].rust_line = 3;
    m[1].rust_line = 2;
    assert!(!verify(MUT_WJ, MUT_RS, &m).is_empty());
}
#[test]
fn verifier_rejects_rust_line_outside_file() {
    let mut m = good_maps();
    m[1].rust_line = 99;
    assert!(!verify(MUT_WJ, MUT_RS, &m).is_empty());
}
#[test]
fn verifier_rejects_wj_column_off_by_one() {
    let mut m = good_maps();
    m[0].wj_column += 1;
    assert!(!verify(MUT_WJ, MUT_RS, &m).is_empty());
}
#[test]
fn verifier_rejects_wj_line_off_by_one() {
    let mut m = good_maps();
    m[0].wj_line = 3;
    m[0].wj_column = 5;
    assert!(!verify(MUT_WJ, MUT_RS, &m).is_empty());
}
#[test]
fn verifier_rejects_position_that_is_not_a_token_start() {
    let mut m = good_maps();
    m[0].wj_column = 1; // leading whitespace
    assert!(!verify(MUT_WJ, MUT_RS, &m).is_empty());
}
