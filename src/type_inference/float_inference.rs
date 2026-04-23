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

    fn register_struct_fields_for_module<'ast>(
        &mut self,
        item: &Item<'ast>,
        module_prefix: &[String],
    ) {
        match item {
            Item::Struct { decl, .. } => {
                let key = struct_field_registry::qualify_struct_key(module_prefix, &decl.name);
                let mut field_map = HashMap::new();
                for field in &decl.fields {
                    field_map.insert(field.name.clone(), field.field_type.clone());
                }
                self.struct_field_types.insert(key, field_map);
            }
            Item::Mod { name, items, .. } => {
                let mut next = module_prefix.to_vec();
                next.push(name.clone());
                for sub_item in items {
                    self.register_struct_fields_for_module(sub_item, &next);
                }
            }
            _ => {}
        }
    }

    fn register_use_imports_from_items<'ast>(&mut self, items: &[Item<'ast>]) {
        for item in items {
            match item {
                Item::Use { path, alias, .. } => {
                    if path.len() == 1 && path[0].contains("::{") {
                        struct_field_registry::register_braced_use_imports(
                            &path[0],
                            &self.current_file_module_path,
                            &self.struct_field_types,
                            &self.struct_defining_module_paths,
                            &mut self.imported_type_registry_keys,
                        );
                        continue;
                    }
                    if path.last().map(|s| s.as_str()) == Some("*") {
                        struct_field_registry::expand_glob_import(
                            path,
                            &self.current_file_module_path,
                            &self.struct_field_types,
                            &self.struct_defining_module_paths,
                            &self.module_re_exports,
                            &mut self.imported_type_registry_keys,
                        );
                        continue;
                    }
                    if path.len() < 2 {
                        continue;
                    }
                    if let Some(key) = struct_field_registry::resolve_use_path_to_qualified_key(
                        path,
                        &self.current_file_module_path,
                        &self.struct_field_types,
                        &self.struct_defining_module_paths,
                    ) {
                        let imported_name = alias
                            .clone()
                            .unwrap_or_else(|| path.last().cloned().unwrap_or_default());
                        self.imported_type_registry_keys.insert(imported_name, key);
                    }
                }
                Item::Mod { items, .. } => self.register_use_imports_from_items(items),
                _ => {}
            }
        }
    }

    /// TDD FIX: Load function signatures from imported modules' metadata files
    fn load_imported_metadata<'ast>(&mut self, program: &Program<'ast>) {
        use crate::metadata::{CrateMetadata, ModuleMetadata};
        use std::path::PathBuf;

        for item in &program.items {
            if let Item::Use { path, .. } = item {
                // Convert import path to file path
                // e.g., "crate::math::vec3::Vec3" → "math/vec3.wj.meta" (skip type name!)
                // e.g., "mylib::vec3::Vec3" → external crate, load from metadata.json
                let mut module_path: Vec<String> = path
                    .iter()
                    .skip_while(|s| {
                        s.as_str() == "crate" || s.as_str() == "self" || s.as_str() == "super"
                    })
                    .cloned()
                    .collect();

                if module_path.is_empty() {
                    continue;
                }

                // TDD FIX: Last element is type/function name, not module!
                let type_name = module_path.pop(); // Remove type name (Vec3)

                // CROSS-CRATE: Check for external crate metadata first
                if let (Some(crate_name), Some(ref ty_name)) = (module_path.first(), &type_name) {
                    let crate_key = crate_name.replace('-', "_");
                    if let Some(meta_dir) = self.external_crate_metadata_paths.get(&crate_key) {
                        let metadata_path = meta_dir.join("metadata.json");
                        if let Ok(meta_json) = std::fs::read_to_string(&metadata_path) {
                            if let Ok(crate_meta) =
                                serde_json::from_str::<CrateMetadata>(&meta_json)
                            {
                                // Load struct field types for the imported type
                                if let Some(fields) = crate_meta.structs.get(ty_name) {
                                    let mut field_map = HashMap::new();
                                    for (field_name, type_str) in fields {
                                        if let Some(field_type) =
                                            ModuleMetadata::deserialize_type(type_str)
                                        {
                                            field_map.insert(field_name.clone(), field_type);
                                        }
                                    }
                                    if !field_map.is_empty() {
                                        self.struct_field_types.insert(ty_name.clone(), field_map);
                                    }
                                }
                                // Load function signatures
                                for (func_name, sig) in &crate_meta.functions {
                                    let params: Vec<Type> = sig
                                        .params
                                        .iter()
                                        .filter_map(|s| ModuleMetadata::deserialize_type(s))
                                        .collect();
                                    let return_type = sig
                                        .return_type
                                        .as_ref()
                                        .and_then(|s| ModuleMetadata::deserialize_type(s));
                                    self.function_signatures
                                        .insert(func_name.clone(), (params, return_type));
                                }
                                continue; // Handled, skip .wj.meta lookup
                            }
                        }
                    }
                }

                if module_path.is_empty() {
                    continue; // Import from current module
                }

                // TDD FIX: Handle module re-exports by trying multiple paths
                // Example: "use crate::math::Vec3" could be:
                //   1. math/vec3.wj.meta (type defined in math/vec3.wj)
                //   2. math.wj.meta (type defined in math.wj)
                //   3. math/mod.wj.meta (type re-exported by math/mod.wj)

                // Build candidate paths
                let mut candidates: Vec<PathBuf> = Vec::new();

                if let Some(ref ty_name) = type_name {
                    // Helper: Convert PascalCase to snake_case
                    let snake_case = ty_name.chars().fold(String::new(), |mut acc, c| {
                        if c.is_uppercase() && !acc.is_empty() {
                            acc.push('_');
                        }
                        acc.push(c.to_lowercase().next().unwrap());
                        acc
                    });

                    // Helper: Truncate common suffixes (State, Config, Manager, etc.)
                    let truncated = ty_name
                        .to_lowercase()
                        .trim_end_matches("state")
                        .trim_end_matches("config")
                        .trim_end_matches("manager")
                        .trim_end_matches("system")
                        .to_string();

                    // Candidate 1: math/vec3.wj.meta (lowercase)
                    let mut p1 = PathBuf::new();
                    for seg in &module_path {
                        p1.push(seg);
                    }
                    p1.push(format!("{}.wj.meta", ty_name.to_lowercase()));
                    candidates.push(p1);

                    // Candidate 2: math/vec_3.wj.meta (snake_case)
                    let mut p2 = PathBuf::new();
                    for seg in &module_path {
                        p2.push(seg);
                    }
                    p2.push(format!("{}.wj.meta", snake_case));
                    candidates.push(p2);

                    // Candidate 3: ai/perception.wj.meta (truncated)
                    if !truncated.is_empty() && truncated != ty_name.to_lowercase() {
                        let mut p3 = PathBuf::new();
                        for seg in &module_path {
                            p3.push(seg);
                        }
                        p3.push(format!("{}.wj.meta", truncated));
                        candidates.push(p3);
                    }

                    // Candidate 4: math.wj.meta (mod file)
                    let mut p4 = PathBuf::new();
                    for (i, segment) in module_path.iter().enumerate() {
                        if i < module_path.len() - 1 {
                            p4.push(segment);
                        } else {
                            p4.push(format!("{}.wj.meta", segment));
                        }
                    }
                    candidates.push(p4);
                } else {
                    // No type name, just use module path
                    let mut meta_path = PathBuf::new();
                    for (i, segment) in module_path.iter().enumerate() {
                        if i < module_path.len() - 1 {
                            meta_path.push(segment);
                        } else {
                            meta_path.push(format!("{}.wj.meta", segment));
                        }
                    }
                    candidates.push(meta_path);
                }

                // Try each candidate until we find one that exists.
                // Check .wj-cache/ first, then fall back to colocated (legacy).
                let mut found_metadata = false;
                for candidate in &candidates {
                    let cache_path = if let Some(ref root) = self.source_root {
                        crate::metadata::meta_cache_root(root).join(candidate)
                    } else {
                        std::path::PathBuf::from(".wj-cache").join(candidate)
                    };
                    let legacy_path = if let Some(ref root) = self.source_root {
                        root.join(candidate)
                    } else {
                        candidate.clone()
                    };
                    let full_meta_path = if cache_path.exists() {
                        cache_path
                    } else {
                        legacy_path
                    };

                    if let Ok(meta_json) = std::fs::read_to_string(&full_meta_path) {
                        if let Ok(meta) = serde_json::from_str::<ModuleMetadata>(&meta_json) {
                            // Load all function signatures from metadata
                            for (func_name, sig) in meta.functions {
                                // Convert serialized types back to Type enum
                                let params: Vec<Type> = sig
                                    .params
                                    .iter()
                                    .filter_map(|s| ModuleMetadata::deserialize_type(s))
                                    .collect();

                                let return_type = sig
                                    .return_type
                                    .as_ref()
                                    .and_then(|s| ModuleMetadata::deserialize_type(s));

                                self.function_signatures
                                    .insert(func_name, (params, return_type));
                            }
                            // TDD FIX: Load struct field types for cross-module float inference
                            // Enables LightingConfig { sun_dir_x: -0.5 } to infer f32 from imported struct
                            for (struct_name, fields) in meta.structs {
                                let mut field_map = HashMap::new();
                                for (field_name, type_str) in fields {
                                    if let Some(field_type) =
                                        ModuleMetadata::deserialize_type(&type_str)
                                    {
                                        field_map.insert(field_name, field_type);
                                    }
                                }
                                if !field_map.is_empty() {
                                    self.struct_field_types.insert(struct_name, field_map);
                                }
                            }
                            found_metadata = true;
                            break; // Found and loaded, stop trying candidates
                        }
                    }
                }

                if !found_metadata {}
            }
        }
    }

    /// Register const and static types for constraint propagation
    fn register_const_and_static<'ast>(&mut self, item: &Item<'ast>) {
        match item {
            Item::Const { name, type_, .. } => {
                self.const_types.insert(name.clone(), type_.clone());
            }
            Item::Static { name, type_, .. } => {
                self.const_types.insert(name.clone(), type_.clone());
            }
            Item::Mod { items, .. } => {
                for sub_item in items {
                    self.register_const_and_static(sub_item);
                }
            }
            _ => {}
        }
    }

    /// Register function signatures for constraint propagation
    fn register_function_signature<'ast>(&mut self, item: &Item<'ast>) {
        match item {
            Item::Function { decl, .. } => {
                let param_types: Vec<Type> =
                    decl.parameters.iter().map(|p| p.type_.clone()).collect();

                self.function_signatures
                    .insert(decl.name.clone(), (param_types, decl.return_type.clone()));
            }
            Item::Impl { block, .. } => {
                // Register associated functions (e.g., Vec3::new)
                let type_name = block.type_name.clone();
                for func_decl in &block.functions {
                    let param_types: Vec<Type> = func_decl
                        .parameters
                        .iter()
                        .map(|p| p.type_.clone())
                        .collect();

                    // Register as "TypeName::method_name"
                    let full_name = format!("{}::{}", type_name, func_decl.name);
                    self.function_signatures
                        .insert(full_name, (param_types, func_decl.return_type.clone()));
                }
            }
            _ => {}
        }
    }

    /// Collect constraints from a top-level item
    fn collect_item_constraints<'ast>(&mut self, item: &Item<'ast>) {
        match item {
            Item::Function { decl, .. } => {
                // THE WINDJAMMER WAY: Clear cache for each function to ensure proper scoping
                self.expr_id_cache.clear();

                // TDD FIX: Register function parameters for type tracking
                for param in &decl.parameters {
                    self.var_types
                        .insert(param.name.clone(), param.type_.clone());
                }

                // TDD FIX: Handle implicit returns FIRST (before collecting constraints)
                // This populates var_element_types so that .push()/.insert() can use them
                if let Some(Statement::Expression { expr, .. }) = decl.body.last() {
                    // This is an implicit return - store variable element types
                    if let Some(return_type) = &decl.return_type {
                        self.constrain_expr_to_type(expr, return_type);
                    }
                }

                // Collect return type constraints (now var_element_types is populated)
                for stmt in decl.body.iter() {
                    self.collect_statement_constraints(stmt, decl.return_type.as_ref());
                }

                // Clear parameter types after function (for proper scoping)
                for param in &decl.parameters {
                    self.var_types.remove(&param.name);
                }
            }
            Item::Impl { block, .. } => {
                // TDD FIX: Set current impl type context for `self` field access resolution
                self.current_impl_type = Some(block.type_name.clone());

                // Process methods in impl block
                for func in &block.functions {
                    // THE WINDJAMMER WAY: Clear cache for each method to ensure proper scoping
                    self.expr_id_cache.clear();

                    // TDD FIX: Register function parameters for type tracking
                    for param in &func.parameters {
                        self.var_types
                            .insert(param.name.clone(), param.type_.clone());
                    }

                    // TDD FIX: Handle implicit returns FIRST (before collecting constraints)
                    if let Some(Statement::Expression { expr, .. }) = func.body.last() {
                        if let Some(return_type) = &func.return_type {
                            self.constrain_expr_to_type(expr, return_type);
                        }
                    }

                    for stmt in func.body.iter() {
                        self.collect_statement_constraints(stmt, func.return_type.as_ref());
                    }

                    // Clear parameter types after method (for proper scoping)
                    for param in &func.parameters {
                        self.var_types.remove(&param.name);
                    }
                }

                // Clear impl type context
                self.current_impl_type = None;
            }
            Item::Struct { .. } => {}
            Item::Enum { .. } => {}
            Item::Trait { .. } => {}
            Item::Const {
                name, type_, value, ..
            } => {
                self.collect_expression_constraints(value, None);
                if let Some(float_ty) = self.extract_float_type(type_) {
                    let expr_id = self.get_expr_id(value);
                    match float_ty {
                        FloatType::F32 => {
                            self.constraints.push(Constraint::MustBeF32(
                                expr_id,
                                format!("const {} is f32", name),
                            ));
                        }
                        FloatType::F64 => {
                            self.constraints.push(Constraint::MustBeF64(
                                expr_id,
                                format!("const {} is f64", name),
                            ));
                        }
                        FloatType::Unknown => {}
                    }
                }
            }
            Item::Static {
                name, type_, value, ..
            } => {
                self.collect_expression_constraints(value, None);
                if let Some(float_ty) = self.extract_float_type(type_) {
                    let expr_id = self.get_expr_id(value);
                    match float_ty {
                        FloatType::F32 => {
                            self.constraints.push(Constraint::MustBeF32(
                                expr_id,
                                format!("static {} is f32", name),
                            ));
                        }
                        FloatType::F64 => {
                            self.constraints.push(Constraint::MustBeF64(
                                expr_id,
                                format!("static {} is f64", name),
                            ));
                        }
                        FloatType::Unknown => {}
                    }
                }
            }
            Item::Mod { name, items, .. } => {
                self.current_file_module_path.push(name.clone());
                for sub_item in items {
                    self.collect_item_constraints(sub_item);
                }
                self.current_file_module_path.pop();
            }
            _ => {}
        }
    }

    /// ExprIds for the "value-producing" tail of a statement list, for if/else float unification.
    /// When the last statement is a nested `if`, collects tails from both of its branches so an outer
    /// `if a { 0.0 } else { if b { 1.0 } else { x } }` still ties `0.0` to `1.0` and `x`.
    fn branch_tail_expression_ids<'ast>(&mut self, stmts: &[&'ast Statement<'ast>]) -> Vec<ExprId> {
        let Some(last) = stmts.last().copied() else {
            return Vec::new();
        };
        match last {
            Statement::Expression { expr, .. } => vec![self.get_expr_id(expr)],
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                let mut ids = self.branch_tail_expression_ids(then_block);
                if let Some(else_b) = else_block {
                    ids.extend(self.branch_tail_expression_ids(else_b));
                }
                ids
            }
            _ => Vec::new(),
        }
    }

    /// Collect constraints from a statement
    fn collect_statement_constraints<'ast>(
        &mut self,
        stmt: &Statement<'ast>,
        return_type: Option<&Type>,
    ) {
        match stmt {
            Statement::Let {
                pattern,
                value,
                type_,
                ..
            } => {
                // TDD FIX: If type annotation exists, constrain value to that type FIRST
                let explicit_type = type_.as_ref().and_then(|ty| self.extract_float_type(ty));

                // TDD: `let x: f32 = match ...` must pass f32 into the match (fn may return void).
                // Explicit non-float types (`let m: HashMap<...> = ...`) must NOT inherit the function's
                // return type as float context — that breaks `match m.get(..)` arm unification.
                let value_float_ctx: Option<&Type> = match type_.as_ref() {
                    Some(ty) if self.extract_float_type(ty).is_some() => Some(ty),
                    Some(_) => None,
                    None => return_type,
                };
                self.collect_expression_constraints(value, value_float_ctx);

                // Track variable assignment for constraint propagation
                if let crate::parser::ast::core::Pattern::Identifier(var_name) = pattern {
                    let value_id = self.get_expr_id(value);
                    self.var_assignments.insert(var_name.clone(), value_id);

                    // TDD FIX: Track explicit type annotations (let x: Type = ...)
                    if let Some(ty) = type_ {
                        self.var_types.insert(var_name.clone(), ty.clone());
                    } else if let Some(inferred_ty) = self.infer_type_from_expression(value) {
                        // TDD FIX: Infer variable type for assert_eq!(transform.x, 15.0)
                        // Enables FieldAccess to resolve struct type → field type → MustBeF32
                        self.var_types.insert(var_name.clone(), inferred_ty);
                    }

                    // TDD FIX: Constrain value expression to match explicit type annotation
                    if let Some(float_ty) = explicit_type {
                        let expr_id = self.get_expr_id(value);
                        match float_ty {
                            FloatType::F32 => {
                                self.constraints.push(Constraint::MustBeF32(
                                    expr_id,
                                    format!("let {} has explicit type f32", var_name),
                                ));
                            }
                            FloatType::F64 => {
                                self.constraints.push(Constraint::MustBeF64(
                                    expr_id,
                                    format!("let {} has explicit type f64", var_name),
                                ));
                            }
                            FloatType::Unknown => {
                                // No constraint needed
                            }
                        }
                    }
                }

                // If this expression might be returned (in a function returning a float),
                // constrain it to the return type
                if let Some(ret_ty) = return_type {
                    if let Some(float_ty) = self.extract_float_type(ret_ty) {
                        let expr_id = self.get_expr_id(value);
                        let constraint = match float_ty {
                            FloatType::F32 => Constraint::MustBeF32(
                                expr_id,
                                "function return type f32".to_string(),
                            ),
                            FloatType::F64 => Constraint::MustBeF64(
                                expr_id,
                                "function return type f64".to_string(),
                            ),
                            FloatType::Unknown => return,
                        };
                        self.constraints.push(constraint);
                    }
                }
            }
            Statement::Expression { expr, .. } => {
                self.collect_expression_constraints(expr, return_type);

                // Implicit return: last expression in function body
                if let Some(ret_ty) = return_type {
                    if let Some(float_ty) = self.extract_float_type(ret_ty) {
                        let expr_id = self.get_expr_id(expr);
                        let constraint = match float_ty {
                            FloatType::F32 => {
                                Constraint::MustBeF32(expr_id, "implicit return f32".to_string())
                            }
                            FloatType::F64 => {
                                Constraint::MustBeF64(expr_id, "implicit return f64".to_string())
                            }
                            FloatType::Unknown => return,
                        };
                        self.constraints.push(constraint);
                    }
                }
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                self.collect_expression_constraints(expr, return_type);

                // Return expression must match function return type
                if let Some(ret_ty) = return_type {
                    if let Some(float_ty) = self.extract_float_type(ret_ty) {
                        let expr_id = self.get_expr_id(expr);
                        let constraint = match float_ty {
                            FloatType::F32 => {
                                Constraint::MustBeF32(expr_id, "return type".to_string())
                            }
                            FloatType::F64 => {
                                Constraint::MustBeF64(expr_id, "return type".to_string())
                            }
                            FloatType::Unknown => return,
                        };
                        self.constraints.push(constraint);
                    }
                }
            }
            Statement::Return { value: None, .. } => {
                // Empty return, nothing to constrain
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                // THE WINDJAMMER WAY: if-else branches that return floats must match return type
                self.collect_expression_constraints(condition, return_type);
                for stmt in then_block {
                    self.collect_statement_constraints(stmt, return_type);
                }
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.collect_statement_constraints(stmt, return_type);
                    }
                }
                // Unify float types across then/else tails, including when else ends with nested if
                // (e.g. blend_tree: `if factor < 0.0 { 0.0 } else { if factor > 1.0 { 1.0 } else { factor } }`).
                let then_tails = self.branch_tail_expression_ids(then_block);
                if let Some(else_stmts) = else_block {
                    let else_tails = self.branch_tail_expression_ids(else_stmts);
                    for &t_id in &then_tails {
                        for &e_id in &else_tails {
                            self.constraints.push(Constraint::MustMatch(
                                t_id,
                                e_id,
                                "if/else branches must have same float type".to_string(),
                            ));
                        }
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                // TDD FIX: Traverse while loop body to find float literals in struct fields
                self.collect_expression_constraints(condition, return_type);
                for stmt in body {
                    self.collect_statement_constraints(stmt, return_type);
                }
            }
            Statement::For { iterable, body, .. } => {
                // TDD FIX: Traverse for loop body (same as While loop)
                self.collect_expression_constraints(iterable, return_type);
                for stmt in body {
                    self.collect_statement_constraints(stmt, return_type);
                }
            }
            Statement::Assignment { target, value, .. } => {
                // TDD FIX: Handle assignments (e.g., self.vy = self.vy * 0.5)
                // Traverse both target and value to collect constraints
                self.collect_expression_constraints(target, return_type);
                self.collect_expression_constraints(value, return_type);

                // THE WINDJAMMER WAY: Target and value must match types
                // For `self.vy = self.vy * 0.5`, this ensures the literal matches the field type
                let target_id = self.get_expr_id(target);
                let value_id = self.get_expr_id(value);
                self.constraints.push(Constraint::MustMatch(
                    target_id,
                    value_id,
                    "assignment target and value".to_string(),
                ));

                // TDD: Direct RHS constraint from inferred LHS type (field, index, deref, etc.).
                // Belt-and-suspenders when MustMatch/unification misses (qualified struct keys, edge cases).
                if let Some(lhs_ty) = self.infer_type_from_expression(target) {
                    if let Some(float_ty) = self.extract_float_type(&lhs_ty) {
                        match float_ty {
                            FloatType::F32 => {
                                self.constraints.push(Constraint::MustBeF32(
                                    value_id,
                                    "assignment RHS matches LHS float type".to_string(),
                                ));
                            }
                            FloatType::F64 => {
                                self.constraints.push(Constraint::MustBeF64(
                                    value_id,
                                    "assignment RHS matches LHS float type".to_string(),
                                ));
                            }
                            FloatType::Unknown => {}
                        }
                    }
                }
            }
            Statement::Match { value, arms, .. } => {
                // THE WINDJAMMER WAY: Match arms must have compatible types AND match return type
                self.collect_expression_constraints(value, return_type);

                // TDD: When the function does not return f32/f64 (e.g. void), still unify arms using
                // the scrutinee (`Option<f32>`, `Result<f32, _>`, `&f32`, etc.).
                let arm_context: Option<Type> = if let Some(rt) = return_type {
                    if self.extract_float_type(rt).is_some() {
                        Some(rt.clone())
                    } else {
                        self.infer_type_from_expression(value)
                            .and_then(|ty| self.float_type_after_peeling_wrappers(ty))
                    }
                } else {
                    self.infer_type_from_expression(value)
                        .and_then(|ty| self.float_type_after_peeling_wrappers(ty))
                };
                // Never fall back to non-float return types (e.g. `-> i32`): that would pass a bogus
                // context into arm bodies and block scrutinee-only float inference for `match map.get(..)`.
                let arm_ctx_ref: Option<&Type> = arm_context
                    .as_ref()
                    .or_else(|| return_type.filter(|rt| self.extract_float_type(rt).is_some()));

                // Traverse all arms to collect constraints
                for (i, arm) in arms.iter().enumerate() {
                    self.collect_expression_constraints(arm.body, arm_ctx_ref);

                    // TDD FIX: Constrain arm to return type if function returns float
                    if let Some(ret_ty) = arm_ctx_ref {
                        if let Some(float_ty) = self.extract_float_type(ret_ty) {
                            let arm_id = self.get_expr_id(arm.body);
                            let constraint = match float_ty {
                                FloatType::F32 => Constraint::MustBeF32(
                                    arm_id,
                                    format!("match arm {} float context", i),
                                ),
                                FloatType::F64 => Constraint::MustBeF64(
                                    arm_id,
                                    format!("match arm {} float context", i),
                                ),
                                FloatType::Unknown => continue,
                            };
                            self.constraints.push(constraint);
                        }
                    }

                    if let Some(guard) = arm.guard {
                        self.collect_expression_constraints(guard, arm_ctx_ref);
                    }
                }

                // TDD FIX: All match arms must have the same type
                if arms.len() > 1 {
                    for i in 0..arms.len() - 1 {
                        let id1 = self.get_expr_id(arms[i].body);
                        let id2 = self.get_expr_id(arms[i + 1].body);
                        self.constraints.push(Constraint::MustMatch(
                            id1,
                            id2,
                            format!("match arms {} and {}", i, i + 1),
                        ));
                    }
                }
            }
            _other => {}
        }
    }

    /// Collect constraints from an expression
    fn collect_expression_constraints<'ast>(
        &mut self,
        expr: &Expression<'ast>,
        return_type: Option<&Type>,
    ) {
        match expr {
            Expression::Identifier { name, .. } => {
                // TDD FIX: Constrain identifier to its declared type (function param, let with type annotation, const)
                let var_type = self
                    .var_types
                    .get(name)
                    .or_else(|| self.const_types.get(name));
                if let Some(var_type) = var_type {
                    if let Some(float_ty) = self.extract_float_type(var_type) {
                        let id = self.get_expr_id(expr);
                        match float_ty {
                            FloatType::F32 => {
                                self.constraints.push(Constraint::MustBeF32(
                                    id,
                                    format!("identifier {} has type f32", name),
                                ));
                            }
                            FloatType::F64 => {
                                self.constraints.push(Constraint::MustBeF64(
                                    id,
                                    format!("identifier {} has type f64", name),
                                ));
                            }
                            FloatType::Unknown => {}
                        }
                    }
                }

                // TDD FIX: Variable assignment type propagation
                // When a variable is used (e.g., "det" in "1.0 / det"),
                // link it to its assigned value so type propagates through
                // Example: let det = a * b; let inv_det = 1.0 / det
                //   -> det must match (a * b) -> 1.0 must match det -> 1.0 matches a's type
                if let Some(&value_id) = self.var_assignments.get(name.as_str()) {
                    let identifier_id = self.get_expr_id(expr);
                    self.constraints.push(Constraint::MustMatch(
                        identifier_id,
                        value_id,
                        format!("variable {} matches its assigned value", name),
                    ));
                }
            }
            Expression::Binary {
                left, right, op, ..
            } => {
                // Binary ops require both operands to have same type
                self.collect_expression_constraints(left, return_type);
                self.collect_expression_constraints(right, return_type);

                // For arithmetic ops (+, -, *, /), operands must match
                use crate::parser::ast::operators::BinaryOp;
                match op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod => {
                        let left_id = self.get_expr_id(left);
                        let right_id = self.get_expr_id(right);

                        let binary_id = self.get_expr_id(expr);
                        self.constraints.push(Constraint::MustMatch(
                            left_id,
                            right_id,
                            format!("binary operation {:?}", op),
                        ));
                        // TDD FIX: Link binary result to operands so if/else MustMatch propagates
                        // e.g. 1.0/dt has type f32 → binary result gets f32 → else 0.0 gets f32
                        self.constraints.push(Constraint::MustMatch(
                            left_id,
                            binary_id,
                            "binary result matches LHS".to_string(),
                        ));

                        // TDD FIX: Direct propagation from function params to literals
                        // When LHS is Identifier with explicit float type (e.g. dt: f32), constrain RHS directly.
                        // Fixes dt * 1000.0 when dt: f32 - ensures 1000.0 gets f32 in multi-file builds.
                        if let Expression::Identifier { name, .. } = left {
                            if let Some(var_type) = self
                                .var_types
                                .get(name)
                                .or_else(|| self.const_types.get(name))
                            {
                                if let Some(float_ty) = self.extract_float_type(var_type) {
                                    match float_ty {
                                        FloatType::F32 => {
                                            self.constraints.push(Constraint::MustBeF32(
                                                right_id,
                                                format!("binary op RHS: {} has type f32", name),
                                            ));
                                        }
                                        FloatType::F64 => {
                                            self.constraints.push(Constraint::MustBeF64(
                                                right_id,
                                                format!("binary op RHS: {} has type f64", name),
                                            ));
                                        }
                                        FloatType::Unknown => {}
                                    }
                                }
                            }
                        }
                        // Same for RHS identifier (e.g. 1000.0 * dt)
                        if let Expression::Identifier { name, .. } = right {
                            if let Some(var_type) = self
                                .var_types
                                .get(name)
                                .or_else(|| self.const_types.get(name))
                            {
                                if let Some(float_ty) = self.extract_float_type(var_type) {
                                    match float_ty {
                                        FloatType::F32 => {
                                            self.constraints.push(Constraint::MustBeF32(
                                                left_id,
                                                format!("binary op LHS: {} has type f32", name),
                                            ));
                                        }
                                        FloatType::F64 => {
                                            self.constraints.push(Constraint::MustBeF64(
                                                left_id,
                                                format!("binary op LHS: {} has type f64", name),
                                            ));
                                        }
                                        FloatType::Unknown => {}
                                    }
                                }
                            }
                        }

                        // TDD FIX: Backward propagation - when either operand is a variable with float
                        // literal initializer, add DIRECT constraint from the typed operand to the
                        // literal. This ensures `self.player.position.x + offset_x` propagates f32
                        // to `let offset_x = 0.0`. Without this, multi-file builds may fail.
                        if let Expression::Identifier { name, .. } = right {
                            if let Some(&value_id) = self.var_assignments.get(name.as_str()) {
                                self.constraints.push(Constraint::MustMatch(
                                    left_id,
                                    value_id,
                                    format!("backward propagation: {} used with typed LHS", name),
                                ));
                            }
                        }
                        if let Expression::Identifier { name, .. } = left {
                            if let Some(&value_id) = self.var_assignments.get(name.as_str()) {
                                self.constraints.push(Constraint::MustMatch(
                                    right_id,
                                    value_id,
                                    format!("backward propagation: {} used with typed RHS", name),
                                ));
                            }
                        }

                        // Recursively constrain nested float literals
                        self.constrain_nested_floats(left, return_type);
                        self.constrain_nested_floats(right, return_type);
                    }
                    // THE WINDJAMMER WAY: Comparison ops also need matching operands
                    BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge => {
                        let left_id = self.get_expr_id(left);
                        let right_id = self.get_expr_id(right);
                        self.constraints.push(Constraint::MustMatch(
                            left_id,
                            right_id,
                            format!("comparison {:?} operands", op),
                        ));
                        // TDD FIX: Direct propagation from params to literals in comparisons (dt > 0.0)
                        if let Expression::Identifier { name, .. } = left {
                            if let Some(var_type) = self
                                .var_types
                                .get(name)
                                .or_else(|| self.const_types.get(name))
                            {
                                if let Some(float_ty) = self.extract_float_type(var_type) {
                                    match float_ty {
                                        FloatType::F32 => {
                                            self.constraints.push(Constraint::MustBeF32(
                                                right_id,
                                                format!("comparison RHS: {} has type f32", name),
                                            ));
                                        }
                                        FloatType::F64 => {
                                            self.constraints.push(Constraint::MustBeF64(
                                                right_id,
                                                format!("comparison RHS: {} has type f64", name),
                                            ));
                                        }
                                        FloatType::Unknown => {}
                                    }
                                }
                            }
                        }
                        if let Expression::Identifier { name, .. } = right {
                            if let Some(var_type) = self
                                .var_types
                                .get(name)
                                .or_else(|| self.const_types.get(name))
                            {
                                if let Some(float_ty) = self.extract_float_type(var_type) {
                                    match float_ty {
                                        FloatType::F32 => {
                                            self.constraints.push(Constraint::MustBeF32(
                                                left_id,
                                                format!("comparison LHS: {} has type f32", name),
                                            ));
                                        }
                                        FloatType::F64 => {
                                            self.constraints.push(Constraint::MustBeF64(
                                                left_id,
                                                format!("comparison LHS: {} has type f64", name),
                                            ));
                                        }
                                        FloatType::Unknown => {}
                                    }
                                }
                            }
                        }
                        // Backward propagation for comparison: variable + typed operand
                        if let Expression::Identifier { name, .. } = right {
                            if let Some(&value_id) = self.var_assignments.get(name.as_str()) {
                                self.constraints.push(Constraint::MustMatch(
                                    left_id,
                                    value_id,
                                    format!("backward propagation: {} in comparison", name),
                                ));
                            }
                        }
                        if let Expression::Identifier { name, .. } = left {
                            if let Some(&value_id) = self.var_assignments.get(name.as_str()) {
                                self.constraints.push(Constraint::MustMatch(
                                    right_id,
                                    value_id,
                                    format!("backward propagation: {} in comparison", name),
                                ));
                            }
                        }
                    }
                    _ => {
                        // Logical ops (&&, ||) don't constrain float types
                    }
                }
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                // Method call: infer argument types from method signature
                self.collect_expression_constraints(object, return_type);

                // TDD FIX: Constrain MethodCall expression to its return type
                // This is critical for binary ops: `t.sin() * 0.8` needs the MethodCall
                // to be constrained to f32, which then propagates through MustMatch to the literal

                // First, try to determine the method's return type
                let method_return_type = self.determine_method_return_type(object, method);

                if let Some(float_ty) = method_return_type {
                    let method_call_id = self.get_expr_id(expr);
                    match float_ty {
                        FloatType::F32 => {
                            self.constraints.push(Constraint::MustBeF32(
                                method_call_id,
                                format!("method {} returns f32", method),
                            ));
                        }
                        FloatType::F64 => {
                            self.constraints.push(Constraint::MustBeF64(
                                method_call_id,
                                format!("method {} returns f64", method),
                            ));
                        }
                        FloatType::Unknown => {}
                    }
                }

                // TDD FIX: .min() and .max() methods - argument must match receiver type
                // Pattern: (self.level + amount).min(100.0) - the 100.0 must be f32
                if (method == "min" || method == "max") && arguments.len() == 1 {
                    let receiver_id = self.get_expr_id(object);
                    let arg_id = self.get_expr_id(arguments[0].1);

                    // Receiver and argument must be same type
                    self.constraints.push(Constraint::MustMatch(
                        receiver_id,
                        arg_id,
                        format!(".{}() argument must match receiver type", method),
                    ));
                }

                // TDD FIX: Method calls - constrain args from method signature (metadata)
                // Handles both: self.field.method(...) and local_var.method(...) e.g. voxelizer.voxelize(...)
                let method_sig = self
                    .function_signatures
                    .iter()
                    .filter(|(func_name, (params, _))| {
                        // Match method name and param count
                        // Instance method: params.len() == arguments.len() + 1 (Self)
                        // Associated fn (Type::new): params.len() == arguments.len()
                        let name_match = func_name.split("::").last() == Some(method.as_str());
                        let param_match =
                            params.len() == arguments.len() + 1 || params.len() == arguments.len();
                        name_match && param_match
                    })
                    .map(|(_, (params, ret))| (params.clone(), ret.clone()))
                    .next();

                if let Some((param_types, _)) = method_sig {
                    // Found a matching method! Constrain arguments
                    // Instance method: skip index 0 (Self); associated fn: use index i
                    let param_offset = if param_types.len() == arguments.len() + 1 {
                        1
                    } else {
                        0
                    };
                    for (i, (_label, arg)) in arguments.iter().enumerate() {
                        if let Some(param_type) = param_types.get(i + param_offset) {
                            if let Some(float_ty) = self.extract_float_type(param_type) {
                                let arg_id = self.get_expr_id(arg);
                                match float_ty {
                                    FloatType::F32 => {
                                        self.constraints.push(Constraint::MustBeF32(
                                            arg_id,
                                            format!("{}() parameter {}", method, i),
                                        ));
                                    }
                                    FloatType::F64 => {
                                        self.constraints.push(Constraint::MustBeF64(
                                            arg_id,
                                            format!("{}() parameter {}", method, i),
                                        ));
                                    }
                                    FloatType::Unknown => {}
                                }
                            }
                        }
                    }
                }

                // TDD FIX: HashMap.insert(K, V) and Vec.push(T) - constrain arguments to collection element type
                if let Expression::Identifier { name, .. } = object {
                    if let Some(var_type) = self.var_types.get(name).cloned() {
                        // HashMap<K, V>.insert(K, V) - constrain second argument to V
                        if method == "insert" {
                            if let Some(value_type) = self.extract_hashmap_value_type(&var_type) {
                                if let Some(float_ty) = self.extract_float_type(&value_type) {
                                    if arguments.len() >= 2 {
                                        let value_arg = arguments[1].1;
                                        let value_id = self.get_expr_id(value_arg);
                                        match float_ty {
                                            FloatType::F32 => {
                                                self.constraints.push(Constraint::MustBeF32(
                                                    value_id,
                                                    "HashMap<K, f32>.insert(K, f32)".to_string(),
                                                ));
                                            }
                                            FloatType::F64 => {
                                                self.constraints.push(Constraint::MustBeF64(
                                                    value_id,
                                                    "HashMap<K, f64>.insert(K, f64)".to_string(),
                                                ));
                                            }
                                            FloatType::Unknown => {}
                                        }
                                    }
                                }
                            }
                        }

                        // Vec<T>.push(T) - constrain first argument to T
                        if method == "push" {
                            if let Some(elem_type) = self.extract_vec_element_type(&var_type) {
                                if let Some(float_ty) = self.extract_float_type(&elem_type) {
                                    if !arguments.is_empty() {
                                        let value_arg = arguments[0].1;
                                        let value_id = self.get_expr_id(value_arg);
                                        match float_ty {
                                            FloatType::F32 => {
                                                self.constraints.push(Constraint::MustBeF32(
                                                    value_id,
                                                    "Vec<f32>.push(f32)".to_string(),
                                                ));
                                            }
                                            FloatType::F64 => {
                                                self.constraints.push(Constraint::MustBeF64(
                                                    value_id,
                                                    "Vec<f64>.push(f64)".to_string(),
                                                ));
                                            }
                                            FloatType::Unknown => {}
                                        }
                                    }
                                }
                            }
                        }
                    // TDD FIX: Also check var_element_types for inferred collection types
                    } else if let Some(elem_type) = self.var_element_types.get(name).cloned() {
                        if method == "push" {
                            if !arguments.is_empty() {
                                let value_arg = arguments[0].1;

                                // TDD FIX: Recursively constrain with the element type
                                // This handles both simple types (f32) and complex types (Tuple)
                                self.collect_expression_constraints(value_arg, Some(&elem_type));
                            }
                        } else if method == "insert" && arguments.len() >= 2 {
                            let value_arg = arguments[1].1;
                            // TDD FIX: Recursively constrain with the value type
                            self.collect_expression_constraints(value_arg, Some(&elem_type));
                        }
                    }
                }

                // Recurse into ALL arguments to collect binary op constraints
                // This ensures that nested expressions like (x, y, method() * 1.414) are visited
                for (_label, arg) in arguments {
                    self.collect_expression_constraints(arg, return_type);
                }
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // `g.get(k)` is Call(FieldAccess, ..): constrain call like MethodCall when receiver is a map.
                if let Expression::FieldAccess { object, field, .. } = function {
                    if field == "get" {
                        if let Some(float_ty) = self.map_receiver_value_float_type(object) {
                            let call_id = self.get_expr_id(expr);
                            match float_ty {
                                FloatType::F32 => {
                                    self.constraints.push(Constraint::MustBeF32(
                                        call_id,
                                        "map get optional value is f32".to_string(),
                                    ));
                                }
                                FloatType::F64 => {
                                    self.constraints.push(Constraint::MustBeF64(
                                        call_id,
                                        "map get optional value is f64".to_string(),
                                    ));
                                }
                                FloatType::Unknown => {}
                            }
                        }
                    }
                }

                // `recv.method(args)` may parse as Call(FieldAccess(recv, method), args).
                // Apply the same float return constraints as MethodCall.
                if let Expression::FieldAccess { object, field, .. } = function {
                    if let Some(float_ty) =
                        self.determine_method_return_type(object, field.as_str())
                    {
                        let call_id = self.get_expr_id(expr);
                        match float_ty {
                            FloatType::F32 => {
                                self.constraints.push(Constraint::MustBeF32(
                                    call_id,
                                    format!("call {} returns f32", field),
                                ));
                            }
                            FloatType::F64 => {
                                self.constraints.push(Constraint::MustBeF64(
                                    call_id,
                                    format!("call {} returns f64", field),
                                ));
                            }
                            FloatType::Unknown => {}
                        }
                    }
                }

                // Look up function signature and constrain arguments
                self.collect_expression_constraints(function, return_type);

                let lookup_keys = Self::call_signature_lookup_keys(function);
                let func_sig = lookup_keys
                    .iter()
                    .find_map(|k| self.function_signatures.get(k).cloned());
                let func_name = lookup_keys
                    .iter()
                    .find(|k| self.function_signatures.contains_key(*k))
                    .cloned()
                    .or_else(|| lookup_keys.first().cloned());

                // TDD FIX: assert_eq/assert_ne (when written as Call, not MacroInvocation)
                // Both args must have same type - e.g. assert_eq(transform.x, 15.0)
                if let Some(ref name) = func_name {
                    if (name == "assert_eq" || name == "assert_ne") && arguments.len() >= 2 {
                        for (_label, arg) in arguments.iter() {
                            self.collect_expression_constraints(arg, return_type);
                        }
                        let first_id = self.get_expr_id(arguments[0].1);
                        let second_id = self.get_expr_id(arguments[1].1);
                        self.constraints.push(Constraint::MustMatch(
                            first_id,
                            second_id,
                            format!("{} requires both arguments to have same type", name),
                        ));
                        return; // Already recursed
                    }
                }

                if let Some((param_types, _)) = func_sig {
                    // Match arguments to parameters
                    let label = func_name.unwrap_or_else(|| "function".to_string());
                    for (i, (_label, arg)) in arguments.iter().enumerate() {
                        self.collect_expression_constraints(arg, return_type);

                        if let Some(param_type) = param_types.get(i) {
                            if let Some(float_ty) = self.extract_float_type(param_type) {
                                let arg_id = self.get_expr_id(arg);
                                let constraint = match float_ty {
                                    FloatType::F32 => Constraint::MustBeF32(
                                        arg_id,
                                        format!("parameter {} of {}", i, label),
                                    ),
                                    FloatType::F64 => Constraint::MustBeF64(
                                        arg_id,
                                        format!("parameter {} of {}", i, label),
                                    ),
                                    FloatType::Unknown => continue,
                                };
                                self.constraints.push(constraint);
                            }
                        }
                    }
                } else {
                    // Not a simple identifier or not found - still collect from arguments
                    for (_label, arg) in arguments {
                        self.collect_expression_constraints(arg, return_type);
                    }
                }
            }
            Expression::StructLiteral { name, fields, .. } => {
                // THE WINDJAMMER WAY: Constrain struct field expressions to match field types

                // Collect field type constraints (two-phase to avoid borrow checker issues)
                let field_constraints: Vec<(String, &'ast Expression<'ast>, FloatType)> =
                    if let Some(struct_fields) = self.lookup_struct_fields(name) {
                        fields
                            .iter()
                            .filter_map(|(field_name, field_expr)| {
                                if let Some(field_type) = struct_fields.get(field_name) {
                                    self.extract_float_type(field_type)
                                        .map(|float_ty| (field_name.clone(), *field_expr, float_ty))
                                } else {
                                    None
                                }
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };

                // Now create constraints with mutable access
                for (field_name, field_expr, float_ty) in field_constraints {
                    let expr_id = self.get_expr_id(field_expr);
                    let constraint = match float_ty {
                        FloatType::F32 => Constraint::MustBeF32(
                            expr_id,
                            format!("struct {}.{} is f32", name, field_name),
                        ),
                        FloatType::F64 => Constraint::MustBeF64(
                            expr_id,
                            format!("struct {}.{} is f64", name, field_name),
                        ),
                        FloatType::Unknown => continue,
                    };
                    self.constraints.push(constraint);
                }

                // Recursively collect constraints from field expressions
                for (_field_name, expr) in fields {
                    self.collect_expression_constraints(expr, return_type);
                }
            }
            Expression::Tuple { elements, .. } => {
                // Tuple expression: match elements with return type positions
                if let Some(Type::Tuple(tuple_types)) = return_type {
                    for (i, elem) in elements.iter().enumerate() {
                        if let Some(elem_type) = tuple_types.get(i) {
                            // Recurse with the specific type for this position
                            self.collect_expression_constraints(elem, Some(elem_type));

                            // If this position is a float type, constrain the element
                            if let Some(float_ty) = self.extract_float_type(elem_type) {
                                let elem_id = self.get_expr_id(elem);
                                let constraint = match float_ty {
                                    FloatType::F32 => Constraint::MustBeF32(
                                        elem_id,
                                        format!("tuple element {}", i),
                                    ),
                                    FloatType::F64 => Constraint::MustBeF64(
                                        elem_id,
                                        format!("tuple element {}", i),
                                    ),
                                    FloatType::Unknown => continue,
                                };
                                self.constraints.push(constraint);

                                // If element is an identifier, also constrain its assigned value
                                if let Expression::Identifier { name, .. } = elem {
                                    if let Some(&value_id) = self.var_assignments.get(name.as_str())
                                    {
                                        let value_constraint = match float_ty {
                                            FloatType::F32 => Constraint::MustBeF32(
                                                value_id,
                                                format!("variable {} assigned value", name),
                                            ),
                                            FloatType::F64 => Constraint::MustBeF64(
                                                value_id,
                                                format!("variable {} assigned value", name),
                                            ),
                                            FloatType::Unknown => continue,
                                        };
                                        self.constraints.push(value_constraint);
                                    }
                                }
                            }
                        } else {
                            // No type info for this position, recurse without constraint
                            self.collect_expression_constraints(elem, None);
                        }
                    }
                } else {
                    // No tuple type info, just recurse
                    for elem in elements {
                        self.collect_expression_constraints(elem, None);
                    }
                }
            }
            Expression::Cast {
                expr: inner, type_, ..
            } => {
                // Cast expression provides explicit type constraint
                self.collect_expression_constraints(inner, return_type);

                if let Some(float_ty) = self.extract_float_type(type_) {
                    let cast_id = self.get_expr_id(expr);
                    let cast_constraint = match float_ty {
                        FloatType::F32 => {
                            Constraint::MustBeF32(cast_id, "cast expression result f32".to_string())
                        }
                        FloatType::F64 => {
                            Constraint::MustBeF64(cast_id, "cast expression result f64".to_string())
                        }
                        FloatType::Unknown => return,
                    };
                    self.constraints.push(cast_constraint);

                    let inner_id = self.get_expr_id(inner);
                    let constraint = match float_ty {
                        FloatType::F32 => {
                            Constraint::MustBeF32(inner_id, "cast to f32".to_string())
                        }
                        FloatType::F64 => {
                            Constraint::MustBeF64(inner_id, "cast to f64".to_string())
                        }
                        FloatType::Unknown => return,
                    };
                    self.constraints.push(constraint);
                }
            }
            Expression::Block { statements, .. } => {
                // TDD FIX: Traverse block expressions (e.g., let x = { match ... })
                // Match statements inside blocks weren't being visited!
                for stmt in statements {
                    self.collect_statement_constraints(stmt, return_type);
                }
                // TDD FIX: Constrain block result to return_type when block is used as value
                // For `x = if c { a } else { b }`, the block wraps the If; constrain both branches
                if let Some(last_stmt) = statements.last() {
                    let block_id = self.get_expr_id(expr);
                    if let Statement::If {
                        then_block,
                        else_block,
                        ..
                    } = last_stmt
                    {
                        if let (Some(then_last), Some(else_stmts)) = (then_block.last(), else_block)
                        {
                            if let Some(else_last) = else_stmts.last() {
                                if let (
                                    Statement::Expression { expr: te, .. },
                                    Statement::Expression { expr: ee, .. },
                                ) = (then_last, else_last)
                                {
                                    let then_id = self.get_expr_id(te);
                                    let else_id = self.get_expr_id(ee);
                                    self.constraints.push(Constraint::MustMatch(
                                        block_id,
                                        then_id,
                                        "block result from if then".to_string(),
                                    ));
                                    self.constraints.push(Constraint::MustMatch(
                                        block_id,
                                        else_id,
                                        "block result from if else".to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Expression::FieldAccess { object, field, .. } => {
                // TDD FIX: Constrain field access to field's type
                // e.g., self.vy (f32) or self.cell_size (f32) should constrain the FieldAccess expression to f32

                // First, get the ID for this FieldAccess expression (before mutating self)
                let field_access_id = self.get_expr_id(expr);

                // Recurse into object
                self.collect_expression_constraints(object, return_type);

                // Determine the struct type that contains this field
                let struct_name: Option<String> =
                    if let Expression::Identifier { name, .. } = object {
                        if name == "self" {
                            // Case 1: self.field - use current impl type context
                            self.current_impl_type.clone()
                        } else {
                            // Case 2: variable.field - look up variable's type
                            self.var_types
                                .get(name)
                                .and_then(|var_type| match var_type {
                                    Type::Custom(name) => Some(name.clone()),
                                    _ => None,
                                })
                        }
                    } else {
                        // TDD FIX: Chained FieldAccess - self.player.position.x
                        // Use infer_type_from_expression to resolve object's type
                        self.infer_type_from_expression(object)
                            .and_then(|obj_ty| match &obj_ty {
                                Type::Custom(name) => Some(name.clone()),
                                _ => None,
                            })
                    };

                // If we know the struct type, constrain the FieldAccess expression to the field's type
                if let Some(struct_name) = struct_name {
                    let field_map = if matches!(
                        *object,
                        Expression::Identifier { ref name, .. } if name == "self"
                    ) {
                        self.current_impl_type
                            .as_deref()
                            .and_then(|ty| self.lookup_struct_fields_for_impl_type(ty))
                    } else {
                        self.lookup_struct_fields(&struct_name)
                    };

                    if let Some(field_types) = field_map {
                        if let Some(field_type) = field_types.get(field) {
                            if let Some(float_ty) = self.extract_float_type(field_type) {
                                // TDD FIX: Constrain the FieldAccess expression itself, not the object!
                                // This is critical for binary ops: `self.vy * 0.5` needs the entire
                                // FieldAccess expression to be constrained, which then propagates
                                // through MustMatch to the literal
                                match float_ty {
                                    FloatType::F32 => {
                                        self.constraints.push(Constraint::MustBeF32(
                                            field_access_id,
                                            format!("{}.{} is f32", struct_name, field),
                                        ));
                                    }
                                    FloatType::F64 => {
                                        self.constraints.push(Constraint::MustBeF64(
                                            field_access_id,
                                            format!("{}.{} is f64", struct_name, field),
                                        ));
                                    }
                                    FloatType::Unknown => {}
                                }
                            }
                        }
                    }
                }
            }
            Expression::Index { object, index, .. } => {
                // TDD FIX: Index expression (arr[i]) yields element type - constrain for binary ops
                // e.g., arr[i] / 2.0 when arr: Vec<f32> → Index is f32, so 2.0 must be f32
                self.collect_expression_constraints(object, return_type);
                self.collect_expression_constraints(index, return_type);

                if let Some(elem_type) = self
                    .infer_type_from_expression(object)
                    .and_then(|ty| self.extract_vec_element_type(&ty))
                {
                    if let Some(float_ty) = self.extract_float_type(&elem_type) {
                        let index_expr_id = self.get_expr_id(expr);
                        match float_ty {
                            FloatType::F32 => {
                                self.constraints.push(Constraint::MustBeF32(
                                    index_expr_id,
                                    "Index yields Vec<f32> element".to_string(),
                                ));
                            }
                            FloatType::F64 => {
                                self.constraints.push(Constraint::MustBeF64(
                                    index_expr_id,
                                    "Index yields Vec<f64> element".to_string(),
                                ));
                            }
                            FloatType::Unknown => {}
                        }
                    }
                }
            }
            Expression::Unary { op, operand, .. } => {
                use crate::parser::ast::operators::UnaryOp;
                self.collect_expression_constraints(operand, return_type);
                if matches!(op, UnaryOp::Deref) {
                    let unary_id = self.get_expr_id(expr);
                    let operand_id = self.get_expr_id(operand);
                    self.constraints.push(Constraint::MustMatch(
                        unary_id,
                        operand_id,
                        "dereference has same float type as pointee".to_string(),
                    ));
                }
            }
            Expression::Literal { value, .. } => {
                // TDD FIX: Constrain float literals to return type
                if matches!(value, crate::parser::ast::literals::Literal::Float(_)) {
                    if let Some(float_ty) = return_type.and_then(|rt| self.extract_float_type(rt)) {
                        let expr_id = self.get_expr_id(expr);
                        let constraint = match float_ty {
                            FloatType::F32 => Constraint::MustBeF32(
                                expr_id,
                                "literal constrained by return type".to_string(),
                            ),
                            FloatType::F64 => Constraint::MustBeF64(
                                expr_id,
                                "literal constrained by return type".to_string(),
                            ),
                            FloatType::Unknown => return,
                        };
                        self.constraints.push(constraint);
                    }
                }
            }
            Expression::MacroInvocation { name, args, .. } => {
                // TDD FIX: assert_eq!/assert_ne! - both args must have same type
                // assert_eq!(transform.x, 15.0) → 15.0 should infer f32 from transform.x
                if (name == "assert_eq" || name == "assert_ne") && args.len() >= 2 {
                    // Recurse into args first (FieldAccess adds MustBeF32 for transform.x)
                    for arg in args {
                        self.collect_expression_constraints(arg, return_type);
                    }
                    // Link first and second arg: same type required
                    let first_id = self.get_expr_id(args[0]);
                    let second_id = self.get_expr_id(args[1]);
                    self.constraints.push(Constraint::MustMatch(
                        first_id,
                        second_id,
                        format!("{}! requires both arguments to have same type", name),
                    ));
                    return; // Already recursed
                }
                // Other macros (format!, vec!, etc.): just recurse into args
                for arg in args {
                    self.collect_expression_constraints(arg, return_type);
                }
            }
            _ => {}
        }
    }

    /// Infer Type from an expression (for variable type tracking)
    /// Used when let x = expr has no explicit type - infer from expr for assert_eq!(x.field, literal)
    /// TDD FIX: Added Binary and MethodCall fallback for len > 0.0 pattern (physics/advanced_collision.wj)
    fn infer_type_from_expression<'ast>(&self, expr: &Expression<'ast>) -> Option<Type> {
        match expr {
            Expression::StructLiteral { name, .. } => Some(Type::Custom(name.clone())),
            Expression::Binary {
                left, right, op, ..
            } => {
                use crate::parser::ast::operators::BinaryOp;
                if matches!(
                    op,
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
                ) {
                    let left_ty = self.infer_type_from_expression(left)?;
                    let right_ty = self.infer_type_from_expression(right)?;
                    if left_ty == right_ty {
                        return Some(left_ty);
                    }
                }
                None
            }
            // TDD: `let x = (n as f32) / (m as f32)` must record x as f32 so `x < 0.3` constrains the literal.
            Expression::Cast { type_, .. } => {
                self.extract_float_type(type_).and_then(|ft| match ft {
                    FloatType::F32 => Some(Type::Custom("f32".to_string())),
                    FloatType::F64 => Some(Type::Custom("f64".to_string())),
                    FloatType::Unknown => None,
                })
            }
            Expression::Call { function, .. } => {
                // Parser desugars `receiver.method(args)` as Call(FieldAccess(receiver, method), args).
                // HashMap/Map `.get` must infer like MethodCall so `match m.get(..)` gets arm float context.
                if let Expression::FieldAccess { object, field, .. } = function {
                    if field == "get" {
                        if let Some(object_type) = self.infer_type_from_expression(object) {
                            if let Some(value_ty) = self.extract_map_value_type(&object_type) {
                                return Some(Type::Option(Box::new(value_ty)));
                            }
                        }
                    }
                }
                // Type::new() or Type::method() - get return type from function signature
                let func_name = match function {
                    Expression::FieldAccess { object, field, .. } => {
                        if let Expression::Identifier {
                            name: type_name, ..
                        } = object
                        {
                            Some(format!("{}::{}", type_name, field))
                        } else {
                            None
                        }
                    }
                    Expression::Identifier { name, .. } => Some(name.clone()),
                    _ => None,
                };
                func_name.and_then(|name| {
                    self.function_signatures
                        .get(&name)
                        .and_then(|(_, ret)| ret.clone())
                })
            }
            Expression::MethodCall { object, method, .. } => {
                // object.method() - need object's type to find method signature
                let object_type = self.infer_type_from_expression(object)?;
                // TDD: Map<K,V>::get / HashMap::get → Option<V> (match arms need float context)
                if method == "get" {
                    if let Some(value_ty) = self.extract_map_value_type(&object_type) {
                        return Some(Type::Option(Box::new(value_ty)));
                    }
                }
                let type_name = match &object_type {
                    Type::Custom(name) => name.clone(),
                    _ => return None,
                };
                let full_name = format!("{}::{}", type_name, method);
                if let Some((_, ret)) = self.function_signatures.get(&full_name) {
                    return ret.clone();
                }
                // TDD FIX: Fallback for primitive methods — return same float type as receiver.
                // Keep in sync with `determine_method_return_type` F32_METHODS (subset used here).
                const PRIMITIVE_SAME_TYPE: &[&str] = &[
                    "sqrt",
                    "sin",
                    "cos",
                    "tan",
                    "asin",
                    "acos",
                    "atan",
                    "atan2",
                    "abs",
                    "floor",
                    "ceil",
                    "round",
                    "length",
                    "magnitude",
                    "distance",
                    "dot",
                    "recip",
                    "powf",
                    "powi",
                    "exp",
                    "ln",
                    "log",
                    "log2",
                    "to_degrees",
                    "to_radians",
                ];
                if PRIMITIVE_SAME_TYPE.contains(&method.as_str())
                    && (matches!(object_type, Type::Custom(ref s) if s == "f32" || s == "f64")
                        || matches!(object_type, Type::Float))
                {
                    return Some(object_type);
                }
                None
            }
            Expression::Identifier { name, .. } => {
                if name == "self" {
                    self.current_impl_type
                        .as_ref()
                        .map(|s| Type::Custom(s.clone()))
                } else {
                    self.var_types
                        .get(name)
                        .or_else(|| self.const_types.get(name))
                        .cloned()
                }
            }
            Expression::FieldAccess { object, field, .. } => {
                let object_type = self.infer_type_from_expression(object)?;
                let struct_name = match &object_type {
                    Type::Custom(name) => name.clone(),
                    _ => return None,
                };
                let base_name = if let Some(idx) = struct_name.find('<') {
                    &struct_name[..idx]
                } else {
                    struct_name.as_str()
                };
                let fields = if matches!(
                    *object,
                    Expression::Identifier { ref name, .. } if name == "self"
                ) {
                    self.current_impl_type
                        .as_deref()
                        .and_then(|ty| self.lookup_struct_fields_for_impl_type(ty))
                } else {
                    self.lookup_struct_fields(base_name)
                };
                fields.and_then(|m| m.get(field)).cloned()
            }
            Expression::Index { object, .. } => {
                let object_type = self.infer_type_from_expression(object)?;
                self.extract_vec_element_type(&object_type)
            }
            Expression::Unary { op, operand, .. } => {
                use crate::parser::ast::operators::UnaryOp;
                if matches!(op, UnaryOp::Deref) {
                    self.infer_type_from_expression(operand)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Extract FloatType from a Type
    fn extract_float_type(&self, ty: &Type) -> Option<FloatType> {
        match ty {
            Type::Float => Some(FloatType::F64), // Windjammer "float" keyword → f64
            Type::Custom(name) if name == "f32" => Some(FloatType::F32),
            Type::Custom(name) if name == "f64" => Some(FloatType::F64),
            Type::Tuple(types) => {
                // Search tuple for float types
                for t in types {
                    if let Some(float_ty) = self.extract_float_type(t) {
                        return Some(float_ty);
                    }
                }
                None
            }
            Type::Vec(inner) => self.extract_float_type(inner),
            Type::Array(inner, _) => self.extract_float_type(inner),
            Type::Parameterized(name, type_args) => {
                let base = crate::type_inference::generic_type_base_name(name);
                if base == "Vec" && !type_args.is_empty() {
                    self.extract_float_type(&type_args[0])
                } else {
                    // HashMap/Map/BTreeMap etc.: not scalar floats — do not recurse into `V`.
                    None
                }
            }
            _ => None,
        }
    }

    /// Value type `V` from `HashMap<K, V>` or Windjammer `Map<K, V>`.
    fn extract_map_value_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Parameterized(name, type_args) => {
                let base = crate::type_inference::generic_type_base_name(name);
                if matches!(base, "HashMap" | "Map" | "BTreeMap") && type_args.len() >= 2 {
                    Some(type_args[1].clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// If `receiver` is map-like and `V` is a scalar float, return that float type.
    fn map_receiver_value_float_type<'ast>(
        &self,
        receiver: &Expression<'ast>,
    ) -> Option<FloatType> {
        let object_type = self.infer_type_from_expression(receiver)?;
        let value_ty = self.extract_map_value_type(&object_type)?;
        self.extract_float_type(&value_ty)
    }

    /// TDD FIX: Extract value type V from HashMap<K, V> (alias for map-like containers)
    fn extract_hashmap_value_type(&self, ty: &Type) -> Option<Type> {
        self.extract_map_value_type(ty)
    }

    /// TDD FIX: Extract element type T from Vec<T>
    fn extract_vec_element_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Vec(inner) => Some((**inner).clone()),
            Type::Parameterized(name, type_args) if name == "Vec" => {
                // Vec<T> has 1 type argument
                if !type_args.is_empty() {
                    Some(type_args[0].clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Peel `Option` / `Result` / references until we reach a concrete type, then keep it if it is f32/f64.
    fn float_type_after_peeling_wrappers(&self, mut ty: Type) -> Option<Type> {
        loop {
            match ty {
                Type::Option(inner) => ty = (*inner).clone(),
                Type::Result(ok, _) => ty = (*ok).clone(),
                Type::Reference(inner) | Type::MutableReference(inner) => ty = (*inner).clone(),
                Type::Parameterized(name, ref args) if name == "Option" && args.len() == 1 => {
                    ty = args[0].clone();
                }
                Type::Parameterized(name, ref args) if name == "Result" && !args.is_empty() => {
                    ty = args[0].clone();
                }
                _ => break,
            }
        }
        if self.extract_float_type(&ty).is_some() {
            Some(ty)
        } else {
            None
        }
    }

    /// Get unique ID for an expression (based on source location)
    /// Get unique ID for expression with location-based caching
    /// THE WINDJAMMER WAY: Cache by location to ensure same expression = same ID
    /// This fixes the problem where same expression got multiple IDs during traversal
    fn get_expr_id<'ast>(&mut self, expr: &Expression<'ast>) -> ExprId {
        let location = expr.location();
        let (line, col) = if let Some(loc) = location {
            (loc.line, loc.column)
        } else {
            (0, 0)
        };

        // TDD FIX: Use file-aware cache key to prevent cross-file collisions
        let cache_key = (self.current_file_id, line, col);

        // Check cache first - if we've seen this location before, return same ID
        if line > 0 {
            // Only cache expressions with valid locations
            if let Some(&cached_id) = self.expr_id_cache.get(&cache_key) {
                return cached_id;
            }
        }

        // Generate new sequential ID (globally unique across all files)
        let seq_id = self.next_seq_id;
        self.next_seq_id += 1;

        let expr_id = ExprId {
            seq_id,
            file_id: self.current_file_id,
            line,
            col,
        };

        // Cache it for future lookups
        if line > 0 {
            self.expr_id_cache.insert(cache_key, expr_id);
        }

        expr_id
    }

    /// Determine the return type of a method call
    /// Returns Some(FloatType) if the method is known to return f32/f64, None otherwise
    fn determine_method_return_type(&self, object: &Expression, method: &str) -> Option<FloatType> {
        // TDD FIX: For methods on f32/f64 primitives, return the same type
        // Standard library f32 methods that return f32:
        const F32_METHODS: &[&str] = &[
            // Trigonometric
            "sin",
            "cos",
            "tan",
            "asin",
            "acos",
            "atan",
            "atan2",
            "sinh",
            "cosh",
            "tanh",
            "asinh",
            "acosh",
            "atanh",
            // Exponential/logarithmic
            "exp",
            "exp2",
            "exp_m1",
            "ln",
            "log",
            "log2",
            "log10",
            "ln_1p",
            // Power/root
            "sqrt",
            "cbrt",
            "hypot",
            "powf",
            "powi",
            // Rounding
            "floor",
            "ceil",
            "round",
            "trunc",
            "fract",
            // Absolute/sign
            "abs",
            "signum",
            "copysign",
            // Min/max
            "max",
            "min",
            "clamp",
            // Misc
            "recip",
            "to_degrees",
            "to_radians",
        ];

        // Check if this is a method call on an identifier
        if let Expression::Identifier { name, .. } = object {
            // Look up the identifier's type from var_types
            if let Some(var_type) = self.var_types.get(name) {
                // Check if it's an f32 or f64 type
                let is_f32 = matches!(var_type, Type::Float)
                    || matches!(var_type, Type::Custom(s) if s == "f32");
                let is_f64 = matches!(var_type, Type::Custom(s) if s == "f64");

                if is_f32 && F32_METHODS.contains(&method) {
                    return Some(FloatType::F32);
                }
                if is_f64 && F32_METHODS.contains(&method) {
                    return Some(FloatType::F64);
                }
            }
        }

        // Method on a field (e.g. self.vy.sqrt(), pos.x.acos()): infer receiver type from the
        // field-access expression. Do NOT scan function_signatures by method basename — HashMap
        // order could pick `f64::acos` while the receiver is f32, forcing spurious f64 promotion.
        if let Expression::FieldAccess { .. } = object {
            if let Some(ty) = self.infer_type_from_expression(object) {
                let is_f32 =
                    matches!(&ty, Type::Float) || matches!(&ty, Type::Custom(s) if s == "f32");
                let is_f64 = matches!(&ty, Type::Custom(s) if s == "f64");
                if is_f32 && F32_METHODS.contains(&method) {
                    return Some(FloatType::F32);
                }
                if is_f64 && F32_METHODS.contains(&method) {
                    return Some(FloatType::F64);
                }
            }
        }

        // For MethodCall on MethodCall (chaining), try to infer from the inner call
        if let Expression::MethodCall { .. } = object {
            // Check if the method is a known f32-returning method
            if F32_METHODS.contains(&method) {
                // Assume it returns the same type as the input (common for math methods)
                // This is a heuristic - ideally we'd recursively determine the type
                return Some(FloatType::F32);
            }
        }

        // TDD FIX: For MethodCall on Binary (e.g., (x*x + y*y).sqrt()), infer from operands
        // Handles physics/advanced_collision.wj: len = (edge_x*edge_x + edge_y*edge_y).sqrt()
        if let Expression::Binary { .. } = object {
            if F32_METHODS.contains(&method) {
                if let Some(object_type) = self.infer_type_from_expression(object) {
                    return self.extract_float_type(&object_type);
                }
            }
        }

        None
    }

    /// Solve constraints using unification
    /// THE WINDJAMMER WAY: Process explicit type constraints (MustBeF32/MustBeF64) before
    /// MustMatch to prevent propagation from unconstrained literals (default f64) from
    /// overwriting parameter types (e.g., dt: f32). Fixes multi-file float type conflicts.
    fn solve_constraints(&mut self) {
        // Pass 0: Establish explicit types first (params, const, struct fields)
        // This prevents MustMatch from propagating f64 from untyped literals to typed params
        for constraint in &self.constraints {
            match constraint {
                Constraint::MustBeF32(expr_id, _) => {
                    if !self.inferred_types.contains_key(expr_id) {
                        self.inferred_types.insert(*expr_id, FloatType::F32);
                    }
                }
                Constraint::MustBeF64(expr_id, _) => {
                    if !self.inferred_types.contains_key(expr_id) {
                        self.inferred_types.insert(*expr_id, FloatType::F64);
                    }
                }
                Constraint::MustMatch(_, _, _) => {}
            }
        }

        // Pass 1+: Unification with conflict detection
        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 100;

        while changed && iterations < MAX_ITERATIONS {
            changed = false;
            iterations += 1;

            for constraint in self.constraints.clone() {
                match constraint {
                    Constraint::MustBeF32(expr_id, reason) => {
                        // THE WINDJAMMER WAY: Always insert if missing, check conflicts if present
                        let current = self.inferred_types.get(&expr_id).copied();
                        match current {
                            Some(FloatType::F64) => {
                                if std::env::var("WJ_DEBUG_FLOAT_CONFLICT").is_ok() {
                                    let source_text = self
                                        .debug_source
                                        .as_ref()
                                        .and_then(|s| s.lines().nth(expr_id.line.saturating_sub(1)))
                                        .unwrap_or("")
                                        .trim();
                                    eprintln!(
                                        "DEBUG: Type conflict at expr seq_id={} ({}:{})",
                                        expr_id.seq_id, expr_id.line, expr_id.col
                                    );
                                    eprintln!("  Current type: F64");
                                    eprintln!("  Required type: F32 ({})", reason);
                                    eprintln!("  Source line {}: {}", expr_id.line, source_text);
                                }
                                self.errors.push(format!(
                                    "Type conflict at seq_id={}, {}:{}: must be f32 ({}) but was inferred as f64",
                                    expr_id.seq_id, expr_id.line, expr_id.col, reason
                                ));
                            }
                            Some(FloatType::F32) => {
                                // Already F32, no change needed
                            }
                            Some(FloatType::Unknown) => {
                                // Unknown -> F32
                                self.inferred_types.insert(expr_id, FloatType::F32);
                                changed = true;
                            }
                            None => {
                                // Not yet inferred, insert f32
                                self.inferred_types.insert(expr_id, FloatType::F32);
                                changed = true;
                            }
                        }
                    }
                    Constraint::MustBeF64(expr_id, reason) => {
                        // THE WINDJAMMER WAY: Always insert if missing, check conflicts if present
                        let current = self.inferred_types.get(&expr_id).copied();
                        match current {
                            Some(FloatType::F32) => {
                                if std::env::var("WJ_DEBUG_FLOAT_CONFLICT").is_ok() {
                                    let source_text = self
                                        .debug_source
                                        .as_ref()
                                        .and_then(|s| s.lines().nth(expr_id.line.saturating_sub(1)))
                                        .unwrap_or("")
                                        .trim();
                                    eprintln!(
                                        "DEBUG: Type conflict at expr seq_id={} ({}:{})",
                                        expr_id.seq_id, expr_id.line, expr_id.col
                                    );
                                    eprintln!("  Current type: F32");
                                    eprintln!("  Required type: F64 ({})", reason);
                                    eprintln!("  Source line {}: {}", expr_id.line, source_text);
                                }
                                self.errors.push(format!(
                                    "Type conflict at seq_id={}, {}:{}: must be f64 ({}) but was inferred as f32",
                                    expr_id.seq_id, expr_id.line, expr_id.col, reason
                                ));
                            }
                            Some(FloatType::F64) => {
                                // Already F64, no change needed
                            }
                            Some(FloatType::Unknown) => {
                                // Unknown -> F64
                                self.inferred_types.insert(expr_id, FloatType::F64);
                                changed = true;
                            }
                            None => {
                                // Not yet inferred, insert f64
                                self.inferred_types.insert(expr_id, FloatType::F64);
                                changed = true;
                            }
                        }
                    }
                    Constraint::MustMatch(id1, id2, reason) => {
                        let ty1 = self.inferred_types.get(&id1).copied();
                        let ty2 = self.inferred_types.get(&id2).copied();

                        match (ty1, ty2) {
                            // Conflict: f32 vs f64
                            (Some(FloatType::F32), Some(FloatType::F64))
                            | (Some(FloatType::F64), Some(FloatType::F32)) => {
                                self.errors.push(format!(
                                    "Type mismatch at {:?} and {:?}: {} requires same float type",
                                    id1, id2, reason
                                ));
                            }
                            // Propagate f32 to unknown or untyped
                            (Some(FloatType::F32), Some(FloatType::Unknown))
                            | (Some(FloatType::F32), None) => {
                                self.inferred_types.insert(id2, FloatType::F32);
                                changed = true;
                            }
                            (Some(FloatType::Unknown), Some(FloatType::F32))
                            | (None, Some(FloatType::F32)) => {
                                self.inferred_types.insert(id1, FloatType::F32);
                                changed = true;
                            }
                            // Propagate f64 to unknown or untyped
                            (Some(FloatType::F64), Some(FloatType::Unknown))
                            | (Some(FloatType::F64), None) => {
                                self.inferred_types.insert(id2, FloatType::F64);
                                changed = true;
                            }
                            (Some(FloatType::Unknown), Some(FloatType::F64))
                            | (None, Some(FloatType::F64)) => {
                                self.inferred_types.insert(id1, FloatType::F64);
                                changed = true;
                            }
                            // Both same concrete type - no change
                            (Some(FloatType::F32), Some(FloatType::F32))
                            | (Some(FloatType::F64), Some(FloatType::F64)) => {}
                            // Both unknown or untyped - wait for more constraints
                            _ => {}
                        }
                    }
                }
            }
        }

        // THE WINDJAMMER WAY: If no new changes occurred, we've converged successfully
        // Only error if we're still changing after max iterations (true infinite loop)
        if iterations >= MAX_ITERATIONS && changed {
            self.errors.push(format!(
                "Type inference did not converge after {} iterations",
                MAX_ITERATIONS
            ));
        }
    }

    /// Recursively constrain float literals in nested expressions
    /// Used for binary ops like: x * y * 0.5 (all must match)
    fn constrain_nested_floats<'ast>(
        &mut self,
        expr: &Expression<'ast>,
        return_type: Option<&Type>,
    ) {
        match expr {
            Expression::Binary {
                left, right, op, ..
            } => {
                use crate::parser::ast::operators::BinaryOp;
                match op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod => {
                        let left_id = self.get_expr_id(left);
                        let right_id = self.get_expr_id(right);
                        self.constraints.push(Constraint::MustMatch(
                            left_id,
                            right_id,
                            "nested binary operation".to_string(),
                        ));

                        // Recurse deeper
                        self.constrain_nested_floats(left, return_type);
                        self.constrain_nested_floats(right, return_type);
                    }
                    _ => {}
                }
            }
            Expression::Literal { .. } => {
                // Base case: literal found
            }
            Expression::Cast {
                expr: inner, type_, ..
            } => {
                // Cast expression provides explicit type hint
                if let Some(float_ty) = self.extract_float_type(type_) {
                    let inner_id = self.get_expr_id(inner);
                    let constraint = match float_ty {
                        FloatType::F32 => {
                            Constraint::MustBeF32(inner_id, "cast to f32".to_string())
                        }
                        FloatType::F64 => {
                            Constraint::MustBeF64(inner_id, "cast to f64".to_string())
                        }
                        FloatType::Unknown => return,
                    };
                    self.constraints.push(constraint);
                }
                self.constrain_nested_floats(inner, return_type);
            }
            _ => {}
        }
    }

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

    /// TDD FIX: Constrain an expression to match a specific type
    /// Used for implicit returns: return type → variable type → collection elements
    fn constrain_expr_to_type<'ast>(&mut self, expr: &Expression<'ast>, target_type: &Type) {
        if let Expression::Identifier { name, .. } = expr {
            // Variable being returned - store its element type if it's a collection
            match target_type {
                Type::Vec(inner) => {
                    // Vec<T>
                    self.var_element_types
                        .insert(name.clone(), (**inner).clone());
                }
                Type::Parameterized(type_name, parameters) => {
                    // Vec<T>, HashMap<K,V>, Option<T>, etc.
                    let base = crate::type_inference::generic_type_base_name(type_name);
                    if base == "Vec" && parameters.len() == 1 {
                        self.var_element_types
                            .insert(name.clone(), parameters[0].clone());
                    } else if matches!(base, "HashMap" | "Map" | "BTreeMap")
                        && parameters.len() == 2
                    {
                        // Store the value type (second parameter)
                        self.var_element_types
                            .insert(name.clone(), parameters[1].clone());
                    }
                }
                _ => {}
            }
        }
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
