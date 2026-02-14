#!/bin/bash
# Install Windjammer git hooks
#
# Usage: ./scripts/install-hooks.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_DIR="$REPO_ROOT/.git/hooks"

echo "📦 Installing Windjammer git hooks..."
echo ""

# Check if we're in a git repository
if [ ! -d "$REPO_ROOT/.git" ]; then
    echo "❌ Error: Not in a git repository"
    exit 1
fi

# Create hooks directory if it doesn't exist
mkdir -p "$HOOKS_DIR"

# Install pre-commit hook
echo "Installing pre-commit hook..."
cp "$SCRIPT_DIR/hooks/pre-commit" "$HOOKS_DIR/pre-commit"
chmod +x "$HOOKS_DIR/pre-commit"
echo "✅ pre-commit hook installed"
echo ""

echo "✅ All hooks installed successfully!"
echo ""
echo "The pre-commit hook will now run:"
echo "  • cargo fmt --all (auto-fix + stage)"
echo "  • cargo clippy --all-targets --all-features"
echo "  • cargo test"
echo ""
echo "To skip the hook (not recommended), use: git commit --no-verify"
echo ""

