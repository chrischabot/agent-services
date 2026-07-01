---
description: Show arcwell health and operations snapshot
argument-hint: [optional-focus]
---

# Ops

The user invoked this command with: $ARGUMENTS

Use `ops_snapshot` in Codex, equivalent to `arcwell ops` in a shell. Highlight
health warnings, pending/failed/dead-lettered jobs, cursor state, edge events,
projects, digest candidates, memory-review backlog, research runs, and watch
source counts.

When `arcwell service status --compact` reports `cockpit_url`, tell the user
that exact URL. Service-hosted cockpits own their private token file and mint
the browser cookie automatically from `/ops/ui`; do not ask the user to copy
tokens into the browser. `service install --http-addr` also seeds the narrow
local ops-control policy rules needed by the cockpit buttons; provider, cost,
source-write, and promotion gates still apply at execution. Otherwise, when an
Arcwell HTTP server is running, tell the user that the browser cockpit is
visible at `/ops/ui` on that server, with `/cockpit` and `/ops/cockpit` as
aliases. If you start a one-off server yourself, include the exact local URL you used such as
`http://127.0.0.1:8787/ops/ui`.
