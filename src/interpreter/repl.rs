//! Persistent REPL session: definitions and `let` bindings survive between inputs.

use super::engine::Interpreter;
use super::error::{ErrorKind, RuntimeError};
use super::loader::parse_source;
use super::source_scan;
use crate::parser::Item;

/// Does this input start a top-level item (rather than a statement/expression)?
fn is_item_start(src: &str) -> bool {
    let s = src.trim_start();
    let s = s.strip_prefix("pub ").unwrap_or(s);
    ["fn ", "struct ", "enum ", "impl ", "impl<", "trait ", "use ", "const ", "static ", "type ", "mod ", "@"]
        .iter()
        .any(|k| s.starts_with(k))
}

/// Does `src` ask for more text before it can be evaluated? True while a bracket or string is open, and while the
/// parser reports that the input ended in the middle of a construct (`1 +`, `x.`, `if a` ...): the session then
/// continues the input on the next line until it parses.
pub fn needs_more(src: &str) -> bool {
    if source_scan::scan(src).open() {
        return true;
    }
    if is_item_start(src) {
        return parse_source(src, "<repl>").err().is_some_and(|e| ends_early(&e, None));
    }
    let wrapped = format!("fn __repl__() {{\n{src}\n}}");
    let closing_line = wrapped.lines().count();
    parse_source(&wrapped, "<repl>").err().is_some_and(|e| ends_early(&e, Some(closing_line)))
}

/// A parse error that stops at the end of the input: it says so, or (for a statement, which is parsed inside a
/// wrapper function) it stops at the wrapper's closing brace on `closing_line`.
fn ends_early(error: &str, closing_line: Option<usize>) -> bool {
    let e = error.to_ascii_lowercase();
    let at_wrapper_end = closing_line.is_some_and(|l| e.contains("rbrace") && e.contains(&format!("(at line {l},")));
    at_wrapper_end || e.contains("eof") || e.contains("end of input") || e.contains("end of file")
}

impl Interpreter {
    /// Evaluate one REPL input. Returns the text to print (if the input produced a value).
    pub fn repl_eval(&mut self, src: &str) -> Result<Option<String>, RuntimeError> {
        let sem = |m: String| RuntimeError::new(ErrorKind::Semantic, m);
        if is_item_start(src) {
            let prog = parse_source(src, "<repl>").map_err(|e| sem(format!("parse error: {}", e)))?;
            let prog = self.retain_program(prog);
            self.register_items(0, &prog.items)?;
            self.finish_registration()?;
            return Ok(None);
        }
        let wrapped = format!("fn __repl__() {{\n{}\n}}", src);
        let prog = parse_source(&wrapped, "<repl>").map_err(|e| sem(format!("parse error: {}", e)))?;
        let prog = self.retain_program(prog);
        let Some(Item::Function { decl, .. }) = prog.items.first() else {
            return Err(sem("could not parse input".into()));
        };
        let r = self.exec_stmts(&decl.body);
        let out = match r {
            Ok(v) => Ok(v),
            Err(super::error::Ctl::Return(v)) => Ok(v),
            Err(c) => Err(self.ctl_to_error(c)),
        };
        self.flush();
        let v = out?;
        if matches!(v, super::value::Value::Unit) {
            return Ok(None);
        }
        let text = match self.display_string(&v) {
            Ok(s) if matches!(v, super::value::Value::Str(_)) => format!("{:?}", s),
            Ok(s) => s,
            Err(_) => self.debug_string(&v).map_err(|c| self.ctl_to_error(c))?,
        };
        Ok(Some(text))
    }
}

/// A fresh REPL interpreter, sandboxed like `wj run` (ADR-0021): the working directory's `wj.toml` grant, else the
/// local profile (files under the working directory, time, random, console).
fn new_repl_interpreter(scratch: Option<&super::caps::Scratch>) -> Interpreter {
    let mut it = Interpreter::new();
    it.guard_stack();
    let here = std::env::current_dir().unwrap_or_default().join("repl.wj");
    if let Ok(gate) = super::caps::gate_for_run(&here, false, &[], false, "") {
        it.set_caps(gate.map(|g| {
            let g = g.for_tests();
            match scratch {
                Some(s) => g.with_scratch(s.path()),
                None => g,
            }
        }));
    }
    it
}

/// What the compiled build would say about the session so far: `(item sources, statement sources)` in, one
/// message per predicted rejection out. The REPL prints them as warnings and goes on; it never refuses an input
/// because of one (the interpreter is the REPL's authority, the prediction is advice).
pub type Predictor = fn(items: &str, body: &str) -> Vec<String>;

/// What a line typed at the prompt means.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    Exit,
    Help,
    Reset,
    History,
    /// `!N` (1-based) or `!!` (the last entry): evaluate an earlier input again.
    Recall(Option<usize>),
}

fn command(line: &str) -> Option<Command> {
    match line.trim() {
        "exit" | "quit" | ":q" => Some(Command::Exit),
        ":help" => Some(Command::Help),
        ":reset" => Some(Command::Reset),
        ":history" => Some(Command::History),
        "!!" => Some(Command::Recall(None)),
        t => t.strip_prefix('!').and_then(|n| n.parse().ok()).map(|n| Command::Recall(Some(n))),
    }
}

const HELP: &str = ":help     this text\n:reset    forget all definitions and variables\n:history  list the inputs of this session\n!N, !!    evaluate input N (or the last one) again\nexit      leave the REPL\n\nAn input continues on the next line until it is complete (an open bracket or string, or an unfinished expression).";

/// One session: the interpreter plus what the REPL remembers about it.
struct Session {
    it: Interpreter,
    /// Every input evaluated since the session began, oldest first (`:history`, `!N`).
    history: Vec<String>,
    /// Item and statement inputs that ran, the program the compiled build would see (for the [`Predictor`]).
    items: Vec<String>,
    body: Vec<String>,
    warned: std::collections::HashSet<String>,
    predictor: Option<Predictor>,
}

impl Session {
    fn eval(&mut self, input: &str) {
        self.history.push(input.to_string());
        match self.it.repl_eval(input) {
            Ok(shown) => {
                if let Some(s) = &shown {
                    println!("{s}");
                }
                if is_item_start(input) {
                    self.items.push(input.to_string());
                } else if shown.is_none() {
                    self.body.push(input.to_string());
                }
                self.predict();
            }
            Err(e) => eprintln!("{}", e.render()),
        }
    }

    /// Warn (once per message) about what the compiled build would reject in the session so far.
    fn predict(&mut self) {
        let Some(predict) = self.predictor else { return };
        for message in predict(&self.items.join("\n"), &self.body.join("\n")) {
            if self.warned.insert(message.clone()) {
                eprintln!("warning: the compiled build would reject this ({message}); the REPL runs it anyway");
            }
        }
    }

    fn list_history(&self) {
        for (i, h) in self.history.iter().enumerate() {
            let mut lines = h.lines();
            println!("{:>3}  {}", i + 1, lines.next().unwrap_or(""));
            for rest in lines {
                println!("     {rest}");
            }
        }
    }

    fn forget(&mut self, it: Interpreter) {
        self.it = it;
        self.items.clear();
        self.body.clear();
        self.warned.clear();
    }
}

/// The REPL with a crate host for the project's surfaced Rust crates (ADR-0031); it survives `:reset`.
pub fn run_repl_with(crates: Option<super::crate_host::CrateHost>, predictor: Option<Predictor>) -> i32 {
    use std::io::{BufRead, IsTerminal, Write};
    super::runner::with_big_stack(move || {
        let crates = crates.map(std::rc::Rc::new);
        let fresh = |scratch: Option<&super::caps::Scratch>| {
            let mut it = new_repl_interpreter(scratch);
            if let Some(h) = &crates {
                it.attach_shared_crate_host(h.clone());
            }
            it
        };
        let interactive = std::io::stdin().is_terminal();
        let scratch = super::caps::Scratch::new();
        let mut s = Session {
            it: fresh(scratch.as_ref()),
            history: Vec::new(),
            items: Vec::new(),
            body: Vec::new(),
            warned: Default::default(),
            predictor,
        };
        if interactive {
            println!("Windjammerscript REPL (interpreted, no cargo). Type :help for commands, exit or Ctrl-D to quit.");
        }
        let stdin = std::io::stdin();
        let mut pending = String::new();
        loop {
            if interactive {
                print!("{}", if pending.is_empty() { "wj> " } else { "... " });
                let _ = std::io::stdout().flush();
            }
            let mut line = String::new();
            match stdin.lock().read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let mut t = line.trim_end().to_string();
            if !pending.is_empty() && t.trim().is_empty() {
                // a blank line ends an input that still looks unfinished, so the parser reports what is wrong with it
                s.eval(&pending);
                pending.clear();
                continue;
            }
            if pending.is_empty() {
                if t.trim().is_empty() {
                    continue;
                }
                match command(&t) {
                    Some(Command::Exit) => break,
                    Some(Command::Help) => {
                        println!("{HELP}");
                        continue;
                    }
                    Some(Command::Reset) => {
                        s.forget(fresh(scratch.as_ref()));
                        continue;
                    }
                    Some(Command::History) => {
                        s.list_history();
                        continue;
                    }
                    Some(Command::Recall(n)) => {
                        let at = n.unwrap_or(s.history.len());
                        match at.checked_sub(1).and_then(|i| s.history.get(i)) {
                            Some(h) => t = h.clone(),
                            None => {
                                eprintln!("no input {at} in the history ({} so far)", s.history.len());
                                continue;
                            }
                        }
                    }
                    None => {}
                }
            }
            pending.push_str(&t);
            pending.push('\n');
            if needs_more(&pending) {
                continue;
            }
            s.eval(&pending);
            pending.clear();
        }
        if !pending.trim().is_empty() {
            s.eval(&pending);
        }
        0
    })
}
