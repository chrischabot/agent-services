use crate::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub sensitivity: BackupSensitivity,
    #[serde(default)]
    pub x: BackupXSummary,
    pub files: Vec<BackupFile>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupSensitivity {
    pub contains_local_secret_values: bool,
    pub local_secret_value_count: usize,
    pub policy: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupXSummary {
    pub canonical_tweets: i64,
    pub portable_export_status: String,
    pub portable_export_missing: bool,
    pub portable_export_stale: bool,
    pub portable_rows_exported: Option<usize>,
    pub portable_generated_at: Option<String>,
    pub portable_manifest_sha256: Option<String>,
    pub portable_bundle_included: bool,
    pub recovery_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupFile {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupVerification {
    pub ok: bool,
    pub path: String,
    pub created_at: String,
    pub sensitivity: BackupSensitivity,
    pub x: BackupXSummary,
    pub checked_files: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRestoreReport {
    pub ok: bool,
    pub backup_path: String,
    pub target_home: String,
    pub restored_files: usize,
    pub x: BackupXSummary,
}

impl BackupManifest {
    pub fn from_dir(dir: &Path) -> Result<Self> {
        let mut files = Vec::new();
        for entry in WalkDir::new(dir) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.file_name().is_some_and(|name| name == "manifest.json") {
                continue;
            }
            let bytes = fs::read(path)?;
            files.push(BackupFile {
                path: path.strip_prefix(dir)?.to_string_lossy().to_string(),
                bytes: bytes.len() as u64,
                sha256: sha256(&bytes),
            });
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(Self {
            version: 1,
            created_at: Utc::now(),
            sensitivity: BackupSensitivity {
                policy: "local backup may contain private Arcwell data; SQLite snapshots are sensitive when secret values exist".to_string(),
                ..BackupSensitivity::default()
            },
            x: BackupXSummary {
                portable_export_status: "unknown".to_string(),
                recovery_note: "Backup manifest predates X recovery metadata or was created outside Arcwell's current backup writer.".to_string(),
                ..BackupXSummary::default()
            },
            files,
        })
    }
}

pub fn verify_backup_path(path: &Path) -> Result<BackupVerification> {
    let manifest_path = path.join("manifest.json");
    let manifest_bytes =
        fs::read(&manifest_path).with_context(|| format!("reading {}", manifest_path.display()))?;
    let manifest: BackupManifest = serde_json::from_slice(&manifest_bytes)
        .with_context(|| format!("parsing {}", manifest_path.display()))?;

    let mut errors = Vec::new();
    if manifest.version != 1 {
        errors.push(format!(
            "unsupported backup manifest version: {}",
            manifest.version
        ));
    }
    for file in &manifest.files {
        let relative = match safe_backup_relative_path(&file.path) {
            Ok(relative) => relative,
            Err(error) => {
                errors.push(error.to_string());
                continue;
            }
        };
        let file_path = path.join(relative);
        match fs::read(&file_path) {
            Ok(bytes) => {
                if bytes.len() as u64 != file.bytes {
                    errors.push(format!(
                        "{} byte mismatch: expected {}, got {}",
                        file.path,
                        file.bytes,
                        bytes.len()
                    ));
                }
                if sha256(&bytes) != file.sha256 {
                    errors.push(format!("{} sha256 mismatch", file.path));
                }
            }
            Err(error) => errors.push(format!("{} missing/unreadable: {error}", file.path)),
        }
    }

    let checked_files = manifest.files.len();
    Ok(BackupVerification {
        ok: errors.is_empty(),
        path: path.to_string_lossy().to_string(),
        created_at: manifest.created_at.to_rfc3339(),
        sensitivity: manifest.sensitivity,
        x: manifest.x,
        checked_files,
        errors,
    })
}

pub(crate) fn safe_backup_relative_path(path: &str) -> Result<PathBuf> {
    let relative = PathBuf::from(path);
    if relative.is_absolute() {
        bail!("backup manifest path must be relative: {path}");
    }
    if relative
        .components()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        bail!("backup manifest path contains unsafe components: {path}");
    }
    Ok(relative)
}
