use super::*;

pub(crate) fn research_host_search_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchHostSearch> {
    let requested_domains_json: String = row.get(8)?;
    let result_count: i64 = row.get(12)?;
    Ok(ResearchHostSearch {
        id: row.get(0)?,
        run_id: row.get(1)?,
        role_run_id: row.get(2)?,
        host: row.get(3)?,
        tool_surface: row.get(4)?,
        query: row.get(5)?,
        query_intent: row.get(6)?,
        requested_recency: row.get(7)?,
        requested_domains: parse_json_string_vec_column(&requested_domains_json, 8)?,
        executed_at: row.get(9)?,
        retrieved_at: row.get(10)?,
        cost_decision_id: row.get(11)?,
        result_count: result_count.max(0) as usize,
        status: row.get(13)?,
        error_kind: row.get(14)?,
        error_message_redacted: row.get(15)?,
    })
}

pub(crate) fn research_host_search_result_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchHostSearchResult> {
    let rank: i64 = row.get(2)?;
    let provider_metadata_json: String = row.get(9)?;
    let selected_for_ingest: i64 = row.get(10)?;
    Ok(ResearchHostSearchResult {
        id: row.get(0)?,
        host_search_id: row.get(1)?,
        rank: rank.max(0) as usize,
        title: row.get(3)?,
        url: row.get(4)?,
        canonical_url: row.get(5)?,
        snippet: row.get(6)?,
        published_at: row.get(7)?,
        source_family_guess: row.get(8)?,
        provider_metadata: parse_json_column(&provider_metadata_json, 9)?,
        selected_for_ingest: selected_for_ingest != 0,
        research_source_id: row.get(11)?,
        source_card_id: row.get(12)?,
    })
}

pub(crate) fn research_document_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchDocument> {
    let byte_len: i64 = row.get(8)?;
    let page_count: i64 = row.get(13)?;
    let sheet_count: i64 = row.get(14)?;
    let table_count: i64 = row.get(15)?;
    let warning_flags_json: String = row.get(16)?;
    Ok(ResearchDocument {
        id: row.get(0)?,
        run_id: row.get(1)?,
        research_source_id: row.get(2)?,
        source_card_id: row.get(3)?,
        url: row.get(4)?,
        local_path: row.get(5)?,
        media_type: row.get(6)?,
        byte_sha256: row.get(7)?,
        byte_len: byte_len.max(0) as u64,
        retrieved_at: row.get(9)?,
        extractor_name: row.get(10)?,
        extractor_version: row.get(11)?,
        extraction_status: row.get(12)?,
        page_count: page_count.max(0) as usize,
        sheet_count: sheet_count.max(0) as usize,
        table_count: table_count.max(0) as usize,
        warning_flags: parse_json_string_vec_column(&warning_flags_json, 16)?,
        error_message_redacted: row.get(17)?,
    })
}

pub(crate) fn research_document_span_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchDocumentSpan> {
    let page_number: Option<i64> = row.get(3)?;
    let char_start: i64 = row.get(5)?;
    let char_end: i64 = row.get(6)?;
    let bbox_json: Option<String> = row.get(9)?;
    let warning_flags_json: String = row.get(11)?;
    Ok(ResearchDocumentSpan {
        id: row.get(0)?,
        document_id: row.get(1)?,
        span_id: row.get(2)?,
        page_number: page_number.map(|value| value.max(0) as usize),
        section_label: row.get(4)?,
        char_start: char_start.max(0) as usize,
        char_end: char_end.max(0) as usize,
        text_sha256: row.get(7)?,
        text_excerpt: row.get(8)?,
        bbox_json: parse_optional_json_column(bbox_json.as_deref(), 9)?,
        confidence: row.get(10)?,
        warning_flags: parse_json_string_vec_column(&warning_flags_json, 11)?,
    })
}

pub(crate) fn research_table_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ResearchTable> {
    let page_number: Option<i64> = row.get(3)?;
    let bbox_json: Option<String> = row.get(6)?;
    let row_count: i64 = row.get(7)?;
    let column_count: i64 = row.get(8)?;
    let warning_flags_json: String = row.get(11)?;
    Ok(ResearchTable {
        id: row.get(0)?,
        document_id: row.get(1)?,
        table_id: row.get(2)?,
        page_number: page_number.map(|value| value.max(0) as usize),
        sheet_name: row.get(4)?,
        caption: row.get(5)?,
        bbox_json: parse_optional_json_column(bbox_json.as_deref(), 6)?,
        row_count: row_count.max(0) as usize,
        column_count: column_count.max(0) as usize,
        extraction_method: row.get(9)?,
        confidence: row.get(10)?,
        warning_flags: parse_json_string_vec_column(&warning_flags_json, 11)?,
    })
}

pub(crate) fn research_table_cell_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchTableCell> {
    let row_index: i64 = row.get(2)?;
    let column_index: i64 = row.get(3)?;
    let footnote_refs_json: String = row.get(10)?;
    let bbox_json: Option<String> = row.get(11)?;
    Ok(ResearchTableCell {
        id: row.get(0)?,
        table_id: row.get(1)?,
        row_index: row_index.max(0) as usize,
        column_index: column_index.max(0) as usize,
        row_header: row.get(4)?,
        column_header: row.get(5)?,
        raw_text: row.get(6)?,
        normalized_text: row.get(7)?,
        numeric_value: row.get(8)?,
        unit: row.get(9)?,
        footnote_refs: parse_json_string_vec_column(&footnote_refs_json, 10)?,
        bbox_json: parse_optional_json_column(bbox_json.as_deref(), 11)?,
        confidence: row.get(12)?,
    })
}

pub(crate) fn research_editorial_run_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchEditorialRun> {
    let score_json: String = row.get(11)?;
    Ok(ResearchEditorialRun {
        id: row.get(0)?,
        run_id: row.get(1)?,
        stage: row.get(2)?,
        model_provider: row.get(3)?,
        model_name: row.get(4)?,
        prompt_version: row.get(5)?,
        input_artifact_hash: row.get(6)?,
        input_artifact_id: row.get(7)?,
        output_artifact_id: row.get(8)?,
        cost_decision_id: row.get(9)?,
        status: row.get(10)?,
        score: parse_json_column(&score_json, 11)?,
        error_message_redacted: row.get(12)?,
        created_at: row.get(13)?,
    })
}

pub(crate) fn research_claim_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ResearchClaim> {
    let caveats_json: String = row.get(9)?;
    let metadata_json: String = row.get(13)?;
    Ok(ResearchClaim {
        id: row.get(0)?,
        run_id: row.get(1)?,
        text: row.get(2)?,
        kind: row.get(3)?,
        subject: row.get(4)?,
        predicate: row.get(5)?,
        object_value: row.get(6)?,
        temporal_scope: row.get(7)?,
        confidence: row.get(8)?,
        caveats: parse_json_string_vec_column(&caveats_json, 9)?,
        extraction_provider: row.get(10)?,
        extraction_model: row.get(11)?,
        extracted_at: row.get(12)?,
        metadata: parse_json_column(&metadata_json, 13)?,
    })
}

pub(crate) fn research_claim_source_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchClaimSource> {
    Ok(ResearchClaimSource {
        id: row.get(0)?,
        claim_id: row.get(1)?,
        source_card_id: row.get(2)?,
        quote: row.get(3)?,
        source_anchor: row.get(4)?,
        created_at: row.get(5)?,
    })
}

pub(crate) fn research_claim_document_anchor_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchClaimDocumentAnchor> {
    let row_index: Option<i64> = row.get(12)?;
    let column_index: Option<i64> = row.get(13)?;
    Ok(ResearchClaimDocumentAnchor {
        id: row.get(0)?,
        claim_source_id: row.get(1)?,
        document_id: row.get(2)?,
        anchor_kind: row.get(3)?,
        document_span_id: row.get(4)?,
        table_id: row.get(5)?,
        table_cell_id: row.get(6)?,
        span_id: row.get(10)?,
        table_local_id: row.get(11)?,
        row_index: row_index.map(|value| value.max(0) as usize),
        column_index: column_index.map(|value| value.max(0) as usize),
        anchor_label: row.get(7)?,
        quote: row.get(8)?,
        created_at: row.get(9)?,
    })
}

pub(crate) fn research_cluster_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchCluster> {
    let claim_count: i64 = row.get(4)?;
    Ok(ResearchCluster {
        id: row.get(0)?,
        run_id: row.get(1)?,
        theme: row.get(2)?,
        summary: row.get(3)?,
        claim_count: claim_count.max(0) as usize,
        evidence_strength: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

pub(crate) fn research_contradiction_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchContradiction> {
    Ok(ResearchContradiction {
        id: row.get(0)?,
        run_id: row.get(1)?,
        left_claim_id: row.get(2)?,
        right_claim_id: row.get(3)?,
        severity: row.get(4)?,
        notes: row.get(5)?,
        created_at: row.get(6)?,
    })
}

pub(crate) fn research_iteration_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchIteration> {
    let iteration_index: i64 = row.get(2)?;
    Ok(ResearchIteration {
        id: row.get(0)?,
        run_id: row.get(1)?,
        iteration_index: iteration_index.max(0) as usize,
        parent_iteration_id: row.get(3)?,
        status: row.get(4)?,
        objective: row.get(5)?,
        position_artifact_id: row.get(6)?,
        statement_set_artifact_id: row.get(7)?,
        challenge_pack_artifact_id: row.get(8)?,
        disproof_pack_artifact_id: row.get(9)?,
        revision_artifact_id: row.get(10)?,
        convergence_snapshot_id: row.get(11)?,
        cost_decision_id: row.get(12)?,
        started_at: row.get(13)?,
        completed_at: row.get(14)?,
        stop_reason: row.get(15)?,
        error_message_redacted: row.get(16)?,
    })
}

pub(crate) fn research_statement_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchStatement> {
    let evidence_json: String = row.get(13)?;
    let counterevidence_json: String = row.get(14)?;
    let assumptions_json: String = row.get(15)?;
    let caveats_json: String = row.get(16)?;
    Ok(ResearchStatement {
        id: row.get(0)?,
        run_id: row.get(1)?,
        iteration_id: row.get(2)?,
        parent_statement_id: row.get(3)?,
        stable_key: row.get(4)?,
        statement_type: row.get(5)?,
        text: row.get(6)?,
        scope: row.get(7)?,
        temporal_scope: row.get(8)?,
        confidence: row.get(9)?,
        certainty_label: row.get(10)?,
        status: row.get(11)?,
        importance: row.get(12)?,
        evidence: parse_json_column(&evidence_json, 13)?,
        counterevidence: parse_json_column(&counterevidence_json, 14)?,
        assumptions: parse_json_column(&assumptions_json, 15)?,
        caveats: parse_json_column(&caveats_json, 16)?,
        created_by_role: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

pub(crate) fn research_challenge_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchChallenge> {
    let would_change_answer_if_true: i64 = row.get(7)?;
    let search_plan_json: String = row.get(8)?;
    let required_source_families_json: String = row.get(9)?;
    Ok(ResearchChallenge {
        id: row.get(0)?,
        run_id: row.get(1)?,
        iteration_id: row.get(2)?,
        statement_id: row.get(3)?,
        challenge_type: row.get(4)?,
        severity: row.get(5)?,
        rationale: row.get(6)?,
        would_change_answer_if_true: would_change_answer_if_true != 0,
        search_plan: parse_json_column(&search_plan_json, 8)?,
        required_source_families: parse_json_column(&required_source_families_json, 9)?,
        status: row.get(10)?,
        created_by_role: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

pub(crate) fn research_disproof_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchDisproof> {
    let evidence_json: String = row.get(7)?;
    let requires_revision: i64 = row.get(10)?;
    Ok(ResearchDisproof {
        id: row.get(0)?,
        run_id: row.get(1)?,
        iteration_id: row.get(2)?,
        challenge_id: row.get(3)?,
        statement_id: row.get(4)?,
        verdict: row.get(5)?,
        strength: row.get(6)?,
        evidence: parse_json_column(&evidence_json, 7)?,
        reasoning_summary: row.get(8)?,
        confidence_delta: row.get(9)?,
        requires_revision: requires_revision != 0,
        created_by_role: row.get(11)?,
        created_at: row.get(12)?,
    })
}

pub(crate) fn research_revision_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchRevision> {
    let trigger_disproof_ids_json: String = row.get(7)?;
    let evidence_delta_json: String = row.get(8)?;
    Ok(ResearchRevision {
        id: row.get(0)?,
        run_id: row.get(1)?,
        iteration_id: row.get(2)?,
        from_statement_id: row.get(3)?,
        to_statement_id: row.get(4)?,
        revision_type: row.get(5)?,
        rationale: row.get(6)?,
        trigger_disproof_ids: parse_json_column(&trigger_disproof_ids_json, 7)?,
        evidence_delta: parse_json_column(&evidence_delta_json, 8)?,
        created_at: row.get(9)?,
    })
}

pub(crate) fn research_fact_check_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchFactCheck> {
    let evidence_json: String = row.get(6)?;
    Ok(ResearchFactCheck {
        id: row.get(0)?,
        run_id: row.get(1)?,
        iteration_id: row.get(2)?,
        statement_id: row.get(3)?,
        label: row.get(4)?,
        impact: row.get(5)?,
        evidence: parse_json_column(&evidence_json, 6)?,
        notes: row.get(7)?,
        created_at: row.get(8)?,
    })
}

pub(crate) fn research_convergence_snapshot_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchConvergenceSnapshot> {
    let stop_rule_json: String = row.get(23)?;
    let settled: i64 = row.get(24)?;
    Ok(ResearchConvergenceSnapshot {
        id: row.get(0)?,
        run_id: row.get(1)?,
        iteration_id: row.get(2)?,
        source_count_total: nonnegative_usize(row.get(3)?),
        source_count_new: nonnegative_usize(row.get(4)?),
        primary_source_count_new: nonnegative_usize(row.get(5)?),
        claim_count_total: nonnegative_usize(row.get(6)?),
        statement_count_current: nonnegative_usize(row.get(7)?),
        statement_count_changed: nonnegative_usize(row.get(8)?),
        critical_open_challenges: nonnegative_usize(row.get(9)?),
        high_open_challenges: nonnegative_usize(row.get(10)?),
        strong_refutations: nonnegative_usize(row.get(11)?),
        unknown_high_impact_claims: nonnegative_usize(row.get(12)?),
        mean_confidence_delta: row.get(13)?,
        max_confidence_delta: row.get(14)?,
        source_novelty_score: row.get(15)?,
        claim_novelty_score: row.get(16)?,
        position_edit_distance: row.get(17)?,
        citation_support_score: row.get(18)?,
        active_fact_check_score: row.get(19)?,
        evaluator_score: row.get(20)?,
        cost_usd_estimated: row.get(21)?,
        elapsed_seconds: row.get(22)?,
        stop_rule: parse_json_column(&stop_rule_json, 23)?,
        settled: settled != 0,
        created_at: row.get(25)?,
    })
}

pub(crate) fn research_report_judgment_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchReportJudgment> {
    let scores_json: String = row.get(5)?;
    let blocking_findings_json: String = row.get(6)?;
    let non_blocking_findings_json: String = row.get(7)?;
    let evidence_checked_json: String = row.get(8)?;
    let remaining_risks_json: String = row.get(9)?;
    let commands_or_artifacts_reviewed_json: String = row.get(10)?;
    Ok(ResearchReportJudgment {
        id: row.get(0)?,
        run_id: row.get(1)?,
        report_id: row.get(2)?,
        judgment_version: row.get(3)?,
        overall_decision: row.get(4)?,
        scores: parse_json_column(&scores_json, 5)?,
        blocking_findings: parse_json_column(&blocking_findings_json, 6)?,
        non_blocking_findings: parse_json_column(&non_blocking_findings_json, 7)?,
        evidence_checked: parse_json_column(&evidence_checked_json, 8)?,
        remaining_risks: parse_json_column(&remaining_risks_json, 9)?,
        commands_or_artifacts_reviewed: parse_json_column(
            &commands_or_artifacts_reviewed_json,
            10,
        )?,
        created_at: row.get(11)?,
    })
}

pub(crate) fn source_health_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SourceHealth> {
    Ok(SourceHealth {
        key: row.get(0)?,
        provider: row.get(1)?,
        source_kind: row.get(2)?,
        locator: row.get(3)?,
        status: row.get(4)?,
        last_success_at: row.get(5)?,
        last_failure_at: row.get(6)?,
        last_error: row.get(7)?,
        last_item_id: row.get(8)?,
        last_item_date: row.get(9)?,
        cursor_key: row.get(10)?,
        cursor_value: row.get(11)?,
        next_run_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}
