# Profiling with Tracy

Windjammer integrates the [Tracy](https://github.com/wolfpld/tracy) profiler through `windjammer-runtime`.

## CPU zones: `@profile("Name")`

Annotate a function so the compiler emits a Tracy zone around its body (via `windjammer_runtime::profiling::tracy_zone`):

```windjammer
@profile("game_update")
pub fn update() {
    // hot path
}
```

This maps to a Rust scope guard at the start of the generated function body, equivalent in spirit to:

```rust
let _wj_profile_zone = windjammer_runtime::profiling::tracy_zone("game_update");
```

Zones nest correctly with other body-wrapping decorators (`@timeout`, `@bench`, …): the profile zone is placed **inside** those wrappers so measured time matches the function work as closely as possible.

**Requirements:** the first argument must be a string literal (compile-time name).

## Enabling Tracy (`--features tracy`)

Tracy instrumentation is **off by default** so there is no extra code or link dependency unless you opt in.

1. In the crate that depends on `windjammer-runtime` (e.g. your game host / `runtime_host`), enable the feature:

   ```toml
   windjammer-runtime = { path = "…/crates/windjammer-runtime", features = ["tracy"] }
   ```

2. Build with Cargo as usual:

   ```bash
   cargo build -p your-game-host --features tracy
   ```

With the feature disabled, `tracy_zone` is an empty inline function and a zero-sized guard: **zero cost** in release builds.

Security note: Tracy’s default client behavior may broadcast discovery on the local network. Review `tracy-client` [feature flags](https://docs.rs/tracy-client) (e.g. `only-localhost`, `ondemand`) if you need a tighter setup; optional features can be enabled via `[patch]` or a forked dependency if your policy requires it.

## GPU zones (optional)

For GPU timing, Tracy expects timestamp pairs from your graphics API. The runtime re-exports Tracy GPU types under:

- `windjammer_runtime::profiling::tracy_gpu` (requires `--features tracy`)

Use `GpuContext` / `GpuSpan` from `tracy_client` together with your backend’s timestamp queries (e.g. `wgpu` timestamp query sets, then map the resolve buffer and call `GpuSpan::upload_timestamp_start` / `upload_timestamp_end`). Exact wiring is engine-specific; see `tracy_client::GpuContext` docs and the Tracy manual for GPU capture.

## Manual Rust API

For Rust-only code paths you can call:

```rust
let _zone = windjammer_runtime::profiling::tracy_zone("my_scope");
```

Use the same `tracy` feature on `windjammer-runtime` as for generated games.
