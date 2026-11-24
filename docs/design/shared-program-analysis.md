---
status: proposed
last_verified: 27293c810
---
# One program analysis, shared by every pass of a run

A cold `wj run` of a 500-line program generates Rust for that program three times, analyzes it five times and
lowers it to IR at least three times, all in one process, to answer questions that each need one answer. This
document designs the replacement: one immutable `ProgramAnalysis` per program per configuration, computed once,
shared by reference, with explicit invalidation keys, and gated so that every diagnostic, every byte of emitted
Rust and every byte the program prints stay exactly what they are today. It then shows how the same structure
makes a sound per-function cache possible, which is the lever that turns an agent's edit-and-rerun loop from a
cold front end into a small one.

Scope: the front end of `wj run` (interpreted), `wj check`, the width gate and `wj fix`. Out of scope: the
multi-file library build (`src/compiler/library_multipass.rs`), cargo, the interpreter's execution engine.

## 1. The problem, measured

Agents edit a program and run it every turn. Every run is a cold front end: the per-file check cache keys on
the file's text, the run cache keys on all inputs, and a one-line edit misses both. The front end is therefore
the per-turn latency floor of the interpreted-first loop (ADR-0033, ADR-0037).

Profile (PC, callgrind, `wj run --no-lint` of the generated 517-line program `gen500.sh`: ten groups of structs,
`Vec`, `HashMap`, closures and small functions; min of two runs; seeds built by the measured binary; raw data on
the transfer branch `pc-results`, `findings/check-front-end-profile/`):

| Part | Instructions (Ir) | What it is |
|---|---|---|
| Whole run, cold | 3,172M (380 ms CPU) | |
| Whole run after a one-line edit | 3,174M (420 ms CPU) | no reuse at all: an edit costs a cold run |
| `wj check` part | 2,660M | |
| of which: width gate, in-process build | 1,360M | `compiler::build_project_with_config` into a temp dir: a full build-configuration codegen |
| of which: `moves::predict` | 780M | `codegen::generate_rust_source` of the whole file, `ir::explain::moves`, `borrow_moves`, text spans |
| of which: `local_mut` | 200M | 195M of it `Facts::build` (two analyzer passes and the IR solver over all files) |
| Interpreter literal widths | about 600M | `compiled_facts::solve_literal_widths`: analyzer and IR pipeline again |
| The program's own execution | about 650M | |

The parts are inclusive callgrind figures and overlap (the first pass to touch the stdlib registry, the stdlib
tables or the registry layer pays for them), so they do not sum to the total. The structure is the point: three
whole-program code generations of one unchanged program, and the same analysis recomputed by every consumer.

Hot leaves inside those passes (inclusive, same profile): `ir::liveness::borrowed_by_later_live_binding` 509M
(fixed on branch `perf-check-cache-fn`, commit c6ba87588, output-identical, cold 3,172M to 2,718M; not yet on
this base), `preregister_function_formals_in_registry` 250M, `method_sig_exact` 238M,
`infer_parameter_ownership` 223M, `stdlib_method_traits::lookup_sig` 222M (1.5M `BTreeMap` string compares).
Per process: the stdlib `SignatureRegistry` baseline rebuild 14.7M (42% of a 34.9M `hello`). Per run in a large
manifest-less directory: `collect_wj_sources` hashes every `.wj` under it (237M for 3.9k files).

A per-function cache was tried first (`REPORT-fncache.md` on the same branch) and correctly rejected as unsound:
a callee's inferred ownership comes from its body, transitively; call-site coercion reads it; the generator runs
whole-file pre-passes and a multi-pass analyzer over global signatures. A sound per-function key today must
contain the transitive callee bodies and every non-function item, so an edit anywhere dirties `main`, and `main`
dirties everything. Section 5 shows what has to change for that key to shrink.

## 2. What each pass computes today

### 2.1 The sequence of one interpreted run

`cli::run_dev::execute_path` for `wj run p.wj`:

1. `cli_execution::report_build_lints`: lex and parse the entry (parse 1), Rust-leakage lints.
2. `cli::check::require_clean`: `check::run_cache::key` (hash of every input) and a run-cache lookup.
3. `check::check_path` (miss): `check::front::parse` of every file (parse 2), resolution, effects, taint, calls,
   semantic checks, then in `check::project::run`:
   - `local_mut::run` → `local_mut::facts::Facts::build`;
   - per file `project::analysis_phase` → analyzer, numeric inference, IR lowering, then `moves::predict`.
4. `cli::widths_gate::predicted_widths_via`: the trigger walks the check's generated Rust
   (`widths::generated_rust`, which takes the text `moves::predict` remembered); a flagged file makes the
   whole program build in-process (`built_in_process` → `compiler::build_project_with_config`, which re-reads
   and re-parses the files, parse 3), and the clashes are read from that output.
5. `runner::run_entry`: the interpreter's loader parses again with its renames (parse 4); on demand
   `compiled_facts::solve_literal_widths` per module and, for programs that reassign a parameter,
   `compiled_facts::compute` (another `generate_rust_source`).

### 2.2 The stages, by input

Let `P` be a parsed file and `R0 = SignatureRegistry::stdlib()` (process-wide, immutable). The analyzer is
`Analyzer::analyze_program_with_global_signatures(P, G)` started from a seed `S` (its copy-struct set and
anything metadata merging registered); its result is `(analyzed functions, registry, trait methods)` plus the
analyzer's post-analysis state (copy structs).

| Product | Definition | Computed by today |
|---|---|---|
| `A1(P)` | analysis with `G = R0`, default seed | `analysis_phase` (check); `RustBackend::generate` inside `moves::predict` (via `analyze_program`, which adds `check_forbidden_rust_patterns`); `Facts::build` pass 1; `solve_literal_widths`; `compiled_facts::compute`; the build's pass 1 when no peer metadata, no external crate and no copy demotion applies |
| `A2(P, G1)` | analysis with `G1 = R0 ∪ A1.registry` (all files' first-pass registries merged), seed `S` | `Facts::build` pass 2 (default seed); the build's pass 2 (seed = pass-1 copy structs) |
| `N(P)` | `UnifiedNumericInference::infer_program(P)` (reads `.wj.meta` sidecars: ambient) | `analysis_phase`; `RustBackend::generate`; the build (plus external string consts) |
| `IR0(A)` | `IrPipeline::lower_to_ir(A, registry, None)` with the session's stdlib tables | `analysis_phase` (diagnostics: effect and execution violations) |
| `IRf(A, fields)` | `lower_to_ir` with a struct field table | `Facts::build` (`fields` = all files' declared fields over `A2`); `solve_literal_widths` (`local_struct_field_table(P)` over `A1`) |
| `IRfp` | `value_formal_fixed_point::lower_program_to_fixed_point`: up to four lowering rounds publishing value-formal modes into the registry | the build only (`compiler::attach_ir_module_if_missing`) |
| `Rust_check(P)` | `RustBackend::generate`: `A1`, `N`, `CodeGenerator::new(registry)`, no IR module, no global registry, default config | `moves::predict`; `compiled_facts::compute` |
| `Rust_build(P, cfg)` | the build's single-file pipeline: peers and external metadata merged, copy demotion, `A1'`, `A2'`, trait signatures from impls, `N'`, global registry, stdlib field and variant tables, `IRfp`, generation, auto-arena optimizer | the width gate only |
| `E(text)` | `ir::explain::emitted::parse_emitted` (signatures and bodies of emitted Rust) | `moves::predict_from`; `compiled_facts::compute` |
| `Syn(text)` | `syn::parse_file` | `widths::clashes` (trigger and build text) |

For a lone single-file script, which is what an agent turn usually runs, the run computes:

- `A1(P)` five times (check, the generator inside `moves::predict`, `Facts::build` pass 1, the literal widths, and
  the build's pass 1 when it qualifies), plus a sixth time when `compiled_facts::compute` runs;
- `A2` twice (local mutability, build), with different seeds when the program declares a struct the build demotes
  from `Copy`;
- `N(P)` three times;
- four IR lowerings with three different input sets, plus up to four fixed-point rounds in the build;
- `Rust_check` once or twice and `Rust_build` once;
- four parses of the entry;
- copies of the full stdlib registry for every `SignatureRegistry::new()` (`Facts::build` makes one per file per
  pass plus the global one) and an `Arc` clone of the global registry per
  `analyze_program_with_global_signatures` call.

### 2.3 Why the configurations differ, and what that means

The check generator and the build generator are different functions of the same program: the check's has no IR
module attached (the generator falls back to analyzer ownership and does not see the solver's published widths,
NUM-LITERAL-WIDTH), uses one analyzer pass, no global registry and the default `CompilerConfig`. That is why
`cli/widths_gate.rs` says the in-process check text is only a trigger: the two disagree on widths. The width
gate is correct to use the build's text, and `moves::predict` is, by today's definition, a function of the check
text. Both outputs are pinned by gates (`tests/move_prediction_test.rs`, `tests/width_prediction_test.rs`,
`tests/prediction_parity_test.rs`).

Consequence for this design: the two emitted texts are not shareable without changing output, and this design
does not merge them. What is shareable is everything below the text that has the same inputs. Converging the
two profiles onto the build's is a separate, output-changing decision (section 7.3).

### 2.4 What the consumers actually read

| Consumer | Reads | Shareable product |
|---|---|---|
| `analysis_phase` diagnostics | `A1` errors, `N.errors`, `IR0` diagnostics | `A1`, `N`, `IR0` |
| `moves::predict` | `E(Rust_check)` (moves), the AST plus `Rust_check` (`borrow_moves`), source text (spans) | `A1`, `N`, `Rust_check`, `E` |
| widths trigger | `Syn(Rust_check)` | `Rust_check` (already shared through `moves::GENERATED`) |
| widths gate | `Syn(Rust_build)` for flagged files | `A1'`, `A2'`, `N'` when their keys equal the check's |
| `local_mut` | declarations; per callable `IRf(A2)` param modes | `A1` (pass 1), `A2`, declarations index |
| literal widths | `IRf(A1, local fields).literal_widths` by (line, column) | `A1` |
| mut-ref params | `E(Rust_check)` signatures | `Rust_check`, `E` |

## 3. Architecture

### 3.1 `ProgramAnalysis`: a keyed table of pure stages

One value per command invocation (and, later, per watch session):

```text
ProgramAnalysis
  inputs:   files (relative name, text, renames), compiler key, ambient digest, profile inputs
  products: OnceLock per (stage, key)  ->  Arc<Product> | Failed
```

Each stage of section 2.2 becomes a method that returns `Arc<Product>`, computing it on first request under
`catch_unwind` (a panic is stored as `Failed`, exactly the `None` each caller produces today):

```text
session.parsed(file)                 -> Parsed            key: name, text, renames
session.analysis(file, globals, seed)-> Analysis          key: parsed key, GlobalsId, SeedId
session.numeric(file)                -> NumericInference  key: parsed key, ambient digest, external-consts id
session.ir(analysis, fields)         -> IrModule          key: analysis key, FieldsId, tables id
session.rust(file, Profile)          -> EmittedText       key: profile key (below)
session.emitted(text)                -> EmittedFile       key: text key
```

Keys are derivations, not content hashes of large structures: a registry or a field table is named by the stage
key that produced it (`GlobalsId::Stdlib`, `GlobalsId::Merged([A1 keys of every file])`,
`GlobalsId::Build(config id, peers id)`), so building a key never walks a registry. Sharing happens exactly when
two consumers ask for the same derivation; when the inputs differ in any way the key names, they get different
products, which is the behavior they have today.

Profiles:

- `Profile::Check`: `RustBackend::generate` with `A1`, `N`, default config. Key: the file's parsed key, the
  compiler key, the ambient digest.
- `Profile::Build(cfg)`: the single-file build pipeline. Key adds a digest of `CompilerConfig` (as resolved by
  `cli::build::with_project_facts`), `enable_lint`, `library`, the external metadata paths with their content
  digests, the `.wj.meta` peers `metadata::merge_reached_peers_of_file` reached (by content), the stdlib modules
  named by the source and `config.stdlib_path`. That list is the build's full input set: the
  "compiler output depends on neighbouring builds" finding of `docs/design/build-speed.md` is exactly an input
  that must be in this key.

### 3.2 Immutability and ownership

Products are immutable once stored and handed out as `Arc`. Today's consumers that mutate what they are given
take a clone at the boundary, the same clone they make now:

- `CodeGenerator::new(registry.clone(), ..)` already clones; the generator mutates its own registry during
  emission (`refresh_free_function_registry_from_emitted_formals`, `apply_self_receiver_upgrades`) and that
  stays private to the generation.
- The build pipeline mutates the pass-2 registry (`register_trait_methods_in_registry`,
  `write_back_trait_owned_string_to_impls`) and the analyzer (copy demotion): those steps run on a clone of the
  shared `A2'` registry, and their result is the product `Rust_build` depends on.
- `analyze_program_with_global_signatures` clones the global registry into an `Arc` per call; the session passes
  `Arc<SignatureRegistry>` to `analyze_program_with_global_arc` instead (same result, no clone).

Products borrow the parsed programs (`'ast` lifetimes, arenas kept alive by `Parser`), so the session owns the
`Parsed` set and every product lives no longer than it. Products must be `Send + Sync` because the check runs
files in parallel; a product that turns out to hold an `Rc` is computed per thread and not shared (recorded in
the module's invariants).

### 3.3 Where it is created and how it travels

- `cli::check::require_clean` creates the session after a run-cache miss, passes it to `check::check_path`,
  then to `cli::widths_gate`, and returns it to `cli::run_dev`, which hands it to `runner::run_entry`. The
  interpreter's `compiled_facts` asks the session first and falls back to today's computation when the session
  has no answer (different renames, a module the check never saw, no session on a run-cache hit).
- `check::local_mutability` (used by `wj build`, `wj fix`) creates a short-lived session; nothing else changes.
- The two process-global side channels that do this job badly today are deleted: `moves::GENERATED` with its
  512-file cap (`remember`, `skip`, `take_generated`) and `compiled_facts::PARSED`. A session is passed, never
  found in a static.

### 3.4 The literal widths across the check and the interpreter

The interpreter evaluates its own AST but reads literal widths by (line, column). When its loader parses a file
with an empty rename map and the same text as the check, the session answers with `IRf(A1, local fields)` built
from the check's AST: the same text gives the same positions, so the lookup is identical. With a non-empty rename
map the key differs and the interpreter computes as today. Lazy evaluation is preserved: the session computes
literal widths on the first unsuffixed literal, still gated by `compiled_facts::width_sensitive`, so programs that
never need them never pay. The cross-process `FactStore` (run cache) is unchanged and consulted first.

### 3.5 Module ownership

A new module `program_analysis` (directory src/program_analysis/, created by step 1), layer `backend`: it
calls the analyzer, numeric inference, the IR pipeline and the Rust generator, and touches no file, environment
variable or cache directory. Everything it needs from the outside world arrives as values with an identity:
`Externals { peers, external_crates, stdlib_types, config }` loaded by the driver (`compiler`, `cli`) from
`metadata`. This is what lets `check` (tooling) and `interpreter` (tooling) use it, and lets the build-profile
stages move out of `compiler::compilation_pipeline` without `check` importing `compiler`. Details in section 8.

## 4. Soundness and the staleness gate

### 4.1 Argument

Replacing a recomputation `f(x)` by an earlier `f(x)` changes nothing when (a) `f` is a function of the inputs
its key names, (b) nothing mutates the stored value, and (c) the consumer cannot tell a fresh value from a stored
one. (b) is structural (`Arc`, clone at mutating boundaries, section 3.2). (c) holds because consumers read
values, not addresses; the one identity-keyed reader, liveness by AST node identity, is internal to a stage and
never crosses the session. (a) is the real claim and is checked three ways rather than argued:

1. **Purity audit.** Every process-global the stages touch is listed in the module's invariants with why it is
   harmless: `SignatureRegistry::stdlib()` and the scanned runtime layer (immutable after init), the stdlib
   tables (immutable, per session), `debug_env` phases (output only), panic hooks (restored), and the
   thread-locals and `OnceLock`/`Mutex` statics in `analyzer`, `codegen/rust`, `ir` and `type_inference`
   (five today, each named). A new static in those modules fails an architecture rule until it is listed.
2. **Verify mode.** A diagnostics switch (planned environment variable, read once at the driver edge) makes
   every hit recompute the product and compare a canonical dump (sorted `Debug` of analyzed functions, registry
   entries, IR functions, numeric facts; the emitted text byte for byte). A mismatch panics with the stage and
   key. This catches impurity the audit missed and keys that are too small.
3. **Differential.** A switch that disables sharing (every consumer computes for itself, as today) is the
   oracle. Shared and unshared runs must agree byte for byte.

### 4.2 Gates (each must fail when its property breaks)

| Gate | Property | Corpus |
|---|---|---|
| G1 differential | `wj check --json`, the stderr of an interpreted run up to the program's first output, the program's stdout, the Rust `wj build --no-cargo` writes, and `wj fix` edits are identical with sharing on and off | the corpora of `tests/prediction_parity_test.rs` (inference corpus, drift repros, engine parity, conformance programs, examples, tests), the generated projects of `tests/check_cache_differential_test.rs`, the move and width prediction corpora, `tests/book_repros/` |
| G2 verify | no hit differs from its recomputation | same corpus, run once per step |
| G3 taken | the shared path is taken: a counter of computed vs reused products per stage, asserted on `gen500.sh` output (`A1` computed once per file, `Rust_check` once) and on a two-file project; a silent fallback fails | fixed fixtures |
| G4 mutations | each of these makes G1 or G2 fail on a designed fixture: renames dropped from the parse key (a program the loader renames); the copy seed dropped from the `A2` key (a struct the build demotes from `Copy`); the peers digest dropped from the build key (a project whose neighbour metadata changes generation, the build-speed finding); a mutating consumer handed the shared registry without a clone | fixtures under tests/fixtures/program_analysis/ (step 1) |
| G5 cost | ceilings in `scripts/wj/check_cost_gate.wj` and `tests/perf_ceiling` lowered after each step, never raised | existing |
| G6 memory | peak RSS of `wj check` on the largest corpus program (engine parity) and on the 517-line program stays within a ratchet; the session's high-water mark is reported by `mem_report` | existing |

Mutation checks run under a hard timeout. Measurements follow the PC protocol of the profile: callgrind Ir on a
fresh cache root, min of two, each row split into check, codegen and run cost with `hello` quoted beside it, the
one-line-edit variant next to every cold row, seeds made by the measured binary (`cold.sh`, `rows.sh` on the transfer branch), plus the row set
hello, agent_loop, closures, hashmap, structs, fib30, loop5m, nbody, sieve, strings, 500-line cold and 500-line
after a one-line edit.

## 5. Toward a sound per-function cache

### 5.1 Why today's key cannot be cut

A function's emitted Rust depends on: its own body; the ownership, receiver mode and widths of every callee, which
the analyzer and the solver infer from the callee's body, transitively; whole-file pre-passes (trait-object,
`PartialEq`, `Drop`, `Copy` registries, module `const string` names, local type names); and the registry the
generator itself rewrites while emitting earlier functions. `moves::moves` looks callees up in the emitted file,
and WJ0007 spans come from the first textual match anywhere in the source. Each of these is a channel through
which an edit in one function reaches another.

### 5.2 The split

Separate what a function promises (its signature) from how it keeps the promise (its body). Then a caller depends
on its callees' signatures, never their bodies, and an edit whose signature does not change stops at the edited
function (early cutoff).

**Item facts** (program-wide, small, recomputed every run from the ASTs; key = digest of the non-function items):
struct and enum declarations with field and payload types; the copy set after demotion; derive sets (trait-object
closure, `PartialEq`, `Drop`, explicit `@derive` opt-outs); module consts and `const string` names; imports and
local type names; trait declarations; inferred bounds (`inference::collect_inferred_bounds`); for the build
profile, the `Externals` identity.

**Signature facts** (per function `f`, the interface): declared formals and result; the decided mode of every
formal and the receiver (`&T`, `&mut T`, owned; the solver's fixed point, as published); whether a formal is lent
for change (`local_mut::facts::Callable`); widths of formals and result; the effect set and latent parameters
(`ir::effect_facts::analysis`); the taint summary (`check::taint::flow`); the emitted path and name. Key of the
signature facts of a strongly connected component `C` of the call graph:

```text
Kσ(C) = H(compiler key, item facts digest, bodies of C (text and parsed shape), Σ(callees of C outside C))
```

where `Σ(g)` is the value of `g`'s signature facts, not its key. Recursion is handled by solving a whole SCC at
once, which is what the analyzer's convergence loop and `value_formal_fixed_point` already do over the whole
program.

**Body facts** (per function, everything the passes read about its inside): liveness and last uses, inserted
clones, local types, literal widths by position, local mutability findings, predicted moves (`Move` values without
spans), the emitted body text. Key:

```text
Kβ(f) = H(compiler key, item facts digest, body of f, Σ(f), Σ(callees of f))
```

**Spans stay whole-file and are never cached.** WJ0007 spans (`moves::span_of`, first match in the whole file)
and WJ0051 spans (`check::locate`) are recomputed every run from cached `Move` and `Found` values and the current
text: a text search, cheap, and identical to today's output by construction. Making spans function-local is a
separate fix with its own span-oracle gate, because it can move a span.

### 5.3 What has to be true first

1. **Emission does not decide signatures.** Today the Rust backend re-decides some formals while emitting
   (`docs/design/ownership-decided-once.md`: `pub_module_api_keeps_owned_string_formal`,
   `param_should_emit_into_string_formal` and others) and writes them back into the registry, so a caller's
   output depends on emission order. That design's stage 1, its gate-5 inventory
   (`tests/golden/own_stages_emitted_vs_ir.txt`) reaching zero, is the prerequisite. After it, `Σ(f)` is fully
   known before any body is emitted.
2. **Signature inference by SCC reproduces the whole-program fixed point.** The analyzer's convergence and the
   value-formal rounds iterate over all functions at once. Run over SCCs in reverse topological order, they must
   give the same `Σ` for every function. Gate: whole-program `Σ` equals SCC-wise `Σ` over the G1 corpus, with a
   mutation that feeds a caller its callee's pre-convergence signature.
3. **The generator's whole-file pre-passes read item facts only.** Each pre-pass in
   `codegen::rust::program_generation::generate_program` is audited and either proven to read only items or
   moved into item facts.

### 5.4 Where cached facts live

In-process first (`wj check --watch`, the hot-reload loop, `wj fix` iterations), then on disk through the
existing `check::shared_cache` store under the compiler key (content keys, no paths: worktrees and copies keep
their hits, the invariant `tests/check_cache_location_free_test.rs` already enforces for file entries).

### 5.5 Edit model

An agent edits `weight_6`. Item facts digest unchanged; `Kσ({weight_6})` changes, its `Σ` is recomputed and, for a
body-only edit, compares equal; every other function's `Kσ` and `Kβ` are unchanged and hit; `Kβ(weight_6)` misses.
The run pays: parsing, item facts, the call graph, one SCC's signature inference, one body, span resolution, the
cheap whole-file passes (resolution, effects, semantic checks) and the program's execution. An edit that changes
`weight_6`'s signature (it starts mutating its argument) re-keys its callers' bodies, which is precisely the set
whose output can change.

## 6. Steps, gates and expected savings

Each step lands alone, keeps output identical (G1 to G4), and lowers the cost ceilings (G5). Savings are against
the 517-line program; baseline = this base, 27293c810. Figures marked *est.* are estimates from the profile's
inclusive attributions and are deliberately conservative (overlapping attributions are not double counted);
step 0 measures the per-stage split that turns each estimate into a measured row before its step starts.

### Step 0: measure the stages, land the measured fix

- Per-stage counters (computed, reused, Ir via `debug_env::phase` boundaries) for the stages of section 2.2,
  printed on request; G3 is built on them.
- Land c6ba87588 (liveness horizon memo), measured: cold 3,172M to 2,718M, edit 3,174M to 2,710M, output
  identical (unit test `derived_horizon_equals_the_per_query_walk`). Its open mutation (dropping
  `binding != name` survives) gets a killing fixture first: a shadowed rebinding that derives from itself.
- Gate: the counters on `gen500.sh` show the duplication table of section 2.2 (the "before" of G3).

### Step 1: the session, check profile

`ProgramAnalysis` with `parsed`, `analysis` (`A1`, `A2`), `numeric`, `ir`, `rust(Check)`, `emitted`. Consumers:
`analysis_phase`, `moves::predict` (the generator receives `A1` and `N` instead of recomputing them: a
`RustBackend` entry that takes them, used only through the session), `Facts::build` (pass 1 is `A1`, pass 2 is
`A2` from the session; no `SignatureRegistry::new()` copies), literal widths, mut-ref params. `moves::GENERATED`
and `compiled_facts::PARSED` deleted.

Expected: removes the generator's `A1` and `N`, `Facts::build`'s pass 1 and registry copies, the literal widths'
`A1`, and the second `Rust_check` where it occurred. *est.* 350M to 600M cold.

### Step 2: the session, build profile

The single-file branch of `compiler::build_project_with_config` is split into "load externals" (driver: metadata,
peers, config) and "stages" (`program_analysis`: pass 1, demotion, pass 2, trait signatures, numeric, fixed point,
generation). The width gate asks the session for `Rust_build` in memory instead of writing a temp directory and
reading it back; `wj build` keeps writing files through the same stages. When the build's keys coincide with the
check's (no peers, no external crates, no demotion, same ambient), `A1'`, `A2'` and `N'` are hits.

Expected: for a flagged program, *est.* 200M to 400M of the 1,360M (analysis and numeric inference reused,
no disk round trip). The fixed-point rounds and the generation remain. `tests/prediction_parity_test.rs` keeps
proving the in-memory text equals the child build's byte for byte.

### Step 3: lookups memoised per registry epoch

`stdlib_method_traits::lookup_sig`, `CodeGenerator::method_sig_exact`, `signatures_for_method_name` memoised per
(receiver type, method, arity) and per registry epoch (a counter every registry mutation bumps, so a memo never
survives a write). `preregister_function_formals_in_registry` and `infer_parameter_ownership` profiled again after
step 1, since fewer analyses already shrink them.

Expected: *est.* 150M to 300M cold, measured after steps 1 and 2 (their reductions overlap).

### Step 4: stdlib baseline snapshot

The fully assembled `SignatureRegistry::stdlib()` (runtime layer, stdlib meta, primitive signatures, Rust std
boundary) stored under the compiler key next to the scanned runtime layer
(`analyzer/signature_registry/snapshot.rs` does this for the layer today), loaded by the next process.
Gate: loaded registry equals the assembled one (canonical dump), plus a mutation that drops one boundary signature.

Expected: about 10M to 14M per process (of the measured 14.7M): small for the 500-line program, about a third of
`hello`. Every agent turn is a new process, so this is paid on every turn.

### Step 5: hash the module closure, not the directory

`metadata::crate_metadata::projectless_tree_key` hashes every `.wj` under a manifest-less root. Key it on the
closure the metadata readers can reach from the entry (the files `merge_reached_peers_of_file` may read), which
is the input the entries actually depend on. (If the closure cannot be bounded before the readers run, the
fallback is to memoise the walk per process; it removes repeats, not the first walk.) Gate: an edit outside the
closure keeps the hit; an edit inside misses; mutation: a key that omits a reached peer fails a fixture where that
peer changes generation; cost: the repro with 0, 200 and 2000 unrelated `.wj` files next to the entry must cost
the same within noise.

Expected: about 13.6k instructions per unrelated file today (up to 237M in a directory with thousands of loose
`.wj` files, measured), zero inside a project.

### Step 6: the run cache stores flagged outcomes

`require_clean` stores a clean outcome only when "the prediction read nothing but the sources", so a program the
width trigger flags is checked again on every unchanged rerun, build included. With the build profile's key
naming every input the build read (section 3.1), that key joins the run-cache entry and flagged outcomes are
stored too. Gate: `tests/run_check_cache_test.rs` extended with a flagged program: hit on rerun, miss after a peer
metadata change, identical stderr.

Expected: unchanged reruns of flagged programs from the full check (2.66G, or what remains after steps 1 to 3) to
the hash-and-replay cost, *est.* under 100M. Edit reruns are unaffected.

### Step 7: signatures before bodies (needs section 5.3)

7a: emission stops writing signatures (ownership-decided-once stage 1, gate 5 at zero).
7b: SCC-wise signature inference; gate: whole-program `Σ` equals SCC-wise `Σ`.
7c: body facts keyed by `Kβ`, in-process, used by `wj check --watch` and `wj fix` iterations; gate G7:
for every function of every corpus program and for `gen500.sh edit`, an incremental run after a one-function edit
equals a cold run of the edited program byte for byte; mutation: `Kβ` without the callee signatures fails a
fixture where a callee starts mutating its parameter (the caller must now lend it `&mut`).
7d: the same facts in the persistent shared store, so separate `wj run` processes (an agent's turns) reuse them.

Expected: edit-rerun front end proportional to the edit. *est.* 0.6G to 1.0G total for the 500-line program after
a body-only edit (fixed per-run passes, one SCC, the program's 650M execution unchanged). Cold runs pay a small
store write.

### Summary

| After step | Cold, 517 lines | Edit rerun | Unchanged rerun (flagged program) |
|---|---|---|---|
| Baseline 27293c810 | 3,172M measured | 3,174M measured | full check, inferred from `require_clean` |
| 0 liveness memo | 2,718M measured | 2,710M measured | as cold |
| 1 shared check profile | *est.* 2,120M to 2,370M | same as cold | as cold |
| 2 shared build profile | *est.* 1,720M to 2,170M | same as cold | as cold |
| 3 lookup memo | *est.* 1,420M to 2,020M | same as cold | as cold |
| 4 stdlib snapshot | minus about 10M to 14M | same | same |
| 5 closure hashing | unchanged here (project-less large directories only) | | |
| 6 flagged outcomes cached | unchanged | unchanged | *est.* under 100M |
| 7 signatures before bodies | about unchanged | *est.* 600M to 1,000M | unchanged |

`hello` (34.9M today) mostly gains from step 4; the other rows of the PC table are measured after each step and
added to `docs/design/build-speed.md`'s style of record in this document.

## 7. Risks and what not to do

### 7.1 Risks

- **Output drift.** The whole point is none. G1 compares the full user-visible output with sharing on and off on
  every step; G2 compares every hit with its recomputation; G4 proves both can fail.
- **Hidden state across stages.** A stage that reads a static, a thread-local or a previous stage's side effect
  makes a product order-dependent. The purity audit lists them; verify mode finds the rest; a new static in the
  stage modules fails the architecture rule until listed.
- **Determinism.** Products built from `HashMap` iteration must not leak that order into anything compared or
  emitted. The canonical dumps sort; the emitted text is already deterministic (gated by the build-speed golden
  hashes and the parity test).
- **Memory.** Today each product dies with its pass; in a session they live until the run ends, because literal
  widths are computed lazily during interpretation. The session drops every product except what the interpreter
  may still ask for (`A1` and field tables per module) before execution begins; G6 ratchets peak RSS. For an
  engine-size program the check profile stays per file, as it is now.
- **`--no-lint`.** It filters which diagnostics block; it does not change any product, and it is not part of any
  analysis key. `enable_lint` of the build profile is part of the build key, because it can make the build stop.
- **The run cache.** On a run-cache hit no session exists and nothing changes: the interpreter falls back to the
  `FactStore` and its own computation, as today. The run-cache key is not modified except in step 6.
- **`wj fix` and `wj check --fix`.** The fix loop re-runs `local_mutability` after each round of edits. Each
  round is a new program text, so it is a new session; files whose text did not change get their `A1` from the
  previous round only once step 7c keys exist. The fix engines must stay in parity (both go through
  `check::apply_fixes`); G1 includes `wj fix` output.
- **Panics.** Every stage runs under `catch_unwind` as today and a panic is stored as a failed product, so a
  consumer that skipped on failure still skips, and the panic is not re-raised by a second consumer.

### 7.2 What not to do

- Do not cache per function before section 5.3 holds. Keyed on function text alone it is unsound (the PC report
  shows why); keyed soundly it covers the whole program.
- Do not share emitted text across profiles, and do not substitute one profile's text for the other's.
- Do not add a new process-global memo. `moves::GENERATED` is the pattern this design removes.
- Do not key on a registry's or a field table's content by hashing it; name it by its derivation.
- Do not let `check` import `compiler`; inject the externals as values.
- Do not answer cost with recognisers (name lists, text sniffing) that skip analysis: `nothing_to_predict` and
  `width_sensitive` are the existing conservative ones, and they stay conservative.
- Do not build this on the salsa `CompilerDatabase`. It is constructed and dropped unused in
  `compiler::salsa_library_build::build_library_salsa` today, and the products here borrow AST arenas; a plain
  keyed table is simpler and enough. Revisit salsa only for step 7's owned signature facts, if the hand-rolled
  table grows dependency tracking.

### 7.3 A separate decision: one generator profile

`moves::predict` reads the check profile; rustc compiles the build profile. Predicting moves from `Rust_build`
would delete a whole codegen (*est.* the generation part of 780M) and make WJ0007 read the code rustc actually
sees, which is ADR-0030's own principle. It changes output (some predictions appear or disappear), so it is not a
step of this plan: it needs its own proposal under ADR-0030, a precision scan with rustc as oracle over the
move-prediction corpus, and updated pins.

## 8. Module ownership and the architecture map

`docs/MODULE_MAP.toml` gains:

```toml
[modules.program_analysis]
layer = "backend"
owner_stage = "shared whole-program analysis: one computation of each analysis product per program and configuration"
module_md = "src/program_analysis/MODULE.md"
allowed_imports = ["analyzer", "codegen", "compiler_config", "diagnostics", "ir", "lexer", "parser", "type_inference", "typing", "type_classification", "stdlib_surface"]
known_violations = []
```

Its MODULE.md states: purpose (section 3); inputs (parsed files, `Externals`, profile) and outputs (`Arc`
products); invariants (stages are pure functions of their keys; every static they touch is listed; products are
never mutated; no file, environment or cache access; a panic is a stored failure; keys name derivations); owns
(the session, keys, profiles, verify and differential switches, counters); never decides (diagnostics wording,
caching policy across processes, which consumer runs); entry points (`ProgramAnalysis::new`, the stage methods);
tests (G1 to G4 files, unit tests of key derivation).

Changes to existing modules:

| Module | Change |
|---|---|
| `check` (`src/check/MODULE.md`) | `project::analysis_phase`, `moves`, `local_mut::facts`, `widths` take the session; the `GENERATED` side channel is removed from the `moves.rs` entry; the cache invariants gain "the session never crosses processes; `file_cache` and `shared_cache` keys are unchanged" |
| `cli` | `check::require_clean` creates and returns the session; `widths_gate` asks it for `Rust_build`; `run_dev` passes it to the interpreter; `cli` loads `Externals` |
| `compiler` | the single-file branch of `compilation_pipeline::build_project_with_config` calls the session's build-profile stages; `value_formal_fixed_point` and `attach_ir_module_if_missing` move behind the stage API (the `ir` known violation on `compiler` can then shrink) |
| `interpreter` | `compiled_facts` asks the session first; `PARSED` removed; `FactStore` unchanged |
| `codegen` | `RustBackend` gains an entry that takes `A1` and `N` instead of computing them, used by the session only; `generate_rust_source` stays for tools |
| `analyzer` | `SignatureRegistry` epoch counter (step 3); stdlib baseline snapshot (step 4) |
| `metadata` | `projectless_tree_key` over the reachable closure (step 5) |
| `ir` | none in steps 0 to 6; step 7 adds SCC-wise entry points to the solver |

`tests/architecture.rs` enforces the new layer edge set; `docs/architecture-allowlist.toml` shrinks when the
fixed-point code leaves `compiler`.
