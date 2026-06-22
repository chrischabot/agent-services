# Arcwell Controller Design

Status: Phase 0 local controller ledger/router implemented; Telegram drain now
routes recorded messages through the controller; a Codex resident host-adapter
skill can consume pending actions with Codex app thread tools. Fresh live
Telegram-to-Codex proof and a daemon-grade hard-stop API remain.

## Goal

Arcwell must let an authorized owner control Codex and related assistant work
from Telegram and future channels.

Required user flows:

- Telegram: "hows codex swift doing" -> project and thread status.
- Telegram: "Implement this feature in arcwell" -> create or resume the right
  Codex thread and start implementation.
- Telegram: "hows it going" -> overview of active/recent work: "Foo finished,
  Bar is working on blah, Baz is blocked on approval."
- Telegram: "stop the blog work" -> resolve project/thread/run, interrupt or
  cancel it, record the decision, and report what changed.
- Telegram: "summarize my recent x bookmarks and mail me a report" -> start a
  new work thread/run, import/source X bookmarks, produce a report, send email,
  and keep progress visible.
- Telegram: "whats on my schedule today" -> use host Google Calendar capability
  or an Arcwell workspace adapter, then reply with a compact schedule.

This is not a Telegram-only feature. Telegram is the first channel. The product
boundary is a controller that understands channels, projects, threads, runs,
tools, policy, delivery, and host capabilities.

## What To Borrow

### OpenClaw Lessons

OpenClaw's important shape is a gateway/control-plane split:

- one gateway receives messages from many channels;
- channels own platform grammar, auth, media, formatting, and receipts;
- core owns shared message dispatch, session/thread bookkeeping, prompt wiring,
  and execution routing;
- session tools let agents list/read/send/spawn/yield/status across sessions;
- Codex is a harness/runtime behind the gateway, not the gateway itself.

For Arcwell, translate that as:

- `arcwell-channel` owns normalized inbound/outbound message contracts.
- `arcwell-telegram`, `arcwell-email`, and future channels own native platform
  details.
- `arcwell-controller` owns routing, project/thread/run registries, policy, and
  host adapters.
- Codex owns native thread execution, compaction, tool continuation, and code
  work. Arcwell records, routes, monitors, and controls.

Do not copy OpenClaw's full gateway. Arcwell already has a local Rust substrate,
Cloudflare edge inbox, policy, costs, work traces, X, email, and project state.
Build the missing controller on top of those.

### Hermes Lessons

Hermes' useful shape is a long-running gateway plus normalized `MessageEvent`,
busy-session guards, interrupt/stop paths, persistent SQLite session history,
background runs, and API/TUI methods for list/status/history/interrupt.

For Arcwell:

- every inbound channel item becomes a normalized controller message;
- active runs have explicit owner state, pending follow-ups, interrupt events,
  and bounded cancellation;
- status can be read while work is running;
- `/stop`-style controls bypass normal busy queues;
- background work emits durable events and can deliver completion to a channel;
- SQLite is the operational truth, not raw chat transcripts.

## Architecture

```text
Telegram / email / future channel
  -> Cloudflare edge inbox or local channel listener
  -> local drain
  -> channel_messages
  -> controller_ingest_message
  -> policy + identity + route resolution
  -> one of:
       status read
       create/resume/continue Codex thread
       stop/interrupt/cancel work
       run workflow: X -> report -> email
       host connector query: Google Calendar
  -> work_runs + controller events + project/thread snapshots
  -> channel outbox
  -> Telegram/email delivery receipt
```

The controller runs locally as part of Arcwell's resident service. Cloudflare is
only a bounded capture/delivery front door. If the local controller is offline,
edge events queue, but thread creation/resume and Google/X/email work do not
pretend to run.

## Core Components

### Channel Ingress

Inputs:

- `channel_messages` from Telegram, email, and future sources;
- source event ids and idempotency keys;
- channel, account, chat/thread/sender identity;
- body and attachment metadata as untrusted evidence.

Responsibilities:

- dedupe;
- enforce sender/channel authorization;
- attach prior channel context;
- record a controller message envelope;
- dispatch to the intent router.

Non-goal: channel text never becomes authority. It can request an action, but
identity and policy decide whether the action may happen.

### Intent Router

The first router should be deterministic plus model-assisted only where useful.
It returns a typed intent with confidence and candidate references.

Intent classes:

- `project_status`
- `thread_status`
- `active_work_status`
- `create_work_thread`
- `continue_thread`
- `stop_work`
- `x_bookmark_report_email`
- `calendar_today`
- `approve_pending`
- `deny_pending`
- `clarify`
- `unknown`

Examples:

- "hows codex swift doing" -> `project_status(project_ref="codex swift")`
- "hows it going" -> `active_work_status(context=last_channel_context)`
- "Implement this feature in arcwell" ->
  `create_work_thread(project_ref="arcwell", prompt=...)`
- "stop the blog work" -> `stop_work(project_ref="blog")`
- "summarize my recent x bookmarks and mail me a report" ->
  `x_bookmark_report_email(recency=recent, delivery=email)`

The router must ask a clarifying question when multiple active projects/threads
fit. It must not guess silently.

### Controller Registry

Arcwell already has `projects`, `project_status_snapshots`, `work_runs`,
`work_events`, `channel_messages`, authorizations, costs, policy decisions, X,
email, and source-card tables. The controller should add the missing thread/run
control tables rather than overloading project snapshots.

Proposed tables:

```sql
CREATE TABLE controller_channel_contexts (
  id TEXT PRIMARY KEY,
  channel TEXT NOT NULL,
  account_id TEXT,
  conversation_id TEXT NOT NULL,
  sender TEXT NOT NULL,
  trust_tier TEXT NOT NULL,
  last_project_id TEXT,
  last_thread_id TEXT,
  last_run_id TEXT,
  last_intent TEXT,
  updated_at TEXT NOT NULL,
  UNIQUE(channel, account_id, conversation_id, sender)
);

CREATE TABLE controller_threads (
  id TEXT PRIMARY KEY,
  host TEXT NOT NULL,
  host_thread_id TEXT NOT NULL,
  project_id TEXT REFERENCES projects(id),
  title TEXT,
  cwd TEXT,
  branch TEXT,
  worktree TEXT,
  status TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 0,
  archived INTEGER NOT NULL DEFAULT 0,
  current_goal TEXT,
  latest_summary TEXT,
  latest_summary_source TEXT,
  last_activity_at TEXT,
  last_synced_at TEXT NOT NULL,
  UNIQUE(host, host_thread_id)
);

CREATE TABLE controller_runs (
  id TEXT PRIMARY KEY,
  thread_id TEXT REFERENCES controller_threads(id),
  project_id TEXT REFERENCES projects(id),
  origin_channel_message_id TEXT REFERENCES channel_messages(id),
  host TEXT NOT NULL,
  host_run_id TEXT,
  kind TEXT NOT NULL,
  status TEXT NOT NULL,
  requested_action TEXT NOT NULL,
  cancel_requested INTEGER NOT NULL DEFAULT 0,
  cancel_reason TEXT,
  started_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  finished_at TEXT
);

CREATE TABLE controller_events (
  id TEXT PRIMARY KEY,
  run_id TEXT REFERENCES controller_runs(id),
  thread_id TEXT REFERENCES controller_threads(id),
  project_id TEXT REFERENCES projects(id),
  event_type TEXT NOT NULL,
  summary TEXT NOT NULL,
  data_json TEXT NOT NULL,
  source TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE controller_pending_actions (
  id TEXT PRIMARY KEY,
  channel TEXT NOT NULL,
  conversation_id TEXT NOT NULL,
  sender TEXT NOT NULL,
  action_type TEXT NOT NULL,
  project_id TEXT,
  thread_id TEXT,
  run_id TEXT,
  payload_json TEXT NOT NULL,
  reason TEXT NOT NULL,
  status TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  resolved_at TEXT
);

CREATE TABLE controller_outbox (
  id TEXT PRIMARY KEY,
  channel TEXT NOT NULL,
  target TEXT NOT NULL,
  related_message_id TEXT,
  run_id TEXT,
  body TEXT NOT NULL,
  status TEXT NOT NULL,
  idempotency_key TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL,
  delivered_at TEXT
);
```

Existing tables still matter:

- `project_status_snapshots` stores project-level summaries.
- `work_runs` and `work_events` remain the evidence substrate for agent work.
- `channel_messages` stores inbound/outbound channel evidence.
- `channel_authorizations` gates project reads/writes and sends.
- `policy_decisions` and `policy_approvals` gate mutating/high-risk actions.
- X/email/source-card tables supply workflow data.

### Host Adapters

The controller needs a narrow host adapter trait:

```text
HostAdapter
  list_threads(filter) -> ThreadSummary[]
  read_thread(thread_id, cursor?, limit?) -> ThreadRead
  create_thread(project, prompt, options) -> ThreadHandle
  send_message(thread_id, prompt, options) -> RunHandle
  stop_thread(thread_id | run_id, reason) -> StopResult
  fork_thread(thread_id, options) -> ThreadHandle
  archive_thread(thread_id, archived) -> Result
  stream_events(thread_id | run_id) -> EventStream
```

Adapters:

- `codex-desktop-host`: implemented as the Codex plugin
  `$arcwell-codex:codex-host-adapter` skill. It uses Codex app thread tools when
  running inside Codex, processes `controller_pending_actions`, upserts
  `controller_threads`, creates/updates `controller_runs`, and records verified
  project sync snapshots. It is good for resident Codex proof, but not enough
  for a headless daemon unless Codex exposes a durable external API.
- `codex-app-server`: preferred resident path. Uses Codex SDK/app-server for
  list/read/create/resume/fork/stop/events. This is the Phase 0 spike because
  the whole product depends on stable access.
- `claude-acp-or-cli`: future adapter for Claude Code/ACP if a stable lifecycle
  surface exists.
- `google-host`: not a standalone Google clone. It calls host Google Calendar,
  Gmail, or Drive tools when available, and otherwise returns a clear unavailable
  result.

Host adapters must fail closed. If a host cannot create a thread, stop a run, or
read live state, the controller says that and records an unavailable event.

### Policy And Trust

Action classes:

- `read_status`: allowed for authorized owner channels.
- `read_calendar`: allowed only through owner Google connection and channel
  policy.
- `create_thread`: owner-only; trusted owner channel may be fast-path allowed.
- `continue_thread`: owner-only; may be fast-path if project/thread is already
  bound to that channel.
- `stop_thread`: owner-only; if multiple targets match, clarify first.
- `send_email`: recipient must be authorized and policy-allowed.
- `x_import`: policy/cost checked before OAuth/network work.
- `dangerous_repo_write`: governed by Codex host approvals plus Arcwell policy.

Every controller action writes a policy decision record before provider
credentials or host mutation are used.

## Detailed Flows

### 1. "hows codex swift doing"

1. Telegram webhook captures message; local drain records `channel_messages`.
2. Controller verifies sender is authorized for project reads.
3. Router classifies `project_status` and extracts `codex swift`.
4. Resolver finds the `codex-swift` project by alias/path/name.
5. Controller refreshes live thread inventory from host adapter if available.
6. Controller reads:
   - latest `controller_threads` for project;
   - active `controller_runs`;
   - recent `work_runs` and `work_events`;
   - latest `project_status_snapshots`.
7. Summary builder returns:
   - project state;
   - active threads/runs;
   - recent completed work;
   - blockers/approvals;
   - freshness timestamps and sources.
8. Telegram delivery records an outbound message and receipt.

If live host refresh fails, reply with the latest known snapshot and say the
live host read failed with timestamp.

### 2. "Implement this feature in arcwell"

1. Router classifies `create_work_thread`.
2. Resolver finds `arcwell`.
3. Policy checks owner identity and thread-creation permission.
4. Controller creates a `controller_pending_action` with the source channel
   message id and requested Codex prompt.
5. The resident Codex host adapter marks the action `processing` and calls
   `codex_app.create_thread` with:
   - project cwd/path;
   - feature prompt;
   - owner/channel provenance;
   - expected status reporting instructions;
   - Arcwell MCP/tool profile.
6. The adapter writes `controller_threads`, `controller_runs`, controller
   events, and resolves the pending action.
7. Reply immediately: "Started Arcwell thread <title/id>; I will report status
   here. Use 'stop the arcwell work' to interrupt."
8. Event stream updates run status and records `controller_events`.

### 3. "hows it going"

1. Router uses `controller_channel_contexts` to resolve recent project/thread.
2. If the channel has active runs, show those first.
3. If no active context exists, list recent owner-visible active runs across all
   projects.
4. Summary includes:
   - running;
   - finished since last check;
   - blocked;
   - waiting for approval;
   - latest host event/activity;
   - stale/unreachable host warning if applicable.

### 4. "stop the blog work"

1. Router classifies `stop_work`.
2. Resolver searches projects, threads, and active runs for `blog`.
3. If multiple active matches exist, ask which one.
4. Policy checks owner identity and stop permission.
5. Controller marks `cancel_requested=1`.
6. Host adapter performs the strongest safe stop available. The current Codex
   app adapter sends a cooperative stop prompt with `send_message_to_thread` and
   records that limitation because no hard-stop thread API is exposed.
7. Controller records stop event and replies with exactly what stopped.

Stop must bypass ordinary busy queues. It is a control message.

### 5. "summarize my recent x bookmarks and mail me a report"

1. Router classifies `x_bookmark_report_email`.
2. Policy checks X import cost/network and email send permission.
3. Controller creates a new work thread/run or uses a local workflow runner if
   the user requested a simple report and no coding is needed.
4. Run steps:
   - `x_import_bookmarks` with recency/max limits;
   - build source cards/wiki entries from tweet bodies, authors, urls, metrics;
   - generate a cited report;
   - optionally run research/skeptic audit;
   - send email through `email_send_message`;
   - record delivery receipt.
5. Telegram receives progress and final delivery status.

If email recipient is not known/authorized, create a pending action and ask.

### 6. "whats on my schedule today"

1. Router classifies `calendar_today`.
2. Policy checks owner identity and read-calendar permission.
3. Google host adapter uses current Google Calendar connector if present.
4. Reply with date, timezone, events, conflicts, free blocks, and source
   freshness.
5. Do not store full private calendar text by default. Store only an optional
   work event that a calendar query happened, with redacted summary metadata.

## MCP And CLI Surface

Initial MCP tools:

- `controller_ingest_channel_message(message_id)`
- `controller_route_text(channel, sender, conversation_id, text)`
- `controller_list_projects(query?, status?)`
- `controller_project_status(project_ref, refresh_host?)`
- `controller_list_threads(project_ref?, host?, status?)`
- `controller_thread_status(thread_ref, refresh_host?)`
- `controller_activity(scope?, since?, limit?)`
- `controller_create_thread(project_ref, prompt, options?)`
- `controller_continue_thread(thread_ref, prompt, options?)`
- `controller_stop(ref, reason?)`
- `controller_run_workflow(kind, args, delivery?)`
- `controller_list_pending(channel?, sender?)`
- `controller_approve(pending_id)`
- `controller_cancel(pending_id)`

Initial CLI:

- `arcwell controller route --channel telegram --sender ... --text ...`
- `arcwell controller status <project-or-thread>`
- `arcwell controller threads [--project ...]`
- `arcwell controller create-thread <project> --prompt ...`
- `arcwell controller continue <thread> --prompt ...`
- `arcwell controller stop <ref>`
- `arcwell controller pending`
- `arcwell controller approve <id>`
- `arcwell controller cancel <id>`
- `arcwell controller sync-codex`

## Service Loop

The resident worker should run bounded loops:

- drain edge events;
- ingest channel messages into controller;
- retry outbox deliveries;
- refresh live host inventory for active projects/threads;
- poll active runs/events where streaming is unavailable;
- consolidate recent work into project snapshots;
- expire pending approvals;
- emit health/ops summaries.

Do not start endless loops inside an interactive Codex task. Install/run as the
Arcwell service.

## Ops UI Requirements

Ops must show:

- authorized channels and trust tier;
- latest controller messages;
- active threads/runs;
- pending approvals;
- stop/cancel state;
- outbox delivery receipts;
- host adapter health;
- stale project snapshots;
- recent errors.

Controls:

- refresh host state;
- retry outbox delivery;
- approve/cancel pending action;
- stop run/thread;
- dead-letter malformed controller message.

Mutating controls need auth, CSRF/idempotency, policy checks, and audit records.

## Build Plan

### Phase 0: Host Adapter Spike

Prove the resident path can control Codex:

- list threads;
- read thread;
- create thread;
- send follow-up;
- interrupt/stop;
- stream or poll run status.

Use Codex app-server/SDK if available. If only Codex Desktop tools are exposed,
record that as an interactive-only adapter and keep the resident controller
blocked for create/stop until app-server access is solved.

Validation:

- disposable thread created;
- follow-up sent;
- stop/interrupt observed;
- status synced into `controller_threads`.

### Phase 1: Controller Store And Resolver

Add controller tables, CLI, MCP, and resolver.

Validation:

- unit tests for ambiguous references, follow-up context, stale host data,
  unauthorized channel reads, and huge/prompt-injection bodies.

### Phase 2: Telegram Status MVP

Wire Telegram incoming message -> controller -> project/thread status -> reply.

Validation:

- exact user phrase live smoke: "hows codex swift doing";
- unauthorized sender denied;
- ambiguous project asks a clarification;
- live unavailable response is honest.

### Phase 3: Create/Continue/Stop Work

Wire `create_thread`, `continue_thread`, and `stop`.

Validation:

- Telegram "Implement this feature in arcwell" creates a Codex thread;
- "hows it going" reports the run;
- "stop the arcwell work" interrupts/cancels;
- duplicate Telegram updates do not create duplicate threads.

### Phase 4: Background Workflows

Wire X report + email delivery as the first multi-package workflow.

Validation:

- Telegram request imports bookmarks with bodies/stats/source provenance;
- report is generated with source links;
- email sends through authorized recipient;
- Telegram reports email delivery receipt.

### Phase 5: Calendar And Workspace Queries

Wire host Google Calendar read path for schedule questions.

Validation:

- owner Telegram request receives today's schedule;
- no full calendar content is stored by default;
- unavailable connector response is explicit and useful.

### Phase 6: Ops UI And Hardening

Add UI controls and exhaustive severe tests.

Validation:

- browser smoke desktop/mobile;
- policy/CSRF/idempotency tests;
- replay/duplicate/race tests;
- host adapter failure tests;
- delivery retry tests.

## Severe Test Matrix

- forged Telegram sender/chat ids;
- replayed Telegram update ids;
- prompt injection in Telegram, email, X, calendar text;
- duplicate "implement" message creates one thread;
- stop command races with a finishing run;
- stop command matches multiple projects and asks;
- unauthorized channel cannot read status or create/stop work;
- channel follow-up context expires and does not leak across senders;
- host adapter unavailable;
- host adapter returns malformed thread ids;
- host create succeeds but local persistence fails;
- local persistence succeeds but host create response is lost;
- email send authorized/unauthorized;
- X quota/token expiry;
- calendar connector unavailable/stale;
- outbox retry avoids duplicate delivery where provider result is uncertain;
- large transcript/status response is bounded and redacted.

## Non-Goals

- Do not build a second Codex UI.
- Do not put every Arcwell MCP tool into every controller thread.
- Do not store raw full private transcripts, mailboxes, or calendars by default.
- Do not let Telegram text bypass policy or host approvals.
- Do not claim always-on execution when only Cloudflare capture is online.

## Reviewed Sources

- OpenClaw docs: gateway, channel routing, session tools, channel plugin
  boundary, Codex harness, Codex thread/resume commands.
- OpenClaw local checkout: `/Users/chabotc/Projects/openclaw`.
- Hermes docs: gateway internals, agent loop, session storage, messaging,
  programmatic integration.
- Hermes local checkout: `/Users/chabotc/Projects/hermes-agent`.
- Arcwell current state: `packages/arcwell-projects`, `PLAN.md`, `TODO.md`,
  `STATUS.md`, `docs/arcwell-architecture-report.md`, `crates/arcwell-core`,
  and `crates/arcwell-cli`.
