# agntz

Agent utility toolkit for AI coding agents.

A standalone CLI providing common agent operations like memory management, messaging, file reservations, and issue tracking. Designed to be used by AI agents across any project, not tied to any specific ecosystem.

## Installation

```bash
just install #if you have the just commandrunner installed
```

or via cargo

```bash
cargo install --path .
```

## Commands

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

### Mail (wraps mailz)

```bash
agntz mail inbox                    # Check inbox
agntz mail send <to> "subject"      # Send message
agntz mail read <id>                # Read message
agntz mail ack <id>                 # Acknowledge message
agntz mail search "query"           # Search messages
```

### File Reservations

```bash
agntz reserve src/main.rs --reason "refactoring"   # Reserve file
agntz reservations                                  # List active reservations
agntz release src/main.rs                          # Release reservation
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
agntz tools install <tool>          # Install a tool (mmry, trx, hstry, mailz)
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
