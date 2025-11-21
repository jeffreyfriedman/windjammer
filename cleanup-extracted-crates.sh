#!/bin/bash
# Cleanup script to remove extracted crates from windjammer repo
# These have been moved to windjammer-ui and game-host

set -e

echo "🧹 Cleaning up extracted crates from windjammer repo..."
echo ""

cd /home/user/src/wj/windjammer

# Remove UI framework (moved to windjammer-ui)
if [ -d "crates/windjammer-ui" ]; then
    echo "❌ Removing crates/windjammer-ui (moved to ../windjammer-ui)"
    rm -rf crates/windjammer-ui
fi

# Remove game framework crates (moved to game-host)
if [ -d "crates/game-host-framework" ]; then
    echo "❌ Removing crates/game-host-framework (moved to ../game-host)"
    rm -rf crates/game-host-framework
fi

if [ -d "crates/game-host-editor" ]; then
    echo "❌ Removing crates/game-host-editor (moved to ../game-host)"
    rm -rf crates/game-host-editor
fi

if [ -d "crates/windjammer-editor-desktop" ]; then
    echo "❌ Removing crates/windjammer-editor-desktop (moved to ../game-host)"
    rm -rf crates/windjammer-editor-desktop
fi

if [ -d "crates/windjammer-editor-web" ]; then
    echo "❌ Removing crates/windjammer-editor-web (moved to ../game-host)"
    rm -rf crates/windjammer-editor-web
fi

if [ -d "crates/windjammer-c-ffi" ]; then
    echo "❌ Removing crates/windjammer-c-ffi (moved to ../game-host)"
    rm -rf crates/windjammer-c-ffi
fi

# Remove SDKs (moved to game-host)
if [ -d "sdks" ]; then
    echo "❌ Removing sdks/ (moved to ../game-host)"
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

