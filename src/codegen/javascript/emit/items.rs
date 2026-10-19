//! Module-level emission: classes, functions, constants, entry point.

use super::names::{ident, js_string, prop, variant_class};
use super::Js;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::Ty;

impl<'m> Js<'m> {
    pub fn module(&mut self) -> PResult<()> {
        let m = self.m;
        for s in &m.structs {
            self.struct_class(s)?;
        }
        for e in &m.enums {
            self.enum_classes(e)?;
        }
        for f in &m.fns {
            self.function(f, None)?;
        }
        for c in &m.consts {
            let v = self.expr(&c.value)?;
            let ann = if self.ts { format!(": {}", self.ts_type(&c.ty)) } else { String::new() };
            self.line(&format!("export const {}{ann} = {v};", ident(&c.name)));
        }
        if m.fns.iter().any(|f| f.name == "main") {
            self.line("$run(main, import.meta.url);");
        }
        Ok(())
    }

    fn params(&self, f: &PFn) -> String {
        f.params
            .iter()
            .map(|p| if self.ts { format!("{}: {}", ident(&p.name), self.param_ts(p)) } else { ident(&p.name) })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Emit a free function (`owner: None`) or a method of the class `owner`.
    pub fn function(&mut self, f: &PFn, owner: Option<&str>) -> PResult<()> {
        let saved = self.enter_fn(f, owner);
        let r = self.function_inner(f, owner);
        self.leave_fn(saved);
        r
    }

    fn function_inner(&mut self, f: &PFn, owner: Option<&str>) -> PResult<()> {
        let ret = if self.ts { format!(": {}", self.ts_type(&f.ret)) } else { String::new() };
        let head = if let Some(o) = owner {
            if self.is_enum_mut_method(o, f) {
                // `&mut self` on an enum: the receiver is a cell (`self.v`), so a static function.
                let ann = if self.ts { format!(": {{ v: {} }}", ident(o)) } else { String::new() };
                let ps = self.params(f);
                let sep = if ps.is_empty() { "" } else { ", " };
                format!("static {}(self{ann}{sep}{ps}){ret} {{", self.method_name(&f.name))
            } else {
                let kw = if f.recv.is_none() { "static " } else { "" };
                format!("{kw}{}({}){ret} {{", self.method_name(&f.name), self.params(f))
            }
        } else {
            format!("export function {}({}){ret} {{", ident(&f.name), self.params(f))
        };
        if f.body.is_none() {
            return Ok(());
        }
        if f.is_async {
            return self.unsupported(format!("async fn `{}`", f.name));
        }
        self.line(&head);
        self.ind += 1;
        if let Some(b) = &f.body {
            self.stmts(&b.stmts)?;
        }
        self.ind -= 1;
        self.line("}");
        Ok(())
    }

    fn field_decls(&mut self, fields: &[(String, Ty)]) {
        if self.ts {
            for (n, t) in fields {
                let decl = format!("{}: {};", prop(n), self.ts_type(t));
                self.line(&decl);
            }
        }
    }

    fn ctor(&mut self, fields: &[(String, Ty)], is_variant: bool, tag: Option<&str>) {
        let ps: Vec<String> = fields.iter().map(|(n, t)| if self.ts { format!("{}: {}", prop(n), self.ts_type(t)) } else { prop(n) }).collect();
        self.line(&format!("constructor({}) {{", ps.join(", ")));
        self.ind += 1;
        if is_variant {
            self.line("super();");
        }
        if let Some(t) = tag {
            self.line(&format!("this.$t = {};", js_string(t)));
        }
        for (n, _) in fields {
            self.line(&format!("this.{0} = {0};", prop(n)));
        }
        self.ind -= 1;
        self.line("}");
    }

    fn clone_method(&mut self, class: &str, fields: &[(String, Ty)]) {
        let args: Vec<String> = fields.iter().map(|(n, t)| self.clone_js(t, &format!("this.{}", prop(n)))).collect();
        self.line(&format!("clone() {{ return new {class}({}); }}", args.join(", ")));
    }

    /// Derived `Debug`: `o` carries the caller's Formatter options to every field.
    fn dbg_method(&mut self, display: &str, fields: &[(String, Ty)], tuple: bool) -> PResult<()> {
        let body = if fields.is_empty() {
            js_string(display)
        } else if tuple {
            let parts: PResult<Vec<String>> = fields.iter().map(|(n, t)| Ok(format!("{}(this.{}, o)", self.dbg_fn(t)?, prop(n)))).collect();
            format!("$dbgTupleS({}, [{}], o)", js_string(display), parts?.join(", "))
        } else {
            let parts: PResult<Vec<String>> = fields.iter().map(|(n, t)| Ok(format!("[{}, {}(this.{}, o)]", js_string(n), self.dbg_fn(t)?, prop(n)))).collect();
            format!("$dbgStruct({}, [{}], o)", js_string(display), parts?.join(", "))
        };
        let param = if self.ts { "o?: any" } else { "o" };
        let ret = if self.ts { ": string" } else { "" };
        self.line(&format!("$dbg({param}){ret} {{ return {body}; }}"));
        Ok(())
    }

    fn struct_class(&mut self, s: &PStruct) -> PResult<()> {
        let name = ident(&s.name);
        self.line(&format!("export class {name} {{"));
        self.ind += 1;
        self.field_decls(&s.fields);
        self.ctor(&s.fields, false, None);
        self.clone_method(&name, &s.fields);
        self.dbg_method(&s.name, &s.fields, s.tuple)?;
        for m in &s.methods {
            self.function(m, Some(&s.name))?;
        }
        self.ind -= 1;
        self.line("}");
        Ok(())
    }

    fn enum_classes(&mut self, e: &PEnum) -> PResult<()> {
        let base = ident(&e.name);
        self.line(&format!("export class {base} {{"));
        self.ind += 1;
        if self.ts {
            self.line("$t!: string;");
        }
        for m in &e.methods {
            self.function(m, Some(&e.name))?;
        }
        self.ind -= 1;
        self.line("}");
        for v in &e.variants {
            let class = variant_class(&e.name, &v.name);
            self.line(&format!("export class {class} extends {base} {{"));
            self.ind += 1;
            self.field_decls(&v.fields);
            self.ctor(&v.fields, true, Some(&v.name));
            self.clone_method(&class, &v.fields);
            self.dbg_method(&v.name, &v.fields, !v.named)?;
            self.ind -= 1;
            self.line("}");
        }
        Ok(())
    }
}
