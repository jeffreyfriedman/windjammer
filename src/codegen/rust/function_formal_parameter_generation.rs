//! Formal parameter list emission (excluding implicit `self` receiver).

use std::collections::HashSet;

use crate::analyzer::*;
use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn compute_unused_formal_parameter_names(
        &self,
        func: &FunctionDecl<'ast>,
    ) -> HashSet<String> {
        let body_refs: Vec<&Statement> = func.body.to_vec();
        func.parameters
            .iter()
            .filter(|p| p.name != "self")
            .filter(|p| {
                let used_in_body = Self::variable_used_in_statements(&body_refs, &p.name);
                let used_in_decorators = func.decorators.iter().any(|d| {
                    d.arguments
                        .iter()
                        .any(|(_, expr)| Self::variable_used_in_expression(expr, &p.name))
                });
                !used_in_body && !used_in_decorators
            })
            .map(|p| p.name.clone())
            .collect()
    }

    pub(in crate::codegen::rust) fn refresh_unused_let_bindings_for_function_body(
        &mut self,
        body: &[&'ast Statement<'ast>],
    ) {
        // TDD FIX: Pre-compute unused let bindings and for-loop variables.
        // Like unused params, these get prefixed with `_` in the generated Rust.
        self.unused_let_bindings.clear();
        Self::find_unused_bindings(body, &mut self.unused_let_bindings);
    }

    pub(in crate::codegen::rust) fn collect_additional_formal_parameter_strings(
        &mut self,
        analyzed: &AnalyzedFunction<'ast>,
        func: &FunctionDecl<'ast>,
        needs_lifetime: bool,
        unused_params: &HashSet<String>,
    ) -> Vec<String> {
        let body_modifies = if self.in_trait_impl {
            self.function_modifies_self(&analyzed.decl)
        } else {
            self.function_modifies_self_or_derived(&analyzed.decl)
        };
        func.parameters
            .iter()
            .enumerate()
            .map(|(param_idx, param)| {
                let payload_stored = self.param_stored_in_owned_payload(
                    func.body.as_slice(),
                    &param.name,
                );
                let moves_via_struct_init = self.param_moves_via_struct_literal_init(
                    func.body.as_slice(),
                    &param.name,
                );
                // Store-only `Vec<u8>` formals (`from_bytes(bytes)` → `WalSegment { bytes }`)
                // emit `&Vec<u8>` + `.clone()` so FFI snapshots can borrow at the call site
                // (regression-049). `Vec<String>` / other element types stay Owned when stored in a
                // field (`with_items(items)` → `node.items = items`).
                let vec_store_borrow_ok = (payload_stored || moves_via_struct_init)
                    && Self::param_type_is_byte_vec(&param.type_)
                    && !self.param_has_owning_method_use(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    );
                let payload_forces_owned = payload_stored && !vec_store_borrow_ok;

                // Readonly Vec formals: analyzer-converged Borrowed + `.len()`/index reads → `&Vec<T>`.
                // Runs before owned-type forcing so declaration-stub/registry Owned cannot block demotion.
                if param.name != "self"
                    && Self::param_type_is_vec_container(&param.type_)
                    && !payload_forces_owned
                    && !self.param_consumed_as_for_loop_iterable(func.body.as_slice(), &param.name)
                    && self.param_has_readonly_expression_use(func.body.as_slice(), &param.name)
                    && matches!(
                        analyzed.inferred_ownership.get(&param.name),
                        Some(OwnershipMode::Borrowed)
                    )
                {
                    let type_str =
                        self.borrowed_formal_rust_type_for_param(param, func, param_idx);
                    self.emitted_rust_ref_formals.insert(param.name.clone());
                    self.inferred_borrowed_params.insert(param.name.clone());
                    return format!("{}: {}", param.name, type_str);
                }

                // Readonly Custom aggregate formals: analyzer Borrowed → `&T` (get_sum node, LsmEngine key).
                let multiparam_store_keeps_owned_early = self
                    .param_multiparam_store_keeps_owned_key_formal(param, func);
                let tuple_discard_keeps_owned_early = self
                    .param_only_used_in_discarding_let_binding(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && !self.param_only_used_as_bare_id_in_discarding_tuple(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    );
                if param.name != "self"
                    && matches!(&param.type_, Type::Custom(_))
                    && !crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
                    && !self.is_type_copy(&param.type_)
                    && !payload_forces_owned
                    && !multiparam_store_keeps_owned_early
                    && !tuple_discard_keeps_owned_early
                    && !self.param_single_arg_owned_self_or_field_forward(param, func)
                    && !self.param_has_field_or_index_move_binding(
                        func.body.as_slice(),
                        &param.name,
                    )
                    && matches!(
                        analyzed.inferred_ownership.get(&param.name),
                        Some(OwnershipMode::Borrowed)
                    )
                    && self.inferred_borrowed_params.contains(&param.name)
                {
                    let type_str =
                        self.borrowed_formal_rust_type_for_param(param, func, param_idx);
                    self.emitted_rust_ref_formals.insert(param.name.clone());
                    return format!("{}: {}", param.name, type_str);
                }

                // Map/set key lookup formals (`self.quests.get(id)`) demote to `&Key` before
                // single-arg owned forward — stale engine metadata Owned must not beat borrow.
                let field_move_forces_owned_early = self.param_has_field_or_index_move_binding(
                    func.body.as_slice(),
                    &param.name,
                );
                let map_key_borrow_forward_early = param.name != "self"
                    && matches!(&param.type_, Type::Custom(_))
                    && !self.is_type_copy(&param.type_)
                    && !crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
                    && self.param_only_forwarded_to_map_key_callee(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && !self.param_multiparam_store_keeps_owned_key_formal(param, func)
                    && !payload_forces_owned
                    && !field_move_forces_owned_early;
                if map_key_borrow_forward_early {
                    let type_str =
                        self.borrowed_formal_rust_type_for_param(param, func, param_idx);
                    self.emitted_rust_ref_formals.insert(param.name.clone());
                    self.inferred_borrowed_params.insert(param.name.clone());
                    self.inferred_mut_borrowed_params.remove(&param.name);
                    return format!("{}: {}", param.name, type_str);
                }

                if param.name != "self" && self.param_single_arg_owned_self_or_field_forward(param, func) {
                    return format!("{}: {}", param.name, self.type_to_rust(&param.type_));
                }

                // SMART STRING INFERENCE: Prefer IR-confirmed param types when cutover is on.
                let inferred_type = if payload_forces_owned
                    || matches!(
                        analyzed.inferred_ownership.get(&param.name),
                        Some(OwnershipMode::Owned)
                    )
                    || self.param_ownership_for_formal_demotion(&param.name, analyzed)
                        == Some(OwnershipMode::Owned)
                {
                    &param.type_
                } else {
                    self.get_effective_param_type(param_idx, param, analyzed)
                };

                // E0053: Trait impl formal parameters must match the trait item. Plain `string` in
                // source is owned `String` — do not emit `&str` from str_ref inference on the impl.
                let is_owned_string_decl = matches!(&param.type_, Type::String)
                    || matches!(&param.type_, Type::Custom(name) if name == "string");

                let converged_analyzer_borrow = matches!(
                    analyzed.inferred_ownership.get(&param.name),
                    Some(OwnershipMode::Borrowed)
                ) || analyzed.str_ref_optimizable_params.contains(&param.name);

                let field_move_forces_owned = self.param_has_field_or_index_move_binding(
                    func.body.as_slice(),
                    &param.name,
                );
                let multiparam_store_keeps_owned = self
                    .param_multiparam_store_keeps_owned_key_formal(param, func);
                let tuple_discard_keeps_owned = self
                    .param_only_used_in_discarding_let_binding(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && !self.param_only_used_as_bare_id_in_discarding_tuple(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && !(Self::param_type_is_vec_container(&param.type_)
                        && self.param_has_readonly_expression_use(
                            func.body.as_slice(),
                            &param.name,
                        )
                        && !self.param_has_owning_method_use(
                            func.body.as_slice(),
                            &param.name,
                            func,
                        ));
                let map_key_borrow_forward = matches!(&param.type_, Type::Custom(_))
                    && !self.is_type_copy(&param.type_)
                    && !crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
                    && self.param_only_forwarded_to_map_key_callee(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && !multiparam_store_keeps_owned
                    && !payload_forces_owned
                    && !field_move_forces_owned;
                // Map/set key lookup formals (`self.quests.get(id)`) demote to `&Key` even when
                // stale engine metadata marks the param Owned (`QuestManager::is_quest_active`).
                if param.name != "self" && map_key_borrow_forward {
                    let type_str =
                        self.borrowed_formal_rust_type_for_param(param, func, param_idx);
                    self.emitted_rust_ref_formals.insert(param.name.clone());
                    self.inferred_borrowed_params.insert(param.name.clone());
                    self.inferred_mut_borrowed_params.remove(&param.name);
                    return format!("{}: {}", param.name, type_str);
                }
                let field_proj_readonly = self.param_only_used_via_field_or_index_projection(
                    func.body.as_slice(),
                    &param.name,
                ) && !field_move_forces_owned
                && !multiparam_store_keeps_owned
                && !tuple_discard_keeps_owned
                && !self.param_has_owning_method_use(
                    func.body.as_slice(),
                    &param.name,
                    func,
                ) && !payload_forces_owned
                    // Field/index *writes* still look like projection-only; exclude mutated
                    // / MutBorrowed formals so they stay `&mut T` (not shared `&T`).
                    && !matches!(
                        analyzed.inferred_ownership.get(&param.name),
                        Some(OwnershipMode::MutBorrowed)
                    )
                    && !(analyzed.mutated_parameters.contains(&param.name)
                        && !analyzed.returned_parameters.contains(&param.name));
                // Port-trait owned `string` forwards keep caller `String` (E0053 / deps tests).
                // Skip when the body only forwards to readonly text callees (`find_index`) —
                // those siblings emit `&str` and the outer formal should match (blackboard set_*).
                if param.name != "self"
                    && crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
                    && !self.in_trait_impl
                    && self.param_passes_to_wj_owned_sibling_call(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && !self.param_only_forwards_to_borrowed_text_callees(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                {
                    self.str_ref_optimized_params.remove(&param.name);
                    self.inferred_borrowed_params.remove(&param.name);
                    return format!("{}: {}", param.name, self.type_to_rust(&param.type_));
                }
                // Blackboard-style keys: forward only to readonly `&str` callees (`find_index`).
                // Runtime AsRef modules (`strings::substring`, `db::connect`, …) keep owned
                // WJ `string` so callers pass by value (CSV while-index / std_db gates).
                if param.name != "self"
                    && crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
                    && !self.in_trait_impl
                    && self.param_only_forwards_to_borrowed_text_callees(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && !self.param_asref_runtime_forces_owned_formal(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                {
                    self.str_ref_optimized_params.insert(param.name.clone());
                    self.inferred_borrowed_params.insert(param.name.clone());
                    self.inferred_mut_borrowed_params.remove(&param.name);
                    return format!("{}: &str", param.name);
                }
                let borrow_delegation = self.param_should_emit_borrowed_delegation_formal(param, func)
                    && !self.param_single_arg_owned_self_or_field_forward(param, func);
                // Shared `&T` is reusable across multiple borrowing call sites
                // (`FpsCamera::update` → `depenetrate` + `collides`). Multi-stmt only
                // forces Owned when the param is moved into owning callees / kept owned.
                let multi_stmt_shared_borrow_ok = self
                    .inferred_borrowed_params
                    .contains(&param.name)
                    && self.param_only_used_as_call_argument(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && self.param_passed_to_borrowing_callee(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && !self.param_passes_to_wj_owned_sibling_call(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    );
                // Multiparam store-forward borrow (`apply_patch_put` → `&Key` + clone) must
                // not undo real payload stores (`self.components.push(component)`, enum
                // variants). Those stay Owned.
                let payload_forces_owned = payload_forces_owned
                    && (!borrow_delegation || payload_stored);
                // Runtime AsRef modules keep owned `string` + call-site borrow only when
                // callees do not already expect a borrow (fs AsRef<Path> demotes).
                let asref_runtime_keep_owned = crate::codegen::rust::types::is_windjammer_text_type(
                    &param.type_,
                ) && !analyzed.str_ref_optimizable_params.contains(&param.name)
                    && self.param_asref_runtime_forces_owned_formal(
                    func.body.as_slice(),
                    &param.name,
                    func,
                );
                if asref_runtime_keep_owned {
                    self.str_ref_optimized_params.remove(&param.name);
                    self.inferred_borrowed_params.remove(&param.name);
                }
                // Analyzer Owned must not block demotion for field-projection-only Custom
                // params, store-only Vec constructors, or borrowed-delegation formals.
                let demotion_ownership =
                    self.param_ownership_for_formal_demotion(&param.name, analyzed);
                let analyzer_or_ir_owned = payload_forces_owned
                    || field_move_forces_owned
                    || multiparam_store_keeps_owned
                    || tuple_discard_keeps_owned
                    || asref_runtime_keep_owned
                    || (matches!(demotion_ownership, Some(OwnershipMode::Owned))
                        && !field_proj_readonly
                        && !vec_store_borrow_ok
                        && !borrow_delegation
                        && !map_key_borrow_forward
                        // Codegen promoted to borrow via callee forwarding — do not let
                        // stale analyzer Owned block `&T` formals (static method passthrough).
                        && !self.inferred_borrowed_params.contains(&param.name));
                if param.name != "self"
                    && !analyzer_or_ir_owned
                    && !self.param_single_arg_owned_self_or_field_forward(param, func)
                    && !self.in_trait_impl
                    && !self.param_must_not_demote_to_shared_borrow(&param.name, analyzed, None)
                    && !analyzed.field_extract_parameters.contains(&param.name)
                    && !analyzed.returned_parameters.contains(&param.name)
                    // Multiparam store forwards (`apply_patch_put` → `apply_put`) store via
                    // owned callees; still emit `&Key` + clone (regression-047 full store layout).
                    && (!(moves_via_struct_init && !vec_store_borrow_ok) || borrow_delegation)
                    && !self.is_type_copy(&param.type_)
                    && !matches!(
                        &param.type_,
                        Type::Reference(_) | Type::MutableReference(_)
                    )
                    && !self.func_is_pure_forwarding_delegate(func)
                    // Multiple call sites block demotion only when the param must stay
                    // Owned (moved into several owning callees). Analyzer Borrowed / codegen
                    // promote of pure borrowing forwards means shared `&T` — reusable across
                    // `FpsCamera::depenetrate` + `collides`.
                    && (!self.param_passed_from_multiple_statements(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    ) || borrow_delegation
                        || converged_analyzer_borrow
                        || multi_stmt_shared_borrow_ok)
                    // Multiparam store forwards (`apply_patch_put`) compute borrow_delegation
                    // despite nested if/else uses — do not let forward-ref keep-owned block
                    // the demotion that borrow_delegation already approved (dogfood full store).
                    && (!self.param_has_forward_ref_keep_owned(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    ) || borrow_delegation)
                    && (!self.current_fn_forward_ref_if_params.contains(&param.name)
                        || borrow_delegation)
                    && (!self.current_fn_mixed_forwarder_params.contains(&param.name)
                        || borrow_delegation)
                    && (!self.param_passes_to_wj_owned_sibling_call(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    ) || borrow_delegation)
                    // Key-facade keep-owned must not block asymmetric field-projection
                    // params (`value.data.len()` → `&Value` beside owned Key) or
                    // borrowed-delegation formals. Avoid `param_keeps_owned_*` here — it
                    // re-enters `param_should_emit_borrowed_delegation_formal`.
                    && (field_proj_readonly
                        || borrow_delegation
                        || !self.current_struct_name.as_ref().is_some_and(|sn| {
                            self.struct_is_owned_engine_key_facade(sn, param)
                        }))
                    && !self.is_collection_key_owned_param(param, func)
                    && !self.param_only_used_in_discarding_let_binding(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    && (borrow_delegation
                        // Analyzer Borrowed: text, Copy *aggregates*, and Custom types
                        // forwarded to borrowing callees (FpsCamera::update grid → &VoxelGrid).
                        // Never demote Copy scalars (`int`/`float`/`bool`) — literals must
                        // call by value (`status_html(2, 1)`), not `&i64`.
                        // Trait impls / payload stores / key facades are gated above.
                        || (converged_analyzer_borrow
                            && (crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
                                || (self.is_type_copy(&param.type_)
                                    && !crate::type_classification::is_copy_pass_by_value_formal(
                                        &param.type_,
                                    )
                                    && field_proj_readonly)
                                || (matches!(&param.type_, Type::Custom(_))
                                    && !self.is_type_copy(&param.type_))
                                || Self::param_type_is_vec_container(&param.type_)))
                        || field_proj_readonly
                        || map_key_borrow_forward
                        || vec_store_borrow_ok
                        || (self.inferred_borrowed_params.contains(&param.name)
                            && !self.param_is_single_arg_call_only_delegate(param, func)
                            && (!self.param_passed_from_multiple_statements(
                                func.body.as_slice(),
                                &param.name,
                                func,
                            ) || converged_analyzer_borrow
                                || multi_stmt_shared_borrow_ok)))
                {
                    let type_str =
                        self.borrowed_formal_rust_type_for_param(param, func, param_idx);
                    if std::env::var("WJ_DEBUG_FORMAL").is_ok() && param.name != "self" {
                        eprintln!("[FORMAL-BORROW] fn={} param={} type_str={}", func.name, param.name, type_str);
                    }
                    self.emitted_rust_ref_formals.insert(param.name.clone());
                    self.inferred_borrowed_params.insert(param.name.clone());
                    self.inferred_mut_borrowed_params.remove(&param.name);
                    if type_str == "&str"
                        || type_str.starts_with("&'a str")
                        || type_str.ends_with(" str")
                    {
                        self.str_ref_optimized_params.insert(param.name.clone());
                    }
                    return format!("{}: {}", param.name, type_str);
                }

                let formal_type: &Type = if asref_runtime_keep_owned
                    && param.name != "self"
                {
                    &param.type_
                } else if self.in_trait_impl
                    && param.name != "self"
                    && is_owned_string_decl
                {
                    &param.type_
                } else if param.name != "self"
                    && self.is_collection_key_owned_param(param, func)
                {
                    &param.type_
                } else if param.name != "self"
                    && matches!(inferred_type, Type::Reference(_))
                    && !matches!(
                        &param.type_,
                        Type::Reference(_) | Type::MutableReference(_)
                    )
                    && self.is_type_copy(&param.type_)
                    && !self.inferred_borrowed_params.contains(&param.name)
                {
                    // Copy aggregate: spurious Reference from ref analysis — emit by-value
                    // unless field-enum-match kept an active borrow (bug_copy_vec3).
                    &param.type_
                } else if param.name != "self"
                    && matches!(inferred_type, Type::Reference(_))
                    && !matches!(
                        &param.type_,
                        Type::Reference(_) | Type::MutableReference(_)
                    )
                    && !self.is_type_copy(&param.type_)
                    && self.param_passed_to_owned_self_method_arg(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                {
                    &param.type_
                } else if param.name != "self"
                    && matches!(inferred_type, Type::Reference(_))
                    && !matches!(
                        &param.type_,
                        Type::Reference(_) | Type::MutableReference(_)
                    )
                    && !self.is_type_copy(&param.type_)
                    && self.param_only_used_in_discarding_let_binding(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                {
                    &param.type_
                } else if param.name != "self"
                    && matches!(inferred_type, Type::Reference(_))
                    && !matches!(
                        &param.type_,
                        Type::Reference(_) | Type::MutableReference(_)
                    )
                    && !self.is_type_copy(&param.type_)
                    && (self.param_only_forwards_to_emitted_owned_callees(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    ) || self.current_struct_name.as_ref().is_some_and(|sn| {
                        self.param_keeps_owned_engine_key_facade(sn, param, func)
                    }))
                {
                    &param.type_
                } else if param.name != "self"
                    && !matches!(
                        &param.type_,
                        Type::Reference(_) | Type::MutableReference(_)
                    )
                    && !self.is_type_copy(&param.type_)
                    && (self.param_has_forward_ref_keep_owned(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    ) || self.current_fn_mixed_forwarder_params.contains(&param.name)
                    || self.current_fn_forward_ref_if_params.contains(&param.name)
                    || self.param_passes_to_wj_owned_sibling_call(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    )
                    || self.param_passed_from_multiple_statements(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    ))
                {
                    &param.type_
                } else {
                    inferred_type
                };
                let trait_impl_owned_string =
                    self.in_trait_impl && param.name != "self" && is_owned_string_decl;

                // PHASE 9 OPTIMIZATION: Check if this parameter should use Cow<'_, T>
                if self.cow_optimizations.contains(&param.name) {
                    let base_type = self.type_to_rust(formal_type);
                    // For String types, use Cow<'_, str>
                    let cow_type = if base_type == "String" {
                        "Cow<'_, str>".to_string()
                    } else {
                        format!("Cow<'_, {}>", base_type)
                    };
                    return format!("{}: {}", param.name, cow_type);
                }

                if param.name == "self"
                    && !self.in_trait_impl
                    && super::self_analysis::function_return_moves_self_fields(&analyzed.decl)
                    && body_modifies
                    && self.method_returns_impl_struct(func)
                {
                    self.inferred_borrowed_params.remove("self");
                    self.inferred_mut_borrowed_params.remove("self");
                    self.record_self_receiver_upgrade(
                        &func.name,
                        self.get_param_ownership("self", analyzed),
                        "mut self",
                    );
                    return "mut self".to_string();
                }

                if param.name == "self"
                    && !self.in_trait_impl
                    && !super::self_analysis::function_calls_owned_self_method(
                        &analyzed.decl,
                        &self.signature_registry,
                        self.current_struct_name.as_deref(),
                    )
                    && !matches!(
                        self.get_effective_self_ownership(&func.name, analyzed),
                        Some(OwnershipMode::Owned)
                    )
                    && self.function_calls_self_with_recorded_receiver(
                        &analyzed.decl,
                        OwnershipMode::MutBorrowed,
                    )
                {
                    self.inferred_mut_borrowed_params.insert("self".to_string());
                    self.inferred_borrowed_params.remove("self");
                    self.record_self_receiver_upgrade(
                        &func.name,
                        self.get_param_ownership("self", analyzed),
                        "&mut self",
                    );
                    return "&mut self".to_string();
                }

                // Self-delegation fallback: if the fixed-point pre-pass recorded this
                // method as MutBorrowed (via transitive delegation), use &mut self.
                if param.name == "self" && !self.in_trait_impl {
                    if let Some(sn) = self.current_struct_name.as_deref() {
                        let qualified = format!("{}::{}", sn, func.name);
                        if self.self_receiver_upgrades.get(&qualified)
                            == Some(&OwnershipMode::MutBorrowed)
                        {
                            if super::self_analysis::function_return_moves_self_fields(&analyzed.decl)
                                && body_modifies
                                && self.method_returns_impl_struct(func)
                            {
                                self.inferred_borrowed_params.remove("self");
                                self.inferred_mut_borrowed_params.remove("self");
                                return "mut self".to_string();
                            }
                            self.inferred_mut_borrowed_params.insert("self".to_string());
                            self.inferred_borrowed_params.remove("self");
                            return "&mut self".to_string();
                        }
                    }
                }

                // Handle explicit ownership hints (self, &self, &mut self)
                let mut type_str = match &param.ownership {
                    OwnershipHint::Owned => {
                        if param.name == "self" {
                            let body_modifies = body_modifies;
                            let consumes_self = super::self_analysis::function_consumes_self(&analyzed.decl)
                                || super::self_analysis::function_return_moves_self_fields(&analyzed.decl);
                            let eff_ownership =
                                self.get_effective_self_ownership(&func.name, analyzed);
                            let self_str = if let Some(ownership_mode) = eff_ownership {
                                match ownership_mode {
                                    OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                        if !self.in_trait_impl
                                            && (self.method_returns_impl_struct(func) || consumes_self) =>
                                    {
                                        if body_modifies { "mut self" } else { "self" }
                                    }
                                    OwnershipMode::MutBorrowed => "&mut self",
                                    OwnershipMode::Borrowed => {
                                        if !self.in_trait_impl && body_modifies {
                                            "&mut self"
                                        } else if !self.in_trait_impl
                                            && self.method_returns_impl_struct(func)
                                        {
                                            if body_modifies {
                                                "mut self"
                                            } else {
                                                self.owned_self_receiver(&analyzed.decl)
                                            }
                                        } else {
                                            "&self"
                                        }
                                    }
                                    OwnershipMode::Owned => {
                                        if self.in_trait_impl {
                                            if body_modifies {
                                                "mut self"
                                            } else {
                                                "self"
                                            }
                                        } else {
                                            self.owned_self_receiver(&analyzed.decl)
                                        }
                                    }
                                }
                            } else {
                                if self.in_trait_impl {
                                    "self"
                                } else {
                                    self.owned_self_receiver(&analyzed.decl)
                                }
                            };
                            // Sync borrowed-params sets with actual generated receiver.
                            match self_str {
                                "&self" => {
                                    self.inferred_borrowed_params.insert("self".to_string());
                                    self.inferred_mut_borrowed_params.remove("self");
                                }
                                "&mut self" => {
                                    self.inferred_mut_borrowed_params.insert("self".to_string());
                                    self.inferred_borrowed_params.remove("self");
                                }
                                _ => {
                                    self.inferred_borrowed_params.remove("self");
                                    self.inferred_mut_borrowed_params.remove("self");
                                }
                            }
                            self.record_self_receiver_upgrade(
                                &func.name,
                                eff_ownership,
                                self_str,
                            );
                            return self_str.to_string();
                        }
                        // Check if the analyzer inferred MutBorrowed (e.g., Copy type
                        // mutated through method call — caller wants mutation visible).
                        // Skip in trait impls: trait signature must match exactly.
                        if !self.in_trait_impl {
                            if !param.is_mutable {
                                if let Some(OwnershipMode::MutBorrowed) =
                                    self.get_param_ownership(&param.name, analyzed)
                                {
                                    self.inferred_mut_borrowed_params
                                        .insert(param.name.clone());
                                    return format!(
                                        "&mut {}",
                                        self.type_to_rust(formal_type)
                                    );
                                }
                            }
                            if !analyzed.returned_parameters.contains(&param.name)
                                && (!param.is_mutable
                                    || analyzed
                                        .field_mutated_parameters
                                        .contains(&param.name))
                                && analyzed.mutated_parameters.contains(&param.name)
                                && !self.is_type_copy(formal_type)
                            {
                                self.inferred_mut_borrowed_params
                                    .insert(param.name.clone());
                                return format!(
                                    "&mut {}",
                                    self.type_to_rust(formal_type)
                                );
                            }
                        }
                        if param.name != "self"
                            && !asref_runtime_keep_owned
                            && !payload_forces_owned
                            && !self.param_only_used_in_discarding_let_binding(
                                func.body.as_slice(),
                                &param.name,
                                func,
                            )
                            && !self.param_single_arg_owned_self_or_field_forward(param, func)
                            && self.param_should_emit_borrowed_delegation_formal(param, func)
                            && !self.is_collection_key_owned_param(param, func)
                        {
                            let type_str =
                                self.borrowed_formal_rust_type_for_param(param, func, param_idx);
                            self.emitted_rust_ref_formals.insert(param.name.clone());
                            self.inferred_borrowed_params.insert(param.name.clone());
                            self.inferred_mut_borrowed_params.remove(&param.name);
                            if type_str == "&str"
                                || type_str.starts_with("&'a str")
                                || type_str.ends_with(" str")
                            {
                                self.str_ref_optimized_params.insert(param.name.clone());
                            }
                            return format!("{}: {}", param.name, type_str);
                        }
                        let str_ref_formal_ok = (self.str_ref_optimized_params.contains(&param.name)
                            || analyzed.str_ref_optimizable_params.contains(&param.name))
                            && !asref_runtime_keep_owned
                            && !payload_forces_owned
                            && !self.in_trait_impl
                            && !param.decorators.iter().any(|d| d.name == "string_ref")
                            && crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
                            && !matches!(
                                &param.type_,
                                Type::Reference(_) | Type::MutableReference(_)
                            )
                            && !self.is_collection_key_owned_param(param, func)
                            && !self.param_stored_in_owned_payload(
                                func.body.as_slice(),
                                &param.name,
                            );
                        if str_ref_formal_ok {
                            let type_str =
                                self.borrowed_formal_rust_type_for_param(param, func, param_idx);
                            self.emitted_rust_ref_formals.insert(param.name.clone());
                            self.inferred_borrowed_params.insert(param.name.clone());
                            self.inferred_mut_borrowed_params.remove(&param.name);
                            self.str_ref_optimized_params.insert(param.name.clone());
                            return format!("{}: {}", param.name, type_str);
                        }
                        // Owned parameters are always mutable in Windjammer
                        return format!("mut {}: {}", param.name, self.type_to_rust(formal_type));
                    }
                    OwnershipHint::Ref => {
                        if param.name == "self" {
                            let body_modifies = body_modifies;
                            if let Some(ownership_mode) =
                                self.get_effective_self_ownership(&func.name, analyzed)
                            {
                                match ownership_mode {
                                    OwnershipMode::MutBorrowed => return "&mut self".to_string(),
                                    OwnershipMode::Borrowed => {
                                        if !self.in_trait_impl && body_modifies {
                                            return "&mut self".to_string();
                                        }
                                        return "&self".to_string();
                                    }
                                    OwnershipMode::Owned => {
                                        return "self".to_string();
                                    }
                                }
                            }
                            if !self.in_trait_impl && body_modifies {
                                return "&mut self".to_string();
                            }
                            return "&self".to_string();
                        }
                        // Don't add & if the type is already a Reference
                        if matches!(
                            formal_type,
                            Type::Reference(_) | Type::MutableReference(_)
                        ) {
                            if self.is_type_copy(&param.type_)
                                && !self.inferred_borrowed_params.contains(&param.name)
                                && (!self.ir_cutover.ownership
                                    || !matches!(
                                        self.get_param_ownership(&param.name, analyzed),
                                        Some(OwnershipMode::Borrowed)
                                    ))
                            {
                                self.type_to_rust(&param.type_)
                            } else {
                                self.type_to_rust(formal_type)
                            }
                        } else {
                            // TDD FIX: Copy types pass by value even with Ref hint
                            if self.is_type_copy(formal_type) {
                                self.type_to_rust(formal_type)
                            } else {
                                // TDD FIX: Borrowed → &T (including &String for strings)
                                // Correctness > idioms: &String works with Vec<String> methods
                                format!("&{}", self.type_to_rust(formal_type))
                            }
                        }
                    }
                    OwnershipHint::Mut => {
                        if param.name == "self" {
                            let body_modifies = body_modifies;
                            if let Some(ownership_mode) =
                                self.get_effective_self_ownership(&func.name, analyzed)
                            {
                                return match ownership_mode {
                                    OwnershipMode::Borrowed => {
                                        if !self.in_trait_impl && body_modifies {
                                            "&mut self".to_string()
                                        } else {
                                            "&self".to_string()
                                        }
                                    }
                                    OwnershipMode::MutBorrowed => "&mut self".to_string(),
                                    OwnershipMode::Owned => {
                                        if self.in_trait_impl {
                                            "self".to_string()
                                        } else {
                                            self.owned_self_receiver(&analyzed.decl).to_string()
                                        }
                                    }
                                };
                            }
                            return "&mut self".to_string();
                        }
                        // Don't add &mut if the type is already a MutableReference
                        if matches!(formal_type, Type::MutableReference(_)) {
                            self.type_to_rust(formal_type)
                        } else {
                            format!("&mut {}", self.type_to_rust(formal_type))
                        }
                    }
                    OwnershipHint::Inferred => {
                        if param.name == "self" {
                            if !self.in_trait_impl
                                && super::self_analysis::function_calls_owned_self_method(
                                    &analyzed.decl,
                                    &self.signature_registry,
                                    self.current_struct_name.as_deref(),
                                )
                            {
                                let body_modifies = body_modifies;
                                let self_str = if body_modifies
                                    && self.method_returns_impl_struct(func)
                                {
                                    "mut self"
                                } else {
                                    "self"
                                };
                                self.inferred_borrowed_params.remove("self");
                                self.inferred_mut_borrowed_params.remove("self");
                                return self_str.to_string();
                            }
                            if !self.in_trait_impl
                                && !matches!(param.ownership, OwnershipHint::Owned)
                                && matches!(
                                    analyzed.inferred_ownership.get("self"),
                                    Some(OwnershipMode::Borrowed)
                                )
                                && !self.method_returns_impl_struct(func)
                                && !super::self_analysis::function_calls_owned_self_method(
                                    &analyzed.decl,
                                    &self.signature_registry,
                                    self.current_struct_name.as_deref(),
                                )
                            {
                                self.inferred_borrowed_params.insert("self".to_string());
                                self.inferred_mut_borrowed_params.remove("self");
                                return "&self".to_string();
                            }
                            if !self.in_trait_impl
                                && matches!(
                                    analyzed.inferred_ownership.get("self"),
                                    Some(OwnershipMode::Owned)
                                )
                                && (super::self_analysis::function_calls_owned_self_method(
                                    &analyzed.decl,
                                    &self.signature_registry,
                                    self.current_struct_name.as_deref(),
                                ) || super::self_analysis::function_consumes_self(&analyzed.decl))
                            {
                                let body_modifies = body_modifies;
                                let self_str = if body_modifies
                                    && self.method_returns_impl_struct(func)
                                {
                                    "mut self"
                                } else {
                                    self.owned_self_receiver(&analyzed.decl)
                                };
                                self.inferred_borrowed_params.remove("self");
                                self.inferred_mut_borrowed_params.remove("self");
                                return self_str.to_string();
                            }
                            let body_modifies = body_modifies;
                            let returns_self = self.method_returns_impl_struct(&analyzed.decl);
                            let consumes_self = super::self_analysis::function_consumes_self(&analyzed.decl)
                                || super::self_analysis::function_return_moves_self_fields(&analyzed.decl)
                                || super::self_analysis::function_calls_owned_self_method(
                                    &analyzed.decl,
                                    &self.signature_registry,
                                    self.current_struct_name.as_deref(),
                                );
                            let self_str = if let Some(ownership_mode) =
                                self.get_effective_self_ownership(&func.name, analyzed)
                            {
                                match ownership_mode {
                                    OwnershipMode::Owned => {
                                        if self.in_trait_impl {
                                            "self"
                                        } else if body_modifies {
                                            "mut self"
                                        } else {
                                            self.owned_self_receiver(&analyzed.decl)
                                        }
                                    }
                                    OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                        if !self.in_trait_impl && (returns_self || consumes_self) =>
                                    {
                                        if body_modifies { "mut self" } else { "self" }
                                    }
                                    OwnershipMode::MutBorrowed
                                        if consumes_self && !body_modifies =>
                                    {
                                        "self"
                                    }
                                    OwnershipMode::MutBorrowed => "&mut self",
                                    OwnershipMode::Borrowed => {
                                        if !self.in_trait_impl && body_modifies {
                                            "&mut self"
                                        } else {
                                            "&self"
                                        }
                                    }
                                }
                            } else if body_modifies && returns_self {
                                "mut self"
                            } else if consumes_self {
                                "self"
                            } else if body_modifies {
                                "&mut self"
                            } else if returns_self {
                                "self"
                            } else {
                                "&self"
                            };
                            // Sync borrowed-params sets with actual generated receiver.
                            match self_str {
                                "&self" => {
                                    self.inferred_borrowed_params.insert("self".to_string());
                                    self.inferred_mut_borrowed_params.remove("self");
                                }
                                "&mut self" => {
                                    self.inferred_mut_borrowed_params.insert("self".to_string());
                                    self.inferred_borrowed_params.remove("self");
                                }
                                _ => {
                                    self.inferred_borrowed_params.remove("self");
                                    self.inferred_mut_borrowed_params.remove("self");
                                }
                            }
                            let eff = self.get_effective_self_ownership(&func.name, analyzed);
                            self.record_self_receiver_upgrade(
                                &func.name,
                                eff,
                                self_str,
                            );
                            return self_str.to_string();
                        }

                        if param.name != "self" {
                            let str_ref_formal_ok = (self.str_ref_optimized_params.contains(&param.name)
                                || analyzed.str_ref_optimizable_params.contains(&param.name))
                                && !asref_runtime_keep_owned
                                && !payload_forces_owned
                                && !self.in_trait_impl
                                && !param.decorators.iter().any(|d| d.name == "string_ref")
                                && crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
                                && !matches!(
                                    &param.type_,
                                    Type::Reference(_) | Type::MutableReference(_)
                                )
                                && !self.is_collection_key_owned_param(param, func)
                                && !self.param_stored_in_owned_payload(
                                    func.body.as_slice(),
                                    &param.name,
                                );
                            if str_ref_formal_ok {
                                let type_str = self.borrowed_formal_rust_type_for_param(
                                    param, func, param_idx,
                                );
                                self.emitted_rust_ref_formals.insert(param.name.clone());
                                self.inferred_borrowed_params.insert(param.name.clone());
                                self.inferred_mut_borrowed_params.remove(&param.name);
                                self.str_ref_optimized_params.insert(param.name.clone());
                                return format!("{}: {}", param.name, type_str);
                            }
                        }

                        // Pure delegation / call-only forwarders: emit &T even when IR/analyzer
                        // left the converged formal as owned (dogfood LsmEngine::get → MemoryEngine::get).
                        // Never demote mutated / MutBorrowed params to shared `&T`.
                        // Tuple discards (`let _ = (key.bytes.len(), value)`) keep source ownership.
                        // Trait-impl owned `string` and `@string_ref` must keep their contracts.
                        if !trait_impl_owned_string
                            && !asref_runtime_keep_owned
                            && !payload_forces_owned
                            && !param.decorators.iter().any(|d| d.name == "string_ref")
                            && !self.param_must_not_demote_to_shared_borrow(&param.name, analyzed, None)
                            && !analyzed.returned_parameters.contains(&param.name)
                            && !analyzed.field_extract_parameters.contains(&param.name)
                            && !self.param_single_arg_owned_self_or_field_forward(param, func)
                            && self.param_should_emit_borrowed_delegation_formal(param, func)
                            && !self.param_only_used_in_discarding_let_binding(
                                func.body.as_slice(),
                                &param.name,
                                func,
                            )
                            && !self.is_collection_key_owned_param(param, func)
                            // Multi-stmt allowed: `param_should_emit_borrowed_delegation_formal`
                            // already gates multiparam store forwards (`apply_patch_put`),
                            // including when the body stores via owned callees (regression-047).
                            && !self.func_is_pure_forwarding_delegate(func)
                        {
                            let type_str =
                                self.borrowed_formal_rust_type_for_param(param, func, param_idx);
                            self.emitted_rust_ref_formals.insert(param.name.clone());
                            self.inferred_borrowed_params.insert(param.name.clone());
                            self.inferred_mut_borrowed_params.remove(&param.name);
                            if type_str == "&str"
                                || type_str.starts_with("&'a str")
                                || type_str.ends_with(" str")
                            {
                                self.str_ref_optimized_params.insert(param.name.clone());
                            }
                            return format!("{}: {}", param.name, type_str);
                        }

                        // Check if type already has ownership baked in (like &str from string inference)
                        let force_owned_collection_key =
                    self.is_collection_key_owned_param(param, func);
                        let discard_only = self.param_only_used_in_discarding_let_binding(
                            func.body.as_slice(),
                            &param.name,
                            func,
                        );
                        // Bare tuple-discard (`let _ = (a, b)`) → shared `&T` so callers
                        // reuse without `.clone()` (authz-reuse regression). Field-projection discards
                        // and owned-Key engine facades (key.bytes lookups) keep owned formals.
                        // Do not use `struct_is_owned_engine_key_facade` here — its early
                        // fallback falsely matches any 2-method impl with a Custom param
                        // (TupleStore::has_tuple).
                        let discard_as_shared_borrow = self
                            .param_only_used_as_bare_id_in_discarding_tuple(
                                func.body.as_slice(),
                                &param.name,
                                func,
                            )
                            && !analyzed.field_extract_parameters.contains(&param.name)
                            && !payload_forces_owned
                            && !force_owned_collection_key
                            && !self.current_struct_name.as_ref().is_some_and(|sn| {
                                self.struct_has_owned_key_field_lookup.contains(sn)
                            })
                            // Bare discard intentionally demotes Owned formals — do not
                            // consult `param_must_not_demote_to_shared_borrow` (Owned hint).
                            && !matches!(
                                self.get_param_ownership(&param.name, analyzed),
                                Some(OwnershipMode::MutBorrowed)
                            )
                            && !(analyzed.mutated_parameters.contains(&param.name)
                                && !analyzed.returned_parameters.contains(&param.name))
                            && !matches!(
                                &param.type_,
                                Type::Reference(_) | Type::MutableReference(_)
                            );
                        if discard_as_shared_borrow {
                            let type_str =
                                self.borrowed_formal_rust_type_for_param(param, func, param_idx);
                            self.emitted_rust_ref_formals.insert(param.name.clone());
                            self.inferred_borrowed_params.insert(param.name.clone());
                            self.inferred_mut_borrowed_params.remove(&param.name);
                            if type_str == "&str"
                                || type_str.starts_with("&'a str")
                                || type_str.ends_with(" str")
                            {
                                self.str_ref_optimized_params.insert(param.name.clone());
                            }
                            return format!("{}: {}", param.name, type_str);
                        }
                        let analyzer_owned_value_param = matches!(
                            analyzed.inferred_ownership.get(&param.name),
                            Some(OwnershipMode::Owned)
                        ) && !matches!(
                            &param.type_,
                            Type::Reference(_) | Type::MutableReference(_)
                        );
                        if matches!(
                            formal_type,
                            Type::Reference(_) | Type::MutableReference(_)
                        ) && !force_owned_collection_key
                            && !discard_only
                            && !analyzer_owned_value_param
                        {
                            // Copy aggregates: strip spurious shared `&T` from readonly field
                            // reads. Keep `&mut T` when the body mutates (MutBorrowed / mutated).
                            let mut_copy_formal = matches!(
                                formal_type,
                                Type::MutableReference(_)
                            ) || matches!(
                                analyzed.inferred_ownership.get(&param.name),
                                Some(OwnershipMode::MutBorrowed)
                            ) || matches!(
                                self.get_param_ownership(&param.name, analyzed),
                                Some(OwnershipMode::MutBorrowed)
                            ) || (analyzed.mutated_parameters.contains(&param.name)
                                && !analyzed.returned_parameters.contains(&param.name));
                            if self.is_type_copy(&param.type_)
                                && !mut_copy_formal
                                && !self.inferred_borrowed_params.contains(&param.name)
                                && !self.inferred_mut_borrowed_params.contains(&param.name)
                                && (!self.ir_cutover.ownership
                                    || !matches!(
                                        self.get_param_ownership(&param.name, analyzed),
                                        Some(OwnershipMode::Borrowed)
                                    ))
                            {
                                self.type_to_rust(&param.type_)
                            } else {
                                // Already has & or &mut - just convert
                                self.type_to_rust(formal_type)
                            }
                        } else if force_owned_collection_key {
                            self.type_to_rust(&param.type_)
                        } else {
                            // Apply ownership from IR solver (or legacy analyzer when cutover off).
                            let registry_sig = func
                                .parent_type
                                .as_ref()
                                .map(|pt| format!("{pt}::{}", func.name))
                                .and_then(|key| self.get_signature_with_global(&key).cloned())
                                .or_else(|| self.get_signature_with_global(&func.name).cloned());
                            let registry_ownership = registry_sig
                                .as_ref()
                                .and_then(|sig| sig.param_ownership.get(param_idx).copied());
                            let registry_stale_engine_owned = registry_sig.as_ref().is_some_and(|sig| {
                                crate::codegen::rust::signature_promotion::param_is_stale_engine_owned_stub(
                                    sig, param_idx,
                                )
                            });
                            let mut ownership_mode = self
                                .param_ownership_for_formal_demotion(&param.name, analyzed)
                                .or_else(|| {
                                    analyzed.inferred_ownership.get(&param.name).copied()
                                })
                                .or(registry_ownership)
                                .unwrap_or(OwnershipMode::Owned);

                            // Converged registry MutBorrowed/Borrowed beats stale analyzed Owned
                            // (cross-module Copy passthrough: update_direction → normalize).
                            // Never let stale engine Owned win over body Borrowed (QuestId keys).
                            if let Some(reg) = registry_ownership {
                                if matches!(
                                    reg,
                                    OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                ) && matches!(ownership_mode, OwnershipMode::Owned)
                                {
                                    ownership_mode = reg;
                                } else if matches!(reg, OwnershipMode::Owned)
                                    && registry_stale_engine_owned
                                    && matches!(
                                        analyzed.inferred_ownership.get(&param.name),
                                        Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
                                    )
                                {
                                    ownership_mode = analyzed.inferred_ownership[&param.name];
                                }
                            }

                            // Single-arg forwarder: keep owned only when the callee truly
                            // takes owned. Never clobber converged MutBorrowed/Borrowed
                            // (cross-module Copy passthrough: update_direction → normalize).
                            if !matches!(
                                ownership_mode,
                                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                            ) && self.param_is_single_arg_call_only_delegate(param, func)
                                && !self.param_passed_to_mut_borrowing_callee(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                                && (self.param_passed_to_owned_non_copy_method_arg(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                ) || !self.param_passed_to_borrowing_callee(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                ))
                            {
                                ownership_mode = OwnershipMode::Owned;
                                self.inferred_borrowed_params.remove(&param.name);
                                self.inferred_mut_borrowed_params.remove(&param.name);
                            }

                            let _debug_formal = std::env::var("WJ_DEBUG_FORMAL").is_ok() && param.name == "grid" && func.name == "do_work";
                            if _debug_formal {
                                eprintln!("[FORMAL-INIT] fn={} param={} initial_mode={:?} ir_own={:?} analyzer_own={:?} registry_own={:?}",
                                    func.name, param.name, ownership_mode,
                                    self.get_param_ownership(&param.name, analyzed),
                                    analyzed.inferred_ownership.get(&param.name),
                                    registry_ownership,
                                );
                            }
                            macro_rules! trace_own {
                                ($line:expr, $mode:expr) => {
                                    if _debug_formal {
                                        eprintln!("[FORMAL-TRACE] line={} mode={:?}", $line, $mode);
                                    }
                                };
                            }
                            macro_rules! set_own {
                                ($line:expr, $val:expr) => {{
                                    ownership_mode = $val;
                                    if _debug_formal {
                                        eprintln!("[FORMAL-SET] line={} mode→{:?}", $line, ownership_mode);
                                    }
                                }};
                            }

                            if matches!(
                                analyzed.inferred_ownership.get(&param.name),
                                Some(OwnershipMode::MutBorrowed)
                            ) && !analyzed.returned_parameters.contains(&param.name)
                                && !self.param_passes_to_wj_owned_sibling_call(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                                && !self.param_only_forwards_to_emitted_owned_callees(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                            {
                                ownership_mode = OwnershipMode::MutBorrowed;
                            } else if analyzed.mutated_parameters.contains(&param.name)
                                && !analyzed.returned_parameters.contains(&param.name)
                                && !param.is_mutable
                            {
                                ownership_mode = OwnershipMode::MutBorrowed;
                            }

                            if self.current_struct_name.as_ref().is_some_and(|sn| {
                                self.param_keeps_owned_engine_key_facade(sn, param, func)
                            }) {
                                ownership_mode = OwnershipMode::Owned;
                                trace_own!(926, ownership_mode);
                            }

                            // Field-extract returns (`key.bytes` / `msg.payload`) need an owned
                            // formal so the body can move the field; call sites clone when the
                            // binding is reused (regression-044/045).
                            if analyzed.field_extract_parameters.contains(&param.name)
                                && !self.is_type_copy(&param.type_)
                            {
                                ownership_mode = OwnershipMode::Owned;
                            }

                            // Directly returned params must stay owned (identity / transform APIs).
                            // Exception: associated WJ `string` identity → `&str` + `.to_string()`.
                            if analyzed.returned_parameters.contains(&param.name)
                                && !self.is_type_copy(&param.type_)
                            {
                                if self.associated_text_identity_return_may_borrow(
                                    func, param, analyzed,
                                ) {
                                    ownership_mode = OwnershipMode::Borrowed;
                                    self.inferred_borrowed_params.insert(param.name.clone());
                                    self.str_ref_optimized_params.insert(param.name.clone());
                                } else {
                                    ownership_mode = OwnershipMode::Owned;
                                }
                            }

                            if matches!(ownership_mode, OwnershipMode::MutBorrowed)
                                && (self.param_passes_to_wj_owned_sibling_call(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                ) || self.param_only_forwards_to_emitted_owned_callees(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                ))
                            {
                                ownership_mode = OwnershipMode::Owned;
                                self.inferred_mut_borrowed_params.remove(&param.name);
                            }

                            if matches!(ownership_mode, OwnershipMode::Borrowed)
                                && crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
                                && self.param_only_forwards_to_emitted_owned_callees(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                            {
                                ownership_mode = OwnershipMode::Owned;
                                self.inferred_borrowed_params.remove(&param.name);
                                self.str_ref_optimized_params.remove(&param.name);
                            }

                            if self.param_used_in_if_with_condition_and_branches(
                                func.body.as_slice(),
                                &param.name,
                            ) || self.param_used_in_if_else_both_branches(
                                func.body.as_slice(),
                                &param.name,
                            ) || self.body_forwards_param_in_if_condition(&param.name, func)
                            || self.current_fn_forward_ref_if_params.contains(&param.name)
                            {
                                // Owned binding for if/else move semantics — but never demote
                                // an already-inferred borrow/mut-borrow (readonly or mutated).
                                if !matches!(
                                    ownership_mode,
                                    OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                ) {
                                    ownership_mode = OwnershipMode::Owned;
                                }
                            }

                            if !self.is_type_copy(&param.type_)
                                && !matches!(
                                    &param.type_,
                                    Type::Reference(_) | Type::MutableReference(_)
                                )
                                && !matches!(
                                    ownership_mode,
                                    OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                )
                                && (self.param_has_forward_ref_keep_owned(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                ) || self.current_fn_mixed_forwarder_params.contains(&param.name))
                            {
                                ownership_mode = OwnershipMode::Owned;
                                trace_own!(976, ownership_mode);
                            }

                            if !self.ir_cutover.ownership {
                                if self.inferred_borrowed_params.contains(&param.name) {
                                    ownership_mode = OwnershipMode::Borrowed;
                                } else if self.inferred_mut_borrowed_params.contains(&param.name) {
                                    ownership_mode = OwnershipMode::MutBorrowed;
                                }

                                // Converged registry Owned wins over stale first-pass borrow hints
                                // (e.g. imported Copy Vec3 with only field reads).
                                // Stale engine Owned (QuestId on is_quest_active) must not beat
                                // body-converged Borrowed map-key formals.
                                if registry_ownership == Some(OwnershipMode::Owned)
                                    && !registry_stale_engine_owned
                                    && !matches!(
                                        analyzed.inferred_ownership.get(&param.name),
                                        Some(
                                            OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                        )
                                    )
                                {
                                    ownership_mode = OwnershipMode::Owned;
                                    trace_own!(999, ownership_mode);
                                }

                                // Copy aggregates pass by value unless the analyzer kept an active
                                // borrow (field-enum-match) or mutation (`&mut T`). Stale registry
                                // `Reference(T)` alone must not emit `&Vec3` formals.
                                if self.is_type_copy(formal_type)
                                    && !crate::type_classification::is_copy_pass_by_value_formal(
                                        formal_type,
                                    )
                                    && !self.inferred_borrowed_params.contains(&param.name)
                                    && ownership_mode != OwnershipMode::MutBorrowed
                                    && !self.inferred_mut_borrowed_params.contains(&param.name)
                                    && !analyzed.mutated_parameters.contains(&param.name)
                                {
                                    ownership_mode = OwnershipMode::Owned;
                                }

                                if self.is_collection_key_owned_param(param, func) {
                                    ownership_mode = OwnershipMode::Owned;
                                }

                                if !self.is_type_copy(formal_type)
                                    && self.param_passed_to_owned_self_method_arg(
                                        func.body.as_slice(),
                                        &param.name,
                                        func,
                                    )
                                {
                                    ownership_mode = OwnershipMode::Owned;
                                    trace_own!(1022, ownership_mode);
                                }

                                // Field mutation on params requires &mut when the body mutates fields.
                                if !self.in_trait_impl
                                    && !analyzed.returned_parameters.contains(&param.name)
                                    && analyzed.mutated_parameters.contains(&param.name)
                                    && self.variable_needs_mut(&param.name)
                                    && (!param.is_mutable
                                        || analyzed
                                            .field_mutated_parameters
                                            .contains(&param.name))
                                {
                                    ownership_mode = OwnershipMode::MutBorrowed;
                                }
                            }

                            if self.param_stored_in_owned_payload(
                                func.body.as_slice(),
                                &param.name,
                            ) && !self.param_should_emit_borrowed_delegation_formal(param, func)
                            {
                                ownership_mode = OwnershipMode::Owned;
                            }

                            if self.param_moves_via_struct_literal_init(
                                func.body.as_slice(),
                                &param.name,
                            ) && !self.param_should_emit_borrowed_delegation_formal(param, func)
                            {
                                ownership_mode = OwnershipMode::Owned;
                            }

                            if self.param_multiparam_store_keeps_owned_key_formal(param, func) {
                                ownership_mode = OwnershipMode::Owned;
                                self.inferred_borrowed_params.remove(&param.name);
                                self.inferred_mut_borrowed_params.remove(&param.name);
                            } else if matches!(&param.type_, Type::Custom(_))
                                && !self.is_type_copy(&param.type_)
                                && !crate::codegen::rust::types::is_windjammer_text_type(
                                    &param.type_,
                                )
                                && (self.param_only_forwarded_to_map_key_callee(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                ))
                            {
                                ownership_mode = OwnershipMode::Borrowed;
                                self.inferred_borrowed_params.insert(param.name.clone());
                            }

                            // Pure delegation wrappers keep &T formals when the body only
                            // forwards to borrowing callees (dogfood LsmEngine::get), even under IR cutover.
                            // Never demote mutated / MutBorrowed params: `c.increment()` (&mut self)
                            // is a borrowing callee, but the formal must be `&mut T` (or owned mut),
                            // not shared `&T`.
                            if !self.param_must_not_demote_to_shared_borrow(
                                &param.name,
                                analyzed,
                                Some(ownership_mode),
                            ) && !self.is_type_copy(&param.type_)
                                && !analyzed.field_extract_parameters.contains(&param.name)
                                && !analyzed.returned_parameters.contains(&param.name)
                                && (!self.param_stored_in_owned_payload(
                                    func.body.as_slice(),
                                    &param.name,
                                ) || self.param_should_emit_borrowed_delegation_formal(param, func))
                                && (!self.param_moves_via_struct_literal_init(
                                    func.body.as_slice(),
                                    &param.name,
                                ) || self.param_should_emit_borrowed_delegation_formal(param, func))
                                && !matches!(
                                    &param.type_,
                                    Type::Reference(_) | Type::MutableReference(_)
                                )
                                && !self.func_is_pure_forwarding_delegate(func)
                                && !self.param_passed_from_multiple_statements(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                                && !self.param_has_forward_ref_keep_owned(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                                && !self.current_fn_mixed_forwarder_params.contains(&param.name)
                                && !self.param_passes_to_wj_owned_sibling_call(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                                && !self.current_struct_name.as_ref().is_some_and(|sn| {
                                    self.param_keeps_owned_engine_key_facade(sn, param, func)
                                        && !self.param_only_used_via_field_or_index_projection(
                                            func.body.as_slice(),
                                            &param.name,
                                        )
                                })
                                && !self.is_collection_key_owned_param(param, func)
                                && !self.param_is_single_arg_call_only_delegate(param, func)
                                && !self.param_multiparam_store_keeps_owned_key_formal(param, func)
                                && (self.inferred_borrowed_params.contains(&param.name)
                                    || (converged_analyzer_borrow
                                        && (crate::codegen::rust::types::is_windjammer_text_type(
                                            &param.type_,
                                        )
                                            || (self.is_type_copy(&param.type_)
                                                && !crate::type_classification::is_copy_pass_by_value_formal(
                                                    &param.type_,
                                                ))
                                            || self.param_should_emit_borrowed_delegation_formal(
                                                param, func,
                                            )))
                                    || self.param_passed_to_borrowing_callee(
                                        func.body.as_slice(),
                                        &param.name,
                                        func,
                                    ))
                            {
                                ownership_mode = OwnershipMode::Borrowed;
                            } else if !self.is_type_copy(&param.type_)
                                && !matches!(
                                    &param.type_,
                                    Type::Reference(_) | Type::MutableReference(_)
                                )
                                && !analyzed.returned_parameters.contains(&param.name)
                                && !self.param_has_forward_ref_keep_owned(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                                && !self.current_fn_mixed_forwarder_params.contains(&param.name)
                                && !self.param_passes_to_wj_owned_sibling_call(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                                && self.inferred_mut_borrowed_params.contains(&param.name)
                            {
                                // Mutated+returned formals stay Owned (IR/solver lattice);
                                // do not re-demote from `inferred_mut_borrowed_params`.
                                ownership_mode = OwnershipMode::MutBorrowed;
                            }

                            // E0053 FIX: Trait impl parameters MUST match the trait
                            // definition's parameter types exactly. Look up the trait's
                            // method signature and use its ownership for each parameter.
                            if trait_impl_owned_string {
                                ownership_mode = OwnershipMode::Owned;
                            } else if self.in_trait_impl {
                                if let Some(trait_own) = self
                                    .current_trait_impl_name
                                    .as_ref()
                                    .and_then(|trait_name| {
                                        let methods = self
                                            .analyzed_trait_methods
                                            .get(trait_name.as_str())
                                            .or_else(|| {
                                                trait_name
                                                    .rfind("::")
                                                    .map(|i| &trait_name[i + 2..])
                                                    .and_then(|key| {
                                                        self.analyzed_trait_methods.get(key)
                                                    })
                                            });
                                        methods.and_then(|m| {
                                            m.get(func.name.as_str()).and_then(|trait_fn| {
                                                // Use position-based lookup: impl param names
                                                // may differ from trait (e.g., ctx vs app).
                                                trait_fn.decl.parameters.get(param_idx)
                                                    .and_then(|trait_param| {
                                                        trait_fn.inferred_ownership.get(&trait_param.name).copied()
                                                    })
                                                    .or_else(|| {
                                                        trait_fn.inferred_ownership.get(&param.name).copied()
                                                    })
                                            })
                                        })
                                    })
                                {
                                    ownership_mode = trait_own;
                                } else if param.name != "self"
                                    && !matches!(
                                        &param.type_,
                                        Type::Reference(_) | Type::MutableReference(_)
                                    )
                                {
                                    // Cross-file trait body not yet analyzed — bare source
                                    // types stay owned (E0053).
                                    ownership_mode = OwnershipMode::Owned;
                                    self.inferred_borrowed_params.remove(&param.name);
                                    self.inferred_mut_borrowed_params.remove(&param.name);
                                }
                            }

                            // Tuple-discard with field projection keeps owned engine Key
                            // formals; bare-id discards already returned as `&T` above.
                            if ownership_mode != OwnershipMode::Owned
                                && self.param_only_used_in_discarding_let_binding(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                                && !self.param_only_used_as_bare_id_in_discarding_tuple(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                                && !(
                                    (matches!(&param.type_, Type::Vec(_))
                                        || matches!(
                                            &param.type_,
                                            Type::Parameterized(name, _) if name == "Vec"
                                        ))
                                        && self.param_has_readonly_expression_use(
                                            func.body.as_slice(),
                                            &param.name,
                                        )
                                        && !self.param_has_owning_method_use(
                                            func.body.as_slice(),
                                            &param.name,
                                            func,
                                        )
                                )
                            {
                                ownership_mode = OwnershipMode::Owned;
                            }

                            let registry_param_ty = self
                                .get_signature_with_global(&func.name)
                                .and_then(|s| s.param_types.get(param_idx).cloned())
                                .or_else(|| analyzed.inferred_param_types.get(param_idx).cloned());
                            let copy_aggregate_ref_formal = if self.in_trait_impl {
                                None
                            } else {
                                registry_param_ty.as_ref().and_then(|ty| {
                                    if let Type::Reference(inner) = ty {
                                        if ownership_mode == OwnershipMode::Borrowed
                                            && self.inferred_borrowed_params.contains(&param.name)
                                            && self.is_type_copy(formal_type)
                                            && !crate::type_classification::is_copy_pass_by_value_formal(
                                                formal_type,
                                            )
                                        {
                                            Some(format!("&{}", self.type_to_rust(inner)))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                })
                            };

                            // Converged analyzer ownership is authoritative unless the body stores
                            // the param in an owned payload (enum variant, constructor, struct field).
                            // Do not overwrite facade / forward-ref / mixed-forwarder Owned contracts
                            // with body-inferred Borrowed (regression-046 get/has_key Key formals).
                            // Tuple-discard suppress-unused (`let _ = (key.bytes.len(), value)`)
                            // must keep source ownership — analyzer Borrowed from field reads
                            // must not overwrite (dogfood MemoryEngine::put / seed_write).
                            // Exception: Vec readonly tuple discards still demote to `&Vec`
                            // (`append_put` / `let _ = (key.len(), value.len())`).
                            // Field-projection tuple-discards keep owned; bare-id discards
                            // demote via early shared-borrow return (authz-reuse regression).
                            let discard_keep_owned = self.param_only_used_in_discarding_let_binding(
                                func.body.as_slice(),
                                &param.name,
                                func,
                            ) && !self.param_only_used_as_bare_id_in_discarding_tuple(
                                func.body.as_slice(),
                                &param.name,
                                func,
                            ) && !(Self::param_type_is_vec_container(&param.type_)
                                && self.param_has_readonly_expression_use(
                                    func.body.as_slice(),
                                    &param.name,
                                )
                                && !self.param_has_owning_method_use(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                ));
                            let keep_owned_facade = self.current_struct_name.as_ref().is_some_and(
                                |sn| self.struct_is_owned_engine_key_facade(sn, param),
                            ) && !field_proj_readonly
                                && !borrow_delegation;
                            let to_owned_sibling = self.param_passes_to_wj_owned_sibling_call(
                                func.body.as_slice(),
                                &param.name,
                                func,
                            );
                            // Reuse outer `borrow_delegation` — do not re-enter
                            // `param_should_emit_borrowed_delegation_formal` (stack overflow).
                            // Prefer definitive IR borrows over body-walk keep-owned.
                            // IR Owned still allows keep-owned / analyzer demotion until
                            // the solver fully replaces those contracts.
                            let ir_borrow = self.ir_param_ownership_definitive(&param.name).filter(
                                |m| {
                                    matches!(
                                        m,
                                        OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                                    )
                                },
                            );
                            let keep_owned_contract = if ir_borrow.is_some() && !discard_keep_owned {
                                false
                            } else {
                                payload_forces_owned
                                || discard_keep_owned
                                || keep_owned_facade
                                || (self.current_fn_mixed_forwarder_params.contains(&param.name)
                                    && !borrow_delegation)
                                || (self.current_fn_forward_ref_if_params.contains(&param.name)
                                    && !borrow_delegation)
                                || (self.param_has_forward_ref_keep_owned(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                ) && !borrow_delegation)
                                || (to_owned_sibling && !borrow_delegation)
                                || (self.param_is_single_arg_call_only_delegate(param, func)
                                    && self.param_passed_to_owned_non_copy_method_arg(
                                        func.body.as_slice(),
                                        &param.name,
                                        func,
                                    )
                                    && !borrow_delegation)
                                || analyzed.field_extract_parameters.contains(&param.name)
                                || (analyzed.returned_parameters.contains(&param.name)
                                    && !self.associated_text_identity_return_may_borrow(
                                        func, param, analyzed,
                                    ))
                                || (moves_via_struct_init && !vec_store_borrow_ok && !borrow_delegation)
                            };
                            // Mutated / MutBorrowed formals must not be forced Owned by facade
                            // keep-owned heuristics (Copy aggregate field writes need `&mut T`).
                            let keep_owned_contract = keep_owned_contract
                                && !matches!(ownership_mode, OwnershipMode::MutBorrowed)
                                && !(analyzed.mutated_parameters.contains(&param.name)
                                    && !analyzed.returned_parameters.contains(&param.name));
                            // Prefer definitive IR borrows over body-walk keep-owned —
                            // unless this is a payload-stored builder/setter without an
                            // explicit IR `str_ref_params` hint (Banner::message: stale
                            // IR Borrowed must not demote owned `string` field stores).
                            let ir_str_ref_hint = self.current_ir_function.as_ref().is_some_and(|ir| {
                                ir.str_ref_params.contains(&param.name)
                            });
                            if payload_forces_owned
                                && !ir_str_ref_hint
                                && !matches!(ownership_mode, OwnershipMode::MutBorrowed)
                                && !(analyzed.mutated_parameters.contains(&param.name)
                                    && !analyzed.returned_parameters.contains(&param.name))
                            {
                                ownership_mode = OwnershipMode::Owned;
                                self.str_ref_optimized_params.remove(&param.name);
                            } else if let Some(ir_mode) = ir_borrow {
                                // E0053: trait formals already applied above — IR MutRef from
                                // body mutation must not rewrite owned trait params to `&mut T`.
                                if !discard_keep_owned && !self.in_trait_impl {
                                    ownership_mode = ir_mode;
                                }
                            } else if (self.param_passed_to_slice_search_string_elem(
                                func.body.as_slice(),
                                &param.name,
                                func,
                            ) || self.param_passed_to_string_ref_formal_callee(
                                func.body.as_slice(),
                                &param.name,
                                func,
                            )) && !matches!(ownership_mode, OwnershipMode::MutBorrowed)
                                && !(analyzed.mutated_parameters.contains(&param.name)
                                    && !analyzed.returned_parameters.contains(&param.name))
                                && !keep_owned_contract
                            {
                                // Vec<String>::contains / binary_search need `&String` formals —
                                // beat keep-owned facades that would leave owned `String`.
                                ownership_mode = OwnershipMode::Borrowed;
                                self.inferred_borrowed_params.insert(param.name.clone());
                                self.str_ref_optimized_params.remove(&param.name);
                            } else if keep_owned_contract {
                                ownership_mode = OwnershipMode::Owned;
                            } else if self.in_trait_impl {
                                // E0053: trait ownership already applied above — do not
                                // demote via field-proj / vec-store / inferred Borrowed.
                            } else if !matches!(ownership_mode, OwnershipMode::MutBorrowed)
                                && !(analyzed.mutated_parameters.contains(&param.name)
                                    && !analyzed.returned_parameters.contains(&param.name))
                                && (field_proj_readonly
                                    || vec_store_borrow_ok
                                    || self.inferred_borrowed_params.contains(&param.name))
                            {
                                // Promote-readonly / store-only Vec / field-proj beat analyzer Owned.
                                // Never demote MutBorrowed / mutated formals (c.value = …, if-let mut).
                                ownership_mode = OwnershipMode::Borrowed;
                                if field_proj_readonly || vec_store_borrow_ok {
                                    self.inferred_borrowed_params.insert(param.name.clone());
                                }
                            } else if let Some(analyzed_own) =
                                analyzed.inferred_ownership.get(&param.name)
                            {
                                // Never demote IR/registry MutBorrowed to analyzer Owned —
                                // the analyzer may not see cross-file passthrough mutations.
                                if !(*analyzed_own == OwnershipMode::Owned
                                    && ownership_mode == OwnershipMode::MutBorrowed)
                                {
                                    ownership_mode = *analyzed_own;
                                }
                            } else if self.get_param_ownership(&param.name, analyzed)
                                == Some(OwnershipMode::Owned)
                            {
                                // Same guard: don't demote MutBorrowed to Owned.
                                if ownership_mode != OwnershipMode::MutBorrowed {
                                    ownership_mode = OwnershipMode::Owned;
                                }
                            }

                            // MutBorrowed / mutated formals beat false-positive Owned heuristics
                            // (payload / if-branch / facade). Shared Borrowed must NOT overwrite
                            // keep-owned contracts (dogfood Key discards, forward-ref facades) — those
                            // stay Owned even when the analyzer marks field reads as Borrowed.
                            // Trait impls: E0053 — trait ownership wins (never flip owned → &/&mut).
                            // Payload stores (enum variant / struct field): keep Owned.
                            // Copy aggregates: field method calls are not `&mut` mutations.
                            if !self.in_trait_impl
                                && !trait_impl_owned_string
                                && !param.decorators.iter().any(|d| d.name == "string_ref")
                                && !analyzed.returned_parameters.contains(&param.name)
                                && !payload_forces_owned
                                && !payload_stored
                            {
                                let analyzed_mode = analyzed
                                    .inferred_ownership
                                    .get(&param.name)
                                    .copied()
                                    .or_else(|| {
                                        self.param_ownership_for_formal_demotion(&param.name, analyzed)
                                    });
                                let copy_aggregate = self.is_type_copy(&param.type_);
                                if _debug_formal {
                                    eprintln!("[FORMAL-MUT-BLOCK] fn={} param={} ownership_mode_before={:?} analyzed_mode={:?} copy_aggregate={} mutated={} field_mutated={} payload_stored={} payload_forces={}",
                                        func.name, param.name, ownership_mode, analyzed_mode, copy_aggregate,
                                        analyzed.mutated_parameters.contains(&param.name),
                                        analyzed.field_mutated_parameters.contains(&param.name),
                                        payload_stored, payload_forces_owned);
                                }
                                // Without local field/index writes: field-method calls alone
                                // are not `&mut` mutations (Copy AppDeps *and* non-Copy
                                // `Writer { tag: string }` — post_journal_entry). Passthrough
                                // args (`shift_right(p)`) must keep analyzer MutBorrowed —
                                // `param_passed_as_call_argument` distinguishes that.
                                // Prefer IR when the solver already decided ownership.
                                let false_mut_on_field_methods = self
                                    .param_false_mut_from_readonly_field_methods(
                                        param, func, analyzed, false,
                                    );
                                if _debug_formal {
                                    eprintln!("[FORMAL-FMC] false_mut_on_field_methods={} field_write={} call_arg={} direct_recv={} field_proj_mut={}",
                                        false_mut_on_field_methods,
                                        self.param_has_field_or_index_write(func.body.as_slice(), &param.name),
                                        self.param_passed_as_call_argument(func.body.as_slice(), &param.name, func),
                                        self.param_is_direct_method_receiver(func.body.as_slice(), &param.name),
                                        self.param_has_mut_method_via_field_projection(func.body.as_slice(), &param.name));
                                }
                                if false_mut_on_field_methods
                                    && matches!(ownership_mode, OwnershipMode::MutBorrowed)
                                {
                                    ownership_mode = OwnershipMode::Owned;
                                } else if false_mut_on_field_methods
                                    && matches!(ownership_mode, OwnershipMode::Borrowed)
                                    && !self.inferred_borrowed_params.contains(&param.name)
                                    && !self.param_only_used_via_field_or_index_projection(
                                        func.body.as_slice(),
                                        &param.name,
                                    )
                                {
                                    ownership_mode = OwnershipMode::Owned;
                                } else if !false_mut_on_field_methods
                                    && (matches!(analyzed_mode, Some(OwnershipMode::MutBorrowed))
                                        || (analyzed.mutated_parameters.contains(&param.name)
                                            && !analyzed.returned_parameters.contains(&param.name)))
                                {
                                    ownership_mode = OwnershipMode::MutBorrowed;
                                } else if matches!(analyzed_mode, Some(OwnershipMode::Borrowed))
                                    && !keep_owned_contract
                                    && !copy_aggregate
                                {
                                    ownership_mode = OwnershipMode::Borrowed;
                                }
                            }

                            if _debug_formal {
                                eprintln!("[FORMAL-POST-MUT-BLOCK] ownership_mode={:?}", ownership_mode);
                            }
                            // Readonly unused WJ `string` formals emit `&str` so forward-ref
                            // call sites (DialogCondition → Inventory::has_item) see converged borrow.
                            // Free functions included (wal `replay_all(path)`).
                            // Discard-only `let _ = path` / `let _ = (path, …)` also demote — analyzer
                            // often keeps Owned for the move-into-discard, which must not win here.
                            // Runtime AsRef modules: keep owned `String` + `&` at the call
                            // site only when callees do not already expect a borrow.
                            let asref_fwd = self.param_asref_runtime_forces_owned_formal(
                                func.body.as_slice(),
                                &param.name,
                                func,
                            );
                            let str_ref_ok = (self.str_ref_optimized_params.contains(&param.name)
                                || analyzed.str_ref_optimizable_params.contains(&param.name)
                                || self.param_only_forwards_to_borrowed_text_callees(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                ))
                                && !asref_fwd;
                            if asref_fwd {
                                if _debug_formal {
                                    eprintln!("[FORMAL-ASREF-FWD] fn={} param={} asref_fwd=true → Owned", func.name, param.name);
                                }
                                // Keep owned `String` + call-site `&` for db/env/… AsRef APIs.
                                self.str_ref_optimized_params.remove(&param.name);
                                self.inferred_borrowed_params.remove(&param.name);
                                ownership_mode = OwnershipMode::Owned;
                            }
                            if !self.in_trait_impl
                                && !trait_impl_owned_string
                                && !param.decorators.iter().any(|d| d.name == "string_ref")
                                && !self.param_only_forwards_to_emitted_owned_callees(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                                && (unused_params.contains(&param.name)
                                    || self.param_only_used_in_simple_or_tuple_discard(
                                        func.body.as_slice(),
                                        &param.name,
                                    )
                                    || str_ref_ok)
                                && crate::codegen::rust::types::is_windjammer_text_type(&param.type_)
                                && !matches!(
                                    &param.type_,
                                    Type::Reference(_) | Type::MutableReference(_)
                                )
                                && !payload_stored
                                && !self.param_stored_in_owned_payload(
                                    func.body.as_slice(),
                                    &param.name,
                                )
                            {
                                ownership_mode = OwnershipMode::Borrowed;
                                self.str_ref_optimized_params.insert(param.name.clone());
                                self.inferred_borrowed_params.insert(param.name.clone());
                                self.inferred_mut_borrowed_params.remove(&param.name);
                            }

                            // Terminal signature-driven restores — beat engine stubs /
                            // keep-owned heuristics that would leave map keys Owned or
                            // Copy passthrough formals as owned `mut T`.
                            if param.name != "self"
                                && !self.is_type_copy(&param.type_)
                                && self.param_only_forwarded_to_collection_key_callee(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                )
                            {
                                ownership_mode = OwnershipMode::Borrowed;
                                self.inferred_borrowed_params.insert(param.name.clone());
                            }
                            // E0053: trait-impl formals already match the trait item above.
                            // Stale registry MutBorrowed (self-slot index bleed, empty-body
                            // stubs) must not rewrite owned Copy/non-self trait params to
                            // `&mut T` (`set_camera(camera: CameraData)`).
                            // Custom aggregates demoted by readonly field-method false-mut
                            // (reverse_entry / post_journal_entry) must not be re-mutated
                            // solely from registry MutBorrowed.
                            let false_mut_on_field_methods = self
                                .param_false_mut_from_readonly_field_methods(
                                    param, func, analyzed, true,
                                );
                            // Forward-only owned callees (`create` → owned `post_journal_entry`)
                            // must not be re-promoted to `&mut` from stale registry MutBorrowed.
                            let forwards_to_owned = self.param_only_forwards_to_emitted_owned_callees(
                                func.body.as_slice(),
                                &param.name,
                                func,
                            );
                            if param.name != "self"
                                && !self.in_trait_impl
                                && !forwards_to_owned
                                && (self.param_passed_to_mut_borrowing_callee(
                                    func.body.as_slice(),
                                    &param.name,
                                    func,
                                ) || (matches!(
                                    registry_ownership,
                                    Some(OwnershipMode::MutBorrowed)
                                ) && !false_mut_on_field_methods))
                            {
                                ownership_mode = OwnershipMode::MutBorrowed;
                                self.inferred_mut_borrowed_params.insert(param.name.clone());
                            }
                            if forwards_to_owned
                                && matches!(
                                    ownership_mode,
                                    OwnershipMode::MutBorrowed | OwnershipMode::Borrowed
                                )
                                && matches!(&param.type_, Type::Custom(_))
                                && !crate::codegen::rust::types::is_windjammer_text_type(
                                    &param.type_,
                                )
                            {
                                ownership_mode = OwnershipMode::Owned;
                                self.inferred_mut_borrowed_params.remove(&param.name);
                                self.inferred_borrowed_params.remove(&param.name);
                            }

                            // Copy pass-by-value scalars (`bool`/`int`/`float`): read-only
                            // formals stay owned. IR/registry can spuriously assign MutBorrowed
                            // for params used only in `if cond` (method if-else assignment).
                            if param.name != "self"
                                && crate::type_classification::is_copy_pass_by_value_formal(
                                    &param.type_,
                                )
                                && !analyzed.mutated_parameters.contains(&param.name)
                                && !analyzed.field_mutated_parameters.contains(&param.name)
                            {
                                ownership_mode = OwnershipMode::Owned;
                                self.inferred_mut_borrowed_params.remove(&param.name);
                                self.inferred_borrowed_params.remove(&param.name);
                            }

                            if _debug_formal {
                                eprintln!("[FORMAL-FINAL] fn={} param={} ownership_mode={:?} formal_type={:?} param.is_mutable={} field_mutated={} param_passed_to_owned_self_method={}",
                                    func.name, param.name, ownership_mode, formal_type, param.is_mutable,
                                    analyzed.field_mutated_parameters.contains(&param.name),
                                    self.param_passed_to_owned_self_method_arg(func.body.as_slice(), &param.name, func),
                                );
                            }
                            copy_aggregate_ref_formal.unwrap_or_else(|| match ownership_mode {
                                OwnershipMode::Owned => {
                                    // Body inference may leave `formal_type` as `Reference(T)`
                                    // while facade/forward-ref ownership stays Owned — emit `T`.
                                    let emit_ty = if matches!(
                                        formal_type,
                                        Type::Reference(_) | Type::MutableReference(_)
                                    ) && !matches!(
                                        &param.type_,
                                        Type::Reference(_) | Type::MutableReference(_)
                                    ) {
                                        &param.type_
                                    } else if crate::codegen::rust::types::is_windjammer_text_type(
                                        &param.type_,
                                    ) && !matches!(
                                        &param.type_,
                                        Type::Reference(_) | Type::MutableReference(_)
                                    ) {
                                        &param.type_
                                    } else {
                                        formal_type
                                    };
                                    self.type_to_rust(emit_ty)
                                }
                                OwnershipMode::MutBorrowed => {
                                    // Prefer owned `mut T` for owned sibling / forward-only
                                    // bodies, and for AppDeps-style `&self` port facades.
                                    // Nested `&mut self` field methods stay `&mut T`.
                                    let bare_wj = !matches!(
                                        &param.type_,
                                        Type::Reference(_) | Type::MutableReference(_)
                                    );
                                    let prefer_owned_mut = bare_wj
                                        && !self.param_passed_to_mut_borrowing_callee(
                                            func.body.as_slice(),
                                            &param.name,
                                            func,
                                        )
                                        && (self.param_only_forwards_to_emitted_owned_callees(
                                            func.body.as_slice(),
                                            &param.name,
                                            func,
                                        ) || self.param_passes_to_wj_owned_sibling_call(
                                            func.body.as_slice(),
                                            &param.name,
                                            func,
                                        ) || self.param_is_owned_mut_field_method_facade(
                                            func.body.as_slice(),
                                            &param.name,
                                            func,
                                        ))
                                        && (!matches!(
                                            registry_ownership,
                                            Some(OwnershipMode::MutBorrowed)
                                        ) || self.param_only_forwards_to_emitted_owned_callees(
                                            func.body.as_slice(),
                                            &param.name,
                                            func,
                                        ));
                                    if prefer_owned_mut {
                                        // Analyzer may leave `formal_type` as
                                        // `MutableReference(T)` while WJ source is bare `T`.
                                        // Prefer the bare AST type so we emit `mut T` /
                                        // `T`, not `&mut T` (dogfood).
                                        let emit_ty = if matches!(
                                            formal_type,
                                            Type::Reference(_) | Type::MutableReference(_)
                                        ) {
                                            &param.type_
                                        } else {
                                            formal_type
                                        };
                                        self.type_to_rust(emit_ty)
                                    } else {
                                        // Peel so we never emit nested `&mut &mut T`.
                                        let inner = match formal_type {
                                            Type::Reference(t) | Type::MutableReference(t) => {
                                                t.as_ref()
                                            }
                                            other => other,
                                        };
                                        format!("&mut {}", self.type_to_rust(inner))
                                    }
                                }
                                OwnershipMode::Borrowed if self.is_type_copy(formal_type) => {
                                    // Copy-by-value: owned formal + call-site borrow, except
                                    // field-projection-only aggregates (`run_query` → `&Graph`).
                                    if self.emitted_rust_ref_formals.contains(&param.name)
                                        && !crate::type_classification::is_copy_pass_by_value_formal(
                                            formal_type,
                                        )
                                    {
                                        format!("&{}", self.type_to_rust(formal_type))
                                    } else {
                                        self.type_to_rust(formal_type)
                                    }
                                }
                                OwnershipMode::Borrowed => {
                                    let is_string = matches!(formal_type, Type::String)
                                        || matches!(formal_type, Type::Custom(ref name) if name == "string");
                                    if is_string && !trait_impl_owned_string {
                                        // Only force owned `String` when explicitly marked
                                        // (collection_key_owned_params) or when
                                        // `is_collection_key_owned_param` agrees — that helper
                                        // already respects str_ref / inferred Borrowed so
                                        // HashMap lookup keys can emit `&str` and avoid
                                        // `get(&key)` double-ref on an already-borrowed formal.
                                        if self.collection_key_owned_params.contains(&param.name)
                                            || self.is_collection_key_owned_param(param, func)
                                            || self.param_passes_to_wj_owned_sibling_call(
                                                func.body.as_slice(),
                                                &param.name,
                                                func,
                                            )
                                            || self.param_only_forwards_to_emitted_owned_callees(
                                                func.body.as_slice(),
                                                &param.name,
                                                func,
                                            )
                                        {
                                            self.type_to_rust(formal_type)
                                        } else if param
                                            .decorators
                                            .iter()
                                            .any(|d| d.name == "string_ref")
                                            || self.param_passed_to_slice_search_string_elem(
                                                func.body.as_slice(),
                                                &param.name,
                                                func,
                                            )
                                        {
                                            // @string_ref and Vec<String>::contains need &String.
                                            "&String".to_string()
                                        } else {
                                        let registry_str_ref = self
                                            .get_signature_with_global(&func.name)
                                            .and_then(|sig| sig.param_types.get(param_idx))
                                            .is_some_and(|ty| {
                                                matches!(
                                                    ty,
                                                    Type::Reference(inner)
                                                        if matches!(
                                                            &**inner,
                                                            Type::Custom(n) if n == "str"
                                                        )
                                                )
                                            });
                                        let registry_string_ref = self
                                            .get_signature_with_global(&func.name)
                                            .and_then(|sig| sig.param_types.get(param_idx))
                                            .is_some_and(|ty| {
                                                crate::codegen::rust::string_utilities::param_is_rust_string_ref(ty)
                                            });
                                        if registry_string_ref {
                                            if param.decorators.iter().any(|d| d.name == "string_ref")
                                                || self.param_passed_to_slice_search_string_elem(
                                                    func.body.as_slice(),
                                                    &param.name,
                                                    func,
                                                )
                                                || self.param_passed_to_string_ref_formal_callee(
                                                    func.body.as_slice(),
                                                    &param.name,
                                                    func,
                                                )
                                            {
                                                "&String".to_string()
                                            } else {
                                                "&str".to_string()
                                            }
                                        } else if self.param_passed_to_slice_search_string_elem(
                                            func.body.as_slice(),
                                            &param.name,
                                            func,
                                        ) || self.param_passed_to_string_ref_formal_callee(
                                            func.body.as_slice(),
                                            &param.name,
                                            func,
                                        ) {
                                            "&String".to_string()
                                        } else if (self.str_ref_optimized_params.contains(&param.name)
                                            || registry_str_ref)
                                            && ownership_mode
                                                != OwnershipMode::Owned
                                            && !payload_stored
                                        {
                                            // Discard-only / unused `string` formals: analyzer may
                                            // still say Owned (move into `let _ = path`) — trust
                                            // str_ref_optimized / registry &str over that.
                                            "&str".to_string()
                                        } else if (payload_stored || payload_forces_owned)
                                            && !self.current_ir_function.as_ref().is_some_and(|ir| {
                                                ir.str_ref_params.contains(&param.name)
                                            })
                                        {
                                            // Builder/setter: WJ formal is owned `string` stored
                                            // into an owned field — never default to `&str`
                                            // unless IR explicitly listed the param in str_ref_params.
                                            self.type_to_rust(&param.type_)
                                        } else {
                                            // Default borrowed string formals to &str
                                            // (idiomatic Rust, accepts both String and &str).
                                            // &String is only emitted by the @string_ref /
                                            // slice-search guard above.
                                            "&str".to_string()
                                        }
                                        }
                                    } else if is_string && trait_impl_owned_string {
                                        // E0053: trait contract is owned String, not &String.
                                        self.type_to_rust(formal_type)
                                    } else if crate::codegen::rust::stdlib_method_traits::is_map_type(
                                        formal_type,
                                    ) || crate::codegen::rust::stdlib_method_traits::is_set_type(
                                        formal_type,
                                    ) {
                                        // Bare HashMap/Set formals stay owned (IR/signature_bridge
                                        // contract). Readonly `.get()` must not flip to `&HashMap`
                                        // while call sites still pass owned maps.
                                        self.type_to_rust(formal_type)
                                    } else {
                                        format!("&{}", self.type_to_rust(formal_type))
                                    }
                                }
                            })
                        }
                    }
                };

                // WINDJAMMER LIFETIME INFERENCE: Add 'a lifetime to reference parameters
                // when the function needs explicit lifetime annotations.
                type_str = if needs_lifetime && param.name != "self" {
                    if let Some(stripped) = type_str.strip_prefix("&mut ") {
                        format!("&'a mut {}", stripped)
                    } else if let Some(stripped) = type_str.strip_prefix("&") {
                        format!("&'a {}", stripped)
                    } else {
                        type_str
                    }
                } else {
                    type_str
                };

                // Copy scalars (`int`/`float`/`bool`): never emit `&T` / read-only `&mut T`.
                // Copy aggregates: strip spurious readonly `&T` from field reads; keep `&mut T`
                // for direct mutation and passthrough to mutating callees.
                if param.name != "self"
                    && type_str.starts_with('&')
                    && crate::type_classification::is_copy_pass_by_value_formal(&param.type_)
                    && !analyzed.mutated_parameters.contains(&param.name)
                    && !analyzed.field_mutated_parameters.contains(&param.name)
                {
                    type_str = self.type_to_rust(&param.type_);
                } else if param.name != "self"
                    && type_str.starts_with('&')
                    && !type_str.starts_with("&mut ")
                    && (self.is_type_copy(&param.type_)
                        && !crate::type_classification::is_copy_pass_by_value_formal(&param.type_)
                        && !crate::analyzer::field_enum_borrow::param_only_used_as_field_enum_match_scrutinee(
                            &param.name,
                            func.body.as_slice(),
                        )
                        && !self.inferred_borrowed_params.contains(&param.name)
                        && !self.emitted_rust_ref_formals.contains(&param.name))
                {
                    type_str = self.type_to_rust(&param.type_);
                }

                // Explicitly Copy types kept as owned (instead of &mut) still need `mut`
                // when the analyzer inferred MutBorrowed — the body mutates the value.
                let copy_mut_as_owned = self.is_type_copy(formal_type)
                    && !type_str.starts_with('&')
                    && self.inferred_mut_borrowed_params.contains(&param.name);

                // Copy scalar owned formals pass by value — clear stale borrow metadata.
                if self.is_type_copy(formal_type)
                    && !type_str.starts_with('&')
                    && crate::type_classification::is_copy_pass_by_value_formal(formal_type)
                {
                    self.inferred_borrowed_params.remove(&param.name);
                    self.inferred_mut_borrowed_params.remove(&param.name);
                }

                // Keep call-site borrow tracking aligned with emitted Rust formals.
                // `name: &String` must not get `contains(&name)` at call sites.
                if param.name != "self" {
                    if type_str.starts_with("&mut ") || type_str.starts_with("&'a mut ") {
                        self.emitted_rust_ref_formals.insert(param.name.clone());
                        self.inferred_mut_borrowed_params.insert(param.name.clone());
                        self.inferred_borrowed_params.remove(&param.name);
                        if param.name != "self" {
                            let user_arg_idx = func
                                .parameters
                                .iter()
                                .filter(|p| p.name != "self")
                                .position(|p| p.name == param.name)
                                .unwrap_or(param_idx);
                            self.current_fn_emitted_mut_arg_indices
                                .insert(user_arg_idx);
                        }
                    } else if type_str.starts_with('&') {
                        self.emitted_rust_ref_formals.insert(param.name.clone());
                        self.inferred_borrowed_params.insert(param.name.clone());
                        self.inferred_mut_borrowed_params.remove(&param.name);
                        if type_str == "&str"
                            || type_str.starts_with("&'a str")
                            || type_str.ends_with(" str")
                        {
                            self.str_ref_optimized_params.insert(param.name.clone());
                        }
                    } else {
                        self.emitted_rust_ref_formals.remove(&param.name);
                        // Owned emitted formal: stale analyzer borrow metadata must not
                        // suppress call-site `&` (dogfood put_value → key_in_latest_base(&key)).
                        self.inferred_borrowed_params.remove(&param.name);
                        self.inferred_mut_borrowed_params.remove(&param.name);
                        self.str_ref_optimized_params.remove(&param.name);
                        if param.name != "self" {
                            let user_arg_idx = func
                                .parameters
                                .iter()
                                .filter(|p| p.name != "self")
                                .position(|p| p.name == param.name)
                                .unwrap_or(param_idx);
                            self.current_fn_emitted_mut_arg_indices
                                .remove(&user_arg_idx);
                        }
                    }
                }

                // TDD FIX: Auto-infer `mut` for owned parameters
                // THE WINDJAMMER WAY: Users don't track mutability - the compiler does.
                // If a parameter has mutating method calls or field mutations,
                // the binding needs `mut` even if not explicitly written.
                let auto_needs_mut = param.name != "self"
                    && !param.is_mutable
                    && matches!(type_str.as_str(), s if !s.starts_with("&"))
                    && (self.variable_needs_mut(&param.name) || copy_mut_as_owned);
                let mut_prefix = if (param.is_mutable || auto_needs_mut)
                    && !type_str.starts_with('&')
                {
                    "mut "
                } else {
                    ""
                };

                // TDD FIX: Prefix unused parameter names with `_` to suppress warnings
                let display_name = if unused_params.contains(&param.name) {
                    format!("_{}", param.name)
                } else {
                    param.name.clone()
                };

                // Check if this is a pattern parameter
                if let Some(pattern) = &param.pattern {
                    // Generate pattern: type syntax
                    format!(
                        "{}{}: {}",
                        mut_prefix,
                        self.generate_pattern(pattern),
                        type_str
                    )
                } else {
                    // Simple name: type syntax
                    format!("{}{}: {}", mut_prefix, display_name, type_str)
                }
            })
            .collect()
    }

    /// Shared guard: never demote a param that needs mutable access to shared `&T`.
    ///
    /// Owned (`mut x: T`) and MutBorrowed (`x: &mut T`) both satisfy `&mut self` callees;
    /// only shared `&T` is wrong. Driven by analyzer mutation sets + IR/analyzer ownership.
    fn param_must_not_demote_to_shared_borrow(
        &self,
        param_name: &str,
        analyzed: &AnalyzedFunction<'_>,
        ownership_hint: Option<OwnershipMode>,
    ) -> bool {
        // Phase-2 &str lowering already validated this param — beat stale mutation flags
        // from match/get patterns (`lookup_borrowed` key used only in `map.get(key)`).
        if analyzed.str_ref_optimizable_params.contains(param_name) {
            return false;
        }
        if matches!(ownership_hint, Some(OwnershipMode::MutBorrowed)) {
            return true;
        }
        if matches!(
            self.get_param_ownership(param_name, analyzed),
            Some(OwnershipMode::MutBorrowed)
        ) {
            return true;
        }
        if matches!(
            self.param_ownership_for_formal_demotion(param_name, analyzed),
            Some(OwnershipMode::Owned)
        ) {
            // Multiparam store forwards intentionally demote Owned formals to `&T`
            // and `.clone()` at owned callee sites (regression-047 `apply_patch_put`).
            if let Some(param) = analyzed
                .decl
                .parameters
                .iter()
                .find(|p| p.name == *param_name)
            {
                if self.param_should_emit_borrowed_delegation_formal(param, &analyzed.decl) {
                    return false;
                }
            }
            return true;
        }
        if self.param_stored_in_owned_payload(analyzed.decl.body.as_slice(), param_name) {
            return true;
        }
        analyzed.mutated_parameters.contains(param_name)
            && !analyzed.returned_parameters.contains(param_name)
    }
}
