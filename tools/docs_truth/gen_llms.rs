//! `llms.txt` and `llms-full.txt` at the repository root (https://llmstxt.org convention).
//! Both are assembled from files that are themselves checked: the agent guide, the generated CLI/targets/keywords
//! references, the MCP tool manifest and (when present) the compact language spec. A missing source is an error,
//! so a rename cannot silently leave a dead link in the index.

use super::Ctx;

const RAW: &str = "https://raw.githubusercontent.com/jeffreyfriedman/windjammer/main/";

/// (path, link text, one-line description) for `llms.txt`. Every path must exist.
const INDEX: &[(&str, &str, &str, &str)] = &[
    ("Start here", "docs/agent/WINDJAMMER_FOR_AGENTS.md", "Windjammer for agents", "The edit-check-run loop, core idioms and pitfalls; every code block is executed in CI"),
    ("Start here", "docs/reference/cli.md", "CLI reference", "Every `wj` subcommand, generated from the clap definitions"),
    ("Start here", "docs/reference/targets.md", "Backend support matrix", "Which backends pass how much of the conformance corpus (generated)"),
    ("Language", "docs/reference/keywords.md", "Keywords and operators", "Lexer keywords (generated from the lexer)"),
    ("Language", "docs/reference/intrinsics.md", "Intrinsics", "Built-in functions available on every backend"),
    ("Language", "docs/reference/stdlib.md", "Standard library", "Signatures scanned from `std/**/*.wj`"),
    ("Language", "docs/reference/errors.md", "Error and lint codes", "WJ#### and W#### codes"),
    ("Language", "docs/windjammer.ebnf", "Grammar", "EBNF cross-checked against the real parser"),
    ("Language", "docs/language-design/ownership-inference.md", "Ownership inference", "What the compiler infers and how"),
    ("Language", "docs/agent/cheatsheet_from_rust.md", "Cheatsheet from Rust", "Rust to Windjammer mapping (also from Go, Python and TypeScript in the same directory)"),
    ("Project state", "docs/known-gaps.md", "Known gaps", "The ranked, honest list of what is missing"),
    ("Project state", "docs/WHY.md", "Why Windjammer", "Comparisons with Rust, Go, Python, Zig, Mojo and Gleam; goals are labelled as goals"),
    ("Project state", "ROADMAP.md", "Roadmap", "Work with a concrete plan attached"),
    ("Optional", "docs/INTERPRETER.md", "Interpreter", "What `wj run --interpret` supports and rejects (generated)"),
    ("Optional", "docs/adr/README.md", "Architecture decision records", "Open and accepted design decisions"),
    ("Optional", "bench/agent-dx/README.md", "agent-dx benchmark", "Same 60 tasks in six languages, graded by hidden tests"),
    ("Optional", "benchmarks/README.md", "Performance benchmarks", "Inputs, runners and the last recorded results"),
];

const TAGLINE: &str = "Windjammer is a high-level language that compiles to Rust, with ownership and borrowing inferred by the compiler. The same `.wj` source also runs in an interpreter and has Go, JavaScript and WebAssembly backends of lower maturity.";

fn version(ctx: &Ctx) -> String {
    ctx.read("Cargo.toml")
        .lines()
        .skip_while(|l| !l.starts_with("[workspace.package]"))
        .find_map(|l| l.strip_prefix("version = ").map(|v| v.trim().trim_matches('"').to_string()))
        .unwrap_or_default()
}

/// Hand-written or generated documents inlined into `llms-full.txt`, in order (the MCP manifest is rendered separately).
const FULL: &[&str] = &[
    "docs/agent/WINDJAMMER_FOR_AGENTS.md",
    "docs/reference/cli.md",
    "docs/reference/targets.md",
    "docs/reference/keywords.md",
    MCP_MANIFEST,
    "docs/known-gaps.md",
];

const MCP_MANIFEST: &str = "crates/windjammer-mcp/tools/manifest.json";

/// The generators' input contract, used by the scratch repository in tests/docs_truth_test.rs so adding a source
/// cannot silently break the fixture: `llms.txt` links (must exist) and `llms-full.txt` inlined documents (must be
/// non-empty).
pub fn link_targets() -> impl Iterator<Item = &'static str> {
    INDEX.iter().map(|(_, p, _, _)| *p)
}

pub fn inlined_sources() -> impl Iterator<Item = &'static str> {
    FULL.iter().copied()
}

fn require(ctx: &Ctx, path: &str) -> Result<String, String> {
    let s = ctx.read(path);
    if s.is_empty() {
        Err(format!("llms generator: source `{path}` is missing or empty"))
    } else {
        Ok(s)
    }
}

pub fn llms_txt(ctx: &Ctx) -> Result<String, String> {
    let mut out = format!("# Windjammer\n\n> {TAGLINE}\n\n");
    out.push_str(&format!(
        "Version {} (pre-1.0: language and compiler behaviour can change between minor versions). You have probably seen almost no Windjammer in training data: do not guess from Rust or Python. Use `wj check --json FILE` for diagnostics, `wj run --interpret FILE` for an instant run, and `wj docs <terms>` for offline search. `llms-full.txt` contains the agent guide, CLI reference, backend matrix, keywords and MCP tool list inline.\n",
        version(ctx)
    ));
    let mut section = "";
    for (sec, path, text, desc) in INDEX {
        if !ctx.root.join(path).is_file() {
            return Err(format!("llms generator: index target `{path}` does not exist"));
        }
        if *sec != section {
            out.push_str(&format!("\n## {sec}\n\n"));
            section = sec;
        }
        out.push_str(&format!("- [{text}]({RAW}{path}): {desc}\n"));
    }
    Ok(out)
}

/// Drop YAML front matter and GENERATED header comments; keep the document body.
fn strip(doc: &str) -> String {
    let mut s = doc;
    if let Some(rest) = s.strip_prefix("---\n") {
        if let Some(i) = rest.find("\n---\n") {
            s = &rest[i + 5..];
        }
    }
    s.lines().filter(|l| !l.contains("GENERATED FILE - DO NOT EDIT")).collect::<Vec<_>>().join("\n").trim().to_string()
}

fn mcp_tools(ctx: &Ctx) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_str(&require(ctx, MCP_MANIFEST)?).map_err(|e| e.to_string())?;
    let mut out = String::from("The `windjammer-mcp` server (`cargo build --release -p windjammer-mcp`, stdio by default) exposes:\n\n");
    for t in v["tools"].as_array().into_iter().flatten() {
        let g = |k: &str| t[k].as_str().unwrap_or("");
        out.push_str(&format!("- `{}` ({}, {}): {}\n", g("name"), g("category"), g("stability"), g("description")));
    }
    Ok(out)
}

pub fn llms_full(ctx: &Ctx) -> Result<String, String> {
    let mut out = format!("# Windjammer (full context for LLM agents)\n\n> {TAGLINE}\n\nVersion {}. Assembled by `scripts/docs_gen.sh` from the files named in each section heading; do not edit. Index version: llms.txt.\n", version(ctx));
    let mut sections: Vec<(String, String)> = Vec::new();
    for path in FULL {
        let body = if *path == MCP_MANIFEST { mcp_tools(ctx)? } else { strip(&require(ctx, path)?) };
        sections.push((path.to_string(), body));
    }
    for opt in ["docs/spec/compact.md", "docs/spec/COMPACT.md"] {
        let s = ctx.read(opt);
        if !s.is_empty() {
            sections.insert(1, (opt.to_string(), strip(&s)));
        }
    }
    for (path, body) in sections {
        // Demote each document's headings one level so the section heading stays the top of its part.
        let body: String = body.lines().map(|l| if l.starts_with('#') { format!("#{l}") } else { l.to_string() }).collect::<Vec<_>>().join("\n");
        out.push_str(&format!("\n\n---\n\n# Source: {path}\n\n{body}\n"));
    }
    Ok(out)
}
