# Cleanup inventory (temporary; deleted at the end of the cleanup)

Method: `git ls-files` sweep, then grep for references from Cargo.toml, workflows, scripts, tests, src.

## Deleted (evidence)
| Item | Reason / evidence |
|------|-------------------|
| `ref_test`, `debug_squad` (root) | Committed Mach-O arm64 executables (4.2 MB); referenced by nothing |
| `rendering_ffi/` | Not a workspace member; only referenced by Dockerfile dummy-build lines (removed) |
| `tools/sdk-generator/`, `api/*.json`, `docker/`, `docker-compose.test.yml`, `.github/workflows/test-sdks.yml`, `scripts/test-all-sdks.sh` | Multi-language SDK generator; `sdks/` directory does not exist; not in workspace |
| `homebrew/windjammer.rb` | Formula pinned to v0.7.0 with placeholder sha256; no tap |
| `tests_wj/` | One fixture, referenced only as a temp-dir name by a test |
| `tests/COMPILER_REPRO_QUEUE.md` (767 KB), `STDLIB_*_QUEUE/HANDOFF`, `run_red_repro_bundle.sh`, `cli_gate_*.sh`, `reproducibility_test.sh` | Per-ticket logs of downstream dogfooding; ADR-0013 recommends removal; scripts hard-coded a local path or needed private projects |
| `tests/TEST_COVERAGE_PLAN.md`, `tests/LANGUAGE_SPEC_TESTS.md` | Counts (734 tests) and file lists contradict the tree |
| `docs/archive/` (20 files) | Superseded docs; git is the archive |
| `docs/API_REFERENCE.md`, `COOKBOOK.md`, `CROSS_PLATFORM_VISION.md`, `WEB_EXPORT_STRATEGY.md`, `MULTI_LANGUAGE_*`, `TOOLING_VISION.md`, `COMPARISON.md`, `BEST_PRACTICES.md`, `PUBLISHING.md`, `DOWNSTREAMAPP_WJ_PIN.md`, `COMPILER_OPTIMIZATIONS.md`, `INDEX.md` | Game-framework era (v0.34), SDK vision, unmeasured performance claims (99%+, 276x), downstream pin note, or superseded by ADR-0006 / docs/README.md |
| `docs/tutorials/` | Game tutorials and a getting-started with a wrong repo URL and "98.7% measured" claim |
| `docs/design/windjammer-ui.md`, `ASYNC_EXECUTION_MODEL.md` | UI lives in its own repo; async model superseded (ADR-0004) |
| `src/optimizer/quarantine/` | Retired AST phases, not compiled |
| `editors/vscode/` | Older (0.19) duplicate of `vscode-extension/` (0.35) |
| `crates/windjammer-mcp/DESIGN.md`, `fix_ast.sh` | 0.31 "in development" design; one-off script |
| `.release-notes/` | Notes for 0.37-0.41, duplicate of CHANGELOG |
| `scripts/{add,fix}_tarpaulin_ignore.sh`, `build_wasm_ui.sh`, `dev_server.sh`, `reorganize_tests_phase18.py` | Referenced by nothing |
| ~45 game/UI/render examples under `examples/`, `examples/VALIDATION_RESULTS.md`, taskflow status docs, `taskflow/rust`, `taskflow/benchmarks` | Depend on the separate game/UI frameworks, status snapshots, unverified comparisons |

## Kept but corrected
README, ROADMAP, AGENTS, CONTRIBUTING, CHANGELOG (header), ADR README and 0006 amendment, VERSIONING, known-gaps,
examples/README, taskflow README, banners on SALSA/STDLIB/CONCURRENCY/EXPORT_TARGETS docs, docs/README (new index).

## Doubts (not changed)
`std/**/API.md` and `std/README.md`, `crates/windjammer-{lsp,mcp}/README.md`, `OAuth.md`, `benchmarks/README.md`,
`templates/*/README.md`, `fuzz/README.md`, editors READMEs: not re-verified. `scripts/test_all_examples.sh` does not test examples.
`benches/*.rs` (11 files) are mostly unreferenced. `generate_game_entity` MCP tool is game-specific.
