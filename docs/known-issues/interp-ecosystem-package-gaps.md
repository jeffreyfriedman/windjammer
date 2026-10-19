# `wj test --interpret` / `wj run`: what stops the ecosystem packages and apps from running interpreted

Measured on the 38 packages and 16 apps of `windjammer-ecosystem` (compiled `wj test`: 49 green):

1. **Windjammer packages declared as `{ path = "../wj-x/build" }` are treated as Rust crates** ("not interpreted yet: call
   into Rust crate dependency `wj_validate` (`wj_validate::require_nonempty`) [deps.rust_crate]"). The `build/` path is the
   transpiled output; the interpreter needs the package source (`path = "../wj-x"`), or to recognize a `build/` path whose
   parent has a `wj.toml` and load that source. This is the cause of most failing apps and of `wj run` falling back to cargo.
2. **Module-qualified standard-library calls that exist compiled are missing**: `regex.find_all`, `sync.unbounded`,
   `sync.sync_channel`, `url.url_decode`/`url_encode`, `encoding.base64_encode_string`/`base64_decode_string`,
   `hex_encode`/`hex_decode`, `regex.escape`, `mime.APPLICATION_JSON` (message: "no method named X found for value of type fn").
3. **`use` inside a function body** is not bound: `fn f() { use std::strings  strings.len("abc") }` -> "cannot find value
   `strings`" (the ecosystem tests import helpers this way: `use crate::is_rectangular` inside a `@test` fn).
4. Apps whose `adapters` module wraps `std::http::server_serve` report `unresolved import crate::adapters::serve (E0432)`
   when the whole test set is interpreted (a plain `pub use` re-export of a module that the interpreter can load works).
5. `std::env::var(..)` as a path expression inside some modules: "cannot find value `std` in this scope".

Status (second sweep): all five are fixed, and the sweep found more of the same classes, fixed with them.

1. `path = "../wj-x/build"` is the generated output of the Windjammer package `../wj-x` (`package::graph::package_of_build_output`); the package is loaded as source, with its own dependencies (transitive, each crate once) and with `crate::` naming the package's own root (`resolve::Collected::crate_root_of`, interpreter `crate_root_module`). Gates: `ecosystem_findings2_interp::build_path_dependencies_are_interpreted_as_windjammer_packages`, `package::graph` and `interpreter::deps` unit tests.
2. Module-qualified std calls: `mime.APPLICATION_JSON` (constants read through a module value), `encoding.*`, `regex.find_all` / `escape` / `compile_with_flags` / `*_compiled`, `sync.unbounded` / `send` / `recv` / `sync_channel`, `path.*`, `math.TAU` / `SQRT_2` / `pow_*`, `env.set_var` / `remove_var`, `fs.read_bytes` / `write_string`, `process.run_with_output`. `path.glob_match` follows the runtime algorithm (`*` stays inside a segment, `**` spans). The agreement gate now checks the interpreter registry (functions and constants, `std/<mod>/mod.wj` and generic fns included) and the runtime's public surface; what is not implemented is listed in `tests/data/std_interpreter_missing.txt` and `tests/data/runtime_interpreter_missing.txt` (ratchets).
3. `use` inside a function body binds for the rest of the body (an overlay module per statement; the resolved engine hands such functions to the walker).
4. `crate::adapters::serve`: a module re-exports the public items of its child modules, as the generated `mod.rs` does (`pub use self::child::*`). A name that is both a module and a function (`pub use version::version`) resolves by use.
5. `std::thread::spawn(..)` / `std::async::..` as a path: `thread` and `async` are keywords and std module names; the parser now accepts them as path segments.
Also: `use std::log::*` (glob import of a std module) binds the module's functions bare, and they shadow same-named intrinsics.
