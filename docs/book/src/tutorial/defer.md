# Defer and resource cleanup

<!-- feature: defer -->

`defer` schedules a statement to run when the surrounding function exits, by any route: reaching the end, an early
`return`, or a panic that unwinds through it. Put the cleanup next to the thing that needs cleaning and it cannot be
forgotten.

## Basics

Deferred statements run in reverse order, last in first out, after the function body finishes:

```wj
// expect: ok run
// expect:   working
// expect:   finished
// expect:   cleanup B
// expect:   cleanup A
// expect: result 1
// expect: early return
// expect:   working
// expect:   cleanup B
// expect:   cleanup A
// expect: result -1
fn work(fail: bool) -> int {
    defer println("  cleanup A")
    defer println("  cleanup B")
    println("  working")
    if fail {
        return -1
    }
    println("  finished")
    1
}

fn main() {
    println("ok run")
    let a = work(false)
    println("result ${a}")
    println("early return")
    let b = work(true)
    println("result ${b}")
}
```

The early `return` still ran both cleanups, in the opposite order to how they were declared. That mirrors how resources
nest: whatever you acquired last you release first.

## Defer and panics

A panic stops the program, but deferred statements of the frames it unwinds through still run:

```sh,verify
$ cat > unwind.wj <<'EOF'
fn main() {
    defer println("cleanup")
    println("start")
    panic("boom")
}
EOF
$ wj run unwind.wj 2>&1 || true
start
cleanup
boom
```

## Types that clean up after themselves

A type can release what it owns by implementing `Drop`. Its `drop` runs when the last owner goes away: locals in reverse
declaration order at the end of their block. Use `Drop` when a value owns the resource, and `defer` when a function does:

```wj
// expect: using first and second
// expect: deferred line
// expect: release second
// expect: release first
struct Guard {
    name: string,
}

impl Drop for Guard {
    fn drop(self) {
        println("release ${self.name}")
    }
}

fn main() {
    let first = Guard { name: "first" }
    let second = Guard { name: "second" }
    defer println("deferred line")
    println("using ${first.name} and ${second.name}")
}
```

The deferred line runs first because the function body ends, and the locals are released after it.

## Where this is useful

Closing a file, releasing a lock, removing a temporary directory, printing a summary after a run, restoring a setting you
changed: each is one `defer` right after the step that needs undoing.

## In loops

A `defer` inside a loop body is meant to run at the end of each pass. The Rust backend does this; the interpreter does not
run it yet, so keep deferred statements at function level when you want them to behave the same in both modes. The
difference is recorded in [Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md).
