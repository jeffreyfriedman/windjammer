//! Doc comments, visibility, generics, and opening `(` for regular function codegen.

use crate::analyzer::*;
use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Emit doc comment through `fn name<...>(` opening paren for a regular function.
    /// Returns whether explicit `'a` lifetimes should be threaded through parameters and return type.
    pub(in crate::codegen::rust) fn append_regular_function_signature_prefix(
        &mut self,
        analyzed: &AnalyzedFunction<'ast>,
        func: &FunctionDecl<'ast>,
        output: &mut String,
    ) -> bool {
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
        let decorator_reg2 = crate::decorator_registry::DecoratorRegistry::new();
        for decorator in &func.decorators {
            if decorator_reg2.should_skip_for_backend(&decorator.name, self.target) {
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

        needs_lifetime
    }
}
