# Concurrency

Doing several things at once is how programs stay responsive and use every core. Windjammer's approach is structured
concurrency: work you start belongs to a scope, you get a handle for each piece of work, and results come back through
`wait` or through channels.

The surface described here is the one chosen in [ADR-0004](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/adr/0004-concurrency-surface.md). It runs today in the
interpreter, which schedules tasks deterministically; the compiled Rust backend still takes the earlier `thread` and
`async fn` forms while the new surface is brought across. [Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md) tracks the work, and the
examples on this page are marked so the book's tests know they run in the interpreter.

## Tasks

`spawn f(x)` starts `f(x)` as a task and evaluates to a handle. `wait()` returns the task's result:

<!-- interpreter-only: spawn and wait run in the interpreter; the compiled backends take the earlier thread forms -->
```wj,interpreter_only
// expect: 28
fn work(n: int) -> int {
    n * 2
}

fn main() {
    let a = spawn work(4)
    let b = spawn work(10)
    println("${a.wait() + b.wait()}")
}
```

Both tasks are running before the first `wait`, so slow work overlaps. The handle's type is `Task<T>`, where `T` is the
result type.

## Scopes

`scope { ... }` waits for every task started inside it before it finishes, and a panic in a task is reported by the
scope. Work cannot outlive the code that started it:

<!-- interpreter-only: scope blocks run in the interpreter; the compiled backends do not have them yet -->
```wj,interpreter_only
// expect: 3
fn work(n: int) -> int {
    n
}

fn main() {
    scope {
        let x = spawn work(1)
        let y = spawn work(2)
        println("${x.wait() + y.wait()}")
    }
}
```

## Channels

A channel carries values from one task to another. `send` puts a value in, `recv` takes the next one out, and values
arrive in the order they were sent:

<!-- interpreter-only: std::sync channels run in the interpreter; the compiled form is being aligned -->
```wj,interpreter_only
// expect: 11
use std::sync

fn main() {
    let (tx, rx) = sync::channel()
    tx.send(5)
    tx.send(6)
    let first = rx.recv()
    let second = rx.recv()
    println("${first + second}")
}
```

## Reproducible schedules

Concurrent bugs are the ones that appear once in a hundred runs. The interpreter makes the schedule a parameter. By
default tasks run in first-in first-out order; `--sched-seed N` explores another interleaving, the same one every time
for the same `N`:

```sh,verify
$ cat > tasks.wj <<'EOF'
fn work(n: int) -> int {
    n * 2
}

fn main() {
    let a = spawn work(4)
    let b = spawn work(10)
    println("${a.wait() + b.wait()}")
}
EOF
$ wj run --interpret --sched-seed 3 tasks.wj
28
```

For tests that must find the rare failing schedule, `wj test --interpret --simulate --seeds 1..500` runs each test in a
deterministic world with a virtual clock and seeded scheduling, and prints the first failing seed together with the
command that reproduces it. See [Testing](testing.md).

## Ordering

Two tasks have no ordering relative to each other except through `wait`, a channel receive, or the end of a scope. Write
programs whose output does not depend on how the tasks interleave, and use the schedule seed to check that they do not.
