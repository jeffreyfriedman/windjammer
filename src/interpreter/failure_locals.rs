//! The live bindings of the failing frame, captured only when an error is raised (ADR-0036).
//!
//! Nothing here runs on the happy path: callers reach it from the `Err` arm of a function's result. Values are
//! rendered structurally (no user `Debug` impl runs, so capturing cannot have effects or fail again), truncated,
//! and redacted when they look like secrets. The decision is made by source first (a string equal to the
//! value of an environment variable, a `Secret*` type) and by a short list of binding names second; `--no-locals`
//! turns the capture off.

use super::error::Local;
use super::value::Value;

/// Bindings shown per error.
pub(crate) const MAX_LOCALS: usize = 8;
/// Characters shown per value.
const MAX_VALUE: usize = 80;
const MAX_ITEMS: usize = 4;
const MAX_DEPTH: usize = 3;
pub(crate) const REDACTED: &str = "<redacted>";

/// Binding or field names that hold credentials by convention (documented in docs/INTERPRETER.md).
const SECRET_NAMES: &[&str] = &["password", "passwd", "secret", "token", "api_key", "apikey", "credential", "private_key", "auth"];

/// What to hide: values that came from the process environment, names that look like credentials.
pub(crate) struct Redactor {
    env_values: Vec<String>,
}

impl Redactor {
    /// `env_values`: values of the process environment, handed over by the driver (`runner.rs`).
    pub(crate) fn new(env_values: &[String]) -> Redactor {
        Redactor { env_values: env_values.to_vec() }
    }

    fn name_is_secret(&self, name: &str) -> bool {
        let n = name.to_ascii_lowercase();
        SECRET_NAMES.iter().any(|s| n.contains(s))
    }

    fn value_is_secret(&self, v: &Value) -> bool {
        match v {
            Value::Str(s) => self.env_values.iter().any(|e| e == s.as_str()),
            Value::Struct(sv) => sv.name.starts_with("Secret"),
            Value::Enum(ev) => ev.ty.starts_with("Secret"),
            _ => false,
        }
    }
}

/// Render the last `MAX_LOCALS` distinct bindings of `vars` (declaration order, a shadowed name shows its newest
/// value). Returns them with the number left out. `Unit` values are skipped: in a compiled frame they are slots
/// that were never assigned.
pub(crate) fn capture(vars: impl Iterator<Item = (String, Value)>, env_values: &[String]) -> (Vec<Local>, usize) {
    let mut newest: Vec<(String, Value)> = Vec::new();
    for (name, v) in vars {
        if matches!(v, Value::Unit) || name.starts_with('$') {
            continue;
        }
        newest.retain(|(n, _)| *n != name);
        newest.push((name, v));
    }
    let omitted = newest.len().saturating_sub(MAX_LOCALS);
    let red = Redactor::new(env_values);
    let locals = newest
        .into_iter()
        .skip(omitted)
        .map(|(name, v)| {
            let redacted = red.name_is_secret(&name) || red.value_is_secret(&v);
            let value = if redacted { REDACTED.to_string() } else { cut(&brief(&v, 0, &red)) };
            Local { name, value, redacted }
        })
        .collect();
    (locals, omitted)
}

fn cut(s: &str) -> String {
    if s.chars().count() <= MAX_VALUE {
        return s.to_string();
    }
    let head: String = s.chars().take(MAX_VALUE).collect();
    format!("{head}...")
}

/// Debug-style text of `v`, bounded in depth and width. Never runs user code.
pub(crate) fn brief(v: &Value, depth: usize, red: &Redactor) -> String {
    if depth > MAX_DEPTH {
        return "..".to_string();
    }
    if red.value_is_secret(v) {
        return REDACTED.to_string();
    }
    match v {
        Value::Unit => "()".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i, _) => i.to_string(),
        Value::Float(f, _) => format!("{f:?}"),
        Value::Char(c) => format!("{c:?}"),
        Value::Str(s) => {
            let head: String = s.chars().take(MAX_VALUE).collect();
            let more = if s.chars().count() > MAX_VALUE { "..." } else { "" };
            format!("{:?}{more}", head)
        }
        Value::Vec(b) => {
            let n = b.len();
            let items: Vec<String> = (0..n.min(MAX_ITEMS)).filter_map(|i| b.get_value(i)).map(|x| brief(&x, depth + 1, red)).collect();
            list("[", "]", items, n)
        }
        Value::Tuple(t) => {
            let items: Vec<String> = t.iter().take(MAX_ITEMS).map(|x| brief(x, depth + 1, red)).collect();
            list("(", ")", items, t.len())
        }
        Value::Struct(s) => {
            let fields: Vec<String> = s.fields.iter().take(MAX_ITEMS).map(|(n, x)| field(n, x, depth, red)).collect();
            format!("{} {}", s.name, list("{ ", " }", fields, s.fields.len()))
        }
        Value::Enum(e) => {
            let head = if matches!(&*e.ty, "Option" | "Result") { e.variant.to_string() } else { format!("{}::{}", &*e.ty, &*e.variant) };
            match &e.payload {
                super::value::Payload::Unit => head,
                super::value::Payload::Tuple(xs) => {
                    let items: Vec<String> = xs.iter().take(MAX_ITEMS).map(|x| brief(x, depth + 1, red)).collect();
                    format!("{head}{}", list("(", ")", items, xs.len()))
                }
                super::value::Payload::Struct(fs) => {
                    let items: Vec<String> = fs.iter().take(MAX_ITEMS).map(|(n, x)| field(n, x, depth, red)).collect();
                    format!("{head} {}", list("{ ", " }", items, fs.len()))
                }
            }
        }
        Value::Map(m) => format!("map of {} entries", m.len()),
        Value::Set(m) => format!("set of {} entries", m.len()),
        Value::Range(r, _) => format!("{}..{}", r.0, r.1),
        Value::Rc(slot) => match slot.try_borrow() {
            Ok(inner) => format!("Rc({})", brief(&inner, depth + 1, red)),
            Err(_) => "Rc(<borrowed>)".to_string(),
        },
        Value::Cell(_, slot, _) => match slot.try_borrow() {
            Ok(inner) => format!("Cell({})", brief(&inner, depth + 1, red)),
            Err(_) => "Cell(<borrowed>)".to_string(),
        },
        // a reference prints as its target (Debug of `&T` is Debug of `T`)
        Value::Ref(p) => match p.slot.try_borrow() {
            Ok(_) => match p.read() {
                Ok(inner) => brief(&inner, depth + 1, red),
                Err(_) => "&<unreadable>".to_string(),
            },
            Err(_) => "&<borrowed>".to_string(),
        },
        Value::Weak(_) => "Weak".to_string(),
        Value::Func(_) => "<fn>".to_string(),
        Value::Iter(_) => "<iterator>".to_string(),
        Value::Formatter(_) => "<formatter>".to_string(),
        Value::Native(_) => "<host object>".to_string(),
    }
}

fn field(name: &str, v: &Value, depth: usize, red: &Redactor) -> String {
    let text = if red.name_is_secret(name) { REDACTED.to_string() } else { brief(v, depth + 1, red) };
    format!("{name}: {text}")
}

fn list(open: &str, close: &str, items: Vec<String>, total: usize) -> String {
    let more = if total > items.len() { format!(", .. {} more", total - items.len()) } else { String::new() };
    format!("{open}{}{more}{close}", items.join(", "))
}

impl super::engine::Interpreter {
    /// Bindings of the walker frame that is current now (call after the body failed, before the frame is popped).
    pub(crate) fn walker_locals(&self) -> (Vec<Local>, usize) {
        capture(self.scope.vars.iter().filter_map(|(n, slot)| slot.try_borrow().ok().map(|v| (n.to_string(), v.clone()))), &self.secret_env)
    }

    /// Fill in the locals of `e` the first time an error passes a frame: `names` are a compiled function's slots.
    pub(crate) fn attach_walker_locals(&self, e: &mut super::error::RuntimeError) {
        if self.capture_locals && e.backtrace.is_empty() && e.locals.is_empty() {
            let (l, more) = self.walker_locals();
            e.locals = l;
            e.locals_omitted = more;
        }
    }

    /// The same for a closure-engine frame whose slots start at `base` of the run stack.
    pub(crate) fn attach_slot_locals(&self, e: &mut super::error::RuntimeError, names: &[(&'static str, usize)], base: usize) {
        if self.capture_locals && e.backtrace.is_empty() && e.locals.is_empty() {
            let (l, more) = capture(names.iter().filter_map(|(n, s)| self.rstack.get(base + s).map(|v| (n.to_string(), v.clone()))), &self.secret_env);
            e.locals = l;
            e.locals_omitted = more;
        }
    }
}
