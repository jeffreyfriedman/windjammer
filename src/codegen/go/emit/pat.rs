//! Pattern tests and bindings (Go).

use super::names::{field, ident, variant_type};
use super::Go;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::Ty;

impl<'m> Go<'m> {
    pub fn pat(&mut self, p: &PPat, scrut: &str, ty: &Ty, conds: &mut Vec<String>, binds: &mut Vec<(String, String)>) -> PResult<()> {
        match p {
            PPat::Wild => {}
            PPat::Bind { name, .. } => binds.push((ident(name), scrut.to_string())),
            PPat::Lit(e) => {
                let lit = self.expr(e)?;
                conds.push(format!("{scrut} == {lit}"));
            }
            PPat::Tuple(ps) => {
                let tys = match ty {
                    Ty::Tuple(ts) => ts.clone(),
                    _ => return self.unsupported("tuple pattern on non-tuple"),
                };
                for (i, sp) in ps.iter().enumerate() {
                    self.pat(sp, &format!("{scrut}.F{i}"), &tys[i], conds, binds)?;
                }
            }
            PPat::Or(alts) => {
                let mut parts = Vec::new();
                for a in alts {
                    let (mut c, mut b) = (Vec::new(), Vec::new());
                    self.pat(a, scrut, ty, &mut c, &mut b)?;
                    if !b.is_empty() {
                        return self.unsupported("or-pattern with bindings");
                    }
                    parts.push(if c.is_empty() { "true".to_string() } else { c.join(" && ") });
                }
                conds.push(format!("({})", parts.join(" || ")));
            }
            PPat::Variant { enum_name, variant, fields } => {
                let targs = self.targs(ty)?;
                let vt = format!("{}{targs}", variant_type(enum_name, variant));
                conds.push(format!("wjIs[{vt}]({scrut})"));
                let ftys = self.variant_field_tys(enum_name, variant, ty);
                for (f, sp) in fields {
                    let fty = ftys.iter().find(|(n, _)| n == f).map(|(_, t)| t.clone()).unwrap_or(Ty::Unknown);
                    self.pat(sp, &format!("{scrut}.({vt}).{}", field(f)), &fty, conds, binds)?;
                }
            }
            PPat::Some(sp) => {
                conds.push(format!("{scrut}.Ok"));
                let inner = if let Ty::Option(t) = ty { (**t).clone() } else { Ty::Unknown };
                self.pat(sp, &format!("{scrut}.V"), &inner, conds, binds)?;
            }
            PPat::None => conds.push(format!("!{scrut}.Ok")),
            PPat::Ok(sp) => {
                conds.push(format!("{scrut}.Ok"));
                let inner = if let Ty::Result(t, _) = ty { (**t).clone() } else { Ty::Unknown };
                self.pat(sp, &format!("{scrut}.V"), &inner, conds, binds)?;
            }
            PPat::Err(sp) => {
                conds.push(format!("!{scrut}.Ok"));
                let inner = if let Ty::Result(_, e) = ty { (**e).clone() } else { Ty::Unknown };
                self.pat(sp, &format!("{scrut}.E"), &inner, conds, binds)?;
            }
        }
        Ok(())
    }

    fn variant_field_tys(&self, en: &str, var: &str, ty: &Ty) -> Vec<(String, Ty)> {
        let Some(e) = self.m.enums.iter().find(|e| e.name == en) else { return vec![] };
        let Some(v) = e.variants.iter().find(|v| v.name == var) else { return vec![] };
        let args = if let Ty::Adt(_, a) = ty { a.clone() } else { vec![] };
        v.fields.iter().map(|(n, t)| (n.clone(), t.subst(&e.type_params, &args))).collect()
    }
}
