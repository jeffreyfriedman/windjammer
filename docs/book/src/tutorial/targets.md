# Targets: Rust, JavaScript, WebAssembly and Go

<!-- feature: compile-targets -->

One Windjammer program can run in several places. `wj build --target` chooses where.

| Target | Output | Use it for |
|---|---|---|
| `rust` (the default) | a Cargo project, then a native binary | servers, tools, anything that wants speed |
| `javascript` | an ES-module project you run with `node` | scripts, front ends, sharing logic with a JavaScript codebase |
| `wasm` | a Rust `cdylib` crate set up for `wasm-bindgen` | code running in a browser or a WebAssembly host |
| `go` | a Go module you run with `go run .` | fitting into a Go codebase |

The [compile targets reference](../reference/targets.md) is generated from the conformance suite: it lists the corpus
programs each backend runs and exactly which are not supported yet.

## The same program, three ways

Here is one program, written once:

```wj
// expect: fib(10) = 55
fn fib(n: int) -> int {
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

fn main() {
    println("fib(10) = ${fib(10)}")
}
```

### JavaScript

`--target javascript` writes a small project: `main.mjs` with your code, and `wj_core.mjs` and `wj_std.mjs` with the
runtime pieces it uses. Run it with Node:

```sh,verify
$ cat > fib.wj <<'EOF'
fn fib(n: int) -> int {
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

fn main() {
    println("fib(10) = ${fib(10)}")
}
EOF
$ wj build fib.wj --target javascript -o js_out
javascript project is ready
$ cd js_out && node main.mjs
fib(10) = 55
```

### Go

`--target go` writes a Go module with the same shape:

```sh,verify
$ cat > fib.wj <<'EOF'
fn fib(n: int) -> int {
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

fn main() {
    println("fib(10) = ${fib(10)}")
}
EOF
$ wj build fib.wj --target go -o go_out
$ cd go_out && go run .
fib(10) = 55
```

### WebAssembly

`--target wasm` writes a Rust crate built as a `cdylib`, with `wasm-bindgen` set up so the compiled module can talk to
JavaScript. Build it with the usual WebAssembly tools, for example `wasm-pack build`:

```sh,verify
$ cat > fib.wj <<'EOF'
fn fib(n: int) -> int {
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

fn main() {
    println("fib(10) = ${fib(10)}")
}
EOF
$ wj build fib.wj --target wasm -o wasm_out
Transpilation complete
$ cat wasm_out/Cargo.toml
crate-type = ["cdylib"]
wasm-bindgen
```

## What carries across

The language is the same on every target: integer overflow is checked, `match` is exhaustive, strings interpolate the
same way. The conformance suite runs every corpus program on every backend and compares the output, which is how the
targets stay in step. Features that depend on the host are the exception: the standard library in the Rust backend is
the fullest, and a program that opens sockets or reads files is written for a host that has them. The support table in
the reference shows the pairs that do not run yet, and [Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md) tracks them.

## Shaders

Windjammer has one more target for the graphics card: shader source compiles to WGSL. That is the subject of the next
chapter, [Shaders](shaders.md).
