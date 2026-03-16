/// Float Type Inference Engine
///
/// Tracks constraints for float literals and unifies them across expressions.

use crate::parser::ast::core::{Expression, Statement, Item};
use crate::parser::ast::types::Type;
use crate::parser::Program;
use std::collections::HashMap;

/// Unique identifier for an expression
/// THE WINDJAMMER WAY: Sequential IDs ensure uniqueness even when expressions lack locations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprId {
    /// Sequential ID assigned during AST traversal (guaranteed unique)
    pub seq_id: usize,
    /// Optional source location for debugging (may be None or duplicate)
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
    /// Key: (line, col), Value: the first ExprId assigned to that location
    expr_id_cache: HashMap<(usize, usize), ExprId>,
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
        }
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
                .entry(struct_name.clone())
                .or_default()
                .extend(fields.clone());
        }
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
        // Pass 0: Build struct field registry, function signatures, and const types
        for item in &program.items {
            self.register_struct_fields(item);
            self.register_function_signature(item);
            self.register_const_and_static(item);
        }
        
        // TDD FIX: Load metadata from imported modules for cross-module inference
        self.load_imported_metadata(program);
        
        // Pass 1: Collect constraints from all expressions
        for (_i, item) in program.items.iter().enumerate() {
            self.collect_item_constraints(item);
        }

        // Pass 2: Solve constraints (unification)
        self.solve_constraints();
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
                let mut module_path: Vec<String> = path.iter()
                    .skip_while(|s| s.as_str() == "crate" || s.as_str() == "self" || s.as_str() == "super")
                    .map(|s| s.clone())
                    .collect();
                
                
                if module_path.is_empty() {
                    continue;
                }
                
                // TDD FIX: Last element is type/function name, not module!
                let type_name = module_path.pop(); // Remove type name (Vec3)
                
                // CROSS-CRATE: Check for external crate metadata first
                if let (Some(ref crate_name), Some(ref ty_name)) = (module_path.first(), &type_name) {
                    let crate_key = crate_name.replace('-', "_");
                    if let Some(meta_dir) = self.external_crate_metadata_paths.get(&crate_key) {
                        let metadata_path = meta_dir.join("metadata.json");
                        if let Ok(meta_json) = std::fs::read_to_string(&metadata_path) {
                            if let Ok(crate_meta) = serde_json::from_str::<CrateMetadata>(&meta_json) {
                                // Load struct field types for the imported type
                                if let Some(fields) = crate_meta.structs.get(ty_name) {
                                    let mut field_map = HashMap::new();
                                    for (field_name, type_str) in fields {
                                        if let Some(field_type) = ModuleMetadata::deserialize_type(type_str) {
                                            field_map.insert(field_name.clone(), field_type);
                                        }
                                    }
                                    if !field_map.is_empty() {
                                        self.struct_field_types.insert(ty_name.clone(), field_map);
                                    }
                                }
                                // Load function signatures
                                for (func_name, sig) in &crate_meta.functions {
                                    let params: Vec<Type> = sig.params.iter()
                                        .filter_map(|s| ModuleMetadata::deserialize_type(s))
                                        .collect();
                                    let return_type = sig.return_type
                                        .as_ref()
                                        .and_then(|s| ModuleMetadata::deserialize_type(s));
                                    self.function_signatures.insert(func_name.clone(), (params, return_type));
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
                    let truncated = ty_name.to_lowercase()
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
                
                
                // Try each candidate until we find one that exists
                let mut found_metadata = false;
                for candidate in &candidates {
                    let full_meta_path = if let Some(ref root) = self.source_root {
                        root.join(candidate)
                    } else {
                        candidate.clone()
                    };
                    
                    
                    if let Ok(meta_json) = std::fs::read_to_string(&full_meta_path) {
                        if let Ok(meta) = serde_json::from_str::<ModuleMetadata>(&meta_json) {
                            // Load all function signatures from metadata
                            for (func_name, sig) in meta.functions {
                                // Convert serialized types back to Type enum
                                let params: Vec<Type> = sig.params.iter()
                                    .filter_map(|s| ModuleMetadata::deserialize_type(s))
                                    .collect();
                                
                                let return_type = sig.return_type
                                    .as_ref()
                                    .and_then(|s| ModuleMetadata::deserialize_type(s));
                                
                                self.function_signatures.insert(func_name, (params, return_type));
                            }
                            // TDD FIX: Load struct field types for cross-module float inference
                            // Enables LightingConfig { sun_dir_x: -0.5 } to infer f32 from imported struct
                            for (struct_name, fields) in meta.structs {
                                let mut field_map = HashMap::new();
                                for (field_name, type_str) in fields {
                                    if let Some(field_type) = ModuleMetadata::deserialize_type(&type_str) {
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
                
                if !found_metadata {
                }
            }
        }
    }

    /// Register struct field types for constraint propagation
    fn register_struct_fields<'ast>(&mut self, item: &Item<'ast>) {
        if let Item::Struct { decl, .. } = item {
            let mut field_map = HashMap::new();
            for field in &decl.fields {
                field_map.insert(field.name.clone(), field.field_type.clone());
            }
            self.struct_field_types.insert(decl.name.clone(), field_map);
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
                let param_types: Vec<Type> = decl
                    .parameters
                    .iter()
                    .map(|p| p.type_.clone())
                    .collect();
                
                self.function_signatures.insert(
                    decl.name.clone(),
                    (param_types, decl.return_type.clone()),
                );
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
                    self.function_signatures.insert(
                        full_name,
                        (param_types, func_decl.return_type.clone()),
                    );
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
                    self.var_types.insert(param.name.clone(), param.type_.clone());
                }
                
                // TDD FIX: Handle implicit returns FIRST (before collecting constraints)
                // This populates var_element_types so that .push()/.insert() can use them
                if let Some(last_stmt) = decl.body.last() {
                    if let Statement::Expression { expr, .. } = last_stmt {
                        // This is an implicit return - store variable element types
                        if let Some(return_type) = &decl.return_type {
                            self.constrain_expr_to_type(expr, return_type);
                        }
                    }
                }
                
                // Collect return type constraints (now var_element_types is populated)
                for (_i, stmt) in decl.body.iter().enumerate() {
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
                        self.var_types.insert(param.name.clone(), param.type_.clone());
                    }
                    
                    // TDD FIX: Handle implicit returns FIRST (before collecting constraints)
                    if let Some(last_stmt) = func.body.last() {
                        if let Statement::Expression { expr, .. } = last_stmt {
                            if let Some(return_type) = &func.return_type {
                                self.constrain_expr_to_type(expr, return_type);
                            }
                        }
                    }
                    
                    for (_i, stmt) in func.body.iter().enumerate() {
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
            Item::Struct { .. } => {
            }
            Item::Enum { .. } => {
            }
            Item::Trait { .. } => {
            }
            Item::Const { name, type_, value, .. } => {
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
            Item::Static { name, type_, value, .. } => {
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
            Item::Mod { items, .. } => {
                for sub_item in items {
                    self.collect_item_constraints(sub_item);
                }
            }
            _ => {}
        }
    }

    /// Collect constraints from a statement
    fn collect_statement_constraints<'ast>(&mut self, stmt: &Statement<'ast>, return_type: Option<&Type>) {
        match stmt {
            Statement::Let { pattern, value, type_, .. } => {
                // TDD FIX: If type annotation exists, constrain value to that type FIRST
                let explicit_type = type_.as_ref().and_then(|ty| self.extract_float_type(ty));
                
                self.collect_expression_constraints(value, return_type);
                
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
                                self.constraints.push(Constraint::MustBeF32(expr_id, format!("let {} has explicit type f32", var_name)));
                            }
                            FloatType::F64 => {
                                self.constraints.push(Constraint::MustBeF64(expr_id, format!("let {} has explicit type f64", var_name)));
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
                            FloatType::F32 => Constraint::MustBeF32(expr_id, "function return type f32".to_string()),
                            FloatType::F64 => Constraint::MustBeF64(expr_id, "function return type f64".to_string()),
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
                            FloatType::F32 => Constraint::MustBeF32(expr_id, "implicit return f32".to_string()),
                            FloatType::F64 => Constraint::MustBeF64(expr_id, "implicit return f64".to_string()),
                            FloatType::Unknown => return,
                        };
                        self.constraints.push(constraint);
                    }
                }
            }
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
                    self.collect_expression_constraints(expr, return_type);
                    
                    // Return expression must match function return type
                    if let Some(ret_ty) = return_type {
                        if let Some(float_ty) = self.extract_float_type(ret_ty) {
                            let expr_id = self.get_expr_id(expr);
                            let constraint = match float_ty {
                                FloatType::F32 => Constraint::MustBeF32(expr_id, "return type".to_string()),
                                FloatType::F64 => Constraint::MustBeF64(expr_id, "return type".to_string()),
                                FloatType::Unknown => return,
                            };
                            self.constraints.push(constraint);
                        }
                    }
                }
            }
            Statement::If { condition, then_block, else_block, .. } => {
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
                // TDD FIX: Unify then/else branches for if-expr used as value (e.g. x = if c { a } else { b })
                // Ensures 0.0 in else branch gets f32 from 1.0/dt in then branch when dt: f32
                if let (Some(then_last), Some(else_stmts)) = (then_block.last(), else_block) {
                    if let Some(else_last) = else_stmts.last() {
                        if let (Statement::Expression { expr: te, .. }, Statement::Expression { expr: ee, .. }) =
                            (then_last, else_last)
                        {
                            let id1 = self.get_expr_id(te);
                            let id2 = self.get_expr_id(ee);
                            self.constraints.push(Constraint::MustMatch(
                                id1,
                                id2,
                                "if/else branches must have same type".to_string(),
                            ));
                        }
                    }
                }
            }
            Statement::While { condition, body, .. } => {
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
                    format!("assignment target and value"),
                ));
            }
            Statement::Match { value, arms, .. } => {
                // THE WINDJAMMER WAY: Match arms must have compatible types AND match return type
                self.collect_expression_constraints(value, return_type);
                
                // Traverse all arms to collect constraints
                for (i, arm) in arms.iter().enumerate() {
                    self.collect_expression_constraints(arm.body, return_type);
                    
                    // TDD FIX: Constrain arm to return type if function returns float
                    if let Some(ret_ty) = return_type {
                        if let Some(float_ty) = self.extract_float_type(ret_ty) {
                            let arm_id = self.get_expr_id(arm.body);
                            let constraint = match float_ty {
                                FloatType::F32 => Constraint::MustBeF32(arm_id, format!("match arm {} return type", i)),
                                FloatType::F64 => Constraint::MustBeF64(arm_id, format!("match arm {} return type", i)),
                                FloatType::Unknown => continue,
                            };
                            self.constraints.push(constraint);
                        }
                    }
                    
                    if let Some(guard) = arm.guard {
                        self.collect_expression_constraints(guard, return_type);
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
            _other => {
            }
        }
    }

    /// Collect constraints from an expression
    fn collect_expression_constraints<'ast>(&mut self, expr: &Expression<'ast>, return_type: Option<&Type>) {
        match expr {
            Expression::Identifier { name, .. } => {
                // TDD FIX: Constrain identifier to its declared type (function param, let with type annotation, const)
                let var_type = self.var_types.get(name).or_else(|| self.const_types.get(name));
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
            Expression::Binary { left, right, op, .. } => {
                // Binary ops require both operands to have same type
                self.collect_expression_constraints(left, return_type);
                self.collect_expression_constraints(right, return_type);
                
                // For arithmetic ops (+, -, *, /), operands must match
                use crate::parser::ast::operators::BinaryOp;
                match op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div |
                    BinaryOp::Mod => {
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
                            if let Some(var_type) = self.var_types.get(name).or_else(|| self.const_types.get(name)) {
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
                            if let Some(var_type) = self.var_types.get(name).or_else(|| self.const_types.get(name)) {
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
                    BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                        let left_id = self.get_expr_id(left);
                        let right_id = self.get_expr_id(right);
                        self.constraints.push(Constraint::MustMatch(
                            left_id,
                            right_id,
                            format!("comparison {:?} operands", op),
                        ));
                        // TDD FIX: Direct propagation from params to literals in comparisons (dt > 0.0)
                        if let Expression::Identifier { name, .. } = left {
                            if let Some(var_type) = self.var_types.get(name).or_else(|| self.const_types.get(name)) {
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
                            if let Some(var_type) = self.var_types.get(name).or_else(|| self.const_types.get(name)) {
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
            Expression::MethodCall { object, method, arguments, .. } => {
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
                if method == "min" || method == "max" {
                    if arguments.len() == 1 {
                        let receiver_id = self.get_expr_id(object);
                        let arg_id = self.get_expr_id(arguments[0].1);
                        
                        // Receiver and argument must be same type
                        self.constraints.push(Constraint::MustMatch(
                            receiver_id,
                            arg_id,
                            format!(".{}() argument must match receiver type", method),
                        ));
                    }
                }
                
                // TDD FIX: Method calls - constrain args from method signature (metadata)
                // Handles both: self.field.method(...) and local_var.method(...) e.g. voxelizer.voxelize(...)
                let method_sig = self.function_signatures.iter()
                    .filter(|(func_name, (params, _))| {
                        // Match method name and param count
                        // Instance method: params.len() == arguments.len() + 1 (Self)
                        // Associated fn (Type::new): params.len() == arguments.len()
                        let name_match = func_name.split("::").last() == Some(method.as_str());
                        let param_match = params.len() == arguments.len() + 1 || params.len() == arguments.len();
                        name_match && param_match
                    })
                    .map(|(_, (params, ret))| (params.clone(), ret.clone()))
                    .next();
                
                if let Some((param_types, _)) = method_sig {
                    // Found a matching method! Constrain arguments
                    // Instance method: skip index 0 (Self); associated fn: use index i
                    let param_offset = if param_types.len() == arguments.len() + 1 { 1 } else { 0 };
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
                                                    format!("HashMap<K, f32>.insert(K, f32)"),
                                                ));
                                            }
                                            FloatType::F64 => {
                                                self.constraints.push(Constraint::MustBeF64(
                                                    value_id,
                                                    format!("HashMap<K, f64>.insert(K, f64)"),
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
                                    if arguments.len() >= 1 {
                                        let value_arg = arguments[0].1;
                                        let value_id = self.get_expr_id(value_arg);
                                        match float_ty {
                                            FloatType::F32 => {
                                                self.constraints.push(Constraint::MustBeF32(
                                                    value_id,
                                                    format!("Vec<f32>.push(f32)"),
                                                ));
                                            }
                                            FloatType::F64 => {
                                                self.constraints.push(Constraint::MustBeF64(
                                                    value_id,
                                                    format!("Vec<f64>.push(f64)"),
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
                            if arguments.len() >= 1 {
                                let value_arg = arguments[0].1;
                                
                                // TDD FIX: Recursively constrain with the element type
                                // This handles both simple types (f32) and complex types (Tuple)
                                self.collect_expression_constraints(value_arg, Some(&elem_type));
                            }
                        } else if method == "insert" {
                            if arguments.len() >= 2 {
                                let value_arg = arguments[1].1;
                                // TDD FIX: Recursively constrain with the value type
                                self.collect_expression_constraints(value_arg, Some(&elem_type));
                            }
                        }
                    }
                }
                
                // Recurse into ALL arguments to collect binary op constraints
                // This ensures that nested expressions like (x, y, method() * 1.414) are visited
                for (_label, arg) in arguments {
                    self.collect_expression_constraints(arg, return_type);
                }
            }
            Expression::Call { function, arguments, .. } => {
                // Look up function signature and constrain arguments
                self.collect_expression_constraints(function, return_type);
                
                // Extract function name (handles both simple and associated functions)
                let func_name = match function {
                    Expression::Identifier { name, .. } => Some(name.clone()),
                    Expression::FieldAccess { object, field, .. } => {
                        // Handle associated functions like Vec3::new
                        if let Expression::Identifier { name: type_name, .. } = object {
                            Some(format!("{}::{}", type_name, field))
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                
                let func_sig = if let Some(name) = &func_name {
                    let sig = self.function_signatures.get(name).cloned();
                    sig
                } else {
                    None
                };
                
                // TDD FIX: assert_eq/assert_ne (when written as Call, not MacroInvocation)
                // Both args must have same type - e.g. assert_eq(transform.x, 15.0)
                if let Some(name) = &func_name {
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
                    for (i, (_label, arg)) in arguments.iter().enumerate() {
                        self.collect_expression_constraints(arg, return_type);
                        
                        if let Some(param_type) = param_types.get(i) {
                            if let Some(float_ty) = self.extract_float_type(param_type) {
                                let arg_id = self.get_expr_id(arg);
                                let func_name = if let Expression::Identifier { name, .. } = function {
                                    name.clone()
                                } else {
                                    "function".to_string()
                                };
                                let constraint = match float_ty {
                                    FloatType::F32 => Constraint::MustBeF32(
                                        arg_id,
                                        format!("parameter {} of {}", i, func_name),
                                    ),
                                    FloatType::F64 => Constraint::MustBeF64(
                                        arg_id,
                                        format!("parameter {} of {}", i, func_name),
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
                let field_constraints: Vec<(String, &'ast Expression<'ast>, FloatType)> = if let Some(struct_fields) = self.struct_field_types.get(name) {
                    fields.iter().filter_map(|(field_name, field_expr)| {
                        if let Some(field_type) = struct_fields.get(field_name) {
                            self.extract_float_type(field_type).map(|float_ty| {
                                (field_name.clone(), *field_expr, float_ty)
                            })
                        } else {
                            None
                        }
                    }).collect()
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
                                    if let Some(&value_id) = self.var_assignments.get(name.as_str()) {
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
                                    } else {
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
            Expression::Cast { expr: inner, type_, .. } => {
                // Cast expression provides explicit type constraint
                self.collect_expression_constraints(inner, return_type);
                
                if let Some(float_ty) = self.extract_float_type(type_) {
                    let inner_id = self.get_expr_id(inner);
                    let constraint = match float_ty {
                        FloatType::F32 => Constraint::MustBeF32(inner_id, "cast to f32".to_string()),
                        FloatType::F64 => Constraint::MustBeF64(inner_id, "cast to f64".to_string()),
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
                    if let Statement::If { then_block, else_block, .. } = last_stmt {
                        if let (Some(then_last), Some(else_stmts)) = (then_block.last(), else_block) {
                            if let Some(else_last) = else_stmts.last() {
                                if let (Statement::Expression { expr: te, .. }, Statement::Expression { expr: ee, .. }) =
                                    (then_last, else_last)
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
                let struct_name: Option<String> = if let Expression::Identifier { name, .. } = object {
                    if name == "self" {
                        // Case 1: self.field - use current impl type context
                        self.current_impl_type.clone()
                    } else {
                        // Case 2: variable.field - look up variable's type
                        self.var_types.get(name).and_then(|var_type| {
                            match var_type {
                                Type::Custom(name) => Some(name.clone()),
                                _ => None,
                            }
                        })
                    }
                } else {
                    // TDD FIX: Chained FieldAccess - self.player.position.x
                    // Use infer_type_from_expression to resolve object's type
                    self.infer_type_from_expression(object).and_then(|obj_ty| {
                        match &obj_ty {
                            Type::Custom(name) => Some(name.clone()),
                            _ => None,
                        }
                    })
                };
                
                // If we know the struct type, constrain the FieldAccess expression to the field's type
                if let Some(struct_name) = struct_name {
                    if let Some(field_types) = self.struct_field_types.get(&struct_name) {
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
                
                if let Some(elem_type) = self.infer_type_from_expression(object)
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
                if name == "assert_eq" || name == "assert_ne" {
                    if args.len() >= 2 {
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
            Expression::Binary { left, right, op, .. } => {
                use crate::parser::ast::operators::BinaryOp;
                if matches!(op, BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod) {
                    let left_ty = self.infer_type_from_expression(left)?;
                    let right_ty = self.infer_type_from_expression(right)?;
                    if left_ty == right_ty {
                        return Some(left_ty);
                    }
                }
                None
            }
            Expression::Call { function, .. } => {
                // Type::new() or Type::method() - get return type from function signature
                let func_name = match function {
                    Expression::FieldAccess { object, field, .. } => {
                        if let Expression::Identifier { name: type_name, .. } = object {
                            Some(format!("{}::{}", type_name, field))
                        } else {
                            None
                        }
                    }
                    Expression::Identifier { name, .. } => Some(name.clone()),
                    _ => None,
                };
                func_name.and_then(|name| {
                    self.function_signatures.get(&name).and_then(|(_, ret)| ret.clone())
                })
            }
            Expression::MethodCall { object, method, .. } => {
                // object.method() - need object's type to find method signature
                let object_type = self.infer_type_from_expression(object)?;
                let type_name = match &object_type {
                    Type::Custom(name) => name.clone(),
                    _ => return None,
                };
                let full_name = format!("{}::{}", type_name, method);
                if let Some((_, ret)) = self.function_signatures.get(&full_name) {
                    return ret.clone();
                }
                // TDD FIX: Fallback for primitive methods (sqrt, length, etc.) - return same type as receiver
                // Handles (x*x + y*y).sqrt() where x,y are f32 - sqrt returns f32
                const PRIMITIVE_SAME_TYPE: &[&str] = &[
                    "sqrt", "sin", "cos", "tan", "abs", "floor", "ceil", "round",
                    "length", "magnitude", "distance", "dot", "recip",
                ];
                if PRIMITIVE_SAME_TYPE.contains(&method.as_str()) {
                    if matches!(object_type, Type::Custom(ref s) if s == "f32" || s == "f64")
                        || matches!(object_type, Type::Float)
                    {
                        return Some(object_type);
                    }
                }
                None
            }
            Expression::Identifier { name, .. } => {
                if name == "self" {
                    self.current_impl_type
                        .as_ref()
                        .map(|s| Type::Custom(s.clone()))
                } else {
                    self.var_types.get(name)
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
                self.struct_field_types
                    .get(&struct_name)
                    .and_then(|fields| fields.get(field))
                    .cloned()
            }
            Expression::Index { object, .. } => {
                let object_type = self.infer_type_from_expression(object)?;
                self.extract_vec_element_type(&object_type)
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
            Type::Parameterized(name, type_args) if name == "Vec" && !type_args.is_empty() => {
                self.extract_float_type(&type_args[0])
            }
            Type::Parameterized(name, type_args) if name == "HashMap" && type_args.len() >= 2 => {
                self.extract_float_type(&type_args[1])
            }
            _ => None,
        }
    }
    
    /// TDD FIX: Extract value type V from HashMap<K, V>
    fn extract_hashmap_value_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Parameterized(name, type_args) if name == "HashMap" => {
                // HashMap<K, V> has 2 type arguments, V is at index 1
                if type_args.len() >= 2 {
                    Some(type_args[1].clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    
    /// TDD FIX: Extract element type T from Vec<T>
    fn extract_vec_element_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Vec(inner) => Some((**inner).clone()),
            Type::Parameterized(name, type_args) if name == "Vec" => {
                // Vec<T> has 1 type argument
                if type_args.len() >= 1 {
                    Some(type_args[0].clone())
                } else {
                    None
                }
            }
            _ => None,
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
        
        // Check cache first - if we've seen this location before, return same ID
        if line > 0 {  // Only cache expressions with valid locations
            if let Some(&cached_id) = self.expr_id_cache.get(&(line, col)) {
                return cached_id;
            }
        }
        
        // Generate new sequential ID
        let seq_id = self.next_seq_id;
        self.next_seq_id += 1;
        
        let expr_id = ExprId { seq_id, line, col };
        
        // Cache it for future lookups
        if line > 0 {
            self.expr_id_cache.insert((line, col), expr_id);
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
            "sin", "cos", "tan", "asin", "acos", "atan", "atan2",
            "sinh", "cosh", "tanh", "asinh", "acosh", "atanh",
            // Exponential/logarithmic
            "exp", "exp2", "exp_m1", "ln", "log", "log2", "log10", "ln_1p",
            // Power/root
            "sqrt", "cbrt", "hypot", "powf", "powi",
            // Rounding
            "floor", "ceil", "round", "trunc", "fract",
            // Absolute/sign
            "abs", "signum", "copysign",
            // Min/max
            "max", "min", "clamp",
            // Misc
            "recip", "to_degrees", "to_radians",
            // Custom (from game code)
            "get_cost", "get", "distance", "length", "dot", "cross", "magnitude",
        ];
        
        // Check if this is a method call on an identifier
        if let Expression::Identifier { name, .. } = object {
            // Look up the identifier's type from var_types
            if let Some(var_type) = self.var_types.get(name) {
                // Check if it's an f32 or f64 type
                let is_f32 = matches!(var_type, Type::Float) || 
                             matches!(var_type, Type::Custom(s) if s == "f32");
                let is_f64 = matches!(var_type, Type::Custom(s) if s == "f64");
                
                if is_f32 && F32_METHODS.contains(&method) {
                    return Some(FloatType::F32);
                }
                if is_f64 && F32_METHODS.contains(&method) {
                    return Some(FloatType::F64);
                }
            }
        }
        
        // Check if this is a method call on a FieldAccess (e.g., self.field.method())
        if let Expression::FieldAccess { .. } = object {
            // Try to find method signature from metadata
            for (func_name, (_, ret_type_opt)) in self.function_signatures.iter() {
                if func_name.split("::").last() == Some(method) {
                    if let Some(ret_type) = ret_type_opt {
                        return self.extract_float_type(ret_type);
                    }
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
    fn solve_constraints(&mut self) {
        // Simple constraint solver: Apply constraints repeatedly until convergence
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
                            (Some(FloatType::F32), Some(FloatType::F64)) |
                            (Some(FloatType::F64), Some(FloatType::F32)) => {
                                self.errors.push(format!(
                                    "Type mismatch at {:?} and {:?}: {} requires same float type",
                                    id1, id2, reason
                                ));
                            }
                            // Propagate f32 to unknown or untyped
                            (Some(FloatType::F32), Some(FloatType::Unknown)) |
                            (Some(FloatType::F32), None) => {
                                self.inferred_types.insert(id2, FloatType::F32);
                                changed = true;
                            }
                            (Some(FloatType::Unknown), Some(FloatType::F32)) |
                            (None, Some(FloatType::F32)) => {
                                self.inferred_types.insert(id1, FloatType::F32);
                                changed = true;
                            }
                            // Propagate f64 to unknown or untyped
                            (Some(FloatType::F64), Some(FloatType::Unknown)) |
                            (Some(FloatType::F64), None) => {
                                self.inferred_types.insert(id2, FloatType::F64);
                                changed = true;
                            }
                            (Some(FloatType::Unknown), Some(FloatType::F64)) |
                            (None, Some(FloatType::F64)) => {
                                self.inferred_types.insert(id1, FloatType::F64);
                                changed = true;
                            }
                            // Both same concrete type - no change
                            (Some(FloatType::F32), Some(FloatType::F32)) |
                            (Some(FloatType::F64), Some(FloatType::F64)) => {}
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
    fn constrain_nested_floats<'ast>(&mut self, expr: &Expression<'ast>, return_type: Option<&Type>) {
        match expr {
            Expression::Binary { left, right, op, .. } => {
                use crate::parser::ast::operators::BinaryOp;
                match op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div |
                    BinaryOp::Mod => {
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
            Expression::Cast { expr: inner, type_, .. } => {
                // Cast expression provides explicit type hint
                if let Some(float_ty) = self.extract_float_type(type_) {
                    let inner_id = self.get_expr_id(inner);
                    let constraint = match float_ty {
                        FloatType::F32 => Constraint::MustBeF32(inner_id, "cast to f32".to_string()),
                        FloatType::F64 => Constraint::MustBeF64(inner_id, "cast to f64".to_string()),
                        FloatType::Unknown => return,
                    };
                    self.constraints.push(constraint);
                }
                self.constrain_nested_floats(inner, return_type);
            }
            _ => {}
        }
    }

    /// Get inferred float type for an expression
    pub fn get_float_type<'ast>(&self, expr: &Expression<'ast>) -> FloatType {
        // Look up by location only (seq_id not available after inference)
        // Find ExprId with matching location
        let location = expr.location();
        let (line, col) = if let Some(loc) = location {
            (loc.line, loc.column)
        } else {
            (0, 0)
        };
        
        // Search for any ExprId with matching location
        for (expr_id, float_type) in &self.inferred_types {
            if expr_id.line == line && expr_id.col == col {
                return *float_type;
            }
        }
        
        // Return Unknown when no match - enables fallback to context-sensitive inference
        // (struct field type, return type, etc.) in generate_literal_with_context
        FloatType::Unknown
    }

    /// TDD FIX: Constrain an expression to match a specific type
    /// Used for implicit returns: return type → variable type → collection elements
    fn constrain_expr_to_type<'ast>(&mut self, expr: &Expression<'ast>, target_type: &Type) {
        match expr {
            Expression::Identifier { name, .. } => {
                // Variable being returned - store its element type if it's a collection
                match target_type {
                    Type::Vec(inner) => {
                        // Vec<T>
                        self.var_element_types.insert(name.clone(), (**inner).clone());
                    }
                    Type::Parameterized(type_name, parameters) => {
                        // Vec<T>, HashMap<K,V>, Option<T>, etc.
                        if type_name == "Vec" && parameters.len() == 1 {
                            self.var_element_types.insert(name.clone(), parameters[0].clone());
                        } else if type_name == "HashMap" && parameters.len() == 2 {
                            // Store the value type (second parameter)
                            self.var_element_types.insert(name.clone(), parameters[1].clone());
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
