# Adversarial Testing And Review Policy

This project treats adversarial review as part of normal development, not as a late security pass.

## Standing Rule

Every meaningful feature should ship with at least one test that tries to refute its safety or correctness claim.

Examples:

- Storage code gets malformed, duplicate, hostile, and oversized inputs.
- File code gets path traversal, tampering, missing file, and checksum tests.
- Agent/MCP code gets unknown tool, missing argument, malformed request, and least-privilege tests.
- Import code gets malformed exports, sensitive candidates, duplicate candidates, and hostile transcript text.
- Backup/delete code gets tamper detection and cross-store coverage tests.
- Channel code gets forged identity, duplicate delivery, prompt injection, formatting, and replay tests.

## Review Checklist

Before considering a feature complete:

- Name the claim the feature makes.
- Name what would prove the claim false.
- Add at least one automated test for invalid or malicious input.
- Add a recovery/error-path test when the feature persists state.
- Run `cargo fmt --all -- --check` and `cargo test`.
- Record any demonstrated bug and the regression test that now covers it.

## Current Severe Tests

Implemented so far:

- Empty and overlong profile keys are rejected.
- SQL-shaped profile keys do not mutate schema.
- Unknown candidate targets do not get marked applied.
- Wiki titles cannot escape the wiki page directory.
- Backups include wiki Markdown pages as well as SQLite.
- Backup verification detects tampered files.
- Backup restore round-trips durable SQLite and wiki state into a fresh home.
- Backup restore round-trips Arcwell Memory vector/history artifacts into a fresh home.
- Backup restore refuses non-empty targets unless replacement is explicit.
- Backup verification rejects missing files, unsupported manifest versions, and path traversal in manifests.
- Strict doctor rejects stale backup manifests, schema drift, missing required directories including Arcwell Memory storage, stale worker heartbeats, dead letters, and missing/non-file service plist paths.
- Arcwell Memory lifecycle tests cover add/search/update/history/delete/forget and canonical `ARCWELL_MEMORY_*` env precedence over legacy `ARCWELL_MEM0_*` names.
- Arcwell Memory review tests cover provider-backed UPDATE and DELETE candidates,
  sensitive capture staying pending under auto-apply, and recall context combining
  profile plus Arcwell Memory.
- Arcwell Memory dream/forget tests cover provider duplicate cleanup,
  same-subject conflict candidate creation, compatibility duplicate cleanup, and
  active-store forget cascade across provider vectors, provider history,
  candidates, lifecycle inputs, and compatibility rows.
- Cloudflare edge worker tests reject forged secrets, accept configured next secrets during rotation, rate-limit replay storms, and rate-limit Telegram webhooks per chat.
- Telegram project binding fails closed for unauthorized forged project ids, while authorized chats can bind explicit or uniquely resolved project references.
- Telegram outgoing sends persist provider delivery attempts, failed status, and retry hints for 429 responses.
- Cost policies reject negative/invalid costs, block budget overruns and kill switches, honor temporary overrides, and stop X/web-search network paths before credentials are read.
- MCP unknown tools and missing required arguments return errors.
- MCP profile writes use parameterized storage.

## Demonstrated Finding

Finding: backup snapshots originally copied only SQLite and omitted wiki Markdown files. A later restore pass also found that WAL mode requires checkpointing before copying the SQLite file, or recent transactions may be absent from the snapshot.

Impact: a restore from backup could have lost source-backed wiki pages or recent SQLite state while preserving only partial metadata.

Fix: `Store::create_backup` now checkpoints WAL, copies `wiki/pages` and Arcwell Memory artifacts, writes a versioned manifest, and severe regression tests prove page inclusion, memory artifact restore, tamper detection, missing-file detection, path-traversal rejection, non-empty target refusal, and full restore into a fresh home.

## Untested Risks To Pull Forward

- Backup restore is local/manual only. Scheduled, encrypted, and off-machine backup are still missing.
- Cloudflare worker deploy, health, forged-secret rejection, and remote D1
  reachability have been smoke-tested, but authenticated deployed ingress,
  lease, ack/nack, rotation, rate-limit behavior, and local Rust remote drain
  still need live proof.
- Telegram has mocked send/drain tests, but no live Telegram webhook or bot smoke recorded yet.
- MCP stdio is hand-rolled and needs validation against real Codex and Claude MCP clients.
- Claude export import currently uses crude heuristics rather than a model-backed extractor with redaction.
- No fuzz/property tests yet for JSON-RPC input, import parsing, or markdown title/slug generation.
- No concurrency tests yet for simultaneous CLI/MCP writes.
