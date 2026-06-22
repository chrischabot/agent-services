# Arcwell Real-User Readiness Plan

Last updated: 2026-06-22

This plan is the design and implementation map for the remaining work needed to
make Arcwell real-user ready. It replaces the older missing-pieces plan and is
grounded in the current `STATUS.md` audit, current tests, and the remaining
unchecked work in `TODO.md`.

Arcwell's strongest direction is still clear: it should be the local-first
memory, provenance, policy, channel, research, and ops layer around Codex,
Claude, browser agents, Cloudflare, and user-owned tools. It should not pretend
to replace those hosts, and it should not claim live automation until a real
host or provider has been exercised.

## Done Standard

A capability is done only when all of these agree:

- implementation exists in code, not only prompts or docs
- local/mock tests cover the happy path and credible failure paths
- severe/adversarial tests cover trust, auth, replay, malformed input, size, and
  provider failure where relevant
- live smoke evidence exists for external providers, deployed services, or host
  clients
- ops surfaces show the state, failure, and next action
- `STATUS.md`, `TODO.md`, package docs, and agent-facing skills/prompts state
  the exact current capability without overclaiming

## 1. Live Telegram And Mobile Channel Loop

### High-Level Design

The first production-grade mobile loop is Telegram:

```text
real Telegram client
-> Telegram bot webhook
-> Cloudflare edge inbox
-> local Arcwell remote drain
-> durable channel message
-> authorization and project/context resolution
-> controller route report or assistant response
-> outgoing Telegram delivery attempt
-> ops visibility and retry
```

Current reality: most of the code path exists and local/synthetic/deployed-edge
proof is strong. The missing proof is the strict fresh real-client incoming
message that lands exactly once in local `channel_messages` and produces the
expected route report after drain.

### Functional Design

- Accept only configured Telegram webhooks through the Cloudflare Worker.
- Normalize text and caption messages; keep other media/update types explicitly
  out of scope until supported.
- Persist edge events while the local service is offline.
- Drain edge events locally with ack-after-local-persist semantics.
- Treat all Telegram text as untrusted channel data, never instructions.
- Enforce chat/sender authorization before project reads, project writes,
  controller actions, or outgoing replies.
- Preserve follow-up context only for authorized chats: last authorized project,
  recent message ids, and freshness.
- Record controller route reports separately from drain persistence.
- Record outgoing delivery attempts, provider failures, retry hints, and
  token-redacted error classes.

### Architecture

- Cloudflare Worker owns public ingress, D1 persistence, leasing, rate limiting,
  idempotency, current/next secret auth, and Telegram normalization.
- Arcwell core owns `channel_messages`, channel authorization, project binding,
  delivery attempts, route reports, and retry scheduling.
- Arcwell CLI/MCP expose bounded operations: drain, inbox, send, retry, delivery
  inspection, authorization inspection, and edge event inspection.
- Ops UI renders freshness, queue age, failed deliveries, authorization state,
  and drain lag.
- Policy and cost layers gate outbound sends, remote drains, and future
  automatic responses.

### Implementation Plan

1. Re-run `scripts/telegram-live-smoke` against the deployed worker and a
   disposable Telegram chat; send the exact printed phrase from the real client.
2. Make the smoke assert exactly one matching local message and one route report
   for the drained update.
3. Add safe follow-up context carryover for authorized chats with expiry,
   ambiguity handling, and explicit reset.
4. Add production monitoring hooks for edge lag, failed Telegram deliveries,
   repeated nacks, and webhook freshness.
5. Add Miniflare coverage only if deployed-worker behavior diverges from local
   Node/D1-fake tests again.

### Validation And Done Criteria

- Fresh real-client Telegram message reaches local SQLite exactly once.
- Duplicate update ids and repeated drains do not duplicate records.
- Unauthorized chats cannot read or mutate projects or send messages.
- Follow-up context expires and refuses ambiguous project references.
- Provider 429/timeouts are recorded as retryable without leaking bot tokens.
- Ops surfaces show lag, last drain, failed deliveries, and authorization state.

## 2. Codex And Claude Host Integration Proof

### High-Level Design

Arcwell needs proof inside real host clients, not only subprocess tests. Codex
and Claude should both be honest surfaces: Codex can use plugin skills, hooks,
and slash commands; Claude can use MCP but lacks Arcwell's Codex-specific
hook/skill lifecycle.

Current reality: plugin generation, docs verification, process-level hook smoke,
local MCP stdio smoke, and MCP Inspector availability are proven. Fresh Codex
thread behavior, authenticated Claude Desktop/Code behavior, and interactive
Inspector evidence remain unproven.

### Functional Design

- Codex fresh-thread smoke covers slash commands, MCP tools, skills, hooks, and
  dev/stable plugin selection.
- Claude smoke covers MCP server discovery, tool listing, representative calls,
  resource reads, malformed requests, and degraded lifecycle messaging.
- Host-sync state is freshness-bounded and provenance-labeled.
- Native Codex/Claude thread inventory is used only if a stable API exists.
- Manual/degraded sync is explicit and cannot masquerade as live host state.

### Architecture

- Stable plugin invokes `arcwell` on `PATH`.
- Dev plugin invokes this checkout's debug wrapper through `.arcwell-dev`.
- `arcwell mcp` remains the portable stdio boundary.
- Host-adapter commands/skills consume host-provided tools only inside a
  resident host session.
- Project status records include host, thread id, source, timestamp, freshness,
  and verification method.

### Implementation Plan

1. Record a fresh Codex thread smoke using the manual matrix in
   `docs/codex-plugin-commands.md`.
2. Record live Codex hook execution from the installed plugin, separate from the
   process-level hook smoke.
3. Run and record interactive MCP Inspector against `arcwell mcp`.
4. Configure and validate Claude Desktop/Code with an authenticated local
   profile.
5. Keep native thread inventory as conditional work until a stable host API is
   exposed.
6. Document degraded behavior in agent-facing prompts so host gaps are visible.

### Validation And Done Criteria

- A fresh Codex thread can call representative slash commands and MCP tools.
- Codex hooks record lifecycle events in a disposable home.
- Claude Desktop/Code can discover and call representative Arcwell MCP tools.
- MCP Inspector evidence is recorded.
- Stale/manual host snapshots are labeled as stale/manual, not live.

## 3. Packaging, Release, Install, And Upgrade

### High-Level Design

Arcwell needs a boring install path for open-source users: install, upgrade,
run service, use the Codex plugin, verify health, and uninstall without data
loss. Local release-readiness is not the same as public release proof.

Current reality: release readiness scripts, installer scaffolds, Homebrew
template, macOS LaunchAgent live smoke, and systemd rendering exist. Public
signed/checksummed artifacts, Homebrew publication, fresh Codex app smoke, and
Linux systemd live proof remain missing.

### Functional Design

- Install a versioned `arcwell` binary and stable Codex plugin.
- Verify checksums before installing release archives.
- Render Homebrew formula from real release artifacts and checksums.
- Support upgrade with backup-before-destructive-migration guarantees.
- Support uninstall without deleting `ARCWELL_HOME` unless explicitly requested.
- Support macOS LaunchAgent and Linux systemd user services.
- Provide a first-run doctor that explains missing service, stale backup,
  credentials, and plugin state.

### Architecture

- Release artifacts are built from the Rust workspace and package templates.
- Installer and Homebrew formula consume the same checksummed artifact metadata.
- Stable plugin uses `arcwell` from `PATH`; dev plugin remains checkout-local.
- Service renderers are non-destructive and path-escape install/home/log paths.
- `doctor --strict` is the release acceptance gate after install/upgrade.

### Implementation Plan

1. Produce real release archive artifacts with signed or published checksums.
2. Generate Homebrew formula input from those artifacts.
3. Smoke install through Homebrew/local tap on macOS.
4. Run fresh Codex app `arc` smoke against the installed stable plugin.
5. Run Linux `systemctl --user` install/status/restart/journal/uninstall on a
   Linux runner.
6. Add CI or release-script gates for checksum, archive traversal, migration,
   service rendering, plugin PATH, and uninstall preservation.

### Validation And Done Criteria

- A clean user can install Arcwell without `cargo`.
- `arcwell doctor --strict` passes after install when required setup exists.
- Homebrew and installer paths verify the same artifact checksum.
- Linux user-service live smoke passes on a real Linux session.
- Uninstall preserves user data by default.

## 4. Ops, Monitoring, And Human Control Surface

### High-Level Design

Arcwell should be operable by a human without reading SQLite or tailing logs.
The ops surface must show what is stale, broken, queued, blocked, or waiting for
review, and it must expose mutating controls only when the core API can make the
operation safe, idempotent, audited, CSRF-protected, and policy-checked.

Current reality: `/ops`, MCP resources, and `/ops/ui` expose broad read state,
filters, detail views, summaries, and one narrow edge-event dead-letter control.
It is not yet a product-grade control room.

### Functional Design

- Show health, backups, worker heartbeat, queues, dead letters, edge events,
  source health, cursors, credentials, costs, channels, projects, memory and
  procedure review, work runs, policy decisions, delivery failures, and project
  status proposals.
- Add remediation controls only for operations with safe public core APIs.
- Preserve Obsidian/Markdown as the wiki editing surface until a real product
  need justifies duplication.
- Provide charts and trend summaries for queue age, failed deliveries, costs,
  stale sources, backups, and provider health.
- Separate read-only diagnostic state from mutating actions.

### Architecture

- Core owns durable state and action APIs.
- CLI HTTP server owns local auth, CSRF, origin checks, request limits,
  idempotency keys, structured redacted errors, and policy decisions.
- `/ops/ui` is currently server-rendered HTML; a frontend package is justified
  only if richer interactions outgrow server rendering.
- Every mutating control writes an audit event and returns a typed result.

### Implementation Plan

1. Decide whether server-rendered HTML remains sufficient before adding many
   controls.
2. Add safe core APIs for job requeue/cancel before UI controls.
3. Add controls for retry delivery, apply/reject candidates, run doctor,
   create/verify backup, drain once, and inspect policy denial reasons.
4. Add browser validation for the richer current UI on desktop and mobile.
5. Add charts and stale-state summaries.
6. Add live-provider probe summaries where probes can be safe and cheap.

### Validation And Done Criteria

- Browser smoke shows no overlap/clipping on desktop or mobile.
- All untrusted state is escaped.
- Mutating actions require auth, CSRF, idempotency, policy checks, and audit.
- Failed controls explain whether the operation is unsupported, denied, stale,
  or failed after execution.

## 5. Proactive Delivery: Email, Telegram, Librarian, And X

### High-Level Design

Arcwell should be able to proactively deliver useful alerts and digests, but
only after routing, schedule, thresholds, quiet hours, dedupe, authorization,
policy, cost, and delivery attempts are explicit. Proactive delivery must not
turn inbound messages or source text into trusted instructions.

Current reality: email ingress/outbound and Telegram outbound have strong
bounded proof; X and source monitors can create candidates; librarian delivery
is still not wired as a production loop.

### Functional Design

- Define digest candidates with source evidence, reason, priority, confidence,
  freshness, and delivery eligibility.
- Schedule delivery windows with quiet hours and per-channel preferences.
- Deduplicate repeated findings across X, RSS, GitHub, arXiv, web, email, and
  research sources.
- Authorize recipients per channel and purpose.
- Send rich HTML email and safe Telegram messages with delivery attempts.
- Record skipped, blocked, sent, failed, retryable, and permanently failed
  states.
- Keep Gmail host-native for selected interactive mailbox work; Arcwell-owned
  email capture remains Cloudflare Email Routing plus local drain.

### Architecture

- Digest candidates live in core and link to source cards, channel messages,
  work runs, or research runs.
- Worker jobs evaluate schedules, thresholds, quiet hours, dedupe, policy, and
  cost before send.
- Channel delivery reuses Telegram/email delivery-attempt infrastructure.
- Ops UI shows candidate age, blocked reasons, delivery history, and next run.
- X Cloudflare callback/cron events use the edge inbox only after durable
  monitoring and replay behavior are clear.

### Implementation Plan

1. Design the digest scheduler state machine.
2. Implement delivery eligibility and dedupe decisions with audit records.
3. Wire email and Telegram delivery attempts for digest candidates.
4. Add quiet hours, per-channel routing, and recipient authorization.
5. Add production monitoring for email/Telegram critical alert paths.
6. Add X Cloudflare callback/cron event capture after edge inbox monitoring is
   sufficient.
7. Add model-backed interestingness only behind policy, cost, and eval gates.

### Validation And Done Criteria

- A fixture source event creates exactly one eligible digest candidate.
- Quiet hours and duplicate detection suppress delivery with inspectable reason.
- Unauthorized recipients and policy-denied sends do not call providers.
- Delivery attempts are recorded and retryable errors are retried safely.
- Ops can answer why a candidate did or did not deliver.

## 6. Deep Research Quality And Host-Native Execution

### High-Level Design

Arcwell Research should mean deep research: broad discovery, source-card and
document evidence, claim extraction, contradiction/refutation, cited synthesis,
audit gates, and durable writeback. It should not sound smarter than its
evidence.

Current reality: the local research substrate is strong and recently expanded:
role traces, source ledgers, host-search proof records, structured claims,
documents/tables, evidence packs, editorial/eval records, mock/OpenAI invocation,
and audit-clean preserved large-corpus reports. Missing proof is fresh in-app
Codex orchestration, actual host-native search, live editorial quality, broader
difficult-document coverage, and page expansion.

### Functional Design

- Expand seed sources into related docs, blogs, repos, papers, social posts, and
  local wiki evidence before synthesis.
- Use host-native search in Codex/OpenAI/Claude where available and record proof:
  query, host/tool, result metadata, selected results, retrieval context, and
  linked source cards.
- Run scout, corpus builder, extractor, skeptic, synthesizer, and auditor roles
  as real subagents where the host supports them; otherwise record degraded
  sequential role phases.
- Require citation verification and adversarial evaluation before completed
  drafts.
- Link claims and report citations to source cards, document spans, and table
  cells.
- Preserve uncertainty, contradiction, source-family coverage, and saturation
  reason in reports.

### Architecture

- Core owns research runs, role tasks, role-run traces, artifacts, source
  ledger, run-source links, claims, clusters, document artifacts, evidence
  packs, editorial/eval records, audits, and compiled reports.
- Host plugins own role prompts and host-native tool execution.
- Provider adapters invoke model-backed editorial/eval work only after policy
  and cost approval.
- Reports are generated artifacts, never primary evidence.

### Implementation Plan

1. Add active page expansion before topic page/report writing.
2. Add native host-search execution paths for Codex/OpenAI and Claude where
   available.
3. Run a fresh in-app Codex subagent orchestration smoke with capability
   discovery and degraded-mode recording.
4. Run live provider-backed editorial synthesis/evals with cost records.
5. Expand difficult document fixtures for PDFs, XLSX, table extraction, and
   publication-grade citation links.
6. Add reference-topic live runs after the above path is proven.

### Validation And Done Criteria

- A fresh host-run deep research task records real role traces and source proof.
- Every material report claim links to evidence and audit state.
- Missing primary evidence, stale sources, generated recursion, weak citations,
  and evaluator rejection block completed status.
- Provider editorial/eval calls record cost and artifacts.

## 7. Memory, Work Graph, And Procedural Retrieval Loop

### High-Level Design

Arcwell needs three related but distinct memory layers:

- personal memory: durable user facts/preferences with review and forget
- work-memory graph: what happened in tasks, with evidence and validation
- procedural learning: reusable ways of working, approved before reuse

Current reality: local core pieces exist for all three. The missing product loop
is host lifecycle capture, scheduled consolidation, model-backed extraction
quality, human review UI, procedure retrieval before relevant tasks, and live
Codex/Claude proof.

### Functional Design

- Capture task start/finish, files touched, sources consulted, failures, root
  causes, validation, outcome, follow-ups, and reusable lessons.
- Consolidate traces into unresolved risks, recurring failures, stale runs,
  pending follow-ups, and project status proposals.
- Extract procedure candidates from traces, optionally model-backed behind cost
  and policy gates.
- Retrieve approved procedures before relevant tasks through plugin prompts or
  hooks.
- Keep personal memory sensitive facts reviewable, forgettable, and auditable.
- Provide a human review UI for memory/procedure/project-status candidates.

### Architecture

- Core owns work runs/events/artifacts/links, memory lifecycle events,
  procedure candidates/procedures, project status proposals, and review state.
- Host plugins/hooks bridge Codex lifecycle events into core where supported.
- Claude uses degraded/manual capture unless host lifecycle APIs are available.
- Ops UI is the shared review/control surface.
- Backup and tombstone policy defines what forget means for active stores and
  retained historical backups.

### Implementation Plan

1. Add Codex plugin prompts/hooks for task start/finish capture where supported.
2. Live-smoke Codex hooks and Claude degraded memory workflow.
3. Add scheduled consolidation for unresolved risks, recurring failures, stale
   runs, pending follow-ups, and reusable lessons.
4. Add optional model-backed procedure extraction behind explicit config and
   cost policy.
5. Add plugin prompt retrieval for approved procedures before relevant tasks.
6. Add live Codex/Claude procedure retrieval smoke.
7. Add human review UI for memory/procedure/project-status candidates.
8. Add live model-backed memory/procedure quality evals.

### Validation And Done Criteria

- A host task creates a work run with validation and outcome evidence.
- Consolidation proposes status/follow-up/risk items with trace provenance.
- Approved procedures are retrieved in a later relevant task.
- Model-backed extraction is gated, costed, auditable, and evaluated.
- Forget removes active personal memory data and clearly reports retained-backup
  limitations or erasure/rotation status.

## 8. Policy, Cost, Secrets, And Provider Safety

### High-Level Design

Arcwell-owned actions must be governed outside prompt text. Policy, cost,
credential health, provider probes, and approval state are product boundaries,
not best-effort instructions.

Current reality: the policy engine, estimated cost gates, secret redaction, and
many high-risk guards exist. The known risk is incomplete sensitive-operation
inventory, missing actual provider-cost reconciliation, and limited live
provider credential/probe evidence.

### Functional Design

- Inventory every sensitive operation across CLI, MCP, worker jobs, HTTP, edge
  drain, memory, projects, channels, source ingestion, provider adapters, and
  credential helpers.
- Apply policy decisions before credential lookup, network calls, sends,
  mutations, worker enqueue/execution, provider calls, and secret admin.
- Record allow, deny, require-approval, defer, override, and approval decisions.
- Reconcile provider-reported usage/cost where APIs reliably expose it.
- Probe credential presence, expiry, scopes, revocation, and provider health
  without leaking secret values.
- Surface denials, pending approvals, budget burn, stale credentials, and probe
  failures in ops.

### Architecture

- `PolicyEngine` evaluates declarative local policy and writes decisions.
- Cost policies reserve estimated spend before provider/network work.
- Provider adapters expose optional actual-usage records.
- Secret store separates local secret values, external refs, metadata, expiry,
  scope, and provider health.
- Ops and doctor show state without values.

### Implementation Plan

1. Complete and document the sensitive-operation inventory.
2. Add missing policy guards from the inventory before side effects.
3. Add provider-reported cost reconciliation for APIs with reliable usage data.
4. Add live provider credential probes and revocation/rotation helpers.
5. Add scheduled rotation reminders and stale-scope warnings.
6. Add ops UI controls for approvals, denials, cost burn-down, and credential
   remediation only where safe.

### Validation And Done Criteria

- Denied actions do not read credentials, enqueue jobs, mutate local state, or
  call providers.
- Required-approval actions create auditable pending approvals.
- Provider actual-cost records reconcile without double-counting estimates.
- Secret values never appear in logs, MCP resources, ops snapshots, backups, or
  provider error strings.

## 9. Backup, Forget, Recovery, And Retention

### High-Level Design

Arcwell must be recoverable and honest about deletion. Local backup/restore is
implemented, but real-user readiness needs scheduled backup, off-machine copies,
encryption, restore drills, and a clear retained-backup erasure or rotation
policy for forgotten data.

Current reality: local backup create/verify/restore and severe restore drills
exist. Historical backup erasure is explicitly not claimed; tombstones warn
that retained snapshots may still contain forgotten data.

### Functional Design

- Schedule local backups through the worker/service.
- Support off-machine backup target configuration.
- Support encrypted backup archives with key-management documentation.
- Run automated restore drills into disposable homes.
- Track backup freshness, verification, encryption status, target, and last
  restore-drill result in ops.
- Define retained-backup erasure or rotation behavior for forget requests.

### Architecture

- Core backup API owns manifest, checksum, WAL checkpoint, included artifacts,
  verification, restore, and tombstone records.
- Worker schedules backup and restore-drill jobs.
- Secret/key material is never embedded in manifests or logs.
- Ops/doctor fail loudly for stale, unverifiable, unencrypted, or missing
  backup state according to policy.

### Implementation Plan

1. Add scheduled local backup jobs.
2. Add encrypted archive support and documentation.
3. Add off-machine target abstraction and at least one tested target.
4. Add disposable restore-drill automation.
5. Add retained-backup erasure/rotation implementation for forget policy.
6. Add ops UI state and controls for create, verify, restore-drill, and policy
   warnings.

### Validation And Done Criteria

- Backup can be created, verified, copied off-machine, restored, and drilled.
- Stale/unverified backup state fails strict doctor.
- Forget policy states exactly which retained backups were erased, rotated, or
  still contain tombstoned data.
- Corrupt, tampered, partial, and path-traversal backups fail closed.

## 10. Garderobe Deployment And Provenance Boundary

### High-Level Design

Garderobe should remain a working adjacent personal-domain MCP app with a clear
Arcwell package boundary. Arcwell can point agents to Garderobe for wardrobe
planning, but it must not leak private inventory into memory/wiki/profile by
default or ship code with unclear redistribution rights.

Current reality: the package is vendored with private seeds/secrets excluded,
docs and local tests exist, and a read-only smoke script is guarded. Missing
proof is fresh live read-only smoke, authenticated/write-capable MCP evidence
with disposable fixture data, and license/provenance clearance.

### Functional Design

- Keep Garderobe inventory private unless the user explicitly syncs selected
  metadata into Arcwell.
- Use disposable fixture rows for write-capable live tests.
- Maintain OAuth/DCR/PKCE and MCP boundaries.
- Document weather/profile/style context handoff without copying private seed
  data.
- Preserve placeholders in tracked config.

### Architecture

- `packages/arcwell-garderobe` owns vendored Worker/MCP code and package docs.
- Arcwell core treats Garderobe as an external package/integration, not a memory
  source by default.
- Live smoke scripts require explicit confirmation, base URL, and fixture scope.
- License/provenance documentation gates redistribution.

### Implementation Plan

1. Run guarded read-only live smoke against the approved deployed base URL.
2. Provision disposable/staging fixture rows for authenticated/write-capable MCP
   smoke.
3. Record Claude/host OAuth handshake proof if that is part of supported use.
4. Resolve top-level license/provenance for vendored Garderobe code.
5. Add package docs for what can sync into Arcwell and what never syncs by
   default.

### Validation And Done Criteria

- Read-only live smoke passes without exposing private inventory.
- Authenticated/write-capable MCP smoke uses disposable data.
- Hostile wardrobe metadata is treated as data.
- Redistribution/license status is explicit.
- Arcwell memory/wiki/profile remain unchanged unless explicit sync is chosen.

## Recommended Sequencing

1. Finish host/client proof and Telegram exact live proof first; these decide
   whether Arcwell is actually usable through the surfaces people will touch.
2. Tighten packaging/release next; real users need install and service behavior
   before broader automation.
3. Build ops controls only where the core action APIs are already safe.
4. Complete proactive delivery after channel delivery, policy, cost, and ops are
   trustworthy.
5. Expand research, memory, procedures, and policy in parallel, but keep every
   model/provider-backed claim behind severe tests and live proof.
