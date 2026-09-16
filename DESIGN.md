# agntz — Design

Agent utility toolkit. **One front door to an agent's operative memory — that also
sees the fleet.** A standalone, cross-platform CLI for AI coding agents.

## Why this exists (and why the old shape failed)

agntz began as a thin delegator (memory→mmry, tasks→trx, search→hstry,
schedule→skdlr). Pure delegation adds no value, so agents correctly bypass it
and call the underlying CLIs directly. **The addition has to be real.**

The one thing no underlying CLI provides is the *joined* picture. Each tool is
powerful in its own domain, but the information an agent needs crosses domains.
agntz is the loading layer over them.

## Role in the system

Three persona-aligned tools; agntz is the agent's own window:

| persona | tool | role |
|---|---|---|
| Governor / main agent | **gvnr** | fleet control plane: capability registry, resolve/list, events, transparency |
| normal agents | **agntz** | operative memory: orient + unified agent surface |
| machines / workloads | **dpty** | per-machine compute unit: recipes → ledger → execute Instances |

Boundaries: gvnr is fleet-only (resolve-and-list, never queues/schedules/owns
agents). dpty executes. agntz is the agent's self — its knowledge, its work, its
context. agntz **does not** do fleet coordination, and gvnr does not grow
per-agent memory/tasks/search/schedule.

## Vision: three pillars

### 1. `agntz ctx` — orient (the flagship)

One snapshot answering *"what is relevant to me here, right now"*: top memories
for this repo, open/in-flight tasks affecting me, recent session history,
upcoming schedule, and (optionally `--gvnr`) what's running on the fleet for me.
This is the cold-start / re-entry / handoff / resume moment. Nothing else in the
system does it; it is the reason an agent reaches for agntz first thing.

`ctx` reads **AGENT_CTX** (VERSION/HARNESS/HARNESS_SESSION_ID/RUN_MODE/
WORKSPACE_ID/MACHINE_ID/OS_ARCH + producer map) to know *who and where* it is,
then filters every store to that context.

### 2. Unified agent surface

Agents pay a fixed tool-budget. Five CLIs → five help texts and five tool defs
to load. agntz collapses this to **one stable tool the agent loads**, with the
wrappers underneath as backends:
- consistent command set + flags across all domains;
- `--json` on every read (machine-parseable, token-efficient, ordered);
- **one MCP server / one tool spec** exposed to agents;
- works identically in any harness (bare pi / oqto-only / herdr / full) because
  it reads AGENT_CTX, not harness internals.

### 3. Explicit non-features

- **No inter-agent live chat (`agntz ask`).** Redundant with herdr, which owns
  agent-to-agent live communication. Rejected — see trx.
- **Correlation is not a feature.** Relating a task id across memory/session/
  deployment is inherited from AGENT_CTX: every tool stamps its lineage
  (session/workspace/machine id), so a read filtered by current AGENT_CTX *is*
  the correlation. No bespoke graph in agntz.
- **snapshot capture is an open design problem** (see below), not a shipped
  feature yet.

## Oqto runner — scopes + sandboxes (the future that earns its keep)

The lasting value of agntz comes from connecting it to the **oqto runner** and
leveraging Oqto's scopes and sandboxes — not from replicating herdr:

- **Workspace scoping.** `ctx` and every read/write bind to the Oqto
  **Workspace scope** the running agent is established in, not ad-hoc repo
  detection. In shared, multi-agent Workspaces this is the only correct
  identity; AGENT_CTX carries `WORKSPACE_ID`. Oqto is the multiplayer surface
  for shared Workspaces / Work dirs, so this is where "whose memory, which
  work" is actually decided.
- **Sandbox awareness.** Inside an Oqto sandbox, agntz honors granted
  capabilities: reads restricted to scope, writes gated, and `tools doctor`
  reports what the sandbox *allows* — exposing the bounding honestly instead of
  pretending it is unrestricted.

This is tracked as a follow-up (oqto runner integration); the core `ctx`
orientation layer is built against AGENT_CTX today so the oqto hookup slots in
without rework.

## Open questions

### Snapshotting

*What is a good snapshot of an agent's operative memory?* Unresolved. Candidates
feed into `ctx` and capture: time-stamped, context-linked, compressible, diff-able.
Do **not** ship a half-baked snapshot; design it explicitly when the capture
taxonomy is settled. Tracked separately.

## Relationship to existing pending work

Previously-filed trx fold into this vision:
- `agntz find` (unified cross-tool search) → pillar 2 / ctx.
- `agntz tag` (cross-tool tagging) → ctx recall + lineage.
- `--agent to mmry` (context-aware capture) → AGENT_CTX lineage stamping.

## Out of scope

- Fleet coordination / scheduling / queues (gvnr).
- Workload execution (dpty).
- Replacing mmry, trx, hstry, skdlr — they remain the underlying stores.
- Base-provisioning of machines (Ansible).
