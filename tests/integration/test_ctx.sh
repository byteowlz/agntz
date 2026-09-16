#!/bin/bash
# Integration tests for `agntz ctx` (the orientation flagship) + the MCP server.
#
# These validate the vision's positive-outcome proofs:
#   1. `agntz ctx` prints a correct snapshot filtered by current AGENT_CTX.
#   2. Same binary tolerates a bare environment (no AGENT_CTX) identically.
#   3. `agntz <any> --json` returns stable machine-parseable JSON on reads.
#   4. The MCP server exposes ONE tool (no five help texts).

set -e

echo "=== Testing ctx + unified surface + MCP ==="

if ! command -v jq &> /dev/null; then
    echo "❌ jq not installed - skipping ctx structural tests"
    exit 0
fi

cargo build --quiet 2>/dev/null || cargo build
AGNTZ="./target/debug/agntz"

PASS=0
FAIL=0
check() {
    local name="$1"; shift
    if "$@" > /tmp/agntz_ctx_out 2>&1; then
        echo "✓ $name"
        PASS=$((PASS+1))
    else
        echo "✗ $name"
        echo "   --- output ---"
        sed 's/^/   /' /tmp/agntz_ctx_out | head -20
        FAIL=$((FAIL+1))
    fi
}

# --- Test 1: ctx text runs and names the agent ---
OUT=$($AGNTZ ctx 2>&1)
if echo "$OUT" | grep -q "operative memory orientation"; then
    echo "✓ Test 1: ctx text renders"; PASS=$((PASS+1))
else
    echo "✗ Test 1: ctx text missing header"; FAIL=$((FAIL+1))
fi

# --- Test 2: ctx --json is stable, machine-parseable, correct schema ---
check "Test 2: ctx --json valid + correct schema" \
    bash -c "$AGNTZ ctx --json | jq -e '.schema == \"agntz.ctx\" and .version == 1 and (.open_problems.snapshot_capture.status == \"open\")'"

# --- Test 3: ctx reads AGENT_CTX producer-map when present ---
if [ -n "${AGENT_CTX_VERSION:-}" ]; then
    ENV_HARNESS=$(printf '%s' "$AGENT_CTX_HARNESS")
    check "Test 3: ctx reflects present AGENT_CTX harness ($ENV_HARNESS)" \
        bash -c "$AGNTZ ctx --json | jq -e --arg h '$ENV_HARNESS' '.agent.bags.harness.HARNESS == \$h'"
else
    echo "⊘ Test 3: no AGENT_CTX in env; skipping producer-map reflection"
fi

# --- Test 4: ctx tolerates a bare environment (no AGENT_CTX) ---
BARE=$(env -i HOME="$HOME" PATH="$PATH" $AGNTZ ctx --json 2>&1)
if echo "$BARE" | jq -e '.schema == "agntz.ctx"' > /dev/null 2>&1; then
    echo "✓ Test 4: ctx tolerates bare env (no AGENT_CTX)"; PASS=$((PASS+1))
else
    echo "✗ Test 4: ctx failed without AGENT_CTX"; FAIL=$((FAIL+1))
fi

# --- Test 5: unified read envelope on tasks list ---
check "Test 5: tasks list --json emits stable read envelope" \
    bash -c "$AGNTZ tasks list --json 2>/dev/null | jq -e '.schema == \"agntz.read\" and .ok == true and .verb == \"tasks/list\"'"

# --- Test 6: MCP server exposes exactly ONE tool ---
MCP_OUT=$(printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"probe","version":"1.0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"agntz_orientation","arguments":{}}}' \
  | $AGNTZ mcp 2>/dev/null)

TOOL_COUNT=$(echo "$MCP_OUT" | jq -s '[.[] | select(.id==2)] | .[0].result.tools | length' 2>/dev/null || echo "0")
TOOL_NAME=$(echo "$MCP_OUT" | jq -s '[.[] | select(.id==2)] | .[0].result.tools[0].name' 2>/dev/null || echo "\"\"")
CALL_TEXT=$(echo "$MCP_OUT" | jq -s '[.[] | select(.id==3)] | .[0].result.content[0].text' 2>/dev/null || echo "{}")

if [ "$TOOL_COUNT" = "1" ] && echo "$CALL_TEXT" | grep -q "agntz.ctx"; then
    echo "✓ Test 6: MCP exposes one tool ($TOOL_NAME) returning orientation"
    PASS=$((PASS+1))
else
    echo "✗ Test 6: MCP tool count/name wrong (count=$TOOL_COUNT, name=$TOOL_NAME)"
    FAIL=$((FAIL+1))
fi

# --- Cleanup ---
rm -f /tmp/agntz_ctx_out

echo ""
echo "=== ctx tests: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
