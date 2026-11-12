# Sockets: TCP and UDP
<!-- feature: raw-sockets -->

`std::net` has an HTTP client and server for the common case, and raw sockets for the rest: a line protocol, a wire
format of your own, a metrics port. A socket is a handle you read from and write to. Three types cover it:
`TcpListener` (accept connections), `TcpStream` (a connection) and `UdpSocket` (datagrams). Errors are plain strings with a
fixed vocabulary (`connection refused`, `timed out`, `connection reset`, `address in use`, `unexpected end of stream`) so a
program can match on them.

A network is an outside resource, so a program must be granted it. `net.listen` covers `bind`, and `net.connect` covers
`connect` and every datagram a UDP socket sends. Grants name hosts, and a pattern without a port means any port. Without a
grant the call fails with a diagnostic that names the flag to add ([Effects and capabilities](effects.md) has the model).

## A conversation over TCP

This program starts a server, connects a client to it and has the two exchange a line each. Binding to port `0` lets the
system pick a free port, and `local_addr()` tells you which. `read_line` returns through the next newline, and an empty
string at the end of the stream:

```sh,verify
$ cat > ping.wj <<'EOF'
use std::net::TcpListener
use std::net::TcpStream

fn main() -> Result<(), string> {
    let server = TcpListener::bind("127.0.0.1:0")?
    let mut client = TcpStream::connect(server.local_addr())?
    let mut conn = server.accept()?
    client.write_str("PING\r\n")?
    let line = conn.read_line()?
    println("server got ${line.trim()}")
    conn.write_str("PONG\r\n")?
    let reply = client.read_line()?
    println("client got ${reply.trim()}")
    Ok(())
}
EOF
$ wj run --allow net.listen=127.0.0.1 --allow net.connect=127.0.0.1 ping.wj
server got PING
client got PONG
```

Without the grants, the same program stops at the first socket call with exit status 126 and tells you what to add:

```sh,verify
$ cat > ping.wj <<'EOF'
use std::net::TcpListener

fn main() -> Result<(), string> {
    let server = TcpListener::bind("127.0.0.1:0")?
    println("listening on ${server.local_addr()}")
    Ok(())
}
EOF
$ wj run ping.wj 2>&1 | head -n 1
error: capability denied: net.listen(127.0.0.1)
```

## Datagrams over UDP

A `UdpSocket` sends one datagram per call. `send_to` takes the bytes and the destination and returns how many bytes it
sent; `recv_from` returns the bytes and the address they came from:

```sh,verify
$ cat > dgram.wj <<'EOF'
use std::net::UdpSocket

fn main() -> Result<(), string> {
    let a = UdpSocket::bind("127.0.0.1:0")?
    let b = UdpSocket::bind("127.0.0.1:0")?
    let sent = a.send_to("hello".as_bytes().to_vec(), b.local_addr())?
    let (data, from) = b.recv_from(64)?
    println("sent ${sent} bytes, received ${data.len()}, from a: ${from == a.local_addr()}")
    Ok(())
}
EOF
$ wj run --allow net.listen=127.0.0.1 --allow net.connect=127.0.0.1 dgram.wj
sent 5 bytes, received 5, from a: true
```

## Test a protocol without a network

`--simulate` runs the program in a seeded world with a virtual loopback network: `bind` registers an in-memory listener,
`connect` to a bound address makes an in-memory pipe, and `connect` to anything else is `connection refused`, because the
simulated world has no outside. A read waits for its peer by letting other tasks run, and a read timeout is virtual time,
so a test of a client and a server in one program is fast and the same on every run. This is the same program as above,
run in the simulated world:

```sh,verify
$ cat > ping.wj <<'EOF'
use std::net::TcpListener
use std::net::TcpStream

fn main() -> Result<(), string> {
    let server = TcpListener::bind("127.0.0.1:0")?
    let mut client = TcpStream::connect(server.local_addr())?
    let mut conn = server.accept()?
    client.write_str("PING\r\n")?
    let line = conn.read_line()?
    println("server got ${line.trim()}")
    conn.write_str("PONG\r\n")?
    let reply = client.read_line()?
    println("client got ${reply.trim()}")
    Ok(())
}
EOF
$ wj run --simulate --seed 1 --allow net.listen=127.0.0.1 --allow net.connect=127.0.0.1 ping.wj
server got PING
client got PONG
```

A run that touches the network can also be recorded and replayed ([Debug a failing run deterministically](time-travel.md)):
a replay answers every socket call from the log and opens no socket, so it needs no server.

Sockets are not available in the [playground](playground.md), whose runs are sandboxed; the programs on this page run
with `wj run`. TLS and non-blocking `select` are on the roadmap.
