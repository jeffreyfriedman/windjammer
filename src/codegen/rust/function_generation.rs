//! Function Generation Module
//!
//! Handles generation of Rust code for function declarations, including:
//! - Regular functions and methods
//! - Extern/FFI function declarations
//! - Functions with decorator wrapping (timeout, bench, requires, ensures, etc.)
//! - Parameterized tests (@test_cases)
//! - Self parameter inference and builder pattern detection

use crate::analyzer::*;
use crate::codegen::rust::{ast_utilities, codegen_helpers, self_analysis, type_analysis};
use crate::parser::*;
use crate::CompilationTarget;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    fn function_returns_self_type(&self, func: &FunctionDecl) -> bool {
        // Check if the function returns Self (for builder pattern detection)
        use crate::parser::{Expression, Statement, Type};

        // First check if return type is a custom type (struct type)
        let returns_custom_type = matches!(&func.return_type, Some(Type::Custom(_)));

        if !returns_custom_type {
            return false;
        }

        // Now check if the function body actually returns `self`
        // Check the last statement in the body
        if let Some(last_stmt) = func.body.last() {
            match last_stmt {
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    // Explicit return self
                    matches!(expr, Expression::Identifier { name, .. } if name == "self")
                }
                Statement::Expression { expr, .. } => {
                    // Implicit return self (last expression)
                    matches!(expr, Expression::Identifier { name, .. } if name == "self")
                }
                _ => false,
            }
        } else {
            false
        }
    }

    fn function_modifies_self(&self, func: &FunctionDecl) -> bool {
        // Check if the function body modifies self (specifically for self parameters)
        for stmt in &func.body {
            if self.statement_modifies_self(stmt) {
                return true;
            }
        }
        false
    }

    fn statement_modifies_self(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Assignment { target, .. } => {
                // Check if target is self.field
                self.expression_is_self_field_modification(target)
            }
            Statement::Expression { expr, .. } => {
                // Check for mutating method calls like self.field.push()
                self.expression_modifies_self(expr)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block.iter().any(|s| self.statement_modifies_self(s))
                    || else_block
                        .as_ref()
                        .is_some_and(|block| block.iter().any(|s| self.statement_modifies_self(s)))
            }
            Statement::While { body, .. } | Statement::For { body, .. } => {
                body.iter().any(|s| self.statement_modifies_self(s))
            }
            Statement::Match { arms, .. } => arms.iter().any(|arm| {
                // Match arms have a body expression, check if it contains modifications
                self.expression_modifies_self(arm.body)
            }),
            _ => false,
        }
    }

    fn expression_is_self_field_modification(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == "self")
            }
            _ => false,
        }
    }

    fn expression_modifies_self(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Block { statements, .. } => {
                statements.iter().any(|s| self.statement_modifies_self(s))
            }
            Expression::MethodCall { object, method, .. } => {
                // Check if this is a mutating method call on self.field
                // Common mutating methods: push, pop, remove, insert, clear, etc.
                let is_mutating_method = matches!(
                    method.as_str(),
                    "push"
                        | "pop"
                        | "remove"
                        | "insert"
                        | "clear"
                        | "append"
                        | "extend"
                        | "drain"
                        | "truncate"
                        | "resize"
                        | "swap_remove"
                        | "retain"
                );

                if is_mutating_method {
                    // Check if the object is self.field
                    if let Expression::FieldAccess {
                        object: field_obj, ..
                    } = &**object
                    {
                        if matches!(&**field_obj, Expression::Identifier { name, .. } if name == "self")
                        {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Generate extern "C" function declaration for FFI
    pub(super) fn generate_extern_function(&self, func: &FunctionDecl) -> String {
        let mut output = String::new();

        output.push_str("    pub fn ");
        output.push_str(&func.name);

        // Add type parameters if present
        if !func.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&func.type_params));
            output.push('>');
        }

        output.push('(');

        // Generate parameters - use FFI-safe types for extern "C"
        // string/String -> windjammer_runtime::ffi::FfiString (Rust String/&str are not C-compatible)
        let mut params: Vec<String> = Vec::new();
        for param in &func.parameters {
            if matches!(&param.type_, Type::Custom(name) if name == "str") {
                params.push(format!("{}_ptr: *const u8", param.name));
                params.push(format!("{}_len: usize", param.name));
            } else if matches!(&param.type_, Type::String)
                || matches!(&param.type_, Type::Custom(name) if name == "string" || name == "String")
            {
                params.push(format!("{}: windjammer_runtime::ffi::FfiString", param.name));
            } else {
                params.push(format!(
                    "{}: {}",
                    param.name,
                    self.type_to_rust(&param.type_)
                ));
            }
        }

        output.push_str(&params.join(", "));
        output.push(')');

        // Add return type - use FfiString for string returns
        if let Some(ret_type) = &func.return_type {
            output.push_str(" -> ");
            let rust_ret = if matches!(ret_type, Type::String)
                || matches!(ret_type, Type::Custom(name) if name == "string" || name == "String")
            {
                "windjammer_runtime::ffi::FfiString".to_string()
            } else {
                self.type_to_rust(ret_type)
            };
            output.push_str(&rust_ret);
        }

        output.push_str(";\n");
        output
    }

    /// Check if function has decorators that need to wrap the function body
    fn has_wrapping_decorator(&self, func: &FunctionDecl<'ast>) -> bool {
        func.decorators.iter().any(|d| {
            matches!(
                d.name.as_str(),
                "timeout" | "bench" | "requires" | "ensures" | "property_test" | "invariant"
            ) || (d.name == "test" && !d.arguments.is_empty())
        })
    }

    /// Generate function with decorator wrapping (timeout, bench, requires, ensures, etc.)
    fn generate_function_with_wrapping(&mut self, analyzed: &AnalyzedFunction<'ast>) -> String {
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
        for decorator in &func.decorators {
            if decorator.name == "async" {
                continue;
            }
            if decorator.name == "export" && self.target != CompilationTarget::Wasm {
                continue;
            }
            // TDD FIX: Skip WGSL-specific decorators when targeting Rust
            // WGSL decorators (@vertex, @fragment, @compute) are GPU-only and invalid in Rust
            if matches!(
                decorator.name.as_str(),
                "vertex" | "fragment" | "compute"
            ) {
                continue;
            }
            // Skip wrapping decorators - they'll be handled in the body
            if matches!(
                decorator.name.as_str(),
                "timeout" | "bench" | "requires" | "ensures" | "property_test" | "invariant"
            ) {
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
                    // Borrowed string params use &str (idiomatic Rust), not &String
                    match ownership {
                        crate::analyzer::OwnershipMode::Borrowed => {
                            let ref_type = if rust_type == "String" {
                                "&str".to_string()
                            } else {
                                format!("&{}", rust_type)
                            };
                            format!("{}: {}", param.name, ref_type)
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
    fn generate_wrapped_function_body(&mut self, analyzed: &AnalyzedFunction<'ast>) -> String {
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

    /// Generate multiple test functions from a parameterized test (@test_cases)
    ///
    /// Example Windjammer:
    /// ```text
    /// @test_cases([
    ///     (5, 3, 8),
    ///     (10, -5, 5),
    ///     (0, 0, 0),
    /// ])
    /// fn add_numbers(a: int, b: int, expected: int) {
    ///     assert_eq(a + b, expected);
    /// }
    /// ```
    ///
    /// Generates:
    /// ```text
    /// fn add_numbers_case_0() { add_numbers_impl(5, 3, 8); }
    /// fn add_numbers_case_1() { add_numbers_impl(10, -5, 5); }
    /// fn add_numbers_case_2() { add_numbers_impl(0, 0, 0); }
    /// fn add_numbers_impl(a: i64, b: i64, expected: i64) {
    ///     assert_eq!(a + b, expected);
    /// }
    /// ```
    fn generate_parameterized_tests(
        &mut self,
        analyzed: &AnalyzedFunction<'ast>,
        test_cases_decorator: &Decorator<'ast>,
    ) -> String {
        use crate::parser::Expression;

        let func = &analyzed.decl;
        let mut output = String::new();

        // Extract test cases from decorator arguments
        // Expected format: @test_cases([(val1, val2, ...), (val1, val2, ...), ...])
        let test_cases = if let Some((_, cases_expr)) = test_cases_decorator.arguments.first() {
            // Parse the array literal
            if let Expression::Array { elements, .. } = cases_expr {
                elements.clone()
            } else {
                // Not an array, try to extract it directly
                vec![*cases_expr]
            }
        } else {
            // No arguments provided, skip parameterized test generation
            return "// ERROR: @test_cases decorator requires arguments\n".to_string();
        };

        if test_cases.is_empty() {
            return "// ERROR: @test_cases decorator requires at least one test case\n".to_string();
        }

        // Generate the implementation function (with _impl suffix)
        let impl_func_name = format!("{}_impl", func.name);

        // Create a modified function declaration for the implementation
        let mut impl_func_decl = func.clone();
        impl_func_decl.name = impl_func_name.clone();
        // Remove the @test_cases decorator from the impl function
        impl_func_decl
            .decorators
            .retain(|d| d.name != "test_cases" && d.name != "test");

        // Create a modified AnalyzedFunction for the implementation
        let mut impl_analyzed = analyzed.clone();
        impl_analyzed.decl = impl_func_decl;

        // Generate the implementation function (non-test, just regular function)
        output.push_str(&self.generate_function_impl(&impl_analyzed));
        output.push_str("\n\n");

        // Generate a test function for each test case
        for (case_idx, case_expr) in test_cases.iter().enumerate() {
            output.push_str("#[test]\n");
            output.push_str(&format!("fn {}_case_{}() {{\n", func.name, case_idx));

            // Generate the call to the implementation function with the test case arguments
            output.push_str("    ");
            output.push_str(&impl_func_name);
            output.push('(');

            // Extract arguments from the tuple or array expression
            // THE WINDJAMMER WAY: Support both (val1, val2) and [val1, val2] syntax
            if let Expression::Tuple { elements, .. } = case_expr {
                let args: Vec<String> = elements
                    .iter()
                    .enumerate()
                    .map(|(idx, arg)| self.generate_test_case_argument(arg, analyzed, idx))
                    .collect();
                output.push_str(&args.join(", "));
            } else if let Expression::Array { elements, .. } = case_expr {
                // Also support array syntax: ["val1", "val2", "val3"]
                let args: Vec<String> = elements
                    .iter()
                    .enumerate()
                    .map(|(idx, arg)| self.generate_test_case_argument(arg, analyzed, idx))
                    .collect();
                output.push_str(&args.join(", "));
            } else {
                // Single argument (not a tuple or array)
                output.push_str(&self.generate_test_case_argument(case_expr, analyzed, 0));
            }

            output.push_str(");\n");
            output.push_str("}\n\n");
        }

        output
    }

    /// Generate a test case argument with auto-conversion for string literals
    /// THE WINDJAMMER WAY: Compiler does the hard work, not the developer
    fn generate_test_case_argument(
        &mut self,
        arg_expr: &Expression<'ast>,
        analyzed: &AnalyzedFunction<'ast>,
        param_idx: usize,
    ) -> String {
        use crate::parser::ast::core::Expression;
        use crate::parser::ast::literals::Literal;
        use crate::parser::ast::types::Type;

        let params = &analyzed.decl.parameters;
        let param = params.get(param_idx);

        // Check if this is a string literal and the parameter expects OWNED String (not &str)
        let needs_to_string = if let Expression::Literal {
            value: Literal::String(_),
            ..
        } = arg_expr
        {
            if let Some(param) = param {
                // Check if parameter type is string
                let is_string_type = matches!(param.type_, Type::String)
                    || matches!(param.type_, Type::Custom(ref name) if name == "string");

                if is_string_type {
                    // Check if this parameter was inferred as OWNED (not borrowed)
                    // If inferred as borrowed → generates &str → string literal passes directly
                    // If inferred as owned → generates String → string literal needs .to_string()
                    let inferred_ownership = analyzed.inferred_ownership.get(&param.name);
                    !matches!(inferred_ownership, Some(OwnershipMode::Borrowed))
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        // Generate the expression
        let mut result = self.generate_expression_immut(arg_expr);

        // Add .to_string() if needed (only for owned String params)
        if needs_to_string {
            result.push_str(".to_string()");
        }

        result
    }

    /// Generate a function without test decorators (used by parameterized tests)
    fn generate_function_impl(&mut self, analyzed: &AnalyzedFunction<'ast>) -> String {
        // Just call the regular generate_function since we've already removed the decorators
        self.generate_function(analyzed)
    }

    pub(crate) fn generate_function(&mut self, analyzed: &AnalyzedFunction<'ast>) -> String {
        let func = &analyzed.decl;

        // PARAMETERIZED TESTS: Check for @test_cases decorator
        // If present, generate multiple test functions instead of one
        if let Some(test_cases_decorator) = func.decorators.iter().find(|d| d.name == "test_cases")
        {
            return self.generate_parameterized_tests(analyzed, test_cases_decorator);
        }

        // TESTING DECORATORS: Check for decorators that need to wrap the function body
        // These include: @timeout, @bench, @requires, @ensures, @property_test, @test(setup/teardown)
        if self.has_wrapping_decorator(func) {
            return self.generate_function_with_wrapping(analyzed);
        }

        let mut output = String::new();

        // TDD FIX: Auto-add #[test] attribute for test functions in test files
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

        // LOCAL VARIABLE TRACKING: Push new scope for this function
        self.local_variable_scopes
            .push(std::collections::HashSet::new());

        // AUTO-CLONE: Load auto-clone analysis for this function
        self.auto_clone_analysis = Some(analyzed.auto_clone_analysis.clone());

        // PHASE 2 OPTIMIZATION: Load clone optimizations for this function
        // Variables in this set can safely avoid .clone() calls
        self.clone_optimizations.clear();
        for opt in &analyzed.clone_optimizations {
            self.clone_optimizations.insert(opt.variable.clone());
        }

        // Track function parameters for compound assignment optimization
        self.current_function_params = func.parameters.clone();

        // Clear local variable types for new function scope
        self.local_var_types.clear();

        // Track function return type for string literal conversion
        self.current_function_return_type = func.return_type.clone();

        // Track method return types for usize inference in comparisons
        // When in an impl block, record the return type so expression_produces_usize
        // can resolve method calls like animation.frame_count() → usize
        if self.in_impl_block {
            if let Some(ref ret_type) = func.return_type {
                self.method_return_types
                    .insert(func.name.to_string(), ret_type.clone());
            }
        }

        // Track function body for data flow analysis
        self.current_function_body = func.body.clone();

        // FOR-LOOP AUTO-BORROW: Pre-scan function body to find local variables
        // that are iterated in for-loops and also used after the loop.
        // These need `&` auto-inserted to prevent consuming the collection.
        self.precompute_for_loop_borrows(&func.body);

        // Track parameters inferred as borrowed/mut-borrowed for codegen decisions
        self.inferred_borrowed_params.clear();
        self.inferred_mut_borrowed_params.clear();
        for (param_name, ownership) in &analyzed.inferred_ownership {
            match ownership {
                crate::analyzer::OwnershipMode::Borrowed => {
                    self.inferred_borrowed_params.insert(param_name.clone());
                }
                crate::analyzer::OwnershipMode::MutBorrowed => {
                    self.inferred_mut_borrowed_params.insert(param_name.clone());
                }
                _ => {}
            }
        }

        // WINDJAMMER FIX: Track usize-typed parameters for auto-cast logic
        // DON'T clear here - we need to accumulate variables from let statements during generation!
        // Only clear at the very beginning of function generation, before body processing.
        // TDD FIX (Bug #3): Moved clear to happen BEFORE pre-passes, so marking during
        // statement generation can accumulate variables.

        // Clear ONCE at function start (before any analysis)
        self.usize_variables.clear();

        // When a parameter is declared as `usize`, add it to usize_variables
        // so expression_produces_usize() correctly identifies it
        for (param_idx, param) in func.parameters.iter().enumerate() {
            // Use inferred type if available, otherwise use declared type
            let param_type = analyzed
                .inferred_param_types
                .get(param_idx)
                .unwrap_or(&param.type_);

            // Check if this parameter is usize
            if matches!(param_type, Type::Custom(name) if name == "usize") {
                self.usize_variables.insert(param.name.clone());
            }
        }

        // PHASE 8 OPTIMIZATION: Load SmallVec optimizations for this function
        // DISABLED: SmallVec optimizations conflict with return types
        // TODO: Re-enable with smarter conversion at return sites
        self.smallvec_optimizations.clear();
        // for opt in &analyzed.smallvec_optimizations {
        //     self.smallvec_optimizations
        //         .insert(opt.variable.clone(), opt.clone());
        //     self.needs_smallvec_import = true; // Mark that we need the smallvec crate
        // }

        // PHASE 9 OPTIMIZATION: Load Cow optimizations for this function
        self.cow_optimizations.clear();
        for opt in &analyzed.cow_optimizations {
            self.cow_optimizations.insert(opt.variable.clone());
            self.needs_cow_import = true; // Mark that we need Cow from std::borrow
        }

        // PHASE 3 OPTIMIZATION: Load struct mapping optimizations
        // Track which structs can use optimized construction strategies
        self.struct_mapping_hints.clear();
        for opt in &analyzed.struct_mapping_optimizations {
            self.struct_mapping_hints
                .insert(opt.target_struct.clone(), opt.strategy.clone());
        }

        // PHASE 4 OPTIMIZATION: Load string operation optimizations
        // Track capacity hints for string operations
        self.string_capacity_hints.clear();

        // PHASE 5 OPTIMIZATION: Load assignment operation optimizations
        // Track which variables can use compound assignment operators
        self.assignment_optimizations.clear();
        for opt in &analyzed.assignment_optimizations {
            self.assignment_optimizations
                .insert(opt.variable.clone(), opt.operation.clone());
        }
        for opt in &analyzed.string_optimizations {
            if let Some(capacity) = opt.estimated_capacity {
                self.string_capacity_hints.insert(opt.location, capacity);
            }
        }

        // PHASE 6 OPTIMIZATION: Load defer drop optimizations
        // Track variables that should have their drops deferred to background thread
        self.defer_drop_optimizations = analyzed.defer_drop_optimizations.clone();

        // Generate doc comment if present
        if let Some(doc_comment) = &func.doc_comment {
            for line in doc_comment.lines() {
                output.push_str(&format!("/// {}\n", line.trim()));
            }
        }

        // Check for @async decorator (special case: it's a keyword, not an attribute)
        let is_async = func.decorators.iter().any(|d| d.name == "async");

        // Special case: async main requires #[tokio::main]
        if is_async && func.name == "main" {
            output.push_str("#[tokio::main]\n");
        }

        // OPTIMIZATION: Add inline hints for hot path functions
        // This is Phase 1 optimization: Generate Inlinable Code
        if self.should_inline_function(func, analyzed) {
            output.push_str("#[inline]\n");
        }

        // Generate decorators (map Windjammer decorators to Rust attributes)
        for decorator in &func.decorators {
            // Skip @async, it's handled specially
            if decorator.name == "async" {
                continue;
            }

            // Skip @export - it's used to determine visibility but doesn't map to a Rust attribute for native targets
            if decorator.name == "export" && self.target != CompilationTarget::Wasm {
                continue;
            }

            // TDD FIX: Skip WGSL-specific decorators when targeting Rust
            // WGSL decorators (@vertex, @fragment, @compute) are GPU-only and invalid in Rust
            if matches!(
                decorator.name.as_str(),
                "vertex" | "fragment" | "compute"
            ) {
                continue;
            }

            // Skip game framework decorators - they're handled by the game loop
            if matches!(
                decorator.name.as_str(),
                "game" | "init" | "update" | "render" | "render3d" | "input" | "cleanup"
            ) {
                continue;
            }

            // Map Windjammer decorator to Rust attribute (same as struct decorator handling)
            let rust_attr = self.map_decorator(&decorator.name);
            if decorator.arguments.is_empty() {
                output.push_str(&format!("#[{}]\n", rust_attr));
            } else {
                output.push_str(&format!("#[{}(", rust_attr));
                let args: Vec<String> = decorator
                    .arguments
                    .iter()
                    .map(|(key, expr)| {
                        format!("{} = {}", key, self.generate_expression_immut(expr))
                    })
                    .collect();
                output.push_str(&args.join(", "));
                output.push_str(")]\n");
            }
        }

        // Add `pub` if function is marked pub OR we're in a #[wasm_bindgen] impl block OR compiling a module OR has @export decorator
        // BUT NOT if we're in a trait implementation (trait methods cannot have visibility modifiers)
        let has_export = func.decorators.iter().any(|d| d.name == "export");
        if !self.in_trait_impl
            && (func.is_pub || self.in_wasm_bindgen_impl || self.is_module || has_export)
        {
            output.push_str("pub ");
        }

        // Add async keyword if decorator present
        if is_async {
            output.push_str("async ");
        }

        output.push_str("fn ");
        output.push_str(&func.name);

        // WINDJAMMER LIFETIME INFERENCE: Determine if explicit lifetime annotations are needed.
        // Rust's lifetime elision rules handle most cases automatically:
        //   1. Single input reference → output gets that lifetime
        //   2. &self/&mut self → output gets self's lifetime
        //   3. Multiple input references with no self → MUST be explicit
        // We only add 'a when case 3 applies AND the return type contains references.
        let needs_lifetime = self.function_needs_lifetime_annotations(func, analyzed);

        // Add type parameters with bounds: fn foo<T: Display, U: Debug>(...)
        // Merge inferred bounds with explicit bounds
        let type_params = if let Some(inferred) = self.inferred_bounds.get(&func.name) {
            let merged = inferred.merge_with_explicit(&func.type_params);
            // Track which traits need imports
            for param in &merged {
                for trait_name in &param.bounds {
                    self.needs_trait_imports.insert(trait_name.clone());
                }
            }
            merged
        } else {
            // Still track explicit bounds
            for param in &func.type_params {
                for trait_name in &param.bounds {
                    self.needs_trait_imports.insert(trait_name.clone());
                }
            }
            func.type_params.clone()
        };

        if needs_lifetime || !type_params.is_empty() {
            output.push('<');
            let mut parts = Vec::new();
            if needs_lifetime {
                parts.push("'a".to_string());
            }
            if !type_params.is_empty() {
                parts.push(self.format_type_params(&type_params));
            }
            output.push_str(&parts.join(", "));
            output.push('>');
        }

        output.push('(');

        // Add implicit &self or &mut self for impl block methods that access fields
        // THE WINDJAMMER WAY: Constructors (associated functions) should NOT get self added!
        let mut params: Vec<String> = Vec::new();
        let has_explicit_self = func.parameters.iter().any(|p| p.name == "self");

        // THE WINDJAMMER WAY: Auto-Self Inference
        // Check if analyzer inferred a self parameter (even if not in AST)
        let has_inferred_self = analyzed.inferred_ownership.contains_key("self");

        // Check if this is a constructor (associated function returning the struct type)
        // A constructor returns the struct being implemented, e.g., fn new() -> Tilemap
        let is_constructor = !has_explicit_self && !has_inferred_self && {
            if let Some(Type::Custom(return_type_name)) = &func.return_type {
                // Check if return type matches current struct name
                self.current_struct_name
                    .as_ref()
                    .is_some_and(|struct_name| struct_name == return_type_name)
            } else {
                false
            }
        };

        // Priority 1: Use analyzer's inferred self if available
        if has_inferred_self && !has_explicit_self {
            if let Some(ownership) = analyzed.inferred_ownership.get("self") {
                let self_param = match ownership {
                    OwnershipMode::Borrowed => "&self",
                    OwnershipMode::MutBorrowed => "&mut self",
                    OwnershipMode::Owned => {
                        // Check if function modifies self (builder pattern)
                        if self.function_modifies_self(&analyzed.decl) {
                            "mut self"
                        } else {
                            "self"
                        }
                    }
                };
                params.push(self_param.to_string());
            }
        }
        // Priority 2: Fallback to old field-based analysis (for backwards compatibility)
        else if self.in_impl_block
            && !has_explicit_self
            && !self.current_struct_fields.is_empty()
            && !is_constructor
        {
            // Check if function body mutates any struct fields
            let ctx =
                self_analysis::AnalysisContext::new(&func.parameters, &self.current_struct_fields);
            if self_analysis::function_mutates_fields(&ctx, func) {
                // Check if this is a builder pattern (modifies fields AND returns Self)
                let returns_self = self.function_returns_self_type(func);
                if returns_self {
                    // Builder pattern: use `mut self` (consuming)
                    params.push("mut self".to_string());
                } else {
                    // Regular mutating method: use `&mut self` (borrowing)
                    params.push("&mut self".to_string());
                }
            } else if self_analysis::function_accesses_fields(&ctx, func) {
                // Only read access needed
                params.push("&self".to_string());
            }
        }

        // TDD FIX: Pre-compute which parameters are actually used in the function body.
        // Unused parameters get prefixed with `_` to suppress "unused variable" warnings.
        // THE WINDJAMMER WAY: The compiler handles this automatically — developers don't
        // need to manually prefix unused parameters with `_`.
        let body_refs: Vec<&Statement> = func.body.to_vec();
        let unused_params: std::collections::HashSet<String> = func
            .parameters
            .iter()
            .filter(|p| p.name != "self")
            .filter(|p| !Self::variable_used_in_statements(&body_refs, &p.name))
            .map(|p| p.name.clone())
            .collect();

        // TDD FIX: Pre-compute unused let bindings and for-loop variables.
        // Like unused params, these get prefixed with `_` in the generated Rust.
        self.unused_let_bindings.clear();
        Self::find_unused_bindings(&func.body, &mut self.unused_let_bindings);

        let additional_params: Vec<String> = func
            .parameters
            .iter()
            .enumerate()
            .map(|(param_idx, param)| {
                // SMART STRING INFERENCE: Use the inferred type from analyzer (string → &str vs String)
                let inferred_type = analyzed
                    .inferred_param_types
                    .get(param_idx)
                    .unwrap_or(&param.type_);

                // PHASE 9 OPTIMIZATION: Check if this parameter should use Cow<'_, T>
                if self.cow_optimizations.contains(&param.name) {
                    let base_type = self.type_to_rust(inferred_type);
                    // For String types, use Cow<'_, str>
                    let cow_type = if base_type == "String" {
                        "Cow<'_, str>".to_string()
                    } else {
                        format!("Cow<'_, {}>", base_type)
                    };
                    return format!("{}: {}", param.name, cow_type);
                }

                // Handle explicit ownership hints (self, &self, &mut self)
                let type_str = match &param.ownership {
                    OwnershipHint::Owned => {
                        if param.name == "self" {
                            // Check if analyzer inferred a different ownership for self
                            if let Some(ownership_mode) =
                                analyzed.inferred_ownership.get(&param.name)
                            {
                                match ownership_mode {
                                    OwnershipMode::MutBorrowed => return "&mut self".to_string(),
                                    OwnershipMode::Borrowed => return "&self".to_string(),
                                    OwnershipMode::Owned => {
                                        // Check if function actually modifies self
                                        // Only add 'mut' if it does
                                        if self.function_modifies_self(&analyzed.decl) {
                                            return "mut self".to_string();
                                        } else {
                                            return "self".to_string();
                                        }
                                    }
                                }
                            }
                            // Default: check if function modifies self
                            if self.function_modifies_self(&analyzed.decl) {
                                return "mut self".to_string();
                            } else {
                                return "self".to_string();
                            }
                        }
                        // Owned parameters are always mutable in Windjammer
                        return format!("mut {}: {}", param.name, self.type_to_rust(inferred_type));
                    }
                    OwnershipHint::Ref => {
                        if param.name == "self" {
                            // Check if analyzer inferred a different ownership (e.g., &mut self)
                            if let Some(ownership_mode) =
                                analyzed.inferred_ownership.get(&param.name)
                            {
                                match ownership_mode {
                                    OwnershipMode::MutBorrowed => return "&mut self".to_string(),
                                    OwnershipMode::Borrowed => return "&self".to_string(),
                                    OwnershipMode::Owned => {
                                        // Shouldn't happen for explicit &self, but handle it
                                        return "self".to_string();
                                    }
                                }
                            }
                            return "&self".to_string();
                        }
                        // Don't add & if the type is already a Reference
                        if matches!(
                            inferred_type,
                            Type::Reference(_) | Type::MutableReference(_)
                        ) {
                            self.type_to_rust(inferred_type)
                        } else {
                            // WINDJAMMER DESIGN: Borrowed String → &str (not &String!)
                            let is_string = matches!(inferred_type, Type::String)
                                || matches!(inferred_type, Type::Custom(name) if name == "string");

                            if is_string {
                                "&str".to_string()
                            } else {
                                format!("&{}", self.type_to_rust(inferred_type))
                            }
                        }
                    }
                    OwnershipHint::Mut => {
                        if param.name == "self" {
                            return "&mut self".to_string();
                        }
                        // Don't add &mut if the type is already a MutableReference
                        if matches!(inferred_type, Type::MutableReference(_)) {
                            self.type_to_rust(inferred_type)
                        } else {
                            format!("&mut {}", self.type_to_rust(inferred_type))
                        }
                    }
                    OwnershipHint::Inferred => {
                        // SMART STRING INFERENCE: inferred_type already has &str vs String resolved!
                        // For strings: Type::Reference(String) → &str, Type::String → String
                        // For other types: Apply ownership mode from analyzer

                        // Special handling for `self` parameters (trait impl methods)
                        if param.name == "self" {
                            // Check analyzer for inferred ownership
                            if let Some(ownership_mode) =
                                analyzed.inferred_ownership.get(&param.name)
                            {
                                match ownership_mode {
                                    OwnershipMode::MutBorrowed => return "&mut self".to_string(),
                                    OwnershipMode::Borrowed => return "&self".to_string(),
                                    OwnershipMode::Owned => {
                                        // Check if function actually modifies self
                                        if self.function_modifies_self(&analyzed.decl) {
                                            return "mut self".to_string();
                                        } else {
                                            return "self".to_string();
                                        }
                                    }
                                }
                            }
                            // Default: check if function modifies self
                            if self.function_modifies_self(&analyzed.decl) {
                                return "mut self".to_string();
                            } else {
                                return "self".to_string();
                            }
                        }

                        // Check if type already has ownership baked in (like &str from string inference)
                        if matches!(
                            inferred_type,
                            Type::Reference(_) | Type::MutableReference(_)
                        ) {
                            // Already has & or &mut - just convert
                            self.type_to_rust(inferred_type)
                        } else {
                            // Apply ownership mode from analyzer
                            // TDD FIX: Default to Owned, not Borrowed
                            // THE WINDJAMMER WAY: Parameters are owned by default unless analyzer
                            // detects they should be borrowed (e.g., only read, passed to & functions)
                            let ownership_mode = analyzed
                                .inferred_ownership
                                .get(&param.name)
                                .unwrap_or(&OwnershipMode::Owned);

                            match ownership_mode {
                                OwnershipMode::Owned => self.type_to_rust(inferred_type),
                                OwnershipMode::Borrowed => {
                                    if type_analysis::is_copy_type(inferred_type) {
                                        // Copy types pass by value even when borrowed
                                        self.type_to_rust(inferred_type)
                                    } else {
                                        // WINDJAMMER DESIGN: Borrowed String → &str (not &String!)
                                        // Check if this is a String type (either Type::String or Type::Custom("string"))
                                        let is_string = matches!(inferred_type, Type::String)
                                            || matches!(inferred_type, Type::Custom(name) if name == "string");

                                        if is_string {
                                            // &str is idiomatic Rust: accepts both String and &str via deref coercion
                                            // &String is an anti-pattern (Clippy warning)
                                            "&str".to_string()
                                        } else {
                                            format!("&{}", self.type_to_rust(inferred_type))
                                        }
                                    }
                                }
                                OwnershipMode::MutBorrowed => {
                                    format!("&mut {}", self.type_to_rust(inferred_type))
                                }
                            }
                        }
                    }
                };

                // WINDJAMMER LIFETIME INFERENCE: Add 'a lifetime to reference parameters
                // when the function needs explicit lifetime annotations.
                let type_str = if needs_lifetime && param.name != "self" {
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

                // TDD FIX: Auto-infer `mut` for owned parameters
                // THE WINDJAMMER WAY: Users don't track mutability - the compiler does.
                // If a parameter has mutating method calls or field mutations,
                // the binding needs `mut` even if not explicitly written.
                let auto_needs_mut = param.name != "self"
                    && !param.is_mutable
                    && matches!(type_str.as_str(), s if !s.starts_with("&"))
                    && self.variable_needs_mut(&param.name);
                let mut_prefix = if param.is_mutable || auto_needs_mut {
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
            .collect();

        params.extend(additional_params);

        output.push_str(&params.join(", "));
        output.push(')');

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

        // Add where clause if present
        output.push_str(&codegen_helpers::format_where_clause(&func.where_clause));

        output.push_str(" {\n");
        self.indent_level += 1;

        // TDD: Generate function body with return optimization
        // Set flag to enable implicit return for last statement
        let old_in_function_body = self.in_function_body;
        self.in_function_body = true;
        let mut body_code = self.generate_block(&func.body);
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

        // LOCAL VARIABLE TRACKING: Pop scope when exiting function
        self.local_variable_scopes.pop();

        output
    }

    /// WINDJAMMER LIFETIME INFERENCE: Determine if a function needs explicit lifetime annotations.
    ///
    /// Rust's lifetime elision rules handle most cases:
    ///   1. Single input reference → output gets that lifetime
    ///   2. &self/&mut self → output gets self's lifetime
    ///   3. Multiple input references with no self → MUST be explicit
    ///
    /// We only add 'a when case 3 applies AND the return type contains references.
    fn function_needs_lifetime_annotations(
        &self,
        func: &FunctionDecl<'ast>,
        analyzed: &AnalyzedFunction<'ast>,
    ) -> bool {
        use crate::codegen::rust::types::type_contains_reference;

        // First check: does the return type contain any references?
        let return_has_ref = match &func.return_type {
            Some(ret_type) => type_contains_reference(ret_type),
            None => false,
        };

        if !return_has_ref {
            return false;
        }

        // Check if there's a self parameter (explicit or inferred)
        let has_self = func.parameters.iter().any(|p| p.name == "self")
            || analyzed.inferred_ownership.contains_key("self");

        if has_self {
            // &self/&mut self methods: Rust elision rule 2 handles this
            return false;
        }

        // Count the number of reference parameters (explicit refs + analyzer-inferred refs)
        let ref_param_count = func
            .parameters
            .iter()
            .enumerate()
            .filter(|(param_idx, param)| {
                if param.name == "self" {
                    return false;
                }

                // Check if the parameter type is already a reference
                let inferred_type = analyzed
                    .inferred_param_types
                    .get(*param_idx)
                    .unwrap_or(&param.type_);

                if matches!(
                    inferred_type,
                    Type::Reference(_) | Type::MutableReference(_)
                ) {
                    return true;
                }

                // Check explicit ownership hints
                if matches!(
                    param.ownership,
                    crate::parser::OwnershipHint::Ref | crate::parser::OwnershipHint::Mut
                ) {
                    return true;
                }

                // Check analyzer-inferred ownership
                if let Some(ownership) = analyzed.inferred_ownership.get(&param.name) {
                    matches!(
                        ownership,
                        crate::analyzer::OwnershipMode::Borrowed
                            | crate::analyzer::OwnershipMode::MutBorrowed
                    )
                } else {
                    false
                }
            })
            .count();

        // Need explicit lifetime when 2+ reference params and reference return
        ref_param_count >= 2
    }

    /// OPTIMIZATION: Determine if a function should be marked #[inline]
    /// Phase 1: Generate Inlinable Code
    ///
    /// Heuristics for inlining:
    /// 1. Module functions (stdlib wrappers) - always inline for zero-cost abstraction
    /// 2. Small functions (< 10 statements) - likely to benefit from inlining
    /// 3. Trivial getters/setters - always inline
    /// 4. Functions with only one return statement - simple enough to inline
    /// 5. Don't inline: main(), test functions, async functions, large functions
    fn should_inline_function(&self, func: &FunctionDecl, _analyzed: &AnalyzedFunction) -> bool {
        // Never inline main
        if func.name == "main" {
            return false;
        }

        // Never inline test functions
        if func.decorators.iter().any(|d| d.name == "test") {
            return false;
        }

        // Don't inline async functions (they're already state machines)
        if func.decorators.iter().any(|d| d.name == "async") {
            return false;
        }

        // ALWAYS inline module functions (stdlib wrappers)
        // These are thin wrappers around Rust stdlib and should have zero overhead
        if self.is_module {
            return true;
        }

        // Count statements in function body
        let statement_count = ast_utilities::count_statements(&func.body);

        // Inline small functions (< 10 statements)
        if statement_count < 10 {
            return true;
        }

        // Inline trivial single-expression functions
        if statement_count == 1 {
            if let Statement::Return { value: Some(_), .. } = &func.body[0] {
                return true;
            }
            if let Statement::Expression { .. } = &func.body[0] {
                return true;
            }
        }

        // Default: don't inline large functions
        false
    }
}
