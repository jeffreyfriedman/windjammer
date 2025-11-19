#!/bin/bash
set -e

echo "🧪 Testing All Windjammer SDKs"
echo "================================"

cd "$(dirname "$0")/.."

# Array of SDK languages
SDKS=("python" "javascript" "go" "java" "csharp" "cpp" "kotlin")

# Track results
PASSED=()
FAILED=()

for sdk in "${SDKS[@]}"; do
    echo ""
    echo "Testing $sdk SDK..."
    echo "-------------------"
    
    if docker-compose -f docker/docker-compose.test.yml run --rm "test-$sdk"; then
        PASSED+=("$sdk")
        echo "✅ $sdk SDK tests passed"
    else
        FAILED+=("$sdk")
        echo "❌ $sdk SDK tests failed"
    fi
done

echo ""
echo "================================"
echo "Test Summary"
echo "================================"
echo "✅ Passed: ${#PASSED[@]}"
for sdk in "${PASSED[@]}"; do
    echo "  - $sdk"
done

if [ ${#FAILED[@]} -gt 0 ]; then
    echo "❌ Failed: ${#FAILED[@]}"
    for sdk in "${FAILED[@]}"; do
        echo "  - $sdk"
    done
    exit 1
else
    echo ""
    echo "🎉 All SDK tests passed!"
    exit 0
fi

