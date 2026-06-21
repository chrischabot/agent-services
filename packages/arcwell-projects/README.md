# arcwell-projects

**Status:** Partial. Manual/local project state and evidence-backed work-run
consolidation exist. Codex and Claude host inventory/read adapters are not
connected, so status reports explicitly return unavailable live-state metadata.
Live Codex/Claude thread inventory is missing.

Project and thread meta-controller package.

Current first-pass implementation:

- Durable project registry in local SQLite.
- Alias-based project resolution with ambiguity detection.
- Follow-up resolution can use an explicit `context_project_id`.
- Channel messages can be bound to a project id.
- Channel authorization policy controls which channel subjects may bind or mutate project state.
- Authorized Telegram chats can auto-bind project-ish messages to a uniquely resolved project; unauthorized messages stay unbound unless they attempt an explicit project id, which fails closed.
- Timestamped project status snapshots can record status, summary, source, thread reference, confidence, and created time.
- `project_status_get` returns a status report envelope with the project,
  latest snapshot, timestamp/source/confidence provenance, and a live-state
  availability matrix. Today Codex and Claude live inventory/read are reported
  unavailable; thread refs are provenance only, not verified live handles.
- Work-memory consolidation can record project status snapshots from trace
  evidence, including validation and work-run provenance. Generated summaries
  alone cannot support consolidation.
- Channel-scoped project status reads can enforce `can_read_projects`; direct
  project id reads from untrusted channel subjects fail closed when channel
  context is supplied.
- CLI fallback for manual sync:
  - `arcwell project create <name> <summary> --alias <alias>`
  - `arcwell project list`
  - `arcwell project resolve <query>`
  - `arcwell project status-record <project-id> <status> <summary> --source manual --thread-ref <ref>`
  - `arcwell project status-get <project-id> [--channel telegram --subject telegram:chat:<id>]`

Degraded live-state capability matrix:

| Host | Live inventory | Live thread read | Manual snapshot | Current blocker |
| --- | --- | --- | --- | --- |
| Codex | Unavailable | Unavailable | Supported via CLI/MCP/plugin prompt | No stable Arcwell-owned Codex thread inventory/read API is wired into the Rust core. |
| Claude | Unavailable | Unavailable | Supported via CLI/MCP | Claude lifecycle/thread inventory hooks are unavailable or unproven. |

MCP tools:

- `project_create`
- `project_list`
- `project_resolve`
- `project_status_record`
- `project_status_get`
- `channel_record`
- `channel_list`
- `channel_authorize`
- `channel_authorizations`

Remaining work:

- Integrate with Codex thread inventory APIs when exposed.
- Add automatic project status summaries from live thread/task state.
- Add channel context carryover for follow-up messages like "and the video project?" from real chat history.
