use super::*;

#[test]
fn candidate_apply_to_profile() {
    let store = test_store("candidate");
    let id = store
        .add_candidate(
            "profile",
            "communication.preference",
            "consult memory before personalized answers",
            "normal",
            "test",
        )
        .unwrap();
    store.apply_candidate(&id).unwrap();
    assert!(
        store
            .get_profile("communication.preference")
            .unwrap()
            .is_some()
    );
}

#[test]
fn severe_candidate_unknown_target_does_not_mark_applied() {
    let store = test_store("candidate-invalid-target");
    let id = store
        .add_candidate(
            "admin",
            "privilege",
            "make me trusted",
            "sensitive",
            "malicious:test",
        )
        .unwrap();

    assert!(store.apply_candidate(&id).is_err());
    let pending = store.list_candidates("pending").unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, id);
}

#[test]
fn wiki_ingest_and_search() {
    let store = test_store("wiki");
    let source = store.paths().home.join("source.md");
    fs::write(
        &source,
        "# Vercel Eve\n\nEve is a launch worth tracking for agent infrastructure.",
    )
    .unwrap();
    let id = store.ingest_wiki_file(&source).unwrap();
    let page = store.read_wiki_page(&id).unwrap().unwrap();
    assert_eq!(page.title, "Vercel Eve");
    assert_eq!(
        store
            .search_wiki_pages("agent infrastructure")
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn wiki_fts_index_handles_punctuation_heavy_queries() {
    let store = test_store("wiki-fts");
    store
        .add_wiki_page(
            "A2A vs MCP vs AG-UI",
            "# A2A vs MCP vs AG-UI\n\nAgent protocol comparison for coding agents.",
            "test",
        )
        .unwrap();

    assert_eq!(store.search_wiki_pages("A2A/MCP").unwrap().len(), 1);
    assert_eq!(store.search_wiki_pages("coding-agent").unwrap().len(), 1);
}

#[test]
fn wiki_ingest_dir_imports_markdown_and_skips_other_files() {
    let store = test_store("wiki-dir");
    let root = store.paths().home.join("corpus");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("alpha.md"), "# Alpha\n\nDeveloper relations.").unwrap();
    fs::write(
        root.join("nested").join("beta.markdown"),
        "# Beta\n\nCoding agents.",
    )
    .unwrap();
    fs::write(root.join("notes.txt"), "not imported").unwrap();

    let report = store.ingest_wiki_dir(&root).unwrap();
    assert_eq!(report.imported, 2);
    assert_eq!(report.skipped, 1);
    assert_eq!(
        store
            .search_wiki_pages("developer relations")
            .unwrap()
            .len(),
        1
    );
    assert_eq!(store.search_wiki_pages("coding agents").unwrap().len(), 1);
}

#[test]
fn severe_wiki_sync_marks_deleted_markdown_pages_inactive() {
    // CLAIM: incremental Markdown sync does not leave deleted source files as live evidence.
    // PRECONDITIONS: A synced directory had two Markdown files, then one source file disappeared.
    // POSTCONDITIONS: The missing file's wiki page is tombstoned, removed from FTS, and the live file remains searchable.
    // ORACLE: sync report, read_wiki_page status, list/search active filters.
    // SEVERITY: Severe because stale local files can otherwise keep grounding research after deletion.
    let store = test_store("wiki-sync-delete");
    let root = store.paths().home.join("corpus");
    fs::create_dir_all(&root).unwrap();
    let keep = root.join("keep.md");
    let gone = root.join("gone.md");
    fs::write(&keep, "# Keep\n\nDurable live evidence.").unwrap();
    fs::write(&gone, "# Gone\n\nDeleted stale evidence.").unwrap();

    let first = store.sync_wiki_dir(&root).unwrap();
    assert_eq!(first.imported, 2);
    assert_eq!(first.deleted, 0);
    let gone_id = store
        .list_wiki_pages()
        .unwrap()
        .into_iter()
        .find(|page| page.title == "Gone")
        .unwrap()
        .id;

    fs::remove_file(&gone).unwrap();
    let second = store.sync_wiki_dir(&root).unwrap();
    assert_eq!(second.imported, 1);
    assert_eq!(second.deleted, 1);
    assert_eq!(second.deleted_page_ids, vec![gone_id.clone()]);

    let gone_page = store.read_wiki_page(&gone_id).unwrap().unwrap();
    assert_eq!(gone_page.status, "deleted");
    assert_eq!(
        store
            .search_wiki_pages("Deleted stale evidence")
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        store
            .search_wiki_pages("Durable live evidence")
            .unwrap()
            .len(),
        1
    );
    assert_eq!(store.list_wiki_pages().unwrap().len(), 1);
}

#[test]
fn codex_swift_source_import_merges_richer_seed_data_idempotently() {
    let store = test_store("codex-swift-sources");
    let root = store.paths().home.join("codex-swift");
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::write(
        root.join("scripts").join("wiki-sources-restore.sh"),
        r#"
FEEDS=(
  "https://www.latent.space/feed"
  "http://127.0.0.1/feed"
)
GITHUB=(
  openai x-ai
)
BLOGS=(
  "https://openai.com/news/"
)
ARXIV=( "cat:cs.AI" )
"#,
    )
    .unwrap();
    fs::write(
        root.join("llm-wiki.md"),
        r#"
### 14.8 Seed watch list — AI / coding-agent orgs & people

| Handle | Kind | Ships / why monitor | Cadence |
|---|---|---|---|
| `openai` | org | OpenAI coding-agent releases | hot |
| `simonw` | user | Simon Willison agent notes | cold |
| `../evil` | org | path traversal attempt | hot |
| `badcadence` | org | invalid cadence | hourly |

### 14.9 Seed source feeds — from agentwiki
"#,
    )
    .unwrap();

    let first = store.import_codex_swift_sources(&root).unwrap();
    assert_eq!(first.added, 6);
    assert_eq!(first.updated, 0);
    assert_eq!(first.unchanged, 0);
    assert_eq!(first.skipped, 3);
    assert_eq!(first.by_kind.get("github_owner"), Some(&3));
    assert_eq!(first.by_kind.get("rss"), Some(&1));
    assert_eq!(first.by_kind.get("blog"), Some(&1));
    assert_eq!(first.by_kind.get("arxiv_query"), Some(&1));

    let sources = store.list_watch_sources().unwrap();
    assert_eq!(sources.len(), 6);
    let openai = sources
        .iter()
        .find(|source| source.source_kind == "github_owner" && source.locator == "openai")
        .expect("openai source imported");
    assert_eq!(openai.cadence, "hot");
    assert_eq!(openai.metadata["origin"], "codex-swift/llm-wiki.md");
    assert!(
        sources
            .iter()
            .any(|source| { source.source_kind == "github_owner" && source.locator == "x-ai" })
    );

    let second = store.import_codex_swift_sources(&root).unwrap();
    assert_eq!(second.added, 0);
    assert_eq!(second.updated, 0);
    assert_eq!(second.unchanged, 6);
    assert_eq!(store.list_watch_sources().unwrap().len(), 6);
}

#[test]
fn severe_watch_source_rejects_unsafe_and_unsupported_locators() {
    let store = test_store("watch-source-invalid");
    let unsafe_rss = store.upsert_watch_source(WatchSourceInput {
        source_kind: "rss".to_string(),
        locator: "http://169.254.169.254/latest/meta-data".to_string(),
        label: "metadata".to_string(),
        cadence: "hot".to_string(),
        status: "active".to_string(),
        metadata: json!({}),
    });
    assert!(unsafe_rss.is_err());

    let bad_kind = store.upsert_watch_source(WatchSourceInput {
        source_kind: "github_repo".to_string(),
        locator: "openai/codex".to_string(),
        label: "wrong layer".to_string(),
        cadence: "hot".to_string(),
        status: "active".to_string(),
        metadata: json!({}),
    });
    assert!(bad_kind.is_err());

    let bad_handle = store.upsert_watch_source(WatchSourceInput {
        source_kind: "github_owner".to_string(),
        locator: "../openai".to_string(),
        label: "path traversal".to_string(),
        cadence: "hot".to_string(),
        status: "active".to_string(),
        metadata: json!({}),
    });
    assert!(bad_handle.is_err());

    let bad_hn_feed = store.upsert_watch_source(WatchSourceInput {
        source_kind: "hackernews".to_string(),
        locator: "private-feed".to_string(),
        label: "unknown HN feed".to_string(),
        cadence: "hot".to_string(),
        status: "active".to_string(),
        metadata: json!({}),
    });
    assert!(bad_hn_feed.is_err());

    let bad_reddit = store.upsert_watch_source(WatchSourceInput {
        source_kind: "reddit".to_string(),
        locator: "../private".to_string(),
        label: "bad reddit".to_string(),
        cadence: "hot".to_string(),
        status: "active".to_string(),
        metadata: json!({}),
    });
    assert!(bad_reddit.is_err());
    assert!(store.list_watch_sources().unwrap().is_empty());
}

#[test]
fn severe_wiki_title_cannot_escape_wiki_directory() {
    let store = test_store("wiki-path");
    let id = store
        .add_wiki_page(
            "../../outside/evil",
            "# ../../outside/evil\n\nPath traversal attempt.",
            "test",
        )
        .unwrap();
    let page = store.read_wiki_page(&id).unwrap().unwrap();
    let page_path = PathBuf::from(page.path);
    assert!(page_path.starts_with(&store.paths().wiki_pages));
    assert!(
        page_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("outside")
    );
    assert!(!store.paths().home.join("outside").exists());
}

#[test]
fn severe_backup_includes_wiki_pages_and_verifies_tampering() {
    let store = test_store("backup-wiki");
    store
        .add_wiki_page(
            "Backup Coverage",
            "# Backup Coverage\n\nWiki pages must be backed up with SQLite.",
            "test",
        )
        .unwrap();

    let backup_path = store.create_backup().unwrap();
    let verification = store.verify_backup_path(&backup_path).unwrap();
    assert!(verification.ok);
    assert!(
        backup_path
            .join("wiki")
            .join("pages")
            .read_dir()
            .unwrap()
            .next()
            .is_some()
    );

    let copied_page = backup_path
        .join("wiki")
        .join("pages")
        .read_dir()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(copied_page, "tampered").unwrap();
    let verification = store.verify_backup_path(&backup_path).unwrap();
    assert!(!verification.ok);
    assert!(
        verification
            .errors
            .iter()
            .any(|error| error.contains("sha256 mismatch"))
    );
}

#[test]
fn severe_backup_restore_round_trips_durable_state() {
    let store = test_store("backup-restore-source");
    store
        .set_profile("communication.style", "direct", "normal", "test")
        .unwrap();
    store
        .add_memory("My cat is called Ophelia", "fact", "normal", "test", 0.9)
        .unwrap();
    store
        .mem0_add_memory(
            "My cat is called Ophelia",
            Some("restore-user"),
            "restore-test",
            "normal",
            false,
        )
        .unwrap();
    let wiki_page_id = store
        .add_wiki_page(
            "Restore Drill",
            "# Restore Drill\n\nThis page must survive backup restore.",
            "test",
        )
        .unwrap();
    let source_card = store
        .add_source_card(SourceCardInput {
            title: "Restore Source".to_string(),
            url: "https://example.com/restore".to_string(),
            source_type: "web".to_string(),
            provider: "test".to_string(),
            summary: "Restore source card summary.".to_string(),
            claims: vec![SourceClaim {
                claim: "Restore should preserve source cards.".to_string(),
                kind: "test".to_string(),
                confidence: 1.0,
            }],
            retrieved_at: Some(now()),
            metadata: json!({ "test": true }),
        })
        .unwrap();
    let project = store
        .create_project(
            "Restore Project",
            "Project must survive restore.",
            &["restore".to_string()],
        )
        .unwrap();
    store
        .record_channel_message(
            "telegram",
            "incoming",
            "user:1",
            "How is restore going?",
            Some(&project.id),
            None,
        )
        .unwrap();
    store
        .run_wiki_compile_job("restore drill")
        .expect("job state should enter backup");

    let backup_path = store.create_backup().unwrap();
    let target_paths = AppPaths::new(
        std::env::temp_dir().join(format!("arcwell-test-restore-target-{}", Uuid::new_v4())),
    );
    let report = Store::restore_backup_path(&backup_path, &target_paths, false).unwrap();
    assert!(report.ok);

    let restored = Store::open(target_paths).unwrap();
    assert_eq!(
        restored
            .get_profile("communication.style")
            .unwrap()
            .unwrap()
            .value,
        "direct"
    );
    assert_eq!(restored.search_memories("Ophelia").unwrap().len(), 1);
    let restored_mem0 = restored
        .mem0_search_memories("Ophelia", Some("restore-user"), 10)
        .unwrap();
    assert_eq!(
        restored_mem0
            .results
            .get("results")
            .and_then(Value::as_array)
            .unwrap()
            .len(),
        1,
        "mem0-rs vector/history artifacts must survive backup restore"
    );
    assert_eq!(
        restored
            .read_wiki_page(&wiki_page_id)
            .unwrap()
            .unwrap()
            .title,
        "Restore Drill"
    );
    assert!(
        restored
            .read_source_card(&source_card.id)
            .unwrap()
            .is_some()
    );
    assert_eq!(restored.list_projects().unwrap().len(), 1);
    assert_eq!(restored.list_channel_messages().unwrap().len(), 1);
    assert_eq!(restored.list_wiki_jobs().unwrap().len(), 1);
}

#[test]
fn severe_backup_manifest_records_x_portable_recovery_state_and_restore_search() {
    // CLAIM: backups make X recovery state explicit instead of implying a
    // portable bundle was included.
    // PRECONDITIONS: canonical X rows exist but no portable export has run.
    // POSTCONDITIONS: backup manifest/verification/restore report flag missing
    // portable export while a disposable restore can still search canonical X
    // rows from the SQLite backup.
    // SEVERITY: Severe because backup success and portable export success are
    // different recovery claims.
    let store = test_store("backup-x-portable-missing");
    store
        .import_x_json_value(&json!([
            {
                "id": "backup-x-1",
                "author": "arcwell",
                "text": "Backup X portable recovery proof searchable after restore.",
                "url": "https://x.com/arcwell/status/backup-x-1",
                "source_kind": "json_import"
            }
        ]))
        .unwrap();

    let backup_path = store.create_backup().unwrap();
    let manifest: BackupManifest =
        serde_json::from_slice(&fs::read(backup_path.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest.x.canonical_tweets, 1);
    assert_eq!(manifest.x.portable_export_status, "missing");
    assert!(manifest.x.portable_export_missing);
    assert!(!manifest.x.portable_bundle_included);
    assert!(manifest.x.recovery_note.contains("SQLite backup includes"));

    let verification = store.verify_backup_path(&backup_path).unwrap();
    assert!(verification.ok);
    assert_eq!(verification.x.portable_export_status, "missing");

    let target_paths = AppPaths::new(
        std::env::temp_dir().join(format!("arcwell-test-backup-x-target-{}", Uuid::new_v4())),
    );
    let report = Store::restore_backup_path(&backup_path, &target_paths, false).unwrap();
    assert!(report.ok);
    assert_eq!(report.x.portable_export_status, "missing");

    let restored = Store::open(target_paths).unwrap();
    let search = restored
        .search_x_tweets("portable recovery proof", 10)
        .unwrap();
    assert_eq!(search.len(), 1);
    assert_eq!(search[0].x_id, "backup-x-1");
    assert!(search[0].source_card_id.is_some());
}

#[test]
fn severe_backup_manifest_flags_stale_x_portable_export() {
    // CLAIM: backup manifests preserve the portable export freshness judgment
    // at backup time.
    // PRECONDITIONS: a portable export succeeded, then canonical tweet state
    // changed before backup.
    // POSTCONDITIONS: backup verification reports stale portable export status,
    // row counts, and manifest hash without bundling portable bytes.
    // SEVERITY: Severe because a fresh backup must not hide stale review/export
    // artifacts.
    let store = test_store("backup-x-portable-stale");
    store
        .import_x_json_value(&json!([
            {
                "id": "backup-x-stale",
                "author": "arcwell",
                "text": "Backup stale portable export proof.",
                "url": "https://x.com/arcwell/status/backup-x-stale",
                "source_kind": "json_import"
            }
        ]))
        .unwrap();
    store
        .export_x_portable(&store.paths().home.join("portable-x"))
        .unwrap();
    store
        .conn
        .execute(
            "UPDATE x_tweets SET updated_at = ?1 WHERE x_id = ?2",
            params!["9999-01-02T00:00:00Z", "backup-x-stale"],
        )
        .unwrap();

    let backup_path = store.create_backup().unwrap();
    let verification = store.verify_backup_path(&backup_path).unwrap();
    assert!(verification.ok);
    assert_eq!(verification.x.canonical_tweets, 1);
    assert_eq!(verification.x.portable_export_status, "stale");
    assert!(verification.x.portable_export_stale);
    assert_eq!(verification.x.portable_rows_exported, Some(1));
    assert!(verification.x.portable_manifest_sha256.is_some());
    assert!(!verification.x.portable_bundle_included);
}

#[test]
fn severe_backup_restore_refuses_non_empty_target_without_replace() {
    let store = test_store("backup-restore-refuse");
    store
        .set_profile("restore.test", "value", "normal", "test")
        .unwrap();
    let backup_path = store.create_backup().unwrap();
    let target_paths = AppPaths::new(
        std::env::temp_dir().join(format!("arcwell-test-restore-refuse-{}", Uuid::new_v4())),
    );
    fs::create_dir_all(&target_paths.home).unwrap();
    fs::write(target_paths.home.join("keep.txt"), "do not overwrite").unwrap();

    let error = Store::restore_backup_path(&backup_path, &target_paths, false)
        .expect_err("restore must refuse non-empty target without replace");
    assert!(error.to_string().contains("not empty"));

    Store::restore_backup_path(&backup_path, &target_paths, true).unwrap();
    assert!(
        Store::open(target_paths)
            .unwrap()
            .get_profile("restore.test")
            .unwrap()
            .is_some()
    );
}

#[test]
fn severe_backup_restore_rejects_manifest_path_traversal() {
    let store = test_store("backup-restore-traversal");
    store
        .set_profile("restore.test", "value", "normal", "test")
        .unwrap();
    let backup_path = store.create_backup().unwrap();
    let manifest_path = backup_path.join("manifest.json");
    let mut manifest: BackupManifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest.files[0].path = "../escape.txt".to_string();
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let verification = store.verify_backup_path(&backup_path).unwrap();
    assert!(!verification.ok);
    assert!(
        verification
            .errors
            .iter()
            .any(|error| error.contains("unsafe components"))
    );
    let target_paths = AppPaths::new(
        std::env::temp_dir().join(format!("arcwell-test-restore-traversal-{}", Uuid::new_v4())),
    );
    assert!(Store::restore_backup_path(&backup_path, &target_paths, false).is_err());
    assert!(!target_paths.home.join("..").join("escape.txt").exists());
}

#[test]
fn severe_backup_verification_detects_missing_files_and_bad_manifest_version() {
    let store = test_store("backup-missing-file");
    store
        .add_wiki_page(
            "Missing File",
            "# Missing File\n\nMust be verified.",
            "test",
        )
        .unwrap();
    let backup_path = store.create_backup().unwrap();
    let manifest_path = backup_path.join("manifest.json");
    let mut manifest: BackupManifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    let wiki_file = manifest
        .files
        .iter()
        .find(|file| file.path.starts_with("wiki/pages/"))
        .expect("wiki page included in manifest")
        .path
        .clone();
    fs::remove_file(backup_path.join(&wiki_file)).unwrap();

    let missing = store.verify_backup_path(&backup_path).unwrap();
    assert!(!missing.ok);
    assert!(
        missing
            .errors
            .iter()
            .any(|error| error.contains("missing/unreadable"))
    );

    manifest.version = 999;
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let bad_version = store.verify_backup_path(&backup_path).unwrap();
    assert!(!bad_version.ok);
    assert!(
        bad_version
            .errors
            .iter()
            .any(|error| error.contains("unsupported backup manifest version"))
    );
}

#[test]
fn severe_strict_doctor_requires_backup_fresh_worker_and_clean_dead_letters() {
    let store = test_store("strict-doctor");
    let options = DoctorOptions {
        strict: true,
        max_worker_heartbeat_age_seconds: 300,
        max_dead_lettered_jobs: 0,
        max_backup_age_seconds: 7 * 24 * 60 * 60,
        service_plist_path: None,
    };

    let missing = store.doctor(options.clone()).unwrap();
    assert!(!missing.ok);
    assert!(
        missing
            .failures
            .iter()
            .any(|failure| failure.contains("no backup"))
    );
    assert!(
        missing
            .failures
            .iter()
            .any(|failure| failure.contains("no worker heartbeat"))
    );

    store
        .set_profile("doctor.test", "value", "normal", "test")
        .unwrap();
    store.create_backup().unwrap();
    store
        .record_worker_heartbeat("worker-test", 0, None)
        .unwrap();
    assert!(store.doctor(options.clone()).unwrap().ok);

    let stale = (Utc::now() - chrono::Duration::seconds(900)).to_rfc3339();
    store
        .conn
        .execute(
            "UPDATE worker_heartbeats SET last_seen_at = ?1 WHERE worker_id = 'worker-test'",
            params![stale],
        )
        .unwrap();
    let stale_report = store.doctor(options.clone()).unwrap();
    assert!(!stale_report.ok);
    assert!(
        stale_report
            .failures
            .iter()
            .any(|failure| failure.contains("heartbeat is stale"))
    );

    store
        .record_worker_heartbeat("worker-test", 0, None)
        .unwrap();
    store
        .insert_wiki_job_with_status(
            "ingest_file",
            "dead_lettered",
            json!({ "path": "/missing.md" }),
        )
        .unwrap();
    let dead = store.doctor(options).unwrap();
    assert!(!dead.ok);
    assert!(
        dead.failures
            .iter()
            .any(|failure| failure.contains("dead-lettered wiki jobs"))
    );
}

#[test]
fn severe_worker_recurrence_audit_requires_retained_multi_event_span() {
    // CLAIM: multi-day service recurrence proof must come from retained
    // heartbeat events over the requested wall-clock span, not a single
    // mutable latest-heartbeat row.
    // ORACLE: one heartbeat or a forged old started_at fails; inserting
    // source-retained events across the requested span passes only when
    // max-gap policy also passes.
    // SEVERITY: Severe because this is the anti-mirage gate for claiming
    // unattended launchd/systemd recurrence across days.
    let store = test_store("worker-recurrence-audit");
    store
        .record_worker_heartbeat("worker-test", 0, None)
        .unwrap();
    let one_event = store
        .audit_worker_recurrence(48 * 60 * 60, 25 * 60 * 60)
        .unwrap();
    assert!(!one_event.ok);
    assert!(
        one_event
            .failures
            .iter()
            .any(|failure| failure.contains("at least two retained heartbeat events"))
    );
    let forged_started_at = (Utc::now() - ChronoDuration::days(3)).to_rfc3339();
    store
        .conn
        .execute(
            "UPDATE worker_heartbeats SET started_at = ?1 WHERE worker_id = 'worker-test'",
            params![forged_started_at],
        )
        .unwrap();
    let forged = store
        .audit_worker_recurrence(48 * 60 * 60, 25 * 60 * 60)
        .unwrap();
    assert!(
        !forged.ok,
        "forged started_at must not prove recurrence without events: {forged:#?}"
    );

    let first = (Utc::now() - ChronoDuration::hours(49)).to_rfc3339();
    let middle = (Utc::now() - ChronoDuration::hours(24)).to_rfc3339();
    for (id, seen_at) in [
        ("historical-worker-event-1", first.as_str()),
        ("historical-worker-event-2", middle.as_str()),
    ] {
        store
            .conn
            .execute(
                r#"
                    INSERT INTO worker_heartbeat_events
                      (id, worker_id, seen_at, processed_jobs, last_error, created_at)
                    VALUES (?1, 'worker-test', ?2, 1, NULL, ?2)
                    "#,
                params![id, seen_at],
            )
            .unwrap();
    }
    let pass = store
        .audit_worker_recurrence(48 * 60 * 60, 26 * 60 * 60)
        .unwrap();
    assert!(pass.ok, "{pass:#?}");
    assert!(pass.observed_span_seconds >= 48 * 60 * 60);
    assert_eq!(pass.worker_id.as_deref(), Some("worker-test"));

    let gap_fail = store
        .audit_worker_recurrence(48 * 60 * 60, 60 * 60)
        .unwrap();
    assert!(!gap_fail.ok);
    assert!(
        gap_fail
            .failures
            .iter()
            .any(|failure| failure.contains("best contiguous"))
    );
}

#[test]
fn severe_worker_recurrence_audit_surfaces_current_worker_even_when_best_span_is_old() {
    // CLAIM: recurrence audit separates long-span proof from current liveness
    // so an old best heartbeat segment cannot hide a freshly running worker.
    // ORACLE: the best segment remains historical, but latest/current segment
    // fields identify the fresh worker and mark it fresh.
    // SEVERITY: Severe because after reboot or sleep, operators need to know
    // whether catch-up can run now even if multi-day recurrence proof is not
    // yet re-established.
    let store = test_store("worker-recurrence-audit-current-liveness");
    for (id, seen_at) in [
        (
            "historical-worker-event-1",
            (Utc::now() - ChronoDuration::hours(51)).to_rfc3339(),
        ),
        (
            "historical-worker-event-2",
            (Utc::now() - ChronoDuration::hours(50)).to_rfc3339(),
        ),
        (
            "historical-worker-event-3",
            (Utc::now() - ChronoDuration::hours(49)).to_rfc3339(),
        ),
    ] {
        store
            .conn
            .execute(
                r#"
                    INSERT INTO worker_heartbeat_events
                      (id, worker_id, seen_at, processed_jobs, last_error, created_at)
                    VALUES (?1, 'worker-old', ?2, 1, NULL, ?2)
                    "#,
                params![id, seen_at],
            )
            .unwrap();
    }
    store
        .record_worker_heartbeat("worker-current", 0, None)
        .unwrap();

    let audit = store.audit_worker_recurrence(2 * 60 * 60, 90 * 60).unwrap();

    assert!(audit.ok, "{audit:#?}");
    assert_eq!(audit.worker_id.as_deref(), Some("worker-old"));
    assert_eq!(audit.latest_worker_id.as_deref(), Some("worker-current"));
    assert!(audit.latest_is_fresh, "{audit:#?}");
    assert!(audit.latest_age_seconds.unwrap_or(i64::MAX) <= 90 * 60);
    assert_eq!(audit.current_segment_event_count, 1);
    assert_eq!(
        audit.current_segment_first_seen_at,
        audit.current_segment_last_seen_at
    );
}

#[test]
fn severe_strict_doctor_rejects_stale_backup_schema_drift_and_missing_dirs() {
    let store = test_store("strict-doctor-drift");
    let options = DoctorOptions {
        strict: true,
        max_worker_heartbeat_age_seconds: 300,
        max_dead_lettered_jobs: 0,
        max_backup_age_seconds: 60,
        service_plist_path: None,
    };
    store
        .set_profile("doctor.test", "value", "normal", "test")
        .unwrap();
    let backup_path = store.create_backup().unwrap();
    store
        .record_worker_heartbeat("worker-test", 0, None)
        .unwrap();
    assert!(store.doctor(options.clone()).unwrap().ok);

    let manifest_path = backup_path.join("manifest.json");
    let manifest_json = fs::read_to_string(&manifest_path).unwrap();
    let mut manifest: BackupManifest = serde_json::from_str(&manifest_json).unwrap();
    manifest.created_at = Utc::now() - chrono::Duration::seconds(3_600);
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let stale_backup = store.doctor(options.clone()).unwrap();
    assert!(!stale_backup.ok);
    assert!(
        stale_backup
            .failures
            .iter()
            .any(|failure| failure.contains("latest backup is stale"))
    );

    fs::write(manifest_path, manifest_json).unwrap();
    store
        .conn
        .execute(
            "UPDATE meta SET value = '999' WHERE key = 'schema_version'",
            [],
        )
        .unwrap();
    let schema_drift = store.doctor(options.clone()).unwrap();
    assert!(!schema_drift.ok);
    assert!(
        schema_drift
            .failures
            .iter()
            .any(|failure| failure.contains("schema version mismatch"))
    );

    store
        .conn
        .execute(
            "UPDATE meta SET value = ?1 WHERE key = 'schema_version'",
            params![SCHEMA_VERSION.to_string()],
        )
        .unwrap();
    fs::remove_dir_all(&store.paths.wiki_pages).unwrap();
    let missing_dir = store.doctor(options.clone()).unwrap();
    assert!(!missing_dir.ok);
    assert!(
        missing_dir
            .failures
            .iter()
            .any(|failure| failure.contains("required wiki pages directory"))
    );

    fs::create_dir_all(&store.paths.wiki_pages).unwrap();
    fs::remove_dir_all(&store.paths.mem0).unwrap();
    let missing_mem0 = store.doctor(options).unwrap();
    assert!(!missing_mem0.ok);
    assert!(
        missing_mem0
            .failures
            .iter()
            .any(|failure| failure.contains("required mem0 directory"))
    );
}

#[test]
fn severe_strict_doctor_requires_service_plist_when_configured() {
    let store = test_store("strict-doctor-service");
    store
        .set_profile("doctor.test", "value", "normal", "test")
        .unwrap();
    store.create_backup().unwrap();
    store
        .record_worker_heartbeat("worker-test", 0, None)
        .unwrap();
    let plist_path = store
        .paths()
        .home
        .join("LaunchAgents")
        .join("arcwell.plist");
    let options = DoctorOptions {
        strict: true,
        max_worker_heartbeat_age_seconds: 300,
        max_dead_lettered_jobs: 0,
        max_backup_age_seconds: 7 * 24 * 60 * 60,
        service_plist_path: Some(plist_path.clone()),
    };

    let missing = store.doctor(options.clone()).unwrap();
    assert!(!missing.ok);
    assert!(
        missing
            .failures
            .iter()
            .any(|failure| failure.contains("service plist is missing"))
    );

    fs::create_dir_all(&plist_path).unwrap();
    let directory = store.doctor(options.clone()).unwrap();
    assert!(!directory.ok);
    assert!(
        directory
            .failures
            .iter()
            .any(|failure| failure.contains("service plist path is not a file"))
    );

    fs::remove_dir_all(&plist_path).unwrap();
    fs::write(&plist_path, "<plist><dict /></plist>").unwrap();
    assert!(store.doctor(options).unwrap().ok);
}
