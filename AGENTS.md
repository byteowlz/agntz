# agntz - Agent Utility Toolkit

## Overview

agntz is a CLI toolkit for AI coding agents. It provides unified access to:

- **Memory** - Store and retrieve context (wraps mmry)
- **Coordination** - Agent-to-agent messaging and file reservations (wraps mailz)
- **Issues** - Task tracking and triage (wraps bd/bv)
- **Search** - Agent session history search (wraps cass)
- **Tools** - Install and manage agent tools

agntz is designed to be used by AI agents in coding environments like octo, Claude Code, Cursor, etc.

## Commands

```bash
# Memory
agntz memory add "learned something important" -c category
agntz memory search "query"
agntz memory export                    # Export to .memories/export.json
agntz memory export --format md        # Export as markdown
agntz memory import .memories/export.json

# Mail / Coordination
agntz mail inbox
agntz mail send <recipient> "subject" --body "message"
agntz mail ack <id>

# File Reservations
agntz reserve src/file.rs --reason "refactoring"
agntz reservations
agntz release src/file.rs

# Issues
agntz issues                           # List issues (bd list)
agntz ready                            # Show unblocked issues (bd ready)
agntz triage                           # Cross-repo triage (bv)

# History Search
agntz search "how did I fix..."        # Search agent session history

# Tools
agntz tools list
agntz tools install mmry
agntz tools doctor
```

## Export Behavior

`agntz memory export` exports memories from the store linked to the current repo:

- Default output: `.memories/export.json`
- Creates `.memories/` directory if it doesn't exist
- Formats: `json` (default), `md` (markdown)

The `.memories/` directory should be gitignored for private memories or committed for shared context.

## Dependencies

agntz wraps these external tools (install via `agntz tools install`):

| Tool | Purpose |
|------|---------|
| mmry | Memory storage and search |
| mailz | Agent coordination, messaging, file reservations |
| bd | Issue tracking (beads) |
| bv | Issue triage and analytics |
| cass | Agent session history search |

## For AI Agents

When working in a repo, use agntz for:

1. **Session start**: `agntz mail inbox` to check messages, `agntz ready` for tasks
2. **Context**: `agntz memory search "topic"` to find relevant memories
3. **Coordination**: `agntz reserve` before editing shared files
4. **Learning**: `agntz memory add` after discovering something useful
5. **Session end**: `agntz release` files, `agntz memory export` if needed
