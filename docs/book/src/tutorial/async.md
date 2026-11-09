# Async and await

<!-- feature: async-await -->

Some work waits: for a file, a network reply, a timer. Windjammer has three ways to overlap that waiting with other
work, and this chapter says plainly what each does today in the interpreter and in compiled code.

| Form | Interpreter | Compiled (Rust) |
|---|---|---|
| `async f(x)` and `spawn f(x)` giving a task, then `.wait()` | runs | not yet |
| `async fn` declarations and `.await` | parses and runs | declarations compile; `.await` inside an `async fn` is not accepted yet |
| `thread::spawn(move \|\| ...)` and `.join()` | runs | runs |

The first row is the surface described in [ADR-0004](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/adr/0004-concurrency-surface.md), the same one the
[concurrency chapter](concurrency.md) introduces. [Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md) tracks bringing each row to
both columns.

## Tasks that overlap

`async f(x)` starts `f(x)` as a task on the current executor, and `spawn f(x)` starts it as a parallel task. Both give a
handle whose `wait()` returns the result:

<!-- interpreter-only: async and spawn tasks run in the interpreter; the compiled backends do not have task expressions yet -->
```wj,interpreter_only
// expect: 16
fn work(n: int) -> int {
    n + 1
}

fn main() {
    let a = async work(4)
    let b = spawn work(10)
    println("${a.wait() + b.wait()}")
}
```

`.await` is accepted as another spelling of `wait()` where an `async fn` is awaited.

## Declaring async functions

An `async fn` returns a value that is produced later. Declaring one is accepted everywhere, so libraries can publish
their asynchronous API now:

```wj
// expect: declared
async fn fetch(n: int) -> int {
    n * 2
}

fn main() {
    println("declared")
}
```

## Threads, in both modes

For work you want in parallel in compiled programs today, use a thread. `thread::spawn` takes a closure and returns a
handle; `join()` waits for it and returns the closure's result:

```wj
// expect: 25
use std::thread

fn square(n: int) -> int {
    n * n
}

fn main() {
    let a = thread::spawn(move || square(3))
    let b = thread::spawn(move || square(4))
    println("${a.join().unwrap_or(0) + b.join().unwrap_or(0)}")
}
```

Threads, tasks and channels follow the same rules: results come back through `join`, `wait` or a channel, and two pieces
of work have no ordering relative to each other except through those. In the interpreter, `--sched-seed N` replays one
particular interleaving every time, which makes a rare race into a test case.

## Which to use

* Writing and testing concurrent logic: tasks and `scope` in the interpreter, with `--sched-seed` to explore schedules.
* Shipping a compiled program that needs parallelism now: `thread::spawn` and channels from `std::sync`.
* Publishing an asynchronous API: `async fn`.
