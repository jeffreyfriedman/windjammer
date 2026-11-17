---
status: measured
---
# Economics results: Windjammer against hand-written Rust

Definitions, programs and thresholds: [DEFINITION.md](DEFINITION.md). Interpretation and fixes:
[ADVANTAGES.md](ADVANTAGES.md). Raw data: `bench/economics/results/`.

Every number below is labelled **measured** (a command in this repository produced it, the command is named) or
**reported** (from a source outside this repository's data). Nothing is estimated in this file.

## Conditions (read before quoting)

- Run: `bench/economics/results/mac-20261116.json` (tables: `mac-20261116.md`), 78 programs, 75 clean (3 listed under
  "Problems"). Compiler: installed `wj` commit 37d50458ec6a (2026-11-11; the integration head is newer, a head run is
  PLAN.md step 1); rustc 1.100.0-nightly; macOS arm64, 8 cores; load average 47 to 13 during the run (above the core
  count most of the time).
- Under that load wall-clock is noise. Counters: **cycles** and **instructions retired** of the measured process
  (`/usr/bin/time -l`), **user+sys CPU** (children included), **peak RSS**. Instructions alone mislead across different
  programs (`wj check` retires 253M instructions in 96M cycles, rustc runs at a third of that IPC), so cross-program
  ratios use cycles and CPU; rustc's linker is a child process, so only CPU includes it.
- Same flags on both sides: debug `-C opt-level=0 -C debuginfo=0`; release `-C opt-level=3 -C lto` (the generated
  crate's dependencies are LTO bitcode, so the twin gets LTO too). "Shipped" = the generated `Cargo.toml` as is
  (LTO, overflow checks on) against the twin's release build.
- Every number is the smallest of 3 runs (dx) or 1 run (kernels' slow variants); every output was checked.
- Ratio = Windjammer / Rust; below 1.00 is cheaper. `geo` = geometric mean over programs.

## Summary per dimension (measured)

| Dim | Metric | dx (58) | parity (10) | interp (7) | Threshold (DEFINITION.md) | Verdict |
|---|---|---:|---:|---:|---|---|
| D2 | verify loop, `wj run --interpret` vs `rustc` debug + run, **CPU s** | **0.24** | 6.01 | 0.40 | win <= 0.50 on dx | **WIN** |
| D2 | same, cycles of the process (excludes rustc's linker) | **0.62** | 7.94 | 1.06 | | win on dx |
| D2 | compiled loop: transpile + rustc debug + run, instructions | 2.20 | 1.58 | 2.07 | | loss (runtime crate linked) |
| D3 | transpile alone vs rustc debug of the twin, CPU s | 0.39 | 0.41 | 0.41 | transpile <= 10% of rustc share | loss vs threshold, cheap in absolute terms (0.06 s) |
| D3 | transpile + rustc `-O3 -C lto`, cycles | 1.36 | 1.03 | 1.05 | parity <= 1.10 | loss on dx (runtime LTO), parity on kernels |
| D3 | shipped `cargo build --release`, warm, CPU s | 1.41 | 1.11 | 1.14 | | loss |
| D3 | cold dependency build, one-off (below) | 30.8 s CPU, 59 crates | | | no dependency build | **largest loss** |
| D4 | compile peak RSS (`-O3`) | 1.28 | 1.00 | 0.99 | <= 1.00 | loss on dx, parity on kernels |
| D4 | run-time peak RSS | 1.03 | 1.03 | 1.00 | win <= 0.95, parity <= 1.05 | parity |
| D5 | run, `-O3 -C lto` same flags, overflow checks OFF on both sides, cycles | 1.00 | 1.20 | 1.12 | win <= 0.95, parity <= 1.02 | parity on dx, loss on kernels |
| D5 | run, shipped (overflow checks ON) vs twin (OFF), instructions | 1.02 | 1.31 | 1.37 | | loss (overflow checks) |
| D6 | stripped binary, same flags | 1.19 | 1.00 | 1.00 | win <= 0.95 | loss on dx (runtime io), parity on kernels |
| D7 | ceremony (`&`, lifetimes, clone, to_string, unwrap, as, iter) | **0.45** | **0.42** | 0.67 (n=2) | win <= 0.50 | **WIN** |
| D7 | lexical tokens | 1.08 | 0.99 | 1.00 | win <= 0.85 | loss on the verbose reference set; 0.88 on pilot-6 agent programs |
| D1 | agent USD per passing trial (pilot 6) | 1.17 / 1.21 | | | win <= 0.90 | loss (CI includes parity) |

Two measured advantages today: **the interpreted verify loop on agent-sized programs** (D2, 0.24x CPU, 58 of 58
programs cheaper) and **less ownership ceremony in the source** (D7, 0.45x). Both have gates (PLAN.md). The interpreter
loses on compute kernels (6x CPU on `parity`: those are the programs to compile, which `wj run --verify` does).

### Run time per kernel (D5, `-O3` same flags)

| kernel | cycles | instructions | shipped instructions | likely cause |
|---|---:|---:|---:|---|
| parity/vec_fill | 2.79 | 1.97 | 2.67 | index loop + `as u64` casts vs iterator `sum` (RESULTS of ADR-0006 work) |
| interp/strings | 2.16 | 2.39 | 2.39 | `push_str("${..}")` allocates a temporary String per iteration; the twin uses `write!` |
| parity/sieve | 1.49 | 1.58 | 1.59 | push loop vs `vec![true; n]` |
| interp/hashmap | 1.42 | 1.65 | 1.69 | `get` then `insert` (two lookups) as written in the kernel; the twin uses `entry` |
| parity/string_build | 1.34 | 1.26 | 1.26 | per-word `format!` temporary |
| interp/sieve | 1.22 | 1.40 | 1.41 | push loop |
| parity/matmul | 1.12 | 1.25 | 1.25 | index arithmetic |
| interp/nbody | 1.09 | 1.01 | 1.02 | - |
| collatz, fib, insertion_sort, nbody, byte_hash, histogram, fib30, structs | 1.00 | 0.93-1.00 | 1.00-1.52 | parity; shipped overflow checks cost up to 1.5x |
| interp/loop5m | **0.56** | 0.49 | 1.14 | the generated `while` loop vectorises better than the twin's `for` range; overflow checks take it back |

### Problems (excluded from the summaries)

- `dx/t054_dir_tree`: every variant differs from the sample's stdout: the runner did not create the sample's nested
  directory (`src/main.rs`; fixed in the runner since), and the interpreted run lists the copied `main.wj` too
  (runner limitation, not a program defect).
- `dx/t055_log_summary`: the generated Rust does not compile (`no method named iter found for SplitN`): compiler bug at
  37d50458ec6a.
- `interp/closures`: the generated Rust does not compile (E0308 in the iterator chain): compiler bug at 37d50458ec6a.
  The interpreter runs both correctly.

## D2 ratchet and C1 (measured)

`economics.wj --d2` measures the verify loop on a fixed set of 12 agent-sized programs: `wj run --interpret` against
`rustc` debug + run of the Rust twin, cycles of the process (CPU seconds where the meter has no cycles), best of 3,
geometric mean. With the integration-head interpreter (wt-int-b + C1): **0.368** (every one of the 12 cheaper; from
0.17x for word_freq to 0.65x for bank_ledger). Recorded in `bench/economics/d2_baseline.json` per meter and machine;
`--d2 --check bench/economics/d2_baseline.json` fails when the ratio rises more than 3% above the record (`--write`
lowers it; raising needs `--allow-raise`). Mutation check: a doctored baseline (0.05) fails the check. The check runs on
a machine with the same meter (Mac, or the PC once it records its own entry); another meter's entry is never
compared.

C1 (branch `wt-econ-c1`, on wt-int-b): `wj run --verify` stops before the build when the interpreted run panics (exit
101, measured 0.51 s instead of a build of about 2 s warm), still builds and compares any other exit status, and the
default agent sheet names `wj run --verify FILE` as the one loop command.

## C12: single-file builds and the surrounding tree (measured before/after)

`wj build --no-cargo` of `bench/agent-dx/solutions/wj/t001_gcd_lcm/main.wj` inside this repository against the same file
alone in an empty directory, `WJ_META_SCAN_STATS=1` and `/usr/bin/time -l`, best of 3:

| compiler | in repository: CPU / instructions / dirs listed / metadata files opened | alone |
|---|---|---|
| installed wj 37d50458ec6a (Nov 11) | 1.26 s / 6.7 G / - / - | 0.09 s / 0.58 G |
| head f55f8b1 + counters, before the fix | 0.04 s / 404 M / 139 / 78 | 0.03 s / 312 M / 7 / 0-1 |
| after the fix (open only reached peers) | 0.03 s / 384 M / 139 / **0** | 0.03 s / 310 M / 7 / 0 |

Most of the 37d50458 cost was already gone on the head (peer metadata read only when current and reached, c52bfb52c);
the fix removes the remaining reads of every cached module's metadata: a single-file build now opens only the
metadata of modules it names. The 139 directory listings remain (finding peers by name in the project cache; cheap,
reported, not gated). Gate: `cargo test --release --test meta_scan_locality` (files opened do not grow from 2 to 40
unrelated cached modules; a file in a project opens no more than a file alone); mutation check: counting the read
before the reachability check (the old order) fails both tests (2 vs 40, 40 vs 0).

## C2 step 4: lean generated manifest (measured before/after)

The generated `Cargo.toml` now names `windjammer-runtime`, `smallvec` and `serde` only when the generated code names
them (`cargo_toml::toml_generation::names_default_deps`). Cold `cargo build` (debug) of the generated crate into an
empty target directory, `/usr/bin/time -l`:

| program | before (wj 37d50458): deps / CPU / peak RSS / rlibs / target | after: deps / CPU / peak RSS / rlibs / target |
|---|---|---|
| `interp/fib30` (std only) | 3 / 32.2 s / 463 MB / 61 / 191 MB | **0 / 0.19 s / 94 MB / 0 / 1.1 MB** |
| `dx/t001_gcd_lcm` (reads stdin through the runtime's `io`) | 3 / 28.4 s / 459 MB / 61 / 193 MB | 1 / 29.2 s / 456 MB / 61 / 193 MB |

18 of the 78 suite programs never name the runtime and now build like their Rust twin (about 170x less CPU on the
first build). The other 60 still pay the runtime's dependency tree: C2 step 5 (runtime dependencies behind features)
is what removes that. Gate: `cargo test --release --test lean_manifest` (a std-only program has no dependencies; a
runtime-using one has exactly the runtime); mutation check: always adding the runtime fails the first test.

`wj check` of one file inside this repository vs the same file alone (the 40 s report from another stream): on this
head the first check inside the repository costs 0.58-1.13 G instructions against 0.43 G alone, repeated checks
0.40 G (the check cache); `wj check` merges no peer metadata here. The 40 s case did not reproduce in three
checkouts (this worktree, the concurrency worktree, the main checkout with a 3.5 GB `target/`); the gate now covers
`wj check` and `wj run --interpret` too (`check_and_run_open_no_metadata_of_unrelated_modules_either`).

## C2 step 5: runtime dependencies behind a feature (measured before/after)

`windjammer-runtime`'s third-party dependencies (serde, serde_json, url and its ICU stack, uuid, rand, base64, hex,
mime_guess, dirs, bumpalo, anyhow, thiserror, async-trait, once_cell, ...) are now optional, behind a `base` feature
that every other feature implies and the default set includes. The modules that use them (`json`, `url`, `uuid`,
`encoding`, `random`, `property`, `arena`, `durable`, `mime`, `subprocess`, `platform`, `cap`) carry
`#[cfg(feature = "base")]`, so `cargo_toml/runtime_features.rs` (which reads that map from the runtime's `lib.rs`)
enables `base` exactly when the generated code names one of them. A program that uses only `io`, `fs`, `env`,
`process`, `thread` and `sync` builds the runtime with no feature and only `libc`.

Cold `cargo build` (debug) of the generated crate into an empty target directory:

| program | before: CPU / peak RSS / rlibs / target | after: CPU / peak RSS / rlibs / target | output |
|---|---|---|---|
| `dx/t001_gcd_lcm` (stdin through `io`) | 28.4 s / 459 MB / 61 / 193 MB | **2.9 s / 252 MB / 2 / 40 MB** | `6 36` (expected) |
| `dx/t048_wc_clone` (files through `fs`) | (same tree) | 3.0 s / 250 MB / 2 / 41 MB | runs |
| `dx/t043_channel_pipeline` (threads, channels) | (same tree) | 3.2 s / 239 MB / 2 / 45 MB | runs |

With step 4, every program in the suite that uses only the core modules now builds from cold in about a tenth of the
CPU (and the std-only ones in about 1/170th). Gates: `cargo check -p windjammer-runtime` with no features, `base`,
`net` and the default set (all compile); `tests/lean_manifest.rs`; the full compiled `dx` oracle run on the PC is
the remaining check.

## C4: release profile and overflow checks (measured)

`economics.wj --profiles` (`bench/economics/results/mac-20261116-profiles.{json,md}`): the 17 std-only kernels (the
generated Rust, and each Rust twin) built with eight rustc flag sets, best of 3 runs, load average 11 to 15. Ratios
are Windjammer with the profile / the Rust twin with cargo's default release flags (`-C opt-level=3`, 16 codegen
units, no LTO, no overflow checks), geometric mean:

| profile | run cycles vs twin | stripped size vs twin | compile CPU vs twin | run cycles vs own default |
|---|---:|---:|---:|---:|
| cargo default (`-C opt-level=3`) | 1.12 | 1.00 | 1.05 | 1.00 |
| fat LTO (today's generated default, without overflow checks) | 1.16 | 0.88 | 14.2 | 1.04 |
| thin LTO | 1.13 | 0.99 | 19.1 | 1.01 |
| fat LTO + 1 codegen unit | 1.16 | 0.88 | 14.1 | 1.04 |
| fat LTO + 1 codegen unit + overflow checks (closest to what ships today) | 1.29 | 0.88 | 14.2 | 1.15 |
| thin LTO + overflow checks | 1.23 | 0.99 | 19.2 | 1.10 |
| cargo default + overflow checks | 1.25 | 1.00 | 1.03 | 1.12 |
| fat LTO + 1 codegen unit + `panic=abort` | 1.15 | 0.88 | 13.9 | 1.02 |

Per program, fat LTO costs about 2 s more CPU to compile (0.13-0.27 s without, 2.0-2.4 s with) to save 40 KB of a
340 KB binary, and runs no faster on these single-crate programs. Overflow checks cost 1.12x run cycles on average
(Windjammer and Rust alike: the twin with checks is 1.25x its own default) and up to 1.77x (`interp/loop5m`), 1.35x
(`collatz`), 1.34x (`structs`), 1.27x (`fib`); about half the kernels are within 5%. (`interp/closures` is excluded:
its generated Rust does not compile with this wj.)

### Overflow checks on vs off per kernel (instructions)

Instructions retired of the run (macOS hardware counter, best of 3; the PC repeats this with callgrind:
`economics.wj --profiles --instr callgrind`). Ratio = overflow checks on / off, same flags otherwise:

| kernel | Windjammer, `-C opt-level=3` | Windjammer, fat LTO + 1 codegen unit | Rust twin, `-C opt-level=3` |
|---|---:|---:|---:|
| parity/sieve | 1.01 | 1.00 | 0.89 |
| parity/matmul | 1.00 | 1.00 | 1.00 |
| parity/collatz | 1.24 | 1.52 | 1.24 |
| parity/string_build | 1.00 | 1.00 | 1.01 |
| parity/fib | 1.32 | 1.32 | 1.32 |
| parity/vec_fill | 1.35 | 1.35 | 1.70 |
| parity/insertion_sort | 1.00 | 1.00 | 1.00 |
| parity/nbody | 1.00 | 1.00 | 0.98 |
| parity/byte_hash | 1.00 | 1.00 | 0.99 |
| parity/histogram | 1.23 | 1.22 | 1.22 |
| interp/fib30 | 1.20 | 1.20 | 1.19 |
| interp/hashmap | 1.02 | 1.01 | 1.00 |
| interp/loop5m | 2.33 | 2.34 | 1.13 |
| interp/nbody | 1.00 | 1.00 | 1.00 |
| interp/sieve | 1.01 | 1.00 | 1.00 |
| interp/strings | 1.00 | 1.00 | 1.00 |
| interp/structs | 1.14 | 1.16 | 1.14 |

### Clawback: the bounded_counter pass (PASSES.md P10, measured)

`economics.wj --clawback` (`bench/economics/results/mac-20261116-clawback.{json,md}`): each std-only kernel's
generated Rust with the production optimizer with and without `bounded_counter`, built `-C opt-level=3` with overflow
checks on, against the same code with checks off (the floor; never shipped). Instructions, best of 3:

| kernel | pass edits | checks on, no pass | checks on, with pass | checks off | cost of the checks | recovered by the pass |
|---|---:|---:|---:|---:|---:|---:|
| interp/strings | 1 | 149.6M | 148.4M | 148.4M | 1.01x | all of it |
| interp/nbody | 4 | 16.3M | 16.2M | 16.2M | 1.00x | 97% |
| interp/sieve | 2 | 43.0M | 42.9M | 42.9M | 1.00x | 94% |
| interp/hashmap | 3 | 54.0M | 53.2M | 52.8M | 1.02x | 72% |
| interp/structs | 1 | 18.1M | 17.9M | 15.4M | 1.17x | 6% |
| interp/closures | 1 | 12.6M | 12.6M | 11.6M | 1.09x | 3% |
| interp/loop5m | 2 | 73.4M | 73.4M | 31.6M | 2.32x | 0% |
| parity kernels (`for` ranges), fib30 | 0 | | | | 1.00x to 1.35x | 0% |

Reading, plainly: the pass is sound (gate below) and removes every counter check it targets, but the checks that cost
real time are not counter checks. In `loop5m` the 2.32x is the accumulator (`total += x`), in `fib`/`fib30` the
additions of recursive results, in `collatz` `3n + 1`, in `vec_fill` and `histogram` the summations: values that can
genuinely overflow for some input, so no proof removes them, and Windjammer keeps them by design (owner decision;
`wrapping_*`/`checked_*` or the profile opt-out are the explicit ways out). LLVM already removes most bounded counter
checks itself; the pass mostly helps where the counter feeds index arithmetic (`nbody`, `sieve`, `hashmap`). The next
clawback with real yield is range facts from the numeric solver for values derived from inputs with declared bounds,
not more syntactic patterns.

Gate: `cargo test --release --test optimizer_bounded_counter`: rewrites and refusals on source; three programs whose
overflow must still panic (a `u8` product reaching 258, a `<=` bound, a second write to the counter) panic before and
after the pass; and each unsound variant (`CounterIgnoresTypeMax`, `CounterAcceptsLessEqual`,
`CounterIgnoresOtherWrites`) turns one of them into a program that no longer panics, which the test detects.

Owner decision (2026-11-16): overflow checks stay **on** by default in release builds (compiled output must match the
interpreter, which panics on overflow). Every D5 number in this file states its setting: "same flags" rows are
without checks on both sides; "shipped" rows are Windjammer with checks against the Rust twin without.

### LTO on programs that use the runtime (measured)

`economics.wj --lto-study` (`bench/economics/results/mac-20261116-lto.json`): 11 programs (9 `dx` programs that use the
runtime crate, 2 std-only kernels) built `cargo build --release` with the generated profile (overflow checks on) with
LTO on and off, each into its own empty target directory. Geometric means, LTO on / off:

| | ratio | per program |
|---|---:|---|
| cold release build CPU (dependencies included) | 1.55 | runtime users 5.6-6.7 s vs 5.1-6.5 s; std-only 2.1 s vs 0.2-0.3 s |
| incremental release rebuild CPU (`main.rs` touched) | **6.74** | 2.8-3.6 s vs 0.3-1.1 s |
| run instructions | 0.99 | 0.96-1.01 |
| run cycles | 0.98 | |
| stripped binary size | 0.86 | e.g. 369 KB vs 430 KB |

(`dx/t009_rpn_eval` did not build with the lean-runtime compiler in this run and is excluded; under investigation.)

Default change (implemented, `wt-econ`): `[profile.release] lto` unset is automatic: on when the project depends on
another Windjammer package (the cross-crate case LTO was chosen for), off for a single crate, where it bought about 1%
run time and 14% size for about 7x the CPU of every release rebuild. `lto = true` / `false` decides explicitly
(`docs/book/src/reference/wj-toml.md`). Gate: `tests/lean_manifest.rs`
(`release_lto_is_automatic_and_an_explicit_setting_wins`).

Proposal (owner decision; no default changed yet):

- **Overflow checks stay on** in shipped Windjammer binaries. The spec says integer overflow panics; dropping the checks
  would make Windjammer silently less safe than its own definition. The opt-out already exists and is explicit:
  `wj build --unchecked-arithmetic`, or `[build] overflow_checks = false` in `wj.toml` (`docs/spec/numeric.md`). The
  cost is now measured per kernel; the way to earn it back is C10 (prove ranges in the numeric solver and emit
  unchecked arithmetic only where overflow is impossible).
- **LTO only where it can pay**: for a crate with no Windjammer/path dependencies, fat LTO buys 12% size for 14x
  compile CPU and no speed. Proposed default: `lto = false` for std-only crates; keep `lto = true` when the crate
  depends on other Windjammer crates (the cross-crate case the current default was chosen for; to be measured on the
  runtime-using `dx` programs once C2b lands); a size profile (fat LTO, `codegen-units = 1`, strip) on request.
- **`panic = "abort"` not by default**: about 1% faster, but it changes a panicking program's exit status (101 to 134)
  and skips unwinding, which the spec's panic semantics fix.
- **One codegen unit**: no measurable effect on these programs; no default change.

## Cold dependency build of a generated crate (measured)

`wj build --no-cargo` of `dx/t001_gcd_lcm`, then `cargo build` and `cargo build --release` into an empty target
directory (`CARGO_BUILD_JOBS=2`), `/usr/bin/time -l`; two runs (by hand, then by `economics.wj`,
`bench/economics/results/mac-20261116-cold.json`):

| profile | CPU s (user+sys) | peak RSS | crates compiled |
|---|---:|---:|---:|
| debug | 28.9 / 30.8 | 429 MB | 69 (59 distinct library crates) |
| release | 36.0 / 39.1 | 455 MB | 69 (59) |

The target directory holds 303 MB afterwards. The Rust twin compiles with `rustc` alone: no dependencies, nothing
to build first. Then, warm, the same crate rebuilds in 2.9 s CPU (release, LTO, peak 255 MB) and 0.35 s CPU (debug).

## Agent dimension (D1)

Measured by the `bench/agent-dx` harness (same tasks, model and grader for every arm); collected by `economics.wj`
from `bench/agent-dx/results/*/report.json`. Arms with a Rust baseline:

| pilot | arm | language / loop | pass@1 | USD / trial | USD / passing trial | turns | tokens out |
|---|---|---|---:|---:|---:|---:|---:|
| pilot 2 | R | Rust | 1.00 | 0.022 | 0.022 | 3.0 | 1191 |
| pilot 2 | A | Windjammer, interpreted-first | 0.40 | 0.028 | 0.071 | 3.3 | 1388 |
| pilot 2 | B | Windjammer, compiled-first | 0.87 | 0.038 | 0.044 | 5.0 | 1718 |
| pilot 2 | G / P | Go / Python | 1.00 | 0.021 / 0.019 | 0.021 / 0.019 | 2.6 / 3.6 | 1070 / 941 |
| pilot 6 | R | Rust | 1.00 | 0.033 | 0.033 | 3.0 | 1710 |
| pilot 6 | U | Windjammer, compiled-first | 1.00 | 0.039 | 0.039 | 3.8 | 1798 |
| pilot 6 | V2 | Windjammer, interpreted-first + verify | 1.00 | 0.040 | 0.040 | 4.8 | 1869 |

Pilot 6 paired differences (R minus arm, 95% bootstrap CI): cost R-U -0.005 [-0.016, 0.005], R-V2 -0.007
[-0.023, 0.006]; turns R-V2 -1.8 [-4.2, -0.4]; tool-wait seconds R-U -9.2 [-14.3, -5.5] (the compiled build in the
Windjammer loop). **Reported** (coordinator summary of pilot 8; raw data not in this repository): Windjammer about
1.7x Rust cost per task, 3x turns, 1.5x output tokens, programs equal by characters.

Reading: D1 is a **loss** today (1.17x to 1.21x in pilot 6, CIs include parity; about 1.7x in pilot 8). The gap is
turns, not program size (D7), and turns trace to friction events (ADR-0037).

## PC commands (quiet box, Linux; PLAN.md step 1)

No root or package installs needed. `economics.wj` picks the best measuring path the machine has and records it in
`env.counter` / `env.meter` of the results file:

| Path | Used when | Gives |
|---|---|---|
| `-l` | macOS | CPU (children included), peak RSS, instructions and cycles of the process |
| `-v perf ...` | GNU `time` and a working `perf` | CPU, peak RSS, instructions and cycles (children included) |
| `-v` | GNU `time`, no perf | CPU, peak RSS |
| `rusage` | no GNU `time` | `bench/economics/rusage_helper.rs` (built with rustc, no crates): CPU and peak RSS of all waited-for children via getrusage |
| `wall` | nothing else | wall clock only, median of the runs |
| `+ callgrind` | `--instr callgrind` (opt-in: slow) | one extra run per command under `valgrind --tool=callgrind --trace-children=yes` for instructions |

Results from different paths are not compared with each other; ratios inside one run always are.

```
# once: any directory names
git clone <windjammer> wj && git clone <windjammer-ecosystem> eco
cd wj && git checkout <integration head>
cargo build --release --bin wj
export WJ_ECOSYSTEM=$PWD/../eco WJ=$PWD/target/release/wj

# which paths work on this machine (each must say ok)
$WJ run scripts/wj/economics.wj --selftest

# quiet check: load below 1 before the timed step, nothing else running
uptime

# the suite; an empty --target measures the cold dependency build first
$WJ run scripts/wj/economics.wj --wj $WJ --target /tmp/econ-target-pc --label pc-$(date +%Y%m%d) --runs 5
#   add --instr callgrind for instruction counts without perf (valgrind; slow)
$WJ run scripts/wj/economics_report.wj bench/economics/results/pc-YYYYMMDD.json --md bench/economics/results/pc-YYYYMMDD.md
$WJ run scripts/wj/economics_report.wj bench/economics/results/pc-YYYYMMDD.json --gate

# wall-clock of the verify loop, alternating, median by eye (D2 in seconds)
for i in 1 2 3 4 5; do
  /usr/bin/env time -p $WJ run --interpret bench/agent-dx/solutions/wj/t011_lru_cache/main.wj < /dev/null > /dev/null
  /usr/bin/env time -p sh -c 'rustc --edition 2021 -C opt-level=0 -o /tmp/r bench/agent-dx/solutions/rust/t011_lru_cache/main.rs && /tmp/r < /dev/null > /dev/null'
done
rm -rf /tmp/econ-target-pc
```

Commit `bench/economics/results/pc-*.{json,md}` and add the PC rows above, labelled "PC, quiet".

The "after" numbers of the later steps, on the integration head (wave 3, cf73e7fe4 or later), same session:

```
# D2: record the PC's own verify-loop ratio (a new meter entry; the Mac entry is never compared), then check it
$WJ run scripts/wj/economics.wj --d2 --wj $WJ --runs 5 --write bench/economics/d2_baseline.json
$WJ run scripts/wj/economics.wj --d2 --wj $WJ --runs 5 --check bench/economics/d2_baseline.json

# C2 lean manifest + lean runtime, and the LTO default: cold and incremental release builds per program, LTO on/off
$WJ run scripts/wj/economics.wj --lto-study --wj $WJ --label pc-$(date +%Y%m%d) --runs 5 \
    --only dx/t001_gcd_lcm,dx/t009_rpn_eval,dx/t011_lru_cache,dx/t013_bst,dx/t018_word_freq,dx/t030_bank_ledger,dx/t043_channel_pipeline,dx/t048_wc_clone,interp/fib30,interp/hashmap,parity/string_build

# C4: overflow checks on/off per kernel and the bounded_counter clawback, with callgrind instruction counts
cargo build --release --example optimize_rs
$WJ run scripts/wj/economics.wj --profiles --wj $WJ --label pc-$(date +%Y%m%d) --runs 3 --instr callgrind
$WJ run scripts/wj/economics.wj --clawback --wj $WJ --optimize-rs target/release/examples/optimize_rs --label pc-$(date +%Y%m%d) --runs 3 --instr callgrind
$WJ run scripts/wj/economics_report.wj bench/economics/results/pc-YYYYMMDD-profiles.json --md bench/economics/results/pc-YYYYMMDD-profiles.md
$WJ run scripts/wj/economics_report.wj bench/economics/results/pc-YYYYMMDD-clawback.json --md bench/economics/results/pc-YYYYMMDD-clawback.md
```

## Reproduce the Mac numbers

```
<slot> env WJ_ECOSYSTEM=<eco> wj run --interpret scripts/wj/economics.wj -- --wj $(command -v wj) --target <empty dir> --label mac-YYYYMMDD
wj run --interpret scripts/wj/economics_report.wj -- bench/economics/results/mac-YYYYMMDD.json --md bench/economics/results/mac-YYYYMMDD.md
wj run --interpret scripts/wj/economics.wj -- --static --check bench/economics/results/static.json   # D7 gate
wj run --interpret scripts/wj/economics_report.wj -- bench/economics/results/mac-YYYYMMDD.json --gate  # claimed wins hold
```
