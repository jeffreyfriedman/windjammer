#!/usr/bin/env bash
# Run tail compiler RED repro gates (expect failures on tip until src/ greens).
# See tests/COMPILER_REPRO_QUEUE.md § P3.205 repro bundle.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

FILTERS=(
  wdb112_full_library
  wdb113_full_library
  wdb114_module_file_vec_type_annotation
  cross_crate_vec_string_helper
  hashmap_field_get_i64_key
  std_fs_dir_entry_name
  std_fs_dir_entry_name_multipass
  app_test_http_method_public_port_must_pass_wj_test
  app_module_file_http_method_public_port_must_pass_wj_test
)

echo "Running ${#FILTERS[@]} RED repro gates (expect failures on tip)..."
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
