//! Typed, statement-oriented IR shared by the Go and JS/TS emitters.
//!
//! Invariants: every expression carries its resolved `Ty`; value-semantics copies are explicit
//! (`PK::Copy`); `?`, interpolation, ranges and patterns are already desugared enough that an
//! emitter only maps to target syntax.

use super::ty::{IntKind, Ty};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamMode {
    /// Callee owns (or copies) the value.
    Value,
    /// Read-only borrow (aliasing is unobservable).
    Ref,
    /// Callee mutates the caller's value in place.
    MutRef,
}

#[derive(Debug, Clone)]
pub struct PModule {
    pub structs: Vec<PStruct>,
    pub enums: Vec<PEnum>,
    pub traits: Vec<PTrait>,
    pub fns: Vec<PFn>,
    pub consts: Vec<PConst>,
}

#[derive(Debug, Clone)]
pub struct PStruct {
    pub name: String,
    pub type_params: Vec<String>,
    pub fields: Vec<(String, Ty)>,
    /// Tuple struct (`struct P(i32, i32)`): fields are named `0`, `1`, ...
    pub tuple: bool,
    pub is_copy: bool,
    pub methods: Vec<PFn>,
    pub traits: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PVariant {
    pub name: String,
    pub fields: Vec<(String, Ty)>,
    /// Struct-like variant (named fields) vs tuple-like (`0`, `1`, ...).
    pub named: bool,
}

#[derive(Debug, Clone)]
pub struct PEnum {
    pub name: String,
    pub type_params: Vec<String>,
    pub variants: Vec<PVariant>,
    pub is_copy: bool,
    pub methods: Vec<PFn>,
    pub traits: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PTrait {
    pub name: String,
    pub methods: Vec<PFn>,
}

#[derive(Debug, Clone)]
pub struct PConst {
    pub name: String,
    pub ty: Ty,
    pub value: PExpr,
}

#[derive(Debug, Clone)]
pub struct PParam {
    pub name: String,
    pub ty: Ty,
    pub mode: ParamMode,
    /// Declared `mut`: the callee works on its own copy.
    pub own_mutable: bool,
}

#[derive(Debug, Clone)]
pub struct PFn {
    pub name: String,
    pub type_params: Vec<String>,
    /// Trait bounds per type parameter (`T: Speak`).
    pub bounds: Vec<(String, Vec<String>)>,
    /// `Some(mode)` when the function takes `self`.
    pub recv: Option<ParamMode>,
    pub params: Vec<PParam>,
    pub ret: Ty,
    /// `None` for trait methods without a default body.
    pub body: Option<PBlock>,
    /// Struct/enum/trait that owns the method.
    pub owner: Option<String>,
    pub is_async: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinKind {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnKind {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub enum PPat {
    Wild,
    Bind { name: String, mutable: bool },
    Lit(PExpr),
    /// Range pattern over ints/chars/floats: `lo..=hi`, `lo..hi`, `lo..`, `..=hi`, `..hi`.
    Range { lo: Option<PExpr>, hi: Option<PExpr>, inclusive: bool },
    Tuple(Vec<PPat>),
    Or(Vec<PPat>),
    /// `Enum::Variant(p0, p1)` / struct-like; fields are (field name, pattern).
    Variant { enum_name: String, variant: String, fields: Vec<(String, PPat)> },
    Some(Box<PPat>),
    None,
    Ok(Box<PPat>),
    Err(Box<PPat>),
}

/// Statements plus an optional value-producing tail expression.
#[derive(Debug, Clone, Default)]
pub struct PBlock {
    pub stmts: Vec<PStmt>,
    pub tail: Option<PExpr>,
}

impl PBlock {
    pub fn of(stmts: Vec<PStmt>) -> PBlock {
        PBlock { stmts, tail: None }
    }
}

#[derive(Debug, Clone)]
pub struct PArm {
    pub pat: PPat,
    pub guard: Option<PExpr>,
    pub body: PBlock,
}

#[derive(Debug, Clone)]
pub enum PStmt {
    Let { name: String, ty: Ty, mutable: bool, init: Option<PExpr> },
    /// `let (a, b) = e` / `let Some(x) = e else {..}` style destructuring.
    LetPat { pat: PPat, init: PExpr, els: Option<Vec<PStmt>> },
    Assign { target: PExpr, op: Option<BinKind>, value: PExpr },
    Expr(PExpr),
    If { cond: PExpr, then: Vec<PStmt>, els: Vec<PStmt> },
    While { cond: PExpr, body: Vec<PStmt> },
    Loop { body: Vec<PStmt> },
    /// `for var in start..end` over integers (the common, fast path).
    ForRange { var: String, start: PExpr, end: PExpr, inclusive: bool, body: Vec<PStmt> },
    /// `for pat in iterable` where iterable is a Vec / Set / Map / Iter / string chars.
    ForIn { pat: PPat, iter: PExpr, body: Vec<PStmt> },
    Match { scrutinee: PExpr, arms: Vec<PArm> },
    Break,
    Continue,
    /// `break 'label` / `continue 'label`: target the loop wrapped by `Labeled { label, .. }`.
    BreakTo(String),
    ContinueTo(String),
    /// `'label: loop/while/for` (only emitted when some `BreakTo`/`ContinueTo` targets it).
    Labeled { label: String, stmt: Box<PStmt> },
    Return(Option<PExpr>),
    Block(Vec<PStmt>),
    Defer(Box<PStmt>),
}

#[derive(Debug, Clone)]
pub struct PExpr {
    pub ty: Ty,
    pub kind: Box<PK>,
    /// 1-based source line (0 when unknown) for diagnostics.
    pub line: u32,
}

/// Parsed `{...}` format spec: Rust's `[[fill]align][sign]['#']['0'][width]['.' precision][type]`
/// with types `?`, `x X b o e`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FmtSpec {
    pub debug: bool,
    /// `{:#?}`: pretty (multi-line) Debug. Implies `alternate`.
    pub pretty: bool,
    /// `+`: always print the sign of numbers.
    pub plus: bool,
    /// `#`: alternate form (`0x`/`0b`/`0o` prefixes; pretty Debug).
    pub alternate: bool,
    pub precision: Option<usize>,
    pub width: Option<usize>,
    pub fill: Option<char>,
    /// '<', '>' or '^'.
    pub align: Option<char>,
    pub zero_pad: bool,
    pub radix: Option<char>,
}

#[derive(Debug, Clone)]
pub enum FmtPiece {
    Lit(String),
    Arg(PExpr, FmtSpec),
}

#[derive(Debug, Clone)]
pub enum Resolution {
    /// Method declared in the program on `owner` (struct, enum or trait).
    User { owner: String },
    /// Built-in method on a std type (Vec, String, HashMap, Option, ...), by name.
    Builtin,
}

#[derive(Debug, Clone)]
pub enum PK {
    Int(i128),
    Float(f64),
    Bool(bool),
    Str(String),
    Char(char),
    Unit,
    Local(String),
    Unary(UnKind, PExpr),
    Binary(BinKind, PExpr, PExpr),
    Cast(PExpr, Ty),
    Field(PExpr, String),
    TupleIdx(PExpr, usize),
    Index(PExpr, PExpr),
    /// Free function call.
    Call { name: String, args: Vec<PExpr> },
    /// `Type::assoc(args)` (static method / constructor).
    StaticCall { owner: String, name: String, args: Vec<PExpr> },
    /// Call of a function value (closure / fn pointer).
    CallValue { f: PExpr, args: Vec<PExpr> },
    Method { recv: PExpr, name: String, args: Vec<PExpr>, res: Resolution },
    Struct { name: String, fields: Vec<(String, PExpr)> },
    Variant { enum_name: String, variant: String, fields: Vec<(String, PExpr)> },
    Some(PExpr),
    None,
    Ok(PExpr),
    Err(PExpr),
    VecLit(Vec<PExpr>),
    VecRepeat(PExpr, PExpr),
    MapLit(Vec<(PExpr, PExpr)>),
    Tuple(Vec<PExpr>),
    /// Captures are by reference (SPEC-EXPR-011): JS/Go closures over the enclosing variables.
    Closure { params: Vec<(String, Ty)>, ret: Ty, body: PBlock },
    If { cond: PExpr, then: PBlock, els: PBlock },
    Match { scrutinee: PExpr, arms: Vec<PArm> },
    Block(PBlock),
    /// `e?`; `fn_ret` is the return type of the enclosing function or closure. `conv` names the
    /// `From` impl (`owner`, emitted method) applied to the error when the error types differ.
    Try { expr: PExpr, fn_ret: Ty, conv: Option<(String, String)> },
    Format(Vec<FmtPiece>),
    Print { newline: bool, to_stderr: bool, pieces: Vec<FmtPiece> },
    Range { start: PExpr, end: PExpr, inclusive: bool },
    /// Value-semantics copy of a place used as an owned value.
    Copy(PExpr),
    /// `&mut place` bound to a local (`let r = &mut z`): the local aliases the place, so writes
    /// through it (`*r = ..`, `r.f = ..`, `r.push(..)`) reach the place. Only appears as the
    /// `init` of a `PStmt::Let`; the expression's type is the pointee type. Emitters realise it
    /// as a pointer (Go) or as the shared object / an accessor cell (JS/TS).
    BorrowMut(PExpr),
    Panic(PExpr),
    /// Reference to a top-level function used as a value.
    FnRef(String),
    /// Call of a portable `std::<module>` function (see `stdlib.rs`).
    StdCall { module: String, name: String, args: Vec<PExpr> },
}

impl PExpr {
    pub fn new(ty: Ty, kind: PK) -> PExpr {
        PExpr { ty, kind: Box::new(kind), line: 0 }
    }

    pub fn at(mut self, line: u32) -> PExpr {
        self.line = line;
        self
    }

    pub fn unit() -> PExpr {
        PExpr::new(Ty::Unit, PK::Unit)
    }

    pub fn int(v: i128, k: IntKind) -> PExpr {
        PExpr::new(Ty::Int(k), PK::Int(v))
    }

    /// True for expressions that denote a storage location (copying them matters).
    pub fn is_place(&self) -> bool {
        matches!(&*self.kind, PK::Local(_) | PK::Field(..) | PK::TupleIdx(..) | PK::Index(..))
    }
}

/// Raised for constructs the portable backends do not implement. Never emitted as garbage code.
#[derive(Debug, Clone)]
pub struct Unsupported {
    pub what: String,
    pub line: u32,
}

impl std::fmt::Display for Unsupported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line > 0 {
            write!(f, "unsupported on this backend: {} (line {})", self.what, self.line)
        } else {
            write!(f, "unsupported on this backend: {}", self.what)
        }
    }
}

impl std::error::Error for Unsupported {}

pub type PResult<T> = Result<T, Unsupported>;

pub fn unsupported<T>(what: impl Into<String>) -> PResult<T> {
    Err(Unsupported { what: what.into(), line: 0 })
}
