---
description: Approve a sourced digest candidate for later delivery gating
argument-hint: [candidate-id]
---

Use `digest_candidate_approve` for the supplied candidate id. Include `reviewed_by` when the user gives a reviewer label. Do not treat approval as delivery; delivery still needs the delivery gate, channel authorization, policy, cost, and provider attempt path.
