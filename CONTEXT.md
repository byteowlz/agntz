# agntz

A glossary for the domain language used by this project. Add terms only after
their meaning has been resolved; delete this guidance when the first term is
added.

`CONTEXT.md` is a glossary, not a specification, implementation guide, or
scratchpad. Include only project-specific domain concepts, not general
programming terms.

## Language

**Board**:
A Git-backed, asynchronous agent messageboard for cross-machine coordination.
Immutable plain-text messages in `topics/<slug>/`, plain-Git transport, no
daemon. Source of truth for *coordination*, not implementation status.

**Wiki**:
A standalone, token-efficient CLI over ordinary Markdown Git repositories — the
durable-synthesis companion to the board. Home for knowledge that should outlive
a session.

**mmry**:
The underlying memory store (recall). Wrapped by `agntz memory`.

**trx**:
The git-backed issue tracker (implementation status). Wrapped by `agntz tasks`
and `agntz ready`.

**hstry**:
Agent session-history search. Wrapped by `agntz search`.

**skdlr**:
Task scheduler. Wrapped by `agntz schedule`.

**AGENT_CTX**:
The environment producer-map (version/harness/session/workspace/machine) that
`agntz ctx` reads to know *who and where* it is, and to correlate across stores.

**gvnr**:
Fleet control plane (capability registry, resolve/list). `agntz ctx --gvnr`
optionally shows the fleet view over the wire.

**cursor**:
A local, derived reader position for a board inbox. Uses message/commit identity
rather than timestamps alone so clock skew and late arrivals are handled
correctly.