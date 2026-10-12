//! `&mut` realisation for JavaScript/TypeScript.
//!
//! JS has no pointers. Values whose JS representation is an immutable primitive or a value that is
//! replaced rather than mutated (numbers, BigInt, strings, booleans, `Option`, `Result`, enum
//! variant objects, generics) are borrowed mutably through an accessor **cell** `{ v }` whose
//! getter/setter read and write the borrowed place (`$cellOf`, `$cellIx`, or an inline literal
//! for a local). Object-represented values (structs, `Vec`, `HashMap`, `HashSet`, tuples) are
//! already shared by reference; a whole-value write through such an alias (`*r = x`) replaces the
//! contents in place (`$replace`).
//!
//! Where a borrow exists is not decided here: parameter modes (`ParamMode::MutRef`) and
//! `let r = &mut place` (`PK::BorrowMut`) come from the portable IR. Code without `&mut` of a
//! value-represented type is emitted exactly as before.

use super::names::{ident, prop};
use super::Js;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::Ty;

impl<'m> Js<'m> {
    /// Is `&mut T` realised as an accessor cell (rather than by sharing the object)?
    pub fn cell_ty(&self, t: &Ty) -> bool {
        match t {
            Ty::Adt(n, _) => !(self.is_struct(n) || self.m.traits.iter().any(|t| t.name == *n)),
            Ty::Vec(_) | Ty::Map(..) | Ty::Set(_) | Ty::Tuple(_) => false,
            _ => true,
        }
    }

    pub fn is_enum(&self, n: &str) -> bool {
        self.m.enums.iter().any(|e| e.name == n)
    }

    /// The program function a call resolves to (free function, or method of a user type).
    pub fn find_fn(&self, owner: Option<&str>, name: &str) -> Option<&'m PFn> {
        let m = self.m;
        match owner {
            None => m.fns.iter().find(|f| f.name == name),
            Some(o) => {
                let s = m.structs.iter().find(|s| s.name == o).and_then(|s| s.methods.iter().find(|f| f.name == name));
                let e = m.enums.iter().find(|e| e.name == o).and_then(|e| e.methods.iter().find(|f| f.name == name));
                s.or(e)
            }
        }
    }

    /// Enum method with a `&mut self` receiver: emitted as a static function taking a cell.
    pub fn is_enum_mut_method(&self, owner: &str, f: &PFn) -> bool {
        f.recv == Some(ParamMode::MutRef) && self.is_enum(owner)
    }

    /// Arguments for a call of `f`; `&mut` parameters of cell types receive a cell.
    pub fn args_for(&mut self, f: Option<&PFn>, args: &[PExpr]) -> PResult<String> {
        let mut out = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let cell = f.and_then(|f| f.params.get(i)).map(|p| p.mode == ParamMode::MutRef && self.cell_ty(&p.ty)).unwrap_or(false);
            out.push(if cell { self.cell_of(a)? } else { self.expr(a)? });
        }
        Ok(out.join(", "))
    }

    /// A cell aliasing the place `e` (a cell local is passed on as is: a reborrow).
    pub fn cell_of(&mut self, e: &PExpr) -> PResult<String> {
        match &*e.kind {
            PK::Local(n) if self.cells.contains(n) => Ok(ident(n)),
            PK::Local(n) if n == "self" => self.unsupported("`&mut self` of a value-represented type outside an enum method"),
            PK::Local(n) => {
                let (id, x) = (ident(n), self.fresh("x"));
                let ann = if self.ts { format!(": {}", self.ts_type(&e.ty)) } else { String::new() };
                Ok(format!("{{ get v(){ann} {{ return {id}; }}, set v({x}{ann}) {{ {id} = {x}; }} }}"))
            }
            PK::Copy(inner) | PK::BorrowMut(inner) => self.cell_of(inner),
            PK::Field(obj, f) => Ok(format!("$cellOf({}, \"{}\")", self.expr(obj)?, prop(f))),
            PK::TupleIdx(obj, i) => Ok(format!("$cellOf({}, {i})", self.expr(obj)?)),
            PK::Index(obj, idx) if matches!(obj.ty, Ty::Vec(_)) => Ok(format!("$cellIx({}, {})", self.expr(obj)?, self.expr(idx)?)),
            PK::Index(..) => self.unsupported("`&mut` of a map entry (use `get_mut` / `entry`)"),
            // A temporary: nothing else can observe it, a plain cell is exact.
            _ => Ok(format!("{{ v: {} }}", self.expr(e)?)),
        }
    }

    /// `let r = &mut place`.
    pub fn let_borrow(&mut self, name: &str, place: &PExpr) -> PResult<()> {
        if self.cell_ty(&place.ty) {
            let c = self.cell_of(place)?;
            self.line(&format!("const {} = {c};", ident(name)));
            self.cells.insert(name.to_string());
        } else {
            let v = self.expr(place)?;
            self.line(&format!("const {} = {v};", ident(name)));
            self.obj_aliases.insert(name.to_string());
        }
        Ok(())
    }

    /// Register how the parameters of `f` are represented; returns the previous state.
    pub fn enter_fn(&mut self, f: &PFn, owner: Option<&str>) -> (std::collections::HashSet<String>, std::collections::HashSet<String>) {
        let saved = (std::mem::take(&mut self.cells), std::mem::take(&mut self.obj_aliases));
        for p in &f.params {
            if p.mode == ParamMode::MutRef {
                if self.cell_ty(&p.ty) {
                    self.cells.insert(p.name.clone());
                } else {
                    self.obj_aliases.insert(p.name.clone());
                }
            }
        }
        if f.recv == Some(ParamMode::MutRef) {
            match owner {
                Some(o) if self.is_enum(o) => self.cells.insert("self".into()),
                _ => self.obj_aliases.insert("self".into()),
            };
        }
        saved
    }

    pub fn leave_fn(&mut self, saved: (std::collections::HashSet<String>, std::collections::HashSet<String>)) {
        self.cells = saved.0;
        self.obj_aliases = saved.1;
    }

    /// TypeScript annotation of a parameter (cells are `{ v: T }`).
    pub fn param_ts(&self, p: &PParam) -> String {
        let t = self.ts_type(&p.ty);
        if p.mode == ParamMode::MutRef && self.cell_ty(&p.ty) { format!("{{ v: {t} }}") } else { t }
    }

    /// Is the assignment target an alias whose whole value is replaced in place?
    pub fn is_obj_alias(&self, target: &PExpr) -> bool {
        matches!(&*target.kind, PK::Local(n) if self.obj_aliases.contains(n))
    }

    /// `for x in v.iter_mut()` / `m.values_mut()` over cell-typed elements: `x` is a cell on the
    /// element slot. Returns false when the ordinary loop applies (object elements are shared).
    pub fn for_in_mut(&mut self, pat: &PPat, iter: &PExpr, body: &[PStmt]) -> PResult<bool> {
        let PK::Method { recv, name, .. } = &*iter.kind else { return Ok(false) };
        let PPat::Bind { name: var, .. } = pat else { return Ok(false) };
        let elem = match (&recv.ty, name.as_str()) {
            (Ty::Vec(e), "iter_mut") => (**e).clone(),
            (Ty::Map(_, v), "values_mut") => (**v).clone(),
            _ => return Ok(false),
        };
        if !self.cell_ty(&elem) {
            return Ok(false);
        }
        let (src, k) = (self.fresh("s"), self.fresh("k"));
        let r = self.expr(recv)?;
        let prefix = self.loop_label_prefix("for");
        self.line(&format!("const {src} = {r};"));
        let (head, cell) = if matches!(recv.ty, Ty::Vec(_)) {
            (format!("for (let {k} = 0; {k} < {src}.length; {k}++)"), format!("$cellOf({src}, {k})"))
        } else {
            let x = self.fresh("x");
            (format!("for (const {k} of Array.from({src}.keys()))"), format!("{{ get v() {{ return {src}.get({k}); }}, set v({x}) {{ {src}.set({k}, {x}); }} }}"))
        };
        self.line(&format!("{prefix}{head} {{"));
        self.ind += 1;
        self.line(&format!("const {} = {cell};", ident(var)));
        self.cells.insert(var.clone());
        self.stmts(body)?;
        self.ind -= 1;
        self.line("}");
        Ok(true)
    }
}
