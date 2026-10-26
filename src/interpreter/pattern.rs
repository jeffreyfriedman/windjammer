//! Pattern matching with optional by-reference bindings (`&mut` scrutinees).

use super::engine::Interpreter;
use super::value::*;
use crate::parser::{EnumPatternBinding, Literal, Pattern};
use std::rc::Rc;

type Binds = Vec<(&'static str, Value)>;

impl Interpreter {
    /// Does `name` (as written in a pattern) denote this enum value's variant?
    fn variant_matches(&self, name: &str, e: &EnumVal) -> bool {
        // the common spelling: a bare variant name (`Some(v)`, `None`)
        if !name.bytes().any(|c| c == b'.' || c == b':') {
            return &*e.variant == name;
        }
        let normalized = name.replace('.', "::");
        let name = normalized.as_str();
        let (en, var) = match name.rsplit_once("::") {
            Some((en, v)) => (Some(en.rsplit("::").next().unwrap_or(en)), v),
            None => (None, name),
        };
        if &*e.variant != var {
            return false;
        }
        match en {
            Some("Self") | None => true,
            Some(t) => {
                *e.ty == *t
                    || self
                        .type_aliases
                        .get(t)
                        .map(|a| matches!(a, crate::parser::Type::Custom(n) if **n == *e.ty))
                        .unwrap_or(false)
            }
        }
    }

    fn literal_matches(&self, l: &Literal, v: &Value) -> bool {
        let lv = self.literal(l);
        match (&lv, v) {
            (Value::Int(a, _), Value::Int(b, _)) => a == b,
            (Value::Float(a, _), Value::Float(b, _)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            _ => false,
        }
    }

    /// Order of `v` relative to the literal bound `l` (`Some(Less)` means `v < l`), if comparable.
    fn literal_order(&self, l: &Literal, v: &Value) -> Option<std::cmp::Ordering> {
        match (&self.literal(l), v) {
            (Value::Int(a, _), Value::Int(b, _)) => Some(b.cmp(a)),
            (Value::Float(a, _), Value::Float(b, _)) => b.partial_cmp(a),
            (Value::Char(a), Value::Char(b)) => Some(b.cmp(a)),
            _ => None,
        }
    }

    /// Match `pat` against `v`. `place` is the location of `v` (when known) so that
    /// `by_ref` bindings can alias the matched parts.
    pub(crate) fn match_pattern(
        &mut self,
        pat: &'static Pattern<'static>,
        v: &Value,
        place: Option<&Place>,
        by_ref: bool,
        binds: &mut Binds,
    ) -> bool {
        let v = match v {
            Value::Ref(p) => match p.read() {
                Ok(inner) => {
                    return self.match_pattern(pat, &inner, Some(p), true, binds);
                }
                Err(_) => return false,
            },
            other => other,
        };
        let bind_val = |name: &'static str, binds: &mut Binds, v: &Value, place: Option<&Place>, by_ref: bool| {
            match (by_ref, place) {
                (true, Some(p)) if !matches!(v, Value::Unit) => binds.push((name, Value::Ref(Box::new(p.clone())))),
                _ => binds.push((name, v.clone())),
            }
        };
        match pat {
            Pattern::Wildcard => true,
            Pattern::Identifier(name) | Pattern::MutBinding(name) | Pattern::Ref(name) => {
                // A bare identifier that names a unit enum variant / const is a value pattern.
                // (a variant name never binds: a non-matching value must fail the pattern)
                if name == "None" || self.is_known_variant_name(name) {
                    return match v {
                        Value::Enum(e) => matches!(e.payload, Payload::Unit) && &*e.variant == name.as_str(),
                        _ => false,
                    };
                }
                bind_val(name.as_str(), binds, v, place, by_ref);
                true
            }
            Pattern::RefMut(name) => {
                match place {
                    Some(p) => binds.push((name.as_str(), Value::Ref(Box::new(p.clone())))),
                    None => binds.push((name.as_str(), v.clone())),
                }
                true
            }
            Pattern::Reference(inner) => self.match_pattern(inner, v, place, by_ref, binds),
            Pattern::Literal(l) => self.literal_matches(l, v),
            Pattern::Range { start, end, inclusive } => {
                use std::cmp::Ordering::*;
                let above = match start {
                    Some(l) => matches!(self.literal_order(l, v), Some(Greater | Equal)),
                    None => true,
                };
                let below = match end {
                    Some(l) => match self.literal_order(l, v) {
                        Some(Less) => true,
                        Some(Equal) => *inclusive,
                        _ => false,
                    },
                    None => true,
                };
                above && below
            }
            Pattern::Or(alts) => {
                for a in alts {
                    let mark = binds.len();
                    if self.match_pattern(a, v, place, by_ref, binds) {
                        return true;
                    }
                    binds.truncate(mark);
                }
                false
            }
            Pattern::Tuple(pats) => match v {
                Value::Tuple(items) if items.len() == pats.len() => {
                    for (i, (p, x)) in pats.iter().zip(items.iter()).enumerate() {
                        let sub = place.map(|pl| pl.proj(Proj::Index(i)));
                        if !self.match_pattern(p, x, sub.as_ref(), by_ref, binds) {
                            return false;
                        }
                    }
                    true
                }
                Value::Unit => pats.is_empty(),
                _ => false,
            },
            Pattern::EnumVariant(name, binding) => self.match_variant(name, binding, v, place, by_ref, binds),
        }
    }

    fn is_known_variant_name(&self, name: &str) -> bool {
        self.enums.values().any(|d| d.variants.iter().any(|v| v.name == name))
            && !self.structs.contains_key(name)
    }

    fn match_variant(
        &mut self,
        name: &'static str,
        binding: &'static EnumPatternBinding<'static>,
        v: &Value,
        place: Option<&Place>,
        by_ref: bool,
        binds: &mut Binds,
    ) -> bool {
        // struct / tuple-struct destructuring
        if let Value::Struct(s) = v {
            let sn = name.rsplit("::").next().unwrap_or(name);
            let sn = if sn == "Self" { self.scope.self_ty.as_deref().unwrap_or(sn) } else { sn };
            if &*s.name != sn {
                return false;
            }
            return match binding {
                EnumPatternBinding::None | EnumPatternBinding::Wildcard => true,
                EnumPatternBinding::Single(b) => {
                    if let Some((_, fv)) = s.fields.first() {
                        let sub = place.map(|p| p.proj(Proj::Field(s.fields[0].0.clone())));
                        match (by_ref, &sub) {
                            (true, Some(sp)) => binds.push((b.as_str(), Value::Ref(Box::new(sp.clone())))),
                            _ => binds.push((b.as_str(), fv.clone())),
                        }
                        true
                    } else {
                        false
                    }
                }
                EnumPatternBinding::Tuple(ps) => {
                    for (i, p) in ps.iter().enumerate() {
                        let Some((fname, fv)) = s.fields.get(i) else { return false };
                        let sub = place.map(|pl| pl.proj(Proj::Field(fname.clone())));
                        if !self.match_pattern(p, fv, sub.as_ref(), by_ref, binds) {
                            return false;
                        }
                    }
                    true
                }
                EnumPatternBinding::Struct(fps, _) => {
                    for (fname, p) in fps {
                        let Some((n, fv)) = s.fields.iter().find(|(n, _)| &**n == fname.as_str()) else { return false };
                        let sub = place.map(|pl| pl.proj(Proj::Field(n.clone())));
                        if !self.match_pattern(p, fv, sub.as_ref(), by_ref, binds) {
                            return false;
                        }
                    }
                    true
                }
            };
        }
        let Value::Enum(e) = v else { return false };
        let e: Rc<EnumVal> = e.clone();
        if !self.variant_matches(name, &e) {
            return false;
        }
        match (binding, &e.payload) {
            (EnumPatternBinding::None, _) | (EnumPatternBinding::Wildcard, _) => true,
            (EnumPatternBinding::Single(b), Payload::Tuple(vs)) => {
                if vs.len() != 1 {
                    // Single binding to a multi-field variant binds nothing sensible
                    return false;
                }
                let sub = place.map(|p| p.proj(Proj::Index(0)));
                match (by_ref, &sub) {
                    (true, Some(sp)) => binds.push((b.as_str(), Value::Ref(Box::new(sp.clone())))),
                    _ => binds.push((b.as_str(), vs[0].clone())),
                }
                true
            }
            (EnumPatternBinding::Tuple(ps), Payload::Tuple(vs)) => {
                if ps.len() != vs.len() {
                    return false;
                }
                for (i, (p, x)) in ps.iter().zip(vs.iter()).enumerate() {
                    let sub = place.map(|pl| pl.proj(Proj::Index(i)));
                    if !self.match_pattern(p, x, sub.as_ref(), by_ref, binds) {
                        return false;
                    }
                }
                true
            }
            (EnumPatternBinding::Struct(fps, _), Payload::Struct(fs)) => {
                for (fname, p) in fps {
                    let Some((n, fv)) = fs.iter().find(|(n, _)| &**n == fname.as_str()) else { return false };
                    let sub = place.map(|pl| pl.proj(Proj::Field(n.clone())));
                    if !self.match_pattern(p, fv, sub.as_ref(), by_ref, binds) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }
}
