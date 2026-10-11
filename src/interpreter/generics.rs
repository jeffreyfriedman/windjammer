//! Static dispatch through type parameters (`T::new()`, `T::default()`, `T::MAX`).
//!
//! Generics are otherwise erased. For the body of a generic function the interpreter keeps
//! a table `T -> concrete type`, bound at call time from (in priority order) an explicit
//! turbofish, the runtime types of the arguments, and the expected type of the call
//! (`let s: Sq = make()`, a function's declared return type). A parameter that none of
//! these determines stays unbound, and using it is a located `unsupported` error.

use super::engine::Interpreter;
use super::int::IntTy;
use super::value::*;
use crate::parser::Type;
use std::rc::Rc;

pub(crate) type Bindings = Vec<(Rc<str>, Option<Type>)>;

/// The static type a runtime value stands for, when it can be read off the value.
pub(crate) fn value_type(v: &Value) -> Option<Type> {
    Some(match v {
        Value::Int(_, IntTy::Lit) => Type::Int32,
        Value::Int(_, t) => Type::Custom(t.name().to_string()),
        Value::Float(_, FloatTy::F32) => Type::Custom("f32".into()),
        Value::Float(..) => Type::Custom("f64".into()),
        Value::Bool(_) => Type::Bool,
        Value::Char(_) => Type::Custom("char".into()),
        Value::Str(_) => Type::String,
        Value::Struct(s) => Type::Custom(s.name.to_string()),
        Value::Enum(e) => match &*e.ty {
            "Option" | "Result" | "Ordering" => return None,
            n => Type::Custom(n.to_string()),
        },
        Value::Vec(x) => Type::Vec(Box::new(value_type(x.first()?)?)),
        Value::Ref(p) => return value_type(&p.read().ok()?),
        _ => return None,
    })
}

/// `(base name, type arguments)` of a type, normalising the parser's several spellings.
fn shape(t: &Type) -> Option<(String, Vec<&Type>)> {
    Some(match t {
        Type::Reference(i) | Type::MutableReference(i) => return shape(i),
        Type::Vec(e) | Type::Array(e, _) => ("Vec".into(), vec![&**e]),
        Type::Option(e) => ("Option".into(), vec![&**e]),
        Type::Result(a, b) => ("Result".into(), vec![&**a, &**b]),
        Type::Tuple(ts) => ("()".into(), ts.iter().collect()),
        Type::Parameterized(n, a) => (n.rsplit("::").next().unwrap_or(n).to_string(), a.iter().collect()),
        Type::Custom(n) | Type::Generic(n) => (n.clone(), vec![]),
        Type::String => ("String".into(), vec![]),
        _ => return None,
    })
}

fn strip_ref(t: &Type) -> &Type {
    match t {
        Type::Reference(i) | Type::MutableReference(i) => strip_ref(i),
        other => other,
    }
}

/// Bind the parameters named in `params` by matching `pat` against `actual`.
pub(crate) fn unify(pat: &Type, actual: &Type, params: &[String], out: &mut Bindings) {
    let pat = strip_ref(pat);
    let actual = strip_ref(actual);
    if let Some((n, args)) = shape(pat) {
        if args.is_empty() && params.iter().any(|p| *p == n) {
            if let Some(slot) = out.iter_mut().find(|(k, _)| **k == *n) {
                if slot.1.is_none() {
                    slot.1 = Some(actual.clone());
                }
            }
            return;
        }
        if let Some((an, aargs)) = shape(actual) {
            if an == n && args.len() == aargs.len() {
                for (p, a) in args.into_iter().zip(aargs) {
                    unify(p, a, params, out);
                }
            }
        }
    }
}

/// Replace bound parameters inside `t`.
pub(crate) fn subst(t: &Type, b: &Bindings) -> Type {
    let lookup = |n: &str| b.iter().find(|(k, _)| &**k == n).and_then(|(_, v)| v.clone());
    match t {
        Type::Generic(n) | Type::Custom(n) => lookup(n).unwrap_or_else(|| t.clone()),
        Type::Vec(e) => Type::Vec(Box::new(subst(e, b))),
        Type::Option(e) => Type::Option(Box::new(subst(e, b))),
        Type::Result(a, c) => Type::Result(Box::new(subst(a, b)), Box::new(subst(c, b))),
        Type::Reference(i) => Type::Reference(Box::new(subst(i, b))),
        Type::MutableReference(i) => Type::MutableReference(Box::new(subst(i, b))),
        Type::Tuple(ts) => Type::Tuple(ts.iter().map(|x| subst(x, b)).collect()),
        Type::Parameterized(n, a) => Type::Parameterized(n.clone(), a.iter().map(|x| subst(x, b)).collect()),
        other => other.clone(),
    }
}

impl Interpreter {
    /// Bindings for a call: explicit type arguments, then the expected result type.
    pub(crate) fn initial_bindings(&self, params: &[String], targs: &[Type], expected: Option<(&Type, &Type)>) -> Bindings {
        let mut out: Bindings = params.iter().map(|p| (Rc::from(p.as_str()), None)).collect();
        // an explicit turbofish names the function's own parameters, which come last
        let own = params.len().saturating_sub(targs.len());
        for (i, t) in targs.iter().enumerate() {
            if let Some(slot) = out.get_mut(own + i) {
                slot.1 = Some(t.clone());
            }
        }
        if let Some((ret, want)) = expected {
            unify(ret, want, params, &mut out);
        }
        out
    }

    /// Infer still-unbound parameters from an argument value.
    pub(crate) fn bind_from_arg(&mut self, declared: &Type, v: &Value) {
        if self.scope.tparams.iter().all(|(_, b)| b.is_some()) {
            return;
        }
        let Some(actual) = value_type(v) else { return };
        let names: Vec<String> = self.scope.tparams.iter().map(|(n, _)| n.to_string()).collect();
        unify(declared, &actual, &names, &mut self.scope.tparams);
    }

    /// If `first` names a type parameter of the running function, its concrete type
    /// (`Err` when the parameter exists but was not inferable).
    pub(crate) fn tparam(&self, first: &str) -> Option<Result<&Type, ()>> {
        self.scope.tparams.iter().find(|(n, _)| &**n == first).map(|(_, t)| t.as_ref().ok_or(()))
    }

    /// Remember the expected type of the next evaluation of call expression `e`.
    pub(crate) fn expect_type_of(&mut self, e: &'static crate::parser::Expression<'static>, t: &Type) {
        use crate::parser::Expression::{Call, MethodCall};
        if matches!(e, Call { .. } | MethodCall { .. }) {
            let t = subst(t, &self.scope.tparams);
            self.expected = Some((e as *const _ as usize, t));
        }
    }

    pub(crate) fn take_expected(&mut self, e: &'static crate::parser::Expression<'static>) -> Option<Type> {
        match &self.expected {
            Some((p, _)) if *p == e as *const _ as usize => self.expected.take().map(|(_, t)| t),
            _ => None,
        }
    }
}

/// Name used to look up associated items of a concrete type (`Sq::new`, `i32::default`).
pub(crate) fn concrete_base(t: &Type) -> String {
    match t {
        Type::Int => "i64".into(),
        Type::Int32 => "i32".into(),
        Type::Uint => "u64".into(),
        Type::Float => "f64".into(),
        Type::Bool => "bool".into(),
        Type::String => "String".into(),
        Type::Vec(_) => "Vec".into(),
        Type::Option(_) => "Option".into(),
        other => match shape(other) {
            Some((n, _)) => super::register::base_type_name(&n),
            None => "?".into(),
        },
    }
}

impl Interpreter {
    /// A function whose last statement is a call gets the (substituted) declared return type as its expectation.
    pub(crate) fn expect_tail_type(&mut self, f: &Rc<super::engine::FuncDef>) {
        if let (Some(ret), Some(crate::parser::Statement::Expression { expr, .. })) = (f.ret, f.body.last()) {
            self.expect_type_of(expr, ret);
        }
    }
}
