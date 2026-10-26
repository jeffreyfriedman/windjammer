//! Definition registration: turns parsed items into the interpreter's tables.

use super::engine::*;
use super::error::RuntimeError;
use crate::parser::{
    EnumVariantData, FunctionDecl, Item, Program, TraitMethod, Type,
};
use std::cell::OnceCell;
use std::rc::Rc;

pub(crate) fn base_type_name(n: &str) -> String {
    let b = n.split('<').next().unwrap_or(n).trim().trim_start_matches('&').trim_start_matches("mut ").trim();
    match b {
        "int" => "i64".to_string(),
        "uint" => "u64".to_string(),
        "float" => "f64".to_string(),
        "string" | "str" => "String".to_string(),
        other => other.to_string(),
    }
}

fn has_copy_derive(decl: &crate::parser::StructDecl<'static>) -> bool {
    decl.decorators.iter().any(|d| {
        d.name == "derive"
            && d.arguments.iter().any(|(k, v)| {
                k == "Copy"
                    || matches!(v, crate::parser::Expression::Identifier { name, .. } if name == "Copy")
            })
    })
}

impl Interpreter {
    pub(crate) fn func_from_decl(
        &self,
        decl: &'static FunctionDecl<'static>,
        module: usize,
        self_ty: Option<&str>,
        impl_params: &[crate::parser::TypeParam],
    ) -> Rc<FuncDef> {
        let qual: Rc<str> = match self_ty {
            Some(t) => Rc::from(format!("{}::{}", t, decl.name)),
            None => Rc::from(decl.name.as_str()),
        };
        Rc::new(FuncDef {
            name: decl.name.clone(),
            qual,
            params: &decl.parameters,
            ret: decl.return_type.as_ref(),
            body: &decl.body,
            module,
            self_ty: self_ty.map(Rc::from),
            decorators: &decl.decorators,
            is_extern: decl.is_extern,
            is_async: decl.is_async,
            type_params: impl_params.iter().chain(decl.type_params.iter()).map(|p| p.name.clone()).collect(),
            mutated: OnceCell::new(),
            self_mutated: OnceCell::new(),
        })
    }

    fn func_from_trait_default(
        &self,
        m: &'static TraitMethod<'static>,
        module: usize,
        self_ty: &str,
    ) -> Option<Rc<FuncDef>> {
        let body = m.body.as_ref()?;
        Some(Rc::new(FuncDef {
            name: m.name.clone(),
            qual: Rc::from(format!("{}::{}", self_ty, m.name)),
            params: &m.parameters,
            ret: m.return_type.as_ref(),
            body,
            module,
            self_ty: Some(Rc::from(self_ty)),
            decorators: &[],
            is_extern: false,
            is_async: false,
            type_params: Vec::new(),
            mutated: OnceCell::new(),
            self_mutated: OnceCell::new(),
        }))
    }

    /// Register every item of `program` into module `module`.
    pub fn load_program(
        &mut self,
        module: usize,
        program: &'static Program<'static>,
    ) -> Result<(), RuntimeError> {
        // `std::arena` (ADR-0025): the compiler's feature gate and escape check apply here too.
        crate::analyzer::arena_escape::gate(program).map_err(|e| RuntimeError::new(super::error::ErrorKind::Semantic, e))?;
        self.note_duplicate_types(&[(self.module_path_string(module), &program.items)]);
        self.register_items(module, &program.items)?;
        self.finish_registration()?;
        Ok(())
    }

    pub(crate) fn register_items(
        &mut self,
        module: usize,
        items: &'static [Item<'static>],
    ) -> Result<(), RuntimeError> {
        // definitions changed (REPL, lazily loaded modules): cached call resolutions may be stale
        self.call_cache.clear();
        for item in items {
            match item {
                Item::Function { decl, .. } => {
                    let f = self.func_from_decl(decl, module, None, &[]);
                    self.modules[module].funcs.insert(decl.name.clone(), f);
                }
                Item::Struct { decl, .. } => {
                    let tuple = decl.tuple_fields.is_some();
                    let fields: Vec<(&'static str, Type)> = if let Some(tf) = &decl.tuple_fields {
                        // tuple-struct field names are leaked once at registration
                        tf.iter()
                            .enumerate()
                            .map(|(i, t)| (&*Box::leak(i.to_string().into_boxed_str()), t.clone()))
                            .collect()
                    } else {
                        decl.fields.iter().map(|f| (f.name.as_str(), f.field_type.clone())).collect()
                    };
                    let rc_fields = fields.iter().map(|(n, _)| Rc::from(*n)).collect();
                    let key = self.declared_type_key(module, &decl.name);
                    self.structs.insert(
                        key.clone(),
                        StructDef { fields: fields.into(), rc_name: Rc::from(key.as_str()), rc_fields, tuple, copy: has_copy_derive(decl) },
                    );
                }
                Item::Enum { decl, .. } => {
                    let variants = decl
                        .variants
                        .iter()
                        .map(|v| VariantDef {
                            name: v.name.as_str(),
                            kind: match &v.data {
                                EnumVariantData::Unit => VariantKind::Unit,
                                EnumVariantData::Tuple(ts) => VariantKind::Tuple(ts.clone()),
                                EnumVariantData::Struct(fs) => VariantKind::Struct(
                                    fs.iter().map(|(n, t)| (n.as_str(), t.clone())).collect(),
                                ),
                            },
                        })
                        .collect::<Vec<VariantDef>>();
                    let rc_variants = variants.iter().map(|v| Rc::from(v.name)).collect();
                    let key = self.declared_type_key(module, &decl.name);
                    self.enums.insert(key, EnumDef { variants, rc_name: Rc::from(decl.name.as_str()), rc_variants });
                }
                Item::Trait { decl, .. } => {
                    self.traits.insert(
                        decl.name.clone(),
                        TraitDef { supertraits: decl.supertraits.clone() },
                    );
                    self.trait_methods
                        .insert(decl.name.clone(), decl.methods.iter().collect());
                }
                Item::Impl { block, .. } => {
                    let ty = self.type_key(module, &base_type_name(&block.type_name));
                    if block.trait_name.as_deref() == Some("Drop") {
                        self.drop_types.insert(ty.clone());
                    }
                    for func in &block.functions {
                        let f = self.func_from_decl(func, module, Some(&ty), &block.type_params);
                        if let (Some("From"), Some([src]), "from") =
                            (block.trait_name.as_deref(), block.trait_type_args.as_deref(), func.name.as_str())
                        {
                            self.from_impls.entry(ty.clone()).or_default().push((src.clone(), f.clone()));
                        }
                        if let Some(tr) = &block.trait_name {
                            // trait-qualified key disambiguates e.g. Display::fmt vs Debug::fmt
                            self.methods.insert((ty.clone(), format!("{}::{}", base_type_name(tr), func.name)), f.clone());
                            self.methods.entry((ty.clone(), func.name.clone())).or_insert(f);
                        } else {
                            self.methods.insert((ty.clone(), func.name.clone()), f);
                        }
                    }
                    if let Some(tr) = &block.trait_name {
                        self.trait_impls.push((ty.clone(), base_type_name(tr)));
                    }
                }
                Item::Const { name, value, type_, .. } | Item::Static { name, value, type_, .. } => {
                    self.modules[module].consts.insert(name.clone(), value);
                    self.modules[module].const_types.insert(name.clone(), type_);
                }
                Item::TypeAlias { name, target, .. } => {
                    self.type_aliases.insert(name.clone(), target.clone());
                }
                Item::Mod { name, items, .. } => {
                    // `mod x` (file module) reuses the module the loader created for `x.wj`;
                    // `mod x { ... }` is an inline module of this file.
                    let child = match self.modules[module].children.get(name) {
                        Some(&c) => c,
                        None if items.is_empty() => match self.load_child_module(module, name)? {
                            Some(c) => c,
                            None => self.inline_module(module, name),
                        },
                        None => self.inline_module(module, name),
                    };
                    self.register_items(child, items)?;
                }
                Item::Use { path, alias, .. } => {
                    self.register_use(module, path, alias.as_deref())?;
                }
                Item::Macro { expr, .. } => {
                    // ADR-0014: the language has no macros; a top-level invocation is never silently skipped
                    let name = match expr {
                        crate::parser::Expression::MacroInvocation { name, .. } => name.as_str(),
                        _ => "?",
                    };
                    return Err(RuntimeError::new(
                        crate::interpreter::error::ErrorKind::Unsupported,
                        format!("top-level macro invocation `{}!` (Windjammer has no macros)", name),
                    ));
                }
                Item::ExternLet { .. } | Item::BoundAlias { .. } => {}
            }
        }
        Ok(())
    }

    fn inline_module(&mut self, parent: usize, name: &str) -> usize {
        let file = self.modules[parent].file.clone();
        let child = self.new_module(name, file, Some(parent));
        self.modules[parent].children.insert(name.to_string(), child);
        child
    }

    pub(crate) fn new_module(
        &mut self,
        name: &str,
        file: std::path::PathBuf,
        parent: Option<usize>,
    ) -> usize {
        self.modules.push(Module::new(name, file, parent));
        self.modules.len() - 1
    }

    /// After all definitions are known: fill trait default methods, compute mutating-method names.
    pub(crate) fn finish_registration(&mut self) -> Result<(), RuntimeError> {
        if let Some(t) = self.drop_types.iter().find(|t| !self.structs.contains_key(*t)) {
            return Err(RuntimeError::new(
                crate::interpreter::error::ErrorKind::Unsupported,
                format!("`impl Drop for {}`: only structs can have destructors in the interpreter", t),
            ));
        }
        let impls = self.trait_impls.clone();
        for (ty, tr) in impls {
            let mut chain = vec![tr.clone()];
            // supertrait defaults are inherited by the implementor only if it implements them itself
            chain.dedup();
            for t in chain {
                if let Some(ms) = self.trait_methods.get(&t).cloned() {
                    for m in ms {
                        let key = (ty.clone(), m.name.clone());
                        if !self.methods.contains_key(&key) {
                            if let Some(f) = self.func_from_trait_default(m, 0, &ty) {
                                self.methods.insert(key, f);
                            }
                        }
                    }
                }
            }
        }
        self.trait_impls.clear();
        Ok(())
    }
}
