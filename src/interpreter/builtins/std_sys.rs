//! `std::fs` (rest), `std::path`, `std::process`, `std::random`, `std::uuid`, `std::log` and the
//! remaining `std::strings` helpers. Semantics mirror `crates/windjammer-runtime` (same results,
//! same error strings: they are the host's `io::Error` texts).

use super::*;
use crate::interpreter::native::{downcast, Native};
use rand::{Rng as _, SeedableRng};
use std::any::Any;
use std::path::Path;

fn s(v: &str) -> Value {
    Value::str(v.to_string())
}

fn unit_res(r: std::io::Result<()>) -> R {
    Ok(match r {
        Ok(()) => Value::ok(Value::Unit),
        Err(e) => Value::err(Value::str(e.to_string())),
    })
}

fn bytes_value(b: &[u8]) -> Value {
    Value::vec(b.iter().map(|x| Value::Int(*x as i128, IntTy::U8)).collect())
}

fn bytes_of(it: &mut Interpreter, v: Value) -> R<Vec<u8>> {
    Ok(it
        .to_items(v)?
        .into_iter()
        .map(|x| if let Value::Int(n, _) = x { n as u8 } else { 0 })
        .collect())
}

// ---------------------------------------------------------------- fs objects

struct DirEntryObj {
    path: String,
}
struct MetadataObj {
    len: u64,
    file: bool,
    dir: bool,
    readonly: bool,
}
macro_rules! native {
    ($t:ty, $n:literal) => {
        impl Native for $t {
            fn type_name(&self) -> &'static str {
                $n
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}
native!(DirEntryObj, "DirEntry");
native!(MetadataObj, "Metadata");

fn entry(v: &Value) -> R<&DirEntryObj> {
    downcast::<DirEntryObj>(v).ok_or_else(|| semantic("expected a DirEntry"))
}

fn meta_value(path: &str) -> Value {
    match std::fs::metadata(path) {
        Ok(m) => Value::ok(Value::Native(Rc::new(MetadataObj {
            len: m.len(),
            file: m.is_file(),
            dir: m.is_dir(),
            readonly: m.permissions().readonly(),
        }))),
        Err(e) => Value::err(Value::str(e.to_string())),
    }
}

fn path_part(p: &str, f: fn(&Path) -> Option<&std::ffi::OsStr>) -> Value {
    Value::option(f(Path::new(p)).map(|x| s(&x.to_string_lossy())))
}

// ---------------------------------------------------------------- random

struct RngObj(RefCell<rand::rngs::StdRng>);
native!(RngObj, "Rng");

impl Interpreter {
    fn rng(&mut self) -> &mut rand::rngs::StdRng {
        let det = self.sched.deterministic;
        self.rng.get_or_insert_with(|| {
            if det {
                rand::rngs::StdRng::seed_from_u64(0x5EED)
            } else {
                rand::rngs::StdRng::from_entropy()
            }
        })
    }
}

fn shuffle_values(rng: &mut rand::rngs::StdRng, mut items: Vec<Value>) -> Vec<Value> {
    use rand::seq::SliceRandom;
    items.shuffle(rng);
    items
}

fn alnum(rng: &mut rand::rngs::StdRng, n: usize, ascii: bool) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    (0..n)
        .map(|_| if ascii { rng.gen_range(0x20u8..0x7f) as char } else { A[rng.gen_range(0..A.len())] as char })
        .collect()
}

pub fn register(r: &mut Registry) {
    for t in ["DirEntry", "Metadata", "Rng"] {
        r.type_name(t);
    }
    register_fs(r);
    register_process_and_random(r);
    register_uuid_log_strings(r);
}

fn register_fs(r: &mut Registry) {
    r.func("fs::read", |_, a| {
        Ok(match std::fs::read(arg_str(&a, 0, "read")?.as_str()) {
            Ok(b) => Value::ok(bytes_value(&b)),
            Err(e) => Value::err(Value::str(e.to_string())),
        })
    });
    r.func("fs::write_bytes", |it, a| {
        let data = bytes_of(it, arg(&a, 1, "write_bytes")?)?;
        unit_res(std::fs::write(arg_str(&a, 0, "write_bytes")?.as_str(), data))
    });
    r.func("fs::copy", |_, a| {
        unit_res(std::fs::copy(arg_str(&a, 0, "copy")?.as_str(), arg_str(&a, 1, "copy")?.as_str()).map(|_| ()))
    });
    r.func("fs::rename", |_, a| unit_res(std::fs::rename(arg_str(&a, 0, "rename")?.as_str(), arg_str(&a, 1, "rename")?.as_str())));
    r.func("fs::create_dir", |_, a| unit_res(std::fs::create_dir(arg_str(&a, 0, "create_dir")?.as_str())));
    r.func("fs::remove_dir", |_, a| unit_res(std::fs::remove_dir(arg_str(&a, 0, "remove_dir")?.as_str())));
    r.func("fs::remove_dir_all", |_, a| unit_res(std::fs::remove_dir_all(arg_str(&a, 0, "remove_dir_all")?.as_str())));
    r.func("fs::read_dir", |_, a| {
        let rd = match std::fs::read_dir(arg_str(&a, 0, "read_dir")?.as_str()) {
            Ok(r) => r,
            Err(e) => return Ok(Value::err(Value::str(e.to_string()))),
        };
        let mut out = Vec::new();
        for e in rd {
            match e {
                Ok(e) => {
                    if let Some(p) = e.path().to_str() {
                        out.push(Value::Native(Rc::new(DirEntryObj { path: p.to_string() })));
                    }
                }
                Err(e) => return Ok(Value::err(Value::str(e.to_string()))),
            }
        }
        Ok(Value::ok(Value::vec(out)))
    });
    r.func("fs::current_dir", |_, _| {
        Ok(match std::env::current_dir() {
            Ok(p) => Value::ok(s(&p.to_string_lossy())),
            Err(e) => Value::err(Value::str(e.to_string())),
        })
    });
    r.func("fs::set_current_dir", |_, a| unit_res(std::env::set_current_dir(arg_str(&a, 0, "set_current_dir")?.as_str())));
    r.func("fs::metadata", |_, a| Ok(meta_value(arg_str(&a, 0, "metadata")?.as_str())));
    r.func("fs::canonicalize", |_, a| {
        Ok(match std::fs::canonicalize(arg_str(&a, 0, "canonicalize")?.as_str()) {
            Ok(p) => Value::ok(s(&p.to_string_lossy())),
            Err(e) => Value::err(Value::str(e.to_string())),
        })
    });
    for n in ["fs::join", "path::join"] {
        r.func(n, |_, a| {
            let p = Path::new(arg_str(&a, 0, "join")?.as_str()).join(arg_str(&a, 1, "join")?.as_str());
            Ok(s(&p.to_string_lossy()))
        });
    }
    r.func("fs::extension", |_, a| Ok(path_part(arg_str(&a, 0, "extension")?.as_str(), |p| p.extension())));
    r.func("fs::file_name", |_, a| Ok(path_part(arg_str(&a, 0, "file_name")?.as_str(), |p| p.file_name())));
    r.func("path::file_name", |_, a| Ok(path_part(arg_str(&a, 0, "file_name")?.as_str(), |p| p.file_name())));
    r.func("fs::file_stem", |_, a| Ok(path_part(arg_str(&a, 0, "file_stem")?.as_str(), |p| p.file_stem())));
    r.func("fs::parent", |_, a| {
        Ok(Value::option(Path::new(arg_str(&a, 0, "parent")?.as_str()).parent().map(|p| s(&p.to_string_lossy()))))
    });
    r.func("fs::is_absolute", |_, a| Ok(Value::Bool(Path::new(arg_str(&a, 0, "is_absolute")?.as_str()).is_absolute())));
    r.func("fs::is_relative", |_, a| Ok(Value::Bool(Path::new(arg_str(&a, 0, "is_relative")?.as_str()).is_relative())));
    r.func("path::glob_match", |_, a| {
        Ok(Value::Bool(glob_match(arg_str(&a, 0, "glob_match")?.as_bytes(), arg_str(&a, 1, "glob_match")?.as_bytes())))
    });
    r.native("DirEntry", "path", |_, v, _| Ok(s(&entry(v)?.path)));
    for n in ["name", "file_name"] {
        r.native("DirEntry", n, |_, v, _| {
            Ok(s(&Path::new(&entry(v)?.path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()))
        });
    }
    r.native("DirEntry", "is_file", |_, v, _| Ok(Value::Bool(Path::new(&entry(v)?.path).is_file())));
    r.native("DirEntry", "is_dir", |_, v, _| Ok(Value::Bool(Path::new(&entry(v)?.path).is_dir())));
    r.native("DirEntry", "metadata", |_, v, _| Ok(meta_value(&entry(v)?.path)));
    fn meta(v: &Value) -> R<&MetadataObj> {
        downcast::<MetadataObj>(v).ok_or_else(|| semantic("expected Metadata"))
    }
    r.native("Metadata", "size", |_, v, _| Ok(Value::Int(meta(v)?.len as i128, IntTy::I64)));
    r.native("Metadata", "len", |_, v, _| Ok(Value::Int(meta(v)?.len as i128, IntTy::U64)));
    r.native("Metadata", "is_file", |_, v, _| Ok(Value::Bool(meta(v)?.file)));
    r.native("Metadata", "is_dir", |_, v, _| Ok(Value::Bool(meta(v)?.dir)));
    r.native("Metadata", "is_readonly", |_, v, _| Ok(Value::Bool(meta(v)?.readonly)));
}

fn register_process_and_random(r: &mut Registry) {
    r.func("process::pid", |_, _| Ok(Value::Int(std::process::id() as i128, IntTy::I64)));
    r.func("process::run", |_, a| run_command(arg_str(&a, 0, "run")?.as_str(), &[]));
    r.func("process::run_with_args", |it, a| {
        let args: Vec<String> = it
            .to_items(arg(&a, 1, "run_with_args")?)?
            .iter()
            .filter_map(|v| if let Value::Str(x) = v { Some((**x).clone()) } else { None })
            .collect();
        run_command(arg_str(&a, 0, "run_with_args")?.as_str(), &args)
    });

    r.func("random::range", |it, a| {
        let (lo, hi) = (arg_int(&a, 0, "range")?, arg_int(&a, 1, "range")?);
        if lo >= hi {
            return Err(panic_ctl("cannot sample empty range"));
        }
        Ok(Value::Int(it.rng().gen_range(lo as i64..hi as i64) as i128, IntTy::I64))
    });
    r.func("random::range_inclusive", |it, a| {
        let (lo, hi) = (arg_int(&a, 0, "range_inclusive")?, arg_int(&a, 1, "range_inclusive")?);
        if lo > hi {
            return Err(panic_ctl("cannot sample empty range"));
        }
        Ok(Value::Int(it.rng().gen_range(lo as i64..=hi as i64) as i128, IntTy::I64))
    });
    r.func("random::float", |it, _| Ok(Value::float(it.rng().gen::<f64>())));
    r.func("random::float_range", |it, a| {
        Ok(Value::float(it.rng().gen_range(arg_f64(&a, 0, "float_range")?..arg_f64(&a, 1, "float_range")?)))
    });
    r.func("random::bool", |it, _| Ok(Value::Bool(it.rng().gen::<bool>())));
    r.func("random::bool_with_probability", |it, a| {
        Ok(Value::Bool(it.rng().gen_bool(arg_f64(&a, 0, "bool_with_probability")?.clamp(0.0, 1.0))))
    });
    r.func("random::shuffle", |it, a| {
        let items = it.to_items(arg(&a, 0, "shuffle")?)?;
        Ok(Value::vec(shuffle_values(it.rng(), items)))
    });
    r.func("random::choice", |it, a| {
        let items = it.to_items(arg(&a, 0, "choice")?)?;
        if items.is_empty() {
            return Ok(Value::none());
        }
        let i = it.rng().gen_range(0..items.len());
        Ok(Value::some(items[i].clone()))
    });
    r.func("random::sample", |it, a| {
        let items = it.to_items(arg(&a, 0, "sample")?)?;
        let n = (arg_int(&a, 1, "sample")?.max(0) as usize).min(items.len());
        Ok(Value::vec(shuffle_values(it.rng(), items).into_iter().take(n).collect()))
    });
    r.func("random::alphanumeric", |it, a| Ok(Value::str(alnum(it.rng(), arg_usize(&a, 0, "alphanumeric")?, false))));
    r.func("random::ascii", |it, a| Ok(Value::str(alnum(it.rng(), arg_usize(&a, 0, "ascii")?, true))));
    r.func("random::bytes", |it, a| {
        let n = arg_usize(&a, 0, "bytes")?;
        let b: Vec<u8> = (0..n).map(|_| it.rng().gen::<u8>()).collect();
        Ok(bytes_value(&b))
    });
    r.func("random::seeded", |_, a| {
        Ok(Value::Native(Rc::new(RngObj(RefCell::new(rand::rngs::StdRng::seed_from_u64(arg_int(&a, 0, "seeded")? as u64))))))
    });
    fn rng(v: &Value) -> R<&RngObj> {
        downcast::<RngObj>(v).ok_or_else(|| semantic("expected an Rng"))
    }
    r.native("Rng", "range", |_, v, a| {
        let (lo, hi) = (arg_int(&a, 0, "range")?, arg_int(&a, 1, "range")?);
        if lo >= hi {
            return Err(panic_ctl("cannot sample empty range"));
        }
        Ok(Value::Int(rng(v)?.0.borrow_mut().gen_range(lo as i64..hi as i64) as i128, IntTy::I64))
    });
    r.native("Rng", "float", |_, v, _| Ok(Value::float(rng(v)?.0.borrow_mut().gen::<f64>())));
    r.native("Rng", "bool", |_, v, _| Ok(Value::Bool(rng(v)?.0.borrow_mut().gen::<bool>())));
    r.native("Rng", "shuffle", |it, v, a| {
        let items = it.to_items(arg(&a, 0, "shuffle")?)?;
        Ok(Value::vec(shuffle_values(&mut rng(v)?.0.borrow_mut(), items)))
    });
}

fn register_uuid_log_strings(r: &mut Registry) {
    r.func("uuid::v4", |it, _| {
        let mut b: [u8; 16] = it.rng().gen();
        b[6] = (b[6] & 0x0f) | 0x40;
        b[8] = (b[8] & 0x3f) | 0x80;
        Ok(Value::str(uuid_text(&b)))
    });
    r.func("uuid::v7", |it, _| {
        let ms = (it.wall_ns() / 1_000_000) as u64;
        let mut b: [u8; 16] = it.rng().gen();
        Ok(Value::str(uuid7(ms, &mut b)))
    });
    r.func("uuid::v7_from_timestamp", |_, a| {
        let ms = arg_int(&a, 0, "v7_from_timestamp")? as u64;
        let hex = arg_str(&a, 1, "v7_from_timestamp")?;
        if hex.len() < 20 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(Value::err(s("random_hex must be at least 20 hex digits")));
        }
        let mut b = [0u8; 16];
        for i in 0..10 {
            b[6 + i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap_or(0);
        }
        Ok(Value::ok(Value::str(uuid7(ms, &mut b))))
    });

    r.func("log::init", |it, _| {
        it.log_level = Some(rust_log(it)?.unwrap_or(0));
        Ok(Value::Unit)
    });
    r.func("log::init_with_level", |it, a| {
        let idx = match arg(&a, 0, "init_with_level")? {
            Value::Enum(e) => e.idx as i32,
            _ => 2,
        };
        let env_level = rust_log(it)?;
        it.log_level = Some(match env_level {
            Some(e) => e,
            None => 4 - idx,
        });
        Ok(Value::Unit)
    });
    macro_rules! log_fn {
        ($($path:literal, $with:literal, $en:literal, $lvl:expr);*) => {$(
            r.func($path, |it, a| {
                it.log_emit($lvl, &arg_str(&a, 0, $path)?, None);
                Ok(Value::Unit)
            });
            r.func($with, |it, a| {
                let kv = (arg_str(&a, 1, $with)?, arg_str(&a, 2, $with)?);
                it.log_emit($lvl, &arg_str(&a, 0, $with)?, Some(kv));
                Ok(Value::Unit)
            });
            r.func($en, |it, _| Ok(Value::Bool(it.log_enabled($lvl))));
        )*};
    }
    log_fn!(
        "log::trace", "log::trace_with", "log::trace_enabled", 4;
        "log::debug", "log::debug_with", "log::debug_enabled", 3;
        "log::info", "log::info_with", "log::info_enabled", 2;
        "log::warn", "log::warn_with", "log::warn_enabled", 1;
        "log::error", "log::error_with", "log::error_enabled", 0
    );

    r.func("strings::trim_start", |_, a| Ok(s(arg_str(&a, 0, "trim_start")?.trim_start())));
    r.func("strings::trim_end", |_, a| Ok(s(arg_str(&a, 0, "trim_end")?.trim_end())));
    r.func("strings::split_lines", |_, a| {
        let text = arg_str(&a, 0, "split_lines")?;
        let mut lines = Vec::new();
        let mut cur = String::new();
        for ch in text.chars() {
            if ch == '\n' {
                lines.push(Value::str(std::mem::take(&mut cur)));
            } else if ch != '\r' {
                cur.push(ch);
            }
        }
        if !cur.is_empty() {
            lines.push(Value::str(cur));
        }
        Ok(Value::vec(lines))
    });
    r.func("strings::split_whitespace", |_, a| {
        let mut parts = Vec::new();
        let mut cur = String::new();
        for ch in arg_str(&a, 0, "split_whitespace")?.chars() {
            if ch == ' ' || ch == '\t' {
                if !cur.is_empty() {
                    parts.push(Value::str(std::mem::take(&mut cur)));
                }
            } else {
                cur.push(ch);
            }
        }
        if !cur.is_empty() {
            parts.push(Value::str(cur));
        }
        Ok(Value::vec(parts))
    });
    r.func("strings::chars", |_, a| Ok(Value::vec(arg_str(&a, 0, "chars")?.chars().map(Value::Char).collect())));
    r.func("strings::from_chars", |it, a| {
        Ok(Value::str(
            it.to_items(arg(&a, 0, "from_chars")?)?
                .iter()
                .filter_map(|v| if let Value::Char(c) = v { Some(*c) } else { None })
                .collect::<String>(),
        ))
    });
    for n in ["strings::substring_chars", "strings::substring"] {
        r.func(n, |_, a| {
            let (st, en) = (arg_usize(&a, 1, "substring")?, arg_usize(&a, 2, "substring")?);
            Ok(Value::str(arg_str(&a, 0, "substring")?.chars().skip(st).take(en.saturating_sub(st)).collect::<String>()))
        });
    }
    r.func("strings::parse_i32", |_, a| Ok(Value::Int(arg_str(&a, 0, "parse_i32")?.trim().parse::<i32>().unwrap_or(0) as i128, IntTy::I32)));
    r.func("strings::parse_f32", |_, a| {
        Ok(Value::Float(arg_str(&a, 0, "parse_f32")?.trim().parse::<f32>().unwrap_or(0.0) as f64, FloatTy::F32))
    });
    r.func("strings::parse_bool", |_, a| Ok(Value::Bool(arg_str(&a, 0, "parse_bool")?.trim() == "true")));
    r.func("strings::byte_at", |_, a| {
        Ok(Value::Int(
            arg_str(&a, 0, "byte_at")?.as_bytes().get(arg_usize(&a, 1, "byte_at")?).copied().unwrap_or(0) as i128,
            IntTy::U8,
        ))
    });
    r.func("strings::is_empty", |_, a| Ok(Value::Bool(arg_str(&a, 0, "is_empty")?.is_empty())));
    r.func("strings::replacen", |_, a| {
        Ok(Value::str(arg_str(&a, 0, "replacen")?.replacen(
            arg_str(&a, 1, "replacen")?.as_str(),
            arg_str(&a, 2, "replacen")?.as_str(),
            arg_usize(&a, 3, "replacen")?,
        )))
    });
}

fn uuid_text(b: &[u8; 16]) -> String {
    let h: String = b.iter().map(|x| format!("{:02x}", x)).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

fn uuid7(ms: u64, rnd: &mut [u8; 16]) -> String {
    let mut b = [0u8; 16];
    b[..6].copy_from_slice(&ms.to_be_bytes()[2..]);
    b[6..].copy_from_slice(&rnd[6..]);
    b[6] = (b[6] & 0x0f) | 0x70;
    b[8] = (b[8] & 0x3f) | 0x80;
    uuid_text(&b)
}

/// `RUST_LOG`, read through the program-visible `env::var` builtin (env_logger honours it).
fn rust_log(it: &mut Interpreter) -> R<Option<i32>> {
    let v = it.call_builtin_fn("env::var", vec![Value::str("RUST_LOG")])?;
    Ok(match &v {
        Value::Enum(e) if &*e.ty == "Result" && e.idx == 0 => match &e.payload {
            Payload::Tuple(p) => match p.first() {
                Some(Value::Str(t)) => level_of(t),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    })
}

fn level_of(v: &str) -> Option<i32> {
    match v.to_ascii_lowercase().as_str() {
        "error" => Some(0),
        "warn" | "warning" => Some(1),
        "info" => Some(2),
        "debug" => Some(3),
        "trace" => Some(4),
        _ => None,
    }
}

fn glob_match(p: &[u8], t: &[u8]) -> bool {
    match (p.first(), t.first()) {
        (None, None) => true,
        (Some(b'*'), _) => glob_match(&p[1..], t) || (!t.is_empty() && glob_match(p, &t[1..])),
        (Some(b'?'), Some(_)) => glob_match(&p[1..], &t[1..]),
        (Some(a), Some(b)) if a == b => glob_match(&p[1..], &t[1..]),
        _ => false,
    }
}

fn run_command(program: &str, args: &[String]) -> R {
    // the same parse the capability gate used to decide which program this is (scope_match::parse_command)
    let out = if args.is_empty() {
        let (prog, argv) = crate::ir::effect_facts::scope_match::parse_command(program);
        std::process::Command::new(prog).args(argv).output()
    } else {
        std::process::Command::new(program).args(args).output()
    };
    Ok(match out {
        Ok(o) if o.status.success() => Value::ok(Value::str(String::from_utf8_lossy(&o.stdout).to_string())),
        Ok(o) => Value::err(Value::str(String::from_utf8_lossy(&o.stderr).to_string())),
        Err(e) => Value::err(Value::str(e.to_string())),
    })
}

impl Interpreter {
    pub(crate) fn log_enabled(&self, level: i32) -> bool {
        matches!(self.log_level, Some(l) if level <= l)
    }

    pub(crate) fn log_emit(&mut self, level: i32, msg: &str, kv: Option<(Rc<String>, Rc<String>)>) {
        if !self.log_enabled(level) {
            return;
        }
        let name = ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"][level as usize];
        let ts = chrono::DateTime::from_timestamp((self.wall_ns() / 1_000_000_000) as i64, 0)
            .map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .unwrap_or_default();
        let extra = kv.map(|(k, v)| format!(" {}={}", k, v)).unwrap_or_default();
        self.write_err(&format!("[{} {:<5} windjammer_runtime::log_mod] {}{}\n", ts, name, msg, extra));
    }
}
