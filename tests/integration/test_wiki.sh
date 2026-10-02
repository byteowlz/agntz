#!/bin/bash
# Integration tests for the agntz wiki CLI (agntz-dr9s) against a throwaway
# local bare remote and temp working tree. Never touches the real wiki or the
# network.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."
echo "=== Testing agntz wiki ==="
cargo build --quiet 2>/dev/null || cargo build

AGNTZ="$PWD/target/debug/agntz"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

export XDG_CONFIG_HOME="$WORK/cfg"
export XDG_DATA_HOME="$WORK/data"
export XDG_STATE_HOME="$WORK/state"
export HOME="$WORK/home"; mkdir -p "$HOME"
export GIT_AUTHOR_NAME="Wiki Test"
export GIT_AUTHOR_EMAIL="wiki@test.invalid"
export GIT_COMMITTER_NAME="Wiki Test"
export GIT_COMMITTER_EMAIL="wiki@test.invalid"

git init -q --bare "$WORK/bare.git"

pass=0; fail=0
check() { if eval "$1" >/dev/null 2>&1; then echo "  ✓ $2"; pass=$((pass+1)); else echo "  ✗ $2"; fail=$((fail+1)); fi; }

echo "[1] wiki init creates a valid wiki repo"
$AGNTZ wiki init main "$WORK/wiki" --remote "$WORK/bare.git" >/dev/null
check "test -d '$WORK/wiki/pages' && test -f '$WORK/wiki/index.md'" "init created pages/ + index.md"

echo "[2] wiki status / list"
check "$AGNTZ wiki status" "status"
check "$AGNTZ wiki list" "list"
check "$AGNTZ wiki list --json" "list --json"

echo "[3] wiki create / list / search / read / validate"
printf '# Guide\n\nThis is a guide about `code` and $HOME.\n' > "$WORK/page.md"
$AGNTZ wiki create guides/setup --title "Setup Guide" --body-file "$WORK/page.md" >/dev/null
check "$AGNTZ wiki list | grep -q 'guides/setup'" "list shows new page"
check "grep -Eq '^source-(agent|session|host):.+$' '$WORK/wiki/pages/guides/setup.md'" "published page carries AGENT_CTX provenance"
check "$AGNTZ wiki search guide --json" "search --json"
check "$AGNTZ wiki read guides/setup | grep -q 'Setup Guide'" "read returns page"
check "$AGNTZ wiki validate" "validate (no broken links)"

echo "[4] wiki update requires a matching revision"
printf '# Guide v2\n\nUpdated content with \`code\` and $HOME.\n' > "$WORK/page2.md"
REV="$(git -C "$WORK/wiki" rev-parse --short HEAD)"
$AGNTZ wiki update guides/setup --revision "$REV" --body-file "$WORK/page2.md" >/dev/null
check "git -C '$WORK/wiki' log -1 --pretty=%s | grep -q 'update guides/setup'" "update with correct revision commits"
check "$AGNTZ wiki read guides/setup | grep -q 'Guide v2'" "update changed the page"
# A stale revision must fail (no overwrite).
if $AGNTZ wiki update guides/setup --revision deadbeef --body-file "$WORK/page.md" >/dev/null 2>&1; then
  echo "  ✗ revision-guard should reject stale revision"; fail=$((fail+1))
else
  echo "  ✓ revision-guard rejects stale revision"; pass=$((pass+1))
fi

echo "[5] path traversal rejected"
if $AGNTZ wiki create '../escape' --body-file "$WORK/page.md" >/dev/null 2>&1; then
  echo "  ✗ traversal should be rejected"; fail=$((fail+1))
else
  echo "  ✓ traversal rejected"; pass=$((pass+1))
fi

echo "[6] idempotent re-init; dry-run leaves no changes"
$AGNTZ wiki init main "$WORK/wiki" --remote "$WORK/bare.git" >/dev/null
check "grep -c 'name = \"main\"' '$WORK/cfg/agntz/config.toml' | grep -q '^1$'" "config has one wiki entry"
$AGNTZ wiki init dry "$WORK/newwiki" --dry-run >/dev/null 2>&1
check "test ! -d '$WORK/newwiki'" "dry-run did not create dir"

echo "[7] init --remote pushes the initial commit; clone keeps pages/ layout (P0-3)"
check "git -C '$WORK/bare.git' rev-parse HEAD >/dev/null" "remote non-empty after init"
git clone -q "$WORK/bare.git" "$WORK/wc"
check "test -d '$WORK/wc/pages'" "clone kept pages/ layout (.gitkeep)"
$AGNTZ wiki register peer "$WORK/wc" --remote "$WORK/bare.git" >/dev/null 2>&1

# Publish a page from the main wiki; the clone is now stale.
printf '# Fresh\n\nBody.\n' > "$WORK/fresh.md"
$AGNTZ wiki create guides/fresh --body-file "$WORK/fresh.md" >/dev/null 2>&1

echo "[8] stale clone auto-syncs (fetch+merge) before search (P0-2)"
# A fresh clone is behind; search must see the new page without a manual pull.
if $AGNTZ wiki search --name peer 'Fresh' >/dev/null 2>&1; then
  echo "  ✓ stale clone sees new page in search"; pass=$((pass+1))
else
  echo "  ✗ stale clone did not see new page"; fail=$((fail+1))
fi

check "$AGNTZ wiki list --name peer | grep -q 'guides/fresh'" "stale clone list syncs new page"

echo "[9] validate --json reports ok:true on a clean wiki (P1-5)"
printf '# Bad\n\n[link](does-not-exist)\n' > "$WORK/bad.md"
$AGNTZ wiki create guides/bad --body-file "$WORK/bad.md" >/dev/null 2>&1
BAD_JSON="$(set +e; $AGNTZ wiki validate --json 2>/dev/null || true)"
check "echo '$BAD_JSON' | python3 -c 'import sys,json; d=json.load(sys.stdin); assert d.get(\"ok\") is False'" \
  "validate --json ok:false on broken link"
VAL_ERR="$(set +e; $AGNTZ wiki validate 2>&1 || true)"
check "echo '$VAL_ERR' | grep -qi 'broken link'" "validate text names broken link"

echo "[10] wiki create/update accept inline --body"
$AGNTZ wiki create guides/inline --body 'Inlined body text.' >/dev/null 2>&1
check "grep -q 'Inlined body text' '$WORK/wiki/pages/guides/inline.md'" "wiki create --body inline"
REV="$(git -C "$WORK/wiki" rev-parse --short HEAD)"
$AGNTZ wiki update guides/inline --revision "$REV" --body 'Updated inline.' >/dev/null 2>&1
check "grep -q 'Updated inline' '$WORK/wiki/pages/guides/inline.md'" "wiki update --body inline"

echo "[11] register accepts --role (symmetry with board)"
git clone -q "$WORK/bare.git" "$WORK/wrole"
check "$AGNTZ wiki register rolepeer '$WORK/wrole' --remote '$WORK/bare.git' --role agent" "wiki register --role accepted"

echo "[12] update accepts full SHA; read exposes the page revision"
REVFULL="$($AGNTZ wiki read guides/setup --json 2>/dev/null | python3 -c 'import sys,json; print(json.load(sys.stdin)["result"]["revision"])')"
check "test -n '$REVFULL'" "read --json exposes full page revision"
$AGNTZ wiki update guides/setup --revision "$REVFULL" --body 'Full sha body' >/dev/null 2>&1
check "grep -q 'Full sha body' '$WORK/wiki/pages/guides/setup.md'" "update with full SHA works"

echo "[13] inline --body interprets \\n escapes"
$AGNTZ wiki create guides/newlines --body 'L1\nL2' >/dev/null 2>&1
check "grep -q 'L2' '$WORK/wiki/pages/guides/newlines.md'" "--body interprets \\n as a real newline"

echo "[14] register --remote clones into the given destination path"
$AGNTZ wiki register dest "$WORK/destdir/custom" --remote "$WORK/bare.git" >/dev/null 2>&1
check "test -d '$WORK/destdir/custom'" "register --remote honors the destination path"

echo "[15] validate --json exits non-zero on broken links (single envelope)"
$AGNTZ wiki validate --json > "$WORK/val.json" 2>/dev/null || true
check "test -s '$WORK/val.json'" "validate --json produced output"
check "python3 -c 'import json; d=json.load(open(\"$WORK/val.json\")); assert d.get(\"ok\") is False'" "validate --json ok:false"
# The generic error envelope should NOT be duplicated: exactly one JSON doc.
check "grep -c '\"schema\"' '$WORK/val.json' | grep -qx 1" "validate --json emits a single envelope"
if set +e; $AGNTZ wiki validate --json >/dev/null 2>&1; then echo "  ✗ validate --json should exit non-zero"; fail=$((fail+1)); else echo "  ✓ validate --json exits non-zero (broken)"; pass=$((pass+1)); fi

echo "[16] wiki read tolerates .md and pages/ prefix"
check "$AGNTZ wiki read guides/setup.md | grep -q 'Full sha body'" "read with .md suffix"
check "$AGNTZ wiki read pages/guides/setup | grep -q 'Full sha body'" "read with pages/ prefix"

echo "[17] wiki update stamps updated-by machine/workspace"
UPD="$AGNTZ wiki read guides/setup --json 2>/dev/null | python3 -c 'import sys,json; print(json.load(sys.stdin)[\"result\"][\"revision\"])'"
# Use the short sha (page's own commit) here to also re-exercise acceptance.
UPD="${UPD:0:7}"
$AGNTZ wiki update guides/setup --revision "$UPD" --body 'Provenance body' >/dev/null 2>&1
check "grep -q 'updated-by-machine' '$WORK/wiki/pages/guides/setup.md'" "updated-by-machine stamped"

echo "[18] wiki update accepts the .md and pages/ forms (parity with read)"
UPD2="$($AGNTZ wiki read guides/setup --json 2>/dev/null | python3 -c 'import sys,json; print(json.load(sys.stdin)["result"]["revision"])')"
check "$AGNTZ wiki update guides/setup.md --revision \"$UPD2\" --body 'Md form body'" "update with .md suffix"
UPD3="$($AGNTZ wiki read guides/setup --json 2>/dev/null | python3 -c 'import sys,json; print(json.load(sys.stdin)["result"]["revision"])')"
check "$AGNTZ wiki update pages/guides/setup --revision \"$UPD3\" --body 'Pages form body'" "update with pages/ prefix"

echo "=== Result: $pass passed, $fail failed ==="
[ "$fail" -eq 0 ] || exit 1