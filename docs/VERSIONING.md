# Versioning and Releases

This is the single versioning document; it replaces `VERSIONING_POLICY.md` and `VERSIONING_STRATEGY.md` (both archived in
[archive/](archive/README.md)). The cross-repository policy is a proposal: [ADR-0011](adr/0011-release-version-policy.md).

## Current state (verified 2026-10-10)
- The workspace version is `0.50.0` (`[workspace.package] version` in `Cargo.toml`). The compiler (`windjammer`), `windjammer-runtime`,
  `windjammer-lsp` and `windjammer-mcp` use `version.workspace = true`, so they cannot drift.
- `scripts/check-versions.sh` and `scripts/sync-version.sh` check and propagate the version (`make check-versions`).
- Binaries: `wj` (unified CLI), `windjammer` (older CLI, includes `eject`), `wj-lint`.
- The standard library in `std/` is versioned with the compiler; it is not published separately.
- Related projects (`windjammer-ui`, the game engine, the database, example applications) live in other repositories with their own
  versions and pin the compiler themselves.

## Rules in force
- Semantic versioning in spirit, pre-1.0: `0.MINOR.PATCH`. A minor bump may break the language or generated-code expectations;
  a patch bump is fixes only.
- Every user-visible change is recorded in [../CHANGELOG.md](../CHANGELOG.md) under the version it ships in.
- A version is released by: updating the workspace version, `make check-versions`, full test suite against the known-failures
  ratchet (`.config/known-test-failures.txt`), CHANGELOG entry, tag `vX.Y.Z`, then the `release`/`publish` workflows.

## Proposed, not in force
Lockstep core with compatible-range ecosystem repositories, a required-compiler field in `wj.toml`, and a 1.0 gate tied to the
ownership, diagnostics and conformance decisions: see ADR-0011.
