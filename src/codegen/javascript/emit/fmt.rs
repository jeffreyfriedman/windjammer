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

    /// Debug formatter function expression for a type.
    pub fn dbg_fn(&mut self, t: &Ty) -> PResult<String> {
        Ok(match t {
            Ty::Int(_) | Ty::Bool => "String".into(),
            Ty::F64 => "$dbgF64".into(),
            Ty::F32 => "$dbgF32".into(),
            Ty::Str => "$dbgStr".into(),
            Ty::Char => "$dbgChar".into(),
            Ty::Unit => "(() => \"()\")".into(),
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

    /// Display (or Debug / numeric-radix per spec) of one argument.
    pub fn display(&mut self, e: &PExpr, spec: &FmtSpec) -> PResult<String> {
        if spec.pretty && !matches!(e.ty, Ty::Int(_) | Ty::F64 | Ty::F32 | Ty::Bool | Ty::Str | Ty::Char | Ty::Unit) {
            return self.unsupported("pretty Debug `{:#?}` of an aggregate (use `{:?}`)");
        }
        let v = self.expr(e)?;
        let base = if spec.debug {
            format!("{}({v})", self.dbg_fn(&e.ty)?)
        } else if let Some(radix) = spec.radix {
            match &e.ty {
                Ty::Int(k) => format!("$radix({v}, \"{radix}\", {})", k.bits()),
                Ty::F64 | Ty::F32 if radix == 'e' => format!("$expFmt({v})"),
                _ => return self.unsupported("radix format on non-integer"),
            }
        } else {
            self.plain_display(&e.ty, &v, spec)?
        };
        if spec.width.is_none() {
            return Ok(base);
        }
        let fill = spec.fill.unwrap_or(if spec.zero_pad { '0' } else { ' ' });
        let align = spec.align.unwrap_or(if e.ty.is_numeric() { '>' } else { '<' });
        Ok(format!("$pad({base}, {}, {}, \"{align}\", {})", spec.width.unwrap(), js_string(&fill.to_string()), spec.zero_pad))
    }

    fn plain_display(&mut self, t: &Ty, v: &str, spec: &FmtSpec) -> PResult<String> {
        Ok(match t {
            Ty::Int(_) => format!("String({v})"),
            Ty::F64 => match spec.precision {
                Some(p) => format!("$fixed({v}, {p})"),
                None => format!("$f64({v})"),
            },
            Ty::F32 => match spec.precision {
                Some(p) => format!("$fixed({v}, {p})"),
                None => format!("$f32({v})"),
            },
            Ty::Bool => format!("String({v})"),
            Ty::Str | Ty::Char => if let Some(p) = spec.precision { format!("$trunc({v}, {p})") } else { v.to_string() },
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
