---
description: Create a scheduled digest alert route
argument-hint: name channel recipient_ref [min_score] [max_candidates] [interval_hours]
---

# /digest-alert-schedule-create

Create a resident worker schedule for reviewed digest candidates.

Use `digest_alert_schedule_create`. Only create schedules from explicit user
intent. The schedule selects already-approved digest candidates above the
configured threshold and routes them through the digest delivery ledger during
`worker_run_once`; it must not be described as auto-approving or auto-writing
digests.

Report the schedule id, channel, recipient, threshold, cadence, and whether
quiet hours are configured. Say plainly that provider sends still require the
normal digest policy, channel authorization, cost, and configured provider
secrets at execution time.
