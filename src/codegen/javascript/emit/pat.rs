//! Pattern tests and bindings.

use super::names::{ident, prop, variant_class};
use super::Js;
use crate::codegen::portable::ir::*;

impl<'m> Js<'m> {
    /// Collect the boolean conditions and `(name, expr)` bindings of matching `scrut` against `p`.
    pub fn pat(&mut self, p: &PPat, scrut: &str, conds: &mut Vec<String>, binds: &mut Vec<(String, String)>) -> PResult<()> {
        match p {
            PPat::Wild => {}
            PPat::Bind { name, .. } => binds.push((ident(name), scrut.to_string())),
            PPat::Lit(e) => {
                let lit = self.expr(e)?;
                conds.push(format!("{scrut} === {lit}"));
            }
            PPat::Range { lo, hi, inclusive } => {
                if let Some(lo) = lo {
                    let l = self.expr(lo)?;
                    conds.push(format!("{scrut} >= {l}"));
                }
                if let Some(hi) = hi {
                    let h = self.expr(hi)?;
                    conds.push(format!("{scrut} {} {h}", if *inclusive { "<=" } else { "<" }));
                }
            }
            PPat::Tuple(ps) => {
                for (i, sp) in ps.iter().enumerate() {
                    self.pat(sp, &format!("{scrut}[{i}]"), conds, binds)?;
                }
            }
            PPat::Or(alts) => {
                let mut parts = Vec::new();
                for a in alts {
                    let (mut c, mut b) = (Vec::new(), Vec::new());
                    self.pat(a, scrut, &mut c, &mut b)?;
                    if !b.is_empty() {
                        return self.unsupported("or-pattern with bindings");
                    }
                    parts.push(if c.is_empty() { "true".to_string() } else { c.join(" && ") });
                }
                conds.push(format!("({})", parts.join(" || ")));
            }
            PPat::Variant { enum_name, variant, fields } => {
                let _ = variant_class(enum_name, variant);
                conds.push(format!("{scrut}.$t === {}", super::names::js_string(variant)));
                for (f, sp) in fields {
                    self.pat(sp, &format!("{scrut}.{}", prop(f)), conds, binds)?;
                }
            }
            PPat::Some(sp) => {
                conds.push(format!("{scrut} !== null"));
                self.pat(sp, &format!("{scrut}.v"), conds, binds)?;
            }
            PPat::None => conds.push(format!("{scrut} === null")),
            PPat::Ok(sp) => {
                conds.push(format!("{scrut}.ok"));
                self.pat(sp, &format!("{scrut}.v"), conds, binds)?;
            }
            PPat::Err(sp) => {
                conds.push(format!("!{scrut}.ok"));
                self.pat(sp, &format!("{scrut}.v"), conds, binds)?;
            }
        }
        Ok(())
    }
}
