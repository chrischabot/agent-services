use super::*;

#[test]
fn severe_radar_worker_job_runs_profile_and_writes_stage() {
    // CLAIM: queued radar_run jobs are real worker work, not just a foreground
    // command with a matching name.
    // ORACLE: worker run-once completes the job and writes durable radar run,
    // item, FTS, score, and audit-clean stage state.
    // SEVERITY: Severe because scheduled radar without worker execution is a
    // classic Horizon-style digest mirage.
    let store = test_store("radar-worker-run");
    let card = store
        .add_source_card(SourceCardInput {
            title: "Worker radar agent release".to_string(),
            url: "https://example.com/worker-radar-agent".to_string(),
            source_type: "release".to_string(),
            provider: "fixture".to_string(),
            summary: "Worker-scheduled radar finds an agent reliability release.".to_string(),
            claims: vec![SourceClaim {
                claim: "The release improves worker reliability.".to_string(),
                kind: "product".to_string(),
                confidence: 0.8,
            }],
            retrieved_at: Some("2026-06-23T00:00:00Z".to_string()),
            metadata: json!({ "source_role": "primary", "trust_level": "medium" }),
        })
        .unwrap();
    let profile = store
            .create_radar_profile(RadarProfileInput {
                name: "worker-radar".to_string(),
                description: "Worker radar".to_string(),
                window_hours: 24,
                min_score: 3.0,
                max_items: Some(5),
                languages: vec!["en".to_string()],
                source_selectors: json!([{ "kind": "source_card_query", "query": "worker radar agent" }]),
                delivery_policy: json!({ "delivery": "manual_only" }),
                model_policy: json!({ "model_scoring": "disabled" }),
                metadata: json!({}),
            })
            .unwrap();

    let queued = store
        .enqueue_radar_run_job(&profile.name, Some(24), false)
        .unwrap();
    assert_eq!(queued.kind, "radar_run");
    assert_eq!(queued.status, "pending");
    assert_eq!(
        queued.input_json.get("profile").and_then(Value::as_str),
        Some(profile.id.as_str())
    );

    let worker = store.run_worker_once(1).unwrap();
    assert_eq!(worker.processed, 1, "{worker:#?}");
    assert_eq!(worker.completed, 1, "{worker:#?}");
    let completed = &worker.jobs[0];
    assert_eq!(completed.kind, "radar_run");
    assert_eq!(completed.status, "completed");
    let result = completed
        .result_json
        .as_ref()
        .expect("radar worker job should record result JSON");
    assert_eq!(result.get("status").and_then(Value::as_str), Some("scored"));
    assert_eq!(
        result.get("items_inserted").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        result.get("scores_inserted").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        result.get("selected_items").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        result.get("fetch_live").and_then(Value::as_bool),
        Some(false)
    );
    let run_id = result.get("run_id").and_then(Value::as_str).unwrap();

    let stage = store.read_radar_stage(run_id).unwrap();
    assert_eq!(stage.items.len(), 1);
    assert_eq!(
        stage.items[0].source_card_id.as_deref(),
        Some(card.id.as_str())
    );
    assert_eq!(stage.scores.len(), 1);
    assert_eq!(stage.scores[0].status, "selected");
    let audit = store.audit_radar_run(run_id).unwrap();
    assert!(audit.ok, "{audit:?}");
}

#[test]
fn severe_radar_scheduled_delivery_worker_delivers_once_and_records_tick() {
    // CLAIM: scheduled radar delivery is resident worker behavior with
    // durable tick/run/summary/delivery linkage, not just a schedule-shaped
    // schema.
    // ORACLE: one worker pass enqueues and completes exactly one scheduled
    // Telegram delivery; the next pass within the interval does not enqueue
    // a duplicate tick.
    // SEVERITY: Severe because scheduled digests are high mirage risk: a
    // profile, table, or worker job name can look operational while sending
    // nothing or sending repeatedly.
    let store = test_store("radar-scheduled-delivery-worker");
    store
        .authorize_channel_subject("telegram", "telegram:chat:123", false, false, true)
        .unwrap();
    store
        .set_secret_value("TELEGRAM_BOT_TOKEN", "TOKEN", "telegram")
        .unwrap();
    let ok_base = mock_status_server("200 OK", "", r#"{"ok":true}"#, "application/json");
    store
        .set_secret_value("TELEGRAM_API_BASE", &ok_base, "telegram")
        .unwrap();
    store
        .add_source_card(SourceCardInput {
            title: "Scheduled radar agent release".to_string(),
            url: "https://example.com/scheduled-radar-agent".to_string(),
            source_type: "release".to_string(),
            provider: "fixture".to_string(),
            summary: "Scheduled radar finds an agent reliability release.".to_string(),
            claims: vec![SourceClaim {
                claim: "The scheduled release improves worker reliability.".to_string(),
                kind: "product".to_string(),
                confidence: 0.8,
            }],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({ "source_role": "primary", "trust_level": "medium" }),
        })
        .unwrap();
    let profile = store
            .create_radar_profile(RadarProfileInput {
                name: "scheduled-delivery-radar".to_string(),
                description: "Scheduled delivery radar".to_string(),
                window_hours: 24,
                min_score: 1.0,
                max_items: Some(5),
                languages: vec!["en".to_string()],
                source_selectors: json!([{ "kind": "source_card_query", "query": "scheduled radar agent" }]),
                delivery_policy: json!({
                    "delivery": "scheduled",
                    "channel": "telegram",
                    "recipient_ref": "123",
                    "interval_hours": 24,
                    "language": "en",
                    "format": "markdown"
                }),
                model_policy: json!({ "model_scoring": "disabled" }),
                metadata: json!({}),
            })
            .unwrap();

    let worker = store.run_worker_once(2).unwrap();
    let schedule = worker.radar_schedule.as_ref().expect("schedule report");
    assert_eq!(schedule.inspected, 1);
    assert_eq!(schedule.enqueued, 1);
    assert_eq!(worker.processed, 1, "{worker:#?}");
    assert_eq!(worker.completed, 1, "{worker:#?}");
    assert_eq!(worker.jobs[0].kind, "radar_scheduled_delivery");
    let result = worker.jobs[0]
        .result_json
        .as_ref()
        .expect("scheduled delivery result");
    assert_eq!(result.get("status").and_then(Value::as_str), Some("sent"));
    let ticks = store.list_radar_schedule_ticks().unwrap();
    assert_eq!(ticks.len(), 1, "{ticks:?}");
    assert_eq!(ticks[0].profile_id, profile.id);
    assert_eq!(ticks[0].status, "sent");
    assert!(ticks[0].run_id.is_some(), "{ticks:?}");
    assert!(ticks[0].summary_id.is_some(), "{ticks:?}");
    assert!(ticks[0].delivery_id.is_some(), "{ticks:?}");
    let run_id = ticks[0].run_id.as_deref().unwrap();
    assert_eq!(
        store.read_radar_run(run_id).unwrap().unwrap().stage,
        "delivered"
    );
    let deliveries = store.list_radar_deliveries(Some(run_id)).unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].status, "sent");
    assert_eq!(deliveries[0].recipient_ref, "telegram:chat:123");
    let channel_messages = store.list_channel_messages().unwrap();
    assert_eq!(channel_messages.len(), 1);
    assert_eq!(channel_messages[0].status, "sent");
    assert!(channel_messages[0].body.contains("GENERATED_RADAR_SUMMARY"));
    let attempts = store.list_channel_delivery_attempts(None).unwrap();
    assert_eq!(attempts.len(), 1);
    assert!(attempts[0].ok);

    let second = store.run_worker_once(2).unwrap();
    let second_schedule = second.radar_schedule.expect("second schedule report");
    assert_eq!(second_schedule.inspected, 1);
    assert_eq!(second_schedule.enqueued, 0);
    assert_eq!(second.processed, 0);
    assert_eq!(store.list_radar_schedule_ticks().unwrap().len(), 1);
    assert_eq!(store.list_radar_deliveries(Some(run_id)).unwrap().len(), 1);
    assert_eq!(store.list_channel_delivery_attempts(None).unwrap().len(), 1);
}

#[test]
fn severe_radar_scheduled_delivery_defers_quiet_hours_and_resumes_same_tick() {
    // CLAIM: quiet-hours configuration is real scheduled-delivery control,
    // not decorative JSON and not a terminal block.
    // ORACLE: a scheduled profile with active quiet_hours records a deferred
    // tick/job without run/summary/delivery/provider side effects, is not
    // claimed before deferred_until, and later resumes the same tick into a
    // single authorized Telegram delivery when the quiet-hours policy clears.
    // SEVERITY: Severe because unattended schedules must not send inside a
    // quiet window, duplicate ticks while waiting, or get stuck forever.
    let store = test_store("radar-scheduled-delivery-quiet-hours");
    let current_minutes = Utc::now().hour() * 60 + Utc::now().minute();
    let start_minutes = (current_minutes + 24 * 60 - 1) % (24 * 60);
    let end_minutes = (current_minutes + 10) % (24 * 60);
    let quiet_time = |minutes: u32| format!("{:02}:{:02}", minutes / 60, minutes % 60);
    store
        .add_source_card(SourceCardInput {
            title: "Quiet hours radar note".to_string(),
            url: "https://example.com/quiet-hours-radar".to_string(),
            source_type: "note".to_string(),
            provider: "fixture".to_string(),
            summary: "Quiet-hours radar should not silently deliver.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({}),
        })
        .unwrap();
    let profile = store
            .create_radar_profile(RadarProfileInput {
                name: "scheduled-quiet-hours-radar".to_string(),
                description: "Scheduled quiet hours radar".to_string(),
                window_hours: 24,
                min_score: 1.0,
                max_items: Some(5),
                languages: vec!["en".to_string()],
                source_selectors: json!([{ "kind": "source_card_query", "query": "quiet-hours radar" }]),
                delivery_policy: json!({
                    "delivery": "scheduled",
                    "channel": "telegram",
                    "recipient_ref": "123",
                    "interval_hours": 24,
                    "quiet_hours": {
                        "start": quiet_time(start_minutes),
                        "end": quiet_time(end_minutes),
                        "timezone": "UTC"
                    }
                }),
                model_policy: json!({ "model_scoring": "disabled" }),
                metadata: json!({}),
            })
            .unwrap();

    let worker = store.run_worker_once(2).unwrap();
    assert_eq!(worker.processed, 1, "{worker:#?}");
    assert_eq!(worker.completed, 0);
    assert_eq!(worker.deferred, 1);
    assert_eq!(worker.jobs[0].status, "deferred");
    assert_eq!(worker.jobs[0].attempts, 0, "{:?}", worker.jobs[0]);
    assert!(worker.jobs[0].next_run_at.is_some());
    let job_id = worker.jobs[0].id.clone();
    let result = worker.jobs[0].result_json.as_ref().unwrap();
    assert_eq!(
        result.get("status").and_then(Value::as_str),
        Some("deferred")
    );
    let deferred_until = result
        .get("deferred_until")
        .and_then(Value::as_str)
        .expect("deferred_until");
    validate_timestamp(deferred_until).unwrap();
    assert_eq!(worker.jobs[0].next_run_at.as_deref(), Some(deferred_until));
    let ticks = store.list_radar_schedule_ticks().unwrap();
    assert_eq!(ticks.len(), 1);
    let tick_id = ticks[0].id.clone();
    assert_eq!(ticks[0].status, "deferred");
    assert!(
        ticks[0]
            .error
            .as_deref()
            .unwrap_or("")
            .contains("quiet hours active until")
    );
    assert!(ticks[0].run_id.is_none(), "{ticks:?}");
    assert!(ticks[0].summary_id.is_none(), "{ticks:?}");
    assert!(ticks[0].delivery_id.is_none(), "{ticks:?}");
    assert!(store.list_radar_runs().unwrap().is_empty());
    assert!(store.list_radar_deliveries(None).unwrap().is_empty());
    assert!(store.list_channel_messages().unwrap().is_empty());
    assert!(
        store
            .list_channel_delivery_attempts(None)
            .unwrap()
            .is_empty()
    );
    let second = store.run_worker_once(2).unwrap();
    let second_schedule = second.radar_schedule.expect("second schedule report");
    assert_eq!(second_schedule.inspected, 1);
    assert_eq!(second_schedule.enqueued, 0);
    assert_eq!(second.processed, 0);
    assert_eq!(store.list_radar_schedule_ticks().unwrap().len(), 1);

    store
        .authorize_channel_subject("telegram", "telegram:chat:123", false, false, true)
        .unwrap();
    store
        .set_secret_value("TELEGRAM_BOT_TOKEN", "TOKEN", "telegram")
        .unwrap();
    let ok_base = mock_status_server("200 OK", "", r#"{"ok":true}"#, "application/json");
    store
        .set_secret_value("TELEGRAM_API_BASE", &ok_base, "telegram")
        .unwrap();
    let resumed_policy = json!({
        "delivery": "scheduled",
        "channel": "telegram",
        "recipient_ref": "123",
        "interval_hours": 24
    });
    store
        .conn
        .execute(
            "UPDATE radar_profiles SET delivery_policy_json = ?2, updated_at = ?3 WHERE id = ?1",
            params![
                profile.id,
                serde_json::to_string(&resumed_policy).unwrap(),
                now()
            ],
        )
        .unwrap();
    store
        .conn
        .execute(
            "UPDATE wiki_jobs SET next_run_at = ?2 WHERE id = ?1",
            params![job_id, "2000-01-01T00:00:00Z"],
        )
        .unwrap();

    let resumed = store.run_worker_once(2).unwrap();
    assert_eq!(resumed.processed, 1);
    assert_eq!(resumed.completed, 1);
    assert_eq!(resumed.deferred, 0);
    assert_eq!(resumed.jobs[0].id, job_id);
    let ticks = store.list_radar_schedule_ticks().unwrap();
    assert_eq!(ticks.len(), 1);
    assert_eq!(ticks[0].id, tick_id);
    assert_eq!(ticks[0].status, "sent");
    let run_id = ticks[0].run_id.as_deref().expect("resumed run id");
    assert_eq!(
        store.read_radar_run(run_id).unwrap().unwrap().stage,
        "delivered"
    );
    assert_eq!(store.list_radar_deliveries(Some(run_id)).unwrap().len(), 1);
    assert_eq!(store.list_channel_messages().unwrap().len(), 1);
    assert_eq!(store.list_channel_delivery_attempts(None).unwrap().len(), 1);
}

#[test]
fn severe_radar_scheduled_email_delivery_worker_delivers_once_and_records_tick() {
    // CLAIM: scheduled radar delivery is cross-channel local worker
    // behavior, not a Telegram-only special case hidden behind generic
    // policy text.
    // ORACLE: one worker pass sends through the authorized Cloudflare Email
    // path, records schedule/run/summary/delivery lineage, and does not
    // duplicate provider attempts inside the interval.
    // SEVERITY: Severe because "scheduled delivery" is misleading if only
    // one transport actually reaches the provider attempt ledger.
    let store = test_store("radar-scheduled-email-delivery-worker");
    store
        .authorize_channel_subject("email", "email:friend@example.com", false, false, true)
        .unwrap();
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
    let ok_base = mock_status_server(
        "200 OK",
        "",
        r#"{"success":true,"result":{"id":"msg_456"}}"#,
        "application/json",
    );
    store
        .set_secret_value("CLOUDFLARE_EMAIL_API_BASE", &ok_base, "email")
        .unwrap();
    store
        .add_source_card(SourceCardInput {
            title: "Scheduled email radar agent release".to_string(),
            url: "https://example.com/scheduled-email-radar-agent".to_string(),
            source_type: "release".to_string(),
            provider: "fixture".to_string(),
            summary: "Scheduled email radar finds an agent reliability release.".to_string(),
            claims: vec![SourceClaim {
                claim: "The scheduled email release improves worker reliability.".to_string(),
                kind: "product".to_string(),
                confidence: 0.8,
            }],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({ "source_role": "primary", "trust_level": "medium" }),
        })
        .unwrap();
    let profile = store
            .create_radar_profile(RadarProfileInput {
                name: "scheduled-email-delivery-radar".to_string(),
                description: "Scheduled email delivery radar".to_string(),
                window_hours: 24,
                min_score: 1.0,
                max_items: Some(5),
                languages: vec!["en".to_string()],
                source_selectors: json!([{ "kind": "source_card_query", "query": "scheduled email radar agent" }]),
                delivery_policy: json!({
                    "delivery": "scheduled",
                    "channel": "email",
                    "recipient_ref": "friend@example.com",
                    "interval_hours": 24,
                    "language": "en",
                    "format": "markdown"
                }),
                model_policy: json!({ "model_scoring": "disabled" }),
                metadata: json!({}),
            })
            .unwrap();

    let worker = store.run_worker_once(2).unwrap();
    let schedule = worker.radar_schedule.as_ref().expect("schedule report");
    assert_eq!(schedule.inspected, 1);
    assert_eq!(schedule.enqueued, 1);
    assert_eq!(worker.processed, 1);
    assert_eq!(worker.completed, 1, "{worker:#?}");
    assert_eq!(worker.jobs[0].kind, "radar_scheduled_delivery");
    let result = worker.jobs[0]
        .result_json
        .as_ref()
        .expect("scheduled email result");
    assert_eq!(result.get("status").and_then(Value::as_str), Some("sent"));
    assert!(
        !serde_json::to_string(result)
            .unwrap()
            .contains("EMAIL_TOKEN_SHOULD_NOT_LEAK")
    );
    let ticks = store.list_radar_schedule_ticks().unwrap();
    assert_eq!(ticks.len(), 1, "{ticks:?}");
    assert_eq!(ticks[0].profile_id, profile.id);
    assert_eq!(ticks[0].status, "sent");
    let run_id = ticks[0].run_id.as_deref().expect("run id");
    let deliveries = store.list_radar_deliveries(Some(run_id)).unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].status, "sent");
    assert_eq!(deliveries[0].channel, "email");
    assert_eq!(deliveries[0].recipient_ref, "email:friend@example.com");
    assert_eq!(
        store.read_radar_run(run_id).unwrap().unwrap().stage,
        "delivered"
    );
    let channel_messages = store.list_channel_messages().unwrap();
    assert_eq!(channel_messages.len(), 1);
    assert_eq!(channel_messages[0].channel, "email");
    assert_eq!(channel_messages[0].status, "sent");
    assert!(channel_messages[0].body.contains("GENERATED_RADAR_SUMMARY"));
    let attempts = store.list_channel_delivery_attempts(None).unwrap();
    assert_eq!(attempts.len(), 1);
    assert!(attempts[0].ok);

    let second = store.run_worker_once(2).unwrap();
    let second_schedule = second.radar_schedule.expect("second schedule report");
    assert_eq!(second_schedule.inspected, 1);
    assert_eq!(second_schedule.enqueued, 0);
    assert_eq!(second.processed, 0);
    assert_eq!(store.list_radar_schedule_ticks().unwrap().len(), 1);
    assert_eq!(store.list_radar_deliveries(Some(run_id)).unwrap().len(), 1);
    assert_eq!(store.list_channel_delivery_attempts(None).unwrap().len(), 1);
}

#[test]
fn severe_radar_scheduled_email_retry_reconciles_tick_delivery_and_run() {
    // CLAIM: scheduled email delivery retry is not just a transport retry;
    // it reconciles the schedule tick, radar delivery row, and radar run.
    // ORACLE: a failed scheduled email send becomes sent after worker retry,
    // the same channel message gains one new attempt, the tick is promoted
    // from failed to sent, and the run reaches delivered.
    // SEVERITY: Severe because a retry that succeeds while the schedule
    // ledger remains failed is an operational mirage.
    let store = test_store("radar-scheduled-email-retry-reconcile");
    store
        .authorize_channel_subject("email", "email:friend@example.com", false, false, true)
        .unwrap();
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
    let failing_base = mock_status_server(
        "503 Service Unavailable",
        "",
        r#"{"success":false,"errors":[{"message":"temporarily unavailable"}]}"#,
        "application/json",
    );
    store
        .set_secret_value("CLOUDFLARE_EMAIL_API_BASE", &failing_base, "email")
        .unwrap();
    store
        .add_source_card(SourceCardInput {
            title: "Scheduled email retry radar release".to_string(),
            url: "https://example.com/scheduled-email-retry-radar".to_string(),
            source_type: "release".to_string(),
            provider: "fixture".to_string(),
            summary: "Scheduled email retry should reconcile radar state.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({}),
        })
        .unwrap();
    store
            .create_radar_profile(RadarProfileInput {
                name: "scheduled-email-retry-radar".to_string(),
                description: "Scheduled email retry radar".to_string(),
                window_hours: 24,
                min_score: 1.0,
                max_items: Some(5),
                languages: vec!["en".to_string()],
                source_selectors: json!([{ "kind": "source_card_query", "query": "scheduled email retry" }]),
                delivery_policy: json!({
                    "delivery": "scheduled",
                    "channel": "email",
                    "recipient_ref": "friend@example.com",
                    "interval_hours": 24
                }),
                model_policy: json!({ "model_scoring": "disabled" }),
                metadata: json!({}),
            })
            .unwrap();

    let first_worker = store.run_worker_once(2).unwrap();
    assert_eq!(first_worker.processed, 1);
    assert_eq!(first_worker.completed, 1, "{first_worker:#?}");
    let ticks = store.list_radar_schedule_ticks().unwrap();
    assert_eq!(ticks.len(), 1);
    assert_eq!(ticks[0].status, "failed");
    let run_id = ticks[0].run_id.as_deref().expect("failed run id");
    let delivery_id = ticks[0].delivery_id.as_deref().expect("delivery id");
    let deliveries = store.list_radar_deliveries(Some(run_id)).unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].id, delivery_id);
    assert_eq!(deliveries[0].status, "failed");
    let messages = store.list_channel_messages().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].status, "failed");
    store
        .conn
        .execute(
            "UPDATE channel_delivery_attempts SET retry_at = ?1 WHERE message_id = ?2",
            params!["2000-01-01T00:00:00.000000000+00:00", messages[0].id],
        )
        .unwrap();
    let ok_base = mock_status_server(
        "200 OK",
        "",
        r#"{"success":true,"result":{"id":"scheduled_email_retry_ok"}}"#,
        "application/json",
    );
    store
        .set_secret_value("CLOUDFLARE_EMAIL_API_BASE", &ok_base, "email")
        .unwrap();

    let retry_worker = store.run_worker_once(2).unwrap();
    assert_eq!(retry_worker.processed, 0);
    let retry = retry_worker
        .email_retry
        .as_ref()
        .expect("worker email retry");
    assert_eq!(retry.sent, 1);
    let reconcile = retry_worker
        .radar_delivery_reconcile
        .as_ref()
        .expect("radar delivery reconciliation");
    assert_eq!(reconcile.sent, 1);
    assert!(
        !serde_json::to_string(&retry_worker)
            .unwrap()
            .contains("EMAIL_TOKEN_SHOULD_NOT_LEAK")
    );
    let ticks = store.list_radar_schedule_ticks().unwrap();
    assert_eq!(ticks.len(), 1);
    assert_eq!(ticks[0].status, "sent");
    assert!(ticks[0].error.is_none());
    let deliveries = store.list_radar_deliveries(Some(run_id)).unwrap();
    assert_eq!(deliveries[0].status, "sent");
    assert_eq!(
        store.read_radar_run(run_id).unwrap().unwrap().stage,
        "delivered"
    );
    assert_eq!(store.list_channel_messages().unwrap().len(), 1);
    let attempts = store
        .list_channel_delivery_attempts(Some(&messages[0].id))
        .unwrap();
    assert_eq!(attempts.len(), 2);
    assert!(attempts.iter().any(|attempt| attempt.ok));
}

#[test]
fn severe_radar_scheduled_email_retry_dead_letters_tick_without_retry_storm() {
    // CLAIM: exhausted scheduled email retries have a terminal state across
    // radar delivery, schedule tick, and channel message ledgers.
    // ORACLE: the third failed email attempt reconciles to dead_lettered,
    // the linked tick is dead_lettered, and the same message is no longer
    // selected for another retry even if a good provider appears later.
    // SEVERITY: Severe because unattended scheduled email failure must not
    // become an infinite provider retry loop.
    let store = test_store("radar-scheduled-email-dead-letter");
    store
        .authorize_channel_subject("email", "email:friend@example.com", false, false, true)
        .unwrap();
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
    let failing_base = mock_status_server(
        "503 Service Unavailable",
        "",
        r#"{"success":false,"errors":[{"message":"temporarily unavailable"}]}"#,
        "application/json",
    );
    store
        .set_secret_value("CLOUDFLARE_EMAIL_API_BASE", &failing_base, "email")
        .unwrap();
    store
        .add_source_card(SourceCardInput {
            title: "Scheduled email dead letter radar release".to_string(),
            url: "https://example.com/scheduled-email-dead-letter-radar".to_string(),
            source_type: "release".to_string(),
            provider: "fixture".to_string(),
            summary: "Scheduled email retry exhaustion should dead-letter.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({}),
        })
        .unwrap();
    store
            .create_radar_profile(RadarProfileInput {
                name: "scheduled-email-dead-letter-radar".to_string(),
                description: "Scheduled email dead letter radar".to_string(),
                window_hours: 24,
                min_score: 1.0,
                max_items: Some(5),
                languages: vec!["en".to_string()],
                source_selectors: json!([{ "kind": "source_card_query", "query": "email dead letter" }]),
                delivery_policy: json!({
                    "delivery": "scheduled",
                    "channel": "email",
                    "recipient_ref": "friend@example.com",
                    "interval_hours": 24
                }),
                model_policy: json!({ "model_scoring": "disabled" }),
                metadata: json!({}),
            })
            .unwrap();

    let first_worker = store.run_worker_once(2).unwrap();
    assert_eq!(first_worker.completed, 1, "{first_worker:#?}");
    let ticks = store.list_radar_schedule_ticks().unwrap();
    let run_id = ticks[0].run_id.as_deref().expect("run id");
    let messages = store.list_channel_messages().unwrap();
    assert_eq!(messages.len(), 1);
    let message_id = messages[0].id.clone();
    store
        .record_channel_delivery_attempt(
            &message_id,
            "email",
            "email:friend@example.com",
            false,
            503,
            &json!({ "success": false }),
            Some("provider still unavailable"),
            Some("2000-01-01T00:00:00.000000000+00:00"),
        )
        .unwrap();
    let latest = store
        .record_channel_delivery_attempt(
            &message_id,
            "email",
            "email:friend@example.com",
            false,
            503,
            &json!({ "success": false }),
            Some("provider still unavailable"),
            Some("2000-01-01T00:00:00.000000000+00:00"),
        )
        .unwrap();
    assert_eq!(latest.attempt, 3);

    let reconcile = store.reconcile_radar_delivery_attempts(3).unwrap();
    assert_eq!(reconcile.inspected, 1);
    assert_eq!(reconcile.dead_lettered, 1);
    let ticks = store.list_radar_schedule_ticks().unwrap();
    assert_eq!(ticks[0].status, "dead_lettered");
    assert!(
        ticks[0]
            .error
            .as_deref()
            .unwrap_or("")
            .contains("retry exhausted")
    );
    let deliveries = store.list_radar_deliveries(Some(run_id)).unwrap();
    assert_eq!(deliveries[0].status, "dead_lettered");
    assert_eq!(
        store
            .get_channel_message(&message_id)
            .unwrap()
            .unwrap()
            .status,
        "dead_lettered"
    );

    let ok_base = mock_status_server(
        "200 OK",
        "",
        r#"{"success":true,"result":{"id":"should_not_send"}}"#,
        "application/json",
    );
    let retry = store
        .retry_due_email_deliveries(
            "abcd1234",
            "EMAIL_TOKEN_SHOULD_NOT_LEAK",
            "agent@example.com",
            Some(&ok_base),
            10,
        )
        .unwrap();
    assert_eq!(retry.attempted, 0);
    assert_eq!(
        store
            .list_channel_delivery_attempts(Some(&message_id))
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn severe_radar_scheduled_delivery_blocks_unauthorized_recipient_without_provider_send() {
    // CLAIM: scheduled Telegram delivery uses the same authorization boundary
    // as manual radar delivery.
    // ORACLE: an unauthorized recipient records a blocked schedule tick and
    // radar delivery after run/summary proof, but never writes a channel
    // message or provider attempt.
    // SEVERITY: Severe because unattended schedules must not become a
    // confused-deputy bypass of explicit channel authorization.
    let store = test_store("radar-scheduled-delivery-unauthorized");
    store
        .set_secret_value("TELEGRAM_BOT_TOKEN", "TOKEN_SHOULD_NOT_LEAK", "telegram")
        .unwrap();
    store
        .add_source_card(SourceCardInput {
            title: "Scheduled unauthorized radar note".to_string(),
            url: "https://example.com/scheduled-unauthorized-radar".to_string(),
            source_type: "note".to_string(),
            provider: "fixture".to_string(),
            summary: "Scheduled radar should block unauthorized Telegram recipients.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({}),
        })
        .unwrap();
    store
            .create_radar_profile(RadarProfileInput {
                name: "scheduled-unauthorized-radar".to_string(),
                description: "Scheduled unauthorized radar".to_string(),
                window_hours: 24,
                min_score: 1.0,
                max_items: Some(5),
                languages: vec!["en".to_string()],
                source_selectors: json!([{ "kind": "source_card_query", "query": "unauthorized radar" }]),
                delivery_policy: json!({
                    "delivery": "scheduled",
                    "channel": "telegram",
                    "recipient_ref": "123",
                    "interval_hours": 24
                }),
                model_policy: json!({ "model_scoring": "disabled" }),
                metadata: json!({}),
            })
            .unwrap();

    let worker = store.run_worker_once(2).unwrap();
    let schedule = worker.radar_schedule.as_ref().expect("schedule report");
    assert_eq!(schedule.inspected, 1);
    assert_eq!(schedule.enqueued, 1);
    assert_eq!(worker.processed, 1);
    assert_eq!(worker.completed, 1);
    let result = worker.jobs[0].result_json.as_ref().unwrap();
    assert_eq!(
        result.get("status").and_then(Value::as_str),
        Some("blocked")
    );
    assert!(
        !serde_json::to_string(result)
            .unwrap()
            .contains("TOKEN_SHOULD_NOT_LEAK")
    );
    let ticks = store.list_radar_schedule_ticks().unwrap();
    assert_eq!(ticks.len(), 1);
    assert_eq!(ticks[0].status, "blocked");
    assert!(
        ticks[0]
            .error
            .as_deref()
            .unwrap_or("")
            .contains("not authorized")
    );
    let run_id = ticks[0].run_id.as_deref().expect("blocked run id");
    let run = store.read_radar_run(run_id).unwrap().unwrap();
    assert_eq!(run.stage, "summarized");
    let deliveries = store.list_radar_deliveries(Some(run_id)).unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].status, "blocked");
    assert!(
        deliveries[0]
            .error
            .as_deref()
            .unwrap_or("")
            .contains("not authorized")
    );
    assert!(store.list_channel_messages().unwrap().is_empty());
    assert!(
        store
            .list_channel_delivery_attempts(None)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn severe_radar_scheduled_email_delivery_blocks_unauthorized_recipient_without_provider_send() {
    // CLAIM: scheduled email delivery preserves the manual email recipient
    // authorization boundary.
    // ORACLE: missing recipient authorization records blocked radar/tick
    // evidence after run/summary proof, redacts credentials, and writes no
    // channel message or provider attempt.
    // SEVERITY: Severe because unattended email delivery must not bypass
    // explicit channel authorization.
    let store = test_store("radar-scheduled-email-delivery-unauthorized");
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
        .set_secret_value("CLOUDFLARE_EMAIL_API_BASE", "http://127.0.0.1:9", "email")
        .unwrap();
    store
        .add_source_card(SourceCardInput {
            title: "Scheduled unauthorized email radar note".to_string(),
            url: "https://example.com/scheduled-unauthorized-email-radar".to_string(),
            source_type: "note".to_string(),
            provider: "fixture".to_string(),
            summary: "Scheduled radar should block unauthorized email recipients.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({}),
        })
        .unwrap();
    store
            .create_radar_profile(RadarProfileInput {
                name: "scheduled-unauthorized-email-radar".to_string(),
                description: "Scheduled unauthorized email radar".to_string(),
                window_hours: 24,
                min_score: 1.0,
                max_items: Some(5),
                languages: vec!["en".to_string()],
                source_selectors: json!([{ "kind": "source_card_query", "query": "unauthorized email radar" }]),
                delivery_policy: json!({
                    "delivery": "scheduled",
                    "channel": "email",
                    "recipient_ref": "friend@example.com",
                    "interval_hours": 24
                }),
                model_policy: json!({ "model_scoring": "disabled" }),
                metadata: json!({}),
            })
            .unwrap();

    let worker = store.run_worker_once(2).unwrap();
    let schedule = worker.radar_schedule.as_ref().expect("schedule report");
    assert_eq!(schedule.inspected, 1);
    assert_eq!(schedule.enqueued, 1);
    assert_eq!(worker.processed, 1);
    assert_eq!(worker.completed, 1, "{worker:#?}");
    let result = worker.jobs[0].result_json.as_ref().unwrap();
    assert_eq!(
        result.get("status").and_then(Value::as_str),
        Some("blocked")
    );
    assert!(
        !serde_json::to_string(result)
            .unwrap()
            .contains("EMAIL_TOKEN_SHOULD_NOT_LEAK")
    );
    let ticks = store.list_radar_schedule_ticks().unwrap();
    assert_eq!(ticks.len(), 1);
    assert_eq!(ticks[0].status, "blocked");
    assert!(
        ticks[0]
            .error
            .as_deref()
            .unwrap_or("")
            .contains("not authorized")
    );
    let run_id = ticks[0].run_id.as_deref().expect("blocked run id");
    assert_eq!(
        store.read_radar_run(run_id).unwrap().unwrap().stage,
        "summarized"
    );
    let deliveries = store.list_radar_deliveries(Some(run_id)).unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].status, "blocked");
    assert_eq!(deliveries[0].channel, "email");
    assert!(
        deliveries[0]
            .error
            .as_deref()
            .unwrap_or("")
            .contains("not authorized")
    );
    assert!(store.list_channel_messages().unwrap().is_empty());
    assert!(
        store
            .list_channel_delivery_attempts(None)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn severe_radar_scheduled_delivery_cost_denial_records_decision_without_provider_send() {
    // CLAIM: scheduled radar delivery records budget denial as durable
    // radar/cost evidence before any channel message or provider attempt.
    // ORACLE: a provider kill switch produces a completed worker job with a
    // blocked tick and blocked radar delivery linked to the exact denied
    // cost decision; channel messages and provider attempts remain empty.
    // SEVERITY: Severe because always-on scheduled delivery must not become
    // an invisible budget bypass or a silent failed send.
    let store = test_store("radar-scheduled-delivery-cost-denial");
    store
        .authorize_channel_subject("telegram", "telegram:chat:123", false, false, true)
        .unwrap();
    store
        .set_secret_value("TELEGRAM_BOT_TOKEN", "TOKEN_SHOULD_NOT_LEAK", "telegram")
        .unwrap();
    store
        .set_secret_value("TELEGRAM_API_BASE", "http://127.0.0.1:9", "telegram")
        .unwrap();
    store
        .set_cost_policy("provider", "telegram", None, true, None)
        .unwrap();
    store
        .add_source_card(SourceCardInput {
            title: "Scheduled cost-denied radar agent release".to_string(),
            url: "https://example.com/scheduled-cost-denied-radar-agent".to_string(),
            source_type: "release".to_string(),
            provider: "fixture".to_string(),
            summary: "Scheduled radar should stop at cost policy before delivery.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({}),
        })
        .unwrap();
    store
            .create_radar_profile(RadarProfileInput {
                name: "scheduled-cost-denied-radar".to_string(),
                description: "Scheduled cost-denied radar".to_string(),
                window_hours: 24,
                min_score: 1.0,
                max_items: Some(5),
                languages: vec!["en".to_string()],
                source_selectors: json!([{ "kind": "source_card_query", "query": "cost-denied radar" }]),
                delivery_policy: json!({
                    "delivery": "scheduled",
                    "channel": "telegram",
                    "recipient_ref": "123",
                    "interval_hours": 24
                }),
                model_policy: json!({ "model_scoring": "disabled" }),
                metadata: json!({}),
            })
            .unwrap();

    let worker = store.run_worker_once(2).unwrap();
    let schedule = worker.radar_schedule.as_ref().expect("schedule report");
    assert_eq!(schedule.inspected, 1);
    assert_eq!(schedule.enqueued, 1);
    assert_eq!(worker.processed, 1);
    assert_eq!(worker.completed, 1, "{worker:#?}");
    let result = worker.jobs[0].result_json.as_ref().unwrap();
    assert_eq!(
        result.get("status").and_then(Value::as_str),
        Some("blocked")
    );
    assert!(
        !serde_json::to_string(result)
            .unwrap()
            .contains("TOKEN_SHOULD_NOT_LEAK")
    );
    let ticks = store.list_radar_schedule_ticks().unwrap();
    assert_eq!(ticks.len(), 1);
    assert_eq!(ticks[0].status, "blocked");
    assert!(
        ticks[0]
            .error
            .as_deref()
            .unwrap_or("")
            .contains("budget blocked Telegram radar delivery")
    );
    let run_id = ticks[0].run_id.as_deref().expect("blocked run id");
    let deliveries = store.list_radar_deliveries(Some(run_id)).unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].status, "blocked");
    assert_eq!(deliveries[0].channel, "telegram");
    let cost_decision_id = deliveries[0]
        .cost_decision_id
        .as_deref()
        .expect("radar delivery cost decision id");
    assert!(
        deliveries[0]
            .error
            .as_deref()
            .unwrap_or("")
            .contains("cost policy provider:telegram kill switch is enabled")
    );
    let decisions = store.list_cost_decisions(10).unwrap();
    let decision = decisions
        .iter()
        .find(|decision| decision.id == cost_decision_id)
        .expect("linked cost decision");
    assert!(!decision.allowed);
    assert_eq!(decision.provider, "telegram");
    assert!(decision.reason.contains("kill switch"));
    assert_eq!(store.cost_summary().unwrap().2, 0);
    assert!(store.list_channel_messages().unwrap().is_empty());
    assert!(
        store
            .list_channel_delivery_attempts(None)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn severe_radar_scheduled_delivery_rejects_raw_secret_policy_before_enqueue() {
    // CLAIM: scheduled delivery policy stores cadence/recipient intent, not
    // provider credentials.
    // ORACLE: token-shaped policy fields fail due-job enqueue and do not
    // create schedule ticks or worker jobs.
    // SEVERITY: Severe because durable schedule config is surfaced through
    // status, jobs, reports, and backups.
    let store = test_store("radar-scheduled-delivery-raw-secret");
    store
        .add_source_card(SourceCardInput {
            title: "Scheduled raw secret radar note".to_string(),
            url: "https://example.com/scheduled-raw-secret-radar".to_string(),
            source_type: "note".to_string(),
            provider: "fixture".to_string(),
            summary: "Scheduled radar should reject raw provider secrets.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({}),
        })
        .unwrap();
    store
        .create_radar_profile(RadarProfileInput {
            name: "scheduled-raw-secret-radar".to_string(),
            description: "Scheduled raw secret radar".to_string(),
            window_hours: 24,
            min_score: 1.0,
            max_items: Some(5),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "raw secret radar" }]),
            delivery_policy: json!({
                "delivery": "scheduled",
                "channel": "telegram",
                "recipient_ref": "123",
                "interval_hours": 24,
                "telegram_bot_token": "SHOULD_NOT_BE_STORED_HERE",
                "cloudflare_email_api_token": "ALSO_SHOULD_NOT_BE_STORED_HERE"
            }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({}),
        })
        .unwrap();

    let report = store.enqueue_due_radar_schedule_jobs(10).unwrap();
    assert_eq!(report.inspected, 1);
    assert_eq!(report.enqueued, 0);
    assert_eq!(report.skipped, 1);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("must not contain raw secrets")),
        "{:?}",
        report.errors
    );
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("SHOULD_NOT_BE_STORED_HERE")
    );
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("ALSO_SHOULD_NOT_BE_STORED_HERE")
    );
    assert!(store.list_radar_schedule_ticks().unwrap().is_empty());
    assert!(store.list_wiki_jobs().unwrap().is_empty());
}

#[test]
fn severe_radar_scheduled_delivery_rejects_invalid_quiet_hours_before_enqueue() {
    // CLAIM: quiet-hours policy is validated before durable scheduled work
    // exists.
    // ORACLE: unsupported timezones and malformed windows fail the schedule
    // enqueue report with no tick, no worker job, and no provider side
    // effects.
    // SEVERITY: Severe because a malformed quiet-hours policy must not be
    // ignored or converted into an always-send schedule.
    let store = test_store("radar-scheduled-delivery-invalid-quiet-hours");
    store
        .add_source_card(SourceCardInput {
            title: "Scheduled invalid quiet hours radar note".to_string(),
            url: "https://example.com/scheduled-invalid-quiet-hours-radar".to_string(),
            source_type: "note".to_string(),
            provider: "fixture".to_string(),
            summary: "Scheduled radar should reject invalid quiet-hours config.".to_string(),
            claims: vec![],
            retrieved_at: Some("2026-06-24T00:00:00Z".to_string()),
            metadata: json!({}),
        })
        .unwrap();
    store
            .create_radar_profile(RadarProfileInput {
                name: "scheduled-invalid-quiet-hours-radar".to_string(),
                description: "Scheduled invalid quiet hours radar".to_string(),
                window_hours: 24,
                min_score: 1.0,
                max_items: Some(5),
                languages: vec!["en".to_string()],
                source_selectors: json!([{ "kind": "source_card_query", "query": "invalid quiet hours radar" }]),
                delivery_policy: json!({
                    "delivery": "scheduled",
                    "channel": "telegram",
                    "recipient_ref": "123",
                    "interval_hours": 24,
                    "quiet_hours": {
                        "start": "22:00",
                        "end": "08:00",
                        "timezone": "Europe/London"
                    }
                }),
                model_policy: json!({ "model_scoring": "disabled" }),
                metadata: json!({}),
            })
            .unwrap();

    let report = store.enqueue_due_radar_schedule_jobs(10).unwrap();
    assert_eq!(report.inspected, 1);
    assert_eq!(report.enqueued, 0);
    assert_eq!(report.skipped, 1);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("timezone='UTC' only")),
        "{:?}",
        report.errors
    );
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

#[test]
fn severe_radar_worker_live_policy_denial_writes_blocked_run() {
    // CLAIM: queued live radar cannot hide provider denial behind a completed
    // worker job.
    // ORACLE: the worker job completes with a blocked radar run payload,
    // failed adapter job, source-health failure, no cursor, and failed audit.
    // SEVERITY: Severe because unattended live radar is where empty scheduled
    // jobs most easily look operational.
    let store = test_store("radar-worker-live-deny");
    write_policy(
        &store,
        r#"
[[rules]]
id = "allow-worker-radar-enqueue"
effect = "allow"
action = "worker.enqueue"
package = "arcwell-radar"
reason = "Allow queuing radar orchestration while denying provider execution"

[[rules]]
id = "deny-worker-radar-rss"
effect = "deny"
action = "provider.network"
provider = "rss"
source = "rss_fetch"
reason = "RSS disabled for queued radar policy test"
"#,
    );
    let profile = store
        .create_radar_profile(RadarProfileInput {
            name: "worker-live-radar".to_string(),
            description: "Worker live radar".to_string(),
            window_hours: 24,
            min_score: 1.0,
            max_items: Some(5),
            languages: vec!["en".to_string()],
            source_selectors: json!([
                { "kind": "rss", "locator": "https://example.com/worker-feed.xml", "limit": 3 }
            ]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({}),
        })
        .unwrap();

    store
        .enqueue_radar_run_job(&profile.id, None, true)
        .unwrap();
    let worker = store.run_worker_once(1).unwrap();
    assert_eq!(worker.completed, 1);
    let result = worker.jobs[0]
        .result_json
        .as_ref()
        .expect("blocked radar run should still be recorded as job result");
    assert_eq!(
        result.get("status").and_then(Value::as_str),
        Some("blocked")
    );
    assert_eq!(
        result.get("fetch_live").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        result.get("adapter_job_count").and_then(Value::as_u64),
        Some(1)
    );
    let adapter_jobs = result
        .get("adapter_jobs")
        .and_then(Value::as_array)
        .expect("adapter job summary should be present");
    assert_eq!(
        adapter_jobs[0].get("kind").and_then(Value::as_str),
        Some("rss_fetch")
    );
    assert_eq!(
        adapter_jobs[0].get("status").and_then(Value::as_str),
        Some("failed")
    );
    assert!(
        adapter_jobs[0]
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("policy denied provider.network")
    );
    let run_id = result.get("run_id").and_then(Value::as_str).unwrap();

    let health = store
        .get_source_health("rss:https://example.com/worker-feed.xml")
        .unwrap()
        .expect("queued live radar failure should record source health");
    assert_ne!(health.status, "healthy");
    assert!(
        health
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("policy denied provider.network")
    );
    assert!(
        store
            .get_cursor("rss:https://example.com/worker-feed.xml")
            .unwrap()
            .is_none()
    );
    let audit = store.audit_radar_run(run_id).unwrap();
    assert!(!audit.ok);
    assert!(audit.findings.iter().any(|finding| {
        finding.code == "radar_live_fetch_failed" && finding.severity == "high"
    }));
}

#[test]
fn severe_radar_enqueue_rejects_missing_profile_and_bad_window_without_jobs() {
    // CLAIM: invalid queued radar requests do not create inert jobs that fail
    // later or clutter ops with unexecutable work.
    // ORACLE: missing profile and invalid window are rejected before insert.
    // SEVERITY: Severe because queued-shell behavior is an operational mirage.
    let store = test_store("radar-worker-invalid-enqueue");
    let missing = store
        .enqueue_radar_run_job("missing-radar-profile", None, false)
        .unwrap_err()
        .to_string();
    assert!(missing.contains("radar profile not found"), "{missing}");
    assert!(store.list_wiki_jobs().unwrap().is_empty());

    let profile = store
        .create_radar_profile(RadarProfileInput {
            name: "invalid-window-radar".to_string(),
            description: "Invalid window radar".to_string(),
            window_hours: 24,
            min_score: 1.0,
            max_items: Some(5),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "agent" }]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({}),
        })
        .unwrap();
    let bad_window = store
        .enqueue_radar_run_job(&profile.id, Some(0), false)
        .unwrap_err()
        .to_string();
    assert!(
        bad_window.contains("window_hours must be greater than zero"),
        "{bad_window}"
    );
    assert!(store.list_wiki_jobs().unwrap().is_empty());
}
