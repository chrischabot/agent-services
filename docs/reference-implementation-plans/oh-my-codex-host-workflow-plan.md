# oh-my-codex Host Workflow Plan

Date: 2026-06-24

Status: design and implementation plan only. No Arcwell feature is implemented
by this file.

Reference repo: https://github.com/Yeachan-Heo/oh-my-codex

Reference commit inspected: `4dde22a`

Local inspection path: `/tmp/arcwell-reference-repos/oh-my-codex`

## Claim Boundary

This plan can claim that oh-my-codex source code was inspected and that its
Codex host workflow, doctor, worktree, hook-smoke, and state-safety patterns
were mapped into Arcwell.

This plan cannot claim that Arcwell has an oh-my-codex equivalent launcher,
HUD, team mode, or Codex host doctor.

## Source And Code Inspected

- `README.md`
- `src/cli/doctor.ts`
- `src/team/worktree.ts`
- `src/cli/index.ts`
- `src/runtime/run-loop.ts`
- `src/state/operations.ts`
- `src/mcp/state-paths.ts`
- `src/hud/*`

## What oh-my-codex Does Well

oh-my-codex is a Codex workflow layer. The useful parts for Arcwell are around
host readiness and runtime hygiene:

- A serious `doctor` that checks Codex CLI, Node, config parsing, model context
  recommendations, native hook coverage, hook smoke tests, MCP servers, plugin
  version/cache, prompts/skills, AGENTS.md, artifact ownership, routing, and
  team readiness.
- Native hook smoke that executes the installed hook with a minimal
  `UserPromptSubmit` payload in a temp cwd.
- PostCompact smoke that validates stdout contract.
- Worktree launch modes with branch validation, branch-in-use detection, dirty
  worktree warnings, reusable named worktrees, and team/autoresearch modes.
- Atomic state writes with temp rename and lock files.
- Strict path validation for session IDs, mode names, file names, and working
  directories.
- Session-scoped runtime overlays and model instructions.
- Prelaunch cleanup that reaps owned orphan MCP processes.
- HUD/tmux reconciliation that only acts on owned sessions/panes.

The best Arcwell lesson is that host workflow should have a doctor that runs
real smoke checks, not merely verifies files exist.

## Arcwell-Native Shape

Arcwell already has a Codex plugin dev loop and many plugin skills/commands.
The missing product surface is a first-class host-adapter workflow guardrail for
Codex sessions, plugin readiness, hook behavior, state paths, and optional
worktree isolation.

Working name: `arcwell codex-host`

This should integrate with Arcwell's plugin dev loop:

- `scripts/arcwell-dev status`
- `scripts/arcwell-dev smoke`
- plugin skill/slash/MCP registration
- hook behavior
- runtime state paths

## Proposed Data Model

- `codex_runtime_profiles`
  - `id`
  - `profile_name`
  - `plugin_mode`
  - `codex_bin`
  - `config_path`
  - `status`
  - `created_at`

- `codex_sessions`
  - `id`
  - `profile_id`
  - `workspace_path`
  - `thread_ref`
  - `state_dir`
  - `started_at`
  - `last_seen_at`

- `codex_worktrees`
  - `id`
  - `repo_root`
  - `worktree_path`
  - `branch`
  - `mode`
  - `owner_session_id`
  - `status`

- `codex_doctor_checks`
  - `id`
  - `profile_id`
  - `check_key`
  - `status`
  - `message`
  - `details_json`
  - `created_at`

- `codex_hook_smokes`
  - `id`
  - `hook_kind`
  - `command`
  - `status`
  - `stdout_contract`
  - `stderr_tail`
  - `created_at`

- `codex_goal_runs`
  - `id`
  - `session_id`
  - `goal`
  - `status`
  - `proof_packet_ref`
  - `created_at`

## CLI, MCP, Slash, And Ops Surfaces

CLI:

- `arcwell codex doctor`
- `arcwell codex hook-smoke`
- `arcwell codex plugin-status`
- `arcwell codex state validate`
- `arcwell codex worktree create`
- `arcwell codex worktree list`
- `arcwell codex preflight`

MCP:

- `codex_host_doctor`
- `codex_hook_smoke`
- `codex_plugin_status`
- `codex_worktree_status`

Slash/plugin:

- `/arcwell-health`
- `/ops`
- Future `/codex-doctor`

Ops:

- Plugin cache freshness, hook status, skill reload needs, MCP server status,
  worktree ownership, stale sessions, runtime profile.

## Implementation Plan

1. Inventory existing Arcwell dev-loop checks.
   - Do not duplicate `scripts/arcwell-dev`.
   - Wrap and persist its status output.

2. Add doctor check registry.
   - Each check has key, description, status, details, and remediation.
   - Checks can be skipped with explicit reason.

3. Add hook smoke.
   - Run hook command with minimal authorized payload.
   - Validate stdout/stderr contract.
   - Enforce timeout.
   - Redact secrets.

4. Add plugin cache freshness check.
   - Compare generated dev plugin to installed cache.
   - Surface reload/new-thread requirements.

5. Add state path validator.
   - Reject separators, `..`, symlinks, unknown modes, and unallowlisted roots.
   - Test canonical path behavior.

6. Add worktree helper.
   - Validate branch names with Git.
   - Detect branch in use.
   - Warn on dirty base.
   - Keep worktrees under a managed directory.

7. Add preflight output for high-risk Arcwell changes.
   - Rust changes.
   - Plugin skills/commands/hooks.
   - MCP schema changes.
   - Worker changes.

## Anti-Mirage Traps

- A doctor file is not proof that the hook ran.
- A plugin manifest is not proof that the current Codex thread sees new tools.
- A worktree directory is not safe if the branch is already checked out
  elsewhere.
- `scripts/arcwell-dev sync` success is not hook behavior proof.
- A session path that passes string checks can still escape through symlinks.
- Tmux/HUD cleanup must only touch owned resources.

## Proof Gates

- Missing: no Codex host doctor.
- Scaffold: command shells out and prints static checks.
- Partial: doctor checks files but no real smokes.
- Local Proof: hook smoke, plugin status, state path validation, worktree
  branch validation, dirty warning, and redaction tests pass.
- Production Data Proof: current Arcwell dev plugin in a real Codex environment
  passes hook/plugin/MCP/sync checks or reports exact blockers.
- Operational: checks are persisted, visible in ops, and tied to reload/new
  thread expectations.
- Done: every claimed host workflow check has a live smoke or explicit
  non-smoke rationale and remediation.

## Severe Tests

- Hook command emits unsupported JSON; doctor fails the hook check.
- Hook command hangs; timeout is recorded.
- Hook stderr contains token-like text; output is redacted.
- Plugin cache is stale after skill change; doctor reports reload/new-thread.
- State path contains `../`, separator, or symlink escape; validation fails.
- Branch name is invalid; worktree creation refuses.
- Branch is already in use; command refuses or requires explicit mode.
- Base repo is dirty; preflight reports risk without altering files.
- Orphan cleanup ignores non-Arcwell-owned processes.
- Doctor check throws; full report still includes other checks.

## First Slice

Implement `arcwell codex doctor --json` as a wrapper over existing dev-loop
facts plus one real hook smoke. The first slice is valuable only if it catches a
stale hook/plugin condition that a file-existence check would miss.

## 2026-06-30 Refresh: Current Arcwell Shape

Arcwell now has several host-workflow guardrails that change the next step:

- `scripts/arcwell-dev smoke/sync` exist for dev plugin regeneration and smoke.
- `scripts/codex-hook-smoke` is part of the status matrix and proves
  process-level Codex hook behavior against a disposable home.
- The proof ledger records proof packets, claims, artifacts, and checks.
- `arcwell-guard` now exists as a cross-model stop gate that captures goals and
  reviews the uncommitted diff at `Stop`.
- `docs/arcwell-controller-design.md` and the Codex host-adapter skill make
  resident host-adapter work a real Arcwell boundary, not only a future note.
- TODO still keeps fresh-thread Codex app smoke and live plugin/Claude host
  proof open.

The oh-my-codex lesson should now be a host-readiness and guardrail cockpit
that composes dev-loop smoke, hook smoke, guard status, proof packets, plugin
cache state, and fresh-thread proof requirements.

## 2026-06-30 Anti-Mirage Development

Claim to build next:

> Arcwell can tell whether the current Codex/Claude host integration is ready
> for a claimed workflow by checking installed plugin freshness, hook behavior,
> guard status, MCP tool visibility, proof packets, and fresh-thread smoke.

Refutations:

- `scripts/arcwell-dev sync` passed but the current Codex thread cannot see the
  new MCP/slash surface.
- Hook smoke passes in process but no live Codex app thread fired the hook.
- Guard is installed but disabled or fail-open in a workflow that claims a
  strict completion gate.
- A proof packet exists but contains unresolved claims or missing/tampered
  artifacts.
- Host-adapter state is a stale snapshot but presented as live thread state.

Revised implementation slices:

1. Build `arcwell codex doctor --json` as a composed view of dev-loop status,
   hook smoke, guard status, proof-ledger state, and plugin cache freshness.
2. Add "fresh-thread required" as an explicit status, not a footnote.
3. Add guard status into ops and proof packets for high-risk implementation
   work.
4. Add a live fresh-thread smoke script or checklist artifact that can be
   recorded in the proof ledger.
5. Keep Claude/Codex host sync states labeled as live, degraded, stale, or
   manual snapshot.

Keep from oh-my-codex:

- real hook smokes;
- plugin cache/version checks;
- state-path validation;
- dirty-worktree warnings;
- owned-process cleanup discipline.

Do not copy:

- another tmux/HUD workflow unless Arcwell's controller needs it;
- a parallel goal system outside guard/proof/project records;
- broad team mode before single-host freshness proof.

Next proof gate:

- Local Proof: doctor fixtures distinguish synced plugin, stale plugin,
  process-hook pass, hook fail, guard disabled, guard strict, and proof-packet
  unresolved states.
- Production Data Proof: a real fresh Codex app thread runs the dev plugin,
  fires hooks, exposes MCP/slash surfaces, and records a proof packet.
