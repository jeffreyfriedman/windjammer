//! Where a statement starts, for stops and breakpoints.
//!
//! The parser records most node locations at the token *after* the node (a statement's location is usually the start
//! of the next line). The debugger wants the line a statement begins on, so it takes the leftmost leaf of the expression
//! the statement starts with: a literal is located at its own token, and an identifier at the token after it, which is
//! on the identifier's own line unless the identifier ends its line; then the statement began on the nearest earlier
//! line that holds code.

use super::super::eval_expr::{expr_loc, stmt_loc};
use crate::parser::{Expression, Statement};
use crate::source_map::Location;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

/// A statement's start: the AST location it came from (for the file) and the corrected line and column.
#[derive(Clone, Copy)]
pub struct Pos {
    pub loc: &'static Location,
    pub line: usize,
    pub column: usize,
}

#[derive(Default)]
pub struct Lines {
    sources: HashMap<PathBuf, Rc<Vec<String>>>,
}

fn head(s: &'static Statement<'static>) -> Option<&'static Expression<'static>> {
    match s {
        Statement::Let { value, .. } | Statement::Const { value, .. } | Statement::Static { value, .. } => Some(*value),
        Statement::Assignment { target, .. } => Some(*target),
        Statement::Return { value, .. } => *value,
        Statement::Expression { expr, .. } => Some(*expr),
        Statement::If { condition, .. } | Statement::While { condition, .. } => Some(*condition),
        Statement::Match { value, .. } => Some(*value),
        Statement::For { iterable, .. } => Some(*iterable),
        _ => None,
    }
}

fn leftmost(mut e: &'static Expression<'static>) -> &'static Expression<'static> {
    loop {
        e = match e {
            Expression::Binary { left, .. } => left,
            Expression::MethodCall { object, .. } | Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => object,
            Expression::Call { function, .. } => function,
            Expression::Cast { expr, .. } => expr,
            Expression::Range { start, .. } => start,
            _ => return e,
        };
    }
}

impl Lines {
    fn source(&mut self, file: &PathBuf) -> Rc<Vec<String>> {
        self.sources.entry(file.clone()).or_insert_with(|| Rc::new(std::fs::read_to_string(file).unwrap_or_default().lines().map(String::from).collect())).clone()
    }

    pub fn start(&mut self, s: &'static Statement<'static>) -> Option<Pos> {
        let leaf = head(s).map(leftmost);
        let Some(loc) = leaf.and_then(expr_loc).or_else(|| stmt_loc(s)) else { return None };
        let mut pos = Pos { loc, line: loc.line, column: loc.display_column() };
        if matches!(leaf, Some(Expression::Identifier { .. })) {
            let src = self.source(&loc.file);
            let first_on_line = src.get(loc.line.wrapping_sub(1)).is_some_and(|l| l.chars().take(loc.column.saturating_sub(1)).all(char::is_whitespace));
            if !first_on_line {
                // the identifier itself: the last place its name ends before the token that follows it
                if let (Some(Expression::Identifier { name, .. }), Some(l)) = (leaf, src.get(loc.line.wrapping_sub(1))) {
                    let before: String = l.chars().take(loc.column.saturating_sub(1)).collect();
                    if let Some(i) = before.trim_end().strip_suffix(name.as_str()).map(|b| b.chars().count()) {
                        pos.column = i + 1;
                    }
                }
            } else {
                let code = |l: &String| !l.trim().is_empty() && !l.trim_start().starts_with("//");
                if let Some(i) = (0..loc.line.saturating_sub(1)).rev().find(|&i| src.get(i).is_some_and(code)) {
                    pos.line = i + 1;
                    pos.column = src[i].len() - src[i].trim_start().len() + 1;
                }
            }
        }
        Some(pos)
    }
}
