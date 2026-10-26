//! Expression evaluation.

use super::engine::{Interpreter, VariantKind};
use super::error::{panic_ctl, semantic, unsupported, Ctl, RuntimeError, R};
use super::int::IntTy;
use super::place::deref_value;
use super::value::*;
use crate::parser::{BinaryOp, Expression, Literal, Statement, Type, UnaryOp};
use crate::source_map::Location;
use std::rc::Rc;

pub(crate) fn expr_loc(e: &'static Expression<'static>) -> Option<&'static Location> {
    use Expression::*;
    match e {
        Literal { location, .. }
        | Identifier { location, .. }
        | Binary { location, .. }
        | Unary { location, .. }
        | Call { location, .. }
        | MethodCall { location, .. }
        | FieldAccess { location, .. }
        | StructLiteral { location, .. }
        | MapLiteral { location, .. }
        | Range { location, .. }
        | Closure { location, .. }
        | Cast { location, .. }
        | Index { location, .. }
        | Tuple { location, .. }
        | Array { location, .. }
        | MacroInvocation { location, .. }
        | TryOp { location, .. }
        | Await { location, .. }
        | AsyncCall { location, .. }
        | SpawnCall { location, .. }
        | ChannelSend { location, .. }
        | ChannelRecv { location, .. }
        | Block { location, .. } => location.as_ref(),
    }
}

pub(crate) fn stmt_loc(s: &'static Statement<'static>) -> Option<&'static Location> {
    use Statement::*;
    match s {
        Let { location, .. }
        | Const { location, .. }
        | Static { location, .. }
        | Assignment { location, .. }
        | Return { location, .. }
        | Expression { location, .. }
        | If { location, .. }
        | Match { location, .. }
        | For { location, .. }
        | Loop { location, .. }
        | While { location, .. }
        | Thread { location, .. }
        | Async { location, .. }
        | Defer { location, .. }
        | Break { location, .. }
        | Continue { location, .. }
        | Use { location, .. } => location.as_ref(),
    }
}

impl Interpreter {
    pub(crate) fn panic_at(&mut self, e: &'static Expression<'static>, msg: String) -> Ctl {
        self.mark(expr_loc(e));
        panic_ctl(msg)
    }

    /// Evaluate to a plain value (references are followed).
    pub(crate) fn eval(&mut self, e: &'static Expression<'static>) -> R {
        let v = self.eval_raw(e)?;
        if let Value::Ref(_) = v {
            super::place::strip_refs(v)
        } else {
            Ok(v)
        }
    }

    /// Evaluate, preserving `&mut` references (used for call results that are borrows).
    pub(crate) fn eval_keep_ref(&mut self, e: &'static Expression<'static>) -> R {
        self.eval_raw(e)
    }

    /// Like `eval`, but a lock/borrow guard stays a guard (so it can be handed to `Condvar::wait`
    /// or reassigned without releasing the lock).
    pub(crate) fn eval_keep_guard(&mut self, e: &'static Expression<'static>) -> R {
        let v = self.eval_raw(e)?;
        match &v {
            Value::Ref(p) if p.guard.is_some() => Ok(v),
            Value::Ref(_) => super::place::strip_refs(v),
            _ => Ok(v),
        }
    }

    pub(crate) fn literal(&self, l: &Literal) -> Value {
        match l {
            Literal::Int(i) => Value::Int(*i as i128, IntTy::Lit),
            Literal::IntSuffixed(i, suf) => match suf.as_str() {
                "f64" => Value::Float(*i as f64, FloatTy::F64),
                "f32" => Value::Float(*i as f64, FloatTy::F32),
                // `u64`/`usize` payloads above i64::MAX are stored as their bit pattern (negative).
                "u64" | "usize" => Value::Int(*i as u64 as i128, IntTy::from_name(suf).unwrap_or(IntTy::Lit)),
                s => Value::Int(*i as i128, IntTy::from_name(s).unwrap_or(IntTy::Lit)),
            },
            Literal::Float(f) => Value::Float(*f, FloatTy::Lit),
            Literal::String(s) => Value::str(s.clone()),
            Literal::Char(c) => Value::Char(*c),
            Literal::Bool(b) => Value::Bool(*b),
        }
    }

    pub(crate) fn eval_raw(&mut self, e: &'static Expression<'static>) -> R {
        match e {
            Expression::Literal { value, .. } => Ok(self.literal(value)),

            Expression::Identifier { name, location } => {
                if let Some(slot) = self.lookup_var(name) {
                    let v = slot.borrow().clone();
                    return Ok(v);
                }
                self.mark(location.as_ref());
                self.eval_path_value(name)
            }

            Expression::Binary { left, op, right, location } => {
                match op {
                    BinaryOp::And => {
                        let l = self.eval(left)?;
                        return match l {
                            Value::Bool(false) => Ok(Value::Bool(false)),
                            Value::Bool(true) => self.eval(right),
                            _ => Err(semantic("`&&` needs bool operands")),
                        };
                    }
                    BinaryOp::Or => {
                        let l = self.eval(left)?;
                        return match l {
                            Value::Bool(true) => Ok(Value::Bool(true)),
                            Value::Bool(false) => self.eval(right),
                            _ => Err(semantic("`||` needs bool operands")),
                        };
                    }
                    _ => {}
                }
                let l = self.eval(left)?;
                let r = self.eval(right)?;
                self.mark(location.as_ref());
                self.binary(*op, l, r)
            }

            Expression::Unary { op, operand, location } => match op {
                // the parser reads `-2..3` as `-(2..3)`; a range cannot be negated, so this is `(-2)..3`
                UnaryOp::Neg if matches!(operand, Expression::Range { .. }) => {
                    let Expression::Range { start, end, inclusive, .. } = operand else { unreachable!() };
                    let s = self.eval(start)?;
                    let s = self.negate(s)?;
                    let en = self.eval(end)?;
                    match (s, en) {
                        (Value::Int(a, at), Value::Int(b, bt)) => {
                            Ok(Value::Range(a, if *inclusive { b + 1 } else { b }, IntTy::unify(at, bt)))
                        }
                        _ => Err(unsupported("range over non-integer values")),
                    }
                }
                UnaryOp::Neg => {
                    let v = self.eval(operand)?;
                    self.mark(location.as_ref());
                    self.negate(v)
                }
                UnaryOp::Not => {
                    let v = self.eval(operand)?;
                    self.logical_not(v)
                }
                UnaryOp::Deref => {
                    let v = self.eval(operand)?;
                    deref_value(v)
                }
                UnaryOp::Ref => self.eval(operand),
                UnaryOp::MutRef => {
                    if let Some(p) = self.resolve_place(operand)? {
                        Ok(Value::Ref(p))
                    } else {
                        self.eval(operand)
                    }
                }
            },

            Expression::Call { function, arguments, location } => {
                self.mark(location.as_ref());
                let expected = self.take_expected(e);
                self.eval_call(function, arguments, expected)
            }

            Expression::MethodCall { object, method, type_args, arguments, location } => {
                self.eval_method_call(e, object, method, type_args.as_ref(), arguments, location.as_ref())
            }

            Expression::FieldAccess { object, field, location } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if self.lookup_var(name).is_none() && (self.enums.contains_key(name.as_str()) || self.structs.contains_key(name.as_str())) {
                        self.mark(location.as_ref());
                        return self.eval_path_value(&format!("{}::{}", name, field));
                    }
                }
                let obj = self.eval(object)?;
                self.mark(location.as_ref());
                self.field_of(&obj, field)
            }

            Expression::StructLiteral { name, fields, location } => {
                self.mark(location.as_ref());
                let v = self.eval_struct_literal(name, fields)?;
                self.note_moves(fields.iter().map(|(_, e)| *e));
                Ok(v)
            }

            Expression::MapLiteral { pairs, .. } => {
                let mut m = std::collections::BTreeMap::new();
                for (k, v) in pairs {
                    let k = self.eval(k)?;
                    let v = self.eval(v)?;
                    m.insert(Key(k), v);
                }
                Ok(Value::hash_map(m))
            }

            Expression::Range { start, end, inclusive, .. } => {
                let s = self.eval(start)?;
                let en = self.eval(end)?;
                match (&s, &en) {
                    (Value::Int(a, at), Value::Int(b, bt)) => {
                        let ty = IntTy::unify(*at, *bt);
                        Ok(Value::Range(*a, if *inclusive { *b + 1 } else { *b }, ty))
                    }
                    (Value::Char(a), Value::Char(b)) => {
                        let hi = if *inclusive { *b as u32 + 1 } else { *b as u32 };
                        let items = (*a as u32..hi).filter_map(char::from_u32).map(Value::Char).collect();
                        Ok(Value::vec(items))
                    }
                    _ => Err(unsupported("range over non-integer values")),
                }
            }

            Expression::Closure { parameters, body, .. } => {
                self.closure_counter += 1;
                Ok(Value::Func(Rc::new(Callable::Closure {
                    params: parameters.as_slice(),
                    body,
                    env: self.scope.vars.clone(),
                    module: self.scope.module,
                    self_ty: self.scope.self_ty.clone(),
                    tparams: self.scope.tparams.clone(),
                })))
            }

            Expression::Cast { expr, type_, location } => {
                let v = self.eval(expr)?;
                self.mark(location.as_ref());
                self.cast(v, type_)
            }

            Expression::Index { object, index, location } => {
                let obj = self.eval(object)?;
                let idx = self.eval(index)?;
                self.mark(location.as_ref());
                self.index_value(&obj, &idx)
            }

            Expression::Tuple { elements, .. } => {
                let mut items = Vec::with_capacity(elements.len());
                for x in elements {
                    items.push(self.eval(x)?);
                }
                self.note_moves(elements.iter().copied());
                Ok(Value::tuple(items))
            }

            Expression::Array { elements, .. } => {
                let mut items = Vec::with_capacity(elements.len());
                for x in elements {
                    items.push(self.eval(x)?);
                }
                self.note_moves(elements.iter().copied());
                Ok(Value::vec(items))
            }

            Expression::MacroInvocation { name, args, is_repeat, location, .. } => {
                self.mark(location.as_ref());
                self.eval_macro(e, name, args, *is_repeat)
            }

            Expression::TryOp { expr, location } => {
                let v = self.eval(expr)?;
                self.mark(location.as_ref());
                self.eval_try(v)
            }

            Expression::Block { statements, .. } => {
                self.push_scope();
                let r = self.exec_stmts(statements);
                self.pop_scope(r)
            }

            Expression::Await { expr, location } => {
                let v = self.eval(expr)?;
                self.mark(location.as_ref());
                self.await_value(v)
            }

            Expression::AsyncCall { expr, location } | Expression::SpawnCall { expr, location } => {
                self.mark(location.as_ref());
                let body = self.task_body(expr)?;
                super::builtins::thread::start_task(self, body)
            }

            Expression::ChannelSend { location, .. }
            | Expression::ChannelRecv { location, .. } => {
                self.mark(location.as_ref());
                Err(unsupported(
                    "`async expr`, `spawn`, channels and `<-` (ADR-0004: the concurrency surface is not settled; only `async fn` + `.await` run, sequentially)",
                ))
            }
        }
    }

    /// What a `spawn`/`async` task runs: a call evaluates its callee and arguments now (as Rust
    /// does) and runs later; any other operand (a block) becomes a closure over the current scope.
    fn task_body(&mut self, e: &'static Expression<'static>) -> R {
        if let Expression::Call { function, arguments, .. } = e {
            let func = self.eval(function)?;
            let args = self.eval_args(arguments, None, false)?;
            return Ok(Value::Func(Rc::new(Callable::Thunk { func, args })));
        }
        Ok(Value::Func(Rc::new(Callable::Closure {
            params: &[],
            body: e,
            env: self.scope.vars.clone(),
            module: self.scope.module,
            self_ty: self.scope.self_ty.clone(),
            tparams: self.scope.tparams.clone(),
        })))
    }

    fn eval_try(&mut self, v: Value) -> R {
        match &v {
            Value::Enum(en) if &*en.ty == "Option" => match &en.payload {
                Payload::Tuple(p) if en.idx == 1 => Ok(p[0].clone()),
                _ => Err(Ctl::Return(Value::none())),
            },
            Value::Enum(en) if &*en.ty == "Result" => match &en.payload {
                Payload::Tuple(p) if en.idx == 0 => Ok(p[0].clone()),
                Payload::Tuple(p) => {
                    let converted = self.convert_error(p[0].clone())?;
                    Err(Ctl::Return(Value::err(converted)))
                }
                _ => Err(semantic("malformed Result")),
            },
            other => Err(semantic(format!("`?` on {}", other.type_name()))),
        }
    }

    /// `From::from` conversion applied by `?` when the function's error type is a user type.
    fn convert_error(&mut self, e: Value) -> R {
        if let Some(Type::Result(_, er)) = self.scope.ret_ty {
            if let Type::Custom(target) = &**er {
                if e.type_name() != *target {
                    if let Some(f) = self.methods.get(&(target.clone(), "from".to_string())).cloned() {
                        return self.call_func(&f, vec![e], None);
                    }
                }
            }
        }
        Ok(e)
    }

    pub(crate) fn field_of(&mut self, obj: &Value, field: &str) -> R {
        let peeled;
        let obj = if let Value::Rc(_) = obj {
            peeled = deref_value(obj.clone())?;
            &peeled
        } else {
            obj
        };
        match obj {
            Value::Struct(s) => s
                .fields
                .iter()
                .find(|(n, _)| &**n == field)
                .map(|(_, v)| v.clone())
                .ok_or_else(|| semantic(format!("no field `{}` on type `{}`", field, s.name))),
            Value::Tuple(t) => {
                let i: usize = field.parse().map_err(|_| semantic(format!("no field `{}` on tuple", field)))?;
                t.get(i).cloned().ok_or_else(|| semantic("tuple index out of range"))
            }
            Value::Enum(e) => match &e.payload {
                Payload::Struct(fs) => fs
                    .iter()
                    .find(|(n, _)| &**n == field)
                    .map(|(_, v)| v.clone())
                    .ok_or_else(|| semantic(format!("no field `{}`", field))),
                _ => Err(semantic(format!("no field `{}` on enum `{}`", field, e.ty))),
            },
            // `mime.APPLICATION_JSON`: a constant of a standard-library module imported by name
            Value::Func(f) if matches!(&**f, Callable::Path(p) if self.builtins.constant(&format!("{p}::{field}")).is_some()) => {
                let Callable::Path(p) = &**f else { unreachable!() };
                Ok(self.builtins.constant(&format!("{p}::{field}")).expect("checked"))
            }
            other => Err(semantic(format!("no field `{}` on value of type `{}`", field, other.type_name()))),
        }
    }

    pub(crate) fn index_value(&mut self, obj: &Value, idx: &Value) -> R {
        if matches!(obj, Value::Rc(_)) || matches!(idx, Value::Rc(_)) {
            return self.index_value(&deref_value(obj.clone())?, &deref_value(idx.clone())?);
        }
        match (obj, idx) {
            (Value::Vec(v), Value::Int(i, _)) => {
                if *i < 0 || *i as usize >= v.len() {
                    return Err(panic_ctl(format!(
                        "index out of bounds: the len is {} but the index is {}",
                        v.len(),
                        i
                    )));
                }
                Ok(v[*i as usize].clone())
            }
            (Value::Vec(v), Value::Range(a, b, _)) => {
                let (a, b) = (*a, *b);
                if let Some(msg) = super::slicing::slice_error(a, b, v.len()) {
                    return Err(panic_ctl(msg));
                }
                Ok(Value::vec(v[a as usize..b as usize].to_vec()))
            }
            (Value::Str(s), Value::Range(a, b, _)) => {
                if let Some(msg) = super::slicing::str_slice_error(s, *a, *b) {
                    return Err(panic_ctl(msg));
                }
                Ok(Value::str(s[*a as usize..*b as usize].to_string()))
            }
            (Value::Map(m), k) => m
                .get(&Key(k.clone()))
                .cloned()
                .ok_or_else(|| panic_ctl("key not found in map")),
            (o, i) => Err(unsupported(format!("indexing {} with {}", o.type_name(), i.type_name()))),
        }
    }

    fn eval_struct_literal(
        &mut self,
        name: &'static str,
        fields: &'static [(String, &'static Expression<'static>)],
    ) -> R {
        self.eval_struct_literal_in(name, fields, &mut super::place::WalkerEnv)
    }

    /// `eval_struct_literal` with the field expressions evaluated by `env` (also the resolved engine's).
    pub(crate) fn eval_struct_literal_in(
        &mut self,
        name: &'static str,
        fields: &'static [(String, &'static Expression<'static>)],
        env: &mut dyn super::place::PlaceEnv,
    ) -> R {
        if matches!(name, "RangeFrom" | "std::ops::RangeFrom") && !self.structs.contains_key("RangeFrom") {
            // `start..`: an unbounded integer range (`end == i128::MAX` marks "no end")
            return match fields {
                [(f, ex)] if f == "start" => match env.eval(self, ex)? {
                    Value::Int(a, t) => Ok(Value::Range(a, i128::MAX, t)),
                    other => Err(unsupported(format!("`..` range from {}", other.type_name()))),
                },
                _ => Err(semantic("malformed RangeFrom")),
            };
        }
        let mut name = name.to_string();
        if let Some(rest) = name.strip_prefix("Self::") {
            name = format!("{}::{}", self.scope.self_ty.as_deref().unwrap_or("Self"), rest);
        } else if name == "Self" {
            name = self.scope.self_ty.as_deref().unwrap_or("Self").to_string();
        }
        // enum struct variant: Enum::Variant { .. }
        if let Some((en, var)) = name.rsplit_once("::") {
            let en = en.rsplit("::").next().unwrap_or(en).to_string();
            let target = self.type_aliases.get(&en).and_then(|t| if let Type::Custom(n) = t { Some(n.clone()) } else { None }).unwrap_or(en);
            let variant_fields = self.enums.get(&target).and_then(|def| {
                def.variants.iter().enumerate().find(|(_, v)| v.name == var).and_then(|(idx, vd)| match &vd.kind {
                    VariantKind::Struct(ftys) => Some((idx, ftys.clone())),
                    _ => None,
                })
            });
            if let Some((idx, ftys)) = variant_fields {
                let mut out = Vec::new();
                for (fname, ty) in &ftys {
                    let (_, ex) = fields
                        .iter()
                        .find(|(n, _)| n == fname)
                        .ok_or_else(|| semantic(format!("missing field `{}` in `{}`", fname, name)))?;
                    let v = env.eval(self, ex)?;
                    out.push((Rc::from(*fname), self.coerce(v, ty)?));
                }
                return Ok(Value::enum_val(&target, var, idx as u32, Payload::Struct(out)));
            }
            name = var.to_string();
        }
        if let Some(Type::Custom(n)) = self.type_aliases.get(&name) {
            name = n.clone();
        }
        if !self.structs.contains_key(&name) {
            if let Some(names) = self.builtins.constructible_fields(&name) {
                return self.eval_host_struct_literal(&name, names, fields, env);
            }
        }
        let (decl_fields, copy, rc_name, rc_fields) = match self.structs.get(&name) {
            Some(d) => (d.fields.clone(), d.copy, d.rc_name.clone(), d.rc_fields.clone()),
            None => return Err(semantic(format!("cannot find struct `{}`", name))),
        };
        let mut out: Vec<(Rc<str>, Value)> = Vec::with_capacity(decl_fields.len());
        for ((fname, ty), rc_fname) in decl_fields.iter().zip(rc_fields.iter()) {
            match fields.iter().find(|(n, _)| n == fname) {
                Some((_, ex)) => {
                    let v = env.eval(self, ex)?;
                    out.push((rc_fname.clone(), self.coerce(v, ty)?));
                }
                None => {
                    // struct update syntax `..base` is parsed as a field named ".."
                    if let Some((_, base_ex)) = fields.iter().find(|(n, _)| n == "..") {
                        let base = env.eval(self, base_ex)?;
                        let v = self.field_of(&base, fname)?;
                        out.push((rc_fname.clone(), v));
                    } else {
                        return Err(semantic(format!("missing field `{}` in initializer of `{}`", fname, name)));
                    }
                }
            }
        }
        for (n, _) in fields {
            if n != ".." && !decl_fields.iter().any(|(f, _)| f == n) {
                return Err(semantic(format!("struct `{}` has no field named `{}`", name, n)));
            }
        }
        Ok(self.make_struct_named(rc_name, out, copy))
    }

    /// Struct literal of a host type with a fixed field list (`std::http::ServerRequest`): every field is
    /// required, as in the runtime's definition.
    fn eval_host_struct_literal(
        &mut self,
        name: &str,
        names: &'static [&'static str],
        fields: &'static [(String, &'static Expression<'static>)],
        env: &mut dyn super::place::PlaceEnv,
    ) -> R {
        let mut out: Vec<(Rc<str>, Value)> = Vec::with_capacity(names.len());
        for fname in names {
            let (_, ex) = fields
                .iter()
                .find(|(n, _)| n == fname)
                .ok_or_else(|| semantic(format!("missing field `{}` in initializer of `{}`", fname, name)))?;
            out.push((Rc::from(*fname), env.eval(self, ex)?));
        }
        for (n, _) in fields {
            if !names.contains(&n.as_str()) {
                return Err(semantic(format!("struct `{}` has no field named `{}`", name, n)));
            }
        }
        Ok(self.make_struct(name, out, false))
    }

    pub(crate) fn make_variant(&mut self, enum_name: &str, idx: usize, args: Vec<Value>) -> R {
        let def = self.enums.get(enum_name).ok_or_else(|| semantic(format!("unknown enum `{}`", enum_name)))?;
        let vd = &def.variants[idx];
        let (ty, variant) = (Name::Shared(def.rc_name.clone()), Name::Shared(def.rc_variants[idx].clone()));
        let named = move |payload| Value::Enum(Rc::new(EnumVal { ty, variant, idx: idx as u32, payload }));
        match &vd.kind {
            VariantKind::Unit => Ok(named(Payload::Unit)),
            VariantKind::Tuple(tys) => {
                if tys.len() != args.len() {
                    return Err(semantic(format!(
                        "variant `{}::{}` takes {} argument(s) but {} supplied",
                        enum_name,
                        vd.name,
                        tys.len(),
                        args.len()
                    )));
                }
                let tys = tys.clone();
                let mut out = Vec::new();
                for (a, t) in args.into_iter().zip(tys.iter()) {
                    out.push(self.coerce(a, t)?);
                }
                Ok(named(Payload::Tuple(out)))
            }
            VariantKind::Struct(_) => Err(semantic("struct variant needs braces")),
        }
    }

    fn eval_macro(
        &mut self,
        whole: &'static Expression<'static>,
        name: &str,
        args: &'static [&'static Expression<'static>],
        is_repeat: bool,
    ) -> R {
        if let Some(intr) = crate::intrinsics::Intrinsic::from_macro_name(name) {
            let args: Vec<&'static Expression<'static>> = args.to_vec();
            return self.eval_intrinsic(intr, whole, &args, is_repeat);
        }
        match name {
            "unimplemented" | "debug_assert" | "debug_assert_eq" | "debug_assert_ne" => {
                let canon = name.trim_start_matches("debug_");
                match (name, crate::intrinsics::Intrinsic::from_call_name(canon)) {
                    (_, Some(intr)) if name != "unimplemented" => self.eval_intrinsic(intr, whole, args, false),
                    _ => {
                        let m = match args.len() {
                            0 => "not implemented".to_string(),
                            _ => {
                                let vals = self.eval_all(args)?;
                                format!("not implemented: {}", self.format_args(&vals)?)
                            }
                        };
                        Err(self.panic_at(whole, m))
                    }
                }
            }
            "write" | "writeln" => {
                if args.is_empty() {
                    return Err(semantic("write! needs a destination"));
                }
                let dest = self.eval_raw(args[0])?;
                let vals = self.eval_all(&args[1..])?;
                let mut s = self.format_args(&vals)?;
                if name == "writeln" {
                    s.push('\n');
                }
                match deref_value(dest.clone())? {
                    Value::Formatter(f) => {
                        f.borrow_mut().push_str(&s);
                        Ok(Value::ok(Value::Unit))
                    }
                    Value::Str(_) => {
                        if let Some(p) = self.resolve_place(args[0])? {
                            p.with_mut(|t| {
                                if let Value::Str(rc) = t {
                                    Rc::make_mut(rc).push_str(&s);
                                }
                                Ok(())
                            })?;
                            Ok(Value::ok(Value::Unit))
                        } else {
                            Err(semantic("write! destination is not a place"))
                        }
                    }
                    other => Err(unsupported(format!("write! to {}", other.type_name()))),
                }
            }
            "matches" => Err(unsupported("matches! macro")),
            other => Err(unsupported(format!("macro `{}!`", other))),
        }
    }

    pub(crate) fn eval_all(&mut self, args: &'static [&'static Expression<'static>]) -> R<Vec<Value>> {
        let mut out = Vec::with_capacity(args.len());
        for a in args {
            out.push(self.eval(a)?);
        }
        Ok(out)
    }

    pub(crate) fn runtime_error(&self, e: RuntimeError) -> Ctl {
        Ctl::Err(Box::new(e))
    }
}
