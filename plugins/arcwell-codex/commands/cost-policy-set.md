---
description: Set an Arcwell cost budget, kill switch, or temporary override.
---

Set a cost policy through `cost_policy_set`.

Use scopes `global`, `package`, `provider`, or `source`. Use key `*` for global. Prefer a narrow provider/source policy over a global kill switch when the user wants to pause one integration.

If setting an override, pass `override_until` as an RFC3339 timestamp and say when it expires.
