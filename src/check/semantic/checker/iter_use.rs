//! WJ0035: a lazy iterator used where only a list can go (ADR-0039 decision H, gate G8).
//!
//! One code, one primary span and one machine-applicable fix for every use kind, reported from the shared front end so
//! `wj check`, `wj run --interpret` and `wj build` print the same thing. The fix is `.collect()` (on the `let` when the
//! iterator is a plain binding, otherwise right before the use), except `.len()` becomes `.count()` and `.first()`
//! becomes `.next()` when the binding is used once.

use super::*;
use crate::parser::BinaryOp;

/// Methods that borrow an iterator instead of consuming it.
const BORROWING: [&str; 5] = ["next", "peek", "next_back", "nth", "size_hint"];

/// How the iterator reached a place that needs a list.
pub(super) enum Use<'u> {
    Index,
    Method(&'u str),
    DeclaredList,
    Argument(&'u str),
    Returned,
    Field(&'u str),
    Compared,
    Debug,
    Twice,
}

impl Use<'_> {
    fn what(&self) -> String {
        match self {
            Use::Index => "it is indexed or sliced, and only a list has positions".to_string(),
            Use::Method(m) => format!("`{m}` is a list method"),
            Use::DeclaredList => "it is stored where a list is declared".to_string(),
            Use::Argument(f) => format!("`{f}` takes a list here"),
            Use::Returned => "the function returns a list".to_string(),
            Use::Field(f) => format!("the field `{f}` is a list"),
            Use::Compared => "it is compared with a list".to_string(),
            Use::Debug => "it is printed with `:?`, which would show the iterator and not its items".to_string(),
            Use::Twice => "it was already consumed by an earlier use".to_string(),
        }
    }
}

impl Checker<'_, '_, '_> {
    /// The iterator root of `e` (the producer method), when `e` is an iterator by declaration.
    pub(super) fn iterator_of(&self, e: &Expression<'_>) -> Option<String> {
        let recv = |e: &Expression<'_>| iterators::receiver_type(e, &self.surface_of).or_else(|| matches!(e, Expression::Identifier { name, .. } if self.scalars.get(name).is_some_and(|k| k.0 == Kind::Str)).then_some("String"));
        iterators::iterator_root(e, &recv, &self.iter_locals)
    }

    /// Report `it` (an iterator expression) reaching a list-only place. `method` is the span of the list method for
    /// `Use::Method`.
    pub(super) fn iterator_as_list(&mut self, it: &Expression<'_>, how: Use<'_>, loc: &SourceLocation) {
        let loc = self.effective(loc);
        let mut d = Diagnostic::error("WJ0035", format!("an iterator is used as a list: {}", how.what()));
        let method_span = match how {
            Use::Method(m) => self.cx.loc.word(&loc, m).or_else(|| self.cx.loc.first_use(m)),
            _ => None,
        };
        let primary = match (&how, it) {
            (Use::Method(_), _) => method_span.clone(),
            (_, Expression::Identifier { name, location }) => {
                let l = self.effective(location);
                self.cx.loc.word(&l, name).or_else(|| self.cx.loc.first_use(name))
            }
            (_, Expression::MethodCall { method, location, .. }) => {
                let l = self.effective(location);
                self.cx.loc.word(&l, method)
            }
            _ => None,
        };
        if let Some(sp) = primary {
            d = d.at(sp.with_label("an iterator"));
        }
        let (help, fix) = self.iterator_fix(it, &how, method_span);
        d = d.with_help(help);
        if let Some(f) = fix {
            d = d.with_fix(f);
        }
        self.cx.out.push(d);
    }

    fn iterator_fix(&mut self, it: &Expression<'_>, how: &Use<'_>, method_span: Option<Span>) -> (String, Option<Fix>) {
        let generic = "`split`, `lines`, `chars` and the adapters are lazy; `.collect()` makes the list (`.count()` is the size of an iterator, `.next()` its first item)";
        match it {
            Expression::Identifier { name, .. } => {
                let single = self.name_uses.get(name) == Some(&1);
                if let (Use::Method(m @ ("len" | "first")), true, Some(sp)) = (how, single, &method_span) {
                    let to = if *m == "len" { "count" } else { "next" };
                    let mut fix = machine(replace_fix(sp, to, format!("call `.{to}()`: an iterator has no `.{m}()`")));
                    // `.next()` borrows the iterator mutably: the binding must be `let mut` (WJ0051 otherwise)
                    let declared = if *m == "first" { self.binding_mut_edit(name) } else { Some(None) };
                    if let Some(maybe_edit) = declared {
                        fix.edits.extend(maybe_edit);
                        return (format!("write `.{to}()` instead of `.{m}()` on an iterator"), Some(fix));
                    }
                }
                if !self.collected.insert(name.clone()) {
                    return (format!("`{name}` is fixed by the `.collect()` suggested at its first report"), None);
                }
                (format!("{generic}: add `.collect()` to the `let {name}`"), self.collect_fix_for_binding(name))
            }
            Expression::MethodCall { method, location, .. } => {
                if let (Use::Method(m @ ("len" | "first")), Some(sp)) = (how, &method_span) {
                    let to = if *m == "len" { "count" } else { "next" };
                    let fix = machine(replace_fix(sp, to, format!("call `.{to}()`: an iterator has no `.{m}()`")));
                    return (format!("write `.{to}()` instead of `.{m}()` on an iterator"), Some(fix));
                }
                let text = self.cx.loc.ix.text();
                let at = match (how, &method_span) {
                    (Use::Method(_), Some(sp)) if sp.byte_start > 0 && text.as_bytes()[sp.byte_start - 1] == b'.' => Some(sp.byte_start - 1),
                    (Use::Method(_), _) => None,
                    _ => {
                        let l = self.effective(location);
                        self.cx.loc.word(&l, method).and_then(|sp| chain_end(text, sp.byte_end))
                    }
                };
                let fix = at.map(|p| {
                    let sp = self.cx.loc.ix.span(p, p);
                    machine(replace_fix(&sp, ".collect()", "call `.collect()` to make the iterator a list".to_string()))
                });
                (format!("{generic}: add `.collect()` right after the iterator"), fix)
            }
            _ => (generic.to_string(), None),
        }
    }

    /// `it[a..b]` / `it[a..]` / `it[..b]` on an iterator: the parser desugars the slice into a synthetic `it.slice(a, b)`
    /// (the omitted end into a synthetic `it.len()`), neither of which has a surface form. When the text after the
    /// receiver is `[`, this is that sugar: report it once, as the index use it is, at the receiver, and skip the
    /// synthetic `len`. A written `it.slice(..)` is a list method and takes the ordinary route.
    pub(super) fn slice_sugar_of_iterator(&mut self, object: &Expression<'_>, arguments: &[(Option<String>, &Expression<'_>)], loc: &SourceLocation) -> bool {
        let Some(root) = self.iterator_of(object) else { return false };
        if !iterators::lists_only("slice", &root) {
            return false;
        }
        let text = self.cx.loc.ix.text();
        let end = match object {
            Expression::Identifier { name, location } => {
                let l = self.effective(location);
                self.cx.loc.word(&l, name).map(|sp| sp.byte_end)
            }
            Expression::MethodCall { method, location, .. } => {
                let l = self.effective(location);
                self.cx.loc.word(&l, method).and_then(|sp| chain_end(text, sp.byte_end))
            }
            _ => None,
        };
        if end.is_none_or(|at| text.as_bytes().get(at) != Some(&b'[')) {
            return false;
        }
        if let Some((_, Expression::MethodCall { method, arguments: a, .. })) = arguments.get(1) {
            if method == "len" && a.is_empty() {
                self.skip.insert(arguments[1].1 as *const Expression<'_> as usize);
            }
        }
        self.iterator_as_list(object, Use::Index, loc);
        true
    }

    /// `parts[0]`, `parts[1..]`, `parts.contains(..)` and the other list uses that need only the object.
    pub(super) fn list_use_in_expr(&mut self, e: &Expression<'_>) {
        match e {
            Expression::Index { object, location, .. } if self.iterator_of(object).is_some() => {
                self.iterator_as_list(object, Use::Index, location);
            }
            Expression::Binary { left, op: BinaryOp::Eq | BinaryOp::Ne, right, location } => {
                let is_list = |e: &Expression<'_>| matches!(e, Expression::Array { .. }) || matches!(e, Expression::MacroInvocation { name, .. } if name == "vec");
                for (side, other) in [(left, right), (right, left)] {
                    if is_list(other) && self.iterator_of(side).is_some() {
                        self.iterator_as_list(side, Use::Compared, location);
                        break;
                    }
                }
            }
            Expression::Call { function, arguments, location } => {
                let Expression::Identifier { name, .. } = &**function else { return };
                if name.contains("::") || self.locals.contains(name) {
                    return;
                }
                let idx: &Index = self.cx.idx;
                let Some([info]) = idx.fns.get(name).map(Vec::as_slice) else { return };
                if info.generic || info.params.len() != arguments.len() || arguments.iter().any(|(l, _)| l.is_some()) {
                    return;
                }
                for ((_, arg), ty) in arguments.iter().zip(&info.params) {
                    if crate::type_classification::type_is_vec_container(ty) && self.iterator_of(arg).is_some() {
                        self.iterator_as_list(arg, Use::Argument(name), location);
                    }
                }
            }
            Expression::StructLiteral { name, fields, location } => {
                let Some(declared) = self.known_struct_fields(name) else { return };
                for (field, value) in fields {
                    let is_list = declared.iter().any(|(f, t)| f == field && crate::type_classification::type_is_vec_container(t));
                    if is_list && self.iterator_of(value).is_some() {
                        self.iterator_as_list(value, Use::Field(field), location);
                    }
                }
            }
            _ => {}
        }
    }

    /// `${it:?}`: a `{:?}` placeholder given an iterator.
    pub(super) fn debug_of_iterator(&mut self, args: &[&Expression<'_>]) {
        let Some(Expression::Literal { value: Literal::String(fmt), .. }) = args.first().map(|a| &**a) else { return };
        for (spec, arg) in rules::placeholder_specs(fmt).into_iter().zip(args.iter().skip(1)) {
            if spec.ends_with('?') && self.iterator_of(arg).is_some() {
                self.iterator_as_list(arg, Use::Debug, &arg.location());
            }
        }
    }

    /// A value that the function declares as its list result.
    pub(super) fn returned_iterator(&mut self, value: &Expression<'_>, location: &SourceLocation) {
        let Some(ty) = &self.ret_ty else { return };
        if crate::type_classification::type_is_vec_container(ty) && self.iterator_of(value).is_some() {
            self.iterator_as_list(value, Use::Returned, location);
        }
    }

    /// `let parts: Vec<string> = line.split(" ")`: a list is declared, an iterator is given.
    pub(super) fn iterator_for_declared_list(&mut self, name: &str, declared: &Type, value: &Expression<'_>, location: &SourceLocation) {
        if !crate::type_classification::type_is_vec_container(declared) || self.iterator_of(value).is_none() {
            return;
        }
        let loc = self.effective(location);
        let mut d = Diagnostic::error("WJ0035", format!("an iterator is used as a list: {}", Use::DeclaredList.what()));
        if let Some(sp) = self.cx.loc.word(&loc, name) {
            d = d.at(sp.with_label("declared a list, given an iterator"));
        }
        d = d.with_help(format!("add `.collect()`: `let {name}: Vec<..> = ...collect()` turns the iterator into the list the type declares"));
        if let Some(fix) = self.collect_fix_for_binding(name) {
            d = d.with_fix(fix);
        }
        self.cx.out.push(d);
    }
}

fn machine(mut f: Fix) -> Fix {
    f.applicability = Applicability::MachineApplicable;
    f
}

/// Byte offset just after the `(..)` that follows a method name ending at `word_end`.
fn chain_end(text: &str, word_end: usize) -> Option<usize> {
    let b = text.as_bytes();
    if b.get(word_end) != Some(&b'(') {
        return None;
    }
    let (mut depth, mut i) = (0usize, word_end);
    while i < b.len() {
        match b[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
            }
            b'\n' => return None,
            _ => {}
        }
        i += 1;
    }
    None
}

/// Every mention of `name` that consumes it, and whether a closure, block or short-circuit hides a mention.
struct Mentions<'n> {
    name: &'n str,
    consuming: Vec<SourceLocation>,
    conditional: bool,
    skip: HashSet<usize>,
}

impl Visitor for Mentions<'_> {
    fn expr(&mut self, e: &Expression<'_>) {
        match e {
            Expression::MethodCall { object, method, .. } if BORROWING.contains(&method.as_str()) => {
                if matches!(&**object, Expression::Identifier { name, .. } if name == self.name) {
                    self.skip.insert(*object as *const Expression<'_> as usize);
                }
            }
            Expression::Identifier { name, location } if name == self.name && !self.skip.contains(&(e as *const Expression<'_> as usize)) => {
                self.consuming.push(location.clone());
            }
            Expression::Closure { .. } | Expression::Block { .. } | Expression::Binary { op: BinaryOp::And | BinaryOp::Or, .. } => self.conditional = true,
            _ => {}
        }
    }
}

fn mentions<'n>(s: &Statement<'_>, name: &'n str) -> Mentions<'n> {
    let mut m = Mentions { name, consuming: Vec::new(), conditional: false, skip: HashSet::new() };
    walk_stmt(&mut m, s);
    m
}

/// Each `(binding, location of its second consuming use)` in a function body: the first use consumed the iterator
/// on every path (a straight-line statement), a later statement uses it again.
pub(super) fn reuse_after_consumption(body: &[&Statement<'_>], iter_locals: &iterators::IterLocals) -> Vec<(String, SourceLocation)> {
    let mut out = Vec::new();
    scan(body, iter_locals, &mut out);
    out
}

fn scan(stmts: &[&Statement<'_>], iter_locals: &iterators::IterLocals, out: &mut Vec<(String, SourceLocation)>) {
    for (i, s) in stmts.iter().enumerate() {
        match s {
            Statement::If { then_block, else_block, .. } => {
                scan(then_block, iter_locals, out);
                if let Some(b) = else_block {
                    scan(b, iter_locals, out);
                }
            }
            Statement::For { body, .. } | Statement::While { body, .. } | Statement::Loop { body, .. } => scan(body, iter_locals, out),
            _ => {}
        }
        if let Statement::Let { pattern: Pattern::Identifier(n) | Pattern::MutBinding(n), .. } = s {
            if iter_locals.contains_key(n) {
                reuse_in(&stmts[i + 1..], n, out);
            }
        }
    }
}

fn reuse_in(rest: &[&Statement<'_>], name: &str, out: &mut Vec<(String, SourceLocation)>) {
    let mut consumed = false;
    for s in rest {
        let m = mentions(s, name);
        if m.consuming.is_empty() {
            continue;
        }
        if consumed {
            out.push((name.to_string(), m.consuming[0].clone()));
            return;
        }
        let straight = matches!(s, Statement::Let { .. } | Statement::Expression { .. } | Statement::Assignment { .. }) && !m.conditional;
        let loops_over_it = matches!(s, Statement::For { iterable: Expression::Identifier { name: n, .. }, .. } if n == name) && m.consuming.len() == 1;
        if !(straight || loops_over_it) {
            return;
        }
        consumed = true;
    }
}
