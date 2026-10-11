# Concurrency

ADR-0004 (accepted 2026-10-10) decides the surface: `async f()`/`spawn f()` prefixes and blocks, one handle type `Task<T>` with `.wait()`, structured concurrency through `scope { }`, channels as methods, no `<-` operators, no `go`. **Almost none of this is implemented.** Rules marked [PLANNED] carry an `xfail` example (the example states the intended behaviour; it is expected to fail today and the run fails if it starts passing).

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/spec_check.sh`; each rule's executable example is the code block below it, stored in `tests/spec/concurrency/`.

## spawn

**SPEC-CONC-001** [PLANNED ADR-0004] `spawn f(x)` and `spawn { ... }` run work in parallel and evaluate to a `Task<T>`; `t.wait()` returns the result (`.await` is accepted sugar). Not implemented: `spawn` is not parsed as a keyword today.

<!-- example: tests/spec/concurrency/SPEC-CONC-001.wj -->
<!-- ignore: known divergence Q-013; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
fn work(n: int) -> int { n * 2 }
fn main() {
    let t = spawn work(4)
    println("${t.wait()}")
}
```
Output:
```text
8
```
> **Known divergence:** the implementation does not do this yet; see [Q-013](OPEN_QUESTIONS.md). The example is marked `xfail`.

## async call

**SPEC-CONC-002** [PLANNED ADR-0004] `async f(x)` and `async { ... }` overlap work on the current executor and evaluate to a `Task<T>`. Not implemented as an expression prefix.

<!-- example: tests/spec/concurrency/SPEC-CONC-002.wj -->
<!-- ignore: known divergence Q-013; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
fn work(n: int) -> int { n + 1 }
fn main() {
    let t = async work(4)
    println("${t.wait()}")
}
```
Output:
```text
5
```
> **Known divergence:** the implementation does not do this yet; see [Q-013](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Structured concurrency

**SPEC-CONC-003** [PLANNED ADR-0004] A `spawn` must occur inside a `scope { }` or return a handle that is waited on; an un-waited task is a compile-time error; errors propagate to the enclosing scope. Not implemented: `scope` is not a construct.

<!-- example: tests/spec/concurrency/SPEC-CONC-003.wj -->
<!-- ignore: known divergence Q-013; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
fn work(n: int) -> int { n }
fn main() {
    scope {
        let a = spawn work(1)
        let b = spawn work(2)
        println("${a.wait() + b.wait()}")
    }
}
```
Output:
```text
3
```
> **Known divergence:** the implementation does not do this yet; see [Q-013](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Channels

**SPEC-CONC-004** [PLANNED ADR-0004] Channels are library values with methods `send(x)` and `recv()`; the `<-` operators are removed. `select { }` stays and is designed in the library first. Not implemented in the interpreter (`std::sync::channel` unsupported).

<!-- example: tests/spec/concurrency/SPEC-CONC-004.wj -->
<!-- ignore: known divergence Q-013; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
use std::sync
fn main() {
    let (tx, rx) = sync::channel()
    tx.send(5)
    println("${rx.recv()}")
}
```
Output:
```text
5
```
> **Known divergence:** the implementation does not do this yet; see [Q-013](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Accepted legacy forms

**SPEC-CONC-005** Until the ADR-0004 surface lands, `async fn`, `.await`, `thread { }` and `async { }` blocks are parsed and compile on the Rust backend; the interpreter rejects thread/async blocks as unsupported. They are deprecated by ADR-0004 decision 1-2.

<!-- example: tests/spec/concurrency/SPEC-CONC-005.wj -->
```windjammer
async fn fetch(n: int) -> int { n }
fn main() {
    println("parsed")
}
```
Output:
```text
parsed
```

## Ordering guarantee

**SPEC-CONC-006** Concurrent tasks have no ordering guarantee relative to each other except through `wait()`, channel receive and scope exit; conformance output must not depend on interleaving. (untestable: nondeterministic by definition)

