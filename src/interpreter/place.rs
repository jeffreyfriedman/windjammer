//! Places: addressable locations inside variables (targets of `&mut`, assignment, mutating methods).

use super::engine::Interpreter;
use super::error::{panic_ctl, unsupported, R};
use super::value::*;
use crate::parser::Expression;
use std::rc::Rc;

fn descend<'v>(cur: &'v mut Value, p: &Proj) -> R<&'v mut Value> {
    match (cur, p) {
        (Value::Struct(s), Proj::Field(name)) => {
            let sv = Rc::make_mut(s);
            sv.fields
                .iter_mut()
                .find(|(n, _)| **n == **name)
                .map(|(_, v)| v)
                .ok_or_else(|| panic_ctl(format!("no field `{}`", name)))
        }
        (Value::Tuple(t), Proj::Index(i)) => {
            let tv = Rc::make_mut(t);
            tv.get_mut(*i).ok_or_else(|| panic_ctl("tuple index out of range"))
        }
        (Value::Tuple(t), Proj::Field(name)) => {
            let i: usize = name.parse().map_err(|_| panic_ctl("bad tuple field"))?;
            let tv = Rc::make_mut(t);
            tv.get_mut(i).ok_or_else(|| panic_ctl("tuple index out of range"))
        }
        (Value::Vec(v), Proj::Index(i)) => {
            let len = v.len();
            let vv = Rc::make_mut(v);
            vv.get_mut(*i).ok_or_else(|| {
                panic_ctl(format!("index out of bounds: the len is {} but the index is {}", len, i))
            })
        }
        (Value::Map(m), Proj::Key(k)) => {
            let mm = Rc::make_mut(m);
            mm.get_mut(&Key(k.clone())).ok_or_else(|| panic_ctl("key not found in map"))
        }
        (Value::Enum(e), Proj::Index(i)) => {
            let ev = Rc::make_mut(e);
            match &mut ev.payload {
                Payload::Tuple(vs) => vs.get_mut(*i).ok_or_else(|| panic_ctl("variant field out of range")),
                _ => Err(panic_ctl("variant has no tuple payload")),
            }
        }
        (Value::Enum(e), Proj::Field(name)) => {
            let ev = Rc::make_mut(e);
            match &mut ev.payload {
                Payload::Struct(fs) => fs
                    .iter_mut()
                    .find(|(n, _)| **n == **name)
                    .map(|(_, v)| v)
                    .ok_or_else(|| panic_ctl("no such variant field")),
                Payload::Tuple(vs) => {
                    let i: usize = name.parse().map_err(|_| panic_ctl("bad field"))?;
                    vs.get_mut(i).ok_or_else(|| panic_ctl("variant field out of range"))
                }
                _ => Err(panic_ctl("variant has no fields")),
            }
        }
        (Value::Ref(_), _) => Err(unsupported("projection through a stored reference")),
        (v, p) => Err(panic_ctl(format!("cannot project {:?} out of {}", p, v.type_name()))),
    }
}

impl Place {
    pub fn of_slot(slot: Slot) -> Place {
        Place { slot, path: Vec::new() }
    }

    pub fn temp(v: Value) -> Place {
        Place::of_slot(Rc::new(std::cell::RefCell::new(v)))
    }

    pub fn proj(&self, p: Proj) -> Place {
        let mut path = self.path.clone();
        path.push(p);
        Place { slot: self.slot.clone(), path }
    }

    /// Run `f` on the target value with unique access (copy-on-write along the path).
    pub fn with_mut<T>(&self, f: impl FnOnce(&mut Value) -> R<T>) -> R<T> {
        let mut guard = self.slot.borrow_mut();
        let mut cur: &mut Value = &mut guard;
        for p in &self.path {
            cur = descend(cur, p)?;
        }
        f(cur)
    }

    /// Read (clone) the target value. Cheap: aggregates are `Rc`.
    pub fn read(&self) -> R<Value> {
        let guard = self.slot.borrow();
        let mut cur: Value = guard.clone();
        drop(guard);
        for p in &self.path {
            cur = project(&cur, p)?;
        }
        Ok(cur)
    }

    pub fn write(&self, v: Value) -> R<()> {
        self.with_mut(|t| {
            *t = v;
            Ok(())
        })
    }

    pub fn take(&self) -> R<Value> {
        self.with_mut(|t| Ok(std::mem::replace(t, Value::Unit)))
    }
}

/// Immutable projection of a field/index out of a value.
pub fn project(v: &Value, p: &Proj) -> R<Value> {
    match (v, p) {
        (Value::Struct(s), Proj::Field(name)) => s
            .fields
            .iter()
            .find(|(n, _)| **n == **name)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| panic_ctl(format!("no field `{}` on {}", name, s.name))),
        (Value::Tuple(t), Proj::Index(i)) => t.get(*i).cloned().ok_or_else(|| panic_ctl("tuple index out of range")),
        (Value::Tuple(t), Proj::Field(n)) => {
            let i: usize = n.parse().map_err(|_| panic_ctl("bad tuple field"))?;
            t.get(i).cloned().ok_or_else(|| panic_ctl("tuple index out of range"))
        }
        (Value::Vec(vs), Proj::Index(i)) => vs.get(*i).cloned().ok_or_else(|| {
            panic_ctl(format!("index out of bounds: the len is {} but the index is {}", vs.len(), i))
        }),
        (Value::Map(m), Proj::Key(k)) => m.get(&Key(k.clone())).cloned().ok_or_else(|| panic_ctl("key not found in map")),
        (Value::Enum(e), Proj::Index(i)) => match &e.payload {
            Payload::Tuple(vs) => vs.get(*i).cloned().ok_or_else(|| panic_ctl("variant field out of range")),
            _ => Err(panic_ctl("variant has no tuple payload")),
        },
        (Value::Enum(e), Proj::Field(name)) => match &e.payload {
            Payload::Struct(fs) => fs
                .iter()
                .find(|(n, _)| **n == **name)
                .map(|(_, v)| v.clone())
                .ok_or_else(|| panic_ctl("no such variant field")),
            Payload::Tuple(vs) => {
                let i: usize = name.parse().map_err(|_| panic_ctl("bad field"))?;
                vs.get(i).cloned().ok_or_else(|| panic_ctl("variant field out of range"))
            }
            _ => Err(panic_ctl("variant has no fields")),
        },
        (Value::Ref(p0), _) => project(&p0.read()?, p),
        (v, p) => Err(panic_ctl(format!("cannot project {:?} out of {}", p, v.type_name()))),
    }
}

/// Follow `Ref` values to the underlying value.
pub fn deref_value(v: Value) -> R<Value> {
    let mut cur = v;
    let mut n = 0;
    while let Value::Ref(p) = cur {
        cur = p.read()?;
        n += 1;
        if n > 64 {
            return Err(panic_ctl("reference cycle"));
        }
    }
    Ok(cur)
}

impl Interpreter {
    /// Is this expression an addressable place?
    pub(crate) fn is_place_expr(e: &Expression<'static>) -> bool {
        match e {
            Expression::Identifier { name, .. } => !name.contains("::"),
            Expression::FieldAccess { object, .. } => Self::is_place_expr(object) || matches!(object, Expression::MethodCall { .. } | Expression::Call { .. }),
            Expression::Index { object, .. } => Self::is_place_expr(object),
            Expression::Unary { op: crate::parser::UnaryOp::Deref, operand, .. } => Self::is_place_expr(operand) || matches!(operand, Expression::MethodCall { .. }),
            Expression::MethodCall { .. } => false,
            _ => false,
        }
    }

    /// Resolve an expression to a place, following references. `None` for rvalues.
    pub(crate) fn resolve_place(&mut self, e: &'static Expression<'static>) -> R<Option<Place>> {
        match e {
            Expression::Identifier { name, .. } => {
                let Some(slot) = self.lookup_var(name) else {
                    // `static mut` / module-level state
                    let module = self.scope.module;
                    if self.modules[module].consts.contains_key(name.as_str()) {
                        self.global_value(module, name)?;
                        return Ok(self.globals.get(&(module, name.clone())).cloned().map(Place::of_slot));
                    }
                    return Ok(None);
                };
                let v = slot.borrow().clone();
                match v {
                    Value::Ref(p) => Ok(Some(self.chase(p)?)),
                    _ => Ok(Some(Place::of_slot(slot))),
                }
            }
            Expression::FieldAccess { object, field, .. } => {
                let base = match self.resolve_place(object)? {
                    Some(p) => p,
                    None => {
                        let v = self.eval(object)?;
                        Place::temp(v)
                    }
                };
                let bv = base.read()?;
                let bv = match bv {
                    Value::Ref(p) => return Ok(Some(self.chase(p)?.proj(Proj::Field(field.as_str().into())))),
                    other => other,
                };
                let proj = match &bv {
                    Value::Tuple(_) => Proj::Index(field.parse().map_err(|_| panic_ctl("bad tuple field"))?),
                    _ => Proj::Field(field.as_str().into()),
                };
                Ok(Some(base.proj(proj)))
            }
            Expression::Index { object, index, .. } => {
                let base = match self.resolve_place(object)? {
                    Some(p) => p,
                    None => {
                        let v = self.eval(object)?;
                        Place::temp(v)
                    }
                };
                let idx = self.eval(index)?;
                let bv = base.read()?;
                match (&bv, &idx) {
                    (Value::Vec(vs), Value::Int(i, _)) => {
                        if *i < 0 || (*i as usize) >= vs.len() {
                            return Err(self.panic_at(
                                e,
                                format!("index out of bounds: the len is {} but the index is {}", vs.len(), i),
                            ));
                        }
                        Ok(Some(base.proj(Proj::Index(*i as usize))))
                    }
                    (Value::Map(_), k) => Ok(Some(base.proj(Proj::Key(k.clone())))),
                    _ => Err(panic_ctl(format!("cannot index {} with {}", bv.type_name(), idx.type_name()))),
                }
            }
            Expression::Unary { op: crate::parser::UnaryOp::Deref, operand, .. } => {
                // `*r`: r's slot holds a Ref -> its target; otherwise same place
                self.resolve_place(operand)
            }
            Expression::Unary { op: crate::parser::UnaryOp::MutRef | crate::parser::UnaryOp::Ref, operand, .. } => {
                self.resolve_place(operand)
            }
            Expression::MethodCall { .. } | Expression::Call { .. } => {
                let v = self.eval_keep_ref(e)?;
                match v {
                    Value::Ref(p) => Ok(Some(self.chase(p)?)),
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    /// Follow a place whose slot itself holds a `Ref` (reference-to-reference).
    pub(crate) fn chase(&self, p: Place) -> R<Place> {
        let mut cur = p;
        for _ in 0..64 {
            if !cur.path.is_empty() {
                // Only the root slot can hold a Ref we jump through.
                return Ok(cur);
            }
            let v = cur.slot.borrow().clone();
            match v {
                Value::Ref(n) => cur = n,
                _ => return Ok(cur),
            }
        }
        Err(panic_ctl("reference cycle"))
    }
}
