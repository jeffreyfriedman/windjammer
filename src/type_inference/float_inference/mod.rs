/// Float Type Inference Engine
///
/// Tracks constraints for float literals and unifies them across expressions.
use crate::parser::ast::core::{Expression, Item, Statement};
use crate::parser::ast::types::Type;
use crate::parser::Program;
use crate::type_inference::struct_field_registry;
use std::collections::HashMap;

/// Unique identifier for an expression
/// THE WINDJAMMER WAY: Sequential IDs ensure uniqueness even when expressions lack locations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprId {
    /// Sequential ID assigned during AST traversal (guaranteed unique GLOBALLY across all files)
    pub seq_id: usize,
    /// Source file ID (for multi-file disambiguation)
    pub file_id: usize,
    /// Optional source location for debugging (may be duplicate within file)
    pub line: usize,
    pub col: usize,
}

/// Float type (f32 or f64)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatType {
    F32,
    F64,
    Unknown, // Not yet inferred
}

/// Constraint on an expression's float type
#[derive(Debug, Clone)]
pub enum Constraint {
    /// Expression must be f32
    MustBeF32(ExprId, String), // reason
    /// Expression must be f64
    MustBeF64(ExprId, String), // reason
    /// Two expressions must have the same type
    MustMatch(ExprId, ExprId, String), // reason
}

/// Float type inference state
#[derive(Clone)]
pub struct FloatInference {
    /// Map expression ID → inferred float type
    pub inferred_types: HashMap<ExprId, FloatType>,
    /// Collected constraints
    constraints: Vec<Constraint>,
    /// Errors detected during inference
    pub errors: Vec<String>,
    /// Function signature registry: name → (param_types, return_type)
    function_signatures: HashMap<String, (Vec<Type>, Option<Type>)>,
    /// Variable assignment tracking: variable name → initial value ExprId
    var_assignments: HashMap<String, ExprId>,
    /// Variable type tracking: variable name → explicit Type (for let x: Type = ...)
    var_types: HashMap<String, Type>,
    /// Sequential ID counter for generating unique ExprIds
    next_seq_id: usize,
    /// Struct field types: struct_name → field_name → Type
    struct_field_types: HashMap<String, HashMap<String, Type>>,
    /// THE WINDJAMMER WAY: Cache ExprIds by location to ensure same expression = same ID
    /// Key: (file_id, line, col), Value: the first ExprId assigned to that location
    expr_id_cache: HashMap<(usize, usize, usize), ExprId>,
    /// Current file being analyzed (for file-aware ExprId generation)
    current_file_id: usize,
    /// File name → file ID mapping (for multi-file builds)
    file_name_to_id: HashMap<String, usize>,
    /// Next file ID to assign
    next_file_id: usize,
    /// Source root for resolving metadata file paths
    source_root: Option<std::path::PathBuf>,
    /// Current impl block type (for resolving `self` field access)
    current_impl_type: Option<String>,
    /// Variable element types: var_name → element_type (for Vec<T>, HashMap<K,V>)
    var_element_types: HashMap<String, Type>,
    /// Const/static types: name → Type (for const F: f32 = 1.0)
    const_types: HashMap<String, Type>,
    /// External crate metadata: crate_name → path to metadata.json directory
    /// Used for cross-crate type inference (e.g., windjammer_game_core)
    external_crate_metadata_paths: std::collections::HashMap<String, std::path::PathBuf>,
    /// Debug: Optional source text for error context (line extraction)
    debug_source: Option<String>,
    /// Library multipass: module path for the current `.wj` file.
    current_file_module_path: Vec<String>,
    struct_defining_module_paths: HashMap<String, Vec<Vec<String>>>,
    imported_type_registry_keys: HashMap<String, String>,
    /// `pub use` per module path — populated by library build pre-pass for glob imports.
    module_re_exports: HashMap<String, HashMap<String, String>>,
    /// Type alias registry: alias_name → resolved Type
    type_aliases: HashMap<String, Type>,
}

impl Default for FloatInference {
    fn default() -> Self {
        Self::new()
    }
}

impl FloatInference {
    pub fn new() -> Self {
        FloatInference {
            inferred_types: HashMap::new(),
            constraints: Vec::new(),
            errors: Vec::new(),
            function_signatures: HashMap::new(),
            var_assignments: HashMap::new(),
            var_types: HashMap::new(),
            next_seq_id: 1, // Start at 1, 0 reserved for "unknown"
            struct_field_types: HashMap::new(),
            expr_id_cache: HashMap::new(),
            source_root: None,
            current_impl_type: None,
            var_element_types: HashMap::new(),
            const_types: HashMap::new(),
            external_crate_metadata_paths: std::collections::HashMap::new(),
            debug_source: None,
            current_file_module_path: Vec::new(),
            struct_defining_module_paths: HashMap::new(),
            imported_type_registry_keys: HashMap::new(),
            module_re_exports: HashMap::new(),
            current_file_id: 0,
            file_name_to_id: HashMap::new(),
            next_file_id: 1,
            type_aliases: HashMap::new(),
        }
    }

    /// Set current file being analyzed (for file-aware ExprId generation)
    /// Returns the file_id assigned to this file
    pub fn set_current_file(&mut self, file: String) -> usize {
        if let Some(&id) = self.file_name_to_id.get(&file) {
            self.current_file_id = id;
            id
        } else {
            let id = self.next_file_id;
            self.next_file_id += 1;
            self.file_name_to_id.insert(file, id);
            self.current_file_id = id;
            id
        }
    }

    /// Set source text for debug output (extracts line context on type conflicts)
    pub fn set_debug_source(&mut self, source: &str) {
        self.debug_source = Some(source.to_string());
    }

    /// Keys for `function_signatures` lookup on `Call` callees (`bar`, `Foo::new`).
    fn call_signature_lookup_keys<'ast>(function: &Expression<'ast>) -> Vec<String> {
        match function {
            Expression::FieldAccess { object, field, .. } => {
                if let Expression::Identifier {
                    name: type_name, ..
                } = *object
                {
                    vec![format!("{}::{}", type_name, field)]
                } else {
                    Vec::new()
                }
            }
            Expression::Identifier { name, .. } => vec![name.clone()],
            _ => Vec::new(),
        }
    }

    /// TDD FIX: Pre-populate function signatures for cross-file float inference
    /// Used during multi-file library builds to share function signatures
    pub fn set_global_function_signatures(
        &mut self,
        signatures: HashMap<String, (Vec<Type>, Option<Type>)>,
    ) {
        self.function_signatures = signatures;
    }

    /// TDD FIX: Get all collected function signatures (for building global registry)
    pub fn get_function_signatures(&self) -> &HashMap<String, (Vec<Type>, Option<Type>)> {
        &self.function_signatures
    }

    /// Set source root for resolving metadata file paths
    pub fn set_source_root(&mut self, path: &std::path::Path) {
        self.source_root = Some(path.to_path_buf());
    }

    /// Pre-populate struct field types from other modules in the same project
    /// Used when compiling multi-file projects - structs from already-compiled files
    pub fn set_global_struct_field_types(
        &mut self,
        field_types: &HashMap<String, HashMap<String, Type>>,
    ) {
        for (struct_name, fields) in field_types {
            self.struct_field_types
                .insert(struct_name.clone(), fields.clone());
        }
    }

    pub fn set_current_file_module_path(&mut self, path: Vec<String>) {
        self.current_file_module_path = path;
    }

    pub fn set_struct_defining_module_paths(&mut self, paths: HashMap<String, Vec<Vec<String>>>) {
        self.struct_defining_module_paths = paths;
    }

    pub fn set_module_re_exports(&mut self, re_exports: HashMap<String, HashMap<String, String>>) {
        self.module_re_exports = re_exports;
    }

    /// Register external crate metadata paths for cross-crate type inference.
    /// When loading imports like `use mylib::vec3::Vec3`, loads metadata.json
    /// from the given path to get Vec3's field types (e.g., x: f32).
    pub fn set_external_crate_metadata_paths(
        &mut self,
        paths: &std::collections::HashMap<String, std::path::PathBuf>,
    ) {
        self.external_crate_metadata_paths = paths.clone();
    }

    /// Main entry point: Infer float types for a program
    pub fn infer_program<'ast>(&mut self, program: &Program<'ast>) {
        self.imported_type_registry_keys.clear();
        // TDD FIX: Extract file from program's source locations for file-aware ExprIds
        if let Some(first_item) = program.items.first() {
            if let Some(loc) = first_item.location() {
                self.set_current_file(loc.file.to_string_lossy().to_string());
            }
        }

        let file_prefix = self.current_file_module_path.clone();
        for item in &program.items {
            self.register_struct_fields_for_module(item, &file_prefix);
            self.register_function_signature(item);
            self.register_const_and_static(item);
        }

        // TDD FIX: Load metadata from imported modules for cross-module inference
        self.load_imported_metadata(program);

        self.register_use_imports_from_items(&program.items);

        // Pass 1: Collect constraints from all expressions
        for item in program.items.iter() {
            self.collect_item_constraints(item);
        }

        // Pass 2: Solve constraints (unification)
        self.solve_constraints();
    }

    fn lookup_struct_fields(&self, type_name: &str) -> Option<&HashMap<String, Type>> {
        struct_field_registry::lookup_struct_field_map(
            &self.struct_field_types,
            type_name,
            &self.imported_type_registry_keys,
            &self.struct_defining_module_paths,
        )
    }

    /// Resolve struct fields for `self` inside the current `impl TypeName`.
    ///
    /// When multiple structs share the same basename (e.g. `CharacterController` in different
    /// modules), unqualified `lookup_struct_fields` returns `None` (ambiguous). The defining
    /// module for this file + nested `mod` path matches how structs are registered, so we try
    /// `qualify_struct_key(current_file_module_path, TypeName)` first.
    fn lookup_struct_fields_for_impl_type(
        &self,
        impl_type_basename: &str,
    ) -> Option<&HashMap<String, Type>> {
        let base = if let Some(idx) = impl_type_basename.find('<') {
            &impl_type_basename[..idx]
        } else {
            impl_type_basename
        };
        if !self.current_file_module_path.is_empty() {
            let k = struct_field_registry::qualify_struct_key(&self.current_file_module_path, base);
            if let Some(m) = self.struct_field_types.get(&k) {
                return Some(m);
            }
        }
        self.lookup_struct_fields(base)
    }
}

include!("struct_registration.rs");
include!("load_imported_metadata.rs");
include!("function_registration.rs");
include!("item_and_statement_constraints.rs");
include!("expression_constraints_binary.rs");
include!("expression_constraints_method_call.rs");
include!("expression_constraints_call.rs");
include!("expression_constraints.rs");
include!("type_helpers.rs");
include!("solve_constraints.rs");

impl FloatInference {
    #[cfg(test)]
    pub(crate) fn test_must_be_f32_sites(&self) -> Vec<(usize, usize)> {
        self.constraints
            .iter()
            .filter_map(|c| {
                if let Constraint::MustBeF32(id, _) = c {
                    Some((id.line, id.col))
                } else {
                    None
                }
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn test_must_be_f32_reasons(&self) -> Vec<String> {
        self.constraints
            .iter()
            .filter_map(|c| {
                if let Constraint::MustBeF32(_, r) = c {
                    Some(r.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get inferred float type for an expression
    pub fn get_float_type<'ast>(&self, expr: &Expression<'ast>) -> FloatType {
        // TDD FIX: Use file-aware cache lookup to prevent cross-file collisions
        let location = expr.location();
        let (file, line, col) = if let Some(loc) = location {
            (loc.file.to_string_lossy().to_string(), loc.line, loc.column)
        } else {
            (String::new(), 0, 0)
        };

        // Map file name to file_id
        let file_id = self.file_name_to_id.get(&file).copied().unwrap_or(0);

        // Priority 1: Direct cache lookup (O(1), uses exact same location logic as constraint collection)
        let cache_key = (file_id, line, col);
        if let Some(&expr_id) = self.expr_id_cache.get(&cache_key) {
            if let Some(&float_type) = self.inferred_types.get(&expr_id) {
                return float_type;
            }
        }

        // Priority 2: Fallback to linear search by file_id+location (for expressions not cached)
        for (expr_id, float_type) in &self.inferred_types {
            if expr_id.file_id == file_id && expr_id.line == line && expr_id.col == col {
                return *float_type;
            }
        }

        // Priority 3: line+column only. Inference records `file_id` from `set_current_file` while
        // codegen may resolve `SourceLocation::file` to a different string (relative vs absolute), so
        // `file_id` can disagree even within a single-file compile. Location within the file is stable.
        if line > 0 {
            for (expr_id, float_type) in &self.inferred_types {
                if expr_id.line == line && expr_id.col == col {
                    return *float_type;
                }
            }
        }

        // Return Unknown when no match - enables fallback to context-sensitive inference
        FloatType::Unknown
    }
}

#[cfg(test)]
mod hashmap_match_float_tests {
    use crate::lexer::Lexer;
    use crate::parser_impl::Parser;
    use crate::type_inference::{FloatInference, FloatType};

    #[test]
    fn match_on_hashmap_get_unifies_default_literal_to_f32() {
        let src = r#"use std::collections::HashMap
fn foo() -> i32 {
    let mut g: HashMap<(i32, i32), f32> = HashMap::new()
    let _x = match g.get(&(0, 0)) {
        Some(v) => *v,
        None => 999999.0,
    }
    0
}
"#;
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let mut fi = FloatInference::new();
        fi.infer_program(&program);
        assert!(
            fi.errors.is_empty(),
            "unexpected float errors: {:?}",
            fi.errors
        );
        let f32_count = fi
            .inferred_types
            .values()
            .filter(|t| **t == FloatType::F32)
            .count();
        assert!(
            f32_count >= 1,
            "expected f32 unification for match on HashMap::get (scrutinee peels to f32); inferred_types={:?}",
            fi.inferred_types
        );
    }

    #[test]
    fn get_float_type_finds_match_arm_literal() {
        use crate::parser::ast::literals::Literal;
        use crate::parser::ast::types::Type;
        use crate::parser::ast::{Expression, Item, Statement};

        let src = r#"use std::collections::HashMap
fn foo(g: HashMap<(i32, i32), f32>) -> i32 {
    let _x = match g.get(&(0, 0)) {
        Some(v) => *v,
        None => 999999.0,
    }
    0
}
"#;
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");
        if let Item::Function { decl, .. } = program
            .items
            .iter()
            .find(|i| matches!(i, Item::Function { .. }))
            .expect("fn")
        {
            assert!(
                matches!(
                    &decl.parameters[0].type_,
                    Type::Parameterized(name, args) if name == "HashMap" && args.len() == 2
                ),
                "param type must be Parameterized HashMap for .get() value inference, got {:?}",
                decl.parameters[0].type_
            );
        }
        let mut fi = FloatInference::new();
        fi.infer_program(&program);
        let f32_sites = fi.test_must_be_f32_sites();

        let mut lit_999: Option<&Expression> = None;
        for item in &program.items {
            let Item::Function { decl, .. } = item else {
                continue;
            };
            for stmt in &decl.body {
                let Statement::Let { value, .. } = stmt else {
                    continue;
                };
                let Expression::Block { statements, .. } = value else {
                    continue;
                };
                for inner in statements {
                    let Statement::Match { arms, .. } = inner else {
                        continue;
                    };
                    for arm in arms {
                        if let Expression::Literal {
                            value: Literal::Float(f),
                            ..
                        } = arm.body
                        {
                            if (*f - 999999.0).abs() < 1e-6 {
                                lit_999 = Some(arm.body);
                                break;
                            }
                        }
                    }
                }
            }
        }
        let lit_999 = lit_999.expect("999999.0 arm literal");
        let loc = lit_999.location();
        assert!(
            f32_sites.iter().any(|(l, c)| {
                loc.as_ref()
                    .is_some_and(|loc| loc.line == *l && loc.column == *c)
            }),
            "expected MustBeF32 on match default literal at {:?}, sites={:?}, reasons={:?}",
            loc,
            f32_sites,
            fi.test_must_be_f32_reasons()
        );
        let ft = fi.get_float_type(lit_999);
        assert_eq!(
            ft,
            FloatType::F32,
            "get_float_type for match default literal: loc {:?}, inferred keys sample {:?}",
            lit_999.location(),
            fi.inferred_types
                .iter()
                .filter(|(_, t)| **t == FloatType::F32)
                .take(8)
                .collect::<Vec<_>>()
        );
    }
}
