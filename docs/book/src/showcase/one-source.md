# One source, many targets

<!-- feature: showcase-targets -->

A Windjammer program does not belong to one runtime. The same source builds for Rust, JavaScript, WebAssembly and Go, and its
shader code builds for the GPU. Here is one request handler built two ways, each run for real:

```sh,verify
$ cat > handler.wj <<'EOF'
fn handle(method: string, path: string) -> string {
    match (method, path) {
        ("GET", "/") => "200 welcome",
        ("POST", "/items") => "201 created",
        _ => "404 not found",
    }
}

fn main() {
    println(handle("GET", "/"))
    println(handle("POST", "/items"))
    println(handle("GET", "/missing"))
}
EOF
$ wj build handler.wj --target javascript -o js_out
javascript project is ready
$ cd js_out && node main.mjs
200 welcome
201 created
404 not found
```

The same file, as a Go module:

```sh,verify
$ cat > handler.wj <<'EOF'
fn handle(method: string, path: string) -> string {
    match (method, path) {
        ("GET", "/") => "200 welcome",
        ("POST", "/items") => "201 created",
        _ => "404 not found",
    }
}

fn main() {
    println(handle("GET", "/"))
    println(handle("POST", "/items"))
    println(handle("GET", "/missing"))
}
EOF
$ wj build handler.wj --target go -o go_out
$ cd go_out && go run .
200 welcome
201 created
404 not found
```

Add `--target wasm` for a WebAssembly crate, and the default target for a native binary through Rust. Programs that need a
host feature, such as sockets or files, are written for a host that has it; the conformance suite runs every corpus program
on every backend and lists exactly which pairs do not pass yet on the [compile targets](../reference/targets.md) page. The
[Targets](../tutorial/targets.md) and [Shaders](../tutorial/shaders.md) chapters cover each one.
