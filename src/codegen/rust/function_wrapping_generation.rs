//! Function generation: decorator wrapping (`@timeout`, `@bench`, `@requires`, etc.)

use crate::analyzer::*;
use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Check if function has decorators that need to wrap the function body
    pub(super) fn has_wrapping_decorator(&self, func: &FunctionDecl<'ast>) -> bool {
        func.decorators.iter().any(|d| {
            matches!(
                d.name.as_str(),
                "timeout" | "bench" | "requires" | "ensures" | "property_test" | "invariant"
            ) || (d.name == "test" && !d.arguments.is_empty())
        })
    }

    /// Generate function with decorator wrapping (timeout, bench, requires, ensures, etc.)
    pub(super) fn generate_function_with_wrapping(
        &mut self,
        analyzed: &AnalyzedFunction<'ast>,
    ) -> String {
        let func = &analyzed.decl;
        let mut output = String::new();

        // TDD FIX: Auto-add #[test] attribute for test functions in test files (EARLY CHECK)
        // THE WINDJAMMER WAY: Test files (*_test.wj) should auto-generate test attributes
        // Bug: Tests don't run because #[test] attributes are missing
        // Root Cause: Codegen doesn't detect test files and test functions
        // Fix: Check if filename ends with _test.wj AND function starts with test_
        let filename_str = self.current_wj_file.to_string_lossy();
        let is_test_file = filename_str.ends_with("_test.wj") || filename_str.contains("_test.wj");
        let is_test_function = func.name.starts_with("test_");
        let has_test_decorator = func.decorators.iter().any(|d| d.name == "test");
        let has_property_test = func.decorators.iter().any(|d| d.name == "property_test");

        if is_test_file && is_test_function && !has_test_decorator && !has_property_test {
            output.push_str("#[test]\n");
        }

        // Generate doc comment if present
        if let Some(doc_comment) = &func.doc_comment {
            for line in doc_comment.lines() {
                output.push_str(&format!("/// {}\n", line.trim()));
            }
        }

        // Check for @async decorator
        let is_async = func.decorators.iter().any(|d| d.name == "async");
        if is_async && func.name == "main" {
            output.push_str("#[tokio::main]\n");
        }

        // Generate non-wrapping decorators (like @test, @ignore)
        let decorator_reg = crate::decorator_registry::DecoratorRegistry::new();
        for decorator in &func.decorators {
            if decorator_reg.should_skip_for_backend(&decorator.name, self.target) {
                continue;
            }
            if decorator_reg.is_wrapping_decorator(&decorator.name) {
                continue;
            }
            // Skip @test with arguments (setup/teardown) - handled in body
            if decorator.name == "test" && !decorator.arguments.is_empty() {
                continue;
            }

            let rust_attr = self.map_decorator(&decorator.name);
            if decorator.arguments.is_empty() {
                output.push_str(&format!("#[{}]\n", rust_attr));
            }
        }

        // Add #[test] attribute for @property_test decorated functions
        let has_property_test = func.decorators.iter().any(|d| d.name == "property_test");
        if has_property_test {
            output.push_str("#[test]\n");
        }

        // PHASE 1: Suppress Clippy warnings for &String parameters
        // We use &String (not &str) for correctness with Vec<String>, but Clippy warns
        // Phase 2 will optimize to &str when safe
        let has_borrowed_string_param = analyzed
            .inferred_ownership
            .iter()
            .any(|(_, ownership)| matches!(ownership, OwnershipMode::Borrowed))
            && func.parameters.iter().enumerate().any(|(idx, param)| {
                let inferred_type = analyzed
                    .inferred_param_types
                    .get(idx)
                    .unwrap_or(&param.type_);
                matches!(inferred_type, Type::String)
                    || matches!(inferred_type, Type::Custom(ref name) if name == "string")
            });

        if has_borrowed_string_param {
            output.push_str("#[allow(clippy::ptr_arg)]\n");
        }

        // Function signature
        let has_export = func.decorators.iter().any(|d| d.name == "export");
        if !self.in_trait_impl
            && (func.is_pub || self.in_wasm_bindgen_impl || self.is_module || has_export)
        {
            output.push_str("pub ");
        }

        if is_async {
            output.push_str("async ");
        }

        output.push_str("fn ");
        output.push_str(&func.name);

        // TDD FIX: Preserve generic type parameters in wrapping path (e.g. @test, @timeout)
        // Bug: E0425 - "cannot find type 'T' in this scope" when generic fn has decorators
        if !func.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&func.type_params));
            output.push('>');
        }

        output.push('(');

        // For @property_test, remove parameters (they become generators)
        let has_property_test = func.decorators.iter().any(|d| d.name == "property_test");

        // For @test(setup/teardown), remove parameters (they come from setup)
        let has_setup_teardown = func
            .decorators
            .iter()
            .any(|d| d.name == "test" && !d.arguments.is_empty());

        if !has_property_test && !has_setup_teardown {
            // Generate normal parameters
            let params: Vec<String> = func
                .parameters
                .iter()
                .enumerate()
                .map(|(idx, param)| {
                    let param_type = analyzed
                        .inferred_param_types
                        .get(idx)
                        .unwrap_or(&param.type_);
                    let ownership = analyzed
                        .inferred_ownership
                        .get(&param.name)
                        .unwrap_or(&crate::analyzer::OwnershipMode::Owned);
                    let rust_type = self.type_to_rust(param_type);

                    // THE WINDJAMMER WAY: Owned parameters are always mutable
                    // TDD FIX: Borrowed string params use &String (not &str) for correctness
                    // While &str is more idiomatic, &String is CORRECT when interfacing with
                    // generic stdlib code like Vec<String>::contains which expects &String
                    match ownership {
                        crate::analyzer::OwnershipMode::Borrowed => {
                            format!("{}: &{}", param.name, rust_type)
                        }
                        crate::analyzer::OwnershipMode::MutBorrowed => {
                            format!("{}: &mut {}", param.name, rust_type)
                        }
                        crate::analyzer::OwnershipMode::Owned => {
                            format!("mut {}: {}", param.name, rust_type)
                        }
                    }
                })
                .collect();
            output.push_str(&params.join(", "));
        }

        output.push(')');

        // Return type (not for @property_test or @test(setup/teardown))
        if !has_property_test && !has_setup_teardown {
            if let Some(return_type) = &func.return_type {
                output.push_str(" -> ");
                output.push_str(&self.type_to_rust(return_type));
            }
        }

        output.push_str(" {\n");
        self.indent_level += 1;

        // Generate wrapped body
        output.push_str(&self.generate_wrapped_function_body(analyzed));

        self.indent_level -= 1;
        output.push_str("}\n\n");

        output
    }

    /// Generate function body with decorator wrapping
    pub(super) fn generate_wrapped_function_body(
        &mut self,
        analyzed: &AnalyzedFunction<'ast>,
    ) -> String {
        let func = &analyzed.decl;
        let mut output = String::new();

        // Collect decorators
        let timeout_decorator = func.decorators.iter().find(|d| d.name == "timeout");
        let bench_decorator = func.decorators.iter().find(|d| d.name == "bench");
        let requires_decorators: Vec<_> = func
            .decorators
            .iter()
            .filter(|d| d.name == "requires")
            .collect();
        let ensures_decorators: Vec<_> = func
            .decorators
            .iter()
            .filter(|d| d.name == "ensures")
            .collect();
        let invariant_decorators: Vec<_> = func
            .decorators
            .iter()
            .filter(|d| d.name == "invariant")
            .collect();
        let property_test_decorator = func.decorators.iter().find(|d| d.name == "property_test");
        let test_decorator = func
            .decorators
            .iter()
            .find(|d| d.name == "test" && !d.arguments.is_empty());

        // Handle @property_test
        if let Some(prop_decorator) = property_test_decorator {
            let iterations = if let Some((_, expr)) = prop_decorator.arguments.first() {
                self.generate_expression_immut(expr)
            } else {
                "100".to_string()
            };

            output.push_str(&self.indent());
            output.push_str(&format!(
                "property_test_with_gen{}({},\n",
                func.parameters.len(),
                iterations
            ));
            self.indent_level += 1;

            // Generate generators for each parameter
            for param in &func.parameters {
                output.push_str(&self.indent());
                output.push_str(&format!(
                    "|| rand::random::<{}>(),\n",
                    self.type_to_rust(&param.type_)
                ));
            }

            // Generate test closure with typed parameters
            output.push_str(&self.indent());
            output.push('|');
            let param_with_types: Vec<String> = func
                .parameters
                .iter()
                .map(|p| format!("{}: {}", p.name, self.type_to_rust(&p.type_)))
                .collect();
            output.push_str(&param_with_types.join(", "));
            output.push_str("| {\n");
            self.indent_level += 1;

            // Generate body
            for stmt in &func.body {
                output.push_str(&self.generate_statement(stmt));
            }

            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push_str("}\n");
            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push_str(");\n");

            return output;
        }

        // Handle @test(setup=fn, teardown=fn)
        if let Some(test_dec) = test_decorator {
            let mut setup_fn = None;
            let mut teardown_fn = None;

            for (key, expr) in &test_dec.arguments {
                if key == "setup" {
                    setup_fn = Some(self.generate_expression_immut(expr));
                } else if key == "teardown" {
                    teardown_fn = Some(self.generate_expression_immut(expr));
                }
            }

            output.push_str(&self.indent());
            output.push_str("with_setup_teardown(\n");
            self.indent_level += 1;

            output.push_str(&self.indent());
            output.push_str(&format!(
                "{},\n",
                setup_fn.unwrap_or_else(|| "|| ()".to_string())
            ));
            output.push_str(&self.indent());
            output.push_str(&format!(
                "{},\n",
                teardown_fn.unwrap_or_else(|| "|_| ()".to_string())
            ));

            output.push_str(&self.indent());
            output.push('|');
            if !func.parameters.is_empty() {
                output.push_str(&func.parameters[0].name);
            } else {
                output.push_str("_resource");
            }
            output.push_str("| {\n");
            self.indent_level += 1;

            // Generate body
            for stmt in &func.body {
                output.push_str(&self.generate_statement(stmt));
            }

            // Return the resource
            output.push_str(&self.indent());
            if !func.parameters.is_empty() {
                output.push_str(&func.parameters[0].name);
            } else {
                output.push_str("_resource");
            }
            output.push('\n');

            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push_str("}\n");
            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push_str(");\n");

            return output;
        }

        // Start with timeout wrapper if present
        let needs_timeout = timeout_decorator.is_some();
        if needs_timeout {
            let timeout_ms = if let Some((_, expr)) = timeout_decorator.unwrap().arguments.first() {
                self.generate_expression_immut(expr)
            } else {
                "1000".to_string()
            };

            output.push_str(&self.indent());
            output.push_str(&format!(
                "windjammer_runtime::timeout::with_timeout(std::time::Duration::from_millis({}), || {{\n",
                timeout_ms
            ));
            self.indent_level += 1;
        }

        // Start with bench wrapper if present
        let needs_bench = bench_decorator.is_some();
        if needs_bench {
            output.push_str(&self.indent());
            output.push_str("let _bench_result = windjammer_runtime::bench::bench(|| {\n");
            self.indent_level += 1;
        }

        // Add @requires checks (preconditions)
        for req_decorator in requires_decorators {
            if let Some((_, expr)) = req_decorator.arguments.first() {
                let condition = self.generate_expression_immut(expr);
                output.push_str(&self.indent());
                output.push_str(&format!(
                    "windjammer_runtime::test::requires({}, \"{}\");\n",
                    condition, condition
                ));
            }
        }

        // If we have @ensures, wrap body in a block and capture result
        let needs_ensures = !ensures_decorators.is_empty();

        // THE WINDJAMMER WAY: Clone owned parameters that are referenced in @ensures
        // This prevents E0382 errors when parameters are moved in the function body
        if needs_ensures {
            // Collect parameter names referenced in @ensures conditions
            let mut params_in_ensures = std::collections::HashSet::new();
            for ens_decorator in &ensures_decorators {
                if let Some((_, expr)) = ens_decorator.arguments.first() {
                    let condition = self.generate_expression_immut(expr);
                    // Extract parameter names from the condition
                    for param in &func.parameters {
                        if condition.contains(&param.name) {
                            params_in_ensures.insert(param.name.clone());
                        }
                    }
                }
            }

            // Clone owned parameters that appear in @ensures
            for param in &func.parameters {
                if params_in_ensures.contains(&param.name) {
                    let ownership = analyzed
                        .inferred_ownership
                        .get(&param.name)
                        .unwrap_or(&crate::analyzer::OwnershipMode::Owned);

                    // Only clone Owned parameters (borrowed ones can be used multiple times)
                    if matches!(ownership, crate::analyzer::OwnershipMode::Owned) {
                        output.push_str(&self.indent());
                        output.push_str(&format!(
                            "let __{}__for_ensures = {}.clone();\n",
                            param.name, param.name
                        ));
                    }
                }
            }

            output.push_str(&self.indent());
            output.push_str("let __result = {\n");
            self.indent_level += 1;
        }

        // Generate function body
        // THE WINDJAMMER WAY: Treat last expression specially (no semicolon for return value)
        // TDD FIX: Also convert explicit `return expr` to implicit return when last statement
        let body_len = func.body.len();
        for (i, stmt) in func.body.iter().enumerate() {
            let is_last = i == body_len - 1;

            // If this is the last statement, use implicit return (suppress `return` keyword)
            if is_last
                && matches!(
                    stmt,
                    Statement::Expression { .. } | Statement::Return { .. }
                )
            {
                match stmt {
                    Statement::Expression { expr, .. } => {
                        output.push_str(&self.indent());
                        output.push_str(&self.generate_expression(expr));
                        output.push('\n');
                    }
                    Statement::Return {
                        value: Some(expr), ..
                    } => {
                        // TDD FIX: Convert explicit `return expr` to implicit return
                        // Generates idiomatic Rust without Clippy warnings
                        output.push_str(&self.indent());
                        output.push_str(&self.generate_expression(expr));
                        output.push('\n');
                    }
                    Statement::Return { value: None, .. } => {
                        // Void return as last statement — omit entirely (function returns () implicitly)
                    }
                    _ => unreachable!(),
                }
            } else {
                // Not last statement — generate normally (early returns keep `return` keyword)
                output.push_str(&self.generate_statement(stmt));
            }
        }

        // Add @invariant checks (after function body)
        for inv_decorator in &invariant_decorators {
            if let Some((_, expr)) = inv_decorator.arguments.first() {
                let condition = self.generate_expression_immut(expr);
                output.push_str(&self.indent());
                output.push_str(&format!(
                    "windjammer_runtime::test::invariant({}, \"{}\");\n",
                    condition, condition
                ));
            }
        }

        // Close @ensures block and add checks
        if needs_ensures {
            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push_str("};\n");

            for ens_decorator in ensures_decorators {
                if let Some((_, expr)) = ens_decorator.arguments.first() {
                    let mut condition = self.generate_expression_immut(expr);
                    // Replace 'result' with '__result' in ensures conditions
                    condition = condition.replace("result", "__result");

                    // Replace parameter names with cloned versions
                    // Replace "name" but not ".name" (field access)
                    for param in &func.parameters {
                        let ownership = analyzed
                            .inferred_ownership
                            .get(&param.name)
                            .unwrap_or(&crate::analyzer::OwnershipMode::Owned);

                        if matches!(ownership, crate::analyzer::OwnershipMode::Owned) {
                            // Split condition into tokens and replace standalone param names
                            // Avoid replacing field accesses (e.g. ".name")
                            let tokens: Vec<&str> = condition.split(' ').collect();
                            let mut new_tokens = Vec::new();

                            for (i, token) in tokens.iter().enumerate() {
                                let prev_ends_with_dot = if i > 0 {
                                    tokens[i - 1].ends_with('.')
                                } else {
                                    false
                                };

                                if *token == param.name && !prev_ends_with_dot {
                                    new_tokens.push(format!("__{}__for_ensures", param.name));
                                } else {
                                    new_tokens.push(token.to_string());
                                }
                            }

                            condition = new_tokens.join(" ");
                        }
                    }

                    output.push_str(&self.indent());
                    output.push_str(&format!(
                        "windjammer_runtime::test::ensures({}, \"{}\");\n",
                        condition, condition
                    ));
                }
            }

            output.push_str(&self.indent());
            output.push_str("__result\n");
        }

        // Close bench wrapper
        if needs_bench {
            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push_str("});\n");
            output.push_str(&self.indent());
            output.push_str("println!(\"Benchmark: {:?}\", _bench_result);\n");
        }

        // Close timeout wrapper
        if needs_timeout {
            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push_str("}).unwrap();\n");
        }

        output
    }
}
