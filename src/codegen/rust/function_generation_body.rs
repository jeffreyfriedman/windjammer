//! Return type / where-clause / brace and generated body for regular functions.

use crate::codegen::rust::codegen_helpers;
use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn append_regular_function_return_where_open_brace(
        &mut self,
        func: &FunctionDecl<'ast>,
        output: &mut String,
        needs_lifetime: bool,
    ) {
        if let Some(return_type) = &func.return_type {
            output.push_str(" -> ");
            if needs_lifetime {
                output.push_str(&crate::codegen::rust::types::type_to_rust_with_lifetime(
                    return_type,
                ));
            } else {
                output.push_str(&self.type_to_rust(return_type));
            }
        }

        // Add where clause if present (impl-level + per-method inferred bounds).
        let mut where_clause = func.where_clause.clone();
        if self.current_impl_generic_type_params.len() == 1 {
            let inferred = codegen_helpers::infer_clone_where_bounds_for_function(
                func,
                &self.current_impl_generic_type_params,
            );
            if !inferred.is_empty() {
                where_clause = codegen_helpers::merge_where_clauses(where_clause, inferred);
            }
        }
        output.push_str(&codegen_helpers::format_where_clause(&where_clause));

        output.push_str(" {\n");
        self.indent_level += 1;
    }

    pub(in crate::codegen::rust) fn append_regular_function_body_and_close(
        &mut self,
        func: &FunctionDecl<'ast>,
        output: &mut String,
    ) {
        // TDD: Generate function body with return optimization
        // Set flag to enable implicit return for last statement
        let old_in_function_body = self.in_function_body;
        self.in_function_body = true;
        let import_scope = self.push_function_local_import_scope(&func.body);
        let mut body_code = self.generate_block(&func.body);
        self.pop_function_local_import_scope(import_scope);
        self.in_function_body = old_in_function_body;

        // PHASE 6 OPTIMIZATION: Add defer drop logic before function returns
        // This defers heavy deallocations to a background thread for 10,000x speedup
        if !self.defer_drop_optimizations.is_empty() {
            body_code =
                self.wrap_with_defer_drop(body_code, &self.defer_drop_optimizations.clone());
        }

        output.push_str(&body_code);

        self.indent_level -= 1;
        output.push('}');
    }

    /// File-scope `use std::strings` is recorded before any body runs. A
    /// function-local import is only in scope for that body, so call sites
    /// (`strings.len`) see the module and emit `strings::len`.
    fn push_function_local_import_scope(
        &mut self,
        body: &[&Statement<'ast>],
    ) -> FunctionLocalImportScope {
        let mut added_modules = Vec::new();
        let mut added_alias_keys = Vec::new();
        let mut added_roots = Vec::new();
        for stmt in body {
            let Statement::Use { path, alias, .. } = stmt else {
                continue;
            };
            if let Some(alias_name) = alias {
                if self.module_alias_map.get(alias_name).is_none() {
                    if let Some(last) = path.last() {
                        self.module_alias_map
                            .insert(alias_name.clone(), last.clone());
                        added_alias_keys.push(alias_name.clone());
                    }
                }
            }
            if path.first().is_some_and(|p| p == "std") {
                if path.len() >= 2 {
                    let module = &path[1];
                    if module
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_lowercase())
                        && self.runtime_std_module_imports.insert(module.clone())
                    {
                        added_modules.push(module.clone());
                    }
                }
            } else if let Some(first) = path.first() {
                if !matches!(first.as_str(), "crate" | "super" | "self")
                    && self.imported_path_roots.insert(first.clone())
                {
                    added_roots.push(first.clone());
                }
            }
        }
        FunctionLocalImportScope {
            added_modules,
            added_alias_keys,
            added_roots,
        }
    }

    fn pop_function_local_import_scope(&mut self, scope: FunctionLocalImportScope) {
        for name in scope.added_modules {
            self.runtime_std_module_imports.remove(&name);
        }
        for name in scope.added_alias_keys {
            self.module_alias_map.remove(&name);
        }
        for name in scope.added_roots {
            self.imported_path_roots.remove(&name);
        }
    }
}

struct FunctionLocalImportScope {
    added_modules: Vec<String>,
    added_alias_keys: Vec<String>,
    added_roots: Vec<String>,
}
