//! Analyzer construction: Copy registry wiring, stdlib trait stubs, prelude trait hydration.

use crate::parser::*;
use std::collections::{HashMap, HashSet};

use super::{AnalyzedFunction, Analyzer, SignatureRegistry};

impl<'ast> Default for Analyzer<'ast> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ast> Analyzer<'ast> {
    pub fn new() -> Self {
        Self::new_with_copy_structs(HashSet::new())
    }

    /// Create a new Analyzer with a pre-populated set of Copy structs from global registry
    /// This enables proper Copy type detection across multiple files
    pub fn new_with_copy_structs(global_copy_structs: HashSet<String>) -> Self {
        let mut analyzer = Analyzer::new_empty(global_copy_structs);

        // Pre-register standard library traits so the analyzer knows their signatures
        analyzer.register_stdlib_traits();
        analyzer
            .hydrate_prelude_trait_method_signatures()
            .expect("prelude trait method analysis (Drop, etc.)");

        analyzer
    }

    /// Update the analyzer's Copy structs registry (for shared analyzer across files)
    /// This allows newly discovered Copy structs to be available for subsequent file analysis
    pub fn update_copy_structs(&mut self, global_copy_structs: HashSet<String>) {
        self.copy_structs = global_copy_structs;
    }

    /// Register a single struct as Copy (for cross-crate metadata or testing)
    pub fn register_copy_struct(&mut self, name: &str) {
        self.copy_structs.insert(name.to_string());
    }

    /// Set the global struct field types for cross-file nested field chain resolution.
    pub fn set_global_struct_field_types(&mut self, types: HashMap<String, HashMap<String, Type>>) {
        self.global_struct_field_types = types;
    }

    /// TDD FIX: Remove a struct from the Copy set (e.g., when local definition differs from metadata)
    pub fn unregister_copy_struct(&mut self, name: &str) {
        self.copy_structs.remove(name);
    }

    /// TDD FIX: Check if a struct is registered as Copy
    pub fn is_copy_struct(&self, name: &str) -> bool {
        self.copy_structs.contains(name)
    }

    /// Get all detected Copy struct names (for metadata emission)
    pub fn get_copy_structs(&self) -> Vec<String> {
        self.copy_structs.iter().cloned().collect()
    }

    /// Pre-register standard library traits (Add, Sub, Mul, Div, etc.)
    /// This allows the analyzer to correctly handle trait implementations
    /// for stdlib traits without needing to parse Rust's stdlib.
    fn register_stdlib_traits(&mut self) {
        use crate::parser::ast::{
            AssociatedType, OwnershipHint, Parameter, TraitDecl, TraitMethod, Type,
        };

        // Helper to create a binary operator trait (Add, Sub, Mul, Div, etc.)
        let create_binary_op_trait = |name: &str, method: &str| -> TraitDecl {
            TraitDecl {
                name: name.to_string(),
                generics: vec!["Rhs".to_string()],
                supertraits: vec![],
                methods: vec![TraitMethod {
                    name: method.to_string(),
                    parameters: vec![
                        Parameter {
                            name: "self".to_string(),
                            pattern: None,
                            type_: Type::Custom("Self".to_string()),
                            ownership: OwnershipHint::Owned,
                            is_mutable: false,
                            decorators: Vec::new(),
                        },
                        Parameter {
                            name: "rhs".to_string(),
                            pattern: None,
                            type_: Type::Custom("Rhs".to_string()),
                            ownership: OwnershipHint::Owned,
                            is_mutable: false,
                            decorators: Vec::new(),
                        },
                    ],
                    return_type: Some(Type::Custom("Output".to_string())),
                    is_async: false,
                    body: None,
                    doc_comment: None,
                }],
                associated_types: vec![AssociatedType {
                    name: "Output".to_string(),
                    concrete_type: None,
                }],
                doc_comment: None,
            }
        };

        // Register common operator traits
        self.trait_definitions
            .insert("Add".to_string(), create_binary_op_trait("Add", "add"));
        self.trait_definitions
            .insert("Sub".to_string(), create_binary_op_trait("Sub", "sub"));
        self.trait_definitions
            .insert("Mul".to_string(), create_binary_op_trait("Mul", "mul"));
        self.trait_definitions
            .insert("Div".to_string(), create_binary_op_trait("Div", "div"));
        self.trait_definitions
            .insert("Rem".to_string(), create_binary_op_trait("Rem", "rem"));

        // Register unary operator traits
        // Neg: -x
        self.trait_definitions.insert(
            "Neg".to_string(),
            TraitDecl {
                name: "Neg".to_string(),
                generics: vec![],
                supertraits: vec![],
                methods: vec![TraitMethod {
                    name: "neg".to_string(),
                    parameters: vec![Parameter {
                        name: "self".to_string(),
                        pattern: None,
                        type_: Type::Custom("Self".to_string()),
                        ownership: OwnershipHint::Owned, // THE WINDJAMMER WAY: Neg uses owned self!
                        is_mutable: false,
                        decorators: Vec::new(),
                    }],
                    return_type: Some(Type::Custom("Output".to_string())),
                    is_async: false,
                    body: None,
                    doc_comment: None,
                }],
                associated_types: vec![AssociatedType {
                    name: "Output".to_string(),
                    concrete_type: None,
                }],
                doc_comment: None,
            },
        );

        // Rust std `Drop::drop(&mut self)` — Windjammer users write `fn drop(self)`; generated Rust
        // must match or rustc reports E0186/E0053. Not parsed from .wj, so register like operator traits.
        self.trait_definitions.insert(
            "Drop".to_string(),
            TraitDecl {
                name: "Drop".to_string(),
                generics: vec![],
                supertraits: vec![],
                methods: vec![TraitMethod {
                    name: "drop".to_string(),
                    parameters: vec![Parameter {
                        name: "self".to_string(),
                        pattern: None,
                        type_: Type::Custom("Self".to_string()),
                        ownership: OwnershipHint::Mut,
                        is_mutable: false,
                        decorators: Vec::new(),
                    }],
                    return_type: None,
                    is_async: false,
                    body: None,
                    doc_comment: None,
                }],
                associated_types: vec![],
                doc_comment: None,
            },
        );
    }

    /// Analyze prelude traits that exist only in `trait_definitions` (no `trait Drop` in user source).
    fn hydrate_prelude_trait_method_signatures(&mut self) -> Result<(), String> {
        const PRELUDE_TRAIT_KEYS: &[&str] = &["Drop"];
        let empty_registry = SignatureRegistry::new();
        for &trait_key in PRELUDE_TRAIT_KEYS {
            let Some(decl) = self.trait_definitions.get(trait_key).cloned() else {
                continue;
            };
            let mut to_add: Vec<(String, AnalyzedFunction<'ast>)> = Vec::new();
            for method in &decl.methods {
                let already = self
                    .analyzed_trait_methods
                    .get(&decl.name)
                    .map(|m| m.contains_key(&method.name))
                    .unwrap_or(false);
                if already {
                    continue;
                }
                let func = FunctionDecl {
                    name: method.name.clone(),
                    is_pub: true,
                    is_extern: false,
                    type_params: vec![],
                    where_clause: vec![],
                    decorators: vec![],
                    is_async: method.is_async,
                    parameters: method.parameters.clone(),
                    return_type: method.return_type.clone(),
                    return_decorators: Vec::new(),
                    body: vec![],
                    parent_type: None,
                    impl_trait: None,
                    doc_comment: method.doc_comment.clone(),
                };
                let analyzed_func =
                    self.analyze_trait_method(&func, &empty_registry, Some(decl.name.as_str()))?;
                to_add.push((method.name.clone(), analyzed_func));
            }
            let entry = self
                .analyzed_trait_methods
                .entry(decl.name.clone())
                .or_default();
            for (name, analyzed_func) in to_add {
                entry.insert(name, analyzed_func);
            }
        }
        Ok(())
    }
}
