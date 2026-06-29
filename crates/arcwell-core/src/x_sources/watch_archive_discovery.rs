use super::*;

pub(crate) fn validate_watch_source_input(input: &WatchSourceInput) -> Result<()> {
    validate_watch_source_kind(&input.source_kind)?;
    validate_watch_source_cadence(&input.cadence)?;
    validate_watch_source_status(&input.status)?;
    validate_query(&input.label)?;
    if input.locator.trim().is_empty() {
        bail!("watch source locator cannot be empty");
    }
    if input.locator.len() > 1_000 {
        bail!("watch source locator is too long");
    }
    match input.source_kind.as_str() {
        "github_owner" => validate_github_segment(&input.locator)?,
        "rss" | "blog" => {
            validate_fetch_url(&input.locator)?;
        }
        "arxiv_query" => validate_query(&input.locator)?,
        "hackernews" => {
            normalize_hackernews_feed(&input.locator)?;
        }
        "reddit" => {
            normalize_reddit_locator(&input.locator)?;
        }
        "x_bookmarks" => {
            if input.locator != "bookmarks" {
                bail!("x_bookmarks watch source locator must be bookmarks");
            }
        }
        "x_handle" => validate_x_handle(&input.locator)?,
        "knowledge_backlog" => {
            if input.locator != "source-cards" {
                bail!("knowledge_backlog watch source locator must be source-cards");
            }
        }
        "knowledge_model_clusters" => validate_query(&input.locator)?,
        "knowledge_model_write" => validate_id(&input.locator)?,
        "knowledge_entity_resolution" => {
            if input.locator != "entities" {
                bail!("knowledge_entity_resolution watch source locator must be entities");
            }
        }
        "job_radar" => validate_id(&input.locator)?,
        _ => unreachable!("source kind validated above"),
    }
    Ok(())
}

pub(crate) fn validate_watch_source_kind(kind: &str) -> Result<()> {
    match kind {
        "rss"
        | "blog"
        | "github_owner"
        | "arxiv_query"
        | "hackernews"
        | "reddit"
        | "x_bookmarks"
        | "x_handle"
        | "knowledge_backlog"
        | "knowledge_model_clusters"
        | "knowledge_model_write"
        | "knowledge_entity_resolution"
        | "job_radar" => Ok(()),
        other => bail!("unsupported watch source kind: {other}"),
    }
}

pub(crate) fn validate_watch_source_cadence(cadence: &str) -> Result<()> {
    match cadence {
        "hot" | "warm" | "cold" => Ok(()),
        other => bail!("unsupported watch source cadence: {other}"),
    }
}

pub(crate) fn watch_source_cadence_seconds(cadence: &str) -> Option<i64> {
    match cadence {
        "hot" => Some(60 * 60),
        "warm" => Some(6 * 60 * 60),
        "cold" => Some(24 * 60 * 60),
        _ => None,
    }
}

pub(crate) fn validate_watch_source_status(status: &str) -> Result<()> {
    match status {
        "active" | "paused" | "error" => Ok(()),
        other => bail!("unsupported watch source status: {other}"),
    }
}

pub(crate) fn validate_x_handle(handle: &str) -> Result<()> {
    validate_key(handle)?;
    if !handle
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        bail!("invalid X handle");
    }
    Ok(())
}

pub(crate) fn watch_source_id(source_kind: &str, locator: &str) -> String {
    let hash = sha256(format!("{source_kind}\n{locator}").as_bytes());
    format!("watch-{}", &hash[..32])
}

pub(crate) fn watch_source_health_key(source: &WatchSource) -> Result<String> {
    match source.source_kind.as_str() {
        "rss" => Ok(format!("rss:{}", canonical_source_url(&source.locator)?)),
        "blog" => Ok(format!("blog:{}", canonical_source_url(&source.locator)?)),
        "github_owner" => Ok(format!("github-owner:{}", source.locator)),
        "arxiv_query" => Ok(format!("arxiv:{}", source.locator)),
        "hackernews" => Ok(format!(
            "hackernews:{}",
            normalize_hackernews_feed(&source.locator)?
        )),
        "reddit" => Ok(format!(
            "reddit:{}",
            normalize_reddit_locator(&source.locator)?.source_detail()
        )),
        "x_bookmarks" => Ok("x:bookmarks".to_string()),
        "x_handle" => Ok(format!("x:watch:{}", source.locator)),
        "knowledge_backlog" => Ok("knowledge:source-card-backlog".to_string()),
        "knowledge_model_clusters" => Ok(format!("knowledge:model-clusters:{}", source.locator)),
        "knowledge_model_write" => Ok(format!("knowledge:model-write:{}", source.locator)),
        "knowledge_entity_resolution" => Ok("knowledge:entity-resolution:entities".to_string()),
        "job_radar" => Ok(format!("job:radar:{}", source.locator)),
        other => bail!("unsupported watch source kind: {other}"),
    }
}

pub(crate) fn timestamp_is_due(timestamp: &str) -> bool {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|parsed| parsed.with_timezone(&Utc) <= Utc::now())
        .unwrap_or(true)
}

pub(crate) fn validate_timestamp(timestamp: &str) -> Result<()> {
    DateTime::parse_from_rfc3339(timestamp)
        .with_context(|| format!("invalid RFC3339 timestamp: {timestamp}"))?;
    Ok(())
}

pub(crate) fn canonical_json(value: &Value) -> Result<String> {
    serde_json::to_string(value).map_err(Into::into)
}

#[derive(Debug)]
pub(crate) struct XItemInput {
    pub(crate) x_id: String,
    pub(crate) author: String,
    pub(crate) text: String,
    pub(crate) url: String,
    pub(crate) created_at: Option<String>,
    pub(crate) conversation_id: Option<String>,
    pub(crate) reply_to_x_id: Option<String>,
    pub(crate) quote_x_id: Option<String>,
    pub(crate) retweet_x_id: Option<String>,
    pub(crate) retrieved_at: Option<String>,
    pub(crate) metrics: Value,
    pub(crate) raw: Value,
    pub(crate) source_kind: String,
    pub(crate) source_detail: Option<String>,
    pub(crate) source_metadata: Value,
}

#[derive(Debug)]
pub(crate) struct RepairableXTweetProjection {
    pub(crate) x_id: String,
    pub(crate) author: String,
    pub(crate) text: String,
    pub(crate) url: String,
    pub(crate) created_at: Option<String>,
    pub(crate) retrieved_at: Option<String>,
    pub(crate) metrics: Value,
    pub(crate) projection_status: String,
    pub(crate) source_card_id: Option<String>,
    pub(crate) wiki_page_id: Option<String>,
    pub(crate) existing_source_card_id: Option<String>,
    pub(crate) existing_wiki_page_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalXThreadTweet {
    pub(crate) x_id: String,
    pub(crate) author: String,
    pub(crate) text: String,
    pub(crate) url: String,
    pub(crate) created_at: Option<String>,
    pub(crate) first_seen_at: String,
    pub(crate) conversation_id: Option<String>,
    pub(crate) reply_to_x_id: Option<String>,
    pub(crate) quote_x_id: Option<String>,
    pub(crate) retweet_x_id: Option<String>,
    pub(crate) source_card_id: Option<String>,
    pub(crate) wiki_page_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct XLinkCandidate {
    pub(crate) url: String,
    pub(crate) expanded_url: Option<String>,
    pub(crate) display_url: Option<String>,
    pub(crate) source: String,
    pub(crate) raw: Value,
}

#[derive(Debug)]
pub(crate) struct XArchiveCollectedItems {
    pub(crate) files_seen: usize,
    pub(crate) files_imported: usize,
    pub(crate) bytes_read: usize,
    pub(crate) skipped_files: usize,
    pub(crate) unsupported_slices: BTreeMap<String, usize>,
    pub(crate) unsupported_files: Vec<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) items: Vec<Value>,
}

pub(crate) fn x_source_card_input_from_repair(
    candidate: &RepairableXTweetProjection,
) -> SourceCardInput {
    SourceCardInput {
        title: format!("X: {} {}", candidate.author, candidate.x_id),
        url: candidate.url.clone(),
        source_type: "x".to_string(),
        provider: "x".to_string(),
        summary: candidate.text.clone(),
        claims: vec![SourceClaim {
            claim: candidate.text.clone(),
            kind: "source_text".to_string(),
            confidence: 1.0,
        }],
        retrieved_at: candidate.retrieved_at.clone(),
        metadata: json!({
            "x_id": candidate.x_id.clone(),
            "author": candidate.author.clone(),
            "created_at": candidate.created_at.clone(),
            "source_kind": "projection_repair",
            "source_detail": null,
            "metrics": candidate.metrics.clone(),
            "projection_repair": true
        }),
    }
}

pub(crate) fn discover_x_archives(
    roots: &[PathBuf],
    limit: usize,
) -> Result<XArchiveDiscoveryReport> {
    let search_roots = if roots.is_empty() {
        default_x_archive_discovery_roots()
    } else {
        roots.to_vec()
    };
    let limit = limit.clamp(1, 1_000);
    let mut report = XArchiveDiscoveryReport {
        generated_at: now(),
        roots: search_roots
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        inspected_paths: 0,
        candidates: Vec::new(),
        warnings: Vec::new(),
    };
    for root in &search_roots {
        if report.inspected_paths >= X_ARCHIVE_DISCOVERY_MAX_PATHS
            || report.candidates.len() >= limit
        {
            break;
        }
        if !root.exists() {
            report
                .warnings
                .push(format!("root does not exist: {}", root.display()));
            continue;
        }
        if root.is_file() {
            report.inspected_paths += 1;
            if let Some(candidate) = inspect_x_archive_discovery_path(root)? {
                report.candidates.push(candidate);
            }
            continue;
        }
        for entry in WalkDir::new(root).max_depth(5).follow_links(false) {
            if report.inspected_paths >= X_ARCHIVE_DISCOVERY_MAX_PATHS
                || report.candidates.len() >= limit
            {
                break;
            }
            let entry = entry?;
            if entry.file_type().is_dir() {
                continue;
            }
            report.inspected_paths += 1;
            if entry.file_type().is_symlink() {
                continue;
            }
            if let Some(candidate) = inspect_x_archive_discovery_path(entry.path())? {
                report.candidates.push(candidate);
            }
        }
    }
    report.candidates.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.path.cmp(&right.path))
    });
    report.candidates.truncate(limit);
    Ok(report)
}

pub(crate) fn default_x_archive_discovery_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.join("Downloads"));
        roots.push(home.join("Desktop"));
        roots.push(home.join("Documents"));
    }
    roots
}

pub(crate) fn inspect_x_archive_discovery_path(
    path: &Path,
) -> Result<Option<XArchiveDiscoveryCandidate>> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let name_hint = x_archive_name_hint_score(&name);
    let supported_by_name = x_archive_supported_slices_from_name(&name);
    let likely_by_name =
        name_hint > 0.0 || !supported_by_name.is_empty() || name.contains("twitter");
    let metadata = fs::metadata(path).ok();
    let size_bytes = metadata.as_ref().map(fs::Metadata::len);
    let modified_at = metadata
        .and_then(|metadata| metadata.modified().ok())
        .map(|modified| DateTime::<Utc>::from(modified).to_rfc3339());

    let mut evidence = Vec::new();
    let mut warnings = Vec::new();
    let mut supported_slices = supported_by_name;
    let mut score = name_hint;
    let kind = if extension == "zip" {
        evidence.push("zip extension".to_string());
        score += 1.0;
        match inspect_x_archive_zip_members(path) {
            Ok((slices, member_evidence, member_warnings)) => {
                for slice in slices {
                    supported_slices.insert(slice);
                }
                evidence.extend(member_evidence);
                warnings.extend(member_warnings);
            }
            Err(error) => {
                warnings.push(format!(
                    "zip shallow inspection failed: {}",
                    excerpt(&error.to_string(), 240)
                ));
            }
        }
        "zip".to_string()
    } else if extension == "js" || extension == "json" {
        if !likely_by_name {
            return Ok(None);
        }
        evidence.push(format!("{extension} archive-slice extension"));
        "file".to_string()
    } else {
        if !likely_by_name {
            return Ok(None);
        }
        "file".to_string()
    };
    if name.contains("twitter") {
        evidence.push("filename mentions twitter".to_string());
        score += 2.0;
    }
    if name.contains("archive") {
        evidence.push("filename mentions archive".to_string());
        score += 2.0;
    }
    if name.contains("tweet") || name.contains("bookmark") || name.contains("like") {
        evidence.push("filename mentions supported archive slice".to_string());
        score += 2.0;
    }
    if !supported_slices.is_empty() {
        score += supported_slices.len() as f64;
    }
    if !likely_by_name && supported_slices.is_empty() && score < 3.0 {
        return Ok(None);
    }
    let mut supported_slices = supported_slices.into_iter().collect::<Vec<_>>();
    supported_slices.sort();
    evidence.sort();
    evidence.dedup();
    warnings.sort();
    warnings.dedup();
    Ok(Some(XArchiveDiscoveryCandidate {
        path: path.display().to_string(),
        kind,
        score,
        size_bytes,
        modified_at,
        supported_slices,
        evidence,
        warnings,
    }))
}

pub(crate) fn x_archive_name_hint_score(name: &str) -> f64 {
    let mut score = 0.0;
    if name.contains("twitter") {
        score += 2.0;
    }
    if name.contains("archive") {
        score += 2.0;
    }
    if name.contains("tweet") || name.contains("bookmark") || name.contains("like") {
        score += 1.0;
    }
    score
}

pub(crate) fn x_archive_supported_slices_from_name(name: &str) -> BTreeSet<String> {
    let mut slices = BTreeSet::new();
    if name.contains("tweet") {
        slices.insert("tweets".to_string());
    }
    if name.contains("bookmark") {
        slices.insert("bookmarks".to_string());
    }
    if name.contains("like") || name.contains("favorite") {
        slices.insert("likes".to_string());
    }
    slices
}

pub(crate) fn inspect_x_archive_zip_members(
    path: &Path,
) -> Result<(BTreeSet<String>, Vec<String>, Vec<String>)> {
    let file = fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file).context("opening X archive zip")?;
    let mut slices = BTreeSet::new();
    let mut evidence = Vec::new();
    let mut warnings = Vec::new();
    if archive.len() > X_ARCHIVE_DISCOVERY_MAX_ZIP_ENTRIES {
        warnings.push(format!(
            "zip has {} entries; inspected first {} names only",
            archive.len(),
            X_ARCHIVE_DISCOVERY_MAX_ZIP_ENTRIES
        ));
    }
    for index in 0..archive.len().min(X_ARCHIVE_DISCOVERY_MAX_ZIP_ENTRIES) {
        let member = archive.by_index(index)?;
        let name = member.name().to_string();
        match safe_x_archive_member_name(&name) {
            Ok(safe_name) => {
                let member_slices =
                    x_archive_supported_slices_from_name(&safe_name.to_ascii_lowercase());
                if !member_slices.is_empty() {
                    evidence.push(format!("member {safe_name} names supported slice"));
                }
                slices.extend(member_slices);
                if let Some(kind) = x_archive_unsupported_slice_kind(&safe_name) {
                    warnings.push(format!(
                        "member {safe_name} names unsupported slice {kind}; discovery does not imply import support"
                    ));
                }
            }
            Err(_) => warnings.push(format!(
                "unsafe member path observed during shallow scan: {}",
                excerpt(&name, 160)
            )),
        }
    }
    Ok((slices, evidence, warnings))
}
