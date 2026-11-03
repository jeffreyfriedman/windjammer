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

    /// `for x in v` over a declared `Vec<T>` / array binds a `T`; any other loop binding is unmodelled.
    pub(super) fn type_loop_element(&mut self, loop_var: ConstraintVar, iterable: &'ast Expression<'ast>) {
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
            if base == BaseType::Inferred {
                continue;
            }
            self.cs.add_because(
                Constraint::TypeIs(v, base.clone()),
                origin("NUM-FIELD-DECLARED", format!("payload {} of `{}` is declared {:?}", i, callee, base), &a.location()),
            );
            emit_numeric_class_constraint(&mut self.cs, v, &base);
        }
    }

    /// `v.get(i)` / `q.remove(i)` / `v.swap(i, j)` on a declared `Vec`, array or `VecDeque`: every
    /// argument is an index (ADR-0028 NUM-USIZE).
    fn std_sequence_index_method(&self, object: &Expression<'ast>, method: &str) -> bool {
        if !matches!(method, "get" | "get_mut" | "remove" | "swap" | "truncate" | "split_off" | "swap_remove") {
            return false;
        }
        match self.receiver_declared_base(object) {
            Some(BaseType::Vec(_) | BaseType::Array(..)) => true,
            Some(BaseType::Custom(n)) => crate::type_classification::type_name_leaf(&n) == "VecDeque",
            _ => false,
        }
    }

    /// The receiver is a declared std collection or string (its `len()` is Rust's `usize`).
    fn receiver_is_std_collection(&self, object: &Expression<'ast>) -> bool {
        match self.receiver_declared_base(object) {
            Some(BaseType::Vec(_) | BaseType::Array(..) | BaseType::HashMap(..) | BaseType::String) => true,
            Some(BaseType::Custom(n)) => matches!(
                crate::type_classification::type_name_leaf(&n),
                "VecDeque" | "HashSet" | "BTreeMap" | "BTreeSet" | "HashMap"
            ),
            _ => false,
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

        // NUM-USIZE (ADR-0028): `v.get(i)`, `q.remove(i)` on a std sequence take an index; a
        // same-named signature of another type (a homonym) must not type the argument.
        let seq_index = self.std_sequence_index_method(object, method);
        if method == "push" && arguments.len() == 1 {
            if let Some(key) = crate::ir::numeric_facts::conversion_key(arguments[0].1) {
                self.element_stores.push((key, arg_vars[0], obj_var));
            }
        }
        if seq_index {
            for (&v, (_, a)) in arg_vars.iter().zip(arguments.iter()) {
                self.index_uses.push(v);
                self.cs.add_because(
                    Constraint::IsNumeric(v),
                    origin("NUM-INDEX", format!("argument of `.{}()` on a sequence is an index", method), &a.location()),
                );
                self.cs.add_because(
                    Constraint::PrefersUsize(v),
                    origin("NUM-USIZE", format!("argument of `.{}()` on a sequence is an index", method), &a.location()),
                );
            }
        }
        if let Some(sig) = sig.as_ref().filter(|_| !seq_index) {
            self.emit_call_site_constraints(sig, &arg_vars);
        }
        // NUM-USIZE (ADR-0028): a length or count (no user signature, or a std one returning
        // `usize`) is an index source: soft `usize`, so a declared width in its class wins and
        // the source is converted at the site.
        let declared_ret = sig
            .as_ref()
            .filter(|_| !self.receiver_is_std_collection(object))
            .and_then(|s| s.return_type.as_ref())
            .map(parser_type_to_base_type);
        // An unresolved receiver may be a user type with its own `len`: only a declared std collection
        // or a signature that returns `usize` makes the call an index source.
        let index_source = arguments.is_empty()
            && is_index_source_method(method)
            && (self.receiver_is_std_collection(object) || declared_ret == Some(BaseType::Usize));
        self.emit_call_result_type_constraints(
            result,
            sig.as_ref().filter(|_| !seq_index && !index_source),
            Some(method.as_str()),
        );
        if arguments.is_empty() && is_index_source_method(method) && !index_source && receiver_type.is_none() {
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
        let fc = generate_constraints(af, None);
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
        let fc = generate_constraints(&analyzed[0], None);
        assert_eq!(fc.index_sources.len(), 1, "{:?}", fc.index_sources);
    }

}
