#!/usr/bin/env bash
# Regenerate docs/generated/deps.{md,dot} (module dependency matrix/graph) and ratchet the
# architecture baseline (docs/architecture-allowlist.toml) DOWN to the current measurements.
# The generator refuses to raise any baseline number. Source of truth: tests/architecture.rs.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test --release --test all architecture::regenerate_architecture_baselines -- --ignored
echo "Updated docs/generated/deps.md, docs/generated/deps.dot, docs/architecture-allowlist.toml"
