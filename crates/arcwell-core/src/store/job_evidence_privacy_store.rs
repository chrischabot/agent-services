use super::*;

impl Store {
    pub fn record_job_candidate_profile(
        &self,
        input: JobCandidateProfileInput,
    ) -> Result<JobCandidateProfile> {
        let input = normalize_job_candidate_profile_input(input)?;
        let id = job_candidate_profile_id(&input.label);
        let metadata_json = serde_json::to_string(&input.metadata)?;
        let timestamp = now();
        self.conn.execute(
            r#"
            INSERT INTO job_candidate_profiles
              (id, label, current_resume_source, linkedin_source, github_profile, blog_url, metadata_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
            ON CONFLICT(label) DO UPDATE SET
              current_resume_source = excluded.current_resume_source,
              linkedin_source = excluded.linkedin_source,
              github_profile = excluded.github_profile,
              blog_url = excluded.blog_url,
              metadata_json = excluded.metadata_json,
              updated_at = excluded.updated_at
            "#,
            params![
                id,
                input.label,
                input.current_resume_source,
                input.linkedin_source,
                input.github_profile,
                input.blog_url,
                metadata_json,
                timestamp,
            ],
        )?;
        self.read_job_candidate_profile(&id)?
            .with_context(|| format!("job candidate profile not found: {id}"))
    }

    pub fn read_job_candidate_profile(&self, id: &str) -> Result<Option<JobCandidateProfile>> {
        validate_id(id)?;
        self.conn
            .query_row(
                r#"
                SELECT id, label, current_resume_source, linkedin_source, github_profile, blog_url, metadata_json, created_at, updated_at
                FROM job_candidate_profiles
                WHERE id = ?1
                "#,
                params![id],
                job_candidate_profile_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_job_candidate_profiles(&self) -> Result<Vec<JobCandidateProfile>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, label, current_resume_source, linkedin_source, github_profile, blog_url, metadata_json, created_at, updated_at
            FROM job_candidate_profiles
            ORDER BY label ASC
            "#,
        )?;
        rows(stmt.query_map([], job_candidate_profile_from_row)?)
    }

    pub fn record_job_evidence_card(&self, input: JobEvidenceCardInput) -> Result<JobEvidenceCard> {
        let input = normalize_job_evidence_card_input(input)?;
        self.require_job_profile(&input.profile_id)?;
        let id = job_evidence_card_id(
            &input.profile_id,
            &input.title,
            &input.evidence_type,
            input.proof_url.as_deref().or(input.local_path.as_deref()),
        );
        let tags_json = serde_json::to_string(&input.tags)?;
        let unsafe_terms_json = serde_json::to_string(&input.unsafe_terms)?;
        let metadata_json = serde_json::to_string(&input.metadata)?;
        let timestamp = now();
        self.conn.execute(
            r#"
            INSERT INTO job_evidence_cards
              (id, profile_id, title, evidence_type, visibility, summary, proof_url, local_path, source_date, confidence, tags_json, safe_application_text, unsafe_terms_json, metadata_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)
            ON CONFLICT(id) DO UPDATE SET
              visibility = excluded.visibility,
              summary = excluded.summary,
              proof_url = excluded.proof_url,
              local_path = excluded.local_path,
              source_date = excluded.source_date,
              confidence = excluded.confidence,
              tags_json = excluded.tags_json,
              safe_application_text = excluded.safe_application_text,
              unsafe_terms_json = excluded.unsafe_terms_json,
              metadata_json = excluded.metadata_json,
              updated_at = excluded.updated_at
            "#,
            params![
                id,
                input.profile_id,
                input.title,
                input.evidence_type,
                input.visibility,
                input.summary,
                input.proof_url,
                input.local_path,
                input.source_date,
                input.confidence,
                tags_json,
                input.safe_application_text,
                unsafe_terms_json,
                metadata_json,
                timestamp,
            ],
        )?;
        self.read_job_evidence_card(&id)?
            .with_context(|| format!("job evidence card not found: {id}"))
    }

    pub fn read_job_evidence_card(&self, id: &str) -> Result<Option<JobEvidenceCard>> {
        validate_id(id)?;
        self.conn
            .query_row(
                r#"
                SELECT id, profile_id, title, evidence_type, visibility, summary, proof_url, local_path, source_date, confidence, tags_json, safe_application_text, unsafe_terms_json, metadata_json, created_at, updated_at
                FROM job_evidence_cards
                WHERE id = ?1
                "#,
                params![id],
                job_evidence_card_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_job_evidence_cards(&self, profile_id: &str) -> Result<Vec<JobEvidenceCard>> {
        self.require_job_profile(profile_id)?;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, profile_id, title, evidence_type, visibility, summary, proof_url, local_path, source_date, confidence, tags_json, safe_application_text, unsafe_terms_json, metadata_json, created_at, updated_at
            FROM job_evidence_cards
            WHERE profile_id = ?1
            ORDER BY created_at ASC
            "#,
        )?;
        rows(stmt.query_map(params![profile_id], job_evidence_card_from_row)?)
    }

    pub fn record_job_evidence_claim(
        &self,
        input: JobEvidenceClaimInput,
    ) -> Result<JobEvidenceClaim> {
        let input = normalize_job_evidence_claim_input(input)?;
        let card = self
            .read_job_evidence_card(&input.evidence_card_id)?
            .with_context(|| format!("job evidence card not found: {}", input.evidence_card_id))?;
        if card.visibility == "private_blocked"
            && (input.can_use_in_resume || input.can_use_in_outreach || input.can_use_in_interview)
        {
            bail!("private-blocked job evidence cannot be marked usable");
        }
        if input.proof_level == "private" && (input.can_use_in_resume || input.can_use_in_outreach)
        {
            bail!("private-only job evidence cannot be used in resume or outreach claims");
        }
        let id = job_evidence_claim_id(&input.evidence_card_id, &input.claim);
        self.conn.execute(
            r#"
            INSERT INTO job_evidence_claims
              (id, evidence_card_id, claim, claim_kind, proof_level, can_use_in_resume, can_use_in_outreach, can_use_in_interview, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO UPDATE SET
              claim_kind = excluded.claim_kind,
              proof_level = excluded.proof_level,
              can_use_in_resume = excluded.can_use_in_resume,
              can_use_in_outreach = excluded.can_use_in_outreach,
              can_use_in_interview = excluded.can_use_in_interview
            "#,
            params![
                id,
                input.evidence_card_id,
                input.claim,
                input.claim_kind,
                input.proof_level,
                if input.can_use_in_resume { 1 } else { 0 },
                if input.can_use_in_outreach { 1 } else { 0 },
                if input.can_use_in_interview { 1 } else { 0 },
                now(),
            ],
        )?;
        self.read_job_evidence_claim(&id)?
            .with_context(|| format!("job evidence claim not found: {id}"))
    }

    pub fn read_job_evidence_claim(&self, id: &str) -> Result<Option<JobEvidenceClaim>> {
        validate_id(id)?;
        self.conn
            .query_row(
                r#"
                SELECT id, evidence_card_id, claim, claim_kind, proof_level, can_use_in_resume, can_use_in_outreach, can_use_in_interview, created_at
                FROM job_evidence_claims
                WHERE id = ?1
                "#,
                params![id],
                job_evidence_claim_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn compile_job_evidence_review_report(
        &self,
        profile_id: &str,
    ) -> Result<JobEvidenceReviewReport> {
        self.require_job_profile(profile_id)?;
        let cards = self.list_job_evidence_cards(profile_id)?;
        let claims = self.list_job_evidence_claims_for_profile(profile_id)?;

        let mut counts_by_visibility = BTreeMap::new();
        let mut counts_by_evidence_type = BTreeMap::new();
        let mut counts_by_confidence = BTreeMap::new();
        let mut counts_by_proof_level = BTreeMap::new();
        let mut claim_use_counts = BTreeMap::new();
        let mut privacy_decision_counts = BTreeMap::new();
        let mut claim_counts_by_card: BTreeMap<String, usize> = BTreeMap::new();
        let mut findings = Vec::new();

        for claim in &claims {
            *claim_counts_by_card
                .entry(claim.evidence_card_id.clone())
                .or_insert(0) += 1;
            *counts_by_proof_level
                .entry(claim.proof_level.clone())
                .or_insert(0) += 1;
            if claim.can_use_in_resume {
                *claim_use_counts.entry("resume".to_string()).or_insert(0) += 1;
            }
            if claim.can_use_in_outreach {
                *claim_use_counts.entry("outreach".to_string()).or_insert(0) += 1;
            }
            if claim.can_use_in_interview {
                *claim_use_counts.entry("interview".to_string()).or_insert(0) += 1;
            }
            if claim.proof_level == "unverified"
                && (claim.can_use_in_resume || claim.can_use_in_outreach)
            {
                findings.push(JobEvidenceReviewFinding {
                    severity: "warn".to_string(),
                    finding_type: "unverified_public_claim".to_string(),
                    evidence_card_id: Some(claim.evidence_card_id.clone()),
                    claim_id: Some(claim.id.clone()),
                    message: "A resume/outreach-usable claim is still marked unverified."
                        .to_string(),
                    next_action: "Verify the claim or mark it interview-only before using it in application material."
                        .to_string(),
                });
            }
            if claim.proof_level == "private"
                && (claim.can_use_in_resume || claim.can_use_in_outreach)
            {
                findings.push(JobEvidenceReviewFinding {
                    severity: "block".to_string(),
                    finding_type: "private_public_claim".to_string(),
                    evidence_card_id: Some(claim.evidence_card_id.clone()),
                    claim_id: Some(claim.id.clone()),
                    message:
                        "A private-only claim is marked usable in resume or outreach material."
                            .to_string(),
                    next_action:
                        "Remove public-use flags or replace the claim with public-safe evidence."
                            .to_string(),
                });
            }
        }

        let mut ready_card_ids = Vec::new();
        let mut needs_review_card_ids = BTreeSet::new();
        let mut blocked_card_ids = BTreeSet::new();

        for card in &cards {
            *counts_by_visibility
                .entry(card.visibility.clone())
                .or_insert(0) += 1;
            *counts_by_evidence_type
                .entry(card.evidence_type.clone())
                .or_insert(0) += 1;
            *counts_by_confidence
                .entry(card.confidence.clone())
                .or_insert(0) += 1;

            let mut card_has_block = false;
            let mut card_has_warning = false;
            let privacy_findings =
                self.evaluate_job_privacy_text(&card.safe_application_text, &card.unsafe_terms)?;
            let privacy_decision = job_privacy_decision(&privacy_findings);
            *privacy_decision_counts
                .entry(privacy_decision.clone())
                .or_insert(0) += 1;
            if privacy_decision != "pass" {
                let severity = if privacy_decision == "block" {
                    card_has_block = true;
                    "block"
                } else {
                    card_has_warning = true;
                    "warn"
                };
                findings.push(JobEvidenceReviewFinding {
                    severity: severity.to_string(),
                    finding_type: "safe_text_privacy".to_string(),
                    evidence_card_id: Some(card.id.clone()),
                    claim_id: None,
                    message: "Application-safe text for this evidence card triggers privacy rules."
                        .to_string(),
                    next_action: "Rewrite the safe application text without private names, secrets, local paths, or blocked terms."
                        .to_string(),
                });
            }

            if card.visibility == "needs_review" {
                card_has_warning = true;
                findings.push(JobEvidenceReviewFinding {
                    severity: "warn".to_string(),
                    finding_type: "visibility_needs_review".to_string(),
                    evidence_card_id: Some(card.id.clone()),
                    claim_id: None,
                    message: "Evidence visibility is still needs_review.".to_string(),
                    next_action: "Classify the card as public, private_safe, or private_blocked before using it."
                        .to_string(),
                });
            } else if card.visibility == "private_blocked" {
                card_has_warning = true;
                findings.push(JobEvidenceReviewFinding {
                    severity: "warn".to_string(),
                    finding_type: "private_blocked_evidence".to_string(),
                    evidence_card_id: Some(card.id.clone()),
                    claim_id: None,
                    message: "Private-blocked evidence exists in the profile ledger.".to_string(),
                    next_action:
                        "Keep it out of resume, outreach, packet, and public proof material."
                            .to_string(),
                });
            }

            if card.confidence == "stale" {
                card_has_warning = true;
                findings.push(JobEvidenceReviewFinding {
                    severity: "warn".to_string(),
                    finding_type: "stale_evidence".to_string(),
                    evidence_card_id: Some(card.id.clone()),
                    claim_id: None,
                    message: "Evidence confidence is stale.".to_string(),
                    next_action:
                        "Refresh or replace this evidence before relying on it for role-fit claims."
                            .to_string(),
                });
            }

            if card.visibility == "public" && card.proof_url.is_none() && card.local_path.is_some()
            {
                card_has_block = true;
                findings.push(JobEvidenceReviewFinding {
                    severity: "block".to_string(),
                    finding_type: "public_local_only_proof".to_string(),
                    evidence_card_id: Some(card.id.clone()),
                    claim_id: None,
                    message: "A public evidence card relies only on a local path.".to_string(),
                    next_action:
                        "Add a public proof URL, change visibility, or keep this card out of public application material."
                            .to_string(),
                });
            }

            if card.visibility == "public"
                && card.proof_url.is_none()
                && card.local_path.is_none()
                && !matches!(card.evidence_type.as_str(), "resume" | "private_safe")
            {
                card_has_warning = true;
                findings.push(JobEvidenceReviewFinding {
                    severity: "warn".to_string(),
                    finding_type: "public_missing_proof_url".to_string(),
                    evidence_card_id: Some(card.id.clone()),
                    claim_id: None,
                    message: "A public evidence card has no public proof URL.".to_string(),
                    next_action: "Attach a public proof URL or downgrade the evidence confidence."
                        .to_string(),
                });
            }

            if job_text_looks_local_reference(&card.safe_application_text) {
                card_has_block = true;
                findings.push(JobEvidenceReviewFinding {
                    severity: "block".to_string(),
                    finding_type: "safe_text_local_reference".to_string(),
                    evidence_card_id: Some(card.id.clone()),
                    claim_id: None,
                    message: "Application-safe text contains a local file/path reference."
                        .to_string(),
                    next_action:
                        "Rewrite the safe application text so it can stand alone publicly."
                            .to_string(),
                });
            }

            if !claim_counts_by_card.contains_key(&card.id) {
                card_has_warning = true;
                findings.push(JobEvidenceReviewFinding {
                    severity: "warn".to_string(),
                    finding_type: "evidence_without_claims".to_string(),
                    evidence_card_id: Some(card.id.clone()),
                    claim_id: None,
                    message: "Evidence card is not mapped to any explicit claim.".to_string(),
                    next_action: "Add evidence claims that explain what this card can support."
                        .to_string(),
                });
            }

            if card_has_block {
                blocked_card_ids.insert(card.id.clone());
            } else if card_has_warning {
                needs_review_card_ids.insert(card.id.clone());
            } else if matches!(card.visibility.as_str(), "public" | "private_safe") {
                ready_card_ids.push(card.id.clone());
            }
        }

        if cards.len() < 20 {
            findings.push(JobEvidenceReviewFinding {
                severity: "warn".to_string(),
                finding_type: "thin_evidence_set".to_string(),
                evidence_card_id: None,
                claim_id: None,
                message: "Profile has fewer than 20 reviewed evidence cards.".to_string(),
                next_action: "Import and review the current resume, GitHub, blog, project, and private-safe work evidence before relying on the ledger."
                    .to_string(),
            });
        }
        if claims.is_empty() {
            findings.push(JobEvidenceReviewFinding {
                severity: "warn".to_string(),
                finding_type: "no_evidence_claims".to_string(),
                evidence_card_id: None,
                claim_id: None,
                message: "Profile has no explicit evidence claims.".to_string(),
                next_action:
                    "Add claims that connect evidence cards to resume, outreach, and interview use."
                        .to_string(),
            });
        }
        if !claim_use_counts.contains_key("resume") && !claim_use_counts.contains_key("outreach") {
            findings.push(JobEvidenceReviewFinding {
                severity: "warn".to_string(),
                finding_type: "no_public_application_claims".to_string(),
                evidence_card_id: None,
                claim_id: None,
                message: "No claims are currently marked usable for resume or outreach material."
                    .to_string(),
                next_action: "Mark only verified public-safe claims as resume/outreach usable."
                    .to_string(),
            });
        }

        let decision = if findings.iter().any(|finding| finding.severity == "block") {
            "block"
        } else if findings.iter().any(|finding| finding.severity == "warn") {
            "warn"
        } else {
            "pass"
        }
        .to_string();

        Ok(JobEvidenceReviewReport {
            profile_id: profile_id.to_string(),
            generated_at: now(),
            decision,
            evidence_card_count: cards.len(),
            claim_count: claims.len(),
            counts_by_visibility,
            counts_by_evidence_type,
            counts_by_confidence,
            counts_by_proof_level,
            claim_use_counts,
            privacy_decision_counts,
            ready_card_ids,
            needs_review_card_ids: needs_review_card_ids.into_iter().collect(),
            blocked_card_ids: blocked_card_ids.into_iter().collect(),
            findings,
        })
    }

    pub fn record_job_privacy_rule(&self, input: JobPrivacyRuleInput) -> Result<JobPrivacyRule> {
        let input = normalize_job_privacy_rule_input(input)?;
        let id = job_privacy_rule_id(&input.pattern, &input.rule_type);
        self.conn.execute(
            r#"
            INSERT INTO job_privacy_rules
              (id, pattern, rule_type, severity, replacement_guidance, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(pattern, rule_type) DO UPDATE SET
              severity = excluded.severity,
              replacement_guidance = excluded.replacement_guidance
            "#,
            params![
                id,
                input.pattern,
                input.rule_type,
                input.severity,
                input.replacement_guidance,
                now(),
            ],
        )?;
        self.read_job_privacy_rule(&id)?
            .with_context(|| format!("job privacy rule not found: {id}"))
    }

    pub fn read_job_privacy_rule(&self, id: &str) -> Result<Option<JobPrivacyRule>> {
        validate_id(id)?;
        self.conn
            .query_row(
                r#"
                SELECT id, pattern, rule_type, severity, replacement_guidance, created_at
                FROM job_privacy_rules
                WHERE id = ?1
                "#,
                params![id],
                job_privacy_rule_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_job_privacy_rules(&self) -> Result<Vec<JobPrivacyRule>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, pattern, rule_type, severity, replacement_guidance, created_at
            FROM job_privacy_rules
            ORDER BY created_at ASC
            "#,
        )?;
        rows(stmt.query_map([], job_privacy_rule_from_row)?)
    }

    pub fn check_job_privacy_text(
        &self,
        artifact_type: &str,
        artifact_id: Option<&str>,
        text: &str,
        extra_blocked_terms: &[String],
    ) -> Result<JobPrivacyCheck> {
        let findings = self.evaluate_job_privacy_text(text, extra_blocked_terms)?;
        let decision = job_privacy_decision(&findings);
        self.record_job_privacy_check_result(artifact_type, artifact_id, &decision, findings, text)
    }
}
