//! Display / Debug formatting.

use super::names::js_string;
use super::Js;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::Ty;

impl<'m> Js<'m> {
    pub fn format(&mut self, pieces: &[FmtPiece]) -> PResult<String> {
        if pieces.is_empty() {
            return Ok("\"\"".into());
        }
        let mut parts = Vec::new();
        for p in pieces {
            parts.push(match p {
                FmtPiece::Lit(s) => js_string(s),
                FmtPiece::Arg(e, spec) => self.display(e, spec)?,
            });
        }
        if !matches!(pieces[0], FmtPiece::Lit(_)) {
            parts.insert(0, "\"\"".into());
        }
        Ok(format!("({})", parts.join(" + ")))
    }

    /// Debug formatter function expression `(value, o) => string` for a type.
    pub fn dbg_fn(&mut self, t: &Ty) -> PResult<String> {
        Ok(match t {
            Ty::Int(_) => "$dbgInt".into(),
            Ty::Bool => "$dbgBool".into(),
            Ty::F64 => "$dbgF64".into(),
            Ty::F32 => "$dbgF32".into(),
            Ty::Str => "$dbgStr".into(),
            Ty::Char => "$dbgChar".into(),
            Ty::Unit => "$dbgUnit".into(),
            Ty::Vec(e) | Ty::Iter(e) => format!("$dbgVec({})", self.dbg_fn(e)?),
            Ty::Set(e) => format!("$dbgSet({})", self.dbg_fn(e)?),
            Ty::Map(k, v) => format!("$dbgMap({}, {})", self.dbg_fn(k)?, self.dbg_fn(v)?),
            Ty::Option(e) => format!("$dbgOpt({})", self.dbg_fn(e)?),
            Ty::Result(a, b) => format!("$dbgRes({}, {})", self.dbg_fn(a)?, self.dbg_fn(b)?),
            Ty::Tuple(ts) => {
                let fs: PResult<Vec<String>> = ts.iter().map(|t| self.dbg_fn(t)).collect();
                format!("$dbgTuple([{}])", fs?.join(", "))
            }
            Ty::Adt(..) => "$dbgObj".into(),
            Ty::Param(_) | Ty::Var(_) | Ty::Unknown => "$dbgAny".into(),
            Ty::Fn(..) | Ty::Range(_) => return self.unsupported("Debug formatting of closures/ranges"),
        })
    }

    /// Rust `Formatter` options of a spec as a JS object literal, or None when all are default.
    fn fmt_opts(spec: &FmtSpec) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(w) = spec.width {
            parts.push(format!("w: {w}"));
        }
        if let Some(f) = spec.fill {
            parts.push(format!("fill: {}", js_string(&f.to_string())));
        }
        if let Some(a) = spec.align {
            parts.push(format!("align: \"{a}\""));
        }
        if spec.zero_pad {
            parts.push("zero: true".into());
        }
        if spec.plus {
            parts.push("plus: true".into());
        }
        if spec.alternate {
            parts.push("alt: true".into());
        }
        if let Some(p) = spec.precision {
            parts.push(format!("prec: {p}"));
        }
        if parts.is_empty() { None } else { Some(format!("{{ {} }}", parts.join(", "))) }
    }

    /// Display (or Debug / numeric-radix per spec) of one argument, with Rust's padding rules.
    pub fn display(&mut self, e: &PExpr, spec: &FmtSpec) -> PResult<String> {
        let v = self.expr(e)?;
        let opts = Self::fmt_opts(spec);
        if spec.debug {
            let f = self.dbg_fn(&e.ty)?;
            return Ok(match opts {
                Some(o) => format!("{f}({v}, {o})"),
                None => format!("{f}({v})"),
            });
        }
        let o = opts.clone().unwrap_or_else(|| "undefined".into());
        if let Some(radix) = spec.radix {
            return match &e.ty {
                Ty::Int(k) => Ok(format!("$fmtInt({v}, {o}, \"{radix}\", {})", k.bits())),
                Ty::F64 | Ty::F32 if radix == 'e' => Ok(match opts {
                    Some(o) => format!("$fmtFloat({v}, {o}, false, false, true)"),
                    None => format!("$expFmt({v})"),
                }),
                _ => self.unsupported("radix format on non-integer"),
            };
        }
        let Some(o) = opts else { return self.plain_display(&e.ty, &v) };
        Ok(match &e.ty {
            Ty::Int(_) => format!("$fmtInt({v}, {o}, \"\", 0)"),
            Ty::F64 => format!("$fmtFloat({v}, {o}, false, false, false)"),
            Ty::F32 => format!("$fmtFloat({v}, {o}, false, true, false)"),
            _ => {
                let s = self.plain_display(&e.ty, &v)?;
                format!("$fmtStr({s}, {o})")
            }
        })
    }

    fn plain_display(&mut self, t: &Ty, v: &str) -> PResult<String> {
        Ok(match t {
            Ty::Int(_) => format!("String({v})"),
            Ty::F64 => format!("$f64({v})"),
            Ty::F32 => format!("$f32({v})"),
            Ty::Bool => format!("String({v})"),
            Ty::Str | Ty::Char => v.to_string(),
            Ty::Unit => "\"()\"".into(),
            Ty::Adt(n, _) => {
                if self.m.structs.iter().any(|s| s.name == *n && s.methods.iter().any(|m| m.name == "to_string")) {
                    format!("{v}.to_string()")
                } else {
                    return self.unsupported(format!("Display of `{n}` (only std scalar types implement `{{}}`; use `{{:?}}`)"));
                }
            }
            other => return self.unsupported(format!("Display of `{}` (use `{{:?}}`)", other.describe())),
        })
    }
}
