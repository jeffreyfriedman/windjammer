# Recipes

Short, complete answers to everyday tasks. Each program runs as written.

## Sort records by a field

`sort_by` takes a comparison. Compare the field you care about with `cmp`:

```wj
// expect: Linus 28
// expect: Ada 36
// expect: Grace 45
struct Person {
    name: string,
    age: int,
}

fn main() {
    let mut people = [
        Person { name: "Ada", age: 36 },
        Person { name: "Grace", age: 45 },
        Person { name: "Linus", age: 28 },
    ]
    people.sort_by(|a, b| a.age.cmp(b.age))
    for p in people {
        println("${p.name} ${p.age}")
    }
}
```

## Mean, minimum and maximum

```wj
// expect: mean 3.69 min 2.25 max 5
fn main() {
    let scores = [3.5, 4.0, 2.25, 5.0]
    let mut total = 0.0
    let mut lo = scores[0]
    let mut hi = scores[0]
    for s in scores {
        total += s
        if s < lo {
            lo = s
        }
        if s > hi {
            hi = s
        }
    }
    let mean = total / (scores.len() as float)
    println("mean ${mean:.2} min ${lo} max ${hi}")
}
```

## Print a table

Format specs align columns: `<` left, `>` right, followed by a width. `.2` fixes the decimals:

```wj
// expect: item      qty   price
// expect: apple       3    0.50
// expect: kiwi       12    0.25
fn line(name: string, qty: string, price: string) -> string {
    "${name:<8}${qty:>5}${price:>8}"
}

fn main() {
    println(line("item", "qty", "price"))
    println(line("apple", "${3}", "${0.5:.2}"))
    println(line("kiwi", "${12}", "${0.25:.2}"))
}
```

## Retry until it works

A loop with a `Result`, a counter and an early `break`:

```wj
// expect: attempt 1: try again
// expect: attempt 2: try again
// expect: connected after 3
fn flaky(attempt: int) -> Result<string, string> {
    if attempt < 3 {
        Err("try again")
    } else {
        Ok("connected")
    }
}

fn main() {
    let mut attempt: int = 1
    let mut last = "never"
    while attempt <= 5 {
        match flaky(attempt) {
            Ok(v) => {
                last = v
                break
            },
            Err(e) => println("attempt ${attempt}: ${e}"),
        }
        attempt += 1
    }
    println("${last} after ${attempt}")
}
```

## Read numbers from standard input

Read all of standard input as one string with `/dev/stdin`, then go line by line. Reading it this way behaves the same
interpreted and compiled:

```sh,verify
$ cat > sum.wj <<'EOF'
use std::fs

fn main() {
    let text = fs::read_to_string("/dev/stdin").unwrap_or("")
    let mut total = 0
    for line in text.lines() {
        let n = line.trim().parse::<int>().unwrap_or(0)
        total += n
    }
    println("sum ${total}")
}
EOF
$ printf '4\n5\nx\n6\n' | wj run sum.wj
sum 15
```

Lines that are not numbers count as zero here because of `unwrap_or(0)`. Match on the `Result` instead when you want to
report them; see [Errors and Result](../tutorial/errors.md).

## Turn a struct into JSON and back

Any struct converts with `std::json`, with no schema or derive. Parse errors name the path to the problem:

```wj
// expect: {"name":"demo","retries":3,"verbose":false}
// expect: demo 3
use std::json

struct Settings {
    name: string,
    retries: int,
    verbose: bool,
}

fn main() {
    let s = Settings { name: "demo", retries: 3, verbose: false }
    let text = json.to_string(s).unwrap_or("")
    println(text)
    let back: Result<Settings, string> = json.parse_string(text)
    match back {
        Ok(v) => println("${v.name} ${v.retries}"),
        Err(e) => println(e),
    }
}
```

## Count things

`entry` finds the slot for a key; `and_modify` and `or_insert` handle the two cases. Sort the keys before printing,
because a `HashMap` has no fixed order:

```wj
// expect: apple=2
// expect: pear=1
use std::collections::HashMap

fn main() {
    let items: Vec<string> = ["apple", "pear", "apple"]
    let mut counts: HashMap<string, int> = HashMap::new()
    for item in items {
        counts.entry(item).and_modify(|c| *c += 1).or_insert(1)
    }
    let mut keys: Vec<string> = counts.keys().collect()
    keys.sort()
    for k in keys {
        println("${k}=${counts[k]}")
    }
}
```

## Look something up offline

<!-- promise: feature offline-docs -->
`wj docs` searches the standard library, the built-in methods and a library of verified examples without a network:

```sh,verify
$ wj docs hashmap entry
HashMap
```

## Run code only when a file changes

`wj run --watch` re-runs a program on every save, and `wj test --interpret --watch` does the same for tests.

## Test for a palindrome

Filter to letters and digits, lowercase them, then compare from both ends:

```wj
// expect: true
// expect: false
fn is_palindrome(text: string) -> bool {
    let mut cleaned: Vec<char> = []
    for c in text.chars() {
        if c.is_alphanumeric() {
            cleaned.push(c.to_ascii_lowercase())
        }
    }
    let n = cleaned.len()
    let mut i = 0
    while i < n / 2 {
        if cleaned[i] != cleaned[n - 1 - i] {
            return false
        }
        i += 1
    }
    true
}

fn main() {
    println("${is_palindrome("A man, a plan, a canal: Panama")}")
    println("${is_palindrome("windjammer")}")
}
```

## Binary search

Return the position as an option, so "not found" is a value:

```wj
// expect: Some(4) None
fn search(sorted: Vec<int>, target: int) -> int? {
    let mut lo: int = 0
    let mut hi: int = sorted.len() as int - 1
    while lo <= hi {
        let mid = lo + (hi - lo) / 2
        if sorted[mid] == target {
            return Some(mid)
        }
        if sorted[mid] < target {
            lo = mid + 1
        } else {
            hi = mid - 1
        }
    }
    None
}

fn main() {
    let data = [2, 5, 8, 12, 16, 23, 38]
    println("${search(data, 16):?} ${search(data, 7):?}")
}
```

## Parse `key=value` lines

Return a `Result` per line and let the caller decide what to do with the bad ones:

```wj
// expect: name -> demo
// expect: port -> 8080
// expect: expected key=value: broken line
fn parse_line(line: string) -> Result<(string, string), string> {
    let parts: Vec<string> = line.split("=").map(|p| p.trim().to_string()).collect()
    if parts.len() != 2 {
        return Err("expected key=value: ${line}")
    }
    Ok((parts[0], parts[1]))
}

fn main() {
    let text = "name = demo\nport=8080\nbroken line"
    for line in text.lines() {
        match parse_line(line) {
            Ok((k, v)) => println("${k} -> ${v}"),
            Err(e) => println(e),
        }
    }
}
```

## Multiply matrices

A matrix is a list of rows. Nested loops with explicit sizes keep it readable:

```wj
// expect: [[19, 22], [43, 50]]
fn multiply(a: Vec<Vec<int>>, b: Vec<Vec<int>>) -> Vec<Vec<int>> {
    let n = a.len()
    let m = b[0].len()
    let k = b.len()
    let mut out: Vec<Vec<int>> = []
    for i in 0..n {
        let mut row: Vec<int> = []
        for j in 0..m {
            let mut sum = 0
            for t in 0..k {
                sum += a[i][t] * b[t][j]
            }
            row.push(sum)
        }
        out.push(row)
    }
    out
}

fn main() {
    let a = [[1, 2], [3, 4]]
    let b = [[5, 6], [7, 8]]
    println("${multiply(a, b):?}")
}
```

## Fibonacci without recursion

```wj
// expect: 0 1 1 2 3 5 8 13 21 34
fn fib(n: int) -> int {
    let mut a: int = 0
    let mut b: int = 1
    for _ in 0..n {
        let next = a + b
        a = b
        b = next
    }
    a
}

fn main() {
    let mut out: Vec<string> = []
    for i in 0..10 {
        out.push("${fib(i)}")
    }
    println(out.join(" "))
}
```

## Run another program

Starting a program is the `process` effect, so a run must be allowed to. The denial tells you exactly how:

```sh,verify
$ cat > child.wj <<'EOF'
use std::process

fn main() {
    match process::run("echo hello from a child") {
        Ok(out) => println(out.trim()),
        Err(e) => println("failed: ${e}"),
    }
}
EOF
$ wj run child.wj 2>&1 || true
capability denied: process(echo)
$ wj run --allow process=echo child.wj
hello from a child
```

For a project, put `process = ["echo"]` under `[capabilities]` in `wj.toml`.

## Read a setting from the environment, with a default

```sh,verify
$ cat > level.wj <<'EOF'
use std::env

fn main() {
    let level = env::var("LOG_LEVEL").unwrap_or("info")
    println("level: ${level}")
}
EOF
$ wj run --allow env.read=LOG_LEVEL level.wj
level: info
$ LOG_LEVEL=debug wj run --allow env.read=LOG_LEVEL level.wj
level: debug
```
