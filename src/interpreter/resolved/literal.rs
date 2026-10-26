//! Struct literals of `--engine=resolved`.
//!
//! The walker's `eval_struct_literal_in` resolves the type name on every evaluation (aliases, `Self`,
//! enum variants, host types) and looks each declared field up in the literal by name. A literal
//! naming a declared struct with exactly its fields is resolved once per site instead: the plan
//! keeps the interned names, the declared types and the literal's field order, and evaluation
//! does what the walker does with them (fields in declaration order, each coerced to its declared
//! type; an integer already of the declared width is unchanged by `coerce`). Every other literal,
//! and every literal under `--hot` (definitions may change), runs the walker's routine.

use super::compile::{Cx, E};
use super::expr::expr_eval;
use super::Code;
use crate::interpreter::engine::Interpreter;
use crate::interpreter::error::{semantic, R};
use crate::interpreter::int::IntTy;
use crate::interpreter::place::PlaceEnv;
use crate::interpreter::value::{Slot, Value};
use crate::parser::{Expression, Type};
use crate::source_map::Location;
use std::cell::OnceCell;
use std::rc::Rc;

struct Plan {
    name: Rc<str>,
    names: Rc<[Rc<str>]>,
    types: Rc<[(&'static str, Type)]>,
    /// The declared integer width of each field, if it is one (`coerce` keeps such a value as is).
    ints: Vec<Option<IntTy>>,
    copy: bool,
    /// For each declared field, the literal's field that gives it.
    order: Vec<usize>,
}

fn plan(it: &Interpreter, name: &str, fields: &[(String, E)]) -> Option<Plan> {
    if name.contains("::") || matches!(name, "Self" | "RangeFrom") || it.type_aliases.contains_key(name) {
        return None;
    }
    let d = it.structs.get(name)?;
    if d.fields.len() != fields.len() {
        return None; // missing, duplicate or `..base` fields: the walker's
    }
    let order = d.fields.iter().map(|(f, _)| fields.iter().position(|(n, _)| n == f)).collect::<Option<Vec<_>>>()?;
    let ints = d.fields.iter().map(|(_, t)| IntTy::from_type(t).filter(|w| *w != IntTy::Lit && !matches!(t, Type::Reference(_) | Type::MutableReference(_)))).collect();
    Some(Plan { name: d.rc_name.clone(), names: d.rc_fields.clone(), types: d.fields.clone(), ints, copy: d.copy, order })
}

/// The literal's compiled field expressions, as the walker's routine asks for them (by address).
struct Fields<'a> {
    codes: &'a [Code],
    fields: &'static [(String, E)],
    b: usize,
}

impl PlaceEnv for Fields<'_> {
    fn var(&mut self, _: &Interpreter, _: &str) -> Option<Slot> {
        None
    }
    fn eval(&mut self, it: &mut Interpreter, e: &'static Expression<'static>) -> R<Value> {
        match self.fields.iter().position(|(_, x)| std::ptr::eq(*x, e)) {
            Some(i) => (self.codes[i])(it, self.b),
            None => Err(semantic("internal: struct literal field not compiled")),
        }
    }
    fn eval_keep_ref(&mut self, it: &mut Interpreter, e: &'static Expression<'static>) -> R<Value> {
        self.eval(it, e)
    }
}

pub(super) fn struct_literal(cx: &mut Cx, name: &'static str, fields: &'static [(String, E)], loc: Option<&'static Location>) -> Option<Code> {
    let codes: Vec<Code> = fields.iter().map(|(_, fe)| expr_eval(cx, fe).map(|x| x.code())).collect::<Option<_>>()?;
    let plan_cell: OnceCell<Option<Plan>> = OnceCell::new();
    Some(Box::new(move |it, b| {
        it.mark(loc);
        if it.hot.is_none() {
            if let Some(p) = plan_cell.get_or_init(|| plan(it, name, fields)) {
                let mut out = Vec::with_capacity(p.order.len());
                for (i, src) in p.order.iter().enumerate() {
                    let v = (codes[*src])(it, b)?;
                    let v = match (&v, p.ints[i]) {
                        (Value::Int(_, t), Some(w)) if *t == w => v,
                        _ => it.coerce(v, &p.types[i].1)?,
                    };
                    out.push((p.names[i].clone(), v));
                }
                return Ok(it.make_struct_named(p.name.clone(), out, p.copy));
            }
        }
        it.eval_struct_literal_in(name, fields, &mut Fields { codes: &codes, fields, b })
    }))
}
