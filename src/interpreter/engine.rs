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
    /// Per-parameter (self excluded): does the body mutate it? Computed lazily.
    pub mutated: OnceCell<Rc<Vec<bool>>>,
    pub self_mutated: OnceCell<bool>,
}

pub struct StructDef {
    pub fields: Vec<(&'static str, Type)>,
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
}

pub struct Scope {
    pub vars: Vec<(&'static str, Slot)>,
    pub marks: Vec<usize>,
    pub module: usize,
    pub self_ty: Option<Rc<str>>,
    pub ret_ty: Option<&'static Type>,
    pub defers: Vec<&'static Statement<'static>>,
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
        }
    }
}

pub struct Interpreter {
    pub(crate) modules: Vec<Module>,
    pub(crate) structs: HashMap<String, StructDef>,
    pub(crate) enums: HashMap<String, EnumDef>,
    pub(crate) traits: HashMap<String, TraitDef>,
    pub(crate) trait_methods: HashMap<String, Vec<&'static crate::parser::TraitMethod<'static>>>,
    pub(crate) trait_impls: Vec<(String, String)>,
    pub(crate) methods: HashMap<(String, String), Rc<FuncDef>>,
    pub(crate) user_mutating: HashSet<String>,
    pub(crate) type_aliases: HashMap<String, Type>,
    pub(crate) globals: HashMap<(usize, String), Slot>,
    pub(crate) scope: Scope,
    /// Active user calls: (qualified name, call-site location). Rendered lazily on error.
    pub(crate) backtrace: Vec<(Rc<str>, Option<&'static Location>)>,
    pub(crate) cur: Option<&'static Location>,
    pub(crate) output: Vec<String>,
    pub(crate) capture_output: bool,
    pub(crate) out: std::io::BufWriter<std::io::Stdout>,
    pub(crate) builtins: Registry,
    pub(crate) depth: usize,
    pub(crate) max_depth: usize,
    pub(crate) args: Vec<String>,
    pub(crate) closure_counter: usize,
    pub(crate) mutating_analysis: HashSet<usize>,
    /// Turbofish / expected type for the next builtin call (`parse::<i32>()`, `collect::<Vec<_>>()`).
    pub(crate) type_hint: Option<Type>,
    /// (module, name) pairs already probed on disk and not found.
    pub(crate) missing_modules: HashSet<(usize, String)>,
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
            user_mutating: HashSet::new(),
            type_aliases: HashMap::new(),
            globals: HashMap::new(),
            scope: Scope::new(0),
            backtrace: Vec::new(),
            cur: None,
            output: Vec::new(),
            capture_output: false,
            out: std::io::BufWriter::new(std::io::stdout()),
            builtins: Registry::standard(),
            depth: 0,
            max_depth: 50_000,
            args: Vec::new(),
            closure_counter: 0,
            mutating_analysis: HashSet::new(),
            type_hint: None,
            missing_modules: HashSet::new(),
        };
        s.modules.push(Module::new("main", PathBuf::from("main.wj"), None));
        s
    }

    /// Create an interpreter that captures output (for testing).
    pub fn new_capturing() -> Self {
        let mut s = Self::new();
        s.capture_output = true;
        s
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
        let prog: &'static Program<'static> = Box::leak(Box::new(program.clone()));
        self.load_program(0, prog)?;
        self.call_main()
    }

    pub fn call_main(&mut self) -> Result<Value, RuntimeError> {
        let main = self.modules[0]
            .funcs
            .get("main")
            .cloned()
            .ok_or_else(|| RuntimeError::new(ErrorKind::Semantic, "No main() function found"))?;
        let r = self.call_func(&main, Vec::new(), None);
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
        out
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
        }
    }
}

#[allow(dead_code)]
pub(crate) type SharedCell<T> = Rc<RefCell<T>>;
