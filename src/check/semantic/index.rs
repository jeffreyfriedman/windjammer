//! Project-wide symbol index: structs, functions and the names each file brings into scope.

use crate::parser::{FunctionDecl, Item, Program, Type};
use std::collections::{HashMap, HashSet};

pub struct StructInfo {
    pub fields: Vec<(String, Type)>,
    pub is_tuple: bool,
    /// Generic structs are skipped by type-level checks (type parameters are not tracked).
    pub generic: bool,
}

pub struct FnInfo {
    pub params: Vec<Type>,
    pub generic: bool,
}

#[derive(Default)]
pub struct Index {
    pub structs: HashMap<String, Vec<StructInfo>>,
    pub fns: HashMap<String, Vec<FnInfo>>,
    /// Every user-declared type-like name (struct, enum, trait, alias, bound).
    pub types: HashSet<String>,
    /// Struct/enum name -> methods available on it (inherent, project-trait and known std-trait impls).
    pub methods: HashMap<String, HashSet<String>>,
    /// Types with an impl of a trait this index does not know (its methods are unknown).
    pub opaque_impls: HashSet<String>,
    /// Methods every type may have through blanket `impl<T> Trait for T` impls.
    pub blanket_methods: HashSet<String>,
    /// Enum names (method checks apply to them too).
    pub enums: HashSet<String>,
    traits: HashMap<String, Vec<String>>,
}

/// Methods of std traits commonly implemented by hand, plus those every value has.
const STD_TRAIT_METHODS: &[(&str, &[&str])] = &[
    ("Display", &["fmt", "to_string"]),
    ("Debug", &["fmt"]),
    ("Clone", &["clone", "clone_from"]),
    ("Copy", &[]),
    ("PartialEq", &["eq", "ne"]),
    ("Eq", &[]),
    ("PartialOrd", &["partial_cmp", "lt", "le", "gt", "ge"]),
    ("Ord", &["cmp", "max", "min", "clamp"]),
    ("Hash", &["hash"]),
    ("Default", &["default"]),
    ("Drop", &["drop"]),
];

/// Names visible in one file without being declared in a function body.
#[derive(Default)]
pub struct FileNames {
    pub values: HashSet<String>,
    pub imported: HashSet<String>,
    pub glob: bool,
}

/// Std modules usable without an explicit `use` (auto-imported by the compiler).
const STD_NAMES: &[&str] = &[
    "std", "async", "cli", "collections", "compress", "compute", "config", "crypto", "csv", "db",
    "dialog", "encoding", "env", "fs", "http", "json", "jwt", "log", "map", "math", "mime", "net",
    "path", "process", "random", "regex", "runtime", "storage", "strings", "subprocess", "sync",
    "test", "testing", "time", "url", "uuid", "yaml", "stdout", "stderr", "stdin", "self", "super",
    "crate", "args", "ui",
];

pub fn is_std_name(n: &str) -> bool {
    STD_NAMES.contains(&n)
}

fn fn_info(d: &FunctionDecl<'_>) -> FnInfo {
    let params: Vec<Type> = d
        .parameters
        .iter()
        .filter(|p| p.name != "self")
        .map(|p| p.type_.clone())
        .collect();
    FnInfo {
        params,
        generic: !d.type_params.is_empty() || d.parameters.iter().any(|p| p.pattern.is_some()),
    }
}

impl Index {
    pub fn build(programs: &[&Program<'_>]) -> Self {
        let mut idx = Index::default();
        for p in programs {
            idx.collect_traits(&p.items);
        }
        for p in programs {
            idx.add_items(&p.items);
        }
        idx
    }

    fn collect_traits(&mut self, items: &[Item<'_>]) {
        for item in items {
            match item {
                Item::Trait { decl, .. } => {
                    let ms = decl.methods.iter().map(|m| m.name.clone()).collect();
                    self.traits.insert(decl.name.clone(), ms);
                }
                Item::Mod { items, .. } => self.collect_traits(items),
                _ => {}
            }
        }
    }

    fn add_impl(&mut self, block: &crate::parser::ImplBlock<'_>) {
        let ty = block.type_name.split('<').next().unwrap_or("").trim().to_string();
        let blanket = block.type_params.iter().any(|t| t.name == ty);
        let mut names: Vec<String> = block.functions.iter().map(|f| f.name.clone()).collect();
        if let Some(tr) = &block.trait_name {
            let tr = tr.rsplit("::").next().unwrap_or(tr);
            if let Some(ms) = self.traits.get(tr) {
                names.extend(ms.iter().cloned());
            } else if let Some((_, ms)) = STD_TRAIT_METHODS.iter().find(|(n, _)| *n == tr) {
                names.extend(ms.iter().map(|m| m.to_string()));
            } else if !blanket {
                self.opaque_impls.insert(ty.clone());
            }
        }
        if blanket {
            self.blanket_methods.extend(names);
        } else {
            self.methods.entry(ty).or_default().extend(names);
        }
    }

    fn add_items(&mut self, items: &[Item<'_>]) {
        for item in items {
            match item {
                Item::Struct { decl, .. } => {
                    self.types.insert(decl.name.clone());
                    self.structs
                        .entry(decl.name.clone())
                        .or_default()
                        .push(StructInfo {
                            fields: decl
                                .fields
                                .iter()
                                .map(|f| (f.name.clone(), f.field_type.clone()))
                                .collect(),
                            is_tuple: decl.tuple_fields.is_some(),
                            generic: !decl.type_params.is_empty(),
                        });
                }
                Item::Enum { decl, .. } => {
                    self.types.insert(decl.name.clone());
                    self.enums.insert(decl.name.clone());
                }
                Item::Impl { block, .. } => self.add_impl(block),
                Item::Trait { decl, .. } => {
                    self.types.insert(decl.name.clone());
                }
                Item::TypeAlias { name, .. } | Item::BoundAlias { name, .. } => {
                    self.types.insert(name.clone());
                }
                Item::Function { decl, .. } => {
                    self.fns.entry(decl.name.clone()).or_default().push(fn_info(decl));
                }
                Item::Mod { items, .. } => self.add_items(items),
                _ => {}
            }
        }
    }
}

fn words(s: &str, into: &mut HashSet<String>) {
    let mut cur = String::new();
    for c in s.chars().chain(std::iter::once(' ')) {
        if c.is_alphanumeric() || c == '_' {
            cur.push(c);
        } else if !cur.is_empty() {
            into.insert(std::mem::take(&mut cur));
        }
    }
}

impl FileNames {
    pub fn collect(items: &[Item<'_>]) -> Self {
        let mut n = FileNames::default();
        n.add(items);
        n
    }

    fn add(&mut self, items: &[Item<'_>]) {
        for item in items {
            match item {
                Item::Function { decl, .. } => {
                    self.values.insert(decl.name.clone());
                }
                Item::Const { name, .. } | Item::Static { name, .. } => {
                    self.values.insert(name.clone());
                }
                Item::ExternLet { name, .. } => {
                    self.values.insert(name.clone());
                }
                Item::Mod { name, items, .. } => {
                    self.values.insert(name.clone());
                    self.add(items);
                }
                Item::Use { path, alias, .. } => {
                    if path.iter().any(|s| s.contains('*')) {
                        self.glob = true;
                    }
                    for seg in path {
                        words(seg, &mut self.imported);
                    }
                    if let Some(a) = alias {
                        self.imported.insert(a.clone());
                    }
                }
                Item::Struct { decl, .. } => {
                    self.values.insert(decl.name.clone());
                }
                _ => {}
            }
        }
    }

    pub fn has_value(&self, name: &str) -> bool {
        self.values.contains(name) || self.imported.contains(name)
    }
}
