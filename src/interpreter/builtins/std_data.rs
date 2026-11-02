//! Data-format std modules: `std::json`, `std::regex`, `std::csv`, `std::url`, `std::mime`,
//! `std::crypto` (hashes and base64) and `std::time` (`DateTime`). Backed by the same crates as
//! `windjammer-runtime`, so results are identical to compiled programs.

use super::*;
use crate::interpreter::native::{downcast, Native};
use crate::parser::Type;
use std::any::Any;

fn s(v: &str) -> Value {
    Value::str(v.to_string())
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

fn struct_val(name: &str, fields: Vec<(&str, Value)>) -> Value {
    Value::Struct(Rc::new(StructVal {
        name: Rc::from(name),
        fields: fields.into_iter().map(|(n, v)| (Rc::from(n), v)).collect(),
        copy: false,
        uid: 0,
    }))
}

fn field(v: &Value, name: &str) -> Option<Value> {
    match v {
        Value::Struct(sv) => sv.fields.iter().find(|(n, _)| &**n == name).map(|(_, x)| x.clone()),
        _ => None,
    }
}

fn strings_of(it: &mut Interpreter, v: Value) -> R<Vec<String>> {
    Ok(it.to_items(v)?.iter().map(|x| if let Value::Str(t) = x { (**t).clone() } else { String::new() }).collect())
}

fn bytes_of(it: &mut Interpreter, v: Value) -> R<Vec<u8>> {
    Ok(it.to_items(v)?.into_iter().map(|x| if let Value::Int(n, _) = x { n as u8 } else { 0 }).collect())
}

fn bytes_value(b: &[u8]) -> Value {
    Value::vec(b.iter().map(|x| Value::Int(*x as i128, IntTy::U8)).collect())
}

pub fn register(r: &mut Registry) {
    for t in ["Value", "Regex", "Match", "Captures", "Url", "DateTime"] {
        r.type_name(t);
    }
    register_json(r);
    register_regex(r);
    register_csv_url_mime(r);
    register_crypto(r);
    register_time(r);
}

// ======================================================================= json

struct JsonObj(RefCell<serde_json::Value>);
native!(JsonObj, "JsonValue");

fn jv(v: serde_json::Value) -> Value {
    Value::Native(Rc::new(JsonObj(RefCell::new(v))))
}

fn json_of(v: &Value) -> R<serde_json::Value> {
    downcast::<JsonObj>(v).map(|j| j.0.borrow().clone()).ok_or_else(|| semantic(format!("expected a json::Value, found {}", v.type_name())))
}

/// Host `json::Value` objects encode as themselves inside typed values.
fn native_json(v: &Value) -> Option<serde_json::Value> {
    downcast::<JsonObj>(v).map(|j| j.0.borrow().clone())
}

fn encoded(it: &Interpreter, v: &Value, pretty: bool) -> Value {
    let wire = |n: &str| it.structs.get(n).map(|d| d.wire.clone());
    match super::json_codec::encode(v, pretty, native_json, &wire) {
        Ok(t) => Value::ok(s(&t)),
        Err(e) => Value::err(s(&e)),
    }
}

fn register_json(r: &mut Registry) {
    r.func("json::parse", |_, a| {
        Ok(match serde_json::from_str::<serde_json::Value>(arg_str(&a, 0, "parse")?.as_str()) {
            Ok(v) => Value::ok(jv(v)),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });
    // a json::Value, or any encodable value (the runtime's `stringify` is generic over Serialize)
    r.func("json::stringify", |it, a| Ok(encoded(it, &arg(&a, 0, "stringify")?, false)));
    r.func("json::stringify_pretty", |it, a| Ok(encoded(it, &arg(&a, 0, "stringify_pretty")?, true)));
    r.func("json::to_string", |it, a| Ok(encoded(it, &arg(&a, 0, "to_string")?, false)));
    r.func("json::to_string_pretty", |it, a| Ok(encoded(it, &arg(&a, 0, "to_string_pretty")?, true)));
    r.func("json::parse_string", |it, a| {
        let ty = it.type_hint.take().ok_or_else(|| {
            unsupported("`json::parse_string` needs its target type: `json::parse_string::<T>(text)` or `let x: Result<T, string> = ...`")
        })?;
        let ty = match ty {
            Type::Result(ok, _) => *ok,
            other => other,
        };
        let text = arg_str(&a, 0, "parse_string")?;
        Ok(match super::json_codec::decode(it, text.as_str(), &ty) {
            Ok(v) => Value::ok(v),
            Err(m) => Value::err(s(&m)),
        })
    });
    for (n, f) in [
        ("is_object", (|j: &serde_json::Value| j.is_object()) as fn(&serde_json::Value) -> bool),
        ("is_array", |j| j.is_array()),
        ("is_string", |j| j.is_string()),
        ("is_number", |j| j.is_number()),
        ("is_bool", |j| j.is_boolean()),
        ("is_null", |j| j.is_null()),
    ] {
        let _ = (n, f);
    }
    macro_rules! pred {
        ($path:literal, $m:ident) => {
            r.func($path, |_, a| Ok(Value::Bool(json_of(&arg(&a, 0, $path)?)?.$m())));
        };
    }
    pred!("json::is_object", is_object);
    pred!("json::is_array", is_array);
    pred!("json::is_string", is_string);
    pred!("json::is_number", is_number);
    pred!("json::is_bool", is_boolean);
    pred!("json::is_null", is_null);
    r.func("json::get", |_, a| {
        let j = json_of(&arg(&a, 0, "get")?)?;
        Ok(Value::option(j.get(arg_str(&a, 1, "get")?.as_str()).cloned().map(jv)))
    });
    r.func("json::get_index", |_, a| {
        let j = json_of(&arg(&a, 0, "get_index")?)?;
        Ok(Value::option(j.get(arg_usize(&a, 1, "get_index")?).cloned().map(jv)))
    });
    r.func("json::as_str", |_, a| Ok(Value::option(json_of(&arg(&a, 0, "as_str")?)?.as_str().map(s))));
    r.func("json::as_i64", |_, a| {
        Ok(Value::option(json_of(&arg(&a, 0, "as_i64")?)?.as_i64().map(|n| Value::Int(n as i128, IntTy::I64))))
    });
    r.func("json::as_f64", |_, a| Ok(Value::option(json_of(&arg(&a, 0, "as_f64")?)?.as_f64().map(Value::float))));
    r.func("json::as_bool", |_, a| Ok(Value::option(json_of(&arg(&a, 0, "as_bool")?)?.as_bool().map(Value::Bool))));
    r.func("json::len", |_, a| {
        let j = json_of(&arg(&a, 0, "len")?)?;
        Ok(Value::usize(match &j {
            serde_json::Value::Array(x) => x.len(),
            serde_json::Value::Object(o) => o.len(),
            serde_json::Value::String(t) => t.len(),
            _ => 0,
        }))
    });
    r.func("json::is_empty", |_, a| {
        let j = json_of(&arg(&a, 0, "is_empty")?)?;
        Ok(Value::Bool(match &j {
            serde_json::Value::Array(x) => x.is_empty(),
            serde_json::Value::Object(o) => o.is_empty(),
            serde_json::Value::String(t) => t.is_empty(),
            _ => true,
        }))
    });
    r.func("json::keys", |_, a| {
        let j = json_of(&arg(&a, 0, "keys")?)?;
        Ok(Value::vec(j.as_object().map(|o| o.keys().map(|k| s(k)).collect()).unwrap_or_default()))
    });
    r.func("json::set", |_, a| {
        let target = arg(&a, 0, "set")?;
        let obj = downcast::<JsonObj>(&target).ok_or_else(|| semantic("json::set needs a json::Value"))?;
        let key = arg_str(&a, 1, "set")?;
        let new = json_of(&arg(&a, 2, "set")?)?;
        let mut cur = obj.0.borrow_mut();
        Ok(match cur.as_object_mut() {
            Some(o) => {
                o.insert((*key).clone(), new);
                Value::ok(Value::Unit)
            }
            None => Value::err(s("not a JSON object")),
        })
    });
    r.func("json::object", |_, _| Ok(jv(serde_json::Value::Object(Default::default()))));
    r.func("json::array", |_, _| Ok(jv(serde_json::Value::Array(vec![]))));
    r.func("json::null", |_, _| Ok(jv(serde_json::Value::Null)));
    r.func("json::boolean", |_, a| Ok(jv(serde_json::Value::Bool(matches!(arg(&a, 0, "boolean")?, Value::Bool(true))))));
    r.func("json::number_i64", |_, a| Ok(jv(serde_json::Value::from(arg_int(&a, 0, "number_i64")? as i64))));
    r.func("json::json_string", |_, a| Ok(jv(serde_json::Value::String(arg_str(&a, 0, "json_string")?.to_string()))));
    r.native("JsonValue", "clone", |_, v, _| Ok(jv(json_of(v)?)));
}

// ===================================================================== regex

struct RegexObj(regex::Regex);
native!(RegexObj, "Regex");

fn re_of(v: &Value) -> R<&regex::Regex> {
    downcast::<RegexObj>(v).map(|r| &r.0).ok_or_else(|| semantic("expected a Regex"))
}

fn match_val(m: regex::Match) -> Value {
    struct_val("Match", vec![("text", s(m.as_str())), ("start", Value::Int(m.start() as i128, IntTy::I64)), ("end", Value::Int(m.end() as i128, IntTy::I64))])
}

fn captures_val(re: &regex::Regex, c: &regex::Captures) -> Value {
    let groups: Vec<Value> = (0..c.len()).map(|i| Value::option(c.get(i).map(|m| s(m.as_str())))).collect();
    let mut named = std::collections::BTreeMap::new();
    for n in re.capture_names().flatten() {
        if let Some(m) = c.name(n) {
            named.insert(Key(s(n)), s(m.as_str()));
        }
    }
    struct_val("Captures", vec![("groups", Value::vec(groups)), ("named", Value::btree_map(named))])
}

fn build(pattern: &str, ci: bool) -> Value {
    match regex::RegexBuilder::new(pattern).case_insensitive(ci).build() {
        Ok(re) => Value::ok(Value::Native(Rc::new(RegexObj(re)))),
        Err(e) => Value::err(s(&e.to_string())),
    }
}

fn register_regex(r: &mut Registry) {
    r.func("regex::compile", |_, a| Ok(build(arg_str(&a, 0, "compile")?.as_str(), false)));
    r.func("regex::new", |_, a| Ok(build(arg_str(&a, 0, "new")?.as_str(), false)));
    r.func("regex::compile_case_insensitive", |_, a| Ok(build(arg_str(&a, 0, "compile_case_insensitive")?.as_str(), true)));
    macro_rules! one_shot {
        ($path:literal, |$re:ident, $a:ident| $body:expr) => {
            r.func($path, |_, $a| {
                Ok(match regex::Regex::new(arg_str(&$a, 0, $path)?.as_str()) {
                    Ok($re) => Value::ok($body),
                    Err(e) => Value::err(s(&e.to_string())),
                })
            });
        };
    }
    one_shot!("regex::is_match", |re, a| Value::Bool(re.is_match(arg_str(&a, 1, "is_match")?.as_str())));
    one_shot!("regex::find", |re, a| Value::option(re.find(arg_str(&a, 1, "find")?.as_str()).map(|m| s(m.as_str()))));
    one_shot!("regex::replace", |re, a| s(&re.replace(arg_str(&a, 1, "replace")?.as_str(), arg_str(&a, 2, "replace")?.as_str())));
    one_shot!("regex::replace_all", |re, a| s(&re.replace_all(arg_str(&a, 1, "replace_all")?.as_str(), arg_str(&a, 2, "replace_all")?.as_str())));
    one_shot!("regex::split", |re, a| Value::vec(re.split(arg_str(&a, 1, "split")?.as_str()).map(s).collect()));
    one_shot!("regex::find_all", |re, a| Value::vec(re.find_iter(arg_str(&a, 1, "find_all")?.as_str()).map(|m| s(m.as_str())).collect()));
    r.func("regex::escape", |_, a| Ok(s(&regex::escape(arg_str(&a, 0, "escape")?.as_str()))));
    r.func("regex::compile_with_flags", |_, a| {
        let mut b = regex::RegexBuilder::new(arg_str(&a, 0, "compile_with_flags")?.as_str());
        for f in arg_str(&a, 1, "compile_with_flags")?.chars() {
            match f {
                'i' => b.case_insensitive(true),
                'm' => b.multi_line(true),
                's' => b.dot_matches_new_line(true),
                _ => return Ok(Value::err(s(&format!("Unknown regex flag: {}", f)))),
            };
        }
        Ok(match b.build() {
            Ok(re) => Value::ok(Value::Native(Rc::new(RegexObj(re)))),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });
    // the pre-compiled forms take the `Regex` first
    r.func("regex::is_match_compiled", |_, a| Ok(Value::Bool(re_of(&super::channel::raw_arg(&a, 0, "is_match_compiled")?)?.is_match(arg_str(&a, 1, "is_match_compiled")?.as_str()))));
    r.func("regex::find_compiled", |_, a| {
        Ok(Value::option(re_of(&super::channel::raw_arg(&a, 0, "find_compiled")?)?.find(arg_str(&a, 1, "find_compiled")?.as_str()).map(|m| s(m.as_str()))))
    });
    r.func("regex::find_all_compiled", |_, a| {
        Ok(Value::vec(re_of(&super::channel::raw_arg(&a, 0, "find_all_compiled")?)?.find_iter(arg_str(&a, 1, "find_all_compiled")?.as_str()).map(|m| s(m.as_str())).collect()))
    });
    r.func("regex::replace_compiled", |_, a| {
        Ok(s(&re_of(&super::channel::raw_arg(&a, 0, "replace_compiled")?)?.replace(arg_str(&a, 1, "replace_compiled")?.as_str(), arg_str(&a, 2, "replace_compiled")?.as_str())))
    });
    r.func("regex::replace_all_compiled", |_, a| {
        Ok(s(&re_of(&super::channel::raw_arg(&a, 0, "replace_all_compiled")?)?.replace_all(arg_str(&a, 1, "replace_all_compiled")?.as_str(), arg_str(&a, 2, "replace_all_compiled")?.as_str())))
    });
    r.native("Regex", "is_match", |_, v, a| Ok(Value::Bool(re_of(v)?.is_match(arg_str(&a, 0, "is_match")?.as_str()))));
    r.native("Regex", "find", |_, v, a| Ok(Value::option(re_of(v)?.find(arg_str(&a, 0, "find")?.as_str()).map(match_val))));
    r.native("Regex", "find_all", |_, v, a| {
        Ok(Value::vec(re_of(v)?.find_iter(arg_str(&a, 0, "find_all")?.as_str()).map(match_val).collect()))
    });
    r.native("Regex", "captures", |_, v, a| {
        let re = re_of(v)?;
        Ok(Value::option(re.captures(arg_str(&a, 0, "captures")?.as_str()).map(|c| captures_val(re, &c))))
    });
    r.native("Regex", "captures_all", |_, v, a| {
        let re = re_of(v)?;
        Ok(Value::vec(re.captures_iter(arg_str(&a, 0, "captures_all")?.as_str()).map(|c| captures_val(re, &c)).collect()))
    });
    r.native("Regex", "replace", |_, v, a| {
        Ok(s(&re_of(v)?.replace(arg_str(&a, 0, "replace")?.as_str(), arg_str(&a, 1, "replace")?.as_str())))
    });
    r.native("Regex", "replace_all", |_, v, a| {
        Ok(s(&re_of(v)?.replace_all(arg_str(&a, 0, "replace_all")?.as_str(), arg_str(&a, 1, "replace_all")?.as_str())))
    });
    r.native("Regex", "split", |_, v, a| Ok(Value::vec(re_of(v)?.split(arg_str(&a, 0, "split")?.as_str()).map(s).collect())));
    r.native("Regex", "clone", |_, v, _| Ok(v.clone()));
    // Match / Captures are plain structs whose methods are builtin
    r.native("Match", "text", |_, v, _| Ok(field(v, "text").unwrap_or(Value::Unit)));
    r.native("Match", "start", |_, v, _| Ok(field(v, "start").unwrap_or(Value::Unit)));
    r.native("Match", "end", |_, v, _| Ok(field(v, "end").unwrap_or(Value::Unit)));
    r.native("Match", "range", |_, v, _| Ok(Value::tuple(vec![field(v, "start").unwrap_or(Value::Unit), field(v, "end").unwrap_or(Value::Unit)])));
    fn group(v: &Value, i: usize) -> Value {
        match field(v, "groups") {
            Some(Value::Vec(g)) => g.get(i).cloned().unwrap_or_else(Value::none),
            _ => Value::none(),
        }
    }
    r.native("Captures", "full_match", |_, v, _| Ok(group(v, 0)));
    r.native("Captures", "get", |_, v, a| Ok(group(v, arg_usize(&a, 0, "get")?)));
    r.native("Captures", "groups", |_, v, _| Ok(field(v, "groups").unwrap_or_else(|| Value::vec(vec![]))));
    r.native("Captures", "len", |_, v, _| {
        Ok(Value::Int(match field(v, "groups") {
            Some(Value::Vec(g)) => g.len() as i128,
            _ => 0,
        }, IntTy::I64))
    });
    r.native("Captures", "name", |_, v, a| {
        let n = arg_str(&a, 0, "name")?;
        Ok(match field(v, "named") {
            Some(Value::Map(m)) => Value::option(m.get(&Key(Value::str((*n).clone()))).cloned()),
            _ => Value::none(),
        })
    });
}

// ================================================================ csv/url/mime

fn register_csv_url_mime(r: &mut Registry) {
    // std declares these as returning the Rust `csv` crate's builder types, which have no
    // Windjammer surface (and no runtime counterpart): loud, pointing at the supported API
    r.func("csv::reader_with_delimiter", |_, _| {
        Err(unsupported("`csv::reader_with_delimiter` returns the Rust csv crate's builder, which has no Windjammer API; use `csv::parse`"))
    });
    r.func("csv::writer_with_delimiter", |_, _| {
        Err(unsupported("`csv::writer_with_delimiter` returns the Rust csv crate's builder, which has no Windjammer API; use `csv::write`"))
    });
    r.func("csv::parse", |it, a| csv_parse(it, &arg_str(&a, 0, "parse")?, b',', false));
    r.func("csv::parse_bytes", |it, a| {
        let b = bytes_of(it, arg(&a, 0, "parse_bytes")?)?;
        csv_parse(it, &String::from_utf8_lossy(&b), b',', false)
    });
    r.func("csv::parse_with_headers", |it, a| csv_parse(it, &arg_str(&a, 0, "parse_with_headers")?, b',', true));
    r.func("csv::write", |it, a| {
        let rows = rows_of(it, arg(&a, 0, "write")?)?;
        csv_write(None, &rows)
    });
    r.func("csv::write_with_headers", |it, a| {
        let head = strings_of(it, arg(&a, 0, "write_with_headers")?)?;
        let rows = rows_of(it, arg(&a, 1, "write_with_headers")?)?;
        csv_write(Some(head), &rows)
    });

    r.func("url::parse", |_, a| {
        Ok(match url::Url::parse(arg_str(&a, 0, "parse")?.as_str()) {
            Ok(u) => Value::ok(struct_val(
                "Url",
                vec![
                    ("scheme", s(u.scheme())),
                    ("host", s(u.host_str().unwrap_or(""))),
                    ("path", s(u.path())),
                    ("query", Value::option(u.query().map(s))),
                    ("fragment", Value::option(u.fragment().map(s))),
                ],
            )),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });
    r.func("url::format", |_, a| {
        let u = arg(&a, 0, "format")?;
        let f = |n: &str| match field(&u, n) {
            Some(Value::Str(t)) => (*t).clone(),
            _ => String::new(),
        };
        let opt = |n: &str| match field(&u, n) {
            Some(v) => v.as_option().and_then(|o| o.cloned()).and_then(|x| if let Value::Str(t) = x { Some((*t).clone()) } else { None }),
            None => None,
        };
        let (scheme, host, path) = (f("scheme"), f("host"), f("path"));
        let mut out = String::new();
        if !scheme.is_empty() {
            out.push_str(&scheme);
            out.push_str("://");
        }
        out.push_str(&host);
        if path.is_empty() && !host.is_empty() {
            out.push('/');
        } else {
            out.push_str(&path);
        }
        if let Some(q) = opt("query") {
            out.push('?');
            out.push_str(&q);
        }
        if let Some(fr) = opt("fragment") {
            out.push('#');
            out.push_str(&fr);
        }
        Ok(s(&out))
    });
    r.func("url::join", |_, a| {
        let rel = arg_str(&a, 1, "join")?;
        Ok(match url::Url::parse(arg_str(&a, 0, "join")?.as_str()).and_then(|b| b.join(rel.as_str())) {
            Ok(u) => Value::ok(s(u.as_str())),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });

    for (n, v) in MIME_CONSTS {
        r.add_const(format!("mime::{}", n), s(v));
    }
    r.func("mime::from_extension", |_, a| Ok(s(&mime_for_ext(arg_str(&a, 0, "from_extension")?.as_str()))));
    r.func("mime::from_path", |_, a| {
        let p = arg_str(&a, 0, "from_path")?;
        let path = std::path::Path::new(p.as_str());
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if let Some(m) = known_mime(&ext.trim().trim_start_matches('.').to_ascii_lowercase()) {
                return Ok(s(m));
            }
        }
        Ok(s(&mime_guess::from_path(path).first_or_octet_stream().to_string()))
    });
    r.func("mime::is_text", |_, a| {
        let m = arg_str(&a, 0, "is_text")?;
        Ok(Value::Bool(["text/", "application/json", "application/javascript", "application/xml"].iter().any(|p| m.starts_with(p))))
    });
    r.func("mime::is_image", |_, a| Ok(Value::Bool(arg_str(&a, 0, "is_image")?.starts_with("image/"))));
    r.func("mime::is_audio", |_, a| Ok(Value::Bool(arg_str(&a, 0, "is_audio")?.starts_with("audio/"))));
    r.func("mime::is_video", |_, a| Ok(Value::Bool(arg_str(&a, 0, "is_video")?.starts_with("video/"))));
}

fn rows_of(it: &mut Interpreter, v: Value) -> R<Vec<Vec<String>>> {
    let mut out = Vec::new();
    for row in it.to_items(v)? {
        out.push(strings_of(it, row)?);
    }
    Ok(out)
}

fn csv_parse(_: &mut Interpreter, data: &str, delim: u8, with_headers: bool) -> R {
    let mut rd = csv::ReaderBuilder::new().has_headers(false).delimiter(delim).from_reader(std::io::Cursor::new(data));
    let mut rows: Vec<Vec<String>> = Vec::new();
    for rec in rd.records() {
        match rec {
            Ok(r) => rows.push(r.iter().map(|f| f.to_string()).collect()),
            Err(e) => return Ok(Value::err(s(&e.to_string()))),
        }
    }
    let to_val = |rows: Vec<Vec<String>>| Value::vec(rows.into_iter().map(|r| Value::vec(r.iter().map(|f| s(f)).collect())).collect());
    if with_headers {
        let head = if rows.is_empty() { Vec::new() } else { rows.remove(0) };
        return Ok(Value::ok(Value::tuple(vec![Value::vec(head.iter().map(|f| s(f)).collect()), to_val(rows)])));
    }
    Ok(Value::ok(to_val(rows)))
}

fn csv_write(head: Option<Vec<String>>, rows: &[Vec<String>]) -> R {
    let mut w = csv::Writer::from_writer(vec![]);
    if let Some(h) = head {
        if let Err(e) = w.write_record(&h) {
            return Ok(Value::err(s(&e.to_string())));
        }
    }
    for r in rows {
        if let Err(e) = w.write_record(r) {
            return Ok(Value::err(s(&e.to_string())));
        }
    }
    Ok(match w.into_inner() {
        Ok(d) => Value::ok(s(&String::from_utf8_lossy(&d))),
        Err(e) => Value::err(s(&e.to_string())),
    })
}

const MIME_CONSTS: &[(&str, &str)] = &[
    ("TEXT_HTML", "text/html; charset=utf-8"),
    ("TEXT_PLAIN", "text/plain; charset=utf-8"),
    ("TEXT_CSS", "text/css; charset=utf-8"),
    ("TEXT_XML", "text/xml; charset=utf-8"),
    ("TEXT_CSV", "text/csv; charset=utf-8"),
    ("APPLICATION_JSON", "application/json; charset=utf-8"),
    ("APPLICATION_JAVASCRIPT", "application/javascript; charset=utf-8"),
    ("APPLICATION_WASM", "application/wasm"),
    ("APPLICATION_PDF", "application/pdf"),
    ("APPLICATION_ZIP", "application/zip"),
    ("APPLICATION_OCTET_STREAM", "application/octet-stream"),
    ("APPLICATION_XML", "application/xml; charset=utf-8"),
    ("APPLICATION_TYPESCRIPT", "application/x-typescript"),
    ("APPLICATION_SOURCEMAP", "application/json"),
    ("IMAGE_PNG", "image/png"),
    ("IMAGE_JPEG", "image/jpeg"),
    ("IMAGE_GIF", "image/gif"),
    ("IMAGE_SVG", "image/svg+xml"),
    ("IMAGE_WEBP", "image/webp"),
    ("IMAGE_ICO", "image/x-icon"),
    ("AUDIO_MPEG", "audio/mpeg"),
    ("AUDIO_OGG", "audio/ogg"),
    ("AUDIO_WAV", "audio/wav"),
    ("AUDIO_WEBM", "audio/webm"),
    ("VIDEO_MP4", "video/mp4"),
    ("VIDEO_WEBM", "video/webm"),
    ("VIDEO_OGG", "video/ogg"),
    ("FONT_WOFF", "font/woff"),
    ("FONT_WOFF2", "font/woff2"),
    ("FONT_TTF", "font/ttf"),
    ("FONT_OTF", "font/otf"),
];

fn known_mime(ext: &str) -> Option<&'static str> {
    let c = |n: &str| MIME_CONSTS.iter().find(|(k, _)| *k == n).map(|(_, v)| *v);
    match ext {
        "html" | "htm" => c("TEXT_HTML"),
        "txt" => c("TEXT_PLAIN"),
        "css" => c("TEXT_CSS"),
        "xml" => c("APPLICATION_XML"),
        "csv" => c("TEXT_CSV"),
        "js" | "mjs" => c("APPLICATION_JAVASCRIPT"),
        "json" => c("APPLICATION_JSON"),
        "wasm" => c("APPLICATION_WASM"),
        "pdf" => c("APPLICATION_PDF"),
        "zip" => c("APPLICATION_ZIP"),
        "png" => c("IMAGE_PNG"),
        "jpg" | "jpeg" => c("IMAGE_JPEG"),
        "gif" => c("IMAGE_GIF"),
        "svg" => c("IMAGE_SVG"),
        "webp" => c("IMAGE_WEBP"),
        "ico" => c("IMAGE_ICO"),
        "mp3" => c("AUDIO_MPEG"),
        "ogg" => c("AUDIO_OGG"),
        "wav" => c("AUDIO_WAV"),
        "mp4" => c("VIDEO_MP4"),
        "webm" => c("VIDEO_WEBM"),
        "ogv" => c("VIDEO_OGG"),
        "woff" => c("FONT_WOFF"),
        "woff2" => c("FONT_WOFF2"),
        "ttf" => c("FONT_TTF"),
        "otf" => c("FONT_OTF"),
        "ts" => c("APPLICATION_TYPESCRIPT"),
        "map" => c("APPLICATION_SOURCEMAP"),
        _ => None,
    }
}

fn mime_for_ext(ext: &str) -> String {
    let n = ext.trim().trim_start_matches('.').to_ascii_lowercase();
    match known_mime(&n) {
        Some(m) => m.to_string(),
        None => mime_guess::from_ext(&n).first_or_octet_stream().to_string(),
    }
}

// =================================================================== crypto

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

fn register_crypto(r: &mut Registry) {
    use base64::{engine::general_purpose, Engine as _};
    use sha2::Digest;
    r.func("crypto::base64_encode", |_, a| Ok(s(&general_purpose::STANDARD.encode(arg_str(&a, 0, "base64_encode")?.as_bytes()))));
    r.func("crypto::base64_decode", |_, a| {
        Ok(match general_purpose::STANDARD.decode(arg_str(&a, 0, "base64_decode")?.as_bytes()) {
            Ok(b) => match String::from_utf8(b) {
                Ok(t) => Value::ok(s(&t)),
                Err(e) => Value::err(s(&e.to_string())),
            },
            Err(e) => Value::err(s(&e.to_string())),
        })
    });
    r.func("crypto::base64_encode_bytes", |it, a| {
        Ok(s(&general_purpose::STANDARD.encode(bytes_of(it, arg(&a, 0, "base64_encode_bytes")?)?)))
    });
    r.func("crypto::base64_decode_bytes", |_, a| {
        Ok(match general_purpose::STANDARD.decode(arg_str(&a, 0, "base64_decode_bytes")?.as_bytes()) {
            Ok(b) => Value::ok(bytes_value(&b)),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });
    for n in ["crypto::sha256", "crypto::sha256_hex"] {
        r.func(n, |_, a| Ok(s(&hex(&sha2::Sha256::digest(arg_str(&a, 0, "sha256")?.as_bytes())))));
    }
    r.func("crypto::sha512", |_, a| Ok(s(&hex(&sha2::Sha512::digest(arg_str(&a, 0, "sha512")?.as_bytes())))));
    r.func("crypto::sha256_bytes", |it, a| Ok(bytes_value(&sha2::Sha256::digest(bytes_of(it, arg(&a, 0, "sha256_bytes")?)?))));
    r.func("crypto::sha512_bytes", |it, a| Ok(bytes_value(&sha2::Sha512::digest(bytes_of(it, arg(&a, 0, "sha512_bytes")?)?))));
    r.func("crypto::sha1_bytes", |it, a| Ok(bytes_value(&sha1::Sha1::digest(bytes_of(it, arg(&a, 0, "sha1_bytes")?)?))));
    // HMAC-SHA-256 and constant-time comparison: pure computation, so no capability check (like the hashes above).
    r.func("crypto::hmac_sha256", |_, a| {
        Ok(s(&hex(&hmac_sha256(arg_str(&a, 0, "hmac_sha256")?.as_bytes(), arg_str(&a, 1, "hmac_sha256")?.as_bytes()))))
    });
    r.func("crypto::hmac_sha256_bytes", |it, a| {
        let key = bytes_of(it, arg(&a, 0, "hmac_sha256_bytes")?)?;
        let data = bytes_of(it, arg(&a, 1, "hmac_sha256_bytes")?)?;
        Ok(bytes_value(&hmac_sha256(&key, &data)))
    });
    r.func("crypto::verify_hmac_sha256", |_, a| {
        use hmac::Mac;
        let expected = match hex_decode(&arg_str(&a, 2, "verify_hmac_sha256")?) {
            Some(b) => b,
            None => return Ok(Value::Bool(false)),
        };
        let (key, data) = (arg_str(&a, 0, "verify_hmac_sha256")?, arg_str(&a, 1, "verify_hmac_sha256")?);
        let mut mac = <hmac::Hmac<sha2::Sha256> as Mac>::new_from_slice(key.as_bytes()).expect("HMAC accepts keys of any length");
        mac.update(data.as_bytes());
        Ok(Value::Bool(mac.verify_slice(&expected).is_ok()))
    });
    r.func("crypto::constant_time_eq", |_, a| {
        use subtle::ConstantTimeEq;
        let (xs, ys) = (arg_str(&a, 0, "constant_time_eq")?, arg_str(&a, 1, "constant_time_eq")?);
        let (x, y) = (xs.as_bytes(), ys.as_bytes());
        Ok(Value::Bool(x.len() == y.len() && bool::from(x.ct_eq(y))))
    });
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    use hmac::Mac;
    let mut mac = <hmac::Hmac<sha2::Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// Case-insensitive hex to bytes; `None` for odd length or a non-hex digit.
fn hex_decode(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    let digit = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    text.as_bytes().chunks(2).map(|p| Some(digit(p[0])? * 16 + digit(p[1])?)).collect()
}

// ===================================================================== time

struct DateTimeObj(chrono::DateTime<chrono::Utc>);
native!(DateTimeObj, "DateTime");

fn dt_of(v: &Value) -> R<chrono::DateTime<chrono::Utc>> {
    downcast::<DateTimeObj>(v).map(|d| d.0).ok_or_else(|| semantic("expected a DateTime"))
}

fn dt(v: chrono::DateTime<chrono::Utc>) -> Value {
    Value::Native(Rc::new(DateTimeObj(v)))
}

fn register_time(r: &mut Registry) {
    use chrono::{Datelike, Timelike};
    for n in ["time::now", "time::utc_now"] {
        r.func(n, |it, _| {
            let ns = it.wall_ns();
            Ok(dt(chrono::DateTime::from_timestamp((ns / 1_000_000_000) as i64, (ns % 1_000_000_000) as u32).unwrap_or_default()))
        });
    }
    r.func("time::parse_rfc3339", |_, a| {
        Ok(match chrono::DateTime::parse_from_rfc3339(arg_str(&a, 0, "parse_rfc3339")?.as_str()) {
            Ok(d) => Value::ok(dt(d.with_timezone(&chrono::Utc))),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });
    r.func("time::parse", |_, a| {
        let (text, fmt) = (arg_str(&a, 0, "parse")?, arg_str(&a, 1, "parse")?);
        let parsed = chrono::DateTime::parse_from_str(&text, &fmt)
            .map(|d| d.with_timezone(&chrono::Utc))
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(&text, &fmt).map(|n| n.and_utc()));
        Ok(match parsed {
            Ok(d) => Value::ok(dt(d)),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });
    r.func("time::to_rfc3339", |_, a| Ok(s(&dt_of(&arg(&a, 0, "to_rfc3339")?)?.to_rfc3339())));
    r.func("time::format", |_, a| Ok(s(&dt_of(&arg(&a, 0, "format")?)?.format(arg_str(&a, 1, "format")?.as_str()).to_string())));
    r.native("DateTime", "timestamp", |_, v, _| Ok(Value::Int(dt_of(v)?.timestamp() as i128, IntTy::I64)));
    r.native("DateTime", "timestamp_millis", |_, v, _| Ok(Value::Int(dt_of(v)?.timestamp_millis() as i128, IntTy::I64)));
    r.native("DateTime", "format", |_, v, a| Ok(s(&dt_of(v)?.format(arg_str(&a, 0, "format")?.as_str()).to_string())));
    r.native("DateTime", "to_rfc3339", |_, v, _| Ok(s(&dt_of(v)?.to_rfc3339())));
    r.native("DateTime", "year", |_, v, _| Ok(Value::Int(dt_of(v)?.year() as i128, IntTy::I64)));
    r.native("DateTime", "month", |_, v, _| Ok(Value::Int(dt_of(v)?.month() as i128, IntTy::I64)));
    r.native("DateTime", "day", |_, v, _| Ok(Value::Int(dt_of(v)?.day() as i128, IntTy::I64)));
    r.native("DateTime", "hour", |_, v, _| Ok(Value::Int(dt_of(v)?.hour() as i128, IntTy::I64)));
    r.native("DateTime", "minute", |_, v, _| Ok(Value::Int(dt_of(v)?.minute() as i128, IntTy::I64)));
    r.native("DateTime", "second", |_, v, _| Ok(Value::Int(dt_of(v)?.second() as i128, IntTy::I64)));
    r.native("DateTime", "add_days", |_, v, a| Ok(dt(dt_of(v)? + chrono::Duration::days(arg_int(&a, 0, "add_days")? as i64))));
    r.native("DateTime", "add_hours", |_, v, a| Ok(dt(dt_of(v)? + chrono::Duration::hours(arg_int(&a, 0, "add_hours")? as i64))));
    r.native("DateTime", "add_minutes", |_, v, a| Ok(dt(dt_of(v)? + chrono::Duration::minutes(arg_int(&a, 0, "add_minutes")? as i64))));
    r.native("DateTime", "add_seconds", |_, v, a| Ok(dt(dt_of(v)? + chrono::Duration::seconds(arg_int(&a, 0, "add_seconds")? as i64))));
    r.native("DateTime", "clone", |_, v, _| Ok(v.clone()));
    r.func("time::duration_days", |_, a| Ok(super::time::duration(arg_int(&a, 0, "duration_days")? * 86_400 * 1_000_000_000)));
    r.func("time::duration_hours", |_, a| Ok(super::time::duration(arg_int(&a, 0, "duration_hours")? * 3_600 * 1_000_000_000)));
    r.func("time::duration_minutes", |_, a| Ok(super::time::duration(arg_int(&a, 0, "duration_minutes")? * 60 * 1_000_000_000)));
    r.func("time::duration_seconds", |_, a| Ok(super::time::duration(arg_int(&a, 0, "duration_seconds")? * 1_000_000_000)));
    r.func("time::parse_duration_ms", |_, a| Ok(match parse_duration_ms(arg_str(&a, 0, "parse_duration_ms")?.as_str()) {
        Ok(n) => Value::ok(Value::Int(n as i128, IntTy::I64)),
        Err(m) => Value::err(s(&m)),
    }));
    r.func("time::format_duration_ms", |_, a| Ok(s(&format_duration_ms(arg_int(&a, 0, "format_duration_ms")? as i64))));
}

fn parse_duration_ms(text: &str) -> Result<i64, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("empty duration".to_string());
    }
    if t.chars().all(|c| c.is_ascii_digit()) {
        return t.parse::<i64>().map_err(|_| "invalid duration number".to_string());
    }
    let b = t.as_bytes();
    let (mut total, mut i, mut saw) = (0i64, 0usize, false);
    while i < b.len() {
        let st = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == st {
            return Err("expected number in duration".to_string());
        }
        let n: i64 = t[st..i].parse().map_err(|_| "invalid duration number".to_string())?;
        if i >= b.len() {
            return Err("expected unit after number".to_string());
        }
        let us = i;
        while i < b.len() && !b[i].is_ascii_digit() {
            i += 1;
        }
        let mult = match t[us..i].to_ascii_lowercase().as_str() {
            "ms" => 1,
            "s" => 1000,
            "m" => 60_000,
            "h" => 3_600_000,
            "d" => 86_400_000,
            _ => return Err("unknown duration unit".to_string()),
        };
        total = total.saturating_add(n.saturating_mul(mult));
        saw = true;
    }
    if !saw {
        return Err("invalid duration".to_string());
    }
    Ok(total)
}

fn format_duration_ms(ms: i64) -> String {
    if ms <= 0 {
        return "0ms".to_string();
    }
    if ms < 1000 {
        return format!("{ms}ms");
    }
    let mut rem = ms;
    let h = rem / 3_600_000;
    rem -= h * 3_600_000;
    let m = rem / 60_000;
    rem -= m * 60_000;
    let sec = rem / 1000;
    rem -= sec * 1000;
    let mut out = String::new();
    if h > 0 {
        out.push_str(&format!("{h}h"));
    }
    if m > 0 {
        out.push_str(&format!("{m}m"));
    }
    if sec > 0 {
        out.push_str(&format!("{sec}s"));
    }
    if rem > 0 {
        out.push_str(&format!("{rem}ms"));
    }
    out
}
