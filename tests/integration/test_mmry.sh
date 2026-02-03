#!/bin/bash
# Integration tests for mmry (memory) commands

set -e

echo "=== Testing mmry integration ==="

# Check if mmry is installed
if ! command -v mmry &> /dev/null; then
    echo "❌ mmry not installed - skipping tests"
    exit 0
fi

# Build agntz
echo "Building agntz..."
cargo build --quiet 2>/dev/null || cargo build

AGNTZ="./target/debug/agntz"
STORE="agntz_test_store"

# Initialize a test store
echo "Setting up test store..."
mmry init --store "$STORE" --force &> /dev/null || true

# Test 1: List memories (should be empty)
echo -n "Test 1: memory list... "
MMRY_STORE="$STORE" $AGNTZ memory list &> /dev/null && echo "✓" || echo "✗"

# Test 2: Add a memory
echo -n "Test 2: memory add... "
MMRY_STORE="$STORE" $AGNTZ memory add "test memory content" -c "test" -i 5 &> /dev/null && echo "✓" || echo "✗"

# Test 3: Search memories
echo -n "Test 3: memory search... "
OUTPUT=$(MMRY_STORE="$STORE" $AGNTZ memory search "test" --json 2>&1)
if echo "$OUTPUT" | grep -q '"memories"\|"result"'; then
    echo "✓"
else
    echo "✗"
fi

# Test 4: List memories (should have one)
echo -n "Test 4: memory list (after add)... "
OUTPUT=$(MMRY_STORE="$STORE" $AGNTZ memory list --json 2>&1)
if echo "$OUTPUT" | grep -q "test memory"; then
    echo "✓"
else
    echo "✗"
fi

# Test 5: List by category
echo -n "Test 5: memory list -c category... "
OUTPUT=$(MMRY_STORE="$STORE" $AGNTZ memory list -c "test" --json 2>&1)
if echo "$OUTPUT" | grep -q "test memory"; then
    echo "✓"
else
    echo "✗"
fi

# Test 6: Stats
echo -n "Test 6: memory stats... "
MMRY_STORE="$STORE" $AGNTZ memory stats &> /dev/null && echo "✓" || echo "✗"

# Test 7: Export JSON
echo -n "Test 7: memory export json... "
MMRY_STORE="$STORE" $AGNTZ memory export -o /tmp/test_export.json &> /dev/null && echo "✓" || echo "✗"

# Test 8: Export markdown
echo -n "Test 8: memory export markdown... "
MMRY_STORE="$STORE" $AGNTZ memory export -o /tmp/test_export.md -f md &> /dev/null && echo "✓" || echo "✗"

# Test 9: Add another memory with tags
echo -n "Test 9: memory add with tags... "
MMRY_STORE="$STORE" $AGNTZ memory add "another memory" -c "test" -t "tag1,tag2" &> /dev/null && echo "✓" || echo "✗"

# Test 10: List stores
echo -n "Test 10: memory stores... "
$AGNTZ memory stores &> /dev/null && echo "✓" || echo "✗"

# Cleanup
echo -n "Cleanup: removing test data... "
rm -f /tmp/test_export.json /tmp/test_export.md
rm -rf ~/.local/share/mmry/stores/$STORE 2>/dev/null || true
echo "✓"

echo ""
echo "=== mmry tests complete ==="
