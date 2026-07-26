//! Constraint generation via full AST walk.
//!
//! Phase 1A of the Safety-Typed IR pipeline. Walks the function body (expressions,
//! statements, calls) and emits first-principles constraints for the unified solver:
//!
//!   - `TypeIs` for literals, declared types, cast targets
//!   - `TypeEquals` for assignments, returns, call arguments, let bindings
//!   - `OwnershipIs` from usage patterns (field mutation, method calls)
//!   - `SharesRegion` for aliasing detection (multiple refs to same variable)
//!   - `IsNumeric`/`IsInteger`/`IsFloat` for arithmetic expressions
//!   - `NeedsClone` when a variable is used at multiple owned sites
//!   - `HasEffects`/`EffectsUnion` for call-graph effect propagation
//!
//! Analyzer metadata is no longer seeded as ownership constraints — the solver
//! derives ownership from declared types, body usage, and registry call-site unification.

use crate::analyzer::{AnalyzedFunction, FunctionSignature, OwnershipMode, SignatureRegistry};
use crate::ir::constraints::{Constraint, ConstraintSet, ConstraintVar};
use crate::ir::execution::{CallLocation, CallSite, ExecutionConstraint};
use crate::ir::node::{
    ir_function_name_from_decl, ownership_mode_from_param_type, ownership_mode_to_owned_type,
    param_ownership_seed_is_copy, parser_type_to_base_type,
};
use crate::ir::safety_type::{BaseType, Effect, EffectSet, ExecutionMode, OwnedType, Region};
use crate::parser::ast::core::{Expression, Pattern, Statement};
use crate::parser::ast::types::Type;
use crate::parser::ast::literals::Literal;
use crate::parser::ast::operators::BinaryOp;
use std::collections::HashMap;

/// Maps between function entities and their constraint variables.
#[derive(Debug, Clone)]
pub struct ConstraintVarMap {
    /// Parameter name → constraint var
    pub params: HashMap<String, ConstraintVar>,
    /// Return value constraint var
    pub return_var: ConstraintVar,
    /// Local variable name → constraint var (populated during body walk)
    pub locals: HashMap<String, ConstraintVar>,
}

/// Result of constraint generation for a single function.
#[derive(Debug)]
pub struct FunctionConstraints {
    pub function_name: String,
    pub constraints: ConstraintSet,
    pub var_map: ConstraintVarMap,
    /// Callee names resolved from call expressions (for effect propagation).
    pub call_targets: Vec<String>,
    /// Taint constraints extracted from the AST (for the taint solver).
    pub taint_constraints: Vec<crate::ir::taint::TaintConstraint>,
    /// Execution mode constraints from `async`/`spawn` call prefixes (WJ-CONC-01).
    pub execution_constraints: Vec<ExecutionConstraint>,
}

/// Walks the AST body and emits constraints for the unified solver.
struct AstConstraintWalker<'a, 'ast> {
    cs: ConstraintSet,
    param_vars: HashMap<String, ConstraintVar>,
    local_vars: HashMap<String, ConstraintVar>,
    return_var: ConstraintVar,
    region_counter: u32,
    owned_use_counts: HashMap<String, u32>,
    analyzed: &'a AnalyzedFunction<'ast>,
    /// Qualified callee names from call expressions (for effect call-graph).
    call_targets: Vec<String>,
    /// Taint constraints from this function's AST.
    taint_constraints: Vec<crate::ir::taint::TaintConstraint>,
    /// Execution mode constraints from `async`/`spawn` call prefixes.
    execution_constraints: Vec<ExecutionConstraint>,
    /// Optional signature registry for call-site unification.
    registry: Option<&'a SignatureRegistry>,
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    fn new(analyzed: &'a AnalyzedFunction<'ast>, registry: Option<&'a SignatureRegistry>) -> Self {
        let mut cs = ConstraintSet::new();
        let mut param_vars = HashMap::new();
        let mut region_counter: u32 = 1;

        // Create constraint vars for each parameter from declared types
        for param in &analyzed.decl.parameters {
            let var = cs.fresh_var();
            param_vars.insert(param.name.clone(), var);

            let base = parser_type_to_base_type(&param.type_);
            if base != BaseType::Inferred {
                cs.add(Constraint::TypeIs(var, base.clone()));
                emit_numeric_class_constraint(&mut cs, var, &base);
            }
        }

        // Ownership seeds: explicit borrows and Copy types only.
        // Non-copy bare `T` params are resolved by body usage, call sites, and impl convergence.
        for param in &analyzed.decl.parameters {
            if let Some(&var) = param_vars.get(&param.name) {
                match &param.type_ {
                    Type::Reference(_) | Type::MutableReference(_) => {
                        let mode = ownership_mode_from_param_type(&param.type_);
                        let ownership = ownership_mode_to_owned_type(mode, &mut region_counter);
                        cs.add(Constraint::OwnershipIs(var, ownership));
                    }
                    ty if param_ownership_seed_is_copy(ty) => {
                        // Copy default is owned, but readonly body analysis (field access only)
                        // must beat the seed so APIs like `replay_to_lsn(..., through: &Lsn)` emit.
                        let analyzer_borrowed = analyzed
                            .inferred_ownership
                            .get(&param.name)
                            .is_some_and(|m| {
                                matches!(
                                    m,
                                    crate::analyzer::OwnershipMode::Borrowed
                                        | crate::analyzer::OwnershipMode::MutBorrowed
                                )
                            });
                        if !analyzer_borrowed {
                            cs.add(Constraint::OwnershipIs(var, OwnedType::Owned));
                        }
                    }
                    _ => {}
                }
            }
        }

        // Returned parameters stay owned even when the body mutates them.
        for param_name in &analyzed.returned_parameters {
            if let Some(&var) = param_vars.get(param_name) {
                cs.add(Constraint::OwnershipIs(var, OwnedType::Owned));
            }
        }

        // Analyzer ownership as solver constraints (not codegen authority).
        // Conflicts resolve in the solver: Owned/MutRef beat Ref for passthrough callees.
        for (param_name, ownership_mode) in &analyzed.inferred_ownership {
            if analyzed.str_ref_optimizable_params.contains(param_name) {
                continue;
            }
            if let Some(&var) = param_vars.get(param_name) {
                let ownership = match ownership_mode {
                    OwnershipMode::Owned => OwnedType::Owned,
                    OwnershipMode::Borrowed => {
                        let r = Region::fresh(region_counter);
                        region_counter += 1;
                        OwnedType::Ref(r)
                    }
                    OwnershipMode::MutBorrowed => {
                        let r = Region::fresh(region_counter);
                        region_counter += 1;
                        OwnedType::MutRef(r)
                    }
                };
                cs.add(Constraint::OwnershipIs(var, ownership));
            }
        }

        // Apply inferred param type overrides from analyzer (type only, not ownership).
        for (idx, inferred_ty) in analyzed.inferred_param_types.iter().enumerate() {
            if let Some(param) = analyzed.decl.parameters.get(idx) {
                if let Some(&var) = param_vars.get(&param.name) {
                    let base = parser_type_to_base_type(inferred_ty);
                    if base != BaseType::Inferred {
                        cs.add(Constraint::TypeIs(var, base.clone()));
                        emit_numeric_class_constraint(&mut cs, var, &base);
                    }
                }
            }
        }

        // str_ref optimization: link to actual param var
        for param_name in &analyzed.str_ref_optimizable_params {
            if let Some(&var) = param_vars.get(param_name) {
                cs.add(Constraint::TypeIs(var, BaseType::String));
                cs.add(Constraint::OwnershipIs(
                    var,
                    OwnedType::Ref(Region::fresh(region_counter)),
                ));
                region_counter += 1;
            }
        }

        // Return type constraint var
        let return_var = cs.fresh_var();
        let return_base = analyzed
            .decl
            .return_type
            .as_ref()
            .map(|rt| parser_type_to_base_type(rt))
            .unwrap_or(BaseType::Unit);
        if return_base != BaseType::Inferred {
            cs.add(Constraint::TypeIs(return_var, return_base.clone()));
            emit_numeric_class_constraint(&mut cs, return_var, &return_base);
        }
        cs.add(Constraint::OwnershipIs(return_var, OwnedType::Owned));

        Self {
            cs,
            param_vars,
            local_vars: HashMap::new(),
            return_var,
            region_counter,
            owned_use_counts: HashMap::new(),
            analyzed,
            call_targets: Vec::new(),
            taint_constraints: Vec::new(),
            execution_constraints: Vec::new(),
            registry,
        }
    }

    fn ownership_mode_to_owned(&mut self, mode: OwnershipMode) -> OwnedType {
        match mode {
            OwnershipMode::Owned => OwnedType::Owned,
            OwnershipMode::Borrowed => {
                let r = self.fresh_region();
                OwnedType::Ref(r)
            }
            OwnershipMode::MutBorrowed => {
                let r = self.fresh_region();
                OwnedType::MutRef(r)
            }
        }
    }

    /// Unify call-site argument vars with callee formal expectations.
    fn emit_call_site_constraints(&mut self, sig: &FunctionSignature, arg_vars: &[ConstraintVar]) {
        for (arg_index, &arg_var) in arg_vars.iter().enumerate() {
            let param_idx = sig.arg_param_index(arg_index);
            let expected_var = self.cs.fresh_var();

            if let Some(ty) = sig
                .formal_param_type(param_idx)
                .or_else(|| sig.param_types.get(param_idx))
            {
                let base = parser_type_to_base_type(ty);
                if base != BaseType::Inferred {
                    self.cs.add(Constraint::TypeIs(expected_var, base));
                }
            }

            if let Some(mode) = sig.param_ownership.get(param_idx) {
                let own = self.ownership_mode_to_owned(*mode);
                self.cs.add(Constraint::OwnershipIs(expected_var, own));
            }

            self.cs
                .add(Constraint::TypeEquals(arg_var, expected_var));
        }
    }

    fn resolve_callee_signature(
        &self,
        name: &str,
        arg_count: usize,
        has_receiver: bool,
    ) -> Option<&FunctionSignature> {
        let registry = self.registry?;
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

    /// Resolve or create a constraint var for a variable name.
    fn resolve_var(&mut self, name: &str) -> ConstraintVar {
        if let Some(&v) = self.param_vars.get(name) {
            return v;
        }
        if let Some(&v) = self.local_vars.get(name) {
            return v;
        }
        let v = self.cs.fresh_var();
        self.local_vars.insert(name.to_string(), v);
        v
    }

    fn fresh_region(&mut self) -> Region {
        let r = Region::fresh(self.region_counter);
        self.region_counter += 1;
        r
    }

    /// Extract a qualified callee name from a call target expression.
    /// Returns `Some("std::fs::read")` for path-like calls, `Some("foo")` for simple identifiers.
    fn extract_callee_name(expr: &Expression<'ast>) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, field, .. } => {
                if let Some(prefix) = Self::extract_callee_name(object) {
                    Some(format!("{}::{}", prefix, field))
                } else {
                    Some(field.clone())
                }
            }
            _ => None,
        }
    }

    /// Resolve the Windjammer type name of a method receiver for `Type::method` lookup.
    fn receiver_type_name_for_method(&self, object: &Expression<'ast>) -> Option<String> {
        match object {
            Expression::Identifier { name, .. } => self
                .analyzed
                .decl
                .parameters
                .iter()
                .find(|p| p.name == *name)
                .and_then(|p| Self::type_name_for_method_lookup(&p.type_)),
            Expression::FieldAccess { object, .. } => self.receiver_type_name_for_method(object),
            _ => None,
        }
    }

    fn type_name_for_method_lookup(ty: &Type) -> Option<String> {
        match ty {
            Type::Custom(name) => Some(name.clone()),
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::type_name_for_method_lookup(inner)
            }
            Type::Vec(_) => Some("Vec".to_string()),
            Type::String => Some("String".to_string()),
            _ => None,
        }
    }

    fn emit_execution_call_mode(
        &mut self,
        inner: &Expression<'ast>,
        mode: ExecutionMode,
        location: &crate::parser::SourceLocation,
    ) {
        let callee = match inner {
            Expression::Call { function, .. } => Self::extract_callee_name(function),
            _ => Self::extract_callee_name(inner),
        };
        if let Some(callee) = callee {
            let call_location = location.as_ref().map(|loc| CallLocation {
                file: loc.file.to_string_lossy().into_owned(),
                line: loc.line,
                col: loc.column,
            }).unwrap_or(CallLocation {
                file: String::new(),
                line: 0,
                col: 0,
            });
            self.execution_constraints
                .push(ExecutionConstraint::CallMode {
                    site: CallSite {
                        callee,
                        mode,
                        location: call_location,
                    },
                });
        }
    }

    /// Walk the entire function body.
    fn walk_body(&mut self) {
        for stmt in &self.analyzed.decl.body {
            self.walk_statement(stmt);
        }
    }

    fn walk_statements(&mut self, stmts: &[&'ast Statement<'ast>]) {
        for stmt in stmts {
            self.walk_statement(stmt);
        }
    }

    fn walk_statement(&mut self, stmt: &Statement<'ast>) {
        match stmt {
            Statement::Let {
                pattern,
                type_,
                value,
                mutable,
                else_block,
                ..
            } => {
                let val_var = self.walk_expression(value);
                let bound_var = self.bind_pattern(pattern);
                self.cs.add(Constraint::TypeEquals(bound_var, val_var));
                if let Some(ty) = type_ {
                    let base = parser_type_to_base_type(ty);
                    if base != BaseType::Inferred {
                        self.cs.add(Constraint::TypeIs(bound_var, base.clone()));
                        emit_numeric_class_constraint(&mut self.cs, bound_var, &base);
                    }
                }
                if *mutable {
                    // Mutable local — solver may need to track mutability
                    // for now, just ensure the var is in locals
                    if let Pattern::Identifier(name) = pattern {
                        self.local_vars.entry(name.clone()).or_insert(bound_var);
                    }
                }
                if let Some(else_stmts) = else_block {
                    self.walk_statements(else_stmts);
                }
            }

            Statement::Const { type_, value, name, .. } => {
                let val_var = self.walk_expression(value);
                let var = self.resolve_var(name);
                self.cs.add(Constraint::TypeEquals(var, val_var));
                let base = parser_type_to_base_type(type_);
                if base != BaseType::Inferred {
                    self.cs.add(Constraint::TypeIs(var, base.clone()));
                    emit_numeric_class_constraint(&mut self.cs, var, &base);
                }
            }

            Statement::Static {
                type_, value, name, ..
            } => {
                let val_var = self.walk_expression(value);
                let var = self.resolve_var(name);
                self.cs.add(Constraint::TypeEquals(var, val_var));
                let base = parser_type_to_base_type(type_);
                if base != BaseType::Inferred {
                    self.cs.add(Constraint::TypeIs(var, base.clone()));
                    emit_numeric_class_constraint(&mut self.cs, var, &base);
                }
            }

            Statement::Assignment { target, value, .. } => {
                let target_var = self.walk_expression(target);
                let val_var = self.walk_expression(value);
                self.cs.add(Constraint::TypeEquals(target_var, val_var));
                self.detect_mutation_target(target);
            }

            Statement::Return { value, .. } => {
                if let Some(val) = value {
                    let val_var = self.walk_expression(val);
                    self.cs.add(Constraint::TypeEquals(self.return_var, val_var));
                }
            }

            Statement::Expression { expr, .. } => {
                self.walk_expression(expr);
            }

            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let cond_var = self.walk_expression(condition);
                self.cs.add(Constraint::TypeIs(cond_var, BaseType::Bool));
                self.walk_statements(then_block);
                if let Some(else_stmts) = else_block {
                    self.walk_statements(else_stmts);
                }
            }

            Statement::Match { value, arms, .. } => {
                self.walk_expression(value);
                for arm in arms {
                    if let Some(guard) = arm.guard {
                        let guard_var = self.walk_expression(guard);
                        self.cs.add(Constraint::TypeIs(guard_var, BaseType::Bool));
                    }
                    self.walk_expression(arm.body);
                }
            }

            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                self.walk_expression(iterable);
                self.bind_pattern(pattern);
                self.walk_statements(body);
            }

            Statement::Loop { body, .. } => {
                self.walk_statements(body);
            }

            Statement::While {
                condition, body, ..
            } => {
                let cond_var = self.walk_expression(condition);
                self.cs.add(Constraint::TypeIs(cond_var, BaseType::Bool));
                self.walk_statements(body);
            }

            Statement::Thread { body, .. } | Statement::Async { body, .. } => {
                self.walk_statements(body);
            }

            Statement::Defer { statement, .. } => {
                self.walk_statement(statement);
            }

            Statement::Break { .. } | Statement::Continue { .. } | Statement::Use { .. } => {}
        }
    }

    /// Walk an expression, returning a constraint var representing its result type.
    fn walk_expression(&mut self, expr: &Expression<'ast>) -> ConstraintVar {
        match expr {
            Expression::Literal { value, .. } => {
                let var = self.cs.fresh_var();
                let base = literal_to_base_type(value);
                self.cs.add(Constraint::TypeIs(var, base.clone()));
                emit_numeric_class_constraint(&mut self.cs, var, &base);
                var
            }

            Expression::Identifier { name, .. } => {
                self.resolve_var(name)
            }

            Expression::Binary {
                left, op, right, ..
            } => {
                let l = self.walk_expression(left);
                let r = self.walk_expression(right);
                let result = self.cs.fresh_var();

                if is_comparison_op(op) {
                    // Comparisons: operands should be same type, result is bool
                    self.cs.add(Constraint::TypeEquals(l, r));
                    self.cs.add(Constraint::TypeIs(result, BaseType::Bool));
                } else if is_logical_op(op) {
                    // Logical ops: operands and result are bool
                    self.cs.add(Constraint::TypeIs(l, BaseType::Bool));
                    self.cs.add(Constraint::TypeIs(r, BaseType::Bool));
                    self.cs.add(Constraint::TypeIs(result, BaseType::Bool));
                } else if is_arithmetic_op(op) {
                    // Arithmetic: operands same numeric type, result same type
                    self.cs.add(Constraint::TypeEquals(l, r));
                    self.cs.add(Constraint::TypeEquals(result, l));
                    self.cs.add(Constraint::IsNumeric(l));
                    self.cs.add(Constraint::IsNumeric(r));
                } else if is_bitwise_op(op) {
                    // Bitwise: operands same integer type, result same type
                    self.cs.add(Constraint::TypeEquals(l, r));
                    self.cs.add(Constraint::TypeEquals(result, l));
                    self.cs.add(Constraint::IsInteger(l));
                    self.cs.add(Constraint::IsInteger(r));
                } else {
                    // Other binary ops: unify operands and result
                    self.cs.add(Constraint::TypeEquals(l, r));
                    self.cs.add(Constraint::TypeEquals(result, l));
                }
                result
            }

            Expression::Unary { op, operand, .. } => {
                let inner = self.walk_expression(operand);
                let result = self.cs.fresh_var();
                match op {
                    crate::parser::ast::operators::UnaryOp::Not => {
                        self.cs.add(Constraint::TypeIs(result, BaseType::Bool));
                    }
                    crate::parser::ast::operators::UnaryOp::Neg => {
                        self.cs.add(Constraint::TypeEquals(result, inner));
                        self.cs.add(Constraint::IsNumeric(inner));
                    }
                    _ => {
                        self.cs.add(Constraint::TypeEquals(result, inner));
                    }
                }
                result
            }

            Expression::Call {
                function,
                arguments,
                ..
            } => {
                let _callee_var = self.walk_expression(function);
                let mut arg_vars = Vec::new();
                for (_label, arg_expr) in arguments {
                    arg_vars.push(self.walk_expression(arg_expr));
                }
                let result = self.cs.fresh_var();

                if let Some(callee_name) = Self::extract_callee_name(function) {
                    self.call_targets.push(callee_name.clone());

                    if let Some(sig) = self
                        .resolve_callee_signature(&callee_name, arguments.len(), false)
                        .cloned()
                    {
                        self.emit_call_site_constraints(&sig, &arg_vars);
                    }

                    let callee_effects = lookup_stdlib_effects(&callee_name);
                    if !callee_effects.is_empty() {
                        self.cs.add(Constraint::HasEffects(
                            result,
                            EffectSet::from_iter(callee_effects),
                        ));
                    }

                    self.emit_taint_for_call(&callee_name, &self.analyzed.decl.name.clone());
                }

                self.cs
                    .add(Constraint::EffectsUnion(result, arg_vars));
                result
            }

            Expression::MethodCall {
                object,
                arguments,
                method,
                ..
            } => {
                let obj_var = self.walk_expression(object);
                let mut arg_vars = Vec::new();
                for (_label, arg_expr) in arguments {
                    arg_vars.push(self.walk_expression(arg_expr));
                }

                // Signature-driven receiver ownership: `Type::method` self mode wins.
                // Fallback to the stdlib mutating-name set only when no signature exists.
                let receiver_type = self.receiver_type_name_for_method(object);
                let qualified = match &receiver_type {
                    Some(ty) => format!("{}::{}", ty, method),
                    None => {
                        if let Some(receiver_name) = Self::extract_callee_name(object) {
                            format!("{}::{}", receiver_name, method)
                        } else {
                            method.clone()
                        }
                    }
                };
                self.call_targets.push(qualified.clone());

                let sig = self
                    .resolve_callee_signature(&qualified, arguments.len(), true)
                    .or_else(|| self.resolve_callee_signature(method, arguments.len(), true))
                    .cloned();

                let mut_self = sig.as_ref().is_some_and(|s| {
                    s.has_self_receiver
                        && s.param_ownership
                            .first()
                            .is_some_and(|m| matches!(m, OwnershipMode::MutBorrowed))
                });
                if mut_self || (sig.is_none() && is_mutating_method(method)) {
                    let r = self.fresh_region();
                    self.cs
                        .add(Constraint::OwnershipIs(obj_var, OwnedType::MutRef(r)));
                }

                let result = self.cs.fresh_var();

                if let Some(sig) = sig.as_ref() {
                    self.emit_call_site_constraints(sig, &arg_vars);
                }

                let callee_effects = lookup_stdlib_effects(&qualified);
                if !callee_effects.is_empty() {
                    self.cs.add(Constraint::HasEffects(
                        obj_var,
                        EffectSet::from_iter(callee_effects),
                    ));
                }

                self.emit_taint_for_call(&qualified, &self.analyzed.decl.name.clone());

                self.cs
                    .add(Constraint::EffectsUnion(result, vec![obj_var]));
                result
            }

            Expression::FieldAccess { object, .. } => {
                if let Some(name) = self.root_identifier(object) {
                    if let Some(&var) = self.param_vars.get(&name) {
                        let readonly_borrow = self
                            .analyzed
                            .inferred_ownership
                            .get(&name)
                            .is_some_and(|m| {
                                matches!(
                                    m,
                                    crate::analyzer::OwnershipMode::Borrowed
                                        | crate::analyzer::OwnershipMode::MutBorrowed
                                )
                            });
                        let copy_field_read = self
                            .analyzed
                            .decl
                            .parameters
                            .iter()
                            .find(|p| p.name == name)
                            .is_some_and(|p| param_ownership_seed_is_copy(&p.type_));
                        if readonly_borrow || copy_field_read {
                            let r = self.fresh_region();
                            self.cs
                                .add(Constraint::OwnershipIs(var, OwnedType::Ref(r)));
                        }
                    }
                }
                let obj_var = self.walk_expression(object);
                let field_var = self.cs.fresh_var();
                let _ = obj_var;
                field_var
            }

            Expression::StructLiteral { fields, .. } => {
                let result = self.cs.fresh_var();
                for (_field_name, field_expr) in fields {
                    if let Expression::Identifier { name, .. } = field_expr {
                        if let Some(&var) = self.param_vars.get(name) {
                            self.cs
                                .add(Constraint::OwnershipIs(var, OwnedType::Owned));
                        }
                    }
                    self.walk_expression(field_expr);
                }
                self.cs
                    .add(Constraint::OwnershipIs(result, OwnedType::Owned));
                result
            }

            Expression::MapLiteral { pairs, .. } => {
                let result = self.cs.fresh_var();
                for (key, val) in pairs {
                    self.walk_expression(key);
                    self.walk_expression(val);
                }
                self.cs
                    .add(Constraint::OwnershipIs(result, OwnedType::Owned));
                result
            }

            Expression::Range { start, end, .. } => {
                let s = self.walk_expression(start);
                let e = self.walk_expression(end);
                self.cs.add(Constraint::TypeEquals(s, e));
                self.cs.add(Constraint::IsNumeric(s));
                let result = self.cs.fresh_var();
                result
            }

            Expression::Closure { body, .. } => {
                let body_var = self.walk_expression(body);
                let result = self.cs.fresh_var();
                let _ = body_var;
                result
            }

            Expression::Cast { expr, type_, .. } => {
                self.walk_expression(expr);
                let result = self.cs.fresh_var();
                let base = parser_type_to_base_type(type_);
                if base != BaseType::Inferred {
                    self.cs.add(Constraint::TypeIs(result, base.clone()));
                    emit_numeric_class_constraint(&mut self.cs, result, &base);
                }
                result
            }

            Expression::Index { object, index, .. } => {
                let obj_var = self.walk_expression(object);
                let idx_var = self.walk_expression(index);
                let _ = obj_var;
                // Index is typically integer
                self.cs.add(Constraint::IsNumeric(idx_var));
                let result = self.cs.fresh_var();
                result
            }

            Expression::Tuple { elements, .. } => {
                let result = self.cs.fresh_var();
                for elem in elements {
                    if let Expression::Identifier { name, .. } = elem {
                        if let Some(&var) = self.param_vars.get(name) {
                            self.cs
                                .add(Constraint::OwnershipIs(var, OwnedType::Owned));
                        }
                    }
                    self.walk_expression(elem);
                }
                result
            }

            Expression::Array { elements, .. } => {
                let result = self.cs.fresh_var();
                let mut prev_var: Option<ConstraintVar> = None;
                for elem in elements {
                    let v = self.walk_expression(elem);
                    if let Some(pv) = prev_var {
                        self.cs.add(Constraint::TypeEquals(pv, v));
                    }
                    prev_var = Some(v);
                }
                result
            }

            Expression::MacroInvocation { args, .. } => {
                let result = self.cs.fresh_var();
                for arg in args {
                    self.walk_expression(arg);
                }
                result
            }

            Expression::TryOp { expr, .. } => {
                let inner = self.walk_expression(expr);
                let result = self.cs.fresh_var();
                let _ = inner;
                result
            }

            Expression::Await { expr, .. } => {
                let inner = self.walk_expression(expr);
                let result = self.cs.fresh_var();
                let _ = inner;
                result
            }

            Expression::AsyncCall { expr, location, .. } => {
                let inner = self.walk_expression(expr);
                self.emit_execution_call_mode(expr, ExecutionMode::Async, location);
                let result = self.cs.fresh_var();
                let _ = inner;
                result
            }

            Expression::SpawnCall { expr, location, .. } => {
                let inner = self.walk_expression(expr);
                self.emit_execution_call_mode(expr, ExecutionMode::Spawn, location);
                let result = self.cs.fresh_var();
                let _ = inner;
                result
            }

            Expression::ChannelSend {
                channel, value, ..
            } => {
                self.walk_expression(channel);
                self.walk_expression(value);
                let result = self.cs.fresh_var();
                result
            }

            Expression::ChannelRecv { channel, .. } => {
                self.walk_expression(channel);
                let result = self.cs.fresh_var();
                result
            }

            Expression::Block { statements, .. } => {
                for s in statements {
                    self.walk_statement(s);
                }
                let result = self.cs.fresh_var();
                result
            }
        }
    }

    /// Bind a pattern to a constraint var (handles identifier, tuple, etc).
    fn bind_pattern(&mut self, pattern: &Pattern<'ast>) -> ConstraintVar {
        match pattern {
            Pattern::Identifier(name) | Pattern::MutBinding(name) => self.resolve_var(name),
            Pattern::Tuple(pats) => {
                let var = self.cs.fresh_var();
                for p in pats {
                    self.bind_pattern(p);
                }
                var
            }
            Pattern::Wildcard => self.cs.fresh_var(),
            Pattern::Literal(lit) => {
                let var = self.cs.fresh_var();
                let base = literal_to_base_type(lit);
                self.cs.add(Constraint::TypeIs(var, base.clone()));
                emit_numeric_class_constraint(&mut self.cs, var, &base);
                var
            }
            Pattern::EnumVariant(_, _) => self.cs.fresh_var(),
            Pattern::Or(pats) => {
                let var = self.cs.fresh_var();
                for p in pats {
                    let pv = self.bind_pattern(p);
                    self.cs.add(Constraint::TypeEquals(var, pv));
                }
                var
            }
            Pattern::Reference(inner) => self.bind_pattern(inner),
            Pattern::Ref(name) | Pattern::RefMut(name) => self.resolve_var(name),
        }
    }

    /// Detect mutation target and emit OwnershipIs(MutRef) on the root object.
    fn detect_mutation_target(&mut self, target: &Expression<'ast>) {
        match target {
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => {
                let root_name = self.root_identifier(object);
                if let Some(name) = root_name {
                    if let Some(&var) = self.param_vars.get(&name) {
                        let r = self.fresh_region();
                        self.cs
                            .add(Constraint::OwnershipIs(var, OwnedType::MutRef(r)));
                    }
                }
            }
            Expression::Identifier { name, .. } => {
                if let Some(&var) = self.param_vars.get(name) {
                    let r = self.fresh_region();
                    self.cs
                        .add(Constraint::OwnershipIs(var, OwnedType::MutRef(r)));
                }
            }
            _ => {}
        }
    }

    /// Extract the root identifier name from a chain of field accesses / indexing.
    fn root_identifier(&self, expr: &Expression<'ast>) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => {
                self.root_identifier(object)
            }
            _ => None,
        }
    }

    /// Track variable usage for clone detection.
    fn track_owned_use(&mut self, name: &str) {
        let count = self.owned_use_counts.entry(name.to_string()).or_insert(0);
        *count += 1;
    }

    /// Emit NeedsClone constraints for variables used multiple times in owned contexts.
    /// Also incorporates analyzer's auto_clone_analysis as secondary source.
    fn emit_clone_constraints(&mut self) {
        for (var_name, count) in &self.owned_use_counts {
            if *count > 1 {
                if let Some(var) = self.param_vars.get(var_name).copied().or_else(|| {
                    self.local_vars.get(var_name).copied()
                }) {
                    self.cs.add(Constraint::NeedsClone(var));
                }
            }
        }
        // From analyzer's auto_clone_analysis (secondary, will be removed in Phase 1F)
        for ((var_name, _stmt_idx), _reason) in &self.analyzed.auto_clone_analysis.clone_sites {
            let var = self.resolve_var(var_name);
            self.cs.add(Constraint::NeedsClone(var));
        }
    }

    /// Emit mutation constraints from analyzer (secondary source).
    fn emit_mutation_constraints(&mut self) {
        for var_name in &self.analyzed.mutated_variables {
            self.resolve_var(var_name);
        }
        for param_name in &self.analyzed.mutated_parameters {
            if let Some(&var) = self.param_vars.get(param_name) {
                let r = self.fresh_region();
                self.cs
                    .add(Constraint::OwnershipIs(var, OwnedType::MutRef(r)));
            }
        }
    }

    /// Emit taint constraints for a call target.
    fn emit_taint_for_call(&mut self, callee_name: &str, fn_name: &str) {
        use crate::ir::taint::{TaintConstraint, TaintVar};

        if let Some(source_kind) = lookup_taint_source(callee_name) {
            let var_name = format!("{}::{}", fn_name, callee_name);
            self.taint_constraints.push(TaintConstraint::IsSource {
                var: TaintVar::new(&var_name),
                source_kind,
            });
        }

        if let Some(sink_desc) = lookup_taint_sink(callee_name) {
            let var_name = format!("{}::arg0", callee_name);
            self.taint_constraints.push(TaintConstraint::RequiresClean {
                var: TaintVar::new(&var_name),
                sink: format!("{} ({})", callee_name, sink_desc),
            });
        }

        if lookup_sanitizer(callee_name) {
            let input_name = format!("{}::input", callee_name);
            let output_name = format!("{}::output", callee_name);
            self.taint_constraints.push(TaintConstraint::Sanitizes {
                input: TaintVar::new(&input_name),
                output: TaintVar::new(&output_name),
                sanitizer: callee_name.to_string(),
            });
        }
    }

    /// Params whose fields are returned (partial move) stay owned at the formal.
    fn emit_return_field_consumption_constraints(&mut self) {
        for stmt in &self.analyzed.decl.body {
            let value = match stmt {
                Statement::Return { value: Some(v), .. } => v,
                _ => continue,
            };
            if let Expression::FieldAccess { object, .. } = value {
                if let Some(name) = self.root_identifier(object) {
                    if let Some(&var) = self.param_vars.get(&name) {
                        self.cs
                            .add(Constraint::OwnershipIs(var, OwnedType::Owned));
                    }
                }
            }
        }
    }

    fn finish(self) -> FunctionConstraints {
        FunctionConstraints {
            function_name: ir_function_name_from_decl(
                &self.analyzed.decl.name,
                self.analyzed.decl.parent_type.as_deref(),
            ),
            constraints: self.cs,
            var_map: ConstraintVarMap {
                params: self.param_vars,
                return_var: self.return_var,
                locals: self.local_vars,
            },
            call_targets: self.call_targets,
            taint_constraints: self.taint_constraints,
            execution_constraints: self.execution_constraints,
        }
    }
}

/// Generate constraints from an analyzed function via full AST walk.
pub fn generate_constraints(
    analyzed: &AnalyzedFunction<'_>,
    registry: Option<&SignatureRegistry>,
) -> FunctionConstraints {
    let mut walker = AstConstraintWalker::new(analyzed, registry);
    walker.walk_body();
    walker.emit_return_field_consumption_constraints();
    walker.emit_clone_constraints();
    walker.emit_mutation_constraints();
    walker.finish()
}

/// Check if a call target is a known taint source.
fn lookup_taint_source(qualified_name: &str) -> Option<crate::ir::taint::TaintSourceKind> {
    use crate::ir::taint::TaintSourceKind;
    match qualified_name {
        "http::request::body" | "std::http::request::body" | "request::body" => {
            Some(TaintSourceKind::HttpRequestBody)
        }
        "http::request::query" | "std::http::request::query" | "request::query" => {
            Some(TaintSourceKind::HttpRequestQuery)
        }
        "http::request::headers" | "std::http::request::headers" | "request::headers" => {
            Some(TaintSourceKind::HttpRequestHeader)
        }
        "std::env::get" | "std::env::var" | "env::get" | "env::var" => {
            Some(TaintSourceKind::EnvironmentVariable)
        }
        "std::io::stdin" | "io::stdin" | "std::io::read_line" | "io::read_line" => {
            Some(TaintSourceKind::UserInput)
        }
        "std::fs::read" | "std::fs::read_to_string" | "fs::read" | "fs::read_to_string" => {
            Some(TaintSourceKind::FileContents)
        }
        "std::db::query" | "db::query" | "std::db::fetch" | "db::fetch" => {
            Some(TaintSourceKind::DatabaseRow)
        }
        _ => None,
    }
}

/// Check if a call target is a dangerous sink requiring clean data.
fn lookup_taint_sink(qualified_name: &str) -> Option<&'static str> {
    match qualified_name {
        "db::query" | "std::db::query" | "db::execute" | "std::db::execute"
        | "db::raw_query" | "std::db::raw_query" => Some("SQL query"),
        "process::exec" | "std::process::exec" | "process::spawn" | "std::process::spawn"
        | "process::command" | "std::process::command" => Some("shell command"),
        "html::render" | "std::html::render" | "template::render"
        | "std::template::render" => Some("HTML template"),
        "eval" | "std::eval" => Some("code evaluation"),
        "fs::write" | "std::fs::write" => Some("file write path"),
        _ => None,
    }
}

/// Check if a call target is a known sanitizer.
fn lookup_sanitizer(qualified_name: &str) -> bool {
    matches!(
        qualified_name,
        "sql_escape" | "std::sql::escape" | "sql::escape" | "sql::parameterize"
            | "std::sql::parameterize"
            | "html_escape" | "html::escape" | "std::html::escape"
            | "shell_escape" | "shell::escape" | "std::shell::escape"
            | "url_encode" | "url::encode" | "std::url::encode"
            | "json_escape" | "json::escape" | "std::json::escape"
            | "sanitize" | "std::sanitize"
    )
}

/// Look up stdlib effects for a given qualified function name.
/// Returns the set of effects the function directly performs.
fn lookup_stdlib_effects(qualified_name: &str) -> Vec<Effect> {
    match qualified_name {
        // Filesystem
        "std::fs::read" | "std::fs::read_to_string" | "std::fs::metadata"
        | "std::fs::read_dir" | "std::fs::exists" | "fs::read" | "fs::read_to_string" => {
            vec![Effect::FsRead]
        }
        "std::fs::write" | "std::fs::create_dir" | "std::fs::create_dir_all"
        | "std::fs::remove" | "std::fs::remove_dir" | "std::fs::copy" | "std::fs::rename"
        | "fs::write" | "fs::create_dir" => vec![Effect::FsWrite],
        // Network
        "std::http::get" | "std::http::post" | "std::http::put" | "std::http::delete"
        | "std::http::request" | "http::get" | "http::post" | "http::put" | "http::delete" => {
            vec![Effect::NetEgress]
        }
        "std::http::listen" | "std::http::serve" | "http::listen" | "http::serve" => {
            vec![Effect::NetIngress]
        }
        // Process
        "std::process::spawn" | "std::process::exec" | "std::process::command"
        | "process::spawn" | "process::exec" => vec![Effect::ProcessSpawn],
        // Environment
        "std::env::get" | "std::env::var" | "env::get" | "env::var" => vec![Effect::EnvRead],
        "std::env::set" | "std::env::set_var" | "env::set" | "env::set_var" => {
            vec![Effect::EnvWrite]
        }
        // Database (implies network)
        "std::db::query" | "std::db::execute" | "db::query" | "db::execute" => {
            vec![Effect::NetEgress]
        }
        // FFI
        "std::ffi::call" | "ffi::call" => vec![Effect::Ffi],
        _ => vec![],
    }
}

/// Map a literal value to its base type.
fn literal_to_base_type(lit: &Literal) -> BaseType {
    match lit {
        Literal::Int(_) => BaseType::I32,
        Literal::IntSuffixed(_, suffix) => match suffix.as_str() {
            "i8" => BaseType::I8,
            "i16" => BaseType::I16,
            "i32" => BaseType::I32,
            "i64" => BaseType::I64,
            "i128" => BaseType::I128,
            "u8" => BaseType::U8,
            "u16" => BaseType::U16,
            "u32" => BaseType::U32,
            "u64" => BaseType::U64,
            "u128" => BaseType::U128,
            "usize" => BaseType::U64,
            "isize" => BaseType::I64,
            _ => BaseType::I32,
        },
        Literal::Float(_) => BaseType::F64,
        Literal::String(_) => BaseType::String,
        Literal::Char(_) => BaseType::Char,
        Literal::Bool(_) => BaseType::Bool,
    }
}

/// Emit numeric class constraints based on base type.
fn emit_numeric_class_constraint(cs: &mut ConstraintSet, var: ConstraintVar, base: &BaseType) {
    match base {
        BaseType::I8
        | BaseType::I16
        | BaseType::I32
        | BaseType::I64
        | BaseType::I128
        | BaseType::U8
        | BaseType::U16
        | BaseType::U32
        | BaseType::U64
        | BaseType::U128 => {
            cs.add(Constraint::IsInteger(var));
            cs.add(Constraint::IsNumeric(var));
        }
        BaseType::F32 | BaseType::F64 => {
            cs.add(Constraint::IsFloat(var));
            cs.add(Constraint::IsNumeric(var));
        }
        _ => {}
    }
}

fn is_arithmetic_op(op: &BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
    )
}

fn is_comparison_op(op: &BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
    )
}

fn is_logical_op(op: &BinaryOp) -> bool {
    matches!(op, BinaryOp::And | BinaryOp::Or)
}

fn is_bitwise_op(op: &BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::Shl
            | BinaryOp::Shr
    )
}

/// Heuristic: common mutating method names.
fn is_mutating_method(method: &str) -> bool {
    matches!(
        method,
        "push"
            | "pop"
            | "insert"
            | "remove"
            | "clear"
            | "extend"
            | "push_str"
            | "set"
            | "drain"
            | "truncate"
            | "resize"
            | "retain"
            | "sort"
            | "sort_by"
            | "sort_unstable"
            | "reverse"
            | "swap"
            | "fill"
            | "append"
    )
}

/// Generate constraints for a batch of analyzed functions.
pub fn generate_module_constraints(
    analyzed: &[AnalyzedFunction<'_>],
    registry: Option<&SignatureRegistry>,
) -> Vec<FunctionConstraints> {
    analyzed
        .iter()
        .map(|af| generate_constraints(af, registry))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::constraints::Constraint;

    fn count_constraints(fc: &FunctionConstraints, pred: impl Fn(&Constraint) -> bool) -> usize {
        fc.constraints.iter().filter(|c| pred(c)).count()
    }

    fn analyze_source(source: &str) -> Vec<crate::analyzer::AnalyzedFunction<'static>> {
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        // Leak the parser so its arena allocators keep AST nodes alive
        let parser = Box::leak(Box::new(crate::parser::Parser::new(tokens)));
        let program = parser.parse().expect("parse");
        let mut analyzer = crate::analyzer::Analyzer::new();
        let (analyzed, _, _) = analyzer.analyze_program(&program).expect("analyze");
        analyzed
    }

    #[test]
    fn test_empty_function_produces_return_var() {
        let analyzed = analyze_source("pub fn empty() {}");
        let fc = generate_constraints(&analyzed[0], None);

        assert_eq!(fc.function_name, "empty");
        assert!(fc.var_map.params.is_empty());
        let has_unit =
            count_constraints(&fc, |c| matches!(c, Constraint::TypeIs(_, BaseType::Unit)));
        assert!(has_unit >= 1, "should have TypeIs(Unit) for return");
    }

    #[test]
    fn test_typed_params_emit_type_constraints() {
        let analyzed = analyze_source("pub fn add(x: i32, y: f64) -> i32 { x }");
        let fc = generate_constraints(&analyzed[0], None);

        assert_eq!(fc.var_map.params.len(), 2);

        let has_i32 = count_constraints(&fc, |c| matches!(c, Constraint::TypeIs(_, BaseType::I32)));
        let has_f64 = count_constraints(&fc, |c| matches!(c, Constraint::TypeIs(_, BaseType::F64)));
        assert!(has_i32 >= 1, "should have TypeIs(I32) for x and return");
        assert!(has_f64 >= 1, "should have TypeIs(F64) for y");

        let has_is_integer = count_constraints(&fc, |c| matches!(c, Constraint::IsInteger(_)));
        let has_is_float = count_constraints(&fc, |c| matches!(c, Constraint::IsFloat(_)));
        assert!(has_is_integer >= 1, "should have IsInteger constraints");
        assert!(has_is_float >= 1, "should have IsFloat constraints");
    }

    #[test]
    fn test_ownership_constraints_emitted() {
        let source = r#"
struct Point { x: f64, y: f64 }
pub fn mutate(p: Point) {
    p.x = 1.0
}
"#;
        let analyzed = analyze_source(source);
        let mutate_fn = analyzed.iter().find(|f| f.decl.name == "mutate").unwrap();
        let fc = generate_constraints(mutate_fn, None);

        let has_ownership = count_constraints(&fc, |c| matches!(c, Constraint::OwnershipIs(..)));
        assert!(
            has_ownership >= 1,
            "should have OwnershipIs constraints for params + return"
        );
    }

    #[test]
    fn test_module_constraints_batch() {
        let analyzed = analyze_source(
            r#"
pub fn a() -> i32 { 1 }
pub fn b(x: string) {}
"#,
        );
        let all = generate_module_constraints(&analyzed, None);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].function_name, "a");
        assert_eq!(all[1].function_name, "b");
    }

    // ---- Phase 1A: New AST-walking tests ----

    #[test]
    fn test_literal_type_inference() {
        let analyzed = analyze_source("pub fn lit() -> i32 { 42 }");
        let fc = generate_constraints(&analyzed[0], None);

        let has_i32 = count_constraints(&fc, |c| matches!(c, Constraint::TypeIs(_, BaseType::I32)));
        assert!(
            has_i32 >= 2,
            "should have TypeIs(I32) for literal 42 and return type"
        );
    }

    #[test]
    fn test_assignment_emits_type_equals() {
        let source = r#"
pub fn assign() {
    let x: i32 = 10
    let y = x
}
"#;
        let analyzed = analyze_source(source);
        let fc = generate_constraints(&analyzed[0], None);

        let has_type_equals =
            count_constraints(&fc, |c| matches!(c, Constraint::TypeEquals(..)));
        assert!(
            has_type_equals >= 1,
            "should have TypeEquals for let bindings"
        );
    }

    #[test]
    fn test_return_emits_type_equals() {
        let source = "pub fn ret() -> i32 { return 42 }";
        let analyzed = analyze_source(source);
        let fc = generate_constraints(&analyzed[0], None);

        let has_type_equals =
            count_constraints(&fc, |c| matches!(c, Constraint::TypeEquals(..)));
        assert!(
            has_type_equals >= 1,
            "return statement should emit TypeEquals linking return var to value"
        );
    }

    #[test]
    fn test_binary_arithmetic_emits_numeric_constraints() {
        let source = "pub fn math(x: i32, y: i32) -> i32 { x + y }";
        let analyzed = analyze_source(source);
        let fc = generate_constraints(&analyzed[0], None);

        let numeric_count = count_constraints(&fc, |c| matches!(c, Constraint::IsNumeric(_)));
        assert!(
            numeric_count >= 2,
            "binary arithmetic should emit IsNumeric for both operands (got {})",
            numeric_count
        );
    }

    #[test]
    fn test_comparison_emits_bool_result() {
        let source = "pub fn cmp(x: i32, y: i32) -> bool { x > y }";
        let analyzed = analyze_source(source);
        let fc = generate_constraints(&analyzed[0], None);

        let bool_count =
            count_constraints(&fc, |c| matches!(c, Constraint::TypeIs(_, BaseType::Bool)));
        assert!(
            bool_count >= 1,
            "comparison should emit TypeIs(Bool) for the result"
        );
    }

    #[test]
    fn test_if_condition_must_be_bool() {
        let source = r#"
pub fn check(x: i32) -> i32 {
    if x > 0 {
        return 1
    }
    0
}
"#;
        let analyzed = analyze_source(source);
        let fc = generate_constraints(&analyzed[0], None);

        let bool_count =
            count_constraints(&fc, |c| matches!(c, Constraint::TypeIs(_, BaseType::Bool)));
        assert!(
            bool_count >= 1,
            "if condition should emit TypeIs(Bool)"
        );
    }

    #[test]
    fn test_field_mutation_emits_mutref() {
        let source = r#"
struct Point { x: f64, y: f64 }
pub fn set_x(p: Point) {
    p.x = 1.0
}
"#;
        let analyzed = analyze_source(source);
        let set_fn = analyzed.iter().find(|f| f.decl.name == "set_x").unwrap();
        let fc = generate_constraints(set_fn, None);

        let mut_ref_count = count_constraints(&fc, |c| {
            matches!(c, Constraint::OwnershipIs(_, OwnedType::MutRef(_)))
        });
        assert!(
            mut_ref_count >= 1,
            "field mutation should emit MutRef constraint on receiver"
        );
    }

    #[test]
    fn test_mut_self_method_on_param_emits_mutref() {
        let source = r#"
pub struct Counter {
    value: i32,
}
impl Counter {
    pub fn increment(self) {
        self.value = self.value + 1
    }
}
pub fn bump(mut c: Counter) {
    c.increment()
}
"#;
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let parser = Box::leak(Box::new(crate::parser::Parser::new(tokens)));
        let program = parser.parse().expect("parse");
        let mut analyzer = crate::analyzer::Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let bump = analyzed.iter().find(|f| f.decl.name == "bump").unwrap();
        let fc = generate_constraints(bump, Some(&registry));

        let mut_ref_count = count_constraints(&fc, |c| {
            matches!(c, Constraint::OwnershipIs(_, OwnedType::MutRef(_)))
        });
        assert!(
            mut_ref_count >= 1,
            "c.increment() (&mut self) must emit MutRef on param c via Counter::increment signature"
        );
    }

    #[test]
    fn test_string_literal_emits_string_type() {
        let source = r#"pub fn greet() -> string { "hello" }"#;
        let analyzed = analyze_source(source);
        let fc = generate_constraints(&analyzed[0], None);

        let string_count =
            count_constraints(&fc, |c| matches!(c, Constraint::TypeIs(_, BaseType::String)));
        assert!(
            string_count >= 1,
            "string literal should emit TypeIs(String)"
        );
    }

    #[test]
    fn test_while_loop_condition_bool() {
        let source = r#"
pub fn loop_fn() {
    let mut i: i32 = 0
    while i < 10 {
        i = i + 1
    }
}
"#;
        let analyzed = analyze_source(source);
        let fc = generate_constraints(&analyzed[0], None);

        let bool_count =
            count_constraints(&fc, |c| matches!(c, Constraint::TypeIs(_, BaseType::Bool)));
        assert!(
            bool_count >= 1,
            "while condition should emit TypeIs(Bool)"
        );
    }

    #[test]
    fn test_array_elements_unified() {
        let source = r#"
pub fn arr() {
    let xs = [1, 2, 3]
}
"#;
        let analyzed = analyze_source(source);
        let fc = generate_constraints(&analyzed[0], None);

        let type_equals_count =
            count_constraints(&fc, |c| matches!(c, Constraint::TypeEquals(..)));
        assert!(
            type_equals_count >= 2,
            "array elements should be unified pairwise (got {})",
            type_equals_count
        );
    }

    #[test]
    fn test_cast_emits_target_type() {
        let source = "pub fn cast_fn(x: i32) -> f64 { x as f64 }";
        let analyzed = analyze_source(source);
        let fc = generate_constraints(&analyzed[0], None);

        let f64_count =
            count_constraints(&fc, |c| matches!(c, Constraint::TypeIs(_, BaseType::F64)));
        assert!(
            f64_count >= 1,
            "cast should emit TypeIs for target type"
        );
    }

    #[test]
    fn test_locals_tracked_in_var_map() {
        let source = r#"
pub fn locals() {
    let x = 1
    let y = 2
    let z = x
}
"#;
        let analyzed = analyze_source(source);
        let fc = generate_constraints(&analyzed[0], None);

        assert!(
            !fc.var_map.locals.is_empty(),
            "locals should be tracked in the var map"
        );
    }
}
