//! Go type spelling and type-directed helpers.

use super::names::ident;
use super::Go;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::{IntKind, Ty};

pub fn int_name(k: IntKind) -> &'static str {
    match k {
        IntKind::I8 => "int8",
        IntKind::I16 => "int16",
        IntKind::I32 => "int32",
        IntKind::I64 => "int64",
        IntKind::Isize => "int",
        IntKind::U8 => "uint8",
        IntKind::U16 => "uint16",
        IntKind::U32 => "uint32",
        IntKind::U64 => "uint64",
        IntKind::Usize => "uint",
    }
}

impl<'m> Go<'m> {
    pub fn is_enum(&self, n: &str) -> bool {
        self.m.enums.iter().any(|e| e.name == n)
    }

    pub fn is_struct(&self, n: &str) -> bool {
        self.m.structs.iter().any(|s| s.name == n)
    }

    pub fn is_trait(&self, n: &str) -> bool {
        self.m.traits.iter().any(|t| t.name == n)
    }

    /// Go spelling of a type; errors on types that were never inferred.
    pub fn ty(&self, t: &Ty) -> PResult<String> {
        Ok(match t {
            Ty::Int(k) => int_name(*k).into(),
            Ty::F64 => "float64".into(),
            Ty::F32 => "float32".into(),
            Ty::Bool => "bool".into(),
            Ty::Str => "string".into(),
            Ty::Char => "Char".into(),
            Ty::Unit => "struct{}".into(),
            Ty::Vec(e) | Ty::Iter(e) | Ty::Range(e) => format!("[]{}", self.ty(e)?),
            Ty::Map(k, v) => format!("map[{}]{}", self.ty(k)?, self.ty(v)?),
            Ty::Set(e) => format!("map[{}]struct{{}}", self.ty(e)?),
            Ty::Option(e) => format!("Option[{}]", self.ty(e)?),
            Ty::Result(a, b) => format!("Result[{}, {}]", self.ty(a)?, self.ty(b)?),
            Ty::Tuple(ts) => {
                let parts: PResult<Vec<String>> = ts.iter().map(|t| self.ty(t)).collect();
                format!("Tuple{}[{}]", ts.len(), parts?.join(", "))
            }
            Ty::Adt(n, args) => {
                if args.is_empty() {
                    ident(n)
                } else {
                    let parts: PResult<Vec<String>> = args.iter().map(|t| self.ty(t)).collect();
                    format!("{}[{}]", ident(n), parts?.join(", "))
                }
            }
            Ty::Fn(ps, r) => {
                let params: PResult<Vec<String>> = ps.iter().map(|t| self.ty(t)).collect();
                let ret = if **r == Ty::Unit { String::new() } else { format!(" {}", self.ty(r)?) };
                format!("func({}){ret}", params?.join(", "))
            }
            Ty::Param(n) => n.clone(),
            Ty::Var(_) | Ty::Unknown => return Err(Unsupported { what: "a type could not be inferred (add a type annotation)".into(), line: 0 }),
        })
    }

    /// Is the type a Go reference type (aliasing already matches Rust `&mut` semantics)?
    pub fn is_ref_type(&self, t: &Ty) -> bool {
        matches!(t, Ty::Map(..) | Ty::Set(_))
    }

    pub fn is_prim(&self, t: &Ty) -> bool {
        matches!(t, Ty::Int(_) | Ty::F32 | Ty::F64 | Ty::Bool | Ty::Str | Ty::Char)
    }

    /// Does a copy of this type need more than Go's assignment copy?
    pub fn needs_deep_clone(&self, t: &Ty) -> bool {
        match t {
            Ty::Vec(_) | Ty::Map(..) | Ty::Set(_) => true,
            Ty::Option(e) => self.needs_deep_clone(e),
            Ty::Result(a, b) => self.needs_deep_clone(a) || self.needs_deep_clone(b),
            Ty::Tuple(ts) => ts.iter().any(|t| self.needs_deep_clone(t)),
            Ty::Adt(n, _) => self.m.structs.iter().any(|s| s.name == *n && s.fields.iter().any(|(_, ft)| self.needs_deep_clone(ft))) || self.is_enum(n) || self.is_trait(n),
            Ty::Param(_) => true,
            _ => false,
        }
    }

    pub fn clone_go(&self, t: &Ty, e: &str) -> PResult<String> {
        Ok(if self.needs_deep_clone(t) { format!("wjClone({e})") } else { e.to_string() })
    }
}
