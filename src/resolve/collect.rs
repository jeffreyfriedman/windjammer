//! Phase 1: walk every file's items, allocate [`DefId`]s, build module scopes and body list.

use super::def::{Def, DefId, DefKind, DefTable};
use super::res::Diagnostic;
use super::uses::{expand_use, UseDecl};
use super::ParsedFile;
use crate::parser::ast::core::{
    EnumDecl, Expression, FunctionDecl, ImplBlock, Item, Parameter, Statement, StructDecl,
    TraitDecl,
};
use crate::source_map::Location;
use std::collections::HashMap;

/// Names and imports visible in one module.
#[derive(Debug, Default)]
pub struct ModuleScope {
    pub items: HashMap<String, Vec<DefId>>,
    pub uses: Vec<UseDecl>,
    pub parent: Option<DefId>,
}

/// An `impl` block, with its self type / trait filled in by the link phase.
#[derive(Debug)]
pub struct ImplInfo {
    pub def: DefId,
    pub module: DefId,
    pub type_name: String,
    pub trait_name: Option<String>,
    pub generics: Vec<String>,
    pub self_def: Option<DefId>,
    pub trait_def: Option<DefId>,
    pub fns: Vec<DefId>,
}

pub enum BodyCode<'a> {
    Stmts(&'a [&'a Statement<'a>]),
    Expr(&'a Expression<'a>),
}

/// Executable code to walk for calls, with the context it resolves in.
pub struct Body<'a> {
    pub owner: Option<DefId>,
    pub module: DefId,
    pub impl_idx: Option<usize>,
    pub trait_def: Option<DefId>,
    pub generics: Vec<String>,
    pub params: &'a [Parameter<'a>],
    pub code: BodyCode<'a>,
}

/// Everything phase 1 produces; extended (impl links) by [`super::lookup`].
pub struct Collected<'a> {
    pub defs: DefTable,
    pub root: DefId,
    pub modules: HashMap<DefId, ModuleScope>,
    pub impls: Vec<ImplInfo>,
    pub bodies: Vec<Body<'a>>,
    /// Owner (enum/trait) -> member defs (variants / trait methods).
    pub members: HashMap<DefId, Vec<DefId>>,
    /// Type def -> indices into `impls`.
    pub impls_of: HashMap<DefId, Vec<usize>>,
    /// Method name -> all `Method`/`TraitMethod` defs.
    pub methods_by_name: HashMap<String, Vec<DefId>>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn collect<'a>(files: &[ParsedFile<'a>]) -> Collected<'a> {
    let mut defs = DefTable::default();
    let root = defs.alloc(blank_def(DefKind::Module, "", "", DefId(0), None));
    let mut c = Collected {
        defs,
        root,
        modules: HashMap::from([(root, ModuleScope::default())]),
        impls: Vec::new(),
        bodies: Vec::new(),
        members: HashMap::new(),
        impls_of: HashMap::new(),
        methods_by_name: HashMap::new(),
        diagnostics: Vec::new(),
    };
    for f in files {
        let m = c.module_for_path(&f.module_path);
        c.collect_items(m, &f.program.items);
    }
    c
}

pub(crate) fn blank_def(
    kind: DefKind,
    name: &str,
    path: &str,
    module: DefId,
    location: Option<Location>,
) -> Def {
    Def {
        id: DefId(0),
        kind,
        name: name.to_string(),
        path: path.to_string(),
        module,
        is_pub: true,
        has_self: false,
        arity: 0,
        is_ctor: false,
        alias_target: None,
        location,
    }
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}::{name}")
    }
}

impl<'a> Collected<'a> {
    pub fn module_path_of(&self, module: DefId) -> &str {
        &self.defs.get(module).path
    }

    fn module_for_path(&mut self, path: &[String]) -> DefId {
        let mut cur = self.root;
        for seg in path {
            cur = self.child_module(cur, seg, true, None);
        }
        cur
    }

    fn child_module(
        &mut self,
        parent: DefId,
        name: &str,
        is_pub: bool,
        loc: Option<Location>,
    ) -> DefId {
        let existing = self.modules[&parent].items.get(name).and_then(|v| {
            v.iter()
                .copied()
                .find(|d| self.defs.get(*d).kind == DefKind::Module)
        });
        if let Some(id) = existing {
            return id;
        }
        let path = join(self.module_path_of(parent), name);
        let mut d = blank_def(DefKind::Module, name, &path, parent, loc);
        d.is_pub = is_pub;
        let id = self.defs.alloc(d);
        self.modules.insert(
            id,
            ModuleScope {
                parent: Some(parent),
                ..Default::default()
            },
        );
        self.add_name(parent, name, id);
        id
    }

    fn add_name(&mut self, module: DefId, name: &str, id: DefId) {
        self.modules
            .get_mut(&module)
            .expect("module scope")
            .items
            .entry(name.to_string())
            .or_default()
            .push(id);
    }

    fn alloc_in(
        &mut self,
        module: DefId,
        kind: DefKind,
        name: &str,
        prefix: &str,
        loc: &Option<Location>,
    ) -> DefId {
        let d = blank_def(kind, name, &join(prefix, name), module, loc.clone());
        self.defs.alloc(d)
    }

    fn collect_items(&mut self, module: DefId, items: &'a [Item<'a>]) {
        for item in items {
            self.collect_item(module, item);
        }
    }

    fn collect_item(&mut self, module: DefId, item: &'a Item<'a>) {
        let mp = self.module_path_of(module).to_string();
        match item {
            Item::Function { decl, location } => self.collect_free_fn(module, &mp, decl, location),
            Item::Struct { decl, location } => self.collect_struct(module, &mp, decl, location),
            Item::Enum { decl, location } => self.collect_enum(module, &mp, decl, location),
            Item::Trait { decl, location } => self.collect_trait(module, &mp, decl, location),
            Item::Impl { block, location } => self.collect_impl(module, &mp, block, location),
            Item::Const {
                name,
                value,
                location,
                is_pub,
                ..
            } => {
                let id = self.alloc_in(module, DefKind::Const, name, &mp, location);
                self.defs_set_pub(id, *is_pub);
                self.add_name(module, name, id);
                self.push_expr_body(module, id, value);
            }
            Item::Static {
                name,
                value,
                location,
                ..
            } => {
                let id = self.alloc_in(module, DefKind::Static, name, &mp, location);
                self.add_name(module, name, id);
                self.push_expr_body(module, id, value);
            }
            Item::ExternLet {
                name,
                location,
                is_pub,
                ..
            } => {
                let id = self.alloc_in(module, DefKind::Static, name, &mp, location);
                self.defs_set_pub(id, *is_pub);
                self.add_name(module, name, id);
            }
            Item::TypeAlias {
                name,
                target,
                is_pub,
                location,
            } => {
                let id = self.alloc_in(module, DefKind::TypeAlias, name, &mp, location);
                self.defs_set_pub(id, *is_pub);
                self.set_alias_target(id, target);
                self.add_name(module, name, id);
            }
            Item::Use {
                path,
                alias,
                is_pub,
                location,
            } => {
                let decls = expand_use(path, alias, *is_pub, location);
                self.modules
                    .get_mut(&module)
                    .expect("scope")
                    .uses
                    .extend(decls);
            }
            Item::Mod {
                name,
                items,
                is_public,
                location,
            } => {
                let child = self.child_module(module, name, *is_public, location.clone());
                self.collect_items(child, items);
            }
            Item::Macro { expr, .. } => self.push_module_expr(module, expr),
            Item::BoundAlias { .. } => {}
        }
    }

    fn defs_set_pub(&mut self, id: DefId, is_pub: bool) {
        self.defs_mut(id).is_pub = is_pub;
    }

    fn defs_mut(&mut self, id: DefId) -> &mut Def {
        self.defs.get_mut(id)
    }

    fn set_alias_target(&mut self, id: DefId, target: &crate::parser::ast::types::Type) {
        use crate::parser::ast::types::Type;
        let name = match target {
            Type::Custom(n) | Type::Parameterized(n, _) => Some(n.clone()),
            _ => None,
        };
        self.defs_mut(id).alias_target = name;
    }

    fn push_expr_body(&mut self, module: DefId, owner: DefId, expr: &'a Expression<'a>) {
        self.bodies.push(Body {
            owner: Some(owner),
            module,
            impl_idx: None,
            trait_def: None,
            generics: Vec::new(),
            params: &[],
            code: BodyCode::Expr(expr),
        });
    }

    fn push_module_expr(&mut self, module: DefId, expr: &'a Expression<'a>) {
        self.bodies.push(Body {
            owner: None,
            module,
            impl_idx: None,
            trait_def: None,
            generics: Vec::new(),
            params: &[],
            code: BodyCode::Expr(expr),
        });
    }

    fn collect_free_fn(
        &mut self,
        module: DefId,
        mp: &str,
        decl: &'a FunctionDecl<'a>,
        loc: &Option<Location>,
    ) {
        let kind = if decl.is_extern {
            DefKind::ExternFn
        } else {
            DefKind::Fn
        };
        let id = self.alloc_in(module, kind, &decl.name, mp, loc);
        self.set_fn_shape(id, decl);
        self.add_name(module, &decl.name, id);
        self.push_fn_body(module, id, decl, None, None, Vec::new());
    }

    fn set_fn_shape(&mut self, id: DefId, decl: &FunctionDecl<'_>) {
        let has_self = decl.parameters.first().is_some_and(|p| p.name == "self");
        let d = self.defs_mut(id);
        d.is_pub = decl.is_pub;
        d.has_self = has_self;
        d.arity = decl.parameters.len() - usize::from(has_self);
    }

    fn push_fn_body(
        &mut self,
        module: DefId,
        owner: DefId,
        decl: &'a FunctionDecl<'a>,
        impl_idx: Option<usize>,
        trait_def: Option<DefId>,
        mut generics: Vec<String>,
    ) {
        generics.extend(decl.type_params.iter().map(|t| t.name.clone()));
        self.bodies.push(Body {
            owner: Some(owner),
            module,
            impl_idx,
            trait_def,
            generics,
            params: &decl.parameters,
            code: BodyCode::Stmts(&decl.body),
        });
    }

    fn collect_struct(
        &mut self,
        module: DefId,
        mp: &str,
        decl: &StructDecl<'_>,
        loc: &Option<Location>,
    ) {
        let id = self.alloc_in(module, DefKind::Struct, &decl.name, mp, loc);
        let d = self.defs_mut(id);
        d.is_pub = decl.is_pub;
        d.is_ctor = decl.tuple_fields.is_some();
        self.add_name(module, &decl.name, id);
    }

    fn collect_enum(&mut self, module: DefId, mp: &str, decl: &EnumDecl, loc: &Option<Location>) {
        let id = self.alloc_in(module, DefKind::Enum, &decl.name, mp, loc);
        self.defs_mut(id).is_pub = decl.is_pub;
        self.add_name(module, &decl.name, id);
        let prefix = join(mp, &decl.name);
        for v in &decl.variants {
            let vid = self.alloc_in(
                module,
                DefKind::Variant { owner: id },
                &v.name,
                &prefix,
                loc,
            );
            self.members.entry(id).or_default().push(vid);
        }
    }

    fn collect_trait(
        &mut self,
        module: DefId,
        mp: &str,
        decl: &'a TraitDecl<'a>,
        loc: &Option<Location>,
    ) {
        let id = self.alloc_in(module, DefKind::Trait, &decl.name, mp, loc);
        self.add_name(module, &decl.name, id);
        let prefix = join(mp, &decl.name);
        for m in &decl.methods {
            let mid = self.alloc_in(
                module,
                DefKind::TraitMethod { owner: id },
                &m.name,
                &prefix,
                loc,
            );
            let has_self = m.parameters.first().is_some_and(|p| p.name == "self");
            let d = self.defs_mut(mid);
            d.has_self = has_self;
            d.arity = m.parameters.len() - usize::from(has_self);
            self.members.entry(id).or_default().push(mid);
            self.methods_by_name
                .entry(m.name.clone())
                .or_default()
                .push(mid);
            if let Some(body) = &m.body {
                self.bodies.push(Body {
                    owner: Some(mid),
                    module,
                    impl_idx: None,
                    trait_def: Some(id),
                    generics: decl.generics.clone(),
                    params: &m.parameters,
                    code: BodyCode::Stmts(body),
                });
            }
        }
    }

    fn collect_impl(
        &mut self,
        module: DefId,
        mp: &str,
        block: &'a ImplBlock<'a>,
        loc: &Option<Location>,
    ) {
        let ty = base_type_name(&block.type_name);
        let (label, prefix) = match &block.trait_name {
            Some(t) => (format!("<impl {t} for {ty}>"), format!("<{ty} as {t}>")),
            None => (format!("<impl {ty}>"), ty.to_string()),
        };
        let impl_def = self.alloc_in(module, DefKind::Impl, &label, mp, loc);
        let idx = self.impls.len();
        let generics: Vec<String> = block.type_params.iter().map(|t| t.name.clone()).collect();
        self.impls.push(ImplInfo {
            def: impl_def,
            module,
            type_name: ty.to_string(),
            trait_name: block.trait_name.clone(),
            generics: generics.clone(),
            self_def: None,
            trait_def: None,
            fns: Vec::new(),
        });
        let full_prefix = join(mp, &prefix);
        for f in &block.functions {
            self.collect_impl_fn(module, idx, impl_def, &full_prefix, f, &generics, loc);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_impl_fn(
        &mut self,
        module: DefId,
        idx: usize,
        impl_def: DefId,
        prefix: &str,
        f: &'a FunctionDecl<'a>,
        generics: &[String],
        loc: &Option<Location>,
    ) {
        let has_self = f.parameters.first().is_some_and(|p| p.name == "self");
        let kind = if has_self {
            DefKind::Method { owner: impl_def }
        } else {
            DefKind::AssocFn { owner: impl_def }
        };
        let id = self.alloc_in(module, kind, &f.name, prefix, loc);
        self.set_fn_shape(id, f);
        self.impls[idx].fns.push(id);
        if has_self {
            self.methods_by_name
                .entry(f.name.clone())
                .or_default()
                .push(id);
        }
        self.push_fn_body(module, id, f, Some(idx), None, generics.to_vec());
    }
}

/// `Vec<T>` -> `Vec`; `a::B<T>` -> `a::B`.
pub(crate) fn base_type_name(name: &str) -> &str {
    name.split('<').next().unwrap_or(name).trim()
}
