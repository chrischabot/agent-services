use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchRun {
    pub id: String,
    pub query: String,
    pub status: String,
    pub result_page_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchTask {
    pub id: String,
    pub run_id: String,
    pub role: String,
    pub status: String,
    pub instructions: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchPlan {
    pub run: ResearchRun,
    pub local_sources: Vec<WikiPageSummary>,
    pub suggested_searches: Vec<String>,
    pub open_questions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchBrief {
    pub run: ResearchRun,
    pub source_count: usize,
    pub result_page_id: Option<String>,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchWorkflow {
    pub run: ResearchRun,
    pub tasks: Vec<ResearchTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchRunStatus {
    pub run: ResearchRun,
    pub task_count: usize,
    pub pending_task_count: usize,
    pub completed_task_count: usize,
    pub cancelled_task_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchRunRead {
    pub run: ResearchRun,
    pub tasks: Vec<ResearchTask>,
    pub role_runs: Vec<ResearchRoleRun>,
    pub artifacts: Vec<ResearchArtifact>,
    pub host_searches: Vec<ResearchHostSearchRecord>,
    pub documents: Vec<ResearchDocumentRecord>,
    pub editorial_runs: Vec<ResearchEditorialRun>,
    pub sources: Vec<ResearchRunSourceRecord>,
    pub claims: Vec<ResearchClaimRecord>,
    pub convergence: Option<ResearchConvergenceStatus>,
    pub result_page: Option<WikiPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchRunAudit {
    pub run: ResearchRun,
    pub audit: ResearchAuditReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSourceInput {
    pub url: Option<String>,
    pub local_ref: Option<String>,
    pub title: String,
    pub source_family: String,
    pub source_type: String,
    pub provider: String,
    pub author: Option<String>,
    pub published_at: Option<String>,
    pub language: Option<String>,
    pub priority: i64,
    pub reason: String,
    pub canonical_key: Option<String>,
    pub fetch_status: String,
    pub read_depth: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSource {
    pub id: String,
    pub url: Option<String>,
    pub local_ref: Option<String>,
    pub title: String,
    pub source_family: String,
    pub source_type: String,
    pub provider: String,
    pub author: Option<String>,
    pub published_at: Option<String>,
    pub language: Option<String>,
    pub priority: i64,
    pub reason: String,
    pub canonical_key: String,
    pub fetch_status: String,
    pub read_depth: String,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchRunSourceLink {
    pub id: String,
    pub run_id: String,
    pub source_id: String,
    pub source_card_id: Option<String>,
    pub triage_status: String,
    pub read_depth: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchRunSourceRecord {
    pub source: ResearchSource,
    pub link: ResearchRunSourceLink,
    pub source_card: Option<SourceCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchExtractionPrompt {
    pub run_id: String,
    pub source_card_id: String,
    pub prompt: String,
    pub schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchClaim {
    pub id: String,
    pub run_id: String,
    pub text: String,
    pub kind: String,
    pub subject: Option<String>,
    pub predicate: Option<String>,
    pub object_value: Option<String>,
    pub temporal_scope: Option<String>,
    pub confidence: f64,
    pub caveats: Vec<String>,
    pub extraction_provider: String,
    pub extraction_model: String,
    pub extracted_at: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchClaimSource {
    pub id: String,
    pub claim_id: String,
    pub source_card_id: String,
    pub quote: Option<String>,
    pub source_anchor: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResearchEvidenceAnchor {
    pub document_id: String,
    pub span_id: Option<String>,
    pub table_id: Option<String>,
    pub row_index: Option<usize>,
    pub column_index: Option<usize>,
    pub quote: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchClaimDocumentAnchor {
    pub id: String,
    pub claim_source_id: String,
    pub document_id: String,
    pub anchor_kind: String,
    pub document_span_id: Option<String>,
    pub table_id: Option<String>,
    pub table_cell_id: Option<String>,
    pub span_id: Option<String>,
    pub table_local_id: Option<String>,
    pub row_index: Option<usize>,
    pub column_index: Option<usize>,
    pub anchor_label: String,
    pub quote: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchClaimRecord {
    pub claim: ResearchClaim,
    pub sources: Vec<ResearchClaimSource>,
    pub document_anchors: Vec<ResearchClaimDocumentAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchCluster {
    pub id: String,
    pub run_id: String,
    pub theme: String,
    pub summary: String,
    pub claim_count: usize,
    pub evidence_strength: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchContradiction {
    pub id: String,
    pub run_id: String,
    pub left_claim_id: String,
    pub right_claim_id: String,
    pub severity: String,
    pub notes: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSkepticReport {
    pub run_id: String,
    pub checked_at: String,
    pub ok: bool,
    pub clusters: Vec<ResearchCluster>,
    pub contradictions: Vec<ResearchContradiction>,
    pub findings: Vec<ResearchAuditFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchReport {
    pub id: String,
    pub run_id: String,
    pub status: String,
    pub wiki_page_id: Option<String>,
    pub saturation_reason: String,
    pub markdown: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConvergenceConfig {
    pub max_iterations: usize,
    pub max_seconds: i64,
    pub max_sources: usize,
    pub max_provider_calls: usize,
    pub cost_cap_usd: f64,
    pub source_novelty_threshold: f64,
    pub confidence_delta_threshold: f64,
    pub no_progress_iteration_limit: usize,
    pub require_active_fact_check: bool,
    pub allow_long_run: bool,
    pub no_write: bool,
    pub editorial_provider: Option<String>,
    pub editorial_model_name: Option<String>,
    pub editorial_endpoint: Option<String>,
    pub editorial_timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConvergenceStartInput {
    pub run_id: String,
    pub max_iterations: Option<usize>,
    pub max_seconds: Option<i64>,
    pub max_sources: Option<usize>,
    pub max_provider_calls: Option<usize>,
    pub cost_cap_usd: Option<f64>,
    pub source_novelty_threshold: Option<f64>,
    pub confidence_delta_threshold: Option<f64>,
    pub no_progress_iteration_limit: Option<usize>,
    pub require_active_fact_check: Option<bool>,
    pub allow_long_run: Option<bool>,
    pub no_write: Option<bool>,
    pub editorial_provider: Option<String>,
    pub editorial_model_name: Option<String>,
    pub editorial_endpoint: Option<String>,
    pub editorial_timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConvergenceStepInput {
    pub run_id: String,
    pub max_iterations: Option<usize>,
    pub max_seconds: Option<i64>,
    pub max_sources: Option<usize>,
    pub max_provider_calls: Option<usize>,
    pub cost_cap_usd: Option<f64>,
    pub source_novelty_threshold: Option<f64>,
    pub confidence_delta_threshold: Option<f64>,
    pub no_progress_iteration_limit: Option<usize>,
    pub require_active_fact_check: Option<bool>,
    pub allow_long_run: Option<bool>,
    pub no_write: Option<bool>,
    pub editorial_provider: Option<String>,
    pub editorial_model_name: Option<String>,
    pub editorial_endpoint: Option<String>,
    pub editorial_timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConvergenceProviderSearchInput {
    pub run_id: String,
    pub provider: String,
    pub max_tasks: Option<usize>,
    pub max_results: Option<usize>,
    pub max_provider_calls: Option<usize>,
    pub enqueue_selected_url_ingest: Option<bool>,
    pub max_ingest_jobs: Option<usize>,
    pub cost_cap_usd: Option<f64>,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConvergenceProviderSearchResult {
    pub run_id: String,
    pub provider: String,
    pub attempted: Vec<ResearchConvergenceProviderSearchAttempt>,
    pub remaining_tasks: Vec<ResearchConvergenceHostSearchTask>,
    pub provider_call_count: usize,
    pub ingest_jobs: Vec<WikiJob>,
    pub projected_cost_usd: f64,
    pub stopped_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConvergenceProviderSearchAttempt {
    pub task: ResearchConvergenceHostSearchTask,
    pub status: String,
    pub host_search_id: Option<String>,
    pub cost_decision_id: Option<String>,
    pub result_count: usize,
    pub selected_result_count: usize,
    pub ingest_job_ids: Vec<String>,
    pub error_message_redacted: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConvergenceStep {
    pub run: ResearchRun,
    pub iteration: ResearchIteration,
    pub statements: Vec<ResearchStatement>,
    pub challenges: Vec<ResearchChallenge>,
    pub disproofs: Vec<ResearchDisproof>,
    pub revisions: Vec<ResearchRevision>,
    pub fact_checks: Vec<ResearchFactCheck>,
    pub snapshot: ResearchConvergenceSnapshot,
    pub status: ResearchConvergenceStatus,
    pub report: Option<ResearchConvergenceReport>,
    pub editorial: Option<ResearchConvergenceEditorialLoop>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConvergenceEditorialLoop {
    pub report: ResearchConvergenceReport,
    pub citation_verifier: Option<ResearchEditorialInvocation>,
    pub adversarial_evaluator: Option<ResearchEditorialInvocation>,
    pub status: String,
    pub blocking_findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConvergenceStatus {
    pub run_id: String,
    pub latest_iteration: Option<ResearchIteration>,
    pub latest_snapshot: Option<ResearchConvergenceSnapshot>,
    pub current_statements: Vec<ResearchStatement>,
    pub open_challenges: Vec<ResearchChallenge>,
    pub host_search_tasks: Vec<ResearchConvergenceHostSearchTask>,
    pub strong_refutations: Vec<ResearchDisproof>,
    pub stop_reason: Option<String>,
    pub settled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConvergenceHostSearchTask {
    pub id: String,
    pub run_id: String,
    pub iteration_id: String,
    pub challenge_id: String,
    pub statement_id: String,
    pub challenge_type: String,
    pub severity: String,
    pub query: String,
    pub normalized_query: String,
    pub required_source_families: Value,
    pub status: String,
    pub matched_host_search_ids: Vec<String>,
    pub matched_result_ids: Vec<String>,
    pub research_source_ids: Vec<String>,
    pub source_card_ids: Vec<String>,
    pub selected_result_count: usize,
    pub instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchIteration {
    pub id: String,
    pub run_id: String,
    pub iteration_index: usize,
    pub parent_iteration_id: Option<String>,
    pub status: String,
    pub objective: String,
    pub position_artifact_id: Option<String>,
    pub statement_set_artifact_id: Option<String>,
    pub challenge_pack_artifact_id: Option<String>,
    pub disproof_pack_artifact_id: Option<String>,
    pub revision_artifact_id: Option<String>,
    pub convergence_snapshot_id: Option<String>,
    pub cost_decision_id: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub stop_reason: Option<String>,
    pub error_message_redacted: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchStatement {
    pub id: String,
    pub run_id: String,
    pub iteration_id: String,
    pub parent_statement_id: Option<String>,
    pub stable_key: String,
    pub statement_type: String,
    pub text: String,
    pub scope: Option<String>,
    pub temporal_scope: Option<String>,
    pub confidence: f64,
    pub certainty_label: String,
    pub status: String,
    pub importance: String,
    pub evidence: Value,
    pub counterevidence: Value,
    pub assumptions: Value,
    pub caveats: Value,
    pub created_by_role: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchChallenge {
    pub id: String,
    pub run_id: String,
    pub iteration_id: String,
    pub statement_id: String,
    pub challenge_type: String,
    pub severity: String,
    pub rationale: String,
    pub would_change_answer_if_true: bool,
    pub search_plan: Value,
    pub required_source_families: Value,
    pub status: String,
    pub created_by_role: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchDisproof {
    pub id: String,
    pub run_id: String,
    pub iteration_id: String,
    pub challenge_id: String,
    pub statement_id: String,
    pub verdict: String,
    pub strength: String,
    pub evidence: Value,
    pub reasoning_summary: String,
    pub confidence_delta: f64,
    pub requires_revision: bool,
    pub created_by_role: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchRevision {
    pub id: String,
    pub run_id: String,
    pub iteration_id: String,
    pub from_statement_id: String,
    pub to_statement_id: Option<String>,
    pub revision_type: String,
    pub rationale: String,
    pub trigger_disproof_ids: Value,
    pub evidence_delta: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchFactCheck {
    pub id: String,
    pub run_id: String,
    pub iteration_id: String,
    pub statement_id: String,
    pub label: String,
    pub impact: String,
    pub evidence: Value,
    pub notes: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchActiveFactCheckInput {
    pub run_id: String,
    pub artifact_id: Option<String>,
    pub max_sentences: Option<usize>,
    pub create_challenges: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchActiveFactCheckResult {
    pub run_id: String,
    pub artifact_id: String,
    pub checked_sentences: usize,
    pub matched_existing_statements: usize,
    pub created_statement_count: usize,
    pub created_challenge_count: usize,
    pub checks: Vec<ResearchFactCheck>,
    pub challenges: Vec<ResearchChallenge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConvergenceCloseLoopInput {
    pub run_id: String,
    pub artifact_id: Option<String>,
    pub max_sentences: Option<usize>,
    pub create_challenges: Option<bool>,
    pub compile_report_before_check: Option<bool>,
    pub rerun_after_check: Option<bool>,
    pub compile_final_report: Option<bool>,
    pub provider: Option<String>,
    pub provider_max_tasks: Option<usize>,
    pub provider_max_results: Option<usize>,
    pub provider_max_provider_calls: Option<usize>,
    pub enqueue_selected_url_ingest: Option<bool>,
    pub max_ingest_jobs: Option<usize>,
    pub provider_cost_cap_usd: Option<f64>,
    pub provider_endpoint: Option<String>,
    pub provider_api_key: Option<String>,
    pub provider_model: Option<String>,
    pub provider_timeout_seconds: Option<u64>,
    pub max_iterations: Option<usize>,
    pub max_seconds: Option<i64>,
    pub max_sources: Option<usize>,
    pub max_provider_calls: Option<usize>,
    pub cost_cap_usd: Option<f64>,
    pub source_novelty_threshold: Option<f64>,
    pub confidence_delta_threshold: Option<f64>,
    pub no_progress_iteration_limit: Option<usize>,
    pub require_active_fact_check: Option<bool>,
    pub allow_long_run: Option<bool>,
    pub no_write: Option<bool>,
    pub editorial_provider: Option<String>,
    pub editorial_model_name: Option<String>,
    pub editorial_endpoint: Option<String>,
    pub editorial_timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConvergenceCloseLoopResult {
    pub run_id: String,
    pub initial_status: ResearchConvergenceStatus,
    pub checked_artifact_id: String,
    pub active_fact_check: ResearchActiveFactCheckResult,
    pub after_active_fact_check_status: ResearchConvergenceStatus,
    pub provider_search: Option<ResearchConvergenceProviderSearchResult>,
    pub after_provider_search_status: ResearchConvergenceStatus,
    pub convergence_rerun: Option<ResearchConvergenceStep>,
    pub final_status: ResearchConvergenceStatus,
    pub final_report: Option<ResearchConvergenceReport>,
    pub editorial: Option<ResearchConvergenceEditorialLoop>,
    pub remaining_host_search_tasks: Vec<ResearchConvergenceHostSearchTask>,
    pub closure_status: String,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConvergenceSnapshot {
    pub id: String,
    pub run_id: String,
    pub iteration_id: String,
    pub source_count_total: usize,
    pub source_count_new: usize,
    pub primary_source_count_new: usize,
    pub claim_count_total: usize,
    pub statement_count_current: usize,
    pub statement_count_changed: usize,
    pub critical_open_challenges: usize,
    pub high_open_challenges: usize,
    pub strong_refutations: usize,
    pub unknown_high_impact_claims: usize,
    pub mean_confidence_delta: f64,
    pub max_confidence_delta: f64,
    pub source_novelty_score: f64,
    pub claim_novelty_score: f64,
    pub position_edit_distance: f64,
    pub citation_support_score: f64,
    pub active_fact_check_score: f64,
    pub evaluator_score: f64,
    pub cost_usd_estimated: f64,
    pub elapsed_seconds: i64,
    pub stop_rule: Value,
    pub settled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchReportJudgment {
    pub id: String,
    pub run_id: String,
    pub report_id: Option<String>,
    pub judgment_version: String,
    pub overall_decision: String,
    pub scores: Value,
    pub blocking_findings: Value,
    pub non_blocking_findings: Value,
    pub evidence_checked: Value,
    pub remaining_risks: Value,
    pub commands_or_artifacts_reviewed: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConvergenceReport {
    pub artifact: ResearchArtifact,
    pub judgment: ResearchReportJudgment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchRoleRunStart {
    pub run_id: String,
    pub role: String,
    pub host: String,
    pub host_thread_id: Option<String>,
    pub host_subagent_id: Option<String>,
    pub tool_surface: Option<String>,
    pub prompt_version: String,
    pub prompt_hash: Option<String>,
    pub execution_mode: String,
    pub input_artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchRoleRun {
    pub id: String,
    pub run_id: String,
    pub role: String,
    pub host: String,
    pub host_thread_id: Option<String>,
    pub host_subagent_id: Option<String>,
    pub tool_surface: Option<String>,
    pub prompt_version: String,
    pub prompt_hash: Option<String>,
    pub execution_mode: String,
    pub input_artifact_ids: Vec<String>,
    pub output_artifact_id: Option<String>,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub error_kind: Option<String>,
    pub error_message_redacted: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchArtifactInput {
    pub run_id: String,
    pub role_run_id: Option<String>,
    pub artifact_type: String,
    pub title: String,
    pub body: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchArtifact {
    pub id: String,
    pub run_id: String,
    pub role_run_id: Option<String>,
    pub artifact_type: String,
    pub title: String,
    pub body: String,
    pub body_sha256: String,
    pub metadata: Value,
    pub created_at: String,
}
