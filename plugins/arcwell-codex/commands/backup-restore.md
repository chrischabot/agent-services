---
description: Restore an arcwell backup into a target home
argument-hint: --from BACKUP_PATH [--target-home PATH] [--replace]
---

# Backup Restore

The user invoked this command with: $ARGUMENTS

Use the local CLI command `arcwell backup restore --from BACKUP_PATH`.

Prefer restoring into a fresh `--target-home` for drills. Do not use `--replace`
unless the user explicitly asked to overwrite that target. Report the restored
file count, target home, and any remaining backup limitations such as missing
scheduled/off-machine/encrypted backup.
