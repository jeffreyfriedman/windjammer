//! Whole-program facts the mutability check reads: which methods change their receiver and which parameters are
//! lent for change, decided by the IR solver (user code) and by the `&mut self` declarations of
//! `std/rust_core.wji` (core types); declared field, payload and result types. Nothing here matches names.

use crate::analyzer::{AnalyzedFunction, SignatureRegistry};
use crate::program_analysis::{Analyzed, FileKey, Globals, ProgramAnalysis};
use crate::ir::safety_type::OwnedType;
use crate::ir::stdlib_tables::StdlibTables;
use crate::parser::ast::core::{EnumVariantData, FunctionDecl, Item, Program};
use crate::parser::Type;
use crate::type_classification::type_name_leaf;
use crate::typing::core_surface::CoreSurface;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// One callable as the solver decided it.
#[derive(Debug, Clone, PartialEq)]
pub struct Callable {
    /// The receiver is `&mut self`.
    pub changes_receiver: bool,
    /// Declared with a `self` receiver.
    pub has_self: bool,
    /// Per value parameter (receiver excluded): lent for change (`&mut`).
    pub lends_param: Vec<bool>,
    /// The declared result type (`Self` replaced by the impl type).
    pub ret: Option<Type>,
}

#[derive(Default)]
pub struct Facts {
    pub core: Option<&'static CoreSurface>,
    /// `(type, method)` by the type's last path segment. `None`: two declarations disagree (not decided).
    methods: HashMap<(String, String), Option<Callable>>,
    /// Free functions by name (last path segment). `None`: ambiguous.
    fns: HashMap<String, Option<Callable>>,
    /// Struct name -> field -> declared type.
    pub fields: HashMap<String, HashMap<String, Type>>,
    /// Enum name -> variant -> tuple payload types.
    pub variants: HashMap<String, HashMap<String, Vec<Type>>>,
    /// Every struct and enum the program declares.
    pub types: HashSet<String>,
    /// Type names declared more than once with different fields.
    conflicting: HashSet<String>,
    /// Files the analyzer could not analyze: nothing is reported in them.
    pub unanalyzed: HashSet<String>,
}

fn strip_generics(name: &str) -> &str {
    type_name_leaf(name)
}

fn self_to(t: &Type, owner: &str) -> Type {
    match t {
        Type::Custom(n) if n == "Self" => Type::Custom(owner.to_string()),
        Type::Reference(x) => Type::Reference(Box::new(self_to(x, owner))),
        Type::MutableReference(x) => Type::MutableReference(Box::new(self_to(x, owner))),
        Type::Option(x) => Type::Option(Box::new(self_to(x, owner))),
        Type::Result(a, b) => Type::Result(Box::new(self_to(a, owner)), Box::new(self_to(b, owner))),
        Type::Vec(x) => Type::Vec(Box::new(self_to(x, owner))),
        other => other.clone(),
    }
}

fn insert<K: std::hash::Hash + Eq>(map: &mut HashMap<K, Option<Callable>>, key: K, c: Callable) {
    match map.get(&key) {
        None => {
            map.insert(key, Some(c));
        }
        Some(Some(old)) if *old == c => {}
        Some(_) => {
            map.insert(key, None);
        }
    }
}

fn quiet<T>(f: impl FnOnce() -> T) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok()
}

/// One file of the program: its name, the text that was parsed and the parse.
pub struct SourceFile<'a> {
    pub name: &'a str,
    pub text: &'a str,
    pub program: &'a Program<'static>,
}

impl Facts {
    /// Facts for every file of one program: two analyzer passes (the second with the signatures of all files, as
    /// the build does), then the IR solver per file. The first pass is the analysis the rest of the check already
    /// has (`session`), the second is computed here once per program.
    pub fn build(
        files: &[SourceFile<'_>],
        tables: Arc<StdlibTables>,
        session: &ProgramAnalysis,
    ) -> Self {
        crate::debug_env::count(crate::debug_env::Stage::FactsBuild);
        let mut facts = Facts {
            core: Some(crate::ir::stdlib_tables::shared_core_surface()),
            ..Default::default()
        };
        for f in files {
            facts.declarations(&f.program.items);
        }
        for n in std::mem::take(&mut facts.conflicting) {
            facts.fields.remove(&n);
            facts.variants.remove(&n);
            facts.types.remove(&n);
        }
        let keys: Vec<FileKey> = files.iter().map(|f| FileKey::new(f.name, f.text)).collect();
        let mut global = SignatureRegistry::new();
        let mut merged = Vec::new();
        for (f, key) in files.iter().zip(&keys) {
            match quiet(|| session.first_pass(key, f.program)) {
                Some(first) if first.result.is_ok() => {
                    if let Ok(ok) = &first.result {
                        global.merge(&ok.registry);
                    }
                    merged.push(key.clone());
                }
                _ => {
                    facts.unanalyzed.insert(f.name.to_string());
                }
            }
        }
        let global = Arc::new(global);
        let globals = Globals::Merged(merged);
        let struct_fields: HashMap<String, HashMap<String, Type>> = facts.fields.clone();
        for (f, key) in files.iter().zip(&keys) {
            if facts.unanalyzed.contains(f.name) {
                continue;
            }
            let solved = quiet(|| {
                let second = session.second_pass(key, f.program, globals.clone(), &global);
                let Ok(Analyzed {
                    functions: analyzed,
                    registry,
                    ..
                }) = &second.result
                else {
                    return None;
                };
                let module = crate::ir::IrPipeline::new()
                    .with_stdlib_tables(tables.clone())
                    .lower_to_ir(analyzed, registry, Some(&struct_fields));
                let modes: Vec<(FunctionDecl<'static>, HashMap<String, bool>)> = analyzed
                    .iter()
                    .zip(module.functions.iter())
                    .map(|(af, irf): (&AnalyzedFunction<'static>, _)| {
                        let lent = irf
                            .param_types
                            .iter()
                            .map(|(n, st)| {
                                (n.clone(), matches!(st.ownership, OwnedType::MutRef(_)))
                            })
                            .collect();
                        (af.decl.clone(), lent)
                    })
                    .collect();
                Some(modes)
            })
            .flatten();
            match solved {
                Some(modes) => {
                    for (decl, lent) in modes {
                        facts.record(&decl, &lent);
                    }
                }
                None => {
                    facts.unanalyzed.insert(f.name.to_string());
                }
            }
        }
        facts
    }

    fn declarations(&mut self, items: &[Item<'static>]) {
        for item in items {
            match item {
                Item::Struct { decl, .. } => {
                    let name = strip_generics(&decl.name).to_string();
                    self.types.insert(name.clone());
                    let fields = decl.fields.iter().map(|f| (f.name.clone(), f.field_type.clone()));
                    // tuple structs: fields `0`, `1`, ...
                    let tuple = decl.tuple_fields.iter().flatten().enumerate().map(|(i, t)| (i.to_string(), t.clone()));
                    let declared: HashMap<String, Type> = fields.chain(tuple).collect();
                    // two different structs of one name (separate programs checked together): neither decides
                    match self.fields.get(&name) {
                        Some(old) if *old != declared => {
                            self.conflicting.insert(name.clone());
                        }
                        _ => {
                            self.fields.insert(name, declared);
                        }
                    }
                }
                Item::Enum { decl, .. } => {
                    let name = strip_generics(&decl.name).to_string();
                    self.types.insert(name.clone());
                    let declared: HashMap<String, Vec<Type>> = decl
                        .variants
                        .iter()
                        .filter_map(|v| match &v.data {
                            EnumVariantData::Tuple(ts) => Some((v.name.clone(), ts.clone())),
                            _ => None,
                        })
                        .collect();
                    match self.variants.get(&name) {
                        Some(old) if *old != declared => {
                            self.conflicting.insert(name.clone());
                        }
                        _ => {
                            self.variants.insert(name, declared);
                        }
                    }
                }
                Item::Mod { items, .. } => self.declarations(items),
                _ => {}
            }
        }
    }

    fn record(&mut self, decl: &FunctionDecl<'static>, lent: &HashMap<String, bool>) {
        let owner = decl.parent_type.as_deref().map(|p| strip_generics(p).to_string());
        let has_self = decl.parameters.iter().any(|p| p.name == "self");
        let lends = |name: &str, declared: &Type| lent.get(name).copied().unwrap_or(false) || matches!(declared, Type::MutableReference(_));
        let lends_param = decl
            .parameters
            .iter()
            .filter(|p| p.name != "self")
            // a Copy scalar is passed by value whatever the body does with its copy
            .map(|p| lends(&p.name, &p.type_) && !is_copy_scalar(&p.type_))
            .collect();
        let changes_receiver = has_self && decl.parameters.iter().find(|p| p.name == "self").is_some_and(|p| lends("self", &p.type_));
        let ret = decl.return_type.as_ref().map(|t| match &owner {
            Some(o) => self_to(t, o),
            None => t.clone(),
        });
        let c = Callable { changes_receiver, has_self, lends_param, ret };
        match owner {
            Some(o) => insert(&mut self.methods, (o, decl.name.clone()), c),
            None => insert(&mut self.fns, decl.name.clone(), c),
        }
    }

    /// The method `name` of the user type `ty` (last path segment), when one declaration decides it.
    pub fn method(&self, ty: &str, name: &str) -> Option<&Callable> {
        self.methods.get(&(strip_generics(ty).to_string(), name.to_string()))?.as_ref()
    }

    /// The free function `name` (last path segment), when one declaration decides it.
    pub fn function(&self, name: &str) -> Option<&Callable> {
        self.fns.get(type_name_leaf(name))?.as_ref()
    }
}

/// A scalar passed by value (`int`, `f32`, `bool`, ...): the callee changes its own copy.
pub fn is_copy_scalar(t: &Type) -> bool {
    !matches!(t, Type::Reference(_)) && crate::type_classification::is_copy_pass_by_value_formal(t)
}
