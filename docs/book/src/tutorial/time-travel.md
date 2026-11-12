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

`--json` prints one JSON object per line (`stopped`, `value`, `locals`, `stack`, `output`, `exited` and `error`), so a
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

The full command set is `break LINE`, `continue`, `step [N]`, `back [N]`, `print NAME`, `locals` and `where`. The
design, and exactly what a replay reproduces, is in ADR-0035.
