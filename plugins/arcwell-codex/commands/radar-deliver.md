---
description: Manually deliver an existing radar summary through an authorized channel.
---

Use `radar_deliver_summary`. Require a run id and recipient. The summary must
already exist and have `audit_ok`; create it with `/radar-summarize` first if
needed.

This records a durable `radar_deliveries` row and, when the provider path is
reached, links it to a channel delivery attempt. Report the radar delivery
status, channel, recipient reference, idempotency result, channel message id,
and channel delivery attempt id when present.

Do not imply scheduled delivery, quiet-hours handling, retry/dead-letter
operation, model synthesis, or production-data delivery proof unless those
specific gates have been run.
