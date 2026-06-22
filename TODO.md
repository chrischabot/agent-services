# Arcwell Remaining Work

Last updated: 2026-06-22

This file is intentionally only unfinished work. Completed historical checklist
items were removed. Existing unchecked items from the prior `TODO.md` have been
preserved here and grouped under the real-user readiness plan in `PLAN.md`.

Do not mark an item complete because a command, scaffold, prompt, package, or
README exists. Mark it complete only when code, tests, severe review, live proof
where relevant, docs, `STATUS.md`, and this file agree.

## Global Execution Gates

- [ ] Every implementation PR/change updates this file and `STATUS.md`.
- [ ] Every meaningful feature names its behavioral claim before coding.
- [ ] Every feature has at least one test that tries to refute that claim.
- [ ] Every P0/P1 feature has a severe/adversarial test gate before completion.
- [ ] Every external integration has one local/mock test and one documented live
      smoke test.
- [ ] Every agent-facing command or skill must fail honestly when the capability
      is partial, scaffolded, or unavailable.
- [ ] Do not silently convert "manual foreground command works" into "service is
      installed and reliable."
- [ ] Do not call generated summaries "research" or "memory" unless source,
      provenance, and uncertainty are inspectable.

## 1. Live Telegram And Mobile Channel Loop

- [ ] Live-smoke real Telegram webhook -> Cloudflare -> local drain ->
      `channel_messages` and controller route report from a fresh real Telegram
      client message.
- [ ] Add safe follow-up context carryover for authorized Telegram chats.
- [ ] Add production monitoring for Telegram webhook freshness, drain lag,
      repeated nacks, and failed delivery retries before treating Telegram as a
      critical alert path.
- [ ] Add Miniflare coverage if future local Node tests miss another
      deployed-worker failure mode.

## 2. Codex And Claude Host Integration Proof

- [ ] Fresh-thread smoke `arc` inside the Codex app.
- [ ] Live-smoke Codex plugin hooks and Claude degraded memory workflow.
- [ ] Add Codex plugin prompts or hooks for task start/finish capture where the
      host can support them.
- [ ] Add a native host adapter for Codex thread inventory if a stable API
      becomes available.
- [ ] Record an interactive MCP Inspector run against `arcwell mcp`.
- [ ] Validate Claude Desktop/Code config in an authenticated local profile.
- [ ] Live-smoke the resident Codex host-adapter flow against a disposable Codex
      thread and record the freshness/provenance behavior.
- [ ] Keep degraded/manual host-sync state explicitly labeled so stale snapshots
      cannot masquerade as live thread state.

## 3. Packaging, Release, Install, And Upgrade

- [ ] Publish signed or checksummed GitHub release artifacts.
- [ ] Render and test a Homebrew formula/tap from real release artifact
      checksums.
- [ ] Run Linux `systemctl --user` live proof for install, status, restart,
      journal/logs, strict doctor, and uninstall.
- [ ] Add release gating so archive traversal, checksum mismatch, interrupted
      upgrade, stale `PATH`, old schema, service rendering, plugin PATH, and
      uninstall preservation all fail closed before publication.
- [ ] Document the exact public install, upgrade, backup-before-migration,
      service, plugin, and uninstall paths after the public artifact smoke
      passes.

## 4. Ops, Monitoring, And Human Control Surface

- [ ] Decide whether to keep server-rendered HTML or split out a small frontend
      package before adding richer controls.
- [ ] Add browser validation for the richer current `/ops/ui` on desktop and
      mobile.
- [ ] Add manual job requeue/cancel controls only after safe public core APIs
      exist; do not fake unsupported remediation.
- [ ] Add safe controls for retry delivery, apply/reject candidate, run doctor,
      create/verify backup, drain once, and inspect policy denial reasons.
- [ ] Add charts and stale-state summaries for queue age, failed deliveries,
      backup freshness, source health, credential health, costs, work runs, and
      pending reviews.
- [ ] Add live-provider probe summaries to ops only where probes are cheap,
      safe, redacted, and policy/cost aware.
- [ ] Keep Obsidian/Markdown as the wiki editing surface; do not duplicate wiki
      authoring unless needed.

## 5. Proactive Delivery: Email, Telegram, Librarian, And X

- [ ] Wire email/librarian digest delivery with schedule, threshold, quiet
      hours, dedupe, policy/cost checks, recipient authorization, delivery
      attempts, and retry behavior.
- [ ] Add production monitoring for email ingress/outbound if email becomes a
      critical alert path.
- [ ] Add Cloudflare callback/cron event capture after edge inbox is durable and
      monitored enough for production use.
- [ ] Add model-backed interestingness for X/source/digest candidates behind
      explicit config, policy, cost gates, and eval coverage.
- [ ] Add delivery routing for X/watch-source digest candidates through the same
      email/Telegram delivery-attempt infrastructure.
- [ ] Preserve tracked email defaults as `agent@example.com` and
      `user@example.com`; keep real local agent/author addresses only in ignored
      env or secret config.

## 6. Deep Research Quality And Host-Native Execution

- [ ] Add page expansion that actively gathers related docs/blogs/repos/social
      sources before writing a topic page.
- [x] Record fresh in-app Codex subagent and host-search proof for the current
      deep-research substrate.
- [x] Prove live OpenAI editorial invocation with cost records and fail-closed
      behavior on insufficient evidence.
- [ ] Add native host-search pathway for Claude where available and finish
      full-report host-search orchestration for Codex/OpenAI.
- [ ] Run live provider-backed research/editorial synthesis and adversarial eval
      quality smokes over saturated source-card corpora with cost records and
      artifacts.
- [ ] Expand difficult-document fixture coverage for PDFs, XLSX, precise table
      extraction, formula/cell handling, and publication-grade citation links.
- [ ] Add publication-grade claim/report citation-quality checks that block
      completed status when evidence links are missing, stale, generated, or too
      weak.
- [ ] Run fresh reference-topic deep-research live runs after host search,
      subagent orchestration, and provider-backed evals are proven.
- [ ] Add browser-rendered JavaScript readability extraction for pages that
      require rendering.

## 7. Memory, Work Graph, And Procedural Retrieval Loop

- [ ] Add consolidation job that can surface unresolved risks, recurring
      failures, stale runs, pending follow-ups, and reusable lessons.
- [ ] Add optional model-backed procedure extraction behind explicit config and
      cost policy.
- [ ] Add plugin prompts that retrieve approved procedures before relevant
      tasks.
- [ ] Live-smoke Codex/Claude procedure retrieval in a host task and prove the
      procedure is retrieved because of task relevance, not manual prompting.
- [ ] Add human review UI for memory, procedure, and project-status candidates.
- [ ] Add live model-backed memory extraction quality evals with explicit
      provider/cost opt-in.
- [ ] Implement retained-backup erasure or rotation for forgotten memory data,
      or keep the limitation visible in strict doctor and ops until implemented.

## 8. Policy, Cost, Secrets, And Provider Safety

- [ ] Inventory every sensitive operation in CLI, MCP, worker jobs, HTTP, edge
      drain, memory, project, channel, source ingestion, and provider adapters.
- [ ] Add missing policy guards found by the sensitive-operation inventory
      before credentials, provider calls, local mutation, worker enqueue, or
      outbound delivery.
- [ ] Record provider-reported actual costs where provider APIs return reliable
      usage/cost data.
- [ ] Add provider-specific live credential probes for configured providers
      without leaking secret values.
- [ ] Add provider-side revocation/rotation helpers where provider APIs make
      that safe and useful.
- [ ] Add scheduled credential rotation reminders and stale-scope warnings.
- [ ] Add ops UI burn-down and override controls for budgets only after
      idempotency, policy, and audit behavior are tested.

## 9. Backup, Forget, Recovery, And Retention

- [ ] Add scheduled local backup jobs through the worker/service.
- [ ] Add encrypted backup archive support and key-management documentation.
- [ ] Add off-machine backup target configuration with at least one tested
      target.
- [ ] Add automated restore drills into disposable homes and expose last drill
      result in ops/doctor.
- [ ] Add retained-backup erasure or rotation implementation for forget
      requests and document exact remaining limits.
- [ ] Add ops controls for create backup, verify backup, and run restore drill
      once safe action APIs exist.

## 10. Garderobe Deployment And Provenance Boundary

- [ ] Import the current live Garderobe deployment config into ignored local
      files such as `packages/arcwell-garderobe/wrangler.live.jsonc` without
      committing real D1/KV ids, owner email, route, or secrets.
- [ ] Preserve existing MCP connector compatibility while another agent is
      connected: keep `/mcp`, `/authorize`, `/token`, `/register`, S256 PKCE,
      scopes `wardrobe.read` / `wardrobe.write`, and MCP server name
      `garderobe` stable until deliberate migration/re-authorization.
- [ ] Run guarded read-only live smoke with the approved deployed Garderobe base
      URL.
- [ ] Add authenticated/write-capable Garderobe MCP live evidence using
      disposable fixture rows or staging data, not private wardrobe seed data,
      and do not clear OAuth KV or force the connected host to reconnect.
- [ ] Record a host OAuth/MCP handshake proof if Garderobe is meant to be used
      from Claude/Codex directly.
- [ ] Resolve and document top-level license/provenance for vendored Garderobe
      code before public redistribution.
- [ ] Keep Garderobe inventory out of Arcwell memory/profile/wiki by default and
      add tests for explicit opt-in sync only.

## 11. External Assistant Utilities

- [ ] Decide whether TIDAL control should remain a Codex plugin skill/script or
      be promoted to a durable Arcwell CLI/MCP package with policy gates,
      tests, ops visibility, and documented live-smoke expectations.
- [ ] If promoted, add explicit confirmation/policy handling for destructive
      TIDAL actions such as deleting playlists, removing playlist items, or
      unfavoriting collection items.
- [ ] Capture live LUMIN P1 device XML/service descriptors, then decide whether
      `lumin-control` should remain a Codex plugin skill/script or become a
      durable Arcwell CLI/MCP package with policy gates and live-smoke
      expectations.
- [ ] If promoted, add stable tests and policy handling for LUMIN writes such as
      standby, source/input selection, volume changes, and playlist mutation.

## Continuous Verification Checklist

Run this before marking any P0/P1 item done:

- [ ] `cargo test --all --all-features`
- [ ] Package-specific typecheck/test commands
- [ ] New severe tests fail on the old broken/scaffold behavior or clearly
      refute a realistic failure mode
- [ ] Live smoke documented when external APIs are involved
- [ ] `STATUS.md` updated
- [ ] `TODO.md` checkbox updated
- [ ] Package README updated
- [ ] Plugin commands/skills updated if the agent-facing behavior changed
- [ ] Ops visibility added for new long-running or failure-prone state
- [ ] Remaining risk explicitly stated
