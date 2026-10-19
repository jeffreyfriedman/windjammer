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
step "rustfmt" cargo fmt --all -- --check
step "test binary" cargo test --release --test all --no-run -q
step "architecture" t architecture::
step "docs-truth" t docs_truth
step "promise registry (acceptance through the installed binary)" t promise_registry_test::
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
step "orphan tests" scripts/check-orphan-tests.sh
if [ $full = 1 ]; then
  step "backend conformance" t backend_conformance_test::
  step "whole suite (ratchet: only new failures)" scripts/check-test-ratchet.sh --release --test all
fi

echo
if [ ${#failed[@]} -eq 0 ]; then
  echo "merge gate: PASS"
else
  echo "merge gate: FAIL (${#failed[@]}):"
  printf '  - %s\n' "${failed[@]}"
  exit 1
fi
