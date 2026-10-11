# Windjammer

**Write Rust-class programs without writing ownership.** Windjammer is a high-level language that compiles to Rust and
infers borrows, moves and clones for you, then shows its reasoning. The same source runs instantly in an interpreter
and can be emitted as Rust, Go, JavaScript or WebAssembly. Built for people and for coding agents.

[![CI](https://github.com/jeffreyfriedman/windjammer/actions/workflows/test.yml/badge.svg)](https://github.com/jeffreyfriedman/windjammer/actions/workflows/test.yml)
![Version](https://img.shields.io/badge/version-0.50.0-blue)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green.svg)](#license)

Pre-1.0 and experimental. [Status](#status) says what works; [Why Windjammer](docs/WHY.md) says where it does not win yet.

## Three reasons to care

1. **Ownership is inferred, and the decision is visible.** Parameters and `self` become `&`, `&mut` or moves by
   analysis, not annotation. `wj build --no-cargo FILE` writes the Rust it chose, so nothing is hidden. See the
   [side-by-side](#same-program-less-noise).
2. **One source, instant feedback, several targets.** `wj run --interpret FILE` runs with no build step. The same file
   compiles to Rust (primary), and to Go, JavaScript and WebAssembly at lower maturity ([status](#status)).
3. **Built for agents.** JSON diagnostics (`wj check --json`), offline docs search (`wj docs`), an MCP server, an agent
   guide whose examples run in CI, and a benchmark harness. See [For AI agents](#for-ai-agents).

## For AI agents

You have probably seen little Windjammer in training. Do not guess from Rust or Python.

- Read [docs/agent/WINDJAMMER_FOR_AGENTS.md](docs/agent/WINDJAMMER_FOR_AGENTS.md): the loop, idioms and pitfalls; its code runs in CI.
- Machine-readable index: [llms.txt](llms.txt). Everything inline: [llms-full.txt](llms-full.txt). Both are generated and drift-checked.
- Check with `wj check --json FILE`, run with `wj run --interpret FILE`, look things up with `wj docs <terms>`.
- MCP server: [crates/windjammer-mcp](crates/windjammer-mcp/README.md) (check, run, test, format, docs search, explain error, more).
- Changing the compiler itself: [AGENTS.md](AGENTS.md).
- Benchmark: [bench/agent-dx](bench/agent-dx/README.md) holds the same 60 tasks in six languages. Its checked-in report
  grades reference solutions, not LLM runs. There is no published claim yet that agents do better in Windjammer.

## Quickstart

Requires Rust 1.90 or newer. Packaged installs (`brew`, `cargo install`) are not verified here; build from source.

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
$ wj run --interpret hello.wj
Hello, World!
$ wj check --json hello.wj
"success": true
$ wj build --no-cargo hello.wj -o out
$ cat out/hello.rs
println!("Hello, {}!", name);
```

`wj check --json` prints diagnostics for tools; `wj build --no-cargo` shows the Rust the program compiles to.

`wj run hello.wj` (no flag) takes the production path: transpile to Rust, then build with cargo.

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

This helps most in code that is mostly data and calls. When you need precise control over ownership, Rust gives you more.

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

Pre-1.0: language and compiler behaviour change between minor versions. Conformance results per backend, generated from
`tests/conformance_programs/` (causes of each failure: [docs/reference/targets.md](docs/reference/targets.md)):

<!-- wj:gen name=targets_summary -->
| Backend | Conformance programs passing | Known unsupported |
|---|---|---|
| `rust` | 103 | 24 |
| `go` | 124 | 3 |
| `js` | 124 | 3 |
| `ts` | 125 | 2 |
<!-- wj:end -->

| Area | Tier |
|---|---|
| Rust backend and ownership inference | Primary. The only backend that uses the ownership IR solver. Type and borrow errors still come from rustc and are translated ([known gaps](docs/known-gaps.md)). |
| Interpreter (`--interpret`) | Experimental. Accepts a subset of the language: [what it rejects](docs/INTERPRETER.md). |
| Go, JavaScript, TypeScript | Experimental source translation; they do not use the IR. |
| WebAssembly | Implemented but outside the conformance corpus. Experimental. |
| WGSL shaders (`.wjsl`) | Experimental. |
| Formatter for `.wj`, package registry, async surface | Not available yet ([roadmap](ROADMAP.md)). |

Ranked gaps: [docs/known-gaps.md](docs/known-gaps.md). Open design decisions: [ADRs](docs/adr/README.md).

### Performance

No speed claims are made. `benchmarks/rust_parity/` holds Windjammer programs next to hand-written Rust, with a runner
and its last recorded results. Numbers appear here only when a CI job records them in `docs/generated/measured.json`.
See [benchmarks/README.md](benchmarks/README.md).

## Ecosystem

- [windjammer-ecosystem](https://github.com/jeffreyfriedman/windjammer-ecosystem): small packages and apps written in Windjammer (experimental).
- [windjammer-ui](https://github.com/jeffreyfriedman/windjammer-ui): UI components and a reactive runtime (experimental).
- Editors: language server in [crates/windjammer-lsp](crates/windjammer-lsp), VS Code extension in [vscode-extension/](vscode-extension/).

## The `wj` command

`wj new`, `wj run`, `wj build`, `wj check`, `wj test`, `wj docs`, `wj explain` and more: see the generated
[CLI reference](docs/reference/cli.md).

## Roadmap and contributing

[ROADMAP.md](ROADMAP.md) lists work with a concrete plan; [docs/WHY.md](docs/WHY.md) labels goals as goals. To contribute,
read [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md) (tests, architecture rules, and the docs-truth system
that keeps this page honest). Report a compiler bug as a minimal failing test.

Documentation index: [docs/index.md](docs/index.md). Changelog: [CHANGELOG.md](CHANGELOG.md). Security: [SECURITY.md](SECURITY.md).

## License

Dual-licensed under MIT OR Apache-2.0 ([LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE)).
Created by [Jeffrey Friedman](https://github.com/jeffreyfriedman) and contributors.
