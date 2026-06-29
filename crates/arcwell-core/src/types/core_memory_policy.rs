use super::*;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub home: PathBuf,
    pub db: PathBuf,
    pub backups: PathBuf,
    pub wiki_pages: PathBuf,
    pub mem0: PathBuf,
    pub procedures: PathBuf,
}

impl AppPaths {
    pub fn new(home: impl Into<PathBuf>) -> Self {
        let home = home.into();
        Self {
            db: home.join("arcwell.sqlite3"),
            backups: home.join("backups"),
            wiki_pages: home.join("wiki").join("pages"),
            mem0: home.join("mem0"),
            procedures: home.join("procedures"),
            home,
        }
    }

    pub fn from_env_or_default() -> Result<Self> {
        if let Ok(home) = std::env::var("ARCWELL_HOME") {
            return Ok(Self::new(home));
        }

        let home = std::env::var("HOME").context("HOME is not set")?;
        Ok(Self::new(PathBuf::from(home).join(".arcwell")))
    }

    pub fn ensure(&self) -> Result<()> {
        fs::create_dir_all(&self.home)
            .with_context(|| format!("creating {}", self.home.display()))?;
        fs::create_dir_all(&self.backups)
            .with_context(|| format!("creating {}", self.backups.display()))?;
        fs::create_dir_all(&self.wiki_pages)
            .with_context(|| format!("creating {}", self.wiki_pages.display()))?;
        fs::create_dir_all(&self.mem0)
            .with_context(|| format!("creating {}", self.mem0.display()))?;
        fs::create_dir_all(&self.procedures)
            .with_context(|| format!("creating {}", self.procedures.display()))?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct HealthReport {
    pub ok: bool,
    pub home: PathBuf,
    pub db: PathBuf,
    pub schema_version: i64,
    pub profile_items: i64,
    pub memories: i64,
    pub wiki_pages: i64,
    pub source_cards: i64,
    pub watch_sources: i64,
    pub wiki_jobs: i64,
    pub x_items: i64,
    pub x_tweets: i64,
    pub x_profiles: i64,
    pub pending_jobs: i64,
    pub cursors: i64,
    pub research_runs: i64,
    pub pending_candidates: i64,
    pub work_runs: i64,
    pub failed_jobs: i64,
    pub dead_lettered_jobs: i64,
    pub latest_backup: Option<String>,
    pub latest_worker_heartbeat: Option<WorkerHeartbeat>,
    pub latest_worker_heartbeat_events: Vec<WorkerHeartbeatEvent>,
    pub secret_health: Vec<SecretHealth>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorOptions {
    pub strict: bool,
    pub max_worker_heartbeat_age_seconds: i64,
    pub max_dead_lettered_jobs: i64,
    pub max_backup_age_seconds: i64,
    pub service_plist_path: Option<PathBuf>,
}

impl Default for DoctorOptions {
    fn default() -> Self {
        Self {
            strict: false,
            max_worker_heartbeat_age_seconds: 300,
            max_dead_lettered_jobs: 0,
            max_backup_age_seconds: 7 * 24 * 60 * 60,
            service_plist_path: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub ok: bool,
    pub strict: bool,
    pub health: HealthReport,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileItem {
    pub key: String,
    pub value: String,
    pub sensitivity: String,
    pub source: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub text: String,
    pub kind: String,
    pub sensitivity: String,
    pub source: String,
    pub user_id: Option<String>,
    pub confidence: f64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mem0AddReport {
    pub provider: String,
    pub user_id: String,
    pub infer: bool,
    pub results: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mem0SearchReport {
    pub provider: String,
    pub user_id: String,
    pub query: String,
    pub results: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mem0MutationReport {
    pub ok: bool,
    pub provider: String,
    pub user_id: String,
    pub response: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryForgetReport {
    pub ok: bool,
    pub provider: String,
    pub user_id: String,
    pub provider_memories_deleted: usize,
    pub provider_response: Value,
    pub candidates_deleted: usize,
    pub legacy_unscoped_candidates_deleted: usize,
    pub compatibility_memories_deleted: usize,
    pub legacy_unscoped_compatibility_deleted: usize,
    pub lifecycle_events_deleted: usize,
    pub decision_ledger_deleted: usize,
    pub tombstone_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub target: String,
    pub kind: String,
    pub content: String,
    pub sensitivity: String,
    pub source_ref: String,
    pub status: String,
    pub created_at: String,
    pub operation: String,
    pub memory_id: Option<String>,
    pub user_id: Option<String>,
    pub metadata: Value,
    pub applied_result: Option<Value>,
    pub applied_at: Option<String>,
    pub rejected_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCandidateApplyReport {
    pub ok: bool,
    pub candidate_id: String,
    pub operation: String,
    pub user_id: Option<String>,
    pub memory_id: Option<String>,
    pub result: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecallReport {
    pub query: String,
    pub user_id: String,
    pub profile_matches: Vec<ProfileItem>,
    pub memory: Mem0SearchReport,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCaptureReport {
    pub mode: String,
    pub user_id: Option<String>,
    pub candidates_created: usize,
    pub duplicates_suppressed: usize,
    pub sensitive_pending: usize,
    pub auto_applied: usize,
    pub candidates: Vec<Candidate>,
    pub applied: Vec<MemoryCandidateApplyReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLifecycleEvent {
    pub id: String,
    pub event_type: String,
    pub hook: Option<String>,
    pub user_id: Option<String>,
    pub source_ref: Option<String>,
    pub input: Option<String>,
    pub result: Value,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDecisionLedgerEntry {
    pub id: String,
    pub user_id: Option<String>,
    pub source_ref: String,
    pub observation: String,
    pub operation: String,
    pub memory_id: Option<String>,
    pub candidate_id: Option<String>,
    pub confidence: f64,
    pub reason: String,
    pub metadata: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryForgetTombstone {
    pub id: String,
    pub user_id_hash: String,
    pub provider: String,
    pub provider_memories_deleted: usize,
    pub candidates_deleted: usize,
    pub compatibility_memories_deleted: usize,
    pub lifecycle_events_deleted: usize,
    pub decision_ledger_deleted: usize,
    pub policy: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvalReport {
    pub ok: bool,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub cases: Vec<MemoryEvalCaseResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvalCaseResult {
    pub name: String,
    pub input: String,
    pub expected_candidates: usize,
    pub actual_candidates: usize,
    pub expected_sensitive: usize,
    pub actual_sensitive: usize,
    pub expected_phrases: Vec<String>,
    pub actual_phrases: Vec<String>,
    pub passed: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDreamReport {
    pub user_id: String,
    pub provider_exact_duplicates_deleted: usize,
    pub compatibility_exact_duplicates_deleted: usize,
    pub compatibility_provider_duplicates_deleted: usize,
    pub conflict_candidates_created: usize,
    pub conflicts_detected: usize,
    pub actions: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEntry {
    pub id: String,
    pub package: String,
    pub job_id: String,
    pub provider: String,
    pub model: String,
    pub source: Option<String>,
    pub estimated_usd: f64,
    pub actual_usd: f64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostPolicy {
    pub scope: String,
    pub key: String,
    pub limit_usd: Option<f64>,
    pub kill_switch: bool,
    pub override_until: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostDecision {
    #[serde(default)]
    pub decision_id: Option<String>,
    pub allowed: bool,
    pub reason: String,
    pub matched_policy: Option<CostPolicy>,
    pub projected_usd: f64,
    pub spent_usd: f64,
    pub remaining_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostDecisionRecord {
    pub id: String,
    pub allowed: bool,
    pub reason: String,
    pub package: String,
    pub job_id: String,
    pub provider: String,
    pub model: String,
    pub source: Option<String>,
    pub projected_usd: f64,
    pub spent_usd: f64,
    pub remaining_usd: Option<f64>,
    pub matched_scope: Option<String>,
    pub matched_key: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub id: String,
    pub effect: String,
    pub action: String,
    pub reason: String,
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub priority: i64,
    #[serde(default)]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRequest {
    pub action: String,
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub projected_usd: Option<f64>,
    #[serde(default)]
    pub metadata: Value,
    #[serde(default)]
    pub untrusted_excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecisionRecord {
    pub id: String,
    pub action: String,
    pub effect: String,
    pub allowed: bool,
    pub reason: String,
    pub matched_rule_id: Option<String>,
    pub approval_id: Option<String>,
    pub package: Option<String>,
    pub provider: Option<String>,
    pub source: Option<String>,
    pub channel: Option<String>,
    pub subject: Option<String>,
    pub target: Option<String>,
    pub metadata: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyApprovalRecord {
    pub id: String,
    pub decision_id: String,
    pub action: String,
    pub status: String,
    pub reason: String,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyExplanation {
    pub request: PolicyRequest,
    pub effect: String,
    pub allowed: bool,
    pub reason: String,
    pub matched_rule: Option<PolicyRule>,
    pub matching_rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyOverrideReport {
    pub policy_path: PathBuf,
    pub rule: PolicyRule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PolicyFile {
    #[serde(default)]
    pub(crate) rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretRef {
    pub name: String,
    pub location: String,
    pub scope: String,
    pub expires_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretValue {
    pub name: String,
    pub scope: String,
    pub provider: Option<String>,
    pub expires_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretHealth {
    pub name: String,
    pub scope: String,
    pub provider: Option<String>,
    pub source: String,
    pub present: bool,
    pub status: String,
    pub expires_at: Option<String>,
    pub updated_at: String,
    pub warnings: Vec<String>,
}
