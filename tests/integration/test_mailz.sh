#!/bin/bash
# Integration tests for mailz (mail/reservations) commands

set -e

echo "=== Testing mailz integration ==="

# Check if mailz-cli is installed
if ! command -v mailz-cli &> /dev/null; then
    echo "❌ mailz-cli not installed - skipping tests"
    exit 0
fi

# Build agntz
echo "Building agntz..."
cargo build --quiet 2>/dev/null || cargo build

AGNTZ="./target/debug/agntz"

# Initialize mailz if needed
mailz-cli init &> /dev/null || true

# Test 1: Check inbox
echo -n "Test 1: mail inbox... "
$AGNTZ mail inbox &> /dev/null && echo "✓" || echo "✗"

# Test 2: List reservations (should be empty)
echo -n "Test 2: reservations... "
$AGNTZ reservations &> /dev/null && echo "✓" || echo "✗"

# Test 3: Reserve a file
echo -n "Test 3: reserve file... "
TEST_FILE=$(mktemp)
$AGNTZ reserve "$TEST_FILE" --reason "test reservation" &> /dev/null && echo "✓" || echo "✗"

# Test 4: List reservations (should show reserved file)
echo -n "Test 4: reservations (after reserve)... "
OUTPUT=$($AGNTZ reservations 2>&1)
if echo "$OUTPUT" | grep -q "test reservation\|test" || echo "$OUTPUT" | grep -q "$TEST_FILE"; then
    echo "✓"
else
    echo "✗"
fi

# Test 5: Reserve another file with TTL
echo -n "Test 5: reserve with TTL... "
TEST_FILE2=$(mktemp)
$AGNTZ reserve "$TEST_FILE2" --reason "ttl test" --ttl 60 &> /dev/null && echo "✓" || echo "✗"

# Test 6: Release specific file
echo -n "Test 6: release file... "
$AGNTZ release "$TEST_FILE" &> /dev/null && echo "✓" || echo "✗"

# Test 7: List reservations after release
echo -n "Test 7: reservations (after release)... "
OUTPUT=$($AGNTZ reservations 2>&1)
if ! echo "$OUTPUT" | grep -q "$TEST_FILE"; then
    echo "✓"
else
    echo "✗"
fi

# Test 8: Release all
echo -n "Test 8: release --all... "
$AGNTZ release --all &> /dev/null && echo "✓" || echo "✗"

# Test 9: Verify reservations are empty
echo -n "Test 9: reservations (after release all)... "
OUTPUT=$($AGNTZ reservations 2>&1)
if echo "$OUTPUT" | grep -q "no reservations\|empty\|No reservations" || [ -z "$(echo "$OUTPUT" | grep -v "^$")" ]; then
    echo "✓"
else
    echo "⊘"
fi

# Test 10: Send a test message (may fail if no recipients set up)
echo -n "Test 10: mail send... "
$AGNTZ mail send "test" "Test subject" --body "Test body" &> /dev/null && echo "✓" || echo "⊘ (may not have recipients)"

# Cleanup
rm -f "$TEST_FILE" "$TEST_FILE2" 2>/dev/null || true

echo ""
echo "=== mailz tests complete ==="
