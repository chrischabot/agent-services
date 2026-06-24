---
description: Inspect scheduled digest alert worker ticks
argument-hint: [schedule_id]
---

# /digest-alert-ticks

Inspect scheduled digest alert worker ticks.

Use `digest_alert_ticks`, optionally filtered by `schedule_id`. Report each
tick status, selected candidate ids, delivery ids, and any error. Treat
`deferred`, `blocked`, `empty`, `failed`, and `partial` as meaningful
production states, not as success.
