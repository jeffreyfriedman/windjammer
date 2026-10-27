//! `std::mcp`: the bridge to MCP tools (ADR-0021 section 12). `mcp::call(server, tool, json)` is the one sink; the rest are pure
//! string helpers the generated bindings use to build arguments and read results.

use super::*;
use serde_json::Value as Json;

fn parse(j: &str) -> Option<Json> {
    serde_json::from_str(j).ok()
}

fn field(j: &str, key: &str) -> Option<Json> {
    parse(j)?.get(key).cloned().filter(|v| !v.is_null())
}

fn items(j: &str, key: &str) -> Vec<Json> {
    field(j, key).and_then(|v| v.as_array().cloned()).unwrap_or_default()
}

pub fn register(r: &mut Registry) {
    r.func("mcp::call", |it, a| {
        let (server, tool, args) = (arg_str(&a, 0, "call")?, arg_str(&a, 1, "call")?, arg_str(&a, 2, "call")?);
        let host = it.mcp.get_or_insert_with(Default::default);
        Ok(match host.call(&server, &tool, &args) {
            Ok(text) => Value::ok(Value::str(text)),
            Err(e) => Value::err(Value::str(e)),
        })
    });
    r.func("mcp::quote", |_, a| Ok(Value::str(serde_json::to_string(arg_str(&a, 0, "quote")?.as_str()).unwrap_or_default())));
    r.func("mcp::get_string", |_, a| {
        let v = field(&arg_str(&a, 0, "get_string")?, &arg_str(&a, 1, "get_string")?);
        Ok(Value::option(v.and_then(|v| v.as_str().map(|s| Value::str(s.to_string())))))
    });
    r.func("mcp::get_int", |_, a| {
        let v = field(&arg_str(&a, 0, "get_int")?, &arg_str(&a, 1, "get_int")?);
        Ok(Value::option(v.and_then(|v| v.as_i64()).map(|n| Value::Int(n as i128, IntTy::I64))))
    });
    r.func("mcp::get_float", |_, a| {
        let v = field(&arg_str(&a, 0, "get_float")?, &arg_str(&a, 1, "get_float")?);
        Ok(Value::option(v.and_then(|v| v.as_f64()).map(Value::float)))
    });
    r.func("mcp::get_bool", |_, a| {
        let v = field(&arg_str(&a, 0, "get_bool")?, &arg_str(&a, 1, "get_bool")?);
        Ok(Value::option(v.and_then(|v| v.as_bool()).map(Value::Bool)))
    });
    r.func("mcp::get_raw", |_, a| {
        let v = field(&arg_str(&a, 0, "get_raw")?, &arg_str(&a, 1, "get_raw")?);
        Ok(Value::option(v.map(|v| Value::str(v.to_string()))))
    });
    r.func("mcp::get_items", |_, a| {
        Ok(Value::vec(items(&arg_str(&a, 0, "get_items")?, &arg_str(&a, 1, "get_items")?).into_iter().map(|v| Value::str(v.to_string())).collect()))
    });
    r.func("mcp::get_strings", |_, a| {
        Ok(Value::vec(items(&arg_str(&a, 0, "get_strings")?, &arg_str(&a, 1, "get_strings")?).into_iter().filter_map(|v| v.as_str().map(|s| Value::str(s.to_string()))).collect()))
    });
    r.func("mcp::get_ints", |_, a| {
        Ok(Value::vec(items(&arg_str(&a, 0, "get_ints")?, &arg_str(&a, 1, "get_ints")?).into_iter().filter_map(|v| v.as_i64()).map(|n| Value::Int(n as i128, IntTy::I64)).collect()))
    });
}
