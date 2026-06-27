#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]

// Self/Field Analysis Module
//
// This module provides functions to analyze AST nodes and determine if they:
// - Access struct fields
// - Mutate struct fields
// - Modify self parameters
// - Modify variables
//
// These functions are used by the ownership inference system to determine
// whether methods need `&self`, `&mut self`, or `self` parameters.
use crate::analyzer::{OwnershipMode, SignatureRegistry};
use crate::parser::{Expression, FunctionDecl, Pattern, Statement};
use std::collections::HashSet;

/// Context needed for field/self analysis
pub struct AnalysisContext<'a, 'ast> {
    /// Current function parameters (to distinguish params from fields)
    pub current_function_params: &'a [crate::parser::Parameter<'ast>],
    /// Fields of the current struct (if in impl block)
    pub current_struct_fields: &'a HashSet<String>,
    /// Locals bound in this function (let / for / match) — shadow struct field names
    pub local_variables: Option<&'a HashSet<String>>,
}

impl<'a, 'ast> AnalysisContext<'a, 'ast> {
    pub fn new(params: &'a [crate::parser::Parameter<'ast>], fields: &'a HashSet<String>) -> Self {
        Self {
            current_function_params: params,
            current_struct_fields: fields,
            local_variables: None,
        }
    }

    pub fn with_locals(
        params: &'a [crate::parser::Parameter<'ast>],
        fields: &'a HashSet<String>,
        locals: &'a HashSet<String>,
    ) -> Self {
        Self {
            current_function_params: params,
            current_struct_fields: fields,
            local_variables: Some(locals),
        }
    }

    fn shadows_struct_field(&self, name: &str) -> bool {
        self.current_function_params.iter().any(|p| p.name == name)
            || self
                .local_variables
                .is_some_and(|locals| locals.contains(name))
    }
}

/// Collect identifier names bound as locals in a function body (including nested blocks).
pub fn collect_local_bindings(body: &[&Statement]) -> HashSet<String> {
    let mut locals = HashSet::new();
    for stmt in body {
        collect_locals_from_statement(stmt, &mut locals);
    }
    locals
}

fn collect_locals_from_statement(stmt: &Statement, locals: &mut HashSet<String>) {
    match stmt {
        Statement::Let {
            pattern,
            else_block,
            ..
        } => {
            collect_locals_from_pattern(pattern, locals);
            if let Some(block) = else_block {
                for s in block {
                    collect_locals_from_statement(s, locals);
                }
            }
        }
        Statement::For { pattern, body, .. } => {
            collect_locals_from_pattern(pattern, locals);
            for s in body {
                collect_locals_from_statement(s, locals);
            }
        }
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            for s in then_block {
                collect_locals_from_statement(s, locals);
            }
            if let Some(else_b) = else_block {
                for s in else_b {
                    collect_locals_from_statement(s, locals);
                }
            }
        }
        Statement::While { body, .. }
        | Statement::Loop { body, .. }
        | Statement::Thread { body, .. }
        | Statement::Async { body, .. } => {
            for s in body {
                collect_locals_from_statement(s, locals);
            }
        }
        Statement::Match { arms, .. } => {
            for arm in arms {
                collect_locals_from_pattern(&arm.pattern, locals);
            }
        }
        _ => {}
    }
}

fn collect_locals_from_pattern(pattern: &crate::parser::Pattern, locals: &mut HashSet<String>) {
    use crate::parser::EnumPatternBinding;
    match pattern {
        Pattern::Identifier(name)
        | Pattern::Ref(name)
        | Pattern::RefMut(name)
        | Pattern::MutBinding(name) => {
            locals.insert(name.clone());
        }
        Pattern::Tuple(patterns) | Pattern::Or(patterns) => {
            for p in patterns {
                collect_locals_from_pattern(p, locals);
            }
        }
        Pattern::Reference(inner) => {
            collect_locals_from_pattern(inner, locals);
        }
        Pattern::EnumVariant(_, binding) => match binding {
            EnumPatternBinding::None | EnumPatternBinding::Wildcard => {}
            EnumPatternBinding::Single(name) => {
                locals.insert(name.clone());
            }
            EnumPatternBinding::Tuple(patterns) => {
                for p in patterns {
                    collect_locals_from_pattern(p, locals);
                }
            }
            EnumPatternBinding::Struct(fields, _) => {
                for (_, p) in fields {
                    collect_locals_from_pattern(p, locals);
                }
            }
        },
        Pattern::Wildcard | Pattern::Literal(_) => {}
    }
}

// =============================================================================
// Function-Level Analysis
// =============================================================================

/// Check if a function accesses any struct fields
pub fn function_accesses_fields(ctx: &AnalysisContext, func: &FunctionDecl) -> bool {
    for stmt in &func.body {
        if statement_accesses_fields(ctx, stmt) {
            return true;
        }
    }
    false
}

/// Check if a function mutates any struct fields
pub fn function_mutates_fields(ctx: &AnalysisContext, func: &FunctionDecl) -> bool {
    for stmt in &func.body {
        if statement_mutates_fields(ctx, stmt) {
            return true;
        }
    }
    false
}

/// Variable names bound directly from `self` (`let mut result = self`, `let result = self`).
fn collect_self_derived_locals(func: &FunctionDecl) -> HashSet<String> {
    let mut locals = HashSet::new();
    for stmt in &func.body {
        if let Statement::Let { pattern, value, .. } = stmt {
            let name = match pattern {
                Pattern::Identifier(n) | Pattern::MutBinding(n) => Some(n.as_str()),
                _ => None,
            };
            if let Some(name) = name {
                if matches!(value, Expression::Identifier { name: n, .. } if n == "self") {
                    locals.insert(name.to_string());
                }
            }
        }
    }
    locals
}

fn function_returns_named_identifier(func: &FunctionDecl, id: &str) -> bool {
    let Some(last_stmt) = func.body.last() else {
        return false;
    };
    match last_stmt {
        Statement::Return {
            value: Some(expr), ..
        }
        | Statement::Expression { expr, .. } => {
            matches!(expr, Expression::Identifier { name, .. } if name == id)
        }
        _ => false,
    }
}

/// Check if the body uses `match self { ... }` which moves the value.
pub fn function_matches_on_self(func: &FunctionDecl) -> bool {
    func.body.iter().any(|stmt| statement_matches_on_self(stmt))
}

fn statement_matches_on_self(stmt: &Statement) -> bool {
    match stmt {
        Statement::Match { value, .. } => expression_is_bare_self(value),
        Statement::Expression { expr, .. }
        | Statement::Return {
            value: Some(expr), ..
        } => expression_contains_match_on_self(expr),
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            then_block.iter().any(|s| statement_matches_on_self(s))
                || else_block
                    .as_ref()
                    .is_some_and(|b| b.iter().any(|s| statement_matches_on_self(s)))
        }
        Statement::For { body, .. }
        | Statement::While { body, .. }
        | Statement::Loop { body, .. } => body.iter().any(|s| statement_matches_on_self(s)),
        _ => false,
    }
}

fn expression_contains_match_on_self(expr: &Expression) -> bool {
    match expr {
        Expression::Block { statements, .. } => {
            statements.iter().any(|s| statement_matches_on_self(s))
        }
        _ => false,
    }
}

/// Check if the body consumes `self` by value — i.e., uses bare `self` (not `self.field`)
/// as a struct literal field, function argument, or other value position.
pub fn function_consumes_self(func: &FunctionDecl) -> bool {
    for stmt in &func.body {
        if statement_consumes_self(stmt) {
            return true;
        }
    }
    false
}

/// Check if the function iterates over a `self.field` (consuming the field)
/// without just calling `.clone()` on elements. This pattern requires owned `self`.
/// e.g. `for cond in self.conditions { if !cond.check() { ... } }`
pub fn function_iterates_self_field_consuming(func: &FunctionDecl) -> bool {
    func.body
        .iter()
        .any(|stmt| statement_iterates_self_field(stmt))
}

fn statement_iterates_self_field(stmt: &Statement) -> bool {
    match stmt {
        Statement::For { iterable, body, .. } => {
            let is_self_field = matches!(
                iterable,
                Expression::FieldAccess { object, .. }
                    if matches!(&**object, Expression::Identifier { name, .. } if name == "self")
            );
            if is_self_field {
                // Only count as consuming if the body doesn't just clone elements
                // (snapshot pattern uses for+clone → should be &self)
                !body.iter().all(|s| statement_only_clones_element(s))
            } else {
                // Check nested statements
                body.iter().any(|s| statement_iterates_self_field(s))
            }
        }
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            then_block.iter().any(|s| statement_iterates_self_field(s))
                || else_block
                    .as_ref()
                    .is_some_and(|b| b.iter().any(|s| statement_iterates_self_field(s)))
        }
        _ => false,
    }
}

fn statement_only_clones_element(stmt: &Statement) -> bool {
    match stmt {
        Statement::Expression { expr, .. } | Statement::Let { value: expr, .. } => {
            expression_only_clones(expr)
        }
        _ => false,
    }
}

fn expression_only_clones(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::MethodCall { method, .. }
            if matches!(
                method.as_str(),
                "clone" | "to_owned" | "to_vec" | "into_iter"
            ) || matches!(
                method.as_str(),
                "push"
                    | "insert"
                    | "extend"
                    | "append"
                    | "push_front"
                    | "push_back"
                    | "add"
                    | "fill"
            )
    )
}

fn expression_is_bare_self(expr: &Expression) -> bool {
    matches!(expr, Expression::Identifier { name, .. } if name == "self")
}

fn expression_consumes_self(expr: &Expression) -> bool {
    match expr {
        Expression::StructLiteral { fields, .. } => {
            fields.iter().any(|(_, val)| expression_is_bare_self(val))
        }
        Expression::Block { statements, .. } => {
            statements.iter().any(|s| statement_consumes_self(s))
        }
        _ => false,
    }
}

fn statement_consumes_self(stmt: &Statement) -> bool {
    match stmt {
        Statement::Return {
            value: Some(expr), ..
        } => expression_is_bare_self(expr) || expression_consumes_self(expr),
        Statement::Expression { expr, .. } => {
            expression_is_bare_self(expr) || expression_consumes_self(expr)
        }
        Statement::Let { value, .. } => {
            expression_is_bare_self(value) || expression_consumes_self(value)
        }
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            then_block.iter().any(|s| statement_consumes_self(s))
                || else_block
                    .as_ref()
                    .is_some_and(|b| b.iter().any(|s| statement_consumes_self(s)))
        }
        Statement::Match { arms, .. } => arms
            .iter()
            .any(|arm| expression_is_bare_self(arm.body) || expression_consumes_self(arm.body)),
        _ => false,
    }
}

/// True when the method's trailing return moves non-Copy `self.field` values into a struct literal.
pub fn function_return_moves_self_fields(func: &FunctionDecl) -> bool {
    use crate::parser::Statement;

    let return_expr = match func.body.last() {
        Some(Statement::Return {
            value: Some(expr), ..
        }) => expr,
        Some(Statement::Expression { expr, .. }) => expr,
        _ => return false,
    };

    expression_moves_self_fields_in_struct_literal(return_expr)
}

fn expression_moves_self_fields_in_struct_literal(expr: &Expression) -> bool {
    match expr {
        Expression::StructLiteral { fields, .. } => fields.iter().any(|(_, v)| match v {
            Expression::Identifier { name, .. } if name == "self" => true,
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == "self")
            }
            _ => false,
        }),
        _ => false,
    }
}

/// True when the function body only returns `self.{field_name}` (trivial Copy field accessor).
pub fn function_returns_self_field_with_name(func: &FunctionDecl, field_name: &str) -> bool {
    use crate::parser::{Expression, Statement};

    let return_expr = match func.body.last() {
        Some(Statement::Return {
            value: Some(expr), ..
        }) => expr,
        Some(Statement::Expression { expr, .. }) => expr,
        _ => return false,
    };

    matches!(
        return_expr,
        Expression::FieldAccess { object, field, .. }
            if field == field_name
                && matches!(&**object, Expression::Identifier { name, .. } if name == "self")
    )
}

/// Snapshot/factory: returns a new parent-type instance from `self.field` reads, not bare `self`.
pub fn function_returns_new_instance_from_self_fields(
    func: &FunctionDecl,
    registry: Option<&SignatureRegistry>,
    struct_name: Option<&str>,
) -> bool {
    use crate::parser::{Expression, Statement, Type};

    // Mutating then returning `Self { field: self.field, ... }` moves fields — not a snapshot.
    if function_modifies_self(func, registry, struct_name, None, None) {
        return false;
    }

    let parent_type = match &func.parent_type {
        Some(name) => name,
        None => return false,
    };
    let return_type_name = match &func.return_type {
        Some(Type::Custom(name)) if name == parent_type => name,
        _ => return false,
    };

    let return_expr = match func.body.last() {
        Some(Statement::Return {
            value: Some(expr), ..
        }) => expr,
        Some(Statement::Expression { expr, .. }) => expr,
        _ => return false,
    };

    match return_expr {
        Expression::StructLiteral { name, fields, .. } if name == return_type_name => !fields
            .iter()
            .any(|(_, v)| matches!(v, Expression::Identifier { name, .. } if name == "self")),
        _ => false,
    }
}

/// Fluent builder: `let mut result = self; result.field = x; result`
pub fn function_flows_self_through_local(func: &FunctionDecl) -> bool {
    let derived = collect_self_derived_locals(func);
    derived
        .iter()
        .any(|name| function_returns_named_identifier(func, name))
}

fn expression_modifies_derived_local(expr: &Expression, derived: &HashSet<String>) -> bool {
    match expr {
        Expression::FieldAccess { object, .. } => {
            if let Expression::Identifier { name, .. } = &**object {
                derived.contains(name)
            } else {
                expression_modifies_derived_local(object, derived)
            }
        }
        Expression::Index { object, .. } => expression_modifies_derived_local(object, derived),
        _ => false,
    }
}

fn statement_modifies_derived_local(stmt: &Statement, derived: &HashSet<String>) -> bool {
    match stmt {
        Statement::Assignment { target, .. } => expression_modifies_derived_local(target, derived),
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            then_block
                .iter()
                .any(|s| statement_modifies_derived_local(s, derived))
                || else_block.as_ref().is_some_and(|block| {
                    block
                        .iter()
                        .any(|s| statement_modifies_derived_local(s, derived))
                })
        }
        Statement::While { body, .. }
        | Statement::For { body, .. }
        | Statement::Loop { body, .. } => body
            .iter()
            .any(|s| statement_modifies_derived_local(s, derived)),
        _ => false,
    }
}

/// Check if a function returns Self (for builder pattern detection)
pub fn function_returns_self_type(func: &FunctionDecl) -> bool {
    use crate::parser::{Expression, Statement, Type};

    // First check if return type is a custom type (struct type)
    let returns_custom_type = matches!(&func.return_type, Some(Type::Custom(_)));

    if !returns_custom_type {
        return false;
    }

    // Now check if the function body actually returns `self`
    // Check the last statement in the body
    if let Some(last_stmt) = func.body.last() {
        match last_stmt {
            Statement::Return {
                value: Some(expr), ..
            } => {
                // Explicit return self
                matches!(expr, Expression::Identifier { name, .. } if name == "self")
            }
            Statement::Expression { expr, .. } => {
                // Implicit return self (last expression)
                matches!(expr, Expression::Identifier { name, .. } if name == "self")
            }
            _ => false,
        }
    } else {
        false
    }
}

/// Check if a function modifies self (for self parameter inference).
///
/// Uses the `SignatureRegistry` to look up method signatures instead of
/// maintaining a hardcoded list of known readonly methods.
pub fn function_modifies_self(
    func: &FunctionDecl,
    registry: Option<&SignatureRegistry>,
    struct_name: Option<&str>,
    field_types: Option<
        &std::collections::HashMap<String, std::collections::HashMap<String, crate::parser::Type>>,
    >,
    receiver_upgrades: Option<&std::collections::HashMap<String, crate::analyzer::OwnershipMode>>,
) -> bool {
    for stmt in &func.body {
        if statement_modifies_self(stmt, registry, struct_name, field_types, receiver_upgrades) {
            return true;
        }
    }
    false
}

/// Like [`function_modifies_self`], but also treats mutations on locals assigned from `self`
/// (fluent `let mut result = self; result.field = x`) as self modification.
/// Also detects indirect mutation through `self.field.get()` → match → mutate pattern.
pub fn function_modifies_self_or_derived_local(
    func: &FunctionDecl,
    registry: Option<&SignatureRegistry>,
    struct_name: Option<&str>,
    field_types: Option<
        &std::collections::HashMap<String, std::collections::HashMap<String, crate::parser::Type>>,
    >,
    receiver_upgrades: Option<&std::collections::HashMap<String, crate::analyzer::OwnershipMode>>,
) -> bool {
    if function_modifies_self(func, registry, struct_name, field_types, receiver_upgrades) {
        return true;
    }
    let derived = collect_self_derived_locals(func);
    if !derived.is_empty() {
        for stmt in &func.body {
            if statement_modifies_derived_local(stmt, &derived) {
                return true;
            }
        }
    }
    self_field_get_binding_is_mutated(func, registry, struct_name)
}

// =============================================================================
// Indirect Mutation Through HashMap.get() Detection
// =============================================================================

/// Detect indirect mutation of `self` through `self.field.get(key)` patterns.
///
/// Pattern: `let x = self.field.get(key)` followed by mutation of the bound
/// variable from `match x { Some(y) => y.mutating_method() }`.
/// This implies `self.field` needs mutable access, therefore `self` needs `&mut`.
fn self_field_get_binding_is_mutated(
    func: &FunctionDecl,
    registry: Option<&SignatureRegistry>,
    struct_name: Option<&str>,
) -> bool {
    let get_bindings = collect_self_field_get_bindings(func);
    if get_bindings.is_empty() {
        return false;
    }
    for stmt in &func.body {
        if match_or_if_let_mutates_get_binding(stmt, &get_bindings, registry, struct_name) {
            return true;
        }
    }
    false
}

/// Collect local variables that are assigned from `self.field.get(...)`.
fn collect_self_field_get_bindings(func: &FunctionDecl) -> HashSet<String> {
    let mut bindings = HashSet::new();
    for stmt in &func.body {
        if let Statement::Let { pattern, value, .. } = stmt {
            let name = match pattern {
                Pattern::Identifier(n) | Pattern::MutBinding(n) => Some(n.as_str()),
                _ => None,
            };
            if let Some(name) = name {
                if is_self_field_get_call(value) {
                    bindings.insert(name.to_string());
                }
            }
        }
    }
    bindings
}

/// Check if an expression is `self.field.get(...)`.
pub fn is_self_field_get_call(expr: &Expression) -> bool {
    if let Expression::MethodCall { object, method, .. } = expr {
        method == "get" && is_self_field_chain(object)
    } else {
        false
    }
}

/// Check if a method name is known to be read-only (no mutation).
pub fn is_known_readonly_method_name(method: &str) -> bool {
    matches!(
        method,
        "len"
            | "is_empty"
            | "contains"
            | "contains_key"
            | "get"
            | "first"
            | "last"
            | "iter"
            | "keys"
            | "values"
            | "clone"
            | "to_string"
            | "as_str"
            | "display"
            | "fmt"
            | "eq"
            | "ne"
            | "cmp"
            | "partial_cmp"
            | "hash"
            | "bone_count"
    )
}

/// Check if a statement contains a match/if-let that binds a get-result
/// variable and mutates the bound value.
fn match_or_if_let_mutates_get_binding(
    stmt: &Statement,
    get_bindings: &HashSet<String>,
    registry: Option<&SignatureRegistry>,
    struct_name: Option<&str>,
) -> bool {
    match stmt {
        Statement::Match { value, arms, .. } => {
            let scrutinee_var = match value {
                Expression::Identifier { name, .. } => Some(name.as_str()),
                _ => None,
            };
            if scrutinee_var.is_some_and(|v| get_bindings.contains(v)) {
                for arm in arms.iter() {
                    if let Some(binding) = extract_some_binding(&arm.pattern) {
                        if match_arm_body_mutates_var(arm.body, binding, registry, struct_name) {
                            return true;
                        }
                    }
                    if let Some(bindings) = extract_tuple_some_bindings(&arm.pattern) {
                        for binding in &bindings {
                            if match_arm_body_mutates_var(arm.body, binding, registry, struct_name)
                            {
                                return true;
                            }
                        }
                    }
                }
            }

            // Handle tuple scrutinee: match (a_opt, b_opt) { (Some(a), Some(b)) => ... }
            if let Expression::Tuple { elements, .. } = *value {
                for elem in elements.iter() {
                    if let Expression::Identifier { name, .. } = elem {
                        if get_bindings.contains(name.as_str()) {
                            for arm in arms.iter() {
                                if let Some(binding) =
                                    find_binding_for_var_in_tuple_match(value, name, &arm.pattern)
                                {
                                    if match_arm_body_mutates_var(
                                        arm.body,
                                        binding,
                                        registry,
                                        struct_name,
                                    ) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            arms.iter().any(|arm| {
                if let Expression::Block { statements, .. } = arm.body {
                    statements.iter().any(|s| {
                        match_or_if_let_mutates_get_binding(s, get_bindings, registry, struct_name)
                    })
                } else {
                    false
                }
            })
        }
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            then_block.iter().any(|s| {
                match_or_if_let_mutates_get_binding(s, get_bindings, registry, struct_name)
            }) || else_block.as_ref().is_some_and(|b| {
                b.iter().any(|s| {
                    match_or_if_let_mutates_get_binding(s, get_bindings, registry, struct_name)
                })
            })
        }
        Statement::While { body, .. }
        | Statement::For { body, .. }
        | Statement::Loop { body, .. } => body
            .iter()
            .any(|s| match_or_if_let_mutates_get_binding(s, get_bindings, registry, struct_name)),
        _ => false,
    }
}

/// Extract binding name from `Some(x)` pattern.
pub fn extract_some_binding<'a>(pattern: &'a Pattern<'a>) -> Option<&'a str> {
    if let Pattern::EnumVariant(variant, binding) = pattern {
        if variant == "Some" || variant.ends_with("::Some") {
            if let crate::parser::EnumPatternBinding::Single(name) = binding {
                return Some(name.as_str());
            }
        }
    }
    None
}

/// Extract binding names from tuple patterns like `(Some(a), Some(b))`.
pub fn extract_tuple_some_bindings<'a>(pattern: &'a Pattern<'a>) -> Option<Vec<&'a str>> {
    if let Pattern::Tuple(elements) = pattern {
        let mut bindings = Vec::new();
        for elem in elements {
            if let Some(name) = extract_some_binding(elem) {
                bindings.push(name);
            }
        }
        if !bindings.is_empty() {
            return Some(bindings);
        }
    }
    None
}

/// Given a tuple scrutinee `(a, b, c)` and a variable name, find which position in the
/// tuple the variable occupies. If found, extract the `Some(binding)` name from the
/// corresponding position in the tuple pattern.
///
/// Used by both `let_statement_generation.rs` and `self_analysis.rs` for get→get_mut upgrade
/// and self-mutation detection through tuple match patterns.
pub fn find_binding_for_var_in_tuple_match<'a>(
    scrutinee: &Expression,
    var_name: &str,
    arm_pattern: &'a Pattern<'a>,
) -> Option<&'a str> {
    if let Expression::Tuple { elements, .. } = scrutinee {
        let position = elements
            .iter()
            .position(|e| matches!(e, Expression::Identifier { name, .. } if name == var_name))?;
        if let Pattern::Tuple(pat_elements) = arm_pattern {
            let pat_at_pos = pat_elements.get(position)?;
            return extract_some_binding(pat_at_pos);
        }
    }
    None
}

/// Check if a match arm body expression mutates a given variable.
fn match_arm_body_mutates_var(
    body: &Expression,
    var_name: &str,
    registry: Option<&SignatureRegistry>,
    struct_name: Option<&str>,
) -> bool {
    match body {
        Expression::Block { statements, .. } => statements
            .iter()
            .any(|s| statement_mutates_var(s, var_name, registry, struct_name)),
        Expression::MethodCall { object, method, .. } => {
            if expression_is_var(object, var_name) {
                return method_is_mutating(method, registry, struct_name, None);
            }
            false
        }
        _ => false,
    }
}

/// Check if a statement mutates a given variable (direct assignment or mutating method call).
fn statement_mutates_var(
    stmt: &Statement,
    var_name: &str,
    registry: Option<&SignatureRegistry>,
    struct_name: Option<&str>,
) -> bool {
    match stmt {
        Statement::Assignment { target, .. } => {
            expression_references_variable_or_field(target, var_name)
        }
        Statement::Expression { expr, .. } => {
            expr_mutates_var(expr, var_name, registry, struct_name)
        }
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            then_block
                .iter()
                .any(|s| statement_mutates_var(s, var_name, registry, struct_name))
                || else_block.as_ref().is_some_and(|b| {
                    b.iter()
                        .any(|s| statement_mutates_var(s, var_name, registry, struct_name))
                })
        }
        Statement::While { body, .. }
        | Statement::For { body, .. }
        | Statement::Loop { body, .. } => body
            .iter()
            .any(|s| statement_mutates_var(s, var_name, registry, struct_name)),
        Statement::Match { arms, .. } => arms.iter().any(|arm| {
            if let Expression::Block { statements, .. } = arm.body {
                statements
                    .iter()
                    .any(|s| statement_mutates_var(s, var_name, registry, struct_name))
            } else {
                expr_mutates_var(arm.body, var_name, registry, struct_name)
            }
        }),
        _ => false,
    }
}

/// Check if an expression contains a mutating method call on a variable.
fn expr_mutates_var(
    expr: &Expression,
    var_name: &str,
    registry: Option<&SignatureRegistry>,
    struct_name: Option<&str>,
) -> bool {
    match expr {
        Expression::MethodCall { object, method, .. } => {
            if expression_is_var(object, var_name) || expression_is_field_of_var(object, var_name) {
                if method_is_mutating(method, registry, struct_name, None) {
                    return true;
                }
            }
            false
        }
        Expression::Block { statements, .. } => statements
            .iter()
            .any(|s| statement_mutates_var(s, var_name, registry, struct_name)),
        _ => false,
    }
}

fn expression_is_var(expr: &Expression, var_name: &str) -> bool {
    matches!(expr, Expression::Identifier { name, .. } if name == var_name)
}

fn expression_is_field_of_var(expr: &Expression, var_name: &str) -> bool {
    if let Expression::FieldAccess { object, .. } = expr {
        return expression_is_var(object, var_name);
    }
    false
}

// =============================================================================
// Statement-Level Analysis
// =============================================================================

/// Check if a statement modifies self
pub fn statement_modifies_self(
    stmt: &Statement,
    registry: Option<&SignatureRegistry>,
    struct_name: Option<&str>,
    field_types: Option<
        &std::collections::HashMap<String, std::collections::HashMap<String, crate::parser::Type>>,
    >,
    receiver_upgrades: Option<&std::collections::HashMap<String, crate::analyzer::OwnershipMode>>,
) -> bool {
    match stmt {
        Statement::Let { value, .. } => {
            expression_modifies_self(value, registry, struct_name, field_types, receiver_upgrades)
        }
        Statement::Assignment { target, .. } => {
            // Check if target is self.field
            expression_is_self_field_modification(target)
        }
        Statement::Expression { expr, .. } => {
            expression_modifies_self(expr, registry, struct_name, field_types, receiver_upgrades)
        }
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            then_block.iter().any(|s| {
                statement_modifies_self(s, registry, struct_name, field_types, receiver_upgrades)
            }) || else_block.as_ref().is_some_and(|block| {
                block.iter().any(|s| {
                    statement_modifies_self(
                        s,
                        registry,
                        struct_name,
                        field_types,
                        receiver_upgrades,
                    )
                })
            })
        }
        Statement::While { body, .. } => body.iter().any(|s| {
            statement_modifies_self(s, registry, struct_name, field_types, receiver_upgrades)
        }),
        Statement::For { iterable, body, .. } => {
            expression_modifies_self(
                iterable,
                registry,
                struct_name,
                field_types,
                receiver_upgrades,
            ) || body.iter().any(|s| {
                statement_modifies_self(s, registry, struct_name, field_types, receiver_upgrades)
            })
        }
        Statement::Match { arms, .. } => arms.iter().any(|arm| {
            expression_modifies_self(
                arm.body,
                registry,
                struct_name,
                field_types,
                receiver_upgrades,
            )
        }),
        _ => false,
    }
}

/// Check if a statement accesses struct fields
pub fn statement_accesses_fields(ctx: &AnalysisContext, stmt: &Statement) -> bool {
    match stmt {
        Statement::Expression { expr, .. }
        | Statement::Return {
            value: Some(expr), ..
        } => expression_accesses_fields(ctx, expr),
        Statement::Let { value, .. } => expression_accesses_fields(ctx, value),
        Statement::Assignment { target, value, .. } => {
            expression_accesses_fields(ctx, target) || expression_accesses_fields(ctx, value)
        }
        Statement::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            expression_accesses_fields(ctx, condition)
                || then_block.iter().any(|s| statement_accesses_fields(ctx, s))
                || else_block
                    .as_ref()
                    .is_some_and(|block| block.iter().any(|s| statement_accesses_fields(ctx, s)))
        }
        Statement::While {
            condition, body, ..
        } => {
            expression_accesses_fields(ctx, condition)
                || body.iter().any(|s| statement_accesses_fields(ctx, s))
        }
        Statement::For { iterable, body, .. } => {
            expression_accesses_fields(ctx, iterable)
                || body.iter().any(|s| statement_accesses_fields(ctx, s))
        }
        Statement::Match { value, arms, .. } => {
            expression_accesses_fields(ctx, value)
                || arms
                    .iter()
                    .any(|arm| expression_accesses_fields(ctx, arm.body))
        }
        _ => false,
    }
}

/// Check if a statement mutates struct fields
pub fn statement_mutates_fields(ctx: &AnalysisContext, stmt: &Statement) -> bool {
    match stmt {
        Statement::Assignment { target, .. } => {
            // Check if we're assigning to a field: self.field = ..., self.field[i] = ..., self.a.b = ...
            // Compound assignment (+=, -=, etc.) also mutates the target
            expression_is_field_access(ctx, target)
                || expression_is_self_field_index_access(ctx, target)
        }
        Statement::Expression { expr, .. } => {
            // Check for mutating method calls on fields: self.field.push(...)
            expression_mutates_fields(ctx, expr)
        }
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            then_block.iter().any(|s| statement_mutates_fields(ctx, s))
                || else_block
                    .as_ref()
                    .is_some_and(|block| block.iter().any(|s| statement_mutates_fields(ctx, s)))
        }
        Statement::While { body, .. } => body.iter().any(|s| statement_mutates_fields(ctx, s)),
        Statement::For { iterable, body, .. } => {
            // Check iterable for field mutations too (e.g., self.field.values_mut())
            expression_mutates_fields(ctx, iterable)
                || body.iter().any(|s| statement_mutates_fields(ctx, s))
        }
        Statement::Match { arms, .. } => {
            arms.iter().any(|arm| {
                // MatchArm body is an Expression, need to check for blocks
                expression_mutates_fields(ctx, arm.body)
            })
        }
        _ => false,
    }
}

/// Check if a statement modifies a specific variable
pub fn statement_modifies_variable(stmt: &Statement, var_name: &str) -> bool {
    match stmt {
        Statement::Assignment { target, .. } => {
            // Check if we're assigning to var_name or var_name.field
            expression_references_variable_or_field(target, var_name)
        }
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            then_block
                .iter()
                .any(|s| statement_modifies_variable(s, var_name))
                || else_block.as_ref().is_some_and(|block| {
                    block
                        .iter()
                        .any(|s| statement_modifies_variable(s, var_name))
                })
        }
        Statement::While { body, .. }
        | Statement::For { body, .. }
        | Statement::Loop { body, .. } => body
            .iter()
            .any(|s| statement_modifies_variable(s, var_name)),
        Statement::Match { arms, .. } => arms.iter().any(|arm| {
            if let Expression::Block { statements, .. } = arm.body {
                statements
                    .iter()
                    .any(|s| statement_modifies_variable(s, var_name))
            } else {
                false
            }
        }),
        _ => false,
    }
}

// =============================================================================
// Expression-Level Analysis
// =============================================================================

/// Check if an expression is a self.field modification
pub fn expression_is_self_field_modification(expr: &Expression) -> bool {
    match expr {
        Expression::FieldAccess { object, .. } => {
            matches!(&**object, Expression::Identifier { name, .. } if name == "self")
        }
        _ => false,
    }
}

/// Check if an expression modifies self.
///
/// Uses the `SignatureRegistry` to determine whether method calls on self.field
/// are mutating. This replaces the old hardcoded `is_known_readonly_method` list
/// with proper signature lookup:
///
/// 1. Check stdlib `method_mutates_receiver` (covers Vec::push, HashMap::insert, etc.)
/// 2. Look up the method in the `SignatureRegistry` — if it has a `&self` receiver,
///    it's readonly. If `&mut self` or owned, it's mutating.
/// 3. For unknown methods (not in stdlib and not in registry), default to **not mutating**.
///    The assignment-level check (`self.field = ...`) already catches actual field
///    mutations, so method calls that aren't known-mutating are safe to assume readonly.
pub fn expression_modifies_self(
    expr: &Expression,
    registry: Option<&SignatureRegistry>,
    struct_name: Option<&str>,
    field_types: Option<
        &std::collections::HashMap<String, std::collections::HashMap<String, crate::parser::Type>>,
    >,
    receiver_upgrades: Option<&std::collections::HashMap<String, crate::analyzer::OwnershipMode>>,
) -> bool {
    match expr {
        Expression::Block { statements, .. } => statements.iter().any(|s| {
            statement_modifies_self(s, registry, struct_name, field_types, receiver_upgrades)
        }),
        Expression::MethodCall {
            object,
            method,
            arguments,
            ..
        } => {
            let receiver_mutates = matches!(&**object, Expression::Identifier { name, .. } if name == "self")
                || is_self_field_chain(object);
            if receiver_mutates
                && method_is_mutating_on_receiver(
                    object,
                    method,
                    registry,
                    struct_name,
                    field_types,
                    receiver_upgrades,
                )
            {
                return true;
            }

            arguments.iter().any(|(_, arg)| {
                expression_modifies_self(arg, registry, struct_name, field_types, receiver_upgrades)
            })
        }
        Expression::Call { arguments, .. } => arguments.iter().any(|(_, arg)| {
            expression_modifies_self(arg, registry, struct_name, field_types, receiver_upgrades)
        }),
        Expression::Binary { left, right, .. } => {
            expression_modifies_self(left, registry, struct_name, field_types, receiver_upgrades)
                || expression_modifies_self(
                    right,
                    registry,
                    struct_name,
                    field_types,
                    receiver_upgrades,
                )
        }
        Expression::Unary { operand, .. } => expression_modifies_self(
            operand,
            registry,
            struct_name,
            field_types,
            receiver_upgrades,
        ),
        Expression::Cast { expr, .. } => {
            expression_modifies_self(expr, registry, struct_name, field_types, receiver_upgrades)
        }
        _ => false,
    }
}

/// Determine if a method call on a receiver chain is mutating.
fn method_is_mutating_on_receiver(
    receiver: &Expression,
    method: &str,
    registry: Option<&SignatureRegistry>,
    struct_name: Option<&str>,
    field_types: Option<
        &std::collections::HashMap<String, std::collections::HashMap<String, crate::parser::Type>>,
    >,
    receiver_upgrades: Option<&std::collections::HashMap<String, crate::analyzer::OwnershipMode>>,
) -> bool {
    if let (Some(ctx), Some(fields)) = (struct_name, field_types) {
        if is_self_field_chain(receiver) {
            if let Some(recv_type) = resolve_self_field_chain_type_name(receiver, ctx, fields) {
                let qname = format!("{}::{}", recv_type, method);
                if let Some(upgrades) = receiver_upgrades {
                    if let Some(mode) = upgrades.get(&qname) {
                        return match mode {
                            OwnershipMode::MutBorrowed => true,
                            // Owned receiver at call site does not imply field mutation.
                            OwnershipMode::Owned => false,
                            _ => false,
                        };
                    }
                }
                if let Some(reg) = registry {
                    if let Some(sig) = reg.get_signature(&qname) {
                        if let Some(mode) = sig.param_ownership.first() {
                            return match mode {
                                OwnershipMode::MutBorrowed => true,
                                OwnershipMode::Owned => false,
                                _ => false,
                            };
                        }
                        if sig.has_self_receiver {
                            return false;
                        }
                    }
                }
                return method_is_mutating(
                    method,
                    registry,
                    Some(recv_type.as_str()),
                    receiver_upgrades,
                );
            }
        }
    }
    method_is_mutating(method, registry, struct_name, receiver_upgrades)
}

/// Determine if a method call is mutating by consulting the stdlib method registry
/// and the user's SignatureRegistry.
///
/// Priority:
/// 1. stdlib `method_mutates_receiver` → definitively mutating (push, insert, clear, etc.)
/// 2. Codegen self-receiver upgrades (same-impl / prior-file &mut self)
/// 3. SignatureRegistry lookup → use the analyzed ownership of the self receiver
/// 4. Unknown → default to not-mutating (assignment detection covers actual field writes)
fn method_is_mutating(
    method: &str,
    registry: Option<&SignatureRegistry>,
    struct_name: Option<&str>,
    receiver_upgrades: Option<&std::collections::HashMap<String, crate::analyzer::OwnershipMode>>,
) -> bool {
    if crate::analyzer::stdlib_method_traits::is_known_readonly(method) {
        return false;
    }
    if super::stdlib_method_traits::method_mutates_receiver(method) {
        return true;
    }

    if let Some(ctx) = struct_name {
        let qualified = format!("{}::{}", ctx, method);
        if let Some(upgrades) = receiver_upgrades {
            if let Some(mode) = upgrades.get(&qualified) {
                return match mode {
                    OwnershipMode::MutBorrowed => true,
                    OwnershipMode::Owned => false,
                    _ => false,
                };
            }
        }
    }

    if let Some(reg) = registry {
        if let Some(ctx) = struct_name {
            let qualified = format!("{}::{}", ctx, method);
            if let Some(sig) = reg.get_signature(&qualified) {
                if let Some(mode) = sig.param_ownership.first() {
                    return match mode {
                        OwnershipMode::MutBorrowed => true,
                        OwnershipMode::Owned => false,
                        _ => false,
                    };
                }
                return false;
            }
            return false;
        }
        if let Some(sig) = lookup_method_in_registry(reg, method) {
            return sig.has_self_receiver
                && sig.param_ownership.first() == Some(&OwnershipMode::MutBorrowed);
        }
    }

    false
}

/// Look up a method in the signature registry by suffix match.
/// Only used as a conservative fallback when the struct context is unknown.
/// Returns Some if ANY type's method with this name exists — conservative for
/// mutation checks (false positive is safe, false negative is not).
fn lookup_method_in_registry<'a>(
    registry: &'a SignatureRegistry,
    method: &str,
) -> Option<&'a crate::analyzer::FunctionSignature> {
    registry.lookup_method(method)
}

pub(crate) fn is_self_field_chain(expr: &Expression) -> bool {
    match expr {
        Expression::FieldAccess { object, .. } => {
            matches!(&**object, Expression::Identifier { name, .. } if name == "self")
                || is_self_field_chain(object)
        }
        Expression::Index { object, .. } => is_self_field_chain(object),
        _ => false,
    }
}

/// Resolve the Rust type name at the end of a `self.field[...]` chain for method lookup.
pub fn resolve_self_field_chain_type_name(
    expr: &Expression,
    struct_name: &str,
    field_types: &std::collections::HashMap<
        String,
        std::collections::HashMap<String, crate::parser::Type>,
    >,
) -> Option<String> {
    let fields = field_types.get(struct_name)?;
    resolve_self_field_chain_type(expr, fields).and_then(|ty| type_to_custom_name(&ty))
}

fn type_to_custom_name(ty: &crate::parser::Type) -> Option<String> {
    match ty {
        crate::parser::Type::Custom(name) => Some(name.clone()),
        crate::parser::Type::Parameterized(name, _) => Some(name.clone()),
        _ => None,
    }
}

fn resolve_self_field_chain_type(
    expr: &Expression,
    fields: &std::collections::HashMap<String, crate::parser::Type>,
) -> Option<crate::parser::Type> {
    use crate::parser::Type;
    match expr {
        Expression::FieldAccess { object, field, .. } => {
            if matches!(&**object, Expression::Identifier { name, .. } if name == "self") {
                fields.get(field.as_str()).cloned()
            } else {
                let base = resolve_self_field_chain_type(object, fields)?;
                lookup_field_on_type(&base, field)
            }
        }
        Expression::Index { object, .. } => {
            let base = resolve_self_field_chain_type(object, fields)?;
            match base {
                Type::Vec(inner) | Type::Array(inner, _) => Some(*inner),
                Type::Parameterized(name, params) if name == "Vec" && !params.is_empty() => {
                    Some(params[0].clone())
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn lookup_field_on_type(ty: &crate::parser::Type, field: &str) -> Option<crate::parser::Type> {
    // Nested field access on non-struct types is rare in this path; defer to None.
    let _ = (ty, field);
    None
}

/// Check if an expression accesses struct fields
pub fn expression_accesses_fields(ctx: &AnalysisContext, expr: &Expression) -> bool {
    match expr {
        Expression::Identifier { name, .. } => {
            // Parameters and locals shadow struct fields — only bare field names imply self.
            !ctx.shadows_struct_field(name) && ctx.current_struct_fields.contains(name)
        }
        Expression::FieldAccess { object, .. } => {
            // Check for self.field or nested field access
            if let Expression::Identifier { name: obj_name, .. } = &**object {
                obj_name == "self"
            } else {
                expression_accesses_fields(ctx, object)
            }
        }
        Expression::MethodCall {
            object, arguments, ..
        } => {
            expression_accesses_fields(ctx, object)
                || arguments
                    .iter()
                    .any(|(_, arg)| expression_accesses_fields(ctx, arg))
        }
        Expression::Call { arguments, .. } => arguments
            .iter()
            .any(|(_, arg)| expression_accesses_fields(ctx, arg)),
        Expression::Binary { left, right, .. } => {
            expression_accesses_fields(ctx, left) || expression_accesses_fields(ctx, right)
        }
        Expression::Unary { operand, .. } => expression_accesses_fields(ctx, operand),
        Expression::Index { object, index, .. } => {
            expression_accesses_fields(ctx, object) || expression_accesses_fields(ctx, index)
        }
        Expression::StructLiteral { fields, .. } => fields
            .iter()
            .any(|(_, expr)| expression_accesses_fields(ctx, expr)),
        Expression::MapLiteral { pairs, .. } => pairs
            .iter()
            .any(|(k, v)| expression_accesses_fields(ctx, k) || expression_accesses_fields(ctx, v)),
        Expression::Array { elements, .. } => {
            elements.iter().any(|e| expression_accesses_fields(ctx, e))
        }
        Expression::Tuple { elements, .. } => {
            elements.iter().any(|e| expression_accesses_fields(ctx, e))
        }
        Expression::Closure { body, .. } => expression_accesses_fields(ctx, body),
        Expression::TryOp { expr, .. }
        | Expression::Await { expr, .. }
        | Expression::Cast { expr, .. } => expression_accesses_fields(ctx, expr),
        Expression::MacroInvocation { args, .. } => {
            // Check if any macro arguments access fields
            args.iter().any(|arg| expression_accesses_fields(ctx, arg))
        }
        Expression::Range { start, end, .. } => {
            expression_accesses_fields(ctx, start) || expression_accesses_fields(ctx, end)
        }
        Expression::ChannelSend { channel, value, .. } => {
            expression_accesses_fields(ctx, channel) || expression_accesses_fields(ctx, value)
        }
        Expression::ChannelRecv { channel, .. } => expression_accesses_fields(ctx, channel),
        Expression::Block { statements, .. } => {
            // Check if any statement in the block accesses fields
            statements.iter().any(|s| statement_accesses_fields(ctx, s))
        }
        _ => false,
    }
}

/// Check if an expression is a field access (self.field, self.field.subfield, or bare field)
pub fn expression_is_field_access(ctx: &AnalysisContext, expr: &Expression) -> bool {
    match expr {
        Expression::Identifier { name, .. } => ctx.current_struct_fields.contains(name),
        Expression::FieldAccess { object, .. } => {
            match &**object {
                Expression::Identifier { name: obj_name, .. } => obj_name == "self",
                // Nested: self.field.subfield or self.field[i].subfield
                Expression::FieldAccess { .. } => expression_is_field_access(ctx, object),
                Expression::Index { .. } => expression_is_self_field_index_access(ctx, object),
                _ => false,
            }
        }
        _ => false,
    }
}

/// Check if an expression is an index access on a self field (self.field[i] or self.field[i][j])
fn expression_is_self_field_index_access(ctx: &AnalysisContext, expr: &Expression) -> bool {
    match expr {
        Expression::Index { object, .. } => {
            expression_is_field_access(ctx, object)
                || expression_is_self_field_index_access(ctx, object)
        }
        _ => false,
    }
}

/// Check if an expression mutates struct fields
pub fn expression_mutates_fields(ctx: &AnalysisContext, expr: &Expression) -> bool {
    match expr {
        Expression::Block { statements, .. } => {
            // Check if any statement in the block mutates fields
            statements.iter().any(|s| statement_mutates_fields(ctx, s))
        }
        Expression::MethodCall { object, method, .. } => {
            // Check if this is a mutating method call on a field: self.field.push(...) or self.field[i].push(...)
            if expression_is_field_access(ctx, object)
                || expression_is_self_field_index_access(ctx, object)
            {
                super::stdlib_method_traits::method_mutates_receiver(method)
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Check if an expression references a variable or its fields
pub fn expression_references_variable_or_field(expr: &Expression, var_name: &str) -> bool {
    match expr {
        Expression::Identifier { name, .. } => name == var_name,
        Expression::FieldAccess { object, .. } => {
            // Check if object is the variable
            if let Expression::Identifier { name, .. } = &**object {
                name == var_name
            } else {
                expression_references_variable_or_field(object, var_name)
            }
        }
        // TDD FIX: Dereference expressions (*val = ...) also reference the variable
        // For: *val = value, need to detect that 'val' is being mutated
        Expression::Unary {
            op: crate::parser::UnaryOp::Deref,
            operand,
            ..
        } => expression_references_variable_or_field(operand, var_name),
        _ => false,
    }
}

// =============================================================================
// Loop-Specific Analysis
// =============================================================================

/// Check if a loop body modifies a variable
pub fn loop_body_modifies_variable(body: &[Statement], var_name: &str) -> bool {
    for stmt in body {
        if statement_modifies_variable(stmt, var_name) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_alloc_expr;

    // Basic smoke tests - comprehensive tests are in tests/codegen_self_analysis_test.rs

    #[test]
    fn test_expression_is_self_field_modification_basic() {
        use crate::parser::Expression;
        use crate::source_map::Location;
        use std::path::PathBuf;

        let loc = Location {
            file: PathBuf::from("test.wj"),
            line: 1,
            column: 1,
        };
        let self_expr = test_alloc_expr(Expression::Identifier {
            name: "self".to_string(),
            location: Some(loc.clone()),
        });
        let field_access = Expression::FieldAccess {
            object: self_expr,
            field: "x".to_string(),
            location: Some(loc),
        };

        assert!(expression_is_self_field_modification(&field_access));
    }

    #[test]
    fn test_expression_is_self_field_modification_not_self() {
        use crate::parser::Expression;
        use crate::source_map::Location;
        use std::path::PathBuf;

        let loc = Location {
            file: PathBuf::from("test.wj"),
            line: 1,
            column: 1,
        };
        let other_expr = test_alloc_expr(Expression::Identifier {
            name: "other".to_string(),
            location: Some(loc.clone()),
        });
        let field_access = Expression::FieldAccess {
            object: other_expr,
            field: "x".to_string(),
            location: Some(loc),
        };

        assert!(!expression_is_self_field_modification(&field_access));
    }

    // More comprehensive tests will be added later
    // These are just basic smoke tests for the module
}
