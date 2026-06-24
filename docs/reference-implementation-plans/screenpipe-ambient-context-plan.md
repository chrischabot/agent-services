# screenpipe Ambient Context Plan

Date: 2026-06-24

Status: design and implementation plan only. No Arcwell feature is implemented
by this file.

Reference repo: https://github.com/screenpipe/screenpipe

Reference commit inspected: `ece9e43`

Local inspection path: `/tmp/arcwell-reference-repos/screenpipe`

## Claim Boundary

This plan can claim that screenpipe source code was inspected and that this file
maps selected architecture patterns into an Arcwell design.

This plan cannot claim that Arcwell records screens, indexes meetings, redacts
PII, or exposes ambient context today.

## Source And Code Inspected

- `crates/screenpipe-db/src/lib.rs`
- `crates/screenpipe-db/src/write_queue.rs`
- `crates/screenpipe-core/src/lib.rs`
- `crates/screenpipe-redact/src/lib.rs`
- `crates/screenpipe-sync/src/cursor.rs`
- `crates/screenpipe-sync/src/pipeline.rs`
- `packages/screenpipe-mcp/src/index.ts`

The clone required a Git LFS checkout workaround. That does not affect the
source-code conclusions here, but it means media assets were not used as proof.

## What screenpipe Does Well

screenpipe is a local-first ambient capture and search system. The pieces worth
copying are not "record everything." The valuable parts are:

- A write queue with observable health.
- Async redaction rather than blocking capture on every expensive privacy pass.
- Cursor persistence with atomic temp-file replacement.
- Bounded MCP tools that push agents to search first and request detail later.
- Namespaced tags such as `person:`, `project:`, and `topic:`.
- Clear separation between capture, storage, sync, MCP access, and redaction.

`WriteQueueHealth` is especially useful. It tracks consecutive fatal batches,
total fatal batches, pool reopens, persistent failure, degraded state, and last
success. It gives operators more than "the queue exists."

The redaction package is also worth studying. Text redaction uses a fast regex
prepass, optional model/enclave paths, and a hash cache. Image redaction uses
bounding boxes and solid redaction rather than a weak blur. This is the right
privacy posture for any Arcwell feature that might ingest user-visible context.

The sync cursor code is intentionally conservative. Missing, corrupt, or
unreadable cursor files reset to default with warnings, and save errors return
to the caller so a pipeline can avoid advancing after a failed write.

## Arcwell-Native Shape

Arcwell should not build a surveillance substrate. The transferable feature is
an opt-in ambient context ledger for work traces Arcwell already has or can
capture with explicit consent:

- terminal/session summaries
- active project/run metadata
- channel messages already authorized for Arcwell
- meeting/transcript imports when deliberately provided
- future window/app/title context only behind explicit local controls

Working name: `arcwell ambient`

The value is "recover what I was doing and connect it to source cards/wiki/runs"
not "record my whole machine."

## Proposed Data Model

- `ambient_sources`
  - `id`
  - `source_kind`
  - `display_name`
  - `capture_mode`
  - `privacy_mode`
  - `enabled`
  - `owner_scope`
  - `created_at`
  - `updated_at`

- `ambient_events`
  - `id`
  - `source_id`
  - `event_kind`
  - `event_time`
  - `project_id`
  - `thread_id`
  - `run_id`
  - `title`
  - `url`
  - `app_name`
  - `metadata_json`
  - `redaction_status`
  - `created_at`

- `ambient_text_chunks`
  - `id`
  - `event_id`
  - `chunk_index`
  - `text`
  - `redacted_text`
  - `hash`
  - `embedding_ref`
  - `fts_synced_at`

- `ambient_tags`
  - `id`
  - `event_id`
  - `tag`
  - `tag_kind`
  - `source`

- `ambient_redaction_runs`
  - `id`
  - `scope`
  - `status`
  - `items_scanned`
  - `items_redacted`
  - `error`
  - `created_at`
  - `completed_at`

- `ambient_capture_health`
  - `source_id`
  - `queue_depth`
  - `consecutive_failures`
  - `total_failures`
  - `degraded`
  - `last_success_at`
  - `last_error`

- `ambient_cursors`
  - `source_id`
  - `cursor_json`
  - `cursor_version`
  - `updated_at`

## CLI, MCP, Slash, And Ops Surfaces

CLI:

- `arcwell ambient status`
- `arcwell ambient sources`
- `arcwell ambient search --query <text> [--project <id>]`
- `arcwell ambient tag <event-id> <tag>`
- `arcwell ambient redact-run [--source <id>]`
- `arcwell ambient forget <event-id|source-id>`

MCP:

- `ambient_search`
- `ambient_event_context`
- `ambient_tags`
- `ambient_health`

Slash/plugin:

- `/ambient-search`
- `/ambient-status`

Ops:

- Queue depth, degraded state, last redaction run, unredacted count, and cursor
  status.

## Implementation Plan

1. Start with non-screen sources.
   - Project/run/thread summaries are already Arcwell-shaped.
   - Prove the pipeline without collecting new sensitive surfaces.

2. Add a write queue.
   - Batch writes.
   - Track health and degraded state.
   - Never call a source healthy only because the process is alive.

3. Add cursor safety.
   - Atomic save.
   - Corrupt cursor detection.
   - No cursor advance until accepted rows and indexes are durable.

4. Add redaction.
   - Regex prepass first.
   - Pluggable stronger redactor later.
   - Store original/raw content only where policy permits.
   - Remote/model calls must see redacted text unless a policy explicitly
     allows raw local-only processing.

5. Add search/detail split.
   - Search returns snippets and IDs.
   - Detail requires explicit ID lookup.
   - MCP tools must impose content limits.

6. Add source-card/wiki projection.
   - Ambient events can become source cards only through an explicit command or
     policy.
   - Generated summaries are never evidence without linked raw event IDs.

7. Add opt-in richer capture later.
   - App/window/title capture is a separate proof gate.
   - Screenshots or OCR should remain out of scope until privacy and deletion
     paths are production-data proven.

## Anti-Mirage Traps

- A background process is not proof that rows are durable.
- A capture row is not proof that search/indexing works.
- Redaction code is not proof that secrets never reach model prompts.
- A cursor file is not proof that cursor advancement is safe.
- MCP access is not safe unless it bounds detail retrieval.
- "Local-first" is not a privacy proof by itself.

## Proof Gates

- Missing: no ambient schema or ingest path.
- Scaffold: tables and commands exist with toy inserts.
- Partial: events write locally but no redaction/search/ops proof.
- Local Proof: queue, cursor, redaction, search, and deletion tests pass with
  deterministic fixtures.
- Production Data Proof: a controlled Arcwell home captures real authorized
  project/run/thread events, redacts them, indexes them, and retrieves them
  with linked provenance.
- Operational: degraded queue, failed redaction, stale cursor, disabled source,
  forget/delete, and ops visibility are proven.
- Done: all enabled capture families satisfy privacy, provenance, deletion,
  search, and ops proof gates.

## Severe Tests

- Queue batch fails halfway; accepted writes remain consistent and cursor does
  not advance.
- Pool/database reopen increments health and recovers.
- Persistent write failure marks degraded and appears in ops.
- Cursor file is missing, corrupt, partially written, and unwritable.
- Redaction catches API-key-like, email-like, token-like, and private-path-like
  strings before remote/model access.
- Prompt injection inside captured text is treated as text.
- Huge transcript is chunked and search result stays bounded.
- Forget/delete removes text, FTS rows, embeddings, and source-card projections.
- Two sources produce duplicate events; dedup preserves provenance.
- MCP detail call refuses unbounded raw dump.
- Tag namespace collision is handled consistently.

## First Slice

Implement ambient capture for Arcwell's own run/thread/project summaries only.
No screen recording, no OCR, no microphone. The first useful feature is:
searchable, redacted, source-linked local work context with honest ops health.

