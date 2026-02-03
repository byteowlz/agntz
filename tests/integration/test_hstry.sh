#!/bin/bash
# Integration tests for hstry (search) commands

set -e

echo "=== Testing hstry integration ==="

# Check if hstry is installed
if ! command -v hstry &> /dev/null; then
    echo "❌ hstry not installed - skipping tests"
    exit 0
fi

# Check if hstry service is running
if ! hstry ping &> /dev/null; then
    echo "⚠️  hstry service not running - starting..."
    hstry start &> /dev/null || {
        echo "❌ Could not start hstry service - skipping tests"
        exit 0
    }
    sleep 2  # Give service time to start
fi

# Build agntz
echo "Building agntz..."
cargo build --quiet 2>/dev/null || cargo build

AGNTZ="./target/debug/agntz"

# Test 1: Search (basic)
echo -n "Test 1: search... "
$AGNTZ search "test" &> /dev/null && echo "✓" || echo "✗"

# Test 2: Search with JSON output
echo -n "Test 2: search --json... "
OUTPUT=$($AGNTZ search "test" --json 2>&1)
if echo "$OUTPUT" | grep -q '"ok"'; then
    echo "✓"
else
    echo "✗"
fi

# Test 3: Search with limit
echo -n "Test 3: search --limit... "
$AGNTZ search "test" --limit 5 &> /dev/null && echo "✓" || echo "✗"

# Test 4: Search all workspaces
echo -n "Test 4: search --all-workspaces... "
$AGNTZ search "test" --all-workspaces &> /dev/null && echo "✓" || echo "✗"

# Test 5: Search with workspace filter
echo -n "Test 5: search --workspace... "
$AGNTZ search "test" --workspace "/tmp" &> /dev/null && echo "✓" || echo "✗"

# Test 6: Search with no dedup
echo -n "Test 6: search --no-dedup... "
$AGNTZ search "test" --no-dedup &> /dev/null && echo "✓" || echo "✗"

# Test 7: Search including system context
echo -n "Test 7: search --include-system... "
$AGNTZ search "test" --include-system &> /dev/null && echo "✓" || echo "✗"

# Test 8: Search with days filter
echo -n "Test 8: search --days... "
$AGNTZ search "test" --days 7 &> /dev/null && echo "✓" || echo "✗"

# Test 9: Search including tools
echo -n "Test 9: search --include-tools... "
$AGNTZ search "test" --include-tools &> /dev/null && echo "✓" || echo "✗"

# Test 10: Empty search (should handle gracefully)
echo -n "Test 10: search with no results expected... "
OUTPUT=$($AGNTZ search "xyz_nonexistent_query_$(date +%s)" --json 2>&1)
if echo "$OUTPUT" | grep -q '"hits":\[\]\|No results'; then
    echo "✓"
else
    echo "⊘"
fi

echo ""
echo "=== hstry tests complete ==="
