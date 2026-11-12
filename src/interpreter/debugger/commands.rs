//! The command loop of the debugger: what each command of the script does at a stop.

use super::script::{is_name, Cmd};
use super::{format_quietly, readonly, Breakpoint, Debugger, Next, Resume, Watch};
use crate::interpreter::engine::Interpreter;
use serde_json::{json, Value as Json};

const ENDED: &str = "the program has exited; `back` goes back into it";

impl Debugger {
    /// Carry out commands until one resumes the run. `here` is the stopped step; `None` after the program ended
    /// (`steps` statements ran), when only `break` and `back` still apply.
    pub(crate) fn commands(&mut self, it: &mut Interpreter, here: Option<u64>, steps: u64) -> Next {
        loop {
            let Some(line) = self.cmds.get(self.next_cmd).cloned() else {
                self.script_ended = true;
                return Next::End;
            };
            self.next_cmd += 1;
            let text = line.text.as_str();
            let cmd = line.cmd;
            match cmd {
                Cmd::Break { file, line, cond } => self.breakpoints.push(Breakpoint { file, line, cond }),
                Cmd::Back(n) => {
                    let current = here.unwrap_or(steps + 1);
                    if n >= current {
                        self.error(text, format!("cannot go back {n} from step {current}: the run starts at step 1"));
                        continue;
                    }
                    self.run_to_reason = "back";
                    return Next::Restart(current - n);
                }
                Cmd::Continue | Cmd::Step(_) | Cmd::Next | Cmd::Finish if here.is_none() => self.error(text, ENDED),
                Cmd::Print(_) | Cmd::Watch(_) | Cmd::Locals | Cmd::Where if here.is_none() => self.error(text, ENDED),
                Cmd::Continue | Cmd::Step(_) | Cmd::Next | Cmd::Finish => {
                    self.resume = self.resume_for(it, &cmd);
                    return Next::Resume;
                }
                Cmd::Print(arg) => self.print(it, text, &arg),
                Cmd::Watch(name) => self.watch(it, name),
                Cmd::Locals => self.locals(it),
                Cmd::Where => self.stack(it),
            }
        }
    }

    fn resume_for(&mut self, it: &Interpreter, cmd: &Cmd) -> Resume {
        match cmd {
            Cmd::Step(n) => Resume::Steps(*n),
            Cmd::Next => Resume::Over { depth: it.backtrace.len(), task: it.sched.current },
            Cmd::Finish => {
                self.finishing = it.backtrace.last().map(|(n, _)| n.to_string());
                self.finish_value = None;
                Resume::Finish { depth: it.backtrace.len() }
            }
            _ => Resume::Continue,
        }
    }

    /// `print NAME` reads a binding; anything else is a read-only expression.
    fn print(&mut self, it: &mut Interpreter, text: &str, arg: &str) {
        let value = if is_name(arg) {
            match it.lookup_var(arg) {
                None => return self.error(text, format!("no binding `{arg}` in the current frame")),
                Some(slot) => {
                    let v = slot.borrow().clone();
                    format_quietly(it, &v)
                }
            }
        } else {
            readonly::eval(it, arg).and_then(|v| format_quietly(it, &v))
        };
        match value {
            Ok(v) => self.events.push(json!({ "event": "value", "name": arg, "value": v })),
            Err(m) => self.error(text, m),
        }
    }

    /// `watch NAME` in the stopped frame: from now on the run stops when the local's value changes.
    fn watch(&mut self, it: &mut Interpreter, name: String) {
        let function = it.backtrace.last().map_or("<top>".to_string(), |(n, _)| n.to_string());
        let now = Self::local_text(it, &name);
        self.events.push(json!({ "event": "watching", "name": name, "function": function, "value": now }));
        self.watches.retain(|w| !(w.name == name && w.function == function));
        self.watches.push(Watch { name, function, depth: it.backtrace.len(), active: true, last: now });
    }

    fn locals(&mut self, it: &mut Interpreter) {
        let vars: Vec<(&'static str, crate::interpreter::value::Slot)> = it.scope.vars.clone();
        let mut bindings: Vec<Json> = Vec::new();
        for (i, (name, slot)) in vars.iter().enumerate() {
            // a shadowed binding is not visible: keep the last of each name
            if vars[i + 1..].iter().any(|(n, _)| n == name) {
                continue;
            }
            let v = slot.borrow().clone();
            let text = format_quietly(it, &v).unwrap_or_else(|m| format!("<{m}>"));
            bindings.push(json!({ "name": name, "value": text }));
        }
        let function = it.backtrace.last().map_or("<top>".to_string(), |(n, _)| n.to_string());
        self.events.push(json!({ "event": "locals", "function": function, "bindings": bindings }));
    }

    fn stack(&mut self, it: &Interpreter) {
        let mut frames = Vec::new();
        let mut at = self.here.map(|p| (p.loc, p.line, p.column));
        for (name, site) in it.backtrace.iter().rev() {
            let mut f = json!({ "function": name.to_string() });
            if let Some((l, line, column)) = at {
                f["file"] = json!(self.file_name(l));
                f["line"] = json!(line);
                f["column"] = json!(column);
            }
            frames.push(f);
            at = site.map(|l| (l, l.line, l.display_column()));
        }
        self.events.push(json!({ "event": "stack", "frames": frames }));
    }
}
