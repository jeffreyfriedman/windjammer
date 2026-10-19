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
