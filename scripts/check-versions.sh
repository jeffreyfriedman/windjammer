#!/usr/bin/env bash
# check-versions.sh — Verify all workspace crate versions are consistent
#
# Usage:
#   ./scripts/check-versions.sh          # Check workspace consistency
#   ./scripts/check-versions.sh v0.41.0  # Also check against a tag version
#
# Returns exit code 0 if all versions match, 1 if there's a mismatch.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors (if terminal supports it)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    NC=''
fi

errors=0

# 1. Get the workspace version from root Cargo.toml
WORKSPACE_VERSION=$(grep '^version = "' "$ROOT_DIR/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/')

if [ -z "$WORKSPACE_VERSION" ]; then
    echo -e "${RED}ERROR: Could not find workspace version in Cargo.toml${NC}"
    exit 1
fi

echo "Workspace version: $WORKSPACE_VERSION"

# 2. Check workspace crate dependency references
for crate_dir in "$ROOT_DIR"/crates/*/; do
    cargo_toml="$crate_dir/Cargo.toml"
    [ -f "$cargo_toml" ] || continue
    
    rel_path="${cargo_toml#$ROOT_DIR/}"
    crate_name=$(basename "$crate_dir")
    
    # Find lines that have both 'path =' and 'version =' (internal workspace deps)
    while IFS= read -r line; do
        # Extract version from lines like: windjammer = { path = "../..", version = "0.41.0" }
        dep_version=$(echo "$line" | sed -n 's/.*version = "\([^"]*\)".*/\1/p')
        dep_name=$(echo "$line" | sed -n 's/^\([a-z_-]*\) *=.*/\1/p')
        
        if [ -n "$dep_version" ] && [ "$dep_version" != "$WORKSPACE_VERSION" ]; then
            echo -e "${RED}ERROR: $rel_path: dependency '$dep_name' has version $dep_version (expected $WORKSPACE_VERSION)${NC}"
            errors=$((errors + 1))
        fi
    done < <(grep 'path =' "$cargo_toml" | grep 'version =' 2>/dev/null || true)
done

# 3. If a tag version was provided, check against it
if [ $# -ge 1 ]; then
    TAG_VERSION="${1#v}"  # Strip leading 'v' if present
    echo ""
    echo "Checking against tag version: $TAG_VERSION"
    
    if [ "$TAG_VERSION" != "$WORKSPACE_VERSION" ]; then
        echo -e "${RED}ERROR: Tag version ($TAG_VERSION) doesn't match workspace version ($WORKSPACE_VERSION)${NC}"
        echo "  Update Cargo.toml: version = \"$TAG_VERSION\""
        errors=$((errors + 1))
    else
        echo -e "${GREEN}Tag version matches workspace version${NC}"
    fi
fi

# 4. Summary
echo ""
if [ $errors -gt 0 ]; then
    echo -e "${RED}FAILED: Found $errors version mismatch(es)${NC}"
    exit 1
else
    echo -e "${GREEN}All versions consistent ($WORKSPACE_VERSION)${NC}"
    exit 0
fi
