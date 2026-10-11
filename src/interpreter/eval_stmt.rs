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
            Statement::Expression { expr, .. } => self.eval(expr),

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
                    Pattern::Wildcard => {}
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
                let c = self.eval(condition)?;
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

            Statement::While { condition, body, .. } => {
                loop {
                    match self.eval(condition)? {
                        Value::Bool(true) => {}
                        Value::Bool(false) => break,
                        other => return Err(semantic(format!("`while` condition must be bool, found {}", other.type_name()))),
                    }
                    match self.exec_block(body) {
                        Ok(_) | Err(Ctl::Continue) => {}
                        Err(Ctl::Break) => break,
                        Err(e) => return Err(e),
                    }
                }
                Ok(Value::Unit)
            }

            Statement::Loop { body, .. } => {
                loop {
                    match self.exec_block(body) {
                        Ok(_) | Err(Ctl::Continue) => {}
                        Err(Ctl::Break) => break,
                        Err(e) => return Err(e),
                    }
                }
                Ok(Value::Unit)
            }

            Statement::For { pattern, iterable, body, .. } => self.exec_for(pattern, iterable, body),

            Statement::Break { .. } => Err(Ctl::Break),
            Statement::Continue { .. } => Err(Ctl::Continue),
            Statement::Defer { statement, .. } => {
                self.scope.defers.push(statement);
                Ok(Value::Unit)
            }
            Statement::Use { .. } => Ok(Value::Unit),
            Statement::Thread { .. } | Statement::Async { .. } => Err(unsupported("thread/async blocks")),
        }
    }

    fn eval_raw_binding(&mut self, e: &'static Expression<'static>) -> R {
        if let Expression::Unary { op: crate::parser::UnaryOp::MutRef, .. } = e {
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
        let rhs = self.eval(value)?;
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
                if self.run_iteration(pattern, Value::Int(i, ty), body)? {
                    break;
                }
                i += 1;
            }
            return Ok(Value::Unit);
        }
        if let Value::Iter(s) = &src {
            while let Some(item) = super::iter::next(self, s)? {
                if self.run_iteration(pattern, item, body)? {
                    break;
                }
            }
            return Ok(Value::Unit);
        }
        let items = self.to_items(src)?;
        for item in items {
            if self.run_iteration(pattern, item, body)? {
                break;
            }
        }
        Ok(Value::Unit)
    }

    /// Returns true when the loop must stop (`break`).
    fn run_iteration(&mut self, pattern: &'static Pattern<'static>, item: Value, body: &'static [&'static Statement<'static>]) -> R<bool> {
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
        match r {
            Ok(_) | Err(Ctl::Continue) => Ok(false),
            Err(Ctl::Break) => Ok(true),
            Err(e) => Err(e),
        }
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
                let r = self.eval(arm.body);
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
