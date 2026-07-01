use super::*;
use chrono::{Duration as ChronoDuration, Utc};

fn fetch_http_text(addr: SocketAddr, path: &str) -> String {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut last_error = None;
    while std::time::Instant::now() < deadline {
        match std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(100)) {
            Ok(mut stream) => {
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                    .unwrap();
                let request =
                    format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
                std::io::Write::write_all(&mut stream, request.as_bytes()).unwrap();
                let mut body = String::new();
                std::io::Read::read_to_string(&mut stream, &mut body).unwrap();
                return body;
            }
            Err(error) => {
                last_error = Some(error);
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
    panic!("HTTP listener {addr} did not become ready: {last_error:?}");
}

#[test]
fn severe_x_oauth_callback_parser_verifies_state_path_and_decoding() {
    // CLAIM: loopback OAuth callback handling accepts only the expected
    // path/state and decodes the authorization code without exposing token
    // material.
    // PRECONDITIONS: browser returns a GET callback with code/state query
    // parameters.
    // POSTCONDITIONS: matching path/state yields the decoded code; wrong
    // state/path/provider errors fail closed.
    // ORACLE: parser result and error text.
    // SEVERITY: Severe because accepting the wrong callback would exchange
    // an attacker-controlled code or turn provider errors into fake success.
    let request =
        "GET /callback?code=abc%2Bdef%3D&state=expected-state HTTP/1.1\r\nhost: 127.0.0.1\r\n\r\n";
    let callback = parse_x_oauth_callback_request(request, "/callback", "expected-state").unwrap();
    assert_eq!(callback.code, "abc+def=");

    let wrong_state = parse_x_oauth_callback_request(request, "/callback", "different-state")
        .unwrap_err()
        .to_string();
    assert!(wrong_state.contains("state mismatch"), "{wrong_state}");

    let wrong_path = parse_x_oauth_callback_request(request, "/other", "expected-state")
        .unwrap_err()
        .to_string();
    assert!(wrong_path.contains("path mismatch"), "{wrong_path}");

    let provider_error = parse_x_oauth_callback_request(
        "GET /callback?error=access_denied&state=expected-state HTTP/1.1\r\n\r\n",
        "/callback",
        "expected-state",
    )
    .unwrap_err()
    .to_string();
    assert!(
        provider_error.contains("authorization failed"),
        "{provider_error}"
    );
    assert!(!provider_error.contains("abc+def"));
}

#[test]
fn severe_x_oauth_loopback_redirect_rejects_non_loopback_or_implicit_ports() {
    // CLAIM: browser-assisted OAuth only binds explicit loopback callback
    // addresses and never listens on broad/public interfaces.
    // PRECONDITIONS: redirect URI comes from config/env/CLI.
    // POSTCONDITIONS: loopback with fixed port is accepted; public hosts,
    // https URLs, and implicit port 80 redirects are rejected before bind.
    // ORACLE: parsed bind address/path or error text.
    // SEVERITY: Severe because OAuth callback capture must not expose a
    // public listener or silently bind the wrong redirect.
    let parsed = parse_loopback_redirect_uri("http://127.0.0.1:8765/callback").unwrap();
    assert_eq!(parsed.bind_addr, "127.0.0.1:8765");
    assert_eq!(parsed.path, "/callback");

    for uri in [
        "https://127.0.0.1:8765/callback",
        "http://example.com:8765/callback",
        "http://127.0.0.1/callback",
        "http://127.0.0.1:0/callback",
    ] {
        assert!(
            parse_loopback_redirect_uri(uri).is_err(),
            "{uri} should not be accepted as an OAuth loopback redirect"
        );
    }
}

#[test]
fn severe_x_oauth_timeout_context_preserves_recovery_evidence_without_pkce_verifier() {
    // CLAIM: browser OAuth timeout errors are actionable and do not leak the
    // PKCE verifier, state, or challenge material used for token exchange.
    // PRECONDITIONS: the browser open succeeded locally, but no loopback
    // callback arrived.
    // POSTCONDITIONS: the error preserves the authorization endpoint and
    // redirect URI needed for diagnosis while excluding the full URL query.
    // ORACLE: formatted timeout context.
    // SEVERITY: Severe because silent callback timeouts recreate the
    // credential-babysitting failure mode this path is meant to remove.
    let context = x_oauth_callback_timeout_context(
        "https://x.com/i/oauth2/authorize?client_id=client&state=state&code_challenge=challenge",
        "http://127.0.0.1:8765/callback",
    );
    assert!(context.contains("authorization_endpoint=https://x.com/i/oauth2/authorize"));
    assert!(context.contains("redirect_uri=http://127.0.0.1:8765/callback"));
    assert!(context.contains("Chrome may still be on the login page"));
    assert!(!context.contains("authorization_url="));
    assert!(!context.contains("client_id=client"));
    assert!(!context.contains("state=state"));
    assert!(!context.contains("code_challenge=challenge"));
    assert!(!context.contains("code_verifier"));
    assert!(!context.contains("secret"));
}

#[test]
fn severe_gmail_oauth_timeout_context_preserves_recovery_evidence_without_pkce_query() {
    // CLAIM: Gmail browser OAuth timeout errors are actionable without logging
    // the full authorization URL query.
    // PRECONDITIONS: the browser open succeeded locally, but no loopback
    // callback arrived.
    // POSTCONDITIONS: the error preserves the authorization endpoint and
    // redirect URI while excluding state and code_challenge material.
    // ORACLE: formatted timeout context.
    // SEVERITY: Severe because Gmail mailbox verification credentials are
    // daemon-owned recovery material and timeout logs should not carry OAuth
    // one-shot parameters.
    let context = gmail_oauth_callback_timeout_context(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id=client&state=state&code_challenge=challenge",
        "http://127.0.0.1:8766/callback",
    );
    assert!(
        context.contains("authorization_endpoint=https://accounts.google.com/o/oauth2/v2/auth")
    );
    assert!(context.contains("http://127.0.0.1:8766/callback"));
    assert!(!context.contains("client_id=client"));
    assert!(!context.contains("state=state"));
    assert!(!context.contains("code_challenge=challenge"));
    assert!(!context.contains("authorization_url="));
    assert!(!context.contains("secret"));
}

#[test]
fn claude_import_reads_canonical_memory_export() {
    let root = std::env::temp_dir().join(format!(
        "arcwell-cli-claude-import-test-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    ));
    let out = root.join("out");
    fs::create_dir_all(&out).unwrap();
    let path = out.join("canonical_memories.jsonl");
    fs::write(
        &path,
        serde_json::to_string(&json!({
            "memory_id": "mem_123",
            "memory": "User prefers reviewable imports.",
            "details": "The import should create candidates rather than apply memories.",
            "category": "preference",
            "subject": "memory import",
            "status": "current",
            "sensitivity": "normal",
            "importance": 9,
            "confidence": 0.91,
            "review_required": false,
            "evidence": [
                {
                    "source_uri": "claude://conversation/example",
                    "quote": "create candidates"
                }
            ]
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    let report = analyze_claude_export(&root, 10, None).unwrap();
    assert_eq!(report.source_kind, "canonical_memories");
    assert_eq!(report.candidates_seen, 1);
    assert_eq!(report.candidates_sampled, 1);
    let candidate = &report.candidates[0];
    assert_eq!(candidate.target, "memory");
    assert_eq!(candidate.kind, "claude_export.preference");
    assert_eq!(candidate.operation, "ADD");
    assert_eq!(candidate.user_id.as_deref(), Some("chris"));
    assert_eq!(candidate.source_ref, "claude_export:mem_123");
    assert_eq!(candidate.metadata["claude_memory_id"], "mem_123");
    assert_eq!(candidate.metadata["imported_from"], "claude_history_export");
    assert!(
        candidate
            .content
            .contains("User prefers reviewable imports.")
    );
    assert!(candidate.content.contains("rather than apply memories."));
}

#[test]
fn severe_claude_import_redacts_secrets_and_preserves_update_scope() {
    // CLAIM: Coalesced Claude import creates reviewable candidates without
    // leaking secret-like content or losing UPDATE memory/user scope.
    // ORACLE: candidate fields, redacted content/metadata, and total-vs-sampled counts.
    // SEVERITY: Severe because imported history is private, inspectable state.
    let root = std::env::temp_dir().join(format!(
        "arcwell-cli-claude-import-redaction-test-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    ));
    let out = root.join("out").join("mem0");
    fs::create_dir_all(&out).unwrap();
    let path = out.join("mem0_ingest.jsonl");
    let token = format!("sk-{}", "a".repeat(48));
    let refresh = format!("ghp_{}", "b".repeat(48));
    let row = json!({
        "memory_id": "mem_update_1",
        "memory": format!("Rotate the API key {token} before publishing."),
        "user_id": "row-user",
        "metadata": {
            "category": "preference",
            "sensitivity": "sensitive",
            "operation": "UPDATE",
            "access_token": token,
            "evidence": [
                {
                    "source_uri": "claude://conversation/private",
                    "quote": format!("Authorization: Bearer {refresh}")
                }
            ]
        }
    });
    let second = json!({
        "memory_id": "mem_add_2",
        "memory": "This second row should count but not be sampled.",
        "metadata": { "category": "fact" }
    });
    fs::write(
        &path,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&row).unwrap(),
            serde_json::to_string(&second).unwrap()
        ),
    )
    .unwrap();

    let report = analyze_claude_export(&root, 1, Some("configured-user")).unwrap();
    assert_eq!(report.source_kind, "canonical_memories");
    assert_eq!(report.candidates_seen, 2);
    assert_eq!(report.candidates_sampled, 1);
    let candidate = &report.candidates[0];
    assert_eq!(candidate.operation, "UPDATE");
    assert_eq!(candidate.memory_id.as_deref(), Some("mem_update_1"));
    assert_eq!(candidate.user_id.as_deref(), Some("configured-user"));
    assert_eq!(candidate.sensitivity, "sensitive");
    let metadata = serde_json::to_string(&candidate.metadata).unwrap();
    assert!(!candidate.content.contains(&token));
    assert!(!metadata.contains(&token));
    assert!(!metadata.contains(&refresh));
    assert!(candidate.content.contains("[REDACTED]"));
    assert_eq!(candidate.metadata["access_token"], "[REDACTED]");
}

#[test]
fn severe_claude_import_write_candidates_is_idempotent() {
    // CLAIM: write-candidates imports coalesced Claude rows into durable
    // pending candidates exactly once across repeated runs.
    // ORACLE: second write suppresses the duplicate and durable candidate count remains one.
    // SEVERITY: Severe because resume/retry must not flood the review queue.
    let paths = test_paths("claude-import-idempotent");
    let root = std::env::temp_dir().join(format!(
        "arcwell-cli-claude-import-idempotent-test-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    ));
    let out = root.join("out");
    fs::create_dir_all(&out).unwrap();
    let canonical_path = out.join("canonical_memories.jsonl");
    fs::write(
        &canonical_path,
        serde_json::to_string(&json!({
            "memory_id": "mem_idempotent",
            "memory": "Imports should be idempotent.",
            "category": "fact",
            "user_id": "row-user"
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();

    let run_import = || {
        import(
            Store::open(paths.clone()).unwrap(),
            ImportCommand {
                command: ImportSubcommand::Claude {
                    path: root.clone(),
                    dry_run: false,
                    limit: 10,
                    user_id: None,
                    write_candidates: true,
                },
            },
        )
    };
    run_import().unwrap();
    fs::write(
        &canonical_path,
        serde_json::to_string(&json!({
            "memory_id": "mem_idempotent",
            "memory": "Imports should be idempotent even if redaction changes content.",
            "category": "fact",
            "user_id": "row-user"
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();
    run_import().unwrap();

    let store = Store::open(paths).unwrap();
    let candidates = store.list_candidates("pending").unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].user_id.as_deref(), Some("row-user"));
    assert_eq!(candidates[0].metadata["claude_memory_id"], "mem_idempotent");
    let runs = store.list_import_runs(10).unwrap();
    assert_eq!(runs.len(), 2);
    assert!(runs.iter().all(|run| run.status == "completed"));
    assert!(runs.iter().any(|run| run.candidates_written == 1));
    assert!(runs.iter().any(|run| run.duplicates_suppressed == 1));
}

#[test]
fn print_json_treats_broken_pipe_as_success() {
    let mut writer = BrokenPipeWriter;

    write_json_pretty(&mut writer, &json!({ "ok": true })).unwrap();
}

#[test]
fn slash_command_files_have_cli_or_mcp_aliases() {
    let command_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/arcwell-codex/commands");
    let mut command_names = fs::read_dir(&command_dir)
        .unwrap()
        .map(|entry| {
            entry
                .unwrap()
                .path()
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect::<Vec<_>>();
    command_names.sort();
    assert_eq!(command_names.len(), 148);
    let missing = command_names
        .into_iter()
        .filter(|name| slash_alias_target(name).is_none() && !slash_alias_is_dynamic(name))
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "missing slash command aliases: {missing:?}"
    );
}

#[test]
fn severe_launch_agent_plist_escapes_paths_and_clamps_worker_args() {
    let plist = launch_agent_plist(
        std::path::Path::new("/tmp/arcwell & \"worker\""),
        std::path::Path::new("/tmp/home <bad>"),
        std::path::Path::new("/tmp/logs 'quoted'"),
        999,
        1,
        Some("127.0.0.1:65535".parse().unwrap()),
        Some(std::path::Path::new("/tmp/arcwell-token & \"secret\"")),
        999,
        999,
    );

    assert!(plist.contains("/tmp/arcwell &amp; &quot;worker&quot;"));
    assert!(plist.contains("/tmp/home &lt;bad&gt;"));
    assert!(plist.contains("/tmp/logs &apos;quoted&apos;/worker.out.log"));
    assert!(plist.contains("<string>100</string>"));
    assert!(plist.contains("<string>250</string>"));
    assert!(plist.contains("<string>--http-addr</string>"));
    assert!(plist.contains("<string>127.0.0.1:65535</string>"));
    assert!(plist.contains("<string>--http-auth-token-file</string>"));
    assert!(plist.contains("/tmp/arcwell-token &amp; &quot;secret&quot;"));
    assert!(plist.contains("<string>--http-max-uri-bytes</string>"));
    assert!(plist.contains("<string>1024</string>"));
    assert!(!plist.contains("<string>--http-auth-token</string>"));
    assert!(!plist.contains("<string>999</string>"));
    assert!(!plist.contains("<string>1</string>"));

    let dir = test_paths("service-plist-cockpit-url").home;
    fs::create_dir_all(&dir).unwrap();
    let plist_path = dir.join("worker.plist");
    fs::write(&plist_path, plist).unwrap();
    assert_eq!(
        service_plist_cockpit_url(&plist_path).as_deref(),
        Some("http://127.0.0.1:65535/ops/ui")
    );
}

#[test]
fn severe_service_http_token_file_is_private_reused_and_not_a_process_arg() {
    let paths = test_paths("service-http-token-file");
    let token_path = ensure_service_http_token_file(&paths).unwrap();
    let first = fs::read_to_string(&token_path).unwrap();
    assert!(first.trim().len() >= 16);

    let reused = ensure_service_http_token_file(&paths).unwrap();
    let second = fs::read_to_string(&reused).unwrap();
    assert_eq!(token_path, reused);
    assert_eq!(first, second);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&token_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    let plist = launch_agent_plist(
        std::path::Path::new("/tmp/arcwell"),
        &paths.home,
        &paths.home.join("logs"),
        10,
        5000,
        Some("127.0.0.1:65535".parse().unwrap()),
        Some(&token_path),
        8192,
        65536,
    );
    assert!(plist.contains("--http-auth-token-file"));
    assert!(plist.contains(&token_path.to_string_lossy().to_string()));
    assert!(!plist.contains(first.trim()));

    let store = Store::open(paths.clone()).unwrap();
    let added = store.ensure_local_ops_ui_policy_rules().unwrap();
    assert!(added.contains(&"allow-local-ops-ui-actions".to_string()));
    let added_again = store.ensure_local_ops_ui_policy_rules().unwrap();
    assert!(added_again.is_empty());
    let ops_explain = store
        .policy_explain(PolicyRequest {
            action: "ops.worker.run_once".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: None,
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some("arcwell-worker".to_string()),
            projected_usd: None,
            metadata: Value::Null,
            untrusted_excerpt: None,
        })
        .unwrap();
    assert!(ops_explain.allowed);
    assert_eq!(
        ops_explain
            .matched_rule
            .as_ref()
            .map(|rule| rule.id.as_str()),
        Some("allow-local-ops-ui-actions")
    );
    let enqueue_explain = store
        .policy_explain(PolicyRequest {
            action: "worker.enqueue".to_string(),
            package: Some("arcwell-x".to_string()),
            provider: Some("x".to_string()),
            source: Some("x_import_bookmarks".to_string()),
            channel: None,
            subject: None,
            target: Some("bookmarks".to_string()),
            projected_usd: None,
            metadata: Value::Null,
            untrusted_excerpt: None,
        })
        .unwrap();
    assert!(enqueue_explain.allowed);
    assert_eq!(
        enqueue_explain
            .matched_rule
            .as_ref()
            .map(|rule| rule.id.as_str()),
        Some("allow-local-ops-ui-x-bookmark-worker-enqueue")
    );
}

#[test]
fn severe_service_plist_contract_rejects_corrupt_metadata_and_missing_binary() {
    let dir = test_paths("service-plist-contract").home;
    let log_dir = dir.join("logs");
    fs::create_dir_all(&log_dir).unwrap();
    let missing_binary = dir.join("missing arcwell");
    let plist_path = dir.join("worker.plist");
    fs::write(
        &plist_path,
        launch_agent_plist(
            &missing_binary,
            &dir,
            &log_dir,
            10,
            5000,
            None,
            None,
            8192,
            65536,
        ),
    )
    .unwrap();

    let missing_binary_failures = service_plist_contract_failures(&plist_path);
    assert!(
        missing_binary_failures
            .iter()
            .any(|failure| failure.contains("service binary is missing")),
        "{missing_binary_failures:?}"
    );

    fs::write(
        &plist_path,
        r#"<plist version="1.0"><dict><key>Label</key><string>evil.worker</string></dict></plist>"#,
    )
    .unwrap();
    let corrupt_failures = service_plist_contract_failures(&plist_path);
    assert!(
        corrupt_failures
            .iter()
            .any(|failure| failure.contains("label mismatch")),
        "{corrupt_failures:?}"
    );
    assert!(
        corrupt_failures
            .iter()
            .any(|failure| failure.contains("missing ProgramArguments")),
        "{corrupt_failures:?}"
    );
}

#[test]
fn severe_service_plist_contract_accepts_generated_worker_plist_with_hostile_paths() {
    let dir = test_paths("service-plist-contract-ok").home;
    let binary = dir.join("arcwell & worker");
    let home = dir.join("home <bad>");
    let log_dir = home.join("logs 'quoted'");
    fs::create_dir_all(&log_dir).unwrap();
    fs::write(&binary, "test").unwrap();
    let plist_path = dir.join("worker.plist");
    fs::write(
        &plist_path,
        launch_agent_plist(&binary, &home, &log_dir, 10, 5000, None, None, 8192, 65536),
    )
    .unwrap();

    let failures = service_plist_contract_failures(&plist_path);
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn severe_compact_recurrence_audit_keeps_readiness_without_large_event_payloads() {
    // CLAIM: `service recurrence-audit --compact` keeps the evidence operators
    // need after sleep/reboot without dumping every worker id or sample event.
    // PRECONDITIONS: multi-day recurrence proof is not yet established, but a
    // current worker heartbeat is fresh.
    // POSTCONDITIONS: compact output says current worker is fresh while span
    // proof is unproven, includes failures, and omits bulky event payloads.
    // ORACLE: compact JSON helper output.
    // SEVERITY: Severe because noisy recurrence proof output makes the
    // briefing catch-up question hard to answer from live ops.
    let audit = arcwell_core::WorkerRecurrenceAudit {
        ok: false,
        worker_id: Some("worker-old".to_string()),
        latest_worker_id: Some("worker-current".to_string()),
        worker_ids: vec![
            "worker-old".to_string(),
            "worker-current".to_string(),
            "worker-another".to_string(),
        ],
        event_count: 10,
        retained_event_count: 3000,
        first_seen_at: Some("2026-06-29T02:57:55Z".to_string()),
        last_seen_at: Some("2026-06-29T18:48:12Z".to_string()),
        latest_seen_at: Some("2026-07-01T02:14:42Z".to_string()),
        latest_age_seconds: Some(54),
        latest_is_fresh: true,
        observed_span_seconds: 57_016,
        max_gap_seconds: Some(647),
        current_segment_event_count: 61,
        current_segment_first_seen_at: Some("2026-07-01T01:10:17Z".to_string()),
        current_segment_last_seen_at: Some("2026-07-01T02:14:42Z".to_string()),
        current_segment_span_seconds: 3_865,
        min_required_span_seconds: 172_800,
        max_allowed_gap_seconds: 900,
        failures: vec!["best contiguous worker heartbeat event span is too short".to_string()],
        sample_events: vec![arcwell_core::WorkerHeartbeatEvent {
            id: "event-id".to_string(),
            worker_id: "worker-old".to_string(),
            seen_at: "2026-06-29T02:57:55Z".to_string(),
            processed_jobs: 10,
            last_error: None,
        }],
    };

    let compact = compact_worker_recurrence_audit_json(&audit);

    assert_eq!(
        compact["proof_status"],
        "current_worker_fresh_span_unproven"
    );
    assert_eq!(compact["latest_worker_id"], "worker-current");
    assert_eq!(compact["latest_is_fresh"], true);
    assert_eq!(compact["failure_count"], 1);
    assert_eq!(compact["retained_event_count"], 3000);
    assert!(compact.get("worker_ids").is_none(), "{compact:#?}");
    assert!(compact.get("sample_events").is_none(), "{compact:#?}");
    let serialized = serde_json::to_string(&compact).unwrap();
    assert!(!serialized.contains("worker-another"), "{serialized}");
    assert!(!serialized.contains("event-id"), "{serialized}");
}

#[test]
fn severe_compact_service_status_reports_running_fresh_without_raw_launchctl_dump() {
    // CLAIM: `service status --compact` gives the operator readiness answer
    // for resident catch-up without requiring raw launchctl parsing.
    // PRECONDITIONS: the service plist exists, launchctl reports running, and
    // the latest heartbeat is within the freshness threshold.
    // POSTCONDITIONS: compact output reports running_fresh and omits raw
    // launchctl stdout/heartbeat event dumps.
    // ORACLE: compact JSON helper output.
    // SEVERITY: Severe because laptop wake-up recovery depends on knowing
    // whether the resident worker is actually alive now.
    let heartbeat = arcwell_core::WorkerHeartbeat {
        worker_id: "arcwell-worker-current".to_string(),
        started_at: (Utc::now() - ChronoDuration::hours(1)).to_rfc3339(),
        last_seen_at: (Utc::now() - ChronoDuration::seconds(30)).to_rfc3339(),
        processed_jobs: 12,
        last_error: None,
    };
    let launchctl = json!({
        "attempted": true,
        "ok": true,
        "status": 0,
        "stdout": "gui/501/com.arcwell.worker = {\n\tstate = running\n\tpid = 123\n\tjob state = running\n}",
        "stderr": ""
    });

    let compact = compact_service_status_json(
        "com.arcwell.worker",
        true,
        Path::new("/Users/example/Library/LaunchAgents/com.arcwell.worker.plist"),
        Some(&heartbeat),
        &launchctl,
        300,
    );

    assert_eq!(compact["ok"], true, "{compact:#?}");
    assert_eq!(compact["status"], "running_fresh", "{compact:#?}");
    assert_eq!(compact["launchctl_running"], true, "{compact:#?}");
    assert_eq!(compact["heartbeat_fresh"], true, "{compact:#?}");
    assert_eq!(compact["worker_id"], "arcwell-worker-current");
    assert!(compact["heartbeat_age_seconds"].as_i64().unwrap() <= 300);
    assert!(compact.get("heartbeat_events").is_none(), "{compact:#?}");
    assert!(compact.get("stdout").is_none(), "{compact:#?}");
    assert!(compact.get("launchctl").is_none(), "{compact:#?}");
    let serialized = serde_json::to_string(&compact).unwrap();
    assert!(!serialized.contains("pid = 123"), "{serialized}");
}

#[test]
fn severe_worker_run_max_ticks_exits_after_repeated_wall_clock_ticks() {
    // CLAIM: worker run can be bounded for proof harnesses without changing
    // service mode, and the bounded loop performs repeated resident ticks
    // rather than exiting after a single run-once drain.
    // ORACLE: two ticks with a sub-clamp sleep record a heartbeat and take
    // at least the clamped 250ms sleep interval twice.
    // SEVERITY: Severe because wall-clock recurrence proof is hollow if it
    // only calls run-once or if the bounded loop exits immediately.
    let paths = test_paths("worker-run-max-ticks");
    let store = Store::open(paths.clone()).unwrap();
    let started = std::time::Instant::now();
    worker(
        store,
        WorkerCommand {
            command: WorkerSubcommand::Run {
                max_jobs_per_tick: 1,
                idle_sleep_ms: 1,
                max_ticks: Some(2),
                http_addr: None,
                http_auth_token: None,
                http_auth_token_file: None,
                http_max_uri_bytes: 8192,
                http_max_body_bytes: 65536,
            },
        },
    )
    .unwrap();
    assert!(
        started.elapsed() >= std::time::Duration::from_millis(450),
        "bounded worker loop exited too quickly: {:?}",
        started.elapsed()
    );
    let store = Store::open(paths).unwrap();
    let heartbeat = store.latest_worker_heartbeat().unwrap().unwrap();
    assert!(heartbeat.worker_id.starts_with("arcwell-worker-"));
    assert_eq!(heartbeat.processed_jobs, 0);
}

#[test]
fn severe_worker_run_can_host_ops_ui_without_separate_service() {
    // CLAIM: the resident worker process can host the same Rust /ops/ui cockpit
    // router, so the dashboard does not require a second daemon or frontend
    // dependency chain.
    // ORACLE: bounded worker run starts an HTTP listener and serves /ops/ui
    // from that worker process.
    // SEVERITY: Severe because a cockpit that only works through a parallel
    // process would violate Arcwell's source-owned service boundary.
    let paths = test_paths("worker-run-http-ops-ui");
    let store = Store::open(paths.clone()).unwrap();
    store
        .capture_memory_from_text(
            "Worker-hosted cockpit proof.",
            "worker-http-test",
            Some("chris"),
            false,
            false,
        )
        .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let handle = std::thread::spawn(move || {
        worker(
            store,
            WorkerCommand {
                command: WorkerSubcommand::Run {
                    max_jobs_per_tick: 1,
                    idle_sleep_ms: 250,
                    max_ticks: Some(8),
                    http_addr: Some(addr),
                    http_auth_token: None,
                    http_auth_token_file: None,
                    http_max_uri_bytes: 8192,
                    http_max_body_bytes: 65536,
                },
            },
        )
    });

    let body = fetch_http_text(addr, "/ops/ui");
    assert!(body.contains("HTTP/1.1 200 OK"), "{body}");
    assert!(body.contains("Arcwell Ops Cockpit"), "{body}");
    assert!(body.contains("Agent Visibility"), "{body}");
    assert!(body.contains(&format!("http://{addr}/ops/ui")), "{body}");
    handle.join().unwrap().unwrap();
}
