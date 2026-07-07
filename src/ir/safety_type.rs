//! Core safety type definitions.
//!
//! `SafetyType` is the canonical type representation for every IR node.
//! It encodes ownership mode, effect set, taint status, const-eval eligibility,
//! and execution mode in a single struct. The constraint solver produces these;
//! codegen backends consume them.

use std::collections::BTreeSet;
use std::fmt;

/// The full type of an IR node, including all safety information.
#[derive(Debug, Clone, PartialEq)]
pub struct SafetyType {
    /// The base type (Int, Float, String, Custom, etc.)
    pub base: BaseType,

    /// Ownership mode — how this value's memory is managed
    pub ownership: OwnedType,

    /// Effects this expression may perform
    pub effects: EffectSet,

    /// Taint status of this value
    pub taint: TaintStatus,

    /// Whether this value can be evaluated at compile time
    pub const_eval: ConstEval,

    /// Execution mode if this is a call expression
    pub exec_mode: Option<ExecutionMode>,
}

impl SafetyType {
    /// Create a simple owned type with no effects or taint.
    pub fn owned(base: BaseType) -> Self {
        Self {
            base,
            ownership: OwnedType::Owned,
            effects: EffectSet::pure(),
            taint: TaintStatus::Clean,
            const_eval: ConstEval::Runtime,
            exec_mode: None,
        }
    }

    /// Create a borrowed reference type.
    pub fn borrowed(base: BaseType, region: Region) -> Self {
        Self {
            base,
            ownership: OwnedType::Ref(region),
            effects: EffectSet::pure(),
            taint: TaintStatus::Clean,
            const_eval: ConstEval::Runtime,
            exec_mode: None,
        }
    }

    /// Create a mutable reference type.
    pub fn mut_borrowed(base: BaseType, region: Region) -> Self {
        Self {
            base,
            ownership: OwnedType::MutRef(region),
            effects: EffectSet::pure(),
            taint: TaintStatus::Clean,
            const_eval: ConstEval::Runtime,
            exec_mode: None,
        }
    }

    /// Create a Copy type (no ownership tracking needed).
    pub fn copy(base: BaseType) -> Self {
        Self {
            base,
            ownership: OwnedType::Copy,
            effects: EffectSet::pure(),
            taint: TaintStatus::Clean,
            const_eval: ConstEval::Runtime,
            exec_mode: None,
        }
    }

    pub fn is_pure(&self) -> bool {
        self.effects.is_pure()
    }

    pub fn is_tainted(&self) -> bool {
        matches!(self.taint, TaintStatus::Tainted(_))
    }

    pub fn is_const(&self) -> bool {
        matches!(
            self.const_eval,
            ConstEval::Const | ConstEval::Comptime { .. }
        )
    }
}

/// Base type without safety annotations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BaseType {
    Unit,
    Bool,
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    F32,
    F64,
    String,
    Char,
    /// A user-defined type (struct, enum, trait object).
    Custom(String),
    /// Generic type parameter (not yet resolved).
    TypeVar(u32),
    /// Vec<T>
    Vec(Box<BaseType>),
    /// Option<T>
    Option(Box<BaseType>),
    /// Result<T, E>
    Result(Box<BaseType>, Box<BaseType>),
    /// HashMap<K, V>
    HashMap(Box<BaseType>, Box<BaseType>),
    /// Tuple of types.
    Tuple(Vec<BaseType>),
    /// Fixed-size array: [T; N]
    Array(Box<BaseType>, usize),
    /// Function type: (params) -> return
    Fn(Vec<BaseType>, Box<BaseType>),
    /// Function pointer type: fn(params) -> return
    FunctionPointer {
        params: Vec<BaseType>,
        return_type: Box<BaseType>,
    },
    /// Trait object: dyn Trait
    TraitObject(String),
    /// impl Trait (opaque return type / existential)
    ImplTrait(String),
    /// Raw pointer (*const T or *mut T, for FFI)
    RawPointer {
        mutable: bool,
        inner: Box<BaseType>,
    },
    /// SIMD vector: Simd<T, N> (WJ-LANG-02)
    Simd {
        element: Box<BaseType>,
        lanes: u32,
    },
    /// Inferred — not yet determined by the solver.
    Inferred,
}

/// Ownership mode for an IR node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OwnedType {
    /// Value is moved; caller gives up ownership.
    Owned,
    /// Shared borrow with a region identifier.
    Ref(Region),
    /// Exclusive borrow with a region identifier.
    MutRef(Region),
    /// Type implements Copy; no ownership tracking needed.
    Copy,
    /// Not yet determined by the solver.
    Inferred,
}

/// A region identifier for borrow tracking.
/// Regions allow the solver to verify aliasing constraints:
/// - `MutRef(r)` asserts no other live reference to region `r`
/// - `Ref(r)` asserts no live `MutRef` to region `r`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Region(pub u32);

impl Region {
    pub fn fresh(id: u32) -> Self {
        Self(id)
    }
}

/// The set of effects a function or expression may perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectSet(BTreeSet<Effect>);

impl EffectSet {
    pub fn pure() -> Self {
        Self(BTreeSet::new())
    }

    pub fn single(effect: Effect) -> Self {
        let mut set = BTreeSet::new();
        set.insert(effect);
        Self(set)
    }

    pub fn union(&self, other: &EffectSet) -> EffectSet {
        EffectSet(self.0.union(&other.0).cloned().collect())
    }

    pub fn is_pure(&self) -> bool {
        self.0.is_empty()
    }

    pub fn contains(&self, effect: &Effect) -> bool {
        self.0.contains(effect)
    }

    pub fn insert(&mut self, effect: Effect) {
        self.0.insert(effect);
    }

    pub fn is_subset_of(&self, other: &EffectSet) -> bool {
        self.0.is_subset(&other.0)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Effect> {
        self.0.iter()
    }

    pub fn from_iter(effects: impl IntoIterator<Item = Effect>) -> Self {
        Self(effects.into_iter().collect())
    }
}

/// Individual effect capabilities (WJ-SEC-01).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Effect {
    FsRead,
    FsWrite,
    NetEgress,
    NetIngress,
    ProcessSpawn,
    EnvRead,
    EnvWrite,
    Ffi,
    /// User-defined effect from a plugin or library.
    Custom(String),
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Effect::FsRead => write!(f, "fs_read"),
            Effect::FsWrite => write!(f, "fs_write"),
            Effect::NetEgress => write!(f, "net_egress"),
            Effect::NetIngress => write!(f, "net_ingress"),
            Effect::ProcessSpawn => write!(f, "process_spawn"),
            Effect::EnvRead => write!(f, "env_read"),
            Effect::EnvWrite => write!(f, "env_write"),
            Effect::Ffi => write!(f, "ffi"),
            Effect::Custom(name) => write!(f, "{}", name),
        }
    }
}

// TaintStatus, TaintSource, and TaintSourceKind are canonically defined in
// crate::ir::taint and re-exported here for SafetyType consumers.
pub use crate::ir::taint::{TaintSource, TaintSourceKind, TaintStatus};

/// Whether an expression can be evaluated at compile time (WJ-LANG-02).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstEval {
    /// Must be evaluated at runtime.
    Runtime,
    /// Evaluated at compile time via a `comptime { }` block.
    Comptime {
        /// Source expression or block identifier for future const evaluation.
        expr: String,
    },
    /// Must be evaluated at compile time (const declarations).
    Const,
}

/// Execution mode for call expressions (WJ-CONC-01).
/// The caller chooses — functions are not colored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Synchronous call — blocks until complete.
    Sync,
    /// Asynchronous call — returns Future<T>.
    Async,
    /// Spawned task — returns JoinHandle<T>.
    Spawn,
}
