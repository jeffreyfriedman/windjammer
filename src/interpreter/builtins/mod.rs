//! Builtin (native) standard library: an extensible registry of methods, functions and constants.
//!
//! Adding a stdlib method = one `reg.pure/mutating/place(Kind, "name", fn)` call in the
//! module for that type. Nothing else in the interpreter needs to change.

use super::engine::Interpreter;
use super::error::{panic_ctl, semantic, unsupported, R};
use super::int::IntTy;
use super::value::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

mod iter;
mod map;
mod num;
mod option;
mod seq;
mod shared;
mod std_fns;
mod text;

pub use seq::collect_into;
pub use std_fns::assoc_numeric;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Any,
    Unit,
    Bool,
    Int,
    Float,
    Char,
    Str,
    Vec,
    Tuple,
    Map,
    Set,
    Range,
    Iter,
    Option,
    Result,
    Ordering,
    Entry,
    Struct,
    Enum,
    Func,
    Formatter,
    Rc,
    Cell,
}

impl Kind {
    pub fn of(v: &Value) -> Kind {
        match v {
            Value::Unit => Kind::Unit,
            Value::Bool(_) => Kind::Bool,
            Value::Int(..) => Kind::Int,
            Value::Float(..) => Kind::Float,
            Value::Char(_) => Kind::Char,
            Value::Str(_) => Kind::Str,
            Value::Vec(_) => Kind::Vec,
            Value::Tuple(_) => Kind::Tuple,
            Value::Map(_) => Kind::Map,
            Value::Set(_) => Kind::Set,
            Value::Range(..) => Kind::Range,
            Value::Iter(_) => Kind::Iter,
            Value::Enum(e) => match &*e.ty {
                "Option" => Kind::Option,
                "Result" => Kind::Result,
                "Ordering" => Kind::Ordering,
                _ => Kind::Enum,
            },
            Value::Struct(s) if &*s.name == "__Entry" => Kind::Entry,
            Value::Struct(_) => Kind::Struct,
            Value::Func(_) => Kind::Func,
            Value::Formatter(_) => Kind::Formatter,
            Value::Ref(_) => Kind::Any,
            Value::Rc(_) | Value::Weak(_) => Kind::Rc,
            Value::Cell(..) => Kind::Cell,
        }
    }

    /// Name under which `impl Trait for <builtin type>` is registered.
    pub fn type_alias(self) -> &'static str {
        match self {
            Kind::Str => "String",
            Kind::Vec => "Vec",
            Kind::Map => "HashMap",
            Kind::Set => "HashSet",
            Kind::Float => "f64",
            Kind::Bool => "bool",
            Kind::Char => "char",
            Kind::Int => "i64",
            Kind::Option => "Option",
            Kind::Result => "Result",
            _ => "",
        }
    }
}

pub type PureFn = fn(&mut Interpreter, &Value, Vec<Value>) -> R;
pub type MutFn = fn(&mut Interpreter, &mut Value, Vec<Value>) -> R;
pub type PlaceFn = fn(&mut Interpreter, Place, Vec<Value>) -> R;
pub type FnImpl = fn(&mut Interpreter, Vec<Value>) -> R;

#[derive(Clone, Copy)]
pub enum MethodImpl {
    /// Reads the receiver.
    Pure(PureFn),
    /// Mutates the receiver in place (receiver taken out of its place while running).
    Mut(MutFn),
    /// Needs the receiver's address (returns references into it).
    Place(PlaceFn),
}

#[derive(Default)]
pub struct Registry {
    methods: HashMap<(Kind, &'static str), MethodImpl>,
    funcs: HashMap<&'static str, FnImpl>,
    consts: HashMap<String, Value>,
    types: HashSet<&'static str>,
    mutating: HashSet<&'static str>,
}

impl Registry {
    pub fn standard() -> Registry {
        let mut r = Registry::default();
        num::register(&mut r);
        text::register(&mut r);
        seq::register(&mut r);
        map::register(&mut r);
        option::register(&mut r);
        std_fns::register(&mut r);
        shared::register(&mut r);
        r
    }

    pub fn pure(&mut self, k: Kind, name: &'static str, f: PureFn) {
        self.methods.insert((k, name), MethodImpl::Pure(f));
    }
    pub fn mutating(&mut self, k: Kind, name: &'static str, f: MutFn) {
        self.methods.insert((k, name), MethodImpl::Mut(f));
        self.mutating.insert(name);
    }
    pub fn place(&mut self, k: Kind, name: &'static str, f: PlaceFn) {
        self.methods.insert((k, name), MethodImpl::Place(f));
        self.mutating.insert(name);
    }
    pub fn func(&mut self, path: &'static str, f: FnImpl) {
        self.funcs.insert(path, f);
    }
    pub fn add_const(&mut self, path: impl Into<String>, v: Value) {
        self.consts.insert(path.into(), v);
    }
    pub fn type_name(&mut self, name: &'static str) {
        self.types.insert(name);
    }

    pub fn is_mutating_name(&self, name: &str) -> bool {
        self.mutating.contains(name)
    }
    pub fn has_type(&self, name: &str) -> bool {
        self.types.contains(name)
    }

    pub fn method(&self, k: Kind, name: &str) -> Option<MethodImpl> {
        if let Some(m) = self.methods.get(&(k, name)) {
            return Some(*m);
        }
        if matches!(k, Kind::Range | Kind::Vec | Kind::Map | Kind::Set) {
            if let Some(m) = self.methods.get(&(Kind::Iter, name)) {
                return Some(*m);
            }
        }
        self.methods.get(&(Kind::Any, name)).copied()
    }

    pub fn has_method(&self, k: Kind, name: &str) -> bool {
        self.method(k, name).is_some()
    }

    /// Look a function up by path, dropping leading segments (`std::collections::HashMap::new`
    /// matches `HashMap::new`).
    pub fn function(&self, path: &str) -> Option<FnImpl> {
        let mut p = path;
        loop {
            if let Some(f) = self.funcs.get(p) {
                return Some(*f);
            }
            match p.split_once("::") {
                Some((_, rest)) => p = rest,
                None => return None,
            }
        }
    }
    pub fn has_function(&self, path: &str) -> bool {
        self.function(path).is_some()
    }

    pub fn constant(&self, path: &str) -> Option<Value> {
        let mut p = path;
        loop {
            if let Some(v) = self.consts.get(p) {
                return Some(v.clone());
            }
            match p.split_once("::") {
                Some((_, rest)) => p = rest,
                None => return None,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Argument helpers
// ---------------------------------------------------------------------------

pub(crate) fn arg(args: &[Value], i: usize, what: &str) -> R<Value> {
    let v = args.get(i).cloned().ok_or_else(|| semantic(format!("`{}` is missing argument {}", what, i + 1)))?;
    super::place::deref_value(v)
}

pub(crate) fn arg_int(args: &[Value], i: usize, what: &str) -> R<i128> {
    match arg(args, i, what)? {
        Value::Int(n, _) => Ok(n),
        other => Err(semantic(format!("`{}` expects an integer, found {}", what, other.type_name()))),
    }
}

pub(crate) fn arg_usize(args: &[Value], i: usize, what: &str) -> R<usize> {
    let n = arg_int(args, i, what)?;
    usize::try_from(n).map_err(|_| panic_ctl(format!("`{}`: value {} is out of range for usize", what, n)))
}

pub(crate) fn arg_str(args: &[Value], i: usize, what: &str) -> R<Rc<String>> {
    match arg(args, i, what)? {
        Value::Str(s) => Ok(s),
        Value::Char(c) => Ok(Rc::new(c.to_string())),
        other => Err(semantic(format!("`{}` expects a string, found {}", what, other.type_name()))),
    }
}

pub(crate) fn arg_f64(args: &[Value], i: usize, what: &str) -> R<f64> {
    match arg(args, i, what)? {
        Value::Float(f, _) => Ok(f),
        Value::Int(n, IntTy::Lit) => Ok(n as f64),
        other => Err(semantic(format!("`{}` expects a float, found {}", what, other.type_name()))),
    }
}

pub(crate) fn new_iter(items: Vec<Value>) -> Value {
    Value::Iter(IterState::from_items(items))
}

pub(crate) fn ordering(o: std::cmp::Ordering) -> Value {
    match o {
        std::cmp::Ordering::Less => Value::enum_val("Ordering", "Less", 0, Payload::Unit),
        std::cmp::Ordering::Equal => Value::enum_val("Ordering", "Equal", 1, Payload::Unit),
        std::cmp::Ordering::Greater => Value::enum_val("Ordering", "Greater", 2, Payload::Unit),
    }
}

pub(crate) fn ordering_of(v: &Value) -> Option<std::cmp::Ordering> {
    match v {
        Value::Enum(e) if &*e.ty == "Ordering" => Some(match e.idx {
            0 => std::cmp::Ordering::Less,
            1 => std::cmp::Ordering::Equal,
            _ => std::cmp::Ordering::Greater,
        }),
        _ => None,
    }
}

pub(crate) fn no_arg_unsupported(what: &str) -> super::error::Ctl {
    unsupported(format!("`{}` is not implemented by the interpreter", what))
}
