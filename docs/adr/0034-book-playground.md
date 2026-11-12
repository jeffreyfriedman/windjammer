---
status: proposed
last_verified: 6dcc9cbfc
---
# ADR-0034: A live playground in the Windjammer Book, running in the reader's browser

- **Status:** Proposed (first slice implemented: Windjammer and TypeScript run in the browser; see "Slice status")
- **Date:** 2026-11-09
- **Related:** [ADR-0021](0021-effects-and-capabilities.md) (capabilities, effect port, `--simulate`, `--limits`),
  [ADR-0032](0032-raw-sockets.md) (the simulated network), [ADR-0030](0030-interpreter-rejects-what-rustc-rejects.md)
  (the interpreter refuses what `wj check` rejects), `docs/INTERPRETER.md`, `docs/testing/INTERPRETER_CONTRACT.md`,
  `crates/windjammer-playground/`, `docs/book/playground/`, `scripts/book/book_samples.wj`,
  `scripts/book/playground_check.wj`

## Context

The book (`docs/book`, mdBook, published to GitHub Pages by `.github/workflows/book.yml`) shows hundreds of
programs; every one is run in CI, interpreted and compiled (`scripts/book/book_samples.wj`). A reader cannot run
them. GitHub Pages is static: there is no server to run code on, and this phase must not add one. So the code has to
run in the reader's browser, which for Windjammer means the interpreter compiled to WebAssembly.

The questions this ADR answers with evidence from a spike on this branch:

1. Can `src/interpreter` build for `wasm32-unknown-unknown`, and what blocks it?
2. How is a reader's program confined (capabilities), stopped (infinite loops), and shown (output, diagnostics)?
3. How does the playground stay in sync with the language?
4. What about the other languages the book shows side by side (TypeScript, Rust, Go)?

## Findings

### 1. The wasm build: no large blocker; every one fixed at a module boundary

Measured with `cargo check`/`cargo build --target wasm32-unknown-unknown` on this branch (Apple clang, which has no
wasm backend: the same position as a stock CI runner). Compile-time blockers came first; run-time ones (std functions
that compile for wasm32-unknown-unknown but panic there) were found by running the module and reading the panic
message out of the trapped instance's memory (`wasm.rs` panic hook), and the call chain from an unstripped build's
stack trace.

| Blocker | Kind | Where | Resolution in this slice |
|---|---|---|---|
| `ring` (C and assembly, via `jsonwebtoken`) | C code: needs a wasm-capable clang | `std::jwt` in `builtins/std_extra.rs` | dependency moved to `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`; on wasm `jwt::sign_hs256`/`verify_hs256` return `Err("std::jwt is not available in the browser playground")` (two cfg'd helpers, one call site each) |
| `libsqlite3-sys` (bundled SQLite C) | C code | `std::db` (`builtins/std_db.rs`) | host-only dependency; the module is not registered on wasm (`db::*` is an unknown function there) |
| `libloading` | dynamic loading | host bridge (`host_bridge.rs`, ADR-0023), C library calls (`c_library.rs`, already `cfg(unix)`) | host-only dependency; `HostLib::open` returns an error on wasm |
| `libc::getrusage` | process API | `limits.rs` (`mem` limit) | on wasm the `mem` limit reads the instance's linear memory size |
| `Instant::now()` at limit setup | clock (panics on wasm32-unknown-unknown) | `limits.rs` | the clock is read only when a `wall` limit is set |
| `getrandom` 0.2 and 0.3 | entropy (compile error without a backend) | `rand`, `bcrypt`, `argon2` | the playground crate registers a custom backend: a fixed stream (the playground has no entropy; programs draw from the seeded world) |
| `wasm-bindgen` imports (via `chrono`'s default `wasmbind` feature) | host imports | `Cargo.toml` | `chrono` with `default-features = false, features = ["clock", "std", "oldtime"]`: the module now imports nothing |
| `usize` overflow in `STACK_BYTES = 4 << 30` | 32-bit target | `runner.rs` | 512 MiB on 32-bit targets |
| `Instant::now()` in `Interpreter::new` (clock origin) | run time: panics | `engine.rs` | an `Epoch` type that is `Instant` natively and a clockless stand-in on wasm32 (the playground's time is always virtual) |
| `Instant::now()` stamping the fs-read decision cache | run time: panics | `caps_core/grant.rs` (shared with the runtime crate) | no decision cache on wasm32 |
| `env::temp_dir()` while materializing embedded assets (`std/`, `stdlib_meta/`, the runtime crate) and for the analysis cache | run time: panics ("no filesystem on this platform") | `embedded_assets.rs`, `metadata/crate_metadata.rs`, `stdlib_scanner.rs` | on wasm32 materializing reports `Unsupported` (no warning), the runtime-source scan is skipped, the cache path is a dummy that fails like any missing file |
| **the library without the `cli` feature did not compile at all** | gating defect | `compiler_database.rs` (salsa), `cargo_integration.rs` (`test_runner`), `toolchain/`, `cargo_toml/` (`cargo_cache`), `ejector.rs`, `compiler/library_multipass.rs` (`build_utils`), `reduce/` (tempfile) | `salsa` is a plain dependency (the build path needs it with or without the CLI); `cargo_cache` and `build_utils` are always compiled (pure std); `copy_dir_recursive` moved from `test_runner` to `build_utils`; `cargo_integration` uses `cargo_toml::sanitize_path_for_toml` (the same function); `reduce` is `cli`-only |

Everything else compiles for wasm32 unchanged: the lexer, parser, resolver, `check` (the whole `wj check` front end),
the analyzer and code generators, the interpreter's two engines, salsa, rayon (unused on the run path), regex,
serde, chrono, the crypto crates. Native builds see the same dependency set as before (the moved dependencies are
host-only, not optional; `cli` enables the same crates minus salsa, which is now always on and which `default` builds
already had; chrono's dropped `wasmbind` feature only ever applied to wasm targets).

What the wasm build cannot do at run time, and what happens instead (all loud, none silent):

| Sink | In the browser | Mechanism |
|---|---|---|
| files, environment, processes, network, FFI, Rust crates, MCP | denied before the call, exit status 126, the same message `wj run --sandbox` prints | the capability gate (ADR-0021) with `Grant::sandbox`: only console, time and random |
| time | virtual clock from Unix time 1 700 000 000, moved only by sleeps | `--simulate` world (`effect_port/sim.rs`) |
| random numbers, hash order, task schedule | functions of the seed (default 0) | `--simulate` |
| threads, `thread {}`, `spawn` | run as nested calls on the one thread (`Sched::one_thread`, the default on wasm32); a schedule that would have to resume a task blocked further down the stack stops with an `Unsupported` error that says so | native tasks are OS threads with a baton (`sched.rs`); `std::thread::spawn` fails on wasm32-unknown-unknown |
| `std::db`, `std::jwt`, host bridge | error naming the module | above |
| a Rust panic inside the interpreter (a defect) | the Worker reports "internal error in the playground interpreter", exit 70 | `panic = "abort"` traps; the JS host catches the trap and uses a fresh instance for the next run |

The checker's view of the standard library is the same in the browser as natively: `stdlib_scanner::populate_runtime_signatures`
and `SignatureRegistry::load_stdlib_meta` read the runtime crate's sources, the `std/` stems and `stdlib_meta/*.wj.meta`
from the copies baked into the binary (`embedded_assets::files`, `embedded_assets::text`), on every target; no directory is
materialized or walked (`merge_embedded_stdlib_meta` in `metadata`). The native materialized copy is written from the same
tables, so what the scanner sees did not change.

Threads: the scheduler is a deterministic single-runner baton (`sched.rs`). Natively each task is an OS thread so the
tree walker can recurse on a private stack. On wasm32 (and wherever `Interpreter::set_one_thread_tasks(true)` is called)
a task is a nested call: when the running task blocks, the scheduler makes the same pick it always makes and, if that task
has not started, runs it to completion on top of the blocked task's stack, swapping the interpreter state in and out the
same way; then it re-checks. The tree walker keeps its state on the host stack, so a task that has *started* is always a
caller further down the same stack and cannot be resumed until the tasks above it finish. When the schedule would resume
one (three tasks that sleep for 300, 100 and 200 ms, for example: the second to start wakes first), the run stops with an
`Unsupported` error naming the task instead of printing a different interleaving. Every thread, task, `scope` and channel
sample in the book runs to completion or waits only on tasks that start later, and prints exactly what the native
scheduler prints (`tests/interpreter_one_thread_tasks_test.rs` runs both modes in one process on those samples and on the
concurrency corpus). Full coroutines for the refused shapes need a stack-switching wasm runtime (the stack-switching
proposal, or Asyncify); revisit when one ships in the browsers the book supports.

### 2. Confinement: the existing capability model, no new mechanism

The playground run is exactly

```sh
wj run --interpret --sandbox --simulate --seed 0 --limits fuel=10000000,output=1048576 main.wj
```

on one in-memory file, with the same code paths (`crates/windjammer-playground/src/lib.rs`):

1. `check::check_sources` on the source; any error-severity diagnostic refuses the run (ADR-0030), rendered by
   `diagnostics::render` exactly as `wj run` renders it, and also returned as JSON (the `wj check --json` schema) for
   an editor to underline.
2. `Interpreter::new_capturing` (+ `set_capture_stderr`, new: the browser has no process stderr), the tree walker
   (fuel is counted there), `CapGate::new(Grant::sandbox(..), CapMode::Enforce)`, `runner::install_simulation`,
   `LimitSpec { fuel, output }`, `guard_stack_bytes` (new: recursion bounded by bytes of the 32 MiB wasm stack, the
   same guard the native big-stack thread uses).
3. `load_sources` + `call_main`; exit status by `runner::exit_code_for` / `runner::main_result_code` (now public, and
   it writes `Error: ...` through the interpreter's stderr instead of `eprintln!`, which is the same bytes natively).

There is no grant to give: the playground has no files, no network and no environment to grant. Because the world is
simulated, a program prints the same thing on every run and on every reader's machine, which is what a book wants.

### 3. Stopping a program

Two independent stops:

- **Fuel** (inside wasm): 10 million evaluated expressions (about 1.4 s of tree-walking in wasm), exit 124
  with `limit exceeded: fuel`. Output is capped at 1 MiB the same way. Memory is capped by the instance itself:
  wasm32 linear memory cannot exceed 4 GiB, and the browser kills a Worker that exhausts it without touching the page.
- **Wall clock** (outside wasm): the run happens in a Web Worker; the page terminates it after 10 s and starts a
  fresh one. Termination does not depend on the interpreter's cooperation, so even a defect that loops without
  ticking fuel cannot hang the tab.

### 4. Bundle size, startup, memory (measured)

Measured on this branch (Apple M-series laptop under load from other builds; node 24 as the host; release profile
`opt-level = "s"`, fat LTO, `panic = "abort"`, stripped):

| Measure | Value |
|---|---|
| `windjammer_playground.wasm` | 9.67 MB raw, 2.86 MB gzip -9, 2.04 MB brotli -q 11 (GitHub Pages serves gzip) |
| compile the module (V8, lazy tiers) | 16 ms in node; the page compiles it in the Worker while the reader reads |
| first run of `hello` (fresh instance, `wj check` + interpret) | 46 ms; later runs 8 ms |
| tree walker throughput | 10 million fuel steps in 1.44 s (50 million in 6.2 s without LTO) |
| node process RSS after the runs | 105 MB |
| wasm build time | about 4 minutes cold for the whole compiler (two build jobs) |

The whole compiler front end is in the module (lexer to checker), which is what makes the diagnostics identical; the
size is dominated by the analyzer and code generators that `wj check` uses to predict ownership errors (ADR-0030).
`wasm-opt -Oz` and splitting the checker into a lazily loaded second module are the obvious size levers (ladder
step 6).

The default fuel is 10 million steps (about 1.4 s), not more: the page's 10 s wall clock is the backstop, and a book
sample needs far less (the largest sample ran well under it).

### 5. Output and errors

`run(source) -> {stdout, stderr, diagnostics, exit_code, steps}` is the whole interface (`outcome.rs`). The wasm
module exports `wj_alloc`, `wj_free`, `wj_run`, `wj_result_ptr`, `wj_result_len` and `memory` (plus the two getrandom
hooks) and has **no imports**: no
`wasm-bindgen`, no JavaScript glue, nothing from the host. `wj-wasm.mjs` checks both facts on load, so a dependency
that started importing a host function (a clock, entropy) would fail loudly. Each run gets a fresh instance of the
compiled module (milliseconds), so one run's memory never leaks into the next.

### 6. Staying in sync with the language

The book's samples are the oracle. `scripts/book/book_samples.wj --wasm RUNNER` runs every single-file Windjammer
sample of the book through `node docs/book/playground/run-node.mjs` (the same `wj-wasm.mjs` loader the page's Worker
uses, so CI exercises the reader's code path) and requires its exit status and stdout to equal the native
`wj run --interpret --sandbox --simulate --seed 0 --limits ...` of the same file. `scripts/book/playground_check.wj`
builds the wasm, checks the generated TypeScript sandbox, extracts the samples and runs that pass.

Result on this branch (`scripts/book/playground_check.wj`, 122 extracted samples):

```text
book samples: 122 passed interpreted, 5 projects, 0 compiled-checked, 115 playground-checked, 5 known playground gaps, 0 failed
```

The five gaps were the five samples that start threads (`thread {}` / `spawn`); they run now. Three project samples remain
listed in `scripts/book/playground_known_gaps.txt` (one with a path dependency on a sibling project, two with a Rust crate
dependency, which cargo builds and the browser cannot). The list stays shrink-only: a listed sample that starts agreeing
fails the run until its line is deleted. Multi-file projects run in the playground too: `wj_run_project` takes the files as
JSON `[{path, source}]` and loads them as an in-memory source tree through `load_sources`, and `run-node.mjs` takes a
project directory. The
four `ts,run` companions pass the TypeScript pass below. Both passes fail when broken: the wasm pass found the five
thread samples before they were listed, and a stripper planted to append a line made all four TypeScript samples fail.

The parity work also found a native bug, fixed here: under `--simulate` (and `--replay`) the file calls that the live
interpreter checks on the descriptor it opens (`fs::read_to_string`, `fs::write`, ...) skipped the capability gate,
because nothing is opened in the simulated world. `wj run --sandbox --simulate` could therefore read the simulated
`/etc/hosts`. The gate now decides those calls by name under a simulating or replaying port
(`tests/effect_simulation_test.rs::the_gate_decides_fd_verified_reads_and_writes_in_the_simulation`).

### 7. Other languages

The book shows TypeScript, Go and Rust next to Windjammer (`ts,run`, `go,run`, `rust,generated` fences).

**TypeScript: built, in the reader's browser, behind four layers.** The owner asked for it if it is safe; this slice
builds it and tests every escape route below in headless Chrome. Every route in the table is closed and tested (the
browser tests all pass), so TypeScript ships in the playground, not as an opt-in link. The one limit the web platform does
not offer is a per-Worker memory cap; it is stated below, and its failure mode (the browser kills the Worker) cannot
reach the page or the network.

1. *Type stripping* with sucrase 3.35.0 (MIT; its bundled dependencies are MIT and Apache-2.0, licenses in
   `docs/book/playground/vendor/LICENSES.txt`), vendored as one pinned file (sha256
   `a7dce55c42fc0649e08a0ccd2cdad0b3e86dde499a1a398d124ab0cf05e2e23f` at the time of writing), never fetched from a CDN.
   Types are erased, nothing else is rewritten (no downlevelling, no module transform), which matches what
   `node --experimental-strip-types` (the book runner's oracle) does to erasable TypeScript.
2. *An opaque-origin frame*: `<iframe sandbox="allow-scripts">`, never `allow-same-origin`, so the frame has no cookies,
   no storage, no access to the page's DOM, and the page's origin is not its origin.
3. *A CSP with no network and no URL-loaded code*: the frame's document is generated
   (`build-ts-sandbox.mjs`) as one inline script allowed by its sha256 hash:
   `default-src 'none'; script-src 'sha256-...' blob:; worker-src blob:; connect-src 'none'`. No `'self'`: a program
   cannot even `import()` from the book's own site, so there is no request at all, to any origin. No
   `'unsafe-eval'`. The Worker that runs the program is created from a `blob:` URL and inherits this policy.
4. *A prelude in the Worker* (defense in depth, `ts-core.mjs`): captures `console` into the output pane, removes
   `fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource`, `importScripts`, `Worker`, storage, `navigator`,
   `postMessage` and the rest of the `BANNED` list from the global object and every prototype it inherits them
   through, and accounts timers so a run ends when nothing is pending (as node exits).

Limits: the frame terminates the Worker after 5 s (the page replaces the whole frame if the frame does not answer
within 7 s) and after 1 MiB of output. The web platform has no per-Worker memory cap; a Worker that exhausts memory is
killed by the browser without taking the page down, and the page then replaces the frame.

Threat model (T1 of ADR-0021: an untrusted program, here possibly from a shared link):

| Escape route | Closed by | Test (`docs/book/playground/tests/browser.test.mjs`) |
|---|---|---|
| network exfiltration: `fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource`, `sendBeacon`, to the site or another origin | CSP `connect-src 'none'` (browser-enforced, inherited by the Worker); prelude removes the APIs | every attempt reported `blocked`, and two trap servers (same origin and a second origin) log zero requests; a second test creates a Worker *without* the prelude inside the frame and the CSP alone blocks fetch, XHR, `importScripts`, `import()` and eval, with zero requests logged |
| loading code: `importScripts`, `import()`, nested `Worker` | `script-src` names only the hash and `blob:`; prelude removes `importScripts`/`Worker` | same tests |
| `eval`, `Function` | no `'unsafe-eval'` | `eval`/`Function constructor` blocked |
| storage and cookies: `localStorage`, `indexedDB`, `caches`, `document.cookie` | opaque origin (sandbox without `allow-same-origin`); not present in a Worker | frame test: `self.origin === "null"`, `localStorage` and `document.cookie` throw |
| parent frame: `window.parent.document`, `top` | Worker has no window; the frame is cross-origin to the page | `parent document`/`top document` blocked; frame test: `window.parent.document` throws |
| forged results via `postMessage` | the program cannot reach `postMessage` (removed); the frame accepts runs only from `window.parent`; the page accepts results only when `ev.source` is its frame and `ev.origin === "null"`, and checks every field's type | `forged result message` blocked; a message posted to the page from elsewhere is ignored |
| denial of service: infinite loop, output flood | Worker terminated on the wall clock; output cap | loop stopped within the budget while the page answers in under a second; flood stopped |
| a transpiler bug that emits code | the emitted code still runs inside all four layers | |

The headless oracle: `scripts/book/book_samples.wj --ts-playground RUNNER` runs every `ts,run` companion through
`docs/book/playground/run-ts-node.mjs`, which uses the same `stripTypes` and the same `programSource` (prelude and
all) in a `node:vm` context, and requires the output and exit status of `node --experimental-strip-types`.

Hosting note: the frame's document is inline, so it needs no CORS; the Windjammer Worker and the wasm file are
same-origin. GitHub Pages serves `.wasm` as `application/wasm` and `.mjs` as JavaScript; no special headers are needed
(no `SharedArrayBuffer`, so no cross-origin isolation).

**Rust and Go: opt-in links, no embedding.** A Rust or Go fence gets an "Open in the Rust/Go Playground" link that
the reader clicks; the code is placed in the link (Rust: `play.rust-lang.org/?code=...`; Go: the Go Playground has
no code-in-URL parameter, so the link opens the playground and the reader pastes, or a later step uses its share API
on click). Nothing is sent before the click, nothing is embedded, no third-party script runs in the book. Compiling
Rust or Go in the browser (a wasm `rustc` or `go`) is out of reach for a static page today (hundreds of MB, minutes
per compile). A self-hosted execution server is ruled out for this phase.

## Options considered

- **A. Interpreter compiled to wasm, run in a Worker (chosen).** No server, the real interpreter (not a port), the
  same checker diagnostics, deterministic output, confinement from the existing capability model.
- **B. A server that runs `wj` (like the Rust and Go playgrounds).** Ruled out: GitHub Pages is static, a public
  execution service is an operational and abuse burden, and code would leave the reader's machine.
- **C. Compile to JavaScript (`wj build --target js`) in the browser.** The JS backend is a separate semantics with its
  own drift; the interpreter is the reference semantics (`INTERPRETER_CONTRACT.md`). It would also need the whole
  compiler in wasm anyway.
- **D. `wasm32-wasip1` with a WASI shim.** Gives a file system and clock shim for free, but every sink would then be
  answered by the shim instead of the capability gate and the simulated world, a second confinement model to audit.
  `wasm32-unknown-unknown` with no imports makes "the module can do nothing but compute" a checkable fact.
- **E. TypeScript through the TypeScript compiler's `transpileModule`.** Correct but about 9 MB for stripping types;
  sucrase is about 200 KB and strips the same erasable syntax. Node's own stripper (amaro, swc in wasm) would be an
  exact match for the oracle but is packaged for node, not the browser; revisit if sucrase and node ever disagree on a
  sample (the `--ts-playground` pass would show it).

## Recommendation

Adopt A for Windjammer and the four-layer sandbox for TypeScript, as implemented in the slice; Rust and Go as opt-in
links. Wire it into the book workflow as below. Keep the playground's semantics defined as "`wj run --interpret
--sandbox --simulate --seed 0 --limits ...`", so it never becomes a third dialect.

## Consequences

- Native builds: unchanged dependency set; `cargo build --lib --no-default-features` compiles again (it did not
  before), which a CI job could hold.
- `crates/windjammer-playground` is a workspace of its own (`[workspace]` in its manifest, its own `Cargo.lock`), so
  the main workspace, its lock file and its gates never build it.
- The interpreter gains three small public entry points (`set_capture_stderr`/`get_err_output`,
  `guard_stack_bytes`, `runner::main_result_code`); wasm32 cfg gates sit in `builtins/{mod,std_extra}.rs`,
  `host_bridge.rs`, `limits.rs`, `engine.rs`, `runner.rs`, and outside the interpreter in `embedded_assets.rs`,
  `stdlib_scanner.rs`, `metadata/crate_metadata.rs` and `caps_core/grant.rs`. Native behaviour is unchanged except
  for the simulation gate fix above. A future host-only dependency or a std call that panics on wasm32 breaks the
  playground; the proposed CI job (build plus parity pass) catches both.
- Book samples that use files or the environment are denied in the playground (exit 126), and the parity pass expects
  exactly that, because native `--sandbox` denies them too.

### CI job (added to `.github/workflows/book.yml`; the commands it runs)

In `.github/workflows/book.yml`, after the book gates:

```sh
rustup target add wasm32-unknown-unknown
cargo build --release --features cli --bin wj --bin wj-docs
wj run --interpret scripts/book/playground_check.wj -- target/release/wj target/release/wj-docs "$RUNNER_TEMP/playground-check"
mdbook build docs/book
```

(Node 22 is set up first: the hosts and the TypeScript pass use type stripping.) The browser tests
(`docs/book/playground/tests/browser.test.mjs`, headless Chrome) are a separate job still to add.

and a "Run" link on each runnable fence (an mdBook preprocessor or a small script that adds
`playground/#lang=wj&code=...` links; the program travels in the URL fragment, which the browser never sends to a
server).

## Slice status

Implemented and measured on this branch:

- `crates/windjammer-playground`: `run(source, Options) -> Outcome`, wasm exports, panic readback; native tests
  (output, fuel, output cap, type error refused with the compiler's diagnostic, panic 101, `process::exit`, files and
  environment denied with 126, seeded clock and random).
- `docs/book/playground/`: the page (editor, language selector, Run, Stop, Share link via URL fragment), the Worker,
  the shared loader, the TypeScript sandbox (generated, hash-pinned CSP), the vendored stripper with licenses, and the
  node hosts for both languages.
- `docs/book/playground/tests/browser.test.mjs`: headless Chrome tests (puppeteer-core 23.11.1, pinned with a lock
  file), all passing: Windjammer hello, diagnostics, fuel stop, file denial; TypeScript run, every escape route of the
  threat model, wall-clock stop with the page responsive, output flood, CSP-only network block, opaque-origin frame,
  forged-message rejection.
- `scripts/book/book_samples.wj --wasm/--wasm-known/--ts-playground` and `scripts/book/playground_check.wj`: 115 of 120
  single-file Windjammer samples equal to native, 5 listed thread gaps; 4 of 4 TypeScript samples equal to node.

Added afterwards: tasks on one thread (above), multi-file programs (`run_project`, `wj_run_project`), the in-memory std
reader, and the CI step in `.github/workflows/book.yml` that builds the wasm and runs `scripts/book/playground_check.wj`
(every sample, files and projects, wasm against native; TypeScript against node) on every change under `src/interpreter`,
`std`, `crates/windjammer-playground` or the book.

Not done: "Run" links on book fences, an editor for several files on the page (the wasm API and the node host take them).

## Ambition ladder

1. (this slice) the playground page, both languages, parity pass, escape tests.
2. "Run" buttons on every book sample, opening the playground with the program; inline output under the fence.
3. Diagnostics underlined in the editor from the JSON spans; `wj explain` text on hover.
4. (done, nested calls) Threads in the browser. Next: true continuations for the schedules that resume a blocked task.
5. (done) Multi-file samples (projects): an in-memory source tree through `load_sources`. Next: a multi-file editor on the page.
6. Startup: the resolved engine with fuel accounting, `wasm-opt -Oz`, streaming compilation.
Kill criteria: if the wasm build needs more than cfg gates at module boundaries (for example a fork of the
interpreter), or parity on the book's samples drops below 95% for a reason other than threads or denied effects,
stop and revisit option C.

## Decision needed from: project owner

1. Accept the playground's definition (`--sandbox --simulate --seed 0 --limits fuel=1e7,output=1MiB`).
2. Accept TypeScript in the browser behind the four layers above (escape tests green), or keep it as an opt-in link.
3. Add the CI job above to the book workflow.
