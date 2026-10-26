//! Phase 3: walk every body, track local bindings, and resolve each call expression.

use super::collect::{Body, BodyCode, Collected};
use super::def::DefKind;
use super::paths::Ctx;
use super::res::{CallKind, CallResolution, ExprKey, FnRef, Reason, Res};
use crate::parser::ast::core::{EnumPatternBinding, Expression, MatchArm, Pattern, Statement};
use std::collections::HashSet;

/// Resolve every call in every collected body.
pub fn resolve_bodies(c: &Collected<'_>) -> (Vec<CallResolution>, Vec<FnRef>) {
    let mut out = Vec::new();
    let mut refs = Vec::new();
    for body in &c.bodies {
        let generics: Vec<String> = body.generics.clone();
        let ctx = Ctx {
            module: body.module,
            impl_idx: body.impl_idx,
            trait_def: body.trait_def,
            generics: &generics,
        };
        let mut w = Walker {
            c,
            ctx,
            body,
            locals: vec![HashSet::new()],
            out: &mut out,
            refs: &mut refs,
        };
        w.run();
    }
    (out, refs)
}

struct Walker<'w, 'c, 'a> {
    c: &'c Collected<'a>,
    ctx: Ctx<'w>,
    body: &'w Body<'a>,
    locals: Vec<HashSet<String>>,
    out: &'w mut Vec<CallResolution>,
    refs: &'w mut Vec<FnRef>,
}

impl<'w, 'c, 'a> Walker<'w, 'c, 'a> {
    fn run(&mut self) {
        for p in self.body.params {
            self.bind(&p.name);
            if let Some(pat) = &p.pattern {
                self.bind_pattern(pat);
            }
        }
        match &self.body.code {
            BodyCode::Stmts(stmts) => self.block(stmts),
            BodyCode::Expr(e) => self.expr(e),
        }
    }

    fn bind(&mut self, name: &str) {
        if let Some(top) = self.locals.last_mut() {
            top.insert(name.to_string());
        }
    }

    fn is_local(&self, name: &str) -> bool {
        self.locals.iter().any(|s| s.contains(name))
    }

    fn scoped<F: FnOnce(&mut Self)>(&mut self, f: F) {
        self.locals.push(HashSet::new());
        f(self);
        self.locals.pop();
    }

    fn block(&mut self, stmts: &[&Statement<'_>]) {
        self.scoped(|w| {
            for s in stmts {
                w.stmt(s);
            }
        });
    }

    fn stmt(&mut self, s: &Statement<'_>) {
        match s {
            Statement::Let {
                pattern,
                value,
                else_block,
                ..
            } => {
                self.expr(value);
                if let Some(eb) = else_block {
                    self.block(eb);
                }
                self.bind_pattern(pattern);
            }
            Statement::Const { name, value, .. } | Statement::Static { name, value, .. } => {
                self.expr(value);
                self.bind(name);
            }
            Statement::Assignment { target, value, .. } => {
                self.expr(target);
                self.expr(value);
            }
            Statement::Return { value, .. } => {
                if let Some(v) = value {
                    self.expr(v);
                }
            }
            Statement::Expression { expr, .. } => self.expr(expr),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr(condition);
                self.block(then_block);
                if let Some(eb) = else_block {
                    self.block(eb);
                }
            }
            Statement::Match { value, arms, .. } => {
                self.expr(value);
                for arm in arms {
                    self.arm(arm);
                }
            }
            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                self.expr(iterable);
                self.scoped(|w| {
                    w.bind_pattern(pattern);
                    w.block(body);
                });
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr(condition);
                self.block(body);
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => self.block(body),
            Statement::Defer { statement, .. } => self.stmt(statement),
            Statement::Use { path, alias, .. } => self.bind_body_use(path, alias.as_deref()),
            Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    /// `use a::b::f` / `use a::b::{f, g}` / `use a::f as h` inside a body: the names are in scope for the rest of the block.
    fn bind_body_use(&mut self, path: &[String], alias: Option<&str>) {
        let joined = path.join("::");
        if let Some((_, names)) = joined.split_once("::{") {
            for n in names.trim_end_matches('}').split(',').map(str::trim).filter(|n| !n.is_empty()) {
                let local = n.rsplit_once(" as ").map_or(n, |(_, a)| a);
                self.bind(local);
            }
        } else if let Some(last) = alias.or_else(|| joined.rsplit("::").next()) {
            if last != "*" {
                self.bind(last);
            }
        }
    }

    fn arm(&mut self, arm: &MatchArm<'_>) {
        self.scoped(|w| {
            w.bind_pattern(&arm.pattern);
            if let Some(g) = arm.guard {
                w.expr(g);
            }
            w.expr(arm.body);
        });
    }

    fn bind_pattern(&mut self, p: &Pattern<'_>) {
        match p {
            Pattern::Identifier(n)
            | Pattern::Ref(n)
            | Pattern::RefMut(n)
            | Pattern::MutBinding(n) => self.bind(n),
            Pattern::Tuple(ps) | Pattern::Or(ps) => ps.iter().for_each(|q| self.bind_pattern(q)),
            Pattern::Reference(q) => self.bind_pattern(q),
            Pattern::EnumVariant(_, b) => self.bind_enum_binding(b),
            Pattern::Wildcard | Pattern::Literal(_) | Pattern::Range { .. } => {}
        }
    }

    fn bind_enum_binding(&mut self, b: &EnumPatternBinding<'_>) {
        match b {
            EnumPatternBinding::Single(n) => self.bind(n),
            EnumPatternBinding::Tuple(ps) => ps.iter().for_each(|q| self.bind_pattern(q)),
            EnumPatternBinding::Struct(fields, _) => {
                fields.iter().for_each(|(_, q)| self.bind_pattern(q))
            }
            EnumPatternBinding::None | EnumPatternBinding::Wildcard => {}
        }
    }

    fn expr(&mut self, e: &Expression<'_>) {
        match e {
            Expression::Call {
                function,
                arguments,
                location,
            } => {
                self.call(e, function, arguments.len(), location);
                arguments.iter().for_each(|(_, a)| self.expr(a));
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                location,
                ..
            } => {
                self.expr(object);
                self.method_call(e, method, arguments.len(), location);
                arguments.iter().for_each(|(_, a)| self.expr(a));
            }
            Expression::Closure {
                parameters, body, ..
            } => self.scoped(|w| {
                parameters.iter().for_each(|p| w.bind(p));
                w.expr(body);
            }),
            Expression::Block { statements, .. } => self.block(statements),
            Expression::Identifier { name, .. } => self.fn_ref(e, name),
            _ => self.expr_children(e),
        }
    }

    fn expr_children(&mut self, e: &Expression<'_>) {
        match e {
            Expression::Binary { left, right, .. } => {
                self.expr(left);
                self.expr(right);
            }
            Expression::Unary { operand, .. } => self.expr(operand),
            Expression::FieldAccess { object, .. } => self.expr(object),
            Expression::StructLiteral { fields, .. } => {
                fields.iter().for_each(|(_, v)| self.expr(v))
            }
            Expression::MapLiteral { pairs, .. } => pairs.iter().for_each(|(k, v)| {
                self.expr(k);
                self.expr(v);
            }),
            Expression::Range { start, end, .. } => {
                self.expr(start);
                self.expr(end);
            }
            Expression::Cast { expr, .. }
            | Expression::TryOp { expr, .. }
            | Expression::Await { expr, .. }
            | Expression::AsyncCall { expr, .. }
            | Expression::SpawnCall { expr, .. } => self.expr(expr),
            Expression::Index { object, index, .. } => {
                self.expr(object);
                self.expr(index);
            }
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
                elements.iter().for_each(|x| self.expr(x))
            }
            Expression::MacroInvocation { args, .. } => args.iter().for_each(|x| self.expr(x)),
            Expression::ChannelSend { channel, value, .. } => {
                self.expr(channel);
                self.expr(value);
            }
            Expression::ChannelRecv { channel, .. } => self.expr(channel),
            _ => {}
        }
    }

    /// A free function or `mod::f` used as a value.
    fn fn_ref(&mut self, e: &Expression<'_>, name: &str) {
        if self.is_local(name) {
            return;
        }
        let res = if name.contains("::") {
            let segs: Vec<&str> = name.split("::").collect();
            self.c.resolve_path(&self.ctx, &segs)
        } else {
            self.c.resolve_free(&self.ctx, name)
        };
        if let Res::Def(id) = res {
            if matches!(self.c.defs.get(id).kind, DefKind::Fn | DefKind::ExternFn) {
                self.refs.push(FnRef { enclosing: self.body.owner, def: id, location: e.location() });
            }
        }
    }

    fn push(&mut self, e: &Expression<'_>, kind: CallKind, callee: String, argc: usize, res: Res) {
        self.out.push(CallResolution {
            key: ExprKey(e as *const Expression<'_> as usize),
            kind,
            callee,
            arg_count: argc,
            enclosing: self.body.owner,
            module: self.body.module,
            location: e.location(),
            res,
            str_args: literal_string_args(e),
            ident_args: ident_args(e),
        });
    }

    fn call(
        &mut self,
        call: &Expression<'_>,
        callee: &Expression<'_>,
        argc: usize,
        _loc: &Option<crate::source_map::Location>,
    ) {
        let Expression::Identifier { name, .. } = callee else {
            self.expr(callee);
            self.push(
                call,
                CallKind::Dynamic,
                "<expr>".into(),
                argc,
                Res::Unresolved(Reason::Dynamic),
            );
            return;
        };
        if !name.contains("::") {
            let res = if self.is_local(name) {
                Res::Local(name.clone())
            } else {
                self.c.resolve_free(&self.ctx, name)
            };
            self.push(call, CallKind::Free, name.clone(), argc, res);
            return;
        }
        let segs: Vec<&str> = name.split("::").collect();
        let res = self.c.resolve_path(&self.ctx, &segs);
        self.push(call, CallKind::Path, name.clone(), argc, res);
    }

    fn method_call(
        &mut self,
        call: &Expression<'_>,
        method: &str,
        argc: usize,
        _loc: &Option<crate::source_map::Location>,
    ) {
        let cands: Vec<_> = self
            .c
            .methods_by_name
            .get(method)
            .into_iter()
            .flatten()
            .copied()
            .filter(|d| {
                let def = self.c.defs.get(*d);
                def.has_self && def.arity == argc && !matches!(def.kind, DefKind::AssocFn { .. })
            })
            .collect();
        let res = if cands.is_empty() {
            Res::Unresolved(Reason::NoMethodCandidate(method.to_string()))
        } else {
            Res::MethodCandidates(cands)
        };
        self.push(call, CallKind::Method, method.to_string(), argc, res);
    }
}

/// Plain string-literal arguments of a call expression (`None` for anything computed or interpolated).
fn literal_string_args(e: &Expression<'_>) -> Vec<Option<String>> {
    use crate::parser::ast::literals::Literal;
    let args = match e {
        Expression::Call { arguments, .. } | Expression::MethodCall { arguments, .. } => arguments,
        _ => return Vec::new(),
    };
    args.iter()
        .map(|(_, a)| match a {
            Expression::Literal { value: Literal::String(s), .. } if !s.contains("${") => Some(s.clone()),
            _ => None,
        })
        .collect()
}

/// Plain identifier arguments of a call expression.
fn ident_args(e: &Expression<'_>) -> Vec<Option<String>> {
    let args = match e {
        Expression::Call { arguments, .. } | Expression::MethodCall { arguments, .. } => arguments,
        _ => return Vec::new(),
    };
    args.iter()
        .map(|(_, a)| match a {
            Expression::Identifier { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}
