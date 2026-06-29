use super::*;

#[test]
fn severe_radar_exact_url_dedupe_preserves_evidence_but_selects_one_primary() {
    // CLAIM: exact dedupe cannot delete or hide evidence rows; it only adds an
    // auditable group and suppresses duplicate score selection.
    // ORACLE: two source cards with the same canonical URL remain as radar
    // items, exactly one is selected, the other is marked duplicate_url, and
    // the dedupe group references both item ids.
    // SEVERITY: Severe because digest dedupe is a classic place for fake
    // "clean output" to destroy source inspectability.
    let store = test_store("radar-exact-url-dedupe");
    for input in [
        SourceCardInput {
            title: "Overlap agent launch from RSS".to_string(),
            url: "https://example.com/overlap-agent-launch".to_string(),
            source_type: "rss".to_string(),
            provider: "rss".to_string(),
            summary: "RSS reports an overlap agent launch for MCP workflows.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-23T00:00:00Z".to_string()),
            metadata: json!({ "source_kind": "rss", "source_detail": "https://example.com/feed.xml" }),
        },
        SourceCardInput {
            title: "Overlap agent launch from GitHub".to_string(),
            url: "https://example.com/overlap-agent-launch".to_string(),
            source_type: "github_release".to_string(),
            provider: "github".to_string(),
            summary: "GitHub release mirror for the same overlap agent launch.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-23T00:00:00Z".to_string()),
            metadata: json!({ "source_kind": "github_release", "source_detail": "example/overlap" }),
        },
    ] {
        store.add_source_card(input).unwrap();
    }
    let profile = store
        .create_radar_profile(RadarProfileInput {
            name: "exact-dedupe-radar".to_string(),
            description: "Exact URL dedupe radar".to_string(),
            window_hours: 24,
            min_score: 1.0,
            max_items: Some(10),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "overlap" }]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({}),
        })
        .unwrap();

    let report = store.run_radar_profile(&profile.id, None).unwrap();
    assert_eq!(report.items_inserted, 2);
    assert_eq!(report.scores_inserted, 2);
    assert_eq!(report.selected_items, 1);
    assert_eq!(report.run.filtered_count, 1);

    let stage = store.read_radar_stage(&report.run.id).unwrap();
    assert_eq!(stage.items.len(), 2);
    assert_eq!(stage.scores.len(), 2);
    assert_eq!(stage.dedup_groups.len(), 1);
    let group = &stage.dedup_groups[0];
    assert_eq!(group.dedup_kind, "canonical_url");
    assert_eq!(group.confidence, 1.0);
    assert_eq!(group.member_item_ids.len(), 2);
    let item_ids = stage
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    assert!(group.member_item_ids.iter().all(|id| item_ids.contains(id)));
    assert!(group.member_item_ids.contains(&group.primary_item_id));

    let statuses = stage
        .scores
        .iter()
        .map(|score| score.status.as_str())
        .collect::<BTreeSet<_>>();
    assert!(statuses.contains("selected"));
    assert!(statuses.contains("duplicate_url"));
    let duplicate = stage
        .scores
        .iter()
        .find(|score| score.status == "duplicate_url")
        .unwrap();
    assert!(duplicate.tags.contains(&"duplicate".to_string()));
    assert!(
        duplicate
            .reason
            .contains("duplicate suppressed by dedupe group")
    );

    let audit = store.audit_radar_run(&report.run.id).unwrap();
    assert!(audit.ok, "{audit:?}");
    assert_eq!(audit.dedup_group_count, 1);
}

#[test]
fn severe_radar_summary_is_generated_report_not_delivery_or_source_evidence() {
    // CLAIM: radar summary writes a local report artifact over selected
    // scored items, with source-card provenance and explicit boundaries.
    // ORACLE: summary row cites the selected item/source card, run summary
    // count advances, delivery count stays zero, and empty selection fails.
    // SEVERITY: Severe because "a summary exists" is easy to mistake for
    // source ingestion, model synthesis, or outbound delivery.
    let store = test_store("radar-summary-boundary");
    let card = store
        .add_source_card(SourceCardInput {
            title: "Radar summary agent launch".to_string(),
            url: "https://example.com/radar-summary-agent-launch".to_string(),
            source_type: "web".to_string(),
            provider: "fixture".to_string(),
            summary: "Agent launch source card supports radar summary proof.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-23T00:00:00Z".to_string()),
            metadata: json!({ "source_kind": "manual" }),
        })
        .unwrap();
    let profile = store
        .create_radar_profile(RadarProfileInput {
            name: "summary-radar".to_string(),
            description: "Summary radar".to_string(),
            window_hours: 24,
            min_score: 1.0,
            max_items: Some(10),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "summary agent" }]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({}),
        })
        .unwrap();
    let report = store.run_radar_profile(&profile.id, None).unwrap();
    let summary = store
        .summarize_radar_run(&report.run.id, "en", "markdown")
        .unwrap();
    assert_eq!(summary.audit_status, "audit_ok");
    assert_eq!(summary.item_ids.len(), 1);
    assert_eq!(summary.source_card_ids, vec![card.id.clone()]);
    assert!(summary.body_markdown.contains("GENERATED_RADAR_SUMMARY"));
    assert!(summary.body_markdown.contains("not source evidence"));
    assert!(summary.body_markdown.contains("did not deliver"));
    assert!(summary.body_markdown.contains(&card.id));
    assert_eq!(
        summary
            .metadata
            .get("not_delivery")
            .and_then(Value::as_bool),
        Some(true)
    );
    let run = store.read_radar_run(&report.run.id).unwrap().unwrap();
    assert_eq!(run.stage, "summarized");
    assert_eq!(run.summary_count, 1);
    assert_eq!(run.delivery_count, 0);
    assert_eq!(
        store
            .read_radar_summary(&report.run.id, "en", "markdown")
            .unwrap()
            .unwrap()
            .id,
        summary.id
    );

    let empty_profile = store
        .create_radar_profile(RadarProfileInput {
            name: "summary-empty-radar".to_string(),
            description: "Summary empty radar".to_string(),
            window_hours: 24,
            min_score: 10.0,
            max_items: Some(10),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "summary agent" }]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({}),
        })
        .unwrap();
    let empty_report = store.run_radar_profile(&empty_profile.id, None).unwrap();
    let error = store
        .summarize_radar_run(&empty_report.run.id, "en", "markdown")
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("requires at least one selected score"),
        "{error}"
    );
}

#[test]
fn severe_radar_manual_delivery_links_summary_to_authorized_channel_attempt_idempotently() {
    // CLAIM: manual radar delivery is a real bridge from an audit-ok radar
    // summary to an authorized channel send, with durable radar and channel
    // delivery rows.
    // ORACLE: local HTTP provider receives the send, radar_deliveries links
    // to channel_delivery_attempts, run delivery_count advances once, and
    // replaying the same idempotency key does not send or insert again.
    // SEVERITY: Severe because a rendered digest can look delivered unless
    // durable attempt state proves the provider path was actually reached.
    let store = test_store("radar-manual-delivery");
    store
        .authorize_channel_subject("telegram", "telegram:chat:123", false, false, true)
        .unwrap();
    let card = store
        .add_source_card(SourceCardInput {
            title: "Radar delivery agent launch".to_string(),
            url: "https://example.com/radar-delivery-agent-launch".to_string(),
            source_type: "web".to_string(),
            provider: "fixture".to_string(),
            summary: "Agent launch source card supports radar delivery proof.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({ "source_kind": "manual" }),
        })
        .unwrap();
    let profile = store
        .create_radar_profile(RadarProfileInput {
            name: "delivery-radar".to_string(),
            description: "Delivery radar".to_string(),
            window_hours: 24,
            min_score: 1.0,
            max_items: Some(10),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "delivery agent" }]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({}),
        })
        .unwrap();
    let report = store.run_radar_profile(&profile.id, None).unwrap();
    let summary = store
        .summarize_radar_run(&report.run.id, "en", "markdown")
        .unwrap();
    let api = mock_status_server("200 OK", "", r#"{"ok":true}"#, "application/json");

    let delivered = store
        .deliver_radar_summary(RadarDeliveryInput {
            run_id: report.run.id.clone(),
            language: "en".to_string(),
            format: "markdown".to_string(),
            channel: "telegram".to_string(),
            recipient_ref: "123".to_string(),
            idempotency_key: Some("radar-delivery-test-key".to_string()),
            telegram_bot_token: Some("TOKEN".to_string()),
            email_account_id: None,
            email_api_token: None,
            email_from: None,
            api_base: Some(api),
        })
        .unwrap();

    assert_eq!(delivered.summary.id, summary.id);
    assert_eq!(delivered.delivery.status, "sent");
    assert_eq!(delivered.delivery.channel, "telegram");
    assert_eq!(delivered.delivery.recipient_ref, "telegram:chat:123");
    assert_eq!(delivered.delivery.error, None);
    assert!(
        delivered.delivery.cost_decision_id.is_some(),
        "{:?}",
        delivered.delivery
    );
    assert!(!delivered.idempotent_replay);
    let attempt = delivered
        .channel_delivery_attempt
        .as_ref()
        .expect("channel attempt");
    assert_eq!(
        delivered.delivery.delivery_attempt_id.as_deref(),
        Some(attempt.id.as_str())
    );
    assert!(attempt.ok);
    let message = delivered.channel_message.as_ref().expect("channel message");
    assert_eq!(message.status, "sent");
    assert_eq!(message.channel, "telegram");
    assert!(message.body.contains("GENERATED_RADAR_SUMMARY"));
    assert!(message.body.contains(&card.id));

    let run = store.read_radar_run(&report.run.id).unwrap().unwrap();
    assert_eq!(run.delivery_count, 1);
    assert_eq!(run.stage, "delivered");
    assert_eq!(
        store
            .list_radar_deliveries(Some(&report.run.id))
            .unwrap()
            .len(),
        1
    );

    let replayed = store
        .deliver_radar_summary(RadarDeliveryInput {
            run_id: report.run.id.clone(),
            language: "en".to_string(),
            format: "markdown".to_string(),
            channel: "telegram".to_string(),
            recipient_ref: "telegram:chat:123".to_string(),
            idempotency_key: Some("radar-delivery-test-key".to_string()),
            telegram_bot_token: Some("TOKEN".to_string()),
            email_account_id: None,
            email_api_token: None,
            email_from: None,
            api_base: Some("http://127.0.0.1:9".to_string()),
        })
        .unwrap();
    assert!(replayed.idempotent_replay);
    assert_eq!(replayed.delivery.id, delivered.delivery.id);
    assert_eq!(store.list_channel_delivery_attempts(None).unwrap().len(), 1);
    assert_eq!(
        store
            .list_radar_deliveries(Some(&report.run.id))
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn severe_radar_delivery_blocks_without_authorization_or_policy_side_effect_confusion() {
    // CLAIM: radar delivery refuses unauthorized or policy-denied sends before
    // provider/channel messages, while still leaving an inspectable radar
    // delivery row and policy decision where applicable.
    // ORACLE: no channel messages or channel attempts are written, blocked
    // radar delivery rows contain redacted errors, and policy decisions record
    // deny effects.
    // SEVERITY: Severe because delivery is a cross-service trust boundary.
    let store = test_store("radar-delivery-blocked");
    store
        .add_source_card(SourceCardInput {
            title: "Radar blocked delivery launch".to_string(),
            url: "https://example.com/radar-blocked-delivery-launch".to_string(),
            source_type: "web".to_string(),
            provider: "fixture".to_string(),
            summary: "Source card supports blocked radar delivery proof.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({ "source_kind": "manual" }),
        })
        .unwrap();
    let profile = store
        .create_radar_profile(RadarProfileInput {
            name: "blocked-delivery-radar".to_string(),
            description: "Blocked delivery radar".to_string(),
            window_hours: 24,
            min_score: 1.0,
            max_items: Some(10),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "blocked delivery" }]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({}),
        })
        .unwrap();
    let report = store.run_radar_profile(&profile.id, None).unwrap();
    store
        .summarize_radar_run(&report.run.id, "en", "markdown")
        .unwrap();

    let unauthorized = store
        .deliver_radar_summary(RadarDeliveryInput {
            run_id: report.run.id.clone(),
            language: "en".to_string(),
            format: "markdown".to_string(),
            channel: "telegram".to_string(),
            recipient_ref: "123".to_string(),
            idempotency_key: Some("radar-delivery-unauthorized".to_string()),
            telegram_bot_token: Some("TOKEN_SHOULD_NOT_LEAK_123".to_string()),
            email_account_id: None,
            email_api_token: None,
            email_from: None,
            api_base: Some("http://127.0.0.1:9".to_string()),
        })
        .unwrap();
    assert_eq!(unauthorized.delivery.status, "blocked");
    assert!(
        unauthorized
            .delivery
            .error
            .as_deref()
            .unwrap_or("")
            .contains("not authorized")
    );
    assert!(
        !serde_json::to_string(&unauthorized)
            .unwrap()
            .contains("TOKEN_SHOULD_NOT_LEAK")
    );
    assert!(unauthorized.channel_message.is_none());
    assert!(unauthorized.channel_delivery_attempt.is_none());
    assert!(store.list_channel_messages().unwrap().is_empty());
    assert!(
        store
            .list_channel_delivery_attempts(None)
            .unwrap()
            .is_empty()
    );

    store
        .authorize_channel_subject("telegram", "telegram:chat:123", false, false, true)
        .unwrap();
    write_policy(
        &store,
        r#"
[[rules]]
id = "deny-radar-delivery-send"
effect = "deny"
action = "channel.send"
reason = "radar delivery disabled during test"
provider = "telegram"
channel = "telegram"
"#,
    );
    let denied = store
        .deliver_radar_summary(RadarDeliveryInput {
            run_id: report.run.id.clone(),
            language: "en".to_string(),
            format: "markdown".to_string(),
            channel: "telegram".to_string(),
            recipient_ref: "123".to_string(),
            idempotency_key: Some("radar-delivery-policy-denied".to_string()),
            telegram_bot_token: Some("TOKEN".to_string()),
            email_account_id: None,
            email_api_token: None,
            email_from: None,
            api_base: Some("http://127.0.0.1:9".to_string()),
        })
        .unwrap();
    assert_eq!(denied.delivery.status, "blocked");
    assert!(
        denied
            .delivery
            .error
            .as_deref()
            .unwrap_or("")
            .contains("policy denied channel.send")
    );
    assert!(store.list_channel_messages().unwrap().is_empty());
    assert!(
        store
            .list_channel_delivery_attempts(None)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store
            .list_radar_deliveries(Some(&report.run.id))
            .unwrap()
            .len(),
        2
    );
    let decisions = store.list_policy_decisions(10).unwrap();
    assert!(decisions.iter().any(|decision| {
        decision.action == "channel.send" && !decision.allowed && decision.effect == "deny"
    }));
}

#[test]
fn severe_radar_delivery_blocked_idempotency_key_can_retry_after_authorization() {
    // CLAIM: an authorization mistake creates inspectable blocked evidence,
    // but does not permanently poison the default/idempotent retry path after
    // the operator fixes channel authorization.
    // ORACLE: the second call with the same idempotency key reaches the local
    // provider, updates the original radar delivery row to sent, and does not
    // report an idempotent replay.
    // SEVERITY: Severe because "blocked once, blocked forever" would make
    // delivery remediation look successful in logs while never reaching a
    // provider.
    let store = test_store("radar-delivery-blocked-retry");
    store
        .add_source_card(SourceCardInput {
            title: "Radar retry delivery launch".to_string(),
            url: "https://example.com/radar-retry-delivery-launch".to_string(),
            source_type: "web".to_string(),
            provider: "fixture".to_string(),
            summary: "Source card supports retryable radar delivery proof.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({ "source_kind": "manual" }),
        })
        .unwrap();
    let profile = store
        .create_radar_profile(RadarProfileInput {
            name: "retry-delivery-radar".to_string(),
            description: "Retry delivery radar".to_string(),
            window_hours: 24,
            min_score: 1.0,
            max_items: Some(10),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "retry delivery" }]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({}),
        })
        .unwrap();
    let report = store.run_radar_profile(&profile.id, None).unwrap();
    store
        .summarize_radar_run(&report.run.id, "en", "markdown")
        .unwrap();

    let blocked = store
        .deliver_radar_summary(RadarDeliveryInput {
            run_id: report.run.id.clone(),
            language: "en".to_string(),
            format: "markdown".to_string(),
            channel: "telegram".to_string(),
            recipient_ref: "123".to_string(),
            idempotency_key: Some("radar-delivery-retry-key".to_string()),
            telegram_bot_token: Some("TOKEN".to_string()),
            email_account_id: None,
            email_api_token: None,
            email_from: None,
            api_base: Some("http://127.0.0.1:9".to_string()),
        })
        .unwrap();
    assert_eq!(blocked.delivery.status, "blocked");
    assert!(blocked.channel_delivery_attempt.is_none());

    store
        .authorize_channel_subject("telegram", "telegram:chat:123", false, false, true)
        .unwrap();
    let api = mock_status_server("200 OK", "", r#"{"ok":true}"#, "application/json");
    let delivered = store
        .deliver_radar_summary(RadarDeliveryInput {
            run_id: report.run.id.clone(),
            language: "en".to_string(),
            format: "markdown".to_string(),
            channel: "telegram".to_string(),
            recipient_ref: "123".to_string(),
            idempotency_key: Some("radar-delivery-retry-key".to_string()),
            telegram_bot_token: Some("TOKEN".to_string()),
            email_account_id: None,
            email_api_token: None,
            email_from: None,
            api_base: Some(api),
        })
        .unwrap();
    assert_eq!(delivered.delivery.id, blocked.delivery.id);
    assert_eq!(delivered.delivery.status, "sent");
    assert!(!delivered.idempotent_replay);
    assert!(delivered.channel_delivery_attempt.is_some());
    assert_eq!(
        store
            .list_radar_deliveries(Some(&report.run.id))
            .unwrap()
            .len(),
        1
    );
    assert_eq!(store.list_channel_delivery_attempts(None).unwrap().len(), 1);
}

#[test]
fn severe_radar_delivery_provider_failure_is_failed_attempt_not_successful_delivery() {
    // CLAIM: provider failures are durable failed radar delivery attempts,
    // not silent success and not lost channel-delivery state.
    // ORACLE: HTTP 429 produces a failed radar delivery linked to a failed
    // channel delivery attempt with retry_at; the run is not promoted to
    // delivered.
    // SEVERITY: Severe because provider failure is the easiest way for a
    // delivery system to become a fake success counter.
    let store = test_store("radar-delivery-provider-failure");
    store
        .authorize_channel_subject("telegram", "telegram:chat:123", false, false, true)
        .unwrap();
    store
        .add_source_card(SourceCardInput {
            title: "Radar failed delivery launch".to_string(),
            url: "https://example.com/radar-failed-delivery-launch".to_string(),
            source_type: "web".to_string(),
            provider: "fixture".to_string(),
            summary: "Source card supports failed radar delivery proof.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({ "source_kind": "manual" }),
        })
        .unwrap();
    let profile = store
        .create_radar_profile(RadarProfileInput {
            name: "failed-delivery-radar".to_string(),
            description: "Failed delivery radar".to_string(),
            window_hours: 24,
            min_score: 1.0,
            max_items: Some(10),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "failed delivery" }]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({}),
        })
        .unwrap();
    let report = store.run_radar_profile(&profile.id, None).unwrap();
    store
        .summarize_radar_run(&report.run.id, "en", "markdown")
        .unwrap();
    let api = mock_status_server(
        "429 Too Many Requests",
        "retry-after: 60\r\n",
        r#"{"ok":false,"description":"Too Many Requests"}"#,
        "application/json",
    );

    let failed = store
        .deliver_radar_summary(RadarDeliveryInput {
            run_id: report.run.id.clone(),
            language: "en".to_string(),
            format: "markdown".to_string(),
            channel: "telegram".to_string(),
            recipient_ref: "123".to_string(),
            idempotency_key: Some("radar-delivery-provider-failure".to_string()),
            telegram_bot_token: Some("TOKEN".to_string()),
            email_account_id: None,
            email_api_token: None,
            email_from: None,
            api_base: Some(api),
        })
        .unwrap();
    assert_eq!(failed.delivery.status, "failed");
    let attempt = failed
        .channel_delivery_attempt
        .expect("failed channel attempt");
    assert!(!attempt.ok);
    assert_eq!(attempt.provider_status, 429);
    assert!(attempt.retry_at.is_some());
    assert_eq!(
        failed.delivery.delivery_attempt_id.as_deref(),
        Some(attempt.id.as_str())
    );
    let message = failed.channel_message.expect("failed channel message");
    assert_eq!(message.status, "failed");
    let run = store.read_radar_run(&report.run.id).unwrap().unwrap();
    assert_eq!(run.delivery_count, 1);
    assert_eq!(run.stage, "summarized");
    let no_new_attempt = store.reconcile_radar_delivery_attempts(3).unwrap();
    assert_eq!(no_new_attempt.inspected, 0);
}

#[test]
fn severe_radar_delivery_worker_retry_reconciles_sent_status_to_run() {
    // CLAIM: worker-driven channel retry is also radar delivery recovery,
    // not a hidden transport-only success.
    // ORACLE: after a due Telegram retry succeeds, the original radar
    // delivery row points at the latest successful channel attempt and the
    // radar run is promoted to delivered.
    // SEVERITY: Severe because otherwise operators see a failed radar
    // delivery after the provider has actually accepted the retry.
    let store = test_store("radar-delivery-worker-retry-reconcile");
    store
        .authorize_channel_subject("telegram", "telegram:chat:123", false, false, true)
        .unwrap();
    store
        .add_source_card(SourceCardInput {
            title: "Radar retry reconcile launch".to_string(),
            url: "https://example.com/radar-retry-reconcile-launch".to_string(),
            source_type: "web".to_string(),
            provider: "fixture".to_string(),
            summary: "Source card supports radar retry reconciliation proof.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({ "source_kind": "manual" }),
        })
        .unwrap();
    let profile = store
        .create_radar_profile(RadarProfileInput {
            name: "retry-reconcile-radar".to_string(),
            description: "Retry reconcile radar".to_string(),
            window_hours: 24,
            min_score: 1.0,
            max_items: Some(10),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "retry reconcile" }]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({}),
        })
        .unwrap();
    let report = store.run_radar_profile(&profile.id, None).unwrap();
    store
        .summarize_radar_run(&report.run.id, "en", "markdown")
        .unwrap();
    let failing_base = mock_status_server(
        "429 Too Many Requests",
        "retry-after: 1\r\n",
        r#"{"ok":false,"description":"Too Many Requests"}"#,
        "application/json",
    );
    let failed = store
        .deliver_radar_summary(RadarDeliveryInput {
            run_id: report.run.id.clone(),
            language: "en".to_string(),
            format: "markdown".to_string(),
            channel: "telegram".to_string(),
            recipient_ref: "123".to_string(),
            idempotency_key: Some("radar-delivery-worker-retry-reconcile".to_string()),
            telegram_bot_token: Some("TOKEN".to_string()),
            email_account_id: None,
            email_api_token: None,
            email_from: None,
            api_base: Some(failing_base),
        })
        .unwrap();
    assert_eq!(failed.delivery.status, "failed");
    let first_message = failed.channel_message.expect("failed channel message");
    store
        .conn
        .execute(
            "UPDATE channel_delivery_attempts SET retry_at = ?1 WHERE message_id = ?2",
            params!["2000-01-01T00:00:00.000000000+00:00", first_message.id],
        )
        .unwrap();
    let ok_base = mock_status_server("200 OK", "", r#"{"ok":true}"#, "application/json");
    store
        .set_secret_value("TELEGRAM_BOT_TOKEN", "TOKEN", "telegram")
        .unwrap();
    store
        .set_secret_value("TELEGRAM_API_BASE", &ok_base, "telegram")
        .unwrap();

    let worker = store.run_worker_once(1).unwrap();
    let retry = worker.telegram_retry.expect("worker retry report");
    assert_eq!(retry.sent, 1);
    let reconcile = worker
        .radar_delivery_reconcile
        .expect("radar delivery reconciliation");
    assert_eq!(reconcile.inspected, 1);
    assert_eq!(reconcile.sent, 1);
    assert_eq!(reconcile.updated[0].status, "sent");

    let deliveries = store.list_radar_deliveries(Some(&report.run.id)).unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].status, "sent");
    let attempts = store
        .list_channel_delivery_attempts(Some(&first_message.id))
        .unwrap();
    assert_eq!(attempts.len(), 2);
    let latest_success = attempts.iter().find(|attempt| attempt.ok).unwrap();
    assert_eq!(
        deliveries[0].delivery_attempt_id.as_deref(),
        Some(latest_success.id.as_str())
    );
    let run = store.read_radar_run(&report.run.id).unwrap().unwrap();
    assert_eq!(run.stage, "delivered");
}

#[test]
fn severe_radar_delivery_reconcile_dead_letters_exhausted_retry_chain() {
    // CLAIM: radar delivery retry has a terminal exhausted state and does
    // not keep resending after the configured attempt ceiling.
    // ORACLE: three failed attempts reconcile to a dead_lettered radar
    // delivery, the channel message leaves the retry candidate set, and a
    // subsequent Telegram retry finds no due work.
    // SEVERITY: Severe because repeated provider failure can otherwise
    // become an unattended delivery storm.
    let store = test_store("radar-delivery-dead-letter-reconcile");
    store
        .authorize_channel_subject("telegram", "telegram:chat:123", false, false, true)
        .unwrap();
    store
        .add_source_card(SourceCardInput {
            title: "Radar dead letter launch".to_string(),
            url: "https://example.com/radar-dead-letter-launch".to_string(),
            source_type: "web".to_string(),
            provider: "fixture".to_string(),
            summary: "Source card supports radar delivery dead-letter proof.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({ "source_kind": "manual" }),
        })
        .unwrap();
    let profile = store
        .create_radar_profile(RadarProfileInput {
            name: "dead-letter-delivery-radar".to_string(),
            description: "Dead letter delivery radar".to_string(),
            window_hours: 24,
            min_score: 1.0,
            max_items: Some(10),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "dead letter" }]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({}),
        })
        .unwrap();
    let report = store.run_radar_profile(&profile.id, None).unwrap();
    store
        .summarize_radar_run(&report.run.id, "en", "markdown")
        .unwrap();
    let failing_base = mock_status_server(
        "429 Too Many Requests",
        "retry-after: 1\r\n",
        r#"{"ok":false,"description":"Too Many Requests"}"#,
        "application/json",
    );
    let failed = store
        .deliver_radar_summary(RadarDeliveryInput {
            run_id: report.run.id.clone(),
            language: "en".to_string(),
            format: "markdown".to_string(),
            channel: "telegram".to_string(),
            recipient_ref: "123".to_string(),
            idempotency_key: Some("radar-delivery-dead-letter".to_string()),
            telegram_bot_token: Some("TOKEN".to_string()),
            email_account_id: None,
            email_api_token: None,
            email_from: None,
            api_base: Some(failing_base),
        })
        .unwrap();
    let message = failed.channel_message.expect("failed channel message");
    store
        .record_channel_delivery_attempt(
            &message.id,
            "telegram",
            "telegram:chat:123",
            false,
            429,
            &json!({ "ok": false }),
            Some("provider still rate limited"),
            Some("2000-01-01T00:00:00.000000000+00:00"),
        )
        .unwrap();
    let latest = store
        .record_channel_delivery_attempt(
            &message.id,
            "telegram",
            "telegram:chat:123",
            false,
            429,
            &json!({ "ok": false }),
            Some("provider still rate limited"),
            Some("2000-01-01T00:00:00.000000000+00:00"),
        )
        .unwrap();
    assert_eq!(latest.attempt, 3);

    let reconcile = store.reconcile_radar_delivery_attempts(3).unwrap();
    assert_eq!(reconcile.inspected, 1);
    assert_eq!(reconcile.dead_lettered, 1);
    assert_eq!(reconcile.updated[0].status, "dead_lettered");
    assert_eq!(
        reconcile.updated[0].delivery_attempt_id.as_deref(),
        Some(latest.id.as_str())
    );
    assert!(
        reconcile.updated[0]
            .error
            .as_deref()
            .unwrap_or("")
            .contains("retry exhausted")
    );
    assert_eq!(
        store
            .get_channel_message(&message.id)
            .unwrap()
            .unwrap()
            .status,
        "dead_lettered"
    );
    let ok_base = mock_status_server("200 OK", "", r#"{"ok":true}"#, "application/json");
    let retry = store
        .retry_due_telegram_deliveries("TOKEN", Some(&ok_base), 10)
        .unwrap();
    assert_eq!(retry.attempted, 0);
}

#[test]
fn severe_radar_schedule_worker_delivers_once_and_records_tick_lineage() {
    // CLAIM: scheduled radar delivery is a resident worker path, not just a
    // profile flag or manual delivery helper.
    // ORACLE: worker run-once enqueues and executes a scheduled tick, writes
    // run/summary/delivery lineage onto the tick, reaches the local Telegram
    // provider exactly once, and a second worker pass does not duplicate the
    // interval slot.
    // SEVERITY: Severe because unattended scheduled digests are where fake
    // orchestration can hide behind a healthy-looking profile row.
    let store = test_store("radar-schedule-worker-delivery");
    store
        .authorize_channel_subject("telegram", "telegram:chat:123", false, false, true)
        .unwrap();
    store
        .add_source_card(SourceCardInput {
            title: "Scheduled radar delivery launch".to_string(),
            url: "https://example.com/scheduled-radar-delivery-launch".to_string(),
            source_type: "web".to_string(),
            provider: "fixture".to_string(),
            summary: "Scheduled radar delivery source supports a worker digest.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({ "source_kind": "manual" }),
        })
        .unwrap();
    store
            .create_radar_profile(RadarProfileInput {
                name: "scheduled-delivery-radar".to_string(),
                description: "Scheduled delivery radar".to_string(),
                window_hours: 24,
                min_score: 1.0,
                max_items: Some(10),
                languages: vec!["en".to_string()],
                source_selectors: json!([{ "kind": "source_card_query", "query": "scheduled radar delivery" }]),
                delivery_policy: json!({
                    "delivery": "scheduled",
                    "enabled": true,
                    "interval_hours": 24,
                    "channel": "telegram",
                    "recipient_ref": "123",
                    "language": "en",
                    "format": "markdown"
                }),
                model_policy: json!({ "model_scoring": "disabled" }),
                metadata: json!({}),
            })
            .unwrap();
    let api = mock_status_server("200 OK", "", r#"{"ok":true}"#, "application/json");
    store
        .set_secret_value("TELEGRAM_BOT_TOKEN", "TOKEN", "telegram")
        .unwrap();
    store
        .set_secret_value("TELEGRAM_API_BASE", &api, "telegram")
        .unwrap();

    let first = store.run_worker_once(5).unwrap();
    let schedule = first.radar_schedule.expect("radar schedule report");
    assert_eq!(schedule.inspected, 1);
    assert_eq!(schedule.enqueued, 1);
    assert_eq!(first.completed, 1);
    assert_eq!(first.failed, 0);
    let ticks = store.list_radar_schedule_ticks().unwrap();
    assert_eq!(ticks.len(), 1);
    let tick = &ticks[0];
    assert_eq!(tick.status, "sent");
    assert!(tick.job_id.is_some());
    assert!(tick.run_id.is_some());
    assert!(tick.summary_id.is_some());
    assert!(tick.delivery_id.is_some());
    assert_eq!(store.list_channel_delivery_attempts(None).unwrap().len(), 1);
    let run = store
        .read_radar_run(tick.run_id.as_deref().expect("scheduled run id"))
        .unwrap()
        .unwrap();
    assert_eq!(run.stage, "delivered");
    assert_eq!(run.delivery_count, 1);

    let second = store.run_worker_once(5).unwrap();
    let second_schedule = second.radar_schedule.expect("second schedule report");
    assert_eq!(second_schedule.inspected, 1);
    assert_eq!(second_schedule.enqueued, 0);
    assert_eq!(second.completed, 0);
    assert_eq!(store.list_radar_schedule_ticks().unwrap().len(), 1);
    assert_eq!(store.list_channel_delivery_attempts(None).unwrap().len(), 1);
}

#[test]
fn severe_radar_schedule_rejects_raw_secret_policy_without_tick_or_job() {
    // CLAIM: scheduled delivery policy is configuration, not a place to
    // persist provider secrets or let malicious profile JSON create work.
    // ORACLE: a profile containing a raw Telegram token is skipped with a
    // sanitized scheduler error, and no tick, job, message, or attempt is
    // created.
    // SEVERITY: Severe because scheduled jobs are durable and unattended; a
    // secret-bearing policy would persist sensitive material and amplify it.
    let store = test_store("radar-schedule-secret-policy");
    store
        .add_source_card(SourceCardInput {
            title: "Scheduled radar secret policy".to_string(),
            url: "https://example.com/scheduled-radar-secret-policy".to_string(),
            source_type: "web".to_string(),
            provider: "fixture".to_string(),
            summary: "Source card should not matter because policy is invalid.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({ "source_kind": "manual" }),
        })
        .unwrap();
    store
            .create_radar_profile(RadarProfileInput {
                name: "scheduled-secret-policy-radar".to_string(),
                description: "Invalid scheduled policy radar".to_string(),
                window_hours: 24,
                min_score: 1.0,
                max_items: Some(10),
                languages: vec!["en".to_string()],
                source_selectors: json!([{ "kind": "source_card_query", "query": "scheduled radar secret" }]),
                delivery_policy: json!({
                    "delivery": "scheduled",
                    "enabled": true,
                    "interval_hours": 24,
                    "channel": "telegram",
                    "recipient_ref": "123",
                    "telegram_bot_token": "TOKEN_SHOULD_NOT_BE_STORED_HERE"
                }),
                model_policy: json!({ "model_scoring": "disabled" }),
                metadata: json!({}),
            })
            .unwrap();

    let report = store.run_worker_once(5).unwrap();
    let schedule = report.radar_schedule.expect("radar schedule report");
    assert_eq!(schedule.inspected, 1);
    assert_eq!(schedule.enqueued, 0);
    assert_eq!(schedule.skipped, 1);
    assert_eq!(report.completed, 0);
    assert_eq!(report.failed, 0);
    let serialized = serde_json::to_string(&schedule).unwrap();
    assert!(serialized.contains("raw secrets"));
    assert!(!serialized.contains("TOKEN_SHOULD_NOT_BE_STORED_HERE"));
    assert!(store.list_radar_schedule_ticks().unwrap().is_empty());
    assert!(store.list_wiki_jobs().unwrap().is_empty());
    assert!(store.list_channel_messages().unwrap().is_empty());
    assert!(
        store
            .list_channel_delivery_attempts(None)
            .unwrap()
            .is_empty()
    );
}
