use super::*;

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_research_convergence_snapshot(
        &self,
        run: &ResearchRun,
        iteration: &ResearchIteration,
        previous_iteration: Option<&ResearchIteration>,
        statements: &[ResearchStatement],
        challenges: &[ResearchChallenge],
        disproofs: &[ResearchDisproof],
        revisions: &[ResearchRevision],
        fact_checks: &[ResearchFactCheck],
        config: &ResearchConvergenceConfig,
        started_at: &str,
    ) -> Result<ResearchConvergenceSnapshot> {
        let sources = self.list_research_run_sources(&run.id)?;
        let claims = self.list_research_claims(&run.id)?;
        let previous_snapshot = self.latest_research_convergence_snapshot(&run.id)?;
        let previous_source_count = previous_snapshot
            .as_ref()
            .map(|snapshot| snapshot.source_count_total)
            .unwrap_or(0);
        let previous_claim_count = previous_snapshot
            .as_ref()
            .map(|snapshot| snapshot.claim_count_total)
            .unwrap_or(0);
        let source_count_total = sources.len();
        let claim_count_total = claims.len();
        let source_count_new = source_count_total.saturating_sub(previous_source_count);
        let claim_count_new = claim_count_total.saturating_sub(previous_claim_count);
        let primary_source_count_new = sources
            .iter()
            .filter_map(|record| record.source_card.as_ref())
            .filter(|card| infer_source_role_from_card(card) == "primary")
            .count();
        let critical_open_challenges = challenges
            .iter()
            .filter(|challenge| challenge.severity == "critical" && challenge.status != "answered")
            .count();
        let high_open_challenges = challenges
            .iter()
            .filter(|challenge| challenge.severity == "error" && challenge.status != "answered")
            .count();
        let strong_refutations = disproofs
            .iter()
            .filter(|disproof| {
                disproof.requires_revision
                    && matches!(disproof.strength.as_str(), "strong" | "moderate")
            })
            .count();
        let unknown_high_impact_claims = fact_checks
            .iter()
            .filter(|check| {
                check.impact == "high" && matches!(check.label.as_str(), "unknown" | "wrong")
            })
            .count();
        let deltas = disproofs
            .iter()
            .map(|disproof| disproof.confidence_delta.abs())
            .collect::<Vec<_>>();
        let mean_confidence_delta = if deltas.is_empty() {
            0.0
        } else {
            deltas.iter().sum::<f64>() / deltas.len() as f64
        };
        let max_confidence_delta = deltas.into_iter().fold(0.0_f64, f64::max);
        let source_novelty_score = if source_count_total == 0 {
            0.0
        } else {
            source_count_new as f64 / source_count_total as f64
        };
        let claim_novelty_score = if claim_count_total == 0 {
            0.0
        } else {
            claim_count_new as f64 / claim_count_total as f64
        };
        let position_edit_distance = if statements.is_empty() {
            0.0
        } else {
            revisions.len() as f64 / statements.len() as f64
        };
        let right_count = fact_checks
            .iter()
            .filter(|check| check.label == "right")
            .count();
        let citation_support_score = if fact_checks.is_empty() {
            0.0
        } else {
            right_count as f64 / fact_checks.len() as f64
        };
        let active_fact_check_score = citation_support_score;
        let no_progress_count = previous_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.stop_rule.get("no_progress_count"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;
        let current_no_progress = if previous_iteration.is_some()
            && source_novelty_score <= config.source_novelty_threshold
            && claim_novelty_score <= config.source_novelty_threshold
            && mean_confidence_delta <= config.confidence_delta_threshold
            && position_edit_distance <= config.confidence_delta_threshold
        {
            no_progress_count + 1
        } else {
            0
        };
        let elapsed_seconds = DateTime::parse_from_rfc3339(started_at)
            .ok()
            .map(|started| (Utc::now() - started.with_timezone(&Utc)).num_seconds())
            .unwrap_or(0)
            .max(0);
        let stop_reason = convergence_stop_reason(
            iteration.iteration_index,
            statements,
            critical_open_challenges,
            high_open_challenges,
            strong_refutations,
            unknown_high_impact_claims,
            source_count_total,
            current_no_progress,
            source_novelty_score,
            mean_confidence_delta,
            elapsed_seconds,
            config,
        );
        let settled = stop_reason == "settled";
        let snapshot = ResearchConvergenceSnapshot {
            id: research_convergence_snapshot_id(&run.id, &iteration.id),
            run_id: run.id.clone(),
            iteration_id: iteration.id.clone(),
            source_count_total,
            source_count_new,
            primary_source_count_new,
            claim_count_total,
            statement_count_current: statements.len(),
            statement_count_changed: revisions.len(),
            critical_open_challenges,
            high_open_challenges,
            strong_refutations,
            unknown_high_impact_claims,
            mean_confidence_delta,
            max_confidence_delta,
            source_novelty_score,
            claim_novelty_score,
            position_edit_distance,
            citation_support_score,
            active_fact_check_score,
            evaluator_score: if critical_open_challenges == 0 && high_open_challenges == 0 {
                1.0
            } else {
                0.5
            },
            cost_usd_estimated: 0.0,
            elapsed_seconds,
            stop_rule: json!({
                "stop_reason": stop_reason,
                "no_progress_count": current_no_progress,
                "max_iterations": config.max_iterations,
                "max_seconds": config.max_seconds,
                "source_novelty_threshold": config.source_novelty_threshold,
                "confidence_delta_threshold": config.confidence_delta_threshold,
            }),
            settled,
            created_at: now(),
        };
        self.insert_research_convergence_snapshot(snapshot)
    }

    pub(crate) fn insert_research_convergence_snapshot(
        &self,
        snapshot: ResearchConvergenceSnapshot,
    ) -> Result<ResearchConvergenceSnapshot> {
        self.conn.execute(
            r#"
            INSERT INTO research_convergence_snapshots
              (id, run_id, iteration_id, source_count_total, source_count_new,
               primary_source_count_new, claim_count_total, statement_count_current,
               statement_count_changed, critical_open_challenges, high_open_challenges,
               strong_refutations, unknown_high_impact_claims, mean_confidence_delta,
               max_confidence_delta, source_novelty_score, claim_novelty_score,
               position_edit_distance, citation_support_score, active_fact_check_score,
               evaluator_score, cost_usd_estimated, elapsed_seconds, stop_rule_json,
               settled, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                    ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)
            ON CONFLICT(iteration_id) DO UPDATE SET
              source_count_total = excluded.source_count_total,
              source_count_new = excluded.source_count_new,
              primary_source_count_new = excluded.primary_source_count_new,
              claim_count_total = excluded.claim_count_total,
              statement_count_current = excluded.statement_count_current,
              statement_count_changed = excluded.statement_count_changed,
              critical_open_challenges = excluded.critical_open_challenges,
              high_open_challenges = excluded.high_open_challenges,
              strong_refutations = excluded.strong_refutations,
              unknown_high_impact_claims = excluded.unknown_high_impact_claims,
              mean_confidence_delta = excluded.mean_confidence_delta,
              max_confidence_delta = excluded.max_confidence_delta,
              source_novelty_score = excluded.source_novelty_score,
              claim_novelty_score = excluded.claim_novelty_score,
              position_edit_distance = excluded.position_edit_distance,
              citation_support_score = excluded.citation_support_score,
              active_fact_check_score = excluded.active_fact_check_score,
              evaluator_score = excluded.evaluator_score,
              cost_usd_estimated = excluded.cost_usd_estimated,
              elapsed_seconds = excluded.elapsed_seconds,
              stop_rule_json = excluded.stop_rule_json,
              settled = excluded.settled,
              created_at = excluded.created_at
            "#,
            params![
                snapshot.id,
                snapshot.run_id,
                snapshot.iteration_id,
                snapshot.source_count_total as i64,
                snapshot.source_count_new as i64,
                snapshot.primary_source_count_new as i64,
                snapshot.claim_count_total as i64,
                snapshot.statement_count_current as i64,
                snapshot.statement_count_changed as i64,
                snapshot.critical_open_challenges as i64,
                snapshot.high_open_challenges as i64,
                snapshot.strong_refutations as i64,
                snapshot.unknown_high_impact_claims as i64,
                snapshot.mean_confidence_delta,
                snapshot.max_confidence_delta,
                snapshot.source_novelty_score,
                snapshot.claim_novelty_score,
                snapshot.position_edit_distance,
                snapshot.citation_support_score,
                snapshot.active_fact_check_score,
                snapshot.evaluator_score,
                snapshot.cost_usd_estimated,
                snapshot.elapsed_seconds,
                canonical_json(&snapshot.stop_rule)?,
                if snapshot.settled { 1 } else { 0 },
                snapshot.created_at,
            ],
        )?;
        self.latest_research_convergence_snapshot(&snapshot.run_id)?
            .with_context(|| format!("inserted convergence snapshot not found: {}", snapshot.id))
    }

    pub(crate) fn record_research_report_judgment(
        &self,
        run_id: &str,
        report_id: Option<&str>,
        judgment: ResearchReportJudgment,
    ) -> Result<ResearchReportJudgment> {
        self.require_research_run(run_id)?;
        let mut normalized = judgment;
        normalize_research_report_judgment(&mut normalized)?;
        self.conn.execute(
            r#"
            INSERT INTO research_report_judgments
              (id, run_id, report_id, judgment_version, overall_decision, scores_json,
               blocking_findings_json, non_blocking_findings_json, evidence_checked_json,
               remaining_risks_json, commands_or_artifacts_reviewed_json, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(id) DO UPDATE SET
              run_id = excluded.run_id,
              report_id = excluded.report_id,
              judgment_version = excluded.judgment_version,
              overall_decision = excluded.overall_decision,
              scores_json = excluded.scores_json,
              blocking_findings_json = excluded.blocking_findings_json,
              non_blocking_findings_json = excluded.non_blocking_findings_json,
              evidence_checked_json = excluded.evidence_checked_json,
              remaining_risks_json = excluded.remaining_risks_json,
              commands_or_artifacts_reviewed_json = excluded.commands_or_artifacts_reviewed_json,
              created_at = excluded.created_at
            "#,
            params![
                normalized.id,
                run_id,
                report_id,
                normalized.judgment_version,
                normalized.overall_decision,
                canonical_json(&normalized.scores)?,
                canonical_json(&normalized.blocking_findings)?,
                canonical_json(&normalized.non_blocking_findings)?,
                canonical_json(&normalized.evidence_checked)?,
                canonical_json(&normalized.remaining_risks)?,
                canonical_json(&normalized.commands_or_artifacts_reviewed)?,
                normalized.created_at,
            ],
        )?;
        Ok(normalized)
    }

    pub(crate) fn upsert_research_cluster(
        &self,
        run_id: &str,
        theme: &str,
        summary: &str,
        claim_count: usize,
        evidence_strength: &str,
    ) -> Result<ResearchCluster> {
        let id = research_cluster_id(run_id, theme);
        let now = now();
        self.conn.execute(
            r#"
            INSERT INTO research_clusters
              (id, run_id, theme, summary, claim_count, evidence_strength, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
            ON CONFLICT(run_id, theme) DO UPDATE SET
              summary = excluded.summary,
              claim_count = excluded.claim_count,
              evidence_strength = excluded.evidence_strength,
              updated_at = excluded.updated_at
            "#,
            params![
                id,
                run_id,
                theme,
                summary,
                claim_count as i64,
                evidence_strength,
                now
            ],
        )?;
        self.read_research_cluster(&id)?
            .with_context(|| format!("inserted research cluster not found: {id}"))
    }

    pub(crate) fn read_research_cluster(&self, id: &str) -> Result<Option<ResearchCluster>> {
        validate_id(id)?;
        self.conn
            .query_row(
                r#"
                SELECT id, run_id, theme, summary, claim_count, evidence_strength, created_at, updated_at
                FROM research_clusters
                WHERE id = ?1
                "#,
                params![id],
                research_cluster_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn link_research_claim_to_cluster(
        &self,
        cluster_id: &str,
        claim_id: &str,
    ) -> Result<()> {
        let id = research_cluster_claim_id(cluster_id, claim_id);
        self.conn.execute(
            r#"
            INSERT OR IGNORE INTO research_cluster_claims (id, cluster_id, claim_id, created_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![id, cluster_id, claim_id, now()],
        )?;
        Ok(())
    }

    pub(crate) fn detect_and_record_research_contradictions(
        &self,
        run_id: &str,
        claims: &[ResearchClaimRecord],
    ) -> Result<Vec<ResearchContradiction>> {
        let mut contradictions = Vec::new();
        for left_index in 0..claims.len() {
            for right_index in (left_index + 1)..claims.len() {
                let left = &claims[left_index].claim;
                let right = &claims[right_index].claim;
                if !research_claims_conflict(left, right) {
                    continue;
                }
                let contradiction = self.insert_research_contradiction(
                    run_id,
                    &left.id,
                    &right.id,
                    "error",
                    &format!(
                        "`{}` conflicts with `{}` for subject {:?} predicate {:?}.",
                        left.text, right.text, left.subject, left.predicate
                    ),
                )?;
                contradictions.push(contradiction);
            }
        }
        Ok(contradictions)
    }

    pub(crate) fn insert_research_contradiction(
        &self,
        run_id: &str,
        left_claim_id: &str,
        right_claim_id: &str,
        severity: &str,
        notes: &str,
    ) -> Result<ResearchContradiction> {
        let id = research_contradiction_id(run_id, left_claim_id, right_claim_id);
        self.conn.execute(
            r#"
            INSERT INTO research_contradictions
              (id, run_id, left_claim_id, right_claim_id, severity, notes, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(run_id, left_claim_id, right_claim_id) DO UPDATE SET
              severity = excluded.severity,
              notes = excluded.notes
            "#,
            params![
                id,
                run_id,
                left_claim_id,
                right_claim_id,
                severity,
                notes,
                now()
            ],
        )?;
        self.read_research_contradiction(&id)?
            .with_context(|| format!("inserted research contradiction not found: {id}"))
    }

    pub(crate) fn read_research_contradiction(
        &self,
        id: &str,
    ) -> Result<Option<ResearchContradiction>> {
        validate_id(id)?;
        self.conn
            .query_row(
                r#"
                SELECT id, run_id, left_claim_id, right_claim_id, severity, notes, created_at
                FROM research_contradictions
                WHERE id = ?1
                "#,
                params![id],
                research_contradiction_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn upsert_research_claim(
        &self,
        run_id: &str,
        source_card_id: &str,
        extraction_provider: &str,
        extraction_model: &str,
        candidate: ResearchClaimCandidate,
    ) -> Result<ResearchClaimRecord> {
        let id = research_claim_id(run_id, source_card_id, &candidate.text);
        let caveats_json = serde_json::to_string(&candidate.caveats)?;
        let metadata_json = serde_json::to_string(&candidate.metadata)?;
        let now = now();
        self.conn.execute("BEGIN IMMEDIATE", [])?;
        let result = (|| -> Result<ResearchClaimRecord> {
            self.conn.execute(
                r#"
                INSERT INTO research_claims
                  (id, run_id, text, kind, subject, predicate, object_value, temporal_scope, confidence, caveats_json, extraction_provider, extraction_model, extracted_at, metadata_json, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?13, ?13)
                ON CONFLICT(id) DO UPDATE SET
                  text = excluded.text,
                  kind = excluded.kind,
                  subject = excluded.subject,
                  predicate = excluded.predicate,
                  object_value = excluded.object_value,
                  temporal_scope = excluded.temporal_scope,
                  confidence = excluded.confidence,
                  caveats_json = excluded.caveats_json,
                  extraction_provider = excluded.extraction_provider,
                  extraction_model = excluded.extraction_model,
                  extracted_at = excluded.extracted_at,
                  metadata_json = excluded.metadata_json,
                  updated_at = excluded.updated_at
                "#,
                params![
                    id,
                    run_id,
                    candidate.text,
                    candidate.kind,
                    candidate.subject,
                    candidate.predicate,
                    candidate.object_value,
                    candidate.temporal_scope,
                    candidate.confidence,
                    caveats_json,
                    extraction_provider,
                    extraction_model,
                    now,
                    metadata_json
                ],
            )?;
            let link_id = research_claim_source_id(&id, source_card_id);
            self.conn.execute(
                r#"
                INSERT INTO research_claim_sources
                  (id, claim_id, source_card_id, quote, source_anchor, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(claim_id, source_card_id) DO UPDATE SET
                  quote = excluded.quote,
                  source_anchor = excluded.source_anchor
                "#,
                params![
                    link_id,
                    id,
                    source_card_id,
                    candidate.quote,
                    candidate.source_anchor,
                    now
                ],
            )?;
            self.conn.execute(
                "DELETE FROM research_claim_document_anchors WHERE claim_source_id = ?1",
                params![link_id],
            )?;
            for evidence_anchor in &candidate.evidence_anchors {
                let anchor = self.resolve_research_claim_document_anchor(
                    run_id,
                    source_card_id,
                    &link_id,
                    evidence_anchor,
                )?;
                self.conn.execute(
                    r#"
                    INSERT INTO research_claim_document_anchors
                      (id, claim_source_id, document_id, anchor_kind, document_span_id, table_id,
                       table_cell_id, anchor_label, quote, created_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                    "#,
                    params![
                        anchor.id,
                        anchor.claim_source_id,
                        anchor.document_id,
                        anchor.anchor_kind,
                        anchor.document_span_id,
                        anchor.table_id,
                        anchor.table_cell_id,
                        anchor.anchor_label,
                        anchor.quote,
                        anchor.created_at,
                    ],
                )?;
            }
            let claim = self
                .read_research_claim(&id)?
                .with_context(|| format!("inserted research claim not found: {id}"))?;
            self.research_claim_record_from_claim(claim)
        })();
        match result {
            Ok(record) => {
                self.conn.execute("COMMIT", [])?;
                Ok(record)
            }
            Err(error) => {
                let _ = self.conn.execute("ROLLBACK", []);
                Err(error)
            }
        }
    }

    pub(crate) fn read_research_claim(&self, id: &str) -> Result<Option<ResearchClaim>> {
        validate_id(id)?;
        self.conn
            .query_row(
                r#"
                SELECT id, run_id, text, kind, subject, predicate, object_value, temporal_scope, confidence, caveats_json, extraction_provider, extraction_model, extracted_at, metadata_json
                FROM research_claims
                WHERE id = ?1
                "#,
                params![id],
                research_claim_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn research_claim_record_from_claim(
        &self,
        claim: ResearchClaim,
    ) -> Result<ResearchClaimRecord> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, claim_id, source_card_id, quote, source_anchor, created_at
            FROM research_claim_sources
            WHERE claim_id = ?1
            ORDER BY created_at ASC
            "#,
        )?;
        let sources = rows(stmt.query_map(params![claim.id], research_claim_source_from_row)?)?;
        let document_anchors = self.list_research_claim_document_anchors(&claim.id)?;
        Ok(ResearchClaimRecord {
            claim,
            sources,
            document_anchors,
        })
    }

    pub(crate) fn list_research_claim_document_anchors(
        &self,
        claim_id: &str,
    ) -> Result<Vec<ResearchClaimDocumentAnchor>> {
        validate_id(claim_id)?;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT a.id, a.claim_source_id, a.document_id, a.anchor_kind,
                   a.document_span_id, a.table_id, a.table_cell_id, a.anchor_label,
                   a.quote, a.created_at, s.span_id, t.table_id, c.row_index, c.column_index
            FROM research_claim_document_anchors a
            JOIN research_claim_sources cs ON cs.id = a.claim_source_id
            LEFT JOIN research_document_spans s ON s.id = a.document_span_id
            LEFT JOIN research_tables t ON t.id = a.table_id
            LEFT JOIN research_table_cells c ON c.id = a.table_cell_id
            WHERE cs.claim_id = ?1
            ORDER BY a.created_at ASC, a.anchor_label ASC, a.id ASC
            "#,
        )?;
        rows(stmt.query_map(params![claim_id], research_claim_document_anchor_from_row)?)
    }

    pub(crate) fn resolve_research_claim_document_anchor(
        &self,
        run_id: &str,
        source_card_id: &str,
        claim_source_id: &str,
        anchor: &ResearchEvidenceAnchor,
    ) -> Result<ResearchClaimDocumentAnchor> {
        validate_id(&anchor.document_id)?;
        let document = self
            .read_research_document(&anchor.document_id)?
            .with_context(|| format!("research document not found: {}", anchor.document_id))?;
        if document.document.run_id != run_id {
            bail!("research document anchor belongs to a different research run");
        }
        if document.document.extraction_status.starts_with("blocked_") {
            bail!("research document anchor points at a blocked extraction");
        }
        if document
            .document
            .source_card_id
            .as_deref()
            .is_some_and(|document_source_card_id| document_source_card_id != source_card_id)
        {
            bail!("research document anchor source card does not match claim source");
        }
        let now = now();
        match (
            &anchor.span_id,
            &anchor.table_id,
            anchor.row_index,
            anchor.column_index,
        ) {
            (Some(span_id), None, None, None) => {
                validate_research_anchor_label(span_id, "document span id")?;
                let span = document
                    .spans
                    .iter()
                    .find(|span| span.span_id == *span_id)
                    .with_context(|| {
                        format!(
                            "document span anchor not found: document={} span={}",
                            anchor.document_id, span_id
                        )
                    })?;
                let label = format!("doc:{}#span:{}", anchor.document_id, span.span_id);
                Ok(ResearchClaimDocumentAnchor {
                    id: research_claim_document_anchor_id(claim_source_id, &label),
                    claim_source_id: claim_source_id.to_string(),
                    document_id: anchor.document_id.clone(),
                    anchor_kind: "span".to_string(),
                    document_span_id: Some(span.id.clone()),
                    table_id: None,
                    table_cell_id: None,
                    span_id: Some(span.span_id.clone()),
                    table_local_id: None,
                    row_index: None,
                    column_index: None,
                    anchor_label: label,
                    quote: sanitize_optional_anchor_quote(anchor.quote.as_deref())?,
                    created_at: now,
                })
            }
            (None, Some(table_id), None, None) => {
                validate_research_anchor_label(table_id, "document table id")?;
                let table = find_document_table(&document, table_id)?;
                let label = format!("doc:{}#table:{}", anchor.document_id, table.table.table_id);
                Ok(ResearchClaimDocumentAnchor {
                    id: research_claim_document_anchor_id(claim_source_id, &label),
                    claim_source_id: claim_source_id.to_string(),
                    document_id: anchor.document_id.clone(),
                    anchor_kind: "table".to_string(),
                    document_span_id: None,
                    table_id: Some(table.table.id.clone()),
                    table_cell_id: None,
                    span_id: None,
                    table_local_id: Some(table.table.table_id.clone()),
                    row_index: None,
                    column_index: None,
                    anchor_label: label,
                    quote: sanitize_optional_anchor_quote(anchor.quote.as_deref())?,
                    created_at: now,
                })
            }
            (None, Some(table_id), Some(row_index), Some(column_index)) => {
                validate_research_anchor_label(table_id, "document table id")?;
                let table = find_document_table(&document, table_id)?;
                let cell = table
                    .cells
                    .iter()
                    .find(|cell| cell.row_index == row_index && cell.column_index == column_index)
                    .with_context(|| {
                        format!(
                            "document table cell anchor not found: document={} table={} row={} column={}",
                            anchor.document_id, table_id, row_index, column_index
                        )
                    })?;
                let label = format!(
                    "doc:{}#table:{}[r{},c{}]",
                    anchor.document_id, table.table.table_id, row_index, column_index
                );
                Ok(ResearchClaimDocumentAnchor {
                    id: research_claim_document_anchor_id(claim_source_id, &label),
                    claim_source_id: claim_source_id.to_string(),
                    document_id: anchor.document_id.clone(),
                    anchor_kind: "cell".to_string(),
                    document_span_id: None,
                    table_id: Some(table.table.id.clone()),
                    table_cell_id: Some(cell.id.clone()),
                    span_id: None,
                    table_local_id: Some(table.table.table_id.clone()),
                    row_index: Some(row_index),
                    column_index: Some(column_index),
                    anchor_label: label,
                    quote: sanitize_optional_anchor_quote(anchor.quote.as_deref())?
                        .or_else(|| Some(cell.normalized_text.clone())),
                    created_at: now,
                })
            }
            _ => bail!(
                "research document anchor must reference exactly one span, table, or table cell"
            ),
        }
    }

    pub(crate) fn require_source_card_linked_to_run(
        &self,
        run_id: &str,
        source_card_id: &str,
    ) -> Result<()> {
        validate_id(source_card_id)?;
        let linked = self
            .list_research_run_sources(run_id)?
            .into_iter()
            .any(|record| record.link.source_card_id.as_deref() == Some(source_card_id));
        if !linked {
            bail!("source card is not linked to research run: {source_card_id}");
        }
        Ok(())
    }

    pub(crate) fn read_research_run_source_record(
        &self,
        id: &str,
    ) -> Result<Option<ResearchRunSourceRecord>> {
        validate_id(id)?;
        let link = self
            .conn
            .query_row(
                r#"
                SELECT id, run_id, source_id, source_card_id, triage_status, read_depth, notes, created_at, updated_at
                FROM research_run_sources
                WHERE id = ?1
                "#,
                params![id],
                research_run_source_link_from_row,
            )
            .optional()?;
        link.map(|link| self.research_run_source_record_from_link(link))
            .transpose()
    }

    pub(crate) fn research_run_source_record_from_link(
        &self,
        link: ResearchRunSourceLink,
    ) -> Result<ResearchRunSourceRecord> {
        let source = self
            .read_research_source(&link.source_id)?
            .with_context(|| format!("linked research source not found: {}", link.source_id))?;
        let source_card = link
            .source_card_id
            .as_deref()
            .map(|id| {
                self.read_source_card(id)?
                    .with_context(|| format!("linked source card not found: {id}"))
            })
            .transpose()?;
        Ok(ResearchRunSourceRecord {
            source,
            link,
            source_card,
        })
    }

    pub fn upsert_watch_source(&self, input: WatchSourceInput) -> Result<WatchSource> {
        validate_watch_source_input(&input)?;
        let id = watch_source_id(&input.source_kind, &input.locator);
        let metadata_json = serde_json::to_string(&input.metadata)?;
        let existing = self.read_watch_source(&id)?;
        let now = now();
        let created_at = existing
            .as_ref()
            .map(|source| source.created_at.clone())
            .unwrap_or_else(|| now.clone());
        self.conn.execute(
            r#"
            INSERT INTO watch_sources
              (id, source_kind, locator, label, cadence, status, metadata_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(source_kind, locator) DO UPDATE SET
              label = excluded.label,
              cadence = excluded.cadence,
              status = excluded.status,
              metadata_json = excluded.metadata_json,
              updated_at = excluded.updated_at
            "#,
            params![
                id,
                input.source_kind,
                input.locator,
                input.label,
                input.cadence,
                input.status,
                metadata_json,
                created_at,
                now
            ],
        )?;
        self.read_watch_source(&id)?
            .with_context(|| format!("inserted watch source not found: {id}"))
    }

    pub fn list_watch_sources(&self) -> Result<Vec<WatchSource>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, source_kind, locator, label, cadence, status, metadata_json, created_at, updated_at
            FROM watch_sources
            ORDER BY source_kind, locator
            "#,
        )?;
        rows(stmt.query_map([], watch_source_from_row)?)
    }

    pub fn read_watch_source(&self, id: &str) -> Result<Option<WatchSource>> {
        validate_id(id)?;
        self.conn
            .query_row(
                r#"
                SELECT id, source_kind, locator, label, cadence, status, metadata_json, created_at, updated_at
                FROM watch_sources
                WHERE id = ?1
                "#,
                params![id],
                watch_source_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn delete_watch_sources_by_kind(&self, source_kind: &str) -> Result<usize> {
        validate_watch_source_kind(source_kind)?;
        self.conn
            .execute(
                "DELETE FROM watch_sources WHERE source_kind = ?1",
                params![source_kind],
            )
            .map_err(Into::into)
    }

    pub fn import_codex_swift_sources(&self, root: &Path) -> Result<WatchSourceImportReport> {
        let root = root
            .canonicalize()
            .with_context(|| format!("canonicalizing {}", root.display()))?;
        if !root.is_dir() {
            bail!(
                "codex-swift source root is not a directory: {}",
                root.display()
            );
        }

        let mut inputs = Vec::new();
        let mut errors = Vec::new();
        let mut skipped = 0;

        let restore_path = root.join("scripts").join("wiki-sources-restore.sh");
        match fs::read_to_string(&restore_path) {
            Ok(script) => {
                let parsed = parse_codex_swift_restore_script(&script);
                skipped += parsed.skipped;
                errors.extend(parsed.errors);
                inputs.extend(parsed.sources);
            }
            Err(error) => errors.push(format!("{}: {error}", restore_path.display())),
        }

        let llm_wiki_path = root.join("llm-wiki.md");
        match fs::read_to_string(&llm_wiki_path) {
            Ok(markdown) => {
                let parsed = parse_codex_swift_llm_wiki_sources(&markdown);
                skipped += parsed.skipped;
                errors.extend(parsed.errors);
                inputs.extend(parsed.sources);
            }
            Err(error) => errors.push(format!("{}: {error}", llm_wiki_path.display())),
        }

        let mut deduped_inputs: BTreeMap<(String, String), WatchSourceInput> = BTreeMap::new();
        for input in inputs {
            deduped_inputs.insert((input.source_kind.clone(), input.locator.clone()), input);
        }

        let mut added = 0;
        let mut updated = 0;
        let mut unchanged = 0;
        let mut by_kind = BTreeMap::new();

        for input in deduped_inputs.into_values() {
            match self.upsert_watch_source_with_status(input) {
                Ok((source, status)) => {
                    *by_kind.entry(source.source_kind.clone()).or_insert(0) += 1;
                    match status {
                        WatchSourceUpsertStatus::Added => added += 1,
                        WatchSourceUpsertStatus::Updated => updated += 1,
                        WatchSourceUpsertStatus::Unchanged => unchanged += 1,
                    }
                }
                Err(error) => {
                    skipped += 1;
                    errors.push(error.to_string());
                }
            }
        }

        Ok(WatchSourceImportReport {
            root,
            imported: added + updated + unchanged,
            added,
            updated,
            unchanged,
            skipped,
            by_kind,
            errors,
        })
    }

    pub(crate) fn upsert_watch_source_with_status(
        &self,
        input: WatchSourceInput,
    ) -> Result<(WatchSource, WatchSourceUpsertStatus)> {
        validate_watch_source_input(&input)?;
        let id = watch_source_id(&input.source_kind, &input.locator);
        let existing = self.read_watch_source(&id)?;
        let new_metadata = canonical_json(&input.metadata)?;
        let status = match &existing {
            None => WatchSourceUpsertStatus::Added,
            Some(existing) => {
                let old_metadata = canonical_json(&existing.metadata)?;
                if existing.source_kind == input.source_kind
                    && existing.locator == input.locator
                    && existing.label == input.label
                    && existing.cadence == input.cadence
                    && existing.status == input.status
                    && old_metadata == new_metadata
                {
                    WatchSourceUpsertStatus::Unchanged
                } else {
                    WatchSourceUpsertStatus::Updated
                }
            }
        };
        if matches!(status, WatchSourceUpsertStatus::Unchanged) {
            return Ok((existing.expect("existing checked above"), status));
        }
        Ok((self.upsert_watch_source(input)?, status))
    }
}
