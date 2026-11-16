---
status: accepted
---
# Economic efficiency: definition, metrics and win thresholds

- **Status:** Accepted as the measurement contract for roadmap gap #8 (2026-11-16)
- **Related:** [ADR-0006](../adr/0006-optimizer-fate.md) (optimizer fate), [ADR-0033](../adr/0033-interpreted-first-cost.md)
  (interpreted-first cost), [ADR-0037](../adr/0037-agent-cost-levers.md) (agent-cost levers),
  [WJ-PERF-01](../proposals/rfcs/WJ-PERF-01-economic-efficiency.md) (the original aspiration),
  [optimization/](../optimization/) (the live post-codegen optimizer), [RESULTS.md](RESULTS.md) (current numbers),
  [ADVANTAGES.md](ADVANTAGES.md) (where Windjammer can win), [COMMANDS.md](COMMANDS.md) (`wj economics`, `wj optimize`),
  [PLAN.md](PLAN.md) (what happens before beta)

## Context

WJ-PERF-01 promised "60-70% total cost reduction versus Rust" without a definition of cost or a measurement. ADR-0006
removed the unmeasured claims. The owner's directive (2026-11-16) is to earn them: define economic efficiency so that it
can be measured, measure it against hand-written Rust, and turn the places where Windjammer can be cheaper into gated,
reproducible advantages.

## Definition

**The economic cost of a program is the total cost of getting a correct program and keeping it running:**

```
cost(program) = agent cost to reach a correct program            (D1: tokens, turns, USD per passing program)
              + verify-loop cost per iteration x iterations       (D2: source edit -> verified output)
              + compile cost (warm and cold)                      (D3: instructions/CPU, dependency builds)
              + memory (compile and run)                          (D4: peak RSS)
              + run cost x executions                             (D5: instructions/CPU of the shipped binary)
              + distribution (binary size)                        (D6: stripped bytes)
              + maintenance (what the next reader must read)      (D7: tokens, characters, lines, ceremony)
```

Each term is measured per program against a **hand-written, idiomatic Rust twin** that produces byte-identical output on
the same input. "Windjammer is more economical" is a statement about a named dimension and a named suite, with a ratio
(Windjammer / Rust; below 1.00 is cheaper) and a threshold, never a blanket percentage.

No single blended dollar figure is published. Prices differ by orders of magnitude between dimensions (an agent turn
costs about USD 0.01; a CPU-second about USD 0.00001), and a weighted sum would hide that the agent and verify terms
dominate everything else for a program built once and run a few times, while D5 dominates for a service run billions of
times. Users weigh the dimensions for their workload; the report gives every dimension.

## Programs

| Suite | Programs | Windjammer side | Rust side | Input |
|---|---:|---|---|---|
| `dx` | 60 | `bench/agent-dx/solutions/wj/<task>/main.wj` | `bench/agent-dx/solutions/rust/<task>/main.rs` (written from the spec by separate authors, graded by the same hidden oracle) | first visible sample case (`tasks/<task>/samples.json`) |
| `parity` | 10 | `benchmarks/rust_parity/wj/*.wj` | `benchmarks/rust_parity/rust/*.rs` (idiomatic, ADR-0006 work) | none (compute kernels) |
| `interp` | 8 | `bench/interp/*.wj` | `bench/economics/twins/*.rs` (written for this suite, idiomatic: iterators, `entry()`, `write!`, `&mut [T]`) | none |

78 programs. Twins were written only where none existed (the 8 `interp` kernels). A twin is honest when it is what a
competent Rust programmer writes first: idiomatic std, no unsafe, no micro-optimisation the Windjammer program does not
also get, the same algorithm. A twin that loses on purpose (strawman) or wins with tricks the Windjammer side cannot
express (hand SIMD) is a defect: file it and fix the twin.

Known bias: the `dx` reference solutions in Windjammer were written in an older, verbose style (index `while` loops);
their token count is 1.27x the Rust references (`cost.write.ref_token_ratio_wj_over_rust`), while programs agents
actually wrote in pilot 6 were 0.88x (`cost.write.pilot6_token_ratio_wj_over_rust`). D7 therefore reports both.

## Dimensions

Every measured command runs under `/usr/bin/time -l` (macOS: instructions retired, cycles, peak RSS, CPU) or GNU
`time -v` with `perf stat -e instructions:u,cycles:u` (Linux). A count is the smallest of N runs (noise only adds).
Cross-program ratios use **cycles** and **CPU seconds**, not instructions: IPC differs several-fold between programs
(the interpreter retires 2.6 instructions per cycle, rustc far fewer), and only CPU seconds include child processes
(rustc's linker, cargo's rustc). Every run's output must equal the expected output (the
sample's stdout, or the Rust twin's output for kernels); a program with any mismatch is excluded from the summaries and
listed as a problem. The runner is `scripts/wj/economics.wj` (interpreted Windjammer); the renderer is
`scripts/wj/economics_report.wj`.

**Loaded-box rule** (see the measurement-discipline note in RESULTS.md): on a machine with load above its core count,
wall-clock is noise. Decisions use cycles and CPU seconds of the measured process (and peak RSS, which is stable
under load). Wall-clock numbers
are taken only on a quiet machine (the Linux PC), with alternating runs, and are labelled with the load.

### D1 Agent cost

| | |
|---|---|
| Metric | USD per passing trial (`cost / pass@1`), turns, input and output tokens; the same tasks, model and harness |
| Measuring script | `bench/agent-dx` harness (pre-existing, Python; recorded fallback in its README) runs the pilots; `economics.wj` reads every `bench/agent-dx/results/*/report.json`; ADR-0037's offline proxies (`bench/agent-dx/friction/cost_proxies.wj`) predict it between pilots without LLM spend |
| Rust baseline | arm `R` of the same pilot (agent writes Rust, verifies with `rustc`) |
| Win | Windjammer cost per passing program at most **0.90x** Rust with the paired 95% CI upper bound below 1.00, on at least 10 tasks, and pass@1 not lower |
| Parity | CI includes 1.00 |

### D2 Verify loop (edit to verified output)

| | |
|---|---|
| Metric | CPU seconds (children included) and cycles from source text to the program's verified output on the sample input |
| Windjammer | `wj run --interpret main.wj` (interpreted-first loop); also `wj build --no-cargo` + rustc debug + run (compiled loop) |
| Rust baseline | `rustc -C opt-level=0 -C debuginfo=0 main.rs` + run (what an agent's `rustc` loop does) |
| Win | geometric mean at most **0.50x** on `dx` with zero output mismatches (the interpreted-first loop is the language's structural advantage: no compile step) |
| Note | in a pilot this term appears as tool-wait seconds; pilot 6 measured 2.1 s (R) against 11.3 / 17.4 s (U / V2) because agents ran compiled builds, not the interpreter (ADR-0037) |

### D3 Compile cost

| | |
|---|---|
| Metrics | (a) transpile + `rustc -O3` instructions with identical flags and a warm dependency cache; (b) transpile + `cargo build --release` as shipped (LTO, overflow checks) CPU s, warm; (c) one-off cold build of the generated crate's dependencies (CPU s, peak RSS, crate count) |
| Rust baseline | `rustc -C opt-level=3 main.rs` (std only, no dependencies) |
| Win | (a) at most **1.00x**; (c) no dependency build for a program that uses only std-backed features |
| Parity | (a) within 1.10x; transpile itself at most 10% of the rustc share |

### D4 Memory

| | |
|---|---|
| Metrics | peak RSS of the release compile and of the binary at run time |
| Win | run-time RSS at most **0.95x** on `parity`+`interp`; compile RSS at most 1.00x |
| Parity | run-time RSS within 1.05x |

### D5 Run cost

| | |
|---|---|
| Metric | instructions of the `-O3` binary (same flags as the twin) and of the shipped binary, on the program's input |
| Win | geometric mean at most **0.95x** on `parity`+`interp`, and no program above 1.10x |
| Parity | geometric mean within 1.02x |

### D6 Binary size

| | |
|---|---|
| Metric | stripped bytes, `-O3` same flags and shipped profile |
| Win | at most **0.95x**; parity within 1.02x |

### D7 Maintenance

| | |
|---|---|
| Metrics | lexical tokens (identifiers, literals, punctuation), non-whitespace characters, lines (no blanks, no comment-only lines), and *ceremony*: `&`, lifetimes, `.clone()`, `.to_string()`, `.to_owned()`, `.unwrap()`, `.as_str()`, `.iter()`, `as` |
| Rust baseline | the twin's source |
| Win | tokens at most **0.85x** and ceremony at most **0.50x** (on agent-written programs and on the reference set) |
| Note | tokens are a proxy for what an agent reads and writes; a BPE token count is in ADR-0037's `cost.write.*` |

## Reproduce

Mac (loaded box; instruction counts are the decision metric):

```
cd <windjammer checkout>
<slot> env WJ_ECOSYSTEM=<windjammer-ecosystem checkout> wj run --interpret scripts/wj/economics.wj -- \
    --target <empty dir for the cargo cache> --label mac-$(date +%Y%m%d)
wj run --interpret scripts/wj/economics_report.wj -- bench/economics/results/mac-YYYYMMDD.json --md bench/economics/results/mac-YYYYMMDD.md
```

Linux PC (quiet box; adds wall-clock and `perf` instruction counts): see RESULTS.md, "PC commands".

## Consequences

- Every advantage claim in public docs must name a dimension, a suite and a ratio from a committed results file, and
  the gate that keeps it true (PLAN.md lists them).
- WJ-PERF-01's percentages stay withdrawn until a row in RESULTS.md meets the win threshold above.
