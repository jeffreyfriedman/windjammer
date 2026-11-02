//! Struct values in loop units: a struct local whose fields are all scalars is kept as one variable
//! per field (it is rebuilt into its slot at an exit or before a closure-run statement), struct
//! literals of a declared struct build such values without allocating, and a call of a small user
//! method on one is inlined (its body lowered with `self` and the parameters bound to the call's
//! values).
//!
//! What a call stands for in the closure engine (a frame, a backtrace entry, the depth check, the
//! parameter and result coercions) is only observable when something fails; any failure inside the
//! inlined body leaves machine code at the caller's statement, which the closure engine then runs
//! with the real call. The depth check is the exception, so a unit with inlined calls only runs when
//! the stack has ample room (`osr::execute`). Inlined: methods with `self` and parameters the body does not mutate,
//! no decorators, not generic/async/extern, not recursive, whose body is `let`s and a tail
//! expression machine code lowers.

use super::kinds::Shape;
use super::lower::{Bind, Lo, StructSpill, Val, E, NO_SLOT};
use crate::interpreter::engine::FuncDef;
use crate::parser::{Expression, Pattern, Statement, Type};
use cranelift_codegen::ir::{types, InstBuilder, MemFlags};
use std::rc::Rc;

/// A struct local: its type, `copy` flag and one variable per field (declaration order).
pub(crate) struct VStruct {
    name: Rc<str>,
    copy: bool,
    fields: Vec<(Rc<str>, usize)>,
    slot: usize,
    fresh: bool,
}

/// A struct value being computed (a literal, a field-by-field copy, an inlined call's result).
#[derive(Clone)]
pub(super) struct VVal {
    name: Rc<str>,
    copy: bool,
    fields: Vec<(Rc<str>, Val)>,
}

/// What an inlined call yields.
pub(super) enum Out {
    Scalar(Val),
    Struct(VVal),
}

/// Nested inlined calls at most.
const MAX_INLINE: usize = 4;

impl VStruct {
    /// Does the next write of the local make its slot's box (a `let` in the loop not yet written)?
    pub(super) fn fresh(&self) -> bool {
        self.fresh
    }
    pub(super) fn slot(&self) -> usize {
        self.slot
    }
    /// A write made the box: later ones fill it.
    pub(super) fn boxed(&mut self) {
        self.fresh = false;
    }
}

impl Lo<'_, '_> {
    /// A struct local the loop is entered with: its fields come in the spill buffer.
    pub(super) fn entry_struct(&mut self, slot: usize, shape: &Shape) -> Option<usize> {
        let mut fields = Vec::new();
        for (n, t) in &shape.fields {
            let t = (*t)?;
            let id = self.new_var(NO_SLOT, t, true);
            let x = self.b.ins().load(types::I64, MemFlags::trusted(), self.buf, (id * 8) as i32);
            let x = self.from_bits(x, t);
            self.b.def_var(self.vars[id].var, x);
            fields.push((n.clone(), id));
        }
        self.vstructs.push(VStruct { name: shape.name.clone(), copy: shape.copy, fields, slot, fresh: false });
        Some(self.vstructs.len() - 1)
    }

    pub(super) fn struct_spill(&self, sid: usize) -> StructSpill {
        let s = &self.vstructs[sid];
        StructSpill {
            slot: s.slot,
            fresh: s.fresh,
            name: s.name.clone(),
            copy: s.copy,
            fields: s.fields.iter().map(|(n, id)| (n.clone(), *id, self.vars[*id].ty)).collect(),
        }
    }

    /// Bind `name` to a new struct local holding `v` (`slot`: its `let`'s slot, or none inside a call).
    fn bind_struct(&mut self, name: &'static str, v: VVal, slot: usize) -> usize {
        let mut fields = Vec::new();
        for (n, x) in v.fields {
            let id = self.new_var(NO_SLOT, x.ty, true);
            self.b.def_var(self.vars[id].var, x.v);
            fields.push((n, id));
        }
        self.vstructs.push(VStruct { name: v.name, copy: v.copy, fields, slot, fresh: true });
        let sid = self.vstructs.len() - 1;
        self.bind(name, Bind::Struct(sid));
        sid
    }

    fn struct_value(&mut self, sid: usize) -> VVal {
        let (name, copy) = (self.vstructs[sid].name.clone(), self.vstructs[sid].copy);
        let fields = self.vstructs[sid].fields.clone();
        let fields = fields.into_iter().map(|(n, id)| (n, Val::of(self.b.use_var(self.vars[id].var), self.vars[id].ty))).collect();
        VVal { name, copy, fields }
    }

    fn struct_local(&self, e: E) -> Option<usize> {
        match e {
            Expression::Identifier { name, .. } => match self.lookup(name)? {
                Bind::Struct(sid) => Some(*sid),
                _ => None,
            },
            _ => None,
        }
    }

    /// The field variable `field` of struct local `sid`.
    fn field_var(&self, sid: usize, field: &str) -> Option<usize> {
        self.vstructs[sid].fields.iter().find(|(n, _)| &**n == field).map(|(_, id)| *id)
    }

    /// `s.f` of a struct-valued expression (`None`: not one machine code holds).
    pub(super) fn struct_field(&mut self, object: E, field: &str) -> Option<Val> {
        if let Some(sid) = self.struct_local(object) {
            let id = self.field_var(sid, field)?;
            return Some(Val::of(self.b.use_var(self.vars[id].var), self.vars[id].ty));
        }
        if !matches!(object, Expression::StructLiteral { .. } | Expression::MethodCall { .. }) {
            return None;
        }
        let v = self.vexpr(object)?;
        v.fields.into_iter().find(|(n, _)| &**n == field).map(|(_, x)| x)
    }

    /// Does `e` name or build a struct machine code holds?
    pub(super) fn struct_valued(&self, e: E) -> bool {
        match e {
            Expression::StructLiteral { .. } => true,
            Expression::Identifier { .. } => self.struct_local(e).is_some(),
            Expression::MethodCall { object, .. } => self.struct_valued(object),
            _ => false,
        }
    }

    /// A struct-valued expression.
    pub(super) fn vexpr(&mut self, e: E) -> Option<VVal> {
        match e {
            Expression::Identifier { .. } => {
                let sid = self.struct_local(e)?;
                Some(self.struct_value(sid))
            }
            Expression::StructLiteral { name, fields, .. } => self.struct_literal(name, fields),
            Expression::MethodCall { object, method, type_args: None, arguments, .. } => match self.inline_call(object, method, arguments)? {
                Out::Struct(v) => Some(v),
                Out::Scalar(_) => None,
            },
            _ => None,
        }
    }

    /// `S { f: e, .. }` of a declared struct, as the walker builds it: fields in declaration order,
    /// each coerced to its declared (scalar) type.
    fn struct_literal(&mut self, name: &str, fields: &'static [(String, E)]) -> Option<VVal> {
        if name.contains("::") || matches!(name, "Self" | "RangeFrom") || self.it.type_aliases.contains_key(name) || !self.it.dup_types.is_empty() {
            return None;
        }
        let d = self.it.structs.get(name)?;
        if d.fields.len() != fields.len() {
            return None;
        }
        let (rc_name, rc_fields, decl, copy) = (d.rc_name.clone(), d.rc_fields.clone(), d.fields.clone(), d.copy);
        let mut out = Vec::with_capacity(decl.len());
        for (i, (f, ty)) in decl.iter().enumerate() {
            let (_, ex) = fields.iter().find(|(n, _)| n == f)?;
            let v = self.expr(ex)?;
            out.push((rc_fields[i].clone(), self.coerce(v, ty)?));
        }
        Some(VVal { name: rc_name, copy, fields: out })
    }

    /// `recv.method(args)` with a user method machine code can inline.
    pub(super) fn inline_call(&mut self, object: E, method: &str, args: &'static [(Option<String>, E)]) -> Option<Out> {
        // the receiver's name is its declared type key, the key methods are registered under, so
        // same-name types in different modules find their own methods
        if self.frames.len() >= MAX_INLINE || self.it.fast_off("inline") || !self.struct_valued(object) {
            return None;
        }
        let recv = self.vexpr(object)?;
        let f = self.it.methods.get(&(recv.name.to_string(), method.to_string()))?.clone();
        if !self.inlinable(&f, args.len()) {
            return None;
        }
        // arguments in the caller's scope, coerced as the callee binds them
        let mut bound: Vec<(&'static str, Result<Val, VVal>)> = Vec::new();
        for (p, (label, a)) in f.params[1..].iter().zip(args) {
            if label.is_some() {
                return None;
            }
            let v = if self.struct_valued(a) { Err(self.vexpr(a)?) } else { Ok(self.coerce_param(a, &p.type_)?) };
            bound.push((p.name.as_str(), v));
        }
        let base = self.scope_count();
        self.frames.push(base);
        self.push_scope();
        self.bind_struct("self", recv, NO_SLOT);
        for (n, v) in bound {
            match v {
                Ok(x) => {
                    let id = self.new_var(NO_SLOT, x.ty, true);
                    self.b.def_var(self.vars[id].var, x.v);
                    self.bind(n, Bind::Var(id));
                }
                Err(s) => {
                    self.bind_struct(n, s, NO_SLOT);
                }
            }
        }
        let r = self.inline_body(&f);
        while self.scope_count() > base {
            self.pop_scope();
        }
        self.frames.pop();
        self.inlines |= r.is_some();
        r
    }

    fn coerce_param(&mut self, a: E, ty: &Type) -> Option<Val> {
        let v = self.expr(a)?;
        self.coerce(v, ty)
    }

    fn inlinable(&mut self, f: &Rc<FuncDef>, nargs: usize) -> bool {
        let self_first = f.params.first().is_some_and(|p| p.name == "self");
        self_first
            && f.params.len() == nargs + 1
            && f.ret.is_some()
            && f.decorators.is_empty()
            && f.type_params.is_empty()
            && !f.is_async
            && !f.is_extern
            && !self.it.param_mutation(f).iter().any(|m| *m)
            && !self.it.self_mutation(f)
    }

    /// The callee's `let`s and tail, its result coerced to the declared return type.
    fn inline_body(&mut self, f: &Rc<FuncDef>) -> Option<Out> {
        let (last, init) = f.body.split_last()?;
        for s in init {
            let Statement::Let { pattern: Pattern::Identifier(n) | Pattern::MutBinding(n), type_: None, value, else_block: None, .. } = s else { return None };
            if self.struct_valued(value) {
                let v = self.vexpr(value)?;
                self.bind_struct(n.as_str(), v, NO_SLOT);
            } else {
                let v = self.expr(value)?;
                let id = self.new_var(NO_SLOT, v.ty, true);
                self.b.def_var(self.vars[id].var, v.v);
                self.bind(n.as_str(), Bind::Var(id));
            }
        }
        let Statement::Expression { expr, .. } = last else { return None };
        let ret = f.ret?;
        if self.struct_valued(expr) {
            let v = self.vexpr(expr)?;
            let declared = match ret {
                Type::Custom(n) if n == "Self" => f.self_ty.as_deref()?.to_string(),
                Type::Custom(n) => n.clone(),
                _ => return None,
            };
            return (*v.name == *declared).then_some(Out::Struct(v));
        }
        let v = self.expr(expr)?;
        Some(Out::Scalar(self.coerce(v, ret)?))
    }

    /// `x = e` where `x` is a struct local: the same struct type, field by field.
    pub(super) fn assign_struct(&mut self, sid: usize, value: E) -> Option<()> {
        let v = self.vexpr(value)?;
        let same = |s: &VStruct| *v.name == *s.name && v.fields.len() == s.fields.len() && v.fields.iter().zip(&s.fields).all(|((a, _), (b, _))| a == b);
        if !same(&self.vstructs[sid]) {
            return None;
        }
        let ids: Vec<usize> = self.vstructs[sid].fields.iter().map(|(_, id)| *id).collect();
        for (id, (_, x)) in ids.into_iter().zip(v.fields) {
            self.set_var(id, x)?;
        }
        Some(())
    }

    /// The field variable a `s.f = ..` targets, if `s` is a struct local.
    pub(super) fn struct_target(&self, target: E) -> Option<usize> {
        let Expression::FieldAccess { object, field, .. } = target else { return None };
        let sid = self.struct_local(object)?;
        self.field_var(sid, field)
    }

    /// `let x = <struct>`: a struct local in the let's slot.
    pub(super) fn let_struct(&mut self, name: &'static str, value: E, slot: usize) -> Option<()> {
        let v = self.vexpr(value)?;
        self.bind_struct(name, v, slot);
        Some(())
    }

}
