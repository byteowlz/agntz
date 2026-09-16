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
agntz schedule remove backup -y                                  # Delete
```

## License

MIT
