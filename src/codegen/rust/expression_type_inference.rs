//! Infers [`Type`] from expressions; stdlib and primitive-float method returns.

use crate::codegen::rust::CodeGenerator;
use crate::parser::{Expression, Literal, Statement, Type};

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {
    /// `f32::sin` / `f64::ln` etc. return the same float type as the receiver. Without this,
    /// codegen falls through to unqualified `acos` from `std/math.wj` (`f64 -> f64`) and
    /// `float_class_for_binary_operand` inserts spurious `as f64` next to real f32 values.
    fn rust_primitive_float_method_return_type(
        receiver: Option<&Type>,
        method: &str,
    ) -> Option<Type> {
        const SAME_FLOAT_RETURN: &[&str] = &[
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
            "exp",
            "exp2",
            "exp_m1",
            "ln",
            "log",
            "log2",
            "log10",
            "ln_1p",
            "sqrt",
            "cbrt",
            "hypot",
            "powf",
            "powi",
            "floor",
            "ceil",
            "round",
            "trunc",
            "fract",
            "abs",
            "signum",
            "copysign",
            "max",
            "min",
            "clamp",
            "recip",
            "to_degrees",
            "to_radians",
        ];
        if !SAME_FLOAT_RETURN.contains(&method) {
            return None;
        }
        let mut t = receiver?;
        loop {
            match t {
                Type::Reference(inner) | Type::MutableReference(inner) => t = inner.as_ref(),
                Type::Custom(s) if s == "f32" || s == "f64" => {
                    return Some(Type::Custom(s.clone()));
                }
                Type::Float => return Some(Type::Custom("f32".to_string())),
                _ => return None,
            }
        }
    }

    /// Try to infer the Type of an expression from local variable tracking and function parameters.
    pub(in crate::codegen::rust) fn infer_expression_type(
        &self,
        expr: &Expression,
    ) -> Option<Type> {
        match expr {
            Expression::Identifier { name, .. } => {
                // Check local variable types first
                if let Some(t) = self.local_var_types.get(name) {
                    return Some(t.clone());
                }
                // Check function parameters
                for param in &self.current_function_params {
                    if param.name == *name {
                        return Some(param.type_.clone());
                    }
                }
                // In impl blocks, identifiers may refer to struct fields (implicit self)
                // e.g., `mouse_x` in `impl Game` → `self.mouse_x` → type is Game.mouse_x's type
                if self.in_impl_block && self.current_struct_fields.contains(name) {
                    if let Some(struct_name) = &self.current_struct_name {
                        if let Some(fields) = self.struct_field_types.get(struct_name.as_str()) {
                            if let Some(field_type) = fields.get(name.as_str()) {
                                return Some(field_type.clone());
                            }
                        }
                    }
                }
                None
            }
            // obj.field → look up field type from struct_field_types
            // Supports: self.field, var.field, and nested: self.config.max_size
            Expression::FieldAccess { object, field, .. } => {
                // Resolve the object's type first
                if let Expression::Identifier { name, .. } = &**object {
                    if name == "self" {
                        // self.field → current struct's field type
                        // TDD FIX: Also try base name for generic types
                        // e.g., "ComponentArray<T>" → try "ComponentArray"
                        if let Some(struct_name) = &self.current_struct_name {
                            let base = struct_name
                                .split('<')
                                .next()
                                .unwrap_or(struct_name.as_str());
                            let resolve = || {
                                self.struct_field_types
                                    .get(struct_name.as_str())
                                    .or_else(|| self.struct_field_types.get(base))
                            };
                            if let Some(fields) = resolve() {
                                if let Some(field_type) = fields.get(field.as_str()) {
                                    return Some(field_type.clone());
                                }
                            }
                            // Library dogfood: registry keys are often `dir::file::StructName`.
                            // Duplicate basenames make unqualified lookup miss; qualify like float inference.
                            if let Some(src_root) = self.library_source_root.as_ref() {
                                if !self.current_wj_file.as_os_str().is_empty() {
                                    if let Some(module_path) =
                                        crate::analyzer::type_collector::wj_file_to_module_path(
                                            src_root,
                                            &self.current_wj_file,
                                        )
                                    {
                                        let key =
                                            crate::type_inference::struct_field_registry::qualify_struct_key(
                                                &module_path,
                                                base,
                                            );
                                        if let Some(fields) = self.struct_field_types.get(&key) {
                                            if let Some(field_type) = fields.get(field.as_str()) {
                                                return Some(field_type.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // var.field → look up var's type, then its field
                        // Check local variables first, then function parameters
                        let var_type =
                            self.local_var_types
                                .get(name.as_str())
                                .cloned()
                                .or_else(|| {
                                    self.current_function_params
                                        .iter()
                                        .find(|p| p.name == *name)
                                        .map(|p| p.type_.clone())
                                });
                        if let Some(var_type) = var_type {
                            let type_name = match &var_type {
                                Type::Custom(n) => n.as_str(),
                                // Handle references: &Recipe → Recipe, &mut Recipe → Recipe
                                Type::Reference(inner) | Type::MutableReference(inner) => {
                                    match inner.as_ref() {
                                        Type::Custom(n) => n.as_str(),
                                        _ => "",
                                    }
                                }
                                _ => "",
                            };
                            if let Some(fields) = self.struct_field_types.get(type_name) {
                                if let Some(field_type) = fields.get(field.as_str()) {
                                    return Some(field_type.clone());
                                }
                            }
                            // Qualified name fallback: when simple name lookup fails
                            // (e.g., ambiguous struct names across modules), try
                            // qualifying with the current module path.
                            if !type_name.is_empty() {
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
                                                type_name,
                                            );
                                            if let Some(fields) = self.struct_field_types.get(&key)
                                            {
                                                if let Some(field_type) = fields.get(field.as_str())
                                                {
                                                    return Some(field_type.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Nested field access: self.config.max_size, obj.inner.field, etc.
                    // Recursively resolve the object's type, then look up the field
                    if let Some(obj_type) = self.infer_expression_type(object) {
                        let type_name = match &obj_type {
                            Type::Custom(n) => n.as_str(),
                            // Handle references: &Config → Config
                            Type::Reference(inner) | Type::MutableReference(inner) => {
                                match inner.as_ref() {
                                    Type::Custom(n) => n.as_str(),
                                    _ => "",
                                }
                            }
                            _ => "",
                        };
                        if !type_name.is_empty() {
                            // Also try stripping generic params: "Config<T>" → "Config"
                            let base_name = type_name.split('<').next().unwrap_or(type_name);
                            if let Some(fields) = self
                                .struct_field_types
                                .get(type_name)
                                .or_else(|| self.struct_field_types.get(base_name))
                            {
                                if let Some(field_type) = fields.get(field.as_str()) {
                                    return Some(field_type.clone());
                                }
                            }
                        }
                    }
                }
                None
            }
            // &expr or &mut expr → Reference(inner_type)
            Expression::Unary {
                op: crate::parser::UnaryOp::Ref,
                operand,
                ..
            } => self
                .infer_expression_type(operand)
                .map(|t| Type::Reference(Box::new(t))),
            Expression::Unary {
                op: crate::parser::UnaryOp::MutRef,
                operand,
                ..
            } => self
                .infer_expression_type(operand)
                .map(|t| Type::MutableReference(Box::new(t))),
            // *expr → unwrap Reference/MutableReference to get inner type
            Expression::Unary {
                op: crate::parser::UnaryOp::Deref,
                operand,
                ..
            } => self.infer_expression_type(operand).and_then(|t| match t {
                Type::Reference(inner) | Type::MutableReference(inner) => Some(*inner),
                _ => Some(t),
            }),
            // Method calls: look up return type from method_return_types registry
            // and signature registry (for cross-file method resolution)
            Expression::MethodCall { object, method, .. } => {
                // Check well-known methods first
                if method == "len" || method == "count" || method == "capacity" {
                    return Some(Type::Custom("usize".to_string()));
                }
                // .clone() returns the same type as the object
                // This enables type inference through cloned iterables:
                //   for x in &collection.clone() → x has same element type as collection
                if method == "clone" {
                    return self.infer_expression_type(object);
                }
                // TDD FIX: .unwrap() on Option<T> → T
                if method == "unwrap" {
                    if let Some(obj_type) = self.infer_expression_type(object) {
                        if let Type::Option(inner) = obj_type {
                            return Some(*inner);
                        }
                    }
                }
                // Iterator methods: return the collection type so
                // extract_iterator_element_type can extract the element type.
                // This enables type inference for loop variables:
                //   for brick in self.bricks.iter_mut() → brick: Brick
                if method == "iter" || method == "iter_mut" || method == "into_iter" {
                    if let Some(obj_type) = self.infer_expression_type(object) {
                        return Some(obj_type);
                    }
                }
                let obj_ty = self.infer_expression_type(object);
                if let Some(ref ot) = obj_ty {
                    if let Some(ret) = Self::stdlib_method_return_type(ot, method) {
                        return Some(ret);
                    }
                }
                if let Some(t) =
                    Self::rust_primitive_float_method_return_type(obj_ty.as_ref(), method.as_str())
                {
                    return Some(t);
                }
                // Look up from the method return type registry (populated during impl generation)
                if let Some(t) = self.method_return_types.get(method.as_str()) {
                    return Some(t.clone());
                }
                // TDD FIX: Cross-file method resolution via signature registry.
                // When the method is on a different type (e.g., animation.frame_count()),
                // method_return_types won't have it. Resolve the object's type, then
                // look up Type::method in the signature registry.
                if let Some(obj_type) = obj_ty {
                    let type_name = match &obj_type {
                        Type::Custom(n) => n.clone(),
                        Type::Reference(inner) | Type::MutableReference(inner) => {
                            match inner.as_ref() {
                                Type::Custom(n) => n.clone(),
                                _ => String::new(),
                            }
                        }
                        _ => String::new(),
                    };
                    if !type_name.is_empty() {
                        let qualified = format!("{}::{}", type_name, method);
                        if let Some(sig) = self.signature_registry.get_signature(&qualified) {
                            return sig.return_type.clone();
                        }
                        // Also try base name for generic types
                        let base_name = type_name.split('<').next().unwrap_or(&type_name);
                        if base_name != type_name {
                            let qualified = format!("{}::{}", base_name, method);
                            if let Some(sig) = self.signature_registry.get_signature(&qualified) {
                                return sig.return_type.clone();
                            }
                        }
                    }
                }
                // Final fallback: try simple method name
                self.signature_registry
                    .get_signature(method)
                    .and_then(|sig| sig.return_type.clone())
            }
            // Block expression: infer from the last statement's expression
            // Handles: let x = { if cond { 64.0 } else { 32.0 } }
            Expression::Block { statements, .. } => {
                if let Some(last_stmt) = statements.last() {
                    match last_stmt {
                        Statement::Expression { expr, .. } => self.infer_expression_type(expr),
                        Statement::If { then_block, .. } => {
                            // Infer from the then branch's last expression
                            if let Some(last) = then_block.last() {
                                if let Statement::Expression { expr, .. } = last {
                                    return self.infer_expression_type(expr);
                                }
                            }
                            None
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            // Literal expressions: directly known types
            Expression::Literal { value, .. } => match value {
                Literal::Int(_) => Some(Type::Int),
                // `0_usize`, `256_i64`, etc. — map suffix to Rust primitive name for comparisons/codegen.
                Literal::IntSuffixed(_, suffix) => Some(Type::Custom(suffix.clone())),
                Literal::Float(_) => Some(Type::Float),
                Literal::Bool(_) => Some(Type::Bool),
                Literal::String(_) => Some(Type::String),
                _ => None,
            },
            // Binary operations: infer from operands (result usually matches operand type)
            Expression::Binary { left, right, .. } => self
                .infer_expression_type(left)
                .or_else(|| self.infer_expression_type(right)),
            // Cast expressions: the target type is explicit
            Expression::Cast { type_, .. } => Some(type_.clone()),
            // Call expressions: Type::method(args) → look up return type from signature registry
            // This is critical for Copy-type inference: let u = MathHelper::fade(x) → u is f32
            Expression::Call { function, .. } => {
                // Extract function name for signature lookup
                // Pattern: Type::method() → "Type::method"
                if let Expression::FieldAccess { object, field, .. } = function {
                    if let Expression::Identifier {
                        name: type_name, ..
                    } = object
                    {
                        let qualified = format!("{}::{}", type_name, field);
                        if let Some(sig) = self.signature_registry.get_signature(&qualified) {
                            if let Some(ref ret) = sig.return_type {
                                return Some(ret.clone());
                            }
                            // Constructor convention: Type::new() returns Type
                            // even when metadata has return_type: null
                            if !sig.has_self_receiver
                                && type_name.starts_with(|c: char| c.is_ascii_uppercase())
                            {
                                return Some(Type::Custom(type_name.clone()));
                            }
                        }
                        // Even without signature, CamelCase::method() likely returns CamelCase
                        if type_name.starts_with(|c: char| c.is_ascii_uppercase()) {
                            return Some(Type::Custom(type_name.clone()));
                        }
                    }
                    // Instance call: Call(FieldAccess(receiver, method), args) — same return type
                    // rules as MethodCall so we do not fall through to unqualified `acos` → f64.
                    let recv_ty = self.infer_expression_type(object);
                    if let Some(t) = Self::rust_primitive_float_method_return_type(
                        recv_ty.as_ref(),
                        field.as_str(),
                    ) {
                        return Some(t);
                    }
                }
                // Pattern: simple function call → "function_name"
                if let Expression::Identifier { name, .. } = function {
                    if let Some(sig) = self.signature_registry.get_signature(name.as_str()) {
                        return sig.return_type.clone();
                    }
                }
                None
            }
            // TDD FIX: Index expressions: vec[i] → element type of the collection
            // Example: let mask: Vec<u8> = ...; let color_id = mask[i]; → color_id: u8
            // Peel `&Vec<T>` / `&mut Vec<T>` so `vals: &Vec<f32>` still yields `f32`.
            Expression::Index { object, .. } => self
                .infer_expression_type(object)
                .as_ref()
                .and_then(|ot| Self::peeled_collection_element_type(ot))
                .cloned(),
            // TDD FIX: Macro invocations return known types
            // format!() always returns String
            // vec![] returns Vec<T> (but we don't infer T here)
            Expression::MacroInvocation {
                name,
                args,
                is_repeat: _,
                ..
            } => {
                match name.as_str() {
                    "format" => Some(Type::String),
                    "panic" => None, // Never returns (diverges)
                    "println" | "print" | "eprintln" | "eprint" => None, // Returns ()
                    "vec" => {
                        // `let v = vec![1.0, 2.0]` must register `Vec<Float>` so `v[i]` knows the
                        // element is Copy and we do not emit `&v[i]` (E0308) or `*&v[i]` (E0614).
                        let elem_ty = args.first().and_then(|e| self.infer_expression_type(e));
                        elem_ty.map(|t| Type::Vec(Box::new(t)))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Infer the return type of a method call on a known Rust stdlib type.
    /// Driven entirely by the receiver's inferred type — the method name selects
    /// the correct return type for that specific receiver type.
    ///
    /// Example: `Vec<T>.get(i)` → `Option<&T>`, `HashMap<K,V>.get(k)` → `Option<&V>`.
    fn stdlib_method_return_type(receiver: &Type, method: &str) -> Option<Type> {
        let inner = Self::peel_references(receiver);

        match inner {
            Type::String => Self::string_method_return_type(method),
            _ => Self::collection_method_return_type(receiver, method),
        }
    }

    /// Return types for String/&str methods.
    fn string_method_return_type(method: &str) -> Option<Type> {
        match method {
            "as_str" | "trim" | "trim_start" | "trim_end" | "trim_start_matches"
            | "trim_end_matches" | "trim_matches" => Some(Type::Reference(Box::new(Type::String))),
            "strip_prefix" | "strip_suffix" => Some(Type::Option(Box::new(Type::Reference(
                Box::new(Type::String),
            )))),
            "to_lowercase" | "to_uppercase" | "to_ascii_lowercase" | "to_ascii_uppercase"
            | "repeat" | "replace" | "replacen" => Some(Type::String),
            "len" | "capacity" => Some(Type::Custom("usize".to_string())),
            "is_empty"
            | "contains"
            | "starts_with"
            | "ends_with"
            | "is_ascii"
            | "eq_ignore_ascii_case" => Some(Type::Bool),
            "find" | "rfind" => Some(Type::Option(Box::new(Type::Custom("usize".to_string())))),
            "chars" | "bytes" | "lines" | "split_whitespace" | "split" | "splitn" | "rsplitn" => {
                None
            } // iterator types
            _ => None,
        }
    }

    fn collection_method_return_type(receiver: &Type, method: &str) -> Option<Type> {
        let inner = Self::peel_references(receiver);

        match inner {
            Type::Vec(elem) | Type::Array(elem, _) => match method {
                "get" | "first" | "last" => {
                    Some(Type::Option(Box::new(Type::Reference(elem.clone()))))
                }
                "get_mut" | "first_mut" | "last_mut" => {
                    Some(Type::Option(Box::new(Type::MutableReference(elem.clone()))))
                }
                _ => None,
            },
            Type::Parameterized(name, params) => {
                let base = name.split('<').next().unwrap_or(name.as_str());
                match base {
                    "HashMap" | "BTreeMap" | "IndexMap" if params.len() >= 2 => match method {
                        "get" => Some(Type::Option(Box::new(Type::Reference(Box::new(
                            params[1].clone(),
                        ))))),
                        "get_mut" => Some(Type::Option(Box::new(Type::MutableReference(
                            Box::new(params[1].clone()),
                        )))),
                        _ => None,
                    },
                    "VecDeque" | "LinkedList" if !params.is_empty() => match method {
                        "get" | "front" | "back" => Some(Type::Option(Box::new(Type::Reference(
                            Box::new(params[0].clone()),
                        )))),
                        "get_mut" | "front_mut" | "back_mut" => Some(Type::Option(Box::new(
                            Type::MutableReference(Box::new(params[0].clone())),
                        ))),
                        _ => None,
                    },
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Strip `&T` / `&mut T` wrappers to get the underlying owned type.
    fn peel_references(ty: &Type) -> &Type {
        match ty {
            Type::Reference(inner) | Type::MutableReference(inner) => Self::peel_references(inner),
            other => other,
        }
    }
}
