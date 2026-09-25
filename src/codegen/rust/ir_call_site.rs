//! IR-driven call-site argument coercion for Rust codegen.
//!
//! When `IrCutoverConfig.call_sites` is enabled, applies `encode_call_argument`
//! using callee signature expectations instead of heuristic borrow passes.

use crate::analyzer::SignatureRegistry;
use crate::codegen::rust::generator::CodeGenerator;
use crate::ir::coercion::compute_coercion;
use crate::ir::coercion::CoercionKind;
use crate::ir::safety_type::{BaseType, OwnedType, Region, SafetyType};
use crate::ir::signature_bridge::{safety_type_from_parser_type, safety_type_from_signature_param};
use crate::ir::target_encodings::{apply_coercion, Target};
use crate::parser::{Expression, Literal, Statement, Type};

impl<'ast> CodeGenerator<'ast> {
    /// Defining-module / registry view: this argument slot emits owned Rust (not `&T`).
    fn sig_arg_confirms_owned_emission(
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
    ) -> bool {
        let pidx = sig.arg_param_index(arg_index);
        // Demoted `&mut Vec` / `&mut T` is never owned emission — the WDB-281 Vec/map
        // shortcut must not override MutBorrowed (recursive `buf.clone()` into `&mut Vec`).
        if matches!(
            sig.param_ownership.get(pidx),
            Some(crate::analyzer::OwnershipMode::MutBorrowed)
        ) || sig
            .param_types
            .get(pidx)
            .is_some_and(|t| matches!(t, Type::MutableReference(_)))
            || sig
                .formal_param_type(pidx)
                .is_some_and(|t| matches!(t, Type::MutableReference(_)))
        {
            return false;
        }
        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx) {
            return true;
        }
        // Bare WJ `Vec`/`Map` formals emit owned Rust containers (WDB-281: `contains(items)`
        // must not auto-borrow `&items` then leave E0308 into owned `Vec`).
        if crate::codegen::rust::signature_promotion::bare_formal_is_vec_or_map(sig, pidx)
            && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(sig, pidx)
            && sig
                .emitted_rust_ref_params
                .as_ref()
                .and_then(|flags| flags.get(pidx))
                .copied()
                != Some(true)
        {
            return true;
        }
        sig.formal_param_type(pidx)
            .or_else(|| sig.param_types.get(pidx))
            .is_some_and(|t| {
                // Converged `&T` / `&mut T` formals are never owned emission — stripping
                // the wrapper and testing bare Custom misclassified cross-crate MutBorrowed
                // metadata (`touch_grid(grid: &mut Grid)`) as owned (grid.clone()).
                if matches!(t, Type::Reference(_) | Type::MutableReference(_)) {
                    return false;
                }
                if crate::codegen::rust::types::is_windjammer_text_type(t) {
                    return !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                        sig, pidx,
                    );
                }
                // WDB-212: bare Custom in WJ AST is not owned emission when codegen demoted
                // to `&T` (`timeseries_ingest_batch_point_count(batch: &Batch)`).
                matches!(t, Type::Custom(_))
                    && !crate::codegen::rust::stdlib_method_traits::is_map_type(t)
                    && !crate::codegen::rust::stdlib_method_traits::is_set_type(t)
                    && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(sig, pidx)
                    && sig
                        .emitted_rust_ref_params
                        .as_ref()
                        .and_then(|flags| flags.get(pidx))
                        .copied()
                        != Some(true)
            })
    }

    fn refreshed_call_site_sig_for_arg<'a>(
        &self,
        registry: &'a SignatureRegistry,
        callee_name: &str,
        arg_index: usize,
        sig: &crate::analyzer::FunctionSignature,
    ) -> crate::analyzer::FunctionSignature {
        self.refresh_call_site_signature_for_arg(Some(sig.clone()), callee_name, arg_index)
            .unwrap_or_else(|| sig.clone())
    }

    /// Path-dep / import-alias metadata: this arg slot emits owned Rust (`String`), not `&str`.
    /// Used to peel IR over-borrow (`&value` into `require_nonempty(field: &str, value: String)`).
    ///
    /// Analyzer `Owned` on a bare WJ `string` is not enough — `parse_level` / `slugify`
    /// keep that flag after demoting the Rust formal to `&str`. Shared-ref emission
    /// (`emitted_rust_ref_params` / `callee_emits_shared_rust_ref_param`) wins
    /// **on the resolved callee**, not a stdlib homonym (`url::parse` vs local `parse`).
    pub(crate) fn cross_crate_dep_arg_confirms_owned(
        &self,
        callee_name: &str,
        arg_index: usize,
    ) -> bool {
        let (shared, owned) = self.cross_crate_dep_arg_emission(callee_name, arg_index);
        owned && !shared
    }

    /// Path-dep slot emits shared `&str` / `&T` (notes-api → `log_tagged` / `slugify`).
    pub(crate) fn cross_crate_dep_arg_confirms_shared(
        &self,
        callee_name: &str,
        arg_index: usize,
    ) -> bool {
        self.cross_crate_dep_arg_emission(callee_name, arg_index).0
    }

    /// One callee signature: path-qualified / import-alias first, then same-crate
    /// exact local. Never OR every `parse`/`get` in the registry (Shared≠Lock).
    pub(crate) fn resolve_cross_crate_dep_signature(
        &self,
        callee_name: &str,
    ) -> Option<crate::analyzer::FunctionSignature> {
        let lookup = self.signature_lookup_callee_name(callee_name);
        let lookup_ref = lookup.as_ref();
        let simple = lookup_ref.rsplit("::").next().unwrap_or(lookup_ref);
        let path_qualified = lookup_ref.contains("::")
            || self.is_import_alias_cross_crate_call(callee_name);

        let from_reg = |reg: &crate::analyzer::SignatureRegistry, key: &str| {
            reg.get_signature(key).cloned()
        };

        if path_qualified {
            if let Some(s) = from_reg(&self.signature_registry, lookup_ref).or_else(|| {
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| from_reg(g, lookup_ref))
            }) {
                return Some(s);
            }
            if let Some(s) = self
                .global_signature_registry
                .as_ref()
                .and_then(|g| g.find_unique_signature_ending_with(simple).cloned())
                .or_else(|| {
                    self.signature_registry
                        .find_unique_signature_ending_with(simple)
                        .cloned()
                })
            {
                return Some(s);
            }
            return None;
        }

        // Bare same-crate name (`parse`, `get`, `stringify`): local exact wins.
        // A global `url::parse` / `HashMap::get` must not steal the slot.
        if let Some(s) = from_reg(&self.signature_registry, callee_name)
            .or_else(|| from_reg(&self.signature_registry, lookup_ref))
            .or_else(|| from_reg(&self.signature_registry, simple))
        {
            return Some(s);
        }
        if let Some(g) = self.global_signature_registry.as_ref() {
            if let Some(s) = g.find_unique_signature_ending_with(simple).cloned() {
                return Some(s);
            }
            if let Some(s) = from_reg(g, callee_name)
                .or_else(|| from_reg(g, lookup_ref))
                .or_else(|| from_reg(g, simple))
            {
                return Some(s);
            }
        }
        None
    }

    fn cross_crate_dep_arg_emission(&self, callee_name: &str, arg_index: usize) -> (bool, bool) {
        let Some(rs) = self.resolve_cross_crate_dep_signature(callee_name) else {
            return (false, false);
        };
        let pidx = rs.arg_param_index(arg_index);
        let shared = crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&rs, pidx)
            || rs
                .emitted_rust_ref_params
                .as_ref()
                .and_then(|f| f.get(pidx))
                .copied()
                == Some(true);
        if shared {
            return (true, false);
        }
        let owned = Self::sig_arg_confirms_owned_emission(&rs, arg_index)
            || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(&rs, pidx);
        (false, owned)
    }

    pub(crate) fn is_collection_key_lookup_at_site(
        &self,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
        receiver_type: Option<&str>,
    ) -> bool {
        let pidx = sig.arg_param_index(arg_index);
        let would_be_ck =
            crate::codegen::rust::stdlib_method_traits::is_collection_key_lookup_with_project(
                sig,
                arg_index,
                receiver_type,
                self.global_signature_registry.as_deref(),
            );
        // Owned Custom formals (`MemoryEngine::get(key: Key)`) must not inherit map-key
        // borrow — but poisoned `HashMap::get(key: K)` / unknown-receiver `g.data.get`
        // must still classify as `&K` (P3.288). Only apply the owned escape when the
        // signature path would *not* otherwise be a collection-key lookup.
        if !would_be_ck
            && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx)
                || crate::codegen::rust::signature_promotion::bare_formal_is_owned_user_type(
                    sig, pidx,
                ))
        {
            return false;
        }
        if let Some(rt) = receiver_type {
            let base = rt.split('<').next().unwrap_or(rt);
            if !crate::type_classification::is_map_type_name(base)
                && !crate::type_classification::is_set_type_name(base)
            {
                if let Some(sn) = self.current_struct_name.as_deref() {
                    if sn == base {
                        if self
                            .struct_method_ast_formal_param_types
                            .get(sn)
                            .and_then(|m| m.get(sig.name.rsplit("::").next().unwrap_or(&sig.name)))
                            .and_then(|formals| {
                                formals.get(pidx.saturating_sub(if sig.has_self_receiver {
                                    1
                                } else {
                                    0
                                }))
                            })
                            .is_some_and(|t| {
                                matches!(t, Type::Custom(_))
                                    && !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                            })
                        {
                            return false;
                        }
                    }
                }
            }
        }
        would_be_ck
    }

    /// Apply IR-driven coercion to a call-site argument when call_sites cutover is on.
    ///
    /// For known callees this is total: always returns `Some` when `call_sites` is on.
    /// Missing signatures at module boundaries are recorded as hard errors (fail closed).
    pub(crate) fn apply_ir_call_site_coercion(
        &self,
        registry: &SignatureRegistry,
        callee_name: &str,
        arg_index: usize,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
        local_sig: Option<&crate::analyzer::FunctionSignature>,
        receiver_type_name: Option<&str>,
        user_arg_count: Option<usize>,
    ) -> Option<String> {
        if !self.ir_cutover.call_sites {
            return None;
        }

        // User-written `&x` / `&mut x`: preserve only when callee expects a borrow.
        // Owned formals need IR coercion (clone/deref), not passthrough.
        if crate::codegen::rust::expression_helpers::is_reference_expression(arg_expr)
            || ownership_from_rust_expr(arg_str).is_some()
        {
            let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
            let early_sig = registry
                .get_signature(callee_name)
                .cloned()
                .or_else(|| registry.lookup_method(callee_name).cloned())
                .or_else(|| {
                    local_sig
                        .cloned()
                        .filter(|local| local.emitted_rust_ref_params.is_none())
                })
                .or_else(|| local_sig.cloned())
                .or_else(|| {
                    receiver_type_name.and_then(|rt| {
                        self.resolve_method_function_signature(
                            rt,
                            simple,
                            user_arg_count.unwrap_or(arg_index + 1),
                        )
                    })
                });
            if let Some(ref sig) = early_sig {
                let pidx = sig.arg_param_index(arg_index);
                let wants_borrow = sig.param_types.get(pidx).is_some_and(|t| {
                    matches!(t, Type::Reference(_) | Type::MutableReference(_))
                }) || matches!(
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                        sig, arg_index,
                    ),
                    crate::analyzer::OwnershipMode::Borrowed | crate::analyzer::OwnershipMode::MutBorrowed,
                ) || sig
                    .emitted_rust_ref_params
                    .as_ref()
                    .and_then(|flags| flags.get(pidx))
                    .copied()
                    == Some(true);
                if wants_borrow {
                    return Some(arg_str.to_string());
                }
            }
        }

        // Never skip auto-clone when the callee emits owned formals: analyzer/global
        // stubs may still say Borrowed (`&Vec`) while codegen emits `Vec` (regression-056/059).
        let emits_owned_formal = self.ir_callee_arg_emits_owned_contract(
            registry,
            callee_name,
            arg_index,
            user_arg_count,
            local_sig,
        );
        let skip_auto_clone_for_borrow = !emits_owned_formal
            && (self.ir_callee_arg_expects_shared_borrow(
                registry,
                callee_name,
                arg_index,
                user_arg_count,
                local_sig,
            ) || self.ir_callee_arg_expects_mut_borrow(
                registry,
                callee_name,
                arg_index,
                user_arg_count,
                local_sig,
            ));
        let skip_auto_clone_for_field_extract = matches!(arg_expr, Expression::Identifier { .. }
            if self.callee_param_field_extracts_by_name(callee_name, arg_index));
        let collecting_ref_vec = matches!(arg_expr, Expression::Identifier { name, .. }
        if (self.borrowed_iterator_vars.contains(name)
            || self.local_var_types.get(name).is_some_and(|t| {
                matches!(t, Type::Reference(_) | Type::MutableReference(_))
            }))
        && crate::codegen::rust::types::return_type_is_vec_of_shared_refs(
            self.current_function_return_type.as_ref(),
        ));
        // Analysis-driven reuse clones must win over stale demoted/`&T` formals
        // (same policy for bare params and field/index paths).
        let auto_clone_wants = match arg_expr {
            Expression::Identifier { name, .. } => match (callee_name, arg_index) {
                (callee, idx)
                    if !emits_owned_formal
                        && self.callee_arg_expects_borrow_at_call(callee, idx) =>
                {
                    false
                }
                _ => self
                    .auto_clone_analysis
                    .as_ref()
                    .is_some_and(|a| a.needs_clone(name, self.current_statement_idx).is_some()),
            },
            Expression::FieldAccess { .. } | Expression::Index { .. } => {
                match (callee_name, arg_index) {
                    (callee, idx)
                        if !emits_owned_formal
                            && self.callee_arg_expects_borrow_at_call(callee, idx) =>
                    {
                        false
                    }
                    _ => self.auto_clone_field_path_wants_at_call(
                        arg_expr,
                        Some(callee_name),
                        Some(arg_index),
                        emits_owned_formal,
                    ),
                }
            }
            _ => false,
        };
        let mut prepared_arg = match arg_expr {
            Expression::Unary {
                op: crate::parser::UnaryOp::Deref,
                operand,
                ..
            } if self.expression_is_copy(operand)
                || self.infer_expression_type(operand).is_some_and(|t| {
                    matches!(
                        t,
                        Type::Reference(inner) | Type::MutableReference(inner)
                            if self.is_type_copy(inner.as_ref())
                    )
                }) =>
            {
                crate::codegen::rust::call_site_borrow::normalize_explicit_deref_copy_operand(
                    arg_expr, arg_str,
                )
            }
            Expression::Identifier { .. }
                // Mut formals: never auto-clone (reuse analysis must not override).
                // Shared demoted formals may still take analysis-driven clones.
                if (!skip_auto_clone_for_borrow
                    || (auto_clone_wants
                        && !self.ir_callee_arg_expects_mut_borrow(
                            registry,
                            callee_name,
                            arg_index,
                            user_arg_count,
                            local_sig,
                        )))
                    && !skip_auto_clone_for_field_extract
                    && !collecting_ref_vec =>
            {
                self.maybe_auto_clone_call_arg(
                    arg_expr,
                    arg_str,
                    Some(callee_name),
                    Some(arg_index),
                )
            }
            // Field paths (`record.key`) moved into owned formals + reused in loops
            // need `.clone()`; identifier-only auto-clone misses them (regression-059).
            Expression::FieldAccess { .. } | Expression::Index { .. }
                if (!skip_auto_clone_for_borrow
                    || (auto_clone_wants
                        && !self.ir_callee_arg_expects_mut_borrow(
                            registry,
                            callee_name,
                            arg_index,
                            user_arg_count,
                            local_sig,
                        )))
                    && !skip_auto_clone_for_field_extract =>
            {
                self.maybe_auto_clone_call_arg(
                    arg_expr,
                    arg_str,
                    Some(callee_name),
                    Some(arg_index),
                )
            }
            _ => arg_str.to_string(),
        };

        let method_simple = callee_name.rsplit("::").next().unwrap_or(callee_name);

        // Module-qualified free calls without an exact registry key: fail closed.
        // Do not coerce from cross-module homonyms or guess ownership. Inline `mod`
        // callees may register only the bare name — allow that fallback.
        // Method calls supply a receiver type — never treat them as module boundaries
        // even if a buggy callee key looks like `local_var::method`.
        if receiver_type_name.is_none()
            && Self::is_module_boundary_callee(callee_name)
            && !crate::codegen::rust::stdlib_method_traits::is_runtime_std_module(
                crate::codegen::rust::stdlib_method_traits::runtime_module_segment_from_callee_path(
                    callee_name,
                ),
            )
        {
            let has_exact_module_sig = registry.get_signature(callee_name).is_some()
                || self
                    .global_signature_registry
                    .as_ref()
                    .is_some_and(|g| g.get_signature(callee_name).is_some());
            let has_inline_simple_sig = self.inline_module_qualified_call(callee_name)
                && (registry.get_signature(method_simple).is_some()
                    || self
                        .global_signature_registry
                        .as_ref()
                        .is_some_and(|g| g.get_signature(method_simple).is_some())
                    || local_sig.is_some());
            // Exact `mod::fn` or inline `mod` bare-name registration only.
            // Bare dependency homonyms (`circuit_delta_from_edge_inserts` without
            // `dep_crate::` alias) and unknown crates must fail closed — never guess
            // ownership or continue into IR promotion (scene_builder `set_if` needs a
            // registered `station_builder::set_if` / crate-prefix key).
            if !has_exact_module_sig && !has_inline_simple_sig {
                self.report_missing_boundary_signature(callee_name);
                return Some(prepared_arg);
            }
        }

        let lookup_callee = self.signature_lookup_callee_name(callee_name);
        let lookup = lookup_callee.as_ref();
        let import_alias_resolved = self.import_fn_alias_map.contains_key(callee_name);
        let simple = lookup.rsplit("::").next().unwrap_or(lookup);
        let mut sig = if receiver_type_name.is_none() && !callee_name.contains("::") {
            let from_global = self
                .global_signature_registry
                .as_ref()
                .and_then(|g| g.get_signature(lookup).cloned());
            let from_global_simple = if import_alias_resolved {
                None
            } else {
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(simple).cloned())
            };
            let from_reg = registry.get_signature(lookup).cloned();
            let from_local = local_sig.cloned();
            // Prefer defining-module / global refresh first so cross-module free calls
            // see `&str` (regression-049 `replay_to_lsn`) over stale local owned stubs.
            crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                from_global,
                from_global_simple,
                from_reg,
                from_local,
            ])
        } else if crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
            callee_name,
        ) {
            // WDB-332: bare leaf keys (`AudioChannel::new`) are last-writer-wins across
            // sibling modules. Prefer caller-module affinity (and the already-resolved
            // `local_sig` from plain-call lookup) over `get_signature("Type::method")`.
            let arg_count = user_arg_count.unwrap_or(arg_index + 1);
            let affinity = callee_name.rsplit_once("::").and_then(|(receiver_ty, method)| {
                self.lookup_method_signature_on_receiver_type(receiver_ty, method, arg_count)
            });
            affinity.or_else(|| local_sig.cloned()).or_else(|| {
                let from_local = local_sig.cloned();
                let from_reg = registry.get_signature(callee_name).cloned();
                let from_global = self
                    .global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(callee_name).cloned());
                crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                    from_local,
                    from_reg,
                    from_global,
                ])
            })
        } else {
            let from_local = local_sig.cloned();
            let from_reg = registry.get_signature(callee_name).cloned();
            let from_global = self
                .global_signature_registry
                .as_ref()
                .and_then(|g| g.get_signature(callee_name).cloned());
            let from_simple = if self.inline_module_qualified_call(callee_name) {
                registry.get_signature(simple).cloned().or_else(|| {
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple).cloned())
                })
            } else {
                None
            };
            crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                from_global,
                from_reg,
                from_simple,
                from_local,
            ])
        }
        .or_else(|| registry.lookup_method(callee_name).cloned());
        let sig_for_owned_literal = sig.clone();
        // Upgrade stale owned WJ `string` formals to defining-module `&str` emission.
        // Never consult bare method-name keys for type-qualified calls (`App::record_resource`)
        // — homonyms can replace the whole signature with unrelated Borrowed formals.
        if let Some(ref base) = sig {
            let pidx = base.arg_param_index(arg_index);
            let mut upgraded = sig.clone();
            let skip_bare_homonym =
                crate::codegen::rust::call_signature_resolution::qualified_callee_skips_bare_homonym_lookup(
                    callee_name,
                );
            let challengers: Vec<Option<&crate::analyzer::FunctionSignature>> =
                if skip_bare_homonym || import_alias_resolved {
                    vec![
                        self.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(lookup)),
                        registry.get_signature(lookup),
                    ]
                } else {
                    vec![
                        self.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(callee_name)),
                        self.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(simple)),
                        registry.get_signature(callee_name),
                        registry.get_signature(simple),
                    ]
                };
            for challenger in challengers {
                upgraded =
                    crate::codegen::rust::signature_promotion::prefer_shared_text_ref_signature(
                        upgraded, challenger, pidx,
                    );
            }
            sig = upgraded;
        }

        if let Some((receiver_ty, method)) = callee_name.rsplit_once("::") {
            // Only Type::method (uppercase receiver). Runtime modules (`strings::join`)
            // and lowercase module paths must keep the free-function / prefer-shared sig —
            // resolving `strings` as a type picks Vec::join-style owned contracts and
            // forces `parts.clone()` into `&[String]` formals (flat lib.wj / wj test).
            if crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                callee_name,
            ) {
                let arg_count = user_arg_count.unwrap_or(arg_index + 1);
                let inferred_recv = receiver_type_name.and_then(|rt| {
                    crate::codegen::rust::stdlib_signature_specialization::receiver_type_from_name_and_hint(
                        Some(rt),
                        None,
                        self.current_function_return_type.as_ref(),
                    )
                });
                if let Some(method_sig) = self.resolve_method_function_signature_specialized(
                    receiver_ty,
                    method,
                    arg_count,
                    inferred_recv.as_ref(),
                ) {
                    let prefer_method = sig.as_ref().is_none_or(|local| {
                        let local_idx = local.arg_param_index(arg_index);
                        let method_idx = method_sig.arg_param_index(arg_index);
                        // Keep local owned / Copy-aggregate contracts over stale method Ref wraps
                        // (regression-060 `other: Lsn` must not become `&through` via method_ref prefer).
                        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            local, local_idx,
                        ) {
                            return false;
                        }
                        if local.formal_param_type(local_idx).is_some_and(|t| {
                            let bare = match t {
                                Type::Reference(inner) | Type::MutableReference(inner) => {
                                    inner.as_ref()
                                }
                                other => other,
                            };
                            self.is_type_copy(bare)
                                && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                        }) {
                            return false;
                        }
                        let local_ref = local
                            .param_types
                            .get(local_idx)
                            .is_some_and(|t| matches!(t, Type::Reference(_)));
                        let method_idx = method_sig.arg_param_index(arg_index);
                        let copy_aggregate_method = method_sig
                            .formal_param_type(method_idx)
                            .is_some_and(|t| {
                                let bare = match t {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        inner.as_ref()
                                    }
                                    other => other,
                                };
                                self.is_type_copy(bare)
                                    && !crate::type_classification::is_copy_pass_by_value_formal(
                                        bare,
                                    )
                            });
                        let method_emits_shared = crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                            &method_sig, method_idx,
                        );
                        let local_owned = matches!(
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                                local, local_idx,
                            ),
                            crate::analyzer::OwnershipMode::Owned
                        );
                        (method_emits_shared && !local_ref && !copy_aggregate_method)
                            || (local_owned && method_emits_shared && !copy_aggregate_method)
                    }) || crate::codegen::rust::signature_promotion::method_registry_reflects_emitted_owned(
                        &method_sig,
                    ) || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &method_sig,
                        method_sig.arg_param_index(arg_index),
                    );
                    if prefer_method {
                        sig = Some(method_sig);
                    }
                }
            }
        }

        if let Some(global) = self.global_signature_registry.as_ref() {
            if let Some(global_sig) = global.get_signature(callee_name) {
                let global_idx = global_sig.arg_param_index(arg_index);
                let method_registry_owned = crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                    callee_name,
                ) && callee_name.rsplit_once("::").is_some_and(
                    |(receiver_ty, method)| {
                        let arg_count = user_arg_count.unwrap_or(arg_index + 1);
                        self.resolve_method_function_signature(receiver_ty, method, arg_count)
                            .map(|method_sig| {
                                crate::codegen::rust::signature_promotion::method_registry_reflects_emitted_owned(
                                    &method_sig,
                                ) || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                    &method_sig,
                                    method_sig.arg_param_index(arg_index),
                                )
                            })
                            .unwrap_or(false)
                    },
                );
                let prefer_global = !method_registry_owned
                    && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        global_sig, global_idx,
                    )
                    && sig.as_ref().is_none_or(|local_sig| {
                        let idx = local_sig.arg_param_index(arg_index);
                        if local_sig
                            .formal_param_type(idx)
                            .or_else(|| local_sig.param_types.get(idx))
                            .is_some_and(
                                crate::codegen::rust::stdlib_method_traits::formal_is_rust_closure_trait,
                            )
                        {
                            return false;
                        }
                        // Never replace codegen-owned / Copy-aggregate formals with a stale
                        // global Borrowed wrap (regression-060 `is_at_or_before(&through)` vs `other: Lsn`).
                        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            local_sig, idx,
                        ) {
                            return false;
                        }
                        // P3.254: bare user owned WJ `string` must not yield to a stdlib/
                        // global `get` (etc.) Borrowed homonym — keep the defining-module
                        // owned contract so call sites move, not `&text`.
                        if !callee_name.contains("::")
                            && !local_sig.name.contains("::")
                            && !crate::codegen::rust::stdlib_method_traits::callee_path_is_runtime_std(
                                &local_sig.name,
                            )
                            && (crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                                local_sig, idx,
                            ) || local_sig.param_types.get(idx).is_some_and(|t| {
                                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                    && crate::codegen::rust::types::is_windjammer_text_type(t)
                            }))
                            && (matches!(
                                local_sig.param_ownership.get(idx),
                                Some(crate::analyzer::OwnershipMode::Owned)
                            ) || local_sig
                                .emitted_rust_ref_params
                                .as_ref()
                                .and_then(|f| f.get(idx))
                                .copied()
                                == Some(false))
                        {
                            return false;
                        }
                        if local_sig.formal_param_type(idx).is_some_and(|t| {
                            let bare = match t {
                                Type::Reference(inner) | Type::MutableReference(inner) => {
                                    inner.as_ref()
                                }
                                other => other,
                            };
                            self.is_type_copy(bare)
                                && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                        }) {
                            return false;
                        }
                        let local_eff =
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                            local_sig, idx,
                        );
                        let global_idx = global_sig.arg_param_index(arg_index);
                        let global_eff =
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                            global_sig, global_idx,
                        );
                        let global_ref_ty = global_sig
                            .param_types
                            .get(global_idx)
                            .is_some_and(|t| matches!(t, Type::Reference(_)));
                        let local_owned =
                            matches!(local_eff, crate::analyzer::OwnershipMode::Owned);
                        let global_borrows = matches!(
                            global_eff,
                            crate::analyzer::OwnershipMode::Borrowed
                                | crate::analyzer::OwnershipMode::MutBorrowed
                        ) || global_ref_ty
                            || matches!(
                                global_sig.param_ownership.get(global_idx),
                                Some(crate::analyzer::OwnershipMode::Borrowed)
                                    | Some(crate::analyzer::OwnershipMode::MutBorrowed)
                            );
                        local_owned && global_borrows
                    });
                if prefer_global {
                    sig = Some(global_sig.clone());
                }
            }
        }

        if callee_name.starts_with("Self::") {
            if let Some(ref tn) = self.current_struct_name {
                if let Some(method) = callee_name.strip_prefix("Self::") {
                    if let Some(ms) = self.lookup_method_signature(tn, method) {
                        sig = Some(ms.to_function_signature());
                    }
                }
            }
        }

        let Some(mut sig) = sig else {
            // Unresolved non-boundary callees (stdlib Pattern, bare names): finish
            // via runtime/registry helpers — never name-based ownership guesses.
            let mut finished = self.finish_runtime_std_call_arg(
                callee_name,
                arg_index,
                arg_expr,
                prepared_arg,
                None,
                receiver_type_name,
            );
            if let Expression::Identifier { name, .. } = arg_expr {
                if self.into_string_formal_params.contains(name) {
                    while finished.starts_with("&mut ") {
                        finished = finished["&mut ".len()..].trim().to_string();
                    }
                    while finished.starts_with('&') {
                        finished = finished[1..].trim().to_string();
                    }
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut finished);
                    if !finished.ends_with(".into()")
                        && !finished.ends_with(".to_string()")
                        && !finished.ends_with(".to_owned()")
                    {
                        finished = format!("{finished}.into()");
                    }
                    return Some(finished);
                }
            }
            self.apply_registry_borrow_to_call_arg(
                &mut finished,
                arg_expr,
                receiver_type_name,
                method_simple,
                arg_index,
                user_arg_count,
            );
            // Unresolved stdlib Pattern/`&str` methods still need `&needle` when the
            // registry knows the formal is `&str` (e.g. `String::find` in stdlib_meta).
            if !finished.starts_with('&')
                && crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_qualified(
                    method_simple,
                    receiver_type_name.or(Some("String")),
                    registry,
                    arg_index,
                )
                && !crate::codegen::rust::call_site_borrow::expression_is_copy_literal(arg_expr)
            {
                crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                    arg_expr,
                    &mut finished,
                );
                if !finished.starts_with('&')
                    && !crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
                {
                    finished = crate::ir::target_encodings::rust_shared_borrow(&finished);
                }
            }
            // Unresolved callee still needs multi-use clones (regression-063 seed_write).
            finished = self.maybe_auto_clone_call_arg(
                arg_expr,
                &finished,
                Some(callee_name),
                Some(arg_index),
            );
            if matches!(
                arg_expr,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            ) && crate::codegen::rust::string_utilities::type_qualified_associated_string_literal_needs_rust_owned_string(
                callee_name,
                arg_index,
                None,
                registry,
                self.global_signature_registry.as_deref(),
            ) && !crate::codegen::rust::string_utilities::already_owned_string_expr(&finished)
            {
                finished = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                    finished.trim_start_matches('&'),
                );
            }
            return Some(
                crate::codegen::rust::expression_utilities::sanitize_cast_trailing_clone(&finished),
            );
        };

        // Specialize stdlib generics (`Vec::push(T)` → `push(String)` on `Vec<String>`).
        if let Some(recv_ty) =
            crate::codegen::rust::stdlib_signature_specialization::receiver_type_from_name_and_hint(
                receiver_type_name,
                None,
                self.current_function_return_type.as_ref(),
            )
        {
            crate::codegen::rust::stdlib_signature_specialization::specialize_signature_for_receiver(
                &mut sig, &recv_ty,
            );
        }

        // Unified local+global resolution (same as `mc_resolve_method_call_signature`) so
        // stale per-caller registry stubs do not beat converged defining-module metadata.
        if let Some(rt) = receiver_type_name {
            if let Some(resolved) =
                crate::codegen::rust::call_signature_resolution::resolve_method_for_call_site_in_module(
                    registry,
                    self.global_signature_registry.as_deref(),
                    rt,
                    method_simple,
                    user_arg_count.unwrap_or(arg_index + 1),
                    self.current_caller_module_path().as_deref(),
                )
            {
                sig = resolved.sig;
            }
            // Fallback: per-caller registry snapshots can predate defining-module
            // convergence; trust the merged global entry when it expects shared borrow.
            // Never downgrade codegen-owned Copy-aggregate locals (regression-060
            // `is_at_or_before(&through)` vs emitted `other: Lsn`). Non-Copy owned
            // formals (Key::has_key) may still need global convergence for clone paths.
            if let Some(global) = self.global_signature_registry.as_deref() {
                let qualified = format!("{rt}::{method_simple}");
                if let Some(gs) = global.get_signature(&qualified) {
                    let gidx = gs.arg_param_index(arg_index);
                    if crate::ir::signature_bridge::call_site_expects_shared_borrow(gs, gidx) {
                        let pidx = sig.arg_param_index(arg_index);
                        let local_copy_aggregate_owned = sig
                            .formal_param_type(pidx)
                            .is_some_and(|t| {
                                let bare = match t {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        inner.as_ref()
                                    }
                                    other => other,
                                };
                                self.is_type_copy(bare)
                                    && !crate::type_classification::is_copy_pass_by_value_formal(
                                        bare,
                                    )
                            })
                            && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                &sig, pidx,
                            )
                                || sig
                                    .emitted_rust_ref_params
                                    .as_ref()
                                    .and_then(|flags| flags.get(pidx))
                                    .copied()
                                    != Some(true));
                        let local_bare_owned_user =
                            crate::codegen::rust::signature_promotion::bare_formal_is_owned_user_type(
                                &sig, pidx,
                            );
                        if !crate::ir::signature_bridge::call_site_expects_shared_borrow(&sig, pidx)
                            && !local_copy_aggregate_owned
                            && !local_bare_owned_user
                        {
                            sig = gs.clone();
                        }
                    }
                }
            }
        }

        // Method callees: always attempt defining-module codegen refresh merge so
        // `emitted_rust_ref_params` / owned Copy-aggregate formals beat stale analyzer
        // `Reference(Lsn)` stubs (`&through` into `other: Lsn`, regression-060). No-op when the
        // registry lacks refresh metadata.
        if let Some(rt) = receiver_type_name {
            let qualified = format!("{rt}::{method_simple}");
            let refresh_keys = vec![qualified, callee_name.to_string()];
            crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                &mut sig,
                registry,
                &refresh_keys,
            );
            if let Some(global) = self.global_signature_registry.as_ref() {
                crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                    &mut sig,
                    global,
                    &refresh_keys,
                );
            }
        }

        // Inline `mod gpu { … }` callees: don't coerce string literals to owned String
        // when module origin can't be verified (single-file conservative guard).
        if self.inline_module_qualified_call(callee_name)
            && matches!(
                arg_expr,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            )
        {
            return Some(prepared_arg);
        }

        let receiver_is_set = receiver_type_name
            .is_some_and(crate::codegen::rust::stdlib_method_traits::is_set_type_name);

        // Refresh free-fn signatures from the codegen registry before expected-type /
        // coercion decisions — analyzer stubs often still say bare `string`+Owned while
        // the defining-fn refresh recorded `&str` (`process("hello")` must stay bare).
        // Mirror method path: merge defining-module `emitted_rust_ref_params` so owned
        // Custom formals (`csr: DenseCsr`) beat WDB-097 cold-meta Borrowed stubs.
        // Skip type-qualified associated calls (`ServerResponse::error`) — bare `error`
        // would merge `log::error`'s `&str` formal onto owned i64 status.
        if receiver_type_name.is_none()
            && !crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                callee_name,
            )
        {
            let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
            let lookup_callee = self.signature_lookup_callee_name(callee_name);
            let lookup_ref = lookup_callee.as_ref();
            let skip_bare_spawn_homonym =
                crate::codegen::rust::call_signature_resolution::qualified_callee_skips_bare_homonym_lookup(
                    callee_name,
                );
            let refresh_keys = if skip_bare_spawn_homonym {
                vec![callee_name.to_string(), lookup_ref.to_string()]
            } else {
                vec![
                    callee_name.to_string(),
                    lookup_ref.to_string(),
                    simple.to_string(),
                ]
            };
            crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                &mut sig,
                registry,
                &refresh_keys,
            );
            if let Some(global) = self.global_signature_registry.as_ref() {
                crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                    &mut sig,
                    global,
                    &refresh_keys,
                );
                for key in refresh_keys.iter().map(String::as_str) {
                    if let Some(reg) = global.lookup_method(key) {
                        if reg.emitted_rust_ref_params.is_some() {
                            crate::codegen::rust::signature_promotion::merge_codegen_refresh_metadata(
                                &mut sig,
                                reg,
                            );
                            break;
                        }
                    }
                }
            }
            let homonym_sigs = if skip_bare_spawn_homonym {
                [None, None, None, None]
            } else {
                [
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple).cloned()),
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.lookup_method(simple).cloned()),
                    self.signature_registry.get_signature(simple).cloned(),
                    self.signature_registry.lookup_method(simple).cloned(),
                ]
            };
            if let Some(refreshed) =
                crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(lookup_ref).cloned()),
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(callee_name).cloned()),
                    homonym_sigs[0].clone(),
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.lookup_method(callee_name).cloned()),
                    homonym_sigs[1].clone(),
                    self.signature_registry.get_signature(callee_name).cloned(),
                    homonym_sigs[2].clone(),
                    self.signature_registry.lookup_method(callee_name).cloned(),
                    homonym_sigs[3].clone(),
                    Some(sig.clone()),
                ])
            {
                let ridx = refreshed.arg_param_index(arg_index);
                let current_pidx = sig.arg_param_index(arg_index);
                let current_mut = matches!(
                    sig.param_ownership.get(current_pidx),
                    Some(crate::analyzer::OwnershipMode::MutBorrowed)
                ) || sig
                    .param_types
                    .get(current_pidx)
                    .is_some_and(|t| matches!(t, Type::MutableReference(_)));
                let refreshed_mut = matches!(
                    refreshed.param_ownership.get(ridx),
                    Some(crate::analyzer::OwnershipMode::MutBorrowed)
                ) || refreshed
                    .param_types
                    .get(ridx)
                    .is_some_and(|t| matches!(t, Type::MutableReference(_)));
                if refreshed.emitted_rust_ref_params.is_some()
                    || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                        &refreshed, ridx,
                    )
                    || refreshed.param_types.get(ridx).is_some_and(|t| {
                        crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                    })
                    || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &refreshed, ridx,
                    )
                {
                    // AST Owned stubs must not overwrite a live MutBorrowed contract
                    // (`apply_rotation(t: &mut Transform)`).
                    if !(current_mut
                        && !refreshed_mut
                        && crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            &refreshed, ridx,
                        ))
                    {
                        sig = refreshed;
                    }
                }
            }
        }

        // Final associated-call refresh: importer stubs may carry all-false
        // `emitted_rust_ref_params` while the defining module published `[true]`.
        if let Some(refreshed) = self.refresh_call_site_signature_for_arg(
            if let Some(rt) = receiver_type_name {
                self.resolve_method_function_signature(
                    rt,
                    method_simple,
                    user_arg_count.unwrap_or(arg_index + 1),
                )
            } else {
                None
            },
            callee_name,
            arg_index,
        ) {
            sig = refreshed;
        }
        // Body-converged `&str` refresh must not undo trait owned `string` contracts
        // (`authenticate(email: string)` → never `&request.email`).
        if let Some(global) = self.global_signature_registry.as_ref() {
            crate::codegen::rust::call_signature_resolution::apply_trait_owned_string_call_site_contracts(
                global,
                method_simple,
                &mut sig,
            );
            sig =
                crate::codegen::rust::call_signature_resolution::finalize_call_site_signature(sig);
        }
        crate::codegen::rust::signature_promotion::restore_stdlib_collection_key_contract(
            &mut sig,
            Some(callee_name),
        );
        if let Some(recv) = crate::codegen::rust::stdlib_method_traits::collection_key_receiver_type(
            callee_name,
            receiver_type_name,
            &sig,
        ) {
            let map_qualified = format!("{recv}::{}", method_simple);
            crate::codegen::rust::signature_promotion::restore_stdlib_collection_key_contract(
                &mut sig,
                Some(&map_qualified),
            );
        }
        if receiver_type_name.is_none() {
            sig =
                crate::codegen::rust::signature_promotion::local_user_fn_beats_runtime_std_homonym(
                    registry,
                    callee_name,
                    sig,
                );
            self.sync_call_sig_from_preregistered_free_fn_emission(callee_name, &mut sig);
        }

        // WDB-332: leaf-name homonyms (`AudioChannel::new`) — caller-module affinity is
        // final after refresh/prefer_method/global bare-key challenges, which otherwise
        // re-poison formals (i32 → String → `.to_string()`).
        if crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
            callee_name,
        ) {
            let arg_count = user_arg_count.unwrap_or(arg_index + 1);
            if let Some((receiver_ty, method)) = callee_name.rsplit_once("::") {
                if let Some(affinity_sig) =
                    self.lookup_method_signature_on_receiver_type(receiver_ty, method, arg_count)
                {
                    sig = affinity_sig;
                }
            }
        }

        // Constraint write-back: MutBorrowed + bare T (including Copy aggregates)
        // becomes MutableReference so expected ownership is MutRef, not owned-mut.
        crate::codegen::rust::signature_promotion::wrap_converged_borrow_param_types(&mut sig);

        let mut param_idx = sig.arg_param_index(arg_index);
        let mut expected = safety_type_from_signature_param(&sig, param_idx);
        if sig
            .formal_param_type(param_idx)
            .or_else(|| sig.param_types.get(param_idx))
            .is_some_and(crate::codegen::rust::stdlib_method_traits::formal_is_rust_closure_trait)
        {
            expected.ownership = OwnedType::Owned;
        }
        if (crate::codegen::rust::signature_promotion::bare_formal_is_vec_or_map(&sig, param_idx)
            || crate::codegen::rust::signature_promotion::bare_formal_is_owned_user_type(
                &sig, param_idx,
            )
            || crate::ir::emission_contract::plain_string_formal_passes_owned_at_call_site(
                &sig, param_idx,
            )
            || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                &sig, param_idx,
            ))
            && !crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(&sig, param_idx)
        {
            if let Some(bare) = crate::ir::signature_bridge::bare_wj_formal_type(&sig, param_idx) {
                expected = crate::ir::signature_bridge::safety_type_from_parser_type(
                    bare,
                    Some(crate::analyzer::OwnershipMode::Owned),
                );
            }
        }
        // Demoted Custom/`&T` emit: prefer Ref expected so owned locals Borrow (WDB-212).
        // Runs after owned-user-type promotion so emission flags win over bare Custom Owned.
        if crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(&sig, param_idx)
            && !Self::sig_arg_confirms_owned_emission(&sig, arg_index)
        {
            expected.ownership = OwnedType::Ref(Region::fresh(13));
        }
        if collecting_ref_vec {
            if let Some(rt) = self.current_function_return_type.as_ref() {
                let elem_ref: Option<&Type> = match rt {
                    Type::Vec(inner) => Some(inner.as_ref()),
                    Type::Parameterized(name, args) if name == "Vec" && args.len() == 1 => {
                        args.first().map(|t| t as &Type)
                    }
                    _ => None,
                };
                if let Some(Type::Reference(inner) | Type::MutableReference(inner)) = elem_ref {
                    expected = safety_type_from_parser_type(
                        &Type::Reference(inner.clone()),
                        Some(crate::analyzer::OwnershipMode::Borrowed),
                    );
                }
            }
        }
        // Match-arm owned String payloads borrow as &str at *shared-ref* text callees.
        // Owned WJ `string` formals (`generate_page(markdown: string)`) must still move.
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.match_arm_bindings.contains(name.as_str()) {
                let expects_shared_text =
                    crate::ir::signature_bridge::call_site_wants_shared_text_ref(&sig, param_idx);
                if expects_shared_text {
                    expected.base = BaseType::String;
                    expected.ownership = OwnedType::Ref(Region::fresh(12));
                }
            }
        }
        let callee_module = crate::codegen::rust::stdlib_method_traits::resolve_runtime_std_module(
            callee_name.split("::").next().unwrap_or(""),
            receiver_type_name,
        );
        let inferred_arg_type = self.infer_expression_type(arg_expr);
        let runtime_param_type = sig
            .formal_param_type(param_idx)
            .or(inferred_arg_type.as_ref());
        if let Some(formal_ty) = runtime_param_type {
            if crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                registry,
                callee_name,
                Some(&sig),
                arg_index,
            ) {
                if crate::codegen::rust::types::is_windjammer_text_type(formal_ty) {
                    expected.base = BaseType::String;
                }
                expected.ownership = OwnedType::Ref(Region::fresh(7));
            } else if crate::codegen::rust::string_utilities::param_is_rust_str_ref(formal_ty) {
                expected.base = BaseType::String;
                expected.ownership = OwnedType::Ref(Region::fresh(5));
            }
        }
        if sig.param_types.get(param_idx).is_some_and(|t| {
            crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
        }) && (crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
            &sig, param_idx,
        ) || !crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
            &sig, param_idx,
        ))
        {
            expected.base = BaseType::String;
            expected.ownership = OwnedType::Ref(Region::fresh(5));
        } else if matches!(
            sig.param_ownership.get(param_idx),
            Some(crate::analyzer::OwnershipMode::Borrowed)
        ) {
            let formal = sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx));
            if formal.is_some_and(|t| {
                matches!(t, Type::Custom(_))
                    && !crate::codegen::rust::types::is_windjammer_text_type(t)
                    && !self.is_type_copy(t)
            }) {
                expected.ownership = OwnedType::Ref(Region::fresh(12));
            }
        } else if let Some(ownership) = sig.param_ownership.get(param_idx).copied() {
            use crate::analyzer::OwnershipMode;
            // Region(8): shared Borrowed only — never demote MutBorrowed/`&mut T` to Ref.
            if matches!(ownership, OwnershipMode::Borrowed)
                && crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
                    &sig, arg_index,
                )
            {
                // Scanned runtime AsRef/&str formals: keep owned WJ text as Ref at call site.
                if crate::codegen::rust::types::is_windjammer_text_type(
                    sig.formal_param_type(param_idx)
                        .or_else(|| sig.param_types.get(param_idx))
                        .unwrap_or(&Type::String),
                ) || runtime_param_type
                    .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
                {
                    expected.base = BaseType::String;
                }
                expected.ownership = OwnedType::Ref(Region::fresh(8));
            }
        }
        if crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_from_sig(
            &sig,
            arg_index,
        ) && !crate::codegen::rust::call_site_borrow::expression_is_copy_literal(arg_expr)
        {
            // `str::find` / `contains` / … take `Pattern` (`&str`, `char`, …).
            // String literals are already `&str`; owned `String` needles need `&`.
            expected.base = BaseType::String;
            expected.ownership = OwnedType::Ref(Region::fresh(9));
        } else if {
            let key_receiver =
                crate::codegen::rust::stdlib_method_traits::collection_key_receiver_type(
                    callee_name,
                    receiver_type_name,
                    &sig,
                );
            self.is_collection_key_lookup_at_site(
                &sig,
                arg_index,
                key_receiver.as_deref(),
            )
        } || (arg_index == 0
            && receiver_is_set
            && crate::codegen::rust::stdlib_method_traits::method_arg_expects_borrowed_reference_from_sig(
                &sig, arg_index,
            ))
        {
            // Map/set lookup formals are always `&K` at the call site. Even when the
            // caller binding is already `&str` / `&T`, keep expected=Ref so Identity
            // wins (`.get(key)`) — not Owned (which would spuriously `.to_string()`).
            expected.ownership = OwnedType::Ref(Region::fresh(4));
        } else if !matches!(expected.ownership, OwnedType::Ref(_))
            && !crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(&sig, param_idx)
            && crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
                &sig, arg_index,
            )
            && matches!(expected.base, BaseType::String | BaseType::Custom(_))
            && !crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
                &sig, arg_index,
            )
        {
            expected.ownership = OwnedType::Owned;
        }
        // Registry-aware Copy aggregate → owned callee formal (regression-060 `through: Lsn`).
        if let Expression::Identifier { name, .. } = arg_expr {
            let caller_copy_aggregate = self.current_function_params.iter().any(|p| {
                p.name == *name
                    && self.is_type_copy(&p.type_)
                    && !crate::type_classification::is_copy_pass_by_value_formal(&p.type_)
            });
            if caller_copy_aggregate {
                let callee_copy_owned =
                    crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &sig, param_idx,
                    ) || sig
                        .emitted_rust_ref_params
                        .as_ref()
                        .and_then(|flags| flags.get(param_idx))
                        .copied()
                        == Some(false)
                        || sig
                            .formal_param_type(param_idx)
                            .or_else(|| sig.param_types.get(param_idx))
                            .is_some_and(|t| {
                                let bare = match t {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        inner.as_ref()
                                    }
                                    other => other,
                                };
                                self.is_type_copy(bare)
                                    && !crate::type_classification::is_copy_pass_by_value_formal(
                                        bare,
                                    )
                            });
                if callee_copy_owned
                    && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                        &sig, param_idx,
                    )
                {
                    if let Some(bare) =
                        crate::ir::signature_bridge::bare_wj_formal_type(&sig, param_idx)
                    {
                        expected = crate::ir::signature_bridge::safety_type_from_parser_type(
                            bare,
                            Some(crate::analyzer::OwnershipMode::Owned),
                        );
                    }
                }
            }
        }
        let actual = self.infer_actual_safety_type(arg_expr, prepared_arg.as_str());
        let thin_wrap_into_string_formal = matches!(
            arg_expr,
            Expression::Identifier { name, .. } if self.into_string_formal_params.contains(name)
        );
        let mut kind = compute_coercion(&actual, &expected);
        // P3.402: Copy literals (incl. char) must pass by value — never Borrow→`&'.'`.
        if matches!(kind, CoercionKind::Borrow | CoercionKind::MutBorrow)
            && crate::codegen::rust::call_site_borrow::expression_is_copy_literal(arg_expr)
        {
            kind = CoercionKind::Identity;
        }
        // WDB-367: unit keywords are constructors, not bindings — never `None.clone()`.
        if matches!(kind, CoercionKind::Clone)
            && matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if name == "None" || name == "true" || name == "false"
            )
        {
            kind = CoercionKind::Identity;
        }
        // WDB-343: Copy cast targets must not receive trailing `.clone()` at call sites.
        if matches!(kind, CoercionKind::Clone)
            && matches!(arg_expr, Expression::Cast { type_, .. } if self.is_type_copy(type_))
        {
            kind = CoercionKind::Identity;
        }
        // WDB-170 / eco wj-toml: demoted `&str` into owned `String` must `.to_string()`, not `.clone()`.
        if matches!(kind, CoercionKind::Clone)
            && crate::ir::coercion::is_string_base(&expected.base)
            && matches!(expected.ownership, OwnedType::Owned)
            && matches!(actual.ownership, OwnedType::Ref(_))
            && crate::ir::coercion::is_string_base(&actual.base)
        {
            kind = CoercionKind::ToOwnedString;
        }
        if matches!(kind, CoercionKind::Clone)
            && crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(&sig, param_idx)
            && !Self::sig_arg_confirms_owned_emission(&sig, arg_index)
        {
            kind = CoercionKind::Borrow;
        }
        // WDB-212 / Custom demotion: owned binding into demoted `&T` at emit must Borrow
        // even without `.clone()` (Clone→Borrow above only covers cloned args).
        if matches!(kind, CoercionKind::Identity)
            && crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(&sig, param_idx)
            && !Self::sig_arg_confirms_owned_emission(&sig, arg_index)
            && !prepared_arg.starts_with('&')
            && !prepared_arg.starts_with("&mut ")
            // WDB-329: `idx as usize` into Vec::remove Owned usize — never promote to Borrow
            // (`&idx as usize` / `&(idx as usize)` are both wrong).
            && !matches!(
                arg_expr,
                Expression::Cast { type_, .. }
                    if crate::codegen::rust::type_casting::type_is_usize(type_)
                        || self.is_type_copy(type_)
            )
        {
            kind = CoercionKind::Borrow;
        }
        // Pub `impl Into<String>` forwards (wj-mime): move once — never `.clone()` / `&`.
        if thin_wrap_into_string_formal
            && matches!(
                kind,
                CoercionKind::Clone
                    | CoercionKind::ToOwnedString
                    | CoercionKind::Borrow
                    | CoercionKind::MutBorrow
            )
        {
            kind = CoercionKind::Identity;
        }
        let key_receiver_for_coercion =
            crate::codegen::rust::stdlib_method_traits::collection_key_receiver_type(
                callee_name,
                receiver_type_name,
                &sig,
            );
        if self.is_collection_key_lookup_at_site(
            &sig,
            arg_index,
            key_receiver_for_coercion.as_deref(),
        ) {
            if matches!(kind, CoercionKind::ToOwnedString | CoercionKind::Clone) {
                kind = CoercionKind::Borrow;
            }
            expected.ownership = OwnedType::Ref(Region::fresh(4));
        }
        let explicit_move_closure = matches!(
            arg_expr,
            Expression::Binary {
                op: crate::parser::BinaryOp::Or,
                left,
                ..
            } if matches!(
                &**left,
                Expression::Identifier { name, .. } if name == "move"
            )
        );
        if matches!(arg_expr, Expression::Closure { .. })
            || explicit_move_closure
            || sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx))
                .is_some_and(
                    crate::codegen::rust::stdlib_method_traits::formal_is_rust_closure_trait,
                )
        {
            kind = CoercionKind::Identity;
            expected.ownership = OwnedType::Owned;
        }
        // `rows[i]` / `self.field` into owned non-Copy formals: clone when the root cannot
        // move (shared/`&mut self`, or WJ bare `self` that emits `&self`). Always cloning
        // `self.field` into Owned is correct for `&self` (E0507) and harmless for owned-self.
        let field_from_self = matches!(
            arg_expr,
            Expression::FieldAccess { object, .. }
                if matches!(&**object, Expression::Identifier { name, .. } if name == "self")
        );
        // Shared-ref at emit only — analyzer `Borrowed` alone must not demote Clone→Borrow
        // for bare owned Vec formals (WDB-281: `contains(items.clone())`, not `&items`).
        let callee_wants_shared =
            crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&sig, param_idx)
                || crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(&sig, param_idx)
                || crate::ir::signature_bridge::call_site_expects_shared_borrow(&sig, param_idx);
        if matches!(expected.ownership, OwnedType::Owned)
            && !prepared_arg.ends_with(".clone()")
            && !callee_wants_shared
            && (matches!(arg_expr, Expression::Index { .. })
                || (matches!(arg_expr, Expression::FieldAccess { .. })
                    && (self.field_access_root_is_behind_reference(arg_expr) || field_from_self)))
        {
            let elem_needs_clone = self.infer_expression_type(arg_expr).map_or_else(
                || matches!(expected.base, BaseType::Custom(_) | BaseType::String),
                |t| {
                    let bare = match &t {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    !self.is_type_copy(bare)
                },
            );
            if elem_needs_clone {
                kind = CoercionKind::Clone;
            }
        } else if callee_wants_shared
            && !Self::sig_arg_confirms_owned_emission(&sig, arg_index)
            && matches!(kind, CoercionKind::Clone)
            && (matches!(arg_expr, Expression::Index { .. })
                || matches!(arg_expr, Expression::FieldAccess { .. })
                || matches!(arg_expr, Expression::Identifier { .. }))
        {
            kind = CoercionKind::Borrow;
        }
        if matches!(kind, CoercionKind::ToOwnedString)
            && (crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&sig, param_idx)
                || crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(&sig, param_idx)
                || (crate::codegen::rust::types::is_windjammer_text_type(
                    sig.formal_param_type(param_idx)
                        .or_else(|| sig.param_types.get(param_idx))
                        .unwrap_or(&Type::String),
                ) && matches!(
                    sig.param_ownership.get(param_idx),
                    Some(crate::analyzer::OwnershipMode::Borrowed)
                ))
                || self.ir_callee_arg_expects_shared_borrow(
                    registry,
                    callee_name,
                    arg_index,
                    user_arg_count,
                    local_sig,
                ))
        {
            kind = CoercionKind::Identity;
        }
        if matches!(kind, CoercionKind::Borrow | CoercionKind::MutBorrow) {
            if let Expression::Identifier { name, .. } = arg_expr {
                let caller_copy_aggregate = self.current_function_params.iter().any(|p| {
                    p.name == *name
                        && self.is_type_copy(&p.type_)
                        && !crate::type_classification::is_copy_pass_by_value_formal(&p.type_)
                });
                if caller_copy_aggregate {
                    let callee_copy_owned = crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &sig, param_idx,
                    ) || sig
                        .emitted_rust_ref_params
                        .as_ref()
                        .and_then(|flags| flags.get(param_idx))
                        .copied()
                        == Some(false)
                        || sig
                            .formal_param_type(param_idx)
                            .or_else(|| sig.param_types.get(param_idx))
                            .is_some_and(|t| {
                                let bare = match t {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        inner.as_ref()
                                    }
                                    other => other,
                                };
                                self.is_type_copy(bare)
                                    && !crate::type_classification::is_copy_pass_by_value_formal(
                                        bare,
                                    )
                            });
                    if callee_copy_owned
                        && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                            &sig, param_idx,
                        )
                    {
                        kind = CoercionKind::Identity;
                    }
                }
            }
        }
        if matches!(kind, CoercionKind::Clone)
            && matches!(expected.ownership, OwnedType::Ref(_))
            && crate::ir::coercion::is_string_base(&expected.base)
            && matches!(actual.ownership, OwnedType::Owned | OwnedType::Copy)
        {
            kind = CoercionKind::Borrow;
        }
        if crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
            && (crate::codegen::rust::stdlib_method_traits::runtime_wj_owned_rust_borrowed_param(
                &sig, arg_index,
            ) || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                &sig, param_idx,
            ) || self.ir_callee_arg_expects_shared_borrow(
                registry,
                callee_name,
                arg_index,
                user_arg_count,
                local_sig,
            ))
            && matches!(kind, CoercionKind::Clone | CoercionKind::ToOwnedString)
        {
            kind = CoercionKind::Identity;
        }
        if matches!(kind, CoercionKind::Clone)
            && self.callee_param_field_extracts_by_name(callee_name, arg_index)
        {
            kind = CoercionKind::Identity;
        }
        if matches!(kind, CoercionKind::Clone) {
            if let Expression::Identifier { name, .. } = arg_expr {
                if self.in_user_written_closure && self.user_closure_params.contains(name) {
                    if self.ir_sig_arg_expects_shared_borrow(&sig, arg_index) {
                        kind = CoercionKind::Borrow;
                    } else {
                        kind = CoercionKind::Identity;
                    }
                }
            }
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            let runtime_needs_borrow =
                crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                    registry,
                    callee_name,
                    Some(&sig),
                    arg_index,
                );
            if self.binding_emits_as_rust_shared_ref(name)
                && matches!(kind, CoercionKind::Borrow | CoercionKind::MutBorrow)
                && !runtime_needs_borrow
            {
                kind = CoercionKind::Identity;
            }
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            let collects_ref_vec = crate::codegen::rust::types::return_type_is_vec_of_shared_refs(
                self.current_function_return_type.as_ref(),
            );
            let is_borrowed_loop_elem = self.borrowed_iterator_vars.contains(name)
                || (collects_ref_vec
                    && self.local_var_types.get(name).is_some_and(|t| {
                        matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    }));
            if is_borrowed_loop_elem && matches!(kind, CoercionKind::Clone) {
                kind = CoercionKind::Identity;
            }
            if matches!(kind, CoercionKind::Clone) {
                let this_arg_expects_borrow =
                    self.ir_sig_arg_expects_shared_borrow(&sig, arg_index);
                let arg_is_current_fn_param =
                    self.current_function_params.iter().any(|p| p.name == *name);
                if this_arg_expects_borrow && arg_is_current_fn_param {
                    // Demoted `&str` formals pass bare; owned `String` formals need `&`.
                    if self.emitted_rust_ref_formals.contains(name)
                        || self.identifier_already_ref(name)
                        || self.inferred_borrowed_params.contains(name)
                    {
                        kind = CoercionKind::Identity;
                    } else {
                        kind = CoercionKind::Borrow;
                    }
                }
            }
        }
        // Demoted `&str` formals: strip stale `.clone()` and borrow the binding.
        // Keeping Identity here leaves `json.clone()` into `&str` (E0308 / wasteful).
        // Never strip explicit user `.clone()` — WDB-106/108.
        if prepared_arg.ends_with(".clone()")
            && matches!(kind, CoercionKind::Borrow)
            && (crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&sig, param_idx)
                || sig.param_types.get(param_idx).is_some_and(|t| {
                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                }))
            && !crate::codegen::rust::expression_helpers::is_explicit_user_clone_call(arg_expr)
        {
            prepared_arg = prepared_arg.trim_end_matches(".clone()").to_string();
        }
        // NOTE: do NOT demote Borrow→Identity on `.clone()` for other shared-ref
        // formals (WDB-222/247/249). Auto-clone leaves `map.clone()` while expected
        // is `&Map`; keep Borrow and let the strip below peel to `&map`.
        if matches!(kind, CoercionKind::Clone) {
            let formal_ty = sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx));
            if formal_ty.is_some_and(|t| self.is_type_copy(t)) {
                // Owned Copy formal: `&T` auto-copies at the call site — no `.clone()`.
                kind = CoercionKind::Identity;
            } else if let Some(ty) = self.infer_expression_type(arg_expr) {
                let pointee = match &ty {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                if self.is_type_copy(pointee) {
                    kind = CoercionKind::Identity;
                }
            } else if let Expression::Unary {
                op: crate::parser::UnaryOp::Deref,
                operand,
                ..
            } = arg_expr
            {
                if self.expression_is_copy(operand)
                    || self.infer_expression_type(operand).is_some_and(|t| {
                        matches!(
                            t,
                            Type::Reference(inner) | Type::MutableReference(inner)
                                if self.is_type_copy(inner.as_ref())
                        )
                    })
                {
                    kind = CoercionKind::Identity;
                }
            }
        }
        if matches!(kind, CoercionKind::Deref | CoercionKind::StripBorrow) {
            if let Expression::Identifier { name, .. } = arg_expr {
                if self.copy_match_payload_binding(name)
                    && !self.ir_sig_arg_expects_shared_borrow(&sig, arg_index)
                {
                    kind = CoercionKind::Identity;
                }
            }
            // Copy field access is already a value in Rust (`failure.status` → i64).
            if matches!(arg_expr, Expression::FieldAccess { .. })
                && self.expression_is_copy(arg_expr)
                && !self.ir_sig_arg_expects_shared_borrow(&sig, arg_index)
            {
                kind = CoercionKind::Identity;
            }
        }
        // Explicit `*binding` / `(*binding).field` on Copy: Rust auto-borrows — no `&*` /
        // `&(binding).field` (auto_ref_deref_copy_test).
        if crate::codegen::rust::call_site_borrow::user_wrote_explicit_deref(arg_expr)
            && matches!(kind, CoercionKind::Borrow | CoercionKind::MutBorrow)
        {
            let callee_formal_is_copy = sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx))
                .is_some_and(|t| {
                    let bare = match t {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    self.is_type_copy(bare)
                });
            if callee_formal_is_copy || self.expression_is_copy(arg_expr) {
                kind = CoercionKind::Identity;
            } else if let Expression::Unary { operand, .. } = arg_expr {
                if self
                    .infer_expression_type(operand)
                    .as_ref()
                    .is_some_and(|t| {
                        matches!(t, Type::Reference(inner) | Type::MutableReference(inner)
                        if self.is_type_copy(inner.as_ref()))
                    })
                {
                    kind = CoercionKind::Identity;
                }
            }
        }
        // Borrow coercion on `binding.clone()` / `self.field.clone()` → `&binding` /
        // `&self.field` (clone-before-borrow is redundant for shared-ref formals).
        // Never strip explicit user `.clone()` at call sites (WDB-106/108).
        if prepared_arg.ends_with(".clone()")
            && matches!(kind, CoercionKind::Borrow | CoercionKind::MutBorrow)
            && matches!(
                arg_expr,
                Expression::Identifier { .. } | Expression::FieldAccess { .. }
            )
            && !crate::codegen::rust::expression_helpers::is_explicit_user_clone_call(arg_expr)
        {
            prepared_arg = prepared_arg.trim_end_matches(".clone()").to_string();
        }
        if matches!(kind, CoercionKind::Borrow | CoercionKind::MutBorrow)
            && matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if self.match_arm_bindings.contains(name.as_str())
            )
            && !prepared_arg.starts_with('&')
        {
            let binding_is_copy = self
                .infer_expression_type(arg_expr)
                .is_some_and(|t| match t {
                    Type::Reference(inner) | Type::MutableReference(inner) => {
                        self.is_type_copy(inner.as_ref())
                    }
                    other => self.is_type_copy(&other),
                });
            if !binding_is_copy {
                prepared_arg = format!("&{prepared_arg}");
            }
        }
        // Ambiguous `Type::method` signatures (two modules define `Emitter::new` with
        // different param types): do not auto-cast int→float from the winning sig.
        // Copy autoderef already yields the expected width (`*r` for `&i32` → `i32`).
        if prepared_arg.starts_with('*')
            && matches!(expected.ownership, OwnedType::Owned | OwnedType::Copy)
        {
            // Autoderef of a Copy ref already yields the formal width — drop
            // redundant `as i32` (`double(*r as i32)` → `double(*r)`).
            for suffix in [" as i32", " as i64", " as u32", " as u64"] {
                if let Some(base) = prepared_arg.strip_suffix(suffix) {
                    prepared_arg = base.to_string();
                    break;
                }
            }
            if matches!(kind, CoercionKind::NumericCast(_)) {
                kind = CoercionKind::Identity;
            }
        }
        if matches!(kind, CoercionKind::NumericCast(_)) {
            let method = callee_name.rsplit("::").next().unwrap_or(callee_name);
            let type_name = receiver_type_name.or_else(|| {
                callee_name
                    .rsplit_once("::")
                    .map(|(q, _)| q)
                    .filter(|q| q.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
            });
            let qualified = type_name.map(|tn| format!("{tn}::{method}"));
            if self.should_skip_int_to_float_auto_cast_with_global(
                type_name,
                method,
                qualified.as_deref().or(Some(callee_name)),
            ) || self.has_collision_with_global(callee_name)
                || qualified
                    .as_ref()
                    .is_some_and(|k| self.has_collision_with_global(k))
            {
                kind = CoercionKind::Identity;
            }
        }
        let mut resolved_kind = kind;
        // WDB-169/WDB-190: callee temps autoborrow into `&T` / `&Vec<T>` — never `&callee()`.
        let call_temp_autoborrow = matches!(
            arg_expr,
            Expression::Call { .. } | Expression::MethodCall { .. }
        );
        let owned_clone_temp = prepared_arg.ends_with(".clone()")
            || prepared_arg.ends_with(".to_string()")
            || prepared_arg.ends_with(".to_owned()");
        // Call temps that rustc can autoborrow (`encode_startup()`) stay Identity.
        // Owned `.clone()` into Custom `&T` does not autoborrow (encode_line).
        let rust_autoborrows_temp = call_temp_autoborrow
            && !(owned_clone_temp && !crate::ir::coercion::is_string_base(&expected.base));
        if rust_autoborrows_temp && matches!(resolved_kind, CoercionKind::Borrow) {
            resolved_kind = CoercionKind::Identity;
        }
        let coerced = apply_coercion(&resolved_kind, prepared_arg.as_str(), Target::Rust);
        let mut coerced = self.finalize_ir_call_arg(arg_expr, prepared_arg.as_str(), &coerced);

        // IR contract pass: when Identity was forced above but `expected` is still Ref,
        // re-apply borrow from SafetyType (not the legacy should_borrow decision tree).
        let user_explicit_deref =
            crate::codegen::rust::call_site_borrow::user_wrote_explicit_deref(arg_expr);
        if !user_explicit_deref {
            crate::ir::coercion::enforce_ownership_contract_on_coerced_arg_with_force_owned(
                &mut coerced,
                &actual,
                &expected,
                false,
                rust_autoborrows_temp,
                false,
            );
        }
        let arg_binding_already_rust_ref = matches!(
            arg_expr,
            Expression::Identifier { name, .. }
                if self.identifier_binding_already_rust_ref(name)
        );
        if arg_binding_already_rust_ref {
            // IR Ref / shared auto-borrow may already have prefixed `&` onto an
            // emitted `&mut T` / `&T` binding (`take_in_edges(&csr)` → `&&mut`).
            coerced =
                crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced).to_string();
        }
        // Stale multipass metadata may infer borrow for plain `string` formals on user
        // free functions that actually emit owned `String` (circular-dep convergence).
        // Never suppress mut-borrow when the slot expects `&mut T`.
        if receiver_type_name.is_none()
            && {
                let registry_lookup = self.signature_lookup_callee_name(callee_name);
                crate::codegen::rust::call_site_borrow::skip_stale_borrow_on_owned_user_free_fn_with_global(
                    &self.signature_registry,
                    self.global_signature_registry.as_deref(),
                    callee_name,
                    &sig,
                    param_idx,
                    arg_index,
                    registry_lookup.as_ref(),
                )
            }
            && coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
            && !self.ir_sig_arg_expects_mut_borrow(&sig, arg_index)
        {
            coerced =
                crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced).to_string();
        }
        // Shared-ref slots: stale owned-string metadata must not leave `binding.clone()` / `&field.clone()`.
        if coerced.ends_with(".clone()")
            && !coerced.starts_with("&mut ")
            && !crate::codegen::rust::expression_helpers::is_explicit_user_clone_call(arg_expr)
            && (crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&sig, param_idx)
                || sig
                    .emitted_rust_ref_params
                    .as_ref()
                    .and_then(|flags| flags.get(param_idx))
                    .copied()
                    == Some(true))
        {
            if coerced.starts_with('&') {
                coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
                    .to_string();
            }
            crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
            if !coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                    &mut coerced,
                );
            }
        } else if (coerced.ends_with(".clone()")
            || coerced.ends_with(".to_string()")
            || coerced.ends_with(".to_owned()"))
            && coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
            // `String.clone()` deref-coerces to `&str`. Custom `&T` must keep `&item.clone()`.
            && crate::ir::coercion::rust_owned_temp_deref_coerces_into_shared_ref(&expected)
        {
            coerced =
                crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced).to_string();
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.in_user_written_closure
                && self.user_closure_params.contains(name)
                && self.ir_sig_arg_expects_shared_borrow(&sig, arg_index)
                && !coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
            {
                coerced = format!("&{coerced}");
            }
        }
        if !(matches!(arg_expr, Expression::Identifier { name, .. }
            if self.in_user_written_closure && self.user_closure_params.contains(name))
            && !matches!(arg_expr, Expression::Closure { .. })
            && self.ir_sig_arg_expects_shared_borrow(&sig, arg_index))
            && {
                let registry_lookup = self.signature_lookup_callee_name(callee_name);
                crate::codegen::rust::call_site_borrow::skip_stale_borrow_on_owned_user_free_fn_with_global(
                    &self.signature_registry,
                    self.global_signature_registry.as_deref(),
                    callee_name,
                    &sig,
                    param_idx,
                    arg_index,
                    registry_lookup.as_ref(),
                )
            }
            && coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
        {
            coerced = coerced[1..].to_string();
        }
        if self.ir_callee_arg_expects_mut_borrow(
            registry,
            callee_name,
            arg_index,
            user_arg_count,
            Some(&sig),
        ) && !self.ir_sig_arg_expects_shared_borrow(&sig, arg_index)
            && crate::codegen::rust::expression_utilities::arg_supports_mut_borrow_coercion(
                arg_expr,
            )
            && !coerced.starts_with("&mut ")
        {
            crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                arg_expr,
                &mut coerced,
                &self.current_function_params,
                &self.inferred_mut_borrowed_params,
                true,
            );
        }
        if self.ir_sig_arg_expects_shared_borrow(&sig, arg_index) {
            if coerced.starts_with("&mut ") {
                coerced = format!(
                    "&{}",
                    crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
                );
            } else if !coerced.starts_with('&')
                && matches!(
                    arg_expr,
                    Expression::Identifier { .. } | Expression::FieldAccess { .. }
                )
            {
                coerced = crate::ir::target_encodings::rust_shared_borrow(&coerced);
            }
        }
        // Owned effective formals must not keep a stale `&` / `&mut` from earlier passes
        // (Copy aggregates and non-Copy deps like AppDeps emit `mut deps: AppDeps`).
        // Never peel when this slot expects mut-borrow (`fill_grid(grid: &mut VoxelGrid)`),
        // including when codegen already recorded the slot in `function_emitted_mut_arg_indices`
        // but a stale Owned refresh is what `sig` currently holds.
        let mut_arg_emitted = self
            .function_emitted_mut_arg_indices
            .get(callee_name)
            .or_else(|| self.function_emitted_mut_arg_indices.get(&sig.name))
            .is_some_and(|indices| indices.contains(&arg_index));
        let expects_mut_here = mut_arg_emitted
            || self.ir_sig_arg_expects_mut_borrow(&sig, arg_index)
            || self.ir_callee_arg_expects_mut_borrow(
                registry,
                callee_name,
                arg_index,
                user_arg_count,
                Some(&sig),
            );
        if !expects_mut_here {
            let peel_sig =
                self.refreshed_call_site_sig_for_arg(registry, callee_name, arg_index, &sig);
            let peel_pidx = peel_sig.arg_param_index(arg_index);
            if !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                &peel_sig, peel_pidx,
            ) && !self.ir_sig_arg_expects_shared_borrow(&peel_sig, arg_index)
                && (crate::ir::signature_bridge::call_site_expects_owned_pass(&peel_sig, peel_pidx)
                    || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &peel_sig, peel_pidx,
                    )
                    || crate::ir::emission_contract::plain_string_formal_passes_owned_at_call_site(
                        &peel_sig, peel_pidx,
                    ))
            {
                if coerced.starts_with("&mut ")
                    || (coerced.starts_with('&') && !coerced.starts_with("&mut "))
                {
                    coerced =
                        crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
                            .to_string();
                }
            }
        } else if !expects_mut_here
            && matches!(
                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                    &sig, arg_index,
                ),
                crate::analyzer::OwnershipMode::Owned,
            )
            && coerced.starts_with("&mut ")
        {
            coerced =
                crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced).to_string();
        }
        // Belt-and-suspenders: owned Copy aggregates pass by value at call sites.
        // Use registry-aware `is_type_copy` (Lsn, PartId, …) — pure analysis only knows
        // primitives and would miss user Copy aggregates (regression-060 `is_at_or_before`).
        // Do not strip `&mut` when the callee emits `&mut T` (Copy + MutBorrowed PlayerState).
        if coerced.starts_with("&mut ")
            || (coerced.starts_with('&') && !coerced.starts_with("&mut "))
        {
            let callee_bare = sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx))
                .map(|t| match t {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                });
            if let Some(bare) = callee_bare {
                if self.is_type_copy(bare)
                    && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                    && !self.ir_sig_arg_expects_shared_borrow(&sig, arg_index)
                    && !self.ir_sig_arg_expects_mut_borrow(&sig, arg_index)
                {
                    coerced =
                        crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
                            .to_string();
                }
            }
        }

        // Method-registry / global converged signatures must win over stale call-site
        // metadata (dogfood `engine.put` delegation, forward refs to later impl methods).
        self.apply_registry_borrow_to_call_arg(
            &mut coerced,
            arg_expr,
            receiver_type_name,
            method_simple,
            arg_index,
            user_arg_count,
        );

        // `apply_registry_borrow_to_call_arg` may re-apply stale `&` from global stubs;
        // Copy-aggregate caller→callee must stay by-value (regression-060).
        self.peel_copy_aggregate_caller_into_owned_callee(
            &mut coerced,
            arg_expr,
            callee_name,
            &sig,
            arg_index,
            receiver_type_name,
            false,
        );

        // Final owned-contract enforcement after registry re-borrow (dogfood), …)` while formal emits owned `mut deps: AppDeps`).
        // Never strip when this slot is a confirmed shared-ref formal (`&str` / `&T`).
        // Never strip `&mut` when the slot expects mut-borrow.
        let expects_mut = matches!(
            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                &sig, arg_index,
            ),
            crate::analyzer::OwnershipMode::MutBorrowed,
        ) || sig
            .param_types
            .get(param_idx)
            .is_some_and(|t| matches!(t, Type::MutableReference(_)));
        let peel_sig = self.refreshed_call_site_sig_for_arg(registry, callee_name, arg_index, &sig);
        let peel_pidx = peel_sig.arg_param_index(arg_index);
        if !expects_mut
            && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                &peel_sig, peel_pidx,
            )
            && !self.ir_sig_arg_expects_shared_borrow(&peel_sig, arg_index)
            && (crate::ir::signature_bridge::call_site_expects_owned_pass(&peel_sig, peel_pidx)
                || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &peel_sig, peel_pidx,
                )
                || crate::ir::emission_contract::plain_string_formal_passes_owned_at_call_site(
                    &peel_sig, peel_pidx,
                ))
        {
            if coerced.starts_with("&mut ")
                || (coerced.starts_with('&') && !coerced.starts_with("&mut "))
            {
                coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
                    .to_string();
            }
        }

        // Stale registry MutBorrowed must not become `&mut` when the slot is shared-ref.
        if coerced.starts_with("&mut ")
            && (self.ir_sig_arg_expects_shared_borrow(&sig, arg_index)
                || crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(&sig, param_idx)
                || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                    &sig, param_idx,
                ))
        {
            coerced = format!(
                "&{}",
                crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
            );
        }
        // Already-emitted `&T` formals must not grow a second `&` (LsmEngine::get → engine.get).
        // Also cover analyzer-inferred / Phase-2 `&str` formals (`identifier_already_ref`).
        if let Expression::Identifier { name, .. } = arg_expr {
            if (self.emitted_rust_ref_formals.contains(name)
                || self.binding_emits_as_rust_shared_ref(name)
                || self.identifier_already_ref(name)
                || self.str_ref_optimized_params.contains(name.as_str()))
                && !self.collection_key_owned_params.contains(name.as_str())
                && coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
                && coerced != name.as_str()
            {
                let base = crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced);
                if base == name.as_str() || base.starts_with(name.as_str()) {
                    coerced = base.to_string();
                }
            }
        }

        let mut coerced = self.finish_runtime_std_call_arg(
            callee_name,
            arg_index,
            arg_expr,
            coerced,
            Some(&sig),
            receiver_type_name,
        );
        if thin_wrap_into_string_formal {
            while coerced.starts_with("&mut ") {
                coerced = coerced["&mut ".len()..].trim().to_string();
            }
            while coerced.starts_with('&') {
                coerced = coerced[1..].trim().to_string();
            }
            crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
            if !coerced.ends_with(".into()")
                && !coerced.ends_with(".to_string()")
                && !coerced.ends_with(".to_owned()")
            {
                coerced = format!("{coerced}.into()");
            }
        }
        if coerced.ends_with(".clone()") {
            let this_arg_expects_borrow = self.ir_sig_arg_expects_shared_borrow(&sig, arg_index);
            let this_arg_expects_mut = self.ir_sig_arg_expects_mut_borrow(&sig, arg_index);
            let preserve_explicit =
                crate::codegen::rust::expression_helpers::is_explicit_user_clone_call(arg_expr);
            // Mut formals: always strip — `&mut x.clone()` is never a valid lvalue (WDB-336/337/342).
            // Shared formals: preserve explicit user `.clone()` (WDB-106/108).
            if this_arg_expects_mut || !preserve_explicit {
                match arg_expr {
                    Expression::Identifier { name, .. } => {
                        let arg_is_fn_param =
                            self.current_function_params.iter().any(|p| p.name == *name);
                        if self.borrowed_iterator_vars.contains(name)
                        && self.current_function_return_type.as_ref().is_some_and(|rt| {
                            matches!(rt, Type::Vec(inner) if matches!(**inner, Type::Reference(_) | Type::MutableReference(_)))
                        })
                    {
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(
                            &mut coerced,
                        );
                    } else if this_arg_expects_mut
                        || (arg_is_fn_param && this_arg_expects_borrow)
                    {
                        // Locals reused into demoted `&mut Vec` must keep the binding
                        // (`&mut data`), not a clone temp (WDB-336/337/342).
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(
                            &mut coerced,
                        );
                    }
                    }
                    Expression::FieldAccess { object, .. } => {
                        // `&mut place.field` / `&place.field` — never `&mut place.field.clone()`.
                        if this_arg_expects_mut
                            || (this_arg_expects_borrow
                                && matches!(
                                    &**object,
                                    Expression::Identifier { name, .. } if name == "self"
                                ))
                        {
                            crate::codegen::rust::expression_utilities::strip_trailing_clone(
                                &mut coerced,
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
        let pidx = sig.arg_param_index(arg_index);
        let simple_callee = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let callee_sig = callee_name
            .rsplit_once("::")
            .and_then(|(rt, method)| {
                self.resolve_method_function_signature(
                    rt,
                    method,
                    user_arg_count.unwrap_or(arg_index + 1),
                )
            })
            .or_else(|| {
                self.signature_registry.get_signature(callee_name).cloned()
                // Never fall back to the bare method name for `module::fn` /
                // `Type::method` — that matches unrelated homonyms
                // (e.g. rendering_api::draw_text for draw::draw_text).
            })
            .unwrap_or(sig.clone());
        let callee_pidx = callee_sig.arg_param_index(arg_index);
        let runtime_std_needs_borrow =
            crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                registry,
                callee_name,
                Some(&callee_sig),
                arg_index,
            ) || {
                let inferred = self.infer_expression_type(arg_expr);
                crate::codegen::rust::stdlib_method_traits::runtime_std_call_arg_needs_auto_borrow(
                    callee_module,
                    simple_callee,
                    Some(&callee_sig),
                    arg_index,
                    inferred.as_ref(),
                    arg_expr,
                    receiver_type_name,
                )
            };
        if !matches!(
            resolved_kind,
            CoercionKind::Identity
                | CoercionKind::Borrow
                | CoercionKind::MutBorrow
                | CoercionKind::ToOwnedString
                | CoercionKind::Clone
        )
            && crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(&callee_sig, callee_pidx)
            && !runtime_std_needs_borrow
            && !(crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
                && crate::codegen::rust::stdlib_method_traits::runtime_or_str_ref_formal_skips_literal_owned(
                    Some(&callee_sig),
                    arg_index,
                ))
        {
            let callee_borrows_text = self.ir_sig_arg_expects_shared_borrow(&callee_sig, arg_index)
                || callee_sig.param_types.get(callee_pidx).is_some_and(|t| {
                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                });
            if !callee_borrows_text {
            let skip_clone_for_user_closure_param = matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if self.in_user_written_closure && self.user_closure_params.contains(name)
            );
            if coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                coerced = coerced[1..].to_string();
            }
            if skip_clone_for_user_closure_param {
                if self.ir_sig_arg_expects_shared_borrow(&callee_sig, arg_index)
                    && !coerced.starts_with('&')
                    && !coerced.starts_with("&mut ")
                {
                    coerced = format!("&{coerced}");
                }
                // User closure params pass through unchanged (e.g. |e| predicate(e)).
            } else if matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if self.identifier_already_ref(name)
                        || self.emitted_rust_ref_formals.contains(name)
            ) && !coerced.ends_with(".clone()")
            {
                let formal_is_copy = callee_sig
                    .formal_param_type(callee_pidx)
                    .is_some_and(|t| self.is_type_copy(t));
                let callee_borrows_text = self.ir_sig_arg_expects_shared_borrow(&callee_sig, arg_index)
                    || callee_sig.param_types.get(callee_pidx).is_some_and(|t| {
                        crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                    });
                let callee_accepts_ref_reborrow = callee_borrows_text
                    || callee_sig.param_types.get(callee_pidx).is_some_and(|t| {
                        matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    })
                    || self.ir_sig_arg_expects_shared_borrow(&callee_sig, arg_index)
                    || self.ir_sig_arg_expects_mut_borrow(&callee_sig, arg_index);
                if !formal_is_copy && !callee_accepts_ref_reborrow {
                    if matches!(
                        arg_expr,
                        Expression::Identifier { name, .. }
                            if self.for_loop_borrow_needed.contains(name)
                    ) {
                        let base = crate::codegen::rust::expression_utilities::borrow_base_expr(
                            &coerced,
                        );
                        coerced = if base.starts_with('&') {
                            base.to_string()
                        } else {
                            format!("&{base}")
                        };
                    } else {
                        coerced = format!("{}.clone()", coerced.trim_start_matches('&'));
                    }
                }
            } else if !coerced.ends_with(".clone()") {
                let skip_user_closure_param = matches!(
                    arg_expr,
                    Expression::Identifier { name, .. }
                        if self.in_user_written_closure && self.user_closure_params.contains(name)
                );
                if skip_user_closure_param {
                    // Preserve user-written closure bodies (e.g. |e| predicate(e)).
                } else {
                let skip_iter_ref_collect = matches!(arg_expr, Expression::Identifier { name, .. }
                    if self.borrowed_iterator_vars.contains(name)
                        && self.current_function_return_type.as_ref().is_some_and(|rt| {
                            matches!(rt, Type::Vec(inner) if matches!(**inner, Type::Reference(_) | Type::MutableReference(_)))
                        }));
                if !skip_iter_ref_collect {
                let callee_borrows_text = self.ir_sig_arg_expects_shared_borrow(&callee_sig, arg_index)
                    || callee_sig.param_types.get(callee_pidx).is_some_and(|t| {
                        crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                    });
                if !callee_borrows_text {
                    let actual = self.infer_actual_safety_type(arg_expr, coerced.as_str());
                    let expected = safety_type_from_signature_param(&callee_sig, callee_pidx);
                    if matches!(
                        compute_coercion(&actual, &expected),
                        CoercionKind::Clone
                    ) && !coerced.ends_with(".to_string()")
                        && !coerced.ends_with(".to_owned()")
                        && !match arg_expr {
                            // Only scalar Copy (i64/bool/…) skips clone; Copy aggregates/enums
                            // still need `.clone()` on multi-use owned moves (regression-063 Value).
                            Expression::Identifier { name, .. } => {
                                self.binding_is_copy_pass_by_value_scalar(name)
                            }
                            Expression::Index { .. } => {
                                self.index_expression_is_copy_scalar(arg_expr)
                            }
                            _ => self.infer_expression_type(arg_expr).is_some_and(|t| {
                                let bare = match &t {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        inner.as_ref()
                                    }
                                    other => other,
                                };
                                crate::type_classification::is_copy_pass_by_value_formal(bare)
                            }),
                        }
                    {
                        if matches!(
                            arg_expr,
                            Expression::Identifier { name, .. }
                                if self.for_loop_borrow_needed.contains(name)
                        ) {
                            let base = crate::codegen::rust::expression_utilities::borrow_base_expr(
                                &coerced,
                            );
                            if !base.starts_with('&') {
                                coerced = format!("&{base}");
                            } else {
                                coerced = base.to_string();
                            }
                        } else {
                            coerced = format!("{}.clone()", coerced.trim_start_matches('&'));
                        }
                    }
                }
                }
                }
            }
            }
        }
        let _ = pidx;
        // Strip redundant `.clone()` only for scalar Copy formals (i64/bool/…).
        // Copy aggregates/enums (Value, Lsn) still need multi-use clones (regression-063).
        if coerced.ends_with(".clone()") {
            let formal_ty = sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx));
            let formal_is_scalar_copy = formal_ty.is_some_and(|t| {
                let bare = match t {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                crate::type_classification::is_copy_pass_by_value_formal(bare)
            });
            let binding_is_scalar_copy = match arg_expr {
                Expression::Identifier { name, .. } => {
                    self.binding_is_copy_pass_by_value_scalar(name)
                }
                Expression::Index { .. } => self.index_expression_is_copy_scalar(arg_expr),
                _ => self.infer_expression_type(arg_expr).is_some_and(|t| {
                    let bare = match &t {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    crate::type_classification::is_copy_pass_by_value_formal(bare)
                }),
            };
            if formal_is_scalar_copy || binding_is_scalar_copy {
                crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
            }
        }
        if coerced.ends_with(".clone()") {
            let callee_wants_mut = self.ir_sig_arg_expects_mut_borrow(&callee_sig, arg_index)
                || self.ir_callee_arg_expects_mut_borrow(
                    registry,
                    callee_name,
                    arg_index,
                    user_arg_count,
                    Some(&callee_sig),
                );
            if callee_wants_mut {
                if let Expression::Identifier { name, .. } = arg_expr {
                    let arg_is_fn_param =
                        self.current_function_params.iter().any(|p| p.name == *name);
                    if arg_is_fn_param
                        && (self.emitted_rust_ref_formals.contains(name)
                            || self.identifier_binding_already_rust_ref(name))
                        && !crate::codegen::rust::expression_helpers::is_explicit_user_clone_call(
                            arg_expr,
                        )
                    {
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(
                            &mut coerced,
                        );
                    }
                }
            }
        }
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && !crate::codegen::rust::stdlib_method_traits::runtime_or_str_ref_formal_skips_literal_owned(
            Some(&sig),
            arg_index,
        )
            && crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                &sig, param_idx,
            )
            && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                &sig, param_idx,
            )
            && !crate::ir::signature_bridge::call_site_expects_owned_pass(&sig, param_idx)
            && !crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
                &sig, arg_index,
            )
            && sig
            .param_type_for_arg(arg_index)
            .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_string_ref)
        {
            let base = coerced.trim_start_matches('&');
            let owned = if crate::codegen::rust::string_utilities::already_owned_string_expr(base)
            {
                base.to_string()
            } else {
                crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(base)
            };
            return Some(format!("&{owned}"));
        }
        // Plain WJ `string` formals emit owned `String` even when multipass left stale
        // `Borrowed` ownership (cross-file analysis before defining-module codegen).
        // Signature-driven only — no method-name heuristics.
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && !self.ir_sig_arg_expects_shared_borrow(&sig, arg_index)
            && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                &sig,
                sig.arg_param_index(arg_index),
            )
            && (crate::codegen::rust::string_utilities::string_literal_needs_to_string(
                &sig, arg_index,
            ) || crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
                &sig, arg_index,
            ))
            && !crate::codegen::rust::string_utilities::already_owned_string_expr(&coerced)
        {
            return Some(
                crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                    coerced.trim_start_matches('&'),
                ),
            );
        }
        // P3.376: `pub const SCOPE_*: string` / module string consts lower to `&'static str`,
        // but IR still types them as owned WJ `string` → compute_coercion is Identity.
        // Owned formals (named `string` *and* `Vec<string>::push(T)`) need `.to_string()`.
        // IR call-sites skip method-call finalize, so this must live here.
        let arg_is_string_const = match arg_expr {
            Expression::Identifier { name, .. } => {
                crate::codegen::rust::string_utilities::is_string_const_identifier(
                    name,
                    self.auto_clone_analysis.as_ref(),
                    Some(&self.module_string_consts),
                )
            }
            Expression::FieldAccess { field, .. } => {
                crate::codegen::rust::string_utilities::is_string_const_identifier(
                    field,
                    self.auto_clone_analysis.as_ref(),
                    Some(&self.module_string_consts),
                )
            }
            _ => false,
        };
        if arg_is_string_const
            && !self.ir_sig_arg_expects_shared_borrow(&sig, arg_index)
            && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                &sig,
                sig.arg_param_index(arg_index),
            )
            && !crate::codegen::rust::string_utilities::already_owned_string_expr(&coerced)
        {
            let pidx = sig.arg_param_index(arg_index);
            // Vec::push / generic Owned T: formal may not look like WJ `string` until
            // specialized — still own when the call site expects an owned pass.
            let wants_owned = crate::codegen::rust::string_utilities::string_literal_needs_to_string(
                &sig, arg_index,
            )
                || crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
                    &sig, arg_index,
                )
                || crate::ir::signature_bridge::call_site_expects_owned_pass(&sig, pidx)
                || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(&sig, pidx)
                || matches!(
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                        &sig, pidx
                    ),
                    crate::analyzer::OwnershipMode::Owned
                );
            if wants_owned {
                return Some(
                    crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                        coerced.trim_start_matches('&'),
                    ),
                );
            }
        }
        let callee_has_ownership_collision =
            crate::codegen::rust::call_signature_resolution::has_ownership_collision_for_call(
                self,
                callee_name,
            );
        let collision_blocks_autoborrow = callee_has_ownership_collision
            && crate::codegen::rust::call_signature_resolution::ownership_collision_blocks_autoborrow(
                callee_name,
            );
        if !collision_blocks_autoborrow {
            crate::codegen::rust::string_utilities::finalize_borrowed_text_call_site_arg(
                Some(&sig),
                arg_index,
                receiver_type_name,
                arg_expr,
                &mut coerced,
                arg_binding_already_rust_ref,
            );
        }
        // Forward-ref: owned caller binding → callee `&T` when registry encodes borrow.
        if let Expression::Identifier { name, .. } = arg_expr {
            let fresh_sig = callee_name.rsplit_once("::").and_then(|(rt, method)| {
                let rt = if rt == "Self" {
                    self.current_struct_name
                        .clone()
                        .unwrap_or_else(|| rt.to_string())
                } else {
                    rt.to_string()
                };
                self.resolve_method_function_signature(
                    rt.as_str(),
                    method,
                    user_arg_count.unwrap_or(arg_index + 1),
                )
            });
            let borrow_sig = fresh_sig.as_ref().unwrap_or(&sig);
            let borrow_idx = borrow_sig.arg_param_index(arg_index);
            let body_refs: Vec<&Statement<'ast>> =
                self.current_function_body.iter().copied().collect();
            let binding_is_text_param = self.current_function_params.iter().any(|p| {
                p.name == *name && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
            });
            let binding_is_vec_local = self.local_var_types.get(name).is_some_and(|t| {
                matches!(t, Type::Vec(_))
                    || matches!(t, Type::Parameterized(name, _) if name == "Vec")
            });
            let binding_used_as_read_local =
                self.param_used_as_read_operand(body_refs.as_slice(), name);
            let callee_wants_shared =
                crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                    borrow_sig, borrow_idx,
                );
            let callee_emits_owned =
                self.ir_callee_arg_emits_owned_contract(
                    &self.signature_registry,
                    callee_name,
                    arg_index,
                    user_arg_count,
                    Some(&sig),
                ) || self.global_signature_registry.as_ref().is_some_and(|g| {
                    self.ir_callee_arg_emits_owned_contract(
                        g,
                        callee_name,
                        arg_index,
                        user_arg_count,
                        Some(&sig),
                    )
                });
            let local_shadows_owned_formal = self.local_owned_binding_shadows_formal(name);
            let callee_owned_text_formal =
                crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    borrow_sig, borrow_idx,
                ) || (borrow_sig
                    .formal_param_type(borrow_idx)
                    .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
                    && borrow_sig
                        .emitted_rust_ref_params
                        .as_ref()
                        .and_then(|flags| flags.get(borrow_idx).copied())
                        == Some(false));
            let local_reuse_after = self.local_binding_reused_after_current_statement(name);
            if !collision_blocks_autoborrow
                && !local_shadows_owned_formal
                && !coerced.starts_with('&')
                && !coerced.ends_with(".clone()")
                && !self.emitted_rust_ref_formals.contains(name)
                && !self.inferred_borrowed_params.contains(name)
                && (binding_is_text_param || binding_used_as_read_local || binding_is_vec_local)
                && callee_wants_shared
                && !callee_emits_owned
                && !callee_owned_text_formal
                && !(local_reuse_after
                    && (callee_owned_text_formal
                        || self.preregistered_free_call_arg_emits_owned(callee_name, arg_index)
                        || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            borrow_sig, borrow_idx,
                        )))
            {
                coerced = format!("&{coerced}");
            }
            if local_reuse_after
                && binding_is_text_param
                && !coerced.starts_with('&')
                && !callee_emits_owned
                && (self.ir_sig_arg_expects_shared_borrow(borrow_sig, arg_index)
                    || borrow_sig
                        .formal_param_type(borrow_idx)
                        .or_else(|| borrow_sig.param_types.get(borrow_idx))
                        .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_str_ref))
            {
                coerced = format!("&{coerced}");
            }
        }
        // Caller-owned text binding → user free-fn callee without confirmed `&str` emission:
        // strip stale multipass `&` (`foo(String)` calling `bar(&x)`). Skip method calls
        // (String::contains etc.) where the library API genuinely expects `&str`.
        if receiver_type_name.is_none()
            && !callee_name.contains("::")
            && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&sig, param_idx)
            && !sig
                .emitted_rust_ref_params
                .as_ref()
                .is_some_and(|flags| flags.get(param_idx).copied().unwrap_or(false))
            && matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if (self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                    }) || self.local_var_types.get(name).is_some_and(|t| {
                        crate::codegen::rust::types::is_windjammer_text_type(t)
                    })) && !self.emitted_rust_ref_formals.contains(name)
            )
            && coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
        {
            coerced = coerced[1..].to_string();
        }
        if collision_blocks_autoborrow {
            // Prefer codegen-refreshed registry entry — analyzer call-site stubs often
            // lack `emitted_rust_ref_params` while the defining-fn refresh has them
            // (`check(item: &Item)` after `fn check` body emission).
            let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
            if let Some(refreshed) =
                crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature([
                    self.signature_registry.get_signature(callee_name).cloned(),
                    self.signature_registry.get_signature(simple).cloned(),
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(callee_name).cloned()),
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(simple).cloned()),
                    Some(sig.clone()),
                ])
            {
                let ridx = refreshed.arg_param_index(arg_index);
                if refreshed.emitted_rust_ref_params.is_some()
                    || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                        &refreshed, ridx,
                    )
                {
                    sig = refreshed;
                    param_idx = ridx;
                }
            }
            let emits_shared =
                crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&sig, param_idx);
            if !emits_shared {
                // Homonym collisions (`check`, `process`, …) strip unsafe auto-borrow
                // when the callee contract is ambiguous across modules.
                while coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                    coerced = coerced[1..].to_string();
                }
                if !matches!(arg_expr, Expression::Index { .. }) {
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
                }
            } else {
                let is_text_shared = crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                    &sig, param_idx,
                ) || sig.param_types.get(param_idx).is_some_and(|t| {
                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                        || matches!(
                            t,
                            Type::Reference(inner)
                                if crate::codegen::rust::types::is_windjammer_text_type(inner)
                        )
                });
                if is_text_shared {
                    // Confirmed `&str`/`&String` under collision: keep `&`, drop owned
                    // literal coercion (string_literal_no_conversion / regression-048).
                    if let Some(stripped) = coerced.strip_suffix(".to_string()") {
                        coerced = stripped.to_string();
                    }
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
                }
                // Confirmed shared-ref Custom (`item: &Item`): ensure `&` survives
                // collision even when earlier IR/should_borrow skipped the prefix.
                if !crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
                    && matches!(
                        arg_expr,
                        Expression::Identifier { .. } | Expression::FieldAccess { .. }
                    )
                {
                    let arg_already_rust_ref = matches!(
                        arg_expr,
                        Expression::Identifier { name, .. }
                            if self.identifier_binding_already_rust_ref(name)
                                || self.str_ref_optimized_params.contains(name.as_str())
                                || self.inferred_borrowed_params.contains(name)
                    );
                    if !arg_already_rust_ref {
                        // Drop owned-move artifacts introduced under stale collision
                        // stripping (`item.clone()` / `"lit".to_string()`).
                        if coerced.ends_with(".clone()") {
                            crate::codegen::rust::expression_utilities::strip_trailing_clone(
                                &mut coerced,
                            );
                        }
                        if coerced.ends_with(".to_string()") && is_text_shared {
                            if let Some(stripped) = coerced.strip_suffix(".to_string()") {
                                coerced = stripped.to_string();
                            }
                        }
                        if !coerced.starts_with('&') {
                            coerced = format!("&{coerced}");
                        }
                    }
                }
            }
        }
        // Final IR ownership contract: strip spurious `&` when callee emits owned formals
        // (regression-056 keys_equal(Vec<u8>, Vec<u8>)), or add borrow when expected.
        self.enforce_call_site_ownership_contract(
            &mut coerced,
            arg_expr,
            &sig,
            param_idx,
            callee_name,
            arg_index,
        );
        self.peel_copy_aggregate_caller_into_owned_callee(
            &mut coerced,
            arg_expr,
            callee_name,
            &sig,
            arg_index,
            receiver_type_name,
            false,
        );
        if let Expression::Identifier { name, .. } = arg_expr {
            let binding_is_owned_string =
                self.local_var_types.get(name).is_some_and(|t| {
                    crate::codegen::rust::string_utilities::type_is_owned_string(t)
                }) || self.current_function_params.iter().any(|p| {
                    p.name == *name
                        && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                        && !self.emitted_rust_ref_formals.contains(name)
                });
            if !self.match_arm_bindings.contains(name.as_str())
                && coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
                && !self.preregistered_free_call_arg_expects_borrow(callee_name, arg_index)
                && (self.caller_owned_non_copy_formal(name)
                    || self.local_binding_is_owned_non_copy(name)
                    || binding_is_owned_string)
                && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &sig, param_idx,
                ) || crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
                    &sig, arg_index,
                ) || (binding_is_owned_string
                    && sig.formal_param_type(param_idx).is_some_and(|t| {
                        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                            && crate::codegen::rust::types::is_windjammer_text_type(t)
                    })
                    && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                        &sig, param_idx,
                    )))
                && !crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                    &self.signature_registry,
                    callee_name,
                    Some(&sig),
                    arg_index,
                )
            {
                coerced = coerced.trim_start_matches('&').to_string();
            }
        }
        // After borrow stripping / collision clone-stripping: restore `.clone()` when
        // auto-clone analysis says this binding/path is moved and reused (regression-059).
        coerced = self.ensure_owned_move_clone_for_reuse(
            arg_expr,
            &coerced,
            &sig,
            param_idx,
            callee_name,
            arg_index,
        );
        crate::codegen::rust::expression_utilities::collapse_redundant_clones(&mut coerced);
        let callee_accepts_str_ref =
            crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&sig, param_idx)
                || crate::ir::signature_bridge::call_site_expects_shared_borrow(&sig, param_idx)
                || sig
                    .formal_param_type(param_idx)
                    .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_str_ref)
                || (sig.has_self_receiver
                    && crate::ir::formal_predicates::formal_is_plain_windjammer_string(
                        &sig, param_idx,
                    )
                    && matches!(
                        sig.param_ownership.get(param_idx),
                        Some(crate::analyzer::OwnershipMode::Borrowed)
                    ));
        if callee_accepts_str_ref {
            crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                arg_expr,
                &mut coerced,
            );
            if receiver_type_name.is_none()
                && matches!(arg_expr, Expression::Identifier { .. })
                && !coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
                && self.ir_callee_arg_expects_shared_borrow(
                    registry,
                    callee_name,
                    arg_index,
                    user_arg_count,
                    Some(&sig),
                )
            {
                crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                    &mut coerced,
                );
            }
        } else if coerced.ends_with(".to_string().clone()")
            || coerced.ends_with(".to_owned().clone()")
        {
            crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
        }
        if coerced.ends_with(".clone()") {
            if let Expression::Identifier { name, .. } = arg_expr {
                let collects_ref_vec =
                    crate::codegen::rust::types::return_type_is_vec_of_shared_refs(
                        self.current_function_return_type.as_ref(),
                    );
                if collects_ref_vec
                    && (self.borrowed_iterator_vars.contains(name)
                        || self.local_var_types.get(name).is_some_and(|t| {
                            matches!(t, Type::Reference(_) | Type::MutableReference(_))
                        }))
                {
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
                }
            }
        }
        // Match-arm owned String payloads: shared-ref text callees want `&binding`.
        // Owned WJ `string` formals must keep the move (no `&`).
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.match_arm_bindings.contains(name.as_str()) {
                let expects_shared_text =
                    crate::ir::signature_bridge::call_site_wants_shared_text_ref(&sig, param_idx);
                let expects_owned_text =
                    crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &sig, param_idx,
                    ) && sig.formal_param_type(param_idx).is_some_and(|t| {
                        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                            && crate::codegen::rust::types::is_windjammer_text_type(t)
                    });
                if expects_owned_text {
                    if coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                        coerced = coerced.trim_start_matches('&').to_string();
                    }
                } else if expects_shared_text {
                    if coerced.ends_with(".clone()") {
                        coerced = coerced[..coerced.len() - ".clone()".len()].to_string();
                    }
                    if !coerced.starts_with('&') {
                        coerced = format!("&{coerced}");
                    }
                }
            }
        }
        coerced =
            self.normalize_owned_copy_match_binding_call_arg(arg_expr, &coerced, &sig, arg_index);
        coerced = self.normalize_borrowed_iter_elem_for_owned_copy_scalar(
            arg_expr, &coerced, &sig, arg_index,
        );
        if coerced.ends_with(".to_string()")
            && crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&sig, param_idx)
        {
            if let Some(stripped) = coerced.strip_suffix(".to_string()") {
                coerced = stripped.to_string();
            }
        }
        // `&str` / `&String` caller bindings → owned `String` formals need `.to_string()`
        // (types-crate `batch_column_i64(name: &str)` → `ArrowBatch::column_i64(name: String)`).
        if let Expression::Identifier { name, .. } = arg_expr {
            let caller_is_str_slice = self.str_ref_optimized_params.contains(name.as_str())
                || (self.emitted_rust_ref_formals.contains(name)
                    && self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                    }));
            let callee_wants_owned_string =
                crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &sig, param_idx,
                ) && sig.formal_param_type(param_idx).is_some_and(|t| {
                    !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                        && crate::codegen::rust::types::is_windjammer_text_type(t)
                });
            if caller_is_str_slice
                && callee_wants_owned_string
                && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                    &sig, param_idx,
                )
                && !crate::ir::signature_bridge::call_site_expects_shared_borrow(&sig, param_idx)
                && !crate::codegen::rust::string_utilities::already_owned_string_expr(&coerced)
            {
                coerced = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                    crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced),
                );
            }
        }
        // Recursive same-fn call into an owned formal emitted by *this* function:
        // strip stale `&` from analyzer/forward-ref borrow (ReBAC `policy: Policy`).
        // Must run last in apply_ir — reconcile also re-peels terminally.
        self.strip_recursive_owned_formal_stale_borrow(&mut coerced, arg_expr, callee_name);
        let callee_wants_shared_ref_formal = crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_from_sig(
            &sig, arg_index,
        ) || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&sig, param_idx)
            || (sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx))
                .is_some_and(|t| matches!(t, Type::Reference(_)))
                && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &sig, param_idx,
                ))
            || receiver_type_name.is_some_and(|rt| {
                crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_qualified(
                    method_simple,
                    Some(rt),
                    registry,
                    arg_index,
                )
            });
        let expects_str_ref = !self.preregistered_free_call_arg_emits_owned(callee_name, arg_index)
            && callee_wants_shared_ref_formal;
        if expects_str_ref {
            crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                arg_expr,
                &mut coerced,
            );
        }
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && coerced.starts_with('&')
            && coerced.ends_with(".to_string()")
            && (crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
                &sig, arg_index,
            ) || crate::ir::signature_bridge::call_site_expects_owned_pass(&sig, param_idx))
        {
            coerced = coerced.trim_start_matches('&').to_string();
        }

        // Single IR finalize pass: collection keys + string literals (signature-driven).
        // Callers must not re-implement collection-key prefix/strip after apply_ir returns.
        self.finalize_ir_collection_key_arg(
            &mut coerced,
            arg_expr,
            callee_name,
            &sig,
            arg_index,
            receiver_type_name,
        );
        crate::codegen::rust::string_utilities::finalize_string_literal_call_site_arg(
            Some(&sig),
            arg_index,
            Some(method_simple),
            arg_expr,
            &mut coerced,
            receiver_type_name,
            Some(&self.enum_variant_types),
        );
        if self.is_collection_key_lookup_at_site(&sig, arg_index, receiver_type_name)
            && matches!(
                arg_expr,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            )
        {
            if coerced.ends_with(".to_string()") {
                if let Some(stripped) = coerced.strip_suffix(".to_string()") {
                    coerced = stripped.to_string();
                }
            }
            crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                arg_expr,
                &mut coerced,
            );
        }

        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && crate::codegen::rust::string_utilities::already_owned_string_expr(&coerced)
            && !self.is_collection_key_lookup_at_site(&sig, arg_index, receiver_type_name)
        {
            coerced = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(&coerced);
        }

        if self.in_if_condition {
            if let Expression::Identifier { name, .. } = arg_expr {
                let body: Vec<_> = self.current_function_body.iter().copied().collect();
                let if_facade_forward_ref = self.current_fn_forward_ref_if_params.contains(name)
                    && self.param_used_in_if_with_condition_and_branches(&body, name);
                if !if_facade_forward_ref
                    && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &sig, param_idx,
                    ) || Self::sig_arg_confirms_owned_emission(&sig, arg_index))
                    && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                        &sig, param_idx,
                    )
                    && self
                        .current_function_params
                        .iter()
                        .any(|p| p.name == *name && !self.is_type_copy(&p.type_))
                    && !coerced.ends_with(".clone()")
                {
                    coerced = format!("{coerced}.clone()");
                }
            }
        }

        if crate::codegen::rust::expression_helpers::is_explicit_user_clone_call(arg_expr) {
            let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
            let refresh_keys = [callee_name.to_string(), simple.to_string()];
            // Importer stubs record `emitted_rust_ref_params = [false, …]`; defining-module
            // bare-pass demotion lives on the global registry — merge local first, global last.
            crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                &mut sig,
                registry,
                &refresh_keys,
            );
            if let Some(global) = self.global_signature_registry.as_ref() {
                crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                    &mut sig,
                    global,
                    &refresh_keys,
                );
            }
            if let Some(refreshed) =
                self.refresh_call_site_signature_for_arg(Some(sig.clone()), callee_name, arg_index)
            {
                sig = refreshed;
            }
            param_idx = sig.arg_param_index(arg_index);
        }

        coerced = crate::codegen::rust::string_utilities::finalize_explicit_user_clone_call_site(
            arg_expr,
            arg_str,
            &coerced,
            Some(&sig),
            arg_index,
            &self.emitted_rust_ref_formals,
            &self.current_function_params,
        );
        coerced = crate::codegen::rust::call_site_borrow::reconcile_explicit_user_clone_into_owned_vec_formal(
            self,
            arg_expr,
            coerced,
            &sig,
            arg_index,
        );

        // Borrowed for-loop elems into demoted `&str` callees: pass `item` (Rust
        // autoref), not `&item` (double-borrow / E0308 on owned loop variables).
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.borrowed_iterator_vars.contains(name)
                && self.ir_sig_arg_expects_shared_borrow(&sig, arg_index)
                && coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
            {
                let base = crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced);
                if base == name.as_str() {
                    coerced = base.to_string();
                }
            }
        }

        // Terminal: codegen-owned formals beat stale borrow metadata on locals
        // (`encode_value(v)`, `display_text(color)`, `put(key.clone())` — not `&v` / `&key.clone()`).
        let forward_ref_keeps_borrow = matches!(
            arg_expr,
            Expression::Identifier { name, .. }
                if self.in_if_condition
                    && (self.current_fn_forward_ref_if_params.contains(name)
                        || self.current_fn_mixed_forwarder_params.contains(name))
        );
        if coerced.starts_with('&') && !coerced.starts_with("&mut ") && !forward_ref_keeps_borrow {
            let callee_emits_owned = self
                .preregistered_free_call_arg_emits_owned(callee_name, arg_index)
                || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &sig, param_idx,
                )
                || self.ir_callee_arg_emits_owned_contract(
                    &self.signature_registry,
                    callee_name,
                    arg_index,
                    user_arg_count,
                    Some(&sig),
                )
                || self.global_signature_registry.as_ref().is_some_and(|g| {
                    self.ir_callee_arg_emits_owned_contract(
                        g,
                        callee_name,
                        arg_index,
                        user_arg_count,
                        Some(&sig),
                    )
                })
                || sig.formal_param_type(param_idx).is_some_and(|t| {
                    !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                        && !crate::codegen::rust::types::is_windjammer_text_type(t)
                        && !crate::codegen::rust::stdlib_method_traits::is_map_type(t)
                        && !crate::codegen::rust::stdlib_method_traits::is_set_type(t)
                });
            if callee_emits_owned
                && !self.preregistered_free_call_arg_expects_borrow(callee_name, arg_index)
                && !crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                    registry,
                    callee_name,
                    Some(&sig),
                    arg_index,
                )
            {
                coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
                    .to_string();
            }
        }

        self.maybe_borrow_vec_or_helper_from_global_metadata(
            &mut coerced,
            arg_expr,
            callee_name,
            arg_index,
            receiver_type_name,
            registry,
        );

        if let Expression::Identifier { name, .. } = arg_expr {
            if self.str_ref_optimized_params.contains(name)
                && (crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                    &sig, param_idx,
                ) || crate::ir::signature_bridge::call_site_expects_shared_borrow(
                    &sig, param_idx,
                ))
            {
                crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
                if coerced.ends_with(".to_string()") {
                    coerced = name.clone();
                }
            }
            if self.local_owned_binding_shadows_formal(name)
                && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &sig, param_idx,
                )
                    || crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
                        &sig, arg_index,
                    ))
                && coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
            {
                coerced = coerced.trim_start_matches('&').to_string();
            }
        }
        if crate::codegen::rust::call_site_borrow::user_wrote_explicit_deref(arg_expr)
            && coerced.starts_with("&*")
        {
            coerced = coerced[1..].to_string();
        } else if !crate::codegen::rust::call_site_borrow::user_wrote_explicit_deref(arg_expr)
            && coerced.starts_with("&*")
        {
            // WDB-368: compiler must not emit `&*ident` for owned `string` into demoted
            // `&str` formals (match payloads / locals). Bare ident coerces via Deref.
            coerced = coerced.trim_start_matches("&*").to_string();
        } else if crate::codegen::rust::call_site_borrow::user_wrote_explicit_deref(arg_expr)
            && matches!(arg_expr, Expression::FieldAccess { .. })
            && coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
        {
            coerced = coerced[1..].to_string();
        }

        // Terminal reconcile: for-loop iterable reuse and local owned-string moves.
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.for_loop_borrow_needed.contains(name) {
                if self.inferred_borrowed_params.contains(name) {
                    coerced = name.clone();
                } else if coerced.ends_with(".clone()") {
                    let base = coerced
                        .trim_end_matches(".clone()")
                        .trim()
                        .trim_start_matches('&');
                    coerced = format!("&{base}");
                } else if !coerced.starts_with('&') {
                    coerced = format!(
                        "&{}",
                        crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
                    );
                }
            } else {
                let local_owned_text =
                    self.local_var_types.get(name).is_some_and(|t| {
                        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                            && crate::codegen::rust::types::is_windjammer_text_type(t)
                    }) || self.infer_expression_type(arg_expr).is_some_and(|t| {
                        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                            && crate::codegen::rust::types::is_windjammer_text_type(&t)
                    });
                let reuse_after_move = !self.current_stmt_restores_binding_after_move(name)
                    && (self.auto_clone_analysis.as_ref().is_some_and(|a| {
                        a.needs_clone(name, self.current_statement_idx).is_some()
                    }) || {
                        let later: Vec<&Statement<'ast>> = self
                            .current_function_body
                            .iter()
                            .skip(self.current_statement_idx + 1)
                            .copied()
                            .collect();
                        !later.is_empty() && Self::variable_used_in_statements(&later, name)
                    });
                let callee_wants_owned_text =
                    crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &sig, param_idx,
                    ) || (sig.formal_param_type(param_idx).is_some_and(|t| {
                        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                            && crate::codegen::rust::types::is_windjammer_text_type(t)
                    }) && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                        &sig, param_idx,
                    ) && sig
                        .emitted_rust_ref_params
                        .as_ref()
                        .and_then(|flags| flags.get(param_idx).copied())
                        != Some(true));
                if local_owned_text
                    && callee_wants_owned_text
                    && !self.into_string_formal_params.contains(name)
                {
                    if reuse_after_move {
                        let base =
                            crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
                                .trim_end_matches(".clone()")
                                .trim()
                                .to_string();
                        if !base.ends_with(".clone()") {
                            coerced = format!("{base}.clone()");
                        } else {
                            coerced = base;
                        }
                    } else if coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                        // P3.264: owned local into owned String formal — move, do not over-borrow.
                        coerced =
                            crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
                                .to_string();
                    }
                }
            }
        }

        if let Expression::Identifier { name, .. } = arg_expr {
            let slot_sig =
                self.refreshed_call_site_sig_for_arg(registry, callee_name, arg_index, &sig);
            let slot_pidx = slot_sig.arg_param_index(arg_index);
            let callee_shared = crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                &slot_sig, slot_pidx,
            ) || slot_sig
                .formal_param_type(slot_pidx)
                .or_else(|| slot_sig.param_types.get(slot_pidx))
                .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_str_ref);
            let caller_owned_string_formal = self.current_function_params.iter().any(|p| {
                p.name == *name && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
            }) && !self.emitted_rust_ref_formals.contains(name)
                && !self.str_ref_optimized_params.contains(name.as_str())
                && !self.into_string_formal_params.contains(name);
            if let Some(cloned) =
                crate::codegen::rust::call_site_borrow::clone_reused_binding_for_owned_vec_formal(
                    self,
                    &slot_sig,
                    arg_index,
                    arg_expr,
                    &coerced,
                    Some(callee_name),
                )
            {
                coerced = cloned;
            } else if callee_shared
                && !Self::sig_arg_confirms_owned_emission(&slot_sig, arg_index)
                && !self.cross_crate_dep_arg_confirms_owned(callee_name, arg_index)
                && !coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
                && !self.identifier_binding_already_rust_ref(name)
                && !self.into_string_formal_params.contains(name)
                && (caller_owned_string_formal
                    || self.local_binding_reused_after_current_statement(name))
            {
                coerced = format!(
                    "&{}",
                    crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced)
                );
            }
        }
        coerced = crate::codegen::rust::call_site_borrow::normalize_explicit_deref_copy_operand(
            arg_expr, &coerced,
        );

        self.peel_fn_trait_or_closure_call_arg(
            &mut coerced,
            arg_expr,
            callee_name,
            &sig,
            arg_index,
        );

        if thin_wrap_into_string_formal {
            while coerced.starts_with("&mut ") {
                coerced = coerced["&mut ".len()..].trim().to_string();
            }
            while coerced.starts_with('&') {
                coerced = coerced[1..].trim().to_string();
            }
            crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut coerced);
            if !coerced.ends_with(".into()")
                && !coerced.ends_with(".to_string()")
                && !coerced.ends_with(".to_owned()")
            {
                coerced = format!("{coerced}.into()");
            }
        }

        if self.in_if_condition {
            if let Expression::Identifier { name, .. } = arg_expr {
                let body: Vec<_> = self.current_function_body.iter().copied().collect();
                if (self.current_fn_forward_ref_if_params.contains(name)
                    || self.current_fn_mixed_forwarder_params.contains(name))
                    && self.param_used_in_if_with_condition_and_branches(&body, name)
                    && self.caller_owned_non_copy_formal(name)
                {
                    if Self::sig_arg_confirms_owned_emission(&sig, arg_index)
                        || crate::codegen::rust::call_site_borrow::callee_formal_is_owned_vec_container(
                            &sig, param_idx,
                        )
                    {
                        if !coerced.ends_with(".clone()") {
                            coerced = format!("{coerced}.clone()");
                        }
                    } else if coerced.ends_with(".clone()") {
                        let base = coerced.trim_end_matches(".clone()").trim();
                        coerced = if base.starts_with('&') {
                            base.to_string()
                        } else {
                            format!("&{base}")
                        };
                    } else if coerced.starts_with("&mut ") {
                        coerced = format!(
                            "&{}",
                            crate::codegen::rust::expression_utilities::borrow_base_expr(&coerced),
                        );
                    } else if !coerced.starts_with('&') {
                        coerced = format!("&{coerced}");
                    }
                }
            }
        }

        if let Some(rt) = receiver_type_name {
            if let Some(resolved) = self.resolve_method_function_signature(
                rt,
                method_simple,
                user_arg_count.unwrap_or(arg_index + 1),
            ) {
                let ridx = resolved.arg_param_index(arg_index);
                let owned_user_slot =
                    crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &resolved, ridx,
                    ) || matches!(
                        resolved.param_ownership.get(ridx),
                        Some(crate::analyzer::OwnershipMode::Owned)
                    );
                if owned_user_slot
                    && !resolved
                        .param_types
                        .get(ridx)
                        .is_some_and(|t| matches!(t, Type::MutableReference(_)))
                    && (coerced.starts_with("&mut ") || coerced.starts_with('&'))
                {
                    coerced =
                        crate::codegen::rust::expression_utilities::coerce_borrowed_arg_to_owned(
                            &coerced,
                        );
                }
            }
        }

        // Terminal: codegen-confirmed demoted `&str` / preregistered shared formals must
        // borrow owned caller params (`compare_identifiers` → `cmp_string(&left, &right)`).
        // Stale registry `param_types: Reference(str)` alone must not force borrow when
        // emission still owns `String` (P3.389 regression / join_path seed).
        if let Expression::Identifier { name, .. } = arg_expr {
            let lookup_callee = self.signature_lookup_callee_name(callee_name);
            let lookup_ref = lookup_callee.as_ref();
            let import_alias_resolved = self.import_fn_alias_map.contains_key(callee_name);
            let cross_crate_import =
                self.is_import_alias_cross_crate_call(callee_name) || lookup_ref != callee_name;
            let dep_emits_shared = (cross_crate_import || import_alias_resolved)
                && self.global_signature_registry.as_ref().is_some_and(|g| {
                    let dep_sig = if import_alias_resolved {
                        g.get_signature(lookup_ref)
                    } else {
                        let simple = lookup_ref.rsplit("::").next().unwrap_or(lookup_ref);
                        g.get_signature(lookup_ref)
                            .or_else(|| g.get_signature(simple))
                    };
                    dep_sig.is_some_and(|rs| {
                            let pidx = rs.arg_param_index(arg_index);
                            if matches!(
                                rs.param_ownership.get(pidx),
                                Some(crate::analyzer::OwnershipMode::Owned)
                            ) {
                                return false;
                            }
                            if rs.param_types.get(pidx).is_some_and(|t| {
                                matches!(t, crate::parser::Type::String)
                                    || matches!(
                                        t,
                                        crate::parser::Type::Custom(n)
                                            if n == "String"
                                    )
                            }) && rs
                                .emitted_rust_ref_params
                                .as_ref()
                                .and_then(|f| f.get(pidx))
                                != Some(&true)
                            {
                                return false;
                            }
                            crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                                rs, pidx,
                            ) && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                rs, pidx,
                            )
                        })
                });
            let callee_wants_shared = if cross_crate_import || import_alias_resolved {
                dep_emits_shared
            } else {
                let registry_sig_shared = |reg: &SignatureRegistry| {
                    reg.get_signature(callee_name)
                        .or_else(|| reg.get_signature(lookup_ref))
                        .or_else(|| {
                            let simple = lookup_ref.rsplit("::").next().unwrap_or(lookup_ref);
                            reg.get_signature(simple)
                        })
                        .is_some_and(|rs| {
                            let pidx = rs.arg_param_index(arg_index);
                            crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                                rs, pidx,
                            )
                        })
                };
                registry_sig_shared(registry)
                    || self
                        .global_signature_registry
                        .as_ref()
                        .is_some_and(|g| registry_sig_shared(g))
                    || self.preregistered_free_call_arg_expects_borrow(callee_name, arg_index)
                    || crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(
                        &sig, param_idx,
                    )
            };
            if callee_wants_shared
                && self.caller_owned_non_copy_formal(name)
                && !self.emitted_rust_ref_formals.contains(name)
                && !coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
            {
                coerced = format!("&{coerced}");
            } else if cross_crate_import
                && dep_emits_shared
                && !coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
            {
                // Path-dep metadata: honor dependency `Borrowed` / `emitted_rust_ref_params`
                // even when the caller formal was demoted to `&str` (explicit `&` at boundary).
                coerced = format!("&{coerced}");
            }
            // Peel IR over-borrow into owned String slots (wj-todo-cli → wj-validate
            // `require_nonempty(&field, &value)` → `(&field, value)`).
            if coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
                && (self.cross_crate_dep_arg_confirms_owned(callee_name, arg_index)
                    || Self::sig_arg_confirms_owned_emission(&sig, arg_index))
            {
                coerced = crate::codegen::rust::expression_utilities::coerce_borrowed_arg_to_owned(
                    &coerced,
                );
            }

            // Already-demoted caller `&str` / `&T`: bare at shared-ref call sites.
            if self.caller_formal_emitted_shared_ref(name)
                && coerced == format!("&{name}")
                && !cross_crate_import
            {
                coerced = name.to_string();
            }

            let mut tmp = coerced.clone();
            if crate::codegen::rust::string_utilities::rewrite_borrowed_str_clone_to_to_string(
                &mut tmp,
                arg_expr,
                &self.emitted_rust_ref_formals,
                &self.current_function_params,
            ) {
                coerced = tmp;
            } else if self.emitted_rust_ref_formals.contains(name) && coerced.ends_with(".clone()")
            {
                // Text demotions: `&str` → owned String needs `.to_string()`.
                // P3.390: demoted `&Vec` / non-text must NEVER get `.to_string()` (E0599).
                // Shared-ref callees: strip stale auto-clone and pass the already-borrowed
                // binding. Owned callees: keep `.clone()` (WDB-281 / demoted→owned).
                let is_text = self.current_function_params.iter().any(|p| {
                    p.name == *name
                        && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                });
                if is_text {
                    coerced = format!("{}.to_string()", name);
                } else {
                    // Keep `.clone()` only when codegen confirmed owned emission (WDB-281).
                    // Bare Vec AST makes `callee_wants_shared` false — do not gate on it.
                    let mut live = sig.clone();
                    let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
                    let refresh_keys = [callee_name.to_string(), simple.to_string()];
                    crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                        &mut live,
                        registry,
                        &refresh_keys,
                    );
                    if let Some(global) = self.global_signature_registry.as_ref() {
                        crate::codegen::rust::signature_promotion::merge_registry_codegen_refresh_if_present(
                            &mut live,
                            global,
                            &refresh_keys,
                        );
                    }
                    let pidx = live.arg_param_index(arg_index);
                    let confirmed_owned_emit = live
                        .emitted_rust_ref_params
                        .as_ref()
                        .and_then(|f| f.get(pidx).copied())
                        == Some(false);
                    if !confirmed_owned_emit {
                        coerced = name.to_string();
                    }
                }
            }
        }

        crate::codegen::rust::string_utilities::rewrite_demoted_text_param_str_clones_in_rust_expr(
            &mut coerced,
            &self.emitted_rust_ref_formals,
            &self.str_ref_optimized_params,
            &self.inferred_borrowed_params,
            &self.current_function_params,
        );

        // Terminal: never leave `n as usize.clone()` (WDB-300).
        coerced =
            crate::codegen::rust::expression_utilities::sanitize_cast_trailing_clone(&coerced);

        Some(coerced)
    }

    /// Signature-driven map/set key finalize: strip `&&` on already-shared bindings,
    /// then ensure `&K` when the formal is a collection-key lookup.
    ///
    /// Binding awareness lives here (IR) so method/free-call sites stay DRY.
    fn finalize_ir_collection_key_arg(
        &self,
        arg_str: &mut String,
        arg_expr: &Expression<'ast>,
        callee_name: &str,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
        receiver_type_name: Option<&str>,
    ) {
        let key_receiver = crate::codegen::rust::stdlib_method_traits::collection_key_receiver_type(
            callee_name,
            receiver_type_name,
            sig,
        );
        if !self.is_collection_key_lookup_at_site(sig, arg_index, key_receiver.as_deref()) {
            return;
        }

        if arg_str.ends_with(".to_string()")
            && matches!(
                arg_expr,
                Expression::Identifier { .. } | Expression::FieldAccess { .. }
            )
        {
            *arg_str = arg_str.trim_end_matches(".to_string()").to_string();
        }

        let binding_name =
            crate::codegen::rust::call_site_borrow::borrow_target_identifier_name(arg_expr);
        let binding_already_shared = binding_name.as_ref().is_some_and(|name| {
            self.emitted_rust_ref_formals.contains(name)
                || self.str_ref_optimized_params.contains(name.as_str())
                || self.binding_emits_as_rust_shared_ref(name)
                || self.identifier_already_ref(name)
                || (self.inferred_borrowed_params.contains(name.as_str())
                    && !self.collection_key_owned_params.contains(name.as_str()))
        });
        let text_param_already_shared = binding_name.as_ref().is_some_and(|name| {
            !self.collection_key_owned_params.contains(name.as_str())
                && self.current_function_params.iter().any(|p| {
                    p.name == *name
                        && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                })
                && (self.emitted_rust_ref_formals.contains(name)
                    || self.str_ref_optimized_params.contains(name.as_str())
                    || self.inferred_borrowed_params.contains(name.as_str())
                    || self.identifier_already_ref(name))
        });
        let arg_already_rust_ref = binding_name.as_ref().is_some_and(|name| {
            self.identifier_already_ref(name)
                || self.emitted_rust_ref_formals.contains(name)
                || self.str_ref_optimized_params.contains(name.as_str())
                || self.inferred_borrowed_params.contains(name.as_str())
        });

        // Drop a spurious leading `&` when the binding already lowers as `&T` / `&str`
        // (HashMap::get(key) never HashMap::get(&key) → &&str).
        if binding_already_shared
            || text_param_already_shared
            || binding_name
                .as_ref()
                .is_some_and(|name| self.current_function_params.iter().any(|p| p.name == *name))
        {
            crate::codegen::rust::call_site_borrow::strip_redundant_borrow_on_ref_binding(
                arg_expr, arg_str,
            );
        }

        if binding_already_shared || text_param_already_shared {
            crate::codegen::rust::expression_utilities::strip_trailing_clone(arg_str);
            if arg_str.ends_with(".to_string().clone()") {
                let base = arg_str.trim_end_matches(".to_string().clone()");
                *arg_str = base.to_string();
            } else if arg_str.ends_with(".to_string()") {
                let base = arg_str.trim_end_matches(".to_string()");
                *arg_str = base.to_string();
            }
        }

        crate::codegen::rust::call_site_borrow::finalize_collection_key_call_site_arg(
            Some(sig),
            arg_index,
            arg_expr,
            arg_str,
            arg_already_rust_ref,
            key_receiver.as_deref(),
            binding_already_shared || text_param_already_shared,
        );
    }

    /// When a binding/path is moved and reused, and the final argument is passed by
    /// value (no leading `&`), ensure `.clone()` is present (regression-059).
    fn ensure_owned_move_clone_for_reuse(
        &self,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
        sig: &crate::analyzer::FunctionSignature,
        param_idx: usize,
        callee_name: &str,
        arg_index: usize,
    ) -> String {
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.caller_emits_mut_ref_formal(name)
                && self.callee_slot_emits_mut_borrow(callee_name, arg_index)
            {
                return arg_str.to_string();
            }
        }
        // Mut-ref call sites reborrow — never append `.clone()` onto `&mut place`
        // (auto_mut `fill(&mut buf)` when `buf` is reused after the call).
        if arg_str.starts_with("&mut ")
            || self.ir_sig_arg_expects_mut_borrow(sig, arg_index)
            || self.callee_slot_emits_mut_borrow(callee_name, arg_index)
        {
            return arg_str.to_string();
        }
        let callee_wants_shared_ref = crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
            sig, param_idx,
        ) || crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, param_idx)
            || crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(sig, param_idx)
            || crate::codegen::rust::stdlib_method_traits::method_arg_expects_rust_str_ref_from_sig(
                sig, arg_index,
            )
            || (sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx))
                .is_some_and(|t| matches!(t, Type::Reference(_)))
                && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    sig, param_idx,
                ));
        if callee_wants_shared_ref {
            let mut normalize_shared_ref_borrow = |mut base: String| -> String {
                base =
                    crate::codegen::rust::expression_utilities::borrow_base_expr(&base).to_string();
                if base.ends_with(".to_string()") {
                    base = base.trim_end_matches(".to_string()").to_string();
                }
                crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut base);
                if base.starts_with('&') {
                    base
                } else {
                    format!("&{base}")
                }
            };
            match arg_expr {
                Expression::Identifier { name, .. } => {
                    let caller_text_borrow = self.str_ref_optimized_params.contains(name)
                        || (self.emitted_rust_ref_formals.contains(name)
                            && self.current_function_params.iter().any(|p| {
                                p.name == *name
                                    && crate::codegen::rust::types::is_windjammer_text_type(
                                        &p.type_,
                                    )
                            }));
                    if caller_text_borrow {
                        return normalize_shared_ref_borrow(arg_str.to_string());
                    }
                }
                Expression::FieldAccess { .. }
                | Expression::Index { .. }
                | Expression::Call { .. }
                | Expression::MethodCall { .. } => {
                    return normalize_shared_ref_borrow(arg_str.to_string());
                }
                _ => {}
            }
        }
        if let Some(rewritten) = self.try_self_field_writeback_owned_arg(arg_expr, arg_str) {
            return rewritten;
        }
        if arg_str.ends_with(".clone()") || arg_str.starts_with('*') {
            return arg_str.to_string();
        }
        if arg_str.contains("std::mem::take(") {
            return arg_str.to_string();
        }
        if arg_str.ends_with(".to_string()") {
            let callee_wants_str_ref =
                crate::ir::emission_contract::callee_emits_shared_rust_ref_param(sig, param_idx)
                    || crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, param_idx)
                    || sig.string_ref_string_formal_for_arg(param_idx)
                    || sig.formal_param_type(param_idx).is_some_and(|t| {
                        crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                    });
            if callee_wants_str_ref {
                let base = arg_str.trim_end_matches(".to_string()");
                if let Expression::Identifier { name, .. } = arg_expr {
                    if self.emitted_rust_ref_formals.contains(name) {
                        return base.to_string();
                    }
                }
                return crate::codegen::rust::expression_utilities::borrow_base_expr(base)
                    .to_string();
            }
            return arg_str.to_string();
        }
        // `if !callee(mut_param)` then `{ mut_param.mutate() }` — owned formal in the
        // condition moves before the then-branch reuses the binding (list_unique / ReBAC).
        if self.in_if_condition {
            if let Expression::Identifier { name, .. } = arg_expr {
                if (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    sig, param_idx,
                ) || Self::sig_arg_confirms_owned_emission(sig, arg_index))
                    && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                        sig, param_idx,
                    )
                    && self
                        .current_function_params
                        .iter()
                        .any(|p| p.name == *name && !self.is_type_copy(&p.type_))
                {
                    return format!("{arg_str}.clone()");
                }
            }
        }
        // Indexing a non-Copy element into a non-shared-ref formal is always an
        // invalid move (E0507). Analyzer may still mark `(Row, T)` chain helpers
        // Borrowed while codegen emits owned `Row` — trust shared-ref emission.
        if matches!(arg_expr, Expression::Index { .. }) {
            let emits_shared = [
                sig.name.as_str(),
                sig.name.rsplit("::").next().unwrap_or(&sig.name),
            ]
            .iter()
            .find_map(|key| {
                self.signature_registry
                    .get_signature(key)
                    .or_else(|| {
                        self.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(key))
                    })
                    .and_then(|s| s.emitted_rust_ref_params.as_ref())
                    .and_then(|flags| flags.get(param_idx).copied())
            })
            .unwrap_or_else(|| {
                sig.emitted_rust_ref_params
                    .as_ref()
                    .and_then(|flags| flags.get(param_idx).copied())
                    .unwrap_or_else(|| {
                        crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                            sig, param_idx,
                        )
                    })
            });
            if !emits_shared {
                if self.index_expression_is_copy_scalar(arg_expr) {
                    return arg_str.to_string();
                }
                let needs_clone = match self.infer_expression_type(arg_expr) {
                    None => true,
                    Some(t) => {
                        let bare = match &t {
                            Type::Reference(inner) | Type::MutableReference(inner) => {
                                inner.as_ref()
                            }
                            other => other,
                        };
                        !self.is_type_copy(bare) || matches!(bare, Type::Custom(_))
                    }
                };
                if needs_clone {
                    return format!("{arg_str}.clone()");
                }
            }
        }
        // Loop / map.values() bindings (`ach: &Achievement`) into owned `Vec::push` (P3.303).
        if matches!(arg_expr, Expression::Identifier { .. }) {
            let emits_shared = [
                sig.name.as_str(),
                sig.name.rsplit("::").next().unwrap_or(&sig.name),
            ]
            .iter()
            .find_map(|key| {
                self.signature_registry
                    .get_signature(key)
                    .or_else(|| {
                        self.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(key))
                    })
                    .and_then(|s| s.emitted_rust_ref_params.as_ref())
                    .and_then(|flags| flags.get(param_idx).copied())
            })
            .unwrap_or_else(|| {
                sig.emitted_rust_ref_params
                    .as_ref()
                    .and_then(|flags| flags.get(param_idx).copied())
                    .unwrap_or_else(|| {
                        crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                            sig, param_idx,
                        )
                    })
            });
            if !emits_shared {
                if let Expression::Identifier { name, .. } = arg_expr {
                    // P3.303: borrowed loop/map.values() elems into owned push.
                    if self.borrowed_iterator_vars.contains(name)
                        && !self.binding_is_copy_pass_by_value_scalar(name)
                        && !arg_str.ends_with(".clone()")
                    {
                        return format!("{arg_str}.clone()");
                    }
                    // Do NOT blanket-clone every non-Copy ident into owned slots.
                    // That undoes move+rebind writeback (`let got = recv_int(rx);
                    // rx = got.0`) and clones non-Clone Receivers (P3.310).
                    // Reuse cloning continues below via auto_clone + writeback.
                }
            }
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.match_arm_bindings.contains(name.as_str()) {
                let mut out = arg_str.to_string();
                if out.ends_with(".clone()") {
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut out);
                }
                let expects_shared_text =
                    crate::ir::signature_bridge::call_site_wants_shared_text_ref(sig, param_idx);
                if expects_shared_text && !out.starts_with('&') {
                    out = format!("&{out}");
                }
                return out;
            }
            if (self.borrowed_iterator_vars.contains(name)
                || self
                    .local_var_types
                    .get(name)
                    .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_))))
                && self
                    .current_function_return_type
                    .as_ref()
                    .is_some_and(|rt| {
                        crate::codegen::rust::types::return_type_is_vec_of_shared_refs(Some(rt))
                    })
            {
                let mut out = arg_str.to_string();
                crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut out);
                return out;
            }
        }
        // Mut-ref formals must keep `&mut binding` — never append `.clone()`.
        if arg_str.starts_with("&mut ") {
            return arg_str.to_string();
        }
        // When reuse requires clone but a stale shared-borrow prefix was applied
        // (`&value` into owned `value: Value`), strip `&` and clone (regression-063).
        if arg_str.starts_with('&') {
            let callee_owned_emission =
                crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    sig, param_idx,
                ) || self.preregistered_free_call_arg_emits_owned(
                    &sig.name,
                    param_idx.saturating_sub(usize::from(sig.has_self_receiver_slot())),
                );
            let needs = match arg_expr {
                Expression::Identifier { name, .. } => {
                    self.local_binding_reused_after_current_statement(name)
                }
                Expression::FieldAccess { .. } | Expression::Index { .. } => {
                    self.auto_clone_analysis.as_ref().is_some_and(|analysis| {
                        Self::auto_clone_expr_path(arg_expr).is_some_and(|path| {
                            analysis
                                .needs_clone(&path, self.current_statement_idx)
                                .is_some()
                        })
                    })
                }
                _ => false,
            };
            if needs
                && callee_owned_emission
                && !crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                    &self.signature_registry,
                    &sig.name,
                    Some(sig),
                    param_idx.saturating_sub(usize::from(sig.has_self_receiver_slot())),
                )
            {
                let base = crate::codegen::rust::expression_utilities::borrow_base_expr(arg_str);
                let skip = match arg_expr {
                    Expression::Identifier { name, .. } => {
                        self.binding_is_copy_pass_by_value_scalar(name)
                    }
                    _ => false,
                };
                if !skip {
                    return format!("{base}.clone()");
                }
                return base.to_string();
            }
            let Some(ref analysis) = self.auto_clone_analysis else {
                return arg_str.to_string();
            };
            // Statement-local reuse only — `needs_clone_anywhere` falsely clones
            // discard-only / single-use owned args reused across calls (authz-reuse regression).
            let needs = match arg_expr {
                Expression::Identifier { name, .. } => analysis
                    .needs_clone(name, self.current_statement_idx)
                    .is_some(),
                Expression::FieldAccess { .. } | Expression::Index { .. } => {
                    Self::auto_clone_expr_path(arg_expr).is_some_and(|path| {
                        analysis
                            .needs_clone(&path, self.current_statement_idx)
                            .is_some()
                    })
                }
                _ => false,
            };
            if needs
                && callee_owned_emission
                && !crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                    &self.signature_registry,
                    &sig.name,
                    Some(sig),
                    param_idx.saturating_sub(usize::from(sig.has_self_receiver_slot())),
                )
            {
                let base = crate::codegen::rust::expression_utilities::borrow_base_expr(arg_str);
                let skip = match arg_expr {
                    Expression::Identifier { name, .. } => {
                        self.binding_is_copy_pass_by_value_scalar(name)
                    }
                    _ => false,
                };
                if !skip {
                    return format!("{base}.clone()");
                }
                return base.to_string();
            }
            return arg_str.to_string();
        }
        if matches!(
            sig.param_ownership.get(param_idx),
            Some(crate::analyzer::OwnershipMode::MutBorrowed)
        ) || sig
            .param_types
            .get(param_idx)
            .is_some_and(|t| matches!(t, Type::MutableReference(_)))
        {
            return arg_str.to_string();
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, param_idx)
                && self.caller_param_has_later_owned_formal_pass(name)
            {
                return self.append_clone_for_owned_non_copy_binding(name, arg_str);
            }
        }
        let Some(ref analysis) = self.auto_clone_analysis else {
            return arg_str.to_string();
        };
        // Statement-local reuse only (matches method-arg policy in arguments.rs).
        // `needs_clone_anywhere` covers stmt-index drift for multipass; writeback
        // detection must keep false clone sites off rebound bindings
        // (`let (row, id) = f(row); g(row)`).
        let needs = match arg_expr {
            Expression::Identifier { name, .. } => {
                // WDB-367: unit keywords are not bindings — multipass `needs_clone_anywhere`
                // on `"None"` yields `None.clone()` at unrelated push sites (tilemap/graph).
                if name == "None"
                    || name == "true"
                    || name == "false"
                    || name.ends_with("::None")
                    || crate::type_classification::is_enum_variant_constructor_path(name)
                {
                    false
                } else {
                    let local = analysis
                        .needs_clone(name, self.current_statement_idx)
                        .is_some();
                    let anywhere =
                        crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            sig, param_idx,
                        ) && !self.binding_is_copy_pass_by_value_scalar(name)
                            && analysis.needs_clone_anywhere(name)
                            && !self.current_stmt_restores_binding_after_move(name);
                    let reused_owned =
                        crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            sig, param_idx,
                        ) && self.caller_owned_non_copy_formal(name)
                            && !self.binding_is_copy_pass_by_value_scalar(name)
                            && self.param_has_later_owned_formal_pass(
                                name,
                                self.current_statement_idx,
                            );
                    local || anywhere || reused_owned
                }
            }
            Expression::FieldAccess { .. } | Expression::Index { .. } => {
                Self::auto_clone_expr_path(arg_expr).is_some_and(|path| {
                    if path == "None" || path.ends_with(".None") || path.ends_with("::None") {
                        return false;
                    }
                    let local = analysis
                        .needs_clone(&path, self.current_statement_idx)
                        .is_some();
                    // Statement-idx drift under multipass / nested blocks: same
                    // owned-formal fallback as bare identifiers (field multi-use).
                    let anywhere =
                        crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            sig, param_idx,
                        ) && analysis.needs_clone_anywhere(&path);
                    local || anywhere
                })
            }
            _ => false,
        };
        if needs {
            // Borrow callees never consume — loop reuse passes borrow/bare, not `.clone()`.
            if callee_wants_shared_ref {
                if let Expression::Identifier { name, .. } = arg_expr {
                    let caller_text_borrow = self.str_ref_optimized_params.contains(name)
                        || (self.emitted_rust_ref_formals.contains(name)
                            && self.current_function_params.iter().any(|p| {
                                p.name == *name
                                    && crate::codegen::rust::types::is_windjammer_text_type(
                                        &p.type_,
                                    )
                            }));
                    if caller_text_borrow {
                        let mut base =
                            crate::codegen::rust::expression_utilities::borrow_base_expr(arg_str)
                                .to_string();
                        if base.ends_with(".to_string()") {
                            base = base.trim_end_matches(".to_string()").to_string();
                        }
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut base);
                        return base;
                    }
                }
                if arg_str.starts_with('&') {
                    return arg_str.to_string();
                }
                return format!(
                    "&{}",
                    crate::codegen::rust::expression_utilities::borrow_base_expr(arg_str)
                );
            }
            let preserve_for_reuse = match arg_expr {
                Expression::Identifier { name, .. } => {
                    analysis
                        .needs_clone(name, self.current_statement_idx)
                        .is_some()
                        || self.caller_param_has_later_owned_formal_pass(name)
                }
                Expression::FieldAccess { .. } | Expression::Index { .. } => {
                    Self::auto_clone_expr_path(arg_expr).is_some_and(|path| {
                        analysis
                            .needs_clone(&path, self.current_statement_idx)
                            .is_some()
                            || (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                sig, param_idx,
                            ) && analysis.needs_clone_anywhere(&path))
                    })
                }
                _ => false,
            };
            if crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, param_idx)
                && !preserve_for_reuse
            {
                if arg_str.starts_with('&') {
                    return arg_str.to_string();
                }
                return format!(
                    "&{}",
                    crate::codegen::rust::expression_utilities::borrow_base_expr(arg_str)
                );
            }
            // Scalar Copy formals (i64/bool/…) need no clone; Copy aggregates/enums still do.
            let skip_clone = match arg_expr {
                Expression::Identifier { name, .. } => {
                    self.binding_is_copy_pass_by_value_scalar(name)
                        || self
                            .current_function_params
                            .iter()
                            .find(|p| p.name == *name)
                            .is_some_and(|p| self.is_type_copy(&p.type_))
                        || self
                            .local_var_types
                            .get(name)
                            .is_some_and(|t| self.is_type_copy(t))
                }
                _ => self.infer_expression_type(arg_expr).is_some_and(|t| {
                    let bare = match &t {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    self.is_type_copy(bare)
                }),
            };
            if skip_clone {
                arg_str.to_string()
            } else {
                format!("{arg_str}.clone()")
            }
        } else if let Expression::Identifier { name, .. } = arg_expr {
            if self.caller_demoted_non_copy_formal_into_owned_callee(name)
                && !arg_str.ends_with(".clone()")
                && !arg_str.ends_with(".to_string()")
                && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                    sig, param_idx,
                )
                && !crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(sig, param_idx)
                && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    sig, param_idx,
                ) || crate::codegen::rust::signature_promotion::wj_registry_bare_owned_formal_slot(
                    sig, param_idx,
                ))
            {
                let is_text = self.current_function_params.iter().any(|p| {
                    p.name == *name
                        && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                });
                if is_text {
                    crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                        crate::codegen::rust::expression_utilities::borrow_base_expr(&arg_str),
                    )
                } else {
                    let base =
                        crate::codegen::rust::expression_utilities::borrow_base_expr(&arg_str);
                    format!("{base}.clone()")
                }
            } else {
                arg_str.to_string()
            }
        } else {
            arg_str.to_string()
        }
    }

    /// Enforce the ownership contract for a single call-site argument by computing
    /// actual vs expected safety types and applying the coercion (strip `&`, add `.clone()`, etc.).
    /// Signature-driven: explicit `&x` at call site → owned callee formal gets `*x` (Copy) or `x.clone()`.
    pub(crate) fn coerce_explicit_ref_for_owned_callee_arg(
        &self,
        arg_expr: &Expression<'ast>,
        mut arg_str: String,
        sig: Option<&crate::analyzer::FunctionSignature>,
        arg_index: usize,
    ) -> String {
        if !crate::codegen::rust::expression_helpers::is_reference_expression(arg_expr) {
            return arg_str;
        }
        let Some(sig) = sig else {
            return arg_str;
        };
        let callee_wants_borrow =
            sig.param_types
                .get(arg_index)
                .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_)))
                || matches!(
            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                sig, arg_index,
            ),
            crate::analyzer::OwnershipMode::Borrowed | crate::analyzer::OwnershipMode::MutBorrowed,
        );
        if callee_wants_borrow {
            return arg_str;
        }
        let formal_idx = sig.arg_param_index(arg_index);
        let callee_formal_is_copy = sig
            .formal_param_type(formal_idx)
            .or_else(|| sig.param_types.get(formal_idx))
            .is_some_and(|t| match t {
                Type::Reference(inner) | Type::MutableReference(inner) => self.is_type_copy(inner),
                other => self.is_type_copy(other),
            });
        // Undo erroneous `*(&x)` / `*&x` from legacy deref coercion on explicit borrows.
        while arg_str.starts_with('*') {
            arg_str = arg_str[1..].to_string();
        }
        if arg_str.ends_with(".clone()") {
            return arg_str;
        }
        if callee_formal_is_copy {
            // Explicit `&owned` / `(&owned)` → owned Copy formal: pass by value
            // (Rust auto-copies). Never emit `*owned` — that E0614s when `owned`
            // is already a value local (IR may have stripped `&` already).
            let mut s = arg_str.trim().to_string();
            if s.starts_with('(') && s.ends_with(')') {
                let inner = s[1..s.len() - 1].trim().to_string();
                if inner.starts_with('&') || inner.starts_with("&mut ") {
                    s = inner;
                }
            }
            return crate::codegen::rust::expression_utilities::borrow_base_expr(&s).to_string();
        }
        let inner = crate::codegen::rust::expression_utilities::borrow_base_expr(&arg_str);
        // Parenthesized unary refs from expression codegen: `(&x)`.
        let inner = if arg_str.trim().starts_with('(') && arg_str.trim().ends_with(')') {
            let peeled = arg_str.trim();
            let mid = peeled[1..peeled.len() - 1].trim();
            if mid.starts_with('&') || mid.starts_with("&mut ") {
                crate::codegen::rust::expression_utilities::borrow_base_expr(mid)
            } else {
                inner
            }
        } else {
            inner
        };
        format!("{inner}.clone()")
    }

    /// Post-IR reconcile: apply mut-borrow and peel stale `&`/`&mut` for owned formals.
    ///
    /// Single source of truth for the clusters previously duplicated in
    /// `regular_call_arguments` / method `arguments` after `apply_ir_call_site_coercion`.
    /// Signature-driven only — no method-name ownership heuristics.
    pub(crate) fn reconcile_post_ir_mut_borrow_and_owned_peel(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        callee_name: &str,
        arg_index: usize,
        sig: &crate::analyzer::FunctionSignature,
        registry: &SignatureRegistry,
        receiver_type_name: Option<&str>,
        receiver: Option<&Expression<'ast>>,
        user_arg_count: Option<usize>,
        has_ownership_collision: bool,
    ) {
        // Prefer defining-module codegen refresh (`emitted_rust_ref_params`) over stale
        // importer/collision stubs before any owned-formal peel (WDB-101 map getters).
        let mut sig = self.refreshed_call_site_sig_for_arg(registry, callee_name, arg_index, sig);
        // Import aliases: never let bare homonym metadata override the qualified dep fn.
        if self.import_fn_alias_map.contains_key(callee_name) {
            if let Some(global) = self.global_signature_registry.as_ref() {
                let lookup = self.signature_lookup_callee_name(callee_name);
                if let Some(dep) = global.get_signature(lookup.as_ref()) {
                    sig = dep.clone();
                }
            }
        }
        let param_idx = sig.arg_param_index(arg_index);
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let is_collection_key_site = {
            let key_receiver =
                crate::codegen::rust::stdlib_method_traits::collection_key_receiver_type(
                    callee_name,
                    receiver_type_name,
                    &sig,
                );
            self.is_collection_key_lookup_at_site(&sig, arg_index, key_receiver.as_deref())
        };
        let skip_bare_homonym =
            crate::codegen::rust::call_signature_resolution::qualified_callee_skips_bare_homonym_lookup(
                callee_name,
            );
        let lookup_callee = self.signature_lookup_callee_name(callee_name);
        let lookup_ref = lookup_callee.as_ref();
        let import_alias_resolved = self.import_fn_alias_map.contains_key(callee_name);
        let global_confirms_shared_ref = |pidx: usize| {
            self.global_signature_registry.as_ref().is_some_and(|g| {
                let keys: Vec<&str> = if import_alias_resolved {
                    vec![lookup_ref]
                } else if skip_bare_homonym {
                    vec![callee_name, lookup_ref]
                } else {
                    vec![callee_name, lookup_ref, simple]
                };
                keys.into_iter()
                    .flat_map(|key| {
                        let mut out = vec![g.get_signature(key), g.lookup_method(key)];
                        if !skip_bare_homonym && !import_alias_resolved {
                            out.push(g.find_unique_signature_ending_with(simple));
                        }
                        out
                    })
                    .flatten()
                    .any(|gs| {
                        !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            gs, pidx,
                        ) && crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                            gs, pidx,
                        )
                    })
            })
        };
        let global_or_local_confirms_owned_emission = || {
            self.ir_callee_arg_emits_owned_contract(
                registry,
                callee_name,
                arg_index,
                user_arg_count,
                Some(&sig),
            ) || self.global_signature_registry.as_ref().is_some_and(|g| {
                self.ir_callee_arg_emits_owned_contract(
                    g,
                    callee_name,
                    arg_index,
                    user_arg_count,
                    Some(&sig),
                )
            })
        };

        // Terminal peel first: IR / collision paths may prefix `&` before reconcile runs.
        self.peel_stacked_amp_on_emitted_ref_binding(
            coerced,
            arg_expr,
            Some(&sig),
            arg_index,
            false,
        );

        // Ownership-collision: do not keep IR/heuristic `&` from a conflicting
        // Borrowed snapshot (draw_text homonyms). Confirmed shared-ref formals skip.
        if has_ownership_collision
            && !is_collection_key_site
            && crate::codegen::rust::call_signature_resolution::ownership_collision_blocks_autoborrow(
                simple,
            )
            && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                &sig, param_idx,
            )
            && !global_confirms_shared_ref(param_idx)
        {
            crate::codegen::rust::call_signature_resolution::strip_collision_blocked_call_site_coercions(
                coerced,
            );
        }

        // Prefer-shared enforce without stripping IR-confirmed shared refs (bug_e0308).
        self.enforce_ir_ownership_preserving_confirmed_shared_ref(
            coerced,
            arg_expr,
            callee_name,
            arg_index,
            &sig,
            registry,
        );

        // Spurious `&mut` on owned Copy-aggregate formals (regression-060).
        self.peel_spurious_mut_borrow_on_owned_copy_aggregate(
            coerced,
            callee_name,
            arg_index,
            &sig,
        );

        // Stale `&` on owned user free-fn formals (circular-dep / multipass).
        let registry_lookup = self.signature_lookup_callee_name(callee_name);
        let skip_stale_borrow = !registry_lookup.contains("::")
            && !self.preregistered_free_call_arg_expects_borrow(callee_name, arg_index)
            && crate::codegen::rust::call_site_borrow::skip_stale_borrow_on_owned_user_free_fn_with_global(
                registry,
                self.global_signature_registry.as_deref(),
                callee_name,
                &sig,
                param_idx,
                arg_index,
                registry_lookup.as_ref(),
            );
        if skip_stale_borrow
            && (coerced.starts_with("&mut ")
                || (coerced.starts_with('&') && !coerced.starts_with("&mut ")))
            && !self.ir_callee_arg_expects_mut_borrow(
                registry,
                callee_name,
                arg_index,
                user_arg_count,
                Some(&sig),
            )
        {
            *coerced =
                crate::codegen::rust::expression_utilities::borrow_base_expr(coerced).to_string();
        }

        // Shared-borrow reapply when IR used a stale stub and the refreshed sig borrows.
        // Skip on ownership collision unless codegen confirmed shared-ref emission.
        let allow_shared = if import_alias_resolved {
            crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&sig, param_idx)
                || global_confirms_shared_ref(param_idx)
        } else {
            !has_ownership_collision
                || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&sig, param_idx)
                || global_confirms_shared_ref(param_idx)
        };
        if allow_shared
            && !skip_stale_borrow
            && !crate::codegen::rust::call_site_borrow::user_wrote_explicit_deref(arg_expr)
            && !crate::codegen::rust::expression_helpers::is_reference_expression(arg_expr)
        {
            let skip_recursive_owned = matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if self.callee_is_recursive_self_call(callee_name)
                        && self.current_function_params.iter().any(|p| p.name == *name)
                        && !self.emitted_rust_ref_formals.contains(name)
                        && !self.str_ref_optimized_params.contains(name.as_str())
            );
            if !skip_recursive_owned {
                let arg_already_rust_ref = matches!(
                    arg_expr,
                    Expression::Identifier { name, .. }
                        if self.identifier_binding_already_rust_ref(name)
                );
                if arg_already_rust_ref {
                    let lookup_callee = self.signature_lookup_callee_name(callee_name);
                    let cross_crate_import = self.is_import_alias_cross_crate_call(callee_name)
                        || lookup_callee.as_ref().contains("::");
                    let dep_emits_shared = (cross_crate_import || import_alias_resolved)
                        && self.global_signature_registry.as_ref().is_some_and(|g| {
                            let lookup_ref = lookup_callee.as_ref();
                            let gs = if import_alias_resolved {
                                g.get_signature(lookup_ref)
                            } else {
                                let simple = lookup_ref.rsplit("::").next().unwrap_or(lookup_ref);
                                g.get_signature(lookup_ref)
                                    .or_else(|| g.get_signature(simple))
                            };
                            gs.is_some_and(|gs| {
                                    let pidx = gs.arg_param_index(arg_index);
                                    crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                                        gs, pidx,
                                    ) && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                        gs, pidx,
                                    )
                                })
                        });
                    if dep_emits_shared {
                        // Path-dep metadata: explicit `&` at import boundary even when the
                        // caller formal was demoted to `&str` / `&Vec` (apps/wj-find).
                        if let Expression::Identifier { name, .. } = arg_expr {
                            if !coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                                *coerced = format!("&{name}");
                            }
                        }
                    } else {
                        // Binding is already `&T` / `&mut T` in Rust — never prefix another `&`
                        // (`take_in_edges(&csr)` → `&&mut DenseCsr`).
                        *coerced =
                            crate::codegen::rust::expression_utilities::borrow_base_expr(coerced)
                                .to_string();
                        // Keep `.clone()` when the slot is owned (iterator `push(item)` into
                        // `Vec<T>`). Only strip clone for true reborrows into `&` / `&mut`.
                        let wants_owned = crate::ir::signature_bridge::call_site_expects_owned_pass(
                        &sig, param_idx,
                    ) || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &sig, param_idx,
                    );
                        if !wants_owned {
                            let borrow_slot =
                                crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(
                                    &sig, param_idx,
                                );
                            if borrow_slot
                            || (!self.must_preserve_auto_clone_for_reuse(arg_expr)
                                && !crate::codegen::rust::expression_helpers::is_explicit_user_clone_call(
                                    arg_expr,
                                ))
                        {
                            crate::codegen::rust::expression_utilities::strip_trailing_clone(
                                coerced,
                            );
                        }
                        }
                    }
                } else {
                    // Fresher-sig shared-borrow reapply via IR contract (not should_borrow).
                    let key_receiver =
                        crate::codegen::rust::stdlib_method_traits::collection_key_receiver_type(
                            callee_name,
                            receiver_type_name,
                            &sig,
                        );
                    let is_ck = self.is_collection_key_lookup_at_site(
                        &sig,
                        arg_index,
                        key_receiver.as_deref(),
                    );
                    // Map/set key borrows applied in apply_ir must not be peeled here when
                    // the callee is type-qualified (`HashMap::get`) but `sig` still reflects
                    // a homonym stub (`NoteStore::get`).
                    if is_ck && coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                        // keep existing borrow
                    } else if matches!(
                        arg_expr,
                        Expression::Call { .. } | Expression::MethodCall { .. }
                    ) {
                        // WDB-169/WDB-190: call temps autoborrow — never `&callee()`.
                    } else {
                        let mut expected =
                            crate::ir::signature_bridge::safety_type_from_signature_param(
                                &sig, param_idx,
                            );
                        if is_ck {
                            expected.ownership = OwnedType::Ref(Region::fresh(4));
                        }
                        let actual = self.infer_actual_safety_type(arg_expr, coerced.as_str());
                        crate::ir::coercion::enforce_ownership_contract_on_coerced_arg(
                            coerced, &actual, &expected,
                        );
                    }
                }
            }
        }

        // Mut-borrow from signature / codegen-recorded mut slots.
        // Defining-module owned Custom formals (field-forward restore / bare emit) must
        // not re-acquire `&mut` from stale MutBorrowed layered stubs.
        let ast_owned_slot = receiver_type_name.is_some_and(|rt| {
            self.struct_method_ast_formal_param_types
                .get(rt)
                .and_then(|methods| methods.get(simple))
                .and_then(|formals| formals.get(arg_index))
                .is_some_and(|t| {
                    !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                        && !self.is_type_copy(t)
                        && !crate::codegen::rust::types::is_windjammer_text_type(t)
                })
                // AST bare `Vec`/`Custom` is not an owned slot when analysis/emission
                // already inferred MutBorrowed (`fill(buf: Vec<f32>)` → `&mut Vec`).
                && !matches!(
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                        &sig, arg_index,
                    ),
                    crate::analyzer::OwnershipMode::MutBorrowed
                )
                && !sig
                    .param_types
                    .get(param_idx)
                    .is_some_and(|t| matches!(t, Type::MutableReference(_)))
                && !self.ir_sig_arg_expects_mut_borrow(&sig, arg_index)
        });
        let project_owned_slot = receiver_type_name.is_some_and(|rt| {
            self.resolve_method_function_signature(
                rt,
                simple,
                user_arg_count.unwrap_or(arg_index + 1),
            )
            .is_some_and(|resolved| {
                let ridx = resolved.arg_param_index(arg_index);
                (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &resolved, ridx,
                ) || crate::codegen::rust::signature_promotion::bare_formal_is_owned_user_type(
                    &resolved, ridx,
                ) || matches!(
                    resolved.param_ownership.get(ridx),
                    Some(crate::analyzer::OwnershipMode::Owned)
                )) && !resolved
                    .param_types
                    .get(ridx)
                    .is_some_and(|t| matches!(t, Type::MutableReference(_)))
            })
        });
        let owned_slot = ast_owned_slot
            || project_owned_slot
            || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                &sig, param_idx,
            )
            || crate::codegen::rust::signature_promotion::bare_formal_is_owned_user_type(
                &sig, param_idx,
            )
            || crate::ir::signature_bridge::call_site_expects_owned_pass(&sig, param_idx)
            || matches!(
                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                    &sig, arg_index,
                ),
                crate::analyzer::OwnershipMode::Owned,
            );
        let wants_mut = !owned_slot
            && (self.ir_callee_arg_expects_mut_borrow(
                registry,
                callee_name,
                arg_index,
                user_arg_count,
                Some(&sig),
            ) || self.ir_sig_arg_expects_mut_borrow(&sig, arg_index));
        if (!has_ownership_collision || wants_mut)
            && wants_mut
            && !coerced.starts_with("&mut ")
            && crate::codegen::rust::expression_utilities::arg_supports_mut_borrow_coercion(
                arg_expr,
            )
        {
            // Already-emitted `&mut T` / `&T` bindings reborrow by value of the binding.
            let already_ref_binding = matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if self.identifier_binding_already_rust_ref(name)
            );
            if already_ref_binding {
                *coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(coerced)
                    .to_string();
                // Mut reborrow never needs `.clone()` on an already-`&mut` binding.
                if !self.must_preserve_auto_clone_for_reuse(arg_expr)
                    && !crate::codegen::rust::expression_helpers::is_explicit_user_clone_call(
                        arg_expr,
                    )
                {
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(coerced);
                }
            } else {
                crate::codegen::rust::expression_utilities::apply_mut_borrow_coercion(
                    arg_expr,
                    coerced,
                    &self.current_function_params,
                    &self.inferred_mut_borrowed_params,
                    true,
                );
            }
        }

        // Owned formals: strip stale `&` / `&mut`. Prefer *any* layered signature
        // candidate that confirms owned emission (defining-module refresh beats a
        // stale Borrowed stub — dogfood `policy: Policy`).
        self.peel_stale_borrow_for_multi_candidate_owned_formal(
            coerced,
            arg_expr,
            callee_name,
            arg_index,
            &sig,
            registry,
            wants_mut,
        );

        // Recursive same-fn call into owned formal emitted by this function.
        self.strip_recursive_owned_formal_stale_borrow(coerced, arg_expr, callee_name);

        // Prefer defining-module shared-text formals (`path: &str`) over stale owned stubs
        // before the IR text/collection finalize pass (regression-049).
        // Type-qualified callees never consult bare method homonyms (`log::error`).
        let mut text_sig_candidates: Vec<Option<crate::analyzer::FunctionSignature>> = self
            .global_signature_registry
            .as_ref()
            .map(|g| {
                crate::codegen::rust::signature_promotion::callee_signature_lookup_candidates(
                    g,
                    callee_name,
                )
            })
            .unwrap_or_default()
            .into_iter()
            .map(Some)
            .collect();
        text_sig_candidates.extend(
            crate::codegen::rust::signature_promotion::callee_signature_lookup_candidates(
                registry,
                callee_name,
            )
            .into_iter()
            .map(Some),
        );
        text_sig_candidates.push(Some(sig.clone()));
        let mut text_sig =
            crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature(
                text_sig_candidates,
            )
            .unwrap_or_else(|| sig.clone());
        let pidx_for_upgrade = text_sig.arg_param_index(arg_index);
        let type_qualified =
            crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                callee_name,
            );
        let text_challengers: Vec<Option<&crate::analyzer::FunctionSignature>> = if type_qualified {
            vec![
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(callee_name)),
                registry.get_signature(callee_name),
            ]
        } else {
            vec![
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(callee_name)),
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(simple)),
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.find_unique_signature_ending_with(simple)),
                registry.get_signature(callee_name),
                registry.get_signature(simple),
                registry.find_unique_signature_ending_with(simple),
            ]
        };
        for challenger in text_challengers {
            text_sig = crate::codegen::rust::signature_promotion::prefer_shared_text_ref_signature(
                Some(text_sig),
                challenger,
                pidx_for_upgrade,
            )
            .unwrap_or_else(|| sig.clone());
        }
        text_sig =
            crate::codegen::rust::signature_promotion::local_user_fn_beats_runtime_std_homonym(
                registry,
                callee_name,
                text_sig,
            );
        crate::codegen::rust::signature_promotion::restore_stdlib_collection_key_contract(
            &mut text_sig,
            Some(callee_name),
        );

        let arg_already_rust_ref = matches!(
            arg_expr,
            Expression::Identifier { name, .. }
                if self.identifier_binding_already_rust_ref(name)
                    || self.str_ref_optimized_params.contains(name.as_str())
                    || self.inferred_borrowed_params.contains(name)
        );
        let skip_borrow_finalize = matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && (coerced.ends_with(".to_string()")
            || coerced.ends_with(".to_owned()"))
            && {
                let pidx = sig.arg_param_index(arg_index);
                crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                    &sig, pidx,
                ) && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&sig, pidx)
            };
        if !skip_borrow_finalize
            && !(has_ownership_collision
                && crate::codegen::rust::call_signature_resolution::ownership_collision_blocks_autoborrow(
                    callee_name,
                )
                && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                    &sig,
                    sig.arg_param_index(arg_index),
                )
                && matches!(
                    arg_expr,
                    Expression::Identifier { .. } | Expression::Literal { .. }
                ))
        {
            // FieldAccess still finalizes under collision (regression-049 confirmed `&str`).
            crate::codegen::rust::string_utilities::finalize_borrowed_text_call_site_arg(
                Some(&text_sig),
                arg_index,
                receiver_type_name,
                arg_expr,
                coerced,
                arg_already_rust_ref,
            );
        }
        if matches!(
            arg_expr,
            Expression::FieldAccess { .. } | Expression::Index { .. }
        ) {
            let pidx = text_sig.arg_param_index(arg_index);
            *coerced = self.ensure_ref_for_owned_string_field_when_callee_expects_str(
                &Some(text_sig.clone()),
                pidx,
                arg_expr,
                coerced.clone(),
                false,
            );
            // Owned plain WJ `string` formals (trait `authenticate(email: string)`) must
            // receive field moves/clones — never force `&request.email` from stale
            // body-converged `&str` emission on the impl.
            let owned_plain_string =
                crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &text_sig, pidx,
                ) || crate::codegen::rust::call_site_borrow::plain_string_formal_passes_owned_at_call_site(
                    &text_sig, pidx,
                ) || crate::ir::signature_bridge::call_site_expects_owned_pass(&text_sig, pidx)
                    || (matches!(
                        text_sig.param_ownership.get(pidx),
                        Some(crate::analyzer::OwnershipMode::Owned)
                    ) && crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                        &text_sig, pidx,
                    ))
                    || self.global_signature_registry.as_ref().is_some_and(|g| {
                        crate::codegen::rust::call_signature_resolution::global_trait_owned_plain_string_arg(
                            g, simple, arg_index,
                        )
                    });
            if owned_plain_string && coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                *coerced = crate::codegen::rust::expression_utilities::coerce_borrowed_arg_to_owned(
                    coerced,
                );
            }
            if !owned_plain_string
                && !coerced.starts_with('&')
                && crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                    &text_sig, pidx,
                )
                && (crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                    &text_sig, pidx,
                ) || text_sig.param_types.get(pidx).is_some_and(|t| {
                    crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                        || matches!(
                            t,
                            Type::Reference(inner)
                                if crate::codegen::rust::types::is_windjammer_text_type(inner)
                        )
                }))
                && self
                    .infer_expression_type(arg_expr)
                    .as_ref()
                    .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
            {
                *coerced = format!("&{coerced}");
            }
        }

        // Re-apply IR collection-key / string-literal finalize after owned peels so
        // HashMap::get / Set::contains keep `&K` (and strip `&&` on shared bindings).
        self.finalize_ir_collection_key_arg(
            coerced,
            arg_expr,
            callee_name,
            &text_sig,
            arg_index,
            receiver_type_name,
        );
        crate::codegen::rust::string_utilities::finalize_string_literal_call_site_arg(
            Some(&text_sig),
            arg_index,
            Some(simple),
            arg_expr,
            coerced,
            receiver_type_name,
            Some(&self.enum_variant_types),
        );
        if self.is_collection_key_lookup_at_site(
            &text_sig,
            arg_index,
            crate::codegen::rust::stdlib_method_traits::collection_key_receiver_type(
                callee_name,
                receiver_type_name,
                &text_sig,
            )
            .as_deref(),
        ) && matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) {
            if coerced.ends_with(".to_string()") {
                if let Some(stripped) = coerced.strip_suffix(".to_string()") {
                    *coerced = stripped.to_string();
                }
            }
            crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                arg_expr,
                coerced,
            );
        }

        // Vec locals into `&Vec<T>` formals (signature-driven; not in apply_ir).
        *coerced =
            crate::codegen::rust::call_site_borrow::maybe_borrow_owned_vec_local_for_ref_formal(
                self,
                &text_sig,
                arg_index,
                arg_expr,
                std::mem::take(coerced),
                receiver_type_name,
                Some(simple),
                user_arg_count,
            );
        // Owned Vec/non-Copy formals: stale `&binding` from reuse must become `.clone()`
        // (ReBAC `contains_string(&out)` into `items: Vec<String>`).
        {
            let owned_pidx = text_sig.arg_param_index(arg_index);
            let callee_emits_shared = !global_or_local_confirms_owned_emission()
                && (crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                    &text_sig, owned_pidx,
                ) || global_confirms_shared_ref(owned_pidx));
            let bare_is_vec = text_sig
                .formal_param_type(owned_pidx)
                .or_else(|| text_sig.param_types.get(owned_pidx))
                .is_some_and(|t| {
                    let bare = match t {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    matches!(bare, Type::Vec(_))
                        || matches!(bare, Type::Parameterized(n, _) if n == "Vec")
                });
            let analyzer_borrows_vec = matches!(
                text_sig.param_ownership.get(owned_pidx),
                Some(
                    crate::analyzer::OwnershipMode::Borrowed
                        | crate::analyzer::OwnershipMode::MutBorrowed
                )
            );
            let owned_slot = !callee_emits_shared
                && (global_or_local_confirms_owned_emission()
                    || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        &text_sig, owned_pidx,
                    )
                    || (crate::ir::signature_bridge::call_site_expects_owned_pass(
                        &text_sig, owned_pidx,
                    ) && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                        &text_sig, owned_pidx,
                    ))
                    || (bare_is_vec
                && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                    &text_sig, owned_pidx,
                )
                // Analyzer-Borrowed / IR shared-ref Vec formals keep `&walls`
                // (cross-file `check_collisions`); bare WJ `Vec` alone is not owned.
                && !analyzer_borrows_vec
                && !crate::ir::signature_bridge::call_site_expects_shared_borrow(
                    &text_sig, owned_pidx,
                )
                && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &text_sig, owned_pidx,
                ) || matches!(
                    text_sig.param_ownership.get(owned_pidx),
                    Some(crate::analyzer::OwnershipMode::Owned)
                ) || text_sig.formal_param_type(owned_pidx).is_some_and(|t| {
                    matches!(t, Type::Vec(_))
                        || matches!(t, Type::Parameterized(n, _) if n == "Vec")
                }))));
            if coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
                && owned_slot
                && !crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                    registry,
                    callee_name,
                    Some(&text_sig),
                    arg_index,
                )
            {
                let key_receiver =
                    crate::codegen::rust::stdlib_method_traits::collection_key_receiver_type(
                        callee_name,
                        receiver_type_name,
                        &text_sig,
                    );
                let is_map_key_lookup = self.is_collection_key_lookup_at_site(
                    &text_sig,
                    arg_index,
                    key_receiver.as_deref(),
                ) || self.is_collection_key_lookup_at_site(
                    &sig,
                    arg_index,
                    key_receiver.as_deref(),
                );
                if !is_map_key_lookup {
                let base = crate::codegen::rust::expression_utilities::borrow_base_expr(coerced)
                    .to_string();
                let needs_clone = match arg_expr {
                    Expression::Identifier { name, .. } => {
                        self.auto_clone_analysis.as_ref().is_some_and(|a| {
                            a.needs_clone(name, self.current_statement_idx).is_some()
                                || (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                                    &text_sig, owned_pidx,
                                ) && a.needs_clone_anywhere(name))
                        })
                    }
                    _ => false,
                };
                *coerced = if needs_clone
                    && !matches!(
                        arg_expr,
                        Expression::Identifier { name, .. }
                            if self.binding_is_copy_pass_by_value_scalar(name)
                    ) {
                    format!("{base}.clone()")
                } else {
                    // `&Vec` into an owned/`Vec` formal is never valid; clone mut locals.
                    match arg_expr {
                        Expression::Identifier { name, .. }
                            if self.local_var_types.get(name).is_some_and(|t| {
                                matches!(t, Type::Vec(_))
                                    || matches!(t, Type::Parameterized(n, _) if n == "Vec")
                            }) =>
                        {
                            format!("{base}.clone()")
                        }
                        _ => base,
                    }
                };
                }
            }
        }

        // Terminal: owned `string` formals must not keep `&"lit".to_string()`.
        let text_param_idx = text_sig.arg_param_index(arg_index);
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && coerced.starts_with('&')
            && coerced.ends_with(".to_string()")
            && (crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                callee_name,
            ) || (crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                &text_sig, text_param_idx,
            ) && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                &text_sig, text_param_idx,
            )) || crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
                &text_sig, arg_index,
            ) || crate::ir::signature_bridge::call_site_expects_owned_pass(
                &text_sig, text_param_idx,
            ))
        {
            *coerced = coerced.trim_start_matches('&').to_string();
        }

        if coerced.ends_with(".to_string().clone()") || coerced.ends_with(".to_owned().clone()") {
            crate::codegen::rust::expression_utilities::strip_trailing_clone(coerced);
        }

        // Pattern / `&str` formals: `"lit".to_string()` / `String::from("lit")` → `"lit"`.
        let expects_str_ref = !self.preregistered_free_call_arg_emits_owned(callee_name, arg_index)
            && (crate::codegen::rust::string_utilities::method_call_arg_expects_pattern_str(
            simple,
            arg_index,
            Some(&text_sig),
            receiver_type_name,
            receiver_type_name.is_some_and(|rt| {
                crate::codegen::rust::types::is_windjammer_text_type(&Type::Custom(rt.to_string()))
                    || rt == "str"
                    || rt.ends_with("::String")
                    || rt.ends_with("::str")
            }),
            registry,
        ) || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
            &text_sig, text_param_idx,
        ) || text_sig
            .param_types
            .get(text_param_idx)
            .is_some_and(crate::codegen::rust::string_utilities::param_is_rust_str_ref)
            // Methods with Borrowed WJ `string` formals lower to Rust `&str`.
            || (text_sig.has_self_receiver_slot()
                && crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string(
                    &text_sig, text_param_idx,
                )
                && matches!(
                    text_sig.param_ownership.get(text_param_idx),
                    Some(crate::analyzer::OwnershipMode::Borrowed)
                )));
        if expects_str_ref {
            crate::codegen::rust::string_utilities::normalize_owned_string_producer_for_str_ref_param(
                arg_expr,
                coerced,
            );
        }

        self.ensure_shared_borrow_on_match_arm_readonly_text(
            coerced, arg_expr, &text_sig, arg_index,
        );

        // Runtime-std WJ-owned / Rust-borrowed slots (`json::get` `&Value`) — signature
        // registry, never module-name lists.
        let runtime_std_borrow =
            crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                registry,
                callee_name,
                Some(&text_sig),
                arg_index,
            );
        if runtime_std_borrow
            && !coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
            && !crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
            && matches!(
                arg_expr,
                Expression::Identifier { .. } | Expression::FieldAccess { .. }
            )
            && !matches!(
                arg_expr,
                Expression::Identifier { name, .. } if self.identifier_already_ref(name)
            )
        {
            crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(coerced);
        }

        // Empty/stub WJ sigs: type-qualified associated + unresolved instance builders
        // still auto-own bare string lits (signature-driven; no `new`/`from` name lists).
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && (crate::codegen::rust::string_utilities::type_qualified_associated_string_literal_needs_rust_owned_string(
            callee_name,
            arg_index,
            Some(&text_sig),
            registry,
            self.global_signature_registry.as_deref(),
        ) || crate::codegen::rust::string_utilities::unresolved_instance_method_string_literal_needs_rust_owned_string(
            simple,
            arg_index,
            Some(&text_sig),
            registry,
            self.global_signature_registry.as_deref(),
            receiver_type_name,
        )) && !crate::codegen::rust::string_utilities::already_owned_string_expr(coerced)
        {
            *coerced = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                coerced.trim_start_matches('&'),
            );
        }

        self.strip_stale_amp_on_already_ref_arg(arg_expr, coerced);

        // Mixed-forwarder / owned-outer / reuse-clone / pure-forwarding: IR coercion
        // for self-receiver calls, then shared helpers. Copy-aggregate peel runs after
        // so `&through` into owned `Lsn` is still stripped (regression-060).
        self.apply_post_ir_forwarder_owned_outer_and_reuse(
            coerced,
            arg_expr,
            callee_name,
            &sig,
            arg_index,
            receiver,
            receiver_type_name,
            is_collection_key_site,
        );

        // After shared-borrow reapply: Copy-aggregate caller → owned Copy-aggregate
        // callee must not keep stale `&` (regression-060).
        self.peel_copy_aggregate_caller_into_owned_callee(
            coerced,
            arg_expr,
            callee_name,
            &text_sig,
            arg_index,
            receiver_type_name,
            is_collection_key_site,
        );

        // Signature-driven numeric formals (usize index/capacity, int→float).
        // Use the call-site contract `sig`, not `text_sig` — suffix refresh of
        // `insert` can pick Vec::insert (usize) over HashMap::insert (K, V).
        self.apply_post_ir_numeric_formal_casts(
            coerced,
            arg_expr,
            callee_name,
            arg_index,
            &sig,
            receiver_type_name,
            receiver,
        );

        // Terminal: strip collision-blocked borrows again after text/forwarder
        // finalize may have re-applied `&` from a conflicting Borrowed snapshot.
        let collision_simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        if has_ownership_collision
            && !is_collection_key_site
            && crate::codegen::rust::call_signature_resolution::ownership_collision_blocks_autoborrow(
                collision_simple,
            )
            && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                &sig, param_idx,
            )
            && !global_confirms_shared_ref(param_idx)
            && matches!(
                arg_expr,
                Expression::Identifier { .. } | Expression::Literal { .. }
            )
        {
            crate::codegen::rust::call_signature_resolution::strip_collision_blocked_call_site_coercions(
                coerced,
            );
        }

        // Normalize `String::from("…").to_string()` left by stacked owned-literal paths.
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) && crate::codegen::rust::string_utilities::already_owned_string_expr(coerced)
            && !self.is_collection_key_lookup_at_site(
                &text_sig,
                arg_index,
                crate::codegen::rust::stdlib_method_traits::collection_key_receiver_type(
                    callee_name,
                    receiver_type_name,
                    &text_sig,
                )
                .as_deref(),
            )
        {
            *coerced = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(coerced);
        }

        // Terminal: recursive same-fn call into an owned formal emitted by *this*
        // function — strip `&` re-applied by later text/forwarder/vec finalize
        // (ReBAC `resolve_check(&policy)` into `policy: Policy`).
        self.strip_recursive_owned_formal_stale_borrow(coerced, arg_expr, callee_name);

        // Terminal: `vec[i]` into owned non-Copy / Custom formals — reconcile may strip
        // IR `.clone()` when a stale Borrowed snapshot briefly applies `&` then peels it
        // (`col_string(rows[0], …)` E0507). Reuse the same IR helper as call-site coerce.
        if matches!(arg_expr, Expression::Index { .. }) {
            *coerced = self.ensure_owned_move_clone_for_reuse(
                arg_expr,
                coerced,
                &sig,
                param_idx,
                callee_name,
                arg_index,
            );
        }

        // Terminal: borrowed `for item in vec` elems into shared-text callees — pass
        // bare `item` (Rust autoref), not `&item` (E0308 / wj-cli-args scan gate).
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.borrowed_iterator_vars.contains(name)
                && self.ir_sig_arg_expects_shared_borrow(&sig, param_idx)
                && coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
            {
                let base = crate::codegen::rust::expression_utilities::borrow_base_expr(coerced);
                if base == name.as_str() {
                    *coerced = base.to_string();
                }
            }
        }

        // Terminal: preregistered / registry-owned formals beat stale borrow reapply
        // (`display_text(color)`, `apply_patch_delete(key.clone())`).
        // Never strip forward-ref borrows inside `if` (`key_in_latest_base(&key)`).
        let forward_ref_keeps_borrow = matches!(
            arg_expr,
            Expression::Identifier { name, .. }
                if self.in_if_condition
                    && (self.current_fn_forward_ref_if_params.contains(name)
                        || self.current_fn_mixed_forwarder_params.contains(name))
        );
        if coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
            && !forward_ref_keeps_borrow
            && !is_collection_key_site
            && !self.preregistered_free_call_arg_expects_borrow(callee_name, arg_index)
            && (self.preregistered_free_call_arg_emits_owned(callee_name, arg_index)
                || Self::sig_arg_confirms_owned_emission(&sig, arg_index)
                || self.ir_callee_arg_emits_owned_contract(
                    registry,
                    callee_name,
                    arg_index,
                    user_arg_count,
                    Some(&sig),
                )
                || self.global_signature_registry.as_ref().is_some_and(|g| {
                    self.ir_callee_arg_emits_owned_contract(
                        g,
                        callee_name,
                        arg_index,
                        user_arg_count,
                        Some(&sig),
                    )
                }))
        {
            *coerced =
                crate::codegen::rust::expression_utilities::borrow_base_expr(coerced).to_string();
        }

        // Terminal: global method metadata (`WalSegment::append_put`) for vec literals
        // and helper returns when local receiver inference lagged.
        self.maybe_borrow_vec_or_helper_from_global_metadata(
            coerced,
            arg_expr,
            callee_name,
            arg_index,
            receiver_type_name,
            registry,
        );

        // WDB-174/178/180/181: cross-module registry ownership can lag defining-module
        // Rust emission (demoted caller formals, shared-ref callees). Reconcile from
        // caller binding shape + owned/shared emission contracts — signature-driven.
        self.reconcile_multipass_demoted_caller_and_shared_callee_terminal(
            coerced,
            arg_expr,
            &sig,
            param_idx,
            callee_name,
            arg_index,
            registry,
            user_arg_count,
            &global_confirms_shared_ref,
        );

        if crate::codegen::rust::call_site_borrow::user_wrote_explicit_deref(arg_expr)
            && coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
        {
            *coerced = coerced[1..].to_string();
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.local_owned_binding_shadows_formal(name)
                && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &sig, param_idx,
                )
                    || crate::codegen::rust::string_utilities::call_site_param_expects_owned_string(
                        &sig, arg_index,
                    ))
                && coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
            {
                *coerced = coerced.trim_start_matches('&').to_string();
            }
        }
        self.peel_fn_trait_or_closure_call_arg(coerced, arg_expr, callee_name, &sig, arg_index);

        // `latest.has_key(key)` — local receiver, owned callee: clone param for reuse upstream.
        if let Some(recv) = receiver {
            if !crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(recv)
                && global_or_local_confirms_owned_emission()
            {
                if let Expression::Identifier { name, .. } = arg_expr {
                    let param_non_copy = self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && !self.is_type_copy(&p.type_)
                            && !crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                    });
                    if param_non_copy
                        && !coerced.ends_with(".clone()")
                        && !coerced.starts_with('&')
                        && !coerced.starts_with("&mut ")
                    {
                        *coerced = format!("{coerced}.clone()");
                    }
                }
            }
        }

        let wants_shared_terminal = global_confirms_shared_ref(param_idx)
            || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&sig, param_idx)
            || crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(&sig, param_idx)
            || self.preregistered_free_call_arg_expects_borrow(callee_name, arg_index);
        if wants_shared_terminal && coerced.ends_with(".clone()") {
            let base = coerced.trim_end_matches(".clone()").trim();
            // Demoted caller `&T` is already shared — bare pass (P3.390).
            *coerced = if let Expression::Identifier { name, .. } = arg_expr {
                if self.emitted_rust_ref_formals.contains(name)
                    || self.caller_formal_emitted_shared_ref(name)
                    || self.identifier_already_ref(name)
                {
                    name.to_string()
                } else if base.starts_with('&') {
                    base.to_string()
                } else {
                    format!("&{base}")
                }
            } else if base.starts_with('&') {
                base.to_string()
            } else {
                format!("&{base}")
            };
        }
    }

    pub(crate) fn peel_fn_trait_or_closure_call_arg(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        _callee_name: &str,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
    ) {
        let pidx = sig.arg_param_index(arg_index);
        // Signature-/AST-driven only: never key off callee leaf names (`spawn`, …).
        // Temporary until coerce/enforce always win; delete when suite proves Identity.
        let explicit_move_closure = matches!(
            arg_expr,
            Expression::Binary {
                op: crate::parser::BinaryOp::Or,
                left,
                ..
            } if matches!(
                &**left,
                Expression::Identifier { name, .. } if name == "move"
            )
        );
        if matches!(arg_expr, Expression::Closure { .. })
            || explicit_move_closure
            || sig
                .formal_param_type(pidx)
                .or_else(|| sig.param_types.get(pidx))
                .is_some_and(
                    crate::codegen::rust::stdlib_method_traits::formal_is_rust_closure_trait,
                )
        {
            *coerced =
                crate::codegen::rust::expression_utilities::borrow_base_expr(coerced).to_string();
        }
    }

    /// Terminal multipass ownership fixes when importer registry stubs disagree with
    /// defining-module Rust emission (WDB-174/178/180/181 product gates).
    fn reconcile_multipass_demoted_caller_and_shared_callee_terminal(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        sig: &crate::analyzer::FunctionSignature,
        param_idx: usize,
        callee_name: &str,
        arg_index: usize,
        registry: &SignatureRegistry,
        user_arg_count: Option<usize>,
        global_confirms_shared_ref: &impl Fn(usize) -> bool,
    ) {
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.caller_demoted_non_copy_formal_into_owned_callee(name)
                && !coerced.ends_with(".clone()")
                && !coerced.ends_with(".to_string()")
                && !coerced.ends_with(".to_owned()")
            {
                let callee_wants_shared = global_confirms_shared_ref(param_idx)
                    || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                        sig, param_idx,
                    )
                    || crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(
                        sig, param_idx,
                    )
                    || self.ir_callee_arg_expects_shared_borrow(
                        registry,
                        callee_name,
                        arg_index,
                        user_arg_count,
                        Some(sig),
                    );
                let callee_wants_owned = !callee_wants_shared
                    && (self.ir_callee_arg_emits_owned_contract(
                        registry,
                        callee_name,
                        arg_index,
                        user_arg_count,
                        Some(sig),
                    ) || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        sig, param_idx,
                    ) || crate::codegen::rust::signature_promotion::wj_registry_bare_owned_formal_slot(
                        sig, param_idx,
                    ));
                if callee_wants_owned {
                    let caller_text = self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                    });
                    let callee_owned_text = sig.formal_param_type(param_idx).is_some_and(|t| {
                        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                            && crate::codegen::rust::types::is_windjammer_text_type(t)
                    })
                        && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                            sig, param_idx,
                        );
                    if caller_text && callee_owned_text {
                        if !crate::codegen::rust::string_utilities::already_owned_string_expr(
                            coerced,
                        ) {
                            *coerced =
                                crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                                    coerced,
                                );
                        }
                    } else {
                        let base =
                            crate::codegen::rust::expression_utilities::borrow_base_expr(coerced);
                        *coerced = format!("{base}.clone()");
                    }
                    return;
                }
            }
        }

        let callee_emits_owned =
            crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, param_idx)
                || Self::sig_arg_confirms_owned_emission(sig, arg_index)
                || self.ir_callee_arg_emits_owned_contract(
                    registry,
                    callee_name,
                    arg_index,
                    user_arg_count,
                    Some(sig),
                )
                || self.global_signature_registry.as_ref().is_some_and(|g| {
                    self.ir_callee_arg_emits_owned_contract(
                        g,
                        callee_name,
                        arg_index,
                        user_arg_count,
                        Some(sig),
                    )
                });
        // Homonym shared-ref hits must not undo a confirmed owned formal on *this* sig
        // (WDB-329: Vec::remove Owned usize vs Blackboard::remove Borrowed).
        let callee_emits_owned = callee_emits_owned
            && (!global_confirms_shared_ref(param_idx)
                || crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    sig, param_idx,
                )
                || Self::sig_arg_confirms_owned_emission(sig, arg_index));
        if callee_emits_owned {
            if coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                *coerced = crate::codegen::rust::expression_utilities::coerce_borrowed_arg_to_owned(
                    coerced,
                );
            }
            return;
        }

        let wants_shared = global_confirms_shared_ref(param_idx)
            || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(sig, param_idx)
            || crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(sig, param_idx)
            || self.preregistered_free_call_arg_expects_borrow(callee_name, arg_index);
        if wants_shared
            && !coerced.starts_with("&mut ")
            && !coerced.ends_with(".to_string()")
            && !coerced.ends_with(".to_owned()")
            && !crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
            // WDB-329: never re-borrow cast-to-usize after numeric formal peel.
            && !matches!(
                arg_expr,
                Expression::Call { .. }
                    | Expression::MethodCall { .. }
                    | Expression::Cast { .. }
            )
            && !coerced.contains(" as usize")
        {
            if coerced.ends_with(".clone()") {
                let base = coerced.trim_end_matches(".clone()").trim();
                *coerced = if base.starts_with('&') {
                    base.to_string()
                } else {
                    format!("&{base}")
                };
            } else if !coerced.starts_with('&') {
                if let Expression::Identifier { name, .. } = arg_expr {
                    if self.emitted_rust_ref_formals.contains(name)
                        || self.identifier_binding_already_rust_ref(name)
                    {
                        return;
                    }
                }
                *coerced = format!("&{coerced}");
            }
        }
    }

    /// Strip stale `&` on recursive calls into owned formals this function emits.
    fn callee_is_recursive_self_call(&self, callee_name: &str) -> bool {
        self.current_function_name.as_deref().is_some_and(|cur| {
            if callee_name == cur {
                return true;
            }
            // Forwarding wrappers (`pub fn write` → `csv.write`) are not recursion.
            if crate::codegen::rust::call_signature_resolution::qualified_callee_skips_bare_homonym_lookup(
                callee_name,
            ) {
                return false;
            }
            callee_name.rsplit("::").next().is_some_and(|s| s == cur)
        })
    }

    fn strip_recursive_owned_formal_stale_borrow(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        callee_name: &str,
    ) {
        let Expression::Identifier { name, .. } = arg_expr else {
            return;
        };
        let recursive = self.callee_is_recursive_self_call(callee_name);
        if recursive
            && self.current_function_params.iter().any(|p| p.name == *name)
            && !self.emitted_rust_ref_formals.contains(name)
            && !self.str_ref_optimized_params.contains(name.as_str())
            && coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
        {
            *coerced =
                crate::codegen::rust::expression_utilities::borrow_base_expr(coerced).to_string();
        }
    }

    /// Identifier / local already typed as `usize` — skip `as usize` / `_usize`.
    ///
    /// Do **not** treat `usize_variables` alone as emitted usize: loop-counter
    /// analysis marks `i` for comparisons (`while i < n`) while the Rust binding
    /// may still be `i64` (WDB-119). Match [`identifier_emits_as_usize`].
    pub(crate) fn arg_expression_already_usize(&self, arg: &Expression<'ast>) -> bool {
        match arg {
            Expression::Identifier { name, .. } => self.identifier_emits_as_usize(name),
            _ => self.infer_expression_type_is_usize(arg) || self.expression_produces_usize(arg),
        }
    }

    /// Runtime/stdlib fallback declares `usize` at `pidx` while the WJ stub may still say `int`.
    fn fallback_signature_param_is_usize(
        &self,
        callee_name: &str,
        simple: &str,
        pidx: usize,
    ) -> bool {
        let is_usize_slot = |sig: &crate::analyzer::FunctionSignature| {
            sig.formal_param_type(pidx)
                .or_else(|| sig.param_types.get(pidx))
                .is_some_and(crate::codegen::rust::type_casting::type_is_usize)
        };
        for reg in [
            self.global_signature_registry.as_deref(),
            Some(&self.signature_registry),
            Some(crate::analyzer::SignatureRegistry::stdlib()),
        ]
        .into_iter()
        .flatten()
        {
            for key in [callee_name, simple] {
                if reg
                    .get_fallback_signature(key)
                    .or_else(|| reg.get_signature(key))
                    .is_some_and(is_usize_slot)
                {
                    return true;
                }
            }
        }
        false
    }

    /// `usize` index/capacity and int→float casts from the resolved formal type.
    fn apply_post_ir_numeric_formal_casts(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        callee_name: &str,
        arg_index: usize,
        sig: &crate::analyzer::FunctionSignature,
        receiver_type_name: Option<&str>,
        receiver: Option<&Expression<'ast>>,
    ) {
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        // Prefer receiver-qualified formals so suffix-ambiguous methods
        // (`insert` → Vec vs HashMap) never drive numeric casts from the wrong sig.
        let user_argc = sig
            .param_types
            .len()
            .saturating_sub(usize::from(sig.has_self_receiver_slot()));
        let inferred_recv = receiver.and_then(|r| self.infer_expression_type(r));
        let mut cast_sig = receiver_type_name
            .and_then(|rt| {
                self.resolve_method_function_signature_specialized(
                    rt,
                    simple,
                    user_argc,
                    inferred_recv.as_ref(),
                )
            })
            .unwrap_or_else(|| sig.clone());
        // Stdlib generics stay `T`/`E` until specialized from the concrete receiver
        // (`Vec<usize>::push` → `usize`). Registry base names are unparameterized (`Vec`).
        if let Some(recv_ty) =
            crate::codegen::rust::stdlib_signature_specialization::receiver_type_from_name_and_hint(
                receiver_type_name,
                inferred_recv.as_ref(),
                self.current_function_return_type.as_ref(),
            )
        {
            crate::codegen::rust::stdlib_signature_specialization::specialize_signature_for_receiver(
                &mut cast_sig,
                &recv_ty,
            );
        }
        let pidx = cast_sig.arg_param_index(arg_index);
        let formal = cast_sig
            .formal_param_type(pidx)
            .or_else(|| cast_sig.param_types.get(pidx));
        let mut fallback_usize_formal = None;
        let collection_elem_usize = inferred_recv
            .as_ref()
            .and_then(|rty| Self::peeled_collection_element_type(rty))
            .filter(|elem| crate::codegen::rust::type_casting::type_is_usize(elem))
            .cloned();
        let receiver_is_map = inferred_recv.as_ref().is_some_and(|rty| match rty {
            Type::Parameterized(name, _) | Type::Custom(name) => {
                crate::type_classification::is_map_type_name(name)
            }
            _ => false,
        }) || receiver_type_name
            .is_some_and(crate::type_classification::is_map_type_name);
        let map_insert_key_slot = simple == "insert"
            && arg_index == 0
            && receiver_is_map
            && formal.is_some_and(|t| {
                matches!(t, Type::Generic(n) if n == "K" || n == "V")
                    || crate::codegen::rust::types::is_windjammer_text_type(t)
            });
        let formal_for_usize =
            if formal.is_some_and(crate::codegen::rust::type_casting::type_is_usize) {
                formal
            } else if formal.is_some_and(
                |t| matches!(t, Type::Custom(n) | Type::Generic(n) if n == "T" || n == "E"),
            ) && collection_elem_usize.is_some()
            {
                // Unspecialized store formal + concrete `Vec<usize>` / similar receiver.
                fallback_usize_formal = collection_elem_usize;
                fallback_usize_formal.as_ref()
            } else if formal.is_none_or(|t| {
                // Runtime fallback may declare `usize` while WJ stubs still say `int`.
                // Never override a concrete non-int formal (WDB-133: `drain(DenseCsr {..})`
                // must not become `(DenseCsr {..}) as usize` via a colliding fallback key).
                crate::codegen::rust::type_casting::type_is_wj_int_formal(t)
                    || matches!(
                        t,
                        Type::Custom(n) | Type::Generic(n)
                            if n == "T" || n == "E" || n == "K" || n == "V" || n == "int"
                    )
            }) && !map_insert_key_slot
                && self.fallback_signature_param_is_usize(callee_name, simple, pidx)
            {
                fallback_usize_formal = Some(Type::Custom("usize".to_string()));
                fallback_usize_formal.as_ref()
            } else if formal.is_some_and(crate::codegen::rust::type_casting::type_is_wj_int_formal)
                && !map_insert_key_slot
                && self.fallback_signature_param_is_usize(callee_name, simple, pidx)
            {
                fallback_usize_formal = Some(Type::Custom("usize".to_string()));
                fallback_usize_formal.as_ref()
            } else {
                formal
            };
        let already_usize = if formal_for_usize.is_some() {
            let expr_is_wj_int = match arg_expr {
                Expression::Identifier { name, .. } => self
                    .current_function_params
                    .iter()
                    .find(|p| p.name == *name)
                    .map(|p| &p.type_)
                    .or_else(|| self.local_var_types.get(name))
                    .is_some_and(|t| crate::codegen::rust::type_casting::type_is_wj_int_formal(t)),
                _ => self
                    .infer_expression_type(arg_expr)
                    .as_ref()
                    .is_some_and(crate::codegen::rust::type_casting::type_is_wj_int_formal),
            };
            if expr_is_wj_int {
                false
            } else {
                self.arg_expression_already_usize(arg_expr)
            }
        } else {
            self.arg_expression_already_usize(arg_expr)
        };
        crate::codegen::rust::type_casting::coerce_arg_str_for_usize_formal(
            Some(self),
            arg_expr,
            coerced,
            formal_for_usize,
            already_usize,
        );
        // WDB-160: runtime `process::exit(i32)` — cast WJ `int`/`i64` args.
        let arg_ty = self.infer_expression_type(arg_expr);
        let mixed_int = Some(self.int_type_for_mixed_int_codegen(arg_expr));
        crate::codegen::rust::type_casting::coerce_arg_str_for_i32_formal(
            arg_expr,
            coerced,
            formal.or(formal_for_usize),
            arg_ty.as_ref(),
            mixed_int,
        );
        crate::codegen::rust::type_casting::coerce_arg_str_for_i64_formal(
            arg_expr,
            coerced,
            formal.or(formal_for_usize),
            arg_ty.as_ref(),
            mixed_int,
        );
        crate::codegen::rust::type_casting::coerce_arg_str_for_u32_formal(
            arg_expr,
            coerced,
            formal.or(formal_for_usize),
            arg_ty.as_ref(),
            mixed_int,
        );
        // Numeric inference may have already emitted `1_usize` from a Vec::insert
        // suffix match; undo when the *effective* formal is not usize (after
        // specialization / collection-element recovery — not raw `T`).
        crate::codegen::rust::type_casting::strip_erroneous_usize_suffix_for_non_usize_formal(
            Some(self),
            arg_expr,
            coerced,
            formal_for_usize.or(formal),
        );

        let skip_cast = self.should_skip_int_to_float_auto_cast_with_global(
            receiver_type_name,
            simple,
            Some(callee_name),
        );
        if skip_cast {
            return;
        }
        // Integer receivers (`i32.max(-100).min(100)`) must not cast bounds to f32 when
        // only float `min`/`max` stubs exist in the registry.
        if receiver_type_name.is_some_and(crate::type_classification::is_integer_type)
            || inferred_recv
                .as_ref()
                .is_some_and(crate::codegen::rust::type_casting::type_is_wj_int_formal)
        {
            return;
        }
        let Some(param_ty) = cast_sig
            .param_type_for_arg(arg_index)
            .or_else(|| cast_sig.formal_param_type(pidx))
            .or_else(|| cast_sig.param_types.get(pidx))
        else {
            return;
        };
        let arg_ty = self.infer_expression_type(arg_expr);
        crate::codegen::rust::type_classification_utilities::maybe_cast_int_arg_to_float(
            coerced,
            arg_expr,
            param_ty,
            arg_ty.as_ref(),
        );
    }

    /// Multi-candidate owned-formal peel: any defining-module / global / local
    /// signature that confirms owned emission wins over stale Borrowed stubs.
    /// Signature-driven only — no method-name ownership heuristics.
    fn peel_owned_borrow_to_move_or_clone(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
    ) {
        let base =
            crate::codegen::rust::expression_utilities::borrow_base_expr(coerced).to_string();
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.local_binding_reused_after_current_statement(name)
                && !self.binding_is_copy_pass_by_value_scalar(name)
                && !base.ends_with(".clone()")
            {
                *coerced = format!("{base}.clone()");
                return;
            }
        }
        *coerced = base;
    }

    pub(crate) fn peel_stale_borrow_for_multi_candidate_owned_formal(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        callee_name: &str,
        arg_index: usize,
        primary_sig: &crate::analyzer::FunctionSignature,
        registry: &SignatureRegistry,
        wants_mut: bool,
    ) {
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let receiver_base = callee_name.rsplit_once("::").map(|(ty, _)| {
            ty.rsplit("::")
                .next()
                .unwrap_or(ty)
                .split('<')
                .next()
                .unwrap_or(ty)
        });
        let type_qualified = callee_name.contains("::");
        let refreshed =
            self.refreshed_call_site_sig_for_arg(registry, callee_name, arg_index, primary_sig);
        // Type-qualified map/set lookups (`HashMap::get`) must not consult bare method
        // homonyms (`NoteStore::get`) when deciding owned vs shared-ref peel.
        if type_qualified {
            if let Some(base) = receiver_base {
                if (crate::type_classification::is_map_type_name(base)
                    || crate::type_classification::is_set_type_name(base))
                    && (self.is_collection_key_lookup_at_site(&refreshed, arg_index, Some(base))
                        || self.is_collection_key_lookup_at_site(
                            primary_sig,
                            arg_index,
                            Some(base),
                        ))
                {
                    return;
                }
            }
        }
        let homonym_candidate = if type_qualified {
            None
        } else {
            registry.find_unique_signature_ending_with(simple)
        };
        let global_homonym = if type_qualified {
            None
        } else {
            self.global_signature_registry
                .as_ref()
                .and_then(|g| g.find_unique_signature_ending_with(simple))
        };
        let method_resolved = callee_name.rsplit_once("::").and_then(|(rt, method)| {
            self.resolve_method_function_signature(rt, method, arg_index + 1)
        });
        let candidates: [Option<&crate::analyzer::FunctionSignature>; 9] = [
            registry.get_signature(callee_name),
            if type_qualified {
                None
            } else {
                registry.get_signature(simple)
            },
            homonym_candidate,
            self.global_signature_registry
                .as_ref()
                .and_then(|g| g.get_signature(callee_name)),
            if type_qualified {
                None
            } else {
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(simple))
            },
            global_homonym,
            method_resolved.as_ref(),
            Some(primary_sig),
            Some(&refreshed),
        ];
        if self.preregistered_free_call_arg_emits_owned(callee_name, arg_index)
            && (coerced.starts_with('&') || coerced.starts_with("&mut "))
            && !self.preregistered_free_call_arg_expects_borrow(callee_name, arg_index)
        {
            self.peel_owned_borrow_to_move_or_clone(coerced, arg_expr);
            return;
        }
        let mut_arg_emitted = self
            .function_emitted_mut_arg_indices
            .get(callee_name)
            .or_else(|| self.function_emitted_mut_arg_indices.get(simple))
            .is_some_and(|indices| indices.contains(&arg_index));
        let any_emitted_owned = candidates
            .iter()
            .flatten()
            .any(|sig| Self::sig_arg_confirms_owned_emission(sig, arg_index));
        let any_emits_shared = candidates.iter().flatten().any(|sig| {
            let pidx = sig.arg_param_index(arg_index);
            crate::ir::emission_contract::callee_emits_shared_rust_ref_param(sig, pidx)
        });
        let any_expects_owned = candidates.iter().flatten().any(|sig| {
            let pidx = sig.arg_param_index(arg_index);
            crate::ir::signature_bridge::call_site_expects_owned_pass(sig, pidx)
        });
        let any_expects_mut = wants_mut
            || mut_arg_emitted
            || candidates.iter().flatten().any(|sig| {
                let pidx = sig.arg_param_index(arg_index);
                matches!(
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                        sig, arg_index,
                    ),
                    crate::analyzer::OwnershipMode::MutBorrowed,
                ) || sig.param_types.get(pidx).is_some_and(|t| {
                    matches!(t, Type::MutableReference(_))
                }) || sig.formal_param_type(pidx).is_some_and(|t| {
                    matches!(t, Type::MutableReference(_))
                })
            });
        let peel_owned = (any_emitted_owned || (!any_emits_shared && any_expects_owned))
            && !crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                registry,
                callee_name,
                Some(primary_sig),
                arg_index,
            );
        // Prefer refreshed / primary for shared-ref confirmation — owned emission among
        // other layered stubs must not peel a confirmed `&str` / `&T` formal.
        // Any candidate with codegen-owned emission beats stale shared-ref stubs
        // (`MemoryEngine::put(key)` not `&key.clone()`).
        let confirmed_shared_ref = !any_emitted_owned && {
            let rpidx = refreshed.arg_param_index(arg_index);
            let ppidx = primary_sig.arg_param_index(arg_index);
            crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&refreshed, rpidx)
                || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                    primary_sig,
                    ppidx,
                )
        };
        // Owned emission wins over stale Borrowed/MutBorrowed stubs (HTTP `to_response(reply)`).
        // Only keep `&mut` when *this* codegen recorded a mut formal slot — not when a
        // layered analysis candidate still says MutBorrowed after field-forward restore.
        if peel_owned && !confirmed_shared_ref {
            if coerced.starts_with("&mut ")
                && (any_emitted_owned || any_expects_owned)
                && !mut_arg_emitted
            {
                self.peel_owned_borrow_to_move_or_clone(coerced, arg_expr);
            } else if !any_expects_mut
                && coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
                && (any_emitted_owned || any_expects_owned)
            {
                self.peel_owned_borrow_to_move_or_clone(coerced, arg_expr);
            }
        }
    }

    /// Prefer-shared enforce that preserves IR-confirmed `&` (bug_e0308).
    ///
    /// Stale analyzer stubs must not strip collision-aware shared borrows when
    /// any registry view emits shared-ref for the slot.
    pub(crate) fn enforce_ir_ownership_preserving_confirmed_shared_ref(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        callee_name: &str,
        arg_index: usize,
        sig: &crate::analyzer::FunctionSignature,
        registry: &SignatureRegistry,
    ) {
        // Import aliases: never let bare homonym metadata (`query_get`) override `dep::fn`.
        if self.import_fn_alias_map.contains_key(callee_name) {
            let lookup = self.signature_lookup_callee_name(callee_name);
            let enforce_sig = self
                .global_signature_registry
                .as_ref()
                .and_then(|g| g.get_signature(lookup.as_ref()).cloned())
                .unwrap_or_else(|| sig.clone());
            let pidx = enforce_sig.arg_param_index(arg_index);
            self.enforce_call_site_ownership_contract(
                coerced,
                arg_expr,
                &enforce_sig,
                pidx,
                callee_name,
                arg_index,
            );
            return;
        }
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let pidx = sig.arg_param_index(arg_index);
        let skip_bare_homonym =
            crate::codegen::rust::call_signature_resolution::qualified_callee_skips_bare_homonym_lookup(
                callee_name,
            );
        let key_receiver = crate::codegen::rust::stdlib_method_traits::collection_key_receiver_type(
            callee_name,
            None,
            sig,
        );
        let is_collection_key =
            self.is_collection_key_lookup_at_site(sig, arg_index, key_receiver.as_deref());
        // Existing `&T` / `&mut T` bindings coerce to shared `&T` — never keep stacked `&`
        // (`run_dense(&csr)` when `csr: &mut DenseCsr` and callee emits `&DenseCsr`).
        if matches!(
            arg_expr,
            Expression::Identifier { name, .. }
                if self.identifier_binding_already_rust_ref(name)
        ) {
            let enforce_sig =
                crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature({
                    let mut candidates = vec![
                        self.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(callee_name).cloned()),
                        Some(sig.clone()),
                        registry.get_signature(callee_name).cloned(),
                    ];
                    if !skip_bare_homonym {
                        candidates.insert(
                            1,
                            self.global_signature_registry
                                .as_ref()
                                .and_then(|g| g.get_signature(simple).cloned()),
                        );
                        candidates.push(registry.get_signature(simple).cloned());
                    }
                    candidates
                })
                .unwrap_or_else(|| sig.clone());
            let epidx = enforce_sig.arg_param_index(arg_index);
            if crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(&enforce_sig, epidx)
            {
                *coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(coerced)
                    .to_string();
                return;
            }
        }
        let mut enforce_sig =
            crate::codegen::rust::signature_promotion::pick_codegen_refreshed_signature({
                let mut candidates = vec![
                    self.global_signature_registry
                        .as_ref()
                        .and_then(|g| g.get_signature(callee_name).cloned()),
                    Some(sig.clone()),
                    registry.get_signature(callee_name).cloned(),
                ];
                if !skip_bare_homonym {
                    candidates.insert(
                        1,
                        self.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(simple).cloned()),
                    );
                    candidates.push(registry.get_signature(simple).cloned());
                }
                candidates
            })
            .unwrap_or_else(|| sig.clone());
        enforce_sig = crate::codegen::rust::signature_promotion::prefer_shared_text_ref_signature(
            Some(enforce_sig),
            self.global_signature_registry
                .as_ref()
                .and_then(|g| g.get_signature(callee_name))
                .or_else(|| {
                    if skip_bare_homonym {
                        None
                    } else {
                        self.global_signature_registry
                            .as_ref()
                            .and_then(|g| g.get_signature(simple))
                    }
                }),
            pidx,
        )
        .unwrap_or_else(|| sig.clone());
        let pidx = enforce_sig.arg_param_index(arg_index);
        let global_confirms_shared = self.global_signature_registry.as_ref().is_some_and(|g| {
            let keys: Vec<&str> = if skip_bare_homonym {
                vec![callee_name]
            } else {
                vec![callee_name, simple]
            };
            keys.into_iter().any(|key| {
                g.lookup_method(key).is_some_and(|gs| {
                    let gp = gs.arg_param_index(arg_index);
                    crate::ir::emission_contract::callee_emits_shared_rust_ref_param(gs, gp)
                })
            })
        });
        let formal_is_closure = enforce_sig
            .formal_param_type(pidx)
            .or_else(|| enforce_sig.param_types.get(pidx))
            .is_some_and(crate::codegen::rust::stdlib_method_traits::formal_is_rust_closure_trait);
        let keep_shared_ref = !matches!(arg_expr, Expression::Closure { .. })
            && !formal_is_closure
            && coerced.starts_with('&')
            && (is_collection_key
                || global_confirms_shared
                || (!crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &enforce_sig,
                    pidx,
                ) && !crate::codegen::rust::signature_promotion::bare_formal_is_vec_or_map(
                    &enforce_sig,
                    pidx,
                )
                    && !crate::codegen::rust::signature_promotion::bare_formal_is_owned_user_type(
                        &enforce_sig,
                        pidx,
                    )
                    && crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(
                        &enforce_sig,
                        pidx,
                    )));
        if !keep_shared_ref {
            self.enforce_call_site_ownership_contract(
                coerced,
                arg_expr,
                &enforce_sig,
                pidx,
                callee_name,
                arg_index,
            );
        }
    }

    /// Peel `&mut` when the formal is an owned Copy aggregate (not a true `&mut T` slot).
    fn peel_spurious_mut_borrow_on_owned_copy_aggregate(
        &self,
        coerced: &mut String,
        callee_name: &str,
        arg_index: usize,
        sig: &crate::analyzer::FunctionSignature,
    ) {
        if !coerced.starts_with("&mut ") {
            return;
        }
        let idx = sig.arg_param_index(arg_index);
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let mut_arg_emitted = self
            .function_emitted_mut_arg_indices
            .get(callee_name)
            .or_else(|| self.function_emitted_mut_arg_indices.get(simple))
            .is_some_and(|indices| indices.contains(&arg_index));
        if mut_arg_emitted {
            return;
        }
        let Some(formal) = sig.formal_param_type(idx) else {
            return;
        };
        let bare = match formal {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            other => other,
        };
        if crate::codegen::rust::type_analysis_pure::is_copy_type(bare)
            && !crate::type_classification::is_copy_pass_by_value_formal(bare)
            && !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
            && !matches!(sig.param_types.get(idx), Some(Type::MutableReference(_)))
            && !matches!(
                crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                    sig, idx,
                ),
                crate::analyzer::OwnershipMode::MutBorrowed,
            )
        {
            *coerced =
                crate::codegen::rust::expression_utilities::borrow_base_expr(coerced).to_string();
        }
    }

    /// Match-arm bindings into shared-ref text / `&str` formals keep a shared borrow
    /// (no `.clone()`, no owned pass). Owned `string` formals must move — do not
    /// treat bare `Type::String` as readonly (multipass match-Ok → owned helper).
    fn ensure_shared_borrow_on_match_arm_readonly_text(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
    ) {
        let Expression::Identifier { name, .. } = arg_expr else {
            return;
        };
        if !self.match_arm_bindings.contains(name.as_str()) {
            return;
        }
        let pidx = sig.arg_param_index(arg_index);
        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx) {
            return;
        }
        let wants_shared = crate::ir::signature_bridge::call_site_wants_shared_text_ref(sig, pidx)
            || sig
                .param_types
                .get(pidx)
                .is_some_and(|t| matches!(t, Type::Reference(_)));
        if !wants_shared {
            return;
        }
        if coerced.ends_with(".clone()") {
            crate::codegen::rust::expression_utilities::strip_trailing_clone(coerced);
        }
        if !coerced.starts_with('&') {
            *coerced = format!("&{coerced}");
        }
    }

    /// Signature-driven `(wants_shared_ref, wants_owned)` for a call-site slot.
    /// Owned Copy-aggregate emission beats stale IR Ref (regression-060).
    /// Runtime-std baseline borrow (`json::is_array` `&Value`) beats WJ Owned stubs.
    pub(crate) fn call_site_slot_wants_ref_and_owned(
        &self,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
    ) -> (bool, bool) {
        self.call_site_slot_wants_ref_and_owned_for(sig, arg_index, &sig.name)
    }

    pub(crate) fn call_site_slot_wants_ref_and_owned_for(
        &self,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
        callee_name: &str,
    ) -> (bool, bool) {
        let slot_sig = self.refreshed_call_site_sig_for_arg(
            &self.signature_registry,
            callee_name,
            arg_index,
            sig,
        );
        let pidx = slot_sig.arg_param_index(arg_index);
        if crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
            &self.signature_registry,
            callee_name,
            Some(&slot_sig),
            arg_index,
        ) {
            return (true, false);
        }
        let wants_owned =
            crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(&slot_sig, pidx)
                || crate::codegen::rust::call_site_borrow::sig_formal_is_copy_aggregate_owned(
                    &slot_sig,
                    pidx,
                    |t| self.is_type_copy(t),
                );
        let wants_ref = !wants_owned
            && crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(&slot_sig, pidx);
        (wants_ref, wants_owned)
    }

    /// Mixed-forwarder / owned-outer / reuse-clone / pure-forwarding strip.
    ///
    /// Uses IR `compute_coercion` for self-receiver calls, then the shared forwarder
    /// helpers. `receiver` is the method object (`None` for free-function calls).
    pub(crate) fn apply_post_ir_forwarder_owned_outer_and_reuse(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        callee_name: &str,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
        receiver: Option<&Expression<'ast>>,
        receiver_type_name: Option<&str>,
        is_collection_key_site: bool,
    ) {
        let slot_sig = if let (Some(rt), Some(recv)) = (receiver_type_name, receiver) {
            if crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(recv) {
                let method = callee_name.rsplit("::").next().unwrap_or(callee_name);
                let qualified = format!("{rt}::{method}");
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(&qualified).cloned())
                    .unwrap_or_else(|| sig.clone())
            } else {
                sig.clone()
            }
        } else {
            sig.clone()
        };
        let (wants_ref, wants_owned) =
            self.call_site_slot_wants_ref_and_owned_for(&slot_sig, arg_index, callee_name);
        let pidx = slot_sig.arg_param_index(arg_index);
        let is_collection_key = is_collection_key_site || {
            let key_receiver =
                crate::codegen::rust::stdlib_method_traits::collection_key_receiver_type(
                    callee_name,
                    receiver_type_name,
                    sig,
                );
            self.is_collection_key_lookup_at_site(sig, arg_index, key_receiver.as_deref())
        };

        if let (Some(object), Expression::Identifier { name, .. }) = (receiver, arg_expr) {
            let receiver_is_self =
                crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(object);
            let caller_owned_param = self
                .current_function_params
                .iter()
                .any(|p| p.name == *name && !self.emitted_rust_ref_formals.contains(name));
            let is_mixed_forwarder = self.current_fn_mixed_forwarder_params.contains(name);
            // `self.notes.get(id)` inside `fn get` — field receiver + homonym method name
            // must not run same-fn forwarder peel (strip `&id` meant for HashMap::get).
            if receiver_is_self && (caller_owned_param || is_mixed_forwarder) && !is_collection_key
            {
                let body: Vec<_> = self.current_function_body.iter().copied().collect();
                let if_facade_param =
                    self.param_used_in_if_with_condition_and_branches(&body, name);
                let mixed_forward_ref =
                    (is_mixed_forwarder || if_facade_param) && self.in_if_condition;
                let actual = self.infer_call_arg_actual_safety_type(arg_expr, coerced.as_str());
                let expected =
                    crate::ir::signature_bridge::safety_type_from_signature_param(&slot_sig, pidx);
                let kind = crate::ir::coercion::compute_coercion(&actual, &expected);
                let forward_ref_borrow_owned_outer =
                    wants_ref && !wants_owned && self.caller_owned_non_copy_formal(name);
                if mixed_forward_ref
                    || forward_ref_borrow_owned_outer
                    || (matches!(
                        kind,
                        crate::ir::coercion::CoercionKind::Borrow
                            | crate::ir::coercion::CoercionKind::MutBorrow
                    ) && !self.caller_owned_non_copy_formal(name)
                        && !wants_owned)
                {
                    if coerced.ends_with(".clone()") {
                        let base = coerced.trim_end_matches(".clone()").trim();
                        *coerced = format!("&{base}");
                    } else if !coerced.starts_with('&') {
                        *coerced = format!("&{coerced}");
                    }
                } else if matches!(kind, crate::ir::coercion::CoercionKind::Identity)
                    && coerced.starts_with('&')
                    && !coerced.starts_with("&mut ")
                    && self.caller_owned_non_copy_formal(name)
                {
                    *coerced = coerced.trim_start_matches('&').to_string();
                } else if wants_owned
                    && !wants_ref
                    && !mixed_forward_ref
                    && !self.current_fn_forward_ref_if_params.contains(name)
                    && coerced.starts_with('&')
                    && !coerced.starts_with("&mut ")
                {
                    let base = coerced.trim_start_matches('&');
                    let copy_aggregate = self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && self.is_type_copy(&p.type_)
                            && !crate::type_classification::is_copy_pass_by_value_formal(&p.type_)
                    });
                    let needs_reuse = self
                        .auto_clone_analysis
                        .as_ref()
                        .is_some_and(|a| a.needs_clone(name, self.current_statement_idx).is_some());
                    *coerced = if copy_aggregate || base.ends_with(".clone()") {
                        base.trim_end_matches(".clone()").trim().to_string()
                    } else if needs_reuse {
                        format!("{base}.clone()")
                    } else {
                        base.trim().to_string()
                    };
                }
            }
            if !is_collection_key {
                self.apply_forward_ref_and_mixed_forwarder_call_coercion(
                    coerced,
                    arg_expr,
                    Some(object),
                    wants_ref,
                    wants_owned,
                );
            }
        }

        if !is_collection_key {
            self.finalize_owned_outer_formal_call_arg(coerced, arg_expr, wants_ref, wants_owned);
        }

        let callee_owned_text_slot = self.callee_arg_emits_owned_contract(callee_name, arg_index);

        if !wants_ref && !is_collection_key {
            if let Expression::Identifier { name, .. } = arg_expr {
                if self.caller_demoted_non_copy_formal_into_owned_callee(name)
                    && self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                    })
                    && callee_owned_text_slot
                    && !coerced.ends_with(".to_string()")
                {
                    *coerced = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                        crate::codegen::rust::expression_utilities::borrow_base_expr(coerced),
                    );
                }
            } else if crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
                && callee_owned_text_slot
                && !coerced.ends_with(".to_string()")
                && !self.ir_callee_arg_expects_shared_borrow(
                    &self.signature_registry,
                    callee_name,
                    arg_index,
                    None,
                    Some(sig),
                )
            {
                *coerced = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                    crate::codegen::rust::expression_utilities::borrow_base_expr(coerced),
                );
            }
        }

        if wants_owned
            && !wants_ref
            && !is_collection_key
            && crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
            && callee_owned_text_slot
            && !coerced.ends_with(".to_string()")
            && !self.ir_callee_arg_expects_shared_borrow(
                &self.signature_registry,
                callee_name,
                arg_index,
                None,
                Some(sig),
            )
        {
            *coerced = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                crate::codegen::rust::expression_utilities::borrow_base_expr(coerced),
            );
        }

        if wants_owned && !wants_ref && !is_collection_key && !coerced.ends_with(".clone()") {
            if let Expression::Identifier { name, .. } = arg_expr {
                if self.caller_demoted_non_copy_formal_into_owned_callee(name)
                    && self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                    })
                    && !coerced.ends_with(".to_string()")
                    && callee_owned_text_slot
                {
                    *coerced = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                        crate::codegen::rust::expression_utilities::borrow_base_expr(coerced),
                    );
                } else if self.for_loop_borrow_needed.contains(name) {
                    let base =
                        crate::codegen::rust::expression_utilities::borrow_base_expr(coerced);
                    if !base.starts_with('&') {
                        *coerced = format!("&{base}");
                    }
                // `&Copy` loop elems already owned via `*binding` — never append `.clone()`
                // (`*post.clone()` is E0614: clone autoderefs to i64).
                } else if self.borrowed_iterator_vars.contains(name)
                    && !coerced.starts_with('*')
                    && !self.binding_is_copy_pass_by_value_scalar(name)
                    && !crate::codegen::rust::types::return_type_is_vec_of_shared_refs(
                        self.current_function_return_type.as_ref(),
                    )
                {
                    *coerced = format!("{coerced}.clone()");
                } else if !self.match_arm_bindings.contains(name.as_str()) {
                    let needs_reuse_clone = self
                        .auto_clone_analysis
                        .as_ref()
                        .is_some_and(|a| a.needs_clone(name, self.current_statement_idx).is_some());
                    let skip_self = receiver.is_some_and(|object| {
                        crate::codegen::rust::expression_helpers::method_receiver_is_self_or_field(
                            object,
                        )
                    });
                    if needs_reuse_clone && self.caller_owned_non_copy_formal(name) && !skip_self {
                        if wants_ref
                            && !wants_owned
                            && !coerced.starts_with('&')
                            && !coerced.starts_with("&mut ")
                        {
                            *coerced = format!("&{coerced}");
                        } else {
                            *coerced = format!("{coerced}.clone()");
                        }
                    } else if self.caller_demoted_non_copy_formal_into_owned_callee(name)
                        && !coerced.ends_with(".clone()")
                        && !coerced.ends_with(".to_string()")
                        && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                            sig, pidx,
                        )
                        && !crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(
                            sig, pidx,
                        )
                        && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            sig, pidx,
                        ) || crate::codegen::rust::signature_promotion::wj_registry_bare_owned_formal_slot(
                            sig, pidx,
                        ))
                    {
                        // WDB-174/175 / WDB-191: demoted caller formal into owned callee.
                        let is_text = self.current_function_params.iter().any(|p| {
                            p.name == *name
                                && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                        });
                        if is_text {
                            *coerced = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                                crate::codegen::rust::expression_utilities::borrow_base_expr(coerced),
                            );
                        } else {
                            let base =
                                crate::codegen::rust::expression_utilities::borrow_base_expr(coerced);
                            *coerced = format!("{base}.clone()");
                        }
                    }
                }
            }
        }

        if self.in_if_condition
            && wants_owned
            && !wants_ref
            && !coerced.ends_with(".clone()")
            && !coerced.starts_with('&')
        {
            if let Expression::Identifier { name, .. } = arg_expr {
                let body: Vec<_> = self.current_function_body.iter().copied().collect();
                let if_facade_forward_ref = self.current_fn_forward_ref_if_params.contains(name)
                    && self.param_used_in_if_with_condition_and_branches(&body, name);
                if if_facade_forward_ref && self.caller_owned_non_copy_formal(name) {
                    *coerced = format!("&{coerced}");
                } else if !if_facade_forward_ref && self.caller_owned_non_copy_formal(name) {
                    *coerced = format!("{coerced}.clone()");
                } else if self.caller_demoted_non_copy_formal_into_owned_callee(name)
                    && !coerced.ends_with(".to_string()")
                    && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                        sig, pidx,
                    )
                    && !crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(sig, pidx)
                    && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        sig, pidx,
                    ) || crate::codegen::rust::signature_promotion::wj_registry_bare_owned_formal_slot(
                        sig, pidx,
                    ))
                {
                    let is_text = self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                    });
                    if is_text {
                        *coerced = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                            crate::codegen::rust::expression_utilities::borrow_base_expr(coerced),
                        );
                    } else {
                        *coerced = format!("{coerced}.clone()");
                    }
                }
            }
        }

        if wants_ref && !wants_owned && matches!(arg_expr, Expression::Identifier { .. }) {
            if coerced.ends_with(".clone()") {
                let base = coerced.trim_end_matches(".clone()").trim();
                *coerced = if base.starts_with('&') {
                    base.to_string()
                } else {
                    format!("&{base}")
                };
            } else if !coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                *coerced = format!("&{coerced}");
            }
        }

        if !is_collection_key {
            self.maybe_pure_forwarding_strip_call_arg(
                coerced,
                arg_expr,
                None,
                None,
                Some(arg_index),
                None,
                Some(sig),
            );
        }
        crate::codegen::rust::expression_utilities::collapse_redundant_clones(coerced);
    }

    /// Peel `&` / `(&x)` when a Copy-aggregate caller binding is passed into an
    /// owned Copy-aggregate formal (regression-060 `through: Lsn` → `other: Lsn`).
    ///
    /// Collection-key slots keep `&K`. Signature-driven — no method-name lists.
    pub(crate) fn peel_copy_aggregate_caller_into_owned_callee(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        callee_name: &str,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
        receiver_type_name: Option<&str>,
        is_collection_key_site: bool,
    ) {
        let Expression::Identifier { name, .. } = arg_expr else {
            return;
        };
        let trimmed = coerced.trim();
        if trimmed.starts_with("&mut ") {
            return;
        }
        if !trimmed.starts_with('&') && !(trimmed.starts_with('(') && trimmed.contains('&')) {
            return;
        }
        if is_collection_key_site
            || self.is_collection_key_lookup_at_site(
                sig,
                arg_index,
                crate::codegen::rust::stdlib_method_traits::collection_key_receiver_type(
                    callee_name,
                    receiver_type_name,
                    sig,
                )
                .as_deref(),
            )
        {
            return;
        }
        let caller_copy = self.current_function_params.iter().any(|p| {
            p.name == *name
                && crate::codegen::rust::call_site_borrow::bare_type_is_copy_aggregate_owned_formal(
                    &p.type_,
                    |ty| self.is_type_copy(ty),
                )
        });
        if !caller_copy {
            return;
        }
        let param_idx = sig.arg_param_index(arg_index);
        if crate::ir::emission_contract::callee_emits_shared_rust_ref_param(sig, param_idx) {
            return;
        }
        let callee_copy = crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
            sig, param_idx,
        ) || crate::codegen::rust::call_site_borrow::sig_formal_is_copy_aggregate_owned(
            sig,
            param_idx,
            |ty| self.is_type_copy(ty),
        ) || sig
            .formal_param_type(param_idx)
            .or_else(|| sig.param_types.get(param_idx))
            .is_some_and(|t| {
                let bare = match t {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                crate::codegen::rust::call_site_borrow::bare_type_is_copy_aggregate_owned_formal(
                    bare,
                    |ty| self.is_type_copy(ty),
                )
            });
        if !callee_copy {
            return;
        }
        let mut s = coerced.trim().to_string();
        loop {
            if s.starts_with('(') && s.ends_with(')') {
                let inner = s[1..s.len() - 1].trim().to_string();
                if inner.starts_with('&') || inner.starts_with("&mut ") {
                    s = inner;
                    continue;
                }
            }
            if s.starts_with("&mut ") {
                break;
            }
            if s.starts_with('&') {
                s = s[1..].trim().to_string();
                continue;
            }
            break;
        }
        *coerced = s;
    }

    /// Uses `emitted_owned_arg_contract` to handle stale analyzer metadata.
    pub(crate) fn enforce_call_site_ownership_contract(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        sig: &crate::analyzer::FunctionSignature,
        param_idx: usize,
        callee_name: &str,
        arg_index: usize,
    ) {
        if self.preregistered_free_call_arg_expects_borrow(callee_name, arg_index)
            && !self.preregistered_free_call_arg_emits_owned(callee_name, arg_index)
            && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                sig, param_idx,
            )
            && !matches!(arg_expr, Expression::Closure { .. })
            && !sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx))
                .is_some_and(
                    crate::codegen::rust::stdlib_method_traits::formal_is_rust_closure_trait,
                )
        {
            if let Expression::Identifier { name, .. } = arg_expr {
                if self.current_function_params.iter().any(|p| p.name == *name)
                    && !coerced.starts_with('&')
                    && !coerced.starts_with("&mut ")
                {
                    if !coerced.ends_with(".clone()") {
                        *coerced = format!("{coerced}.clone()");
                    }
                    *coerced = format!("&{coerced}");
                    return;
                }
            }
            // WDB-169/WDB-190: Call/MethodCall temps must fall through to owned peel — never
            // prefix `&callee()` here and never return early (blocks force_owned strip).
            if !matches!(
                arg_expr,
                Expression::Call { .. } | Expression::MethodCall { .. } | Expression::Closure { .. }
            )
                && !coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
                && !crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
                // P3.402: char / other Copy literals must not get `&` for Pattern/`&str`.
                && !crate::codegen::rust::call_site_borrow::expression_is_copy_literal(arg_expr)
                && !crate::codegen::rust::expression_utilities::is_rust_char_literal_text(coerced)
            {
                *coerced = format!("&{coerced}");
                return;
            }
            if !matches!(
                arg_expr,
                Expression::Call { .. } | Expression::MethodCall { .. }
            ) {
                return;
            }
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            if (self.borrowed_iterator_vars.contains(name)
                || self
                    .local_var_types
                    .get(name)
                    .is_some_and(|t| matches!(t, Type::Reference(_) | Type::MutableReference(_))))
                && crate::codegen::rust::types::return_type_is_vec_of_shared_refs(
                    self.current_function_return_type.as_ref(),
                )
            {
                crate::codegen::rust::expression_utilities::strip_trailing_clone(coerced);
                return;
            }
        }
        let runtime_std_borrow =
            crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                &self.signature_registry,
                callee_name,
                Some(sig),
                arg_index,
            );
        let emits_shared_ref =
            crate::ir::emission_contract::callee_emits_shared_rust_ref_param(sig, param_idx);
        let global_emits_shared_ref = {
            let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
            let skip_bare_homonym =
                crate::codegen::rust::call_signature_resolution::qualified_callee_skips_bare_homonym_lookup(
                    callee_name,
                );
            self.global_signature_registry.as_ref().is_some_and(|g| {
                let keys: Vec<&str> = if skip_bare_homonym {
                    vec![callee_name]
                } else {
                    vec![callee_name, simple]
                };
                keys.into_iter().any(|key| {
                    g.lookup_method(key).is_some_and(|gs| {
                        crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                            gs,
                            gs.arg_param_index(arg_index),
                        )
                    })
                })
            })
        };
        let mut_arg_emitted = {
            let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
            self.function_emitted_mut_arg_indices
                .get(callee_name)
                .or_else(|| self.function_emitted_mut_arg_indices.get(simple))
                .is_some_and(|indices| indices.contains(&arg_index))
        };
        let expects_mut = mut_arg_emitted
            || matches!(
                sig.param_types.get(param_idx),
                Some(Type::MutableReference(_))
            )
            || matches!(
                sig.formal_param_type(param_idx),
                Some(Type::MutableReference(_))
            )
            || matches!(
                crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                    sig, param_idx,
                ),
                crate::analyzer::OwnershipMode::MutBorrowed,
            );
        // Registry-aware Copy aggregates (Lsn, …) always emit owned formals — strip over-borrow
        // even when `emitted_owned_arg_contract` lacks pure-analysis Copy knowledge (regression-060).
        // Stale `Reference(Lsn)` in formal_param_types must not block this: formal generation
        // strips Copy-aggregate `&T` while analyzer metadata may still wrap the type.
        // Never treat true `&mut T` / MutBorrowed slots as owned Copy (apply_rotation /
        // fill_grid / update_health_regen).
        //
        // WDB-165: when *this* callee's emission record says owned (`emitted_owned_arg_contract`),
        // do not let a stale *global* shared-ref hit (`global_emits_shared_ref`) block stripping
        // `&state.clone()` into an owned `PgWireServeState` formal.
        let local_owned_emission =
            crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, param_idx);
        let copy_aggregate_owned = !expects_mut
            && sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx))
                .is_some_and(|t| {
                    if matches!(t, Type::MutableReference(_)) {
                        return false;
                    }
                    let bare = match t {
                        Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                        other => other,
                    };
                    self.is_type_copy(bare)
                        && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                })
            && !emits_shared_ref
            && (local_owned_emission || !global_emits_shared_ref);
        let force_owned = !expects_mut
            && !self.is_collection_key_lookup_at_site(
                sig,
                arg_index,
                callee_name
                    .rsplit_once("::")
                    .filter(|_| {
                        crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                            callee_name,
                        )
                    })
                    .map(|(ty, _)| {
                        ty.rsplit("::")
                            .next()
                            .unwrap_or(ty)
                            .split('<')
                            .next()
                            .unwrap_or(ty)
                    })
                    .or_else(|| {
                        crate::codegen::rust::stdlib_method_traits::receiver_type_from_qualified_sig(
                            sig,
                        )
                    }),
            )
            && (local_owned_emission
                || copy_aggregate_owned
                || crate::codegen::rust::signature_promotion::bare_formal_is_vec_or_map(
                    sig, param_idx,
                )
                || crate::codegen::rust::signature_promotion::bare_formal_is_owned_user_type(
                    sig, param_idx,
                ))
            && !runtime_std_borrow
            // Never strip `&` for text formals the callee emits as `&str` / shared ref
            // (regression-049 `replay_to_lsn(&self.path)`).
            && !emits_shared_ref
            // Same-file demotion is recorded on preregistered formals before sibling
            // bodies run (`encode_line` → `todo: &Todo`). Do not force-owned peel.
            && !self.preregistered_free_call_arg_expects_borrow(callee_name, arg_index)
            // WDB-169: owned Call/MethodCall temps (`empty_bakeoff_run()`) must move into
            // owned Custom formals even when a stale global shared-ref homonym exists —
            // `&empty_bakeoff_run()` is never a valid shared-ref binding.
            && (local_owned_emission
                || !global_emits_shared_ref
                || matches!(
                    arg_expr,
                    Expression::Call { .. } | Expression::MethodCall { .. }
                ));
        // Owned emission wins over stale analyzer/IR Ref expectations (regression-060
        // `is_at_or_before(&through)` → `other: Lsn`). Strip before shared-borrow path.
        // Do not strip `.clone()` — Copy aggregates still need multi-use clones (dogfood seed_write).
        // Also peel parenthesized unary refs from expression codegen: `(&through)`.
        if force_owned {
            let before = coerced.trim().to_string();
            let mut s = before.clone();
            loop {
                if s.starts_with('(') && s.ends_with(')') {
                    let inner = s[1..s.len() - 1].trim().to_string();
                    if inner.starts_with('&') || inner.starts_with("&mut ") {
                        s = inner;
                        continue;
                    }
                }
                if s.starts_with("&mut ") {
                    s = s["&mut ".len()..].trim().to_string();
                    continue;
                }
                if s.starts_with('&') {
                    s = s[1..].trim().to_string();
                    continue;
                }
                break;
            }
            let expected = safety_type_from_signature_param(sig, param_idx);
            let actual = self.infer_call_arg_actual_safety_type(arg_expr, before.as_str());
            if matches!(compute_coercion(&actual, &expected), CoercionKind::Clone)
                && !s.ends_with(".clone()")
                && !s.ends_with(".to_string()")
                && !s.ends_with(".to_owned()")
            {
                // WDB-343: Copy formals / Copy cast values auto-copy — never
                // `(x as i32).clone()` into owned i32 slots.
                let formal_is_copy = sig
                    .formal_param_type(param_idx)
                    .or_else(|| sig.param_types.get(param_idx))
                    .is_some_and(|t| self.is_type_copy(t));
                let arg_is_copy = self.expression_is_copy(arg_expr)
                    || self.infer_expression_type(arg_expr).is_some_and(|t| {
                        let pointee = match &t {
                            Type::Reference(inner) | Type::MutableReference(inner) => {
                                inner.as_ref()
                            }
                            other => other,
                        };
                        self.is_type_copy(pointee)
                    })
                    || Self::arg_str_is_copy_scalar_numeric_cast(&s);
                if !formal_is_copy && !arg_is_copy {
                    s = crate::codegen::rust::expression_utilities::append_rust_clone(&s);
                }
            }
            *coerced = s;
            return;
        }
        if self.preregistered_free_call_arg_expects_borrow(callee_name, arg_index)
            || crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(sig, param_idx)
        {
            // Owned clones/`to_string` deref-coerce to `&str` only. Custom `&T`
            // formals need `&item.clone()` (encode_line / WDB-125).
            let expected_for_clone = safety_type_from_signature_param(sig, param_idx);
            if crate::ir::coercion::rust_owned_temp_deref_coerces_into_shared_ref(
                &expected_for_clone,
            ) && (coerced.ends_with(".clone()")
                || coerced.ends_with(".to_string()")
                || coerced.ends_with(".to_owned()"))
            {
                let mut s = coerced.trim().to_string();
                while s.starts_with('&') {
                    s = s[1..].trim().to_string();
                }
                *coerced = s;
                return;
            }
            // Symmetric borrows for readonly helpers (`keys_equal(&a, &b)`).
            if let Expression::Identifier { name, .. } = arg_expr {
                let caller_borrowed_formal = self.emitted_rust_ref_formals.contains(name)
                    || (self.inferred_borrowed_params.contains(name)
                        && self.current_function_params.iter().any(|p| p.name == *name));
                let callee_param_is_current_fn_formal =
                    self.current_function_params.iter().any(|p| p.name == *name);
                if (caller_borrowed_formal || callee_param_is_current_fn_formal)
                    && !coerced.starts_with('&')
                    && !coerced.starts_with("&mut ")
                {
                    if !coerced.ends_with(".clone()") {
                        *coerced = format!("{coerced}.clone()");
                    }
                    *coerced = format!("&{coerced}");
                    return;
                }
                if (caller_borrowed_formal || callee_param_is_current_fn_formal)
                    && !coerced.ends_with(".clone()")
                    && self.identifier_binding_already_rust_ref(name)
                {
                    *coerced = format!("{coerced}.clone()");
                    *coerced = format!("&{coerced}");
                    return;
                }
            }
            if matches!(
                arg_expr,
                Expression::Identifier { name, .. }
                    if self.identifier_binding_already_rust_ref(name)
            ) {
                // `&mut T` coerces to `&T` — never stack shared `&` (`&&mut`, `&&T`).
                *coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(coerced)
                    .to_string();
                return;
            }
            // WDB-169/WDB-190: Call temps into owned formals must not grow `&`.
            // MethodCall `.clone()` into Custom `&T` does not autoborrow.
            // WDB-329: Cast-to-Copy/`usize` is a value (`idx as usize`), not a place.
            let clone_temp_needs_explicit_ref = matches!(arg_expr, Expression::MethodCall { .. })
                && (coerced.ends_with(".clone()")
                    || coerced.ends_with(".to_string()")
                    || coerced.ends_with(".to_owned()"))
                && !crate::ir::coercion::rust_owned_temp_deref_coerces_into_shared_ref(
                    &expected_for_clone,
                );
            if !coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
                && !crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr)
                && (!matches!(
                    arg_expr,
                    Expression::Call { .. }
                        | Expression::MethodCall { .. }
                        | Expression::Closure { .. }
                        | Expression::Cast { .. }
                ) || clone_temp_needs_explicit_ref)
                && !coerced.contains(" as usize")
            {
                *coerced = format!("&{coerced}");
            }
            // Vec literals at `append_put(&vec![…], …)` sites still need explicit `&`.
            if !coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
                && matches!(arg_expr, Expression::Array { .. })
            {
                *coerced = format!("&{coerced}");
            }
            return;
        }
        let expected = safety_type_from_signature_param(sig, param_idx);
        let actual = self.infer_call_arg_actual_safety_type(arg_expr, coerced.as_str());
        crate::ir::coercion::enforce_ownership_contract_on_coerced_arg_with_force_owned(
            coerced,
            &actual,
            &expected,
            force_owned,
            false,
            runtime_std_borrow,
        );
    }

    /// Cross-crate metadata (`WalSegment::append_put`) — borrow `vec![…]` and helper returns.
    fn maybe_borrow_vec_or_helper_from_global_metadata(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        callee_name: &str,
        arg_index: usize,
        receiver_type_name: Option<&str>,
        registry: &SignatureRegistry,
    ) {
        if coerced.starts_with('&') || coerced.starts_with("&mut ") {
            return;
        }
        let lvalue_borrow_site =
            crate::codegen::rust::call_site_borrow::expression_is_vec_literal_producer(arg_expr)
                || matches!(
                    arg_expr,
                    Expression::Index { .. }
                        | Expression::FieldAccess { .. }
                        | Expression::Call { .. }
                );
        if !lvalue_borrow_site {
            return;
        }
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let mut lookup_keys = vec![callee_name.to_string()];
        if let Some(rt) = receiver_type_name {
            lookup_keys.push(format!("{rt}::{simple}"));
        }
        let suffix = format!("::{simple}");
        let registries: Vec<&SignatureRegistry> = self
            .global_signature_registry
            .as_ref()
            .map(|g| vec![g.as_ref(), registry])
            .unwrap_or_else(|| vec![registry]);
        for reg in registries {
            if let Some(gs) = reg.find_unique_signature_ending_with(&suffix) {
                lookup_keys.push(gs.name.clone());
            }
            for key in &lookup_keys {
                let Some(gs) = reg.get_signature(key).or_else(|| reg.lookup_method(key)) else {
                    continue;
                };
                let pidx = gs.arg_param_index(arg_index);
                if gs
                    .forwarding_borrow_params
                    .as_ref()
                    .and_then(|flags| flags.get(pidx))
                    .copied()
                    .unwrap_or(false)
                    || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(gs, pidx)
                    || crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(gs, pidx)
                    || crate::codegen::rust::call_site_borrow::callee_arg_expects_shared_vec_ref(
                        gs, arg_index,
                    )
                {
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(coerced);
                    if coerced.ends_with(".to_string()") {
                        if let Some(stripped) = coerced.strip_suffix(".to_string()") {
                            *coerced = stripped.to_string();
                        }
                    }
                    if !coerced.starts_with('&') {
                        *coerced = format!("&{coerced}");
                    }
                    return;
                }
            }
        }
    }

    pub(crate) fn apply_registry_borrow_to_call_arg(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        receiver_type_name: Option<&str>,
        method: &str,
        arg_index: usize,
        user_arg_count: Option<usize>,
    ) {
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.identifier_binding_already_rust_ref(name)
                || self.inferred_borrowed_params.contains(name)
                || self.str_ref_optimized_params.contains(name)
            {
                return;
            }
        }
        let Some(_rt) = receiver_type_name else {
            return;
        };
        let arg_count = user_arg_count.unwrap_or(arg_index + 1);
        let mut receiver_types: Vec<&str> = Vec::new();
        if let Some(rt) = receiver_type_name {
            receiver_types.push(rt);
        }
        if let Some(sn) = self.current_struct_name.as_deref() {
            if !receiver_types.contains(&sn) {
                receiver_types.push(sn);
            }
        }
        for rt in &receiver_types {
            let qualified = format!("{rt}::{method}");
            let initial = self.resolve_method_function_signature(rt, method, arg_count);
            let Some(sig) =
                self.refresh_call_site_signature_for_arg(initial, &qualified, arg_index)
            else {
                continue;
            };
            let pidx = sig.arg_param_index(arg_index);
            if let Some(resolved) = self.resolve_method_function_signature(rt, method, arg_count) {
                let ridx = resolved.arg_param_index(arg_index);
                if (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &resolved, ridx,
                ) || crate::codegen::rust::signature_promotion::bare_formal_is_owned_user_type(
                    &resolved, ridx,
                ) || matches!(
                    resolved.param_ownership.get(ridx),
                    Some(crate::analyzer::OwnershipMode::Owned)
                )) && !resolved
                    .param_types
                    .get(ridx)
                    .is_some_and(|t| matches!(t, Type::MutableReference(_)))
                {
                    if coerced.starts_with('&') {
                        *coerced = crate::codegen::rust::expression_utilities::coerce_borrowed_arg_to_owned(
                            coerced,
                        );
                    } else if let Expression::Identifier { name, .. } = arg_expr {
                        if (self.emitted_rust_ref_formals.contains(name)
                            || self.inferred_borrowed_params.contains(name))
                            && !coerced.ends_with(".clone()")
                        {
                            *coerced = format!("{coerced}.clone()");
                        }
                    }
                    return;
                }
            }
            if let Some(global) = self.global_signature_registry.as_ref() {
                let qualified = format!("{rt}::{method}");
                if let Some(gs) = global.get_signature(&qualified) {
                    let gpidx = gs.arg_param_index(arg_index);
                    if crate::ir::emission_contract::callee_emits_shared_rust_ref_param(gs, gpidx)
                        && !coerced.starts_with('&')
                    {
                        crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                            coerced,
                        );
                        return;
                    }
                }
            }
            // Emitted owned formals win over stale MutBorrowed/Borrowed metadata.
            // Never peel true `&mut T` / MutBorrowed slots (auto_mut `fill(&mut buf)`):
            // AST bare `Vec`/`Custom` formals are still MutBorrowed after analyzer inference.
            let effective_own =
                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                    &sig, arg_index,
                );
            let slot_expects_mut = sig
                .param_types
                .get(pidx)
                .is_some_and(|t| matches!(t, Type::MutableReference(_)))
                || matches!(effective_own, crate::analyzer::OwnershipMode::MutBorrowed);
            if !slot_expects_mut
                && (crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &sig, pidx,
                ) || crate::codegen::rust::signature_promotion::bare_formal_is_owned_user_type(
                    &sig, pidx,
                ) || self
                    .struct_method_ast_formal_param_types
                    .get(*rt)
                    .and_then(|methods| methods.get(method))
                    .and_then(|formals| formals.get(arg_index))
                    .is_some_and(|t| {
                        !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                            && !self.is_type_copy(t)
                            && !crate::codegen::rust::types::is_windjammer_text_type(t)
                    }))
            {
                if coerced.starts_with('&') {
                    *coerced =
                        crate::codegen::rust::expression_utilities::coerce_borrowed_arg_to_owned(
                            coerced,
                        );
                } else if let Expression::Identifier { name, .. } = arg_expr {
                    if (self.emitted_rust_ref_formals.contains(name)
                        || self.inferred_borrowed_params.contains(name))
                        && !coerced.ends_with(".clone()")
                    {
                        *coerced = format!("{coerced}.clone()");
                    }
                }
                return;
            }
            let wants_shared_ref =
                crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(&sig, pidx)
                    || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&sig, pidx);
            let param_is_shared_ref_type = sig
                .param_types
                .get(pidx)
                .is_some_and(|t| matches!(t, Type::Reference(_)));
            let effective_own =
                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                    &sig, arg_index,
                );
            let wants_mut = !wants_shared_ref
                && !crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                    &sig, pidx,
                )
                && !crate::codegen::rust::signature_promotion::bare_formal_is_owned_user_type(
                    &sig, pidx,
                )
                && (sig
                    .param_types
                    .get(pidx)
                    .is_some_and(|t| matches!(t, Type::MutableReference(_)))
                    || (matches!(effective_own, crate::analyzer::OwnershipMode::MutBorrowed)
                        && !param_is_shared_ref_type));
            if wants_mut {
                if self.in_if_condition {
                    if let Expression::Identifier { name, .. } = arg_expr {
                        if self.current_fn_forward_ref_if_params.contains(name)
                            && self.caller_keeps_owned_outer_formal(name)
                        {
                            if coerced.starts_with("&mut ") {
                                *coerced = format!(
                                    "&{}",
                                    crate::codegen::rust::expression_utilities::borrow_base_expr(
                                        coerced,
                                    ),
                                );
                            } else if !coerced.starts_with('&') {
                                crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                                    coerced,
                                );
                            }
                            return;
                        }
                    }
                }
                // String literals are never `&mut` lvalues.
                if crate::codegen::rust::call_site_borrow::expression_is_string_literal(arg_expr) {
                    return;
                }
                if coerced.starts_with("&mut ") {
                    return;
                }
                // Mut borrow needs an lvalue — never `&mut binding.clone()` (WDB-336/337/342).
                // Auto-clone may have fired for reuse before demotion to `&mut Vec`.
                crate::codegen::rust::expression_utilities::apply_mut_borrow_prefix(coerced);
                return;
            }
            if crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&sig, pidx)
                && !coerced.starts_with('&')
            {
                crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(coerced);
                return;
            }
        }
        if coerced.starts_with('&') {
            return;
        }
        for rt in &receiver_types {
            if self.method_registry_arg_expects_shared_borrow(rt, method, arg_index, arg_count) {
                crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(coerced);
                return;
            }
        }
        // Cross-crate metadata (`WalSegment::append_put`) — borrow vec literals and
        // helper returns even when local receiver-type inference failed.
        if crate::codegen::rust::call_site_borrow::expression_is_vec_literal_producer(arg_expr)
            || matches!(arg_expr, Expression::Call { .. })
        {
            if let Some(resolved) =
                self.resolve_call_signature_with_global(method, receiver_type_name, arg_count)
            {
                let pidx = resolved.sig.arg_param_index(arg_index);
                if crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                    &resolved.sig,
                    pidx,
                ) && !coerced.starts_with('&')
                {
                    crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(coerced);
                    return;
                }
            }
            if let Some(global) = self.global_signature_registry.as_ref() {
                let qualified = receiver_type_name
                    .map(|rt| format!("{rt}::{method}"))
                    .unwrap_or_else(|| method.to_string());
                if let Some(gs) = global
                    .get_signature(&qualified)
                    .or_else(|| global.get_signature(method))
                {
                    let pidx = gs.arg_param_index(arg_index);
                    if crate::ir::emission_contract::callee_emits_shared_rust_ref_param(gs, pidx)
                        && !coerced.starts_with('&')
                    {
                        crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(
                            coerced,
                        );
                    }
                }
            }
        }
        // Type-qualified associated calls (`Metric::new`) without a matching
        // `{Type}::{method}` signature must fail closed — never borrow via bare
        // method-name homonyms (`path::new` → Borrowed) for cross-crate Copy args.
        if let Some(rt) = receiver_type_name {
            let qualified = format!("{rt}::{method}");
            if crate::codegen::rust::call_signature_resolution::is_type_qualified_associated_call(
                &qualified,
            ) {
                return;
            }
        }
        if let Some(resolved) =
            self.resolve_call_signature_with_global(method, receiver_type_name, arg_count)
        {
            let pidx = resolved.sig.arg_param_index(arg_index);
            if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                &resolved.sig,
                pidx,
            ) {
                return;
            }
            if crate::ir::emission_contract::callee_emits_shared_rust_ref_param(&resolved.sig, pidx)
            {
                crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(coerced);
            }
        }
    }

    /// Runtime std modules declare owned WJ params but Rust takes `&T` / `&str`.
    pub(crate) fn finish_runtime_std_call_arg(
        &self,
        callee_name: &str,
        arg_index: usize,
        arg_expr: &Expression<'ast>,
        mut coerced: String,
        signature: Option<&crate::analyzer::FunctionSignature>,
        receiver_type_name: Option<&str>,
    ) -> String {
        if coerced.contains("string_to_ffi(") {
            return coerced;
        }
        let is_closure_arg = matches!(arg_expr, Expression::Closure { .. })
            || matches!(
                arg_expr,
                Expression::Binary {
                    op: crate::parser::BinaryOp::Or,
                    left,
                    ..
                } if matches!(
                    &**left,
                    Expression::Identifier { name, .. } if name == "move"
                )
            );
        if is_closure_arg {
            let mut s = coerced;
            while s.starts_with("&mut ") {
                s = s["&mut ".len()..].trim().to_string();
            }
            while s.starts_with('&') {
                s = s[1..].trim().to_string();
            }
            return s;
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            if self.into_string_formal_params.contains(name) {
                let mut s = coerced;
                while s.starts_with("&mut ") {
                    s = s["&mut ".len()..].trim().to_string();
                }
                while s.starts_with('&') {
                    s = s[1..].trim().to_string();
                }
                crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut s);
                if !s.ends_with(".into()")
                    && !s.ends_with(".to_string()")
                    && !s.ends_with(".to_owned()")
                {
                    s = format!("{s}.into()");
                }
                return s;
            }
        }
        if ownership_from_rust_expr(coerced.as_str()).is_some() {
            return coerced;
        }
        if let Some(sig) = signature {
            let idx = sig.arg_param_index(arg_index);
            if crate::codegen::rust::call_signature_resolution::plain_string_owned_consumer_at_call_site(
                sig, idx,
            ) {
                while coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                    coerced = coerced[1..].trim().to_string();
                }
            } else if coerced.starts_with('&') {
                return coerced;
            }
        } else if coerced.starts_with('&') {
            return coerced;
        }
        if let Expression::Identifier { name, .. } = arg_expr {
            // Only skip when Rust already emits this binding as a reference formal.
            // `identifier_already_ref` is too broad for match-arm payloads and
            // stale inferred_borrowed_params (json::get(&v) needs & on owned `v`).
            if self.emitted_rust_ref_formals.contains(name)
                || self.str_ref_optimized_params.contains(name)
                || self.current_function_params.iter().any(|p| {
                    p.name == *name
                        && matches!(p.type_, Type::Reference(_) | Type::MutableReference(_))
                })
            {
                return coerced;
            }
        }
        let module = callee_name.split("::").next().unwrap_or("");
        let method = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let inferred_type = self.infer_expression_type(arg_expr);
        let effective_module =
            crate::codegen::rust::stdlib_method_traits::resolve_runtime_std_module(
                module,
                receiver_type_name,
            );

        if !coerced.starts_with('&')
            && (crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                &self.signature_registry,
                callee_name,
                signature,
                arg_index,
            )
                || crate::codegen::rust::stdlib_method_traits::runtime_std_call_arg_needs_auto_borrow(
                    effective_module,
                    method,
                    signature,
                    arg_index,
                    inferred_type.as_ref(),
                    arg_expr,
                    receiver_type_name,
                ))
        {
            coerced = crate::ir::target_encodings::rust_shared_borrow(&coerced);
        }

        let signature_for_borrow =
            if crate::codegen::rust::call_signature_resolution::is_external_module_qualified_call(
                callee_name,
            ) {
                self.global_signature_registry
                    .as_ref()
                    .and_then(|g| g.get_signature(callee_name))
                    .or(signature)
            } else {
                signature
            };

        if !coerced.starts_with('&') {
            if let Some(sig) = signature_for_borrow {
                let idx = sig.arg_param_index(arg_index);
                let param_ty = sig
                    .formal_param_type(idx)
                    .or_else(|| sig.param_types.get(idx));
                let effective_ownership =
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                        sig, arg_index,
                    );
                // Plain WJ `string` / codegen-owned formals pass by value — stale analyzer
                // `Reference(str)` and Borrowed must not prefix `&field` (join_path seed).
                let owned_plain_string =
                    crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                        sig, idx,
                    ) || crate::ir::emission_contract::plain_string_formal_passes_owned_at_call_site(
                        sig, idx,
                    ) || crate::ir::signature_bridge::call_site_expects_owned_pass(sig, idx)
                    || crate::codegen::rust::call_signature_resolution::plain_string_owned_consumer_at_call_site(
                        sig, idx,
                    );
                let callee_borrows_text = !owned_plain_string
                    && (param_ty.is_some_and(|t| {
                        crate::codegen::rust::string_utilities::param_is_rust_string_ref(t)
                            || crate::codegen::rust::string_utilities::param_is_rust_str_ref(t)
                    }) || crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(
                        sig, idx,
                    ) || (matches!(
                        effective_ownership,
                        crate::analyzer::OwnershipMode::Borrowed
                            | crate::analyzer::OwnershipMode::MutBorrowed
                    ) && crate::ir::emission_contract::callee_emits_shared_rust_ref_param(
                        sig, idx,
                    ) && (param_ty
                        .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
                        || inferred_type
                            .as_ref()
                            .is_some_and(crate::codegen::rust::types::is_windjammer_text_type))));
                if callee_borrows_text
                    && matches!(
                        arg_expr,
                        Expression::Identifier { .. } | Expression::FieldAccess { .. }
                    )
                    && !matches!(
                        arg_expr,
                        Expression::Identifier { name, .. }
                            if self.binding_emits_as_rust_shared_ref(name)
                    )
                {
                    coerced = crate::ir::target_encodings::rust_shared_borrow(&coerced);
                }
            } else if crate::codegen::rust::call_signature_resolution::is_external_module_qualified_call(
                callee_name,
            ) {
                if let Some(global) = self.global_signature_registry.as_ref() {
                    if let Some(global_sig) = global.get_signature(callee_name) {
                        let effective =
                            crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                                global_sig, arg_index,
                            );
                        if matches!(
                            effective,
                            crate::analyzer::OwnershipMode::Borrowed
                                | crate::analyzer::OwnershipMode::MutBorrowed
                        ) && matches!(
                            arg_expr,
                            Expression::Identifier { .. } | Expression::FieldAccess { .. }
                        ) && !matches!(
                            arg_expr,
                            Expression::Identifier { name, .. }
                                if self.binding_emits_as_rust_shared_ref(name)
                        ) {
                            coerced =
                                crate::ir::target_encodings::rust_shared_borrow(&coerced);
                        }
                    }
                }
            }
        }

        if matches!(
                arg_expr,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            )
            && coerced.ends_with(".to_string()")
            && crate::codegen::rust::stdlib_method_traits::runtime_or_str_ref_formal_skips_literal_owned(
                signature,
                arg_index,
            )
        {
            coerced = coerced
                .trim_end_matches(".to_string()")
                .to_string();
        }

        coerced
    }

    /// Strip spurious re-borrows when the argument is already a borrowed parameter.
    fn finalize_ir_call_arg(
        &self,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
        coerced: &str,
    ) -> String {
        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) {
            if let Some(rest) = coerced.strip_prefix("&mut ") {
                return rest.to_string();
            }
            if let Some(rest) = coerced.strip_prefix('&') {
                return rest.to_string();
            }
            return coerced.to_string();
        }

        let Expression::Identifier { name, .. } = arg_expr else {
            if arg_str.ends_with(".clone()")
                && coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
            {
                return coerced[1..].to_string();
            }
            return coerced.to_string();
        };
        if self.inferred_mut_borrowed_params.contains(name) && coerced.starts_with("&mut ") {
            return coerced["&mut ".len()..].to_string();
        }
        if (self.identifier_already_ref(name)
            || self.emitted_rust_ref_formals.contains(name)
            || self.borrowed_iterator_vars.contains(name))
            && !self.inferred_mut_borrowed_params.contains(name)
            && coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
        {
            if self.copy_match_payload_binding(name) {
                return coerced[1..].to_string();
            }
            let is_copy = self
                .local_var_types
                .get(name)
                .is_some_and(|t| self.is_type_copy(t))
                || self
                    .infer_expression_type(arg_expr)
                    .is_some_and(|t| self.is_type_copy(&t))
                || self.binding_is_copy_pass_by_value_scalar(name);
            if is_copy {
                // `&binding.clone()` → `*binding` (never `*binding.clone()`, E0614 on Copy).
                let mut core = coerced[1..].to_string();
                crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut core);
                if core.starts_with('*') {
                    return core;
                }
                return format!("*{core}");
            }
            return coerced[1..].to_string();
        }
        coerced.to_string()
    }

    /// Infer the safety type of a call-site argument from solver-resolved types,
    /// parameter borrow state, generated Rust text, and expression shape (fallback).
    pub(crate) fn infer_actual_safety_type(
        &self,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
    ) -> SafetyType {
        if arg_str.ends_with(".clone()") {
            if let Expression::Identifier { name, .. } = arg_expr {
                if self.emitted_rust_ref_formals.contains(name)
                    || (self.inferred_borrowed_params.contains(name)
                        && self.current_function_params.iter().any(|p| {
                            p.name == *name
                                && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                        }))
                {
                    return SafetyType::borrowed(BaseType::String, Region::fresh(12));
                }
            }
            let base = self
                .infer_expression_type(arg_expr)
                .as_ref()
                .map(|ty| crate::ir::node::parser_type_to_base_type(ty))
                .unwrap_or(BaseType::Inferred);
            return SafetyType::owned(base);
        }

        // Generated owned String (`"lit".to_string()` / `.to_owned()`) must not be
        // classified as a borrowed string-literal — that causes a later `&` prefix
        // (`&"lit".to_string()`) for Owned formals (Objective::kill factory).
        if arg_str.ends_with(".to_string()") || arg_str.ends_with(".to_owned()") {
            return SafetyType::owned(BaseType::String);
        }

        if matches!(arg_expr, Expression::Closure { .. })
            || matches!(
                arg_expr,
                Expression::Binary {
                    op: crate::parser::BinaryOp::Or,
                    left,
                    ..
                } if matches!(
                    &**left,
                    Expression::Identifier { name, .. } if name == "move"
                )
            )
        {
            return safety_type_from_arg_expression(arg_expr);
        }

        if matches!(
            arg_expr,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) {
            return SafetyType::borrowed(BaseType::String, Region::fresh(2));
        }

        if matches!(
            arg_expr,
            Expression::Unary {
                op: crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef,
                operand: _,
                ..
            }
        ) {
            if let Some(inner_ty) = self.infer_expression_type(arg_expr) {
                return safety_type_from_parser_type(&inner_ty, None);
            }
        }

        if let Expression::Identifier { name, .. } = arg_expr {
            if let Some(param) = self
                .current_function_params
                .iter()
                .find(|p| p.name == *name)
            {
                if self.emitted_rust_ref_formals.contains(name.as_str())
                    && crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
                {
                    return SafetyType::borrowed(BaseType::String, Region::fresh(13));
                }
                if self.is_type_copy(&param.type_)
                    && !crate::type_classification::is_copy_pass_by_value_formal(&param.type_)
                {
                    // Demoted Copy-aggregate formals emit `&T` in Rust. Treating them as
                    // OwnedType::Copy makes Vec/HashSet::contains Borrow → `&*entity`
                    // (auto_ref_deref_copy). Honor emitted shared-ref formals as Ref.
                    if self.emitted_rust_ref_formals.contains(name.as_str())
                        || self.inferred_borrowed_params.contains(name.as_str())
                        || self.identifier_already_ref(name)
                    {
                        return self.safety_type_for_param_binding(
                            arg_expr,
                            OwnedType::Ref(Region::fresh(0)),
                        );
                    }
                    return SafetyType::copy(crate::ir::node::parser_type_to_base_type(
                        &param.type_,
                    ));
                }
            }
            if self.borrowed_iterator_vars.contains(name) {
                let base = self
                    .infer_expression_type(arg_expr)
                    .as_ref()
                    .map(|ty| match ty {
                        Type::Reference(inner) | Type::MutableReference(inner) => {
                            crate::ir::node::parser_type_to_base_type(inner)
                        }
                        other => crate::ir::node::parser_type_to_base_type(other),
                    })
                    .unwrap_or(BaseType::Inferred);
                return SafetyType::borrowed(base, Region::fresh(11));
            }
            // Match-arm bindings are owned enum/struct payloads even when
            // `local_var_types` temporarily marks them as references.
            if self.match_arm_bindings.contains(name.as_str()) {
                let base = self
                    .infer_expression_type(arg_expr)
                    .as_ref()
                    .map(|ty| match ty {
                        Type::Reference(inner) | Type::MutableReference(inner) => {
                            crate::ir::node::parser_type_to_base_type(inner)
                        }
                        other => crate::ir::node::parser_type_to_base_type(other),
                    })
                    .unwrap_or(BaseType::Inferred);
                return SafetyType::owned(base);
            }
            if self.inferred_mut_borrowed_params.contains(name) {
                return self
                    .safety_type_for_param_binding(arg_expr, OwnedType::MutRef(Region::fresh(1)));
            }
            if self.identifier_already_ref(name) {
                return self
                    .safety_type_for_param_binding(arg_expr, OwnedType::Ref(Region::fresh(0)));
            }
            // Forward-ref / owned formals: analyzer may still infer `&str` while Rust
            // emits `String` — coerce as owned at call sites (`check(&text)`).
            if self.current_function_params.iter().any(|p| p.name == *name)
                && !self.emitted_rust_ref_formals.contains(name)
            {
                if let Some(ty) = self.infer_expression_type(arg_expr) {
                    if matches!(
                        &ty,
                        Type::Reference(inner) | Type::MutableReference(inner)
                            if crate::codegen::rust::types::is_windjammer_text_type(inner.as_ref())
                    ) || crate::codegen::rust::types::is_windjammer_text_type(&ty)
                    {
                        return SafetyType::owned(BaseType::String);
                    }
                }
            }
        }

        // Copy field reads through `&Struct` / match bindings are values in Rust
        // (`failure.status` → `i64`), not `&i64`. Treating them as Ref makes
        // ownership coercion emit `*failure.status` (E0614).
        if matches!(arg_expr, Expression::FieldAccess { .. }) {
            if let Some(ty) = self.infer_expression_type(arg_expr) {
                let bare = match &ty {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                if self.is_type_copy(bare) {
                    return SafetyType::copy(crate::ir::node::parser_type_to_base_type(bare));
                }
            }
        }

        if let Some(ownership) = ownership_from_rust_expr(arg_str) {
            let base = self
                .infer_expression_type(arg_expr)
                .as_ref()
                .map(|ty| crate::ir::node::parser_type_to_base_type(ty))
                .unwrap_or(BaseType::Inferred);
            let mut from_ast = SafetyType {
                base,
                ownership,
                effects: crate::ir::safety_type::EffectSet::pure(),
                taint: crate::ir::safety_type::TaintStatus::Clean,
                const_eval: crate::ir::safety_type::ConstEval::Runtime,
                exec_mode: None,
            };
            from_ast = self.merge_actual_with_solver_binding(arg_expr, arg_str, from_ast);
            return from_ast;
        }

        if let Some(ty) = self.infer_expression_type(arg_expr) {
            let from_ast = safety_type_from_parser_type(&ty, None);
            return self.merge_actual_with_solver_binding(arg_expr, arg_str, from_ast);
        }

        let from_ast = safety_type_from_arg_expression(arg_expr);
        self.merge_actual_with_solver_binding(arg_expr, arg_str, from_ast)
    }

    /// Prefer current-function IR binding types, then module-wide solver lookup.
    fn merge_actual_with_solver_binding(
        &self,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
        from_ast: SafetyType,
    ) -> SafetyType {
        let binding = if let Expression::Identifier { name, .. } = arg_expr {
            Some(name.as_str())
        } else {
            None
        };
        if let Some(name) = binding {
            if let Some(ir_fn) = self.current_ir_function.as_ref() {
                if let Some(from_ir) = ir_fn
                    .param_types
                    .get(name)
                    .or_else(|| ir_fn.local_types.get(name))
                {
                    return crate::ir::signature_bridge::merge_call_arg_actual_with_ir(
                        from_ast,
                        from_ir.clone(),
                    );
                }
            }
        }
        if let Some(module) = self.ir_module.as_ref() {
            if let Some(from_ir) =
                crate::ir::signature_bridge::safety_type_from_ir_binding(module, arg_str.trim())
                    .or_else(|| {
                        binding.and_then(|name| {
                            crate::ir::signature_bridge::safety_type_from_ir_binding(module, name)
                        })
                    })
            {
                return crate::ir::signature_bridge::merge_call_arg_actual_with_ir(
                    from_ast, from_ir,
                );
            }
        }
        from_ast
    }

    fn safety_type_for_param_binding(
        &self,
        arg_expr: &Expression<'ast>,
        ownership: OwnedType,
    ) -> SafetyType {
        let base = self
            .infer_expression_type(arg_expr)
            .as_ref()
            .map(|ty| type_pointee_base(ty))
            .unwrap_or(BaseType::Inferred);
        SafetyType {
            base,
            ownership,
            effects: crate::ir::safety_type::EffectSet::pure(),
            taint: crate::ir::safety_type::TaintStatus::Clean,
            const_eval: crate::ir::safety_type::ConstEval::Runtime,
            exec_mode: None,
        }
    }

    /// True when a callee param only field-extracts the argument (partial move semantics).
    fn callee_param_field_extracts(
        &self,
        registry: &SignatureRegistry,
        callee_name: &str,
        arg_index: usize,
        _arg_name: &str,
    ) -> bool {
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let sig = registry
            .get_signature(callee_name)
            .or_else(|| registry.lookup_method(callee_name))
            .or_else(|| registry.find_signature_ending_with(simple))
            .or_else(|| {
                self.global_signature_registry.as_ref().and_then(|g| {
                    g.get_signature(callee_name)
                        .or_else(|| g.lookup_method(callee_name))
                        .or_else(|| g.find_signature_ending_with(simple))
                })
            });
        let Some(sig) = sig else {
            return false;
        };
        let param_idx = sig.arg_param_index(arg_index);
        sig.field_extract_params
            .as_ref()
            .and_then(|flags| flags.get(param_idx))
            .copied()
            .unwrap_or(false)
    }

    /// Apply auto-clone analysis before IR coercion when a binding is reused after partial move.
    pub(crate) fn maybe_auto_clone_call_arg(
        &self,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
        callee_name: Option<&str>,
        arg_index: Option<usize>,
    ) -> String {
        match arg_expr {
            Expression::Identifier { name, .. } => {
                if name == "None"
                    || name == "true"
                    || name == "false"
                    || name.ends_with("::None")
                    || crate::type_classification::is_enum_variant_constructor_path(name)
                {
                    return arg_str.to_string();
                }
                if self.into_string_formal_params.contains(name) {
                    return arg_str.to_string();
                }
                if self.param_used_in_prior_field_extract_call(name) {
                    return arg_str.to_string();
                }
                if self.for_loop_borrow_needed.contains(name) {
                    if let (Some(callee), Some(idx)) = (callee_name, arg_index) {
                        let callee_wants_borrow = self
                            .callee_arg_expects_borrow_at_call(callee, idx)
                            || self.ir_callee_arg_expects_shared_borrow(
                                &self.signature_registry,
                                callee,
                                idx,
                                None,
                                None,
                            )
                            || self.global_signature_registry.as_ref().is_some_and(|g| {
                                self.ir_callee_arg_expects_shared_borrow(g, callee, idx, None, None)
                            })
                            || self.inferred_borrowed_params.contains(name)
                            || self.emitted_rust_ref_formals.contains(name);
                        if callee_wants_borrow {
                            let base = crate::codegen::rust::expression_utilities::borrow_base_expr(
                                arg_str,
                            );
                            if base.starts_with('&') {
                                return base.to_string();
                            }
                            return format!("&{base}");
                        }
                    }
                }
                if let (Some(callee), Some(idx)) = (callee_name, arg_index) {
                    if self.should_auto_clone_reused_owned_param_at_call(name, callee, idx) {
                        return self.emit_signature_driven_reused_owned_param_clone(
                            name, arg_str, callee, idx,
                        );
                    }
                    if self.callee_param_field_extracts_by_name(callee, idx) {
                        return arg_str.to_string();
                    }
                    let analysis_wants_clone = self
                        .auto_clone_analysis
                        .as_ref()
                        .is_some_and(|a| a.needs_clone(name, self.current_statement_idx).is_some());
                    if analysis_wants_clone {
                        if self.caller_emits_mut_ref_formal(name)
                            && self.callee_slot_emits_mut_borrow(callee, idx)
                        {
                            return arg_str.to_string();
                        }
                        if !self.callee_arg_emits_owned_contract(callee, idx)
                            && (self.callee_arg_expects_borrow_at_call(callee, idx)
                                || self.ir_callee_arg_expects_shared_borrow(
                                    &self.signature_registry,
                                    callee,
                                    idx,
                                    None,
                                    None,
                                )
                                || self.global_signature_registry.as_ref().is_some_and(|g| {
                                    self.ir_callee_arg_expects_shared_borrow(
                                        g, callee, idx, None, None,
                                    )
                                }))
                        {
                            let base = crate::codegen::rust::expression_utilities::borrow_base_expr(
                                arg_str,
                            );
                            if base.starts_with('&') {
                                return base.to_string();
                            }
                            return format!("&{base}");
                        }
                        return self.maybe_auto_clone(name, arg_str);
                    }
                    let emits_owned = self.callee_arg_emits_owned_contract(callee, idx);
                    if self.borrowed_iterator_vars.contains(name) {
                        let wants_borrow = self.callee_arg_expects_borrow_at_call(callee, idx)
                            || self.ir_callee_arg_expects_shared_borrow(
                                &self.signature_registry,
                                callee,
                                idx,
                                None,
                                None,
                            )
                            || self.global_signature_registry.as_ref().is_some_and(|g| {
                                self.ir_callee_arg_expects_shared_borrow(g, callee, idx, None, None)
                            })
                            || self.ir_callee_arg_expects_mut_borrow(
                                &self.signature_registry,
                                callee,
                                idx,
                                None,
                                None,
                            );
                        if !wants_borrow && !self.binding_is_copy_pass_by_value_scalar(name) {
                            let non_copy =
                                self.infer_expression_type(arg_expr)
                                    .map(|t| {
                                        let bare = match &t {
                                            Type::Reference(inner)
                                            | Type::MutableReference(inner) => inner.as_ref(),
                                            other => other,
                                        };
                                        !self.is_type_copy(bare)
                                    })
                                    .unwrap_or(true);
                            if non_copy {
                                let base = arg_str.trim_start_matches('&');
                                if !base.ends_with(".clone()") {
                                    return format!("{base}.clone()");
                                }
                            }
                        }
                    }
                    if !emits_owned
                        && (self.ir_callee_arg_expects_mut_borrow(
                            &self.signature_registry,
                            callee,
                            idx,
                            None,
                            None,
                        ) || self.global_signature_registry.as_ref().is_some_and(|g| {
                            self.ir_callee_arg_expects_mut_borrow(g, callee, idx, None, None)
                        }) || self.ir_callee_arg_expects_shared_borrow(
                            &self.signature_registry,
                            callee,
                            idx,
                            None,
                            None,
                        ) || self.global_signature_registry.as_ref().is_some_and(|g| {
                            self.ir_callee_arg_expects_shared_borrow(g, callee, idx, None, None)
                        }))
                    {
                        let reuse_after = self.local_binding_reused_after_current_statement(name);
                        if reuse_after {
                            if let Some(sig) =
                                self.signature_registry.get_signature(callee).or_else(|| {
                                    self.global_signature_registry
                                        .as_ref()
                                        .and_then(|g| g.get_signature(callee))
                                })
                            {
                                if let Some(cloned) = crate::codegen::rust::call_site_borrow::clone_reused_binding_for_owned_vec_formal(
                                    self,
                                    sig,
                                    idx,
                                    arg_expr,
                                    arg_str,
                                    Some(callee),
                                ) {
                                    return cloned;
                                }
                            }
                            let base = crate::codegen::rust::expression_utilities::borrow_base_expr(
                                arg_str,
                            );
                            if base.starts_with('&') {
                                return base.to_string();
                            }
                            return format!("&{base}");
                        }
                        return arg_str.to_string();
                    }
                }
                if let (Some(callee), Some(idx)) = (callee_name, arg_index) {
                    if !self.callee_arg_emits_owned_contract(callee, idx)
                        && self.callee_arg_expects_borrow_at_call(callee, idx)
                    {
                        if self.local_binding_reused_after_current_statement(name) {
                            if let Some(sig) =
                                self.signature_registry.get_signature(callee).or_else(|| {
                                    self.global_signature_registry
                                        .as_ref()
                                        .and_then(|g| g.get_signature(callee))
                                })
                            {
                                if let Some(cloned) = crate::codegen::rust::call_site_borrow::clone_reused_binding_for_owned_vec_formal(
                                    self,
                                    sig,
                                    idx,
                                    arg_expr,
                                    arg_str,
                                    Some(callee),
                                ) {
                                    return cloned;
                                }
                            }
                            let base = crate::codegen::rust::expression_utilities::borrow_base_expr(
                                arg_str,
                            );
                            if base.starts_with('&') {
                                return base.to_string();
                            }
                            return format!("&{base}");
                        }
                        return arg_str.to_string();
                    }
                }
                self.maybe_auto_clone(name, arg_str)
            }
            Expression::FieldAccess { .. } | Expression::Index { .. } => {
                if let (Some(callee), Some(idx)) = (callee_name, arg_index) {
                    if self.callee_param_field_extracts_by_name(callee, idx) {
                        return arg_str.to_string();
                    }
                    let emits_owned = self.callee_arg_emits_owned_contract(callee, idx);
                    if !emits_owned && self.callee_arg_expects_borrow_at_call(callee, idx) {
                        return arg_str.to_string();
                    }
                    if self.auto_clone_field_path_wants_at_call(
                        arg_expr,
                        Some(callee),
                        Some(idx),
                        emits_owned,
                    ) {
                        return self.maybe_auto_clone_expr_path(
                            arg_expr,
                            arg_str,
                            Some(callee),
                            Some(idx),
                        );
                    }
                    if !emits_owned
                        && (self.ir_callee_arg_expects_mut_borrow(
                            &self.signature_registry,
                            callee,
                            idx,
                            None,
                            None,
                        ) || self.global_signature_registry.as_ref().is_some_and(|g| {
                            self.ir_callee_arg_expects_mut_borrow(g, callee, idx, None, None)
                        }) || self.ir_callee_arg_expects_shared_borrow(
                            &self.signature_registry,
                            callee,
                            idx,
                            None,
                            None,
                        ) || self.global_signature_registry.as_ref().is_some_and(|g| {
                            self.ir_callee_arg_expects_shared_borrow(g, callee, idx, None, None)
                        }))
                    {
                        return arg_str.to_string();
                    }
                }
                self.maybe_auto_clone_expr_path(arg_expr, arg_str, callee_name, arg_index)
            }
            _ => arg_str.to_string(),
        }
    }

    /// True when auto-clone analysis says a field/index path needs `.clone()` at this call.
    /// `needs_clone_anywhere` applies only for owned formals (never borrow callees).
    fn auto_clone_field_path_wants_at_call(
        &self,
        arg_expr: &Expression<'ast>,
        callee_name: Option<&str>,
        arg_index: Option<usize>,
        emits_owned_formal: bool,
    ) -> bool {
        let Some(path) = Self::auto_clone_expr_path(arg_expr) else {
            return false;
        };
        if path == "None" || path.ends_with(".None") || path.ends_with("::None") {
            return false;
        }
        let Some(analysis) = self.auto_clone_analysis.as_ref() else {
            return false;
        };
        let emits_owned = emits_owned_formal
            || match (callee_name, arg_index) {
                (Some(callee), Some(idx)) => self.callee_arg_emits_owned_contract(callee, idx),
                _ => false,
            };
        if !emits_owned {
            if let (Some(callee), Some(idx)) = (callee_name, arg_index) {
                if self.callee_arg_expects_borrow_at_call(callee, idx) {
                    return false;
                }
            }
        }
        if analysis
            .needs_clone(&path, self.current_statement_idx)
            .is_some()
        {
            return emits_owned;
        }
        emits_owned && analysis.needs_clone_anywhere(&path)
    }

    /// Shared- or mut-borrow formal at this call (demoted `&str`, `&T`, …).
    pub(in crate::codegen::rust) fn callee_arg_expects_borrow_at_call(
        &self,
        callee: &str,
        arg_index: usize,
    ) -> bool {
        // Same-file demotion (`encode_line(todo: Todo)` → `todo: &Todo`) is recorded
        // on preregistered emitted formals before sibling bodies run.
        if self.preregistered_free_call_arg_expects_borrow(callee, arg_index) {
            return true;
        }
        self.ir_callee_arg_expects_shared_borrow(
            &self.signature_registry,
            callee,
            arg_index,
            None,
            None,
        ) || self.global_signature_registry.as_ref().is_some_and(|g| {
            self.ir_callee_arg_expects_shared_borrow(g, callee, arg_index, None, None)
        }) || self.ir_callee_arg_expects_mut_borrow(
            &self.signature_registry,
            callee,
            arg_index,
            None,
            None,
        ) || self.global_signature_registry.as_ref().is_some_and(|g| {
            self.ir_callee_arg_expects_mut_borrow(g, callee, arg_index, None, None)
        })
    }

    /// Clone a field/index path when auto-clone analysis recorded a move+reuse site.
    pub(crate) fn maybe_auto_clone_expr_path(
        &self,
        arg_expr: &Expression<'ast>,
        arg_str: &str,
        callee_name: Option<&str>,
        arg_index: Option<usize>,
    ) -> String {
        // WDB-367: unit `None` / enum `Value::None` must not pick up reuse `.clone()`.
        if arg_str == "None" || arg_str.ends_with("::None") {
            return arg_str.to_string();
        }
        if let (Some(callee), Some(idx)) = (callee_name, arg_index) {
            if self.callee_slot_emits_mut_borrow(callee, idx)
                || self.callee_arg_expects_borrow_at_call(callee, idx)
            {
                let mut kept = arg_str.to_string();
                if !crate::codegen::rust::expression_helpers::is_explicit_user_clone_call(arg_expr)
                {
                    crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut kept);
                }
                return kept;
            }
        }
        if arg_str.ends_with(".clone()") || arg_str.starts_with('*') {
            // Still rewrite clone → mem::take for call-arg writeback behind &mut self.
            if let Some(rewritten) = self.try_self_field_writeback_owned_arg(arg_expr, arg_str) {
                return rewritten;
            }
            return arg_str.to_string();
        }
        if let Some(rewritten) = self.try_self_field_writeback_owned_arg(arg_expr, arg_str) {
            return rewritten;
        }
        let needs =
            self.auto_clone_field_path_wants_at_call(arg_expr, callee_name, arg_index, false);
        if needs {
            // Copy values (scalars *and* aggregates like Coord/Vec3) are passed by
            // value — never `.coord.clone()` / `pc.clone()` (WDB-370/371). Non-Copy
            // enums (regression-063 Value) still clone on multi-use owned moves.
            let skip = self.infer_expression_type(arg_expr).is_some_and(|t| {
                let bare = match &t {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                self.is_type_copy(bare)
            }) || match arg_expr {
                Expression::Identifier { name, .. } => {
                    self.binding_is_copy_pass_by_value_scalar(name)
                }
                Expression::FieldAccess { field, .. } => self
                    .module_const_type_for_binding(field)
                    .is_some_and(|t| self.is_type_copy(t)),
                _ => false,
            };
            if !skip {
                // Clone yields an owned value. If a prior pass prefixed `&`, drop it
                // (`&stack.item.id.clone()` is `&String`, not `String`).
                let owned = if arg_str.starts_with("&mut ") {
                    arg_str
                } else {
                    arg_str.trim_start_matches('&')
                };
                return format!("{owned}.clone()");
            }
        }
        arg_str.to_string()
    }

    pub(in crate::codegen::rust) fn auto_clone_expr_path(
        expr: &Expression<'ast>,
    ) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, field, .. } => {
                Some(format!("{}.{}", Self::auto_clone_expr_path(object)?, field))
            }
            Expression::Index { object, index, .. } => {
                let base = Self::auto_clone_expr_path(object)?;
                let index_str = match index {
                    Expression::Literal {
                        value: Literal::Int(n),
                        ..
                    } => n.to_string(),
                    Expression::Identifier { name, .. } => name.clone(),
                    _ => "*".to_string(),
                };
                Some(format!("{base}[{index_str}]"))
            }
            _ => None,
        }
    }

    pub(crate) fn ir_sig_arg_expects_shared_borrow(
        &self,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
    ) -> bool {
        let pidx = sig.arg_param_index(arg_index);
        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx) {
            return false;
        }
        if crate::codegen::rust::call_signature_resolution::formal_is_plain_windjammer_string_for_call_arg(
            sig, arg_index,
        ) && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(sig, pidx)
        {
            return false;
        }
        if sig
            .emitted_rust_ref_params
            .as_ref()
            .and_then(|flags| flags.get(pidx))
            .copied()
            == Some(false)
        {
            return false;
        }
        if let Some(bare) = sig
            .formal_param_type(pidx)
            .or_else(|| sig.param_types.get(pidx))
            .map(|t| match t {
                Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                other => other,
            })
        {
            // Copy aggregates without confirmed shared-ref emission usually stay owned
            // (Lsn). Defer to the signature bridge so analyzer-Borrowed bare Custom that
            // the bridge still marks Ref (WDB-097 DenseCsr; also when field layout is
            // unknown and `is_type_copy` is a false positive) is not demoted to Identity.
            if self.is_type_copy(bare)
                && !crate::type_classification::is_copy_pass_by_value_formal(bare)
                && !crate::ir::emission_contract::callee_emits_shared_rust_ref_param(sig, pidx)
            {
                return crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, pidx);
            }
        }
        crate::ir::signature_bridge::call_site_expects_shared_borrow(sig, pidx)
    }

    fn ir_callee_arg_expects_shared_borrow(
        &self,
        registry: &SignatureRegistry,
        callee_name: &str,
        arg_index: usize,
        user_arg_count: Option<usize>,
        local_sig: Option<&crate::analyzer::FunctionSignature>,
    ) -> bool {
        let lookup_callee = self.signature_lookup_callee_name(callee_name);
        let lookup = lookup_callee.as_ref();
        // Preregistered tables are keyed by bare import alias (`glob_filter`), not qualified lookup.
        if self.preregistered_free_call_arg_expects_borrow(callee_name, arg_index) {
            return true;
        }
        if lookup != callee_name
            && self.preregistered_free_call_arg_expects_borrow(lookup, arg_index)
        {
            return true;
        }
        if self.preregistered_free_call_arg_emits_owned(callee_name, arg_index) {
            return false;
        }
        if lookup != callee_name && self.preregistered_free_call_arg_emits_owned(lookup, arg_index)
        {
            return false;
        }
        // Prefer emitted owned contracts over stale Borrowed analyzer/global stubs.
        if let Some(sig) = local_sig {
            if sig.name == callee_name
                || (!callee_name.contains("::")
                    && sig.name.rsplit("::").next() == callee_name.rsplit("::").next())
            {
                if self.ir_callee_arg_emits_owned_contract(
                    registry,
                    callee_name,
                    arg_index,
                    user_arg_count,
                    Some(sig),
                ) {
                    return false;
                }
                return self.ir_sig_arg_expects_shared_borrow(sig, arg_index);
            }
        }
        if self.ir_callee_arg_emits_owned_contract(
            registry,
            callee_name,
            arg_index,
            user_arg_count,
            None,
        ) {
            return false;
        }
        if let Some(sig) = registry.get_signature(lookup) {
            if self.ir_sig_arg_expects_shared_borrow(sig, arg_index) {
                return true;
            }
        }
        if let Some(global) = self.global_signature_registry.as_ref() {
            if let Some(sig) = global.get_signature(lookup) {
                if self.ir_sig_arg_expects_shared_borrow(sig, arg_index) {
                    return true;
                }
            }
            // Qualified callees must not fall back to bare simple-name globals
            // (homonym ownership from a different module).
            if !lookup.contains("::")
                && global
                    .lookup_method(lookup)
                    .is_some_and(|sig| self.ir_sig_arg_expects_shared_borrow(sig, arg_index))
            {
                return true;
            }
        }
        if !lookup.contains("::")
            && registry
                .lookup_method(lookup)
                .is_some_and(|sig| self.ir_sig_arg_expects_shared_borrow(sig, arg_index))
        {
            return true;
        }
        if let Some((rt, method)) = lookup.rsplit_once("::") {
            let arg_count = user_arg_count.unwrap_or(arg_index + 1);
            if self.method_registry_arg_expects_shared_borrow(rt, method, arg_index, arg_count) {
                return true;
            }
        }
        false
    }

    /// True when any resolved signature emits an owned (non-`&T`) formal for this arg.
    pub(in crate::codegen::rust) fn ir_callee_arg_emits_owned_contract(
        &self,
        registry: &SignatureRegistry,
        callee_name: &str,
        arg_index: usize,
        user_arg_count: Option<usize>,
        local_sig: Option<&crate::analyzer::FunctionSignature>,
    ) -> bool {
        let lookup_callee = self.signature_lookup_callee_name(callee_name);
        let lookup = lookup_callee.as_ref();
        if self.preregistered_free_call_arg_emits_owned(lookup, arg_index) {
            return true;
        }
        let check = |sig: &crate::analyzer::FunctionSignature| {
            Self::sig_arg_confirms_owned_emission(sig, arg_index)
        };
        // Call-resolved signatures are authoritative only when they match this callee.
        // Enclosing-fn homonyms (`pub fn write` while lowering `csv.write`) must not win.
        if let Some(sig) = local_sig {
            if sig.name == callee_name
                || (!callee_name.contains("::")
                    && sig.name.rsplit("::").next() == callee_name.rsplit("::").next())
            {
                return check(sig);
            }
        }
        if registry.get_signature(lookup).is_some_and(check) {
            return true;
        }
        if self
            .global_signature_registry
            .as_ref()
            .is_some_and(|g| g.get_signature(lookup).is_some_and(check))
        {
            return true;
        }
        // Imported bare calls (`use crate::html::escape_html` → `escape_html(...)`) register
        // as `html::escape_html` — suffix lookup for unqualified callees only.
        if !lookup.contains("::") && registry.lookup_method(lookup).is_some_and(check) {
            return true;
        }
        if !lookup.contains("::") {
            if self
                .global_signature_registry
                .as_ref()
                .is_some_and(|g| g.lookup_method(lookup).is_some_and(check))
            {
                return true;
            }
        }
        if let Some((rt, method)) = callee_name.rsplit_once("::") {
            let arg_count = user_arg_count.unwrap_or(arg_index + 1);
            if let Some(sig) = self.resolve_method_function_signature(rt, method, arg_count) {
                if check(&sig) {
                    return true;
                }
            }
        }
        false
    }

    fn ir_sig_arg_expects_mut_borrow(
        &self,
        sig: &crate::analyzer::FunctionSignature,
        arg_index: usize,
    ) -> bool {
        let pidx = sig.arg_param_index(arg_index);
        if crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx) {
            return false;
        }
        // Stale analyzer `MutableReference` on owned Copy aggregate formals (trait
        // `set_camera(camera: CameraData)`) must not force `&mut` at call sites when
        // defining-module emission kept an owned formal (no `function_emitted_mut_arg_indices` slot).
        // Live MutBorrowed (`apply_rotation(t: Transform)` → `t: &mut Transform`) must
        // still expect mut-borrow even before mut-index / MutableReference sync.
        if sig.formal_param_type(pidx).is_some_and(|formal| {
            !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
                && self.is_type_copy(formal)
                && !crate::type_classification::is_copy_pass_by_value_formal(formal)
        }) {
            let simple = sig.name.rsplit("::").next().unwrap_or(sig.name.as_str());
            let defining_module_emits_mut = self
                .function_emitted_mut_arg_indices
                .get(&sig.name)
                .or_else(|| self.function_emitted_mut_arg_indices.get(simple))
                .is_some_and(|indices| indices.contains(&arg_index));
            if !defining_module_emits_mut
                && !matches!(
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                        sig, pidx,
                    ),
                    crate::analyzer::OwnershipMode::MutBorrowed,
                )
            {
                return false;
            }
        }
        // Stale analyzer `MutableReference` on bare non-Copy Custom formals (MemoryEngine::put Key)
        // must not force `&mut` when defining-module emission did not record mut.
        if sig.formal_param_type(pidx).is_some_and(|formal| {
            matches!(formal, Type::Custom(_))
                && !self.is_type_copy(formal)
                && !matches!(formal, Type::Reference(_) | Type::MutableReference(_))
        }) {
            let simple = sig.name.rsplit("::").next().unwrap_or(sig.name.as_str());
            let defining_module_emits_mut = self
                .function_emitted_mut_arg_indices
                .get(&sig.name)
                .or_else(|| self.function_emitted_mut_arg_indices.get(simple))
                .is_some_and(|indices| indices.contains(&arg_index));
            if !defining_module_emits_mut
                && matches!(sig.param_types.get(pidx), Some(Type::MutableReference(_)))
            {
                return false;
            }
        }
        sig.param_types
            .get(pidx)
            .is_some_and(|t| matches!(t, Type::MutableReference(_)))
            || matches!(
                crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                    sig, arg_index,
                ),
                crate::analyzer::OwnershipMode::MutBorrowed,
            )
    }

    fn ir_callee_arg_expects_mut_borrow(
        &self,
        registry: &SignatureRegistry,
        callee_name: &str,
        arg_index: usize,
        user_arg_count: Option<usize>,
        local_sig: Option<&crate::analyzer::FunctionSignature>,
    ) -> bool {
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        // Field-forward / defining-module owned Custom formals never ask for `&mut`.
        let owned_from = |sig: &crate::analyzer::FunctionSignature| {
            let pidx = sig.arg_param_index(arg_index);
            crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx)
                || crate::codegen::rust::signature_promotion::bare_formal_is_owned_user_type(
                    sig, pidx,
                )
                || matches!(
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                        sig, arg_index,
                    ),
                    crate::analyzer::OwnershipMode::Owned,
                )
        };
        if local_sig.is_some_and(owned_from)
            || registry.get_signature(callee_name).is_some_and(owned_from)
            || registry.get_signature(simple).is_some_and(owned_from)
            || self.global_signature_registry.as_ref().is_some_and(|g| {
                g.get_signature(callee_name).is_some_and(owned_from)
                    || g.get_signature(simple).is_some_and(owned_from)
            })
        {
            return false;
        }
        if self
            .function_emitted_mut_arg_indices
            .get(callee_name)
            .or_else(|| {
                local_sig
                    .filter(|s| s.name.contains("::"))
                    .and_then(|s| self.function_emitted_mut_arg_indices.get(&s.name))
            })
            .is_some_and(|indices| indices.contains(&arg_index))
        {
            // Stale multipass slots can linger; prefer emitted owned contract.
            let owned_emitted = local_sig
                .map(|sig| {
                    let pidx = sig.arg_param_index(arg_index);
                    crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(sig, pidx)
                })
                .or_else(|| {
                    registry.get_signature(callee_name).map(|sig| {
                        let pidx = sig.arg_param_index(arg_index);
                        crate::codegen::rust::signature_promotion::emitted_owned_arg_contract(
                            sig, pidx,
                        )
                    })
                })
                .unwrap_or(false);
            if !owned_emitted {
                return true;
            }
        }
        if let Some(sig) = local_sig {
            if self.ir_sig_arg_expects_mut_borrow(sig, arg_index) {
                return true;
            }
        }
        if let Some(sig) = registry.get_signature(callee_name) {
            if self.ir_sig_arg_expects_mut_borrow(sig, arg_index) {
                return true;
            }
        }
        if let Some(global) = self.global_signature_registry.as_ref() {
            for key in [callee_name, simple] {
                if let Some(sig) = global.get_signature(key) {
                    if self.ir_sig_arg_expects_mut_borrow(sig, arg_index) {
                        return true;
                    }
                }
            }
        }
        if let Some((rt, method)) = callee_name.rsplit_once("::") {
            let arg_count = user_arg_count.unwrap_or(arg_index + 1);
            if let Some(sig) = self.resolve_method_function_signature(rt, method, arg_count) {
                if self.ir_sig_arg_expects_mut_borrow(&sig, arg_index) {
                    return true;
                }
            }
        }
        false
    }

    /// Same-file preregistered Rust formals beat stale homonym borrow metadata on `sig`
    /// (`join(base, relative)` vs `strings::join` scanner baseline).
    pub(in crate::codegen::rust) fn sync_call_sig_from_preregistered_free_fn_emission(
        &self,
        callee_name: &str,
        sig: &mut crate::analyzer::FunctionSignature,
    ) {
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let Some(formals) = [callee_name, simple]
            .iter()
            .find_map(|key| self.preregistered_free_function_emitted_params.get(*key))
        else {
            return;
        };
        let mut emitted_flags = sig
            .emitted_rust_ref_params
            .clone()
            .unwrap_or_else(|| vec![false; sig.param_ownership.len()]);
        while emitted_flags.len() < sig.param_ownership.len() {
            emitted_flags.push(false);
        }
        for (arg_index, formal) in formals.iter().enumerate() {
            let pidx = sig.arg_param_index(arg_index);
            if pidx >= sig.param_ownership.len() {
                continue;
            }
            let emitted_mut = formal.contains(": &mut ") || formal.contains(": &'a mut ");
            let shared = (formal.contains(": &") || formal.contains(": &'a ")) && !emitted_mut;
            emitted_flags[pidx] = shared;
            if emitted_mut {
                // `t: &mut Transform` is a live mut-borrow formal, not owned `mut t: T`.
                sig.param_ownership[pidx] = crate::analyzer::OwnershipMode::MutBorrowed;
                if let Some(ty) = sig
                    .formal_param_type(pidx)
                    .or_else(|| sig.param_types.get(pidx))
                    .cloned()
                {
                    let bare = match &ty {
                        crate::parser::Type::Reference(inner)
                        | crate::parser::Type::MutableReference(inner) => inner.as_ref().clone(),
                        other => other.clone(),
                    };
                    let mut_ty = crate::parser::Type::MutableReference(Box::new(bare.clone()));
                    sig.param_types[pidx] = mut_ty.clone();
                    while sig.formal_param_types.len() <= pidx {
                        sig.formal_param_types.push(bare.clone());
                    }
                    sig.formal_param_types[pidx] = mut_ty;
                }
            } else if shared {
                sig.param_ownership[pidx] = crate::analyzer::OwnershipMode::Borrowed;
                if crate::codegen::rust::types::is_windjammer_text_type(
                    sig.formal_param_type(pidx)
                        .or_else(|| sig.param_types.get(pidx))
                        .unwrap_or(&crate::parser::Type::String),
                ) && (formal.contains(": &str") || formal.ends_with(": &str"))
                {
                    sig.param_types[pidx] = crate::parser::Type::Reference(Box::new(
                        crate::parser::Type::Custom("str".into()),
                    ));
                } else if let Some(bare) = sig
                    .formal_param_type(pidx)
                    .or_else(|| sig.param_types.get(pidx))
                    .map(|t| match t {
                        crate::parser::Type::Reference(inner)
                        | crate::parser::Type::MutableReference(inner) => inner.as_ref().clone(),
                        other => other.clone(),
                    })
                {
                    sig.param_types[pidx] = crate::parser::Type::Reference(Box::new(bare));
                }
            } else {
                sig.param_ownership[pidx] = crate::analyzer::OwnershipMode::Owned;
                if let Some(ty) = sig
                    .formal_param_type(pidx)
                    .or_else(|| sig.param_types.get(pidx))
                    .cloned()
                {
                    let bare = match &ty {
                        crate::parser::Type::Reference(inner)
                        | crate::parser::Type::MutableReference(inner) => inner.as_ref().clone(),
                        other => other.clone(),
                    };
                    sig.param_types[pidx] = bare.clone();
                    while sig.formal_param_types.len() <= pidx {
                        sig.formal_param_types.push(bare.clone());
                    }
                    sig.formal_param_types[pidx] = bare;
                }
            }
        }
        sig.emitted_rust_ref_params = Some(emitted_flags);
    }
}

/// Detect ownership already encoded in generated Rust (e.g. prior phases emitted `&x`).
fn ownership_from_rust_expr(expr: &str) -> Option<OwnedType> {
    let trimmed = expr.trim();
    if trimmed.starts_with("&mut ") {
        Some(OwnedType::MutRef(Region::fresh(1)))
    } else if trimmed.starts_with('&') {
        Some(OwnedType::Ref(Region::fresh(0)))
    } else if trimmed.starts_with('(') && trimmed.ends_with(')') {
        ownership_from_rust_expr(&trimmed[1..trimmed.len() - 1])
    } else if trimmed.ends_with(".to_string()") || trimmed.ends_with(".to_owned()") {
        Some(OwnedType::Owned)
    } else {
        None
    }
}

fn type_pointee_base(ty: &Type) -> BaseType {
    match ty {
        Type::Reference(inner) | Type::MutableReference(inner) => {
            crate::ir::node::parser_type_to_base_type(inner)
        }
        other => crate::ir::node::parser_type_to_base_type(other),
    }
}

fn safety_type_from_arg_expression(expr: &Expression) -> SafetyType {
    match expr {
        Expression::Literal { value, .. } => match value {
            Literal::String(_) => SafetyType::borrowed(BaseType::String, Region::fresh(2)),
            Literal::Int(_) => SafetyType::copy(BaseType::I32),
            Literal::Float(_) => SafetyType::copy(BaseType::F32),
            Literal::Bool(_) => SafetyType::copy(BaseType::Bool),
            _ => SafetyType::owned(BaseType::Inferred),
        },
        Expression::Identifier { name, .. } => {
            if name.starts_with('"') {
                SafetyType::borrowed(BaseType::String, Region::fresh(2))
            } else {
                SafetyType::owned(BaseType::Inferred)
            }
        }
        Expression::FieldAccess { .. } => {
            SafetyType::borrowed(BaseType::Inferred, Region::fresh(3))
        }
        Expression::Closure { .. } => SafetyType::owned(BaseType::Custom("FnOnce".into())),
        Expression::Binary {
            op: crate::parser::BinaryOp::Or,
            left,
            ..
        } if matches!(
            &**left,
            Expression::Identifier { name, .. } if name == "move"
        ) =>
        {
            SafetyType::owned(BaseType::Custom("FnOnce".into()))
        }
        _ => SafetyType::owned(BaseType::Inferred),
    }
}

#[cfg(test)]
mod ir_total_tests {
    use super::*;
    use crate::analyzer::{Analyzer, OwnershipMode};
    use crate::codegen::rust::{CodeGenerator, IrCutoverConfig};
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::CompilationTarget;

    #[test]
    fn apply_ir_returns_some_when_call_sites_on_for_known_callee() {
        let source = r#"
fn takes_borrowed(s: string) {}
fn main() {
    let x = "hi"
    takes_borrowed(x)
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let mut analyzer = Analyzer::new();
        let (analyzed, mut registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let _ = analyzed;
        // Ensure Borrowed formal is visible to IR.
        let sig = registry
            .signatures
            .get_mut("takes_borrowed")
            .expect("takes_borrowed signature from analyzer");
        sig.param_ownership = vec![OwnershipMode::Borrowed];
        sig.emitted_rust_ref_params = Some(vec![true]);

        let mut gen = CodeGenerator::new(registry.clone(), CompilationTarget::Rust);
        gen.ir_cutover = IrCutoverConfig {
            ownership: true,
            clones: true,
            param_types: true,
            str_ref: true,
            call_sites: true,
            locals: true,
        };

        let call_arg = program
            .items
            .iter()
            .find_map(|item| {
                if let crate::parser::Item::Function { decl, .. } = item {
                    if decl.name != "main" {
                        return None;
                    }
                    decl.body.iter().find_map(|stmt| {
                        if let crate::parser::Statement::Expression { expr, .. } = stmt {
                            if let Expression::Call { arguments, .. } = expr {
                                return arguments.first().map(|(_, a)| *a);
                            }
                        }
                        None
                    })
                } else {
                    None
                }
            })
            .expect("call arg");

        let coerced = gen.apply_ir_call_site_coercion(
            &registry,
            "takes_borrowed",
            0,
            call_arg,
            "x",
            registry.get_signature("takes_borrowed"),
            None,
            Some(1),
        );
        assert!(
            coerced.is_some(),
            "IR must always coerce known callees when call_sites is on"
        );
    }

    #[test]
    fn module_boundary_callee_detection() {
        assert!(CodeGenerator::is_module_boundary_callee(
            "unknown_crate::missing_api"
        ));
        assert!(CodeGenerator::is_module_boundary_callee(
            "wal::replay_to_lsn"
        ));
        assert!(!CodeGenerator::is_module_boundary_callee("HashMap::new"));
        assert!(!CodeGenerator::is_module_boundary_callee("Self::new"));
        assert!(!CodeGenerator::is_module_boundary_callee("plain_fn"));
        // Same-crate paths resolve via bare-name registry entries (WDB-094).
        assert!(!CodeGenerator::is_module_boundary_callee(
            "crate::circuit_row::copy_rows"
        ));
        assert!(!CodeGenerator::is_module_boundary_callee(
            "crate::vec_map::vec_map_get_f64"
        ));
    }

    #[test]
    fn demoted_str_formal_strips_stale_clone_before_borrow() {
        let source = r#"
fn takes_str(json: string) -> string { json }
fn dispatch(json: string) -> string {
    if takes_str(json) != "" { json } else { "" }
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let mut analyzer = Analyzer::new();
        let (_, mut registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let sig = registry
            .signatures
            .get_mut("takes_str")
            .expect("takes_str signature");
        sig.param_ownership = vec![OwnershipMode::Borrowed];
        sig.emitted_rust_ref_params = Some(vec![true]);
        sig.param_types = vec![Type::Reference(Box::new(Type::Custom("str".into())))];

        let mut gen = CodeGenerator::new(registry.clone(), CompilationTarget::Rust);
        gen.ir_cutover = IrCutoverConfig {
            ownership: true,
            clones: true,
            param_types: true,
            str_ref: true,
            call_sites: true,
            locals: true,
        };
        gen.current_function_params = vec![crate::parser::Parameter {
            name: "json".into(),
            pattern: None,
            type_: Type::String,
            ownership: crate::parser::OwnershipHint::Inferred,
            is_mutable: false,
            decorators: vec![],
        }];
        gen.in_if_condition = true;

        let call_arg = program
            .items
            .iter()
            .find_map(|item| {
                if let crate::parser::Item::Function { decl, .. } = item {
                    if decl.name != "dispatch" {
                        return None;
                    }
                    decl.body.iter().find_map(|stmt| {
                        if let crate::parser::Statement::If { condition, .. } = stmt {
                            if let Expression::Binary { left, .. } = condition {
                                if let Expression::Call { arguments, .. } = &**left {
                                    return arguments.first().map(|(_, a)| *a);
                                }
                            }
                        }
                        None
                    })
                } else {
                    None
                }
            })
            .expect("call arg in if condition");

        let coerced = gen
            .apply_ir_call_site_coercion(
                &registry,
                "takes_str",
                0,
                call_arg,
                "json.clone()",
                registry.get_signature("takes_str"),
                None,
                Some(1),
            )
            .expect("IR coercion");
        assert!(
            !coerced.contains(".clone()"),
            "demoted &str formal must not keep stale clone, got: {coerced}"
        );
        assert!(
            coerced.starts_with('&'),
            "demoted &str formal must borrow owned caller param, got: {coerced}"
        );
    }
}
