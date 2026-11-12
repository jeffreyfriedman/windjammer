# Debug a failing run deterministically
<!-- feature: time-travel-debugging -->

A failure you can reproduce is a failure you can fix. Windjammer can record every effect a run performs (the files it
reads, the environment, the clock) and play the run back exactly, as many times as you like. `wj replay` turns that
replay into a debugger: stop at a line, look at the bindings, step forward, and step **back**.

## Record the failing run, then ask it what happened

Here is a small program that sums the numbers in a file, and a file with a bad line in it. `--record run.wjlog` runs
the program normally and writes everything it read from the outside world to `run.wjlog`:

```sh,verify
$ printf '10\n20\nx\n' > numbers.txt
$ cat > sum.wj <<'EOF'
use std::fs

fn total(text: string) -> int {
    let mut sum = 0
    for line in text.lines() {
        let n = line.trim().parse::<int>() ?? -1
        if n < 0 {
            panic("not a number: ${line}")
        }
        sum = sum + n
    }
    sum
}

fn main() {
    let text = fs::read_to_string("numbers.txt") ?? ""
    println("read ${text.len()} bytes")
    let sum = total(text)
    println("sum ${sum}")
}
EOF
$ wj run --record run.wjlog sum.wj 2>&1 || true
read 8 bytes
not a number: x
$ rm numbers.txt
$ printf 'continue\nprint line\nprint sum\n' > why.txt
$ wj replay run.wjlog sum.wj --script why.txt
[step 1] start in main at sum.wj:16:16
read 8 bytes
[step 14] failure in total at sum.wj:8:13: not a number: x
line = "x"
sum = 30
```

`wj replay` takes the log, the program and a short script of commands. `continue` runs to the failure and stops there,
before the stack unwinds, and `print` shows a binding in the failing frame. The replay answers the file read from the
log, which is why it still works after `rm numbers.txt`: a replay never touches the real file system or network.

## Step back

Every replay of the log is the same run, so going back is simply running again to an earlier step. `break` stops when a
line starts, `step` moves forward one statement, `back` moves backward, and `where` shows the call stack:

```sh,verify
$ printf '10\n20\nx\n' > numbers.txt
$ cat > sum.wj <<'EOF'
use std::fs

fn total(text: string) -> int {
    let mut sum = 0
    for line in text.lines() {
        let n = line.trim().parse::<int>() ?? -1
        if n < 0 {
            panic("not a number: ${line}")
        }
        sum = sum + n
    }
    sum
}

fn main() {
    let text = fs::read_to_string("numbers.txt") ?? ""
    println("read ${text.len()} bytes")
    let sum = total(text)
    println("sum ${sum}")
}
EOF
$ wj run --record run.wjlog sum.wj 2>&1 || true
$ printf 'break 10\ncontinue\ncontinue\nprint sum\nback 3\nprint sum\nprint n\nwhere\n' > walk.txt
$ wj replay run.wjlog sum.wj --script walk.txt
[step 8] breakpoint in total at sum.wj:10:9
[step 11] breakpoint in total at sum.wj:10:9
sum = 10
[step 8] back in total at sum.wj:10:9
sum = 0
n = 10
#0 total at sum.wj:10:9
#1 main at sum.wj:18:20
```

## Read a value, not just a name

`print` takes any read-only expression, evaluated in the stopped frame: fields, indexing, arithmetic, comparisons and
the built-in methods that only read, like `len`. It never changes the run. An expression that could have an effect (a
call to a function, `xs.push(9)`, a file read) is refused, and the message says so; the replay is untouched, so the
journal is not consumed and the next command sees the same state:

```sh,verify
$ cat > double.wj <<'EOF'
fn double(x: int) -> int {
    let y = x * 2
    y
}

fn main() {
    let word = "wind"
    let a = double(4)
    println("a = ${a}")
}
EOF
$ printf 'break 9\ncontinue\nprint word.len() * 10\nprint word.to_uppercase()\nprint word.push_str("x")\nprint word.len()\n' | wj replay --simulate double.wj
word.len() * 10 = 40
word.to_uppercase() = "WIND"
with no side effects: the method call `.push_str(..)` (it may have effects) is not evaluated
word.len() = 4
```

## Step over, and finish

`next` runs to the next statement of the current function, stepping over the calls it makes (a breakpoint inside a call
still stops there). `finish` runs until the current function returns, tells you the value it returned, and stops in the
caller:

```sh,verify
$ cat > double.wj <<'EOF'
fn double(x: int) -> int {
    let y = x * 2
    y
}

fn main() {
    let word = "wind"
    let a = double(4)
    println("a = ${a}")
}
EOF
$ printf 'next\nnext\nprint a\nback 2\nstep\nfinish\n' | wj replay --simulate double.wj
[step 2] next in main
[step 5] next in main
a = 8
[step 3] back in double
[step 4] step in double
double returned 8
[step 5] finish in main
```

## Watch a local, stop on a condition

`watch NAME` stops the run whenever the local changes, and tells you the old and the new value (`<unset>` before the
binding exists). `break LINE if EXPR` stops at a line only when the expression is true there. Both are found by
running the same deterministic run again, so they work after `back` as well:

```sh,verify
$ cat > count.wj <<'EOF'
fn main() {
    let mut n = 0
    while n < 3 {
        n = n + 1
    }
    println("n = ${n}")
}
EOF
$ printf 'watch n\ncontinue\ncontinue\ncontinue\ncontinue\nprint n\n' | wj replay --simulate count.wj
watching n in main (now <unset>)
n: <unset> -> 0
n: 0 -> 1
n: 1 -> 2
n: 2 -> 3
n = 3
$ printf 'break 4 if n == 2\ncontinue\nprint n\n' | wj replay --simulate count.wj
[step 5] breakpoint in main
if n == 2
n = 2
```

## Start where the journal says

`--at-effect K` starts the session stopped at the statement that answers the K-th event of the journal, so you do not
have to find it by stepping. The recording also stores the capability grants it ran with (`--allow`), so the replay
needs no flags:

```sh,verify
$ printf 'alpha\n' > a.txt
$ printf 'beta\n' > b.txt
$ cat > two.wj <<'EOF'
use std::fs

fn first_line(path: string) -> string {
    let text = fs::read_to_string(path) ?? ""
    text.trim()
}

fn main() {
    let a = first_line("a.txt")
    let b = first_line("b.txt")
    println("${a} ${b}")
}
EOF
$ wj run --record run.wjlog two.wj
alpha beta
$ rm a.txt b.txt
$ printf 'print path\nwhere\n' | wj replay --at-effect 2 run.wjlog two.wj
effect in first_line at two.wj:4:
path = "b.txt"
#1 main at two.wj:10:
```

```sh,verify
$ cat > greet.wj <<'EOF'
use std::env

fn main() {
    let v = env::var("WJ_GREETING").unwrap_or("none")
    println("greeting: ${v}")
}
EOF
$ WJ_GREETING=hello wj run --allow env.read=WJ_GREETING --record run.wjlog greet.wj
greeting: hello
$ grep -o '"grants":[^]]*]' run.wjlog
"grants":["env.read=WJ_GREETING"]
$ printf 'break 5\ncontinue\nprint v\n' | wj replay run.wjlog greet.wj
v = "hello"
```

A log written by an older `wj` (before the header carried grants) is refused with a message that says to record the run
again.

## Seeded simulation

A run in the simulated world (`wj run --simulate --seed N`) is a function of its seed, so `wj replay --simulate` debugs
it with no log at all. Here seed 6 rolls a six on the third roll:

```sh,verify
$ cat > dice.wj <<'EOF'
use std::random

fn roll() -> int {
    random::range(1, 7)
}

fn main() {
    let mut total = 0
    for i in 0..5 {
        let r = roll()
        total = total + r
        println("roll ${i}: ${r}")
        if r == 6 {
            panic("rolled a six")
        }
    }
    println("total ${total}")
}
EOF
$ wj run --simulate --seed 6 dice.wj 2>&1 || true
roll 2: 6
rolled a six
$ printf 'continue\nprint total\nprint i\n' | wj replay --simulate --seed 6 dice.wj
[step 18] failure in main at dice.wj:14:13: rolled a six
total = 14
i = 2
```

## For agents: JSON events

`--json` prints one JSON object per line (`stopped`, `value`, `returned`, `watching`, `locals`, `stack`, `output`, `exited` and `error`), so a
script or an agent can drive a whole debugging session in one call and read the answers back. Two commands are enough
to reproduce a failure and read the value that caused it:

```sh,verify
$ cat > dice.wj <<'EOF'
use std::random

fn main() {
    let mut total = 0
    for i in 0..5 {
        let r = random::range(1, 7)
        total = total + r
        if r == 6 {
            panic("rolled a six")
        }
    }
    println("total ${total}")
}
EOF
$ printf 'continue\nprint r\n' | wj replay --json --simulate --seed 6 dice.wj
"reason":"failure"
{"event":"value","name":"r","value":"6"}
```

The full command set is `break LINE [if EXPR]`, `continue`, `step [N]`, `next`, `finish`, `back [N]`, `print EXPR`,
`watch NAME`, `locals` and `where`. The design, and exactly what a replay reproduces, is in ADR-0035.
