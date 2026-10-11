# Backend conformance system

`tests/backend_conformance_test.rs` (harness code in `tests/common/conformance/*.rs`, corpus in
`tests/conformance_programs/`) checks that the Rust, Go and JavaScript backends compute the **right** answer, not merely
the **same** answer. Agreement between backends is never accepted as evidence: three backends can share one wrong
semantics, and a program can print a constant.

## Oracles (all independent of every Windjammer backend)

| oracle | where | used for |
|--------|-------|----------|
| known-answer vectors | `// expect:` lines in a program | published vectors (CRC-32, FNV-1a, SHA-256, PCG32, Park-Miller MINSTD, RFC 4648 base64, UTF-8 encodings, harmonic numbers, factorials, Collatz record, textbook determinants) and hand-derived semantic tables (integer casts, division signs, float formatting) |
| reference model | `<name>.ref.py` + committed `<name>.vectors/<seed>.out` | seeded programs; the committed vector is *derived* by `regen_vectors.py` and the test `reference_vectors_are_derived_from_models` re-runs the model whenever `python3` exists and fails on drift |
| generator evaluator | `tests/common/conformance/gen_ast.rs` | generated programs: a second, independent implementation of the intended semantics (wrapping i64, truncating division, Copy-struct value semantics) |
| the interpreter | `src/interpreter` via `wj run --interpret` | a fourth, **explicitly weaker** voice. Reported by `interpreter_oracle_report`, never authoritative |

A divergence report names which voice disagrees with which (`oracle != go`), and gives the first diverging trace line.

## Anti-faking

* Seeded programs contain `__SEED__`; the harness substitutes each of >= 5 seeds, recompiles, and compares with the
  reference model's output for that seed. The program implements its own PRNG and checksums (FNV-64 over every
  intermediate value of 10^4-10^5 steps), so a literal `println` cannot pass.
* `cheating_program_is_rejected`: `selftest/cheat_constant_for_seed_1.wj` prints the correct constant for seed 1 and must
  FAIL on seed 2.
* `injected_backend_bugs_are_all_caught`: mutation self-test. Known defects are injected into the *generated* artifacts
  (text edits only; no compiler flags) and every one must be caught. Seeded list (kill rate must be 100%):

| mutant | models | targeted programs |
|--------|--------|-------------------|
| js-i64-via-f64 | i64 arithmetic through f64 | 04, 46 |
| js-struct-copy-by-reference | struct assignment aliases | 10, 43 |
| js-int-division-as-float | integer division not truncated | 01 |
| js-u64-as-signed | u64 wrapping signed | 40 |
| js-no-overflow-check | i32 overflow ignored | 60 |
| go-i64-as-i32 | 64-bit ints narrowed | 02, 46 |
| go-u64-as-i64 | unsigned treated as signed | 40, 45 |
| go-clone-shares-backing-store | deep copy -> shallow copy | 11, 43 |
| go-no-overflow-check | overflow wraps silently | 60 |

## Trace-level comparison

A trace is the ordered stdout lines (checkpoint hashes), the exit status and the stderr class (panic / clean / other).
Panicking programs declare `// exit: 101` and the stdout lines printed before the panic. First divergence is reported
with its line number.

## Semantic edge matrix

Each cell is at least one corpus program (number in brackets) or a `SUPPORT.toml` entry with a reason.

- [x] integer widths and wrapping/checked/saturating ops (41, 54, 57, 60, 90-94)
- [x] casts between widths, float->int saturation, NaN->0 (52)
- [x] division/modulo signs, `rem_euclid` (53)
- [x] shifts, bit ops, `count_ones`, rotates (54, 93, 94)
- [x] float formatting: NaN/inf/-0, exponent expansion, `{:.N}` half-even, f32 shortest (55, 101)
- [x] float math and rounding (56, 110)
- [x] strings and Unicode: byte `len`, `chars`, byte-offset `find`, slicing (59, 61, 95, 96)
- [x] value semantics: clone independence, mutate-through-callee, nested aggregates, copy structs (10, 11, 43, 62)
- [x] closures: by-value capture, counters, passing closures (63, 49)
- [x] enum/match: guards, nested patterns, struct variants (14, 15, 64, 50)
- [x] Option/Result/`?` (18-20, 51, 65, 66)
- [x] panics, exit codes (60, 68-71)
- [x] HashMap/HashSet semantics with sorted keys, integer and string keys (24, 44, 72, 73, 114)
- [x] sort/dedup/binary_search/retain/windows/chunks (42, 74); stability (102)
- [x] iterators: map/filter/fold/zip/enumerate/take/skip/chain/rev/step_by (30, 75, 76)
- [x] recursion depth (81), shadowing/blocks/if-expressions (77), tuples (78), consts (82)
- [x] traits and generics (27, 28, 79, 80)
- [x] std shims (110-113)
- [ ] async/spawn ordering: ABSENT on Go/JS (`SUPPORT.toml`); the AST nodes `AsyncCall`/`SpawnCall` are rejected with an explicit error
- [ ] labeled loops, struct update syntax: not representable in the Windjammer AST today (language gap, not a backend gap)

## Feature coverage gate

`FEATURES.toml` lists AST node kinds and std method names that must appear in the corpus. The harness parses every
program, collects kinds from the AST (`coverage.rs`) and fails if a listed feature is unexercised.

## Unsupported is explicit (ratchet)

`SUPPORT.toml` lists `(program, backend, reason)` pairs known to fail. A listed pair must fail; if it starts passing the
test fails until the entry is removed, so the file only shrinks. Unlisted failures are test failures.

## Toolchains

Missing `go`/`node`/`rustc`/`python3` produce a loud `CONFORMANCE-SKIP:` line per tool and the summary table counts skips.
CI sets `WJ_REQUIRE_TOOLCHAINS=1` (read in `tests/common/conformance/toolchain.rs`, never by the compiler) to turn skips into failures.

## Generative differential testing and metamorphic relations

`gen_build.rs` is a seeded grammar-based generator over a UB-free `i64` subset (wrapping ops, literal divisors, shifts,
`Vec`/`HashMap`/Copy-struct mutation, loops, branches). Each program prints a checkpoint trace and is compared with the
generator's own evaluator on every backend: 200 programs by default, 5,000 with `--features conformance_soak`. A mismatch
is shrunk by delta debugging (`shrink.rs`) and printed as a minimal program plus its first diverging checkpoint.
Metamorphic tests re-print the same AST after semantic-preserving transformations (rename locals, reorder independent
statements, wrap in blocks, split into temporaries, extract through a helper) and require identical traces.

## Adding a case

1. Create `tests/conformance_programs/NN_name.wj`. Known-answer: add `// expect:` lines (and `// exit: N` for panics).
   Seeded: use `__SEED__`, write `NN_name.ref.py` (reads the seed from `argv[1]`, prints the expected stdout) and run
   `python3 tests/conformance_programs/regen_vectors.py`.
2. If a backend cannot run it, add a `[[unsupported]]` entry to `SUPPORT.toml` with the missing capability as the reason.
3. Make sure any new method/AST kind is listed in `FEATURES.toml`.
4. `cargo test --release --test all backend_conformance`.
