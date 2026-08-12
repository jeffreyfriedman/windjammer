//! Infers [`Type`] from expressions; stdlib and primitive-float method returns.

use crate::analyzer::SignatureRegistry;
use crate::codegen::rust::CodeGenerator;
use crate::parser::{Expression, Literal, Statement, Type};

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {
    /// `TypeName::assoc` return type from the signature registry (Call or MethodCall form).
    /// No method-name heuristics — drives let-binding typing for Borrowed-string coercion.
    fn infer_associated_fn_return_type(&self, type_name: &str, method: &str) -> Option<Type> {
        if !type_name.starts_with(|c: char| c.is_ascii_uppercase()) {
            return None;
        }
        let resolve_self = |ty: &Type| -> Type {
            match ty {
                Type::Custom(n) if n == "Self" || n == type_name => {
                    if type_name == "String" {
                        Type::String
                    } else {
                        Type::Custom(type_name.to_string())
                    }
                }
                Type::Custom(n) if n == "String" => Type::String,
                other => other.clone(),
            }
        };
        let qualified = format!("{type_name}::{method}");
        if let Some(sig) = self.get_signature_with_global(&qualified) {
            if let Some(ret) = &sig.return_type {
                return Some(resolve_self(ret));
            }
            if !sig.has_self_receiver {
                return Some(if type_name == "String" {
                    Type::String
                } else {
                    Type::Custom(type_name.to_string())
                });
            }
        }
        if let Some(ms) = self.lookup_method_signature(type_name, method) {
            if let Some(ret) = &ms.return_type {
                return Some(resolve_self(ret));
            }
        }
        // Cold pass / missing meta: prefer typing the local as the nominal type so later
        // method coercion can resolve Borrowed string formals (WDB-091).
        Some(if type_name == "String" {
            Type::String
        } else {
            Type::Custom(type_name.to_string())
        })
    }

    /// `f32::sin` / `f64::ln` etc. return the same float type as the receiver.
    fn rust_primitive_float_method_return_type(
        receiver: Option<&Type>,
        method: &str,
    ) -> Option<Type> {
        let recv = receiver?;
        if !crate::codegen::rust::stdlib_method_traits::method_preserves_float_receiver(
            method,
            Some(recv),
            &SignatureRegistry::stdlib(),
        ) {
            return None;
        }
        crate::codegen::rust::stdlib_method_traits::float_primitive_name(recv)
            .map(|n| Type::Custom(n.to_string()))
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
                            if let Some(fields) = self
                                .lookup_struct_field_types(struct_name)
                                .or_else(|| self.lookup_struct_field_types(base))
                            {
                                if let Some(field_type) = fields.get(field.as_str()) {
                                    return Some(field_type.clone());
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
                            if let Some(elem_ty) =
                                Self::tuple_field_type_from_var_type(&var_type, field)
                            {
                                return Some(elem_ty);
                            }
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
                        if let Some(elem_ty) =
                            Self::tuple_field_type_from_var_type(&obj_type, field)
                        {
                            return Some(elem_ty);
                        }
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
            } => self.infer_expression_type(operand).map(|t| match t {
                Type::Reference(inner) | Type::MutableReference(inner) => *inner,
                _ => t,
            }),
            // Method calls: look up return type from method_return_types registry
            // and signature registry (for cross-file method resolution)
            Expression::MethodCall { object, method, .. } => {
                // Prefer usize returns from the signature registry (len/capacity/…).
                let obj_ty_early = self.infer_expression_type(object);
                let recv_early = obj_ty_early.as_ref().and_then(Self::type_to_name);
                let usize_recv = match recv_early.as_deref() {
                    Some("str") => Some("String"),
                    other => other,
                };
                if crate::codegen::rust::stdlib_method_traits::method_returns_usize_qualified(
                    method,
                    usize_recv,
                    &self.signature_registry,
                ) || crate::codegen::rust::stdlib_method_traits::method_returns_usize_qualified(
                    method,
                    None,
                    &self.signature_registry,
                ) {
                    return Some(Type::Custom("usize".to_string()));
                }
                // Associated functions: `TypeName::assoc()` (may parse as MethodCall).
                // Signature-driven — same rules as Call(FieldAccess(Type, method)).
                if let Expression::Identifier {
                    name: type_name, ..
                } = &**object
                {
                    if type_name.chars().next().is_some_and(|c| c.is_uppercase()) {
                        if let Some(ret) = self.infer_associated_fn_return_type(type_name, method) {
                            return Some(ret);
                        }
                    }
                }
                // Type-preserving methods (registry return `Self`) keep the receiver type.
                if crate::codegen::rust::stdlib_method_traits::method_is_type_preserving_qualified(
                    method,
                    recv_early.as_deref(),
                    &self.signature_registry,
                ) {
                    return obj_ty_early;
                }
                // Option/Result owned-self adapters: peel wrapper from receiver type.
                if let Some(obj_type) = obj_ty_early.as_ref() {
                    match obj_type {
                        Type::Option(inner)
                            if crate::codegen::rust::stdlib_method_traits::option_owned_self_method(
                                method,
                                &self.signature_registry,
                            ) =>
                        {
                            return Some((**inner).clone());
                        }
                        Type::Result(ok, _)
                            if crate::codegen::rust::stdlib_method_traits::result_owned_self_method(
                                method,
                                &self.signature_registry,
                            ) =>
                        {
                            return Some((**ok).clone());
                        }
                        _ => {}
                    }
                }
                // Iterator methods: return the collection type so
                // extract_iterator_element_type can extract the element type.
                // Driven by registry return metadata (`Iterator` / `Iterator<item>`).
                if let Some(obj_type) = self.infer_expression_type(object) {
                    let receiver = Self::type_to_name(&obj_type);
                    if crate::codegen::rust::stdlib_method_traits::method_returns_iterable_qualified(
                        method,
                        receiver.as_deref(),
                        &self.signature_registry,
                    ) {
                        return Some(obj_type);
                    }
                }
                let obj_ty = self.infer_expression_type(object);
                // Prefer signature-registry return types (stdlib_meta) over hardcoded
                // method-name tables — `String::trim` → `&str`, `HashMap::get` → `Option<&V>`.
                // Must cover Parameterized/Vec/Option receivers via `type_to_name`.
                if let Some(obj_type) = obj_ty.as_ref() {
                    if let Some(ret) = self.registry_method_return_type(obj_type, method) {
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
                // No type context available — don't do bare lookup to avoid
                // picking a different type's method (e.g., "new" → wrong type).
                None
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
                        if let Some(ret) = self.infer_associated_fn_return_type(type_name, field) {
                            return Some(ret);
                        }
                    }
                    // Instance call: Call(FieldAccess(receiver, method), args) — same return type
                    // rules as MethodCall so we do not fall through to unqualified `acos` → f64.
                    let recv_ty = self.infer_expression_type(object);
                    if let Some(ref ot) = recv_ty {
                        if let Some(ret) = self.registry_method_return_type(ot, field) {
                            return Some(ret);
                        }
                    }
                    if let Some(t) = Self::rust_primitive_float_method_return_type(
                        recv_ty.as_ref(),
                        field.as_str(),
                    ) {
                        return Some(t);
                    }
                }
                // Pattern: simple function call → "function_name"
                // Also: collapsed `Type::assoc` path as a single Identifier (`HashMap::new`).
                if let Expression::Identifier { name, .. } = function {
                    if let Some((type_name, method)) = name.rsplit_once("::") {
                        if type_name
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_uppercase())
                        {
                            if let Some(ret) =
                                self.infer_associated_fn_return_type(type_name, method)
                            {
                                return Some(ret);
                            }
                        }
                    }
                    if let Some(sig) = self.get_signature_with_global(name.as_str()) {
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

    /// Look up a method return type from the signature registry with generic substitution.
    ///
    /// Uses [`Self::type_to_name`] so `HashMap<K,V>`, `Vec<T>`, and other parameterized
    /// receivers resolve (`HashMap::get` → `Option<&V>`), not only bare `Custom`/`String`.
    pub(in crate::codegen::rust) fn registry_method_return_type(
        &self,
        receiver: &Type,
        method: &str,
    ) -> Option<Type> {
        let type_name = Self::type_to_name(receiver)?;
        let base = type_name.split('<').next().unwrap_or(type_name.as_str());
        for candidate in [type_name.as_str(), base] {
            let qualified = format!("{candidate}::{method}");
            if let Some(sig) = self.get_signature_with_global(&qualified) {
                if let Some(ret) = &sig.return_type {
                    return Some(Self::substitute_stdlib_generics(ret, receiver));
                }
            }
        }
        None
    }

    /// Tuple destructuring via numeric fields: `i_c.0` when `i_c: (usize, char)`.
    fn tuple_field_type_from_var_type(var_type: &Type, field: &str) -> Option<Type> {
        let tuple = match Self::peel_references(var_type) {
            Type::Tuple(elems) => elems,
            _ => return None,
        };
        let idx = field.parse::<usize>().ok()?;
        tuple.get(idx).cloned()
    }

    /// Substitute stdlib generic placeholders (`T`, `K`, `V`) from the concrete receiver type.
    fn substitute_stdlib_generics(ret: &Type, receiver: &Type) -> Type {
        match ret {
            Type::Custom(n) if n == "T" => Self::peeled_collection_element_type(receiver)
                .cloned()
                .unwrap_or_else(|| ret.clone()),
            Type::Custom(n) if n == "V" => {
                if let Type::Parameterized(_, params) = Self::peel_references(receiver) {
                    if params.len() >= 2 {
                        return params[1].clone();
                    }
                }
                ret.clone()
            }
            Type::Custom(n) if n == "K" => {
                if let Type::Parameterized(_, params) = Self::peel_references(receiver) {
                    if !params.is_empty() {
                        return params[0].clone();
                    }
                }
                ret.clone()
            }
            Type::Option(inner) => {
                Type::Option(Box::new(Self::substitute_stdlib_generics(inner, receiver)))
            }
            Type::Reference(inner) => {
                Type::Reference(Box::new(Self::substitute_stdlib_generics(inner, receiver)))
            }
            Type::MutableReference(inner) => {
                Type::MutableReference(Box::new(Self::substitute_stdlib_generics(inner, receiver)))
            }
            other => other.clone(),
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
