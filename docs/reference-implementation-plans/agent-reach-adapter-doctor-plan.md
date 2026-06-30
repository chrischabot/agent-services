# Agent-Reach Adapter Doctor Plan

Date: 2026-06-24

Status: design and implementation plan only. No Arcwell feature is implemented
by this file.

Reference repo: https://github.com/Panniantong/Agent-Reach

Reference commit inspected: `a7c56eb`

Local inspection path: `/tmp/arcwell-reference-repos/Agent-Reach`

## Claim Boundary

This plan can claim that Agent-Reach's source code was inspected and that this
document maps the transferable patterns into Arcwell-native design work.

This plan cannot claim that Arcwell has a working multi-source adapter doctor,
that any provider was probed, or that any source has been ingested.

## Source And Code Inspected

- `agent_reach/cli.py`
- `agent_reach/core.py`
- `agent_reach/config.py`
- `agent_reach/doctor.py`
- `agent_reach/probe.py`
- `agent_reach/channels/base.py`
- `agent_reach/channels/__init__.py`
- `agent_reach/channels/twitter.py`
- `agent_reach/channels/reddit.py`
- `agent_reach/channels/youtube.py`
- `agent_reach/backends/opencli.py`
- `agent_reach/transcribe.py`
- `agent_reach/skill/references/*`
- `tests/test_channel_contracts.py`
- `tests/test_twitter_channel.py`

## What Agent-Reach Does Well

Agent-Reach is mostly a capability-discovery and agent-instruction bridge. It
does not try to own every social/source runtime. It installs a skill, detects
available tools, and teaches agents which upstream tool to call directly.

The interesting implementation detail is its channel contract:

- Each channel declares ordered backends.
- The active backend can be overridden by config.
- The doctor probes all available paths and reports `ok`, `warn`, `missing`,
  `broken`, `timeout`, and `error` states.
- A warning backend does not block a later working backend.
- Config files are written with `0600` permissions and secret values are
  masked in display output.
- `probe_command` treats command probing as a side-effect-sensitive operation:
  it has timeouts, missing executable detection, broken command detection, and
  stale virtualenv shim detection.
- `doctor.check_all` isolates per-channel failures so one broken adapter does
  not collapse the whole report.

The Twitter channel was the best concrete example. It tries `twitter-cli`,
`OpenCLI`, and a legacy Bird CLI path, checks which backend is actually usable,
and records which one became active. The Reddit channel is valuable for a
different reason: it is honest about not having a zero-config path and names the
official API/session limits rather than pretending they are solved.

## Arcwell-Native Shape

Arcwell should not copy Agent-Reach as a runtime. The useful capability is a
shared adapter capability registry and doctor that covers Arcwell source
families before ingestion runs.

Working name: `arcwell reach`

Core idea: every source adapter gets a capability record that can be probed
without mutating durable content. That capability record feeds source health,
ops, CLI/MCP/slash surfaces, and implementation status.

The registry should cover:

- Existing Arcwell source families: wiki, source cards, radar, research, X,
  Telegram, email, RSS, GitHub, Hacker News, web, bookmarks, and local files.
- Future social/read adapters: Reddit, YouTube, Bluesky, Mastodon, LinkedIn,
  Discord, Slack, and external CLI-backed tools.
- Provider requirements: executable, local session, OAuth token, API key,
  network endpoint, browser session, filesystem path, or unsupported.
- Trust tier: zero-config local, configured credentials, manual session,
  browser-controlled, experimental, unsupported.

## Proposed Data Model

Add or extend a source-capability table rather than creating a parallel product:

- `adapter_capabilities`
  - `id`
  - `adapter_key`
  - `source_kind`
  - `backend_key`
  - `display_name`
  - `tier`
  - `requires_secret_ref`
  - `requires_executable`
  - `requires_oauth_profile`
  - `supports_search`
  - `supports_fetch`
  - `supports_thread_expand`
  - `supports_media`
  - `supports_live_cursor`
  - `created_at`
  - `updated_at`

- `adapter_probe_runs`
  - `id`
  - `adapter_key`
  - `backend_key`
  - `status`
  - `message`
  - `diagnostic_code`
  - `duration_ms`
  - `exit_code`
  - `timed_out`
  - `secret_ref_ids_used`
  - `redacted_stdout_tail`
  - `redacted_stderr_tail`
  - `created_at`

- `source_adapter_health`
  - `adapter_key`
  - `active_backend_key`
  - `health_status`
  - `last_probe_run_id`
  - `last_success_at`
  - `last_failure_at`
  - `blocked_reason`
  - `next_action_hint`

If Arcwell already has source-health rows for a given source family, this should
project into that model instead of becoming a second truth store.

## CLI, MCP, Slash, And Ops Surfaces

CLI:

- `arcwell reach doctor --json`
- `arcwell reach doctor --adapter x --json`
- `arcwell reach probe <adapter> [--backend <backend>]`
- `arcwell reach backends <adapter>`
- `arcwell reach explain <adapter>`

MCP:

- `reach_capabilities`
- `reach_probe`
- `reach_source_health`

Slash/plugin:

- `/reach-doctor`
- `/reach-probe <adapter>`

Ops:

- Add an adapter health section to `arcwell ops`.
- Show active backend, last probe, last successful proof, blocked reason, and
  whether the adapter has only doctor proof or durable ingestion proof.

## Implementation Plan

1. Define the Rust adapter capability trait.
   - `key()`
   - `source_kind()`
   - `backends()`
   - `probe_backend()`
   - `active_backend(config)`
   - `health_projection()`

2. Build a small executable probe helper.
   - Detect missing executables.
   - Enforce timeout.
   - Capture bounded stdout/stderr tails.
   - Redact secrets.
   - Detect stale symlink/shim paths where possible.

3. Register existing Arcwell adapters first.
   - Start with adapters already present in the repo.
   - Avoid adding external source support in the same slice.
   - Make unsupported adapters honest rows with `unsupported` or `missing`,
     not empty successes.

4. Persist probe runs.
   - Probe output must be durable and inspectable.
   - Do not update source cursors during a probe.
   - Do not claim source freshness from a probe.

5. Add CLI and JSON output.
   - Human output groups adapters by tier.
   - JSON output is stable and tested.

6. Add MCP and slash parity.
   - MCP schema uses the same DTO as CLI JSON.
   - Slash command must not duplicate logic in prompt text.

7. Wire ops and status language.
   - Show `doctor_ok` separately from `ingestion_ok`.
   - A green doctor never means real rows were fetched.

## Anti-Mirage Traps

- A registered adapter is not a working adapter.
- `which <tool>` is not enough; the tool must run a harmless command.
- A working backend is not proof that Arcwell can ingest, index, or summarize.
- A local fixture is not proof that OAuth/API scopes are valid.
- A doctor report with all channels skipped is not a successful report.
- Secret detection must not log token values in failure tails.

## Proof Gates

- Missing: no registry, no probes, no persisted health.
- Scaffold: trait, commands, and sample adapter exist.
- Partial: probes run in memory but are not persisted or surfaced everywhere.
- Local Proof: deterministic tests cover missing executable, timeout, broken
  executable, warning backend fallback, JSON schema, and redaction.
- Production Data Proof: at least one configured real adapter has a successful
  probe and then a separate durable ingestion run that writes source rows/cards
  without advancing cursor early.
- Operational: scheduled or on-demand probe history appears in ops, source
  health is accurate, failures are actionable, and docs/status name the exact
  proof level.
- Done: all claimed source families have doctor, ingestion, ops, and recovery
  proof at the claimed level.

## Severe Tests

- Missing executable returns `missing` and does not panic.
- Executable exists but exits nonzero with secret-like stderr; output is
  redacted.
- Probe times out and leaves bounded logs.
- First backend returns `warn`, second returns `ok`; active backend becomes the
  second backend.
- Config names an unavailable backend; report says blocked by config override.
- Credential file has unsafe permissions; report warns and refuses live proof.
- Malformed provider output is classified as `broken`.
- Huge stdout/stderr is truncated deterministically.
- Prompt-injection text from provider diagnostics is stored as data only.
- Concurrent doctor runs do not corrupt probe history.
- Probe success cannot advance source cursors.

## First Slice

Implement `arcwell reach doctor --json` for three existing source families and
make `arcwell ops` show their health. The first slice is complete only when a
test proves a fake implementation that returns static JSON cannot pass.

## 2026-06-30 Refresh: Current Arcwell Shape

Arcwell has moved past the original "add a doctor" starting point. The current
repo already has several pieces that should become the spine of an
Agent-Reach-inspired adapter doctor:

- `provider_credential_probe` exists in CLI/MCP and writes redacted
  `source_health` rows for GitHub, OpenAI, Brave Search, and Cloudflare.
- `secret_health`, `health`, `doctor`, `/ops/ui`, and `ops_snapshot` expose
  missing, expiring, refreshable, and policy-blocked credential states.
- `source_health` is now the shared place where provider probes, email mailbox
  verification, X OAuth/sync, wiki jobs, source adapters, and delivery gaps can
  surface attention states.
- Worker jobs, source cursors, radar source-quality windows, and X sync runs
  already record operational context; a separate "reach" health database would
  fragment the system.
- The proof ledger exists (`proof_packets`, `proof_claims`,
  `proof_artifacts`, `proof_checks`), so adapter-doctor promotion should record
  proof packets rather than living only in docs.

The refreshed Arcwell direction is therefore not a new `reach_*` subsystem. It
is a normalized adapter-health layer over existing `source_health`,
provider-probe, secret-health, policy/cost, worker, cursor, and ops surfaces.

## 2026-06-30 Anti-Mirage Development

Claim to build next:

> Arcwell can tell which configured source/provider adapters are ready,
> blocked, stale, rate-limited, credential-expired, policy-denied, or only
> locally scaffolded, using real probe rows and source-health state rather than
> static command descriptions.

Refutations:

- A probe passes locally but no `source_health` row changes.
- Provider credentials exist but no cheap endpoint accepted them.
- A source family has healthy provider credentials but stale cursors or blocked
  jobs.
- MCP tool descriptions claim live fetch while policy/cost/secret gates would
  block the real path.
- Ops cannot distinguish missing credentials, policy denial, quota/rate-limit,
  stale cursor, and partial write.

Revised implementation slices:

1. Add an adapter-doctor view that composes provider probes, `secret_health`,
   source-health rows, cursor age, worker job failures, and last successful
   durable source-card/write proof.
2. Define per-source-family capability metadata: expected probes, expected
   cursors, required policy actions, required secrets, and expected downstream
   projections.
3. Teach `doctor --strict` and `/ops/ui` to show "credential probe ok but
   ingestion stale" as a distinct state.
4. Add proof-ledger integration: an adapter cannot be promoted past Local Proof
   unless a packet records the exact test/probe commands and rows inspected.
5. Add MCP parity with bounded result shape and redacted diagnostics.

Keep from Agent-Reach:

- ordered backend probing;
- per-backend status;
- warning-vs-blocking classification;
- config override visibility;
- secret redaction and permission checks.

Do not copy:

- a parallel config system;
- a separate source-health truth store;
- an "agent calls random upstream CLI directly" model where Arcwell already has
  owned policy/cost/source-card boundaries.

Next proof gate:

- Local Proof: fixture source families cover ok, missing secret, policy denied,
  quota/rate-limited, stale cursor, failed worker, and partial durable-write
  states, and the doctor view renders each differently.
- Production Data Proof: a copied or controlled home runs a real provider probe
  plus one real source-family fetch and shows both credential readiness and
  durable source-card/cursor/source-health outcome.
