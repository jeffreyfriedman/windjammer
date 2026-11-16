//! The scoped walk over one function body: every local binding, and the first place each one is changed.
//!
//! A change is an assignment to the binding (`x = ..`, `x += ..`), an assignment through it (`x.f = ..`,
//! `x[i] = ..`), a call of a method that changes its receiver (`x.push(..)`, `x.items.push(..)`), passing it where
//! the callee changes the argument (`fill(x)`, `&mut x`), including inside a closure. Only bindings whose value the
//! declarations decide are judged by changes through them: a binding that holds a reference, or whose type is
//! unknown, is only judged by assignments to the binding itself.

use super::facts::Facts;
use super::types::{self, is_reference, strip_refs, user_type, Env};
use crate::parser::ast::core::{EnumPatternBinding, Expression, FunctionDecl, Pattern, Statement};
use crate::parser::{SourceLocation, Type, UnaryOp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `let` (also each name of a `let` pattern).
    Let,
    /// A `for` loop variable (each name of its pattern).
    For,
    /// Parameters, closure parameters, match arm and other pattern bindings: never reported.
    Other,
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub name: String,
    pub kind: Kind,
    pub declared_mut: bool,
    pub ty: Option<Type>,
    /// The `let` / `for` statement's own location (the parser records the end of the statement).
    pub stmt_end: SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Assign { compound: bool },
    Field,
    Element,
    Method(String),
    Argument(String),
    LendMut,
}

#[derive(Debug, Clone)]
pub struct Found {
    pub binding: Binding,
    pub change: Change,
    /// Location of the binding's name at the change (or of the enclosing `${..}`).
    pub at: SourceLocation,
    pub in_closure: bool,
}

pub struct Walker<'f> {
    facts: &'f Facts,
    scopes: Vec<Vec<usize>>,
    bindings: Vec<Binding>,
    reported: Vec<bool>,
    pub found: Vec<Found>,
    macro_locs: Vec<SourceLocation>,
    closure_depth: usize,
    self_type: Option<Type>,
    /// Names declared `mut` in `for` patterns, by the `for` statement's (line, column) (the parser's side table).
    loop_mut: std::collections::HashSet<(usize, usize, String)>,
}

impl Env for Walker<'_> {
    fn binding_type(&self, name: &str) -> Option<Option<Type>> {
        if name == "self" {
            return self.self_type.clone().map(Some);
        }
        self.lookup(name).map(|i| self.bindings[i].ty.clone())
    }
}

/// The local a place expression is rooted at (`x`, `x.f`, `x[i].g`), with the root's location. A dereference, a
/// call or anything else in the chain is not a place of a local.
fn place_root<'a>(e: &'a Expression<'a>) -> Option<(&'a str, &'a SourceLocation)> {
    match e {
        Expression::Identifier { name, location } => Some((name.as_str(), location)),
        Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => place_root(object),
        _ => None,
    }
}

impl<'f> Walker<'f> {
    pub fn new(facts: &'f Facts) -> Self {
        Walker {
            facts,
            scopes: Vec::new(),
            bindings: Vec::new(),
            reported: Vec::new(),
            found: Vec::new(),
            macro_locs: Vec::new(),
            closure_depth: 0,
            self_type: None,
            loop_mut: Default::default(),
        }
    }

    /// The `for` patterns' `mut` names recorded by the parser.
    pub fn with_loop_mut(mut self, names: &[(usize, usize, String)]) -> Self {
        self.loop_mut = names.iter().cloned().collect();
        self
    }

    fn lookup(&self, name: &str) -> Option<usize> {
        self.scopes.iter().rev().flat_map(|s| s.iter().rev()).copied().find(|&i| self.bindings[i].name == name)
    }

    fn bind(&mut self, b: Binding) {
        self.bindings.push(b);
        self.reported.push(false);
        let id = self.bindings.len() - 1;
        if let Some(s) = self.scopes.last_mut() {
            s.push(id);
        }
    }

    fn bind_other(&mut self, name: &str) {
        self.bind(Binding { name: name.to_string(), kind: Kind::Other, declared_mut: false, ty: None, stmt_end: None });
    }

    pub fn function(&mut self, decl: &FunctionDecl<'_>) {
        self.self_type = decl.parent_type.as_ref().map(|p| Type::Custom(crate::type_classification::type_name_leaf(p).to_string()));
        self.scopes.push(Vec::new());
        for p in &decl.parameters {
            if p.name == "self" {
                continue;
            }
            match &p.pattern {
                Some(pat) => self.bind_pattern(pat, None, Kind::Other, false, &None),
                None => self.bind(Binding { name: p.name.clone(), kind: Kind::Other, declared_mut: false, ty: Some(p.type_.clone()), stmt_end: None }),
            }
        }
        self.block(&decl.body);
        self.scopes.pop();
    }

    fn block(&mut self, stmts: &[&Statement<'_>]) {
        self.scopes.push(Vec::new());
        for s in stmts {
            self.stmt(s);
        }
        self.scopes.pop();
    }

    fn bind_pattern(&mut self, p: &Pattern<'_>, ty: Option<Type>, kind: Kind, all_mut: bool, end: &SourceLocation) {
        let ty_ref = ty.as_ref().map(strip_refs);
        // a binding inside a pattern matched through a reference holds a reference
        let through_ref = ty.as_ref().is_some_and(is_reference);
        let part = |t: Option<Type>| if through_ref { t.map(|t| Type::Reference(Box::new(t))) } else { t };
        match p {
            Pattern::Identifier(n) => {
                self.bind(Binding { name: n.clone(), kind, declared_mut: all_mut, ty, stmt_end: end.clone() });
            }
            Pattern::MutBinding(n) => {
                self.bind(Binding { name: n.clone(), kind, declared_mut: true, ty, stmt_end: end.clone() });
            }
            Pattern::Ref(n) | Pattern::RefMut(n) => self.bind_other(n),
            Pattern::Tuple(ps) => {
                for (i, q) in ps.iter().enumerate() {
                    let t = match ty_ref {
                        Some(Type::Tuple(ts)) => ts.get(i).cloned().filter(|t| *t != Type::Infer),
                        _ => None,
                    };
                    self.bind_pattern(q, part(t), kind, all_mut, end);
                }
            }
            Pattern::Reference(q) => {
                let t = match ty_ref {
                    Some(t) if through_ref => Some(t.clone()),
                    _ => None,
                };
                self.bind_pattern(q, t, kind, all_mut, end);
            }
            Pattern::Or(ps) => {
                // names of an or-pattern are matched values, not declared locals
                for q in ps {
                    self.bind_pattern(q, None, Kind::Other, all_mut, end);
                }
            }
            Pattern::EnumVariant(variant, binding) => {
                let payload = self.payload_types(variant, ty_ref);
                match binding {
                    EnumPatternBinding::Single(n) => {
                        let t = payload.as_ref().and_then(|ts| ts.first().cloned());
                        self.bind(Binding { name: n.clone(), kind, declared_mut: all_mut, ty: part(t), stmt_end: end.clone() });
                    }
                    EnumPatternBinding::Tuple(ps) => {
                        for (i, q) in ps.iter().enumerate() {
                            let t = payload.as_ref().and_then(|ts| ts.get(i).cloned());
                            self.bind_pattern(q, part(t), kind, all_mut, end);
                        }
                    }
                    EnumPatternBinding::Struct(fields, _) => {
                        let leaf = crate::type_classification::type_name_leaf(variant);
                        let declared = self.facts.fields.get(leaf).filter(|_| self.facts.types.contains(leaf));
                        for (f, q) in fields {
                            let t = declared.and_then(|fs| fs.get(f).cloned());
                            self.bind_pattern(q, part(t), kind, all_mut, end);
                        }
                    }
                    EnumPatternBinding::None | EnumPatternBinding::Wildcard => {}
                }
            }
            Pattern::Wildcard | Pattern::Literal(_) | Pattern::Range { .. } => {}
        }
    }

    /// Payload types of `Some(..)`, `Ok(..)`, `Err(..)` or a user enum variant matched against `ty`.
    fn payload_types(&self, variant: &str, ty: Option<&Type>) -> Option<Vec<Type>> {
        let leaf = crate::type_classification::type_name_leaf(variant);
        match (leaf, ty?) {
            ("Some", Type::Option(t)) | ("Ok", Type::Result(t, _)) | ("Err", Type::Result(_, t)) => Some(vec![(**t).clone()]),
            _ => {
                let owner = variant.rsplit_once("::").map(|(o, _)| crate::type_classification::type_name_leaf(o))?;
                self.facts.variants.get(owner)?.get(leaf).cloned()
            }
        }
    }

    fn stmt(&mut self, s: &Statement<'_>) {
        match s {
            Statement::Let { pattern, mutable, type_, value, else_block, location } => {
                self.expr(value);
                if let Some(b) = else_block {
                    self.block(b);
                }
                let ty = type_.clone().or_else(|| types::type_of(self.facts, self, value));
                self.bind_pattern(pattern, ty, Kind::Let, *mutable, location);
            }
            Statement::Const { name, value, .. } | Statement::Static { name, value, .. } => {
                self.expr(value);
                self.bind_other(name);
            }
            Statement::Assignment { target, value, compound_op, .. } => {
                self.expr(value);
                self.assigned(target, compound_op.is_some());
            }
            Statement::Return { value, .. } | Statement::Break { value, .. } => {
                if let Some(v) = value {
                    self.expr(v);
                }
            }
            Statement::Expression { expr, .. } => self.expr(expr),
            Statement::If { condition, then_block, else_block, .. } => {
                self.expr(condition);
                self.block(then_block);
                if let Some(b) = else_block {
                    self.block(b);
                }
            }
            Statement::Match { value, arms, .. } => {
                self.expr(value);
                for arm in arms {
                    self.scopes.push(Vec::new());
                    self.bind_pattern(&arm.pattern, None, Kind::Other, false, &None);
                    if let Some(g) = arm.guard {
                        self.expr(g);
                    }
                    self.expr(arm.body);
                    self.scopes.pop();
                }
            }
            Statement::For { pattern, iterable, body, location, .. } => {
                self.expr(iterable);
                self.scopes.push(Vec::new());
                // ADR-0002 (owner decision): a loop variable that is assigned or changed through is declared `mut`
                // (`for mut x in ..`, `for (k, mut v) in ..`), over a range or a collection alike. The iteration's
                // borrow (iter / iter_mut / into_iter) stays inferred; `mut` is the visible sign the loop writes.
                let elem = types::type_of(self.facts, self, iterable).and_then(|t| element_type(&t));
                self.bind_pattern(pattern, elem, Kind::For, false, location);
                if let Some(l) = location {
                    let loop_scope = self.scopes.last().cloned().unwrap_or_default();
                    for id in loop_scope {
                        if self.loop_mut.contains(&(l.line, l.column, self.bindings[id].name.clone())) {
                            self.bindings[id].declared_mut = true;
                        }
                    }
                }
                self.block(body);
                self.scopes.pop();
            }
            Statement::While { condition, body, .. } => {
                self.expr(condition);
                self.block(body);
            }
            Statement::Loop { body, .. } => self.block(body),
            Statement::Defer { statement, .. } => self.stmt(statement),
            Statement::Continue { .. } | Statement::Use { .. } => {}
        }
    }

    /// The index expressions inside a place are reads.
    fn place_reads(&mut self, e: &Expression<'_>) {
        match e {
            Expression::FieldAccess { object, .. } => self.place_reads(object),
            Expression::Index { object, index, .. } => {
                self.place_reads(object);
                self.expr(index);
            }
            Expression::Identifier { .. } => {}
            other => self.expr(other),
        }
    }

    /// Whether every step of a place below its root holds a value (not a reference): `x.a.b` changes `x` only when
    /// `x` and `x.a` are values the declarations decide.
    fn place_of_value(&self, e: &Expression<'_>) -> bool {
        match e {
            Expression::Identifier { .. } => true,
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => {
                types::type_of(self.facts, self, object).is_some_and(|t| !is_reference(&t)) && self.place_of_value(object)
            }
            _ => false,
        }
    }

    fn assigned(&mut self, target: &Expression<'_>, compound: bool) {
        self.place_reads(target);
        match target {
            Expression::Identifier { name, location } => self.change(name, Change::Assign { compound }, location, false),
            Expression::FieldAccess { .. } | Expression::Index { .. } => {
                if let Some((root, loc)) = place_root(target) {
                    // a loop variable is changed by an assignment through it whatever its element type
                    let loop_var = self.lookup(root).is_some_and(|i| self.bindings[i].kind == Kind::For);
                    if loop_var || self.place_of_value(target) {
                        let change = if matches!(target, Expression::Index { .. }) { Change::Element } else { Change::Field };
                        self.change(root, change, loc, true);
                    }
                }
            }
            _ => {}
        }
    }

    /// `place` is changed through (a receiver or an argument the callee changes).
    fn changed_through(&mut self, place: &Expression<'_>, change: Change) {
        let place = match place {
            Expression::Unary { op: UnaryOp::MutRef, operand, .. } => operand,
            other => other,
        };
        if let Some((root, loc)) = place_root(place) {
            if self.place_of_value(place) && types::type_of(self.facts, self, place).is_some_and(|t| !is_reference(&t)) {
                self.change(root, change, loc, true);
            }
        }
    }

    /// Record the first change of the binding `name` resolves to. `through` changes (fields, elements, receivers,
    /// arguments) count only for a `let` binding whose declared or inferred type is a value.
    fn change(&mut self, name: &str, change: Change, at: &SourceLocation, through: bool) {
        let Some(id) = self.lookup(name) else { return };
        let b = &self.bindings[id];
        if b.declared_mut || self.reported[id] || b.kind == Kind::Other {
            return;
        }
        let judged_through = match b.kind {
            Kind::Let => b.ty.as_ref().is_some_and(|t| !is_reference(t)),
            Kind::For => true,
            Kind::Other => false,
        };
        if through && !judged_through {
            return;
        }
        let at = match (at, self.macro_locs.last()) {
            (Some(l), Some(m)) if l.file.as_os_str().is_empty() => m.clone(),
            _ => at.clone(),
        };
        self.reported[id] = true;
        self.found.push(Found { binding: b.clone(), change, at, in_closure: self.closure_depth > 0 });
    }

    fn args_lent(&mut self, lends: &[bool], callee: &str, arguments: &[(Option<String>, &Expression<'_>)]) {
        for (i, (_, a)) in arguments.iter().enumerate() {
            if lends.get(i).copied().unwrap_or(false) {
                self.changed_through(a, Change::Argument(callee.to_string()));
            }
        }
    }

    fn expr(&mut self, e: &Expression<'_>) {
        match e {
            Expression::MethodCall { object, method, arguments, .. } => {
                self.method_call(object, method, arguments);
                self.receiver_reads(object);
                for (_, a) in arguments {
                    self.expr(a);
                }
            }
            Expression::Call { function, arguments, .. } => {
                if let Expression::Identifier { name, .. } = &**function {
                    if self.lookup(name).is_none() {
                        let (owner, f) = types::path_call(self.facts, name);
                        let callee = match owner {
                            Some(o) => self.facts.method(o, f).filter(|c| !c.has_self),
                            None => self.facts.function(name),
                        };
                        if let Some(c) = callee {
                            let lends = c.lends_param.clone();
                            self.args_lent(&lends, f, arguments);
                        }
                    }
                } else {
                    self.expr(function);
                }
                for (_, a) in arguments {
                    self.expr(a);
                }
            }
            Expression::Unary { op, operand, .. } => {
                if *op == UnaryOp::MutRef {
                    self.changed_through(e, Change::LendMut);
                }
                self.expr(operand);
            }
            Expression::Closure { parameters, body, .. } => {
                self.scopes.push(Vec::new());
                for p in parameters {
                    // a parameter may be a pattern such as `(k, v)`: bind every name in it
                    for w in p.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty()) {
                        self.bind_other(w);
                    }
                }
                self.closure_depth += 1;
                self.expr(body);
                self.closure_depth -= 1;
                self.scopes.pop();
            }
            Expression::Block { statements, .. } => self.block(statements),
            Expression::MacroInvocation { args, location, .. } => {
                self.macro_locs.push(location.clone());
                for a in args {
                    self.expr(a);
                }
                self.macro_locs.pop();
            }
            Expression::Binary { left, right, .. } => {
                self.expr(left);
                self.expr(right);
            }
            Expression::FieldAccess { object, .. } | Expression::Cast { expr: object, .. } | Expression::TryOp { expr: object, .. } | Expression::Await { expr: object, .. } | Expression::AsyncCall { expr: object, .. } | Expression::SpawnCall { expr: object, .. } | Expression::Scope { body: object, .. } => self.expr(object),
            Expression::Index { object, index, .. } => {
                self.expr(object);
                self.expr(index);
            }
            Expression::Range { start, end, .. } => {
                self.expr(start);
                self.expr(end);
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, v) in fields {
                    self.expr(v);
                }
            }
            Expression::MapLiteral { pairs, .. } => {
                for (k, v) in pairs {
                    self.expr(k);
                    self.expr(v);
                }
            }
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
                for x in elements {
                    self.expr(x);
                }
            }
            Expression::Identifier { .. } | Expression::Literal { .. } => {}
        }
    }

    /// Reads inside a receiver chain (`a[i].f`, `make().g`), without counting the chain itself again.
    fn receiver_reads(&mut self, object: &Expression<'_>) {
        match object {
            Expression::Identifier { .. } => {}
            Expression::FieldAccess { object, .. } => self.receiver_reads(object),
            Expression::Index { object, index, .. } => {
                self.receiver_reads(object);
                self.expr(index);
            }
            other => self.expr(other),
        }
    }

    fn method_call(&mut self, object: &Expression<'_>, method: &str, arguments: &[(Option<String>, &Expression<'_>)]) {
        // `Type::f(..)` written as a call on the type name
        if let Expression::Identifier { name, .. } = object {
            if self.lookup(name).is_none() && name != "self" {
                if let Some(c) = self.facts.method(name, method).filter(|c| !c.has_self) {
                    let lends = c.lends_param.clone();
                    self.args_lent(&lends, method, arguments);
                }
                return;
            }
        }
        let Some(recv) = types::type_of(self.facts, self, object) else { return };
        if let Some(owner) = user_type(self.facts, &recv) {
            let Some(c) = self.facts.method(owner, method).cloned() else { return };
            if c.changes_receiver && !is_reference(&recv) {
                self.changed_through(object, Change::Method(method.to_string()));
            }
            self.args_lent(&c.lends_param, method, arguments);
            return;
        }
        let changes = self.facts.core.as_ref().and_then(|core| core.changes_receiver(&recv, method));
        if changes == Some(true) && !is_reference(&recv) {
            self.changed_through(object, Change::Method(method.to_string()));
        }
    }
}

/// The type of one item of a loop over a value of type `t`: a sequence's element, a map's `(key, value)`.
fn element_type(t: &Type) -> Option<Type> {
    match strip_refs(t) {
        Type::Vec(e) | Type::Array(e, _) => Some((**e).clone()),
        Type::Parameterized(n, args) => {
            let leaf = crate::type_classification::type_name_leaf(n);
            if crate::typing::core_surface::is_map_container(leaf) && args.len() == 2 {
                Some(Type::Tuple(args.clone()))
            } else if crate::typing::core_surface::is_element_container(leaf) && args.len() == 1 {
                Some(args[0].clone())
            } else {
                None
            }
        }
        _ => None,
    }
}
