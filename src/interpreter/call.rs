//! Name resolution and calls: functions, methods, closures, enum constructors, builtins.

use super::analysis;
use super::builtins::{Kind, MethodImpl};
use super::engine::*;
use super::error::{semantic, unsupported, Ctl, R};
use super::place::deref_value;
use super::value::*;
use crate::parser::{Expression, Pattern, Type};
use crate::source_map::Location;
use std::rc::Rc;

pub(crate) enum Resolved {
    Func(Rc<FuncDef>),
    Const(usize, String),
    Variant(String, usize),
    TupleStruct(String),
    Builtin(String),
}

impl Interpreter {
    // ------------------------------------------------------------------
    // Name resolution
    // ------------------------------------------------------------------

    fn follow_import(&mut self, imp: Import, rest: &[&str]) -> R<Option<Resolved>> {
        match imp {
            Import::Std(p) => {
                let mut full = p;
                for r in rest {
                    full.push_str("::");
                    full.push_str(r);
                }
                Ok(Some(Resolved::Builtin(full)))
            }
            Import::Module(m) => {
                if rest.is_empty() {
                    Ok(None)
                } else {
                    self.resolve_in(m, rest)
                }
            }
            Import::Item(m, name) => {
                let mut segs: Vec<&str> = name.split("::").collect();
                segs.extend_from_slice(rest);
                self.resolve_in(m, &segs)
            }
        }
    }

    pub(crate) fn resolve_in(&mut self, module: usize, segs: &[&str]) -> R<Option<Resolved>> {
        if segs.is_empty() {
            return Ok(None);
        }
        let first = segs[0];
        let rest = &segs[1..];
        let mut module = module;
        let mut first = first.to_string();
        let mut rest: Vec<&str> = rest.to_vec();
        match first.as_str() {
            "Self" => {
                if let Some(t) = self.scope.self_ty.clone() {
                    first = t.to_string();
                }
            }
            "crate" => {
                if rest.is_empty() {
                    return Ok(None);
                }
                module = 0;
                first = rest[0].to_string();
                rest = rest[1..].to_vec();
            }
            "self" => {
                if rest.is_empty() {
                    return Ok(None);
                }
                first = rest[0].to_string();
                rest = rest[1..].to_vec();
            }
            "super" => {
                if rest.is_empty() {
                    return Ok(None);
                }
                module = self.modules[module].parent.unwrap_or(0);
                first = rest[0].to_string();
                rest = rest[1..].to_vec();
            }
            _ => {}
        }
        if let Some(Type::Custom(n)) = self.type_aliases.get(&first) {
            first = n.clone();
        }
        if rest.is_empty() {
            if let Some(f) = self.modules[module].funcs.get(&first) {
                return Ok(Some(Resolved::Func(f.clone())));
            }
            if self.modules[module].consts.contains_key(&first) {
                return Ok(Some(Resolved::Const(module, first)));
            }
            if let Some(imp) = self.modules[module].imports.get(&first).cloned() {
                if let Some(r) = self.follow_import(imp, &[])? {
                    return Ok(Some(r));
                }
            }
            if let Some(st) = self.structs.get(&first) {
                if st.tuple {
                    return Ok(Some(Resolved::TupleStruct(first)));
                }
            }
            return Ok(None);
        }
        // module-qualified
        if let Some(&child) = self.modules[module].children.get(&first) {
            return self.resolve_in(child, &rest);
        }
        if let Some(imp) = self.modules[module].imports.get(&first).cloned() {
            if let Some(r) = self.follow_import(imp, &rest)? {
                return Ok(Some(r));
            }
        }
        let is_type = self.structs.contains_key(&first)
            || self.enums.contains_key(&first)
            || self.traits.contains_key(&first)
            || self.builtins.has_type(&first);
        if !is_type {
            if let Some(child) = self.load_child_module(module, &first).map_err(Ctl::from)? {
                return self.resolve_in(child, &rest);
            }
        }
        // Type::item
        if rest.len() == 1 {
            let item = rest[0];
            if let Some(en) = self.enums.get(&first) {
                if let Some(idx) = en.variants.iter().position(|v| v.name == item) {
                    return Ok(Some(Resolved::Variant(first, idx)));
                }
            }
            if let Some(f) = self.methods.get(&(first.clone(), item.to_string())) {
                return Ok(Some(Resolved::Func(f.clone())));
            }
            // `Trait::method(x)` style: dispatched at call time
        }
        Ok(None)
    }

    /// Evaluate a path used as a value (`Color::Red`, `i32::MAX`, a function name, a const).
    pub(crate) fn eval_path_value(&mut self, name: &str) -> R {
        let segs: Vec<&str> = name.split("::").collect();
        let module = self.scope.module;
        match self.resolve_in(module, &segs)? {
            Some(Resolved::Func(f)) => Ok(Value::Func(Rc::new(Callable::Named {
                module: f.module,
                name: f.qual.clone(),
            }))),
            Some(Resolved::Const(m, n)) => self.global_value(m, &n),
            Some(Resolved::Variant(en, idx)) => {
                let def = &self.enums[&en];
                match &def.variants[idx].kind {
                    VariantKind::Unit => self.make_variant(&en, idx, vec![]),
                    _ => Ok(Value::Func(Rc::new(Callable::Path(Rc::from(format!("{}::{}", en, def.variants[idx].name)))))),
                }
            }
            Some(Resolved::TupleStruct(n)) => Ok(Value::Func(Rc::new(Callable::Path(Rc::from(n.as_str()))))),
            Some(Resolved::Builtin(p)) => {
                if let Some(v) = self.builtins.constant(&p) {
                    return Ok(v);
                }
                Ok(Value::Func(Rc::new(Callable::Path(Rc::from(p.as_str())))))
            }
            None => {
                if let Some(v) = self.builtins.constant(name) {
                    return Ok(v);
                }
                if self.builtins.has_function(name) {
                    return Ok(Value::Func(Rc::new(Callable::Path(Rc::from(name)))));
                }
                if let Some(p) = self.std_import_member(name, false) {
                    if let Some(v) = self.builtins.constant(&p) {
                        return Ok(v);
                    }
                    return Ok(Value::Func(Rc::new(Callable::Path(Rc::from(p.as_str())))));
                }
                // method path used as a function value: Type::method for builtin types
                Err(semantic(format!("cannot find value `{}` in this scope", name)))
            }
        }
    }

    /// A bare name that a glob-style `use std::module` makes visible (`sqrt` after `use std::math`).
    fn std_import_member(&self, name: &str, function: bool) -> Option<String> {
        let module = self.scope.module;
        self.modules[module].imports.values().find_map(|imp| match imp {
            Import::Std(p) => {
                let path = format!("{}::{}", p, name);
                let hit = if function { self.builtins.has_function(&path) } else { self.builtins.constant(&path).is_some() || self.builtins.has_function(&path) };
                hit.then_some(path)
            }
            _ => None,
        })
    }

    pub(crate) fn global_value(&mut self, module: usize, name: &str) -> R {
        if let Some(slot) = self.globals.get(&(module, name.to_string())) {
            return Ok(slot.borrow().clone());
        }
        let expr = *self.modules[module].consts.get(name).ok_or_else(|| semantic(format!("unknown constant `{}`", name)))?;
        let ty = self.modules[module].const_types.get(name).copied();
        // evaluate in the defining module's context with a fresh scope
        let saved = std::mem::replace(&mut self.scope, Scope::new(module));
        let r = self.eval(expr);
        self.scope = saved;
        let mut v = r?;
        if let Some(t) = ty {
            v = self.coerce(v, t)?;
        }
        self.globals.insert((module, name.to_string()), Rc::new(std::cell::RefCell::new(v.clone())));
        Ok(v)
    }

    // ------------------------------------------------------------------
    // Calls
    // ------------------------------------------------------------------

    pub(crate) fn eval_call(
        &mut self,
        function: &'static Expression<'static>,
        arguments: &'static [(Option<String>, &'static Expression<'static>)],
        expected: Option<Type>,
    ) -> R {
        if let Expression::Identifier { name, .. } = function {
            if self.lookup_var(name).is_none() {
                if let Some(intr) = self.intrinsic_for_call(name) {
                    return self.eval_intrinsic_call(intr, function, arguments);
                }
                return self.call_named(name, arguments, &[], expected);
            }
        }
        if let Expression::FieldAccess { object, field, location } = function {
            // `a.b(args)` parsed as field access + call (e.g. inside match scrutinees)
            if let Expression::Identifier { name, .. } = object {
                if self.lookup_var(name).is_none() && (self.structs.contains_key(name.as_str()) || self.enums.contains_key(name.as_str())) {
                    let path: &'static str = Box::leak(format!("{}::{}", name, field).into_boxed_str());
                    return self.call_named(path, arguments, &[], expected);
                }
            }
            return self.eval_method_call(function, object, field.as_str(), None, arguments, location.as_ref());
        }
        let f = self.eval(function)?;
        let args = self.eval_args(arguments, None, false)?;
        self.call_value(&f, args)
    }

    /// Evaluate call arguments; parameters the callee mutates receive `&mut` references.
    pub(crate) fn eval_args(
        &mut self,
        arguments: &'static [(Option<String>, &'static Expression<'static>)],
        callee: Option<&Rc<FuncDef>>,
        receiver_separate: bool,
    ) -> R<Vec<Value>> {
        let mutated = callee.map(|c| self.param_mutation(c));
        let mut out = Vec::with_capacity(arguments.len());
        // `Type::method(obj, ..)`: arg 0 is self (offset 1); `obj.method(..)`: receiver is separate.
        let offset = if receiver_separate {
            0
        } else {
            callee
                .map(|c| if c.params.first().map(|p| p.name == "self").unwrap_or(false) { 1 } else { 0 })
                .unwrap_or(0)
        };
        for (i, (_, a)) in arguments.iter().enumerate() {
            let want_ref = match (&mutated, callee) {
                (Some(m), Some(_)) => {
                    if i < offset {
                        callee.map(|c| self.self_mutation(c)).unwrap_or(false)
                    } else {
                        m.get(i - offset).copied().unwrap_or(false)
                    }
                }
                _ => false,
            };
            if want_ref && Self::is_place_expr(a) {
                if let Some(p) = self.resolve_place(a)? {
                    out.push(Value::Ref(p));
                    continue;
                }
            }
            out.push(self.eval_raw_for_arg(a)?);
        }
        Ok(out)
    }

    fn eval_raw_for_arg(&mut self, a: &'static Expression<'static>) -> R {
        // explicit `&mut x` keeps its reference; everything else is a plain value
        if let Expression::Unary { op: crate::parser::UnaryOp::MutRef, .. } = a {
            return self.eval_raw(a);
        }
        self.eval(a)
    }

    pub(crate) fn call_named(
        &mut self,
        name: &str,
        arguments: &'static [(Option<String>, &'static Expression<'static>)],
        targs: &[Type],
        expected: Option<Type>,
    ) -> R {
        let module = self.scope.module;
        // fast path: plain function in the current module (hot in recursion)
        if let Some(f) = self.modules[module].funcs.get(name).cloned() {
            let args = self.eval_args(arguments, Some(&f), false)?;
            return self.call_func_g(&f, args, None, targs, expected);
        }
        if matches!(name, "drop" | "mem::drop" | "std::mem::drop") && arguments.len() == 1 && self.lookup_var(name).is_none() {
            return self.explicit_drop(arguments[0].1);
        }
        let mut name = name.to_string();
        // `T::new()` / `T::default()` through a type parameter of the running function
        let mut via_param: Option<Type> = None;
        if let Some((first, rest)) = name.split_once("::") {
            match self.tparam(first) {
                Some(Ok(t)) => {
                    let t = t.clone();
                    name = format!("{}::{}", super::generics::concrete_base(&t), rest);
                    via_param = Some(t);
                }
                Some(Err(())) => {
                    return Err(unsupported(format!(
                        "cannot infer type parameter `{}` for `{}`: pass it explicitly (`f::<Type>(..)`), annotate the binding, or take a value of that type",
                        first, name
                    )))
                }
                None => {}
            }
        }
        if let (Some(t), Some(m)) = (&via_param, name.rsplit("::").next()) {
            if m == "default" && arguments.is_empty() && !name.contains("::default::") {
                let t = t.clone();
                return self.default_for(&t);
            }
        }
        let name = name.as_str();
        let segs: Vec<&str> = name.split("::").collect();
        match self.resolve_in(module, &segs)? {
            Some(Resolved::Func(f)) => {
                let args = self.eval_args(arguments, Some(&f), false)?;
                self.call_func_g(&f, args, None, targs, expected)
            }
            Some(Resolved::Variant(en, idx)) => {
                let args = self.eval_args(arguments, None, false)?;
                self.make_variant(&en, idx, args)
            }
            Some(Resolved::TupleStruct(n)) => {
                let args = self.eval_args(arguments, None, false)?;
                self.make_tuple_struct(&n, args)
            }
            Some(Resolved::Builtin(p)) => {
                let args = self.eval_args(arguments, None, false)?;
                self.call_builtin_fn(&p, args)
            }
            Some(Resolved::Const(m, n)) => {
                let f = self.global_value(m, &n)?;
                let args = self.eval_args(arguments, None, false)?;
                self.call_value(&f, args)
            }
            None => {
                let args = self.eval_args(arguments, None, false)?;
                // `Trait::method(x, ..)` / `Type::method(x)` on builtin types: dispatch on first argument
                if segs.len() >= 2 {
                    let method = segs[segs.len() - 1];
                    let ty = segs[segs.len() - 2];
                    if self.builtins.has_function(name) {
                        return self.call_builtin_fn(name, args);
                    }
                    if !args.is_empty() && (self.traits.contains_key(ty) || self.builtins.has_type(ty)) {
                        let mut args = args;
                        let recv = args.remove(0);
                        return self.call_method_value(recv, method, args, None);
                    }
                    return Err(unsupported(format!(
                        "cannot resolve `{}`: unknown type or unimplemented std item",
                        name
                    )));
                }
                if self.builtins.has_function(name) {
                    return self.call_builtin_fn(name, args);
                }
                if let Some(p) = self.std_import_member(name, true) {
                    return self.call_builtin_fn(&p, args);
                }
                Err(semantic(format!("cannot find function `{}` in this scope", name)))
            }
        }
    }

    /// `drop(x)`: a local argument is moved out (its slot emptied) and destructors run.
    fn explicit_drop(&mut self, arg: &'static Expression<'static>) -> R {
        let v = self.eval(arg)?;
        if let Expression::Identifier { name, .. } = arg {
            if let Some(slot) = self.lookup_var(name) {
                *slot.borrow_mut() = Value::Unit;
            }
        }
        self.drop_unreachable(&[v], &[])?;
        Ok(Value::Unit)
    }

    fn make_tuple_struct(&mut self, name: &str, args: Vec<Value>) -> R {
        let (fields, copy) = {
            let d = &self.structs[name];
            (d.fields.iter().map(|(n, t)| (*n, t.clone())).collect::<Vec<_>>(), d.copy)
        };
        if fields.len() != args.len() {
            return Err(semantic(format!("`{}` takes {} field(s) but {} supplied", name, fields.len(), args.len())));
        }
        let mut out = Vec::new();
        for ((n, t), a) in fields.iter().zip(args) {
            out.push((Rc::from(*n), self.coerce(a, t)?));
        }
        Ok(self.make_struct(name, out, copy))
    }

    pub(crate) fn call_builtin_fn(&mut self, path: &str, args: Vec<Value>) -> R {
        if let Some(r) = super::builtins::assoc_numeric(self, path, &args) {
            return r;
        }
        let f = self.builtins.function(path);
        match f {
            Some(f) => f(self, args),
            None => Err(unsupported(format!("`{}` is not implemented by the interpreter", path))),
        }
    }

    /// Call any function value with already-evaluated arguments.
    pub(crate) fn call_value(&mut self, f: &Value, args: Vec<Value>) -> R {
        match deref_value(f.clone())? {
            Value::Func(c) => self.call_callable(&c, args),
            other => Err(semantic(format!("`{}` is not callable", other.type_name()))),
        }
    }

    pub(crate) fn call_callable(&mut self, c: &Rc<Callable>, mut args: Vec<Value>) -> R {
        match &**c {
            Callable::Closure { params, body, env, module, self_ty, tparams } => {
                self.depth_check()?;
                let mut scope = Scope::new(*module);
                scope.vars = env.clone();
                scope.self_ty = self_ty.clone();
                scope.tparams = tparams.clone();
                scope.ret_ty = None;
                let nenv = scope.vars.len();
                let saved = std::mem::replace(&mut self.scope, scope);
                self.frames.push(saved);
                self.depth += 1;
                // closures called with a single tuple for multiple params (e.g. fold on pairs) are not auto-splatted
                for (i, p) in params.iter().enumerate() {
                    let a = if i < args.len() { std::mem::replace(&mut args[i], Value::Unit) } else { Value::Unit };
                    self.bind_closure_param(p.as_str(), a);
                }
                let r = self.eval(body);
                let r = match r {
                    Err(Ctl::Return(v)) => Ok(v),
                    other => other,
                };
                let r = self.exit_scope_vars(nenv, r);
                self.depth -= 1;
                self.scope = self.frames.pop().expect("frame stack");
                r
            }
            Callable::Named { module, name } => {
                let segs: Vec<&str> = name.split("::").collect();
                let r = self.resolve_in(*module, &segs)?;
                match r {
                    Some(Resolved::Func(f)) => self.call_func(&f, args, None),
                    _ => {
                        if let Some((t, m)) = name.rsplit_once("::") {
                            if let Some(f) = self.methods.get(&(t.to_string(), m.to_string())).cloned() {
                                return self.call_func(&f, args, None);
                            }
                        }
                        Err(semantic(format!("cannot resolve function `{}`", name)))
                    }
                }
            }
            Callable::Deferred { .. } => Err(semantic("a future is not callable; `.await` it")),
            Callable::Path(p) => {
                let p = p.to_string();
                if let Some((en, var)) = p.rsplit_once("::") {
                    if let Some(def) = self.enums.get(en) {
                        if let Some(idx) = def.variants.iter().position(|v| v.name == var) {
                            return self.make_variant(en, idx, args);
                        }
                    }
                }
                if self.structs.get(&p).map(|s| s.tuple).unwrap_or(false) {
                    return self.make_tuple_struct(&p, args);
                }
                self.call_builtin_fn(&p, args)
            }
        }
    }

    fn bind_closure_param(&mut self, p: &'static str, a: Value) {
        let p = p.split(':').next().unwrap_or(p).trim().trim_start_matches('&').trim();
        let p = p.strip_prefix("mut ").unwrap_or(p).trim();
        if p.starts_with('(') {
            let names: Vec<&'static str> = p
                .trim_matches(|c| c == '(' || c == ')')
                .split(',')
                .map(|s| s.trim().trim_start_matches('&').trim())
                .filter(|s| !s.is_empty())
                .collect();
            if let Value::Tuple(items) = &a {
                for (n, v) in names.iter().zip(items.iter()) {
                    self.define(n, v.clone());
                }
                return;
            }
            for n in names {
                self.define(n, a.clone());
            }
            return;
        }
        self.define(p, a);
    }

    fn depth_check(&mut self) -> R<()> {
        let over_stack = match self.stack_guard {
            Some((base, budget)) => {
                let here = 0u8;
                base.saturating_sub(&here as *const u8 as usize) > budget
            }
            None => self.depth > self.max_depth,
        };
        if over_stack {
            return Err(super::error::panic_ctl("stack overflow: recursion too deep for the interpreter stack"));
        }
        Ok(())
    }

    pub(crate) fn call_func(&mut self, f: &Rc<FuncDef>, args: Vec<Value>, self_val: Option<Value>) -> R {
        self.call_func_g(f, args, self_val, &[], None)
    }

    /// `call_func` with explicit type arguments and the expected result type (generic dispatch).
    pub(crate) fn call_func_g(
        &mut self,
        f: &Rc<FuncDef>,
        args: Vec<Value>,
        self_val: Option<Value>,
        targs: &[Type],
        expected: Option<Type>,
    ) -> R {
        if f.is_async {
            // Rust semantics: calling an async fn does nothing until the future is awaited
            return Ok(Value::Func(Rc::new(Callable::Deferred { func: f.clone(), args, self_val, targs: targs.to_vec(), expected })));
        }
        self.call_func_run(f, args, self_val, targs, expected)
    }

    /// `.await`: run a deferred async call to completion; any other value is already a result.
    pub(crate) fn await_value(&mut self, v: Value) -> R {
        match &v {
            Value::Func(c) => match &**c {
                Callable::Deferred { func, args, self_val, targs, expected } => {
                    let r = self.call_func_run(func, args.clone(), self_val.clone(), targs, expected.clone())?;
                    self.await_value(r)
                }
                _ => Ok(v),
            },
            _ => Ok(v),
        }
    }

    fn call_func_run(
        &mut self,
        f: &Rc<FuncDef>,
        mut args: Vec<Value>,
        self_val: Option<Value>,
        targs: &[Type],
        expected: Option<Type>,
    ) -> R {
        if f.is_extern {
            return Err(unsupported(format!("extern function `{}`", f.name)));
        }
        self.depth_check()?;
        self.backtrace.push((f.qual.clone(), self.cur));
        let mut scope = Scope::new(f.module);
        scope.self_ty = f.self_ty.clone();
        scope.ret_ty = f.ret;
        if !f.type_params.is_empty() {
            let exp = match (f.ret, &expected) {
                (Some(r), Some(e)) => Some((r, e)),
                _ => None,
            };
            scope.tparams = self.initial_bindings(&f.type_params, targs, exp);
        }
        let saved = std::mem::replace(&mut self.scope, scope);
        self.frames.push(saved);
        self.depth += 1;

        let has_self = f.params.first().map(|p| p.name == "self").unwrap_or(false);
        let mut it = args.drain(..);
        if !has_self {
            if let Some(v) = self_val {
                self.define("self", v);
            }
        } else {
            let sv = match self_val {
                Some(v) => Some(v),
                None => it.next(),
            };
            if let Some(v) = sv {
                self.define("self", v);
            }
        }
        let mut bind_err: Option<Ctl> = None;
        for p in f.params.iter().filter(|p| p.name != "self") {
            let a = it.next().unwrap_or(Value::Unit);
            self.bind_from_arg(&p.type_, &a);
            let a = match &a {
                Value::Ref(_) => a,
                _ => match self.coerce(a, &p.type_) {
                    Ok(v) => v,
                    Err(e) => {
                        bind_err = Some(e);
                        break;
                    }
                },
            };
            match &p.pattern {
                Some(pat) if !matches!(pat, Pattern::Identifier(_)) => {
                    let mut binds = Vec::new();
                    if self.match_pattern(pat, &a, None, false, &mut binds) {
                        for (n, v) in binds {
                            self.define(n, v);
                        }
                    }
                }
                _ => self.define(p.name.as_str(), a),
            }
        }
        drop(it);

        let r = match bind_err {
            Some(e) => Err(e),
            None => {
                self.expect_tail_type(f);
                self.exec_stmts(f.body)
            }
        };
        let r = self.run_defers(r);
        let r = match r {
            Err(Ctl::Return(v)) => Ok(v),
            Err(Ctl::Break) | Err(Ctl::Continue) => Err(semantic("`break`/`continue` outside of a loop")),
            other => other,
        };
        let r = match r {
            Ok(v) => match f.ret {
                None => Ok(Value::Unit),
                Some(t) => deref_value(v).and_then(|v| self.coerce(v, t)),
            },
            Err(Ctl::Err(e)) => Err(Ctl::Err(Box::new(self.annotate(*e)))),
            other => other,
        };
        let r = self.exit_scope_vars(0, r);
        self.depth -= 1;
        self.scope = self.frames.pop().expect("frame stack");
        self.backtrace.pop();
        r
    }

    // ------------------------------------------------------------------
    // Methods
    // ------------------------------------------------------------------

    pub(crate) fn eval_method_call(
        &mut self,
        whole: &'static Expression<'static>,
        object: &'static Expression<'static>,
        method: &'static str,
        type_args: Option<&'static Vec<Type>>,
        arguments: &'static [(Option<String>, &'static Expression<'static>)],
        loc: Option<&'static Location>,
    ) -> R {
        let _ = whole;
        // match-scrutinee parser quirk: `x.parse::<T>()` arrives as MethodCall{ FieldAccess(x, parse), method: "" }
        if method.is_empty() {
            match object {
                Expression::FieldAccess { object: inner, field, .. } => {
                    return self.eval_method_call(whole, inner, field.as_str(), type_args, arguments, loc);
                }
                Expression::Identifier { name, .. } => {
                    // `f::<T>(args)` is a plain call with a turbofish
                    self.mark(loc);
                    self.type_hint = type_args.and_then(|t| t.first().cloned());
                    let targs: &[Type] = type_args.map(|v| v.as_slice()).unwrap_or(&[]);
                    let expected = self.take_expected(whole);
                    return self.call_named(name.as_str(), arguments, targs, expected);
                }
                _ => {}
            }
        }
        // legacy `Enum.Variant(args)` / `Type.assoc(args)` spelling
        if let Expression::Identifier { name, .. } = object {
            if self.lookup_var(name).is_none() && (self.enums.contains_key(name.as_str()) || self.structs.contains_key(name.as_str())) {
                let path = format!("{}::{}", name, method);
                self.mark(loc);
                return self.call_named(&path, arguments, &[], None);
            }
        }
        // legacy `module.func(args)` on an imported std module (`fs.exists(p)`, `time.now_millis()`)
        if let Expression::Identifier { name, .. } = object {
            if self.lookup_var(name).is_none() {
                let module = self.scope.module;
                if let Some(Import::Std(p)) = self.modules[module].imports.get(name.as_str()).cloned() {
                    let path = format!("{}::{}", p, method);
                    if self.builtins.has_function(&path) {
                        let args = self.eval_args(arguments, None, false)?;
                        self.mark(loc);
                        return self.call_builtin_fn(&path, args);
                    }
                }
            }
        }
        // `Type.method` on a type name used like a path (e.g. `Self.new()`) is not valid; ignore.
        let place = if Self::is_place_expr(object) { self.resolve_place(object)? } else { None };
        let recv = match &place {
            Some(p) => p.read()?,
            None => self.eval_raw(object)?,
        };
        let place = match (&place, &recv) {
            (None, Value::Ref(p)) => Some(self.chase(p.clone())?),
            _ => place,
        };
        if method == "clone" && matches!(recv, Value::Rc(_)) {
            return Ok(recv); // Rc::clone: another handle to the same allocation
        }
        let recv = deref_value(recv)?;
        // which user method (if any) will run? decides argument ref-ness
        let user = self.find_user_method(&recv, method);
        let args = self.eval_args(arguments, user.as_ref(), true)?;
        self.mark(loc);
        self.type_hint = type_args.and_then(|t| t.first().cloned());
        if let Some(f) = user {
            return self.call_user_method(&f, recv, place, args);
        }
        self.call_builtin_method(recv, place, method, args)
    }

    pub(crate) fn find_user_method(&self, recv: &Value, method: &str) -> Option<Rc<FuncDef>> {
        let tn = match recv {
            Value::Struct(s) if &*s.name != "__Entry" => s.name.to_string(),
            Value::Enum(e) if !matches!(&*e.ty, "Option" | "Result" | "Ordering") => e.ty.to_string(),
            _ => return None,
        };
        self.methods.get(&(tn, method.to_string())).cloned()
    }

    fn call_user_method(&mut self, f: &Rc<FuncDef>, recv: Value, place: Option<Place>, args: Vec<Value>) -> R {
        let has_self = f.params.first().map(|p| p.name == "self").unwrap_or(false);
        if !has_self {
            // `fn render() -> T` written without `self` but called on an instance: the body may
            // still mention `self` (implicit receiver), so bind it
            return self.call_func(f, args, Some(recv));
        }
        let self_val = if place.is_some() && self.self_mutation(f) {
            // release our clone of the receiver so in-place mutation stays copy-free
            drop(recv);
            Value::Ref(place.unwrap())
        } else {
            recv
        };
        self.call_func(f, args, Some(self_val))
    }

    /// Call a method on an already-evaluated receiver (no place available).
    pub(crate) fn call_method_value(&mut self, recv: Value, method: &str, args: Vec<Value>, place: Option<Place>) -> R {
        let recv = deref_value(recv)?;
        if let Some(f) = self.find_user_method(&recv, method) {
            return self.call_user_method(&f, recv, place, args);
        }
        self.call_builtin_method(recv, place, method, args)
    }

    pub(crate) fn call_builtin_method(
        &mut self,
        recv: Value,
        place: Option<Place>,
        method: &str,
        args: Vec<Value>,
    ) -> R {
        let kind = Kind::of(&recv);
        let imp = self.builtins.method(kind, method);
        match imp {
            Some(MethodImpl::Pure(f)) => f(self, &recv, args),
            Some(MethodImpl::Mut(f)) => {
                // the receiver was read (cloned) from `place`; release that handle so the take
                // below leaves the value uniquely owned and mutation stays O(1)
                let place = match place {
                    Some(p) => {
                        drop(recv);
                        p
                    }
                    None => Place::temp(recv),
                };
                let mut v = place.take()?;
                let r = f(self, &mut v, args);
                place.write(v)?;
                r
            }
            Some(MethodImpl::Place(f)) => {
                let place = place.unwrap_or_else(|| Place::temp(recv.clone()));
                f(self, place, args)
            }
            None => {
                // user trait impl on a builtin type (e.g. `impl Describe for i32`)
                let tn = recv.type_name();
                let mut alt = vec![tn.clone(), kind.type_alias().to_string()];
                if matches!(recv, Value::Int(_, super::int::IntTy::Lit)) {
                    alt.extend(["i32", "u32", "u64", "usize", "u8", "i16", "u16", "i8", "isize", "i128", "u128"].map(String::from));
                }
                for t in alt {
                    if let Some(f) = self.methods.get(&(t, method.to_string())).cloned() {
                        return self.call_user_method(&f, recv, place, args);
                    }
                }
                Err(semantic(format!("no method named `{}` found for value of type `{}`", method, recv.type_name())))
            }
        }
    }

    // ------------------------------------------------------------------
    // Mutation analysis (inferred &mut parameters)
    // ------------------------------------------------------------------

    pub(crate) fn param_mutation(&mut self, f: &Rc<FuncDef>) -> Rc<Vec<bool>> {
        if let Some(m) = f.mutated.get() {
            return m.clone();
        }
        let key = Rc::as_ptr(f) as usize;
        if !self.mutating_analysis.insert(key) {
            return Rc::new(vec![false; f.params.len()]); // recursion: assume not mutated
        }
        let (params, self_mut) = analysis::analyze(self, f);
        self.mutating_analysis.remove(&key);
        let rc = Rc::new(params);
        let _ = f.mutated.set(rc.clone());
        let _ = f.self_mutated.set(self_mut);
        rc
    }

    pub(crate) fn self_mutation(&mut self, f: &Rc<FuncDef>) -> bool {
        if let Some(b) = f.self_mutated.get() {
            return *b;
        }
        self.param_mutation(f);
        f.self_mutated.get().copied().unwrap_or(false)
    }
}

