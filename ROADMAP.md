# Roadmap

*Last updated: 2026-10-10. Version in `Cargo.toml`: 0.50.0.*

This roadmap lists only work with a concrete plan or decision attached. It does not carry dates, performance targets or
feature promises that nothing in the repository supports. The previous, much longer roadmap (versions 0.34 to 0.41) is
is no longer kept in this repository.

Related: [known gaps](docs/known-gaps.md) (ranked), [architecture improvement plan](docs/ARCHITECTURE_IMPROVEMENT_PLAN.md)
(work packages), [ADRs](docs/adr/README.md) (open decisions), [RFC status](docs/proposals/rfcs/README.md).

## Now: foundations (plan work packages 0 to 5)
- Safety net: golden generated-Rust corpus and fitness tests (`tests/architecture.rs`) with ratchets that only shrink.
- `MODULE.md` for every top-level module; layering map in `docs/MODULE_MAP.toml`.
- One `CompilerConfig` instead of environment toggles; retire `IrCutoverConfig`; move global state into a compilation session.
- Break analyzer/ir/codegen import cycles; remove crate-wide `allow(dead_code)` and dead modules.
- Test discovery: every file under `tests/` runs, CI fails on orphans.

## Next: correctness core (work packages 6 to 9)
- Name resolution to `DefId`s; delete suffix and bare-name registry lookups.
- One numeric solver, one ownership/clone solver ([ADR-0001](docs/adr/0001-ownership-truth.md)); delete demotion/promotion heuristics.
- Exit criterion: dogfood projects compile without compiler workarounds in application code.

## Then: language and tooling gaps
1. Front-end diagnostics that do not need cargo ([ADR-0005](docs/adr/0005-error-strategy.md)); a real `wj check`.
2. `.wj` formatter behind `wj fmt`; `wj watch`; `wj doc`.
3. Path and git dependencies built in dependency order, with a lock file ([ADR-0010](docs/adr/0010-package-source-of-truth.md)).
4. Concurrency per [ADR-0004](docs/adr/0004-concurrency-surface.md) (parse `async expr`/`spawn expr` or drop WJ-CONC-01).
5. Security enforcement wired into `wj build` ([ADR-0009](docs/adr/0009-security-enforcement.md)).
6. Backend trait over the IR; Go/JS/WASM consume it; conformance suite per target ([ADR-0007](docs/adr/0007-target-tiers-conformance.md)).
7. Optimizer: delete or rebuild on measurement ([ADR-0006](docs/adr/0006-optimizer-fate.md)).

## Later (only after an accepted ADR)
Comptime and const generics, an `owned` convention ([ADR-0003](docs/adr/0003-owned-keyword.md)), package registry, map type
hierarchy (WJ-LANG-03), economics tooling (WJ-PERF-01), crate split enforcing layering.

## Not goals
No rewrite of the compiler (fixtures are the asset). No new heuristic ownership passes. No new environment-variable toggles.
No fourth shader stack.

## Release rule
1.0 is gated on the ownership/diagnostics/conformance decisions above, not on a feature count. See
[ADR-0011](docs/adr/0011-release-version-policy.md) and [docs/VERSIONING.md](docs/VERSIONING.md).

## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md).
