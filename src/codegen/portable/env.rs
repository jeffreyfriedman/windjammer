//! Program-wide symbol tables used while lowering (types of items, never of locals).

use super::alias::Aliases;
use super::ir::{PExpr, ParamMode};
use super::ty::{from_ast, Ty};
use crate::analyzer::{OwnershipMode, SignatureRegistry};
use crate::parser::{EnumVariantData, FunctionDecl, Item, Program};
use crate::ir::stdlib_tables::CoreSurface;
use crate::stdlib_surface::lowering::{CoreType, LoweringTable};
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

/// The typed std surface (`std/rust_core.wji`) and its lowering table: one parse per process, shared.
#[derive(Debug, Clone, Copy)]
pub struct StdSurface {
    pub core: &'static CoreSurface,
    pub lowering: &'static LoweringTable,
}

impl StdSurface {
    pub fn shared() -> StdSurface {
        StdSurface { core: crate::ir::stdlib_tables::shared_core_surface(), lowering: crate::ir::stdlib_tables::shared_lowering() }
    }

    /// The result type the declaration in `std/rust_core.wji` gives `recv.name(args)` for a float, integer, char, string,
    /// `Option` or `Result` receiver, `T` being the receiver's own type (an `Option<T>`'s `T`, a `Result<T, E>`'s `T` and
    /// `E`); `None` when the surface declares no such method. A method generic is bound from the arguments (`map<U>`
    /// takes `U` from the closure's result type) and then from the expected result `want` (`parse<F>`: `Result<i32, string>`
    /// binds `F = i32`).
    pub fn declared_ret(&self, recv: &Ty, name: &str, args: &[PExpr], want: Option<&Ty>) -> Option<Ty> {
        let (receiver, core_type) = match recv {
            Ty::F64 => (crate::parser::Type::Custom("f64".to_string()), CoreType::Float),
            Ty::F32 => (crate::parser::Type::Custom("f32".to_string()), CoreType::Float),
            Ty::Int(k) => (crate::parser::Type::Custom(k.name().to_string()), CoreType::Integer),
            Ty::Char => return self.declared_on_trait(CoreType::Char, &[], name, args, want),
            Ty::Str => return self.declared_on_trait(CoreType::String, &[], name, args, want),
            Ty::Option(t) => return self.declared_on_trait(CoreType::Option, &[(**t).clone()], name, args, want),
            Ty::Result(t, e) => return self.declared_on_trait(CoreType::Result, &[(**t).clone(), (**e).clone()], name, args, want),
            _ => return None,
        };
        // Only a method with a row: the width traits (`f32::to_bits`) are declared too, but have no lowering yet.
        self.lowering.row(core_type, name)?;
        let ret = self.core.method_signature(&receiver, name, &[])?.ret?;
        Some(from_ast(&ret, &[], &Aliases::default()))
    }

    /// The expected type of argument `index` of `recv.name(..)` as the declaration gives it (a string's `contains` takes a
    /// `&str`, `repeat` a `usize`; an `Option<T>`'s `map` takes a `fn(T) -> ?`, its `unwrap_or` a `T`); `None` when the
    /// receiver's type has no rows, the method no such parameter, or the parameter is a bare method generic.
    pub fn declared_arg(&self, recv: &Ty, name: &str, index: usize) -> Option<Ty> {
        let (core_type, recv_args) = match recv {
            Ty::Str => {
                self.lowering.row(CoreType::String, name)?;
                let (_, generics, params, _) = self.core.declared(CoreType::String.trait_name(), name)?;
                return Some(from_ast(params.get(index)?, &generics, &Aliases::default()));
            }
            Ty::Option(t) => (CoreType::Option, vec![(**t).clone()]),
            Ty::Result(t, e) => (CoreType::Result, vec![(**t).clone(), (**e).clone()]),
            _ => return None,
        };
        self.lowering.row(core_type, name)?;
        let (type_generics, method_generics, params, _) = self.core.declared(core_type.trait_name(), name)?;
        let generics: Vec<String> = type_generics.iter().chain(&method_generics).cloned().collect();
        let want = from_ast(params.get(index)?, &generics, &Aliases::default());
        if matches!(&want, Ty::Param(n) if method_generics.contains(n)) {
            return None;
        }
        // the receiver's generics are known; a method generic is not yet (the closure's result): `Unknown`
        let mut known = recv_args;
        known.resize(generics.len(), Ty::Unknown);
        Some(want.subst(&generics, &known))
    }

    /// The declared result of `name` on a type of `std/rust_core.wji` whose generics (`recv_args` for the trait's, then
    /// the method's own) are instantiated: the receiver's, then the arguments' types (`map<U>(f: fn(T) -> U)` binds `U` to
    /// the closure's result), then the expected type.
    fn declared_on_trait(&self, core_type: CoreType, recv_args: &[Ty], name: &str, args: &[PExpr], want: Option<&Ty>) -> Option<Ty> {
        self.lowering.row(core_type, name)?;
        let (type_generics, method_generics, params, ret) = self.core.declared(core_type.trait_name(), name)?;
        let Some(ret) = ret else { return Some(Ty::Unit) };
        let generics: Vec<String> = type_generics.iter().chain(&method_generics).cloned().collect();
        let ty = match from_ast(&ret, &generics, &Aliases::default()) {
            // `Iterator<T>` is the portable type of a lazy sequence
            Ty::Adt(n, mut args) if n == crate::ir::stdlib_tables::ITERATOR && args.len() == 1 => Ty::Iter(Box::new(args.remove(0))),
            other => other,
        };
        if generics.is_empty() {
            return Some(ty);
        }
        let mut bound: Vec<Option<Ty>> = vec![None; generics.len()];
        for (slot, t) in bound.iter_mut().zip(recv_args) {
            *slot = Some(t.clone());
        }
        for (param, arg) in params.iter().zip(args) {
            bind_generics(&from_ast(param, &generics, &Aliases::default()), &arg.ty, &generics, &mut bound, true);
        }
        if let Some(want) = want {
            bind_generics(&ty, want, &generics, &mut bound, false);
        }
        let all: Option<Vec<Ty>> = bound.into_iter().collect();
        Some(ty.subst(&generics, &all?))
    }
}

/// Bind the generics that `pattern` mentions to the parts of `found` in the same places (`Result<F, string>` against
/// `Result<i32, string>` binds `F = i32`; `fn(T) -> U` against a closure type binds `U` to its result). The first binding
/// of a generic wins. `lenient`: an argument whose type is not known yet (`Unknown`) binds every generic the pattern mentions
/// to `Unknown` (a closure whose result is not known types the call as not known, it does not make it untypable).
fn bind_generics(pattern: &Ty, found: &Ty, generics: &[String], bound: &mut [Option<Ty>], lenient: bool) {
    if lenient && *found == Ty::Unknown && !matches!(pattern, Ty::Param(_)) {
        let mut names = Vec::new();
        mentioned(pattern, &mut names);
        for n in names {
            if let Some(i) = generics.iter().position(|g| *g == n) {
                bound[i].get_or_insert(Ty::Unknown);
            }
        }
        return;
    }
    match (pattern, found) {
        (Ty::Param(n), w) => {
            if let Some(i) = generics.iter().position(|g| g == n) {
                bound[i].get_or_insert_with(|| w.clone());
            }
        }
        (Ty::Result(a, b), Ty::Result(wa, wb)) => {
            bind_generics(a, wa, generics, bound, lenient);
            bind_generics(b, wb, generics, bound, lenient);
        }
        (Ty::Option(a), Ty::Option(wa)) | (Ty::Vec(a), Ty::Vec(wa)) | (Ty::Iter(a), Ty::Iter(wa)) => bind_generics(a, wa, generics, bound, lenient),
        (Ty::Fn(pa, pr), Ty::Fn(wa, wr)) => {
            for (p, w) in pa.iter().zip(wa) {
                bind_generics(p, w, generics, bound, lenient);
            }
            bind_generics(pr, wr, generics, bound, lenient);
        }
        _ => {}
    }
}

/// The generic names a type mentions.
fn mentioned(t: &Ty, out: &mut Vec<String>) {
    match t {
        Ty::Param(n) => out.push(n.clone()),
        Ty::Vec(a) | Ty::Set(a) | Ty::Iter(a) | Ty::Range(a) | Ty::Option(a) => mentioned(a, out),
        Ty::Map(a, b) | Ty::Result(a, b) => {
            mentioned(a, out);
            mentioned(b, out);
        }
        Ty::Tuple(ts) | Ty::Adt(_, ts) => ts.iter().for_each(|t| mentioned(t, out)),
        Ty::Fn(ps, r) => {
            ps.iter().for_each(|t| mentioned(t, out));
            mentioned(r, out);
        }
        _ => {}
    }
}

#[derive(Debug)]
pub struct ItemEnv {
    /// The std methods' types and lowering rules (`builtins::ret_ty` reads signatures, the emitters read the rules).
    pub surface: StdSurface,
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
        Self::build_with(program, reg, StdSurface::shared())
    }

    pub fn build_with(program: &Program, reg: Option<&SignatureRegistry>, surface: StdSurface) -> ItemEnv {
        let mut env = ItemEnv {
            surface,
            structs: HashMap::new(),
            enums: HashMap::new(),
            fns: HashMap::new(),
            methods: HashMap::new(),
            trait_methods: HashMap::new(),
            impl_traits: HashMap::new(),
            consts: HashMap::new(),
            trait_defaults: HashMap::new(),
            without_facts: false,
            trait_impls: HashMap::new(),
            aliases: Aliases::from_program(program),
        };
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

#[cfg(test)]
mod surface_tests {
    use super::*;
    use crate::codegen::portable::ty::IntKind;

    fn parsed(ok: Ty) -> Ty {
        Ty::Result(Box::new(ok), Box::new(Ty::Str))
    }

    #[test]
    fn a_string_or_char_method_has_the_type_its_declaration_gives() {
        let s = StdSurface::shared();
        assert_eq!(s.declared_ret(&Ty::Str, "len", &[], None), Some(Ty::usize()));
        assert_eq!(s.declared_ret(&Ty::Str, "trim", &[], None), Some(Ty::Str));
        assert_eq!(s.declared_ret(&Ty::Str, "chars", &[], None), Some(Ty::Iter(Box::new(Ty::Char))));
        assert_eq!(s.declared_ret(&Ty::Str, "char_indices", &[], None), Some(Ty::Iter(Box::new(Ty::Tuple(vec![Ty::usize(), Ty::Char])))));
        assert_eq!(s.declared_ret(&Ty::Str, "find", &[], None), Some(Ty::Option(Box::new(Ty::usize()))));
        assert_eq!(s.declared_ret(&Ty::Str, "push", &[], None), Some(Ty::Unit), "no declared result is a unit");
        assert_eq!(s.declared_ret(&Ty::Char, "to_digit", &[], None), Some(Ty::Option(Box::new(Ty::Int(IntKind::U32)))));
        assert_eq!(s.declared_ret(&Ty::Char, "to_uppercase", &[], None), Some(Ty::Str));
        assert_eq!(s.declared_ret(&Ty::Char, "is_alphabetic", &[], None), Some(Ty::Bool));
    }

    #[test]
    fn a_removed_spelling_or_an_undeclared_method_has_no_type() {
        let s = StdSurface::shared();
        for name in ["as_str", "into_string", "substring", "to_ascii_uppercase", "as_bytes"] {
            assert_eq!(s.declared_ret(&Ty::Str, name, &[], None), None, "{name}");
        }
        for name in ["is_ascii_digit", "is_numeric", "len_utf8", "to_ascii_lowercase"] {
            assert_eq!(s.declared_ret(&Ty::Char, name, &[], None), None, "{name}");
        }
    }

    #[test]
    fn a_method_generic_takes_its_type_from_the_expected_result() {
        let s = StdSurface::shared();
        assert_eq!(s.declared_ret(&Ty::Str, "parse", &[], Some(&parsed(Ty::Int(IntKind::U8)))), Some(parsed(Ty::Int(IntKind::U8))));
        assert_eq!(s.declared_ret(&Ty::Str, "parse", &[], Some(&parsed(Ty::F32))), Some(parsed(Ty::F32)));
        assert_eq!(s.declared_ret(&Ty::Str, "parse", &[], None), None, "no expected type, no target");
        assert_eq!(s.declared_ret(&Ty::Str, "parse", &[], Some(&Ty::Bool)), None);
    }

    #[test]
    fn a_string_argument_has_the_type_its_parameter_declares() {
        let s = StdSurface::shared();
        assert_eq!(s.declared_arg(&Ty::Str, "contains", 0), Some(Ty::Str));
        assert_eq!(s.declared_arg(&Ty::Str, "replace", 1), Some(Ty::Str));
        assert_eq!(s.declared_arg(&Ty::Str, "repeat", 0), Some(Ty::usize()));
        assert_eq!(s.declared_arg(&Ty::Str, "push", 0), Some(Ty::Char));
        assert_eq!(s.declared_arg(&Ty::Str, "len", 0), None);
        assert_eq!(s.declared_arg(&Ty::Char, "to_digit", 0), None, "a char argument keeps the numeric rule");
    }

    fn arg_of(ty: Ty) -> PExpr {
        PExpr::new(ty, super::ir::PK::None)
    }

    fn closure(ret: Ty) -> PExpr {
        arg_of(Ty::Fn(vec![Ty::Int(IntKind::I32)], Box::new(ret)))
    }

    #[test]
    fn an_option_or_result_method_has_the_type_its_declaration_gives() {
        let s = StdSurface::shared();
        let int = Ty::Int(IntKind::I32);
        let opt = Ty::Option(Box::new(int.clone()));
        let res = Ty::Result(Box::new(int.clone()), Box::new(Ty::Str));
        let ret = |recv: &Ty, name: &str, args: &[PExpr]| s.declared_ret(recv, name, args, None);
        assert_eq!(ret(&opt, "is_some", &[]), Some(Ty::Bool));
        assert_eq!(ret(&opt, "unwrap", &[]), Some(int.clone()));
        assert_eq!(ret(&opt, "filter", &[closure(Ty::Bool)]), Some(opt.clone()));
        assert_eq!(ret(&opt, "map", &[closure(Ty::Bool)]), Some(Ty::Option(Box::new(Ty::Bool))), "U is the closure's result");
        assert_eq!(ret(&opt, "map", &[closure(Ty::Unknown)]), Some(Ty::Option(Box::new(Ty::Unknown))));
        assert_eq!(ret(&opt, "map", &[arg_of(Ty::Bool)]), None, "no closure, no U");
        assert_eq!(ret(&opt, "and_then", &[closure(Ty::Option(Box::new(Ty::Str)))]), Some(Ty::Option(Box::new(Ty::Str))));
        assert_eq!(ret(&opt, "ok_or", &[arg_of(Ty::F64)]), Some(Ty::Result(Box::new(int.clone()), Box::new(Ty::F64))));
        assert_eq!(ret(&res, "err", &[]), Some(Ty::Option(Box::new(Ty::Str))));
        assert_eq!(ret(&res, "unwrap_err", &[]), Some(Ty::Str));
        assert_eq!(ret(&res, "map_err", &[closure(Ty::F64)]), Some(Ty::Result(Box::new(int.clone()), Box::new(Ty::F64))));
        assert_eq!(ret(&res, "map", &[closure(Ty::Bool)]), Some(Ty::Result(Box::new(Ty::Bool), Box::new(Ty::Str))));
        // the error type of `and_then`'s closure result is the receiver's: an `Ok(..)` whose error is not known still types
        let ok_only = Ty::Result(Box::new(Ty::Bool), Box::new(Ty::Unknown));
        assert_eq!(ret(&res, "and_then", &[closure(ok_only)]), Some(Ty::Result(Box::new(Ty::Bool), Box::new(Ty::Str))));
        for name in ["map_or", "is_some_and", "expect_err"] {
            assert_eq!(ret(&opt, name, &[closure(Ty::Bool)]), None, "{name}: no declaration (ADR-0039)");
            assert_eq!(ret(&res, name, &[closure(Ty::Bool)]), None, "{name}: no declaration (ADR-0039)");
        }
    }

    #[test]
    fn an_option_or_result_argument_has_the_type_its_parameter_declares() {
        let s = StdSurface::shared();
        let int = Ty::Int(IntKind::I32);
        let opt = Ty::Option(Box::new(int.clone()));
        let res = Ty::Result(Box::new(int.clone()), Box::new(Ty::Str));
        assert_eq!(s.declared_arg(&opt, "unwrap_or", 0), Some(int.clone()));
        assert_eq!(s.declared_arg(&opt, "expect", 0), Some(Ty::Str));
        assert_eq!(s.declared_arg(&opt, "map", 0), Some(Ty::Fn(vec![int.clone()], Box::new(Ty::Unknown))));
        assert_eq!(s.declared_arg(&opt, "filter", 0), Some(Ty::Fn(vec![int.clone()], Box::new(Ty::Bool))));
        assert_eq!(s.declared_arg(&opt, "unwrap_or_else", 0), Some(Ty::Fn(vec![], Box::new(int.clone()))));
        assert_eq!(s.declared_arg(&opt, "ok_or", 0), None, "a bare method generic has no expected type");
        assert_eq!(s.declared_arg(&res, "map_err", 0), Some(Ty::Fn(vec![Ty::Str], Box::new(Ty::Unknown))));
        assert_eq!(s.declared_arg(&res, "unwrap_or_else", 0), Some(Ty::Fn(vec![Ty::Str], Box::new(int.clone()))));
        assert_eq!(s.declared_arg(&res, "and_then", 0), Some(Ty::Fn(vec![int], Box::new(Ty::Result(Box::new(Ty::Unknown), Box::new(Ty::Str))))));
        assert_eq!(s.declared_arg(&res, "is_ok", 0), None);
    }
}
