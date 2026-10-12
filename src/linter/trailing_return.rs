//! W0009 (ADR-0015 #4): a trailing `return x` is redundant; the final expression is the value.
//!
//! Detection is AST-based (the function body's last statement, and the last statements of both
//! branches of a trailing `if`/`else`). The `wj fix` rewrite is a positional edit: it deletes the
//! `return ` keyword at the location the parser recorded, so it is idempotent and keeps formatting.

use crate::parser::ast::core::{FunctionDecl, Item, Program, Statement};
use crate::source_map::Location;

/// A redundant tail `return`: position of the keyword and whether it carries a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TailReturn {
    pub location: Location,
    pub has_value: bool,
}

fn collect_block<'a>(block: &[&'a Statement<'a>], out: &mut Vec<TailReturn>) {
    match block.last() {
        Some(Statement::Return { value, location: Some(l) }) => {
            out.push(TailReturn { location: l.clone(), has_value: value.is_some() })
        }
        Some(Statement::If { then_block, else_block: Some(else_block), .. }) => {
            collect_block(then_block, out);
            collect_block(else_block, out);
        }
        _ => {}
    }
}

fn collect_function(f: &FunctionDecl, out: &mut Vec<TailReturn>) {
    collect_block(&f.body, out);
}

fn collect_item(item: &Item, out: &mut Vec<TailReturn>) {
    match item {
        Item::Function { decl, .. } => collect_function(decl, out),
        Item::Impl { block, .. } => block.functions.iter().for_each(|f| collect_function(f, out)),
        Item::Mod { items, .. } => items.iter().for_each(|i| collect_item(i, out)),
        _ => {}
    }
}

/// All redundant tail `return`s in `program`, in source order.
pub fn tail_returns(program: &Program) -> Vec<TailReturn> {
    let mut out = Vec::new();
    program.items.iter().for_each(|i| collect_item(i, &mut out));
    out.sort_by_key(|t| (t.location.line, t.location.column));
    out
}

/// Remove `return ` from each valued tail return. Skips bare `return` (nothing to promote) and any
/// site whose text is not exactly `return` + same-line whitespace + value. Idempotent.
pub fn fix_source(source: &str, returns: &[TailReturn]) -> String {
    let line_starts: Vec<usize> =
        std::iter::once(0).chain(source.match_indices('\n').map(|(i, _)| i + 1)).collect();
    let mut edits: Vec<(usize, usize)> = Vec::new();
    for t in returns.iter().filter(|t| t.has_value) {
        let Some(&ls) = line_starts.get(t.location.line.saturating_sub(1)) else { continue };
        let start = ls + t.location.column.saturating_sub(1);
        let Some(rest) = source.get(start..) else { continue };
        let Some(after) = rest.strip_prefix("return") else { continue };
        let ws = after.len() - after.trim_start_matches([' ', '\t']).len();
        if ws == 0 || after[ws..].starts_with('\n') || after[ws..].starts_with('\r') {
            continue;
        }
        edits.push((start, "return".len() + ws));
    }
    let mut out = source.to_string();
    for (start, len) in edits.into_iter().rev() {
        out.replace_range(start..start + len, "");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Program<'static> {
        let tokens = crate::lexer::Lexer::new(src).tokenize_with_locations();
        let mut p = crate::parser::Parser::new_with_source(tokens, "t.wj".to_string(), src.to_string());
        p.parse().unwrap()
    }

    fn fixed(src: &str) -> String {
        fix_source(src, &tail_returns(&parse(src)))
    }

    #[test]
    fn trailing_return_is_found_and_removed() {
        let src = "fn f(x: int) -> int {\n    let y = x + 1\n    return y * 2\n}\n";
        assert_eq!(tail_returns(&parse(src)).len(), 1);
        assert_eq!(fixed(src), "fn f(x: int) -> int {\n    let y = x + 1\n    y * 2\n}\n");
    }

    #[test]
    fn early_return_is_kept() {
        let src = "fn f(x: int) -> int {\n    if x > 1 {\n        return 1\n    }\n    x\n}\n";
        assert!(tail_returns(&parse(src)).is_empty());
        assert_eq!(fixed(src), src);
    }

    #[test]
    fn both_branches_of_trailing_if_else() {
        let src = "fn f(x: int) -> int {\n    if x > 1 {\n        return 1\n    } else {\n        return 2\n    }\n}\n";
        assert_eq!(tail_returns(&parse(src)).len(), 2);
        let out = fixed(src);
        assert!(!out.contains("return"), "{out}");
        assert_eq!(fixed(&out), out, "idempotent");
    }

    #[test]
    fn bare_trailing_return_is_reported_not_rewritten() {
        let src = "fn f() {\n    println(\"x\")\n    return\n}\n";
        let r = tail_returns(&parse(src));
        assert_eq!(r.len(), 1);
        assert!(!r[0].has_value);
        assert_eq!(fixed(src), src);
    }
}
