use super::*;

#[test]
fn research_editorial_evidence_pack_and_eval_gate_round_trip() {
    let store = test_store("research-editorial-gate");
    let workflow = store
        .create_deep_research_run("safe cloud execution")
        .unwrap();
    let evidence = store
        .build_research_evidence_pack(&workflow.run.id)
        .unwrap();
    assert_eq!(evidence.artifact_type, "evidence_pack");

    let draft = store
        .record_research_artifact(ResearchArtifactInput {
            run_id: workflow.run.id.clone(),
            role_run_id: None,
            artifact_type: "generated_synthesis".to_string(),
            title: "Analyst draft".to_string(),
            body: "Cloud execution platforms need compile-time and runtime controls [claim:1]."
                .to_string(),
            metadata: json!({ "draft_version": 1 }),
        })
        .unwrap();
    let draft_run = store
        .record_research_editorial_run(ResearchEditorialRunInput {
            run_id: workflow.run.id.clone(),
            stage: "editorial_drafter".to_string(),
            model_provider: "openai".to_string(),
            model_name: "gpt-5".to_string(),
            prompt_version: "deep-editor-v1".to_string(),
            input_artifact_id: Some(evidence.id.clone()),
            output_artifact_id: Some(draft.id.clone()),
            cost_decision_id: None,
            status: "completed".to_string(),
            score: json!({ "draft_sections": 1 }),
            error_message: None,
        })
        .unwrap();
    assert_eq!(
        draft_run.input_artifact_hash.as_deref(),
        Some(evidence.body_sha256.as_str())
    );

    let missing_gate_audit = store.audit_research_run(&workflow.run.id).unwrap();
    assert!(
        missing_gate_audit
            .audit
            .findings
            .iter()
            .any(|finding| finding.code == "missing_citation_verifier")
    );
    assert!(
        missing_gate_audit
            .audit
            .findings
            .iter()
            .any(|finding| finding.code == "missing_adversarial_evaluator")
    );

    let verified = store
        .record_research_artifact(ResearchArtifactInput {
            run_id: workflow.run.id.clone(),
            role_run_id: None,
            artifact_type: "citation_verified_draft".to_string(),
            title: "Citation-verified draft".to_string(),
            body: "Verifier found all factual sentences cited.".to_string(),
            metadata: json!({ "verifier": "citation" }),
        })
        .unwrap();
    let verifier_run = store
        .record_research_editorial_run(ResearchEditorialRunInput {
            run_id: workflow.run.id.clone(),
            stage: "citation_verifier".to_string(),
            model_provider: "openai".to_string(),
            model_name: "gpt-5".to_string(),
            prompt_version: "citation-verifier-v1".to_string(),
            input_artifact_id: Some(draft.id.clone()),
            output_artifact_id: Some(verified.id.clone()),
            cost_decision_id: None,
            status: "completed".to_string(),
            score: json!({
                "unsupported_factual_sentence_rate": 0.0,
                "unsupported_factual_sentences": 0,
                "valid_citations": true
            }),
            error_message: None,
        })
        .unwrap();
    let mut live_shape_verifier = verifier_run.clone();
    live_shape_verifier.score = json!({
        "unsupported_count": 0,
        "unsupported_rate": 0.0,
        "valid_citations": 237
    });
    assert!(
        audit_citation_verifier_score(&live_shape_verifier).is_empty(),
        "live provider score aliases must satisfy the citation gate"
    );
    let evaluated = store
        .record_research_artifact(ResearchArtifactInput {
            run_id: workflow.run.id.clone(),
            role_run_id: None,
            artifact_type: "adversarial_eval_report".to_string(),
            title: "Adversarial evaluator report".to_string(),
            body: "Accepted for analyst use.".to_string(),
            metadata: json!({ "gate": "analyst_grade" }),
        })
        .unwrap();
    store
        .record_research_editorial_run(ResearchEditorialRunInput {
            run_id: workflow.run.id.clone(),
            stage: "adversarial_evaluator".to_string(),
            model_provider: "openai".to_string(),
            model_name: "gpt-5".to_string(),
            prompt_version: "adversarial-eval-v1".to_string(),
            input_artifact_id: Some(verified.id.clone()),
            output_artifact_id: Some(evaluated.id.clone()),
            cost_decision_id: None,
            status: "accepted".to_string(),
            score: json!({ "passed": true, "score": 0.91 }),
            error_message: None,
        })
        .unwrap();

    let read = store.read_research_run(&workflow.run.id).unwrap();
    assert_eq!(read.editorial_runs.len(), 3);
    let clean_gate_audit = store.audit_research_run(&workflow.run.id).unwrap();
    assert!(
        clean_gate_audit
            .audit
            .findings
            .iter()
            .all(|finding| finding.code != "missing_citation_verifier")
    );
    assert!(
        clean_gate_audit
            .audit
            .findings
            .iter()
            .all(|finding| finding.code != "missing_adversarial_evaluator")
    );
    assert!(
        clean_gate_audit
            .audit
            .findings
            .iter()
            .all(|finding| finding.code != "unsupported_factual_sentence_rate")
    );
}

#[test]
fn research_editorial_mock_invoke_records_output_and_passes_eval_gates() {
    let store = test_store("research-editorial-invoke-mock");
    let workflow = store
        .create_deep_research_run("safe cloud execution")
        .unwrap();

    let draft = store
        .invoke_research_editorial(ResearchEditorialInvokeInput {
            run_id: workflow.run.id.clone(),
            stage: "editorial_drafter".to_string(),
            model_provider: "mock".to_string(),
            model_name: None,
            prompt_version: "test-editor-v1".to_string(),
            input_artifact_id: None,
            endpoint: None,
            api_key: None,
            timeout_seconds: None,
        })
        .unwrap();
    assert_eq!(draft.editorial_run.status, "completed");
    assert_eq!(draft.editorial_run.stage, "editorial_drafter");
    let draft_artifact = draft.output_artifact.as_ref().unwrap();
    assert_eq!(draft_artifact.artifact_type, "generated_synthesis");

    let verified = store
        .invoke_research_editorial(ResearchEditorialInvokeInput {
            run_id: workflow.run.id.clone(),
            stage: "citation_verifier".to_string(),
            model_provider: "mock".to_string(),
            model_name: None,
            prompt_version: "test-verifier-v1".to_string(),
            input_artifact_id: Some(draft_artifact.id.clone()),
            endpoint: None,
            api_key: None,
            timeout_seconds: None,
        })
        .unwrap();
    assert_eq!(
        verified.editorial_run.output_artifact_id,
        verified
            .output_artifact
            .as_ref()
            .map(|artifact| artifact.id.clone())
    );

    let verified_artifact_id = verified.output_artifact.as_ref().unwrap().id.clone();
    store
        .invoke_research_editorial(ResearchEditorialInvokeInput {
            run_id: workflow.run.id.clone(),
            stage: "adversarial_evaluator".to_string(),
            model_provider: "mock".to_string(),
            model_name: None,
            prompt_version: "test-eval-v1".to_string(),
            input_artifact_id: Some(verified_artifact_id),
            endpoint: None,
            api_key: None,
            timeout_seconds: None,
        })
        .unwrap();

    let read = store.read_research_run(&workflow.run.id).unwrap();
    assert_eq!(read.editorial_runs.len(), 3);
    let audit = store.audit_research_run(&workflow.run.id).unwrap();
    assert!(
        audit
            .audit
            .findings
            .iter()
            .all(|finding| finding.code != "missing_citation_verifier")
    );
    assert!(
        audit
            .audit
            .findings
            .iter()
            .all(|finding| finding.code != "missing_adversarial_evaluator")
    );
}

#[test]
fn severe_research_editorial_live_provider_invocation_fails_closed_on_malformed_output() {
    // CLAIM: live provider invocation records an explicit failed editorial run
    // when the provider returns prose or malformed JSON instead of the required contract.
    // ORACLE: failed run, no output artifact, redacted/inspectable provider response.
    // SEVERITY: Severe because prose-only evals would otherwise launder unsupported reports.
    let store = test_store("research-editorial-invoke-severe");
    let workflow = store
        .create_deep_research_run("safe cloud execution")
        .unwrap();
    let endpoint = mock_base_server(
        r#"{"output_text":"not editorial json"}"#,
        "application/json",
    );

    let invocation = store
        .invoke_research_editorial(ResearchEditorialInvokeInput {
            run_id: workflow.run.id.clone(),
            stage: "editorial_drafter".to_string(),
            model_provider: "openai".to_string(),
            model_name: Some("gpt-test".to_string()),
            prompt_version: "test-live-v1".to_string(),
            input_artifact_id: None,
            endpoint: Some(endpoint),
            api_key: Some("test-key".to_string()),
            timeout_seconds: Some(2),
        })
        .unwrap();
    assert_eq!(invocation.editorial_run.status, "failed");
    assert!(invocation.output_artifact.is_none());
    assert!(invocation.editorial_run.error_message_redacted.is_some());
    assert!(
        invocation
            .provider_response
            .get("output_text")
            .and_then(Value::as_str)
            .is_some()
    );

    let read = store.read_research_run(&workflow.run.id).unwrap();
    assert_eq!(read.editorial_runs.len(), 1);
    assert!(read.artifacts.iter().all(|artifact| {
        artifact.artifact_type != "generated_synthesis"
            && artifact.artifact_type != "citation_verified_draft"
    }));
}

#[test]
fn severe_research_editorial_bodyless_completed_provider_output_is_recorded() {
    // CLAIM: bodyless completed provider output with structured scores must not
    // disappear as a thrown exception.
    // ORACLE: completed run is recorded with a synthesized score artifact.
    // SEVERITY: Severe because live verifier/evaluator failures are part of the audit trail.
    let store = test_store("research-editorial-bodyless-completed");
    let workflow = store
        .create_deep_research_run("bodyless editorial verifier")
        .unwrap();
    let endpoint = mock_base_server(
        r##"{
                "output": [{
                    "type": "message",
                    "content": [{
                        "type": "output_text",
                        "text": "{\"status\":\"completed\",\"body\":null,\"score\":{\"unsupported_count\":7,\"unsupported_rate\":0.5,\"valid_citations\":3},\"error_message\":null}"
                    }]
                }]
            }"##,
        "application/json",
    );

    let invocation = store
        .invoke_research_editorial(ResearchEditorialInvokeInput {
            run_id: workflow.run.id.clone(),
            stage: "citation_verifier".to_string(),
            model_provider: "openai".to_string(),
            model_name: Some("gpt-test".to_string()),
            prompt_version: "test-bodyless-v1".to_string(),
            input_artifact_id: None,
            endpoint: Some(endpoint),
            api_key: Some("test-key".to_string()),
            timeout_seconds: Some(2),
        })
        .unwrap();
    assert_eq!(invocation.editorial_run.status, "completed");
    assert!(invocation.output_artifact.is_some());
    assert!(invocation.editorial_run.output_artifact_id.is_some());
    assert_eq!(
        invocation.editorial_run.score["unsupported_count"].as_i64(),
        Some(7)
    );
    assert!(
        invocation
            .output_artifact
            .as_ref()
            .map(|artifact| artifact.body.contains("structured editorial score"))
            .unwrap_or(false)
    );
}

#[test]
fn research_editorial_live_provider_invocation_parses_responses_api_envelope() {
    // CLAIM: OpenAI Responses API transport status must not be mistaken for
    // the editorial contract status.
    // ORACLE: nested output message JSON becomes a completed editorial run
    // with an inspectable output artifact.
    let store = test_store("research-editorial-responses-envelope");
    let workflow = store
        .create_deep_research_run("document anchored research")
        .unwrap();
    let endpoint = mock_base_server(
        r##"{
                "id": "resp_test",
                "object": "response",
                "status": "completed",
                "output": [
                    {
                        "type": "message",
                        "status": "completed",
                        "role": "assistant",
                        "content": [
                            {
                                "type": "output_text",
                                "text": "{\"status\":\"completed\",\"body\":\"# Analyst Draft\\n\\nBounded by cited evidence.\",\"score\":{\"source_bound\":true},\"error_message\":null}"
                            }
                        ]
                    }
                ]
            }"##,
        "application/json",
    );

    let invocation = store
        .invoke_research_editorial(ResearchEditorialInvokeInput {
            run_id: workflow.run.id.clone(),
            stage: "editorial_drafter".to_string(),
            model_provider: "openai".to_string(),
            model_name: Some("gpt-test".to_string()),
            prompt_version: "test-live-v1".to_string(),
            input_artifact_id: None,
            endpoint: Some(endpoint),
            api_key: Some("test-key".to_string()),
            timeout_seconds: Some(2),
        })
        .unwrap();
    assert_eq!(invocation.editorial_run.status, "completed");
    assert_eq!(
        invocation.output_artifact.as_ref().unwrap().artifact_type,
        "generated_synthesis"
    );
    assert!(invocation.output_artifact.unwrap().body.contains("Bounded"));
}

#[test]
fn severe_research_editorial_runs_reject_unsupported_or_cross_run_state() {
    // CLAIM: model-backed editorial/eval runs must be inspectable, run-scoped, and
    // auditable against structured acceptance criteria rather than trusted as prose.
    // ORACLE: invalid stages, cross-run artifacts, missing outputs, rejected runs without
    // errors, secret metadata, unsupported citation scores, and low eval scores fail closed.
    // SEVERITY: Severe because editorial polish can otherwise launder unsupported claims.
    let store = test_store("research-editorial-severe");
    let left = store.create_deep_research_run("London AI").unwrap();
    let right = store.create_deep_research_run("image compression").unwrap();
    let evidence = store.build_research_evidence_pack(&left.run.id).unwrap();
    let draft = store
        .record_research_artifact(ResearchArtifactInput {
            run_id: left.run.id.clone(),
            role_run_id: None,
            artifact_type: "generated_synthesis".to_string(),
            title: "Draft".to_string(),
            body: "Draft body".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    let wrong_run_artifact = store
        .record_research_artifact(ResearchArtifactInput {
            run_id: right.run.id.clone(),
            role_run_id: None,
            artifact_type: "generated_synthesis".to_string(),
            title: "Wrong run".to_string(),
            body: "Wrong body".to_string(),
            metadata: json!({}),
        })
        .unwrap();

    assert!(
        store
            .record_research_editorial_run(ResearchEditorialRunInput {
                run_id: left.run.id.clone(),
                stage: "style_polisher".to_string(),
                model_provider: "openai".to_string(),
                model_name: "gpt-5".to_string(),
                prompt_version: "v1".to_string(),
                input_artifact_id: Some(evidence.id.clone()),
                output_artifact_id: Some(draft.id.clone()),
                cost_decision_id: None,
                status: "completed".to_string(),
                score: json!({}),
                error_message: None,
            })
            .is_err()
    );
    assert!(
        store
            .record_research_editorial_run(ResearchEditorialRunInput {
                run_id: left.run.id.clone(),
                stage: "editorial_drafter".to_string(),
                model_provider: "openai".to_string(),
                model_name: "gpt-5".to_string(),
                prompt_version: "v1".to_string(),
                input_artifact_id: Some(evidence.id.clone()),
                output_artifact_id: None,
                cost_decision_id: None,
                status: "completed".to_string(),
                score: json!({}),
                error_message: None,
            })
            .is_err()
    );
    assert!(
        store
            .record_research_editorial_run(ResearchEditorialRunInput {
                run_id: left.run.id.clone(),
                stage: "editorial_drafter".to_string(),
                model_provider: "openai".to_string(),
                model_name: "gpt-5".to_string(),
                prompt_version: "v1".to_string(),
                input_artifact_id: Some(evidence.id.clone()),
                output_artifact_id: Some(wrong_run_artifact.id.clone()),
                cost_decision_id: None,
                status: "completed".to_string(),
                score: json!({}),
                error_message: None,
            })
            .is_err()
    );
    assert!(
        store
            .record_research_editorial_run(ResearchEditorialRunInput {
                run_id: left.run.id.clone(),
                stage: "citation_verifier".to_string(),
                model_provider: "openai".to_string(),
                model_name: "gpt-5".to_string(),
                prompt_version: "v1".to_string(),
                input_artifact_id: Some(draft.id.clone()),
                output_artifact_id: None,
                cost_decision_id: None,
                status: "rejected".to_string(),
                score: json!({}),
                error_message: None,
            })
            .is_err()
    );

    let bad_draft_run = store
        .record_research_editorial_run(ResearchEditorialRunInput {
            run_id: left.run.id.clone(),
            stage: "editorial_drafter".to_string(),
            model_provider: "openai".to_string(),
            model_name: "gpt-5".to_string(),
            prompt_version: "v1".to_string(),
            input_artifact_id: Some(evidence.id.clone()),
            output_artifact_id: Some(draft.id.clone()),
            cost_decision_id: None,
            status: "completed".to_string(),
            score: json!({ "api_key": "sk-secret-in-score" }),
            error_message: None,
        })
        .unwrap();
    assert_eq!(
        bad_draft_run.score.get("api_key").and_then(Value::as_str),
        Some("[REDACTED]")
    );
    let rejected_report = store
        .record_research_artifact(ResearchArtifactInput {
            run_id: left.run.id.clone(),
            role_run_id: None,
            artifact_type: "citation_rejection".to_string(),
            title: "Citation rejection".to_string(),
            body: "Unsupported claims found.".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    let rejected = store
        .record_research_editorial_run(ResearchEditorialRunInput {
            run_id: left.run.id.clone(),
            stage: "citation_verifier".to_string(),
            model_provider: "openai".to_string(),
            model_name: "gpt-5".to_string(),
            prompt_version: "v1".to_string(),
            input_artifact_id: Some(draft.id.clone()),
            output_artifact_id: Some(rejected_report.id.clone()),
            cost_decision_id: None,
            status: "rejected".to_string(),
            score: json!({ "valid_citations": false }),
            error_message: Some(
                "provider returned Authorization: Bearer sk-error-secret".to_string(),
            ),
        })
        .unwrap();
    assert!(
        rejected
            .error_message_redacted
            .as_deref()
            .unwrap()
            .contains("[REDACTED]")
    );

    let bad_verified = store
        .record_research_artifact(ResearchArtifactInput {
            run_id: left.run.id.clone(),
            role_run_id: None,
            artifact_type: "bad_verified_draft".to_string(),
            title: "Bad verified draft".to_string(),
            body: "Verifier report with unsupported claims.".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    let bad_verifier_run = store
        .record_research_editorial_run(ResearchEditorialRunInput {
            run_id: left.run.id.clone(),
            stage: "citation_verifier".to_string(),
            model_provider: "openai".to_string(),
            model_name: "gpt-5".to_string(),
            prompt_version: "v1".to_string(),
            input_artifact_id: Some(draft.id.clone()),
            output_artifact_id: Some(bad_verified.id.clone()),
            cost_decision_id: None,
            status: "completed".to_string(),
            score: json!({
                "unsupported_factual_sentence_rate": 0.2,
                "valid_citations": false
            }),
            error_message: None,
        })
        .unwrap();
    assert_eq!(
        bad_verifier_run
            .score
            .get("unsupported_factual_sentence_rate")
            .and_then(Value::as_f64),
        Some(0.2)
    );
    let bad_eval = store
        .record_research_artifact(ResearchArtifactInput {
            run_id: left.run.id.clone(),
            role_run_id: None,
            artifact_type: "bad_eval_report".to_string(),
            title: "Bad evaluator report".to_string(),
            body: "Rejected below gate.".to_string(),
            metadata: json!({}),
        })
        .unwrap();
    store
        .record_research_editorial_run(ResearchEditorialRunInput {
            run_id: left.run.id.clone(),
            stage: "adversarial_evaluator".to_string(),
            model_provider: "openai".to_string(),
            model_name: "gpt-5".to_string(),
            prompt_version: "v1".to_string(),
            input_artifact_id: Some(bad_verified.id.clone()),
            output_artifact_id: Some(bad_eval.id.clone()),
            cost_decision_id: None,
            status: "completed".to_string(),
            score: json!({ "passed": false, "score": 0.3 }),
            error_message: None,
        })
        .unwrap();

    let audit = store.audit_research_run(&left.run.id).unwrap();
    let codes: BTreeSet<&str> = audit
        .audit
        .findings
        .iter()
        .map(|finding| finding.code.as_str())
        .collect();
    assert!(
        codes.contains("unsupported_factual_sentence_rate"),
        "audit codes: {codes:?}"
    );
    assert!(
        codes.contains("invalid_editorial_citations"),
        "audit codes: {codes:?}"
    );
    assert!(
        codes.contains("editorial_evaluator_rejected"),
        "audit codes: {codes:?}"
    );
    assert!(
        codes.contains("editorial_evaluator_score_below_gate"),
        "audit codes: {codes:?}"
    );
    assert!(
        codes.contains("editorial_stage_failed_or_rejected"),
        "audit codes: {codes:?}"
    );
}

#[test]
fn severe_web_search_rejects_host_native_inside_daemon() {
    let store = test_store("web-host-native");
    let error = store
        .web_search(
            "current agent news",
            WebSearchConfig {
                provider: "host".to_string(),
                max_results: 5,
                endpoint: None,
                api_key: None,
                model: None,
                timeout_seconds: 2,
            },
        )
        .expect_err("host-native search must not pretend to run in daemon");
    assert!(error.to_string().contains("host-native search must be run"));
}

#[test]
fn severe_web_search_rejects_non_https_non_loopback_endpoint() {
    let store = test_store("web-endpoint");
    let error = store
        .web_search(
            "current agent news",
            WebSearchConfig {
                provider: "brave".to_string(),
                max_results: 5,
                endpoint: Some("http://example.com/search".to_string()),
                api_key: Some("test-key".to_string()),
                model: None,
                timeout_seconds: 2,
            },
        )
        .expect_err("non-loopback http endpoints must be rejected");
    assert!(error.to_string().contains("endpoint must use https"));
}

#[test]
fn severe_web_search_rejects_custom_https_endpoint_without_override() {
    let store = test_store("web-custom-endpoint");
    let error = store
        .web_search(
            "current agent news",
            WebSearchConfig {
                provider: "brave".to_string(),
                max_results: 5,
                endpoint: Some("https://attacker.example/search".to_string()),
                api_key: Some("test-key".to_string()),
                model: None,
                timeout_seconds: 2,
            },
        )
        .expect_err("custom non-loopback endpoints must be rejected by default");
    assert!(
        error
            .to_string()
            .contains("custom non-loopback search endpoints are disabled")
    );
}

#[test]
fn severe_brave_search_skips_unsafe_result_urls_and_writes_source_card() {
    let store = test_store("web-brave");
    let endpoint = mock_json_server(
        r#"{
              "web": {
                "results": [
                  {
                    "title": "Good Source",
                    "url": "https://example.com/good",
                    "description": "Useful source text."
                  },
                  {
                    "title": "Bad Source",
                    "url": "javascript:alert(1)",
                    "description": "Must not become a markdown link."
                  }
                ]
              }
            }"#,
    );
    let (response, page_id) = store
        .web_search_to_wiki(
            "agent monitors",
            WebSearchConfig {
                provider: "brave".to_string(),
                max_results: 5,
                endpoint: Some(endpoint),
                api_key: Some("test-key".to_string()),
                model: None,
                timeout_seconds: 2,
            },
        )
        .unwrap();

    assert_eq!(response.results.len(), 1);
    assert_eq!(response.results[0].url, "https://example.com/good");
    let page = store.read_wiki_page(&page_id).unwrap().unwrap();
    assert!(page.content.contains("Good Source"));
    assert!(!page.content.contains("javascript:alert"));
}

#[test]
fn openai_citation_collection_finds_nested_url_annotations() {
    let value = json!({
        "output": [
            {
                "content": [
                    {
                        "annotations": [
                            {
                                "type": "url_citation",
                                "url": "https://example.com/source",
                                "title": "Source"
                            }
                        ]
                    }
                ]
            }
        ]
    });
    let citations = collect_url_citations(&value);
    assert_eq!(citations.len(), 1);
    assert_eq!(citations[0].url, "https://example.com/source");
    assert_eq!(citations[0].title.as_deref(), Some("Source"));
}
