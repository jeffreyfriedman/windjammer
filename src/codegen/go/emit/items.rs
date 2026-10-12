//! Module-level emission for Go.

use super::names::{field, go_string, ident, variant_type};
use super::Go;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::Ty;

impl<'m> Go<'m> {
    pub fn module(&mut self) -> PResult<()> {
        let m = self.m;
        for t in &m.traits {
            self.trait_iface(t)?;
        }
        for s in &m.structs {
            self.struct_decl(s)?;
        }
        for e in &m.enums {
            self.enum_decl(e)?;
        }
        for f in &m.fns {
            self.function(f, None)?;
        }
        for s in &m.structs {
            for f in &s.methods {
                self.function(f, Some((&s.name, &s.type_params, true)))?;
            }
        }
        for e in &m.enums {
            for f in &e.methods {
                self.function(f, Some((&e.name, &e.type_params, false)))?;
            }
        }
        for c in &m.consts {
            let v = self.expr(&c.value)?;
            let t = self.ty(&c.ty)?;
            self.line(&format!("var {} {t} = {v}", ident(&c.name)));
        }
        if m.fns.iter().any(|f| f.name == "main") {
            self.line("func main() { wjRun(wj_main) }");
        }
        Ok(())
    }

    fn tparams_decl(&self, names: &[String], bounds: &[(String, Vec<String>)]) -> String {
        if names.is_empty() {
            return String::new();
        }
        let parts: Vec<String> = names
            .iter()
            .map(|n| {
                let b = bounds.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone()).unwrap_or_default();
                let user: Vec<&String> = b.iter().filter(|t| self.is_trait(t)).collect();
                match user.first() {
                    Some(t) => format!("{n} {}", ident(t)),
                    None => format!("{n} any"),
                }
            })
            .collect();
        format!("[{}]", parts.join(", "))
    }

    fn tparams_use(names: &[String]) -> String {
        if names.is_empty() { String::new() } else { format!("[{}]", names.join(", ")) }
    }

    fn sig_params(&mut self, f: &PFn) -> PResult<String> {
        let mut ps = Vec::new();
        for p in &f.params {
            let t = self.ty(&p.ty)?;
            if p.mode == ParamMode::MutRef && !self.is_ref_type(&p.ty) {
                self.ptr_locals.insert(p.name.clone(), p.ty.clone());
                ps.push(format!("{} *{t}", ident(&p.name)));
            } else {
                ps.push(format!("{} {t}", ident(&p.name)));
            }
        }
        Ok(ps.join(", "))
    }

    fn trait_iface(&mut self, t: &PTrait) -> PResult<()> {
        self.line(&format!("type {} interface {{", ident(&t.name)));
        self.ind += 1;
        for m in &t.methods {
            let ps: PResult<Vec<String>> = m.params.iter().map(|p| Ok(format!("{} {}", ident(&p.name), self.ty(&p.ty)?))).collect();
            let ret = if m.ret == Ty::Unit { String::new() } else { format!(" {}", self.ty(&m.ret)?) };
            self.line(&format!("{}({}){ret}", ident(&m.name), ps?.join(", ")));
        }
        self.ind -= 1;
        self.line("}");
        Ok(())
    }

    /// Derived `Debug`: `f` carries the caller's Formatter options to every field.
    fn dbg_body(&mut self, display: &str, fields: &[(String, Ty)], tuple: bool, recv: &str) -> String {
        if fields.is_empty() {
            return go_string(display);
        }
        let vals: Vec<String> = fields.iter().map(|(n, _)| format!("{recv}.{}", field(n))).collect();
        if tuple {
            format!("wjDbgTupleS(f, {}, {})", go_string(display), vals.join(", "))
        } else {
            let names: Vec<String> = fields.iter().map(|(n, _)| go_string(n)).collect();
            format!("wjDbgStruct(f, {}, []string{{{}}}, {})", go_string(display), names.join(", "), vals.join(", "))
        }
    }

    fn struct_decl(&mut self, s: &PStruct) -> PResult<()> {
        let tp = self.tparams_decl(&s.type_params, &[]);
        let tu = Self::tparams_use(&s.type_params);
        let name = ident(&s.name);
        self.line(&format!("type {name}{tp} struct {{"));
        self.ind += 1;
        for (n, t) in &s.fields {
            let ty = self.ty(t)?;
            self.line(&format!("{} {ty}", field(n)));
        }
        self.ind -= 1;
        self.line("}");
        let body = self.dbg_body(&s.name, &s.fields, s.tuple, "self");
        self.line(&format!("func (self {name}{tu}) wjDbgF(f *wjFmt) string {{ return {body} }}"));
        Ok(())
    }

    fn enum_decl(&mut self, e: &PEnum) -> PResult<()> {
        let tp = self.tparams_decl(&e.type_params, &[]);
        let tu = Self::tparams_use(&e.type_params);
        let name = ident(&e.name);
        self.line(&format!("type {name}{tp} interface {{ is{name}() }}"));
        for v in &e.variants {
            let vt = variant_type(&e.name, &v.name);
            self.line(&format!("type {vt}{tp} struct {{"));
            self.ind += 1;
            for (n, t) in &v.fields {
                let ty = self.ty(t)?;
                self.line(&format!("{} {ty}", field(n)));
            }
            self.ind -= 1;
            self.line("}");
            self.line(&format!("func ({vt}{tu}) is{name}() {{}}"));
            let body = self.dbg_body(&v.name, &v.fields, !v.named, "self");
            self.line(&format!("func (self {vt}{tu}) wjDbgF(f *wjFmt) string {{ return {body} }}"));
        }
        Ok(())
    }

    /// Emit a free function, struct method (`(owner, params, is_struct)`) or enum/static function.
    pub fn function(&mut self, f: &PFn, owner: Option<(&String, &Vec<String>, bool)>) -> PResult<()> {
        if f.body.is_none() {
            return Ok(());
        }
        if f.is_async {
            return self.unsupported(format!("async fn `{}`", f.name));
        }
        self.ptr_locals.clear();
        let mut tparams: Vec<String> = Vec::new();
        if let Some((_, op, false)) = owner {
            tparams.extend(op.iter().cloned());
        }
        if let Some((_, op, true)) = owner {
            if f.recv.is_none() {
                tparams.extend(op.iter().cloned());
            }
        }
        tparams.extend(f.type_params.iter().cloned());
        let tp = self.tparams_decl(&tparams, &f.bounds);
        let mut params = self.sig_params(f)?;
        let ret = if f.ret == Ty::Unit { String::new() } else { format!(" {}", self.ty(&f.ret)?) };
        let head = match owner {
            None => format!("func {}{tp}({params}){ret} {{", ident(&f.name)),
            Some((o, op, is_struct)) => {
                let (on, tu) = (ident(o), Self::tparams_use(op));
                match (f.recv, is_struct) {
                    (Some(mode), true) => {
                        if !f.type_params.is_empty() {
                            return self.unsupported(format!("generic method `{}::{}` (Go methods cannot declare type parameters)", o, f.name));
                        }
                        let rcv = if mode == ParamMode::MutRef {
                            self.ptr_locals.insert("self".into(), Ty::Adt(o.clone(), vec![]));
                            format!("self *{on}{tu}")
                        } else {
                            format!("self {on}{tu}")
                        };
                        format!("func ({rcv}) {}({params}){ret} {{", ident(&f.name))
                    }
                    (Some(mode), false) => {
                        let sep = if params.is_empty() { "" } else { ", " };
                        if mode == ParamMode::MutRef {
                            self.ptr_locals.insert("self".into(), Ty::Adt(o.clone(), vec![]));
                            params = format!("self *{on}{tu}{sep}{params}");
                        } else {
                            params = format!("self {on}{tu}{sep}{params}");
                        }
                        format!("func {on}_{}{tp}({params}){ret} {{", ident(&f.name))
                    }
                    (None, _) => format!("func {on}_{}{tp}({params}){ret} {{", ident(&f.name)),
                }
            }
        };
        self.ret = f.ret.clone();
        self.line(&head);
        self.ind += 1;
        let body = f.body.as_ref().unwrap();
        self.stmts(&body.stmts)?;
        if f.ret != Ty::Unit && !self.terminating(&body.stmts) {
            self.line("panic(\"unreachable\")");
        }
        self.ind -= 1;
        self.line("}");
        Ok(())
    }
}
