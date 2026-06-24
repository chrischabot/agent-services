---
description: Check digest candidate delivery gates
argument-hint: [candidate-id]
---

Use `digest_candidate_delivery_check` before any digest candidate delivery attempt. Provide `channel`, `subject`, and `target` only from explicit user/channel context. Report whether review and policy allow delivery; do not send anything from this command.
