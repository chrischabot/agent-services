# Arcwell Guard

A portable **cross-model stop-gate** for Claude Code **and** Codex CLI (one plugin, same
`hooks.json` schema in both runtimes). It exists to stop an agent from *claiming* a task is
done when it actually shipped a partial fix or a workaround.

## What it does

1. **Captures the goal** at `SessionStart` / `UserPromptSubmit` and persists it
   (`arcwell guard capture-goal`). This gives the review a stable target — turning a
   completion gate into a correctness gate.
2. **Reviews the work at `Stop`** (`arcwell guard stop-review`): the *opposite* model
   reviews the actual uncommitted diff against the captured goal —
   **Codex reviews Claude's work, Claude reviews Codex's** — using an adversarial
   "do not trust the report" prompt that specifically hunts for workarounds (new parallel
   systems, bypassed components, stubs, disabled checks). New **untracked** files are
   included, so a brand-new parallel system is actually seen.
3. **Blocks finishing** with `{"decision":"block","reason":...}` when the reviewer returns
   `BLOCK:`; the reason is fed back so the agent fixes it before stopping.

## Why it won't wreck your session (the safety rails)

- **Recursion guard** — the reviewer subprocess runs with `ARCWELL_GUARD_DISABLE=1`, so a
  reviewer that triggers its own Stop hook never recurses.
- **Bounded iteration count** — after N blocks (default 3) for a session the gate
  hard-allows with a warning. It can never trap the agent in a block loop.
- **Default-allow** on non-edit turns (no diff) and outside git repos.
- **Fail-open** — if the reviewer errors or is missing, the turn is allowed (set
  `ARCWELL_GUARD_STRICT=1` to fail-closed instead).
- **Kill switch** — `arcwell guard disable` (persisted) or `ARCWELL_GUARD_DISABLE=1`
  (per-shell).

## Configuration (env vars)

| Var | Effect |
|---|---|
| `ARCWELL_GUARD_DISABLE=1` | Bypass the gate entirely (also the recursion guard). |
| `ARCWELL_GUARD_STRICT=1` | Block instead of fail-open when the reviewer can't run. |
| `ARCWELL_GUARD_MAX_BLOCKS=N` | Blocks per session before hard-allow (default 3). |
| `ARCWELL_GUARD_REVIEWER=claude\|codex` | Force the reviewing model. |
| `ARCWELL_GUARD_WORKER=claude\|codex` | Override runtime auto-detection. |

## Per-project sharpening

Drop a `.arcwell-guardrails.md` at the repo root listing owned capabilities the agent must
not duplicate or bypass (e.g. "All LLM calls go through the owned gateway — never a second
provider path"). Its contents are fed to the reviewer as extra block criteria.

## Requirements

The `arcwell` binary must be on `PATH`, plus the reviewing model's CLI (`claude` and/or
`codex`). Inspect activity with `arcwell guard status [--session-id <id>]`.
