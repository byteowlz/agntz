# AGENTS.md

Guidance for coding agents working on agntz. Kept short: it carries enforceable
boundaries and workflows and points at machine-checked configuration instead of
duplicating any inventory.

## Source of truth

Static facts are machine-checked, not copied here:

- Crate/dependency/lint settings → `Cargo.toml`; authoritative enumeration is
  `cargo metadata --no-deps --format-version 1`.
- Task commands → `just` (run `just` to list).
- Issues → `trx` (see below).
- Drift guard → `scripts/drift-check.sh` (verifies required commands, the
  JSON/TOML-only constraint, the `config` crate feature set, that `cargo
  metadata` resolves, and that `examples/config.toml` + `config.schema.json`
  match the config structs). Run it after touching the manifest, config structs,
  or any doc claim.

If `cargo metadata`, `just`, or `scripts/drift-check.sh` disagree with anything
here, the command is right and this file is wrong.

## Domain and architecture

- Read `CONTEXT.md` before domain work; keep it a glossary only.
- Read `docs/adr/` before architectural changes; add an ADR only for
  hard-to-reverse decisions with a real trade-off.
- agntz is a loading layer: memory→mmry, tasks→trx, search→hstry,
  schedule→skdlr, plus the board and wiki (self-contained Git-backed modules)
  and `ctx` (orientation). Keep backend delegation thin.
- Board is coordination, wiki is durable synthesis, trx is implementation
  status, mmry is recall. Do not blur those boundaries.
- Never bundle unrelated existing working-tree changes into a feature commit.

## Strict lints

`[lints.rust]` and `[lints.clippy]` are strict: `unsafe_code = "deny"`,
`panic`/`dbg_macro`/`todo`/`unimplemented`/`exit`/`mem_forget` = deny, and the
`all`/`cargo` clippy groups at deny. `unwrap_used`/`expect_used` and the
`pedantic`/`nursery` style groups are relaxed (see the comments in `Cargo.toml`)
so the large pre-existing modules and tests build without ad-hoc rewrites.
Propagate errors with `?`, `anyhow::Result`, `.context("...")`. Commands must
be safe against reuse: never stash/reset/discard another session's work, never
force-push, never overwrite an existing message/page implicitly.

## Workflow

- CLI: subcommands for verbs. Global flags `-q`, `-v`, `--debug`, `--trace`,
  `--json`, `--no-color`, `--dry-run`, `--yes`, `--no-input`, `--timeout`,
  `--no-progress`, `--diagnostics`.
- Config structs: after editing `src/config.rs`, run `just generate-config` and
  `just test` (the lib tests enforce the examples match).
- Before anything significant: `just check-all`.

## Application formats: JSON and TOML only

- Machine output and config are **JSON or TOML only** — `--json` plus TOML
  config files. Never add a YAML output mode, a YAML example, or a YAML crate.
- The `config` crate runs with `default-features = false` and only
  `json`/`toml` features. `scripts/drift-check.sh` enforces the absence of
  `serde_yaml`.

## Configuration & storage

- XDG paths with sensible fallbacks; expand `~` and env vars; a commented
  example lives under `examples/`; a default config is written on first run.
  Named board/wiki repos are selected via `flag > env (AGNTZ_BOARD/AGNTZ_WIKI)
  > config default > first`.

## Issue tracking (trx)

Use `trx` for all issue tracking — never markdown TODOs or `.beads`.

```bash
trx ready --json                                   # find unblocked work
trx create "Title" -t task -p 2 --json             # create (bug/feature/task/epic/chore)
trx update <id> --status in_progress --json        # claim
trx close <id> -r "reason" --json                  # complete with reason
```

Priorities: 0=critical, 1=high, 2=medium (default), 3=low, 4=backlog.
Issue state lives in `.trx/` (JSONL) — commit it with code changes.

## House rules

- Do exactly what the user asks — no unsolicited files.
- Keep README updates concise and emoji-free.
- Never commit secrets or sensitive paths; scrub logs.
- `Cargo.lock` is committed; bump manifest + lock together.
- Reference canonical agent skills in `~/byteowlz/skillissues` rather than
  creating unmanaged skill copies.