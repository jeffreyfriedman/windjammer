//! Checking one fenced example: parse / analyze / lint / interpret, compare `// expect:` output.
//!
//! Used by docs-truth (Markdown and rustdoc fences) and by `wj test` doctests (ADR-0018 feature 6).

use super::fence::{expects, Ann};
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn parse(src: &str) -> Result<crate::parser::Program<'static>, String> {
    let src = src.to_string();
    catch_unwind(AssertUnwindSafe(|| {
        let tokens = Lexer::new(&src).tokenize_with_locations();
        // Dropping the parser is sound: its arenas are retired, not freed (docs/design/parser-arena-ownership.md).
        Parser::new_with_source(tokens, "doc.wj".to_string(), src.clone()).parse()
    }))
    .unwrap_or_else(|_| Err("parser panicked".to_string()))
}

/// Parse as a program; fragments (statements) are retried as the body of `fn main`.
fn parse_fragment(code: &str) -> Result<(crate::parser::Program<'static>, String), String> {
    match parse(code) {
        Ok(p) => Ok((p, code.to_string())),
        Err(e1) => {
            let wrapped = format!("fn main() {{\n{code}\n}}\n");
            parse(&wrapped)
                .map(|p| (p, wrapped))
                .map_err(|_| format!("parse error: {e1}"))
        }
    }
}

fn interpret(src: &str) -> Result<String, String> {
    let (prog, _) = parse_fragment(src)?;
    catch_unwind(AssertUnwindSafe(|| {
        let mut it = crate::interpreter::Interpreter::new_capturing();
        it.run(&prog).map(|_| it.get_output())
    }))
    .unwrap_or_else(|_| Err("interpreter panicked".into()))
}

fn analyze(prog: &crate::parser::Program<'static>) -> Result<(), String> {
    catch_unwind(AssertUnwindSafe(|| {
        crate::analyzer::Analyzer::new()
            .analyze_program(prog)
            .map(|_| ())
            .map_err(|e| format!("{e}"))
    }))
    .unwrap_or_else(|_| Err("analyzer panicked".into()))
}

fn lint_codes(prog: &crate::parser::Program<'static>, src: &str) -> Vec<String> {
    let mut l = crate::linter::rust_leakage::RustLeakageLinter::new("doc.wj");
    l.lint_program(prog);
    l.lint_source(src);
    l.into_diagnostics()
        .into_iter()
        .map(|d| d.lint_name)
        .collect()
}

/// Run on a thread with a big stack: the parser/analyzer recurse deeply and test threads have 2 MB.
pub fn check_wj(code: &str, a: &Ann) -> Result<(), String> {
    check_with_expects(code, &expects(code), a)
}

/// Like [`check_wj`], but the expected output is given (a doctest's program includes its module source, whose
/// comments must not count as `// expect:` lines).
pub fn check_with_expects(code: &str, want: &[String], a: &Ann) -> Result<(), String> {
    let (code, want, a) = (code.to_string(), want.to_vec(), a.clone());
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || check_wj_inner(&code, &want, &a))
        .map_err(|e| e.to_string())?
        .join()
        .unwrap_or_else(|_| Err("checker thread panicked".into()))
}

fn check_wj_inner(code: &str, want: &[String], a: &Ann) -> Result<(), String> {
    if let Some(want) = &a.should_fail {
        return match parse_fragment(code) {
            Err(e) => match want {
                Some(c) if c.starts_with("WJ") && !e.contains(c.as_str()) => {
                    Err(format!("should_fail={c}: parse error lacks the code: {e}"))
                }
                Some(c) if !c.starts_with("WJ") => Err(format!(
                    "should_fail={c}: expected a lint but the fence does not parse: {e}"
                )),
                _ => Ok(()),
            },
            Ok((prog, src)) => match want {
                Some(c) if c.starts_with('W') && !c.starts_with("WJ") => {
                    let got = lint_codes(&prog, &src);
                    if got.iter().any(|g| g == c) {
                        Ok(())
                    } else {
                        Err(format!("should_fail={c}: lints reported {got:?}"))
                    }
                }
                other => match analyze(&prog) {
                    Err(e) if other.as_ref().map_or(true, |c| e.contains(c.as_str())) => Ok(()),
                    Err(e) => Err(format!(
                        "should_fail={}: error lacks the code: {e}",
                        other.clone().unwrap_or_default()
                    )),
                    Ok(()) => {
                        Err("marked should_fail but it parses and analyzes cleanly".to_string())
                    }
                },
            },
        };
    }
    let (prog, _) = parse_fragment(code)?;
    if a.check {
        analyze(&prog).map_err(|e| format!("analyze error: {e}"))?;
    }
    if !want.is_empty() || a.run {
        let out = interpret(code).map_err(|e| format!("interpreter error: {e}"))?;
        if !want.is_empty() {
            let got: Vec<String> = out.lines().map(|l| l.trim_end().to_string()).collect();
            if got != want {
                return Err(format!("output {got:?} != expected {want:?}"));
            }
        }
    }
    Ok(())
}
