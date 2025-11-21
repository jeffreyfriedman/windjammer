#!/bin/bash
# Cleanup script to remove extracted crates from windjammer repo
# These have been moved to windjammer-ui and windjammer-game

set -e

echo "🧹 Cleaning up extracted crates from windjammer repo..."
echo ""

cd /Users/jeffreyfriedman/src/wj/windjammer

# Remove UI framework (moved to windjammer-ui)
if [ -d "crates/windjammer-ui" ]; then
    echo "❌ Removing crates/windjammer-ui (moved to ../windjammer-ui)"
    rm -rf crates/windjammer-ui
fi

# Remove game framework crates (moved to windjammer-game)
if [ -d "crates/windjammer-game-framework" ]; then
    echo "❌ Removing crates/windjammer-game-framework (moved to ../windjammer-game)"
    rm -rf crates/windjammer-game-framework
fi

if [ -d "crates/windjammer-game-editor" ]; then
    echo "❌ Removing crates/windjammer-game-editor (moved to ../windjammer-game)"
    rm -rf crates/windjammer-game-editor
fi

if [ -d "crates/windjammer-editor-desktop" ]; then
    echo "❌ Removing crates/windjammer-editor-desktop (moved to ../windjammer-game)"
    rm -rf crates/windjammer-editor-desktop
fi

if [ -d "crates/windjammer-editor-web" ]; then
    echo "❌ Removing crates/windjammer-editor-web (moved to ../windjammer-game)"
    rm -rf crates/windjammer-editor-web
fi

if [ -d "crates/windjammer-c-ffi" ]; then
    echo "❌ Removing crates/windjammer-c-ffi (moved to ../windjammer-game)"
    rm -rf crates/windjammer-c-ffi
fi

# Remove SDKs (moved to windjammer-game)
if [ -d "sdks" ]; then
    echo "❌ Removing sdks/ (moved to ../windjammer-game)"
    rm -rf sdks
fi

echo ""
echo "✅ Cleanup complete!"
echo ""
echo "Remaining crates in windjammer/crates/:"
ls -1 crates/
echo ""
echo "⚠️  NEXT STEPS:"
echo "1. Update Cargo.toml workspace members"
echo "2. Test compilation: cargo build"
echo "3. Clean git history (see MASTER_SESSION.md)"

