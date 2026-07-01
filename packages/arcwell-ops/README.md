# arcwell-ops

**Status:** Partial.

Repository tracking: [STATUS.md](../../STATUS.md) and [TODO.md](../../TODO.md).

Local operations surface.

Current implementation:

- HTTP `GET /ops` returns an ops snapshot.
- HTTP `GET /ops/ui` renders the Arcwell Cockpit, a localhost browser dashboard
  over the same snapshot. `GET /cockpit` and `GET /ops/cockpit` are aliases.
- The resident worker service can host the same Rust HTTP router when installed
  with `arcwell service install --http-addr 127.0.0.1:PORT`; `arcwell service
  status --compact` reports `cockpit_url` from the installed LaunchAgent. This
  creates a private service-owned token file, lets `/ops/ui` set an HttpOnly
  same-origin browser cookie automatically, seeds narrow local ops-control
  policy rules, and avoids a second daemon or frontend dependency chain for the
  always-on cockpit and controls surface.
- MCP `ops_snapshot` returns the same durable state through the agent control plane.
- Snapshot includes health, backups, worker heartbeat, wiki jobs, dead letters,
  edge events, cursors, source health, watch sources, projects, project status
  snapshots, channels, Telegram delivery failures, source cards, digest
  candidates, work runs, procedure candidates, memory candidates, policy
  decisions/approvals, costs, and secret health.
- `arcwell provider probe` / MCP `provider_credential_probe` can write
  provider credential health rows into the same source-health surface for
  GitHub, OpenAI, Brave Search, and Cloudflare. This is local/CLI/MCP substrate;
  richer browser summaries and live-provider proof remain tracked in
  `TODO.md`.
- `/ops/ui` includes a cockpit first screen with memory-review, wiki/knowledge,
  task-runner, research/reporting, delivery/channel, history/governance, and
  agent-visibility panels. It also includes search/status filters, stable
  sorting, detail views, a unified event log, summary health scoring,
  queue/source/radar-run/radar-quality/credential summaries, one narrow
  authenticated edge-event dead-letter control, X schedule/enqueue/run-worker
  controls, and Knowledge controls for backlog, model-cluster/model-writer,
  investigation, promotion, and review-only entity-resolution recurrence.
  Mutations use token auth, hostile-origin rejection, CSRF/idempotency checks,
  policy enforcement, and replay tests.
- Codex agents should tell users where this lives whenever a served Arcwell
  browser surface is relevant: canonical path `/ops/ui`, aliases `/cockpit` and
  `/ops/cockpit`, with the exact host/port from `cockpit_url` or the server the
  agent started.
- `scripts/ops-ui-browser-smoke` runs browser-backed desktop, detail, and
  mobile validation against a seeded authenticated local `/ops/ui`, preserving
  screenshots and a proof packet under `.arcwell-dev/proofs/`.
- `scripts/ops-ui-x-browser-smoke` runs browser-backed desktop, filtered, and
  mobile validation for hostile X tweet/link/provider-error rows, including
  token-like provider-error redaction, local dummy-secret non-rendering, row-focused screenshots, and no body overflow.
- Broader mutating controls remain deferred until each action has explicit
  core support, auth, policy, CSRF/origin, idempotency/replay handling, and
  severe tests.

MCP resources:

- `arcwell://ops`
- `arcwell://edge-events`
- `arcwell://projects`
- `arcwell://channels`
- `arcwell://digest-candidates`

Remaining work:

- Manual requeue/cancel controls with confirmation policy.
- More browser fixtures for future mutating controls.
- Richer historical charts and watchdog summaries beyond the current cockpit
  panels and event log.
