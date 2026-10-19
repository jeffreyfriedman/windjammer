# compiler_config

## Purpose
Explicit, plain-data configuration for the compiler (`CompilerConfig`). Replaces ambient
environment variables and process-global flags so tests and production run identical code.

## Inputs / outputs
- In: nothing (`Default`), builder calls, or an env-style lookup (`from_lookup`/`from_env`).
- Out: a `CompilerConfig` value that is cloned into `ModuleCompiler` / `CodeGenerator` /
  Cargo.toml generation / library builds.

## Invariants
- `CompilerConfig::default()` is production behavior (all IR cutover flags on).
- `from_env()` is the ONLY place behavior-changing env vars are interpreted
  (`WJ_IR_CUTOVER_DISABLE_*`, `WJ_IR_COERCION_ENGINE`, `WJ_LEGACY_MULTIPASS`,
  `WJ_EMIT_AOSOA_HINTS`, `WJ_CACHE_LOCALITY_JSON`, `WINDJAMMER_STDLIB`,
  `WINDJAMMER_RUNTIME_PATH`, `game-host_PATH`, `WJ_AUTO_ARENA`); call it only from the CLI driver.
- Diagnostics-only variables (`WJ_DEBUG_*`, `*_TRACE`, `WJ_PROFILE`) live in `debug_env`
  and must never influence generated output.
- Guarded by `tests/compiler_config_env_isolation_test.rs` (greps `src/` for `env::var`).

## Owns / never decides
Owns: the list of knobs and their defaults. Never decides codegen behavior itself.

## Allowed imports
`codegen::rust::IrCutoverConfig` (type only), `std`.

## Entry points
`CompilerConfig::{default, from_env, from_lookup, with_*}`;
`build_project_with_config`, `ModuleCompiler::with_config`, `CodeGenerator::set_config`.

## How to test
`cargo test --release --test all compiler_config` and `determinism`.
