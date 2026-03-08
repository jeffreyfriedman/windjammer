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

use super::arm_string_analysis;
use super::ast_utilities;
use super::constant_folding;
use super::expression_helpers;
use super::operators;
use super::pattern_analysis;
use super::string_analysis;
use super::CodeGenerator;

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {
    // Helper method for expressions that need to be evaluated without &mut self
    pub(crate) fn generate_expression_immut(&self, expr: &Expression) -> String {
        use crate::parser::ast::operators::{BinaryOp, UnaryOp};

        match expr {
            Expression::Literal { value: lit, .. } => self.generate_literal(lit),
            Expression::Identifier { name, .. } => name.clone(),
            Expression::Unary { op, operand, .. } => {
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
                let left_str = self.generate_expression_immut(left);
                let right_str = self.generate_expression_immut(right);

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
                let obj_str = self.generate_expression_immut(object);
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
                let args_str = arguments
                    .iter()
                    .map(|(_label, arg)| self.generate_expression_immut(arg))
                    .collect::<Vec<_>>()
                    .join(", ");
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

    /// Check if match needs .clone() to avoid partial move from self
    fn match_needs_clone_for_self_field(
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

    fn generate_expression_with_precedence(&mut self, expr: &Expression<'ast>) -> String {
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
            Expression::Literal { value: lit, .. } => self.generate_literal(lit),
            Expression::Identifier { name, .. } => {
                // Qualified paths use :: from parser (e.g., std::fs::read)
                // Simple identifiers: variable_name -> variable_name
                // Check if this is a struct field and we're in an impl block
                // BUT: Don't apply implicit field access if:
                // 1. It's a parameter name (parameters shadow fields)
                // 2. It's a local variable (local vars shadow fields)
                let is_parameter = self.current_function_params.iter().any(|p| p.name == *name);
                let is_local_variable = self
                    .local_variable_scopes
                    .iter()
                    .any(|scope| scope.contains(name));

                let base_name = if self.in_impl_block
                    && !is_parameter
                    && !is_local_variable  // NEW: Local variables shadow fields!
                    && self.current_struct_fields.contains(name)
                {
                    format!("self.{}", name)
                } else {
                    name.clone()
                };

                // AUTO-CLONE: Check if this variable needs to be cloned at this point
                // CRITICAL: Never clone assignment targets (left side of `=`)
                // DOUBLE-CLONE FIX: Skip auto-clone when inside an explicit .clone() call
                if !self.generating_assignment_target && !self.in_explicit_clone_call {
                    if let Some(ref analysis) = self.auto_clone_analysis {
                        if analysis
                            .needs_clone(name, self.current_statement_idx)
                            .is_some()
                        {
                            // Skip .clone() for Copy types — they are implicitly copied,
                            // so .clone() is unnecessary noise.
                            let is_copy_type = analysis.string_literal_vars.contains(name)
                                || self.usize_variables.contains(name)
                                || self
                                    .infer_expression_type(expr_to_generate)
                                    .as_ref()
                                    .is_some_and(|t| self.is_type_copy(t));

                            if !is_copy_type {
                                // Automatically insert .clone() - this is the magic!
                                return format!("{}.clone()", base_name);
                            }
                        }
                    }
                }

                base_name
            }
            Expression::Binary {
                left, op, right, ..
            } => {
                // TDD FIX: Optimize .len() comparisons to .is_empty()
                // Clippy warns about .len() == 0, .len() != 0, .len() > 0
                // Transform to .is_empty() or !.is_empty()
                if let Expression::MethodCall {
                    object,
                    method,
                    arguments,
                    ..
                } = left
                {
                    if method == "len" && arguments.is_empty() {
                        // Check if comparing to 0
                        if let Expression::Literal {
                            value: Literal::Int(0),
                            ..
                        } = right
                        {
                            match op {
                                BinaryOp::Eq => {
                                    // .len() == 0 → .is_empty()
                                    let obj_str = self.generate_expression(object);
                                    return format!("{}.is_empty()", obj_str);
                                }
                                BinaryOp::Ne | BinaryOp::Gt => {
                                    // .len() != 0 → !.is_empty()
                                    // .len() > 0 → !.is_empty()
                                    let obj_str = self.generate_expression(object);
                                    return format!("!{}.is_empty()", obj_str);
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Special handling for string concatenation
                if matches!(op, BinaryOp::Add) {
                    // Only treat as string concat if at least one operand is definitely a string literal
                    let has_string_literal = matches!(
                        left,
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    ) || matches!(
                        right,
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    ) || string_analysis::contains_string_literal(left)
                        || string_analysis::contains_string_literal(right);

                    if has_string_literal {
                        // For string concatenation, use format! macro for clean, efficient code
                        return self.generate_string_concat(left, right);
                    }
                }

                // Check for usize/i32 comparison or arithmetic - cast if needed
                let is_comparison = matches!(
                    op,
                    BinaryOp::Lt
                        | BinaryOp::Le
                        | BinaryOp::Gt
                        | BinaryOp::Ge
                        | BinaryOp::Eq
                        | BinaryOp::Ne
                );
                let is_arithmetic = matches!(
                    op,
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div
                );
                let left_is_usize = self.expression_produces_usize(left);
                let right_is_usize = self.expression_produces_usize(right);
                let right_is_int_literal = matches!(
                    right,
                    Expression::Literal {
                        value: Literal::Int(_),
                        ..
                    }
                );
                let left_is_int_literal = matches!(
                    left,
                    Expression::Literal {
                        value: Literal::Int(_),
                        ..
                    }
                );

                // COMPARISON CLONE SUPPRESSION: For comparison operators (==, !=, <, >, etc.),
                // suppress borrowed-iterator cloning on operands. Comparisons work on references
                // in Rust (&String == &String, &T == &T via PartialEq), so cloning is unnecessary.
                // e.g., `recipe.name.clone() == target` → `recipe.name == target`
                let prev_suppress = self.suppress_borrowed_clone;
                if is_comparison {
                    self.suppress_borrowed_clone = true;
                }

                // Wrap operands in parens if they have lower precedence
                let mut left_str = match left {
                    Expression::Binary { op: left_op, .. } => {
                        if operators::op_precedence(left_op) < operators::op_precedence(op) {
                            format!("({})", self.generate_expression(left))
                        } else {
                            self.generate_expression(left)
                        }
                    }
                    _ => self.generate_expression(left),
                };
                let mut right_str = match right {
                    Expression::Binary { op: right_op, .. } => {
                        if operators::op_precedence(right_op) < operators::op_precedence(op) {
                            format!("({})", self.generate_expression(right))
                        } else {
                            self.generate_expression(right)
                        }
                    }
                    _ => self.generate_expression(right),
                };

                // Restore previous suppress state
                self.suppress_borrowed_clone = prev_suppress;

                // WINDJAMMER PHILOSOPHY: Auto-cast int/usize in comparisons
                // When comparing int (i64) with usize, automatically cast to make it work.
                //
                // CORRECTNESS: Always cast the usize side to i64, NOT the int side to usize.
                // Casting i64 → usize is UNSAFE for negative values (wraps to huge number).
                // Casting usize → i64 is SAFE (vec lengths always fit in i64).
                //
                // For int literals compared to usize: cast literal to usize (always non-negative).
                // For int variables compared to usize: cast usize to i64 (preserves negative semantics).
                //
                // Example: items.len() >= 10 → items.len() >= 10usize (literal, always safe)
                // Example: index >= items.len() → index >= (items.len() as i64) (safe cast)
                //
                // IMPORTANT: Wrap the cast operand in ((...) as i64) to handle compound
                // expressions like `width * height` → ((width * height) as i64), not
                // (width * (height as i64)) which would have wrong precedence.
                if is_comparison && left_is_usize && !right_is_usize {
                    // Left is usize, right is NOT usize
                    if right_is_int_literal {
                        // Int literals in comparisons with usize don't need explicit cast —
                        // Rust infers the literal type from context. `vec.len() > 0` is fine.
                    } else {
                        // Cast the usize side (LEFT) to i64 for safety
                        // Use parens around compound expressions to prevent precedence issues
                        // because `as` has higher precedence than arithmetic:
                        // `a + b as i64` → `a + (b as i64)` (wrong), need `(a + b) as i64`
                        let needs_inner_parens = matches!(left, Expression::Binary { .. });
                        if needs_inner_parens {
                            left_str = format!("({}) as i64", left_str);
                        } else {
                            left_str = format!("{} as i64", left_str);
                        }
                    }
                } else if is_comparison && right_is_usize && !left_is_usize {
                    // Right is usize, left is NOT usize
                    if left_is_int_literal {
                        // Int literals in comparisons with usize don't need explicit cast —
                        // Rust infers the literal type from context.
                    } else {
                        // Cast the usize side (RIGHT) to i64 for safety
                        // Use parens around compound expressions to prevent precedence issues
                        let needs_inner_parens = matches!(right, Expression::Binary { .. });
                        if needs_inner_parens {
                            right_str = format!("({}) as i64", right_str);
                        } else {
                            right_str = format!("{} as i64", right_str);
                        }
                    }
                }
                // If both are usize: no cast (usize == usize is fine)
                // If neither is usize: no cast (i64 == i64 is fine)

                // AUTO-CAST: When doing arithmetic between usize and int literal, Rust infers
                // the literal type from context. So `items.len() - 1` works without casting.
                // Only cast if the literal is negative (usize can't represent negative values).
                if is_arithmetic && left_is_usize && right_is_int_literal && !right_is_usize {
                    let is_negative = matches!(right, Expression::Literal { value: Literal::Int(n), .. } if *n < 0);
                    if is_negative {
                        right_str = format!("{} as usize", right_str);
                    }
                } else if is_arithmetic && right_is_usize && left_is_int_literal && !left_is_usize {
                    let is_negative = matches!(left, Expression::Literal { value: Literal::Int(n), .. } if *n < 0);
                    if is_negative {
                        left_str = format!("{} as usize", left_str);
                    }
                }

                let op_str = operators::binary_op_to_rust(op);

                // TDD FIX: Rust parses `expr as usize < y` as `expr as usize<y>` (generics).
                // When the left operand is a cast (or ends with `as TYPE`) and the operator
                // is `<`, we must wrap the left side in parentheses to disambiguate.
                // Other comparison operators (>=, <=, ==, !=, >) don't have this ambiguity.
                //
                // TDD FIX (VOXEL DOGFOODING): Bitwise operators (<<, >>, |, &, ^) have
                // LOWER precedence than `as` in Rust, so `(x as u32) << 8` is required.
                // Without parens: `x as u32 << 8` is parsed as `x as (u32 << 8)` - WRONG!
                //
                // DISCOVERED: VoxelColor::to_hex() compilation failure
                //   Source: `let r = (self.r as u32) << 24;`
                //   Generated: `let r = self.r as u32 << 24;`  ← Missing parens!
                //   Error: `<<` is interpreted as start of generic arguments for `u32`
                let needs_cast_parens_for_op =
                    matches!(op_str, "<" | ">" | "<<" | ">>" | "|" | "&" | "^");
                let left_needs_cast_parens = needs_cast_parens_for_op
                    && (matches!(left, Expression::Cast { .. }) || left_str.contains(" as "));
                let right_needs_cast_parens = needs_cast_parens_for_op
                    && (matches!(right, Expression::Cast { .. }) || right_str.contains(" as "));

                if left_needs_cast_parens {
                    left_str = format!("({})", left_str);
                }
                if right_needs_cast_parens {
                    right_str = format!("({})", right_str);
                }

                // TDD FIX: Smart XOR deref logic for string comparisons
                // Check if BOTH sides are borrowed, or only ONE side is borrowed
                //
                // Rules:
                // - Both borrowed (&String == &String): NO deref (PartialEq<&T> works)
                // - Both owned (String == String): NO deref (PartialEq<T> works)
                // - One borrowed, one owned: Add * to borrowed side (XOR)
                //
                // Borrowed sources:
                // - Identifier in inferred_borrowed_params (function parameters like `name: &String`)
                // - Identifier in borrowed_iterator_vars (for-loop variables like `for item in items.iter()`)
                // - MethodCall returning &str (e.g., `t.as_str()` returns `&str`)
                //
                // Owned sources (everything else):
                // - FieldAccess (e.g., `m.id` where `m: &Member` → `String`)
                // - Literal values
                // - Method calls returning owned types

                // TDD FIX: Check if parameter is &str type (never needs deref in comparisons)
                let is_str_param = |name: &str| {
                    self.current_function_params.iter().any(|p| {
                        p.name == name
                            && (matches!(p.type_, crate::parser::Type::String)
                                || matches!(p.type_, crate::parser::Type::Custom(ref n) if n == "string"))
                            && self.inferred_borrowed_params.contains(name)
                    })
                };

                let left_is_borrowed = match left {
                    Expression::Identifier { name, .. } => {
                        // Don't treat &str params as "borrowed" for deref purposes
                        !is_str_param(name)
                            && (self.inferred_borrowed_params.contains(name.as_str())
                                || self.borrowed_iterator_vars.contains(name))
                    }
                    Expression::MethodCall { method, .. } => {
                        // Methods like .as_str() return &str (borrowed)
                        method == "as_str"
                    }
                    _ => false, // FieldAccess, Literal, etc. are owned
                };

                let right_is_borrowed = match right {
                    Expression::Identifier { name, .. } => {
                        // Don't treat &str params as "borrowed" for deref purposes
                        !is_str_param(name)
                            && (self.inferred_borrowed_params.contains(name.as_str())
                                || self.borrowed_iterator_vars.contains(name))
                    }
                    Expression::MethodCall { method, .. } => {
                        // Methods like .as_str() return &str (borrowed)
                        method == "as_str"
                    }
                    _ => false, // FieldAccess, Literal, etc. are owned
                };

                // XOR: Add deref only if exactly ONE side is borrowed
                if left_is_borrowed != right_is_borrowed {
                    if left_is_borrowed {
                        // &String == String → *&String == String
                        left_str = format!("*{}", left_str);
                    } else {
                        // String == &String → String == *&String
                        right_str = format!("*{}", right_str);
                    }
                }
                // If both borrowed OR both owned: NO deref needed

                format!("{} {} {}", left_str, op_str, right_str)
            }
            Expression::Unary { op, operand, .. } => {
                let op_str = operators::unary_op_to_rust(op);

                // BORROW CONTEXT: When generating &expr or &mut expr, suppress Vec index
                // auto-clone in the operand. We want a reference to the original element.
                // e.g., &self.items[i] → NOT &self.items[i].clone()
                //        &mut self.items[i] → NOT &mut self.items[i].clone()
                let is_borrow = matches!(
                    op,
                    crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef
                );
                let prev_borrow = self.in_borrow_context;
                if is_borrow {
                    self.in_borrow_context = true;
                }
                let operand_str = self.generate_expression(operand);
                self.in_borrow_context = prev_borrow;

                // CRITICAL: Preserve parentheses for binary expressions in unary context
                // !(a || b) should generate !(a || b), not !a || b
                // Binary operators have lower precedence than unary operators, so we need parens
                let needs_parens = matches!(&**operand, Expression::Binary { .. });

                if needs_parens {
                    format!("{}({})", op_str, operand_str)
                } else {
                    format!("{}{}", op_str, operand_str)
                }
            }
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
                let is_user_defined = self
                    .signature_registry
                    .get_signature(&func_name)
                    .map(|sig| !sig.is_extern)
                    .unwrap_or(false);

                if !is_user_defined {
                    // Special case: convert test assertion functions to macros
                    // THE WINDJAMMER WAY: assert_eq(a, b) -> assert_eq!(a, b)
                    // NOTE: assert_gt, assert_gte, assert_is_some, assert_is_none, etc. are runtime functions, not macros
                    // Print functions need special handling (format! unwrapping, interpolation)
                    // so they are NOT in the simple macro list — handled separately below.
                    let test_macros = [
                        "assert",
                        "assert_eq",
                        "assert_ne",
                        "assert_ok",
                        "assert_err",
                        "panic",
                        "vec",
                        "format",
                        "write",
                        "writeln",
                        "dbg",
                        "todo",
                        "unimplemented",
                        "unreachable",
                    ];

                    if test_macros.contains(&func_name.as_str()) {
                        let args: Vec<String> = arguments
                            .iter()
                            .map(|(_label, arg)| self.generate_expression(arg))
                            .collect();
                        return format!("{}!({})", func_name, args.join(", "));
                    }

                    // Special case: qualify test assertion runtime functions
                    // THE WINDJAMMER WAY: These are functions, not macros, so they need proper paths
                    let test_functions = [
                        "assert_gt",
                        "assert_lt",
                        "assert_gte",
                        "assert_lte",
                        "assert_approx",
                        "assert_not_empty",
                        "assert_empty",
                        "assert_contains",
                        "assert_is_some",
                        "assert_is_none",
                    ];

                    if test_functions.contains(&func_name.as_str()) {
                        let args: Vec<String> = arguments
                            .iter()
                            .enumerate()
                            .map(|(idx, (_label, arg))| {
                                let generated = self.generate_expression(arg);
                                // assert_is_some and assert_is_none expect &Option, so add & for first arg
                                if (func_name == "assert_is_some" || func_name == "assert_is_none")
                                    && idx == 0
                                {
                                    format!("&{}", generated)
                                } else {
                                    generated
                                }
                            })
                            .collect();
                        return format!(
                            "windjammer_runtime::test::{}({})",
                            func_name,
                            args.join(", ")
                        );
                    }
                }

                // Special case: convert print/println/eprintln/eprint() to macros
                if func_name == "print"
                    || func_name == "println"
                    || func_name == "eprintln"
                    || func_name == "eprint"
                {
                    let macro_name = func_name.clone();

                    // For print() -> println!(), otherwise keep the same name
                    let target_macro = if macro_name == "print" {
                        "println".to_string()
                    } else {
                        macro_name.clone()
                    };
                    // Check if the first argument is a format! macro (from string interpolation)
                    if let Some((_, first_arg)) = arguments.first() {
                        // Check for MacroInvocation (explicit format! calls)
                        // first_arg is &&Expression (ref to ref from Vec element), deref both
                        if let Expression::MacroInvocation {
                            is_repeat: _,
                            ref name,
                            args: ref macro_args,
                            ..
                        } = **first_arg
                        {
                            if name == "format" && !macro_args.is_empty() {
                                // Unwrap the format! call and put its arguments directly into println!
                                // format!("text {}", var) -> println!("text {}", var)
                                let format_str = self.generate_expression(macro_args[0]);
                                let format_args: Vec<String> = macro_args[1..]
                                    .iter()
                                    .map(|arg| self.generate_expression(arg))
                                    .collect();

                                let args_str = if format_args.is_empty() {
                                    String::new()
                                } else {
                                    format!(", {}", format_args.join(", "))
                                };

                                return format!("{}!({}{})", target_macro, format_str, args_str);
                            }
                        }

                        // Check for Binary expression with string concatenation (will become format!)
                        if let Expression::Binary {
                            left,
                            op: BinaryOp::Add,
                            right,
                            ..
                        } = **first_arg
                        {
                            // Check if this is string concatenation
                            let has_string_literal =
                                matches!(
                                    left,
                                    Expression::Literal {
                                        value: Literal::String(_),
                                        ..
                                    }
                                ) || matches!(
                                    right,
                                    Expression::Literal {
                                        value: Literal::String(_),
                                        ..
                                    }
                                ) || string_analysis::contains_string_literal(left)
                                    || string_analysis::contains_string_literal(right);

                            if has_string_literal {
                                // Collect all parts of the concatenation
                                let mut parts = Vec::new();
                                string_analysis::collect_concat_parts_static(left, &mut parts);
                                string_analysis::collect_concat_parts_static(right, &mut parts);

                                // Generate format string and arguments
                                let format_str = "{}".repeat(parts.len());
                                let format_args: Vec<String> = parts
                                    .iter()
                                    .map(|expr| self.generate_expression(expr))
                                    .collect();

                                return format!(
                                    "{}!(\"{}\", {})",
                                    target_macro,
                                    format_str,
                                    format_args.join(", ")
                                );
                            }
                        }

                        // Check if the first argument is a string literal with ${} (old-style, shouldn't happen but keep for safety)
                        if let Expression::Literal {
                            value: Literal::String(ref s),
                            ..
                        } = **first_arg
                        {
                            if s.contains("${") {
                                // Handle string interpolation directly in println!
                                // Convert "${var}" to "{}" and extract variables
                                let mut format_str = String::new();
                                let mut args = Vec::new();
                                let mut chars = s.chars().peekable();

                                while let Some(ch) = chars.next() {
                                    if ch == '$' && chars.peek() == Some(&'{') {
                                        chars.next(); // consume {
                                        let mut var_name = String::new();

                                        while let Some(&next_ch) = chars.peek() {
                                            if next_ch == '}' {
                                                chars.next(); // consume }
                                                break;
                                            } else {
                                                var_name.push(next_ch);
                                                chars.next();
                                            }
                                        }

                                        if !var_name.is_empty() {
                                            format_str.push_str("{}");
                                            // Check if this is a struct field
                                            if self.in_impl_block
                                                && self.current_struct_fields.contains(&var_name)
                                            {
                                                args.push(format!("self.{}", var_name));
                                            } else {
                                                args.push(var_name);
                                            }
                                        }
                                    } else {
                                        format_str.push(ch);
                                    }
                                }

                                let args_str = if args.is_empty() {
                                    String::new()
                                } else {
                                    format!(", {}", args.join(", "))
                                };

                                return format!(
                                    "{}!(\"{}\"{})",
                                    target_macro,
                                    format_str.replace('\\', "\\\\").replace('"', "\\\""),
                                    args_str
                                );
                            }
                        }
                    }

                    // No interpolation, just regular print
                    // TDD FIX: Auto-format non-string arguments
                    // println(value) where value: bool → println!("{}", value)
                    // println("text") → println!("text") (string literals stay as-is)
                    let args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| self.generate_expression(arg))
                        .collect();

                    // Check if first argument is a string literal
                    let first_arg_is_string_literal = arguments
                        .first()
                        .map(|(_, arg)| {
                            matches!(
                                arg,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            )
                        })
                        .unwrap_or(false);

                    if args.len() == 1 && !first_arg_is_string_literal {
                        // Single non-string argument - format it
                        return format!("{}!(\"{{}}\", {})", target_macro, args[0]);
                    } else {
                        // Multiple args or string literal - keep as-is
                        return format!("{}!({})", target_macro, args.join(", "));
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
                    // Try multiple lookup strategies:
                    // 1. Type::method (if we can infer object type)
                    // 2. method (simple name fallback)
                    let method_signature =
                        self.signature_registry.get_signature(call_method).cloned();

                    // Generate arguments with ownership awareness (same logic as regular Call)
                    let args: Vec<String> = if let Some(ref sig) = method_signature {
                        arguments
                            .iter()
                            .enumerate()
                            .flat_map(|(i, (_label, arg))| {
                                let mut arg_str = self.generate_expression(arg);

                                // Apply ownership conversion based on signature
                                if let Some(&ownership) = sig.param_ownership.get(i) {
                                    match ownership {
                                        OwnershipMode::Borrowed => {
                                            // Destination wants borrowed - add & if needed
                                            let is_string_literal = matches!(
                                                arg,
                                                Expression::Literal {
                                                    value: Literal::String(_),
                                                    ..
                                                }
                                            );
                                            // THE WINDJAMMER WAY: Preserve user-written closure params
                                            let is_user_closure_param =
                                                if let Expression::Identifier { name, .. } = arg {
                                                    self.in_user_written_closure
                                                        && self.user_closure_params.contains(name)
                                                } else {
                                                    false
                                                };
                                            if !is_string_literal
                                                && !arg_str.starts_with("&")
                                                && !is_user_closure_param
                                            {
                                                arg_str = format!("&{}", arg_str);
                                            }
                                        }
                                        OwnershipMode::Owned => {
                                            // Destination wants owned - add .clone() for borrowed sources
                                            if let Expression::FieldAccess {
                                                object: field_obj,
                                                ..
                                            } = arg
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
                                                            .infer_expression_type(arg)
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
                                        _ => {}
                                    }
                                }

                                vec![arg_str]
                            })
                            .collect()
                    } else {
                        // No signature - just generate args without ownership hints
                        arguments
                            .iter()
                            .map(|(_label, arg)| self.generate_expression(arg))
                            .collect()
                    };

                    return format!("{}.{}({})", obj_str, call_method, args.join(", "));
                }

                let mut func_str = self.generate_expression(function);

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

                // WINDJAMMER PHILOSOPHY: Some/Ok/Err with string literals need .to_string()
                // Some("literal") -> Some("literal".to_string())
                // Ok("literal") -> Ok("literal".to_string())
                // Err("literal") -> Err("literal".to_string())
                // Also: Some(borrowed_iterator_var) -> Some(borrowed_iterator_var.clone())

                // TDD FIX (Bug #2): Detect ALL enum constructors, not just Some/Ok/Err
                // Pattern: Module::Variant or Enum::Variant (both CamelCase)
                let is_std_enum = matches!(func_name.as_str(), "Some" | "Ok" | "Err");
                let is_custom_enum = func_name.contains("::") && {
                    let parts: Vec<&str> = func_name.split("::").collect();
                    parts.len() == 2
                        && parts[0].chars().next().is_some_and(|c| c.is_uppercase())
                        && parts[1].chars().next().is_some_and(|c| c.is_uppercase())
                };

                if is_std_enum || is_custom_enum {
                    // TDD FIX (Bug #16 completion): Extract format!() to temp variables for enum variants too!
                    let generated_args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| self.generate_expression(arg))
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

                                // AUTO-CLONE: When wrapping a borrowed iterator variable in Some/Ok/Err,
                                // we need to clone it since the wrapper takes ownership
                                // UNLESS we're returning Option<&T>, Option<&mut T>, Result<&T, E>, etc.
                                if !returns_option_ref
                                    && !returns_result_ref
                                    && self.borrowed_iterator_vars.contains(name)
                                    && !result.ends_with(".clone()")
                                {
                                    // Function returns owned, but variable is borrowed - need to clone
                                    format!("{}.clone()", result)
                                } else {
                                    // Function returns reference, or variable not borrowed - don't clone
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
                let signature = if let Some(param) = self
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
                    self.signature_registry
                        .get_signature(&func_name)
                        .cloned()
                        .or_else(|| {
                            // If qualified lookup fails, try simple name (just the method)
                            if let Some(pos) = func_name.rfind("::") {
                                let simple_name = &func_name[pos + 2..];
                                self.signature_registry.get_signature(simple_name).cloned()
                            } else {
                                None
                            }
                        })
                };

                // Check if this is an extern function call for FFI str handling
                let is_extern_call = if let Some(ref sig) = signature {
                    sig.is_extern
                } else {
                    false
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

                        let mut arg_str = self.generate_expression(arg);

                        self.in_call_argument_generation = prev_in_call_arg;
                        self.in_field_access_object = prev_field_access_obj;

                        // WINDJAMMER FFI: Convert string arguments to (*const u8, usize) for extern functions
                        if is_extern_call {
                            if let Some(ref sig) = signature {
                                if let Some(param_type) = sig.param_types.get(i) {
                                    if matches!(param_type, Type::Custom(name) if name == "str") {
                                        // Expand str to (ptr, len)
                                        // Always use .as_bytes() for consistency (works for both &str and String)
                                        return vec![
                                            format!("{}.as_bytes().as_ptr()", arg_str),
                                            format!("{}.as_bytes().len()", arg_str),
                                        ];
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
                                return vec![arg_str];
                            }

                            if let Some(&ownership) = sig.param_ownership.get(i) {
                                match ownership {
                                    OwnershipMode::Borrowed => {
                                        // NEW DESIGN: Borrowed string parameters → &str (not &String!)
                                        // String literals are already &str in Rust, so they can be passed directly.
                                        // No conversion needed: "literal" → &str parameter is a perfect match
                                        let is_string_literal = matches!(
                                            arg,
                                            Expression::Literal {
                                                value: Literal::String(_),
                                                ..
                                            }
                                        );

                                        if is_string_literal {
                                            // String literals are already &str, pass directly to &str parameter
                                            // No & needed, no .to_string() needed
                                            return vec![arg_str];
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
                                    OwnershipMode::MutBorrowed => {
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
                                        // TDD FIX: AUTO-CONVERT for &str/&String → String, &T → T
                                        // When passing a reference to a function expecting owned, convert it
                                        // - &str → String: use .to_string()
                                        // - &String → String: use .clone()
                                        // - &T → T: use .clone()
                                        if let Expression::Identifier { name, .. } = arg {
                                            // Find the parameter type
                                            let param_type = self
                                                .current_function_params
                                                .iter()
                                                .find(|p| &p.name == name)
                                                .map(|p| &p.type_);

                                            // Check if it's a reference parameter (&str, &String, &T)
                                            if let Some(Type::Reference(inner_type)) = param_type {
                                                // Special case: &str (Type::Reference(Type::String) in Rust parlance)
                                                // &str.clone() → &str, but we need String, so use .to_string()
                                                if matches!(**inner_type, Type::String)
                                                    && !arg_str.ends_with(".to_string()")
                                                    && !arg_str.ends_with(".clone()")
                                                {
                                                    arg_str = format!("{}.to_string()", arg_str);
                                                } else if !arg_str.ends_with(".clone()") {
                                                    // For other reference types, .clone() works
                                                    arg_str = format!("{}.clone()", arg_str);
                                                }
                                            } else {
                                                // TDD FIX: Check if it's from a borrowed iterator (for loop)
                                                // Example: for npc_id in npc_ids { Member::new(npc_id) }
                                                // npc_id is &String from iterator, needs .clone() for owned String
                                                //
                                                // CRITICAL: We're in OwnershipMode::Owned block, which means
                                                // the DESTINATION parameter wants an owned value (String, not &String).
                                                // So it's correct to .clone() borrowed iterator vars.
                                                //
                                                // This block is fine - it only runs when ownership == Owned
                                                let is_borrowed_iterator_var =
                                                    self.borrowed_iterator_vars.contains(name);

                                                // Also check if it's inferred as borrowed
                                                let is_inferred_borrowed =
                                                    self.inferred_borrowed_params.contains(name);

                                                if (is_borrowed_iterator_var
                                                    || is_inferred_borrowed)
                                                    && !arg_str.ends_with(".clone()")
                                                {
                                                    // Borrowed from iterator or inferred - use .clone()
                                                    // This handles &String → String, &T → T
                                                    arg_str = format!("{}.clone()", arg_str);
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
                                    }
                                }
                            }
                        } else {
                            // No signature found - don't auto-clone!
                            // Without signature info, we can't know if destination wants Owned or Borrowed
                            // Better to let Rust compiler catch the error than guess wrong
                        }

                        vec![arg_str]
                    })
                    .collect();

                // TDD FIX (Bug #3): Extract format!() macros in arguments to temp variables
                // The args vec has already been generated as Rust strings
                // Check if any contain format!() and extract them
                let has_format_arg = args.iter().any(|arg_str| arg_str.contains("format!("));

                // WINDJAMMER PHILOSOPHY: Auto-wrap extern function calls in unsafe blocks
                // THE WINDJAMMER WAY: Users shouldn't have to write `unsafe` manually
                if has_format_arg {
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
                    if is_extern_call {
                        format!("unsafe {{ {}{}  }}", temp_decls, call_expr)
                    } else {
                        format!("{{ {}{} }}", temp_decls, call_expr)
                    }
                } else {
                    // No format!() args - generate normally with optional unsafe wrapper
                    let call_str = format!("{}({})", func_str, args.join(", "));
                    if is_extern_call {
                        format!("unsafe {{ {} }}", call_str)
                    } else {
                        call_str
                    }
                }
            }
            Expression::MethodCall {
                object,
                method,
                type_args,
                arguments,
                ..
            } => {
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
                // BUG #8 FIX: Look up method signature with qualified name (Type::method)
                // First try to infer the type from the object expression
                let type_name = self.infer_type_name(object);
                let method_signature = if let Some(type_name) = type_name {
                    let qualified_name = format!("{}::{}", type_name, method);
                    self.signature_registry
                        .get_signature(&qualified_name)
                        .cloned()
                    // CRITICAL: Do NOT fall back to unqualified method name lookup!
                    // Unqualified lookup for common names like "get", "remove", "contains"
                    // can match WRONG user-defined methods (e.g., ComponentArray::get when
                    // we want HashMap::get), causing incorrect auto-ref/auto-clone behavior.
                    // When the qualified name isn't found, method_signature stays None and
                    // the stdlib heuristics in should_add_ref handle common patterns correctly.
                } else {
                    // No type info available - only look up methods that are unlikely to
                    // conflict with stdlib methods (i.e., not "get", "remove", "contains_key" etc.)
                    let is_common_stdlib_name = matches!(
                        method.as_str(),
                        "get"
                            | "get_mut"
                            | "remove"
                            | "contains_key"
                            | "contains"
                            | "insert"
                            | "push"
                            | "pop"
                            | "len"
                            | "is_empty"
                            | "iter"
                            | "keys"
                            | "values"
                            | "first"
                            | "last"
                            | "clear"
                            | "binary_search"
                            | "starts_with"
                            | "ends_with"
                    );
                    if is_common_stdlib_name {
                        None // Use stdlib heuristics instead of potentially wrong signature
                    } else {
                        self.signature_registry.get_signature(method).cloned()
                    }
                };

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

                        let prev_suppress = self.suppress_borrowed_clone;
                        if param_expects_borrowed && matches!(arg, Expression::FieldAccess { .. }) {
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
                            // Check if this is a HashMap/BTreeMap key method with a borrowed String argument
                            let is_hashmap_key_method = matches!(
                                method.as_str(),
                                "contains_key" | "get" | "get_mut" | "remove" | "get_key_value"
                            ) && i == 0; // Key is always first argument

                            if is_hashmap_key_method {
                                // Check if the operand is a borrowed String parameter
                                if let Expression::Identifier { name, .. } = &**operand {
                                    let is_string_type = |t: &Type| {
                                        matches!(t, Type::String)
                                            || matches!(t, Type::Custom(s) if s == "String" || s == "string")
                                    };
                                    let is_borrowed_string = self.inferred_borrowed_params.contains(name)
                                        && self.current_function_params.iter().any(|param| {
                                            &param.name == name && is_string_type(&param.type_)
                                        });
                                    if is_borrowed_string {
                                        operand // Strip & — &String auto-derefs to &str
                                    } else {
                                        arg // Keep as-is
                                    }
                                } else {
                                    arg // Not an identifier — keep as-is
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
                        let mut arg_str = self.generate_expression(arg_to_generate);
                        self.in_field_access_object = prev_field_access_obj;

                        // TDD FIX: AUTO-WRAP function pointers in iterator adapter methods.
                        // Rust's .filter()/.any()/.find() on iter() yield &&T, expecting FnMut(&&T) -> bool,
                        // but bare function pointers fn(&T) -> bool don't auto-deref.
                        // THE WINDJAMMER WAY: Users write the natural `filter(predicate)` and the
                        // compiler generates `filter(|__e| predicate(__e))`.
                        if i == 0
                            && matches!(
                                method.as_str(),
                                "filter" | "any" | "all" | "find" | "find_map" | "position"
                                    | "take_while" | "skip_while" | "map_while" | "partition"
                                    | "rposition"
                            )
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
                        let string_literal_converted = if is_string_literal {
                            // Check what the parameter wants
                            let sig_param_idx = if method_signature.as_ref().is_some_and(|s| s.has_self_receiver) { i + 1 } else { i };
                            let param_ownership = method_signature
                                .as_ref()
                                .and_then(|sig| sig.param_ownership.get(sig_param_idx));

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
                                    Some(&OwnershipMode::Owned) => {
                                        // Parameter wants owned String → add .to_string()
                                        arg_str = format!("{}.to_string()", arg_str);
                                        true // Mark that we converted
                                    }
                                    Some(&OwnershipMode::Borrowed) => {
                                        // NEW DESIGN: Borrowed string parameters → &str (not &String!)
                                        // String literals are already &str, so pass directly (no conversion)
                                        false // No conversion needed
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

                        // TDD FIX: AUTO-CONVERT &str/&String → String for method calls
                        // When passing a &str parameter to a method expecting owned String, convert it
                        // This handles cases like: recipe.add_ingredient("herb", 1) where add_ingredient expects String
                        if let Expression::Identifier { name, .. } = arg {
                            // Find the parameter type
                            let param_type = self.current_function_params.iter()
                                .find(|p| &p.name == name)
                                .map(|p| &p.type_);

                            // Check if parameter type is &str (Type::Reference(Type::String))
                            if let Some(Type::Reference(inner_type)) = param_type {
                                if matches!(**inner_type, Type::String) {
                                    // Check if method signature expects owned String for this parameter
                                    let expects_owned = method_signature
                                        .as_ref()
                                        .and_then(|sig| sig.param_ownership.get(i))
                                        .is_some_and(|&ownership| matches!(ownership, OwnershipMode::Owned));

                                    if expects_owned && !arg_str.ends_with(".to_string()") && !arg_str.ends_with(".clone()") {
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

                        // AUTO-REF: Add & when parameter expects reference but arg is owned
                        // TDD FIX: Don't add & if we already handled string literal conversion above
                        if !string_literal_converted {
                            let should_ref = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                                arg,
                                &arg_str,
                                method,
                                i,
                                &method_signature,
                                &self.usize_variables,
                                &self.current_function_params,
                                &self.borrowed_iterator_vars,
                                &self.inferred_borrowed_params,
                                arguments.len(),
                            );
                            if should_ref {
                                arg_str = format!("&{}", arg_str);
                            }
                        }

                        // AUTO-BORROW for push_str: String::push_str expects &str, not String
                        // If arg is a String variable/expression (not a string literal), add &
                        if method == "push_str" && i == 0 {
                            let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                            // If not a string literal and not already a reference, add &
                            if !is_string_literal && !arg_str.starts_with('&') {
                                // Check if it's a String-producing expression (variable, field access, method call)
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

                        // Restore suppress flag
                        self.suppress_borrowed_clone = prev_suppress;

                        arg_str
                    })
                    .collect();

                // Generate turbofish if present
                let turbofish = if let Some(types) = type_args {
                    let type_strs: Vec<String> =
                        types.iter().map(|t| self.type_to_rust(t)).collect();
                    format!("::<{}>", type_strs.join(", "))
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

                                // TDD FIX (Bug #16): Don't always add & - format!() returns owned String
                                // If the parameter expects &str, Rust's coercion handles it automatically
                                // If the parameter expects String, we need the owned value
                                // Check if original arg_str had & to preserve caller's intent
                                if arg_str.starts_with("&") {
                                    // Original code had &format!() → keep the &
                                    format!("&{}", temp_name)
                                } else {
                                    // Original code had format!() → pass owned value
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

                // AUTO-CLONE: Method call results are ALWAYS owned values.
                // Unlike field accesses (self.field borrows from self) or identifiers
                // (which may be borrowed), calling a method produces a fresh value.
                // The auto-clone analysis may flag the *object* for cloning, but that
                // doesn't mean the *result of the method call* needs cloning.
                //
                // Exception: methods that return references (get, first, last) are
                // handled separately by should_add_cloned().
                //
                // WINDJAMMER PHILOSOPHY: Only clone when semantically necessary.
                // Method call results are never borrowed — cloning them is pure noise.
                base_expr
            }
            Expression::FieldAccess { object, field, .. } => {
                // FIELD CHAIN OPTIMIZATION: If we're accessing a likely-Copy sub-field
                // (e.g., .x, .y, .width, .speed), suppress borrowed-iterator cloning
                // on the intermediate object. In Rust, (&enemy).velocity.y works fine
                // through auto-deref — no need to clone the intermediate Vec2.
                let field_is_likely_copy = matches!(
                    field.as_str(),
                    "x" | "y"
                        | "z"
                        | "w"
                        | "width"
                        | "height"
                        | "depth"
                        | "r"
                        | "g"
                        | "b"
                        | "a"
                        | "left"
                        | "right"
                        | "top"
                        | "bottom"
                        | "min"
                        | "max"
                        | "start"
                        | "end"
                        | "offset"
                        | "scale"
                        | "speed"
                        | "time"
                        | "delta"
                        | "angle"
                        | "radius"
                        | "distance"
                        | "visible"
                        | "enabled"
                        | "active"
                        | "selected"
                        | "focused"
                        | "id"
                        | "type"
                        | "kind"
                        | "priority"
                        | "level"
                        | "len"
                        | "count"
                        | "size"
                        | "index"
                        | "idx"
                        | "vx"
                        | "vy"
                        | "vz"
                        | "dx"
                        | "dy"
                        | "dz"
                        | "health"
                        | "damage"
                        | "score"
                        | "lives"
                        | "frame"
                );
                // Also check via type inference if the outer expression (self.obj.field) is Copy
                let field_is_copy_by_type = self
                    .infer_expression_type(expr_to_generate)
                    .as_ref()
                    .is_some_and(|t| self.is_type_copy(t));

                let prev_suppress = self.suppress_borrowed_clone;
                let prev_field_access = self.in_field_access_object;
                if field_is_likely_copy || field_is_copy_by_type {
                    self.suppress_borrowed_clone = true;
                }
                // Suppress Vec index clone when we're just accessing a field
                // e.g., players[i].score → no need to clone the whole Player
                self.in_field_access_object = true;
                let obj_str = self.generate_expression_with_precedence(object);
                self.in_field_access_object = prev_field_access;
                self.suppress_borrowed_clone = prev_suppress;

                // Determine if this is a module/type path (::) or field access (.)
                // Check the object to decide:
                let separator = match &**object {
                    Expression::Identifier { name, .. }
                        if name.contains("::")
                            || (!name.is_empty()
                                && name.chars().next().unwrap().is_uppercase()) =>
                    {
                        "::" // Module path: std::fs or Type::CONST
                    }
                    Expression::FieldAccess { .. } => {
                        // Check if this is a module path or a field chain
                        // If the object string contains ::, it's a module path
                        if obj_str.contains("::") {
                            "::" // Module path: std::fs::File
                        } else {
                            "." // Field chain: transform.position.x
                        }
                    }
                    _ => ".", // Actual field access (e.g., config.field)
                };

                let base_expr = format!("{}{}{}", obj_str, separator, field);

                // AUTO-CLONE: Check if this field access needs to be cloned
                // Extract the full path (e.g., "config.paths")
                // CRITICAL: Never clone assignment targets (left side of `=`)
                // e.g., `emitter.lifetime = 1.0` must NOT become `emitter.clone().lifetime = 1.0`
                // DOUBLE-CLONE FIX: Skip auto-clone when we're inside an explicit .clone() call
                // The source already has .clone(), so we must not add another one.
                if !self.generating_assignment_target && !self.in_explicit_clone_call {
                    if let Some(path) = ast_utilities::extract_field_access_path(expr_to_generate) {
                        if let Some(ref analysis) = self.auto_clone_analysis {
                            if analysis
                                .needs_clone(&path, self.current_statement_idx)
                                .is_some()
                            {
                                // Skip .clone() for Copy types (f32, i32, bool, etc.)
                                // They are implicitly copied — .clone() is unnecessary noise.
                                let is_copy = self
                                    .infer_expression_type(expr_to_generate)
                                    .as_ref()
                                    .is_some_and(|t| self.is_type_copy(t));
                                if !is_copy {
                                    // Type inference failed — fall back to name heuristic
                                    // Fields like x, y, z, width, height are almost always Copy
                                    let is_likely_copy_field = matches!(
                                        field.as_str(),
                                        "x" | "y"
                                            | "z"
                                            | "w"
                                            | "width"
                                            | "height"
                                            | "depth"
                                            | "r"
                                            | "g"
                                            | "b"
                                            | "a"
                                            | "left"
                                            | "right"
                                            | "top"
                                            | "bottom"
                                            | "min"
                                            | "max"
                                            | "start"
                                            | "end"
                                            | "offset"
                                            | "scale"
                                            | "speed"
                                            | "time"
                                            | "delta"
                                            | "angle"
                                            | "radius"
                                            | "distance"
                                            | "visible"
                                            | "enabled"
                                            | "active"
                                            | "selected"
                                            | "focused"
                                            | "id"
                                            | "type"
                                            | "kind"
                                            | "priority"
                                            | "level"
                                            | "len"
                                            | "count"
                                            | "size"
                                            | "index"
                                            | "idx"
                                            | "vx"
                                            | "vy"
                                            | "vz"
                                            | "dx"
                                            | "dy"
                                            | "dz"
                                            | "health"
                                            | "damage"
                                            | "score"
                                            | "lives"
                                            | "frame"
                                    );
                                    if !is_likely_copy_field {
                                        return format!("{}.clone()", base_expr);
                                    }
                                }
                            }
                        }
                    }
                }

                // BORROWED ITERATOR: If accessing fields through a borrowed iterator variable,
                // we need to clone non-Copy fields since we can't move out of a reference
                // BUT: Don't clone for assignment targets (left side of =)
                // AND: Don't clone when a parent FieldAccess is reading a Copy sub-field
                //      (e.g., bullet.velocity.y → .y is Copy, so no need to clone velocity)
                // AND: Don't clone when inside an explicit .clone() call (prevents double clone)
                // AND: Don't clone when this is an intermediate object in a field access chain
                //      (e.g., stack.item.stats.armor → don't clone item, Rust auto-derefs through &)
                // AND: Don't clone in borrow context (&recipe.ingredients → reference is sufficient)
                // TDD FIX: Don't clone when generating call arguments (Call handler applies ownership)
                // WINDJAMMER PHILOSOPHY: Use type inference first, fall back to name heuristics
                if !self.generating_assignment_target
                    && !self.suppress_borrowed_clone
                    && !self.in_explicit_clone_call
                    && !self.in_field_access_object
                    && !self.in_borrow_context
                    && !self.in_call_argument_generation
                {
                    if let Expression::Identifier { name: var_name, .. } = &**object {
                        if self.borrowed_iterator_vars.contains(var_name) {
                            // First: use type inference to check if the field type is Copy
                            let is_copy = self
                                .infer_expression_type(expr_to_generate)
                                .as_ref()
                                .is_some_and(|t| self.is_type_copy(t));

                            if !is_copy {
                                // Fall back to name-based heuristics for fields we KNOW are Copy
                                let is_likely_copy_field = matches!(
                                    field.as_str(),
                                    "len" | "count" | "size" | "index" | "idx" | "i" | "j" | "k" |
                                    "x" | "y" | "z" | "w" | "width" | "height" | "depth" |
                                    "r" | "g" | "b" | "a" | "left" | "right" | "top" | "bottom" |
                                    "min" | "max" | "start" | "end" | "offset" | "scale" |
                                    "speed" | "time" | "delta" | "angle" | "radius" | "distance" |
                                    "visible" | "enabled" | "active" | "selected" | "focused" |
                                    "id" | "type" | "kind" | "priority" | "level" |
                                    // Method-like names that should NOT be cloned
                                    "as_str" | "to_string" | "clone" | "iter" | "iter_mut" | "is_empty"
                                );
                                if !is_likely_copy_field && !base_expr.ends_with(".clone()") {
                                    return format!("{}.clone()", base_expr);
                                }
                            }
                        }
                    }
                }

                // NOTE: Auto-clone for self.field is handled at a higher level
                // (in struct literal generation and specific return contexts)
                // Do NOT clone here as it causes issues with .iter() on collections

                base_expr
            }
            Expression::StructLiteral { name, fields, .. } => {
                // PHASE 3 OPTIMIZATION: Check if we have optimization hints for this struct
                let _has_optimization_hint = self.struct_mapping_hints.get(name);

                // Generate field assignments
                let field_str: Vec<String> = fields
                    .iter()
                    .map(|(field_name, expr)| {
                        // STRUCT LITERAL CONTEXT: Array literals in struct fields should use
                        // fixed-size [...] syntax, not vec![...], because struct fields have
                        // explicit type annotations (e.g., position: [f32; 3]).
                        let prev_in_struct_field = self.in_struct_literal_field;
                        self.in_struct_literal_field = true;

                        // WINDJAMMER PHILOSOPHY: Auto-convert string literals to String
                        // In Windjammer, `string` type is always owned (maps to Rust String)
                        // So string literals in struct fields should be converted automatically
                        let mut expr_str = self.generate_expression(expr);

                        // Restore previous context
                        self.in_struct_literal_field = prev_in_struct_field;

                        // Auto-convert string literals to String for struct fields
                        if matches!(
                            expr,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        ) {
                            expr_str = format!("{}.to_string()", expr_str);
                        }

                        // CRITICAL: Auto-convert &str parameters to String for struct fields
                        // Pattern: fn create(name: &str) -> User { User { name: name } }
                        // When struct field is String but parameter is &str, add .to_string()
                        if let Expression::Identifier { name: id, .. } = expr {
                            // Check if this identifier is a &str parameter
                            // In the AST, &str parameters have type Reference(Custom("str"))
                            let is_str_param = self.current_function_params.iter().any(|p| {
                                p.name == *id && matches!(
                                    &p.type_,
                                    crate::parser::Type::Reference(inner) if matches!(**inner, crate::parser::Type::Custom(ref name) if name == "str")
                                )
                            });

                            if is_str_param && !expr_str.contains(".to_string()") {
                                expr_str = format!("{}.to_string()", expr_str);
                            }
                        }

                        // CRITICAL: Auto-clone self.field when constructing struct from borrowed self
                        // Pattern: fn method(&self) -> Self { Self { field: self.field } }
                        // Non-Copy fields from borrowed self need to be cloned
                        if let Expression::FieldAccess { object, .. } = expr {
                            if let Expression::Identifier { name: obj_name, .. } = &**object {
                                if obj_name == "self" && !expr_str.contains(".clone()") {
                                    // Check if current function takes &self (borrowed)
                                    let self_is_borrowed =
                                        self.current_function_params.iter().any(|p| {
                                            p.name == "self"
                                                && matches!(
                                                    p.ownership,
                                                    crate::parser::OwnershipHint::Ref
                                                )
                                        });

                                    if self_is_borrowed {
                                        // Clone the field access since self is borrowed
                                        expr_str = format!("{}.clone()", expr_str);
                                    }
                                }
                            }
                        }

                        // Check for field shorthand: if expr is just the field name AND no conversion applied, use shorthand
                        // Only use shorthand if the generated expression exactly matches the field name
                        // (no .to_string(), .clone(), etc. conversions)
                        if let Expression::Identifier { name: id, .. } = expr {
                            if id == field_name && expr_str == *field_name {
                                // Shorthand: User { name } instead of User { name: name }
                                // Only safe when no type conversion was needed
                                return field_name.clone();
                            }
                        }

                        format!("{}: {}", field_name, expr_str)
                    })
                    .collect();

                format!("{} {{ {} }}", name, field_str.join(", "))
            }
            Expression::MapLiteral { pairs, .. } => {
                // Generate HashMap literal: HashMap::from([(key, value), ...])
                if pairs.is_empty() {
                    "std::collections::HashMap::new()".to_string()
                } else {
                    let entries_str: Vec<String> = pairs
                        .iter()
                        .map(|(k, v)| {
                            let key_str = self.generate_expression(k);
                            let val_str = self.generate_expression(v);
                            format!("({}, {})", key_str, val_str)
                        })
                        .collect();
                    format!(
                        "std::collections::HashMap::from([{}])",
                        entries_str.join(", ")
                    )
                }
            }
            Expression::TryOp { expr: inner, .. } => {
                format!("{}?", self.generate_expression(inner))
            }
            Expression::Await { expr: inner, .. } => {
                format!("{}.await", self.generate_expression(inner))
            }
            Expression::ChannelSend { channel, value, .. } => {
                let ch_str = self.generate_expression(channel);
                let val_str = self.generate_expression(value);
                format!("{}.send({})", ch_str, val_str)
            }
            Expression::ChannelRecv { channel, .. } => {
                let ch_str = self.generate_expression(channel);
                format!("{}.recv()", ch_str)
            }
            Expression::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                let start_str = self.generate_expression(start);
                let end_str = self.generate_expression(end);
                if *inclusive {
                    format!("{}..={}", start_str, end_str)
                } else {
                    format!("{}..{}", start_str, end_str)
                }
            }
            Expression::Closure {
                parameters, body, ..
            } => {
                let params = parameters.join(", ");

                // THE WINDJAMMER WAY: Smart `move` inference for closures
                //
                // Add `move` automatically ONLY for compiler-generated closures (params start with __).
                // User-written closures are preserved as-is (respect explicit intent).
                // Rationale:
                // 1. Compiler-generated closures (function pointer wrappers) → add `move` for safety
                // 2. User-written closures → preserve exactly as written (explicit is explicit)
                // 3. Method closures that capture `self` → don't add `move` (UI callbacks need to borrow)
                //
                // This makes Windjammer code simpler while respecting explicit user intent.

                // Check if this is a compiler-generated closure (params start with __)
                let is_compiler_generated = parameters.iter().any(|p| p.starts_with("__"));

                // Check if the closure body references `self`
                let captures_self = self.expression_references_self(body);

                // For user-written closures, set flag and track params to suppress transformations
                let prev_in_user_closure = self.in_user_written_closure;
                let mut prev_closure_params = None;
                if !is_compiler_generated {
                    self.in_user_written_closure = true;
                    prev_closure_params = Some(std::mem::take(&mut self.user_closure_params));
                    for param in parameters {
                        self.user_closure_params.insert(param.clone());
                    }
                }

                // Generate closure body with context flags set
                let body_str = self.generate_expression(body);

                // Restore previous state
                if !is_compiler_generated {
                    self.in_user_written_closure = prev_in_user_closure;
                    if let Some(prev_params) = prev_closure_params {
                        self.user_closure_params = prev_params;
                    }
                }

                if is_compiler_generated && !captures_self {
                    // Compiler-generated closure that doesn't capture self → add `move`
                    format!("move |{}| {}", params, body_str)
                } else {
                    // User-written closure or captures self → preserve as-is
                    format!("|{}| {}", params, body_str)
                }
            }
            Expression::Index { object, index, .. } => {
                // INDEX CHAIN OPTIMIZATION: When generating the object of an Index expression,
                // suppress auto-clone. In `a[i][j]`, Rust auto-derefs `a[i]` (returns &Vec<T>)
                // to access [j]. Cloning the intermediate Vec is wasteful and wrong.
                // Same logic as in_field_access_object for FieldAccess chains.
                let prev_field_access = self.in_field_access_object;
                self.in_field_access_object = true;
                let obj_str = self.generate_expression(object);
                self.in_field_access_object = prev_field_access;

                // Special case: if index is a Range, this is slice syntax
                // FIXED: Don't add & - Rust will auto-coerce to &[T] when needed
                // This prevents "&temporary" errors when chaining methods like .to_vec()
                if let Expression::Range {
                    start,
                    end,
                    inclusive,
                    ..
                } = &**index
                {
                    let start_str = self.generate_expression(start);
                    let end_str = self.generate_expression(end);
                    let range_op = if *inclusive { "..=" } else { ".." };
                    return format!("{}[{}{}{}]", obj_str, start_str, range_op, end_str);
                }

                let idx_str = self.generate_expression(index);

                // WINDJAMMER PHILOSOPHY: Auto-cast to usize for array indexing
                // Rust requires usize for indexing, but Windjammer uses int (i64)
                // Handle cases:
                // 1. Simple identifier: arr[idx] -> arr[idx as usize]
                // 2. Integer literal: arr[0] -> arr[0 as usize]
                // 3. Cast to int/i64: arr[x as int] -> arr[x as usize]
                // 4. Parenthesized cast: arr[(x as int)] -> arr[x as usize]
                // 5. Already usize: don't double-cast
                let final_idx = if idx_str.ends_with("as i64)") || idx_str.ends_with("as int)") {
                    // Replace (... as i64/int) with (... as usize)
                    let base = idx_str
                        .trim_end_matches("as i64)")
                        .trim_end_matches("as int)")
                        .trim()
                        .trim_start_matches('(')
                        .trim();
                    format!("{} as usize", base)
                } else if idx_str.ends_with("as i64") || idx_str.ends_with("as int") {
                    // Replace ... as i64/int with ... as usize
                    let base = idx_str
                        .trim_end_matches("as i64")
                        .trim_end_matches("as int")
                        .trim();
                    format!("{} as usize", base)
                } else if matches!(
                    &**index,
                    Expression::Identifier { .. }
                        | Expression::Literal {
                            value: Literal::Int(_),
                            ..
                        }
                ) && !idx_str.contains(" as ")
                {
                    // Skip cast if identifier is already usize (e.g. assigned from `expr as usize`)
                    if let Expression::Identifier { name, .. } = &**index {
                        if self.usize_variables.contains(name)
                            || self.expression_produces_usize(index)
                        {
                            idx_str // Already usize — no cast needed
                        } else {
                            format!("{} as usize", idx_str)
                        }
                    } else if let Expression::Literal {
                        value: Literal::Int(n),
                        ..
                    } = &**index
                    {
                        // Integer literal: Rust infers type from context in index position,
                        // so `arr[0]` works without `as usize`. Only cast if negative
                        // (which would be a logic error, but preserve the cast for clarity).
                        if *n < 0 {
                            format!("{} as usize", idx_str)
                        } else {
                            idx_str
                        }
                    } else {
                        format!("{} as usize", idx_str)
                    }
                } else {
                    idx_str
                };

                let base_expr = format!("{}[{}]", obj_str, final_idx);

                // WINDJAMMER PHILOSOPHY: Auto-clone Vec indexing for non-Copy types.
                // Rust doesn't allow moving out of a Vec index (E0507).
                // For Copy types: vec[idx] works directly (value is copied).
                // For non-Copy types: vec[idx].clone() is needed.
                //
                // CRITICAL: NEVER auto-clone in these contexts:
                // 1. Assignment target: vec[i] = value (can't assign to .clone())
                // 2. Borrow context: &vec[i] (want reference to original, not to clone)
                // 3. Field access: vec[i].field (Rust allows field access through ref)
                // 4. Comparison context: vec[i] == val (comparisons work on &T)
                let suppress_clone = self.generating_assignment_target
                    || self.in_borrow_context
                    || self.in_field_access_object
                    || self.suppress_borrowed_clone;

                if !suppress_clone {
                    // First check auto_clone_analysis (path-based analysis)
                    if let Some(path) = ast_utilities::extract_field_access_path(expr_to_generate) {
                        if let Some(ref analysis) = self.auto_clone_analysis {
                            if analysis
                                .needs_clone(&path, self.current_statement_idx)
                                .is_some()
                            {
                                let is_copy = self
                                    .infer_expression_type(expr_to_generate)
                                    .as_ref()
                                    .is_some_and(|t| self.is_type_copy(t));
                                if !is_copy {
                                    return format!("{}.clone()", base_expr);
                                }
                            }
                        }
                    }

                    // Fallback: Type-based auto-clone for Vec<NonCopy>[idx]
                    // If we can infer the collection's element type and it's not Copy, clone.
                    // This handles the common case: vec[i] passed to a function taking ownership.
                    if let Some(obj_type) = self.infer_expression_type(object) {
                        let element_type = match &obj_type {
                            Type::Vec(inner) => Some(inner.as_ref()),
                            Type::Array(inner, _) => Some(inner.as_ref()),
                            _ => None,
                        };
                        if let Some(elem_type) = element_type {
                            if !self.is_type_copy(elem_type) {
                                return format!("{}.clone()", base_expr);
                            }
                        }
                    }
                }

                base_expr
            }
            Expression::Tuple {
                elements: exprs, ..
            } => {
                let expr_strs: Vec<String> =
                    exprs.iter().map(|e| self.generate_expression(e)).collect();
                format!("({})", expr_strs.join(", "))
            }
            Expression::Array {
                elements: exprs, ..
            } => {
                let expr_strs: Vec<String> =
                    exprs.iter().map(|e| self.generate_expression(e)).collect();

                // WINDJAMMER PHILOSOPHY: Array literal syntax determines Rust output.
                //
                // In WJ, `[a, b, c]` is a fixed-size array literal → generates `[a, b, c]` in Rust.
                // In WJ, `vec![a, b, c]` is an explicit Vec constructor → generates `vec![a, b, c]`.
                //
                // Empty arrays `[]` remain `vec![]` because Rust's empty `[]` can't infer its type.
                //
                // This distinction is critical: `painter.line_segment([p1, p2], stroke)` expects
                // `[Pos2; 2]`, not `Vec<Pos2>`. The developer chose `[...]` syntax intentionally.
                if exprs.is_empty() {
                    // Empty array [] → vec![] (Vec::new())
                    // Rust's [] is a fixed-size array and can't infer type from later usage.
                    "vec![]".to_string()
                } else {
                    // Non-empty array literals: generate fixed-size array [a, b, c]
                    // The developer uses `vec![...]` macro syntax when Vec is needed.
                    format!("[{}]", expr_strs.join(", "))
                }
            }
            Expression::MacroInvocation {
                is_repeat,
                name,
                args,
                delimiter,
                ..
            } => {
                use crate::parser::MacroDelimiter;

                // PHASE 4 OPTIMIZATION: Check for format! with capacity hints
                if name == "format" {
                    if let Some(&capacity) =
                        self.string_capacity_hints.get(&self.current_statement_idx)
                    {
                        // Clone capacity to avoid borrow issues
                        let capacity_val = capacity;
                        // Generate optimized String::with_capacity + write! instead of format!
                        self.needs_write_import = true;
                        let arg_strs: Vec<String> =
                            args.iter().map(|e| self.generate_expression(e)).collect();

                        return format!(
                            "{{\n{}    let mut __s = String::with_capacity({});\n{}    write!(&mut __s, {}).unwrap();\n{}    __s\n{}}}",
                            self.indent(),
                            capacity_val,
                            self.indent(),
                            arg_strs.join(", "),
                            self.indent(),
                            self.indent()
                        );
                    }
                }

                // Special case: if this is println!/eprintln!/print!/eprint! and first arg is format!, flatten it
                let should_flatten = (name == "println"
                    || name == "eprintln"
                    || name == "print"
                    || name == "eprint")
                    && !args.is_empty()
                    && matches!(&args[0], Expression::MacroInvocation { name: macro_name, .. } if macro_name == "format");

                let arg_strs: Vec<String> = if should_flatten {
                    // Flatten format! macro arguments into the print macro
                    if let Expression::MacroInvocation {
                        is_repeat: _,
                        args: format_args,
                        ..
                    } = &args[0]
                    {
                        format_args
                            .iter()
                            .map(|e| self.generate_expression(e))
                            .collect()
                    } else {
                        args.iter().map(|e| self.generate_expression(e)).collect()
                    }
                } else {
                    // Special case: if this is println!/eprintln!/print!/eprint! with a single non-literal arg,
                    // wrap it with "{}" to make it valid Rust: println!(var) -> println!("{}", var)
                    // Also wrap format!() calls: println!(format!(...)) -> println!("{}", format!(...))
                    if (name == "println"
                        || name == "eprintln"
                        || name == "print"
                        || name == "eprint")
                        && args.len() == 1
                        && !matches!(
                            &args[0],
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        )
                    {
                        vec!["\"{}\"".to_string(), self.generate_expression(args[0])]
                    } else {
                        args.iter().map(|e| self.generate_expression(e)).collect()
                    }
                };

                let (open, close) = match delimiter {
                    MacroDelimiter::Parens => ("(", ")"),
                    MacroDelimiter::Brackets => ("[", "]"),
                    MacroDelimiter::Braces => ("{", "}"),
                };

                // WINDJAMMER FIX: vec![value; count] repeat syntax
                // The parser sets is_repeat=true for vec![x; n] syntax
                // Use semicolon for repeat, comma for regular args
                let separator = if *is_repeat { "; " } else { ", " };

                // WINDJAMMER FIX: String literal coercion in vec![]
                // In Windjammer, `string` maps to Rust `String`, so vec!["a", "b"] must
                // become vec!["a".to_string(), "b".to_string()] for Vec<String>.
                // Only apply when: macro is vec, brackets delimiter, has string literal args.
                let final_arg_strs: Vec<String> = if name == "vec"
                    && matches!(delimiter, MacroDelimiter::Brackets)
                    && !*is_repeat
                {
                    arg_strs
                        .iter()
                        .enumerate()
                        .map(|(idx, s)| {
                            // Check if the original arg is a string literal
                            if idx < args.len() {
                                if let Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                } = &args[idx]
                                {
                                    // Add .to_string() if not already present
                                    if !s.ends_with(".to_string()") {
                                        return format!("{}.to_string()", s);
                                    }
                                }
                            }
                            s.clone()
                        })
                        .collect()
                } else {
                    arg_strs
                };

                format!(
                    "{}!{}{}{}",
                    name,
                    open,
                    final_arg_strs.join(separator),
                    close
                )
            }
            Expression::Cast { expr, type_, .. } => {
                // Add parentheses around binary expressions for correct precedence
                // because `as` has higher precedence than arithmetic in Rust:
                // `a + b as usize` is parsed as `a + (b as usize)`, not `(a + b) as usize`
                let expr_str = match &**expr {
                    Expression::Binary { .. } => {
                        format!("({})", self.generate_expression(expr))
                    }
                    _ => self.generate_expression(expr),
                };
                let type_str = self.type_to_rust(type_);
                // TDD FIX: Do NOT wrap cast in outer parentheses.
                // `as` has higher precedence than comparison/arithmetic operators in Rust,
                // so `x as usize >= y` correctly parses as `(x as usize) >= y`.
                // Outer parens are ONLY needed when the cast is followed by `.method()`
                // or `.field` (handled at the MethodCall/FieldAccess generation sites).
                format!("{} as {}", expr_str, type_str)
            }
            Expression::Block {
                statements: stmts,
                is_unsafe,
                ..
            } => {
                let block_open = if *is_unsafe { "unsafe {\n" } else { "{\n" };
                // Special case: if the block contains only a match statement, generate it as a match expression
                // BUT: Skip this optimization when the match is an if-let pattern (2 arms, last is wildcard with empty body)
                // In that case, fall through to normal block generation which will generate `if let` via Statement::Match handler
                if stmts.len() == 1 {
                    if let Statement::Match { value, arms, .. } = &stmts[0] {
                        // Check if this is an if-let pattern that should be generated as `if let`
                        let is_if_let_pattern = arms.len() == 2
                            && matches!(arms[1].pattern, Pattern::Wildcard)
                            && arms[1].guard.is_none()
                            && matches!(arms[1].body, Expression::Block { statements, .. } if statements.is_empty());

                        if is_if_let_pattern {
                            // Fall through to normal block generation — generate_statement will emit `if let`
                            let mut output = String::from(block_open);
                            self.indent_level += 1;
                            for stmt in stmts {
                                output.push_str(&self.generate_statement(stmt));
                            }
                            self.indent_level -= 1;
                            output.push_str(&self.indent());
                            output.push('}');
                            return output;
                        }

                        let mut output = String::from("match ");

                        // Check if any arm has a string literal pattern
                        // BUT: Don't add .as_str() if the match value is a tuple
                        let has_string_literal = arms
                            .iter()
                            .any(|arm| pattern_analysis::pattern_has_string_literal(&arm.pattern));

                        let is_tuple_match = arms
                            .iter()
                            .any(|arm| matches!(arm.pattern, Pattern::Tuple(_)));

                        // CRITICAL: Check if matching on self.field to avoid partial move
                        let needs_clone_for_match =
                            self.match_needs_clone_for_self_field(value, arms);

                        let value_str = self.generate_expression(value);
                        if has_string_literal && !is_tuple_match {
                            // Add .as_str() if the value doesn't already end with it
                            if !value_str.ends_with(".as_str()") {
                                output.push_str(&format!("{}.as_str()", value_str));
                            } else {
                                output.push_str(&value_str);
                            }
                        } else if needs_clone_for_match && !value_str.ends_with(".clone()") {
                            // Clone the field to avoid partial move from self
                            output.push_str(&format!("{}.clone()", value_str));
                        } else {
                            output.push_str(&value_str);
                        }

                        output.push_str(" {\n");

                        self.indent_level += 1;

                        // WINDJAMMER PHILOSOPHY: Detect if any arm returns String and convert all arms
                        let needs_string_conversion_from_type = match &self
                            .current_function_return_type
                        {
                            Some(Type::String) => true,
                            Some(Type::Custom(name)) if name == "String" => true,
                            _ => arms.iter().any(|arm| {
                                string_analysis::expression_produces_string(arm.body)
                                    || arm_string_analysis::arm_returns_converted_string(arm.body)
                            }),
                        };

                        // Set context flag BEFORE generating arms
                        let old_in_match_arm = self.in_match_arm_needing_string;
                        if needs_string_conversion_from_type {
                            self.in_match_arm_needing_string = true;
                        }

                        // Generate all arms with the flag set
                        let arm_strings: Vec<(String, bool)> = arms
                            .iter()
                            .map(|arm| {
                                let body_str = self.generate_expression(arm.body);
                                let is_string_literal = matches!(
                                    &arm.body,
                                    Expression::Literal {
                                        value: Literal::String(_),
                                        ..
                                    }
                                );
                                (body_str, is_string_literal)
                            })
                            .collect();

                        // Restore flag
                        self.in_match_arm_needing_string = old_in_match_arm;

                        // For direct string literals, we still need to apply .to_string()
                        let any_arm_produces_string = needs_string_conversion_from_type;

                        for (arm, (arm_str, is_string_literal)) in
                            arms.iter().zip(arm_strings.iter())
                        {
                            output.push_str(&self.indent());
                            output.push_str(&self.generate_pattern(&arm.pattern));

                            // Add guard if present
                            if let Some(guard) = &arm.guard {
                                output.push_str(" if ");
                                output.push_str(&self.generate_expression(guard));
                            }

                            output.push_str(" => ");

                            // Auto-convert string literals to String when other arms return String
                            if any_arm_produces_string
                                && *is_string_literal
                                && !arm_str.ends_with(".to_string()")
                            {
                                output.push_str(&format!("{}.to_string()", arm_str));
                            } else {
                                output.push_str(arm_str);
                            }
                            output.push_str(",\n");
                        }
                        self.indent_level -= 1;

                        output.push_str(&self.indent());
                        output.push('}');
                        return output;
                    }
                }

                // Regular block - must handle last expression correctly
                let mut output = String::from(block_open);
                self.indent_level += 1;

                let len = stmts.len();
                for (i, stmt) in stmts.iter().enumerate() {
                    let is_last = i == len - 1;
                    if is_last
                        && matches!(
                            stmt,
                            Statement::Expression { .. }
                                | Statement::Thread { .. }
                                | Statement::Async { .. }
                        )
                    {
                        // Last statement is an expression, thread/async block - generate as implicit return
                        match stmt {
                            Statement::Expression { expr, .. } => {
                                output.push_str(&self.indent());
                                let mut expr_str = self.generate_expression(expr);

                                // If in a match arm needing string conversion, convert string literals
                                if self.in_match_arm_needing_string {
                                    let is_string_literal = matches!(
                                        expr,
                                        Expression::Literal {
                                            value: Literal::String(_),
                                            ..
                                        }
                                    );
                                    if is_string_literal && !expr_str.ends_with(".to_string()") {
                                        expr_str = format!("{}.to_string()", expr_str);
                                    }
                                }

                                output.push_str(&expr_str);

                                // TDD FIX: In statement-context matches, add semicolons to all statements
                                if self.in_statement_match {
                                    output.push_str(";\n");
                                } else {
                                    output.push('\n');
                                }
                            }
                            Statement::Thread { body, .. } => {
                                output.push_str(&self.indent());
                                output.push_str("std::thread::spawn(move || {\n");
                                self.indent_level += 1;
                                for stmt in body {
                                    output.push_str(&self.generate_statement(stmt));
                                }
                                self.indent_level -= 1;
                                output.push_str(&self.indent());
                                output.push_str("})\n");
                            }
                            Statement::Async { body, .. } => {
                                output.push_str(&self.indent());
                                output.push_str("tokio::spawn(async move {\n");
                                self.indent_level += 1;
                                for stmt in body {
                                    output.push_str(&self.generate_statement(stmt));
                                }
                                self.indent_level -= 1;
                                output.push_str(&self.indent());
                                output.push_str("})\n");
                            }
                            _ => unreachable!(),
                        }
                    } else if !is_last {
                        let old_expr_ctx = self.in_expression_context;
                        self.in_expression_context = false;
                        output.push_str(&self.generate_statement(stmt));
                        self.in_expression_context = old_expr_ctx;
                    } else {
                        output.push_str(&self.generate_statement(stmt));
                    }
                }

                self.indent_level -= 1;
                output.push_str(&self.indent());
                output.push('}');
                output
            }
        }
    }

    pub(super) fn generate_literal(&self, lit: &Literal) -> String {
        match lit {
            Literal::Int(n) => n.to_string(),
            Literal::Float(f) => {
                let s = f.to_string();
                // Ensure float literals always have a decimal point
                if !s.contains('.') && !s.contains('e') {
                    format!("{}.0", s)
                } else {
                    s
                }
            }
            Literal::String(s) => {
                // Check for string interpolation: {variable}
                if s.contains('{') && s.contains('}') {
                    // Convert to format! macro
                    // "Count: {count}" -> format!("Count: {}", count)
                    let mut format_str = String::new();
                    let mut args = Vec::new();
                    let mut chars = s.chars().peekable();

                    while let Some(ch) = chars.next() {
                        if ch == '{' {
                            // Check if it's {variable} pattern or {} placeholder
                            let mut var_name = String::new();
                            let mut is_variable = true;

                            while let Some(&next_ch) = chars.peek() {
                                if next_ch == '}' {
                                    chars.next(); // consume }
                                    break;
                                } else if next_ch.is_alphanumeric() || next_ch == '_' {
                                    var_name.push(next_ch);
                                    chars.next();
                                } else {
                                    // Not a simple variable pattern
                                    is_variable = false;
                                    break;
                                }
                            }

                            if is_variable && !var_name.is_empty() {
                                // It's a variable interpolation: {count} -> {}, count
                                format_str.push_str("{}");
                                args.push(var_name);
                            } else if is_variable && var_name.is_empty() {
                                // It's an empty placeholder: {} -> keep as-is (format! placeholder)
                                format_str.push_str("{}");
                            } else {
                                // Not a variable, escape the literal brace
                                format_str.push_str("{{");
                                format_str.push_str(&var_name);
                            }
                        } else if ch == '}' {
                            // Escape literal closing brace (not part of a placeholder)
                            format_str.push_str("}}");
                        } else {
                            format_str.push(ch);
                        }
                    }

                    if args.is_empty() {
                        // No interpolation found, just a regular string
                        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
                    } else {
                        // Generate format! call with implicit self for struct fields
                        let formatted_args = args
                            .iter()
                            .map(|a| {
                                // Check if this is a struct field and add self. prefix
                                if self.in_impl_block && self.current_struct_fields.contains(a) {
                                    format!(", self.{}", a)
                                } else {
                                    format!(", {}", a)
                                }
                            })
                            .collect::<String>();

                        format!(
                            "format!(\"{}\"{})",
                            format_str.replace('\\', "\\\\").replace('"', "\\\""),
                            formatted_args
                        )
                    }
                } else {
                    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
                }
            }
            Literal::Char(c) => {
                // Escape special characters
                match c {
                    '\n' => "'\\n'".to_string(),
                    '\t' => "'\\t'".to_string(),
                    '\r' => "'\\r'".to_string(),
                    '\\' => "'\\\\'".to_string(),
                    '\'' => "'\\''".to_string(),
                    '\0' => "'\\0'".to_string(),
                    _ => format!("'{}'", c),
                }
            }
            Literal::Bool(b) => b.to_string(),
        }
    }

    /// Generate efficient string concatenation using format! macro
    fn generate_string_concat(
        &mut self,
        left: &Expression<'ast>,
        right: &Expression<'ast>,
    ) -> String {
        // Collect all parts of the concatenation chain
        let mut parts = Vec::new();
        string_analysis::collect_concat_parts_static(left, &mut parts);
        string_analysis::collect_concat_parts_static(right, &mut parts);

        // Generate format! macro call
        let format_str = "{}".repeat(parts.len());

        // Generate expressions for each part
        let mut args = Vec::new();
        for expr in &parts {
            args.push(self.generate_expression(expr));
        }

        format!("format!(\"{}\", {})", format_str, args.join(", "))
    }
}
