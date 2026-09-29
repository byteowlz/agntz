#!/bin/bash
# Integration tests for the agntz board CLI (agntz-kmn7) against a throwaway
# local bare remote and temp working tree. Never touches the real board or the
# network.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."
echo "=== Testing agntz board ==="
cargo build --quiet 2>/dev/null || cargo build

AGNTZ="$PWD/target/debug/agntz"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

export XDG_CONFIG_HOME="$WORK/cfg"
export XDG_DATA_HOME="$WORK/data"
export XDG_STATE_HOME="$WORK/state"
export HOME="$WORK/home"; mkdir -p "$HOME"
export GIT_AUTHOR_NAME="Board Test"
export GIT_AUTHOR_EMAIL="board@test.invalid"
export GIT_COMMITTER_NAME="Board Test"
export GIT_COMMITTER_EMAIL="board@test.invalid"

git init -q --bare "$WORK/bare.git"

pass=0; fail=0
check() { if eval "$1" >/dev/null 2>&1; then echo "  ✓ $2"; pass=$((pass+1)); else echo "  ✗ $2"; fail=$((fail+1)); fi; }

echo "[1] board init creates a working board repo"
$AGNTZ board init main "$WORK/board" --remote "$WORK/bare.git" --role agent >/dev/null
check "test -d '$WORK/board/topics' && test -f '$WORK/board/README.md'" "init created topics/ + README.md"

echo "[2] board status reports the repo"
check "$AGNTZ board status" "status"
check "$AGNTZ board status --json" "status --json"

echo "[3] board register for an existing local repo"
git init -q "$WORK/board2"
mkdir -p "$WORK/board2/topics"
$AGNTZ board register other "$WORK/board2" --role agent >/dev/null
check "$AGNTZ board list" "list shows registered boards"

echo "[4] board topics / read / inbox over a seeded message"
mkdir -p "$WORK/board/topics/hello"
cat > "$WORK/board/topics/hello/20260928T000000Z-11111111-1111-1111-1111-111111111111.txt" <<'EOF'
Message-ID: 11111111-1111-1111-1111-111111111111
Sent-At: 2026-09-28T00:00:00Z
From: other-agent
From-Agent: pi
From-Host: other-host
To: agent
In-Reply-To: none

Subject: Hello

Hello `code` and $NOT_EXPANDED.
EOF
(cd "$WORK/board" && git add -A && git commit -q -m "inbound" && git push -q origin master)
check "$AGNTZ board topics" "topics"
check "$AGNTZ board read 11111111-1111-1111-1111-111111111111" "read"
check "$AGNTZ board inbox --role agent --json" "inbox --role agent --json"

echo "[5] board reply publishes an immutable message (body preserved verbatim)"
printf 'Reply with backticks `x` and $HOME and $(cmd).\n' > "$WORK/body.txt"
REPLY_OUT="$($AGNTZ board reply 11111111-1111-1111-1111-111111111111 --body-file "$WORK/body.txt" --json)"
RID="$(echo "$REPLY_OUT" | python3 -c 'import sys,json; print(json.load(sys.stdin)["result"]["message_id"])')"
check "test -n '$RID'" "reply produced a Message-ID"
RFILE="$(find "$WORK/board/topics/hello" -name "*$RID*.txt" | head -1)"
check "grep -q '\`x\`' '$RFILE'" "reply body kept backticks"
check "grep -q '\$HOME' '$RFILE'" "reply body kept dollar (no shell expansion)"
check "grep -Eq '^From-(Agent|Host|Session-ID):.+$' '$RFILE'" "reply has non-empty provenance headers"

echo "[6] idempotent re-init; dry-run changes nothing"
$AGNTZ board init main "$WORK/board" --remote "$WORK/bare.git" --role agent >/dev/null
check "grep -c 'name = \"main\"' '$WORK/cfg/agntz/config.toml' | grep -q '^1$'" "config has one entry"
CONF_BEFORE="$(wc -c < "$WORK/cfg/agntz/config.toml")"
$AGNTZ board init dry "$WORK/newboard" --dry-run >/dev/null 2>&1
check "test ! -d '$WORK/newboard'" "dry-run did not create dir"
check "test -z \"$(git -C "$WORK/board" status --porcelain)\"" "dry-run leaves board tree clean"

echo "=== Result: $pass passed, $fail failed ==="
[ "$fail" -eq 0 ] || exit 1