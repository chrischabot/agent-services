use super::*;

impl Store {
    pub fn start_research_convergence(
        &self,
        input: ResearchConvergenceStartInput,
    ) -> Result<ResearchConvergenceStep> {
        self.run_research_convergence_step(ResearchConvergenceStepInput {
            run_id: input.run_id,
            max_iterations: input.max_iterations,
            max_seconds: input.max_seconds,
            max_sources: input.max_sources,
            max_provider_calls: input.max_provider_calls,
            cost_cap_usd: input.cost_cap_usd,
            source_novelty_threshold: input.source_novelty_threshold,
            confidence_delta_threshold: input.confidence_delta_threshold,
            no_progress_iteration_limit: input.no_progress_iteration_limit,
            require_active_fact_check: input.require_active_fact_check,
            allow_long_run: input.allow_long_run,
            no_write: input.no_write,
            editorial_provider: input.editorial_provider,
            editorial_model_name: input.editorial_model_name,
            editorial_endpoint: input.editorial_endpoint,
            editorial_timeout_seconds: input.editorial_timeout_seconds,
        })
    }

    pub fn run_research_convergence_to_stop(
        &self,
        input: ResearchConvergenceStepInput,
    ) -> Result<ResearchConvergenceStep> {
        let config = normalize_research_convergence_config(&input)?;
        let run = self.require_research_run(&input.run_id)?;
        let existing_status = self.research_convergence_status(&input.run_id)?;
        if existing_status.settled
            || existing_status
                .stop_reason
                .as_deref()
                .is_some_and(|reason| reason != "continue")
        {
            let iteration = existing_status
                .latest_iteration
                .clone()
                .context("terminal convergence is missing its latest iteration")?;
            let snapshot = existing_status
                .latest_snapshot
                .clone()
                .context("terminal convergence is missing its latest snapshot")?;
            let editorial = if config.editorial_provider.is_some()
                && !self.convergence_accepted_editorial_judgment_recorded(&input.run_id)?
            {
                Some(self.run_research_convergence_editorial_loop(&input.run_id, &config)?)
            } else {
                None
            };
            let report = editorial.as_ref().map(|editorial| editorial.report.clone());
            return Ok(ResearchConvergenceStep {
                run,
                iteration,
                statements: existing_status.current_statements.clone(),
                challenges: existing_status.open_challenges.clone(),
                disproofs: existing_status.strong_refutations.clone(),
                revisions: self.list_research_revisions(&input.run_id)?,
                fact_checks: self.list_research_fact_checks(&input.run_id)?,
                snapshot,
                status: existing_status,
                report,
                editorial,
            });
        }
        let mut last = self.run_research_convergence_step(input.clone())?;
        let mut guard = 1usize;
        while !last.status.settled
            && last
                .status
                .stop_reason
                .as_deref()
                .is_none_or(|reason| reason == "continue")
            && guard < config.max_iterations
        {
            last = self.run_research_convergence_step(input.clone())?;
            guard += 1;
        }
        let exhausted_iteration_budget = !last.status.settled
            && last
                .status
                .stop_reason
                .as_deref()
                .is_none_or(|reason| reason == "continue")
            && guard >= config.max_iterations;
        if exhausted_iteration_budget {
            last.status.stop_reason = Some("max_iterations".to_string());
        }
        if config.editorial_provider.is_some()
            && (last.status.settled
                || last
                    .status
                    .stop_reason
                    .as_deref()
                    .is_some_and(|reason| reason != "continue")
                || exhausted_iteration_budget)
            && !self.convergence_accepted_editorial_judgment_recorded(&last.status.run_id)?
        {
            let editorial =
                self.run_research_convergence_editorial_loop(&last.status.run_id, &config)?;
            last.report = Some(editorial.report.clone());
            last.editorial = Some(editorial);
        }
        Ok(last)
    }

    pub fn run_research_convergence_step(
        &self,
        input: ResearchConvergenceStepInput,
    ) -> Result<ResearchConvergenceStep> {
        let config = normalize_research_convergence_config(&input)?;
        let run = self.require_research_run(&input.run_id)?;
        if matches!(
            run.status.as_str(),
            "stopped" | "completed" | "completed_no_write"
        ) {
            bail!("research run is not open for convergence: {}", run.status);
        }
        let previous = self.latest_research_iteration(&run.id)?;
        if self
            .latest_research_convergence_snapshot(&run.id)?
            .is_some()
        {
            let current_status = self.research_convergence_status(&run.id)?;
            if current_status.settled {
                bail!("research convergence is already settled for run {}", run.id);
            }
            if current_status
                .stop_reason
                .as_deref()
                .is_some_and(|reason| reason != "continue")
            {
                bail!("research convergence is already stopped for run {}", run.id);
            }
        }
        let next_index = previous
            .as_ref()
            .map(|iteration| iteration.iteration_index + 1)
            .unwrap_or(1);
        if next_index > config.max_iterations {
            let status = self.research_convergence_status(&run.id)?;
            return Ok(ResearchConvergenceStep {
                run,
                iteration: previous
                    .context("missing previous iteration after max-iteration stop")?,
                statements: status.current_statements.clone(),
                challenges: status.open_challenges.clone(),
                disproofs: status.strong_refutations.clone(),
                revisions: self.list_research_revisions(&input.run_id)?,
                fact_checks: self.list_research_fact_checks(&input.run_id)?,
                snapshot: status
                    .latest_snapshot
                    .clone()
                    .context("missing latest convergence snapshot")?,
                status,
                report: None,
                editorial: None,
            });
        }

        let started_at = now();
        let iteration = self.insert_research_iteration(
            &run.id,
            next_index,
            previous.as_ref().map(|iteration| iteration.id.as_str()),
            "running",
            &format!("Convergence iteration {next_index} for `{}`", run.query),
            &started_at,
        )?;
        let config_artifact = self.record_research_artifact(ResearchArtifactInput {
            run_id: run.id.clone(),
            role_run_id: None,
            artifact_type: "convergence_config".to_string(),
            title: format!("Convergence config iteration {next_index}"),
            body: serde_json::to_string_pretty(&config)?,
            metadata: json!({
                "iteration_id": iteration.id,
                "artifact_role": "convergence_control",
            }),
        })?;

        let mut statements = self.compile_research_statements_for_iteration(
            &run.id,
            &iteration.id,
            previous.as_ref().map(|iteration| iteration.id.as_str()),
        )?;
        let statement_artifact = self.record_research_artifact(ResearchArtifactInput {
            run_id: run.id.clone(),
            role_run_id: None,
            artifact_type: "statement_set".to_string(),
            title: format!("Statement set iteration {next_index}"),
            body: serde_json::to_string_pretty(&statements)?,
            metadata: json!({
                "iteration_id": iteration.id,
                "artifact_role": "statement_set",
                "source": "deterministic_statement_compiler",
            }),
        })?;

        let challenges =
            self.generate_research_challenges_for_iteration(&run.id, &iteration.id, &statements)?;
        let challenges =
            self.apply_research_host_search_proofs_to_challenges(&run.id, challenges)?;
        let challenge_artifact = self.record_research_artifact(ResearchArtifactInput {
            run_id: run.id.clone(),
            role_run_id: None,
            artifact_type: "challenge_pack".to_string(),
            title: format!("Challenge pack iteration {next_index}"),
            body: serde_json::to_string_pretty(&challenges)?,
            metadata: json!({
                "iteration_id": iteration.id,
                "artifact_role": "challenge_pack",
                "source": "deterministic_red_team",
            }),
        })?;

        let disproofs =
            self.generate_research_disproofs_for_iteration(&run.id, &iteration.id, &challenges)?;
        let disproof_artifact = self.record_research_artifact(ResearchArtifactInput {
            run_id: run.id.clone(),
            role_run_id: None,
            artifact_type: "disproof_pack".to_string(),
            title: format!("Disproof pack iteration {next_index}"),
            body: serde_json::to_string_pretty(&disproofs)?,
            metadata: json!({
                "iteration_id": iteration.id,
                "artifact_role": "disproof_pack",
                "source": "deterministic_verifier",
            }),
        })?;

        let revisions = self.apply_research_revisions_for_iteration(
            &run.id,
            &iteration.id,
            &disproofs,
            &mut statements,
        )?;
        let revision_artifact = self.record_research_artifact(ResearchArtifactInput {
            run_id: run.id.clone(),
            role_run_id: None,
            artifact_type: "revision_set".to_string(),
            title: format!("Revision set iteration {next_index}"),
            body: serde_json::to_string_pretty(&revisions)?,
            metadata: json!({
                "iteration_id": iteration.id,
                "artifact_role": "revision_set",
                "source": "deterministic_reviser",
            }),
        })?;

        let fact_checks =
            self.run_research_fact_checks_for_iteration(&run.id, &iteration.id, &statements)?;
        let snapshot = self.create_research_convergence_snapshot(
            &run,
            &iteration,
            previous.as_ref(),
            &statements,
            &challenges,
            &disproofs,
            &revisions,
            &fact_checks,
            &config,
            &started_at,
        )?;
        let final_iteration = self.finish_research_iteration(
            &iteration.id,
            &config_artifact.id,
            &statement_artifact.id,
            &challenge_artifact.id,
            &disproof_artifact.id,
            &revision_artifact.id,
            &snapshot.id,
            if snapshot.settled {
                "settled"
            } else if snapshot
                .stop_rule
                .get("stop_reason")
                .and_then(Value::as_str)
                .is_some_and(|reason| reason != "continue")
            {
                "stopped"
            } else {
                "completed"
            },
            snapshot
                .stop_rule
                .get("stop_reason")
                .and_then(Value::as_str),
        )?;
        let stop_reason = snapshot
            .stop_rule
            .get("stop_reason")
            .and_then(Value::as_str)
            .unwrap_or("continue");
        if snapshot.settled {
            self.update_research_run_status(&run.id, "converged_settled")?;
        } else if stop_reason != "continue" {
            self.update_research_run_status(&run.id, "converged_incomplete")?;
        } else {
            self.update_research_run_status(&run.id, "converging")?;
        }
        let refreshed_run = self.require_research_run(&run.id)?;
        let status = self.research_convergence_status(&run.id)?;
        Ok(ResearchConvergenceStep {
            run: refreshed_run,
            iteration: final_iteration,
            statements,
            challenges,
            disproofs,
            revisions,
            fact_checks,
            snapshot,
            status,
            report: None,
            editorial: None,
        })
    }

    pub fn research_convergence_status(&self, run_id: &str) -> Result<ResearchConvergenceStatus> {
        self.require_research_run(run_id)?;
        let latest_iteration = self.latest_research_iteration(run_id)?;
        let latest_snapshot = self.latest_research_convergence_snapshot(run_id)?;
        let current_statements = if let Some(iteration) = &latest_iteration {
            self.list_research_statements_for_iteration(&iteration.id)?
        } else {
            Vec::new()
        };
        let current_statement_ids = current_statements
            .iter()
            .map(|statement| statement.id.clone())
            .collect::<BTreeSet<_>>();
        let challenges = self.list_research_challenges(run_id)?;
        let answered_challenge_ids = challenges
            .iter()
            .filter(|challenge| challenge.status == "answered")
            .map(|challenge| challenge.id.clone())
            .collect::<BTreeSet<_>>();
        let open_challenges = challenges
            .iter()
            .filter(|challenge| {
                matches!(
                    challenge.status.as_str(),
                    "open" | "searching" | "unresolved"
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        let host_search_tasks = self.list_research_convergence_host_search_tasks(run_id)?;
        let strong_refutations = self
            .list_research_disproofs(run_id)?
            .into_iter()
            .filter(|disproof| {
                !answered_challenge_ids.contains(&disproof.challenge_id)
                    && disproof.requires_revision
                    && matches!(disproof.strength.as_str(), "strong" | "moderate")
                    && matches!(disproof.verdict.as_str(), "refutes" | "weakens" | "unknown")
            })
            .collect::<Vec<_>>();
        let unresolved_high_fact_checks = self
            .list_research_fact_checks(run_id)?
            .into_iter()
            .filter(|check| {
                current_statement_ids.contains(&check.statement_id)
                    && check.impact == "high"
                    && matches!(check.label.as_str(), "wrong" | "unknown")
            })
            .collect::<Vec<_>>();
        let snapshot_stop_reason = latest_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.stop_rule.get("stop_reason"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let has_open_blocking_challenges = open_challenges
            .iter()
            .any(|challenge| matches!(challenge.severity.as_str(), "critical" | "error"));
        let settled = latest_snapshot
            .as_ref()
            .map(|snapshot| snapshot.settled)
            .unwrap_or(false)
            && !has_open_blocking_challenges
            && unresolved_high_fact_checks.is_empty()
            && strong_refutations.is_empty();
        let snapshot_is_incomplete_terminal = snapshot_stop_reason
            .as_deref()
            .is_some_and(|reason| !matches!(reason, "continue" | "settled"));
        let stop_reason = if snapshot_is_incomplete_terminal {
            snapshot_stop_reason
        } else if !settled
            && (!unresolved_high_fact_checks.is_empty() || has_open_blocking_challenges)
        {
            Some("continue".to_string())
        } else {
            snapshot_stop_reason
        };
        Ok(ResearchConvergenceStatus {
            run_id: run_id.to_string(),
            latest_iteration,
            latest_snapshot,
            current_statements,
            open_challenges,
            host_search_tasks,
            strong_refutations,
            stop_reason,
            settled,
        })
    }

    pub fn compile_research_convergence_report(
        &self,
        run_id: &str,
    ) -> Result<ResearchConvergenceReport> {
        let run = self.require_research_run(run_id)?;
        let iterations = self.list_research_iterations(run_id)?;
        let statements = self.list_research_statements(run_id)?;
        let challenges = self.list_research_challenges(run_id)?;
        let disproofs = self.list_research_disproofs(run_id)?;
        let revisions = self.list_research_revisions(run_id)?;
        let fact_checks = self.list_research_fact_checks(run_id)?;
        let snapshots = self.list_research_convergence_snapshots(run_id)?;
        let claims = self.list_research_claims(run_id)?;
        let sources = self.list_research_run_sources(run_id)?;
        let status = self.research_convergence_status(run_id)?;
        let markdown = render_research_convergence_report(
            &run,
            &iterations,
            &statements,
            &challenges,
            &disproofs,
            &revisions,
            &fact_checks,
            &snapshots,
            &status,
        );
        let artifact = self.record_research_artifact(ResearchArtifactInput {
            run_id: run_id.to_string(),
            role_run_id: None,
            artifact_type: "convergence_report".to_string(),
            title: format!("Convergence Report: {}", run.query),
            body: markdown,
            metadata: json!({
                "artifact_role": "final_convergence_report",
                "settled": status.settled,
                "stop_reason": status.stop_reason,
            }),
        })?;
        let judgment = self.record_research_report_judgment(
            run_id,
            None,
            build_research_report_judgment(
                run_id,
                None,
                &status,
                &statements,
                &challenges,
                &disproofs,
                &fact_checks,
                &claims,
                &sources,
            )?,
        )?;
        Ok(ResearchConvergenceReport { artifact, judgment })
    }

    pub(crate) fn run_research_convergence_editorial_loop(
        &self,
        run_id: &str,
        config: &ResearchConvergenceConfig,
    ) -> Result<ResearchConvergenceEditorialLoop> {
        let provider = config
            .editorial_provider
            .as_deref()
            .context("convergence editorial provider is not configured")?;
        let report = self.compile_research_convergence_report(run_id)?;
        let citation_verifier = self.invoke_research_editorial(ResearchEditorialInvokeInput {
            run_id: run_id.to_string(),
            stage: "citation_verifier".to_string(),
            model_provider: provider.to_string(),
            model_name: config.editorial_model_name.clone(),
            prompt_version: "convergence-citation-verifier-v1".to_string(),
            input_artifact_id: Some(report.artifact.id.clone()),
            endpoint: config.editorial_endpoint.clone(),
            api_key: None,
            timeout_seconds: config.editorial_timeout_seconds,
        })?;
        let mut blocking_findings = Vec::new();
        if !citation_verifier_passed(&citation_verifier) {
            blocking_findings.push("citation_verifier_failed_or_rejected".to_string());
            let report = self.record_convergence_editorial_judgment(
                report,
                Some(&citation_verifier),
                None,
                &blocking_findings,
            )?;
            return Ok(ResearchConvergenceEditorialLoop {
                report,
                citation_verifier: Some(citation_verifier),
                adversarial_evaluator: None,
                status: "rejected".to_string(),
                blocking_findings,
            });
        }
        let verifier_output_id = citation_verifier
            .output_artifact
            .as_ref()
            .map(|artifact| {
                if artifact
                    .metadata
                    .get("score_body_synthesized")
                    .and_then(Value::as_bool)
                    == Some(true)
                {
                    report.artifact.id.clone()
                } else {
                    artifact.id.clone()
                }
            })
            .context("citation verifier passed without an output artifact")?;
        let adversarial_evaluator =
            self.invoke_research_editorial(ResearchEditorialInvokeInput {
                run_id: run_id.to_string(),
                stage: "adversarial_evaluator".to_string(),
                model_provider: provider.to_string(),
                model_name: config.editorial_model_name.clone(),
                prompt_version: "convergence-adversarial-evaluator-v1".to_string(),
                input_artifact_id: Some(verifier_output_id),
                endpoint: config.editorial_endpoint.clone(),
                api_key: None,
                timeout_seconds: config.editorial_timeout_seconds,
            })?;
        if !adversarial_evaluator_passed(&adversarial_evaluator) {
            blocking_findings.push("adversarial_evaluator_failed_or_rejected".to_string());
        }
        let accepted = blocking_findings.is_empty();
        let report = self.record_convergence_editorial_judgment(
            report,
            Some(&citation_verifier),
            Some(&adversarial_evaluator),
            &blocking_findings,
        )?;
        Ok(ResearchConvergenceEditorialLoop {
            report,
            citation_verifier: Some(citation_verifier),
            adversarial_evaluator: Some(adversarial_evaluator),
            status: if accepted { "accepted" } else { "rejected" }.to_string(),
            blocking_findings,
        })
    }

    pub(crate) fn convergence_accepted_editorial_judgment_recorded(
        &self,
        run_id: &str,
    ) -> Result<bool> {
        Ok(self
            .list_research_report_judgments(run_id)?
            .into_iter()
            .any(|judgment| {
                judgment
                    .scores
                    .get("model_backed_convergence_editorial")
                    .and_then(|score| score.get("accepted"))
                    .and_then(Value::as_bool)
                    == Some(true)
            }))
    }

    pub(crate) fn record_convergence_editorial_judgment(
        &self,
        report: ResearchConvergenceReport,
        citation_verifier: Option<&ResearchEditorialInvocation>,
        adversarial_evaluator: Option<&ResearchEditorialInvocation>,
        editorial_blockers: &[String],
    ) -> Result<ResearchConvergenceReport> {
        let run_id = report.artifact.run_id.clone();
        let status = self.research_convergence_status(&run_id)?;
        let statements = self.list_research_statements(&run_id)?;
        let challenges = self.list_research_challenges(&run_id)?;
        let disproofs = self.list_research_disproofs(&run_id)?;
        let fact_checks = self.list_research_fact_checks(&run_id)?;
        let claims = self.list_research_claims(&run_id)?;
        let sources = self.list_research_run_sources(&run_id)?;
        let mut judgment = build_research_report_judgment(
            &run_id,
            None,
            &status,
            &statements,
            &challenges,
            &disproofs,
            &fact_checks,
            &claims,
            &sources,
        )?;
        judgment.id = research_report_judgment_id(
            &run_id,
            Some(&format!("model-backed-convergence:{}", report.artifact.id)),
        );
        judgment.scores = merge_json_objects(
            judgment.scores,
            json!({
                "model_backed_convergence_editorial": {
                    "citation_verifier": citation_verifier.map(|invocation| json!({
                        "run_id": invocation.editorial_run.id,
                        "status": invocation.editorial_run.status,
                        "score": invocation.editorial_run.score,
                        "output_artifact_id": invocation.editorial_run.output_artifact_id
                    })),
                    "citation_verifier_status": citation_verifier.map(|invocation| invocation.editorial_run.status.clone()),
                    "citation_verifier_score": citation_verifier.map(|invocation| invocation.editorial_run.score.clone()),
                    "adversarial_evaluator": adversarial_evaluator.map(|invocation| json!({
                        "run_id": invocation.editorial_run.id,
                        "status": invocation.editorial_run.status,
                        "score": invocation.editorial_run.score,
                        "output_artifact_id": invocation.editorial_run.output_artifact_id
                    })),
                    "adversarial_evaluator_status": adversarial_evaluator.map(|invocation| invocation.editorial_run.status.clone()),
                    "adversarial_evaluator_score": adversarial_evaluator.map(|invocation| invocation.editorial_run.score.clone()),
                    "accepted": editorial_blockers.is_empty()
                }
            }),
        );
        judgment.commands_or_artifacts_reviewed = merge_json_objects(
            judgment.commands_or_artifacts_reviewed,
            json!({
                "convergence_report_artifact_id": report.artifact.id,
                "citation_verifier_run_id": citation_verifier.map(|invocation| invocation.editorial_run.id.clone()),
                "citation_verifier_output_artifact_id": citation_verifier.and_then(|invocation| invocation.editorial_run.output_artifact_id.clone()),
                "adversarial_evaluator_run_id": adversarial_evaluator.map(|invocation| invocation.editorial_run.id.clone()),
                "adversarial_evaluator_output_artifact_id": adversarial_evaluator.and_then(|invocation| invocation.editorial_run.output_artifact_id.clone())
            }),
        );
        if !editorial_blockers.is_empty() {
            let mut blocking = judgment
                .blocking_findings
                .as_array()
                .cloned()
                .unwrap_or_default();
            for blocker in editorial_blockers {
                blocking.push(json!({
                    "code": blocker,
                    "message": "Model-backed convergence editorial/evaluator gate rejected the report."
                }));
            }
            judgment.blocking_findings = Value::Array(blocking);
            judgment.overall_decision = "reject".to_string();
        } else if judgment.overall_decision == "accept" {
            judgment.overall_decision = "accept".to_string();
        }
        let judgment = self.record_research_report_judgment(&run_id, None, judgment)?;
        Ok(ResearchConvergenceReport {
            artifact: report.artifact,
            judgment,
        })
    }

    pub fn list_research_iterations(&self, run_id: &str) -> Result<Vec<ResearchIteration>> {
        self.require_research_run(run_id)?;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, run_id, iteration_index, parent_iteration_id, status, objective,
                   position_artifact_id, statement_set_artifact_id, challenge_pack_artifact_id,
                   disproof_pack_artifact_id, revision_artifact_id, convergence_snapshot_id,
                   cost_decision_id, started_at, completed_at, stop_reason, error_message_redacted
            FROM research_iterations
            WHERE run_id = ?1
            ORDER BY iteration_index ASC
            "#,
        )?;
        rows(stmt.query_map(params![run_id], research_iteration_from_row)?)
    }

    pub fn read_research_iteration(&self, id: &str) -> Result<Option<ResearchIteration>> {
        validate_id(id)?;
        self.conn
            .query_row(
                r#"
                SELECT id, run_id, iteration_index, parent_iteration_id, status, objective,
                       position_artifact_id, statement_set_artifact_id, challenge_pack_artifact_id,
                       disproof_pack_artifact_id, revision_artifact_id, convergence_snapshot_id,
                       cost_decision_id, started_at, completed_at, stop_reason, error_message_redacted
                FROM research_iterations
                WHERE id = ?1
                "#,
                params![id],
                research_iteration_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_research_statements(&self, run_id: &str) -> Result<Vec<ResearchStatement>> {
        self.require_research_run(run_id)?;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, run_id, iteration_id, parent_statement_id, stable_key, statement_type, text,
                   scope, temporal_scope, confidence, certainty_label, status, importance,
                   evidence_json, counterevidence_json, assumptions_json, caveats_json,
                   created_by_role, created_at, updated_at
            FROM research_statements
            WHERE run_id = ?1
            ORDER BY created_at ASC
            "#,
        )?;
        rows(stmt.query_map(params![run_id], research_statement_from_row)?)
    }

    pub fn list_research_challenges(&self, run_id: &str) -> Result<Vec<ResearchChallenge>> {
        self.require_research_run(run_id)?;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, run_id, iteration_id, statement_id, challenge_type, severity, rationale,
                   would_change_answer_if_true, search_plan_json, required_source_families_json,
                   status, created_by_role, created_at, updated_at
            FROM research_challenges
            WHERE run_id = ?1
            ORDER BY created_at ASC
            "#,
        )?;
        rows(stmt.query_map(params![run_id], research_challenge_from_row)?)
    }

    pub fn list_research_convergence_host_search_tasks(
        &self,
        run_id: &str,
    ) -> Result<Vec<ResearchConvergenceHostSearchTask>> {
        self.require_research_run(run_id)?;
        let challenges = self.list_research_challenges(run_id)?;
        let host_searches = self.list_research_host_searches(run_id)?;
        let mut tasks_by_query: BTreeMap<String, ResearchConvergenceHostSearchTask> =
            BTreeMap::new();
        for challenge in challenges {
            if challenge
                .search_plan
                .get("requires_host_search_proof")
                .and_then(Value::as_bool)
                != Some(true)
            {
                continue;
            }
            for query in research_challenge_planned_queries(&challenge) {
                let normalized_query = normalized_research_search_query(&query);
                let proof =
                    host_search_proof_for_challenge_query(&challenge, &query, &host_searches);
                let (
                    status,
                    matched_host_search_ids,
                    matched_result_ids,
                    research_source_ids,
                    source_card_ids,
                    selected_result_count,
                ) = if let Some(proof) = proof {
                    (
                        "recorded".to_string(),
                        proof.matched_search_ids,
                        proof.matched_result_ids,
                        proof.research_source_ids,
                        proof.source_card_ids,
                        proof.selected_result_count,
                    )
                } else {
                    (
                        "pending".to_string(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        0,
                    )
                };
                let task = ResearchConvergenceHostSearchTask {
                    id: research_convergence_host_search_task_id(&challenge.id, &query),
                    run_id: challenge.run_id.clone(),
                    iteration_id: challenge.iteration_id.clone(),
                    challenge_id: challenge.id.clone(),
                    statement_id: challenge.statement_id.clone(),
                    challenge_type: challenge.challenge_type.clone(),
                    severity: challenge.severity.clone(),
                    query: query.clone(),
                    normalized_query,
                    required_source_families: challenge.required_source_families.clone(),
                    status,
                    matched_host_search_ids,
                    matched_result_ids,
                    research_source_ids,
                    source_card_ids,
                    selected_result_count,
                    instructions: "Run this exact query with host-native search, then call research_host_search_record with structured result objects and selected linked sources before rerunning convergence.".to_string(),
                };
                tasks_by_query
                    .entry(task.normalized_query.clone())
                    .and_modify(|existing| {
                        Self::merge_research_convergence_host_search_task(existing, &task)
                    })
                    .or_insert(task);
            }
        }
        let mut tasks = tasks_by_query.into_values().collect::<Vec<_>>();
        tasks.sort_by(|left, right| {
            left.status
                .cmp(&right.status)
                .then_with(|| left.severity.cmp(&right.severity))
                .then_with(|| left.challenge_id.cmp(&right.challenge_id))
                .then_with(|| left.normalized_query.cmp(&right.normalized_query))
        });
        Ok(tasks)
    }

    pub(crate) fn merge_research_convergence_host_search_task(
        existing: &mut ResearchConvergenceHostSearchTask,
        candidate: &ResearchConvergenceHostSearchTask,
    ) {
        if Self::research_challenge_severity_rank(&candidate.severity)
            > Self::research_challenge_severity_rank(&existing.severity)
        {
            existing.id = candidate.id.clone();
            existing.iteration_id = candidate.iteration_id.clone();
            existing.challenge_id = candidate.challenge_id.clone();
            existing.statement_id = candidate.statement_id.clone();
            existing.challenge_type = candidate.challenge_type.clone();
            existing.severity = candidate.severity.clone();
            existing.query = candidate.query.clone();
            existing.required_source_families = candidate.required_source_families.clone();
        }
        if existing.status != "pending" && candidate.status == "pending" {
            existing.status = candidate.status.clone();
        }
        Self::merge_unique_strings(
            &mut existing.matched_host_search_ids,
            &candidate.matched_host_search_ids,
        );
        Self::merge_unique_strings(
            &mut existing.matched_result_ids,
            &candidate.matched_result_ids,
        );
        Self::merge_unique_strings(
            &mut existing.research_source_ids,
            &candidate.research_source_ids,
        );
        Self::merge_unique_strings(&mut existing.source_card_ids, &candidate.source_card_ids);
        existing.selected_result_count = existing
            .selected_result_count
            .max(candidate.selected_result_count);
    }

    pub(crate) fn merge_unique_strings(target: &mut Vec<String>, source: &[String]) {
        let mut seen = target.iter().cloned().collect::<BTreeSet<_>>();
        for value in source {
            if seen.insert(value.clone()) {
                target.push(value.clone());
            }
        }
    }

    pub(crate) fn research_challenge_severity_rank(severity: &str) -> usize {
        match severity {
            "critical" => 4,
            "error" => 3,
            "warning" => 2,
            "info" => 1,
            _ => 0,
        }
    }
}
