//! Interpreter-native language intrinsics (ADR-0014 Phase 1): stderr output, `panic`,
//! `assert*`, `todo`, `unreachable`, `debug`, plus `{}` / `{:.N}` / `{:?}` rendering of the
//! interpolation templates the parser produces. Output matches the compiled backends.

use super::engine::Interpreter;
use super::value::Value;
use crate::intrinsics::Intrinsic;
use std::io::Write;

impl<'a> Interpreter<'a> {
    /// Evaluate a registry intrinsic call. `None` = not an intrinsic handled here.
    pub(crate) fn eval_intrinsic_call(&mut self, name: &str, args: &[Value]) -> Option<Value> {
        let intr = Intrinsic::from_call_name(name)?;
        match intr {
            Intrinsic::Eprintln | Intrinsic::Eprint => {
                let text = self.format_string(args);
                if intr.appends_newline() {
                    eprintln!("{text}");
                } else {
                    eprint!("{text}");
                    let _ = std::io::stderr().flush();
                }
                Some(Value::Unit)
            }
            Intrinsic::Panic => {
                let msg = if args.is_empty() {
                    "explicit panic".to_string()
                } else {
                    self.format_string(args)
                };
                panic!("{msg}");
            }
            Intrinsic::Todo | Intrinsic::Unreachable => {
                let base = intr.default_panic_message().unwrap_or("panic");
                if args.is_empty() {
                    panic!("{base}");
                }
                panic!("{base}: {}", self.format_string(args));
            }
            Intrinsic::Assert => {
                let ok = args.first().map(|v| v.is_truthy()).unwrap_or(false);
                if !ok {
                    if args.len() > 1 {
                        panic!("{}", self.format_string(&args[1..]));
                    }
                    panic!("assertion failed");
                }
                Some(Value::Unit)
            }
            Intrinsic::AssertEq | Intrinsic::AssertNe if args.len() >= 2 => {
                let equal = args[0].to_display_string() == args[1].to_display_string();
                if equal != (intr == Intrinsic::AssertEq) {
                    let what = if intr == Intrinsic::AssertEq { "==" } else { "!=" };
                    let extra = if args.len() > 2 {
                        format!(": {}", self.format_string(&args[2..]))
                    } else {
                        String::new()
                    };
                    panic!(
                        "assertion `left {what} right` failed{extra}\n  left: {}\n right: {}",
                        args[0].to_display_string(),
                        args[1].to_display_string()
                    );
                }
                Some(Value::Unit)
            }
            Intrinsic::Debug if args.len() == 1 => {
                eprintln!("[debug] {}", debug_repr(&args[0]));
                Some(args[0].clone())
            }
            _ => None,
        }
    }

    /// Render a template (`args[0]`) with `{}`, `{:?}`, `{:.N}`, `{{`, `}}` placeholders.
    pub(crate) fn format_template(template: &str, args: &[Value]) -> String {
        let chars: Vec<char> = template.chars().collect();
        let mut out = String::new();
        let mut next = 0usize;
        let mut i = 0;
        while i < chars.len() {
            match chars[i] {
                '{' if chars.get(i + 1) == Some(&'{') => {
                    out.push('{');
                    i += 2;
                }
                '}' if chars.get(i + 1) == Some(&'}') => {
                    out.push('}');
                    i += 2;
                }
                '{' => {
                    let Some(rel) = chars[i..].iter().position(|&c| c == '}') else {
                        out.push('{');
                        i += 1;
                        continue;
                    };
                    let body: String = chars[i + 1..i + rel].iter().collect();
                    let spec = body.split_once(':').map(|(_, s)| s).unwrap_or("");
                    let Some(arg) = args.get(next) else {
                        out.push('{');
                        i += 1;
                        continue;
                    };
                    next += 1;
                    out.push_str(&render_value(arg, spec));
                    i += rel + 1;
                }
                c => {
                    out.push(c);
                    i += 1;
                }
            }
        }
        out
    }
}

fn debug_repr(v: &Value) -> String {
    match v {
        Value::String(s) => format!("{s:?}"),
        Value::Char(c) => format!("{c:?}"),
        other => other.to_display_string(),
    }
}

fn render_value(v: &Value, spec: &str) -> String {
    if spec == "?" || spec == "#?" {
        return debug_repr(v);
    }
    if let Some(p) = spec.strip_prefix('.') {
        let digits: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse::<usize>() {
            match v {
                Value::Float(f) => return format!("{f:.n$}"),
                Value::Int(i) => return format!("{:.n$}", *i as f64),
                _ => {}
            }
        }
    }
    v.to_display_string()
}
