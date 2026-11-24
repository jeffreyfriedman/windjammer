# src/program_analysis/

## Purpose
One analysis of a program per `wj check` run, shared by reference by every consumer that needs it (docs/design/shared-program-analysis.md, step 1): the analyzer's answer for a file (A1, with the stdlib signatures; A2, with the signatures of all files) and numeric inference. Before it, the diagnostics phase, the Rust generator inside the move prediction and the mutability facts each ran the analyzer and numeric inference on the same file.

## Inputs/Outputs
In: a `FileKey` (file name and the exact parsed text), the parsed `Program`, a `Mode`. Out: `Arc<Analysis>` and `Arc<UnifiedNumericInference>`, immutable once stored.

## Invariants
- A key names how a product was derived (`keys.rs`): the file's name and exact text, and for A2 the files whose first-pass registries were merged, in order. A product is reused only when every input the key names is equal; adding an input to a stage means adding it to its key.
- Products are immutable (`Arc`); a consumer that changes one takes a clone first (`moves::generate_shared` clones the registry, the trait-method table and the numeric inference, as the generator always owned its copies).
- A panic in a stage propagates to the consumer that asked, exactly as before sharing, and stores nothing: the next consumer recomputes and meets the same panic under its own `catch_unwind` (or none).
- Purity audit of what a stage reads besides its inputs: `SignatureRegistry::stdlib()` and the scanned runtime layer (immutable after first use), the stdlib tables (per run), `debug_env` counters and phases (output only), the `.wj.meta` sidecars numeric inference reads (constant during a run, which is why a session lives for one run). The analyzer, `codegen/rust`, `ir` and `type_inference` keep thread-locals and statics; none holds state a second run of the same stage would see differently (G2 checks this on the corpora).
- The module touches no file, environment variable or cache directory; the driver chooses the `Mode` (`check::analysis_mode`).
- Modes: `Shared` reuses, `Off` computes every request fresh (the differential oracle, `WJ_NO_SHARED_ANALYSIS=1`), `Verify` reuses and recomputes every reuse, panicking on a difference (`WJ_VERIFY_SHARED_ANALYSIS=1`).
- Counters (`debug_env::Stage`, `WJ_STAGE_COUNTERS=1`): `analyzer_reused` and `numeric_reused` count reuses; tests/program_analysis_test.rs asserts the shared path is taken.

## Owns
The session, its keys, the verify mode.

## Never decides
What a product means (the analyzer and numeric inference do), what is reported (the check), how the driver finds files.

## Allowed imports
Layer `backend`: `analyzer`, `ir`, `parser`, `debug_env`. Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`ProgramAnalysis::{new, first_pass, second_pass, numeric}`.

## Tests
tests/program_analysis_test.rs (differential, verify, shared path taken, mutations), tests/stage_counters_test.rs (the ledger), the unit tests here.
