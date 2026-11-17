# wj.toml

Every project has a `wj.toml` at its root. It names the project, lists its dependencies, tunes the build, and declares
what the program may do.

## A complete example

Two packages side by side, `app` depending on `util` by path. The sections below explain each table.

```sh,verify
$ mkdir -p util/src app/src
$ cat > util/wj.toml <<'EOF'
[package]
name = "util"
version = "0.1.0"
EOF
$ cat > util/src/lib.wj <<'EOF'
pub fn twice(n: int) -> int {
    n * 2
}
EOF
$ cat > app/wj.toml <<'EOF'
[package]
name = "app"
version = "0.2.0"
edition = "2025"

[dependencies]
util = { path = "../util" }

[profile.release]
opt-level = 3
lto = true

[capabilities]
fs.read = ["./**"]
env.read = ["HOME"]

[[bin]]
name = "app"
path = "src/main.wj"
EOF
$ cat > app/src/main.wj <<'EOF'
use util::twice

fn main() {
    println("${twice(21)}")
}
EOF
$ cd app && wj run
42
```

## `[package]`

| Key | Meaning |
|---|---|
| `name` | the package name, also the crate name in compiled output |
| `version` | the package version |
| `edition` | the language edition; `"2025"` for new projects |

## `[dependencies]` and `[dev-dependencies]`

A dependency is a table, or a version string for a Rust crate:

```toml
[dependencies]
util = { path = "../util" }                     # a Windjammer package next to this one
tools = { git = "https://example.com/tools.git", branch = "main" }
strsim = "0.11"                                   # a Rust crate from crates.io
```

`wj add NAME` and `wj remove NAME` edit this table for you. Path and git dependencies are resolved in dependency order;
git dependencies are pinned in `wj.lock` the first time they resolve. Dev dependencies are available to tests only.
Calling Rust crates is covered under Rust interop in the tutorial.

## `[[bin]]`

A project can have several programs. `wj run --bin NAME` runs one. Without a `[[bin]]` table, `main.wj` in the `src` folder is the
program, a `main.wj` inside `cmd/NAME` is a program called `NAME`, and files in a `bin` folder under `src` are programs named after the file.

## `[profile.release]`

Build settings for `wj build --release` and `wj run --release`:

- `opt-level` (default `3`).
- `lto`: when it is not set, link-time optimization is on only if the project depends on another Windjammer package
  (where calls cross crates); a single crate builds without it, which keeps release rebuilds fast. `lto = true` or
  `lto = false` decides explicitly.
- `overflow-checks` (default `true`): integer overflow panics in release builds too, exactly as in the interpreter.
  `overflow-checks = false` is the explicit opt-out; a single hot path can use `wrapping_*`, `checked_*` or
  `saturating_*` instead.
- `codegen-units`.

## `[capabilities]`

What the program may do: one key per effect, each listing the resources it may use. Without this table a run may use
files inside the project, plus time, random and console. See [Effects and capabilities](../tutorial/effects.md).

```toml
[capabilities]
fs.read = ["./**"]
fs.write = ["./out/**"]
net.connect = ["api.example.com"]
env.read = ["HOME"]
process = ["git"]
```

Per-dependency grants go in `[capabilities.dependencies.NAME]`, and `[policy]` forbids a module from performing certain
effects at all.

## Lock files

`wj.lock` pins git dependencies. `.wj-capabilities.lock` records the effects of dependencies you have reviewed
(`wj caps show`, `wj caps approve`). Commit both.
