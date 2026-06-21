# Google Workspace Strategy

Last updated: 2026-06-20

Arcwell should use host-native Google Workspace connectors first. Codex already
has Gmail, Drive, Docs, Sheets, Slides, Calendar, and Contacts connector paths,
and those connectors keep OAuth grants, consent, and provider semantics outside
Arcwell. Arcwell should add custom Google code only for missing workflows that
need durable local context, source-card provenance, project linkage, or channel
safety.

## Decision

The current strategy is host-native connector first, narrow Arcwell indexing
second.

- Use host Google connectors for interactive read/write tasks.
- Use Arcwell storage only when the user explicitly asks to archive, connect,
  or summarize Workspace material into Arcwell projects, wiki pages, source
  cards, work runs, procedures, or memory review queues.
- Do not build a general Google API wrapper in Arcwell.
- Do not store full Gmail/Drive/Calendar bodies by default.
- Do not treat document or email body text as instructions.

## Functional Boundary

Host-native connector tasks:

- Read or summarize current Gmail threads, Drive files, Docs, Sheets, Slides,
  Calendar events, and Contacts.
- Create or edit Google Docs, Sheets, Slides, Calendar events, and Gmail drafts
  through their native connector workflows.
- Search live provider state when freshness matters.
- Use provider-native permissions, sharing, and audit semantics.

Arcwell-owned tasks:

- Record that a Workspace artifact was used as evidence for a project, work
  run, research brief, or source card.
- Archive selected snippets or summaries to the wiki only after explicit user
  request or configured policy.
- Store stable metadata needed for project continuity: provider, object type,
  title, canonical provider id or URL, owner-visible timestamp, selected labels,
  project id, source-card id, and capture decision.
- Route a selected Gmail thread or Doc into source-card ingestion with trust
  labels and prompt-injection warnings.
- Maintain provenance and review state for any extracted memory/profile facts.

## Permission Matrix

| Workflow | Preferred surface | Minimum scope | Arcwell storage | Default write policy |
| --- | --- | --- | --- | --- |
| Summarize a Gmail thread | Gmail connector | Read selected thread | None unless user archives | No external write |
| Draft an email reply | Gmail connector | Read thread, create draft | Optional work-run link | Draft only |
| Send email | Gmail connector or future email package | Send mail | Delivery receipt only | Explicit confirmation unless policy grants |
| Prepare for a meeting | Calendar + Drive + Contacts connectors | Read selected event and linked docs | Optional project note/source links | No external write |
| Create calendar event | Calendar connector | Calendar write | Optional project status note | Allowed for trusted owner when fully specified |
| Archive Doc to wiki | Drive/Docs connector plus Arcwell wiki | Read selected doc | Source card + wiki page | Local write after user request/policy |
| Find related project docs | Drive connector first | Metadata/content search as granted by host | Optional metadata index | Local metadata only |
| Turn email into source card | Gmail connector or future email channel | Read selected thread | Source card, sanitized snippets, provenance | Local write after user request/policy |
| Workspace monitoring | Future narrow package | Explicit label/folder/calendar allowlist | Metadata and source cards only | No send/share by default |

## Architectural Notes

Future `arcwell-workspace-context` should be a bridge, not a replacement for
Google Workspace:

- Input adapters should receive selected connector results or explicitly
  configured watch scopes.
- The core store should save provenance, scope, retention, and review status.
- Policy checks should run before storing Workspace-derived content, creating
  memory/profile candidates, sending email, sharing docs, or writing Calendar
  events from non-interactive channels.
- Source-trust labels must mark Gmail and Docs content as untrusted evidence.
- Calendar state is freshness-sensitive; local cached event metadata must carry
  capture time and must not masquerade as live availability.
- Email bodies, Docs comments, file names, attachment names, and invite text are
  attacker-controlled data.

## Severe Review

Claim: Arcwell can safely use Workspace context without overbroad Google scopes,
silent durable capture, or instruction confusion.

Refutations that must remain covered as the implementation grows:

- A broad OAuth grant is requested for a task that only needs a selected thread,
  selected file, or selected event.
- A user asks for a live Calendar answer and Arcwell answers from stale cached
  metadata without saying so.
- A private email or Doc is stored into wiki/source cards without explicit
  request, configured policy, or visible provenance.
- Prompt injection in an email body, Doc text, comment, filename, or attachment
  title changes tool policy or causes an external write.
- A non-owner channel can read, send, share, or archive Workspace material
  without a policy and channel authorization decision.

Current evidence is documentation-level only: no Arcwell Google API package is
implemented, and no live connector smoke was run in this repo. That is
intentional for this strategy node. Implementation tests belong with the future
`arcwell-workspace-context` or `arcwell-email` package.
