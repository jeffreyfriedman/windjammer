#!/bin/bash
# Script to graft clean-public history onto main's history
# This preserves your development history while having a clean public state

set -e

cd "$(dirname "$0")"

echo "🔍 Finding commits..."

# Find the last commit before UI/game frameworks (v0.32.0 - multi-target compilation)
# This is pure language work, before proprietary code
PARENT_COMMIT="31b8f737"  # v0.32.0 - good stopping point

# Find the first (orphan) commit on clean-public  
CLEAN_COMMIT=$(git log clean-public --oneline | tail -1 | cut -d' ' -f1)

echo "📍 Parent commit (last good commit from main): $PARENT_COMMIT"
echo "   $(git log -1 --oneline $PARENT_COMMIT)"
echo ""
echo "📍 Clean commit (first commit on clean-public): $CLEAN_COMMIT"
echo "   $(git log -1 --oneline $CLEAN_COMMIT)"
echo ""

# Create the graft (makes clean-public appear to continue from main)
echo "🔗 Grafting histories together..."
git replace --graft $CLEAN_COMMIT $PARENT_COMMIT

# Create a new branch with the grafted history
echo "🌿 Creating new branch 'main-grafted' with combined history..."
git checkout clean-public
git branch -f main-grafted clean-public

echo ""
echo "✅ Success! The histories are now connected."
echo ""
echo "📊 View the combined history:"
echo "   git log main-grafted --oneline | head -20"
echo ""
echo "🔄 To make this permanent:"
echo "   git filter-repo --force --refs main-grafted"
echo ""
echo "❌ To undo the graft:"
echo "   git replace -d $CLEAN_COMMIT"
echo ""
echo "Current branches:"
echo "  - main: Your original history (unchanged)"
echo "  - clean-public: Clean orphan history (unchanged)" 
echo "  - main-grafted: Combined history (uses git replace)"

