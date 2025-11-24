#!/bin/bash
# Sync version number across all relevant files
# Usage: ./scripts/sync-version.sh [version]
# Example: ./scripts/sync-version.sh 0.36.1

set -e

VERSION="${1}"

if [ -z "$VERSION" ]; then
    # Extract version from workspace Cargo.toml
    VERSION=$(grep -A 1 '^\[workspace.package\]' Cargo.toml | grep '^version' | sed 's/version = "\(.*\)"/\1/')
    echo "📦 Detected version from Cargo.toml: $VERSION"
else
    echo "📦 Using provided version: $VERSION"
fi

if [ -z "$VERSION" ]; then
    echo "❌ Error: Could not determine version"
    echo "Usage: $0 [version]"
    exit 1
fi

echo "🔄 Syncing version $VERSION across project files..."

# Update README.md
if grep -q "Current Version:" README.md; then
    sed -i '' "s/Current Version:\*\* [0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*/Current Version:** $VERSION/" README.md
    echo "✅ Updated README.md"
else
    echo "⚠️  Warning: Could not find version in README.md"
fi

# Update ROADMAP.md if it has a version reference
if [ -f "ROADMAP.md" ] && grep -q "Current Version" ROADMAP.md; then
    sed -i '' "s/Current Version: [0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*/Current Version: $VERSION/" ROADMAP.md
    echo "✅ Updated ROADMAP.md"
fi

# Update installation instructions in README if present
if grep -q "cargo install windjammer --version" README.md; then
    sed -i '' "s/cargo install windjammer --version [0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*/cargo install windjammer --version $VERSION/" README.md
    echo "✅ Updated installation command in README.md"
fi

# Update crate READMEs
for crate_readme in crates/*/README.md; do
    if [ -f "$crate_readme" ]; then
        if grep -q "Windjammer v[0-9]" "$crate_readme"; then
            sed -i '' "s/Windjammer v[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*/Windjammer v$VERSION/" "$crate_readme"
            echo "✅ Updated $crate_readme"
        fi
    fi
done

echo ""
echo "✨ Version sync complete!"
echo ""
echo "📝 Files updated:"
git diff --name-only | grep -E '\.(md|toml)$' || echo "  (no changes detected)"
echo ""
echo "🔍 Review changes with: git diff"
echo "💾 Commit with: git add -A && git commit -m 'chore: Sync version to $VERSION'"

