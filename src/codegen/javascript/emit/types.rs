//! Type-directed helpers: TypeScript annotations, cloning, key/equality classification.

use super::names::ident;
use super::Js;
use crate::codegen::portable::ty::Ty;

impl<'m> Js<'m> {
    /// TypeScript spelling of a type (`any` when not expressible).
    pub fn ts_type(&self, t: &Ty) -> String {
        match t {
            Ty::Int(k) => if k.is_wide() { "bigint".into() } else { "number".into() },
            Ty::F32 | Ty::F64 => "number".into(),
            Ty::Bool => "boolean".into(),
            Ty::Str | Ty::Char => "string".into(),
            Ty::Unit => "void".into(),
            Ty::Vec(e) | Ty::Iter(e) | Ty::Range(e) => format!("Array<{}>", self.ts_type(e)),
            Ty::Set(e) => format!("Set<{}>", self.ts_type(e)),
            Ty::Map(k, v) => format!("Map<{}, {}>", self.ts_type(k), self.ts_type(v)),
            Ty::Option(e) => format!("{{ v: {} }} | null", self.ts_type(e)),
            Ty::Result(a, b) => format!("{{ ok: true; v: {} }} | {{ ok: false; v: {} }}", self.ts_type(a), self.ts_type(b)),
            Ty::Tuple(ts) => format!("[{}]", ts.iter().map(|t| self.ts_type(t)).collect::<Vec<_>>().join(", ")),
            Ty::Adt(n, _) => ident(n),
            Ty::Fn(p, r) => format!("({}) => {}", p.iter().enumerate().map(|(i, t)| format!("a{i}: {}", self.ts_type(t))).collect::<Vec<_>>().join(", "), self.ts_type(r)),
            Ty::Param(n) => n.clone(),
            Ty::Var(_) | Ty::Unknown => "any".into(),
        }
    }

    pub fn is_struct(&self, name: &str) -> bool {
        self.m.structs.iter().any(|s| s.name == name)
    }

    /// JS expression of a function cloning a value of this type, or None for immutable values.
    pub fn cloner(&self, t: &Ty) -> Option<String> {
        match t {
            Ty::Adt(n, _) if self.is_struct(n) => Some("$cl".into()),
            Ty::Adt(..) => Some("$cl".into()),
            Ty::Vec(e) => Some(match self.cloner(e) {
                None => "((x) => x.slice())".into(),
                Some(c) => format!("((x) => x.map({c}))"),
            }),
            Ty::Set(_) => Some("((x) => new Set(x))".into()),
            Ty::Map(_, v) => Some(match self.cloner(v) {
                None => "((x) => new Map(x))".into(),
                Some(c) => format!("((x) => new Map(Array.from(x, ([k, v]) => [k, {c}(v)])))"),
            }),
            Ty::Tuple(ts) => {
                let parts: Vec<String> = ts.iter().enumerate().map(|(i, t)| self.cloner(t).map(|c| format!("{c}(x[{i}])")).unwrap_or(format!("x[{i}]"))).collect();
                Some(format!("((x) => [{}])", parts.join(", ")))
            }
            Ty::Option(e) => self.cloner(e).map(|c| format!("((x) => x === null ? null : {{ v: {c}(x.v) }})")),
            Ty::Result(a, b) => {
                let (ca, cb) = (self.cloner(a), self.cloner(b));
                if ca.is_none() && cb.is_none() {
                    return None;
                }
                let fa = ca.map(|c| format!("{c}(x.v)")).unwrap_or("x.v".into());
                let fb = cb.map(|c| format!("{c}(x.v)")).unwrap_or("x.v".into());
                Some(format!("((x) => x.ok ? {{ ok: true, v: {fa} }} : {{ ok: false, v: {fb} }})"))
            }
            Ty::Param(_) | Ty::Unknown | Ty::Var(_) => Some("$cl".into()),
            _ => None,
        }
    }

    pub fn clone_js(&self, t: &Ty, e: &str) -> String {
        match self.cloner(t) {
            None => e.to_string(),
            Some(c) if c == "$cl" => format!("$cl({e})"),
            Some(c) => format!("{c}({e})"),
        }
    }

    /// Primitive types compare with `===`/`<` directly.
    pub fn is_prim(&self, t: &Ty) -> bool {
        matches!(t, Ty::Int(_) | Ty::F32 | Ty::F64 | Ty::Bool | Ty::Str | Ty::Char | Ty::Unit)
    }

}
