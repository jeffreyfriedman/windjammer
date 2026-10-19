//! Runtime value model.
//!
//! Values are immutable trees shared through `Rc` with copy-on-write mutation
//! (`Rc::make_mut`), so moving/cloning is cheap and `.clone()` is always independent.
//! Mutation through `&mut` goes through [`Place`] projections, never raw aliasing.
//! Integers carry their Rust width ([`IntTy`]).

use super::int::IntTy;
pub use super::iter::{Iter, IterState};
use crate::parser::{Expression, Type};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

pub type Slot = Rc<RefCell<Value>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatTy {
    F32,
    F64,
    Lit,
}

#[derive(Clone, Debug)]
pub struct StructVal {
    pub name: Rc<str>,
    pub fields: Vec<(Rc<str>, Value)>,
    /// `@derive(Copy)`: reading the place yields an independent copy.
    pub copy: bool,
    /// Identity of an instance whose type has `impl Drop` (0 = no destructor). See `heap.rs`.
    pub uid: u32,
}

#[derive(Clone, Debug)]
pub enum Payload {
    Unit,
    Tuple(Vec<Value>),
    Struct(Vec<(Rc<str>, Value)>),
}

#[derive(Clone, Debug)]
pub struct EnumVal {
    pub ty: Rc<str>,
    pub variant: Rc<str>,
    pub idx: u32,
    pub payload: Payload,
}

/// A map or set body. `hashed` marks `HashMap`/`HashSet`, whose iteration order is unspecified in Rust
/// (ADR-0021 D6): by default they iterate in key order, and `--hash-seed N` permutes that order
/// deterministically so order-dependent programs can be found. `BTreeMap`/`BTreeSet` always iterate sorted.
#[derive(Clone, Debug, Default)]
pub struct KMap {
    pub m: BTreeMap<Key, Value>,
    pub hashed: bool,
}

impl std::ops::Deref for KMap {
    type Target = BTreeMap<Key, Value>;
    fn deref(&self) -> &Self::Target {
        &self.m
    }
}
impl std::ops::DerefMut for KMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.m
    }
}
impl From<BTreeMap<Key, Value>> for KMap {
    fn from(m: BTreeMap<Key, Value>) -> KMap {
        KMap { m, hashed: false }
    }
}

fn seeded_hash(seed: u64, k: &Key) -> u64 {
    // FNV-1a over the key's structural text, then a splitmix-style finalizer mixed with the seed
    let mut h: u64 = 0xcbf29ce484222325;
    for b in format!("{:?}", k.0).bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    let mut z = h ^ seed.wrapping_mul(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

impl KMap {
    pub fn hash_map(m: BTreeMap<Key, Value>) -> KMap {
        KMap { m, hashed: true }
    }

    /// Entries in iteration order (see the type docs); `seed` is `--hash-seed` (0 = key order).
    pub fn ordered(&self, seed: u64) -> Vec<(&Key, &Value)> {
        let mut v: Vec<(&Key, &Value)> = self.m.iter().collect();
        if self.hashed && seed != 0 {
            v.sort_by_cached_key(|(k, _)| seeded_hash(seed, k));
        }
        v
    }

    pub fn ordered_keys(&self, seed: u64) -> Vec<Value> {
        self.ordered(seed).into_iter().map(|(k, _)| k.0.clone()).collect()
    }

    pub fn ordered_values(&self, seed: u64) -> Vec<Value> {
        self.ordered(seed).into_iter().map(|(_, v)| v.clone()).collect()
    }
}

/// Total-ordered map key wrapper.
#[derive(Clone, Debug)]
pub struct Key(pub Value);

impl PartialEq for Key {
    fn eq(&self, o: &Key) -> bool {
        self.0.structural_cmp(&o.0) == Ordering::Equal
    }
}
impl Eq for Key {}
impl PartialOrd for Key {
    fn partial_cmp(&self, o: &Key) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Key {
    fn cmp(&self, o: &Key) -> Ordering {
        self.0.structural_cmp(&o.0)
    }
}

#[derive(Clone, Debug)]
pub enum Proj {
    Field(Rc<str>),
    Index(usize),
    Key(Value),
}

/// A location inside a variable slot: the target of a `&mut` borrow.
#[derive(Clone, Debug)]
pub struct Place {
    pub slot: Slot,
    pub path: Vec<Proj>,
    /// Set for lock/borrow guards: keeps the lock held while any copy of the guard lives.
    pub guard: Option<Rc<super::locks::GuardToken>>,
}


pub enum Callable {
    Closure {
        params: &'static [String],
        body: &'static Expression<'static>,
        env: Vec<(&'static str, Slot)>,
        module: usize,
        self_ty: Option<Rc<str>>,
        tparams: Vec<(Rc<str>, Option<Type>)>,
    },
    /// A user function addressed by (module, qualified name), e.g. `Type::method`.
    Named { module: usize, name: Rc<str> },
    /// A builtin function or enum-variant constructor addressed by path, e.g. `Some`.
    Path(Rc<str>),
    /// A call with evaluated arguments, run later (the body of a `spawn f(x)` task).
    Thunk { func: Value, args: Vec<Value> },
    /// The future of a not-yet-run `async fn` call: arguments are evaluated, the body runs on `.await`.
    Deferred { func: Rc<crate::interpreter::engine::FuncDef>, args: Vec<Value>, self_val: Option<Value>, targs: Vec<Type>, expected: Option<Type> },
}

impl fmt::Debug for Callable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Callable::Closure { params, .. } => write!(f, "<closure |{}|>", params.join(", ")),
            Callable::Named { name, .. } => write!(f, "<fn {}>", name),
            Callable::Path(p) => write!(f, "<fn {}>", p),
            Callable::Deferred { func, .. } => write!(f, "<future {}>", func.qual),
            Callable::Thunk { .. } => write!(f, "<task body>"),
        }
    }
}

impl fmt::Debug for IterState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<iter>")
    }
}

/// The interior-mutability wrappers (`Rc`/`Arc` are `Value::Rc`, not a cell).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellKind {
    RefCell,
    Cell,
    Mutex,
    RwLock,
    Atomic,
}

impl CellKind {
    pub fn name(self) -> &'static str {
        match self {
            CellKind::RefCell => "RefCell",
            CellKind::Cell => "Cell",
            CellKind::Mutex => "Mutex",
            CellKind::RwLock => "RwLock",
            CellKind::Atomic => "Atomic",
        }
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    Unit,
    Bool(bool),
    Int(i128, IntTy),
    Float(f64, FloatTy),
    Char(char),
    Str(Rc<String>),
    Vec(Rc<Vec<Value>>),
    Tuple(Rc<Vec<Value>>),
    Struct(Rc<StructVal>),
    Enum(Rc<EnumVal>),
    Map(Rc<KMap>),
    Set(Rc<KMap>),
    /// Half-open integer range; `a..=b` is stored as `a..b+1`.
    Range(i128, i128, IntTy),
    Iter(Iter),
    Func(Rc<Callable>),
    /// `fmt::Formatter` / `String` sink used by `write!` inside `impl Display`.
    Formatter(Rc<RefCell<String>>),
    Ref(Place),
    /// `Rc<T>` / `Arc<T>`: a shared slot. Clones alias; strong counts are computed by reachability.
    Rc(Slot),
    /// `Weak<T>` of an `Rc` slot: a real `std::rc::Weak`, so a value kept alive only by an `Rc`
    /// cycle still upgrades (Rust leaks cycles).
    Weak(std::rc::Weak<RefCell<Value>>),
    /// `RefCell` / `Cell` / `Mutex` / `RwLock`: an interior-mutable slot.
    Cell(CellKind, Slot, Rc<super::locks::CellState>),
    /// Host objects: threads, channels, durations... (see `native.rs`).
    Native(Rc<dyn super::native::Native>),
}

impl Value {
    pub fn int(v: i128) -> Value {
        Value::Int(v, IntTy::Lit)
    }
    pub fn typed(v: i128, t: IntTy) -> Value {
        Value::Int(v, t)
    }
    pub fn usize(v: usize) -> Value {
        Value::Int(v as i128, IntTy::Usize)
    }
    pub fn str(s: impl Into<String>) -> Value {
        Value::Str(Rc::new(s.into()))
    }
    pub fn vec(v: Vec<Value>) -> Value {
        Value::Vec(Rc::new(v))
    }
    pub fn tuple(v: Vec<Value>) -> Value {
        if v.is_empty() {
            Value::Unit
        } else {
            Value::Tuple(Rc::new(v))
        }
    }
    pub fn btree_map(m: BTreeMap<Key, Value>) -> Value {
        Value::Map(Rc::new(KMap::from(m)))
    }
    pub fn hash_map(m: BTreeMap<Key, Value>) -> Value {
        Value::Map(Rc::new(KMap::hash_map(m)))
    }
    pub fn btree_set(m: BTreeMap<Key, Value>) -> Value {
        Value::Set(Rc::new(KMap::from(m)))
    }
    pub fn hash_set(m: BTreeMap<Key, Value>) -> Value {
        Value::Set(Rc::new(KMap::hash_map(m)))
    }
    pub fn float(f: f64) -> Value {
        Value::Float(f, FloatTy::F64)
    }

    pub fn enum_val(ty: &str, variant: &str, idx: u32, payload: Payload) -> Value {
        Value::Enum(Rc::new(EnumVal {
            ty: ty.into(),
            variant: variant.into(),
            idx,
            payload,
        }))
    }
    pub fn some(v: Value) -> Value {
        Value::enum_val("Option", "Some", 1, Payload::Tuple(vec![v]))
    }
    pub fn none() -> Value {
        Value::enum_val("Option", "None", 0, Payload::Unit)
    }
    pub fn ok(v: Value) -> Value {
        Value::enum_val("Result", "Ok", 0, Payload::Tuple(vec![v]))
    }
    pub fn err(v: Value) -> Value {
        Value::enum_val("Result", "Err", 1, Payload::Tuple(vec![v]))
    }
    pub fn option(v: Option<Value>) -> Value {
        match v {
            Some(v) => Value::some(v),
            None => Value::none(),
        }
    }

    /// Option/Result payload views.
    pub fn as_option(&self) -> Option<Option<&Value>> {
        match self {
            Value::Enum(e) if &*e.ty == "Option" => match &e.payload {
                Payload::Tuple(v) if e.idx == 1 => Some(v.first()),
                _ => Some(None),
            },
            _ => None,
        }
    }

    pub fn is_copy_aggregate(&self) -> bool {
        match self {
            Value::Struct(s) => s.copy,
            Value::Tuple(_) => true,
            _ => false,
        }
    }

    pub fn type_name(&self) -> String {
        match self {
            Value::Unit => "()".into(),
            Value::Bool(_) => "bool".into(),
            Value::Int(_, t) => t.name().into(),
            Value::Float(_, FloatTy::F32) => "f32".into(),
            Value::Float(..) => "f64".into(),
            Value::Char(_) => "char".into(),
            Value::Str(_) => "String".into(),
            Value::Vec(_) => "Vec".into(),
            Value::Tuple(_) => "tuple".into(),
            Value::Struct(s) => s.name.to_string(),
            Value::Enum(e) => e.ty.to_string(),
            Value::Map(_) => "HashMap".into(),
            Value::Set(_) => "HashSet".into(),
            Value::Range(..) => "Range".into(),
            Value::Iter(_) => "Iterator".into(),
            Value::Func(_) => "fn".into(),
            Value::Formatter(_) => "Formatter".into(),
            Value::Ref(_) => "&mut".into(),
            Value::Rc(_) => "Rc".into(),
            Value::Weak(_) => "Weak".into(),
            Value::Cell(k, ..) => k.name().into(),
            Value::Native(n) => n.type_name().into(),
        }
    }

    /// Total structural ordering (used for ==, <, sort, BTreeMap keys).
    pub fn structural_cmp(&self, other: &Value) -> Ordering {
        use Value::*;
        match (self, other) {
            (Unit, Unit) => Ordering::Equal,
            (Bool(a), Bool(b)) => a.cmp(b),
            (Int(a, IntTy::U128), Int(b, _)) | (Int(a, _), Int(b, IntTy::U128)) => (*a as u128).cmp(&(*b as u128)),
            (Int(a, _), Int(b, _)) => a.cmp(b),
            (Float(a, _), Float(b, _)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
            (Int(a, _), Float(b, _)) => (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal),
            (Float(a, _), Int(b, _)) => a.partial_cmp(&(*b as f64)).unwrap_or(Ordering::Equal),
            (Char(a), Char(b)) => a.cmp(b),
            (Str(a), Str(b)) => a.as_str().cmp(b.as_str()),
            (Vec(a), Vec(b)) | (Tuple(a), Tuple(b)) => cmp_seq(a, b),
            (Struct(a), Struct(b)) => {
                for ((_, x), (_, y)) in a.fields.iter().zip(b.fields.iter()) {
                    let o = x.structural_cmp(y);
                    if o != Ordering::Equal {
                        return o;
                    }
                }
                Ordering::Equal
            }
            (Enum(a), Enum(b)) => a.idx.cmp(&b.idx).then_with(|| match (&a.payload, &b.payload) {
                (Payload::Tuple(x), Payload::Tuple(y)) => cmp_seq(x, y),
                (Payload::Struct(x), Payload::Struct(y)) => {
                    for ((_, p), (_, q)) in x.iter().zip(y.iter()) {
                        let o = p.structural_cmp(q);
                        if o != Ordering::Equal {
                            return o;
                        }
                    }
                    Ordering::Equal
                }
                _ => Ordering::Equal,
            }),
            (Map(a), Map(b)) | (Set(a), Set(b)) => {
                let ka: std::vec::Vec<_> = a.iter().collect();
                let kb: std::vec::Vec<_> = b.iter().collect();
                for ((k1, v1), (k2, v2)) in ka.iter().zip(kb.iter()) {
                    let o = k1.cmp(k2).then_with(|| v1.structural_cmp(v2));
                    if o != Ordering::Equal {
                        return o;
                    }
                }
                ka.len().cmp(&kb.len())
            }
            (Range(a, b, _), Range(c, d, _)) => (a, b).cmp(&(c, d)),
            (Rc(a), Rc(b)) => a.borrow().structural_cmp(&b.borrow()),
            (Cell(_, a, _), Cell(_, b, _)) => a.borrow().structural_cmp(&b.borrow()),
            (Native(a), Native(b)) => match (a.order_key(), b.order_key()) {
                (Some(x), Some(y)) => x.cmp(&y),
                _ => (std::rc::Rc::as_ptr(a) as *const () as usize).cmp(&(std::rc::Rc::as_ptr(b) as *const () as usize)),
            },
            (Rc(p), o) => p.borrow().structural_cmp(o),
            (s, Rc(p)) => s.structural_cmp(&p.borrow()),
            (Ref(p), o) => p.slot.borrow().structural_cmp(o),
            (s, Ref(p)) => s.structural_cmp(&p.slot.borrow()),
            _ => self.rank().cmp(&other.rank()),
        }
    }

    fn rank(&self) -> u8 {
        match self {
            Value::Unit => 0,
            Value::Bool(_) => 1,
            Value::Int(..) => 2,
            Value::Float(..) => 3,
            Value::Char(_) => 4,
            Value::Str(_) => 5,
            _ => 6,
        }
    }

    pub fn structural_eq(&self, other: &Value) -> bool {
        // NaN != NaN like Rust
        if let (Value::Float(a, _), Value::Float(b, _)) = (self, other) {
            return a == b;
        }
        // runtime `impl PartialEq<&str> for HttpMethod` (case-insensitive)
        if let (Value::Enum(e), Value::Str(s)) | (Value::Str(s), Value::Enum(e)) = (self, other) {
            if &*e.ty == "HttpMethod" {
                return e.variant.eq_ignore_ascii_case(s);
            }
        }
        self.structural_cmp(other) == Ordering::Equal
    }
}

fn cmp_seq(a: &[Value], b: &[Value]) -> Ordering {
    for (x, y) in a.iter().zip(b.iter()) {
        let o = x.structural_cmp(y);
        if o != Ordering::Equal {
            return o;
        }
    }
    a.len().cmp(&b.len())
}

/// `Type` helpers shared by evaluator and builtins.
pub fn type_is_float(t: &Type) -> Option<FloatTy> {
    match t {
        Type::Float => Some(FloatTy::F64),
        Type::Custom(n) if n == "f64" => Some(FloatTy::F64),
        Type::Custom(n) if n == "f32" => Some(FloatTy::F32),
        Type::Reference(i) | Type::MutableReference(i) => type_is_float(i),
        _ => None,
    }
}
