# Arcwell X Architecture And Implementation Plan

Date: 2026-06-22

Related note: [Birdclaw Lessons For Arcwell X](./birdclaw-x-upgrade-plan.md)

## Objective

Turn Arcwell X from a source-card-oriented importer into a local-first social
intelligence substrate:

- canonical local X/Twitter memory across tweets, profiles, collections, watch
  observations, timelines, mentions, threads, links, media, follow graph, and
  eventually DMs/moderation
- archive-first historical import plus live sync into the same canonical model
- durable source-card/wiki/research/digest projections from canonical rows
- fast local search through FTS5
- policy, cost, secret, provenance, and prompt-injection boundaries preserved
  or strengthened
- narrow, reliable CLI/MCP surfaces for agents
- local review/ops lanes for watch health, digest candidates, research packs,
  credential health, and sync status

This plan borrows Birdclaw's product and data architecture while keeping
Arcwell's existing advantages: source cards, wiki evidence, severe tests,
policy/cost gates, redacted secret handling, and ops visibility.

## Current State

Arcwell X already has useful working pieces:

- `x_items`: imported tweet-shaped rows with text, author, URL, metrics, raw
  payload, source-card id, and wiki-page id
- `x_item_sources`: provenance rows keyed by `x_id`, `source_kind`, and
  `source_detail`
- `x import-json`: replay/export fixture import
- `x recent-search`: live X API v2 recent search with cursor state
- `x import-bookmarks`: authenticated bookmark import with body/metrics/source
  provenance
- `x rebuild-definitive-watch-sources`: bookmark authors plus recent follows
  as the normal monitor seed
- `x monitor-watch-sources`: active watch-source polling into X items, source
  cards, wiki pages, and digest candidates
- OAuth URL/exchange/refresh helpers storing only local secret values
- policy and cost checks before network/provider work
- source health and cursor inspection
- severe tests for token expiry, refresh failure redaction, quota behavior,
  partial/malformed X responses, duplicate cursor pages, unsafe URLs, and
  prompt-injection-as-evidence

The main limitation is architectural: the local truth is still `x_items`, a
single evidence table. It cannot cleanly represent profiles, collections,
account-scoped observations, thread relationships, timeline/mention edges,
profile history, URL/media/link indexes, follow graph churn, DMs, or moderation.

## Target Architecture

```text
Twitter/X archive zip
X API v2 user context
optional xurl/bird adapters later
manual/replay JSON
        |
        v
transport adapters and archive readers
        |
        v
normalized mappers
        |
        v
canonical X write pipeline
        |
        +--> x_accounts
        +--> x_profiles / snapshots / bio entities
        +--> x_tweets / tweet refs / tweet edges
        +--> x_collections
        +--> x_urls / link occurrences
        +--> x_media
        +--> x_follow_snapshots / edges / events
        +--> x_dms later
        +--> x_scores overlays
        +--> FTS indexes
        |
        v
repairable projections
        |
        +--> source cards
        +--> wiki pages
        +--> digest candidates
        +--> research briefs
        +--> ops snapshots
        +--> portable JSONL export
        |
        v
CLI / MCP / local ops UI / deep research / delivery
```

Core rule:

> Raw provider/archive payloads are retained as evidence and debugging context,
> but all user-facing search, reports, research, digest, and UI lanes read from
> canonical X tables.

## Design Principles

1. Normalize first, project second.
   Current source cards are valuable, but source-card rows should be a
   projection from canonical X records, not the only primary record.

2. Preserve current user-facing behavior during migration.
   `x list`, `x bookmarks`, `x report`, MCP `x_list`, and MCP `x_report` should
   keep working while the backend moves under them.

3. Treat every X field as untrusted external data.
   Tweet text, profile descriptions, display names, URLs, media metadata, and
   archive payloads are evidence only. They never become instructions.

4. Make live network use optional, policy-gated, and cache-aware.
   Archive import should be the bulk path. Live sync should fill gaps and stay
   cursor/rate-limit aware.

5. Keep model judgment as an overlay.
   Interestingness, actionability, spam/low-signal, digest ranking, and
   identity confidence are separate scored rows with model/cost/provenance, not
   mutations of canonical rows.

6. Add agent tools sparingly.
   Arcwell already has a large MCP surface. Prefer a few task-level tools over
   mirroring every CLI subcommand.

7. Never imply live completeness without proof.
   Archive-derived, live-synced, cache-derived, and projected evidence must be
   visible as distinct provenance in reports and ops.

## Proposed Schema

Names are `x_*` to avoid collisions with existing Arcwell memory/wiki/channel
tables.

### Accounts

`x_accounts`

- `id TEXT PRIMARY KEY`
- `x_user_id TEXT UNIQUE`
- `handle TEXT NOT NULL`
- `display_name TEXT NOT NULL DEFAULT ''`
- `profile_id TEXT`
- `is_default INTEGER NOT NULL DEFAULT 0`
- `preferred_transport TEXT NOT NULL DEFAULT 'x_api'`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

Purpose:

- account-scoped collections, mentions, timelines, follows, DMs, blocks, and
  sync cursors
- future multi-account support

Initial migration:

- create a synthetic `acct_default` when importing rows without a known account
- map current `x_items` to `acct_default` edges with source provenance

### Profiles

`x_profiles`

- `id TEXT PRIMARY KEY`
- `x_user_id TEXT UNIQUE`
- `handle TEXT NOT NULL`
- `display_name TEXT NOT NULL DEFAULT ''`
- `description TEXT NOT NULL DEFAULT ''`
- `location TEXT`
- `url TEXT`
- `profile_image_url TEXT`
- `verified INTEGER`
- `verified_type TEXT`
- `followers_count INTEGER`
- `following_count INTEGER`
- `tweet_count INTEGER`
- `listed_count INTEGER`
- `public_metrics_json TEXT NOT NULL DEFAULT '{}'`
- `entities_json TEXT NOT NULL DEFAULT '{}'`
- `raw_json TEXT NOT NULL DEFAULT '{}'`
- `first_seen_at TEXT NOT NULL`
- `last_seen_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

Indexes:

- unique normalized handle where practical
- follower count descending
- last seen descending

`x_profile_snapshots`

- `profile_id TEXT NOT NULL`
- `snapshot_hash TEXT NOT NULL`
- `observed_at TEXT NOT NULL`
- `last_seen_at TEXT NOT NULL`
- `source TEXT NOT NULL`
- identity/count/entity fields copied from profile
- `raw_json TEXT NOT NULL DEFAULT '{}'`
- primary key `(profile_id, snapshot_hash)`

`x_profile_entities`

- `profile_id TEXT NOT NULL`
- `kind TEXT NOT NULL`
- `value TEXT NOT NULL`
- `normalized_value TEXT NOT NULL`
- `source TEXT NOT NULL`
- `weight INTEGER NOT NULL DEFAULT 1`
- `is_active INTEGER NOT NULL DEFAULT 1`
- `first_seen_at TEXT NOT NULL`
- `last_seen_at TEXT NOT NULL`
- primary key `(profile_id, kind, value, source)`

Purpose:

- identity lookup such as "the Blacksmith person"
- profile history and current-vs-former affiliation context
- DMs and mentions can show profile context without raw payload spelunking

### Tweets

`x_tweets`

- `id TEXT PRIMARY KEY`
- `x_id TEXT NOT NULL UNIQUE`
- `author_profile_id TEXT`
- `text TEXT NOT NULL`
- `created_at TEXT`
- `lang TEXT`
- `conversation_id TEXT`
- `reply_to_x_id TEXT`
- `quote_x_id TEXT`
- `retweet_x_id TEXT`
- `possibly_sensitive INTEGER`
- `like_count INTEGER`
- `reply_count INTEGER`
- `repost_count INTEGER`
- `quote_count INTEGER`
- `bookmark_count INTEGER`
- `impression_count INTEGER`
- `metrics_json TEXT NOT NULL DEFAULT '{}'`
- `entities_json TEXT NOT NULL DEFAULT '{}'`
- `raw_json TEXT NOT NULL DEFAULT '{}'`
- `first_seen_at TEXT NOT NULL`
- `last_seen_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

Indexes:

- `x_id`
- `author_profile_id, created_at DESC`
- `created_at DESC`
- `conversation_id, created_at ASC`

`x_tweet_refs`

- `tweet_x_id TEXT NOT NULL`
- `ref_kind TEXT NOT NULL` such as `reply_to`, `quote`, `retweet`,
  `conversation_root`, `parent_walk`
- `ref_x_id TEXT NOT NULL`
- `source TEXT NOT NULL`
- `created_at TEXT NOT NULL`
- primary key `(tweet_x_id, ref_kind, ref_x_id, source)`

Purpose:

- thread reconstruction
- quoted tweet expansion
- explicit missing-parent tracking

### Account And Source Edges

`x_tweet_edges`

- `account_id TEXT NOT NULL`
- `tweet_x_id TEXT NOT NULL`
- `edge_kind TEXT NOT NULL`
- `source_kind TEXT NOT NULL`
- `source_detail TEXT`
- `transport TEXT NOT NULL`
- `first_seen_at TEXT NOT NULL`
- `last_seen_at TEXT NOT NULL`
- `seen_count INTEGER NOT NULL DEFAULT 1`
- `cursor_key TEXT`
- `raw_json TEXT NOT NULL DEFAULT '{}'`
- primary key `(account_id, tweet_x_id, edge_kind, source_kind, source_detail)`

Allowed initial `edge_kind` values:

- `json_import`
- `recent_search`
- `bookmark`
- `watch`
- `mention`
- `timeline`
- `authored`
- `archive`

This is the Birdclaw-style upgrade to `x_item_sources`: a tweet is canonical,
and each account/source observation is an edge.

`x_collections`

- `account_id TEXT NOT NULL`
- `tweet_x_id TEXT NOT NULL`
- `collection_kind TEXT NOT NULL` such as `bookmark` or `like`
- `collected_at TEXT`
- `source TEXT NOT NULL`
- `first_seen_at TEXT NOT NULL`
- `last_seen_at TEXT NOT NULL`
- `raw_json TEXT NOT NULL DEFAULT '{}'`
- primary key `(account_id, tweet_x_id, collection_kind)`

Purpose:

- bookmarks and likes as durable account-scoped collections
- research starts here

### Search

`x_tweets_fts`

- FTS5 with `x_id UNINDEXED`, `author_handle`, `text`, `url_text`

Update strategy:

- transactional update when inserting/updating canonical tweet rows
- `arcwell x rebuild-fts` repair command
- migration backfill from existing `x_items`

Later:

- `x_dms_fts`
- profile/entity search index if FTS5 over profile history is useful

### URL And Link Index

`x_urls`

- `url TEXT PRIMARY KEY`
- `expanded_url TEXT`
- `final_url TEXT`
- `display_url TEXT`
- `title TEXT`
- `description TEXT`
- `image_url TEXT`
- `site_name TEXT`
- `status TEXT NOT NULL`
- `error TEXT`
- `provider TEXT NOT NULL`
- `retrieved_at TEXT`
- `updated_at TEXT NOT NULL`

`x_link_occurrences`

- `source_kind TEXT NOT NULL` such as `tweet`, `dm`, `profile`
- `source_id TEXT NOT NULL`
- `position INTEGER NOT NULL`
- `url TEXT NOT NULL`
- `tweet_x_id TEXT`
- `profile_id TEXT`
- `account_id TEXT`
- `created_at TEXT`
- primary key `(source_kind, source_id, position, url)`

Safety:

- reuse existing URL ingestion SSRF/content-type/size rules
- do not fetch URLs from X text unless a command explicitly requests expansion
  and policy allows it

### Media

`x_media`

- `media_key TEXT PRIMARY KEY`
- `tweet_x_id TEXT`
- `media_type TEXT NOT NULL`
- `url TEXT`
- `preview_image_url TEXT`
- `alt_text TEXT`
- `width INTEGER`
- `height INTEGER`
- `duration_ms INTEGER`
- `variants_json TEXT NOT NULL DEFAULT '[]'`
- `local_original_path TEXT`
- `local_thumbnail_path TEXT`
- `source TEXT NOT NULL`
- `raw_json TEXT NOT NULL DEFAULT '{}'`
- `first_seen_at TEXT NOT NULL`
- `last_seen_at TEXT NOT NULL`

Initial scope:

- metadata only
- archive-extracted media bytes later
- live media fetch even later, with explicit size and pacing controls

### Follow Graph

`x_follow_snapshots`

- `id TEXT PRIMARY KEY`
- `account_id TEXT NOT NULL`
- `direction TEXT NOT NULL` such as `followers` or `following`
- `source TEXT NOT NULL`
- `status TEXT NOT NULL` such as `complete`, `partial`, `dry_run`, `failed`
- `page_count INTEGER NOT NULL DEFAULT 0`
- `result_count INTEGER NOT NULL DEFAULT 0`
- `started_at TEXT NOT NULL`
- `completed_at TEXT`
- `raw_meta_json TEXT NOT NULL DEFAULT '{}'`

`x_follow_snapshot_members`

- `snapshot_id TEXT NOT NULL`
- `profile_id TEXT NOT NULL`
- `x_user_id TEXT`
- `position INTEGER NOT NULL`
- primary key `(snapshot_id, profile_id)`

`x_follow_edges`

- `account_id TEXT NOT NULL`
- `direction TEXT NOT NULL`
- `profile_id TEXT NOT NULL`
- `x_user_id TEXT`
- `source TEXT NOT NULL`
- `current INTEGER NOT NULL DEFAULT 1`
- `first_seen_at TEXT NOT NULL`
- `last_seen_at TEXT NOT NULL`
- `ended_at TEXT`
- `updated_at TEXT NOT NULL`
- primary key `(account_id, direction, profile_id)`

`x_follow_events`

- `id TEXT PRIMARY KEY`
- `account_id TEXT NOT NULL`
- `direction TEXT NOT NULL`
- `profile_id TEXT NOT NULL`
- `kind TEXT NOT NULL` such as `started` or `ended`
- `event_at TEXT NOT NULL`
- `snapshot_id TEXT NOT NULL`

Initial scope:

- archive import and limited OAuth following reads
- no full following import as default watch list
- keep current definitive watch rebuild behavior as the normal seed

### Sync Runs, Cache, And Cursors

Reuse existing `cursors` for compatibility, but add typed sync rows:

`x_sync_runs`

- `id TEXT PRIMARY KEY`
- `account_id TEXT`
- `stream TEXT NOT NULL`
- `transport TEXT NOT NULL`
- `status TEXT NOT NULL`
- `started_at TEXT NOT NULL`
- `completed_at TEXT`
- `seen INTEGER NOT NULL DEFAULT 0`
- `inserted INTEGER NOT NULL DEFAULT 0`
- `updated INTEGER NOT NULL DEFAULT 0`
- `skipped_duplicates INTEGER NOT NULL DEFAULT 0`
- `rejected INTEGER NOT NULL DEFAULT 0`
- `page_count INTEGER NOT NULL DEFAULT 0`
- `cursor_key TEXT`
- `previous_cursor TEXT`
- `new_cursor TEXT`
- `saturation_reason TEXT`
- `cost_decision_id TEXT`
- `source_health_key TEXT`
- `error TEXT`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`

`x_sync_cache`

- `cache_key TEXT PRIMARY KEY`
- `transport TEXT NOT NULL`
- `surface TEXT NOT NULL`
- `value_json TEXT NOT NULL`
- `created_at TEXT NOT NULL`
- `expires_at TEXT`

Purpose:

- ops status
- live-cost control
- early-stop behavior
- resumability

### Scoring Overlays

`x_scores`

- `entity_kind TEXT NOT NULL` such as `tweet`, `thread`, `profile`, `dm`,
  `source_candidate`
- `entity_id TEXT NOT NULL`
- `score_kind TEXT NOT NULL` such as `interestingness`, `actionability`,
  `low_signal`, `digest_priority`
- `score REAL NOT NULL`
- `label TEXT`
- `reason TEXT NOT NULL`
- `model TEXT`
- `prompt_version TEXT`
- `cost_decision_id TEXT`
- `source_card_id TEXT`
- `scored_at TEXT NOT NULL`
- `expires_at TEXT`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`
- primary key `(entity_kind, entity_id, score_kind, prompt_version)`

No score row should cause outbound delivery by itself. Delivery remains a
separate policy/cost/authorization decision.

### Compatibility Projection

Keep `x_items` during migration.

Phase 1 compatibility options:

1. dual-write `x_items` and canonical rows
2. keep `x_items` as the source-card projection table
3. later replace reads with a view-like query over canonical rows plus
   projection metadata

Recommended:

- Phase 1: dual-write, keep existing table and tests green
- Phase 2: move read APIs to canonical queries while still populating `x_items`
- Phase 3: mark `x_items` as compatibility/projection storage in docs
- only remove it after all CLI/MCP/docs/tests have migrated and backups know
  how to export canonical X rows

## Core Write Pipeline

Introduce a single canonical upsert path:

```text
XRawInput
  -> XNormalizedBatch
  -> upsert profiles
  -> upsert tweets
  -> upsert refs/media/urls
  -> upsert account/source edges
  -> upsert collections
  -> update FTS
  -> write x_sync_run/source_health/cursor
  -> optionally project source cards/wiki/digest candidates
```

Suggested internal types:

- `XNormalizedProfile`
- `XNormalizedTweet`
- `XNormalizedTweetRef`
- `XNormalizedMedia`
- `XNormalizedUrl`
- `XObservation`
- `XCollectionMembership`
- `XCanonicalWriteInput`
- `XCanonicalWriteReport`
- `XProjectionRequest`
- `XProjectionReport`

Root-cause rule for imports:

- provider/archive parse failures should reject the specific item or fail the
  run before cursor advancement, depending on whether the provider response is
  partial or structurally untrustworthy
- cursor advancement happens only after canonical rows and required projections
  are durable
- source-card projection failure must not leave the canonical row invisible; it
  should create repairable projection state

## Projection Architecture

Current `insert_x_item` creates source cards immediately. That couples canonical
storage and projection.

Target:

1. canonical write succeeds
2. projection request is recorded
3. source-card/wiki projection runs transactionally where possible
4. projection metadata links back to canonical ids
5. repair command can recreate missing projections

Add:

`x_projections`

- `id TEXT PRIMARY KEY`
- `entity_kind TEXT NOT NULL`
- `entity_id TEXT NOT NULL`
- `projection_kind TEXT NOT NULL` such as `source_card`, `wiki_page`,
  `digest_candidate`, `research_brief`
- `status TEXT NOT NULL`
- `source_card_id TEXT`
- `wiki_page_id TEXT`
- `digest_candidate_id TEXT`
- `last_error TEXT`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`
- unique `(entity_kind, entity_id, projection_kind)`

Commands:

- `arcwell x repair-projections`
- `arcwell x project-source-cards --since <iso>`

MCP:

- do not expose repair initially unless agents have a real workflow need
- ops UI can show projection failures

## CLI Surface Plan

### Preserve Existing Commands

Keep these stable:

```text
arcwell x import-json <path>
arcwell x recent-search <query> --max-results N
arcwell x enqueue-recent-search <query> --max-results N
arcwell x import-bookmarks --bookmark-days N --max-bookmarks N
arcwell x import-following-watch-sources --max-users N
arcwell x rebuild-definitive-watch-sources ...
arcwell x monitor-watch-sources ...
arcwell x oauth-url ...
arcwell x oauth-exchange ...
arcwell x oauth-refresh ...
arcwell x list ...
arcwell x bookmarks ...
arcwell x report ...
```

Backend behavior can move to canonical writes while output envelopes remain
compatible.

### Add In Order

Phase 1:

```text
arcwell x rebuild-fts
arcwell x stats
arcwell x search-tweets <query>
```

Phase 2:

```text
arcwell x sync-bookmarks --bookmark-days N --max-pages N --early-stop --refresh
arcwell x sync-likes --max-pages N --early-stop --refresh
```

`import-bookmarks` can remain as an alias or narrow import command. The
Birdclaw-like sync naming should become the normal mental model for paged live
surfaces.

Phase 3:

```text
arcwell x import-archive [path] --select tweets,likes,bookmarks,profiles,followers,following,dms,media
arcwell x discover-archives
```

Phase 4:

```text
arcwell x research <query> --bookmarks --watch --thread-depth N --out PATH
arcwell x expand-thread <tweet-id> --mode local|auto
arcwell x links search <query>
arcwell x links backfill --limit N --dry-run
```

Phase 5:

```text
arcwell x graph summary
arcwell x graph events
arcwell x export-portable --out DIR
arcwell x validate-portable DIR
```

Phase 6:

```text
arcwell x score-candidates --kind interestingness --limit N
arcwell x digest [today|24h|week]
```

Later, only after approval UX:

```text
arcwell x mute <handle-or-id>
arcwell x block <handle-or-id>
arcwell x compose reply <tweet-id>
```

These are deliberately late because social writes need stronger confirmation,
authorization, audit, and rollback semantics.

## MCP Surface Plan

Keep existing tools working, but do not mirror every new command.

Current tools to preserve:

- `x_import_json_file`
- `x_recent_search`
- `x_enqueue_recent_search`
- `x_import_bookmarks`
- `x_import_following_watch_sources`
- `x_rebuild_definitive_watch_sources`
- `x_monitor_watch_sources`
- `x_oauth_authorize_url`
- `x_oauth_exchange_code`
- `x_oauth_refresh`
- `x_list`
- `x_bookmarks`
- `x_report`

New task-level tools:

- `x_search_tweets`
- `x_import_archive`
- `x_research_brief`
- `x_sync_bookmarks`
- `x_source_health`
- `x_digest_candidates`
- `x_export_portable`

Avoid tools for:

- every graph query
- every repair operation
- write actions such as block/mute/reply until approval UX is proven
- raw secret retrieval

Resource additions:

- `arcwell://x-tweets`
- `arcwell://x-profiles`
- `arcwell://x-sync-runs`
- `arcwell://x-source-health`
- keep `arcwell://x-items` as compatibility

## Module Boundaries

The current Rust core is concentrated in `crates/arcwell-core/src/lib.rs`.
Avoid a disruptive file split as the first step, but introduce boundaries in
small increments.

Recommended eventual layout:

```text
crates/arcwell-core/src/x/
  mod.rs
  schema.rs
  canonical.rs
  projection.rs
  search.rs
  archive.rs
  api.rs
  sync.rs
  research.rs
  export.rs
  scoring.rs
  tests.rs
```

Transition strategy:

1. add internal structs and functions near existing X code
2. move cohesive groups only after tests cover the new behavior
3. keep public `Store` methods as the stable internal API for CLI/MCP
4. avoid a big mechanical move in the same change as schema migration

## Archive Import Plan

### Discovery

Add macOS-friendly discovery:

- explicit path always wins
- search `~/Downloads` for likely `twitter-*.zip`, `x-*.zip`,
  `*archive*.zip`
- optional Spotlight `mdfind` probe on macOS
- report candidates without importing when ambiguous

### Reader

Archive reader must handle:

- JavaScript wrapper files such as `window.YTD.tweets.part0 = [...]`
- JSON arrays
- multiple split tweet files
- note tweets
- likes
- bookmarks
- account/profile files
- follower/following files
- direct message files later
- media paths later

Safety:

- reject path traversal
- cap file count and total uncompressed bytes
- reject nested archive recursion
- never execute JS
- preserve parse errors with file names but not huge payload dumps

### Apply

Archive import writes canonical rows only through the normal write pipeline:

- account identity
- local profile
- authored tweets
- likes/bookmarks as collections
- profiles from available account/user metadata
- follows/following snapshots
- DMs later behind explicit retention choice
- media metadata and optional extracted bytes

Selected re-import rules:

- unselected slices are preserved
- selected slices are idempotent
- if an existing account identity conflicts with archive identity, fail before
  writing
- partial import reports exactly which slices were applied

## Live Sync Plan

### Transport Priority

Short term:

1. archive
2. X API v2 user context
3. manual/replay JSON

Later:

4. optional `xurl` adapter
5. optional `bird` adapter

If `xurl` or `bird` are added, shell out through adapter seams. Do not make
their config/storage the Arcwell truth model.

### Sync Semantics

Every live sync should:

- create `x_sync_runs`
- run policy before credential lookup/network
- reserve estimated cost before network
- retrieve current token without printing it
- read previous cursor
- fetch page(s)
- map to canonical batch
- write canonical rows and projections
- advance cursor only after durable write/projection
- record source health
- release budget on classified quota/auth failures
- emit a stable JSON report

### Early Stop

For paged bookmarks, likes, timeline, and follows:

- `--early-stop` stops when a fetched page creates no new canonical tweet,
  collection, profile, or edge rows
- `--max-pages` caps work
- `--refresh` bypasses cache
- report includes `saturation_reason`

### Caching

Use `x_sync_cache` only for transport response reuse, not as truth.

Rules:

- canonical tables are the source of truth
- cache rows have TTL and transport/surface metadata
- write commands invalidate overlapping cache rows
- ops can show cache freshness

## Research And Digest Plan

### X Research Brief

`arcwell x research <query>` should:

1. search local canonical tweets, defaulting to bookmarks and watch-source rows
2. rank by collection/watch provenance, recency, engagement, and optional score
3. expand local thread context through `conversation_id`, `reply_to_x_id`, and
   `x_tweet_refs`
4. label missing ancestors/descendants instead of inventing them
5. optionally perform live thread lookup only behind policy/cost gates
6. extract links and handles
7. write a Markdown wiki page and structured source-card-backed evidence pack
8. return JSON with seed tweets, thread nodes, source cards, links, handles,
   missing context, and costs

### Digest Candidates

Current `x_monitor_watch_sources` already creates digest candidates. Upgrade it
to:

- attach candidate ids to canonical tweet/thread ids
- include provenance and score freshness
- distinguish heuristic candidate from model-scored candidate
- require delivery policy before outbound email/Telegram

### AI Scoring

Add scoring only after canonical rows exist:

- deterministic heuristic ranker first
- optional provider-backed scoring behind explicit config
- store scores in `x_scores`
- add eval fixtures before model-backed default use
- no auto-delivery based only on score

## Ops And UI Plan

Initial UI lane should be operational, not a full Birdclaw clone.

Add to `/ops/ui` or a focused local route:

- X canonical counts
- latest sync runs
- watch-source status
- source-health failures
- cursor values
- credential health
- quota/auth failure summaries
- projection failures
- recent bookmark/watch imports
- digest candidates with source-card/wiki links
- FTS health and last rebuild status

Controls, in this order:

1. read-only filters and detail views
2. repair projection action
3. rebuild FTS action
4. run one bounded sync action
5. apply/reject digest candidate only after candidate APIs are safe

Browser validation:

- desktop and mobile
- no clipped text in dense tables
- no overlapping controls
- XSS fixtures for tweet text, profile names, descriptions, URLs, and errors
- auth/CSRF/idempotency for any POST action

## Backup And Export Plan

Arcwell backup already copies SQLite/wiki/memory artifacts. Add an optional
portable X export for review, Git storage, and migration.

Command:

```text
arcwell x export-portable --out <dir>
arcwell x validate-portable <dir>
arcwell x import-portable <dir>
```

Shard layout:

```text
manifest.json
data/x/accounts.jsonl
data/x/profiles.jsonl
data/x/profile_snapshots.jsonl
data/x/tweets/YYYY.jsonl
data/x/tweets/unknown.jsonl
data/x/tweet_edges/YYYY.jsonl
data/x/collections/bookmarks.jsonl
data/x/collections/likes.jsonl
data/x/follow_snapshots.jsonl
data/x/follow_edges.jsonl
data/x/follow_events.jsonl
data/x/urls.jsonl
data/x/media.jsonl
data/x/scores.jsonl
data/x/projections.jsonl
```

Do not export:

- OAuth tokens
- SQLite secret values
- FTS shadow tables
- transient sync cache rows unless explicitly requested for debugging
- raw DMs by default

Validation:

- manifest hashes
- JSONL parseability
- row counts
- schema version
- no token-like values in default export

## Security, Privacy, And Policy Boundaries

### Reads

- local reads are allowed by default but should carry provenance
- DMs need explicit retention/import opt-in
- profile descriptions and tweet text are untrusted source strings

### Network

- policy guard before credential lookup/network
- cost reservation before API calls
- provider failures redacted
- rate-limit/quota failures preserve cursor
- cache can prevent unnecessary network calls

### Writes

Social writes are late-phase only:

- block/mute/reply/post require explicit confirmation or durable approval
- output must show target account, target profile/tweet, transport, body, and
  policy decision before execution
- every remote write gets an audit row and local pending/reconciled state

### DMs

DM support should be a separate gated phase:

- default no raw DM import from archive
- explicit command flag required
- separate retention setting
- export redacts or omits by default
- ops labels whether DMs are enabled

## Implementation Phases

### Phase 0: Baseline And Contract Freeze

Goal: define current behavior before changing storage.

Tasks:

- record current `x import-json`, `x recent-search`, `x import-bookmarks`,
  `x monitor-watch-sources`, `x list`, `x bookmarks`, and `x report` JSON
  envelopes as compatibility fixtures
- add a small `x stats` command if needed to inspect current counts
- document `x_items` as compatibility/projection storage, not future truth
- identify every test currently asserting `x_items` directly

Done when:

- compatibility fixtures exist
- current tests pass
- no implementation change claims new capability

Validation:

```sh
cargo fmt -- --check
cargo test --all --all-features x_
scripts/verify-codex-plugin-docs
```

### Phase 1: Canonical Tweets, Profiles, Edges, Collections, FTS

Goal: land the durable schema and dual-write without changing command UX.

Tasks:

- add schema tables:
  - `x_accounts`
  - `x_profiles`
  - `x_profile_snapshots`
  - `x_profile_entities`
  - `x_tweets`
  - `x_tweet_refs`
  - `x_tweet_edges`
  - `x_collections`
  - `x_tweets_fts`
  - `x_sync_runs`
- implement canonical upsert helpers
- make `insert_x_item` call canonical write first, then existing source-card
  projection path
- backfill canonical rows from existing `x_items`
- add FTS insert/update and `x rebuild-fts`
- keep `x_items` and `x_item_sources` populated

Severe tests:

- duplicate `x_id` updates canonical row without duplicating source card
- prompt-injection text is searchable as data, not executed
- unsafe URL rejects before canonical insert
- malformed profile metadata rejects only the affected profile/tweet where safe
- FTS query handles punctuation, URLs, handles, and quoted phrases
- migration from old `x_items` fixture preserves current `x list` output

Done when:

- old CLI/MCP outputs still work
- canonical counts match compatibility counts
- FTS search works locally
- no live provider needed for proof

### Phase 2: Canonical Bookmark And Watch Sync

Goal: current live X paths write canonical state and provenance.

Tasks:

- update `x_import_bookmarks` to write:
  - profiles
  - tweets
  - `x_collections` bookmark rows
  - `x_tweet_edges` bookmark rows
  - compatibility `x_items` projection
- update `x_recent_search` to write search edges and canonical tweets/profiles
- update `x_monitor_watch_sources` to write watch edges and canonical tweets
- add `x_sync_runs` for these commands
- add early-stop and `--max-pages` for bookmark sync
- move `x list`, `x bookmarks`, `x report` reads to canonical query layer while
  preserving output shape

Severe tests:

- quota failure does not advance cursor or corrupt sync run
- duplicate newest id does not create digest candidates
- partial protected/deleted tweet responses do not advance cursor
- expired token failure is redacted and visible in source health
- bookmark page with all duplicate canonical rows stops with saturation reason
- source-card projection failure leaves repairable projection state

Live smoke:

```sh
X_USER_CONTEXT_SOURCE_HOME="$ARCWELL_HOME" scripts/x-live-smoke
```

Done when:

- copied-home live smoke still passes
- bookmark provenance is queryable from canonical rows
- digest candidates link to canonical tweet ids and source cards

### Phase 3: Archive Import

Goal: historical completeness without API spend.

Tasks:

- add archive discovery command
- add archive reader and slice parser
- import tweets/profile/account slices first
- import likes/bookmarks collections
- import followers/following snapshots
- parse media metadata but defer byte extraction unless cheap
- implement selected imports
- add explicit account identity conflict checks
- write all rows through canonical pipeline

Severe tests:

- archive path traversal rejected
- JS wrapper is parsed as data, never executed
- malformed slice fails the slice with precise error
- selected `bookmarks` import preserves existing tweets/profiles where needed
- selected `tweets` import preserves existing bookmark collection rows
- account mismatch fails before writing
- import is idempotent

Done when:

- fixture archive imports tweets, bookmarks, likes, profiles, followers, and
  following into canonical rows
- no network/secret access occurs during archive import
- `x search-tweets --bookmarked` finds archive bookmarks

### Phase 4: Thread, Link, And Research Briefs

Goal: make saved/watched X material useful for research.

Tasks:

- add thread expansion query over `conversation_id`, `reply_to_x_id`, and
  `x_tweet_refs`
- add missing-parent labels
- add local link occurrence extraction
- add optional safe URL expansion with existing URL security rules
- add `x research`
- create wiki/source-card-backed research brief projection
- connect to deep research as a source pack, not a generated final answer

Severe tests:

- thread cycles cannot loop forever
- missing ancestors are labeled, not invented
- prompt-injection tweets remain quoted evidence
- URL expansion rejects loopback/private/metadata hosts
- generated brief links all claims back to tweet/source-card ids
- brief creation fails honestly on no evidence

Done when:

- local bookmark/watch research brief can be generated without live network
- optional live expansion is separately policy/cost gated

### Phase 5: Ops/UI Lane

Goal: make X state inspectable and repairable.

Tasks:

- add X section to ops snapshot
- add UI views for:
  - canonical counts
  - latest sync runs
  - source health
  - watch sources
  - projection failures
  - credential health
  - digest candidates
- add read-only filters and detail views
- add authenticated `rebuild-fts` and `repair-projections` controls only after
  core APIs are idempotent

Severe/browser tests:

- desktop/mobile browser smoke
- no overlap/clipping in dense X tables
- XSS fixtures for tweet/profile/link/error fields
- POST controls require auth, origin, CSRF/idempotency, and policy where
  appropriate

Done when:

- an operator can tell whether X sync is fresh, blocked, stale, rate-limited, or
  projection-broken without reading raw SQLite

### Phase 6: Follow Graph And Identity

Goal: use follows/profile history as context, not as the default noisy watch
seed.

Tasks:

- archive followers/following snapshots
- live limited following import can populate graph rows
- follow events for started/ended edges
- graph summary and graph events commands
- profile entity extraction from bio/url/affiliation
- identity search for profiles

Severe tests:

- partial follow snapshot does not generate churn events
- duplicate snapshot does not duplicate events
- full following import is not used as default watch list
- hostile profile bio remains data
- profile entity extraction cannot create commands or source instructions

Done when:

- graph summary is useful locally
- watch rebuild still uses the definitive bookmark/recent-follow seed

### Phase 7: Portable Export And Backup Integration

Goal: Git-friendly, reviewable, token-free X data export.

Tasks:

- implement `x export-portable`
- implement `x validate-portable`
- optional `x import-portable`
- add backup manifest warning that X data exists and whether portable export is
  configured
- decide whether scheduled backup should include portable X export

Severe tests:

- manifest hash mismatch fails
- JSONL parse failure fails
- token-like secret values are not exported
- FTS/cache rows excluded
- DMs excluded by default
- import-portable is idempotent

Done when:

- disposable export/validate/import round trip passes

### Phase 8: AI Scoring And Digest Routing

Goal: rank and route important X items without confusing scores for truth.

Tasks:

- deterministic heuristic scoring
- `x_scores` overlay rows
- optional model-backed scorer behind explicit config
- evaluation fixtures
- digest candidate upgrade path from heuristic to model-scored
- delivery routing through existing email/Telegram delivery attempts

Severe tests:

- scoring prompt injection cannot trigger tool/action behavior
- low-confidence score does not delete or hide canonical evidence
- score expiry visible
- delivery requires separate policy/authorization
- cost records exist for model-backed scoring

Done when:

- X digest candidates can be reviewed and optionally delivered through existing
  channel infrastructure with provenance and cost trail

### Phase 9: Media Cache

Goal: make media inspectable without turning network fetch into a surprise.

Tasks:

- archive media extraction
- metadata-only live media ingestion
- optional live fetch command with pacing, size limit, retry budget
- local path storage under `ARCWELL_HOME/x-media`
- source-card/wiki media references

Severe tests:

- archive media path traversal rejected
- remote media fetch obeys size and content-type limits
- retry/rate limit does not spin
- local paths do not escape media root

Done when:

- archive media for imported tweets is locally inspectable
- live media fetching remains opt-in

### Phase 10: DMs And Moderation

Goal: reach Birdclaw-class personal console features only after privacy and
approval boundaries exist.

DM tasks:

- explicit retention opt-in
- archive DM import
- `x_dms`/`x_dm_conversations`/`x_dm_events` schema
- `x_dms_fts`
- redacted/default-off portable export
- profile context for participants

Moderation/write tasks:

- local block/mute state
- live action adapters
- explicit confirmation flow
- audit rows
- remote reconciliation

Severe tests:

- raw DMs not imported/exported by default
- DM prompt injection remains data
- block/mute/reply requires approval
- wrong-account remote writes impossible
- remote write failure leaves local state pending, not falsely reconciled

Done when:

- DMs are opt-in, searchable locally, and safely excluded from default exports
- moderation writes are confirmed, audited, and reconciled

## Validation Baseline

For every meaningful phase:

```sh
cargo fmt -- --check
cargo test --all --all-features
scripts/verify-codex-plugin-docs
scripts/x-live-smoke   # when live X behavior changed and credentials are available
scripts/arcwell-dev smoke
scripts/arcwell-dev sync
```

Additional phase-specific checks:

- archive import: fixture archive import/round trip
- UI: browser desktop/mobile smoke
- worker/scheduled sync: worker run-once tests and source-health assertions
- MCP: tool round-trip tests and resource shape tests
- docs: update `packages/arcwell-x/README.md`, `docs/functionality-and-packages.md`,
  `STATUS.md`, and `TODO.md` when capability work lands

## Primary Risks And Controls

| Risk | Control |
| --- | --- |
| Data migration corrupts existing `x_items` behavior | dual-write first, compatibility fixtures, old-output tests |
| Source-card projection failure hides canonical evidence | `x_projections` repair state and repair command |
| X API quota or tier changes break sync | archive-first path, early-stop, cache, source-health visibility |
| MCP surface becomes too broad | add task-level tools only |
| Prompt injection enters digest/research | untrusted-source rendering, severe fixtures, no source text as instructions |
| Secrets leak through reports/export | redaction tests, no secret get, export token scan |
| Model scores become false authority | overlay rows with reasons/freshness, no auto-delivery |
| DMs create privacy/backups risk | default-off retention, default-off export, explicit ops labeling |
| Social writes hit wrong account | late phase, account-scoped confirmation, audit and reconciliation |

## First Implementation Slice

Start with the smallest change that moves the architecture:

1. Add canonical tables for accounts, profiles, tweets, tweet edges,
   collections, FTS, sync runs, and projections.
2. Add a migration/backfill from `x_items` into canonical tables.
3. Change `insert_x_item` to dual-write canonical rows and existing
   `x_items`/source-card projection.
4. Add `x rebuild-fts` and `x search-tweets`.
5. Keep all existing CLI/MCP JSON output compatible.
6. Add severe tests for duplicate rows, prompt injection, unsafe URLs, FTS
   search, and migration compatibility.

This slice gives Arcwell the Birdclaw-shaped backbone without taking on archive
parsing, UI, model scoring, media, DMs, or social writes yet. Once this lands,
every other feature has the right place to attach.

## Anti-Mirage Execution Contract

This section exists because this project has repeatedly had features that looked
done until pressure exposed that they were scaffolds, thin happy paths, or
unverified local-only demos. The plan below is intentionally strict. It is
better to leave a checkbox open than to mark a fragile illusion as complete.

### Status Vocabulary

Use these labels consistently in code comments, docs, `STATUS.md`, `TODO.md`,
PR descriptions, and final summaries.

- [ ] `Missing`: no meaningful implementation exists.
- [ ] `Scaffold`: command, schema, prompt, README, or placeholder exists, but
      behavior is not real enough to rely on.
- [ ] `Partial`: useful behavior exists, but important failure modes,
      integrations, or verification remain.
- [ ] `Local Proof`: behavior is implemented and proven in deterministic local
      tests or disposable local smokes.
- [ ] `Live Proof`: behavior is proven against the real provider/deployment
      when that provider/deployment matters.
- [ ] `Operational`: behavior is implemented, tested, documented, observable in
      ops/doctor/source-health where relevant, and has a recovery path.
- [ ] `Done`: operational, documented, checked into status/TODO, and backed by
      tests that would fail for plausible broken implementations.

Do not use `Done` for:

- [ ] a command that only returns a success-shaped JSON envelope
- [ ] a command that only works with one fixture
- [ ] a feature whose docs are more complete than its behavior
- [ ] a live integration proven only by mocked provider data
- [ ] a path that works only when the database is pristine
- [ ] a sync path with no cursor/rate-limit/error proof
- [ ] an import path with no idempotency proof
- [ ] an ops-visible path with no stale/failure state
- [ ] an MCP tool that has no CLI parity or round-trip test
- [ ] a CLI path that has no MCP/slash-command parity when exposed to agents
- [ ] a background job with no retry/dead-letter/source-health behavior
- [ ] a model-backed feature with no cost record and no eval gate
- [ ] a privacy-sensitive feature whose backup/export/forget behavior is
      undefined

### Feature Claim Ledger

Every feature slice must start by adding a claim ledger to the PR description,
implementation note, or issue before code is written.

Template:

```text
FEATURE:

CLAIM:

USER-VISIBLE BEHAVIOR:

INPUTS:

OUTPUTS:

PERSISTED STATE:

SIDE EFFECTS:

AUTH / POLICY / COST BOUNDARIES:

FAILURE SEMANTICS:

IDEMPOTENCY RULE:

CURSOR / CACHE RULE:

BACKUP / EXPORT RULE:

OPS / DOCTOR VISIBILITY:

CLI SURFACE:

MCP / PLUGIN SURFACE:

TESTS THAT WOULD REFUTE THIS CLAIM:

LIVE PROOF REQUIRED:

WHAT WOULD MAKE THIS A MIRAGE:
```

No implementation slice can be marked complete until the ledger has been
answered in concrete terms.

### Completion Gate Stack

Each feature moves through the same gate stack.

- [ ] Gate 0: Claim is named.
- [ ] Gate 1: Existing behavior is inspected in the real codebase.
- [ ] Gate 2: Success and failure semantics are written down.
- [ ] Gate 3: Schema/storage changes are designed with migration and rollback.
- [ ] Gate 4: Public surfaces are listed: CLI, MCP, slash command, docs, ops.
- [ ] Gate 5: Severe tests are written or planned before implementation.
- [ ] Gate 6: Implementation is complete enough to run the tests.
- [ ] Gate 7: Targeted tests pass.
- [ ] Gate 8: Broad regression tests pass.
- [ ] Gate 9: Live smoke passes when external behavior is claimed.
- [ ] Gate 10: Ops/doctor/source-health visibility exists for long-running or
      failure-prone behavior.
- [ ] Gate 11: Docs and status files are updated honestly.
- [ ] Gate 12: Adversarial review finds no blocking issue.
- [ ] Gate 13: Remaining risks are explicitly listed.

If a gate is skipped, the feature status cannot exceed `Partial`.

### Evidence Tiers

Use evidence tiers when describing confidence.

- [ ] Tier 0: No proof. Idea only.
- [ ] Tier 1: Code inspection only.
- [ ] Tier 2: Single local happy-path test.
- [ ] Tier 3: Local unit/integration tests including negative cases.
- [ ] Tier 4: Severe tests with malicious, malformed, duplicate, stale, and
      recovery cases.
- [ ] Tier 5: Disposable local smoke with real binary/CLI/MCP process.
- [ ] Tier 6: Live provider/deployment smoke with real credentials and redacted
      artifacts.
- [ ] Tier 7: Operational proof: live smoke plus ops visibility, retry/recovery,
      docs, and ongoing monitor/doctor signal.

Default minimum tiers:

- [ ] storage/migration: Tier 4
- [ ] CLI-only local import: Tier 4
- [ ] MCP-exposed import/search/report: Tier 5
- [ ] live provider read: Tier 6
- [ ] background sync/monitor: Tier 7
- [ ] outbound delivery/social write: Tier 7 plus explicit approval proof
- [ ] model-backed scoring/synthesis: Tier 4 deterministic plus optional Tier 6
      provider proof before live quality claims
- [ ] DMs/privacy-sensitive import: Tier 4 plus explicit backup/export/forget
      proof

### False-Done Traps

During every review, explicitly search for these traps.

- [ ] The schema exists but no code writes it.
- [ ] The code writes it but no reader uses it.
- [ ] The reader uses it but old compatibility paths silently diverge.
- [ ] The CLI works but MCP returns a different shape.
- [ ] MCP works but plugin/slash docs still point to obsolete behavior.
- [ ] The import is idempotent only because the fixture has one item.
- [ ] The sync advances cursor before projection/source-card write.
- [ ] The sync returns success when every item was rejected.
- [ ] The test asserts a count but not the durable row contents.
- [ ] The test checks JSON shape but not source-card/wiki/projection links.
- [ ] The migration handles current schema but not old fixture schema.
- [ ] The feature works only with an empty database.
- [ ] The feature works only with env tokens, not stored local secrets.
- [ ] Error messages leak token-like text.
- [ ] Quota failures consume budget or corrupt source health.
- [ ] Rate limits retry forever or silently give up.
- [ ] Live smoke uses app-only bearer but claims user-context proof.
- [ ] Watch-source rebuild deletes the old list before new candidates are
      collected.
- [ ] Archive import trusts filenames inside a zip.
- [ ] Archive import executes or evaluates wrapper JavaScript.
- [ ] Source-card projection failure hides the canonical evidence.
- [ ] Model scoring overwrites canonical truth.
- [ ] Digest delivery happens without separate delivery authorization.
- [ ] Ops UI shows stale data without freshness labels.
- [ ] Portable export includes secrets, cache rows, FTS rows, or raw DMs by
      default.
- [ ] Docs say "implemented" when the real status is scaffold or partial.

### Mandatory Adversarial Review Lenses

Every phase must run an adversarial review through the relevant lenses. The
review should report demonstrated findings, not long speculative lists.

- [ ] Storage integrity: schema, migrations, transactions, rollback, old data.
- [ ] Idempotency: repeated import, duplicate pages, retries, partial writes.
- [ ] Cursor safety: cursor advances only after durable accepted writes.
- [ ] Projection safety: canonical data is not lost if source-card/wiki fails.
- [ ] Provider failure: 401/403/429/5xx, malformed payloads, partial errors.
- [ ] Secret privacy: tokens not printed, logged, exported, cached, or put in
      source cards.
- [ ] Prompt injection: tweet/profile/DM/link text never becomes instructions.
- [ ] URL safety: SSRF, redirect, content-type, size, timeout, private hosts.
- [ ] Archive safety: zip slip, decompression bombs, wrapper parsing, huge
      files, malformed slices.
- [ ] Multi-account correctness: no cross-account reads/writes/cursors.
- [ ] Policy/cost: guard before credentials/network/mutation, reservations
      released on classified failures.
- [ ] MCP/CLI parity: same behavior, same honesty, compatible JSON.
- [ ] Ops/doctor visibility: stale, failed, blocked, partial, retrying, and
      healthy states are distinguishable.
- [ ] Backup/export/forget: no private leakage, restore/import round trip.
- [ ] UI abuse: XSS, clipped content, hidden controls, stale action state.
- [ ] Model misuse: scoring/synthesis does not invent evidence or authorize
      actions.
- [ ] Performance/resource: unbounded loops, memory growth, huge archives,
      repeated provider calls.
- [ ] Live proof: the exact claimed integration is the one tested.

### Root-Cause Response Rules

When a test or live smoke fails:

- [ ] Stop adding speculative fixes.
- [ ] Preserve the failing artifact if it does not contain secrets.
- [ ] Identify whether failure is code, test, fixture, provider, credentials,
      queue state, stale binary, or docs.
- [ ] Reproduce locally if possible.
- [ ] Add or keep the reproducer as a regression test.
- [ ] Fix the smallest root cause.
- [ ] Re-run the failing test first.
- [ ] Then run the nearest broader gate.
- [ ] Record remaining risk if the failure depends on external provider state.

Do not:

- [ ] weaken assertions to match broken behavior
- [ ] delete failing fixtures without replacing coverage
- [ ] mark live-provider failures as done because local tests pass
- [ ] ignore stale binary, stale queue, or stale credential explanations
- [ ] call an intermittent pass enough for production monitoring

## Detailed Completion Matrices

The matrices below are intentionally checklist-heavy. They are the work queue
for avoiding another incomplete mirage.

### 1. Canonical Schema And Migration

Feature claim:

> Existing and future X imports have a canonical, normalized SQLite home that
> can represent tweets, profiles, account/source edges, collections, sync runs,
> projections, and FTS search without breaking existing `x_items` behavior.

Implementation checklist:

- [ ] Add schema version entry for canonical X tables.
- [ ] Add `x_accounts`.
- [ ] Add `x_profiles`.
- [ ] Add `x_profile_snapshots`.
- [ ] Add `x_profile_entities`.
- [ ] Add `x_tweets`.
- [ ] Add `x_tweet_refs`.
- [ ] Add `x_tweet_edges`.
- [ ] Add `x_collections`.
- [ ] Add `x_tweets_fts`.
- [ ] Add `x_sync_runs`.
- [ ] Add `x_projections`.
- [ ] Add indexes for tweet id, author/date, conversation/date, collection,
      edge, profile handle, sync run, and projection status.
- [ ] Add schema introspection to ops snapshot counts.
- [ ] Add migration from old `x_items` rows.
- [ ] Add migration from old `x_item_sources` rows into `x_tweet_edges`.
- [ ] Create synthetic default account for legacy rows.
- [ ] Preserve `source_card_id` and `wiki_page_id` links.
- [ ] Backfill FTS from migrated rows.
- [ ] Keep old `x_items` table readable.
- [ ] Keep old `x_item_sources` table readable.
- [ ] Document `x_items` as compatibility/projection storage.

Severe tests:

- [ ] CLAIM: migration preserves old `x list` output.
- [ ] CLAIM: migration preserves old `x report` output.
- [ ] CLAIM: migration preserves source-card/wiki links.
- [ ] CLAIM: duplicate legacy `x_id` rows cannot create duplicate canonical
      tweets.
- [ ] CLAIM: malformed legacy raw JSON falls back safely or fails migration with
      exact blocker.
- [ ] CLAIM: FTS backfill returns migrated tweet text.
- [ ] CLAIM: migration can run twice without changing row counts.
- [ ] CLAIM: old database fixture with missing post-migration `x_items` columns
      is upgraded.
- [ ] CLAIM: migration rollback on failure leaves prior schema readable.
- [ ] CLAIM: schema drift is detected by strict doctor or test fixture.

False-done traps:

- [ ] canonical tables exist but import paths do not write them
- [ ] migration only tested on empty database
- [ ] FTS exists but is never populated
- [ ] old CLI reads canonical rows but MCP still reads stale `x_items`
- [ ] docs claim migration when only new installs work

Done evidence:

- [ ] targeted migration tests pass
- [ ] old-output compatibility fixture passes
- [ ] FTS fixture passes
- [ ] `cargo test --all --all-features` passes
- [ ] `STATUS.md` says `Local Proof` until a live sync uses canonical rows

### 2. Canonical Write Pipeline

Feature claim:

> Every X import path writes profiles, tweets, references, edges, collections,
> FTS rows, sync-run metadata, and projection requests through one canonical
> pipeline.

Implementation checklist:

- [ ] Define `XCanonicalWriteInput`.
- [ ] Define `XCanonicalWriteReport`.
- [ ] Define normalized profile/tweet/ref/media/url/edge/collection structs.
- [ ] Add input validation before transaction.
- [ ] Add canonical profile upsert.
- [ ] Add profile snapshot hashing.
- [ ] Add profile entity extraction placeholder.
- [ ] Add canonical tweet upsert.
- [ ] Add metrics merge rules.
- [ ] Add raw JSON merge rules.
- [ ] Add tweet refs upsert.
- [ ] Add tweet edge upsert.
- [ ] Add collection membership upsert.
- [ ] Add FTS update.
- [ ] Add projection request insert.
- [ ] Add report counts for inserted, updated, duplicate, rejected, projected.
- [ ] Add transaction boundary around canonical writes.
- [ ] Add clear failure semantics for validation vs storage errors.
- [ ] Add compatibility write to `x_items`.
- [ ] Add compatibility write to `x_item_sources`.

Severe tests:

- [ ] CLAIM: a duplicate tweet updates metrics and edge provenance without
      duplicating tweet rows.
- [ ] CLAIM: invalid URL rejects before any partial write.
- [ ] CLAIM: invalid handle rejects profile while preserving safely accepted
      independent items where batch semantics allow.
- [ ] CLAIM: transaction rollback removes FTS/projection rows after injected
      storage failure.
- [ ] CLAIM: source-card projection failure does not roll back canonical row if
      projection is asynchronous/repairable.
- [ ] CLAIM: batch report counts match actual durable rows.
- [ ] CLAIM: prompt-injection text is preserved as text and never parsed as
      config/policy/tool instruction.
- [ ] CLAIM: raw payload too large is bounded or rejected.
- [ ] CLAIM: repeated batch with different source kind creates a new edge, not a
      new tweet.
- [ ] CLAIM: repeated batch with same source kind updates `last_seen_at` and
      `seen_count`.

False-done traps:

- [ ] report says imported while canonical transaction failed
- [ ] FTS update happens outside transaction and survives rollback
- [ ] projection state is not repairable
- [ ] compatibility write silently diverges from canonical write
- [ ] batch partially writes with no rejected-item accounting

Done evidence:

- [ ] canonical write tests pass
- [ ] compatibility read tests pass
- [ ] injected-failure tests pass
- [ ] source-card/wiki projection links remain valid

### 3. Compatibility Surface

Feature claim:

> Existing CLI/MCP users cannot tell that the storage backend changed, except
> that search/report behavior becomes more accurate and better proven.

Implementation checklist:

- [ ] Capture current `x import-json` output fixture.
- [ ] Capture current `x recent-search` mocked output fixture.
- [ ] Capture current `x import-bookmarks` mocked output fixture.
- [ ] Capture current `x list` output fixture.
- [ ] Capture current `x bookmarks` output fixture.
- [ ] Capture current `x report` output fixture.
- [ ] Capture MCP `x_import_json_file` output fixture.
- [ ] Capture MCP `x_list` output fixture.
- [ ] Capture MCP `x_report` output fixture.
- [ ] Make `x list` read canonical rows through compatibility projection.
- [ ] Make `x bookmarks` read canonical collection rows.
- [ ] Make `x report` read canonical rows.
- [ ] Keep old JSON fields stable: `id`, `x_id`, `author`, `text`, `url`,
      `created_at`, `imported_at`, `retrieved_at`, `metrics`, `raw`,
      `source_card_id`, `wiki_page_id`, `sources`.
- [ ] Add new fields only if optional and documented.
- [ ] Keep MCP schemas honest about optional fields.

Severe tests:

- [ ] CLAIM: old fixture output still parses under existing consumer shape.
- [ ] CLAIM: source filter `bookmark` returns collection rows, not stale
      compatibility rows.
- [ ] CLAIM: query filter cannot bypass validation.
- [ ] CLAIM: limit clamps work the same through CLI and MCP.
- [ ] CLAIM: missing optional new fields do not break old output.
- [ ] CLAIM: docs verifier catches stale command/tool descriptions.

False-done traps:

- [ ] CLI migrated but MCP left stale
- [ ] command docs mention old `x_items` truth
- [ ] source filter reads old table and misses canonical data
- [ ] report markdown links to old source cards only

Done evidence:

- [ ] CLI fixture tests pass
- [ ] MCP fixture tests pass
- [ ] `scripts/verify-codex-plugin-docs` passes
- [ ] package README updated honestly

### 4. FTS Search

Feature claim:

> X tweet search uses durable FTS5 indexes and can be rebuilt, verified, and
> queried without relying on weak `LIKE` scans.

Implementation checklist:

- [ ] Add `x_tweets_fts`.
- [ ] Add insert/update/delete sync helpers.
- [ ] Add `x rebuild-fts`.
- [ ] Add `x search-tweets <query>`.
- [ ] Add optional filters: author, source, bookmarked, liked, since, until,
      limit.
- [ ] Add search result shape with source/provenance.
- [ ] Add FTS health count to ops.
- [ ] Add strict doctor warning when FTS count is stale.
- [ ] Keep `LIKE` fallback only for repair/debug if needed.

Severe tests:

- [ ] CLAIM: punctuation-heavy query finds tweet text.
- [ ] CLAIM: URL-heavy query finds expanded/display URL text where indexed.
- [ ] CLAIM: handle query finds author handle.
- [ ] CLAIM: Unicode normalization cases are handled predictably.
- [ ] CLAIM: empty query is rejected or treated as bounded list.
- [ ] CLAIM: very long query is rejected before expensive FTS.
- [ ] CLAIM: FTS rebuild restores missing rows.
- [ ] CLAIM: deleted/repaired canonical row does not leave stale FTS result.
- [ ] CLAIM: source/bookmark filters combine with FTS correctly.
- [ ] CLAIM: CLI and MCP return same search results.

False-done traps:

- [ ] command exists but still uses `LIKE`
- [ ] FTS only populated for new rows, not migrated rows
- [ ] rebuild command prints success but does not compare counts
- [ ] search has no source/provenance in output

Done evidence:

- [ ] FTS tests pass
- [ ] rebuild test corrupts then repairs index
- [ ] ops/doctor stale-index visibility exists

### 5. Source-Card And Wiki Projections

Feature claim:

> Canonical X rows can be projected into source cards, wiki pages, digest
> candidates, and research briefs without hiding canonical data or duplicating
> projections.

Implementation checklist:

- [ ] Add `x_projections`.
- [ ] Record projection status for source card.
- [ ] Record projection status for wiki page.
- [ ] Record projection status for digest candidate.
- [ ] Add unique projection key per entity/projection kind.
- [ ] Link source-card metadata back to canonical `x_tweets.x_id`.
- [ ] Link wiki page metadata back to source-card and canonical tweet.
- [ ] Add repair command for missing/failed source-card projections.
- [ ] Add repair command for missing/failed wiki projections.
- [ ] Add ops list for failed projections.
- [ ] Keep untrusted-source warning in rendered source-card/wiki content.
- [ ] Ensure projection is idempotent.

Severe tests:

- [ ] CLAIM: projection failure leaves canonical tweet visible in search.
- [ ] CLAIM: repair creates missing source card exactly once.
- [ ] CLAIM: duplicate projection request does not duplicate wiki page.
- [ ] CLAIM: hostile tweet text is quoted/escaped in wiki markdown.
- [ ] CLAIM: hostile profile name cannot inject markdown/script into ops UI.
- [ ] CLAIM: source-card metadata includes canonical ids.
- [ ] CLAIM: failed projection appears in ops snapshot.
- [ ] CLAIM: digest candidate links to source-card id and canonical x id.

False-done traps:

- [ ] projection occurs inline and failed projection loses canonical row
- [ ] repair command creates duplicates
- [ ] source cards have no canonical back-reference
- [ ] wiki page text treats tweet body as instructions

Done evidence:

- [ ] projection failure-injection tests pass
- [ ] repair tests pass
- [ ] ops failed-projection visibility exists

### 6. OAuth And Credential Handling

Feature claim:

> X OAuth setup, exchange, refresh, token use, token expiry, and credential
> health are safe, redacted, policy-aware, and distinguish app-only from
> user-context capabilities.

Implementation checklist:

- [ ] Keep OAuth URL PKCE generation.
- [ ] Keep code exchange.
- [ ] Keep refresh.
- [ ] Store access token under secret metadata only.
- [ ] Store refresh token under secret metadata only.
- [ ] Store expiry and scopes where available.
- [ ] Store user-context capability flag where known.
- [ ] Distinguish env bearer from SQLite secret in health output.
- [ ] Add credential probe command only if cheap and policy-gated.
- [ ] Add source-health failure for expired/rejected token.
- [ ] Add ops credential health.
- [ ] Redact token-like strings from errors.
- [ ] Avoid passing secrets as CLI args in docs where possible.

Severe tests:

- [ ] CLAIM: token values never appear in CLI output.
- [ ] CLAIM: token values never appear in MCP output.
- [ ] CLAIM: token values never appear in source health.
- [ ] CLAIM: token values never appear in ops UI.
- [ ] CLAIM: refresh failure preserves refresh token and redacts error.
- [ ] CLAIM: missing user-context scopes fail honestly for bookmarks/follows.
- [ ] CLAIM: app-only bearer cannot mask copied user-context proof in smoke.
- [ ] CLAIM: expired token blocks before budget burn where appropriate.
- [ ] CLAIM: policy denial happens before credential lookup.
- [ ] CLAIM: malformed OAuth token response creates no secret rows.

False-done traps:

- [ ] app-only recent search is described as bookmark/follow proof
- [ ] token refresh works once but expiry metadata missing
- [ ] error redaction only applied to stdout, not source health
- [ ] docs encourage secrets in shell history

Done evidence:

- [ ] OAuth severe tests pass
- [ ] secret-health tests pass
- [ ] copied-home live smoke proves user context when claimed

### 7. Recent Search Sync

Feature claim:

> `x recent-search` imports live search results into canonical tweets and source
> cards, advances cursor only after durable accepted writes, and fails visibly on
> provider errors.

Implementation checklist:

- [ ] Map X API tweet payload into canonical tweet.
- [ ] Map included users into canonical profiles.
- [ ] Write `x_tweet_edges` with `edge_kind = recent_search`.
- [ ] Preserve existing `x_items` compatibility projection.
- [ ] Write sync run.
- [ ] Write source health.
- [ ] Write cursor after durable write.
- [ ] Add partial-error classification.
- [ ] Add rejected-item accounting.
- [ ] Add cost reservation and release behavior.
- [ ] Add CLI and MCP parity tests.

Severe tests:

- [ ] CLAIM: malformed tweet prevents cursor advance.
- [ ] CLAIM: provider partial errors do not create false success.
- [ ] CLAIM: duplicate newest id does not regress cursor.
- [ ] CLAIM: older provider newest id does not regress cursor.
- [ ] CLAIM: 429 preserves cursor and releases cost reservation.
- [ ] CLAIM: 401 records redacted source-health failure.
- [ ] CLAIM: query validation rejects unsafe/oversized query.
- [ ] CLAIM: source-card projection contains untrusted-source warning.
- [ ] CLAIM: canonical and compatibility counts agree.
- [ ] CLAIM: MCP and CLI reports match.

False-done traps:

- [ ] import succeeds but cursor not saved
- [ ] cursor saved before projection/source-card write
- [ ] report only counts provider `data` length, not accepted rows
- [ ] source health shows success when all rows rejected

Done evidence:

- [ ] mocked severe tests pass
- [ ] live recent-search smoke passes when token available
- [ ] source health visible after success and failure

### 8. Bookmark Sync

Feature claim:

> Authenticated bookmark sync imports bookmark tweets into canonical tweets,
> profiles, bookmark collections, source-card projections, and watch-source seed
> candidates without corrupting cursors, cost records, or provenance.

Implementation checklist:

- [ ] Add canonical mapper for bookmark endpoint.
- [ ] Write profiles.
- [ ] Write tweets.
- [ ] Write `x_collections` bookmark rows.
- [ ] Write `x_tweet_edges` bookmark rows.
- [ ] Preserve public metrics.
- [ ] Preserve tweet entities.
- [ ] Preserve author metadata.
- [ ] Add max-pages.
- [ ] Add early-stop.
- [ ] Add refresh/cache semantics.
- [ ] Add sync run.
- [ ] Add source health.
- [ ] Keep `x bookmarks` compatibility command.
- [ ] Add `x sync-bookmarks` alias or replacement.

Severe tests:

- [ ] CLAIM: duplicate page triggers early-stop when requested.
- [ ] CLAIM: duplicate page without early-stop respects max-pages.
- [ ] CLAIM: old bookmark outside window is skipped with count.
- [ ] CLAIM: malformed author expansion rejects affected tweet.
- [ ] CLAIM: protected/deleted tweet does not advance cursor incorrectly.
- [ ] CLAIM: app-only token fails with user-context scope message.
- [ ] CLAIM: quota failure preserves previous state.
- [ ] CLAIM: repeated sync updates collection `last_seen_at`.
- [ ] CLAIM: source-card projection links bookmark provenance.
- [ ] CLAIM: bookmark query reads canonical collections.

False-done traps:

- [ ] imported tweet body exists but no collection row
- [ ] collection row exists but no account id
- [ ] bookmark-days filtering applies after cursor advancement
- [ ] watch-source rebuild reads stale compatibility metadata only

Done evidence:

- [ ] mock bookmark sync tests pass
- [ ] copied user-context live smoke passes
- [ ] `x bookmarks` shows canonical collection rows

### 9. Watch-Source Rebuild

Feature claim:

> The definitive X watch list is rebuilt from recent bookmark authors plus a
> capped recent-follow sample, only replacing the old list after all candidates
> are collected and validated.

Implementation checklist:

- [ ] Preserve current rebuild command.
- [ ] Read bookmark authors from canonical collections when available.
- [ ] Read recent-follow candidates from provider or graph snapshot.
- [ ] Validate handles.
- [ ] Merge duplicate reasons.
- [ ] Cap recent follows.
- [ ] Preserve old watch list until candidate collection succeeds.
- [ ] Transactionally replace active `x_handle` watch sources.
- [ ] Record source health.
- [ ] Record sync run or rebuild run.
- [ ] Expose counts and rejected reasons.

Severe tests:

- [ ] CLAIM: provider failure preserves old watch list exactly.
- [ ] CLAIM: malformed handle rejected without aborting valid candidates.
- [ ] CLAIM: duplicate bookmark/follow author merges reasons.
- [ ] CLAIM: full following import is not used as default seed.
- [ ] CLAIM: max recent follows cap is enforced.
- [ ] CLAIM: old polluted list is removed only after success.
- [ ] CLAIM: no token leak on failure.
- [ ] CLAIM: output counts match durable watch rows.

False-done traps:

- [ ] command appends instead of replacing
- [ ] command deletes first then fails
- [ ] command imports whole following graph by default
- [ ] source-health absent on provider failure

Done evidence:

- [ ] transaction failure tests pass
- [ ] provider failure tests pass
- [ ] live smoke passes with user-context token

### 10. Watch-Source Monitor

Feature claim:

> Active X watch sources are polled safely, new tweets are canonicalized,
> source-card/wiki projections are created or repairable, digest candidates are
> linked, and per-source cursors advance only after durable accepted writes.

Implementation checklist:

- [ ] Read active `x_handle` watch sources.
- [ ] Enforce max sources.
- [ ] Enforce max results per source.
- [ ] Build search query per handle.
- [ ] Read per-source cursor.
- [ ] Fetch provider page.
- [ ] Map tweets/users.
- [ ] Write canonical tweets/profiles/edges.
- [ ] Project source cards/wiki.
- [ ] Create digest candidates.
- [ ] Advance per-source cursor after durable write.
- [ ] Record per-source health.
- [ ] Record aggregate monitor health.
- [ ] Release budget on classified quota failures.
- [ ] Continue or fail according to source-level semantics.

Severe tests:

- [ ] CLAIM: one failed source does not corrupt successful source cursors.
- [ ] CLAIM: 429 on one source preserves its cursor and records failure.
- [ ] CLAIM: malformed item prevents that source cursor advance.
- [ ] CLAIM: duplicate newest id creates no duplicate digest candidate.
- [ ] CLAIM: prompt-injection tweet creates evidence/digest, not instructions.
- [ ] CLAIM: projection failure is visible and repairable.
- [ ] CLAIM: max source cap prevents runaway monitor.
- [ ] CLAIM: cost reservation is bounded by configured source/result caps.
- [ ] CLAIM: source health includes cursor key/value without secrets.
- [ ] CLAIM: worker-triggered monitor behaves like CLI monitor.

False-done traps:

- [ ] monitor reports success when every source failed
- [ ] cursor is global instead of per-source
- [ ] digest candidate created without source-card link
- [ ] failed source disappears from ops

Done evidence:

- [ ] severe monitor tests pass
- [ ] worker run-once monitor test passes if scheduled
- [ ] live monitor smoke passes with disposable/copied home

### 11. Archive Discovery

Feature claim:

> Arcwell can find likely local X/Twitter archives without importing the wrong
> file or mutating state before the user selects an archive.

Implementation checklist:

- [ ] Add explicit path support.
- [ ] Add `x discover-archives`.
- [ ] Search `~/Downloads`.
- [ ] Search configured directories.
- [ ] Optional macOS Spotlight probe.
- [ ] Score candidates by filename, path, recency, and archive contents.
- [ ] Show candidate list with path, size, modified time, and confidence.
- [ ] Do not import automatically when ambiguous.
- [ ] Do not read huge archives deeply during discovery.
- [ ] Add docs for explicit path as safest route.

Severe tests:

- [ ] CLAIM: discovery performs no database writes.
- [ ] CLAIM: unsupported file type is ignored.
- [ ] CLAIM: malicious path with newline/control chars is displayed safely.
- [ ] CLAIM: huge candidate is not fully decompressed during discovery.
- [ ] CLAIM: ambiguous candidates require explicit selection.
- [ ] CLAIM: missing path fails with precise error.

False-done traps:

- [ ] discovery imports automatically
- [ ] discovery trusts filename without content sniff
- [ ] discovery does not handle spaces/control chars in paths

Done evidence:

- [ ] fixture discovery tests pass
- [ ] no-write assertion passes

### 12. Archive Reader And Parser

Feature claim:

> Archive import parses X/Twitter archive slices as data, never code, and safely
> rejects traversal, oversized, malformed, or ambiguous input.

Implementation checklist:

- [ ] Implement archive open with size/file count limits.
- [ ] Reject path traversal.
- [ ] Reject symlink entries where relevant.
- [ ] Reject nested archive recursion.
- [ ] Parse JavaScript wrapper prefix/suffix safely.
- [ ] Parse JSON arrays.
- [ ] Parse account/profile slices.
- [ ] Parse tweet slices.
- [ ] Parse note tweet slices.
- [ ] Parse likes.
- [ ] Parse bookmarks.
- [ ] Parse followers.
- [ ] Parse following.
- [ ] Parse DM metadata later only behind opt-in.
- [ ] Parse media metadata.
- [ ] Preserve per-file parse errors.
- [ ] Return selected-slice summary.

Severe tests:

- [ ] CLAIM: wrapper JS is not executed.
- [ ] CLAIM: zip slip path is rejected.
- [ ] CLAIM: decompression bomb is rejected by configured budget.
- [ ] CLAIM: duplicate JSON keys are handled predictably.
- [ ] CLAIM: malformed slice reports file path and slice.
- [ ] CLAIM: selected import skips unselected files.
- [ ] CLAIM: selected import validates account identity before writes.
- [ ] CLAIM: unsupported archive shape fails honestly.
- [ ] CLAIM: old Twitter and newer X naming variants are covered by fixtures.

False-done traps:

- [ ] parser handles only one happy-path archive export
- [ ] parser strips wrapper with brittle string replace and accepts junk
- [ ] parser writes before identity validation
- [ ] parser silently ignores malformed selected slices

Done evidence:

- [ ] archive fixture corpus passes
- [ ] malicious archive fixture corpus passes
- [ ] selected-slice tests pass

### 13. Archive Apply

Feature claim:

> Archive slices apply idempotently into canonical rows and preserve unselected
> existing state.

Implementation checklist:

- [ ] Build archive import plan before writing.
- [ ] Validate account identity.
- [ ] Apply account/profile.
- [ ] Apply authored tweets.
- [ ] Apply note tweets.
- [ ] Apply likes as collection rows.
- [ ] Apply bookmarks as collection rows.
- [ ] Apply followers snapshot.
- [ ] Apply following snapshot.
- [ ] Apply media metadata.
- [ ] Defer DM bodies unless explicit opt-in.
- [ ] Apply through canonical write pipeline.
- [ ] Record import run.
- [ ] Record source/provenance for archive rows.
- [ ] Rebuild/update FTS.
- [ ] Project source cards only for selected/interesting rows or explicit flag.

Severe tests:

- [ ] CLAIM: re-import produces no duplicate tweets.
- [ ] CLAIM: selected bookmarks import preserves existing tweets.
- [ ] CLAIM: selected tweets import preserves existing bookmark collection rows.
- [ ] CLAIM: account mismatch aborts before writes.
- [ ] CLAIM: partial failure rolls back selected transaction.
- [ ] CLAIM: archive import performs no network or secret reads.
- [ ] CLAIM: follower partial snapshot does not generate churn events.
- [ ] CLAIM: media path cannot escape media root.
- [ ] CLAIM: import report counts match durable rows.

False-done traps:

- [ ] imported tweets but no collections
- [ ] collections without account identity
- [ ] import works only on empty database
- [ ] FTS not updated after archive import
- [ ] source-card fanout creates thousands of unwanted wiki pages by default

Done evidence:

- [ ] fixture archive round trip passes
- [ ] no-network assertion passes
- [ ] status docs describe exact supported slices

### 14. Thread Expansion

Feature claim:

> Arcwell can reconstruct local thread context from canonical tweet references,
> label missing context honestly, and avoid live lookup unless explicitly
> allowed.

Implementation checklist:

- [ ] Add local thread query by root/conversation id.
- [ ] Add local parent-walk query by `reply_to_x_id`.
- [ ] Add local descendant query.
- [ ] Add cycle detection.
- [ ] Add max-depth cap.
- [ ] Add missing-parent markers.
- [ ] Add missing-descendant markers where known.
- [ ] Add quoted-tweet inclusion option.
- [ ] Add thread node output with source/provenance.
- [ ] Add `x expand-thread`.
- [ ] Add optional live parent lookup behind policy/cost gate.
- [ ] Store live-expanded refs as `parent_walk` provenance.

Severe tests:

- [ ] CLAIM: cyclic refs cannot loop forever.
- [ ] CLAIM: max-depth cap is enforced.
- [ ] CLAIM: missing parent is labeled, not invented.
- [ ] CLAIM: quoted tweet is distinct from reply parent.
- [ ] CLAIM: live lookup never happens in `--mode local`.
- [ ] CLAIM: live lookup policy denial leaves local context intact.
- [ ] CLAIM: thread order is stable and deterministic.
- [ ] CLAIM: duplicate refs do not duplicate nodes.

False-done traps:

- [ ] thread command returns only seed tweet
- [ ] command silently fetches live data in local mode
- [ ] missing context omitted without warning
- [ ] parent/quote/retweet semantics are collapsed

Done evidence:

- [ ] thread fixture tests pass
- [ ] missing-context tests pass
- [ ] policy-denied live lookup test passes

### 15. URL And Link Index

Feature claim:

> URLs found in tweets, profiles, and later DMs are extracted, indexed, and
> optionally expanded through existing URL-safety rules without surprise
> network calls.

Implementation checklist:

- [ ] Extract URL entities from X payloads.
- [ ] Extract bare URLs only when safe and explicitly desired.
- [ ] Write `x_urls`.
- [ ] Write `x_link_occurrences`.
- [ ] Add `x links search`.
- [ ] Add `x links backfill`.
- [ ] Add `--dry-run`.
- [ ] Reuse existing URL SSRF protection.
- [ ] Reuse existing redirect limits.
- [ ] Reuse existing content-type limits.
- [ ] Reuse existing response-size limits.
- [ ] Add timeout and concurrency controls.
- [ ] Add cache TTL.
- [ ] Add source-card creation for expanded links only after safety checks.
- [ ] Add ops visibility for failed expansions.

Severe tests:

- [ ] CLAIM: link extraction never performs network.
- [ ] CLAIM: loopback URL expansion is rejected.
- [ ] CLAIM: cloud metadata URL expansion is rejected.
- [ ] CLAIM: redirect to private host is rejected.
- [ ] CLAIM: non-HTTP scheme is rejected.
- [ ] CLAIM: huge response is truncated/rejected.
- [ ] CLAIM: slow response times out.
- [ ] CLAIM: duplicate URL occurrences preserve separate source positions.
- [ ] CLAIM: hostile markdown URL text is escaped in reports.
- [ ] CLAIM: failed expansion is visible and retryable.

False-done traps:

- [ ] URL index stores only first URL per tweet
- [ ] expansion fetches URLs automatically during import
- [ ] expansion ignores redirect target safety
- [ ] link search has no provenance back to tweet/source-card

Done evidence:

- [ ] SSRF fixture tests pass
- [ ] link occurrence tests pass
- [ ] dry-run performs no network writes

### 16. X Research Briefs

Feature claim:

> `arcwell x research` turns saved or watched X material into an inspectable
> evidence pack with thread context, links, handles, source cards, wiki output,
> missing-context labels, and no fabricated claims.

Implementation checklist:

- [ ] Define research input query.
- [ ] Define default corpus: bookmarks plus watch-sourced tweets.
- [ ] Add corpus filters for bookmarks, likes, watch, search, author, date.
- [ ] Search canonical tweets through FTS.
- [ ] Rank seed tweets by provenance, recency, engagement, and optional score.
- [ ] Expand local thread context.
- [ ] Extract links.
- [ ] Extract handles.
- [ ] Collect source-card ids.
- [ ] Generate Markdown brief.
- [ ] Generate JSON envelope.
- [ ] Write wiki page when requested.
- [ ] Link brief to source cards.
- [ ] Label missing context.
- [ ] Add `--no-write`.
- [ ] Add `--out`.
- [ ] Add live thread expansion only in `--mode auto` with policy/cost gate.

Severe tests:

- [ ] CLAIM: empty evidence fails honestly.
- [ ] CLAIM: no fake citations are generated.
- [ ] CLAIM: every quoted tweet links to canonical x id.
- [ ] CLAIM: every source claim links to source-card or canonical tweet id.
- [ ] CLAIM: prompt-injection tweet text is quoted evidence.
- [ ] CLAIM: missing thread ancestor is labeled.
- [ ] CLAIM: live expansion denial still produces local brief.
- [ ] CLAIM: hostile handle/display name cannot break markdown.
- [ ] CLAIM: no-write mode writes no wiki/source-card rows.
- [ ] CLAIM: output file path cannot escape allowed filesystem behavior.

False-done traps:

- [ ] brief is just model prose over raw tweet text
- [ ] brief omits source-card ids
- [ ] missing context silently disappears
- [ ] research command fetches live data without policy/cost record
- [ ] no-write still mutates database

Done evidence:

- [ ] local research fixture passes
- [ ] no-evidence failure test passes
- [ ] prompt-injection fixture passes
- [ ] optional live expansion has separate smoke/proof

### 17. Digest Candidates And Delivery Routing

Feature claim:

> X digest candidates are durable, reviewable, source-linked, scored as
> overlays, and delivered only through explicit delivery policy and
> authorization.

Implementation checklist:

- [ ] Link digest candidates to canonical tweet/thread ids.
- [ ] Link digest candidates to source-card ids.
- [ ] Preserve current digest candidate table compatibility.
- [ ] Add candidate provenance: watch source, bookmark, search, archive.
- [ ] Add candidate status transitions.
- [ ] Add candidate score freshness.
- [ ] Add candidate dedupe by canonical entity.
- [ ] Add review list command.
- [ ] Add apply/reject command only with clear semantics.
- [ ] Route delivery through existing delivery-attempt infrastructure.
- [ ] Require policy/cost/authorization before delivery.
- [ ] Record delivery attempts.
- [ ] Add quiet-hours/schedule integration later.

Severe tests:

- [ ] CLAIM: duplicate watched tweet creates one candidate.
- [ ] CLAIM: candidate has source-card link.
- [ ] CLAIM: candidate has canonical tweet id.
- [ ] CLAIM: rejected candidate is not delivered.
- [ ] CLAIM: model score alone cannot deliver candidate.
- [ ] CLAIM: delivery policy denial creates audit, no send.
- [ ] CLAIM: Telegram/email send errors leave retryable attempt state.
- [ ] CLAIM: prompt-injection text cannot alter delivery destination/body.

False-done traps:

- [ ] digest candidate is just a markdown row with no canonical link
- [ ] delivery bypasses delivery-attempt table
- [ ] score threshold auto-sends without authorization
- [ ] duplicate candidates flood review queue

Done evidence:

- [ ] candidate dedupe tests pass
- [ ] delivery-denial tests pass
- [ ] channel delivery smoke passes only when claiming live delivery

### 18. AI Scoring Overlays

Feature claim:

> Model-backed or heuristic X scoring ranks content without mutating canonical
> truth, inventing evidence, leaking private data, or authorizing actions.

Implementation checklist:

- [ ] Add `x_scores`.
- [ ] Add deterministic heuristic scorer.
- [ ] Add optional provider-backed scorer.
- [ ] Add score kinds: interestingness, actionability, low_signal,
      digest_priority.
- [ ] Add model/prompt version.
- [ ] Add reason text.
- [ ] Add cost decision id.
- [ ] Add freshness/expiry.
- [ ] Add score invalidation on canonical content change.
- [ ] Add eval fixture corpus.
- [ ] Add command to score candidates.
- [ ] Add ops visibility for stale scores.
- [ ] Keep scores separate from canonical rows.

Severe tests:

- [ ] CLAIM: score insert never modifies tweet text/profile text.
- [ ] CLAIM: stale score is labeled stale.
- [ ] CLAIM: prompt-injection content cannot affect tool/policy behavior.
- [ ] CLAIM: provider failure creates no false score.
- [ ] CLAIM: cost record exists for provider scoring.
- [ ] CLAIM: deterministic eval catches obvious spam/low-signal cases.
- [ ] CLAIM: private/DM content is excluded unless explicitly enabled.
- [ ] CLAIM: score is not enough to trigger delivery.

False-done traps:

- [ ] score overwrites candidate status
- [ ] model output accepted without schema validation
- [ ] no eval corpus
- [ ] no cost record
- [ ] stale score displayed as current

Done evidence:

- [ ] heuristic eval gate passes
- [ ] provider-backed scoring smoke passes only when live quality is claimed
- [ ] scoring docs say overlay, not truth

### 19. Ops UI And Doctor

Feature claim:

> Operators can tell whether X is healthy, stale, blocked, partial, rate
> limited, projection-broken, credential-broken, or untested without reading raw
> SQLite or guessing from command output.

Implementation checklist:

- [ ] Add X counts to ops snapshot.
- [ ] Add canonical-vs-compatibility counts.
- [ ] Add FTS health.
- [ ] Add latest sync runs.
- [ ] Add failed sync runs.
- [ ] Add source health.
- [ ] Add watch-source health.
- [ ] Add credential health.
- [ ] Add cursor state.
- [ ] Add projection failures.
- [ ] Add digest candidate counts.
- [ ] Add archive import runs.
- [ ] Add portable export freshness.
- [ ] Add stale-state summary.
- [ ] Add doctor warnings for stale/failed X monitors.
- [ ] Add doctor warnings for expired/missing user-context tokens when X
      monitors are configured.
- [ ] Add doctor warning for FTS drift.
- [ ] Add doctor warning for projection failure backlog.

UI checklist:

- [ ] Add X section to `/ops/ui`.
- [ ] Add filters for status/source/age.
- [ ] Add detail drawer or detail rows.
- [ ] Add links to source cards/wiki pages.
- [ ] Add redacted errors.
- [ ] Add freshness timestamps.
- [ ] Add no-overlap desktop layout.
- [ ] Add narrow/mobile layout.
- [ ] Add empty state.
- [ ] Add partial/live-unproven labels.

Severe tests:

- [ ] CLAIM: hostile tweet text is escaped in ops UI.
- [ ] CLAIM: hostile profile display name is escaped in ops UI.
- [ ] CLAIM: token-like error text is redacted.
- [ ] CLAIM: stale monitor state is visibly stale.
- [ ] CLAIM: failed projection backlog is visible.
- [ ] CLAIM: authenticated POST controls reject missing auth.
- [ ] CLAIM: POST controls reject hostile origin.
- [ ] CLAIM: POST controls require idempotency where mutation can repeat.
- [ ] CLAIM: browser desktop smoke has no overlap/clipping.
- [ ] CLAIM: browser mobile smoke has no overlap/clipping.

False-done traps:

- [ ] ops shows only row counts, no failure state
- [ ] source-health failures not tied to watch source
- [ ] stale state looks healthy
- [ ] UI action works only by unprotected POST
- [ ] no browser validation after UI change

Done evidence:

- [ ] ops snapshot tests pass
- [ ] ops UI XSS tests pass
- [ ] browser smoke artifacts captured
- [ ] strict doctor tests pass

### 20. Portable Export, Import, And Backup

Feature claim:

> X data can be exported to deterministic, Git-friendly, token-free JSONL
> shards, validated independently, and imported into a disposable home without
> losing provenance.

Implementation checklist:

- [ ] Add export manifest.
- [ ] Add schema version.
- [ ] Export accounts.
- [ ] Export profiles.
- [ ] Export profile snapshots.
- [ ] Export tweets by year.
- [ ] Export unknown-date tweets.
- [ ] Export tweet refs.
- [ ] Export tweet edges.
- [ ] Export collections.
- [ ] Export follow snapshots.
- [ ] Export follow edges.
- [ ] Export follow events.
- [ ] Export URLs.
- [ ] Export media metadata.
- [ ] Export scores.
- [ ] Export projections.
- [ ] Exclude FTS rows.
- [ ] Exclude sync cache by default.
- [ ] Exclude OAuth secrets.
- [ ] Exclude raw DMs by default.
- [ ] Add hash and row count per shard.
- [ ] Add validate command.
- [ ] Add import command.
- [ ] Add disposable restore drill.

Severe tests:

- [ ] CLAIM: token-like values are absent from default export.
- [ ] CLAIM: manifest hash mismatch fails validation.
- [ ] CLAIM: row count mismatch fails validation.
- [ ] CLAIM: malformed JSONL fails validation.
- [ ] CLAIM: import is idempotent.
- [ ] CLAIM: import preserves source-card/projection references where possible.
- [ ] CLAIM: import rejects path traversal.
- [ ] CLAIM: DMs excluded by default.
- [ ] CLAIM: FTS/cache rows excluded by default.
- [ ] CLAIM: disposable restore drill can search imported tweets.

False-done traps:

- [ ] export is just a SQLite copy
- [ ] export includes tokens or raw DMs
- [ ] validate only checks manifest exists
- [ ] import loses provenance
- [ ] no restore drill

Done evidence:

- [ ] export/validate/import tests pass
- [ ] secret scan test passes
- [ ] disposable restore drill passes

### 21. Follow Graph And Identity

Feature claim:

> Followers/following are represented as snapshots, current edges, and events,
> with partial snapshots unable to create false churn.

Implementation checklist:

- [ ] Add follow snapshot write path.
- [ ] Add follow member write path.
- [ ] Add current edge reconciliation.
- [ ] Add event generation for complete snapshots.
- [ ] Add partial snapshot behavior.
- [ ] Add graph summary.
- [ ] Add graph events.
- [ ] Add mutuals/non-mutual queries later.
- [ ] Add profile entity extraction from bio/url.
- [ ] Add identity search helper.
- [ ] Keep full following import out of default watch seed.
- [ ] Add ops graph freshness.

Severe tests:

- [ ] CLAIM: complete new snapshot creates started events.
- [ ] CLAIM: complete missing member creates ended event.
- [ ] CLAIM: duplicate snapshot creates no duplicate events.
- [ ] CLAIM: partial snapshot creates no ended events.
- [ ] CLAIM: malformed profile in snapshot is rejected or quarantined.
- [ ] CLAIM: hostile profile bio remains data.
- [ ] CLAIM: graph command is account-scoped.
- [ ] CLAIM: watch rebuild does not silently switch to full graph.

False-done traps:

- [ ] follower list stored but no history
- [ ] partial snapshot treated as complete
- [ ] events duplicate on repeated import
- [ ] graph lacks account scope

Done evidence:

- [ ] graph reconciliation tests pass
- [ ] archive follower/following fixture passes
- [ ] docs warn full following import is not default watch seed

### 22. Media Cache

Feature claim:

> X media metadata and optional media bytes are local, bounded, provenance
> linked, and never fetched unexpectedly.

Implementation checklist:

- [ ] Add media metadata mapper from API payload.
- [ ] Add archive media metadata mapper.
- [ ] Add media table writes.
- [ ] Add local media root under `ARCWELL_HOME`.
- [ ] Add archive media extraction later.
- [ ] Add thumbnail generation only if needed.
- [ ] Add live media fetch command with explicit confirmation/flags.
- [ ] Add size limit.
- [ ] Add content-type validation.
- [ ] Add pacing.
- [ ] Add retry budget.
- [ ] Add dry-run.
- [ ] Add ops media cache stats.
- [ ] Add portable export metadata but not bytes by default.

Severe tests:

- [ ] CLAIM: import stores metadata without fetching bytes.
- [ ] CLAIM: archive media path traversal is rejected.
- [ ] CLAIM: local media path cannot escape media root.
- [ ] CLAIM: live fetch rejects huge file.
- [ ] CLAIM: live fetch rejects unexpected content type.
- [ ] CLAIM: live fetch obeys retry budget.
- [ ] CLAIM: dry-run performs no writes.
- [ ] CLAIM: media export excludes bytes by default.

False-done traps:

- [ ] media URL present but no metadata table
- [ ] live fetch runs during import
- [ ] archive extraction trusts entry paths
- [ ] no size limit

Done evidence:

- [ ] media metadata tests pass
- [ ] path traversal tests pass
- [ ] live fetch remains opt-in in docs

### 23. DMs

Feature claim:

> DMs are imported only with explicit retention opt-in, searched locally only
> when enabled, and excluded from default exports/backups unless deliberately
> configured.

Implementation checklist:

- [ ] Add retention config.
- [ ] Add explicit import flag.
- [ ] Add `x_dm_conversations`.
- [ ] Add `x_dm_events`.
- [ ] Add `x_dm_participants`.
- [ ] Add `x_dm_payloads` if needed.
- [ ] Add `x_dms_fts`.
- [ ] Add archive DM parser.
- [ ] Add DM profile reconciliation.
- [ ] Add search command.
- [ ] Add ops label for enabled/disabled.
- [ ] Exclude DMs from portable export by default.
- [ ] Add redacted export option.
- [ ] Add forget/retention story before done.

Severe tests:

- [ ] CLAIM: DM archive slice ignored unless opt-in.
- [ ] CLAIM: default export excludes DMs.
- [ ] CLAIM: DM prompt injection remains data.
- [ ] CLAIM: participant profiles are account-scoped.
- [ ] CLAIM: malformed DM event fails without corrupting conversation.
- [ ] CLAIM: FTS does not include DMs when disabled.
- [ ] CLAIM: ops shows DM disabled by default.
- [ ] CLAIM: forget/delete removes DM-derived FTS rows.

False-done traps:

- [ ] parser exists but default privacy undefined
- [ ] DMs exported by accident
- [ ] DM text enters model scoring without opt-in
- [ ] delete forgets FTS/cache rows

Done evidence:

- [ ] opt-in tests pass
- [ ] export exclusion tests pass
- [ ] retention/forget tests pass

### 24. Moderation And Social Writes

Feature claim:

> Block, mute, reply, and post actions are impossible without explicit
> account-scoped confirmation, policy approval, audit rows, and reconciliation
> between local pending state and remote result.

Implementation checklist:

- [ ] Defer implementation until read substrate is operational.
- [ ] Add local block/mute tables.
- [ ] Add pending action table.
- [ ] Add remote action adapter seam.
- [ ] Add account identity confirmation.
- [ ] Add target profile/tweet resolution.
- [ ] Add exact action preview.
- [ ] Add confirmation flow.
- [ ] Add policy approval flow.
- [ ] Add audit row before remote write.
- [ ] Add remote result reconciliation.
- [ ] Add failure state.
- [ ] Add retry rules.
- [ ] Add ops visibility.
- [ ] Add no default automation.

Severe tests:

- [ ] CLAIM: action without confirmation is rejected.
- [ ] CLAIM: action with wrong account is rejected.
- [ ] CLAIM: policy denial prevents remote call.
- [ ] CLAIM: remote failure leaves pending/failed local state.
- [ ] CLAIM: success reconciles local state.
- [ ] CLAIM: retry is idempotent.
- [ ] CLAIM: target spoofing via handle/display name cannot redirect action.
- [ ] CLAIM: prompt-injection tweet cannot request action.
- [ ] CLAIM: MCP tool cannot perform write without approval.
- [ ] CLAIM: audit row redacts secrets and stores target/action.

False-done traps:

- [ ] local block row inserted but remote write never happened
- [ ] remote write happened but no audit row
- [ ] confirmation text omits account/target
- [ ] action can be triggered through MCP hidden path

Done evidence:

- [ ] local fake-adapter tests pass
- [ ] approval boundary tests pass
- [ ] live write smoke only with disposable target and explicit confirmation

### 25. Worker And Scheduled Sync

Feature claim:

> Scheduled X jobs run through the same guarded sync paths as CLI, with bounded
> attempts, source health, cost records, and no silent death.

Implementation checklist:

- [ ] Add job kinds for bounded X sync.
- [ ] Validate job input.
- [ ] Policy guard before enqueue when appropriate.
- [ ] Policy guard before execution.
- [ ] Cost reservation before execution.
- [ ] Worker records heartbeat.
- [ ] Worker records source health.
- [ ] Worker records sync run.
- [ ] Retry with backoff.
- [ ] Dead-letter after max attempts.
- [ ] Ops shows failed/dead-lettered X jobs.
- [ ] No unbounded watch-source loops.
- [ ] No default job without explicit config.

Severe tests:

- [ ] CLAIM: unknown X job kind rejected.
- [ ] CLAIM: malformed job input fails before provider call.
- [ ] CLAIM: policy denial marks job failed/deferred without credentials.
- [ ] CLAIM: quota failure preserves cursor and releases budget.
- [ ] CLAIM: retry storm cannot overspend.
- [ ] CLAIM: dead-letter visible in ops.
- [ ] CLAIM: worker and CLI share implementation path.
- [ ] CLAIM: missed heartbeat appears in doctor/ops.

False-done traps:

- [ ] CLI works but worker uses separate weaker path
- [ ] failed worker job only logs to stderr
- [ ] retry loop burns cost
- [ ] source health not updated from worker

Done evidence:

- [ ] worker run-once tests pass
- [ ] strict doctor tests pass
- [ ] live scheduled behavior only claimed after real service proof

### 26. CLI, MCP, Slash Commands, And Docs Parity

Feature claim:

> Agent-facing X behavior is consistent across CLI, MCP, slash commands,
> skills, README/package docs, and source-health resources.

Implementation checklist:

- [ ] Update CLI command.
- [ ] Update MCP tool only if agent-useful.
- [ ] Update MCP schema.
- [ ] Update MCP resource if state should be inspectable.
- [ ] Update plugin slash command.
- [ ] Update `x-research` skill.
- [ ] Update `packages/arcwell-x/README.md`.
- [ ] Update `docs/functionality-and-packages.md`.
- [ ] Update `docs/codex-plugin-commands.md`.
- [ ] Update `STATUS.md`.
- [ ] Update `TODO.md`.
- [ ] Run plugin docs verifier.
- [ ] Run dev plugin smoke/sync when plugin changes.

Severe tests:

- [ ] CLAIM: CLI and MCP return equivalent result for same fixture.
- [ ] CLAIM: MCP schema rejects invalid args.
- [ ] CLAIM: slash command points to the correct MCP tool.
- [ ] CLAIM: docs verifier catches missing command/tool entry.
- [ ] CLAIM: skill preserves untrusted-source warning.
- [ ] CLAIM: tool count increase is justified by workflow.

False-done traps:

- [ ] CLI implemented but MCP stale
- [ ] MCP implemented but slash docs stale
- [ ] README claims live capability unproven
- [ ] skill tells agent to use deprecated tool

Done evidence:

- [ ] MCP round-trip tests pass
- [ ] docs verifier passes
- [ ] `scripts/arcwell-dev smoke` passes if plugin changed
- [ ] `scripts/arcwell-dev sync` passes if plugin changed

### 27. Policy And Cost

Feature claim:

> Every X network, model, mutation, delivery, secret-admin, worker enqueue, and
> live-probe path is policy-checked before side effects and cost-checked before
> paid/provider work.

Implementation checklist:

- [ ] Inventory X provider network paths.
- [ ] Inventory model scoring paths.
- [ ] Inventory delivery paths.
- [ ] Inventory social write paths.
- [ ] Inventory worker enqueue paths.
- [ ] Inventory secret admin paths.
- [ ] Inventory archive local file paths.
- [ ] Add policy guard before credential lookup/network.
- [ ] Add cost reservation before provider/model call.
- [ ] Add release on classified provider failure.
- [ ] Add cost decision id to sync/scoring/delivery rows.
- [ ] Add ops visibility.
- [ ] Add tests for denied policy.
- [ ] Add tests for approval-required policy.
- [ ] Add tests for kill switch.

Severe tests:

- [ ] CLAIM: denied `x_recent_search` reads no token and makes no network call.
- [ ] CLAIM: denied bookmark sync reads no token and makes no network call.
- [ ] CLAIM: denied model scoring makes no provider call.
- [ ] CLAIM: denied delivery sends nothing.
- [ ] CLAIM: denied social write records audit but no remote call.
- [ ] CLAIM: cost cap blocks before provider call.
- [ ] CLAIM: quota failure releases reservation where designed.
- [ ] CLAIM: retry storm cannot overspend package budget.
- [ ] CLAIM: temporary override expiry is enforced.

False-done traps:

- [ ] policy applied after token lookup
- [ ] cost estimated but not reserved atomically
- [ ] denied path still mutates cursor/source health as success
- [ ] model scorer bypasses cost gates

Done evidence:

- [ ] policy severe tests pass
- [ ] cost severe tests pass
- [ ] ops shows blocked decisions

### 28. Secrets And Privacy

Feature claim:

> X secrets, private data, tokens, DMs, local paths, and provider errors do not
> leak through CLI, MCP, logs, ops, source cards, wiki pages, exports, backups,
> tests, or model prompts.

Implementation checklist:

- [ ] Maintain no `secret_value_get` MCP tool.
- [ ] Redact token-like text everywhere.
- [ ] Mark local secret presence in health only.
- [ ] Avoid writing secrets to source cards/wiki.
- [ ] Avoid writing secrets to sync runs.
- [ ] Avoid writing secrets to job errors.
- [ ] Avoid writing secrets to ops UI.
- [ ] Avoid writing secrets to portable export.
- [ ] Add token scanner to export tests.
- [ ] Add prompt payload audit for model scoring.
- [ ] Keep DMs default-off.
- [ ] Keep real local config ignored.
- [ ] Keep tracked docs using placeholders.

Severe tests:

- [ ] CLAIM: CLI secret-health output contains no token values.
- [ ] CLAIM: MCP resources contain no token values.
- [ ] CLAIM: source-health errors are redacted.
- [ ] CLAIM: sync-run errors are redacted.
- [ ] CLAIM: portable export contains no token-like values.
- [ ] CLAIM: source-card/wiki pages contain no secret values.
- [ ] CLAIM: model scoring prompt excludes DMs unless enabled.
- [ ] CLAIM: local ignored config is not referenced by tracked docs.

False-done traps:

- [ ] redaction applied to stdout but not database error rows
- [ ] export excludes secrets but includes raw provider auth payload
- [ ] model prompt includes private raw blobs
- [ ] tests use real tokens in fixtures

Done evidence:

- [ ] redaction tests pass
- [ ] export secret-scan passes
- [ ] docs contain placeholders only

### 29. Performance And Resource Limits

Feature claim:

> X import, search, sync, archive parsing, projection, export, and UI remain
> bounded under large local archives and hostile inputs.

Implementation checklist:

- [ ] Define max archive size.
- [ ] Define max archive file count.
- [ ] Define max JSON slice size.
- [ ] Define max tweet text length via existing validation.
- [ ] Define max profile description length.
- [ ] Define max raw JSON stored length or compression strategy.
- [ ] Define sync page caps.
- [ ] Define worker max jobs.
- [ ] Define URL expansion concurrency.
- [ ] Define media fetch concurrency.
- [ ] Define FTS rebuild transaction/chunking.
- [ ] Define export chunking.
- [ ] Add benchmark or stress smoke for large fixture.
- [ ] Add cancellation or timeout where applicable.

Severe tests:

- [ ] CLAIM: archive bomb rejected before memory blowup.
- [ ] CLAIM: huge JSON slice rejected or streamed safely.
- [ ] CLAIM: repeated duplicate import does not grow unbounded.
- [ ] CLAIM: FTS rebuild handles large fixture within budget.
- [ ] CLAIM: export handles large fixture without loading all rows at once
      where practical.
- [ ] CLAIM: URL expansion concurrency cap is enforced.
- [ ] CLAIM: worker max source/result caps are enforced.
- [ ] CLAIM: UI limits row rendering and remains responsive.

False-done traps:

- [ ] feature works only for tiny fixtures
- [ ] export builds all rows in memory
- [ ] archive parser reads whole zip without limits
- [ ] UI tries to render every tweet row

Done evidence:

- [ ] resource-limit tests pass
- [ ] stress fixture result documented
- [ ] perf caveats listed if not fully optimized

### 30. Live Proof And Smoke Discipline

Feature claim:

> Live behavior is proven by the exact integration being claimed, with fresh
> binaries, isolated/disposable state, redacted artifacts, and no cross-smoke
> queue interference.

Implementation checklist:

- [ ] Rebuild binary before live CLI smoke.
- [ ] Use disposable `ARCWELL_HOME` where possible.
- [ ] Use copied user-context source home for X user-context proof.
- [ ] Unset env app bearer when copied user-context proof must be tested.
- [ ] Preserve artifacts with secrets redacted.
- [ ] Record exact command.
- [ ] Record exact provider scopes required.
- [ ] Record whether app-only or user-context token was used.
- [ ] Run queue-sensitive smokes sequentially.
- [ ] Avoid draining unrelated live traffic.
- [ ] Add retry/wait where provider propagation delay is normal.
- [ ] Update `STATUS.md` with exact live proof and limitation.

Severe live proof cases:

- [ ] recent search with app/user token as claimed
- [ ] bookmark import with user-context token
- [ ] definitive watch rebuild with user-context token
- [ ] watch-source monitor with user-context token
- [ ] source-health after forced/observed provider failure
- [ ] copied-home smoke does not mutate real home
- [ ] live smoke does not print token values

False-done traps:

- [ ] live smoke used stale binary
- [ ] live smoke used env app bearer while claiming user-context proof
- [ ] live smoke mutated real watch list accidentally
- [ ] live smoke passed local replay but live section was skipped
- [ ] edge/Telegram queue smokes run in parallel and interfere

Done evidence:

- [ ] script output recorded
- [ ] artifacts retained/redacted
- [ ] source-health/cursor state inspected
- [ ] status docs updated with exact scope

## Phase Exit Checklist

Before any phase is marked complete:

- [ ] Claim ledger completed.
- [ ] Implementation checklist completed or remaining items explicitly split
      into later phase.
- [ ] Severe tests added for realistic failure modes.
- [ ] Tests fail on the intended broken/scaffold behavior or otherwise refute a
      plausible broken implementation.
- [ ] Targeted tests pass.
- [ ] Broad Rust tests pass.
- [ ] CLI surface verified.
- [ ] MCP surface verified if exposed.
- [ ] Plugin/slash docs verified if changed.
- [ ] Ops/doctor/source-health visibility added where relevant.
- [ ] Live smoke run when external behavior is claimed.
- [ ] Backup/export/privacy behavior decided.
- [ ] `STATUS.md` updated.
- [ ] `TODO.md` updated.
- [ ] Package README updated.
- [ ] Remaining risk stated.
- [ ] Adversarial review completed.
- [ ] No known false-done trap remains unaddressed.

## Adversarial Review Report Template

Use this template after each substantial phase.

```text
PHASE:

SCOPE REVIEWED:

CLAIMS REVIEWED:

EVIDENCE INSPECTED:

COMMANDS RUN:

FINDINGS:
- [score] [severity] file:line - finding with evidence

UNTESTED RISKS:
- risk, why it remains untested, what would test it

FALSE-DONE TRAPS CHECKED:
- checked trap and outcome

ROOT-CAUSE NOTES:
- any failure and actual cause

REQUIRED FIXES BEFORE STATUS CAN ADVANCE:
- fix

STATUS RECOMMENDATION:
- Missing | Scaffold | Partial | Local Proof | Live Proof | Operational | Done
```

Finding score:

- [ ] 0: inapplicable, do not report
- [ ] 25: speculative, list as untested risk only
- [ ] 50: reproduced under contrived conditions
- [ ] 75: reliably reproduced under realistic local conditions
- [ ] 100: demonstrated in real runtime/live environment

Do not promote a finding to a bug unless the evidence reaches at least 50.
Do not promote a fixed behavior to done unless the regression test would have
caught the broken behavior.

## Test Naming Convention

Tests that guard against mirages should make the claim visible in the name.

Examples:

- [ ] `severe_x_migration_preserves_legacy_x_items_projection`
- [ ] `severe_x_canonical_write_rolls_back_fts_on_failure`
- [ ] `severe_x_recent_search_malformed_item_preserves_cursor`
- [ ] `severe_x_bookmark_sync_duplicate_page_early_stops_without_cursor_loss`
- [ ] `severe_x_archive_rejects_zip_slip_before_writes`
- [ ] `severe_x_research_brief_refuses_no_evidence`
- [ ] `severe_x_export_excludes_tokens_and_raw_dms_by_default`
- [ ] `severe_x_ops_ui_escapes_profile_and_tweet_text`
- [ ] `severe_x_social_write_requires_account_scoped_approval`

Each severe test should include a nearby comment block:

```text
CLAIM:
PRECONDITIONS:
POSTCONDITIONS:
ORACLE:
SEVERITY:
```

## Open Decisions That Must Stay Visible

These are not implementation blockers for Phase 1, but they must not disappear.

- [ ] Whether `x_items` remains permanently as a projection table or is later
      replaced by a view/read model.
- [ ] Whether archive media bytes are extracted by default or only metadata is
      imported.
- [ ] Whether DMs are ever included in normal backup, or only in explicit
      encrypted/private export.
- [ ] Whether `xurl` and `bird` adapters are worth adding after X API/archive
      paths mature.
- [ ] Whether social writes belong in Arcwell at all before broader approval UX
      is built.
- [ ] Whether X digest delivery should be automatic on a schedule or always
      review-first.
- [ ] Whether profile identity extraction should be deterministic only or use a
      model-backed scorer later.
- [ ] Whether portable X export becomes part of scheduled backup or remains a
      separate explicit command.

## Non-Negotiable Stop Conditions

Stop and reassess instead of pushing forward if any of these happen:

- [ ] A migration loses or duplicates existing `x_items` behavior.
- [ ] A live sync advances cursor before durable accepted writes.
- [ ] A provider error leaks token-like text.
- [ ] Archive import writes before account identity validation.
- [ ] Projection failure hides canonical evidence.
- [ ] A model-generated digest includes unsupported claims.
- [ ] A social write can occur without exact account/target confirmation.
- [ ] DMs enter export/model prompts without explicit opt-in.
- [ ] Ops says healthy while source-health or sync-run state is stale/failed.
- [ ] Tests are weakened to pass a known broken behavior.
- [ ] Documentation claims live proof that was not actually run.

If a stop condition is hit:

- [ ] record the failing command/artifact
- [ ] classify root cause
- [ ] add a regression test
- [ ] fix the root cause
- [ ] rerun targeted and broad gates
- [ ] update remaining risk

## First Three PRs

### PR 1: Canonical Schema And Dual Write

Scope:

- [ ] schema
- [ ] migration/backfill
- [ ] canonical write structs
- [ ] `insert_x_item` dual-write
- [ ] FTS table and rebuild command
- [ ] compatibility tests

Explicitly out of scope:

- [ ] archive import
- [ ] live bookmark pagination changes
- [ ] ops UI controls
- [ ] AI scoring
- [ ] media bytes
- [ ] DMs
- [ ] social writes

Merge gate:

- [ ] severe migration tests pass
- [ ] severe canonical write tests pass
- [ ] FTS tests pass
- [ ] old CLI/MCP fixtures pass
- [ ] `cargo test --all --all-features` passes
- [ ] docs say this is canonical local proof, not full Birdclaw parity

### PR 2: Canonical Live Bookmark/Search/Monitor

Scope:

- [ ] recent search canonical writes
- [ ] bookmark sync canonical writes
- [ ] monitor canonical writes
- [ ] sync runs
- [ ] source health
- [ ] cursor safety
- [ ] early-stop for bookmarks
- [ ] canonical reads for list/bookmarks/report

Explicitly out of scope:

- [ ] archive import
- [ ] model scoring
- [ ] delivery
- [ ] DMs
- [ ] social writes

Merge gate:

- [ ] severe recent-search tests pass
- [ ] severe bookmark tests pass
- [ ] severe monitor tests pass
- [ ] copied-home X live smoke passes when credentials are available
- [ ] source-health failure path visible
- [ ] status docs list live scope exactly

### PR 3: Archive Import MVP

Scope:

- [ ] archive discovery
- [ ] archive reader
- [ ] account/profile/tweet parser
- [ ] likes/bookmarks parser
- [ ] followers/following parser
- [ ] selected import
- [ ] canonical apply
- [ ] no-network proof

Explicitly out of scope:

- [ ] raw DMs
- [ ] archive media byte extraction
- [ ] social writes
- [ ] model scoring

Merge gate:

- [ ] archive fixture corpus passes
- [ ] malicious archive corpus passes
- [ ] selected import tests pass
- [ ] no network/secret assertion passes
- [ ] FTS search finds archive-imported bookmark
- [ ] docs clearly state supported archive slices
