//! `std::http` client behind a port.
//!
//! The interpreter never touches the network unless asked: the default port refuses with an error
//! that names the flags. `--mock-http FILE` serves canned responses (deterministic tests);
//! `--allow-network` opens plain `http://` connections (no TLS in the interpreter: `https://` is an
//! error pointing at the mock port or `wj run`). Servers: `std_http_server.rs`, same port.

use super::*;
use crate::interpreter::native::{downcast, Native};
use std::any::Any;
use std::io::{Read, Write};

/// How HTTP requests are answered.
#[derive(Clone, Default)]
pub enum HttpMode {
    /// No network (the default): every request is an error.
    #[default]
    Deny,
    /// Canned responses `{"method","url","status","headers":{..},"body"}` and, for servers,
    /// incoming requests `{"request": {"method","path","headers":{..},"body"}}`.
    Mock(Rc<Mock>),
    /// Plain `http://` over TCP.
    Real,
}

#[derive(Default)]
pub struct Mock {
    entries: Vec<MockEntry>,
    pub(crate) requests: Vec<super::std_http_server::RawRequest>,
}

#[derive(Clone)]
pub struct MockEntry {
    method: String,
    url: String,
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    /// Seconds the canned server takes to answer; a request whose timeout is shorter fails with `timed out`.
    delay_secs: Option<u64>,
}

impl HttpMode {
    pub fn mock_from_file(path: &std::path::Path) -> Result<HttpMode, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read mock file {}: {e}", path.display()))?;
        let j: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("bad mock file {}: {e}", path.display()))?;
        let arr = j.as_array().ok_or("mock file must be a JSON array")?;
        let mut out = Mock::default();
        for e in arr {
            if let Some(req) = e.get("request") {
                out.requests.push(super::std_http_server::RawRequest::from_json(req)?);
                continue;
            }
            let get = |k: &str| e.get(k).and_then(|v| v.as_str()).map(str::to_string);
            out.entries.push(MockEntry {
                method: get("method").unwrap_or_else(|| "GET".into()).to_ascii_uppercase(),
                url: get("url").ok_or("mock entry needs a url")?,
                status: e.get("status").and_then(|v| v.as_u64()).unwrap_or(200) as u16,
                headers: e
                    .get("headers")
                    .and_then(|h| h.as_object())
                    .map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
                    .unwrap_or_default(),
                body: get("body").unwrap_or_default().into_bytes(),
                delay_secs: e.get("delay_secs").and_then(|v| v.as_u64()),
            });
        }
        Ok(HttpMode::Mock(Rc::new(out)))
    }
}

struct ResponseObj {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}
struct HeadersObj(Vec<(String, String)>);
impl Native for ResponseObj {
    fn type_name(&self) -> &'static str {
        "Response"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl Native for HeadersObj {
    fn type_name(&self) -> &'static str {
        "Headers"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Per-request options of a `ClientRequest` (`std::http::request`); the verb functions use the defaults.
#[derive(Clone, Default)]
struct ReqOpts {
    headers: Vec<(String, String)>,
    timeout_secs: Option<u64>,
}

fn send(mode: &HttpMode, method: &str, url: &str, body: Option<&str>) -> Result<ResponseObj, String> {
    send_with(mode, method, url, body, &ReqOpts::default())
}

fn send_with(mode: &HttpMode, method: &str, url: &str, body: Option<&str>, opts: &ReqOpts) -> Result<ResponseObj, String> {
    match mode {
        HttpMode::Deny => Err(format!(
            "network access is disabled in the interpreter: {method} {url} (use --mock-http FILE for canned responses or --allow-network for plain http://)"
        )),
        HttpMode::Mock(m) => {
            let entry = m.entries.iter().find(|e| e.method == method && e.url == url);
            match (entry, opts.timeout_secs) {
                // a server that never answers, or answers later than the timeout, is a timeout
                (None, Some(_)) => Err("timed out".to_string()),
                (Some(e), Some(t)) if e.delay_secs.is_some_and(|d| d > t) => Err("timed out".to_string()),
                (Some(e), _) => Ok(ResponseObj { status: e.status, headers: e.headers.clone(), body: e.body.clone() }),
                (None, None) => Err(format!("mock http: no response recorded for {method} {url}")),
            }
        }
        HttpMode::Real => real_request(method, url, body, opts),
    }
}

fn io_error(e: std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => "timed out".to_string(),
        _ => e.to_string(),
    }
}

fn real_request(method: &str, url: &str, body: Option<&str>, opts: &ReqOpts) -> Result<ResponseObj, String> {
    if !url.starts_with("http://") {
        return Err(format!("only http:// is available in the interpreter (no TLS): {url}; use --mock-http or `wj run`"));
    }
    // the capability gate checked this same parse (ir::effect_facts::scope_match::split_url): connect to what was checked
    let (host, port, path) = crate::ir::effect_facts::scope_match::split_url(url).ok_or_else(|| format!("invalid url: {url}"))?;
    let port = port.unwrap_or(80);
    let hostport = if port == 80 { host.clone() } else { format!("{host}:{port}") };
    let timeout = opts.timeout_secs.map(std::time::Duration::from_secs);
    let mut stream = match timeout {
        Some(t) => {
            use std::net::ToSocketAddrs;
            let addr = (host.as_str(), port).to_socket_addrs().map_err(io_error)?.next().ok_or_else(|| format!("cannot resolve {host}"))?;
            std::net::TcpStream::connect_timeout(&addr, t).map_err(io_error)?
        }
        None => std::net::TcpStream::connect((host.as_str(), port)).map_err(|e| e.to_string())?,
    };
    stream.set_read_timeout(timeout).map_err(io_error)?;
    stream.set_write_timeout(timeout).map_err(io_error)?;
    let payload = body.unwrap_or("");
    let extra: String = opts.headers.iter().map(|(k, v)| format!("{k}: {v}\r\n")).collect();
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: {hostport}\r\nConnection: close\r\n{extra}Content-Length: {}\r\n\r\n{payload}",
        payload.len()
    );
    stream.write_all(req.as_bytes()).map_err(io_error)?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).map_err(io_error)?;
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or("malformed HTTP response")?;
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let mut lines = head.lines();
    let status: u16 = lines.next().and_then(|l| l.split_whitespace().nth(1)).and_then(|c| c.parse().ok()).ok_or("malformed status line")?;
    let headers = lines.filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect();
    Ok(ResponseObj { status, headers, body: raw[split + 4..].to_vec() })
}

/// A request under construction: `http.request(method, url)` then `.header` / `.body` / `.timeout_secs`, then `.send()`.
struct ClientRequestObj {
    method: String,
    url: String,
    body: Option<String>,
    opts: ReqOpts,
}
impl Native for ClientRequestObj {
    fn type_name(&self) -> &'static str {
        "ClientRequest"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn creq(v: &Value) -> R<&ClientRequestObj> {
    downcast::<ClientRequestObj>(v).ok_or_else(|| semantic("expected an http ClientRequest"))
}

fn derived(v: &Value, f: impl FnOnce(&mut ClientRequestObj)) -> R {
    let c = creq(v)?;
    let mut n = ClientRequestObj { method: c.method.clone(), url: c.url.clone(), body: c.body.clone(), opts: c.opts.clone() };
    f(&mut n);
    Ok(Value::Native(Rc::new(n)))
}

fn resp(v: &Value) -> R<&ResponseObj> {
    downcast::<ResponseObj>(v).ok_or_else(|| semantic("expected an http Response"))
}

pub fn register(r: &mut Registry) {
    r.type_name("Response");
    r.type_name("ClientRequest");
    // `request` is a sink (net.connect on the url, argument 1); `send` asks the gate again, as the compiled runtime does
    r.func("http::request", |_, a| {
        let method = arg_str(&a, 0, "http::request")?.to_ascii_uppercase();
        let url = arg_str(&a, 1, "http::request")?;
        Ok(Value::Native(Rc::new(ClientRequestObj { method, url: url.to_string(), body: None, opts: ReqOpts::default() })))
    });
    r.native("ClientRequest", "header", |_, v, a| {
        let k = arg_str(&a, 0, "header")?;
        let val = arg_str(&a, 1, "header")?;
        derived(v, |c| c.opts.headers.push((k.to_string(), val.to_string())))
    });
    r.native("ClientRequest", "body", |_, v, a| {
        let b = arg_str(&a, 0, "body")?;
        derived(v, |c| c.body = Some(b.to_string()))
    });
    r.native("ClientRequest", "timeout_secs", |_, v, a| {
        let n = match a.first() {
            Some(Value::Int(n, _)) => (*n).max(0) as u64,
            _ => return Err(semantic("timeout_secs expects an int")),
        };
        derived(v, |c| c.opts.timeout_secs = Some(n))
    });
    r.native("ClientRequest", "send", |it, v, _| {
        let c = creq(v)?;
        it.cap_decide(&crate::ir::effect_facts::EffectKind::NetConnect, Some(c.url.as_str()))?;
        Ok(match send_with(&it.http, &c.method, &c.url, c.body.as_deref(), &c.opts) {
            Ok(resp) => Value::ok(Value::Native(Rc::new(resp))),
            Err(e) => Value::err(Value::str(e)),
        })
    });
    for (name, method, has_body) in [
        ("get", "GET", false),
        ("post", "POST", true),
        ("put", "PUT", true),
        ("patch", "PATCH", true),
        ("delete", "DELETE", false),
        ("head", "HEAD", false),
    ] {
        let _ = (name, method, has_body);
    }
    macro_rules! verb {
        ($path:literal, $method:literal, $body:expr) => {
            r.func($path, |it, a| {
                let url = arg_str(&a, 0, $path)?;
                let body = if $body { Some(arg_str(&a, 1, $path)?) } else { None };
                Ok(match send(&it.http, $method, &url, body.as_deref().map(|s| s.as_str())) {
                    Ok(resp) => Value::ok(Value::Native(Rc::new(resp))),
                    Err(e) => Value::err(Value::str(e)),
                })
            });
        };
    }
    verb!("http::get", "GET", false);
    verb!("http::post", "POST", true);
    verb!("http::put", "PUT", true);
    verb!("http::patch", "PATCH", true);
    verb!("http::delete", "DELETE", false);
    verb!("http::head", "HEAD", false);
    r.native("Response", "text", |_, v, _| {
        Ok(match String::from_utf8(resp(v)?.body.clone()) {
            Ok(t) => Value::ok(Value::str(t)),
            Err(e) => Value::err(Value::str(e.to_string())),
        })
    });
    r.native("Response", "bytes", |_, v, _| {
        Ok(Value::ok(Value::vec(resp(v)?.body.iter().map(|b| Value::Int(*b as i128, IntTy::U8)).collect())))
    });
    for n in ["status_code", "status"] {
        r.native("Response", n, |_, v, _| Ok(Value::Int(resp(v)?.status as i128, IntTy::U16)));
    }
    r.native("Response", "is_success", |_, v, _| Ok(Value::Bool((200..300).contains(&resp(v)?.status))));
    r.native("Response", "headers", |_, v, _| Ok(Value::Native(Rc::new(HeadersObj(resp(v)?.headers.clone())))));
    r.native("Headers", "get", |_, v, a| {
        let k = arg_str(&a, 0, "get")?;
        let h = downcast::<HeadersObj>(v).ok_or_else(|| semantic("expected Headers"))?;
        Ok(Value::option(h.0.iter().find(|(n, _)| n.eq_ignore_ascii_case(&k)).map(|(_, x)| Value::str(x.clone()))))
    });
    r.native("Headers", "contains", |_, v, a| {
        let k = arg_str(&a, 0, "contains")?;
        let h = downcast::<HeadersObj>(v).ok_or_else(|| semantic("expected Headers"))?;
        Ok(Value::Bool(h.0.iter().any(|(n, _)| n.eq_ignore_ascii_case(&k))))
    });
}
