//! Literal-versus-declared-type mismatches (the obvious `E0308` cases, caught without rustc).

use super::index::Index;
use crate::diagnostics::{Applicability, Diagnostic, Edit, Fix, Span};
use crate::parser::{Literal, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Int,
    Float,
    Bool,
    Str,
    Char,
}

fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Int => "int",
        Kind::Float => "float",
        Kind::Bool => "bool",
        Kind::Str => "string",
        Kind::Char => "char",
    }
}

fn literal_kind(l: &Literal) -> Kind {
    match l {
        Literal::Int(_) | Literal::IntSuffixed(..) => Kind::Int,
        Literal::Float(_) => Kind::Float,
        Literal::Bool(_) => Kind::Bool,
        Literal::String(_) => Kind::Str,
        Literal::Char(_) => Kind::Char,
    }
}

/// What the declared type demands of a literal: a scalar kind or a user struct.
enum Expect {
    Scalar(Kind),
    Struct(String),
}

fn expectation(ty: &Type, idx: &Index) -> Option<Expect> {
    match ty {
        Type::Int | Type::Int32 | Type::Uint => Some(Expect::Scalar(Kind::Int)),
        Type::Float => Some(Expect::Scalar(Kind::Float)),
        Type::Bool => Some(Expect::Scalar(Kind::Bool)),
        Type::String => Some(Expect::Scalar(Kind::Str)),
        Type::Reference(inner) => match **inner {
            Type::String => Some(Expect::Scalar(Kind::Str)),
            _ => None,
        },
        Type::Custom(n) => match n.as_str() {
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
            | "u128" | "usize" => Some(Expect::Scalar(Kind::Int)),
            "f32" | "f64" => Some(Expect::Scalar(Kind::Float)),
            "bool" => Some(Expect::Scalar(Kind::Bool)),
            "char" => Some(Expect::Scalar(Kind::Char)),
            "str" | "String" | "string" => Some(Expect::Scalar(Kind::Str)),
            other => match idx.structs.get(other) {
                Some(v) if v.len() == 1 && !v[0].generic => Some(Expect::Struct(other.to_string())),
                _ => None,
            },
        },
        _ => None,
    }
}

/// A name for the scalar or user-struct type `t` (what an element type is called in a message), if it has one.
fn type_label(t: &Type, idx: &Index) -> Option<String> {
    match expectation(t, idx)? {
        Expect::Scalar(k) => Some(kind_name(k).to_string()),
        Expect::Struct(n) => Some(n),
    }
}

/// `Some(diagnostic)` when `lit` can never be a value of `declared`. `span` covers the literal.
pub fn check_literal(
    lit: &Literal,
    declared: &Type,
    idx: &Index,
    span: Option<Span>,
    context: &str,
) -> Option<Diagnostic> {
    let found = literal_kind(lit);
    // A bare literal is never an `Option<T>` (there is no implicit `Some`) and never a `Vec<T>`.
    let container = match declared {
        Type::Option(inner) => type_label(inner, idx).map(|n| format!("Option<{n}>")),
        Type::Vec(inner) | Type::Array(inner, _) => type_label(inner, idx).map(|n| format!("Vec<{n}>")),
        _ => None,
    };
    if let Some(expected_name) = container {
        let mut d = Diagnostic::error("WJ0003", format!("mismatched types: expected `{expected_name}`, found `{}`{context}", kind_name(found)));
        if let Some(mut sp) = span {
            sp.label = Some(format!("expected `{expected_name}`, found `{}`", kind_name(found)));
            d = d.at(sp);
        }
        let help = if expected_name.starts_with("Option") { "wrap the value in `Some(..)`, or write `None`" } else { "write a list: `[..]`" };
        return Some(d.with_help(help));
    }
    let (expected_name, fix_kind) = match expectation(declared, idx)? {
        Expect::Scalar(k) => {
            // An integer literal is acceptable where a float is declared.
            if k == found || (k == Kind::Float && found == Kind::Int) {
                return None;
            }
            (kind_name(k).to_string(), Some(k))
        }
        Expect::Struct(n) => (n, None),
    };
    let mut d = Diagnostic::error(
        "WJ0003",
        format!(
            "mismatched types: expected `{expected_name}`, found `{}`{context}",
            kind_name(found)
        ),
    );
    if let Some(mut sp) = span {
        sp.label = Some(format!("expected `{expected_name}`, found `{}`", kind_name(found)));
        d = d.at(sp.clone());
        if let Some(fix) = convert_fix(lit, fix_kind, &sp) {
            d = d.with_fix(fix);
        }
    }
    Some(d.with_help(format!(
        "make the value match `{expected_name}`, or change the declared type"
    )))
}

fn convert_fix(lit: &Literal, expected: Option<Kind>, span: &Span) -> Option<Fix> {
    let edit = |replacement: String, description: String| Fix {
        description,
        applicability: Applicability::MaybeIncorrect,
        edits: vec![Edit {
            span: Span { label: None, ..span.clone() },
            replacement,
        }],
    };
    match (expected?, lit) {
        (Kind::Str, Literal::Int(n)) => Some(edit(format!("\"{n}\""), "quote the number to make it a string".into())),
        (Kind::Str, Literal::Bool(b)) => Some(edit(format!("\"{b}\""), "quote the value to make it a string".into())),
        (Kind::Int, Literal::String(s)) if s.trim().parse::<i64>().is_ok() => {
            Some(edit(s.trim().to_string(), "use the number instead of the string".into()))
        }
        (Kind::Float, Literal::String(s)) if s.trim().parse::<f64>().is_ok() => {
            Some(edit(s.trim().to_string(), "use the number instead of the string".into()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx() -> Index {
        Index::default()
    }

    #[test]
    fn bare_literal_is_not_an_option_or_a_vec() {
        let o = check_literal(&Literal::String("open".into()), &Type::Option(Box::new(Type::String)), &idx(), None, "").unwrap();
        assert!(o.message.contains("expected `Option<string>`, found `string`"), "{}", o.message);
        let v = check_literal(&Literal::String("bug".into()), &Type::Vec(Box::new(Type::String)), &idx(), None, "").unwrap();
        assert!(v.message.contains("expected `Vec<string>`"), "{}", v.message);
        assert!(check_literal(&Literal::Int(1), &Type::Option(Box::new(Type::Custom("Mystery".into()))), &idx(), None, "").is_none(), "unknown element types stay unchecked");
    }

    #[test]
    fn string_into_int_is_flagged() {
        let d = check_literal(&Literal::String("hello".into()), &Type::Int, &idx(), None, "").unwrap();
        assert_eq!(d.code, "WJ0003");
        assert!(d.message.contains("expected `int`, found `string`"));
    }

    #[test]
    fn int_into_float_is_allowed() {
        assert!(check_literal(&Literal::Int(1), &Type::Float, &idx(), None, "").is_none());
    }

    #[test]
    fn matching_kinds_pass() {
        assert!(check_literal(&Literal::Bool(true), &Type::Bool, &idx(), None, "").is_none());
        assert!(check_literal(&Literal::String("a".into()), &Type::String, &idx(), None, "").is_none());
    }

    #[test]
    fn unknown_declared_types_are_not_judged() {
        let t = Type::Custom("Mystery".into());
        assert!(check_literal(&Literal::Int(1), &t, &idx(), None, "").is_none());
    }
}
