use super::*;

impl Store {
    pub fn record_commerce_run_config(
        &self,
        input: CommerceRunConfigInput,
    ) -> Result<CommerceRunConfig> {
        let input = normalize_commerce_run_config_input(input)?;
        self.require_research_run(&input.run_id)?;
        let allowed_private_context_sources_json =
            serde_json::to_string(&input.allowed_private_context_sources)?;
        let allowed_public_source_families_json =
            serde_json::to_string(&input.allowed_public_source_families)?;
        let stop_rules_json = serde_json::to_string(&input.stop_rules)?;
        let timestamp = now();
        self.conn.execute(
            r#"
            INSERT INTO commerce_run_configs
              (run_id, domain_profile, target_qualified_count, geography, freshness_window, allowed_private_context_sources_json, allowed_public_source_families_json, allow_marketplaces, allow_chrome_profile, max_provider_calls, max_browser_pages, max_cost_usd, stop_rules_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)
            ON CONFLICT(run_id) DO UPDATE SET
              domain_profile = excluded.domain_profile,
              target_qualified_count = excluded.target_qualified_count,
              geography = excluded.geography,
              freshness_window = excluded.freshness_window,
              allowed_private_context_sources_json = excluded.allowed_private_context_sources_json,
              allowed_public_source_families_json = excluded.allowed_public_source_families_json,
              allow_marketplaces = excluded.allow_marketplaces,
              allow_chrome_profile = excluded.allow_chrome_profile,
              max_provider_calls = excluded.max_provider_calls,
              max_browser_pages = excluded.max_browser_pages,
              max_cost_usd = excluded.max_cost_usd,
              stop_rules_json = excluded.stop_rules_json,
              updated_at = excluded.updated_at
            "#,
            params![
                input.run_id,
                input.domain_profile,
                input.target_qualified_count as i64,
                input.geography,
                input.freshness_window,
                allowed_private_context_sources_json,
                allowed_public_source_families_json,
                if input.allow_marketplaces { 1 } else { 0 },
                if input.allow_chrome_profile { 1 } else { 0 },
                input.max_provider_calls.map(|value| value as i64),
                input.max_browser_pages.map(|value| value as i64),
                input.max_cost_usd,
                stop_rules_json,
                timestamp,
            ],
        )?;
        self.read_commerce_run_config(&input.run_id)?
            .with_context(|| format!("commerce run config not found: {}", input.run_id))
    }

    pub fn read_commerce_run_config(&self, run_id: &str) -> Result<Option<CommerceRunConfig>> {
        self.require_research_run(run_id)?;
        self.conn
            .query_row(
                r#"
                SELECT run_id, domain_profile, target_qualified_count, geography, freshness_window, allowed_private_context_sources_json, allowed_public_source_families_json, allow_marketplaces, allow_chrome_profile, max_provider_calls, max_browser_pages, max_cost_usd, stop_rules_json, created_at, updated_at
                FROM commerce_run_configs
                WHERE run_id = ?1
                "#,
                params![run_id],
                commerce_run_config_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn record_commerce_candidate(
        &self,
        input: CommerceCandidateInput,
    ) -> Result<CommerceCandidate> {
        let input = normalize_commerce_candidate_input(input)?;
        self.require_research_run(&input.run_id)?;
        let id = commerce_candidate_id(
            &input.run_id,
            &input.source_url,
            &input.normalized_item_key,
            &input.variant_key,
        );
        let score_reasons_json = serde_json::to_string(&input.score_reasons)?;
        let disqualification_reasons_json = serde_json::to_string(&input.disqualification_reasons)?;
        let metadata_json = serde_json::to_string(&input.metadata)?;
        let timestamp = now();
        self.conn.execute(
            r#"
            INSERT INTO commerce_candidates
              (id, run_id, domain, source_url, retailer_or_provider, title, normalized_item_key, variant_key, price, currency, geography, candidate_status, score, score_reasons_json, disqualification_reasons_json, metadata_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?17)
            ON CONFLICT(run_id, source_url, normalized_item_key, variant_key) DO UPDATE SET
              retailer_or_provider = excluded.retailer_or_provider,
              title = excluded.title,
              price = excluded.price,
              currency = excluded.currency,
              geography = excluded.geography,
              candidate_status = excluded.candidate_status,
              score = excluded.score,
              score_reasons_json = excluded.score_reasons_json,
              disqualification_reasons_json = excluded.disqualification_reasons_json,
              metadata_json = excluded.metadata_json,
              updated_at = excluded.updated_at
            "#,
            params![
                id,
                input.run_id,
                input.domain,
                input.source_url,
                input.retailer_or_provider,
                input.title,
                input.normalized_item_key,
                input.variant_key,
                input.price,
                input.currency,
                input.geography,
                input.candidate_status,
                input.score,
                score_reasons_json,
                disqualification_reasons_json,
                metadata_json,
                timestamp,
            ],
        )?;
        self.read_commerce_candidate(&id)?
            .with_context(|| format!("commerce candidate not found: {id}"))
    }

    pub fn list_commerce_candidates(&self, run_id: &str) -> Result<Vec<CommerceCandidate>> {
        self.require_research_run(run_id)?;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, run_id, domain, source_url, retailer_or_provider, title, normalized_item_key, variant_key, price, currency, geography, candidate_status, score, score_reasons_json, disqualification_reasons_json, metadata_json, created_at, updated_at
            FROM commerce_candidates
            WHERE run_id = ?1
            ORDER BY created_at ASC
            "#,
        )?;
        rows(stmt.query_map(params![run_id], commerce_candidate_from_row)?)
    }

    pub fn read_commerce_candidate(&self, id: &str) -> Result<Option<CommerceCandidate>> {
        validate_id(id)?;
        self.conn
            .query_row(
                r#"
                SELECT id, run_id, domain, source_url, retailer_or_provider, title, normalized_item_key, variant_key, price, currency, geography, candidate_status, score, score_reasons_json, disqualification_reasons_json, metadata_json, created_at, updated_at
                FROM commerce_candidates
                WHERE id = ?1
                "#,
                params![id],
                commerce_candidate_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn record_commerce_availability_proof(
        &self,
        input: CommerceAvailabilityProofInput,
    ) -> Result<CommerceAvailabilityProof> {
        let input = normalize_commerce_availability_proof_input(input)?;
        self.require_research_run(&input.run_id)?;
        let candidate = self
            .read_commerce_candidate(&input.candidate_id)?
            .with_context(|| format!("commerce candidate not found: {}", input.candidate_id))?;
        if candidate.run_id != input.run_id {
            bail!("commerce availability proof candidate belongs to a different run");
        }
        if candidate.variant_key != input.variant_key {
            bail!("commerce availability proof variant does not match candidate variant");
        }
        if input.availability_state == "available"
            && input.screenshot_artifact_id.is_none()
            && input.page_snapshot_artifact_id.is_none()
        {
            bail!(
                "available commerce proof requires screenshot or page-snapshot artifact provenance"
            );
        }
        for artifact_id in [
            input.screenshot_artifact_id.as_ref(),
            input.page_snapshot_artifact_id.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            let artifact = self
                .read_research_artifact(artifact_id)?
                .with_context(|| format!("commerce proof artifact not found: {artifact_id}"))?;
            if artifact.run_id != input.run_id {
                bail!("commerce proof artifact belongs to a different research run");
            }
        }
        let id = commerce_availability_proof_id();
        let caveats_json = serde_json::to_string(&input.caveats)?;
        let created_at = now();
        self.conn.execute(
            r#"
            INSERT INTO commerce_availability_proofs
              (id, run_id, candidate_id, proof_method, variant_key, variant_label, availability_state, visible_evidence, selector_or_dom_hint, screenshot_artifact_id, page_snapshot_artifact_id, confidence, caveats_json, checked_at, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
            params![
                id,
                input.run_id,
                input.candidate_id,
                input.proof_method,
                input.variant_key,
                input.variant_label,
                input.availability_state,
                input.visible_evidence,
                input.selector_or_dom_hint,
                input.screenshot_artifact_id,
                input.page_snapshot_artifact_id,
                input.confidence,
                caveats_json,
                input.checked_at.unwrap_or_else(now),
                created_at,
            ],
        )?;
        self.read_commerce_availability_proof(&id)?
            .with_context(|| format!("commerce availability proof not found: {id}"))
    }

    pub fn list_commerce_availability_proofs(
        &self,
        run_id: &str,
    ) -> Result<Vec<CommerceAvailabilityProof>> {
        self.require_research_run(run_id)?;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, run_id, candidate_id, proof_method, variant_key, variant_label, availability_state, visible_evidence, selector_or_dom_hint, screenshot_artifact_id, page_snapshot_artifact_id, confidence, caveats_json, checked_at, created_at
            FROM commerce_availability_proofs
            WHERE run_id = ?1
            ORDER BY checked_at ASC, created_at ASC
            "#,
        )?;
        rows(stmt.query_map(params![run_id], commerce_availability_proof_from_row)?)
    }

    pub fn read_commerce_availability_proof(
        &self,
        id: &str,
    ) -> Result<Option<CommerceAvailabilityProof>> {
        validate_id(id)?;
        self.conn
            .query_row(
                r#"
                SELECT id, run_id, candidate_id, proof_method, variant_key, variant_label, availability_state, visible_evidence, selector_or_dom_hint, screenshot_artifact_id, page_snapshot_artifact_id, confidence, caveats_json, checked_at, created_at
                FROM commerce_availability_proofs
                WHERE id = ?1
                "#,
                params![id],
                commerce_availability_proof_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn record_commerce_context_fact(
        &self,
        input: CommerceContextFactInput,
    ) -> Result<CommerceContextFact> {
        let input = normalize_commerce_context_fact_input(input)?;
        self.require_research_run(&input.run_id)?;
        let id = commerce_context_fact_id(&input.run_id, &input.fact_key, &input.source_family);
        let metadata_json = serde_json::to_string(&input.metadata)?;
        self.conn.execute(
            r#"
            INSERT INTO commerce_context_facts
              (id, run_id, fact_key, fact_kind, redacted_value, source_family, source_ref, confidence, user_confirmed, may_persist_to_memory, metadata_json, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(id) DO UPDATE SET
              fact_kind = excluded.fact_kind,
              redacted_value = excluded.redacted_value,
              source_ref = excluded.source_ref,
              confidence = excluded.confidence,
              user_confirmed = excluded.user_confirmed,
              may_persist_to_memory = excluded.may_persist_to_memory,
              metadata_json = excluded.metadata_json
            "#,
            params![
                id,
                input.run_id,
                input.fact_key,
                input.fact_kind,
                input.redacted_value,
                input.source_family,
                input.source_ref,
                input.confidence,
                if input.user_confirmed { 1 } else { 0 },
                if input.may_persist_to_memory { 1 } else { 0 },
                metadata_json,
                now(),
            ],
        )?;
        self.read_commerce_context_fact(&id)?
            .with_context(|| format!("commerce context fact not found: {id}"))
    }

    pub fn list_commerce_context_facts(&self, run_id: &str) -> Result<Vec<CommerceContextFact>> {
        self.require_research_run(run_id)?;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, run_id, fact_key, fact_kind, redacted_value, source_family, source_ref, confidence, user_confirmed, may_persist_to_memory, metadata_json, created_at
            FROM commerce_context_facts
            WHERE run_id = ?1
            ORDER BY created_at ASC
            "#,
        )?;
        rows(stmt.query_map(params![run_id], commerce_context_fact_from_row)?)
    }

    pub fn read_commerce_context_fact(&self, id: &str) -> Result<Option<CommerceContextFact>> {
        validate_id(id)?;
        self.conn
            .query_row(
                r#"
                SELECT id, run_id, fact_key, fact_kind, redacted_value, source_family, source_ref, confidence, user_confirmed, may_persist_to_memory, metadata_json, created_at
                FROM commerce_context_facts
                WHERE id = ?1
                "#,
                params![id],
                commerce_context_fact_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn record_commerce_verification_attempt(
        &self,
        input: CommerceVerificationAttemptInput,
    ) -> Result<CommerceVerificationAttempt> {
        let input = normalize_commerce_verification_attempt_input(input)?;
        self.require_research_run(&input.run_id)?;
        let candidate = self
            .read_commerce_candidate(&input.candidate_id)?
            .with_context(|| format!("commerce candidate not found: {}", input.candidate_id))?;
        if candidate.run_id != input.run_id {
            bail!("commerce verification candidate belongs to a different run");
        }
        for artifact_id in &input.artifact_ids {
            let artifact = self.read_research_artifact(artifact_id)?.with_context(|| {
                format!("commerce verification artifact not found: {artifact_id}")
            })?;
            if artifact.run_id != input.run_id {
                bail!("commerce verification artifact belongs to a different research run");
            }
        }
        let id = commerce_verification_attempt_id();
        let artifact_ids_json = serde_json::to_string(&input.artifact_ids)?;
        let created_at = now();
        self.conn.execute(
            r#"
            INSERT INTO commerce_verification_attempts
              (id, run_id, candidate_id, attempted_at, method, result, error_kind, final_url, http_status, browser_required, chrome_profile_required, artifact_ids_json, next_action, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            "#,
            params![
                id,
                input.run_id,
                input.candidate_id,
                input.attempted_at.unwrap_or_else(now),
                input.method,
                input.result,
                input.error_kind,
                input.final_url,
                input.http_status,
                if input.browser_required { 1 } else { 0 },
                if input.chrome_profile_required { 1 } else { 0 },
                artifact_ids_json,
                input.next_action,
                created_at,
            ],
        )?;
        self.read_commerce_verification_attempt(&id)?
            .with_context(|| format!("commerce verification attempt not found: {id}"))
    }

    pub fn list_commerce_verification_attempts(
        &self,
        run_id: &str,
    ) -> Result<Vec<CommerceVerificationAttempt>> {
        self.require_research_run(run_id)?;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, run_id, candidate_id, attempted_at, method, result, error_kind, final_url, http_status, browser_required, chrome_profile_required, artifact_ids_json, next_action, created_at
            FROM commerce_verification_attempts
            WHERE run_id = ?1
            ORDER BY attempted_at ASC, created_at ASC
            "#,
        )?;
        rows(stmt.query_map(params![run_id], commerce_verification_attempt_from_row)?)
    }

    pub fn read_commerce_verification_attempt(
        &self,
        id: &str,
    ) -> Result<Option<CommerceVerificationAttempt>> {
        validate_id(id)?;
        self.conn
            .query_row(
                r#"
                SELECT id, run_id, candidate_id, attempted_at, method, result, error_kind, final_url, http_status, browser_required, chrome_profile_required, artifact_ids_json, next_action, created_at
                FROM commerce_verification_attempts
                WHERE id = ?1
                "#,
                params![id],
                commerce_verification_attempt_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn record_commerce_report_judgment(
        &self,
        input: CommerceReportJudgmentInput,
    ) -> Result<CommerceReportJudgment> {
        let input = normalize_commerce_report_judgment_input(input)?;
        self.require_research_run(&input.run_id)?;
        let id = commerce_report_judgment_id();
        self.conn.execute(
            r#"
            INSERT INTO commerce_report_judgments
              (id, run_id, decision, blocking_findings_json, non_blocking_findings_json, claims_checked_json, availability_proofs_checked_json, privacy_review_json, remaining_risks_json, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                id,
                input.run_id,
                input.decision,
                serde_json::to_string(&input.blocking_findings)?,
                serde_json::to_string(&input.non_blocking_findings)?,
                serde_json::to_string(&input.claims_checked)?,
                serde_json::to_string(&input.availability_proofs_checked)?,
                serde_json::to_string(&input.privacy_review)?,
                serde_json::to_string(&input.remaining_risks)?,
                now(),
            ],
        )?;
        self.read_commerce_report_judgment(&id)?
            .with_context(|| format!("commerce report judgment not found: {id}"))
    }

    pub fn list_commerce_report_judgments(
        &self,
        run_id: &str,
    ) -> Result<Vec<CommerceReportJudgment>> {
        self.require_research_run(run_id)?;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, run_id, decision, blocking_findings_json, non_blocking_findings_json, claims_checked_json, availability_proofs_checked_json, privacy_review_json, remaining_risks_json, created_at
            FROM commerce_report_judgments
            WHERE run_id = ?1
            ORDER BY created_at ASC
            "#,
        )?;
        rows(stmt.query_map(params![run_id], commerce_report_judgment_from_row)?)
    }

    pub fn read_commerce_report_judgment(
        &self,
        id: &str,
    ) -> Result<Option<CommerceReportJudgment>> {
        validate_id(id)?;
        self.conn
            .query_row(
                r#"
                SELECT id, run_id, decision, blocking_findings_json, non_blocking_findings_json, claims_checked_json, availability_proofs_checked_json, privacy_review_json, remaining_risks_json, created_at
                FROM commerce_report_judgments
                WHERE id = ?1
                "#,
                params![id],
                commerce_report_judgment_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn record_commerce_rendered_page_check(
        &self,
        input: CommerceRenderedPageCheckInput,
    ) -> Result<CommerceRenderedPageCheck> {
        let input = normalize_commerce_rendered_page_check_input(input)?;
        self.require_research_run(&input.run_id)?;
        let mut candidate = self
            .read_commerce_candidate(&input.candidate_id)?
            .with_context(|| format!("commerce candidate not found: {}", input.candidate_id))?;
        if candidate.run_id != input.run_id {
            bail!("commerce rendered page check candidate belongs to a different run");
        }
        if candidate.variant_key != input.variant_key {
            bail!("commerce rendered page check variant does not match candidate variant");
        }
        let doc = rendered_page_snapshot_document(&input.snapshot)?;
        let checked_at = input.snapshot.captured_at.clone().unwrap_or_else(now);
        let rendered = classify_commerce_rendered_availability(
            &doc.readable_text,
            &input.variant_label,
            input.chrome_profile_required,
        )?;
        let rendered = if rendered.availability_state == "available"
            && input.selector_or_dom_hint.is_none()
        {
            CommerceRenderedAvailability {
                availability_state: "unknown".to_string(),
                visible_evidence: rendered.visible_evidence.clone(),
                confidence: 0.45,
                caveats: json!([
                    "Positive availability cue was visible near the variant, but no selector or DOM hint was supplied for recommendation-grade proof.",
                    rendered.caveats
                ]),
                next_action: "Provide a selector or DOM hint for the selected variant control before treating this as available.".to_string(),
            }
        } else {
            rendered
        };
        let structured = extract_commerce_rendered_structured_fields(&doc.readable_text);
        if candidate.price.is_none() && structured.price.is_some()
            || candidate.currency.is_none() && structured.currency.is_some()
        {
            candidate = self.record_commerce_candidate(CommerceCandidateInput {
                run_id: candidate.run_id.clone(),
                domain: candidate.domain.clone(),
                source_url: candidate.source_url.clone(),
                retailer_or_provider: candidate.retailer_or_provider.clone(),
                title: candidate.title.clone(),
                normalized_item_key: candidate.normalized_item_key.clone(),
                variant_key: candidate.variant_key.clone(),
                price: candidate.price.clone().or_else(|| structured.price.clone()),
                currency: candidate
                    .currency
                    .clone()
                    .or_else(|| structured.currency.clone()),
                geography: candidate.geography.clone(),
                candidate_status: candidate.candidate_status.clone(),
                score: candidate.score,
                score_reasons: candidate.score_reasons.clone(),
                disqualification_reasons: candidate.disqualification_reasons.clone(),
                metadata: candidate.metadata.clone(),
            })?;
        }
        let source_card = self.add_source_card(SourceCardInput {
            title: doc.title.clone(),
            url: doc.final_url.clone(),
            source_type: "web".to_string(),
            provider: "commerce-rendered-page".to_string(),
            summary: commerce_rendered_source_card_summary(
                &candidate,
                &input,
                &rendered,
                &structured,
                &doc,
            ),
            claims: commerce_rendered_source_card_claims(
                &candidate,
                &input,
                &rendered,
                &structured,
            ),
            retrieved_at: Some(checked_at.clone()),
            metadata: json!({
                "source_family": "commerce_retailer",
                "source_role": "primary",
                "trust_level": "medium",
                "commerce_candidate_id": candidate.id,
                "commerce_variant_key": input.variant_key,
                "commerce_variant_label": input.variant_label,
                "commerce_availability_state": rendered.availability_state,
                "commerce_price": structured.price,
                "commerce_currency": structured.currency,
                "commerce_shipping_caveat": structured.shipping_caveat,
                "capture_method": "host_supplied_rendered_page",
                "browser": doc.browser,
                "screenshot_path": doc.screenshot_path
            }),
        })?;
        let research_source_link = self.link_source_card_to_research_run(
            &input.run_id,
            &source_card.id,
            "commerce_retailer",
            "rendered-page",
            if rendered.availability_state == "available" {
                "checked"
            } else {
                "needs-review"
            },
            Some("Commerce rendered-page check source card"),
        )?;
        let page_snapshot_artifact = self.record_research_artifact(ResearchArtifactInput {
            run_id: input.run_id.clone(),
            role_run_id: None,
            artifact_type: "commerce_rendered_page_snapshot".to_string(),
            title: format!("Rendered commerce page: {}", doc.title),
            body: render_commerce_rendered_page_artifact(&doc, &input, &rendered),
            metadata: json!({
                "requested_url": doc.requested_url,
                "final_url": doc.final_url,
                "canonical_url": doc.canonical_url,
                "content_type": doc.content_type,
                "byte_len": doc.byte_len,
                "extraction_method": doc.extraction_method,
                "captured_at": checked_at,
                "browser": doc.browser,
                "screenshot_path": doc.screenshot_path,
                "candidate_id": candidate.id,
                "variant_key": input.variant_key,
                "variant_label": input.variant_label,
                "availability_state": rendered.availability_state,
                "source_card_id": source_card.id,
                "research_source_link_id": research_source_link.link.id,
                "extracted_price": structured.price,
                "extracted_currency": structured.currency,
                "shipping_caveat": structured.shipping_caveat,
                "source": "host_supplied_rendered_page"
            }),
        })?;
        let needs_next_action =
            rendered.availability_state == "blocked" || rendered.availability_state == "unknown";
        let verification_attempt =
            self.record_commerce_verification_attempt(CommerceVerificationAttemptInput {
                run_id: input.run_id.clone(),
                candidate_id: input.candidate_id.clone(),
                method: if input.chrome_profile_required {
                    "chrome_profile".to_string()
                } else {
                    "rendered_browser".to_string()
                },
                result: rendered.availability_state.clone(),
                error_kind: (rendered.availability_state == "blocked")
                    .then(|| "rendered_page_blocked".to_string()),
                final_url: Some(doc.final_url.clone()),
                http_status: None,
                browser_required: true,
                chrome_profile_required: input.chrome_profile_required,
                artifact_ids: vec![page_snapshot_artifact.id.clone()],
                next_action: needs_next_action.then(|| rendered.next_action.clone()),
                attempted_at: Some(checked_at.clone()),
            })?;
        let availability_proof =
            self.record_commerce_availability_proof(CommerceAvailabilityProofInput {
                run_id: input.run_id,
                candidate_id: input.candidate_id,
                proof_method: if input.chrome_profile_required {
                    "chrome_profile".to_string()
                } else {
                    "rendered_browser".to_string()
                },
                variant_key: input.variant_key,
                variant_label: input.variant_label,
                availability_state: rendered.availability_state.clone(),
                visible_evidence: rendered.visible_evidence.clone(),
                selector_or_dom_hint: input.selector_or_dom_hint,
                screenshot_artifact_id: None,
                page_snapshot_artifact_id: Some(page_snapshot_artifact.id.clone()),
                confidence: rendered.confidence,
                caveats: rendered.caveats.clone(),
                checked_at: Some(checked_at.clone()),
            })?;
        Ok(CommerceRenderedPageCheck {
            candidate,
            page_snapshot_artifact,
            source_card,
            research_source_link,
            verification_attempt,
            availability_proof,
            availability_state: rendered.availability_state,
            visible_evidence: rendered.visible_evidence,
            extracted_price: structured.price,
            extracted_currency: structured.currency,
            shipping_caveat: structured.shipping_caveat,
            checked_at,
            caveats: rendered.caveats,
        })
    }

    pub fn compile_commerce_context_packet(&self, run_id: &str) -> Result<CommerceContextPacket> {
        self.require_research_run(run_id)?;
        let facts = self.list_commerce_context_facts(run_id)?;
        let missing_fact_count = facts
            .iter()
            .filter(|fact| fact.fact_kind == "missing")
            .count();
        let user_confirmed_count = facts.iter().filter(|fact| fact.user_confirmed).count();
        let may_persist_to_memory_count = facts
            .iter()
            .filter(|fact| fact.may_persist_to_memory)
            .count();
        let artifact = self.record_research_artifact(ResearchArtifactInput {
            run_id: run_id.to_string(),
            role_run_id: None,
            artifact_type: "commerce_context_packet".to_string(),
            title: "Qualified commerce context packet".to_string(),
            body: render_commerce_context_packet(run_id, &facts),
            metadata: json!({
                "fact_count": facts.len(),
                "missing_fact_count": missing_fact_count,
                "user_confirmed_count": user_confirmed_count,
                "may_persist_to_memory_count": may_persist_to_memory_count,
                "raw_private_data_policy": "redacted facts only; raw private sources are not copied into this artifact"
            }),
        })?;
        Ok(CommerceContextPacket {
            run_id: run_id.to_string(),
            artifact,
            fact_count: facts.len(),
            missing_fact_count,
            user_confirmed_count,
            may_persist_to_memory_count,
        })
    }

    pub fn compile_commerce_report(&self, run_id: &str) -> Result<CommerceReport> {
        self.require_research_run(run_id)?;
        let config = self.read_commerce_run_config(run_id)?;
        let candidates = self.list_commerce_candidates(run_id)?;
        let proofs = self.list_commerce_availability_proofs(run_id)?;
        let context_facts = self.list_commerce_context_facts(run_id)?;
        let attempts = self.list_commerce_verification_attempts(run_id)?;
        let source_links = self.list_research_run_sources(run_id)?;
        let report_model =
            build_commerce_report_model(&candidates, &proofs, &attempts, &context_facts);
        let blocking_findings = commerce_report_blocking_findings(&report_model, &config);
        let decision = if blocking_findings.is_empty() {
            "accept"
        } else {
            "hold"
        };
        let artifact = self.record_research_artifact(ResearchArtifactInput {
            run_id: run_id.to_string(),
            role_run_id: None,
            artifact_type: "commerce_report".to_string(),
            title: "Qualified commerce report".to_string(),
            body: render_commerce_report(run_id, config.as_ref(), &report_model, &source_links),
            metadata: json!({
                "recommended_count": report_model.recommended.len(),
                "unavailable_count": report_model.unavailable.len(),
                "blocked_count": report_model.blocked.len(),
                "unknown_count": report_model.unknown.len(),
                "context_fact_count": context_facts.len(),
                "source_card_count": source_links.len(),
                "decision": decision
            }),
        })?;
        let judgment = self.record_commerce_report_judgment(CommerceReportJudgmentInput {
            run_id: run_id.to_string(),
            decision: decision.to_string(),
            blocking_findings: json!(blocking_findings),
            non_blocking_findings: json!(commerce_report_non_blocking_findings(
                &report_model,
                &source_links
            )),
            claims_checked: json!([
                "exact_variant_availability",
                "private_context_redaction",
                "source_card_linkage"
            ]),
            availability_proofs_checked: json!(
                proofs
                    .iter()
                    .map(|proof| proof.id.clone())
                    .collect::<Vec<_>>()
            ),
            privacy_review: json!({
                "context_packet_redacted": true,
                "raw_private_sources_in_report": false
            }),
            remaining_risks: json!(commerce_report_remaining_risks(
                &report_model,
                config.as_ref()
            )),
        })?;
        Ok(CommerceReport {
            run_id: run_id.to_string(),
            artifact,
            judgment,
            recommended_count: report_model.recommended.len(),
            unavailable_count: report_model.unavailable.len(),
            blocked_count: report_model.blocked.len(),
            unknown_count: report_model.unknown.len(),
            context_fact_count: context_facts.len(),
            source_card_count: source_links.len(),
        })
    }
}
