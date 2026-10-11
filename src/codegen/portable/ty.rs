//! Resolved types for the portable (Go/JS/TS) backends.
//!
//! The parser's `Type` collapses `i8..i64`/`u8..u64` into `Custom(..)`; this module resolves them
//! into exact widths so emitters can implement wrapping, division and formatting correctly.

use crate::parser::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntKind {
    I8,
    I16,
    I32,
    I64,
    Isize,
    U8,
    U16,
    U32,
    U64,
    Usize,
}

impl IntKind {
    pub fn bits(self) -> u32 {
        match self {
            IntKind::I8 | IntKind::U8 => 8,
            IntKind::I16 | IntKind::U16 => 16,
            IntKind::I32 | IntKind::U32 => 32,
            _ => 64,
        }
    }

    pub fn signed(self) -> bool {
        matches!(self, IntKind::I8 | IntKind::I16 | IntKind::I32 | IntKind::I64 | IntKind::Isize)
    }

    /// 64-bit kinds exceed f64's exact range; the JS backend represents them as BigInt.
    pub fn is_wide(self) -> bool {
        self.bits() == 64
    }

    pub fn name(self) -> &'static str {
        match self {
            IntKind::I8 => "i8",
            IntKind::I16 => "i16",
            IntKind::I32 => "i32",
            IntKind::I64 => "i64",
            IntKind::Isize => "isize",
            IntKind::U8 => "u8",
            IntKind::U16 => "u16",
            IntKind::U32 => "u32",
            IntKind::U64 => "u64",
            IntKind::Usize => "usize",
        }
    }

    pub fn from_name(n: &str) -> Option<IntKind> {
        Some(match n {
            "i8" => IntKind::I8,
            "i16" => IntKind::I16,
            "i32" | "int32" => IntKind::I32,
            "i64" | "int" => IntKind::I64,
            "isize" => IntKind::Isize,
            "u8" => IntKind::U8,
            "u16" => IntKind::U16,
            "u32" => IntKind::U32,
            "u64" | "uint" => IntKind::U64,
            "usize" => IntKind::Usize,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Int(IntKind),
    F32,
    F64,
    Bool,
    Str,
    Char,
    Unit,
    Vec(Box<Ty>),
    Map(Box<Ty>, Box<Ty>),
    Set(Box<Ty>),
    Option(Box<Ty>),
    Result(Box<Ty>, Box<Ty>),
    Tuple(Vec<Ty>),
    /// Struct or enum declared in the program, with type arguments.
    Adt(String, Vec<Ty>),
    Fn(Vec<Ty>, Box<Ty>),
    /// Eagerly materialised iterator over elements.
    Iter(Box<Ty>),
    Range(Box<Ty>),
    Param(String),
    /// Inference variable (resolved before emission).
    Var(u32),
    /// Not yet constrained (integer literal defaults to i64 when still unknown).
    Unknown,
}

impl Ty {
    pub fn i64() -> Ty {
        Ty::Int(IntKind::I64)
    }

    pub fn usize() -> Ty {
        Ty::Int(IntKind::Usize)
    }

    pub fn is_int(&self) -> bool {
        matches!(self, Ty::Int(_))
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Ty::F32 | Ty::F64)
    }

    pub fn is_numeric(&self) -> bool {
        self.is_int() || self.is_float()
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Ty::Unknown | Ty::Var(_))
    }

    pub fn int_kind(&self) -> Option<IntKind> {
        if let Ty::Int(k) = self { Some(*k) } else { None }
    }

    /// Element type for Vec/Iter/Set/Range-like containers.
    pub fn elem(&self) -> Option<Ty> {
        match self {
            Ty::Vec(t) | Ty::Iter(t) | Ty::Set(t) | Ty::Range(t) => Some((**t).clone()),
            Ty::Str => Some(Ty::Char),
            _ => None,
        }
    }

    /// Stable, identifier-safe spelling (used for instantiation names and diagnostics).
    pub fn describe(&self) -> String {
        match self {
            Ty::Int(k) => k.name().to_string(),
            Ty::F32 => "f32".into(),
            Ty::F64 => "f64".into(),
            Ty::Bool => "bool".into(),
            Ty::Str => "string".into(),
            Ty::Char => "char".into(),
            Ty::Unit => "()".into(),
            Ty::Vec(t) => format!("Vec<{}>", t.describe()),
            Ty::Map(k, v) => format!("HashMap<{}, {}>", k.describe(), v.describe()),
            Ty::Set(t) => format!("HashSet<{}>", t.describe()),
            Ty::Option(t) => format!("Option<{}>", t.describe()),
            Ty::Result(a, b) => format!("Result<{}, {}>", a.describe(), b.describe()),
            Ty::Tuple(ts) => format!("({})", ts.iter().map(|t| t.describe()).collect::<Vec<_>>().join(", ")),
            Ty::Adt(n, a) if a.is_empty() => n.clone(),
            Ty::Adt(n, a) => format!("{}<{}>", n, a.iter().map(|t| t.describe()).collect::<Vec<_>>().join(", ")),
            Ty::Fn(p, r) => format!("fn({}) -> {}", p.iter().map(|t| t.describe()).collect::<Vec<_>>().join(", "), r.describe()),
            Ty::Iter(t) => format!("Iter<{}>", t.describe()),
            Ty::Range(t) => format!("Range<{}>", t.describe()),
            Ty::Param(n) => n.clone(),
            Ty::Var(_) => "_".into(),
            Ty::Unknown => "_".into(),
        }
    }

    /// Replace `Param(name)` by the corresponding argument.
    pub fn subst(&self, params: &[String], args: &[Ty]) -> Ty {
        let rec = |t: &Ty| t.subst(params, args);
        match self {
            Ty::Param(n) => params.iter().position(|p| p == n).and_then(|i| args.get(i).cloned()).unwrap_or_else(|| self.clone()),
            Ty::Vec(t) => Ty::Vec(Box::new(rec(t))),
            Ty::Set(t) => Ty::Set(Box::new(rec(t))),
            Ty::Iter(t) => Ty::Iter(Box::new(rec(t))),
            Ty::Range(t) => Ty::Range(Box::new(rec(t))),
            Ty::Option(t) => Ty::Option(Box::new(rec(t))),
            Ty::Map(k, v) => Ty::Map(Box::new(rec(k)), Box::new(rec(v))),
            Ty::Result(a, b) => Ty::Result(Box::new(rec(a)), Box::new(rec(b))),
            Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(rec).collect()),
            Ty::Adt(n, a) => Ty::Adt(n.clone(), a.iter().map(rec).collect()),
            Ty::Fn(p, r) => Ty::Fn(p.iter().map(rec).collect(), Box::new(rec(r))),
            _ => self.clone(),
        }
    }

    /// Replace the pseudo-type `Self` by the implementing type.
    pub fn replace_self(&self, self_ty: &Ty) -> Ty {
        let rec = |t: &Ty| t.replace_self(self_ty);
        match self {
            Ty::Adt(n, _) if n == "Self" => self_ty.clone(),
            Ty::Vec(t) => Ty::Vec(Box::new(rec(t))),
            Ty::Set(t) => Ty::Set(Box::new(rec(t))),
            Ty::Iter(t) => Ty::Iter(Box::new(rec(t))),
            Ty::Range(t) => Ty::Range(Box::new(rec(t))),
            Ty::Option(t) => Ty::Option(Box::new(rec(t))),
            Ty::Map(k, v) => Ty::Map(Box::new(rec(k)), Box::new(rec(v))),
            Ty::Result(a, b) => Ty::Result(Box::new(rec(a)), Box::new(rec(b))),
            Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(rec).collect()),
            Ty::Adt(n, a) => Ty::Adt(n.clone(), a.iter().map(rec).collect()),
            Ty::Fn(p, r) => Ty::Fn(p.iter().map(rec).collect(), Box::new(rec(r))),
            _ => self.clone(),
        }
    }

    /// Unify two types for inference: `Unknown` yields to the other side.
    pub fn join(&self, other: &Ty) -> Ty {
        if self.is_unknown() { other.clone() } else { self.clone() }
    }
}

/// Resolve a parser `Type`. `generics` lists type-parameter names in scope.
pub fn from_ast(t: &Type, generics: &[String]) -> Ty {
    let rec = |t: &Type| from_ast(t, generics);
    match t {
        Type::Int => Ty::Int(IntKind::I64),
        Type::Int32 => Ty::Int(IntKind::I32),
        Type::Uint => Ty::Int(IntKind::U64),
        Type::Float => Ty::F64,
        Type::Bool => Ty::Bool,
        Type::String => Ty::Str,
        Type::Custom(n) => from_name(n, generics),
        Type::Generic(n) => Ty::Param(n.clone()),
        Type::Parameterized(n, args) => from_parameterized(n, args, generics),
        Type::Option(t) => Ty::Option(Box::new(rec(t))),
        Type::Result(a, b) => Ty::Result(Box::new(rec(a)), Box::new(rec(b))),
        Type::Vec(t) => Ty::Vec(Box::new(rec(t))),
        Type::Array(t, _) => Ty::Vec(Box::new(rec(t))),
        Type::Reference(t) | Type::MutableReference(t) => rec(t),
        Type::Tuple(ts) if ts.is_empty() => Ty::Unit,
        Type::Tuple(ts) => Ty::Tuple(ts.iter().map(rec).collect()),
        Type::FunctionPointer { params, return_type } => Ty::Fn(
            params.iter().map(rec).collect(),
            Box::new(return_type.as_ref().map(|t| rec(t)).unwrap_or(Ty::Unit)),
        ),
        Type::Infer => Ty::Unknown,
        Type::TraitObject(n) | Type::ImplTrait(n) => Ty::Adt(n.clone(), vec![]),
        Type::Associated(..) | Type::RawPointer { .. } => Ty::Unknown,
    }
}

fn from_name(n: &str, generics: &[String]) -> Ty {
    if let Some(k) = IntKind::from_name(n) {
        return Ty::Int(k);
    }
    match n {
        "f64" | "float" => Ty::F64,
        "f32" => Ty::F32,
        "bool" => Ty::Bool,
        "string" | "String" | "str" => Ty::Str,
        "char" => Ty::Char,
        "()" | "unit" => Ty::Unit,
        _ if generics.iter().any(|g| g == n) => Ty::Param(n.to_string()),
        _ => Ty::Adt(n.to_string(), vec![]),
    }
}

fn from_parameterized(n: &str, args: &[Type], generics: &[String]) -> Ty {
    let a: Vec<Ty> = args.iter().map(|t| from_ast(t, generics)).collect();
    let arg = |i: usize| Box::new(a.get(i).cloned().unwrap_or(Ty::Unknown));
    match n {
        "Vec" => Ty::Vec(arg(0)),
        "HashMap" | "BTreeMap" | "Map" => Ty::Map(arg(0), arg(1)),
        "HashSet" | "BTreeSet" | "Set" => Ty::Set(arg(0)),
        "Option" => Ty::Option(arg(0)),
        "Result" => Ty::Result(arg(0), arg(1)),
        "Box" | "Rc" | "Arc" => *arg(0),
        _ => Ty::Adt(n.to_string(), a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_int_names_resolve_to_widths() {
        assert_eq!(from_ast(&Type::Custom("u8".into()), &[]), Ty::Int(IntKind::U8));
        assert_eq!(from_ast(&Type::Custom("usize".into()), &[]), Ty::Int(IntKind::Usize));
        assert_eq!(from_ast(&Type::Int32, &[]), Ty::Int(IntKind::I32));
        assert_eq!(from_ast(&Type::Int, &[]), Ty::Int(IntKind::I64));
    }

    #[test]
    fn wide_ints_are_64_bit() {
        assert!(IntKind::I64.is_wide() && IntKind::Usize.is_wide());
        assert!(!IntKind::I32.is_wide());
    }

    #[test]
    fn subst_replaces_params() {
        let t = Ty::Vec(Box::new(Ty::Param("T".into())));
        assert_eq!(t.subst(&["T".into()], &[Ty::Bool]), Ty::Vec(Box::new(Ty::Bool)));
    }
}
