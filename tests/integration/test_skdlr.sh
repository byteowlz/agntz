#!/bin/bash
# Integration tests for skdlr (schedule) commands

set -e

echo "=== Testing skdlr integration ==="

# Check if skdlr is installed
if ! command -v skdlr &> /dev/null; then
    echo "❌ skdlr not installed - skipping tests"
    exit 0
fi

# Build agntz
echo "Building agntz..."
cargo build --quiet 2>/dev/null || cargo build

AGNTZ="./target/debug/agntz"

# Test 1: List schedules
echo -n "Test 1: schedule list... "
$AGNTZ schedule list &> /dev/null && echo "✓" || echo "✗"

# Test 2: Add a schedule
echo -n "Test 2: schedule add... "
$AGNTZ schedule add test-job -s "0 0 * * *" -c "echo test" -d "Test job" &> /dev/null && echo "✓" || echo "✗"

# Test 3: Show schedule details
echo -n "Test 3: schedule show... "
$AGNTZ schedule show test-job &> /dev/null && echo "✓" || echo "✗"

# Test 4: List schedules again (should show new one)
echo -n "Test 4: schedule list (after add)... "
OUTPUT=$($AGNTZ schedule list 2>&1)
if echo "$OUTPUT" | grep -q "test-job"; then
    echo "✓"
else
    echo "✗"
fi

# Test 5: Status command
echo -n "Test 5: schedule status... "
$AGNTZ schedule status &> /dev/null && echo "✓" || echo "✗"

# Test 6: Next runs
echo -n "Test 6: schedule next... "
$AGNTZ schedule next &> /dev/null && echo "✓" || echo "✗"

# Test 7: Backend info
echo -n "Test 7: schedule backend... "
$AGNTZ schedule backend &> /dev/null && echo "✓" || echo "✗"

# Test 8: Edit schedule
echo -n "Test 8: schedule edit... "
$AGNTZ schedule edit test-job -d "Updated description" &> /dev/null && echo "✓" || echo "✗"

# Test 9: Run schedule (dry-run would be ideal but check if flag exists)
echo -n "Test 9: schedule run... "
$AGNTZ schedule run test-job --dry-run &> /dev/null && echo "✓" || echo "✗ (dry-run may not be supported)"

# Test 10: Logs
echo -n "Test 10: schedule logs... "
$AGNTZ schedule logs test-job &> /dev/null && echo "✓" || echo "✗"

# Test 11: Disable schedule
echo -n "Test 11: schedule disable... "
$AGNTZ schedule disable test-job &> /dev/null && echo "✓" || echo "✗"

# Test 12: Enable schedule
echo -n "Test 12: schedule enable... "
$AGNTZ schedule enable test-job &> /dev/null && echo "✓" || echo "✗"

# Cleanup
echo -n "Cleanup: removing test schedule... "
$AGNTZ schedule remove test-job -y &> /dev/null && echo "✓" || echo "✗"

echo ""
echo "=== skdlr tests complete ==="
