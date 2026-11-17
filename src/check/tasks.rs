//! ADR-0004 diagnostics of `wj check`: un-waited tasks (WJ0190, found by `analyzer::task_structure`) and removed
//! concurrency forms (WJ0191, met by the parser: `parser::task_prefix::RemovedForm`), each with its fix.
//!
//! WJ0190 fix: wrap the statement that loses the task, and the rest of its block, in `scope { }` (the scope waits
//! for the task where the block ends, the closest structured equivalent of a task nobody joins). It is
//! machine-applicable for a dropped task; for a task bound to a name it is offered only (the author may rather
//! call `.wait()`).

use super::front::Parsed;
use crate::analyzer::task_structure::{self, Lost};
use crate::diagnostics::{Applicability, Diagnostic, Edit, Fix, SourceIndex};
use crate::lexer::{Lexer, Token};

/// WJ0190 for every un-waited task of the file.
pub fn unwaited(p: &Parsed, ix: &SourceIndex, out: &mut Vec<Diagnostic>) {
    for f in task_structure::check(&p.program) {
        if f.line == 0 {
            continue;
        }
        let at = ix.offset(f.line, f.column);
        let mut d = Diagnostic::error(task_structure::CODE, f.message())
            .at(ix.word_span_from(at).with_label("this task"))
            .with_help(f.help());
        if let Some(edits) = wrap_rest_in_scope(ix, at) {
            let applicability = match f.lost {
                Lost::Dropped => Applicability::MachineApplicable,
                Lost::NeverUsed(_) => Applicability::MaybeIncorrect,
            };
            d = d.with_fix(Fix {
                description: "start it inside a `scope { }` that ends with the block".into(),
                applicability,
                edits,
            });
        }
        out.push(d);
    }
}

/// WJ0192 (a task changes a captured variable) and WJ0193 (a spawned task captures a thread-bound value).
pub fn captures(p: &Parsed, ix: &SourceIndex, out: &mut Vec<Diagnostic>) {
    for f in crate::analyzer::task_captures::check(&p.program) {
        if f.line == 0 {
            continue;
        }
        let at = ix.offset(f.line, f.column);
        out.push(Diagnostic::error(f.code, f.message()).at(ix.word_span_from(at).with_label("this task")).with_help(f.help()));
    }
}

/// Edits that put `scope {` before the statement on the line of byte `at` and `}` before the brace that closes its
/// block, indenting the lines in between by four spaces. `None` when the block's end cannot be found.
fn wrap_rest_in_scope(ix: &SourceIndex, at: usize) -> Option<Vec<Edit>> {
    let text = ix.text();
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let indent: String = text[line_start..]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    // a statement after `{` on the same line (`fn main() { spawn w() }`) starts after that brace
    let same_line = text[line_start..at].rfind('{').map(|i| line_start + i + 1);
    let stmt_start = match same_line {
        Some(b) => b + text[b..at].len() - text[b..at].trim_start().len(),
        None => line_start + indent.len(),
    };
    let tokens = std::panic::catch_unwind(|| Lexer::new(text).tokenize_with_locations()).ok()?;
    let mut depth = 0usize;
    let mut close = None;
    for t in tokens.iter().filter(|t| t.byte_start >= stmt_start) {
        match t.token {
            Token::LBrace | Token::LParen | Token::LBracket => depth += 1,
            Token::RParen | Token::RBracket => depth = depth.checked_sub(1)?,
            Token::RBrace if depth == 0 => {
                close = Some(t.byte_start);
                break;
            }
            Token::RBrace => depth -= 1,
            _ => {}
        }
    }
    let close = close?;
    let close_line_start = text[..close].rfind('\n').map_or(0, |i| i + 1);
    let brace_on_own_line =
        text[close_line_start..close].trim().is_empty() && close_line_start > stmt_start;
    let opening = if same_line.is_some() {
        "scope { ".to_string()
    } else {
        format!("scope {{\n{indent}    ")
    };
    let mut edits = vec![Edit {
        span: ix.span(stmt_start, stmt_start),
        replacement: opening,
    }];
    // indent every following line of the region
    let region_end = if brace_on_own_line {
        close_line_start
    } else {
        close
    };
    let mut i = stmt_start;
    while let Some(nl) = text[i..region_end.max(i)].find('\n') {
        let next = i + nl + 1;
        if next >= region_end {
            break;
        }
        if !text[next..].lines().next().unwrap_or("").trim().is_empty() {
            edits.push(Edit {
                span: ix.span(next, next),
                replacement: "    ".into(),
            });
        }
        i = next;
    }
    edits.push(if brace_on_own_line {
        Edit {
            span: ix.span(close_line_start, close_line_start),
            replacement: format!("{indent}}}\n"),
        }
    } else {
        Edit {
            span: ix.span(close, close),
            replacement: " } ".into(),
        }
    });
    Some(edits)
}

/// WJ0191 for every removed concurrency form the parser met; `true` when there was one.
pub fn removed_forms(
    parser: &crate::parser::Parser,
    ix: &SourceIndex,
    out: &mut Vec<Diagnostic>,
) -> bool {
    for f in parser.removed_forms() {
        let mut d = Diagnostic::error(
            crate::parser::task_prefix::REMOVED_FORM_CODE,
            f.message.clone(),
        )
        .at(ix.span(f.byte_start, f.byte_end).with_label("removed"))
        .with_help(f.help.clone());
        if !f.edits.is_empty() {
            d = d.with_fix(Fix {
                description: "rewrite with the ADR-0004 surface".into(),
                applicability: Applicability::MachineApplicable,
                edits: f
                    .edits
                    .iter()
                    .map(|(a, b, r)| Edit {
                        span: ix.span(*a, *b),
                        replacement: r.clone(),
                    })
                    .collect(),
            });
        }
        out.push(d);
    }
    !parser.removed_forms().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed(src: &str) -> String {
        let ix = SourceIndex::new("t.wj", src);
        let at = src.find("spawn").unwrap();
        let edits = wrap_rest_in_scope(&ix, at).expect("block end found");
        crate::diagnostics::apply::apply_edits(src, &edits).expect("edits apply")
    }

    #[test]
    fn wraps_the_rest_of_the_block_in_a_scope() {
        let src =
            "fn main() {\n    let x = 1\n    spawn { println(\"a\") }\n    println(\"b\")\n}\n";
        assert_eq!(fixed(src), "fn main() {\n    let x = 1\n    scope {\n        spawn { println(\"a\") }\n        println(\"b\")\n    }\n}\n");
    }

    #[test]
    fn a_closing_brace_on_the_same_line_is_handled() {
        let src = "fn main() { spawn w() }\n";
        assert_eq!(fixed(src), "fn main() { scope { spawn w()  } }\n");
    }
}
