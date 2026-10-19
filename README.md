# Windjammer

**Write it like a script. Ship it like Rust.**

*80% of Rust's power with 20% of its complexity.* 

Windjammer is a clean, expressive language that compiles to
Rust, with ergonomics as its first priority and performance close behind. You write plain values and functions; the
compiler works out the borrows, moves and clones. Iterate instantly in the interpreter, then ship native Rust, or
target Go, JavaScript and WebAssembly from the same code. Designed for developers and their coding agents alike.

[![CI](https://github.com/jeffreyfriedman/windjammer/actions/workflows/test.yml/badge.svg)](https://github.com/jeffreyfriedman/windjammer/actions/workflows/test.yml)
![Version](https://img.shields.io/badge/version-0.50.0-blue)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green.svg)](#license)

## Why Windjammer

1. **Ownership, handled for you.** Parameters and `self` become `&`, `&mut` or moves automatically, and you can always
   see what was chosen: `wj build --no-cargo FILE` writes out the Rust. See the [side-by-side](#same-program-less-noise).
2. **Instant feedback, native output.** `wj run` starts a whole multi-module project in milliseconds, like `go run`:
   it interprets by default and switches to the compiled path on its own when a program needs it. `wj build` and
   `wj run --release` compile the same source to native Rust, with Go, JavaScript and WebAssembly backends alongside.
3. **Made for agents.** Structured JSON diagnostics, offline docs search, an MCP server and a compact agent guide give
   coding agents a fast, reliable write-check-run loop. See [For AI agents](#for-ai-agents).

## For AI agents

Windjammer is young, so start from its own docs rather than Rust or Python habits:

- Read [docs/agent/WINDJAMMER_FOR_AGENTS.md](docs/agent/WINDJAMMER_FOR_AGENTS.md): the loop, idioms and pitfalls; its code runs in CI.
- Machine-readable index: [llms.txt](llms.txt). Everything inline: [llms-full.txt](llms-full.txt). Both are generated and drift-checked.
- Check with `wj check --json FILE`, run with `wj run FILE` (or `wj run` inside a project), look things up with `wj docs <terms>`.
- MCP server: [crates/windjammer-mcp](crates/windjammer-mcp/README.md) (check, run, test, format, docs search, explain error, more).
- Changing the compiler itself: [AGENTS.md](AGENTS.md).
- Benchmark: [bench/agent-dx](bench/agent-dx/README.md) runs the same task suite across six languages so agent results
  can be compared directly.

## Quickstart

Requires Rust 1.90 or newer.

<!-- ignore: builds the compiler itself; CI builds wj before running the verified fences below -->
```bash,ignore
git clone https://github.com/jeffreyfriedman/windjammer.git
cd windjammer && cargo build --release
export PATH="$PWD/target/release:$PATH"
```

Write a program and run it with no build step:

```sh,verify
$ wj --version
wj
$ cat > hello.wj <<'EOF'
fn main() {
    let name = "World"
    println("Hello, ${name}!")
}
EOF
$ wj run hello.wj
Hello, World!
$ wj check --json hello.wj
{
  "schema_version": 2,
  "windjammer_version": "0.50.0",
  "success": true,
  "diagnostics": []
}
$ wj build --no-cargo hello.wj -o out
$ cat out/hello.rs
println!("Hello, {}!", name);
```

`wj check --json` prints diagnostics for tools; `wj build --no-cargo` shows the Rust the program compiles to.

`wj run` interprets: no build step, whole projects included (`wj run` in any directory of a project finds its
`wj.toml`; `--bin NAME` picks a binary; `--watch` re-runs on save). `wj run --release` and `wj build` take the
production path: transpile to Rust, then build with cargo. Details: [docs/INTERPRETER.md](docs/INTERPRETER.md).

## Same program, less noise

A playlist with one mutating and two reading functions. Windjammer has no `&`, `&mut`, `.to_string()` or `format!` here;
the compiler infers them. The generated Rust has exactly the signatures `fn add_song(list: &mut Playlist, song: String)`,
`fn count(list: &Playlist)` and `fn describe(list: &Playlist)`. Both programs below are compiled and compared by
[tests/readme_samples_test.rs](tests/readme_samples_test.rs); each prints `Road trip: Africa, Hey Jude` then `2 songs`.

```windjammer
struct Playlist {
    name: string,
    songs: Vec<string>,
}

fn add_song(list: Playlist, song: string) {
    list.songs.push(song)
}

fn count(list: Playlist) -> int {
    list.songs.len()
}

fn describe(list: Playlist) -> string {
    "${list.name}: ${list.songs.join(", ")}"
}

fn main() {
    let mut list = Playlist { name: "Road trip", songs: [] }
    add_song(list, "Africa")
    add_song(list, "Hey Jude")
    println(describe(list))
    println("${count(list)} songs")
}
```

```rust
struct Playlist {
    name: String,
    songs: Vec<String>,
}

fn add_song(list: &mut Playlist, song: &str) {
    list.songs.push(song.to_string());
}

fn count(list: &Playlist) -> usize {
    list.songs.len()
}

fn describe(list: &Playlist) -> String {
    format!("{}: {}", list.name, list.songs.join(", "))
}

fn main() {
    let mut list = Playlist { name: "Road trip".to_string(), songs: vec![] };
    add_song(&mut list, "Africa");
    add_song(&mut list, "Hey Jude");
    println!("{}", describe(&list));
    println!("{} songs", count(&list));
}
```

You keep Rust's performance and safety; the ceremony goes away.

## Tour

Every snippet is executed in CI by the interpreter and its output checked.

String interpolation with format specs, and `if` as an expression:

```windjammer
// expect: Ada is 36, pi=3.14, [   42], small
fn size(n: int) -> string {
    if n < 10 { "small" } else { "large" }
}

fn main() {
    let name = "Ada"
    let age = 36
    println("${name} is ${age}, pi=${3.14159:.2}, [${42:>5}], ${size(3)}")
}
```

Pipe operator (`x |> f(a)` is `f(x, a)`):

```windjammer
// expect: 16
fn add(a: int, b: int) -> int { a + b }
fn square(a: int) -> int { a * a }

fn main() {
    let r = 1 |> add(1) |> add(2) |> square
    println("${r}")
}
```

Enums and pattern matching with guards:

```windjammer
// expect: circle 12
// expect: big
enum Shape {
    Circle(int),
    Rect(int, int),
}

fn describe(s: Shape) -> string {
    match s {
        Shape::Circle(r) => "circle ${r * r * 3}",
        Shape::Rect(w, h) if w * h > 100 => "big",
        Shape::Rect(w, h) => "rect ${w * h}",
    }
}

fn main() {
    println(describe(Shape::Circle(2)))
    println(describe(Shape::Rect(20, 20)))
}
```

Traits (derives are inferred; you never write `#[derive]`):

```windjammer
// expect: Circle r=2
trait Named {
    fn label(self) -> string
}

struct Circle { r: int }

impl Named for Circle {
    fn label(self) -> string { "Circle r=${self.r}" }
}

fn main() {
    println(Circle { r: 2 }.label())
}
```

Tests live in `*_test.wj` files and run without a build:

```sh,verify
$ cat > math_test.wj <<'EOF'
fn double(n: int) -> int { n * 2 }

@test
fn doubles_numbers() {
    assert_eq(double(21), 42)
}
EOF
$ wj test --interpret .
test result: ok. 1 passed; 0 failed
```

More: [agent guide](docs/agent/WINDJAMMER_FOR_AGENTS.md), [grammar](docs/windjammer.ebnf),
[keywords](docs/reference/keywords.md), [standard library](docs/reference/stdlib.md), and cheatsheets from
[Rust](docs/agent/cheatsheet_from_rust.md), [Go](docs/agent/cheatsheet_from_go.md),
[Python](docs/agent/cheatsheet_from_python.md) and [TypeScript](docs/agent/cheatsheet_from_typescript.md).

## Status

Windjammer is pre-1.0 and moving quickly. Every backend runs the shared conformance suite in
`tests/conformance_programs/`; per-program details are in [docs/reference/targets.md](docs/reference/targets.md):

<!-- wj:gen name=targets_summary -->
| Backend | Conformance programs passing | Known unsupported |
|---|---|---|
| `rust` | 125 | 29 |
| `go` | 154 | 0 |
| `js` | 154 | 0 |
<!-- wj:end -->

| Area | Tier |
|---|---|
| Rust backend and ownership inference | Primary target |
| Interpreter (`wj run`, default dev path) | Preview ([details](docs/INTERPRETER.md)) |
| Go, JavaScript, TypeScript | Preview |
| WebAssembly | Preview |
| WGSL shaders (`.wjsl`) | Preview |
| Package registry, async surface | On the [roadmap](ROADMAP.md) |

Design decisions are recorded as [ADRs](docs/adr/README.md); current work is tracked in [docs/known-gaps.md](docs/known-gaps.md).

### Performance

Windjammer compiles to ordinary Rust, so it builds on the same optimizer and runtime. A benchmark suite comparing
Windjammer with hand-written Rust lives in `benchmarks/rust_parity/`; published results will appear here as CI records
them. See [benchmarks/README.md](benchmarks/README.md).

## Ecosystem

- [windjammer-ecosystem](https://github.com/jeffreyfriedman/windjammer-ecosystem): packages and apps written in Windjammer.
- [windjammer-ui](https://github.com/jeffreyfriedman/windjammer-ui): a reactive UI framework for Windjammer.
- Editors: language server in [crates/windjammer-lsp](crates/windjammer-lsp), VS Code extension in [vscode-extension/](vscode-extension/).

## The `wj` command

`wj new`, `wj run`, `wj build`, `wj check`, `wj test`, `wj docs`, `wj explain` and more: see the generated
[CLI reference](docs/reference/cli.md).

## Roadmap and contributing

See [ROADMAP.md](ROADMAP.md) for what's next and [docs/WHY.md](docs/WHY.md) for how Windjammer compares with other
languages. Contributions are welcome: start with [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md).
Bug reports with a small failing example are the fastest to fix.

Documentation index: [docs/index.md](docs/index.md). Changelog: [CHANGELOG.md](CHANGELOG.md). Security: [SECURITY.md](SECURITY.md).

## License

Dual-licensed under MIT OR Apache-2.0 ([LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE)).
Created by [Jeffrey Friedman](https://github.com/jeffreyfriedman) and contributors.
