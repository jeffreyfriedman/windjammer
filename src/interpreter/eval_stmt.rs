//! Statement execution: blocks, bindings, assignment, control flow, loops, match.

use super::engine::Interpreter;
use super::error::{panic_ctl, semantic, unsupported, Ctl, R};
use super::eval_expr::stmt_loc;
use super::int::IntTy;
use super::place::deref_value;
use super::value::*;
use crate::parser::{Expression, Pattern, Statement};
use std::rc::Rc;

impl Interpreter {
    /// Execute statements in the *current* scope; the value of the last statement is the result.
    pub(crate) fn exec_stmts(&mut self, stmts: &[&'static Statement<'static>]) -> R {
        let mut last = Value::Unit;
        let n = stmts.len();
        for (i, s) in stmts.iter().enumerate() {
            let v = self.exec_stmt(s)?;
            if i + 1 == n {
                last = v;
            } else if !self.drop_types.is_empty() && matches!(s, Statement::Expression { .. }) {
                // a discarded temporary is dropped at the end of its statement
                self.drop_unreachable(&[v], &[])?;
            }
        }
        Ok(last)
    }

    /// Execute a nested block in its own scope.
    pub(crate) fn exec_block(&mut self, stmts: &[&'static Statement<'static>]) -> R {
        self.push_scope();
        let r = self.exec_stmts(stmts);
        self.pop_scope(r)
    }

    pub(crate) fn run_defers(&mut self, r: R) -> R {
        let defers = std::mem::take(&mut self.scope.defers);
        let mut r = r;
        for d in defers.into_iter().rev() {
            if let Err(e) = self.exec_stmt(d) {
                if r.is_ok() {
                    r = Err(e);
                }
            }
        }
        r
    }

    pub(crate) fn exec_stmt(&mut self, s: &'static Statement<'static>) -> R {
        self.mark(stmt_loc(s));
        match s {
            Statement::Expression { expr, .. } => self.eval_raw(expr),

            Statement::Let { pattern, type_, value, else_block, .. } => {
                if let Some(t) = type_ {
                    self.expect_type_of(value, t);
                }
                let mut v = match pattern {
                    // `let r = &mut x` keeps the reference
                    Pattern::Identifier(_) | Pattern::MutBinding(_) => self.eval_raw_binding(value)?,
                    _ => self.eval(value)?,
                };
                if let Some(t) = type_ {
                    if !matches!(v, Value::Ref(_)) {
                        v = self.coerce(v, t)?;
                    }
                }
                match pattern {
                    Pattern::Identifier(n) | Pattern::MutBinding(n) => {
                        self.define(n.as_str(), v);
                    }
                    Pattern::Wildcard => {
                        self.drop_unreachable(&[v], &[])?;
                    }
                    pat => {
                        let mut binds = Vec::new();
                        if self.match_pattern(pat, &v, None, false, &mut binds) {
                            for (n, b) in binds {
                                self.define(n, b);
                            }
                        } else if let Some(eb) = else_block {
                            let r = self.exec_block(eb);
                            return match r {
                                Ok(_) => Err(semantic("`let ... else` block must diverge")),
                                Err(e) => Err(e),
                            };
                        } else {
                            return Err(semantic("refutable pattern in `let`"));
                        }
                    }
                }
                Ok(Value::Unit)
            }

            Statement::Const { name, value, type_, .. } | Statement::Static { name, value, type_, .. } => {
                let v = self.eval(value)?;
                let v = self.coerce(v, type_)?;
                self.define(name.as_str(), v);
                Ok(Value::Unit)
            }

            Statement::Assignment { target, value, compound_op, .. } => {
                self.exec_assign(target, value, *compound_op)?;
                Ok(Value::Unit)
            }

            Statement::Return { value, .. } => {
                let v = match value {
                    Some(e) => {
                        if let Some(rt) = self.scope.ret_ty {
                            self.expect_type_of(e, rt);
                        }
                        self.eval(e)?
                    }
                    None => Value::Unit,
                };
                Err(Ctl::Return(v))
            }

            Statement::If { condition, then_block, else_block, .. } => {
                let c = deref_value(self.eval(condition)?)?;
                match c {
                    Value::Bool(true) => self.exec_block(then_block),
                    Value::Bool(false) => match else_block {
                        Some(b) => self.exec_block(b),
                        None => Ok(Value::Unit),
                    },
                    other => Err(semantic(format!("`if` condition must be bool, found {}", other.type_name()))),
                }
            }

            Statement::Match { value, arms, .. } => self.exec_match(value, arms),

            Statement::While { condition, body, label, .. } => {
                loop {
                    match self.eval(condition)? {
                        Value::Bool(true) => {}
                        Value::Bool(false) => break,
                        other => return Err(semantic(format!("`while` condition must be bool, found {}", other.type_name()))),
                    }
                    let r = self.exec_block(body);
                    if loop_exit(r, label.as_deref())?.is_some() {
                        break;
                    }
                }
                Ok(Value::Unit)
            }

            // A `loop` evaluates to its `break value` (Unit for a plain `break`).
            Statement::Loop { body, label, .. } => {
                loop {
                    let r = self.exec_block(body);
                    if let Some(v) = loop_exit(r, label.as_deref())? {
                        return Ok(v);
                    }
                }
            }

            Statement::For { pattern, iterable, body, label, .. } => self.exec_for(pattern, iterable, body, label.as_deref()),

            Statement::Break { label, value, .. } => {
                let v = match value {
                    Some(e) => self.eval(e)?,
                    None => Value::Unit,
                };
                Err(Ctl::Break { label: label.clone(), value: v })
            }
            Statement::Continue { label, .. } => Err(Ctl::Continue { label: label.clone() }),
            Statement::Defer { statement, .. } => {
                self.scope.defers.push(statement);
                Ok(Value::Unit)
            }
            Statement::Use { .. } => Ok(Value::Unit),
            Statement::Thread { .. } | Statement::Async { .. } => Err(unsupported("`thread {}` / `async {}` blocks (ADR-0004: surface not settled; use `async fn` + `.await`)")),
        }
    }

    fn eval_raw_binding(&mut self, e: &'static Expression<'static>) -> R {
        if let Expression::Unary { op: crate::parser::UnaryOp::MutRef, .. } = e {
            return self.eval_raw(e);
        }
        // a guard/reference returned by a call (`cell.borrow_mut()`, `lock().unwrap()`) stays a reference
        if let Expression::MethodCall { .. } | Expression::Block { .. } = e {
            return self.eval_raw(e);
        }
        self.eval(e)
    }

    fn exec_assign(
        &mut self,
        target: &'static Expression<'static>,
        value: &'static Expression<'static>,
        op: Option<crate::parser::CompoundOp>,
    ) -> R<()> {
        let rhs = self.eval_keep_guard(value)?;
        if let Expression::Identifier { name, .. } = target {
            if name == "_" {
                return Ok(());
            }
        }
        let place = match self.resolve_place(target)? {
            Some(p) => p,
            None => return Err(semantic("invalid left-hand side of assignment")),
        };
        // writing a Ref value into a plain variable rebinds the variable
        if let (Expression::Identifier { name, .. }, Value::Ref(_)) = (target, &rhs) {
            if let Some(slot) = self.lookup_var(name) {
                *slot.borrow_mut() = rhs;
                return Ok(());
            }
        }
        self.mark(super::eval_expr::expr_loc(target));
        let new = match op {
            None => {
                let old = place.read().ok();
                match old {
                    Some(o) => retag(o, rhs),
                    None => rhs,
                }
            }
            Some(cop) => {
                let cur = place.read()?;
                let cur = deref_value(cur)?;
                if let (Value::Str(a), Value::Str(b), crate::parser::CompoundOp::Add) = (&cur, &rhs, cop) {
                    // in-place string append (amortised O(1))
                    let b = b.clone();
                    let _ = a;
                    place.with_mut(|t| {
                        if let Value::Str(rc) = t {
                            Rc::make_mut(rc).push_str(&b);
                        }
                        Ok(())
                    })?;
                    return Ok(());
                }
                let bop = super::ops::compound_to_binary(cop);
                self.binary(bop, cur, rhs)?
            }
        };
        let old = if self.drop_types.is_empty() { None } else { place.read().ok() };
        place.write(new)?;
        if let Some(old) = old {
            self.drop_unreachable(&[old], &[])?;
        }
        Ok(())
    }

    fn exec_for(
        &mut self,
        pattern: &'static Pattern<'static>,
        iterable: &'static Expression<'static>,
        body: &'static [&'static Statement<'static>],
        label: Option<&'static str>,
    ) -> R {
        let raw = self.eval_raw(iterable)?;
        // by-reference iteration: `for x in &mut v`
        let src = match raw {
            Value::Ref(p) => {
                let cur = p.read()?;
                match cur {
                    Value::Vec(items) => {
                        let refs: Vec<Value> = (0..items.len()).map(|i| Value::Ref(p.proj(Proj::Index(i)))).collect();
                        Value::vec(refs)
                    }
                    Value::Map(m) => {
                        let items: Vec<Value> = m
                            .keys()
                            .map(|k| Value::tuple(vec![k.0.clone(), Value::Ref(p.proj(Proj::Key(k.0.clone())))]))
                            .collect();
                        Value::vec(items)
                    }
                    other => other,
                }
            }
            other => other,
        };
        // lazy integer ranges
        if let Value::Range(a, b, ty) = &src {
            let (mut i, end, ty) = (*a, *b, *ty);
            while i < end {
                if self.run_iteration(pattern, Value::Int(i, ty), body, label)?.is_some() {
                    break;
                }
                i += 1;
            }
            return Ok(Value::Unit);
        }
        let src = if let Value::Native(_) = &src { Value::Iter(super::iter::as_iter(self, src)?) } else { src };
        if let Value::Iter(s) = &src {
            while let Some(item) = super::iter::next(self, s)? {
                if self.run_iteration(pattern, item, body, label)?.is_some() {
                    break;
                }
            }
            return Ok(Value::Unit);
        }
        let items = self.to_items(src)?;
        for item in items {
            if self.run_iteration(pattern, item, body, label)?.is_some() {
                break;
            }
        }
        Ok(Value::Unit)
    }

    /// `Some(value)` when the loop must stop (`break`), `None` to keep iterating.
    fn run_iteration(&mut self, pattern: &'static Pattern<'static>, item: Value, body: &'static [&'static Statement<'static>], label: Option<&str>) -> R<Option<Value>> {
        self.push_scope();
        match pattern {
            Pattern::Identifier(n) | Pattern::MutBinding(n) => self.define(n.as_str(), item),
            Pattern::Wildcard => {}
            pat => {
                let mut binds = Vec::new();
                if !self.match_pattern(pat, &item, None, false, &mut binds) {
                    let _ = self.pop_scope(Ok(Value::Unit));
                    return Err(semantic("refutable pattern in `for`"));
                }
                for (n, v) in binds {
                    self.define(n, v);
                }
            }
        }
        let r = self.exec_stmts(body);
        let r = self.pop_scope(r);
        loop_exit(r, label)
    }

    /// Materialise anything iterable.
    pub(crate) fn to_items(&mut self, v: Value) -> R<Vec<Value>> {
        match deref_value(v)? {
            Value::Vec(v) => Ok(v.to_vec()),
            Value::Range(a, b, ty) => {
                if b - a > 50_000_000 {
                    return Err(unsupported("materialising a range of more than 50M elements"));
                }
                Ok((a..b).map(|i| Value::Int(i, ty)).collect())
            }
            Value::Iter(s) => self.drain_iter(&s),
            n @ Value::Native(_) => {
                let s = super::iter::as_iter(self, n)?;
                self.drain_iter(&s)
            }
            Value::Map(m) => Ok(m.iter().map(|(k, v)| Value::tuple(vec![k.0.clone(), v.clone()])).collect()),
            Value::Set(m) => Ok(m.keys().map(|k| k.0.clone()).collect()),
            Value::Enum(e) if &*e.ty == "Option" => Ok(match &e.payload {
                Payload::Tuple(p) => p.clone(),
                _ => vec![],
            }),
            other => Err(semantic(format!("`{}` is not iterable", other.type_name()))),
        }
    }

    fn exec_match(
        &mut self,
        value: &'static Expression<'static>,
        arms: &'static [crate::parser::MatchArm<'static>],
    ) -> R {
        // scrutinee: keep a place when mutation through bindings is possible
        let mut by_ref = false;
        let mut place: Option<Place> = None;
        let v = if let Expression::Unary { op: crate::parser::UnaryOp::MutRef, operand, .. } = value {
            by_ref = true;
            place = self.resolve_place(operand)?;
            match &place {
                Some(p) => p.read()?,
                None => self.eval(operand)?,
            }
        } else if Self::is_place_expr(value) {
            let p = self.resolve_place(value)?;
            match p {
                Some(p) => {
                    // a place reached through a reference (&mut self / mutated param) binds by reference
                    by_ref = self.place_is_borrowed(value);
                    let v = p.read()?;
                    place = Some(p);
                    v
                }
                None => self.eval(value)?,
            }
        } else {
            let raw = self.eval_raw(value)?;
            match raw {
                Value::Ref(p) => {
                    by_ref = true;
                    let p = self.chase(p)?;
                    let v = p.read()?;
                    place = Some(p);
                    v
                }
                other => other,
            }
        };
        let v = deref_value(v)?;
        for arm in arms {
            let mut binds = Vec::new();
            self.push_scope();
            if self.match_pattern(&arm.pattern, &v, place.as_ref(), by_ref, &mut binds) {
                for (n, b) in binds {
                    self.define(n, b);
                }
                if let Some(g) = arm.guard {
                    match self.eval(g) {
                        Ok(Value::Bool(true)) => {}
                        Ok(_) => {
                            let _ = self.pop_scope(Ok(Value::Unit));
                            continue;
                        }
                        Err(e) => {
                            return self.pop_scope(Err(e));
                        }
                    }
                }
                let r = self.eval_raw(arm.body);
                return self.pop_scope(r);
            }
            let _ = self.pop_scope(Ok(Value::Unit));
        }
        Err(panic_ctl(format!("no match arm matched value of type `{}`", v.type_name())))
    }

    /// True when the root variable of a place expression holds a `&mut` reference.
    fn place_is_borrowed(&self, e: &'static Expression<'static>) -> bool {
        match e {
            Expression::Identifier { name, .. } => self
                .lookup_var(name)
                .map(|s| matches!(&*s.borrow(), Value::Ref(_)))
                .unwrap_or(false),
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => self.place_is_borrowed(object),
            _ => false,
        }
    }
}

/// An assignment keeps the declared width of the old value when the new one is a bare literal.
fn retag(old: Value, new: Value) -> Value {
    match (&old, &new) {
        (Value::Int(_, ot), Value::Int(n, IntTy::Lit)) if *ot != IntTy::Lit && ot.fits(*n) => Value::Int(*n, *ot),
        (Value::Float(_, ot), Value::Float(n, FloatTy::Lit)) => Value::Float(*n, *ot),
        (Value::Float(_, ot), Value::Int(n, IntTy::Lit)) => Value::Float(*n as f64, *ot),
        _ => new,
    }
}

/// Decide what a loop does with the result of one body execution.
/// `Ok(None)`: keep looping (normal completion, or a `continue` aimed at this loop).
/// `Ok(Some(v))`: stop with value `v` (a `break` aimed at this loop).
/// `Err(..)`: propagate (return, error, or a labeled `break`/`continue` aimed at an outer loop).
/// A jump without a label targets the innermost loop; a labeled jump targets the loop that
/// carries exactly that label.
pub(crate) fn loop_exit(r: R, my_label: Option<&str>) -> R<Option<Value>> {
    let mine = |l: &Option<String>| l.is_none() || l.as_deref() == my_label;
    match r {
        Ok(_) => Ok(None),
        Err(Ctl::Continue { label }) if mine(&label) => Ok(None),
        Err(Ctl::Break { label, value }) if mine(&label) => Ok(Some(value)),
        Err(e) => Err(e),
    }
}
