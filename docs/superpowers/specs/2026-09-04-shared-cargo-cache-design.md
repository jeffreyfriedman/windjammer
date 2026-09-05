# Shared Cargo Cache Design (2026-09-04)

## Goals

- Slow disk growth from Windjammer / dogfood builds and test suites
- Protect rebuild speed via shared compiled artifacts
- Keep application repos free of `target/` pollution
- Work on macOS, Linux, and Windows
- One place to inspect and prune

## How dependency sharing works

Cargo already shares **downloaded** crates in `~/.cargo/registry` (and git checkouts in `~/.cargo/git`). That does **not** share **compiled** `.rlib` / `.rmeta` artifacts.

Compiled units live under `target/<profile>/deps/` as `libfoo-<fingerprint>.rlib`. The fingerprint includes crate version, features, profile, and rustflags. **Two projects reuse a compiled dependency only if they share the same target directory and the fingerprint matches.**

Therefore this design uses:

| Dir | Purpose |
|-----|---------|
| `$ROOT/cargo-target/shared` | All real projects (compiler, game-core, runtime-host, DB crates, UI, …). Maximizes `.rlib` reuse. |
| `$ROOT/cargo-target/verify` | Ephemeral codegen / integration `cargo check` crates (often identical package names like `windjammer-app`). Isolated so they do not clobber final binaries in `shared`. |

Final binaries still land at `$ROOT/cargo-target/shared/<profile>/<package>`. Ecosystem package names must remain distinct (they already are).

## Cache root (cross-platform)

Resolve in order:

1. `WJ_CARGO_TARGET_ROOT` if set
2. Platform default:
   - macOS: `~/Library/Caches/windjammer`
   - Linux: `${XDG_CACHE_HOME:-~/.cache}/windjammer`
   - Windows: `%LOCALAPPDATA%\windjammer`
3. If the chosen root is not writable → fall back to `<cwd>/target` (sandbox / restricted CI)

Opt out of shared cache: `WJ_USE_LOCAL_TARGET=1`.

## Speed vs size knobs

- Keep incremental compilation **on** for `dev`
- `[profile.dev] debug = "line-tables-only"` in the compiler workspace (large size win, still useful backtraces)
- Prefer `--release` for the heavy compiler suite (existing practice)
- Serialize cargo on the shared verify dir (existing test mutex)

## CLI

```
wj cache path [--verify]     # print target dir (for scripts / cargo)
wj cache status              # sizes + free disk
wj cache prune               # age-based + low-disk reclaim
wj cache prune --aggressive  # also drop unused profile trees when low on disk
wj cache setup               # write env.sh / env.ps1 under the cache root
```

Defaults: prune incremental artifacts older than **7 days**; if free space **&lt; 15 GiB**, prune more aggressively (stale incremental + empty-ish fingerprint dirt).

`wj clean` removes temp `wj_*` dirs; `wj clean --all` also offers clearing in-repo `target/` leftovers. `wj cache prune` owns the shared cache.

## Integration points

- `wj` cargo invocations (`build --check`, `run`, `test`, `cargo_integration`) set `CARGO_TARGET_DIR` / `--target-dir` to `shared` or `verify`
- Compiler integration tests use `verify`
- `wj-game` cargo builds set `CARGO_TARGET_DIR` to `shared` and resolve binaries from that path
- Agents/docs: **set** canonical target dir (do not only `unset` sandbox overrides)

## Deferred (not in v1)

### sccache

A `rustc` wrapper that caches compiler outputs by input hash across machines/CI. Helps **CPU** and clean CI rebuilds; it is a second cache (can grow disk) and does not replace consolidating local `target/` trees. Can layer on later.

### One mega Cargo workspace

Putting compiler + game + DB + UI in a single `[workspace]` would share one `target/` and one lockfile automatically. It also couples unrelated release cycles, features, and dependency versions. Organizationally expensive; shared `CARGO_TARGET_DIR` gets most of the disk win without that coupling.

### Deleting `~/.cargo/registry`

That tree is the **source/download** cache (already global per user). Cleaning it forces re-downloads and does not remove compiled `.rlib`s in `target/`. Use `cargo cache` / registry trim tools separately if downloads become large.

## Migration

After adopting shared cache, one-time reclaim of old in-repo trees:

```bash
wj cache setup
# source the printed env, or rely on wj/wj-game wiring
wj cache prune
# optional: remove leftover ./target under dogfood repos
```
