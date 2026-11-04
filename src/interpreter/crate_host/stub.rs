//! `--simulate` stubs for crate functions (`fixtures/rust_stubs.json`), read as plain JSON by the surface type:
//! a string for `string`, a number for integers and floats, `true`/`false`, a one-character string for `char`,
//! an array for `Vec` and tuples, `null` for `None` (anything else is the `Some` value), and `{"Ok": x}` /
//! `{"Err": e}` for `Result`. A function with `&mut` parameters is stubbed by its result only.

use super::super::int::IntTy;
use super::super::value::{FloatTy, Value};
use super::sig::WireTy;
use super::wire::show;
use serde_json::Value as Json;

pub fn value(j: &Json, t: &WireTy) -> Result<Value, String> {
    let bad = || format!("expected {}, found `{j}`", show(t));
    Ok(match (t, j) {
        (WireTy::Unit, Json::Null) => Value::Unit,
        (WireTy::Int(n), Json::Number(x)) => {
            let v = x.as_i64().map(i128::from).or_else(|| x.as_u64().map(i128::from)).ok_or_else(bad)?;
            Value::Int(v, IntTy::from_name(n).unwrap_or(IntTy::I64))
        }
        (WireTy::Float(n), Json::Number(x)) => Value::Float(x.as_f64().ok_or_else(bad)?, if *n == "f32" { FloatTy::F32 } else { FloatTy::F64 }),
        (WireTy::Bool, Json::Bool(b)) => Value::Bool(*b),
        (WireTy::Char, Json::String(s)) if s.chars().count() == 1 => Value::Char(s.chars().next().unwrap_or(' ')),
        (WireTy::Str, Json::String(s)) => Value::str(s.clone()),
        (WireTy::Vec(e), Json::Array(xs)) => Value::vec(xs.iter().map(|x| value(x, e)).collect::<Result<_, _>>()?),
        (WireTy::Tuple(ts), Json::Array(xs)) if ts.len() == xs.len() => {
            Value::tuple(xs.iter().zip(ts).map(|(x, t)| value(x, t)).collect::<Result<_, _>>()?)
        }
        (WireTy::Option(_), Json::Null) => Value::none(),
        (WireTy::Option(e), x) => Value::some(value(x, e)?),
        (WireTy::Result(a, b), Json::Object(m)) if m.len() == 1 => match (m.get("Ok"), m.get("Err")) {
            (Some(x), _) => Value::ok(value(x, a)?),
            (_, Some(e)) => Value::err(value(e, b)?),
            _ => return Err(bad()),
        },
        _ => return Err(bad()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stubs_read_by_the_declared_type() {
        let t = WireTy::Option(Box::new(WireTy::Vec(Box::new(WireTy::Str))));
        let v = value(&json!(["a", "b"]), &t).unwrap();
        assert_eq!(v.as_option().unwrap().unwrap().items().len(), 2);
        assert!(matches!(value(&json!(null), &t).unwrap().as_option(), Some(None)));
        let r = WireTy::Result(Box::new(WireTy::Int("u32")), Box::new(WireTy::Str));
        assert!(matches!(value(&json!({"Ok": 4}), &r).unwrap(), Value::Enum(e) if e.idx == 0));
        assert!(matches!(value(&json!({"Err": "no"}), &r).unwrap(), Value::Enum(e) if e.idx == 1));
        assert!(value(&json!("x"), &WireTy::Int("i64")).unwrap_err().contains("expected i64"));
    }
}
