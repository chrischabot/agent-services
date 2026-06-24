---
description: Deliver an approved digest candidate through Cloudflare Email
argument-hint: [candidate-id]
---

# /digest-candidate-deliver-email

Deliver an approved digest candidate to an email recipient through Cloudflare
Email.

Use `digest_candidate_deliver_email` only after the user has explicitly
approved the candidate and named the recipient. This command must pass the
digest candidate review/policy gate and the normal email send
authorization/policy/cost/provider path. Report the resulting digest delivery,
channel message, and delivery attempt ids. Do not treat this as Telegram
delivery, quiet-hours scheduling, due retry orchestration, or recurring
delivery.
