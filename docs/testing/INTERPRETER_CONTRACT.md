# The interpreter's contract

`wj run --interpret` exists so that an edit-run loop takes milliseconds instead of a compile. That is only worth
something if you can trust what it prints. This page states exactly what is promised and how it is checked.

## The promise

For every program that `wj run --interpret` accepts, its standard output, its exit status and the source location of
its panic equal those of the same program built and run by the compiled backend, except for the divergences listed in
`tests/drift_corpus/known_divergences.txt`.

Three things follow.

- **A refusal is not a divergence.** The interpreter refuses (exits before running) what `wj check` rejects, and it
  also refuses ownership errors that rustc would report (ADR-0030, WJ0007). Integer widths the compiled build would mix
  (a backend defect, ADR-0033) are an error in `wj check` and a warning before an interpreted run. Refusing early is the
  promise working.
- **The ledger is the only escape hatch and it only shrinks.** A divergence that is not in the ledger fails the drift
  gate; a ledgered one that is fixed fails the gate until its line is deleted. The ledger has a ceiling
  (`tests/drift_corpus/ledger_ceiling.txt`) that `tests/drift_contract_test.rs` enforces in the default test run, so
  a divergence cannot be added without a visible, deliberate edit.
- **Documented differences are listed here, not hidden.** See the table below.

## What is not promised

| Area | What differs | Why |
|---|---|---|
| Speed, memory, stack depth | the interpreter is slower on hot numeric code and has its own recursion limit (ADR-0020) | it is an interpreter |
| `HashMap` / `HashSet` iteration order | the interpreter iterates in key order (`--hash-seed N` shuffles reproducibly); Rust's order is unspecified | decision D6: Rust's convention. Programs must not depend on the order; the drift probes print sorted keys |
| Real-time clocks, randomness, network | the interpreter can run with `--deterministic-time`, `--simulate`, `--mock-http` | non-determinism cannot be compared |
| Thread interleaving | the interpreter schedules deterministically (`--sched-seed`); a data-race-free program that joins its threads gives the same result | the compiled schedule is the OS's |
| Programs the compiled backend rejects | the interpreter may run a program whose generated Rust rustc rejects (code-generation defects). Those are compiler bugs; each is a ledger line with a minimal repro in `docs/drift/findings.md` | the interpreter is the reference semantics, the compiled backend is being fixed toward it |

## How it is checked

`scripts/wj/drift_gate.wj` (see [DRIFT.md](DRIFT.md)) runs, on every merge:

1. the engine parity corpus (tree walker == resolved engine, with and without machine-code loops);
2. the differential corpus: every test of a pinned snapshot of the ecosystem packages and apps, the repository's
   conformance and example programs, and the value-position, move-shape, probe and generated programs, each under the
   walker, the resolved engine and the compiled backend (stdout, exit status, panic location, and an independent
   `// expect:` oracle where a spec provides one);
3. the ledger comparison above.

`scripts/wj/drift_fuzz.wj` runs the generators over a long random seed range for a nightly report.

## The ledger today

The ledger lists, per case, what each engine produced. Classes at the time of writing: compiled-backend code-generation
defects (integer literal widths, `Box<dyn Fn>`, nested `Vec` indexing, `HashMap::get` with a loop variable, closure
parameters), ecosystem apps whose compiled build is red, and the interpreter-side divergences: none known between
the two engines. The counts are produced by the gate, not typed here.

## Verdicts by class of observable behaviour

See the verdict table in [DRIFT.md](DRIFT.md#verdicts-by-semantic-class).
