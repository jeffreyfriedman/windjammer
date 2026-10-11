# optimizer

Post-codegen Rust optimizer: behaviour-preserving rewrites of the Rust text that
`CodeGenerator::generate_program` emits.

## Purpose
Remove inefficiencies the code generator cannot cheaply avoid (redundant clones,
growth reallocations, quadratic string building, no-op casts, inline-hint bloat,
dead layout/serialization helpers). Each rewrite has a written soundness
argument in `docs/optimization/PASSES.md`; a pass without one stays off.

## Inputs / outputs
- In: `&OptimizationConfig`, generated Rust source (`&str`).
- Out: `OptimizationOutput { code, applied: BTreeMap<pass, count> }`.
- Unparsable input, files declaring `macro_rules!`, or edit sets that yield
  unparsable Rust are returned unchanged.

## Invariants
- Edits are span-based; untouched bytes are preserved exactly.
- Line count never changes (removals keep their newlines), so source maps stay valid.
- `OptimizationConfig::disabled()` is the identity; `run` is idempotent.
- Facts (`facts.rs`) are definite or absent: a name has a known type only if EVERY
  binding of that name in the function (params, lets, patterns, closures) agrees.

## Owns / never decides
- Owns: the pass list, pass preconditions, edit application.
- Never decides: language semantics, type inference, ownership (those stay in
  `analyzer`/`codegen`); it never consults WJ AST or global state.

## Allowed imports
`syn`, `proc_macro2`, `quote`, `std`. No `crate::parser`, `crate::analyzer`, `crate::codegen`.

## Entry points
`optimizer::run(&OptimizationConfig, &str)`, `optimizer::run_default(&str)`.
Hooks: `file_compiler.rs`, `output_generation.rs`, `compiler_database.rs` (after `generate_program`).

## Layout
`config.rs` switches · `edit.rs` span edits · `facts.rs` per-fn binding facts ·
`fn_walk.rs` fn iteration · `passes/*` one file per pass · `mutants.rs` deliberately
unsound variants (tests only).

## Testing
- `cargo test --lib optimizer::` (unit, per pass, with rejection cases)
- `tests/optimizer_fixture_test.rs` (fixture programs vs independent Python model, metamorphic)
- `tests/optimizer_generative_test.rs` (seeded generator + shrinking; `--features optimizer-soak` for 5000)
- `tests/optimizer_mutation_test.rs` (every unsound mutant must be caught)
- `tests/optimizer_ratchet_test.rs` (clone/to_string/cast counts must not grow)
