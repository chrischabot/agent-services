use super::*;

#[tokio::test]
async fn severe_ops_ui_knowledge_backlog_controls_require_auth_csrf_policy_and_idempotency() {
    // CLAIM: knowledge backlog ops controls are real, narrow,
    // CSRF-protected mutations over durable watch-source/job state.
    // ORACLE: HTTP status, durable watch_sources/jobs state, policy
    // decision count, duplicate idempotency behavior, and rendered routes.
    // SEVERITY: Severe because otherwise the new autonomous clustering path
    // could remain CLI-only while the ops UI implies operator control.
    let unauthenticated = test_http_state("ops-ui-knowledge-controls-no-auth", None);
    let (no_config_status, no_config_json) = response_json(
        http_ops_knowledge_backlog_schedule(
            State(unauthenticated.clone()),
            HeaderMap::new(),
            Uri::from_static("/ops/actions/knowledge/backlog/schedule"),
            Bytes::from(knowledge_backlog_schedule_body(
                &unauthenticated.csrf_token,
                "ops-ui-knowledge-schedule-no-auth",
                100,
                2,
                12,
                "warm",
                "active",
            )),
        )
        .await,
    )
    .await;
    assert_eq!(no_config_status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        no_config_json
            .pointer("/error/type")
            .and_then(Value::as_str),
        Some("mutation_auth_required")
    );

    let state = test_http_state("ops-ui-knowledge-controls", Some("local-auth-token-123"));
    let store = Store::open(state.paths.clone()).unwrap();
    let denied_schedule_body = knowledge_backlog_schedule_body(
        &state.csrf_token,
        "ops-ui-knowledge-schedule-denied",
        100,
        2,
        12,
        "warm",
        "active",
    );
    let (policy_status, policy_json) = response_json(
        http_ops_knowledge_backlog_schedule(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/backlog/schedule"),
            Bytes::from(denied_schedule_body),
        )
        .await,
    )
    .await;
    assert_eq!(policy_status, StatusCode::BAD_REQUEST);
    assert_eq!(
        policy_json.pointer("/error/type").and_then(Value::as_str),
        Some("ops_action_failed")
    );
    assert!(store.list_watch_sources().unwrap().is_empty());
    assert_eq!(store.list_policy_decisions(10).unwrap().len(), 1);

    let (denied_editorial_status, denied_editorial_json) = response_json(
        http_ops_knowledge_cluster_editorial_decisions_enqueue(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/clusters/enqueue-editorial-decisions"),
            Bytes::from(knowledge_due_clusters_body(
                &state.csrf_token,
                "ops-ui-knowledge-cluster-editorial-denied",
                5,
            )),
        )
        .await,
    )
    .await;
    assert_eq!(denied_editorial_status, StatusCode::BAD_REQUEST);
    assert_eq!(
        denied_editorial_json
            .pointer("/error/type")
            .and_then(Value::as_str),
        Some("ops_action_failed")
    );
    assert!(store.list_wiki_jobs().unwrap().is_empty());

    let (denied_model_schedule_status, denied_model_schedule_json) = response_json(
        http_ops_knowledge_model_clusters_schedule(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/model-clusters/schedule"),
            Bytes::from(knowledge_model_clusters_schedule_body(
                &state.csrf_token,
                "ops-ui-knowledge-model-clusters-denied",
                "agent infrastructure MCP",
                "mock",
                24,
                6,
                "warm",
                "active",
            )),
        )
        .await,
    )
    .await;
    assert_eq!(denied_model_schedule_status, StatusCode::BAD_REQUEST);
    assert_eq!(
        denied_model_schedule_json
            .pointer("/error/type")
            .and_then(Value::as_str),
        Some("ops_action_failed")
    );
    assert!(store.list_watch_sources().unwrap().is_empty());

    let (denied_due_model_writes_status, denied_due_model_writes_json) = response_json(
        http_ops_knowledge_model_writes_enqueue_due(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/model-writes/enqueue-due"),
            Bytes::from(knowledge_due_model_writes_body(
                &state.csrf_token,
                "ops-ui-knowledge-model-writes-due-denied",
                5,
                "mock",
                true,
            )),
        )
        .await,
    )
    .await;
    assert_eq!(denied_due_model_writes_status, StatusCode::BAD_REQUEST);
    assert_eq!(
        denied_due_model_writes_json
            .pointer("/error/type")
            .and_then(Value::as_str),
        Some("ops_action_failed")
    );
    assert!(store.list_wiki_jobs().unwrap().is_empty());

    let (denied_entity_resolution_schedule_status, denied_entity_resolution_schedule_json) =
        response_json(
            http_ops_knowledge_entity_resolution_schedule(
                State(state.clone()),
                authed_local_headers(),
                Uri::from_static("/ops/actions/knowledge/entity-resolution/schedule"),
                Bytes::from(knowledge_entity_resolution_schedule_body(
                    &state.csrf_token,
                    "ops-ui-knowledge-entity-resolution-schedule-denied",
                    5,
                    "mock",
                    "warm",
                    "active",
                )),
            )
            .await,
        )
        .await;
    assert_eq!(
        denied_entity_resolution_schedule_status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        denied_entity_resolution_schedule_json
            .pointer("/error/type")
            .and_then(Value::as_str),
        Some("ops_action_failed")
    );
    assert!(store.list_watch_sources().unwrap().is_empty());

    let (denied_entity_resolution_enqueue_status, denied_entity_resolution_enqueue_json) =
        response_json(
            http_ops_knowledge_entity_resolution_enqueue_due(
                State(state.clone()),
                authed_local_headers(),
                Uri::from_static("/ops/actions/knowledge/entity-resolution/enqueue-due"),
                Bytes::from(knowledge_entity_resolution_enqueue_body(
                    &state.csrf_token,
                    "ops-ui-knowledge-entity-resolution-enqueue-denied",
                    5,
                    "mock",
                )),
            )
            .await,
        )
        .await;
    assert_eq!(
        denied_entity_resolution_enqueue_status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        denied_entity_resolution_enqueue_json
            .pointer("/error/type")
            .and_then(Value::as_str),
        Some("ops_action_failed")
    );
    assert!(store.list_wiki_jobs().unwrap().is_empty());

    let (denied_investigation_status, denied_investigation_json) = response_json(
        http_ops_knowledge_investigation_execution_enqueue(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/investigations/enqueue-execution"),
            Bytes::from(knowledge_due_clusters_body(
                &state.csrf_token,
                "ops-ui-knowledge-investigation-execution-denied",
                5,
            )),
        )
        .await,
    )
    .await;
    assert_eq!(denied_investigation_status, StatusCode::BAD_REQUEST);
    assert_eq!(
        denied_investigation_json
            .pointer("/error/type")
            .and_then(Value::as_str),
        Some("ops_action_failed")
    );
    assert!(store.list_wiki_jobs().unwrap().is_empty());

    let card_a = store
        .add_source_card(SourceCardInput {
            title: "Ops cluster expansion source A".to_string(),
            url: "https://example.com/ops-knowledge/a".to_string(),
            source_type: "web".to_string(),
            provider: "test".to_string(),
            summary: "Ops due expansion evidence says a shared cluster needs wiki expansion."
                .to_string(),
            claims: Vec::new(),
            retrieved_at: None,
            metadata: json!({ "source_role": "primary", "trust_level": "medium" }),
        })
        .unwrap();
    let card_b = store
        .add_source_card(SourceCardInput {
            title: "Ops cluster expansion source B".to_string(),
            url: "https://example.com/ops-knowledge/b".to_string(),
            source_type: "web".to_string(),
            provider: "test".to_string(),
            summary:
                "Ops due expansion evidence says investigation execution must be operator visible."
                    .to_string(),
            claims: Vec::new(),
            retrieved_at: None,
            metadata: json!({ "source_role": "primary", "trust_level": "medium" }),
        })
        .unwrap();
    let projected = store
        .project_knowledge_from_source_card_query(
            "Ops due expansion evidence",
            Some("Ops visible knowledge recurrence trend"),
            10,
        )
        .unwrap();
    assert!(projected.cluster.source_card_ids.contains(&card_a.id));
    assert!(projected.cluster.source_card_ids.contains(&card_b.id));
    let model_invocation = store
        .invoke_knowledge_cluster_model(KnowledgeClusterProposalModelInput {
            source_card_ids: vec![card_a.id.clone(), card_b.id.clone()],
            model_provider: "mock".to_string(),
            model_name: None,
            endpoint: None,
            timeout_seconds: None,
            max_clusters: 6,
        })
        .unwrap();
    let model_cluster = model_invocation.clusters.first().unwrap().clone();
    assert_eq!(model_cluster.status, "candidate");
    assert_eq!(
        model_cluster.metadata.get("origin").and_then(Value::as_str),
        Some("model_cluster_proposal_v1")
    );
    let entity_left = store
        .upsert_knowledge_entity(KnowledgeEntityInput {
            entity_type: "company".to_string(),
            name: "Ops UI OpenAI".to_string(),
            canonical_key: "company:ops-ui-openai".to_string(),
            aliases: vec!["Ops UI OpenAI".to_string()],
            homepage_url: Some("https://openai.com".to_string()),
            source_card_ids: vec![card_a.id.clone()],
            wiki_page_id: None,
            confidence: 0.91,
            metadata: json!({ "seed": "ops-ui-entity-resolution" }),
        })
        .unwrap();
    let entity_right = store
        .upsert_knowledge_entity(KnowledgeEntityInput {
            entity_type: "company".to_string(),
            name: "Ops UI OpenAI LP".to_string(),
            canonical_key: "company:ops-ui-openai-lp".to_string(),
            aliases: vec!["Ops UI OpenAI LP".to_string()],
            homepage_url: Some("https://openai.com".to_string()),
            source_card_ids: vec![card_b.id.clone()],
            wiki_page_id: None,
            confidence: 0.83,
            metadata: json!({ "seed": "ops-ui-entity-resolution" }),
        })
        .unwrap();

    let (denied_promote_status, denied_promote_json) = response_json(
        http_ops_knowledge_cluster_promote(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/clusters/promote"),
            Bytes::from(knowledge_cluster_promote_body(
                &state.csrf_token,
                "ops-ui-knowledge-cluster-promote-denied",
                &model_cluster.id,
                "ops-ui-test",
                "Denied promotion before explicit policy.",
            )),
        )
        .await,
    )
    .await;
    assert_eq!(denied_promote_status, StatusCode::BAD_REQUEST);
    assert_eq!(
        denied_promote_json
            .pointer("/error/type")
            .and_then(Value::as_str),
        Some("ops_action_failed")
    );
    assert_eq!(
        store
            .get_knowledge_cluster(&model_cluster.id)
            .unwrap()
            .unwrap()
            .status,
        "candidate"
    );

    std::fs::write(
        state.paths.home.join("arcwell-policy.toml"),
        r#"
[[rules]]
id = "allow-ops-knowledge-backlog-schedule"
effect = "allow"
action = "ops.knowledge_backlog.schedule"
reason = "local operator may schedule knowledge backlog clustering"

[[rules]]
id = "allow-ops-knowledge-backlog-enqueue"
effect = "allow"
action = "ops.knowledge_backlog.enqueue"
reason = "local operator may enqueue knowledge backlog clustering"

[[rules]]
id = "allow-ops-knowledge-model-clusters-schedule"
effect = "allow"
action = "ops.knowledge_model_clusters.schedule"
reason = "local operator may schedule review-only model cluster proposals"

[[rules]]
id = "allow-ops-knowledge-model-clusters-enqueue"
effect = "allow"
action = "ops.knowledge_model_clusters.enqueue"
reason = "local operator may enqueue review-only model cluster proposals"

[[rules]]
id = "allow-ops-knowledge-cluster-editorial"
effect = "allow"
action = "ops.knowledge_clusters.enqueue_editorial_decisions"
reason = "local operator may enqueue due shared knowledge cluster editorial decisions"

[[rules]]
id = "allow-ops-knowledge-cluster-promote"
effect = "allow"
action = "ops.knowledge_clusters.promote"
reason = "local operator may promote reviewed model-origin clusters"

[[rules]]
id = "allow-ops-knowledge-model-write-schedule"
effect = "allow"
action = "ops.knowledge_model_write.schedule"
reason = "local operator may schedule promoted cluster model writer jobs"

[[rules]]
id = "allow-ops-knowledge-model-write-enqueue"
effect = "allow"
action = "ops.knowledge_model_write.enqueue"
reason = "local operator may enqueue promoted cluster model writer jobs"

[[rules]]
id = "allow-ops-knowledge-model-write-enqueue-due"
effect = "allow"
action = "ops.knowledge_model_write.enqueue_due"
reason = "local operator may enqueue due promoted model-origin cluster writer jobs"

[[rules]]
id = "allow-ops-knowledge-entity-resolution-schedule"
effect = "allow"
action = "ops.knowledge_entity_resolution.schedule"
reason = "local operator may schedule review-only entity resolution jobs"

[[rules]]
id = "allow-ops-knowledge-entity-resolution-enqueue-due"
effect = "allow"
action = "ops.knowledge_entity_resolution.enqueue_due"
reason = "local operator may enqueue due review-only entity resolution jobs"

[[rules]]
id = "allow-core-knowledge-cluster-promote"
effect = "allow"
action = "knowledge_cluster.promote"
package = "arcwell-librarian"
source = "knowledge_cluster_model_review"
reason = "reviewed model-origin cluster may become active"

[[rules]]
id = "allow-ops-knowledge-investigation-execution"
effect = "allow"
action = "ops.knowledge_investigations.enqueue_execution"
reason = "local operator may enqueue due shared knowledge investigation execution"

[[rules]]
id = "allow-worker-enqueue"
effect = "allow"
action = "worker.enqueue"
reason = "ops controls may enqueue local worker jobs"
"#,
    )
    .unwrap();

    let allowed_schedule_body = knowledge_backlog_schedule_body(
        &state.csrf_token,
        "ops-ui-knowledge-schedule-allowed",
        77,
        3,
        9,
        "warm",
        "active",
    );
    let (allowed_status, _) = response_text(
        http_ops_knowledge_backlog_schedule(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/backlog/schedule"),
            Bytes::from(allowed_schedule_body.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(allowed_status, StatusCode::SEE_OTHER);
    let sources = store.list_watch_sources().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].source_kind, "knowledge_backlog");
    assert_eq!(sources[0].locator, "source-cards");
    assert_eq!(sources[0].metadata["max_source_cards"], 77);
    assert_eq!(sources[0].metadata["min_group_size"], 3);
    assert_eq!(sources[0].metadata["max_clusters"], 9);
    let decisions_after_schedule = store.list_policy_decisions(10).unwrap().len();

    let (duplicate_status, _) = response_text(
        http_ops_knowledge_backlog_schedule(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/backlog/schedule"),
            Bytes::from(allowed_schedule_body),
        )
        .await,
    )
    .await;
    assert_eq!(duplicate_status, StatusCode::SEE_OTHER);
    assert_eq!(
        store.list_policy_decisions(10).unwrap().len(),
        decisions_after_schedule
    );

    let (enqueue_status, _) = response_text(
        http_ops_knowledge_backlog_enqueue(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/backlog/enqueue"),
            Bytes::from(knowledge_backlog_enqueue_body(
                &state.csrf_token,
                "ops-ui-knowledge-enqueue-allowed",
                88,
                4,
                10,
            )),
        )
        .await,
    )
    .await;
    assert_eq!(enqueue_status, StatusCode::SEE_OTHER);
    assert!(store.list_wiki_jobs().unwrap().iter().any(|job| job.kind
        == "knowledge_cluster_backlog"
        && job.input_json["max_source_cards"] == 88
        && job.input_json["min_group_size"] == 4
        && job.input_json["max_clusters"] == 10));

    let model_schedule_body = knowledge_model_clusters_schedule_body(
        &state.csrf_token,
        "ops-ui-knowledge-model-clusters-schedule-allowed",
        "Ops due expansion evidence",
        "mock",
        12,
        3,
        "warm",
        "active",
    );
    let (model_schedule_status, _) = response_text(
        http_ops_knowledge_model_clusters_schedule(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/model-clusters/schedule"),
            Bytes::from(model_schedule_body.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(model_schedule_status, StatusCode::SEE_OTHER);
    let sources = store.list_watch_sources().unwrap();
    assert!(
        sources
            .iter()
            .any(|source| source.source_kind == "knowledge_model_clusters"
                && source.locator == "Ops due expansion evidence"
                && source.metadata["model_provider"] == "mock"
                && source.metadata["max_source_cards"] == 12
                && source.metadata["max_clusters"] == 3)
    );
    let decisions_after_model_schedule = store.list_policy_decisions(20).unwrap().len();
    let (model_schedule_duplicate_status, _) = response_text(
        http_ops_knowledge_model_clusters_schedule(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/model-clusters/schedule"),
            Bytes::from(model_schedule_body),
        )
        .await,
    )
    .await;
    assert_eq!(model_schedule_duplicate_status, StatusCode::SEE_OTHER);
    assert_eq!(
        store.list_policy_decisions(20).unwrap().len(),
        decisions_after_model_schedule
    );

    let model_enqueue_body = knowledge_model_clusters_enqueue_body(
        &state.csrf_token,
        "ops-ui-knowledge-model-clusters-enqueue-allowed",
        "Ops due expansion evidence",
        "mock",
        8,
        2,
    );
    let (model_enqueue_status, _) = response_text(
        http_ops_knowledge_model_clusters_enqueue(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/model-clusters/enqueue"),
            Bytes::from(model_enqueue_body.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(model_enqueue_status, StatusCode::SEE_OTHER);
    assert!(store.list_wiki_jobs().unwrap().iter().any(|job| job.kind
        == "knowledge_cluster_model_propose"
        && job.input_json.get("query").and_then(Value::as_str)
            == Some("Ops due expansion evidence")
        && job.input_json["max_source_cards"] == 8
        && job.input_json["max_clusters"] == 2));
    let model_proposal_job_count = store
        .list_wiki_jobs()
        .unwrap()
        .iter()
        .filter(|job| job.kind == "knowledge_cluster_model_propose")
        .count();
    let (model_enqueue_duplicate_status, _) = response_text(
        http_ops_knowledge_model_clusters_enqueue(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/model-clusters/enqueue"),
            Bytes::from(model_enqueue_body),
        )
        .await,
    )
    .await;
    assert_eq!(model_enqueue_duplicate_status, StatusCode::SEE_OTHER);
    assert_eq!(
        store
            .list_wiki_jobs()
            .unwrap()
            .iter()
            .filter(|job| job.kind == "knowledge_cluster_model_propose")
            .count(),
        model_proposal_job_count
    );

    let (cluster_enqueue_status, _) = response_text(
        http_ops_knowledge_cluster_editorial_decisions_enqueue(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/clusters/enqueue-editorial-decisions"),
            Bytes::from(knowledge_due_clusters_body(
                &state.csrf_token,
                "ops-ui-knowledge-cluster-editorial-allowed",
                7,
            )),
        )
        .await,
    )
    .await;
    assert_eq!(cluster_enqueue_status, StatusCode::SEE_OTHER);
    assert!(
        store
            .list_wiki_jobs()
            .unwrap()
            .iter()
            .any(|job| job.kind == "knowledge_cluster_editorial_decide"
                && job.input_json.get("cluster_id").and_then(Value::as_str)
                    == Some(projected.cluster.id.as_str()))
    );
    let editorial_job_count = store
        .list_wiki_jobs()
        .unwrap()
        .iter()
        .filter(|job| job.kind == "knowledge_cluster_editorial_decide")
        .count();
    let (cluster_duplicate_status, _) = response_text(
        http_ops_knowledge_cluster_editorial_decisions_enqueue(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/clusters/enqueue-editorial-decisions"),
            Bytes::from(knowledge_due_clusters_body(
                &state.csrf_token,
                "ops-ui-knowledge-cluster-editorial-allowed",
                7,
            )),
        )
        .await,
    )
    .await;
    assert_eq!(cluster_duplicate_status, StatusCode::SEE_OTHER);
    assert_eq!(
        store
            .list_wiki_jobs()
            .unwrap()
            .iter()
            .filter(|job| job.kind == "knowledge_cluster_editorial_decide")
            .count(),
        editorial_job_count
    );

    let (promote_status, _) = response_text(
        http_ops_knowledge_cluster_promote(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/clusters/promote"),
            Bytes::from(knowledge_cluster_promote_body(
                &state.csrf_token,
                "ops-ui-knowledge-cluster-promote-allowed",
                &model_cluster.id,
                "ops-ui-test",
                "Reviewed source-card evidence and approved active promotion.",
            )),
        )
        .await,
    )
    .await;
    assert_eq!(promote_status, StatusCode::SEE_OTHER);
    let promoted = store
        .get_knowledge_cluster(&model_cluster.id)
        .unwrap()
        .unwrap();
    assert_eq!(promoted.status, "active");
    assert!(
        promoted
            .metadata
            .get("promotion")
            .and_then(|value| value.get("policy_decision_id"))
            .and_then(Value::as_str)
            .is_some()
    );
    assert!(
        store
            .list_knowledge_editorial_decisions(20)
            .unwrap()
            .iter()
            .any(|decision| decision.cluster_id == model_cluster.id
                && decision.decision == "promote_model_cluster"
                && decision.status == "completed")
    );
    let decisions_after_promote = store.list_policy_decisions(20).unwrap();
    assert!(decisions_after_promote.iter().any(
            |decision| decision.allowed && decision.action == "ops.knowledge_clusters.promote"
        ));
    assert!(
        decisions_after_promote
            .iter()
            .any(|decision| decision.allowed && decision.action == "knowledge_cluster.promote")
    );
    let editorial_count_after_promote = store.list_knowledge_editorial_decisions(20).unwrap().len();
    let (promote_duplicate_status, _) = response_text(
        http_ops_knowledge_cluster_promote(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/clusters/promote"),
            Bytes::from(knowledge_cluster_promote_body(
                &state.csrf_token,
                "ops-ui-knowledge-cluster-promote-allowed",
                &model_cluster.id,
                "ops-ui-test",
                "Reviewed source-card evidence and approved active promotion.",
            )),
        )
        .await,
    )
    .await;
    assert_eq!(promote_duplicate_status, StatusCode::SEE_OTHER);
    assert_eq!(
        store.list_knowledge_editorial_decisions(20).unwrap().len(),
        editorial_count_after_promote
    );

    let model_write_schedule_body = knowledge_model_write_schedule_body(
        &state.csrf_token,
        "ops-ui-knowledge-model-write-schedule-allowed",
        &model_cluster.id,
        "mock",
        true,
        "warm",
        "active",
    );
    let (model_write_schedule_status, _) = response_text(
        http_ops_knowledge_model_write_schedule(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/model-writes/schedule"),
            Bytes::from(model_write_schedule_body.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(model_write_schedule_status, StatusCode::SEE_OTHER);
    let sources = store.list_watch_sources().unwrap();
    assert!(
        sources
            .iter()
            .any(|source| source.source_kind == "knowledge_model_write"
                && source.locator == model_cluster.id
                && source.metadata["model_provider"] == "mock"
                && source.metadata["create_digest"] == true)
    );
    let decisions_after_model_write_schedule = store.list_policy_decisions(30).unwrap().len();
    let (model_write_schedule_duplicate_status, _) = response_text(
        http_ops_knowledge_model_write_schedule(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/model-writes/schedule"),
            Bytes::from(model_write_schedule_body),
        )
        .await,
    )
    .await;
    assert_eq!(model_write_schedule_duplicate_status, StatusCode::SEE_OTHER);
    assert_eq!(
        store.list_policy_decisions(30).unwrap().len(),
        decisions_after_model_write_schedule
    );

    let model_write_enqueue_body = knowledge_model_write_enqueue_body(
        &state.csrf_token,
        "ops-ui-knowledge-model-write-enqueue-allowed",
        &model_cluster.id,
        "mock",
        false,
    );
    let (model_write_enqueue_status, _) = response_text(
        http_ops_knowledge_model_write_enqueue(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/model-writes/enqueue"),
            Bytes::from(model_write_enqueue_body.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(model_write_enqueue_status, StatusCode::SEE_OTHER);
    assert!(store.list_wiki_jobs().unwrap().iter().any(|job| job.kind
        == "knowledge_cluster_model_write"
        && job.input_json.get("cluster_id").and_then(Value::as_str)
            == Some(model_cluster.id.as_str())
        && job.input_json["create_digest"] == false));
    let model_write_job_count = store
        .list_wiki_jobs()
        .unwrap()
        .iter()
        .filter(|job| job.kind == "knowledge_cluster_model_write")
        .count();
    let (model_write_enqueue_duplicate_status, _) = response_text(
        http_ops_knowledge_model_write_enqueue(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/model-writes/enqueue"),
            Bytes::from(model_write_enqueue_body),
        )
        .await,
    )
    .await;
    assert_eq!(model_write_enqueue_duplicate_status, StatusCode::SEE_OTHER);
    assert_eq!(
        store
            .list_wiki_jobs()
            .unwrap()
            .iter()
            .filter(|job| job.kind == "knowledge_cluster_model_write")
            .count(),
        model_write_job_count
    );

    let due_model_writes_body = knowledge_due_model_writes_body(
        &state.csrf_token,
        "ops-ui-knowledge-model-writes-due-allowed",
        7,
        "mock",
        true,
    );
    let (due_model_writes_status, _) = response_text(
        http_ops_knowledge_model_writes_enqueue_due(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/model-writes/enqueue-due"),
            Bytes::from(due_model_writes_body.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(due_model_writes_status, StatusCode::SEE_OTHER);
    assert!(
        store
            .list_policy_decisions(50)
            .unwrap()
            .iter()
            .any(|decision| decision.allowed
                && decision.action == "ops.knowledge_model_write.enqueue_due")
    );
    assert_eq!(
        store
            .list_wiki_jobs()
            .unwrap()
            .iter()
            .filter(|job| job.kind == "knowledge_cluster_model_write")
            .count(),
        model_write_job_count
    );
    let decisions_after_due_model_writes = store.list_policy_decisions(50).unwrap().len();
    let (due_model_writes_duplicate_status, _) = response_text(
        http_ops_knowledge_model_writes_enqueue_due(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/model-writes/enqueue-due"),
            Bytes::from(due_model_writes_body),
        )
        .await,
    )
    .await;
    assert_eq!(due_model_writes_duplicate_status, StatusCode::SEE_OTHER);
    assert_eq!(
        store.list_policy_decisions(50).unwrap().len(),
        decisions_after_due_model_writes
    );

    let entity_resolution_schedule_body = knowledge_entity_resolution_schedule_body(
        &state.csrf_token,
        "ops-ui-knowledge-entity-resolution-schedule-allowed",
        11,
        "mock",
        "warm",
        "active",
    );
    let (entity_resolution_schedule_status, _) = response_text(
        http_ops_knowledge_entity_resolution_schedule(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/entity-resolution/schedule"),
            Bytes::from(entity_resolution_schedule_body.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(entity_resolution_schedule_status, StatusCode::SEE_OTHER);
    let sources = store.list_watch_sources().unwrap();
    assert!(
        sources
            .iter()
            .any(|source| source.source_kind == "knowledge_entity_resolution"
                && source.locator == "entities"
                && source.metadata["model_provider"] == "mock"
                && source.metadata["max_pairs"] == 11)
    );
    let decisions_after_entity_resolution_schedule = store.list_policy_decisions(60).unwrap().len();
    let (entity_resolution_schedule_duplicate_status, _) = response_text(
        http_ops_knowledge_entity_resolution_schedule(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/entity-resolution/schedule"),
            Bytes::from(entity_resolution_schedule_body),
        )
        .await,
    )
    .await;
    assert_eq!(
        entity_resolution_schedule_duplicate_status,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        store.list_policy_decisions(60).unwrap().len(),
        decisions_after_entity_resolution_schedule
    );

    let entity_resolution_enqueue_body = knowledge_entity_resolution_enqueue_body(
        &state.csrf_token,
        "ops-ui-knowledge-entity-resolution-enqueue-allowed",
        7,
        "mock",
    );
    let (entity_resolution_enqueue_status, _) = response_text(
        http_ops_knowledge_entity_resolution_enqueue_due(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/entity-resolution/enqueue-due"),
            Bytes::from(entity_resolution_enqueue_body.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(entity_resolution_enqueue_status, StatusCode::SEE_OTHER);
    assert!(
        store
            .list_policy_decisions(80)
            .unwrap()
            .iter()
            .any(|decision| decision.allowed
                && decision.action == "ops.knowledge_entity_resolution.enqueue_due")
    );
    let entity_resolution_jobs: Vec<_> = store
        .list_wiki_jobs()
        .unwrap()
        .into_iter()
        .filter(|job| job.kind == "knowledge_entity_resolution_model")
        .collect();
    assert!(!entity_resolution_jobs.is_empty());
    let entity_resolution_job_count = entity_resolution_jobs.len();
    let entity_job = entity_resolution_jobs
        .iter()
        .find(|job| {
            let left_job_id = job.input_json.get("left_entity_id").and_then(Value::as_str);
            let right_job_id = job
                .input_json
                .get("right_entity_id")
                .and_then(Value::as_str);
            [left_job_id, right_job_id].contains(&Some(entity_left.id.as_str()))
                && [left_job_id, right_job_id].contains(&Some(entity_right.id.as_str()))
        })
        .expect("entity-resolution enqueue should include the seeded source-card-backed pair");
    assert_eq!(
        entity_job
            .input_json
            .pointer("/lineage/trigger")
            .and_then(Value::as_str),
        Some("ops_ui_enqueue_due")
    );
    let (entity_resolution_enqueue_duplicate_status, _) = response_text(
        http_ops_knowledge_entity_resolution_enqueue_due(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/entity-resolution/enqueue-due"),
            Bytes::from(entity_resolution_enqueue_body),
        )
        .await,
    )
    .await;
    assert_eq!(
        entity_resolution_enqueue_duplicate_status,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        store
            .list_wiki_jobs()
            .unwrap()
            .iter()
            .filter(|job| job.kind == "knowledge_entity_resolution_model")
            .count(),
        entity_resolution_job_count
    );

    store
        .create_knowledge_cluster_investigation(&projected.cluster.id)
        .unwrap();
    let (investigation_enqueue_status, _) = response_text(
        http_ops_knowledge_investigation_execution_enqueue(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/investigations/enqueue-execution"),
            Bytes::from(knowledge_due_clusters_body(
                &state.csrf_token,
                "ops-ui-knowledge-investigation-execution-allowed",
                7,
            )),
        )
        .await,
    )
    .await;
    assert_eq!(investigation_enqueue_status, StatusCode::SEE_OTHER);
    assert!(
        store
            .list_wiki_jobs()
            .unwrap()
            .iter()
            .any(|job| job.kind == "knowledge_cluster_investigation_execute"
                && job.input_json.get("cluster_id").and_then(Value::as_str)
                    == Some(projected.cluster.id.as_str()))
    );
    let investigation_job_count = store
        .list_wiki_jobs()
        .unwrap()
        .iter()
        .filter(|job| job.kind == "knowledge_cluster_investigation_execute")
        .count();
    let (investigation_duplicate_status, _) = response_text(
        http_ops_knowledge_investigation_execution_enqueue(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/knowledge/investigations/enqueue-execution"),
            Bytes::from(knowledge_due_clusters_body(
                &state.csrf_token,
                "ops-ui-knowledge-investigation-execution-allowed",
                7,
            )),
        )
        .await,
    )
    .await;
    assert_eq!(investigation_duplicate_status, StatusCode::SEE_OTHER);
    assert_eq!(
        store
            .list_wiki_jobs()
            .unwrap()
            .iter()
            .filter(|job| job.kind == "knowledge_cluster_investigation_execute")
            .count(),
        investigation_job_count
    );

    let (bad_form_status, bad_form_json) = response_json(
            http_ops_knowledge_backlog_enqueue(
                State(state.clone()),
                authed_local_headers(),
                Uri::from_static("/ops/actions/knowledge/backlog/enqueue"),
                Bytes::from(format!(
                    "csrf_token={}&idempotency_key={}&max_source_cards=0&min_group_size=2&max_clusters=5",
                    url_component(&state.csrf_token),
                    url_component("ops-ui-knowledge-bad-form")
                )),
            )
            .await,
        )
        .await;
    assert_eq!(bad_form_status, StatusCode::BAD_REQUEST);
    assert_eq!(
        bad_form_json.pointer("/error/type").and_then(Value::as_str),
        Some("bad_form")
    );

    let (bad_entity_form_status, bad_entity_form_json) = response_json(
            http_ops_knowledge_entity_resolution_enqueue_due(
                State(state.clone()),
                authed_local_headers(),
                Uri::from_static("/ops/actions/knowledge/entity-resolution/enqueue-due"),
                Bytes::from(format!(
                    "csrf_token={}&idempotency_key={}&max_pairs=0&model_provider=mock&model_name=&endpoint=&timeout_seconds=",
                    url_component(&state.csrf_token),
                    url_component("ops-ui-knowledge-entity-resolution-bad-form")
                )),
            )
            .await,
        )
        .await;
    assert_eq!(bad_entity_form_status, StatusCode::BAD_REQUEST);
    assert_eq!(
        bad_entity_form_json
            .pointer("/error/type")
            .and_then(Value::as_str),
        Some("bad_form")
    );

    let html = render_ops_ui_with_options(
        &store.ops_snapshot().unwrap(),
        &OpsUiOptions::default(),
        Some(&state.csrf_token),
        true,
    );
    assert!(html.contains("Knowledge Controls"));
    assert!(html.contains("/ops/actions/knowledge/backlog/schedule"));
    assert!(html.contains("/ops/actions/knowledge/backlog/enqueue"));
    assert!(html.contains("/ops/actions/knowledge/model-clusters/schedule"));
    assert!(html.contains("/ops/actions/knowledge/model-clusters/enqueue"));
    assert!(html.contains("/ops/actions/knowledge/clusters/enqueue-editorial-decisions"));
    assert!(!html.contains("/ops/actions/knowledge/clusters/enqueue-expansions"));
    assert!(html.contains("/ops/actions/knowledge/clusters/promote"));
    assert!(html.contains("/ops/actions/knowledge/model-writes/schedule"));
    assert!(html.contains("/ops/actions/knowledge/model-writes/enqueue"));
    assert!(html.contains("/ops/actions/knowledge/model-writes/enqueue-due"));
    assert!(html.contains("/ops/actions/knowledge/entity-resolution/schedule"));
    assert!(html.contains("/ops/actions/knowledge/entity-resolution/enqueue-due"));
    assert!(html.contains("/ops/actions/knowledge/investigations/enqueue-execution"));
    assert!(html.contains("Schedule model clustering"));
    assert!(html.contains("Queue model clustering"));
    assert!(html.contains("Queue cluster editorial review"));
    assert!(html.contains("Promote model cluster"));
    assert!(html.contains("Schedule model writer"));
    assert!(html.contains("Queue model writer"));
    assert!(html.contains("Queue due model writers"));
    assert!(html.contains("Schedule entity resolution"));
    assert!(html.contains("Queue due entity resolution"));
    assert!(html.contains("knowledge_backlog"));
}

#[tokio::test]
async fn severe_ops_ui_edge_dead_letter_requires_auth_csrf_idempotency_and_policy() {
    // CLAIM: The only ops UI mutation is narrow and fails closed without auth, local Origin, CSRF, idempotency, and policy allow.
    // POSTCONDITIONS: Failed attempts do not change event status; duplicate successful submissions do not reapply or re-audit.
    // ORACLE: HTTP status, edge-event state, redacted stored error, and policy decision count.
    // SEVERITY: Severe because this is an authenticated local remediation control over durable queue state.
    let unauthenticated = test_http_state("ops-ui-no-auth-mutation", None);
    let unauth_store = Store::open(unauthenticated.paths.clone()).unwrap();
    let unauth_event = unauth_store
        .enqueue_edge_event(
            "telegram",
            "telegram:no-auth",
            json!({ "text": "hello" }),
            3600,
        )
        .unwrap();
    let (no_config_status, no_config_json) = response_json(
        http_ops_edge_event_dead_letter(
            State(unauthenticated.clone()),
            HeaderMap::new(),
            Uri::from_static("/ops/actions/edge-events/dead-letter"),
            Bytes::from(dead_letter_body(
                &unauthenticated.csrf_token,
                "no-auth-key",
                &unauth_event.id,
                "should fail",
            )),
        )
        .await,
    )
    .await;
    assert_eq!(no_config_status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        no_config_json
            .pointer("/error/type")
            .and_then(Value::as_str),
        Some("mutation_auth_required")
    );
    assert_eq!(
        unauth_store
            .get_edge_event(&unauth_event.id)
            .unwrap()
            .unwrap()
            .status,
        "pending"
    );

    let state = test_http_state("ops-ui-dead-letter", Some("local-auth-token-123"));
    let store = Store::open(state.paths.clone()).unwrap();
    let event = store
        .enqueue_edge_event(
            "telegram",
            "telegram:dead-letter",
            json!({ "text": "hello" }),
            3600,
        )
        .unwrap();
    let valid_body = dead_letter_body(
        &state.csrf_token,
        "ops-ui-dead-letter-denied",
        &event.id,
        "manual review",
    );

    let (missing_auth_status, _) = response_json(
        http_ops_edge_event_dead_letter(
            State(state.clone()),
            HeaderMap::new(),
            Uri::from_static("/ops/actions/edge-events/dead-letter"),
            Bytes::from(valid_body.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(missing_auth_status, StatusCode::UNAUTHORIZED);

    let mut hostile_headers = authed_local_headers();
    hostile_headers.insert(
        header::ORIGIN,
        HeaderValue::from_static("https://evil.example"),
    );
    let (hostile_status, _) = response_json(
        http_ops_edge_event_dead_letter(
            State(state.clone()),
            hostile_headers,
            Uri::from_static("/ops/actions/edge-events/dead-letter"),
            Bytes::from(valid_body.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(hostile_status, StatusCode::FORBIDDEN);

    let (bad_csrf_status, bad_csrf_json) = response_json(
        http_ops_edge_event_dead_letter(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/edge-events/dead-letter"),
            Bytes::from(dead_letter_body(
                "wrong-csrf",
                "ops-ui-dead-letter-bad-csrf",
                &event.id,
                "manual review",
            )),
        )
        .await,
    )
    .await;
    assert_eq!(bad_csrf_status, StatusCode::FORBIDDEN);
    assert_eq!(
        bad_csrf_json.pointer("/error/type").and_then(Value::as_str),
        Some("bad_csrf")
    );

    let (policy_status, policy_json) = response_json(
        http_ops_edge_event_dead_letter(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/edge-events/dead-letter"),
            Bytes::from(valid_body),
        )
        .await,
    )
    .await;
    assert_eq!(policy_status, StatusCode::BAD_REQUEST);
    assert_eq!(
        policy_json.pointer("/error/type").and_then(Value::as_str),
        Some("ops_action_failed")
    );
    assert_eq!(
        store.get_edge_event(&event.id).unwrap().unwrap().status,
        "pending"
    );
    assert_eq!(store.list_policy_decisions(10).unwrap().len(), 1);

    std::fs::write(
        state.paths.home.join("arcwell-policy.toml"),
        r#"
[[rules]]
id = "allow-ops-edge-dead-letter"
effect = "allow"
action = "ops.edge_event.dead_letter"
reason = "local operator may dead-letter reviewed edge events"
"#,
    )
    .unwrap();
    let secret = format!("sk-{}", "a".repeat(40));
    let allowed_body = dead_letter_body(
        &state.csrf_token,
        "ops-ui-dead-letter-allowed",
        &event.id,
        &format!("manual review Authorization: Bearer {secret}"),
    );
    let (allowed_status, _) = response_text(
        http_ops_edge_event_dead_letter(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/edge-events/dead-letter"),
            Bytes::from(allowed_body.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(allowed_status, StatusCode::SEE_OTHER);
    let updated = store.get_edge_event(&event.id).unwrap().unwrap();
    assert_eq!(updated.status, "dead_lettered");
    assert!(!updated.error.unwrap_or_default().contains(&secret));
    let decisions_after_success = store.list_policy_decisions(10).unwrap().len();
    assert_eq!(decisions_after_success, 2);

    let (duplicate_status, _) = response_text(
        http_ops_edge_event_dead_letter(
            State(state.clone()),
            authed_local_headers(),
            Uri::from_static("/ops/actions/edge-events/dead-letter"),
            Bytes::from(allowed_body),
        )
        .await,
    )
    .await;
    assert_eq!(duplicate_status, StatusCode::SEE_OTHER);
    assert_eq!(
        store.list_policy_decisions(10).unwrap().len(),
        decisions_after_success
    );
}
