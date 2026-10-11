#!/usr/bin/env bash
# Regenerate (or verify) generated documentation. See docs/CONTRIBUTING-docs.md.
#   scripts/docs_gen.sh                  rewrite generated files and <!-- wj:gen --> regions
#   scripts/docs_gen.sh --check          fail on drift (what CI runs)
#   scripts/docs_gen.sh --verify         run every docs check (gen, regions, fences, links, claims, freshness)
#   scripts/docs_gen.sh --update-baseline  re-record ratchet baselines in docs/docs-truth.toml
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
run() { cargo run --release --quiet --bin wj-docs -- "$@"; }
case "${1:-}" in
  --check) run gen --check ;;
  --verify) run check ;;
  --update-baseline) run update-baseline ;;
  "") run gen ;;
  *) echo "usage: $0 [--check|--verify|--update-baseline]" >&2; exit 2 ;;
esac
