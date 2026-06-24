# arcwell-librarian

**Status:** Scaffold/Partial.

Repository tracking: [STATUS.md](../../STATUS.md) and [TODO.md](../../TODO.md).

Wiki librarian and interestingness package.

Current first-pass implementation:

- Digest candidates can be created from source-card ids.
- Candidates are scored with transparent rule-based signals.
- Topics can be expanded into wiki pages through `librarian_expand_topic`.
- Expanded pages include deterministic source-card audit notes and exclude generated/model-answer, untrusted, and low-reliability source cards from primary evidence.
- Reviewed digest candidates can be delivered through Telegram or email after
  policy, channel authorization, cost, and provider-send gates.
- Resident worker digest alert schedules can route already-approved candidates
  above a threshold, record durable ticks and delivery ids, and defer active UTC
  quiet-hours before provider sends.

MCP tools:

- `digest_candidate_create`
- `digest_candidate_list`
- `digest_alert_schedule_create`
- `digest_alert_schedules`
- `digest_alert_ticks`
- `librarian_expand_topic`
- `source_card_add`
- `source_card_search`
- `source_card_read`
- `wiki_expand_page`

Remaining work:

- Cluster related source cards across RSS, GitHub, arXiv, X, and web search.
- Add richer contradiction detection beyond deterministic source-card audit heuristics.
- Add model-backed extraction/synthesis only behind explicit config and source-grounded citation checks; no model-backed librarian synthesis is claimed today.
- Prove live external Telegram/email digest alert delivery and add production
  monitoring before treating scheduled digests as a critical alert path.
- Add model-backed page synthesis with source-grounded citations.
