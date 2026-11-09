//! The virtual loopback network of the simulated world (ADR-0032). A `--simulate` run never opens a socket: `bind`
//! registers a virtual listener (port 0 picks from 49152 upward, in order), `connect` to a bound address makes an
//! in-memory pipe pair and queues the server end for `accept`, `connect` to anything else is `connection refused`
//! (the world has no outside network). Reads block the task on the scheduler and advance the virtual clock; a read
//! timeout is virtual time. A dropped or shut-down end is end of stream to its peer. UDP datagrams to an unbound
//! address are dropped. With `--faults`, a read or a write fails with `connection reset` from the seeded fault stream.
//!
//! The state is per thread (one interpreter runs on one thread at a time) and is reset when a world is made.

use super::super::builtins::std_net::{bytes_value, next_id};
use super::super::engine::Interpreter;
use super::super::error::{Ctl, ErrorKind, RuntimeError};
use super::super::int::IntTy;
use super::super::sched::Wait;
use super::super::value::Value;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

type R = Result<Value, Ctl>;

#[derive(Default)]
struct Pipe {
    data: VecDeque<u8>,
    /// The writer is done (shut down or dropped): the reader sees end of stream once `data` is empty.
    closed: bool,
    /// The reader is gone: a write is `broken pipe`.
    reader_gone: bool,
}

struct VStream {
    rx: Rc<RefCell<Pipe>>,
    tx: Rc<RefCell<Pipe>>,
    local: String,
    peer: String,
    read_ms: i64,
}

struct VListener {
    addr: String,
    backlog: Rc<RefCell<VecDeque<i128>>>,
}

type Inbox = Rc<RefCell<VecDeque<(Vec<u8>, String)>>>;

struct VUdp {
    addr: String,
    inbox: Inbox,
    read_ms: i64,
}

#[derive(Default)]
struct Net {
    next_port: u16,
    streams: HashMap<i128, VStream>,
    listeners: HashMap<i128, VListener>,
    udps: HashMap<i128, VUdp>,
    listen_at: HashMap<String, i128>,
    udp_at: HashMap<String, i128>,
}

thread_local! {
    static NET: RefCell<Net> = RefCell::new(Net::default());
}

/// A fresh, empty network (each simulated run starts from one).
pub fn reset() {
    let _ = NET.try_with(|n| *n.borrow_mut() = Net::default());
}

/// A handle was dropped: its peer reads end of stream, its address is free again.
pub fn close(id: i128) {
    let _ = NET.try_with(|n| {
        let Ok(mut n) = n.try_borrow_mut() else { return };
        if let Some(s) = n.streams.remove(&id) {
            s.tx.borrow_mut().closed = true;
            s.rx.borrow_mut().reader_gone = true;
        }
        if let Some(l) = n.listeners.remove(&id) {
            n.listen_at.remove(&l.addr);
        }
        if let Some(u) = n.udps.remove(&id) {
            n.udp_at.remove(&u.addr);
        }
    });
}

fn ok(v: Value) -> R {
    Ok(Value::ok(v))
}

fn err(m: impl Into<String>) -> R {
    Ok(Value::err(Value::str(m.into())))
}

fn int(n: i128) -> Value {
    Value::Int(n, IntTy::I64)
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
    a.get(i).map(|v| v.items().iter().map(|x| if let Value::Int(n, _) = x { *n as u8 } else { 0 }).collect()).unwrap_or_default()
}

/// `host:port` with `localhost` spelled as the loopback address; `None` when it is not an address.
fn normalize(addr: &str) -> Option<(String, u16)> {
    let (h, p) = addr.rsplit_once(':')?;
    let port: u16 = p.parse().ok()?;
    let host = match h.to_ascii_lowercase().as_str() {
        "localhost" => "127.0.0.1".to_string(),
        "" => return None,
        other => other.to_string(),
    };
    Some((host, port))
}

impl Net {
    fn port(&mut self, host: &str, port: u16, taken: &HashMap<String, i128>) -> Option<String> {
        if port != 0 {
            let a = format!("{host}:{port}");
            return (!taken.contains_key(&a)).then_some(a);
        }
        if self.next_port < 49152 {
            self.next_port = 49152;
        }
        loop {
            let a = format!("{host}:{}", self.next_port);
            self.next_port = self.next_port.wrapping_add(1).max(49152);
            if !taken.contains_key(&a) {
                return Some(a);
            }
        }
    }

    fn listener_for(&self, host: &str, port: u16) -> Option<i128> {
        self.listen_at.get(&format!("{host}:{port}")).or_else(|| self.listen_at.get(&format!("0.0.0.0:{port}"))).copied()
    }
}

fn id_arg(a: &[Value]) -> i128 {
    match a.first() {
        Some(Value::Int(n, _)) => *n,
        _ => 0,
    }
}

fn fault(it: &mut Interpreter) -> bool {
    match &it.port {
        Some(p) => p.borrow_mut().world().is_some_and(|w| w.fault()),
        None => false,
    }
}

/// Block the task until `ready` holds or `ms` (virtual) milliseconds pass; `false` when the time ran out.
fn wait(it: &mut Interpreter, ms: i64, ready: Rc<dyn Fn() -> bool>) -> Result<bool, Ctl> {
    if ready() {
        return Ok(true);
    }
    let deadline = (ms > 0).then(|| it.sched.clock.saturating_add(ms as u64 * 1_000_000));
    it.block_on(Wait::Poll(ready.clone(), deadline))?;
    Ok(ready())
}

fn stream_parts(id: i128) -> Option<(Rc<RefCell<Pipe>>, Rc<RefCell<Pipe>>, i64)> {
    NET.with(|n| n.borrow().streams.get(&id).map(|s| (s.rx.clone(), s.tx.clone(), s.read_ms)))
}

fn read_until(it: &mut Interpreter, id: i128, done: fn(&Pipe) -> bool) -> Result<Option<Rc<RefCell<Pipe>>>, Ctl> {
    let Some((rx, _, ms)) = stream_parts(id) else { return Ok(None) };
    let p = rx.clone();
    let ready: Rc<dyn Fn() -> bool> = Rc::new(move || {
        let p = p.borrow();
        p.closed || done(&p)
    });
    if !wait(it, ms, ready)? {
        return Err(Ctl::Err(Box::new(RuntimeError::new(ErrorKind::Semantic, "timed out"))));
    }
    Ok(Some(rx))
}

/// Run a read: a timeout becomes `Err("timed out")`, a missing handle `Err("not connected")`.
fn reading(it: &mut Interpreter, id: i128, done: fn(&Pipe) -> bool, take: impl FnOnce(&mut Pipe) -> R) -> R {
    if fault(it) {
        return err("connection reset");
    }
    match read_until(it, id, done) {
        Ok(Some(rx)) => take(&mut rx.borrow_mut()),
        Ok(None) => err("not connected"),
        Err(Ctl::Err(e)) if e.message == "timed out" => err("timed out"),
        Err(e) => Err(e),
    }
}

fn connect(it: &mut Interpreter, a: &[Value]) -> R {
    let addr = s_arg(a, 0);
    let Some((host, port)) = normalize(&addr) else { return err(format!("invalid address: {addr}")) };
    if fault(it) {
        return err("connection refused");
    }
    NET.with(|n| {
        let mut n = n.borrow_mut();
        let Some(lid) = n.listener_for(&host, port) else { return err("connection refused") };
        let (up, down) = (Rc::new(RefCell::new(Pipe::default())), Rc::new(RefCell::new(Pipe::default())));
        let taken: HashMap<String, i128> = HashMap::new();
        let local = n.port("127.0.0.1", 0, &taken).unwrap_or_default();
        let server_addr = n.listeners[&lid].addr.clone();
        let (cid, sid) = (next_id(), next_id());
        n.streams.insert(cid, VStream { rx: down.clone(), tx: up.clone(), local: local.clone(), peer: server_addr.clone(), read_ms: 0 });
        n.streams.insert(sid, VStream { rx: up, tx: down, local: server_addr, peer: local, read_ms: 0 });
        n.listeners[&lid].backlog.borrow_mut().push_back(sid);
        ok(int(cid))
    })
}

fn bind(a: &[Value], udp: bool) -> R {
    let addr = s_arg(a, 0);
    let Some((host, port)) = normalize(&addr) else { return err(format!("invalid address: {addr}")) };
    NET.with(|n| {
        let mut n = n.borrow_mut();
        let taken = if udp { n.udp_at.clone() } else { n.listen_at.clone() };
        let Some(at) = n.port(&host, port, &taken) else { return err("address in use") };
        let id = next_id();
        if udp {
            n.udp_at.insert(at.clone(), id);
            n.udps.insert(id, VUdp { addr: at, inbox: Rc::default(), read_ms: 0 });
        } else {
            n.listen_at.insert(at.clone(), id);
            n.listeners.insert(id, VListener { addr: at, backlog: Rc::default() });
        }
        ok(int(id))
    })
}

fn accept(it: &mut Interpreter, id: i128) -> R {
    let Some(backlog) = NET.with(|n| n.borrow().listeners.get(&id).map(|l| l.backlog.clone())) else { return err("not connected") };
    let b = backlog.clone();
    wait(it, 0, Rc::new(move || !b.borrow().is_empty()))?;
    let next = backlog.borrow_mut().pop_front();
    match next {
        Some(sid) => ok(int(sid)),
        None => err("timed out"),
    }
}

fn write(it: &mut Interpreter, id: i128, data: Vec<u8>) -> R {
    if fault(it) {
        return err("connection reset");
    }
    let Some((_, tx, _)) = stream_parts(id) else { return err("not connected") };
    let mut tx = tx.borrow_mut();
    if tx.reader_gone {
        return err("broken pipe");
    }
    if tx.closed {
        return err("not connected");
    }
    tx.data.extend(data);
    ok(Value::Unit)
}

fn with_stream<T>(id: i128, f: impl FnOnce(&mut VStream) -> T) -> Option<T> {
    NET.with(|n| n.borrow_mut().streams.get_mut(&id).map(f))
}

fn udp_recv(it: &mut Interpreter, id: i128, max: i64) -> R {
    if max < 1 {
        return err(format!("read size must be at least 1, got {max}"));
    }
    let Some((inbox, ms)) = NET.with(|n| n.borrow().udps.get(&id).map(|u| (u.inbox.clone(), u.read_ms))) else { return err("not connected") };
    let b = inbox.clone();
    if !wait(it, ms, Rc::new(move || !b.borrow().is_empty()))? {
        return err("timed out");
    }
    let next = inbox.borrow_mut().pop_front();
    match next {
        Some((mut d, from)) => {
            d.truncate(max as usize);
            ok(Value::tuple(vec![bytes_value(&d), Value::str(from)]))
        }
        None => err("timed out"),
    }
}

fn udp_send(it: &mut Interpreter, id: i128, data: Vec<u8>, to: &str) -> R {
    let Some((host, port)) = normalize(to) else { return err(format!("invalid address: {to}")) };
    if fault(it) {
        return err("connection reset");
    }
    NET.with(|n| {
        let n = n.borrow();
        let Some(from) = n.udps.get(&id).map(|u| u.addr.clone()) else { return err("not connected") };
        let len = data.len() as i128;
        let target = n.udp_at.get(&format!("{host}:{port}")).or_else(|| n.udp_at.get(&format!("0.0.0.0:{port}")));
        if let Some(u) = target.and_then(|t| n.udps.get(t)) {
            u.inbox.borrow_mut().push_back((data, from));
        }
        ok(int(len))
    })
}

fn size_ok(n: i64) -> Result<usize, String> {
    if n < 1 {
        Err(format!("read size must be at least 1, got {n}"))
    } else {
        Ok(n as usize)
    }
}

/// One socket operation in the simulated world (`key` and `args` as `builtins/std_net.rs` logs them).
pub fn answer(it: &mut Interpreter, key: &str, a: &[Value]) -> R {
    let id = id_arg(a);
    match key {
        "TcpStream::connect" | "TcpStream::connect_timeout" => connect(it, a),
        "TcpListener::bind" => bind(a, false),
        "UdpSocket::bind" => bind(a, true),
        "TcpListener.accept" => accept(it, id),
        "TcpListener.local_addr" => Ok(Value::str(NET.with(|n| n.borrow().listeners.get(&id).map(|l| l.addr.clone())).unwrap_or_default())),
        "UdpSocket.local_addr" => Ok(Value::str(NET.with(|n| n.borrow().udps.get(&id).map(|u| u.addr.clone())).unwrap_or_default())),
        "TcpStream.read" => match size_ok(n_arg(a, 1)) {
            Err(e) => err(e),
            Ok(max) => reading(it, id, |p| !p.data.is_empty(), |p| {
                let n = max.min(p.data.len());
                ok(bytes_value(&p.data.drain(..n).collect::<Vec<u8>>()))
            }),
        },
        "TcpStream.read_exact" => match size_ok(n_arg(a, 1)) {
            Err(e) => err(e),
            Ok(want) => {
                // a pipe never holds more than its writer sent: wait for `want` bytes or the end
                let Some((rx, _, _)) = stream_parts(id) else { return err("not connected") };
                let p = rx.clone();
                let ms = stream_parts(id).map(|s| s.2).unwrap_or(0);
                if !wait(it, ms, Rc::new(move || { let p = p.borrow(); p.closed || p.data.len() >= want }))? {
                    return err("timed out");
                }
                let mut p = rx.borrow_mut();
                if p.data.len() < want {
                    return err("unexpected end of stream");
                }
                ok(bytes_value(&p.data.drain(..want).collect::<Vec<u8>>()))
            }
        },
        "TcpStream.read_line" => reading(it, id, |p| p.data.contains(&b'\n'), |p| {
            let end = p.data.iter().position(|b| *b == b'\n').map(|i| i + 1).unwrap_or(p.data.len());
            let line: Vec<u8> = p.data.drain(..end).collect();
            match String::from_utf8(line) {
                Ok(s) => ok(Value::str(s)),
                Err(_) => err("invalid UTF-8 in line"),
            }
        }),
        "TcpStream.write_all" => write(it, id, b_arg(a, 1)),
        "TcpStream.flush" => ok(Value::Unit),
        "TcpStream.shutdown_write" | "TcpStream.shutdown" => {
            let both = key.ends_with("shutdown");
            match with_stream(id, |s| {
                s.tx.borrow_mut().closed = true;
                if both {
                    s.rx.borrow_mut().closed = true;
                    s.rx.borrow_mut().data.clear();
                    s.rx.borrow_mut().reader_gone = true;
                }
            }) {
                Some(()) => ok(Value::Unit),
                None => err("not connected"),
            }
        }
        "TcpStream.set_timeouts" => match with_stream(id, |s| s.read_ms = n_arg(a, 1)) {
            Some(()) => ok(Value::Unit),
            None => err("not connected"),
        },
        "TcpStream.peer_addr" => Ok(Value::str(with_stream(id, |s| s.peer.clone()).unwrap_or_default())),
        "TcpStream.local_addr" => Ok(Value::str(with_stream(id, |s| s.local.clone()).unwrap_or_default())),
        "UdpSocket.send_to" => {
            let to = s_arg(a, 2);
            udp_send(it, id, b_arg(a, 1), &to)
        }
        "UdpSocket.recv_from" => udp_recv(it, id, n_arg(a, 1)),
        "UdpSocket.set_read_timeout" => {
            let ms = n_arg(a, 1);
            match NET.with(|n| n.borrow_mut().udps.get_mut(&id).map(|u| u.read_ms = ms)) {
                Some(()) => ok(Value::Unit),
                None => err("not connected"),
            }
        }
        other => Err(Ctl::Err(Box::new(RuntimeError::new(ErrorKind::Unsupported, format!("`{other}` has no simulated socket operation"))))),
    }
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn addresses_normalize_localhost_and_reject_garbage() {
        assert_eq!(normalize("localhost:80"), Some(("127.0.0.1".into(), 80)));
        assert_eq!(normalize("127.0.0.1:0"), Some(("127.0.0.1".into(), 0)));
        assert_eq!(normalize("nope"), None);
        assert_eq!(normalize(":80"), None);
    }
}
