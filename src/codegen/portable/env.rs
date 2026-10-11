//! Program-wide symbol tables used while lowering (types of items, never of locals).

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

fn sig_of(decl: &FunctionDecl, key: &str, reg: Option<&SignatureRegistry>, outer: &[String], self_ty: Option<&Ty>) -> FnSig {
    let mut generics: Vec<String> = outer.to_vec();
    generics.extend(decl.type_params.iter().map(|t| t.name.clone()));
    let (recv, modes) = modes_for(key, decl, reg);
    let params = decl
        .parameters
        .iter()
        .enumerate()
        .filter(|(_, p)| p.name != "self")
        .map(|(i, p)| (p.name.clone(), fix_self(from_ast(&p.type_, &generics), self_ty), modes[i]))
        .collect();
    let ret = decl.return_type.as_ref().map(|t| fix_self(from_ast(t, &generics), self_ty)).unwrap_or(Ty::Unit);
    FnSig { type_params: generics, recv, params, ret }
}

impl ItemEnv {
    pub fn build(program: &Program, reg: Option<&SignatureRegistry>) -> ItemEnv {
        let mut env = ItemEnv::default();
        for item in &program.items {
            env.register(item, reg);
        }
        env
    }

    fn register(&mut self, item: &Item, reg: Option<&SignatureRegistry>) {
        match item {
            Item::Struct { decl, .. } => {
                let tp: Vec<String> = decl.type_params.iter().map(|t| t.name.clone()).collect();
                let (fields, tuple) = match &decl.tuple_fields {
                    Some(ts) => (ts.iter().enumerate().map(|(i, t)| (i.to_string(), from_ast(t, &tp))).collect(), true),
                    None => (decl.fields.iter().map(|f| (f.name.clone(), from_ast(&f.field_type, &tp))).collect(), false),
                };
                self.structs.insert(decl.name.clone(), StructInfo { type_params: tp, fields, tuple });
            }
            Item::Enum { decl, .. } => {
                let tp: Vec<String> = decl.type_params.iter().map(|t| t.name.clone()).collect();
                let variants = decl.variants.iter().map(|v| variant_info(v, &tp)).collect();
                self.enums.insert(decl.name.clone(), EnumInfo { type_params: tp, variants });
            }
            Item::Function { decl, .. } => {
                self.fns.insert(decl.name.clone(), sig_of(decl, &decl.name, reg, &[], None));
            }
            Item::Impl { block, .. } => self.register_impl(block, reg),
            Item::Trait { decl, .. } => {
                let names = decl.methods.iter().map(|m| m.name.clone()).collect();
                self.trait_methods.insert(decl.name.clone(), names);
                for m in &decl.methods {
                    self.methods.insert((decl.name.clone(), m.name.clone()), trait_sig(m, &decl.name));
                }
            }
            Item::Const { name, type_, .. } | Item::Static { name, type_, .. } => {
                self.consts.insert(name.clone(), from_ast(type_, &[]));
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
        if let Some(t) = &block.trait_name {
            self.impl_traits.entry(block.type_name.clone()).or_default().push(t.clone());
        }
        for f in &block.functions {
            let key = format!("{}::{}", block.type_name, f.name);
            let self_ty = Ty::Adt(block.type_name.clone(), outer.iter().map(|n| Ty::Param(n.clone())).collect());
            self.methods.insert((block.type_name.clone(), f.name.clone()), sig_of(f, &key, reg, &outer, Some(&self_ty)));
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

fn variant_info(v: &crate::parser::EnumVariant, tp: &[String]) -> VariantInfo {
    match &v.data {
        EnumVariantData::Unit => VariantInfo { name: v.name.clone(), fields: vec![], named: false },
        EnumVariantData::Tuple(ts) => VariantInfo {
            name: v.name.clone(),
            fields: ts.iter().enumerate().map(|(i, t)| (i.to_string(), from_ast(t, tp))).collect(),
            named: false,
        },
        EnumVariantData::Struct(fs) => VariantInfo {
            name: v.name.clone(),
            fields: fs.iter().map(|(n, t)| (n.clone(), from_ast(t, tp))).collect(),
            named: true,
        },
    }
}

fn trait_sig(m: &crate::parser::TraitMethod, trait_name: &str) -> FnSig {
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
        params: m.parameters.iter().filter(|p| p.name != "self").map(|p| (p.name.clone(), fix_self(from_ast(&p.type_, &[]), Some(&self_ty)), ParamMode::Value)).collect(),
        ret: m.return_type.as_ref().map(|t| fix_self(from_ast(t, &[]), Some(&self_ty))).unwrap_or(Ty::Unit),
    }
}
