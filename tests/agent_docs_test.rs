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

//! Agent docs cannot rot: every fenced code block under `docs/agent/` is extracted and executed.
//!
//! * ```` ```windjammer ````       a program: `wj run --interpret`, stdout must equal its `// expect:` lines
//! * ```` ```windjammer,test ````  a test file: `wj test --interpret` must pass
//! * ```` ```windjammer,bad ````   a "what not to write" block: `wj check --json` must report a diagnostic
//! * ```` ```windjammer,parse ```` a fragment: wrapped in `fn main() { .. }` and only parsed (`wj check`)
//!
//! The info string is a comma-separated annotation list shared with docs-truth (`tools/docs_truth/fences.rs`),
//! so `windjammer,should_fail,bad` is a `bad` block here: the kind is the first annotation this file knows.
//!
//! Also: the main context pack stays within its token budget, the `wj docs` CLI answers real queries
//! from the embedded index, and the agent-tool instruction files agree with the CLI.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn docs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/agent")
}

#[derive(Debug)]
struct Block {
    file: String,
    line: usize,
    kind: String,
    code: String,
}

fn md_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                md_files(&p, out);
            } else if p.extension().and_then(|x| x.to_str()) == Some("md") || p.extension().and_then(|x| x.to_str()) == Some("mdc") {
                out.push(p);
            }
        }
    }
}

fn extract(text: &str, file: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut cur: Option<(String, usize, Vec<&str>)> = None;
    for (i, l) in text.lines().enumerate() {
        if let Some(rest) = l.strip_prefix("```") {
            match cur.take() {
                Some((kind, line, code)) => {
                    if kind == "windjammer" || kind.starts_with("windjammer,") || kind == "wj" || kind.starts_with("wj,") {
                        blocks.push(Block { file: file.to_string(), line, kind, code: code.join("\n") + "\n" });
                    }
                }
                None => cur = Some((rest.trim().to_string(), i + 1, Vec::new())),
            }
        } else if let Some((_, _, code)) = cur.as_mut() {
            code.push(l);
        }
    }
    blocks
}

fn all_blocks() -> Vec<Block> {
    let mut files = Vec::new();
    md_files(&docs_dir(), &mut files);
    files.sort();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    files
        .iter()
        .flat_map(|f| extract(&fs::read_to_string(f).unwrap(), &f.strip_prefix(root).unwrap().to_string_lossy()))
        .collect()
}

fn expect_lines(code: &str) -> Vec<String> {
    code.lines()
        .filter_map(|l| l.strip_prefix("// expect:").map(|r| r.strip_prefix(' ').unwrap_or(r).to_string()))
        .collect()
}

fn check_block(b: &Block) -> Option<String> {
    let d = TempDir::new().unwrap();
    let who = format!("{}:{}", b.file, b.line);
    match b.kind.split(',').skip(1).find(|a| matches!(*a, "bad" | "parse" | "test")).unwrap_or("run") {
        "bad" => {
            let f = d.path().join("bad.wj");
            fs::write(&f, &b.code).unwrap();
            let o = Command::new(WJ).args(["check", "--json"]).arg(&f).output().unwrap();
            let v: serde_json::Value = match serde_json::from_slice(&o.stdout) {
                Ok(v) => v,
                Err(e) => return Some(format!("{}: check printed invalid JSON: {}", who, e)),
            };
            if v["diagnostics"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
                return Some(format!("{}: a `bad` block produced no diagnostic: {}", who, v));
            }
            None
        }
        "parse" => {
            let f = d.path().join("frag.wj");
            fs::write(&f, format!("fn main() {{\n{}}}\n", b.code)).unwrap();
            let o = Command::new(WJ).args(["check", "--json"]).arg(&f).output().unwrap();
            let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap_or_default();
            if v["success"] != true {
                return Some(format!("{}: fragment does not check: {}", who, v));
            }
            None
        }
        "test" => {
            let f = d.path().join("doc_test.wj");
            fs::write(&f, &b.code).unwrap();
            let o = Command::new(WJ).args(["test", "--interpret"]).arg(&f).output().unwrap();
            if o.status.success() {
                None
            } else {
                Some(format!("{}: tests failed:\n{}", who, String::from_utf8_lossy(&o.stdout)))
            }
        }
        _ => {
            let f = d.path().join("doc.wj");
            fs::write(&f, &b.code).unwrap();
            let want = expect_lines(&b.code);
            if want.is_empty() {
                return Some(format!("{}: program block has no `// expect:` line", who));
            }
            let o = Command::new(WJ).args(["run", "--interpret"]).arg(&f).output().unwrap();
            let got: Vec<String> = String::from_utf8_lossy(&o.stdout).lines().map(String::from).collect();
            if got != want || !o.status.success() {
                Some(format!("{}: expected {:?}, got {:?} ({})", who, want, got, String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("")))
            } else {
                None
            }
        }
    }
}

#[test]
fn every_code_block_in_agent_docs_is_checked() {
    let blocks = all_blocks();
    assert!(blocks.len() >= 25, "expected many checked blocks, found {}", blocks.len());
    assert!(blocks.iter().any(|b| b.kind.ends_with(",bad")), "the guide must show what NOT to write");
    assert!(blocks.iter().any(|b| b.kind.ends_with(",test")), "the guide must show a test");
    let failures: Vec<String> = blocks.iter().filter_map(check_block).collect();
    assert!(failures.is_empty(), "{} of {} blocks failed:\n{}", failures.len(), blocks.len(), failures.join("\n"));
}

#[test]
fn doc_block_checker_detects_a_wrong_expectation() {
    let wrong = Block {
        file: "synthetic".into(),
        line: 1,
        kind: "windjammer".into(),
        code: "// expect: 4\nfn main() { println(\"${2 + 3}\") }\n".into(),
    };
    assert!(check_block(&wrong).is_some(), "a block whose expect line is wrong must fail");
    let clean_bad = Block {
        file: "synthetic".into(),
        line: 1,
        kind: "windjammer,bad".into(),
        code: "fn main() { println(\"ok\") }\n".into(),
    };
    assert!(check_block(&clean_bad).is_some(), "a `bad` block that is actually fine must fail");
}

#[test]
fn context_pack_fits_its_token_budget() {
    let text = fs::read_to_string(docs_dir().join("WINDJAMMER_FOR_AGENTS.md")).unwrap();
    // 6k tokens at a conservative 3.2 chars/token for code-heavy text.
    assert!(text.len() <= 19_200, "WINDJAMMER_FOR_AGENTS.md is {} bytes; budget is 19200 (~6k tokens)", text.len());
    for needle in ["wj check --json", "wj run --interpret", "wj test --interpret", "wj build", "ADR-0014", "ADR-0015"] {
        assert!(text.contains(needle), "context pack must mention `{}`", needle);
    }
}

#[test]
fn cheat_sheets_exist_and_are_checked() {
    for name in ["cheatsheet_from_rust.md", "cheatsheet_from_python.md", "cheatsheet_from_go.md", "cheatsheet_from_typescript.md"] {
        let text = fs::read_to_string(docs_dir().join(name)).unwrap_or_else(|_| panic!("missing {}", name));
        let n = extract(&text, name).len();
        assert!(n >= 4, "{} needs >= 4 checked examples, has {}", name, n);
        assert!(text.contains("| "), "{} must contain mapping tables", name);
    }
}

#[test]
fn wj_docs_answers_queries_offline() {
    let run = |q: &[&str]| {
        let mut args = vec!["docs"];
        args.extend_from_slice(q);
        let o = Command::new(WJ).args(&args).output().unwrap();
        assert!(o.status.success());
        String::from_utf8_lossy(&o.stdout).to_string()
    };
    assert!(run(&["hashmap", "entry"]).contains("entry"), "HashMap entry must be findable");
    assert!(run(&["sort_by_key"]).contains("sort_by_key"));
    assert!(run(&["pipe"]).to_lowercase().contains("pipe"));
    assert!(run(&["unwrap"]).contains("unwrap_or"), "agents searching unwrap must be pointed at alternatives");
    assert!(run(&["zzzz-no-such-thing"]).contains("no matches"));
    let json = run(&["--json", "fold"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(v.as_array().map(|a| !a.is_empty()).unwrap_or(false));
}

#[test]
fn wj_check_json_reports_diagnostics_with_spans() {
    let d = TempDir::new().unwrap();
    let f = d.path().join("lint.wj");
    fs::write(&f, "fn main() {\n    let v = [1, 2]\n    let n = v.iter().count()\n    println(\"${n}\")\n}\n").unwrap();
    let o = Command::new(WJ).args(["check", "--json"]).arg(&f).output().unwrap();
    assert_eq!(o.status.code(), Some(1), "an error diagnostic must exit 1");
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["success"], false);
    let diag = &v["diagnostics"][0];
    assert_eq!(diag["code"], "W0003");
    assert_eq!(diag["span"]["start_line"], 3);
    assert!(diag["repair_hint"]["description"].as_str().unwrap().contains("iter"));

    let ok = d.path().join("ok.wj");
    fs::write(&ok, "fn main() {\n    println(\"hi\")\n}\n").unwrap();
    let o = Command::new(WJ).args(["check", "--json"]).arg(&ok).output().unwrap();
    assert!(o.status.success());
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["success"], true);
    assert_eq!(v["diagnostics"].as_array().unwrap().len(), 0);

    let bad = d.path().join("bad.wj");
    fs::write(&bad, "fn main() { let x = }\n").unwrap();
    let o = Command::new(WJ).args(["check", "--json"]).arg(&bad).output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["diagnostics"][0]["code"], "parse");
}

#[test]
fn skill_and_templates_describe_the_real_loop() {
    let skill = fs::read_to_string(docs_dir().join("claude-skill/SKILL.md")).expect("SKILL.md");
    assert!(skill.starts_with("---\nname: windjammer"), "SKILL.md needs YAML frontmatter with a name");
    for cmd in ["wj check --json", "wj run --interpret", "wj test --interpret", "wj build", "wj docs"] {
        assert!(skill.contains(cmd), "SKILL.md must describe `{}`", cmd);
    }
    for f in ["templates/AGENTS.md", "templates/cursor-windjammer.mdc", "templates/GENERIC_AGENT_INSTRUCTIONS.md"] {
        let t = fs::read_to_string(docs_dir().join(f)).unwrap_or_else(|_| panic!("missing {}", f));
        assert!(t.contains("wj check --json") && t.contains("wj run --interpret"), "{} must state the loop", f);
        assert!(!t.to_lowercase().contains("co-authored-by"), "{} must not embed vendor attribution", f);
    }
}
