#!/usr/bin/env bash
# Recompute the architecture baselines and generated dependency graph.
#   scripts/update-architecture-baselines.sh            shrink only: refuses to raise any number
#   scripts/update-architecture-baselines.sh --rebase   reset baselines to the measured numbers
#                                                       (integrator only, after merging branches)
# Writes docs/architecture-allowlist.toml (the single baseline data file) and
# docs/generated/deps.{md,dot}. New top-level src modules must first be added to
# docs/MODULE_MAP.toml (layer, allowed_imports) with a MODULE.md.
# Source of truth: tests/architecture.rs.
set -euo pipefail
cd "$(dirname "$0")/.."
case "${1:-}" in
  --rebase) test_name=rebaseline_architecture_baselines ;;
  "")       test_name=regenerate_architecture_baselines ;;
  *) echo "usage: $0 [--rebase]" >&2; exit 2 ;;
esac
cargo test --release --test all "architecture::${test_name}" -- --ignored
echo "Updated docs/architecture-allowlist.toml, docs/generated/deps.md, docs/generated/deps.dot"
