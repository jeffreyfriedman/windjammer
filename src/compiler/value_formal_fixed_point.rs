//! Convergence O4: a value formal (a user type or collection taken by value) is spelled from its IR mode, so
//! every other function must read that same mode: a caller's IR (OWN-CALL-FORMAL) and its call sites both look
//! the callee up in the registry. After each IR lowering round the solved mode of every spelled value formal
//! is published under the keys that name its function; lowering repeats until no published mode changes, so
//! callers and callees agree (a fixed point over the call graph). Before this, callers read the analyzer's or
//! the bare-pass pass's prediction of a callee's mode, and a forwarder could borrow what its callee consumes.

use std::collections::{HashMap, HashSet};

use crate::analyzer::{AnalyzedFunction, OwnershipMode, SignatureRegistry};
use crate::ir::node::IrFunction;
use crate::ir::safety_type::OwnedType;
use crate::parser::{Item, Parameter, Program, Type};

/// At most this many lowering rounds: one per level of forwarding the fixed point has to cross.
pub(crate) const MAX_ROUNDS: usize = 4;

/// What publication needs to know about the crate besides the IR.
pub(crate) struct FormalScope {
    /// `Type::method` of trait impls: their formals follow the trait, not the IR.
    trait_impl_methods: HashSet<String>,
    /// Free functions per bare name: a homonym never owns the bare key.
    bare_name_counts: HashMap<String, usize>,
    copy_types: HashSet<String>,
}

impl FormalScope {
    pub(crate) fn new(programs: &[Program], copy_types: &HashSet<String>) -> Self {
        let mut scope = FormalScope { trait_impl_methods: HashSet::new(), bare_name_counts: HashMap::new(), copy_types: copy_types.clone() };
        for program in programs {
            scope.collect(&program.items);
        }
        scope
    }

    fn collect(&mut self, items: &[Item]) {
        for item in items {
            match item {
                Item::Function { decl, .. } => *self.bare_name_counts.entry(decl.name.clone()).or_insert(0) += 1,
                Item::Impl { block, .. } if block.trait_name.is_some() => {
                    let base = block.type_name.split('<').next().unwrap_or(&block.type_name);
                    for f in &block.functions {
                        self.trait_impl_methods.insert(format!("{base}::{}", f.name));
                    }
                }
                _ => {}
            }
        }
    }

    fn is_copy(&self, ty: &Type) -> bool {
        match ty {
            Type::Custom(name) => {
                let leaf = name.rsplit("::").next().unwrap_or(name);
                self.copy_types.contains(name)
                    || self.copy_types.contains(leaf)
                    || crate::type_classification::is_copy_primitive(leaf)
                    || crate::type_classification::is_known_copy_aggregate(leaf)
            }
            Type::Parameterized(base, args) if base == "Option" => args.iter().all(|a| self.is_copy(a)),
            _ => false,
        }
    }

    /// The formals codegen spells from the IR mode (`function_self_helpers::ir_value_formal`).
    fn spelled(&self, param: &Parameter) -> bool {
        crate::ir::formal_predicates::ir_spells_value_formal(param)
            && !self.is_copy(&param.type_)
            && !crate::analyzer::Analyzer::is_generic_type_param(&param.type_)
    }

    /// Publish the solved value-formal modes of one file's functions. True when a published mode changed.
    pub(crate) fn publish(
        &self,
        registry: &mut SignatureRegistry,
        file_stem: Option<&str>,
        ir_functions: &[IrFunction],
        analyzed: &[AnalyzedFunction],
    ) -> bool {
        let mut changed = false;
        for (ir_fn, af) in ir_functions.iter().zip(analyzed) {
            for key in self.publish_keys(file_stem, ir_fn, af) {
                changed |= self.publish_key(registry, key, ir_fn, af);
            }
        }
        changed
    }

    /// Record in `sig.emitted_rust_ref_params` that formal `idx` is (not) a shared reference. A signature
    /// that has no flags yet gets them from what it already says: a declared reference, or a text formal
    /// the prediction borrows. True when the vector changed.
    fn publish_emitted_flag(sig: &mut crate::analyzer::FunctionSignature, idx: usize, mode: OwnershipMode) -> bool {
        let before = sig.emitted_rust_ref_params.clone();
        // Only a shared borrow is published into a signature that has no flags: `Some(false)` reads as "owned by
        // value", which a `&mut` formal is not (a forward through a self alias must keep passing the borrow on).
        if sig.emitted_rust_ref_params.is_none() && mode != OwnershipMode::Borrowed {
            return false;
        }
        if sig.emitted_rust_ref_params.is_none() {
            let flags = (0..sig.param_ownership.len())
                .map(|i| {
                    let declared_ref = sig.formal_param_types.get(i).is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)));
                    let text_borrowed = sig.param_ownership[i] == OwnershipMode::Borrowed
                        && sig.formal_param_types.get(i).is_some_and(crate::ir::formal_predicates::is_windjammer_text_type);
                    declared_ref || text_borrowed
                })
                .collect();
            sig.emitted_rust_ref_params = Some(flags);
        }
        if let Some(flag) = sig.emitted_rust_ref_params.as_mut().and_then(|f| f.get_mut(idx)) {
            *flag = mode == OwnershipMode::Borrowed;
        }
        sig.emitted_rust_ref_params != before
    }

    /// The registry keys one function's solved modes are published under (none for a trait impl method).
    pub(crate) fn publish_keys(&self, file_stem: Option<&str>, ir_fn: &IrFunction, af: &AnalyzedFunction) -> Vec<String> {
        if ir_fn.name.contains("::") {
            if self.trait_impl_methods.contains(&ir_fn.name) {
                return Vec::new();
            }
            // `Host::run` and the module-qualified `graph::Host::run` other files read
            let mut keys = vec![ir_fn.name.clone()];
            keys.extend(file_stem.map(|stem| format!("{stem}::{}", ir_fn.name)));
            keys
        } else {
            let mut keys: Vec<String> = file_stem.map(|stem| format!("{stem}::{}", af.decl.name)).into_iter().collect();
            if self.bare_name_counts.get(&af.decl.name) == Some(&1) {
                keys.push(af.decl.name.clone());
            }
            keys
        }
    }

    /// Publish one function's solved modes under `key` when `registry` holds it. True when a mode changed.
    pub(crate) fn publish_key(&self, registry: &mut SignatureRegistry, key: String, ir_fn: &IrFunction, af: &AnalyzedFunction) -> bool {
        let Some(mut sig) = registry.signatures.get(&key).cloned() else {
            return false;
        };
        if sig.param_ownership.len() != af.decl.parameters.len() {
            return false;
        }
        let mut touched = false;
        for (idx, param) in af.decl.parameters.iter().enumerate() {
            // Receivers outside trait impls are spelled from the IR mode too (`self_receiver`).
            let receiver = param.name == "self";
            // An unused text formal of an API is owned by the solver's rule (OWN-PUB-TEXT-UNUSED,
            // OWN-TEXT-API-UNUSED) and emitted as such: callers generated first read that mode too.
            let text_api = crate::ir::constraint_gen::text_formals::unused_text_api_formal(&af.decl, param).is_some();
            if !receiver && !text_api && !self.spelled(param) {
                continue;
            }
            let Some(solved) = ir_fn.param_types.get(&param.name).map(|t| &t.ownership) else {
                continue;
            };
            let mode = match solved {
                _ if param.is_mutable && !receiver => OwnershipMode::Owned,
                OwnedType::Inferred => continue,
                other => crate::ir::signature_bridge::owned_type_to_ownership_mode(other),
            };
            // The backend spells this formal from the solved mode, so the published signature says so
            // before the callee's file is generated: a caller generated first (a split `impl` in a file
            // the caller does not import) otherwise reads no emitted flag and takes a bare declared formal
            // for an owned one (`upload(p: Palette)` emitted `&Palette`, the call passed `p.clone()`).
            if !receiver {
                touched |= Self::publish_emitted_flag(&mut sig, idx, mode);
            }
            if sig.param_ownership[idx] == mode {
                continue;
            }
            sig.param_ownership[idx] = mode;
            let bare = param.type_.clone();
            if let Some(slot) = sig.param_types.get_mut(idx).filter(|_| !receiver) {
                *slot = match mode {
                    OwnershipMode::Borrowed => Type::Reference(Box::new(bare)),
                    OwnershipMode::MutBorrowed => Type::MutableReference(Box::new(bare)),
                    OwnershipMode::Owned => bare,
                };
            }
            touched = true;
        }
        if touched {
            registry.signatures.insert(key, sig);
        }
        touched
    }
}

/// A single-file build (`attach_ir_module_if_missing`) lowers its program the way a library build lowers each
/// file: in rounds, publishing the solved value-formal and
/// receiver modes into `registry` until none changes, so a caller reads its callee's solved mode instead of the
/// analyzer's prediction (`Merchant::add_item` forwarding `item` to `Inventory::add_item`, which stores it).
pub(crate) fn lower_program_to_fixed_point(
    program: &Program,
    analyzed: &[AnalyzedFunction],
    registry: &mut SignatureRegistry,
    fields: &HashMap<String, HashMap<String, Type>>,
    copy_types: &HashSet<String>,
) -> crate::ir::pipeline::IrModule {
    let scope = FormalScope::new(std::slice::from_ref(program), copy_types);
    let mut module = crate::ir::IrPipeline::new().lower_to_ir(analyzed, registry, Some(fields));
    for _ in 1..MAX_ROUNDS {
        if !scope.publish(registry, None, &module.functions, analyzed) {
            break;
        }
        module = crate::ir::IrPipeline::new().lower_to_ir(analyzed, registry, Some(fields));
    }
    module
}
