use super::*;

#[test]
fn severe_radar_source_quality_windows_and_health_penalties_are_real() {
    // CLAIM: radar scoring records source-quality windows and visibly accounts
    // for source freshness/source-health, instead of ranking failed or stale
    // sources as if they were healthy.
    // ORACLE: score tags/reasons split an otherwise similar healthy source from
    // a stale failed source, quality rows are durable, and audit fails if the
    // source-quality window is removed after scoring.
    // SEVERITY: Severe because source-quality tables without ranking/audit
    // effects are a plausible Horizon-style telemetry mirage.
    let store = test_store("radar-source-quality-health");
    let healthy_feed = "https://example.com/healthy-feed.xml";
    let failed_feed = "https://example.com/failed-feed.xml";
    let healthy_key = "rss:https://example.com/healthy-feed.xml";
    let failed_key = "rss:https://example.com/failed-feed.xml";
    let healthy_cursor_value = now();
    let healthy_next_run = now_plus_seconds(3600);
    store
        .record_source_success(SourceHealthUpdate {
            key: healthy_key,
            provider: "rss",
            source_kind: "rss",
            locator: healthy_feed,
            last_item_id: Some("healthy-release"),
            last_item_date: Some(&healthy_cursor_value),
            cursor_key: Some(healthy_key),
            cursor_value: Some(&healthy_cursor_value),
            next_run_at: Some(&healthy_next_run),
        })
        .unwrap();
    store
        .record_source_failure(
            failed_key,
            "rss",
            "rss",
            failed_feed,
            "HTTP 429 rate limited while fetching stale feed",
        )
        .unwrap();
    for input in [
        SourceCardInput {
            title: "Agent MCP release from healthy feed".to_string(),
            url: "https://example.com/healthy-agent-mcp-release".to_string(),
            source_type: "rss".to_string(),
            provider: "rss".to_string(),
            summary: "Agent MCP release signal with worker reliability.".to_string(),
            claims: vec![],
            retrieved_at: Some(now()),
            metadata: json!({
                "source_kind": "rss",
                "source_detail": healthy_feed,
                "id": "healthy-release"
            }),
        },
        SourceCardInput {
            title: "Agent MCP release from failed stale feed".to_string(),
            url: "https://example.com/failed-agent-mcp-release".to_string(),
            source_type: "rss".to_string(),
            provider: "rss".to_string(),
            summary: "Agent MCP release signal with worker reliability.".to_string(),
            claims: vec![],
            retrieved_at: Some("2020-01-01T00:00:00Z".to_string()),
            metadata: json!({
                "source_kind": "rss",
                "source_detail": failed_feed,
                "id": "failed-release"
            }),
        },
    ] {
        store.add_source_card(input).unwrap();
    }
    let profile = store
            .create_radar_profile(RadarProfileInput {
                name: "quality-window-radar".to_string(),
                description: "Quality window radar".to_string(),
                window_hours: 24,
                min_score: 2.5,
                max_items: Some(10),
                languages: vec!["en".to_string()],
                source_selectors: json!([{ "kind": "source_card_query", "query": "Agent MCP release" }]),
                delivery_policy: json!({ "delivery": "manual_only" }),
                model_policy: json!({ "model_scoring": "disabled" }),
                metadata: json!({}),
            })
            .unwrap();

    let report = store.run_radar_profile(&profile.id, None).unwrap();
    assert_eq!(report.items_inserted, 2);
    assert_eq!(report.scores_inserted, 2);
    assert_eq!(
        report
            .run
            .metadata
            .get("source_quality_windows")
            .and_then(Value::as_u64),
        Some(2)
    );
    let score_distribution = report
        .run
        .metadata
        .get("score_distribution")
        .expect("run score distribution");
    assert_eq!(
        score_distribution.get("score_kind").and_then(Value::as_str),
        Some("heuristic_v1")
    );
    assert_eq!(
        score_distribution
            .get("score_count")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        score_distribution
            .get("selected_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert!(
        score_distribution
            .get("average")
            .and_then(Value::as_f64)
            .is_some()
    );
    assert!(
        score_distribution
            .get("p90")
            .and_then(Value::as_f64)
            .is_some()
    );
    let stage = store.read_radar_stage(&report.run.id).unwrap();
    let score_by_item = stage
        .scores
        .iter()
        .map(|score| (score.item_id.clone(), score))
        .collect::<BTreeMap<_, _>>();
    let healthy_item = stage
        .items
        .iter()
        .find(|item| {
            item.metadata.get("source_detail").and_then(Value::as_str) == Some(healthy_feed)
        })
        .expect("healthy radar item");
    let failed_item = stage
        .items
        .iter()
        .find(|item| {
            item.metadata.get("source_detail").and_then(Value::as_str) == Some(failed_feed)
        })
        .expect("failed radar item");
    let healthy_score = score_by_item.get(&healthy_item.id).unwrap();
    let failed_score = score_by_item.get(&failed_item.id).unwrap();
    assert!(
        healthy_score.score > failed_score.score + 1.0,
        "healthy_score={} failed_score={} failed_reason={}",
        healthy_score.score,
        failed_score.score,
        failed_score.reason
    );
    assert!(healthy_score.tags.contains(&"fresh-source".to_string()));
    assert!(
        healthy_score
            .tags
            .contains(&"source-health-healthy".to_string())
    );
    assert!(failed_score.tags.contains(&"very-stale-source".to_string()));
    assert!(
        failed_score
            .tags
            .contains(&"source-health-nonhealthy".to_string())
    );
    assert!(failed_score.reason.contains("source-health"));

    assert_eq!(stage.source_quality.len(), 2, "{stage:?}");
    let quality = store.list_radar_source_quality(&report.run.id).unwrap();
    assert_eq!(quality.len(), 2, "{quality:?}");
    let healthy_quality = quality
        .iter()
        .find(|row| row.locator == healthy_feed)
        .expect("healthy source-quality row");
    assert_eq!(healthy_quality.raw_count, 1);
    assert_eq!(healthy_quality.failure_count, 0);
    assert_eq!(healthy_quality.status, "healthy");
    let failed_quality = quality
        .iter()
        .find(|row| row.locator == failed_feed)
        .expect("failed source-quality row");
    assert_eq!(failed_quality.raw_count, 1);
    assert_eq!(failed_quality.accepted_count, 0);
    assert_eq!(failed_quality.failure_count, 1);
    assert_eq!(failed_quality.status, "failed");
    assert_eq!(failed_quality.duplicate_rate, Some(1.0));
    assert!(
        failed_quality.average_score.unwrap() < healthy_quality.average_score.unwrap(),
        "{quality:?}"
    );

    let audit = store.audit_radar_run(&report.run.id).unwrap();
    assert!(audit.ok, "{audit:?}");
    assert_eq!(audit.source_quality_count, 2);
    let ops = store.ops_snapshot().unwrap();
    let ops_run = ops
        .radar_runs
        .iter()
        .find(|run| run.id == report.run.id)
        .expect("radar run visible in ops snapshot");
    assert_eq!(
        ops_run
            .metadata
            .pointer("/score_distribution/score_count")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(ops.radar_source_quality.len(), 2);
    assert!(
        ops.health
            .warnings
            .iter()
            .any(|warning| warning.contains("Radar source quality")),
        "{:?}",
        ops.health.warnings
    );

    store
            .conn
            .execute(
                "UPDATE radar_source_quality SET raw_count = 99, signal_to_noise = 2.0 WHERE run_id = ?1 AND locator = ?2",
                params![report.run.id, healthy_feed],
            )
            .unwrap();
    let drift_audit = store.audit_radar_run(&report.run.id).unwrap();
    assert!(!drift_audit.ok);
    assert!(drift_audit.findings.iter().any(|finding| {
        finding.code == "radar_source_quality_drift" && finding.severity == "high"
    }));
    store
        .record_radar_source_quality_window(&report.run.id)
        .unwrap();
    assert!(store.audit_radar_run(&report.run.id).unwrap().ok);

    store
        .conn
        .execute(
            "DELETE FROM radar_source_quality WHERE run_id = ?1",
            params![report.run.id],
        )
        .unwrap();
    let broken_audit = store.audit_radar_run(&report.run.id).unwrap();
    assert!(!broken_audit.ok);
    assert!(broken_audit.findings.iter().any(|finding| {
        finding.code == "radar_source_quality_missing" && finding.severity == "high"
    }));
}

#[test]
fn severe_radar_source_quality_trends_rank_local_history_without_global_claims() {
    // CLAIM: source-quality trends rank durable local history across runs,
    // rather than returning raw windows or inventing global/community quality.
    // ORACLE: multi-window sources are aggregated with weighted metrics,
    // single-window sources are filtered by min_windows, latest decay/failure
    // is visible, and invalid bounds fail before returning misleading rows.
    // SEVERITY: Severe because trend dashboards are high mirage risk: a table
    // can look authoritative while hiding thin history, hostile locators, or
    // declining recent source quality.
    let store = test_store("radar-source-quality-trends");
    let insert_quality = |run_id: &str,
                          source_kind: &str,
                          locator: &str,
                          window_start: &str,
                          window_end: &str,
                          raw_count: i64,
                          accepted_count: i64,
                          average_score: f64,
                          signal_to_noise: f64,
                          duplicate_rate: f64,
                          failure_count: i64,
                          status: &str| {
        store
            .conn
            .execute(
                r#"
                    INSERT INTO radar_source_quality
                      (id, run_id, source_kind, locator, window_start, window_end,
                       raw_count, accepted_count, average_score, score_p50, score_p90,
                       signal_to_noise, duplicate_rate, delivery_contribution_count,
                       failure_count, status, created_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, ?9, ?10, ?11, 0, ?12, ?13, ?6)
                    "#,
                params![
                    Uuid::new_v4().to_string(),
                    run_id,
                    source_kind,
                    locator,
                    window_start,
                    window_end,
                    raw_count,
                    accepted_count,
                    average_score,
                    signal_to_noise,
                    duplicate_rate,
                    failure_count,
                    status
                ],
            )
            .unwrap();
    };

    let good = "https://example.com/good-feed.xml";
    let decaying = "https://example.com/decay.xml?<script>alert(1)</script>";
    let failing = "https://example.com/failing-feed.xml";
    let thin = "https://example.com/thin-feed.xml";
    insert_quality(
        "run-good-1",
        "rss",
        good,
        "2026-06-20T00:00:00Z",
        "2026-06-21T00:00:00Z",
        10,
        7,
        7.0,
        0.70,
        0.10,
        0,
        "healthy",
    );
    insert_quality(
        "run-good-2",
        "rss",
        good,
        "2026-06-21T00:00:00Z",
        "2026-06-22T00:00:00Z",
        10,
        8,
        7.5,
        0.80,
        0.10,
        0,
        "healthy",
    );
    insert_quality(
        "run-good-3",
        "rss",
        good,
        "2026-06-22T00:00:00Z",
        "2026-06-23T00:00:00Z",
        10,
        9,
        8.0,
        0.90,
        0.05,
        0,
        "healthy",
    );
    insert_quality(
        "run-decay-1",
        "rss",
        decaying,
        "2026-06-20T00:00:00Z",
        "2026-06-21T00:00:00Z",
        10,
        8,
        8.0,
        0.80,
        0.10,
        0,
        "healthy",
    );
    insert_quality(
        "run-decay-2",
        "rss",
        decaying,
        "2026-06-21T00:00:00Z",
        "2026-06-22T00:00:00Z",
        10,
        5,
        6.5,
        0.50,
        0.20,
        0,
        "partial",
    );
    insert_quality(
        "run-decay-3",
        "rss",
        decaying,
        "2026-06-22T00:00:00Z",
        "2026-06-23T00:00:00Z",
        10,
        1,
        4.0,
        0.10,
        0.40,
        0,
        "low_signal",
    );
    insert_quality(
        "run-fail-1",
        "rss",
        failing,
        "2026-06-21T00:00:00Z",
        "2026-06-22T00:00:00Z",
        4,
        2,
        5.0,
        0.50,
        0.00,
        0,
        "partial",
    );
    insert_quality(
        "run-fail-2",
        "rss",
        failing,
        "2026-06-22T00:00:00Z",
        "2026-06-23T00:00:00Z",
        4,
        0,
        2.0,
        0.00,
        0.00,
        1,
        "failed",
    );
    insert_quality(
        "run-thin-1",
        "rss",
        thin,
        "2026-06-22T00:00:00Z",
        "2026-06-23T00:00:00Z",
        10,
        10,
        9.0,
        1.00,
        0.00,
        0,
        "healthy",
    );

    let trends = store.list_radar_source_quality_trends(2, 10).unwrap();
    assert_eq!(trends.len(), 3, "{trends:?}");
    assert_eq!(trends[0].locator, good);
    assert_eq!(trends[0].window_count, 3);
    assert_eq!(trends[0].run_count, 3);
    assert_eq!(trends[0].raw_count, 30);
    assert_eq!(trends[0].accepted_count, 24);
    assert_eq!(trends[0].trend_status, "improving");
    assert_eq!(trends[0].latest_status, "healthy");
    assert!(trends[0].quality_score > trends[1].quality_score);
    assert!((trends[0].signal_to_noise.unwrap() - 0.80).abs() < 0.001);
    assert!(
        !trends.iter().any(|trend| trend.locator == thin),
        "single-window source should not satisfy min_windows=2: {trends:?}"
    );

    let decaying_trend = trends
        .iter()
        .find(|trend| trend.locator == decaying)
        .expect("decaying source trend");
    assert_eq!(decaying_trend.trend_status, "decaying");
    assert_eq!(decaying_trend.latest_status, "low_signal");
    assert_eq!(decaying_trend.non_healthy_count, 2);
    assert_eq!(decaying_trend.accepted_count, 14);
    assert_eq!(decaying_trend.locator, decaying);

    let failing_trend = trends
        .iter()
        .find(|trend| trend.locator == failing)
        .expect("failing source trend");
    assert_eq!(failing_trend.trend_status, "failing");
    assert_eq!(failing_trend.latest_status, "failed");
    assert_eq!(failing_trend.failure_count, 1);

    let limited = store.list_radar_source_quality_trends(1, 2).unwrap();
    assert_eq!(limited.len(), 2);
    assert!(limited.iter().all(|trend| trend.quality_score >= 0.0));
    assert!(
        store
            .list_radar_source_quality_trends(0, 10)
            .unwrap_err()
            .to_string()
            .contains("min_windows")
    );
    assert!(
        store
            .list_radar_source_quality_trends(2, 501)
            .unwrap_err()
            .to_string()
            .contains("limit")
    );
}

#[test]
fn severe_radar_balance_caps_sources_and_categories_without_hiding_rejections() {
    // CLAIM: explicit radar balance metadata can stop one source or category
    // from dominating selected items while keeping rejected items inspectable.
    // ORACLE: source/category quota rows remain in radar_scores with reasons,
    // tags, and source-card provenance, and audit still passes.
    // SEVERITY: Severe because balanced digests are easy to fake by silently
    // dropping rows or by claiming balance while only applying a global limit.
    let store = test_store("radar-balance-quotas");
    for (title, url, source_kind, source_detail, summary) in [
        (
            "Alpha Agent MCP release from feed",
            "https://example.com/feed-alpha-agent",
            "rss",
            "https://example.com/agent-feed.xml",
            "Agent MCP release improves worker reliability.",
        ),
        (
            "Beta Agent MCP funding round from same feed",
            "https://example.com/feed-beta-agent",
            "rss",
            "https://example.com/agent-feed.xml",
            "Agent MCP funding round expands the company.",
        ),
        (
            "Gamma Agent MCP breaking architecture note from GitHub",
            "https://github.com/example/agent/releases/tag/v1",
            "github_release",
            "example/agent",
            "Agent MCP breaking architecture note improves source-card reliability.",
        ),
        (
            "Security MCP vulnerability incident",
            "https://example.com/security-mcp-vulnerability",
            "rss",
            "https://example.com/security-feed.xml",
            "MCP vulnerability incident with technical mitigation details.",
        ),
        (
            "Zulu Agent MCP benchmark results from Hacker News",
            "https://news.ycombinator.com/item?id=123",
            "hackernews",
            "frontpage",
            "Agent MCP benchmark results with detailed implementation notes.",
        ),
    ] {
        store
            .add_source_card(SourceCardInput {
                title: title.to_string(),
                url: url.to_string(),
                source_type: source_kind.to_string(),
                provider: source_kind.to_string(),
                summary: summary.to_string(),
                claims: vec![],
                retrieved_at: Some(now()),
                metadata: json!({
                    "source_kind": source_kind,
                    "source_detail": source_detail,
                }),
            })
            .unwrap();
    }

    let malformed = store
        .create_radar_profile(RadarProfileInput {
            name: "bad-balance-radar".to_string(),
            description: "Bad balance config".to_string(),
            window_hours: 24,
            min_score: 2.0,
            max_items: Some(10),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "MCP" }]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({ "balance": { "max_per_source": 0 } }),
        })
        .unwrap_err()
        .to_string();
    assert!(
        malformed.contains("max_per_source") && malformed.contains("between 1 and 500"),
        "{malformed}"
    );

    let profile = store
        .create_radar_profile(RadarProfileInput {
            name: "balanced-radar".to_string(),
            description: "Balanced radar".to_string(),
            window_hours: 24,
            min_score: 2.0,
            max_items: Some(10),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "MCP" }]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({
                "balance": {
                    "max_per_source": 1,
                    "category_quotas": { "agent": 2 }
                }
            }),
        })
        .unwrap();

    let report = store.run_radar_profile(&profile.id, None).unwrap();
    assert_eq!(report.items_inserted, 5);
    assert_eq!(report.scores_inserted, 5);
    assert_eq!(report.selected_items, 3);
    assert_eq!(
        report
            .run
            .metadata
            .get("balance_config")
            .and_then(|value| value.get("max_per_source"))
            .and_then(Value::as_u64),
        Some(1)
    );

    let stage = store.read_radar_stage(&report.run.id).unwrap();
    let item_by_id = stage
        .items
        .iter()
        .map(|item| (item.id.clone(), item))
        .collect::<BTreeMap<_, _>>();
    let scores_by_title = stage
        .scores
        .iter()
        .map(|score| {
            let item = item_by_id.get(&score.item_id).unwrap();
            (item.title.as_str(), (item, score))
        })
        .collect::<BTreeMap<_, _>>();

    let (_, beta_score) = scores_by_title
        .get("Beta Agent MCP funding round from same feed")
        .expect("source-quota score");
    assert_eq!(beta_score.status, "source_quota");
    assert!(beta_score.reason.contains("source quota cap 1"));
    assert!(beta_score.tags.contains(&"source-quota".to_string()));

    let (_, zulu_score) = scores_by_title
        .get("Zulu Agent MCP benchmark results from Hacker News")
        .expect("category-quota score");
    assert_eq!(zulu_score.status, "category_quota");
    assert!(zulu_score.reason.contains("category quota cap 2"));
    assert!(zulu_score.tags.contains(&"category-quota".to_string()));
    assert!(
        zulu_score
            .tags
            .contains(&"category-quota-agent".to_string())
    );

    let selected = stage
        .scores
        .iter()
        .filter(|score| score.status == "selected")
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 3);
    let selected_agent_count = selected
        .iter()
        .filter(|score| score.tags.contains(&"agent".to_string()))
        .count();
    assert_eq!(selected_agent_count, 2);
    let selected_agent_feed_count = selected
        .iter()
        .filter(|score| {
            let item = item_by_id.get(&score.item_id).unwrap();
            item.metadata.get("source_detail").and_then(Value::as_str)
                == Some("https://example.com/agent-feed.xml")
        })
        .count();
    assert_eq!(selected_agent_feed_count, 1);
    for status in ["source_quota", "category_quota"] {
        let score = stage
            .scores
            .iter()
            .find(|score| score.status == status)
            .expect("quota-rejected score");
        let item = item_by_id.get(&score.item_id).unwrap();
        assert!(item.source_card_id.is_some(), "{item:?}");
        assert!(item.wiki_page_id.is_some(), "{item:?}");
    }

    let audit = store.audit_radar_run(&report.run.id).unwrap();
    assert!(audit.ok, "{audit:?}");
}

#[test]
fn severe_radar_category_quota_matches_projected_source_card_provider_family() {
    // CLAIM: category quotas can balance source-card projected items by
    // their real provider/source family, not just the generic `source_card`
    // item kind or hand-authored topic tags.
    // ORACLE: two providers with two eligible items each produce exactly
    // one selected item and one category_quota row per provider, with the
    // provider-specific category tag preserved.
    // SEVERITY: Severe because production-data balance proofs configure
    // provider-family quotas; ignoring provider/source metadata makes the
    // proof look broad while silently leaving families uncapped.
    let store = test_store("radar-provider-category-quota");
    for (title, url, source_type, provider, summary) in [
        (
            "Quartz provider balance launch",
            "https://example.com/rss-quartz",
            "article",
            "rss",
            "Quartz provider balance launch note for source-card quota proof.",
        ),
        (
            "Maple provider balance funding",
            "https://example.com/rss-maple",
            "article",
            "rss",
            "Maple provider balance funding note for source-card quota proof.",
        ),
        (
            "Lumen provider balance benchmark",
            "https://news.ycombinator.com/item?id=456",
            "discussion",
            "hackernews",
            "Lumen provider balance benchmark note for source-card quota proof.",
        ),
        (
            "Nimbus provider balance incident",
            "https://news.ycombinator.com/item?id=789",
            "discussion",
            "hackernews",
            "Nimbus provider balance incident note for source-card quota proof.",
        ),
    ] {
        store
            .add_source_card(SourceCardInput {
                title: title.to_string(),
                url: url.to_string(),
                source_type: source_type.to_string(),
                provider: provider.to_string(),
                summary: summary.to_string(),
                claims: vec![],
                retrieved_at: Some(now()),
                metadata: json!({
                    "source_detail": url,
                }),
            })
            .unwrap();
    }

    let profile = store
        .create_radar_profile(RadarProfileInput {
            name: "provider-family-balanced-radar".to_string(),
            description: "Provider family balanced radar".to_string(),
            window_hours: 24,
            min_score: 1.0,
            max_items: Some(10),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "provider balance" }]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({
                "balance": {
                    "category_quotas": {
                        "rss": 1,
                        "hackernews": 1
                    }
                }
            }),
        })
        .unwrap();

    let report = store.run_radar_profile(&profile.id, None).unwrap();
    assert_eq!(report.items_inserted, 4);
    assert_eq!(report.scores_inserted, 4);
    assert_eq!(report.selected_items, 2);

    let stage = store.read_radar_stage(&report.run.id).unwrap();
    let item_by_id = stage
        .items
        .iter()
        .map(|item| (item.id.clone(), item))
        .collect::<BTreeMap<_, _>>();
    let selected = stage
        .scores
        .iter()
        .filter(|score| score.status == "selected")
        .collect::<Vec<_>>();
    let category_quota = stage
        .scores
        .iter()
        .filter(|score| score.status == "category_quota")
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 2);
    assert_eq!(category_quota.len(), 2);
    assert!(
        stage
            .scores
            .iter()
            .all(|score| score.status != "source_quota"),
        "{:?}",
        stage.scores
    );

    let mut selected_by_provider = BTreeMap::<String, usize>::new();
    for score in &selected {
        let item = item_by_id.get(&score.item_id).unwrap();
        *selected_by_provider
            .entry(item.provider.clone())
            .or_default() += 1;
    }
    assert_eq!(selected_by_provider.get("rss"), Some(&1));
    assert_eq!(selected_by_provider.get("hackernews"), Some(&1));

    let mut rejected_by_provider = BTreeMap::<String, usize>::new();
    for score in &category_quota {
        let item = item_by_id.get(&score.item_id).unwrap();
        *rejected_by_provider
            .entry(item.provider.clone())
            .or_default() += 1;
        assert!(item.source_card_id.is_some(), "{item:?}");
        assert!(item.wiki_page_id.is_some(), "{item:?}");
        assert!(score.reason.contains("category quota cap 1"));
        assert!(score.tags.contains(&"category-quota".to_string()));
        assert!(
            score
                .tags
                .contains(&format!("category-quota-{}", item.provider))
        );
    }
    assert_eq!(rejected_by_provider.get("rss"), Some(&1));
    assert_eq!(rejected_by_provider.get("hackernews"), Some(&1));

    let audit = store.audit_radar_run(&report.run.id).unwrap();
    assert!(audit.ok, "{audit:?}");
}

#[test]
fn severe_radar_topic_dedupe_preserves_evidence_and_avoids_broad_drops() {
    // CLAIM: deterministic topic dedupe can suppress near-duplicate story
    // variants without deleting evidence or merging merely adjacent stories.
    // ORACLE: one near-duplicate receives `duplicate_topic`, the distinct
    // adjacent SDK story remains selected, all source-card/wiki provenance is
    // retained, and the dedupe group explains the shared-token basis.
    // SEVERITY: Severe because semantic dedupe is a classic fake-done trap:
    // silent drops or over-broad grouping both make a digest look cleaner than
    // the evidence supports.
    let store = test_store("radar-topic-dedupe");
    for (title, url, source_kind, source_detail, summary, topic) in [
        (
            "OpenAI releases Codex agent SDK",
            "https://example.com/openai-codex-agent-sdk",
            "rss",
            "https://example.com/openai.xml",
            "Launch notes for the Codex agent SDK with MCP worker support.",
            "codex-agent-sdk",
        ),
        (
            "Codex agent SDK release notes",
            "https://github.com/openai/codex-agent-sdk/releases/tag/v1",
            "github_release",
            "openai/codex-agent-sdk",
            "Release notes describe the same Codex agent SDK launch.",
            "codex-agent-sdk",
        ),
        (
            "Anthropic releases browser agent SDK",
            "https://example.com/anthropic-browser-agent-sdk",
            "rss",
            "https://example.com/anthropic.xml",
            "A different browser agent SDK release for another ecosystem.",
            "browser-agent-sdk",
        ),
    ] {
        store
            .add_source_card(SourceCardInput {
                title: title.to_string(),
                url: url.to_string(),
                source_type: source_kind.to_string(),
                provider: source_kind.to_string(),
                summary: summary.to_string(),
                claims: vec![],
                retrieved_at: Some(now()),
                metadata: json!({
                    "source_kind": source_kind,
                    "source_detail": source_detail,
                    "topic": topic,
                }),
            })
            .unwrap();
    }
    let profile = store
        .create_radar_profile(RadarProfileInput {
            name: "topic-dedupe-radar".to_string(),
            description: "Topic dedupe radar".to_string(),
            window_hours: 24,
            min_score: 2.0,
            max_items: Some(10),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "agent SDK" }]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({}),
        })
        .unwrap();

    let report = store.run_radar_profile(&profile.id, None).unwrap();
    assert_eq!(report.items_inserted, 3);
    assert_eq!(report.scores_inserted, 3);
    assert_eq!(report.selected_items, 2);

    let stage = store.read_radar_stage(&report.run.id).unwrap();
    assert_eq!(stage.dedup_groups.len(), 1, "{stage:?}");
    let group = &stage.dedup_groups[0];
    assert_eq!(group.dedup_kind, "semantic_topic");
    assert_eq!(group.member_item_ids.len(), 2);
    assert!(group.reason.contains("deterministic topic similarity"));
    assert!(group.reason.contains("codex"));
    assert!(group.reason.contains("agent"));
    assert!(group.reason.contains("sdk"));

    let item_by_id = stage
        .items
        .iter()
        .map(|item| (item.id.clone(), item))
        .collect::<BTreeMap<_, _>>();
    let scores_by_title = stage
        .scores
        .iter()
        .map(|score| {
            let item = item_by_id.get(&score.item_id).unwrap();
            (item.title.as_str(), (item, score))
        })
        .collect::<BTreeMap<_, _>>();
    let duplicate_scores = stage
        .scores
        .iter()
        .filter(|score| score.status == "duplicate_topic")
        .collect::<Vec<_>>();
    assert_eq!(duplicate_scores.len(), 1, "{:?}", stage.scores);
    let duplicate_score = duplicate_scores[0];
    assert!(duplicate_score.tags.contains(&"duplicate".to_string()));
    assert!(
        duplicate_score
            .tags
            .contains(&"semantic-dedupe".to_string())
    );
    assert!(!duplicate_score.tags.contains(&"exact-dedupe".to_string()));

    let (_, adjacent_score) = scores_by_title
        .get("Anthropic releases browser agent SDK")
        .expect("adjacent SDK story");
    assert_eq!(adjacent_score.status, "selected");
    assert!(!group.member_item_ids.contains(&adjacent_score.item_id));

    for score in &stage.scores {
        let item = item_by_id.get(&score.item_id).unwrap();
        assert!(item.source_card_id.is_some(), "{item:?}");
        assert!(item.wiki_page_id.is_some(), "{item:?}");
    }

    let quality = store.list_radar_source_quality(&report.run.id).unwrap();
    assert_eq!(quality.iter().map(|row| row.raw_count).sum::<i64>(), 3);
    assert_eq!(quality.iter().map(|row| row.accepted_count).sum::<i64>(), 2);
    assert_eq!(
        quality
            .iter()
            .map(|row| row.duplicate_rate.unwrap())
            .sum::<f64>(),
        1.0
    );
    let audit = store.audit_radar_run(&report.run.id).unwrap();
    assert!(audit.ok, "{audit:?}");

    store
        .conn
        .execute(
            "UPDATE radar_scores SET status = 'selected' WHERE run_id = ?1 AND item_id = ?2",
            params![report.run.id, duplicate_score.item_id],
        )
        .unwrap();
    let drift_audit = store.audit_radar_run(&report.run.id).unwrap();
    assert!(!drift_audit.ok);
    assert!(drift_audit.findings.iter().any(|finding| {
        finding.code == "radar_dedup_score_drift" && finding.severity == "high"
    }));
}

#[test]
fn severe_radar_semantic_topic_dedupe_preserves_evidence_and_separates_events() {
    // CLAIM: deterministic topic dedupe runs after initial scoring, suppresses
    // obvious same-story repeats, and does not collapse same-product but
    // different-event items.
    // ORACLE: a semantic_topic dedupe group preserves both member items,
    // the duplicate keeps a score row with duplicate_topic status, and the
    // distinct funding item remains selected.
    // SEVERITY: Severe because semantic dedupe can become a quiet evidence-loss
    // mirage if duplicate rows disappear or broad token overlap collapses
    // different events.
    let store = test_store("radar-semantic-topic-dedupe");
    for (title, url, provider, summary) in [
        (
            "Acme Agent MCP release improves worker reliability",
            "https://example.com/acme-agent-mcp-release",
            "rss",
            "Acme Agent MCP release improves worker reliability for queues.",
        ),
        (
            "Acme Agent MCP release improves worker reliability analysis",
            "https://news.ycombinator.com/item?id=987",
            "hackernews",
            "Community analysis of the Acme Agent MCP release reliability improvements.",
        ),
        (
            "Acme Agent MCP funding round",
            "https://example.com/acme-agent-mcp-funding",
            "rss",
            "Acme Agent MCP funding round expands the company.",
        ),
    ] {
        store
            .add_source_card(SourceCardInput {
                title: title.to_string(),
                url: url.to_string(),
                source_type: provider.to_string(),
                provider: provider.to_string(),
                summary: summary.to_string(),
                claims: vec![],
                retrieved_at: Some(now()),
                metadata: json!({
                    "source_kind": provider,
                    "source_detail": url,
                }),
            })
            .unwrap();
    }
    let profile = store
        .create_radar_profile(RadarProfileInput {
            name: "semantic-topic-radar".to_string(),
            description: "Semantic topic radar".to_string(),
            window_hours: 24,
            min_score: 2.0,
            max_items: Some(10),
            languages: vec!["en".to_string()],
            source_selectors: json!([{ "kind": "source_card_query", "query": "Acme Agent MCP" }]),
            delivery_policy: json!({ "delivery": "manual_only" }),
            model_policy: json!({ "model_scoring": "disabled" }),
            metadata: json!({}),
        })
        .unwrap();

    let report = store.run_radar_profile(&profile.id, None).unwrap();
    assert_eq!(report.items_inserted, 3);
    assert_eq!(report.scores_inserted, 3);
    assert_eq!(report.selected_items, 2);
    assert_eq!(
        report
            .run
            .metadata
            .get("semantic_dedup_groups")
            .and_then(Value::as_u64),
        Some(1)
    );

    let stage = store.read_radar_stage(&report.run.id).unwrap();
    let semantic_groups = stage
        .dedup_groups
        .iter()
        .filter(|group| group.dedup_kind == "semantic_topic")
        .collect::<Vec<_>>();
    assert_eq!(semantic_groups.len(), 1, "{:?}", stage.dedup_groups);
    assert_eq!(semantic_groups[0].member_item_ids.len(), 2);
    assert!(
        semantic_groups[0]
            .reason
            .contains("deterministic topic similarity")
    );
    assert!(semantic_groups[0].model_provider.is_none());

    let item_by_id = stage
        .items
        .iter()
        .map(|item| (item.id.clone(), item))
        .collect::<BTreeMap<_, _>>();
    let scores_by_title = stage
        .scores
        .iter()
        .map(|score| {
            let item = item_by_id.get(&score.item_id).unwrap();
            (item.title.as_str(), (item, score))
        })
        .collect::<BTreeMap<_, _>>();
    let (_, analysis_score) = scores_by_title
        .get("Acme Agent MCP release improves worker reliability analysis")
        .expect("analysis duplicate score");
    assert_eq!(analysis_score.status, "duplicate_topic");
    assert!(analysis_score.tags.contains(&"semantic-dedupe".to_string()));
    assert!(analysis_score.reason.contains("duplicate suppressed"));

    let (funding_item, funding_score) = scores_by_title
        .get("Acme Agent MCP funding round")
        .expect("funding score");
    assert_eq!(funding_score.status, "selected");
    assert!(
        !semantic_groups[0]
            .member_item_ids
            .contains(&funding_item.id),
        "{semantic_groups:?}"
    );
    for item_id in &semantic_groups[0].member_item_ids {
        let item = item_by_id.get(item_id).unwrap();
        assert!(item.source_card_id.is_some(), "{item:?}");
        assert!(item.wiki_page_id.is_some(), "{item:?}");
    }

    let quality = store.list_radar_source_quality(&report.run.id).unwrap();
    assert_eq!(quality.len(), 3, "{quality:?}");
    assert!(
        quality
            .iter()
            .any(|row| row.duplicate_rate == Some(1.0) && row.accepted_count == 0),
        "{quality:?}"
    );
    let audit = store.audit_radar_run(&report.run.id).unwrap();
    assert!(audit.ok, "{audit:?}");
}
