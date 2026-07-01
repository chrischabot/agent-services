use super::*;

impl Store {
    pub(crate) fn execute_x_recent_search(
        &self,
        input: &Value,
        job_id: Option<&str>,
    ) -> Result<Value> {
        let query = input
            .get("query")
            .and_then(Value::as_str)
            .context("x_recent_search missing query")?;
        let max_results = input
            .get("max_results")
            .and_then(Value::as_u64)
            .unwrap_or(10) as usize;
        let endpoint =
            std::env::var("ARCWELL_X_API_BASE").unwrap_or_else(|_| "https://api.x.com".to_string());
        let transport = input.get("transport").and_then(Value::as_str);
        let response = if transport
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        {
            let transport = XProviderTransport::parse(transport)?;
            self.x_recent_search_with_base_transport_and_job_id(
                query,
                max_results,
                &endpoint,
                transport,
                job_id,
            )?
        } else {
            self.x_recent_search_with_base_default_and_job_id(
                query,
                max_results,
                &endpoint,
                job_id,
            )?
        };
        Ok(json!(response))
    }

    pub(crate) fn execute_x_import_bookmarks(&self, input: &Value) -> Result<Value> {
        let bookmark_days = input
            .get("bookmark_days")
            .and_then(Value::as_i64)
            .unwrap_or(92);
        let max_bookmarks = input
            .get("max_bookmarks")
            .and_then(Value::as_u64)
            .unwrap_or(100) as usize;
        let transport = input.get("transport").and_then(Value::as_str);
        let endpoint =
            std::env::var("ARCWELL_X_API_BASE").unwrap_or_else(|_| "https://api.x.com".to_string());
        let response = if transport
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        {
            let transport = XProviderTransport::parse(transport)?;
            self.x_import_bookmarks_with_base_and_transport(
                bookmark_days,
                max_bookmarks,
                &endpoint,
                transport,
            )?
        } else {
            self.x_import_bookmarks_with_base(bookmark_days, max_bookmarks, &endpoint)?
        };
        if let Some(lineage) = input.get("lineage")
            && let Some(source_key) = lineage.get("watch_source_key").and_then(Value::as_str)
        {
            let source_kind = lineage
                .get("health_source_kind")
                .and_then(Value::as_str)
                .unwrap_or("x_import_bookmarks");
            let locator = lineage
                .get("locator")
                .and_then(Value::as_str)
                .unwrap_or("bookmarks");
            let next_run_at = lineage
                .get("cadence")
                .and_then(Value::as_str)
                .and_then(watch_source_cadence_seconds)
                .map(now_plus_seconds);
            self.record_source_success(SourceHealthUpdate {
                key: source_key,
                provider: "x",
                source_kind,
                locator,
                last_item_id: response.items.first().map(|item| item.x_id.as_str()),
                last_item_date: response
                    .items
                    .first()
                    .and_then(|item| item.created_at.as_deref()),
                cursor_key: None,
                cursor_value: response.next_token.as_deref(),
                next_run_at: next_run_at.as_deref(),
            })?;
        }
        Ok(json!(response))
    }

    pub(crate) fn execute_x_monitor_watch_source(&self, input: &Value) -> Result<Value> {
        let handle = input
            .get("handle")
            .and_then(Value::as_str)
            .context("x_monitor_watch_source missing handle")?;
        let max_results = input
            .get("max_results")
            .and_then(Value::as_u64)
            .unwrap_or(10) as usize;
        let endpoint =
            std::env::var("ARCWELL_X_API_BASE").unwrap_or_else(|_| "https://api.x.com".to_string());
        let response = self.x_monitor_watch_source_with_base(handle, max_results, &endpoint)?;
        Ok(json!(response))
    }

    pub(crate) fn execute_knowledge_cluster_expand(&self, input: &Value) -> Result<Value> {
        let cluster_id = input
            .get("cluster_id")
            .and_then(Value::as_str)
            .context("knowledge_cluster_expand missing cluster_id")?;
        let create_digest = input
            .get("create_digest")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let report = self.expand_knowledge_cluster(cluster_id, create_digest)?;
        Ok(json!({
            "cluster_id": report.cluster.id,
            "wiki_page_id": report.wiki_page.id,
            "report_id": report.report.id,
            "editorial_decision_id": report.editorial_decision.id,
            "digest_candidate_id": report.digest_candidate.as_ref().map(|candidate| candidate.id.clone()),
            "investigation_research_run_id": report.investigation.research_run.id,
            "investigation_task_count": report.investigation.tasks.len(),
            "investigation_reused_existing": report.investigation.reused_existing,
            "source_card_count": report.source_cards.len(),
            "quality_findings": report.quality_findings,
            "status": "completed"
        }))
    }

    pub(crate) fn execute_knowledge_cluster_editorial_decide(
        &self,
        input: &Value,
    ) -> Result<Value> {
        let cluster_id = input
            .get("cluster_id")
            .and_then(Value::as_str)
            .context("knowledge_cluster_editorial_decide missing cluster_id")?;
        let auto_enqueue = input
            .get("auto_enqueue")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let report = self.decide_knowledge_cluster_editorial(cluster_id, auto_enqueue)?;
        Ok(json!({
            "cluster_id": report.cluster.id,
            "editorial_decision_id": report.editorial_decision.id,
            "recommended_action": report.recommended_action,
            "decision_status": report.editorial_decision.status,
            "matched_wiki_page_id": report.matched_wiki_page.as_ref().map(|page| page.id.clone()),
            "digest_candidate_id": report.editorial_decision.digest_candidate_id,
            "enqueued_job_id": report.enqueued_job.as_ref().map(|job| job.id.clone()),
            "enqueued_job_kind": report.enqueued_job.as_ref().map(|job| job.kind.clone()),
            "source_card_count": report.source_card_count,
            "proof_level": report.proof_level,
            "status": "completed"
        }))
    }

    pub(crate) fn execute_knowledge_cluster_model_write(&self, input: &Value) -> Result<Value> {
        let cluster_id = input
            .get("cluster_id")
            .and_then(Value::as_str)
            .context("knowledge_cluster_model_write missing cluster_id")?;
        let model_provider = input
            .get("model_provider")
            .and_then(Value::as_str)
            .unwrap_or("mock");
        let model_name = input
            .get("model_name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let endpoint = input
            .get("endpoint")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let timeout_seconds = input.get("timeout_seconds").and_then(Value::as_u64);
        let create_digest = input
            .get("create_digest")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let report =
            self.expand_knowledge_cluster_with_model_writer(KnowledgeClusterWriterModelInput {
                cluster_id: cluster_id.to_string(),
                model_provider: model_provider.to_string(),
                model_name,
                endpoint,
                timeout_seconds,
                create_digest,
            })?;
        if let Some(lineage) = input.get("lineage")
            && let Some(source_key) = lineage.get("watch_source_key").and_then(Value::as_str)
        {
            let locator = lineage
                .get("locator")
                .and_then(Value::as_str)
                .unwrap_or(report.cluster.id.as_str());
            self.record_source_success(SourceHealthUpdate {
                key: source_key,
                provider: "arcwell",
                source_kind: "knowledge_model_write",
                locator,
                last_item_id: Some(report.cluster.id.as_str()),
                last_item_date: None,
                cursor_key: None,
                cursor_value: None,
                next_run_at: Some(&now_plus_seconds(self.watch_source_next_run_seconds(
                    "knowledge_model_write",
                    locator,
                    6 * 60 * 60,
                ))),
            })?;
        }
        Ok(json!({
            "cluster_id": report.cluster.id,
            "wiki_page_id": report.wiki_page.id,
            "report_id": report.report.id,
            "editorial_decision_id": report.editorial_decision.id,
            "digest_candidate_id": report.digest_candidate.as_ref().map(|candidate| candidate.id.clone()),
            "investigation_research_run_id": report.investigation.research_run.id,
            "investigation_task_count": report.investigation.tasks.len(),
            "investigation_reused_existing": report.investigation.reused_existing,
            "source_card_count": report.source_cards.len(),
            "quality_findings": report.quality_findings,
            "model_writer": report.metadata.get("model_writer"),
            "status": "completed"
        }))
    }

    pub(crate) fn execute_knowledge_entity_resolution_model(&self, input: &Value) -> Result<Value> {
        let left_entity_id = input
            .get("left_entity_id")
            .and_then(Value::as_str)
            .context("knowledge_entity_resolution_model missing left_entity_id")?;
        let right_entity_id = input
            .get("right_entity_id")
            .and_then(Value::as_str)
            .context("knowledge_entity_resolution_model missing right_entity_id")?;
        let model_provider = input
            .get("model_provider")
            .and_then(Value::as_str)
            .unwrap_or("mock");
        let model_name = input
            .get("model_name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let endpoint = input
            .get("endpoint")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let timeout_seconds = input.get("timeout_seconds").and_then(Value::as_u64);
        let invocation_result =
            self.invoke_knowledge_entity_resolution_model(KnowledgeEntityResolutionModelInput {
                left_entity_id: left_entity_id.to_string(),
                right_entity_id: right_entity_id.to_string(),
                model_provider: model_provider.to_string(),
                model_name,
                endpoint,
                timeout_seconds,
            });
        let invocation = match invocation_result {
            Ok(invocation) => invocation,
            Err(error) => {
                let source_key = input
                    .get("lineage")
                    .and_then(|lineage| lineage.get("watch_source_key"))
                    .and_then(Value::as_str)
                    .unwrap_or("knowledge:entity-resolution:entities");
                let locator = input
                    .get("lineage")
                    .and_then(|lineage| lineage.get("locator"))
                    .and_then(Value::as_str)
                    .unwrap_or("entities");
                let _ = self.record_source_failure(
                    source_key,
                    "knowledge_entity_resolution",
                    "knowledge_entity_resolution",
                    locator,
                    &error.to_string(),
                );
                bail!("{}", redact_secret_like_text(&error.to_string()));
            }
        };
        let source_key = input
            .get("lineage")
            .and_then(|lineage| lineage.get("watch_source_key"))
            .and_then(Value::as_str)
            .unwrap_or("knowledge:entity-resolution:entities");
        let locator = input
            .get("lineage")
            .and_then(|lineage| lineage.get("locator"))
            .and_then(Value::as_str)
            .unwrap_or("entities");
        self.record_source_success(SourceHealthUpdate {
            key: source_key,
            provider: "arcwell",
            source_kind: "knowledge_entity_resolution",
            locator,
            last_item_id: Some(invocation.resolution.id.as_str()),
            last_item_date: None,
            cursor_key: None,
            cursor_value: None,
            next_run_at: Some(&now_plus_seconds(self.watch_source_next_run_seconds(
                "knowledge_entity_resolution",
                locator,
                6 * 60 * 60,
            ))),
        })?;
        Ok(json!({
            "status": "completed",
            "resolution_id": invocation.resolution.id,
            "left_entity_id": invocation.resolution.left_entity_id,
            "right_entity_id": invocation.resolution.right_entity_id,
            "decision": invocation.resolution.decision,
            "resolution_status": invocation.resolution.status,
            "resolver": invocation.resolution.resolver,
            "source_card_ids": invocation.resolution.source_card_ids,
            "model_provider": invocation.model_provider,
            "model_name": invocation.model_name,
            "prompt_version": invocation.prompt_version,
            "cost_decision_id": invocation.cost_decision_id,
            "proof_level": invocation.proof_level,
            "boundary": "Scheduled model entity resolution writes review-only proposals only; it does not merge entities or create relations."
        }))
    }

    pub(crate) fn execute_knowledge_cluster_backlog(&self, input: &Value) -> Result<Value> {
        let max_source_cards = input
            .get("max_source_cards")
            .and_then(Value::as_u64)
            .unwrap_or(100) as usize;
        let min_group_size = input
            .get("min_group_size")
            .and_then(Value::as_u64)
            .unwrap_or(2) as usize;
        let max_clusters = input
            .get("max_clusters")
            .and_then(Value::as_u64)
            .unwrap_or(12) as usize;
        let report =
            self.cluster_source_card_backlog(max_source_cards, min_group_size, max_clusters)?;
        let cluster_ids = report
            .projections
            .iter()
            .map(|projection| projection.cluster.id.clone())
            .collect::<Vec<_>>();
        let last_item_id = report
            .projections
            .iter()
            .flat_map(|projection| projection.cluster.source_card_ids.iter())
            .last()
            .cloned();
        self.record_source_success(SourceHealthUpdate {
            key: "knowledge:source-card-backlog",
            provider: "arcwell",
            source_kind: "knowledge_backlog",
            locator: "source-cards",
            last_item_id: last_item_id.as_deref(),
            last_item_date: None,
            cursor_key: None,
            cursor_value: None,
            next_run_at: Some(&now_plus_seconds(self.watch_source_next_run_seconds(
                "knowledge_backlog",
                "source-cards",
                6 * 60 * 60,
            ))),
        })?;
        Ok(json!({
            "status": "completed",
            "inspected": report.inspected,
            "accepted": report.accepted,
            "skipped": report.skipped,
            "groups_considered": report.groups_considered,
            "clusters_created": cluster_ids.len(),
            "cluster_ids": cluster_ids,
            "warnings": report.warnings,
        }))
    }

    pub(crate) fn execute_knowledge_cluster_model_propose(&self, input: &Value) -> Result<Value> {
        let query = input
            .get("query")
            .and_then(Value::as_str)
            .context("knowledge_cluster_model_propose missing query")?;
        let query = normalize_knowledge_model_cluster_query(query)?;
        let model_provider = input
            .get("model_provider")
            .and_then(Value::as_str)
            .unwrap_or("mock");
        let model_name = input
            .get("model_name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let endpoint = input
            .get("endpoint")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let timeout_seconds = input.get("timeout_seconds").and_then(Value::as_u64);
        let max_source_cards = input
            .get("max_source_cards")
            .and_then(Value::as_u64)
            .unwrap_or(24) as usize;
        let max_clusters = input
            .get("max_clusters")
            .and_then(Value::as_u64)
            .unwrap_or(6) as usize;
        let broad_source_card_sweep = knowledge_model_cluster_query_is_broad(&query);
        let clustered_source_card_ids = self.knowledge_clustered_source_card_ids()?;
        let candidates = if broad_source_card_sweep {
            self.list_source_cards()?
        } else {
            self.search_source_cards(&query)?
        };
        let mut skipped_clustered = 0usize;
        let mut skipped_generated_only = 0usize;
        let mut source_cards = Vec::new();
        for card in candidates {
            if clustered_source_card_ids.contains(&card.id) {
                skipped_clustered += 1;
                continue;
            }
            if source_card_is_generated_only_evidence(&card) {
                skipped_generated_only += 1;
                continue;
            }
            source_cards.push(card);
            if source_cards.len() >= max_source_cards.clamp(1, 80) {
                break;
            }
        }
        let source_card_ids = source_cards
            .iter()
            .map(|card| card.id.clone())
            .collect::<Vec<_>>();
        if source_card_ids.is_empty() {
            self.record_source_success(SourceHealthUpdate {
                key: &format!("knowledge:model-clusters:{query}"),
                provider: "arcwell",
                source_kind: "knowledge_model_clusters",
                locator: &query,
                last_item_id: None,
                last_item_date: None,
                cursor_key: None,
                cursor_value: None,
                next_run_at: Some(&now_plus_seconds(self.watch_source_next_run_seconds(
                    "knowledge_model_clusters",
                    &query,
                    6 * 60 * 60,
                ))),
            })?;
            return Ok(json!({
                "status": "skipped_no_source_cards",
                "query": query,
                "source_card_count": 0,
                "broad_source_card_sweep": broad_source_card_sweep,
                "skipped_clustered_source_cards": skipped_clustered,
                "skipped_generated_only_source_cards": skipped_generated_only,
                "cluster_ids": [],
                "boundary": "No model provider was invoked because no source-card evidence matched the query."
            }));
        }
        let invocation =
            self.invoke_knowledge_cluster_model(KnowledgeClusterProposalModelInput {
                source_card_ids: source_card_ids.clone(),
                model_provider: model_provider.to_string(),
                model_name,
                endpoint,
                timeout_seconds,
                max_clusters,
            })?;
        let cluster_ids = invocation
            .clusters
            .iter()
            .map(|cluster| cluster.id.clone())
            .collect::<Vec<_>>();
        let last_item_id = source_card_ids.last().map(String::as_str);
        let last_item_date = source_cards
            .iter()
            .map(|card| card.retrieved_at.as_str())
            .max();
        self.record_source_success(SourceHealthUpdate {
            key: &format!("knowledge:model-clusters:{query}"),
            provider: "arcwell",
            source_kind: "knowledge_model_clusters",
            locator: &query,
            last_item_id,
            last_item_date,
            cursor_key: None,
            cursor_value: None,
            next_run_at: Some(&now_plus_seconds(self.watch_source_next_run_seconds(
                "knowledge_model_clusters",
                &query,
                6 * 60 * 60,
            ))),
        })?;
        Ok(json!({
            "status": "completed",
            "query": query,
            "model_provider": invocation.model_provider,
            "model_name": invocation.model_name,
            "prompt_version": invocation.prompt_version,
            "proof_level": invocation.proof_level,
            "cost_decision_id": invocation.cost_decision_id,
            "source_card_count": source_card_ids.len(),
            "source_cards": source_card_ids,
            "broad_source_card_sweep": broad_source_card_sweep,
            "skipped_clustered_source_cards": skipped_clustered,
            "skipped_generated_only_source_cards": skipped_generated_only,
            "clusters_created": cluster_ids.len(),
            "cluster_ids": cluster_ids,
            "boundary": "Scheduled model clustering writes review-only candidate clusters; promotion is required before wiki/report/digest expansion."
        }))
    }

    pub(crate) fn execute_knowledge_cluster_investigate(&self, input: &Value) -> Result<Value> {
        let cluster_id = input
            .get("cluster_id")
            .and_then(Value::as_str)
            .context("knowledge_cluster_investigate missing cluster_id")?;
        let report = self.create_knowledge_cluster_investigation(cluster_id)?;
        Ok(json!({
            "cluster_id": report.cluster.id,
            "research_run_id": report.research_run.id,
            "task_count": report.tasks.len(),
            "source_link_count": report.source_links.len(),
            "editorial_decision_id": report.editorial_decision.id,
            "reused_existing": report.reused_existing,
            "status": "completed"
        }))
    }

    pub(crate) fn execute_knowledge_cluster_investigation_execute(
        &self,
        input: &Value,
    ) -> Result<Value> {
        let cluster_id = input
            .get("cluster_id")
            .and_then(Value::as_str)
            .context("knowledge_cluster_investigation_execute missing cluster_id")?;
        let report = self.execute_knowledge_cluster_investigation(cluster_id)?;
        Ok(json!({
            "cluster_id": report.cluster.id,
            "research_run_id": report.research_run.id,
            "research_run_status": report.research_run.status,
            "task_count": report.tasks.len(),
            "executed_task_count": report.executed_task_count,
            "already_completed_task_count": report.already_completed_task_count,
            "role_run_count": report.role_runs.len(),
            "artifact_count": report.artifacts.len(),
            "editorial_decision_id": report.editorial_decision.id,
            "quality_findings": report.quality_findings,
            "status": report.editorial_decision.status
        }))
    }

    pub(crate) fn execute_research_convergence_run(&self, input: &Value) -> Result<Value> {
        let input: ResearchConvergenceStepInput = serde_json::from_value(input.clone())
            .context("research_convergence_run invalid input")?;
        let config = normalize_research_convergence_config(&input)?;
        let run = self.require_research_run(&input.run_id)?;
        let existing_status = self.research_convergence_status(&input.run_id)?;
        if matches!(
            run.status.as_str(),
            "stopped" | "completed" | "completed_no_write"
        ) {
            return Ok(json!({
                "run_id": input.run_id,
                "action": "skipped",
                "reason": format!("research run is {}", run.status),
                "config": config,
                "status": existing_status,
                "step": null,
                "report": null
            }));
        }
        if existing_status.settled
            || existing_status
                .stop_reason
                .as_deref()
                .is_some_and(|reason| reason != "continue")
        {
            let report = if input.no_write.unwrap_or(false) {
                None
            } else if config.editorial_provider.is_some()
                && !self.convergence_accepted_editorial_judgment_recorded(&input.run_id)?
            {
                Some(
                    self.run_research_convergence_editorial_loop(&input.run_id, &config)?
                        .report,
                )
            } else {
                Some(self.compile_research_convergence_report(&input.run_id)?)
            };
            return Ok(json!({
                "run_id": input.run_id,
                "action": "already_terminal",
                "config": config,
                "status": existing_status,
                "step": null,
                "report": report
            }));
        }
        let step = self.run_research_convergence_to_stop(input.clone())?;
        let terminal = step.status.settled
            || step
                .status
                .stop_reason
                .as_deref()
                .is_some_and(|reason| reason != "continue");
        let report = if terminal && !input.no_write.unwrap_or(false) {
            match step.report.clone() {
                Some(report) => Some(report),
                None => Some(self.compile_research_convergence_report(&input.run_id)?),
            }
        } else {
            None
        };
        Ok(json!({
            "run_id": input.run_id,
            "action": if terminal { "terminal" } else { "advanced" },
            "config": config,
            "status": step.status,
            "step": step,
            "report": report
        }))
    }
}
