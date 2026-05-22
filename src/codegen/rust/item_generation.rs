//! Item Generation Module
//!
//! Handles generation of top-level items: structs, enums, traits, and impl blocks.
//! This includes both type declarations and their implementations.

use crate::analyzer::*;
use crate::parser::OwnershipHint;
use crate::parser::*;

use super::codegen_helpers;
use super::self_analysis;
use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    fn collect_derive_trait_identifiers(expr: &Expression<'_>, out: &mut Vec<String>) {
        match expr {
            Expression::Identifier { name, .. } => out.push(name.clone()),
            Expression::Tuple { elements, .. } => {
                for e in elements {
                    Self::collect_derive_trait_identifiers(e, out);
                }
            }
            _ => {}
        }
    }

    /// E0053 / E0599: pick the `AnalyzedFunction` for this exact `impl` method AST node.
    ///
    /// `impl Trait for T` and `impl T` can both define `set_lighting` (etc.). Matching only
    /// `name` + `parent_type` + `impl_trait` can still bind the wrong analysis when trait
    /// metadata is missing cross-file; parameter names + arity are stable identifiers for the
    /// syntactic method being codegen'd.
    fn analyzed_matches_impl_ast(
        af: &AnalyzedFunction<'ast>,
        func: &FunctionDecl<'ast>,
        impl_trait: &Option<String>,
    ) -> bool {
        if af.decl.name != func.name {
            return false;
        }
        if af.decl.parent_type != func.parent_type {
            return false;
        }
        if af.decl.impl_trait != *impl_trait {
            return false;
        }
        if af.decl.parameters.len() != func.parameters.len() {
            return false;
        }
        af.decl
            .parameters
            .iter()
            .zip(func.parameters.iter())
            .all(|(a, b)| a.name == b.name)
    }

    pub(super) fn generate_struct(&mut self, s: &StructDecl) -> String {
        let mut output = String::new();

        // Track which fields have type usize (for auto-casting in comparisons)
        let mut usize_fields = std::collections::HashSet::new();
        for field in &s.fields {
            if matches!(field.field_type, Type::Custom(ref name) if name == "usize") {
                usize_fields.insert(field.name.clone());
            }
        }
        self.usize_struct_fields
            .insert(s.name.clone(), usize_fields);

        // STRUCT FIELD TYPE TRACKING: Record all field types for type inference
        let mut field_types = std::collections::HashMap::new();
        for field in &s.fields {
            field_types.insert(field.name.clone(), field.field_type.clone());
        }
        self.struct_field_types.insert(s.name.clone(), field_types);

        // Convert decorators to Rust attributes
        let decorator_reg = crate::decorator_registry::DecoratorRegistry::new();
        for decorator in &s.decorators {
            if decorator_reg.should_skip_for_backend(&decorator.name, self.target) {
                continue;
            }

            if decorator.name == "command" {
                // Special handling for @command decorator - generates clap attributes
                // @command(name: "app", about: "Description") -> #[derive(Parser)] + #[command(...)]
                output.push_str("#[derive(Parser)]\n");

                if !decorator.arguments.is_empty() {
                    output.push_str("#[command(");
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
                continue;
            } else if decorator.name == "auto" {
                // Special handling for @auto decorator
                let traits = if decorator.arguments.is_empty() {
                    // Smart inference: no arguments, so infer traits based on field types
                    self.infer_derivable_traits(s)
                } else {
                    // Explicit trait list still merges with inference so @auto(Clone) keeps Debug, etc.
                    let mut explicit_traits = Vec::new();
                    for (_key, expr) in &decorator.arguments {
                        if let Expression::Identifier {
                            name: trait_name, ..
                        } = expr
                        {
                            explicit_traits.push(trait_name.clone());
                        }
                    }
                    CodeGenerator::merge_standard_derive_traits(
                        explicit_traits,
                        self.infer_derivable_traits(s),
                    )
                };

                if !traits.is_empty() {
                    output.push_str(&format!("#[derive({})]\n", traits.join(", ")));

                    // Track if this struct has PartialEq for enum derive inference
                    if traits.iter().any(|t| t == "PartialEq") {
                        // Note: partial_eq_types is already populated in pre-pass, no need to insert here
                    }
                }
            } else if decorator.name == "derive" {
                // Special handling for @derive decorator - generates #[derive(Trait1, Trait2)]
                let mut traits = Vec::new();
                for (_key, expr) in &decorator.arguments {
                    Self::collect_derive_trait_identifiers(expr, &mut traits);
                }
                if !traits.is_empty() {
                    let merged = CodeGenerator::merge_standard_derive_traits(
                        traits,
                        self.infer_derivable_traits(s),
                    );
                    output.push_str(&format!("#[derive({})]\n", merged.join(", ")));

                    // TDD FIX: Register this struct as Copy if derived (explicit or inferred)
                    if merged.contains(&"Copy".to_string()) {
                        self.copy_types_registry.insert(s.name.clone());
                    }
                }
            } else {
                // Map Windjammer decorator to Rust attribute
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
        }

        // WINDJAMMER PHILOSOPHY: Auto-derive common traits for simple structs
        // If a struct has no @auto or @derive decorator, but all fields are primitive/Copy types,
        // automatically add Clone, Copy, Debug, PartialEq - this is what the user would want 90% of the time
        let has_derive_decorator = s
            .decorators
            .iter()
            .any(|d| d.name == "auto" || d.name == "derive");
        if !has_derive_decorator {
            let inferred_traits = self.infer_derivable_traits(s);
            if !inferred_traits.is_empty() {
                output.push_str(&format!("#[derive({})]\n", inferred_traits.join(", ")));

                // TDD FIX: Register this struct as Copy if it was inferred
                // This allows other structs to know this type is Copy when checking their fields
                if inferred_traits.contains(&"Copy".to_string()) {
                    self.copy_types_registry.insert(s.name.clone());
                }
            }
        }

        // CRITICAL: All Windjammer structs must use #[repr(C)] to guarantee
        // field ordering matches declaration order. Without this, Rust may
        // reorder fields for optimization, corrupting GPU uniform buffers
        // and any code that depends on memory layout (to_bytes, FFI, etc.).
        output.push_str("#[repr(C)]\n");

        // Add struct declaration with type parameters
        let pub_prefix = if s.is_pub {
            "pub "
        } else {
            ""
        };
        output.push_str(&format!("{}struct ", pub_prefix));
        output.push_str(&s.name);
        if !s.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&s.type_params));
            output.push('>');
        }

        // Add where clause if present
        output.push_str(&codegen_helpers::format_where_clause(&s.where_clause));

        // Check for tuple struct: struct Name(T1, T2);
        if let Some(tuple_types) = &s.tuple_fields {
            self.tuple_struct_names.insert(s.name.clone());
            output.push('(');
            let fields: Vec<String> = tuple_types
                .iter()
                .map(|t| format!("pub {}", self.type_to_rust(t)))
                .collect();
            output.push_str(&fields.join(", "));
            output.push_str(");");
            return output;
        }

        // Check if this is a unit struct (no fields)
        if s.fields.is_empty() {
            // Unit struct - end with semicolon
            output.push(';');
            return output;
        }

        output.push_str(" {\n");

        for field in &s.fields {
            // Emit doc comment for field if present
            if let Some(doc) = &field.doc_comment {
                output.push_str(&format!("    /// {}\n", doc));
            }

            // Generate decorators for the field (convert to Rust attributes)
            for decorator in &field.decorators {
                // Handle @arg decorator specially - it's a clap field attribute
                if decorator.name == "arg" {
                    output.push_str("    #[arg(");
                    let args: Vec<String> = decorator
                        .arguments
                        .iter()
                        .map(|(key, expr)| {
                            // Handle special cases for clap arguments
                            match key.as_str() {
                                "short" => {
                                    // short takes a character literal
                                    format!("short = {}", self.generate_expression_immut(expr))
                                }
                                "long" => {
                                    // long takes a string literal
                                    format!("long = {}", self.generate_expression_immut(expr))
                                }
                                "default_value" => {
                                    format!(
                                        "default_value = {}",
                                        self.generate_expression_immut(expr)
                                    )
                                }
                                "help" => {
                                    format!("help = {}", self.generate_expression_immut(expr))
                                }
                                _ => format!("{} = {}", key, self.generate_expression_immut(expr)),
                            }
                        })
                        .collect();
                    output.push_str(&args.join(", "));
                    output.push_str(")]\n");
                } else {
                    // Generic decorator handling
                    output.push_str(&format!("    #[{}(", decorator.name));
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
            // Respect field visibility from source
            let pub_keyword = if field.is_pub {
                "pub "
            } else {
                ""
            };
            output.push_str(&format!(
                "    {}{}: {},\n",
                pub_keyword,
                field.name,
                self.type_to_rust(&field.field_type)
            ));
        }

        output.push('}');

        // WINDJAMMER PHILOSOPHY: Auto-generate to_bytes() for structs with only
        // GPU-serializable fields (f32, u32, i32, bool, fixed-size arrays of those).
        // This eliminates manual byte management for GPU uniform uploads and ensures
        // correct bit patterns for all types. The compiler does the hard work.
        if !s.fields.is_empty()
            && s.fields
                .iter()
                .all(|f| Self::is_gpu_serializable_type(&f.field_type))
        {
            output.push_str(&Self::generate_to_bytes_impl(s));
        }

        output
    }

    /// Returns true if a type can be serialized to fixed-size GPU-compatible bytes.
    fn is_gpu_serializable_type(ty: &Type) -> bool {
        matches!(
            ty,
            Type::Float    // f64 -> but typically f32 in context
            | Type::Bool
            | Type::Int    // i64
            | Type::Int32  // i32
            | Type::Uint // u64
        ) || matches!(ty, Type::Custom(name) if matches!(name.as_str(), "f32" | "u32" | "i32" | "f64" | "u64" | "i64" | "u8" | "i8" | "u16" | "i16" | "usize" | "isize"))
            || matches!(ty, Type::Array(inner, _) if Self::is_gpu_serializable_type(inner))
    }

    /// Generates an impl block with `to_bytes(&self) -> Vec<u8>` for GPU-serializable structs.
    fn generate_to_bytes_impl(s: &StructDecl) -> String {
        let mut out = String::new();
        out.push_str(&format!("\nimpl {} {{\n", s.name));
        out.push_str("    pub fn to_bytes(&self) -> Vec<u8> {\n");

        // Calculate total byte size for capacity hint
        let cap = s
            .fields
            .iter()
            .map(|f| Self::byte_size_of_type(&f.field_type))
            .sum::<usize>();
        out.push_str(&format!(
            "        let mut __bytes = Vec::with_capacity({});\n",
            cap
        ));

        for field in &s.fields {
            Self::emit_field_serialization(
                &mut out,
                &format!("self.{}", field.name),
                &field.field_type,
            );
        }

        out.push_str("        __bytes\n");
        out.push_str("    }\n");
        out.push_str("}\n");
        out
    }

    fn byte_size_of_type(ty: &Type) -> usize {
        match ty {
            Type::Float => 8, // f64
            Type::Int => 8,   // i64
            Type::Uint => 8,  // u64
            Type::Int32 => 4, // i32
            Type::Bool => 4,  // GPU bools are u32
            Type::Custom(name) => match name.as_str() {
                "f32" | "u32" | "i32" => 4,
                "f64" | "u64" | "i64" => 8,
                "u8" | "i8" => 1,
                "u16" | "i16" => 2,
                "usize" | "isize" => 8,
                _ => 4,
            },
            Type::Array(inner, n) => Self::byte_size_of_type(inner) * n,
            _ => 4,
        }
    }

    fn emit_field_serialization(out: &mut String, expr: &str, ty: &Type) {
        match ty {
            Type::Bool => {
                // GPU bools are 4 bytes (u32): true=1, false=0
                out.push_str(&format!(
                    "        __bytes.extend_from_slice(&(if {} {{ 1u32 }} else {{ 0u32 }}).to_ne_bytes());\n",
                    expr
                ));
            }
            Type::Array(inner, _) => {
                out.push_str(&format!("        for __el in &{} {{\n", expr));
                let mut inner_out = String::new();
                Self::emit_field_serialization(&mut inner_out, "__el", inner);
                // Indent inner serialization by one extra level
                for line in inner_out.lines() {
                    if !line.is_empty() {
                        out.push_str("    ");
                    }
                    out.push_str(line);
                    out.push('\n');
                }
                out.push_str("        }\n");
            }
            _ => {
                // f32, u32, i32, f64, u64, i64, etc. - all have to_ne_bytes()
                out.push_str(&format!(
                    "        __bytes.extend_from_slice(&{}.to_ne_bytes());\n",
                    expr
                ));
            }
        }
    }

    pub(super) fn generate_enum(&mut self, e: &EnumDecl) -> String {
        let mut output = String::new();

        // WINDJAMMER PHILOSOPHY: Auto-derive common traits for enums
        // All enums get Clone, Debug by default
        // Only add PartialEq if ALL variants support it
        // Unit-only enums (no data) also get Copy
        let mut traits = vec!["Clone".to_string(), "Debug".to_string()];

        // Check if all variants support PartialEq
        let all_variants_partial_eq = self.all_enum_variants_are_partial_eq(&e.variants);
        if all_variants_partial_eq {
            traits.push("PartialEq".to_string());
        }

        // WINDJAMMER PHILOSOPHY: Auto-derive Copy for enums when ALL variant fields are Copy types.
        // This includes unit-only enums (trivially Copy) and data-carrying enums where
        // every field in every variant is a Copy type (i32, f32, bool, etc.).
        // Enums with String, Vec, or other non-Copy fields should NOT get Copy.
        let all_variants_copy = self.all_enum_variants_are_copy(&e.variants);
        if all_variants_copy {
            traits.push("Copy".to_string());
            self.copy_types_registry.insert(e.name.clone());
        }
        output.push_str(&format!("#[derive({})]\n", traits.join(", ")));

        let pub_prefix = if e.is_pub {
            "pub "
        } else {
            ""
        };
        output.push_str(&format!("{}enum {}", pub_prefix, e.name));

        // Generate generic parameters: enum Option<T>, enum Result<T, E>
        if !e.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&e.type_params));
            output.push('>');
        }

        output.push_str(" {\n");

        for variant in &e.variants {
            // Emit doc comment for variant if present
            if let Some(doc) = &variant.doc_comment {
                output.push_str(&format!("    /// {}\n", doc));
            }

            use crate::parser::EnumVariantData;
            match &variant.data {
                EnumVariantData::Unit => {
                    output.push_str(&format!("    {},\n", variant.name));
                }
                EnumVariantData::Tuple(types) => {
                    let type_strs: Vec<String> =
                        types.iter().map(|t| self.type_to_rust(t)).collect();
                    output.push_str(&format!(
                        "    {}({}),\n",
                        variant.name,
                        type_strs.join(", ")
                    ));
                }
                EnumVariantData::Struct(fields) => {
                    let field_strs: Vec<String> = fields
                        .iter()
                        .map(|(name, ty)| format!("{}: {}", name, self.type_to_rust(ty)))
                        .collect();
                    output.push_str(&format!(
                        "    {} {{ {} }},\n",
                        variant.name,
                        field_strs.join(", ")
                    ));
                }
            }
        }

        output.push('}');
        output
    }

    pub(super) fn generate_trait_with_analysis(
        &mut self,
        trait_decl: &crate::parser::TraitDecl<'ast>,
        analyzed: &[AnalyzedFunction<'ast>],
    ) -> String {
        // RECURSION GUARD: Prevent infinite recursion during trait generation
        // This can happen if the same trait is generated multiple times in a cycle
        if self.generating_traits.contains(&trait_decl.name) {
            eprintln!(
                "⚠️  TRAIT RECURSION GUARD: Skipping trait {} (already generating)",
                trait_decl.name
            );
            eprintln!(
                "   Currently generating {} traits: {:?}",
                self.generating_traits.len(),
                self.generating_traits
            );
            eprintln!("   🚨 WARNING: Returning EMPTY STRING for this trait!");
            return String::new(); // Return empty to break the cycle
        }

        // Add to generating set
        self.generating_traits.insert(trait_decl.name.clone());

        let mut output = String::new();

        // TODO: Add is_pub field to TraitDecl and check it properly
        // For now, always emit pub for traits (the common case)
        output.push_str("pub trait ");
        output.push_str(&trait_decl.name);

        // Generate generic parameters: trait From<T> { ... }
        if !trait_decl.generics.is_empty() {
            output.push('<');
            output.push_str(&trait_decl.generics.join(", "));
            output.push('>');
        }

        // Generate supertraits: trait Manager: Employee + Person
        if !trait_decl.supertraits.is_empty() {
            output.push_str(": ");
            output.push_str(&trait_decl.supertraits.join(" + "));
        }

        output.push_str(" {\n");
        self.indent_level += 1;

        // Generate associated type declarations: type Item;
        for assoc_type in &trait_decl.associated_types {
            output.push_str(&self.indent());
            output.push_str(&format!("type {};\n", assoc_type.name));
        }

        if !trait_decl.associated_types.is_empty() {
            output.push('\n');
        }

        // Generate trait methods
        for method in &trait_decl.methods {
            // THE WINDJAMMER WAY: Look up analyzed data for this method
            // Priority: 1) Global cross-file inferred (analyzed_trait_methods)
            //           2) Local analyzed (for default implementations)
            let analyzed_method =
                if let Some(trait_methods) = self.analyzed_trait_methods.get(&trait_decl.name) {
                    if let Some(global_analysis) = trait_methods.get(&method.name) {
                        // Use global cross-file inferred analysis
                        Some(global_analysis)
                    } else if method.body.is_some() {
                        // Fallback to local analysis for default impl
                        analyzed.iter().find(|f| f.decl.name == method.name)
                    } else {
                        None
                    }
                } else if method.body.is_some() {
                    // No global analysis available, use local for default impl
                    analyzed.iter().find(|f| f.decl.name == method.name)
                } else {
                    None
                };
            output.push_str(&self.indent());

            if method.is_async {
                output.push_str("async ");
            }

            output.push_str("fn ");
            output.push_str(&method.name);
            output.push('(');

            // TDD FIX: Trait Method Ownership Inference
            // THE WINDJAMMER WAY: If trait method has no explicit self parameter,
            // infer it automatically based on the method type:
            // - Associated functions returning Self → No self (constructor)
            // - All other methods → inferred from impl bodies or default &mut self
            let has_self_param = method.parameters.iter().any(|p| p.name == "self");

            // Structural detection: a method that returns Self or the trait's type and has no self
            // parameter is an associated function (constructor), not an instance method.
            let returns_self = matches!(
                &method.return_type,
                Some(Type::Custom(name)) if name == "Self"
            );
            let returns_trait_type = matches!(
                &method.return_type,
                Some(Type::Custom(name)) if name == &trait_decl.name
            );
            let is_associated_fn = !has_self_param && (returns_self || returns_trait_type);

            let mut params: Vec<String> = Vec::new();

            // Add self parameter if missing and not an associated function
            if !has_self_param && !is_associated_fn {
                let returns_bare_self = returns_self;
                // Check if we have analyzed ownership for this method
                let self_ownership = if let Some(analyzed) = analyzed_method {
                    analyzed.inferred_ownership.get("self").copied()
                } else if let Some(trait_methods) =
                    self.analyzed_trait_methods.get(&trait_decl.name)
                {
                    trait_methods
                        .get(&method.name)
                        .and_then(|m| m.inferred_ownership.get("self").copied())
                } else {
                    None
                };

                // Associated function: `fn create() -> Self` has no receiver — do not emit &mut self.
                if !(self_ownership.is_none() && returns_bare_self) {
                    let self_param = match self_ownership {
                        Some(OwnershipMode::Borrowed) => "&self",
                        Some(OwnershipMode::MutBorrowed) | None => "&mut self",
                        Some(OwnershipMode::Owned) => "self",
                    };
                    params.push(self_param.to_string());
                }
            }

            // Generate parameters
            // NOTE: Trait method signatures cannot have 'mut' keyword in Rust
            // Only implementations can have 'mut self' or 'mut param'
            let method_params: Vec<String> = method
                .parameters
                .iter()
                .map(|param| {
                    // THE WINDJAMMER WAY:
                    // Use the analyzed ownership from the analyzer, which has inferred
                    // the most permissive signature needed based on ALL implementations!
                    let ownership = if let Some(analyzed) = analyzed_method {
                        // Has default implementation OR global cross-file analysis - use analyzer's inferred ownership
                        match analyzed.inferred_ownership.get(&param.name) {
                            Some(OwnershipMode::Borrowed) => OwnershipHint::Ref,
                            Some(OwnershipMode::MutBorrowed) => OwnershipHint::Mut,
                            Some(OwnershipMode::Owned) => OwnershipHint::Owned,
                            None => param.ownership.clone(), // Fallback to AST
                        }
                    } else {
                        // No default implementation - check analyzed_trait_methods
                        // The analyzer has inferred the signature from ALL impls!
                        if let Some(trait_methods) =
                            self.analyzed_trait_methods.get(&trait_decl.name)
                        {
                            if let Some(method_analysis) = trait_methods.get(&method.name) {
                                if let Some(inferred_ownership) =
                                    method_analysis.inferred_ownership.get(&param.name)
                                {
                                    match inferred_ownership {
                                        OwnershipMode::Borrowed => OwnershipHint::Ref,
                                        OwnershipMode::MutBorrowed => OwnershipHint::Mut,
                                        OwnershipMode::Owned => OwnershipHint::Owned,
                                    }
                                } else {
                                    param.ownership.clone()
                                }
                            } else {
                                // Fallback to AST
                                param.ownership.clone()
                            }
                        } else {
                            // Fallback to AST
                            param.ownership.clone()
                        }
                    };

                    // THE WINDJAMMER WAY: Check if param.type_ already contains a reference
                    // If so, don't add another & (prevents &&Input bug)
                    let type_already_has_ref =
                        matches!(param.type_, Type::Reference(_) | Type::MutableReference(_));

                    let type_str = match &ownership {
                        OwnershipHint::Owned => {
                            if param.name == "self" {
                                // Trait signatures: just 'self' (no 'mut')
                                return "self".to_string();
                            }
                            // Trait signatures: no 'mut' for parameters
                            return format!("{}: {}", param.name, self.type_to_rust(&param.type_));
                        }
                        OwnershipHint::Ref => {
                            if param.name == "self" {
                                return "&self".to_string();
                            }
                            // CRITICAL FIX: If type already has &, don't add another!
                            if type_already_has_ref {
                                self.type_to_rust(&param.type_) // Already has &
                            } else {
                                format!("&{}", self.type_to_rust(&param.type_))
                            }
                        }
                        OwnershipHint::Mut => {
                            if param.name == "self" {
                                return "&mut self".to_string();
                            }
                            // CRITICAL FIX: If type already has &mut, don't add another!
                            if type_already_has_ref {
                                self.type_to_rust(&param.type_) // Already has &mut
                            } else {
                                format!("&mut {}", self.type_to_rust(&param.type_))
                            }
                        }
                        OwnershipHint::Inferred => {
                            // TRAIT SIGNATURES: Default to &self for trait methods
                            // This prevents E0277 (Self not Sized) errors
                            if param.name == "self" {
                                return "&self".to_string();
                            }
                            // Owned parameter (no &)
                            self.type_to_rust(&param.type_)
                        }
                    };

                    format!("{}: {}", param.name, type_str)
                })
                .collect();

            // Append method parameters to params (which may already have self)
            params.extend(method_params);

            output.push_str(&params.join(", "));
            output.push(')');

            // Return type
            if let Some(ret_type) = &method.return_type {
                output.push_str(" -> ");
                output.push_str(&self.type_to_rust(ret_type));
            }

            // Default implementation (if provided)
            if let Some(body) = &method.body {
                output.push_str(" {\n");
                self.indent_level += 1;

                // THE WINDJAMMER WAY: Handle implicit returns in default trait methods.
                // The last expression in a block must NOT have a trailing semicolon
                // if it's the return value. `0;` evaluates to `()`, not `i32`.
                let body_len = body.len();
                for (i, stmt) in body.iter().enumerate() {
                    let is_last = i == body_len - 1;

                    if is_last && matches!(stmt, Statement::Expression { .. }) {
                        // Last statement is an expression - generate without semicolon
                        // (it's the implicit return value of the default implementation)
                        if let Statement::Expression { expr, .. } = stmt {
                            output.push_str(&self.indent());
                            output.push_str(&self.generate_expression(expr));
                            output.push('\n');
                        }
                    } else {
                        output.push_str(&self.generate_statement(stmt));
                    }
                }

                self.indent_level -= 1;
                output.push_str(&self.indent());
                output.push_str("}\n");
            } else {
                output.push_str(";\n");
            }
        }

        self.indent_level -= 1;
        output.push('}');

        // Remove from generating set before returning
        self.generating_traits.remove(&trait_decl.name);

        output
    }

    pub(super) fn generate_impl(
        &mut self,
        impl_block: &ImplBlock<'ast>,
        analyzed: &[AnalyzedFunction<'ast>],
    ) -> String {
        let mut output = String::new();

        // Check if this impl block has @export or @wasm_bindgen decorator
        let has_wasm_export = impl_block
            .decorators
            .iter()
            .any(|d| d.name == "export" || d.name == "wasm_bindgen");

        // Generate decorators (map Windjammer decorators to Rust attributes)
        for decorator in &impl_block.decorators {
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

        // Generate impl with type parameters
        output.push_str("impl");
        if !impl_block.type_params.is_empty() {
            output.push('<');
            output.push_str(&self.format_type_params(&impl_block.type_params));
            output.push('>');
        } else if let Some(inferred) =
            super::codegen_helpers::infer_impl_header_type_params_from_type_name(
                &impl_block.type_name,
            )
        {
            // Rust requires `impl<T> Foo<T>` when the user wrote `impl Foo<T>` (no `impl<T>`).
            output.push('<');
            output.push_str(&inferred.join(", "));
            output.push('>');
        }
        output.push(' ');

        if let Some(trait_name) = &impl_block.trait_name {
            // Trait implementation: impl<T> Trait<TypeArgs> for Type<T>
            output.push_str(trait_name);

            // Generate trait type arguments if present: From<int> -> From<i64>
            if let Some(type_args) = &impl_block.trait_type_args {
                output.push('<');
                let args_str: Vec<String> =
                    type_args.iter().map(|t| self.type_to_rust(t)).collect();
                output.push_str(&args_str.join(", "));
                output.push('>');
            }

            output.push_str(&format!(" for {}", impl_block.type_name));
        } else {
            // Inherent implementation: impl<T> Type<T>
            output.push_str(&impl_block.type_name);
        }

        // Generic impls that clone `self.dense` / `self.dense[i]` need `T: Clone` for Rust Vec/element Clone.
        let mut merged_where = impl_block.where_clause.clone();
        let inferred_clone = codegen_helpers::infer_clone_where_bounds_for_impl(impl_block);
        if !inferred_clone.is_empty() {
            merged_where = codegen_helpers::merge_where_clauses(merged_where, inferred_clone);
        }
        output.push_str(&codegen_helpers::format_where_clause(&merged_where));

        output.push_str(" {\n");

        self.indent_level += 1;

        // Generate associated type implementations: type Item = i32;
        for assoc_type in &impl_block.associated_types {
            output.push_str(&self.indent());
            output.push_str(&format!("type {}", assoc_type.name));
            if let Some(concrete_type) = &assoc_type.concrete_type {
                output.push_str(&format!(" = {};\n", self.type_to_rust(concrete_type)));
            } else {
                output.push_str(";\n");
            }
        }

        if !impl_block.associated_types.is_empty() {
            output.push('\n');
        }

        // Store the wasm export flag and trait impl flag for use in generate_function
        let old_in_wasm_impl = self.in_wasm_bindgen_impl;
        let old_in_trait_impl = self.in_trait_impl;
        let old_trait_impl_name = self.current_trait_impl_name.take();
        self.in_wasm_bindgen_impl = has_wasm_export;
        self.in_trait_impl = impl_block.trait_name.is_some();
        // E0053 FIX: Track trait name so impl methods use trait's ownership (not impl's inferred)
        self.current_trait_impl_name = impl_block.trait_name.clone();

        // Pre-classify methods as instance (takes self) vs static for Self:: vs self. dispatch.
        // A method is instance if: it has explicit self, analyzer inferred self, or it accesses fields.
        let mut instance_methods: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for func in &impl_block.functions {
            let has_explicit_self = func.parameters.iter().any(|p| p.name == "self");
            let has_inferred_self = analyzed
                .iter()
                .find(|af| Self::analyzed_matches_impl_ast(af, func, &impl_block.trait_name))
                .map(|af| af.inferred_ownership.contains_key("self"))
                .unwrap_or(false);
            let accesses_fields = if !self.current_struct_fields.is_empty() {
                let local_bindings = self_analysis::collect_local_bindings(&func.body);
                let ctx = self_analysis::AnalysisContext::with_locals(
                    &func.parameters,
                    &self.current_struct_fields,
                    &local_bindings,
                );
                self_analysis::function_accesses_fields(&ctx, func)
                    || self_analysis::function_mutates_fields(&ctx, func)
            } else {
                false
            };
            if has_explicit_self || has_inferred_self || accesses_fields {
                instance_methods.insert(func.name.clone());
            }
        }
        self.current_impl_instance_methods = instance_methods;

        for func in &impl_block.functions {
            if let Some(analyzed_func) = analyzed
                .iter()
                .find(|af| Self::analyzed_matches_impl_ast(af, func, &impl_block.trait_name))
            {
                output.push_str(&self.generate_function(analyzed_func));
                output.push('\n');
            }
        }

        self.current_impl_instance_methods.clear();
        self.in_wasm_bindgen_impl = old_in_wasm_impl;
        self.in_trait_impl = old_in_trait_impl;
        self.current_trait_impl_name = old_trait_impl_name;

        self.indent_level -= 1;
        output.push('}');
        output
    }

    /// Generate automatic trait implementation for @component decorator
    pub(super) fn generate_component_impl(&mut self, s: &StructDecl) -> String {
        let mut output = String::new();

        // For now, generate a marker comment
        // In future iterations, we'll generate actual trait implementations
        output.push_str(&format!(
            "// Component trait implementation for {}\n// TODO: Implement Component trait",
            s.name
        ));

        output
    }

    /// Generate automatic trait implementation for @game decorator
    pub(super) fn generate_game_impl(&mut self, s: &StructDecl) -> String {
        let mut output = String::new();

        // Generate Default implementation
        // All fields are initialized to their default values (0, 0.0, false, etc.)
        output.push_str(&format!("impl Default for {} {{\n", s.name));
        output.push_str("    fn default() -> Self {\n");
        output.push_str(&format!("        {} {{\n", s.name));

        for field in &s.fields {
            let default_value = match &field.field_type {
                Type::Int | Type::Int32 | Type::Uint => "0",
                Type::Float => "0.0",
                Type::Bool => "false",
                Type::String => "String::new()",
                Type::Vec(_) => "Vec::new()",
                Type::Custom(name) if name == "String" => "String::new()",
                _ => "Default::default()",
            };
            output.push_str(&format!("            {}: {},\n", field.name, default_value));
        }

        output.push_str("        }\n");
        output.push_str("    }\n");
        output.push('}');

        output
    }
}
