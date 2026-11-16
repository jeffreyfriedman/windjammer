---
status: proposal
---
# Removing `async fn` (ADR-0004 decision 1 and 5, PLAN.md stage 2b step 4)

> **Proposal: not implemented.** The design for stage 2b step 4; `async fn` and `@async` still parse today.

Functions are not coloured: the caller decides how a call runs (`f()`, `async f()`, `spawn f()`). This note is the
design for deleting `async fn`, `@async` and `.await` on plain calls, with the bridge that keeps the standard library's
HTTP client and server (the only users of tokio today) working, and the exact code the compiler generates.

## What exists today

- `async fn f()` / `@async fn f()` parse; the Rust backend emits `async fn` and, for `main`, `#[tokio::main]`; the
  interpreter runs the body lazily on `.await`.
- `std/http.wj` declares the HTTP client (`get`, `post`, `put`, `patch`, `delete`, `head`, `Response.text/json/bytes`)
  as `@async`. The runtime already has blocking implementations of the same calls (`windjammer_runtime::http` drives an
  internal tokio runtime with `block_on`).
- The HTTP server is already synchronous at the Windjammer surface: `Server::serve(handle)`, `serve_fn`, `Router` run
  each handler with `tokio::task::spawn_blocking` (crates/windjammer-runtime/src/http.rs `run_handler_blocking`), so a
  handler is an ordinary `fn(ServerRequest) -> ServerResponse`. The axum 0.8 route spelling fix (`/{*path}`,
  `:name` -> `{name}`, commit d7c3d8dd4) lives entirely inside the runtime and is unaffected by this change.

So the only async surface a program sees is the client and `async fn` itself. Tokio stays where it is: inside the
runtime's `net`/`db` feature groups, compiled only for programs that use `std::http` or `std::db`
(src/cargo_toml/runtime_features.rs). Lean builds keep no tokio by default.

## The bridge: blocking calls, with overlap from the task surface

Every runtime entry point is blocking (`fn get(url: &str) -> Result<Response, String>`). Overlap comes from the call
site, not from the callee's colour:

| Source | Meaning | Compiled (Rust) |
|---|---|---|
| `http.get(url)?` | call, wait for the result | `windjammer_runtime::http::get(&url)?` (internal `block_on`) |
| `let t = spawn http.get(url)` | overlap on a thread | `task::spawn(move \|\| http::get(&url))` |
| `let t = async http.get(url)` | overlap on the current executor | `task::io(move \|\| http::get(&url))` (see below) |

`async` of a call whose effect set (ADR-0021, `ir::effect_facts`) contains `net` or `fs` lowers to `task::io`: the
runtime runs it on a small shared I/O pool (2 threads by default, `WJ_IO_THREADS`), because a blocking call deferred
on the local queue would not overlap with anything. `async` of effect-free work keeps the local queue
(`task::local`). The effect facts already exist and are solver-decided, so the user never chooses. A program that needs
thousands of concurrent connections (a proxy, a chat server) is the case a reactor serves better; that is the
post-beta "async variants on demand" item (decision 5): the compiler generates a reactor-driven variant of a function
only when a call site needs it, behind the `async-io` runtime feature. It is not needed for beta.

The interpreter needs no bridge: its scheduler already overlaps `async`/`spawn` tasks that block on the effect port
(sockets, sleeps) and keeps the virtual clock exact.

## Lowering examples

### A client call

```windjammer
// before (today)
@async
fn fetch(url: string) -> Result<string, string> {
    let response = http.get(url).await?
    response.text().await
}

@async
fn main() {
    let body = fetch("https://example.com").await
    println("${body ?? \"failed\"}")
}
```

```windjammer
// after `wj fix`
fn fetch(url: string) -> Result<string, string> {
    let response = http.get(url)?
    response.text()
}

fn main() {
    let body = fetch("https://example.com")
    println("${body ?? \"failed\"}")
}
```

Generated Rust (after):

```rust
fn fetch(url: &str) -> Result<String, String> {
    let response = windjammer_runtime::http::get(url)?;
    response.text()
}

fn main() {
    let body = fetch("https://example.com");
    windjammer_runtime::io::println(&format!("{}", body.unwrap_or_else(|_| "failed".to_string())));
}
```

No `#[tokio::main]`, no `async fn`. The runtime's `http::get` keeps its internal runtime.

### Two requests that overlap

```windjammer
fn main() {
    let a = async http.get("https://a.example")
    let b = async http.get("https://b.example")
    println("${a.wait()?.status_code()} ${b.wait()?.status_code()}")
}
```

```rust
fn main() {
    let a = { windjammer_runtime::task::io(move || windjammer_runtime::http::get("https://a.example")) };
    let b = { windjammer_runtime::task::io(move || windjammer_runtime::http::get("https://b.example")) };
    // ...
}
```

### A server handler (unchanged)

```windjammer
fn handle(req: ServerRequest) -> ServerResponse {
    let profile = spawn load_profile(req.query("id") ?? "0")
    let stats = load_stats()
    ServerResponse::ok("${profile.wait()} ${stats}")
}

fn main() {
    Server::new("0.0.0.0", 8080).serve(handle)
}
```

The handler runs on `spawn_blocking` as today; `spawn` inside it is an ordinary task (a thread; the handler's scope
rules apply: the handler waits for it). No route changes.

## Migration (`wj fix`) and diagnostics

- `async fn` / `@async fn` / `@async` on a method: WJ0191 (removed concurrency form), machine-applicable fix deleting the
  keyword or decorator.
- `.await` on a call of a function that was `async` (same program, or one of the std client functions listed in
  `std/http.wj`): fix deletes `.await`. The parser cannot know this (it is name-based), so it is a typed fix in
  `check/tasks.rs`: the fix runs after the `async fn` rewrite, with the program's former-async function names.
- `.await` on anything else stays legal: it is sugar for `.wait()` on a `Task`. On a value that is not a task, it is
  WJ0003 (type mismatch) with the hint "remove `.await`: this call is not a task".
- `std/http.wj`: the `@async` decorators go; the stubs become plain `fn`s mapped to the blocking runtime functions.
- `std/async.wj` (`sleep_ms`, `block_on`, `spawn`): `block_on` and `spawn` are removed (`spawn` is a keyword now);
  `sleep_ms` stays as a plain blocking sleep.

## Work list and estimate (2.5 agent-days)

1. Runtime: `task::io` + I/O pool (std threads; 0.5 day). Effect-directed choice in `generate_task` (0.5 day).
2. Parser: `async fn` and `@async` as WJ0191 removed forms with edits (0.5 day); typed `.await` fix (0.5 day).
3. `std/http.wj`, `std/async.wj`, docs/book samples, the interpreter's lazy-future path (delete) (0.5 day).
4. Gates: the HTTP runtime tests (`crates/windjammer-runtime/tests/http_serve_fn_root_test.rs`) unchanged and green;
   a migration test (old client program -> `wj fix` -> same output interpreted and compiled against a local test
   server); WJ0191 cases for `async fn`; a test that no generated program for a non-HTTP sample references tokio.

Kill criterion: if the blocking client in a handler measurably starves the server's blocking pool under the PC load
test (p99 latency more than 2x the async version at 200 concurrent requests), keep `@async` for the client only, as a
runtime-internal annotation, and revisit with the reactor variant.
