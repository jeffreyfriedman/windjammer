//! Module-qualified standard-library functions that compiled programs reach through the runtime
//! (`windjammer_runtime::{encoding, mime, math, env, fs, process, regex}`) and that the interpreter
//! lacked: `encoding.base64_encode_string(..)`, `encoding.url_decode(..)`, `hex_*`, `form_parse`,
//! `regex.find_all(..)` / `regex.escape(..)`, `mime.from_filename(..)`, `math.TAU`, ...
//!
//! Semantics follow the runtime functions of the same name (same crates, same error texts).
//! The agreement gate `tests/std_interpreter_agreement_test.rs` keeps this surface from drifting.

use super::*;
use base64::{engine::general_purpose, Engine as _};

fn s(v: &str) -> Value {
    Value::str(v.to_string())
}

fn bytes_of(it: &mut Interpreter, v: Value) -> R<Vec<u8>> {
    Ok(it.to_items(v)?.into_iter().map(|x| if let Value::Int(n, _) = x { n as u8 } else { 0 }).collect())
}

fn bytes_value(b: &[u8]) -> Value {
    Value::vec(b.iter().map(|x| Value::Int(*x as i128, IntTy::U8)).collect())
}

fn pairs_of(it: &mut Interpreter, v: Value) -> R<Vec<(String, String)>> {
    let mut out = Vec::new();
    for p in it.to_items(v)? {
        let items = match p {
            Value::Tuple(t) => t.to_vec(),
            other => it.to_items(other)?,
        };
        let text = |x: Option<&Value>| match x {
            Some(Value::Str(t)) => (**t).clone(),
            _ => String::new(),
        };
        out.push((text(items.first()), text(items.get(1))));
    }
    Ok(out)
}

/// `urlencoding::encode`: everything except unreserved characters is percent-encoded (upper-case hex).
fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// `urlencoding::decode`: `%XX` becomes a byte, a malformed escape stays literal, the result must be UTF-8.
fn percent_decode(text: &str) -> Result<String, String> {
    let bytes = text.as_bytes();
    let hex = |b: u8| (b as char).to_digit(16).map(|d| d as u8);
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match (bytes[i], bytes.get(i + 1).and_then(|b| hex(*b)), bytes.get(i + 2).and_then(|b| hex(*b))) {
            (b'%', Some(hi), Some(lo)) => {
                out.push(hi * 16 + lo);
                i += 3;
            }
            (b, _, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|e| e.to_string())
}

fn hex_decode(text: &str) -> Result<Vec<u8>, String> {
    // the runtime slices two bytes at a time: an odd length or a multi-byte character is a parse error
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let pair = text.get(i..i + 2).ok_or_else(|| "cannot parse integer from empty string".to_string())?;
        out.push(u8::from_str_radix(pair, 16).map_err(|e| e.to_string())?);
        i += 2;
    }
    Ok(out)
}

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

pub fn register(r: &mut Registry) {
    register_encoding(r);
    register_misc(r);
    register_path_and_aliases(r);
}

/// A runtime function that is another registered function under its runtime name.
macro_rules! alias {
    ($r:expr, $name:literal => $target:literal) => {
        $r.func($name, |it, a| match it.builtins.function($target) {
            Some(f) => f(it, a),
            None => Err(semantic(concat!($target, " is not registered"))),
        });
    };
}

/// `windjammer_runtime::path` takes and returns paths; the interpreter's paths are strings, so the
/// functions that exist as `fs::` builtins are the same function, and the rest are string operations.
fn register_path_and_aliases(r: &mut Registry) {
    alias!(r, "path::exists" => "fs::exists");
    alias!(r, "path::is_file" => "fs::is_file");
    alias!(r, "path::is_dir" => "fs::is_dir");
    alias!(r, "path::is_absolute" => "fs::is_absolute");
    alias!(r, "path::is_relative" => "fs::is_relative");
    alias!(r, "path::extension" => "fs::extension");
    alias!(r, "path::file_stem" => "fs::file_stem");
    alias!(r, "path::parent" => "fs::parent");
    alias!(r, "path::file_name_path" => "path::file_name");
    alias!(r, "path::join_path" => "path::join");
    alias!(r, "path::canonicalize" => "fs::canonicalize");
    r.func("path::current_dir", |_, _| {
        Ok(match std::env::current_dir() {
            Ok(p) => Value::ok(s(&p.to_string_lossy())),
            Err(e) => Value::err(s(&format!("Failed to get current directory: {e}"))),
        })
    });
    alias!(r, "random::int_range" => "random::range");
    alias!(r, "uuid::new_v4" => "uuid::v4");
    alias!(r, "crypto::sha256_string" => "crypto::sha256");
    r.func("path::new", |_, a| Ok(s(arg_str(&a, 0, "new")?.as_str())));
    r.func("path::from_str", |_, a| Ok(s(arg_str(&a, 0, "from_str")?.as_str())));
    r.func("path::to_string_lossy", |_, a| Ok(s(arg_str(&a, 0, "to_string_lossy")?.as_str())));
    r.func("path::as_os_str", |_, a| Ok(s(arg_str(&a, 0, "as_os_str")?.as_str())));
    r.func("path::to_string", |_, a| Ok(Value::some(s(arg_str(&a, 0, "to_string")?.as_str()))));
    r.func("path::components", |_, a| {
        let p = arg_str(&a, 0, "components")?;
        Ok(Value::vec(std::path::Path::new(p.as_str()).components().map(|c| s(&c.as_os_str().to_string_lossy())).collect()))
    });
    r.func("path::has_extension", |_, a| {
        let p = arg_str(&a, 0, "has_extension")?;
        let want = arg_str(&a, 1, "has_extension")?;
        Ok(Value::Bool(std::path::Path::new(p.as_str()).extension().and_then(|e| e.to_str()).is_some_and(|e| e == want.as_str())))
    });
    r.func("path::strip_prefix", |_, a| {
        let p = arg_str(&a, 0, "strip_prefix")?;
        let prefix = arg_str(&a, 1, "strip_prefix")?;
        Ok(Value::option(std::path::Path::new(p.as_str()).strip_prefix(prefix.as_str()).ok().map(|r| s(&r.to_string_lossy()))))
    });
}

fn register_encoding(r: &mut Registry) {
    r.func("encoding::base64_encode", |it, a| {
        let b = bytes_of(it, arg(&a, 0, "base64_encode")?)?;
        Ok(s(&general_purpose::STANDARD.encode(b)))
    });
    r.func("encoding::base64_encode_string", |_, a| {
        Ok(s(&general_purpose::STANDARD.encode(arg_str(&a, 0, "base64_encode_string")?.as_bytes())))
    });
    r.func("encoding::base64_decode", |_, a| {
        Ok(match general_purpose::STANDARD.decode(arg_str(&a, 0, "base64_decode")?.as_str()) {
            Ok(b) => Value::ok(bytes_value(&b)),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });
    r.func("encoding::base64_decode_string", |_, a| {
        let decoded = general_purpose::STANDARD
            .decode(arg_str(&a, 0, "base64_decode_string")?.as_str())
            .map_err(|e| e.to_string())
            .and_then(|b| String::from_utf8(b).map_err(|e| e.to_string()));
        Ok(match decoded {
            Ok(t) => Value::ok(s(&t)),
            Err(e) => Value::err(s(&e)),
        })
    });
    r.func("encoding::hex_encode", |it, a| {
        let b = bytes_of(it, arg(&a, 0, "hex_encode")?)?;
        Ok(s(&hex_encode(&b)))
    });
    r.func("encoding::hex_encode_string", |_, a| Ok(s(&hex_encode(arg_str(&a, 0, "hex_encode_string")?.as_bytes()))));
    r.func("encoding::hex_decode", |_, a| {
        Ok(match hex_decode(arg_str(&a, 0, "hex_decode")?.as_str()) {
            Ok(b) => Value::ok(bytes_value(&b)),
            Err(e) => Value::err(s(&e)),
        })
    });
    r.func("encoding::hex_decode_string", |_, a| {
        let decoded = hex_decode(arg_str(&a, 0, "hex_decode_string")?.as_str())
            .and_then(|b| String::from_utf8(b).map_err(|e| e.to_string()));
        Ok(match decoded {
            Ok(t) => Value::ok(s(&t)),
            Err(e) => Value::err(s(&e)),
        })
    });
    for name in ["encoding::url_encode", "encoding::url_encode_component"] {
        r.func(name, |_, a| Ok(s(&percent_encode(arg_str(&a, 0, "url_encode")?.as_str()))));
    }
    for name in ["encoding::url_decode", "encoding::url_decode_component"] {
        r.func(name, |_, a| {
            Ok(match percent_decode(arg_str(&a, 0, "url_decode")?.as_str()) {
                Ok(t) => Value::ok(s(&t)),
                Err(e) => Value::err(s(&e)),
            })
        });
    }
    r.func("encoding::form_parse", |_, a| {
        let text = arg_str(&a, 0, "form_parse")?;
        let text = text.strip_prefix('?').unwrap_or(text.as_str());
        Ok(Value::vec(
            url::form_urlencoded::parse(text.as_bytes()).map(|(k, v)| Value::tuple(vec![s(&k), s(&v)])).collect(),
        ))
    });
    r.func("encoding::form_stringify", |it, a| {
        let pairs = pairs_of(it, arg(&a, 0, "form_stringify")?)?;
        Ok(s(&url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .finish()))
    });
}

fn register_misc(r: &mut Registry) {
    r.add_const("math::TAU", Value::float(std::f64::consts::TAU));
    r.add_const("math::SQRT_2", Value::float(std::f64::consts::SQRT_2));
    r.func("math::pow_f64", |_, a| Ok(Value::float(arg_f64(&a, 0, "pow_f64")?.powf(arg_f64(&a, 1, "pow_f64")?))));
    r.func("math::pow_i32", |_, a| {
        let (base, exp) = (arg_int(&a, 0, "pow_i32")?, arg_int(&a, 1, "pow_i32")?);
        match i32::try_from(base).ok().zip(u32::try_from(exp).ok()).and_then(|(b, e)| b.checked_pow(e)) {
            Some(v) => Ok(Value::Int(v as i128, IntTy::I32)),
            None => Err(panic_ctl("attempt to multiply with overflow")),
        }
    });

    r.func("env::set_var", |_, a| {
        std::env::set_var(arg_str(&a, 0, "set_var")?.as_str(), arg_str(&a, 1, "set_var")?.as_str());
        Ok(Value::Unit)
    });
    r.func("env::remove_var", |_, a| {
        std::env::remove_var(arg_str(&a, 0, "remove_var")?.as_str());
        Ok(Value::Unit)
    });
    r.func("env::vars", |_, _| {
        Ok(Value::vec(std::env::vars().map(|(k, v)| Value::tuple(vec![s(&k), s(&v)])).collect()))
    });
    r.func("env::current_exe", |_, _| {
        Ok(match std::env::current_exe() {
            Ok(p) => Value::ok(s(&p.to_string_lossy())),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });

    r.func("fs::read_bytes", |_, a| {
        Ok(match std::fs::read(arg_str(&a, 0, "read_bytes")?.as_str()) {
            Ok(b) => Value::ok(bytes_value(&b)),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });
    r.func("fs::write_string", |_, a| {
        Ok(match std::fs::write(arg_str(&a, 0, "write_string")?.as_str(), arg_str(&a, 1, "write_string")?.as_bytes()) {
            Ok(()) => Value::ok(Value::Unit),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });

    r.func("mime::is_application", |_, a| Ok(Value::Bool(arg_str(&a, 0, "is_application")?.starts_with("application/"))));
    // `from_filename` is `from_path` under its runtime name
    r.func("mime::from_filename", |it, a| match it.builtins.function("mime::from_path") {
        Some(f) => f(it, a),
        None => Err(semantic("mime::from_path is not registered")),
    });

    r.func("process::run_with_output", |it, a| {
        let args: Vec<String> = it
            .to_items(arg(&a, 1, "run_with_output")?)?
            .iter()
            .filter_map(|v| if let Value::Str(x) = v { Some((**x).clone()) } else { None })
            .collect();
        Ok(match std::process::Command::new(arg_str(&a, 0, "run_with_output")?.as_str()).args(&args).output() {
            Ok(o) => Value::ok(Value::Struct(Rc::new(StructVal {
                name: Rc::from("ProcessOutput"),
                fields: vec![
                    (Rc::from("status"), Value::Int(o.status.code().unwrap_or(-1) as i128, IntTy::I32)),
                    (Rc::from("stdout"), s(&String::from_utf8_lossy(&o.stdout))),
                    (Rc::from("stderr"), s(&String::from_utf8_lossy(&o.stderr))),
                ],
                copy: false,
                uid: 0,
            }))),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });
}
