//! Program-wide symbol tables used while lowering (types of items, never of locals).

use super::alias::Aliases;
use super::ir::ParamMode;
use super::ty::{from_ast, Ty};
use crate::analyzer::{OwnershipMode, SignatureRegistry};
use crate::parser::{EnumVariantData, FunctionDecl, Item, Program};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FnSig {
    pub type_params: Vec<String>,
    pub recv: Option<ParamMode>,
    pub params: Vec<(String, Ty, ParamMode)>,
    pub ret: Ty,
}

#[derive(Debug, Clone)]
pub struct StructInfo {
    pub type_params: Vec<String>,
    pub fields: Vec<(String, Ty)>,
    pub tuple: bool,
}

#[derive(Debug, Clone)]
pub struct VariantInfo {
    pub name: String,
    pub fields: Vec<(String, Ty)>,
    pub named: bool,
}

#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub type_params: Vec<String>,
    pub variants: Vec<VariantInfo>,
}

#[derive(Debug, Default)]
pub struct ItemEnv {
    pub structs: HashMap<String, StructInfo>,
    pub enums: HashMap<String, EnumInfo>,
    pub fns: HashMap<String, FnSig>,
    pub methods: HashMap<(String, String), FnSig>,
    pub trait_methods: HashMap<String, Vec<String>>,
    pub impl_traits: HashMap<String, Vec<String>>,
    pub consts: HashMap<String, Ty>,
    /// Trait name -> names of methods that have a default body.
    pub trait_defaults: HashMap<String, Vec<String>>,
    /// Ignore the semantic layer's `last_use` fact (copy at every owned use). Only for measuring
    /// the fact's effect (`lower_program_without_facts`).
    pub without_facts: bool,
    /// Methods of generic-trait impls (`impl From<X> for E`), which may coexist under one name:
    /// (owner, method) -> [(trait type arguments, emitted name)].
    pub trait_impls: HashMap<(String, String), Vec<(Vec<Ty>, String)>>,
    /// `type` aliases and module names of the program (explicit state for `ty::from_ast`).
    pub aliases: Aliases,
}

/// Emitted name of a method of a generic-trait impl: `from` in `impl From<string> for E` is
/// `from__string`, so several `From` impls of one type coexist. Other methods keep their name.
pub fn impl_method_name(block: &crate::parser::ImplBlock, method: &str, al: &Aliases) -> String {
    match &block.trait_type_args {
        Some(args) if !args.is_empty() => {
            let outer: Vec<String> = block.type_params.iter().map(|t| t.name.clone()).collect();
            let tags: Vec<String> = args.iter().map(|t| ty_tag(&from_ast(t, &outer, al))).collect();
            format!("{method}__{}", tags.join("_"))
        }
        _ => method.to_string(),
    }
}

fn ty_tag(t: &Ty) -> String {
    t.describe().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

/// `Stack<T>` -> `Stack`.
pub fn base_name(n: &str) -> &str {
    n.split('<').next().unwrap_or(n).trim()
}

fn mode_of(m: &OwnershipMode) -> ParamMode {
    match m {
        OwnershipMode::Owned => ParamMode::Value,
        OwnershipMode::Borrowed => ParamMode::Ref,
        OwnershipMode::MutBorrowed => ParamMode::MutRef,
    }
}

/// Resolve parameter modes: analyzer-inferred ownership (by qualified name) wins over the AST.
fn modes_for(key: &str, decl: &FunctionDecl, reg: Option<&SignatureRegistry>) -> (Option<ParamMode>, Vec<ParamMode>) {
    let has_self = decl.parameters.first().map(|p| p.name == "self").unwrap_or(false);
    let sig = reg.and_then(|r| r.get_signature(key));
    let ast_mode = |p: &crate::parser::Parameter| match p.type_ {
        crate::parser::Type::MutableReference(_) => ParamMode::MutRef,
        crate::parser::Type::Reference(_) => ParamMode::Ref,
        _ => ParamMode::Value,
    };
    let inferred: Vec<ParamMode> = match sig {
        Some(s) => s.param_ownership.iter().map(mode_of).collect(),
        None => Vec::new(),
    };
    // Registry vectors cover `self` first when `has_self_receiver` is set.
    let modes: Vec<ParamMode> = decl
        .parameters
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let from_ast = ast_mode(p);
            if from_ast != ParamMode::Value {
                return from_ast;
            }
            inferred.get(i).copied().unwrap_or(ParamMode::Value)
        })
        .collect();
    let recv = if has_self { Some(modes[0]) } else { None };
    (recv, modes)
}

fn sig_of(decl: &FunctionDecl, key: &str, reg: Option<&SignatureRegistry>, outer: &[String], self_ty: Option<&Ty>, al: &Aliases) -> FnSig {
    let mut generics: Vec<String> = outer.to_vec();
    generics.extend(decl.type_params.iter().map(|t| t.name.clone()));
    let (recv, modes) = modes_for(key, decl, reg);
    let params = decl
        .parameters
        .iter()
        .enumerate()
        .filter(|(_, p)| p.name != "self")
        .map(|(i, p)| (p.name.clone(), fix_self(from_ast(&p.type_, &generics, al), self_ty), modes[i]))
        .collect();
    let ret = decl.return_type.as_ref().map(|t| fix_self(from_ast(t, &generics, al), self_ty)).unwrap_or(Ty::Unit);
    FnSig { type_params: generics, recv, params, ret }
}

impl ItemEnv {
    pub fn build(program: &Program, reg: Option<&SignatureRegistry>) -> ItemEnv {
        let mut env = ItemEnv { aliases: Aliases::from_program(program), ..ItemEnv::default() };
        // Traits first so impls can inherit default-method signatures regardless of source order.
        for item in &program.items {
            if matches!(item, Item::Trait { .. }) {
                env.register(item, reg);
            }
        }
        for item in &program.items {
            if !matches!(item, Item::Trait { .. }) {
                env.register(item, reg);
            }
        }
        env
    }

    fn register(&mut self, item: &Item, reg: Option<&SignatureRegistry>) {
        match item {
            Item::Struct { decl, .. } => {
                let tp: Vec<String> = decl.type_params.iter().map(|t| t.name.clone()).collect();
                let (fields, tuple) = match &decl.tuple_fields {
                    Some(ts) => (ts.iter().enumerate().map(|(i, t)| (i.to_string(), from_ast(t, &tp, &self.aliases))).collect(), true),
                    None => (decl.fields.iter().map(|f| (f.name.clone(), from_ast(&f.field_type, &tp, &self.aliases))).collect(), false),
                };
                self.structs.insert(decl.name.clone(), StructInfo { type_params: tp, fields, tuple });
            }
            Item::Enum { decl, .. } => {
                let tp: Vec<String> = decl.type_params.iter().map(|t| t.name.clone()).collect();
                let variants = decl.variants.iter().map(|v| variant_info(v, &tp, &self.aliases)).collect();
                self.enums.insert(decl.name.clone(), EnumInfo { type_params: tp, variants });
            }
            Item::Function { decl, .. } => {
                self.fns.insert(decl.name.clone(), sig_of(decl, &decl.name, reg, &[], None, &self.aliases));
            }
            Item::Impl { block, .. } => self.register_impl(block, reg),
            Item::Trait { decl, .. } => {
                let names = decl.methods.iter().map(|m| m.name.clone()).collect();
                self.trait_methods.insert(decl.name.clone(), names);
                let defaults = decl.methods.iter().filter(|m| m.body.is_some()).map(|m| m.name.clone()).collect();
                self.trait_defaults.insert(decl.name.clone(), defaults);
                for m in &decl.methods {
                    self.methods.insert((decl.name.clone(), m.name.clone()), trait_sig(m, &decl.name, &self.aliases));
                }
            }
            Item::Const { name, type_, .. } | Item::Static { name, type_, .. } => {
                self.consts.insert(name.clone(), from_ast(type_, &[], &self.aliases));
            }
            Item::Mod { items, .. } => {
                for i in items {
                    self.register(i, reg);
                }
            }
            _ => {}
        }
    }

    fn register_impl(&mut self, block: &crate::parser::ImplBlock, reg: Option<&SignatureRegistry>) {
        let outer: Vec<String> = block.type_params.iter().map(|t| t.name.clone()).collect();
        let owner = base_name(&block.type_name).to_string();
        if let Some(t) = &block.trait_name {
            self.impl_traits.entry(owner.clone()).or_default().push(t.clone());
        }
        let self_ty = Ty::Adt(owner.clone(), outer.iter().map(|n| Ty::Param(n.clone())).collect());
        let trait_args: Option<Vec<Ty>> = block.trait_type_args.as_ref().filter(|a| !a.is_empty()).map(|a| a.iter().map(|t| from_ast(t, &outer, &self.aliases)).collect());
        for f in &block.functions {
            let key = format!("{}::{}", block.type_name, f.name);
            let name = impl_method_name(block, &f.name, &self.aliases);
            self.methods.insert((owner.clone(), name.clone()), sig_of(f, &key, reg, &outer, Some(&self_ty), &self.aliases));
            if let Some(args) = &trait_args {
                self.trait_impls.entry((owner.clone(), f.name.clone())).or_default().push((args.clone(), name));
            }
        }
        if let Some(t) = &block.trait_name {
            for d in self.trait_defaults.get(t).cloned().unwrap_or_default() {
                if !block.functions.iter().any(|f| f.name == d) {
                    if let Some(sig) = self.methods.get(&(t.clone(), d.clone())).cloned() {
                        let mut sig = sig;
                        let fix = |ty: &Ty| ty.replace_self(&self_ty);
                        sig.params = sig.params.iter().map(|(n, ty, m)| (n.clone(), fix(ty), *m)).collect();
                        sig.ret = fix(&sig.ret);
                        self.methods.insert((owner.clone(), d), sig);
                    }
                }
            }
        }
    }

    pub fn method(&self, owner: &str, name: &str) -> Option<&FnSig> {
        self.methods.get(&(owner.to_string(), name.to_string()))
    }

    /// Variant lookup `Enum::Variant`.
    pub fn variant(&self, enum_name: &str, variant: &str) -> Option<&VariantInfo> {
        self.enums.get(enum_name)?.variants.iter().find(|v| v.name == variant)
    }

    /// The enum that declares a bare variant name (for `use Enum::*` style code).
    pub fn enum_of_variant(&self, variant: &str) -> Option<&str> {
        self.enums.iter().find(|(_, e)| e.variants.iter().any(|v| v.name == variant)).map(|(n, _)| n.as_str())
    }
}

pub fn fix_self(t: Ty, self_ty: Option<&Ty>) -> Ty {
    match self_ty {
        Some(s) => t.replace_self(s),
        None => t,
    }
}

fn variant_info(v: &crate::parser::EnumVariant, tp: &[String], al: &Aliases) -> VariantInfo {
    match &v.data {
        EnumVariantData::Unit => VariantInfo { name: v.name.clone(), fields: vec![], named: false },
        EnumVariantData::Tuple(ts) => VariantInfo {
            name: v.name.clone(),
            fields: ts.iter().enumerate().map(|(i, t)| (i.to_string(), from_ast(t, tp, al))).collect(),
            named: false,
        },
        EnumVariantData::Struct(fs) => VariantInfo {
            name: v.name.clone(),
            fields: fs.iter().map(|(n, t)| (n.clone(), from_ast(t, tp, al))).collect(),
            named: true,
        },
    }
}

fn trait_sig(m: &crate::parser::TraitMethod, trait_name: &str, al: &Aliases) -> FnSig {
    let has_self = m.parameters.first().map(|p| p.name == "self").unwrap_or(false);
    let self_ty = Ty::Adt(trait_name.to_string(), vec![]);
    let mode = |p: &crate::parser::Parameter| match p.type_ {
        crate::parser::Type::MutableReference(_) => ParamMode::MutRef,
        crate::parser::Type::Reference(_) => ParamMode::Ref,
        _ => ParamMode::Ref,
    };
    FnSig {
        type_params: vec![],
        recv: if has_self { Some(mode(&m.parameters[0])) } else { None },
        params: m.parameters.iter().filter(|p| p.name != "self").map(|p| (p.name.clone(), fix_self(from_ast(&p.type_, &[], al), Some(&self_ty)), ParamMode::Value)).collect(),
        ret: m.return_type.as_ref().map(|t| fix_self(from_ast(t, &[], al), Some(&self_ty))).unwrap_or(Ty::Unit),
    }
}
