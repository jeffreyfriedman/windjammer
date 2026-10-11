//! Pattern lowering: resolves enum variants, declares bindings with their types.

use super::{Lowerer, PResult};
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::Ty;
use crate::parser::{EnumPatternBinding, Pattern};

impl<'e> Lowerer<'e> {
    pub(super) fn pattern(&mut self, p: &Pattern, ty: &Ty) -> PResult<PPat> {
        let ty = self.resolve(ty);
        match p {
            Pattern::Wildcard => Ok(PPat::Wild),
            Pattern::Identifier(name) => self.ident_pattern(name, &ty),
            Pattern::MutBinding(name) | Pattern::Ref(name) | Pattern::RefMut(name) => {
                let out = self.declare(name, ty, false);
                Ok(PPat::Bind { name: out, mutable: true })
            }
            Pattern::Reference(inner) => self.pattern(inner, &ty),
            Pattern::Literal(l) => {
                let e = self.literal(l, Some(&ty))?;
                Ok(PPat::Lit(e))
            }
            Pattern::Tuple(ps) => {
                let tys = match &ty {
                    Ty::Tuple(ts) => ts.clone(),
                    _ => vec![Ty::Unknown; ps.len()],
                };
                let mut out = Vec::new();
                for (i, p) in ps.iter().enumerate() {
                    out.push(self.pattern(p, tys.get(i).unwrap_or(&Ty::Unknown))?);
                }
                Ok(PPat::Tuple(out))
            }
            Pattern::Or(ps) => {
                let mut out = Vec::new();
                for p in ps {
                    out.push(self.pattern(p, &ty)?);
                }
                Ok(PPat::Or(out))
            }
            Pattern::EnumVariant(name, binding) => self.variant_pattern(name, binding, &ty),
        }
    }

    fn ident_pattern(&mut self, name: &str, ty: &Ty) -> PResult<PPat> {
        if name == "None" {
            return Ok(PPat::None);
        }
        if let Ty::Adt(en, _) = ty {
            if self.env.variant(en, name).map(|v| v.fields.is_empty()).unwrap_or(false) {
                return Ok(PPat::Variant { enum_name: en.clone(), variant: name.to_string(), fields: vec![] });
            }
        }
        let out = self.declare(name, ty.clone(), false);
        Ok(PPat::Bind { name: out, mutable: false })
    }

    fn payload_pattern(&mut self, binding: &EnumPatternBinding, tys: &[Ty]) -> PResult<Vec<PPat>> {
        match binding {
            EnumPatternBinding::None | EnumPatternBinding::Wildcard => Ok(vec![]),
            EnumPatternBinding::Single(n) => {
                let ty = tys.first().cloned().unwrap_or(Ty::Unknown);
                let out = self.declare(n, ty, false);
                Ok(vec![PPat::Bind { name: out, mutable: false }])
            }
            EnumPatternBinding::Tuple(ps) => {
                let mut out = Vec::new();
                for (i, p) in ps.iter().enumerate() {
                    out.push(self.pattern(p, tys.get(i).unwrap_or(&Ty::Unknown))?);
                }
                Ok(out)
            }
            EnumPatternBinding::Struct(..) => self.err("struct pattern on built-in wrapper"),
        }
    }

    fn variant_pattern(&mut self, name: &str, binding: &EnumPatternBinding, ty: &Ty) -> PResult<PPat> {
        let first = |v: Vec<PPat>| v.into_iter().next().unwrap_or(PPat::Wild);
        match (name, ty) {
            ("None", _) => return Ok(PPat::None),
            ("Some", Ty::Option(t)) => return Ok(PPat::Some(Box::new(first(self.payload_pattern(binding, &[(**t).clone()])?)))),
            ("Ok", Ty::Result(t, _)) => return Ok(PPat::Ok(Box::new(first(self.payload_pattern(binding, &[(**t).clone()])?)))),
            ("Err", Ty::Result(_, e)) => return Ok(PPat::Err(Box::new(first(self.payload_pattern(binding, &[(**e).clone()])?)))),
            ("Some" | "Ok" | "Err", _) if ty.is_unknown() => {
                let inner = first(self.payload_pattern(binding, &[Ty::Unknown])?);
                return Ok(match name {
                    "Some" => PPat::Some(Box::new(inner)),
                    "Ok" => PPat::Ok(Box::new(inner)),
                    _ => PPat::Err(Box::new(inner)),
                });
            }
            _ => {}
        }
        let (en, var) = match name.rsplit_once("::") {
            Some((e, v)) => (if e == "Self" { self.owner.clone().unwrap_or_default() } else { e.to_string() }, v.to_string()),
            None => match ty {
                Ty::Adt(en, _) if self.env.variant(en, name).is_some() => (en.clone(), name.to_string()),
                _ => match self.env.enum_of_variant(name) {
                    Some(e) => (e.to_string(), name.to_string()),
                    None => return self.err(format!("unknown pattern `{name}`")),
                },
            },
        };
        let Some(info) = self.env.variant(&en, &var).cloned() else { return self.err(format!("unknown variant `{en}::{var}`")) };
        let einfo = self.env.enums.get(&en).cloned().unwrap();
        let args: Vec<Ty> = match ty {
            Ty::Adt(n, a) if *n == en => a.clone(),
            _ => vec![],
        };
        let ftys: Vec<Ty> = info.fields.iter().map(|(_, t)| t.subst(&einfo.type_params, &args)).collect();
        let fields = match binding {
            EnumPatternBinding::Struct(named, _) => {
                let mut out = Vec::new();
                for (fname, p) in named {
                    let idx = info.fields.iter().position(|(n, _)| n == fname);
                    let t = idx.map(|i| ftys[i].clone()).unwrap_or(Ty::Unknown);
                    out.push((fname.clone(), self.pattern(p, &t)?));
                }
                out
            }
            other => {
                let pats = self.payload_pattern(other, &ftys)?;
                pats.into_iter().enumerate().map(|(i, p)| (info.fields.get(i).map(|f| f.0.clone()).unwrap_or_else(|| i.to_string()), p)).collect()
            }
        };
        Ok(PPat::Variant { enum_name: en, variant: var, fields })
    }
}
