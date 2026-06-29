use super::*;

pub(crate) async fn http_ops(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    json_response(&state, &headers, &uri, || {
        Ok(json!(Store::open(state.paths.clone())?.ops_snapshot()?))
    })
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct OpsUiQuery {
    pub(crate) q: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) sort: Option<String>,
    pub(crate) detail: Option<String>,
    pub(crate) notice: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpsEdgeDeadLetterForm {
    pub(crate) csrf_token: String,
    pub(crate) idempotency_key: String,
    pub(crate) edge_event_id: String,
    pub(crate) reason: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpsXBookmarksScheduleForm {
    pub(crate) csrf_token: String,
    pub(crate) idempotency_key: String,
    pub(crate) bookmark_days: i64,
    pub(crate) max_bookmarks: usize,
    pub(crate) cadence: String,
    pub(crate) status: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpsXBookmarksEnqueueForm {
    pub(crate) csrf_token: String,
    pub(crate) idempotency_key: String,
    pub(crate) bookmark_days: i64,
    pub(crate) max_bookmarks: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpsKnowledgeBacklogScheduleForm {
    pub(crate) csrf_token: String,
    pub(crate) idempotency_key: String,
    pub(crate) max_source_cards: usize,
    pub(crate) min_group_size: usize,
    pub(crate) max_clusters: usize,
    pub(crate) cadence: String,
    pub(crate) status: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpsKnowledgeBacklogEnqueueForm {
    pub(crate) csrf_token: String,
    pub(crate) idempotency_key: String,
    pub(crate) max_source_cards: usize,
    pub(crate) min_group_size: usize,
    pub(crate) max_clusters: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpsKnowledgeModelClustersScheduleForm {
    pub(crate) csrf_token: String,
    pub(crate) idempotency_key: String,
    pub(crate) query: String,
    pub(crate) model_provider: String,
    pub(crate) model_name: Option<String>,
    pub(crate) endpoint: Option<String>,
    pub(crate) timeout_seconds: Option<u64>,
    pub(crate) max_source_cards: usize,
    pub(crate) max_clusters: usize,
    pub(crate) cadence: String,
    pub(crate) status: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpsKnowledgeModelClustersEnqueueForm {
    pub(crate) csrf_token: String,
    pub(crate) idempotency_key: String,
    pub(crate) query: String,
    pub(crate) model_provider: String,
    pub(crate) model_name: Option<String>,
    pub(crate) endpoint: Option<String>,
    pub(crate) timeout_seconds: Option<u64>,
    pub(crate) max_source_cards: usize,
    pub(crate) max_clusters: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpsKnowledgeModelWriteScheduleForm {
    pub(crate) csrf_token: String,
    pub(crate) idempotency_key: String,
    pub(crate) cluster_id: String,
    pub(crate) model_provider: String,
    pub(crate) model_name: Option<String>,
    pub(crate) endpoint: Option<String>,
    pub(crate) timeout_seconds: Option<u64>,
    pub(crate) create_digest: bool,
    pub(crate) cadence: String,
    pub(crate) status: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpsKnowledgeModelWriteEnqueueForm {
    pub(crate) csrf_token: String,
    pub(crate) idempotency_key: String,
    pub(crate) cluster_id: String,
    pub(crate) model_provider: String,
    pub(crate) model_name: Option<String>,
    pub(crate) endpoint: Option<String>,
    pub(crate) timeout_seconds: Option<u64>,
    pub(crate) create_digest: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpsKnowledgeDueClustersForm {
    pub(crate) csrf_token: String,
    pub(crate) idempotency_key: String,
    pub(crate) max_clusters: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpsKnowledgeDueModelWritesForm {
    pub(crate) csrf_token: String,
    pub(crate) idempotency_key: String,
    pub(crate) max_clusters: usize,
    pub(crate) model_provider: String,
    pub(crate) model_name: Option<String>,
    pub(crate) endpoint: Option<String>,
    pub(crate) timeout_seconds: Option<u64>,
    pub(crate) create_digest: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpsKnowledgeEntityResolutionScheduleForm {
    pub(crate) csrf_token: String,
    pub(crate) idempotency_key: String,
    pub(crate) model_provider: String,
    pub(crate) model_name: Option<String>,
    pub(crate) endpoint: Option<String>,
    pub(crate) timeout_seconds: Option<u64>,
    pub(crate) max_pairs: usize,
    pub(crate) cadence: String,
    pub(crate) status: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpsKnowledgeEntityResolutionEnqueueForm {
    pub(crate) csrf_token: String,
    pub(crate) idempotency_key: String,
    pub(crate) model_provider: String,
    pub(crate) model_name: Option<String>,
    pub(crate) endpoint: Option<String>,
    pub(crate) timeout_seconds: Option<u64>,
    pub(crate) max_pairs: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpsKnowledgeClusterPromoteForm {
    pub(crate) csrf_token: String,
    pub(crate) idempotency_key: String,
    pub(crate) cluster_id: String,
    pub(crate) reviewer: String,
    pub(crate) reason: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpsWorkerRunOnceForm {
    pub(crate) csrf_token: String,
    pub(crate) idempotency_key: String,
    pub(crate) max_jobs: usize,
}

pub(crate) async fn http_ops_ui(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    query: Result<Query<OpsUiQuery>, QueryRejection>,
) -> Response {
    if let Err(error) = validate_http_request(&state, &headers, &uri) {
        return http_html_error_response(error);
    }
    let Query(query) = match query {
        Ok(query) => query,
        Err(error) => {
            return http_html_error_response(HttpError::bad_request(
                "bad_query",
                error.to_string(),
            ));
        }
    };
    if let Err(error) = validate_ops_ui_query(&query) {
        return http_html_error_response(error);
    }
    match Store::open(state.paths.clone()).and_then(|store| store.ops_snapshot()) {
        Ok(snapshot) => with_http_security_headers(
            Html(render_ops_ui_with_options(
                &snapshot,
                &OpsUiOptions::from_query(query),
                Some(&state.csrf_token),
                state.auth_token.is_some(),
            ))
            .into_response(),
        ),
        Err(error) => http_html_error_response(HttpError::internal(error.to_string())),
    }
}

pub(crate) async fn http_ops_edge_event_dead_letter(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_dead_letter_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if !constant_time_eq(form.csrf_token.as_bytes(), state.csrf_token.as_bytes()) {
        return http_error_response(HttpError::new(
            StatusCode::FORBIDDEN,
            "bad_csrf",
            "CSRF token is missing or invalid",
        ));
    }
    if let Err(error) = validate_ops_idempotency_key(&form.idempotency_key) {
        return http_error_response(error);
    }
    if form.reason.trim().is_empty() || form.reason.len() > 1000 {
        return http_error_response(HttpError::bad_request(
            "bad_reason",
            "dead-letter reason must be non-empty and at most 1000 bytes",
        ));
    }
    let idempotency_scope = format!(
        "edge-event-dead-letter:{}:{}",
        form.edge_event_id, form.idempotency_key
    );
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui(&format!(
            "/ops/ui?detail=edge:{}&notice=duplicate",
            url_component(&form.edge_event_id)
        ));
    }

    let result = (|| -> Result<String> {
        let store = Store::open(state.paths.clone())?;
        let event = store
            .get_edge_event(&form.edge_event_id)?
            .with_context(|| format!("edge event not found: {}", form.edge_event_id))?;
        if !is_dead_letterable_edge_status(&event.status) {
            bail!(
                "edge event {} is status {}; only pending, failed, or leased events can be dead-lettered from ops UI",
                event.id,
                event.status
            );
        }
        let decision = store.policy_check(PolicyRequest {
            action: "ops.edge_event.dead_letter".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: None,
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some(event.id.clone()),
            projected_usd: None,
            metadata: json!({
                "edge_event_source": event.source,
                "edge_event_status": event.status,
                "idempotency_key": form.idempotency_key,
            }),
            untrusted_excerpt: Some(form.reason.clone()),
        })?;
        if !decision.allowed {
            bail!(
                "policy denied ops.edge_event.dead_letter: {}",
                decision.reason
            );
        }
        let reason = redact_secret_like_text(&form.reason);
        let updated = store.dead_letter_edge_event(&form.edge_event_id, &reason)?;
        Ok(updated.id)
    })();

    match result {
        Ok(id) => redirect_to_ops_ui(&format!(
            "/ops/ui?detail=edge:{}&notice=dead_lettered",
            url_component(&id)
        )),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_x_bookmarks_schedule(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_x_bookmarks_schedule_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!("x-bookmarks-schedule:{}", form.idempotency_key);
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?q=x_bookmarks&notice=duplicate");
    }

    let result = (|| -> Result<String> {
        let store = Store::open(state.paths.clone())?;
        let bookmark_days = form.bookmark_days.clamp(1, 36_500);
        let max_bookmarks = form.max_bookmarks.clamp(1, 100_000);
        let cadence = validate_ops_x_schedule_word(&form.cadence, "cadence")?;
        let status = validate_ops_x_schedule_word(&form.status, "status")?;
        let decision = store.policy_check(PolicyRequest {
            action: "ops.x_bookmarks.schedule".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: Some("x".to_string()),
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some("x:bookmarks".to_string()),
            projected_usd: None,
            metadata: json!({
                "bookmark_days": bookmark_days,
                "max_bookmarks": max_bookmarks,
                "cadence": cadence,
                "status": status,
                "idempotency_key": form.idempotency_key,
            }),
            untrusted_excerpt: None,
        })?;
        if !decision.allowed {
            bail!(
                "policy denied ops.x_bookmarks.schedule: {}",
                decision.reason
            );
        }
        let source =
            store.schedule_x_bookmark_import(bookmark_days, max_bookmarks, &cadence, &status)?;
        Ok(source.id)
    })();

    match result {
        Ok(_) => redirect_to_ops_ui("/ops/ui?q=x_bookmarks&notice=x_bookmarks_scheduled"),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_x_bookmarks_enqueue(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_x_bookmarks_enqueue_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!("x-bookmarks-enqueue:{}", form.idempotency_key);
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?q=x_import_bookmarks&notice=duplicate");
    }

    let result = (|| -> Result<String> {
        let store = Store::open(state.paths.clone())?;
        let bookmark_days = form.bookmark_days.clamp(1, 36_500);
        let max_bookmarks = form.max_bookmarks.clamp(1, 100_000);
        let decision = store.policy_check(PolicyRequest {
            action: "ops.x_bookmarks.enqueue".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: Some("x".to_string()),
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some("x_import_bookmarks".to_string()),
            projected_usd: None,
            metadata: json!({
                "bookmark_days": bookmark_days,
                "max_bookmarks": max_bookmarks,
                "idempotency_key": form.idempotency_key,
            }),
            untrusted_excerpt: None,
        })?;
        if !decision.allowed {
            bail!("policy denied ops.x_bookmarks.enqueue: {}", decision.reason);
        }
        let job = store.enqueue_x_import_bookmarks_job(bookmark_days, max_bookmarks)?;
        Ok(job.id)
    })();

    match result {
        Ok(id) => redirect_to_ops_ui(&format!(
            "/ops/ui?detail=job:{}&notice=x_bookmarks_enqueued",
            url_component(&id)
        )),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_knowledge_backlog_schedule(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_knowledge_backlog_schedule_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!("knowledge-backlog-schedule:{}", form.idempotency_key);
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?q=knowledge_backlog&notice=duplicate");
    }

    let result = (|| -> Result<String> {
        let store = Store::open(state.paths.clone())?;
        let max_source_cards = form.max_source_cards.clamp(1, 500);
        let min_group_size = form.min_group_size.clamp(1, 20);
        let max_clusters = form.max_clusters.clamp(1, 50);
        let cadence = validate_ops_x_schedule_word(&form.cadence, "cadence")?;
        let status = validate_ops_x_schedule_word(&form.status, "status")?;
        let decision = store.policy_check(PolicyRequest {
            action: "ops.knowledge_backlog.schedule".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: None,
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some("knowledge:source-card-backlog".to_string()),
            projected_usd: None,
            metadata: json!({
                "max_source_cards": max_source_cards,
                "min_group_size": min_group_size,
                "max_clusters": max_clusters,
                "cadence": cadence,
                "status": status,
                "idempotency_key": form.idempotency_key,
            }),
            untrusted_excerpt: None,
        })?;
        if !decision.allowed {
            bail!(
                "policy denied ops.knowledge_backlog.schedule: {}",
                decision.reason
            );
        }
        let source = store.schedule_knowledge_cluster_backlog(
            max_source_cards,
            min_group_size,
            max_clusters,
            &cadence,
            &status,
        )?;
        Ok(source.id)
    })();

    match result {
        Ok(_) => {
            redirect_to_ops_ui("/ops/ui?q=knowledge_backlog&notice=knowledge_backlog_scheduled")
        }
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_knowledge_backlog_enqueue(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_knowledge_backlog_enqueue_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!("knowledge-backlog-enqueue:{}", form.idempotency_key);
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?q=knowledge_cluster_backlog&notice=duplicate");
    }

    let result = (|| -> Result<String> {
        let store = Store::open(state.paths.clone())?;
        let max_source_cards = form.max_source_cards.clamp(1, 500);
        let min_group_size = form.min_group_size.clamp(1, 20);
        let max_clusters = form.max_clusters.clamp(1, 50);
        let decision = store.policy_check(PolicyRequest {
            action: "ops.knowledge_backlog.enqueue".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: None,
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some("knowledge_cluster_backlog".to_string()),
            projected_usd: None,
            metadata: json!({
                "max_source_cards": max_source_cards,
                "min_group_size": min_group_size,
                "max_clusters": max_clusters,
                "idempotency_key": form.idempotency_key,
            }),
            untrusted_excerpt: None,
        })?;
        if !decision.allowed {
            bail!(
                "policy denied ops.knowledge_backlog.enqueue: {}",
                decision.reason
            );
        }
        let job = store.enqueue_knowledge_cluster_backlog_job(
            max_source_cards,
            min_group_size,
            max_clusters,
        )?;
        Ok(job.id)
    })();

    match result {
        Ok(id) => redirect_to_ops_ui(&format!(
            "/ops/ui?detail=job:{}&notice=knowledge_backlog_enqueued",
            url_component(&id)
        )),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_knowledge_model_clusters_schedule(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_knowledge_model_clusters_schedule_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!("knowledge-model-clusters-schedule:{}", form.idempotency_key);
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?q=knowledge_model_clusters&notice=duplicate");
    }

    let result = (|| -> Result<String> {
        let store = Store::open(state.paths.clone())?;
        let cadence = validate_ops_x_schedule_word(&form.cadence, "cadence")?;
        let status = validate_ops_x_schedule_word(&form.status, "status")?;
        let max_source_cards = form.max_source_cards.clamp(1, 80);
        let max_clusters = form.max_clusters.clamp(1, 12);
        let provider = form.model_provider.trim().to_ascii_lowercase();
        let decision = store.policy_check(PolicyRequest {
            action: "ops.knowledge_model_clusters.schedule".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: Some(provider.clone()),
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some("knowledge_model_clusters".to_string()),
            projected_usd: None,
            metadata: json!({
                "query": form.query.clone(),
                "model_provider": provider.clone(),
                "model_name": form.model_name.clone(),
                "endpoint_configured": form.endpoint.is_some(),
                "timeout_seconds": form.timeout_seconds,
                "max_source_cards": max_source_cards,
                "max_clusters": max_clusters,
                "cadence": cadence,
                "status": status,
                "idempotency_key": form.idempotency_key.clone(),
            }),
            untrusted_excerpt: None,
        })?;
        if !decision.allowed {
            bail!(
                "policy denied ops.knowledge_model_clusters.schedule: {}",
                decision.reason
            );
        }
        let source = store.schedule_knowledge_cluster_model_proposals(
            &form.query,
            &provider,
            form.model_name.as_deref(),
            form.endpoint.as_deref(),
            form.timeout_seconds,
            max_source_cards,
            max_clusters,
            &cadence,
            &status,
        )?;
        Ok(source.id)
    })();

    match result {
        Ok(_) => redirect_to_ops_ui(
            "/ops/ui?q=knowledge_model_clusters&notice=knowledge_model_clusters_scheduled",
        ),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_knowledge_model_clusters_enqueue(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_knowledge_model_clusters_enqueue_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!("knowledge-model-clusters-enqueue:{}", form.idempotency_key);
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?q=knowledge_cluster_model_propose&notice=duplicate");
    }

    let result = (|| -> Result<String> {
        let store = Store::open(state.paths.clone())?;
        let max_source_cards = form.max_source_cards.clamp(1, 80);
        let max_clusters = form.max_clusters.clamp(1, 12);
        let provider = form.model_provider.trim().to_ascii_lowercase();
        let decision = store.policy_check(PolicyRequest {
            action: "ops.knowledge_model_clusters.enqueue".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: Some(provider.clone()),
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some("knowledge_cluster_model_propose".to_string()),
            projected_usd: None,
            metadata: json!({
                "query": form.query.clone(),
                "model_provider": provider.clone(),
                "model_name": form.model_name.clone(),
                "endpoint_configured": form.endpoint.is_some(),
                "timeout_seconds": form.timeout_seconds,
                "max_source_cards": max_source_cards,
                "max_clusters": max_clusters,
                "idempotency_key": form.idempotency_key.clone(),
            }),
            untrusted_excerpt: None,
        })?;
        if !decision.allowed {
            bail!(
                "policy denied ops.knowledge_model_clusters.enqueue: {}",
                decision.reason
            );
        }
        let job = store.enqueue_knowledge_cluster_model_proposal_job(
            &form.query,
            &provider,
            form.model_name.as_deref(),
            form.endpoint.as_deref(),
            form.timeout_seconds,
            max_source_cards,
            max_clusters,
        )?;
        Ok(job.id)
    })();

    match result {
        Ok(id) => redirect_to_ops_ui(&format!(
            "/ops/ui?detail=job:{}&notice=knowledge_model_clusters_enqueued",
            url_component(&id)
        )),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_knowledge_model_write_schedule(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_knowledge_model_write_schedule_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!("knowledge-model-write-schedule:{}", form.idempotency_key);
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?q=knowledge_model_write&notice=duplicate");
    }

    let result = (|| -> Result<String> {
        let store = Store::open(state.paths.clone())?;
        let cadence = validate_ops_x_schedule_word(&form.cadence, "cadence")?;
        let status = validate_ops_x_schedule_word(&form.status, "status")?;
        let provider = form.model_provider.trim().to_ascii_lowercase();
        let decision = store.policy_check(PolicyRequest {
            action: "ops.knowledge_model_write.schedule".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: Some(provider.clone()),
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some(form.cluster_id.clone()),
            projected_usd: None,
            metadata: json!({
                "cluster_id": form.cluster_id.clone(),
                "model_provider": provider.clone(),
                "model_name": form.model_name.clone(),
                "endpoint_configured": form.endpoint.is_some(),
                "timeout_seconds": form.timeout_seconds,
                "create_digest": form.create_digest,
                "cadence": cadence,
                "status": status,
                "idempotency_key": form.idempotency_key.clone(),
            }),
            untrusted_excerpt: None,
        })?;
        if !decision.allowed {
            bail!(
                "policy denied ops.knowledge_model_write.schedule: {}",
                decision.reason
            );
        }
        let source = store.schedule_knowledge_cluster_model_write(
            &form.cluster_id,
            &provider,
            form.model_name.as_deref(),
            form.endpoint.as_deref(),
            form.timeout_seconds,
            form.create_digest,
            &cadence,
            &status,
        )?;
        Ok(source.id)
    })();

    match result {
        Ok(_) => redirect_to_ops_ui(
            "/ops/ui?q=knowledge_model_write&notice=knowledge_model_write_scheduled",
        ),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_knowledge_model_write_enqueue(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_knowledge_model_write_enqueue_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!("knowledge-model-write-enqueue:{}", form.idempotency_key);
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?q=knowledge_cluster_model_write&notice=duplicate");
    }

    let result = (|| -> Result<String> {
        let store = Store::open(state.paths.clone())?;
        let provider = form.model_provider.trim().to_ascii_lowercase();
        let decision = store.policy_check(PolicyRequest {
            action: "ops.knowledge_model_write.enqueue".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: Some(provider.clone()),
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some(form.cluster_id.clone()),
            projected_usd: None,
            metadata: json!({
                "cluster_id": form.cluster_id.clone(),
                "model_provider": provider.clone(),
                "model_name": form.model_name.clone(),
                "endpoint_configured": form.endpoint.is_some(),
                "timeout_seconds": form.timeout_seconds,
                "create_digest": form.create_digest,
                "idempotency_key": form.idempotency_key.clone(),
            }),
            untrusted_excerpt: None,
        })?;
        if !decision.allowed {
            bail!(
                "policy denied ops.knowledge_model_write.enqueue: {}",
                decision.reason
            );
        }
        let job = store.enqueue_knowledge_cluster_model_writer_job(
            &form.cluster_id,
            &provider,
            form.model_name.as_deref(),
            form.endpoint.as_deref(),
            form.timeout_seconds,
            form.create_digest,
        )?;
        Ok(job.id)
    })();

    match result {
        Ok(id) => redirect_to_ops_ui(&format!(
            "/ops/ui?detail=job:{}&notice=knowledge_model_write_enqueued",
            url_component(&id)
        )),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_knowledge_cluster_expansions_enqueue(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_knowledge_due_clusters_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!("knowledge-cluster-expansions:{}", form.idempotency_key);
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?q=knowledge_cluster&notice=duplicate");
    }

    let result = (|| -> Result<usize> {
        let store = Store::open(state.paths.clone())?;
        let max_clusters = form.max_clusters.clamp(1, 100);
        let decision = store.policy_check(PolicyRequest {
            action: "ops.knowledge_clusters.enqueue_expansions".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: None,
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some("knowledge_cluster_expand".to_string()),
            projected_usd: None,
            metadata: json!({
                "max_clusters": max_clusters,
                "idempotency_key": form.idempotency_key,
            }),
            untrusted_excerpt: None,
        })?;
        if !decision.allowed {
            bail!(
                "policy denied ops.knowledge_clusters.enqueue_expansions: {}",
                decision.reason
            );
        }
        let report = store.enqueue_due_knowledge_cluster_expansion_jobs(max_clusters)?;
        Ok(report.enqueued)
    })();

    match result {
        Ok(enqueued) => redirect_to_ops_ui(&format!(
            "/ops/ui?q=knowledge_cluster&notice=knowledge_cluster_expansions_enqueued&count={}",
            enqueued
        )),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_knowledge_cluster_editorial_decisions_enqueue(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_knowledge_due_clusters_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!(
        "knowledge-cluster-editorial-decisions:{}",
        form.idempotency_key
    );
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?q=knowledge_cluster&notice=duplicate");
    }

    let result = (|| -> Result<usize> {
        let store = Store::open(state.paths.clone())?;
        let max_clusters = form.max_clusters.clamp(1, 100);
        let decision = store.policy_check(PolicyRequest {
            action: "ops.knowledge_clusters.enqueue_editorial_decisions".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: None,
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some("knowledge_cluster_editorial_decide".to_string()),
            projected_usd: None,
            metadata: json!({
                "max_clusters": max_clusters,
                "idempotency_key": form.idempotency_key,
            }),
            untrusted_excerpt: None,
        })?;
        if !decision.allowed {
            bail!(
                "policy denied ops.knowledge_clusters.enqueue_editorial_decisions: {}",
                decision.reason
            );
        }
        let report = store.enqueue_due_knowledge_cluster_editorial_decision_jobs(max_clusters)?;
        Ok(report.enqueued)
    })();

    match result {
        Ok(enqueued) => redirect_to_ops_ui(&format!(
            "/ops/ui?q=knowledge_cluster&notice=knowledge_cluster_editorial_decisions_enqueued&count={}",
            enqueued
        )),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_knowledge_model_writes_enqueue_due(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_knowledge_due_model_writes_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!("knowledge-model-writes-due:{}", form.idempotency_key);
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?q=knowledge_cluster_model_write&notice=duplicate");
    }

    let result = (|| -> Result<usize> {
        let store = Store::open(state.paths.clone())?;
        let max_clusters = form.max_clusters.clamp(1, 100);
        let provider = form.model_provider.trim().to_ascii_lowercase();
        let decision = store.policy_check(PolicyRequest {
            action: "ops.knowledge_model_write.enqueue_due".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: Some(provider.clone()),
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some("knowledge_cluster_model_write".to_string()),
            projected_usd: None,
            metadata: json!({
                "max_clusters": max_clusters,
                "model_provider": provider.clone(),
                "model_name": form.model_name.clone(),
                "endpoint_configured": form.endpoint.is_some(),
                "timeout_seconds": form.timeout_seconds,
                "create_digest": form.create_digest,
                "idempotency_key": form.idempotency_key,
            }),
            untrusted_excerpt: None,
        })?;
        if !decision.allowed {
            bail!(
                "policy denied ops.knowledge_model_write.enqueue_due: {}",
                decision.reason
            );
        }
        let report = store.enqueue_due_knowledge_cluster_model_writer_jobs(
            max_clusters,
            &provider,
            form.model_name.as_deref(),
            form.endpoint.as_deref(),
            form.timeout_seconds,
            form.create_digest,
        )?;
        Ok(report.enqueued)
    })();

    match result {
        Ok(enqueued) => redirect_to_ops_ui(&format!(
            "/ops/ui?q=knowledge_cluster_model_write&notice=knowledge_model_writes_due_enqueued&count={}",
            enqueued
        )),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_knowledge_entity_resolution_schedule(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_knowledge_entity_resolution_schedule_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!(
        "knowledge-entity-resolution-schedule:{}",
        form.idempotency_key
    );
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?q=knowledge_entity_resolution&notice=duplicate");
    }

    let result = (|| -> Result<String> {
        let store = Store::open(state.paths.clone())?;
        let cadence = validate_ops_x_schedule_word(&form.cadence, "cadence")?;
        let status = validate_ops_x_schedule_word(&form.status, "status")?;
        let max_pairs = form.max_pairs.clamp(1, 100);
        let provider = form.model_provider.trim().to_ascii_lowercase();
        let decision = store.policy_check(PolicyRequest {
            action: "ops.knowledge_entity_resolution.schedule".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: Some(provider.clone()),
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some("knowledge_entity_resolution".to_string()),
            projected_usd: None,
            metadata: json!({
                "model_provider": provider.clone(),
                "model_name": form.model_name.clone(),
                "endpoint_configured": form.endpoint.is_some(),
                "timeout_seconds": form.timeout_seconds,
                "max_pairs": max_pairs,
                "cadence": cadence,
                "status": status,
                "idempotency_key": form.idempotency_key.clone(),
                "boundary": "Ops control schedules review-only entity-resolution proposals; it cannot merge entities or create relations.",
            }),
            untrusted_excerpt: None,
        })?;
        if !decision.allowed {
            bail!(
                "policy denied ops.knowledge_entity_resolution.schedule: {}",
                decision.reason
            );
        }
        let source = store.schedule_knowledge_entity_resolution(
            &provider,
            form.model_name.as_deref(),
            form.endpoint.as_deref(),
            form.timeout_seconds,
            max_pairs,
            &cadence,
            &status,
        )?;
        Ok(source.id)
    })();

    match result {
        Ok(_) => redirect_to_ops_ui(
            "/ops/ui?q=knowledge_entity_resolution&notice=knowledge_entity_resolution_scheduled",
        ),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_knowledge_entity_resolution_enqueue_due(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_knowledge_entity_resolution_enqueue_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!(
        "knowledge-entity-resolution-enqueue-due:{}",
        form.idempotency_key
    );
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?q=knowledge_entity_resolution&notice=duplicate");
    }

    let result = (|| -> Result<usize> {
        let store = Store::open(state.paths.clone())?;
        let max_pairs = form.max_pairs.clamp(1, 100);
        let provider = form.model_provider.trim().to_ascii_lowercase();
        let decision = store.policy_check(PolicyRequest {
            action: "ops.knowledge_entity_resolution.enqueue_due".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: Some(provider.clone()),
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some("knowledge_entity_resolution_model".to_string()),
            projected_usd: None,
            metadata: json!({
                "model_provider": provider.clone(),
                "model_name": form.model_name.clone(),
                "endpoint_configured": form.endpoint.is_some(),
                "timeout_seconds": form.timeout_seconds,
                "max_pairs": max_pairs,
                "idempotency_key": form.idempotency_key,
                "boundary": "Ops control queues review-only entity-resolution proposals; merge decisions remain separate review.",
            }),
            untrusted_excerpt: None,
        })?;
        if !decision.allowed {
            bail!(
                "policy denied ops.knowledge_entity_resolution.enqueue_due: {}",
                decision.reason
            );
        }
        let report = store.enqueue_due_knowledge_entity_resolution_jobs(
            max_pairs,
            &provider,
            form.model_name.as_deref(),
            form.endpoint.as_deref(),
            form.timeout_seconds,
            Some(json!({
                "trigger": "ops_ui_enqueue_due",
                "operator": "local-operator",
                "boundary": "Ops UI enqueue writes review-only entity-resolution model jobs and does not authorize entity merges.",
            })),
        )?;
        Ok(report.enqueued)
    })();

    match result {
        Ok(enqueued) => redirect_to_ops_ui(&format!(
            "/ops/ui?q=knowledge_entity_resolution&notice=knowledge_entity_resolutions_due_enqueued&count={}",
            enqueued
        )),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_knowledge_cluster_promote(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_knowledge_cluster_promote_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!(
        "knowledge-cluster-promote:{}:{}",
        form.cluster_id, form.idempotency_key
    );
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?q=knowledge_cluster&notice=duplicate");
    }

    let result = (|| -> Result<String> {
        let store = Store::open(state.paths.clone())?;
        let cluster = store
            .get_knowledge_cluster(&form.cluster_id)?
            .with_context(|| format!("knowledge cluster not found: {}", form.cluster_id))?;
        let decision = store.policy_check(PolicyRequest {
            action: "ops.knowledge_clusters.promote".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: None,
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some(cluster.id.clone()),
            projected_usd: None,
            metadata: json!({
                "cluster_id": cluster.id,
                "cluster_topic": cluster.topic,
                "cluster_status": cluster.status,
                "cluster_origin": cluster.metadata.get("origin").and_then(Value::as_str),
                "reviewer": form.reviewer.clone(),
                "idempotency_key": form.idempotency_key,
                "boundary": "Ops control only authorizes the local operator action; core knowledge_cluster.promote policy still gates activating model-origin clusters.",
            }),
            untrusted_excerpt: Some(form.reason.clone()),
        })?;
        if !decision.allowed {
            bail!(
                "policy denied ops.knowledge_clusters.promote: {}",
                decision.reason
            );
        }
        let report = store.promote_knowledge_cluster(
            &cluster.id,
            Some(&form.reviewer),
            Some(&form.reason),
        )?;
        Ok(report.cluster.id)
    })();

    match result {
        Ok(id) => redirect_to_ops_ui(&format!(
            "/ops/ui?q=knowledge_cluster&notice=knowledge_cluster_promoted&cluster={}",
            url_component(&id)
        )),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_knowledge_investigation_execution_enqueue(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_knowledge_due_clusters_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!("knowledge-investigation-execution:{}", form.idempotency_key);
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?q=knowledge_investigation&notice=duplicate");
    }

    let result = (|| -> Result<usize> {
        let store = Store::open(state.paths.clone())?;
        let max_clusters = form.max_clusters.clamp(1, 100);
        let decision = store.policy_check(PolicyRequest {
            action: "ops.knowledge_investigations.enqueue_execution".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: None,
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some("knowledge_cluster_investigation_execute".to_string()),
            projected_usd: None,
            metadata: json!({
                "max_clusters": max_clusters,
                "idempotency_key": form.idempotency_key,
            }),
            untrusted_excerpt: None,
        })?;
        if !decision.allowed {
            bail!(
                "policy denied ops.knowledge_investigations.enqueue_execution: {}",
                decision.reason
            );
        }
        let report =
            store.enqueue_due_knowledge_cluster_investigation_execution_jobs(max_clusters)?;
        Ok(report.enqueued)
    })();

    match result {
        Ok(enqueued) => redirect_to_ops_ui(&format!(
            "/ops/ui?q=knowledge_investigation&notice=knowledge_investigations_enqueued&count={}",
            enqueued
        )),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}

pub(crate) async fn http_ops_worker_run_once(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(error) = validate_http_mutation_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    if body.len() as u64 > state.max_body_bytes {
        return http_error_response(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    let form = match parse_ops_worker_run_once_form(&body) {
        Ok(form) => form,
        Err(error) => return http_error_response(error),
    };
    if let Err(error) =
        validate_ops_csrf_and_idempotency(&state, &form.csrf_token, &form.idempotency_key)
    {
        return http_error_response(error);
    }
    let idempotency_scope = format!("worker-run-once:{}", form.idempotency_key);
    let inserted = match reserve_ops_idempotency(&state, idempotency_scope) {
        Ok(inserted) => inserted,
        Err(error) => return http_error_response(error),
    };
    if !inserted {
        return redirect_to_ops_ui("/ops/ui?notice=duplicate");
    }

    let result = (|| -> Result<usize> {
        let store = Store::open(state.paths.clone())?;
        let max_jobs = form.max_jobs.clamp(1, 25);
        let decision = store.policy_check(PolicyRequest {
            action: "ops.worker.run_once".to_string(),
            package: Some("arcwell-cli".to_string()),
            provider: None,
            source: Some("ops-ui".to_string()),
            channel: Some("http".to_string()),
            subject: Some("local-operator".to_string()),
            target: Some("arcwell-worker".to_string()),
            projected_usd: None,
            metadata: json!({
                "max_jobs": max_jobs,
                "idempotency_key": form.idempotency_key,
            }),
            untrusted_excerpt: None,
        })?;
        if !decision.allowed {
            bail!("policy denied ops.worker.run_once: {}", decision.reason);
        }
        let report = store.run_worker_once(max_jobs)?;
        Ok(report.processed)
    })();

    match result {
        Ok(_) => redirect_to_ops_ui("/ops/ui?notice=worker_ran_once"),
        Err(error) => http_error_response(HttpError::bad_request(
            "ops_action_failed",
            error.to_string(),
        )),
    }
}
