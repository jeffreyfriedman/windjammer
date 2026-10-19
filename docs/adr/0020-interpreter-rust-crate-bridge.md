# ADR-0020: Interpreter host bridge for Rust code and Rust-crate dependencies

- **Status:** Proposed (spike landed)
- **Date:** 2026-10-11
- **Related:** [ADR-0012](0012-rust-interop-boundary.md), [ADR-0019](0019-interpret-by-default.md), `src/interpreter/host_bridge.rs`, `src/cli/host_build.rs`, `src/package/graph.rs`

## Context
Windjammer reaches Rust through `extern fn` declarations (ADR-0012 option A). The compiled path emits them as
`extern "C" { fn f(a: i64, s: FfiString) -> FfiString; }` and links the project's hand-written Rust (`ffi.rs`
next to `wj.toml` or in `src/`) plus the `wj.toml` Rust dependencies through Cargo. The interpreter cannot compile
Rust, so before this ADR every `extern fn` call was "unsupported", and any project touching a Rust crate could only
run compiled. For `wj run` to be the default dev loop (ADR-0019), those projects must run interpreted too.

## Decision (spike implemented)
Compile the project's Rust **once** into a cached dynamic library and call it from the interpreter through
generated C-ABI shims:

1. `cli::host_build::ensure` creates a `cdylib` crate in the wj cache: `#[path]` modules for every hand-written
   `.rs` file (the files the compiled build links), the Rust dependencies rendered by the same `Cargo.toml` line
   generator the compiled build uses, and `wj_host_shims.rs` from `interpreter::host_bridge::shim_source`.
2. Each `extern fn f` gets `__wj_shim_f(args: *const WjVal, n: usize, out: *mut WjVal) -> i32`, a uniform
   signature that unpacks [`WjVal`] (int / float / bool / UTF-8 string) and calls the real `extern "C" fn f`
   symbol with the **same C signature the compiled path uses** (`FfiString` for strings). One ABI, one Rust
   implementation, two callers.
3. The crate is keyed by a hash of: wj version, shim source (all signatures), dependency lines, every Rust source
   path and content. Hit: one `stat`, the library is `dlopen`ed (libloading). Miss: one `cargo build --release`.
4. Types that cannot cross the bridge are rejected before anything runs, naming the item:
   ``extern fn `f`: parameter `v: Vec<int>` cannot cross the host bridge (supported: integers, floats, bool, string)``.
   `wj run` then falls back to the compiled path (ADR-0019); `wj run --interpret` exits 2 with that message.
5. Calls into a Rust-crate dependency that are *not* wrapped by an `extern fn` (`serde_json::to_string(x)`) are
   reported by the preflight (`deps.rust_crate`) and run compiled.

## Measured (spike, this machine, loaded)
See `docs/INTERPRETER.md` "Rust code via the host bridge": first run (cargo build of the bridge) vs cached run.

## Not covered yet (next experiments, in order)
1. Structs and `Vec`/`Option` across the bridge: generate `#[repr(C)]` mirrors or a length-prefixed encoding in the
   shim; first experiment: `Vec<i64>` and `Option<string>` via (ptr, len) and a tag.
2. Opaque handles (`extern struct`): pass `u64` handles minted by the shim; a table in the library owns the objects.
3. Callbacks from Rust into interpreted closures: a C trampoline with a context pointer back into the interpreter.
4. Generic Rust APIs: out of scope; wrap them in a concrete `extern fn`.

## Kill criteria
- Cached-run overhead above 10 ms for a project with a bridge, or first-run build above the compiled `wj run` time
  for the same project: the bridge must never make the dev loop slower than compiling.
- Any divergence between bridged and compiled results on the gated fixtures (`tests/interpreter_projects_test.rs`
  `host_bridge_*`): the bridge is removed rather than special-cased.
- Platform without `dlopen` (wasm hosts): the bridge is disabled there and the fallback applies.
