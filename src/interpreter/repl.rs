//! Persistent REPL session: definitions and `let` bindings survive between inputs.

use super::engine::Interpreter;
use super::error::{ErrorKind, RuntimeError};
use super::loader::parse_source;
use crate::parser::Item;

/// Does this input start a top-level item (rather than a statement/expression)?
fn is_item_start(src: &str) -> bool {
    let s = src.trim_start();
    let s = s.strip_prefix("pub ").unwrap_or(s);
    ["fn ", "struct ", "enum ", "impl ", "impl<", "trait ", "use ", "const ", "static ", "type ", "mod ", "@"]
        .iter()
        .any(|k| s.starts_with(k))
}

/// Number of unclosed `{ ( [` in `src` (ignoring string/char literals and comments).
pub fn open_depth(src: &str) -> i32 {
    let mut depth = 0;
    let mut in_str = false;
    let mut prev = '\0';
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if in_str {
            if c == '"' && prev != '\\' {
                in_str = false;
            }
            prev = c;
            continue;
        }
        match c {
            '"' => in_str = true,
            '/' if chars.peek() == Some(&'/') => {
                while let Some(&n) = chars.peek() {
                    if n == '\n' {
                        break;
                    }
                    chars.next();
                }
            }
            '{' | '(' | '[' => depth += 1,
            '}' | ')' | ']' => depth -= 1,
            _ => {}
        }
        prev = c;
    }
    depth
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
fn new_repl_interpreter() -> Interpreter {
    let mut it = Interpreter::new();
    it.guard_stack();
    let here = std::env::current_dir().unwrap_or_default().join("repl.wj");
    if let Ok(gate) = super::caps::gate_for_run(&here, false, &[], false, "") {
        it.set_caps(gate.map(|g| g.for_tests()));
    }
    it
}

/// Interactive loop on stdin/stdout. Prompts are only printed when stdin is a terminal.
pub fn run_repl_stdio() -> i32 {
    use std::io::{BufRead, IsTerminal, Write};
    super::runner::with_big_stack(|| {
        let interactive = std::io::stdin().is_terminal();
        let mut it = new_repl_interpreter();
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
            let t = line.trim_end();
            if pending.is_empty() {
                match t.trim() {
                    "exit" | "quit" | ":q" => break,
                    "" => continue,
                    ":help" => {
                        println!(":help  this text\n:reset forget all definitions and variables\nexit   leave the REPL");
                        continue;
                    }
                    ":reset" => {
                        it = new_repl_interpreter();
                        continue;
                    }
                    _ => {}
                }
            }
            pending.push_str(t);
            pending.push('\n');
            if open_depth(&pending) > 0 {
                continue;
            }
            match it.repl_eval(&pending) {
                Ok(Some(s)) => println!("{}", s),
                Ok(None) => {}
                Err(e) => eprintln!("{}", e.render()),
            }
            pending.clear();
        }
        0
    })
}
