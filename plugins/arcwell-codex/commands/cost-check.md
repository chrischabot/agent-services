---
description: Check whether a projected provider operation is allowed by cost policy.
---

Use `cost_check` with package, provider, optional source, and projected cost.

Report the allow/block decision and the matched policy. If blocked, do not call the provider unless the user changes policy or supplies a valid temporary override.
