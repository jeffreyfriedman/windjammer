# The error model

Windjammer has no exceptions and no null. Absence and failure are values, and a panic is for bugs.

| Rule | Behaviour |
|---|---|
| `Option<T>` | `Some(T)` or `None` |
| `Result<T, E>` | `Ok(T)` or `Err(E)` |
| handling | `match`, `if let`, and combinators: `unwrap_or`, `unwrap_or_else`, `map`, `map_err`, `and_then`, `ok_or`, `is_some`, `is_none`, `is_ok`, `is_err` |
| `e?` on a `Result` | the `Ok` payload, or returns `Err(e)` from the enclosing function |
| `e?` on an `Option` | the `Some` payload, or returns `None` |
| conversion | `?` converts the error with `From` when the function's error type differs |
| panic | prints the message and location to standard error, runs `defer`red statements while unwinding, exits with status 101 |
| `.unwrap()`, `.expect(..)` | panic on `None` or `Err`; both are lint errors in everyday code (`W0002`) |

## Error types and `From`

An error type is any type, usually an enum with a variant per kind of failure. Implementing `From` for a source error makes
`?` convert it for you:

```wj
// expect: ok 5
// expect: parse bad: x
// expect: range 500
enum AppError {
    Parse(string),
    Range(int),
}

impl From<string> for AppError {
    fn from(s: string) -> AppError {
        AppError::Parse(s)
    }
}

fn parse(text: string) -> Result<int, string> {
    text.parse::<int>().map_err(|_| "bad: ${text}")
}

fn load(text: string) -> Result<int, AppError> {
    let n = parse(text)?
    if n > 100 {
        return Err(AppError::Range(n))
    }
    Ok(n)
}

fn show(r: Result<int, AppError>) -> string {
    match r {
        Ok(n) => "ok ${n}",
        Err(AppError::Parse(s)) => "parse ${s}",
        Err(AppError::Range(n)) => "range ${n}",
    }
}

fn main() {
    println(show(load("5")))
    println(show(load("x")))
    println(show(load("500")))
}
```

## Panics

A panic is for a violated assumption: an index out of range, a failed assertion, integer overflow, an explicit
`panic("...")`. Deferred statements still run on the way out:

```sh,verify
$ cat > unwind.wj <<'EOF'
fn main() {
    defer println("cleanup")
    let v = [1, 2, 3]
    println("start")
    println("${v[5]}")
}
EOF
$ wj run unwind.wj 2>&1 || true
start
cleanup
index out of bounds
```

## Exit status

`0` is success, `101` a panic, `N` for `process::exit(N)`, `126` a denied capability, `124` a resource limit under
`--limits`, and `2` for a feature the interpreter does not support when run with `--interpret`.

## Compile-time diagnostics

The compiler reports its errors as data: a stable code, a message, a span and often a fix. `wj check --json` prints them,
`wj explain CODE` explains one, and the [diagnostic codes](diagnostic-codes.md) page lists them all. Type and borrow errors
that come from the Rust compiler are translated into diagnostics on the `.wj` source.

## Option chaining

`a?.b` reads a field of the value inside an optional and yields an optional that is `None` as soon as any link is `None`.
It runs in the interpreter; the Rust backend does not accept it yet, so compiled programs use `match` or `and_then`. See
[Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md).
