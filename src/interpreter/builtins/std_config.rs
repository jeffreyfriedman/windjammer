//! `std::config`: the same code path as `windjammer_runtime::config` (toml / serde_yaml into
//! `serde_json::Value`, flattened to dotted keys), so parse results and error texts agree.
//!
//! The legacy `windjammer.toml` hooks (`get_backend`, `has_backend`, `get`, `get_or`) have no
//! implementation in any backend ("compiler will generate platform-specific code" never landed):
//! they are loud errors here rather than a guess at what they should return.

use super::*;
use std::collections::BTreeMap;

fn normalize_format(format: &str) -> Result<&'static str, String> {
    match format.trim().to_ascii_lowercase().as_str() {
        "toml" => Ok("toml"),
        "yaml" | "yml" => Ok("yaml"),
        other => Err(format!("unknown config format: {other}")),
    }
}

fn parse_value(text: &str, format: &str) -> Result<serde_json::Value, String> {
    let format = normalize_format(format)?;
    if text.trim().is_empty() {
        return Err(format!("empty {format}"));
    }
    if format == "toml" {
        let v: toml::Value = toml::from_str(text).map_err(|e| format!("toml parse: {e}"))?;
        serde_json::to_value(v).map_err(|e| format!("json encode: {e}"))
    } else {
        serde_yaml::from_str(text).map_err(|e| format!("yaml parse: {e}"))
    }
}

fn flatten(prefix: &str, v: &serde_json::Value, out: &mut BTreeMap<String, String>) -> Result<(), String> {
    use serde_json::Value as J;
    let leaf = match v {
        J::Object(m) => {
            for (k, child) in m {
                let next = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                flatten(&next, child, out)?;
            }
            return Ok(());
        }
        J::Null if prefix.is_empty() => return Ok(()),
        J::Null => String::new(),
        J::Bool(b) => b.to_string(),
        J::Number(n) => n.to_string(),
        J::String(s) => s.clone(),
        J::Array(_) => serde_json::to_string(v).map_err(|e| format!("json encode: {e}"))?,
    };
    out.insert(prefix.to_string(), leaf);
    Ok(())
}

fn parse_flat(text: &str, format: &str) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    flatten("", &parse_value(text, format)?, &mut out)?;
    Ok(out)
}

type M = BTreeMap<Key, Value>;

fn map_arg(a: &[Value], i: usize, what: &str) -> R<M> {
    match crate::interpreter::place::deref_value(arg(a, i, what)?)? {
        Value::Map(m) => Ok(m.to_btree()),
        other => Err(semantic(format!("`config::{what}` expects a HashMap<string, string>, got {}", other.type_name()))),
    }
}

fn merge(mut base: M, overlay: M) -> M {
    base.extend(overlay);
    base
}

fn overlay_matching(mut map: M, env: M) -> M {
    for (k, v) in env {
        if map.contains_key(&k) {
            map.insert(k, v);
        }
    }
    map
}

fn str_map(m: BTreeMap<String, String>) -> M {
    m.into_iter().map(|(k, v)| (Key(Value::str(k)), Value::str(v))).collect()
}

fn result(r: Result<Value, String>) -> Value {
    match r {
        Ok(v) => Value::ok(v),
        Err(e) => Value::err(Value::str(e)),
    }
}

fn map(m: M) -> Value {
    Value::btree_map(m)
}

pub fn register(r: &mut Registry) {
    r.func("config::to_json", |_, a| {
        let (t, f) = (arg_str(&a, 0, "to_json")?, arg_str(&a, 1, "to_json")?);
        let json = parse_value(&t, &f).and_then(|v| serde_json::to_string(&v).map_err(|e| format!("json encode: {e}")));
        Ok(result(json.map(Value::str)))
    });
    r.func("config::parse_flat", |_, a| {
        let (t, f) = (arg_str(&a, 0, "parse_flat")?, arg_str(&a, 1, "parse_flat")?);
        Ok(result(parse_flat(&t, &f).map(|m| map(str_map(m)))))
    });
    r.func("config::merge", |_, a| Ok(map(merge(map_arg(&a, 0, "merge")?, map_arg(&a, 1, "merge")?))));
    r.func("config::overlay_matching", |_, a| {
        Ok(map(overlay_matching(map_arg(&a, 0, "overlay_matching")?, map_arg(&a, 1, "overlay_matching")?)))
    });
    r.func("config::resolve", |_, a| {
        let merged = merge(map_arg(&a, 0, "resolve")?, map_arg(&a, 1, "resolve")?);
        Ok(map(overlay_matching(merged, map_arg(&a, 2, "resolve")?)))
    });
    r.func("config::resolve_text", |_, a| {
        let defaults = map_arg(&a, 0, "resolve_text")?;
        let (t, f) = (arg_str(&a, 1, "resolve_text")?, arg_str(&a, 2, "resolve_text")?);
        let env = map_arg(&a, 3, "resolve_text")?;
        Ok(result(parse_flat(&t, &f).map(|file| map(overlay_matching(merge(defaults, str_map(file)), env)))))
    });
    macro_rules! legacy {
        ($($name:literal),*) => {$(
            r.func($name, |_, _| {
                Err(unsupported(concat!(
                    "`", $name, "` (legacy windjammer.toml hook) has no implementation in any backend; ",
                    "load settings with `config::resolve_text` / `config::parse_flat`"
                )))
            });
        )*};
    }
    legacy!("config::get_backend", "config::has_backend", "config::get", "config::get_or");
}
