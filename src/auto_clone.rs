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
        }
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
                    },
                );
            }
        }

        // For each variable, determine if it needs clones
        for (var_name, usages) in &usage_map {
            analysis.analyze_variable_usages(var_name, usages, &func.body);
        }

        // Partial-move detection: if a field path like "s.item" is moved,
        // and the root variable "s" has later uses, the field access must
        // be cloned to avoid a partial move error (E0382).
        analysis.detect_partial_moves(&usage_map, &func.body);

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
                let value_kind = match value {
                    Expression::FieldAccess { .. } | Expression::Identifier { .. } => {
                        UsageKind::Move
                    }
                    _ => UsageKind::Read,
                };
                Self::collect_usages_from_expression(value, idx, value_kind, in_loop, map, registry);

                if let Pattern::Identifier(name) = pattern {
                    map.entry(name.clone()).or_default().push(Usage {
                        statement_idx: idx,
                        kind: UsageKind::Definition,
                        is_move: false,
                        in_loop,
                        is_projection_parent: false,
                    });
                }
            }
            Statement::Assignment { target, value, .. } => {
                Self::collect_usages_from_expression(target, idx, UsageKind::Write, in_loop, map, registry);
                // Owned identifiers move on assignment; loop bodies may assign the same
                // param on every iteration (E0382 without `.clone()` at the use site).
                let value_kind = match value {
                    Expression::Identifier { .. } => UsageKind::Move,
                    _ => UsageKind::Read,
                };
                Self::collect_usages_from_expression(value, idx, value_kind, in_loop, map, registry);
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                Self::collect_usages_from_expression(expr, idx, UsageKind::Move, in_loop, map, registry);
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
                Self::collect_usages_from_expression(condition, idx, UsageKind::Read, in_loop, map, registry);
                for stmt in then_block.iter() {
                    Self::collect_usages_from_statement(stmt, counter, in_loop, map, registry);
                }
                if let Some(else_b) = else_block {
                    for stmt in else_b.iter() {
                        Self::collect_usages_from_statement(stmt, counter, in_loop, map, registry);
                    }
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                Self::collect_usages_from_expression(condition, idx, UsageKind::Read, in_loop, map, registry);
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
                Self::collect_usages_from_expression(iterable, idx, UsageKind::Read, in_loop, map, registry);
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
                Self::collect_usages_from_expression(value, idx, UsageKind::Read, in_loop, map, registry);
                for arm in arms {
                    // Process arm body blocks using the parent counter (like
                    // Statement::If does for then_block/else_block) so that
                    // statement indices stay synchronized with the codegen's
                    // auto_clone_counter which is global.
                    if let Expression::Block { statements, .. } = arm.body {
                        for stmt in statements {
                            Self::collect_usages_from_statement(stmt, counter, in_loop, map, registry);
                        }
                    } else {
                        Self::collect_usages_from_expression(
                            arm.body,
                            idx,
                            UsageKind::Read,
                            in_loop,
                            map,
                            registry,
                        );
                    }
                }
            }
            _ => {}
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

        for (key, sig) in registry.all_signatures_for_suffix_search() {
            let simple = key.rsplit("::").next().unwrap_or(key.as_str());
            if simple != method {
                continue;
            }
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
                    });
                }
                // Parent binding uses from `root.field` are not whole-root reuse.
                Self::collect_field_projection_parent_usages(
                    object, idx, in_loop, map, registry,
                );
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                Self::collect_usages_from_expression(function, idx, UsageKind::Read, in_loop, map, registry);
                for (i, (_label, arg_expr)) in arguments.iter().enumerate() {
                    let arg_kind = if registry
                        .is_some_and(|r| Self::callee_arg_field_extracts(function, i, r))
                    {
                        UsageKind::Read
                    } else {
                        UsageKind::Move
                    };
                    Self::collect_usages_from_expression(
                        arg_expr,
                        idx,
                        arg_kind,
                        in_loop,
                        map,
                        registry,
                    );
                }
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                if let Some(path) = Self::extract_expression_path(expr) {
                    map.entry(path).or_default().push(Usage {
                        statement_idx: idx,
                        kind,
                        is_move: kind == UsageKind::Move,
                        in_loop,
                        is_projection_parent: false,
                    });
                }
                Self::collect_usages_from_expression(object, idx, UsageKind::Read, in_loop, map, registry);
                for (i, (_label, arg_expr)) in arguments.iter().enumerate() {
                    // Signature-driven: owned formals move (need `.clone()` on reuse).
                    // Do NOT treat every method named `get`/`remove` as a HashMap key borrow —
                    // trait methods like `StorageEngine::get(txn: Txn, key)` take owned args.
                    let arg_kind = Self::method_call_arg_usage_kind(
                        method,
                        i,
                        arguments.len(),
                        registry,
                    );
                    Self::collect_usages_from_expression(arg_expr, idx, arg_kind, in_loop, map, registry);
                }
            }
            Expression::Binary { left, right, .. } => {
                Self::collect_usages_from_expression(left, idx, UsageKind::Read, in_loop, map, registry);
                Self::collect_usages_from_expression(right, idx, UsageKind::Read, in_loop, map, registry);
            }
            Expression::Unary { operand, .. } => {
                Self::collect_usages_from_expression(operand, idx, UsageKind::Read, in_loop, map, registry);
            }
            Expression::Index { object, index, .. } => {
                if let Some(path) = Self::extract_expression_path(expr) {
                    map.entry(path).or_default().push(Usage {
                        statement_idx: idx,
                        kind,
                        is_move: kind == UsageKind::Move,
                        in_loop,
                        is_projection_parent: false,
                    });
                }
                Self::collect_usages_from_expression(object, idx, UsageKind::Read, in_loop, map, registry);
                Self::collect_usages_from_expression(index, idx, UsageKind::Read, in_loop, map, registry);
            }
            Expression::Tuple { elements, .. } => {
                for elem in elements {
                    let elem_kind = match elem {
                        Expression::Identifier { .. } | Expression::FieldAccess { .. } => {
                            UsageKind::Move
                        }
                        _ => UsageKind::Read,
                    };
                    Self::collect_usages_from_expression(elem, idx, elem_kind, in_loop, map, registry);
                }
            }
            Expression::Array { elements, .. } => {
                for elem in elements {
                    Self::collect_usages_from_expression(elem, idx, UsageKind::Move, in_loop, map, registry);
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
                    Self::collect_usages_from_statement(stmt, &mut block_counter, in_loop, map, registry);
                }
            }
            Expression::Cast { expr, .. } => {
                Self::collect_usages_from_expression(expr, idx, UsageKind::Read, in_loop, map, registry);
            }
            Expression::Range { start, end, .. } => {
                Self::collect_usages_from_expression(start, idx, UsageKind::Read, in_loop, map, registry);
                Self::collect_usages_from_expression(end, idx, UsageKind::Read, in_loop, map, registry);
            }
            Expression::TryOp { expr, .. } => {
                Self::collect_usages_from_expression(expr, idx, UsageKind::Read, in_loop, map, registry);
            }
            Expression::Await { expr, .. } => {
                Self::collect_usages_from_expression(expr, idx, UsageKind::Read, in_loop, map, registry);
            }
            Expression::ChannelSend { channel, value, .. } => {
                Self::collect_usages_from_expression(channel, idx, UsageKind::Read, in_loop, map, registry);
                Self::collect_usages_from_expression(value, idx, UsageKind::Move, in_loop, map, registry);
            }
            Expression::ChannelRecv { channel, .. } => {
                Self::collect_usages_from_expression(channel, idx, UsageKind::Read, in_loop, map, registry);
            }
            Expression::MacroInvocation { args, .. } => {
                for arg in args {
                    Self::collect_usages_from_expression(arg, idx, UsageKind::Read, in_loop, map, registry);
                }
            }
            Expression::MapLiteral { pairs, .. } => {
                for (key, value) in pairs {
                    Self::collect_usages_from_expression(key, idx, UsageKind::Move, in_loop, map, registry);
                    Self::collect_usages_from_expression(value, idx, UsageKind::Move, in_loop, map, registry);
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
            crate::parser::Pattern::Identifier(name) => {
                map.entry(name.clone()).or_default().push(Usage {
                    statement_idx,
                    kind: UsageKind::Definition,
                    is_move: false,
                    in_loop,
                        is_projection_parent: false,
                    });
            }
            crate::parser::Pattern::Tuple(patterns) => {
                for p in patterns {
                    Self::register_pattern_definitions(p, statement_idx, in_loop, map);
                }
            }
            _ => {}
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
                    .any(|u| u.statement_idx > move_usage.statement_idx);
                let same_stmt_read_after_move = total_uses.iter().any(|u| {
                    u.statement_idx == move_usage.statement_idx
                        && u.kind == UsageKind::Read
                        && move_usage.kind == UsageKind::Move
                });
                let same_stmt_moves = moves
                    .iter()
                    .filter(|m| m.statement_idx == move_usage.statement_idx)
                    .count();
                if has_later_use
                    || same_stmt_read_after_move
                    || same_stmt_moves > 1
                    || move_usage.in_loop
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
                .any(|u| u.statement_idx > move_usage.statement_idx);

            // Same statement: `visit_cycle(doc, doc.root_id, ...)` moves `doc` then reads
            // `doc.root_id` — clone the move site so the field access still compiles.
            let same_stmt_read_after_move = total_uses.iter().any(|u| {
                u.statement_idx == move_usage.statement_idx
                    && u.kind == UsageKind::Read
                    && move_usage.kind == UsageKind::Move
            });

            let same_stmt_moves = moves
                .iter()
                .filter(|m| m.statement_idx == move_usage.statement_idx)
                .count();

            // Moves inside loops need clone when the variable is captured from
            // an outer scope (each iteration re-uses the same binding). But
            // loop-scoped variables (for-loop pattern vars, let bindings inside
            // the body) get fresh bindings each iteration — no clone needed.
            let loop_capture_needs_clone = move_usage.in_loop && !definition_is_loop_scoped;
            let needs_clone = has_later_use
                || same_stmt_read_after_move
                || same_stmt_moves > 1
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
                // moving distinct fields of an owned struct without cloning (WDB-096).
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
                if root == "self"
                    && Self::self_field_has_writeback(statements, path, field_move.statement_idx)
                {
                    continue;
                }
                if root == "self" || root_used_later || field_used_later {
                    self.clone_sites.insert(
                        (path.clone(), field_move.statement_idx),
                        CloneReason::MovedButUsedLater,
                    );
                }
            }
        }
    }

    /// True when `field_path` is moved and later written back (extract or call-arg).
    ///
    /// Patterns:
    /// - `let mut x = self.field; …; self.field = x`
    /// - `let r = f(self.field); …; self.field = r.subfield`
    /// - `map = f(map, …)` (direct binding writeback — WDB-084)
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
    /// - `binding = f(binding, …)` (WDB-084)
    /// - `let t = f(binding, …); …; binding = t.i` (WDB-087 tuple writeback)
    fn collect_direct_writeback_indices(
        statements: &[&Statement],
        binding: &str,
    ) -> HashSet<usize> {
        let mut out = HashSet::new();
        let mut counter: usize = 0;
        Self::collect_direct_writeback_indices_in_stmts(statements, binding, &mut counter, &mut out);
        out
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

            // WDB-087: `let t = f(binding, …); …; binding = t.0` — mark the Let (move site).
            if let Statement::Let {
                pattern: Pattern::Identifier(tmp),
                value,
                ..
            } = stmt
            {
                if Self::expr_call_or_method_has_field_arg(value, binding) {
                    let restored = statements[i + 1..].iter().any(|later| {
                        Self::stmt_assigns_binding_or_prefix_to_field_path(later, tmp, binding)
                    });
                    if restored {
                        out.insert(idx);
                    }
                }
            }

            match stmt {
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
    /// These don't need .clone() because they're just &str references
    fn find_string_literal_vars<'ast>(&mut self, statements: &[&'ast Statement<'ast>]) {
        for stmt in statements {
            match stmt {
                Statement::Let {
                    pattern: Pattern::Identifier(var_name),
                    value,
                    ..
                }
                    // Check if value is a string literal or a match/if that returns string literals
                    if Self::expr_returns_string_literal(value) => {
                        self.string_literal_vars.insert(var_name.clone());
                    }
                Statement::Let { .. } => {
                    // Non-identifier patterns (tuple, wildcard, etc.)
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
    /// as whole-root reuse (WDB-096).
    is_projection_parent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum UsageKind {
    Definition,
    Read,
    Write,
    Move,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{test_alloc_expr, test_alloc_stmt};

    #[test]
    fn test_simple_move_and_reuse() {
        // let x = vec![1, 2, 3]
        // takes_ownership(x)  // <- Should insert .clone() here
        // println!("{}", x.len())

        let func = FunctionDecl {
            name: "test".to_string(),
            is_pub: false,
            is_extern: false,
            parameters: vec![],
            return_type: None,
            return_decorators: Vec::new(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parent_type: None,
            impl_trait: None,
            doc_comment: None,
            body: vec![
                test_alloc_stmt(Statement::Let {
                    pattern: Pattern::Identifier("x".to_string()),
                    mutable: false,
                    type_: None,
                    value: test_alloc_expr(Expression::Array {
                        elements: vec![
                            test_alloc_expr(Expression::Literal {
                                value: Literal::Int(1),
                                location: None,
                            }),
                            test_alloc_expr(Expression::Literal {
                                value: Literal::Int(2),
                                location: None,
                            }),
                            test_alloc_expr(Expression::Literal {
                                value: Literal::Int(3),
                                location: None,
                            }),
                        ],
                        location: None,
                    }),
                    else_block: None,
                    location: None,
                }),
                test_alloc_stmt(Statement::Expression {
                    expr: test_alloc_expr(Expression::Call {
                        function: test_alloc_expr(Expression::Identifier {
                            name: "takes_ownership".to_string(),
                            location: None,
                        }),
                        arguments: vec![(
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "x".to_string(),
                                location: None,
                            }),
                        )],
                        location: None,
                    }),
                    location: None,
                }),
                test_alloc_stmt(Statement::Expression {
                    expr: test_alloc_expr(Expression::MethodCall {
                        object: test_alloc_expr(Expression::Identifier {
                            name: "x".to_string(),
                            location: None,
                        }),
                        method: "len".to_string(),
                        arguments: vec![],
                        type_args: None,
                        location: None,
                    }),
                    location: None,
                }),
            ],
        };

        let analysis = AutoCloneAnalysis::analyze_function(&func);

        // Should detect that x needs to be cloned at statement 1 (the function call)
        assert!(analysis.needs_clone("x", 1).is_some());
        assert_eq!(
            analysis.needs_clone("x", 1),
            Some(&CloneReason::MovedButUsedLater)
        );
    }

    #[test]
    fn test_owned_field_extract_value_param_still_needs_clone_on_reuse() {
        // regression-063: value_tag(value: Value) match-projects but emits owned Value —
        // field_extract must not demote Move→Read; first call needs .clone().
        let mut registry = crate::analyzer::SignatureRegistry::new();
        let mut sig = crate::analyzer::FunctionSignature {
            name: "value_tag".to_string(),
            param_types: vec![Type::Custom("Value".to_string())],
            formal_param_types: vec![Type::Custom("Value".to_string())],
            param_ownership: vec![crate::analyzer::OwnershipMode::Borrowed],
            return_type: Some(Type::Int32),
            return_ownership: crate::analyzer::OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: Some(vec![false]),
            field_extract_params: Some(vec![true]),
            forwarding_borrow_params: None,
        };
        registry.add_function("value_tag".to_string(), sig.clone());
        sig.name = "value_i64".to_string();
        registry.add_function("value_i64".to_string(), sig);

        let func = FunctionDecl {
            name: "seed_write".to_string(),
            is_pub: false,
            is_extern: false,
            parameters: vec![Parameter {
                name: "value".to_string(),
                pattern: None,
                type_: Type::Custom("Value".to_string()),
                ownership: OwnershipHint::Owned,
                is_mutable: false,
                decorators: vec![],
            }],
            return_type: None,
            return_decorators: Vec::new(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parent_type: None,
            impl_trait: None,
            doc_comment: None,
            body: vec![
                test_alloc_stmt(Statement::Let {
                    pattern: Pattern::Identifier("tag".to_string()),
                    mutable: false,
                    type_: None,
                    value: test_alloc_expr(Expression::Call {
                        function: test_alloc_expr(Expression::Identifier {
                            name: "value_tag".to_string(),
                            location: None,
                        }),
                        arguments: vec![(
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "value".to_string(),
                                location: None,
                            }),
                        )],
                        location: None,
                    }),
                    else_block: None,
                    location: None,
                }),
                test_alloc_stmt(Statement::Let {
                    pattern: Pattern::Identifier("payload".to_string()),
                    mutable: false,
                    type_: None,
                    value: test_alloc_expr(Expression::Call {
                        function: test_alloc_expr(Expression::Identifier {
                            name: "value_i64".to_string(),
                            location: None,
                        }),
                        arguments: vec![(
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "value".to_string(),
                                location: None,
                            }),
                        )],
                        location: None,
                    }),
                    else_block: None,
                    location: None,
                }),
            ],
        };

        let analysis = AutoCloneAnalysis::analyze_function_with_registry(&func, Some(&registry));
        assert!(
            analysis.needs_clone("value", 0).is_some(),
            "owned field-extract formal must still move — clone at first use"
        );
    }

    #[test]
    fn test_param_let_alias_then_reuse_needs_clone_at_alias() {
        let func = FunctionDecl {
            name: "send".to_string(),
            is_pub: false,
            is_extern: false,
            parameters: vec![Parameter {
                name: "message".to_string(),
                pattern: None,
                type_: Type::Custom("Message".to_string()),
                ownership: OwnershipHint::Owned,
                is_mutable: false,
                decorators: vec![],
            }],
            return_type: None,
            return_decorators: Vec::new(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parent_type: None,
            impl_trait: None,
            doc_comment: None,
            body: vec![
                test_alloc_stmt(Statement::Let {
                    pattern: Pattern::Identifier("msg_copy".to_string()),
                    mutable: false,
                    type_: None,
                    value: test_alloc_expr(Expression::Identifier {
                        name: "message".to_string(),
                        location: None,
                    }),
                    else_block: None,
                    location: None,
                }),
                test_alloc_stmt(Statement::Expression {
                    expr: test_alloc_expr(Expression::MethodCall {
                        object: test_alloc_expr(Expression::Identifier {
                            name: "msg_copy".to_string(),
                            location: None,
                        }),
                        method: "use_it".to_string(),
                        arguments: vec![],
                        type_args: None,
                        location: None,
                    }),
                    location: None,
                }),
                test_alloc_stmt(Statement::Expression {
                    expr: test_alloc_expr(Expression::Call {
                        function: test_alloc_expr(Expression::Identifier {
                            name: "push".to_string(),
                            location: None,
                        }),
                        arguments: vec![(
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "message".to_string(),
                                location: None,
                            }),
                        )],
                        location: None,
                    }),
                    location: None,
                }),
            ],
        };

        let analysis = AutoCloneAnalysis::analyze_function(&func);
        assert!(
            analysis.needs_clone("message", 0).is_some(),
            "let msg_copy = message moves param; push(message) later needs clone at let"
        );
    }

    #[test]
    fn test_no_clone_needed_single_use() {
        // let x = vec![1, 2, 3]
        // takes_ownership(x)  // <- No clone needed, x not used again

        let func = FunctionDecl {
            name: "test".to_string(),
            is_pub: false,
            is_extern: false,
            parameters: vec![],
            return_type: None,
            return_decorators: Vec::new(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parent_type: None,
            impl_trait: None,
            doc_comment: None,
            body: vec![
                test_alloc_stmt(Statement::Let {
                    pattern: Pattern::Identifier("x".to_string()),
                    mutable: false,
                    type_: None,
                    value: test_alloc_expr(Expression::Array {
                        elements: vec![],
                        location: None,
                    }),
                    else_block: None,
                    location: None,
                }),
                test_alloc_stmt(Statement::Expression {
                    expr: test_alloc_expr(Expression::Call {
                        function: test_alloc_expr(Expression::Identifier {
                            name: "takes_ownership".to_string(),
                            location: None,
                        }),
                        arguments: vec![(
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "x".to_string(),
                                location: None,
                            }),
                        )],
                        location: None,
                    }),
                    location: None,
                }),
            ],
        };

        let analysis = AutoCloneAnalysis::analyze_function(&func);

        // Should NOT detect any clones needed
        assert!(analysis.needs_clone("x", 1).is_none());
    }

    #[test]
    fn test_distinct_owned_field_moves_do_not_clone() {
        // WDB-096: return_f64(buf.scores); return_f64(buf.next) — distinct fields.
        let func = FunctionDecl {
            name: "release".to_string(),
            is_pub: false,
            is_extern: false,
            parameters: vec![Parameter {
                name: "buf".to_string(),
                pattern: None,
                type_: Type::Custom("Buf".to_string()),
                ownership: OwnershipHint::Owned,
                is_mutable: false,
                decorators: vec![],
            }],
            return_type: None,
            return_decorators: Vec::new(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parent_type: None,
            impl_trait: None,
            doc_comment: None,
            body: vec![
                test_alloc_stmt(Statement::Expression {
                    expr: test_alloc_expr(Expression::Call {
                        function: test_alloc_expr(Expression::Identifier {
                            name: "return_f64".to_string(),
                            location: None,
                        }),
                        arguments: vec![(
                            None,
                            test_alloc_expr(Expression::FieldAccess {
                                object: test_alloc_expr(Expression::Identifier {
                                    name: "buf".to_string(),
                                    location: None,
                                }),
                                field: "scores".to_string(),
                                location: None,
                            }),
                        )],
                        location: None,
                    }),
                    location: None,
                }),
                test_alloc_stmt(Statement::Expression {
                    expr: test_alloc_expr(Expression::Call {
                        function: test_alloc_expr(Expression::Identifier {
                            name: "return_f64".to_string(),
                            location: None,
                        }),
                        arguments: vec![(
                            None,
                            test_alloc_expr(Expression::FieldAccess {
                                object: test_alloc_expr(Expression::Identifier {
                                    name: "buf".to_string(),
                                    location: None,
                                }),
                                field: "next".to_string(),
                                location: None,
                            }),
                        )],
                        location: None,
                    }),
                    location: None,
                }),
            ],
        };

        let analysis = AutoCloneAnalysis::analyze_function(&func);
        assert!(
            analysis.needs_clone("buf.scores", 0).is_none(),
            "distinct field move must not clone earlier field"
        );
        assert!(
            analysis.needs_clone("buf.next", 1).is_none(),
            "distinct field move must not clone later field"
        );
    }

    #[test]
    fn test_self_field_in_if_expr_inside_while_loop_needs_clone() {
        // Reproduces rating.wj bug:
        //   while i <= self.max {
        //       let star_color = if filled { self.color } else { "#e2e8f0" }
        //       html.push_str(star_color)
        //       i += 1
        //   }
        // self.color is a String field used in a while loop if-expression.
        // It must be cloned because the loop executes multiple times.

        let func = FunctionDecl {
            name: "render".to_string(),
            is_pub: false,
            is_extern: false,
            parameters: vec![Parameter {
                name: "self".to_string(),
                pattern: None,
                type_: Type::Custom("Rating".to_string()),
                ownership: OwnershipHint::Owned,
                is_mutable: false,
                decorators: vec![],
            }],
            return_type: Some(Type::String),
            return_decorators: Vec::new(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parent_type: Some("Rating".to_string()),
            impl_trait: None,
            doc_comment: None,
            body: vec![
                // let mut i = 1
                test_alloc_stmt(Statement::Let {
                    pattern: Pattern::Identifier("i".to_string()),
                    mutable: true,
                    type_: None,
                    value: test_alloc_expr(Expression::Literal {
                        value: Literal::Int(1),
                        location: None,
                    }),
                    else_block: None,
                    location: None,
                }),
                // while i <= self.max {
                test_alloc_stmt(Statement::While {
                    condition: test_alloc_expr(Expression::Binary {
                        left: test_alloc_expr(Expression::Identifier {
                            name: "i".to_string(),
                            location: None,
                        }),
                        op: BinaryOp::Le,
                        right: test_alloc_expr(Expression::FieldAccess {
                            object: test_alloc_expr(Expression::Identifier {
                                name: "self".to_string(),
                                location: None,
                            }),
                            field: "max".to_string(),
                            location: None,
                        }),
                        location: None,
                    }),
                    body: vec![
                        // let star_color = if filled { self.color } else { "#e2e8f0" }
                        test_alloc_stmt(Statement::Let {
                            pattern: Pattern::Identifier("star_color".to_string()),
                            mutable: false,
                            type_: None,
                            value: test_alloc_expr(Expression::Block {
                                is_unsafe: false,
                                statements: vec![test_alloc_stmt(Statement::If {
                                    condition: test_alloc_expr(Expression::Identifier {
                                        name: "filled".to_string(),
                                        location: None,
                                    }),
                                    then_block: vec![test_alloc_stmt(Statement::Expression {
                                        expr: test_alloc_expr(Expression::FieldAccess {
                                            object: test_alloc_expr(Expression::Identifier {
                                                name: "self".to_string(),
                                                location: None,
                                            }),
                                            field: "color".to_string(),
                                            location: None,
                                        }),
                                        location: None,
                                    })],
                                    else_block: Some(vec![test_alloc_stmt(
                                        Statement::Expression {
                                            expr: test_alloc_expr(Expression::Literal {
                                                value: Literal::String("#e2e8f0".to_string()),
                                                location: None,
                                            }),
                                            location: None,
                                        },
                                    )]),
                                    location: None,
                                })],
                                location: None,
                            }),
                            else_block: None,
                            location: None,
                        }),
                        // i += 1
                        test_alloc_stmt(Statement::Assignment {
                            target: test_alloc_expr(Expression::Identifier {
                                name: "i".to_string(),
                                location: None,
                            }),
                            value: test_alloc_expr(Expression::Binary {
                                left: test_alloc_expr(Expression::Identifier {
                                    name: "i".to_string(),
                                    location: None,
                                }),
                                op: BinaryOp::Add,
                                right: test_alloc_expr(Expression::Literal {
                                    value: Literal::Int(1),
                                    location: None,
                                }),
                                location: None,
                            }),
                            compound_op: None,
                            location: None,
                        }),
                    ],
                    location: None,
                }),
            ],
        };

        let analysis = AutoCloneAnalysis::analyze_function(&func);

        // self.color is used inside a while loop but defined outside (it's a field
        // of self). The loop executes multiple times, so each iteration would try
        // to move self.color. Auto-clone must detect this.
        assert!(
            analysis.needs_clone_anywhere("self.color"),
            "self.color in while loop if-expression must be flagged for clone"
        );
    }

    #[test]
    fn test_owned_field_extract_without_emitted_flags_still_moves() {
        let mut registry = crate::analyzer::SignatureRegistry::new();
        let sig = crate::analyzer::FunctionSignature {
            name: "value_tag".to_string(),
            param_types: vec![Type::Reference(Box::new(Type::Custom("Value".to_string())))],
            formal_param_types: vec![Type::Custom("Value".to_string())],
            param_ownership: vec![crate::analyzer::OwnershipMode::Borrowed],
            return_type: Some(Type::Int32),
            return_ownership: crate::analyzer::OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            field_extract_params: Some(vec![true]),
            forwarding_borrow_params: None,
        };
        registry.add_function("value_tag".to_string(), sig.clone());
        let mut sig2 = sig.clone();
        sig2.name = "value_i64".to_string();
        registry.add_function("value_i64".to_string(), sig2);

        let func = FunctionDecl {
            name: "seed_write".to_string(),
            is_pub: false,
            is_extern: false,
            parameters: vec![Parameter {
                name: "value".to_string(),
                pattern: None,
                type_: Type::Custom("Value".to_string()),
                ownership: OwnershipHint::Owned,
                is_mutable: false,
                decorators: vec![],
            }],
            return_type: None,
            return_decorators: Vec::new(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parent_type: None,
            impl_trait: None,
            doc_comment: None,
            body: vec![
                test_alloc_stmt(Statement::Let {
                    pattern: Pattern::Identifier("tag".to_string()),
                    mutable: false,
                    type_: None,
                    value: test_alloc_expr(Expression::Call {
                        function: test_alloc_expr(Expression::Identifier {
                            name: "value_tag".to_string(),
                            location: None,
                        }),
                        arguments: vec![(
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "value".to_string(),
                                location: None,
                            }),
                        )],
                        location: None,
                    }),
                    else_block: None,
                    location: None,
                }),
                test_alloc_stmt(Statement::Let {
                    pattern: Pattern::Identifier("payload".to_string()),
                    mutable: false,
                    type_: None,
                    value: test_alloc_expr(Expression::Call {
                        function: test_alloc_expr(Expression::Identifier {
                            name: "value_i64".to_string(),
                            location: None,
                        }),
                        arguments: vec![(
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "value".to_string(),
                                location: None,
                            }),
                        )],
                        location: None,
                    }),
                    else_block: None,
                    location: None,
                }),
            ],
        };

        let analysis = AutoCloneAnalysis::analyze_function_with_registry(&func, Some(&registry));
        assert!(
            analysis.needs_clone("value", 0).is_some(),
            "bare WJ formal + Reference param_types must still move; got sites={:?}",
            analysis.clone_sites
        );
    }

    #[test]
    fn test_seed_write_value_reuse_needs_clone_without_registry() {
        let func = FunctionDecl {
            name: "seed_write".to_string(),
            is_pub: false,
            is_extern: false,
            parameters: vec![Parameter {
                name: "value".to_string(),
                pattern: None,
                type_: Type::Custom("Value".to_string()),
                ownership: OwnershipHint::Owned,
                is_mutable: false,
                decorators: vec![],
            }],
            return_type: None,
            return_decorators: Vec::new(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parent_type: None,
            impl_trait: None,
            doc_comment: None,
            body: vec![
                test_alloc_stmt(Statement::Let {
                    pattern: Pattern::Identifier("tag".to_string()),
                    mutable: false,
                    type_: None,
                    value: test_alloc_expr(Expression::Call {
                        function: test_alloc_expr(Expression::Identifier {
                            name: "value_tag".to_string(),
                            location: None,
                        }),
                        arguments: vec![(
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "value".to_string(),
                                location: None,
                            }),
                        )],
                        location: None,
                    }),
                    else_block: None,
                    location: None,
                }),
                test_alloc_stmt(Statement::Let {
                    pattern: Pattern::Identifier("payload".to_string()),
                    mutable: false,
                    type_: None,
                    value: test_alloc_expr(Expression::Call {
                        function: test_alloc_expr(Expression::Identifier {
                            name: "value_i64".to_string(),
                            location: None,
                        }),
                        arguments: vec![(
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "value".to_string(),
                                location: None,
                            }),
                        )],
                        location: None,
                    }),
                    else_block: None,
                    location: None,
                }),
            ],
        };
        let analysis = AutoCloneAnalysis::analyze_function(&func);
        assert!(
            analysis.needs_clone("value", 0).is_some(),
            "seed_write without registry must still clone; sites={:?}",
            analysis.clone_sites
        );
    }

    #[test]
    fn trait_owned_get_arg_not_treated_as_map_key_borrow() {
        // `StorageEngine::get(txn: Txn, key)` must not inherit HashMap::get's map-key
        // Read heuristic — first call moves `txn`, second needs `.clone()`.
        let mut registry = crate::analyzer::SignatureRegistry::new();
        // Homonym: stdlib-style borrowed map key.
        registry.add_function(
            "HashMap::get".to_string(),
            crate::analyzer::FunctionSignature {
                name: "get".to_string(),
                param_types: vec![
                    crate::parser::Type::Reference(Box::new(crate::parser::Type::Custom(
                        "HashMap".to_string(),
                    ))),
                    crate::parser::Type::Reference(Box::new(crate::parser::Type::String)),
                ],
                formal_param_types: vec![
                    crate::parser::Type::Custom("HashMap".to_string()),
                    crate::parser::Type::String,
                ],
                param_ownership: vec![
                    crate::analyzer::OwnershipMode::Borrowed,
                    crate::analyzer::OwnershipMode::Borrowed,
                ],
                return_type: None,
                return_ownership: crate::analyzer::OwnershipMode::Owned,
                has_self_receiver: true,
                is_extern: false,
                emitted_rust_ref_params: Some(vec![true, true]),
                field_extract_params: None,
                forwarding_borrow_params: None,
            },
        );
        registry.add_function(
            "StorageEngine::get".to_string(),
            crate::analyzer::FunctionSignature {
                name: "get".to_string(),
                param_types: vec![
                    crate::parser::Type::Reference(Box::new(crate::parser::Type::Custom(
                        "StorageEngine".to_string(),
                    ))),
                    crate::parser::Type::Custom("Txn".to_string()),
                    crate::parser::Type::String,
                ],
                formal_param_types: vec![
                    crate::parser::Type::Custom("StorageEngine".to_string()),
                    crate::parser::Type::Custom("Txn".to_string()),
                    crate::parser::Type::String,
                ],
                param_ownership: vec![
                    crate::analyzer::OwnershipMode::Borrowed,
                    crate::analyzer::OwnershipMode::Owned,
                    crate::analyzer::OwnershipMode::Owned,
                ],
                return_type: None,
                return_ownership: crate::analyzer::OwnershipMode::Owned,
                has_self_receiver: true,
                is_extern: false,
                emitted_rust_ref_params: Some(vec![true, false, false]),
                field_extract_params: None,
                forwarding_borrow_params: None,
            },
        );

        let func = FunctionDecl {
            name: "read_twice".to_string(),
            is_pub: false,
            is_extern: false,
            parameters: vec![
                Parameter {
                    name: "engine".to_string(),
                    pattern: None,
                    type_: Type::Custom("MemoryEngine".to_string()),
                    ownership: OwnershipHint::Owned,
                    is_mutable: false,
                    decorators: vec![],
                },
                Parameter {
                    name: "txn".to_string(),
                    pattern: None,
                    type_: Type::Custom("Txn".to_string()),
                    ownership: OwnershipHint::Owned,
                    is_mutable: false,
                    decorators: vec![],
                },
            ],
            return_type: None,
            return_decorators: Vec::new(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![],
            is_async: false,
            parent_type: None,
            impl_trait: None,
            doc_comment: None,
            body: vec![
                test_alloc_stmt(Statement::Let {
                    pattern: Pattern::Identifier("a".to_string()),
                    mutable: false,
                    type_: None,
                    value: test_alloc_expr(Expression::MethodCall {
                        object: test_alloc_expr(Expression::Identifier {
                            name: "engine".to_string(),
                            location: None,
                        }),
                        method: "get".to_string(),
                        arguments: vec![
                            (
                                None,
                                test_alloc_expr(Expression::Identifier {
                                    name: "txn".to_string(),
                                    location: None,
                                }),
                            ),
                            (
                                None,
                                test_alloc_expr(Expression::Literal {
                                    value: Literal::String("a".to_string()),
                                    location: None,
                                }),
                            ),
                        ],
                        type_args: None,
                        location: None,
                    }),
                    else_block: None,
                    location: None,
                }),
                test_alloc_stmt(Statement::Let {
                    pattern: Pattern::Identifier("b".to_string()),
                    mutable: false,
                    type_: None,
                    value: test_alloc_expr(Expression::MethodCall {
                        object: test_alloc_expr(Expression::Identifier {
                            name: "engine".to_string(),
                            location: None,
                        }),
                        method: "get".to_string(),
                        arguments: vec![
                            (
                                None,
                                test_alloc_expr(Expression::Identifier {
                                    name: "txn".to_string(),
                                    location: None,
                                }),
                            ),
                            (
                                None,
                                test_alloc_expr(Expression::Literal {
                                    value: Literal::String("b".to_string()),
                                    location: None,
                                }),
                            ),
                        ],
                        type_args: None,
                        location: None,
                    }),
                    else_block: None,
                    location: None,
                }),
            ],
        };

        let analysis = AutoCloneAnalysis::analyze_function_with_registry(&func, Some(&registry));
        assert!(
            analysis.needs_clone("txn", 0).is_some(),
            "owned trait get arg must clone on reuse despite HashMap::get homonym; sites={:?}",
            analysis.clone_sites
        );
    }

}
