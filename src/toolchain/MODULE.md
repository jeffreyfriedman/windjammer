# src/toolchain/

## Purpose
Knowing which `wj` you are running and whether it is the right one: build identity, project pins, age warnings, `wj doctor`, `wj install-dev`.

## Inputs/Outputs
In: `build.rs` identity (`WJ_GIT_*`, `WJ_BUILD_*`), `wj.toml` / `WJ_PIN.md`, `PATH`, the clock. Out: version text and JSON, pin errors, a once-per-day stderr warning, a doctor report, an atomically installed binary.

## Invariants
- The commit date is the only time that may reach generated output; the wall-clock build date is read in `identity.rs` only and printed by `--version` (gate: tests/toolchain_identity_test.rs).
- A `wj` younger than 30 days returns from `stale::warn_if_stale` after one clock read and a subtraction (hot path); only older binaries touch the stamp under the cache dir.
- A pin mismatch is an error for build/run/test/check unless `WJ_ALLOW_TOOLCHAIN_MISMATCH=1`; `WJ_NO_STALE_WARN=1` silences the age warning.
- Installs stage next to the destination, verify `--version`, keep a timestamped `.bak` (last 5), and refuse downgrades without `--force`.

## Owns
Identity rendering and parsing, pin lookup and matching, staleness thresholds, PATH scanning, installation.

## Never decides
Compilation semantics. It never changes what a build produces.

## Allowed imports
Layer `driver`. May import: `cargo_cache` (cache root). No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`identity::{current, parse_version_line}`, `pin::{find_pin, check, enforce}`, `stale::warn_if_stale`, `doctor::{collect, render_text, render_json, expect_head}`, `install::install`. Wired in src/bin/wj.rs (`--version`, pre-dispatch checks) and src/bin/wj_cli_cmds.rs (`version`, `doctor`, `install-dev`).

## Tests
Inline unit tests per file; tests/toolchain_identity_test.rs, tests/toolchain_pin_test.rs, tests/toolchain_stale_test.rs, tests/toolchain_doctor_test.rs, tests/toolchain_install_test.rs (the `wj` binary end to end).
Run: `cargo test --release --test all toolchain`; architecture rules: `cargo test --release --test all architecture`.
