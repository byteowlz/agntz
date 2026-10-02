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
check "$AGNTZ board repos" "list shows registered boards"

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

echo "[7] init --remote pushes the initial commit; status is honest"

check "git -C '$WORK/bare.git' rev-parse HEAD >/dev/null" "remote non-empty after init (P0-3)"
check "$AGNTZ board status | grep -qE 'in sync|locally committed'" "status state honest (P0-3)"

# A second clone of the same remote, kept deliberately stale.
git clone -q "$WORK/bare.git" "$WORK/peerc"
$AGNTZ board register peer "$WORK/peerc" --remote "$WORK/bare.git" --role agent >/dev/null
# Reliability of the layout clone: topics/ must be present (tracked .gitkeep).
check "test -d '$WORK/peerc/topics'" "clone kept topics/ layout (P0-3 gitkeep)"

# Publish a new message from the main board only; peer clone is now behind.
mkdir -p "$WORK/board/topics/hello"
cat > "$WORK/board/topics/hello/20260929T000000Z-22222222-2222-2222-2222-222222222222.txt" <<'EOF'
Message-ID: 22222222-2222-2222-2222-222222222222
Sent-At: 2026-09-29T00:00:00Z
From: other-agent
From-Agent: pi
To: agent
In-Reply-To: none

Subject: Late

Second.
EOF
(cd "$WORK/board" && git add -A && git commit -q -m "inbound2" && git push -q origin master)

echo "[8] stale clone auto-syncs (fetch+merge) before a read (P0-1)"
check "$AGNTZ board inbox --role agent --name peer | grep -q 22222222-2222-2222-2222-222222222222" \
  "stale clone sees pushed message without manual pull"

FIRST_REV="$(git -C "$WORK/board" rev-parse --short master~1)"
echo "[9] inbox --since <commit> narrows to messages after it (P1-4)"
check "$AGNTZ board inbox --role agent --since "$FIRST_REV" | grep -q 22222222-2222-2222-2222-222222222222" \
  "--since commit shows the later message"
check "! $AGNTZ board inbox --role agent --since "$FIRST_REV" | grep -q 11111111-1111-1111-1111-111111111111" \
  "--since commit hides the earlier message"
check "! $AGNTZ board inbox --role agent --since notaref >/dev/null 2>&1" "invalid --since is rejected"

echo "[10] --json failure emits the error envelope (P1-2)"
# agntz exits non-zero on the error, so capture its JSON stdout and assert.
ERR_JSON="$(set +e; $AGNTZ board inbox --name nope --json 2>/dev/null || true)"
check "echo '$ERR_JSON' | python3 -c 'import sys,json; d=json.load(sys.stdin); assert d.get(\"ok\") is False and d.get(\"verb\")==\"error\"'" \
  "--json error envelope (ok:false)"

echo "[11] read --json schema matches inbox (lowercase keys, nested headers) (P1-3)"
check "$AGNTZ board read 22222222-2222-2222-2222-222222222222 --json | python3 -c 'import sys,json; d=json.load(sys.stdin)[\"result\"]; assert \"message_id\" in d and \"headers\" in d'" \
  "read --json normalized keys"

echo "[12] reply --new-topic opens a cross-linked new topic"
"$AGNTZ" board reply 11111111-1111-1111-1111-111111111111 --body-file "$WORK/body.txt" --new-topic spin >/dev/null
check "test -n \"$(ls "$WORK/board/topics/spin" 2>/dev/null)\"" "--new-topic created the topic dir"
SPIN="$(find "$WORK/board/topics/spin" -name '*.txt' | head -1)"
check "grep -q 'In-Reply-To: 11111111-1111-1111-1111-111111111111' '$SPIN'" "--new-topic reply cross-links parent via In-Reply-To"

# A reply into the parent topic, then an idempotent retry.
R1="$("$AGNTZ" board reply 11111111-1111-1111-1111-111111111111 --body-file "$WORK/body.txt" --idempotency-key op-1 --json 2>/dev/null | python3 -c 'import sys,json;print(json.load(sys.stdin)["result"]["message_id"])')"
R2="$("$AGNTZ" board reply 11111111-1111-1111-1111-111111111111 --body-file "$WORK/body.txt" --idempotency-key op-1 --json 2>/dev/null | python3 -c 'import sys,json;print(json.load(sys.stdin)["result"]["message_id"])')"
echo "[13] reply --idempotency-key reuses Message-ID (no duplicate)"
check "test -n '$R1' && test '$R1' = '$R2'" "idempotent retry reuses same Message-ID"
TOTAL="$(find "$WORK/board/topics" -name '*.txt' | wc -l)"
UNIQ="$(find "$WORK/board/topics" -name '*.txt' -exec grep -h '^Message-ID:' {} \; | sort -u | wc -l)"
check "test '$UNIQ' -eq '$TOTAL'" "no duplicate Message-IDs across the board"

echo "[14] failed fetch is surfaced, not disguised as an empty inbox"
"$AGNTZ" board init stuck "$WORK/stuck" --remote "http://127.0.0.1:1/none" --role agent >/dev/null 2>&1
SYNC_JSON="$(set +e; "$AGNTZ" board inbox --role agent --name stuck --json 2>/dev/null || true)"
check "echo '$SYNC_JSON' | python3 -c 'import sys,json; d=json.load(sys.stdin); assert d[\"result\"][\"sync_warning\"]'" "failed fetch surfaces sync_warning"

echo "[15] two concurrent writers' posts both survive"
git clone -q "$WORK/bare.git" "$WORK/wc2"
$AGNTZ board register writer "$WORK/wc2" --remote "$WORK/bare.git" --role agent >/dev/null 2>&1
printf 'Writer A.\n' > "$WORK/a.txt"
printf 'Writer B.\n' > "$WORK/b.txt"
$AGNTZ board reply 11111111-1111-1111-1111-111111111111 --body-file "$WORK/a.txt" --role agent >/dev/null 2>&1
$AGNTZ board reply 11111111-1111-1111-1111-111111111111 --name writer --body-file "$WORK/b.txt" --role agent >/dev/null 2>&1
# Sync main, then confirm both writers' messages are in the shared topic.
$AGNTZ board inbox --role agent >/dev/null 2>&1
check "grep -rl 'Writer A' '$WORK/board/topics/hello' >/dev/null" "writer A's post survived"
check "grep -rl 'Writer B' '$WORK/board/topics/hello' >/dev/null" "writer B's post survived"

echo "[16] replying to a stale role warns"
mkdir -p "$WORK/board/topics/ghost"
cat > "$WORK/board/topics/ghost/20260820T000000Z-99999999-1111-1111-1111-111111111111.txt" <<'EOF'
Message-ID: 99999999-1111-1111-1111-111111111111
Sent-At: 2026-08-20T00:00:00Z
From: ghost
From-Agent: pi
To: agent
In-Reply-To: none

Subject: Old

Old.
EOF
(cd "$WORK/board" && git add -A && git commit -q -m ghost && git push -q origin master)
STALE_OUT="$(set +e; $AGNTZ board reply 99999999-1111-1111-1111-111111111111 --body-file "$WORK/a.txt" 2>&1 || true)"
check "echo '$STALE_OUT' | grep -qi 'stale'" "stale-role warning emitted"

# A fresh local-only board so its read cursor starts clean.
$AGNTZ board init cur "$WORK/curb" --role agent >/dev/null
msgfile() { # $1=id $2=subject $3=body; writes a message file into topics/t
  printf 'Message-ID: %s\nSent-At: 2026-09-28T00:00:00Z\nFrom: other-agent\nFrom-Agent: pi\nTo: agent\nIn-Reply-To: none\n\nSubject: %s\n\n%s.\n' "$1" "$2" "$3" > "$WORK/curb/topics/t/20260928T000000Z-$1.txt"
}
mkdir -p "$WORK/curb/topics/t"
msgfile aaaaaaaa-1111-1111-1111-111111111111 One One
(cd "$WORK/curb" && git add -A && git commit -q -m one)

echo "[17] default inbox advances the read cursor (incremental)"
check "$AGNTZ board inbox --role agent --name cur | grep -q 'One'" "first read shows the message"
check "! $AGNTZ board inbox --role agent --name cur | grep -q 'One'" "second read is empty (cursor advanced)"
msgfile bbbbbbbb-2222-2222-2222-222222222222 Two Two
(cd "$WORK/curb" && git add -A && git commit -q -m two)
check "$AGNTZ board inbox --role agent --name cur | grep -q 'Two'" "new message surfaces incrementally"

# Bounded output must not be marked read (no skip, re-readable).
msgfile cccccccc-3333-3333-3333-333333333333 Three Three
msgfile dddddddd-4444-4444-4444-444444444444 Four Four
(cd "$WORK/curb" && git add -A && git commit -q -m threefour)
echo "[18] bounded output is NOT marked read (never skips)"
check "$AGNTZ board inbox --role agent --name cur --limit 1 | grep -q 'Three'" "bounded read shows oldest new message"
check "$AGNTZ board inbox --role agent --name cur --limit 1 | grep -q 'Three'" "same bounded read still shows it (not marked read)"

# Explicit ack: local receipt advances the cursor; --publish posts a receipt.
echo "[19] explicit ack advances cursor; --publish posts a receipt-only message"
$AGNTZ board ack cccccccc-3333-3333-3333-333333333333 --name cur >/dev/null
check "$AGNTZ board inbox --role agent --name cur | grep -q 'Four'" "after ack, only later messages remain"
$AGNTZ board ack dddddddd-4444-4444-4444-444444444444 --name cur --publish >/dev/null
ACKFILE="$(grep -rl 'Ack: read' "$WORK/curb/topics/t" | head -1)"
check "test -n '$ACKFILE'" "--publish created an ack message"
check "grep -q 'does not imply agreement' '$ACKFILE'" "ack text is receipt-only (no agreement)"

# Inline --body and title defaulting on a fresh local board.
echo "[20] inline --body and title defaulting"
$AGNTZ board init t2 "$WORK/t2b" --role agent >/dev/null
mkdir -p "$WORK/t2b/topics/t"
printf 'Message-ID: eeeeeeee-5555-5555-5555-555555555555\nSent-At: 2026-09-28T00:00:00Z\nFrom: other-agent\nFrom-Agent: pi\nTo: agent\nIn-Reply-To: none\n\nTitle from this body line.\n' > "$WORK/t2b/topics/t/20260928T000000Z-eeeeeeee-5555-5555-5555-555555555555.txt"
(cd "$WORK/t2b" && git add -A && git commit -q -m m)
check "$AGNTZ board inbox --role agent --name t2 --json 2>/dev/null | python3 -c 'import sys,json; d=json.load(sys.stdin)[\"result\"][\"messages\"][0]; assert d[\"title\"]==\"Title from this body line.\" and d[\"subject\"] is None'" "title defaults to body line in --json"
$AGNTZ board reply eeeeeeee-5555-5555-5555-555555555555 --name t2 --body 'Inline reply.' >/dev/null 2>&1
check "$AGNTZ board inbox --role all --name t2 --json 2>/dev/null | python3 -c 'import sys,json; ms=json.load(sys.stdin)[\"result\"][\"messages\"]; assert any(\"Inline reply\" in m[\"title\"] for m in ms)'" "reply --body inline (no file)"

echo "[21] configured repo paths are stored absolute (cwd-independent)"
check "grep -Eq '^path = \"/' '$WORK/cfg/agntz/config.toml'" "config stores absolute repo paths"

# register --remote clones into the given destination, not the URL basename.
git init -q "$WORK/gitrepo" && (cd "$WORK/gitrepo" && git -c user.name=t -c user.email=t@t.invalid commit -q --allow-empty -m seed)
echo "[22] register --remote honors the destination path"
check "$AGNTZ board register dest '$WORK/destdir/custom' --remote '$WORK/gitrepo' --role agent" "board register --remote into given destination"
check "test -d '$WORK/destdir/custom'" "destination directory created"

echo "[23] register without --remote inherits git origin (stale clone syncs)"
git clone -q "$WORK/bare.git" "$WORK/peer2"
$AGNTZ board register peer2 "$WORK/peer2" --role agent >/dev/null 2>&1
check "grep -A3 'name = \"peer2\"' '$WORK/cfg/agntz/config.toml' | grep -q 'remote ='" "register inherited git origin into config"
printf 'Message-ID: abcdabcd-7777-7777-7777-777777777777\nSent-At: 2026-09-29T00:00:00Z\nFrom: other\nFrom-Agent: pi\nTo: agent\nIn-Reply-To: none\n\nSubject: Fresh\n\nFresh.\n' > "$WORK/board/topics/hello/20260929T000000Z-abcdabcd-7777-7777-7777-777777777777.txt"
(cd "$WORK/board" && git add -A && git commit -q -m fresh && git push -q origin master)
check "$AGNTZ board inbox --role agent --name peer2 2>/dev/null | grep -q abcdabcd" "stale peer2 auto-syncs via inherited remote"

echo "[24] topic timestamps with an offset render as UTC"
mkdir -p "$WORK/board/topics/off"
printf 'Message-ID: efefefef-8888-8888-8888-888888888888\nSent-At: 2026-10-02T12:00:00+02:00\nFrom: other\nFrom-Agent: pi\nTo: agent\nIn-Reply-To: none\n\nSubject: Off\n\nOff.\n' > "$WORK/board/topics/off/20261002T100000Z-efefefef-8888-8888-8888-888888888888.txt"
(cd "$WORK/board" && git add -A && git commit -q -m off && git push -q origin master)
check "$AGNTZ board topics 2>/dev/null | grep -q '2026-10-02T10:00:00Z'" "+02:00 offset renders as UTC in topics"

# A message whose Message-ID header carries RFC5322 angle brackets must be
# addressable by BOTH the bracketed and the bare id (read/reply/ack/--since).
echo "[25] Message-ID lookup is bracket-insensitive"
mkdir -p "$WORK/board/topics/br"
printf 'Message-ID: <cccccccc-9999-9999-9999-999999999999>\nSent-At: 2026-10-03T00:00:00Z\nFrom: other\nFrom-Agent: pi\nTo: agent\nIn-Reply-To: none\n\nSubject: Br\n\nBr.\n' > "$WORK/board/topics/br/20261003T000000Z-br.txt"
(cd "$WORK/board" && git add -A && git commit -q -m br && git push -q origin master)
check "$AGNTZ board read '<cccccccc-9999-9999-9999-999999999999>'" "read with bracketed id"
check "$AGNTZ board read 'cccccccc-9999-9999-9999-999999999999'" "read with bare id"
check "$AGNTZ board reply 'cccccccc-9999-9999-9999-999999999999' --body r >/dev/null 2>&1" "reply by bare id of a bracketed message"

echo "[26] --since all bypasses the read cursor (full history)"
$AGNTZ board inbox --role agent >/dev/null 2>&1
check "$AGNTZ board inbox --role agent --since all 2>/dev/null | grep -q 'cccccccc-9999'" "--since all shows already-read messages"
# And the no-arg read states that the cursor consumed the board.
check "$AGNTZ board inbox --role agent 2>/dev/null | grep -q 'use --since all'" "empty inbox notes the cursor + escape hatch"

echo "=== Result: $pass passed, $fail failed ==="
[ "$fail" -eq 0 ] || exit 1
