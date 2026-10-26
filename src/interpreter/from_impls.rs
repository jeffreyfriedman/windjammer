//! `impl From<S> for T`: several impls of one target coexist and are chosen by the argument's type,
//! for `T::from(x)`, `x.into()` and the conversion `?` applies to an error.

use super::engine::{FuncDef, Interpreter};
use super::int::IntTy;
use super::place::deref_value;
use super::value::*;
use crate::parser::Type;
use std::rc::Rc;

/// 0 = the value cannot be a `t`, 1 = it might be (unsuffixed literal, unknown shape), 2 = it is.
fn fit(v: &Value, t: &Type) -> u8 {
    if let Type::Reference(i) | Type::MutableReference(i) = t {
        return fit(v, i);
    }
    let named = |n: &str| matches!(t, Type::Custom(c) if c == n);
    match v {
        Value::Int(_, w) => match IntTy::from_type(t) {
            Some(ty) if *w == IntTy::Lit => 1 + u8::from(ty == IntTy::I64 && matches!(t, Type::Int)),
            Some(ty) => 2 * u8::from(ty == *w),
            None => u8::from(matches!(t, Type::Generic(_) | Type::Infer)),
        },
        Value::Float(_, ft) => match type_is_float(t) {
            Some(tt) => 2 * u8::from(tt == *ft),
            None => u8::from(matches!(t, Type::Generic(_) | Type::Infer)),
        },
        Value::Bool(_) => 2 * u8::from(matches!(t, Type::Bool) || named("bool")),
        Value::Char(_) => 2 * u8::from(named("char")),
        Value::Str(_) => 2 * u8::from(matches!(t, Type::String) || named("String") || named("str") || named("string")),
        Value::Vec(_) => 2 * u8::from(matches!(t, Type::Vec(_) | Type::Array(..)) || matches!(t, Type::Parameterized(n, _) if n == "Vec")),
        Value::Tuple(_) => 2 * u8::from(matches!(t, Type::Tuple(_))),
        Value::Struct(_) | Value::Enum(_) => match t {
            Type::Custom(n) => 2 * u8::from(*n == v.type_name()),
            Type::Option(_) => 2 * u8::from(v.type_name() == "Option"),
            Type::Result(..) => 2 * u8::from(v.type_name() == "Result"),
            Type::Parameterized(n, _) => 2 * u8::from(*n == v.type_name()),
            Type::Generic(_) | Type::Infer => 1,
            _ => 0,
        },
        _ => 1,
    }
}

impl Interpreter {
    /// The `From<S>` impl of `target` whose `S` fits `arg` best (`None` when `target` has no `From` impls).
    pub(crate) fn from_impl_for(&self, target: &str, arg: &Value) -> Option<Rc<FuncDef>> {
        let impls = self.from_impls.get(target)?;
        if impls.len() == 1 {
            return Some(impls[0].1.clone());
        }
        let arg = deref_value(arg.clone()).ok()?;
        let mut best: Option<(u8, &Rc<FuncDef>)> = None;
        for (src, f) in impls {
            let score = fit(&arg, src);
            if score > 0 && best.is_none_or(|(b, _)| score > b) {
                best = Some((score, f));
            }
        }
        best.map(|(_, f)| f.clone())
    }

    /// `f` is one `from` of a type with several `From` impls: the impl chosen by `arg`'s type, if different.
    pub(crate) fn pick_from(&self, f: &Rc<FuncDef>, arg: &Value) -> Option<Rc<FuncDef>> {
        let ty = f.self_ty.as_ref()?;
        if self.from_impls.get(&**ty).is_none_or(|v| v.len() < 2) {
            return None;
        }
        self.from_impl_for(ty, arg).filter(|p| !Rc::ptr_eq(p, f))
    }

    /// `v.into()`: the declared type's `From` impl chosen by `v`, else the plain coercion to that type.
    pub(crate) fn convert_into(&mut self, v: &Value) -> super::error::R {
        match self.type_hint.take() {
            Some(Type::Custom(target)) if v.type_name() != target && self.from_impl_for(&target, v).is_some() => {
                let f = self.from_impl_for(&target, v).expect("checked");
                self.call_func(&f, vec![v.clone()], None)
            }
            Some(t) => self.coerce(v.clone(), &t),
            None => Ok(v.clone()),
        }
    }
}
