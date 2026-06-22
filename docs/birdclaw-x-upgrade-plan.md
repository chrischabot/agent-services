# Birdclaw Lessons For Arcwell X

Date: 2026-06-22

Birdclaw source reviewed:

- Site: <https://birdclaw.sh/>
- Repository: <https://github.com/steipete/birdclaw>
- Local checkout: `/tmp/birdclaw`
- Commit reviewed: `10f98d3fb36ae6406b46566b52ee88b14b34ce5e`

## Executive Take

Birdclaw is not just a richer X importer. It is a local Twitter memory and
operator console:

- one canonical SQLite model for tweets, profiles, DMs, follows, collections,
  moderation state, cursors, sync runs, cache rows, media, and FTS indexes
- archive-first import that converges with live sync into the same tables
- transport adapters for `xurl` and `bird`, with the adapters kept outside the
  core truth model
- local web lanes for home, mentions, bookmarks, likes, DMs, inbox, links,
  blocks, network map, and digests
- AI overlays for inbox scoring and period digests, without mutating the
  canonical source records
- deterministic Git-friendly text backups
- stable scriptable CLI output, with diagnostics on stderr and parseable JSON
  on stdout

Arcwell X is currently a source-evidence pipeline: import JSON or live X API
results into `x_items`, source cards, wiki pages, digest candidates, watch
sources, cursors, source health, cost/policy records, and secret health. That is
useful and safer than a naive scraper, but it is not yet a local Twitter memory.

The right move is not to copy Birdclaw wholesale. The right move is to promote
Arcwell X from "tweet-shaped source cards" to a normalized local social-data
substrate that still feeds Arcwell's wiki, research, digest, policy, and channel
systems.

## What Birdclaw Gets Right

### 1. Canonical Domain Model

Birdclaw's most important invariant is that every source maps into one
normalized model. Archive rows, live `xurl` results, `bird` GraphQL/cookie
results, and UI actions all land in the same tables. Raw payloads may be kept,
but they do not define separate primary worlds.

The reviewed schema includes:

- `accounts`
- `profiles`
- `profile_affiliations`
- `profile_snapshots`
- `profile_bio_entities`
- `identity_search_index`
- `tweets`
- `tweet_collections`
- `tweet_account_edges`
- `dm_conversations`
- `dm_messages`
- `blocks`
- `mutes`
- `ai_scores`
- `sync_cache`
- `url_expansions`
- `link_occurrences`
- `follow_snapshots`
- `follow_snapshot_members`
- `follow_edges`
- `follow_events`
- `tweets_fts`
- `dm_fts`

Arcwell currently has:

- `x_items`
- `x_item_sources`
- `watch_sources`
- `source_health`
- `cursors`
- source cards
- wiki pages
- digest candidates

That is enough for evidence ingestion and reporting, but not enough for local
Twitter memory, thread reconstruction, profile-aware triage, media, DMs, or
follow graph history.

### 2. Archive-First, Live-Aware Sync

Birdclaw treats the user archive as the best initial truth source and live sync
as a gap-filler and freshness layer. This matters because the public X API has
tier, quota, and scope limits. The archive gives bulk history without API spend.

Birdclaw supports:

- archive discovery on macOS
- selective archive re-imports
- archive import for tweets, likes, bookmarks, profiles, followers, following,
  DMs, and media
- live sync for authored tweets, likes, bookmarks, home timeline, mentions,
  mention threads, followers/following, and DMs
- cursor-aware, cache-aware, resumable sync

Arcwell should borrow this as a convergence principle:

> Archive import and live sync must write the same canonical X tables, and
> source-card/wiki generation should be a projection from canonical rows.

### 3. Collections And Edges Instead Of Boolean Flags

Birdclaw moved from simple tweet state toward collection and account-edge
tables. This is the right shape for multi-account support and repeated live
observations:

- a tweet can be authored, home-timeline, mentioned, liked, bookmarked, searched,
  and watched for one or more accounts
- source provenance belongs on edges and collection rows, not only on the tweet
  body
- repeated observations can refresh `last_seen_at` without duplicating tweets

Arcwell already has `x_item_sources`, which is the seed of this idea. The
upgrade is to make it first-class:

- `x_tweets` as canonical tweet rows
- `x_profiles` as canonical profile rows
- `x_tweet_edges` for account/source stream membership
- `x_collections` for bookmarks/likes
- `x_observations` or source-specific edge metadata for provenance

### 4. Full-Text Search Day One

Birdclaw uses FTS5 for tweets and DMs. Arcwell X currently uses `LIKE` across
`x_id`, `author`, `text`, and `url`. That is fine for a small source-card list,
but it will not hold up once bookmarks, follows, timeline, mentions, archive
history, and DMs exist.

Arcwell should add:

- `x_tweets_fts`
- likely `x_dms_fts` later
- rebuild/repair command
- tests for punctuation-heavy and URL-heavy queries
- migration coverage from existing `x_items`

### 5. Sync Cache And Early Stop

Birdclaw's sync commands cache transport responses and stop paginating when a
page is already locally known. This is the practical API-cost lesson.

Arcwell already has policy/cost gates, source health, and cursor safety. It
should add Birdclaw's dedupe-saturation behavior:

- stop a paged bookmark/like/following sync when a fetched page imports zero
  new canonical rows
- record a visible saturation reason
- keep a `--max-pages` cap for scheduled runs
- let `--refresh` bypass caches deliberately

### 6. Research From Saved Social Material

Birdclaw's `research` mode is bookmark-driven. It finds matching saved tweets,
expands their threads, extracts links/handles, and emits Markdown or JSON.

This maps directly to Arcwell's stronger pieces:

- source cards
- wiki pages
- digest candidates
- deep research runs
- evidence audit

Arcwell should build an `x research` workflow that:

1. searches bookmarks or watched-source tweets locally
2. expands thread context from local canonical tweets first
3. optionally uses live thread lookup only for missing ancestors
4. creates source cards for external links and key tweet/thread evidence
5. produces a wiki-backed evidence pack, not just stdout prose
6. can queue deep research over the extracted links

### 7. AI As Overlay, Not Truth

Birdclaw stores OpenAI scores as overlays in `ai_scores`; raw mentions, tweets,
and DMs remain canonical. This is exactly the posture Arcwell should keep.

For Arcwell:

- interestingness, relevance, actionability, spam/low-signal, and digest scores
  should be overlay rows
- score rows should include model, prompt/policy version, reason, freshness, and
  cost decision id
- score expiry should be visible in ops/source health
- canonical tweet/profile/DM rows should not be rewritten by model judgment

### 8. Local Media And URL Cache

Birdclaw stores media originals, archive-extracted bytes, avatars, URL
expansions, link occurrences, and preview metadata. Arcwell currently preserves
tweet URL and metrics/raw JSON, but does not have a social media/link cache.

This is a strong upgrade path because media and links are often the actual
research payload:

- cache tweet media metadata first, bytes later
- reuse archive media bytes before CDN fetches
- extract URL occurrences into a searchable link index
- add SSRF-safe expansion with timeout, content-type, and size limits
- create source cards from expanded links only after safety checks

### 9. Local Web Lanes

Birdclaw's web app is not incidental. It turns the local database into a daily
workflow: home, mentions, likes, bookmarks, DMs, inbox, blocks, links, network
map, and digests.

Arcwell should not build a separate large product surface immediately. It
already has `/ops/ui` and wiki/source-card views. The useful first UI slice is:

- `X Sources` lane in ops or a small local HTML route
- watch-source status, last success/failure, cursor, next run
- recent accepted tweets with source-card links
- bookmark/watch provenance filters
- digest candidate review state
- credential/rate-limit health

### 10. Git-Friendly Backups

Birdclaw can export deterministic text shards:

- yearly tweet JSONL
- profile JSONL
- collections
- DMs
- moderation lists
- manifest validation
- Git commit/push support

Arcwell has backup support for SQLite/wiki/memory artifacts. It should add an
optional portable export for X/social data:

- `data/x/tweets/YYYY.jsonl`
- `data/x/profiles.jsonl`
- `data/x/collections/bookmarks.jsonl`
- `data/x/follow_edges.jsonl`
- `data/x/source_cards.jsonl` or references into existing source-card export
- no tokens, no transient cache, no FTS shadow rows

## Where Arcwell Is Already Stronger

Arcwell should preserve these advantages:

- policy gates before network/provider/token work
- estimated cost reservations and kill switches
- redacted secret health and no MCP `secret_value_get`
- source cards and wiki pages as inspectable evidence
- prompt-injection text treated as untrusted source data
- severe tests around quota, expired tokens, malformed X payloads, partial
  provider errors, duplicate cursors, unsafe URLs, and digest candidate flow
- watch-source monitor imports creating source cards and digest candidates
- provider/source health visible in ops snapshots

Birdclaw is product-richer. Arcwell is already more explicit about policy,
cost, provenance, and agent trust boundaries. The upgrade should combine those,
not replace one with the other.

## Proposed Arcwell X Target Shape

### Storage

Add canonical X tables instead of expanding `x_items` forever:

- `x_accounts`
- `x_profiles`
- `x_profile_snapshots`
- `x_tweets`
- `x_tweet_edges`
- `x_collections`
- `x_threads`
- `x_urls`
- `x_media`
- `x_sync_runs`
- `x_sync_cursors` or typed cursor rows layered over existing `cursors`
- `x_scores`
- `x_tweets_fts`

Keep compatibility views or adapters:

- existing `x_items` should remain readable during migration
- new imports should write canonical rows first
- `x_items` can become a projection for source-card-era callers

### Commands

Near-term command tree:

```text
arcwell x import-json <path>
arcwell x import-archive [path] [--select tweets,likes,bookmarks,profiles,followers,following,dms]
arcwell x sync bookmarks [--max-pages N] [--early-stop] [--refresh]
arcwell x sync likes [--max-pages N] [--early-stop] [--refresh]
arcwell x sync mentions [--max-pages N] [--refresh]
arcwell x sync watch-sources [--max-sources N] [--max-results-per-source N]
arcwell x search tweets <query> [--bookmarked] [--liked] [--author HANDLE] [--since DATE]
arcwell x research <query> [--bookmarks] [--thread-depth N] [--out PATH]
arcwell x digest [today|24h|week]
arcwell x graph summary
arcwell x export-portable --out <dir>
arcwell x rebuild-fts
```

Do not add compose/reply/block/mute until the read/import/sync substrate is
solid and policy approval UX is ready. Acting on X is a higher-risk product
surface than reading and organizing evidence.

### MCP Tools

Add only the MCP tools that agents actually need:

- `x_import_archive`
- `x_sync_bookmarks`
- `x_search_tweets`
- `x_research_brief`
- `x_digest_candidates`
- `x_source_health`
- `x_export_portable`

Avoid exposing every CLI subcommand as a tool by reflex. Arcwell already has a
large MCP surface; adding Birdclaw-scale breadth without routing discipline will
make agents worse at choosing tools.

### Projections

Canonical X rows should project into existing Arcwell systems:

- source cards for durable evidence
- wiki pages for selected saved/watched tweets and generated research briefs
- digest candidates for high-interest watched items
- work/project records when an X item causes a task
- ops UI for health/cursor/credential state

The projection should be idempotent and repairable:

- `arcwell x project-source-cards --since ...`
- `arcwell x repair-projections`
- tests that prove canonical rows survive projection failure

## Implementation Order

### Phase 1: Model And Migration

Goal: canonical tweet/profile storage without changing user-facing behavior.

- add `x_profiles`, `x_tweets`, `x_tweet_edges`, `x_collections`, `x_tweets_fts`
- migrate or dual-write existing `x_items`
- make `x list` and `x report` read through a compatibility projection
- add tests for dedupe, source provenance, FTS, unsafe URLs, and prompt
  injection preservation
- keep source-card/wiki fanout unchanged

Validation:

```sh
cargo fmt -- --check
cargo test --all --all-features x_
scripts/verify-codex-plugin-docs
```

### Phase 2: Bookmark Sync Becomes Canonical

Goal: make the already-present bookmark import path write canonical rows and
record collection provenance.

- replace direct `x_items` bookmark insert with canonical tweet/profile write
- store bookmark collection row with account/source/seen timestamps
- add early-stop and max-pages behavior
- keep cost/policy/token behavior as-is
- expose `x search tweets --bookmarked`

Validation:

- mocked paginated bookmark sync with duplicate saturation
- quota failure preserves cursor and releases budget
- expired token is redacted in source health
- source-card projection still creates the expected wiki/source-card rows

### Phase 3: Archive Import

Goal: bulk history without API spend.

- add archive reader for the common Twitter/X archive layout
- support selected imports for tweets, likes, bookmarks, profiles, followers,
  following, and later DMs
- implement explicit account identity matching before selected re-imports
- extract media metadata first; media bytes can be a later phase
- reuse the same canonical write pipeline as live sync

Validation:

- fixtures for archive wrappers, malformed JS preambles, missing account file,
  selected re-import preserving unselected slices, and duplicate rows
- path traversal and zip-bomb limits
- archive import does not write secrets or call network

### Phase 4: Research Briefs

Goal: turn saved/watched social material into Arcwell evidence packs.

- local query over bookmarks/watch-sourced tweets
- thread expansion from local rows
- optional live lookup for missing ancestors behind policy/cost gates
- extract links/handles
- produce Markdown plus structured JSON
- create source cards/wiki page for the brief and link original evidence

Validation:

- no fake citations
- missing thread ancestors are labeled, not invented
- prompt-injection tweet text remains quoted evidence
- source-card links are inspectable

### Phase 5: Ops/UI Lane

Goal: make the system usable and honest.

- add X lane to `/ops/ui` or a focused local route
- show watch-source/source-health/cursor/credential status
- show recent imports with source-card/wiki links
- show digest candidates and scoring freshness
- add browser smoke for desktop and mobile

Validation:

- browser-rendered smoke
- XSS tests for tweet/profile/link text
- credential redaction tests

### Phase 6: AI Overlays

Goal: model-backed interestingness without muddying truth.

- `x_scores` overlay rows
- model/prompt version, reason, score, freshness, cost decision id
- eval fixtures for high-signal, low-signal, spam, prompt-injection, and stale
  content
- digest routing through existing delivery infrastructure

Validation:

- deterministic eval gate
- model-backed optional eval when credentials are explicitly enabled
- cost records
- no score row can auto-publish or auto-deliver without delivery policy

## Things Not To Borrow Yet

- Posting/replying/blocking/muting from Arcwell X. Birdclaw supports this, but
  Arcwell needs stronger approval UX before social writes.
- A full Birdclaw-style React app. Arcwell should first expose narrow ops/review
  lanes and source-card/wiki projections.
- Depending on `xurl` or `bird` internals as the architecture. If Arcwell uses
  them, they should be adapters.
- Storing raw private DMs by default. This needs a separate retention, privacy,
  and backup policy.
- Broad MCP tool mirroring. Add agent tools only where they improve workflows.

## Concrete "Steal This" Checklist

- [ ] Normalize first, project to source cards second.
- [ ] Add FTS5 for X tweet search.
- [ ] Treat bookmarks/likes/watch/timeline/mentions as edges over canonical
      tweets, not separate item worlds.
- [ ] Add archive import as the cheapest route to historical completeness.
- [ ] Add early-stop pagination for duplicate-saturated syncs.
- [ ] Add local research briefs from bookmarks/watch items with thread context.
- [ ] Add profile snapshots and bio/entity history for identity lookup.
- [ ] Add URL/link occurrence index.
- [ ] Add portable JSONL export for X tables.
- [ ] Keep model scoring as overlay rows.
- [ ] Keep Arcwell's existing policy/cost/secret/provenance gates stronger than
      Birdclaw's current defaults.

## Immediate Recommendation

Start with Phase 1 and Phase 2. They preserve the current external behavior but
move the storage model into the right shape. Once `x_import_bookmarks`,
`x_recent_search`, and `x_monitor_watch_sources` write canonical tweets/profiles
plus compatibility projections, every later Birdclaw-inspired feature becomes
cleaner:

- search is real FTS instead of `LIKE`
- reports can filter by source/provenance without losing dedupe
- research can expand threads from canonical tweet rows
- digest scoring can attach to tweet/profile/edge ids
- archive import can reuse the same write path
- ops can show source health and canonical counts honestly

This is the high-leverage transplant. UI, archive, AI scoring, and backup should
come after the canonical model lands.
