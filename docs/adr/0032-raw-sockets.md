---
status: proposed
last_verified: 6dfc1aa49
---
# ADR-0032: Raw TCP and UDP sockets in `std::net`

- **Status:** Proposed
- **Date:** 2026-11-08
- **Related:** [ADR-0021](0021-effects-and-capabilities.md) (effects, capabilities, the effect port, fd-verified sinks),
  [ADR-0008](0008-stdlib-portability.md), [ADR-0022](0022-interpret-by-default.md), `std/net/mod.wj`,
  `crates/windjammer-runtime/src/platform/native/net.rs`, `crates/windjammer-runtime/src/net_core/mod.rs`,
  `src/interpreter/builtins/std_net.rs`, `src/ir/effect_facts/sinks.rs`, `src/interpreter/effect_port/`,
  `tests/std_net_sockets_test.rs`, `tests/drift_corpus/net/`

## Context
`std::net` declares an HTTP client, a WebSocket stub and an HTTP server, but no socket a program can speak its own
protocol over. A Redis or Postgres wire client, an SMTP or IRC line protocol, a custom game or metrics server all
need a byte stream to a `host:port`, and today the only way to one is the HTTP stack. Raw sockets exist only inside
the interpreter's HTTP client and server (`builtins/std_http.rs`, `builtins/std_http_server.rs`), where the effect
port lists `std::net`, `TcpStream`, `TcpListener` and `UdpSocket` as host reads that `--record`/`--replay` refuse
(`effect_port/mod.rs`, `COVERED`).

Constraints:
- The compiled backend and the interpreter must say the same thing about every program (drift gate, `docs/testing/DRIFT.md`).
- Every OS-touching runtime function must reach a `caps::` check (`tests/effects_runtime_audit_test.rs`).
- `wj test` must stay deterministic: tests never depend on the real network, and record/replay must reproduce a run
  bit for bit (ADR-0021 E3).

## Decision

### 1. API
Three handle types and one error type, `NetResult<T> = Result<T, string>`, in `std::net`:

```windjammer
use std::net::{TcpListener, TcpStream, UdpSocket}

let server = TcpListener::bind("127.0.0.1:0")?          // port 0: the OS (or the simulated world) picks one
let mut client = TcpStream::connect(server.local_addr())?
let mut conn = server.accept()?
client.write_str("PING\r\n")?
let line = conn.read_line()?                              // "PING\r\n"; "" at end of stream
```

| Type | Operation | Meaning |
|---|---|---|
| `TcpStream` | `connect(addr)`, `connect_timeout(addr, ms)` | open a stream to `host:port` |
| | `read(max) -> Vec<u8>` | up to `max` bytes as soon as some are available; `[]` = the peer closed its write half |
| | `read_exact(n) -> Vec<u8>` | exactly `n` bytes, or `Err("unexpected end of stream")` |
| | `read_line() -> string` | through the next `\n` (kept), `""` at end of stream, `Err` on invalid UTF-8 |
| | `write_all(Vec<u8>)`, `write_str(string)`, `flush()` | write everything or fail |
| | `shutdown_write()` (half close), `shutdown()` (both halves) | |
| | `set_timeouts(read_ms, write_ms)` | `0` = no timeout; a read past it is `Err("timed out")` |
| | `peer_addr()`, `local_addr() -> string` | `ip:port` |
| `TcpListener` | `bind(addr)`, `accept() -> TcpStream`, `local_addr()` | |
| `UdpSocket` | `bind(addr)`, `send_to(Vec<u8>, addr) -> int`, `recv_from(max) -> (Vec<u8>, string)`, `set_read_timeout(ms)`, `local_addr()` | one datagram per call |

- **Bytes are `Vec<u8>`**, as in `fs::read_bytes` and `http::Response::bytes`; `write_str`/`read_line` are the text
  conveniences line protocols want.
- **Errors are strings with a fixed vocabulary**, produced by one function (`net_core::describe`) used by both
  runtimes: `connection refused`, `timed out`, `connection reset`, `broken pipe`, `address in use`,
  `unexpected end of stream`, `invalid address: ADDR`, `not connected`, else the OS message. Programs match on
  them; the drift gate compares them.
- A handle closes when its last owner drops it (the peer then reads end of stream), as in Rust.
- `read_line` buffers inside the stream; `read`/`read_exact` drain that buffer first, so mixing them never loses bytes.

### 2. Capabilities
The sink table (`sinks.rs`) gains:

| Sink | Effect | Resource |
|---|---|---|
| `TcpStream::connect`, `TcpStream::connect_timeout` | `net.connect` | argument 0, `host:port` |
| `TcpListener::bind` | `net.listen` | argument 0, `host:port` |
| `UdpSocket::bind` | `net.listen`, and `net.connect` as an optional scope | argument 0; the socket may send anywhere, so `send_to` checks `net.connect` on its destination (argument 1) for every datagram, and the optional scope makes a compiled program install its grant |

Grants are the existing per-host allowlist (`net.connect = ["db.internal:5432", "*.example.com"]`,
`net.listen = ["127.0.0.1"]`; no port in a pattern = any port). `accept`, `read`, `write` and `recv_from` are
operations on an authorized handle and are not sinks (`NOT_SINKS` documents them), the same rule as `subprocess`.

**Address-verified connects** (the network analogue of ADR-0021's fd-verified reads): `connect` resolves the name
once, checks the grant on the name as written, then connects only to the addresses that one resolution returned
(`net_core::resolve_once` + `connect_resolved`); it never resolves again between the check and the connect, so a
DNS answer cannot change between the decision and the use. `sinks::ADDR_VERIFIED_CONNECTS` names these sinks, and
`tests/effects_runtime_audit_test.rs` requires the compiled runtime to connect through `connect_resolved`.

Compiled: the sink rows give `wj build` the same `Proven`/`Runtime`/`Denied` facts as every other sink; the runtime
methods call `caps::net_connect` / `caps::net_listen` (a no-op when no grant is installed). Interpreter: the gate
runs on the table row before the builtin, and `UdpSocket.send_to` asks it per datagram (`cap_decide`), as
`ClientRequest.send` does. A denial is the uncatchable fault of ADR-0021 (exit status 126) in both.

### 3. The effect port: record, replay, simulate
Socket handles are not refused under the port (unlike `subprocess::spawn`). Every socket operation is one event:

- `connect`/`bind`/`accept` log a **handle id** (`{"ok": 3}`), never the OS socket. Later operations log
  `(id, arguments) -> result` (`TcpStream.read [3, 4096] -> {"ok": [104, 105]}`).
- `--record` performs them live and appends; `--replay` answers every one from the log and opens no socket (a
  replayed handle is just its id), so a replayed run needs no server.
- `--simulate` answers from the **virtual loopback network** of the seeded world (`effect_port/sim_net.rs`): `bind`
  registers a virtual listener (port 0 picks from 49152 upward, deterministically), `connect` to a bound address makes
  an in-memory pipe pair and queues the server end for `accept`, `connect` to anything else is
  `connection refused` (the simulated world has no outside network), reads block the task cooperatively and
  advance the virtual clock (a read timeout is virtual time), a dropped or shut-down end is end of stream to its
  peer, UDP datagrams to an unbound address are dropped, and `--faults` turns a read or write into
  `connection reset` from the seeded fault stream. A read that no task can ever satisfy is the scheduler's
  `deadlock` error, which a compiled program would hang on instead.

So a test of a line protocol runs a server and a client in one program (or two tasks), under `wj test --simulate`,
without touching the network, and the same test passes live on loopback and compiled.

### 4. One implementation, two runtimes
`crates/windjammer-runtime/src/net_core/mod.rs` is the only code that touches `std::net` for these types: buffering,
timeouts, resolution, error vocabulary. The compiled runtime's `platform/native/net.rs` wraps it with `caps::` checks;
the interpreter includes the same file (`#[path]`, the pattern of `caps_core` and `durable_core`) behind its gate and
port. Codegen needs no special case: `use std::net::TcpStream` already maps to
`windjammer_runtime::platform::native::net::TcpStream`, and the scanned signatures (`&str`, `&[u8]`, `i64`) give the
ownership facts.

Gates that keep them identical:
- `tests/std_net_sockets_test.rs`: one suite of programs run interpreted, compiled, simulated and replayed, compared
  with outputs written by hand from the protocol, and with **an independent peer** (a server and a client written
  in Rust `std::net` inside the test process) for echo, partial reads, timeouts, connection refused, half close,
  UDP and grant denial.
- `tests/drift_corpus/net/`: the loopback programs join the drift corpus (walker, resolved, compiled).
- `tests/std_runtime_parity_test.rs` also reads `platform/native/<module>.rs` and the methods of the declared types,
  so a method declared in `std/net/mod.wj` without a compiled twin is a gap.

## Consequences
- Line and binary protocol clients and servers are ordinary Windjammer, testable without a network.
- TLS is out of scope (a later `std::tls` wraps a `TcpStream`); so are non-blocking sockets and `select` (the
  interpreter's tasks and timeouts cover concurrency for now).
- A live socket call in the interpreter blocks its OS thread, so other interpreter tasks do not run while it waits (compiled threads do). Lockstep protocols in one task work everywhere; concurrent servers in one interpreted program are tested under `--simulate`, where reads block cooperatively. Cooperative live polling is the next rung.
- The wasm target has no sockets: `platform/wasm/net.rs` returns `Err("sockets are not available on wasm")`.

## Kill criteria
- The drift corpus net cases diverge in a way that needs a per-backend special case in codegen: stop and move the
  types behind one runtime trait instead.
- Replay of a recorded socket session is not bit-exact on the suite: narrow the port claim to simulate-only.
