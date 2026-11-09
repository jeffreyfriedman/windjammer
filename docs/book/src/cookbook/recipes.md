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
