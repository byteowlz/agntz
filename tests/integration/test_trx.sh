#!/bin/bash
# Integration tests for trx (issues/tasks) commands

set -e

echo "=== Testing trx integration ==="

# Check if trx is installed
if ! command -v trx &> /dev/null; then
    echo "❌ trx not installed - skipping tests"
    exit 0
fi

# Build agntz
echo "Building agntz..."
cargo build --quiet 2>/dev/null || cargo build

AGNTZ="./target/debug/agntz"

# Setup test repo
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"
git init &> /dev/null
trx init --prefix "agntz_test" &> /dev/null || true

# Test 1: List issues (should be empty or show help)
echo -n "Test 1: tasks list... "
$AGNTZ tasks list &> /dev/null && echo "✓" || echo "✗"

# Test 2: Create a task
echo -n "Test 2: tasks create... "
$AGNTZ tasks create "Test task" -T task -p 2 -d "Test description" &> /dev/null && echo "✓" || echo "✗"

# Test 3: List tasks (should show new one)
echo -n "Test 3: tasks list (after create)... "
OUTPUT=$($AGNTZ tasks list 2>&1)
if echo "$OUTPUT" | grep -q "Test task\|test"; then
    echo "✓"
else
    echo "✗"
fi

# Test 4: Show ready tasks
echo -n "Test 4: ready... "
$AGNTZ ready &> /dev/null && echo "✓" || echo "✗"

# Test 5: Show task details
echo -n "Test 5: tasks show... "
# Get task ID
TASK_ID=$($AGNTZ tasks list --json 2>/dev/null | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
if [ -n "$TASK_ID" ]; then
    $AGNTZ tasks show "$TASK_ID" &> /dev/null && echo "✓" || echo "✗"
else
    echo "⊘ (no task to show)"
fi

# Test 6: Update task
echo -n "Test 6: tasks update... "
if [ -n "$TASK_ID" ]; then
    $AGNTZ tasks update "$TASK_ID" --priority 3 &> /dev/null && echo "✓" || echo "✗"
else
    echo "⊘ (no task to update)"
fi

# Test 7: Create different issue types
echo -n "Test 7: tasks create (bug)... "
$AGNTZ tasks create "Test bug" -T bug -p 1 &> /dev/null && echo "✓" || echo "✗"

# Test 8: List by status
echo -n "Test 8: tasks list --status... "
$AGNTZ tasks list --status "open" &> /dev/null && echo "✓" || echo "✗"

# Test 9: List by type
echo -n "Test 9: tasks list --issue-type... "
$AGNTZ tasks list --issue-type "bug" &> /dev/null && echo "✓" || echo "✗"

# Test 10: Create feature
echo -n "Test 10: tasks create (feature)... "
$AGNTZ tasks create "Test feature" -T feature -p 2 &> /dev/null && echo "✓" || echo "✗"

# Cleanup
cd - &> /dev/null
rm -rf "$TEST_DIR"
echo -n "Cleanup: removed test dir... "
echo "✓"

echo ""
echo "=== trx tests complete ==="
