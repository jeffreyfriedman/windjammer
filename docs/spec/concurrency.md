# Concurrency

ADR-0004 (accepted 2026-10-10) decides the surface: `async f()`/`spawn f()` prefixes and blocks, one handle type `Task<T>` with `.wait()`, structured concurrency through `scope { }`, channels as methods, no `<-` operators, no `thread { }`, no `go`. The interpreter implements all of it on a deterministic cooperative scheduler; the compiled backends follow the stages in [concurrency/PLAN.md](../concurrency/PLAN.md) (the Rust backend refuses tasks with a `compile_error!` until its stage lands).

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/wj/spec_check.wj`; each rule's executable example is the code block below it, stored in `tests/spec/concurrency/`. Programs with longer expected outputs live in `tests/conformance/concurrency/`, the cases every backend must pass.

## spawn

**SPEC-CONC-001** `spawn f(x)` and `spawn { ... }` start a task that may run in parallel and evaluate to a `Task<T>`; `t.wait()` returns the task's value. The callee and the arguments of `spawn f(x)` are evaluated when the task starts; a block sees the variables around it.

<!-- example: tests/spec/concurrency/SPEC-CONC-001.wj -->
```windjammer
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

## async

**SPEC-CONC-002** `async f(x)` and `async { ... }` start a task on the current executor (overlap without parallelism) and evaluate to a `Task<T>`. `t.await` is the same as `t.wait()`.

<!-- example: tests/spec/concurrency/SPEC-CONC-002.wj -->
```windjammer
fn work(n: int) -> int { n + 1 }
fn main() {
    let t = async work(4)
    println("${t.wait()}")
    let name = "world"
    let b = async { "hello ${name}" }
    println("${b.await}")
}
```
Output:
```text
5
hello world
```

## Structured concurrency

**SPEC-CONC-003** `scope { ... }` runs its block, then waits for every task started inside it, including tasks started by those tasks. Its value is the block's value. If a task panics, the scope waits for the others and then re-raises the first panic; `t.wait()` re-raises the panic of `t`.

<!-- example: tests/spec/concurrency/SPEC-CONC-003.wj -->
```windjammer
fn work(n: int) -> int { n }
fn main() {
    scope {
        let a = spawn work(1)
        let b = spawn work(2)
        println("${a.wait() + b.wait()}")
        spawn work(3)
    }
    println("done")
}
```
Output:
```text
3
done
```

**SPEC-CONC-008** Every task is waited on. Outside a `scope { }`, a task used as a statement, bound to `_`, or bound to a name that nothing mentions afterwards is a compile-time error (WJ0190); a function may return a task, and its caller must then wait on it. The fix that `wj check --fix` and `wj fix` apply wraps the rest of the block in `scope { }`.

<!-- example: tests/spec/concurrency/SPEC-CONC-008.wj -->
<!-- should_fail: the rule says this program is rejected -->
```windjammer,should_fail
fn work(n: int) -> int { n }
fn main() {
    spawn work(1)
    println("x")
}
```
Rejected at compile time (WJ0190).

**SPEC-CONC-009** A `scope { }` is a block: `return`, `?` and `break` inside it leave it as from any block, after its tasks have finished.

<!-- example: tests/spec/concurrency/SPEC-CONC-009.wj -->
```windjammer
use std::sync
fn first(tx: sync::Sender<int>) -> string {
    scope {
        spawn { tx.send(1) }
        spawn { tx.send(1) }
        return "early"
    }
}
fn main() {
    let (tx, rx) = sync::channel()
    println(first(tx))
    println("${rx.recv() + rx.recv()}")
}
```
Output:
```text
early
2
```

## Channels

**SPEC-CONC-004** Channels are library values with methods: `tx.send(x)` sends, `rx.recv()` waits for a value and returns it (and panics on a channel whose senders are all gone and which is empty). Values from one sender arrive in the order they were sent. `select { }` stays in the language and is specified with stage 2a of the plan.

<!-- example: tests/spec/concurrency/SPEC-CONC-004.wj -->
```windjammer
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

## Removed forms

**SPEC-CONC-007** `thread { }`, `ch <- v` and `<-ch` are not Windjammer: each is a compile-time error (WJ0191) whose machine-applicable fix writes `spawn { }`, `ch.send(v)` and `ch.recv()`. `x < -1` (with a space) is a comparison.

<!-- example: tests/spec/concurrency/SPEC-CONC-007.wj -->
<!-- should_fail: the rule says this program is rejected -->
```windjammer,should_fail
fn main() {
    thread {
        println("x")
    }
}
```
Rejected at compile time (WJ0191).

**SPEC-CONC-005** `async fn` declarations and `.await` still parse; `async fn` is removed in stage 2b of the plan with a `wj fix` (functions are not coloured: the caller decides with `async f()`).

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

**SPEC-CONC-006** Concurrent tasks have no ordering guarantee relative to each other except through `wait()`, channel receive and scope exit; conformance output must not depend on interleaving. (untestable: nondeterministic by definition; `tests/concurrency_surface_test.rs` runs the conformance programs under several schedules)
