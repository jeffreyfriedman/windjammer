#!/usr/bin/env bash
# Run tail compiler RED repro gates (expect failures on tip until src/ greens).
# See tests/COMPILER_REPRO_QUEUE.md § P3.214 repro bundle.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

FILTERS=(
  json_is_array_len_owned_value_multipass_must_cargo_check
  multipass_cross_crate_join_url_owned_base_must_cargo_check
  multipass_cross_crate_render_owned_template_helper_must_cargo_check
  multipass_http_hexagonal_adapter_must_use_owned
)

echo "Running ${#FILTERS[@]} RED repro gate(s) (expect failures on tip)..."
PASSED=0
for f in "${FILTERS[@]}"; do
  echo "--- $f ---"
  if cargo test --test all "$f" -- --test-threads=1 2>&1; then
    echo "UNEXPECTED PASS: $f"
    PASSED=$((PASSED + 1))
  else
    echo "expected RED: $f"
  fi
done

if [[ "$PASSED" -gt 0 ]]; then
  echo "Done: $PASSED gate(s) unexpectedly passed — update COMPILER_REPRO_QUEUE.md"
  exit 1
fi
echo "Done: all ${#FILTERS[@]} gates failed as expected (RED on tip)."
