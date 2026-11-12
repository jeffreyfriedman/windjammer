//! `print EXPR` and `break LINE if EXPR`: evaluate an expression in the stopped frame without changing the run.
//!
//! The interpreter's own evaluator does the work. The guard is what keeps it read-only, and it is a whitelist over the
//! syntax tree: names, literals, fields, indexing, operators, casts, tuples, arrays, struct literals and a short list of
//! built-in methods that only read. Anything else (a call to a program function or to a sink such as `fs::read_to_string`,
//! a block, a closure, a macro, a task or channel operation) is refused before it runs, so a `print` can neither perform
//! an effect, which would consume a journal event and diverge the replay, nor mutate a binding. The debugger is detached
//! while the expression runs, so it adds no steps and cannot stop.

use super::super::engine::Interpreter;
use super::super::error::Ctl;
use super::super::loader::parse_source;
use super::super::value::Value;
use crate::parser::{Expression, Item, Statement};

/// Built-in methods that read their receiver and answer a fresh value.
const READ_ONLY_METHODS: &[&str] = &[
    "len", "is_empty", "get", "contains", "contains_key", "starts_with", "ends_with", "to_string", "to_uppercase", "to_lowercase", "trim", "first", "last", "abs", "min",
    "max", "pow", "sqrt", "clone", "unwrap", "unwrap_or", "is_some", "is_none", "is_ok", "is_err", "as_str", "chars", "count",
];

/// Constructors that wrap a value.
const READ_ONLY_CALLS: &[&str] = &["Some", "Ok", "Err"];

fn refuse(what: &str) -> String {
    format!("`print` and `break .. if` evaluate read-only expressions with no side effects: {what} is not evaluated (allowed: names, fields, indexing, operators, and read-only built-in methods such as `len`)")
}

fn all(es: &[&Expression<'_>]) -> Result<(), String> {
    es.iter().try_for_each(|e| check(e))
}

/// Is this expression made only of reads?
pub(crate) fn check(e: &Expression<'_>) -> Result<(), String> {
    use Expression as E;
    match e {
        E::Literal { .. } | E::Identifier { .. } => Ok(()),
        E::Binary { left, right, .. } => all(&[left, right]),
        E::Index { object, index, .. } => all(&[object, index]),
        E::Range { start, end, .. } => all(&[start, end]),
        E::Unary { operand, .. } => check(operand),
        E::Cast { expr, .. } | E::TryOp { expr, .. } => check(expr),
        E::FieldAccess { object, .. } => check(object),
        E::Tuple { elements, .. } | E::Array { elements, .. } => elements.iter().try_for_each(|x| check(x)),
        E::StructLiteral { fields, .. } => fields.iter().try_for_each(|(_, x)| check(x)),
        E::MapLiteral { pairs, .. } => pairs.iter().try_for_each(|(k, v)| all(&[k, v])),
        E::MethodCall { object, method, arguments, .. } if READ_ONLY_METHODS.contains(&method.as_str()) => {
            check(object)?;
            arguments.iter().try_for_each(|(_, a)| check(a))
        }
        E::MethodCall { method, .. } => Err(refuse(&format!("the method call `.{method}(..)` (it may have effects)"))),
        E::Call { function, arguments, .. } => match &**function {
            E::Identifier { name, .. } if READ_ONLY_CALLS.contains(&name.as_str()) => arguments.iter().try_for_each(|(_, a)| check(a)),
            E::Identifier { name, .. } => Err(refuse(&format!("the call `{name}(..)` (it may have effects)"))),
            _ => Err(refuse("a call (it may have effects)")),
        },
        E::MacroInvocation { name, .. } => Err(refuse(&format!("the macro `{name}!`"))),
        E::Closure { .. } | E::Block { .. } => Err(refuse("a closure or block")),
        E::Await { .. } | E::AsyncCall { .. } | E::SpawnCall { .. } | E::ChannelSend { .. } | E::ChannelRecv { .. } => Err(refuse("a task or channel operation")),
    }
}

/// The one expression `text` parses to.
fn parse_expr(it: &mut Interpreter, text: &str) -> Result<&'static Expression<'static>, String> {
    let wrapped = format!("fn __print__() {{\n{text}\n}}");
    let prog = parse_source(&wrapped, "<print>").map_err(|e| format!("cannot parse `{text}`: {e}"))?;
    let prog = it.retain_program(prog);
    let Some(Item::Function { decl, .. }) = prog.items.first() else { return Err(format!("cannot parse `{text}` as an expression")) };
    match decl.body.as_slice() {
        [Statement::Expression { expr, .. }] => Ok(*expr),
        [Statement::Assignment { .. }] => Err(refuse("an assignment")),
        [Statement::Let { .. }] => Err(refuse("a `let`")),
        _ => Err(format!("`print` takes one expression, found `{text}`")),
    }
}

/// Evaluate `text` in the current frame, read-only. Output the evaluation writes is discarded.
pub(crate) fn eval(it: &mut Interpreter, text: &str) -> Result<Value, String> {
    let expr = parse_expr(it, text)?;
    check(expr)?;
    let (cur, out, err) = (it.cur, it.output.len(), it.err_output.as_ref().map(Vec::len));
    let r = it.eval(expr);
    it.cur = cur;
    it.output.truncate(out);
    if let (Some(n), Some(buf)) = (err, it.err_output.as_mut()) {
        buf.truncate(n);
    }
    r.map_err(|c| match c {
        Ctl::Err(e) => e.message,
        _ => format!("`{text}` cannot be evaluated here"),
    })
}

/// A condition: `text` must evaluate to a boolean.
pub(crate) fn eval_bool(it: &mut Interpreter, text: &str) -> Result<bool, String> {
    match eval(it, text)? {
        Value::Bool(b) => Ok(b),
        other => Err(format!("the condition `{text}` is not a boolean: {}", super::format_quietly(it, &other).unwrap_or_default())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verdict(src: &str) -> Result<(), String> {
        let prog = parse_source(&format!("fn f() {{\n{src}\n}}"), "<t>").unwrap();
        let Some(Item::Function { decl, .. }) = prog.items.first() else { panic!("no fn") };
        let Statement::Expression { expr, .. } = decl.body[0] else { panic!("not an expression: {src}") };
        check(expr)
    }

    #[test]
    fn reads_are_allowed() {
        for ok in ["x", "x + 1 * y", "p.x", "xs[0]", "xs.len() > 2", "s.to_uppercase() == \"A\"", "(a, b)", "Some(x)", "n as float", "-n", "xs.get(1).unwrap_or(0)"] {
            assert!(verdict(ok).is_ok(), "{ok}: {:?}", verdict(ok));
        }
    }

    #[test]
    fn effects_and_mutation_are_refused() {
        for bad in ["fs::read_to_string(\"a\")", "total(3)", "xs.push(1)", "xs.clear()", "println(\"x\")", "f(1) + 2", "|a| a", "xs.len() + xs.pop().unwrap()"] {
            let e = verdict(bad).unwrap_err();
            assert!(e.contains("read-only") && e.contains("no side effects"), "{bad}: {e}");
        }
    }
}
