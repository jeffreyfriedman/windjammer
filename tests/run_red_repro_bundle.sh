#!/usr/bin/env bash
# Run tail compiler RED repro gates (expect failures on tip until src/ greens).
# See tests/COMPILER_REPRO_QUEUE.md § P3.211 repro bundle.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

FILTERS=(
  std_fs_dir_entry_name_multipass
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
echo "Done: all ${#FILTERS[@]} gate(s) failed as expected (RED on tip)."
