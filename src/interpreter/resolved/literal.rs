//! Struct literals of `--engine=resolved`.
//!
//! The walker's `eval_struct_literal_in` resolves the type name on every evaluation (aliases, `Self`,
//! enum variants, host types) and looks each declared field up in the literal by name. A literal
//! naming a declared struct with exactly its fields is resolved once per site instead: the plan
//! keeps the interned names, the declared types and the literal's field order, and evaluation
//! does what the walker does with them (fields in declaration order, each coerced to its declared
//! type). Every other literal, and every literal under `--hot` (definitions may change), runs the
//! walker's routine.

use super::compile::{Cx, E};
use super::place::PlaceCode;
use super::Code;
use crate::interpreter::engine::Interpreter;
use crate::parser::Type;
use crate::source_map::Location;
use std::cell::OnceCell;
use std::rc::Rc;

struct Plan {
    name: Rc<str>,
    names: Rc<[Rc<str>]>,
    types: Rc<[(&'static str, Type)]>,
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
    Some(Plan { name: d.rc_name.clone(), names: d.rc_fields.clone(), types: d.fields.clone(), copy: d.copy, order })
}

pub(super) fn struct_literal(cx: &mut Cx, name: &'static str, fields: &'static [(String, E)], loc: Option<&'static Location>) -> Option<Code> {
    let mut codes = PlaceCode::default();
    for (_, fe) in fields.iter() {
        codes.add_eval(cx, fe)?;
    }
    let plan_cell: OnceCell<Option<Plan>> = OnceCell::new();
    Some(Box::new(move |it, b| {
        it.mark(loc);
        let mut env = codes.env(b);
        if it.hot.is_none() {
            if let Some(p) = plan_cell.get_or_init(|| plan(it, name, fields)) {
                let mut out = Vec::with_capacity(p.order.len());
                for (i, src) in p.order.iter().enumerate() {
                    let v = crate::interpreter::place::PlaceEnv::eval(&mut env, it, fields[*src].1)?;
                    out.push((p.names[i].clone(), it.coerce(v, &p.types[i].1)?));
                }
                return Ok(it.make_struct_named(p.name.clone(), out, p.copy));
            }
        }
        it.eval_struct_literal_in(name, fields, &mut env)
    }))
}
