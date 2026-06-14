//! Data structure expression generation
//!
//! Handles generation of:
//! - Tuples
//! - Arrays
//! - Maps
//! - Struct literals
//! - Index operations
//! - Field access

use crate::parser::{Expression, Type};

use super::{ast_utilities, float_type_utilities, string_utilities, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn generate_tuple(
        &mut self,
        elements: &[&Expression<'ast>],
    ) -> String {
        let return_tuple_types = match &self.current_function_return_type {
            Some(Type::Tuple(types)) => Some(types.clone()),
            _ => None,
        };
        let expr_strs: Vec<String> = elements
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let mut s = self.generate_expression(e);
                if matches!(
                    e,
                    Expression::Literal {
                        value: crate::parser::Literal::String(_),
                        ..
                    }
                ) {
                    let needs_owned = return_tuple_types
                        .as_ref()
                        .and_then(|types| types.get(i))
                        .is_some_and(crate::codegen::rust::types::is_windjammer_text_type);
                    if needs_owned
                        && !crate::codegen::rust::string_utilities::already_owned_string_expr(&s)
                    {
                        s = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(&s);
                    }
                }
                if !s.ends_with(".clone()")
                    && !crate::codegen::rust::literals::is_already_owned_string(&s)
                {
                    let ty = self.infer_expression_type(e);
                    let needs_clone = ty.as_ref().is_some_and(|t| match t {
                        Type::Reference(inner) | Type::MutableReference(inner) => {
                            !self.is_type_copy(inner)
                        }
                        _ => false,
                    });
                    if !needs_clone {
                        // Also clone non-Copy field accesses through references
                        // (e.g. from_stack.item.id where from_stack is behind &)
                        if let Expression::FieldAccess { object, .. } = e {
                            let root_is_ref = self.field_access_root_is_behind_reference(object);
                            if root_is_ref {
                                let is_copy = ty.as_ref().is_some_and(|t| self.is_type_copy(t));
                                if !is_copy {
                                    s = format!("{}.clone()", s);
                                }
                            }
                        }
                    } else {
                        s = format!("{}.clone()", s);
                    }
                }
                s
            })
            .collect();
        format!("({})", expr_strs.join(", "))
    }

    pub(in crate::codegen::rust) fn generate_map_literal(
        &mut self,
        pairs: &[(&Expression<'ast>, &Expression<'ast>)],
    ) -> String {
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

    pub(in crate::codegen::rust) fn generate_array(
        &mut self,
        elements: &[&Expression<'ast>],
    ) -> String {
        use crate::parser::Literal;
        let expected_elem_ty = self.struct_array_field_element_type();
        let expr_strs: Vec<String> = elements
            .iter()
            .map(|e| {
                let mut s = self.generate_expression(e);
                if let Some(ref exp_ty) = expected_elem_ty {
                    if let Some(actual_ty) = self.infer_expression_type(e) {
                        let skip_float_literal = matches!(
                            e,
                            Expression::Literal {
                                value: Literal::Float(_),
                                ..
                            }
                        );
                        if !skip_float_literal {
                            if let Some(cast) = float_type_utilities::float_array_elem_cast_target(
                                exp_ty, &actual_ty,
                            ) {
                                s = format!("({} as {})", s, cast);
                            }
                        }
                    }
                }
                s
            })
            .collect();

        // WINDJAMMER PHILOSOPHY: Array literal syntax determines Rust output.
        //
        // In WJ, `[a, b, c]` is a fixed-size array literal → generates `[a, b, c]` in Rust.
        // In WJ, `vec![a, b, c]` is an explicit Vec constructor → generates `vec![a, b, c]`.
        //
        // Empty arrays `[]` remain `vec![]` because Rust's empty `[]` can't infer its type.
        //
        // This distinction is critical: `painter.line_segment([p1, p2], stroke)` expects
        // `[Pos2; 2]`, not `Vec<Pos2>`. The developer chose `[...]` syntax intentionally.
        if elements.is_empty() {
            // Empty array [] → vec![] (Vec::new())
            // Rust's [] is a fixed-size array and can't infer type from later usage.
            "vec![]".to_string()
        } else {
            // Non-empty array literals: generate fixed-size array [a, b, c]
            // The developer uses `vec![...]` macro syntax when Vec is needed.
            format!("[{}]", expr_strs.join(", "))
        }
    }

    pub(in crate::codegen::rust) fn generate_field_access(
        &mut self,
        object: &Expression<'ast>,
        field: &str,
        expr_to_generate: &Expression<'ast>,
    ) -> String {
        // FIELD CHAIN OPTIMIZATION: If we're accessing a Copy sub-field,
        // suppress borrowed-iterator cloning on the intermediate object.
        // In Rust, (&enemy).velocity.y works fine through auto-deref.
        let field_is_copy_by_type = self
            .infer_expression_type(expr_to_generate)
            .as_ref()
            .is_some_and(|t| self.is_type_copy(t));

        let prev_suppress = self.suppress_borrowed_clone;
        let prev_field_access = self.in_field_access_object;
        if field_is_copy_by_type {
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
        let separator = match object {
            Expression::Identifier { name, .. }
                if name.contains("::")
                    || (!name.is_empty() && name.chars().next().unwrap().is_uppercase()) =>
            {
                "::" // Module path: std::fs or Type::CONST
            }
            Expression::FieldAccess { .. }
                // Check if this is a module path or a field chain
                // If the object string contains ::, it's a module path
                if obj_str.contains("::") => {
                    "::" // Module path: std::fs::File
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
        // METHOD RECEIVER / FOR-LOOP FIX: Skip auto-clone when in a method receiver
        // or for-loop iterable context. Rust auto-borrows method receivers (&self),
        // and for-loops iterate by reference with `&`. Cloning is unnecessary and
        // breaks for Vec<Box<dyn Trait>> or Vec<T> where T may not be Clone.
        if !self.generating_assignment_target
            && !self.in_explicit_clone_call
            && !self.in_field_access_object
        {
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
                            // Fallback: field names that are universally numeric
                            // primitives across all domains (coordinates, dimensions,
                            // color channels, booleans). No game-specific names here.
                            let is_likely_copy_field = matches!(
                                field,
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
            && !self.in_user_written_closure
        {
            if let Expression::Identifier { name: var_name, .. } = object {
                if self.borrowed_iterator_vars.contains(var_name) {
                    // First: use type inference to check if the field type is Copy
                    let is_copy = self
                        .infer_expression_type(expr_to_generate)
                        .as_ref()
                        .is_some_and(|t| self.is_type_copy(t));

                    if !is_copy && !base_expr.ends_with(".clone()") {
                        return format!("{}.clone()", base_expr);
                    }
                }
            }
        }

        // Borrowed param field clone: when accessing param.field on a borrowed
        // parameter (&self or any &T param), non-Copy types can't be moved
        // out of the reference — auto-clone.
        // Skip in comparison contexts — refs compare fine without cloning.
        if !self.generating_assignment_target
            && !self.in_explicit_clone_call
            && !self.in_field_access_object
            && !self.in_borrow_context
            && !self.suppress_borrowed_clone
            && !self.in_call_argument_generation
            && !self.in_user_written_closure
        {
            if let Expression::Identifier { name: obj_name, .. } = object {
                if self.inferred_borrowed_params.contains(obj_name.as_str()) {
                    let field_is_copy = if obj_name == "self" && self.in_impl_block {
                        self.current_struct_name
                            .as_ref()
                            .and_then(|sn| self.lookup_struct_field_types(sn.as_str()))
                            .and_then(|fields| fields.get(field))
                            .is_some_and(|ty| self.is_type_copy(ty))
                    } else {
                        self.infer_expression_type(expr_to_generate)
                            .as_ref()
                            .is_some_and(|t| self.is_type_copy(t))
                    };
                    if !field_is_copy && !base_expr.ends_with(".clone()") {
                        return format!("{}.clone()", base_expr);
                    }
                }
            }
        }

        // VEC INDEX FIELD ACCESS: When accessing a non-Copy field through Vec
        // indexing (e.g., choices[i].text), Rust can't move out of a Vec element.
        // The Index handler suppresses its own borrow/clone when in_field_access_object
        // is true (correct for Copy fields like .score), but for non-Copy fields
        // like String, the resulting expression `vec[i].text` is still a move.
        // Fix: clone the field access result when the field type is non-Copy.
        if !self.generating_assignment_target
            && !self.in_explicit_clone_call
            && !self.in_field_access_object
            && !self.in_borrow_context
        {
            let object_has_index = matches!(object, Expression::Index { .. })
                || matches!(object, Expression::FieldAccess { object: inner, .. }
                    if matches!(&**inner, Expression::Index { .. }));

            if object_has_index && !field_is_copy_by_type {
                return format!("{}.clone()", base_expr);
            }
        }

        // Reference binding field access: `let step = &items[i]; step.str_value` in owned contexts.
        if !self.generating_assignment_target
            && !self.in_explicit_clone_call
            && !self.in_field_access_object
            && !self.in_borrow_context
            && !self.in_user_written_closure
            && (self.in_call_argument_generation
                || self.in_struct_literal_field
                || self.in_owned_value_context)
            && self.field_access_root_is_behind_reference(expr_to_generate)
            && !field_is_copy_by_type
            && !base_expr.ends_with(".clone()")
        {
            return format!("{}.clone()", base_expr);
        }

        base_expr
    }

    /// Generate code for struct literal expression Struct { field: value }
    /// Handles string coercion, field shorthand, auto-clone for borrowed self
    pub(in crate::codegen::rust) fn generate_struct_literal(
        &mut self,
        name: &str,
        fields: &[(String, &Expression<'ast>)],
    ) -> String {
        use crate::parser::{Literal, Type};

        // PHASE 3 OPTIMIZATION: Check if we have optimization hints for this struct
        let _has_optimization_hint = self.struct_mapping_hints.get(name);

        // CONTEXT-SENSITIVE INFERENCE: Set struct literal context for float type inference
        let prev_struct_name = self.current_struct_literal_name.clone();
        self.current_struct_literal_name = Some(name.to_string());

        let identifier_usage_counts =
            crate::codegen::rust::expression_helpers::count_identifier_usages_in_fields(fields);

        // Generate field assignments
        let field_str: Vec<String> = fields
            .iter()
            .map(|(field_name, expr)| {
                // STRUCT LITERAL CONTEXT: Array literals in struct fields should use
                // fixed-size [...] syntax, not vec![...], because struct fields have
                // explicit type annotations (e.g., position: [f32; 3]).
                let prev_in_struct_field = self.in_struct_literal_field;
                let prev_field_name = self.current_struct_field_name.clone();
                self.in_struct_literal_field = true;
                self.current_struct_field_name = Some(field_name.to_string());

                // WINDJAMMER PHILOSOPHY: Auto-convert string literals to String
                // In Windjammer, `string` type is always owned (maps to Rust String)
                // So string literals in struct fields should be converted automatically.
                // Set coercion flag BEFORE generation so nested expressions (if-else
                // branches, match arms, blocks) also coerce their string literals.
                let prev_coerce = self.coerce_string_literals_to_owned;
                self.coerce_string_literals_to_owned = true;
                let mut expr_str = self.generate_expression(expr);
                self.coerce_string_literals_to_owned = prev_coerce;

                // Restore previous context
                self.in_struct_literal_field = prev_in_struct_field;
                self.current_struct_field_name = prev_field_name;

                // Auto-convert direct string literals that weren't already coerced
                if matches!(
                    expr,
                    Expression::Literal {
                        value: Literal::String(_),
                        ..
                    }
                ) && !string_utilities::already_owned_string_expr(&expr_str) {
                    expr_str = string_utilities::coerce_expr_to_owned_string(&expr_str);
                }

                // Auto-convert borrowed string parameters to owned String for struct fields.
                // Windjammer `string` params are already Rust `String` — only coerce `&str`/reference params.
                if let Expression::Identifier { name: id, .. } = expr {
                    let is_borrowed_string_param = self.current_function_params.iter().any(|p| {
                        if p.name != *id {
                            return false;
                        }
                        match &p.type_ {
                            Type::Reference(inner) => matches!(**inner, Type::String)
                                || matches!(**inner, Type::Custom(ref name) if name == "str" || name == "string"),
                            _ => false,
                        }
                    });

                    if is_borrowed_string_param
                        && !string_utilities::already_owned_string_expr(&expr_str)
                    {
                        let struct_name = self.current_struct_literal_name.as_deref().unwrap_or("");
                        if let Some(field_types) = self.lookup_struct_field_types(struct_name) {
                            if let Some(field_type) = field_types.get(field_name) {
                                let field_is_string = matches!(field_type, Type::String)
                                    || matches!(field_type, Type::Custom(ref n) if n == "string" || n == "String");
                                if field_is_string {
                                    expr_str = string_utilities::coerce_expr_to_owned_string(&expr_str);
                                }
                            }
                        }
                    }

                    // Phase 2 &str param stored into owned String field → .to_string() at site.
                    if self.str_ref_optimized_params.contains(id) {
                        let struct_name = self.current_struct_literal_name.as_deref().unwrap_or("");
                        if let Some(field_types) = self.lookup_struct_field_types(struct_name) {
                            if let Some(field_type) = field_types.get(field_name) {
                                let field_is_string = matches!(field_type, Type::String)
                                    || matches!(field_type, Type::Custom(ref n) if n == "string" || n == "String");
                                if field_is_string && !expr_str.ends_with(".to_string()") {
                                    expr_str = format!("{}.to_string()", expr_str);
                                }
                            }
                        }
                    }
                }

                // Windjammer `string` params inferred as borrowed (`&String`/`&str`) need
                // `.clone()` when assigned to owned String struct fields.
                if let Expression::Identifier { name: id, .. } = expr {
                    if self.inferred_borrowed_params.contains(id) {
                        let struct_name = self.current_struct_literal_name.as_deref().unwrap_or("");
                        if let Some(field_types) = self.lookup_struct_field_types(struct_name) {
                            if let Some(field_type) = field_types.get(field_name) {
                                let field_is_string = matches!(field_type, Type::String)
                                    || matches!(field_type, Type::Custom(ref n) if n == "string" || n == "String");
                                if field_is_string && !expr_str.ends_with(".clone()") {
                                    if expr_str.ends_with(".to_string()") {
                                        // Already coerced from &str → String
                                    } else {
                                        expr_str = format!("{}.clone()", expr_str);
                                    }
                                }
                            }
                        }
                    }
                }

                // Iterator binding `for label in &self.tracked_labels` → `&String` into `String` field
                if let Expression::Identifier { name: id, .. } = expr {
                    if self.borrowed_iterator_vars.contains(id) {
                        let struct_name = self.current_struct_literal_name.as_deref().unwrap_or("");
                        if let Some(field_types) = self.lookup_struct_field_types(struct_name) {
                            if let Some(field_type) = field_types.get(field_name) {
                                let field_is_string = matches!(field_type, Type::String)
                                    || matches!(field_type, Type::Custom(ref n) if n == "string" || n == "String");
                                if field_is_string && !expr_str.ends_with(".clone()") {
                                    if expr_str.ends_with(".to_string()") {
                                        // Already coerced from &str → String
                                    } else {
                                        expr_str = format!("{}.clone()", expr_str);
                                    }
                                }
                            }
                        }
                    }
                }

                // For-loop iterator `for x in &vec`: field access in struct literal needs clone.
                if let Expression::FieldAccess { object, .. } = expr {
                    if let Expression::Identifier { name: obj_name, .. } = &**object {
                        if self.borrowed_iterator_vars.contains(obj_name)
                            && !expr_str.ends_with(".clone()")
                        {
                            let is_copy = self
                                .infer_expression_type(expr)
                                .as_ref()
                                .is_some_and(|t| self.is_type_copy(t));
                            if !is_copy {
                                expr_str = format!("{}.clone()", expr_str);
                            }
                        }
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

                // E0308: bindings from match/if-let on `&T` are `&U` when `U: Copy`
                if matches!(
                    expr,
                    Expression::Identifier { .. } | Expression::FieldAccess { .. }
                ) {
                    expr_str = self.peel_copy_ref_binding_for_struct_field(expr, &expr_str);
                    expr_str =
                        self.clone_non_copy_ref_binding_for_struct_field(expr, &expr_str);
                }

                // Check for field shorthand: if expr is just the field name AND no conversion applied, use shorthand
                // Only use shorthand if the generated expression exactly matches the field name
                // (no .to_string(), .clone(), etc. conversions)
                if let Expression::Identifier { name: id, .. } = expr {
                    if id == field_name && expr_str == *field_name {
                        let used_multiple_times = identifier_usage_counts
                            .get(id)
                            .is_some_and(|count| *count > 1);
                        if used_multiple_times {
                            let is_copy = self
                                .current_function_params
                                .iter()
                                .find(|p| p.name == *id)
                                .is_some_and(|p| self.is_type_copy(&p.type_));
                            if !is_copy {
                                return format!("{}: {}.clone()", field_name, field_name);
                            }
                        }
                        // Shorthand: User { name } instead of User { name: name }
                        // Only safe when no type conversion was needed
                        return field_name.clone();
                    }
                }

                format!("{}: {}", field_name, expr_str)
            })
            .collect();

        // Restore struct literal context
        self.current_struct_literal_name = prev_struct_name;

        let qualified_name = self.qualify_external_path_identifier(name);
        format!("{} {{ {} }}", qualified_name, field_str.join(", "))
    }

    /// Generate code for index expression array[index]
    /// Handles auto-cast to usize, slice syntax, auto-borrow/clone for non-Copy elements
    pub(in crate::codegen::rust) fn generate_index(
        &mut self,
        object: &Expression<'ast>,
        index: &Expression<'ast>,
        expr_to_generate: &Expression<'ast>,
    ) -> String {
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
        } = index
        {
            let start_str = self.generate_expression(start);
            let end_str = self.generate_expression(end);
            let range_op = if *inclusive { "..=" } else { ".." };
            return format!("{}[{}{}{}]", obj_str, start_str, range_op, end_str);
        }

        let mut idx_str = self.generate_expression(index);

        self.maybe_cast_index_to_usize(&mut idx_str, index);
        let final_idx = idx_str;

        let base_expr = format!("{}[{}]", obj_str, final_idx);

        // WINDJAMMER PHILOSOPHY: Auto-borrow Vec indexing for non-Copy types (E0507 fix).
        // Rust doesn't allow moving out of a Vec index (E0507).
        // For Copy types: vec[idx] works directly (value is copied).
        // For non-Copy types: &vec[idx] (borrow) or vec[idx].clone() when owned needed.
        //
        // PREFER BORROW over clone: &vec[idx] is zero-cost; .clone() allocates.
        //
        // CRITICAL: NEVER add & or .clone() in these contexts:
        // 1. Assignment target: vec[i] = value (can't assign to .clone() or &)
        // 2. Borrow context: &vec[i] (parent adds &, we output vec[idx] only)
        // 3. Field access: vec[i].field (Rust allows field access through ref)
        // 4. Comparison context: vec[i] == val (comparisons work on &T)
        let suppress_borrow_or_clone = self.generating_assignment_target
            || self.in_borrow_context
            || self.in_field_access_object
            || self.suppress_borrowed_clone;

        // TDD: Struct literal fields need owned values - force .clone() for Vec<String> etc.
        // Peel &Vec<T> (generated Rust for WJ `Vec<T>` params) so Copy element detection works.
        let element_type = self
            .infer_expression_type(object)
            .as_ref()
            .and_then(|t| Self::peeled_collection_element_type(t))
            .cloned();
        let force_clone_for_owned_context = (self.in_struct_literal_field
            || self.in_owned_value_context)
            && element_type
                .as_ref()
                .map(|et| !self.is_type_copy(et))
                .unwrap_or(true)
            && !self.in_borrow_context
            && !self.generating_assignment_target;

        let suppress_borrow_or_clone = suppress_borrow_or_clone && !force_clone_for_owned_context;

        if !suppress_borrow_or_clone {
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
                            // Path analysis says clone needed (e.g. passed to owned param)
                            return format!("{}.clone()", base_expr);
                        }
                    }
                }
            }

            // Fallback: Type-based handling for Vec<NonCopy>[idx]
            // E0507 fix: vec[idx] for String tries to move → use &vec[idx] (borrow)
            // When owned value needed (struct literal): vec[idx].clone()
            let element_resolution = self
                .infer_expression_type(object)
                .as_ref()
                .and_then(|obj_ty| Self::peeled_collection_element_type(obj_ty))
                .map(|elem_type| (true, !self.is_type_copy(elem_type)));

            match element_resolution {
                Some((_, true)) => {
                    if force_clone_for_owned_context {
                        return format!("{}.clone()", base_expr);
                    } else {
                        return format!("&{}", base_expr);
                    }
                }
                Some((_, false)) => {
                    // Copy type — bare indexing is fine
                }
                None => {
                    // Unknown element type: use .clone() as a safe default.
                    // .clone() works for both Copy (trivial copy) and non-Copy
                    // (deep clone). Avoids E0507 without changing the expression
                    // type the way & would.
                    return format!("{}.clone()", base_expr);
                }
            }
        }

        // `Vec<T>` / slice indexing in Rust already yields `T` for `T: Copy` in value
        // contexts (via the `Index` trait's desugaring). Emitting `*(vec[i])` was an
        // attempted E0308 workaround but is invalid: for `Copy` elements the inner
        // expression is already `T`, so `*` triggers E0614 for both owned and `&Vec<T>`
        // receivers.

        base_expr
    }
}
