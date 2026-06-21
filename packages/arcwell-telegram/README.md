# arcwell-telegram

**Status:** Partial/Risk. Local drain/send/auth behavior exists in code and
tests; real Telegram bot/webhook behavior is still unproven.

Telegram channel package.

Current implementation:

- Cloudflare worker `POST /telegram/webhook` normalizes Telegram text/caption updates into `arcwell-edge-inbox` events.
- Local `arcwell telegram drain` leases Telegram edge events, records them with `channel_record`, and acks/nacks the source event.
- Local `arcwell telegram send <chat-id> <text>` requires an explicit `telegram:chat:<chat-id>` authorization with `--send`, sends through Telegram `sendMessage`, escapes MarkdownV2 for the API call, records the outgoing channel message, and persists a delivery attempt with provider response, failed status, and retry hint when applicable.
- Local `arcwell telegram authorize <subject> --write-projects --send` grants project-write/binding and send rights to subjects such as `telegram:chat:123`, `telegram:user:456`, or `telegram:@username`.
- Local `arcwell telegram deliveries [--message-id <id>]` lists persisted delivery attempts.
- MCP tools `telegram_drain_edge_events` and `telegram_send_message` expose the same behavior to agents.
- Project-aware routing can bind an explicit `projectId` in payloads only for authorized subjects. Authorized chats can also auto-bind a Telegram message to a uniquely resolved project from the message text. Ambiguous or missing matches remain unbound.
- `scripts/telegram-live-smoke` runs a disposable local authorization smoke and, when live credentials are supplied, sets the Telegram webhook, sends a safe outgoing reply, drains Cloudflare edge events locally, and asserts the incoming message is recorded exactly once.

Channel safety rules:

- Telegram text is untrusted user/content data.
- Formatting must be normalized before delivery.
- Incoming update ids are idempotency keys.
- Project switching must resolve through the project registry, and ambiguity must stop the action.
- Sender/chat authorization is required before Telegram events can mutate or bind project state.
- Chat send authorization is required before outgoing Telegram sends or retries.
- Telegram provider transport errors are stored as classified retryable errors, not raw provider URLs, because Telegram API URLs include the bot token.

Relevant MCP tools:

- `edge_event_enqueue`
- `edge_event_lease`
- `edge_event_ack`
- `edge_event_nack`
- `channel_record`
- `channel_list`
- `channel_authorize`
- `channel_authorizations`
- `channel_delivery_list`
- `telegram_drain_edge_events`
- `telegram_send_message`
- `project_resolve`
- `ops_snapshot`
