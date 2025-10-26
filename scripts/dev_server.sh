#!/bin/bash
# Simple development server for Windjammer UI apps
# Serves files with proper MIME types and watches for changes

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PORT="${1:-8080}"
DIR="${2:-.}"

echo -e "${BLUE}🚀 Windjammer Dev Server${NC}"
echo -e "${GREEN}Starting server on http://localhost:${PORT}${NC}"
echo -e "${YELLOW}Serving files from: ${DIR}${NC}"
echo ""
echo "Press Ctrl+C to stop"
echo ""

# Check if Python 3 is available
if command -v python3 &> /dev/null; then
    cd "$DIR"
    python3 -m http.server "$PORT"
elif command -v python &> /dev/null; then
    cd "$DIR"
    python -m http.server "$PORT"
else
    echo "Error: Python is required to run the dev server"
    echo "Please install Python 3"
    exit 1
fi

