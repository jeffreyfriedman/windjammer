#!/bin/bash
# Serve script for Windjammer Web Editor

set -e

echo "🚀 Starting Windjammer Web Editor..."

# Check if pkg directory exists
if [ ! -d "pkg" ]; then
    echo "📦 pkg directory not found, building first..."
    ./build.sh
fi

echo "🌐 Starting HTTP server on http://localhost:8080"
echo "Press Ctrl+C to stop"
echo ""

# Start server
python3 -m http.server 8080

