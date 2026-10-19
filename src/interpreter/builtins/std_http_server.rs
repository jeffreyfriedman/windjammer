//! `std::http` servers (`http::serve`, `http::serve_fn`, `Server::serve`, `http::server_serve`) with
//! the semantics of `windjammer_runtime::http` (axum), behind the client's capability port:
//!
//! * default: refused with an `Err` naming the flags (no socket is opened);
//! * `--mock-http FILE`: every `{"request": {"method","path","headers","body"}}` entry is dispatched
//!   to the handler in file order, one transcript line per request goes to stderr
//!   (`[mock-http] GET /x -> 200 <body>`), then `serve` returns `Ok(())`;
//! * `--allow-network`: a real HTTP/1.1 listener (one request at a time, `connection: close`) that
//!   runs until the process is stopped, like the compiled server.
//!
//! Routing is axum's: `/users/:id` / `/users/{id}` match one segment, `/*rest` / `{*rest}` the tail;
//! an unrouted path is 404 with an empty body, a routed path with another method 405. Router
//! methods the runtime lacks (`any`, `nest`, `Request::path_param`) are not provided: calling them
//! is the interpreter's loud "no method" error, as the compiled program fails to build.

use super::std_http::HttpMode;
use super::*;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};

/// A parsed request, independent of where it came from. Header names are lowercase (as axum's).
#[derive(Clone)]
pub struct RawRequest {
    pub method: String,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl RawRequest {
    /// A `--mock-http` `"request"` object; `path` may carry a `?query`.
    pub fn from_json(j: &serde_json::Value) -> Result<RawRequest, String> {
        let get = |k: &str| j.get(k).and_then(|v| v.as_str()).map(str::to_string);
        let target = get("path").ok_or("mock request needs a path")?;
        let (path, query) = parse_target(&target);
        let headers = j
            .get("headers")
            .and_then(|h| h.as_object())
            .map(|o| o.iter().map(|(k, v)| (k.to_ascii_lowercase(), v.as_str().unwrap_or("").to_string())).collect())
            .unwrap_or_default();
        let method = get("method").unwrap_or_else(|| "GET".into()).to_ascii_uppercase();
        Ok(RawRequest { method, path, query, headers, body: get("body").unwrap_or_default().into_bytes() })
    }
}

const METHODS: [&str; 7] = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

fn s(v: &str) -> Value {
    Value::str(v.to_string())
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
    match v {
        Value::Struct(sv) => sv.fields.iter().find(|(n, _)| &**n == name).map(|(_, x)| x.clone()),
        _ => None,
    }
}

fn set_field(v: &Value, name: &str, x: Value) -> Value {
    match v {
        Value::Struct(sv) => {
            let mut sv = (**sv).clone();
            if let Some(f) = sv.fields.iter_mut().find(|(n, _)| &**n == name) {
                f.1 = x;
            }
            Value::Struct(Rc::new(sv))
        }
        other => other.clone(),
    }
}

/// `HttpMethod::<m>`; extension methods fall back to GET (as the runtime's `From<Method>`).
fn method_value(m: &str) -> Value {
    let idx = METHODS.iter().position(|x| x.eq_ignore_ascii_case(m)).unwrap_or(0);
    Value::enum_val("HttpMethod", METHODS[idx], idx as u32, Payload::Unit)
}

fn str_map(pairs: &[(String, String)]) -> Value {
    let mut m = BTreeMap::new();
    for (k, v) in pairs {
        m.insert(Key(s(k)), s(v));
    }
    Value::btree_map(m)
}

fn pair_vec(pairs: &[(String, String)]) -> Value {
    Value::vec(pairs.iter().map(|(k, v)| Value::tuple(vec![s(k), s(v)])).collect())
}

fn request_value(r: &RawRequest) -> Value {
    let body = Value::vec(r.body.iter().map(|b| Value::Int(*b as i128, IntTy::U8)).collect());
    strukt(
        "Request",
        vec![
            ("method", method_value(&r.method)),
            ("path", s(&r.path)),
            ("query", str_map(&r.query)),
            ("headers", str_map(&r.headers)),
            ("body", body),
        ],
    )
}

fn server_request_value(r: &RawRequest) -> Value {
    strukt(
        "ServerRequest",
        vec![
            ("method", method_value(&r.method)),
            ("path", s(&r.path)),
            ("query", str_map(&r.query)),
            ("headers", pair_vec(&r.headers)),
            ("body", s(&String::from_utf8_lossy(&r.body))),
        ],
    )
}

fn response(status: i128, body: &str, headers: &[(&str, &str)]) -> Value {
    let hs: Vec<(String, String)> = headers.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    strukt(
        "ServerResponse",
        vec![
            ("status", Value::Int(status as u16 as i128, IntTy::U16)),
            ("headers", str_map(&hs)),
            ("body", s(body)),
            ("binary_body", Value::none()),
        ],
    )
}

fn str_of(v: &Value) -> String {
    match v {
        Value::Str(x) => (**x).clone(),
        _ => String::new(),
    }
}

/// Status, headers and body of a handler's `ServerResponse`.
fn response_parts(v: &Value) -> R<(u16, Vec<(String, String)>, Vec<u8>)> {
    let status = match field(v, "status") {
        Some(Value::Int(n, _)) => n as u16,
        _ => return Err(semantic("an http handler must return a ServerResponse")),
    };
    let headers = match field(v, "headers") {
        Some(Value::Map(m)) => m.iter().map(|(k, x)| (str_of(&k.0), str_of(x))).collect(),
        _ => Vec::new(),
    };
    let body = match field(v, "binary_body").as_ref().and_then(|b| b.as_option().flatten().cloned()) {
        Some(Value::Vec(bytes)) => bytes.iter().map(|b| if let Value::Int(n, _) = b { *n as u8 } else { 0 }).collect(),
        _ => field(v, "body").map(|b| str_of(&b).into_bytes()).unwrap_or_default(),
    };
    Ok((status, headers, body))
}

/// axum-style match of `pattern` against `path`.
fn route_matches(pattern: &str, path: &str) -> bool {
    let pat: Vec<&str> = pattern.split('/').filter(|x| !x.is_empty()).collect();
    let segs: Vec<&str> = path.split('/').filter(|x| !x.is_empty()).collect();
    for (i, p) in pat.iter().enumerate() {
        if p.starts_with('*') || p.starts_with("{*") {
            return segs.len() > i; // axum's wildcard needs a non-empty tail
        }
        let Some(seg) = segs.get(i) else { return false };
        let capture = p.starts_with(':') || (p.starts_with('{') && p.ends_with('}'));
        if !capture && p != seg {
            return false;
        }
    }
    pat.len() == segs.len()
}

fn routes(router: &Value) -> Vec<(String, String, Value)> {
    let Some(Value::Vec(rs)) = field(router, "routes") else { return Vec::new() };
    rs.iter()
        .filter_map(|r| match r {
            Value::Tuple(t) => Some((str_of(&t[0]), str_of(&t[1]), t[2].clone())),
            _ => None,
        })
        .collect()
}

enum Handler {
    /// `serve(addr, router)`: handlers take a `Request`.
    Router(Value),
    /// `serve_fn` / `Server::serve` / `server_serve`: one handler for every path. `serve_fn` is a
    /// GET route on `/*path` in the runtime; the simple server is an axum fallback (any method).
    Catch { f: Value, get_only: bool, simple: bool },
}

fn dispatch(it: &mut Interpreter, h: &Handler, req: &RawRequest) -> R<(u16, Vec<(String, String)>, Vec<u8>)> {
    let resp = match h {
        Handler::Catch { f, get_only, simple } => {
            if *get_only {
                // `/*path` needs a non-empty tail; axum's GET routes also answer HEAD
                if req.path.trim_matches('/').is_empty() {
                    return Ok((404, vec![], vec![]));
                }
                if req.method != "GET" && req.method != "HEAD" {
                    return Ok((405, vec![], vec![]));
                }
            }
            let arg = if *simple { server_request_value(req) } else { request_value(req) };
            it.call_value(f, vec![arg])?
        }
        Handler::Router(r) => {
            let table = routes(r);
            let routed: Vec<_> = table.iter().filter(|(_, p, _)| route_matches(p, &req.path)).collect();
            let hit = routed.iter().find(|(m, _, _)| *m == req.method || (m == "GET" && req.method == "HEAD"));
            match hit {
                Some((_, _, f)) => it.call_value(f, vec![request_value(req)])?,
                None if !routed.is_empty() => return Ok((405, vec![], vec![])),
                None => return Ok((404, vec![], vec![])),
            }
        }
    };
    response_parts(&crate::interpreter::place::deref_value(resp)?)
}

fn parse_target(target: &str) -> (String, Vec<(String, String)>) {
    match target.split_once('?') {
        Some((p, q)) => (p.to_string(), url::form_urlencoded::parse(q.as_bytes()).map(|(k, v)| (k.to_string(), v.to_string())).collect()),
        None => (target.to_string(), Vec::new()),
    }
}

fn read_request(stream: &std::net::TcpStream) -> Option<RawRequest> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let (method, target) = (parts.next()?.to_ascii_uppercase(), parts.next()?.to_string());
    let mut headers = Vec::new();
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h).ok()? == 0 || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
    }
    let len = headers.iter().find(|(k, _)| k == "content-length").and_then(|(_, v)| v.parse::<usize>().ok()).unwrap_or(0);
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).ok()?;
    let (path, query) = parse_target(&target);
    Some(RawRequest { method, path, query, headers, body })
}

fn write_response(stream: &mut std::net::TcpStream, status: u16, headers: &[(String, String)], body: &[u8]) {
    let mut out = format!("HTTP/1.1 {status} {}\r\n", reason(status));
    for (k, v) in headers {
        out.push_str(&format!("{k}: {v}\r\n"));
    }
    out.push_str(&format!("content-length: {}\r\nconnection: close\r\n\r\n", body.len()));
    let _ = stream.write_all(out.as_bytes()).and_then(|_| stream.write_all(body));
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "",
    }
}

fn serve(it: &mut Interpreter, addr: &str, h: Handler) -> R {
    if addr.parse::<std::net::SocketAddr>().is_err() {
        return Ok(Value::err(s("invalid socket address syntax"))); // std's AddrParseError text
    }
    match it.http.clone() {
        HttpMode::Deny => Ok(Value::err(s(&format!(
            "network access is disabled in the interpreter: serve {addr} (use --mock-http FILE to replay requests or --allow-network to listen)"
        )))),
        HttpMode::Mock(m) => {
            for req in &m.requests {
                let (status, _, body) = dispatch(it, &h, req)?;
                it.write_err(&format!("[mock-http] {} {} -> {} {}\n", req.method, req.path, status, String::from_utf8_lossy(&body)));
            }
            Ok(Value::ok(Value::Unit))
        }
        HttpMode::Real => {
            let listener = match std::net::TcpListener::bind(addr) {
                Ok(l) => l,
                Err(e) => return Ok(Value::err(s(&e.to_string()))),
            };
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let Some(req) = read_request(&stream) else { continue };
                let (status, headers, body) = dispatch(it, &h, &req)?;
                write_response(&mut stream, status, &headers, &body);
            }
            Ok(Value::ok(Value::Unit))
        }
    }
}

fn with_route(router: &Value, method: &str, a: &[Value]) -> R {
    let mut rs = match field(router, "routes") {
        Some(Value::Vec(v)) => (*v).clone(),
        _ => Vec::new(),
    };
    rs.push(Value::tuple(vec![s(method), s(&arg_str(a, 0, "route")?), arg(a, 1, "route")?]));
    Ok(strukt("Router", vec![("routes", Value::vec(rs))]))
}

fn map_lookup(v: &Value, f: &str, k: &str) -> Value {
    match field(v, f) {
        Some(Value::Map(m)) => Value::option(m.get(&Key(s(k))).cloned()),
        _ => Value::none(),
    }
}

fn simple_addr(host: &str, port: i128) -> String {
    format!("{host}:{port}")
}

pub fn register(r: &mut Registry) {
    for t in ["Router", "ServerResponse", "Server", "Request", "ServerRequest", "HttpMethod"] {
        r.type_name(t);
    }
    for (i, m) in METHODS.iter().enumerate() {
        r.add_const(format!("HttpMethod::{m}"), Value::enum_val("HttpMethod", m, i as u32, Payload::Unit));
    }
    r.func("http::serve", |it, a| {
        let addr = arg_str(&a, 0, "serve")?;
        serve(it, &addr, Handler::Router(arg(&a, 1, "serve")?))
    });
    r.func("http::serve_fn", |it, a| {
        let addr = arg_str(&a, 0, "serve_fn")?;
        serve(it, &addr, Handler::Catch { f: arg(&a, 1, "serve_fn")?, get_only: true, simple: false })
    });
    r.func("http::server_serve", |it, a| {
        let addr = simple_addr(&arg_str(&a, 0, "server_serve")?, arg_int(&a, 1, "server_serve")?);
        serve(it, &addr, Handler::Catch { f: arg(&a, 2, "server_serve")?, get_only: false, simple: true })
    });
    r.func("Server::new", |_, a| {
        Ok(strukt("Server", vec![("address", s(&arg_str(&a, 0, "new")?)), ("port", Value::Int(arg_int(&a, 1, "new")?, IntTy::I64))]))
    });
    r.native("Server", "serve", |it, v, a| {
        let addr = match (field(v, "address"), field(v, "port")) {
            (Some(h), Some(Value::Int(p, _))) => simple_addr(&str_of(&h), p),
            _ => return Err(semantic("malformed Server value")),
        };
        serve(it, &addr, Handler::Catch { f: arg(&a, 0, "serve")?, get_only: false, simple: true })
    });
    r.func("Router::new", |_, _| Ok(strukt("Router", vec![("routes", Value::vec(vec![]))])));
    r.native("Router", "get", |_, v, a| with_route(v, "GET", &a));
    r.native("Router", "post", |_, v, a| with_route(v, "POST", &a));
    r.native("Router", "put", |_, v, a| with_route(v, "PUT", &a));
    r.native("Router", "delete", |_, v, a| with_route(v, "DELETE", &a));
    r.native("Router", "patch", |_, v, a| with_route(v, "PATCH", &a));
    r.native("HttpMethod", "as_str", |_, v, _| match v {
        Value::Enum(e) => Ok(s(&e.variant)),
        _ => Err(semantic("expected an HttpMethod")),
    });
    r.func("HttpMethod::from_str", |_, a| {
        let x = arg_str(&a, 0, "from_str")?;
        Ok(Value::option(METHODS.iter().any(|m| m.eq_ignore_ascii_case(&x)).then(|| method_value(&x))))
    });
    r.native("Request", "method", |_, v, _| Ok(field(v, "method").unwrap_or(Value::Unit)));
    r.native("Request", "path", |_, v, _| Ok(field(v, "path").unwrap_or(Value::Unit)));
    for n in ["query", "query_param"] {
        r.native("Request", n, |_, v, a| Ok(map_lookup(v, "query", &arg_str(&a, 0, "query")?)));
        r.native("ServerRequest", n, |_, v, a| Ok(map_lookup(v, "query", &arg_str(&a, 0, "query")?)));
    }
    // the runtime's `Request::header` is an exact lookup in axum's (lowercase) header map
    r.native("Request", "header", |_, v, a| Ok(map_lookup(v, "headers", &arg_str(&a, 0, "header")?)));
    r.native("Request", "body_string", |_, v, _| {
        let bytes: Vec<u8> = match field(v, "body") {
            Some(Value::Vec(b)) => b.iter().map(|x| if let Value::Int(n, _) = x { *n as u8 } else { 0 }).collect(),
            _ => Vec::new(),
        };
        Ok(match String::from_utf8(bytes) {
            Ok(t) => Value::ok(s(&t)),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });
    r.native("ServerRequest", "header", |_, v, a| {
        let k = arg_str(&a, 0, "header")?;
        let Some(Value::Vec(hs)) = field(v, "headers") else { return Ok(Value::none()) };
        Ok(Value::option(hs.iter().find_map(|h| match h {
            Value::Tuple(t) if str_of(&t[0]).eq_ignore_ascii_case(&k) => Some(t[1].clone()),
            _ => None,
        })))
    });
    register_responses(r);
}

/// `ServerResponse` constructors and builders, field for field as the runtime's.
fn register_responses(r: &mut Registry) {
    const JSON: &[(&str, &str)] = &[("Content-Type", "application/json")];
    r.func("ServerResponse::new", |_, a| Ok(response(arg_int(&a, 0, "new")?, &arg_str(&a, 1, "new")?, &[])));
    r.func("ServerResponse::with_status", |_, a| Ok(response(arg_int(&a, 0, "with_status")?, &arg_str(&a, 1, "with_status")?, &[])));
    r.func("ServerResponse::error", |_, a| {
        Ok(response(arg_int(&a, 0, "error")?, &arg_str(&a, 1, "error")?, &[("Content-Type", "text/plain")]))
    });
    r.func("ServerResponse::ok", |_, a| Ok(response(200, &arg_str(&a, 0, "ok")?, &[])));
    r.func("ServerResponse::json", |_, a| Ok(response(200, &arg_str(&a, 0, "json")?, JSON)));
    r.func("ServerResponse::html", |_, a| Ok(response(200, &arg_str(&a, 0, "html")?, &[("Content-Type", "text/html")])));
    r.func("ServerResponse::created", |_, a| Ok(response(201, &arg_str(&a, 0, "created")?, &[])));
    r.func("ServerResponse::bad_request", |_, a| Ok(response(400, &arg_str(&a, 0, "bad_request")?, &[])));
    r.func("ServerResponse::unauthorized", |_, a| Ok(response(401, &arg_str(&a, 0, "unauthorized")?, &[])));
    r.func("ServerResponse::forbidden", |_, a| Ok(response(403, &arg_str(&a, 0, "forbidden")?, &[])));
    r.func("ServerResponse::payload_too_large", |_, a| Ok(response(413, &arg_str(&a, 0, "payload_too_large")?, &[])));
    r.func("ServerResponse::too_many_requests", |_, a| Ok(response(429, &arg_str(&a, 0, "too_many_requests")?, &[])));
    r.func("ServerResponse::service_unavailable", |_, a| Ok(response(503, &arg_str(&a, 0, "service_unavailable")?, &[])));
    r.func("ServerResponse::internal_error", |_, a| Ok(response(500, &arg_str(&a, 0, "internal_error")?, &[])));
    r.func("ServerResponse::no_content", |_, _| Ok(response(204, "", &[])));
    r.func("ServerResponse::not_found", |_, _| Ok(response(404, "Not Found", &[])));
    // runtime: `json_value<T: Serialize>(&T) -> Result<ServerResponse, String>`
    r.func("ServerResponse::json_value", |it, a| {
        let text = it.call_builtin_fn("json::to_string", vec![arg(&a, 0, "json_value")?])?;
        let text = match text {
            Value::Enum(ref e) if &*e.ty == "Result" => match (&*e.variant, &e.payload) {
                ("Ok", Payload::Tuple(p)) => str_of(&p[0]),
                _ => return Ok(text.clone()),
            },
            other => str_of(&other),
        };
        Ok(Value::ok(response(200, &text, JSON)))
    });
    r.func("ServerResponse::binary", |it, a| {
        let data = it.to_items(arg(&a, 1, "binary")?)?;
        let v = response(arg_int(&a, 0, "binary")?, "", &[]);
        Ok(set_field(&v, "binary_body", Value::some(Value::vec(data))))
    });
    for n in ["with_header", "header"] {
        r.native("ServerResponse", n, |_, v, a| {
            let mut hs = match field(v, "headers") {
                Some(Value::Map(m)) => m.m.clone(),
                _ => BTreeMap::new(),
            };
            hs.insert(Key(s(&arg_str(&a, 0, "header")?)), s(&arg_str(&a, 1, "header")?));
            Ok(set_field(v, "headers", Value::btree_map(hs)))
        });
    }
}
