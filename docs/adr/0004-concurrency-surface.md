# ADR-0004: Concurrency surface

- **Status:** Accepted (2026-10-10) — see "Decision" below
- **Date:** 2026-10-10
- **Related:** [WJ-CONC-01](../proposals/rfcs/WJ-CONC-01-async-concurrency.md), [design/ASYNC_EXECUTION_MODEL.md](../design/ASYNC_EXECUTION_MODEL.md), [CONCURRENCY_ARCHITECTURE.md](../CONCURRENCY_ARCHITECTURE.md), `std/sync.wj`, `std/async.wj`

## Context
Several incompatible concurrency stories exist at once:
- **Docs/README (before this change):** Go-style `go { }` and `chan::new()`. There is no `go` or `chan` token in
  `src/lexer/identifier_lexing.rs`; the claim was false.
- **Lexer/parser reality:** keywords `thread`, `async`, `await`, `defer`, plus Go-style channel operators: `ch <- value` (send, `Expression::ChannelSend`) and `<-ch` (receive, `ChannelRecv`), lowered by the Rust and JavaScript generators. There is no `select`. `thread { ... }` and `async { ... }` are
  statements (`statement_parser.rs`); `async fn` is parsed (`item_parser.rs`). Rust codegen emits `std::thread::spawn`
  / `tokio::spawn(async move { ... })` (`block_generation.rs`, `async_generation.rs`).
- **WJ-CONC-01 / CHANGELOG 0.50.0:** caller-controlled `async expr` / `spawn expr` with `AsyncCall`/`SpawnCall`
  AST nodes and an `ExecutionConstraint` solver. The AST nodes exist, but nothing in `src/parser` constructs them;
  `docs/windjammer.ebnf` lists the syntax anyway. The `ExecutionValidator` therefore has no real input from source.
- **Library:** `std/sync.wj` (channels, shared cells) and `std/async.wj` (a `spawn` stub that returns its argument).
- **Design docs:** ASYNC_EXECUTION_MODEL.md says "APPROVED - Implementation Pending, v2.0 breaking change" and
  deprecates `async fn`, yet `async fn` is the only path that parses today.

## Options
### A. Implement WJ-CONC-01 as the single surface
`async expr` and `spawn expr` (parser work), functions are plain, `thread {}` kept as a structured-parallelism block,
channels via `std::sync`. `async fn` is deprecated with a migration lint, then removed.
- Pros: one story; matches the design and the solver investment. Cons: parser, three backends, runtime story (tokio vs JS promises vs goroutines).

### B. Keep today's surface (`async fn`/`await`/`thread {}`) and drop WJ-CONC-01
- Pros: nothing to build. Cons: function colouring stays; discards the RFC and the solver code; no caller control.

### C. Add Go-style `go {}` and make `<-` channels the primary primitive
- Pros: matches the original pitch. Cons: a third mechanism; needs runtime scheduler semantics on Rust; conflicts with A.

## Recommendation
Option A, staged: (1) parse `async expr`/`spawn expr` so the solver gets real input, (2) keep `async fn` working with a
deprecation lint, (3) define channel and `select` in `std::sync` (library, not syntax), (4) decide per backend in ADR-0007.
Do not add `go`. Decide separately whether the existing `<-` operators stay (as sugar over `std::sync` channels) or are removed. Until step 1 lands, docs must not advertise `async expr`/`spawn expr` as shipped.

## Consequences
- README and EBNF stop advertising `go`/`chan`; EBNF notes which constructs are parsed today.
- `select` is unresolved by this ADR; it needs a library design before any syntax. The fate of the `<-` operators is part of the owner's decision.
- Fate of `async fn`: deprecated-but-supported until A step 1 and a migration path ship; removal is a separate, versioned breaking change.

## Decision needed from: project owner
Choose A, B or C; and state whether `async fn` is to be removed (breaking) or kept permanently as sugar.


## Decision (2026-10-10, project owner)
Implement WJ-CONC-01 with these amendments:
1. **Two call prefixes and two blocks:** `async f()` / `async { }` (overlap on the current executor) and
   `spawn f()` / `spawn { }` (run in parallel). `thread { }` is dropped (alias during migration only).
2. **One handle type:** `Task<T>` replaces `Future<T>` / `JoinHandle<T>`; `.wait()` is the single way to get the
   result (`.await` accepted as sugar).
3. **Structured concurrency:** `spawn` must occur inside a `scope { }` block or return a handle that must be
   waited on (a dropped, un-waited task is a compile error/lint). Errors propagate to the scope.
4. **Channels are methods:** `ch.send(x)` / `ch.recv()`; the `<-` operators are removed (they are ambiguous with
   `<` and unary minus). `select { }` stays.
5. **Variants on demand:** for the Rust backend the compiler generates the async variant of a function only when a
   call site needs it (transitive, cached). A call mode the compiler cannot satisfy is an error naming the cause
   ("`f` cannot run async because it calls extern X"). Blocking calls on JS are explicitly unsupported.
6. **Order of work:** Rust backend first; Go/JS follow once the conformance harness covers async and spawn.
   Supervisors (Erlang-style) are deferred.
