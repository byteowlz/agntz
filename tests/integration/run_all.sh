#!/bin/bash
# Run all integration tests

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/../.."

echo "========================================"
echo "Running agntz integration tests"
echo "========================================"
echo ""

PASSED=0
FAILED=0

# Run each test script
for test_script in tests/integration/test_*.sh; do
    if [ -f "$test_script" ]; then
        echo ""
        bash "$test_script"
        if [ $? -eq 0 ]; then
            ((PASSED++))
        else
            ((FAILED++))
        fi
    fi
done

echo ""
echo "========================================"
echo "Test Summary"
echo "========================================"
echo "Passed: $PASSED"
echo "Failed: $FAILED"
echo ""

if [ $FAILED -eq 0 ]; then
    echo "✓ All tests passed!"
    exit 0
else
    echo "✗ Some tests failed"
    exit 1
fi
