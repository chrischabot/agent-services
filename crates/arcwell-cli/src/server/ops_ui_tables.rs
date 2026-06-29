use super::*;

pub(crate) fn render_ops_summary(snapshot: &OpsSnapshot, score: &OpsHealthScore) -> String {
    let mut html = String::new();
    html.push_str(
        "<section class=\"section\"><h2>Summary</h2><section class=\"grid summary-grid\">",
    );
    for (label, value) in [
        ("Health", format!("{} ({})", score.score, score.label)),
        (
            "Queue statuses",
            summarize_counts(snapshot.jobs.iter().map(|job| job.status.as_str())),
        ),
        (
            "Job kinds",
            summarize_counts(snapshot.jobs.iter().map(|job| job.kind.as_str())),
        ),
        (
            "Edge statuses",
            summarize_counts(
                snapshot
                    .edge_events
                    .iter()
                    .map(|event| event.status.as_str()),
            ),
        ),
        (
            "Edge sources",
            summarize_counts(
                snapshot
                    .edge_events
                    .iter()
                    .map(|event| event.source.as_str()),
            ),
        ),
        (
            "Source statuses",
            summarize_counts(
                snapshot
                    .source_health
                    .iter()
                    .map(|source| source.status.as_str()),
            ),
        ),
        (
            "Radar source quality",
            summarize_counts(
                snapshot
                    .radar_source_quality
                    .iter()
                    .map(|quality| quality.status.as_str()),
            ),
        ),
        ("Radar run scores", summarize_radar_run_scores(snapshot)),
        ("Job roles", summarize_job_ops(snapshot)),
        (
            "Job source health",
            summarize_usize_count_map(&snapshot.job_hunting.source_health_counts),
        ),
        (
            "Job privacy",
            summarize_usize_count_map(&snapshot.job_hunting.privacy_decision_counts),
        ),
        (
            "Job applications",
            summarize_usize_count_map(&snapshot.job_hunting.application_status_counts),
        ),
        (
            "Credential statuses",
            summarize_counts(
                snapshot
                    .secret_health
                    .iter()
                    .map(|secret| secret.status.as_str()),
            ),
        ),
        ("X drift", summarize_x_drift(&snapshot.x_stats)),
        (
            "X sync statuses",
            summarize_count_map(&snapshot.x_stats.sync_runs_by_status),
        ),
        (
            "X source statuses",
            summarize_count_map(&snapshot.x_stats.source_health_by_status),
        ),
        (
            "X portable export",
            summarize_x_portable_export(&snapshot.x_stats),
        ),
        (
            "X digest queue",
            summarize_x_digest_queue(&snapshot.x_stats),
        ),
        ("X knowledge", summarize_x_knowledge(snapshot)),
    ] {
        html.push_str(&format!(
            "<div class=\"metric\"><span>{}</span><b>{}</b></div>",
            html_escape(label),
            html_escape(&value)
        ));
    }
    html.push_str("</section>");
    if !score.issues.is_empty() {
        html.push_str("<ul>");
        for issue in score.issues.iter().take(8) {
            html.push_str(&format!("<li class=\"warn\">{}</li>", html_escape(issue)));
        }
        html.push_str("</ul>");
    }
    html.push_str("</section>");
    html
}

pub(crate) fn render_ops_detail(snapshot: &OpsSnapshot, detail: &str) -> String {
    let Some((kind, id)) = detail.split_once(':') else {
        return format!(
            "<section class=\"section detail\"><h2>Detail</h2><p class=\"bad\">Unsupported detail target: {}</p></section>",
            html_escape(detail)
        );
    };
    let value = match kind {
        "job" => snapshot
            .jobs
            .iter()
            .find(|job| job.id == id)
            .and_then(|job| serde_json::to_value(job).ok()),
        "edge" => snapshot
            .edge_events
            .iter()
            .find(|event| event.id == id)
            .and_then(|event| serde_json::to_value(event).ok()),
        "secret" => snapshot
            .secret_health
            .iter()
            .find(|secret| secret.name == id)
            .and_then(|secret| serde_json::to_value(secret).ok()),
        "radar-run" => snapshot
            .radar_runs
            .iter()
            .find(|run| run.id == id)
            .and_then(|run| serde_json::to_value(run).ok()),
        "x-cluster" => snapshot
            .x_knowledge_clusters
            .iter()
            .find(|cluster| cluster.id == id)
            .and_then(|cluster| serde_json::to_value(cluster).ok()),
        "x-editorial" => snapshot
            .x_editorial_decisions
            .iter()
            .find(|decision| decision.id == id)
            .and_then(|decision| serde_json::to_value(decision).ok()),
        _ => None,
    };
    match value {
        Some(value) => format!(
            "<section class=\"section detail\"><h2>Detail: {}</h2><pre>{}</pre></section>",
            html_escape(detail),
            html_escape(&json_cell(&value))
        ),
        None => format!(
            "<section class=\"section detail\"><h2>Detail</h2><p class=\"bad\">No matching ops detail for {}</p></section>",
            html_escape(detail)
        ),
    }
}

pub(crate) fn job_lineage_summary(job: &arcwell_core::WikiJob) -> String {
    let Some(lineage) = job.input_json.get("lineage").and_then(Value::as_object) else {
        return String::new();
    };
    let mut parts = Vec::new();
    if let Some(trigger) = lineage.get("trigger").and_then(Value::as_str) {
        parts.push(format!("trigger:{trigger}"));
    }
    if let Some(parent_kind) = lineage.get("parent_kind").and_then(Value::as_str) {
        parts.push(format!("parent:{parent_kind}"));
    }
    if let Some(parent_job_id) = lineage.get("parent_job_id").and_then(Value::as_str) {
        parts.push(format!("parent_job:{}", short_id(parent_job_id)));
    }
    if let Some(watch_source_key) = lineage.get("watch_source_key").and_then(Value::as_str) {
        parts.push(format!("source:{watch_source_key}"));
    }
    if let Some(cluster_id) = lineage.get("cluster_id").and_then(Value::as_str) {
        parts.push(format!("cluster:{}", short_id(cluster_id)));
    }
    if let Some(source_card_count) = lineage.get("source_card_count").and_then(Value::as_u64) {
        parts.push(format!("source_cards:{source_card_count}"));
    }
    if let Some(task_count) = lineage
        .get("investigation_task_count")
        .and_then(Value::as_u64)
    {
        parts.push(format!("tasks:{task_count}"));
    }
    if parts.is_empty() {
        json_cell(&Value::Object(lineage.clone()))
    } else {
        parts.join(" ")
    }
}

pub(crate) fn filtered_jobs<'a>(
    snapshot: &'a OpsSnapshot,
    options: &OpsUiOptions,
) -> Vec<&'a arcwell_core::WikiJob> {
    let mut jobs = snapshot
        .jobs
        .iter()
        .filter(|job| {
            let lineage_summary = job_lineage_summary(job);
            matches_status(&job.status, options)
                && matches_query(
                    options,
                    [
                        job.id.as_str(),
                        job.kind.as_str(),
                        job.status.as_str(),
                        job.worker_id.as_deref().unwrap_or_default(),
                        job.error.as_deref().unwrap_or_default(),
                        lineage_summary.as_str(),
                    ],
                )
        })
        .collect::<Vec<_>>();
    jobs.sort_by(|left, right| match normalized_sort(options) {
        "updated_asc" => left.updated_at.cmp(&right.updated_at),
        "status" => left
            .status
            .cmp(&right.status)
            .then(left.updated_at.cmp(&right.updated_at)),
        "kind" => left
            .kind
            .cmp(&right.kind)
            .then(right.updated_at.cmp(&left.updated_at)),
        "attempts_desc" => right
            .attempts
            .cmp(&left.attempts)
            .then(right.updated_at.cmp(&left.updated_at)),
        _ => right.updated_at.cmp(&left.updated_at),
    });
    jobs
}

pub(crate) fn filtered_edge_events<'a>(
    snapshot: &'a OpsSnapshot,
    options: &OpsUiOptions,
) -> Vec<&'a arcwell_core::EdgeEvent> {
    let mut events = snapshot
        .edge_events
        .iter()
        .filter(|event| {
            matches_status(&event.status, options)
                && matches_query(
                    options,
                    [
                        event.id.as_str(),
                        event.source.as_str(),
                        event.idempotency_key.as_str(),
                        event.status.as_str(),
                        event.error.as_deref().unwrap_or_default(),
                    ],
                )
        })
        .collect::<Vec<_>>();
    events.sort_by(|left, right| match normalized_sort(options) {
        "updated_asc" => left.updated_at.cmp(&right.updated_at),
        "status" => left
            .status
            .cmp(&right.status)
            .then(left.updated_at.cmp(&right.updated_at)),
        "kind" => left
            .source
            .cmp(&right.source)
            .then(right.updated_at.cmp(&left.updated_at)),
        "attempts_desc" => right
            .attempts
            .cmp(&left.attempts)
            .then(right.updated_at.cmp(&left.updated_at)),
        _ => right.updated_at.cmp(&left.updated_at),
    });
    events
}

pub(crate) fn filtered_watch_sources<'a>(
    snapshot: &'a OpsSnapshot,
    options: &OpsUiOptions,
) -> Vec<&'a arcwell_core::WatchSource> {
    snapshot
        .watch_sources
        .iter()
        .filter(|source| {
            matches_status(&source.status, options)
                && matches_query(
                    options,
                    [
                        source.source_kind.as_str(),
                        source.label.as_str(),
                        source.locator.as_str(),
                        source.cadence.as_str(),
                        source.status.as_str(),
                    ],
                )
        })
        .collect()
}

pub(crate) fn filtered_source_health<'a>(
    snapshot: &'a OpsSnapshot,
    options: &OpsUiOptions,
) -> Vec<&'a arcwell_core::SourceHealth> {
    snapshot
        .source_health
        .iter()
        .filter(|health| {
            matches_status(&health.status, options)
                && matches_query(
                    options,
                    [
                        health.provider.as_str(),
                        health.source_kind.as_str(),
                        health.locator.as_str(),
                        health.status.as_str(),
                        health.last_error.as_deref().unwrap_or_default(),
                    ],
                )
        })
        .collect()
}

pub(crate) fn filtered_x_knowledge_clusters<'a>(
    snapshot: &'a OpsSnapshot,
    options: &OpsUiOptions,
) -> Vec<&'a arcwell_core::XKnowledgeCluster> {
    let mut rows = snapshot
        .x_knowledge_clusters
        .iter()
        .filter(|cluster| {
            matches_status(&cluster.status, options)
                && matches_query(
                    options,
                    [
                        cluster.id.as_str(),
                        cluster.topic.as_str(),
                        cluster.status.as_str(),
                        cluster.reason.as_str(),
                        cluster.radar_run_id.as_deref().unwrap_or_default(),
                        cluster
                            .metadata
                            .get("cluster_key")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                    ],
                )
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| match normalized_sort(options) {
        "updated_asc" => left.updated_at.cmp(&right.updated_at),
        "status" => left
            .status
            .cmp(&right.status)
            .then(left.updated_at.cmp(&right.updated_at)),
        "kind" => left.topic.cmp(&right.topic),
        _ => right.updated_at.cmp(&left.updated_at),
    });
    rows
}

pub(crate) fn filtered_x_editorial_decisions<'a>(
    snapshot: &'a OpsSnapshot,
    options: &OpsUiOptions,
) -> Vec<&'a arcwell_core::XEditorialDecision> {
    let mut rows = snapshot
        .x_editorial_decisions
        .iter()
        .filter(|decision| {
            matches_status(&decision.status, options)
                && matches_query(
                    options,
                    [
                        decision.id.as_str(),
                        decision.cluster_id.as_str(),
                        decision.decision.as_str(),
                        decision.status.as_str(),
                        decision.reason.as_str(),
                        decision.wiki_page_id.as_deref().unwrap_or_default(),
                        decision.digest_candidate_id.as_deref().unwrap_or_default(),
                    ],
                )
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| match normalized_sort(options) {
        "updated_asc" => left.updated_at.cmp(&right.updated_at),
        "status" => left
            .status
            .cmp(&right.status)
            .then(left.updated_at.cmp(&right.updated_at)),
        "kind" => left.decision.cmp(&right.decision),
        _ => right.updated_at.cmp(&left.updated_at),
    });
    rows
}

pub(crate) fn filtered_radar_source_quality<'a>(
    snapshot: &'a OpsSnapshot,
    options: &OpsUiOptions,
) -> Vec<&'a arcwell_core::RadarSourceQuality> {
    let mut rows = snapshot
        .radar_source_quality
        .iter()
        .filter(|quality| {
            matches_status(&quality.status, options)
                && matches_query(
                    options,
                    [
                        quality.run_id.as_str(),
                        quality.source_kind.as_str(),
                        quality.locator.as_str(),
                        quality.status.as_str(),
                    ],
                )
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| match normalized_sort(options) {
        "updated_asc" => left.created_at.cmp(&right.created_at),
        "status" => left
            .status
            .cmp(&right.status)
            .then(left.created_at.cmp(&right.created_at)),
        "kind" => left
            .source_kind
            .cmp(&right.source_kind)
            .then(left.locator.cmp(&right.locator)),
        _ => right.created_at.cmp(&left.created_at),
    });
    rows
}

pub(crate) fn filtered_radar_runs<'a>(
    snapshot: &'a OpsSnapshot,
    options: &OpsUiOptions,
) -> Vec<&'a RadarRun> {
    let mut rows = snapshot
        .radar_runs
        .iter()
        .filter(|run| {
            matches_status(&run.status, options)
                && matches_query(
                    options,
                    [
                        run.id.as_str(),
                        run.profile_id.as_str(),
                        run.status.as_str(),
                        run.stage.as_str(),
                        run.error.as_deref().unwrap_or_default(),
                    ],
                )
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| match normalized_sort(options) {
        "updated_asc" => left.updated_at.cmp(&right.updated_at),
        "status" => left
            .status
            .cmp(&right.status)
            .then(left.updated_at.cmp(&right.updated_at)),
        "kind" => left
            .profile_id
            .cmp(&right.profile_id)
            .then(left.updated_at.cmp(&right.updated_at)),
        _ => right.updated_at.cmp(&left.updated_at),
    });
    rows
}

pub(crate) fn filtered_radar_deliveries<'a>(
    snapshot: &'a OpsSnapshot,
    options: &OpsUiOptions,
) -> Vec<&'a arcwell_core::RadarDelivery> {
    let mut rows = snapshot
        .radar_deliveries
        .iter()
        .filter(|delivery| {
            matches_status(&delivery.status, options)
                && matches_query(
                    options,
                    [
                        delivery.run_id.as_str(),
                        delivery.summary_id.as_str(),
                        delivery.channel.as_str(),
                        delivery.recipient_ref.as_str(),
                        delivery.status.as_str(),
                        delivery.delivery_attempt_id.as_deref().unwrap_or_default(),
                        delivery.error.as_deref().unwrap_or_default(),
                    ],
                )
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| match normalized_sort(options) {
        "updated_asc" => left.updated_at.cmp(&right.updated_at),
        "status" => left
            .status
            .cmp(&right.status)
            .then(left.updated_at.cmp(&right.updated_at)),
        "kind" => left
            .channel
            .cmp(&right.channel)
            .then(left.recipient_ref.cmp(&right.recipient_ref)),
        _ => right.updated_at.cmp(&left.updated_at),
    });
    rows
}

pub(crate) fn filtered_secret_health<'a>(
    snapshot: &'a OpsSnapshot,
    options: &OpsUiOptions,
) -> Vec<&'a arcwell_core::SecretHealth> {
    snapshot
        .secret_health
        .iter()
        .filter(|secret| {
            matches_status(&secret.status, options)
                && matches_query(
                    options,
                    [
                        secret.name.as_str(),
                        secret.scope.as_str(),
                        secret.provider.as_deref().unwrap_or_default(),
                        secret.source.as_str(),
                        secret.status.as_str(),
                    ],
                )
        })
        .collect()
}

pub(crate) fn render_edge_event_action(
    event: &arcwell_core::EdgeEvent,
    csrf_token: Option<&str>,
    controls_enabled: bool,
) -> String {
    if !is_dead_letterable_edge_status(&event.status) {
        return "No safe action for this status.".to_string();
    }
    let Some(csrf_token) = csrf_token else {
        return "Open /ops/ui from the authenticated HTTP server to use controls.".to_string();
    };
    if !controls_enabled {
        return "Disabled: start server with ARCWELL_HTTP_AUTH_TOKEN to enable mutations."
            .to_string();
    }
    format!(
        "<div class=\"actions\"><form method=\"post\" action=\"/ops/actions/edge-events/dead-letter\"><input type=\"hidden\" name=\"csrf_token\" value=\"{}\"><input type=\"hidden\" name=\"edge_event_id\" value=\"{}\"><input type=\"hidden\" name=\"idempotency_key\" value=\"{}\"><input name=\"reason\" value=\"manual ops review\" maxlength=\"1000\"><button class=\"danger\" type=\"submit\">Dead-letter</button></form></div>",
        html_escape(csrf_token),
        html_escape(&event.id),
        html_escape(&format!("ops-ui-{}", event.id))
    )
}

pub(crate) fn matches_status(status: &str, options: &OpsUiOptions) -> bool {
    options
        .status
        .as_deref()
        .map(|filter| {
            status
                .to_ascii_lowercase()
                .contains(&filter.to_ascii_lowercase())
        })
        .unwrap_or(true)
}

pub(crate) fn matches_query<'a>(
    options: &OpsUiOptions,
    values: impl IntoIterator<Item = &'a str>,
) -> bool {
    let Some(query) = options.q.as_deref() else {
        return true;
    };
    let query = query.to_ascii_lowercase();
    values
        .into_iter()
        .any(|value| value.to_ascii_lowercase().contains(&query))
}

pub(crate) fn normalized_sort(options: &OpsUiOptions) -> &str {
    match options.sort.as_str() {
        "updated_asc" | "status" | "kind" | "attempts_desc" => options.sort.as_str(),
        _ => "updated_desc",
    }
}

pub(crate) fn summarize_counts<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for value in values {
        *counts.entry(value.to_string()).or_default() += 1;
    }
    if counts.is_empty() {
        return "none".to_string();
    }
    counts
        .into_iter()
        .map(|(key, count)| format!("{key}:{count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn summarize_count_map(counts: &BTreeMap<String, i64>) -> String {
    let summary = counts
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(key, count)| format!("{key}:{count}"))
        .collect::<Vec<_>>();
    if summary.is_empty() {
        "none".to_string()
    } else {
        summary.join(", ")
    }
}

pub(crate) fn summarize_usize_count_map(counts: &BTreeMap<String, usize>) -> String {
    let summary = counts
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(key, count)| format!("{key}:{count}"))
        .collect::<Vec<_>>();
    if summary.is_empty() {
        "none".to_string()
    } else {
        summary.join(", ")
    }
}

pub(crate) fn summarize_job_ops(snapshot: &OpsSnapshot) -> String {
    format!(
        "profiles:{}, evidence:{}, sources:{}, roles:{} [{}], scores:[{}], follow_ups:{}",
        snapshot.job_hunting.profile_count,
        snapshot.job_hunting.evidence_card_count,
        snapshot.job_hunting.source_count,
        snapshot.job_hunting.role_count,
        summarize_usize_count_map(&snapshot.job_hunting.role_status_counts),
        summarize_usize_count_map(&snapshot.job_hunting.score_tier_counts),
        snapshot.job_hunting.follow_up_count
    )
}

pub(crate) fn summarize_x_drift(stats: &XStatsReport) -> String {
    let entries = [
        (
            "compat_missing_canonical",
            stats.drift.compatibility_without_canonical,
        ),
        (
            "canonical_missing_compat",
            stats.drift.canonical_without_compatibility,
        ),
        ("tweets_missing_fts", stats.drift.tweets_without_fts),
        ("fts_missing_tweets", stats.drift.fts_without_tweets),
        ("projection_failures", stats.drift.projection_failures),
        ("non_healthy_sources", stats.drift.non_healthy_sources),
    ];
    let summary = entries
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(label, count)| format!("{label}:{count}"))
        .collect::<Vec<_>>();
    if summary.is_empty() {
        "ok".to_string()
    } else {
        summary.join(", ")
    }
}

pub(crate) fn summarize_x_portable_export(stats: &XStatsReport) -> String {
    let export = &stats.portable_export;
    match &export.latest_completed_at {
        Some(completed_at) if export.stale => format!(
            "stale since {completed_at}; {} changed tweet(s)",
            export.tweets_updated_after_export
        ),
        Some(completed_at) => format!(
            "fresh at {completed_at}; {} row(s)",
            export.latest_rows_exported.unwrap_or(0)
        ),
        None if export.latest_failed_at.is_some() => {
            "no completed export; latest failed".to_string()
        }
        None => "not exported".to_string(),
    }
}

pub(crate) fn summarize_x_digest_queue(stats: &XStatsReport) -> String {
    let projection_summary = summarize_count_map(&stats.digest_projections_by_status);
    if stats.digest_candidates_linked_to_x == 0 && projection_summary == "none" {
        "none".to_string()
    } else {
        format!(
            "{} linked candidate(s); projections {}",
            stats.digest_candidates_linked_to_x, projection_summary
        )
    }
}

pub(crate) fn summarize_x_knowledge(snapshot: &OpsSnapshot) -> String {
    if snapshot.x_knowledge_clusters.is_empty() && snapshot.x_editorial_decisions.is_empty() {
        return "none".to_string();
    }
    let cluster_statuses = summarize_counts(
        snapshot
            .x_knowledge_clusters
            .iter()
            .map(|cluster| cluster.status.as_str()),
    );
    let decision_statuses = summarize_counts(
        snapshot
            .x_editorial_decisions
            .iter()
            .map(|decision| decision.status.as_str()),
    );
    format!(
        "{} cluster(s) {}; {} editorial decision(s) {}",
        snapshot.x_knowledge_clusters.len(),
        cluster_statuses,
        snapshot.x_editorial_decisions.len(),
        decision_statuses
    )
}

pub(crate) fn summarize_radar_run_scores(snapshot: &OpsSnapshot) -> String {
    let Some(run) = snapshot
        .radar_runs
        .iter()
        .find(|run| run.metadata.get("score_distribution").is_some())
    else {
        return "none".to_string();
    };
    let distribution = run
        .metadata
        .get("score_distribution")
        .unwrap_or(&Value::Null);
    format!(
        "{} scored; selected:{} over-limit:{} below:{} duplicate:{} source-quota:{} category-quota:{} other:{} p50:{}",
        radar_distribution_u64(distribution, "score_count").unwrap_or(run.scored_count as u64),
        radar_distribution_u64(distribution, "selected_count").unwrap_or(run.filtered_count as u64),
        radar_distribution_u64(distribution, "over_profile_limit_count").unwrap_or(0),
        radar_distribution_u64(distribution, "below_threshold_count").unwrap_or(0),
        radar_distribution_u64(distribution, "duplicate_count").unwrap_or(0),
        radar_distribution_status_count(distribution, "source_quota"),
        radar_distribution_status_count(distribution, "category_quota"),
        radar_distribution_other_count(distribution),
        radar_distribution_f64(distribution, "p50")
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "n/a".to_string())
    )
}

pub(crate) fn render_radar_score_bar(distribution: &Value) -> String {
    let total = radar_distribution_u64(distribution, "score_count").unwrap_or(0);
    if total == 0 {
        return "<span class=\"muted\">No scores</span>".to_string();
    }
    let selected = radar_distribution_u64(distribution, "selected_count").unwrap_or(0);
    let over = radar_distribution_u64(distribution, "over_profile_limit_count").unwrap_or(0);
    let below = radar_distribution_u64(distribution, "below_threshold_count").unwrap_or(0);
    let duplicate = radar_distribution_u64(distribution, "duplicate_count").unwrap_or(0);
    let source_quota = radar_distribution_status_count(distribution, "source_quota");
    let category_quota = radar_distribution_status_count(distribution, "category_quota");
    let other = radar_distribution_other_count(distribution);
    let mut html = "<div class=\"bar\" aria-label=\"radar score distribution\">".to_string();
    for (class, label, count) in [
        ("selected", "selected", selected),
        ("over", "over_profile_limit", over),
        ("below", "below_threshold", below),
        ("duplicate", "duplicate", duplicate),
        ("quota", "source_quota", source_quota),
        ("quota", "category_quota", category_quota),
        ("other", "other_status", other),
    ] {
        if count == 0 {
            continue;
        }
        let width = ((count as f64 / total as f64) * 100.0).clamp(1.0, 100.0);
        html.push_str(&format!(
            "<span class=\"{}\" title=\"{}:{}\" aria-label=\"{}:{}\" style=\"width:{:.1}%\"></span>",
            class, label, count, label, count, width
        ));
    }
    html.push_str("</div>");
    html
}

pub(crate) fn radar_distribution_status_count(distribution: &Value, status: &str) -> u64 {
    distribution
        .get("status_counts")
        .and_then(|counts| counts.get(status))
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

pub(crate) fn radar_distribution_other_count(distribution: &Value) -> u64 {
    let total = radar_distribution_u64(distribution, "score_count").unwrap_or(0);
    let shown = radar_distribution_u64(distribution, "selected_count")
        .unwrap_or(0)
        .saturating_add(
            radar_distribution_u64(distribution, "over_profile_limit_count").unwrap_or(0),
        )
        .saturating_add(radar_distribution_u64(distribution, "below_threshold_count").unwrap_or(0))
        .saturating_add(radar_distribution_u64(distribution, "duplicate_count").unwrap_or(0))
        .saturating_add(radar_distribution_status_count(
            distribution,
            "source_quota",
        ))
        .saturating_add(radar_distribution_status_count(
            distribution,
            "category_quota",
        ));
    total.saturating_sub(shown)
}

pub(crate) fn radar_distribution_u64(distribution: &Value, key: &str) -> Option<u64> {
    distribution.get(key).and_then(Value::as_u64)
}

pub(crate) fn radar_distribution_f64(distribution: &Value, key: &str) -> Option<f64> {
    distribution.get(key).and_then(Value::as_f64)
}

pub(crate) fn detail_link(kind: &str, id: &str, label: &str) -> String {
    format!(
        "<a href=\"/ops/ui?detail={}:{}\">{}</a>",
        html_escape(kind),
        html_escape(&url_component(id)),
        html_escape(label)
    )
}

pub(crate) fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

pub(crate) fn trimmed_non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn ops_notice_text(notice: &str) -> String {
    match notice {
        "dead_lettered" => "Edge event dead-lettered.".to_string(),
        "x_bookmarks_scheduled" => "X bookmark ingestion schedule updated.".to_string(),
        "x_bookmarks_enqueued" => "X bookmark import job queued.".to_string(),
        "knowledge_backlog_scheduled" => {
            "Knowledge backlog clustering schedule updated.".to_string()
        }
        "knowledge_backlog_enqueued" => "Knowledge backlog clustering job queued.".to_string(),
        "knowledge_model_clusters_scheduled" => {
            "Knowledge model clustering schedule updated.".to_string()
        }
        "knowledge_model_clusters_enqueued" => "Knowledge model clustering job queued.".to_string(),
        "knowledge_model_write_scheduled" => "Knowledge model writer schedule updated.".to_string(),
        "knowledge_model_write_enqueued" => "Knowledge model writer job queued.".to_string(),
        "knowledge_model_writes_due_enqueued" => {
            "Due promoted model-cluster writer jobs queued.".to_string()
        }
        "knowledge_cluster_expansions_enqueued" => {
            "Due knowledge cluster expansion jobs queued.".to_string()
        }
        "knowledge_cluster_editorial_decisions_enqueued" => {
            "Due knowledge cluster editorial decision jobs queued.".to_string()
        }
        "knowledge_investigations_enqueued" => {
            "Due knowledge investigation execution jobs queued.".to_string()
        }
        "worker_ran_once" => "Worker run completed.".to_string(),
        "duplicate" => {
            "Duplicate idempotency key ignored; no second mutation was applied.".to_string()
        }
        other => format!("Ops notice: {other}"),
    }
}

pub(crate) fn url_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

pub(crate) fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
