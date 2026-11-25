//! The compiler's own decisions, read by the interpreter where it would otherwise guess differently.
//!
//! Parameter passing: most mutations (`p.x = ..`, `v.push(..)`, `n += 1`) make a parameter `&mut` in both
//! the interpreter and the generated Rust. A plain reassignment of the whole parameter (`slot = f(slot)`)
//! does not always: the IR keeps it `&mut`, then code generation takes some such parameters by value (one
//! it also passes on or returns; `wj explain-ownership` shows the stages). The decision is the emitted
//! signature itself, so for such parameters the interpreter reads it: the module's file is generated to
//! Rust in-process (no cargo) and its signatures parsed (`ir::explain::emitted`, the reader
//! `explain-ownership` uses).
//!
//! Computed once per module, and only for programs that reassign a parameter.
//!
//! Integer widths (ADR-0028, ADR-0038 rung C6b): an unsuffixed integer literal is `int` unless the numeric solver
//! fixes another width for it (an index counter is `usize`), and the generated Rust prints that width
//! (`let mut i = 2_usize`). The interpreter gives the literal the same width, so `i - 1` at 0 panics with
//! Rust's overflow message where the compiled program does. The widths are the solver's published facts
//! (`IrFunction::literal_widths`, analyzer and IR pipeline in-process, no codegen), computed once per module
//! and only when the module evaluates its first unsuffixed integer literal; `int` stays the interpreter's
//! open literal width (`IntTy::Lit`).

use super::engine::{FuncDef, Interpreter};
use super::int::IntTy;
use crate::source_map::Location;
use crate::ir::explain::emitted::{parse_emitted, EmittedMode};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// Literal widths shared by the tests of one `wj test --interpret` run (soundness:
/// `docs/design/interp-soundness/widthshare.md`). Every test runs in a fresh `Interpreter` that re-parses its
/// project, so without this each test would run the whole analysis again for each file it loads. The key is the file's
/// content key (`note_parsed`: name, text, renames); the compiler build, the stdlib tables and the `WJ_*` environment
/// are constant inside one process. It is passed down by the test runner, never a static.
#[derive(Default)]
pub struct WidthShare {
    entries: Mutex<HashMap<String, SharedWidths>>,
    hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
    mismatches: std::sync::atomic::AtomicU64,
    /// Recompute on every hit and count a result that differs (the staleness gate).
    verify: bool,
}

/// A stored answer: `None` is "no width other than `int`" (the analysis failed, or did not run).
type SharedWidths = Option<Arc<LiteralWidths>>;

impl WidthShare {
    pub fn new() -> Self {
        Self::default()
    }
    /// A share that recomputes every hit and counts a stored answer that differs from the fresh one.
    pub fn verifying() -> Self {
        WidthShare { verify: true, ..Self::default() }
    }
    /// (hits, misses): a miss is one analysis run, a hit one skipped.
    pub fn counts(&self) -> (u64, u64) {
        use std::sync::atomic::Ordering::Relaxed;
        (self.hits.load(Relaxed), self.misses.load(Relaxed))
    }
    /// Hits whose stored answer differed from a fresh computation (only counted by a verifying share; must be 0).
    pub fn mismatches(&self) -> u64 {
        self.mismatches.load(std::sync::atomic::Ordering::Relaxed)
    }
    /// The `[fast]` line of `WJ_JIT_DEBUG` for a test run.
    pub fn line(&self) -> String {
        let (h, m) = self.counts();
        format!("[fast] widthshare hits {h} misses {m}")
    }
    /// The widths of `program` (`file` is the name it was parsed under), from this share or computed and kept.
    fn get_or_compute(&self, program: &'static crate::parser::Program<'static>, file: &str) -> Option<LiteralWidths> {
        use std::sync::atomic::Ordering::Relaxed;
        let Some(key) = parsed_key(file) else { return stored_literal_widths(program, file) };
        let found = self.entries.lock().unwrap_or_else(|e| e.into_inner()).get(&key).cloned();
        if let Some(w) = found {
            self.hits.fetch_add(1, Relaxed);
            let shared = w.as_deref().cloned();
            if self.verify && encode_widths(shared.as_ref()) != encode_widths(stored_literal_widths(program, file).as_ref()) {
                self.mismatches.fetch_add(1, Relaxed);
            }
            return shared;
        }
        self.misses.fetch_add(1, Relaxed);
        let w = stored_literal_widths(program, file);
        self.entries.lock().unwrap_or_else(|e| e.into_inner()).insert(key, w.clone().map(Arc::new));
        w
    }
}

/// A store for facts that are functions of one parsed file, injected by the driver (`cli::run_dev`): `key` covers
/// the file's name, text and renames; the store adds everything else the result depends on (the compiler build by
/// content, the baked stdlib, the working directory, the `WJ_*` environment, the `.wj.meta` sidecars `program`'s
/// `use`s can reach) and never answers when any of it changed.
pub trait FactStore: Send + Sync {
    fn get(&self, key: &str, program: &crate::parser::Program<'static>) -> Option<String>;
    fn put(&self, key: &str, program: &crate::parser::Program<'static>, text: &str);
}

static FACT_STORE: Mutex<Option<Arc<dyn FactStore>>> = Mutex::new(None);

/// The literal widths of `source` parsed as `file` with `renames`, as the store's text (`-`: no answer), through `share`
/// when given: the staleness gate compares a shared answer with a fresh one.
pub fn widths_text_of_source(share: Option<&WidthShare>, source: &str, file: &str, renames: &HashMap<String, String>) -> Option<String> {
    let program = super::loader::parse_source_renaming(source, file, renames).ok()?;
    let program: &'static crate::parser::Program<'static> = Box::leak(Box::new(program));
    let w = match share {
        Some(s) => s.get_or_compute(program, file),
        None => stored_literal_widths(program, file),
    };
    Some(encode_widths(w.as_ref()))
}

/// Content key of each parsed file (by the name it was parsed under): its text and the renames applied.
static PARSED: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

pub fn install_fact_store(store: Arc<dyn FactStore>) {
    *FACT_STORE.lock().unwrap_or_else(|e| e.into_inner()) = Some(store);
}

/// Record what `file` was parsed from (`loader::parse_source_renaming`); a later parse of it replaces the entry.
pub(crate) fn note_parsed(file: &str, source: &str, renames: &HashMap<String, String>) {
    let key = content_key(file, source, renames);
    PARSED.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(file.to_string(), key);
}

/// The content key of a parse: its name, text and renames (what a parse is a function of).
pub(crate) fn content_key(file: &str, source: &str, renames: &HashMap<String, String>) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    let mut part = |b: &[u8]| {
        h.update((b.len() as u64).to_le_bytes());
        h.update(b);
    };
    part(b"wj-literal-widths 1");
    part(file.as_bytes());
    part(source.as_bytes());
    let mut r: Vec<(&String, &String)> = renames.iter().collect();
    r.sort();
    for (a, b) in r {
        part(a.as_bytes());
        part(b.as_bytes());
    }
    format!("{:x}", h.finalize())
}

fn parsed_key(file: &str) -> Option<String> {
    PARSED.lock().unwrap_or_else(|e| e.into_inner()).as_ref()?.get(file).cloned()
}

/// `solve_literal_widths` through the injected store: `file` is the name `program` was parsed under.
fn stored_literal_widths(program: &'static crate::parser::Program<'static>, file: &str) -> Option<LiteralWidths> {
    let store = FACT_STORE.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let key = store.as_ref().and_then(|_| parsed_key(file));
    if let (Some(store), Some(key)) = (&store, &key) {
        if let Some(w) = store.get(key, program).and_then(|t| decode_widths(&t)) {
            return w;
        }
    }
    let w = width_sensitive(program).then(|| solve_literal_widths(program)).flatten();
    if let (Some(store), Some(key)) = (&store, &key) {
        store.put(key, program, &encode_widths(w.as_ref()));
    }
    w
}

/// `widths` as text: `-` for no answer (the analysis failed or did not run), else `line:col:width` lines.
fn encode_widths(widths: Option<&LiteralWidths>) -> String {
    let Some(w) = widths else { return "-".to_string() };
    let mut v: Vec<(&(usize, usize), &IntTy)> = w.iter().collect();
    v.sort_by_key(|(k, _)| **k);
    v.iter().map(|((l, c), t)| format!("{l}:{c}:{}\n", t.name())).collect()
}

fn decode_widths(text: &str) -> Option<Option<LiteralWidths>> {
    if text == "-" {
        return Some(None);
    }
    let mut out = LiteralWidths::new();
    for line in text.lines() {
        let mut it = line.split(':');
        let (l, c, t) = (it.next()?.parse().ok()?, it.next()?.parse().ok()?, IntTy::from_name(it.next()?)?);
        out.insert((l, c), t);
    }
    Some(Some(out))
}

/// Per function (`name`, or `Type::name` for a method): the parameters compiled code takes by `&mut`.
pub(crate) type MutRefParams = HashMap<String, HashSet<String>>;

/// Generate `program` to Rust and read each function's `&mut` parameters; `None` when generation fails
/// (the interpreter then keeps its own answer).
fn compute(program: &'static crate::parser::Program<'static>) -> Option<MutRefParams> {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let rust = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::codegen::generate_rust_source(program).ok()));
    std::panic::set_hook(previous);
    let file = parse_emitted(&rust.ok()??)?;
    Some(
        file.fns
            .into_iter()
            .map(|(name, f)| (name, f.params.into_iter().filter(|p| p.mode == EmittedMode::MutRef).map(|p| p.name).collect()))
            .collect(),
    )
}

/// Solver widths of a module's unsuffixed integer literals other than `int`, by source position (line, column).
pub(crate) type LiteralWidths = HashMap<(usize, usize), IntTy>;

/// What the interpreter knows about a module's literal widths.
#[derive(Clone, Default)]
pub(crate) enum LitWidths {
    #[default]
    Unknown,
    /// Analysis failed, or every literal is `int`.
    Open,
    Known(Rc<LiteralWidths>),
}

fn int_width(b: &crate::ir::safety_type::BaseType) -> Option<IntTy> {
    use crate::ir::safety_type::BaseType as B;
    Some(match b {
        B::I8 => IntTy::I8,
        B::I16 => IntTy::I16,
        B::I32 => IntTy::I32,
        B::I128 => IntTy::I128,
        B::U8 => IntTy::U8,
        B::U16 => IntTy::U16,
        B::U32 => IntTy::U32,
        B::U64 => IntTy::U64,
        B::U128 => IntTy::U128,
        B::Usize => IntTy::Usize,
        B::Isize => IntTy::Isize,
        _ => return None, // `int` (i64) stays the open literal width; floats are not integer literals
    })
}

/// The solver's literal widths for `program` (`None`: the analysis failed). The analyzer and the IR pipeline the
/// code generator runs (`ir::context::analyze_and_lower`), with the session's stdlib tables from the shared store.
fn solve_literal_widths(program: &'static crate::parser::Program<'static>) -> Option<LiteralWidths> {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let module = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut analyzer = crate::analyzer::Analyzer::new();
        analyzer.check_forbidden_rust_patterns(program).ok()?;
        let (analyzed, registry, _) = analyzer.analyze_program(program).ok()?;
        let tables = std::sync::Arc::new(crate::ir::stdlib_tables::StdlibTables::load_or_compute());
        let fields = crate::type_inference::struct_field_registry::local_struct_field_table(program);
        Some(crate::ir::IrPipeline::new().with_stdlib_tables(tables).lower_to_ir(&analyzed, &registry, Some(&fields)))
    }));
    std::panic::set_hook(previous);
    let module = module.ok()??;
    let mut out = LiteralWidths::new();
    for f in &module.functions {
        for (k, b) in &f.literal_widths {
            if let Some(t) = int_width(b) {
                out.insert(*k, t);
            }
        }
    }
    Some(out)
}

/// Std methods whose signature has a `usize` (an index, a length, a count): an argument or a result of one puts
/// the integers it meets in the solver's `usize` class.
const USIZE_METHODS: &[&str] = &[
    "len", "get", "get_mut", "remove", "insert", "swap", "swap_remove", "nth", "truncate", "split_at", "split_off", "resize",
    "with_capacity", "reserve", "repeat", "rotate_left", "rotate_right", "capacity", "position", "rposition", "find", "rfind",
    "count", "skip", "take", "step_by", "chunks", "windows",
];

/// Integer widths other than `int`, as type names.
const WIDTHS: &[&str] = &["i8", "i16", "i32", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "uint"];

/// Does a declared type name an integer width other than `int` (anywhere inside it)?
fn has_width(t: &crate::parser::Type) -> bool {
    let text = format!("{t:?}");
    WIDTHS.iter().any(|w| text.split(|c: char| !c.is_ascii_alphanumeric()).any(|word| word == *w || word.eq_ignore_ascii_case(w)))
}

/// Can the solver give one of `program`'s literals a width other than `int`? Only where a declared type names
/// another width, or an index, a cast or a std method with a `usize` in its signature appears (anything this walk
/// does not know counts as one, so the analysis is skipped only for programs that certainly have none).
/// Programs without one never pay for it.
fn width_sensitive(program: &crate::parser::Program<'_>) -> bool {
    use crate::parser::Item;
    fn func(f: &crate::parser::FunctionDecl<'_>) -> bool {
        f.parameters.iter().any(|p| has_width(&p.type_)) || f.return_type.as_ref().is_some_and(has_width) || f.body.iter().any(|s| stmt(s))
    }
    fn items(list: &[Item<'_>]) -> bool {
        list.iter().any(|i| match i {
            Item::Function { decl, .. } => func(decl),
            Item::Impl { block, .. } => block.functions.iter().any(func),
            Item::Mod { items: inner, .. } => items(inner),
            Item::Const { value, type_, .. } | Item::Static { value, type_, .. } => has_width(type_) || expr(value),
            Item::Struct { decl, .. } => decl.fields.iter().any(|f| has_width(&f.field_type)) || decl.tuple_fields.as_ref().is_some_and(|ts| ts.iter().any(has_width)),
            Item::Use { .. } | Item::BoundAlias { .. } => false,
            _ => true,
        })
    }
    fn block(list: &[&crate::parser::Statement<'_>]) -> bool {
        list.iter().any(|s| stmt(s))
    }
    fn stmt(s: &crate::parser::Statement<'_>) -> bool {
        use crate::parser::Statement as S;
        match s {
            S::Let { value, else_block, type_, .. } => type_.as_ref().is_some_and(has_width) || expr(value) || else_block.as_deref().is_some_and(block),
            S::Const { value, type_, .. } | S::Static { value, type_, .. } => has_width(type_) || expr(value),
            S::Assignment { target, value, .. } => expr(target) || expr(value),
            S::Return { value, .. } => matches!(value, Some(v) if expr(v)),
            S::Expression { expr: e, .. } => expr(e),
            S::If { condition, then_block, else_block, .. } => expr(condition) || block(then_block) || else_block.as_deref().is_some_and(block),
            S::Match { value, arms, .. } => expr(value) || arms.iter().any(|a| a.guard.is_some_and(expr) || expr(a.body)),
            S::For { iterable, body, .. } => expr(iterable) || block(body),
            S::While { condition, body, .. } => expr(condition) || block(body),
            S::Loop { body, .. } => block(body),
            S::Break { value, .. } => matches!(value, Some(v) if expr(v)),
            S::Continue { .. } | S::Use { .. } => false,
            _ => true,
        }
    }
    fn expr(e: &crate::parser::Expression<'_>) -> bool {
        use crate::parser::Expression as E;
        match e {
            E::Index { .. } | E::Cast { .. } => true,
            E::MethodCall { method, .. } if USIZE_METHODS.contains(&method.as_str()) => true,
            E::Literal { .. } | E::Identifier { .. } => false,
            E::Binary { left, right, .. } | E::Range { start: left, end: right, .. } => expr(left) || expr(right),
            E::Unary { operand: x, .. } | E::FieldAccess { object: x, .. } | E::TryOp { expr: x, .. } | E::Closure { body: x, .. } => expr(x),
            E::Call { function, arguments, .. } => expr(function) || arguments.iter().any(|(_, a)| expr(a)),
            E::MethodCall { object, arguments, .. } => expr(object) || arguments.iter().any(|(_, a)| expr(a)),
            E::StructLiteral { fields, .. } => fields.iter().any(|(_, f)| expr(f)),
            E::MapLiteral { pairs, .. } => pairs.iter().any(|(k, v)| expr(k) || expr(v)),
            E::Tuple { elements, .. } | E::Array { elements, .. } | E::MacroInvocation { args: elements, .. } => elements.iter().any(|x| expr(x)),
            E::Block { statements, .. } => block(statements),
            _ => true,
        }
    }
    items(&program.items)
}

/// An unsuffixed integer literal's value at width `t` (the lexer keeps a literal above `i64::MAX` as its bit
/// pattern, which a `u64`/`usize` reads back, as the suffixed literal does).
pub(crate) fn int_literal(i: i64, t: IntTy) -> crate::interpreter::value::Value {
    match t {
        IntTy::U64 | IntTy::Usize | IntTy::U128 => crate::interpreter::value::Value::Int(i as u64 as i128, t),
        _ => crate::interpreter::value::Value::Int(i as i128, t),
    }
}

impl Interpreter {
    /// The width the compiled program gives the unsuffixed integer literal at `loc` in `module`, when it is not
    /// `int` (`None`: an open `int` literal).
    pub(crate) fn literal_width(&mut self, module: usize, loc: Option<&Location>) -> Option<IntTy> {
        let loc = loc?;
        if !self.literal_widths_on {
            return None;
        }
        let state = match self.lit_widths.get(module) {
            Some(LitWidths::Unknown) | None => self.compute_literal_widths(module),
            Some(s) => s.clone(),
        };
        match state {
            LitWidths::Known(m) => m.get(&(loc.line, loc.column)).copied(),
            _ => None,
        }
    }

    /// All of `module`'s literal widths (`None`: every literal is `int`, or the analysis failed).
    pub(crate) fn literal_widths_of(&mut self, module: usize) -> Option<Rc<LiteralWidths>> {
        if !self.literal_widths_on {
            return None;
        }
        let state = match self.lit_widths.get(module) {
            Some(LitWidths::Unknown) | None => self.compute_literal_widths(module),
            Some(s) => s.clone(),
        };
        match state {
            LitWidths::Known(m) => Some(m),
            _ => None,
        }
    }

    /// `literal_width` for a literal known only by its location (machine-code lowering): the module whose file it is.
    pub(crate) fn literal_width_at(&mut self, loc: Option<&Location>) -> Option<IntTy> {
        let l = loc?;
        let module = self.modules.iter().position(|m| m.file == l.file)?;
        self.literal_width(module, loc)
    }

    fn compute_literal_widths(&mut self, module: usize) -> LitWidths {
        let state = self.compute_literal_widths_in(module);
        if self.lit_widths.len() <= module {
            self.lit_widths.resize(module + 1, LitWidths::Unknown);
        }
        self.lit_widths[module] = state.clone();
        state
    }

    fn compute_literal_widths_in(&mut self, module: usize) -> LitWidths {
        // an overlay module (function-body `use`) reads its file's program through `fallback`
        let mut m = module;
        let program = loop {
            if let Some(p) = self.programs.get(&m) {
                break Some(*p);
            }
            match self.modules.get(m).and_then(|x| x.fallback) {
                Some(f) => m = f,
                None => break None,
            }
        };
        let key = program.map(|p| p as *const _ as usize);
        let state = match key.and_then(|k| self.lit_widths_by_program.get(&k).cloned()) {
            Some(s) => s,
            None => {
                let file = self.modules.get(m).map(|x| x.file.to_string_lossy().to_string()).unwrap_or_default();
                let computed = match (program, &self.width_share) {
                    (Some(p), Some(share)) => share.get_or_compute(p, &file),
                    (Some(p), None) => stored_literal_widths(p, &file),
                    (None, _) => None,
                };
                let s = match computed {
                    Some(w) if !w.is_empty() => LitWidths::Known(Rc::new(w)),
                    _ => LitWidths::Open,
                };
                if let Some(k) = key {
                    self.lit_widths_by_program.insert(k, s.clone());
                }
                s
            }
        };
        state
    }

    /// Does compiled code take parameter `param` of `f` by `&mut`? `None`: no fact (the module was not
    /// loaded from a program, the analysis failed, or it does not know the function).
    pub(crate) fn compiled_mut_ref(&mut self, f: &FuncDef, param: &str) -> Option<bool> {
        if !self.compiled_facts.contains_key(&f.module) {
            let facts = self.programs.get(&f.module).and_then(|p| compute(p)).map(Rc::new);
            self.compiled_facts.insert(f.module, facts);
        }
        let facts = self.compiled_facts.get(&f.module)?.as_ref()?;
        let key = match &f.self_ty {
            Some(t) => format!("{t}::{}", f.name),
            None => f.name.clone(),
        };
        facts.get(&key).map(|ps| ps.contains(param))
    }
}
