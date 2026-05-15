//! Parameterized `@test_cases` codegen.

use crate::analyzer::*;
use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
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
    pub(super) fn generate_parameterized_tests(
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
    pub(super) fn generate_test_case_argument(
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

        let needs_to_string = if let Expression::Literal {
            value: Literal::String(_),
            ..
        } = arg_expr
        {
            if let Some(param) = param {
                let is_string_type = matches!(param.type_, Type::String)
                    || matches!(param.type_, Type::Custom(ref name) if name == "string");

                if is_string_type {
                    // Phase 2: if the param was optimized to &str, string literals are already
                    // &str — no .to_string() needed. Only add for Owned or &String params.
                    !analyzed.str_ref_optimizable_params.contains(&param.name)
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
    pub(super) fn generate_function_impl(&mut self, analyzed: &AnalyzedFunction<'ast>) -> String {
        // Just call the regular generate_function since we've already removed the decorators
        self.generate_function(analyzed)
    }
}
