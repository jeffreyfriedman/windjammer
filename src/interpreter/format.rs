//! `format!`-style rendering: Display, Debug (incl. `{:#?}`), width/fill/precision/radix specs.

use super::engine::Interpreter;
use super::error::{semantic, R};
use super::int::IntTy;
use super::place::deref_value;
use super::value::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default, Debug)]
struct Spec {
    fill: Option<char>,
    align: Option<char>,
    plus: bool,
    alt: bool,
    zero: bool,
    width: Option<usize>,
    precision: Option<usize>,
    ty: String,
}

fn is_align(c: char) -> bool {
    matches!(c, '<' | '>' | '^')
}

impl Interpreter {
    pub(crate) fn format_args(&mut self, args: &[Value]) -> R<String> {
        let Some(first) = args.first() else { return Ok(String::new()) };
        let fmt = match deref_value(first.clone())? {
            Value::Str(s) => s,
            other => return Ok(self.display_string(&other)?),
        };
        let mut out = String::new();
        let mut next = 1usize;
        // arguments referenced by the template; rustc rejects a template that leaves one unused
        let mut used = vec![false; args.len()];
        used[0] = true;
        let chars: Vec<char> = fmt.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '{' {
                if chars.get(i + 1) == Some(&'{') {
                    out.push('{');
                    i += 2;
                    continue;
                }
                let start = i + 1;
                let mut j = start;
                while j < chars.len() && chars[j] != '}' {
                    j += 1;
                }
                if j >= chars.len() {
                    return Err(semantic("invalid format string: unmatched `{`"));
                }
                let inner: String = chars[start..j].iter().collect();
                i = j + 1;
                let (arg, spec_s) = match inner.split_once(':') {
                    Some((a, s)) => (a.trim().to_string(), s.to_string()),
                    None => (inner.trim().to_string(), String::new()),
                };
                let mut spec = parse_spec(&spec_s);
                // width/precision from arguments
                let resolve_count = |this: &mut Interpreter, s: &str, next: &mut usize, used: &mut [bool]| -> R<Option<usize>> {
                    if s == "*" {
                        let v = args.get(*next).cloned();
                        if let Some(u) = used.get_mut(*next) {
                            *u = true;
                        }
                        *next += 1;
                        return Ok(match v {
                            Some(Value::Int(n, _)) => Some(n as usize),
                            _ => None,
                        });
                    }
                    let name = s.trim_end_matches('$');
                    if let Ok(idx) = name.parse::<usize>() {
                        if let Some(u) = used.get_mut(idx + 1) {
                            *u = true;
                        }
                        return Ok(match args.get(idx + 1) {
                            Some(Value::Int(n, _)) => Some(*n as usize),
                            _ => None,
                        });
                    }
                    match this.lookup_var(name) {
                        Some(slot) => Ok(match &*slot.borrow() {
                            Value::Int(n, _) => Some(*n as usize),
                            _ => None,
                        }),
                        None => Err(semantic(format!("format width `{}` is not defined", name))),
                    }
                };
                if let Some(w) = spec_width_arg(&spec_s) {
                    spec.width = resolve_count(self, &w, &mut next, &mut used)?;
                }
                if let Some(p) = spec_prec_arg(&spec_s) {
                    spec.precision = resolve_count(self, &p, &mut next, &mut used)?;
                }
                let v = if arg.is_empty() {
                    let v = args.get(next).cloned();
                    if let Some(u) = used.get_mut(next) {
                        *u = true;
                    }
                    next += 1;
                    v.ok_or_else(|| semantic("format string has more `{}` than arguments"))?
                } else if let Ok(idx) = arg.parse::<usize>() {
                    if let Some(u) = used.get_mut(idx + 1) {
                        *u = true;
                    }
                    args.get(idx + 1).cloned().ok_or_else(|| semantic("format argument index out of range"))?
                } else {
                    match self.lookup_var(&arg) {
                        Some(slot) => slot.borrow().clone(),
                        None => match self.eval_path_value(&arg) {
                            Ok(v) => v,
                            Err(_) => return Err(semantic(format!("cannot find value `{}` in this scope", arg))),
                        },
                    }
                };
                let v = deref_value(v)?;
                let body = self.render_with_spec(&v, &spec)?;
                out.push_str(&pad(&body, &spec, is_numeric(&v)));
            } else if c == '}' {
                if chars.get(i + 1) == Some(&'}') {
                    i += 2;
                } else {
                    i += 1;
                }
                out.push('}');
            } else {
                out.push(c);
                i += 1;
            }
        }
        if used.iter().any(|u| !u) {
            return Err(semantic("format string leaves an argument unused (rustc: argument never used)"));
        }
        Ok(out)
    }

    fn render_with_spec(&mut self, v: &Value, spec: &Spec) -> R<String> {
        match spec.ty.as_str() {
            "" => {
                let s = match (v, spec.precision) {
                    (Value::Float(f, t), Some(p)) => {
                        if *t == FloatTy::F32 { format!("{:.*}", p, *f as f32) } else { format!("{:.*}", p, f) }
                    }
                    (Value::Str(s), Some(p)) => s.chars().take(p).collect(),
                    _ => self.display_string(v)?,
                };
                Ok(sign(s, v, spec))
            }
            "?" => {
                let s = match (v, spec.precision) {
                    (Value::Float(f, _), Some(p)) => format!("{:.*?}", p, f),
                    _ => {
                        let mut out = String::new();
                        self.debug_fmt(v, spec.alt, 0, &mut out)?;
                        out
                    }
                };
                Ok(sign(s, v, spec))
            }
            "x" | "X" | "b" | "o" => {
                let Value::Int(n, t) = v else {
                    return Err(semantic(format!("`{{:{}}}` needs an integer", spec.ty)));
                };
                let u: u128 = if *n < 0 { (t.wrap(*n) + (1i128 << t.bits().min(127))) as u128 } else { *n as u128 };
                let u: u128 = if *n < 0 && t.bits() == 128 { *n as u128 } else { u };
                let (digits, prefix) = match spec.ty.as_str() {
                    "x" => (format!("{:x}", u), "0x"),
                    "X" => (format!("{:X}", u), "0x"),
                    "b" => (format!("{:b}", u), "0b"),
                    _ => (format!("{:o}", u), "0o"),
                };
                Ok(if spec.alt { format!("{}{}", prefix, digits) } else { digits })
            }
            "e" | "E" => match v {
                Value::Float(f, _) => Ok(if spec.ty == "e" { format!("{:e}", f) } else { format!("{:E}", f) }),
                Value::Int(n, _) => Ok(if spec.ty == "e" { format!("{:e}", n) } else { format!("{:E}", n) }),
                _ => Err(semantic("`{:e}` needs a number")),
            },
            other => Err(semantic(format!("unknown format trait `{}`", other))),
        }
    }

    pub(crate) fn display_string(&mut self, v: &Value) -> R<String> {
        match v {
            Value::Unit => Ok("()".into()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Int(n, IntTy::U128) => Ok(super::u128ops::display(*n)),
            Value::Int(n, _) => Ok(n.to_string()),
            Value::Float(f, FloatTy::F32) => Ok(format!("{}", *f as f32)),
            Value::Float(f, _) => Ok(format!("{}", f)),
            Value::Char(c) => Ok(c.to_string()),
            Value::Str(s) => Ok((**s).clone()),
            Value::Ref(p) => {
                let inner = p.read()?;
                self.display_string(&inner)
            }
            Value::Rc(slot) => {
                let inner = slot.borrow().clone();
                self.display_string(&inner)
            }
            Value::Native(n) if n.display().is_some() => Ok(n.display().unwrap_or_default()),
            // `char::to_lowercase()`/`to_uppercase()`/`escape_*` are char iterators that implement Display
            Value::Iter(i) => {
                let copy = super::iter::clone_iter(i);
                let items = self.drain_iter(&copy)?;
                let mut out = String::new();
                for x in &items {
                    match x {
                        Value::Char(c) => out.push(*c),
                        other => return Err(semantic(format!("`{}` iterator doesn't implement `std::fmt::Display`", other.type_name()))),
                    }
                }
                Ok(out)
            }
            Value::Struct(_) | Value::Enum(_) => {
                let tn = v.type_name();
                let f = self
                    .methods
                    .get(&(tn.clone(), "Display::fmt".to_string()))
                    .or_else(|| self.methods.get(&(tn.clone(), "fmt".to_string())))
                    .cloned();
                if let Some(f) = f {
                    let buf = Rc::new(RefCell::new(String::new()));
                    self.call_func(&f, vec![Value::Formatter(buf.clone())], Some(v.clone()))?;
                    let s = buf.borrow().clone();
                    return Ok(s);
                }
                // ADR-0015 #3: a user `fn to_string(self) -> string` makes the type printable.
                if let Some(f) = self.methods.get(&(tn.clone(), "to_string".to_string())).cloned() {
                    return match self.call_func(&f, vec![], Some(v.clone()))? {
                        Value::Str(s) => Ok((*s).clone()),
                        other => self.display_string(&other),
                    };
                }
                Err(semantic(format!("`{}` doesn't implement `std::fmt::Display`", tn)))
            }
            other => Err(semantic(format!("`{}` doesn't implement `std::fmt::Display`", other.type_name()))),
        }
    }

    pub(crate) fn debug_string(&mut self, v: &Value) -> R<String> {
        let mut s = String::new();
        self.debug_fmt(v, false, 0, &mut s)?;
        Ok(s)
    }

    fn user_debug(&mut self, v: &Value) -> R<Option<String>> {
        let tn = v.type_name();
        if let Some(f) = self.methods.get(&(tn, "Debug::fmt".to_string())).cloned() {
            let buf = Rc::new(RefCell::new(String::new()));
            self.call_func(&f, vec![Value::Formatter(buf.clone())], Some(v.clone()))?;
            let s = buf.borrow().clone();
            return Ok(Some(s));
        }
        Ok(None)
    }

    pub(crate) fn debug_fmt(&mut self, v: &Value, pretty: bool, ind: usize, out: &mut String) -> R<()> {
        let pad = |n: usize| "    ".repeat(n);
        match v {
            Value::Unit => out.push_str("()"),
            Value::Bool(b) => out.push_str(&b.to_string()),
            Value::Int(n, IntTy::U128) => out.push_str(&super::u128ops::display(*n)),
            Value::Int(n, _) => out.push_str(&n.to_string()),
            Value::Float(f, FloatTy::F32) => out.push_str(&format!("{:?}", *f as f32)),
            Value::Float(f, _) => out.push_str(&format!("{:?}", f)),
            Value::Char(c) => out.push_str(&format!("{:?}", c)),
            Value::Str(s) => out.push_str(&format!("{:?}", s.as_str())),
            Value::Ref(p) => {
                let inner = p.read()?;
                self.debug_fmt(&inner, pretty, ind, out)?;
            }
            Value::Vec(items) => self.debug_seq("[", "]", items.iter(), pretty, ind, out)?,
            Value::Tuple(items) => {
                if items.len() == 1 && !pretty {
                    out.push('(');
                    self.debug_fmt(&items[0], pretty, ind, out)?;
                    out.push_str(",)");
                } else {
                    self.debug_seq("(", ")", items.iter(), pretty, ind, out)?;
                }
            }
            Value::Set(m) => {
                let items: Vec<Value> = m.ordered_keys(self.hash_seed);
                self.debug_seq("{", "}", items.iter(), pretty, ind, out)?
            }
            Value::Map(m) => {
                if m.is_empty() {
                    out.push_str("{}");
                } else if pretty {
                    out.push_str("{\n");
                    for (k, val) in m.ordered(self.hash_seed) {
                        out.push_str(&pad(ind + 1));
                        self.debug_fmt(&k.0, true, ind + 1, out)?;
                        out.push_str(": ");
                        self.debug_fmt(val, true, ind + 1, out)?;
                        out.push_str(",\n");
                    }
                    out.push_str(&pad(ind));
                    out.push('}');
                } else {
                    out.push('{');
                    for (n, (k, val)) in m.ordered(self.hash_seed).into_iter().enumerate() {
                        if n > 0 {
                            out.push_str(", ");
                        }
                        self.debug_fmt(&k.0, false, ind, out)?;
                        out.push_str(": ");
                        self.debug_fmt(val, false, ind, out)?;
                    }
                    out.push('}');
                }
            }
            Value::Range(a, b, _) => out.push_str(&format!("{}..{}", a, b)),
            Value::Struct(s) => {
                if let Some(u) = self.user_debug(v)? {
                    out.push_str(&u);
                    return Ok(());
                }
                if s.fields.is_empty() {
                    out.push_str(&s.name);
                } else if s.fields.iter().enumerate().all(|(i, (n, _))| **n == i.to_string()) {
                    out.push_str(&s.name);
                    let items: Vec<Value> = s.fields.iter().map(|(_, v)| v.clone()).collect();
                    self.debug_seq("(", ")", items.iter(), pretty, ind, out)?;
                } else {
                    out.push_str(&s.name);
                    self.debug_fields(s.fields.iter().map(|(n, v)| (n.to_string(), v.clone())).collect(), pretty, ind, out)?;
                }
            }
            Value::Enum(e) => {
                if let Some(u) = self.user_debug(v)? {
                    out.push_str(&u);
                    return Ok(());
                }
                out.push_str(&e.variant);
                match &e.payload {
                    Payload::Unit => {}
                    Payload::Tuple(items) => self.debug_seq("(", ")", items.iter(), pretty, ind, out)?,
                    Payload::Struct(fs) => {
                        self.debug_fields(fs.iter().map(|(n, v)| (n.to_string(), v.clone())).collect(), pretty, ind, out)?
                    }
                }
            }
            Value::Rc(slot) => {
                let inner = slot.borrow().clone();
                self.debug_fmt(&inner, pretty, ind, out)?;
            }
            Value::Weak(_) => out.push_str("(Weak)"),
            Value::Native(n) if n.debug_entries().is_some() => {
                let (is_map, entries) = n.debug_entries().unwrap_or_default();
                if is_map {
                    let pairs = Value::vec(entries.into_iter().map(|(k, v)| Value::tuple(vec![k, v.unwrap_or(Value::Unit)])).collect());
                    out.push('{');
                    if let Value::Vec(ps) = &pairs {
                        for (i, p) in ps.iter().enumerate() {
                            if i > 0 {
                                out.push_str(", ");
                            }
                            if let Value::Tuple(t) = p {
                                self.debug_fmt(&t[0], false, ind, out)?;
                                out.push_str(": ");
                                self.debug_fmt(&t[1], false, ind, out)?;
                            }
                        }
                    }
                    out.push('}');
                } else {
                    let items: Vec<Value> = entries.into_iter().map(|(k, _)| k).collect();
                    self.debug_seq("{", "}", items.iter(), pretty, ind, out)?
                }
            }
            Value::Native(n) => out.push_str(&n.debug()),
            Value::Cell(CellKind::Atomic, slot, _) => {
                let inner = slot.borrow().clone();
                self.debug_fmt(&inner, pretty, ind, out)?;
            }
            Value::Cell(kind, slot, _) => {
                let inner = slot.borrow().clone();
                let (name, field) = match kind {
                    CellKind::RefCell => ("RefCell", "value"),
                    CellKind::Cell => ("Cell", "value"),
                    CellKind::Mutex => ("Mutex", "data"),
                    CellKind::RwLock => ("RwLock", "data"),
                    CellKind::Atomic => ("", ""),
                };
                out.push_str(name);
                let mut fields = vec![(field.to_string(), inner)];
                if matches!(kind, CellKind::Mutex | CellKind::RwLock) {
                    fields.push(("poisoned".to_string(), Value::Bool(false)));
                }
                self.debug_fields(fields, pretty, ind, out)?;
                if matches!(kind, CellKind::Mutex | CellKind::RwLock) {
                    // rustc prints `, .. }` after the poison flag
                    if pretty {
                        out.truncate(out.len() - (4 * ind + 1));
                        out.push_str(&"    ".repeat(ind + 1));
                        out.push_str("..
");
                        out.push_str(&"    ".repeat(ind));
                        out.push('}');
                    } else {
                        out.truncate(out.len() - 2);
                        out.push_str(", .. }");
                    }
                }
            }
            Value::Iter(_) => out.push_str("Iter { .. }"),
            Value::Func(_) => out.push_str("<function>"),
            Value::Formatter(_) => out.push_str("Formatter"),
        }
        Ok(())
    }

    fn debug_seq<'a>(
        &mut self,
        open: &str,
        close: &str,
        items: impl Iterator<Item = &'a Value>,
        pretty: bool,
        ind: usize,
        out: &mut String,
    ) -> R<()> {
        let items: Vec<&Value> = items.collect();
        out.push_str(open);
        if items.is_empty() {
            out.push_str(close);
            return Ok(());
        }
        if pretty {
            out.push('\n');
            for x in items {
                out.push_str(&"    ".repeat(ind + 1));
                self.debug_fmt(x, true, ind + 1, out)?;
                out.push_str(",\n");
            }
            out.push_str(&"    ".repeat(ind));
        } else {
            for (i, x) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                self.debug_fmt(x, false, ind, out)?;
            }
        }
        out.push_str(close);
        Ok(())
    }

    fn debug_fields(&mut self, fields: Vec<(String, Value)>, pretty: bool, ind: usize, out: &mut String) -> R<()> {
        if pretty {
            out.push_str(" {\n");
            for (n, v) in fields {
                out.push_str(&"    ".repeat(ind + 1));
                out.push_str(&n);
                out.push_str(": ");
                self.debug_fmt(&v, true, ind + 1, out)?;
                out.push_str(",\n");
            }
            out.push_str(&"    ".repeat(ind));
            out.push('}');
        } else {
            out.push_str(" { ");
            for (i, (n, v)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(n);
                out.push_str(": ");
                self.debug_fmt(v, false, ind, out)?;
            }
            out.push_str(" }");
        }
        Ok(())
    }
}

fn is_numeric(v: &Value) -> bool {
    matches!(v, Value::Int(..) | Value::Float(..))
}

fn sign(s: String, v: &Value, spec: &Spec) -> String {
    if spec.plus && is_numeric(v) && !s.starts_with('-') {
        format!("+{}", s)
    } else {
        s
    }
}

fn pad(body: &str, spec: &Spec, numeric: bool) -> String {
    let Some(w) = spec.width else { return body.to_string() };
    let len = body.chars().count();
    if len >= w {
        return body.to_string();
    }
    let fill_n = w - len;
    if spec.zero && numeric && spec.align.is_none() {
        let (sgn, rest) = match body.chars().next() {
            Some(c @ ('-' | '+')) => (c.to_string(), &body[1..]),
            _ => (String::new(), body),
        };
        let (prefix, digits) = if rest.starts_with("0x") || rest.starts_with("0b") || rest.starts_with("0o") {
            (&rest[..2], &rest[2..])
        } else {
            ("", rest)
        };
        return format!("{}{}{}{}", sgn, prefix, "0".repeat(fill_n), digits);
    }
    let fill = spec.fill.unwrap_or(' ');
    let align = spec.align.unwrap_or(if numeric { '>' } else { '<' });
    let f = |n: usize| fill.to_string().repeat(n);
    match align {
        '<' => format!("{}{}", body, f(fill_n)),
        '>' => format!("{}{}", f(fill_n), body),
        _ => {
            let l = fill_n / 2;
            format!("{}{}{}", f(l), body, f(fill_n - l))
        }
    }
}

fn spec_width_arg(s: &str) -> Option<String> {
    // a `name$` / `N$` width appears before an optional `.`
    let head = s.split('.').next().unwrap_or("");
    let idx = head.find('$')?;
    let start = head[..idx].rfind(|c: char| !(c.is_alphanumeric() || c == '_')).map(|i| i + 1).unwrap_or(0);
    Some(format!("{}$", &head[start..idx]))
}

fn spec_prec_arg(s: &str) -> Option<String> {
    let (_, p) = s.split_once('.')?;
    if p.starts_with('*') {
        return Some("*".to_string());
    }
    let idx = p.find('$')?;
    Some(format!("{}$", &p[..idx]))
}

fn parse_spec(s: &str) -> Spec {
    let mut sp = Spec::default();
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    if c.len() >= 2 && is_align(c[1]) {
        sp.fill = Some(c[0]);
        sp.align = Some(c[1]);
        i = 2;
    } else if !c.is_empty() && is_align(c[0]) {
        sp.align = Some(c[0]);
        i = 1;
    }
    if i < c.len() && c[i] == '+' {
        sp.plus = true;
        i += 1;
    }
    if i < c.len() && c[i] == '#' {
        sp.alt = true;
        i += 1;
    }
    if i < c.len() && c[i] == '0' && !(i + 1 < c.len() && c[i + 1] == '$') {
        sp.zero = true;
        i += 1;
    }
    let mut w = String::new();
    while i < c.len() && (c[i].is_ascii_digit()) {
        w.push(c[i]);
        i += 1;
    }
    if i < c.len() && c[i] == '$' {
        // positional/named width handled by spec_width_arg
        i += 1;
        w.clear();
    } else if i < c.len() && (c[i].is_alphabetic() || c[i] == '_') {
        // maybe `name$` width
        let mut j = i;
        while j < c.len() && (c[j].is_alphanumeric() || c[j] == '_') {
            j += 1;
        }
        if j < c.len() && c[j] == '$' {
            i = j + 1;
        }
    }
    if !w.is_empty() {
        sp.width = w.parse().ok();
    }
    if i < c.len() && c[i] == '.' {
        i += 1;
        let mut p = String::new();
        if i < c.len() && c[i] == '*' {
            i += 1;
        } else {
            while i < c.len() && (c[i].is_ascii_digit()) {
                p.push(c[i]);
                i += 1;
            }
            if i < c.len() && (c[i].is_alphabetic() || c[i] == '_') {
                let mut j = i;
                while j < c.len() && (c[j].is_alphanumeric() || c[j] == '_') {
                    j += 1;
                }
                if j < c.len() && c[j] == '$' {
                    i = j + 1;
                }
            } else if i < c.len() && c[i] == '$' {
                i += 1;
            }
        }
        if !p.is_empty() {
            sp.precision = p.parse().ok();
        }
    }
    let rest: String = c[i..].iter().collect();
    sp.ty = rest;
    if sp.ty == "#?" {
        sp.alt = true;
        sp.ty = "?".into();
    }
    sp
}
