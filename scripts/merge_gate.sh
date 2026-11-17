#!/usr/bin/env bash
# Merge gate: run before merging ANY branch into next/main (mandatory, see AGENTS.md).
#
#   scripts/merge_gate.sh            fast gates (~10-20 min on an idle machine)
#   scripts/merge_gate.sh --full     + cargo-backed promise cases, all backends, 6x determinism, whole suite
#
# Gates (docs/testing/INVARIANTS.md): build, fmt, architecture, docs-truth, promise registry, dead
# capabilities + single driver, differential/metamorphic oracles (check-vs-build, interpreter-vs-Rust,
# codegen determinism, fmt/fix semantics, spec conformance, eject), installed-binary smoke, orphan tests.
# Targeted tests only: a merge that passes "the tests I touched" is how promises broke unnoticed.
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
full=0
[ "${1:-}" = "--full" ] && full=1
if [ $full = 1 ]; then
  export WJ_PROMISES_CARGO=1 WJ_DETERMINISM_FULL=1 WJ_SPEC_FULL=1
fi

failed=()
step() {
  local name="$1"; shift
  echo "==> $name"
  if "$@"; then echo "    ok: $name"; else echo "    FAILED: $name"; failed+=("$name"); fi
}
t() { cargo test --release --test all -q -- "$@"; }

step "build (release binaries)" cargo build --release --bins -q
# The build under test is the toolchain for every later step (interpreted scripts, `wj` in tests): putting it first
# on PATH is the explicit marker, and the next step proves PATH's `wj` is built from this checkout's HEAD, so a
# stale `wj` installed months ago can never stand in for it.
WJ_BIN_DIR="${CARGO_TARGET_DIR:-$PWD/target}/release"
export PATH="$WJ_BIN_DIR:$PATH"
step "toolchain freshness (first wj on PATH is built from HEAD)" "$WJ_BIN_DIR/wj" doctor --expect-head
step "library builds without default features (plugins depend on it this way)" cargo check --release --lib --no-default-features --features cli -q
step "rustfmt" cargo fmt --all -- --check
step "test binary" cargo test --release --test all --no-run -q
step "architecture" t architecture::
step "docs-truth" t docs_truth
step "promise registry (acceptance through the installed binary)" t promise_registry_test::
step "untrusted data into sinks (WJ0173 oracle cases)" t taint_flow_test::
step "untrusted data precision (tests, examples, scripts, pinned ecosystem snapshot)" wj run --allow process --allow 'fs.read=**' --allow env.read scripts/wj/taint_precision.wj "$WJ_BIN_DIR/wj"
step "effect spellings (module-dot calls = path calls)" t effect_spelling_test::
step "effect spelling gate (check, build, run refuse an ungranted effect in both spellings)" wj run --allow process --allow 'fs.read=**' --allow 'fs.write=**' scripts/wj/effect_spelling_gate.wj "$WJ_BIN_DIR/wj"
step "disk janitor selftest (fixture tree; removing the in-use check must fail it)" wj run --allow process --allow env.read scripts/wj/disk_janitor.wj -- --selftest
step "slot gate syntax" sh -n scripts/slot_gate.sh
step "build ceiling (tests/perf_ceiling build within 5 s and 2 GB)" wj run --allow process --allow 'fs.read=**' --allow 'fs.write=**' scripts/wj/build_ceiling_gate.wj "$WJ_BIN_DIR/wj"
step "check cost (12 dx programs: instruction ceiling, taint pass free without a sink)" wj run --allow process --allow 'fs.read=**' scripts/wj/check_cost_gate.wj "$WJ_BIN_DIR/wj"
step "target tiers table current (ADR-0007)" wj run --allow process --allow 'fs.read=**' scripts/wj/target_tiers.wj check
step "wj.lock reproducibility (ADR-0010)" wj run --allow process --allow 'fs.read=**' --allow 'fs.write=**' --allow env.read scripts/wj/lock_gate.wj "$WJ_BIN_DIR/wj"
step "dead capabilities" t capability_reachability_test::
step "single driver (legacy windjammer has nothing wj lacks)" t single_driver_parity_test::
step "check vs build differential" t check_build_differential_test::
step "interpreter vs compiled Rust" t interpreter_differential_test::
step "interpreter refuses what check rejects" t interpreter_check_parity_test::
step "codegen determinism" t codegen_determinism_test::
step "fmt/fix preserve behavior" t fmt_fix_semantics_test::
step "spec examples (interpreter, in-process)" t spec_conformance_test::
step "spec examples on every backend" t spec_backends_test::
step "eject == build" t eject_equivalence_test::
step "installed-binary smoke" t installed_binary_smoke_test::
WJ="${WJ:-target/release/wj}"
step "orphan tests" "$WJ" run scripts/wj/check_orphan_tests.wj
step "release scorecard (page and JSON match the definition)" "$WJ" run scripts/wj/release_scorecard.wj -- --check
step "drift gate (walker == resolved == compiled; shrink-only ledger)" "$WJ" run scripts/wj/drift_gate.wj --wj "$WJ"
if [ $full = 1 ]; then
  step "drift gate, compiled side re-run now (the default step compares with the recorded baseline)" "$WJ" run scripts/wj/drift_gate.wj --wj "$WJ" --fast --compiled live
  step "backend conformance" t backend_conformance_test::
  step "whole suite (ratchet: only new failures)" "$WJ" run scripts/wj/check_test_ratchet.wj --release --test all
fi

echo
if [ ${#failed[@]} -eq 0 ]; then
  echo "merge gate: PASS"
else
  echo "merge gate: FAIL (${#failed[@]}):"
  printf '  - %s\n' "${failed[@]}"
  exit 1
fi
