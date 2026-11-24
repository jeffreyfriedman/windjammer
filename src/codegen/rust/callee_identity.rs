//! Callee signatures keyed by declaration: a bare call of a file's own function reads that function, never a
//! same-named function of another module, and its numeric formals decide its argument casts.

use crate::codegen::rust::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// A bare call `f(..)` names a free function, never a same-named method (C3 identity), and a parameter or
    /// local of that name (`accept: fn(int) -> bool`) shadows every function: no registry signature then.
    pub(crate) fn resolve_bare_free_call_signature(&self, name: &str, n: usize) -> Option<crate::codegen::rust::call_signature_resolution::ResolvedSignature> {
        let shadowed = !name.contains("::") && (self.current_function_params.iter().any(|p| p.name == name) || self.local_variable_scopes.iter().any(|s| s.contains(name)));
        let own = if shadowed { None } else { self.own_file_definition(name) };
        self.resolve_call_signature_of(name, None, n, own.as_ref()).filter(|r| !shadowed && (name.contains("::") || !r.sig.has_self_receiver))
    }

    /// `name(..)` names a function this file declares at top level: not a parameter or local, not a sibling
    /// method. A struct field of the same name is a different declaration and never the callee.
    pub(crate) fn bare_callee_is_declared_free_fn(&self, name: &str) -> bool {
        !name.contains("::")
            && self.idiom.file_scope_fn_names.contains(name)
            && !self.current_function_params.iter().any(|p| p.name == name)
            && !self.local_variable_scopes.iter().any(|s| s.contains(name))
            && !self.bare_call_is_sibling_method(name)
    }

    /// The text of a call's callee. A struct field of the same name as a declared free function does not make the
    /// call a field read (`self.f(..)`).
    pub(crate) fn generate_callee_expression(&mut self, function: &crate::parser::Expression<'ast>, func_name: &str) -> String {
        let text = self.generate_expression(function);
        let names_free_fn = matches!(function, crate::parser::Expression::Identifier { .. }) && self.bare_callee_is_declared_free_fn(func_name);
        match text.strip_prefix("self.") {
            Some(bare) if names_free_fn => bare.to_string(),
            _ => text,
        }
    }

    pub(crate) fn resolve_call_signature_with_global(
        &self,
        func_name: &str,
        receiver_type: Option<&str>,
        arg_count: usize,
    ) -> Option<crate::codegen::rust::call_signature_resolution::ResolvedSignature> {
        self.resolve_call_signature_of(func_name, receiver_type, arg_count, None)
    }

    /// `own`: the declaration a bare call names lexically (this file's function). A global entry that declares a
    /// different function is a same-named function of another module: it is never read, not even for metadata.
    fn resolve_call_signature_of(
        &self,
        func_name: &str,
        receiver_type: Option<&str>,
        arg_count: usize,
        own: Option<&crate::analyzer::FunctionSignature>,
    ) -> Option<crate::codegen::rust::call_signature_resolution::ResolvedSignature> {
        let same_function = |r: &crate::codegen::rust::call_signature_resolution::ResolvedSignature| {
            own.is_none_or(|o| o.declares_same_function(&r.sig))
        };
        let owned_name = self.imported_runtime_qualified_callee(func_name);
        let import_qualified = self.import_fn_alias_map.get(func_name).cloned();
        let resolved_callee = import_qualified
            .or(owned_name)
            .unwrap_or_else(|| func_name.to_string());
        let caller_module = self.current_caller_module_path();
        let local = crate::codegen::rust::call_signature_resolution::resolve_call_signature(
            &self.signature_registry,
            resolved_callee.as_str(),
            receiver_type,
            arg_count,
            &self.module_alias_map,
            &self.import_fn_alias_map,
            caller_module.as_deref(),
        );
        let global = self.global_signature_registry.as_ref().and_then(|global| {
            crate::codegen::rust::call_signature_resolution::resolve_call_signature(
                global,
                resolved_callee.as_str(),
                receiver_type,
                arg_count,
                &self.module_alias_map,
                &self.import_fn_alias_map,
                caller_module.as_deref(),
            )
        }).filter(same_function);
        // `use dep::module::f`: the dependency function the import declares, by its registered key.
        let imported = || {
            let registries = std::iter::once(&self.signature_registry).chain(self.global_signature_registry.as_deref());
            registries.filter_map(|reg| reg.imported_free_function(&resolved_callee)).next().map(|(key, sig)| {
                crate::codegen::rust::call_signature_resolution::ResolvedSignature {
                    sig: sig.clone(),
                    qualified_key: key,
                    resolution_method: crate::codegen::rust::call_signature_resolution::ResolutionMethod::ModuleAlias,
                    has_collision: false,
                }
            })
        };
        let local = local.or_else(|| (global.is_none() && resolved_callee.contains("::")).then(imported).flatten());
        let codegen_refresh_source = global
            .clone()
            .filter(|r| r.sig.emitted_rust_ref_params.is_some())
            .or_else(|| {
                local
                    .clone()
                    .filter(|r| r.sig.emitted_rust_ref_params.is_some())
            });
        let picked = crate::codegen::rust::call_signature_resolution::pick_best_resolved_signature(
            local, global,
        );
        if let Some(ref resolved) = picked {
            if crate::codegen::rust::call_signature_resolution::has_stale_owned_non_copy_params(
                &resolved.sig,
            ) {
                if let Some(global_reg) = self.global_signature_registry.as_ref() {
                    if let Some(global_only) =
                        crate::codegen::rust::call_signature_resolution::resolve_call_signature(
                            global_reg,
                            resolved_callee.as_str(),
                            receiver_type,
                            arg_count,
                            &self.module_alias_map,
                            &self.import_fn_alias_map,
                            caller_module.as_deref(),
                        )
                        .filter(same_function)
                    {
                        if !crate::codegen::rust::call_signature_resolution::has_stale_owned_non_copy_params(
                            &global_only.sig,
                        ) {
                            return Some(global_only);
                        }
                    }
                }
            }
        }
        picked.map(|mut resolved| {
            if resolved.sig.emitted_rust_ref_params.is_none() {
                if let Some(alt) = &codegen_refresh_source {
                    crate::codegen::rust::signature_promotion::merge_codegen_refresh_metadata(
                        &mut resolved.sig,
                        &alt.sig,
                    );
                }
            }
            resolved.sig =
                crate::codegen::rust::call_signature_resolution::finalize_call_site_signature(
                    resolved.sig,
                );
            resolved
        })
    }
}
