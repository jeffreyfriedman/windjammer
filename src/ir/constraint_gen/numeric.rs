//! Numeric-width constraints of the AST walker (ADR-0028): NUM-USIZE index sources and uses, std
//! sequence index arguments, receiver classification, tuple-variant payload types.

use super::*;

/// Values the numeric facts must not publish a width for: a negated literal cannot be unsigned, and
/// a loop over a collection whose element type is not declared binds a value of unknown width.
#[derive(Debug, Default, Clone)]
pub struct Unmodelled {
    pub negated_literals: Vec<ConstraintVar>,
    pub loop_bindings: Vec<ConstraintVar>,
    /// `x.len()` on a receiver of unknown type: its width (usize, or a user type's `int`) is unknown.
    pub unknown_lengths: Vec<ConstraintVar>,
    /// A field read whose declared type the walk cannot see (another module's struct, a generic).
    pub unknown_fields: Vec<ConstraintVar>,
    /// Locals bound to a borrow (`let r = &x`, `opt.unwrap_or(&d)`): they hold a reference, so no width is
    /// published for the name (the class itself is still solved through the referent).
    pub borrowed_bindings: Vec<String>,
    /// Unbound names that are module constants with a declared type: modelled (not held as unknown names).
    pub declared_constants: Vec<String>,
    /// Receivers of a numeric method that are literals (`2.pow(10)`, `(-7).abs()`): rustc cannot infer their width
    /// (E0689), so their class is published even at its default (`int` / `float`).
    pub literal_receivers: Vec<ConstraintVar>,
    /// Values used where the walk does not know the expected type: an argument or result of a call with no
    /// signature, an argument at a generic or alias-typed formal, a struct literal field of an unknown struct, a
    /// value passed to an unknown method. Their classes (and the parts of those classes) are not published.
    pub opaque: Vec<ConstraintVar>,
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// A field read has the field's declared type; a field whose type the walk cannot see is unmodelled.
    pub(super) fn walk_field_access(&mut self, object: &'ast Expression<'ast>, expr: &Expression<'ast>) -> ConstraintVar {
        self.emit_read_field_ref(object, expr);
        let obj_var = self.walk_expression(object);
        let field_var = self.cs.fresh_var();
        let _ = obj_var;
        let fields = self.struct_field_types.unwrap_or_else(|| empty_struct_field_types());
        match crate::ir::expr_static_type::declared_type_of(expr, &self.analyzed.decl, fields)
            .or_else(|| self.local_struct_field_type(object, expr, fields))
            .map(|ty| parser_type_to_base_type(&ty))
            .filter(|b| *b != BaseType::Inferred)
        {
            Some(base) => {
                self.cs.add_because(
                    Constraint::TypeIs(field_var, base.clone()),
                    origin("NUM-FIELD-DECLARED", "a field read has the field's declared type", &expr.location()),
                );
                emit_numeric_class_constraint(&mut self.cs, field_var, &base);
            }
            None => self.numeric_unmodelled.unknown_fields.push(field_var),
        }
        field_var
    }

    /// `b.n` where `b` is a local bound to a struct literal (or an annotated struct local).
    fn local_struct_field_type(
        &self,
        object: &Expression<'ast>,
        expr: &Expression<'ast>,
        fields: &HashMap<String, HashMap<String, Type>>,
    ) -> Option<Type> {
        let (Expression::Identifier { name, .. }, Expression::FieldAccess { field, .. }) = (object, expr) else { return None };
        let Some(BaseType::Custom(s)) = self.local_decl_types.get(name) else { return None };
        crate::type_inference::struct_field_registry::lookup_struct_field_map(fields, s, &HashMap::new(), &HashMap::new())
            .and_then(|f| f.get(field.as_str()))
            .cloned()
    }

    /// `for x in v` binds an element of `v` (a declared `Vec<T>` / array binds a `T`). A loop over a call result
    /// whose element the surface did not type is unmodelled.
    pub(super) fn type_loop_element(&mut self, loop_var: ConstraintVar, iterable_var: ConstraintVar, iterable: &'ast Expression<'ast>) {
        self.cs.add_because(Constraint::HasPart(iterable_var, crate::ir::constraints::Part::Elem, loop_var), tagged("NUM-ELEMENT"));
        let structural = match iterable {
            Expression::MethodCall { .. } | Expression::Call { .. } => self.surface_typed.contains(&iterable_var),
            _ => true,
        };
        let source = match iterable {
            Expression::Unary { operand, .. } => *operand,
            Expression::MethodCall { object, method, arguments, .. }
                if arguments.is_empty() && matches!(method.as_str(), "iter" | "into_iter" | "iter_mut") =>
            {
                *object
            }
            other => other,
        };
        let elem = match self.receiver_declared_base(source) {
            Some(BaseType::Vec(e) | BaseType::Array(e, _)) => Some(*e),
            _ => None,
        };
        match elem.filter(|e| *e != BaseType::Inferred && !matches!(e, BaseType::TypeVar(_))) {
            Some(e) => {
                self.cs.add_because(
                    Constraint::TypeIs(loop_var, e.clone()),
                    origin("NUM-ELEMENT", "a loop over a declared collection binds its element type", &iterable.location()),
                );
                emit_numeric_class_constraint(&mut self.cs, loop_var, &e);
            }
            None if structural => {}
            None => self.numeric_unmodelled.loop_bindings.push(loop_var),
        }
    }

    /// NUM-FIELD-DECLARED: the arguments of a tuple-variant constructor (`Val::Int(-1)`) have the
    /// variant's declared payload types.
    pub(super) fn emit_variant_payload_types(
        &mut self,
        callee: &str,
        arg_vars: &[ConstraintVar],
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
    ) {
        let Some(payload) = self.struct_field_types.and_then(|f| f.get(callee)) else { return };
        for (i, (&v, (_, a))) in arg_vars.iter().zip(arguments.iter()).enumerate() {
            let Some(ty) = payload.get(&i.to_string()) else { continue };
            let base = parser_type_to_base_type(ty);
            // A named payload type (a struct, or the enum's own generic `T`) fixes no width.
            if base == BaseType::Inferred || crate::ir::constraints::contains_named(&base) {
                continue;
            }
            self.cs.add_because(
                Constraint::TypeIs(v, base.clone()),
                origin("NUM-FIELD-DECLARED", format!("payload {} of `{}` is declared {:?}", i, callee, base), &a.location()),
            );
            emit_numeric_class_constraint(&mut self.cs, v, &base);
        }
    }

    /// Declared type of a receiver: a parameter, an annotated local (or a local bound to one), a field.
    pub(super) fn receiver_declared_base(&self, object: &Expression<'ast>) -> Option<BaseType> {
        match object {
            Expression::Identifier { name, .. } => self.local_decl_types.get(name).cloned().or_else(|| {
                self.analyzed
                    .decl
                    .parameters
                    .iter()
                    .find(|p| &p.name == name)
                    .map(|p| parser_type_to_base_type(&p.type_))
            }),
            // `let b = Box { .. }`: the struct.
            Expression::StructLiteral { name, .. } => Some(BaseType::Custom(name.clone())),
            // `let parts = pieces(text)`: the callee's declared return type.
            Expression::Call { function, arguments, .. } => {
                let name = Self::extract_callee_name(function)?;
                let sig = self.resolve_free_callee_signature(&name, arguments.len())?;
                sig.return_type.as_ref().map(parser_type_to_base_type).filter(|b| *b != BaseType::Inferred)
            }
            // `let parts = strings.split(text, "-")`: a module function's declared return type.
            Expression::MethodCall { object, method, arguments, .. } => {
                let Expression::Identifier { name, .. } = &**object else { return None };
                if self.param_vars.contains_key(name) || self.scopes.iter().any(|s| s.contains_key(name)) {
                    return None;
                }
                let sig = self.resolve_callee_signature(&format!("{}::{}", name, method), arguments.len())?;
                sig.return_type.as_ref().map(parser_type_to_base_type).filter(|b| *b != BaseType::Inferred)
            }
            other => {
                let fields = self.struct_field_types.unwrap_or_else(|| empty_struct_field_types());
                crate::ir::expr_static_type::declared_type_of(other, &self.analyzed.decl, fields)
                    .map(|t| parser_type_to_base_type(&t))
            }
        }
    }

    /// A method call: receiver ownership, call-site and result types, NUM-USIZE index sources and uses.
    pub(super) fn walk_method_call(
        &mut self,
        expr: &Expression<'ast>,
        object: &'ast Expression<'ast>,
        method: &String,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
    ) -> ConstraintVar {
        let receiver_type = self.receiver_type_name_for_method(object);
        if let Expression::MethodCall { type_args, .. } = expr {
            self.prepare_closure_param_types(object, method, arguments, type_args.as_deref().unwrap_or(&[]));
        }
        for (i, (_, arg)) in arguments.iter().enumerate() {
            let arg_in_payload = self.method_arg_is_owned_payload_store(
                method,
                receiver_type.as_deref(),
                i,
                arguments.len(),
            );
            self.emit_owned_for_stored_param_identifiers_inner(arg, arg_in_payload);
        }
        let obj_var = self.walk_expression(object);
        self.emit_consuming_method_on_generic_param(object, method, obj_var);
        let mut arg_vars = Vec::new();
        for (_label, arg_expr) in arguments {
            arg_vars.push(self.walk_expression(arg_expr));
        }

        // Signature-driven receiver ownership: `Type::method` self mode wins.
        // Unresolved methods use qualified stdlib consensus (receiver type + registry).
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
            .resolve_callee_signature(&qualified, arguments.len())
            .or_else(|| {
                // Runtime-std `json::to_string` must not fall back to the first
                // `*::to_string` method-index hit (often MutBorrowed self).
                if callee_path_is_runtime_std(
                    &qualified,
                ) {
                    None
                } else {
                    self.resolve_callee_signature(method, arguments.len())
                }
            })
            .cloned();

        let mut_self = sig.as_ref().is_some_and(|s| {
            s.has_self_receiver
                && s.param_ownership
                    .first()
                    .is_some_and(|m| matches!(m, OwnershipMode::MutBorrowed))
        }) || (sig.is_none()
            && self.registry.is_some_and(|reg| {
                crate::analyzer::stdlib_method_traits::method_mutates_receiver_qualified(
                    method,
                    receiver_type.as_deref(),
                    reg,
                )
            }));
        if mut_self {
            self.emit_mutref_on_receiver_root(object);
        }

        let result = self.cs.fresh_var();

        // Option C stage C1: a method of a Rust core type is typed by `std/rust_core.wji` (index slots,
        // lengths, element and payload types); a same-named registry signature is a homonym and is not used.
        let surface = self.emit_numeric_method_constraints(object, obj_var, method, arguments, &arg_vars, result)
            || self.emit_core_surface_constraints(expr, object, obj_var, method, arguments, &arg_vars, result);
        if let Some(sig) = sig.as_ref().filter(|_| !surface) {
            self.emit_call_site_constraints(sig, &arg_vars, arguments);
        } else if !surface {
            // A literal receiver keeps its default: rustc cannot infer a literal's width for a method call (E0689).
            let receiver = (!self.numeric_unmodelled.literal_receivers.contains(&obj_var)).then_some(obj_var);
            self.numeric_unmodelled.opaque.extend(arg_vars.iter().copied().chain(receiver).chain([result]));
        }
        // NUM-USIZE (ADR-0028): a length or count whose user signature returns `usize` is an index source
        // (core-type lengths are typed by the surface above). An unresolved receiver may be a user type
        // with its own `len`: its width is unknown.
        let declared_ret = sig.as_ref().filter(|_| !surface).and_then(|s| s.return_type.as_ref()).map(parser_type_to_base_type);
        let index_source = !surface && arguments.is_empty() && is_index_source_method(method) && declared_ret == Some(BaseType::Usize);
        self.emit_call_result_type_constraints(result, sig.as_ref().filter(|_| !surface && !index_source), Some(method.as_str()));
        if arguments.is_empty() && is_index_source_method(method) && !surface && !index_source && receiver_type.is_none() {
            self.numeric_unmodelled.unknown_lengths.push(result);
        }
        if index_source {
            if let Some(l) = expr.location() {
                self.index_sources.push(((l.line, l.column), result));
            }
            self.cs.add_because(
                Constraint::PrefersUsize(result),
                origin("NUM-USIZE", format!("`.{}()` is a length: an index class unless a width is declared", method), &expr.location()),
            );
        }

        let callee_effects = crate::ir::effects::effects_for_runtime_callee(&qualified);
        if !callee_effects.is_empty() {
            self.cs.add_because(
                Constraint::HasEffects(obj_var, EffectSet::from_iter(callee_effects)),
                tagged("EFF-CALL"),
            );
        }

        self.emit_taint_for_call(&qualified, &self.analyzed.decl.name.clone());

        self.cs.add_because(Constraint::EffectsUnion(result, vec![obj_var]), tagged("EFF-CALL"));
        result
    }

}

/// An integer literal, or arithmetic / negation over integer literals (what constant folding folds).
pub(super) fn is_int_literal_tree(e: &Expression<'_>) -> bool {
    match e {
        Expression::Literal { value: Literal::Int(_), .. } => true,
        Expression::Unary { op: crate::parser::ast::operators::UnaryOp::Neg, operand, .. } => is_int_literal_tree(operand),
        Expression::Binary { left, op, right, .. } => {
            is_arithmetic_op(op) && is_int_literal_tree(left) && is_int_literal_tree(right)
        }
        _ => false,
    }
}

/// Zero-argument methods whose Rust result is `usize` on std receivers (ADR-0028 index sources).
pub(super) fn is_index_source_method(method: &str) -> bool {
    matches!(method, "len" | "count" | "capacity")
}


/// Provenance rule for a literal's type constraint.
pub(super) fn literal_rule(value: &Literal) -> &'static str {
    match value {
        Literal::Int(_) => "NUM-DEFAULT-INT",
        Literal::Float(_) => "NUM-DEFAULT-FLOAT",
        Literal::IntSuffixed(..) => "NUM-LITERAL-SUFFIX",
        _ => "NUM-LITERAL",
    }
}

/// Map a literal value to its base type.
pub(super) fn literal_to_base_type(lit: &Literal) -> BaseType {
    match lit {
        // Unsuffixed integers in patterns: `int` (ADR-0015). Expressions use a soft class instead.
        Literal::Int(_) => BaseType::I64,
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
            "usize" => BaseType::Usize,
            "isize" => BaseType::Isize,
            _ => BaseType::I32,
        },
        Literal::Float(_) => BaseType::F64,
        Literal::String(_) => BaseType::String,
        Literal::Char(_) => BaseType::Char,
        Literal::Bool(_) => BaseType::Bool,
    }
}

/// Emit numeric class constraints based on base type.
pub(super) fn emit_numeric_class_constraint(cs: &mut ConstraintSet, var: ConstraintVar, base: &BaseType) {
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
        | BaseType::U128
        | BaseType::Usize
        | BaseType::Isize => {
            cs.add_because(Constraint::IsInteger(var), tagged("NUM-CLASS"));
            cs.add_because(Constraint::IsNumeric(var), tagged("NUM-CLASS"));
        }
        BaseType::F32 | BaseType::F64 => {
            cs.add_because(Constraint::IsFloat(var), tagged("NUM-CLASS"));
            cs.add_because(Constraint::IsNumeric(var), tagged("NUM-CLASS"));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze_source(source: &str) -> Vec<crate::analyzer::AnalyzedFunction<'static>> {
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let parser = Box::leak(Box::new(crate::parser::Parser::new(tokens)));
        let program = Box::leak(Box::new(parser.parse().expect("parse")));
        let mut analyzer = crate::analyzer::Analyzer::new();
        let (analyzed, _, _) = analyzer.analyze_program(program).expect("analyze");
        analyzed
    }

    fn solved_local(source: &str, fn_name: &str, local: &str) -> Option<BaseType> {
        let analyzed = analyze_source(source);
        let af = analyzed.iter().find(|f| f.decl.name == fn_name).unwrap();
        let fc = generate_constraints_typed(af, None, None, Some(&crate::typing::core_surface::CoreSurface::parse()));
        let r = crate::ir::solver::Solver::new(&fc.constraints).solve(&fc.constraints);
        let v = fc.var_map.locals[local];
        r.types[v.0 as usize].clone()
    }

    /// ADR-0028: a counter bounded by `len()` and used as an index is `usize`; the same counter
    /// stored into a declared `int` is `int`.
    #[test]
    fn counter_bounded_by_len_is_usize_unless_it_meets_an_int() {
        let usize_src = "pub fn f(v: Vec<string>) -> bool {\n    let mut i = 0\n    while i < v.len() {\n        i = i + 1\n    }\n    true\n}\n";
        assert_eq!(solved_local(usize_src, "f", "i"), Some(BaseType::Usize));
        let int_src = "pub fn f(v: Vec<string>) -> bool {\n    let mut i = 0\n    while i < v.len() {\n        let k: int = i\n        i = i + 1\n    }\n    true\n}\n";
        assert_eq!(solved_local(int_src, "f", "i"), Some(BaseType::I64));
        let index_src = "pub fn f(v: Vec<string>) -> string {\n    let j = 2\n    v[j]\n}\n";
        assert_eq!(solved_local(index_src, "f", "j"), Some(BaseType::Usize));
        let plain_src = "pub fn f() -> bool {\n    let mut i = 0\n    while i < 10 {\n        i = i + 1\n    }\n    true\n}\n";
        assert_eq!(solved_local(plain_src, "f", "i"), Some(BaseType::I64));
    }

    #[test]
    fn declared_int_param_fixes_the_class_of_a_len_sum() {
        let src = "pub fn f(v: Vec<string>, n: int) -> int {\n    let mut t = v.len() + n\n    t = t + 1\n    t\n}\n";
        let analyzed = analyze_source(src);
        let fc = generate_constraints(&analyzed[0], None);
        let r = crate::ir::solver::Solver::new(&fc.constraints).solve(&fc.constraints);
        let n = fc.var_map.params["n"];
        let t = fc.var_map.locals["t"];
        assert_eq!(r.types[t.0 as usize], Some(BaseType::I64), "n={:?} diags={:?} cs={:?}", r.types[n.0 as usize], r.diagnostics, fc.constraints.iter().collect::<Vec<_>>());
    }

    #[test]
    fn len_call_is_recorded_as_an_index_source() {
        let analyzed = analyze_source("pub fn f(v: Vec<string>) -> bool {\n    let mut i = 0\n    while i < v.len() {\n        i = i + 1\n    }\n    true\n}\n");
        let fc = generate_constraints_typed(&analyzed[0], None, None, Some(&crate::typing::core_surface::CoreSurface::parse()));
        assert_eq!(fc.index_sources.len(), 1, "{:?}", fc.index_sources);
    }

}

/// `&x`, `&mut x`, or `opt.unwrap_or(&d)`: an initializer whose value is a reference.
pub(super) fn is_borrow_initializer(e: &Expression<'_>) -> bool {
    use crate::parser::ast::operators::UnaryOp;
    match e {
        Expression::Unary { op: UnaryOp::Ref | UnaryOp::MutRef, .. } => true,
        Expression::MethodCall { method, arguments, .. } if method == "unwrap_or" => {
            matches!(arguments.first(), Some((_, Expression::Unary { op: UnaryOp::Ref, .. })))
        }
        _ => false,
    }
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// `Ok(x)` / `Err(x)` / `Some(x)` returned from a function with a declared `Result` / `Option`
    /// return type: the local `x` has the declared payload type (`Ok(out)` under `-> Result<Vec<int>, _>`
    /// types `out` as `Vec<int>`).
    pub(super) fn type_wrapped_return_payload(&mut self, value: &Expression<'ast>) {
        let Some(rt) = self.analyzed.decl.return_type.as_ref() else { return };
        let Expression::Call { function, arguments, .. } = value else { return };
        let (Expression::Identifier { name: ctor, .. }, [(_, Expression::Identifier { name, .. })]) = (&**function, arguments.as_slice()) else {
            return;
        };
        let payload = match (parser_type_to_base_type(rt), ctor.as_str()) {
            (BaseType::Result(t, _), "Ok") | (BaseType::Option(t), "Some") => *t,
            (BaseType::Result(_, e), "Err") => *e,
            _ => return,
        };
        if payload == BaseType::Inferred || matches!(payload, BaseType::TypeVar(_)) {
            return;
        }
        let var = self.resolve_var(name);
        self.cs.add_because(Constraint::TypeIs(var, payload.clone()), tagged("NUM-DECLARED"));
        emit_numeric_class_constraint(&mut self.cs, var, &payload);
    }
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// The variable a name is bound to here, without creating one.
    fn bound_var(&self, name: &str) -> Option<ConstraintVar> {
        self.scopes.iter().rev().find_map(|s| s.get(name)).or_else(|| self.param_vars.get(name)).copied()
    }

    /// The static type of an expression, from declarations and the typed core surface (option C, stage C1).
    /// `None` when a part of it is not declared: callers then keep their previous behaviour.
    pub(super) fn static_type(&self, expr: &Expression<'ast>) -> Option<Type> {
        match expr {
            Expression::Identifier { name, .. } => {
                let var = self.bound_var(name);
                if let Some(t) = var.and_then(|v| self.var_static_types.get(&v)) {
                    return Some(t.clone());
                }
                if var.is_some() && !self.param_vars.contains_key(name.as_str()) {
                    return None;
                }
                self.analyzed.decl.parameters.iter().find(|p| &p.name == name).map(|p| p.type_.clone())
            }
            Expression::Literal { value: Literal::String(_), .. } => Some(Type::String),
            Expression::Unary { op: crate::parser::ast::operators::UnaryOp::Ref, operand, .. } => {
                self.static_type(operand).map(|t| Type::Reference(Box::new(t)))
            }
            Expression::StructLiteral { name, .. } => Some(Type::Custom(name.clone())),
            Expression::MethodCall { object, method, type_args, arguments, .. } => {
                // `strings.split(text, "-")`: a module function's declared return type.
                if let Expression::Identifier { name, .. } = &**object {
                    if self.bound_var(name).is_none() && !self.analyzed.decl.parameters.iter().any(|p| &p.name == name) {
                        let sig = self.resolve_callee_signature(&format!("{}::{}", name, method), arguments.len())?;
                        return sig.return_type.clone();
                    }
                }
                let receiver = self.static_type(object)?;
                let sig = self.core_surface?.method_signature(&receiver, method, type_args.as_deref().unwrap_or(&[]))?;
                sig.ret.filter(|t| !contains_generic(t))
            }
            Expression::Call { function, arguments, .. } => {
                let name = Self::extract_callee_name(function)?;
                self.resolve_free_callee_signature(&name, arguments.len())?.return_type.clone()
            }
            other => {
                let fields = self.struct_field_types.unwrap_or_else(|| empty_struct_field_types());
                crate::ir::expr_static_type::declared_type_of(other, &self.analyzed.decl, fields)
            }
        }
    }
}

/// The integer or float width a (reference to a) numeric type names; `None` for every other type.
pub(super) fn numeric_base_of(t: &Type) -> Option<BaseType> {
    let b = crate::ir::node::parser_type_to_base_type(t);
    matches!(
        b,
        BaseType::I8 | BaseType::I16 | BaseType::I32 | BaseType::I64 | BaseType::I128 | BaseType::U8 | BaseType::U16
            | BaseType::U32 | BaseType::U64 | BaseType::U128 | BaseType::Usize | BaseType::Isize | BaseType::F32 | BaseType::F64
    )
    .then_some(b)
}

fn contains_generic(t: &Type) -> bool {
    match t {
        Type::Generic(_) => true,
        Type::Infer => false,
        Type::Reference(x) | Type::MutableReference(x) | Type::Option(x) | Type::Vec(x) | Type::Array(x, _) => contains_generic(x),
        Type::Result(a, b) => contains_generic(a) || contains_generic(b),
        Type::Parameterized(_, xs) | Type::Tuple(xs) => xs.iter().any(contains_generic),
        _ => false,
    }
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// A module `const` / `static` has its declared type (`const SIZE: i32`): the class is modelled.
    pub(super) fn type_module_constant(&mut self, name: &str, v: ConstraintVar) {
        let Some(ty) = self.analyzed.module_const_types.get(name) else { return };
        let base = parser_type_to_base_type(ty);
        if base != BaseType::Inferred {
            self.cs.add_because(Constraint::TypeIs(v, base.clone()), tagged("NUM-DECLARED"));
            emit_numeric_class_constraint(&mut self.cs, v, &base);
            self.numeric_unmodelled.declared_constants.push(name.to_string());
        }
    }

    /// The float class (like the integer class): an unsuffixed float literal takes the width its uses require
    /// (`f32` field, parameter, payload, operand), else `float` (f64).
    pub(super) fn walk_float_literal(&mut self, var: ConstraintVar, location: &crate::parser::SourceLocation) -> ConstraintVar {
        self.cs.add_because(
            Constraint::IsFloat(var),
            origin("NUM-DEFAULT-FLOAT", "unsuffixed float literal: width from its uses, else float (f64)", location),
        );
        if let Some(l) = location {
            self.literal_vars.push(((l.line, l.column), var));
        }
        var
    }

    /// A `let` binding's static type (for the typed core surface) and whether it holds a borrow.
    pub(super) fn record_let_binding_types(
        &mut self,
        pattern: &Pattern<'ast>,
        declared: Option<&Type>,
        value: &Expression<'ast>,
        bound_var: ConstraintVar,
        static_ty: Option<Type>,
    ) {
        let (Pattern::Identifier(n) | Pattern::MutBinding(n)) = pattern else { return };
        if let Some(t) = static_ty {
            self.var_static_types.insert(bound_var, t);
        }
        if declared.is_none() && is_borrow_initializer(value) {
            self.numeric_unmodelled.borrowed_bindings.push(n.clone());
        }
    }
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// A method of the integer or float numeric surface (`Integer<T>` / `Float<T>` in `std/rust_core.wji`) on a
    /// receiver that is a number: a declared numeric type or a numeric literal (`10.min(3)`, `2.pow(10)`). An
    /// argument or result of type `T` joins the receiver's class; other numeric types are exact (`pow` takes a
    /// `u32`). Returns whether the call was typed this way.
    pub(super) fn emit_numeric_method_constraints(
        &mut self,
        object: &'ast Expression<'ast>,
        obj_var: ConstraintVar,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        arg_vars: &[ConstraintVar],
        result: ConstraintVar,
    ) -> bool {
        let float = match self.static_type(object).as_ref().map(crate::ir::node::parser_type_to_base_type) {
            Some(BaseType::F32 | BaseType::F64) => true,
            Some(b) if crate::ir::solver::is_integer_type(&b) => false,
            Some(_) => return false,
            None if is_int_literal_tree(object) => false,
            None if is_float_literal_tree(object) => true,
            // A receiver of unknown static type (`(a - b).abs()`): a method the integer and float surfaces declare
            // alike relates its `T` arguments and result to the receiver whatever the receiver's number kind is.
            None => return self.emit_numeric_method_either_kind(obj_var, method, arguments, arg_vars, result),
        };
        if is_int_literal_tree(object) || is_float_literal_tree(object) {
            self.numeric_unmodelled.literal_receivers.push(obj_var);
        }
        let Some((params, ret)) = self.core_surface.and_then(|s| s.numeric_method(float, method)) else { return false };
        if params.len() != arguments.len() {
            return false;
        }
        let same = |t: &Type| crate::typing::core_surface::is_receiver_type(t);
        for ((param, &v), (_, a)) in params.iter().zip(arg_vars).zip(arguments) {
            if same(param) {
                self.cs.add_because(Constraint::TypeEquals(v, obj_var), origin("NUM-STD-SIG", format!("`.{}()` takes the receiver's type", method), &a.location()));
            } else if let Some(base) = numeric_base_of(param) {
                self.cs.add_because(Constraint::TypeIs(v, base.clone()), origin("NUM-STD-SIG", format!("`.{}()` takes `{}` here", method, crate::type_display::format_wj_type(param)), &a.location()));
                emit_numeric_class_constraint(&mut self.cs, v, &base);
            }
        }
        match ret.as_ref() {
            Some(t) if same(t) => self.cs.add_because(Constraint::TypeEquals(result, obj_var), tagged("NUM-STD-SIG")),
            Some(t) => {
                if let Some(base) = numeric_base_of(t) {
                    self.cs.add_because(Constraint::TypeIs(result, base.clone()), tagged("NUM-STD-SIG"));
                    emit_numeric_class_constraint(&mut self.cs, result, &base);
                }
            }
            None => {}
        }
        true
    }
}

/// A float literal, possibly negated.
fn is_float_literal_tree(e: &Expression<'_>) -> bool {
    match e {
        Expression::Literal { value: Literal::Float(_), .. } => true,
        Expression::Unary { op: crate::parser::ast::operators::UnaryOp::Neg, operand, .. } => is_float_literal_tree(operand),
        _ => false,
    }
}

/// A type with a part no receiver or turbofish fixed (a generic, or an inferred `_`).
pub(super) fn contains_unfixed(t: &Type) -> bool {
    match t {
        Type::Infer => true,
        Type::Reference(x) | Type::MutableReference(x) | Type::Option(x) | Type::Vec(x) | Type::Array(x, _) => contains_unfixed(x),
        Type::Result(a, b) => contains_unfixed(a) || contains_unfixed(b),
        Type::Parameterized(_, xs) | Type::Tuple(xs) => xs.iter().any(contains_unfixed),
        other => contains_generic(other),
    }
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// `receiver.method(..)` where the receiver's static type is unknown but numeric methods of both kinds declare the
    /// method with the same shape over `T` (`abs`, `min`, `max`, `clamp`, ...): every `T` argument and a `T` result are
    /// the receiver's class. A method that differs between the kinds (or takes a fixed numeric type) is not typed here.
    fn emit_numeric_method_either_kind(
        &mut self,
        obj_var: ConstraintVar,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        arg_vars: &[ConstraintVar],
        result: ConstraintVar,
    ) -> bool {
        let Some(surface) = self.core_surface else { return false };
        let (Some((ip, ir)), Some((fp, fr))) = (surface.numeric_method(false, method), surface.numeric_method(true, method)) else {
            return false;
        };
        let same = |t: &Type| crate::typing::core_surface::is_receiver_type(t);
        let all_receiver = |ps: &[Type]| ps.iter().all(same);
        let ret_receiver = |r: &Option<Type>| r.as_ref().is_some_and(same);
        if ip.len() != arguments.len() || fp.len() != ip.len() || !all_receiver(&ip) || !all_receiver(&fp) || !ret_receiver(&ir) || !ret_receiver(&fr) {
            return false;
        }
        for (&v, (_, a)) in arg_vars.iter().zip(arguments) {
            self.cs.add_because(Constraint::TypeEquals(v, obj_var), origin("NUM-STD-SIG", format!("`.{}()` takes the receiver's type", method), &a.location()));
        }
        self.cs.add_because(Constraint::TypeEquals(result, obj_var), tagged("NUM-STD-SIG"));
        true
    }
}
