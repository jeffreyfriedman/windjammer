// Automatic clone insertion for Windjammer ergonomics
//
// Philosophy: Users should NEVER need to write .clone() manually.
// The compiler should automatically insert clones when:
// 1. A value is moved AND used again later
// 2. A value is passed to a function that takes ownership AND used again
// 3. A value is stored in a collection AND used again
//
// This module tracks variable usage and determines where clones are needed.

use crate::parser::*;
use std::collections::{HashMap, HashSet};

/// Tracks where automatic clones should be inserted
#[derive(Debug, Clone)]
pub struct AutoCloneAnalysis {
    /// Variables that need to be cloned at specific usage sites
    /// Key: (variable_name, statement_index)
    /// Value: reason for clone
    pub clone_sites: HashMap<(String, usize), CloneReason>,
    /// Variables that are bound to string literals (don't need .clone())
    /// These are Copy types (references) so .clone() is a no-op
    pub string_literal_vars: std::collections::HashSet<String>,
    /// Names bound inside the analysed function (parameters and `let`/pattern definitions).
    /// Any other bare identifier is a module-level item (const/static/fn) rather than a
    /// local, which the usage analysis cannot see the definition of.
    pub bound_names: std::collections::HashSet<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CloneReason {
    /// Value is moved here but used again later
    MovedButUsedLater,
    /// Value is passed to function that takes ownership
    PassedToOwningFunction,
    /// Value is stored in collection
    StoredInCollection,
    /// Value is returned but also used in function
    ReturnedButUsedAgain,
}

impl Default for AutoCloneAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoCloneAnalysis {
    pub fn new() -> Self {
        AutoCloneAnalysis {
            clone_sites: HashMap::new(),
            string_literal_vars: std::collections::HashSet::new(),
            bound_names: std::collections::HashSet::new(),
        }
    }

    /// Drop clone sites recorded for module-level items mistaken for locals. A `const` is
    /// re-instantiated at every use and never moves, so a "moved then used again" clone is
    /// meaningless (and for a `&'static str` const `.clone()` would leave the value a `&str`).
    /// `is_module_item` is answered by the caller, which owns the module/library symbol table.
    pub fn forget_module_item_clone_sites(&mut self, is_module_item: impl Fn(&str) -> bool) {
        let bound = &self.bound_names;
        self.clone_sites
            .retain(|(name, _), _| bound.contains(name) || !is_module_item(name));
    }

    /// Analyze a function to determine where clones should be inserted
    pub fn analyze_function(func: &FunctionDecl) -> Self {
        Self::analyze_function_with_registry(func, None)
    }

    /// Like [`analyze_function`], but treats args to field-extract callees as borrows not moves.
    pub fn analyze_function_with_registry(
        func: &FunctionDecl,
        registry: Option<&crate::analyzer::SignatureRegistry>,
    ) -> Self {
        let mut analysis = AutoCloneAnalysis::new();

        // Track variables bound to string literals (don't need .clone())
        analysis.find_string_literal_vars(&func.body);

        // Track all variable usages
        let mut usage_map = Self::build_usage_map(&func.body, registry);

        // Register function parameters as definitions at statement_idx 0.
        // Without this, parameters are skipped by analyze_variable_usages
        // because they have no Definition usage, causing auto-clone to miss
        // parameters used multiple times (E0382).
        for param in &func.parameters {
            if param.name == "self" {
                continue;
            }
            let usages = usage_map.entry(param.name.clone()).or_default();
            let has_def = usages.iter().any(|u| u.kind == UsageKind::Definition);
            if !has_def {
                usages.insert(
                    0,
                    Usage {
                        kind: UsageKind::Definition,
                        statement_idx: 0,
                        is_move: false,
                        in_loop: false,
                        is_projection_parent: false,
                        in_exclusive_match_arm: false,
                        in_diverging_early_return: false,
                        diverging_region_end: 0,
                    },
                );
            }
        }

        analysis.bound_names = usage_map
            .iter()
            .filter(|(_, usages)| usages.iter().any(|u| u.kind == UsageKind::Definition))
            .map(|(name, _)| name.clone())
            .collect();

        // For each variable, determine if it needs clones
        for (var_name, usages) in &usage_map {
            analysis.analyze_variable_usages(var_name, usages, &func.body);
        }

        // Partial-move detection: if a field path like "s.item" is moved,
        // and the root variable "s" has later uses, the field access must
        // be cloned to avoid a partial move error (E0382).
        analysis.detect_partial_moves(&usage_map, &func.body);
        Self::detect_match_scrutinee_reuse(&func.body, &mut analysis);

        analysis
    }

    /// Build a map of all variable usages in the function.
    /// Uses a global counter so that every statement across all scopes gets a unique index.
    fn build_usage_map<'ast>(
        statements: &[&'ast Statement<'ast>],
        registry: Option<&crate::analyzer::SignatureRegistry>,
    ) -> HashMap<String, Vec<Usage>> {
        let mut map = HashMap::new();
        let mut counter: usize = 0;

        for stmt in statements.iter() {
            Self::collect_usages_from_statement(stmt, &mut counter, false, &mut map, registry);
        }

        map
    }

    /// Collect all usages of variables from a statement.
    /// `counter` is incremented for each statement to guarantee unique indices.
    fn collect_usages_from_statement(
        stmt: &Statement,
        counter: &mut usize,
        in_loop: bool,
        map: &mut HashMap<String, Vec<Usage>>,
        registry: Option<&crate::analyzer::SignatureRegistry>,
    ) {
        let idx = *counter;
        *counter += 1;

        match stmt {
            Statement::Let { pattern, value, .. } => {
                // `let copy = param` moves the param; field reads partial-move too.
                // `let x = { match ... }` / match-as-expression: nested statements must
                // consume outer counter indices so the following body statement does not
                // collide with the Match (false `same_stmt_read_after_move` → `row.clone()`).
                if let Expression::Block { statements, .. } = value {
                    for stmt in statements {
                        Self::collect_usages_from_statement(stmt, counter, in_loop, map, registry);
                    }
                } else {
                    let value_kind = match value {
                        Expression::FieldAccess { .. } | Expression::Identifier { .. } => {
                            UsageKind::Move
                        }
                        _ => UsageKind::Read,
                    };
                    Self::collect_usages_from_expression(
                        value, idx, value_kind, in_loop, map, registry,
                    );
                }

                // Tuple / struct destructure rebinds (`let (row, id) = …`) — register all
                // pattern bindings so moves after the let refer to the new definition.
                Self::register_pattern_definitions(pattern, idx, in_loop, map);
            }
            Statement::Assignment { target, value, .. } => {
                Self::collect_usages_from_expression(
                    target,
                    idx,
                    UsageKind::Write,
                    in_loop,
                    map,
                    registry,
                );
                // Owned identifiers move on assignment; loop bodies may assign the same
                // param on every iteration (E0382 without `.clone()` at the use site).
                let value_kind = match value {
                    Expression::Identifier { .. } => UsageKind::Move,
                    _ => UsageKind::Read,
                };
                Self::collect_usages_from_expression(
                    value, idx, value_kind, in_loop, map, registry,
                );
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                Self::collect_usages_from_expression(
                    expr,
                    idx,
                    UsageKind::Move,
                    in_loop,
                    map,
                    registry,
                );
            }
            Statement::Expression { expr, .. } => {
                // A bare FieldAccess or Identifier in expression-statement position is a
                // value expression (e.g. last expression in an if/else branch). This moves
                // the value, so mark it Move so auto-clone detects loop-captured fields.
                let kind = match expr {
                    Expression::FieldAccess { .. } | Expression::Identifier { .. } => {
                        UsageKind::Move
                    }
                    _ => UsageKind::Read,
                };
                Self::collect_usages_from_expression(expr, idx, kind, in_loop, map, registry);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                Self::collect_usages_from_expression(
                    condition,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
                // a then/else that always `return`s is exclusive vs. statements
                // after the `if`. Do not treat those sibling-path uses as "later".
                let then_lens: HashMap<String, usize> =
                    map.iter().map(|(k, v)| (k.clone(), v.len())).collect();
                for stmt in then_block.iter() {
                    Self::collect_usages_from_statement(stmt, counter, in_loop, map, registry);
                }
                if Self::block_unconditionally_returns(then_block) {
                    let region_end = counter.saturating_sub(1);
                    Self::mark_diverging_early_return_usages(map, &then_lens, region_end);
                }
                if let Some(else_b) = else_block {
                    let else_lens: HashMap<String, usize> =
                        map.iter().map(|(k, v)| (k.clone(), v.len())).collect();
                    for stmt in else_b.iter() {
                        Self::collect_usages_from_statement(stmt, counter, in_loop, map, registry);
                    }
                    if Self::block_unconditionally_returns(else_b) {
                        let region_end = counter.saturating_sub(1);
                        Self::mark_diverging_early_return_usages(map, &else_lens, region_end);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                Self::collect_usages_from_expression(
                    condition,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
                for stmt in body.iter() {
                    Self::collect_usages_from_statement(stmt, counter, true, map, registry);
                }
            }
            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                Self::collect_usages_from_expression(
                    iterable,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
                Self::register_pattern_definitions(pattern, idx, true, map);
                for stmt in body.iter() {
                    Self::collect_usages_from_statement(stmt, counter, true, map, registry);
                }
            }
            Statement::Loop { body, .. } => {
                for stmt in body.iter() {
                    Self::collect_usages_from_statement(stmt, counter, true, map, registry);
                }
            }
            Statement::Match { value, arms, .. } => {
                Self::collect_usages_from_expression(
                    value,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
                for arm in arms {
                    // P3.574 / wj-cron: `Ok(expr) => { matches_cron(expr); matches_cron(expr) }`
                    // must see `expr` as a defined binding so sequential moves get `.clone()`.
                    Self::register_pattern_definitions(&arm.pattern, idx, in_loop, map);
                    // Process arm body blocks using the parent counter (like
                    // Statement::If does for then_block/else_block) so that
                    // statement indices stay synchronized with the codegen's
                    // auto_clone_counter which is global.
                    if let Expression::Block { statements, .. } = arm.body {
                        // `if let` lowers to Match. A block arm that always
                        // returns is exclusive vs. statements after the match
                        // (`return substitute(text, w)` then `return text`).
                        let lens_before: HashMap<String, usize> =
                            map.iter().map(|(k, v)| (k.clone(), v.len())).collect();
                        for stmt in statements {
                            Self::collect_usages_from_statement(
                                stmt, counter, in_loop, map, registry,
                            );
                        }
                        if Self::block_unconditionally_returns(statements) {
                            let region_end = counter.saturating_sub(1);
                            Self::mark_diverging_early_return_usages(map, &lens_before, region_end);
                        }
                    } else {
                        let lens_before: HashMap<String, usize> =
                            map.iter().map(|(k, v)| (k.clone(), v.len())).collect();
                        Self::collect_usages_from_expression(
                            arm.body,
                            idx,
                            UsageKind::Read,
                            in_loop,
                            map,
                            registry,
                        );
                        // P3.332: Ok/Err arms are exclusive — do not treat multi-arm
                        // moves of the same binding as same-stmt multi-move.
                        for (key, usages) in map.iter_mut() {
                            let start = lens_before.get(key).copied().unwrap_or(0);
                            for u in usages.iter_mut().skip(start) {
                                u.in_exclusive_match_arm = true;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn block_unconditionally_returns(body: &[&Statement]) -> bool {
        body.last()
            .is_some_and(|s| matches!(s, Statement::Return { .. }))
    }

    fn mark_diverging_early_return_usages(
        map: &mut HashMap<String, Vec<Usage>>,
        lens_before: &HashMap<String, usize>,
        region_end: usize,
    ) {
        for (key, usages) in map.iter_mut() {
            let start = lens_before.get(key).copied().unwrap_or(0);
            for u in usages.iter_mut().skip(start) {
                // An inner `if let` that returns is a tighter exclusive region
                // than the enclosing arm that also returns
                // (`return substitute(text, w)` then `return text`).
                if u.in_diverging_early_return {
                    u.diverging_region_end = u.diverging_region_end.min(region_end);
                } else {
                    u.in_diverging_early_return = true;
                    u.diverging_region_end = region_end;
                }
            }
        }
    }

    /// Extract a path string from an expression (e.g., "config.paths", "obj.method()", "items[0]")
    fn extract_expression_path(expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, field, .. } => {
                // Recursively build the path: object.field
                Self::extract_expression_path(object)
                    .map(|base_path| format!("{}.{}", base_path, field))
            }
            Expression::MethodCall { object, method, .. } => {
                // Build path for method calls: object.method()
                Self::extract_expression_path(object)
                    .map(|base_path| format!("{}.{}()", base_path, method))
            }
            Expression::Index { object, index, .. } => {
                // Build path for index expressions: object[index]
                // For simplicity, we use [*] as a placeholder since the actual index
                // might vary (e.g., items[0], items[i])
                if let Some(base_path) = Self::extract_expression_path(object) {
                    // Try to get a more specific index if it's a literal
                    let index_str = match index {
                        Expression::Literal {
                            value: crate::parser::Literal::Int(n),
                            ..
                        } => n.to_string(),
                        Expression::Identifier { name, .. } => name.clone(),
                        _ => "*".to_string(), // Generic placeholder
                    };
                    Some(format!("{}[{}]", base_path, index_str))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn callee_name_from_call_function(function: &Expression) -> Option<String> {
        match function {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, field, .. } => {
                Self::callee_name_from_call_function(object).map(|base| format!("{base}::{field}"))
            }
            _ => None,
        }
    }

    fn callee_arg_field_extracts(
        function: &Expression,
        arg_index: usize,
        registry: &crate::analyzer::SignatureRegistry,
    ) -> bool {
        let Some(callee_name) = Self::callee_name_from_call_function(function) else {
            return false;
        };
        let simple = callee_name.rsplit("::").next().unwrap_or(&callee_name);
        let Some(sig) = registry
            .get_signature(&callee_name)
            .or_else(|| registry.lookup_method(&callee_name))
            .or_else(|| registry.find_signature_ending_with(simple))
        else {
            return false;
        };
        let param_idx = sig.arg_param_index(arg_index);
        Self::sig_arg_is_field_extract_shared_borrow(sig, param_idx)
    }

    /// Field-extract demotes Move→Read only for shared-ref formals. Owned WJ
    /// formals that match/project (`value_tag(value: Value)`) still move — callers
    /// must `.clone()` on reuse (regression-063).
    fn sig_arg_is_field_extract_shared_borrow(
        sig: &crate::analyzer::FunctionSignature,
        param_idx: usize,
    ) -> bool {
        let field_extract = sig
            .field_extract_params
            .as_ref()
            .and_then(|flags| flags.get(param_idx))
            .copied()
            .unwrap_or(false);
        if !field_extract {
            return false;
        }
        match sig
            .emitted_rust_ref_params
            .as_ref()
            .and_then(|flags| flags.get(param_idx))
            .copied()
        {
            Some(true) => return true,
            Some(false) => return false,
            None => {}
        }
        // Bare WJ source formals (`value: Value`) still move — even when analyzer marks
        // Borrowed from match/field reads. Only explicit `&T` formals field-extract as Read.
        if !sig.formal_param_types.is_empty() {
            sig.formal_param_types.get(param_idx).is_some_and(|t| {
                matches!(
                    t,
                    crate::parser::Type::Reference(_) | crate::parser::Type::MutableReference(_)
                )
            })
        } else {
            false
        }
    }

    /// True when a method formal at `param_idx` is emitted / declared as shared borrow.
    fn sig_arg_is_shared_borrow_formal(
        sig: &crate::analyzer::FunctionSignature,
        param_idx: usize,
    ) -> bool {
        if Self::sig_arg_is_field_extract_shared_borrow(sig, param_idx) {
            return true;
        }
        match sig
            .emitted_rust_ref_params
            .as_ref()
            .and_then(|flags| flags.get(param_idx))
            .copied()
        {
            Some(true) => return true,
            Some(false) => return false,
            None => {}
        }
        if !sig.formal_param_types.is_empty() {
            if sig.formal_param_types.get(param_idx).is_some_and(|t| {
                matches!(
                    t,
                    crate::parser::Type::Reference(_) | crate::parser::Type::MutableReference(_)
                )
            }) {
                return true;
            }
            // Bare WJ owned formal (`txn: Txn`) — move even if analyzer marked Borrowed.
            if sig.formal_param_types.get(param_idx).is_some() {
                return false;
            }
        }
        matches!(
            sig.param_ownership.get(param_idx),
            Some(
                crate::analyzer::OwnershipMode::Borrowed
                    | crate::analyzer::OwnershipMode::MutBorrowed
            )
        )
    }

    /// Method-call argument usage: prefer Move unless every matching signature agrees
    /// the formal is a shared borrow. Homonyms like `HashMap::get` (borrowed key) and
    /// `StorageEngine::get` (owned `Txn`) must not let the map-key name win — missing
    /// `.clone()` on owned reuse is E0382.
    fn method_call_arg_usage_kind(
        method: &str,
        arg_index: usize,
        arg_count: usize,
        registry: Option<&crate::analyzer::SignatureRegistry>,
    ) -> UsageKind {
        let Some(registry) = registry else {
            return UsageKind::Move;
        };

        let mut saw_candidate = false;
        let mut any_owned_move = false;
        let mut all_shared_borrow = true;

        for (_key, sig) in registry.signatures_for_method_name(method) {
            let user_args = if sig.has_self_receiver {
                sig.param_ownership.len().saturating_sub(1)
            } else {
                sig.param_ownership.len()
            };
            if user_args != arg_count {
                continue;
            }
            saw_candidate = true;
            let pidx = sig.arg_param_index(arg_index);
            if Self::sig_arg_is_shared_borrow_formal(sig, pidx) {
                continue;
            }
            // Bare owned WJ formals on *user* methods (`clips: Vec<Clip>`) must Move
            // even when analyzer Borrowed + `runtime_wj_owned_rust_borrowed` would
            // auto-borrow (P3.647). Runtime-std callees (`json::is_array`) still use
            // the borrow baseline so multipass records Read not `&v.clone()` (P3.668).
            let bare_owned_ast = sig
                .formal_param_type(pidx)
                .or_else(|| sig.formal_param_types.get(pidx))
                .is_some_and(|t| {
                    !matches!(
                        t,
                        crate::parser::Type::Reference(_)
                            | crate::parser::Type::MutableReference(_)
                    )
                });
            let runtime_std_callee = Self::callee_is_runtime_std_baseline(&sig.name);
            if !(bare_owned_ast && !runtime_std_callee)
                && crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                    registry,
                    &sig.name,
                    Some(sig),
                    arg_index,
                )
            {
                continue;
            }
            any_owned_move = true;
            all_shared_borrow = false;
        }

        if any_owned_move {
            return UsageKind::Move;
        }
        if saw_candidate && all_shared_borrow {
            return UsageKind::Read;
        }
        // No conclusive signature — default Move (safe for owned Custom reuse).
        UsageKind::Move
    }

    fn free_call_arg_usage_kind(
        function: &Expression,
        arg_index: usize,
        arg_count: usize,
        registry: Option<&crate::analyzer::SignatureRegistry>,
    ) -> UsageKind {
        let Some(registry) = registry else {
            return UsageKind::Move;
        };
        let Some(callee_name) = Self::callee_name_from_call_function(function) else {
            return UsageKind::Move;
        };
        Self::free_call_arg_usage_kind_by_name(&callee_name, arg_index, arg_count, registry)
    }

    fn free_call_arg_usage_kind_by_name(
        callee_name: &str,
        arg_index: usize,
        arg_count: usize,
        registry: &crate::analyzer::SignatureRegistry,
    ) -> UsageKind {
        let simple = callee_name.rsplit("::").next().unwrap_or(callee_name);
        let Some(sig) = registry
            .get_signature(callee_name)
            .or_else(|| registry.lookup_method(callee_name))
            .or_else(|| registry.find_signature_ending_with(simple))
        else {
            return UsageKind::Move;
        };
        let user_args = if sig.has_self_receiver {
            sig.param_ownership.len().saturating_sub(1)
        } else {
            sig.param_ownership.len()
        };
        if arg_count > 0 && user_args != arg_count {
            return UsageKind::Move;
        }
        let pidx = sig.arg_param_index(arg_index);
        if Self::sig_arg_is_shared_borrow_formal(sig, pidx) {
            return UsageKind::Read;
        }
        // Same P3.647 / P3.668 policy as method_call_arg_usage_kind.
        // `self.evaluate_node(...)` is often Call(FieldAccess(self, …)) here.
        let bare_owned_ast = sig
            .formal_param_type(pidx)
            .or_else(|| sig.formal_param_types.get(pidx))
            .is_some_and(|t| {
                !matches!(
                    t,
                    crate::parser::Type::Reference(_) | crate::parser::Type::MutableReference(_)
                )
            });
        let runtime_std_callee = Self::callee_is_runtime_std_baseline(callee_name)
            || Self::callee_is_runtime_std_baseline(&sig.name);
        if !(bare_owned_ast && !runtime_std_callee)
            && crate::codegen::rust::stdlib_method_traits::runtime_std_param_needs_auto_borrow_resolved(
                registry,
                callee_name,
                Some(sig),
                arg_index,
            )
        {
            UsageKind::Read
        } else {
            UsageKind::Move
        }
    }

    fn callee_is_runtime_std_baseline(callee_name: &str) -> bool {
        if crate::codegen::rust::stdlib_method_traits::callee_path_is_runtime_std(callee_name) {
            return true;
        }
        let stdlib = crate::analyzer::SignatureRegistry::stdlib();
        stdlib.get_signature(callee_name).is_some()
            || stdlib.get_fallback_signature(callee_name).is_some()
    }

    fn match_arm_reuses_binding(arm_body: &Expression, name: &str) -> bool {
        match arm_body {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| Self::statement_uses_binding(s, name)),
            other => Self::expression_uses_binding(other, name),
        }
    }

    fn statement_uses_binding(stmt: &Statement, name: &str) -> bool {
        match stmt {
            Statement::Return { value: Some(e), .. }
            | Statement::Expression { expr: e, .. }
            | Statement::Const { value: e, .. }
            | Statement::Static { value: e, .. } => Self::expression_uses_binding(e, name),
            Statement::Let {
                value, else_block, ..
            } => {
                Self::expression_uses_binding(value, name)
                    || else_block.as_ref().is_some_and(|stmts| {
                        stmts.iter().any(|s| Self::statement_uses_binding(s, name))
                    })
            }
            Statement::Assignment { target, value, .. } => {
                Self::expression_uses_binding(target, name)
                    || Self::expression_uses_binding(value, name)
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                Self::expression_uses_binding(condition, name)
                    || then_block
                        .iter()
                        .any(|s| Self::statement_uses_binding(s, name))
                    || else_block.as_ref().is_some_and(|stmts| {
                        stmts.iter().any(|s| Self::statement_uses_binding(s, name))
                    })
            }
            // P3.642: nested match in Ok arm reuses scrutinee call args
            // (`match decode_store(snapshot)` under `match todos_to_json`).
            Statement::Match { value, arms, .. } => {
                Self::expression_uses_binding(value, name)
                    || arms.iter().any(|arm| {
                        arm.guard
                            .is_some_and(|g| Self::expression_uses_binding(g, name))
                            || Self::match_arm_reuses_binding(arm.body, name)
                    })
            }
            Statement::For { iterable, body, .. }
            | Statement::While {
                condition: iterable,
                body,
                ..
            } => {
                Self::expression_uses_binding(iterable, name)
                    || body.iter().any(|s| Self::statement_uses_binding(s, name))
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => {
                body.iter().any(|s| Self::statement_uses_binding(s, name))
            }
            Statement::Defer { statement, .. } => Self::statement_uses_binding(statement, name),
            _ => false,
        }
    }

    fn expression_uses_binding(expr: &Expression, name: &str) -> bool {
        match expr {
            Expression::Identifier { name: id, .. } => id == name,
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| Self::statement_uses_binding(s, name)),
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                Self::expression_uses_binding(function, name)
                    || arguments
                        .iter()
                        .any(|(_, a)| Self::expression_uses_binding(a, name))
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                Self::expression_uses_binding(object, name)
                    || arguments
                        .iter()
                        .any(|(_, a)| Self::expression_uses_binding(a, name))
            }
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements
                .iter()
                .any(|e| Self::expression_uses_binding(e, name)),
            Expression::Binary { left, right, .. }
            | Expression::Range {
                start: left,
                end: right,
                ..
            } => {
                Self::expression_uses_binding(left, name)
                    || Self::expression_uses_binding(right, name)
            }
            Expression::Unary { operand, .. } => Self::expression_uses_binding(operand, name),
            Expression::FieldAccess { object, .. } => Self::expression_uses_binding(object, name),
            Expression::Index { object, index, .. } => {
                Self::expression_uses_binding(object, name)
                    || Self::expression_uses_binding(index, name)
            }
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, e)| Self::expression_uses_binding(e, name)),
            Expression::MapLiteral { pairs, .. } => pairs.iter().any(|(k, v)| {
                Self::expression_uses_binding(k, name) || Self::expression_uses_binding(v, name)
            }),
            Expression::Closure { body, .. } => Self::expression_uses_binding(body, name),
            Expression::Cast { expr, .. }
            | Expression::TryOp { expr, .. }
            | Expression::Await { expr, .. }
            | Expression::AsyncCall { expr, .. }
            | Expression::SpawnCall { expr, .. }
            | Expression::ChannelRecv { channel: expr, .. } => {
                Self::expression_uses_binding(expr, name)
            }
            Expression::ChannelSend { channel, value, .. } => {
                Self::expression_uses_binding(channel, name)
                    || Self::expression_uses_binding(value, name)
            }
            Expression::MacroInvocation { args, .. } => {
                args.iter().any(|e| Self::expression_uses_binding(e, name))
            }
            Expression::Literal { .. } => false,
        }
    }

    fn detect_match_scrutinee_reuse(statements: &[&Statement], analysis: &mut AutoCloneAnalysis) {
        let mut counter: usize = 0;
        for stmt in statements {
            Self::detect_match_scrutinee_reuse_stmt(stmt, &mut counter, analysis);
        }
    }

    /// P3.642: also walk Matches nested under `If` / arm blocks (todo-cli export/stats).
    fn detect_match_scrutinee_reuse_stmt(
        stmt: &Statement,
        counter: &mut usize,
        analysis: &mut AutoCloneAnalysis,
    ) {
        let idx = *counter;
        *counter += 1;
        if let Statement::Match { value, arms, .. } = stmt {
            Self::record_match_scrutinee_arg_clones(value, arms, idx, analysis);
            for arm in arms {
                if let Expression::Block { statements, .. } = arm.body {
                    for s in statements {
                        Self::detect_match_scrutinee_reuse_stmt(s, counter, analysis);
                    }
                }
            }
            return;
        }
        match stmt {
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                for s in then_block {
                    Self::detect_match_scrutinee_reuse_stmt(s, counter, analysis);
                }
                if let Some(e) = else_block {
                    for s in e {
                        Self::detect_match_scrutinee_reuse_stmt(s, counter, analysis);
                    }
                }
            }
            Statement::While { body, .. }
            | Statement::For { body, .. }
            | Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => {
                for s in body {
                    Self::detect_match_scrutinee_reuse_stmt(s, counter, analysis);
                }
            }
            Statement::Let {
                value, else_block, ..
            } => {
                if let Expression::Block { statements, .. } = value {
                    for s in statements {
                        Self::detect_match_scrutinee_reuse_stmt(s, counter, analysis);
                    }
                }
                if let Some(e) = else_block {
                    for s in e {
                        Self::detect_match_scrutinee_reuse_stmt(s, counter, analysis);
                    }
                }
            }
            Statement::Defer { statement, .. } => {
                Self::detect_match_scrutinee_reuse_stmt(statement, counter, analysis);
            }
            _ => {}
        }
    }

    fn record_match_scrutinee_arg_clones(
        value: &Expression,
        arms: &[crate::parser::MatchArm],
        idx: usize,
        analysis: &mut AutoCloneAnalysis,
    ) {
        if let Expression::Identifier { name, .. } = value {
            let reused = arms
                .iter()
                .any(|arm| Self::match_arm_reuses_binding(arm.body, name));
            if reused {
                analysis
                    .clone_sites
                    .insert((name.clone(), idx), CloneReason::MovedButUsedLater);
            }
        } else if let Expression::Call { arguments, .. } = value {
            for (_label, arg) in arguments {
                if let Expression::Identifier { name, .. } = arg {
                    let reused = arms
                        .iter()
                        .any(|arm| Self::match_arm_reuses_binding(arm.body, name));
                    if reused {
                        analysis
                            .clone_sites
                            .insert((name.clone(), idx), CloneReason::MovedButUsedLater);
                    }
                }
            }
        } else if let Expression::MethodCall { arguments, .. } = value {
            for (_label, arg) in arguments {
                if let Expression::Identifier { name, .. } = arg {
                    let reused = arms
                        .iter()
                        .any(|arm| Self::match_arm_reuses_binding(arm.body, name));
                    if reused {
                        analysis
                            .clone_sites
                            .insert((name.clone(), idx), CloneReason::MovedButUsedLater);
                    }
                }
            }
        }
    }

    /// Record that `expr` is only touched as the parent of a field projection
    /// (`buf` inside `buf.scores`). Nested chains mark each identifier root once.
    fn collect_field_projection_parent_usages(
        expr: &Expression,
        idx: usize,
        in_loop: bool,
        map: &mut HashMap<String, Vec<Usage>>,
        registry: Option<&crate::analyzer::SignatureRegistry>,
    ) {
        match expr {
            Expression::Identifier { name, .. } => {
                map.entry(name.clone()).or_default().push(Usage {
                    statement_idx: idx,
                    kind: UsageKind::Read,
                    is_move: false,
                    in_loop,
                    is_projection_parent: true,
                    in_exclusive_match_arm: false,
                    in_diverging_early_return: false,
                    diverging_region_end: 0,
                });
            }
            Expression::FieldAccess { object, .. } => {
                if let Some(path) = Self::extract_expression_path(expr) {
                    map.entry(path).or_default().push(Usage {
                        statement_idx: idx,
                        kind: UsageKind::Read,
                        is_move: false,
                        in_loop,
                        is_projection_parent: true,
                        in_exclusive_match_arm: false,
                        in_diverging_early_return: false,
                        diverging_region_end: 0,
                    });
                }
                Self::collect_field_projection_parent_usages(object, idx, in_loop, map, registry);
            }
            _ => {
                Self::collect_usages_from_expression(
                    expr,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
            }
        }
    }

    /// `vec![a, b]` moves its elements into the new collection, whatever the consumer of the
    /// collection does; `format!` / `println!` and friends only read their arguments.
    fn collect_macro_arg_usages(
        name: &str,
        args: &[&Expression],
        idx: usize,
        in_loop: bool,
        map: &mut HashMap<String, Vec<Usage>>,
        registry: Option<&crate::analyzer::SignatureRegistry>,
    ) {
        let kind = if name == "vec" {
            UsageKind::Move
        } else {
            UsageKind::Read
        };
        for arg in args {
            Self::collect_usages_from_expression(arg, idx, kind, in_loop, map, registry);
        }
    }

    /// Collect usages from an expression
    fn collect_usages_from_expression(
        expr: &Expression,
        idx: usize,
        kind: UsageKind,
        in_loop: bool,
        map: &mut HashMap<String, Vec<Usage>>,
        registry: Option<&crate::analyzer::SignatureRegistry>,
    ) {
        match expr {
            Expression::Identifier { name, .. } => {
                map.entry(name.clone()).or_default().push(Usage {
                    statement_idx: idx,
                    kind,
                    is_move: kind == UsageKind::Move,
                    in_loop,
                    is_projection_parent: false,
                    in_exclusive_match_arm: false,
                    in_diverging_early_return: false,
                    diverging_region_end: 0,
                });
            }
            Expression::FieldAccess { object, .. } => {
                if let Some(path) = Self::extract_expression_path(expr) {
                    map.entry(path).or_default().push(Usage {
                        statement_idx: idx,
                        kind,
                        is_move: kind == UsageKind::Move,
                        in_loop,
                        is_projection_parent: false,
                        in_exclusive_match_arm: false,
                        in_diverging_early_return: false,
                        diverging_region_end: 0,
                    });
                }
                // Parent binding uses from `root.field` are not whole-root reuse.
                Self::collect_field_projection_parent_usages(object, idx, in_loop, map, registry);
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // Parser often emits `Call(FieldAccess)` for `recv.method(args)` —
                // treat like MethodCall so `&self` receivers stay Reads (not moves via
                // FieldAccess path bookkeeping alone).
                if let Expression::FieldAccess {
                    object,
                    field: method,
                    ..
                } = function
                {
                    if let Some(path) = Self::extract_expression_path(expr) {
                        map.entry(path).or_default().push(Usage {
                            statement_idx: idx,
                            kind,
                            is_move: kind == UsageKind::Move,
                            in_loop,
                            is_projection_parent: false,
                            in_exclusive_match_arm: false,
                            in_diverging_early_return: false,
                            diverging_region_end: 0,
                        });
                    }
                    Self::collect_usages_from_expression(
                        object,
                        idx,
                        UsageKind::Read,
                        in_loop,
                        map,
                        registry,
                    );
                    for (i, (_label, arg_expr)) in arguments.iter().enumerate() {
                        // Prefer `json::is_array` over bare method-name homonyms so
                        // runtime-std `&Value` wins over WJ owned stubs (json multipass).
                        let arg_kind =
                            if let (Some(registry), Expression::Identifier { name: module, .. }) =
                                (registry, &**object)
                            {
                                let qualified = format!("{module}::{method}");
                                Self::free_call_arg_usage_kind_by_name(
                                    &qualified,
                                    i,
                                    arguments.len(),
                                    registry,
                                )
                            } else {
                                Self::method_call_arg_usage_kind(
                                    method,
                                    i,
                                    arguments.len(),
                                    registry,
                                )
                            };
                        Self::collect_usages_from_expression(
                            arg_expr, idx, arg_kind, in_loop, map, registry,
                        );
                    }
                } else {
                    Self::collect_usages_from_expression(
                        function,
                        idx,
                        UsageKind::Read,
                        in_loop,
                        map,
                        registry,
                    );
                    for (i, (_label, arg_expr)) in arguments.iter().enumerate() {
                        let arg_kind = if registry
                            .is_some_and(|r| Self::callee_arg_field_extracts(function, i, r))
                        {
                            UsageKind::Read
                        } else {
                            Self::free_call_arg_usage_kind(function, i, arguments.len(), registry)
                        };
                        Self::collect_usages_from_expression(
                            arg_expr, idx, arg_kind, in_loop, map, registry,
                        );
                    }
                }
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                // Explicit `.clone()` masks bare binding moves in reuse analysis;
                // still register a move so preserve_for_reuse / auto-clone fire.
                if crate::type_classification::is_language_level_explicit_clone(method)
                    && arguments.is_empty()
                    && matches!(&**object, Expression::Identifier { .. })
                {
                    Self::collect_usages_from_expression(
                        object,
                        idx,
                        UsageKind::Move,
                        in_loop,
                        map,
                        registry,
                    );
                    return;
                }
                if let Some(path) = Self::extract_expression_path(expr) {
                    map.entry(path).or_default().push(Usage {
                        statement_idx: idx,
                        kind,
                        is_move: kind == UsageKind::Move,
                        in_loop,
                        is_projection_parent: false,
                        in_exclusive_match_arm: false,
                        in_diverging_early_return: false,
                        diverging_region_end: 0,
                    });
                }
                Self::collect_usages_from_expression(
                    object,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
                for (i, (_label, arg_expr)) in arguments.iter().enumerate() {
                    // Signature-driven: owned formals move (need `.clone()` on reuse).
                    // Module-qualified `json.is_array(v)` must use `json::is_array` so
                    // runtime-std `&Value` beats WJ owned stubs (multipass clone bug).
                    let arg_kind =
                        if let (Some(registry), Expression::Identifier { name: module, .. }) =
                            (registry, &**object)
                        {
                            let qualified = format!("{module}::{method}");
                            Self::free_call_arg_usage_kind_by_name(
                                &qualified,
                                i,
                                arguments.len(),
                                registry,
                            )
                        } else {
                            Self::method_call_arg_usage_kind(method, i, arguments.len(), registry)
                        };
                    Self::collect_usages_from_expression(
                        arg_expr, idx, arg_kind, in_loop, map, registry,
                    );
                }
            }
            Expression::Binary { left, right, .. } => {
                Self::collect_usages_from_expression(
                    left,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
                Self::collect_usages_from_expression(
                    right,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
            }
            Expression::Unary { operand, .. } => {
                Self::collect_usages_from_expression(
                    operand,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
            }
            Expression::Index { object, index, .. } => {
                if let Some(path) = Self::extract_expression_path(expr) {
                    map.entry(path).or_default().push(Usage {
                        statement_idx: idx,
                        kind,
                        is_move: kind == UsageKind::Move,
                        in_loop,
                        is_projection_parent: false,
                        in_exclusive_match_arm: false,
                        in_diverging_early_return: false,
                        diverging_region_end: 0,
                    });
                }
                Self::collect_usages_from_expression(
                    object,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
                Self::collect_usages_from_expression(
                    index,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
            }
            Expression::Tuple { elements, .. } => {
                for elem in elements {
                    let elem_kind = match elem {
                        Expression::Identifier { .. } | Expression::FieldAccess { .. } => {
                            UsageKind::Move
                        }
                        _ => UsageKind::Read,
                    };
                    Self::collect_usages_from_expression(
                        elem, idx, elem_kind, in_loop, map, registry,
                    );
                }
            }
            Expression::Array { elements, .. } => {
                for elem in elements {
                    Self::collect_usages_from_expression(
                        elem,
                        idx,
                        UsageKind::Move,
                        in_loop,
                        map,
                        registry,
                    );
                }
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, field_expr) in fields {
                    Self::collect_usages_from_expression(
                        field_expr,
                        idx,
                        UsageKind::Move,
                        in_loop,
                        map,
                        registry,
                    );
                }
            }
            Expression::Block { statements, .. } => {
                let mut block_counter = idx + 1;
                for stmt in statements {
                    Self::collect_usages_from_statement(
                        stmt,
                        &mut block_counter,
                        in_loop,
                        map,
                        registry,
                    );
                }
            }
            Expression::Cast { expr, .. } => {
                Self::collect_usages_from_expression(
                    expr,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
            }
            Expression::Range { start, end, .. } => {
                Self::collect_usages_from_expression(
                    start,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
                Self::collect_usages_from_expression(
                    end,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
            }
            Expression::TryOp { expr, .. } => {
                Self::collect_usages_from_expression(
                    expr,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
            }
            Expression::Await { expr, .. } => {
                Self::collect_usages_from_expression(
                    expr,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
            }
            Expression::ChannelSend { channel, value, .. } => {
                Self::collect_usages_from_expression(
                    channel,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
                Self::collect_usages_from_expression(
                    value,
                    idx,
                    UsageKind::Move,
                    in_loop,
                    map,
                    registry,
                );
            }
            Expression::ChannelRecv { channel, .. } => {
                Self::collect_usages_from_expression(
                    channel,
                    idx,
                    UsageKind::Read,
                    in_loop,
                    map,
                    registry,
                );
            }
            Expression::MacroInvocation { name, args, .. } => {
                Self::collect_macro_arg_usages(name, args, idx, in_loop, map, registry)
            }
            Expression::MapLiteral { pairs, .. } => {
                for (key, value) in pairs {
                    Self::collect_usages_from_expression(
                        key,
                        idx,
                        UsageKind::Move,
                        in_loop,
                        map,
                        registry,
                    );
                    Self::collect_usages_from_expression(
                        value,
                        idx,
                        UsageKind::Move,
                        in_loop,
                        map,
                        registry,
                    );
                }
            }
            _ => {}
        }
    }

    /// Register pattern bindings as definitions (for for-loop variables, match bindings, etc.)
    fn register_pattern_definitions(
        pattern: &crate::parser::Pattern,
        statement_idx: usize,
        in_loop: bool,
        map: &mut HashMap<String, Vec<Usage>>,
    ) {
        match pattern {
            crate::parser::Pattern::Identifier(name)
            | crate::parser::Pattern::MutBinding(name)
            | crate::parser::Pattern::Ref(name)
            | crate::parser::Pattern::RefMut(name) => {
                map.entry(name.clone()).or_default().push(Usage {
                    statement_idx,
                    kind: UsageKind::Definition,
                    is_move: false,
                    in_loop,
                    is_projection_parent: false,
                    in_exclusive_match_arm: false,
                    in_diverging_early_return: false,
                    diverging_region_end: 0,
                });
            }
            crate::parser::Pattern::Tuple(patterns) => {
                for p in patterns {
                    Self::register_pattern_definitions(p, statement_idx, in_loop, map);
                }
            }
            crate::parser::Pattern::Or(patterns) => {
                for p in patterns {
                    Self::register_pattern_definitions(p, statement_idx, in_loop, map);
                }
            }
            crate::parser::Pattern::Reference(inner) => {
                Self::register_pattern_definitions(inner, statement_idx, in_loop, map);
            }
            crate::parser::Pattern::EnumVariant(_, binding) => match binding {
                crate::parser::EnumPatternBinding::Single(name) => {
                    map.entry(name.clone()).or_default().push(Usage {
                        statement_idx,
                        kind: UsageKind::Definition,
                        is_move: false,
                        in_loop,
                        is_projection_parent: false,
                        in_exclusive_match_arm: false,
                        in_diverging_early_return: false,
                        diverging_region_end: 0,
                    });
                }
                crate::parser::EnumPatternBinding::Tuple(patterns) => {
                    for p in patterns {
                        Self::register_pattern_definitions(p, statement_idx, in_loop, map);
                    }
                }
                crate::parser::EnumPatternBinding::Struct(fields, _) => {
                    for (_, p) in fields {
                        Self::register_pattern_definitions(p, statement_idx, in_loop, map);
                    }
                }
                crate::parser::EnumPatternBinding::None
                | crate::parser::EnumPatternBinding::Wildcard => {}
            },
            crate::parser::Pattern::Wildcard | crate::parser::Pattern::Literal(_) | crate::parser::Pattern::Range { .. } => {}
        }
    }

    /// Analyze usages of a single variable to determine where clones are needed.
    /// `statements` is the function body (for writeback detection: `map = f(map, …)`).
    fn analyze_variable_usages(
        &mut self,
        var_name: &str,
        usages: &[Usage],
        statements: &[&Statement],
    ) {
        // Find the definition
        let definition = usages.iter().find(|u| u.kind == UsageKind::Definition);
        let definition_idx = definition.map(|u| u.statement_idx);
        let definition_is_loop_scoped = definition.is_some_and(|u| u.in_loop);

        // Field accesses (e.g., "config.paths"), method calls (e.g., "source.get_items()"),
        // and index expressions (e.g., "items[0]") don't have definitions.
        // They're valid if they contain a dot, parentheses, or square brackets.
        let is_complex_expr =
            var_name.contains('.') || var_name.contains('(') || var_name.contains('[');

        // Statement indices where `var = f(var, …)` restores ownership after the move.
        let writeback_idxs = Self::collect_direct_writeback_indices(statements, var_name);

        if definition_idx.is_none() && !is_complex_expr {
            // Parameters have no Definition in the body, but moves still need `.clone()`
            // when the parameter is used again later (e.g. `affected.push(changed_file)`
            // then `nodes[i].file_path == changed_file`), or inside loops.
            let moves: Vec<&Usage> = usages
                .iter()
                .filter(|u| u.is_move && u.kind != UsageKind::Definition)
                .collect();
            let total_uses: Vec<&Usage> = usages
                .iter()
                .filter(|u| u.kind != UsageKind::Definition)
                .collect();
            for move_usage in &moves {
                if writeback_idxs.contains(&move_usage.statement_idx) {
                    continue;
                }
                let has_later_use = total_uses
                    .iter()
                    .any(|u| move_usage.has_reachable_later_use(u));
                let same_stmt_read_after_move = total_uses.iter().any(|u| {
                    u.statement_idx == move_usage.statement_idx
                        && u.kind == UsageKind::Read
                        && !u.is_projection_parent
                        && move_usage.kind == UsageKind::Move
                });
                let same_stmt_moves = moves
                    .iter()
                    .filter(|m| m.statement_idx == move_usage.statement_idx)
                    .count();
                let same_stmt_exclusive_arm_moves = moves
                    .iter()
                    .filter(|m| {
                        m.statement_idx == move_usage.statement_idx && m.in_exclusive_match_arm
                    })
                    .count();
                // Exclusive Match arms each move `rx` once — not a multi-move conflict (P3.332).
                let multi_move_conflict =
                    same_stmt_moves > 1 && same_stmt_exclusive_arm_moves != same_stmt_moves;
                if has_later_use
                    || same_stmt_read_after_move
                    || multi_move_conflict
                    || (move_usage.in_loop && !move_usage.in_diverging_early_return)
                {
                    self.clone_sites.insert(
                        (var_name.to_string(), move_usage.statement_idx),
                        CloneReason::MovedButUsedLater,
                    );
                }
            }
            return;
        }

        // Find all moves
        let moves: Vec<&Usage> = usages
            .iter()
            .filter(|u| u.is_move && u.kind != UsageKind::Definition)
            .collect();

        if moves.is_empty() {
            // No moves, no clones needed
            return;
        }

        // For each move, check if it needs cloning:
        // 1. There are later usages after this move
        // 2. Multiple moves in the same statement
        // 3. The move is inside a loop (loop may execute again, consuming the value twice)
        let total_uses: Vec<&Usage> = usages
            .iter()
            .filter(|u| u.kind != UsageKind::Definition)
            .collect();

        for move_usage in &moves {
            // `map = f(map, …)` moves then restores — do not clone at the writeback site.
            if writeback_idxs.contains(&move_usage.statement_idx) {
                continue;
            }

            let has_later_use = total_uses
                .iter()
                .any(|u| move_usage.has_reachable_later_use(u));

            // Same statement: `visit_cycle(doc, doc.root_id, ...)` moves `doc` then reads
            // `doc.root_id` — clone the move site so the field access still compiles.
            let same_stmt_read_after_move = total_uses.iter().any(|u| {
                u.statement_idx == move_usage.statement_idx
                    && u.kind == UsageKind::Read
                    && !u.is_projection_parent
                    && move_usage.kind == UsageKind::Move
            });

            let same_stmt_moves = moves
                .iter()
                .filter(|m| m.statement_idx == move_usage.statement_idx)
                .count();
            let same_stmt_exclusive_arm_moves = moves
                .iter()
                .filter(|m| m.statement_idx == move_usage.statement_idx && m.in_exclusive_match_arm)
                .count();
            let multi_move_conflict =
                same_stmt_moves > 1 && same_stmt_exclusive_arm_moves != same_stmt_moves;

            // Moves inside loops need clone when the variable is captured from
            // an outer scope (each iteration re-uses the same binding). But
            // loop-scoped variables (for-loop pattern vars, let bindings inside
            // the body) get fresh bindings each iteration — no clone needed.
            // Function-exiting `return` inside a loop does not re-enter.
            let loop_capture_needs_clone = move_usage.in_loop
                && !definition_is_loop_scoped
                && !move_usage.in_diverging_early_return;
            let needs_clone = has_later_use
                || same_stmt_read_after_move
                || multi_move_conflict
                || loop_capture_needs_clone;

            if needs_clone {
                self.clone_sites.insert(
                    (var_name.to_string(), move_usage.statement_idx),
                    CloneReason::MovedButUsedLater,
                );
            }
        }
    }

    /// Detect partial moves: field accesses like `s.item` where `s` is used later.
    /// When `s.item` is moved (e.g., passed to a function taking ownership) and `s`
    /// itself is used afterwards, `s.item` must be cloned to avoid E0382.
    fn detect_partial_moves(
        &mut self,
        usage_map: &HashMap<String, Vec<Usage>>,
        statements: &[&Statement],
    ) {
        let field_paths: Vec<String> = usage_map
            .keys()
            .filter(|k| k.contains('.') && !k.contains('('))
            .cloned()
            .collect();

        for path in &field_paths {
            let Some(dot_pos) = path.find('.') else {
                continue;
            };
            let root = &path[..dot_pos];

            let Some(root_usages) = usage_map.get(root) else {
                continue;
            };

            let Some(field_usages) = usage_map.get(path.as_str()) else {
                continue;
            };

            let field_moves: Vec<&Usage> = field_usages
                .iter()
                .filter(|u| u.is_move && u.kind != UsageKind::Definition)
                .collect();

            for field_move in &field_moves {
                // Ignore projection-parent Reads (`buf` in later `buf.next`) — Rust allows
                // moving distinct fields of an owned struct without cloning.
                let root_used_later = root_usages.iter().any(|u| {
                    u.kind != UsageKind::Definition
                        && !u.is_projection_parent
                        && u.statement_idx > field_move.statement_idx
                });
                let field_used_later = field_usages.iter().any(|u| {
                    u.kind != UsageKind::Definition && u.statement_idx > field_move.statement_idx
                });

                // Moving a non-Copy field off `&self` is always a move-out of a shared
                // reference (E0507) — clone even when `self` is not used again later.
                // Example: `let lo = self.start.bytes; let hi = self.end.bytes` must
                // clone both, not only the first field when `self` is reused.
                // Exception: writeback patterns (codegen emits `mem::take` / bare move):
                // - extract-assign: `let mut x = self.f; …; self.f = x` (regression-042)
                // - call-arg writeback: `let r = f(self.f); …; self.f = r.remaining`
                // Exception: owned-self wither (`let mut items = self.items` then
                // reconstruct `Builder { graph: self.graph, … }`, or struct-lit-only
                // `PassBuilder { graph: self.graph, … }`) — analyzer 2.5 keeps `self`
                // Owned; distinct fields may move.
                if root == "self"
                    && Self::self_field_has_writeback(statements, path, field_move.statement_idx)
                {
                    continue;
                }
                // Only force-clone every `self.field` when there is no owned-self field
                // move in the body. Struct-lit withers count as moves (product
                // `PassBuilder::shader`) — previously only `let x = self.field` did.
                let self_always_clone =
                    root == "self" && !Self::stmts_contain_owned_self_field_move(statements);
                if self_always_clone || root_used_later || field_used_later {
                    self.clone_sites.insert(
                        (path.clone(), field_move.statement_idx),
                        CloneReason::MovedButUsedLater,
                    );
                }
            }
        }
    }

    /// Owned-self field moves in the body (`let` or struct-lit wither).
    fn stmts_contain_owned_self_field_move(statements: &[&Statement]) -> bool {
        Self::stmts_contain_self_field_move_let(statements)
    }

    /// `let x = self.field` (one-level) is an owned-self field move (analyzer 2.5).
    /// Nested `self.start.bytes` stays a `&self` extract and still always-clones.
    fn stmts_contain_self_field_move_let(statements: &[&Statement]) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Let { value, .. }
                | Statement::Expression { expr: value, .. }
                | Statement::Return {
                    value: Some(value), ..
                }
                | Statement::Assignment { value, .. } => {
                    if Self::expr_contains_self_field_move(value) {
                        return true;
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if Self::stmts_contain_self_field_move_let(then_block) {
                        return true;
                    }
                    if else_block
                        .as_ref()
                        .is_some_and(|b| Self::stmts_contain_self_field_move_let(b))
                    {
                        return true;
                    }
                }
                Statement::While { body, .. }
                | Statement::For { body, .. }
                | Statement::Loop { body, .. } => {
                    if Self::stmts_contain_self_field_move_let(body) {
                        return true;
                    }
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            if Self::stmts_contain_self_field_move_let(statements) {
                                return true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn expr_is_self_field_move(expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::FieldAccess { object, .. }
                if matches!(&**object, Expression::Identifier { name, .. } if name == "self")
        )
    }

    /// `Vox::new(self.scene)` / `Builder { graph: self.graph }` are owned-self
    // field moves, same as a let.
    fn expr_contains_self_field_move(expr: &Expression) -> bool {
        if Self::expr_is_self_field_move(expr) {
            return true;
        }
        match expr {
            Expression::Call { arguments, .. } | Expression::MethodCall { arguments, .. } => {
                arguments
                    .iter()
                    .any(|(_, arg)| Self::expr_contains_self_field_move(arg))
            }
            Expression::StructLiteral { fields, .. } => fields
                .iter()
                .any(|(_, value)| Self::expr_contains_self_field_move(value)),
            Expression::Block { statements, .. } => {
                Self::stmts_contain_self_field_move_let(statements)
            }
            Expression::Binary { left, right, .. } => {
                Self::expr_contains_self_field_move(left)
                    || Self::expr_contains_self_field_move(right)
            }
            Expression::Unary { operand, .. } | Expression::TryOp { expr: operand, .. } => {
                Self::expr_contains_self_field_move(operand)
            }
            _ => false,
        }
    }

    /// True when `field_path` is moved and later written back (extract or call-arg).
    ///
    /// Patterns:
    /// - `let mut x = self.field; …; self.field = x`
    /// - `let r = f(self.field); …; self.field = r.subfield`
    // - `map = f(map, …)` (direct binding writeback)
    pub fn self_field_has_writeback(
        statements: &[&Statement],
        field_path: &str,
        _move_stmt_idx: usize,
    ) -> bool {
        Self::find_writeback_in_stmts(statements, field_path)
    }

    /// Statement indices where ownership is restored after a move of `binding`.
    ///
    /// Patterns (same global counter scheme as [`build_usage_map`]):
    // - `binding = f(binding, …)`
    // - `let t = f(binding, …); …; binding = t.i` (tuple writeback)
    /// - `let (binding, …) = f(binding, …)` / `let binding = f(binding, …)` (same-stmt restore)
    fn collect_direct_writeback_indices(
        statements: &[&Statement],
        binding: &str,
    ) -> HashSet<usize> {
        let mut out = HashSet::new();
        let mut counter: usize = 0;
        Self::collect_direct_writeback_indices_in_stmts(
            statements,
            binding,
            &mut counter,
            &mut out,
        );
        out
    }

    /// True when `stmt_idx` moves `binding` into a call and restores it in the same statement
    /// (direct assign, same-name let, or tuple-slot rebind). Used by codegen multi-use guards.
    pub fn stmt_restores_binding_after_move(
        statements: &[&Statement],
        binding: &str,
        stmt_idx: usize,
    ) -> bool {
        Self::collect_direct_writeback_indices(statements, binding).contains(&stmt_idx)
    }

    fn pattern_binds_identifier(pattern: &Pattern, name: &str) -> bool {
        match pattern {
            Pattern::Identifier(n)
            | Pattern::MutBinding(n)
            | Pattern::Ref(n)
            | Pattern::RefMut(n) => n == name,
            Pattern::Tuple(patterns) | Pattern::Or(patterns) => patterns
                .iter()
                .any(|p| Self::pattern_binds_identifier(p, name)),
            Pattern::Reference(inner) => Self::pattern_binds_identifier(inner, name),
            Pattern::EnumVariant(_, binding) => match binding {
                crate::parser::EnumPatternBinding::None
                | crate::parser::EnumPatternBinding::Wildcard => false,
                crate::parser::EnumPatternBinding::Single(n) => n == name,
                crate::parser::EnumPatternBinding::Tuple(patterns) => patterns
                    .iter()
                    .any(|p| Self::pattern_binds_identifier(p, name)),
                crate::parser::EnumPatternBinding::Struct(fields, _) => fields
                    .iter()
                    .any(|(_, p)| Self::pattern_binds_identifier(p, name)),
            },
            Pattern::Wildcard | Pattern::Literal(_) | Pattern::Range { .. } => false,
        }
    }

    fn collect_direct_writeback_indices_in_stmts(
        statements: &[&Statement],
        binding: &str,
        counter: &mut usize,
        out: &mut HashSet<usize>,
    ) {
        for (i, stmt) in statements.iter().enumerate() {
            let idx = *counter;
            *counter += 1;

            if Self::stmt_is_direct_binding_writeback(stmt, binding) {
                out.insert(idx);
            }

            // Same-stmt ownership restore: `let binding = f(binding, …)` or
            // `let (binding, …) = f(binding, …)` — move then rebind (Row chain).
            if let Statement::Let { pattern, value, .. } = stmt {
                if Self::pattern_binds_identifier(pattern, binding)
                    && Self::expr_call_or_method_has_field_arg(value, binding)
                {
                    out.insert(idx);
                }
            }

            // `let t = f(binding, …); …; binding = t.0` — mark the Let (move site).
            if let Statement::Let {
                pattern: Pattern::Identifier(tmp),
                value,
                ..
            } = stmt
            {
                if tmp != binding && Self::expr_call_or_method_has_field_arg(value, binding) {
                    let restored = statements[i + 1..].iter().any(|later| {
                        Self::stmt_assigns_binding_or_prefix_to_field_path(later, tmp, binding)
                    });
                    if restored {
                        out.insert(idx);
                    }
                }
            }

            match stmt {
                Statement::Let { value, .. } => {
                    // Match `collect_usages_from_statement`: nested Block stmts advance
                    // the shared counter (let-match-as-expression).
                    if let Expression::Block { statements, .. } = value {
                        Self::collect_direct_writeback_indices_in_stmts(
                            statements.as_slice(),
                            binding,
                            counter,
                            out,
                        );
                    }
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    Self::collect_direct_writeback_indices_in_stmts(
                        then_block.as_slice(),
                        binding,
                        counter,
                        out,
                    );
                    if let Some(eb) = else_block {
                        Self::collect_direct_writeback_indices_in_stmts(
                            eb.as_slice(),
                            binding,
                            counter,
                            out,
                        );
                    }
                }
                Statement::While { body, .. }
                | Statement::For { body, .. }
                | Statement::Loop { body, .. } => {
                    Self::collect_direct_writeback_indices_in_stmts(
                        body.as_slice(),
                        binding,
                        counter,
                        out,
                    );
                }
                Statement::Match { arms, .. } => {
                    for arm in arms {
                        if let Expression::Block { statements, .. } = arm.body {
                            Self::collect_direct_writeback_indices_in_stmts(
                                statements.as_slice(),
                                binding,
                                counter,
                                out,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// `map = f(map, …)` or `map = map.method(…)` — move into call, assign result back.
    fn stmt_is_direct_binding_writeback(stmt: &Statement, binding: &str) -> bool {
        match stmt {
            Statement::Assignment {
                target,
                value,
                compound_op: None,
                ..
            } => {
                Self::expr_is_field_path(target, binding)
                    && Self::expr_call_or_method_has_field_arg(value, binding)
            }
            _ => false,
        }
    }

    fn find_writeback_in_stmts(statements: &[&Statement], field_path: &str) -> bool {
        for (i, stmt) in statements.iter().enumerate() {
            if Self::stmt_is_direct_binding_writeback(stmt, field_path) {
                return true;
            }
            if let Statement::Let {
                pattern: Pattern::Identifier(binding),
                value,
                ..
            } = stmt
            {
                let extracts_field = Self::expr_is_field_path(value, field_path);
                let call_moves_field = Self::expr_call_or_method_has_field_arg(value, field_path);
                if extracts_field || call_moves_field {
                    for later in statements.iter().skip(i + 1) {
                        if Self::stmt_assigns_binding_or_prefix_to_field_path(
                            later, binding, field_path,
                        ) {
                            return true;
                        }
                    }
                }
            }
            match stmt {
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if Self::find_writeback_in_stmts(then_block, field_path) {
                        return true;
                    }
                    if let Some(eb) = else_block {
                        if Self::find_writeback_in_stmts(eb, field_path) {
                            return true;
                        }
                    }
                }
                Statement::While { body, .. }
                | Statement::For { body, .. }
                | Statement::Loop { body, .. } => {
                    if Self::find_writeback_in_stmts(body, field_path) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn expr_is_field_path(expr: &Expression, field_path: &str) -> bool {
        Self::field_access_path(expr).as_deref() == Some(field_path)
    }

    /// True when `expr` is a call/method-call that passes `field_path` as an owned argument.
    fn expr_call_or_method_has_field_arg(expr: &Expression, field_path: &str) -> bool {
        match expr {
            Expression::Call { arguments, .. } | Expression::MethodCall { arguments, .. } => {
                arguments
                    .iter()
                    .any(|(_, arg)| Self::expr_is_field_path(arg, field_path))
            }
            _ => false,
        }
    }

    fn field_access_path(expr: &Expression) -> Option<String> {
        match expr {
            Expression::FieldAccess { object, field, .. } => {
                let obj = Self::field_access_path(object)?;
                Some(format!("{obj}.{field}"))
            }
            Expression::Identifier { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    /// True when `expr` is `binding` or `binding.field…` (writeback from call result).
    fn expr_is_binding_or_field_of(expr: &Expression, binding: &str) -> bool {
        match Self::field_access_path(expr) {
            Some(path) => path == binding || path.starts_with(&format!("{binding}.")),
            None => false,
        }
    }

    fn stmt_assigns_binding_or_prefix_to_field_path(
        stmt: &Statement,
        binding: &str,
        field_path: &str,
    ) -> bool {
        match stmt {
            Statement::Assignment {
                target,
                value,
                compound_op: None,
                ..
            } => {
                Self::expr_is_field_path(target, field_path)
                    && Self::expr_is_binding_or_field_of(value, binding)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block.iter().any(|s| {
                    Self::stmt_assigns_binding_or_prefix_to_field_path(s, binding, field_path)
                }) || else_block.as_ref().is_some_and(|eb| {
                    eb.iter().any(|s| {
                        Self::stmt_assigns_binding_or_prefix_to_field_path(s, binding, field_path)
                    })
                })
            }
            Statement::While { body, .. }
            | Statement::For { body, .. }
            | Statement::Loop { body, .. } => body.iter().any(|s| {
                Self::stmt_assigns_binding_or_prefix_to_field_path(s, binding, field_path)
            }),
            _ => false,
        }
    }

    /// Check if a variable needs to be cloned at a specific statement
    pub fn needs_clone(&self, var_name: &str, statement_idx: usize) -> Option<&CloneReason> {
        // Don't clone string literal variables (they're just &str references)
        if self.string_literal_vars.contains(var_name) {
            return None;
        }
        self.clone_sites.get(&(var_name.to_string(), statement_idx))
    }

    /// True when analysis recorded any clone site for this binding (used when nested
    /// statement indices in codegen don't match flat auto_clone indices).
    pub fn needs_clone_anywhere(&self, var_name: &str) -> bool {
        if self.string_literal_vars.contains(var_name) {
            return false;
        }
        self.clone_sites.keys().any(|(n, _)| n == var_name)
    }

    /// Find variables that are bound to string literals
    /// These don't need .clone() because they're just &str references.
    /// Mutable bindings (`let mut s = "…"`) are promoted to owned `String` when
    // reassigned — they still move at owned call sites and need reuse clones.
    fn find_string_literal_vars<'ast>(&mut self, statements: &[&'ast Statement<'ast>]) {
        for stmt in statements {
            match stmt {
                Statement::Let {
                    pattern: Pattern::Identifier(var_name),
                    mutable: false,
                    value,
                    ..
                }
                    // Check if value is a string literal or a match/if that returns string literals
                    if Self::expr_returns_string_literal(value) => {
                        self.string_literal_vars.insert(var_name.clone());
                    }
                Statement::Let { .. } => {
                    // Mutable / non-identifier patterns (tuple, wildcard, etc.)
                }
                Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.find_string_literal_vars(then_block);
                    if let Some(else_b) = else_block {
                        self.find_string_literal_vars(else_b);
                    }
                }
                Statement::While { body, .. }
                | Statement::For { body, .. }
                | Statement::Loop { body, .. } => {
                    self.find_string_literal_vars(body);
                }
                Statement::Match { .. } => {
                    // Match arms are expressions, handled in expr_returns_string_literal
                }
                _ => {}
            }
        }
    }

    /// Check if an expression returns a string literal
    /// This includes direct literals, match expressions with all string literal arms, etc.
    fn expr_returns_string_literal(expr: &Expression) -> bool {
        match expr {
            Expression::Literal {
                value: crate::parser::Literal::String(_),
                ..
            } => true,
            Expression::Block { statements, .. } => {
                // Check if the block ends with a match statement that returns string literals
                if let Some(Statement::Match { arms, .. }) = statements.last() {
                    arms.iter()
                        .all(|arm| Self::expr_returns_string_literal(arm.body))
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Usage {
    statement_idx: usize,
    kind: UsageKind,
    is_move: bool,
    in_loop: bool,
    /// True when this use exists only because a field/index chain projected through
    /// the binding (`buf` in `buf.scores`). Distinct field moves must not treat these
    // as whole-root reuse.
    is_projection_parent: bool,
    /// True when this use is in a Match arm expression body (arms are exclusive).
    /// Multiple moves of the same binding across Ok/Err arms must not force `.clone()`
    /// (P3.332 `recv` returning `rx` from each arm).
    in_exclusive_match_arm: bool,
    /// True when this use is in an `if` then/else that unconditionally `return`s
    /// Later sibling-path uses after the `if` are not reachable.
    in_diverging_early_return: bool,
    /// Last statement index belonging to that diverging block (inclusive).
    diverging_region_end: usize,
}

impl Usage {
    /// Later-statement reuse on a complementary path after a diverging `return`
    // is not reachable. Same-block uses still conflict.
    fn has_reachable_later_use(&self, other: &Usage) -> bool {
        if other.statement_idx <= self.statement_idx {
            return false;
        }
        if self.in_diverging_early_return {
            other.statement_idx <= self.diverging_region_end
        } else {
            true
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum UsageKind {
    Definition,
    Read,
    Write,
    Move,
}

#[cfg(test)]
#[path = "auto_clone_tests.rs"]
mod tests;
