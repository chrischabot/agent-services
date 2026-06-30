# OpenViking Context Filesystem Plan

Date: 2026-06-24

Status: design and implementation plan only. No Arcwell feature is implemented
by this file.

Reference repo: https://github.com/volcengine/OpenViking

Reference commit inspected: `61f0537`

Local inspection path: `/tmp/arcwell-reference-repos/OpenViking`

License note: OpenViking is AGPL. This plan borrows concepts only. Do not copy
source code into Arcwell without a license decision.

## Claim Boundary

This plan can claim that OpenViking source code was inspected and that the
useful context-layer, retrieval, virtual filesystem, and Codex-memory-plugin
ideas were mapped into an Arcwell-native design.

This plan cannot claim that Arcwell has a RAG filesystem, hierarchical context
summaries, or OpenViking-compatible memory.

## Source And Code Inspected

- `crates/ragfs/src/lib.rs`
- `crates/ragfs/src/core/filesystem.rs`
- `crates/ragfs/src/core/mountable.rs`
- `crates/ragfs/src/core/multibackend_wrapper/routing.rs`
- `crates/ragfs/src/multibackend/types.rs`
- `crates/ragfs/src/multibackend/mod.rs`
- `examples/codex-memory-plugin/DESIGN.md`
- `examples/codex-memory-plugin/scripts/auto-recall.mjs`
- `examples/codex-memory-plugin/scripts/session-start-commit.mjs`
- `docs/en/concepts/03-context-layers.md`
- `docs/en/concepts/07-retrieval.md`
- `docs/design/openclaw-agent-experience-memory-design.md`

## What OpenViking Does Well

OpenViking's RAGFS design treats memory/context as a mountable virtual
filesystem with pluggable backends. The implementation has a broad filesystem
trait, mount routing through a radix-trie-like mount map, backend wrappers for
encryption/cache/multi-write/stats, and multi-backend read routing.

The most transferable pieces are conceptual:

- Path-like access to heterogeneous context.
- Layered summaries:
  - L0 `.abstract.md` for tiny vector/filter summaries.
  - L1 `.overview.md` for navigation-level summaries.
  - L2 full detail.
- Directory relation metadata such as `.relations.json`.
- Retrieval as a staged process: intent, typed queries, hierarchical traversal,
  rerank, and evidence traces.
- Separate `find` and `search` semantics.
- A Codex plugin that auto-recalls context but wraps injected text in a
  distinct XML-ish envelope and strips that envelope from later capture to avoid
  self-pollution.
- Atomic state writes and session-derived paths.

The Codex memory plugin is especially valuable for Arcwell because it shows a
clear anti-pollution stance: recalled context is injected as recall context, not
allowed to become new user memory by accident.

## Arcwell-Native Shape

Arcwell should not port RAGFS. It already has durable shapes: source cards,
wiki pages, memory events, profiles, projects, radar/research runs, channel
deliveries, and costs.

The useful Arcwell feature is a virtual context namespace over existing durable
objects.

Working name: `arcwell context`

Example paths:

- `arcwell://projects/<id>/overview.md`
- `arcwell://source-cards/<id>/abstract.md`
- `arcwell://wiki/<page>/overview.md`
- `arcwell://runs/<id>/trace.json`
- `arcwell://memory/<profile>/<id>.md`
- `arcwell://research/<run-id>/stage/<stage>.json`

These are not necessarily real files. They are stable context URIs with read,
list, grep, search, and provenance traces.

## Proposed Data Model

- `context_nodes`
  - `id`
  - `uri`
  - `node_kind`
  - `backing_kind`
  - `backing_id`
  - `title`
  - `project_id`
  - `created_at`
  - `updated_at`

- `context_edges`
  - `id`
  - `from_node_id`
  - `to_node_id`
  - `edge_kind`
  - `weight`
  - `reason`
  - `created_at`

- `context_summaries`
  - `id`
  - `node_id`
  - `layer`
  - `summary_text`
  - `summary_model`
  - `source_hash`
  - `status`
  - `created_at`

- `context_retrieval_runs`
  - `id`
  - `query`
  - `intent_json`
  - `status`
  - `created_at`

- `context_retrieval_hits`
  - `run_id`
  - `node_id`
  - `rank`
  - `score`
  - `matched_layer`
  - `matched_text_hash`
  - `reason`

- `context_mounts`
  - `mount_key`
  - `uri_prefix`
  - `backing_kind`
  - `enabled`
  - `policy_json`

## CLI, MCP, Slash, And Ops Surfaces

CLI:

- `arcwell context list arcwell://projects/<id>/`
- `arcwell context read arcwell://source-cards/<id>/overview.md`
- `arcwell context grep <pattern> --mount source-cards`
- `arcwell context search <query> --project <id>`
- `arcwell context trace <retrieval-run-id>`
- `arcwell context summarize <uri> --layer l0|l1`

MCP:

- `context_list`
- `context_read`
- `context_grep`
- `context_search`
- `context_retrieval_trace`

Slash/plugin:

- `/context-search`
- `/context-read`

Ops:

- Summary coverage, stale summaries, retrieval errors, mount health, and
  indexing lag.

## Implementation Plan

1. Define URI grammar.
   - No raw filesystem path interpretation.
   - Parse and validate path segments.
   - Reject `..`, empty segments, alternate separators, and unknown mounts.

2. Build read-only adapters over existing objects.
   - Source cards first.
   - Wiki pages second.
   - Project/run traces third.

3. Add L0/L1 summaries.
   - L0 small abstract from deterministic extraction or model with validation.
   - L1 navigation summary.
   - Store source hash and regenerate when backing object changes.

4. Add list/read/grep.
   - Read returns content plus provenance.
   - Grep is bounded.
   - No mutation yet.

5. Add retrieval with traces.
   - Intent extraction is optional and validated.
   - Retrieval records every hit and reason.
   - Search result content is bounded.

6. Add anti-pollution envelope for recalled context.
   - Any context injected into an agent thread has a wrapper like
     `<arcwell-context source="recall">`.
   - Memory capture strips or marks that wrapper so recalled content is not
     re-ingested as a new user-authored fact.

7. Add write/mount support only after read/search proof.
   - Mutation through virtual context URIs is a later design problem.

## Anti-Mirage Traps

- A path string is not a filesystem.
- A summary is not evidence unless linked to source rows.
- A vector hit is not enough without provenance.
- Retrieval that injects text can poison future memory if not marked.
- A read-only context view should not silently mutate wiki/source-card state.
- Mounting local files needs separate path-policy proof.

## Proof Gates

- Missing: no context URI layer.
- Scaffold: URI parser and static read command exist.
- Partial: read/list work for one object type but no summaries/search.
- Local Proof: URI validation, read/list/grep, L0/L1 freshness, retrieval
  traces, and anti-pollution stripping are tested.
- Production Data Proof: real Arcwell source cards/wiki pages produce summaries
  and retrieval traces that cite backing rows.
- Operational: stale summary detection, reindex jobs, mount health, and ops
  visibility work.
- Done: all claimed mounts provide read/list/search/trace with provenance and
  anti-pollution proof.

## Severe Tests

- URI traversal attempts are rejected.
- Percent-encoded traversal is rejected.
- Unknown mount cannot fall back to local filesystem.
- Summary source hash changes after backing text changes.
- Corrupt summary row triggers regeneration, not silent use.
- Retrieval query containing prompt injection is stored as query data only.
- Recalled context wrapper is stripped before memory capture.
- Search returns bounded snippets, not whole private pages.
- Cross-project query cannot read objects outside the authorized scope.
- Concurrent summary regeneration produces one final summary.
- Invalid model output cannot create arbitrary context edges.

## First Slice

Implement read-only `arcwell context read/list/search` over source cards and
wiki pages, with L0/L1 summaries and retrieval traces. Do not add filesystem
mounting or local-file mutation in the first slice.

## 2026-06-30 Refresh: Current Arcwell Shape

Arcwell now has a larger context graph than the original plan assumed:

- source cards, wiki pages, X canonical rows/projections, knowledge events,
  clusters, reports, editorial decisions, entities, and relations;
- project status snapshots, controller threads/runs/events/actions, work runs,
  work events, proof packets, guard goals/reviews, channel messages, and
  delivery observations;
- search/report surfaces for source cards, X, radar, jobs, projects, and ops;
- an active proof ledger that can already model claims, evidence, artifacts,
  checks, and promotion status.

The OpenViking lesson should be stable context URIs and layered summaries over
existing Arcwell truth, not a new RAG filesystem. The proof ledger and
knowledge graph are now better backing stores than a virtual filesystem would
be.

## 2026-06-30 Anti-Mirage Development

Claim to build next:

> Arcwell can expose stable, provenance-preserving context URIs over existing
> source, wiki, knowledge, project, proof, guard, and delivery objects, with
> bounded read/search and anti-pollution markers for injected recall.

Refutations:

- A context URI can escape to local filesystem paths.
- A summary is returned without backing row IDs and source hashes.
- Recalled/generated context is later captured as user-authored memory.
- Cross-project/context-scope reads leak private evidence.
- Retrieval traces cite model prose rather than source cards/proof artifacts.

Revised implementation slices:

1. Define `arcwell://` URI grammar over current object IDs:
   source-card, wiki, X, knowledge-event, knowledge-cluster, project,
   controller-run, proof-packet, guard-review, delivery-attempt.
2. Add L0/L1 summaries only where source hashes and stale detection exist.
3. Add `context_read` and `context_search` as bounded views, not filesystem
   mutation.
4. Add retrieval traces that point to proof/source/knowledge rows.
5. Add anti-pollution wrappers for recalled context injected into agent prompts;
   memory capture must strip or label those wrappers.

Keep from OpenViking:

- L0/L1/L2 layering;
- `find` vs `search` distinction;
- retrieval traces;
- relation metadata;
- anti-pollution envelope discipline.

Do not copy:

- mountable backend abstraction before URI/read/search proof;
- multi-backend file routing where Arcwell has typed durable tables;
- AGPL code or filesystem mutation semantics.

Next proof gate:

- Local Proof: URI parser rejects traversal/unknown mounts, reads source-card
  and proof-packet contexts, records retrieval traces, and strips injected
  context from memory capture.
- Production Data Proof: a copied current home resolves a real question using
  source-card/wiki/knowledge/proof contexts and produces an inspectable
  retrieval trace without filesystem access.
