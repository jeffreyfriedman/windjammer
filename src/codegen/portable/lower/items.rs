//! Item-level lowering: structs, enums, traits, impls, functions, constants.

use super::{Lowerer, PResult};
use crate::analyzer::SignatureRegistry;
use crate::codegen::portable::env::{fix_self, ItemEnv};
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::{from_ast, Ty};
use crate::codegen::portable::visit;
use crate::parser::{FunctionDecl, ImplBlock, Item, Program, TraitMethod, TypeParam};
use std::collections::HashMap;

/// Lower a whole program. `reg` supplies analyzer-inferred parameter ownership.
pub fn lower_program(program: &Program, reg: Option<&SignatureRegistry>) -> PResult<PModule> {
    let env = ItemEnv::build(program, reg);
    let mut m = PModule { structs: vec![], enums: vec![], traits: vec![], fns: vec![], consts: vec![] };
    let mut items: Vec<&Item> = Vec::new();
    flatten(&program.items, &mut items);
    let defaults = trait_defaults(&items);
    for item in &items {
        lower_item(&env, item, &defaults, &mut m)?;
    }
    for item in &items {
        if let Item::Impl { block, .. } = item {
            attach_impl(&env, block, &defaults, &mut m)?;
        }
    }
    Ok(m)
}

fn flatten<'a, 'ast>(items: &'a [Item<'ast>], out: &mut Vec<&'a Item<'ast>>) {
    for i in items {
        match i {
            Item::Mod { items, .. } => flatten(items, out),
            other => out.push(other),
        }
    }
}

type Defaults<'a, 'ast> = HashMap<String, Vec<&'a TraitMethod<'ast>>>;

fn trait_defaults<'a, 'ast>(items: &[&'a Item<'ast>]) -> Defaults<'a, 'ast> {
    let mut d: Defaults = HashMap::new();
    for i in items {
        if let Item::Trait { decl, .. } = i {
            d.insert(decl.name.clone(), decl.methods.iter().filter(|m| m.body.is_some()).collect());
        }
    }
    d
}

fn lower_item(env: &ItemEnv, item: &Item, defaults: &Defaults, m: &mut PModule) -> PResult<()> {
    match item {
        Item::Struct { decl, .. } => {
            let info = &env.structs[&decl.name];
            m.structs.push(PStruct { name: decl.name.clone(), type_params: info.type_params.clone(), fields: info.fields.clone(), tuple: info.tuple, is_copy: false, methods: vec![], traits: vec![] });
        }
        Item::Enum { decl, .. } => {
            let info = &env.enums[&decl.name];
            let variants = info.variants.iter().map(|v| PVariant { name: v.name.clone(), fields: v.fields.clone(), named: v.named }).collect();
            m.enums.push(PEnum { name: decl.name.clone(), type_params: info.type_params.clone(), variants, is_copy: false, methods: vec![], traits: vec![] });
        }
        Item::Function { decl, .. } => {
            let sig = &env.fns[&decl.name];
            m.fns.push(lower_fn(env, decl, sig, None, &[])?);
        }
        Item::Trait { decl, .. } => {
            let mut methods = Vec::new();
            for tm in &decl.methods {
                methods.push(lower_trait_method(env, tm, &decl.name, None)?);
            }
            m.traits.push(PTrait { name: decl.name.clone(), methods });
        }
        Item::Const { name, type_, value, .. } | Item::Static { name, type_, value, .. } => {
            let ty = from_ast(type_, &[], &env.aliases);
            let mut lw = Lowerer::new(env);
            let mut e = lw.expr(value, Some(&ty))?;
            visit::types_in_expr(&mut e, &mut |t| *t = lw.resolve_final(t));
            m.consts.push(PConst { name: name.clone(), ty, value: e });
        }
        _ => {}
    }
    let _ = defaults;
    Ok(())
}

fn tparams_names(tp: &[TypeParam]) -> Vec<String> {
    tp.iter().map(|t| t.name.clone()).collect()
}

fn attach_impl(env: &ItemEnv, block: &ImplBlock, defaults: &Defaults, m: &mut PModule) -> PResult<()> {
    let outer = tparams_names(&block.type_params);
    let owner = crate::codegen::portable::env::base_name(&block.type_name).to_string();
    let mut methods = Vec::new();
    for f in &block.functions {
        let name = crate::codegen::portable::env::impl_method_name(block, &f.name, &env.aliases);
        let sig = &env.methods[&(owner.clone(), name.clone())];
        let mut fx = lower_fn(env, f, sig, Some(&owner), &outer)?;
        fx.name = name;
        fx.owner = Some(owner.clone());
        methods.push(fx);
    }
    if let Some(tr) = &block.trait_name {
        for tm in defaults.get(tr).into_iter().flatten() {
            if !block.functions.iter().any(|f| f.name == tm.name) {
                methods.push(lower_trait_method(env, tm, tr, Some((&owner, &outer)))?);
            }
        }
    }
    if let Some(s) = m.structs.iter_mut().find(|s| s.name == owner) {
        s.methods.extend(methods);
        s.traits.extend(block.trait_name.clone());
    } else if let Some(e) = m.enums.iter_mut().find(|e| e.name == owner) {
        e.methods.extend(methods);
        e.traits.extend(block.trait_name.clone());
    } else {
        return Err(Unsupported { what: format!("impl for unknown type `{owner}`"), line: 0 });
    }
    Ok(())
}

/// Lower a trait method (declaration, or default body specialised for `for_type`).
fn lower_trait_method(env: &ItemEnv, tm: &TraitMethod, trait_name: &str, for_type: Option<(&String, &[String])>) -> PResult<PFn> {
    let decl = FunctionDecl {
        name: tm.name.clone(),
        is_pub: true,
        is_extern: false,
        type_params: tm.type_params.clone(),
        where_clause: vec![],
        decorators: vec![],
        is_async: tm.is_async,
        parameters: tm.parameters.clone(),
        return_type: tm.return_type.clone(),
        return_decorators: vec![],
        body: tm.body.clone().unwrap_or_default(),
        parent_type: Some(trait_name.to_string()),
        impl_trait: Some(trait_name.to_string()),
        doc_comment: None,
    };
    let owner = for_type.map(|(t, _)| t.as_str()).unwrap_or(trait_name);
    let outer: &[String] = for_type.map(|(_, o)| o).unwrap_or(&[]);
    let sig = env.method(owner, &tm.name).cloned().or_else(|| env.method(trait_name, &tm.name).cloned()).unwrap();
    let mut f = lower_fn(env, &decl, &sig, Some(owner), outer)?;
    f.owner = Some(owner.to_string());
    if tm.body.is_none() {
        f.body = None;
    }
    Ok(f)
}

fn bounds_of(decl: &FunctionDecl) -> HashMap<String, Vec<String>> {
    let mut b: HashMap<String, Vec<String>> = HashMap::new();
    for t in &decl.type_params {
        b.entry(t.name.clone()).or_default().extend(t.bounds.iter().cloned());
    }
    for (name, tr) in &decl.where_clause {
        b.entry(name.clone()).or_default().extend(tr.iter().cloned());
    }
    b
}

pub(super) fn lower_fn(env: &ItemEnv, decl: &FunctionDecl, sig: &crate::codegen::portable::env::FnSig, owner: Option<&str>, outer: &[String]) -> PResult<PFn> {
    let mut lw = Lowerer::new(env);
    lw.owner = owner.map(|s| s.to_string());
    lw.generics = outer.iter().cloned().chain(decl.type_params.iter().map(|t| t.name.clone())).collect();
    lw.bounds = bounds_of(decl);
    lw.line = 0;
    let self_ty = owner.map(|o| Ty::Adt(o.to_string(), outer.iter().map(|n| Ty::Param(n.clone())).collect()));
    lw.push();
    let mut params = Vec::new();
    for (i, p) in decl.parameters.iter().enumerate() {
        if p.name == "self" {
            let st = self_ty.clone().unwrap_or(Ty::Unknown);
            lw.declare("self", st, sig.recv != Some(ParamMode::Value));
            continue;
        }
        let (_, pty, mode) = sig.params.iter().find(|(n, _, _)| *n == p.name).cloned().unwrap_or((p.name.clone(), fix_self(from_ast(&p.type_, &lw.generics, &env.aliases), self_ty.as_ref()), ParamMode::Value));
        let _ = i;
        let out = lw.declare(&p.name, pty.clone(), mode != ParamMode::Value);
        params.push(PParam { name: out, ty: pty, mode, own_mutable: p.is_mutable });
    }
    lw.rets.push(sig.ret.clone());
    let need_value = sig.ret != Ty::Unit;
    let body = lw.stmts_with_tail(&decl.body, Some(&sig.ret).filter(|t| **t != Ty::Unit), need_value)?;
    let mut body = body;
    if let Some(t) = body.tail.take() {
        lw.constrain(&sig.ret, &t.ty);
        body.tail = Some(lw.own_for_return(t));
    }
    lw.pop();
    let mut bounds: Vec<(String, Vec<String>)> = lw.bounds.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    bounds.sort();
    let mut f = PFn { name: decl.name.clone(), type_params: decl.type_params.iter().map(|t| t.name.clone()).collect(), bounds, recv: sig.recv, params, ret: sig.ret.clone(), body: Some(body), owner: owner.map(|s| s.to_string()), is_async: decl.is_async };
    let resolver = Lowerer { vars: lw.vars.clone(), int_vars: lw.int_vars.clone(), float_vars: lw.float_vars.clone(), usize_vars: lw.usize_vars.clone(), ..Lowerer::new(env) };
    visit::types_in_fn(&mut f, &mut |t| *t = resolver.resolve_final(t));
    Ok(f)
}
