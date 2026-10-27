//! `wj mcp bind`: typed Windjammer bindings for the tools of an MCP server (ADR-0021 section 12, stage E5).
//!
//! Input is the `tools/list` result (`{"tools": [{"name", "description", "inputSchema", "outputSchema"?}]}`, or the bare array).
//! Output is one module: per tool an arguments struct (nested objects become structs), a result type, JSON encoder and decoder
//! functions over the `mcp::*` string builtins, and a function `tool_name(args) -> Result<R, string>` that calls
//! `mcp::call("server", "tool", json)`. The tool's effect is derived from those two literals by the effect pass, so the
//! generated code needs no annotations and a hand-written `mcp::call` is covered by the same rule.
//!
//! The generator is pure text in, text out and deterministic (tools sorted by name), so a changed tool list is a readable diff.

use serde_json::Value as Json;
use std::collections::BTreeMap;
use std::fmt::Write;

const KEYWORDS: &[&str] = &[
    "fn", "let", "mut", "struct", "enum", "impl", "trait", "use", "pub", "match", "if", "else", "for", "while", "loop", "return", "type", "self", "in",
    "as", "mod", "const", "static", "break", "continue", "true", "false", "async", "await", "move", "where", "spawn", "scope", "defer", "durable", "step",
];

/// A Windjammer type for a schema, with what the encoder and decoder need to know.
#[derive(Clone, Debug, PartialEq)]
enum Ty {
    Str,
    Int,
    Float,
    Bool,
    /// Free-form JSON, carried as its text.
    Raw,
    List(Box<Ty>),
    Named(String),
}

impl Ty {
    fn show(&self) -> String {
        match self {
            Ty::Str | Ty::Raw => "string".into(),
            Ty::Int => "int".into(),
            Ty::Float => "float".into(),
            Ty::Bool => "bool".into(),
            Ty::List(t) => format!("Vec<{}>", t.show()),
            Ty::Named(n) => n.clone(),
        }
    }
}

struct Field {
    json_name: String,
    name: String,
    ty: Ty,
    required: bool,
    doc: String,
}

struct Struct {
    name: String,
    doc: String,
    fields: Vec<Field>,
    /// A result: its list fields are plain `Vec`s (a missing list is empty), and it is only ever decoded.
    output: bool,
}

fn snake(s: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() && prev_lower {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
            prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        } else {
            if !out.ends_with('_') && !out.is_empty() {
                out.push('_');
            }
            prev_lower = false;
        }
    }
    let out = out.trim_matches('_').to_string();
    let out = if out.is_empty() { "tool".to_string() } else { out };
    let out = if out.chars().next().is_some_and(|c| c.is_ascii_digit()) { format!("t_{out}") } else { out };
    if KEYWORDS.contains(&out.as_str()) {
        format!("{out}_")
    } else {
        out
    }
}

fn pascal(s: &str) -> String {
    snake(s).trim_end_matches('_').split('_').filter(|p| !p.is_empty()).map(|p| {
        let mut c = p.chars();
        c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
    }).collect()
}

struct Ctx<'a> {
    defs: &'a Json,
    structs: BTreeMap<String, Struct>,
    output: bool,
}

impl Ctx<'_> {
    /// The type of `schema`; object schemas with properties become structs named `hint`.
    fn ty(&mut self, schema: &Json, hint: &str) -> Ty {
        if let Some(r) = schema.get("$ref").and_then(|r| r.as_str()) {
            let key = r.rsplit('/').next().unwrap_or(r);
            return match self.defs.get(key) {
                Some(d) => {
                    let d = d.clone();
                    self.ty(&d, &pascal(key))
                }
                None => Ty::Raw,
            };
        }
        // `anyOf` / `oneOf` with a null branch is an optional value: the non-null branch decides the type
        for k in ["anyOf", "oneOf"] {
            if let Some(arr) = schema.get(k).and_then(|a| a.as_array()) {
                let non_null: Vec<&Json> = arr.iter().filter(|b| b.get("type").and_then(|t| t.as_str()) != Some("null")).collect();
                return match non_null.as_slice() {
                    [one] => {
                        let one = (*one).clone();
                        self.ty(&one, hint)
                    }
                    _ => Ty::Raw,
                };
            }
        }
        let t = match schema.get("type") {
            Some(Json::String(t)) => t.clone(),
            Some(Json::Array(ts)) => ts.iter().filter_map(|t| t.as_str()).find(|t| *t != "null").unwrap_or("string").to_string(),
            _ => {
                if schema.get("properties").is_some() { "object".into() } else { "string".into() }
            }
        };
        match t.as_str() {
            "string" => Ty::Str,
            "integer" => Ty::Int,
            "number" => Ty::Float,
            "boolean" => Ty::Bool,
            "array" => {
                let items = schema.get("items").cloned().unwrap_or(Json::Null);
                Ty::List(Box::new(self.ty(&items, &format!("{hint}Item"))))
            }
            "object" => {
                let Some(props) = schema.get("properties").and_then(|p| p.as_object()) else { return Ty::Raw };
                if props.is_empty() {
                    return Ty::Raw;
                }
                let required: Vec<&str> = schema.get("required").and_then(|r| r.as_array()).map(|r| r.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default();
                let mut fields = Vec::new();
                let mut names: Vec<&String> = props.keys().collect();
                names.sort();
                for k in names {
                    let p = &props[k];
                    let name = snake(k);
                    let ty = self.ty(p, &format!("{hint}{}", pascal(k)));
                    let mut doc = p.get("description").and_then(|d| d.as_str()).unwrap_or("").replace('\n', " ");
                    if let Some(vals) = p.get("enum").and_then(|e| e.as_array()) {
                        let vals: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
                        doc = format!("{doc} One of: {}.", vals.join(", ")).trim().to_string();
                    }
                    fields.push(Field { json_name: k.clone(), name, ty, required: required.contains(&k.as_str()), doc });
                }
                let doc = schema.get("description").and_then(|d| d.as_str()).unwrap_or("").replace('\n', " ");
                self.structs.entry(hint.to_string()).or_insert(Struct { name: hint.to_string(), doc, fields, output: self.output });
                Ty::Named(hint.to_string())
            }
            _ => Ty::Raw,
        }
    }
}

/// A Windjammer string literal's content for the JSON object key `k`, with the colon: `\"key\":`.
fn key_literal(k: &str) -> String {
    let json = serde_json::to_string(k).unwrap_or_default();
    format!("{}:", json.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Statements (indented by `ind`) and an expression that produce the JSON text of `expr` of type `ty`.
fn encode(ty: &Ty, expr: &str, tag: &str, ind: &str) -> (String, String) {
    match ty {
        Ty::Str => (String::new(), format!("mcp::quote({expr})")),
        Ty::Raw => (String::new(), expr.to_string()),
        Ty::Int | Ty::Float | Ty::Bool => (String::new(), format!("{expr}.to_string()")),
        Ty::Named(n) => (String::new(), format!("{}_json({expr})", snake(n))),
        Ty::List(inner) => {
            let (xs, v) = (format!("xs_{tag}"), format!("x_{tag}"));
            let (inner_stmts, inner_expr) = encode(inner, &v, &format!("{tag}i"), &format!("{ind}    "));
            let stmts = format!("{ind}let mut {xs}: Vec<string> = Vec::new()\n{ind}for {v} in {expr} {{\n{inner_stmts}{ind}    {xs}.push({inner_expr})\n{ind}}}\n");
            let joined = format!("j_{tag}");
            let arr = format!("arr_{tag}");
            (format!("{stmts}{ind}let {joined} = {xs}.join(\",\")\n{ind}let {arr} = \"[${{{joined}}}]\"\n"), arr)
        }
    }
}

/// Statements and an expression decoding the value at `key` of the JSON text `j` into `ty`.
fn decode(ty: &Ty, j: &str, key: &str, required: bool, tag: &str, ind: &str) -> (String, String) {
    let k = format!("{key:?}");
    let get = |f: &str| format!("mcp::{f}({j}, {k})");
    let plain = |e: String| (String::new(), e);
    match (ty, required) {
        (Ty::Str, true) => plain(format!("{} ?? \"\"", get("get_string"))),
        (Ty::Str, false) => plain(get("get_string")),
        (Ty::Int, true) => plain(format!("{} ?? 0", get("get_int"))),
        (Ty::Int, false) => plain(get("get_int")),
        (Ty::Float, true) => plain(format!("{} ?? 0.0", get("get_float"))),
        (Ty::Float, false) => plain(get("get_float")),
        (Ty::Bool, true) => plain(format!("{} ?? false", get("get_bool"))),
        (Ty::Bool, false) => plain(get("get_bool")),
        (Ty::Raw, true) => plain(format!("{} ?? \"\"", get("get_raw"))),
        (Ty::Raw, false) => plain(get("get_raw")),
        (Ty::List(inner), _) => match **inner {
            Ty::Str => plain(get("get_strings")),
            Ty::Int => plain(get("get_ints")),
            Ty::Named(ref n) => {
                let v = format!("v_{tag}");
                let stmts = format!("{ind}let mut {v}: Vec<{n}> = Vec::new()\n{ind}for r in {} {{\n{ind}    {v}.push({}_from_json(r))\n{ind}}}\n", get("get_items"), snake(n));
                (stmts, v)
            }
            _ => plain(get("get_items")),
        },
        (Ty::Named(n), true) => plain(format!("{}_from_json(mcp::get_raw({j}, {k}) ?? \"{{}}\")", snake(n))),
        (Ty::Named(n), false) => {
            let v = format!("v_{tag}");
            let stmts = format!("{ind}let {v}: Option<{n}> = match mcp::get_raw({j}, {k}) {{\n{ind}    Some(r) => Some({}_from_json(r)),\n{ind}    None => None,\n{ind}}}\n", snake(n));
            (stmts, v)
        }
    }
}

/// `Some(expr)` for an optional list field of an argument struct (never decoded in practice; kept total).
fn wrap_some(stmts: String, e: String, i: usize) -> (String, String) {
    let v = format!("some_{i}");
    (format!("{stmts}    let {v} = Some({e})\n"), v)
}

fn doc_lines(out: &mut String, indent: &str, text: &str) {
    let text = text.trim();
    if !text.is_empty() {
        let _ = writeln!(out, "{indent}/// {}", text.replace('\n', " "));
    }
}

fn emit_struct(out: &mut String, s: &Struct) {
    doc_lines(out, "", &s.doc);
    let _ = writeln!(out, "pub struct {} {{", s.name);
    for f in &s.fields {
        doc_lines(out, "    ", &f.doc);
        let t = f.ty.show();
        let plain = f.required || (s.output && matches!(f.ty, Ty::List(_)));
        let _ = writeln!(out, "    pub {}: {},", f.name, if plain { t } else { format!("Option<{t}>") });
    }
    let _ = writeln!(out, "}}\n");
    // encoder
    let _ = writeln!(out, "fn {}_json(v: {}) -> string {{\n    let mut parts: Vec<string> = Vec::new()", snake(&s.name), s.name);
    for (i, f) in s.fields.iter().enumerate() {
        let key = key_literal(&f.json_name);
        if f.required {
            let (stmts, e) = encode(&f.ty, &format!("v.{}", f.name), &format!("f{i}"), "    ");
            let _ = write!(out, "{stmts}    parts.push(\"{key}${{{e}}}\")\n");
        } else {
            let (stmts, e) = encode(&f.ty, "x", &format!("f{i}"), "            ");
            let _ = write!(out, "    match v.{} {{\n        Some(x) => {{\n{stmts}            parts.push(\"{key}${{{e}}}\")\n        }}\n        None => {{}}\n    }}\n", f.name);
        }
    }
    let _ = writeln!(out, "    let body = parts.join(\",\")\n    \"{{${{body}}}}\"\n}}\n");
    // decoder
    let _ = writeln!(out, "fn {}_from_json(j: string) -> {} {{", snake(&s.name), s.name);
    let mut exprs = Vec::new();
    for (i, f) in s.fields.iter().enumerate() {
        let (stmts, e) = decode(&f.ty, "j", &f.json_name, f.required, &format!("f{i}"), "    ");
        // an optional list in a result is a plain `Vec` (empty when absent): `decode` already yields a `Vec` for lists
        let (stmts, e) = if matches!(f.ty, Ty::List(_)) && !s.output && !f.required { wrap_some(stmts, e, i) } else { (stmts, e) };
        out.push_str(&stmts);
        exprs.push((f.name.clone(), e));
    }
    let _ = writeln!(out, "    {} {{", s.name);
    for (n, e) in exprs {
        let _ = writeln!(out, "        {n}: {e},");
    }
    let _ = writeln!(out, "    }}\n}}\n");
}

/// The module text for all of `server`'s tools.
pub fn generate(server: &str, tools_json: &str) -> Result<String, String> {
    generate_only(server, tools_json, &[])
}

/// The module text for the tools named in `only` (all of them when it is empty). Binding just the tools a script needs keeps the
/// program's effect set, and so its permission request, as small as the script.
pub fn generate_only(server: &str, tools_json: &str, only: &[String]) -> Result<String, String> {
    let doc: Json = serde_json::from_str(tools_json).map_err(|e| format!("the tool list is not JSON: {e}"))?;
    let tools: Vec<Json> = match &doc {
        Json::Array(a) => a.clone(),
        Json::Object(o) => o.get("tools").and_then(|t| t.as_array()).cloned().or_else(|| doc.get("result").and_then(|r| r.get("tools")).and_then(|t| t.as_array()).cloned()).ok_or("expected {\"tools\": [...]}")?,
        _ => return Err("expected {\"tools\": [...]} or an array of tools".into()),
    };
    if !server.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') || server.is_empty() {
        return Err(format!("`{server}` is not a usable server name (letters, digits, `_` and `-`)"));
    }
    let mut tools = tools;
    tools.sort_by_key(|t| t["name"].as_str().unwrap_or("").to_string());
    if !only.is_empty() {
        let have: Vec<String> = tools.iter().filter_map(|t| t["name"].as_str().map(String::from)).collect();
        if let Some(missing) = only.iter().find(|o| !have.contains(o)) {
            return Err(format!("the server has no tool named `{missing}` (it has: {})", have.join(", ")));
        }
        tools.retain(|t| t["name"].as_str().is_some_and(|n| only.iter().any(|o| o == n)));
    }
    let mut out = String::new();
    let _ = writeln!(out, "// Generated by `wj mcp bind {server}`: typed bindings for the tools of the MCP server `{server}`.\n// Do not edit; regenerate when the tool list changes. Each function performs the effect `mcp.{server}.<tool>`.\n");
    let mut fn_names = std::collections::BTreeSet::new();
    let mut body = String::new();
    for t in &tools {
        let name = t["name"].as_str().ok_or("a tool has no name")?;
        let fname = snake(name);
        if !fn_names.insert(fname.clone()) {
            return Err(format!("two tools map to the function name `{fname}`; rename one"));
        }
        let defs = t.get("inputSchema").and_then(|s| s.get("$defs").or_else(|| s.get("definitions"))).cloned().unwrap_or(Json::Null);
        let mut cx = Ctx { defs: &defs, structs: BTreeMap::new(), output: false };
        let args_name = format!("{}Args", pascal(name));
        let input = t.get("inputSchema").cloned().unwrap_or(Json::Null);
        let args_ty = cx.ty(&input, &args_name);
        let args_ty = match args_ty {
            Ty::Named(n) => Some(n),
            _ => None,
        };
        let result_name = format!("{}Result", pascal(name));
        let out_schema = t.get("outputSchema").cloned();
        let defs_out = out_schema.as_ref().and_then(|s| s.get("$defs").or_else(|| s.get("definitions"))).cloned().unwrap_or(Json::Null);
        let mut cx_out = Ctx { defs: &defs_out, structs: BTreeMap::new(), output: true };
        let result_ty = out_schema.as_ref().map(|s| cx_out.ty(s, &result_name));
        for s in cx.structs.values().chain(cx_out.structs.values()) {
            emit_struct(&mut body, s);
        }
        doc_lines(&mut body, "", t["description"].as_str().unwrap_or(""));
        let (param, arg_json) = match &args_ty {
            Some(n) => (format!("args: {n}"), format!("{}_json(args)", snake(n))),
            None => (String::new(), "\"{}\"".to_string()),
        };
        let ret = match &result_ty {
            Some(Ty::Named(n)) => n.clone(),
            _ => "string".to_string(),
        };
        let _ = writeln!(body, "pub fn {fname}({param}) -> Result<{ret}, string> {{");
        match &result_ty {
            Some(Ty::Named(n)) => {
                let _ = writeln!(body, "    match mcp::call({server:?}, {name:?}, {arg_json}) {{\n        Ok(text) => Ok({}_from_json(text)),\n        Err(e) => Err(e),\n    }}", snake(n));
            }
            _ => {
                let _ = writeln!(body, "    mcp::call({server:?}, {name:?}, {arg_json})");
            }
        }
        let _ = writeln!(body, "}}\n");
    }
    out.push_str("use std::mcp\n\n");
    out.push_str(&body);
    Ok(out.trim_end().to_string() + "\n")
}
