# Issues

## Open

### [agntz-58dj] Implement agntz find - unified cross-tool search (P1, feature)
Add 'agntz find' command that searches across hstry, mmry, and trx simultaneously. Queries each tool's native interface with appropriate filters, merges results into a common format, and ranks by relevance + recency. Flags: --since (date range), --source mmry,hstry,trx (limit to specific tools), --tag (filter by tag where supported), --workspace (filter by workspace/repo), --limit, --json. Requires: standardized JSON output from all three tools (see trx issues in hstry/mmry/trx repos). Example: 'agntz find authentication --since 2w' searches all three tools for authentication-related content from the last 2 weeks.

### [agntz-r4fx] Add agntz tag - cross-tool tagging (P2, feature)
Add 'agntz tag' command that applies tags across tools. 'agntz tag authentication session-id-xyz' tags a hstry conversation. 'agntz tag authentication memory-id-xyz' tags a mmry memory. 'agntz tag authentication trx-abc' adds a label to a trx issue. Tags become the cross-tool connector (Pal pattern) that enables queries like 'everything tagged authentication'.

### [agntz-yjqg] Add GitHub release workflow with AUR + Homebrew publishing (P2, feature)

### [agntz-pagv] Pass --agent to mmry when running inside an agent session (P3, feature)
agntz memory add shells out to mmry without --agent, so all memories default to 'human (human)'. When running inside a Pi/agent session, agntz should detect the agent environment (e.g. PI_SESSION_ID env var) and pass --agent pi --agent-kind coding_agent to mmry.

