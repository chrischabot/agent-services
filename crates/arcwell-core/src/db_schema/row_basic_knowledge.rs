use super::*;

pub(crate) fn normalized_memory_text(text: &str) -> String {
    text.to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c: char| c == '.' || c == '!' || c == '?' || c == '"' || c == '\'')
        .to_string()
}

pub(crate) fn parse_codex_swift_llm_wiki_sources(markdown: &str) -> ParsedWatchSources {
    let Some(start) = markdown.find("### 14.8 Seed watch list") else {
        return ParsedWatchSources {
            errors: vec!["llm-wiki.md missing section 14.8 seed watch list".to_string()],
            ..Default::default()
        };
    };
    let end = markdown[start + 1..]
        .find("\n### 14.9 ")
        .map(|offset| start + 1 + offset)
        .unwrap_or(markdown.len());
    let section = &markdown[start..end];
    let mut parsed = ParsedWatchSources::default();

    for (line_number, line) in section.lines().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || trimmed.contains("|---") {
            continue;
        }
        let cells: Vec<String> = trimmed
            .trim_matches('|')
            .split('|')
            .map(clean_markdown_table_cell)
            .collect();
        if cells.len() != 4 || cells[0].eq_ignore_ascii_case("handle") {
            continue;
        }
        let handle = cells[0].trim_matches('`').trim().to_string();
        let kind = cells[1].to_ascii_lowercase();
        let label = cells[2].clone();
        let cadence = cells[3].to_ascii_lowercase();
        let input = WatchSourceInput {
            source_kind: "github_owner".to_string(),
            locator: handle.clone(),
            label,
            cadence,
            status: "active".to_string(),
            metadata: json!({
                "origin": "codex-swift/llm-wiki.md",
                "github_kind": kind,
                "line": line_number + 1,
            }),
        };
        match validate_watch_source_input(&input) {
            Ok(()) => parsed.sources.push(input),
            Err(error) => {
                parsed.skipped += 1;
                parsed.errors.push(format!(
                    "llm-wiki.md line {} skipped: {error}",
                    line_number + 1
                ));
            }
        }
    }

    parsed
}

pub(crate) fn parse_codex_swift_restore_script(script: &str) -> ParsedWatchSources {
    let mut parsed = ParsedWatchSources::default();
    for (array_name, source_kind, cadence) in [
        ("FEEDS", "rss", "warm"),
        ("GITHUB", "github_owner", "warm"),
        ("BLOGS", "blog", "warm"),
        ("ARXIV", "arxiv_query", "warm"),
    ] {
        match parse_shell_array(script, array_name) {
            Ok(values) => {
                for value in values {
                    let input = WatchSourceInput {
                        source_kind: source_kind.to_string(),
                        locator: value.clone(),
                        label: restore_source_label(source_kind, &value),
                        cadence: cadence.to_string(),
                        status: "active".to_string(),
                        metadata: json!({
                            "origin": "codex-swift/scripts/wiki-sources-restore.sh",
                            "array": array_name,
                        }),
                    };
                    match validate_watch_source_input(&input) {
                        Ok(()) => parsed.sources.push(input),
                        Err(error) => {
                            parsed.skipped += 1;
                            parsed.errors.push(format!(
                                "wiki-sources-restore.sh {array_name} `{value}` skipped: {error}"
                            ));
                        }
                    }
                }
            }
            Err(error) => parsed.errors.push(error.to_string()),
        }
    }
    parsed
}

pub(crate) fn parse_shell_array(script: &str, array_name: &str) -> Result<Vec<String>> {
    let needle = format!("{array_name}=(");
    let Some(start) = script.find(&needle) else {
        bail!("wiki-sources-restore.sh missing {array_name} array");
    };
    let mut values = Vec::new();
    let mut in_array = false;
    for line in script[start..].lines() {
        let mut current = line.trim();
        if !in_array {
            let Some(after) = current.strip_prefix(&needle) else {
                continue;
            };
            current = after;
            in_array = true;
        }
        let closes = current.contains(')');
        current = current.split(')').next().unwrap_or(current);
        current = current.split('#').next().unwrap_or(current).trim();
        values.extend(parse_shell_array_values(current));
        if closes {
            break;
        }
    }
    Ok(values)
}

pub(crate) fn parse_shell_array_values(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    for ch in line.chars() {
        match ch {
            '"' => {
                if in_quote {
                    if !current.trim().is_empty() {
                        values.push(current.trim().to_string());
                    }
                    current.clear();
                }
                in_quote = !in_quote;
            }
            ch if ch.is_whitespace() && !in_quote => {
                if !current.trim().is_empty() {
                    values.push(current.trim().to_string());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        values.push(current.trim().to_string());
    }
    values
}

pub(crate) fn restore_source_label(source_kind: &str, locator: &str) -> String {
    match source_kind {
        "github_owner" => format!("GitHub: {locator}"),
        "arxiv_query" => format!("arXiv: {locator}"),
        _ => locator.to_string(),
    }
}

pub(crate) fn clean_markdown_table_cell(cell: &str) -> String {
    cell.trim()
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

pub(crate) fn rows<T>(iter: impl Iterator<Item = rusqlite::Result<T>>) -> Result<Vec<T>> {
    iter.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub(crate) fn profile_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProfileItem> {
    Ok(ProfileItem {
        key: row.get(0)?,
        value: row.get(1)?,
        sensitivity: row.get(2)?,
        source: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

pub(crate) fn memory_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryItem> {
    Ok(MemoryItem {
        id: row.get(0)?,
        text: row.get(1)?,
        kind: row.get(2)?,
        sensitivity: row.get(3)?,
        source: row.get(4)?,
        user_id: row.get(5)?,
        confidence: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

pub(crate) fn candidate_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Candidate> {
    let metadata_json: String = row.get(11)?;
    let applied_result_json: Option<String> = row.get(12)?;
    Ok(Candidate {
        id: row.get(0)?,
        target: row.get(1)?,
        kind: row.get(2)?,
        content: row.get(3)?,
        sensitivity: row.get(4)?,
        source_ref: row.get(5)?,
        status: row.get(6)?,
        created_at: row.get(7)?,
        operation: row.get(8)?,
        memory_id: row.get(9)?,
        user_id: row.get(10)?,
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
        applied_result: applied_result_json
            .as_deref()
            .and_then(|value| serde_json::from_str(value).ok()),
        applied_at: row.get(13)?,
        rejected_reason: row.get(14)?,
    })
}

pub(crate) fn import_run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ImportRunRecord> {
    let metadata_json: String = row.get(12)?;
    let conversations_seen: i64 = row.get(5)?;
    let conversations_sampled: i64 = row.get(6)?;
    let candidates_seen: i64 = row.get(7)?;
    let candidates_sampled: i64 = row.get(8)?;
    let candidates_written: i64 = row.get(9)?;
    let duplicates_suppressed: i64 = row.get(10)?;
    Ok(ImportRunRecord {
        id: row.get(0)?,
        source_kind: row.get(1)?,
        source_path: row.get(2)?,
        mode: row.get(3)?,
        status: row.get(4)?,
        conversations_seen: conversations_seen.max(0) as usize,
        conversations_sampled: conversations_sampled.max(0) as usize,
        candidates_seen: candidates_seen.max(0) as usize,
        candidates_sampled: candidates_sampled.max(0) as usize,
        candidates_written: candidates_written.max(0) as usize,
        duplicates_suppressed: duplicates_suppressed.max(0) as usize,
        error: row.get(11)?,
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
        started_at: row.get(13)?,
        finished_at: row.get(14)?,
    })
}

pub(crate) fn memory_lifecycle_event_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<MemoryLifecycleEvent> {
    let result_json: String = row.get(6)?;
    Ok(MemoryLifecycleEvent {
        id: row.get(0)?,
        event_type: row.get(1)?,
        hook: row.get(2)?,
        user_id: row.get(3)?,
        source_ref: row.get(4)?,
        input: row.get(5)?,
        result: serde_json::from_str(&result_json).unwrap_or_else(|_| json!({})),
        status: row.get(7)?,
        created_at: row.get(8)?,
    })
}

pub(crate) fn memory_decision_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<MemoryDecisionLedgerEntry> {
    let metadata_json: String = row.get(9)?;
    Ok(MemoryDecisionLedgerEntry {
        id: row.get(0)?,
        user_id: row.get(1)?,
        source_ref: row.get(2)?,
        observation: row.get(3)?,
        operation: row.get(4)?,
        memory_id: row.get(5)?,
        candidate_id: row.get(6)?,
        confidence: row.get(7)?,
        reason: row.get(8)?,
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(10)?,
    })
}

pub(crate) fn memory_forget_tombstone_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<MemoryForgetTombstone> {
    Ok(MemoryForgetTombstone {
        id: row.get(0)?,
        user_id_hash: row.get(1)?,
        provider: row.get(2)?,
        provider_memories_deleted: row.get::<_, i64>(3)? as usize,
        candidates_deleted: row.get::<_, i64>(4)? as usize,
        compatibility_memories_deleted: row.get::<_, i64>(5)? as usize,
        lifecycle_events_deleted: row.get::<_, i64>(6)? as usize,
        decision_ledger_deleted: row.get::<_, i64>(7)? as usize,
        policy: row.get(8)?,
        created_at: row.get(9)?,
    })
}

pub(crate) fn secret_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SecretRef> {
    Ok(SecretRef {
        name: row.get(0)?,
        location: row.get(1)?,
        scope: row.get(2)?,
        expires_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

pub(crate) fn secret_value_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SecretValue> {
    Ok(SecretValue {
        name: row.get(0)?,
        scope: row.get(1)?,
        provider: row.get(2)?,
        expires_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

pub(crate) fn wiki_summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WikiPageSummary> {
    Ok(WikiPageSummary {
        id: row.get(0)?,
        title: row.get(1)?,
        path: row.get(2)?,
        content_sha256: row.get(3)?,
        source: row.get(4)?,
        status: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

pub(crate) fn wiki_page_metadata_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WikiPage> {
    Ok(WikiPage {
        id: row.get(0)?,
        title: row.get(1)?,
        path: row.get(2)?,
        content_sha256: row.get(3)?,
        source: row.get(4)?,
        status: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        content: String::new(),
    })
}

pub(crate) fn source_card_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SourceCard> {
    let claims_json: String = row.get(6)?;
    let claims = serde_json::from_str(&claims_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let metadata_json: String = row.get(10)?;
    Ok(SourceCard {
        id: row.get(0)?,
        title: row.get(1)?,
        url: row.get(2)?,
        source_type: row.get(3)?,
        provider: row.get(4)?,
        summary: row.get(5)?,
        claims,
        retrieved_at: row.get(7)?,
        wiki_page_id: row.get(8)?,
        content_sha256: row.get(9)?,
        metadata: parse_json_column(&metadata_json, 10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

pub(crate) fn knowledge_event_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<KnowledgeEvent> {
    let metadata_json: String = row.get(11)?;
    Ok(KnowledgeEvent {
        id: row.get(0)?,
        event_type: row.get(1)?,
        status: row.get(2)?,
        title: row.get(3)?,
        canonical_key: row.get(4)?,
        primary_entity_key: row.get(5)?,
        event_time: row.get(6)?,
        summary: row.get(7)?,
        first_seen_at: row.get(8)?,
        last_seen_at: row.get(9)?,
        confidence: row.get(10)?,
        metadata: parse_json_column(&metadata_json, 11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

pub(crate) fn knowledge_event_source_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<KnowledgeEventSource> {
    let metadata_json: String = row.get(6)?;
    Ok(KnowledgeEventSource {
        id: row.get(0)?,
        event_id: row.get(1)?,
        source_card_id: row.get(2)?,
        role: row.get(3)?,
        confidence: row.get(4)?,
        claim_summary: row.get(5)?,
        metadata: parse_json_column(&metadata_json, 6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

pub(crate) fn knowledge_cluster_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<KnowledgeCluster> {
    let source_card_ids_json: String = row.get(3)?;
    let event_ids_json: String = row.get(4)?;
    let duplicate_groups_json: String = row.get(11)?;
    let metadata_json: String = row.get(12)?;
    Ok(KnowledgeCluster {
        id: row.get(0)?,
        topic: row.get(1)?,
        status: row.get(2)?,
        source_card_ids: parse_json_string_vec_column(&source_card_ids_json, 3)?,
        event_ids: parse_json_string_vec_column(&event_ids_json, 4)?,
        first_seen_at: row.get(5)?,
        last_seen_at: row.get(6)?,
        novelty_score: row.get(7)?,
        momentum_score: row.get(8)?,
        stale_score: row.get(9)?,
        reason: row.get(10)?,
        duplicate_groups: parse_json_column(&duplicate_groups_json, 11)?,
        metadata: parse_json_column(&metadata_json, 12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

pub(crate) fn knowledge_editorial_decision_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<KnowledgeEditorialDecision> {
    let source_card_ids_json: String = row.get(6)?;
    let quality_findings_json: String = row.get(8)?;
    let metadata_json: String = row.get(9)?;
    Ok(KnowledgeEditorialDecision {
        id: row.get(0)?,
        cluster_id: row.get(1)?,
        decision: row.get(2)?,
        status: row.get(3)?,
        wiki_page_id: row.get(4)?,
        digest_candidate_id: row.get(5)?,
        source_card_ids: parse_json_string_vec_column(&source_card_ids_json, 6)?,
        reason: row.get(7)?,
        quality_findings: parse_json_string_vec_column(&quality_findings_json, 8)?,
        metadata: parse_json_column(&metadata_json, 9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

pub(crate) fn knowledge_report_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<KnowledgeReport> {
    let source_card_ids_json: String = row.get(5)?;
    let quality_findings_json: String = row.get(6)?;
    let metadata_json: String = row.get(7)?;
    Ok(KnowledgeReport {
        id: row.get(0)?,
        cluster_id: row.get(1)?,
        title: row.get(2)?,
        body_markdown: row.get(3)?,
        status: row.get(4)?,
        source_card_ids: parse_json_string_vec_column(&source_card_ids_json, 5)?,
        quality_findings: parse_json_string_vec_column(&quality_findings_json, 6)?,
        metadata: parse_json_column(&metadata_json, 7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub(crate) fn knowledge_entity_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<KnowledgeEntity> {
    let aliases_json: String = row.get(4)?;
    let source_card_ids_json: String = row.get(6)?;
    let metadata_json: String = row.get(9)?;
    Ok(KnowledgeEntity {
        id: row.get(0)?,
        entity_type: row.get(1)?,
        name: row.get(2)?,
        canonical_key: row.get(3)?,
        aliases: parse_json_string_vec_column(&aliases_json, 4)?,
        homepage_url: row.get(5)?,
        source_card_ids: parse_json_string_vec_column(&source_card_ids_json, 6)?,
        wiki_page_id: row.get(7)?,
        confidence: row.get(8)?,
        metadata: parse_json_column(&metadata_json, 9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

pub(crate) fn knowledge_relation_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<KnowledgeRelation> {
    let source_card_ids_json: String = row.get(7)?;
    let metadata_json: String = row.get(10)?;
    Ok(KnowledgeRelation {
        id: row.get(0)?,
        relation_key: row.get(1)?,
        relation_type: row.get(2)?,
        subject_entity_id: row.get(3)?,
        object_entity_id: row.get(4)?,
        event_id: row.get(5)?,
        cluster_id: row.get(6)?,
        source_card_ids: parse_json_string_vec_column(&source_card_ids_json, 7)?,
        confidence: row.get(8)?,
        reason: row.get(9)?,
        metadata: parse_json_column(&metadata_json, 10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

pub(crate) fn knowledge_adapter_run_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<KnowledgeAdapterRun> {
    let source_card_ids_json: String = row.get(12)?;
    let metadata_json: String = row.get(17)?;
    Ok(KnowledgeAdapterRun {
        id: row.get(0)?,
        job_id: row.get(1)?,
        adapter_kind: row.get(2)?,
        provider: row.get(3)?,
        source_kind: row.get(4)?,
        locator: row.get(5)?,
        status: row.get(6)?,
        error_kind: row.get(7)?,
        error: row.get(8)?,
        cursor_key: row.get(9)?,
        cursor_before: row.get(10)?,
        cursor_after: row.get(11)?,
        source_card_ids: parse_json_string_vec_column(&source_card_ids_json, 12)?,
        raw_count: row.get(13)?,
        accepted_count: row.get(14)?,
        rejected_count: row.get(15)?,
        duplicate_count: row.get(16)?,
        metadata: parse_json_column(&metadata_json, 17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

pub(crate) fn knowledge_entity_resolution_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<KnowledgeEntityResolution> {
    let evidence_json: String = row.get(8)?;
    let source_card_ids_json: String = row.get(9)?;
    Ok(KnowledgeEntityResolution {
        id: row.get(0)?,
        left_entity_id: row.get(1)?,
        right_entity_id: row.get(2)?,
        status: row.get(3)?,
        decision: row.get(4)?,
        confidence: row.get(5)?,
        resolver: row.get(6)?,
        reason: row.get(7)?,
        evidence_json: parse_json_column(&evidence_json, 8)?,
        source_card_ids: parse_json_string_vec_column(&source_card_ids_json, 9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

pub(crate) fn x_knowledge_cluster_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<XKnowledgeCluster> {
    let source_card_ids_json: String = row.get(3)?;
    let radar_item_ids_json: String = row.get(5)?;
    let metadata_json: String = row.get(12)?;
    Ok(XKnowledgeCluster {
        id: row.get(0)?,
        topic: row.get(1)?,
        status: row.get(2)?,
        source_card_ids: parse_json_string_vec_column(&source_card_ids_json, 3)?,
        radar_run_id: row.get(4)?,
        radar_item_ids: parse_json_string_vec_column(&radar_item_ids_json, 5)?,
        first_seen_at: row.get(6)?,
        last_seen_at: row.get(7)?,
        novelty_score: row.get(8)?,
        momentum_score: row.get(9)?,
        stale_score: row.get(10)?,
        reason: row.get(11)?,
        metadata: parse_json_column(&metadata_json, 12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

pub(crate) fn x_editorial_decision_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<XEditorialDecision> {
    let source_card_ids_json: String = row.get(6)?;
    let quality_findings_json: String = row.get(8)?;
    let metadata_json: String = row.get(9)?;
    Ok(XEditorialDecision {
        id: row.get(0)?,
        cluster_id: row.get(1)?,
        decision: row.get(2)?,
        status: row.get(3)?,
        wiki_page_id: row.get(4)?,
        digest_candidate_id: row.get(5)?,
        source_card_ids: parse_json_string_vec_column(&source_card_ids_json, 6)?,
        reason: row.get(7)?,
        quality_findings: parse_json_string_vec_column(&quality_findings_json, 8)?,
        metadata: parse_json_column(&metadata_json, 9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

pub(crate) fn research_source_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchSource> {
    let metadata_json: String = row.get(15)?;
    Ok(ResearchSource {
        id: row.get(0)?,
        url: row.get(1)?,
        local_ref: row.get(2)?,
        title: row.get(3)?,
        source_family: row.get(4)?,
        source_type: row.get(5)?,
        provider: row.get(6)?,
        author: row.get(7)?,
        published_at: row.get(8)?,
        language: row.get(9)?,
        priority: row.get(10)?,
        reason: row.get(11)?,
        canonical_key: row.get(12)?,
        fetch_status: row.get(13)?,
        read_depth: row.get(14)?,
        metadata: parse_json_column(&metadata_json, 15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}

pub(crate) fn research_run_source_link_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchRunSourceLink> {
    Ok(ResearchRunSourceLink {
        id: row.get(0)?,
        run_id: row.get(1)?,
        source_id: row.get(2)?,
        source_card_id: row.get(3)?,
        triage_status: row.get(4)?,
        read_depth: row.get(5)?,
        notes: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

pub(crate) fn research_role_run_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchRoleRun> {
    let input_artifact_ids_json: String = row.get(10)?;
    Ok(ResearchRoleRun {
        id: row.get(0)?,
        run_id: row.get(1)?,
        role: row.get(2)?,
        host: row.get(3)?,
        host_thread_id: row.get(4)?,
        host_subagent_id: row.get(5)?,
        tool_surface: row.get(6)?,
        prompt_version: row.get(7)?,
        prompt_hash: row.get(8)?,
        execution_mode: row.get(9)?,
        input_artifact_ids: parse_json_string_vec_column(&input_artifact_ids_json, 10)?,
        output_artifact_id: row.get(11)?,
        status: row.get(12)?,
        started_at: row.get(13)?,
        finished_at: row.get(14)?,
        error_kind: row.get(15)?,
        error_message_redacted: row.get(16)?,
    })
}

pub(crate) fn research_artifact_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ResearchArtifact> {
    let metadata_json: String = row.get(7)?;
    Ok(ResearchArtifact {
        id: row.get(0)?,
        run_id: row.get(1)?,
        role_run_id: row.get(2)?,
        artifact_type: row.get(3)?,
        title: row.get(4)?,
        body: row.get(5)?,
        body_sha256: row.get(6)?,
        metadata: parse_json_column(&metadata_json, 7)?,
        created_at: row.get(8)?,
    })
}
