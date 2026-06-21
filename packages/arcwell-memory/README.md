# arcwell-memory

**Status:** Partial.

Personal memory service.

Current implementation has two paths:

- Arcwell Memory, the in-repo Rust memory provider derived from the former
  `mem0-rs` codebase. This is the primary path for add/search/update/delete,
  history, and user-scoped forget.
- Reviewable candidate flows that now apply ADD/UPDATE/DELETE/NONE operations
  through Arcwell Memory.
- A simple SQLite compatibility table used by older commands while the full
  dream/reconcile workflow is being completed.

```sh
arcwell memory mem0-add "My cat is called Ophelia" --user-id chris
arcwell memory mem0-search Ophelia --user-id chris
arcwell memory mem0-update <memory-id> "My cat is called Ophelia Blue"
arcwell memory mem0-history <memory-id>
arcwell memory mem0-forget-user --user-id chris

arcwell memory recall "personal preferences for this task"
arcwell memory capture "My cat is called Ophelia." --source manual-note
arcwell memory dream
arcwell memory events --limit 20
arcwell memory decisions --limit 20
arcwell memory tombstones --limit 20
arcwell memory eval-corpus

arcwell memory add "My cat is called Ophelia" --kind fact
arcwell memory search Ophelia
arcwell memory list
arcwell memory delete <id>
```

Canonical configuration:

- `ARCWELL_MEMORY_CONFIG`
- `ARCWELL_MEMORY_PROVIDER`
- `ARCWELL_MEMORY_USER_ID`
- `ARCWELL_MEMORY_EMBEDDING_MODEL`
- `ARCWELL_MEMORY_LLM_MODEL`
- `ARCWELL_MEMORY_REASONING_EFFORT`

Legacy `ARCWELL_MEM0_*` names still work as compatibility aliases.

Codex integration:

- MCP tools expose Arcwell Memory CRUD, recall, capture, lifecycle events, and
  review candidates.
- The Codex plugin ships hooks for pre-turn/session recall and compact/stop
  capture.
- Hook capture defaults to review mode. Non-sensitive auto-apply requires
  explicit configuration, and UPDATE/DELETE/conflict candidates still remain
  pending for review.
- Memory extraction decisions are auditable through `arcwell memory decisions`
  and `/ops/ui`; entries include operation, confidence, reason, source, and
  candidate/memory ids where available.
- `arcwell memory eval-corpus` runs the local personal-memory eval corpus for
  false positives, sensitive medical/secret capture, and prompt-injection text
  treated as data.
- `mem0-forget-user` purges active provider memories, provider history,
  candidates, compatibility rows, lifecycle inputs, and user-scoped decision
  observations. It writes a tombstone with a hashed user id and counts.
- Procedural memory is separate from personal memory: reviewed procedures live
  in `ARCWELL_HOME/procedures` and are managed with `arcwell procedure ...` /
  MCP procedure tools.

Still incomplete:

- Live model-backed extraction quality. The current eval corpus is deterministic
  and local.
- Confidence aging and stale preference review.
- Model-backed procedural-memory synthesis and automatic skill export.
- Historical backup erasure. Forget writes a tombstone and documents that
  retained backups are not rewritten by forget.
- Human review UI.
