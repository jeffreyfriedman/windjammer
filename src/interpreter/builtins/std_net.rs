//! `std::net` raw sockets (ADR-0032): `TcpStream`, `TcpListener`, `UdpSocket`.
//!
//! A socket value is a handle id (`Sock`). Every operation is `net_op(key, [id, args..])`, which goes
//! - live (no port): to the OS through `net_core`, the same source the compiled runtime wraps;
//! - `--record`: live, and logged as one event; `--replay`: answered from the log (no socket is opened);
//! - `--simulate`: to the virtual loopback network of the seeded world (`effect_port/sim_net.rs`).
//!
//! The capability gate checks `connect`/`bind` on their sink rows before the builtin runs; `UdpSocket.send_to`
//! asks it per datagram. Results are plain data (`Ok(id)`, bytes, strings), so the log can hold every one.

use super::*;
use crate::interpreter::effect_port::{sim_net, Kind};
use crate::interpreter::native::{downcast, Native};
use crate::interpreter::net_core;
use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

enum Live {
    Stream(net_core::Stream),
    Listener(net_core::Listener),
    Udp(net_core::Datagrams),
}

/// Live sockets by handle id. Process-wide: interpreter tasks run on their own OS threads (one at a time).
fn live_lock() -> &'static Mutex<HashMap<i128, Arc<Mutex<Live>>>> {
    static LIVE: OnceLock<Mutex<HashMap<i128, Arc<Mutex<Live>>>>> = OnceLock::new();
    LIVE.get_or_init(Default::default)
}

fn live_table() -> MutexGuard<'static, HashMap<i128, Arc<Mutex<Live>>>> {
    live_lock().lock().unwrap_or_else(|p| p.into_inner())
}

/// Run `f` on live socket `id`, holding only that socket (a blocking read never holds the table).
fn on_live<T>(id: i128, f: impl FnOnce(&mut Live) -> T) -> Option<T> {
    let s = live_table().get(&id).cloned()?;
    let mut g = s.lock().unwrap_or_else(|p| p.into_inner());
    Some(f(&mut g))
}

/// A fresh handle id (live and simulated handles share the sequence; replayed ones come from the log).
pub(crate) fn next_id() -> i128 {
    static NEXT: AtomicI64 = AtomicI64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed) as i128
}

/// A socket handle. Dropping the last copy closes it (the peer reads end of stream).
pub(crate) struct Sock {
    id: i128,
    kind: &'static str,
}

impl Native for Sock {
    fn type_name(&self) -> &'static str {
        self.kind
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Drop for Sock {
    fn drop(&mut self) {
        // removed outside the lock: closing a socket never runs while the table is held
        let gone = live_lock().try_lock().ok().and_then(|mut t| t.remove(&self.id));
        drop(gone);
        sim_net::close(self.id);
    }
}

fn sock_id(v: &Value, kind: &str) -> R<i128> {
    match downcast::<Sock>(v) {
        Some(s) if s.kind == kind => Ok(s.id),
        _ => Err(semantic(format!("expected a {kind}"))),
    }
}

pub(crate) fn bytes_value(b: &[u8]) -> Value {
    Value::vec(b.iter().map(|x| Value::Int(*x as i128, IntTy::U8)).collect())
}

pub(crate) fn bytes_arg(it: &mut Interpreter, v: Option<&Value>, what: &str) -> R<Vec<u8>> {
    let v = v.cloned().ok_or_else(|| semantic(format!("`{what}` expects bytes")))?;
    let v = crate::interpreter::place::deref_value(crate::interpreter::place::strip_refs(v)?)?;
    if let Value::Str(s) = &v {
        return Ok(s.as_bytes().to_vec());
    }
    let mut out = Vec::new();
    for x in it.to_items(v)? {
        match x {
            Value::Int(n, _) if (0..=255).contains(&n) => out.push(n as u8),
            other => return Err(semantic(format!("`{what}` expects bytes (Vec<u8>), found {}", other.type_name()))),
        }
    }
    Ok(out)
}

fn result<T>(r: Result<T, String>, ok: impl FnOnce(T) -> Value) -> Value {
    match r {
        Ok(t) => Value::ok(ok(t)),
        Err(e) => Value::err(Value::str(e)),
    }
}

fn int(n: i128) -> Value {
    Value::Int(n, IntTy::I64)
}

/// The effect a logged operation belongs to (the log's `effect` field).
fn effect_of(key: &str) -> &'static str {
    if key.starts_with("TcpListener") || key == "UdpSocket::bind" || key == "UdpSocket.recv_from" {
        "net.listen"
    } else {
        "net.connect"
    }
}

/// One socket operation: live, recorded, replayed or simulated (see the module comment).
fn net_op(it: &mut Interpreter, key: &'static str, args: Vec<Value>, live: fn(&mut Interpreter, &[Value]) -> R) -> R {
    match it.port_kind() {
        None => live(it, &args),
        Some(Kind::Simulate) => sim_net::answer(it, key, &args),
        Some(_) => it.port_call_with(effect_of(key), key, &args, &mut |i, a| live(i, a)),
    }
}

/// `Ok(id)` from an operation that opens a handle becomes `Ok(handle)`.
fn handle(v: Value, kind: &'static str) -> R {
    if let Value::Enum(e) = &v {
        if &*e.ty == "Result" && e.idx == 0 {
            if let crate::interpreter::value::Payload::Tuple(p) = &e.payload {
                if let Some(Value::Int(id, _)) = p.first() {
                    return Ok(Value::ok(Value::Native(Rc::new(Sock { id: *id, kind }))));
                }
            }
            return Err(semantic("malformed socket handle"));
        }
    }
    Ok(v)
}

fn with_live<T>(id: i128, f: impl FnOnce(&mut Live) -> Result<T, String>) -> Result<T, String> {
    on_live(id, f).unwrap_or_else(|| Err("not connected".to_string()))
}

fn live_stream<T>(a: &[Value], f: impl FnOnce(&mut net_core::Stream) -> Result<T, String>) -> Result<T, String> {
    let id = match a.first() {
        Some(Value::Int(n, _)) => *n,
        _ => return Err("not connected".into()),
    };
    with_live(id, |s| match s {
        Live::Stream(s) => f(s),
        _ => Err("not connected".into()),
    })
}

fn opened(r: Result<Live, String>) -> Value {
    result(r, |s| {
        let id = next_id();
        live_table().insert(id, Arc::new(Mutex::new(s)));
        int(id)
    })
}

fn n_arg(a: &[Value], i: usize) -> i64 {
    match a.get(i) {
        Some(Value::Int(n, _)) => *n as i64,
        _ => 0,
    }
}

fn s_arg(a: &[Value], i: usize) -> String {
    match a.get(i) {
        Some(Value::Str(s)) => (**s).clone(),
        _ => String::new(),
    }
}

fn b_arg(a: &[Value], i: usize) -> Vec<u8> {
    match a.get(i) {
        Some(v @ Value::Vec(_)) => v.items().iter().map(|x| if let Value::Int(n, _) = x { *n as u8 } else { 0 }).collect(),
        _ => Vec::new(),
    }
}

fn live_connect(_: &mut Interpreter, a: &[Value]) -> R {
    let addr = s_arg(a, 0);
    let timeout = a.get(1).map(|_| net_core::timeout_of(n_arg(a, 1))).unwrap_or(None);
    Ok(opened(net_core::resolve_once(&addr).and_then(|xs| net_core::connect_resolved(&xs, timeout)).map(Live::Stream)))
}

fn live_listen(_: &mut Interpreter, a: &[Value]) -> R {
    Ok(opened(net_core::Listener::bind(&s_arg(a, 0)).map(Live::Listener)))
}

fn live_udp_bind(_: &mut Interpreter, a: &[Value]) -> R {
    Ok(opened(net_core::Datagrams::bind(&s_arg(a, 0)).map(Live::Udp)))
}

fn live_accept(_: &mut Interpreter, a: &[Value]) -> R {
    let r = with_live(match a.first() {
        Some(Value::Int(n, _)) => *n,
        _ => 0,
    }, |s| match s {
        Live::Listener(l) => l.accept(),
        _ => Err("not connected".into()),
    });
    Ok(opened(r.map(Live::Stream)))
}

fn live_udp<T>(a: &[Value], f: impl FnOnce(&net_core::Datagrams) -> Result<T, String>) -> Result<T, String> {
    let id = match a.first() {
        Some(Value::Int(n, _)) => *n,
        _ => 0,
    };
    with_live(id, |s| match s {
        Live::Udp(u) => f(u),
        _ => Err("not connected".into()),
    })
}

fn live_listener_addr(a: &[Value]) -> String {
    let id = match a.first() {
        Some(Value::Int(n, _)) => *n,
        _ => 0,
    };
    with_live(id, |s| {
        Ok(match s {
            Live::Listener(l) => l.local_addr(),
            Live::Udp(u) => u.local_addr(),
            Live::Stream(s) => s.local_addr(),
        })
    })
    .unwrap_or_default()
}

/// A stream method: (registered name, logged key, live body).
type StreamOp = (&'static str, &'static str, fn(&mut Interpreter, &[Value]) -> R);

const STREAM_OPS: &[StreamOp] = &[
    ("read", "TcpStream.read", |_, a| Ok(result(live_stream(a, |s| s.read(n_arg(a, 1))), |b| bytes_value(&b)))),
    ("read_exact", "TcpStream.read_exact", |_, a| Ok(result(live_stream(a, |s| s.read_exact(n_arg(a, 1))), |b| bytes_value(&b)))),
    ("read_line", "TcpStream.read_line", |_, a| Ok(result(live_stream(a, |s| s.read_line()), Value::str))),
    ("write_all", "TcpStream.write_all", |_, a| Ok(result(live_stream(a, |s| s.write_all(&b_arg(a, 1))), |_| Value::Unit))),
    ("write_str", "TcpStream.write_all", |_, a| Ok(result(live_stream(a, |s| s.write_all(&b_arg(a, 1))), |_| Value::Unit))),
    ("flush", "TcpStream.flush", |_, a| Ok(result(live_stream(a, |s| s.flush()), |_| Value::Unit))),
    ("shutdown_write", "TcpStream.shutdown_write", |_, a| Ok(result(live_stream(a, |s| s.shutdown_write()), |_| Value::Unit))),
    ("shutdown", "TcpStream.shutdown", |_, a| Ok(result(live_stream(a, |s| s.shutdown()), |_| Value::Unit))),
    ("set_timeouts", "TcpStream.set_timeouts", |_, a| Ok(result(live_stream(a, |s| s.set_timeouts(n_arg(a, 1), n_arg(a, 2))), |_| Value::Unit))),
    ("peer_addr", "TcpStream.peer_addr", |_, a| Ok(Value::str(live_stream(a, |s| Ok(s.peer_addr())).unwrap_or_default()))),
    ("local_addr", "TcpStream.local_addr", |_, a| Ok(Value::str(live_stream(a, |s| Ok(s.local_addr())).unwrap_or_default()))),
];

/// The arguments of a logged operation: the handle id, then the call's own arguments (bytes as `Vec<u8>`).
fn op_args(it: &mut Interpreter, id: i128, name: &str, a: &[Value]) -> R<Vec<Value>> {
    let mut out = vec![int(id)];
    match name {
        "write_all" | "write_str" | "send_to" => {
            out.push(bytes_value(&bytes_arg(it, a.first(), name)?));
            if name == "send_to" {
                out.push(Value::str(arg_str(a, 1, name)?.to_string()));
            }
        }
        "read" | "read_exact" | "recv_from" | "set_read_timeout" => out.push(int(arg_int(a, 0, name)?)),
        "set_timeouts" => {
            out.push(int(arg_int(a, 0, name)?));
            out.push(int(arg_int(a, 1, name)?));
        }
        _ => {}
    }
    Ok(out)
}

pub fn register(r: &mut Registry) {
    for t in ["TcpStream", "TcpListener", "UdpSocket"] {
        r.type_name(t);
    }
    // constructors: sink rows (the gate has run), then one logged operation that returns the handle id
    r.func("TcpStream::connect", |it, a| {
        let addr = arg_str(&a, 0, "TcpStream::connect")?.to_string();
        let v = net_op(it, "TcpStream::connect", vec![Value::str(addr)], live_connect)?;
        handle(v, "TcpStream")
    });
    r.func("TcpStream::connect_timeout", |it, a| {
        let addr = arg_str(&a, 0, "TcpStream::connect_timeout")?.to_string();
        let ms = arg_int(&a, 1, "TcpStream::connect_timeout")?;
        let v = net_op(it, "TcpStream::connect_timeout", vec![Value::str(addr), int(ms)], live_connect)?;
        handle(v, "TcpStream")
    });
    r.func("TcpListener::bind", |it, a| {
        let addr = arg_str(&a, 0, "TcpListener::bind")?.to_string();
        let v = net_op(it, "TcpListener::bind", vec![Value::str(addr)], live_listen)?;
        handle(v, "TcpListener")
    });
    r.func("UdpSocket::bind", |it, a| {
        let addr = arg_str(&a, 0, "UdpSocket::bind")?.to_string();
        let v = net_op(it, "UdpSocket::bind", vec![Value::str(addr)], live_udp_bind)?;
        handle(v, "UdpSocket")
    });
    macro_rules! stream_method {
        ($($name:literal),*) => { $( r.native("TcpStream", $name, |it, v, a| stream_call(it, v, $name, &a)); )* };
    }
    stream_method!("read", "read_exact", "read_line", "write_all", "write_str", "flush", "shutdown_write", "shutdown", "set_timeouts", "peer_addr", "local_addr");
    r.native("TcpListener", "accept", |it, v, _| {
        let id = sock_id(v, "TcpListener")?;
        cooperate(it, id, |l| match l {
            Live::Listener(l) => l.ready(),
            _ => true,
        })?;
        let out = net_op(it, "TcpListener.accept", vec![int(id)], live_accept)?;
        handle(out, "TcpStream")
    });
    r.native("TcpListener", "local_addr", |it, v, _| {
        let id = sock_id(v, "TcpListener")?;
        net_op(it, "TcpListener.local_addr", vec![int(id)], |_, a| Ok(Value::str(live_listener_addr(a))))
    });
    r.native("UdpSocket", "send_to", |it, v, a| {
        let id = sock_id(v, "UdpSocket")?;
        let to = arg_str(&a, 1, "send_to")?;
        // every datagram is a connect to its destination
        it.cap_decide(&crate::ir::effect_facts::EffectKind::NetConnect, Some(to.as_str()))?;
        let args = op_args(it, id, "send_to", &a)?;
        net_op(it, "UdpSocket.send_to", args, |_, a| Ok(result(live_udp(a, |u| u.send_to(&b_arg(a, 1), &s_arg(a, 2))), |n| int(n as i128))))
    });
    r.native("UdpSocket", "recv_from", |it, v, a| {
        let id = sock_id(v, "UdpSocket")?;
        cooperate(it, id, |l| match l {
            Live::Udp(u) => u.ready(),
            _ => true,
        })?;
        let args = op_args(it, id, "recv_from", &a)?;
        net_op(it, "UdpSocket.recv_from", args, |_, a| {
            Ok(result(live_udp(a, |u| u.recv_from(n_arg(a, 1))), |(b, from)| Value::tuple(vec![bytes_value(&b), Value::str(from)])))
        })
    });
    r.native("UdpSocket", "set_read_timeout", |it, v, a| {
        let id = sock_id(v, "UdpSocket")?;
        let args = op_args(it, id, "set_read_timeout", &a)?;
        net_op(it, "UdpSocket.set_read_timeout", args, |_, a| Ok(result(live_udp(a, |u| u.set_read_timeout(n_arg(a, 1))), |_| Value::Unit)))
    });
    r.native("UdpSocket", "local_addr", |it, v, _| {
        let id = sock_id(v, "UdpSocket")?;
        net_op(it, "UdpSocket.local_addr", vec![int(id)], |_, a| Ok(Value::str(live_listener_addr(a))))
    });
}

fn stream_call(it: &mut Interpreter, v: &Value, name: &str, a: &[Value]) -> R {
    let id = sock_id(v, "TcpStream")?;
    let (_, key, live) = STREAM_OPS.iter().find(|(n, _, _)| *n == name).copied().ok_or_else(|| semantic(format!("no TcpStream.{name}")))?;
    let args = op_args(it, id, name, a)?;
    if matches!(name, "read" | "read_exact" | "read_line") {
        let want = n_arg(&args, 1).max(1) as usize;
        cooperate(it, id, |l| match l {
            Live::Stream(s) => match name {
                "read_exact" => s.buffered().len() >= want,
                "read_line" => s.buffered().contains(&b'\n'),
                _ => !s.buffered().is_empty(),
            },
            _ => true,
        })?;
    }
    net_op(it, key, args, live)
}

fn other_tasks(it: &Interpreter) -> bool {
    it.sched.tasks.iter().filter(|t| !t.done).count() > 1
}

fn live_ready(id: i128) -> bool {
    on_live(id, |l| match l {
        Live::Stream(s) => s.ready(),
        Live::Listener(l) => l.ready(),
        Live::Udp(u) => u.ready(),
    })
    .unwrap_or(true)
}

/// A live run with other tasks: wait for the socket on the scheduler (other tasks run meanwhile) instead of blocking the
/// thread, until `done` holds, the socket reaches end of stream, or its read timeout passes (then the operation itself reports
/// it). Not under the effect port: a recorded run must switch tasks exactly as its replay will, so there the call blocks.
fn cooperate(it: &mut Interpreter, id: i128, done: impl Fn(&Live) -> bool) -> R<()> {
    if it.port.is_some() || !other_tasks(it) {
        return Ok(());
    }
    let timeout = on_live(id, |l| match l {
        Live::Stream(s) => s.read_timeout(),
        Live::Udp(u) => u.read_timeout(),
        Live::Listener(_) => None,
    })
    .flatten();
    let start = std::time::Instant::now();
    loop {
        let finished = on_live(id, |l| done(l)).unwrap_or(true);
        if finished || timeout.is_some_and(|d| start.elapsed() >= d) {
            return Ok(());
        }
        if live_ready(id) {
            // one read into the buffer: data (loop again) or end of stream (the operation sees it)
            let got = on_live(id, |l| match l {
                Live::Stream(s) => s.pump().unwrap_or(0),
                _ => 0,
            })
            .unwrap_or(0);
            if got == 0 {
                return Ok(());
            }
            continue;
        }
        let tick = it.sched.clock.saturating_add(1_000_000);
        it.block_on(crate::interpreter::sched::Wait::Poll(Rc::new(move || live_ready(id)), Some(tick)))?;
    }
}
