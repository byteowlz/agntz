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

echo "=== Result: $pass passed, $fail failed ==="
[ "$fail" -eq 0 ] || exit 1