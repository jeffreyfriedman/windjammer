#!/bin/bash
# Windjammer Version Tagging Script

set -e  # Exit on error

echo "🏷️  Windjammer Version Tagging"
echo "================================"
echo ""

# Tag v0.1.0 on main branch (initial release)
echo "📌 Step 1: Tagging v0.1.0 on main..."
git checkout main

git tag -a v0.1.0 8bae99b -m "Release v0.1.0 - Initial Windjammer Compiler

Features:
- Core compiler pipeline (lexer, parser, analyzer, codegen)
- Automatic ownership inference
- String interpolation (\${expr})
- Pipe operator (|>)
- Explicit @auto derive decorator
- Go-style concurrency (go keyword, channels)
- Pattern matching with guards
- Trait system (definitions and implementations)
- Closures, ranges, for loops
- 8/9 tests passing

Examples:
- hello_world (working)
- http_server, wasm_game, cli_tool (need fixes)

License: MIT OR Apache-2.0"

echo "✅ Tagged v0.1.0 on main branch"
echo ""

# Show tag info
echo "📋 Tag Details:"
git show v0.1.0 --no-patch --format="%H %s%n%n%b"
echo ""

# Ask before pushing
read -p "Push v0.1.0 tag to GitHub? (y/n) " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Yy]$ ]]
then
    echo "🚀 Pushing v0.1.0 to GitHub..."
    git push origin v0.1.0
    echo "✅ v0.1.0 pushed successfully!"
    echo ""
    echo "🌐 Create GitHub release: https://github.com/jeffreyfriedman/windjammer/releases/new?tag=v0.1.0"
else
    echo "⏸️  Skipped pushing. Run 'git push origin v0.1.0' when ready."
fi

echo ""
echo "================================"
echo "✨ v0.1.0 tagging complete!"
echo ""
echo "Next steps:"
echo "1. Merge feature branch to main"
echo "2. Update Cargo.toml to version = \"0.2.0\""
echo "3. Update CHANGELOG.md"
echo "4. Commit version bump"
echo "5. Tag v0.2.0"
echo "6. Push tags"

