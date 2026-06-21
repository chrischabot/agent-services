# arcwell-ops

**Status:** Partial.

Repository tracking: [STATUS.md](../../STATUS.md) and [TODO.md](../../TODO.md).

Local operations surface.

Current first-pass implementation:

- HTTP `GET /ops` returns an ops snapshot.
- HTTP `GET /ops/ui` renders a read-only localhost browser dashboard over the
  same snapshot.
- MCP `ops_snapshot` returns the same durable state through the agent control plane.
- Snapshot includes health, backups, worker heartbeat, wiki jobs, dead letters,
  edge events, cursors, source health, watch sources, projects, project status
  snapshots, channels, Telegram delivery failures, source cards, digest
  candidates, work runs, procedure candidates, memory candidates, policy
  decisions/approvals, costs, and secret health.
- `/ops/ui` is intentionally read-only. Mutating controls are deferred until
  each action has explicit auth, policy, CSRF/origin, and idempotency tests.

MCP resources:

- `arcwell://ops`
- `arcwell://edge-events`
- `arcwell://projects`
- `arcwell://channels`
- `arcwell://digest-candidates`

Remaining work:

- Filtering, search, detail drawers, and summary health scoring.
- Manual requeue/cancel controls with confirmation policy.
- Error charts and recent failures.
