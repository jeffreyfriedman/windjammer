//! Definition identities: every nameable item gets a stable [`DefId`].

use crate::source_map::Location;
use std::collections::HashMap;

/// Dense, copyable identity of one definition inside a [`DefTable`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DefId(pub u32);

/// What a definition is. `owner` ids point at the owning `Impl`, `Trait` or `Enum` def.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefKind {
    Fn,
    ExternFn,
    /// Function in an `impl` block that takes a `self` receiver.
    Method {
        owner: DefId,
    },
    /// Function in an `impl` block without a receiver (`Type::new`).
    AssocFn {
        owner: DefId,
    },
    Struct,
    Enum,
    Variant {
        owner: DefId,
    },
    Trait,
    /// Method declared in a `trait` block (with or without default body).
    TraitMethod {
        owner: DefId,
    },
    Impl,
    Const,
    Static,
    Module,
    TypeAlias,
}

/// One definition. Fields are plain data so consumers can inspect without the resolver.
#[derive(Debug, Clone)]
pub struct Def {
    pub id: DefId,
    pub kind: DefKind,
    pub name: String,
    /// Fully module-qualified path (`a::b::Type::method`); the root module is `""`.
    pub path: String,
    /// Module that lexically contains this definition (a module contains itself at the root).
    pub module: DefId,
    pub is_pub: bool,
    /// For functions/methods: has a `self` receiver.
    pub has_self: bool,
    /// For functions/methods: parameter count excluding the receiver.
    pub arity: usize,
    /// For tuple structs: callable as a constructor (lives in the value namespace).
    pub is_ctor: bool,
    /// For type aliases: the aliased type name.
    pub alias_target: Option<String>,
    pub location: Option<Location>,
}

impl Def {
    /// True when this def can be named in a value position (call, path expression).
    pub fn in_value_ns(&self) -> bool {
        match self.kind {
            DefKind::Fn
            | DefKind::ExternFn
            | DefKind::Const
            | DefKind::Static
            | DefKind::Variant { .. } => true,
            DefKind::Struct => self.is_ctor,
            _ => false,
        }
    }

    /// True when this def can be named in a type/module position (path prefix).
    pub fn in_type_ns(&self) -> bool {
        matches!(
            self.kind,
            DefKind::Module | DefKind::Struct | DefKind::Enum | DefKind::Trait | DefKind::TypeAlias
        )
    }
}

/// Arena of definitions plus a path index. Paths may map to several defs (that is data, not
/// a decision: the resolver decides what an ambiguity means).
#[derive(Debug, Default)]
pub struct DefTable {
    defs: Vec<Def>,
    by_path: HashMap<String, Vec<DefId>>,
}

impl DefTable {
    pub fn alloc(&mut self, mut def: Def) -> DefId {
        let id = DefId(self.defs.len() as u32);
        def.id = id;
        self.by_path.entry(def.path.clone()).or_default().push(id);
        self.defs.push(def);
        id
    }

    pub fn get(&self, id: DefId) -> &Def {
        &self.defs[id.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.defs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Def> {
        self.defs.iter()
    }

    /// All defs registered under a full path.
    pub fn find_path(&self, path: &str) -> &[DefId] {
        self.by_path.get(path).map(Vec::as_slice).unwrap_or(&[])
    }

    /// The single def under a path, or `None` when absent or shared.
    pub fn find_one(&self, path: &str) -> Option<DefId> {
        match self.find_path(path) {
            [one] => Some(*one),
            _ => None,
        }
    }
}

impl DefTable {
    pub(crate) fn get_mut(&mut self, id: DefId) -> &mut Def {
        &mut self.defs[id.0 as usize]
    }
}
