//! A statement `match` whose arms give different types (WJ0003, rustc E0308): one arm the value of a map or set
//! `insert`/`remove` (an `Option` or a `bool`), another unit (`println(..)`, an assignment). The interpreter runs
//! such a match; the compiled build stops at rustc. Conservative by construction (`tests/check_corpus_test.rs`):
//! the receiver must be a local that every `let` of its name in the function binds to a map or a set, the value arm
//! must be the bare call (a block arm may end in `;` in the generated Rust) and the match must be in unit position.

use crate::check::locate::Locator;
use crate::diagnostics::Diagnostic;
use crate::parser::{Expression, MatchArm, Pattern, SourceLocation, Statement, Type};
use std::collections::HashMap;

const COLLECTIONS: [&str; 4] = ["HashMap", "BTreeMap", "HashSet", "BTreeSet"];

/// WJ0003 for every mixed statement `match` of one function body (`unit`: the function returns nothing, so its last
/// statement is in unit position too).
pub fn check(params: &[crate::parser::Parameter<'_>], body: &[&Statement<'_>], unit: bool, loc: &Locator<'_>, out: &mut Vec<Diagnostic>) {
    let mut lets: HashMap<String, bool> = HashMap::new();
    for p in params {
        lets.insert(p.name.clone(), is_collection_type(&p.type_));
    }
    collect_lets(body, &mut lets);
    let maps: Vec<String> = lets.into_iter().filter(|(_, m)| *m).map(|(n, _)| n).collect();
    if maps.is_empty() {
        return;
    }
    block(body, unit, &maps, loc, out);
}

/// Every `let` name of the body (nested blocks included), and whether all of them bind a map or a set.
fn collect_lets(body: &[&Statement<'_>], lets: &mut HashMap<String, bool>) {
    for s in body {
        match s {
            Statement::Let { pattern, type_, value, .. } => {
                let names = pattern_names(pattern);
                let map = names.len() == 1 && (type_.as_ref().is_some_and(is_collection_type) || is_collection_new(value));
                for n in names {
                    let e = lets.entry(n).or_insert(true);
                    *e &= map;
                }
            }
            Statement::For { body, .. } | Statement::Loop { body, .. } | Statement::While { body, .. } => collect_lets(body, lets),
            Statement::If { then_block, else_block, .. } => {
                collect_lets(then_block, lets);
                if let Some(e) = else_block {
                    collect_lets(e, lets);
                }
            }
            _ => {}
        }
    }
}

fn pattern_names(p: &Pattern<'_>) -> Vec<String> {
    match p {
        Pattern::Identifier(n) => vec![n.clone()],
        Pattern::Tuple(ps) => ps.iter().flat_map(pattern_names).collect(),
        _ => vec!["?".to_string()],
    }
}

fn is_collection_type(t: &Type) -> bool {
    match t {
        Type::Parameterized(n, _) | Type::Custom(n) => COLLECTIONS.iter().any(|c| n == c || n.ends_with(&format!("::{c}"))),
        _ => false,
    }
}

/// `HashMap::new()` (and the other collections' `new` / `default`).
fn is_collection_new(e: &Expression<'_>) -> bool {
    let (ty, f) = match e {
        Expression::Call { function: Expression::Identifier { name, .. }, arguments, .. } if arguments.is_empty() => match name.rsplit_once("::") {
            Some((ty, f)) => (ty.rsplit("::").next().unwrap_or(ty).to_string(), f.to_string()),
            None => return false,
        },
        Expression::MethodCall { object: Expression::Identifier { name, .. }, method, arguments, .. } if arguments.is_empty() => {
            (name.rsplit("::").next().unwrap_or(name).to_string(), method.clone())
        }
        _ => return false,
    };
    (f == "new" || f == "default") && COLLECTIONS.contains(&ty.as_str())
}

/// `unit`: the block's statements are in unit position when they are not its last, or when the block is unit (a loop
/// body, a statement `if` branch).
fn block(stmts: &[&Statement<'_>], unit: bool, maps: &[String], loc: &Locator<'_>, out: &mut Vec<Diagnostic>) {
    for (i, s) in stmts.iter().enumerate() {
        let unit_here = unit || i + 1 < stmts.len();
        match s {
            Statement::Match { arms, location, .. } => {
                if unit_here {
                    mixed(arms, location, maps, loc, out);
                }
            }
            Statement::For { body, .. } | Statement::Loop { body, .. } | Statement::While { body, .. } => block(body, true, maps, loc, out),
            Statement::If { then_block, else_block, .. } => {
                // a statement `if` is in unit position exactly when this statement is
                block(then_block, unit_here, maps, loc, out);
                if let Some(e) = else_block {
                    block(e, unit_here, maps, loc, out);
                }
            }
            _ => {}
        }
    }
}

fn mixed(arms: &[MatchArm<'_>], location: &SourceLocation, maps: &[String], loc: &Locator<'_>, out: &mut Vec<Diagnostic>) {
    let valued = arms.iter().find_map(|a| collection_result(a.body, maps));
    let unit = arms.iter().any(|a| is_unit(a.body));
    let Some((what, ty)) = valued else { return };
    if !unit {
        return;
    }
    let mut d = Diagnostic::error("WJ0003", format!("mismatched types: the arm `{what}(..)` is a value ({ty}), another arm is `()`"));
    if let Some(sp) = loc.word(location, "match") {
        d = d.at(sp.with_label("arms of different types"));
    }
    out.push(d.with_help(format!("discard the value in its arm: `_ => {{ let _ = {what}(..) }}`, or make every arm the same type")));
}

/// `m.insert(..)` / `m.remove(..)` on a map or set local: what it returns.
fn collection_result(e: &Expression<'_>, maps: &[String]) -> Option<(String, &'static str)> {
    let Expression::MethodCall { object: Expression::Identifier { name, .. }, method, .. } = e else { return None };
    if !maps.contains(name) || !(method == "insert" || method == "remove") {
        return None;
    }
    Some((format!("{name}.{method}"), "an `Option` for a map, a `bool` for a set"))
}

/// An arm whose value is certainly `()`: a print call, or a block whose last statement is an assignment or a print.
fn is_unit(e: &Expression<'_>) -> bool {
    match e {
        Expression::Call { function: Expression::Identifier { name, .. }, .. } => matches!(name.as_str(), "println" | "print" | "eprintln" | "eprint"),
        Expression::MacroInvocation { name, .. } => matches!(name.as_str(), "println" | "print" | "eprintln" | "eprint"),
        Expression::Block { statements, .. } => match statements.last() {
            Some(Statement::Assignment { .. }) => true,
            Some(Statement::Expression { expr, .. }) => is_unit(expr),
            _ => false,
        },
        _ => false,
    }
}
