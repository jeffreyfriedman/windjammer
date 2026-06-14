impl FloatInference {
    /// Used when let x = expr has no explicit type - infer from expr for assert_eq!(x.field, literal)
    /// TDD FIX: Added Binary and MethodCall fallback for len > 0.0 pattern (physics/advanced_collision.wj)
    fn infer_type_from_expression<'ast>(&self, expr: &Expression<'ast>) -> Option<Type> {
        match expr {
            Expression::StructLiteral { name, .. } => Some(Type::Custom(name.clone())),
            Expression::Binary {
                left, right, op, ..
            } => {
                use crate::parser::ast::operators::BinaryOp;
                if matches!(
                    op,
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
                ) {
                    let left_ty = self.infer_type_from_expression(left)?;
                    let right_ty = self.infer_type_from_expression(right)?;
                    if left_ty == right_ty {
                        return Some(left_ty);
                    }
                }
                None
            }
            // TDD: `let x = (n as f32) / (m as f32)` must record x as f32 so `x < 0.3` constrains the literal.
            Expression::Cast { type_, .. } => {
                self.extract_float_type(type_).and_then(|ft| match ft {
                    FloatType::F32 => Some(Type::Custom("f32".to_string())),
                    FloatType::F64 => Some(Type::Custom("f64".to_string())),
                    FloatType::Unknown => None,
                })
            }
            Expression::Call { function, .. } => {
                // Parser desugars `receiver.method(args)` as Call(FieldAccess(receiver, method), args).
                // HashMap/Map `.get` must infer like MethodCall so `match m.get(..)` gets arm float context.
                if let Expression::FieldAccess { object, field, .. } = function {
                    if field == "get" {
                        if let Some(object_type) = self.infer_type_from_expression(object) {
                            if let Some(value_ty) = self.extract_map_value_type(&object_type) {
                                return Some(Type::Option(Box::new(value_ty)));
                            }
                        }
                    }
                }
                // Type::new() or Type::method() - get return type from function signature
                let func_name = match function {
                    Expression::FieldAccess { object, field, .. } => {
                        if let Expression::Identifier {
                            name: type_name, ..
                        } = object
                        {
                            Some(format!("{}::{}", type_name, field))
                        } else {
                            None
                        }
                    }
                    Expression::Identifier { name, .. } => Some(name.clone()),
                    _ => None,
                };
                func_name.and_then(|name| {
                    self.function_signatures
                        .get(&name)
                        .and_then(|(_, ret)| ret.clone())
                })
            }
            Expression::MethodCall { object, method, .. } => {
                // object.method() - need object's type to find method signature
                let object_type = self.infer_type_from_expression(object)?;
                // TDD: Map<K,V>::get / HashMap::get → Option<V> (match arms need float context)
                if method == "get" {
                    if let Some(value_ty) = self.extract_map_value_type(&object_type) {
                        return Some(Type::Option(Box::new(value_ty)));
                    }
                }
                let type_name = match &object_type {
                    Type::Custom(name) => name.clone(),
                    _ => return None,
                };
                let full_name = format!("{}::{}", type_name, method);
                if let Some((_, ret)) = self.function_signatures.get(&full_name) {
                    return ret.clone();
                }
                // TDD FIX: Fallback for primitive methods — return same float type as receiver.
                // Keep in sync with `determine_method_return_type` F32_METHODS (subset used here).
                const PRIMITIVE_SAME_TYPE: &[&str] = &[
                    "sqrt",
                    "sin",
                    "cos",
                    "tan",
                    "asin",
                    "acos",
                    "atan",
                    "atan2",
                    "abs",
                    "floor",
                    "ceil",
                    "round",
                    "length",
                    "magnitude",
                    "distance",
                    "dot",
                    "recip",
                    "powf",
                    "powi",
                    "exp",
                    "ln",
                    "log",
                    "log2",
                    "to_degrees",
                    "to_radians",
                ];
                if PRIMITIVE_SAME_TYPE.contains(&method.as_str())
                    && (matches!(object_type, Type::Custom(ref s) if s == "f32" || s == "f64")
                        || matches!(object_type, Type::Float))
                {
                    return Some(object_type);
                }
                None
            }
            Expression::Identifier { name, .. } => {
                if name == "self" {
                    self.current_impl_type
                        .as_ref()
                        .map(|s| Type::Custom(s.clone()))
                } else {
                    self.var_types
                        .get(name)
                        .or_else(|| self.const_types.get(name))
                        .cloned()
                }
            }
            Expression::FieldAccess { object, field, .. } => {
                let object_type = self.infer_type_from_expression(object)?;
                let struct_name = match &object_type {
                    Type::Custom(name) => name.clone(),
                    _ => return None,
                };
                let base_name = if let Some(idx) = struct_name.find('<') {
                    &struct_name[..idx]
                } else {
                    struct_name.as_str()
                };
                let fields = if matches!(
                    *object,
                    Expression::Identifier { ref name, .. } if name == "self"
                ) {
                    self.current_impl_type
                        .as_deref()
                        .and_then(|ty| self.lookup_struct_fields_for_impl_type(ty))
                } else {
                    self.lookup_struct_fields(base_name)
                };
                fields.and_then(|m| m.get(field)).cloned()
            }
            Expression::Index { object, .. } => {
                let object_type = self.infer_type_from_expression(object)?;
                self.extract_vec_element_type(&object_type)
            }
            Expression::Unary { op, operand, .. } => {
                use crate::parser::ast::operators::UnaryOp;
                if matches!(op, UnaryOp::Deref) {
                    self.infer_type_from_expression(operand)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Extract FloatType from a Type
    fn extract_float_type(&self, ty: &Type) -> Option<FloatType> {
        match ty {
            Type::Float => Some(FloatType::F64),
            Type::Custom(name) if name == "f32" => Some(FloatType::F32),
            Type::Custom(name) if name == "f64" => Some(FloatType::F64),
            Type::Custom(name) => {
                // Resolve type aliases: e.g., Quat = (f32, f32, f32, f32)
                if let Some(resolved) = self.type_aliases.get(name.as_str()) {
                    self.extract_float_type(resolved)
                } else {
                    None
                }
            }
            Type::Tuple(types) => {
                for t in types {
                    if let Some(float_ty) = self.extract_float_type(t) {
                        return Some(float_ty);
                    }
                }
                None
            }
            Type::Vec(inner) | Type::Array(inner, _) => self.extract_float_type(inner),
            Type::Option(inner) => self.extract_float_type(inner),
            Type::Result(ok_type, _) => self.extract_float_type(ok_type),
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.extract_float_type(inner)
            }
            Type::Parameterized(name, type_args) => {
                let base = crate::type_inference::generic_type_base_name(name);
                if (base == "Vec" || base == "Option" || base == "Result") && !type_args.is_empty()
                {
                    self.extract_float_type(&type_args[0])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Value type `V` from `HashMap<K, V>` or Windjammer `Map<K, V>`.
    fn extract_map_value_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Parameterized(name, type_args) => {
                let base = crate::type_inference::generic_type_base_name(name);
                if matches!(base, "HashMap" | "Map" | "BTreeMap") && type_args.len() >= 2 {
                    Some(type_args[1].clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// If `receiver` is map-like and `V` is a scalar float, return that float type.
    fn map_receiver_value_float_type<'ast>(
        &self,
        receiver: &Expression<'ast>,
    ) -> Option<FloatType> {
        let object_type = self.infer_type_from_expression(receiver)?;
        let value_ty = self.extract_map_value_type(&object_type)?;
        self.extract_float_type(&value_ty)
    }

    /// TDD FIX: Extract value type V from HashMap<K, V> (alias for map-like containers)
    fn extract_hashmap_value_type(&self, ty: &Type) -> Option<Type> {
        self.extract_map_value_type(ty)
    }

    /// TDD FIX: Extract element type T from Vec<T>
    fn extract_vec_element_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Vec(inner) => Some((**inner).clone()),
            Type::Parameterized(name, type_args) if name == "Vec" => {
                // Vec<T> has 1 type argument
                if !type_args.is_empty() {
                    Some(type_args[0].clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Peel `Option` / `Result` / references until we reach a concrete type, then keep it if it is f32/f64.
    fn float_type_after_peeling_wrappers(&self, mut ty: Type) -> Option<Type> {
        loop {
            match ty {
                Type::Option(inner) => ty = (*inner).clone(),
                Type::Result(ok, _) => ty = (*ok).clone(),
                Type::Reference(inner) | Type::MutableReference(inner) => ty = (*inner).clone(),
                Type::Parameterized(name, ref args) if name == "Option" && args.len() == 1 => {
                    ty = args[0].clone();
                }
                Type::Parameterized(name, ref args) if name == "Result" && !args.is_empty() => {
                    ty = args[0].clone();
                }
                _ => break,
            }
        }
        if self.extract_float_type(&ty).is_some() {
            Some(ty)
        } else {
            None
        }
    }

    /// Get unique ID for an expression (based on source location)
    /// Get unique ID for expression with location-based caching
    /// THE WINDJAMMER WAY: Cache by location to ensure same expression = same ID
    /// This fixes the problem where same expression got multiple IDs during traversal
    fn get_expr_id<'ast>(&mut self, expr: &Expression<'ast>) -> ExprId {
        let location = expr.location();
        let (line, col) = if let Some(loc) = location {
            (loc.line, loc.column)
        } else {
            (0, 0)
        };

        // TDD FIX: Use file-aware cache key to prevent cross-file collisions
        let cache_key = (self.current_file_id, line, col);

        // Check cache first - if we've seen this location before, return same ID
        if line > 0 {
            // Only cache expressions with valid locations
            if let Some(&cached_id) = self.expr_id_cache.get(&cache_key) {
                return cached_id;
            }
        }

        // Generate new sequential ID (globally unique across all files)
        let seq_id = self.next_seq_id;
        self.next_seq_id += 1;

        let expr_id = ExprId {
            seq_id,
            file_id: self.current_file_id,
            line,
            col,
        };

        // Cache it for future lookups
        if line > 0 {
            self.expr_id_cache.insert(cache_key, expr_id);
        }

        expr_id
    }

    /// Determine the return type of a method call
    /// Returns Some(FloatType) if the method is known to return f32/f64, None otherwise
    fn determine_method_return_type(&self, object: &Expression, method: &str) -> Option<FloatType> {
        // TDD FIX: For methods on f32/f64 primitives, return the same type
        // Standard library f32 methods that return f32:
        const F32_METHODS: &[&str] = &[
            // Trigonometric
            "sin",
            "cos",
            "tan",
            "asin",
            "acos",
            "atan",
            "atan2",
            "sinh",
            "cosh",
            "tanh",
            "asinh",
            "acosh",
            "atanh",
            // Exponential/logarithmic
            "exp",
            "exp2",
            "exp_m1",
            "ln",
            "log",
            "log2",
            "log10",
            "ln_1p",
            // Power/root
            "sqrt",
            "cbrt",
            "hypot",
            "powf",
            "powi",
            // Rounding
            "floor",
            "ceil",
            "round",
            "trunc",
            "fract",
            // Absolute/sign
            "abs",
            "signum",
            "copysign",
            // Min/max
            "max",
            "min",
            "clamp",
            // Misc
            "recip",
            "to_degrees",
            "to_radians",
        ];

        // Check if this is a method call on an identifier
        if let Expression::Identifier { name, .. } = object {
            // Look up the identifier's type from var_types
            if let Some(var_type) = self.var_types.get(name) {
                // Check if it's an f32 or f64 type
                let is_f32 = matches!(var_type, Type::Float)
                    || matches!(var_type, Type::Custom(s) if s == "f32");
                let is_f64 = matches!(var_type, Type::Custom(s) if s == "f64");

                if is_f32 && F32_METHODS.contains(&method) {
                    return Some(FloatType::F32);
                }
                if is_f64 && F32_METHODS.contains(&method) {
                    return Some(FloatType::F64);
                }
            }
        }

        // Method on a field (e.g. self.vy.sqrt(), pos.x.acos()): infer receiver type from the
        // field-access expression. Do NOT scan function_signatures by method basename — HashMap
        // order could pick `f64::acos` while the receiver is f32, forcing spurious f64 promotion.
        if let Expression::FieldAccess { .. } = object {
            if let Some(ty) = self.infer_type_from_expression(object) {
                let is_f32 =
                    matches!(&ty, Type::Float) || matches!(&ty, Type::Custom(s) if s == "f32");
                let is_f64 = matches!(&ty, Type::Custom(s) if s == "f64");
                if is_f32 && F32_METHODS.contains(&method) {
                    return Some(FloatType::F32);
                }
                if is_f64 && F32_METHODS.contains(&method) {
                    return Some(FloatType::F64);
                }
            }
        }

        // For MethodCall on MethodCall (chaining), try to infer from the inner call.
        // Guard: only treat as a float method if the receiver actually IS a float type.
        // Struct builder methods like `Slider::max` must NOT collide with `f32::max`.
        if let Expression::MethodCall { .. } = object {
            if F32_METHODS.contains(&method) {
                if let Some(object_type) = self.infer_type_from_expression(object) {
                    if let Some(float_ty) = self.extract_float_type(&object_type) {
                        return Some(float_ty);
                    }
                    // Receiver is a known non-float type (e.g. Slider) — not a numeric method.
                    return None;
                }
                // Can't determine receiver type — fall back to f32 heuristic for
                // chained math like `vec.x.sin().cos()` where type info isn't available.
                return Some(FloatType::F32);
            }
        }

        // TDD FIX: For MethodCall on Binary (e.g., (x*x + y*y).sqrt()), infer from operands
        // Handles physics/advanced_collision.wj: len = (edge_x*edge_x + edge_y*edge_y).sqrt()
        if let Expression::Binary { .. } = object {
            if F32_METHODS.contains(&method) {
                if let Some(object_type) = self.infer_type_from_expression(object) {
                    return self.extract_float_type(&object_type);
                }
            }
        }

        None
    }
}
