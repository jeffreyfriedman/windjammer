# The playground and Run links
<!-- feature: playground -->

Under almost every program in this book there is a **Run this in the playground** link. It opens the playground with the
program loaded, and one click runs it in your browser. Nothing is sent to a server: the playground is Windjammer's
interpreter compiled to WebAssembly, served as static files next to these pages.

Because it is the real interpreter, what you see is what `wj run` prints. The book's own tests check this: every sample is
run through the playground's WebAssembly interpreter and through the native one, and the output and exit status must be
equal.

## What a playground run is

A playground run is this command, on the program in the editor:

```text
wj run --interpret --sandbox --simulate --seed 0 --limits fuel=10000000,output=1048576
```

Read the flags as a description of the page:

- `--sandbox`: only the console, time and random numbers are granted. Files, environment, processes and the network are
  refused before the call, with the same message and exit status (126) the command line gives. The
  [effects chapter](effects.md) explains why that is a feature and not a limit.
- `--simulate --seed 0`: the clock is virtual and starts at a fixed time, and random numbers, hash order and the task
  schedule are functions of the seed. The same program prints the same thing every time, which is why a shared link
  always shows the result its author saw.
- `--limits`: a program that loops forever stops at its fuel budget with a diagnostic, and a flood of output is cut off.

## Threads and tasks

Programs that use `thread::spawn`, `scope` or channels (see [Concurrency](concurrency.md)) run in the playground. A
browser page has one thread, so each task runs as a nested call: when the running task has to wait, the scheduler makes
its usual choice and, if that task has not started yet, runs it to completion on top. That covers the book's thread, scope
and channel samples, and they print exactly what the native scheduler prints.

One shape is refused rather than guessed at. A schedule that would have to *resume* a task that is already waiting further
down the stack (three tasks that sleep for 300, 100 and 200 milliseconds, where the second to start wakes first) stops
with an error that names the task, instead of printing an interleaving a real run would not. Run that program with
`wj run` and it behaves as usual. True coroutines in the browser are on the roadmap, to arrive with stack switching in
WebAssembly.

Here is a program in the supported shape. Run it in the playground and with `wj run`; both print the same two lines:

```wj
// expect: total 42
// expect: done
use std::thread

fn work(n: int) -> int {
    n * 2
}

fn main() {
    let a = thread::spawn(move || work(10))
    let b = thread::spawn(move || work(11))
    let total = a.join().unwrap_or(0) + b.join().unwrap_or(0)
    println("total ${total}")
    println("done")
}
```

## Several files

The playground API takes a whole in-memory source tree, which is how the book checks its multi-file samples: a project
with `wj.toml`, `src/main.wj` and modules is run in the browser's interpreter and compared with the native run. The
page's editor holds one file today, so a multi-file project is something you run with `wj run`; a multi-file editor is
on the roadmap.

## What the playground does not do

Anything that needs the outside world is refused by design: files, environment variables, subprocesses, sockets
([Sockets](sockets.md) shows how to run those under the simulated network from the command line), the database module and
Rust crates. A program that needs one prints the capability diagnostic, which tells you the grant to use with `wj run`.

## Share a program

The editor writes your program into the link's fragment (everything after the `#`), so a link carries the code with it and
no server ever sees it. Copy the address from the page's Share button to send someone a program.
