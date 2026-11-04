# Scratch directory hygiene

Test harnesses and scripts must not leave `wj-*` directories behind in the OS temp dir. Before this
work a developer machine accumulated about a thousand of them (about 20 GB), mostly the conformance
corpus work trees.

## The rule

* Rust tests create scratch directories with `tests/common/scratch.rs`:
  `Scratch::new("tag")` makes `<tmp>/wj-<tag>-<pid>-<n>` (canonical path) and removes it on drop,
  including while a panic unwinds. `WJ_KEEP_TMP=1` keeps it and prints `WJ_KEEP_TMP=1: keeping <path>`.
  `tempfile::TempDir` is also fine (it cleans on drop).
* Expensive shared inputs (compiled targets) go under `scratch::shared_cache_dir(name)`, inside the
  shared cargo cache root (`wj cache path`), never into a per-pid temp dir.
* Shell scripts use `trap 'rm -rf "$d"' EXIT INT TERM` after `mktemp`; embedded Python uses `atexit`.
* Interpreted Windjammer scripts clean up at the end of a normal run (no signal handlers yet).
  Interrupted runs are covered by the janitor below.

## Gates

`tests/tmp_hygiene_test.rs` (module of the `all` test binary):

| Gate | What can fail |
| --- | --- |
| allowlist ratchet | a new `std::env::temp_dir()` in `tests/**/*.rs`, or `mktemp`/`/tmp/wj` without a trap in `scripts/`, outside `docs/testing/tmp-allowlist.txt`; also fails when an entry shrinks without the list being updated |
| end to end | the converted tests run in a child process with a fresh `TMPDIR`; any `wj-*` left behind fails |
| non-vacuity | the same run with `WJ_KEEP_TMP=1` must leave at least three directories |
| panic unwinding | a planted test panics while holding a `Scratch`; the directory must be gone |
| janitor | `wj clean --tmp` dry run, `--yes`, age threshold, live-pid and prefix rules |

## Janitor

```
wj clean --tmp                      # dry run: lists wj-* dirs older than 6 h with no live pid in the name
wj clean --tmp --older-than-hours 24
wj clean --tmp --yes                # delete
```

`wj doctor` prints a note with the count and total size when such directories exist.

## Inventory of creators (owner file)

Converted to `Scratch` (removed on drop):

| Prefix | Owner file |
| --- | --- |
| `wj-conformance-*-corpus/-interp/-ts/...` | `tests/backend_conformance_test.rs` (per-cell dirs are also removed as soon as a cell is judged) |
| `wj-lintmodes-*` | `tests/lint_modes_agree_test.rs` |
| `wj-stdin-*` | `tests/stdin_modes_test.rs` |
| `wj-nestedq-*` | `tests/interpolation_nested_quotes_test.rs` |
| `wj-opt-test-*` (raw, opt, scratch-reuse, gen, mut) | `tests/fixtures/optimizer/support.rs` (used by `optimizer_fixture_test`, `optimizer_generative_test`, `optimizer_mutation_test`, `optimizer_scratch_reuse_test`) |
| `wj-opt-ratchet-*` | `tests/optimizer_ratchet_test.rs` |
| `wj-stale-hotpath-*` | `tests/toolchain_stale_test.rs` |
| `wj-arena-be-*`, `wj-arena-rust-*` | `tests/arena_backends_test.rs`, `tests/arena_rust_backend_test.rs` (shared cargo target moved to the cache dir) |
| `wj-e2e-*` | `tests/diag_requests_e2e.rs` (target dir moved to the cache dir) |
| `wj-hints-*` | `tests/diag_hints_suite.rs` |
| `wj-vec_js/go`, `wj-pkgs`, `wj-compiled`, `wj-escape`, `wj-durable_c`, `wj-collision`, `wj-infer`, `wj-hygiene`, others | `tests/effects_vectors_test.rs`, `effects_packages_test.rs`, `effects_compiled_test.rs`, `effects_escape_test.rs`, `durable_compiled_test.rs`, `method_collision_mutation_test.rs`, `inference_differential_test.rs`, `public_hygiene_test.rs`, `spec_conformance_test.rs`, `bug_*`, `extern_unsafe_wrapping_test.rs`, `unknown_method_mut_inference_test.rs`, `owned_param_insert_clone_test.rs` |
| `wj-interp-compare-*` | `scripts/interp_compare.sh` (atexit + SIGINT/SIGTERM handler) |
| `mktemp -d` in `scripts/mutants.sh` | trap added |

Still on the allowlist (`docs/testing/tmp-allowlist.txt`):

* Fixed-name helpers returning a bare path that clean up by hand: `constructor_type_inference_test`,
  `cross_crate_string_literal_test`, `string_literal_method_call_test`, `crate_internal_not_extern_test`,
  `cross_module_extern_unsafe_test`, `bug_some_wrap_auto_clone_test`,
  `bug_module_file_output_must_not_import_tmp_sibling_rs_test`.
* `interpreter_projects_test`: keeps failing fuzz seeds on purpose.
* `effects_scratch_test`: exercises `env::temp_dir()` of the interpreter itself.
* Interpreted `.wj` scripts under `scripts/wj/` that use `/tmp/wj-*` and remove at the end of a normal run.

Product code that creates temp directories at run time (`src/cli_execution.rs`, `src/test_runner`,
`src/doctest/run.rs`, `src/embedded_assets.rs`, `src/ejector.rs`, `src/interpreter/caps.rs`, `src/metadata/crate_metadata.rs`)
is outside the harness gate: `wj-embedded` and `wj-meta` are deliberate shared caches, the rest are per-run
directories owned by the command.
