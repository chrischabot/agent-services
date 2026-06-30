use super::*;

impl Store {
    /// Persist or clear the guard kill switch (durable, survives restarts).
    pub fn guard_set_enabled(&self, enabled: bool) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES ('guard_enabled', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![if enabled { "1" } else { "0" }],
        )?;
        Ok(())
    }

    /// Guard is enabled unless explicitly disabled (the installed plugin is the opt-in).
    pub fn guard_enabled(&self) -> Result<bool> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'guard_enabled'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value.as_deref() != Some("0"))
    }

    /// Record the user's stated goal / definition-of-done for a session.
    pub fn guard_capture_goal(
        &self,
        session_id: &str,
        cwd: Option<&str>,
        source: &str,
        goal: &str,
        success_criteria: Option<&str>,
    ) -> Result<String> {
        let id = format!("guard-goal-{}", &Uuid::new_v4().simple().to_string()[..16]);
        let ts = now();
        self.conn.execute(
            r#"
            INSERT INTO guard_goals
              (id, session_id, cwd, source, goal, success_criteria, status, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?7)
            "#,
            params![id, session_id, cwd, source, goal, success_criteria, ts],
        )?;
        Ok(id)
    }

    /// The most recent active goal for a session (the Stop-gate's review target).
    pub fn guard_active_goal(&self, session_id: &str) -> Result<Option<serde_json::Value>> {
        let row = self
            .conn
            .query_row(
                r#"
                SELECT id, goal, success_criteria, created_at
                FROM guard_goals
                WHERE session_id = ?1 AND status = 'active'
                ORDER BY created_at DESC
                LIMIT 1
                "#,
                params![session_id],
                |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "goal": row.get::<_, String>(1)?,
                        "success_criteria": row.get::<_, Option<String>>(2)?,
                        "created_at": row.get::<_, String>(3)?,
                    }))
                },
            )
            .optional()?;
        Ok(row)
    }

    /// Append a verdict to the review ledger.
    #[allow(clippy::too_many_arguments)]
    pub fn guard_record_review(
        &self,
        session_id: &str,
        goal_id: Option<&str>,
        attempt: i64,
        worker: &str,
        reviewer: &str,
        verdict: &str,
        reason: &str,
        diff_summary: Option<&str>,
    ) -> Result<String> {
        let id = format!(
            "guard-review-{}",
            &Uuid::new_v4().simple().to_string()[..16]
        );
        self.conn.execute(
            r#"
            INSERT INTO guard_reviews
              (id, session_id, goal_id, attempt, worker, reviewer, verdict, reason, diff_summary, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                id,
                session_id,
                goal_id,
                attempt,
                worker,
                reviewer,
                verdict,
                reason,
                diff_summary,
                now()
            ],
        )?;
        Ok(id)
    }

    /// Number of times this session has been blocked (or hard-capped) — the bounded
    /// iteration counter that backstops the loop.
    pub fn guard_block_streak(&self, session_id: &str) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM guard_reviews WHERE session_id = ?1 AND verdict IN ('block', 'capped')",
            params![session_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Recent captured goals + review verdicts, optionally scoped to one session.
    pub fn guard_status(&self, session_id: Option<&str>, limit: i64) -> Result<serde_json::Value> {
        let limit = limit.clamp(1, 500);
        let goals = self.guard_collect(
            "SELECT id, session_id, source, goal, success_criteria, status, created_at \
             FROM guard_goals {where} ORDER BY created_at DESC LIMIT ?2",
            session_id,
            limit,
            |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "session_id": row.get::<_, String>(1)?,
                    "source": row.get::<_, String>(2)?,
                    "goal": row.get::<_, String>(3)?,
                    "success_criteria": row.get::<_, Option<String>>(4)?,
                    "status": row.get::<_, String>(5)?,
                    "created_at": row.get::<_, String>(6)?,
                }))
            },
        )?;
        let reviews = self.guard_collect(
            "SELECT id, session_id, attempt, worker, reviewer, verdict, reason, created_at \
             FROM guard_reviews {where} ORDER BY created_at DESC LIMIT ?2",
            session_id,
            limit,
            |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "session_id": row.get::<_, String>(1)?,
                    "attempt": row.get::<_, i64>(2)?,
                    "worker": row.get::<_, String>(3)?,
                    "reviewer": row.get::<_, String>(4)?,
                    "verdict": row.get::<_, String>(5)?,
                    "reason": row.get::<_, String>(6)?,
                    "created_at": row.get::<_, String>(7)?,
                }))
            },
        )?;
        Ok(serde_json::json!({ "goals": goals, "reviews": reviews }))
    }

    fn guard_collect<F>(
        &self,
        sql_template: &str,
        session_id: Option<&str>,
        limit: i64,
        map: F,
    ) -> Result<Vec<serde_json::Value>>
    where
        F: Fn(&rusqlite::Row<'_>) -> rusqlite::Result<serde_json::Value>,
    {
        // `?1` is the session filter (NULL => match all), `?2` is the limit.
        let where_clause = if session_id.is_some() {
            "WHERE session_id = ?1"
        } else {
            "WHERE (?1 IS NULL)"
        };
        let sql = sql_template.replace("{where}", where_clause);
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![session_id, limit], |row| map(row))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}
