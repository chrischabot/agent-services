# arcwell-email

**Status:** Scaffold/Partial. The package defines the email channel boundary and
ships a tested local mapper for normalized inbound email metadata. It does not
fetch mail, run a live Cloudflare Email Routing Worker, call Gmail, send email,
or persist source cards/channel messages yet.

Email channel and ingestion package.

## Decision

Inbound capture should use Cloudflare Email Routing into `arcwell-edge-inbox`
for Arcwell-owned proactive addresses such as `launches@...` or `alerts@...`.
That matches the existing always-on edge buffer and avoids broad mailbox OAuth
scopes for passive ingestion.

Gmail remains host-native first:

- Use connected Gmail tools for interactive user-selected reading, drafting,
  and sending.
- Only archive a selected thread into Arcwell when the user or local policy
  explicitly asks for project/source-card/work-run provenance.
- Do not add Gmail API polling until a narrow label/scope/storage need is
  proven. Gmail body scopes are too broad for the default proactive loop.

## Boundary

The package-owned adapter contract is normalized metadata, not raw authority:

- `messageId`: required idempotency key seed.
- `envelopeFrom` / `signedSender`: trusted sender identity.
- `headerFrom`: display evidence only; never authorization.
- `envelopeTo`: route key.
- `auth`: SPF/DKIM/DMARC verdicts from the capture layer.
- `subject`, `bodyText`, `bodyHtml`: untrusted evidence.
- `attachments`: metadata only unless a future policy explicitly stages content.

The mapper emits:

- an `email` edge event with a stable idempotency key,
- an inbound `channel_message` draft with `UNTRUSTED_CHANNEL_EVIDENCE`,
- an optional `source_card` draft with `untrusted_email_evidence`.

The Rust/local service remains the durable owner of SQLite, wiki source cards,
channel messages, policy decisions, and project bindings.

## Routing And Authorization

Routing is fail-closed:

- recipient address must match a configured route,
- trusted sender must match an allowed address or domain,
- sender allow rules must include the route id,
- DMARC must pass by default,
- auto-replies and bulk/list mail do not enter the proactive loop,
- duplicate `Message-ID` values map to one idempotency key.

Sender authorization uses envelope/signed metadata. A spoofed `From:` header is
preserved as evidence and may add a warning, but it cannot grant access to a
route or project.

## Content Safety

Email body/content is untrusted evidence, never instructions.

Current local mapper behavior:

- strips active HTML/script/style before preview mapping,
- records prompt-injection text as inert evidence,
- flags tracking links but does not fetch them,
- rejects oversized bodies,
- ignores attachment content by default and records bounded metadata,
- rejects attachment-count and attachment-size bombs,
- rejects auto-responder/list-loop hazards.

Future live adapters must keep raw MIME parsing in a bounded, disposable path
before enqueueing sanitized metadata into `arcwell-edge-inbox`.

## Digest Delivery Boundary

Librarian digest candidates may later target email delivery, but this package
does not implement outbound email. Any future email delivery path must require:

- explicit recipient authorization,
- loop prevention headers,
- dedupe per digest candidate,
- delivery attempt records,
- policy/cost checks before provider egress,
- no reuse of inbound body text as outbound instructions.

Until that exists, librarian email delivery is a documented option only, not a
live capability.

## Local Severe Tests

```sh
cd packages/arcwell-email
npm test
```

The fixture-backed tests try to refute the mapper with spoofed `From:` headers,
malicious HTML/script/CSS, markdown prompt injection, attachment bombs, tracking
links, duplicate `Message-ID`, oversized bodies, auto-responders, and
unauthorized routing.
