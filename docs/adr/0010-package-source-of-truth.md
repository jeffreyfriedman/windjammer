---
status: proposed
last_verified: 3406ef05bb35
---
# ADR-0010: Package source of truth

- **Status:** Proposed
- **Date:** 2026-10-10
- **Related:** [WJ-PKG-01](../proposals/rfcs/WJ-PKG-01-package-management.md), `src/config.rs` (`DependencySpec`), `src/package/manifest.rs`, `src/cli/{add,remove,update}.rs`

## Context
`wj.toml` already parses `dependencies` with `version`, `path`, `git`, `branch`, `features`, `package` (a Cargo rename) and `registry`.
`wj add`/`wj remove` edit that file. `src/package/` holds only manifest parsing: an earlier resolver stub ("accepts declared versions as-is") and registry client stub
(no HTTP calls, defaulting to `wj-registry.io`, which is not a service we operate) were removed as dead code. There is no `wj.lock` or
`.wj-capabilities.lock` writer. Multi-package builds in practice rely on path dependencies plus `--module-file` and generated `Cargo.toml`
(Cargo does the actual resolution). `wj publish` does not exist. Dogfood projects live in sibling repositories.

## Options
### A. Path and git first; lock file; registry later
Resolve only path and git dependencies, in dependency order, writing a `wj.lock` (name, source, commit/hash, resolved Cargo features). Cargo remains the build engine and `[rust-dependencies]` pass through (ADR-0012). A registry becomes a later, separate ADR once the lock format is stable.
### B. Registry first (WJ-PKG-01 as written)
Pros: complete story. Cons: needs hosting, auth, yanking and security review; blocks everything else; the stub proves nothing.
### C. Cargo is the package manager; `wj` only generates `Cargo.toml`
Pros: least work. Cons: `.wj` packages cannot be distinguished from Rust crates; no effect/taint metadata travels with dependencies (ADR-0009).

## Recommendation
Option A. Define the lock file schema and semantic-version rules first; make path and git dependencies work through `wj add`; put the registry client behind a
feature flag until a registry exists. Remove the default registry URL from code so nothing implies a service.

## Consequences
- `src/package/` is renamed/trimmed to what exists (manifest + path/git resolver); README stops advertising a package manager.
- Capability lock (ADR-0009) piggybacks on `wj.lock`.
- A dependency must compile before its dependents (build ordering in the driver, plan WP 13).

## Decision needed from: project owner
Approve A, and the lock file name/location (`wj.lock` next to `wj.toml`).
