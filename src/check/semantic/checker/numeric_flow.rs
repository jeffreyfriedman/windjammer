//! WJ0003: a float value where an int is declared, and an int value where a float is declared (rustc E0308).
//!
//! The literal forms (`let m: int = 7.5`) are judged by `mismatch::check_literal`; this is the same diagnostic for
//! every non-literal value whose class is known from DECLARATIONS: an annotated or parameter binding, the declared
//! return of a project or std function or method, a declared struct field, an `as` cast, and arithmetic over those.
//! Anything else has no class and is never judged. An untyped integer literal stays flexible (it may be a float
//! literal in a float context), and a binding that only a literal typed (`let n = 5`) is not an int by declaration.
//!
//! The slots are an annotated `let`, a call argument (project fns, associated fns, methods, std fns and methods by
//! their declared parameters), a struct field initialiser, a return value, an assignment (binding, field, list
//! element) and the elements of a list literal stored in an annotated list. The hint spells the explicit conversion;
//! it is never a machine-applicable fix, because `as int` drops the fraction (ADR-0015: no silent truncation).

use super::*;
use crate::parser::{BinaryOp, UnaryOp};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Int,
    Float,
    /// An untyped integer literal: an int or a float, as its context needs.
    IntLiteral,
}

fn class_of_kind(k: Kind) -> Option<Class> {
    match k {
        Kind::Int => Some(Class::Int),
        Kind::Float => Some(Class::Float),
        _ => None,
    }
}

/// The class of `l op r` for arithmetic: both sides must be known, and a float with an int is not judged here.
fn combine(l: Class, r: Class) -> Option<Class> {
    use Class::*;
    match (l, r) {
        (Float, Float | IntLiteral) | (IntLiteral, Float) => Some(Float),
        (Int, Int | IntLiteral) | (IntLiteral, Int) => Some(Int),
        (IntLiteral, IntLiteral) => Some(IntLiteral),
        _ => None,
    }
}

fn name(k: Kind) -> &'static str {
    rules::display_kind(k)
}

impl Checker<'_, '_, '_> {
    fn class_of(&self, e: &Expression<'_>) -> Option<Class> {
        match e {
            Expression::Literal { value, .. } => match value {
                Literal::Float(_) => Some(Class::Float),
                Literal::Int(_) => Some(Class::IntLiteral),
                Literal::IntSuffixed(..) => Some(Class::Int),
                _ => None,
            },
            Expression::Identifier { name, .. } => match self.scalars.get(name).copied()? {
                (Kind::Float, _) => Some(Class::Float),
                (Kind::Int, true) => Some(Class::Int),
                _ => None,
            },
            Expression::Cast { type_, .. } => rules::type_kind(type_).and_then(class_of_kind),
            Expression::Unary { op: UnaryOp::Neg, operand, .. } => self.class_of(operand),
            Expression::Binary { left, op, right, .. } => match op {
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                    combine(self.class_of(left)?, self.class_of(right)?)
                }
                _ => None,
            },
            Expression::Call { function, .. } => {
                let Expression::Identifier { name, .. } = &**function else { return None };
                self.declared_return(name).and_then(class_of_kind)
            }
            Expression::FieldAccess { object, field, .. } => {
                let ty = self.struct_of(object)?;
                let fields = self.known_struct_fields(&ty)?;
                fields.iter().find(|(n, _)| n == field).and_then(|(_, t)| rules::type_kind(t)).and_then(class_of_kind)
            }
            Expression::MethodCall { object, method, .. } => {
                let (_, ret) = self.method_signature(object, method)?;
                ret.and_then(class_of_kind)
            }
            _ => None,
        }
    }

    /// The user struct a field-access receiver has by declaration (`self`, a parameter or an annotated binding).
    fn struct_of(&self, object: &Expression<'_>) -> Option<String> {
        let Expression::Identifier { name, .. } = object else { return None };
        if name == "self" {
            self.self_ty.clone()
        } else {
            self.typed.get(name).cloned()
        }
    }

    /// The std module a name stands for in this file (`use std::math` -> `math`), when no local hides it.
    fn std_module(&self, head: &str) -> Option<&str> {
        self.cx.aliases.get(head).filter(|_| !self.locals.contains(head)).map(String::as_str)
    }

    /// The project functions named `name`, unless a local or another crate's import is what the name means here.
    fn project_fn(&self, name: &str) -> Option<&[super::super::index::FnInfo]> {
        if self.locals.contains(name) || self.cx.names.foreign.contains(name) {
            return None;
        }
        self.cx.idx.fns.get(name).map(Vec::as_slice)
    }

    /// The declared return kind of the call `name(..)`: a unique project fn, or a std module function.
    fn declared_return(&self, callee: &str) -> Option<Kind> {
        if let Some((head, func)) = callee.rsplit_once("::") {
            let module = self.std_module(head)?;
            return rules::type_kind(self.cx.idx.declared.as_ref()?.std_return(module, func)?);
        }
        let [info] = self.project_fn(callee)? else { return None };
        if info.generic {
            return None;
        }
        info.ret.as_ref().and_then(rules::type_kind)
    }

    /// Declared parameter kinds of the call `name(..)` (`None` for a parameter that is not a scalar).
    fn declared_params(&self, callee: &str) -> Option<Vec<Option<Kind>>> {
        let kinds = |ts: &[Type]| ts.iter().map(rules::type_kind).collect::<Vec<_>>();
        if let Some((head, func)) = callee.rsplit_once("::") {
            if let Some(module) = self.std_module(head) {
                return Some(kinds(self.cx.idx.declared.as_ref()?.std_params(module, func)?));
            }
            let [info] = self.cx.idx.method_sigs.get(&(head.to_string(), func.to_string()))?.as_slice() else { return None };
            return (!info.generic && !info.has_self).then(|| kinds(&info.params));
        }
        let [info] = self.project_fn(callee)? else { return None };
        (!info.generic).then(|| kinds(&info.params))
    }

    /// Declared parameter kinds and return kind of `object.method(..)`: a method of a project type, a std module
    /// function (`math.sqrt(x)`), or a method `std/rust_core.wji` declares for a float or an annotated list.
    fn method_signature(&self, object: &Expression<'_>, method: &str) -> Option<(Vec<Option<Kind>>, Option<Kind>)> {
        let kinds = |ts: &[Type]| ts.iter().map(rules::type_kind).collect::<Vec<_>>();
        if let Expression::Identifier { name, .. } = object {
            if let Some(module) = self.std_module(name) {
                let declared = self.cx.idx.declared.as_ref()?;
                let params = kinds(declared.std_params(module, method)?);
                return Some((params, declared.std_return(module, method).and_then(rules::type_kind)));
            }
        }
        if let Some(ty) = self.struct_of(object) {
            let [info] = self.cx.idx.method_sigs.get(&(ty, method.to_string()))?.as_slice() else { return None };
            if info.generic || !info.has_self {
                return None;
            }
            return Some((kinds(&info.params), info.ret.as_ref().and_then(rules::type_kind)));
        }
        let receiver = match self.class_of(object) {
            Some(Class::Float) => Type::Float,
            _ => return self.list_signature(object, method),
        };
        let sig = super::super::declared::core().method_signature(&receiver, method, &[])?;
        let float = |t: &Type| if matches!(t, Type::Generic(n) if n == "T") { Some(Kind::Float) } else { rules::type_kind(t) };
        Some((sig.params.iter().map(float).collect(), sig.ret.as_ref().and_then(float)))
    }

    /// A method of a list bound with an annotated element kind (`let xs: Vec<int>`; `xs.push(v)` takes the element).
    fn list_signature(&self, object: &Expression<'_>, method: &str) -> Option<(Vec<Option<Kind>>, Option<Kind>)> {
        let Expression::Identifier { name, .. } = object else { return None };
        let elem = match self.list_elems.get(name)? {
            Kind::Int => Type::Int,
            Kind::Float => Type::Float,
            _ => return None,
        };
        let sig = super::super::declared::core().method_signature(&Type::Vec(Box::new(elem)), method, &[])?;
        Some((sig.params.iter().map(rules::type_kind).collect(), None))
    }

    /// The source span of `e` when it is a plain expression whose first and last token can be found.
    fn expr_span(&self, e: &Expression<'_>) -> Option<Span> {
        let ix = &self.cx.loc.ix;
        let text = ix.text();
        let span = match e {
            Expression::Identifier { .. } | Expression::Literal { .. } => self.operand_span(e)?,
            Expression::Binary { left, right, .. } => {
                let (a, b) = (self.expr_span(left)?, self.expr_span(right)?);
                ix.span(a.byte_start, b.byte_end)
            }
            Expression::Cast { expr, .. } => {
                let a = self.expr_span(expr)?;
                let rest = text.get(a.byte_end..)?;
                let after_as = rest.trim_start().strip_prefix("as")?;
                let word = after_as.trim_start();
                if !after_as.starts_with(char::is_whitespace) {
                    return None;
                }
                let len = word.chars().take_while(|c| c.is_alphanumeric() || *c == '_').count();
                let end = text.len() - word.len() + len;
                ix.span(a.byte_start, end)
            }
            Expression::FieldAccess { object, field, .. } => {
                let a = self.expr_span(object)?;
                let tail = format!(".{field}");
                text.get(a.byte_end..)?.starts_with(&tail).then(|| ix.span(a.byte_start, a.byte_end + tail.len()))?
            }
            Expression::Call { function, .. } => {
                let Expression::Identifier { name, .. } = &**function else { return None };
                let head = self.operand_span(function)?;
                ix.span(head.byte_start, close_paren(text, head.byte_start + name.len())?)
            }
            Expression::MethodCall { object, method, .. } => {
                let a = self.expr_span(object)?;
                let tail = format!(".{method}");
                if !text.get(a.byte_end..)?.starts_with(&tail) {
                    return None;
                }
                ix.span(a.byte_start, close_paren(text, a.byte_end + tail.len())?)
            }
            _ => return None,
        };
        let shown = text.get(span.byte_start..span.byte_end)?;
        (shown.matches('(').count() == shown.matches(')').count()).then_some(span)
    }

    /// The leftmost name or literal of `e`, to anchor a diagnostic when the whole expression has no span.
    fn first_leaf_span(&self, e: &Expression<'_>) -> Option<Span> {
        match e {
            Expression::Binary { left, .. } => self.first_leaf_span(left),
            Expression::Cast { expr, .. } => self.first_leaf_span(expr),
            Expression::Unary { operand, .. } => self.first_leaf_span(operand),
            Expression::FieldAccess { object, .. } | Expression::MethodCall { object, .. } => self.first_leaf_span(object),
            Expression::Call { function, .. } => self.first_leaf_span(function),
            other => self.operand_span(other),
        }
    }

    /// Report `value` when its declared class cannot be stored in a slot of kind `want`.
    /// `judge_literal`: also judge a literal value (assignments; every other slot has the literal check).
    pub(super) fn numeric_into(&mut self, want: Kind, value: &Expression<'_>, what: &str, judge_literal: bool) {
        if matches!(value, Expression::Literal { .. }) && !judge_literal {
            return;
        }
        let (Some(slot), Some(found)) = (class_of_kind(want), self.class_of(value)) else { return };
        if found == Class::IntLiteral || found == slot {
            return;
        }
        let (e, f) = (name(want), if found == Class::Float { "float" } else { "int" });
        let mut d = Diagnostic::error("WJ0003", format!("mismatched types: expected `{e}`, found `{f}`{what}"));
        let shown = self.expr_span(value);
        if let Some(sp) = shown.clone().or_else(|| self.first_leaf_span(value)) {
            d = d.at(sp.with_label(format!("expected `{e}`, found `{f}`")));
        }
        let text = shown.and_then(|sp| self.cx.loc.ix.text().get(sp.byte_start..sp.byte_end).map(str::to_string));
        let conv = match (&text, value) {
            (Some(t), Expression::Binary { .. }) => format!("({t}) as {e}"),
            (Some(t), _) => format!("{t} as {e}"),
            (None, _) => format!("the value as {e}"),
        };
        let cost = if want == Kind::Int { " (it drops the fraction)" } else { "" };
        self.cx.out.push(d.with_help(format!("convert explicitly: `{conv}`{cost}, or declare it `{f}`")));
    }

    pub(super) fn numeric_into_type(&mut self, want: &Type, value: &Expression<'_>, what: &str) {
        if let Some(k) = rules::type_kind(want) {
            self.numeric_into(k, value, what, false);
        }
    }

    /// The elements of a list literal (`[a, b]`, `vec![a, b]`) stored where `Vec<int>` / `[float; N]` is declared.
    pub(super) fn numeric_elements(&mut self, want: &Type, value: &Expression<'_>, what: &str) {
        let (Type::Vec(inner) | Type::Array(inner, _)) = want else { return };
        let elements: Vec<&Expression<'_>> = match value {
            Expression::Array { elements, .. } => elements.clone(),
            Expression::MacroInvocation { name, args, is_repeat, .. } if name == "vec" => {
                if *is_repeat { args.iter().take(1).copied().collect() } else { args.clone() }
            }
            _ => return,
        };
        for el in elements {
            self.numeric_into_type(inner, el, what);
        }
    }

    /// `f(a, b)`, `Type::make(a)`, `module.f(a)` and `value.method(a)`: each argument against the declared parameter.
    pub(super) fn numeric_arguments(&mut self, callee: Callee<'_, '_>, arguments: &[(Option<String>, &Expression<'_>)]) {
        if arguments.iter().any(|(label, _)| label.is_some()) {
            return;
        }
        let (params, label) = match callee {
            Callee::Fn(name) => (self.declared_params(name), name.to_string()),
            Callee::Method(object, method) => (self.method_signature(object, method).map(|s| s.0), method.to_string()),
        };
        let Some(params) = params else { return };
        if params.len() != arguments.len() {
            return;
        }
        let what = format!(" in argument of `{label}`");
        for ((_, arg), want) in arguments.iter().zip(params) {
            if let Some(k) = want {
                self.numeric_into(k, arg, &what, false);
            }
        }
    }

    /// `Point { x: a * 2.0 }`: each initialiser against the declared field.
    pub(super) fn numeric_fields(&mut self, name: &str, fields: &[(String, &Expression<'_>)]) {
        let Some(declared) = self.known_struct_fields(name) else { return };
        for (field, value) in fields {
            if let Some((_, ty)) = declared.iter().find(|(n, _)| n == field) {
                let what = format!(" in field `{field}` of `{name}`");
                self.numeric_into_type(ty, value, &what);
                self.numeric_elements(ty, value, &what);
            }
        }
    }

    /// `target = value` for a binding, a field or a list element whose kind is declared.
    pub(super) fn numeric_assignment(&mut self, target: &Expression<'_>, value: &Expression<'_>) {
        let slot = match target {
            Expression::Identifier { name, .. } => match self.scalars.get(name).copied() {
                // A binding typed only by its first literal may still be a float by inference: only a float literal
                // is certainly wrong for it.
                Some((Kind::Int, false)) if !matches!(value, Expression::Literal { value: Literal::Float(_), .. }) => None,
                Some((k, _)) => Some((k, format!(" in assignment to `{name}`"))),
                None => None,
            },
            Expression::FieldAccess { object, field, .. } => {
                let fields = self.struct_of(object).and_then(|t| self.known_struct_fields(&t));
                fields
                    .and_then(|fs| fs.iter().find(|(n, _)| n == field))
                    .and_then(|(_, t)| rules::type_kind(t))
                    .map(|k| (k, format!(" in assignment to `{field}`")))
            }
            Expression::Index { object: Expression::Identifier { name, .. }, .. } => {
                self.list_elems.get(name).map(|k| (*k, format!(" in assignment to an element of `{name}`")))
            }
            _ => None,
        };
        if let Some((k, what)) = slot {
            self.numeric_into(k, value, &what, true);
        }
    }
}

/// The offset just past the `)` that closes the `(` at `open`.
fn close_paren(text: &str, open: usize) -> Option<usize> {
    let rest = text.get(open..)?;
    if !rest.starts_with('(') {
        return None;
    }
    let mut depth = 0usize;
    for (i, c) in rest.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// What a call names, to find its declared parameters.
pub(super) enum Callee<'n, 'e> {
    /// `f(..)`, `Type::make(..)`, `module::f(..)`.
    Fn(&'n str),
    /// `object.method(..)`.
    Method(&'n Expression<'e>, &'n str),
}
