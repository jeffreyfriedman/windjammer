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

    pub fn display(&mut self, e: &PExpr, spec: &FmtSpec) -> PResult<String> {
        if spec.pretty && !matches!(e.ty, Ty::Int(_) | Ty::F64 | Ty::F32 | Ty::Bool | Ty::Str | Ty::Char | Ty::Unit) {
            return self.unsupported("pretty Debug `{:#?}` of an aggregate (use `{:?}`)");
        }
        let v = self.expr(e)?;
        let base = if spec.debug {
            format!("wjDbg({v})")
        } else if let Some(r) = spec.radix {
            match &e.ty {
                Ty::Int(_) => format!("wjRadix({v}, '{r}')"),
                Ty::F64 | Ty::F32 if r == 'e' => format!("wjExpFmt(float64({v}))"),
                _ => return self.unsupported("radix format on non-integer"),
            }
        } else {
            self.plain_display(&e.ty, &v, spec)?
        };
        let Some(w) = spec.width else { return Ok(base) };
        let fill = spec.fill.unwrap_or(if spec.zero_pad { '0' } else { ' ' });
        let align = spec.align.unwrap_or(if e.ty.is_numeric() { '>' } else { '<' });
        Ok(format!("wjPad({base}, {w}, {}, '{align}', {})", go_string(&fill.to_string()), spec.zero_pad))
    }

    fn plain_display(&mut self, t: &Ty, v: &str, spec: &FmtSpec) -> PResult<String> {
        Ok(match t {
            Ty::Int(_) => format!("wjItoa({v})"),
            Ty::F64 | Ty::F32 => match (spec.precision, t) {
                (Some(p), _) => format!("wjFixed(float64({v}), {p})"),
                (None, Ty::F32) => format!("wjF32({v})"),
                _ => format!("wjF64({v})"),
            },
            Ty::Bool => format!("wjBool({v})"),
            Ty::Str => match spec.precision {
                Some(p) => format!("wjTrunc({v}, {p})"),
                None => v.to_string(),
            },
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
