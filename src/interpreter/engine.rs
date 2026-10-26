//! The interpreter state and public entry points.
//!
//! Responsibilities are split across sibling modules:
//! `register` (definitions) / `loader` (files) / `eval_expr`, `eval_stmt`, `pattern`,
//! `place` (evaluation) / `call` (functions, methods, closures) / `ops`, `coerce`
//! (integer-width semantics) / `format` (println/format!) / `builtins` (stdlib).

use super::builtins::Registry;
use super::error::{ErrorKind, Frame, RuntimeError, Span, R};
use super::value::{Slot, Value};
use crate::parser::{Decorator, Parameter, Program, Statement, Type};
use crate::source_map::Location;
use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;

/// A user function / method / default trait method.
pub struct FuncDef {
    pub name: String,
    pub qual: Rc<str>,
    pub params: &'static [Parameter<'static>],
    pub ret: Option<&'static Type>,
    pub body: &'static [&'static Statement<'static>],
    pub module: usize,
    pub self_ty: Option<Rc<str>>,
    pub decorators: &'static [Decorator<'static>],
    pub is_extern: bool,
    /// `async fn`: calling it yields a lazy future that runs when awaited.
    pub is_async: bool,
    /// Generic parameters in scope for the body: impl-level first, then the function's own.
    pub type_params: Vec<String>,
    /// Per-parameter (self excluded): does the body mutate it? Computed lazily.
    pub mutated: OnceCell<Rc<Vec<bool>>>,
    pub self_mutated: OnceCell<bool>,
}

pub struct StructDef {
    pub fields: Rc<[(&'static str, Type)]>,
    /// The type and field names, interned: every instance built from a literal shares them.
    pub rc_name: Rc<str>,
    pub rc_fields: Rc<[Rc<str>]>,
    pub tuple: bool,
    pub copy: bool,
}

pub struct VariantDef {
    pub name: &'static str,
    pub kind: VariantKind,
}

pub enum VariantKind {
    Unit,
    Tuple(Vec<Type>),
    Struct(Vec<(&'static str, Type)>),
}

pub struct EnumDef {
    pub variants: Vec<VariantDef>,
    /// The type and variant names, interned: every value built by `make_variant` shares them.
    pub rc_name: Rc<str>,
    pub rc_variants: Vec<Rc<str>>,
}

pub struct TraitDef {
    pub supertraits: Vec<String>,
}

/// What a `use` introduced into a module's namespace.
#[derive(Clone, Debug)]
pub enum Import {
    Module(usize),
    /// Item `name` inside `module`.
    Item(usize, String),
    /// A builtin (native) std path such as `std::fs`.
    Std(String),
    /// A name that is both a module and a value (`pub use version::version` next to `mod version`):
    /// the module for paths (`version::f`), the item `name` of `module` for a bare use.
    Both(usize, usize, String),
}

pub struct Module {
    pub name: String,
    pub file: PathBuf,
    pub parent: Option<usize>,
    pub funcs: HashMap<String, Rc<FuncDef>>,
    pub imports: HashMap<String, Import>,
    pub children: HashMap<String, usize>,
    pub consts: HashMap<String, &'static crate::parser::Expression<'static>>,
    pub const_types: HashMap<String, &'static Type>,
    /// Names a function-body `use` added on top of this module (lookups fall through to `fallback`).
    pub fallback: Option<usize>,
    /// Standard-library modules glob-imported here (`use std::log::*`), as `std::log`.
    pub std_globs: Vec<String>,
}

pub struct Scope {
    pub vars: Vec<(&'static str, Slot)>,
    pub marks: Vec<usize>,
    pub module: usize,
    pub self_ty: Option<Rc<str>>,
    pub ret_ty: Option<&'static Type>,
    pub defers: Vec<&'static Statement<'static>>,
    /// Generic parameters of the running function and what they were bound to (`None` = not inferable).
    pub tparams: Vec<(Rc<str>, Option<Type>)>,
}

impl Scope {
    pub fn new(module: usize) -> Self {
        Scope {
            vars: Vec::new(),
            marks: Vec::new(),
            module,
            self_ty: None,
            ret_ty: None,
            defers: Vec::new(),
            tparams: Vec::new(),
        }
    }
}

pub struct Interpreter {
    // SAFETY note for `retained`: it is declared last, so it is dropped after every field that holds `&'static` AST
    // references (fields drop in declaration order); see `retain_program`.
    pub(crate) modules: Vec<Module>,
    pub(crate) structs: HashMap<String, StructDef>,
    pub(crate) enums: HashMap<String, EnumDef>,
    pub(crate) traits: HashMap<String, TraitDef>,
    pub(crate) trait_methods: HashMap<String, Vec<&'static crate::parser::TraitMethod<'static>>>,
    pub(crate) trait_impls: Vec<(String, String)>,
    pub(crate) methods: HashMap<(String, String), Rc<FuncDef>>,
    /// `impl From<S> for T`: every impl of a target type with its source type (several coexist; see `from_impls.rs`).
    /// The synthesized `{ ... }` block of each `thread {}` / `async {}` statement, by statement address.
    pub(crate) task_blocks: HashMap<usize, &'static crate::parser::Expression<'static>>,
    /// Struct/enum names declared in several modules, and the key (`module::Name`) of each declaration (`type_keys.rs`).
    pub(crate) dup_types: HashSet<String>,
    pub(crate) dup_decls: HashSet<String>,
    pub(crate) from_impls: HashMap<String, Vec<(Type, Rc<FuncDef>)>>,
    pub(crate) user_mutating: HashSet<String>,
    pub(crate) type_aliases: HashMap<String, Type>,
    pub(crate) globals: HashMap<(usize, String), Slot>,
    pub(crate) scope: Scope,
    /// Call-site cache: call expression address -> the module function it resolved to.
    pub(crate) call_cache: HashMap<usize, Rc<FuncDef>, std::hash::BuildHasherDefault<super::fasthash::AddrHasher>>,
    /// Recycled variable vectors of finished frames.
    pub(crate) vec_pool: Vec<Vec<(&'static str, Slot)>>,
    /// `--engine=resolved`: run compiled functions where possible (see `resolved/`).
    pub(crate) engine_resolved: bool,
    /// Compiled forms by `FuncDef` address (`None` = not compilable).
    /// The `FuncDef` is kept alive with its entry so its address is never reused (hot reload replaces bodies).
    pub(crate) compiled: HashMap<usize, (Rc<FuncDef>, Option<Rc<super::resolved::CompiledFn>>), std::hash::BuildHasherDefault<super::fasthash::AddrHasher>>,
    /// Compiled closure bodies by body address (`None` = run in the walker).
    pub(crate) compiled_closures: HashMap<usize, Option<Rc<super::resolved::CompiledClosure>>, std::hash::BuildHasherDefault<super::fasthash::AddrHasher>>,
    /// Locals of active compiled calls (one window per call).
    pub(crate) rstack: Vec<Value>,
    /// Machine code for hot integer functions is allowed (`--no-jit` / `WJ_JIT=0` clear it).
    pub(crate) jit_enabled: bool,
    /// Closure-engine iterations after which a loop enters machine code (`WJ_JIT_LOOP_HOT` in the CLI
    /// overrides it: 0 makes every loop try at its first iteration, which the jit stress tests use).
    pub(crate) jit_loop_hot: u32,
    /// Report machine-code loop decisions on stderr (`WJ_JIT_DEBUG`).
    pub(crate) jit_debug: bool,
    /// A machine-code check to leave out (`WJ_JIT_MUTANT`; honoured in debug builds only).
    pub(crate) jit_mutant: Option<String>,
    /// Statement registries of compiled code (`resolved/osr.rs`), alive as long as the code is.
    pub(crate) osr_regs: Vec<Rc<super::resolved::osr::Reg>>,
    /// Machine code of hot integer functions (ADR-0020 step 7, `--features jit`).
    #[cfg(feature = "jit")]
    pub(crate) jit: Option<Box<super::resolved::jit::Jit>>,
    /// `std::subprocess` children by handle id.
    pub(crate) subprocesses: HashMap<u64, super::builtins::std_proc::Session>,
    pub(crate) next_subprocess: u64,
    /// Threads/tasks, the virtual clock and the schedule seed (see `sched.rs`).
    pub(crate) sched: super::sched::Sched,
    pub(crate) epoch: std::time::Instant,
    /// Global random generator (`std::random`), seeded from entropy unless time is deterministic.
    pub(crate) rng: Option<rand::rngs::StdRng>,
    /// `std::log` threshold (0 = error .. 4 = trace); `None` until `log::init`.
    pub(crate) log_level: Option<i32>,
    /// How `std::http` client calls are answered (default: refuse).
    pub(crate) http: super::builtins::std_http::HttpMode,
    /// Capability gate for sink calls (`caps.rs`); `None` = unrestricted (library and test use).
    pub(crate) caps: Option<super::caps::CapGate>,
    /// Record / replay / simulate port for effects (`effect_port`); `None` = effects are live.
    pub(crate) port: Option<std::cell::RefCell<super::effect_port::EffectPort>>,
    /// Journal of `durable fn` steps (`durable_rt`); `None` until a durable function is called or `enable_durable`.
    pub(crate) durable: Option<Box<super::durable_rt::DurableRt>>,
    /// Scopes of the callers of the running function (roots for reachability, see `heap.rs`).
    pub(crate) frames: Vec<Scope>,
    /// Types with `impl Drop` and the bookkeeping for their instances.
    pub(crate) drop_types: HashSet<String>,
    pub(crate) next_uid: u32,
    pub(crate) dropped: HashSet<u32>,
    /// Ownership moves out of variables (`moves.rs`).
    pub(crate) moves: super::moves::Moves,
    /// `--hash-seed`: permutation of HashMap/HashSet iteration order (0 = key order).
    pub hash_seed: u64,
    /// Active user calls: (qualified name, call-site location). Rendered lazily on error.
    pub(crate) backtrace: Vec<(Rc<str>, Option<&'static Location>)>,
    pub(crate) cur: Option<&'static Location>,
    pub(crate) output: Vec<String>,
    pub(crate) capture_output: bool,
    pub(crate) out: std::io::BufWriter<std::io::Stdout>,
    pub(crate) builtins: Registry,
    pub(crate) depth: usize,
    pub(crate) max_depth: usize,
    /// (stack base address, byte budget) when running on the big interpreter thread: recursion is then
    /// bounded by stack bytes used instead of a frame count.
    pub(crate) stack_guard: Option<(usize, usize)>,
    pub(crate) args: Vec<String>,
    pub(crate) closure_counter: usize,
    pub(crate) mutating_analysis: HashSet<usize>,
    /// Turbofish / expected type for the next builtin call (`parse::<i32>()`, `collect::<Vec<_>>()`).
    pub(crate) type_hint: Option<Type>,
    /// Expected type of one specific call expression (address, type); see `generics.rs`.
    pub(crate) expected: Option<(usize, Type)>,
    /// (module, name) pairs already probed on disk and not found.
    pub(crate) missing_modules: HashSet<(usize, String)>,
    /// `(module, address of the `use` statement)` -> the module that adds that function-body import.
    /// Names of the dependency crates mounted as top-level modules (reachable by name from any module; package attribution for grants).
    pub(crate) dep_crates: HashSet<String>,
    pub(crate) body_use_modules: HashMap<(usize, usize), usize>,
    /// Set while a whole program is loaded through the shared resolver (`project.rs`): `use` items
    /// are then bound after every module is registered, instead of one at a time.
    pub(crate) project_loading: bool,
    /// Bridge library for `extern fn` (ADR-0020), when the project provides Rust code.
    pub(crate) host: Option<std::rc::Rc<super::host_bridge::HostLib>>,
    /// `wj run --hot`: pending file edits applied at function entry (`hot.rs`).
    pub(crate) hot: Option<Box<super::hot::HotState>>,
    /// Programs loaded into this interpreter (files, REPL inputs, hot-reload revisions). Every `&'static` AST node the
    /// definitions above point at lives in one of these (or in the arenas its `Program` owns), so they are kept for the
    /// interpreter's lifetime and freed when it is dropped, instead of being leaked for the life of the process.
    pub(crate) retained: Vec<Box<Program<'static>>>,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        let mut s = Interpreter {
            modules: Vec::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            traits: HashMap::new(),
            trait_methods: HashMap::new(),
            trait_impls: Vec::new(),
            methods: HashMap::new(),
            from_impls: HashMap::new(),
            dup_types: HashSet::new(),
            dup_decls: HashSet::new(),
            task_blocks: HashMap::new(),
            user_mutating: HashSet::new(),
            type_aliases: HashMap::new(),
            globals: HashMap::new(),
            scope: Scope::new(0),
            call_cache: HashMap::default(),
            vec_pool: Vec::new(),
            engine_resolved: false,
            compiled: HashMap::default(),
            compiled_closures: HashMap::default(),
            rstack: Vec::new(),
            jit_enabled: true,
            osr_regs: Vec::new(),
            jit_loop_hot: 256,
            jit_debug: false,
            jit_mutant: None,
            #[cfg(feature = "jit")]
            jit: None,
            subprocesses: HashMap::new(),
            next_subprocess: 0,
            sched: super::sched::Sched::new(),
            epoch: std::time::Instant::now(),
            rng: None,
            log_level: None,
            http: Default::default(),
            caps: None,
            port: None,
            durable: None,
            frames: Vec::new(),
            drop_types: HashSet::new(),
            next_uid: 0,
            dropped: HashSet::new(),
            moves: Default::default(),
            hash_seed: 0,
            backtrace: Vec::new(),
            cur: None,
            output: Vec::new(),
            capture_output: false,
            out: std::io::BufWriter::new(std::io::stdout()),
            builtins: Registry::standard(),
            depth: 0,
            max_depth: 10_000,
            stack_guard: None,
            args: Vec::new(),
            closure_counter: 0,
            mutating_analysis: HashSet::new(),
            type_hint: None,
            retained: Vec::new(),
            expected: None,
            missing_modules: HashSet::new(),
            dep_crates: HashSet::new(),
            body_use_modules: HashMap::new(),
            project_loading: false,
            host: None,
            hot: None,
        };
        s.modules.push(Module::new("main", PathBuf::from("main.wj"), None));
        if std::thread::current().name() == Some(super::runner::BIG_STACK_THREAD) {
            s.guard_stack();
        }
        s
    }

    /// Create an interpreter that captures output (for testing).
    pub fn new_capturing() -> Self {
        let mut s = Self::new();
        s.capture_output = true;
        s
    }

    /// Call first thing on the big-stack thread (see `runner::with_big_stack`).
    pub fn guard_stack(&mut self) {
        let here = 0u8;
        self.stack_guard = Some((&here as *const u8 as usize, super::runner::STACK_BYTES - super::runner::STACK_HEADROOM));
    }

    /// Schedule seed: 0 = FIFO run-to-block, N = seeded exploration of other interleavings.
    pub fn set_sched_seed(&mut self, seed: u64) {
        self.sched.set_seed(seed);
    }

    /// Deterministic time: no real clock is read; time advances only through `sleep`.
    pub fn set_deterministic_time(&mut self, on: bool) {
        self.sched.deterministic = on;
    }

    pub fn set_http_mode(&mut self, mode: super::builtins::std_http::HttpMode) {
        self.http = mode;
    }

    /// Select the resolved (compiled closure) engine for compilable functions.
    pub fn set_engine_resolved(&mut self, on: bool) {
        self.engine_resolved = on;
    }

    /// Allow machine code for hot integer functions (on by default; a no-op without the `jit` feature).
    pub fn set_jit(&mut self, on: bool) {
        self.jit_enabled = on;
    }

    pub fn set_args(&mut self, args: Vec<String>) {
        self.args = args;
    }

    /// Get captured output
    pub fn get_output(&self) -> String {
        self.output.join("")
    }

    pub fn flush(&mut self) {
        let _ = self.out.flush();
    }

    pub(crate) fn write_out(&mut self, s: &str) {
        if self.capture_output {
            self.output.push(s.to_string());
        } else {
            let _ = self.out.write_all(s.as_bytes());
        }
    }

    pub(crate) fn write_err(&mut self, s: &str) {
        if !self.capture_output {
            let _ = self.out.flush();
            eprint!("{}", s);
        }
    }

    /// Run a complete program (single file). Compatibility entry point.
    pub fn run(&mut self, program: &Program<'static>) -> Result<Value, String> {
        self.run_checked(program).map_err(|e| {
            self.flush();
            match &e.span {
                Some(s) => format!("{} ({})", e.message, s),
                None => e.message.clone(),
            }
        })
    }

    /// Run a program, keeping the structured error (kind, span, backtrace).
    pub fn run_checked(&mut self, program: &Program<'static>) -> Result<Value, RuntimeError> {
        let prog = self.retain_program(program.clone());
        self.load_program(0, prog)?;
        self.call_main()
    }

    pub fn call_main(&mut self) -> Result<Value, RuntimeError> {
        let main = self.modules[0]
            .funcs
            .get("main")
            .cloned()
            .ok_or_else(|| RuntimeError::new(ErrorKind::Semantic, "No main() function found"))?;
        let r = self.call_func(&main, Vec::new(), None).and_then(|v| self.await_value(v));
        self.finish(r)
    }

    pub(crate) fn finish(&mut self, r: R) -> Result<Value, RuntimeError> {
        let out = match r {
            Ok(v) => Ok(v),
            Err(super::error::Ctl::Return(v)) => Ok(v),
            Err(super::error::Ctl::Err(e)) => Err(*e),
            Err(_) => Ok(Value::Unit),
        };
        self.flush();
        // a replay that did not use the whole log is a divergence; a recording is flushed to disk
        match (out, self.finish_port()) {
            (Ok(_), Err(e)) => Err(e),
            (out, _) => out,
        }
    }

    pub(crate) fn span_of(&self, loc: Option<&'static Location>) -> Option<Span> {
        loc.map(|l| Span {
            file: l.file.to_string_lossy().to_string(),
            line: l.line,
            column: l.column,
        })
    }

    /// Attach the current span and backtrace to an error that has none yet.
    pub(crate) fn annotate(&self, mut e: RuntimeError) -> RuntimeError {
        if e.span.is_none() {
            e.span = self.span_of(self.cur);
        }
        if e.backtrace.is_empty() {
            e.backtrace = self
                .backtrace
                .iter()
                .rev()
                .map(|(name, site)| Frame { function: name.to_string(), call_site: self.span_of(*site) })
                .collect();
        }
        e
    }

    pub(crate) fn mark(&mut self, loc: Option<&'static Location>) {
        if loc.is_some() {
            self.cur = loc;
        }
    }
}

impl Module {
    pub fn new(name: &str, file: PathBuf, parent: Option<usize>) -> Module {
        Module {
            name: name.to_string(),
            file,
            parent,
            funcs: HashMap::new(),
            imports: HashMap::new(),
            children: HashMap::new(),
            consts: HashMap::new(),
            const_types: HashMap::new(),
            fallback: None,
            std_globs: Vec::new(),
        }
    }
}

#[allow(dead_code)]
pub(crate) type SharedCell<T> = Rc<RefCell<T>>;

impl Interpreter {
    /// Keep `program` alive as long as this interpreter and hand out the `&'static` view the evaluator uses.
    ///
    /// SAFETY: the box is never moved or dropped before the interpreter (`retained` is the last field, programs are
    /// only pushed), so the reference is valid for every use inside the interpreter. The `'static` is a promise
    /// scoped to the interpreter: a `Value` holding a function must not be called after the interpreter is dropped
    /// (it cannot be: calling needs `&mut Interpreter`).
    pub(crate) fn retain_program(&mut self, program: Program<'static>) -> &'static Program<'static> {
        let boxed = Box::new(program);
        let ptr: *const Program<'static> = &*boxed;
        self.retained.push(boxed);
        unsafe { &*ptr }
    }
}
