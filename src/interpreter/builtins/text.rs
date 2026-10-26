//! String (`String` / `&str`) methods.

use super::*;

enum Pat {
    Str(String),
    Chars(Vec<char>),
    Closure(Value),
}

fn pat(v: Value) -> R<Pat> {
    match super::super::place::deref_value(v)? {
        Value::Str(s) => Ok(Pat::Str((*s).clone())),
        Value::Char(c) => Ok(Pat::Str(c.to_string())),
        Value::Vec(items) => Ok(Pat::Chars(items.iter().filter_map(|x| if let Value::Char(c) = x { Some(*c) } else { None }).collect())),
        f @ Value::Func(_) => Ok(Pat::Closure(f)),
        other => Err(semantic(format!("unsupported string pattern of type {}", other.type_name()))),
    }
}

/// Byte ranges (start, end) of every non-overlapping match, left to right.
fn matches(it: &mut Interpreter, s: &str, p: &Pat) -> R<Vec<(usize, usize)>> {
    let mut out = Vec::new();
    match p {
        Pat::Str(pt) => {
            if pt.is_empty() {
                // Rust: matches at every char boundary
                for (i, _) in s.char_indices() {
                    out.push((i, i));
                }
                out.push((s.len(), s.len()));
            } else {
                let mut from = 0;
                while let Some(i) = s[from..].find(pt.as_str()) {
                    out.push((from + i, from + i + pt.len()));
                    from += i + pt.len();
                }
            }
        }
        Pat::Chars(cs) => {
            for (i, c) in s.char_indices() {
                if cs.contains(&c) {
                    out.push((i, i + c.len_utf8()));
                }
            }
        }
        Pat::Closure(f) => {
            for (i, c) in s.char_indices() {
                if matches!(it.call_value(f, vec![Value::Char(c)])?, Value::Bool(true)) {
                    out.push((i, i + c.len_utf8()));
                }
            }
        }
    }
    Ok(out)
}

fn split_parts(it: &mut Interpreter, s: &str, p: &Pat, limit: Option<usize>) -> R<Vec<String>> {
    let ms = matches(it, s, p)?;
    let mut parts = Vec::new();
    let mut last = 0;
    if let Pat::Str(pt) = p {
        if pt.is_empty() {
            // "abc".split("") == ["", "a", "b", "c", ""]
            let mut v = vec![String::new()];
            v.extend(s.chars().map(|c| c.to_string()));
            v.push(String::new());
            return Ok(v);
        }
    }
    for (a, b) in ms {
        if let Some(l) = limit {
            if parts.len() + 1 >= l {
                break;
            }
        }
        parts.push(s[last..a].to_string());
        last = b;
    }
    parts.push(s[last..].to_string());
    Ok(parts)
}

fn strs(v: Vec<String>) -> Value {
    new_iter(v.into_iter().map(Value::str).collect())
}

fn sv(v: &Value) -> Rc<String> {
    match v {
        Value::Str(s) => s.clone(),
        _ => Rc::new(String::new()),
    }
}

fn parse_target(it: &mut Interpreter, text: &str) -> Value {
    let hint = it.type_hint.take();
    let t = text;
    let int_ty = hint.as_ref().and_then(IntTy::from_type);
    if int_ty == Some(IntTy::U128) {
        let body = t.strip_prefix('+').unwrap_or(t);
        return match body.parse::<u128>() {
            Ok(n) => Value::ok(Value::Int(n as i128, IntTy::U128)),
            Err(e) => Value::err(Value::str(e.to_string())),
        };
    }
    if let Some(ty) = int_ty {
        let parsed = if t.starts_with('+') { t[1..].parse::<i128>() } else { t.parse::<i128>() };
        return match parsed {
            Ok(n) if ty.fits(n) && !(t.starts_with('-') && !ty.signed()) => Value::ok(Value::Int(n, ty)),
            Ok(_) => Value::err(Value::str("number too large or too small to fit in target type")),
            Err(e) => Value::err(Value::str(e.to_string())),
        };
    }
    match hint {
        Some(ref h) if super::super::value::type_is_float(h).is_some() => {
            let ft = super::super::value::type_is_float(h).unwrap();
            match t.parse::<f64>() {
                Ok(f) if !t.is_empty() => Value::ok(Value::Float(if ft == FloatTy::F32 { f as f32 as f64 } else { f }, ft)),
                _ => Value::err(Value::str("invalid float literal")),
            }
        }
        Some(crate::parser::Type::Bool) => match t {
            "true" => Value::ok(Value::Bool(true)),
            "false" => Value::ok(Value::Bool(false)),
            _ => Value::err(Value::str("provided string was not `true` or `false`")),
        },
        Some(crate::parser::Type::Custom(ref n)) if n == "char" => {
            let mut cs = t.chars();
            match (cs.next(), cs.next()) {
                (Some(c), None) => Value::ok(Value::Char(c)),
                (None, _) => Value::err(Value::str("cannot parse char from empty string")),
                _ => Value::err(Value::str("too many characters in string")),
            }
        }
        Some(crate::parser::Type::String) => Value::ok(Value::str(t.to_string())),
        _ => {
            // no type information: integer if it looks like one, else float
            if let Ok(n) = t.parse::<i128>() {
                Value::ok(Value::int(n))
            } else if let Ok(f) = t.parse::<f64>() {
                Value::ok(Value::Float(f, FloatTy::Lit))
            } else {
                Value::err(Value::str("invalid digit found in string"))
            }
        }
    }
}

pub fn register(r: &mut Registry) {
    r.pure(Kind::Str, "slice", |it, v, a| {
        let (x, y) = (arg_int(&a, 0, "slice")?, arg_int(&a, 1, "slice")?);
        it.index_value(v, &Value::Range(x, y, IntTy::Usize))
    });
    r.pure(Kind::Str, "len", |_, v, _| Ok(Value::usize(sv(v).len())));
    r.pure(Kind::Str, "is_empty", |_, v, _| Ok(Value::Bool(sv(v).is_empty())));
    r.pure(Kind::Str, "to_string", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Str, "to_owned", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Str, "as_str", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Str, "into_string", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Str, "to_uppercase", |_, v, _| Ok(Value::str(sv(v).to_uppercase())));
    r.pure(Kind::Str, "to_lowercase", |_, v, _| Ok(Value::str(sv(v).to_lowercase())));
    r.pure(Kind::Str, "to_ascii_uppercase", |_, v, _| Ok(Value::str(sv(v).to_ascii_uppercase())));
    r.pure(Kind::Str, "to_ascii_lowercase", |_, v, _| Ok(Value::str(sv(v).to_ascii_lowercase())));
    r.pure(Kind::Str, "trim", |_, v, _| Ok(Value::str(sv(v).trim())));
    r.pure(Kind::Str, "trim_start", |_, v, _| Ok(Value::str(sv(v).trim_start())));
    r.pure(Kind::Str, "trim_end", |_, v, _| Ok(Value::str(sv(v).trim_end())));
    r.pure(Kind::Str, "chars", |_, v, _| Ok(new_iter(sv(v).chars().map(Value::Char).collect())));
    r.pure(Kind::Str, "bytes", |_, v, _| Ok(new_iter(sv(v).bytes().map(|b| Value::Int(b as i128, IntTy::U8)).collect())));
    r.pure(Kind::Str, "as_bytes", |_, v, _| Ok(Value::vec(sv(v).bytes().map(|b| Value::Int(b as i128, IntTy::U8)).collect())));
    r.pure(Kind::Str, "into_bytes", |_, v, _| Ok(Value::vec(sv(v).bytes().map(|b| Value::Int(b as i128, IntTy::U8)).collect())));
    r.pure(Kind::Str, "char_indices", |_, v, _| {
        Ok(new_iter(sv(v).char_indices().map(|(i, c)| Value::tuple(vec![Value::usize(i), Value::Char(c)])).collect()))
    });
    r.pure(Kind::Str, "lines", |_, v, _| Ok(strs(sv(v).lines().map(|l| l.to_string()).collect())));
    r.pure(Kind::Str, "split_whitespace", |_, v, _| Ok(strs(sv(v).split_whitespace().map(|l| l.to_string()).collect())));
    r.pure(Kind::Str, "split_ascii_whitespace", |_, v, _| Ok(strs(sv(v).split_whitespace().map(|l| l.to_string()).collect())));
    r.pure(Kind::Str, "split", |it, v, a| {
        let p = pat(arg(&a, 0, "split")?)?;
        Ok(strs(split_parts(it, &sv(v), &p, None)?))
    });
    r.pure(Kind::Str, "splitn", |it, v, a| {
        let n = arg_usize(&a, 0, "splitn")?;
        let p = pat(arg(&a, 1, "splitn")?)?;
        Ok(strs(split_parts(it, &sv(v), &p, Some(n))?))
    });
    r.pure(Kind::Str, "rsplit", |it, v, a| {
        let p = pat(arg(&a, 0, "rsplit")?)?;
        let mut parts = split_parts(it, &sv(v), &p, None)?;
        parts.reverse();
        Ok(strs(parts))
    });
    r.pure(Kind::Str, "split_terminator", |it, v, a| {
        let p = pat(arg(&a, 0, "split_terminator")?)?;
        let mut parts = split_parts(it, &sv(v), &p, None)?;
        if parts.last().map(|s| s.is_empty()).unwrap_or(false) {
            parts.pop();
        }
        Ok(strs(parts))
    });
    r.pure(Kind::Str, "split_once", |it, v, a| {
        let s = sv(v);
        let p = pat(arg(&a, 0, "split_once")?)?;
        let ms = matches(it, &s, &p)?;
        Ok(match ms.first() {
            Some((x, y)) => Value::some(Value::tuple(vec![Value::str(&s[..*x]), Value::str(&s[*y..])])),
            None => Value::none(),
        })
    });
    r.pure(Kind::Str, "rsplit_once", |it, v, a| {
        let s = sv(v);
        let p = pat(arg(&a, 0, "rsplit_once")?)?;
        let ms = matches(it, &s, &p)?;
        Ok(match ms.last() {
            Some((x, y)) => Value::some(Value::tuple(vec![Value::str(&s[..*x]), Value::str(&s[*y..])])),
            None => Value::none(),
        })
    });
    r.pure(Kind::Str, "contains", |it, v, a| {
        let p = pat(arg(&a, 0, "contains")?)?;
        Ok(Value::Bool(!matches(it, &sv(v), &p)?.is_empty()))
    });
    r.pure(Kind::Str, "starts_with", |it, v, a| {
        let s = sv(v);
        let p = pat(arg(&a, 0, "starts_with")?)?;
        Ok(Value::Bool(matches(it, &s, &p)?.first().map(|(x, _)| *x == 0).unwrap_or(false)
            || matches!(&p, Pat::Str(t) if t.is_empty())))
    });
    r.pure(Kind::Str, "ends_with", |it, v, a| {
        let s = sv(v);
        let p = pat(arg(&a, 0, "ends_with")?)?;
        if let Pat::Str(t) = &p {
            return Ok(Value::Bool(s.ends_with(t.as_str())));
        }
        Ok(Value::Bool(matches(it, &s, &p)?.last().map(|(_, y)| *y == s.len()).unwrap_or(false)))
    });
    r.pure(Kind::Str, "find", |it, v, a| {
        let p = pat(arg(&a, 0, "find")?)?;
        Ok(match matches(it, &sv(v), &p)?.first() {
            Some((x, _)) => Value::some(Value::usize(*x)),
            None => Value::none(),
        })
    });
    r.pure(Kind::Str, "rfind", |it, v, a| {
        let s = sv(v);
        let p = pat(arg(&a, 0, "rfind")?)?;
        if let Pat::Str(t) = &p {
            return Ok(match s.rfind(t.as_str()) {
                Some(i) => Value::some(Value::usize(i)),
                None => Value::none(),
            });
        }
        Ok(match matches(it, &s, &p)?.last() {
            Some((x, _)) => Value::some(Value::usize(*x)),
            None => Value::none(),
        })
    });
    r.pure(Kind::Str, "matches", |it, v, a| {
        let s = sv(v);
        let p = pat(arg(&a, 0, "matches")?)?;
        let ms = matches(it, &s, &p)?;
        Ok(strs(ms.into_iter().map(|(x, y)| s[x..y].to_string()).collect()))
    });
    r.pure(Kind::Str, "replace", |it, v, a| {
        let s = sv(v);
        let p = pat(arg(&a, 0, "replace")?)?;
        let to = arg_str(&a, 1, "replace")?;
        let ms = matches(it, &s, &p)?;
        let mut out = String::new();
        let mut last = 0;
        for (x, y) in ms {
            out.push_str(&s[last..x]);
            out.push_str(&to);
            last = y;
        }
        out.push_str(&s[last..]);
        Ok(Value::str(out))
    });
    r.pure(Kind::Str, "replacen", |it, v, a| {
        let s = sv(v);
        let p = pat(arg(&a, 0, "replacen")?)?;
        let to = arg_str(&a, 1, "replacen")?;
        let n = arg_usize(&a, 2, "replacen")?;
        let ms = matches(it, &s, &p)?;
        let mut out = String::new();
        let mut last = 0;
        for (x, y) in ms.into_iter().take(n) {
            out.push_str(&s[last..x]);
            out.push_str(&to);
            last = y;
        }
        out.push_str(&s[last..]);
        Ok(Value::str(out))
    });
    r.pure(Kind::Str, "repeat", |_, v, a| Ok(Value::str(sv(v).repeat(arg_usize(&a, 0, "repeat")?))));
    r.pure(Kind::Str, "strip_prefix", |it, v, a| {
        let s = sv(v);
        let p = pat(arg(&a, 0, "strip_prefix")?)?;
        if let Pat::Str(t) = &p {
            return Ok(match s.strip_prefix(t.as_str()) {
                Some(rest) => Value::some(Value::str(rest)),
                None => Value::none(),
            });
        }
        Ok(match matches(it, &s, &p)?.first() {
            Some((0, y)) => Value::some(Value::str(&s[*y..])),
            _ => Value::none(),
        })
    });
    r.pure(Kind::Str, "strip_suffix", |_, v, a| {
        let s = sv(v);
        let t = arg_str(&a, 0, "strip_suffix")?;
        Ok(match s.strip_suffix(t.as_str()) {
            Some(rest) => Value::some(Value::str(rest)),
            None => Value::none(),
        })
    });
    r.pure(Kind::Str, "trim_matches", |it, v, a| {
        let s = sv(v);
        let p = pat(arg(&a, 0, "trim_matches")?)?;
        trim_with(it, &s, &p, true, true)
    });
    r.pure(Kind::Str, "trim_start_matches", |it, v, a| {
        let s = sv(v);
        let p = pat(arg(&a, 0, "trim_start_matches")?)?;
        trim_with(it, &s, &p, true, false)
    });
    r.pure(Kind::Str, "trim_end_matches", |it, v, a| {
        let s = sv(v);
        let p = pat(arg(&a, 0, "trim_end_matches")?)?;
        trim_with(it, &s, &p, false, true)
    });
    r.pure(Kind::Str, "parse", |it, v, _| Ok(parse_target(it, sv(v).trim_end_matches('\0'))));
    r.pure(Kind::Str, "get", |_, v, a| {
        let s = sv(v);
        Ok(match arg(&a, 0, "get")? {
            Value::Range(x, y, _) => {
                let (x, y) = (x as usize, y as usize);
                if x <= y && y <= s.len() && s.is_char_boundary(x) && s.is_char_boundary(y) {
                    Value::some(Value::str(&s[x..y]))
                } else {
                    Value::none()
                }
            }
            _ => Value::none(),
        })
    });
    r.pure(Kind::Str, "is_char_boundary", |_, v, a| Ok(Value::Bool(sv(v).is_char_boundary(arg_usize(&a, 0, "is_char_boundary")?))));
    r.pure(Kind::Str, "eq_ignore_ascii_case", |_, v, a| Ok(Value::Bool(sv(v).eq_ignore_ascii_case(&arg_str(&a, 0, "eq_ignore_ascii_case")?))));
    r.pure(Kind::Str, "char_count", |_, v, _| Ok(Value::usize(sv(v).chars().count())));
    r.pure(Kind::Str, "substring", |it, v, a| {
        // byte offsets, like slicing
        let (x, y) = (arg_int(&a, 0, "substring")?, arg_int(&a, 1, "substring")?);
        it.index_value(v, &Value::Range(x, y, IntTy::Usize))
    });
    r.pure(Kind::Str, "capacity", |_, v, _| Ok(Value::usize(sv(v).capacity())));

    r.mutating(Kind::Str, "push", |_, v, a| {
        let c = arg(&a, 0, "push")?;
        if let (Value::Str(s), Value::Char(c)) = (v, c) {
            Rc::make_mut(s).push(c);
        }
        Ok(Value::Unit)
    });
    r.mutating(Kind::Str, "push_str", |_, v, a| {
        let t = arg_str(&a, 0, "push_str")?;
        if let Value::Str(s) = v {
            Rc::make_mut(s).push_str(&t);
        }
        Ok(Value::Unit)
    });
    r.mutating(Kind::Str, "pop", |_, v, _| {
        if let Value::Str(s) = v {
            return Ok(Value::option(Rc::make_mut(s).pop().map(Value::Char)));
        }
        Ok(Value::none())
    });
    r.mutating(Kind::Str, "clear", |_, v, _| {
        if let Value::Str(s) = v {
            Rc::make_mut(s).clear();
        }
        Ok(Value::Unit)
    });
    r.mutating(Kind::Str, "truncate", |_, v, a| {
        let n = arg_usize(&a, 0, "truncate")?;
        if let Value::Str(s) = v {
            if n < s.len() {
                Rc::make_mut(s).truncate(n);
            }
        }
        Ok(Value::Unit)
    });
    r.mutating(Kind::Str, "insert", |_, v, a| {
        let i = arg_usize(&a, 0, "insert")?;
        let c = arg(&a, 1, "insert")?;
        if let (Value::Str(s), Value::Char(c)) = (v, c) {
            Rc::make_mut(s).insert(i, c);
        }
        Ok(Value::Unit)
    });
    r.mutating(Kind::Str, "insert_str", |_, v, a| {
        let i = arg_usize(&a, 0, "insert_str")?;
        let t = arg_str(&a, 1, "insert_str")?;
        if let Value::Str(s) = v {
            Rc::make_mut(s).insert_str(i, &t);
        }
        Ok(Value::Unit)
    });
    r.mutating(Kind::Str, "remove", |_, v, a| {
        let i = arg_usize(&a, 0, "remove")?;
        if let Value::Str(s) = v {
            if i >= s.len() {
                return Err(panic_ctl("cannot remove a char from the end of a string"));
            }
            return Ok(Value::Char(Rc::make_mut(s).remove(i)));
        }
        Ok(Value::Unit)
    });
    r.mutating(Kind::Str, "reserve", |_, _, _| Ok(Value::Unit));
}

fn trim_with(it: &mut Interpreter, s: &str, p: &Pat, start: bool, end: bool) -> R {
    let mut lo = 0;
    let mut hi = s.len();
    let test = |it: &mut Interpreter, c: char| -> R<bool> {
        match p {
            Pat::Str(t) => Ok(t.chars().count() == 1 && t.starts_with(c)),
            Pat::Chars(cs) => Ok(cs.contains(&c)),
            Pat::Closure(f) => Ok(matches!(it.call_value(f, vec![Value::Char(c)])?, Value::Bool(true))),
        }
    };
    if let Pat::Str(t) = p {
        if t.chars().count() != 1 {
            let mut cur = s;
            if start {
                while !t.is_empty() && cur.starts_with(t.as_str()) {
                    cur = &cur[t.len()..];
                }
            }
            if end {
                while !t.is_empty() && cur.ends_with(t.as_str()) {
                    cur = &cur[..cur.len() - t.len()];
                }
            }
            return Ok(Value::str(cur));
        }
    }
    if start {
        for (i, c) in s.char_indices() {
            if test(it, c)? {
                lo = i + c.len_utf8();
            } else {
                break;
            }
        }
    }
    if end {
        for (i, c) in s.char_indices().rev() {
            if i < lo {
                break;
            }
            if test(it, c)? {
                hi = i;
            } else {
                break;
            }
        }
    }
    Ok(Value::str(if lo <= hi { &s[lo..hi] } else { "" }))
}
