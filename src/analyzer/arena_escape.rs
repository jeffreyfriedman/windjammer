//! Arena escape check (ADR-0025, `std::arena`, unstable).
//!
//! A value allocated in an arena may not outlive the arena. Rust's borrow checker enforces this for
//! `Scratch` values in the emitted code, but its error talks about lifetimes in generated Rust; `Arena<T>`
//! handles are plain indices, so nothing would stop them from escaping at all (a stale handle is only caught
//! at run time, in debug builds). This pass reports both, in Windjammer terms, before code generation.
//!
//! Model (per function body, no annotations):
//! * An *arena* is a local `let a = Arena::new()` / `Scratch::new()` (or `with_capacity`). Its scope is the
//!   block that declares it. Arenas reached through parameters or `self` fields are *external*: values from
//!   them may leave the function (the caller owns the arena), but use after `reset()` is still checked.
//! * A value is *arena-bound* to `a` when it is `a.alloc(..)`, `a.str(..)`, `a.new_string()`, `a.new_list()`, a copy of
//!   an arena-bound variable, a struct/tuple/list literal containing one, or a block whose tail is one.
//! * Escapes (errors): returning a value bound to a local arena (`return`, implicit final expression); assigning
//!   it to a variable, field or index place rooted outside the arena's scope; storing it into a collection
//!   rooted outside the arena's scope (`push`, `insert`, `push_back`, `push_front`, `extend`, `add`, `set`);
//!   a block value that leaves the block declaring the arena; and using a value after its arena's `reset()`
//!   in the same block.
//!
//! Copying out is always allowed (`s.to_string()`, `list.to_vec()`, `out.push_str(s)`): those produce owned
//! values that are not arena-bound.

use crate::parser::ast::core::{Expression, Statement};
use crate::parser::*;
use std::collections::HashMap;

/// One diagnostic. `line` is 1-based (0 when the parser recorded no location).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArenaEscape {
    pub line: usize,
    pub message: String,
    pub hint: String,
}

impl ArenaEscape {
    pub fn render(&self) -> String {
        format!(
            "error[WJ0160]: {}\n  --> line {}\n  = help: {}",
            self.message, self.line, self.hint
        )
    }
}

const ALLOC_METHODS: &[&str] = &["alloc", "str", "new_string", "new_list"];
const STORE_METHODS: &[&str] = &[
    "push",
    "insert",
    "push_back",
    "push_front",
    "extend",
    "add",
    "set",
    "append",
];

#[derive(Clone)]
struct ArenaInfo {
    depth: usize,
    local: bool,
    line: usize,
    resets: Vec<usize>, // statement serials of `a.reset()`
}

#[derive(Clone)]
struct VarInfo {
    depth: usize,
    /// Arena names this value is bound to, with the statement serial at which it was bound.
    bound: Vec<(String, usize)>,
}

struct Checker {
    arenas: HashMap<String, ArenaInfo>,
    vars: Vec<HashMap<String, VarInfo>>,
    serial: usize,
    errors: Vec<ArenaEscape>,
}

fn line_of(loc: &SourceLocation) -> usize {
    loc.as_ref().map(|l| l.line).unwrap_or(0)
}

fn expr_line(e: &Expression) -> usize {
    match e {
        Expression::Literal { location, .. }
        | Expression::Identifier { location, .. }
        | Expression::Binary { location, .. }
        | Expression::Unary { location, .. }
        | Expression::Call { location, .. }
        | Expression::MethodCall { location, .. }
        | Expression::FieldAccess { location, .. }
        | Expression::StructLiteral { location, .. }
        | Expression::MapLiteral { location, .. }
        | Expression::Range { location, .. }
        | Expression::Closure { location, .. }
        | Expression::Cast { location, .. }
        | Expression::Index { location, .. }
        | Expression::Tuple { location, .. }
        | Expression::Array { location, .. }
        | Expression::MacroInvocation { location, .. }
        | Expression::TryOp { location, .. }
        | Expression::Await { location, .. }
        | Expression::AsyncCall { location, .. }
        | Expression::SpawnCall { location, .. }
        | Expression::ChannelSend { location, .. }
        | Expression::ChannelRecv { location, .. }
        | Expression::Block { location, .. } => line_of(location),
    }
}

/// `Arena::new`, `arena::Scratch::with_capacity`, ... as a `::`-joined path.
fn callee_path(e: &Expression) -> Option<String> {
    match e {
        Expression::Identifier { name, .. } => Some(name.clone()),
        Expression::FieldAccess { object, field, .. } => {
            Some(format!("{}::{}", callee_path(object)?, field))
        }
        _ => None,
    }
}

fn is_arena_ctor(e: &Expression) -> bool {
    let Expression::Call { function, .. } = e else {
        return false;
    };
    let Some(p) = callee_path(function) else {
        return false;
    };
    let segs: Vec<&str> = p.split("::").filter(|s| !s.is_empty()).collect();
    matches!(
        segs.as_slice(),
        [.., "Arena" | "Scratch", "new" | "with_capacity"]
    )
}

/// Root variable of a place (`a`, `a.b`, `a[i].c`, `self.f`).
fn place_root<'a>(e: &'a Expression<'_>) -> Option<&'a str> {
    match e {
        Expression::Identifier { name, .. } => Some(name.as_str()),
        Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => {
            place_root(object)
        }
        Expression::MethodCall { object, method, .. }
            if method == "get_mut" || method == "borrow_mut" =>
        {
            place_root(object)
        }
        _ => None,
    }
}

/// Text key for an arena expression (`frame`, `self.particles`).
fn arena_key(e: &Expression) -> Option<String> {
    match e {
        Expression::Identifier { name, .. } => Some(name.clone()),
        Expression::FieldAccess { object, field, .. } => {
            Some(format!("{}.{}", arena_key(object)?, field))
        }
        _ => None,
    }
}

fn pattern_names(p: &Pattern) -> Vec<String> {
    match p {
        Pattern::Identifier(n) | Pattern::MutBinding(n) | Pattern::Ref(n) | Pattern::RefMut(n) => {
            vec![n.clone()]
        }
        Pattern::Tuple(ps) | Pattern::Or(ps) => ps.iter().flat_map(pattern_names).collect(),
        Pattern::Reference(p) => pattern_names(p),
        _ => Vec::new(),
    }
}

impl Checker {
    fn depth(&self) -> usize {
        self.vars.len()
    }

    fn lookup(&self, name: &str) -> Option<&VarInfo> {
        self.vars.iter().rev().find_map(|s| s.get(name))
    }

    /// Declaration depth of a place root; parameters, `self` and globals are depth 0.
    fn root_depth(&self, root: &str) -> usize {
        if let Some(v) = self.lookup(root) {
            return v.depth;
        }
        self.arenas.get(root).map(|a| a.depth).unwrap_or(0)
    }

    fn declare(&mut self, name: &str, bound: Vec<(String, usize)>) {
        let depth = self.depth();
        self.vars
            .last_mut()
            .expect("scope")
            .insert(name.to_string(), VarInfo { depth, bound });
    }

    fn err(&mut self, line: usize, message: String, hint: String) {
        self.errors.push(ArenaEscape {
            line,
            message,
            hint,
        });
    }

    /// Arenas `e` is bound to (empty: an ordinary value). Also checks nested expressions.
    fn bound_of(&mut self, e: &Expression) -> Vec<(String, usize)> {
        match e {
            Expression::Identifier { name, location } => {
                let Some(v) = self.lookup(name).cloned() else {
                    return Vec::new();
                };
                self.check_use_after_reset(name, &v, line_of(location));
                v.bound
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                location,
                ..
            } => {
                // `try_get` is the checked lookup: a possibly stale handle is its purpose.
                let arg_bounds: Vec<Vec<(String, usize)>> = if method == "try_get" {
                    Vec::new()
                } else {
                    arguments.iter().map(|(_, a)| self.bound_of(a)).collect()
                };
                if ALLOC_METHODS.contains(&method.as_str()) {
                    if let Some(k) = arena_key(object) {
                        if self.arena_for_mut(&k).is_some() {
                            return vec![(k, self.serial)];
                        }
                    }
                }
                if method == "reset" && arguments.is_empty() {
                    if let Some(k) = arena_key(object) {
                        let serial = self.serial;
                        if let Some(a) = self.arena_for_mut(&k) {
                            a.resets.push(serial);
                        }
                    }
                }
                let inner = self.bound_of(object);
                if STORE_METHODS.contains(&method.as_str()) {
                    self.check_store(object, &arg_bounds, line_of(location));
                }
                // Views of an arena value stay in the arena; anything else (len, to_string, to_vec, get) is owned.
                if matches!(
                    method.as_str(),
                    "as_str" | "as_slice" | "iter" | "clone" | "trim"
                ) {
                    inner
                } else {
                    Vec::new()
                }
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                self.bound_of(function);
                for (_, a) in arguments {
                    self.bound_of(a);
                }
                Vec::new()
            }
            Expression::StructLiteral { fields, .. } => {
                fields.iter().flat_map(|(_, f)| self.bound_of(f)).collect()
            }
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
                elements.iter().flat_map(|x| self.bound_of(x)).collect()
            }
            Expression::Block {
                statements,
                location,
                ..
            } => self.block_value(statements, line_of(location)),
            Expression::Binary { left, right, .. } => {
                self.bound_of(left);
                self.bound_of(right);
                Vec::new()
            }
            Expression::Unary { operand, .. } => self.bound_of(operand),
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => {
                self.bound_of(object);
                Vec::new()
            }
            Expression::Cast { expr, .. }
            | Expression::TryOp { expr, .. }
            | Expression::Await { expr, .. } => self.bound_of(expr),
            Expression::MacroInvocation { args, .. } => {
                for a in args {
                    self.bound_of(a);
                }
                Vec::new()
            }
            Expression::Closure { body, .. } => {
                self.bound_of(body);
                Vec::new()
            }
            Expression::MapLiteral { pairs, .. } => pairs
                .iter()
                .flat_map(|(k, v)| [self.bound_of(k), self.bound_of(v)].concat())
                .collect(),
            Expression::Range { start, end, .. } => {
                self.bound_of(start);
                self.bound_of(end);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn arena_for_mut(&mut self, key: &str) -> Option<&mut ArenaInfo> {
        if !self.arenas.contains_key(key)
            && (key.starts_with("self.") || self.lookup(key).is_none())
        {
            // External arena (parameter or field): tracked for reset only.
            self.arenas.insert(
                key.to_string(),
                ArenaInfo {
                    depth: 0,
                    local: false,
                    line: 0,
                    resets: Vec::new(),
                },
            );
        }
        self.arenas.get_mut(key)
    }

    fn check_use_after_reset(&mut self, name: &str, v: &VarInfo, line: usize) {
        for (arena, at) in &v.bound {
            if let Some(a) = self.arenas.get(arena) {
                if let Some(r) = a.resets.iter().find(|r| **r > *at && **r < self.serial) {
                    let _ = r;
                    self.err(
                        line,
                        format!("`{name}` is used after `{arena}.reset()`; it was allocated in `{arena}` before the reset"),
                        format!("allocate `{name}` again after the reset, or copy what you need out of the arena before calling `{arena}.reset()`"),
                    );
                    return;
                }
            }
        }
    }

    fn escape_msg(
        &mut self,
        bound: &[(String, usize)],
        line: usize,
        what: &str,
        outer_depth: Option<usize>,
    ) {
        for (arena, _) in bound {
            let Some(a) = self.arenas.get(arena).cloned() else {
                continue;
            };
            if !a.local {
                continue;
            }
            let escapes = match outer_depth {
                None => true, // return
                Some(d) => d < a.depth,
            };
            if escapes {
                self.err(
                    line,
                    format!("a value allocated in arena `{arena}` {what}, but `{arena}` (declared on line {}) is freed at the end of its scope", a.line),
                    format!("copy the value out (`.to_string()`, `.to_vec()`, or build a regular value), or declare `{arena}` in the caller and pass it in so it lives long enough"),
                );
                return;
            }
        }
    }

    fn check_store(
        &mut self,
        receiver: &Expression,
        arg_bounds: &[Vec<(String, usize)>],
        line: usize,
    ) {
        let Some(root) = place_root(receiver) else {
            return;
        };
        if self.arenas.contains_key(root) {
            return; // storing into the arena itself
        }
        let d = self.root_depth(root);
        let root = root.to_string();
        for b in arg_bounds {
            self.escape_msg(
                b,
                line,
                &format!("is stored into `{root}`, which outlives it"),
                Some(d),
            );
        }
    }

    /// Value of a statement list used as an expression; an arena declared inside cannot be in the result.
    fn block_value(&mut self, stmts: &[&Statement], line: usize) -> Vec<(String, usize)> {
        self.vars.push(HashMap::new());
        let inner_depth = self.depth();
        let mut tail = Vec::new();
        for (i, s) in stmts.iter().enumerate() {
            if i + 1 == stmts.len() {
                if let Statement::Expression { expr, .. } = s {
                    self.serial += 1;
                    tail = self.bound_of(expr);
                    continue;
                }
            }
            self.stmt(s);
        }
        for (arena, _) in tail.clone() {
            if self
                .arenas
                .get(&arena)
                .is_some_and(|a| a.local && a.depth >= inner_depth)
            {
                self.escape_msg(
                    &tail,
                    line,
                    "is the value of the block that declares the arena",
                    Some(inner_depth - 1),
                );
                break;
            }
        }
        self.pop_scope();
        tail
    }

    fn pop_scope(&mut self) {
        let d = self.depth();
        self.arenas.retain(|_, a| !(a.local && a.depth >= d));
        self.vars.pop();
    }

    fn block(&mut self, stmts: &[&Statement]) {
        self.vars.push(HashMap::new());
        for s in stmts {
            self.stmt(s);
        }
        self.pop_scope();
    }

    fn stmt(&mut self, s: &Statement) {
        self.serial += 1;
        match s {
            Statement::Let {
                pattern,
                value,
                else_block,
                location,
                ..
            } => {
                let names = pattern_names(pattern);
                if is_arena_ctor(value) {
                    if let [n] = names.as_slice() {
                        let depth = self.depth();
                        self.arenas.insert(
                            n.clone(),
                            ArenaInfo {
                                depth,
                                local: true,
                                line: line_of(location),
                                resets: Vec::new(),
                            },
                        );
                        return;
                    }
                }
                let b = self.bound_of(value);
                for n in names {
                    self.arenas.remove(&n); // shadowing an arena name
                    self.declare(&n, b.clone());
                }
                if let Some(e) = else_block {
                    self.block(e);
                }
            }
            Statement::Assignment {
                target,
                value,
                location,
                ..
            } => {
                let b = self.bound_of(value);
                if let Some(root) = place_root(target) {
                    let d = self.root_depth(root);
                    let root = root.to_string();
                    if !b.is_empty() && !self.arenas.contains_key(&root) {
                        // Plain rebinding of a variable inside the arena's scope keeps it bound.
                        if let (Expression::Identifier { .. }, Some(v)) = (
                            *target,
                            self.vars.iter_mut().rev().find_map(|s| s.get_mut(&root)),
                        ) {
                            if v.depth >= 1 {
                                v.bound.extend(b.iter().cloned());
                            }
                        }
                        self.escape_msg(
                            &b,
                            line_of(location),
                            &format!("is assigned to `{root}`, which outlives it"),
                            Some(d),
                        );
                    }
                }
            }
            Statement::Return {
                value: Some(v),
                location,
            } => {
                let b = self.bound_of(v);
                self.escape_msg(&b, line_of(location), "is returned from the function", None);
            }
            Statement::Expression { expr, .. } => {
                self.bound_of(expr);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.bound_of(condition);
                self.block(then_block);
                if let Some(e) = else_block {
                    self.block(e);
                }
            }
            Statement::Match { value, arms, .. } => {
                self.bound_of(value);
                for arm in arms {
                    self.vars.push(HashMap::new());
                    for n in pattern_names(&arm.pattern) {
                        self.declare(&n, Vec::new());
                    }
                    if let Some(g) = arm.guard {
                        self.bound_of(g);
                    }
                    self.bound_of(arm.body);
                    self.pop_scope();
                }
            }
            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                let b = self.bound_of(iterable);
                self.vars.push(HashMap::new());
                for n in pattern_names(pattern) {
                    self.declare(&n, b.clone());
                }
                for s in body {
                    self.stmt(s);
                }
                self.pop_scope();
            }
            Statement::While {
                condition, body, ..
            } => {
                self.bound_of(condition);
                self.block(body);
            }
            Statement::Loop { body, .. }
            | Statement::Thread { body, .. }
            | Statement::Async { body, .. } => self.block(body),
            Statement::Defer { statement, .. } => self.stmt(statement),
            Statement::Break { value: Some(v), .. } => {
                self.bound_of(v);
            }
            _ => {}
        }
    }

    fn function(&mut self, f: &FunctionDecl) {
        self.arenas.clear();
        self.vars = vec![HashMap::new()]; // depth 1: function body; parameters are depth 0 (not tracked)
        let n = f.body.len();
        for (i, s) in f.body.iter().enumerate() {
            if i + 1 == n {
                if let Statement::Expression { expr, location } = s {
                    self.serial += 1;
                    let b = self.bound_of(expr);
                    self.escape_msg(
                        &b,
                        line_of(location).max(expr_line(expr)),
                        "is returned from the function (final expression)",
                        None,
                    );
                    continue;
                }
            }
            self.stmt(s);
        }
    }
}

fn walk_items<'a>(items: &'a [Item<'a>], out: &mut Vec<&'a FunctionDecl<'a>>) {
    for it in items {
        match it {
            Item::Function { decl, .. } => out.push(decl),
            Item::Impl { block, .. } => out.extend(block.functions.iter()),
            Item::Mod { items, .. } => walk_items(items, out),
            _ => {}
        }
    }
}

/// True when the program imports `std::arena` (the check is free for every other program).
pub fn uses_arenas(program: &Program) -> bool {
    fn any(items: &[Item]) -> bool {
        items.iter().any(|i| match i {
            Item::Use { path, .. } => {
                let p = path.join("::").replace('.', "::");
                p == "std::arena" || p.starts_with("std::arena::")
            }
            Item::Mod { items, .. } => any(items),
            _ => false,
        })
    }
    any(&program.items)
}

/// Run the escape check over every function of `program`.
pub fn check_program(program: &Program) -> Vec<ArenaEscape> {
    let mut fns = Vec::new();
    walk_items(&program.items, &mut fns);
    let mut c = Checker {
        arenas: HashMap::new(),
        vars: Vec::new(),
        serial: 0,
        errors: Vec::new(),
    };
    for f in fns {
        c.function(f);
    }
    c.errors
}

/// Compiler entry point: `std::arena` needs the `unstable_arenas` feature, and arena values must not escape.
pub fn gate(program: &Program) -> Result<(), String> {
    if !uses_arenas(program) {
        return Ok(());
    }
    if !cfg!(feature = "unstable_arenas") {
        return Err("error[WJ0161]: `std::arena` is unstable (ADR-0025)\n  = help: build the compiler with \
                    `--features unstable_arenas` to opt in"
            .to_string());
    }
    let errors = check_program(program);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors
            .iter()
            .map(ArenaEscape::render)
            .collect::<Vec<_>>()
            .join("\n\n"))
    }
}
