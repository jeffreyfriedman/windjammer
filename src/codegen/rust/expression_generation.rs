//! Expression Generation Module
//!
//! Handles generation of Rust code for all expression types:
//! - Literals, identifiers, binary/unary operations
//! - Function and method calls
//! - Field access, index access
//! - Struct/array/map literals
//! - Closures, blocks, match expressions
//! - Cast, try, await, range expressions

use crate::analyzer::*;
use crate::parser::*;

use super::ast_utilities;
use super::constant_folding;
use super::expression_helpers;
use super::expression_utilities;
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
        self.struct_field_types.get(struct_name).or_else(|| {
            struct_name
                .rsplit("::")
                .next()
                .and_then(|short| self.struct_field_types.get(short))
        })
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
                if method == "as_str"
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
                    name == "Vec::with_capacity"
                        || name == "HashMap::with_capacity"
                        || name == "String::with_capacity"
                        || name == "Vec::reserve"
                });

                // Resolve the full method signature for instance method calls
                // (parsed as Call(FieldAccess(object, method_name))).
                // For `sys.register_function("fire")`, the parser produces:
                //   Call { function: FieldAccess { object: Identifier("sys"), field: "register_function" } }
                // We need to resolve `sys`'s type to find `SystemCoverage::register_function` in the registry.
                let instance_method_sig = match function {
                    Expression::FieldAccess { object, field, .. } => {
                        self.infer_type_name(object)
                            .and_then(|tn| {
                                let qualified = format!("{}::{}", tn, field);
                                self.signature_registry.get_signature(&qualified).cloned()
                            })
                    }
                    _ => None,
                };

                // PHASE 2 CALL-SITE OPTIMIZATION: Look up function signature to check for &str parameters
                // If a parameter is &str and we're passing a string literal, pass it directly (no .to_string())
                let param_types: Option<Vec<Type>> = if instance_method_sig.is_some() {
                    instance_method_sig.as_ref().map(|sig| sig.param_types.clone())
                } else {
                    func_name.and_then(|name| {
                        // Try direct lookup first (e.g., "Thing::new")
                        self.signature_registry
                            .get_signature(name)
                            .or_else(|| {
                                // Fallback: Try finding by suffix (e.g., "::new" matches "Thing::new")
                                let method_name =
                                    name.rsplit_once("::").map(|(_, m)| m).unwrap_or(name);
                                self.signature_registry
                                    .find_signature_ending_with(method_name)
                            })
                            .map(|sig| sig.param_types.clone())
                    })
                };

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
                    if let Some(ref sig) = instance_method_sig {
                        let sig_param_idx = if sig.has_self_receiver { idx + 1 } else { idx };
                        if let Some(&ownership) = sig.param_ownership.get(sig_param_idx) {
                            match ownership {
                                crate::analyzer::OwnershipMode::Owned => {
                                    let is_str_lit = matches!(
                                        arg,
                                        Expression::Literal { value: crate::parser::Literal::String(_), .. }
                                    );
                                    if is_str_lit {
                                        let is_explicit_str_ref = sig.param_types.get(sig_param_idx)
                                            .is_some_and(|t| matches!(t, Type::Reference(inner) if
                                                matches!(**inner, Type::String) ||
                                                matches!(**inner, Type::Custom(ref s) if s == "str")
                                            ));
                                        if !is_explicit_str_ref {
                                            arg_str = format!("{}.to_string()", arg_str);
                                        }
                                    }
                                }
                                crate::analyzer::OwnershipMode::Borrowed => {
                                    if is_string_literal {
                                        let param_is_str_ref_explicit = sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                            matches!(t, Type::Reference(inner) if matches!(**inner, Type::Custom(ref name) if name == "str"))
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
    fn extract_root_identifier(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, .. } => self.extract_root_identifier(object),
            Expression::Index { object, .. } => self.extract_root_identifier(object),
            _ => None,
        }
    }

    /// Borrow an owned `String` field when the callee's parameter is `&str` and `should_add_ref`
    /// missed it (e.g. signature collision / edge cases).
    fn ensure_ref_for_owned_string_field_when_callee_expects_str(
        &self,
        method_signature: &Option<crate::analyzer::FunctionSignature>,
        sig_param_idx: usize,
        arg_to_generate: &Expression<'ast>,
        arg_str: String,
        string_literal_converted: bool,
    ) -> String {
        if string_literal_converted {
            return arg_str;
        }
        if arg_str.starts_with('&') {
            return arg_str;
        }
        if !crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::callee_param_is_rust_str_slice(
            method_signature,
            sig_param_idx,
        ) {
            return arg_str;
        }
        if let Expression::FieldAccess { .. } = arg_to_generate {
            if self
                .infer_expression_type(arg_to_generate)
                .as_ref()
                .is_some_and(|t| crate::codegen::rust::types::is_windjammer_text_type(t))
            {
                return format!("&{}", arg_str);
            }
        }
        arg_str
    }

    /// `if let` / `match` on `&enum` binds `&U` for Copy fields; struct literals need `U` (E0308).
    pub(in crate::codegen::rust) fn peel_copy_ref_binding_for_struct_field(
        &self,
        expr: &Expression<'ast>,
        generated: &str,
    ) -> String {
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
        format!("*({generated})")
    }

    /// After `peel_copy_ref_binding_for_struct_field`, non-Copy `&T` bindings still need `.clone()`
    /// for owned struct fields (e.g. `Vec` from `if let E { clips, .. } = &vec[i]`).
    pub(in crate::codegen::rust) fn clone_non_copy_ref_binding_for_struct_field(
        &self,
        expr: &Expression<'ast>,
        expr_str: &str,
    ) -> String {
        if expr_str.contains(".clone()") || expr_str.contains(".to_string()") {
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

    pub(in crate::codegen::rust) fn generate_expression_with_precedence(&mut self, expr: &Expression<'ast>) -> String {
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
            Expression::Identifier { name, .. } => {
                self.generate_identifier(name, expr_to_generate)
            }
            Expression::Binary {
                left, op, right, ..
            } => self.generate_binary_expression(left, op, right),
            Expression::Unary { op, operand, .. } => self.generate_unary(op, operand),
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // Extract function name for signature lookup
                let func_name = ast_utilities::extract_function_name(function);

                // THE WINDJAMMER WAY: User-defined functions always take priority
                // over built-in name mappings. If the user defines a function with
                // the same name as a test macro or runtime function (e.g., their own
                // `assert_approx`), their definition wins. We check the signature
                // registry: if the function exists and is NOT extern, it's user-defined.
                //
                // EXCEPTION: print/println/eprintln/eprint always convert to macros
                // (Rust requires them to be macros, not functions)

                // Try print/println/eprintln macro conversion FIRST (before user-defined check)
                if let Some(print_macro) = self.try_generate_print_macro(&func_name, arguments) {
                    return print_macro;
                }

                let is_user_defined = self
                    .signature_registry
                    .get_signature(&func_name)
                    .map(|sig| !sig.is_extern)
                    .unwrap_or(false);

                if !is_user_defined {
                    // Try test macro conversion first
                    if let Some(macro_call) = self.try_generate_test_macro(&func_name, arguments) {
                        return macro_call;
                    }

                    // Try test runtime function qualification
                    if let Some(qualified_call) = self.try_qualify_test_function(&func_name, arguments) {
                        return qualified_call;
                    }
                }

                // Special case: convert assert() to assert!()
                if func_name == "assert" {
                    let args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| self.generate_expression(arg))
                        .collect();
                    return format!("assert!({})", args.join(", "));
                }

                // TDD FIX: Call(FieldAccess) → method call WITH SIGNATURE LOOKUP
                // When the parser produces Call { function: FieldAccess { object, field }, args }
                // instead of MethodCall { object, method, args }, we need to:
                // 1. Handle it as a method call (not function call)
                // 2. Do signature lookup to get parameter ownership info
                // 3. Apply correct ownership conversions (& vs .clone() etc.)
                //
                // This was the AUTO-CLONE BUG: method calls skipped signature lookup!
                if let Expression::FieldAccess {
                    object: call_obj,
                    field: call_method,
                    ..
                } = &**function
                {
                    // DOUBLE-CLONE FIX: When the method is .clone(), suppress auto-clone on
                    // the object to prevent .clone().clone(). Same as MethodCall handler.
                    let prev_explicit_clone = self.in_explicit_clone_call;
                    if call_method == "clone" {
                        self.in_explicit_clone_call = true;
                    }
                    let mut obj_str = self.generate_expression(call_obj);
                    self.in_explicit_clone_call = prev_explicit_clone;
                    // DOUBLE-CLONE SAFETY NET: Strip redundant auto-clone from object
                    if call_method == "clone" && obj_str.ends_with(".clone()") {
                        obj_str = obj_str[..obj_str.len() - 8].to_string();
                    }

                    // TDD FIX: Lookup method signature for ownership inference
                    // Prefer `Type::method` (matches MethodCall path) so `HashMap::get` wins over wrong `get`.
                    let type_name = self.infer_type_name(call_obj);
                    let method_signature = type_name
                        .as_ref()
                        .map(|tn| format!("{}::{}", tn, call_method))
                        .and_then(|q| {
                            self.signature_registry.get_signature(&q).cloned()
                        })
                        .or_else(|| {
                            // When `call_obj` is a module identifier (e.g., `draw` in `draw::draw_text`),
                            // infer_type_name returns None. Try module-qualified lookup directly.
                            if let Expression::Identifier { name: mod_name, .. } = &**call_obj {
                                let qualified = format!("{}::{}", mod_name, call_method);
                                if let Some(sig) = self.signature_registry.get_signature(&qualified)
                                {
                                    return Some(sig.clone());
                                }
                            }
                            if super::stdlib_method_traits::is_common_stdlib_method(call_method) {
                                None
                            } else {
                                let bare_sig =
                                    self.signature_registry.get_signature(call_method).cloned();
                                bare_sig
                            }
                        });

                    // Generate arguments with ownership awareness (same logic as regular Call)
                    let args: Vec<String> = if let Some(ref sig) = method_signature {
                        arguments
                            .iter()
                            .enumerate()
                            .flat_map(|(i, (_label, arg))| {
                                let arg_to_generate =
                                    expression_utilities::strip_unary_ref_for_collection_key_arg(call_method, i, arg);
                                let prev_coerce_string_literals = self.coerce_string_literals_to_owned;
                                self.coerce_string_literals_to_owned = false;
                                let prev_match_arm_str = self.in_match_arm_needing_string;
                                self.in_match_arm_needing_string = false;
                                let mut arg_str = self.generate_expression(arg_to_generate);
                                self.coerce_string_literals_to_owned = prev_coerce_string_literals;
                                self.in_match_arm_needing_string = prev_match_arm_str;

                                // Apply ownership conversion based on signature
                                let sig_param_idx = if sig.has_self_receiver {
                                    i + 1
                                } else {
                                    i
                                };
                                if let Some(&ownership) = sig.param_ownership.get(sig_param_idx) {
                                    match ownership {
                                        OwnershipMode::Borrowed => {
                                            // PHASE 1: Generate &String parameters for correctness
                                            let is_string_literal = matches!(
                                                arg_to_generate,
                                                Expression::Literal {
                                                    value: Literal::String(_),
                                                    ..
                                                }
                                            );
                                            let is_user_closure_param =
                                                if let Expression::Identifier { name, .. } =
                                                    arg_to_generate
                                                {
                                                    self.in_user_written_closure
                                                        && self.user_closure_params.contains(name)
                                                } else {
                                                    false
                                                };

                                            let mut string_literal_converted_here = false;

                                            // PHASE 2: String literals need conversion for &String parameters (but not &str!)
                                            if is_string_literal {
                                                // Check if parameter is explicitly &str
                                                let param_is_str_ref = sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                                    matches!(t, Type::Reference(inner) if matches!(**inner, Type::Custom(ref name) if name == "str"))
                                                });

                                                if param_is_str_ref {
                                                    // Parameter is &str - pass literal directly (already a &str)
                                                    // No conversion needed!
                                                } else {
                                                    // Parameter is Type::String (becomes &String in Rust)
                                                    let param_is_string = sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                                        matches!(t, Type::String) || matches!(t, Type::Custom(ref name) if name == "string")
                                                    });
                                                    if param_is_string {
                                                        // Parameter is &String - need conversion
                                                        arg_str = format!("&{}.to_string()", arg_str);
                                                        string_literal_converted_here = true;
                                                    }
                                                }
                                            } else if !is_user_closure_param {
                                                let should_ref = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                                                    arg_to_generate,
                                                    &arg_str,
                                                    call_method.as_str(),
                                                    i,
                                                    &method_signature,
                                                    &self.usize_variables,
                                                    &self.current_function_params,
                                                    &self.borrowed_iterator_vars,
                                                    &self.inferred_borrowed_params,
                                                    arguments.len(),
                                                    type_name.as_deref(),
                                                    Some(&self.local_var_types),
                                                    Some(&self.stdlib_method_signatures),
                                                    Some(&self.method_signatures_by_type),
                                                    &self.match_arm_bindings, // TDD FIX: E0308 fix
                                                );
                                                if should_ref {
                                                    arg_str = format!("&{}", arg_str);
                                                }
                                            }

                                            arg_str = self.ensure_ref_for_owned_string_field_when_callee_expects_str(
                                                &method_signature,
                                                sig_param_idx,
                                                arg_to_generate,
                                                arg_str,
                                                string_literal_converted_here,
                                            );
                                        }
                                        OwnershipMode::MutBorrowed => {
                                            let is_already_mut_ref =
                                                if let Expression::Identifier { name, .. } = arg_to_generate {
                                                    let explicit_mut_ref = self.current_function_params.iter().any(|param| {
                                                        param.name == *name
                                                            && matches!(&param.type_, crate::parser::Type::MutableReference(_))
                                                    });
                                                    let inferred_mut_ref = self.inferred_mut_borrowed_params.contains(name.as_str());
                                                    explicit_mut_ref || inferred_mut_ref
                                                } else {
                                                    false
                                                };
                                            if !expression_helpers::is_reference_expression(arg_to_generate)
                                                && !is_already_mut_ref
                                            {
                                                let mut mut_arg_str = if arg_str.ends_with(".clone()") {
                                                    arg_str[..arg_str.len() - 8].to_string()
                                                } else {
                                                    arg_str
                                                };
                                                if mut_arg_str.starts_with("&") && !mut_arg_str.starts_with("&mut ") {
                                                    mut_arg_str = mut_arg_str[1..].to_string();
                                                }
                                                arg_str = format!("&mut {}", mut_arg_str);
                                            }
                                        }
                                        OwnershipMode::Owned => {
                                            // String literal coercion: "foo" → "foo".to_string()
                                            // when param expects owned String
                                            let is_str_lit = matches!(
                                                arg_to_generate,
                                                Expression::Literal { value: Literal::String(_), .. }
                                            );
                                            // Also handle &str parameters being passed to methods expecting String
                                            let is_str_param = matches!(
                                                arg_to_generate,
                                                Expression::Identifier { name, .. }
                                                    if self.current_function_params.iter().any(|p| {
                                                        &p.name == name && matches!(
                                                            &p.type_,
                                                            Type::Reference(inner) if matches!(**inner, Type::Custom(ref s) if s == "str")
                                                        )
                                                    })
                                            );
                                            if is_str_lit || is_str_param {
                                                let is_explicit_str_ref = sig.param_types.get(sig_param_idx)
                                                    .is_some_and(|t| matches!(t, Type::Reference(inner) if
                                                        matches!(**inner, Type::String) ||
                                                        matches!(**inner, Type::Custom(ref s) if s == "str")
                                                    ));
                                                if !is_explicit_str_ref {
                                                    arg_str = format!("{}.to_string()", arg_str);
                                                }
                                            }
                                            // Destination wants owned - add .clone() for borrowed sources
                                            if let Expression::FieldAccess {
                                                object: field_obj,
                                                ..
                                            } = arg_to_generate
                                            {
                                                if let Expression::Identifier { name, .. } =
                                                    &**field_obj
                                                {
                                                    let is_borrowed =
                                                        self.borrowed_iterator_vars.contains(name)
                                                            || self
                                                                .inferred_borrowed_params
                                                                .contains(name);
                                                    if is_borrowed && !arg_str.ends_with(".clone()")
                                                    {
                                                        let is_copy = self
                                                            .infer_expression_type(arg_to_generate)
                                                            .as_ref()
                                                            .is_some_and(|t| self.is_type_copy(t));
                                                        if !is_copy {
                                                            arg_str =
                                                                format!("{}.clone()", arg_str);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // AUTO-CAST int → float: Call(FieldAccess) path
                                // Skip when signature has a collision (different types with same name).
                                let qualified_key = type_name.as_ref()
                                    .map(|tn| format!("{}::{}", tn, call_method));
                                let has_collision = qualified_key.as_ref()
                                    .is_some_and(|k| self.signature_registry.has_collision(k))
                                    || self.signature_registry.has_collision(call_method);
                                if !has_collision {
                                    if let Some(param_ty) = sig.param_types.get(sig_param_idx) {
                                        let param_is_f32 = matches!(param_ty, Type::Custom(n) if n == "f32");
                                        let param_is_f64 = matches!(param_ty, Type::Custom(n) if n == "f64");
                                        if param_is_f32 || param_is_f64 {
                                            let arg_ty = self.infer_expression_type(arg);
                                            let arg_is_int = arg_ty.as_ref().is_some_and(|t| {
                                                matches!(t, Type::Int)
                                                    || matches!(t, Type::Custom(n) if crate::type_classification::is_integer_type(n))
                                            });
                                            if arg_is_int && !arg_str.contains(" as f32") && !arg_str.contains(" as f64") {
                                                let target = if param_is_f32 { "f32" } else { "f64" };
                                                arg_str = if arg_str.contains(' ') || matches!(arg, Expression::Binary { .. }) {
                                                    format!("({}) as {}", arg_str, target)
                                                } else {
                                                    format!("{} as {}", arg_str, target)
                                                };
                                            }
                                        }
                                    }
                                }

                                vec![arg_str]
                            })
                            .collect()
                    } else {
                        // No signature: still apply map-key strip + stdlib `should_add_ref` (parser uses Call+FieldAccess)
                        // Try to find signature by qualified or simple method name for string coercion.
                        // CRITICAL: For common stdlib methods (get, remove, contains, etc.),
                        // do NOT fall back to unqualified lookup — it can match the WRONG
                        // user-defined method (e.g., ComponentArray::get when we want
                        // HashMap::get), causing incorrect auto-ref/auto-clone behavior.
                        // This mirrors the guard in the MethodCall handler.
                        let fallback_sig = type_name
                            .as_ref()
                            .map(|tn| format!("{}::{}", tn, call_method))
                            .and_then(|q| self.signature_registry.get_signature(&q).cloned())
                            .or_else(|| {
                                if super::stdlib_method_traits::is_common_stdlib_method(call_method)
                                {
                                    None
                                } else {
                                    self.signature_registry.get_signature(call_method).cloned()
                                }
                            });
                        arguments
                            .iter()
                            .enumerate()
                            .map(|(i, (_label, arg))| {
                                let arg_to_generate =
                                    expression_utilities::strip_unary_ref_for_collection_key_arg(call_method, i, arg);
                                let prev_coerce_string_literals = self.coerce_string_literals_to_owned;
                                self.coerce_string_literals_to_owned = false;
                                let prev_match_arm_str = self.in_match_arm_needing_string;
                                self.in_match_arm_needing_string = false;
                                let mut arg_str = self.generate_expression(arg_to_generate);
                                self.coerce_string_literals_to_owned = prev_coerce_string_literals;
                                self.in_match_arm_needing_string = prev_match_arm_str;

                                // Check if this argument needs .to_string() conversion
                                // This handles both string literals AND &str parameters
                                let is_string_literal = matches!(
                                    arg_to_generate,
                                    Expression::Literal { value: Literal::String(_), .. }
                                );
                                let is_str_param = matches!(
                                    arg_to_generate,
                                    Expression::Identifier { name, .. }
                                        if self.inferred_borrowed_params.contains(name)
                                            || self.current_function_params.iter().any(|p| {
                                                &p.name == name && matches!(
                                                    &p.type_,
                                                    Type::Reference(inner) if matches!(**inner, Type::Custom(ref s) if s == "str")
                                                )
                                            })
                                );
                                if is_string_literal || is_str_param {
                                    let needs_to_string = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(
                                        i,
                                        call_method,
                                        &fallback_sig,
                                    );
                                    if needs_to_string {
                                        arg_str = format!("{}.to_string()", arg_str);
                                    }
                                }

                                let should_ref =
                                    crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                                        arg_to_generate,
                                        &arg_str,
                                        call_method.as_str(),
                                        i,
                                        &fallback_sig,
                                        &self.usize_variables,
                                        &self.current_function_params,
                                        &self.borrowed_iterator_vars,
                                        &self.inferred_borrowed_params,
                                        arguments.len(),
                                        type_name.as_deref(),
                                        Some(&self.local_var_types),
                                        Some(&self.stdlib_method_signatures),
                                        Some(&self.method_signatures_by_type),
                                        &self.match_arm_bindings, // TDD FIX: E0308 fix
                                    );
                                if should_ref {
                                    arg_str = format!("&{}", arg_str);
                                }

                                let string_literal_converted_here = (is_string_literal || is_str_param)
                                    && arg_str.ends_with(".to_string()");
                                if let Some(fb_idx) = fallback_sig.as_ref().map(|s| {
                                    if s.has_self_receiver {
                                        i + 1
                                    } else {
                                        i
                                    }
                                }) {
                                    arg_str =
                                        self.ensure_ref_for_owned_string_field_when_callee_expects_str(
                                            &fallback_sig,
                                            fb_idx,
                                            arg_to_generate,
                                            arg_str,
                                            string_literal_converted_here,
                                        );
                                }
                                arg_str
                            })
                            .collect()
                    };

                    let call_str = format!("{}.{}({})", obj_str, call_method, args.join(", "));

                    let is_extern_call = method_signature.as_ref().is_some_and(|sig| sig.is_extern)
                        || self
                            .signature_registry
                            .get_signature(call_method)
                            .is_some_and(|sig| sig.is_extern)
                        || self.extern_function_names.contains(call_method);

                    return if is_extern_call && !self.in_unsafe_block {
                        format!("(unsafe {{ {} }})", call_str)
                    } else {
                        call_str
                    };
                }

                let mut func_str = self.generate_expression(function);

                // Windjammer stdlib type mapping: Map::method → HashMap::method
                if func_str.starts_with("Map::") {
                    func_str = func_str.replacen("Map::", "HashMap::", 1);
                }

                // E0282 turbofish: Vec::new() / HashSet::new() → Vec::<T>::new() / HashSet::<T>::new()
                // when the function return type provides the element type.
                // Skip when suppress_collection_turbofish is set (let binding already has type ascription).
                if arguments.is_empty() && !self.suppress_collection_turbofish {
                    if func_str == "Vec::new" {
                        if let Some(Type::Vec(inner)) = &self.current_function_return_type {
                            func_str = format!("Vec::<{}>::new", self.type_to_rust(inner));
                        }
                    } else if func_str == "HashSet::new" {
                        if let Some(Type::Parameterized(base, args)) =
                            &self.current_function_return_type
                        {
                            if base == "HashSet" && args.len() == 1 {
                                func_str =
                                    format!("HashSet::<{}>::new", self.type_to_rust(&args[0]));
                            }
                        }
                    } else if func_str == "HashMap::new" {
                        if let Some(Type::Parameterized(base, args)) =
                            &self.current_function_return_type
                        {
                            if base == "HashMap" && args.len() == 2 {
                                func_str = format!(
                                    "HashMap::<{}, {}>::new",
                                    self.type_to_rust(&args[0]),
                                    self.type_to_rust(&args[1])
                                );
                            }
                        }
                    }
                }

                // In an impl block, bare function calls to sibling methods need qualified dispatch.
                // Instance methods (take self) → self.method(args)
                // Static methods → Self::method(args)
                if self.in_impl_block
                    && !func_name.contains("::")
                    && self.current_impl_methods.contains(&func_name)
                {
                    if self.current_impl_instance_methods.contains(&func_name) {
                        func_str = format!("self.{}", func_str);
                    } else {
                        func_str = format!("Self::{}", func_str);
                    }
                }

                // E0282 turbofish: Some(expr) → Some::<T>(expr)
                // Only needed when the type parameter is truly ambiguous
                // (e.g. numeric literals outside a typed context). In return
                // position or when the inner type involves references/structs,
                // Rust infers the type from the function signature.
                if func_str == "Some" && arguments.len() == 1 {
                    if let Some(Type::Option(inner)) = &self.current_function_return_type {
                        let inner_rust = self.type_to_rust(inner);
                        let is_ambiguous_primitive = matches!(
                            inner.as_ref(),
                            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool
                        );
                        if is_ambiguous_primitive {
                            func_str = format!("Some::<{}>", inner_rust);
                        }
                    }
                }

                // WINDJAMMER PHILOSOPHY: Some/Ok/Err with string literals need .to_string()
                // Some("literal") -> Some("literal".to_string())
                // Ok("literal") -> Ok("literal".to_string())
                // Err("literal") -> Err("literal".to_string())
                // Also: Some(borrowed_iterator_var) -> Some(borrowed_iterator_var.clone())

                // TDD FIX (Bug #2): Detect ALL enum constructors AND tuple struct constructors
                // Pattern: Some/Ok/Err, Module::Variant, or TupleStruct(args)
                let is_std_enum = matches!(func_name.as_str(), "Some" | "Ok" | "Err");
                let is_custom_enum = func_name.contains("::") && {
                    let parts: Vec<&str> = func_name.split("::").collect();
                    parts.len() == 2
                        && parts[0].chars().next().is_some_and(|c| c.is_uppercase())
                        && parts[1].chars().next().is_some_and(|c| c.is_uppercase())
                };
                // Tuple struct constructors: Point(x, y), Id(42)
                // Uppercase name without :: that is a known tuple struct
                let is_tuple_struct_constructor = !is_std_enum
                    && !is_custom_enum
                    && !func_name.contains("::")
                    && func_name.chars().next().is_some_and(|c| c.is_uppercase())
                    && self.tuple_struct_names.contains(&func_name);

                if is_std_enum || is_custom_enum || is_tuple_struct_constructor {
                    // Enum variant constructors need owned values (Some(T), Ok(T), Err(E)).
                    // Set owned context so index expressions use .clone() instead of &,
                    // BUT only for arguments that aren't already explicit references.
                    let prev_owned_context = self.in_owned_value_context;
                    let generated_args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| {
                            let is_explicit_ref = matches!(
                                arg,
                                Expression::Unary {
                                    op: crate::parser::UnaryOp::Ref
                                        | crate::parser::UnaryOp::MutRef,
                                    ..
                                }
                            );
                            if !is_explicit_ref {
                                self.in_owned_value_context = true;
                            }
                            let result = self.generate_expression(arg);
                            self.in_owned_value_context = prev_owned_context;
                            result
                        })
                        .collect();

                    let has_format_arg = generated_args
                        .iter()
                        .any(|arg_str| arg_str.contains("format!("));

                    if has_format_arg {
                        // Extract format!() macros to temp variables
                        let mut temp_decls = String::new();
                        let mut temp_counter = 0;
                        let fixed_args: Vec<String> = generated_args
                            .iter()
                            .map(|arg_str| {
                                if arg_str.starts_with("format!(")
                                    || arg_str.starts_with("&format!(")
                                {
                                    // Strip leading & if present
                                    let format_expr = if arg_str.starts_with("&") {
                                        arg_str.strip_prefix("&").unwrap()
                                    } else {
                                        arg_str
                                    };
                                    // Extract to temp var
                                    let temp_name = format!("_temp{}", temp_counter);
                                    temp_counter += 1;
                                    temp_decls.push_str(&format!(
                                        "let {} = {}; ",
                                        temp_name, format_expr
                                    ));

                                    // TDD FIX: Don't add & for owned parameters
                                    // Err(format!(...)) should be Err(_temp0), not Err(&_temp0)
                                    // Original arg didn't have &, so pass owned value
                                    if arg_str.starts_with("&") {
                                        format!("&{}", temp_name)
                                    } else {
                                        temp_name
                                    }
                                } else {
                                    arg_str.clone()
                                }
                            })
                            .collect();

                        return format!(
                            "{{ {}{}({}) }}",
                            temp_decls,
                            func_str,
                            fixed_args.join(", ")
                        );
                    }

                    let args: Vec<String> = generated_args
                        .iter()
                        .enumerate()
                        .map(|(i, arg_str)| {
                            // Get the original argument expression for type checking
                            let arg = &arguments[i].1;
                            let result = arg_str.clone();

                            // Auto-convert string literals to String for Option/Result wrappers
                            if matches!(
                                arg,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            ) {
                                format!("{}.to_string()", result)
                            } else if let Expression::Identifier { name, .. } = arg {
                                // BUGFIX: Don't clone if function returns Option<&T>, Option<&mut T>, or Result<&T, E>
                                // When returning Option<&Squad>, Some(squad) should NOT become Some(squad.clone())

                                // Check if return type is Option<&T> or Option<&mut T> (reference inside)
                                let returns_option_ref = match &self.current_function_return_type {
                                    Some(Type::Option(inner_type)) => {
                                        matches!(
                                            **inner_type,
                                            Type::Reference(_) | Type::MutableReference(_)
                                        )
                                    }
                                    _ => false,
                                };

                                // Check if return type is Result<&T, E> or Result<&mut T, E>
                                let returns_result_ref = match &self.current_function_return_type {
                                    Some(Type::Result(ok_type, _err_type)) => {
                                        matches!(
                                            **ok_type,
                                            Type::Reference(_) | Type::MutableReference(_)
                                        )
                                    }
                                    _ => false,
                                };

                                // AUTO-CONVERT: Borrowed variables in enum constructors need
                                // ownership conversion since the wrapper takes ownership.
                                // &str params → .to_string(), other borrowed → .clone()
                                // UNLESS returning Option<&T>, Result<&T, E>, etc.
                                if !returns_option_ref
                                    && !returns_result_ref
                                    && !result.ends_with(".clone()")
                                    && !result.ends_with(".to_string()")
                                    && !result.trim_start().starts_with('*')
                                {
                                    if self.str_ref_optimized_params.contains(name.as_str()) {
                                        format!("{}.to_string()", result)
                                    } else if self.borrowed_iterator_vars.contains(name)
                                        || self.inferred_borrowed_params.contains(name.as_str())
                                    {
                                        format!("{}.clone()", result)
                                    } else {
                                        result
                                    }
                                } else {
                                    result
                                }
                            } else {
                                result
                            }
                        })
                        .collect();
                    return format!("{}({})", func_str, args.join(", "));
                }

                // Look up signature and clone it to avoid borrow conflicts
                // THE WINDJAMMER WAY: Try qualified name first, then simple name
                // e.g., "Sound::new" -> try "Sound::new", then "new"

                // TDD FIX: Function pointer signature extraction
                // When calling a function pointer parameter (e.g., has_item(arg1, arg2)),
                // extract the signature from the parameter's type instead of the registry
                let mut signature = if let Some(param) = self
                    .current_function_params
                    .iter()
                    .find(|p| p.name == func_name)
                {
                    // Check if this parameter is a function pointer
                    if let Type::FunctionPointer {
                        params,
                        return_type,
                    } = &param.type_
                    {
                        // TDD FIX: Build signature from function pointer type
                        // CRITICAL: Match the conversion logic in types.rs type_to_rust()!
                        // fn(string, i32) in Windjammer → fn(&String, i32) in Rust
                        //
                        // Conversion rules (from types.rs lines 148-160):
                        // - Type::String → "&String" → Borrowed
                        // - Type::Custom("string") → "&String" → Borrowed
                        // - Type::Reference(_) → "&T" → Borrowed
                        // - Copy types (Int, Bool, etc.) → owned → Owned
                        // - Everything else → as-is (keep explicit types)
                        let param_ownership: Vec<OwnershipMode> = params
                            .iter()
                            .map(|ty| {
                                match ty {
                                    // Idiomatic Windjammer: string parameters are borrowed (types.rs:151)
                                    Type::String => OwnershipMode::Borrowed,
                                    Type::Custom(name) if name == "string" => {
                                        OwnershipMode::Borrowed
                                    }
                                    // Explicit references - borrowed (types.rs:154)
                                    Type::Reference(_) | Type::MutableReference(_) => {
                                        OwnershipMode::Borrowed
                                    }
                                    // Copy types - owned (types.rs:156-157)
                                    Type::Int
                                    | Type::Int32
                                    | Type::Uint
                                    | Type::Float
                                    | Type::Bool => OwnershipMode::Owned,
                                    Type::Custom(name)
                                        if matches!(
                                            name.as_str(),
                                            "i32"
                                                | "i64"
                                                | "u32"
                                                | "u64"
                                                | "f32"
                                                | "f64"
                                                | "bool"
                                                | "char"
                                                | "usize"
                                                | "isize"
                                        ) =>
                                    {
                                        OwnershipMode::Owned
                                    }
                                    // Everything else - keep as-is (types.rs:159)
                                    // For non-Copy custom types, default is as-is, which means Owned in this context
                                    // (the analyzer will have determined the correct type already)
                                    _ => OwnershipMode::Owned,
                                }
                            })
                            .collect();

                        Some(crate::analyzer::FunctionSignature {
                            name: func_name.clone(),
                            param_types: params.clone(),
                            param_ownership,
                            return_type: return_type.as_ref().map(|t| (**t).clone()),
                            return_ownership: OwnershipMode::Owned, // Functions return owned by default
                            has_self_receiver: false,
                            is_extern: false,
                        })
                    } else {
                        // Not a function pointer - try registry
                        self.signature_registry.get_signature(&func_name).cloned()
                    }
                } else {
                    // Not a parameter - try registry lookup
                    let direct = self.signature_registry.get_signature(&func_name).cloned();
                    direct.or_else(|| {
                        if let Some(pos) = func_name.rfind("::") {
                            let qualifier = &func_name[..pos];
                            let simple_name = &func_name[pos + 2..];
                            let is_type_qualifier =
                                qualifier.chars().next().is_some_and(|c| c.is_uppercase());
                            if is_type_qualifier {
                                self.signature_registry.get_signature(simple_name).cloned()
                            } else {
                                // For module-qualified calls (e.g., draw::draw_text),
                                // try progressively shorter qualified names.
                                // Do NOT fall back to simple name - it may collide
                                // with a different module's function with the same name.
                                let parts: Vec<&str> = func_name.split("::").collect();
                                let mut found = None;
                                for start in (0..parts.len().saturating_sub(1)).rev() {
                                    let candidate = parts[start..].join("::");
                                    if let Some(sig) =
                                        self.signature_registry.get_signature(&candidate)
                                    {
                                        found = Some(sig.clone());
                                        break;
                                    }
                                }
                                found
                            }
                        } else {
                            None
                        }
                    })
                };

                // For module-qualified calls (e.g., gpu::load_compute_shader_from_file),
                // the signature lookup above may fail. Try resolving through module aliases
                // first (e.g., `use crate::ffi::gpu_safe as gpu` → try gpu_safe::func),
                // then fall back to the simple name.
                let mut signature_from_simple_fallback = false;
                if signature.is_none() && func_name.contains("::") {
                    let qualifier = func_name.split("::").next().unwrap_or("");
                    let simple = func_name.rsplit("::").next().unwrap_or(&func_name);

                    // Try resolving through module alias map first
                    if let Some(original_module) = self.module_alias_map.get(qualifier) {
                        let resolved_name = format!("{}::{}", original_module, simple);
                        if let Some(resolved_sig) =
                            self.signature_registry.get_signature(&resolved_name)
                        {
                            signature = Some(resolved_sig.clone());
                        }
                    }

                    // If alias resolution didn't work, try simple-name fallback
                    // with arg count validation to avoid name collisions.
                    if signature.is_none() {
                        if let Some(found) = self.signature_registry
                            .find_signature_by_name_and_arg_count(simple, arguments.len())
                        {
                            signature = Some(found.clone());
                            signature_from_simple_fallback = true;
                        }
                    }

                }

                // Check if this is an extern function call for unsafe wrapping + FFI str handling.
                // TDD FIX: When a signature was found via simple-name fallback for a
                // module-qualified call (e.g. vnode_ffi::vnode_element), suppress extern
                // detection ONLY when the signature is NOT explicitly extern. If the
                // signature has is_extern=true, the function really is extern (e.g.
                // input::input_is_key_pressed) and must be wrapped in unsafe.
                let is_extern_call = if signature_from_simple_fallback && func_name.contains("::") {
                    signature.as_ref().is_some_and(|sig| sig.is_extern)
                } else if let Some(ref sig) = signature {
                    sig.is_extern
                } else {
                    let simple = func_name.rsplit("::").next().unwrap_or(&func_name);
                    self.extern_function_names.contains(simple)
                };

                let args: Vec<String> = arguments
                    .iter()
                    .enumerate()
                    .flat_map(|(i, (_label, arg))| {
                        // CRITICAL: Reset in_field_access_object for argument generation.
                        // Arguments are independent expressions, NOT part of a field/method/index chain.
                        // Without this, `process_property(prop.name, prop.value).as_str()` would
                        // leak in_field_access_object from the MethodCall handler into prop.name/prop.value,
                        // suppressing necessary .clone() calls.
                        let prev_field_access_obj = self.in_field_access_object;
                        self.in_field_access_object = false;

                        // TDD FIX: Set call argument context to suppress premature .clone()
                        // The FieldAccess handler normally adds .clone() for borrowed iterator vars,
                        // but in call arguments, we need to let the ownership check below decide
                        let prev_in_call_arg = self.in_call_argument_generation;
                        self.in_call_argument_generation = true;

                        // Return/match contexts set `coerce_string_literals_to_owned` and
                        // `in_match_arm_needing_string` for the outer expression; nested call
                        // arguments must use only parameter-type conversion (below), not context
                        // coercion — avoids `"x".to_string().to_string()` and wrong `.to_string()`
                        // on &str params, and prevents format!("...".to_string(), ...) in match arms.
                        let prev_coerce_string_literals = self.coerce_string_literals_to_owned;
                        self.coerce_string_literals_to_owned = false;
                        let prev_match_arm_str = self.in_match_arm_needing_string;
                        self.in_match_arm_needing_string = false;
                        let mut arg_str = self.generate_expression(arg);
                        self.coerce_string_literals_to_owned = prev_coerce_string_literals;
                        self.in_match_arm_needing_string = prev_match_arm_str;

                        self.in_call_argument_generation = prev_in_call_arg;
                        self.in_field_access_object = prev_field_access_obj;

                        // TDD FIX: Cast int arguments to usize for stdlib methods
                        // Vec::with_capacity(size) where size: int → Vec::with_capacity(size as usize)
                        // Vec::with_capacity(10) where 10: int literal → Vec::with_capacity(10_usize)
                        if i == 0 && (func_name == "Vec::with_capacity" || func_name == "HashMap::with_capacity" ||
                                      func_name == "String::with_capacity" || func_name == "Vec::reserve") {
                            match arg {
                                Expression::Identifier { .. } => {
                                    // Variables: add explicit cast
                                    arg_str = format!("{} as usize", arg_str);
                                }
                                Expression::Literal { value: Literal::Int(val), .. } => {
                                    // Literals: use usize suffix
                                    arg_str = format!("{}_usize", val);
                                }
                                _ => {
                                    // Other expressions (e.g., calculations): wrap in (expr) as usize
                                    if !arg_str.ends_with("_usize") && !arg_str.contains(" as usize") {
                                        arg_str = format!("({}) as usize", arg_str);
                                    }
                                }
                            }
                        }

                        // WINDJAMMER FFI: Convert string arguments for extern functions
                        if is_extern_call {
                            if let Some(ref sig) = signature {
                                if let Some(param_type) = sig.param_types.get(i) {
                                    if matches!(param_type, Type::Custom(name) if name == "str") {
                                        // Expand str to (ptr, len)
                                        return vec![
                                            format!("{}.as_bytes().as_ptr()", arg_str),
                                            format!("{}.as_bytes().len()", arg_str),
                                        ];
                                    }
                                    // string/String params → FfiString via string_to_ffi
                                    // TDD FIX: Always use .to_string() - infer_expression_type returns
                                    // declared param type (Type::String), not actual Rust type. When
                                    // ownership infers Borrowed, param becomes &str in Rust, but we
                                    // thought it was String and passed directly → E0308.
                                    // .to_string() works for both &str and String (String::to_string = clone).
                                    //
                                    // TDD FIX: Strip redundant .to_string() before wrapping.
                                    // Bug: User writes render_text(label.to_string(), x, y). Expression
                                    // generation produces "label.to_string()", then we added another
                                    // → string_to_ffi(label.to_string().to_string()). Fix: If arg_str
                                    // already ends with .to_string(), don't add another.
                                    if matches!(param_type, Type::String)
                                        || matches!(param_type, Type::Custom(n) if n == "string" || n == "String")
                                    {
                                        let inner = if arg_str.ends_with(".to_string()") {
                                            arg_str.clone()
                                        } else {
                                            format!("{}.to_string()", arg_str)
                                        };
                                        return vec![format!(
                                            "windjammer_runtime::ffi::string_to_ffi({})",
                                            inner
                                        )];
                                    }
                                }
                            }
                        }

                        // Auto-convert string literals to String for functions expecting owned String
                        // THE WINDJAMMER WAY: Smart inference based on available information!
                        if matches!(
                            arg,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        ) {
                            // Check if the parameter expects an owned String
                            let should_convert = if let Some(ref sig) = signature {
                                if sig.is_extern {
                                    // Extern functions have explicit types; ownership inference
                                    // is meaningless (empty body defaults to Borrowed).
                                    // Convert if parameter type is String.
                                    sig.param_types.get(i).is_some_and(|ty| {
                                        matches!(ty, Type::String)
                                            || matches!(ty, Type::Custom(name) if name == "string" || name == "String")
                                    })
                                } else if sig.param_types.get(i).is_some_and(|ty| {
                                    matches!(ty, Type::Reference(inner) if
                                        matches!(**inner, Type::Custom(ref s) if s == "str"))
                                }) {
                                    // Parameter type is &str (string optimization inferred this).
                                    // String literals are already &str in Rust — no .to_string() needed.
                                    false
                                } else if signature_from_simple_fallback && {
                                    let qualifier = func_name.split("::").next().unwrap_or("");
                                    qualifier.chars().next().is_some_and(|c| c.is_lowercase())
                                } {
                                    // Fallback-resolved from module::function: the signature may
                                    // be from a different module. Don't trust ownership for
                                    // string coercion — the actual target may take &str.
                                    false
                                } else if let Some(&ownership) = sig.param_ownership.get(i) {
                                    // Convert if parameter expects owned String
                                    matches!(ownership, OwnershipMode::Owned)
                                } else {
                                    // No ownership info for this param
                                    // THE WINDJAMMER WAY: Heuristic for constructors
                                    // Functions named 'new' (or Type::new) taking string params likely expect String
                                    func_name == "new" || func_name.ends_with("::new")
                                }
                            } else {
                                // No signature found — check enum variant registry
                                // WINDJAMMER FIX: Enum variant constructors like GameEvent::ItemPickup("text")
                                // need .to_string() when the variant field is String type
                                if let Some(variant_types) = self.enum_variant_types.get(&func_name) {
                                    // TDD FIX: Check for both Type::String and Type::Custom("String")
                                    variant_types.get(i).is_some_and(|ty| {
                                        matches!(ty, Type::String)
                                            || matches!(ty, Type::Custom(name) if name == "String")
                                    })
                                } else {
                                    // Fallback heuristic for constructors
                                    func_name == "new" || func_name.ends_with("::new")
                                }
                            };

                            if should_convert {
                                arg_str = format!("{}.to_string()", arg_str);
                            }
                        }

                        // Check if this parameter expects a borrow
                        // Skip ownership inference for extern function calls - they have explicit types
                        if let Some(ref sig) = signature {
                            if sig.is_extern {
                                // Auto-convert mut locals to &mut when FFI param is *mut T
                                // This eliminates Rust leakage: users write `ffi_fn(x)` not `ffi_fn(&mut x)`
                                if let Some(param_type) = sig.param_types.get(i) {
                                    if matches!(param_type, crate::parser::ast::types::Type::RawPointer { mutable: true, .. }) {
                                        return vec![format!("&mut {}", arg_str)];
                                    }
                                }
                                return vec![arg_str];
                            }

                            // COLLISION GUARD: When the signature was resolved via a
                            // simple-name fallback from a module-qualified call AND the
                            // simple name has a collision, skip auto-borrow/auto-mutborrow.
                            // The looked-up signature may be from the wrong module,
                            // so applying its ownership blindly can produce incorrect
                            // `&` or `&mut` prefixes.
                            //
                            // We only guard fallback-resolved signatures because:
                            // - Direct qualified lookups are unambiguous (right signature)
                            // - Bare-name calls within the same file are also unambiguous
                            // - Only fallback from module::fn → fn is risky (wrong module)
                            let simple_name = func_name.rsplit("::").next().unwrap_or(&func_name);
                            let has_ownership_collision = signature_from_simple_fallback
                                && (self.signature_registry.has_collision(&func_name)
                                    || self.signature_registry.has_collision(simple_name))
                                && {
                                    // Validate collision: if the found signature's arg count
                                    // matches the actual call, it's the right overload despite
                                    // the collision. Only suppress ownership when arg count
                                    // doesn't match (genuinely ambiguous signature).
                                    let sig_args = if sig.has_self_receiver {
                                        sig.param_ownership.len().saturating_sub(1)
                                    } else {
                                        sig.param_ownership.len()
                                    };
                                    sig_args != arguments.len()
                                };

                            if let Some(&ownership) = sig.param_ownership.get(i) {
                                match ownership {
                                    OwnershipMode::Borrowed if !has_ownership_collision => {
                                        // PHASE 1: Generate &String parameters for correctness
                                        // String literals need conversion: "foo" → &"foo".to_string()
                                        let is_string_literal = matches!(
                                            arg,
                                            Expression::Literal {
                                                value: Literal::String(_),
                                                ..
                                            }
                                        );

                                        if is_string_literal {
                                            // PHASE 2 CALL-SITE OPTIMIZATION: Check if parameter is &String vs &str
                                            // In the AST, `string` parameters are Type::String (converted to &String by codegen)
                                            // Explicit `&str` parameters are Type::Reference(Custom("str"))
                                            let param_is_str_ref = sig.param_types.get(i).is_some_and(|t| {
                                                matches!(t, Type::Reference(inner) if matches!(**inner, Type::Custom(ref name) if name == "str"))
                                            });

                                            if param_is_str_ref {
                                                // Parameter is explicitly &str - pass literal directly (already a &str)
                                                // "World" is already &str in Rust, no conversion needed!
                                                return vec![arg_str];
                                            } else {
                                                // Parameter is Type::String (becomes &String in Rust)
                                                // Check if it's actually a String type
                                                let param_is_string = sig.param_types.get(i).is_some_and(|t| {
                                                    matches!(t, Type::String) || matches!(t, Type::Custom(ref name) if name == "string")
                                                });
                                                if param_is_string {
                                                    // Convert &str literal to &String: "World" → &"World".to_string()
                                                    return vec![format!("&{}.to_string()", arg_str)];
                                                } else {
                                                    // Non-string type - pass directly
                                                    return vec![arg_str];
                                                }
                                            }
                                        }

                                        // TDD FIX: Check if parameter is already a reference type
                                        // If param is &string, don't add another & (would be &&string)
                                        let is_param_already_ref =
                                            if let Expression::Identifier { name, .. } = arg {
                                                self.current_function_params.iter().any(|param| {
                                                    param.name == *name
                                                        && matches!(
                                                            &param.type_,
                                                            Type::Reference(_)
                                                                | Type::MutableReference(_)
                                                        )
                                                })
                                            } else {
                                                false
                                            };

                                        // TDD FIX: Don't add & for Copy type parameters
                                        // When signature says Borrowed but param type is Copy,
                                        // codegen keeps it as owned (e.g., x: usize not x: &usize)
                                        // So the call site should NOT add &
                                        // BUT: Reference types (&Vec<T>, &[T]) are NOT treated as
                                        // Copy here - if param type is &T, caller still needs &
                                        let is_copy_param = sig.param_types.get(i)
                                            .map(|t| {
                                                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                                    && crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::is_copy_type_annotation_pub(t)
                                            })
                                            .unwrap_or(false);

                                        // TDD FIX (Bug #16): Don't add & to temp variables!
                                        // Temp variables (like _temp0) hold OWNED values from format!()
                                        // format!() returns String, not &str, so _temp0 is String
                                        // If we add &, we get &String when we need String
                                        let is_temp_variable = arg_str.starts_with("_temp")
                                            && arg_str.chars().skip(5).all(|c| c.is_numeric());

                                        // TDD FIX: IDIOMATIC WINDJAMMER - Strip .clone() if present!
                                        // When destination wants Borrowed, pass &field, NOT &field.clone()
                                        // Example: has_item(ingredient.item_id) with has_item(item_id: string)
                                        // Should generate: has_item(&ingredient.item_id)
                                        // NOT: has_item(&ingredient.item_id.clone())
                                        // The .clone() may have been added by generate_expression for borrowed iterator vars
                                        if arg_str.ends_with(".clone()") {
                                            arg_str = arg_str[..arg_str.len() - 8].to_string();
                                        }

                                        // Insert & if not already a reference and not a string literal and not a temp var
                                        // THE WINDJAMMER WAY: Preserve user-written closure params
                                        let is_user_closure_param = if let Expression::Identifier { name, .. } = arg {
                                            self.in_user_written_closure && self.user_closure_params.contains(name)
                                        } else {
                                            false
                                        };

                                        if !expression_helpers::is_reference_expression(arg)
                                            && !is_param_already_ref
                                            && !is_copy_param
                                            && !is_temp_variable
                                            && !is_user_closure_param
                                        {
                                            return vec![format!("&{}", arg_str)];
                                        } else {
                                            return vec![arg_str];
                                        }
                                    }
                                    OwnershipMode::MutBorrowed if !has_ownership_collision => {
                                        // TDD FIX: Don't add &mut if arg is already a &mut parameter
                                        // Covers both explicitly declared &mut params AND
                                        // params inferred as &mut through ownership analysis
                                        let is_already_mut_ref =
                                            if let Expression::Identifier { name, .. } = arg {
                                                // Check 1: Explicit &mut in AST type
                                                let explicit_mut_ref = self.current_function_params.iter().any(|param| {
                                                    param.name == *name
                                                        && matches!(
                                                            &param.type_,
                                                            Type::MutableReference(_)
                                                        )
                                                });
                                                // Check 2: Inferred &mut through ownership analysis
                                                let inferred_mut_ref = self.inferred_mut_borrowed_params.contains(name.as_str());
                                                explicit_mut_ref || inferred_mut_ref
                                            } else {
                                                false
                                            };

                                        // Insert &mut if not already a reference
                                        if !expression_helpers::is_reference_expression(arg)
                                            && !is_already_mut_ref
                                        {
                                            // CRITICAL FIX: Remove .clone() if present - we want to mutate the original!
                                            // &mut counter.clone() → &mut counter
                                            // When passing &mut, we're giving mutable access to the original,
                                            // not a clone. The .clone() would break mutation semantics.
                                            let mut_arg_str = if arg_str.ends_with(".clone()") {
                                                arg_str[..arg_str.len() - 8].to_string()
                                            } else {
                                                arg_str
                                            };
                                            return vec![format!("&mut {}", mut_arg_str)];
                                        }
                                    }
                                    OwnershipMode::Owned => {
                                        // String optimization override: param_types may say &str
                                        // while param_ownership is stale as Owned. Trust param_types.
                                        let param_is_str_ref = sig.param_types.get(i).is_some_and(|t| {
                                            matches!(t, Type::Reference(inner) if
                                                matches!(**inner, Type::Custom(ref s) if s == "str"))
                                        });
                                        if param_is_str_ref {
                                            return vec![arg_str];
                                        }

                                        if let Expression::Identifier { name, .. } = arg {
                                            // Find the parameter type
                                            let param_type = self
                                                .current_function_params
                                                .iter()
                                                .find(|p| &p.name == name)
                                                .map(|p| &p.type_);

                                            // Check if it's a reference parameter (&str, &String, &T, &mut T)
                                            let inner_from_ref = match param_type {
                                                Some(Type::Reference(inner)) => Some(inner.as_ref()),
                                                Some(Type::MutableReference(inner)) => Some(inner.as_ref()),
                                                _ => None,
                                            };
                                            if let Some(inner_type) = inner_from_ref {
                                                if matches!(inner_type, Type::String)
                                                    && !arg_str.ends_with(".to_string()")
                                                    && !arg_str.ends_with(".clone()")
                                                {
                                                    arg_str = format!("{}.to_string()", arg_str);
                                                } else if self.is_type_copy(inner_type)
                                                    && !arg_str.trim_start().starts_with('*')
                                                {
                                                    arg_str = format!("*{}", arg_str);
                                                } else if !arg_str.ends_with(".clone()")
                                                    && !arg_str.trim_start().starts_with('*')
                                                {
                                                    arg_str = format!("{}.clone()", arg_str);
                                                }
                                            } else {
                                                // TDD FIX: Check if it's from a borrowed iterator (for loop)
                                                // Example: for npc_id in npc_ids { Member::new(npc_id) }
                                                // npc_id is &String from iterator, needs .clone() for owned String
                                                //
                                                // CRITICAL: We're in OwnershipMode::Owned block, which means
                                                // the DESTINATION parameter wants an owned value (String, not &String).
                                                //
                                                // Windjammer `string` parameters lower to `&str`: `.clone()` keeps
                                                // `&str` (E0308). Use `.to_string()` for text types instead.
                                                let is_borrowed_iterator_var =
                                                    self.borrowed_iterator_vars.contains(name);

                                                let is_inferred_borrowed =
                                                    self.inferred_borrowed_params.contains(name);

                                                let is_inferred_mut_borrowed =
                                                    self.inferred_mut_borrowed_params.contains(name);

                                                if (is_borrowed_iterator_var
                                                    || is_inferred_borrowed
                                                    || is_inferred_mut_borrowed)
                                                    && !arg_str.ends_with(".clone()")
                                                {
                                                    // `*ident` = owned Copy from &/&mut (see Identifier
                                                    // in_owned_value_context); do not append .clone().
                                                    if !arg_str.trim_start().starts_with('*') {
                                                        let is_text = self
                                                            .infer_expression_type(arg)
                                                            .as_ref()
                                                            .is_some_and(|t| {
                                                            crate::codegen::rust::types::is_windjammer_text_type(t)
                                                        });
                                                        let is_phase2_str_param = self
                                                            .str_ref_optimized_params
                                                            .contains(name.as_str());
                                                        if is_text && !is_phase2_str_param {
                                                            arg_str =
                                                                format!("{}.to_string()", arg_str);
                                                        } else if !is_text {
                                                            // Borrowed from iterator or inferred - use .clone()
                                                            // This handles &T → T for non-text types
                                                            arg_str = format!("{}.clone()", arg_str);
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        // TDD FIX: AUTO-CLONE for borrowed_param.field
                                        // When passing ingredient.item_id where ingredient is borrowed,
                                        // we need to clone() IF destination wants Owned.
                                        //
                                        // We're ALREADY in OwnershipMode::Owned block,
                                        // so destination wants owned. Safe to add .clone().
                                        //
                                        // This handles: for ingredient in &vec { func(ingredient.field) }
                                        // where func(field: String) expects owned.
                                        if let Expression::FieldAccess { .. } = arg {
                                            // Trace through nested field accesses to find the root identifier
                                            // Handles: stack.field, stack.item.id, stack.item.nested.deep
                                            let root_name = self.extract_root_identifier(arg);
                                            if let Some(name) = root_name {
                                                let is_borrowed_iterator_var =
                                                    self.borrowed_iterator_vars.contains(&name);
                                                let is_explicitly_borrowed =
                                                    self.current_function_params.iter().any(|p| {
                                                        p.name == name
                                                            && matches!(
                                                                p.ownership,
                                                                crate::parser::OwnershipHint::Ref
                                                            )
                                                    });
                                                let is_inferred_borrowed =
                                                    self.inferred_borrowed_params.contains(&name);

                                                if (is_borrowed_iterator_var
                                                    || is_explicitly_borrowed
                                                    || is_inferred_borrowed)
                                                    && !arg_str.ends_with(".clone()")
                                                {
                                                    let is_copy = self.infer_expression_type(arg)
                                                        .as_ref()
                                                        .is_some_and(|t| self.is_type_copy(t));
                                                    if !is_copy {
                                                        arg_str = format!("{}.clone()", arg_str);
                                                    }
                                                }
                                            }
                                        }
                                        // DOGFOODING FIX: Vec indexing &vec[idx] passed to owned param
                                        // e.g. enterable.push(self.buildings[i]) → need (.clone())
                                        if let Expression::Index { .. } = arg {
                                            if arg_str.starts_with("&")
                                                && !arg_str.ends_with(".clone()")
                                            {
                                                if let Some(inner) = self.infer_expression_type(arg)
                                                {
                                                    if !self.is_type_copy(&inner) {
                                                        arg_str =
                                                            format!("({}).clone()", arg_str);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        // Collision guard triggered: Borrowed or MutBorrowed
                                        // with a signature collision. Don't apply auto-borrow;
                                        // pass the argument as-is and let downstream Rust
                                        // compilation determine the correct behavior.
                                    }
                                }
                            }
                        } else {
                            // No signature found - don't auto-clone!
                            // Without signature info, we can't know if destination wants Owned or Borrowed
                            // Better to let Rust compiler catch the error than guess wrong
                        }

                        // AUTO-CAST int → float: regular Call path
                        // Skip when the signature key has a collision (different types registered
                        // the same function name with different param types). The auto-cast
                        // cannot be trusted when the looked-up signature may be from a different
                        // type in another module.
                        if let Some(ref sig) = signature {
                            let has_collision = self.signature_registry.has_collision(&func_name)
                                || self.signature_registry.has_collision(&func_str);
                            if !has_collision {
                                if let Some(param_ty) = sig.param_types.get(i) {
                                    let param_is_f32 = matches!(param_ty, Type::Custom(n) if n == "f32");
                                    let param_is_f64 = matches!(param_ty, Type::Custom(n) if n == "f64");
                                    if param_is_f32 || param_is_f64 {
                                        let arg_ty = self.infer_expression_type(arg);
                                        let arg_is_int = arg_ty.as_ref().is_some_and(|t| {
                                            matches!(t, Type::Int)
                                                || matches!(t, Type::Custom(n) if crate::type_classification::is_integer_type(n))
                                        });
                                        if arg_is_int && !arg_str.contains(" as f32") && !arg_str.contains(" as f64") {
                                            let target = if param_is_f32 { "f32" } else { "f64" };
                                            arg_str = if arg_str.contains(' ') || matches!(arg, Expression::Binary { .. }) {
                                                format!("({}) as {}", arg_str, target)
                                            } else {
                                                format!("{} as {}", arg_str, target)
                                            };
                                        }
                                    }
                                }
                            }
                        }

                        vec![arg_str]
                    })
                    .collect();

                // TDD FIX (Bug #3): Extract format!() macros in arguments to temp variables
                // The args vec has already been generated as Rust strings
                // Check if any contain format!() and extract them
                let has_format_arg = args.iter().any(|arg_str| arg_str.contains("format!("));

                // WINDJAMMER FFI: Extern functions returning string use FfiString - wrap with ffi_to_string
                let returns_string = signature
                    .as_ref()
                    .and_then(|s| s.return_type.as_ref())
                    .is_some_and(|t| {
                        matches!(t, Type::String)
                            || matches!(t, Type::Custom(n) if n == "string" || n == "String")
                    });

                // WINDJAMMER PHILOSOPHY: Auto-wrap extern function calls in unsafe blocks
                // THE WINDJAMMER WAY: Users shouldn't have to write `unsafe` manually
                let call_result = if has_format_arg {
                    // Extract format!() macros to temp variables
                    let mut temp_decls = String::new();
                    let mut temp_counter = 0;
                    let fixed_args: Vec<String> = args
                        .iter()
                        .map(|arg_str| {
                            if arg_str.starts_with("format!(") || arg_str.starts_with("&format!(") {
                                // TDD FIX (Bug #16 COMPLETE): Check if original had & to preserve intent
                                let has_borrow_prefix = arg_str.starts_with("&");
                                // Strip leading & if present
                                let format_expr = if has_borrow_prefix {
                                    &arg_str[1..]
                                } else {
                                    arg_str
                                };
                                // Extract to temp var
                                let temp_name = format!("_temp{}", temp_counter);
                                temp_counter += 1;
                                temp_decls
                                    .push_str(&format!("let {} = {}; ", temp_name, format_expr));

                                // TDD FIX: Only add & if original had it!
                                // format!() returns owned String, so if caller wants owned, pass temp directly
                                // If caller wants borrowed, pass &temp (when original was &format!())
                                if has_borrow_prefix {
                                    format!("&{}", temp_name)
                                } else {
                                    temp_name
                                }
                            } else {
                                arg_str.clone()
                            }
                        })
                        .collect();

                    let call_expr = format!("{}({})", func_str, fixed_args.join(", "));

                    // Wrap in unsafe block if extern, otherwise regular block
                    // Parenthesize so the block can be used as a sub-expression (e.g., in comparisons)
                    if is_extern_call && !self.in_unsafe_block {
                        format!("(unsafe {{ {}{}  }})", temp_decls, call_expr)
                    } else {
                        format!("{{ {}{} }}", temp_decls, call_expr)
                    }
                } else {
                    // No format!() args - generate normally with optional unsafe wrapper
                    let call_str = format!("{}({})", func_str, args.join(", "));
                    if is_extern_call && !self.in_unsafe_block {
                        format!("(unsafe {{ {} }})", call_str)
                    } else {
                        call_str
                    }
                };

                // Wrap extern string return with ffi_to_string
                if is_extern_call && returns_string {
                    format!("windjammer_runtime::ffi::ffi_to_string({})", call_result)
                } else {
                    call_result
                }
            }
            Expression::MethodCall {
                object,
                method,
                type_args,
                arguments,
                ..
            } => {
                // TDD FIX: Strip redundant .as_str() on &str parameters
                // If method is .as_str() and object is already inferred as &str, just return object
                if method == "as_str" && arguments.is_empty() {
                    if let Expression::Identifier { name, .. } = object {
                        let is_borrowed = self.inferred_borrowed_params.contains(name.as_str());
                        if is_borrowed {
                            // Parameter is already &str, .as_str() is redundant
                            return self.generate_expression(object);
                        }
                    }
                }

                // METHOD CALL CONTEXT: Suppress Vec index auto-clone when generating the
                // object of a method call. Methods take &self or &mut self, so Rust allows
                // calling methods on &T returned by Vec indexing without cloning.
                // e.g., self.lights[i].is_enabled() → no need to clone the whole Light2D
                let prev_field_access = self.in_field_access_object;
                self.in_field_access_object = true;
                // DOUBLE-CLONE FIX: When the source has explicit .clone(), suppress auto-clone
                // on the object to prevent .clone().clone(). The explicit clone IS the clone.
                let prev_explicit_clone = self.in_explicit_clone_call;
                if method == "clone" {
                    self.in_explicit_clone_call = true;
                }
                let mut obj_str = self.generate_expression_with_precedence(object);
                self.in_field_access_object = prev_field_access;
                self.in_explicit_clone_call = prev_explicit_clone;
                // E0507: `collection[i].method(args)` when the method consumes `self` (owned receiver)
                // must clone the element: `self.tracks[i].clone().sample(t)` (otherwise move out of &Vec).
                if matches!(&**object, Expression::Index { .. }) {
                    if let Some(recv_ty) = self.infer_expression_type(object) {
                        if !self.is_type_copy(&recv_ty) {
                            if let Some(tn) = Self::type_to_name(&recv_ty) {
                                let qualified = format!("{}::{}", tn, method);
                                let sig_opt = self
                                    .signature_registry
                                    .get_signature(&qualified)
                                    .or_else(|| self.signature_registry.get_signature(method));
                                if let Some(sig) = sig_opt {
                                    if sig.has_self_receiver
                                        && sig.param_ownership.first()
                                            == Some(&crate::analyzer::OwnershipMode::Owned)
                                        && !obj_str.ends_with(".clone()")
                                    {
                                        obj_str = format!("{}.clone()", obj_str);
                                    }
                                }
                            }
                        }
                    }
                }

                // E0507: `borrowed_var.method(args)` when the method consumes `self` (owned receiver)
                // and the variable is a borrowed iterator variable (from `for x in &collection`).
                // Must clone: `condition.clone().evaluate(state)` instead of `condition.evaluate(state)`.
                if let Expression::Identifier { name, .. } = &**object {
                    if self.borrowed_iterator_vars.contains(name) && method != "clone" {
                        if let Some(recv_ty) = self.infer_expression_type(object) {
                            if !self.is_type_copy(&recv_ty) {
                                if let Some(tn) = Self::type_to_name(&recv_ty) {
                                    let qualified = format!("{}::{}", tn, method);
                                    let sig_opt = self
                                        .signature_registry
                                        .get_signature(&qualified)
                                        .or_else(|| self.signature_registry.get_signature(method));
                                    if let Some(sig) = sig_opt {
                                        if sig.has_self_receiver
                                            && sig.param_ownership.first()
                                                == Some(&crate::analyzer::OwnershipMode::Owned)
                                            && !obj_str.ends_with(".clone()")
                                        {
                                            obj_str = format!("{}.clone()", obj_str);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // DOUBLE-CLONE SAFETY NET: If the object was auto-cloned by the FieldAccess
                // handler and this IS a .clone() call, strip the redundant auto-clone.
                // e.g., "stack.item.clone()" from auto-clone + ".clone()" from source
                //     → should be "stack.item.clone()", not "stack.item.clone().clone()"
                if method == "clone" && obj_str.ends_with(".clone()") {
                    obj_str = obj_str[..obj_str.len() - 8].to_string();
                }

                // TDD FIX: Option::unwrap() move error prevention
                // TDD FIX: AUTO-CLONE Option::unwrap() on borrowed fields
                // When calling .unwrap() on a borrowed Option field, we must clone before unwrap:
                //   node.children.unwrap() where node is &Node → ERROR: cannot move from &Option
                //   node.children.clone().unwrap() → ✅ OK
                // THE WINDJAMMER WAY: Users write .unwrap() naturally, compiler handles ownership
                if method == "unwrap" {
                    // Check if object is a field access (node.children) that needs clone
                    let needs_clone = if let Expression::FieldAccess {
                        object: field_obj, ..
                    } = object
                    {
                        // Is this accessing a field on a borrowed parameter?
                        if let Expression::Identifier { ref name, .. } = **field_obj {
                            // Check if the identifier is an inferred borrowed parameter
                            self.inferred_borrowed_params.contains(name)
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if needs_clone && !obj_str.contains(".clone()") {
                        obj_str = format!("{}.clone()", obj_str);
                    }
                }

                // E0507 fix: Option::map on self.field with &self must use .as_ref().map(...)
                // self.children.map(|c| ...) with &self → self.children.as_ref().map(|c| ...)
                if method == "map"
                    && self.inferred_borrowed_params.contains("self")
                    && self.codegen_expression_traces_to_self(object)
                {
                    if !obj_str.contains(".as_ref()") {
                        obj_str = format!("{}.as_ref()", obj_str);
                    }
                }

                // BUG #8 FIX: Look up method signature with qualified name (Type::method)
                // First try to infer the type from the object expression
                let type_name = self.infer_type_name(object);
                let method_signature = if let Some(ref type_name) = type_name {
                    let qualified_name = format!("{}::{}", type_name, method);
                    let mut sig = self
                        .signature_registry
                        .get_signature(&qualified_name)
                        .cloned();
                    // Validate: if the signature's param count doesn't match the call's
                    // argument count, it's a name collision (e.g., two different types
                    // both named Ability with different activate methods). In that case,
                    // try module-qualified alternatives from the registry.
                    if let Some(ref found_sig) = sig {
                        let expected_args = if found_sig.has_self_receiver {
                            found_sig.param_ownership.len().saturating_sub(1)
                        } else {
                            found_sig.param_ownership.len()
                        };
                        if expected_args != arguments.len() {
                            // Wrong signature due to name collision; try alternatives
                            sig = None;
                            for (key, alt_sig) in &self.signature_registry.signatures {
                                if key.ends_with(&format!("::{}", qualified_name))
                                    && key != &qualified_name
                                {
                                    let alt_args = if alt_sig.has_self_receiver {
                                        alt_sig.param_ownership.len().saturating_sub(1)
                                    } else {
                                        alt_sig.param_ownership.len()
                                    };
                                    if alt_args == arguments.len() {
                                        sig = Some(alt_sig.clone());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    sig
                    // CRITICAL: Do NOT fall back to unqualified method name lookup!
                    // Unqualified lookup for common names like "get", "remove", "contains"
                    // can match WRONG user-defined methods (e.g., ComponentArray::get when
                    // we want HashMap::get), causing incorrect auto-ref/auto-clone behavior.
                    // When the qualified name isn't found, method_signature stays None and
                    // the stdlib heuristics in should_add_ref handle common patterns correctly.
                } else {
                    if super::stdlib_method_traits::is_common_stdlib_method(method) {
                        None
                    } else {
                        self.signature_registry
                            .get_signature(method)
                            .cloned()
                            .or_else(|| {
                                let suffix_sig = self
                                    .signature_registry
                                    .find_signature_ending_with(method)
                                    .cloned();
                                if let Some(ref sig) = suffix_sig {
                                    let expected_args = if sig.has_self_receiver {
                                        sig.param_ownership.len().saturating_sub(1)
                                    } else {
                                        sig.param_ownership.len()
                                    };
                                    if expected_args == arguments.len() {
                                        return suffix_sig;
                                    }
                                }
                                None
                            })
                    }
                };

                // Float method argument context: for methods like clamp/max/min on float
                // receivers, arguments should use the same float type as the receiver.
                let prev_float_target = self.assignment_float_target_type.clone();
                let receiver_float_type = self.infer_expression_type(object);
                let is_float_method = crate::type_classification::is_float_receiver_method(method);
                if is_float_method {
                    if let Some(ref rft) = receiver_float_type {
                        match rft {
                            Type::Custom(n) if n == "f64" => {
                                self.assignment_float_target_type =
                                    Some(Type::Custom("f64".to_string()));
                            }
                            Type::Custom(n) if n == "f32" => {
                                self.assignment_float_target_type =
                                    Some(Type::Custom("f32".to_string()));
                            }
                            Type::Float => {
                                self.assignment_float_target_type =
                                    Some(Type::Custom("f64".to_string()));
                            }
                            _ => {}
                        }
                    }
                }

                let args: Vec<String> = arguments
                    .iter()
                    .enumerate()
                    .map(|(i, (_label, arg))| {
                        // TDD FIX: Suppress auto-clone for FieldAccess when method expects Borrowed
                        // Bug: ingredient.item_id generates .clone(), then & is added -> &cloned_value
                        // Fix: Suppress clone when param expects Borrowed -> just add & to field
                        let sig_param_idx = if method_signature.as_ref().is_some_and(|s| s.has_self_receiver) { i + 1 } else { i };
                        let param_expects_borrowed = method_signature
                            .as_ref()
                            .and_then(|sig| sig.param_ownership.get(sig_param_idx))
                            .is_some_and(|&o| matches!(o, crate::analyzer::OwnershipMode::Borrowed));

                        const AUTO_BORROW_METHODS: &[&str] = &["push_str", "extend_from_slice"];
                        let is_auto_borrow_target = AUTO_BORROW_METHODS.contains(&method.as_str()) && i == 0;

                        let prev_suppress = self.suppress_borrowed_clone;
                        if (param_expects_borrowed || is_auto_borrow_target)
                            && matches!(arg, Expression::FieldAccess { .. } | Expression::Identifier { .. })
                        {
                            self.suppress_borrowed_clone = true;
                        }

                        // CRITICAL: Reset in_field_access_object for method argument generation.
                        // Same rationale as function call arguments — method arguments are
                        // independent expressions, not part of a field/method/index chain.
                        // TDD FIX: STRIP explicit &ref when parameter expects owned value.
                        // WINDJAMMER PHILOSOPHY: The developer shouldn't need to think about &.
                        // If the user writes `&object.transform` but the method takes `Transform` (owned),
                        // the compiler strips the & and passes by value (Copy types) or moves.
                        // Example: self.render_transform(&object.transform) → self.render_transform(object.transform)
                        //
                        // TDD FIX: ALSO strip explicit & for HashMap/BTreeMap key methods with &String arguments.
                        // HashMap<String, V>.contains_key() expects &str, not &&String.
                        // User writes: map.contains_key(&key) where key is inferred as &String
                        // Compiler generates: map.contains_key(key) which auto-derefs &String to &str ✅
                        let arg_to_generate = if let Expression::Unary {
                            op: crate::parser::UnaryOp::Ref,
                            operand,
                            ..
                        } = arg
                        {
                            let is_hashmap_key_method =
                                super::stdlib_method_traits::is_map_key_method(method) && i == 0;

                            if is_hashmap_key_method {
                                // Strip explicit `&ident` for map keys: `should_add_ref` will add `&` back when the
                                // Rust type is owned or a Copy `K` that still needs `&K`. For `key: &str` / `&String`
                                // parameters, `should_add_ref` stays false → we emit `get(key)` not `get(&key)` (E0277).
                                if let Expression::Identifier { .. } = &**operand {
                                    operand
                                } else {
                                    arg
                                }
                            } else if let Some(ref sig) = method_signature {
                                let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                                let param_is_owned = sig
                                    .param_ownership
                                    .get(sig_param_idx)
                                    .is_some_and(|&o| matches!(o, crate::analyzer::OwnershipMode::Owned));
                                if param_is_owned {
                                    operand // Strip & — generate the inner expression
                                } else {
                                    arg // Keep the & — parameter expects a reference
                                }
                            } else {
                                arg // No signature info — keep as-is
                            }
                        } else {
                            arg // Not a & expression — keep as-is
                        };

                        let prev_field_access_obj = self.in_field_access_object;
                        self.in_field_access_object = false;
                        let prev_coerce_string_literals = self.coerce_string_literals_to_owned;
                        self.coerce_string_literals_to_owned = false;
                        let prev_match_arm_str = self.in_match_arm_needing_string;
                        self.in_match_arm_needing_string = false;
                        let mut arg_str = self.generate_expression(arg_to_generate);
                        self.coerce_string_literals_to_owned = prev_coerce_string_literals;
                        self.in_match_arm_needing_string = prev_match_arm_str;
                        self.in_field_access_object = prev_field_access_obj;

                        // TDD FIX: PHASE 2 CALL-SITE OPTIMIZATION
                        // Strip unnecessary .to_string() when parameter was optimized to &str
                        // Example: User writes `loader.load("name".to_string())` but Phase 2 optimized
                        // the signature from `fn load(self, name: String)` to `fn load(self, name: &str)`.
                        // Result: Call site should be `loader.load("name")` not `loader.load("name".to_string())`
                        //
                        // IMPORTANT: Only strip for &str parameters, NOT &String parameters!
                        // &String parameters still need .to_string() (creates String, then borrows it)
                        if let Some(ref sig) = method_signature {
                            let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                            if let Some(param_type) = sig.param_types.get(sig_param_idx) {
                                // Check if parameter is specifically &str (not &String!)
                                let param_is_str_slice_ref = if let Type::Reference(inner) = param_type {
                                    matches!(&**inner, Type::Custom(name) if name == "str")
                                } else {
                                    false
                                };
                                if param_is_str_slice_ref && arg_str.ends_with(".to_string()") {
                                    // Strip .to_string() - &str accepts string literals directly
                                    arg_str = arg_str[..arg_str.len() - 12].to_string();
                                }
                            }
                        }

                        // TDD FIX: Vec index methods require usize arguments.
                        // Int inference may resolve the literal to i32/u32/i64/u64 due to
                        // conflicting constraints. Fix at codegen level: rewrite any
                        // integer suffix to _usize for the first argument of known
                        // index-taking methods.
                        if i == 0
                            && super::stdlib_method_traits::is_index_taking_method(method)
                        {
                            let is_int_literal = matches!(
                                arg,
                                Expression::Literal {
                                    value: Literal::Int(_) | Literal::IntSuffixed(_, _),
                                    ..
                                }
                            );
                            if is_int_literal {
                                let int_suffixes =
                                    ["_i32", "_i64", "_u32", "_u64", "_i16", "_u16", "_i8", "_u8"];
                                for suffix in &int_suffixes {
                                    if arg_str.ends_with(suffix) {
                                        arg_str = format!(
                                            "{}_usize",
                                            &arg_str[..arg_str.len() - suffix.len()]
                                        );
                                        break;
                                    }
                                }
                            }
                        }

                        // TDD FIX: AUTO-WRAP function pointers in iterator adapter methods.
                        // Rust's .filter()/.any()/.find() on iter() yield &&T, expecting FnMut(&&T) -> bool,
                        // but bare function pointers fn(&T) -> bool don't auto-deref.
                        // THE WINDJAMMER WAY: Users write the natural `filter(predicate)` and the
                        // compiler generates `filter(|__e| predicate(__e))`.
                        if i == 0
                            && super::stdlib_method_traits::is_closure_taking_method(method)
                            && matches!(arg, Expression::Identifier { .. })
                        {
                            // Bare identifier (function pointer) passed to iterator adapter -
                            // wrap in closure so Rust's auto-deref handles &&T -> &T.
                            arg_str = format!("|__e| {}(__e)", arg_str);
                        }

                        // TDD FIX: String literal ownership conversion
                        // Windjammer philosophy: "sword" should work whether parameter wants String or &String
                        // CRITICAL: Do NOT convert for explicit &str parameters! Only for inferred &String.
                        let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                        let sig_param_idx = if method_signature.as_ref().is_some_and(|s| s.has_self_receiver) { i + 1 } else { i };
                        let param_ownership = method_signature
                            .as_ref()
                            .and_then(|sig| sig.param_ownership.get(sig_param_idx));
                        let string_literal_converted = if is_string_literal {
                            // Check what the parameter wants

                            // CRITICAL: Check if parameter is explicitly &str (not inferred &String)
                            // Explicit &str parameters should NOT get .to_string() conversion
                            let param_type = method_signature
                                .as_ref()
                                .and_then(|sig| sig.param_types.get(sig_param_idx));
                            let is_explicit_str_ref = if let Some(Type::Reference(inner)) = param_type {
                                matches!(**inner, Type::String) ||
                                matches!(**inner, Type::Custom(ref s) if s == "str")
                            } else {
                                false
                            };

                            if is_explicit_str_ref {
                                // Explicit &str parameter - no conversion needed
                                false
                            } else {
                                match param_ownership {
                                    Some(&OwnershipMode::Owned) | Some(&OwnershipMode::Borrowed) => {
                                        // TDD FIX: Both Owned and Borrowed string params need .to_string()
                                        // Owned → String needs .to_string()
                                        // Borrowed → &String needs .to_string() (then & is added later)
                                        // String literals are &str, must allocate to get String/&String
                                        arg_str = format!("{}.to_string()", arg_str);
                                        true // Mark that we converted
                                    }
                                    _ => {
                                        // No signature info - use heuristic (fallback to old logic)
                                        if crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(i, method, &method_signature) {
                                            arg_str = format!("{}.to_string()", arg_str);
                                            true
                                        } else {
                                            false
                                        }
                                    }
                                }
                            }
                        } else {
                            false
                        };

                        // TDD FIX: If we converted string literal for Borrowed parameter,
                        // we need to add & since .to_string() produces String but param wants &String
                        if string_literal_converted {
                            if let Some(&OwnershipMode::Borrowed) = param_ownership {
                                // .to_string() produces String, but Borrowed param wants &String
                                // So we need to add &
                                arg_str = format!("&{}", arg_str);
                            }
                        }

                        // TDD FIX: AUTO-CONVERT &str → String for method calls
                        // When passing a Phase 2 optimized &str parameter to a method expecting owned String, convert it
                        // This handles cases like: HashMap::insert(key, value) where key is &str but insert expects String
                        if let Expression::Identifier { name, .. } = arg_to_generate {
                            let is_str_ref_optimized =
                                self.str_ref_optimized_params.contains(name.as_str());

                            if is_str_ref_optimized {
                                let sig_param_idx = if method_signature
                                    .as_ref()
                                    .is_some_and(|s| s.has_self_receiver)
                                {
                                    i + 1
                                } else {
                                    i
                                };
                                if !crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::callee_param_is_rust_str_slice(
                                    &method_signature,
                                    sig_param_idx,
                                ) {
                                    let expects_owned = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(
                                        i,
                                        method,
                                        &method_signature,
                                    );

                                    if expects_owned
                                        && !arg_str.ends_with(".to_string()")
                                        && !arg_str.ends_with(".clone()")
                                    {
                                        arg_str = format!("{}.to_string()", arg_str);
                                    }
                                }
                            }
                        }

                        // AUTO .clone(): Add .clone() when needed for borrowed values
                        if crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_clone(
                            arg,
                            &arg_str,
                            method,
                            i,
                            &method_signature,
                            &self.borrowed_iterator_vars,
                            &self.current_function_params,
                            &self.inferred_borrowed_params,
                            &self.current_function_return_type,
                        ) {
                            arg_str = format!("{}.clone()", arg_str);
                        }

                        // DOGFOODING FIX: Vec indexing vec[idx] passed to owned param (e.g. push)
                        // should_add_clone handles Identifier/FieldAccess; Index needs explicit check
                        // Vec::push uses stdlib heuristics (method_signature=None) - param 0 expects Owned
                        if let Expression::Index { .. } = arg {
                            let sig_param_idx = method_signature
                                .as_ref()
                                .map(|s| if s.has_self_receiver { i + 1 } else { i })
                                .unwrap_or(i);
                            let param_expects_owned = method_signature
                                .as_ref()
                                .and_then(|sig| sig.param_ownership.get(sig_param_idx))
                                .is_some_and(|&o| matches!(o, OwnershipMode::Owned))
                                || (method == "push" && i == 0);
                            if param_expects_owned && !arg_str.ends_with(".clone()") {
                                let inferred = self.infer_expression_type(arg);
                                let is_copy = inferred.as_ref().is_some_and(|t| self.is_type_copy(t));
                                if is_copy {
                                    if arg_str.starts_with("&") {
                                        arg_str = arg_str
                                            .strip_prefix('&')
                                            .unwrap_or(&arg_str)
                                            .to_string();
                                    }
                                } else {
                                    // Non-Copy or unknown type: clone to prevent E0507
                                    if arg_str.starts_with("&") {
                                        arg_str = format!("({}).clone()", arg_str);
                                    } else {
                                        arg_str = format!("{}.clone()", arg_str);
                                    }
                                }
                            }
                        }

                        // TDD FIX: Strip unnecessary .clone() when method param is Borrowed
                        // When a field like `ingredient.item_id` is auto-cloned by the
                        // FieldAccess handler (because owner is borrowed), but the method
                        // expects &String (Borrowed), the clone is wasteful:
                        //   &ingredient.item_id.clone()  ← clones then borrows (wasteful)
                        //   &ingredient.item_id          ← borrows directly (correct)
                        // Strip the .clone() so should_add_ref can add & cleanly.
                        if let Some(ref sig) = method_signature {
                            let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                            let param_is_borrowed = sig
                                .param_ownership
                                .get(sig_param_idx)
                                .is_some_and(|&o| matches!(o, OwnershipMode::Borrowed));
                            if param_is_borrowed && arg_str.ends_with(".clone()") {
                                arg_str = arg_str[..arg_str.len() - 8].to_string();
                            }
                        }

                        // AUTO-MUT-BORROW: Add &mut when parameter expects MutBorrowed
                        if let Some(ref sig) = method_signature {
                            let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                            let param_is_mut_borrowed = sig
                                .param_ownership
                                .get(sig_param_idx)
                                .is_some_and(|&o| matches!(o, OwnershipMode::MutBorrowed));
                            if param_is_mut_borrowed {
                                let is_already_mut_ref =
                                    if let Expression::Identifier { name, .. } = arg {
                                        let explicit_mut_ref = self.current_function_params.iter().any(|param| {
                                            param.name == *name
                                                && matches!(&param.type_, Type::MutableReference(_))
                                        });
                                        let inferred_mut_ref = self.inferred_mut_borrowed_params.contains(name.as_str());
                                        explicit_mut_ref || inferred_mut_ref
                                    } else {
                                        false
                                    };
                                if !expression_helpers::is_reference_expression(arg)
                                    && !is_already_mut_ref
                                {
                                    if arg_str.ends_with(".clone()") {
                                        arg_str = arg_str[..arg_str.len() - 8].to_string();
                                    }
                                    if arg_str.starts_with("&") && !arg_str.starts_with("&mut ") {
                                        arg_str = arg_str[1..].to_string();
                                    }
                                    arg_str = format!("&mut {}", arg_str);
                                }
                            }
                        }

                        // AUTO-REF: Add & when parameter expects reference but arg is owned
                        if !string_literal_converted {
                            // Use `arg_to_generate` (after stripping explicit `&` for map keys / owned params)
                            // so `should_add_ref` sees `key` not `&key` — otherwise the Unary(Ref) early-return
                            // skips HashMap `str` key handling and we emit `get(&key)` for `key: &str` (E0277).
                            let should_ref = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                                arg_to_generate,
                                &arg_str,
                                method,
                                i,
                                &method_signature,
                                &self.usize_variables,
                                &self.current_function_params,
                                &self.borrowed_iterator_vars,
                                &self.inferred_borrowed_params,
                                arguments.len(),
                                type_name.as_deref(),
                                Some(&self.local_var_types),
                                Some(&self.stdlib_method_signatures),
                                Some(&self.method_signatures_by_type),
                                &self.match_arm_bindings, // TDD FIX: Pass match arm bindings for E0308 fix
                            );
                            if should_ref {
                                if let Expression::Cast { .. } = arg_to_generate {
                                    arg_str = format!("&({})", arg_str);
                                } else {
                                    arg_str = format!("&{}", arg_str);
                                }
                            }
                        }

                        let sig_param_idx_str_field = method_signature.as_ref().map(|sig| {
                            if sig.has_self_receiver {
                                i + 1
                            } else {
                                i
                            }
                        });
                        if let Some(idx) = sig_param_idx_str_field {
                            arg_str = self.ensure_ref_for_owned_string_field_when_callee_expects_str(
                                &method_signature,
                                idx,
                                arg_to_generate,
                                arg_str,
                                string_literal_converted,
                            );
                        }

                        // AUTO-BORROW: Methods that take &T or &[T] should auto-borrow
                        // when given owned values. Eliminates Rust leakage in .wj files.
                        let auto_borrow_methods = ["push_str", "extend_from_slice"];
                        let map_key_methods = ["remove", "get", "contains_key", "entry"];
                        let is_map_method = map_key_methods.contains(&method.as_str())
                            && i == 0
                            && {
                                let obj_ty = self.infer_expression_type(object);
                                obj_ty.as_ref().is_some_and(|t| matches!(t,
                                    Type::Parameterized(base, _) if base == "HashMap" || base == "BTreeMap" || base == "Map"
                                ))
                            };
                        if (auto_borrow_methods.contains(&method.as_str()) || is_map_method) && i == 0 {
                            let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                            let arg_already_ref = {
                                let arg_ty = self.infer_expression_type(arg);
                                let ty_is_ref = arg_ty.as_ref().is_some_and(|t| matches!(t,
                                    Type::Reference(_) | Type::MutableReference(_)
                                ) || matches!(t, Type::Custom(n) if n == "&str"));
                                let param_is_borrowed = match arg {
                                    Expression::Identifier { name, .. } =>
                                        self.inferred_borrowed_params.contains(&name.to_string()),
                                    _ => false,
                                };
                                ty_is_ref || param_is_borrowed
                            };
                            if !is_string_literal && !arg_str.starts_with('&') && !arg_already_ref {
                                let needs_borrow = matches!(arg,
                                    Expression::Identifier { .. } |
                                    Expression::FieldAccess { .. } |
                                    Expression::MethodCall { .. }
                                );
                                if needs_borrow {
                                    arg_str = format!("&{}", arg_str);
                                }
                            }
                        }

                        // AUTO-CAST int → float: when parameter expects f32/f64 but argument is int
                        // Skip when signature has a collision (different types with same name).
                        {
                            let effective_sig = method_signature.as_ref()
                                .or_else(|| self.signature_registry.get_signature(method));
                            let has_collision = self.signature_registry.has_collision(method);
                            if let Some(sig) = effective_sig {
                                let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                                if !has_collision {
                                    if let Some(param_ty) = sig.param_types.get(sig_param_idx) {
                                        let param_is_f32 = matches!(param_ty, Type::Custom(n) if n == "f32");
                                        let param_is_f64 = matches!(param_ty, Type::Custom(n) if n == "f64");
                                        if param_is_f32 || param_is_f64 {
                                            let arg_ty = self.infer_expression_type(arg);
                                            let arg_is_int = arg_ty.as_ref().is_some_and(|t| {
                                                matches!(t, Type::Int)
                                                    || matches!(t, Type::Custom(n) if crate::type_classification::is_integer_type(n))
                                            });
                                            if arg_is_int && !arg_str.contains(" as f32") && !arg_str.contains(" as f64") {
                                                let target = if param_is_f32 { "f32" } else { "f64" };
                                                arg_str = if arg_str.contains(' ') || matches!(arg, Expression::Binary { .. }) {
                                                    format!("({}) as {}", arg_str, target)
                                                } else {
                                                    format!("{} as {}", arg_str, target)
                                                };
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Restore suppress flag
                        self.suppress_borrowed_clone = prev_suppress;

                        arg_str
                    })
                    .collect();

                // E0499 FIX: Extract temporaries when receiver and arguments both borrow self.
                // Pattern: self.field.method(self.other_method()) generates two &mut self borrows.
                // Fix: { let __wj_tmp0 = self.other_method(); self.field.method(__wj_tmp0) }
                let receiver_borrows_self = self.codegen_expression_traces_to_self(object);
                let mut self_borrow_temps: Vec<(String, String)> = Vec::new();
                let args = if receiver_borrows_self {
                    let needs_extraction = arguments.iter().any(|(_label, arg)| self.expression_borrows_self(arg));
                    if needs_extraction {
                        args.into_iter()
                            .enumerate()
                            .map(|(i, arg_str)| {
                                let (_label, arg_expr) = &arguments[i];
                                if self.expression_borrows_self(arg_expr) {
                                    let temp_name = format!("__wj_tmp{}", i);
                                    self_borrow_temps.push((temp_name.clone(), arg_str));
                                    temp_name
                                } else {
                                    arg_str
                                }
                            })
                            .collect()
                    } else {
                        args
                    }
                } else {
                    args
                };

                // Restore float target type after argument generation
                self.assignment_float_target_type = prev_float_target;

                // Generate turbofish if present, or infer for collect() from return type
                let turbofish = if let Some(types) = type_args {
                    let type_strs: Vec<String> =
                        types.iter().map(|t| self.type_to_rust(t)).collect();
                    format!("::<{}>", type_strs.join(", "))
                } else if method == "collect" {
                    if let Some(target_ty) = &self.collect_target_type {
                        format!("::<{}>", self.type_to_rust(target_ty))
                    } else if let Some(ret_ty) = &self.current_function_return_type {
                        format!("::<{}>", self.type_to_rust(ret_ty))
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                // Special case: empty method name means turbofish on a function call (func::<T>())
                if method.is_empty() {
                    return format!("{}{}({})", obj_str, turbofish, args.join(", "));
                }

                // Special case: substring(start, end) -> &text[start..end]
                if method == "substring" && args.len() == 2 {
                    return format!("&{}[{}..{}]", obj_str, args[0], args[1]);
                }

                // Special case: contains() with String argument needs .as_str()
                // String::contains() expects &str, not String
                if method == "contains" && args.len() == 1 {
                    // Check if argument is a method call that returns String (like to_lowercase())
                    if let Some((_label, arg)) = arguments.first() {
                        if matches!(arg, Expression::MethodCall { method: m, .. } if
                            m == "to_lowercase" || m == "to_uppercase" ||
                            m == "to_string" || m == "trim" || m == "clone")
                        {
                            // The argument is String, needs .as_str()
                            return format!("{}.{}({}.as_str())", obj_str, method, args[0]);
                        }
                    }
                }

                // Determine separator: :: for static calls, . for instance methods
                // - Type/Module (starts with uppercase): use ::
                // - Variable (starts with lowercase): use .
                let separator = match &**object {
                    Expression::Call { .. } | Expression::MethodCall { .. } => ".", // Instance method on return value
                    Expression::Identifier { name, .. } => {
                        // Check for known module/crate names that should use ::
                        // Note: Avoid common variable names like "path", "config" which are used as variables
                        let known_modules = [
                            "std",
                            "serde_json",
                            "serde",
                            "tokio",
                            "reqwest",
                            "sqlx",
                            "chrono",
                            "sha2",
                            "bcrypt",
                            "base64",
                            "rand",
                            "Vec",
                            "String",
                            "Option",
                            "Result",
                            "Box",
                            "Arc",
                            "Mutex",
                            "Utc",
                            "Local",
                            "DEFAULT_COST",
                            // Stdlib modules (avoid common variable names)
                            "mime",
                            "http",
                            "fs",
                            "strings",
                            // NOTE: "json" removed - it's a common variable name!
                            // Use "serde_json" for the module instead
                            "regex",
                            "cli",
                            "log",
                            "crypto",
                            "io",
                            "env",
                            "time",
                            "sync",
                            "thread",
                            "collections",
                            "cmp",
                        ];

                        // Type or module (uppercase) vs variable (lowercase)
                        if name.chars().next().is_some_and(|c| c.is_uppercase())
                            || name.contains('.')
                            || known_modules.contains(&name.as_str())
                        {
                            "::" // Vec::new(), std::fs::read(), serde_json::to_string()
                        } else {
                            "." // x.abs(), value.method()
                        }
                    }
                    Expression::FieldAccess { ref object, .. } => {
                        // Check if this is a module path (e.g., std::fs) or a field access (e.g., self.count)
                        // If the object is an identifier that looks like a module, use ::
                        // Otherwise, use . for instance methods on fields
                        match object {
                            Expression::Identifier { name, .. } => {
                                if name.chars().next().is_some_and(|c| c.is_uppercase())
                                    || name == "std"
                                {
                                    "::" // Module::path::method() -> static method
                                } else {
                                    "." // self.field.method() or variable.field.method() -> instance method
                                }
                            }
                            _ => ".", // Default to instance method
                        }
                    }
                    _ => ".", // Instance method on expressions
                };

                // SPECIAL CASE: .slice() method is our desugared slice syntax [start..end]
                // Convert it back to proper Rust slice syntax
                // For strings, we need to add & to get &str (a reference)
                if method == "slice" && args.len() == 2 {
                    return format!("&{}[{}..{}]", obj_str, args[0], args[1]);
                }

                // E0308: Borrowed Windjammer `string` parameters lower to `&str`. `.clone()` on `&str`
                // is still `&str`, but users mean an owned copy → emit `.to_string()`.
                if method == "clone" && arguments.is_empty() {
                    if let Expression::Identifier { name, .. } = &**object {
                        if self.inferred_borrowed_params.contains(name.as_str())
                            && self
                                .current_function_params
                                .iter()
                                .find(|p| p.name == *name)
                                .is_some_and(|p| {
                                    crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                                })
                        {
                            return format!("{}.to_string()", obj_str);
                        }
                    }
                }

                // PHASE 2 OPTIMIZATION: Eliminate unnecessary .clone() calls
                // DISABLED: This optimization was too aggressive and removed needed clones
                // TODO: Make this more conservative - only remove clone when we can prove
                // the value is Copy or when it's the last use
                // if method == "clone" && arguments.is_empty() {
                //     if let Expression::Identifier { name: ref var_name, location: None } = **object {
                //         if self.clone_optimizations.contains(var_name) {
                //             // Skip the .clone(), just return the variable (or borrow if needed)
                //             return obj_str;
                //         }
                //     }
                // }

                // UI FRAMEWORK: Check if we need to add .to_vnode() for .child() methods
                // DISABLED: Too aggressive - needs type checking to determine if parameter expects VNode
                // TODO: Re-enable with proper type checking when VNode type bindings are implemented
                let processed_args = args;

                // WINDJAMMER STDLIB → RUST TRANSLATION
                // Some Windjammer methods don't exist in Rust and need translation.
                //
                // reversed() → into_iter().rev().collect::<Vec<_>>()
                if method == "reversed" && processed_args.is_empty() {
                    return format!("{}.into_iter().rev().collect::<Vec<_>>()", obj_str);
                }
                // enumerate() → iter().enumerate()
                // Rust Vec doesn't have .enumerate() — only iterators do.
                // But if the object already ends with .iter(), .iter_mut(), or
                // .into_iter(), don't add a redundant .iter() prefix.
                if method == "enumerate" && processed_args.is_empty() {
                    let already_iterator = obj_str.ends_with(".iter()")
                        || obj_str.ends_with(".iter_mut()")
                        || obj_str.ends_with(".into_iter()");
                    if already_iterator {
                        return format!("{}.enumerate()", obj_str);
                    } else {
                        return format!("{}.iter().enumerate()", obj_str);
                    }
                }

                // TDD FIX (Bug #3): Extract format!() macros in method arguments too
                let has_format_arg = processed_args
                    .iter()
                    .any(|arg_str| arg_str.contains("format!("));

                let base_expr = if has_format_arg {
                    // Extract format!() macros to temp variables
                    let mut temp_decls = String::new();
                    let mut temp_counter = 0;
                    let fixed_args: Vec<String> = processed_args
                        .iter()
                        .map(|arg_str| {
                            if arg_str.starts_with("format!(") || arg_str.starts_with("&format!(") {
                                // Strip leading & if present (was added by argument processing)
                                let format_expr = if arg_str.starts_with("&") {
                                    arg_str.strip_prefix("&").unwrap()
                                } else {
                                    arg_str
                                };
                                // Extract to temp var
                                let temp_name = format!("_temp{}", temp_counter);
                                temp_counter += 1;
                                temp_decls
                                    .push_str(&format!("let {} = {}; ", temp_name, format_expr));

                                // When the method expects &str (push_str, extend_from_slice),
                                // add & to pass borrowed temp. Otherwise, pass owned value.
                                let method_needs_borrow =
                                    matches!(method.as_str(), "push_str" | "extend_from_slice");
                                if arg_str.starts_with("&") || method_needs_borrow {
                                    format!("&{}", temp_name)
                                } else {
                                    temp_name
                                }
                            } else {
                                arg_str.clone()
                            }
                        })
                        .collect();

                    // Wrap in block: { let _temp0 = format!(...); obj.method(&_temp0, ...) }
                    format!(
                        "{{ {}{}{}{}{}({}) }}",
                        temp_decls,
                        obj_str,
                        separator,
                        method,
                        turbofish,
                        fixed_args.join(", ")
                    )
                } else {
                    format!(
                        "{}{}{}{}({})",
                        obj_str,
                        separator,
                        method,
                        turbofish,
                        processed_args.join(", ")
                    )
                };

                // E0499 FIX: Wrap in block with temporaries if self-borrow extraction was needed
                let base_expr = if !self_borrow_temps.is_empty() {
                    let mut temp_decls = String::new();
                    for (name, value) in &self_borrow_temps {
                        temp_decls.push_str(&format!("let {} = {}; ", name, value));
                    }
                    format!("{{ {}{} }}", temp_decls, base_expr)
                } else {
                    base_expr
                };

                base_expr
            }
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

    /// Generate code for identifier expression
    /// Handles:
    /// - Implicit self.field access (in impl blocks)
    /// - Auto-clone analysis for variables
    /// - Copy type optimization (skip .clone())
    /// - Reference dereferencing (*ref for Copy types)

    /// Generate code for try operator expression (expr?)

    /// Try to generate a test macro call (assert_eq!, panic!, vec!, etc.)
    /// Returns Some(code) if this is a test macro, None otherwise


    /// Try to convert print/println/eprintln/eprint to macros
    /// Returns Some(code) if this is a print function, None otherwise

    /// Generate code for block expression ({ ... })
    /// Handles:
    /// - Unsafe blocks (unsafe { ... })
    /// - Match expression optimization (single-statement blocks with match)
    /// - If-let pattern detection
    /// - String literal auto-conversion in match arms
    /// - Implicit returns for last expression

    /// Generate code for await expression (expr.await)

    /// Generate code for channel send expression (channel.send(value))

    /// Generate code for channel receive expression (channel.recv())

    /// Generate code for range expression (start..end or start..=end)
    /// TDD FIX: Range type unification for 0..vec.len()

    /// Generate code for tuple expression

    /// Generate code for cast expression (expr as Type)
    /// E0606 FIX: Cannot cast &T as U - auto-deref borrowed parameters first

    /// Generate code for map literal expression
    /// Produces HashMap::new() for empty maps, HashMap::from([...]) for non-empty

    /// Generate code for unary expression (!expr, -expr, *expr, &expr, &mut expr)

    /// Generate code for macro invocation expression
    /// Handles format!, println!, vec!, and other macros with special semantics

    /// Generate code for field access expression (object.field)
    /// Handles module paths (::), auto-clone for non-Copy fields, borrowed iterators

    /// Generate code for struct literal expression Struct { field: value }
    /// Handles string coercion, field shorthand, auto-clone for borrowed self

    /// Generate code for index expression array[index]
    /// Handles auto-cast to usize, slice syntax, auto-borrow/clone for non-Copy elements

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
            Literal::String(s) => {
                if s.is_empty() && self.should_coerce_string_literal_to_owned() {
                    // Use `"".to_string()` (not `String::new()`) so implicit-return / match-arm
                    // post-processing does not append another `.to_string()` (E0308 / redundant call).
                    "\"\".to_string()".to_string()
                } else {
                    let base = crate::codegen::rust::literals::generate_literal(lit);
                    if self.should_coerce_string_literal_to_owned() {
                        format!("{}.to_string()", base)
                    } else {
                        base
                    }
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
                            let from_assignment = self
                                .assignment_float_target_type
                                .as_ref()
                                .and_then(float_type_utilities::float_literal_suffix_from_assignment_lhs);
                            let from_struct_field = if let (Some(struct_name), Some(field_name)) = (
                                &self.current_struct_literal_name,
                                &self.current_struct_field_name,
                            ) {
                                self.lookup_struct_field_types(struct_name)
                                    .and_then(|fields| fields.get(field_name))
                                    .map(|ft| float_type_utilities::extract_float_type_from_context(ft))
                            } else {
                                None
                            };
                            let from_return = self
                                .current_function_return_type
                                .as_ref()
                                .map(|rt| float_type_utilities::extract_float_type_from_context(rt));
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

    /// Old context-sensitive approach (fallback when inference not available)

    /// Generate literal without expression context (used in older code paths)

    /// Generate efficient string concatenation using format! macro

    /// `f32`/`f64` classification for binary operand codegen (inference + casts + WJ types).
    /// Used for E0507 Option::map fix - self.children.map() needs .as_ref()
    fn codegen_expression_traces_to_self(&self, expr: &Expression) -> bool {
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
    fn expression_borrows_self(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier { name, .. } => name == "self",
            Expression::FieldAccess { object, .. } => self.expression_borrows_self(object),
            Expression::Index { object, .. } => self.expression_borrows_self(object),
            Expression::MethodCall { object, arguments, .. } => {
                self.expression_borrows_self(object)
                    || arguments.iter().any(|(_, arg)| self.expression_borrows_self(arg))
            }
            Expression::Call { arguments, function, .. } => {
                self.expression_borrows_self(function)
                    || arguments.iter().any(|(_, arg)| self.expression_borrows_self(arg))
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
    pub(in crate::codegen::rust) fn field_access_root_is_behind_reference(&self, expr: &Expression) -> bool {
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
