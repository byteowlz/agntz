# agntz

Agent utility toolkit for AI coding agents.

A standalone CLI providing common agent operations like memory management, issue tracking, and search. Designed to be used by AI agents across any project, not tied to any specific ecosystem.

## Installation

### Package Managers

#### Arch Linux (AUR)
```bash
paru -S agntz  # or yay/pacman
```

#### macOS/Linux (Homebrew)
```bash
brew install byteowlz/tap/agntz
```

#### Windows (Scoop)
```bash
scoop bucket add byteowlz https://github.com/byteowlz/scoop-bucket
scoop install agntz
```

### From Source

```bash
just install  # if you have just installed
```

or via cargo

```bash
cargo install --path .
```

## Commands

All subcommands share global flags: `-q/--quiet`, `-v/--verbose`, `--debug`,
`--trace`, `--json`, `--no-color`, `--color`, `--dry-run`, `--yes`,
`--no-input`, `--timeout`, `--no-progress`, `--diagnostics`, `--config <path>`.

### Orientation (`ctx`)

`agntz ctx` is the operative-memory snapshot — who/where you are and what's
relevant right now. It reads the `AGENT_CTX` v2 producer-map (multiplexer,
harness, workspace, host), surfaces operative memory (open tasks, memories,
recent history), and optionally the gvnr fleet view over the wire.

```bash
agntz ctx                       # Snapshot (text)
agntz ctx --json                # Same, machine-parseable
agntz ctx --gvnr                # Include optional fleet view (degrades if gvnr absent)
```

### MCP server (unified agent surface)

`agntz mcp` runs a single-tool MCP server over stdio. The ONE tool,
`agntz_orientation`, returns the same snapshot as `agntz ctx` — a unified
agent surface instead of five help texts.

```bash
agntz mcp                       # JSON-RPC server on stdin/stdout
```

### Unified `--json` surface

Every *read* command accepts a global `--json` flag and emits a stable
envelope:

```json
{ "schema": "agntz.read", "version": 1, "verb": "tasks/list",
  "ok": true, "error": null, "result": [...] }
```

`agntz ctx --json` emits `{ "schema": "agntz.ctx", ... }`. Writes keep
their human output unless `--json` is given.

```bash
agntz tasks list --json
agntz ready --json
agntz schedule list --json
agntz search "query" --json
```

### Memory (wraps mmry)

```bash
agntz memory add "insight" -c category -i 7    # Add a memory
agntz memory search "query"                     # Search memories
agntz memory export                             # Export to .memories/export.json
agntz memory export --format md                 # Export as markdown
agntz memory import memories.json               # Import from file
agntz memory stats                              # Show statistics
agntz memory stores                             # List available stores
```

### Tasks (wraps trx)

```bash
agntz tasks                         # List all tasks
agntz tasks list                    # List all tasks
agntz tasks create "title" -t bug -p 1
agntz tasks update <id> --status in_progress
agntz tasks close <id> -r "reason"
agntz tasks show <id>
agntz ready                         # Show unblocked tasks
```

### Search (wraps hstry)

Defaults to the current repo/dir unless `--all-workspaces` is set.

```bash
agntz search "query"                # Search agent session history
agntz search "query" --days 7       # Limit to last 7 days
agntz search "query" --session <id> # Search within a session
agntz search "query" --all-workspaces
```

### Tools

```bash
agntz tools list                    # List available/installed tools
agntz tools install <tool>          # Install a tool (mmry, trx, hstry)
agntz tools update <tool>           # Update a tool
agntz tools doctor                  # Check tool health
```

### Schedule (wraps skdlr)

```bash
agntz schedule add backup -s "0 2 * * *" -c "restic backup ~"   # Add task
agntz schedule list                                              # List all
agntz schedule show backup                                       # Show details
agntz schedule edit backup -s "0 3 * * *"                       # Edit schedule
agntz schedule enable backup                                     # Enable
agntz schedule disable backup                                    # Disable
agntz schedule run backup                                        # Trigger now
agntz schedule logs backup                                       # View history
agntz schedule status                                            # Overview
agntz schedule next                                              # Upcoming runs
agntz schedule remove backup --force                            # Delete (or global -y/--yes)
```

### Board (Git-backed agent messageboard)

`agntz board` is a compact, robust CLI over an existing byteowlz-style
messageboard repo: immutable plain-text messages in `topics/<slug>/`, plain-Git
transport, no daemon. This is *coordination*, not a task queue or approval
authority.

```bash
agntz board init main ~/boards/main --remote <url> --role agent  # Bootstrap a new board
agntz board register other ~/boards/other --remote <url>         # Register an existing repo
agntz board list                                                  # Registered boards
agntz board topics                                                # Topic metadata/ordering
agntz board inbox --role agent                                   # New relevant messages
agntz board read <message-id>                                    # Full body + headers
agntz board reply <message-id> --body-file reply.md              # Publish an immutable reply
agntz board status                                                # Local vs remote state
```

### Wiki (Git-backed Markdown knowledge)

`agntz wiki` is a standalone, token-efficient CLI over ordinary Markdown Git
repositories — the durable-synthesis companion to the board. No server,
embeddings, or model inference.

```bash
agntz wiki init main ~/wiki --remote <url>                       # Bootstrap a new wiki
agntz wiki register other ~/wiki-other --remote <url>            # Register an existing repo
agntz wiki list                                                   # Pages with metadata
agntz wiki search "topic"                                       # Bounded search
agntz wiki read guides/setup                                     # Read a page
agntz wiki create guides/setup --title "Setup" --body-file s.md # Create a page
agntz wiki update guides/setup --revision <sha> --body-file s.md # Update (revision-guarded)
agntz wiki validate                                               # Check links/index
agntz wiki status                                                 # Local/committed/published
```

### Configuration (board/wiki repo registry)

Named board and wiki Git repos are registered in
`$XDG_CONFIG_HOME/agntz/config.toml`. Selection precedence is
**flag `--name` > env `AGNTZ_BOARD`/`AGNTZ_WIKI` > config default > first**;
the effective repo and source are shown by `agntz board config` / `agntz wiki
config`.

```bash
agntz config show        # Effective config
agntz config path        # Config file path
agntz board config       # Effective board repo + source
agntz wiki config        # Effective wiki repo + source
```

## License

MIT
