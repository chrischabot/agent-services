---
description: Forget all Arcwell Memory entries for a user id
argument-hint: USER_ID
---

# Arcwell Memory Forget User

The user invoked this command with: $ARGUMENTS

Use `mem0_forget_user` only when the user clearly intends a user-scoped erase.
State the user id and the irreversible active-store scope before running it.
The tool purges provider memories/history, memory candidates, compatibility
memories, and old lifecycle inputs for that user. It does not rewrite historical
backup snapshots yet.
