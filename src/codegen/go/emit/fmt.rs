//! Display / Debug formatting (Go).

use super::names::go_string;
use super::Go;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::Ty;

impl<'m> Go<'m> {
    pub fn format(&mut self, pieces: &[FmtPiece]) -> PResult<String> {
        if pieces.is_empty() {
            return Ok("\"\"".into());
        }
        let mut parts = Vec::new();
        for p in pieces {
            parts.push(match p {
                FmtPiece::Lit(s) => go_string(s),
                FmtPiece::Arg(e, spec) => self.display(e, spec)?,
            });
        }
        if !matches!(pieces[0], FmtPiece::Lit(_)) {
            parts.insert(0, "\"\"".into());
        }
        Ok(format!("({})", parts.join(" + ")))
    }

    /// Rust `Formatter` options of a spec as a `*wjFmt` literal, or None when all are default.
    fn fmt_opts(spec: &FmtSpec) -> Option<String> {
        let default = spec.width.is_none() && spec.precision.is_none() && spec.fill.is_none() && spec.align.is_none() && !spec.zero_pad && !spec.plus && !spec.alternate;
        if default {
            return None;
        }
        let w = spec.width.map(|w| w as i64).unwrap_or(-1);
        let p = spec.precision.map(|p| p as i64).unwrap_or(-1);
        let fill = spec.fill.map(|c| c as u32).unwrap_or(0);
        let align = spec.align.map(|c| c as u32).unwrap_or(0);
        Some(format!("&wjFmt{{W: {w}, P: {p}, Fill: {fill}, Align: {align}, Zero: {}, Plus: {}, Alt: {}}}", spec.zero_pad, spec.plus, spec.alternate))
    }

    /// Display (or Debug / numeric-radix per spec) of one argument, with Rust's padding rules.
    pub fn display(&mut self, e: &PExpr, spec: &FmtSpec) -> PResult<String> {
        let v = self.expr(e)?;
        let opts = Self::fmt_opts(spec);
        if spec.debug {
            return Ok(match opts {
                Some(o) => format!("wjDbgO({v}, {o})"),
                None => format!("wjDbg({v})"),
            });
        }
        let o = opts.clone().unwrap_or_else(|| "nil".into());
        if let Some(r) = spec.radix {
            return match &e.ty {
                Ty::Int(_) => Ok(format!("wjFmtInt({v}, '{r}', {o})")),
                Ty::F64 | Ty::F32 if r == 'e' => Ok(match opts {
                    Some(o) => format!("wjFmtFloat(float64({v}), 64, false, true, {o})"),
                    None => format!("wjExpFmt(float64({v}))"),
                }),
                _ => self.unsupported("radix format on non-integer"),
            };
        }
        let Some(o) = opts else { return self.plain_display(&e.ty, &v) };
        Ok(match &e.ty {
            Ty::Int(_) => format!("wjFmtInt({v}, 0, {o})"),
            Ty::F64 => format!("wjFmtFloat({v}, 64, false, false, {o})"),
            Ty::F32 => format!("wjFmtFloat(float64({v}), 32, false, false, {o})"),
            _ => {
                let s = self.plain_display(&e.ty, &v)?;
                format!("wjFmtStr({s}, {o})")
            }
        })
    }

    fn plain_display(&mut self, t: &Ty, v: &str) -> PResult<String> {
        Ok(match t {
            Ty::Int(_) => format!("wjItoa({v})"),
            Ty::F32 => format!("wjF32({v})"),
            Ty::F64 => format!("wjF64({v})"),
            Ty::Bool => format!("wjBool({v})"),
            Ty::Str => v.to_string(),
            Ty::Char => format!("string(rune({v}))"),
            Ty::Unit => "\"()\"".into(),
            Ty::Adt(n, _) => {
                if self.find_fn(Some(n), "to_string").is_some() {
                    format!("{}.to_string()", v)
                } else {
                    return self.unsupported(format!("Display of `{n}` (use `{{:?}}`)"));
                }
            }
            other => return self.unsupported(format!("Display of `{}` (use `{{:?}}`)", other.describe())),
        })
    }
}
