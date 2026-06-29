use super::*;

#[test]
fn severe_job_privacy_blocks_private_terms_in_packet() {
    // CLAIM: application material cannot pass if it contains blocked
    // private project names, even when the role/evidence shape is valid.
    // ORACLE: privacy scan records a block decision and packet creation
    // refuses to write a draft that contains the blocked term.
    // SEVERITY: Severe because public job material is exactly where a
    // leaked private name would cause real harm.
    let store = test_store("job-privacy-blocks-private-terms");
    let profile = job_fixture_profile(&store);
    let evidence = job_fixture_evidence(&store, &profile.id);
    let role = job_fixture_role(&store, &evidence.id, "canonical_confirmed");
    store
        .record_job_privacy_rule(JobPrivacyRuleInput {
            pattern: "secret-project".to_string(),
            rule_type: "blocked_term".to_string(),
            severity: "block".to_string(),
            replacement_guidance: Some("Use public project phrasing instead.".to_string()),
        })
        .unwrap();

    let check = store
        .check_job_privacy_text(
            "outreach",
            Some(&role.id),
            "I can talk about secret-project in detail.",
            &[],
        )
        .unwrap();
    assert_eq!(check.decision, "block");
    assert_eq!(check.findings.len(), 1);

    let result = store.create_job_application_packet(JobApplicationPacketInput {
        role_id: role.id,
        profile_id: profile.id,
        evidence_card_ids: vec![evidence.id],
        resume_emphasis: "Lead with public developer tooling and agent systems.".to_string(),
        tailored_bullets: vec!["Built public developer tooling for cloud workflows.".to_string()],
        outreach_note: "Example AI looks relevant because secret-project maps to this role."
            .to_string(),
        proof_links: json!(["https://github.com/chrischabot/opencloud"]),
        likely_objections: vec!["No direct company context yet.".to_string()],
        interview_stories: vec!["Public agent-tooling project story.".to_string()],
        questions_to_ask: vec!["How do agent systems fail in production today?".to_string()],
        reviewer_note: None,
    });
    assert!(result.is_err());
}

#[test]
fn severe_job_evidence_review_report_passes_only_reviewed_claim_mapped_safe_evidence() {
    // CLAIM: an evidence ledger is ready for application work only when it
    // has enough reviewed, public-safe cards with explicit usable claims.
    // ORACLE: twenty public verified cards with public claims produce a
    // pass report with ready card ids and no findings.
    // SEVERITY: Severe because "evidence exists" is otherwise easy to
    // mistake for "evidence can safely support applications."
    let store = test_store("job-evidence-review-pass");
    let profile = job_fixture_profile(&store);
    for index in 0..20 {
        job_fixture_reviewed_evidence_with_claim(&store, &profile.id, index);
    }

    let report = store
        .compile_job_evidence_review_report(&profile.id)
        .unwrap();
    assert_eq!(report.decision, "pass");
    assert_eq!(report.evidence_card_count, 20);
    assert_eq!(report.claim_count, 20);
    assert_eq!(report.ready_card_ids.len(), 20);
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    assert_eq!(
        report.privacy_decision_counts.get("pass").copied(),
        Some(20)
    );
    assert_eq!(report.claim_use_counts.get("resume").copied(), Some(20));
    assert_eq!(
        report.counts_by_proof_level.get("public").copied(),
        Some(20)
    );
}

#[test]
fn severe_job_evidence_review_report_blocks_public_local_and_private_term_mirages() {
    // CLAIM: the evidence review report catches unsafe public-use evidence
    // before packets or shortlist prose can inherit it.
    // ORACLE: local-only public proof and blocked terms in safe text produce
    // block findings; thin/unmapped/needs-review evidence stays visible.
    // SEVERITY: Severe because the next production-data step imports real
    // resume/GitHub/blog evidence, where false public readiness would leak
    // private names or unsupported claims.
    let store = test_store("job-evidence-review-blocks");
    let profile = job_fixture_profile(&store);
    store
        .record_job_privacy_rule(JobPrivacyRuleInput {
            pattern: "secret-project".to_string(),
            rule_type: "blocked_term".to_string(),
            severity: "block".to_string(),
            replacement_guidance: Some("Use public phrasing.".to_string()),
        })
        .unwrap();

    let local_public = store
        .record_job_evidence_card(JobEvidenceCardInput {
            profile_id: profile.id.clone(),
            title: "Local-only public proof".to_string(),
            evidence_type: "work".to_string(),
            visibility: "public".to_string(),
            summary: "This should not be public-proof ready.".to_string(),
            proof_url: None,
            local_path: Some("/Users/chabotc/private/proof.md".to_string()),
            source_date: Some("2026-06-28".to_string()),
            confidence: "verified".to_string(),
            tags: vec!["agents".to_string()],
            safe_application_text: "Built a public agent workflow.".to_string(),
            unsafe_terms: vec![],
            metadata: json!({}),
        })
        .unwrap();
    store
        .record_job_evidence_claim(JobEvidenceClaimInput {
            evidence_card_id: local_public.id.clone(),
            claim: "Can support a public work claim.".to_string(),
            claim_kind: "work".to_string(),
            proof_level: "public".to_string(),
            can_use_in_resume: true,
            can_use_in_outreach: true,
            can_use_in_interview: true,
        })
        .unwrap();

    let private_text = store
        .record_job_evidence_card(JobEvidenceCardInput {
            profile_id: profile.id.clone(),
            title: "Unsafe safe text".to_string(),
            evidence_type: "work".to_string(),
            visibility: "private_safe".to_string(),
            summary: "The summary is not the public application text.".to_string(),
            proof_url: None,
            local_path: None,
            source_date: None,
            confidence: "user_claimed".to_string(),
            tags: vec!["agents".to_string()],
            safe_application_text: "Talked about secret-project with customers.".to_string(),
            unsafe_terms: vec![],
            metadata: json!({}),
        })
        .unwrap();
    store
        .record_job_evidence_claim(JobEvidenceClaimInput {
            evidence_card_id: private_text.id.clone(),
            claim: "Can support a private-safe interview claim.".to_string(),
            claim_kind: "work".to_string(),
            proof_level: "private_safe".to_string(),
            can_use_in_resume: false,
            can_use_in_outreach: false,
            can_use_in_interview: true,
        })
        .unwrap();

    let needs_review = store
        .record_job_evidence_card(JobEvidenceCardInput {
            profile_id: profile.id.clone(),
            title: "Unmapped draft evidence".to_string(),
            evidence_type: "blog".to_string(),
            visibility: "needs_review".to_string(),
            summary: "Not yet reviewed.".to_string(),
            proof_url: Some("https://example.com/blog/draft".to_string()),
            local_path: None,
            source_date: None,
            confidence: "inferred".to_string(),
            tags: vec!["writing".to_string()],
            safe_application_text: "Draft writing evidence.".to_string(),
            unsafe_terms: vec![],
            metadata: json!({}),
        })
        .unwrap();

    let report = store
        .compile_job_evidence_review_report(&profile.id)
        .unwrap();
    assert_eq!(report.decision, "block");
    assert_eq!(report.evidence_card_count, 3);
    assert!(report.blocked_card_ids.contains(&local_public.id));
    assert!(report.blocked_card_ids.contains(&private_text.id));
    assert!(report.needs_review_card_ids.contains(&needs_review.id));
    for finding_type in [
        "public_local_only_proof",
        "safe_text_privacy",
        "visibility_needs_review",
        "evidence_without_claims",
        "thin_evidence_set",
    ] {
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.finding_type == finding_type),
            "missing finding type {finding_type}: {:?}",
            report.findings
        );
    }
}

#[test]
fn severe_job_source_confidence_demotes_aggregator_only_listing() {
    // CLAIM: job source confidence gates fit tiering.
    // ORACLE: a high-scoring aggregator-only listing is blocked from
    // apply-now tier, while an equivalent secondary source caps at Tier 2.
    // SEVERITY: Severe because stale job-board snippets are an easy way to
    // produce impressive but unusable shortlists.
    let store = test_store("job-source-confidence-demotion");
    let profile = job_fixture_profile(&store);
    let evidence = job_fixture_evidence(&store, &profile.id);
    let aggregator = job_fixture_role(&store, &evidence.id, "aggregator_only");
    let aggregator_score = store
        .record_job_fit_score(job_fixture_score_input(
            &aggregator.id,
            &profile.id,
            &evidence.id,
        ))
        .unwrap();
    assert_eq!(aggregator_score.tier, "blocked");
    assert!(
        aggregator_score
            .blockers
            .iter()
            .any(|blocker| blocker.contains("aggregator-only"))
    );

    let secondary = store
        .record_job_role_card(JobRoleCardInput {
            company: "Example AI Secondary".to_string(),
            role_title: "Staff Agent Platform Engineer".to_string(),
            canonical_url: None,
            source_family: "vc_board".to_string(),
            source_url: "https://vc.example/jobs/example-ai-secondary".to_string(),
            source_confidence: "secondary_confirmed".to_string(),
            date_accessed: Some("2026-06-28T10:00:00Z".to_string()),
            posting_freshness: "same_day".to_string(),
            location: Some("London".to_string()),
            work_mode: Some("hybrid".to_string()),
            company_stage_or_size: Some("startup".to_string()),
            role_seniority: Some("staff".to_string()),
            core_requirements: vec!["agent systems".to_string()],
            implied_business_problem: None,
            why_they_might_need_user: None,
            evidence_card_ids: vec![evidence.id.clone()],
            gaps_or_blockers: vec![],
            cluster: None,
            current_status: "live".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    let secondary_score = store
        .record_job_fit_score(job_fixture_score_input(
            &secondary.id,
            &profile.id,
            &evidence.id,
        ))
        .unwrap();
    assert_eq!(secondary_score.tier, "tier_2");
}

#[test]
fn severe_job_score_requires_evidence_and_hard_blockers_win() {
    // CLAIM: numeric scoring is auditable and cannot turn title matching
    // into a Tier 1 recommendation.
    // ORACLE: high evidence_fit without evidence links fails; a hard
    // blocker wins over otherwise excellent dimensions.
    // SEVERITY: Severe because prose-only fit notes are the central mirage
    // this subsystem is meant to prevent.
    let store = test_store("job-score-evidence-and-blockers");
    let profile = job_fixture_profile(&store);
    let evidence = job_fixture_evidence(&store, &profile.id);
    let role = job_fixture_role(&store, &evidence.id, "canonical_confirmed");

    let mut no_evidence = job_fixture_score_input(&role.id, &profile.id, &evidence.id);
    no_evidence.evidence_card_ids = vec![];
    assert!(store.record_job_fit_score(no_evidence).is_err());

    let mut blocked = job_fixture_score_input(&role.id, &profile.id, &evidence.id);
    blocked.blockers = vec!["Location requires weekly San Francisco presence.".to_string()];
    let score = store.record_job_fit_score(blocked).unwrap();
    assert_eq!(score.tier, "blocked");
    assert!(score.weighted_score > 90.0);
}

#[test]
fn severe_job_packet_rejects_private_evidence_and_local_proof_links() {
    // CLAIM: packets are application-safe artifacts, not polished wrappers
    // around private evidence or local filesystem proof.
    // ORACLE: private-blocked evidence and local proof links both fail
    // before a packet row is written.
    // SEVERITY: Severe because leaking private or local-only evidence in
    // outreach is a direct public-shelf failure.
    let store = test_store("job-packet-private-evidence-local-links");
    let profile = job_fixture_profile(&store);
    let evidence = job_fixture_evidence(&store, &profile.id);
    let private = store
        .record_job_evidence_card(JobEvidenceCardInput {
            profile_id: profile.id.clone(),
            title: "Private unreleased system".to_string(),
            evidence_type: "work".to_string(),
            visibility: "private_blocked".to_string(),
            summary: "Private system that cannot be named.".to_string(),
            proof_url: None,
            local_path: Some("/Users/chabotc/private/notes.md".to_string()),
            source_date: None,
            confidence: "user_claimed".to_string(),
            tags: vec!["agents".to_string()],
            safe_application_text: "Do not use in applications.".to_string(),
            unsafe_terms: vec!["private-system".to_string()],
            metadata: json!({}),
        })
        .unwrap();
    let role = job_fixture_role(&store, &evidence.id, "canonical_confirmed");

    let private_result = store.create_job_application_packet(JobApplicationPacketInput {
        role_id: role.id.clone(),
        profile_id: profile.id.clone(),
        evidence_card_ids: vec![private.id],
        resume_emphasis: "Lead with public project work.".to_string(),
        tailored_bullets: vec!["Public developer-tooling project.".to_string()],
        outreach_note: "Example AI is relevant to public agent tooling.".to_string(),
        proof_links: json!(["https://github.com/chrischabot/opencloud"]),
        likely_objections: vec![],
        interview_stories: vec!["Public project story.".to_string()],
        questions_to_ask: vec!["What should this platform make safer?".to_string()],
        reviewer_note: None,
    });
    assert!(private_result.is_err());

    let local_result = store.create_job_application_packet(JobApplicationPacketInput {
        role_id: role.id,
        profile_id: profile.id,
        evidence_card_ids: vec![evidence.id],
        resume_emphasis: "Lead with public project work.".to_string(),
        tailored_bullets: vec!["Public developer-tooling project.".to_string()],
        outreach_note: "Example AI is relevant to public agent tooling.".to_string(),
        proof_links: json!(["/Users/chabotc/private/demo.mov"]),
        likely_objections: vec![],
        interview_stories: vec!["Public project story.".to_string()],
        questions_to_ask: vec!["What should this platform make safer?".to_string()],
        reviewer_note: None,
    });
    assert!(local_result.is_err());
}

#[test]
fn severe_job_application_requires_approved_packet_for_applied_status() {
    // CLAIM: draft packets are not application-sent artifacts.
    // ORACLE: an applied application record with a draft packet fails; the
    // same record succeeds only after explicit packet approval with a
    // reviewer note.
    // SEVERITY: Severe because otherwise "draft packet exists" can become
    // a false claim that application material was reviewed and used.
    let store = test_store("job-application-packet-approval");
    let profile = job_fixture_profile(&store);
    let evidence = job_fixture_evidence(&store, &profile.id);
    let role = job_fixture_role(&store, &evidence.id, "canonical_confirmed");
    let packet = store
        .create_job_application_packet(JobApplicationPacketInput {
            role_id: role.id.clone(),
            profile_id: profile.id.clone(),
            evidence_card_ids: vec![evidence.id.clone()],
            resume_emphasis: "Lead with public developer tooling and agents.".to_string(),
            tailored_bullets: vec!["Built public cloud developer tooling.".to_string()],
            outreach_note: "Example AI appears to need agent tooling discipline.".to_string(),
            proof_links: json!(["https://github.com/chrischabot/opencloud"]),
            likely_objections: vec!["No direct company-specific evidence yet.".to_string()],
            interview_stories: vec!["Public project technical story.".to_string()],
            questions_to_ask: vec!["Where do agent workflows fail today?".to_string()],
            reviewer_note: None,
        })
        .unwrap();
    assert_eq!(packet.status, "draft");

    let draft_application = store.record_job_application(JobApplicationInput {
        role_id: role.id.clone(),
        packet_id: Some(packet.id.clone()),
        status: "applied".to_string(),
        applied_at: Some("2026-06-28".to_string()),
        follow_up_at: None,
        outcome_note: None,
    });
    assert!(
        draft_application
            .unwrap_err()
            .to_string()
            .contains("requires an approved application packet")
    );

    let missing_note =
        store.update_job_application_packet_status(JobApplicationPacketStatusInput {
            packet_id: packet.id.clone(),
            status: "approved".to_string(),
            reviewer_note: None,
        });
    assert!(
        missing_note
            .unwrap_err()
            .to_string()
            .contains("requires reviewer_note")
    );

    let approved = store
        .update_job_application_packet_status(JobApplicationPacketStatusInput {
            packet_id: packet.id.clone(),
            status: "approved".to_string(),
            reviewer_note: Some("Reviewed by user for this application.".to_string()),
        })
        .unwrap();
    assert_eq!(approved.status, "approved");
    assert_eq!(
        approved.reviewer_note.as_deref(),
        Some("Reviewed by user for this application.")
    );

    let application = store
        .record_job_application(JobApplicationInput {
            role_id: role.id,
            packet_id: Some(packet.id),
            status: "applied".to_string(),
            applied_at: Some("2026-06-28".to_string()),
            follow_up_at: None,
            outcome_note: None,
        })
        .unwrap();
    assert_eq!(application.status, "applied");
}

#[test]
fn severe_job_packet_export_requires_approved_packet_without_recording_application() {
    // CLAIM: local packet export turns a reviewed packet into an
    // inspectable Markdown file without pretending the application was
    // submitted.
    // ORACLE: draft export fails; approved export writes Markdown with the
    // not-sent boundary and leaves the applications table empty.
    // SEVERITY: Severe because a file export is easy to confuse with an
    // applied/sent job application if the boundary is not enforced.
    let store = test_store("job-packet-export-approved-local-only");
    let profile = job_fixture_profile(&store);
    let evidence = job_fixture_evidence(&store, &profile.id);
    let role = job_fixture_role(&store, &evidence.id, "canonical_confirmed");
    let packet = store
        .create_job_application_packet(JobApplicationPacketInput {
            role_id: role.id.clone(),
            profile_id: profile.id.clone(),
            evidence_card_ids: vec![evidence.id.clone()],
            resume_emphasis: "Lead with public developer tooling and agents.".to_string(),
            tailored_bullets: vec!["Built public cloud developer tooling.".to_string()],
            outreach_note: "Example AI appears to need agent tooling discipline.".to_string(),
            proof_links: json!(["https://github.com/chrischabot/opencloud"]),
            likely_objections: vec!["No direct company-specific evidence yet.".to_string()],
            interview_stories: vec!["Public project technical story.".to_string()],
            questions_to_ask: vec!["Where do agent workflows fail today?".to_string()],
            reviewer_note: None,
        })
        .unwrap();

    let out_dir =
        std::env::temp_dir().join(format!("arcwell-job-packet-export-{}", Uuid::new_v4()));
    let draft_export = store.export_job_application_packet(&packet.id, &out_dir);
    assert!(
        draft_export
            .unwrap_err()
            .to_string()
            .contains("requires approved status")
    );
    assert!(!out_dir.exists());

    let approved = store
        .update_job_application_packet_status(JobApplicationPacketStatusInput {
            packet_id: packet.id.clone(),
            status: "approved".to_string(),
            reviewer_note: Some("Reviewed by user for local export.".to_string()),
        })
        .unwrap();
    let export = store
        .export_job_application_packet(&approved.id, &out_dir)
        .unwrap();
    assert_eq!(export.proof_level, "local_proof");
    assert_eq!(export.delivery_status, "not_sent");
    assert!(!export.application_status_changed);
    assert!(export.byte_len > 0);
    assert_eq!(export.sha256.len(), 64);
    assert!(
        export
            .warnings
            .iter()
            .any(|warning| warning.contains("no application was sent or recorded"))
    );

    let body = fs::read_to_string(&export.path).unwrap();
    assert!(body.contains("Delivery status: not_sent"), "{body}");
    assert!(
        body.contains("not proof that an application was sent"),
        "{body}"
    );
    assert!(body.contains("Example AI"), "{body}");
    assert!(body.contains("Open Cloud"), "{body}");
    assert!(store.list_job_applications().unwrap().is_empty());
}

#[test]
fn severe_job_packet_export_rechecks_privacy_before_writing_file() {
    // CLAIM: approval is not a stale privacy bypass; export rechecks the
    // exact Markdown artifact before it reaches disk.
    // ORACLE: a privacy rule added after approval blocks export-only text,
    // records no file, and keeps applications empty.
    // SEVERITY: Severe because privacy policy can change between packet
    // drafting and user-visible export.
    let store = test_store("job-packet-export-rechecks-privacy");
    let profile = job_fixture_profile(&store);
    let evidence = job_fixture_evidence(&store, &profile.id);
    let role = job_fixture_role(&store, &evidence.id, "canonical_confirmed");
    let packet = store
        .create_job_application_packet(JobApplicationPacketInput {
            role_id: role.id,
            profile_id: profile.id,
            evidence_card_ids: vec![evidence.id],
            resume_emphasis: "Lead with public developer tooling and agents.".to_string(),
            tailored_bullets: vec!["Built public cloud developer tooling.".to_string()],
            outreach_note: "Example AI appears to need agent tooling discipline.".to_string(),
            proof_links: json!(["https://github.com/chrischabot/opencloud"]),
            likely_objections: vec!["No direct company-specific evidence yet.".to_string()],
            interview_stories: vec!["Public project technical story.".to_string()],
            questions_to_ask: vec!["Where do agent workflows fail today?".to_string()],
            reviewer_note: None,
        })
        .unwrap();
    let approved = store
        .update_job_application_packet_status(JobApplicationPacketStatusInput {
            packet_id: packet.id,
            status: "approved".to_string(),
            reviewer_note: Some("Reviewed before export privacy policy changed.".to_string()),
        })
        .unwrap();
    store
        .record_job_privacy_rule(JobPrivacyRuleInput {
            pattern: "Delivery status".to_string(),
            rule_type: "blocked_term".to_string(),
            severity: "block".to_string(),
            replacement_guidance: Some(
                "Do not export artifacts matching the current policy.".to_string(),
            ),
        })
        .unwrap();

    let out_dir = std::env::temp_dir().join(format!(
        "arcwell-job-packet-export-blocked-{}",
        Uuid::new_v4()
    ));
    let export = store.export_job_application_packet(&approved.id, &out_dir);
    assert!(
        export
            .unwrap_err()
            .to_string()
            .contains("export failed privacy check with decision block")
    );
    assert!(!out_dir.exists());
    assert!(store.list_job_applications().unwrap().is_empty());
}

#[test]
fn severe_job_intro_public_profile_is_not_warm_intro() {
    // CLAIM: public contact discovery is not mislabeled as a warm intro.
    // ORACLE: a public-only contact cannot become a confirmed mutual path;
    // it can only remain an identify-stage weak path.
    // SEVERITY: Severe because false warm-intro claims distort application
    // prioritization and user trust.
    let store = test_store("job-intro-public-profile-not-warm");
    let profile = job_fixture_profile(&store);
    let evidence = job_fixture_evidence(&store, &profile.id);
    let role = job_fixture_role(&store, &evidence.id, "canonical_confirmed");
    let contact = store
        .record_job_contact(JobContactInput {
            name: "Hiring Manager".to_string(),
            company_id: None,
            role_title: Some("Engineering Manager".to_string()),
            public_profile_url: "https://example.com/people/hiring-manager".to_string(),
            source_url: "https://example.com/team".to_string(),
            relationship_status: "public_only".to_string(),
            relevance: "hiring_manager".to_string(),
            note: Some("Public profile only; no relationship path.".to_string()),
        })
        .unwrap();

    assert!(
        store
            .record_job_intro_path(JobIntroPathInput {
                role_id: role.id.clone(),
                contact_id: contact.id.clone(),
                path_type: "mutual".to_string(),
                confidence: "confirmed".to_string(),
                next_action: Some("Ask for intro.".to_string()),
                status: "ask".to_string(),
            })
            .is_err()
    );

    let weak = store
        .record_job_intro_path(JobIntroPathInput {
            role_id: role.id,
            contact_id: contact.id,
            path_type: "unknown".to_string(),
            confidence: "weak".to_string(),
            next_action: Some("Look for real mutual path.".to_string()),
            status: "identify".to_string(),
        })
        .unwrap();
    assert_eq!(weak.status, "identify");
}

#[test]
fn severe_job_weekly_report_preserves_application_status_and_source_health() {
    // CLAIM: weekly reporting is driven by durable role/application/source
    // state, not a fresh prose snapshot that forgets failures.
    // ORACLE: an applied role and failed source health both appear in the
    // persisted report body and metadata.
    // SEVERITY: Severe because scheduled-looking reports are misleading if
    // they hide stale sources or application state.
    let store = test_store("job-weekly-state-report");
    let profile = job_fixture_profile(&store);
    let evidence = job_fixture_evidence(&store, &profile.id);
    let role = job_fixture_role(&store, &evidence.id, "canonical_confirmed");
    let packet = store
        .create_job_application_packet(JobApplicationPacketInput {
            role_id: role.id.clone(),
            profile_id: profile.id.clone(),
            evidence_card_ids: vec![evidence.id.clone()],
            resume_emphasis: "Lead with public developer tooling and agents.".to_string(),
            tailored_bullets: vec!["Built public cloud developer tooling.".to_string()],
            outreach_note: "Example AI appears to need agent tooling discipline.".to_string(),
            proof_links: json!(["https://github.com/chrischabot/opencloud"]),
            likely_objections: vec!["No direct company-specific evidence yet.".to_string()],
            interview_stories: vec!["Public project technical story.".to_string()],
            questions_to_ask: vec!["Where do agent workflows fail today?".to_string()],
            reviewer_note: None,
        })
        .unwrap();
    store
        .record_job_fit_score(job_fixture_score_input(&role.id, &profile.id, &evidence.id))
        .unwrap();
    let packet = store
        .update_job_application_packet_status(JobApplicationPacketStatusInput {
            packet_id: packet.id,
            status: "approved".to_string(),
            reviewer_note: Some("Reviewed by user for weekly-report fixture.".to_string()),
        })
        .unwrap();
    store
        .record_job_application(JobApplicationInput {
            role_id: role.id,
            packet_id: Some(packet.id),
            status: "applied".to_string(),
            applied_at: Some("2026-06-28".to_string()),
            follow_up_at: Some("2026-07-05".to_string()),
            outcome_note: Some("Submitted manually by user.".to_string()),
        })
        .unwrap();
    let source = store
        .record_job_source(JobSourceInput {
            source_family: "company".to_string(),
            name: "Example AI careers".to_string(),
            url: "https://example.com/careers".to_string(),
            market_scope: "london".to_string(),
            refresh_policy: "manual".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    store
        .record_job_source_health(JobSourceHealthInput {
            source_id: source.id,
            status: "failed".to_string(),
            http_status: Some(503),
            error_code: Some("service_unavailable".to_string()),
            fetched_count: 0,
            accepted_count: 0,
            rejected_count: 0,
            note: Some("Careers page unavailable during refresh.".to_string()),
        })
        .unwrap();

    let report = store
        .compile_job_weekly_report(&profile.id, "London agent platform roles")
        .unwrap();
    assert!(report.body.contains("applied: 1"), "{}", report.body);
    assert!(report.body.contains("failed: 1"), "{}", report.body);
    assert_eq!(report.metadata["application_count"], 1);
    assert_eq!(report.metadata["source_health_count"], 1);
}

#[test]
fn severe_job_outcome_history_adds_notes_without_tier_fabrication() {
    // CLAIM: application outcomes inform future role review as explicit
    // notes, not as hidden causal scoring rules.
    // ORACLE: a prior rejection at the same company appears in the
    // shortlist and weekly report outcome notes for a new role, while the
    // new role's evidence/source-based Tier 1 score remains unchanged.
    // SEVERITY: Severe because one anecdotal outcome can otherwise become
    // fake conversion intelligence or silently suppress a good target.
    let store = test_store("job-outcome-history-notes");
    let profile = job_fixture_profile(&store);
    let evidence = job_fixture_evidence(&store, &profile.id);
    let prior_role = store
        .record_job_role_card(JobRoleCardInput {
            company: "Example AI".to_string(),
            role_title: "Senior Backend Engineer".to_string(),
            canonical_url: Some("https://example.com/careers/senior-backend".to_string()),
            source_family: "company".to_string(),
            source_url: "https://example.com/careers/senior-backend".to_string(),
            source_confidence: "canonical_confirmed".to_string(),
            date_accessed: Some("2026-06-20T10:00:00Z".to_string()),
            posting_freshness: "same_day".to_string(),
            location: Some("London".to_string()),
            work_mode: Some("hybrid".to_string()),
            company_stage_or_size: Some("startup".to_string()),
            role_seniority: Some("senior".to_string()),
            core_requirements: vec!["backend systems".to_string()],
            implied_business_problem: Some("Scale backend services.".to_string()),
            why_they_might_need_user: Some("Developer-platform experience.".to_string()),
            evidence_card_ids: vec![evidence.id.clone()],
            gaps_or_blockers: vec![],
            cluster: Some("backend-platform".to_string()),
            current_status: "live".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    store
        .record_job_application(JobApplicationInput {
            role_id: prior_role.id,
            packet_id: None,
            status: "rejected".to_string(),
            applied_at: Some("2026-06-21".to_string()),
            follow_up_at: None,
            outcome_note: Some(
                "Recruiter wanted more recent backend-specific production depth.".to_string(),
            ),
        })
        .unwrap();
    let new_role = job_fixture_role(&store, &evidence.id, "canonical_confirmed");
    let score = store
        .record_job_fit_score(job_fixture_score_input(
            &new_role.id,
            &profile.id,
            &evidence.id,
        ))
        .unwrap();
    assert_eq!(score.tier, "tier_1");

    let shortlist = store.compile_job_shortlist(&profile.id).unwrap();
    let entry = shortlist
        .entries
        .iter()
        .find(|entry| entry.role.id == new_role.id)
        .unwrap();
    let score = entry.score.as_ref().unwrap();
    assert_eq!(score.tier, "tier_1");
    assert_eq!(score.weighted_score, 97.4);
    assert!(
        entry
            .outcome_warnings
            .iter()
            .any(|warning| warning.contains("Previous application to Example AI was rejected"))
    );
    assert!(
        entry
            .outcome_warnings
            .iter()
            .any(|warning| warning.contains("one data point, not a scoring rule"))
    );

    let report = store
        .compile_job_weekly_report(&profile.id, "London agent platform roles")
        .unwrap();
    assert!(report.body.contains("Outcome notes"), "{}", report.body);
    assert!(
        report.body.contains("one data point, not a scoring rule"),
        "{}",
        report.body
    );
}

#[test]
fn severe_job_refresh_audit_blocks_immediate_repeats_for_one_day_gate() {
    // CLAIM: a repeated local/manual refresh cannot satisfy the one-day
    // operational proof gate just because two runs exist.
    // ORACLE: two completed refresh runs with valid transition evidence
    // still block under the default 24-hour elapsed requirement.
    // SEVERITY: Severe because otherwise local replay could be mislabeled
    // as wall-clock recurrence proof.
    let store = test_store("job-refresh-audit-blocks-immediate");
    let (profile, scope) = seed_job_refresh_audit_fixture(&store);

    let audit = store
        .audit_job_refresh_history(&profile.id, &scope, None)
        .unwrap();
    assert_eq!(audit.decision, "block");
    assert_eq!(audit.minimum_elapsed_hours, 24);
    assert_eq!(audit.completed_run_count, 2);
    assert!(
        audit
            .missing_requirements
            .iter()
            .any(|missing| missing.contains("not at least 24 hours apart")),
        "{:?}",
        audit.missing_requirements
    );
    assert_eq!(audit.transition_counts.get("new").copied(), Some(1));
    assert_eq!(audit.transition_counts.get("unchanged").copied(), Some(1));
    assert_eq!(audit.transition_counts.get("stale").copied(), Some(1));
    assert_eq!(audit.transition_counts.get("closed").copied(), Some(1));
}

#[test]
fn severe_job_refresh_audit_passes_transition_logic_only_with_lowered_elapsed_gate() {
    // CLAIM: the refresh audit can verify durable transition/source
    // evidence separately from the operational one-day wall-clock gate.
    // ORACLE: the same two completed runs pass when the minimum elapsed
    // threshold is explicitly zero, and the result warns that this is not
    // the operational gate.
    // SEVERITY: Severe because test fixtures should prove audit logic
    // without smuggling a false recurrence claim into status docs.
    let store = test_store("job-refresh-audit-transition-logic");
    let (profile, scope) = seed_job_refresh_audit_fixture(&store);

    let audit = store
        .audit_job_refresh_history(&profile.id, &scope, Some(0))
        .unwrap();
    assert_eq!(audit.decision, "pass", "{:?}", audit.missing_requirements);
    assert_eq!(audit.minimum_elapsed_hours, 0);
    assert_eq!(audit.completed_run_count, 2);
    assert!(audit.elapsed_hours.unwrap() >= 0.0);
    assert_eq!(audit.total_source_count, 2);
    assert_eq!(audit.total_role_count, 4);
    assert_eq!(audit.transition_counts.get("new").copied(), Some(1));
    assert_eq!(audit.transition_counts.get("unchanged").copied(), Some(1));
    assert_eq!(audit.transition_counts.get("stale").copied(), Some(1));
    assert_eq!(audit.transition_counts.get("closed").copied(), Some(1));
    assert!(
        audit
            .warnings
            .iter()
            .any(|warning| warning.contains("below the operational one-day gate")),
        "{:?}",
        audit.warnings
    );
}

#[test]
fn severe_job_import_batch_records_reviewed_packet_without_claiming_live_discovery() {
    // CLAIM: reviewed job-search packets can be imported into durable
    // evidence/source/role/score state without implying live discovery.
    // ORACLE: imported rows round-trip, source-health warnings remain
    // visible, and the report proof level names local reviewed import.
    // SEVERITY: Severe because a JSON ingest endpoint that returns ok but
    // stores no auditable evidence would make the whole workflow hollow.
    let store = test_store("job-import-reviewed-packet");
    let profile_id = job_candidate_profile_id("Chris Batch");
    let evidence_id = job_evidence_card_id(
        &profile_id,
        "Open Cloud",
        "github",
        Some("https://github.com/chrischabot/opencloud"),
    );
    let source_id = job_source_id("https://example.com/careers");
    let role_id = job_role_card_id(
        "Example AI",
        "Staff Agent Platform Engineer",
        "https://example.com/jobs/staff-agent",
    );

    let report = store
        .import_job_batch(JobImportBatchInput {
            profile: Some(JobCandidateProfileInput {
                label: "Chris Batch".to_string(),
                current_resume_source: Some("reviewed local resume".to_string()),
                linkedin_source: None,
                github_profile: Some("https://github.com/chrischabot".to_string()),
                blog_url: Some("https://chabot.dev".to_string()),
                metadata: json!({"source": "test-reviewed-packet"}),
            }),
            privacy_rules: vec![JobPrivacyRuleInput {
                pattern: "private-product-name".to_string(),
                rule_type: "blocked_term".to_string(),
                severity: "block".to_string(),
                replacement_guidance: Some("Use public project language.".to_string()),
            }],
            evidence_cards: vec![JobEvidenceCardInput {
                profile_id: profile_id.clone(),
                title: "Open Cloud".to_string(),
                evidence_type: "github".to_string(),
                visibility: "public".to_string(),
                summary: "Public cloud/developer-tooling project.".to_string(),
                proof_url: Some("https://github.com/chrischabot/opencloud".to_string()),
                local_path: None,
                source_date: Some("2026-06-28".to_string()),
                confidence: "verified".to_string(),
                tags: vec!["developer-tools".to_string(), "cloud".to_string()],
                safe_application_text: "Built public developer tooling for cloud workflows."
                    .to_string(),
                unsafe_terms: vec!["private-product-name".to_string()],
                metadata: json!({}),
            }],
            sources: vec![JobSourceInput {
                source_family: "company".to_string(),
                name: "Example AI careers".to_string(),
                url: "https://example.com/careers".to_string(),
                market_scope: "london".to_string(),
                refresh_policy: "manual".to_string(),
                metadata: json!({}),
            }],
            source_health: vec![JobSourceHealthInput {
                source_id: source_id.clone(),
                status: "failed".to_string(),
                http_status: Some(503),
                error_code: Some("service_unavailable".to_string()),
                fetched_count: 1,
                accepted_count: 0,
                rejected_count: 1,
                note: Some("Fixture source failure must remain visible.".to_string()),
            }],
            roles: vec![JobRoleCardInput {
                company: "Example AI".to_string(),
                role_title: "Staff Agent Platform Engineer".to_string(),
                canonical_url: Some("https://example.com/jobs/staff-agent".to_string()),
                source_family: "company".to_string(),
                source_url: "https://example.com/jobs/staff-agent".to_string(),
                source_confidence: "canonical_confirmed".to_string(),
                date_accessed: Some("2026-06-28T12:00:00Z".to_string()),
                posting_freshness: "same_day".to_string(),
                location: Some("London".to_string()),
                work_mode: Some("hybrid".to_string()),
                company_stage_or_size: Some("startup".to_string()),
                role_seniority: Some("staff".to_string()),
                core_requirements: vec!["agent systems".to_string()],
                implied_business_problem: Some("Make agent workflows reliable.".to_string()),
                why_they_might_need_user: Some(
                    "Public developer-tooling evidence maps to the role.".to_string(),
                ),
                evidence_card_ids: vec![evidence_id.clone()],
                gaps_or_blockers: vec![],
                cluster: Some("agent-platform".to_string()),
                current_status: "live".to_string(),
                metadata: json!({}),
            }],
            fit_scores: vec![JobFitScoreInput {
                role_id: role_id.clone(),
                profile_id: profile_id.clone(),
                scorer: "human".to_string(),
                role_fit: 5.0,
                domain_fit: 5.0,
                evidence_fit: 4.0,
                geo_work_fit: 5.0,
                stage_fit: 4.0,
                practical_odds: 4.0,
                interest_energy: 5.0,
                blockers: vec![],
                evidence_card_ids: vec![evidence_id.clone()],
                explanation: "Reviewed packet maps public evidence to role requirements."
                    .to_string(),
            }],
            companies: vec![JobCompanyCardInput {
                company_name: "Example AI".to_string(),
                website_url: "https://example.com".to_string(),
                source_family: "company".to_string(),
                market: "london".to_string(),
                stage: Some("seed".to_string()),
                funding_signal: None,
                product_category: Some("agent tooling".to_string()),
                technical_audience: Some("developers".to_string()),
                developer_facing_score: 4.0,
                london_relevance: "London office and hiring page.".to_string(),
                remote_maturity: Some("hybrid".to_string()),
                hiring_page_url: Some("https://example.com/careers".to_string()),
                founder_or_team_signal: None,
                metadata: json!({}),
            }],
            ..Default::default()
        })
        .unwrap();

    assert_eq!(report.proof_level, "local_proof_reviewed_packet");
    assert_eq!(report.profile_ids, vec![profile_id.clone()]);
    assert_eq!(report.evidence_card_ids, vec![evidence_id.clone()]);
    assert_eq!(report.source_ids, vec![source_id]);
    assert_eq!(report.role_ids, vec![role_id.clone()]);
    assert_eq!(report.fit_score_ids.len(), 1);
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.contains("does not prove live source discovery"))
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.contains("recorded as failed"))
    );
    assert!(store.read_job_role_card(&role_id).unwrap().is_some());
    let shortlist = store.compile_job_shortlist(&profile_id).unwrap();
    assert_eq!(shortlist.entries[0].score.as_ref().unwrap().tier, "tier_1");
}

#[test]
fn severe_job_manual_refresh_does_not_reannounce_unchanged_or_closed_roles() {
    // CLAIM: refresh reports compare against durable prior state.
    // ORACLE: the first observation is new, the second same observation is
    // unchanged, and a later closed role is blocked in the effective
    // shortlist.
    // SEVERITY: Severe because weekly reports that call every old role
    // "new" or leave closed roles as Tier 1 would be operational fiction.
    let store = test_store("job-manual-refresh-transitions");
    let profile = job_fixture_profile(&store);
    let evidence = job_fixture_evidence(&store, &profile.id);
    let role = job_fixture_role(&store, &evidence.id, "canonical_confirmed");
    store
        .record_job_fit_score(job_fixture_score_input(&role.id, &profile.id, &evidence.id))
        .unwrap();
    let source = store
        .record_job_source(JobSourceInput {
            source_family: "company".to_string(),
            name: "Example AI careers".to_string(),
            url: "https://example.com/careers".to_string(),
            market_scope: "london".to_string(),
            refresh_policy: "manual".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    let healthy = store
        .record_job_source_health(JobSourceHealthInput {
            source_id: source.id.clone(),
            status: "healthy".to_string(),
            http_status: Some(200),
            error_code: None,
            fetched_count: 1,
            accepted_count: 1,
            rejected_count: 0,
            note: Some("Role visible.".to_string()),
        })
        .unwrap();

    let first = store
        .run_job_manual_refresh(JobManualRefreshInput {
            profile_id: profile.id.clone(),
            scope: "London agent platform roles".to_string(),
            observed_role_ids: vec![role.id.clone()],
            stale_role_ids: vec![],
            closed_role_ids: vec![],
            source_health_ids: vec![healthy.id],
            proof_level: "local_proof".to_string(),
            report_artifact_id: None,
        })
        .unwrap();
    assert_eq!(first.new_role_count, 1);
    assert_eq!(first.events[0].status, "new");
    assert_eq!(first.events[0].current_tier.as_deref(), Some("tier_1"));

    let second = store
        .run_job_manual_refresh(JobManualRefreshInput {
            profile_id: profile.id.clone(),
            scope: "London agent platform roles".to_string(),
            observed_role_ids: vec![role.id.clone()],
            stale_role_ids: vec![],
            closed_role_ids: vec![],
            source_health_ids: vec![],
            proof_level: "local_proof".to_string(),
            report_artifact_id: None,
        })
        .unwrap();
    assert_eq!(second.unchanged_role_count, 1);
    assert_eq!(second.events[0].status, "unchanged");

    let failed = store
        .record_job_source_health(JobSourceHealthInput {
            source_id: source.id,
            status: "failed".to_string(),
            http_status: Some(404),
            error_code: Some("not_found".to_string()),
            fetched_count: 1,
            accepted_count: 0,
            rejected_count: 1,
            note: Some("Official role page closed.".to_string()),
        })
        .unwrap();
    let closed = store
        .run_job_manual_refresh(JobManualRefreshInput {
            profile_id: profile.id.clone(),
            scope: "London agent platform roles".to_string(),
            observed_role_ids: vec![],
            stale_role_ids: vec![],
            closed_role_ids: vec![role.id.clone()],
            source_health_ids: vec![failed.id],
            proof_level: "local_proof".to_string(),
            report_artifact_id: None,
        })
        .unwrap();
    assert_eq!(closed.closed_role_count, 1);
    assert_eq!(closed.error_count, 1);
    assert_eq!(closed.events[0].status, "closed");
    assert_eq!(closed.events[0].current_tier.as_deref(), Some("blocked"));

    let role_after = store.read_job_role_card(&role.id).unwrap().unwrap();
    assert_eq!(role_after.current_status, "closed");
    let shortlist = store.compile_job_shortlist(&profile.id).unwrap();
    let score = shortlist.entries[0].score.as_ref().unwrap();
    assert_eq!(score.tier, "blocked");
    assert!(
        score
            .blockers
            .iter()
            .any(|blocker| blocker.contains("role source status is closed"))
    );
}

#[test]
fn severe_job_source_refresh_writes_roles_health_and_stales_missing_roles() {
    // CLAIM: a configured company/ATS job source can be refreshed from a
    // captured page into durable source health, role/source-link rows, and
    // stale events for previously linked roles that disappear.
    // ORACLE: first refresh creates one canonical-confirmed live role and
    // a healthy source-health row; second no-openings snapshot marks the
    // role stale and records stale source health.
    // SEVERITY: Severe because job-search refresh looks complete if it
    // finds new roles but never demotes dead ones.
    let store = test_store("job-source-refresh-company-stale");
    let source = store
        .record_job_source(JobSourceInput {
            source_family: "company".to_string(),
            name: "Example Careers".to_string(),
            url: "https://example.com/careers".to_string(),
            market_scope: "london".to_string(),
            refresh_policy: "manual".to_string(),
            metadata: json!({}),
        })
        .unwrap();

    let first = store
            .run_job_source_refresh(JobSourceRefreshInput {
                source_id: source.id.clone(),
                fetched_url: Some("https://example.com/careers".to_string()),
                body: Some(
                    r#"
                    <main>
                      <h1>Example Careers</h1>
                      <p>Developer platform and agent infrastructure roles in London.</p>
                      <a href="/careers/staff-agent-platform-engineer">Staff Agent Platform Engineer - London hybrid</a>
                    </main>
                    "#
                    .to_string(),
                ),
                fetch_live: false,
            })
            .unwrap();
    assert_eq!(first.source_health.status, "healthy");
    assert_eq!(first.roles.len(), 1);
    assert_eq!(first.companies.len(), 1);
    assert_eq!(first.roles[0].source_confidence, "canonical_confirmed");
    assert_eq!(first.roles[0].current_status, "live");
    assert_eq!(first.roles[0].location.as_deref(), Some("London"));
    assert_eq!(
        first.role_source_links[0].source_id.as_deref(),
        Some(source.id.as_str())
    );
    assert!(
        first
            .warnings
            .iter()
            .any(|warning| warning.contains("caller-supplied page text/html"))
    );

    let second = store
        .run_job_source_refresh(JobSourceRefreshInput {
            source_id: source.id.clone(),
            fetched_url: Some("https://example.com/careers".to_string()),
            body: Some(
                "<main><h1>Example Careers</h1><p>No current openings.</p></main>".to_string(),
            ),
            fetch_live: false,
        })
        .unwrap();
    assert_eq!(second.source_health.status, "stale");
    assert_eq!(second.stale_role_events.len(), 1);
    assert_eq!(second.stale_role_events[0].status, "stale");
    assert_eq!(
        second.stale_role_events[0].current_tier.as_deref(),
        Some("blocked")
    );
    let stale_role = store
        .read_job_role_card(&first.roles[0].id)
        .unwrap()
        .unwrap();
    assert_eq!(stale_role.current_status, "stale");
}

#[test]
fn severe_job_source_refresh_confirms_direct_linked_role_pages() {
    // CLAIM: a reachable direct role URL linked to a known role should not
    // be marked stale merely because the page shell has no parsable job
    // listing anchors.
    // ORACLE: the direct page refresh keeps the role live and records
    // healthy source health; an explicit closed/no-longer-available page
    // still marks the same linked role stale.
    // SEVERITY: Severe because many ATS/detail pages render through a
    // sparse shell, and false stale events would corrupt an application
    // shortlist.
    let store = test_store("job-source-refresh-direct-role");
    let source = store
        .record_job_source(JobSourceInput {
            source_family: "company_ats".to_string(),
            name: "Example ATS role".to_string(),
            url: "https://jobs.example.com/example/staff-agent-platform-engineer".to_string(),
            market_scope: "london".to_string(),
            refresh_policy: "manual_review_before_apply".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    let role = store
        .record_job_role_card(JobRoleCardInput {
            company: "Example".to_string(),
            role_title: "Staff Agent Platform Engineer".to_string(),
            canonical_url: Some(source.url.clone()),
            source_family: source.source_family.clone(),
            source_url: source.url.clone(),
            source_confidence: "canonical_confirmed".to_string(),
            date_accessed: Some("2026-06-28T00:00:00Z".to_string()),
            posting_freshness: "same_day_manual_review".to_string(),
            location: Some("London".to_string()),
            work_mode: Some("hybrid".to_string()),
            company_stage_or_size: Some("startup".to_string()),
            role_seniority: Some("staff".to_string()),
            core_requirements: vec!["agent systems".to_string()],
            implied_business_problem: Some("Make agent workflows reliable.".to_string()),
            why_they_might_need_user: None,
            evidence_card_ids: Vec::new(),
            gaps_or_blockers: Vec::new(),
            cluster: Some("agent-platform".to_string()),
            current_status: "live".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    store
        .record_job_role_source_link(JobRoleSourceLinkInput {
            role_id: role.id.clone(),
            source_id: Some(source.id.clone()),
            source_url: source.url.clone(),
            confidence: "canonical_confirmed".to_string(),
            evidence_excerpt: Some("Known direct role URL.".to_string()),
        })
        .unwrap();

    let direct = store
        .run_job_source_refresh(JobSourceRefreshInput {
            source_id: source.id.clone(),
            fetched_url: Some(source.url.clone()),
            body: Some(
                r#"
                    <html>
                      <main id="app">
                        <h1>Example hiring</h1>
                        <p>This application shell loads details client-side.</p>
                      </main>
                    </html>
                    "#
                .to_string(),
            ),
            fetch_live: false,
        })
        .unwrap();
    assert_eq!(direct.source_health.status, "healthy");
    assert_eq!(direct.roles.len(), 1);
    assert_eq!(direct.roles[0].id, role.id);
    assert_eq!(direct.stale_role_events.len(), 0);
    assert_eq!(
        direct.role_source_links[0].source_id.as_deref(),
        Some(source.id.as_str())
    );
    let still_live = store.read_job_role_card(&role.id).unwrap().unwrap();
    assert_eq!(still_live.current_status, "live");

    let closed = store
        .run_job_source_refresh(JobSourceRefreshInput {
            source_id: source.id.clone(),
            fetched_url: Some(source.url.clone()),
            body: Some(
                "<main><h1>Example hiring</h1><p>This job is no longer available.</p></main>"
                    .to_string(),
            ),
            fetch_live: false,
        })
        .unwrap();
    assert_eq!(closed.source_health.status, "stale");
    assert_eq!(closed.stale_role_events.len(), 1);
    let stale = store.read_job_role_card(&role.id).unwrap().unwrap();
    assert_eq!(stale.current_status, "stale");
}

#[test]
fn severe_job_source_refresh_rejects_product_navigation_as_roles() {
    // CLAIM: a careers-page refresh must not turn product/navigation links
    // into fake job openings just because their text mentions developer,
    // platform, infrastructure, or security.
    // ORACLE: only the anchor whose target URL is a role/careers URL is
    // accepted; product and security links are ignored.
    // SEVERITY: Severe because false-positive role cards would make company
    // targets look application-ready when no actual opening was confirmed.
    let store = test_store("job-source-refresh-no-product-links");
    let source = store
        .record_job_source(JobSourceInput {
            source_family: "company_careers".to_string(),
            name: "North Example careers".to_string(),
            url: "https://north.example/careers".to_string(),
            market_scope: "london".to_string(),
            refresh_policy: "manual".to_string(),
            metadata: json!({}),
        })
        .unwrap();

    let refresh = store
            .run_job_source_refresh(JobSourceRefreshInput {
                source_id: source.id,
                fetched_url: Some("https://north.example/careers".to_string()),
                body: Some(
                    r#"
                    <html>
                      <main>
                        <h1>Careers</h1>
                        <a href="/careers/backend-software-engineer-europe">Backend Software Engineer - Europe</a>
                        <a href="/product/idp">Internal developer platform</a>
                        <a href="/features/infrastructure-layer">Infrastructure layer</a>
                        <a href="/security">Security</a>
                      </main>
                    </html>
                    "#
                    .to_string(),
                ),
                fetch_live: false,
            })
            .unwrap();

    assert_eq!(refresh.roles.len(), 1, "{:#?}", refresh.roles);
    assert_eq!(refresh.roles[0].role_title, "Backend Software Engineer");
    assert!(
        refresh.roles[0]
            .source_url
            .ends_with("/careers/backend-software-engineer-europe")
    );
}

#[test]
fn severe_job_source_refresh_jobs_listing_keeps_anchor_scanning() {
    // CLAIM: a normal /jobs/ listing page is not a single direct job-detail
    // page, so its anchors must still be scanned for role links.
    // ORACLE: the listing URL yields the role linked from the page.
    // SEVERITY: Severe because over-broad direct-page detection would make
    // live source refresh silently miss current openings.
    let store = test_store("job-source-refresh-jobs-listing");
    let source = store
        .record_job_source(JobSourceInput {
            source_family: "company_careers".to_string(),
            name: "Jobs Example careers".to_string(),
            url: "https://jobs.example/jobs/".to_string(),
            market_scope: "london".to_string(),
            refresh_policy: "manual".to_string(),
            metadata: json!({}),
        })
        .unwrap();

    let refresh = store
            .run_job_source_refresh(JobSourceRefreshInput {
                source_id: source.id,
                fetched_url: Some("https://jobs.example/jobs/".to_string()),
                body: Some(
                    r#"
                    <html>
                      <head><title>Open jobs</title></head>
                      <main>
                        <h1>Open jobs</h1>
                        <a href="/jobs/backend-software-engineer-europe">Backend Software Engineer - Europe</a>
                      </main>
                    </html>
                    "#
                    .to_string(),
                ),
                fetch_live: false,
            })
            .unwrap();

    assert_eq!(refresh.roles.len(), 1, "{:#?}", refresh.roles);
    assert_eq!(refresh.roles[0].role_title, "Backend Software Engineer");
    assert!(
        refresh.roles[0]
            .source_url
            .ends_with("/jobs/backend-software-engineer-europe")
    );
}

#[test]
fn severe_job_source_refresh_extracts_direct_role_title_without_html_noise() {
    // CLAIM: a direct ATS role page can create one role card from metadata
    // without scanning raw HTML, scripts, and job-description bullets as
    // separate openings.
    // ORACLE: the Ashby-style page yields exactly the title from <title>
    // and does not import markup or requirement bullets as roles.
    // SEVERITY: Severe because job-detail pages often contain role-like
    // requirement text that must not become duplicate job cards.
    let store = test_store("job-source-refresh-direct-title");
    let source = store
        .record_job_source(JobSourceInput {
            source_family: "company_ats".to_string(),
            name: "Encord Ashby".to_string(),
            url: "https://jobs.ashbyhq.com/encord/role-123".to_string(),
            market_scope: "london".to_string(),
            refresh_policy: "manual_review_before_apply".to_string(),
            metadata: json!({}),
        })
        .unwrap();

    let refresh = store
            .run_job_source_refresh(JobSourceRefreshInput {
                source_id: source.id,
                fetched_url: Some("https://jobs.ashbyhq.com/encord/role-123".to_string()),
                body: Some(
                    r#"
                    <!doctype html>
                    <html>
                      <head>
                        <title>Senior Software Engineer, Infrastructure @ Encord</title>
                        <meta property="og:title" content="Senior Software Engineer, Infrastructure" />
                      </head>
                      <body>
                        <main>
                          <p>Strong backend engineering experience with production-grade systems.</p>
                          <script>fetch("https://cdn.example/manifest.json").then(function (res) { return res.json() })</script>
                        </main>
                      </body>
                    </html>
                    "#
                    .to_string(),
                ),
                fetch_live: false,
            })
            .unwrap();

    assert_eq!(refresh.roles.len(), 1, "{:#?}", refresh.roles);
    assert_eq!(
        refresh.roles[0].role_title,
        "Senior Software Engineer, Infrastructure"
    );
    assert!(!refresh.roles[0].role_title.contains('<'));
    assert_eq!(
        refresh.roles[0].canonical_url.as_deref(),
        Some("https://jobs.ashbyhq.com/encord/role-123")
    );
}

#[test]
fn severe_job_source_refresh_direct_detail_stales_old_requirement_fragment_roles() {
    // CLAIM: refreshing a direct job-detail URL must not keep old fake role
    // rows alive merely because they share the same canonical URL.
    // ORACLE: the page title confirms the actual role, while a previously
    // imported requirement-like row linked to the same URL is marked stale.
    // SEVERITY: Severe because direct ATS pages often include role-like
    // requirement bullets that can otherwise masquerade as openings.
    let store = test_store("job-source-refresh-direct-stales-fragments");
    let source = store
        .record_job_source(JobSourceInput {
            source_family: "company_ats".to_string(),
            name: "Anthropic Greenhouse".to_string(),
            url: "https://job-boards.greenhouse.io/anthropic/jobs/5198999008".to_string(),
            market_scope: "london".to_string(),
            refresh_policy: "manual_review_before_apply".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    let real_role = store
        .record_job_role_card(JobRoleCardInput {
            company: "Anthropic".to_string(),
            role_title: "Technical Specialist, Claude Code".to_string(),
            canonical_url: Some(
                "https://job-boards.greenhouse.io/anthropic/jobs/5198999008".to_string(),
            ),
            source_family: "company_ats".to_string(),
            source_url: "https://job-boards.greenhouse.io/anthropic/jobs/5198999008".to_string(),
            source_confidence: "canonical_confirmed".to_string(),
            date_accessed: Some("2026-06-28T00:00:00Z".to_string()),
            posting_freshness: "same_day".to_string(),
            location: Some("London".to_string()),
            work_mode: Some("hybrid".to_string()),
            company_stage_or_size: None,
            role_seniority: None,
            core_requirements: vec!["developer-facing systems".to_string()],
            implied_business_problem: None,
            why_they_might_need_user: None,
            evidence_card_ids: Vec::new(),
            gaps_or_blockers: Vec::new(),
            cluster: Some("agent-platform".to_string()),
            current_status: "live".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    let fake_fragment = store
        .record_job_role_card(JobRoleCardInput {
            company: "Anthropic".to_string(),
            role_title: "Experience With Go Networking Infrastructure".to_string(),
            canonical_url: Some(
                "https://job-boards.greenhouse.io/anthropic/jobs/5198999008".to_string(),
            ),
            source_family: "company_ats".to_string(),
            source_url: "https://job-boards.greenhouse.io/anthropic/jobs/5198999008".to_string(),
            source_confidence: "canonical_confirmed".to_string(),
            date_accessed: Some("2026-06-28T00:00:00Z".to_string()),
            posting_freshness: "same_day".to_string(),
            location: None,
            work_mode: None,
            company_stage_or_size: None,
            role_seniority: None,
            core_requirements: vec!["infrastructure".to_string()],
            implied_business_problem: None,
            why_they_might_need_user: None,
            evidence_card_ids: Vec::new(),
            gaps_or_blockers: Vec::new(),
            cluster: Some("platform-engineering".to_string()),
            current_status: "live".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    for role in [&real_role, &fake_fragment] {
        store
            .record_job_role_source_link(JobRoleSourceLinkInput {
                role_id: role.id.clone(),
                source_id: Some(source.id.clone()),
                source_url: role.source_url.clone(),
                confidence: "canonical_confirmed".to_string(),
                evidence_excerpt: Some("Previously observed during source refresh.".to_string()),
            })
            .unwrap();
    }

    let refresh = store
            .run_job_source_refresh(JobSourceRefreshInput {
                source_id: source.id,
                fetched_url: Some(
                    "https://job-boards.greenhouse.io/anthropic/jobs/5198999008".to_string(),
                ),
                body: Some(
                    r#"
                    <!doctype html>
                    <html>
                      <head>
                        <title>Job Application for Technical Specialist, Claude Code</title>
                        <meta property="og:title" content="Technical Specialist, Claude Code" />
                      </head>
                      <body>
                        <main>
                          <h1>Technical Specialist, Claude Code</h1>
                          <a href="/anthropic/jobs/5198999008">
                            <li>Experience with Go, networking, VPNs, infrastructure, or security products.</li>
                          </a>
                          <p>Experience with Go, networking, VPNs, infrastructure, or security products.</p>
                        </main>
                      </body>
                    </html>
                    "#
                    .to_string(),
                ),
                fetch_live: false,
            })
            .unwrap();

    assert_eq!(refresh.roles.len(), 1, "{:#?}", refresh.roles);
    assert_eq!(
        refresh.roles[0].role_title,
        "Technical Specialist, Claude Code"
    );
    assert_eq!(refresh.stale_role_events.len(), 1);
    assert_eq!(refresh.stale_role_events[0].role_id, fake_fragment.id);
    let stale_fragment = store
        .read_job_role_card(&fake_fragment.id)
        .unwrap()
        .unwrap();
    assert_eq!(stale_fragment.current_status, "stale");
    let still_live = store.read_job_role_card(&real_role.id).unwrap().unwrap();
    assert_eq!(still_live.current_status, "live");
}

#[test]
fn severe_job_source_refresh_keeps_vc_board_roles_secondary_and_company_cards_monitored() {
    // CLAIM: startup/VC source refresh can create useful company/role
    // monitoring state without pretending secondary sources are canonical.
    // ORACLE: a VC board role is secondary_confirmed, a startup company
    // link becomes a company card, and a high score cannot become Tier 1.
    // SEVERITY: Severe because London startup discovery is useful only if
    // weak source families remain visibly weaker than company/ATS proof.
    let store = test_store("job-source-refresh-vc-board");
    let profile = job_fixture_profile(&store);
    let evidence = job_fixture_evidence(&store, &profile.id);
    let source = store
        .record_job_source(JobSourceInput {
            source_family: "vc_board".to_string(),
            name: "Example VC London Jobs".to_string(),
            url: "https://vc.example/jobs".to_string(),
            market_scope: "london".to_string(),
            refresh_policy: "manual".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    let refresh = store
            .run_job_source_refresh(JobSourceRefreshInput {
                source_id: source.id,
                fetched_url: Some("https://vc.example/jobs".to_string()),
                body: Some(
                    r#"
                    <main>
                      <p>London AI infrastructure startups building APIs and SDKs for developers.</p>
                      <a href="https://orbital.example">Orbital Tools</a>
                      <a href="https://orbital.example/jobs/senior-developer-platform-engineer">Senior Developer Platform Engineer</a>
                      <a href="/jobs">Jobs</a>
                    </main>
                    "#
                    .to_string(),
                ),
                fetch_live: false,
            })
            .unwrap();
    assert_eq!(refresh.roles.len(), 1);
    assert_eq!(refresh.companies.len(), 1);
    assert_eq!(refresh.roles[0].source_confidence, "secondary_confirmed");
    assert_eq!(refresh.source_health.status, "partial");
    assert!(refresh.rejected_count > 0);
    assert!(refresh.companies[0].developer_facing_score >= 4.0);

    let mut score_input = job_fixture_score_input(&refresh.roles[0].id, &profile.id, &evidence.id);
    score_input.evidence_card_ids = vec![evidence.id];
    let score = store.record_job_fit_score(score_input).unwrap();
    assert_eq!(score.tier, "tier_2");
}

#[test]
fn severe_job_company_targets_rank_without_creating_fake_roles() {
    // CLAIM: company-card scouting can rank London startup targets using
    // public evidence tags without pretending those companies have live
    // openings.
    // ORACLE: the best London company ranks first, warnings preserve the
    // no-current-role boundary, and no job_role_cards are written.
    // SEVERITY: Severe because startup scouting is useful only if company
    // targets cannot bypass canonical role-source gates.
    let store = test_store("job-company-targets");
    let profile = job_fixture_profile(&store);
    job_fixture_evidence(&store, &profile.id);
    let strong = store
        .record_job_company_card(JobCompanyCardInput {
            company_name: "Orbital Cloud".to_string(),
            website_url: "https://orbital.example".to_string(),
            source_family: "company".to_string(),
            market: "london".to_string(),
            stage: Some("seed".to_string()),
            funding_signal: Some("technical founder-led developer tools company".to_string()),
            product_category: Some("cloud developer tools".to_string()),
            technical_audience: Some("developer-tools and platform teams".to_string()),
            developer_facing_score: 4.8,
            london_relevance: "high London relevance".to_string(),
            remote_maturity: Some("remote Europe and London hybrid".to_string()),
            hiring_page_url: Some("https://orbital.example/careers".to_string()),
            founder_or_team_signal: Some(
                "Founders write about cloud developer workflows.".to_string(),
            ),
            metadata: json!({ "source": "company map" }),
        })
        .unwrap();
    store
        .record_job_company_card(JobCompanyCardInput {
            company_name: "Local HR App".to_string(),
            website_url: "https://hr.example".to_string(),
            source_family: "directory".to_string(),
            market: "london".to_string(),
            stage: None,
            funding_signal: None,
            product_category: Some("consumer productivity".to_string()),
            technical_audience: None,
            developer_facing_score: 1.5,
            london_relevance: "low".to_string(),
            remote_maturity: None,
            hiring_page_url: None,
            founder_or_team_signal: None,
            metadata: json!({}),
        })
        .unwrap();
    store
        .record_job_company_card(JobCompanyCardInput {
            company_name: "Berlin Platform".to_string(),
            website_url: "https://berlin-platform.example".to_string(),
            source_family: "company".to_string(),
            market: "berlin".to_string(),
            stage: None,
            funding_signal: None,
            product_category: Some("developer-tools".to_string()),
            technical_audience: Some("cloud engineers".to_string()),
            developer_facing_score: 5.0,
            london_relevance: "none".to_string(),
            remote_maturity: None,
            hiring_page_url: Some("https://berlin-platform.example/jobs".to_string()),
            founder_or_team_signal: None,
            metadata: json!({}),
        })
        .unwrap();

    let report = store
        .compile_job_company_target_report(&profile.id, Some("london"), 10)
        .unwrap();
    assert_eq!(report.proof_level, "local_proof");
    assert_eq!(report.market.as_deref(), Some("london"));
    assert_eq!(report.entries.len(), 2);
    assert_eq!(report.entries[0].company.id, strong.id);
    assert_eq!(report.entries[0].tier, "target_now");
    assert!(
        report.entries[0]
            .matched_evidence_tags
            .iter()
            .any(|tag| tag == "developer-tools")
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.contains("not current role cards"))
    );
    assert!(
        report.entries[0]
            .warnings
            .iter()
            .any(|warning| warning.contains("No current role is implied"))
    );
    assert_eq!(store.list_job_role_cards().unwrap().len(), 0);
}

#[test]
fn severe_job_source_refresh_policy_denial_records_failed_health_without_writes() {
    // CLAIM: live job source refresh is explicitly policy-gated before
    // network access and leaves durable failed source health on denial.
    // ORACLE: denied provider.network writes one failed health row and no
    // roles/companies.
    // SEVERITY: Severe because fetch_live=true must not bypass policy or
    // fail invisibly.
    let store = test_store("job-source-refresh-policy-denial");
    write_policy(
        &store,
        r#"
[[rules]]
id = "deny-job-refresh-network"
action = "provider.network"
effect = "deny"
package = "arcwell-job-hunting"
provider = "web"
source = "job_source_refresh"
reason = "test denies job refresh network"
"#,
    );
    let source = store
        .record_job_source(JobSourceInput {
            source_family: "company".to_string(),
            name: "Example Careers".to_string(),
            url: "https://example.com/careers".to_string(),
            market_scope: "london".to_string(),
            refresh_policy: "live".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    let report = store
        .run_job_source_refresh(JobSourceRefreshInput {
            source_id: source.id,
            body: None,
            fetched_url: None,
            fetch_live: true,
        })
        .unwrap();
    assert_eq!(report.source_health.status, "failed");
    assert_eq!(
        report.source_health.error_code.as_deref(),
        Some("policy_denied")
    );
    assert!(report.roles.is_empty());
    assert!(report.companies.is_empty());
    assert!(
        report
            .source_health
            .note
            .as_deref()
            .unwrap()
            .contains("policy denied provider.network")
    );
}

#[test]
fn severe_job_radar_schedule_replay_refreshes_sources_and_reports() {
    // CLAIM: a scheduled job radar watch source is not just a row in the
    // schedule table; the worker can enqueue it, refresh configured
    // sources from replay snapshots, reconcile observed roles, write a
    // weekly report, and advance source-health scheduling state.
    // ORACLE: one worker pass completes a job_radar_refresh job with a
    // durable role, job-source-health row, job search run, weekly report,
    // and healthy generic watch-source health carrying a future next_run.
    // SEVERITY: Severe because scheduled job hunting would otherwise look
    // operational while only manual refresh commands actually work.
    let store = test_store("job-radar-scheduled-replay");
    let profile = job_fixture_profile(&store);
    let source = store
        .record_job_source(JobSourceInput {
            source_family: "company".to_string(),
            name: "Example Careers".to_string(),
            url: "https://example.com/careers".to_string(),
            market_scope: "london".to_string(),
            refresh_policy: "scheduled_replay".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    let mut snapshots = serde_json::Map::new();
    snapshots.insert(
            source.id.clone(),
            json!({
                "fetched_url": "https://example.com/careers",
                "body": r#"
                <main>
                  <h1>Example Careers</h1>
                  <p>Agent infrastructure and developer tooling roles in London.</p>
                  <a href="/careers/staff-agent-platform-engineer">Staff Agent Platform Engineer - London hybrid</a>
                </main>
                "#
            }),
        );
    let scheduled = store
        .schedule_job_radar_refresh(
            &profile.id,
            "London agent platform roles",
            vec![source.id.clone()],
            false,
            Value::Object(snapshots),
            "warm",
            "active",
        )
        .unwrap();
    assert_eq!(scheduled.source_kind, "job_radar");

    let report = store.run_worker_once(1).unwrap();
    assert_eq!(report.processed, 1);
    assert_eq!(report.completed, 1);
    let watch_poll = report.watch_poll.as_ref().unwrap();
    assert_eq!(watch_poll.inspected, 1);
    assert_eq!(watch_poll.enqueued, 1);
    assert_eq!(report.jobs[0].kind, "job_radar_refresh");
    assert_eq!(report.jobs[0].status, "completed");
    let result = report.jobs[0].result_json.as_ref().unwrap();
    assert_eq!(result["action"], "job_radar_refresh");
    assert_eq!(result["source_count"], 1);
    assert_eq!(result["fetch_live"], false);
    assert_eq!(result["proof_level"], "local_proof");
    assert_eq!(result["error_count"], 0);
    assert_eq!(result["observed_role_count"], 1);
    let search_run_id = result["search_run_id"].as_str().unwrap();
    let weekly_report_id = result["weekly_report_id"].as_str().unwrap();

    let roles = store.list_job_role_cards().unwrap();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].role_title, "Staff Agent Platform Engineer");
    assert_eq!(roles[0].source_confidence, "canonical_confirmed");
    assert_eq!(roles[0].current_status, "live");
    let job_health = store.list_job_source_health_recent(10).unwrap();
    assert_eq!(job_health.len(), 1);
    assert_eq!(job_health[0].source_id, source.id);
    assert_eq!(job_health[0].status, "healthy");

    let watch_health = store
        .get_source_health(&format!("job:radar:{}", profile.id))
        .unwrap()
        .unwrap();
    assert_eq!(watch_health.status, "healthy");
    assert_eq!(watch_health.last_item_id.as_deref(), Some(search_run_id));
    assert!(watch_health.next_run_at.is_some());
    assert!(
        store
            .read_job_weekly_report(weekly_report_id)
            .unwrap()
            .is_some()
    );

    let second_pass = store.run_worker_once(1).unwrap();
    assert_eq!(second_pass.processed, 0);
    assert!(second_pass.watch_poll.is_none());
}

#[test]
fn severe_job_radar_schedule_missing_snapshot_records_failed_health() {
    // CLAIM: replay-only scheduled job radar cannot silently succeed when
    // no snapshot is available for a configured source.
    // ORACLE: the worker completes the orchestration job with an explicit
    // missing_snapshot source-health row and failed generic watch health;
    // it does not write any role cards.
    // SEVERITY: Severe because an empty scheduled refresh is the easiest
    // mirage for this workflow: a worker ran, but no source was checked.
    let store = test_store("job-radar-missing-snapshot");
    let profile = job_fixture_profile(&store);
    let source = store
        .record_job_source(JobSourceInput {
            source_family: "company".to_string(),
            name: "Example Careers".to_string(),
            url: "https://example.com/careers".to_string(),
            market_scope: "london".to_string(),
            refresh_policy: "scheduled_replay".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    store
        .schedule_job_radar_refresh(
            &profile.id,
            "London agent platform roles",
            vec![source.id.clone()],
            false,
            json!({}),
            "warm",
            "active",
        )
        .unwrap();

    let report = store.run_worker_once(1).unwrap();
    assert_eq!(report.processed, 1);
    assert_eq!(report.completed, 1);
    let job = &report.jobs[0];
    assert_eq!(job.kind, "job_radar_refresh");
    assert_eq!(job.status, "completed");
    let result = job.result_json.as_ref().unwrap();
    assert_eq!(result["error_count"], 1);
    assert_eq!(result["observed_role_count"], 0);
    assert!(store.list_job_role_cards().unwrap().is_empty());

    let job_health = store.list_job_source_health_recent(10).unwrap();
    assert_eq!(job_health.len(), 1);
    assert_eq!(job_health[0].status, "failed");
    assert_eq!(
        job_health[0].error_code.as_deref(),
        Some("missing_snapshot")
    );

    let watch_health = store
        .get_source_health(&format!("job:radar:{}", profile.id))
        .unwrap()
        .unwrap();
    assert_eq!(watch_health.status, "failed");
    assert!(
        watch_health
            .last_error
            .as_deref()
            .unwrap()
            .contains("unhealthy source")
    );
    assert!(watch_health.next_run_at.is_some());
}

#[test]
fn severe_job_radar_refresh_policy_denial_records_watch_failure_without_source_writes() {
    // CLAIM: live job radar refresh is policy-gated before network/source
    // work, and a pre-executor policy denial still leaves operator-visible
    // source health for the radar profile.
    // ORACLE: a queued job_radar_refresh fails under provider.network
    // denial, writes no job-source-health or role rows, and records failed
    // source_health at job:radar:<profile_id>.
    // SEVERITY: Severe because otherwise live radar can look quiet rather
    // than blocked when policy/cost gates stop it before the executor.
    let store = test_store("job-radar-policy-denial-health");
    let profile = job_fixture_profile(&store);
    let source = store
        .record_job_source(JobSourceInput {
            source_family: "company".to_string(),
            name: "Example Careers".to_string(),
            url: "https://example.com/careers".to_string(),
            market_scope: "london".to_string(),
            refresh_policy: "scheduled_live".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    store
        .enqueue_job_radar_refresh_job(
            &profile.id,
            "London agent platform roles",
            vec![source.id.clone()],
            true,
            json!({}),
        )
        .unwrap();
    write_policy(
        &store,
        r#"
[[rules]]
id = "deny-job-radar-refresh-network"
action = "provider.network"
effect = "deny"
package = "arcwell-job-hunting"
provider = "web"
source = "job_source_refresh"
reason = "test denies job radar refresh network"
"#,
    );

    let report = store.run_worker_once(1).unwrap();
    assert_eq!(report.processed, 1);
    assert_eq!(report.failed, 1);
    assert_eq!(report.completed, 0);
    let job = &report.jobs[0];
    assert_eq!(job.kind, "job_radar_refresh");
    assert_eq!(job.status, "failed");
    assert!(
        job.error
            .as_deref()
            .unwrap()
            .contains("policy denied provider.network")
    );
    assert!(job.result_json.is_none());
    assert!(store.list_job_role_cards().unwrap().is_empty());
    assert!(store.list_job_source_health_recent(10).unwrap().is_empty());

    let watch_health = store
        .get_source_health(&format!("job:radar:{}", profile.id))
        .unwrap()
        .unwrap();
    assert_eq!(watch_health.status, "failed");
    assert!(
        watch_health
            .last_error
            .as_deref()
            .unwrap()
            .contains("policy denied provider.network")
    );
    assert!(watch_health.next_run_at.is_some());
}

#[test]
fn severe_job_radar_refresh_policy_recovery_retries_same_failed_job() {
    // CLAIM: a policy-blocked scheduled radar job is not a terminal quiet
    // failure; the same queued job can retry after policy is repaired and
    // then refresh configured sources.
    // ORACLE: the first worker pass fails before source writes, the second
    // worker pass reclaims the same job after next_run_at is due, completes
    // it, writes job/source health and a role card, and clears the job error.
    // SEVERITY: Severe because otherwise recovery would require manual
    // re-enqueueing while status claimed worker retry support.
    let store = test_store("job-radar-policy-recovery");
    let profile = job_fixture_profile(&store);
    let source = store
        .record_job_source(JobSourceInput {
            source_family: "company".to_string(),
            name: "Example Careers".to_string(),
            url: "https://example.com/careers".to_string(),
            market_scope: "london".to_string(),
            refresh_policy: "scheduled_live".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    let mut snapshots = serde_json::Map::new();
    snapshots.insert(
            source.id.clone(),
            json!({
                "fetched_url": "https://example.com/careers",
                "body": r#"
                <main>
                  <h1>Example Careers</h1>
                  <p>Agent infrastructure and developer tooling roles in London.</p>
                  <a href="/careers/staff-agent-platform-engineer">Staff Agent Platform Engineer - London hybrid</a>
                </main>
                "#
            }),
        );
    let queued = store
        .enqueue_job_radar_refresh_job_with_lineage(
            &profile.id,
            "London agent platform roles",
            vec![source.id.clone()],
            true,
            Value::Object(snapshots),
            Some(json!({
                "watch_source_key": format!("job:radar:{}", profile.id),
                "source_kind": "job_radar",
                "locator": profile.id.clone(),
            })),
        )
        .unwrap();
    assert_eq!(queued.input_json["proof_level"], "local_proof");
    write_policy(
        &store,
        r#"
[[rules]]
id = "deny-job-radar-refresh-network"
action = "provider.network"
effect = "deny"
package = "arcwell-job-hunting"
provider = "web"
source = "job_source_refresh"
reason = "test denies job radar refresh network"
"#,
    );

    let first = store.run_worker_once(1).unwrap();
    assert_eq!(first.processed, 1);
    assert_eq!(first.failed, 1);
    assert_eq!(first.completed, 0);
    let failed = &first.jobs[0];
    assert_eq!(failed.id, queued.id);
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.attempts, 1);
    assert!(failed.next_run_at.is_some());
    assert!(
        failed
            .error
            .as_deref()
            .unwrap()
            .contains("policy denied provider.network")
    );
    assert!(store.list_job_role_cards().unwrap().is_empty());
    assert!(store.list_job_source_health_recent(10).unwrap().is_empty());

    let watch_health = store
        .get_source_health(&format!("job:radar:{}", profile.id))
        .unwrap()
        .unwrap();
    assert_eq!(watch_health.status, "failed");
    assert!(
        watch_health
            .last_error
            .as_deref()
            .unwrap()
            .contains("policy denied provider.network")
    );

    write_policy(
        &store,
        r#"
[[rules]]
id = "allow-job-radar-refresh-network"
action = "provider.network"
effect = "allow"
package = "arcwell-job-hunting"
provider = "web"
source = "job_source_refresh"
reason = "test allows job radar refresh recovery"
"#,
    );
    store
        .conn
        .execute(
            "UPDATE wiki_jobs SET next_run_at = ?2 WHERE id = ?1",
            params![queued.id, "2000-01-01T00:00:00.000000+00:00"],
        )
        .unwrap();

    let second = store.run_worker_once(1).unwrap();
    assert_eq!(second.processed, 1);
    assert_eq!(second.completed, 1);
    assert_eq!(second.failed, 0);
    let recovered = &second.jobs[0];
    assert_eq!(recovered.id, queued.id);
    assert_eq!(recovered.status, "completed");
    assert_eq!(recovered.attempts, 2);
    assert!(recovered.error.is_none());
    let result = recovered.result_json.as_ref().unwrap();
    assert_eq!(result["proof_level"], "local_proof");
    assert_eq!(result["error_count"], 0);
    assert_eq!(result["observed_role_count"], 1);

    let roles = store.list_job_role_cards().unwrap();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].role_title, "Staff Agent Platform Engineer");
    let job_health = store.list_job_source_health_recent(10).unwrap();
    assert_eq!(job_health.len(), 1);
    assert_eq!(job_health[0].source_id, source.id);
    assert_eq!(job_health[0].status, "healthy");

    let watch_health = store
        .get_source_health(&format!("job:radar:{}", profile.id))
        .unwrap()
        .unwrap();
    assert_eq!(watch_health.status, "healthy");
    assert_eq!(
        watch_health.last_item_id.as_deref(),
        result["search_run_id"].as_str()
    );
    assert!(watch_health.next_run_at.is_none());
}

#[test]
fn severe_job_radar_refresh_policy_denial_dead_letters_after_retry_exhaustion() {
    // CLAIM: repeated scheduled radar policy failures do not stay in an
    // endlessly retryable or ambiguous state.
    // ORACLE: after max_attempts denied worker passes, the queued job is
    // dead_lettered, no source/role rows are written, and the radar watch
    // health remains operator-visible as failed.
    // SEVERITY: Severe because operators need to distinguish retrying from
    // exhausted scheduled radar failures.
    let store = test_store("job-radar-policy-dead-letter");
    let profile = job_fixture_profile(&store);
    let source = store
        .record_job_source(JobSourceInput {
            source_family: "company".to_string(),
            name: "Example Careers".to_string(),
            url: "https://example.com/careers".to_string(),
            market_scope: "london".to_string(),
            refresh_policy: "scheduled_live".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    let queued = store
        .enqueue_job_radar_refresh_job(
            &profile.id,
            "London agent platform roles",
            vec![source.id.clone()],
            true,
            json!({}),
        )
        .unwrap();
    assert_eq!(queued.max_attempts, 3);
    write_policy(
        &store,
        r#"
[[rules]]
id = "deny-job-radar-refresh-network"
action = "provider.network"
effect = "deny"
package = "arcwell-job-hunting"
provider = "web"
source = "job_source_refresh"
reason = "test denies job radar refresh network"
"#,
    );

    for expected_attempt in 1..=queued.max_attempts {
        if expected_attempt > 1 {
            store
                .conn
                .execute(
                    "UPDATE wiki_jobs SET next_run_at = ?2 WHERE id = ?1",
                    params![queued.id, "2000-01-01T00:00:00.000000+00:00"],
                )
                .unwrap();
        }
        let report = store.run_worker_once(1).unwrap();
        assert_eq!(report.processed, 1);
        assert_eq!(report.completed, 0);
        let job = &report.jobs[0];
        assert_eq!(job.id, queued.id);
        assert_eq!(job.attempts, expected_attempt);
        assert!(
            job.error
                .as_deref()
                .unwrap()
                .contains("policy denied provider.network")
        );
        if expected_attempt < queued.max_attempts {
            assert_eq!(report.failed, 1);
            assert_eq!(report.dead_lettered, 0);
            assert_eq!(job.status, "failed");
            assert!(job.next_run_at.is_some());
            assert!(job.dead_lettered_at.is_none());
        } else {
            assert_eq!(report.failed, 0);
            assert_eq!(report.dead_lettered, 1);
            assert_eq!(job.status, "dead_lettered");
            assert!(job.next_run_at.is_none());
            assert!(job.dead_lettered_at.is_some());
        }
    }

    assert!(store.list_job_role_cards().unwrap().is_empty());
    assert!(store.list_job_source_health_recent(10).unwrap().is_empty());
    let watch_health = store
        .get_source_health(&format!("job:radar:{}", profile.id))
        .unwrap()
        .unwrap();
    assert_eq!(watch_health.status, "failed");
    assert!(
        watch_health
            .last_error
            .as_deref()
            .unwrap()
            .contains("policy denied provider.network")
    );
}
