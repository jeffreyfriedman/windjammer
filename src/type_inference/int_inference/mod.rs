/// Integer Type Inference Engine
///
/// Tracks constraints for integer literals and unifies them across expressions.
/// Mirrors FloatInference architecture. Defaults to i32 for unknown contexts (Rust convention).
use crate::parser::ast::core::{Expression, Item, Statement};
use crate::parser::ast::operators::BinaryOp;
use crate::parser::ast::types::Type;
use crate::parser::Program;
use crate::type_inference::int_implicit_casts::is_safe_implicit_cast;
use crate::type_inference::struct_field_registry;
use crate::type_inference::ExprId;
use std::collections::HashMap;

/// Integer type (i32, i64, u32, u64, usize, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntType {
    I32,
    I64,
    U32,
    U64,
    Usize,
    Isize,
    U8,
    I8,
    U16,
    I16,
    Unknown,
}

impl IntType {
    /// Rust suffix for codegen (e.g., 42_i32)
    pub fn rust_suffix(self) -> &'static str {
        match self {
            IntType::I32 => "i32",
            IntType::I64 => "i64",
            IntType::U32 => "u32",
            IntType::U64 => "u64",
            IntType::Usize => "usize",
            IntType::Isize => "isize",
            IntType::U8 => "u8",
            IntType::I8 => "i8",
            IntType::U16 => "u16",
            IntType::I16 => "i16",
            IntType::Unknown => "i32", // Rust default
        }
    }
}

/// Constraint on an expression's integer type
#[derive(Debug, Clone)]
pub enum IntConstraint {
    MustBe(ExprId, IntType, String),
    MustMatch(ExprId, ExprId, String),
}

/// Integer type inference state
#[derive(Clone)]
pub struct IntInference {
    pub inferred_types: HashMap<ExprId, IntType>,
    constraints: Vec<IntConstraint>,
    pub errors: Vec<String>,
    function_signatures: HashMap<String, (Vec<Type>, Option<Type>)>,
    var_assignments: HashMap<String, ExprId>,
    var_types: HashMap<String, Type>,
    next_seq_id: usize,
    struct_field_types: HashMap<String, HashMap<String, Type>>,
    /// Library multipass: module path for the current `.wj` file (`dialogue/examples` → `["dialogue","examples"]`).
    current_file_module_path: Vec<String>,
    /// For each bare struct name, all module prefixes where that name is defined (disambiguate duplicates).
    struct_defining_module_paths: HashMap<String, Vec<Vec<String>>>,
    /// `use` resolution: bare imported type → fully qualified registry key.
    imported_type_registry_keys: HashMap<String, String>,
    /// `pub use` from each module (`module_path` as `a::b` or `""` for crate root) → exported name → qualified struct key.
    module_re_exports: HashMap<String, HashMap<String, String>>,
    expr_id_cache: HashMap<(usize, usize, usize), ExprId>,
    current_file_id: usize,
    file_name_to_id: HashMap<String, usize>,
    id_to_file_name: HashMap<usize, String>,
    next_file_id: usize,
    current_impl_type: Option<String>,
    const_types: HashMap<String, Type>,
}

impl Default for IntInference {
    fn default() -> Self {
        Self::new()
    }
}

impl IntInference {
    pub fn new() -> Self {
        let mut inference = IntInference {
            inferred_types: HashMap::new(),
            constraints: Vec::new(),
            errors: Vec::new(),
            function_signatures: HashMap::new(),
            var_assignments: HashMap::new(),
            var_types: HashMap::new(),
            next_seq_id: 1,
            struct_field_types: HashMap::new(),
            current_file_module_path: Vec::new(),
            struct_defining_module_paths: HashMap::new(),
            imported_type_registry_keys: HashMap::new(),
            module_re_exports: HashMap::new(),
            expr_id_cache: HashMap::new(),
            current_file_id: 0,
            file_name_to_id: HashMap::new(),
            id_to_file_name: HashMap::new(),
            next_file_id: 1,
            current_impl_type: None,
            const_types: HashMap::new(),
        };

        // TDD FIX: Register stdlib method signatures (Vec, HashMap, etc.)
        inference.register_stdlib_signatures();

        inference
    }

    /// Register common stdlib method signatures for type inference
    fn register_stdlib_signatures(&mut self) {
        use crate::parser::Type;

        // Vec<T> methods
        self.function_signatures.insert(
            "Vec::with_capacity".to_string(),
            (vec![Type::Custom("usize".to_string())], None),
        );
        self.function_signatures.insert(
            "Vec::reserve".to_string(),
            (vec![Type::Custom("usize".to_string())], None),
        );
        self.function_signatures.insert(
            "Vec::resize".to_string(),
            (vec![Type::Custom("usize".to_string()), Type::Int], None), // (new_len: usize, value: T)
        );
        self.function_signatures.insert(
            "Vec::truncate".to_string(),
            (vec![Type::Custom("usize".to_string())], None),
        );

        // HashMap<K,V> methods
        self.function_signatures.insert(
            "HashMap::with_capacity".to_string(),
            (vec![Type::Custom("usize".to_string())], None),
        );

        // String methods
        self.function_signatures.insert(
            "String::with_capacity".to_string(),
            (vec![Type::Custom("usize".to_string())], None),
        );
    }

    pub fn set_current_file(&mut self, file: String) -> usize {
        if let Some(&id) = self.file_name_to_id.get(&file) {
            self.current_file_id = id;
            id
        } else {
            let id = self.next_file_id;
            self.next_file_id += 1;
            self.file_name_to_id.insert(file.clone(), id);
            self.id_to_file_name.insert(id, file);
            self.current_file_id = id;
            id
        }
    }

    pub fn set_global_function_signatures(
        &mut self,
        signatures: HashMap<String, (Vec<Type>, Option<Type>)>,
    ) {
        self.function_signatures = signatures;
    }

    pub fn set_global_struct_field_types(
        &mut self,
        field_types: &HashMap<String, HashMap<String, Type>>,
    ) {
        for (struct_name, fields) in field_types {
            self.struct_field_types
                .insert(struct_name.clone(), fields.clone());
        }
    }

    /// Multipass library: module path segments for the source file being analyzed.
    pub fn set_current_file_module_path(&mut self, path: Vec<String>) {
        self.current_file_module_path = path;
    }

    /// All defining module prefixes per bare struct name (for duplicate names across modules).
    pub fn set_struct_defining_module_paths(&mut self, paths: HashMap<String, Vec<Vec<String>>>) {
        self.struct_defining_module_paths = paths;
    }

    /// Multipass: `pub use` re-exports per module (from a full-crate pre-pass). Required for `use super::*` glob resolution.
    pub fn set_module_re_exports(&mut self, re_exports: HashMap<String, HashMap<String, String>>) {
        self.module_re_exports = re_exports;
    }

    /// Main entry point: Infer integer types for a program
    pub fn infer_program<'ast>(&mut self, program: &Program<'ast>) {
        self.imported_type_registry_keys.clear();
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

        self.register_use_imports_from_items(&program.items);

        for item in &program.items {
            self.collect_item_constraints(item);
        }

        self.solve_constraints();
    }

    /// TDD FIX: Substitute generic type parameters with concrete types
    /// E.g., for HashMap<u32, String>::insert, parameter type K becomes u32
    /// Generic params: K=0, V=1, T=2, etc.
    fn substitute_generic_params(&self, ty: &Type, generics: &[String]) -> Type {
        match ty {
            Type::Custom(name) if name.len() == 1 => {
                // Single-letter types like K, V, T are likely generics
                let ch = name.chars().next().unwrap();
                if ch.is_ascii_uppercase() {
                    // Common generic parameter names and their indices
                    let idx = match ch {
                        'K' => 0, // Key type (HashMap)
                        'V' => 1, // Value type (HashMap)
                        'T' => 0, // Generic T (Vec, Option, etc.)
                        'U' => 1, // Second generic
                        'E' => 1, // Error type (Result)
                        _ => return ty.clone(),
                    };
                    if let Some(concrete) = generics.get(idx) {
                        // Parse the concrete type string
                        return self.parse_type_from_string(concrete);
                    }
                }
                ty.clone()
            }
            Type::Option(inner) => Type::Option(Box::new(self.substitute_generic_params(inner, generics))),
            Type::Result(ok, err) => Type::Result(
                Box::new(self.substitute_generic_params(ok, generics)),
                Box::new(self.substitute_generic_params(err, generics)),
            ),
            Type::Vec(inner) => Type::Vec(Box::new(self.substitute_generic_params(inner, generics))),
            _ => ty.clone(),
        }
    }

    /// Parse a type from a string representation (e.g., "u32" → Type::Uint)
    fn parse_type_from_string(&self, s: &str) -> Type {
        match s {
            "u32" => Type::Uint,
            "i32" => Type::Int32,
            "i64" => Type::Int,
            "f32" => Type::Float,
            "f64" => Type::Float,
            "bool" => Type::Bool,
            "usize" => Type::Custom("usize".to_string()),
            "string" => Type::String,
            _ => Type::Custom(s.to_string()),
        }
    }

    fn lookup_struct_fields(&self, type_name: &str) -> Option<&HashMap<String, Type>> {
        struct_field_registry::lookup_struct_field_map(
            &self.struct_field_types,
            type_name,
            &self.imported_type_registry_keys,
            &self.struct_defining_module_paths,
        )
    }

    /// See `float_inference::FloatInference::lookup_struct_fields_for_impl_type`.
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
                        if struct_field_registry::debug_struct_import_trace() {
                            eprintln!(
                                "=== int_inference: glob import path={:?} file_module={:?}",
                                path, self.current_file_module_path
                            );
                        }
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

    fn register_function_signature<'ast>(&mut self, item: &Item<'ast>) {
        match item {
            Item::Function { decl, .. } => {
                let param_types: Vec<Type> =
                    decl.parameters.iter().map(|p| p.type_.clone()).collect();
                self.function_signatures
                    .insert(decl.name.clone(), (param_types, decl.return_type.clone()));
            }
            Item::Impl { block, .. } => {
                let type_name = block.type_name.clone();
                for func_decl in &block.functions {
                    let param_types: Vec<Type> = func_decl
                        .parameters
                        .iter()
                        .map(|p| p.type_.clone())
                        .collect();
                    let full_name = format!("{}::{}", type_name, func_decl.name);
                    self.function_signatures
                        .insert(full_name, (param_types, func_decl.return_type.clone()));
                }
            }
            _ => {}
        }
    }

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

    fn collect_item_constraints<'ast>(&mut self, item: &Item<'ast>) {
        match item {
            Item::Function { decl, .. } => {
                self.expr_id_cache.clear();
                self.var_assignments.clear(); // TDD FIX: Clear per-function scope
                let saved_var_types = self.var_types.clone(); // TDD FIX: Save for later restore
                for param in &decl.parameters {
                    self.var_types
                        .insert(param.name.clone(), param.type_.clone());
                }
                // TDD FIX: Pre-pass - populate var_types from return statements
                // e.g. "return count" with return type u32 → var_types["count"] = u32
                // Enables compound assignment "count += 1" to infer 1_u32 before we process it
                if let Some(return_type) = &decl.return_type {
                    for stmt in &decl.body {
                        if let Statement::Return {
                            value: Some(Expression::Identifier { name, .. }),
                            ..
                        } = stmt
                        {
                            self.var_types.insert(name.clone(), return_type.clone());
                        }
                    }
                }
                if let Some(Statement::Expression { expr, .. }) = decl.body.last() {
                    if let Some(return_type) = &decl.return_type {
                        self.constrain_expr_to_int_type(expr, return_type);
                    }
                }
                for stmt in &decl.body {
                    self.collect_statement_constraints(stmt, decl.return_type.as_ref());
                }
                // TDD FIX: Restore saved var_types (clear function-local variables)
                self.var_types = saved_var_types;
            }
            Item::Impl { block, .. } => {
                self.current_impl_type = Some(block.type_name.clone());
                for func in &block.functions {
                    self.expr_id_cache.clear();
                    self.var_assignments.clear(); // TDD FIX: Clear per-function scope
                    let saved_var_types = self.var_types.clone(); // TDD FIX: Save for later restore
                    for param in &func.parameters {
                        self.var_types
                            .insert(param.name.clone(), param.type_.clone());
                    }
                    // TDD FIX: Pre-pass - populate var_types from return statements (same as Function)
                    if let Some(return_type) = &func.return_type {
                        for stmt in &func.body {
                            if let Statement::Return {
                                value: Some(Expression::Identifier { name, .. }),
                                ..
                            } = stmt
                            {
                                self.var_types.insert(name.clone(), return_type.clone());
                            }
                        }
                    }
                    if let Some(Statement::Expression { expr, .. }) = func.body.last() {
                        if let Some(return_type) = &func.return_type {
                            self.constrain_expr_to_int_type(expr, return_type);
                        }
                    }
                    for stmt in &func.body {
                        self.collect_statement_constraints(stmt, func.return_type.as_ref());
                    }
                    // TDD FIX: Restore saved var_types (clear function-local variables)
                    self.var_types = saved_var_types.clone();
                }
                self.current_impl_type = None;
            }
            Item::Const {
                name, type_, value, ..
            } => {
                self.collect_expression_constraints(value, None);
                if let Some(int_ty) = self.extract_int_type(type_) {
                    let expr_id = self.get_expr_id(value);
                    self.constraints.push(IntConstraint::MustBe(
                        expr_id,
                        int_ty,
                        format!("const {} is {:?}", name, int_ty),
                    ));
                }
            }
            Item::Static {
                name, type_, value, ..
            } => {
                self.collect_expression_constraints(value, None);
                if let Some(int_ty) = self.extract_int_type(type_) {
                    let expr_id = self.get_expr_id(value);
                    self.constraints.push(IntConstraint::MustBe(
                        expr_id,
                        int_ty,
                        format!("static {} is {:?}", name, int_ty),
                    ));
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

    fn constrain_expr_to_int_type<'ast>(&mut self, expr: &Expression<'ast>, ty: &Type) {
        if let Some(int_ty) = self.extract_int_type(ty) {
            let expr_id = self.get_expr_id(expr);
            self.constraints.push(IntConstraint::MustBe(
                expr_id,
                int_ty,
                "return/constraint type".to_string(),
            ));
        }
        self.collect_expression_constraints(expr, Some(ty));
    }

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
                let explicit_type = type_.as_ref().and_then(|ty| self.extract_int_type(ty));

                // Don't pass return_type to let statement values - they have their own types!
                self.collect_expression_constraints(value, type_.as_ref());

                if let crate::parser::ast::core::Pattern::Identifier(var_name) = pattern {
                    let value_id = self.get_expr_id(value);
                    self.var_assignments.insert(var_name.clone(), value_id);
                    if let Some(ty) = type_ {
                        self.var_types.insert(var_name.clone(), ty.clone());
                    } else if let Some(inferred_ty) = self.infer_type_from_expression(value) {
                        // TDD: Infer var type from StructLiteral, Call, etc. for method receiver resolution
                        self.var_types.insert(var_name.clone(), inferred_ty);
                    }
                    if let Some(int_ty) = explicit_type {
                        self.constraints.push(IntConstraint::MustBe(
                            value_id,
                            int_ty,
                            format!("let {} has explicit type", var_name),
                        ));
                    }
                }

                // TDD FIX: REMOVED buggy code that constrained let values to function return type!
                // Let statements should NOT be constrained by function return type.
                // Only implicit returns (Expression statements) should be.
            }
            Statement::Expression { expr, .. } => {
                self.collect_expression_constraints(expr, return_type);
                if let Some(ret_ty) = return_type {
                    if let Some(int_ty) = self.extract_int_type(ret_ty) {
                        let expr_id = self.get_expr_id(expr);
                        self.constraints.push(IntConstraint::MustBe(
                            expr_id,
                            int_ty,
                            "implicit return".to_string(),
                        ));
                    }
                }
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                self.collect_expression_constraints(expr, return_type);
                if let Some(ret_ty) = return_type {
                    if let Some(int_ty) = self.extract_int_type(ret_ty) {
                        let expr_id = self.get_expr_id(expr);
                        self.constraints.push(IntConstraint::MustBe(
                            expr_id,
                            int_ty,
                            "return type".to_string(),
                        ));
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
                self.collect_expression_constraints(condition, return_type);
                for s in then_block {
                    self.collect_statement_constraints(s, return_type);
                }
                if let Some(else_stmts) = else_block {
                    for s in else_stmts {
                        self.collect_statement_constraints(s, return_type);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.collect_expression_constraints(condition, return_type);
                for s in body {
                    self.collect_statement_constraints(s, return_type);
                }
            }
            Statement::For { iterable, body, .. } => {
                self.collect_expression_constraints(iterable, return_type);
                for s in body {
                    self.collect_statement_constraints(s, return_type);
                }
            }
            Statement::Assignment {
                target,
                value,
                compound_op,
                ..
            } => {
                self.collect_expression_constraints(target, return_type);
                self.collect_expression_constraints(value, return_type);
                let target_id = self.get_expr_id(target);
                let value_id = self.get_expr_id(value);
                self.constraints.push(IntConstraint::MustMatch(
                    target_id,
                    value_id,
                    "assignment".to_string(),
                ));

                // Simple assignment: constrain RHS literals to the target's integer type (i64, usize, u32, …).
                // MustMatch alone can leave the literal as default i32 when the LHS is a field access.
                if compound_op.is_none() {
                    let target_type = self.infer_type_from_expression(target);
                    if let Some(ref tt) = target_type {
                        if let Some(int_ty) = self.extract_int_type(tt) {
                            if int_ty != IntType::Unknown {
                                self.constraints.push(IntConstraint::MustBe(
                                    value_id,
                                    int_ty,
                                    format!("assignment to {:?} field/variable", int_ty),
                                ));
                            }
                        }
                    }
                }

                // TDD FIX: Detect compound assignment pattern even when compound_op is None
                // Parser generates: x = x + 1 (compound_op: None, value: Binary)
                // Codegen optimizes to: x += 1
                // We need to constrain the literal to match x's type!
                //
                // Pattern: target = Binary { left: target, op: Add/Sub/Mul/Div, right: literal }
                let is_compound_pattern = if let Expression::Binary { left, op, .. } = value {
                    let targets_match = match (target, &**left) {
                        (
                            Expression::Identifier { name: t, .. },
                            Expression::Identifier { name: l, .. },
                        ) => t == l,
                        (
                            Expression::FieldAccess {
                                object: to,
                                field: tf,
                                ..
                            },
                            Expression::FieldAccess {
                                object: lo,
                                field: lf,
                                ..
                            },
                        ) => {
                            // Both are field accesses - check if same field
                            tf == lf
                                && match (&**to, &**lo) {
                                    (
                                        Expression::Identifier { name: ton, .. },
                                        Expression::Identifier { name: lon, .. },
                                    ) => ton == lon,
                                    _ => false,
                                }
                        }
                        _ => false,
                    };
                    targets_match
                        && matches!(
                            op,
                            BinaryOp::Add
                                | BinaryOp::Sub
                                | BinaryOp::Mul
                                | BinaryOp::Div
                                | BinaryOp::Mod
                        )
                } else {
                    false
                };

                // TDD FIX: Compound assignment (+=, -=, etc.) - RHS literal must match LHS type
                // e.g. count += 1 where count: u32 → 1 must be u32, not i32
                if compound_op.is_some() || is_compound_pattern {
                    let target_type = self.infer_type_from_expression(target);
                    if let Some(ref tt) = target_type {
                        if let Some(int_ty) = self.extract_int_type(tt) {
                            // For compound pattern (x = x + 1), constrain the RHS of the binary op
                            if is_compound_pattern {
                                if let Expression::Binary { right, .. } = value {
                                    let right_id = self.get_expr_id(right);
                                    self.constraints.push(IntConstraint::MustBe(
                                        right_id,
                                        int_ty,
                                        format!(
                                            "compound pattern RHS must match LHS type {:?}",
                                            int_ty
                                        ),
                                    ));
                                }
                            } else {
                                // Explicit compound_op: constrain the value directly
                                self.constraints.push(IntConstraint::MustBe(
                                    value_id,
                                    int_ty,
                                    format!("compound assignment target has type {:?}", int_ty),
                                ));
                            }
                        }
                    }
                }
            }
            Statement::Match { value, arms, .. } => {
                self.collect_expression_constraints(value, return_type);
                for arm in arms {
                    self.collect_expression_constraints(arm.body, return_type);
                    if let Some(ret_ty) = return_type {
                        if let Some(int_ty) = self.extract_int_type(ret_ty) {
                            let expr_id = self.get_expr_id(arm.body);
                            self.constraints.push(IntConstraint::MustBe(
                                expr_id,
                                int_ty,
                                "match arm return".to_string(),
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

include!("literal_inference.rs");
include!("get_expr_id.rs");
include!("binary_op_inference.rs");
include!("expression_constraints.rs");
include!("constraint_solving.rs");
include!("get_int_type.rs");
