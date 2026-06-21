---
description: Record a timestamped Arcwell project status snapshot
argument-hint: PROJECT_ID STATUS SUMMARY [SOURCE] [THREAD_REF]
---

# Project Status Record

The user invoked this command with: $ARGUMENTS

Use `project_status_record`. Preserve the source and thread reference when known.
Do not imply this is live Codex or Claude state unless the source actually came
from a live host integration. Even host-labeled snapshots are durable evidence;
`project_status_get` is the source of truth for whether Arcwell can currently
verify live thread state.
