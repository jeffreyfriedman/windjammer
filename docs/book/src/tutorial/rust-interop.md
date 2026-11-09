# Rust interop

Windjammer compiles to Rust, so the whole crates.io ecosystem is within reach. You list a crate in `wj.toml` like any
dependency and call it by name. A small interface file, the **signature surface**, tells the compiler what the crate's
functions look like in Windjammer types, which is how ownership inference, type checking and effect tracking work across
the boundary.

## Calling a crate

Add the crate under `[dependencies]` and call its functions with `crate::function(...)`:

```sh,verify
$ mkdir -p fuzzy/src
$ cat > fuzzy/wj.toml <<'EOF'
[package]
name = "fuzzy"
version = "0.1.0"

[dependencies]
strsim = "0.11"
EOF
$ cat > fuzzy/src/main.wj <<'EOF'
fn main() {
    let d = strsim::levenshtein("kitten", "sitting")
    println("distance: ${d}")
}
EOF
$ cd fuzzy && wj run
distance: 3
```

`wj run` uses the interpreter, which talks to the crate through a small host program it builds once and caches. The
same source compiles with `wj build`, where the call is an ordinary Rust call.

## Signature surfaces

`wj rust-surface` reads the crate's source and prints what it can express in Windjammer: free functions over numbers,
strings, lists, options and results. Pure functions are marked `@pure`:

```sh,verify
$ mkdir -p fuzzy/src
$ cat > fuzzy/wj.toml <<'EOF'
[package]
name = "fuzzy"
version = "0.1.0"

[dependencies]
strsim = "0.11"
EOF
$ cd fuzzy && wj rust-surface strsim
pub fn levenshtein(a: string, b: string) -> usize
```

The compiler generates a surface for every crate you depend on, so most projects never write one. When you want to pin
the exact functions your project may call, write an editable copy into the project:

```sh,verify
$ mkdir -p fuzzy/src
$ cat > fuzzy/wj.toml <<'EOF'
[package]
name = "fuzzy"
version = "0.1.0"

[dependencies]
strsim = "0.11"
EOF
$ cd fuzzy && wj rust-surface strsim --write --fn levenshtein
wrote
$ cat fuzzy/rust/strsim.wji
@pure
pub fn levenshtein(a: string, b: string) -> usize
```

A hand-written `rust/strsim.wji` is the source of truth and wins over the generated one. It holds body-less function
declarations in Windjammer types, where `string` parameters are passed as Rust `&str` and explicit widths such as
`usize` mean exactly that Rust type. Commit it, and every call to the crate is checked against it: a misspelled
function name, a wrong argument count or a wrong literal type is a diagnostic at the call, not a rustc error in
generated code.

## Effects of crate calls

A call into a crate has an effect like any other. A `@pure` declaration in the surface says the function does nothing
to the outside world. A function declared `@uses(net.connect)` is checked against your `[capabilities]`, and a function
with no declaration performs `ffi(<crate>)`, which a project grants by name:

```toml
[capabilities]
ffi = ["strsim"]
```

The authority of every crate is visible in `wj.toml`; there is no setting that grants all of them at once. See
[Effects and capabilities](effects.md).

## Your own Rust code

For Rust you wrote yourself, declare the function in Windjammer with `extern fn` and keep the implementation in a Rust
file next to the manifest. The interpreter compiles that Rust once into a cached library and calls it; `wj build` links
it. Use `@pure` or `@uses(...)` on the declaration to say what it does, or leave it undeclared and it counts as `ffi`.
An `extern fn` call crosses the C ABI, so integers, floats, `bool` and `string` are the types that pass through.

## Leaving Windjammer

`wj eject` writes the Rust project that `wj build` produces, with your path dependencies vendored, as a standalone Cargo
project. Nothing about it depends on Windjammer at run time. It is the exit door, and it is why adopting Windjammer for
part of a Rust codebase is a low-risk move.
