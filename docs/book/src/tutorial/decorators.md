# Decorators

<!-- feature: decorators -->

A decorator is a line that starts with `@` and precedes an item: a function, struct, enum, impl or trait. It tells the
compiler or a tool something about the item without changing the code inside it. Several decorators can stack, and
arguments are written as you would write them in a call.

```text
@name
@name(args)
```

Rust's `#[...]` attribute syntax is not used; the `@` form is the one spelling.

## `@derive`: standard traits on request

Most standard traits are inferred from how a type is used: if every field can be compared, you can compare. When you want
to be explicit, or to ask for a trait a use does not imply, `@derive` lists them:

```wj
// expect: true false Version { major: 1, minor: 2 }
// expect: Version { major: 0, minor: 0 }
// expect: true Low
@derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)
struct Version {
    major: int,
    minor: int,
}

@derive(Debug, PartialEq)
enum Level {
    Low,
    High(int),
}

fn main() {
    let a = Version { major: 1, minor: 2 }
    let b = Version { major: 1, minor: 10 }
    println("${a < b} ${a == b} ${a:?}")
    let d: Version = Version::default()
    println("${d:?}")
    println("${Level::High(3) == Level::High(3)} ${Level::Low:?}")
}
```

`Copy`, `Serialize` and `Deserialize` are the exceptions. The compiler decides copying from the type, and JSON conversion
needs no derive (see `@json` below), so asking for them is a `WJ0023` diagnostic that tells you to delete the argument:
`wj explain WJ0023`.

## `@json`: shaping JSON

Any struct converts to and from JSON with `std::json`. Decorators adjust the result: `@json(rename = "...")` on a field
renames it, and `@json(allow_unknown)` on the struct accepts fields it does not know:

```wj
// expect: ada 3
// expect: {"userName":"bo","retries":1}
use std::json

@json(allow_unknown)
struct Config {
    @json(rename = "userName")
    name: string,
    retries: int,
}

fn main() {
    let text = "{\"userName\":\"ada\",\"retries\":3,\"extra\":true}"
    let parsed: Result<Config, string> = json.parse_string(text)
    match parsed {
        Ok(c) => println("${c.name} ${c.retries}"),
        Err(e) => println(e),
    }
    let out = json.to_string(Config { name: "bo", retries: 1 }).unwrap_or("")
    println(out)
}
```

## `@test` and `@test_cases`

A function marked `@test` is collected by `wj test` and ignored by `wj run`. `@test_cases` runs one function for a table
of arguments. The [testing chapter](testing.md) shows both.

## `@pure` and `@uses`: saying what a function does

These declare a function's effects. `@pure` means none; `@uses(console)` lists exactly which. The compiler holds the
function to what you declared, and reports `WJ0042` when the body does more:

```sh,verify
$ cat > declared.wj <<'EOF'
use std::fs

@pure
fn parse(s: string) -> int {
    let extra = fs::read_to_string("x").unwrap_or("")
    s.len() + extra.len()
}

fn main() {
    println("${parse("abc")}")
}
EOF
$ wj check declared.wj 2>&1 || true
WJ0042
parse
fs.read
```

They are checked by `wj check`, `wj explain-effects` and capability enforcement; see
[Effects and capabilities](effects.md). `@effect("name")` defines an effect of your own, and `@uses` on an `extern fn`
declares what foreign code does.

## Other decorators

`@deprecated` marks an item that callers should move away from. Decorators the compiler does not know are kept for
tooling and otherwise ignored by the language itself, which is how libraries and tools attach their own metadata:
`@audit("billing")` can be read by a report generator without the compiler needing to understand it.

A decorator the Rust backend also reads, such as `@derive`, `@json` and `@test`, takes effect in compiled programs. The
effect decorators `@pure` and `@uses` are checked by the Windjammer front end; passing them through the compiled path is
an open item in [Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md).
