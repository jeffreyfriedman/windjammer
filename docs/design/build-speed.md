# Build speed

Compiled Windjammer builds are fast when the dependency work is done once and shared. This note records
how the time is spent, what changed, and the gates that keep it that way.

## Method

Machine: shared macOS developer laptop, load average 18 to 60 during the runs (many parallel builds), so
wall time is reported but **CPU time (user+sys of the whole process tree, `/usr/bin/time -p`)** is the
number that is compared and gated. Program: a ten-line single file (`for` loop, one `println`).
`wj` binaries: the base (`next/v0.50.0` at the start of this work) and the new build, both release.
`cold` means an empty shared target tree for that profile; `warm` means the dependencies are cached.
Build parallelism was 2 jobs.

Phases of a compiled build: `wj` transpile, cargo resolve (no-op with a lockfile), dependency compile
(`windjammer-runtime` with axum/tokio, serde, smallvec), program crate compile, link.

## What the measurements showed

| scenario (small program) | wall s | CPU s | load |
|---|---|---|---|
| base `wj build`, cold (empty `<out>/target`) | 649 | 158 | 40 |
| base `wj build`, warm same output dir | 10.7 | 0.5 | 40 |
| base `wj build` into a **new** output dir | **828** | **193** | 43 |
| base `wj run --compiled`, warm | 18 to 20 | 1.1 | 25 |
| `wj transpile` only (`--no-cargo`) | 0.2 | 0.1 | 18 |

Findings:

1. `wj build` ran plain `cargo build` inside the output directory, so every new output directory paid for
   a full dependency compile (about 190 CPU seconds, dominated by `windjammer-runtime`'s tokio and axum).
   `wj run --compiled`, `wj check` and `wj test` already used the shared target tree; `wj build` was the
   outlier. That is the 167 s median (423 s p90) the agent pilot saw.
2. `wj run --compiled` created a fresh temporary directory per run. A new path is a new cargo package
   identity, so the program crate recompiled and relinked on every run even with nothing changed.
3. Transpiling is not a factor (0.2 s). Cargo resolve with a lockfile and path-only dependencies is well
   under a second.
4. Debuginfo is a minor cost for this program; the runtime dependency graph is the cost.

## Changes

* `wj build` (Rust target) now compiles in the shared target tree (`wj cache path`) like `wj run
  --compiled`. The executable is still published at `<output>/target/{debug,release}/<name>`, the
  documented location, and only rewritten when its bytes change.
* One dev profile for both commands, set through `CARGO_PROFILE_DEV_*` environment variables (no debuginfo,
  incremental on for the program crate, 256 codegen units, no LTO). The generated `Cargo.toml` is
  byte-identical to before. `--release` keeps the profile from `wj.toml`.
* `wj run --compiled` uses a stable work directory per source file (`<cache>/run/<hash of path>`), so an
  unchanged program is a cargo no-op and an edited one compiles incrementally. Generated files are written
  only when their content changes (`write_if_changed`). Directories unused for 14 days are pruned.
* Cache eviction under the size cap (`WJ_CACHE_MAX_GIB`, default 8) now removes incremental state first
  and compiled dependency artifacts last, so a prune does not throw away the minutes-long runtime build.
* Invalidation: the embedded runtime is materialized at `<tmp>/wj-embedded/windjammer-runtime-<version>-<content
  hash>`. The content hash is part of the path, so any edit to runtime sources (or a new compiler version)
  produces a new path and therefore a new cargo fingerprint; the old artifacts age out through prune.

## After

| scenario (small program) | wall s | CPU s | load |
|---|---|---|---|
| `wj build` into a new output dir, cold shared tree (one time per machine) | 287 | 181 | 28 |
| `wj build` into a **new** output dir, warm shared tree | **13.9 to 25.6** | **1.5 to 2.1** | 20 to 28 |
| `wj build`, same output dir, unchanged | 8.8 | 0.5 | 28 |
| `wj run --compiled`, warm | 15 to 19 | 0.4 to 0.9 | 20 to 25 |

The new-output-dir build, the case the pilot's final `wj build` hits, drops from 193 CPU seconds to about
2 (roughly 100x). Wall time on a loaded machine is dominated by scheduling and disk waits for a process that
needs one to two CPU seconds; on an idle machine these runs finish in the CPU time. The first compiled build
on a machine still compiles the runtime once (181 CPU seconds); every later build and run shares it.

Cache disk cost: the compiled runtime and its dependencies take about 1.1 GiB in the shared tree
(`<cache>/cargo-target/shared/debug/build/windjammer-runtime`), plus about 150 MiB of incremental state; the
whole shared tree on the measuring machine is 5.7 GiB including other projects, under the 8 GiB cap
(`WJ_CACHE_MAX_GIB`). Per-source run directories hold a few KiB each.


## Round 2: feature-gated runtime, persistent unpack, more gates

The first compiled build on a machine used to compile the whole runtime (181 CPU seconds, 1.1 GiB) because
tokio, axum, reqwest, clap, regex and the crypto crates were unconditional. They are now cargo features of
`windjammer-runtime` (`net`, `async`, `cli`, `crypto`, `datetime`, `jwt`, `regex`, `parallel`, `yaml`,
`compress`, `csv`, `log`, `db`; `default` still enables everything, so every other consumer is unchanged).

* The generated `Cargo.toml` enables only the groups the generated Rust names. The module-to-feature map is
  read from the runtime's own `src/lib.rs` (`#[cfg(feature = "net")] pub mod http;`), so the runtime is the
  single source of truth (`src/cargo_toml/runtime_features.rs`). Anything the scanner cannot classify (a
  glob import, an unknown module) enables `full`: a miss costs compile time, never correctness.
* `wj test`, the host bridge and the test helpers keep the previous full set.
* The embedded runtime is unpacked into the wj cache root (`<cache>/embedded/...`) instead of the OS temp
  directory, so an OS temp purge cannot invalidate the compiled runtime. Disk cost: about 1 MiB.
* `wj cache warm` builds a trivial program once so the first real build is already warm.
* Concurrent `wj run --compiled` of one source serialize the transpile step with a file lock.

| scenario (small program, no network code) | wall s | CPU s | cache on disk | load |
|---|---|---|---|---|
| first build on a machine, before | 287 | 181 | 1.1 GiB | 28 |
| first build on a machine, after | 17 | 34.6 | 132 MiB | 50 |
| second build into a new output dir, after | 0.8 | 0.37 | shared | 50 |

Programs that use a gated module pay for exactly that module: a program using `std::time` enables
`datetime` (chrono) and nothing else; one using `http` enables `net`.

### Further gates

* `tests/build_speed_gate.rs`: first-build CPU budget on an empty cache, concurrent compiled runs, and an
  ignored mutation check: with the shared target tree disabled (`WJ_USE_LOCAL_TARGET=1`, the old behaviour)
  the warm-build budget is exceeded, so the budget gate has teeth.
* `tests/build_output_golden.rs`: generated Rust for the verified examples hashes to the recorded values
  (`tests/golden/build_output_hashes.tsv`); a sampled subset runs every commit, the whole corpus with
  `--ignored`. Re-record after an intended codegen change with `WJ_RECORD_GOLDEN=1`.
* Runtime feature selection has unit tests against the real runtime `lib.rs` (a no-network program gets no
  heavy group, `http` gets `net`, unknown usage gets `full`).

### Finding: compiler output depends on neighbouring builds

While recording the golden hashes, the generated Rust for the same example differed depending on which
other examples had been built before it from the same project root: the compiler keeps `.wj-cache`
metadata under the project root and a neighbour's metadata changed how `println("{:?}", x)` was generated
(macro form versus a `println(&_temp)` call). The golden test therefore builds each example alone in a
private directory. The underlying behaviour belongs to the compiler's metadata cache, not this work. It also
made the 169-example record run take 5 to 15 minutes instead of 13 seconds.

## Gates

* `tests/build_speed_gate.rs`: CPU budgets for warm `wj run --compiled` and warm `wj build` into a fresh
  output directory; the binary appears at the documented path; its output equals the run output; the
  generated `main.rs` and `Cargo.toml` equal the `--no-cargo` output.
* `src/embedded_assets.rs` `cache_key_tests`: the cache key changes when the runtime content changes; the
  same property is applied to a mutant key that drops the hash and must fail (in-process mutation check);
  the baked hashes equal the runtime hash function.
* `src/cargo_cache/build_profile.rs` unit tests: executable discovery, idempotent publish, stable work
  directories.

## Roadmap

* Ship a pre-built runtime artifact with release binaries so even the first build on a machine is warm (`wj cache warm` is the on-demand version today).
* Use `--offline` when the lockfile is satisfied and all dependencies are vendored or path-based.
