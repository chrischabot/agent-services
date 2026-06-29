use super::*;

impl Store {
    pub(crate) fn complete_wiki_job(&self, id: &str, result_json: Value) -> Result<WikiJob> {
        let existing = self
            .get_wiki_job(id)?
            .with_context(|| format!("wiki job not found before completion: {id}"))?;
        let mut result_json = result_json;
        if let Some(auto_knowledge_backlog) =
            self.auto_enqueue_knowledge_backlog_after_adapter_job(&existing, &result_json)?
            && let Some(object) = result_json.as_object_mut()
        {
            object.insert("auto_knowledge_backlog".to_string(), auto_knowledge_backlog);
        }
        if let Some(auto_editorial_decision) = self
            .auto_enqueue_knowledge_editorial_decision_after_backlog_job(&existing, &result_json)?
            && let Some(object) = result_json.as_object_mut()
        {
            object.insert(
                "auto_knowledge_cluster_editorial_decision".to_string(),
                auto_editorial_decision,
            );
        }
        if let Some(auto_execution) = self
            .auto_enqueue_knowledge_investigation_execution_after_expansion_job(
                &existing,
                &result_json,
            )?
            && let Some(object) = result_json.as_object_mut()
        {
            object.insert(
                "auto_knowledge_investigation_execution".to_string(),
                auto_execution,
            );
        }
        self.conn.execute(
            r#"
            UPDATE wiki_jobs
            SET status = 'completed',
                result_json = ?2,
                error = NULL,
                leased_until = NULL,
                worker_id = NULL,
                next_run_at = NULL,
                dead_lettered_at = NULL,
                updated_at = ?3
            WHERE id = ?1
            "#,
            params![id, serde_json::to_string(&result_json)?, now()],
        )?;
        let job = self
            .get_wiki_job(id)?
            .with_context(|| format!("completed wiki job not found: {id}"))?;
        self.record_knowledge_adapter_run_for_job(&job)?;
        Ok(job)
    }

    pub(crate) fn auto_enqueue_knowledge_backlog_after_adapter_job(
        &self,
        job: &WikiJob,
        result_json: &Value,
    ) -> Result<Option<Value>> {
        if knowledge_adapter_context_for_job(job)?.is_none() {
            return Ok(None);
        }
        let source_card_ids = adapter_source_card_ids_from_result(result_json);
        if source_card_ids.is_empty() {
            return Ok(None);
        }
        let Some(source) = self.list_watch_sources()?.into_iter().find(|source| {
            source.source_kind == "knowledge_backlog"
                && source.locator == "source-cards"
                && source.status == "active"
        }) else {
            return Ok(None);
        };
        if self.knowledge_cluster_backlog_has_active_job()? {
            return Ok(Some(json!({
                "status": "skipped",
                "reason": "knowledge_cluster_backlog_job_already_active",
                "source_card_count": source_card_ids.len(),
                "source_card_ids": source_card_ids,
            })));
        }
        let max_source_cards = source
            .metadata
            .get("max_source_cards")
            .and_then(Value::as_u64)
            .unwrap_or(100) as usize;
        let min_group_size = source
            .metadata
            .get("min_group_size")
            .and_then(Value::as_u64)
            .unwrap_or(2) as usize;
        let max_clusters = source
            .metadata
            .get("max_clusters")
            .and_then(Value::as_u64)
            .unwrap_or(12) as usize;
        match self.enqueue_knowledge_cluster_backlog_job_with_lineage(
            max_source_cards,
            min_group_size,
            max_clusters,
            Some(json!({
                "trigger": "adapter_completion",
                "parent_job_id": job.id,
                "parent_kind": job.kind,
                "source_card_count": source_card_ids.len(),
                "source_card_ids": source_card_ids,
                "watch_source_id": source.id,
                "watch_source_key": watch_source_health_key(&source)?,
                "source_kind": source.source_kind,
                "locator": source.locator,
                "cadence": source.cadence,
            })),
        ) {
            Ok(backlog_job) => Ok(Some(json!({
                "status": "enqueued",
                "job_id": backlog_job.id,
                "source_card_count": source_card_ids.len(),
                "source_card_ids": source_card_ids,
                "max_source_cards": max_source_cards.clamp(1, 500),
                "min_group_size": min_group_size.clamp(1, 20),
                "max_clusters": max_clusters.clamp(1, 50),
            }))),
            Err(error) => Ok(Some(json!({
                "status": "blocked",
                "error": excerpt(&redact_secret_like_text(&error.to_string()), 500),
                "source_card_count": source_card_ids.len(),
                "source_card_ids": source_card_ids,
            }))),
        }
    }

    pub(crate) fn auto_enqueue_knowledge_editorial_decision_after_backlog_job(
        &self,
        job: &WikiJob,
        result_json: &Value,
    ) -> Result<Option<Value>> {
        if job.kind != "knowledge_cluster_backlog" {
            return Ok(None);
        }
        let cluster_ids = knowledge_cluster_ids_from_result(result_json);
        if cluster_ids.is_empty() {
            return Ok(None);
        }
        let mut enqueued = Vec::new();
        let mut skipped = Vec::new();
        let mut errors = Vec::new();
        for cluster_id in &cluster_ids {
            if let Some(status) = self
                .get_knowledge_editorial_decision_for_cluster(cluster_id, "editorial_decide")?
                .map(|decision| decision.status)
                && matches!(status.as_str(), "completed" | "blocked")
            {
                skipped.push(json!({
                    "cluster_id": cluster_id,
                    "reason": format!("editorial_decision_{status}")
                }));
                continue;
            }
            if let Some(status) = self.knowledge_cluster_expansion_decision_status(cluster_id)?
                && matches!(status.as_str(), "completed" | "blocked")
            {
                skipped.push(json!({
                    "cluster_id": cluster_id,
                    "reason": format!("expansion_decision_{status}")
                }));
                continue;
            }
            if self.knowledge_cluster_editorial_decision_has_active_job(cluster_id)? {
                skipped.push(json!({
                    "cluster_id": cluster_id,
                    "reason": "knowledge_cluster_editorial_decide_job_already_active"
                }));
                continue;
            }
            if self.knowledge_cluster_expansion_has_active_job(cluster_id)? {
                skipped.push(json!({
                    "cluster_id": cluster_id,
                    "reason": "knowledge_cluster_expand_job_already_active"
                }));
                continue;
            }
            let cluster = self
                .get_knowledge_cluster(cluster_id)?
                .with_context(|| format!("knowledge cluster not found: {cluster_id}"))?;
            match self.enqueue_knowledge_cluster_editorial_decision_job_with_lineage(
                cluster_id,
                true,
                Some(json!({
                    "trigger": "backlog_completion",
                    "parent_job_id": job.id.clone(),
                    "parent_kind": job.kind.clone(),
                    "cluster_id": cluster.id.clone(),
                    "topic": cluster.topic.clone(),
                    "source_card_count": cluster.source_card_ids.len(),
                    "source_card_ids": cluster.source_card_ids.clone(),
                    "boundary": "Backlog completion queues an editorial decision first; expansion is only a decider follow-up."
                })),
            ) {
                Ok(editorial_job) => enqueued.push(json!({
                    "cluster_id": cluster_id,
                    "job_id": editorial_job.id,
                })),
                Err(error) => errors.push(json!({
                    "cluster_id": cluster_id,
                    "error": excerpt(&redact_secret_like_text(&error.to_string()), 500),
                })),
            }
        }
        let status = if !enqueued.is_empty() {
            "enqueued"
        } else if !errors.is_empty() {
            "blocked"
        } else {
            "skipped"
        };
        Ok(Some(json!({
            "status": status,
            "cluster_count": cluster_ids.len(),
            "enqueued": enqueued,
            "skipped": skipped,
            "errors": errors,
        })))
    }

    pub(crate) fn auto_enqueue_knowledge_investigation_execution_after_expansion_job(
        &self,
        job: &WikiJob,
        result_json: &Value,
    ) -> Result<Option<Value>> {
        if job.kind != "knowledge_cluster_expand" {
            return Ok(None);
        }
        let Some(cluster_id) = result_json.get("cluster_id").and_then(Value::as_str) else {
            return Ok(None);
        };
        validate_id(cluster_id)?;
        let task_count = result_json
            .get("investigation_task_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if task_count == 0 {
            return Ok(Some(json!({
                "status": "skipped",
                "cluster_id": cluster_id,
                "reason": "no_investigation_tasks",
            })));
        }
        if let Some(status) =
            self.knowledge_cluster_investigation_execution_decision_status(cluster_id)?
            && matches!(status.as_str(), "completed" | "blocked")
        {
            return Ok(Some(json!({
                "status": "skipped",
                "cluster_id": cluster_id,
                "reason": format!("investigation_execution_decision_{status}"),
            })));
        }
        if self.knowledge_cluster_investigation_execution_has_active_job(cluster_id)? {
            return Ok(Some(json!({
                "status": "skipped",
                "cluster_id": cluster_id,
                "reason": "knowledge_cluster_investigation_execute_job_already_active",
            })));
        }
        match self.enqueue_knowledge_cluster_investigation_execution_job_with_lineage(
            cluster_id,
            Some(json!({
                "trigger": "expansion_completion",
                "parent_job_id": job.id,
                "parent_kind": job.kind,
                "cluster_id": cluster_id,
                "wiki_page_id": result_json.get("wiki_page_id").cloned().unwrap_or(Value::Null),
                "report_id": result_json.get("report_id").cloned().unwrap_or(Value::Null),
                "editorial_decision_id": result_json
                    .get("editorial_decision_id")
                    .cloned()
                    .unwrap_or(Value::Null),
                "digest_candidate_id": result_json
                    .get("digest_candidate_id")
                    .cloned()
                    .unwrap_or(Value::Null),
                "investigation_research_run_id": result_json
                    .get("investigation_research_run_id")
                    .cloned()
                    .unwrap_or(Value::Null),
                "investigation_task_count": task_count,
            })),
        ) {
            Ok(execution_job) => Ok(Some(json!({
                "status": "enqueued",
                "cluster_id": cluster_id,
                "job_id": execution_job.id,
                "investigation_task_count": task_count,
            }))),
            Err(error) => Ok(Some(json!({
                "status": "blocked",
                "cluster_id": cluster_id,
                "error": excerpt(&redact_secret_like_text(&error.to_string()), 500),
                "investigation_task_count": task_count,
            }))),
        }
    }

    pub(crate) fn defer_wiki_job(
        &self,
        id: &str,
        result_json: Value,
        next_run_at: &str,
    ) -> Result<WikiJob> {
        validate_timestamp(next_run_at)?;
        self.conn.execute(
            r#"
            UPDATE wiki_jobs
            SET status = 'deferred',
                result_json = ?2,
                error = NULL,
                attempts = CASE WHEN attempts > 0 THEN attempts - 1 ELSE attempts END,
                leased_until = NULL,
                worker_id = NULL,
                next_run_at = ?3,
                dead_lettered_at = NULL,
                updated_at = ?4
            WHERE id = ?1
            "#,
            params![id, serde_json::to_string(&result_json)?, next_run_at, now()],
        )?;
        self.get_wiki_job(id)?
            .with_context(|| format!("deferred wiki job not found: {id}"))
    }

    pub(crate) fn fail_wiki_job(&self, id: &str, error: &str) -> Result<WikiJob> {
        self.mark_digest_alert_tick_failed_for_job(id, error)?;
        let job = self
            .get_wiki_job(id)?
            .with_context(|| format!("failed wiki job not found before update: {id}"))?;
        let dead_letter = job.attempts >= job.max_attempts;
        let status = if dead_letter {
            "dead_lettered"
        } else {
            "failed"
        };
        let next_run_at = if dead_letter {
            None
        } else {
            Some(now_plus_seconds(retry_backoff_seconds(job.attempts)))
        };
        let dead_lettered_at = if dead_letter { Some(now()) } else { None };
        let error = redact_secret_like_text(error);
        self.conn.execute(
            r#"
            UPDATE wiki_jobs
            SET status = ?2,
                result_json = NULL,
                error = ?3,
                leased_until = NULL,
                worker_id = NULL,
                next_run_at = ?4,
                dead_lettered_at = ?5,
                updated_at = ?6
            WHERE id = ?1
            "#,
            params![
                id,
                status,
                excerpt(&error, 2000),
                next_run_at,
                dead_lettered_at,
                now()
            ],
        )?;
        let job = self
            .get_wiki_job(id)?
            .with_context(|| format!("failed wiki job not found: {id}"))?;
        self.record_knowledge_adapter_run_for_job(&job)?;
        Ok(job)
    }

    pub(crate) fn record_knowledge_adapter_run_for_job(&self, job: &WikiJob) -> Result<()> {
        let Some(context) = knowledge_adapter_context_for_job(job)? else {
            return Ok(());
        };
        let result = job.result_json.as_ref();
        let source_card_ids = result
            .and_then(|value| value.get("source_cards"))
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let cursor_key = result
            .and_then(|value| value.get("cursor").or_else(|| value.get("cursor_key")))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| context.cursor_key.clone());
        let cursor_before = result
            .and_then(|value| value.get("cursor_before"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| {
                if matches!(job.status.as_str(), "failed" | "dead_lettered") {
                    cursor_key
                        .as_deref()
                        .and_then(|key| self.get_cursor(key).ok().flatten())
                        .map(|cursor| cursor.value)
                } else {
                    None
                }
            });
        let cursor_after = result
            .and_then(|value| {
                value
                    .get("cursor_value")
                    .or_else(|| value.get("new_cursor"))
            })
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let raw_count = result
            .and_then(|value| value.get("raw_count").or_else(|| value.get("seen")))
            .and_then(Value::as_i64)
            .or_else(|| {
                result
                    .and_then(|value| value.get("count"))
                    .and_then(Value::as_i64)
            })
            .unwrap_or(source_card_ids.len() as i64);
        let accepted_count = result
            .and_then(|value| {
                value
                    .get("accepted_count")
                    .or_else(|| value.get("imported"))
            })
            .and_then(Value::as_i64)
            .or_else(|| {
                result
                    .and_then(|value| value.get("count"))
                    .and_then(Value::as_i64)
            })
            .unwrap_or(source_card_ids.len() as i64);
        let rejected_count = result
            .and_then(|value| value.get("rejected"))
            .and_then(Value::as_i64)
            .or_else(|| {
                result
                    .and_then(|value| value.get("skipped_items"))
                    .and_then(Value::as_array)
                    .map(|items| items.len() as i64)
            })
            .unwrap_or(0);
        let duplicate_count = result
            .and_then(|value| value.get("skipped_duplicates"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let status = match job.status.as_str() {
            "completed" => "completed",
            "deferred" => "deferred",
            "dead_lettered" => "dead_lettered",
            _ if job.error.is_some() => "failed",
            other => other,
        };
        let error_kind = job.error.as_deref().map(classify_source_adapter_error_kind);
        let id = format!("kadapter-{}", &sha256(job.id.as_bytes())[..16]);
        let metadata = json!({
            "contract_version": 1,
            "input": policy_safe_job_input(&job.input_json),
            "result_shape": result.map(adapter_result_shape).unwrap_or_else(|| json!({})),
            "job_attempts": job.attempts,
            "job_max_attempts": job.max_attempts,
            "next_run_at": job.next_run_at,
        });
        self.conn.execute(
            r#"
            INSERT INTO knowledge_adapter_runs
              (id, job_id, adapter_kind, provider, source_kind, locator, status,
               error_kind, error, cursor_key, cursor_before, cursor_after,
               source_card_ids_json, raw_count, accepted_count, rejected_count,
               duplicate_count, metadata_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?19)
            ON CONFLICT(job_id) DO UPDATE SET
              status = excluded.status,
              error_kind = excluded.error_kind,
              error = excluded.error,
              cursor_key = COALESCE(excluded.cursor_key, knowledge_adapter_runs.cursor_key),
              cursor_before = COALESCE(excluded.cursor_before, knowledge_adapter_runs.cursor_before),
              cursor_after = COALESCE(excluded.cursor_after, knowledge_adapter_runs.cursor_after),
              source_card_ids_json = excluded.source_card_ids_json,
              raw_count = excluded.raw_count,
              accepted_count = excluded.accepted_count,
              rejected_count = excluded.rejected_count,
              duplicate_count = excluded.duplicate_count,
              metadata_json = excluded.metadata_json,
              updated_at = excluded.updated_at
            "#,
            params![
                id,
                job.id,
                job.kind,
                context.provider,
                context.source_kind,
                context.locator,
                status,
                error_kind,
                job.error.as_deref().map(|error| excerpt(error, 2000)),
                cursor_key,
                cursor_before,
                cursor_after,
                serde_json::to_string(&source_card_ids)?,
                raw_count,
                accepted_count,
                rejected_count,
                duplicate_count,
                metadata.to_string(),
                now(),
            ],
        )?;
        Ok(())
    }

    pub(crate) fn mark_digest_alert_tick_failed_for_job(
        &self,
        job_id: &str,
        error: &str,
    ) -> Result<()> {
        validate_id(job_id)?;
        let Some(tick) = self.digest_alert_tick_for_job(job_id)? else {
            return Ok(());
        };
        if matches!(
            tick.status.as_str(),
            "sent" | "partial" | "empty" | "deferred" | "blocked" | "failed"
        ) {
            return Ok(());
        }
        let sanitized = sanitize_radar_delivery_error(error)?;
        self.update_digest_alert_tick(
            &tick.id,
            "failed",
            &tick.candidate_ids,
            &tick.delivery_ids,
            Some(&sanitized),
        )?;
        Ok(())
    }

    pub(crate) fn digest_alert_tick_for_job(
        &self,
        job_id: &str,
    ) -> Result<Option<DigestAlertTick>> {
        validate_id(job_id)?;
        self.conn
            .query_row(
                r#"
                SELECT id, schedule_id, tick_key, due_at, status, job_id,
                       candidate_ids_json, delivery_ids_json, error, created_at, updated_at
                FROM digest_alert_ticks
                WHERE job_id = ?1
                "#,
                params![job_id],
                digest_alert_tick_from_row,
            )
            .optional()
            .map_err(Into::into)
    }
}
