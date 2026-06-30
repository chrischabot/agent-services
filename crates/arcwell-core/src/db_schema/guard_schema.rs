use crate::*;

/// Schema for the cross-model stop-gate guardrail.
///
/// `guard_goals` persists the user's stated goal / definition-of-done per session
/// (captured at SessionStart / UserPromptSubmit) so the Stop-time review has a stable
/// target to judge the work against — a completion gate becomes a correctness gate.
///
/// `guard_reviews` is the durable verdict ledger. The count of `block`/`capped` rows
/// per session is the bounded iteration counter that prevents infinite block→continue
/// loops (the failure mode every comparable community hook gets wrong).
pub(crate) fn ensure_guard_schema_on(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS guard_goals (
          id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          cwd TEXT,
          source TEXT NOT NULL DEFAULT 'user-prompt-submit',
          goal TEXT NOT NULL,
          success_criteria TEXT,
          status TEXT NOT NULL DEFAULT 'active',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          CHECK(status IN ('active', 'closed'))
        );

        CREATE INDEX IF NOT EXISTS idx_guard_goals_session
        ON guard_goals(session_id, created_at DESC);

        CREATE TABLE IF NOT EXISTS guard_reviews (
          id TEXT PRIMARY KEY,
          session_id TEXT NOT NULL,
          goal_id TEXT,
          attempt INTEGER NOT NULL DEFAULT 1,
          worker TEXT NOT NULL,
          reviewer TEXT NOT NULL,
          verdict TEXT NOT NULL,
          reason TEXT NOT NULL DEFAULT '',
          diff_summary TEXT,
          created_at TEXT NOT NULL,
          CHECK(verdict IN ('allow', 'block', 'skipped', 'error', 'capped')),
          FOREIGN KEY(goal_id) REFERENCES guard_goals(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_guard_reviews_session
        ON guard_reviews(session_id, created_at DESC);
        "#,
    )?;
    Ok(())
}
