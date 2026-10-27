//! The host side of `mcp::call`: the MCP servers a script may reach, bound by name (`--bind NAME=ENDPOINT`).
//!
//! * `mock:FILE`   canned results, for tests, CI and offline work: `{"tool": "text" | {"error": "text"} | [replies in order]}`
//! * `stdio:CMD ARGS...`   a child process speaking MCP over its standard input and output (JSON-RPC 2.0, one message per line)
//!
//! The child inherits the host's environment, so credentials stay with the host and the script never sees them.
//! A tool result is the text of its content blocks; a tool error (`isError`) is the `Err` side of `Result<string, string>`.

use serde_json::{json, Value as Json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

enum Reply {
    Text(String),
    Error(String),
}

struct Mock {
    tools: HashMap<String, Vec<Reply>>,
    used: HashMap<String, usize>,
}

struct Server {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
    next_id: u64,
}

enum Endpoint {
    Mock(Mock),
    Server(Box<Server>),
}

#[derive(Default)]
pub struct McpHost {
    binds: HashMap<String, Endpoint>,
}

fn reply_of(v: &Json) -> Reply {
    match v {
        Json::String(s) => Reply::Text(s.clone()),
        Json::Object(o) if o.contains_key("error") => Reply::Error(o["error"].as_str().unwrap_or("error").to_string()),
        other => Reply::Text(other.to_string()),
    }
}

fn load_mock(path: &str) -> Result<Mock, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read the mock file {path}: {e}"))?;
    let doc: Json = serde_json::from_str(&text).map_err(|e| format!("the mock file {path} is not JSON: {e}"))?;
    let obj = doc.as_object().ok_or_else(|| format!("the mock file {path} must be an object of tool -> result"))?;
    let tools = obj
        .iter()
        .map(|(k, v)| (k.clone(), match v {
            Json::Array(a) => a.iter().map(reply_of).collect(),
            other => vec![reply_of(other)],
        }))
        .collect();
    Ok(Mock { tools, used: HashMap::new() })
}

impl Server {
    /// Start `cmd` (program and arguments separated by spaces) and complete the MCP handshake.
    fn start(cmd: &str) -> Result<Server, String> {
        let mut parts = cmd.split_whitespace();
        let program = parts.next().ok_or("`stdio:` needs a command")?;
        let mut child = Command::new(program)
            .args(parts)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("cannot start the MCP server `{program}`: {e}"))?;
        let stdin = child.stdin.take().ok_or("no stdin")?;
        let out = BufReader::new(child.stdout.take().ok_or("no stdout")?);
        let mut server = Server { child, stdin, out, next_id: 0 };
        let init = server.request(
            "initialize",
            json!({"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "wj", "version": env!("CARGO_PKG_VERSION")}}),
        )?;
        if init.get("error").is_some() {
            return Err(format!("the MCP server refused to initialize: {}", init["error"]));
        }
        server.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))?;
        Ok(server)
    }

    fn send(&mut self, msg: &Json) -> Result<(), String> {
        writeln!(self.stdin, "{msg}").and_then(|_| self.stdin.flush()).map_err(|e| format!("the MCP server stopped answering: {e}"))
    }

    /// Send a request and read lines until its response arrives (notifications and log lines are skipped).
    fn request(&mut self, method: &str, params: Json) -> Result<Json, String> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        loop {
            let mut line = String::new();
            let n = self.out.read_line(&mut line).map_err(|e| format!("the MCP server stopped answering: {e}"))?;
            if n == 0 {
                return Err("the MCP server closed its output".into());
            }
            let Ok(v) = serde_json::from_str::<Json>(line.trim()) else { continue };
            if v.get("id") == Some(&json!(id)) {
                return Ok(v);
            }
        }
    }
}

impl McpHost {
    /// `NAME=ENDPOINT` from the command line.
    pub fn bind(&mut self, name: &str, spec: &str) -> Result<(), String> {
        if let Some(path) = spec.strip_prefix("mock:") {
            self.binds.insert(name.to_string(), Endpoint::Mock(load_mock(path)?));
            return Ok(());
        }
        if let Some(cmd) = spec.strip_prefix("stdio:") {
            self.binds.insert(name.to_string(), Endpoint::Server(Box::new(Server::start(cmd)?)));
            return Ok(());
        }
        Err(format!("unknown endpoint `{spec}`: use mock:FILE or stdio:COMMAND ARGS"))
    }

    pub fn is_bound(&self, name: &str) -> bool {
        self.binds.contains_key(name)
    }

    /// `Ok(text)` is a result, `Err(text)` a tool error or a host problem; both reach the script as values.
    pub fn call(&mut self, server: &str, tool: &str, args_json: &str) -> Result<String, String> {
        let Some(ep) = self.binds.get_mut(server) else {
            return Err(format!("no MCP server is bound as `{server}` (pass --bind {server}=mock:FILE or --bind {server}=stdio:COMMAND)"));
        };
        match ep {
            Endpoint::Mock(m) => {
                let replies = m.tools.get(tool).ok_or_else(|| format!("the mock server `{server}` has no tool `{tool}`"))?;
                let n = m.used.entry(tool.to_string()).or_insert(0);
                let r = &replies[(*n).min(replies.len() - 1)];
                *n += 1;
                match r {
                    Reply::Text(t) => Ok(t.clone()),
                    Reply::Error(e) => Err(e.clone()),
                }
            }
            Endpoint::Server(s) => {
                let args: Json = serde_json::from_str(args_json).map_err(|e| format!("the arguments are not JSON: {e}"))?;
                let resp = s.request("tools/call", json!({"name": tool, "arguments": args}))?;
                if let Some(e) = resp.get("error") {
                    return Err(e["message"].as_str().unwrap_or("tool call failed").to_string());
                }
                let result = &resp["result"];
                let text = result["content"]
                    .as_array()
                    .map(|c| c.iter().filter_map(|b| b["text"].as_str()).collect::<Vec<_>>().join("\n"))
                    .unwrap_or_default();
                if result["isError"].as_bool() == Some(true) {
                    Err(text)
                } else {
                    Ok(text)
                }
            }
        }
    }
}

impl McpHost {
    pub fn is_mock(&self, name: &str) -> bool {
        matches!(self.binds.get(name), Some(Endpoint::Mock(_)))
    }
}

impl crate::interpreter::engine::Interpreter {
    /// `--bind NAME=ENDPOINT`.
    pub fn bind_mcp(&mut self, name: &str, spec: &str) -> Result<(), String> {
        self.mcp.get_or_insert_with(Default::default).bind(name, spec)
    }

    pub(crate) fn mcp_is_mock(&self, name: &str) -> bool {
        self.mcp.as_ref().is_some_and(|h| h.is_mock(name))
    }
}

impl Drop for McpHost {
    fn drop(&mut self) {
        for ep in self.binds.values_mut() {
            if let Endpoint::Server(s) = ep {
                let _ = s.child.kill();
                let _ = s.child.wait();
            }
        }
    }
}

/// Ask a live MCP server (started with `cmd`) for its tools: the text of the `tools/list` result.
pub fn list_tools(cmd: &str) -> Result<String, String> {
    let mut server = Server::start(cmd)?;
    let resp = server.request("tools/list", json!({}))?;
    let _ = server.child.kill();
    let _ = server.child.wait();
    resp.get("result").map(|r| r.to_string()).ok_or_else(|| format!("the MCP server did not list its tools: {}", resp["error"]))
}
