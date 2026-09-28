#!/usr/bin/env bash
#
# Drift check: verify that facts this repo documents (commands, the JSON/TOML
# constraint) match the machine-checked config (the real Cargo.toml manifest and
# PATH). Anything checked here is "source of truth" in AGENTS.md — a failure
# means code or docs have drifted.

set -u
FAILED=0

echo "==> Required commands on PATH"
for cmd in cargo just trx python3; do
  if command -v "$cmd" >/dev/null 2>&1; then
    echo "  ok: $cmd -> $(command -v "$cmd")"
  else
    echo "  FAIL: required command not found on PATH: $cmd"
    FAILED=1
  fi
done

echo "==> No YAML application format (JSON/TOML only)"
if grep -q 'serde_yaml' Cargo.toml; then
  echo "  FAIL: serde_yaml present in Cargo.toml"
  FAILED=1
else
  echo "  ok: no serde_yaml dependency"
fi

echo "==> Config crate: default-features=false, json/toml only"
if grep -q '^config = ' Cargo.toml; then
  if grep -Eq 'config\s*=\s*\{[^}]*default-features = false' Cargo.toml; then
    echo "  ok: config crate uses default-features = false"
  else
    echo "  FAIL: config crate missing default-features = false"
    FAILED=1
  fi
  if grep -Eq 'features\s*=\s*\["json".*"toml"\]' Cargo.toml; then
    echo "  ok: config features include json and toml"
  else
    echo "  FAIL: config features do not list json + toml"
    FAILED=1
  fi
else
  echo "  warn: no config crate dependency found (nothing to check)"
fi

echo "==> Workspace/manifest resolves (cargo metadata)"
if cargo metadata --no-deps --format-version 1 >/dev/null 2>&1; then
  echo "  ok: cargo metadata resolves"
else
  echo "  FAIL: cargo metadata --no-deps does not resolve"
  FAILED=1
fi

echo "==> Generated config examples are up to date"
if cargo run --quiet --example generate_config >/dev/null 2>&1; then
  if ! git diff --quiet -- examples/config.toml examples/config.schema.json; then
    echo "  FAIL: examples/config differ from structs (run 'just generate-config')"
    FAILED=1
  else
    echo "  ok: examples/config match the config structs"
  fi
else
  echo "  FAIL: could not run generate_config example"
  FAILED=1
fi

echo
if [[ "$FAILED" -eq 0 ]]; then
  echo "drift-check: PASS"
else
  echo "drift-check: FAIL"
  exit 1
fi