//! Struct fields, collections, index assignment, and enum-variant “stores” a value.

use crate::parser::*;

use super::super::{Analyzer, FunctionSignature, OwnershipMode, SignatureRegistry};

impl<'ast> Analyzer<'ast> {
    /// Extract a qualified callee name from a call target expression.
    fn extract_call_target_name(function: &Expression) -> Option<String> {
        match function {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, field, .. } => {
                if let Some(prefix) = Self::extract_call_target_name(object) {
                    Some(format!("{}::{}", prefix, field))
                } else if let Expression::Identifier { name, .. } = &**object {
                    Some(format!("{}::{}", name, field))
                } else {
                    Some(field.clone())
                }
            }
            _ => None,
        }
    }

    /// Language-level payload stores: `Some`/`Ok`/`Err` and PascalCase enum variant constructors.
    pub(crate) fn is_language_level_call_payload_store(function: &Expression) -> bool {
        match function {
            Expression::Identifier { name, .. } => {
                crate::type_classification::is_language_level_payload_call_name(name)
            }
            Expression::FieldAccess { field, .. } => {
                crate::type_classification::is_language_level_payload_method(field)
            }
            _ => false,
        }
    }

    /// `Type::Variant(...)` lowers as MethodCall with a PascalCase method name.
    pub(crate) fn is_language_level_method_payload_store(method: &str) -> bool {
        crate::type_classification::is_language_level_payload_method(method)
    }

    fn resolve_callee_signature<'a>(
        name: &str,
        arg_count: usize,
        has_receiver: bool,
        registry: &'a SignatureRegistry,
    ) -> Option<&'a FunctionSignature> {
        registry
            .get_signature(name)
            .or_else(|| registry.lookup_method(name))
            .or_else(|| registry.find_signature_by_name_and_arg_count(name, arg_count))
            .or_else(|| {
                if has_receiver {
                    registry.find_signature_ending_with(name)
                } else {
                    None
                }
            })
    }

    /// True when a return type denotes a composite that stores owned arguments (struct/enum).
    fn return_type_stores_owned_payload(ret: &Type) -> bool {
        match ret {
            Type::Custom(name) => !matches!(
                name.as_str(),
                "string"
                    | "str"
                    | "String"
                    | "bool"
                    | "i32"
                    | "i64"
                    | "u32"
                    | "u64"
                    | "f32"
                    | "f64"
                    | "usize"
                    | "isize"
                    | "char"
                    | "()"
            ),
            Type::Parameterized(base, _) => Self::return_type_stores_owned_payload_base(base),
            Type::Option(inner) | Type::Result(inner, _) => {
                Self::return_type_stores_owned_payload(inner)
            }
            _ => false,
        }
    }

    fn return_type_stores_owned_payload_base(base: &str) -> bool {
        !crate::type_classification::is_stdlib_wrapper_type_base(base)
    }

    fn same_payload_type(a: &Type, b: &Type) -> bool {
        match (a, b) {
            (Type::Custom(na), Type::Custom(nb)) => na == nb,
            (Type::String, Type::String) => true,
            (Type::Int, Type::Int)
            | (Type::Int32, Type::Int32)
            | (Type::Float, Type::Float)
            | (Type::Bool, Type::Bool) => true,
            (Type::Vec(i_a), Type::Vec(i_b)) | (Type::Option(i_a), Type::Option(i_b)) => {
                Self::same_payload_type(i_a, i_b)
            }
            (Type::Parameterized(n_a, a_a), Type::Parameterized(n_b, b_a)) => {
                n_a == n_b
                    && a_a.len() == b_a.len()
                    && a_a
                        .iter()
                        .zip(b_a.iter())
                        .all(|(x, y)| Self::same_payload_type(x, y))
            }
            _ => false,
        }
    }

    fn formal_stores_into_composite_return(sig: &FunctionSignature, arg_index: usize) -> bool {
        let Some(formal_ty) = sig.formal_param_type_for_arg(arg_index) else {
            return false;
        };
        let Some(ret) = sig.return_type.as_ref() else {
            return false;
        };
        if !Self::return_type_stores_owned_payload(ret) {
            return false;
        }
        !Self::same_payload_type(formal_ty, ret)
    }

    /// True when call argument `arg_index` is stored into an owned formal (signature-driven).
    pub(crate) fn call_argument_stores_owned_payload(
        function: &Expression,
        arg_index: usize,
        arg_count: usize,
        registry: &SignatureRegistry,
    ) -> bool {
        if Self::is_language_level_call_payload_store(function) {
            return true;
        }
        let Some(name) = Self::extract_call_target_name(function) else {
            return false;
        };
        let Some(sig) = Self::resolve_callee_signature(&name, arg_count, false, registry) else {
            return false;
        };
        matches!(
            sig.param_ownership_for_arg(arg_index),
            Some(OwnershipMode::Owned)
        ) && Self::formal_stores_into_composite_return(sig, arg_index)
    }

    /// Owned argument is stored into the callee's return composite.
    fn signature_arg_stores_owned_payload(sig: &FunctionSignature, arg_index: usize) -> bool {
        matches!(
            sig.param_ownership_for_arg(arg_index),
            Some(OwnershipMode::Owned)
        ) && Self::formal_stores_into_composite_return(sig, arg_index)
    }

    /// `Vec`/`HashMap`/… `&mut self` methods that take owned values store into the receiver.
    /// User methods like `Squad::send_message` must not match — only collection receivers.
    fn signature_stores_into_collection_receiver(
        sig: &FunctionSignature,
        arg_index: usize,
        receiver_type: Option<&str>,
        qualified_key: Option<&str>,
    ) -> bool {
        if !matches!(
            sig.param_ownership_for_arg(arg_index),
            Some(OwnershipMode::Owned)
        ) {
            return false;
        }
        if !(sig.has_self_receiver
            && matches!(
                sig.param_ownership.first(),
                Some(OwnershipMode::MutBorrowed)
            ))
        {
            return false;
        }
        let type_name = receiver_type
            .map(|ty| {
                let base = ty.split('<').next().unwrap_or(ty);
                base.rsplit("::").next().unwrap_or(base)
            })
            .or_else(|| {
                qualified_key.and_then(|k| {
                    k.rsplit_once("::")
                        .map(|(ty, _)| ty.rsplit("::").next().unwrap_or(ty))
                })
            });
        type_name.is_some_and(Self::is_collection_receiver_type)
    }

    fn is_collection_receiver_type(name: &str) -> bool {
        crate::type_classification::is_collection_storage_receiver(name)
    }

    /// True when method-call argument `arg_index` is stored into an owned formal (signature-driven).
    pub(crate) fn method_call_argument_stores_owned_payload(
        method: &str,
        receiver_type: Option<&str>,
        arg_index: usize,
        arg_count: usize,
        registry: &SignatureRegistry,
    ) -> bool {
        if Self::is_language_level_method_payload_store(method) {
            return true;
        }
        if let Some(sig) =
            Self::resolve_method_signature_for_storage(method, receiver_type, arg_count, registry)
        {
            let key = receiver_type.map(|ty| {
                let base = ty.split('<').next().unwrap_or(ty);
                let short = base.rsplit("::").next().unwrap_or(base);
                format!("{short}::{method}")
            });
            return Self::signature_arg_stores_owned_payload(sig, arg_index)
                || Self::signature_stores_into_collection_receiver(
                    sig,
                    arg_index,
                    receiver_type,
                    key.as_deref(),
                );
        }
        // Multiple receivers share the method name: only accept when every
        // candidate agrees the arg is an owned store into a collection / return.
        Self::consensus_collection_owned_store(method, arg_count, arg_index, registry)
    }

    fn resolve_method_signature_for_storage<'a>(
        method: &str,
        receiver_type: Option<&str>,
        arg_count: usize,
        registry: &'a SignatureRegistry,
    ) -> Option<&'a FunctionSignature> {
        if let Some(ty) = receiver_type {
            if let Some(sig) = crate::analyzer::stdlib_method_traits::lookup_method_signature(
                method,
                Some(ty),
                registry,
            ) {
                return Some(sig);
            }
        }
        let _ = arg_count;
        if !registry.has_collision(method) {
            if let Some(sig) = registry.get_signature(method) {
                return Some(sig);
            }
        }
        registry.find_unique_signature_ending_with(method)
    }

    fn consensus_collection_owned_store(
        method: &str,
        arg_count: usize,
        arg_index: usize,
        registry: &SignatureRegistry,
    ) -> bool {
        let Some(keys) = registry.method_keys_for(method) else {
            return false;
        };
        let mut any_collection_store = false;
        let mut any_borrowed_arg = false;
        let mut saw = false;
        for key in keys {
            let Some(sig) = registry.get_signature(key) else {
                continue;
            };
            let sig_args = if sig.has_self_receiver {
                sig.param_ownership.len().saturating_sub(1)
            } else {
                sig.param_ownership.len()
            };
            if arg_count > 0 && sig_args != arg_count {
                continue;
            }
            saw = true;
            if matches!(
                sig.param_ownership_for_arg(arg_index),
                Some(OwnershipMode::Borrowed)
            ) {
                any_borrowed_arg = true;
            }
            if Self::signature_arg_stores_owned_payload(sig, arg_index)
                || Self::signature_stores_into_collection_receiver(
                    sig,
                    arg_index,
                    None,
                    Some(key.as_str()),
                )
            {
                any_collection_store = true;
            }
        }
        // Homonyms like Vec::push / String::push both store owned; reject only when
        // some candidate treats the same slot as a borrowed lookup (e.g. remove).
        saw && any_collection_store && !any_borrowed_arg
    }

    fn receiver_type_name_for_storage(object: &Expression) -> Option<String> {
        match object {
            Expression::Identifier { name, .. }
                if name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) =>
            {
                Some(name.clone())
            }
            Expression::FieldAccess { object, .. } => Self::receiver_type_name_for_storage(object),
            _ => None,
        }
    }

    /// Check if an expression stores a parameter by value.
    /// Matches direct identifier use, wrapping in Some/Ok/Err, enum variant constructors,
    /// tuples, and struct literals containing the parameter.
    pub(crate) fn expression_stores_identifier(
        &self,
        name: &str,
        expr: &Expression,
        registry: &SignatureRegistry,
    ) -> bool {
        self.expression_stores_identifier_inner(name, expr, false, registry)
    }

    fn expression_stores_identifier_inner(
        &self,
        name: &str,
        expr: &Expression,
        in_payload: bool,
        registry: &SignatureRegistry,
    ) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => in_payload && id == name,
            Expression::Call {
                function,
                arguments,
                ..
            } => arguments.iter().enumerate().any(|(i, (_label, arg))| {
                let arg_in_payload = Self::call_argument_stores_owned_payload(
                    function,
                    i,
                    arguments.len(),
                    registry,
                );
                self.expression_stores_identifier_inner(name, arg, arg_in_payload, registry)
            }),
            Expression::MethodCall {
                method,
                arguments,
                object,
                ..
            } => {
                let receiver_type = Self::receiver_type_name_for_storage(object);
                arguments.iter().enumerate().any(|(i, (_label, arg))| {
                    let arg_in_payload = Self::method_call_argument_stores_owned_payload(
                        method,
                        receiver_type.as_deref(),
                        i,
                        arguments.len(),
                        registry,
                    );
                    self.expression_stores_identifier_inner(name, arg, arg_in_payload, registry)
                })
            }
            Expression::Tuple { elements, .. } => elements
                .iter()
                .any(|el| self.expression_stores_identifier_inner(name, el, true, registry)),
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, v)| self.expression_stores_identifier_inner(name, v, true, registry)),
            Expression::Array { elements, .. } => elements
                .iter()
                .any(|el| self.expression_stores_identifier_inner(name, el, true, registry)),
            _ => false,
        }
    }

    pub(crate) fn is_stored(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
    ) -> bool {
        // Check if the parameter is stored in a struct field or collection
        for stmt in statements {
            match stmt {
                Statement::Let { value, .. } => {
                    if self.expression_stores_identifier(name, value, registry) {
                        return true;
                    }
                }
                Statement::Return {
                    value: Some(expr), ..
                } => {
                    if self.expression_stores_identifier(name, expr, registry) {
                        return true;
                    }
                }
                Statement::Expression { expr, .. } => {
                    if self.expression_stores_identifier(name, expr, registry) {
                        return true;
                    }
                    // Struct literals in method args may bind the param by field without
                    // going through a composite-return factory signature.
                    if let Expression::MethodCall { arguments, .. } = expr {
                        for (_label, arg) in arguments {
                            if let Expression::StructLiteral { fields, .. } = arg {
                                for (_field_name, field_expr) in fields {
                                    if self.expression_uses_identifier(name, field_expr) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
                Statement::Assignment {
                    target: Expression::FieldAccess { object, .. },
                    value,
                    ..
                } => {
                    if Self::is_rooted_field_access(object) {
                        if self.expression_stores_identifier(name, value, registry) {
                            return true;
                        }
                        if matches!(value, Expression::Identifier { name: id, .. } if id == name) {
                            return true;
                        }
                    }
                }
                Statement::Assignment {
                    target: Expression::Index { .. },
                    value,
                    ..
                } => {
                    if self.expression_stores_identifier(name, value, registry) {
                        return true;
                    }
                    if matches!(value, Expression::Identifier { name: id, .. } if id == name) {
                        return true;
                    }
                }
                // Recursively check if/else bodies for storage operations
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if self.is_stored(name, then_block, registry) {
                        return true;
                    }
                    if let Some(else_stmts) = else_block {
                        if self.is_stored(name, else_stmts, registry) {
                            return true;
                        }
                    }
                    if self.stmt_has_enum_variant_consuming(name, stmt) {
                        return true;
                    }
                }
                // Match scrutinee / arm bodies may construct consuming enum variants
                // (`match Value::Text(label) { ... }` → label stays Owned).
                Statement::Match { value, arms, .. } => {
                    if self.expression_stores_identifier(name, value, registry) {
                        return true;
                    }
                    if self.expr_has_enum_variant_consuming(name, value) {
                        return true;
                    }
                    for arm in arms {
                        if self.expression_stores_identifier(name, arm.body, registry) {
                            return true;
                        }
                        if self.expr_has_enum_variant_consuming(name, arm.body) {
                            return true;
                        }
                    }
                }
                // Recursively check loop bodies
                Statement::While { body, .. } | Statement::For { body, .. } => {
                    if self.is_stored(name, body, registry) {
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

    /// Like [`is_stored`], but `{ field: param }` struct-literal init with a bare identifier
    /// does not force owned formals for non-Copy aggregate params — codegen clones at the site.
    pub(crate) fn is_stored_requiring_owned(
        &self,
        name: &str,
        param_type: &Type,
        statements: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
    ) -> bool {
        if !self.is_stored(name, statements, registry) {
            return false;
        }
        // Copy params in `{ field: param }` may stay borrowed — codegen clones at the site.
        if self.is_only_stored_via_bare_struct_literal_field(name, statements, registry)
            && self.is_copy_type(param_type)
        {
            return false;
        }
        true
    }

    /// String formals moved into a composite or returned must stay owned `String`.
    pub(crate) fn string_param_consumed_owned(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
    ) -> bool {
        self.is_stored(name, statements, registry)
            || self.is_returned(name, statements)
            || self.param_is_consumed_into_return(name, statements)
    }

    fn is_only_stored_via_bare_struct_literal_field(
        &self,
        name: &str,
        statements: &[&'ast Statement<'ast>],
        registry: &SignatureRegistry,
    ) -> bool {
        let mut bare_struct_store = false;
        for stmt in statements {
            match stmt {
                Statement::Return {
                    value: Some(Expression::StructLiteral { fields, .. }),
                    ..
                }
                | Statement::Expression {
                    expr: Expression::StructLiteral { fields, .. },
                    ..
                }
                | Statement::Let {
                    value: Expression::StructLiteral { fields, .. },
                    ..
                } => {
                    for (_, field_expr) in fields {
                        if matches!(
                            field_expr,
                            Expression::Identifier { name: id, .. } if id == name
                        ) {
                            bare_struct_store = true;
                        } else if self.expression_stores_identifier(name, field_expr, registry) {
                            return false;
                        }
                    }
                }
                _ => {
                    if self.statement_stores_identifier_excluding_bare_struct_literal(
                        name, stmt, registry,
                    ) {
                        return false;
                    }
                }
            }
        }
        bare_struct_store
    }

    fn statement_stores_identifier_excluding_bare_struct_literal(
        &self,
        name: &str,
        stmt: &Statement<'ast>,
        registry: &SignatureRegistry,
    ) -> bool {
        match stmt {
            Statement::Return {
                value: Some(Expression::StructLiteral { fields, .. }),
                ..
            }
            | Statement::Expression {
                expr: Expression::StructLiteral { fields, .. },
                ..
            }
            | Statement::Let {
                value: Expression::StructLiteral { fields, .. },
                ..
            } => fields.iter().any(|(_, field_expr)| {
                self.expression_stores_identifier(name, field_expr, registry)
                    && !matches!(
                        field_expr,
                        Expression::Identifier { name: id, .. } if id == name
                    )
            }),
            _ => self.stmt_stores_identifier(name, stmt, registry),
        }
    }

    fn stmt_stores_identifier(
        &self,
        name: &str,
        stmt: &Statement<'ast>,
        registry: &SignatureRegistry,
    ) -> bool {
        match stmt {
            Statement::Return {
                value: Some(expr), ..
            } => self.expression_stores_identifier(name, expr, registry),
            Statement::Expression { expr, .. } => {
                self.expression_stores_identifier(name, expr, registry)
            }
            Statement::Let { value, .. } => {
                self.expression_stores_identifier(name, value, registry)
            }
            Statement::Assignment { value, .. } => {
                self.expression_stores_identifier(name, value, registry)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                self.is_stored(name, then_block, registry)
                    || else_block
                        .as_ref()
                        .is_some_and(|b| self.is_stored(name, b, registry))
            }
            Statement::While { body, .. } | Statement::For { body, .. } => {
                self.is_stored(name, body, registry)
            }
            _ => self.stmt_has_enum_variant_consuming(name, stmt),
        }
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
            // `match Value::Text(label) { ... }` — variant construction in the scrutinee
            // consumes the param (embedded-crate `owned_string`).
            Statement::Match { value, arms, .. } => {
                if self.expr_has_enum_variant_consuming(name, value) {
                    return true;
                }
                arms.iter()
                    .any(|arm| self.expr_has_enum_variant_consuming(name, arm.body))
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr_has_enum_variant_consuming(name, condition)
                    || then_block
                        .iter()
                        .any(|s| self.stmt_has_enum_variant_consuming(name, s))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter()
                            .any(|s| self.stmt_has_enum_variant_consuming(name, s))
                    })
            }
            Statement::While { body, .. } | Statement::For { body, .. } => body
                .iter()
                .any(|s| self.stmt_has_enum_variant_consuming(name, s)),
            _ => false,
        }
    }

    /// Recursively check if an expression contains an enum variant constructor
    /// (`Type::Variant(param)` or `Enum::Variant(param)`) that consumes the parameter.
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
                let is_enum_variant = match &**function {
                    Expression::Identifier { name: fn_name, .. } => {
                        Self::looks_like_enum_variant_constructor(fn_name)
                    }
                    Expression::FieldAccess { field, .. } => {
                        field.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                    }
                    _ => false,
                };

                if is_enum_variant {
                    for (_label, arg) in arguments {
                        if matches!(arg, Expression::Identifier { name: id, .. } if id == name) {
                            return true;
                        }
                    }
                }

                for (_label, arg) in arguments {
                    if self.expr_has_enum_variant_consuming(name, arg) {
                        return true;
                    }
                }
                self.expr_has_enum_variant_consuming(name, function)
            }
            Expression::MethodCall {
                method,
                arguments,
                object,
                ..
            } => {
                if method
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_uppercase())
                {
                    for (_label, arg) in arguments {
                        if matches!(arg, Expression::Identifier { name: id, .. } if id == name) {
                            return true;
                        }
                    }
                }
                for (_label, arg) in arguments {
                    if self.expr_has_enum_variant_consuming(name, arg) {
                        return true;
                    }
                }
                self.expr_has_enum_variant_consuming(name, object)
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
        crate::type_classification::is_enum_variant_constructor_path(qualified_name)
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

    /// True when expr is rooted at an identifier through FieldAccess chains.
    /// Matches `obj`, `obj.field`, `obj.sub.field`, etc.
    fn is_rooted_field_access(expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { .. } => true,
            Expression::FieldAccess { object, .. } => Self::is_rooted_field_access(object),
            _ => false,
        }
    }
}

#[cfg(test)]
mod kill_factory_storage_tests {
    use crate::analyzer::{Analyzer, OwnershipMode};
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse_program(src: &str) -> crate::parser::Program<'static> {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_with_locations();
        let parser = Box::leak(Box::new(Parser::new(tokens)));
        parser.parse().expect("parse")
    }

    #[test]
    fn nested_enum_variant_in_struct_literal_stores_string_param() {
        let src = r#"
pub enum ObjectiveType {
    KillEnemies(string, u32),
}
pub struct Objective {
    pub kind: ObjectiveType,
}
impl Objective {
    pub fn kill(enemy_type: string, count: u32) -> Objective {
        Objective { kind: ObjectiveType::KillEnemies(enemy_type, count) }
    }
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let kill = analyzed
            .iter()
            .find(|f| f.decl.name == "kill")
            .expect("kill fn");

        assert!(
            analyzer.is_stored("enemy_type", kill.decl.body.as_slice(), &registry),
            "is_stored(enemy_type) must be true for nested enum payload"
        );
        let mode = kill
            .inferred_ownership
            .get("enemy_type")
            .copied()
            .expect("enemy_type ownership");
        assert_eq!(
            mode,
            OwnershipMode::Owned,
            "enemy_type moved into enum payload must be Owned; got {mode:?}. ownership map: {:?}",
            kill.inferred_ownership
        );

        let ir_ctx = crate::ir::analyze_and_lower(&program).expect("ir lower");
        let ir_fn = ir_ctx
            .module
            .functions
            .iter()
            .find(|f| f.name.ends_with("::kill") || f.name == "kill")
            .expect("ir kill");
        let ir_own = ir_fn.param_types.get("enemy_type").map(|st| &st.ownership);
        assert!(
            matches!(ir_own, Some(crate::ir::OwnedType::Owned)),
            "IR must keep Owned for enum payload store; got {ir_own:?}"
        );
    }

    #[test]
    fn nested_methodcall_enum_variant_in_call_arg_stores_string_param() {
        let src = r#"
pub enum ObjectiveType {
    Kill(string, i32),
}
pub struct Objective {
    pub obj_type: ObjectiveType,
    pub count: i32,
}
impl Objective {
    pub fn new(obj_type: ObjectiveType, count: i32) -> Objective {
        Objective { obj_type, count }
    }
}
pub fn create_kill(enemy_type: string, count: i32) -> Objective {
    let _desc = format!("Kill {} {}", count, enemy_type);
    Objective::new(ObjectiveType::Kill(enemy_type, count), count)
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let kill = analyzed
            .iter()
            .find(|f| f.decl.name == "create_kill")
            .expect("create_kill fn");

        assert!(
            analyzer.is_stored("enemy_type", kill.decl.body.as_slice(), &registry),
            "ObjectiveType::Kill MethodCall must store enemy_type"
        );
        let mode = kill
            .inferred_ownership
            .get("enemy_type")
            .copied()
            .expect("enemy_type ownership");
        assert_eq!(
            mode,
            OwnershipMode::Owned,
            "nested enum payload must infer Owned; got {mode:?}"
        );

        let ir_ctx = crate::ir::analyze_and_lower(&program).expect("ir lower");
        let ir_fn = ir_ctx
            .module
            .functions
            .iter()
            .find(|f| f.name == "create_kill")
            .expect("ir create_kill");
        let ir_own = ir_fn.param_types.get("enemy_type").map(|st| &st.ownership);
        assert!(
            matches!(ir_own, Some(crate::ir::OwnedType::Owned)),
            "IR must keep Owned for nested enum payload; got {ir_own:?}"
        );
        assert!(
            !ir_fn.str_ref_params.contains("enemy_type"),
            "stored enemy_type must not be str_ref"
        );
    }

    #[test]
    fn static_method_call_does_not_store_param() {
        let src = r#"
struct Grid { data: Vec<i32> }
impl Grid {
    fn check(grid: Grid, x: i32) -> bool { false }
}
struct Player { x: i32 }
impl Player {
    fn can_move(self, grid: Grid) -> bool {
        Grid::check(grid, self.x)
    }
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let can_move = analyzed
            .iter()
            .find(|f| f.decl.name == "can_move")
            .expect("can_move fn");
        assert!(
            !analyzer.is_stored("grid", can_move.decl.body.as_slice(), &registry),
            "Grid::check must not classify grid as stored"
        );
    }

    #[test]
    fn fps_camera_update_grid_inferred_borrowed_via_static_passthrough() {
        let src = r#"
struct VoxelGrid {
    data: Vec<i32>
}

impl VoxelGrid {
    fn is_solid(self, x: i32, y: i32, z: i32) -> bool {
        false
    }
}

struct Vec3 {
    x: f32,
    y: f32,
    z: f32
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }
}

struct FpsCamera {
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
    speed: f32
}

impl FpsCamera {
    fn update(self, dt: f32, grid: VoxelGrid) {
        FpsCamera::depenetrate(grid, self.pos_x, self.pos_y, self.pos_z)
        let dx = self.speed * dt
        let test_x = Vec3::new(self.pos_x + dx, self.pos_y, self.pos_z)
        if !FpsCamera::collides(grid, test_x) {
            self.pos_x = self.pos_x + dx
        }
    }

    fn collides(grid: VoxelGrid, pos: Vec3) -> bool {
        grid.is_solid(pos.x as i32, pos.y as i32, pos.z as i32)
    }

    fn depenetrate(grid: VoxelGrid, x: f32, y: f32, z: f32) {
        grid.is_solid(x as i32, y as i32, z as i32)
    }
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let update = analyzed
            .iter()
            .find(|f| f.decl.name == "update")
            .expect("update fn");
        let mode = update
            .inferred_ownership
            .get("grid")
            .copied()
            .expect("grid ownership");
        assert_eq!(
            mode,
            OwnershipMode::Borrowed,
            "update(grid) should passthrough Borrowed from static callees; got {mode:?}"
        );
        let depenetrate = analyzed
            .iter()
            .find(|f| f.decl.name == "depenetrate")
            .expect("depenetrate fn");
        let dep_mode = depenetrate
            .inferred_ownership
            .get("grid")
            .copied()
            .expect("depenetrate grid ownership");
        assert_eq!(
            dep_mode,
            OwnershipMode::Borrowed,
            "depenetrate grid should be Borrowed; got {dep_mode:?}"
        );

        let ir_ctx = crate::ir::analyze_and_lower(&program).expect("ir lower");
        let ir_update = ir_ctx
            .module
            .functions
            .iter()
            .find(|f| f.name.ends_with("::update"))
            .expect("ir update");
        let ir_grid = ir_update.param_types.get("grid").map(|st| &st.ownership);
        assert!(
            matches!(ir_grid, Some(crate::ir::OwnedType::Ref(_))),
            "IR update(grid) must be Borrowed; got {ir_grid:?}"
        );
    }

    #[test]
    fn inventory_add_item_stores_item_as_owned() {
        let src = r#"
pub struct Item {
    pub name: string,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

impl ItemStack {
    pub fn new(item: Item, quantity: i32) -> ItemStack {
        ItemStack { item, quantity }
    }
}

pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
}

impl Inventory {
    pub fn add_item(self, item: Item, quantity: i32) {
        self.slots[0] = Some(ItemStack::new(item, quantity))
    }
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let add_item = analyzed
            .iter()
            .find(|f| f.decl.name == "add_item")
            .expect("add_item fn");
        assert!(
            analyzer.is_stored("item", add_item.decl.body.as_slice(), &registry),
            "ItemStack::new(item) must store item"
        );
        let mode = add_item
            .inferred_ownership
            .get("item")
            .copied()
            .expect("item ownership");
        assert_eq!(mode, OwnershipMode::Owned, "stored item must be Owned");
    }

    #[test]
    fn passthrough_program_inventory_add_item_owned() {
        let src = r#"
pub struct Item {
    pub name: string,
    pub weight: f32,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

impl ItemStack {
    pub fn new(item: Item, quantity: i32) -> ItemStack {
        ItemStack { item, quantity }
    }
}

pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
}

impl Inventory {
    pub fn add_item(self, item: Item, quantity: i32) {
        self.slots[0] = Some(ItemStack::new(item, quantity))
    }
}

pub struct Merchant {
    pub inventory: Inventory,
}

impl Merchant {
    pub fn add_item(self, item: Item, quantity: i32) {
        self.inventory.add_item(item, quantity)
    }
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let inv = analyzed
            .iter()
            .find(|f| {
                f.decl.name == "add_item" && f.decl.parent_type.as_deref() == Some("Inventory")
            })
            .expect("Inventory::add_item");
        let mode = inv
            .inferred_ownership
            .get("item")
            .copied()
            .expect("item ownership");
        assert_eq!(
            mode,
            OwnershipMode::Owned,
            "Inventory::add_item item must be Owned; got {mode:?}"
        );
    }

    #[test]
    fn itemstack_new_item_param_owned_for_bare_struct_literal_store() {
        let src = r#"
pub struct Item {
    pub name: string,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

impl ItemStack {
    pub fn new(item: Item, quantity: i32) -> ItemStack {
        ItemStack { item, quantity }
    }
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let new_fn = analyzed.iter().find(|f| f.decl.name == "new").expect("new");
        let mode = new_fn
            .inferred_ownership
            .get("item")
            .copied()
            .expect("item ownership");
        assert_eq!(
            mode,
            OwnershipMode::Owned,
            "Item in struct literal constructor must be Owned"
        );
    }

    #[test]
    fn create_kill_quest_enemy_type_owned_after_two_pass_analyze() {
        let program_src = r#"
pub enum ObjectiveType {
    Kill(string, i32),
}

pub struct Objective {
    obj_type: ObjectiveType,
    count: i32,
}

impl Objective {
    pub fn new(obj_type: ObjectiveType, count: i32) -> Objective {
        Objective { obj_type, count }
    }
}

pub fn create_kill(enemy_type: string, count: i32) -> Objective {
    let desc = format!("Kill {} {}", count, enemy_type);
    Objective::new(ObjectiveType::Kill(enemy_type, count), count)
}
"#;
        let program = parse_program(program_src);
        let mut analyzer = Analyzer::new();
        let (_, first_registry, _) = analyzer
            .analyze_program_with_global_signatures(&program, &Default::default())
            .expect("pass1");
        let mut global = first_registry;
        let copy: std::collections::HashSet<String> =
            analyzer.get_copy_structs().into_iter().collect();
        let mut analyzer2 = Analyzer::new_with_copy_structs(copy);
        let (analyzed, _, _) = analyzer2
            .analyze_program_with_global_signatures(&program, &global)
            .expect("pass2");
        let kill = analyzed
            .iter()
            .find(|f| f.decl.name == "create_kill")
            .expect("create_kill");
        let mode = kill
            .inferred_ownership
            .get("enemy_type")
            .copied()
            .expect("ownership");
        assert_eq!(
            mode,
            OwnershipMode::Owned,
            "two-pass analyze must keep enemy_type Owned; got {mode:?}"
        );
    }

    #[test]
    fn create_kill_quest_enemy_type_owned_in_full_program() {
        let src = r#"
pub enum ObjectiveType {
    Kill(string, i32),
}

pub struct Quest {
    name: string,
    desc: string,
    quest_giver: string,
}

impl Quest {
    pub fn new(name: string, title: string, desc: string) -> Quest {
        Quest { name, desc, quest_giver: "".to_string() }
    }
    pub fn add_objective(self, obj: Objective) {}
}

pub struct Objective {
    name: string,
    desc: string,
    count: i32,
}

impl Objective {
    pub fn new_with_progress(name: string, desc: string, obj_type: ObjectiveType, count: i32) -> Objective {
        Objective { name, desc, count }
    }
}

pub fn create_kill_quest(
    id: string,
    title: string,
    enemy_type: string,
    count: i32,
    quest_giver: string
) -> Quest {
    let mut quest = Quest::new(id, title, format!("Kill {} {}", count, enemy_type))
    quest.quest_giver = quest_giver

    let obj = Objective::new_with_progress(
        format!("{}_kill", id),
        format!("Kill {} {}", count, enemy_type),
        ObjectiveType::Kill(enemy_type, count),
        count
    )
    quest.add_objective(obj)

    quest
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let kill_quest = analyzed
            .iter()
            .find(|f| f.decl.name == "create_kill_quest")
            .expect("create_kill_quest");
        assert!(
            analyzer.is_stored("enemy_type", kill_quest.decl.body.as_slice(), &registry),
            "enemy_type must be stored in full program"
        );
        let mode = kill_quest
            .inferred_ownership
            .get("enemy_type")
            .copied()
            .expect("enemy_type ownership");
        assert_eq!(
            mode,
            OwnershipMode::Owned,
            "enemy_type in full program must be Owned; got {mode:?}"
        );
    }

    #[test]
    fn field_assignment_bare_identifier_stores_param() {
        let src = r#"
pub struct Node {
    pub items: Vec<string>,
}
impl Node {
    pub fn with_items(items: Vec<string>) -> Node {
        let mut node = Node { items: Vec::new() }
        node.items = items
        node
    }
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let with_items = analyzed
            .iter()
            .find(|f| f.decl.name == "with_items")
            .expect("with_items");
        assert!(
            analyzer.is_stored("items", with_items.decl.body.as_slice(), &registry),
            "field assignment must store items param"
        );
        let mode = with_items
            .inferred_ownership
            .get("items")
            .copied()
            .expect("items ownership");
        assert_eq!(mode, OwnershipMode::Owned, "stored Vec param must be Owned");
    }

    #[test]
    fn expression_stores_identifier_detects_itemstack_new_call() {
        let src = r#"
pub struct Item { pub name: string }
pub struct ItemStack { pub item: Item, pub quantity: i32 }
impl ItemStack {
    pub fn new(item: Item, quantity: i32) -> ItemStack {
        ItemStack { item, quantity }
    }
}
pub fn f(item: Item) {
    let _stack = ItemStack::new(item, 1)
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let f = analyzed.iter().find(|x| x.decl.name == "f").expect("f");
        let stmt = f.decl.body[0];
        let value = match stmt {
            crate::parser::Statement::Let { value, .. } => value,
            _ => panic!("expected let"),
        };
        assert!(
            analyzer.expression_stores_identifier("item", value, &registry),
            "ItemStack::new(item) must store via expression_stores_identifier; expr={value:?}"
        );
    }

    #[test]
    fn bare_itemstack_new_in_let_stores_item() {
        let src = r#"
pub struct Item { pub name: string }
pub struct ItemStack { pub item: Item, pub quantity: i32 }
pub struct Inventory { pub slots: Vec<Option<ItemStack>> }
impl ItemStack {
    pub fn new(item: Item, quantity: i32) -> ItemStack {
        ItemStack { item, quantity }
    }
}
impl Inventory {
    pub fn add_item(self, item: Item, quantity: i32) {
        let _stack = ItemStack::new(item, quantity)
    }
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let add_item = analyzed
            .iter()
            .find(|f| f.decl.name == "add_item")
            .expect("add_item");
        assert!(
            analyzer.is_stored("item", add_item.decl.body.as_slice(), &registry),
            "ItemStack::new in let must store item"
        );
    }

    #[test]
    fn methodcall_new_in_some_on_index_assignment_stores_item() {
        let src = r#"
pub struct Item { pub name: string }
pub struct ItemStack { pub item: Item, pub quantity: i32 }
impl ItemStack {
    pub fn new(item: Item, quantity: i32) -> ItemStack {
        ItemStack { item, quantity }
    }
}
pub struct Inventory { pub slots: Vec<Option<ItemStack>> }
impl Inventory {
    pub fn add_item(self, item: Item, quantity: i32) {
        self.slots[0] = Some(ItemStack::new(item, quantity))
    }
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let add_item = analyzed
            .iter()
            .find(|f| f.decl.name == "add_item")
            .expect("add_item");
        assert!(
            analyzer.is_stored("item", add_item.decl.body.as_slice(), &registry),
            "index assign Some(ItemStack::new(item)) must store item"
        );
    }
}
