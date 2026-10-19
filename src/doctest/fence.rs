//! Fenced code examples in Markdown and doc comments: extraction and annotations.
//!
//! Shared by the docs-truth gate (`tools/docs_truth`) and `wj test` doctests (ADR-0018 feature 6), so there is
//! exactly one extractor. Annotations (comma-separated after the language, e.g. `wj,should_fail=WJ0007`):
//! * (none)               must parse (as a program, else as the body of `fn main`)
//! * `ignore`             not checked
//! * `check`              must also pass the analyzer
//! * `should_fail`        must fail to parse/analyze; `should_fail=WJ0007` the error must carry the code;
//!                        `should_fail=W0002` the Rust-leakage linter must report that lint
//! * `run`                must run in the interpreter
//! * `// expect: line`    inside the code: run, stdout lines must equal (in order)
//! * `sh,verify`          (bash fences, docs-truth only) executed in a sandbox temp dir

#[derive(Debug, Clone)]
pub struct Fence {
    /// Raw info string after the backticks (e.g. `wj,should_fail=WJ0007`).
    pub info: String,
    /// 1-based line of the opening fence.
    pub line: usize,
    pub body: String,
}

/// Fences of a Markdown text. Handles indentation and ```/~~~ delimiters of >=3 chars.
pub fn fences(text: &str) -> Vec<Fence> {
    let mut out = Vec::new();
    let mut open: Option<(char, usize, Fence)> = None;
    for (i, raw) in text.lines().enumerate() {
        let t = raw.trim_start();
        let ch = t.chars().next().unwrap_or(' ');
        let run = if ch == '`' || ch == '~' {
            t.chars().take_while(|&c| c == ch).count()
        } else {
            0
        };
        match &mut open {
            None if run >= 3 => {
                let info = t[run..].trim().to_string();
                if ch == '`' && info.contains('`') {
                    continue;
                }
                open = Some((
                    ch,
                    run,
                    Fence {
                        info,
                        line: i + 1,
                        body: String::new(),
                    },
                ));
            }
            Some((c, n, f)) => {
                if ch == *c && run >= *n && t[run..].trim().is_empty() {
                    out.push(open.take().unwrap().2);
                } else {
                    f.body.push_str(raw);
                    f.body.push('\n');
                }
            }
            None => {}
        }
    }
    out
}

#[derive(Debug, Default, Clone)]
pub struct Ann {
    pub wj: bool,
    pub sh_verify: bool,
    pub ignore: bool,
    pub check: bool,
    pub run: bool,
    pub should_fail: Option<Option<String>>,
}

pub fn parse_info(info: &str) -> Ann {
    let mut a = Ann::default();
    let mut parts = info.split([',', ' ']).filter(|p| !p.is_empty());
    let lang = parts.next().unwrap_or("");
    a.wj = matches!(lang, "windjammer" | "wj");
    let sh = matches!(lang, "sh" | "bash" | "shell" | "console");
    for p in parts {
        match p {
            "ignore" => a.ignore = true,
            "check" => a.check = true,
            "run" => a.run = true,
            "verify" if sh => a.sh_verify = true,
            "should_fail" => a.should_fail = Some(None),
            p if p.starts_with("should_fail=") => {
                a.should_fail = Some(Some(p["should_fail=".len()..].to_string()))
            }
            _ => {}
        }
    }
    a
}

/// Doc-comment lines become Markdown lines; everything else blank, so line numbers are preserved.
pub fn rustdoc_as_markdown(src: &str) -> String {
    src.lines()
        .map(|l| {
            let t = l.trim_start();
            let rest = t.strip_prefix("///").or_else(|| t.strip_prefix("//!"));
            rest.map(|r| r.strip_prefix(' ').unwrap_or(r))
                .unwrap_or("")
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The `// expect:` lines of a fence, in order.
pub fn expects(code: &str) -> Vec<String> {
    code.lines()
        .filter_map(|l| l.trim().strip_prefix("// expect:"))
        .map(|e| e.strip_prefix(' ').unwrap_or(e).trim_end().to_string())
        .collect()
}
