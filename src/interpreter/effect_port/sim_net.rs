//! The virtual loopback network of the simulated world (ADR-0032). A `--simulate` run never opens a socket: `bind`
//! registers a virtual listener (port 0 picks from 49152 upward, in order), `connect` to a bound address makes an
//! in-memory pipe pair and queues the server end for `accept`, `connect` to anything else is `connection refused`
//! (the world has no outside network). Reads block the task on the scheduler and advance the virtual clock; a read
//! timeout is virtual time. A dropped or shut-down end is end of stream to its peer. UDP datagrams to an unbound
//! address are dropped. With `--faults`, a read or a write fails with `connection reset` from the seeded fault stream.
//!
//! The state belongs to the world (`World::net`), behind a mutex because interpreter tasks run on their own OS threads (one
//! at a time); a simulated handle keeps its network, so dropping it closes the right socket.

use super::super::builtins::std_net::{bytes_value, next_id};
use super::super::engine::Interpreter;
use super::super::error::{Ctl, ErrorKind, RuntimeError};
use super::super::int::IntTy;
use super::super::sched::Wait;
use super::super::value::Value;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};

type R = Result<Value, Ctl>;

#[derive(Default)]
struct Pipe {
    data: VecDeque<u8>,
    /// The writer is done (shut down or dropped): the reader sees end of stream once `data` is empty.
    closed: bool,
    /// The reader is gone: a write is `broken pipe`.
    reader_gone: bool,
}

type Shared<T> = Arc<Mutex<T>>;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

struct VStream {
    rx: Shared<Pipe>,
    tx: Shared<Pipe>,
    local: String,
    peer: String,
    read_ms: i64,
}

struct VListener {
    addr: String,
    backlog: Shared<VecDeque<i128>>,
}

type Inbox = Shared<VecDeque<(Vec<u8>, String)>>;

struct VUdp {
    addr: String,
    inbox: Inbox,
    read_ms: i64,
}

#[derive(Default)]
pub(crate) struct Net {
    next_port: u16,
    streams: HashMap<i128, VStream>,
    listeners: HashMap<i128, VListener>,
    udps: HashMap<i128, VUdp>,
    listen_at: HashMap<String, i128>,
    udp_at: HashMap<String, i128>,
}

/// One world's virtual network. Interpreter tasks run on their own OS threads (one at a time), hence the mutex.
pub(crate) type Network = Arc<Mutex<Net>>;

/// A fresh, empty network for a new world.
pub(crate) fn new_network() -> Network {
    Arc::default()
}

/// The network of the running simulated world.
pub(crate) fn network_of(it: &Interpreter) -> Option<Network> {
    it.port.as_ref().and_then(|p| p.borrow_mut().world().map(|w| w.net.clone()))
}

/// Run `f` on the network (briefly: never across a scheduler switch).
fn with_net<T>(net: &Option<Network>, f: impl FnOnce(&mut Net) -> T) -> Option<T> {
    net.as_ref().map(|n| f(&mut lock(n)))
}

/// A handle was dropped: its peer reads end of stream, its address is free again.
pub(crate) fn close(net: &Network, id: i128) {
    let Ok(mut n) = net.try_lock() else { return };
    if let Some(s) = n.streams.remove(&id) {
        lock(&s.tx).closed = true;
        lock(&s.rx).reader_gone = true;
    }
    if let Some(l) = n.listeners.remove(&id) {
        n.listen_at.remove(&l.addr);
    }
    if let Some(u) = n.udps.remove(&id) {
        n.udp_at.remove(&u.addr);
    }
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
    fn port(&mut self, host: &str, port: u16, udp: bool) -> Option<String> {
        let taken = |n: &Net, a: &str| if udp { n.udp_at.contains_key(a) } else { n.listen_at.contains_key(a) };
        if port != 0 {
            let a = format!("{host}:{port}");
            return (!taken(self, &a)).then_some(a);
        }
        if self.next_port < 49152 {
            self.next_port = 49152;
        }
        loop {
            let a = format!("{host}:{}", self.next_port);
            self.next_port = self.next_port.wrapping_add(1).max(49152);
            if !taken(self, &a) {
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

/// The network of the running world, and whether this call draws a fault.
fn world(it: &mut Interpreter, faulty: bool) -> (Option<Network>, bool) {
    match &it.port {
        Some(p) => match p.borrow_mut().world() {
            Some(w) => (Some(w.net.clone()), faulty && w.fault()),
            None => (None, false),
        },
        None => (None, false),
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

fn stream_parts(net: &Option<Network>, id: i128) -> Option<(Shared<Pipe>, Shared<Pipe>, i64)> {
    with_net(&net, |n| n.streams.get(&id).map(|s| (s.rx.clone(), s.tx.clone(), s.read_ms))).flatten()
}

/// A read of stream `id`: wait until `done` (or end of stream), then `take` from the pipe.
fn reading(it: &mut Interpreter, id: i128, done: fn(&Pipe) -> bool, take: impl FnOnce(&mut Pipe) -> R) -> R {
    let (net, fault) = world(it, true);
    if fault {
        return err("connection reset");
    }
    let Some((rx, _, ms)) = stream_parts(&net, id) else { return err("not connected") };
    let p = rx.clone();
    let ready: Rc<dyn Fn() -> bool> = Rc::new(move || {
        let p = lock(&p);
        p.closed || done(&p)
    });
    if !wait(it, ms, ready)? {
        return err("timed out");
    }
    let mut p = lock(&rx);
    take(&mut p)
}

fn connect(it: &mut Interpreter, a: &[Value]) -> R {
    let addr = s_arg(a, 0);
    let Some((host, port)) = normalize(&addr) else { return err(format!("invalid address: {addr}")) };
    let (net, fault) = world(it, true);
    if fault {
        return err("connection refused");
    }
    let (cid, sid) = (next_id(it), next_id(it));
    let made = with_net(&net, |n| {
        let lid = n.listener_for(&host, port)?;
        let (up, down): (Shared<Pipe>, Shared<Pipe>) = (Arc::default(), Arc::default());
        let local = n.port("127.0.0.1", 0, false).unwrap_or_default();
        let server_addr = n.listeners[&lid].addr.clone();
        n.streams.insert(cid, VStream { rx: down.clone(), tx: up.clone(), local: local.clone(), peer: server_addr.clone(), read_ms: 0 });
        n.streams.insert(sid, VStream { rx: up, tx: down, local: server_addr, peer: local, read_ms: 0 });
        lock(&n.listeners[&lid].backlog).push_back(sid);
        Some(cid)
    })
    .flatten();
    match made {
        Some(cid) => ok(int(cid)),
        None => err("connection refused"),
    }
}

fn bind(it: &mut Interpreter, a: &[Value], udp: bool) -> R {
    let addr = s_arg(a, 0);
    let Some((host, port)) = normalize(&addr) else { return err(format!("invalid address: {addr}")) };
    let (net, _) = world(it, false);
    let id = next_id(it);
    let made = with_net(&net, |n| {
        let at = n.port(&host, port, udp)?;
        if udp {
            n.udp_at.insert(at.clone(), id);
            n.udps.insert(id, VUdp { addr: at, inbox: Arc::default(), read_ms: 0 });
        } else {
            n.listen_at.insert(at.clone(), id);
            n.listeners.insert(id, VListener { addr: at, backlog: Arc::default() });
        }
        Some(id)
    })
    .flatten();
    match made {
        Some(id) => ok(int(id)),
        None => err("address in use"),
    }
}

fn accept(it: &mut Interpreter, id: i128) -> R {
    let (net, _) = world(it, false);
    let Some(backlog) = with_net(&net, |n| n.listeners.get(&id).map(|l| l.backlog.clone())).flatten() else { return err("not connected") };
    let b = backlog.clone();
    wait(it, 0, Rc::new(move || !lock(&b).is_empty()))?;
    let next = lock(&backlog).pop_front();
    match next {
        Some(sid) => ok(int(sid)),
        None => err("timed out"),
    }
}

fn write(it: &mut Interpreter, id: i128, data: Vec<u8>) -> R {
    let (net, fault) = world(it, true);
    if fault {
        return err("connection reset");
    }
    let Some((_, tx, _)) = stream_parts(&net, id) else { return err("not connected") };
    let mut tx = lock(&tx);
    if tx.reader_gone {
        return err("broken pipe");
    }
    if tx.closed {
        return err("not connected");
    }
    tx.data.extend(data);
    ok(Value::Unit)
}

fn with_stream<T>(net: &Option<Network>, id: i128, f: impl FnOnce(&mut VStream) -> T) -> Option<T> {
    with_net(&net, |n| n.streams.get_mut(&id).map(f)).flatten()
}

fn udp_recv(it: &mut Interpreter, id: i128, max: i64) -> R {
    if max < 1 {
        return err(format!("read size must be at least 1, got {max}"));
    }
    let (net, _) = world(it, false);
    let Some((inbox, ms)) = with_net(&net, |n| n.udps.get(&id).map(|u| (u.inbox.clone(), u.read_ms))).flatten() else { return err("not connected") };
    let b = inbox.clone();
    if !wait(it, ms, Rc::new(move || !lock(&b).is_empty()))? {
        return err("timed out");
    }
    let next = lock(&inbox).pop_front();
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
    let (net, fault) = world(it, true);
    if fault {
        return err("connection reset");
    }
    let sent = with_net(&net, |n| {
        let from = n.udps.get(&id).map(|u| u.addr.clone())?;
        let len = data.len() as i128;
        let target = n.udp_at.get(&format!("{host}:{port}")).or_else(|| n.udp_at.get(&format!("0.0.0.0:{port}")));
        if let Some(u) = target.and_then(|t| n.udps.get(t)) {
            lock(&u.inbox).push_back((data, from));
        }
        Some(len)
    })
    .flatten();
    match sent {
        Some(len) => ok(int(len)),
        None => err("not connected"),
    }
}

fn size_ok(n: i64) -> Result<usize, String> {
    if n < 1 {
        Err(format!("read size must be at least 1, got {n}"))
    } else {
        Ok(n as usize)
    }
}

fn unit(r: Option<()>) -> R {
    match r {
        Some(()) => ok(Value::Unit),
        None => err("not connected"),
    }
}

/// One socket operation in the simulated world (`key` and `args` as `builtins/std_net.rs` logs them).
pub fn answer(it: &mut Interpreter, key: &str, a: &[Value]) -> R {
    let id = id_arg(a);
    let net = world(it, false).0;
    match key {
        "TcpStream::connect" | "TcpStream::connect_timeout" => connect(it, a),
        "TcpListener::bind" => bind(it, a, false),
        "UdpSocket::bind" => bind(it, a, true),
        "TcpListener.accept" => accept(it, id),
        "TcpListener.local_addr" => Ok(Value::str(with_net(&net, |n| n.listeners.get(&id).map(|l| l.addr.clone())).flatten().unwrap_or_default())),
        "UdpSocket.local_addr" => Ok(Value::str(with_net(&net, |n| n.udps.get(&id).map(|u| u.addr.clone())).flatten().unwrap_or_default())),
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
                // a pipe holds only what its writer sent: wait for `want` bytes or the end
                let Some((rx, _, ms)) = stream_parts(&net, id) else { return err("not connected") };
                let p = rx.clone();
                if !wait(it, ms, Rc::new(move || {
                    let p = lock(&p);
                    p.closed || p.data.len() >= want
                }))? {
                    return err("timed out");
                }
                let mut p = lock(&rx);
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
            unit(with_stream(&net, id, |s| {
                lock(&s.tx).closed = true;
                if both {
                    let mut rx = lock(&s.rx);
                    rx.closed = true;
                    rx.data.clear();
                    rx.reader_gone = true;
                }
            }))
        }
        "TcpStream.set_timeouts" => unit(with_stream(&net, id, |s| s.read_ms = n_arg(a, 1))),
        "TcpStream.peer_addr" => Ok(Value::str(with_stream(&net, id, |s| s.peer.clone()).unwrap_or_default())),
        "TcpStream.local_addr" => Ok(Value::str(with_stream(&net, id, |s| s.local.clone()).unwrap_or_default())),
        "UdpSocket.send_to" => {
            let to = s_arg(a, 2);
            udp_send(it, id, b_arg(a, 1), &to)
        }
        "UdpSocket.recv_from" => udp_recv(it, id, n_arg(a, 1)),
        "UdpSocket.set_read_timeout" => {
            let ms = n_arg(a, 1);
            unit(with_net(&net, |n| n.udps.get_mut(&id).map(|u| u.read_ms = ms)).flatten())
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

    #[test]
    fn two_worlds_never_share_an_address() {
        let (a, b) = (Some(super::new_network()), Some(super::new_network()));
        let pa = super::with_net(&a, |n| n.port("127.0.0.1", 0, false)).flatten();
        let pb = super::with_net(&b, |n| n.port("127.0.0.1", 0, false)).flatten();
        assert_eq!(pa, pb, "each world counts its own ports");
        assert!(super::with_net(&None, |_| ()).is_none());
    }
}
