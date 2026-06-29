use super::*;

pub(crate) fn cursor_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CursorState> {
    Ok(CursorState {
        key: row.get(0)?,
        value: row.get(1)?,
        updated_at: row.get(2)?,
    })
}

pub(crate) fn edge_event_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EdgeEvent> {
    let payload_json: String = row.get(4)?;
    Ok(EdgeEvent {
        id: row.get(0)?,
        source: row.get(1)?,
        idempotency_key: row.get(2)?,
        status: row.get(3)?,
        payload_json: parse_json_column(&payload_json, 4)?,
        attempts: row.get(5)?,
        max_attempts: row.get(6)?,
        leased_until: row.get(7)?,
        next_run_at: row.get(8)?,
        error: row.get(9)?,
        received_at: row.get(10)?,
        expires_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

pub(crate) fn channel_message_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ChannelMessage> {
    Ok(ChannelMessage {
        id: row.get(0)?,
        channel: row.get(1)?,
        direction: row.get(2)?,
        project_id: row.get(3)?,
        sender: row.get(4)?,
        body: row.get(5)?,
        status: row.get(6)?,
        source_event_id: row.get(7)?,
        created_at: row.get(8)?,
    })
}

pub(crate) fn channel_authorization_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ChannelAuthorization> {
    Ok(ChannelAuthorization {
        channel: row.get(0)?,
        subject: row.get(1)?,
        can_read_projects: row.get::<_, i64>(2)? != 0,
        can_write_projects: row.get::<_, i64>(3)? != 0,
        can_send: row.get::<_, i64>(4)? != 0,
        updated_at: row.get(5)?,
    })
}

pub(crate) fn channel_delivery_attempt_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ChannelDeliveryAttempt> {
    let response_json: String = row.get(7)?;
    let response = parse_json_column(&response_json, 7)?;
    Ok(ChannelDeliveryAttempt {
        id: row.get(0)?,
        message_id: row.get(1)?,
        channel: row.get(2)?,
        destination: row.get(3)?,
        attempt: row.get(4)?,
        ok: row.get::<_, i64>(5)? != 0,
        provider_status: row.get(6)?,
        response,
        error: row.get(8)?,
        retry_at: row.get(9)?,
        created_at: row.get(10)?,
    })
}

pub(crate) fn cost_policy_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CostPolicy> {
    Ok(CostPolicy {
        scope: row.get(0)?,
        key: row.get(1)?,
        limit_usd: row.get(2)?,
        kill_switch: row.get::<_, i64>(3)? != 0,
        override_until: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

pub(crate) fn cost_decision_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<CostDecisionRecord> {
    Ok(CostDecisionRecord {
        id: row.get(0)?,
        allowed: row.get::<_, i64>(1)? != 0,
        reason: row.get(2)?,
        package: row.get(3)?,
        job_id: row.get(4)?,
        provider: row.get(5)?,
        model: row.get(6)?,
        source: row.get(7)?,
        projected_usd: row.get(8)?,
        spent_usd: row.get(9)?,
        remaining_usd: row.get(10)?,
        matched_scope: row.get(11)?,
        matched_key: row.get(12)?,
        created_at: row.get(13)?,
    })
}

pub(crate) fn policy_decision_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<PolicyDecisionRecord> {
    let metadata_json: String = row.get(13)?;
    let metadata = parse_json_column(&metadata_json, 13)?;
    Ok(PolicyDecisionRecord {
        id: row.get(0)?,
        action: row.get(1)?,
        effect: row.get(2)?,
        allowed: row.get::<_, i64>(3)? != 0,
        reason: row.get(4)?,
        matched_rule_id: row.get(5)?,
        approval_id: row.get(6)?,
        package: row.get(7)?,
        provider: row.get(8)?,
        source: row.get(9)?,
        channel: row.get(10)?,
        subject: row.get(11)?,
        target: row.get(12)?,
        metadata,
        created_at: row.get(14)?,
    })
}

pub(crate) fn policy_approval_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<PolicyApprovalRecord> {
    Ok(PolicyApprovalRecord {
        id: row.get(0)?,
        decision_id: row.get(1)?,
        action: row.get(2)?,
        status: row.get(3)?,
        reason: row.get(4)?,
        created_at: row.get(5)?,
        resolved_at: row.get(6)?,
    })
}

pub(crate) fn bool_to_i64(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

pub(crate) fn nonnegative_usize(value: i64) -> usize {
    value.max(0) as usize
}

pub(crate) fn count_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

pub(crate) fn project_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRecord> {
    let aliases_json: String = row.get(2)?;
    Ok(ProjectRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        aliases: serde_json::from_str(&aliases_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        status: row.get(3)?,
        summary: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

pub(crate) fn project_status_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProjectStatusSnapshot> {
    Ok(ProjectStatusSnapshot {
        id: row.get(0)?,
        project_id: row.get(1)?,
        status: row.get(2)?,
        summary: row.get(3)?,
        source: row.get(4)?,
        thread_ref: row.get(5)?,
        confidence: row.get(6)?,
        created_at: row.get(7)?,
        live_verified: row.get::<_, i64>(8)? != 0,
        verified_host: row.get(9)?,
        verified_thread_id: row.get(10)?,
        verified_at: row.get(11)?,
        stale_after_seconds: row.get(12)?,
    })
}

pub(crate) fn controller_context_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ControllerChannelContext> {
    Ok(ControllerChannelContext {
        id: row.get(0)?,
        channel: row.get(1)?,
        account_id: row.get(2)?,
        conversation_id: row.get(3)?,
        sender: row.get(4)?,
        trust_tier: row.get(5)?,
        last_project_id: row.get(6)?,
        last_thread_id: row.get(7)?,
        last_run_id: row.get(8)?,
        last_intent: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub(crate) fn controller_thread_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ControllerThread> {
    Ok(ControllerThread {
        id: row.get(0)?,
        host: row.get(1)?,
        host_thread_id: row.get(2)?,
        project_id: row.get(3)?,
        title: row.get(4)?,
        cwd: row.get(5)?,
        branch: row.get(6)?,
        worktree: row.get(7)?,
        status: row.get(8)?,
        active: row.get::<_, i64>(9)? != 0,
        archived: row.get::<_, i64>(10)? != 0,
        current_goal: row.get(11)?,
        latest_summary: row.get(12)?,
        latest_summary_source: row.get(13)?,
        last_activity_at: row.get(14)?,
        last_synced_at: row.get(15)?,
    })
}

pub(crate) fn controller_run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ControllerRun> {
    Ok(ControllerRun {
        id: row.get(0)?,
        thread_id: row.get(1)?,
        project_id: row.get(2)?,
        origin_channel_message_id: row.get(3)?,
        host: row.get(4)?,
        host_run_id: row.get(5)?,
        kind: row.get(6)?,
        status: row.get(7)?,
        requested_action: row.get(8)?,
        cancel_requested: row.get::<_, i64>(9)? != 0,
        cancel_reason: row.get(10)?,
        started_at: row.get(11)?,
        updated_at: row.get(12)?,
        finished_at: row.get(13)?,
    })
}

pub(crate) fn controller_event_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ControllerEvent> {
    let data_json: String = row.get(6)?;
    Ok(ControllerEvent {
        id: row.get(0)?,
        run_id: row.get(1)?,
        thread_id: row.get(2)?,
        project_id: row.get(3)?,
        event_type: row.get(4)?,
        summary: row.get(5)?,
        data: parse_json_column(&data_json, 6)?,
        source: row.get(7)?,
        created_at: row.get(8)?,
    })
}

pub(crate) fn controller_pending_action_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ControllerPendingAction> {
    let payload_json: String = row.get(8)?;
    Ok(ControllerPendingAction {
        id: row.get(0)?,
        channel: row.get(1)?,
        conversation_id: row.get(2)?,
        sender: row.get(3)?,
        action_type: row.get(4)?,
        project_id: row.get(5)?,
        thread_id: row.get(6)?,
        run_id: row.get(7)?,
        payload: parse_json_column(&payload_json, 8)?,
        reason: row.get(9)?,
        status: row.get(10)?,
        expires_at: row.get(11)?,
        created_at: row.get(12)?,
        resolved_at: row.get(13)?,
    })
}

pub(crate) fn work_run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkRun> {
    let follow_ups_json: String = row.get(9)?;
    let reusable_lessons_json: String = row.get(10)?;
    Ok(WorkRun {
        id: row.get(0)?,
        goal: row.get(1)?,
        project_id: row.get(2)?,
        host_id: row.get(3)?,
        thread_id: row.get(4)?,
        agent_surface: row.get(5)?,
        status: row.get(6)?,
        outcome: row.get(7)?,
        validation_summary: row.get(8)?,
        follow_ups: serde_json::from_str(&follow_ups_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                9,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        reusable_lessons: serde_json::from_str(&reusable_lessons_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                10,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
        completed_at: row.get(13)?,
    })
}

pub(crate) fn work_event_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkEvent> {
    let data_json: String = row.get(4)?;
    Ok(WorkEvent {
        id: row.get(0)?,
        run_id: row.get(1)?,
        event_type: row.get(2)?,
        summary: row.get(3)?,
        data: parse_json_column(&data_json, 4)?,
        created_at: row.get(5)?,
    })
}

pub(crate) fn work_artifact_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkArtifact> {
    let metadata_json: String = row.get(5)?;
    Ok(WorkArtifact {
        id: row.get(0)?,
        run_id: row.get(1)?,
        artifact_type: row.get(2)?,
        locator: row.get(3)?,
        role: row.get(4)?,
        metadata: parse_json_column(&metadata_json, 5)?,
        created_at: row.get(6)?,
    })
}

pub(crate) fn work_link_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkLink> {
    Ok(WorkLink {
        id: row.get(0)?,
        run_id: row.get(1)?,
        target_type: row.get(2)?,
        target_id: row.get(3)?,
        role: row.get(4)?,
        generated_summary: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
    })
}

pub(crate) fn procedure_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Procedure> {
    let preconditions_json: String = row.get(4)?;
    let tools_json: String = row.get(5)?;
    let validation_commands_json: String = row.get(6)?;
    let known_risks_json: String = row.get(7)?;
    Ok(Procedure {
        id: row.get(0)?,
        title: row.get(1)?,
        trigger_context: row.get(2)?,
        problem: row.get(3)?,
        preconditions: parse_json_string_vec_column(&preconditions_json, 4)?,
        tools: parse_json_string_vec_column(&tools_json, 5)?,
        validation_commands: parse_json_string_vec_column(&validation_commands_json, 6)?,
        known_risks: parse_json_string_vec_column(&known_risks_json, 7)?,
        confidence: row.get(8)?,
        freshness_days: row.get(9)?,
        last_reviewed_at: row.get(10)?,
        status: row.get(11)?,
        current_version: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        archived_at: row.get(15)?,
    })
}

pub(crate) fn procedure_version_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProcedureVersion> {
    let source_run_ids_json: String = row.get(4)?;
    let provenance_json: String = row.get(5)?;
    let artifact_path: String = row.get(6)?;
    Ok(ProcedureVersion {
        id: row.get(0)?,
        procedure_id: row.get(1)?,
        version: row.get(2)?,
        method: row.get(3)?,
        source_run_ids: parse_json_string_vec_column(&source_run_ids_json, 4)?,
        provenance: parse_json_column(&provenance_json, 5)?,
        artifact_path: PathBuf::from(artifact_path),
        content_sha256: row.get(7)?,
        created_at: row.get(8)?,
    })
}

pub(crate) fn procedure_candidate_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProcedureCandidate> {
    let preconditions_json: String = row.get(7)?;
    let tools_json: String = row.get(9)?;
    let validation_commands_json: String = row.get(10)?;
    let known_risks_json: String = row.get(11)?;
    let source_run_ids_json: String = row.get(12)?;
    let provenance_json: String = row.get(13)?;
    let applied_result_json: Option<String> = row.get(22)?;
    Ok(ProcedureCandidate {
        id: row.get(0)?,
        operation: row.get(1)?,
        procedure_id: row.get(2)?,
        base_version: row.get(3)?,
        title: row.get(4)?,
        trigger_context: row.get(5)?,
        problem: row.get(6)?,
        preconditions: parse_json_string_vec_column(&preconditions_json, 7)?,
        method: row.get(8)?,
        tools: parse_json_string_vec_column(&tools_json, 9)?,
        validation_commands: parse_json_string_vec_column(&validation_commands_json, 10)?,
        known_risks: parse_json_string_vec_column(&known_risks_json, 11)?,
        source_run_ids: parse_json_string_vec_column(&source_run_ids_json, 12)?,
        provenance: parse_json_column(&provenance_json, 13)?,
        sensitivity: row.get(14)?,
        status: row.get(15)?,
        reason: row.get(16)?,
        content_sha256: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
        applied_at: row.get(20)?,
        rejected_reason: row.get(21)?,
        applied_result: match applied_result_json {
            Some(raw) => Some(parse_json_column(&raw, 22)?),
            None => None,
        },
    })
}

pub(crate) fn digest_candidate_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<DigestCandidate> {
    let source_card_ids_json: String = row.get(5)?;
    Ok(DigestCandidate {
        id: row.get(0)?,
        topic: row.get(1)?,
        score: row.get(2)?,
        reason: row.get(3)?,
        status: row.get(4)?,
        source_card_ids: serde_json::from_str(&source_card_ids_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                5,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        review_status: row.get(6)?,
        reviewed_at: row.get(7)?,
        reviewed_by: row.get(8)?,
        review_note: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

pub(crate) fn digest_delivery_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<DigestDelivery> {
    Ok(DigestDelivery {
        id: row.get(0)?,
        candidate_id: row.get(1)?,
        channel: row.get(2)?,
        subject: row.get(3)?,
        target: row.get(4)?,
        idempotency_key: row.get(5)?,
        status: row.get(6)?,
        policy_decision_id: row.get(7)?,
        channel_message_id: row.get(8)?,
        channel_delivery_attempt_id: row.get(9)?,
        error: row.get(10)?,
        retry_at: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

pub(crate) fn digest_alert_schedule_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<DigestAlertSchedule> {
    let quiet_hours_json: Option<String> = row.get(8)?;
    Ok(DigestAlertSchedule {
        id: row.get(0)?,
        name: row.get(1)?,
        status: row.get(2)?,
        channel: row.get(3)?,
        recipient_ref: row.get(4)?,
        min_score: row.get(5)?,
        max_candidates: row.get(6)?,
        interval_hours: row.get(7)?,
        quiet_hours: quiet_hours_json
            .map(|raw| parse_json_column(&raw, 8))
            .transpose()?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub(crate) fn digest_alert_tick_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<DigestAlertTick> {
    let candidate_ids_json: String = row.get(6)?;
    let delivery_ids_json: String = row.get(7)?;
    Ok(DigestAlertTick {
        id: row.get(0)?,
        schedule_id: row.get(1)?,
        tick_key: row.get(2)?,
        due_at: row.get(3)?,
        status: row.get(4)?,
        job_id: row.get(5)?,
        candidate_ids: parse_json_string_vec_column(&candidate_ids_json, 6)?,
        delivery_ids: parse_json_string_vec_column(&delivery_ids_json, 7)?,
        error: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub(crate) fn issue_schedule_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IssueSchedule> {
    let metadata_json: String = row.get(10)?;
    Ok(IssueSchedule {
        id: row.get(0)?,
        name: row.get(1)?,
        status: row.get(2)?,
        kind: row.get(3)?,
        channel: row.get(4)?,
        recipient_ref: row.get(5)?,
        time_zone: row.get(6)?,
        hour: row.get(7)?,
        minute: row.get(8)?,
        catch_up_hours: row.get(9)?,
        metadata: parse_json_column(&metadata_json, 10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

pub(crate) fn issue_schedule_tick_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<IssueScheduleTick> {
    Ok(IssueScheduleTick {
        id: row.get(0)?,
        schedule_id: row.get(1)?,
        tick_key: row.get(2)?,
        due_at: row.get(3)?,
        status: row.get(4)?,
        job_id: row.get(5)?,
        candidate_id: row.get(6)?,
        delivery_id: row.get(7)?,
        error: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub(crate) fn radar_profile_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RadarProfile> {
    let languages_json: String = row.get(7)?;
    let category_groups_json: String = row.get(8)?;
    let source_selectors_json: String = row.get(9)?;
    let delivery_policy_json: String = row.get(10)?;
    let model_policy_json: String = row.get(11)?;
    let metadata_json: String = row.get(12)?;
    Ok(RadarProfile {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        status: row.get(3)?,
        window_hours: row.get(4)?,
        min_score: row.get(5)?,
        max_items: row.get(6)?,
        languages: parse_json_string_vec_column(&languages_json, 7)?,
        category_groups: parse_json_column(&category_groups_json, 8)?,
        source_selectors: parse_json_column(&source_selectors_json, 9)?,
        delivery_policy: parse_json_column(&delivery_policy_json, 10)?,
        model_policy: parse_json_column(&model_policy_json, 11)?,
        metadata: parse_json_column(&metadata_json, 12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

pub(crate) fn radar_run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RadarRun> {
    let source_selection_json: String = row.get(6)?;
    let metadata_json: String = row.get(16)?;
    Ok(RadarRun {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        status: row.get(2)?,
        window_start: row.get(3)?,
        window_end: row.get(4)?,
        stage: row.get(5)?,
        source_selection: parse_json_column(&source_selection_json, 6)?,
        raw_count: row.get(7)?,
        normalized_count: row.get(8)?,
        indexed_count: row.get(9)?,
        scored_count: row.get(10)?,
        filtered_count: row.get(11)?,
        enriched_count: row.get(12)?,
        summary_count: row.get(13)?,
        delivery_count: row.get(14)?,
        error: row.get(15)?,
        metadata: parse_json_column(&metadata_json, 16)?,
        started_at: row.get(17)?,
        finished_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

pub(crate) fn radar_item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RadarItem> {
    let metadata_json: String = row.get(14)?;
    Ok(RadarItem {
        id: row.get(0)?,
        run_id: row.get(1)?,
        stable_key: row.get(2)?,
        source_kind: row.get(3)?,
        provider: row.get(4)?,
        source_locator: row.get(5)?,
        native_id: row.get(6)?,
        canonical_url: row.get(7)?,
        title: row.get(8)?,
        author: row.get(9)?,
        published_at: row.get(10)?,
        fetched_at: row.get(11)?,
        content_text: row.get(12)?,
        content_sha256: row.get(13)?,
        metadata: parse_json_column(&metadata_json, 14)?,
        source_card_id: row.get(15)?,
        wiki_page_id: row.get(16)?,
        canonical_entity_ref: row.get(17)?,
        trust_level: row.get(18)?,
        created_at: row.get(19)?,
        updated_at: row.get(20)?,
    })
}

pub(crate) fn radar_score_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RadarScore> {
    let tags_json: String = row.get(6)?;
    Ok(RadarScore {
        id: row.get(0)?,
        run_id: row.get(1)?,
        item_id: row.get(2)?,
        score_kind: row.get(3)?,
        score: row.get(4)?,
        reason: row.get(5)?,
        tags: parse_json_string_vec_column(&tags_json, 6)?,
        model_provider: row.get(7)?,
        model_name: row.get(8)?,
        cost_decision_id: row.get(9)?,
        input_artifact_id: row.get(10)?,
        output_artifact_id: row.get(11)?,
        schema_version: row.get(12)?,
        status: row.get(13)?,
        error: row.get(14)?,
        created_at: row.get(15)?,
    })
}

pub(crate) fn radar_source_quality_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<RadarSourceQuality> {
    Ok(RadarSourceQuality {
        id: row.get(0)?,
        run_id: row.get(1)?,
        source_kind: row.get(2)?,
        locator: row.get(3)?,
        window_start: row.get(4)?,
        window_end: row.get(5)?,
        raw_count: row.get(6)?,
        accepted_count: row.get(7)?,
        average_score: row.get(8)?,
        score_p50: row.get(9)?,
        score_p90: row.get(10)?,
        signal_to_noise: row.get(11)?,
        duplicate_rate: row.get(12)?,
        delivery_contribution_count: row.get(13)?,
        failure_count: row.get(14)?,
        status: row.get(15)?,
        created_at: row.get(16)?,
    })
}

pub(crate) fn radar_source_quality_trend_from_rows(
    source_kind: String,
    locator: String,
    rows: &[RadarSourceQuality],
) -> Result<RadarSourceQualityTrend> {
    let Some(first) = rows.first() else {
        bail!("radar source-quality trend requires at least one window");
    };
    let latest = rows
        .last()
        .expect("non-empty radar source-quality rows checked above");
    let run_count = rows
        .iter()
        .map(|row| row.run_id.as_str())
        .collect::<BTreeSet<_>>()
        .len() as i64;
    let raw_count = rows.iter().map(|row| row.raw_count).sum::<i64>();
    let accepted_count = rows.iter().map(|row| row.accepted_count).sum::<i64>();
    let failure_count = rows.iter().map(|row| row.failure_count).sum::<i64>();
    let non_healthy_count = rows.iter().filter(|row| row.status != "healthy").count() as i64;
    let average_score = weighted_optional_average(
        rows.iter()
            .filter_map(|row| row.average_score.map(|value| (value, row.raw_count))),
    );
    let signal_to_noise = if raw_count > 0 {
        Some(accepted_count as f64 / raw_count as f64)
    } else {
        weighted_optional_average(
            rows.iter()
                .filter_map(|row| row.signal_to_noise.map(|value| (value, row.raw_count))),
        )
    };
    let duplicate_rate = weighted_optional_average(
        rows.iter()
            .filter_map(|row| row.duplicate_rate.map(|value| (value, row.raw_count))),
    );
    let failure_rate = failure_count as f64 / rows.len().max(1) as f64;
    let quality_score =
        radar_source_quality_score(average_score, signal_to_noise, duplicate_rate, failure_rate);
    let trend_status = radar_source_quality_trend_status(rows);
    Ok(RadarSourceQualityTrend {
        source_kind,
        locator,
        window_count: rows.len() as i64,
        run_count,
        raw_count,
        accepted_count,
        failure_count,
        non_healthy_count,
        average_score,
        signal_to_noise,
        duplicate_rate,
        quality_score,
        latest_status: latest.status.clone(),
        trend_status,
        first_window_start: first.window_start.clone(),
        last_window_end: latest.window_end.clone(),
        latest_run_id: latest.run_id.clone(),
    })
}

pub(crate) fn weighted_optional_average<I>(values: I) -> Option<f64>
where
    I: IntoIterator<Item = (f64, i64)>,
{
    let mut weighted_sum = 0.0;
    let mut weight_sum = 0.0;
    for (value, weight) in values {
        let weight = weight.max(1) as f64;
        weighted_sum += value * weight;
        weight_sum += weight;
    }
    if weight_sum > 0.0 {
        Some(weighted_sum / weight_sum)
    } else {
        None
    }
}

pub(crate) fn radar_source_quality_score(
    average_score: Option<f64>,
    signal_to_noise: Option<f64>,
    duplicate_rate: Option<f64>,
    failure_rate: f64,
) -> f64 {
    let score_component = average_score.unwrap_or(0.0).clamp(0.0, 10.0) * 0.60;
    let signal_component = signal_to_noise.unwrap_or(0.0).clamp(0.0, 1.0) * 4.0;
    let duplicate_penalty = duplicate_rate.unwrap_or(0.0).clamp(0.0, 1.0) * 2.0;
    let failure_penalty = failure_rate.clamp(0.0, 1.0) * 2.0;
    (score_component + signal_component - duplicate_penalty - failure_penalty).clamp(0.0, 10.0)
}

pub(crate) fn radar_source_quality_trend_status(rows: &[RadarSourceQuality]) -> String {
    let Some(latest) = rows.last() else {
        return "insufficient_history".to_string();
    };
    if latest.status == "failed" {
        return "failing".to_string();
    }
    if rows.len() < 2 {
        return "insufficient_history".to_string();
    }
    let prior_rows = &rows[..rows.len() - 1];
    let prior_signal = weighted_optional_average(
        prior_rows
            .iter()
            .filter_map(|row| row.signal_to_noise.map(|value| (value, row.raw_count))),
    );
    let prior_score = weighted_optional_average(
        prior_rows
            .iter()
            .filter_map(|row| row.average_score.map(|value| (value, row.raw_count))),
    );
    let signal_delta = match (latest.signal_to_noise, prior_signal) {
        (Some(latest), Some(prior)) => Some(latest - prior),
        _ => None,
    };
    let score_delta = match (latest.average_score, prior_score) {
        (Some(latest), Some(prior)) => Some(latest - prior),
        _ => None,
    };
    if signal_delta.is_some_and(|delta| delta <= -0.25)
        || score_delta.is_some_and(|delta| delta <= -1.0)
    {
        "decaying".to_string()
    } else if signal_delta.is_some_and(|delta| delta >= 0.10)
        || score_delta.is_some_and(|delta| delta >= 0.50)
    {
        "improving".to_string()
    } else {
        "stable".to_string()
    }
}

pub(crate) fn radar_dedup_group_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<RadarDedupGroup> {
    let member_item_ids_json: String = row.get(4)?;
    Ok(RadarDedupGroup {
        id: row.get(0)?,
        run_id: row.get(1)?,
        dedup_kind: row.get(2)?,
        primary_item_id: row.get(3)?,
        member_item_ids: parse_json_string_vec_column(&member_item_ids_json, 4)?,
        reason: row.get(5)?,
        confidence: row.get(6)?,
        model_provider: row.get(7)?,
        cost_decision_id: row.get(8)?,
        created_at: row.get(9)?,
    })
}

pub(crate) fn radar_summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RadarSummary> {
    let item_ids_json: String = row.get(6)?;
    let source_card_ids_json: String = row.get(7)?;
    let metadata_json: String = row.get(9)?;
    Ok(RadarSummary {
        id: row.get(0)?,
        run_id: row.get(1)?,
        language: row.get(2)?,
        format: row.get(3)?,
        title: row.get(4)?,
        body_markdown: row.get(5)?,
        item_ids: parse_json_string_vec_column(&item_ids_json, 6)?,
        source_card_ids: parse_json_string_vec_column(&source_card_ids_json, 7)?,
        audit_status: row.get(8)?,
        metadata: parse_json_column(&metadata_json, 9)?,
        created_at: row.get(10)?,
    })
}

pub(crate) fn radar_delivery_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RadarDelivery> {
    Ok(RadarDelivery {
        id: row.get(0)?,
        run_id: row.get(1)?,
        summary_id: row.get(2)?,
        channel: row.get(3)?,
        recipient_ref: row.get(4)?,
        status: row.get(5)?,
        policy_decision_id: row.get(6)?,
        cost_decision_id: row.get(7)?,
        delivery_attempt_id: row.get(8)?,
        quiet_hours_deferred_until: row.get(9)?,
        idempotency_key: row.get(10)?,
        error: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

pub(crate) fn radar_schedule_tick_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<RadarScheduleTick> {
    Ok(RadarScheduleTick {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        tick_key: row.get(2)?,
        due_at: row.get(3)?,
        status: row.get(4)?,
        job_id: row.get(5)?,
        run_id: row.get(6)?,
        summary_id: row.get(7)?,
        delivery_id: row.get(8)?,
        error: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}
