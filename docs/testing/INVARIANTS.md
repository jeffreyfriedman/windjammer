# Automated invariant gates

Every class of defect that was once found by inspection has a gate that fails for the whole class. A gate has an
**independent oracle** (not derived from the code under test), a **corpus** (repo programs plus generated ones),
**mutation self-tests** (the gate is fed deliberately broken input and must reject it) and, where the product is not
there yet, a **ratchet** file that may only shrink. All gates below run in `cargo test --release --test all` and in
`scripts/merge_gate.sh` (mandatory before a merge, see AGENTS.md).

## Promise, differential and capability gates

Why these exist: bugs were found only when someone asked ("does the eject command exist?", "is `wj check` really checking?").
Tests verified what implementers chose to test, mostly positive cases, never the product's promises; modes were
never compared with each other; capabilities existed in code but not in the CLI; two drivers drifted.

| Gate | Test | Oracle | Corpus | Self-tests | Ratchet |
|---|---|---|---|---|---|
| **Promise registry** | `promise_registry_test.rs` | the clap tree of `wj` (every subcommand and long flag), `docs/reference/errors.md` (every code), quoted README sentences; each needs a positive AND a negative acceptance case that passes through the installed binary (sandbox: binary alone, empty PATH, private HOME and TMPDIR). Which promises a case covers is read back from clap's parse of its arguments and from its expected output, never declared | `tests/promises/*.toml` (~150 cases) | always-exit-0 mutant binary must fail every negative case; audit rejects missing negatives, short flags map to long names, a new clap flag is detected, a loose ratchet is detected | `docs/promises.toml` `max_pending` (pending promises), `xfail` cases document known bugs and fail when fixed |
| Docs mention only real commands | same file | clap tree | inline/fenced `wj` commands in README, WHY, agent guide, llms.txt, AGENTS.md | unknown command/flag detected | none |
| Every documented code is explainable | same file | `wj explain CODE` exits 0 and names the code | `docs/reference/errors.md` | n/a | none |
| **Dead capabilities** | `capability_reachability_test.rs` | name-based call graph over `syn` ASTs from the entry points `wj`, `wj-lint`, the LSP and MCP servers (the legacy `windjammer` binary is not an entry point); test-only callers never count | all of `src/`, LSP and MCP crates | hand-labelled programs (methods need their type reached, trait-impl methods need the type, private items need their module, `include!` files, test-only use is dead) | `docs/reachability-allowlist.toml` (`[dead_files]` exact set with reasons, `[dead_pub_fns]` per-file counts); intended library API: `[library_api]` in `docs/MODULE_MAP.toml` |
| **Single driver** | `single_driver_parity_test.rs` | clap trees of `wj` and the legacy `windjammer` | both drivers | missing-subcommand detection | `LEGACY_ONLY` (today: `eject`) |
| **check vs build** | `check_build_differential_test.rs` | the build pipeline (`wj build --no-cargo` + `rustc --emit=metadata`) and labelled mutation operators (each operator's output MUST be rejected) | 16 operators x 3 seeds over a seeded base program, controls, the interpreter corpus | classification table, generator determinism, every operator changes and marks the program | `tests/check_vs_build_known.toml`: `check-miss.<op>`, `check-false-error.<op>`, `pipeline-accepts-invalid.<op>` counts, exact (CHECK_DIFF_BLESS=1 after a fix) |
| Interpreter vs compiled | `interpreter_differential_test.rs` (existing) | Python reference models + compiled Rust | `tests/interpreter_corpus` | print-faking program, source mutants | none |
| All backends vs independent oracle | `backend_conformance_test.rs` (existing, default suite) | known-answer vectors, Python models, generator evaluator | `tests/conformance_programs`, generated programs | injected backend bugs | `docs/backends/` support manifest |
| **Spec on every backend** | `spec_backends_test.rs` (the in-process interpreter run is `spec_conformance_test.rs`) | `// expect:` lines written from the language spec | `tests/spec/**` (lands with wt/lang-spec); interpreter, Rust, JS by default, + Go, TS with `WJ_SPEC_FULL=1` | WRONG vs REJECTED classification | `tests/spec_backend_status.tsv`; WRONG (ran, wrong output) is reported first and separately from REJECTED; `// xfail: Q-NNN` must exist in `docs/spec/OPEN_QUESTIONS.md` |
| **Codegen determinism** | `codegen_determinism_test.rs` | the output of the same build in another process (fresh hash seeds) with another rayon thread count | every verified example and conformance program, 3 runs (6 runs x 4 backends with `WJ_DETERMINISM_FULL=1`) | divergence detector | `tests/determinism_known.txt` (cause required) |
| Hash iteration in codegen paths | same file (`common/hash_iteration_lint.rs`) | text lint: iteration over `HashMap`/`HashSet` names in `src/codegen`, `src/ir`, `src/analyzer` | those trees | labelled snippet | `tests/determinism_hash_iteration.toml` (per-file counts, DETERMINISM_BLESS=1 to tighten) |
| **fmt / fix preserve behavior** | `fmt_fix_semantics_test.rs` | the interpreter run of the untouched program | interpreter corpus at 2 seeds + verified examples; for fix, each program is first rewritten into the deprecated surface (`println!`, `{}` templates, `vec![]`) | `unfix` produces the legacy surface | `tests/fmt_fix_known.txt` |
| **eject == build** (scaffold) | `eject_equivalence_test.rs` | `wj build --no-cargo` output, compared modulo comments | verified examples | comment-insensitive comparison | `EJECT_AVAILABLE` flips when wt/eject lands |
| Docs examples == interpreter | `docs_truth_test.rs` (existing) | interpreter output recorded in the docs | `sh,verify` fences | see docs/CONTRIBUTING-docs.md | docs-truth baselines |

### Heavy, opt-in tools

- **Coverage** (`scripts/coverage.sh`, cargo-llvm-cov): line/branch coverage of `src/` per top-level module by a test
  scope (default: the user-facing acceptance suites); baseline in the coverage JSON the script writes under `docs/generated/`, a module may not drop
  by more than 0.5 points. The report lists the biggest uncovered user-facing files. The installed-binary sandbox
  forwards `LLVM_PROFILE_FILE`, so lines run by the `wj` subprocess count.
- **Mutation testing** (`scripts/mutants.sh`, cargo-mutants): mutates the CLI, test runner, linter and checker files
  and runs the acceptance suites against each mutant; survivors are recorded in a text file the script writes under `docs/generated/`.
  Nightly / CI-optional (each mutant rebuilds the compiler).

### Adding to a gate

- New `wj` flag or subcommand: add a `[[promise]]` to `docs/promises.toml` and a positive and a negative case in
  `tests/promises/<command>.toml`. A known bug is an `xfail = "BUG: ..."` case, not a missing test.
- New module: make it reachable from `wj` (or the LSP/MCP server), or list it under `[library_api]`.
- A nondeterministic build: fix it (BTreeMap/IndexMap or sort); `tests/determinism_known.txt` only shrinks.
