# Arcwell Missing-Pieces Plan

Last updated: 2026-06-20

This plan turns the 2026 agent-stack inventory into concrete Arcwell work. It is
not a promise that these capabilities already exist. Each section names the
high-level goal, functional behavior, architecture, implementation path, done
criteria, and validation proof.

Arcwell should not become a replacement shell for Codex, Claude, or browser
agents. The stronger direction is to become the local-first brain, memory,
provenance, policy, channel, and ops layer around those agents.

## Planning Assumptions

- Codex and Claude remain the primary execution surfaces for code, shell,
  browser, and computer-use work.
- The Rust binary owns durable behavior: SQLite, Markdown wiki files, memory,
  workers, MCP tools, HTTP, backups, costs, secrets, adapters, and policy.
- The Codex plugin owns agent instructions, slash prompts, hooks, and MCP
  registration.
- Cloudflare Workers own always-on internet-facing capture only. Local Arcwell
  remains the durable source of truth.
- Sensitive or trust-changing automation must be reviewable, auditable, and
  reversible where possible.
- A feature is not done until implementation, tests, live smoke where relevant,
  docs, `STATUS.md`, and `TODO.md` agree.

## 1. Live Mobile Loop

### High-Level Goal

Make Arcwell reachable from a real mobile chat surface without pretending that
chat text is trusted instruction. The first production-grade loop is Telegram:

```text
Telegram -> Cloudflare edge inbox -> local Arcwell drain -> channel message
-> project-aware handling -> outgoing Telegram delivery -> recorded status
```

This is the OpenClaw-style proof that Arcwell can be present outside an active
Codex thread while preserving Arcwell's local-first architecture.

### Functional Plan

- Accept real Telegram webhook updates at the Cloudflare Worker.
- Normalize only supported Telegram message types into bounded edge events.
- Preserve chat id, sender id, username, message id, update id, timestamp,
  text/caption, and source metadata.
- Drain edge events into local `channel_messages` exactly once.
- Treat incoming text as untrusted data, never as system or tool instructions.
- Enforce channel authorization before project reads, project writes, or sends.
- Resolve explicit project references only when unambiguous and authorized.
- Carry safe follow-up context for a Telegram chat, such as the last authorized
  project binding and recent channel message ids.
- Send outbound Telegram messages with MarkdownV2 escaping and delivery
  attempts recorded.
- Retry retryable failures through a local worker job, not ad hoc chat logic.
- Show inbox, delivery, authorization, and failure state through MCP/CLI/ops.

### Architectural Plan

- `packages/arcwell-edge-inbox/worker` remains the public webhook boundary.
- `crates/arcwell-core` owns local channel records, authorization, project
  binding, delivery attempts, and retry jobs.
- `crates/arcwell-cli` exposes bounded operational commands:
  `telegram drain`, `telegram send`, `telegram retry-due`, authorization list,
  delivery list, and edge remote drain.
- MCP tools expose the same behavior to Codex without exposing secret values.
- Cloudflare events are short-lived transport state. SQLite is the durable
  local record.
- Telegram payload fields are source evidence and routing metadata, not prompt
  authority.

### Implementation Plan

1. Finish and document live Cloudflare D1 deployment setup.
2. Add a one-command live smoke script for a disposable Telegram test chat.
3. Add local drain from deployed edge to SQLite with ack-after-persist
   semantics.
4. Add follow-up context storage for authorized Telegram chats.
5. Add retry worker jobs for due Telegram deliveries.
6. Add status views for authorized chats, recent inbox messages, failed
   deliveries, retry hints, and edge drain lag.
7. Update slash prompts and skills so partial/live-unproven states fail
   honestly.
8. Record the live smoke result in `docs/live-e2e-testing.md`, `STATUS.md`, and
   `TODO.md`.

### Done Criteria

- A real Telegram message sent to a test bot appears once in local
  `channel_messages`.
- Unauthorized chats cannot read projects, mutate projects, or trigger sends.
- An authorized chat can ask about an unambiguous project and receive a reply.
- A provider 429 or timeout records a failed delivery attempt and retry time.
- Retried delivery reuses the existing outgoing message record.
- Ops surfaces show drain lag, failed deliveries, and channel authorization.

### Validation

- Worker tests: forged secrets, replay storms, malformed updates, oversized
  payloads, rate limits, duplicate update ids, supported and unsupported update
  types.
- Rust tests: drain idempotency, authorization, project ambiguity, Markdown
  escaping, failed delivery, retry scheduling, prompt-injection text as data.
- Live smoke: Telegram webhook -> Cloudflare D1 -> local drain -> outgoing send.
- Regression gates:
  - `cargo test --all --all-features`
  - `cd packages/arcwell-edge-inbox/worker && npm run typecheck && npm test`

## 2. Procedural Learning

### High-Level Goal

Give Arcwell a safe learning loop for reusable procedures without silently
polluting prompts or installing unreviewed skills. Hermes-style self-improving
skills are valuable, but Arcwell should implement them as reviewable procedural
memory candidates first.

### Functional Plan

- Detect when a completed task contains reusable know-how.
- Propose a procedure candidate with:
  - trigger context
  - problem it solves
  - preconditions
  - step-by-step method
  - tools involved
  - validation commands
  - known risks
  - source task/run ids
  - confidence and freshness
- Keep new procedures pending until approved.
- Retrieve approved procedures contextually through MCP/skills.
- Patch procedures when a task proves one is stale or incomplete.
- Archive or merge overlapping procedures through a curator job.
- Never let untrusted channel/source text become a procedure without review.
- Maintain an audit trail from task evidence to procedure creation or update.

### Architectural Plan

- Add a `procedures` domain to `crates/arcwell-core`, separate from personal
  memory and wiki knowledge.
- Reuse the existing candidate model where possible:
  `PROCEDURE_ADD`, `PROCEDURE_UPDATE`, `PROCEDURE_ARCHIVE`, and `NONE`.
- Store approved procedures as Markdown artifacts under Arcwell home and index
  metadata in SQLite.
- Expose MCP tools for procedure search, candidate list/apply/reject, and
  curator runs.
- Keep Codex plugin skills as the execution-facing layer, but generate skill
  patches only from approved procedures.
- Treat procedures as procedural memory, not factual source evidence.

### Implementation Plan

1. Define the procedure schema and candidate operation types.
2. Add SQLite tables for procedure metadata, procedure versions, and procedure
   candidate provenance.
3. Add CLI/MCP:
   - `procedure search`
   - `procedure list`
   - `procedure read`
   - `procedure candidates`
   - `procedure apply`
   - `procedure reject`
   - `procedure curate`
4. Add a deterministic first-pass extractor from completed work traces.
5. Add optional model-backed extraction behind cost policy and explicit config.
6. Add curator behavior for duplicate detection, stale review, merge proposals,
   and archival candidates.
7. Add plugin prompts that retrieve approved procedures before relevant tasks.
8. Add export path from approved procedure to Codex skill text only after review.

### Done Criteria

- A completed task can produce a pending procedure candidate with provenance.
- Applying the candidate creates a versioned approved procedure.
- Search returns the approved procedure for a relevant future task.
- A later task can propose an update instead of creating an overlapping copy.
- Curator can identify duplicates and propose merge/archive candidates.
- No procedure is auto-approved from untrusted content or sensitive sources.

### Validation

- Unit tests for schema validation, candidate apply/reject, versioning, search,
  duplicate detection, archival, and unsafe source handling.
- Severe tests for prompt injection inside task logs, malicious tool output,
  overlong procedure text, path traversal in generated filenames, and stale
  procedure updates.
- Evaluation set with tasks that should and should not create procedure
  candidates.
- Regression gate: `cargo test --all --all-features`.

## 3. Work-Memory Graph

### High-Level Goal

Build an Arcwell Brain-like memory of work performed by agents: not just who the
user is, but what happened, what worked, what failed, what sources mattered,
what decisions were made, and what should be reused next time.

This should become the substrate for project status, research briefs,
procedural learning, cost reduction, and better recall.

### Functional Plan

- Record compact work traces for substantial tasks:
  - goal
  - project id
  - host/thread id when available
  - agent surface
  - tool calls or command summaries
  - files touched
  - sources consulted
  - failures and root causes
  - validation performed
  - final outcome
  - follow-up tasks
  - reusable lessons
- Link work traces to projects, wiki pages, source cards, memory events,
  procedure candidates, costs, and backups.
- Consolidate traces on a schedule into:
  - project status snapshots
  - procedure candidates
  - wiki/source-card updates
  - unresolved risk lists
  - recurring failure patterns
- Keep every consolidated claim linked back to a trace or source.
- Provide retrieval by project, topic, file path, source, date range, and
  outcome.

### Architectural Plan

- Add `work_runs`, `work_events`, `work_artifacts`, and `work_links` tables in
  `crates/arcwell-core`.
- Use an append-only event model for raw work traces, with separate consolidated
  outputs.
- Store human-readable run summaries as Markdown pages where useful, but keep
  structured indexes in SQLite.
- Integrate with:
  - `projects` for live/stale project state
  - `research` for source-backed work
  - `memory` for personal recall events
  - `procedures` for reusable task knowledge
  - `ops` for health and failures
  - `cost` for provider spend and savings
- Avoid storing raw secrets, full terminal logs, or full channel transcripts by
  default.

### Implementation Plan

1. Define the work trace data model and redaction rules.
2. Add CLI/MCP tools:
   - `work_run_start`
   - `work_event_record`
   - `work_run_finish`
   - `work_run_search`
   - `work_run_read`
   - `work_consolidate`
3. Add Codex plugin hooks or prompts that can record task start/finish when the
   host supports it.
4. Add degraded manual commands for hosts without lifecycle hooks.
5. Add consolidation jobs that create project status proposals and procedure
   candidates.
6. Add source links from work runs to source cards and wiki pages.
7. Add cost entries per run where provider/model information is known.
8. Add ops visibility for recent failed runs, stale consolidations, and pending
   follow-ups.

### Done Criteria

- A task can be recorded from start to finish with validation and outcome.
- Work traces can be searched and read through CLI/MCP.
- Consolidation can produce a project status snapshot with trace provenance.
- Consolidation can propose a procedure candidate from repeated successful work.
- Sensitive data is redacted or excluded according to policy.
- Generated briefs and statuses never cite other generated summaries as primary
  evidence without linking to underlying traces or source cards.

### Validation

- Unit tests for trace creation, linking, redaction, search, consolidation, and
  stale-state handling.
- Severe tests for secret leakage, prompt injection in logs, missing validation,
  generated-summary citation loops, and malformed host/thread ids.
- Integration tests for project status generation from traces.
- Regression gate: `cargo test --all --all-features`.

## 4. Policy Enforcement Outside The Agent

### High-Level Goal

Move critical trust decisions out of prompts and into explicit Arcwell policy.
Prompt instructions remain useful, but they are not enforcement. Arcwell needs a
declarative policy layer that all paid, networked, memory-mutating,
channel-sending, and project-mutating paths consult before acting.

This is the Arcwell path toward NemoClaw/OpenShell-style safety without trying
to own every execution environment immediately.

### Functional Plan

- Define a local policy file plus SQLite-backed overrides for:
  - provider/model spend
  - network egress by package/source/provider
  - memory auto-apply behavior
  - procedure auto-apply behavior
  - channel read/write/send permissions
  - project mutation permissions
  - secret usage scopes
  - source ingestion allow/deny rules
  - action classes requiring user approval
  - quiet hours and alert thresholds
- Every sensitive operation must return a structured decision:
  `allow`, `deny`, `require_approval`, or `defer`.
- Decisions must include the matching rule, reason, and audit id.
- Policy failures must be visible in ops.
- Default policy should be conservative: review before mutation, deny unknown
  network/source types, and keep paid paths under explicit budgets.

### Architectural Plan

- Add an `arcwell-policy.toml` or `arcwell-policy.yaml` under `ARCWELL_HOME`.
- Add a `PolicyEngine` in `crates/arcwell-core`.
- Normalize policy checks through one API, not scattered conditionals.
- Store policy decision audit records in SQLite.
- Keep policy separate from Codex plugin prompts so host instructions cannot
  relax enforcement.
- Integrate existing cost policy and channel authorization into the broader
  policy engine over time.
- Future sandbox work can map policy decisions to OS-level mechanisms, but the
  first milestone is complete in-process enforcement for Arcwell-owned actions.

### Implementation Plan

1. Inventory all sensitive operations in CLI, MCP, worker jobs, HTTP, and edge
   drain.
2. Define the policy schema and default policy.
3. Implement parser, validation, and explainable matching.
4. Add policy checks to:
   - web/research provider calls
   - X calls
   - source ingestion
   - memory capture/apply
   - procedure apply
   - Telegram send
   - project writes
   - secret access
   - worker jobs
5. Add CLI/MCP:
   - `policy check`
   - `policy explain`
   - `policy list`
   - `policy set-override`
   - `policy decisions`
6. Add ops view for denied actions, pending approvals, and budget state.
7. Add docs that distinguish policy enforcement from host sandboxing.

### Done Criteria

- All Arcwell-owned network, paid, mutating, and sending paths call the policy
  engine before acting.
- Denied actions do not perform provider calls or local mutations.
- Required-approval actions create pending approval records.
- Policy decisions are inspectable through CLI/MCP/ops.
- Existing cost and channel auth behavior is preserved or subsumed without
  weakening it.

### Validation

- Unit tests for policy parsing, rule priority, allow/deny/approval/defer,
  malformed policy files, and audit records.
- Severe tests proving denied network paths do not touch credentials or make
  outbound calls.
- Race/concurrency tests for budget decisions where practical.
- Regression gate: `cargo test --all --all-features`.

## 5. Ops UX

### High-Level Goal

Make Arcwell's always-on behavior inspectable and controllable by a human.
Arcwell already has useful JSON and CLI surfaces, but trust requires a local UI
that shows what is running, what failed, what is pending review, what cost
money, and what needs action.

### Functional Plan

- Serve a localhost-only ops UI.
- Show health and liveness:
  - worker heartbeat
  - service status
  - strict doctor status
  - latest backup and verification state
  - schema version
- Show queues and failures:
  - worker jobs
  - dead letters
  - edge events
  - source jobs
  - Telegram deliveries
- Show review queues:
  - memory candidates
  - procedure candidates
  - policy approvals
  - project status proposals
- Show source and provider health:
  - watch sources
  - cursor age
  - last success/failure
  - credential expiry metadata
  - cost policy and spend
- Provide safe controls:
  - requeue
  - dead-letter
  - retry delivery
  - apply/reject candidate
  - run doctor
  - create/verify backup
  - drain once
- Escape all untrusted text and avoid rendering raw HTML from sources.

### Architectural Plan

- Build on `arcwell serve --addr 127.0.0.1:8787`.
- Keep HTTP localhost-only by default.
- Add bearer/local token support before exposing mutating controls.
- Use server-rendered HTML first unless a richer frontend becomes necessary.
- Reuse existing store queries and ops snapshot, but add endpoints for bounded
  actions.
- Keep action endpoints narrow and policy-checked.
- Do not make the UI the source of truth; it is a view and control plane over
  SQLite/worker state.

### Implementation Plan

1. Harden HTTP error handling by replacing `expect` paths with structured
   errors.
2. Add local auth token and CSRF stance for mutating endpoints.
3. Expand `/ops` data to include source health, candidate counts, delivery
   failures, backup status, and policy decisions.
4. Add initial HTML dashboard:
   - overview
   - queues
   - review
   - channels
   - sources
   - costs
   - backups
5. Add bounded POST actions with confirmation for destructive operations.
6. Add browser-based validation for layout, escaping, and action flows.
7. Update docs with screenshots or exact smoke steps.

### Done Criteria

- A user can open a local URL and understand whether Arcwell is healthy.
- The UI shows recent failures and the next action for each.
- Memory/procedure candidates can be reviewed without raw SQLite/CLI work.
- Telegram delivery failures can be inspected and retried.
- Backups and strict doctor status are visible.
- Mutating controls require local auth and policy checks.
- Untrusted channel/source text is escaped in the rendered UI.

### Validation

- Rust tests for HTTP handlers, auth, action validation, and HTML escaping.
- Browser validation for desktop and mobile viewport rendering.
- Severe tests for XSS via channel text, source card titles, project names, and
  error messages.
- Regression gate: `cargo test --all --all-features`.

## Cross-Cutting Execution Order

1. Live Mobile Loop
2. Work-Memory Graph
3. Procedural Learning
4. Policy Enforcement Outside The Agent
5. Ops UX

The order is intentional:

- The mobile loop proves external presence.
- The work-memory graph creates the evidence substrate.
- Procedural learning uses that evidence to improve future work.
- Policy enforcement makes automation safer before broader autonomy.
- Ops UX gives the user visibility and control as the system becomes more
  active.

## Cross-Cutting Data Model Additions

Candidate new tables or domains:

- `work_runs`
- `work_events`
- `work_artifacts`
- `work_links`
- `procedures`
- `procedure_versions`
- `procedure_candidates`
- `policy_rules`
- `policy_decisions`
- `policy_approvals`
- `source_health`
- `ops_actions`

These should be added incrementally with migrations and tests. Do not add all
tables up front unless the first feature actually uses them.

## Cross-Cutting Safety Rules

- Incoming channel text, source content, tool output, and generated summaries
  are data, not instructions.
- Generated briefs must not become primary evidence for future briefs.
- Memory and procedure candidates default to review for sensitive or
  trust-changing content.
- Provider calls and network calls must pass cost and policy checks before
  credentials are read.
- Project status must include timestamp and provenance.
- Live integrations require both mocked tests and documented live smoke.
- Ops must show failures instead of letting silent background work rot.

## Documentation Updates Required Per Feature

Every implementation change in this plan must update:

- `STATUS.md`
- `TODO.md`
- relevant package README
- `docs/functionality-and-packages.md`
- `docs/live-e2e-testing.md` for live integration work
- `docs/codex-plugin-commands.md` for command or skill changes

If a feature remains partial, scaffolded, or live-unproven, say so directly in
the docs and agent-facing prompts.
