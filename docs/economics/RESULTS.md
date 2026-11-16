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
| D5 | run, `-O3` same flags, cycles | 1.00 | 1.20 | 1.12 | win <= 0.95, parity <= 1.02 | parity on dx, loss on kernels |
| D5 | run, shipped vs twin, instructions | 1.02 | 1.31 | 1.37 | | loss (overflow checks) |
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

```
# once: toolchain and checkouts (any directory names)
git clone <windjammer> wj && git clone <windjammer-ecosystem> eco
cd wj && git checkout <integration head>
cargo build --release --bin wj
export WJ_ECOSYSTEM=$PWD/../eco WJ=$PWD/target/release/wj
sudo apt-get install -y time linux-tools-$(uname -r)     # GNU time -v and perf (instruction counts)
sudo sysctl kernel.perf_event_paranoid=1                  # perf stat as a normal user

# quiet check: load below 1 before each timed step, nothing else running
uptime

# the suite (instructions from perf, CPU and wall from GNU time); an empty --target measures the cold build
$WJ run scripts/wj/economics.wj --wj $WJ --target /tmp/econ-target-pc --label pc-$(date +%Y%m%d) --runs 5
$WJ run scripts/wj/economics_report.wj bench/economics/results/pc-YYYYMMDD.json --md bench/economics/results/pc-YYYYMMDD.md

# wall-clock of the verify loop, alternating runs (D2 in seconds), for the record
for i in 1 2 3 4 5; do
  /usr/bin/time -f "wj %e s" $WJ run --interpret bench/agent-dx/solutions/wj/t011_lru_cache/main.wj < /dev/null > /dev/null
  /usr/bin/time -f "rustc+run %e s" sh -c 'rustc --edition 2021 -C opt-level=0 -o /tmp/r bench/agent-dx/solutions/rust/t011_lru_cache/main.rs && /tmp/r < /dev/null > /dev/null'
done
rm -rf /tmp/econ-target-pc
```

Commit `bench/economics/results/pc-*.{json,md}` and update the tables above with the PC numbers labelled "PC,
quiet".

## Reproduce the Mac numbers

```
<slot> env WJ_ECOSYSTEM=<eco> wj run --interpret scripts/wj/economics.wj -- --wj $(command -v wj) --target <empty dir> --label mac-YYYYMMDD
wj run --interpret scripts/wj/economics_report.wj -- bench/economics/results/mac-YYYYMMDD.json --md bench/economics/results/mac-YYYYMMDD.md
wj run --interpret scripts/wj/economics.wj -- --static --check bench/economics/results/static.json   # D7 gate
```
