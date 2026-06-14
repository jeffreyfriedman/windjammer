//! Struct fields, collections, index assignment, and enum-variant “stores” a value.

use crate::parser::*;

use super::Analyzer;

impl<'ast> Analyzer<'ast> {
    /// Check if an expression stores a parameter by value.
    /// Matches direct identifier use, wrapping in Some/Ok/Err, enum variant constructors,
    /// tuples, and struct literals containing the parameter.
    pub(crate) fn expression_stores_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name: fn_name, .. } = &**function {
                    let is_constructor =
                        matches!(fn_name.as_str(), "Some" | "Ok" | "Err") || fn_name.contains("::");
                    if is_constructor {
                        return arguments
                            .iter()
                            .any(|(_label, arg)| self.expression_stores_identifier(name, arg));
                    }
                }
                false
            }
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|el| self.expression_stores_identifier(name, el)),
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| self.expression_stores_identifier(name, v)),
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|el| self.expression_stores_identifier(name, el)),
            _ => false,
        }
    }

    pub(crate) fn is_stored(&self, name: &str, statements: &[&'ast Statement<'ast>]) -> bool {
        // Check if the parameter is stored in a struct field or collection
        for stmt in statements {
            match stmt {
                Statement::Let {
                    value: Expression::StructLiteral { fields, .. },
                    ..
                } => {
                    for (_field_name, field_expr) in fields {
                        if self.expression_stores_identifier(name, field_expr) {
                            return true;
                        }
                    }
                }
                Statement::Return {
                    value: Some(Expression::StructLiteral { fields, .. }),
                    ..
                } => {
                    // Check if parameter is stored (moved) in a returned struct literal.
                    // Field access like `Data { value: d.value }` borrows a field; it does not
                    // store the whole parameter and should not force Owned.
                    for (_, field_expr) in fields {
                        if self.expression_stores_identifier(name, field_expr) {
                            return true;
                        }
                    }
                }
                Statement::Expression {
                    expr: Expression::StructLiteral { fields, .. },
                    ..
                } => {
                    for (_, field_expr) in fields {
                        if self.expression_stores_identifier(name, field_expr) {
                            return true;
                        }
                    }
                }
                Statement::Assignment {
                    target: Expression::FieldAccess { object, .. },
                    value,
                    ..
                } => {
                    // Check if the parameter is assigned to a struct field, either directly
                    // or wrapped in Some/Enum constructors/tuples.
                    //
                    // Direct: obj.field = param
                    // Wrapped: obj.field = Some(param)
                    // Enum: obj.field = Enum::Variant(param)
                    if matches!(&**object, Expression::Identifier { .. }) {
                        if self.expression_stores_identifier(name, value) {
                            return true;
                        }
                    }
                }
                // Check if parameter is stored via index assignment
                // e.g., self.slots[i] = item
                // e.g., self.slots[i] = Some(ItemStack::new(item, qty))
                Statement::Assignment {
                    target: Expression::Index { .. },
                    value,
                    ..
                } => {
                    if self.expression_stores_identifier(name, value) {
                        return true;
                    }
                }
                Statement::Expression {
                    expr:
                        Expression::MethodCall {
                            object,
                            method,
                            arguments,
                            ..
                        },
                    ..
                } => {
                    let is_storage_method =
                        super::super::stdlib_method_traits::is_storage_method(method);

                    if is_storage_method {
                        // Check for storage method calls on ANY object:
                        // - self.field.push(param)
                        // - self.field.push((param, other))  ← tuple wrapping
                        // - self.field.push(Enum::Variant(param))  ← enum wrapping
                        // - local_var.push(param)
                        let is_on_field_or_var =
                            matches!(&**object, Expression::FieldAccess { .. })
                                || matches!(&**object, Expression::Identifier { .. });

                        if is_on_field_or_var {
                            for (_label, arg) in arguments {
                                if self.expression_stores_identifier(name, arg) {
                                    return true;
                                }
                            }
                        }

                        // TDD FIX: Also check for method calls on LOCAL struct fields: local_var.field.push(param)
                        // e.g., choice.conditions.push(condition) where choice is a local variable
                        if let Expression::FieldAccess {
                            object: field_obj, ..
                        } = &**object
                        {
                            // Check if it's a local variable (not self)
                            if matches!(&**field_obj, Expression::Identifier { name: id, .. } if id != "self")
                            {
                                for (_label, arg) in arguments {
                                    if matches!(arg, Expression::Identifier { name: id, .. } if id == name)
                                    {
                                        return true;
                                    }
                                }
                            }
                        }
                    }

                    // Also check for method calls on local variables: props.push(Property { name, ... })
                    // The parameter might be used in a struct literal passed as an argument
                    for (_label, arg) in arguments {
                        if let Expression::StructLiteral { fields, .. } = arg {
                            for (_field_name, field_expr) in fields {
                                if self.expression_uses_identifier(name, field_expr) {
                                    return true;
                                }
                            }
                        }
                    }

                    // Check for push/insert with a constructor call: vec.push(Node::new(param, ...))
                    // The parameter is being stored if passed to a constructor that stores it
                    if is_storage_method {
                        for (_label, arg) in arguments {
                            if let Expression::Call {
                                arguments: call_args,
                                ..
                            } = arg
                            {
                                for (_call_label, call_arg) in call_args {
                                    if matches!(call_arg, Expression::Identifier { name: id, .. } if id == name)
                                    {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
                // Recursively check if/else bodies for storage operations
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_stored(name, then_block) {
                        return true;
                    }
                    if let Some(else_stmts) = else_block {
                        if self.is_stored(name, else_stmts) {
                            return true;
                        }
                    }
                }
                // Recursively check loop bodies
                Statement::While { body, .. } | Statement::For { body, .. } => {
                    if self.is_stored(name, body) {
                        return true;
                    }
                }
                // General case: check any statement for enum variant constructors
                // that consume the parameter. Covers patterns like:
                //   let x = Func(EnumType::Variant(param, ...))
                //   let x = Func(format!(..., param), &EnumType::Variant(param, ...))
                _ => {
                    if self.stmt_has_enum_variant_consuming(name, stmt) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if a statement contains an enum variant constructor that consumes a parameter.
    /// Recursively scans all expressions within the statement.
    pub(crate) fn stmt_has_enum_variant_consuming(
        &self,
        name: &str,
        stmt: &Statement<'ast>,
    ) -> bool {
        match stmt {
            Statement::Let { value, .. } => self.expr_has_enum_variant_consuming(name, value),
            Statement::Expression { expr, .. } => self.expr_has_enum_variant_consuming(name, expr),
            Statement::Return {
                value: Some(expr), ..
            } => self.expr_has_enum_variant_consuming(name, expr),
            Statement::Assignment { value, .. } => {
                self.expr_has_enum_variant_consuming(name, value)
            }
            _ => false,
        }
    }

    /// Recursively check if an expression contains an enum variant constructor
    /// (function call where name contains "::") that has the parameter as a direct argument.
    pub(crate) fn expr_has_enum_variant_consuming(
        &self,
        name: &str,
        expr: &Expression<'ast>,
    ) -> bool {
        match expr {
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                let is_enum_variant = if let Expression::Identifier { name: fn_name, .. } = function
                {
                    Self::looks_like_enum_variant_constructor(fn_name)
                } else if let Expression::FieldAccess { field, .. } = function {
                    Self::looks_like_enum_variant_constructor(field)
                } else {
                    false
                };

                if is_enum_variant {
                    for (_label, arg) in arguments {
                        if matches!(arg, Expression::Identifier { name: id, .. } if id == name) {
                            return true;
                        }
                    }
                }

                // Recurse into all arguments
                for (_label, arg) in arguments {
                    if self.expr_has_enum_variant_consuming(name, arg) {
                        return true;
                    }
                }
                // Recurse into function expression
                self.expr_has_enum_variant_consuming(name, function)
            }
            Expression::Unary { operand, .. } => {
                self.expr_has_enum_variant_consuming(name, operand)
            }
            Expression::Block { statements, .. } => {
                for s in statements {
                    if self.stmt_has_enum_variant_consuming(name, s) {
                        return true;
                    }
                }
                false
            }
            Expression::Tuple { elements, .. } => {
                for el in elements {
                    if self.expr_has_enum_variant_consuming(name, el) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Check if a qualified name like "Type::Variant" looks like an enum variant constructor
    /// rather than a static method call. Enum variants use PascalCase after "::"
    /// (e.g., Option::Some, Color::Custom), while methods use snake_case
    /// (e.g., FpsCamera::collides_aabb, Vec3::new).
    pub(crate) fn looks_like_enum_variant_constructor(qualified_name: &str) -> bool {
        if let Some(pos) = qualified_name.rfind("::") {
            let after_colons = &qualified_name[pos + 2..];
            after_colons
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
        } else {
            false
        }
    }

    pub(crate) fn struct_field_is_text_type(&self, struct_name: &str, field_name: &str) -> bool {
        let lookup = |name: &str| {
            self.global_struct_field_types
                .get(name)
                .and_then(|fields| fields.get(field_name))
        };
        lookup(struct_name)
            .or_else(|| struct_name.rsplit("::").next().and_then(lookup))
            .is_some_and(Self::is_windjammer_text_param_type)
    }

    /// True when the parameter is stored only via struct literals into `string` fields
    /// (e.g. `User { name }`), where codegen emits `.to_string()` at the assignment site.
    /// Field assignment (`self.name = name`) and collection push are excluded — those use owned `String`.
    pub(crate) fn is_stored_via_text_struct_fields_only(
        &self,
        param_name: &str,
        body: &[&Statement],
    ) -> bool {
        if !self.stores_in_text_struct_literals(param_name, body) {
            return false;
        }
        !self.has_non_text_struct_field_storage(param_name, body)
    }

    /// True when the parameter appears in a struct literal field that stores into a `string` field.
    fn stores_in_text_struct_literals(&self, param_name: &str, statements: &[&Statement]) -> bool {
        for stmt in statements {
            if self.stmt_stores_in_text_struct_literals(param_name, stmt) {
                return true;
            }
        }
        false
    }

    fn stmt_stores_in_text_struct_literals(&self, param_name: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::Let {
                value: Expression::StructLiteral { name, fields, .. },
                ..
            }
            | Statement::Return {
                value: Some(Expression::StructLiteral { name, fields, .. }),
                ..
            }
            | Statement::Expression {
                expr: Expression::StructLiteral { name, fields, .. },
                ..
            } => fields.iter().any(|(field_name, fv)| {
                self.expression_stores_identifier(param_name, fv)
                    && self.struct_field_is_text_type(name, field_name)
            }),
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                self.stores_in_text_struct_literals(param_name, then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|b| self.stores_in_text_struct_literals(param_name, b))
            }
            Statement::While { body, .. } | Statement::For { body, .. } => {
                self.stores_in_text_struct_literals(param_name, body)
            }
            _ => false,
        }
    }

    fn has_non_text_struct_field_storage(
        &self,
        param_name: &str,
        statements: &[&Statement],
    ) -> bool {
        for stmt in statements {
            if self.stmt_has_non_text_struct_field_storage(param_name, stmt) {
                return true;
            }
        }
        false
    }

    fn stmt_has_non_text_struct_field_storage(&self, param_name: &str, stmt: &Statement) -> bool {
        match stmt {
            Statement::Return {
                value: Some(expr), ..
            }
            | Statement::Expression { expr, .. }
            | Statement::Let { value: expr, .. } => {
                self.expr_has_non_text_struct_field_storage(param_name, expr)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                self.has_non_text_struct_field_storage(param_name, then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|b| self.has_non_text_struct_field_storage(param_name, b))
            }
            Statement::While { body, .. } | Statement::For { body, .. } => {
                self.has_non_text_struct_field_storage(param_name, body)
            }
            _ => self.stmt_has_enum_variant_consuming(param_name, stmt),
        }
    }

    fn expr_has_non_text_struct_field_storage(&self, param_name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::StructLiteral { name, fields, .. } => {
                fields.iter().any(|(field_name, fv)| {
                    if !self.expression_stores_identifier(param_name, fv) {
                        return false;
                    }
                    // Shorthand `User { name }` — param moves into field; codegen coerces &str → String.
                    if self.expr_is_param_or_ref_to_param(param_name, fv) {
                        return false;
                    }
                    !self.struct_field_is_text_type(name, field_name)
                })
            }
            Expression::MethodCall {
                method, arguments, ..
            } => {
                if super::super::stdlib_method_traits::is_storage_method(method) {
                    return arguments
                        .iter()
                        .any(|(_, arg)| self.expression_stores_identifier(param_name, arg));
                }
                arguments
                    .iter()
                    .any(|(_, arg)| self.expr_has_non_text_struct_field_storage(param_name, arg))
            }
            Expression::Call { arguments, .. } => arguments
                .iter()
                .any(|(_, arg)| self.expr_has_non_text_struct_field_storage(param_name, arg)),
            Expression::Block { statements, .. } => {
                self.has_non_text_struct_field_storage(param_name, statements)
            }
            _ => self.expression_stores_identifier(param_name, expr),
        }
    }
}
