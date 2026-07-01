use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofPacketInput {
    pub scope: String,
    pub title: String,
    pub proof_level: String,
    pub status: String,
    pub summary: String,
    pub artifact_root: Option<String>,
    pub reviewer: Option<String>,
    #[serde(default)]
    pub claims: Vec<ProofClaimInput>,
    #[serde(default)]
    pub artifacts: Vec<ProofArtifactInput>,
    #[serde(default)]
    pub checks: Vec<ProofCheckInput>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofClaimInput {
    pub claim_key: String,
    pub claim: String,
    pub status: String,
    pub proof_level: String,
    #[serde(default)]
    pub evidence: Value,
    #[serde(default)]
    pub refutation: Value,
    #[serde(default)]
    pub gates: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofArtifactInput {
    pub artifact_kind: String,
    pub label: String,
    pub path: Option<String>,
    pub sha256: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofCheckInput {
    pub check_kind: String,
    pub command: String,
    pub status: String,
    pub exit_code: Option<i64>,
    pub duration_ms: Option<i64>,
    pub output_excerpt: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofPacket {
    pub id: String,
    pub scope: String,
    pub title: String,
    pub proof_level: String,
    pub status: String,
    pub summary: String,
    pub artifact_root: Option<String>,
    pub reviewer: Option<String>,
    pub metadata: Value,
    pub created_at: String,
    pub promoted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofClaim {
    pub id: String,
    pub packet_id: String,
    pub claim_key: String,
    pub claim: String,
    pub status: String,
    pub proof_level: String,
    pub evidence: Value,
    pub refutation: Value,
    pub gates: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofArtifact {
    pub id: String,
    pub packet_id: String,
    pub artifact_kind: String,
    pub label: String,
    pub path: Option<String>,
    pub sha256: Option<String>,
    pub metadata: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofCheck {
    pub id: String,
    pub packet_id: String,
    pub check_kind: String,
    pub command: String,
    pub status: String,
    pub exit_code: Option<i64>,
    pub duration_ms: Option<i64>,
    pub output_excerpt: Option<String>,
    pub metadata: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofPacketJudgment {
    pub promotable: bool,
    pub blockers: Vec<String>,
    pub proven_claims: usize,
    pub partial_claims: usize,
    pub blocked_claims: usize,
    pub refuted_claims: usize,
    pub not_claimed: usize,
    pub passed_checks: usize,
    pub failed_checks: usize,
    pub artifacts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofPacketReport {
    pub packet: ProofPacket,
    pub claims: Vec<ProofClaim>,
    pub artifacts: Vec<ProofArtifact>,
    pub checks: Vec<ProofCheck>,
    pub judgment: ProofPacketJudgment,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofPacketSummary {
    pub id: String,
    pub scope: String,
    pub title: String,
    pub proof_level: String,
    pub status: String,
    pub claim_count: usize,
    pub passed_checks: usize,
    pub blocker_count: usize,
    pub created_at: String,
    pub promoted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofPacketVerificationReport {
    pub path: String,
    pub packet_id: Option<String>,
    pub proof_name: Option<String>,
    pub proof_level: Option<String>,
    pub ok: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub checked_artifacts: Vec<ProofArtifactVerification>,
    pub redaction_findings: Vec<ProofRedactionFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofArtifactVerification {
    pub label: String,
    pub path: Option<String>,
    pub resolved_path: Option<String>,
    pub exists: bool,
    pub sha256_expected: Option<String>,
    pub sha256_actual: Option<String>,
    pub sha256_matches: Option<bool>,
    pub redaction_findings: usize,
    pub warnings: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRedactionFinding {
    pub location: String,
    pub kind: String,
    pub evidence_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialReviewRunInput {
    pub packet_id: Option<String>,
    pub scope: String,
    pub title: String,
    pub reviewer: String,
    pub requested_proof_level: String,
    pub judgment: String,
    pub summary: String,
    pub strongest_fake_done_path: String,
    #[serde(default)]
    pub refutations: Value,
    #[serde(default)]
    pub skipped_categories: Value,
    #[serde(default)]
    pub findings: Vec<AdversarialReviewFindingInput>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialReviewFindingInput {
    pub severity: i64,
    pub status: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub evidence: Value,
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialReviewRun {
    pub id: String,
    pub packet_id: Option<String>,
    pub scope: String,
    pub title: String,
    pub reviewer: String,
    pub requested_proof_level: String,
    pub judgment: String,
    pub summary: String,
    pub strongest_fake_done_path: String,
    pub refutations: Value,
    pub skipped_categories: Value,
    pub metadata: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialReviewFinding {
    pub id: String,
    pub review_id: String,
    pub severity: i64,
    pub status: String,
    pub title: String,
    pub body: String,
    pub evidence: Value,
    pub recommendation: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialReviewReport {
    pub review: AdversarialReviewRun,
    pub findings: Vec<AdversarialReviewFinding>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialReviewSummary {
    pub id: String,
    pub packet_id: Option<String>,
    pub scope: String,
    pub title: String,
    pub reviewer: String,
    pub requested_proof_level: String,
    pub judgment: String,
    pub finding_count: usize,
    pub blocking_finding_count: usize,
    pub created_at: String,
}
