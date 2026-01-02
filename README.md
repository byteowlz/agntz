# agntz

Agent utility toolkit for AI coding agents.

A standalone CLI providing common agent operations like memory management, messaging, file reservations, and issue tracking. Designed to be used by AI agents across any project, not tied to any specific ecosystem.

## Installation

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

### Issues (wraps bd/beads)

```bash
agntz issues                        # List all issues
agntz issues list                   # List all issues
agntz issues create "title" -t bug -p 1
agntz issues update <id> --status in_progress
agntz issues close <id> -r "reason"
agntz issues show <id>
agntz ready                         # Show unblocked issues
```

### Triage (wraps bv)

```bash
agntz triage                        # Cross-repo issue triage
agntz triage --next                 # Show next recommended item
agntz triage --refresh              # Regenerate workspace.yaml first
```

### Search (wraps cass)

```bash
agntz search "query"                # Search agent session history
agntz search "query" --days 7       # Limit to last 7 days
```

### Tools

```bash
agntz tools list                    # List available/installed tools
agntz tools install <tool>          # Install a tool (mmry, bd, bv, cass, mailz)
agntz tools update <tool>           # Update a tool
agntz tools doctor                  # Check tool health
```

## Dependencies

agntz wraps several external tools. Install them as needed:

- **[mmry](https://github.com/byteowlz/mmry)** - Memory system for humans and AI agents
- **[beads](https://github.com/steveyegge/beads)** (bd) - Distributed git-backed issue tracker
- **[beads_viewer](https://github.com/Dicklesworthstone/beads_viewer)** (bv) - TUI for beads with graph analytics
- **[cass](https://github.com/Dicklesworthstone/coding_agent_session_search)** - Agent session history search
- **[mailz](https://github.com/byteowlz/mailz)** - Agent coordination messaging

Install all with:

```bash
agntz tools install all
```

## License

MIT
