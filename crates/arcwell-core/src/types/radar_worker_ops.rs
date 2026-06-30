use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarProfileInput {
    pub name: String,
    pub description: String,
    pub window_hours: i64,
    pub min_score: f64,
    pub max_items: Option<i64>,
    pub languages: Vec<String>,
    pub source_selectors: Value,
    pub delivery_policy: Value,
    pub model_policy: Value,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub window_hours: i64,
    pub min_score: f64,
    pub max_items: Option<i64>,
    pub languages: Vec<String>,
    pub category_groups: Value,
    pub source_selectors: Value,
    pub delivery_policy: Value,
    pub model_policy: Value,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarRun {
    pub id: String,
    pub profile_id: String,
    pub status: String,
    pub window_start: String,
    pub window_end: String,
    pub stage: String,
    pub source_selection: Value,
    pub raw_count: i64,
    pub normalized_count: i64,
    pub indexed_count: i64,
    pub scored_count: i64,
    pub filtered_count: i64,
    pub enriched_count: i64,
    pub summary_count: i64,
    pub delivery_count: i64,
    pub error: Option<String>,
    pub metadata: Value,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarItem {
    pub id: String,
    pub run_id: String,
    pub stable_key: String,
    pub source_kind: String,
    pub provider: String,
    pub source_locator: String,
    pub native_id: Option<String>,
    pub canonical_url: Option<String>,
    pub title: String,
    pub author: Option<String>,
    pub published_at: Option<String>,
    pub fetched_at: String,
    pub content_text: String,
    pub content_sha256: String,
    pub metadata: Value,
    pub source_card_id: Option<String>,
    pub wiki_page_id: Option<String>,
    pub canonical_entity_ref: Option<String>,
    pub trust_level: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarScore {
    pub id: String,
    pub run_id: String,
    pub item_id: String,
    pub score_kind: String,
    pub score: f64,
    pub reason: String,
    pub tags: Vec<String>,
    pub model_provider: Option<String>,
    pub model_name: Option<String>,
    pub cost_decision_id: Option<String>,
    pub input_artifact_id: Option<String>,
    pub output_artifact_id: Option<String>,
    pub schema_version: i64,
    pub status: String,
    pub error: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarModelScoreReport {
    pub run_id: String,
    pub provider: String,
    pub model: String,
    pub score_kind: String,
    pub scored: usize,
    pub blocked: usize,
    pub input_artifact_id: String,
    pub output_artifact_id: Option<String>,
    pub cost_decision_id: Option<String>,
    pub proof_level: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarDedupGroup {
    pub id: String,
    pub run_id: String,
    pub dedup_kind: String,
    pub primary_item_id: String,
    pub member_item_ids: Vec<String>,
    pub reason: String,
    pub confidence: f64,
    pub model_provider: Option<String>,
    pub cost_decision_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarSourceQuality {
    pub id: String,
    pub run_id: String,
    pub source_kind: String,
    pub locator: String,
    pub window_start: String,
    pub window_end: String,
    pub raw_count: i64,
    pub accepted_count: i64,
    pub average_score: Option<f64>,
    pub score_p50: Option<f64>,
    pub score_p90: Option<f64>,
    pub signal_to_noise: Option<f64>,
    pub duplicate_rate: Option<f64>,
    pub delivery_contribution_count: i64,
    pub failure_count: i64,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarSourceQualityTrend {
    pub source_kind: String,
    pub locator: String,
    pub window_count: i64,
    pub run_count: i64,
    pub raw_count: i64,
    pub accepted_count: i64,
    pub failure_count: i64,
    pub non_healthy_count: i64,
    pub average_score: Option<f64>,
    pub signal_to_noise: Option<f64>,
    pub duplicate_rate: Option<f64>,
    pub quality_score: f64,
    pub latest_status: String,
    pub trend_status: String,
    pub first_window_start: String,
    pub last_window_end: String,
    pub latest_run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarFetchReport {
    pub run: RadarRun,
    pub profile: RadarProfile,
    pub items_inserted: usize,
    pub scores_inserted: usize,
    pub selected_items: usize,
    pub adapter_jobs: Vec<WikiJob>,
    pub adapter_runs: Vec<KnowledgeAdapterRun>,
    pub unsupported_selectors: Vec<Value>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarStageReport {
    pub run: RadarRun,
    pub items: Vec<RadarItem>,
    pub scores: Vec<RadarScore>,
    pub dedup_groups: Vec<RadarDedupGroup>,
    pub source_quality: Vec<RadarSourceQuality>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarSummary {
    pub id: String,
    pub run_id: String,
    pub language: String,
    pub format: String,
    pub title: String,
    pub body_markdown: String,
    pub item_ids: Vec<String>,
    pub source_card_ids: Vec<String>,
    pub audit_status: String,
    pub metadata: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarDeliveryInput {
    pub run_id: String,
    pub language: String,
    pub format: String,
    pub channel: String,
    pub recipient_ref: String,
    pub idempotency_key: Option<String>,
    pub telegram_bot_token: Option<String>,
    pub email_account_id: Option<String>,
    pub email_api_token: Option<String>,
    pub email_from: Option<String>,
    pub api_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarDelivery {
    pub id: String,
    pub run_id: String,
    pub summary_id: String,
    pub channel: String,
    pub recipient_ref: String,
    pub status: String,
    pub policy_decision_id: Option<String>,
    pub cost_decision_id: Option<String>,
    pub delivery_attempt_id: Option<String>,
    pub quiet_hours_deferred_until: Option<String>,
    pub idempotency_key: String,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarDeliveryReport {
    pub delivery: RadarDelivery,
    pub summary: RadarSummary,
    pub channel_message: Option<ChannelMessage>,
    pub channel_delivery_attempt: Option<ChannelDeliveryAttempt>,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarDeliveryReconcileReport {
    pub inspected: usize,
    pub sent: usize,
    pub failed: usize,
    pub dead_lettered: usize,
    pub updated: Vec<RadarDelivery>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestDeliveryReconcileReport {
    pub inspected: usize,
    pub sent: usize,
    pub failed: usize,
    pub dead_lettered: usize,
    pub updated: Vec<DigestDelivery>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestAlertScheduleInput {
    pub name: String,
    pub channel: String,
    pub recipient_ref: String,
    pub min_score: f64,
    pub max_candidates: i64,
    pub interval_hours: i64,
    pub quiet_hours: Option<Value>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestAlertSchedule {
    pub id: String,
    pub name: String,
    pub status: String,
    pub channel: String,
    pub recipient_ref: String,
    pub min_score: f64,
    pub max_candidates: i64,
    pub interval_hours: i64,
    pub quiet_hours: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestAlertScheduleEnqueueReport {
    pub inspected: usize,
    pub enqueued: usize,
    pub skipped: usize,
    pub jobs: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestAlertTick {
    pub id: String,
    pub schedule_id: String,
    pub tick_key: String,
    pub due_at: String,
    pub status: String,
    pub job_id: Option<String>,
    pub candidate_ids: Vec<String>,
    pub delivery_ids: Vec<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueScheduleInput {
    pub name: String,
    pub kind: String,
    pub channel: String,
    pub recipient_ref: String,
    pub time_zone: String,
    pub hour: i64,
    pub minute: i64,
    pub catch_up_hours: i64,
    pub metadata: Value,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueSchedule {
    pub id: String,
    pub name: String,
    pub status: String,
    pub kind: String,
    pub channel: String,
    pub recipient_ref: String,
    pub time_zone: String,
    pub hour: i64,
    pub minute: i64,
    pub catch_up_hours: i64,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueScheduleTick {
    pub id: String,
    pub schedule_id: String,
    pub tick_key: String,
    pub due_at: String,
    pub status: String,
    pub job_id: Option<String>,
    pub candidate_id: Option<String>,
    pub delivery_id: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueScheduleEnqueueReport {
    pub inspected: usize,
    pub enqueued: usize,
    pub skipped: usize,
    pub jobs: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailDeliveryVerificationEnqueueReport {
    pub inspected: usize,
    pub enqueued: usize,
    pub skipped: usize,
    pub jobs: Vec<String>,
    pub errors: Vec<String>,
    pub gap_count: usize,
    pub request_count: usize,
    pub active_job_id: Option<String>,
    pub recent_job_id: Option<String>,
    pub minimum_age_seconds: i64,
    pub throttle_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMailboxPlacementRepairEnqueueReport {
    pub inspected: usize,
    pub enqueued: usize,
    pub skipped: usize,
    pub jobs: Vec<String>,
    pub errors: Vec<String>,
    pub gap_count: usize,
    pub repairable_count: usize,
    pub active_job_id: Option<String>,
    pub recent_job_id: Option<String>,
    pub throttle_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarAuditFinding {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarAuditReport {
    pub run_id: String,
    pub checked_at: String,
    pub ok: bool,
    pub item_count: i64,
    pub fts_count: i64,
    pub scored_count: i64,
    pub dedup_group_count: i64,
    pub source_quality_count: i64,
    pub findings: Vec<RadarAuditFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchAuditFinding {
    pub severity: String,
    pub code: String,
    pub source_card_id: Option<String>,
    pub message: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchAuditReport {
    pub query: String,
    pub checked_at: String,
    pub ok: bool,
    pub source_card_count: usize,
    pub local_source_count: usize,
    pub findings: Vec<ResearchAuditFinding>,
    pub checklist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceHealth {
    pub key: String,
    pub provider: String,
    pub source_kind: String,
    pub locator: String,
    pub status: String,
    pub last_success_at: Option<String>,
    pub last_failure_at: Option<String>,
    pub last_error: Option<String>,
    pub last_item_id: Option<String>,
    pub last_item_date: Option<String>,
    pub cursor_key: Option<String>,
    pub cursor_value: Option<String>,
    pub next_run_at: Option<String>,
    pub updated_at: String,
}

pub(crate) struct SourceHealthUpdate<'a> {
    pub(crate) key: &'a str,
    pub(crate) provider: &'a str,
    pub(crate) source_kind: &'a str,
    pub(crate) locator: &'a str,
    pub(crate) last_item_id: Option<&'a str>,
    pub(crate) last_item_date: Option<&'a str>,
    pub(crate) cursor_key: Option<&'a str>,
    pub(crate) cursor_value: Option<&'a str>,
    pub(crate) next_run_at: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchSource {
    pub id: String,
    pub source_kind: String,
    pub locator: String,
    pub label: String,
    pub cadence: String,
    pub status: String,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchSourceInput {
    pub source_kind: String,
    pub locator: String,
    pub label: String,
    pub cadence: String,
    pub status: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchSourceImportReport {
    pub root: PathBuf,
    pub imported: usize,
    pub added: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub skipped: usize,
    pub by_kind: BTreeMap<String, usize>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchSourcePollEnqueueReport {
    pub inspected: usize,
    pub enqueued: usize,
    pub skipped: usize,
    pub jobs: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarScheduleEnqueueReport {
    pub inspected: usize,
    pub enqueued: usize,
    pub skipped: usize,
    pub jobs: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeClusterExpansionEnqueueReport {
    pub inspected: usize,
    pub enqueued: usize,
    pub skipped: usize,
    pub jobs: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeClusterEditorialDecisionEnqueueReport {
    pub inspected: usize,
    pub enqueued: usize,
    pub skipped: usize,
    pub jobs: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeClusterModelWriterEnqueueReport {
    pub inspected: usize,
    pub enqueued: usize,
    pub skipped: usize,
    pub jobs: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntityResolutionEnqueueReport {
    pub inspected: usize,
    pub enqueued: usize,
    pub skipped: usize,
    pub jobs: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeClusterInvestigationExecutionEnqueueReport {
    pub inspected: usize,
    pub enqueued: usize,
    pub skipped: usize,
    pub jobs: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarScheduleTick {
    pub id: String,
    pub profile_id: String,
    pub tick_key: String,
    pub due_at: String,
    pub status: String,
    pub job_id: Option<String>,
    pub run_id: Option<String>,
    pub summary_id: Option<String>,
    pub delivery_id: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ScheduledRadarDeliveryPolicy {
    pub(crate) interval_hours: i64,
    pub(crate) channel: String,
    pub(crate) recipient_ref: String,
    pub(crate) language: String,
    pub(crate) format: String,
    pub(crate) fetch_live: bool,
    pub(crate) quiet_hours: Option<ScheduledRadarQuietHours>,
}

#[derive(Debug, Clone)]
pub(crate) struct ScheduledRadarQuietHours {
    pub(crate) start_minutes: u32,
    pub(crate) end_minutes: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct DigestAlertSchedulePolicy {
    pub(crate) channel: String,
    pub(crate) recipient_ref: String,
    pub(crate) min_score: f64,
    pub(crate) max_candidates: i64,
    pub(crate) interval_hours: i64,
    pub(crate) quiet_hours: Option<ScheduledRadarQuietHours>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiJob {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub input_json: Value,
    pub result_json: Option<Value>,
    pub error: Option<String>,
    pub attempts: i64,
    pub max_attempts: i64,
    pub leased_until: Option<String>,
    pub worker_id: Option<String>,
    pub next_run_at: Option<String>,
    pub dead_lettered_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRunReport {
    pub processed: usize,
    pub completed: usize,
    pub failed: usize,
    pub deferred: usize,
    pub dead_lettered: usize,
    pub jobs: Vec<WikiJob>,
    pub watch_poll: Option<WatchSourcePollEnqueueReport>,
    pub radar_schedule: Option<RadarScheduleEnqueueReport>,
    pub digest_alert_schedule: Option<DigestAlertScheduleEnqueueReport>,
    pub issue_schedule: Option<IssueScheduleEnqueueReport>,
    pub email_delivery_verification: Option<EmailDeliveryVerificationEnqueueReport>,
    pub email_mailbox_placement_repair: Option<EmailMailboxPlacementRepairEnqueueReport>,
    pub knowledge_cluster_model_writer: Option<KnowledgeClusterModelWriterEnqueueReport>,
    pub knowledge_entity_resolution: Option<KnowledgeEntityResolutionEnqueueReport>,
    pub knowledge_cluster_editorial_decision:
        Option<KnowledgeClusterEditorialDecisionEnqueueReport>,
    pub knowledge_cluster_expansion: Option<KnowledgeClusterExpansionEnqueueReport>,
    pub knowledge_cluster_investigation_execution:
        Option<KnowledgeClusterInvestigationExecutionEnqueueReport>,
    pub telegram_retry: Option<TelegramRetryReport>,
    pub email_retry: Option<EmailRetryReport>,
    pub radar_delivery_reconcile: Option<RadarDeliveryReconcileReport>,
    pub digest_delivery_reconcile: Option<DigestDeliveryReconcileReport>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerHeartbeat {
    pub worker_id: String,
    pub started_at: String,
    pub last_seen_at: String,
    pub processed_jobs: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerHeartbeatEvent {
    pub id: String,
    pub worker_id: String,
    pub seen_at: String,
    pub processed_jobs: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRecurrenceAudit {
    pub ok: bool,
    pub worker_id: Option<String>,
    pub worker_ids: Vec<String>,
    pub event_count: usize,
    pub retained_event_count: usize,
    pub first_seen_at: Option<String>,
    pub last_seen_at: Option<String>,
    pub observed_span_seconds: i64,
    pub max_gap_seconds: Option<i64>,
    pub min_required_span_seconds: i64,
    pub max_allowed_gap_seconds: i64,
    pub failures: Vec<String>,
    pub sample_events: Vec<WorkerHeartbeatEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorState {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}
