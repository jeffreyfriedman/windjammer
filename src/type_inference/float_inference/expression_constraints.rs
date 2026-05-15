impl FloatInference {
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

}
