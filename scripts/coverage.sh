#!/usr/bin/env bash
# Line/branch coverage of src/ with cargo-llvm-cov, aggregated per top-level module, ratcheted against
# docs/generated/coverage.json (docs/testing/INVARIANTS.md, "Coverage ratchet").
#
#   scripts/coverage.sh --check  [FILTER...]   measure, fail if any module's line coverage dropped
#   scripts/coverage.sh --update [FILTER...]   measure and rewrite the baseline (only after an increase)
#
# FILTER: libtest filters for `--test all` (default: the user-facing acceptance suites). The whole suite
# works too (`scripts/coverage.sh --check ''`) but takes hours on a loaded machine; CI runs it nightly.
# Needs `cargo install cargo-llvm-cov` and `rustup component add llvm-tools-preview`.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
mode="${1:---check}"; shift || true
filters=("$@")
[ ${#filters[@]} -eq 0 ] && filters=(promise_registry_test:: installed_binary_smoke_test:: fmt_fix_semantics_test:: check_build_differential_test::)
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
out="$(mktemp -d)"; trap 'rm -rf "$out"' EXIT
cargo llvm-cov clean --workspace >/dev/null 2>&1 || true
# Tests run the real `wj` binary; llvm-cov instruments it and collects its profiles too.
cargo llvm-cov --release --test all --json --summary-only --output-path "$out/cov.json" -- "${filters[@]}" >"$out/log" 2>&1 \
  || { tail -40 "$out/log"; echo "coverage run failed (test failures above?)"; exit 1; }
"${WJ:-wj}" run scripts/wj/coverage_report.wj "$mode" "$out/cov.json" docs/generated/coverage.json "${filters[*]}"
