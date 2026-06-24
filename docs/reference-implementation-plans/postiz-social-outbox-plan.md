# Postiz Social Outbox Plan

Date: 2026-06-24

Status: design and implementation plan only. No Arcwell feature is implemented
by this file.

Reference repo: https://github.com/gitroomhq/postiz-app

Reference commit inspected: `1c8d9b8`

Local inspection path: `/tmp/arcwell-reference-repos/postiz-app`

License note: Postiz is AGPL-3.0. This plan borrows product and architecture
ideas only. Do not copy source code into Arcwell without a license decision.

## Claim Boundary

This plan can claim that Postiz source code was inspected and that the outbox,
provider-contract, validation, scheduling, retry, token-refresh, and media
patterns were mapped to an Arcwell design.

This plan cannot claim that Arcwell can publish to any social network.

## Source And Code Inspected

- `package.json`
- `apps/backend/src/main.ts`
- `apps/orchestrator/src/main.ts`
- `libraries/nestjs-libraries/src/integrations/social.abstract.ts`
- `libraries/nestjs-libraries/src/integrations/integration.manager.ts`
- `libraries/nestjs-libraries/src/integrations/refresh.integration.service.ts`
- `apps/orchestrator/src/workflows/post-workflows/post.workflow.v1.0.5.ts`
- `apps/orchestrator/src/activities/post.activity.ts`
- `apps/orchestrator/src/workflows/missing.post.workflow.ts`
- `apps/backend/src/api/routes/posts.controller.ts`
- `apps/backend/src/api/routes/integrations.controller.ts`
- `apps/backend/src/api/routes/media.controller.ts`

## What Postiz Does Well

Postiz is a social publishing control plane. The valuable idea for Arcwell is
not a marketing calendar clone. It is a durable outbox with provider contracts,
validation, approval, scheduled dispatch, retries, token refresh, and provider
post IDs/URLs recorded after publish.

Strong source-code patterns:

- `SocialAbstract` defines provider-level behavior: posting, media validation,
  mention lookup, refresh-token behavior, retry classification, scopes, and
  provider media limits.
- `IntegrationManager` keeps a central registry of social providers and tools.
- `RefreshIntegrationService` updates tokens and marks channels disconnected or
  refresh-needed when refresh fails.
- The Temporal workflow fetches scheduled post state, waits until publish time,
  skips disabled integrations, retries publish, refreshes tokens on
  refresh-token failures, and records provider IDs/release URLs.
- Backend controllers validate content, settings, provider selection, draft
  status, media, upload size, and integration status before dispatch.
- Media upload is its own service surface, not embedded in post creation.

Provider coverage in the inspected code includes X, LinkedIn, Reddit,
Instagram, Facebook, Threads, YouTube, Google Business, TikTok, Pinterest,
Discord, Slack, Mastodon, Bluesky, Lemmy, Farcaster, Telegram, Nostr, Medium,
Dev.to, Hashnode, WordPress, and Listmonk.

## Arcwell-Native Shape

Arcwell should build a social outbox only after its X/source work has real
read-side proof. The outbox should be approval-first and source-linked:

- Drafts are generated from source cards, wiki notes, radar digests, or user
  text.
- Nothing publishes without explicit approval.
- Every outgoing post links back to evidence and a decision record.
- Provider-specific limits are validated before approval, not only at publish
  time.
- Delivery attempts are durable.

Working name: `arcwell social-outbox`

This should be manual-first. Scheduling can arrive after manual publish proof.

## Proposed Data Model

- `social_accounts`
  - `id`
  - `provider_key`
  - `display_name`
  - `owner_scope`
  - `secret_ref_id`
  - `oauth_profile_id`
  - `status`
  - `refresh_needed`
  - `disabled_reason`
  - `scopes_json`
  - `last_refresh_at`

- `social_provider_capabilities`
  - `provider_key`
  - `max_text_length`
  - `max_media_count`
  - `supported_media_types`
  - `supports_thread`
  - `supports_comments`
  - `supports_scheduled_publish`
  - `max_concurrent_jobs`

- `social_drafts`
  - `id`
  - `title`
  - `status`
  - `source_card_ids`
  - `wiki_page_ids`
  - `created_by`
  - `approved_by`
  - `approved_at`
  - `scheduled_for`
  - `policy_snapshot_json`

- `social_post_parts`
  - `id`
  - `draft_id`
  - `account_id`
  - `part_index`
  - `body`
  - `settings_json`
  - `validation_status`
  - `validation_errors_json`

- `social_media`
  - `id`
  - `draft_id`
  - `storage_ref`
  - `media_type`
  - `width`
  - `height`
  - `duration_ms`
  - `alt_text`
  - `validation_status`

- `social_delivery_attempts`
  - `id`
  - `draft_id`
  - `account_id`
  - `attempt`
  - `status`
  - `provider_post_id`
  - `provider_url`
  - `provider_error_code`
  - `redacted_error`
  - `idempotency_key`
  - `started_at`
  - `completed_at`

- `social_provider_errors`
  - `provider_key`
  - `error_code`
  - `classification`
  - `retryable`
  - `requires_refresh`
  - `requires_user_action`

## CLI, MCP, Slash, And Ops Surfaces

CLI:

- `arcwell social accounts`
- `arcwell social draft create --from-source-card <id>`
- `arcwell social draft validate <draft-id>`
- `arcwell social approval request <draft-id>`
- `arcwell social publish <draft-id> --account <id>`
- `arcwell social attempts <draft-id>`

MCP:

- `social_draft_create`
- `social_draft_validate`
- `social_approval_request`
- `social_delivery_status`

Slash/plugin:

- `/social-draft`
- `/social-approve`
- `/social-publish`

Ops:

- Account health, refresh-needed state, queued/scheduled/delivered/error counts,
  provider rate-limit state, and last delivery attempt.

## Implementation Plan

1. Provider contract only.
   - Define `SocialProvider` trait.
   - Implement a fake/local provider for tests.
   - Add provider capability validation.

2. Draft and media model.
   - Drafts link to source cards/wiki pages.
   - Media records use existing storage/secrets patterns.
   - Validation returns structured errors.

3. Approval gate.
   - Draft status cannot move to publishable without a user approval row.
   - Approval snapshot includes body, account, media, schedule, and policy.

4. Manual publish.
   - One provider, one draft, manual command.
   - Persist provider post ID and URL.
   - Idempotency key prevents duplicate posts on retry.

5. Token refresh and account health.
   - Detect refresh-needed errors.
   - Refresh only via stored secret/OAuth profile.
   - Do not retry forever on bad credentials.

6. Scheduling.
   - Add worker job after manual publish is proven.
   - Jobs write delivery-attempt rows.
   - Queue state appears in ops.

7. Multi-provider/thread/comment support.
   - Only after one-provider manual and scheduled proof.

## Anti-Mirage Traps

- A draft generator is not a publisher.
- A provider registry is not proof that scopes or tokens work.
- A scheduled job row is not proof that publish happened.
- A provider API success is not enough unless provider ID/URL are persisted.
- Approval text cannot be separate from the exact payload being approved.
- Model-generated social copy is not evidence; source links must remain.

## Proof Gates

- Missing: no outbox schema.
- Scaffold: provider trait, local fake provider, draft command.
- Partial: drafts validate but cannot publish.
- Local Proof: tests prove validation, approval gating, idempotency, media
  limits, retry classification, and redaction.
- Production Data Proof: a controlled provider/account posts one approved
  draft and records the provider URL and delivery attempt without leaking
  secrets.
- Operational: scheduled publish, retry, refresh-needed, disabled account,
  quiet-hours/policy denial, and ops visibility are proven.
- Done: all claimed providers satisfy provider-specific validation,
  delivery-attempt, idempotency, refresh, and recovery proof.

## Severe Tests

- Publish without approval is rejected.
- Approval payload changes after approval; publish is rejected until reapproved.
- Disabled or refresh-needed account cannot publish.
- Token refresh succeeds once and retry publishes once.
- Token refresh fails and account health changes to blocked.
- Provider returns 429/500; retry classification is honored.
- Provider returns bad-body/content-policy error; no retry storm.
- Media dimensions/type/count exceed provider limits.
- First post succeeds, comment/thread part fails; state shows partial delivery.
- Duplicate worker execution does not duplicate provider post.
- Secret-like provider error is redacted in attempts and ops.
- Source-card prompt injection in draft text is stored as content only.
- Webhook/retry replay cannot change approved payload.

## First Slice

Build a local-only `social-outbox` with draft validation and approval gating.
The first publish-capable slice should target a disposable or explicitly
authorized account and produce a delivery proof packet before any docs claim
Arcwell can publish.

