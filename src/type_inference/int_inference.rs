/// Integer Type Inference Engine
///
/// Tracks constraints for integer literals and unifies them across expressions.
/// Mirrors FloatInference architecture. Defaults to i32 for unknown contexts (Rust convention).

use crate::parser::ast::core::{Expression, Statement, Item};
use crate::parser::ast::types::Type;
use crate::parser::Program;
use crate::type_inference::ExprId;
use crate::type_inference::int_implicit_casts::{is_safe_implicit_cast, promote_types};
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
    expr_id_cache: HashMap<(usize, usize, usize), ExprId>,
    current_file_id: usize,
    file_name_to_id: HashMap<String, usize>,
    id_to_file_name: HashMap<usize, String>,
    next_file_id: usize,
    current_impl_type: Option<String>,
    const_types: HashMap<String, Type>,
}

impl IntInference {
    pub fn new() -> Self {
        IntInference {
            inferred_types: HashMap::new(),
            constraints: Vec::new(),
            errors: Vec::new(),
            function_signatures: HashMap::new(),
            var_assignments: HashMap::new(),
            var_types: HashMap::new(),
            next_seq_id: 1,
            struct_field_types: HashMap::new(),
            expr_id_cache: HashMap::new(),
            current_file_id: 0,
            file_name_to_id: HashMap::new(),
            id_to_file_name: HashMap::new(),
            next_file_id: 1,
            current_impl_type: None,
            const_types: HashMap::new(),
        }
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
                .entry(struct_name.clone())
                .or_default()
                .extend(fields.clone());
        }
    }

    /// Main entry point: Infer integer types for a program
    pub fn infer_program<'ast>(&mut self, program: &Program<'ast>) {
        if let Some(first_item) = program.items.first() {
            if let Some(loc) = first_item.location() {
                self.set_current_file(loc.file.to_string_lossy().to_string());
            }
        }

        for item in &program.items {
            self.register_struct_fields(item);
            self.register_function_signature(item);
            self.register_const_and_static(item);
        }

        for item in &program.items {
            self.collect_item_constraints(item);
        }

        self.solve_constraints();
    }

    fn register_struct_fields<'ast>(&mut self, item: &Item<'ast>) {
        if let Item::Struct { decl, .. } = item {
            let mut field_map = HashMap::new();
            for field in &decl.fields {
                field_map.insert(field.name.clone(), field.field_type.clone());
            }
            self.struct_field_types.insert(decl.name.clone(), field_map);
        }
    }

    fn register_function_signature<'ast>(&mut self, item: &Item<'ast>) {
        match item {
            Item::Function { decl, .. } => {
                let param_types: Vec<Type> = decl.parameters.iter().map(|p| p.type_.clone()).collect();
                self.function_signatures.insert(
                    decl.name.clone(),
                    (param_types, decl.return_type.clone()),
                );
            }
            Item::Impl { block, .. } => {
                let type_name = block.type_name.clone();
                for func_decl in &block.functions {
                    let param_types: Vec<Type> =
                        func_decl.parameters.iter().map(|p| p.type_.clone()).collect();
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
                for param in &decl.parameters {
                    self.var_types.insert(param.name.clone(), param.type_.clone());
                }
                // TDD FIX: Pre-pass - populate var_types from return statements
                // e.g. "return count" with return type u32 → var_types["count"] = u32
                // Enables compound assignment "count += 1" to infer 1_u32 before we process it
                if let Some(return_type) = &decl.return_type {
                    for stmt in &decl.body {
                        if let Statement::Return { value: Some(expr), .. } = stmt {
                            if let Expression::Identifier { name, .. } = expr {
                                self.var_types.insert(name.clone(), return_type.clone());
                            }
                        }
                    }
                }
                if let Some(last_stmt) = decl.body.last() {
                    if let Statement::Expression { expr, .. } = last_stmt {
                        if let Some(return_type) = &decl.return_type {
                            self.constrain_expr_to_int_type(expr, return_type);
                        }
                    }
                }
                for stmt in &decl.body {
                    self.collect_statement_constraints(stmt, decl.return_type.as_ref());
                }
                for param in &decl.parameters {
                    self.var_types.remove(&param.name);
                }
            }
            Item::Impl { block, .. } => {
                self.current_impl_type = Some(block.type_name.clone());
                for func in &block.functions {
                    self.expr_id_cache.clear();
                    for param in &func.parameters {
                        self.var_types.insert(param.name.clone(), param.type_.clone());
                    }
                    // TDD FIX: Pre-pass - populate var_types from return statements (same as Function)
                    if let Some(return_type) = &func.return_type {
                        for stmt in &func.body {
                            if let Statement::Return { value: Some(expr), .. } = stmt {
                                if let Expression::Identifier { name, .. } = expr {
                                    self.var_types.insert(name.clone(), return_type.clone());
                                }
                            }
                        }
                    }
                    if let Some(last_stmt) = func.body.last() {
                        if let Statement::Expression { expr, .. } = last_stmt {
                            if let Some(return_type) = &func.return_type {
                                self.constrain_expr_to_int_type(expr, return_type);
                            }
                        }
                    }
                    for stmt in &func.body {
                        self.collect_statement_constraints(stmt, func.return_type.as_ref());
                    }
                    for param in &func.parameters {
                        self.var_types.remove(&param.name);
                    }
                }
                self.current_impl_type = None;
            }
            Item::Const { name, type_, value, .. } => {
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
            Item::Static { name, type_, value, .. } => {
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
            Item::Mod { items, .. } => {
                for sub_item in items {
                    self.collect_item_constraints(sub_item);
                }
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
            Statement::Let { pattern, value, type_, .. } => {
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
            Statement::Return { value, .. } => {
                if let Some(expr) = value {
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
            }
            Statement::If { condition, then_block, else_block, .. } => {
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
            Statement::While { condition, body, .. } => {
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
            Statement::Assignment { target, value, compound_op, .. } => {
                self.collect_expression_constraints(target, return_type);
                self.collect_expression_constraints(value, return_type);
                let target_id = self.get_expr_id(target);
                let value_id = self.get_expr_id(value);
                self.constraints.push(IntConstraint::MustMatch(
                    target_id,
                    value_id,
                    "assignment".to_string(),
                ));
                // TDD FIX: Compound assignment (+=, -=, etc.) - RHS literal must match LHS type
                // e.g. count += 1 where count: u32 → 1 must be u32, not i32
                if compound_op.is_some() {
                    if let Some(target_type) = self.infer_type_from_expression(target) {
                        if let Some(int_ty) = self.extract_int_type(&target_type) {
                            self.constraints.push(IntConstraint::MustBe(
                                value_id,
                                int_ty,
                                "compound assignment RHS must match LHS type".to_string(),
                            ));
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

    fn collect_expression_constraints<'ast>(
        &mut self,
        expr: &Expression<'ast>,
        return_type: Option<&Type>,
    ) {
        match expr {
            Expression::Identifier { name, .. } => {
                if let Some(var_type) = self.var_types.get(name).or_else(|| self.const_types.get(name)) {
                    if let Some(int_ty) = self.extract_int_type(var_type) {
                        let id = self.get_expr_id(expr);
                        self.constraints.push(IntConstraint::MustBe(
                            id,
                            int_ty,
                            format!("identifier {} type", name),
                        ));
                    }
                }
            }
            Expression::Call { function, arguments, .. } => {
                self.collect_expression_constraints(function, return_type);

                let func_name = match function {
                    Expression::Identifier { name, .. } => Some(name.clone()),
                    Expression::FieldAccess { object, field, .. } => {
                        if let Expression::Identifier { name: type_name, .. } = object {
                            Some(format!("{}::{}", type_name, field))
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                // TDD FIX: assert_eq/assert_ne - both args must have same int type
                if (func_name.as_deref() == Some("assert_eq") || func_name.as_deref() == Some("assert_ne"))
                    && arguments.len() >= 2
                {
                    for (_label, arg) in arguments {
                        self.collect_expression_constraints(arg, return_type);
                    }
                    if let (Some((_, first)), Some((_, second))) =
                        (arguments.get(0), arguments.get(1))
                    {
                        let first_id = self.get_expr_id(first);
                        let second_id = self.get_expr_id(second);
                        self.constraints.push(IntConstraint::MustMatch(
                            first_id,
                            second_id,
                            "assert_eq/assert_ne requires both arguments to have same type"
                                .to_string(),
                        ));
                    }
                    return;
                }

                // TDD FIX: Some(expr) with expected Option<T> - constrain argument to T
                // Handles: Node { children: Some(vec![2, 3]) } where children: Option<Vec<int>>
                let arg_expected_type: Option<Type> = if arguments.len() == 1 {
                    if func_name.as_deref() == Some("Some") || func_name.as_deref() == Some("Option::Some") {
                        return_type.and_then(|ty| self.extract_option_inner_type(ty)).or_else(|| {
                            func_name.as_ref().and_then(|name| self.function_signatures.get(name))
                                .and_then(|(params, _)| params.first())
                                .and_then(|param_ty| self.extract_option_inner_type(param_ty))
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };

                let func_sig = func_name
                    .as_ref()
                    .and_then(|name| self.function_signatures.get(name).cloned());

                if let Some((param_types, _)) = func_sig {
                    for (i, (_label, arg)) in arguments.iter().enumerate() {
                        let arg_ty: Option<&Type> = if i == 0 {
                            arg_expected_type.as_ref().or_else(|| param_types.get(i))
                        } else {
                            param_types.get(i)
                        };
                        self.collect_expression_constraints(arg, arg_ty.or(return_type));
                        if let Some(param_type) = param_types.get(i) {
                            if let Some(int_ty) = self.extract_nested_int_type(param_type) {
                                let arg_id = self.get_expr_id(arg);
                                self.constraints.push(IntConstraint::MustBe(
                                    arg_id,
                                    int_ty,
                                    format!("parameter {} of function", i),
                                ));
                            }
                        }
                    }
                } else {
                    for (i, (_label, arg)) in arguments.iter().enumerate() {
                        let arg_ty: Option<&Type> = if i == 0 {
                            arg_expected_type.as_ref()
                        } else {
                            None
                        };
                        self.collect_expression_constraints(arg, arg_ty.or(return_type));
                        // TDD FIX: Add MustBe for argument when we have int type (e.g. Some(42), Option<Option<int>>)
                        if i == 0 {
                            if let Some(ref at) = arg_expected_type {
                                if let Some(int_ty) = self.extract_nested_int_type(at) {
                                    let arg_id = self.get_expr_id(arg);
                                    self.constraints.push(IntConstraint::MustBe(
                                        arg_id,
                                        int_ty,
                                        "Option::Some argument type".to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Expression::MethodCall { object, method, arguments, .. } => {
                self.collect_expression_constraints(object, return_type);

                let method_sig = self
                    .function_signatures
                    .iter()
                    .filter(|(func_name, (params, _))| {
                        let name_match = func_name.split("::").last() == Some(method.as_str());
                        let param_match =
                            params.len() == arguments.len() + 1 || params.len() == arguments.len();
                        name_match && param_match
                    })
                    .map(|(_, (params, _))| params.clone())
                    .next();

                if let Some(param_types) = method_sig {
                    let param_offset = if param_types.len() == arguments.len() + 1 {
                        1
                    } else {
                        0
                    };
                    for (i, (_label, arg)) in arguments.iter().enumerate() {
                        if let Some(param_type) = param_types.get(i + param_offset) {
                            if let Some(int_ty) = self.extract_int_type(param_type) {
                                let arg_id = self.get_expr_id(arg);
                                self.constraints.push(IntConstraint::MustBe(
                                    arg_id,
                                    int_ty,
                                    format!("{}() parameter {}", method, i),
                                ));
                            }
                        }
                        self.collect_expression_constraints(arg, return_type);
                    }
                } else {
                    for (_label, arg) in arguments {
                        self.collect_expression_constraints(arg, return_type);
                    }
                }

                // TDD FIX: HashMap<K,V>::insert and Vec<T>::push - propagate generic types from receiver
                // Handles: mgr.name_to_id.insert("test", 42) where name_to_id: HashMap<string, int>
                if let Some(receiver_type) = self.infer_type_from_expression(object) {
                    // HashMap<K,V>.insert(K, V) - constrain second argument to V
                    if method == "insert" {
                        if let Some(value_type) = self.extract_map_value_type(&receiver_type) {
                            if let Some(int_ty) = self.extract_int_type(&value_type) {
                                if let Some((_label, value_arg)) = arguments.get(1) {
                                    let value_id = self.get_expr_id(value_arg);
                                    self.constraints.push(IntConstraint::MustBe(
                                        value_id,
                                        int_ty,
                                        format!("HashMap/BTreeMap.insert value type"),
                                    ));
                                }
                            }
                        }
                    }
                    // Vec<T>.push(T) - constrain first argument to T
                    if method == "push" {
                        if let Some(elem_type) = self.extract_vec_element_type(&receiver_type) {
                            if let Some(int_ty) = self.extract_int_type(&elem_type) {
                                if let Some((_label, value_arg)) = arguments.first() {
                                    let value_id = self.get_expr_id(value_arg);
                                    self.constraints.push(IntConstraint::MustBe(
                                        value_id,
                                        int_ty,
                                        format!("Vec.push element type"),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Expression::StructLiteral { name, fields, .. } => {
                // Two-phase to avoid borrow checker: collect field types first, then iterate
                let field_data: Vec<(&'ast Expression<'ast>, IntType, String, Option<Type>)> =
                    self.struct_field_types.get(name).map_or_else(Vec::new, |struct_fields| {
                        fields
                            .iter()
                            .map(|(field_name, field_expr)| {
                                let field_type = struct_fields.get(field_name).cloned();
                                let (int_ty, reason) = field_type
                                    .as_ref()
                                    .and_then(|ft| self.extract_nested_int_type(ft).map(|it| (it, format!("struct {}.{}", name, field_name))))
                                    .unwrap_or((IntType::Unknown, String::new()));
                                (*field_expr, int_ty, reason, field_type)
                            })
                            .collect()
                    });
                for (field_expr, int_ty, reason, _) in &field_data {
                    if *int_ty != IntType::Unknown {
                        let expr_id = self.get_expr_id(field_expr);
                        self.constraints.push(IntConstraint::MustBe(
                            expr_id,
                            *int_ty,
                            reason.clone(),
                        ));
                    }
                }
                // TDD FIX: Pass field type when recursing so nested generics (Option<Vec<int>>) propagate
                for (i, (_field_name, expr)) in fields.iter().enumerate() {
                    let expected_type = field_data.get(i).and_then(|(_, _, _, ft)| ft.as_ref());
                    self.collect_expression_constraints(
                        expr,
                        expected_type.or(return_type),
                    );
                }
            }
            Expression::Binary { left, op, right, .. } => {
                use crate::parser::ast::operators::BinaryOp;
                let left_id = self.get_expr_id(left);
                let right_id = self.get_expr_id(right);

                match op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
                    | BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor
                    | BinaryOp::Shl | BinaryOp::Shr => {
                        self.constraints.push(IntConstraint::MustMatch(
                            left_id,
                            right_id,
                            format!("binary op {:?}", op),
                        ));
                        
                        // TDD FIX: Propagate type from left side (identifier or field access)
                        let left_int_ty = match left {
                            Expression::Identifier { name, .. } => {
                                self.var_types.get(name)
                                    .or_else(|| self.const_types.get(name))
                                    .and_then(|ty| self.extract_int_type(ty))
                            }
                            Expression::FieldAccess { object, field, .. } => {
                                // Handle self.field or obj.field
                                if let Expression::Identifier { ref name, .. } = **object {
                                    // Get struct name from variable or self
                                    let struct_name = if name == "self" {
                                        self.current_impl_type.as_ref()
                                    } else {
                                        self.var_types.get(name.as_str())
                                            .and_then(|ty| {
                                                if let Type::Custom(sname) = ty {
                                                    Some(sname)
                                                } else {
                                                    None
                                                }
                                            })
                                    };
                                    
                                    struct_name.and_then(|sname| {
                                        self.struct_field_types.get(sname.as_str())
                                            .and_then(|fields| fields.get(field.as_str()))
                                            .and_then(|field_ty| self.extract_int_type(field_ty))
                                    })
                                } else {
                                    None
                                }
                            }
                            _ => {
                                // TDD FIX: Fallback to expression type inference for complex expressions
                                self.infer_type_from_expression(left)
                                    .and_then(|ty| self.extract_int_type(&ty))
                            }
                        };
                        
                        if let Some(int_ty) = left_int_ty {
                            self.constraints.push(IntConstraint::MustBe(
                                right_id,
                                int_ty,
                                format!("LHS has type {:?}", int_ty),
                            ));
                        }
                        
                        // TDD FIX: Propagate type from right side (identifier or field access)
                        let right_int_ty = match right {
                            Expression::Identifier { name, .. } => {
                                self.var_types.get(name)
                                    .or_else(|| self.const_types.get(name))
                                    .and_then(|ty| self.extract_int_type(ty))
                            }
                            Expression::FieldAccess { object, field, .. } => {
                                if let Expression::Identifier { ref name, .. } = **object {
                                    let struct_name = if name == "self" {
                                        self.current_impl_type.as_ref()
                                    } else {
                                        self.var_types.get(name.as_str())
                                            .and_then(|ty| {
                                                if let Type::Custom(sname) = ty {
                                                    Some(sname)
                                                } else {
                                                    None
                                                }
                                            })
                                    };
                                    
                                    struct_name.and_then(|sname| {
                                        self.struct_field_types.get(sname.as_str())
                                            .and_then(|fields| fields.get(field.as_str()))
                                            .and_then(|field_ty| self.extract_int_type(field_ty))
                                    })
                                } else {
                                    None
                                }
                            }
                            _ => {
                                // TDD FIX: Fallback to expression type inference
                                self.infer_type_from_expression(right)
                                    .and_then(|ty| self.extract_int_type(&ty))
                            }
                        };
                        
                        if let Some(int_ty) = right_int_ty {
                            self.constraints.push(IntConstraint::MustBe(
                                left_id,
                                int_ty,
                                format!("RHS has type {:?}", int_ty),
                            ));
                        }
                    }
                    BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le
                    | BinaryOp::Gt | BinaryOp::Ge => {
                        self.constraints.push(IntConstraint::MustMatch(
                            left_id,
                            right_id,
                            format!("comparison {:?}", op),
                        ));
                        
                        // TDD FIX: Propagate type from left side for comparisons
                        // First try pattern matching for direct field access/identifier
                        let left_int_ty = match left {
                            Expression::Identifier { name, .. } => {
                                self.var_types.get(name)
                                    .or_else(|| self.const_types.get(name))
                                    .and_then(|ty| self.extract_int_type(ty))
                            }
                            Expression::FieldAccess { object, field, .. } => {
                                if let Expression::Identifier { ref name, .. } = **object {
                                    let struct_name = if name == "self" {
                                        self.current_impl_type.as_ref()
                                    } else {
                                        self.var_types.get(name.as_str())
                                            .and_then(|ty| {
                                                if let Type::Custom(sname) = ty {
                                                    Some(sname)
                                                } else {
                                                    None
                                                }
                                            })
                                    };
                                    
                                    struct_name.and_then(|sname| {
                                        self.struct_field_types.get(sname.as_str())
                                            .and_then(|fields| fields.get(field.as_str()))
                                            .and_then(|field_ty| self.extract_int_type(field_ty))
                                    })
                                } else {
                                    None
                                }
                            }
                            _ => {
                                // TDD FIX: Fallback to expression type inference for complex expressions
                                // This handles cases like (self.count % 60) where the result type matters
                                self.infer_type_from_expression(left)
                                    .and_then(|ty| self.extract_int_type(&ty))
                            }
                        };
                        
                        if let Some(int_ty) = left_int_ty {
                            self.constraints.push(IntConstraint::MustBe(
                                right_id,
                                int_ty,
                                format!("comparison LHS has type {:?}", int_ty),
                            ));
                        }
                        
                        // TDD FIX: Propagate type from right side for comparisons
                        let right_int_ty = match right {
                            Expression::Identifier { name, .. } => {
                                self.var_types.get(name)
                                    .or_else(|| self.const_types.get(name))
                                    .and_then(|ty| self.extract_int_type(ty))
                            }
                            Expression::FieldAccess { object, field, .. } => {
                                if let Expression::Identifier { ref name, .. } = **object {
                                    let struct_name = if name == "self" {
                                        self.current_impl_type.as_ref()
                                    } else {
                                        self.var_types.get(name.as_str())
                                            .and_then(|ty| {
                                                if let Type::Custom(sname) = ty {
                                                    Some(sname)
                                                } else {
                                                    None
                                                }
                                            })
                                    };
                                    
                                    struct_name.and_then(|sname| {
                                        self.struct_field_types.get(sname.as_str())
                                            .and_then(|fields| fields.get(field.as_str()))
                                            .and_then(|field_ty| self.extract_int_type(field_ty))
                                    })
                                } else {
                                    None
                                }
                            }
                            _ => {
                                // TDD FIX: Fallback to expression type inference
                                self.infer_type_from_expression(right)
                                    .and_then(|ty| self.extract_int_type(&ty))
                            }
                        };
                        
                        if let Some(int_ty) = right_int_ty {
                            self.constraints.push(IntConstraint::MustBe(
                                left_id,
                                int_ty,
                                format!("comparison RHS has type {:?}", int_ty),
                            ));
                        }
                    }
                    _ => {}
                }

                self.collect_expression_constraints(left, return_type);
                self.collect_expression_constraints(right, return_type);
            }
            Expression::Cast { expr: inner, type_, .. } => {
                // Cast converts between types - do NOT constrain operand to match target.
                // e.g. (x as usize) is valid when x is i32, u32, etc.
                self.collect_expression_constraints(inner, return_type);
                // Constrain the cast RESULT to the target type (fixes return type conflicts)
                if let Some(int_ty) = self.extract_int_type(type_) {
                    let cast_id = self.get_expr_id(expr);
                    self.constraints.push(IntConstraint::MustBe(
                        cast_id,
                        int_ty,
                        "cast target type".to_string(),
                    ));
                }
            }
            Expression::Tuple { elements, .. } => {
                if let Some(Type::Tuple(tuple_types)) = return_type {
                    for (i, elem) in elements.iter().enumerate() {
                        if let Some(elem_type) = tuple_types.get(i) {
                            self.collect_expression_constraints(elem, Some(elem_type));
                            if let Some(int_ty) = self.extract_int_type(elem_type) {
                                let elem_id = self.get_expr_id(elem);
                                self.constraints.push(IntConstraint::MustBe(
                                    elem_id,
                                    int_ty,
                                    format!("tuple element {}", i),
                                ));
                            }
                        } else {
                            self.collect_expression_constraints(elem, None);
                        }
                    }
                } else {
                    for elem in elements {
                        self.collect_expression_constraints(elem, None);
                    }
                }
            }
            Expression::Block { statements, .. } => {
                for stmt in statements {
                    self.collect_statement_constraints(stmt, return_type);
                }
            }
            Expression::FieldAccess { object, .. } => {
                self.collect_expression_constraints(object, return_type);
            }
            Expression::Index { object, index, .. } => {
                // TDD FIX: Vec<T>[idx] should infer result as T, not usize
                // Extract Vec element type and constrain Index result accordingly
                if let Some(object_type) = self.infer_type_from_expression(object) {
                    if let Some(elem_type) = self.extract_vec_element_type(&object_type) {
                        if let Some(int_ty) = self.extract_nested_int_type(&elem_type) {
                            let expr_id = self.get_expr_id(expr);
                            self.constraints.push(IntConstraint::MustBe(
                                expr_id,
                                int_ty,
                                format!("Vec<{:?}> element type", int_ty),
                            ));
                        }
                    }
                }
                
                // Don't pass return_type to object - Vec has its own type
                self.collect_expression_constraints(object, None);
                
                // TDD FIX: Array indices must be usize in Rust
                let index_id = self.get_expr_id(index);
                self.constraints.push(IntConstraint::MustBe(
                    index_id,
                    IntType::Usize,
                    "array index must be usize".to_string(),
                ));
                self.collect_expression_constraints(index, None);
            }
            Expression::Array { elements, .. } => {
                // TDD FIX: [a, b, c] with expected Vec<T> - constrain elements to T
                if let Some(elem_type) = return_type.and_then(|ty| self.extract_vec_element_type(ty)) {
                    for elem in elements {
                        self.collect_expression_constraints(elem, Some(&elem_type));
                        if let Some(int_ty) = self.extract_nested_int_type(&elem_type) {
                            let elem_id = self.get_expr_id(elem);
                            self.constraints.push(IntConstraint::MustBe(
                                elem_id,
                                int_ty,
                                "array/vec element type".to_string(),
                            ));
                        }
                    }
                } else {
                    for elem in elements {
                        self.collect_expression_constraints(elem, return_type);
                    }
                }
            }
            Expression::MacroInvocation { name, args, is_repeat, .. } => {
                // TDD FIX: assert_eq!/assert_ne! - both args must have same int type
                if (*name == "assert_eq" || *name == "assert_ne") && args.len() >= 2 {
                    for arg in args {
                        self.collect_expression_constraints(arg, return_type);
                    }
                    let first_id = self.get_expr_id(&args[0]);
                    let second_id = self.get_expr_id(&args[1]);
                    self.constraints.push(IntConstraint::MustMatch(
                        first_id,
                        second_id,
                        format!("{}! requires both arguments to have same type", name),
                    ));
                    return;
                }
                // TDD FIX: vec![a, b, c] with expected Vec<T> - constrain elements to T
                if *name == "vec" && !*is_repeat {
                    if let Some(elem_type) = return_type.and_then(|ty| self.extract_vec_element_type(ty)) {
                        for arg in args {
                            self.collect_expression_constraints(arg, Some(&elem_type));
                            if let Some(int_ty) = self.extract_nested_int_type(&elem_type) {
                                let arg_id = self.get_expr_id(arg);
                                self.constraints.push(IntConstraint::MustBe(
                                    arg_id,
                                    int_ty,
                                    "vec! element type".to_string(),
                                ));
                            }
                        }
                    } else {
                        for arg in args {
                            self.collect_expression_constraints(arg, return_type);
                        }
                    }
                } else {
                    for arg in args {
                        self.collect_expression_constraints(arg, return_type);
                    }
                }
            }
            _ => {}
        }
    }

    fn extract_int_type(&self, ty: &Type) -> Option<IntType> {
        self.extract_nested_int_type(ty)
    }

    /// Extract int type from nested generics: Option<Vec<int>>, Vec<Option<int>>, etc.
    fn extract_nested_int_type(&self, ty: &Type) -> Option<IntType> {
        match ty {
            Type::Int32 => Some(IntType::I32),
            Type::Int => Some(IntType::I64),
            Type::Uint => Some(IntType::U64),
            Type::Custom(name) => match name.as_str() {
                "i32" => Some(IntType::I32),
                "i64" => Some(IntType::I64),
                "u32" => Some(IntType::U32),
                "u64" => Some(IntType::U64),
                "usize" => Some(IntType::Usize),
                "isize" => Some(IntType::Isize),
                "u8" => Some(IntType::U8),
                "i8" => Some(IntType::I8),
                "u16" => Some(IntType::U16),
                "i16" => Some(IntType::I16),
                _ => None,
            },
            Type::Tuple(types) => {
                for t in types {
                    if let Some(it) = self.extract_nested_int_type(t) {
                        return Some(it);
                    }
                }
                None
            }
            Type::Vec(inner) => self.extract_nested_int_type(inner),
            Type::Array(inner, _) => self.extract_nested_int_type(inner),
            Type::Parameterized(name, args) if name == "Vec" && !args.is_empty() => {
                self.extract_nested_int_type(&args[0])
            }
            Type::Parameterized(name, args) if name == "Option" && !args.is_empty() => {
                self.extract_nested_int_type(&args[0])
            }
            Type::Parameterized(name, args) if name == "HashMap" && args.len() >= 2 => {
                self.extract_nested_int_type(&args[1])
            }
            Type::Parameterized(name, args) if name == "BTreeMap" && args.len() >= 2 => {
                self.extract_nested_int_type(&args[1])
            }
            Type::Reference(inner) | Type::MutableReference(inner) => self.extract_nested_int_type(inner),
            Type::Option(inner) => self.extract_nested_int_type(inner),
            Type::Result(ok, err) => self.extract_nested_int_type(ok).or_else(|| self.extract_nested_int_type(err)),
            _ => None,
        }
    }

    /// Extract inner type T from Option<T> (handles Option, Parameterized, Reference)
    fn extract_option_inner_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Option(inner) => Some((**inner).clone()),
            Type::Parameterized(name, args) if name == "Option" && !args.is_empty() => {
                Some(args[0].clone())
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.extract_option_inner_type(inner)
            }
            _ => None,
        }
    }

    /// Infer Type from an expression (for receiver type resolution in method calls)
    /// TDD: Enables HashMap<K,V>.insert and Vec<T>.push generic type propagation
    fn infer_type_from_expression<'ast>(&self, expr: &Expression<'ast>) -> Option<Type> {
        match expr {
            Expression::StructLiteral { name, .. } => Some(Type::Custom(name.clone())),
            Expression::Call { function, .. } => {
                let func_name = match function {
                    Expression::Identifier { name, .. } => Some(name.clone()),
                    Expression::FieldAccess { object, field, .. } => {
                        if let Expression::Identifier { name: type_name, .. } = object {
                            Some(format!("{}::{}", type_name, field))
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                func_name.and_then(|name| {
                    self.function_signatures
                        .get(&name)
                        .and_then(|(_, ret)| ret.clone())
                })
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
                self.struct_field_types
                    .get(&struct_name)
                    .and_then(|fields| fields.get(field))
                    .cloned()
            }
            Expression::Index { object, .. } => {
                let object_type = self.infer_type_from_expression(object)?;
                self.extract_vec_element_type(&object_type)
            }
            Expression::Cast { type_, .. } => Some(type_.clone()),
            Expression::Binary { left, op, .. } => {
                // TDD FIX: Binary operations return the type of their operands
                // For arithmetic (Add, Sub, Mul, Div, Mod): result type = operand type
                // For comparison (Eq, Lt, Gt, etc.): result type = bool
                use crate::parser::ast::operators::BinaryOp;
                match op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
                    | BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor
                    | BinaryOp::Shl | BinaryOp::Shr => {
                        // Arithmetic: result has same type as operands
                        self.infer_type_from_expression(left)
                    }
                    BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le
                    | BinaryOp::Gt | BinaryOp::Ge => {
                        // Comparison: result is bool
                        Some(Type::Bool)
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        // Logical: result is bool
                        Some(Type::Bool)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Extract value type V from HashMap<K,V> or BTreeMap<K,V>
    fn extract_map_value_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Parameterized(name, args)
                if (name == "HashMap" || name == "BTreeMap") && args.len() >= 2 =>
            {
                Some(args[1].clone())
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.extract_map_value_type(inner)
            }
            _ => None,
        }
    }

    /// Extract element type T from Vec<T>
    fn extract_vec_element_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Vec(inner) => Some((**inner).clone()),
            Type::Parameterized(name, args) if name == "Vec" && !args.is_empty() => {
                Some(args[0].clone())
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.extract_vec_element_type(inner)
            }
            _ => None,
        }
    }

    fn get_expr_id<'ast>(&mut self, expr: &Expression<'ast>) -> ExprId {
        let (line, col) = expr
            .location()
            .map(|loc| (loc.line, loc.column))
            .unwrap_or((0, 0));

        let cache_key = (self.current_file_id, line, col);
        if line > 0 {
            if let Some(&cached_id) = self.expr_id_cache.get(&cache_key) {
                return cached_id;
            }
        }

        let seq_id = self.next_seq_id;
        self.next_seq_id += 1;

        let expr_id = ExprId {
            seq_id,
            file_id: self.current_file_id,
            line,
            col,
        };

        if line > 0 {
            self.expr_id_cache.insert(cache_key, expr_id);
        }

        expr_id
    }

    fn solve_constraints(&mut self) {
        for constraint in &self.constraints.clone() {
            if let IntConstraint::MustBe(expr_id, int_ty, _) = constraint {
                if self.inferred_types.get(expr_id).is_none() {
                    self.inferred_types.insert(*expr_id, *int_ty);
                }
            }
        }

        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 100;

        while changed && iterations < MAX_ITERATIONS {
            changed = false;
            iterations += 1;

            for constraint in self.constraints.clone() {
                match constraint {
                    IntConstraint::MustBe(expr_id, int_ty, reason) => {
                        let current = self.inferred_types.get(&expr_id).copied();
                        match current {
                            Some(other) if other != int_ty && other != IntType::Unknown => {
                                if int_ty != IntType::Unknown {
                                    // TDD FIX: Only emit error if NOT a safe implicit cast
                                    // DON'T modify inferred types - let codegen insert casts
                                    if !is_safe_implicit_cast(other, int_ty) {
                                        let file_path = self.id_to_file_name.get(&expr_id.file_id)
                                            .map(|s| s.as_str()).unwrap_or("?");
                                        self.errors.push(format!(
                                            "{}:{}:{}: Type conflict: must be {:?} ({}) but was {:?}",
                                            file_path, expr_id.line, expr_id.col, int_ty, reason, other
                                        ));
                                    }
                                    // else: Safe cast - silently allow it
                                }
                            }
                            Some(IntType::Unknown) | None => {
                                let to_insert = if int_ty == IntType::Unknown {
                                    IntType::I32
                                } else {
                                    int_ty
                                };
                                self.inferred_types.insert(expr_id, to_insert);
                                changed = true;
                            }
                            _ => {}
                        }
                    }
                    IntConstraint::MustMatch(id1, id2, reason) => {
                        let t1 = self.inferred_types.get(&id1).copied();
                        let t2 = self.inferred_types.get(&id2).copied();

                        match (t1, t2) {
                            (Some(a), Some(b)) if a != b && a != IntType::Unknown && b != IntType::Unknown => {
                                // TDD FIX: Only emit error if NOT a safe implicit cast in either direction
                                // DON'T modify inferred types - let codegen insert casts
                                if !is_safe_implicit_cast(a, b) && !is_safe_implicit_cast(b, a) {
                                    let file_path = self.id_to_file_name.get(&id1.file_id)
                                        .map(|s| s.as_str()).unwrap_or("?");
                                    self.errors.push(format!(
                                        "{}:{}:{}: Type mismatch {}: {:?} vs {:?} ({})",
                                        file_path, id1.line, id1.col, reason, a, b, reason
                                    ));
                                }
                                // else: Safe cast in at least one direction - silently allow it
                            }
                            (Some(concrete), None | Some(IntType::Unknown)) => {
                                let to_use = if concrete == IntType::Unknown {
                                    IntType::I32
                                } else {
                                    concrete
                                };
                                self.inferred_types.insert(id2, to_use);
                                changed = true;
                            }
                            (None | Some(IntType::Unknown), Some(concrete)) => {
                                let to_use = if concrete == IntType::Unknown {
                                    IntType::I32
                                } else {
                                    concrete
                                };
                                self.inferred_types.insert(id1, to_use);
                                changed = true;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    /// Get inferred integer type for expression (O(1) cache lookup via ExprId)
    pub fn get_int_type<'ast>(&self, expr: &Expression<'ast>) -> IntType {
        let (file, line, col) = expr
            .location()
            .map(|loc| {
                (
                    self.file_name_to_id
                        .get(&loc.file.to_string_lossy().to_string())
                        .copied()
                        .unwrap_or(0),
                    loc.line,
                    loc.column,
                )
            })
            .unwrap_or((0, 0, 0));

        for (expr_id, int_ty) in &self.inferred_types {
            if expr_id.file_id == file && expr_id.line == line && expr_id.col == col {
                return *int_ty;
            }
        }

        IntType::Unknown
    }
}
