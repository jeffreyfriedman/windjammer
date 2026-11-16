//! Structured values in the numeric solver (option C, stage C2): collection and tuple literals, indexing,
//! loops, closures and patterns relate a container to its parts with `HasPart`, so a width declared on one
//! side (a `Vec<i32>` parameter, a `fn(int) -> int` formal, an `Option<u8>` field) reaches the literals and
//! bindings on the other side, and a binding the walk can name is no longer an unknown name.

use super::*;
use crate::ir::constraints::Part;
use crate::parser::ast::core::{EnumPatternBinding, MatchArm};
use crate::parser::ast::literals::MacroDelimiter;

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    fn part(&mut self, container: ConstraintVar, part: Part, var: ConstraintVar, rule: &'static str) {
        self.cs.add_because(Constraint::HasPart(container, part, var), tagged(rule));
    }

    /// `[a, b, c]` / `vec![a, b]`: every element is the collection's element.
    pub(super) fn walk_collection_literal(&mut self, elements: &[&'ast Expression<'ast>]) -> ConstraintVar {
        let result = self.cs.fresh_var();
        for e in elements {
            let v = self.walk_expression(e);
            self.part(result, Part::Elem, v, "NUM-ELEMENT");
        }
        result
    }

    /// A macro: a bracketed one is a collection literal (`vec![a, b]`, `[x; n]` repeats `x` `n` times, `n` is an
    /// index use); the equality assertions (`assert_eq!(a, b)`, intrinsics) compare two values of one type, like
    /// `a == b`; any other macro's arguments are walked for their own constraints.
    pub(super) fn walk_macro(
        &mut self,
        name: &str,
        args: &[&'ast Expression<'ast>],
        delimiter: &MacroDelimiter,
        is_repeat: bool,
    ) -> ConstraintVar {
        if !matches!(delimiter, MacroDelimiter::Brackets) && args.len() >= 2 && crate::intrinsics::Intrinsic::is_equality_assertion(name) {
            let vars: Vec<ConstraintVar> = args.iter().map(|a| self.walk_expression(a)).collect();
            self.relate_equality_assertion(&vars);
            return self.cs.fresh_var();
        }
        if is_repeat && args.len() == 2 {
            let result = self.walk_collection_literal(&args[..1]);
            let n = self.walk_expression(args[1]);
            self.cs.add_because(Constraint::IsNumeric(n), tagged("NUM-INDEX"));
            self.index_uses.push(n);
            self.cs.add_because(Constraint::PrefersUsize(n), origin("NUM-USIZE", "a repeat count is an index", &args[1].location()));
            return result;
        }
        if matches!(delimiter, MacroDelimiter::Brackets) {
            return self.walk_collection_literal(args);
        }
        for a in args {
            self.walk_expression(a);
        }
        self.cs.fresh_var()
    }

    /// `(a, b)`: field `i` is the tuple's part `i`.
    pub(super) fn tuple_parts(&mut self, result: ConstraintVar, field_vars: &[ConstraintVar]) {
        for (i, &v) in field_vars.iter().enumerate() {
            self.part(result, Part::Field(i as u8), v, "NUM-TUPLE");
        }
    }

    /// `v[i]` is an element of `v`; `v[a..b]` has `v`'s elements.
    pub(super) fn index_parts(&mut self, object: ConstraintVar, index: &Expression<'ast>, result: ConstraintVar) {
        if matches!(index, Expression::Range { .. }) {
            let e = self.cs.fresh_var();
            self.part(object, Part::Elem, e, "NUM-ELEMENT");
            self.part(result, Part::Elem, e, "NUM-ELEMENT");
        } else {
            self.part(object, Part::Elem, result, "NUM-ELEMENT");
        }
    }

    /// `|a, (b, c)| body`: each parameter is bound in the closure's scope and is the closure's parameter part; the
    /// body is its result part. A parameter with a written type (`|x: i32|`) is not modelled here.
    pub(super) fn walk_closure(&mut self, closure: &Expression<'ast>, parameters: &[String], body: &'ast Expression<'ast>) -> ConstraintVar {
        let result = self.cs.fresh_var();
        let id = crate::ir::numeric_defaults::node_id(closure);
        self.scopes.push(HashMap::new());
        for (i, p) in parameters.iter().enumerate() {
            let v = self.bind_closure_parameter(p);
            // A typed std method fixed this parameter's type (`text.split(" ").map(|s| s.len())`: `s` is text).
            if let Some(t) = self.closure_param_types.remove(&(id, i)) {
                self.var_static_types.insert(v, t);
            }
            self.part(result, Part::Param(i as u8), v, "NUM-CLOSURE");
        }
        let body_var = self.walk_expression(body);
        self.part(result, Part::Ret, body_var, "NUM-CLOSURE");
        self.scopes.pop();
        result
    }

    fn bind_closure_parameter(&mut self, p: &str) -> ConstraintVar {
        if let Some(inner) = p.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
            let var = self.cs.fresh_var();
            for (i, name) in inner.split(',').map(str::trim).enumerate() {
                let v = self.bind_name(name);
                self.part(var, Part::Field(i as u8), v, "NUM-TUPLE");
            }
            return var;
        }
        match p.split_once(':') {
            Some((name, _written_type)) => {
                let v = self.bind_name(name.trim());
                self.numeric_unmodelled.unknown_fields.push(v);
                v
            }
            None => self.bind_name(p.trim()),
        }
    }

    /// `match value { pattern => body }`: each arm binds its pattern (in its own scope) against the value.
    /// Returns the arm bodies' variables (the match's value positions).
    pub(super) fn walk_match_arms(&mut self, value: ConstraintVar, scrutinee: &Expression<'ast>, arms: &[MatchArm<'ast>]) -> Vec<ConstraintVar> {
        let mut out = Vec::new();
        let scrutinee_ty = self.static_type(scrutinee);
        for arm in arms {
            self.scopes.push(HashMap::new());
            let pv = self.bind_pattern(&arm.pattern);
            if let Some(t) = &scrutinee_ty {
                self.record_pattern_static_types(&arm.pattern, t);
            }
            self.cs.add_because(Constraint::TypeEquals(pv, value), tagged("TY-UNIFY"));
            if let Some(guard) = arm.guard {
                let guard_var = self.walk_expression(guard);
                self.cs.add_because(Constraint::TypeIs(guard_var, BaseType::Bool), tagged("TY-BOOL"));
            }
            out.push(self.walk_expression(arm.body));
            self.scopes.pop();
        }
        out
    }

    /// The static types of the names a pattern binds against a value of declared type `ty` (`Some(r)` against an
    /// `Option<Rect>` binds a `Rect`), so a field read on a binding has the field's declared type.
    fn record_pattern_static_types(&mut self, pattern: &Pattern<'ast>, ty: &Type) {
        let ty = match ty {
            Type::Reference(t) | Type::MutableReference(t) => &**t,
            other => other,
        };
        match (pattern, ty) {
            (Pattern::Identifier(n) | Pattern::MutBinding(n), _) => {
                if let Some(v) = self.scopes.last().and_then(|s| s.get(n)).copied() {
                    if !super::numeric::contains_unfixed(ty) {
                        self.var_static_types.insert(v, ty.clone());
                    }
                }
            }
            (Pattern::EnumVariant(name, binding), Type::Option(_) | Type::Result(_, _)) => {
                let payload = match (prelude_payload(name), ty) {
                    (Some(Part::Payload), Type::Option(t) | Type::Result(t, _)) => &**t,
                    (Some(Part::ErrPayload), Type::Result(_, e)) => &**e,
                    _ => return,
                };
                match binding {
                    EnumPatternBinding::Single(n) => self.record_pattern_static_types(&Pattern::Identifier(n.clone()), payload),
                    EnumPatternBinding::Tuple(pats) if pats.len() == 1 => self.record_pattern_static_types(&pats[0], payload),
                    _ => {}
                }
            }
            (Pattern::Tuple(pats), Type::Tuple(ts)) if pats.len() == ts.len() => {
                for (p, t) in pats.iter().zip(ts) {
                    self.record_pattern_static_types(p, t);
                }
            }
            _ => {}
        }
    }

    /// The variable a pattern matches, with every name it binds bound and related to it by parts.
    pub(super) fn bind_pattern(&mut self, pattern: &Pattern<'ast>) -> ConstraintVar {
        match pattern {
            Pattern::Identifier(name) | Pattern::MutBinding(name) => self.bind_name(name),
            Pattern::Tuple(pats) => {
                let var = self.cs.fresh_var();
                for (i, p) in pats.iter().enumerate() {
                    let pv = self.bind_pattern(p);
                    self.part(var, Part::Field(i as u8), pv, "NUM-TUPLE");
                }
                var
            }
            Pattern::Wildcard => self.cs.fresh_var(),
            Pattern::Literal(lit) => self.pattern_literal(lit),
            Pattern::Range { start, end, .. } => match start.as_ref().or(end.as_ref()) {
                Some(lit) => self.pattern_literal(lit),
                None => self.cs.fresh_var(),
            },
            Pattern::EnumVariant(name, binding) => self.bind_variant(name, binding),
            Pattern::Or(pats) => {
                let var = self.cs.fresh_var();
                for p in pats {
                    let pv = self.bind_pattern(p);
                    self.cs.add_because(Constraint::TypeEquals(var, pv), tagged("TY-UNIFY"));
                }
                var
            }
            Pattern::Reference(inner) => self.bind_pattern(inner),
            Pattern::Ref(name) | Pattern::RefMut(name) => self.resolve_var(name),
        }
    }

    /// A literal in a pattern takes the matched value's width: an unsuffixed number is only a numeric class.
    fn pattern_literal(&mut self, lit: &Literal) -> ConstraintVar {
        let var = self.cs.fresh_var();
        match lit {
            Literal::Int(_) => self.cs.add_because(Constraint::IsInteger(var), tagged(literal_rule(lit))),
            Literal::Float(_) => self.cs.add_because(Constraint::IsFloat(var), tagged(literal_rule(lit))),
            _ => {
                let base = literal_to_base_type(lit);
                self.cs.add_because(Constraint::TypeIs(var, base.clone()), tagged(literal_rule(lit)));
                emit_numeric_class_constraint(&mut self.cs, var, &base);
            }
        }
        var
    }

    /// `Some(x)`, `Ok(x)`, `Err(e)`: the payload part of the matched value. A user variant's bindings take the
    /// variant's declared payload types; a binding whose type is not known is held (unknown field).
    fn bind_variant(&mut self, name: &str, binding: &EnumPatternBinding<'ast>) -> ConstraintVar {
        let var = self.cs.fresh_var();
        let prelude = prelude_payload(name);
        let declared = self.variant_payload_fields(name);
        let subs: Vec<(String, ConstraintVar)> = match binding {
            EnumPatternBinding::Single(n) => vec![("0".to_string(), self.bind_name(n))],
            EnumPatternBinding::Tuple(pats) => pats.iter().enumerate().map(|(i, p)| (i.to_string(), self.bind_pattern(p))).collect(),
            EnumPatternBinding::Struct(fields, _) => fields.iter().map(|(f, p)| (f.clone(), self.bind_pattern(p))).collect(),
            EnumPatternBinding::None | EnumPatternBinding::Wildcard => Vec::new(),
        };
        for (key, sub) in subs {
            if let (Some(part), "0") = (prelude, key.as_str()) {
                self.part(var, part, sub, "NUM-PAYLOAD");
                continue;
            }
            // A user-type payload (`Box(extents)` of a `Vec3`): the binding has that type, so `extents.x` is a field read.
            if let Some(t) = declared.as_ref().and_then(|d| d.get(&key)).filter(|t| !super::numeric::contains_unfixed(t)) {
                self.var_static_types.entry(sub).or_insert_with(|| t.clone());
            }
            let base = declared
                .as_ref()
                .and_then(|d| d.get(&key))
                .map(parser_type_to_base_type)
                .filter(|b| *b != BaseType::Inferred && !crate::ir::constraints::contains_named(b));
            match base {
                Some(b) => {
                    self.cs.add_because(Constraint::TypeIs(sub, b.clone()), tagged("NUM-FIELD-DECLARED"));
                    emit_numeric_class_constraint(&mut self.cs, sub, &b);
                }
                None => self.numeric_unmodelled.unknown_fields.push(sub),
            }
        }
        var
    }

    /// `Some(x)` / `Ok(x)` / `Err(e)` built by a call: the argument is the result's payload. Returns whether the
    /// call is such a constructor (its typing is complete: no signature is consulted).
    pub(super) fn prelude_constructor_parts(&mut self, callee: &str, result: ConstraintVar, arg_vars: &[ConstraintVar]) -> bool {
        match (prelude_payload(callee), arg_vars) {
            (Some(part), [arg]) => {
                self.part(result, part, *arg, "NUM-PAYLOAD");
                true
            }
            _ => false,
        }
    }

    /// ADR-0004: the value a task hands back (`t.wait()`) is its operand's value.
    pub(super) fn task_value(&mut self, task: ConstraintVar, value: ConstraintVar) {
        self.part(task, Part::Elem, value, "NUM-TASK");
    }

    /// `spawn f(x)` / `async { .. }`: a `Task<T>` whose `T` is the operand's value.
    pub(super) fn walk_task(&mut self, expr: &Expression<'ast>, mode: ExecutionMode, location: &crate::parser::SourceLocation) -> ConstraintVar {
        let inner = self.walk_expression(expr);
        self.emit_execution_call_mode(expr, mode, location);
        let result = self.cs.fresh_var();
        self.task_value(result, inner);
        result
    }

    /// `select { pat = src => body, .. }`: each arm's pattern binds its source's element (`rx.recv()`, `t.wait()`) and
    /// every arm's value is the select's.
    pub(super) fn walk_select(&mut self, arms: &[crate::parser::SelectArm<'ast>]) -> ConstraintVar {
        let result = self.cs.fresh_var();
        for a in arms {
            let source = a.source.map(|s| self.walk_expression(s));
            self.scopes.push(HashMap::new());
            let bound = self.bind_pattern(&a.pattern);
            if let (Some(src), crate::parser::SelectKind::Recv | crate::parser::SelectKind::Wait) = (source, a.kind) {
                self.task_value(src, bound);
            }
            let body = self.walk_expression(a.body);
            self.scopes.pop();
            self.cs.add_because(Constraint::TypeEquals(result, body), tagged("TY-UNIFY"));
        }
        result
    }

    /// `Vec::new()`, `Vec::with_capacity(n)`: an associated function of a core type, typed by `std/rust_core.wji`
    /// (its result is a fresh container whose parts are related to the arguments). Returns whether it was typed.
    pub(super) fn emit_surface_static_call(
        &mut self,
        callee: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        arg_vars: &[ConstraintVar],
        result: ConstraintVar,
    ) -> bool {
        let Some((type_name, (type_generics, method_generics, params, ret))) = self.surface_static(callee) else { return false };
        if params.len() != arguments.len() {
            return false;
        }
        let mut g = SurfaceGenerics::new(type_generics.iter().chain(&method_generics));
        for ((param, &v), (_, a)) in params.iter().zip(arg_vars).zip(arguments) {
            self.bind_surface_type(param, v, &mut g, &a.location());
        }
        let ret = ret.unwrap_or_else(|| crate::typing::core_surface::self_type(type_name, &type_generics));
        self.bind_surface_type(&ret, result, &mut g, &None);
        self.surface_typed.insert(result);
        true
    }

    #[allow(clippy::type_complexity)]
    pub(super) fn surface_static(&self, callee: &str) -> Option<(&'static str, (Vec<String>, Vec<String>, Vec<Type>, Option<Type>))> {
        let (ty, method) = callee.rsplit_once("::")?;
        let type_name = crate::typing::core_surface::surface_type_name(&Type::Parameterized(ty.to_string(), Vec::new()))?;
        let entry = self.core_surface?.declared_static(type_name, method)?;
        Some((type_name, entry))
    }
}

/// The payload part a prelude constructor (`Option` / `Result`, docs/spec/types.md) wraps its argument in.
fn prelude_payload(constructor: &str) -> Option<Part> {
    match constructor.rsplit("::").next().unwrap_or(constructor) {
        "Some" | "Ok" => Some(Part::Payload),
        "Err" => Some(Part::ErrPayload),
        _ => None,
    }
}

/// The generics of a surface signature and the variable each stands for at one call.
pub(super) struct SurfaceGenerics {
    names: std::collections::BTreeSet<String>,
    vars: std::collections::BTreeMap<String, ConstraintVar>,
}

impl SurfaceGenerics {
    fn new<'s>(names: impl IntoIterator<Item = &'s String>) -> Self {
        Self { names: names.into_iter().cloned().collect(), vars: Default::default() }
    }
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// Relate `var` to a surface type: a generic is one variable per call, a container type relates `var` to its
    /// parts, a numeric type is exact (`usize` is an index use). References carry no width.
    fn bind_surface_type(&mut self, ty: &Type, var: ConstraintVar, g: &mut SurfaceGenerics, at: &crate::parser::SourceLocation) {
        let mut sub = |w: &mut Self, part: Part, inner: &Type, g: &mut SurfaceGenerics| {
            let pv = w.cs.fresh_var();
            w.part(var, part, pv, "NUM-STD-SIG");
            w.bind_surface_type(inner, pv, g, at);
        };
        match ty {
            Type::Generic(n) | Type::Custom(n) if g.names.contains(n) => match g.vars.get(n) {
                Some(&gv) => self.cs.add_because(Constraint::TypeEquals(var, gv), tagged("NUM-STD-SIG")),
                None => {
                    g.vars.insert(n.clone(), var);
                }
            },
            Type::Reference(x) | Type::MutableReference(x) => self.bind_surface_type(x, var, g, at),
            Type::Vec(x) | Type::Array(x, _) => sub(self, Part::Elem, x, g),
            Type::Option(x) => sub(self, Part::Payload, x, g),
            Type::Result(a, b) => {
                sub(self, Part::Payload, a, g);
                sub(self, Part::ErrPayload, b, g);
            }
            Type::Tuple(xs) => {
                for (i, x) in xs.iter().enumerate() {
                    sub(self, Part::Field(i as u8), x, g);
                }
            }
            Type::FunctionPointer { params, return_type } => {
                for (i, x) in params.iter().enumerate() {
                    sub(self, Part::Param(i as u8), x, g);
                }
                if let Some(r) = return_type {
                    sub(self, Part::Ret, r, g);
                }
            }
            Type::Parameterized(n, args) if args.len() == 1 && crate::typing::core_surface::has_element_part(n) => {
                sub(self, Part::Elem, &args[0], g)
            }
            Type::Parameterized(n, args) if args.len() == 2 && crate::typing::core_surface::is_map_container(n) => {
                sub(self, Part::Key, &args[0], g);
                sub(self, Part::Value, &args[1], g);
            }
            other => match super::numeric::numeric_base_of(other) {
                Some(BaseType::Usize) => {
                    self.index_uses.push(var);
                    self.cs.add_because(Constraint::IsNumeric(var), origin("NUM-INDEX", "a `usize` slot of a std method is an index", at));
                    self.cs.add_because(Constraint::PrefersUsize(var), origin("NUM-USIZE", "a `usize` slot of a std method is an index", at));
                }
                Some(base) => {
                    self.cs.add_because(Constraint::TypeIs(var, base.clone()), origin("NUM-STD-SIG", "a std method's declared numeric type", at));
                    emit_numeric_class_constraint(&mut self.cs, var, &base);
                }
                None => {}
            },
        }
    }

    /// The static shape of an expression for choosing a surface entry: its static type, or for a collection
    /// literal `Vec<_>`, or for a surface method call the declared result (generics left unfixed).
    pub(super) fn surface_shape(&self, expr: &Expression<'ast>) -> Option<Type> {
        if let Some(t) = self.static_type(expr) {
            return Some(t);
        }
        match expr {
            Expression::Array { .. } => Some(Type::Vec(Box::new(Type::Infer))),
            // ADR-0004: `spawn ..` / `async ..` is a `Task<T>` (std/rust_core.wji): `t.wait()` is the operand's value
            Expression::SpawnCall { .. } | Expression::AsyncCall { .. } => Some(Type::Parameterized("Task".to_string(), vec![Type::Infer])),
            Expression::Range { .. } => Some(Type::Parameterized("Iterator".to_string(), vec![Type::Infer])),
            Expression::MapLiteral { .. } => Some(Type::Parameterized("HashMap".to_string(), vec![Type::Infer, Type::Infer])),
            Expression::MacroInvocation { delimiter: MacroDelimiter::Brackets, .. } => Some(Type::Vec(Box::new(Type::Infer))),
            Expression::Identifier { name, .. } => self.bound_shape(name),
            Expression::Call { function, .. } => {
                let callee = Self::extract_callee_name(function)?;
                let (type_name, (generics, _, _, ret)) = self.surface_static(&callee)?;
                Some(ret.unwrap_or_else(|| crate::typing::core_surface::self_type(type_name, &generics)))
            }
            Expression::MethodCall { object, method, .. } => {
                // `thread::spawn(..)`: a std module function the surface declares (the receiver names a module).
                if let Expression::Identifier { name, .. } = &**object {
                    if self.bound_var(name).is_none() {
                        if let Some((type_name, (generics, _, _, ret))) = self.surface_static(&format!("{name}::{method}")) {
                            return Some(ret.unwrap_or_else(|| crate::typing::core_surface::self_type(type_name, &generics)));
                        }
                    }
                }
                let receiver = self.surface_shape(object)?;
                let type_name = crate::typing::core_surface::surface_type_name(&receiver)?;
                let (_, _, _, ret) = self.core_surface?.declared(type_name, method)?;
                ret.filter(|r| crate::typing::core_surface::surface_type_name(r).is_some())
            }
            _ => None,
        }
    }

    fn bound_shape(&self, name: &str) -> Option<Type> {
        let v = self.scopes.iter().rev().find_map(|s| s.get(name)).copied()?;
        self.var_static_types.get(&v).cloned()
    }

    /// Option C stage C2: a method of a Rust core type, typed by `std/rust_core.wji` structurally. The receiver is
    /// bound to the declared receiver type (its generics become variables shared with the arguments and the
    /// result), a closure argument meets a `fn(..)` parameter through its parameter and result parts, a turbofish
    /// fixes a method generic (or, on a method without generics, the result). Returns whether the surface typed the call.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_core_surface_constraints(
        &mut self,
        expr: &Expression<'ast>,
        object: &'ast Expression<'ast>,
        obj_var: ConstraintVar,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        arg_vars: &[ConstraintVar],
        result: ConstraintVar,
    ) -> bool {
        let Expression::MethodCall { type_args, .. } = expr else { return false };
        let Some(receiver) = self.surface_shape(object) else { return false };
        let Some(surface) = self.core_surface else { return false };
        let Some(type_name) = crate::typing::core_surface::surface_type_name(&receiver) else { return false };
        let Some((type_generics, method_generics, params, ret)) = surface.declared(type_name, method) else { return false };
        if params.len() != arguments.len() {
            return false;
        }
        let at = expr.location();
        let mut g = SurfaceGenerics::new(type_generics.iter().chain(&method_generics));
        let self_ty = crate::typing::core_surface::self_type(type_name, &type_generics);
        self.bind_surface_type(&self_ty, obj_var, &mut g, &at);
        // A receiver whose static type is known fixes the generics it names (`v: Vec<u8>`).
        self.bind_surface_type(&receiver, obj_var, &mut SurfaceGenerics::new(&[]), &at);
        let turbofish = type_args.as_deref().unwrap_or(&[]);
        if method_generics.is_empty() {
            if let Some(t) = turbofish.first() {
                self.bind_surface_type(t, result, &mut SurfaceGenerics::new(&[]), &at);
            }
        }
        for (name, t) in method_generics.iter().zip(turbofish) {
            let v = self.cs.fresh_var();
            g.vars.entry(name.clone()).or_insert(v);
            let gv = g.vars[name];
            self.bind_surface_type(t, gv, &mut SurfaceGenerics::new(&[]), &at);
        }
        for ((param, &v), (_, a)) in params.iter().zip(arg_vars).zip(arguments) {
            self.bind_surface_argument(param, v, a, &mut g);
        }
        self.refine_element_from_argument(object, &type_generics, &params, arguments);
        match ret.as_ref() {
            Some(r) if super::numeric::numeric_base_of(r) == Some(BaseType::Usize) => {
                if let Some(l) = expr.location() {
                    self.index_sources.push(((l.line, l.column), result));
                }
                self.cs.add_because(
                    Constraint::PrefersUsize(result),
                    origin("NUM-USIZE", format!("`.{}()` is a length: an index class unless a width is declared", method), &at),
                );
            }
            Some(r) => self.bind_surface_type(r, result, &mut g, &at),
            None => {}
        }
        self.surface_typed.insert(result);
        let _ = object;
        true
    }
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// An argument at a formal whose type the solver cannot read structurally (a generic, an alias or another named
    /// type, a type with an unknown part) is opaque: the formal may fix a width the walk does not see.
    pub(super) fn note_opaque_formal(&mut self, formal: &BaseType, arg: ConstraintVar) {
        if crate::ir::constraints::contains_named(formal) || !crate::ir::constraints::is_fully_known(formal) {
            self.numeric_unmodelled.opaque.push(arg);
        }
    }

    /// A call with no signature is still modelled when it constructs a prelude payload or a declared variant.
    pub(super) fn call_is_modelled(&self, callee: &str) -> bool {
        prelude_payload(callee).is_some() || self.variant_payload_fields(callee).is_some()
    }

    /// `S { f: v }`: `v` has field `f`'s declared type; a field of a struct the walk cannot see is opaque.
    pub(super) fn type_struct_literal_field(&mut self, struct_name: &str, field: &str, v: ConstraintVar) {
        let declared = self.struct_field_types.and_then(|f| f.get(struct_name)).and_then(|fs| fs.get(field)).map(parser_type_to_base_type);
        match declared {
            Some(base) if base != BaseType::Inferred => {
                self.note_opaque_formal(&base, v);
                self.cs.add_because(Constraint::TypeIs(v, base.clone()), tagged("NUM-FIELD-DECLARED"));
                emit_numeric_class_constraint(&mut self.cs, v, &base);
            }
            _ => self.numeric_unmodelled.opaque.push(v),
        }
    }
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// A name in an expression: `None` is the prelude's empty `Option` (a fresh value, not a local); any other
    /// name is its binding.
    pub(super) fn resolve_identifier(&mut self, name: &str) -> ConstraintVar {
        if name == "None" {
            return self.cs.fresh_var();
        }
        self.resolve_var(name)
    }
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// `p * 0.5` where `p` is a user type with `impl Mul<f32> for P`: the operator is that impl's method (`mul`), so
    /// the right operand has the method's declared parameter type and the result its return type. Returns whether
    /// the left operand is such a type (then the operands are not unified with each other).
    pub(super) fn emit_operator_impl(&mut self, op: &BinaryOp, left: &Expression<'ast>, r: ConstraintVar, result: ConstraintVar) -> bool {
        let Some(method) = operator_method(op) else { return false };
        let Some(Type::Custom(type_name)) = self.static_type(left) else { return false };
        let Some(sig) = self.resolve_type_member(&type_name, method, 1).cloned() else { return false };
        let Some(rhs) = sig.formal_param_type(sig.arg_param_index(0)).or_else(|| sig.param_types.get(sig.arg_param_index(0))).cloned() else {
            return false;
        };
        let rhs = parser_type_to_base_type(&rhs);
        if rhs != BaseType::Inferred {
            self.cs.add_because(Constraint::TypeIs(r, rhs.clone()), tagged("NUM-OPERATOR-IMPL"));
            emit_numeric_class_constraint(&mut self.cs, r, &rhs);
        }
        if let Some(ret) = sig.return_type.as_ref().map(parser_type_to_base_type).filter(|b| *b != BaseType::Inferred) {
            self.cs.add_because(Constraint::TypeIs(result, ret), tagged("NUM-OPERATOR-IMPL"));
        }
        true
    }

    /// The static type of `left op right` when `left` is a user type with an operator impl: the impl method's declared
    /// result (`(a + b) * 0.5` is a `P` when `impl Add for P` returns `P`).
    pub(super) fn operator_impl_result(&self, op: &BinaryOp, left: &Expression<'ast>) -> Option<Type> {
        let method = operator_method(op)?;
        let receiver = self.static_type(left)?;
        let Type::Custom(_) = &receiver else { return None };
        self.user_method_return(&receiver, method, 1)
    }
}

/// The operator-trait method an arithmetic operator calls on a user type (`a * b` is `a.mul(b)`).
fn operator_method(op: &BinaryOp) -> Option<&'static str> {
    Some(match op {
        BinaryOp::Add => "add",
        BinaryOp::Sub => "sub",
        BinaryOp::Mul => "mul",
        BinaryOp::Div => "div",
        BinaryOp::Mod => "rem",
        _ => return None,
    })
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// Before the arguments of `receiver.method(.., |p| ..)` are walked: when the typed core surface gives the
    /// closure's parameters concrete types (the receiver's element instantiated), record them so the closure body
    /// sees its parameters' static types (a method on a parameter is then typed by the surface too).
    pub(super) fn prepare_closure_param_types(
        &mut self,
        object: &Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        type_args: &[Type],
    ) {
        let Some(receiver) = self.surface_shape(object) else { return };
        let Some(sig) = self.core_surface.and_then(|s| s.method_signature(&receiver, method, type_args)) else { return };
        for (param, (_, arg)) in sig.params.iter().zip(arguments) {
            let (Type::FunctionPointer { params, .. }, Expression::Closure { .. }) = (param, arg) else { continue };
            let id = crate::ir::numeric_defaults::node_id(arg);
            for (i, t) in params.iter().enumerate() {
                if !super::numeric::contains_unfixed(t) {
                    self.closure_param_types.insert((id, i), t.clone());
                }
            }
        }
    }
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// An argument of a surface method. At a slot that is one of the signature's generics (`push(value: T)`) the value
    /// flows into the slot (`FlowsInto`: an index class keeps its width and is converted at the argument); any other
    /// parameter type relates the argument structurally.
    fn bind_surface_argument(&mut self, param: &Type, arg: ConstraintVar, arg_expr: &Expression<'ast>, g: &mut SurfaceGenerics) {
        // An argument at a declared `usize` slot (`s[i..i + 1]` is `s.slice(i, i + 1)`) keeps its class's width and is
        // converted where it is passed, as at a user `usize` formal (NUM-USIZE-TO-INT).
        if super::numeric::numeric_base_of(param) == Some(BaseType::Usize) {
            if let Some(key) = crate::ir::numeric_facts::conversion_key(arg_expr) {
                self.index_use_keys.push((key, arg));
            }
        }
        let (Type::Generic(n) | Type::Custom(n)) = param else {
            return self.bind_surface_type(param, arg, g, &arg_expr.location());
        };
        if !g.names.contains(n) || matches!(arg_expr, Expression::Closure { .. }) {
            return self.bind_surface_type(param, arg, g, &arg_expr.location());
        }
        let slot = match g.vars.get(n) {
            Some(&v) => v,
            None => {
                let v = self.cs.fresh_var();
                g.vars.insert(n.clone(), v);
                v
            }
        };
        self.cs.add_because(Constraint::FlowsInto(arg, slot), tagged("NUM-STD-SIG"));
        if let Some(key) = crate::ir::numeric_facts::conversion_key(arg_expr) {
            self.element_stores.push((key, arg, slot));
        }
    }
}

/// A type with the file's `type` aliases replaced by their targets (`Pair<int>` with `type Pair<T> = (T, T)` is
/// `(int, int)`), so the solver reads the structure an alias names.
pub(crate) fn expand_aliases(ty: &Type, aliases: &std::collections::HashMap<String, (Vec<String>, Type)>) -> Type {
    expand_aliases_depth(ty, aliases, 0)
}

fn expand_aliases_depth(ty: &Type, aliases: &std::collections::HashMap<String, (Vec<String>, Type)>, depth: usize) -> Type {
    if depth > 8 || aliases.is_empty() {
        return ty.clone();
    }
    let rec = |t: &Type| expand_aliases_depth(t, aliases, depth + 1);
    match ty {
        Type::Custom(n) => match aliases.get(n) {
            Some((params, target)) if params.is_empty() => rec(target),
            _ => ty.clone(),
        },
        Type::Parameterized(n, args) => match aliases.get(n) {
            Some((params, target)) if params.len() == args.len() => {
                let args: Vec<Type> = args.iter().map(rec).collect();
                rec(&substitute_params(target, params, &args))
            }
            _ => Type::Parameterized(n.clone(), args.iter().map(rec).collect()),
        },
        Type::Vec(x) => Type::Vec(Box::new(rec(x))),
        Type::Option(x) => Type::Option(Box::new(rec(x))),
        Type::Reference(x) => Type::Reference(Box::new(rec(x))),
        Type::MutableReference(x) => Type::MutableReference(Box::new(rec(x))),
        Type::Array(x, n) => Type::Array(Box::new(rec(x)), *n),
        Type::Result(a, b) => Type::Result(Box::new(rec(a)), Box::new(rec(b))),
        Type::Tuple(xs) => Type::Tuple(xs.iter().map(rec).collect()),
        other => other.clone(),
    }
}

fn substitute_params(t: &Type, params: &[String], args: &[Type]) -> Type {
    let sub = |x: &Type| substitute_params(x, params, args);
    match t {
        Type::Custom(n) | Type::Generic(n) => params.iter().position(|p| p == n).map_or_else(|| t.clone(), |i| args[i].clone()),
        Type::Parameterized(n, xs) => Type::Parameterized(n.clone(), xs.iter().map(sub).collect()),
        Type::Vec(x) => Type::Vec(Box::new(sub(x))),
        Type::Option(x) => Type::Option(Box::new(sub(x))),
        Type::Reference(x) => Type::Reference(Box::new(sub(x))),
        Type::MutableReference(x) => Type::MutableReference(Box::new(sub(x))),
        Type::Array(x, n) => Type::Array(Box::new(sub(x)), *n),
        Type::Result(a, b) => Type::Result(Box::new(sub(a)), Box::new(sub(b))),
        Type::Tuple(xs) => Type::Tuple(xs.iter().map(sub).collect()),
        other => other.clone(),
    }
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// A built-in function (`println`, `debug`, `panic`, `assert`, ...; the registry is `intrinsics`) displays or checks its
    /// arguments and never stores them at a width, so the call is modelled (not opaque). `assert_eq` / `assert_ne` compare
    /// two values of one type, like `a == b`: the two are one class (a side the walk cannot type, such as a user type's
    /// `len()`, holds that class for the legacy deciders). Returns whether the call is modelled here.
    pub(super) fn intrinsic_call(&mut self, callee: &str, arg_vars: &[ConstraintVar]) -> bool {
        if arg_vars.len() >= 2 && crate::intrinsics::Intrinsic::is_equality_assertion(callee) {
            self.relate_equality_assertion(arg_vars);
            return true;
        }
        crate::intrinsics::Intrinsic::from_call_name(callee).is_some()
    }

    /// `assert_eq(a, b)` / `assert_ne(a, b)`: `a` and `b` have one type (rustc's `PartialEq` on two values of a type).
    pub(super) fn relate_equality_assertion(&mut self, arg_vars: &[ConstraintVar]) {
        self.cs.add_because(Constraint::TypeEquals(arg_vars[0], arg_vars[1]), tagged("TY-UNIFY"));
    }
}


impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// `loop { .. }`: its value is the value of its `break`s (`let r = loop { .. break k * 10 }`). Returns that value.
    pub(super) fn walk_loop(&mut self, body: &[&'ast Statement<'ast>]) -> ConstraintVar {
        let value = self.cs.fresh_var();
        self.loop_values.push(value);
        self.scopes.push(HashMap::new());
        for s in body {
            self.walk_statement(s);
        }
        self.scopes.pop();
        self.loop_values.pop();
        value
    }

    /// `break v`: `v` is walked like any expression and is the innermost `loop`'s value.
    pub(super) fn walk_break_value(&mut self, value: Option<&'ast Expression<'ast>>) {
        let Some(v) = value else { return };
        let var = self.walk_expression(v);
        if let Some(&loop_value) = self.loop_values.last() {
            self.cs.add_because(Constraint::TypeEquals(loop_value, var), tagged("TY-UNIFY"));
        }
    }
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// An argument at a `usize` formal (`substring(s, start, end)`) is an index use: its class keeps its own width and,
    /// when that width is another integer type, the argument is converted where it is passed (NUM-USIZE-TO-INT).
    /// NUM-USIZE-TO-INT at the index: `v[k]` on a sequence (a `Vec`, `VecDeque`, list or text) whose index `k` is in a
    /// class of another width is converted where it indexes (`v[(k as usize)]`), as an argument at a `usize` formal is.
    /// Map lookups and slice bounds are not positions.
    pub(super) fn index_conversion_site(&mut self, object: &Expression<'ast>, index: &Expression<'ast>, idx_var: ConstraintVar) {
        if matches!(index, Expression::Range { .. }) {
            return;
        }
        let positional = self.surface_shape(object).as_ref().and_then(crate::typing::core_surface::surface_type_name).is_some_and(|n| {
            matches!(n, "String") || (crate::typing::core_surface::is_element_container(n) && !matches!(n, "HashSet" | "BTreeSet" | "Iterator"))
        });
        if let (true, Some(key)) = (positional, crate::ir::numeric_facts::conversion_key(index)) {
            self.index_use_keys.push((key, idx_var));
        }
    }

    pub(super) fn index_slot_argument(&mut self, arg: ConstraintVar, arg_expr: Option<&Expression<'ast>>) {
        self.cs.add_because(Constraint::IsNumeric(arg), tagged("NUM-INDEX"));
        self.index_uses.push(arg);
        self.cs.add_because(Constraint::PrefersUsize(arg), tagged("NUM-USIZE"));
        if let Some(key) = arg_expr.and_then(crate::ir::numeric_facts::conversion_key) {
            self.index_use_keys.push((key, arg));
        }
    }
}
