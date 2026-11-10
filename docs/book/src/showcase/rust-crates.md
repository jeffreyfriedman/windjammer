# Your Rust crates, as they are

<!-- feature: showcase-interop -->

Windjammer compiles to Rust, so the crates you already trust are one line in `wj.toml` away, and what you wrote in
Windjammer can leave as a plain Rust project whenever you want it to.

Using a crate is a dependency and a call:

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

The call runs in the interpreter through a small host program, and compiled it is an ordinary Rust call. Because the
compiler knows the crate's signatures from a surface file, ownership inference works across the boundary, the call is checked
before it runs, and its effects are part of what `wj explain-effects` reports:

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

The way out is `wj eject`, which writes the Rust project behind any Windjammer project, with path dependencies vendored. The
[Rust interop](../tutorial/rust-interop.md) chapter has the details, including your own Rust code through `extern fn`.
