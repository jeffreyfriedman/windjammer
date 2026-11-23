# Disk hygiene

Building a Rust-backed compiler is generous with disk: a release build of the workspace writes several gigabytes, and
a team of agents and worktrees multiplies that. This page is the whole policy, in one place, so a build machine stays
healthy without anyone watching it.

## The policy

1. **One shared cargo target.** Every build, in every worktree, writes to the same `CARGO_TARGET_DIR`. Default:
   `~/Library/Caches/windjammer/cargo-target` on macOS, `${XDG_CACHE_HOME:-~/.cache}/windjammer/cargo-target` on
   Linux. Override with `WJ_SHARED_TARGET`. Shared means warm: the second worktree to build reuses the first one's
   dependencies.
2. **No private targets.** A `target/` inside a worktree, or a `wt-*` / `cargo-target-*` cache next to the shared
   one, is a policy violation. The janitor flags it, and removes it once it is idle or its branch is merged.
3. **Build through the slot gate.** `scripts/slot_gate.sh <command...>` takes a build slot (default 3), checks the
   disk floor, and sets `CARGO_TARGET_DIR` to the shared target. It refuses (exit 4) when `CARGO_TARGET_DIR` points
   elsewhere inside a worktree.
4. **The janitor tidies what we create, and only that.** It removes our own idle build output and scratch. It sizes
   other tools' and other sessions' directories, and never deletes them.

## The compiler's own size budget

`wj build`, `run`, `test` and `clean` keep wj's generated-project cache (`<cache>/cargo-target/shared`) under
`WJ_CACHE_MAX_GIB` (default 8). The budget sweep is safe by construction:

- It evicts only whole cargo units (a unit's `.fingerprint` directory, `build/` directory and `deps/` files together)
  or whole `incremental/` sessions, least recently used first. It never deletes a single file from `build/` or
  `deps/`: that leaves cargo believing a unit is fresh while its outputs (a build script's `bindgen.rs`, an `.rlib`,
  linker inputs) are gone.
- It sweeps only a tree that holds the `.wj-cache-root` marker, which wj writes into the trees it creates. A
  compiler build (`debug/`, `release/`) and any directory wj did not create is never swept.
- Over budget with nothing safe to evict gives one warning line, not corruption.

Anything written within the last hour, and any tree a `wj` process holds, is skipped.

## Floors

| Free space | What happens |
|---|---|
| 40 GB | Comfortable. The watchdog tick may run `clean --apply` to stay here. |
| 28 GB | Build floor. The slot gate waits (up to `WJ_DISK_WAIT_SECS`, default 300) and then refuses to start a build. |
| 20 GB | Hard stop. No new heavy jobs anywhere until space returns. |

`WJ_DISK_FLOOR_GB` changes the build floor.

## Commands

The janitor is interpreted Windjammer. Run these from the repository root, with `WJ_ECOSYSTEM` pointing at your
checkout of the ecosystem packages (or the packages checked out at the repository root):

```
wj run --allow process --allow env.read scripts/wj/disk_janitor.wj -- report --worktree-root <dir>
wj run --allow process --allow env.read scripts/wj/disk_janitor.wj -- clean --dry-run --worktree-root <dir>
wj run --allow process --allow env.read scripts/wj/disk_janitor.wj -- clean --apply --worktree-root <dir> --scratchpad <dir>
wj run --allow process --allow env.read scripts/wj/disk_janitor.wj -- gate --floor-gb 28
wj run --allow process --allow env.read scripts/wj/disk_janitor.wj -- --selftest
```

- `report` lists the ten biggest users among the shared target, every worktree target, the cargo registry and the
  temp dirs, each classified `ours`, `other` or `unknown`. Other sessions' directories are sized, never opened.
- `clean` removes only: (a) worktree targets whose worktree is gone, whose branch is merged into `--base`, or which
  nothing wrote to for `--hours` (default 6); (b) `wj-*` directories we own in the temp dirs and the children of
  `--scratchpad`, idle for `--hours`; (c) private target caches, flagged as violations. Nothing that a
  cargo/rustc/rustdoc/rust-analyzer process has open or mentions on its command line is touched.
  `--purge-shared` also empties the shared target, and is refused (exit 4) while any cargo or rustc process runs.
- `gate` exits 1 when free space is below the floor; the slot gate uses it before each build.
- Exit codes: 0 ok, 1 gate below floor, 2 usage, 3 still below the floor after cleaning, 4 refused.
- Locations come from `--shared-target`, `--worktree-root`, `--scratchpad`, `--base` or `WJ_SHARED_TARGET`,
  `WJ_WORKTREE_ROOT`, `WJ_SCRATCHPAD`, `WJ_BASE_REF`. Nothing assumes where the repository is cloned.

The self-test builds a fixture tree in a temp dir and checks every keep and remove decision. It then removes the
in-use check and requires the fixture to fail: a gate that cannot fail proves nothing.

## Installing the slot gate

Put `scripts/slot_gate.sh` where your agents find their `slot` command (for example `cp scripts/slot_gate.sh
<bin-dir>/slot`; it finds the janitor next to itself in a checkout, or set `WJ_JANITOR_CMD`). It is shell because
the Windjammer slot gate hangs under `wj run --interpret` (repro: `wj run --interpret slot.wj -- echo hello`); the
reason is recorded in the file header.

## Mac and PC

| | Linux/PC build host | macOS |
|---|---|---|
| Stale target pruning | `cargo-sweep` | the janitor |
| Compile cache | `sccache` | none by default |
| Shared target | yes | yes |

`cargo-sweep` prunes build artifacts by age or toolchain, and `sccache` makes a rebuild after a purge cheap. Both are
worth having on a Mac too: `cargo install cargo-sweep sccache`, then `export RUSTC_WRAPPER=sccache`. The janitor and
the sweep do not conflict; run the sweep against the shared target when no build is running.
