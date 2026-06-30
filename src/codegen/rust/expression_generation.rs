//! Expression Generation Module
//!
//! Handles generation of Rust code for all expression types:
//! - Literals, identifiers, binary/unary operations
//! - Function and method calls
//! - Field access, index access
//! - Struct/array/map literals
//! - Closures, blocks, match expressions
//! - Cast, try, await, range expressions

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use super::constant_folding;
use super::float_type_utilities;
use super::pattern_analysis;
use super::CodeGenerator;

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {
    /// Field-type map is keyed by the struct's declared name (`GpuVertex`); literals may use a
    /// qualified path (`ffi::GpuVertex`). Try full path, then the last `::` segment.
    pub(in crate::codegen::rust) fn lookup_struct_field_types(
        &self,
        struct_name: &str,
    ) -> Option<&std::collections::HashMap<String, Type>> {
        if let Some(fields) = self.struct_field_types.get(struct_name) {
            return Some(fields);
        }
        if let Some(short) = struct_name.rsplit("::").next() {
            if short != struct_name {
                if let Some(fields) = self.struct_field_types.get(short) {
                    return Some(fields);
                }
            }
            if let Some(src_root) = self.library_source_root.as_ref() {
                if !self.current_wj_file.as_os_str().is_empty() {
                    if let Some(module_path) =
                        crate::analyzer::type_collector::wj_file_to_module_path(
                            src_root,
                            &self.current_wj_file,
                        )
                    {
                        let key = crate::type_inference::struct_field_registry::qualify_struct_key(
                            &module_path,
                            short,
                        );
                        if let Some(fields) = self.struct_field_types.get(&key) {
                            return Some(fields);
                        }
                    }
                }
            }
            let suffix = format!("::{short}");
            for (key, fields) in &self.struct_field_types {
                if key.ends_with(&suffix) {
                    return Some(fields);
                }
            }
        }
        None
    }

    /// Inside `S { field: [...] }`, returns element type `T` when `field` is `[T; N]`.
    pub(in crate::codegen::rust) fn struct_array_field_element_type(&self) -> Option<Type> {
        if !self.in_struct_literal_field {
            return None;
        }
        let sn = self.current_struct_literal_name.as_deref()?;
        let fnm = self.current_struct_field_name.as_deref()?;
        let fields = self.lookup_struct_field_types(sn)?;
        match fields.get(fnm)? {
            Type::Array(inner, _) => Some((**inner).clone()),
            _ => None,
        }
    }

    // Helper method for expressions that need to be evaluated without &mut self
    pub(crate) fn generate_expression_immut(&self, expr: &Expression) -> String {
        use crate::parser::ast::operators::{BinaryOp, UnaryOp};

        match expr {
            Expression::Literal { value: lit, .. } => self.generate_literal_with_context(lit, expr),
            Expression::Identifier { name, .. } => self.qualify_external_path_identifier(name),
            Expression::Unary { op, operand, .. } => {
                use crate::parser::Literal;
                // IntInference attaches constraints to the Unary for `-n` struct fields (score: -10).
                // Inner Literal would otherwise miss lookup and default to i32.
                if matches!(op, UnaryOp::Neg) {
                    if let Expression::Literal {
                        value: lit @ Literal::Int(_),
                        ..
                    } = &**operand
                    {
                        let s = self.generate_literal_with_context(lit, expr);
                        return format!("-{}", s);
                    }
                }

                // TDD FIX: Skip explicit * deref of &String in string comparisons
                // Problem: In Rust, *(&String) yields &str (not String), breaking &str == &String
                // Solution: Just use the identifier without *, making it &String == &String
                if matches!(op, UnaryOp::Deref) && self.in_string_comparison {
                    if let Some(operand_type) = self.infer_expression_type(operand) {
                        if matches!(operand_type, Type::Reference(inner)
                            if crate::codegen::rust::types::is_windjammer_text_type(&inner))
                        {
                            // Skip the *, just generate the operand (keeping it as &String)
                            return self.generate_expression_immut(operand);
                        }
                    }
                }

                // Strip explicit & when operand is already a reference type
                // (e.g., user writes `&key` but key: str → &str, so &key = &&str)
                if matches!(op, UnaryOp::Ref) {
                    if let Expression::Identifier { name, .. } = &**operand {
                        if self.identifier_already_ref(name) {
                            return self.generate_expression_immut(operand);
                        }
                    }
                }

                let op_str = match op {
                    UnaryOp::Not => "!",
                    UnaryOp::Neg => "-",
                    UnaryOp::Ref => "&",
                    UnaryOp::MutRef => "&mut ",
                    UnaryOp::Deref => "*",
                };
                format!("({}{})", op_str, self.generate_expression_immut(operand))
            }
            Expression::Binary {
                left, op, right, ..
            } => {
                let op_str = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Mod => "%",
                    BinaryOp::Eq => "==",
                    BinaryOp::Ne => "!=",
                    BinaryOp::Lt => "<",
                    BinaryOp::Le => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::Ge => ">=",
                    BinaryOp::And => "&&",
                    BinaryOp::Or => "||",
                    BinaryOp::BitAnd => "&",
                    BinaryOp::BitOr => "|",
                    BinaryOp::BitXor => "^",
                    BinaryOp::Shl => "<<",
                    BinaryOp::Shr => ">>",
                };

                // TDD FIX: Generate comparison without adding incorrect dereferences
                // When comparing &String == &String, both sides are already borrowed - no deref needed!
                // Rust's PartialEq trait handles comparisons correctly for references.
                let mut left_str = self.generate_expression_immut(left);
                let mut right_str = self.generate_expression_immut(right);

                // Auto-deref borrowed bool operands in logical ops (&&, ||).
                // Rust requires `bool`, not `&bool`, for these operators.
                if matches!(op, BinaryOp::And | BinaryOp::Or) {
                    let deref_if_borrowed_bool = |expr: &Expression, s: &str| -> String {
                        if let Expression::Identifier { name, .. } = expr {
                            if self.inferred_borrowed_params.contains(name.as_str())
                                || self.borrowed_iterator_vars.contains(name)
                            {
                                if !s.starts_with('*') {
                                    return format!("*{}", s);
                                }
                            }
                        }
                        s.to_string()
                    };
                    left_str = deref_if_borrowed_bool(left, &left_str);
                    right_str = deref_if_borrowed_bool(right, &right_str);
                }

                // Mixed int/float promotion in const/immutable expressions
                if matches!(
                    op,
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
                ) {
                    self.promote_int_to_float_in_mixed_arithmetic(
                        left,
                        right,
                        &mut left_str,
                        &mut right_str,
                    );
                }

                format!("{} {} {}", left_str, op_str, right_str)
            }
            Expression::FieldAccess { object, field, .. } => {
                format!("{}.{}", self.generate_expression_immut(object), field)
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                if super::rust_stdlib_annotations::is_strip_redundant(method)
                    && arguments.is_empty()
                    && self.expression_produces_str_ref(object)
                {
                    return self.generate_expression_immut(object);
                }

                let obj_str = self.generate_expression_immut(object);

                // TDD FIX: For stdlib methods like HashMap::insert that expect owned String,
                // convert &str parameters to String automatically
                let args_str = arguments
                    .iter()
                    .map(|(_label, arg)| self.generate_expression_immut(arg))
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("{}.{}({})", obj_str, method, args_str)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                let func_str = self.generate_expression_immut(function);

                // TDD FIX: Check if this is a stdlib method that needs usize parameters
                // e.g., Vec::with_capacity(size) where size: int should generate: Vec::with_capacity(size as usize)
                let func_name = match function {
                    Expression::Identifier { name, .. } => Some(name.as_str()),
                    Expression::FieldAccess { object, field, .. } => {
                        if let Expression::Identifier {
                            name: type_name, ..
                        } = &**object
                        {
                            Some(format!("{}::{}", type_name, field).leak() as &str)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                let needs_usize_first_arg = func_name.is_some_and(|name| {
                    let method_part = name.rsplit("::").next().unwrap_or(name);
                    matches!(method_part, "with_capacity" | "reserve")
                });

                // Unified signature resolution for immut path.
                let receiver_type = match function {
                    Expression::FieldAccess { object, .. } => self.infer_type_name(object),
                    _ => None,
                };

                let resolved_sig = func_name.and_then(|name| {
                    crate::codegen::rust::call_signature_resolution::resolve_call_signature(
                        &self.signature_registry,
                        name,
                        receiver_type.as_deref(),
                        arguments.len(),
                        &self.module_alias_map,
                        self.library_source_root.as_ref().and_then(|root| {
                            if self.current_wj_file.as_os_str().is_empty() {
                                None
                            } else {
                                crate::analyzer::type_collector::wj_file_to_module_path(
                                    root,
                                    &self.current_wj_file,
                                )
                                .map(|parts| parts.join("::"))
                            }
                        })
                        .as_deref(),
                    )
                    .filter(|r| {
                        !matches!(
                            r.resolution_method,
                            crate::codegen::rust::call_signature_resolution::ResolutionMethod::ArgCountValidated
                        )
                    })
                });

                let param_types: Option<Vec<Type>> =
                    resolved_sig.as_ref().map(|r| r.sig.param_types.clone());

                let mut arg_strings = Vec::new();
                for (idx, (_label, arg)) in arguments.iter().enumerate() {
                    // Check if this parameter is &str
                    let param_is_str_ref = param_types.as_ref().and_then(|types| types.get(idx)).map(|t| {
                        matches!(t, Type::Reference(inner) if matches!(**inner, Type::Custom(ref name) if name == "str"))
                    }).unwrap_or(false);

                    // Check if argument is a string literal (with or without &)
                    let is_string_literal = matches!(
                        arg,
                        Expression::Literal {
                            value: crate::parser::Literal::String(_),
                            ..
                        }
                    );

                    // Also check if it's &"string"
                    let is_ref_string_literal = if let Expression::Unary {
                        op: crate::parser::UnaryOp::Ref,
                        operand,
                        ..
                    } = arg
                    {
                        matches!(
                            &**operand,
                            Expression::Literal {
                                value: crate::parser::Literal::String(_),
                                ..
                            }
                        )
                    } else {
                        false
                    };

                    // PHASE 2 CALL-SITE OPTIMIZATION: Suppress .to_string() for &str parameters
                    let old_suppress = self.suppress_string_conversion.get();
                    if param_is_str_ref && (is_string_literal || is_ref_string_literal) {
                        self.suppress_string_conversion.set(true);
                    }

                    // Generate the argument string
                    let mut arg_str = self.generate_expression_immut(arg);

                    // Restore suppress flag
                    self.suppress_string_conversion.set(old_suppress);

                    // For first argument to with_capacity/reserve, cast int to usize if it's an identifier
                    if idx == 0 && needs_usize_first_arg {
                        if matches!(arg, Expression::Identifier { .. }) {
                            arg_str = format!("{} as usize", arg_str);
                        }
                    }

                    // Ownership-based string coercion for instance method calls
                    // (Call(FieldAccess) path — same logic as MethodCall path)
                    if let Some(ref r) = resolved_sig {
                        let sig = &r.sig;
                        let sig_param_idx = sig.arg_param_index(idx);
                        if let Some(&ownership) = sig.param_ownership.get(sig_param_idx) {
                            match ownership {
                                crate::analyzer::OwnershipMode::Owned => {
                                    let is_str_lit = matches!(
                                        arg,
                                        Expression::Literal {
                                            value: crate::parser::Literal::String(_),
                                            ..
                                        }
                                    );
                                    if is_str_lit {
                                        let is_explicit_str_ref = sig.param_types.get(sig_param_idx)
                                            .is_some_and(|t| matches!(t, Type::Reference(inner) if
                                                matches!(&**inner, Type::String) ||
                                                matches!(&**inner, Type::Custom(s) if s == "str")
                                            ));
                                        if !is_explicit_str_ref {
                                            arg_str = format!("{}.to_string()", arg_str);
                                        }
                                    }
                                }
                                crate::analyzer::OwnershipMode::Borrowed => {
                                    if is_string_literal {
                                        let param_is_str_ref_explicit = sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                            matches!(t, Type::Reference(inner) if matches!(&**inner, Type::Custom(name) if name == "str"))
                                        });
                                        if !param_is_str_ref_explicit {
                                            let param_is_string = sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                                matches!(t, Type::String) || matches!(t, Type::Custom(ref name) if name == "string")
                                            });
                                            if param_is_string {
                                                arg_str = format!("&{}.to_string()", arg_str);
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    arg_strings.push(arg_str);
                }

                let args_str = arg_strings.join(", ");
                format!("{}({})", func_str, args_str)
            }
            Expression::Index { object, index, .. } => {
                format!(
                    "{}[{}]",
                    self.generate_expression_immut(object),
                    self.generate_expression_immut(index)
                )
            }
            // For complex expressions, just output a placeholder
            // Decorators are primarily documentation/runtime checks
            _ => "true".to_string(),
        }
    }

    /// e.g., stack.item.id → "stack", self.field → "self"
    pub(in crate::codegen::rust) fn extract_root_identifier(
        &self,
        expr: &Expression,
    ) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, .. } => self.extract_root_identifier(object),
            Expression::Index { object, .. } => self.extract_root_identifier(object),
            _ => None,
        }
    }

    /// Borrow an owned `String` field when the callee's parameter is `&str` and `should_add_ref`
    /// missed it (e.g. signature collision / edge cases).
    pub(in crate::codegen::rust) fn ensure_ref_for_owned_string_field_when_callee_expects_str(
        &self,
        method_signature: &Option<crate::analyzer::FunctionSignature>,
        sig_param_idx: usize,
        arg_to_generate: &Expression<'ast>,
        arg_str: String,
        string_literal_converted: bool,
    ) -> String {
        let callee_wants_borrowed_str = |sig: &crate::analyzer::FunctionSignature, idx: usize| {
            matches!(
                crate::codegen::rust::call_signature_resolution::effective_param_ownership(
                    sig, idx
                ),
                OwnershipMode::Borrowed,
            )
        };

        if string_literal_converted {
            return arg_str;
        }
        if method_signature.as_ref().is_some_and(|sig| {
            sig.formal_param_type(sig_param_idx).is_some_and(|t| {
                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    && crate::codegen::rust::types::is_windjammer_text_type(t)
            }) || matches!(
                sig.param_ownership.get(sig_param_idx),
                Some(OwnershipMode::Owned)
            ) || sig.param_types.get(sig_param_idx).is_some_and(|t| {
                matches!(t, Type::String)
                    || matches!(t, Type::Custom(n) if n == "string" || n == "String")
            })
        }) {
            return arg_str;
        }
        if arg_str.starts_with('&') {
            return arg_str;
        }
        // If the argument is already a reference (str_ref_optimized param or
        // inferred borrowed), adding & would create &&str.
        if let Expression::Identifier { name, .. } = arg_to_generate {
            if self.identifier_already_ref(name) {
                return arg_str;
            }
        }
        let wants_str = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::callee_param_is_rust_str_slice(
            method_signature,
            sig_param_idx,
        ) || method_signature
            .as_ref()
            .is_some_and(|sig| callee_wants_borrowed_str(sig, sig_param_idx));
        if !wants_str {
            return arg_str;
        }
        // When callee expects &str and arg produces an owned String, add &.
        // &String auto-derefs to &str. Skip for string literals — they are already
        // &str. After later passes strip .to_string(), bare "lit" would become
        // &"lit" (&&str), so we must also skip "lit".to_string().
        let is_bare_str_literal = arg_str.starts_with('"');
        if is_bare_str_literal {
            return arg_str;
        }
        if arg_str.starts_with('&') {
            return arg_str;
        }
        if self
            .infer_expression_type(arg_to_generate)
            .as_ref()
            .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
        {
            let mut borrowed = arg_str;
            crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(&mut borrowed);
            return borrowed;
        }
        if matches!(
            arg_to_generate,
            Expression::Identifier { .. } | Expression::FieldAccess { .. }
        ) {
            let mut borrowed = arg_str;
            crate::codegen::rust::expression_utilities::apply_shared_borrow_prefix(&mut borrowed);
            return borrowed;
        }
        arg_str
    }

    /// `match` / `if let` on `&enum` binds `&U` for Copy payloads; value contexts need `U`.
    pub(in crate::codegen::rust) fn peel_copy_ref_match_binding_for_value(
        &self,
        expr: &Expression<'ast>,
        generated: &str,
    ) -> String {
        if generated.starts_with('*') {
            return generated.to_string();
        }
        let Some(ty) = self.infer_expression_type(expr) else {
            return generated.to_string();
        };
        let pointee = match &ty {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            _ => return generated.to_string(),
        };
        if !self.is_type_copy(pointee) {
            return generated.to_string();
        }
        if let Expression::Identifier { name, .. } = expr {
            if self.match_arm_bindings.contains(name.as_str()) {
                // HashMap/Option match arms already lower Copy payloads as owned locals.
                return generated.to_string();
            }
            if generated == *name {
                return format!("*{generated}");
            }
        }
        format!("*({generated})")
    }

    /// Back-compat alias for struct literal field emission.
    pub(in crate::codegen::rust) fn peel_copy_ref_binding_for_struct_field(
        &self,
        expr: &Expression<'ast>,
        generated: &str,
    ) -> String {
        self.peel_copy_ref_match_binding_for_value(expr, generated)
    }

    /// After `peel_copy_ref_binding_for_struct_field`, non-Copy `&T` bindings still need `.clone()`
    /// for owned struct fields (e.g. `Vec` from `if let E { clips, .. } = &vec[i]`).
    pub(in crate::codegen::rust) fn clone_non_copy_ref_binding_for_struct_field(
        &self,
        expr: &Expression<'ast>,
        expr_str: &str,
    ) -> String {
        if expr_str.contains(".clone()")
            || expr_str.contains(".to_string()")
            || expr_str.ends_with(".into()")
        {
            return expr_str.to_string();
        }
        let Some(ty) = self.infer_expression_type(expr) else {
            return expr_str.to_string();
        };
        match ty {
            Type::Reference(inner) | Type::MutableReference(inner) => {
                if self.is_type_copy(inner.as_ref()) {
                    expr_str.to_string()
                } else {
                    format!("{}.clone()", expr_str)
                }
            }
            _ => expr_str.to_string(),
        }
    }

    /// Check if match needs .clone() to avoid partial move from self
    pub(in crate::codegen::rust) fn match_needs_clone_for_self_field(
        &self,
        value: &Expression,
        arms: &[crate::parser::MatchArm],
    ) -> bool {
        let is_self_field = if let Expression::FieldAccess { object, .. } = value {
            matches!(&**object, Expression::Identifier { name, .. } if name == "self")
        } else {
            false
        };

        if !is_self_field {
            return false;
        }

        let has_self = self
            .current_function_params
            .iter()
            .any(|p| p.name == "self");

        if !has_self {
            return false;
        }

        arms.iter()
            .any(|arm| pattern_analysis::pattern_extracts_value(&arm.pattern))
    }

    pub(in crate::codegen::rust) fn generate_expression_with_precedence(
        &mut self,
        expr: &Expression<'ast>,
    ) -> String {
        // Wrap expressions in parentheses if they need them for proper precedence
        // when used as the object of a method call or field access
        match expr {
            Expression::Range { .. }
            | Expression::Binary { .. }
            | Expression::Closure { .. }
            | Expression::Unary { .. }
            | Expression::Cast { .. } => {
                // Unary expressions like (*entity).field need parens for correct precedence
                // Without parens: *entity.field means *(entity.field) - WRONG
                // With parens: (*entity).field means dereference then access field - CORRECT
                //
                // Cast expressions like (x as usize).method() need parens because `as` has
                // lower precedence than `.` in Rust:
                // Without parens: x as usize.method() means x as (usize.method()) - WRONG
                // With parens: (x as usize).method() - CORRECT
                format!("({})", self.generate_expression(expr))
            }
            _ => self.generate_expression(expr),
        }
    }

    // PHASE 7: Constant folding - evaluate constant expressions at compile time
    pub(crate) fn generate_expression(&mut self, expr: &Expression<'ast>) -> String {
        // RECURSION GUARD: Check depth before processing expression
        if let Err(e) = self.enter_recursion("generate_expression") {
            eprintln!("{}", e);
            return format!("/* {} */", e);
        }

        // PHASE 7: Try constant folding first
        let folded_expr = constant_folding::try_fold_constant(expr);
        let expr_to_generate = folded_expr.as_ref().unwrap_or(expr);

        let result = self.generate_expression_impl(expr_to_generate);
        self.exit_recursion();
        result
    }

    fn generate_expression_impl(&mut self, expr_to_generate: &Expression<'ast>) -> String {
        match expr_to_generate {
            Expression::Literal { value: lit, .. } => {
                self.generate_literal_with_context(lit, expr_to_generate)
            }
            Expression::Identifier { name, .. } => self.generate_identifier(name, expr_to_generate),
            Expression::Binary {
                left, op, right, ..
            } => self.generate_binary_expression(left, op, right),
            Expression::Unary { op, operand, .. } => self.generate_unary(op, operand),
            Expression::Call {
                function,
                arguments,
                ..
            } => self.generate_call_expression(function, arguments),
            Expression::MethodCall {
                object,
                method,
                type_args,
                arguments,
                ..
            } => self.generate_method_call_expression(object, method, type_args, arguments),
            Expression::FieldAccess { object, field, .. } => {
                self.generate_field_access(object, field, expr_to_generate)
            }
            Expression::StructLiteral { name, fields, .. } => {
                self.generate_struct_literal(name, fields)
            }
            Expression::MapLiteral { pairs, .. } => self.generate_map_literal(pairs),
            Expression::TryOp { expr: inner, .. } => self.generate_try_op(inner),
            Expression::Await { expr: inner, .. } => self.generate_await(inner),
            Expression::ChannelSend { channel, value, .. } => {
                self.generate_channel_send(channel, value)
            }
            Expression::ChannelRecv { channel, .. } => self.generate_channel_recv(channel),
            Expression::Range {
                start,
                end,
                inclusive,
                ..
            } => self.generate_range(start, end, *inclusive),
            Expression::Closure {
                parameters, body, ..
            } => self.generate_closure(parameters, body),
            Expression::Index { object, index, .. } => {
                self.generate_index(object, index, expr_to_generate)
            }
            Expression::Tuple {
                elements: exprs, ..
            } => self.generate_tuple(exprs),
            Expression::Array {
                elements: exprs, ..
            } => self.generate_array(exprs),
            Expression::MacroInvocation {
                is_repeat,
                name,
                args,
                delimiter,
                ..
            } => self.generate_macro_invocation(*is_repeat, name, args, delimiter),
            Expression::Cast { expr, type_, .. } => self.generate_cast(expr, type_),
            Expression::Block {
                statements: stmts,
                is_unsafe,
                ..
            } => self.generate_block_expr(stmts, *is_unsafe),
        }
    }

    /// Whether string literals in this context coerce to owned `String` (match arms, inference).
    #[inline]
    fn should_coerce_string_literal_to_owned(&self) -> bool {
        !self.suppress_string_conversion.get()
            && (self.in_match_arm_needing_string || self.coerce_string_literals_to_owned)
    }

    pub(super) fn generate_literal_with_context(
        &self,
        lit: &Literal,
        expr: &Expression<'ast>,
    ) -> String {
        // WINDJAMMER PHILOSOPHY: Expression-level type inference for literals
        // Int: Check IntInference first (i32, i64, u32, etc.)
        // Float: Check FloatInference (f32, f64)
        match lit {
            Literal::String(_) => {
                let base = crate::codegen::rust::literals::generate_literal(lit);
                if self.should_coerce_string_literal_to_owned() {
                    crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(&base)
                } else {
                    base
                }
            }
            Literal::IntSuffixed(i, suffix) => {
                format!("{}_{}", i, suffix)
            }
            Literal::Int(i) => {
                if let Some(inference) = &self.int_inference {
                    use crate::type_inference::IntType;
                    let inferred = inference.get_int_type(expr);
                    if inferred != IntType::Unknown {
                        let suffix = inferred.rust_suffix();
                        return format!("{}_{}", i, suffix);
                    }
                }
                crate::codegen::rust::literals::generate_literal(lit)
            }
            Literal::Float(f) => {
                // Struct field annotations beat float inference (avoids `100.0_f64` in `f32` slots).
                if self.in_struct_literal_field {
                    if let (Some(struct_name), Some(field_name)) = (
                        &self.current_struct_literal_name,
                        &self.current_struct_field_name,
                    ) {
                        if let Some(field_type) = self
                            .lookup_struct_field_types(struct_name)
                            .and_then(|fields| fields.get(field_name))
                        {
                            if let Some(suffix) =
                                float_type_utilities::try_extract_float_type(field_type)
                            {
                                let s = f.to_string();
                                return if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                                    format!("{}.0_{}", s, suffix)
                                } else {
                                    format!("{}_{}", s, suffix)
                                };
                            }
                        }
                    }
                }

                // Priority 1: Use inference engine results (most accurate)
                if let Some(inference) = &self.float_inference {
                    use crate::type_inference::FloatType;
                    let inferred = inference.get_float_type(expr);

                    let suffix: Option<&str> = match inferred {
                        FloatType::F32 => Some("f32"),
                        FloatType::F64 => Some("f64"),
                        FloatType::Unknown => {
                            // Same resolution order as `generate_literal_context_sensitive`, so
                            // `[f32; 3]` struct fields still get `_f32` when inference is Unknown.
                            let from_assignment =
                                self.assignment_float_target_type.as_ref().and_then(
                                    float_type_utilities::float_literal_suffix_from_assignment_lhs,
                                );
                            let from_struct_field = if let (Some(struct_name), Some(field_name)) = (
                                &self.current_struct_literal_name,
                                &self.current_struct_field_name,
                            ) {
                                self.lookup_struct_field_types(struct_name)
                                    .and_then(|fields| fields.get(field_name))
                                    .map(|ft| {
                                        float_type_utilities::extract_float_type_from_context(ft)
                                    })
                            } else {
                                None
                            };
                            let from_return =
                                self.current_function_return_type.as_ref().map(|rt| {
                                    float_type_utilities::extract_float_type_from_context(rt)
                                });
                            Some(
                                from_assignment
                                    .or(from_struct_field)
                                    .or(from_return)
                                    .unwrap_or("f32"),
                            )
                        }
                    };

                    if let Some(suffix) = suffix {
                        let s = f.to_string();
                        return if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                            format!("{}.0_{}", s, suffix)
                        } else {
                            format!("{}_{}", s, suffix)
                        };
                    }

                    return self.generate_literal_context_sensitive(lit);
                }

                // Priority 2: Fallback to old context-sensitive approach
                self.generate_literal_context_sensitive(lit)
            }
            _ => crate::codegen::rust::literals::generate_literal(lit),
        }
    }

    /// Walks field/index chains to see if codegen traces to `self`.
    ///
    /// Used for `f32`/`f64` classification and E0507 `Option::map` (`self.children.map()` → `.as_ref()`).
    pub(in crate::codegen::rust) fn codegen_expression_traces_to_self(
        &self,
        expr: &Expression,
    ) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == "self")
                    || self.codegen_expression_traces_to_self(object)
            }
            Expression::Index { object, .. } => self.codegen_expression_traces_to_self(object),
            _ => false,
        }
    }

    /// Check if an expression involves borrowing `self` — including method calls on self.
    /// Broader than `codegen_expression_traces_to_self` which only checks field access chains.
    /// Used for self-borrow temporary extraction (E0499 prevention).
    pub(in crate::codegen::rust) fn expression_borrows_self(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == "self",
            Expression::FieldAccess { object, .. } => self.expression_borrows_self(object),
            Expression::Index { object, .. } => self.expression_borrows_self(object),
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.expression_borrows_self(object)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_borrows_self(arg))
            }
            Expression::Call {
                arguments,
                function,
                ..
            } => {
                self.expression_borrows_self(function)
                    || arguments
                        .iter()
                        .any(|(_, arg)| self.expression_borrows_self(arg))
            }
            Expression::Binary { left, right, .. } => {
                self.expression_borrows_self(left) || self.expression_borrows_self(right)
            }
            Expression::Unary { operand, .. } => self.expression_borrows_self(operand),
            _ => false,
        }
    }

    /// Check whether the root of a field-access chain is behind a reference.
    /// Walks up through nested FieldAccess nodes until it finds the root
    /// Identifier, then checks if that variable is a borrowed or match-bound ref.
    pub(in crate::codegen::rust) fn field_access_root_is_behind_reference(
        &self,
        expr: &Expression,
    ) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                self.field_access_root_is_behind_reference(object)
            }
            Expression::Identifier { name, .. } => {
                self.inferred_borrowed_params.contains(name.as_str())
                    || self.borrowed_iterator_vars.contains(name)
                    || self.local_var_types.get(name.as_str()).is_some_and(|t| {
                        matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    })
            }
            _ => false,
        }
    }
}
