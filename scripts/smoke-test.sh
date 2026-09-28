#!/usr/bin/env bash
#
# Smoke test: build the binary and exercise the top-level command surface and
# the new board/wiki subcommands against a throwaway XDG config + temp repos.
# Uses local bare remotes only; never touches the real board/wiki or network.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> Build agntz"
cargo build --quiet

AGNTZ="$ROOT/target/debug/agntz"

echo "==> Top-level help render"
"$AGNTZ" --help >/dev/null
"$AGNTZ" board --help >/dev/null
"$AGNTZ" wiki --help >/dev/null

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

export XDG_CONFIG_HOME="$WORK/cfg"
export XDG_DATA_HOME="$WORK/data"
export XDG_STATE_HOME="$WORK/state"
export HOME="$WORK/home"; mkdir -p "$HOME"
export GIT_AUTHOR_NAME="Smoke Test"; export GIT_AUTHOR_EMAIL="smoke@test.invalid"
export GIT_COMMITTER_NAME="Smoke Test"; export GIT_COMMITTER_EMAIL="smoke@test.invalid"

git init -q --bare "$WORK/bare.git"
"$AGNTZ" board init main "$WORK/board" --remote "$WORK/bare.git" --role agent >/dev/null
"$AGNTZ" wiki init main "$WORK/wiki" --remote "$WORK/bare.git" >/dev/null

echo "==> Board status"
"$AGNTZ" board status >/dev/null
echo "==> Wiki list"
"$AGNTZ" wiki list >/dev/null
echo "==> Config show"
"$AGNTZ" config show >/dev/null

echo "PASS: agntz smoke test (build + board/wiki/config surface)"