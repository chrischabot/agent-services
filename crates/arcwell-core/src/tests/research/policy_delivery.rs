use super::*;

#[test]
fn severe_memory_decision_ledger_records_add_and_suppressed_duplicate() {
    let store = test_store("memory-ledger");
    let first = store
        .extract_memory_candidates_from_text_for_user(
            "My cat is called Ophelia.",
            "test:conversation",
            Some("chris"),
        )
        .unwrap();
    assert_eq!(first.candidates_created, 1);

    let second = store
        .extract_memory_candidates_from_text_for_user(
            "My cat is called Ophelia.",
            "test:conversation",
            Some("chris"),
        )
        .unwrap();
    assert_eq!(second.duplicates_suppressed, 1);

    let decisions = store.list_memory_decisions(10).unwrap();
    assert_eq!(decisions.len(), 2);
    assert!(decisions.iter().any(|entry| entry.candidate_id.is_some()));
    assert!(decisions.iter().any(|entry| {
        entry.candidate_id.is_none()
            && entry
                .metadata
                .get("duplicate_suppressed")
                .and_then(Value::as_bool)
                == Some(true)
    }));
    assert!(decisions.iter().all(|entry| {
        (0.0..=1.0).contains(&entry.confidence) && entry.user_id.as_deref() == Some("chris")
    }));
}

#[test]
fn severe_memory_forget_writes_tombstone_without_raw_user_id() {
    let store = test_store("memory-tombstone");
    store
        .mem0_add_memory(
            "My cat is called Ophelia.",
            Some("chris"),
            "test",
            "normal",
            false,
        )
        .unwrap();
    let report = store.mem0_forget_user(Some("chris")).unwrap();
    assert!(!report.tombstone_id.is_empty());

    let tombstones = store.list_memory_forget_tombstones(10).unwrap();
    assert_eq!(tombstones.len(), 1);
    assert_eq!(tombstones[0].id, report.tombstone_id);
    assert_ne!(tombstones[0].user_id_hash, "chris");
    assert_eq!(tombstones[0].user_id_hash, sha256(b"chris"));
    assert!(tombstones[0].policy.contains("active_store_purged"));
    assert!(tombstones[0].policy.contains("historical_backups_retained"));
    assert!(
        tombstones[0]
            .policy
            .contains("backups_not_rewritten_by_forget")
    );
}

#[test]
fn severe_source_cost_policy_accumulates_source_spend() {
    let store = test_store("source-cost");
    store
        .add_cost_for_source(
            "arcwell-deep-research",
            "job-1",
            "brave",
            "web_search",
            Some("web_search"),
            0.04,
            0.0,
        )
        .unwrap();
    store
        .set_cost_policy("source", "web_search", Some(0.05), false, None)
        .unwrap();

    let blocked = store
        .cost_decision("arcwell-deep-research", "brave", Some("web_search"), 0.02)
        .unwrap();
    assert!(!blocked.allowed);
    assert_eq!(blocked.spent_usd, 0.04);
    assert_eq!(
        blocked
            .matched_policy
            .as_ref()
            .map(|policy| policy.scope.as_str()),
        Some("source")
    );
}

#[test]
fn severe_telegram_retry_reuses_existing_message_and_records_attempts() {
    let store = test_store("telegram-retry");
    store
        .authorize_channel_subject("telegram", "telegram:chat:123", false, false, true)
        .unwrap();
    let failing_base = mock_status_server(
        "429 Too Many Requests",
        "retry-after: 1\r\n",
        r#"{"ok":false}"#,
        "application/json",
    );
    let first = store
        .send_telegram_message("token", "123", "Retry me", Some(&failing_base))
        .unwrap();
    assert!(!first.ok);
    assert_eq!(first.message.status, "failed");
    store
        .conn
        .execute(
            "UPDATE channel_delivery_attempts SET retry_at = ?1 WHERE message_id = ?2",
            params!["2000-01-01T00:00:00.000000000+00:00", first.message.id],
        )
        .unwrap();

    let ok_base = mock_status_server("200 OK", "", r#"{"ok":true}"#, "application/json");
    let retry = store
        .retry_due_telegram_deliveries("token", Some(&ok_base), 10)
        .unwrap();
    assert_eq!(retry.attempted, 1);
    assert_eq!(retry.sent, 1);
    assert_eq!(store.list_channel_messages().unwrap().len(), 1);
    let attempts = store
        .list_channel_delivery_attempts(Some(&first.message.id))
        .unwrap();
    assert_eq!(attempts.len(), 2);
    assert!(attempts.iter().any(|attempt| attempt.ok));
    assert_eq!(
        store
            .get_channel_message(&first.message.id)
            .unwrap()
            .unwrap()
            .status,
        "sent"
    );
}

#[test]
fn severe_worker_retries_due_telegram_delivery_from_local_config() {
    // CLAIM: The resident worker path automatically retries due Telegram
    // deliveries using local config, without creating a duplicate channel message.
    // ORACLE: run_worker_once reports a Telegram retry, message count stays one,
    // the existing message becomes sent, and delivery attempts increment to two.
    // SEVERITY: Severe because unattended retries can otherwise duplicate sends
    // or silently skip due delivery work.
    let store = test_store("telegram-worker-retry");
    store
        .authorize_channel_subject("telegram", "telegram:chat:123", false, false, true)
        .unwrap();
    let failing_base = mock_status_server(
        "429 Too Many Requests",
        "retry-after: 1\r\n",
        r#"{"ok":false}"#,
        "application/json",
    );
    let first = store
        .send_telegram_message("token", "123", "Retry from worker", Some(&failing_base))
        .unwrap();
    assert!(!first.ok);
    store
        .conn
        .execute(
            "UPDATE channel_delivery_attempts SET retry_at = ?1 WHERE message_id = ?2",
            params!["2000-01-01T00:00:00.000000000+00:00", first.message.id],
        )
        .unwrap();
    let ok_base = mock_status_server("200 OK", "", r#"{"ok":true}"#, "application/json");
    store
        .set_secret_value("TELEGRAM_BOT_TOKEN", "token", "telegram")
        .unwrap();
    store
        .set_secret_value("TELEGRAM_API_BASE", &ok_base, "telegram")
        .unwrap();

    let report = store.run_worker_once(1).unwrap();
    assert_eq!(report.processed, 0);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let retry = report.telegram_retry.expect("worker should retry Telegram");
    assert_eq!(retry.attempted, 1);
    assert_eq!(retry.sent, 1);
    assert_eq!(retry.failed, 0);
    assert_eq!(store.list_channel_messages().unwrap().len(), 1);
    let attempts = store
        .list_channel_delivery_attempts(Some(&first.message.id))
        .unwrap();
    assert_eq!(attempts.len(), 2);
    assert!(attempts.iter().any(|attempt| attempt.ok));
    assert_eq!(
        store
            .get_channel_message(&first.message.id)
            .unwrap()
            .unwrap()
            .status,
        "sent"
    );
}

#[test]
fn severe_worker_retries_due_email_delivery_from_local_config() {
    // CLAIM: The resident worker retries due failed email deliveries from
    // local Cloudflare Email config without creating duplicate channel
    // messages or leaking configured tokens.
    // ORACLE: one failed email message gains a second successful attempt,
    // the original channel message becomes sent, message count stays one,
    // and serialized worker output omits the API token.
    // SEVERITY: Severe because cross-channel delivery retry is hollow if
    // email has a send path but no unattended retry producer.
    let store = test_store("email-worker-retry");
    store
        .authorize_channel_subject("email", "email:friend@example.com", false, false, true)
        .unwrap();
    let failing_base = mock_status_server(
        "503 Service Unavailable",
        "",
        r#"{"success":false,"errors":[{"message":"temporarily unavailable"}]}"#,
        "application/json",
    );
    let first = store
        .send_cloudflare_email(
            "abcd1234",
            "EMAIL_TOKEN_SHOULD_NOT_LEAK",
            "agent@example.com",
            "friend@example.com",
            "Retry this",
            "Retry from worker",
            None,
            None,
            Some(&failing_base),
        )
        .unwrap();
    assert!(!first.ok);
    assert_eq!(first.message.status, "failed");
    store
        .conn
        .execute(
            "UPDATE channel_delivery_attempts SET retry_at = ?1 WHERE message_id = ?2",
            params!["2000-01-01T00:00:00.000000000+00:00", first.message.id],
        )
        .unwrap();
    let ok_base = mock_status_server(
        "200 OK",
        "",
        r#"{"success":true,"result":{"id":"email_retry_ok"}}"#,
        "application/json",
    );
    store
        .set_secret_value("CLOUDFLARE_ACCOUNT_ID", "abcd1234", "email")
        .unwrap();
    store
        .set_secret_value(
            "CLOUDFLARE_EMAIL_API_TOKEN",
            "EMAIL_TOKEN_SHOULD_NOT_LEAK",
            "email",
        )
        .unwrap();
    store
        .set_secret_value("ARCWELL_AGENT_EMAIL_FROM", "agent@example.com", "email")
        .unwrap();
    store
        .set_secret_value("CLOUDFLARE_EMAIL_API_BASE", &ok_base, "email")
        .unwrap();

    let report = store.run_worker_once(1).unwrap();
    assert_eq!(report.processed, 0);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let retry = report
        .email_retry
        .as_ref()
        .expect("worker should retry email");
    assert_eq!(retry.attempted, 1);
    assert_eq!(retry.sent, 1);
    assert_eq!(retry.failed, 0);
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("EMAIL_TOKEN_SHOULD_NOT_LEAK")
    );
    assert_eq!(store.list_channel_messages().unwrap().len(), 1);
    let attempts = store
        .list_channel_delivery_attempts(Some(&first.message.id))
        .unwrap();
    assert_eq!(attempts.len(), 2);
    assert!(attempts.iter().any(|attempt| attempt.ok));
    assert_eq!(
        store
            .get_channel_message(&first.message.id)
            .unwrap()
            .unwrap()
            .status,
        "sent"
    );
}

#[test]
fn severe_remote_edge_drain_acks_only_after_local_persist() {
    let store = test_store("remote-edge-drain");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut lease_stream, _) = listener.accept().unwrap();
        let mut lease_buffer = [0_u8; 8192];
        let _ = lease_stream.read(&mut lease_buffer);
        let lease_body = r#"{"event":{"source":"telegram","idempotencyKey":"remote:1","payload":{"text":"hello","chatId":"123"},"status":"leased"}}"#;
        let lease_response = format!(
            "HTTP/1.1 200 OK\r\nconnection: close\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            lease_body.len(),
            lease_body
        );
        lease_stream.write_all(lease_response.as_bytes()).unwrap();

        let (mut ack_stream, _) = listener.accept().unwrap();
        let mut ack_buffer = [0_u8; 8192];
        let read = ack_stream.read(&mut ack_buffer).unwrap();
        let ack_request = String::from_utf8_lossy(&ack_buffer[..read]);
        assert!(ack_request.contains("/drain/ack"));
        assert!(ack_request.contains("remote:1"));
        let ack_body = r#"{"ok":true}"#;
        let ack_response = format!(
            "HTTP/1.1 200 OK\r\nconnection: close\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            ack_body.len(),
            ack_body
        );
        ack_stream.write_all(ack_response.as_bytes()).unwrap();
    });

    let report = store
        .drain_remote_edge_inbox(&format!("http://{addr}"), "secret", 1)
        .unwrap();
    assert_eq!(report.imported, 1);
    assert_eq!(report.acked, 1);
    let local = store.list_edge_events().unwrap();
    assert_eq!(local.len(), 1);
    assert_eq!(local[0].idempotency_key, "remote:1");
    handle.join().unwrap();
}

#[test]
fn severe_remote_edge_drain_nacks_when_local_persist_fails() {
    let store = test_store("remote-edge-drain-persist-fail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut lease_stream, _) = listener.accept().unwrap();
        let mut lease_buffer = [0_u8; 8192];
        let _ = lease_stream.read(&mut lease_buffer);
        let lease_body = r#"{"event":{"source":"   ","idempotencyKey":"remote:invalid-source","payload":{"text":"hello"},"status":"leased"}}"#;
        let lease_response = format!(
            "HTTP/1.1 200 OK\r\nconnection: close\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            lease_body.len(),
            lease_body
        );
        lease_stream.write_all(lease_response.as_bytes()).unwrap();

        let (mut nack_stream, _) = listener.accept().unwrap();
        let mut nack_buffer = [0_u8; 8192];
        let read = nack_stream.read(&mut nack_buffer).unwrap();
        let nack_request = String::from_utf8_lossy(&nack_buffer[..read]);
        assert!(nack_request.contains("/drain/nack"));
        assert!(!nack_request.contains("/drain/ack"));
        assert!(nack_request.contains("remote:invalid-source"));
        let nack_body = r#"{"ok":true}"#;
        let nack_response = format!(
            "HTTP/1.1 200 OK\r\nconnection: close\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            nack_body.len(),
            nack_body
        );
        nack_stream.write_all(nack_response.as_bytes()).unwrap();
    });

    let report = store
        .drain_remote_edge_inbox(&format!("http://{addr}"), "secret", 1)
        .unwrap();
    assert_eq!(report.imported, 0);
    assert_eq!(report.acked, 0);
    assert_eq!(report.nacked, 1);
    assert!(store.list_edge_events().unwrap().is_empty());
    handle.join().unwrap();
}
