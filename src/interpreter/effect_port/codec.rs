//! Values in the effect log. Data round-trips exactly: a decoded result is indistinguishable from the live one.
//! JSON string = `string`, bool = `bool`, null = `()`; everything else is a one-key (or tagged) object.
//! A value that is a live handle (iterator, closure, thread, socket...) has no encoding: recording it is an error.

use super::super::builtins::time::{time_from_parts, time_parts};
use super::super::builtins::std_sys::{dirent_path, dirent_value, metadata_from_parts, metadata_parts};
use super::super::int::IntTy;
use super::super::value::{EnumVal, FloatTy, Payload, StructVal, Value};
use serde_json::{json, Map, Value as Json};
use std::rc::Rc;

pub fn encode(v: &Value) -> Result<Json, String> {
    Ok(match v {
        Value::Unit => Json::Null,
        Value::Bool(b) => Json::Bool(*b),
        Value::Str(s) => Json::String((**s).clone()),
        Value::Int(n, t) => json!({ "i": if matches!(t, IntTy::Lit) { "lit" } else { t.name() }, "v": n.to_string() }),
        Value::Float(f, t) => {
            let ty = match t {
                FloatTy::F32 => "f32",
                FloatTy::F64 => "f64",
                FloatTy::Lit => "lit",
            };
            // the bit pattern is the value: text would lose NaN payloads and -0.0 on some paths
            json!({ "f": ty, "bits": format!("{:016x}", f.to_bits()) })
        }
        Value::Char(c) => json!({ "c": c.to_string() }),
        Value::Vec(items) => json!({ "vec": items.iter().map(encode).collect::<Result<Vec<_>, _>>()? }),
        Value::Tuple(items) => json!({ "tup": items.iter().map(encode).collect::<Result<Vec<_>, _>>()? }),
        Value::Struct(s) => {
            let fields = s.fields.iter().map(|(n, v)| Ok(json!([&**n, encode(v)?]))).collect::<Result<Vec<_>, String>>()?;
            json!({ "st": &*s.name, "f": fields, "copy": s.copy })
        }
        Value::Enum(e) => {
            let payload = match &e.payload {
                Payload::Unit => Json::Null,
                Payload::Tuple(vs) => json!({ "t": vs.iter().map(encode).collect::<Result<Vec<_>, _>>()? }),
                Payload::Struct(fs) => {
                    json!({ "s": fs.iter().map(|(n, v)| Ok(json!([&**n, encode(v)?]))).collect::<Result<Vec<_>, String>>()? })
                }
            };
            json!({ "en": &*e.ty, "var": &*e.variant, "idx": e.idx, "p": payload })
        }
        Value::Range(a, b, t) => json!({ "range": [a.to_string(), b.to_string(), t.name()] }),
        Value::Native(_) => match (metadata_parts(v), dirent_path(v)) {
            (Some((len, file, dir, ro)), _) => json!({ "meta": [len, file, dir, ro] }),
            (_, Some(path)) => json!({ "dirent": path }),
            _ => match time_parts(v) {
                // clock values are plain nanosecond counts
                Some((kind, ns)) => json!({ "time": kind, "ns": ns.to_string() }),
                None => return Err(format!("a `{}` handle", v.type_name())),
            },
        },
        other => return Err(format!("a `{}` value", other.type_name())),
    })
}

/// A request for the log: like `encode`, but never fails (a value without an encoding is named by its type).
pub fn encode_lossy(v: &Value) -> Json {
    encode(v).unwrap_or_else(|_| json!({ "unencodable": v.type_name() }))
}

pub fn decode(j: &Json) -> Result<Value, String> {
    let bad = |what: &str| format!("malformed {what} in the effect log");
    Ok(match j {
        Json::Null => Value::Unit,
        Json::Bool(b) => Value::Bool(*b),
        Json::String(s) => Value::str(s.clone()),
        Json::Object(m) => decode_object(m, &bad)?,
        _ => return Err(bad("value")),
    })
}

fn list(j: &Json, what: &str) -> Result<Vec<Value>, String> {
    j.as_array().ok_or_else(|| format!("malformed {what} in the effect log"))?.iter().map(decode).collect()
}

fn named(j: &Json) -> Result<Vec<(Rc<str>, Value)>, String> {
    j.as_array()
        .ok_or("malformed fields in the effect log")?
        .iter()
        .map(|p| Ok((Rc::from(p[0].as_str().ok_or("malformed field name")?), decode(&p[1])?)))
        .collect()
}

fn decode_object(m: &Map<String, Json>, bad: &dyn Fn(&str) -> String) -> Result<Value, String> {
    let int_ty = |n: &str| if n == "lit" { Some(IntTy::Lit) } else { IntTy::from_name(n) };
    if let Some(t) = m.get("i") {
        let ty = int_ty(t.as_str().unwrap_or("")).ok_or_else(|| bad("integer type"))?;
        let n: i128 = m.get("v").and_then(|v| v.as_str()).and_then(|v| v.parse().ok()).ok_or_else(|| bad("integer"))?;
        return Ok(Value::Int(n, ty));
    }
    if let Some(t) = m.get("f") {
        let ty = match t.as_str() {
            Some("f32") => FloatTy::F32,
            Some("f64") => FloatTy::F64,
            Some("lit") => FloatTy::Lit,
            _ => return Err(bad("float type")),
        };
        let bits = m.get("bits").and_then(|b| b.as_str()).and_then(|b| u64::from_str_radix(b, 16).ok()).ok_or_else(|| bad("float"))?;
        return Ok(Value::Float(f64::from_bits(bits), ty));
    }
    if let Some(c) = m.get("c") {
        return c.as_str().and_then(|s| s.chars().next()).map(Value::Char).ok_or_else(|| bad("char"));
    }
    if let Some(v) = m.get("vec") {
        return Ok(Value::vec(list(v, "vec")?));
    }
    if let Some(v) = m.get("tup") {
        return Ok(Value::tuple(list(v, "tuple")?));
    }
    if let Some(n) = m.get("st") {
        let fields = named(m.get("f").ok_or_else(|| bad("struct"))?)?;
        let copy = m.get("copy").and_then(|c| c.as_bool()).unwrap_or(false);
        return Ok(Value::Struct(Rc::new(StructVal { name: Rc::from(n.as_str().ok_or_else(|| bad("struct name"))?), fields, copy, uid: 0 })));
    }
    if let Some(ty) = m.get("en") {
        let p = &m["p"];
        let payload = if p.is_null() {
            Payload::Unit
        } else if let Some(t) = p.get("t") {
            Payload::Tuple(list(t, "payload")?)
        } else {
            Payload::Struct(named(p.get("s").ok_or_else(|| bad("payload"))?)?)
        };
        let (ty, var) = (ty.as_str().ok_or_else(|| bad("enum"))?, m["var"].as_str().ok_or_else(|| bad("variant"))?);
        return Ok(Value::Enum(Rc::new(EnumVal { ty: ty.into(), variant: var.into(), idx: m["idx"].as_u64().unwrap_or(0) as u32, payload })));
    }
    if let Some(r) = m.get("range").and_then(|r| r.as_array()) {
        let n = |i: usize| r.get(i).and_then(|x| x.as_str()).and_then(|x| x.parse::<i128>().ok()).ok_or_else(|| bad("range"));
        let ty = int_ty(r.get(2).and_then(|t| t.as_str()).unwrap_or("")).ok_or_else(|| bad("range type"))?;
        return Ok(Value::Range(n(0)?, n(1)?, ty));
    }
    if let Some(kind) = m.get("time").and_then(|k| k.as_str()) {
        let ns: i128 = m.get("ns").and_then(|n| n.as_str()).and_then(|n| n.parse().ok()).ok_or_else(|| bad("time value"))?;
        return time_from_parts(kind, ns).ok_or_else(|| bad("time value"));
    }
    if let Some(p) = m.get("dirent").and_then(|p| p.as_str()) {
        return Ok(dirent_value(p.to_string()));
    }
    if let Some(a) = m.get("meta").and_then(|a| a.as_array()) {
        let b = |i: usize| a.get(i).and_then(|x| x.as_bool()).unwrap_or(false);
        return Ok(metadata_from_parts(a.first().and_then(|x| x.as_u64()).unwrap_or(0), b(1), b(2), b(3)));
    }
    Err(bad("value"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(v: Value) {
        let j = encode(&v).unwrap();
        let back = decode(&serde_json::from_str(&super::super::log::canon(&j)).unwrap()).unwrap();
        assert_eq!(format!("{v:?}"), format!("{back:?}"), "round trip of {j}");
    }

    #[test]
    fn data_round_trips() {
        rt(Value::Unit);
        rt(Value::Bool(true));
        rt(Value::str("a \"quoted\"\nline"));
        rt(Value::Int(-5, IntTy::I32));
        rt(Value::int(7));
        rt(Value::Int(u64::MAX as i128, IntTy::U64));
        rt(Value::float(0.1));
        rt(Value::float(-0.0));
        rt(Value::float(f64::NAN));
        rt(Value::Char('é'));
        rt(Value::vec(vec![Value::str("x"), Value::int(1)]));
        rt(Value::ok(Value::tuple(vec![Value::int(1), Value::Bool(false)])));
        rt(Value::err(Value::str("No such file or directory (os error 2)")));
        rt(Value::none());
        rt(Value::Range(0, 9, IntTy::Usize));
        rt(metadata_from_parts(12, true, false, false));
        rt(dirent_value("a/b.txt".into()));
        rt(time_from_parts("Instant", 1234).unwrap());
        rt(time_from_parts("Duration", -5).unwrap());
    }

    #[test]
    fn handles_have_no_encoding() {
        assert!(encode(&Value::Formatter(Rc::new(std::cell::RefCell::new(String::new())))).is_err());
    }
}
