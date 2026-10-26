//! Structural rules that need no expression types: missing trait items (WJ0018), match
//! exhaustiveness over a user enum (WJ0010), operand kinds (WJ0020) and `${x}` on a type without
//! `Display` (WJ0017). Each is conservative: anything it cannot classify is accepted.

use super::index::Index;
use crate::diagnostics::Diagnostic;
use crate::parser::{BinaryOp, EnumPatternBinding, ImplBlock, Literal, MatchArm, Pattern};

/// Required methods of a project trait that the impl does not define.
pub fn missing_trait_items(idx: &Index, block: &ImplBlock<'_>) -> Option<(String, Vec<String>)> {
    let tr = block.trait_name.as_deref()?;
    let tr = tr.rsplit("::").next().unwrap_or(tr);
    let req = idx.required.get(tr)?;
    let missing: Vec<String> = req
        .iter()
        .filter(|m| !block.functions.iter().any(|f| &f.name == *m))
        .cloned()
        .collect();
    (!missing.is_empty()).then(|| (tr.to_string(), missing))
}

fn irrefutable(p: &Pattern<'_>) -> bool {
    match p {
        Pattern::Wildcard | Pattern::Identifier(_) | Pattern::Ref(_) | Pattern::RefMut(_) | Pattern::MutBinding(_) => true,
        Pattern::Tuple(ps) => ps.iter().all(irrefutable),
        Pattern::Reference(q) => irrefutable(q),
        _ => false,
    }
}

fn binding_irrefutable(b: &EnumPatternBinding<'_>) -> bool {
    match b {
        EnumPatternBinding::None | EnumPatternBinding::Wildcard | EnumPatternBinding::Single(_) => true,
        EnumPatternBinding::Tuple(ps) => ps.iter().all(irrefutable),
        EnumPatternBinding::Struct(fs, _) => fs.iter().all(|(_, p)| irrefutable(p)),
    }
}

/// `(enum, missing variants)` when every guard-free arm names a variant of one user enum and
/// some variants have no arm. `None` whenever anything is not understood.
pub fn non_exhaustive(idx: &Index, arms: &[MatchArm<'_>]) -> Option<(String, Vec<String>)> {
    let mut enum_name: Option<String> = None;
    let mut covered: Vec<String> = Vec::new();
    for arm in arms {
        let pats: Vec<&Pattern<'_>> = match &arm.pattern {
            Pattern::Or(ps) => ps.iter().collect(),
            p => vec![p],
        };
        for p in pats {
            match p {
                p if irrefutable(p) && arm.guard.is_none() => return None,
                Pattern::EnumVariant(path, binding) => {
                    let (e, v) = path.rsplit_once("::")?;
                    let e = e.rsplit("::").next().unwrap_or(e);
                    if enum_name.get_or_insert_with(|| e.to_string()) != e {
                        return None;
                    }
                    if arm.guard.is_none() && binding_irrefutable(binding) {
                        covered.push(v.to_string());
                    }
                }
                _ => return None,
            }
        }
    }
    let e = enum_name?;
    let all = idx.variants.get(&e)?;
    let missing: Vec<String> = all.iter().filter(|v| !covered.contains(v)).cloned().collect();
    (!missing.is_empty()).then_some((e, missing))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Int,
    Float,
    Str,
    Bool,
}

/// ADR-0002 migration switch: today the compiler infers `mut` for locals, so assigning to a
/// binding declared without `mut` is reported as a warning (WJ0004). When the ADR-0002 migration
/// makes explicit `let mut` mandatory, flip this to `true` to make it an error.
pub const MUTABILITY_REQUIRED: bool = false;

/// Scalar kind of a declared type, if it is one of the four scalars.
pub fn type_kind(t: &crate::parser::Type) -> Option<Kind> {
    use crate::parser::Type;
    match t {
        Type::Int | Type::Int32 | Type::Uint => Some(Kind::Int),
        Type::Float => Some(Kind::Float),
        Type::Bool => Some(Kind::Bool),
        Type::String => Some(Kind::Str),
        Type::Custom(n) => match n.as_str() {
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => Some(Kind::Int),
            "f32" | "f64" => Some(Kind::Float),
            "bool" => Some(Kind::Bool),
            "String" | "str" | "string" => Some(Kind::Str),
            _ => None,
        },
        _ => None,
    }
}

pub fn display_kind(k: Kind) -> &'static str {
    kind_name(k)
}

pub fn literal_kind(l: &Literal) -> Option<Kind> {
    match l {
        Literal::Int(_) | Literal::IntSuffixed(..) => Some(Kind::Int),
        Literal::Float(_) => Some(Kind::Float),
        Literal::String(_) => Some(Kind::Str),
        Literal::Bool(_) => Some(Kind::Bool),
        _ => None,
    }
}

fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Int => "int",
        Kind::Float => "float",
        Kind::Str => "string",
        Kind::Bool => "bool",
    }
}

fn op_text(op: BinaryOp) -> Option<&'static str> {
    Some(match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        _ => return None,
    })
}

/// Arithmetic on a string (other than `+`), or int mixed with float. `strict` is true when both
/// kinds come from declared types (literals may be coerced, so int/float literals are not judged).
pub fn bad_operands(op: BinaryOp, l: Option<Kind>, r: Option<Kind>, strict: bool) -> Option<Diagnostic> {
    let sym = op_text(op)?;
    let (l, r) = (l?, r?);
    if l == Kind::Bool || r == Kind::Bool {
        return None;
    }
    let numeric = |k: Kind| matches!(k, Kind::Int | Kind::Float);
    let string_arith = (op != BinaryOp::Add && (l == Kind::Str || r == Kind::Str))
        || (op == BinaryOp::Add && ((l == Kind::Str && numeric(r)) || (r == Kind::Str && numeric(l))));
    let mixed = strict && matches!((l, r), (Kind::Int, Kind::Float) | (Kind::Float, Kind::Int));
    if !(string_arith || mixed) {
        return None;
    }
    Some(
        Diagnostic::error(
            "WJ0020",
            format!("operator `{sym}` cannot be applied to `{}` and `{}`", kind_name(l), kind_name(r)),
        )
        .with_help(if string_arith {
            "parse the string first, for example `s.parse::<int>()`".to_string()
        } else {
            "convert one operand explicitly, for example `a as float`".to_string()
        }),
    )
}

/// Placeholders of a lowered interpolation format string: `true` for `{}` (Display), `false` for
/// any `{:..}` spec.
pub fn placeholders(fmt: &str) -> Vec<bool> {
    let mut out = Vec::new();
    let b: Vec<char> = fmt.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '{' {
            if b.get(i + 1) == Some(&'{') {
                i += 2;
                continue;
            }
            let close = b[i..].iter().position(|c| *c == '}').map(|p| i + p).unwrap_or(b.len());
            out.push(close == i + 1);
            i = close + 1;
        } else if b[i] == '}' && b.get(i + 1) == Some(&'}') {
            i += 2;
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_specs() {
        assert_eq!(placeholders("a {} b {:?} {{x}} {:.2}"), vec![true, false, false]);
    }

    #[test]
    fn operand_rules() {
        assert!(bad_operands(BinaryOp::Sub, Some(Kind::Str), Some(Kind::Int), false).is_some());
        assert!(bad_operands(BinaryOp::Add, Some(Kind::Str), Some(Kind::Str), false).is_none());
        assert!(bad_operands(BinaryOp::Add, Some(Kind::Int), Some(Kind::Float), false).is_none());
        assert!(bad_operands(BinaryOp::Add, Some(Kind::Int), Some(Kind::Float), true).is_some());
    }
}
