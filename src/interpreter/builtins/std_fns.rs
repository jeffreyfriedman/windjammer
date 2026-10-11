//! Free functions, associated functions of builtin types, constants, and native `std` modules
//! (`std::fs`, `std::env`, `std::process`, `std::io`, `std::time`, `std::mem`, `std::cmp`).

use super::*;
use crate::interpreter::error::{ErrorKind, RuntimeError};
use std::collections::BTreeMap;

fn call(it: &mut Interpreter, f: &Value, args: Vec<Value>) -> R {
    it.call_value(f, args)
}

fn strukt(name: &str, fields: Vec<(&str, Value)>) -> Value {
    Value::Struct(Rc::new(StructVal {
        name: Rc::from(name),
        fields: fields.into_iter().map(|(n, v)| (Rc::from(n), v)).collect(),
        copy: false,
        uid: 0,
    }))
}

fn field(v: &Value, name: &str) -> Option<Value> {
    if let Value::Struct(s) = v {
        return s.fields.iter().find(|(n, _)| &**n == name).map(|(_, v)| v.clone());
    }
    None
}

fn io_err(e: std::io::Error) -> Value {
    Value::err(Value::str(e.to_string()))
}

pub fn register(r: &mut Registry) {
    // print/println/eprint/eprintln/panic/assert/assert_eq/assert_ne/todo/unreachable/debug are
    // language intrinsics (`crate::intrinsics::Intrinsic`), evaluated in `interpreter/intrinsics.rs`.
    r.func("format", |it, a| Ok(Value::str(it.format_args(&a)?)));

    // ----- std::test assertions -----
    fn cmp_assert(it: &mut Interpreter, a: &[Value], name: &str, ok: fn(std::cmp::Ordering) -> bool) -> R {
        let (x, y) = (arg(a, 0, name)?, arg(a, 1, name)?);
        if ok(x.structural_cmp(&y)) {
            return Ok(Value::Unit);
        }
        let (l, rr) = (it.debug_string(&x)?, it.debug_string(&y)?);
        let extra = if a.len() > 2 { format!(": {}", it.format_args(&a[2..])?) } else { String::new() };
        Err(panic_ctl(format!("{} failed{}\n  left: {}\n right: {}", name, extra, l, rr)))
    }
    r.func("assert_gt", |it, a| cmp_assert(it, &a, "assert_gt", |o| o == std::cmp::Ordering::Greater));
    r.func("assert_lt", |it, a| cmp_assert(it, &a, "assert_lt", |o| o == std::cmp::Ordering::Less));
    r.func("assert_gte", |it, a| cmp_assert(it, &a, "assert_gte", |o| o != std::cmp::Ordering::Less));
    r.func("assert_lte", |it, a| cmp_assert(it, &a, "assert_lte", |o| o != std::cmp::Ordering::Greater));
    r.func("assert_true", |_, a| {
        if matches!(arg(&a, 0, "assert_true")?, Value::Bool(true)) { Ok(Value::Unit) } else { Err(panic_ctl("assert_true failed")) }
    });
    r.func("assert_false", |_, a| {
        if matches!(arg(&a, 0, "assert_false")?, Value::Bool(false)) { Ok(Value::Unit) } else { Err(panic_ctl("assert_false failed")) }
    });
    r.func("assert_contains", |it, a| {
        let items = it.to_items(arg(&a, 0, "assert_contains")?)?;
        let x = arg(&a, 1, "assert_contains")?;
        if items.iter().any(|y| y.structural_eq(&x)) { Ok(Value::Unit) } else { Err(panic_ctl("assert_contains failed: item not found")) }
    });
    r.func("assert_empty", |it, a| {
        if it.to_items(arg(&a, 0, "assert_empty")?)?.is_empty() { Ok(Value::Unit) } else { Err(panic_ctl("assert_empty failed")) }
    });
    r.func("assert_not_empty", |it, a| {
        if !it.to_items(arg(&a, 0, "assert_not_empty")?)?.is_empty() { Ok(Value::Unit) } else { Err(panic_ctl("assert_not_empty failed")) }
    });
    r.func("assert_in_range", |_, a| {
        let (x, lo, hi) = (arg(&a, 0, "assert_in_range")?, arg(&a, 1, "assert_in_range")?, arg(&a, 2, "assert_in_range")?);
        if x.structural_cmp(&lo) != std::cmp::Ordering::Less && x.structural_cmp(&hi) != std::cmp::Ordering::Greater {
            Ok(Value::Unit)
        } else {
            Err(panic_ctl("assert_in_range failed"))
        }
    });

    // ----- constructors -----
    r.func("Some", |_, a| Ok(Value::some(arg(&a, 0, "Some")?)));
    r.func("Ok", |_, a| Ok(Value::ok(a.first().cloned().map(super::super::place::deref_value).transpose()?.unwrap_or(Value::Unit))));
    r.func("Err", |_, a| Ok(Value::err(arg(&a, 0, "Err")?)));
    r.add_const("None", Value::none());
    r.add_const("Ordering::Less", ordering(std::cmp::Ordering::Less));
    r.add_const("Ordering::Equal", ordering(std::cmp::Ordering::Equal));
    r.add_const("Ordering::Greater", ordering(std::cmp::Ordering::Greater));
    for n in ["Box::new", "Cow::Borrowed", "Cow::Owned", "identity", "convert::identity"] {
        r.func(n, |_, a| arg(&a, 0, "new"));
    }
    r.func("drop", |_, _| Ok(Value::Unit));
    r.func("Vec::new", |_, _| Ok(Value::vec(vec![])));
    r.func("Vec::with_capacity", |_, _| Ok(Value::vec(vec![])));
    r.func("VecDeque::new", |_, _| Ok(Value::vec(vec![])));
    r.func("VecDeque::with_capacity", |_, _| Ok(Value::vec(vec![])));
    r.func("Vec::from", |it, a| {
        let x = arg(&a, 0, "Vec::from")?;
        Ok(match x {
            Value::Str(s) => Value::vec(s.bytes().map(|b| Value::Int(b as i128, IntTy::U8)).collect()),
            other => Value::vec(it.to_items(other)?),
        })
    });
    r.func("VecDeque::from", |it, a| Ok(Value::vec(it.to_items(arg(&a, 0, "from")?)?)));
    r.func("String::new", |_, _| Ok(Value::str("")));
    r.func("String::with_capacity", |_, _| Ok(Value::str("")));
    r.func("String::from", |it, a| Ok(Value::str(it.display_string(&arg(&a, 0, "String::from")?)?)));
    r.func("string::from", |it, a| Ok(Value::str(it.display_string(&arg(&a, 0, "string::from")?)?)));
    r.func("String::from_utf8", |it, a| {
        let bytes: Vec<u8> = it.to_items(arg(&a, 0, "from_utf8")?)?.into_iter().filter_map(|v| if let Value::Int(n, _) = v { Some(n as u8) } else { None }).collect();
        Ok(match String::from_utf8(bytes) {
            Ok(s) => Value::ok(Value::str(s)),
            Err(e) => Value::err(Value::str(e.to_string())),
        })
    });
    r.func("String::from_utf8_lossy", |it, a| {
        let bytes: Vec<u8> = it.to_items(arg(&a, 0, "from_utf8_lossy")?)?.into_iter().filter_map(|v| if let Value::Int(n, _) = v { Some(n as u8) } else { None }).collect();
        Ok(Value::str(String::from_utf8_lossy(&bytes).to_string()))
    });
    for n in ["HashMap::new", "BTreeMap::new", "HashMap::with_capacity", "BTreeMap::with_capacity"] {
        r.func(n, |_, _| Ok(Value::Map(Rc::new(BTreeMap::new()))));
    }
    for n in ["HashSet::new", "BTreeSet::new", "HashSet::with_capacity"] {
        r.func(n, |_, _| Ok(Value::Set(Rc::new(BTreeMap::new()))));
    }
    for n in ["HashMap::from", "BTreeMap::from"] {
        r.func(n, |it, a| super::seq::collect_into(it.to_items(arg(&a, 0, "from")?)?, Some(&crate::parser::Type::Custom("HashMap".into()))));
    }
    for n in ["HashSet::from", "BTreeSet::from"] {
        r.func(n, |it, a| super::seq::collect_into(it.to_items(arg(&a, 0, "from")?)?, Some(&crate::parser::Type::Custom("HashSet".into()))));
    }
    for t in ["Vec", "String", "HashMap", "HashSet", "BTreeMap", "BTreeSet", "VecDeque", "Option", "Result", "Box", "Rc", "Arc", "char", "bool", "str"] {
        r.type_name(t);
    }
    for t in ["i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32", "f64", "int", "uint", "float"] {
        r.type_name(t);
    }

    // ----- std::cmp / std::mem -----
    r.func("cmp::min", |_, a| {
        let (x, y) = (arg(&a, 0, "min")?, arg(&a, 1, "min")?);
        Ok(if y.structural_cmp(&x) == std::cmp::Ordering::Less { y } else { x })
    });
    r.func("cmp::max", |_, a| {
        let (x, y) = (arg(&a, 0, "max")?, arg(&a, 1, "max")?);
        Ok(if y.structural_cmp(&x) == std::cmp::Ordering::Less { x } else { y })
    });
    r.func("min", |_, a| {
        let (x, y) = (arg(&a, 0, "min")?, arg(&a, 1, "min")?);
        Ok(if y.structural_cmp(&x) == std::cmp::Ordering::Less { y } else { x })
    });
    r.func("max", |_, a| {
        let (x, y) = (arg(&a, 0, "max")?, arg(&a, 1, "max")?);
        Ok(if y.structural_cmp(&x) == std::cmp::Ordering::Less { x } else { y })
    });
    r.func("mem::swap", |_, a| match (a.first(), a.get(1)) {
        (Some(Value::Ref(p)), Some(Value::Ref(q))) => {
            let (x, y) = (p.read()?, q.read()?);
            p.write(y)?;
            q.write(x)?;
            Ok(Value::Unit)
        }
        _ => Err(semantic("mem::swap needs two `&mut` arguments")),
    });
    r.func("mem::replace", |_, a| match (a.first(), a.get(1)) {
        (Some(Value::Ref(p)), Some(v)) => {
            let old = p.read()?;
            p.write(v.clone())?;
            Ok(old)
        }
        _ => Err(semantic("mem::replace needs a `&mut` argument")),
    });
    r.func("mem::take", |it, a| match a.first() {
        Some(Value::Ref(p)) => {
            let old = p.read()?;
            let d = it.default_like(&old)?;
            p.write(d)?;
            Ok(old)
        }
        _ => Err(semantic("mem::take needs a `&mut` argument")),
    });

    // ----- std::process / std::env / std::fs / std::io / std::time -----
    r.func("process::exit", |_, a| {
        let code = arg_int(&a, 0, "exit")? as i32;
        Err(RuntimeError::new(ErrorKind::Exit(code), format!("exit({})", code)).into())
    });
    r.func("env::args", |it, _| {
        let mut v = vec![Value::str("wj-interpreted")];
        v.extend(it.args.iter().cloned().map(Value::str));
        Ok(new_iter(v))
    });
    r.func("env::var", |_, a| {
        let n = arg_str(&a, 0, "env::var")?;
        Ok(match std::env::var(&*n) {
            Ok(v) => Value::ok(Value::str(v)),
            Err(e) => Value::err(Value::str(e.to_string())),
        })
    });
    r.func("fs::read_to_string", |_, a| {
        let p = arg_str(&a, 0, "fs::read_to_string")?;
        Ok(match std::fs::read_to_string(&*p) {
            Ok(s) => Value::ok(Value::str(s)),
            Err(e) => io_err(e),
        })
    });
    r.func("fs::write", |it, a| {
        let p = arg_str(&a, 0, "fs::write")?;
        let c = arg(&a, 1, "fs::write")?;
        let bytes: Vec<u8> = match c {
            Value::Str(s) => s.as_bytes().to_vec(),
            other => it.to_items(other)?.into_iter().filter_map(|v| if let Value::Int(n, _) = v { Some(n as u8) } else { None }).collect(),
        };
        Ok(match std::fs::write(&*p, bytes) {
            Ok(()) => Value::ok(Value::Unit),
            Err(e) => io_err(e),
        })
    });
    r.func("fs::remove_file", |_, a| {
        let p = arg_str(&a, 0, "fs::remove_file")?;
        Ok(match std::fs::remove_file(&*p) {
            Ok(()) => Value::ok(Value::Unit),
            Err(e) => io_err(e),
        })
    });
    r.func("fs::create_dir_all", |_, a| {
        let p = arg_str(&a, 0, "fs::create_dir_all")?;
        Ok(match std::fs::create_dir_all(&*p) {
            Ok(()) => Value::ok(Value::Unit),
            Err(e) => io_err(e),
        })
    });
    r.func("fs::exists", |_, a| Ok(Value::Bool(std::path::Path::new(&*arg_str(&a, 0, "fs::exists")?).exists())));
    r.func("io::stdin", |_, _| Ok(strukt("Stdin", vec![])));
    r.pure(Kind::Struct, "read_line", |_, v, a| {
        if !matches!(v, Value::Struct(s) if &*s.name == "Stdin") {
            return Err(semantic("`read_line` is only available on stdin"));
        }
        let mut line = String::new();
        let n = std::io::stdin().read_line(&mut line).map_err(|e| semantic(e.to_string()))?;
        if let Some(Value::Ref(p)) = a.first() {
            p.with_mut(|t| {
                if let Value::Str(s) = t {
                    Rc::make_mut(s).push_str(&line);
                }
                Ok(())
            })?;
        }
        Ok(Value::ok(Value::usize(n)))
    });
    let _ = call;
    register_std_modules(r);
}

fn f1(a: &[Value], what: &str) -> R<f64> {
    arg_f64(a, 0, what)
}

/// Native versions of the `std::math`, `std::strings`, `std::time` stubs plus `fs::append`.
fn register_std_modules(r: &mut Registry) {
    macro_rules! math1 {
        ($($name:literal => $f:expr),* $(,)?) => {$(
            r.func($name, |_, a| {
                let f: fn(f64) -> f64 = $f;
                Ok(Value::float(f(f1(&a, $name)?)))
            });
        )*};
    }
    math1! {
        "math::abs_f64" => |x| x.abs(), "math::sqrt" => |x| x.sqrt(), "math::cbrt" => |x| x.cbrt(),
        "math::round" => |x| x.round(), "math::floor" => |x| x.floor(), "math::ceil" => |x| x.ceil(),
        "math::trunc" => |x| x.trunc(), "math::sin" => |x| x.sin(), "math::cos" => |x| x.cos(),
        "math::tan" => |x| x.tan(), "math::asin" => |x| x.asin(), "math::acos" => |x| x.acos(),
        "math::atan" => |x| x.atan(), "math::sinh" => |x| x.sinh(), "math::cosh" => |x| x.cosh(),
        "math::tanh" => |x| x.tanh(), "math::exp" => |x| x.exp(), "math::exp2" => |x| x.exp2(),
        "math::ln" => |x| x.ln(), "math::log2" => |x| x.log2(), "math::log10" => |x| x.log10(),
        "math::signum" => |x| x.signum(), "math::to_radians" => |x| x.to_radians(),
        "math::to_degrees" => |x| x.to_degrees(),
    }
    r.func("math::abs_i64", |_, a| Ok(Value::Int(arg_int(&a, 0, "abs_i64")?.abs(), IntTy::I64)));
    r.func("math::pow", |_, a| Ok(Value::float(f1(&a, "pow")?.powf(arg_f64(&a, 1, "pow")?))));
    r.func("math::atan2", |_, a| Ok(Value::float(f1(&a, "atan2")?.atan2(arg_f64(&a, 1, "atan2")?))));
    r.func("math::hypot", |_, a| Ok(Value::float(f1(&a, "hypot")?.hypot(arg_f64(&a, 1, "hypot")?))));
    r.func("math::log", |_, a| Ok(Value::float(f1(&a, "log")?.ln() / arg_f64(&a, 1, "log")?.ln())));
    r.func("math::copysign", |_, a| Ok(Value::float(f1(&a, "copysign")?.copysign(arg_f64(&a, 1, "copysign")?))));
    r.func("math::fma", |_, a| Ok(Value::float(f1(&a, "fma")?.mul_add(arg_f64(&a, 1, "fma")?, arg_f64(&a, 2, "fma")?))));
    r.func("math::min_f64", |_, a| Ok(Value::float(f1(&a, "min_f64")?.min(arg_f64(&a, 1, "min_f64")?))));
    r.func("math::max_f64", |_, a| Ok(Value::float(f1(&a, "max_f64")?.max(arg_f64(&a, 1, "max_f64")?))));
    r.func("math::clamp_f64", |_, a| {
        Ok(Value::float(f1(&a, "clamp_f64")?.clamp(arg_f64(&a, 1, "clamp_f64")?, arg_f64(&a, 2, "clamp_f64")?)))
    });
    r.func("math::min_i64", |_, a| Ok(Value::Int(arg_int(&a, 0, "min_i64")?.min(arg_int(&a, 1, "min_i64")?), IntTy::I64)));
    r.func("math::max_i64", |_, a| Ok(Value::Int(arg_int(&a, 0, "max_i64")?.max(arg_int(&a, 1, "max_i64")?), IntTy::I64)));
    r.func("math::clamp_i64", |_, a| {
        Ok(Value::Int(arg_int(&a, 0, "clamp_i64")?.clamp(arg_int(&a, 1, "clamp_i64")?, arg_int(&a, 2, "clamp_i64")?), IntTy::I64))
    });
    r.func("math::is_nan", |_, a| Ok(Value::Bool(f1(&a, "is_nan")?.is_nan())));
    r.func("math::is_infinite", |_, a| Ok(Value::Bool(f1(&a, "is_infinite")?.is_infinite())));
    r.func("math::is_finite", |_, a| Ok(Value::Bool(f1(&a, "is_finite")?.is_finite())));

    // std::strings: free-function forms of the String methods
    r.func("strings::to_upper", |_, a| Ok(Value::str(arg_str(&a, 0, "to_upper")?.to_uppercase())));
    r.func("strings::to_lower", |_, a| Ok(Value::str(arg_str(&a, 0, "to_lower")?.to_lowercase())));
    r.func("strings::trim", |_, a| Ok(Value::str(arg_str(&a, 0, "trim")?.trim())));
    r.func("strings::len", |_, a| Ok(Value::usize(arg_str(&a, 0, "len")?.len())));
    r.func("strings::char_count", |_, a| Ok(Value::usize(arg_str(&a, 0, "char_count")?.chars().count())));
    r.func("strings::contains", |_, a| Ok(Value::Bool(arg_str(&a, 0, "contains")?.contains(&**arg_str(&a, 1, "contains")?))));
    r.func("strings::starts_with", |_, a| Ok(Value::Bool(arg_str(&a, 0, "starts_with")?.starts_with(&**arg_str(&a, 1, "starts_with")?))));
    r.func("strings::ends_with", |_, a| Ok(Value::Bool(arg_str(&a, 0, "ends_with")?.ends_with(&**arg_str(&a, 1, "ends_with")?))));
    r.func("strings::replace", |_, a| {
        Ok(Value::str(arg_str(&a, 0, "replace")?.replace(&**arg_str(&a, 1, "replace")?, &arg_str(&a, 2, "replace")?)))
    });
    r.func("strings::repeat", |_, a| Ok(Value::str(arg_str(&a, 0, "repeat")?.repeat(arg_usize(&a, 1, "repeat")?))));
    r.func("strings::split", |_, a| {
        let s = arg_str(&a, 0, "split")?;
        let sep = arg_str(&a, 1, "split")?;
        Ok(Value::vec(s.split(&**sep).map(Value::str).collect()))
    });
    r.func("strings::join", |it, a| {
        let items = it.to_items(arg(&a, 0, "join")?)?;
        let sep = arg_str(&a, 1, "join")?;
        let mut parts = Vec::new();
        for x in &items {
            parts.push(it.display_string(x)?);
        }
        Ok(Value::str(parts.join(&sep)))
    });

    // std::time / std::fs extras
    fn now() -> std::time::Duration {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default()
    }
    let _ = now;
    r.func("time::now_millis", |it, _| Ok(Value::Int((it.wall_ns() / 1_000_000) as i128, IntTy::I64)));
    r.func("time::now_unix", |it, _| Ok(Value::Int((it.wall_ns() / 1_000_000_000) as i128, IntTy::I64)));
    r.func("time::now_unix_nano", |it, _| Ok(Value::Int(it.wall_ns() as i128, IntTy::I64)));
    r.func("time::sleep_millis", |it, a| {
        it.sleep_ns(arg_int(&a, 0, "sleep_millis")?.max(0) as u64 * 1_000_000)?;
        Ok(Value::Unit)
    });
    macro_rules! unary_math {
        ($($name:literal => $f:expr),* $(,)?) => {
            $( r.func(concat!("math::", $name), |_, a| {
                let f: fn(f64) -> f64 = $f;
                Ok(Value::float(f(arg_f64(&a, 0, $name)?)))
            }); )*
        };
    }
    unary_math!(
        "abs_f64" => f64::abs, "sqrt" => f64::sqrt, "cbrt" => f64::cbrt, "round" => f64::round,
        "floor" => f64::floor, "ceil" => f64::ceil, "trunc" => f64::trunc, "sin" => f64::sin,
        "cos" => f64::cos, "tan" => f64::tan, "asin" => f64::asin, "acos" => f64::acos,
        "atan" => f64::atan, "sinh" => f64::sinh, "cosh" => f64::cosh, "tanh" => f64::tanh,
        "exp" => f64::exp, "exp2" => f64::exp2, "ln" => f64::ln, "log2" => f64::log2,
        "log10" => f64::log10, "signum" => f64::signum, "to_radians" => f64::to_radians,
        "to_degrees" => f64::to_degrees,
    );
    r.add_const("math::PI", Value::float(std::f64::consts::PI));
    r.add_const("math::E", Value::float(std::f64::consts::E));
    r.func("fs::is_dir", |_, a| Ok(Value::Bool(std::path::Path::new(arg_str(&a, 0, "is_dir")?.as_str()).is_dir())));
    r.func("fs::is_file", |_, a| Ok(Value::Bool(std::path::Path::new(arg_str(&a, 0, "is_file")?.as_str()).is_file())));
    r.func("fs::append", |_, a| {
        use std::io::Write;
        let p = arg_str(&a, 0, "fs::append")?;
        let c = arg_str(&a, 1, "fs::append")?;
        let r = std::fs::OpenOptions::new().append(true).create(true).open(&*p).and_then(|mut f| f.write_all(c.as_bytes()));
        Ok(match r {
            Ok(()) => Value::ok(Value::Unit),
            Err(e) => io_err(e),
        })
    });
}

/// `u8::from(x)`, `i64::try_from(x)`, `u32::from_str_radix(s, 16)`, `f64::from(x)`, `char::from_u32`, `T::default()`.
pub fn assoc_numeric(it: &mut Interpreter, path: &str, args: &[Value]) -> Option<R> {
    let (ty, method) = path.rsplit_once("::")?;
    let ty = ty.rsplit("::").next().unwrap_or(ty);
    if ty == "char" {
        return match method {
            "from_u32" => Some((|| {
                let n = arg_int(args, 0, "char::from_u32")?;
                Ok(Value::option(u32::try_from(n).ok().and_then(char::from_u32).map(Value::Char)))
            })()),
            "from_digit" => Some((|| {
                let (n, radix) = (arg_int(args, 0, "from_digit")?, arg_int(args, 1, "from_digit")?);
                Ok(Value::option(char::from_digit(n as u32, radix as u32).map(Value::Char)))
            })()),
            "from" => Some((|| match arg(args, 0, "char::from")? {
                Value::Int(n, _) => Ok(Value::Char(char::from_u32(n as u32).unwrap_or('\u{fffd}'))),
                c => Ok(c),
            })()),
            "default" => Some(Ok(Value::Char('\0'))),
            _ => None,
        };
    }
    if let Some(ity) = IntTy::from_name(ty) {
        return match method {
            "from" => Some((|| match arg(args, 0, "from")? {
                Value::Int(n, _) => Ok(Value::Int(n, ity)),
                Value::Bool(b) => Ok(Value::Int(b as i128, ity)),
                Value::Char(c) => Ok(Value::Int(c as i128, ity)),
                other => Err(semantic(format!("cannot convert {} to {}", other.type_name(), ty))),
            })()),
            "try_from" => Some((|| match arg(args, 0, "try_from")? {
                Value::Int(n, _) if ity.fits(n) => Ok(Value::ok(Value::Int(n, ity))),
                Value::Int(..) => Ok(Value::err(Value::str("out of range integral type conversion attempted"))),
                other => Err(semantic(format!("cannot convert {} to {}", other.type_name(), ty))),
            })()),
            "from_str_radix" => Some((|| {
                let s = arg_str(args, 0, "from_str_radix")?;
                let radix = arg_int(args, 1, "from_str_radix")? as u32;
                Ok(match i128::from_str_radix(&s, radix) {
                    Ok(n) if ity.fits(n) => Value::ok(Value::Int(n, ity)),
                    Ok(_) => Value::err(Value::str("number too large to fit in target type")),
                    Err(e) => Value::err(Value::str(e.to_string())),
                })
            })()),
            "default" => Some(Ok(Value::Int(0, ity))),
            "pow" | "abs" | "min" | "max" | "wrapping_add" | "wrapping_mul" | "wrapping_sub" | "checked_add" | "checked_mul" | "checked_sub" | "saturating_sub" | "saturating_add" | "rem_euclid" | "signum" | "count_ones" | "leading_zeros" | "trailing_zeros" | "abs_diff" => {
                // UFCS: i32::pow(x, 2)
                let recv = match args.first() {
                    Some(Value::Int(n, _)) => Value::Int(*n, ity),
                    _ => return None,
                };
                Some(it.call_method_value(recv, method, args[1..].to_vec(), None))
            }
            _ => None,
        };
    }
    if matches!(ty, "f64" | "f32" | "float") {
        let ft = if ty == "f32" { FloatTy::F32 } else { FloatTy::F64 };
        return match method {
            "from" => Some((|| match arg(args, 0, "from")? {
                Value::Int(n, _) => Ok(Value::Float(n as f64, ft)),
                Value::Float(f, _) => Ok(Value::Float(f, ft)),
                other => Err(semantic(format!("cannot convert {} to {}", other.type_name(), ty))),
            })()),
            "default" => Some(Ok(Value::Float(0.0, ft))),
            "sqrt" | "abs" | "floor" | "ceil" | "powi" | "powf" | "min" | "max" | "sin" | "cos" | "exp" | "ln" => {
                let recv = match args.first() {
                    Some(Value::Float(f, _)) => Value::Float(*f, ft),
                    Some(Value::Int(n, _)) => Value::Float(*n as f64, ft),
                    _ => return None,
                };
                Some(it.call_method_value(recv, method, args[1..].to_vec(), None))
            }
            _ => None,
        };
    }
    match (ty, method) {
        ("String" | "string" | "str", "default") => Some(Ok(Value::str(""))),
        ("Vec" | "VecDeque", "default") => Some(Ok(Value::vec(vec![]))),
        ("HashMap" | "BTreeMap", "default") => Some(Ok(Value::Map(Rc::new(BTreeMap::new())))),
        ("HashSet" | "BTreeSet", "default") => Some(Ok(Value::Set(Rc::new(BTreeMap::new())))),
        ("bool", "default") => Some(Ok(Value::Bool(false))),
        ("Option", "default") => Some(Ok(Value::none())),
        _ => None,
    }
}
