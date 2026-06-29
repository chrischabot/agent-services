use super::*;

pub(crate) fn required_string(arguments: &Value, key: &str) -> Result<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .with_context(|| format!("missing string argument: {key}"))
}

pub(crate) fn parse_json_arg(raw: &str, label: &str) -> Result<Value> {
    serde_json::from_str(raw).with_context(|| format!("parsing {label}"))
}

pub(crate) fn parse_json_arg_or_file(raw: &str, path: Option<&PathBuf>) -> Result<Value> {
    match path {
        Some(path) => {
            let text = fs::read_to_string(path)
                .with_context(|| format!("reading JSON from {}", path.display()))?;
            parse_json_arg(&text, &path.display().to_string())
        }
        None => parse_json_arg(raw, "--source-snapshots-json"),
    }
}

pub(crate) fn optional_string(arguments: &Value, key: &str, default: &str) -> String {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

pub(crate) fn provider_list_from_mcp(arguments: &Value) -> Vec<String> {
    if let Some(values) = arguments.get("providers").and_then(Value::as_array) {
        return values
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect();
    }
    arguments
        .get("providers")
        .and_then(Value::as_str)
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn optional_inline_or_file(
    inline: Option<String>,
    path: Option<PathBuf>,
) -> Result<Option<String>> {
    match (inline, path) {
        (Some(_), Some(path)) => bail!(
            "provide either inline text or file path, not both: {}",
            path.display()
        ),
        (Some(value), None) => Ok(Some(value)),
        (None, Some(path)) => fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))
            .map(Some),
        (None, None) => Ok(None),
    }
}

pub(crate) fn optional_bool(arguments: &Value, key: &str, default: bool) -> bool {
    arguments
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

pub(crate) fn optional_usize(arguments: &Value, key: &str, default: usize) -> usize {
    arguments
        .get(key)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(default)
}

pub(crate) fn optional_usize_arg(arguments: &Value, key: &str) -> Option<usize> {
    arguments
        .get(key)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
}

pub(crate) fn optional_i64_arg(arguments: &Value, key: &str) -> Option<i64> {
    arguments.get(key).and_then(Value::as_i64)
}

pub(crate) fn optional_f64_arg(arguments: &Value, key: &str) -> Option<f64> {
    arguments.get(key).and_then(Value::as_f64)
}

pub(crate) fn required_f64_arg(arguments: &Value, key: &str) -> Result<f64> {
    arguments
        .get(key)
        .and_then(Value::as_f64)
        .with_context(|| format!("missing numeric argument: {key}"))
}

pub(crate) fn optional_bool_arg(arguments: &Value, key: &str) -> Option<bool> {
    arguments.get(key).and_then(Value::as_bool)
}

pub(crate) fn research_convergence_step_input_from_mcp(
    arguments: &Value,
) -> Result<ResearchConvergenceStepInput> {
    Ok(ResearchConvergenceStepInput {
        run_id: required_string(arguments, "run_id")?,
        max_iterations: optional_usize_arg(arguments, "max_iterations"),
        max_seconds: optional_i64_arg(arguments, "max_seconds"),
        max_sources: optional_usize_arg(arguments, "max_sources"),
        max_provider_calls: optional_usize_arg(arguments, "max_provider_calls"),
        cost_cap_usd: optional_f64_arg(arguments, "cost_cap_usd"),
        source_novelty_threshold: optional_f64_arg(arguments, "source_novelty_threshold"),
        confidence_delta_threshold: optional_f64_arg(arguments, "confidence_delta_threshold"),
        no_progress_iteration_limit: optional_usize_arg(arguments, "no_progress_iteration_limit"),
        require_active_fact_check: optional_bool_arg(arguments, "require_active_fact_check"),
        allow_long_run: optional_bool_arg(arguments, "allow_long_run"),
        no_write: optional_bool_arg(arguments, "no_write"),
        editorial_provider: arguments
            .get("editorial_provider")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        editorial_model_name: arguments
            .get("editorial_model_name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        editorial_endpoint: arguments
            .get("editorial_endpoint")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        editorial_timeout_seconds: arguments
            .get("editorial_timeout_seconds")
            .and_then(Value::as_u64),
    })
}

pub(crate) fn research_convergence_start_input_from_mcp(
    arguments: &Value,
) -> Result<ResearchConvergenceStartInput> {
    Ok(ResearchConvergenceStartInput {
        run_id: required_string(arguments, "run_id")?,
        max_iterations: optional_usize_arg(arguments, "max_iterations"),
        max_seconds: optional_i64_arg(arguments, "max_seconds"),
        max_sources: optional_usize_arg(arguments, "max_sources"),
        max_provider_calls: optional_usize_arg(arguments, "max_provider_calls"),
        cost_cap_usd: optional_f64_arg(arguments, "cost_cap_usd"),
        source_novelty_threshold: optional_f64_arg(arguments, "source_novelty_threshold"),
        confidence_delta_threshold: optional_f64_arg(arguments, "confidence_delta_threshold"),
        no_progress_iteration_limit: optional_usize_arg(arguments, "no_progress_iteration_limit"),
        require_active_fact_check: optional_bool_arg(arguments, "require_active_fact_check"),
        allow_long_run: optional_bool_arg(arguments, "allow_long_run"),
        no_write: optional_bool_arg(arguments, "no_write"),
        editorial_provider: arguments
            .get("editorial_provider")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        editorial_model_name: arguments
            .get("editorial_model_name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        editorial_endpoint: arguments
            .get("editorial_endpoint")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        editorial_timeout_seconds: arguments
            .get("editorial_timeout_seconds")
            .and_then(Value::as_u64),
    })
}

pub(crate) fn research_convergence_provider_search_input_from_mcp(
    arguments: &Value,
) -> Result<ResearchConvergenceProviderSearchInput> {
    Ok(ResearchConvergenceProviderSearchInput {
        run_id: required_string(arguments, "run_id")?,
        provider: required_string(arguments, "provider")?,
        max_tasks: optional_usize_arg(arguments, "max_tasks"),
        max_results: optional_usize_arg(arguments, "max_results"),
        max_provider_calls: optional_usize_arg(arguments, "max_provider_calls"),
        enqueue_selected_url_ingest: arguments
            .get("enqueue_selected_url_ingest")
            .and_then(Value::as_bool),
        max_ingest_jobs: optional_usize_arg(arguments, "max_ingest_jobs"),
        cost_cap_usd: optional_f64_arg(arguments, "cost_cap_usd"),
        endpoint: arguments
            .get("endpoint")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        api_key: arguments
            .get("api_key")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        model: arguments
            .get("model")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        timeout_seconds: arguments.get("timeout_seconds").and_then(Value::as_u64),
    })
}

pub(crate) fn research_active_fact_check_input_from_mcp(
    arguments: &Value,
) -> Result<ResearchActiveFactCheckInput> {
    Ok(ResearchActiveFactCheckInput {
        run_id: required_string(arguments, "run_id")?,
        artifact_id: arguments
            .get("artifact_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        max_sentences: optional_usize_arg(arguments, "max_sentences"),
        create_challenges: optional_bool_arg(arguments, "create_challenges"),
    })
}

pub(crate) fn research_convergence_close_loop_input_from_mcp(
    arguments: &Value,
) -> Result<ResearchConvergenceCloseLoopInput> {
    Ok(ResearchConvergenceCloseLoopInput {
        run_id: required_string(arguments, "run_id")?,
        artifact_id: arguments
            .get("artifact_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        max_sentences: optional_usize_arg(arguments, "max_sentences"),
        create_challenges: optional_bool_arg(arguments, "create_challenges"),
        compile_report_before_check: optional_bool_arg(arguments, "compile_report_before_check"),
        rerun_after_check: optional_bool_arg(arguments, "rerun_after_check"),
        compile_final_report: optional_bool_arg(arguments, "compile_final_report"),
        provider: arguments
            .get("provider")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        provider_max_tasks: optional_usize_arg(arguments, "provider_max_tasks"),
        provider_max_results: optional_usize_arg(arguments, "provider_max_results"),
        provider_max_provider_calls: optional_usize_arg(arguments, "provider_max_provider_calls"),
        enqueue_selected_url_ingest: optional_bool_arg(arguments, "enqueue_selected_url_ingest"),
        max_ingest_jobs: optional_usize_arg(arguments, "max_ingest_jobs"),
        provider_cost_cap_usd: optional_f64_arg(arguments, "provider_cost_cap_usd"),
        provider_endpoint: arguments
            .get("provider_endpoint")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        provider_api_key: arguments
            .get("provider_api_key")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        provider_model: arguments
            .get("provider_model")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        provider_timeout_seconds: arguments
            .get("provider_timeout_seconds")
            .and_then(Value::as_u64),
        max_iterations: optional_usize_arg(arguments, "max_iterations"),
        max_seconds: optional_i64_arg(arguments, "max_seconds"),
        max_sources: optional_usize_arg(arguments, "max_sources"),
        max_provider_calls: optional_usize_arg(arguments, "max_provider_calls"),
        cost_cap_usd: optional_f64_arg(arguments, "cost_cap_usd"),
        source_novelty_threshold: optional_f64_arg(arguments, "source_novelty_threshold"),
        confidence_delta_threshold: optional_f64_arg(arguments, "confidence_delta_threshold"),
        no_progress_iteration_limit: optional_usize_arg(arguments, "no_progress_iteration_limit"),
        require_active_fact_check: optional_bool_arg(arguments, "require_active_fact_check"),
        allow_long_run: optional_bool_arg(arguments, "allow_long_run"),
        no_write: optional_bool_arg(arguments, "no_write"),
        editorial_provider: arguments
            .get("editorial_provider")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        editorial_model_name: arguments
            .get("editorial_model_name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        editorial_endpoint: arguments
            .get("editorial_endpoint")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        editorial_timeout_seconds: arguments
            .get("editorial_timeout_seconds")
            .and_then(Value::as_u64),
    })
}

pub(crate) fn policy_request_from_mcp_args(arguments: &Value) -> Result<PolicyRequest> {
    Ok(PolicyRequest {
        action: required_string(arguments, "action")?,
        package: arguments
            .get("package")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        provider: arguments
            .get("provider")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        source: arguments
            .get("source")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        channel: arguments
            .get("channel")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        subject: arguments
            .get("subject")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        target: arguments
            .get("target")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        projected_usd: arguments.get("projected_usd").and_then(Value::as_f64),
        metadata: arguments
            .get("metadata")
            .cloned()
            .unwrap_or_else(|| json!({})),
        untrusted_excerpt: arguments
            .get("untrusted_excerpt")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    })
}

pub(crate) fn string_array_argument(arguments: &Value, key: &str) -> Result<Vec<String>> {
    arguments
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    item.as_str()
                        .map(ToOwned::to_owned)
                        .with_context(|| format!("{key} must contain only strings"))
                })
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

pub(crate) fn json_argument(
    arguments: &Value,
    key: &str,
    json_key: &str,
    default: Value,
) -> Result<Value> {
    if let Some(value) = arguments.get(key) {
        return Ok(value.clone());
    }
    if let Some(raw) = arguments.get(json_key).and_then(Value::as_str) {
        return serde_json::from_str(raw).with_context(|| format!("parsing {json_key}"));
    }
    Ok(default)
}

pub(crate) fn job_import_batch_from_mcp(arguments: &Value) -> Result<JobImportBatchInput> {
    if let Some(raw) = arguments.get("batch_json").and_then(Value::as_str) {
        return serde_json::from_str(raw).context("parsing batch_json");
    }
    let value = arguments
        .get("batch")
        .cloned()
        .unwrap_or_else(|| arguments.clone());
    serde_json::from_value(value).context("parsing job import batch")
}

pub(crate) fn commerce_run_config_input_from_mcp(
    arguments: &Value,
) -> Result<CommerceRunConfigInput> {
    Ok(CommerceRunConfigInput {
        run_id: required_string(arguments, "run_id")?,
        domain_profile: required_string(arguments, "domain_profile")?,
        target_qualified_count: optional_usize(arguments, "target_qualified_count", 20),
        geography: arguments
            .get("geography")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        freshness_window: optional_string(arguments, "freshness_window", "24h"),
        allowed_private_context_sources: string_array_argument(
            arguments,
            "allowed_private_context_sources",
        )?,
        allowed_public_source_families: string_array_argument(
            arguments,
            "allowed_public_source_families",
        )?,
        allow_marketplaces: optional_bool(arguments, "allow_marketplaces", false),
        allow_chrome_profile: optional_bool(arguments, "allow_chrome_profile", false),
        max_provider_calls: optional_usize_arg(arguments, "max_provider_calls"),
        max_browser_pages: optional_usize_arg(arguments, "max_browser_pages"),
        max_cost_usd: optional_f64_arg(arguments, "max_cost_usd"),
        stop_rules: json_argument(arguments, "stop_rules", "stop_rules_json", json!({}))?,
    })
}

pub(crate) fn commerce_candidate_input_from_mcp(
    arguments: &Value,
) -> Result<CommerceCandidateInput> {
    Ok(CommerceCandidateInput {
        run_id: required_string(arguments, "run_id")?,
        domain: required_string(arguments, "domain")?,
        source_url: required_string(arguments, "source_url")?,
        retailer_or_provider: required_string(arguments, "retailer_or_provider")?,
        title: required_string(arguments, "title")?,
        normalized_item_key: required_string(arguments, "normalized_item_key")?,
        variant_key: required_string(arguments, "variant_key")?,
        price: arguments
            .get("price")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        currency: arguments
            .get("currency")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        geography: arguments
            .get("geography")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        candidate_status: optional_string(arguments, "candidate_status", "maybe"),
        score: optional_f64_arg(arguments, "score"),
        score_reasons: json_argument(arguments, "score_reasons", "score_reasons_json", json!({}))?,
        disqualification_reasons: json_argument(
            arguments,
            "disqualification_reasons",
            "disqualification_reasons_json",
            json!([]),
        )?,
        metadata: json_argument(arguments, "metadata", "metadata_json", json!({}))?,
    })
}

pub(crate) fn commerce_availability_proof_input_from_mcp(
    arguments: &Value,
) -> Result<CommerceAvailabilityProofInput> {
    Ok(CommerceAvailabilityProofInput {
        run_id: required_string(arguments, "run_id")?,
        candidate_id: required_string(arguments, "candidate_id")?,
        proof_method: required_string(arguments, "proof_method")?,
        variant_key: required_string(arguments, "variant_key")?,
        variant_label: required_string(arguments, "variant_label")?,
        availability_state: required_string(arguments, "availability_state")?,
        visible_evidence: arguments
            .get("visible_evidence")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        selector_or_dom_hint: arguments
            .get("selector_or_dom_hint")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        screenshot_artifact_id: arguments
            .get("screenshot_artifact_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        page_snapshot_artifact_id: arguments
            .get("page_snapshot_artifact_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        confidence: optional_f64_arg(arguments, "confidence").unwrap_or(0.7),
        caveats: json_argument(arguments, "caveats", "caveats_json", json!([]))?,
        checked_at: arguments
            .get("checked_at")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    })
}

pub(crate) fn commerce_rendered_page_check_input_from_mcp(
    arguments: &Value,
) -> Result<CommerceRenderedPageCheckInput> {
    Ok(CommerceRenderedPageCheckInput {
        run_id: required_string(arguments, "run_id")?,
        candidate_id: required_string(arguments, "candidate_id")?,
        variant_key: required_string(arguments, "variant_key")?,
        variant_label: required_string(arguments, "variant_label")?,
        snapshot: RenderedPageSnapshotInput {
            requested_url: required_string(arguments, "requested_url")?,
            final_url: arguments
                .get("final_url")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            title: arguments
                .get("title")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            rendered_html: arguments
                .get("rendered_html")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            rendered_text: arguments
                .get("rendered_text")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            captured_at: arguments
                .get("captured_at")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            browser: arguments
                .get("browser")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            screenshot_path: arguments
                .get("screenshot_path")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
        },
        selector_or_dom_hint: arguments
            .get("selector_or_dom_hint")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        chrome_profile_required: optional_bool(arguments, "chrome_profile_required", false),
    })
}

pub(crate) fn commerce_context_fact_input_from_mcp(
    arguments: &Value,
) -> Result<CommerceContextFactInput> {
    Ok(CommerceContextFactInput {
        run_id: required_string(arguments, "run_id")?,
        fact_key: required_string(arguments, "fact_key")?,
        fact_kind: required_string(arguments, "fact_kind")?,
        redacted_value: required_string(arguments, "redacted_value")?,
        source_family: required_string(arguments, "source_family")?,
        source_ref: arguments
            .get("source_ref")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        confidence: optional_f64_arg(arguments, "confidence").unwrap_or(0.7),
        user_confirmed: optional_bool(arguments, "user_confirmed", false),
        may_persist_to_memory: optional_bool(arguments, "may_persist_to_memory", false),
        metadata: json_argument(arguments, "metadata", "metadata_json", json!({}))?,
    })
}

pub(crate) fn commerce_verification_attempt_input_from_mcp(
    arguments: &Value,
) -> Result<CommerceVerificationAttemptInput> {
    Ok(CommerceVerificationAttemptInput {
        run_id: required_string(arguments, "run_id")?,
        candidate_id: required_string(arguments, "candidate_id")?,
        method: required_string(arguments, "method")?,
        result: required_string(arguments, "result")?,
        error_kind: arguments
            .get("error_kind")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        final_url: arguments
            .get("final_url")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        http_status: optional_i64_arg(arguments, "http_status"),
        browser_required: optional_bool(arguments, "browser_required", false),
        chrome_profile_required: optional_bool(arguments, "chrome_profile_required", false),
        artifact_ids: string_array_argument(arguments, "artifact_ids")?,
        next_action: arguments
            .get("next_action")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        attempted_at: arguments
            .get("attempted_at")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    })
}

pub(crate) fn commerce_report_judgment_input_from_mcp(
    arguments: &Value,
) -> Result<CommerceReportJudgmentInput> {
    Ok(CommerceReportJudgmentInput {
        run_id: required_string(arguments, "run_id")?,
        decision: required_string(arguments, "decision")?,
        blocking_findings: json_argument(
            arguments,
            "blocking_findings",
            "blocking_findings_json",
            json!([]),
        )?,
        non_blocking_findings: json_argument(
            arguments,
            "non_blocking_findings",
            "non_blocking_findings_json",
            json!([]),
        )?,
        claims_checked: json_argument(
            arguments,
            "claims_checked",
            "claims_checked_json",
            json!([]),
        )?,
        availability_proofs_checked: json_argument(
            arguments,
            "availability_proofs_checked",
            "availability_proofs_checked_json",
            json!([]),
        )?,
        privacy_review: json_argument(
            arguments,
            "privacy_review",
            "privacy_review_json",
            json!({}),
        )?,
        remaining_risks: json_argument(
            arguments,
            "remaining_risks",
            "remaining_risks_json",
            json!([]),
        )?,
    })
}
