---
status: measured
---
# Economics results: Windjammer against hand-written Rust

Definitions, programs and thresholds: [DEFINITION.md](DEFINITION.md). Interpretation and fixes:
[ADVANTAGES.md](ADVANTAGES.md). Raw data: `bench/economics/results/`.

Every number below is labelled **measured** (a command in this repository produced it, the command is named) or
**reported** (from a source outside this repository's data). Nothing is estimated in this file.

## Conditions (read before quoting)

- Machine: macOS arm64 (8 cores), shared with other build/test jobs. Load averages during the runs are recorded in
  the results JSON (`env.load_start`, `env.load_end`); they were far above the core count (17 to 125). Under that
  load, wall-clock is noise; the decision metric is **instructions retired** (`/usr/bin/time -l`, the hardware counter
  of the measured process), which load does not move, and **peak RSS**. CPU seconds are given where the measured
  command spawns children (cargo), because instructions retired counts only the direct process.
- Compiler: the installed `wj` named in each results file (`env.wj_version`); rustc: `env.rustc` (nightly). A run
  on the integration head itself is PLAN.md step 1 (PC).
- Every row passed its output check (the program's stdout equals the expected output) unless listed under
  "Problems".

## D2-D6 (pending)

The full Mac run of `economics.wj` over the 78 programs is queued behind the build slots; its tables replace this section. D7 (static, deterministic) is below.

## Cold dependency build of a generated crate (measured)

`wj build --no-cargo` of `dx/t001_gcd_lcm`, then `cargo build` and `cargo build --release` into an empty target
directory (`CARGO_BUILD_JOBS=2`), `/usr/bin/time -l`:

| profile | CPU s (user+sys) | peak RSS | crates compiled |
|---|---:|---:|---:|
| debug | 28.9 | 429 MB | 69 |
| release | 36.0 | 455 MB | 69 |

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
