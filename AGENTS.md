# Agent instructions

**Writing Windjammer code (not working on the compiler)?** Read [docs/agent/WINDJAMMER_FOR_AGENTS.md](docs/agent/WINDJAMMER_FOR_AGENTS.md); the machine-readable index is [llms.txt](llms.txt) and the inline version is [llms-full.txt](llms-full.txt). The rest of this file is for changing this repository.

Orientation: [docs/README.md](docs/README.md) (index), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), per-module `MODULE.md`
files under `src/` (one module's context should be enough to work on it), [docs/known-gaps.md](docs/known-gaps.md).

## Architecture in one paragraph

The IR solver (`src/ir/`) is the single source of truth for ownership, clone insertion and integer widths
([ADR-0001](docs/adr/0001-ownership-truth.md)). Legacy analyzer/codegen heuristics are being deleted, not extended.
Details and migration status: [docs/IR_SOLVER_CODEGEN_ARCHITECTURE.md](docs/IR_SOLVER_CODEGEN_ARCHITECTURE.md).

## Bugs and issues

Compiler bugs are not tracked in the repository tree ([ADR-0013](docs/adr/0013-bug-triage-tracker.md)). Add a minimal,
generic failing test under `tests/` (`bug_<behavior>_test.rs`), then fix it. Known failures live in
`.config/known-test-failures.txt` (a ratchet: it only shrinks). `bd` is an optional local tracker.

## Tests

Add `tests/<name>_test.rs`; `build.rs` picks it up automatically (see `tests/README.md`).

```bash
cargo test --release --test all <filter>    # targeted
wj run scripts/wj/check_orphan_tests.wj      # every test file must be reached by a runner
scripts/update-architecture-baselines.sh    # after adding modules/files (also declare new top-level modules in src/lib.rs and src/main.rs)
```

Never run tests with `--all-features` (it silently disables many test files; use it for clippy only).

## Shared cargo cache

<!-- ignore: machine-level cache commands: they change the user's environment, so CI does not run them -->
```bash,ignore
wj cache status | wj cache prune | wj cache path | wj cache setup
export CARGO_TARGET_DIR="$(wj cache path)"
```
Design: [docs/superpowers/specs/2026-09-04-shared-cargo-cache-design.md](docs/superpowers/specs/2026-09-04-shared-cargo-cache-design.md).

## Hardening rules

1. One decision-maker: ownership, clones, borrows and numeric widths are decided by the IR solver. No method-name string
   matches (`"push"`, `"clone"`) or special cases in `src/codegen`; add a constraint or signature instead.
2. Tests exercise production paths; no test-only defaults.
3. Done means whole-program green: a fix must pass the isolated test and the multi-file test.
4. Never silently change semantics; prefer a Windjammer diagnostic over code that compiles but behaves differently.
5. Repros are generic: name tests by behavior, never by a downstream project or ticket. The compiler never references a
   downstream project's code or name.
6. Minimal repros, one bug each; delete duplicates a broader test subsumes.
7. Tests write to temp dirs only; no `*.rlib`, `tmp_*`, binaries or archives in the repo.
8. No status/session docs in the repo; user-visible changes go in `CHANGELOG.md`.
9. Gates before commit: `cargo fmt --check`, `cargo clippy -- -D warnings`, targeted tests, `tests/architecture.rs`.
10. Docs honesty: README and CLI help claim only what exists and is tested. Mark experimental backends (JS, Go, WASM,
    interpreter) as such.
11. Promises are tested, not assumed: every `wj` subcommand and flag, documented error code and README feature claim
    is listed in `docs/promises.toml` and has a positive AND a negative acceptance case in `tests/promises/*.toml`
    that runs through the installed binary. Adding a flag without both fails `tests/promise_registry_test.rs`.
12. Code no user can reach is a bug: `tests/capability_reachability_test.rs` fails on new modules or pub fns that no
    entry point (`wj`, `wj-lint`, LSP, MCP) reaches. Wire it into `wj`, declare it in `docs/MODULE_MAP.toml`
    `[library_api]`, or delete it. The legacy `windjammer` binary may not grow a subcommand `wj` lacks.
13. A bug found by inspection lands with a class-level gate. If you notice a defect by reading code, reading output or
    spot-checking (a wrong column, an off-by-one line, a dummy span, a panic on odd input), a one-off regression test is
    not enough: add or extend an automated gate that fails for the whole class (an independent oracle, a corpus sweep,
    or a mutation-tested invariant), prove it can fail (mutation self-test), and list it in `docs/testing/INVARIANTS.md`.
    Ratchet files (`tests/span_oracle_ratchet/`) may only shrink.

## Merge gate (mandatory)

Run before merging any branch; targeted tests are not enough (that is how eject on the main CLI, `wj check` on type errors and
`wj new` outside a checkout broke without anyone noticing). Gate list and oracles: [docs/testing/INVARIANTS.md](docs/testing/INVARIANTS.md).

```bash
scripts/merge_gate.sh           # build, fmt, architecture, docs-truth, promises, reachability, differential gates, smoke
scripts/merge_gate.sh --full    # + cargo-backed cases, all backends, 6x determinism, whole-suite ratchet
```

## Scripts

Repository tooling is written in Windjammer (`scripts/wj/`, run with `wj run scripts/wj/<name>.wj`; it uses the packages in
`windjammer-ecosystem/`, checked out or symlinked at the repository root). A Python or shell step is an exception recorded in
[docs/design/python-replacement-audit.md](docs/design/python-replacement-audit.md). Every port is pinned to its original by the
recorded cases in `tests/py_ports/` (`wj run scripts/wj/port_gates.wj --wj target/release/wj`).
