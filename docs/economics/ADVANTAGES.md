---
status: accepted
---
# Where Windjammer can be cheaper than hand-written Rust (and where it is not yet)

- **Date:** 2026-11-16. Numbers: [RESULTS.md](RESULTS.md) (measured on the integration head named there).
- **Rules:** a pass or feature listed here is admitted under [PASSES.md](../optimization/PASSES.md): a written
  soundness argument naming the facts it relies on, a mutant the tests catch, and a ratchet. "Expected win" is an
  estimate until RESULTS.md has the row; each candidate names the row that will prove it.

Windjammer compiles to Rust, so on run time the honest ceiling against an *idiomatic* twin is parity: the same LLVM
sees the same loops. The places where Windjammer can be strictly cheaper are the ones where it decides something a Rust
programmer has to decide by hand (and often decides expensively), or where it can skip work Rust cannot skip:

1. **No compile in the verify loop** (the interpreter), which no Rust workflow has.
2. **Ownership decided by a solver** with whole-program facts: borrow where a human writes a clone to get past the
   borrow checker, move at the last use, reuse buffers across iterations.
3. **The build is owned by the compiler**: dependency set, release profile and features are chosen from what the
   program uses, not from defaults.
4. **Less to write and read**: no lifetimes, borrows or conversions in the source.

## Summary

| # | Candidate | Dimension | Today (RESULTS.md) | Expected after | Effort | Verdict |
|---|---|---|---|---|---|---|
| C1 | Interpreter-first verify loop as the default agent loop | D2, D1 | **measured win: 0.24x CPU (0.62x cycles) on 58/58 `dx` programs**; 6x loss on compute kernels | keep <= 0.5x on `dx`; agents actually use it (pilot tool-wait) | 1-2 agent-days (L5 of ADR-0037 + default) | **Top 3** |
| C2 | Pay-for-what-you-use runtime (no dependency build unless used) | D3, D4, D6 | **measured loss**: cold build 30.8 s CPU / 450 MB / 59 crates before the first program (twin: none); dx binaries 1.19x; dx compile 1.36x cycles (runtime under LTO) | 0 crates for 18/78 programs, at most 2 for the rest; dx size and compile to parity | 2-3 agent-days | **Top 3** (removes the largest loss) |
| C3 | Solver-decided borrow of read-only aggregate parameters + last-use move | D5, D4 | 156 `.clone()` in the generated `dx` code vs 12 in the twins | at most 20 | 3-5 agent-days (ADR-0001 solver work) | **Top 3** (closes the run-time gap) |
| C4 | Release profile chosen by the compiler (LTO, one codegen unit, strip) | D5, D6 | shipped profile already has LTO; see RESULTS D5/D6 | smaller and faster than `cargo build --release` defaults; equal to a hand-tuned profile | 0.5 agent-day | do with C2 |
| C5 | Auto-arena scratch reuse (P8, default off) | D5, D4 | off | fewer allocations than the idiomatic twin when the twin allocates per iteration | 1 agent-day (benchmark + default flip) | next |
| C6 | Ceremony-free source | D7, D1 | ceremony 0.46x (measured, win) ; tokens 1.06x on the verbose reference set | tokens at most 0.85x on agent-written programs | ongoing (std surface, sheet) | holding |
| C7 | Smaller generated Rust (less for rustc to compile) | D3 | generated `dx` Rust has 1.76x the twin's lines | at most 1.2x | 2-3 agent-days, spread | after C3 |
| C11 | Idiom lowering in codegen: fill loops to `vec![x; n]`/`resize`, `push_str("${..}")` to `write!`, get-then-insert to `entry` | D5 | **measured loss** on 6 kernels: vec_fill 2.79x, strings 2.16x, sieve 1.49x / 1.22x, hashmap 1.42x, string_build 1.34x cycles | parity (<= 1.02x) on every kernel | 3-4 agent-days | next after C2 |
| C12 | Single-file build reads only reached peers' metadata | D2 compiled, D3 | **measured**: 1.26 s CPU inside this repository vs 0.09 s for the same file alone (docs/known-issues/build-scans-enclosing-tree-for-metadata.md) | location-independent | 1 agent-day | quick fix |
| C8 | Defer-drop of large values | D5 latency | live (`--defer-drop auto`, 100 KB) | latency only; no instruction win | - | keep, not a headline |
| C9 | Small-string / arena types chosen by ownership facts | D5, D4 | not started | unknown | weeks | research |
| C10 | Overflow-check elimination from solver ranges | D5 | shipped binaries keep overflow checks (spec) | the twin has none in release, so this is a loss to close | 3+ agent-days | after C3 |

## C1 Interpreter-first verify loop

- **Expected win:** the D2 ratio. An agent's iteration is "edit, verify, read the result". Rust has to compile
  (`rustc` debug: hundreds of millions of instructions even for a 30-line program); `wj run --interpret` parses,
  checks and runs (interpreter startup is 69M instructions on macOS, `bench/interp/instr_baseline.json`).
- **Evidence:** RESULTS.md D2: `wj run --interpret` costs 0.24x the CPU of `rustc` debug + run on the 58 clean `dx` programs (0.62x in cycles of the process alone; every one of the 58 is cheaper); on the compute kernels it costs 6x, and those are the programs a final compiled build is for. Pilot 6 shows the other side: tool-wait was 2.1 s for Rust and
  11.3 / 17.4 s for the Windjammer arms, because agents ran compiled builds (`wj run --verify`) on every call
  (ADR-0037 table, "the interpreted-first arm never used the interpreter").
- **Pass/feature:** ADR-0037 L5: one command that is the whole loop and is cheap: `wj run --verify` interprets first,
  stops on failure, and builds only once at the end (or speculatively in the background, ADR-0033 `--speculate`).
  The agent sheet names that command only.
- **Soundness:** the interpreted result is trusted only because of the agreement gates (`compiled_agreement`,
  `engine_parity`, silent divergence 0/90 in pilot 6); the final compiled build still runs before "done".
- **Gate:** `economics.wj` D2 ratio on `dx` recorded in `bench/economics/results/` and checked (PLAN.md step 2); the
  agreement suites stay green.

## C2 Pay-for-what-you-use runtime

- **Expected win:** removes a one-off cost every new checkout, CI cache and agent sandbox pays: the generated
  `Cargo.toml` always depends on `windjammer-runtime` (plus `serde` with derive and `smallvec`), and the runtime's
  non-optional dependencies (serde_json, url with the ICU stack, uuid, rand, mime_guess, anyhow, thiserror, async-trait,
  base64, hex, dirs, bumpalo, ...) compile 69 crates. Measured: RESULTS.md "cold dependency build".
- **Evidence (this survey, 78 generated programs):** 18 never reference `windjammer_runtime`; the other 60 reference
  only `io`, `fs`, `env`, `process`, `thread`, `sync`, whose sources import nothing but `std`; none uses `serde` or
  `smallvec`. The twin pays nothing.
- **Pass/feature:** (a) emit the runtime/serde/smallvec dependency lines only when the generated code references them
  (`src/cargo_toml/`); (b) put the runtime's non-optional dependencies behind features enabled by
  `runtime_features.rs` from the modules the code references, so `io`/`fs`/`env`/`process`/`thread`/`sync` compile
  with zero dependencies.
- **Soundness:** removing an unreferenced crate cannot change the program: Rust links only crates the code names; a
  dependency that was needed fails to compile (caught, not silent). Global side effects of a crate (a
  `#[global_allocator]`, a panic hook installed at start) would be the exception: the runtime declares neither for
  these modules (check is part of the gate).
- **Gate:** a ratchet on the cold-build crate count of three fixed programs (no runtime use; io only; io + fs), and the
  full `dx` suite still passing its oracle in compiled mode.

## C3 Solver-decided borrow of read-only aggregates, and last-use move

- **Expected win:** allocation and copy work the twin does not do. The generated `dx` code has 156 `.clone()` against 12
  in the twins.
- **Evidence:** `wj explain-ownership` over the 78 programs: 298 parameters, 150 emitted owned; 40 owned non-Copy
  parameters have no consuming use (not stored, returned or mutated), each tagged `OWN-ANALYZER-SEED` ("analyzer
  pre-decided ... ADR-0001 says the solver should decide"). Examples: `fn skip_level(people: HashMap<string, Person>, ..)`
  is called as `skip_level(people.clone(), ..)` (a whole map copied per query, t033); `fn less(a: Entry, b: Entry)` is
  called as `less(self.data[i].clone(), self.data[parent].clone())` in a heap sift loop (t012);
  `pair_before(v[j].clone(), v[j - 1].clone())` in a sort loop (t018, t055). Also 6 `D-CLONE-OUTSIDE-SOLVER` clones
  (codegen decided), and `let sku = p[1].clone(); inv.insert(sku.clone(), ..)` (the second clone is a last use).
- **Pass/feature:** in the IR solver (not the text optimizer): a parameter whose only uses are reads (field reads,
  `&`-taking calls, comparisons) gets `Ref` when it is not Copy; call sites then pass `&x` and the clone disappears.
  Last-use move: a `.clone()` whose source is dead afterwards and not borrowed at that point becomes a move.
- **Soundness:** `&T` for a never-moved, never-mutated parameter is accepted by rustc whenever the owned version was
  (the body only reads); behaviour differs only in when the clone is dropped, so the rule is refused when `T` (or a
  field type) has a user `impl Drop`. Last-use move needs liveness (no later read, no live borrow), which the
  solver has and a text pass does not (PASSES.md "Considered and rejected").
- **Gate:** the clone ratchet (`tests/optimizer_ratchet_test.rs` corpus plus the `dx` generated count from
  `economics.wj`), a mutant that borrows a stored parameter (must fail to compile in the fixture suite), and D5 rows.

## C4 Release profile chosen by the compiler

- **Expected win:** `cargo build --release` defaults (16 codegen units, no LTO, symbols kept) are what most Rust
  programs ship with. Windjammer writes the profile itself: `lto = true` today; adding `codegen-units = 1` and
  `strip = "symbols"` is cheap. Binary size and run time then beat the default-profile twin and equal a hand-tuned one.
- **Evidence:** RESULTS.md D6 `size_shipped` and D5 `run_shipped_instr` against the twin at rustc `-O3` defaults.
- **Soundness:** LTO, codegen units and stripping symbols do not change behaviour; stripping symbols removes function
  names from `RUST_BACKTRACE` output, so it is `strip = "debuginfo"` unless `--release-small`. `panic = "abort"` is
  excluded: it changes the exit status of a panic (101 vs 134), which the spec fixes.
- **Gate:** D6 ratio recorded; the shipped run passes the same output checks.

## C5 Auto-arena scratch reuse (P8)

- **Expected win:** a twin that allocates a fresh `String`/`Vec` per iteration (the usual first draft) does one
  allocation per iteration; P8 hoists the buffer and `clear()`s it. Zero per-iteration allocations is strictly better
  than that twin. Our `interp/strings` twin already reuses its buffer, so the suite needs a kernel where the idiomatic
  twin allocates in the loop (line-by-line parsing).
- **Soundness:** PASSES.md P8 (written, mutant `ScratchIgnoresMoves`).
- **Gate:** the kernel's allocation count (counting allocator, as in `rust_parity_run.wj`) and the P8 tests; then flip
  the default.

## C6 Ceremony-free source

- **Measured:** ceremony (`&`, lifetimes, clone, to_string, unwrap, as, iter) is **0.46x** the twins (geometric mean,
  68 programs where both are non-zero): a win under DEFINITION.md's threshold. Tokens are 1.06x on the reference set,
  whose Windjammer programs are written in an older, workaround-heavy style (index loops, hand insertion sorts,
  explicit `.clone()` in the source); on programs agents actually wrote in pilot 6 the ratio is 0.88x
  (`cost.write.pilot6_token_ratio_wj_over_rust`).
- **Not yet a win:** D1. Agent cost per passing program: pilot 6 1.17x / 1.21x Rust (both CIs include parity);
  pilot 8 (coordinator's summary, raw data not in this repository) about 1.7x, with 3x turns and 1.5x output tokens
  and programs equal in characters. The extra turns are friction (ADR-0037's corpus), not program length.
- **Next:** ADR-0037 L4 (open compiler bugs in the friction corpus) and L5 (C1). Rewrite the `dx` reference programs in
  current idiomatic Windjammer so D7 measures the language, not 2026-10 workarounds.

## C7 Smaller generated Rust

- **Evidence:** on `dx`, generated Rust has 5221 non-blank lines against 2964 for the twins (Windjammer source: 3938):
  150 `#[inline]`, 149 `let _ = v.push(..)`, 149 ` as ` casts (twins 34), 106 `.to_string()` (45), 54 `String::from`
  (4). Each is work rustc does per build (D3 `compile_release_instr`).
- **Pass/feature:** emission fixes in codegen (no `let _ =` on unit calls, fewer literal casts once widths are solved
  (ADR-0028), `#[inline]` only where P6 would keep it).
- **Soundness:** each is the identity on behaviour; the existing optimizer fixtures and ratchet cover them.

## C11 Idiom lowering in codegen

- **Evidence (RESULTS.md, run per kernel, cycles):** `parity/vec_fill` 2.79x (index loop with `as u64` casts vs iterator
  `sum`), `interp/strings` 2.16x (`s.push_str("${i % 10},")` builds a temporary String per iteration; the twin writes
  with `write!`), `parity/sieve` 1.49x and `interp/sieve` 1.22x (a push loop vs `vec![true; n]`), `interp/hashmap` 1.42x
  (the kernel calls `get` then `insert`; the twin uses `entry`, so part of this is how the kernel was written),
  `parity/string_build` 1.34x.
- **Pass/feature:** emission rules with the shape known at codegen: interpolation appended to a String binding lowers to
  `write!` (the P5 argument extended to `push_str`); a `0..n` loop whose only effect is `v.push(c)` with a loop-invariant
  `c` lowers to `v.resize(len + n, c)`; `match m.get(&k) { Some(v) => *v, None => d }` followed by `m.insert(k, f(..))`
  lowers to `entry` when the map is not read in between.
- **Soundness:** each is an identity on observable behaviour under preconditions of the P4/P5 kind (no early exit, no
  aliasing of the target, `Clone` of a Copy or literal value); each gets a PASSES.md section and a mutant before it is
  on.
- **Gate:** the per-kernel cycle ratios in `bench/economics/results/` (a ratchet per kernel, 3% slack, the
  `interp_instr.wj --check` pattern).

## C8-C10

- **Defer-drop** never fires on this suite (no value above 100 KB is dropped on a hot path); it lowers latency, not
  instructions, so it is not an economic headline.
- **Small-string/arena types** would beat idiomatic Rust (which uses `String` everywhere), but need a runtime type,
  conversions at API boundaries and a benchmark showing the conversions do not eat the win. Research item.
- **Overflow checks** are on in shipped Windjammer binaries because the spec says overflow panics; the twin's release
  build has none. Ranges from the numeric solver could prove most loop counters safe and emit unchecked arithmetic
  for them. Until then, D5 `run_shipped_instr` carries that cost honestly.

## Where Windjammer loses today

Stated plainly, from RESULTS.md: the cold dependency build (C2: 31 s CPU before the first program), runtime linked
into every `dx` binary (C2: 1.19x size, 1.36x release compile), six kernels slower by 1.1x to 2.8x (C11), overflow
checks in shipped binaries (C10: up to 1.5x on arithmetic kernels), clones in read-heavy helpers (C3), the interpreter on
compute kernels (6x; those are compiled), and agent cost per passing program (C6/D1). None is structural; each has a
named fix above.
