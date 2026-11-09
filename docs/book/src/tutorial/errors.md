# Errors and Result

Windjammer has no exceptions and no null. A function that can fail says so in its return type, and the caller has to
deal with both outcomes. Two enums carry the idea:

* `Option<T>` is `Some(value)` or `None`: a value that may be absent.
* `Result<T, E>` is `Ok(value)` or `Err(error)`: an operation that may fail, with the reason.

## Handling a Result

`match` is the most direct way to look inside:

```wj
// expect: Ok(12)
// expect: Err("not a number: zz")
fn parse_port(text: string) -> Result<int, string> {
    text.parse::<int>().map_err(|_| "not a number: ${text}")
}

fn main() {
    println("${parse_port("12"):?}")
    println("${parse_port("zz"):?}")
}
```

`Option` and `Result` print with the debug spec `:?`. For everyday use there are shorter forms:

```wj
// expect: 7
// expect: true 3
fn main() {
    let missing: Option<int> = None
    println("${missing.unwrap_or(7)}")
    let found: Result<int, string> = Ok(3)
    println("${found.is_ok()} ${found.unwrap_or(0)}")
}
```

`.unwrap()` and `.expect("...")` are deliberately absent from everyday code: they turn a recoverable situation into
a crash, and the linter flags them (`W0002`). Reach for `unwrap_or`, `unwrap_or_default`, `match` or `?`.

## The `?` operator

`?` at the end of an expression means: if this is an `Err` (or `None`), return it from the current function right now;
otherwise give me the value inside. A chain of fallible steps reads top to bottom with no nesting:

```wj
// expect: Some(8)
// expect: None
fn first_even(values: Vec<int>) -> Option<int> {
    for v in values {
        if v % 2 == 0 {
            return Some(v)
        }
    }
    None
}

fn doubled_first_even(values: Vec<int>) -> Option<int> {
    let e = first_even(values)?
    Some(e * 2)
}

fn main() {
    println("${doubled_first_even([1, 3, 4]):?}")
    println("${doubled_first_even([1, 3]):?}")
}
```

Combinators such as `map`, `map_err` and `and_then` transform the inside of a `Result` without unpacking it:

```wj
// expect: Ok(3)
fn parse(text: string) -> Result<int, string> {
    text.parse::<int>().map_err(|_| "bad number: ${text}")
}

fn main() {
    let sum = parse("1").and_then(|a| parse("2").map(|b| a + b))
    println("${sum:?}")
}
```

## Your own error types

An error is any type. A plain enum with one variant per kind of failure is usually the right shape, and `match` makes
every case visible at the place you report it. `map_err` converts an error on the way through `?`:

```wj
// expect: port 8080
// expect: missing key port
// expect: bad port: not a number: abc
// expect: bad port: out of range
enum ConfigError {
    Missing(string),
    Invalid(string, string),
}

impl ConfigError {
    fn message(self) -> string {
        match self {
            ConfigError::Missing(key) => "missing key ${key}",
            ConfigError::Invalid(key, why) => "bad ${key}: ${why}",
        }
    }
}

fn parse_port(text: string) -> Result<int, string> {
    text.parse::<int>().map_err(|_| "not a number: ${text}")
}

fn checked_port(text: string) -> Result<int, ConfigError> {
    if text == "" {
        return Err(ConfigError::Missing("port"))
    }
    let n = parse_port(text).map_err(|e| ConfigError::Invalid("port", e))?
    if n < 1 || n > 65535 {
        return Err(ConfigError::Invalid("port", "out of range"))
    }
    Ok(n)
}

fn main() {
    for text in ["8080", "", "abc", "70000"] {
        match checked_port(text) {
            Ok(n) => println("port ${n}"),
            Err(e) => println(e.message()),
        }
    }
}
```

## Handling the error at the top

Decide in `main` what a failure means for the program: print a message and, if the caller of your program needs to know,
end it with `std::process::exit(code)`:

```wj
// expect: could not read the setting: negative
fn load(n: int) -> Result<int, string> {
    if n < 0 {
        return Err("negative")
    }
    Ok(n)
}

fn run() -> Result<int, string> {
    let a = load(2)?
    let b = load(-1)?
    Ok(a + b)
}

fn main() {
    match run() {
        Ok(total) => println("total ${total}"),
        Err(e) => println("could not read the setting: ${e}"),
    }
}
```

How a `main` that itself returns a `Result` reports its error is on the list of things being aligned between the
interpreter and compiled code; see [Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md). Handling the error in `main` with `match`, as above, behaves
the same everywhere.

## Panics

A panic is for a bug, not for an expected failure: an index past the end of a list, a failed `assert`, an explicit
`panic("...")`, integer overflow. The program stops, prints where and why, and exits with status 101:

```sh,verify
$ cat > boom.wj <<'EOF'
fn main() {
    let v = [1, 2, 3]
    println("start")
    println("${v[7]}")
}
EOF
$ wj run boom.wj 2>&1 || true
start
index out of bounds: the len is 3 but the index is 7
```

A good rule: use `Result` when the caller can do something about the failure, and a panic when the program's own
assumptions are wrong.

## Diagnostics are data, too

The compiler reports its own errors the same way you report yours: a stable code, a message, a location and often a fix.
`wj check --json` prints them as structured data, `wj explain CODE` explains any code in full, and the
[diagnostic codes](../reference/diagnostic-codes.md) page lists them all.
