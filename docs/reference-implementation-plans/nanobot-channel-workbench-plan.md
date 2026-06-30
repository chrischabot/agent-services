# nanobot Channel Workbench Plan

Date: 2026-06-24

Status: design and implementation plan only. No Arcwell feature is implemented
by this file.

Reference repo: https://github.com/HKUDS/nanobot

Reference commit inspected: `c915e98`

Local inspection path: `/tmp/arcwell-reference-repos/nanobot`

## Claim Boundary

This plan can claim that nanobot source code was inspected and that its session,
channel, WebUI, sustained-goal, and workspace-policy ideas were mapped to an
Arcwell design.

This plan cannot claim that Arcwell has a nanobot-compatible UI, channel
workbench, or sustained-goal runtime.

## Source And Code Inspected

- `nanobot/session/manager.py`
- `nanobot/session/turn_continuation.py`
- `nanobot/session/goal_state.py`
- `nanobot/session/webui_turns.py`
- `nanobot/channels/base.py`
- `nanobot/channels/manager.py`
- `nanobot/gateway/service.py`
- `nanobot/security/workspace_policy.py`
- `nanobot/webui/session_automations.py`

## What nanobot Does Well

nanobot is valuable as a session and channel workbench reference. The code is
not just "chat UI around an agent." It contains a lot of careful session hygiene:

- JSONL sessions with metadata sidecars.
- Atomic metadata writes with temp replacement and optional fsync.
- Corrupt JSONL repair by skipping bad lines.
- Replay cleanup that strips assistant metadata artifacts, local image
  breadcrumbs, and tool-call echoes.
- Legal context slicing that avoids starting mid-turn or orphaning tool results.
- File-cap trimming/archive of old prefixes.
- Sustained goal state in metadata with internal continuation prompts that are
  not persisted as user input.
- WebUI event stream for run status, runtime events, titles, attachments, and
  automations.
- Channel abstraction with streaming deltas, reasoning deltas, file edit
  events, pairing code for unknown DMs, and per-channel policy checks.
- Channel manager duplicate suppression, retry backoff, progress/tool hints,
  and streaming coalescing.
- Workspace policy based on canonical path resolution and allowed roots/files.

The biggest Arcwell lesson is that session replay is a security and product
surface. If replay leaks agent metadata, local file breadcrumbs, or tool echoes
back into the next prompt, the agent learns the wrong thing.

## Arcwell-Native Shape

Arcwell already has channels, memory, controller work, worker jobs, and Codex
plugin surfaces. It needs a coherent channel workbench that can show and operate
sessions across transports without turning each transport into its own product.

Working name: `arcwell channel-workbench`

Core capabilities:

- Persistent controller sessions with legal replay windows.
- Runtime event stream for runs, tools, reasoning, file edits, and delivery.
- Channel abstraction for Telegram/email/Codex/WebUI/future chat transports.
- Pairing and approval for unknown senders.
- Sustained goal metadata for long-running work.
- Session-attached automations with origin preview and audit.

## Proposed Data Model

- `controller_sessions`
  - `id`
  - `project_id`
  - `channel_kind`
  - `channel_thread_ref`
  - `status`
  - `title`
  - `goal_state_json`
  - `last_consolidated_seq`
  - `created_at`
  - `updated_at`

- `session_messages`
  - `id`
  - `session_id`
  - `seq`
  - `role`
  - `content`
  - `content_kind`
  - `metadata_json`
  - `created_at`

- `session_runtime_events`
  - `id`
  - `session_id`
  - `run_id`
  - `event_kind`
  - `payload_json`
  - `created_at`

- `channel_streams`
  - `id`
  - `session_id`
  - `stream_kind`
  - `last_delta_seq`
  - `coalesced_text`
  - `status`

- `channel_pairing_requests`
  - `id`
  - `channel_kind`
  - `sender_ref`
  - `pairing_code_hash`
  - `status`
  - `expires_at`

- `session_automations`
  - `id`
  - `session_id`
  - `origin_preview`
  - `schedule_json`
  - `status`
  - `created_at`

## CLI, MCP, Slash, And Ops Surfaces

CLI:

- `arcwell session list`
- `arcwell session read <id>`
- `arcwell session replay <id> --budget <tokens>`
- `arcwell session repair <id>`
- `arcwell channel pair <code>`
- `arcwell channel streams <session-id>`
- `arcwell goal continue <session-id>`

MCP:

- `session_list`
- `session_read`
- `session_replay_window`
- `channel_pairing_status`
- `session_runtime_events`

Slash/plugin:

- `/session-read`
- `/channel-list`
- `/channel-record`

Ops:

- Active sessions, corrupt sessions repaired/skipped, paired/unpaired senders,
  streaming lag, delivery failures, sustained-goal continuation count.

## Implementation Plan

1. Define session record and message invariants.
   - Monotonic sequence.
   - Durable role/content/metadata split.
   - Replay view is derived, not raw dump.

2. Add replay sanitizer.
   - Strip tool echoes and assistant artifacts.
   - Preserve deliberate media breadcrumbs as attachments, not prompt text.
   - Avoid timestamps on assistant turns that would train the wrong prefix.

3. Add legal slicing.
   - Never start with orphan tool output.
   - Prefer nearest user turn when token budget is tight.
   - Reset bad `last_consolidated` to a safe value.

4. Add runtime event bus.
   - Tool start/end.
   - Reasoning delta/end.
   - File edit event.
   - Progress hints.
   - Run status.

5. Add channel adapter contract.
   - Send, delta, reasoning, file edits, pairing, allowed-sender checks.
   - Existing Telegram/email surfaces should implement the same contract.

6. Add sustained goal state.
   - Hidden continuation prompt is internal metadata.
   - Continuation cannot be persisted as a user-authored turn.
   - Enforce max rounds and budget.

7. Add session automations.
   - Automation creation records origin preview.
   - User-visible audit trail shows who/what created it.

## Anti-Mirage Traps

- A JSONL file is not a replay-safe session.
- A channel send method is not streaming support.
- A WebUI route is not a channel workbench unless runtime events are real.
- A sustained-goal prompt is unsafe if it is persisted as user input.
- Pairing code text is not authorization unless it binds sender and expires.
- Workspace policy must be enforced at path resolution, not only UI display.

## Proof Gates

- Missing: no unified session/channel model.
- Scaffold: session tables and read command exist.
- Partial: sessions store messages but replay sanitation or channels absent.
- Local Proof: corruption repair, legal slicing, sanitizer, pairing, duplicate
  suppression, event ordering, and workspace policy tests pass.
- Production Data Proof: a real authorized channel/session records messages,
  runtime events, and a replay window without leaking internal artifacts.
- Operational: ops shows broken channels, unpaired senders, stream lag,
  corrupt-session repair, and continuation limits.
- Done: every claimed channel supports the contract at its stated level with
  replay, pairing, events, delivery, and recovery proof.

## Severe Tests

- Corrupt JSONL line is skipped and preserved in repair report.
- Metadata sidecar is corrupt; session opens with safe defaults.
- Replay cannot start with orphan tool result.
- Assistant tool-call echo is not reinserted into the next prompt.
- Local image breadcrumb becomes attachment metadata, not raw prompt text.
- Hidden continuation is not persisted as user input.
- Continuation stops at max rounds.
- Duplicate inbound message is suppressed only within the correct source scope.
- Streaming deltas arrive out of order; final coalesced stream is correct or
  marked invalid.
- Unknown DM requires pairing and cannot send commands.
- Path traversal and symlink escape are blocked by workspace policy.

## First Slice

Add a read-only session replay/sanitizer over the existing Arcwell channel and
run records. The first UI/workbench behavior should be inspectable session
history and runtime events before any new always-on channel automation.

## 2026-06-30 Refresh: Current Arcwell Shape

Arcwell now has more than a raw channel ledger:

- `channel_messages`, channel authorizations, project bindings,
  `channel_delivery_attempts`, and `channel_delivery_observations` exist.
- The controller design has Phase 0 local ledger/router work; Telegram drain
  routes recorded messages through the controller, and Codex host-adapter skill
  work can consume pending actions.
- Project status snapshots, work runs/events, proof packets, guard
  goals/reviews, and ops UI summaries provide the raw material for a session
  workbench.
- Email verification gaps and delivery observations already show why replay
  needs explicit source/observation labels.

The nanobot lesson should now become "replay hygiene and runtime event
ergonomics over current Arcwell sessions," not a second chat/session store.

## 2026-06-30 Anti-Mirage Development

Claim to build next:

> Arcwell can present a replay-safe, source-labeled session view across channel
> messages, controller events, work runs, guard reviews, proof packets, and
> delivery observations without feeding internal artifacts back as user text.

Refutations:

- Replay starts with orphan tool/delivery output.
- Generated summaries, proof checks, or guard reviews appear as user-authored
  messages.
- Delivery provider acceptance is displayed as recipient observation.
- Unknown channel senders can trigger project writes without authorization.
- Runtime event ordering changes the meaning of a session.

Revised implementation slices:

1. Add a session/read-model over existing channel/controller/work/proof rows.
2. Sanitize replay by role and origin: user text, assistant output, source
   evidence, generated summary, guard review, proof check, delivery observation.
3. Add event ordering and legal-slice tests before adding any interactive
   workbench UI.
4. Extend ops UI detail views to show a compact session timeline.
5. Add pairing/authorization status to session views rather than creating a
   separate pairing subsystem.

Keep from nanobot:

- legal context slicing;
- artifact stripping on replay;
- runtime event bus vocabulary;
- streaming/reasoning/file-edit event types;
- bounded sustained-goal continuation state.

Do not copy:

- JSONL as a second Arcwell session truth;
- WebUI-first design before CLI/MCP/ops read models are stable;
- hidden continuation prompts that bypass Arcwell proof/guard boundaries.

Next proof gate:

- Local Proof: a fixture timeline with channel messages, controller events,
  proof packets, guard reviews, and delivery observations renders in legal
  order with no internal artifact re-ingestion.
- Production Data Proof: one real authorized channel/controller session is
  replayed from durable rows and shows exact delivery-observation status.
