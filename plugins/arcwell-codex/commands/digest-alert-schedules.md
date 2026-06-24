---
description: List scheduled digest alert routes
argument-hint: ""
---

# /digest-alert-schedules

List scheduled digest alert routes.

Use `digest_alert_schedules`. Summarize active versus paused schedules, channel,
recipient, threshold, max candidates, cadence, and quiet-hours configuration.
Do not infer that a schedule has delivered anything; use
`digest_alert_ticks` and `digest_candidate_deliveries` for delivery evidence.
