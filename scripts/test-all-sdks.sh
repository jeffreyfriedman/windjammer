#!/bin/bash
# Test all SDK examples in Docker containers

set -e

echo "🧪 Testing All Windjammer SDK Examples"
echo "======================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track results
PASSED=0
FAILED=0
TOTAL=12

# Array of languages
LANGUAGES=("python" "rust" "nodejs" "csharp" "cpp" "go" "java" "kotlin" "lua" "swift" "ruby")

# Test each language
for lang in "${LANGUAGES[@]}"; do
    echo "Testing $lang SDK..."
    
    if docker-compose -f docker-compose.test.yml run --rm "test-$lang"; then
        echo -e "${GREEN}✅ $lang SDK tests PASSED${NC}"
        ((PASSED++))
    else
        echo -e "${RED}❌ $lang SDK tests FAILED${NC}"
        ((FAILED++))
    fi
    
    echo ""
done

# Summary
echo "======================================"
echo "📊 Test Summary"
echo "======================================"
echo -e "Total: $TOTAL"
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some tests failed${NC}"
    exit 1
fi
