use super::*;

pub(crate) async fn http_mutation_rejected(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if let Err(error) = validate_http_request(&state, &headers, &uri) {
        return http_error_response(error);
    }
    http_error_response(HttpError::new(
        StatusCode::METHOD_NOT_ALLOWED,
        "method_not_allowed",
        "Arcwell local HTTP currently exposes read-only GET routes; mutating browser requests are disabled until explicit CSRF-protected controls exist",
    ))
}

pub(crate) fn json_response(
    state: &HttpState,
    headers: &HeaderMap,
    uri: &Uri,
    build: impl FnOnce() -> Result<Value>,
) -> Response {
    if let Err(error) = validate_http_request(state, headers, uri) {
        return http_error_response(error);
    }
    match build() {
        Ok(value) => with_http_security_headers(Json(value).into_response()),
        Err(error) => http_error_response(HttpError::internal(error.to_string())),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HttpError {
    status: StatusCode,
    kind: &'static str,
    message: String,
}

impl HttpError {
    pub(crate) fn new(status: StatusCode, kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            kind,
            message: message.into(),
        }
    }

    pub(crate) fn bad_request(kind: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, kind, message)
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", message)
    }
}

pub(crate) fn validate_http_request(
    state: &HttpState,
    headers: &HeaderMap,
    uri: &Uri,
) -> std::result::Result<(), HttpError> {
    if uri.to_string().len() > state.max_uri_bytes {
        return Err(HttpError::new(
            StatusCode::URI_TOO_LONG,
            "uri_too_large",
            "request URI is too large",
        ));
    }
    if let Some(length) = headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        && length > state.max_body_bytes
    {
        return Err(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "request body is too large",
        ));
    }
    validate_local_origin(headers)?;
    validate_http_auth(state, headers)
}

pub(crate) fn validate_http_mutation_request(
    state: &HttpState,
    headers: &HeaderMap,
    uri: &Uri,
) -> std::result::Result<(), HttpError> {
    if state.auth_token.is_none() {
        return Err(HttpError::new(
            StatusCode::UNAUTHORIZED,
            "mutation_auth_required",
            "Arcwell HTTP mutations require an explicit auth token",
        ));
    }
    validate_http_request(state, headers, uri)
}

pub(crate) fn validate_http_auth(
    state: &HttpState,
    headers: &HeaderMap,
) -> std::result::Result<(), HttpError> {
    let Some(expected) = state.auth_token.as_deref() else {
        return Ok(());
    };
    let supplied = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .or_else(|| {
            headers
                .get("x-arcwell-http-token")
                .and_then(|value| value.to_str().ok())
        });
    let Some(supplied) = supplied else {
        return Err(HttpError::new(
            StatusCode::UNAUTHORIZED,
            "missing_auth",
            "HTTP auth token is required",
        ));
    };
    if !constant_time_eq(supplied.as_bytes(), expected.as_bytes()) {
        return Err(HttpError::new(
            StatusCode::UNAUTHORIZED,
            "bad_auth",
            "HTTP auth token is invalid",
        ));
    }
    Ok(())
}

pub(crate) fn validate_local_origin(headers: &HeaderMap) -> std::result::Result<(), HttpError> {
    let Some(origin) = headers.get(header::ORIGIN) else {
        return Ok(());
    };
    let origin = origin.to_str().map_err(|_| {
        HttpError::new(
            StatusCode::FORBIDDEN,
            "bad_origin",
            "Origin header is not valid UTF-8",
        )
    })?;
    if is_local_http_origin(origin) {
        return Ok(());
    }
    Err(HttpError::new(
        StatusCode::FORBIDDEN,
        "bad_origin",
        "cross-origin browser access is not allowed for the local HTTP API",
    ))
}

pub(crate) fn is_local_http_origin(origin: &str) -> bool {
    let Some(rest) = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
    else {
        return false;
    };
    let authority = rest.split('/').next().unwrap_or_default();
    let host = if authority.starts_with('[') {
        authority
            .strip_prefix('[')
            .and_then(|value| value.split(']').next())
            .unwrap_or_default()
    } else {
        authority.split(':').next().unwrap_or_default()
    };
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

pub(crate) fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (left, right) in left.iter().zip(right) {
        diff |= left ^ right;
    }
    diff == 0
}

pub(crate) fn validate_ops_ui_query(query: &OpsUiQuery) -> std::result::Result<(), HttpError> {
    for (name, value, max_len) in [
        ("q", query.q.as_deref(), 512),
        ("status", query.status.as_deref(), 80),
        ("sort", query.sort.as_deref(), 80),
        ("detail", query.detail.as_deref(), 160),
        ("notice", query.notice.as_deref(), 80),
    ] {
        let Some(value) = value else {
            continue;
        };
        if value.len() > max_len {
            return Err(HttpError::new(
                StatusCode::URI_TOO_LONG,
                "query_too_large",
                format!("query parameter {name} is too large"),
            ));
        }
        if value.chars().any(char::is_control) {
            return Err(HttpError::bad_request(
                "bad_query",
                format!("query parameter {name} contains control characters"),
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_ops_idempotency_key(key: &str) -> std::result::Result<(), HttpError> {
    let trimmed = key.trim();
    if trimmed.len() < 8 || trimmed.len() > 120 {
        return Err(HttpError::bad_request(
            "bad_idempotency_key",
            "idempotency key must be between 8 and 120 bytes",
        ));
    }
    if trimmed != key
        || !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.'))
    {
        return Err(HttpError::bad_request(
            "bad_idempotency_key",
            "idempotency key may only contain ASCII letters, numbers, dot, colon, underscore, or hyphen",
        ));
    }
    Ok(())
}

pub(crate) fn validate_ops_csrf_and_idempotency(
    state: &HttpState,
    csrf_token: &str,
    idempotency_key: &str,
) -> std::result::Result<(), HttpError> {
    if !constant_time_eq(csrf_token.as_bytes(), state.csrf_token.as_bytes()) {
        return Err(HttpError::new(
            StatusCode::FORBIDDEN,
            "bad_csrf",
            "CSRF token is missing or invalid",
        ));
    }
    validate_ops_idempotency_key(idempotency_key)
}

pub(crate) fn reserve_ops_idempotency(
    state: &HttpState,
    scope: String,
) -> std::result::Result<bool, HttpError> {
    state
        .idempotency_keys
        .lock()
        .map(|mut keys| keys.insert(scope))
        .map_err(|_| HttpError::internal("idempotency registry is unavailable"))
}

pub(crate) fn parse_ops_dead_letter_form(
    body: &[u8],
) -> std::result::Result<OpsEdgeDeadLetterForm, HttpError> {
    let mut values = parse_ops_form_fields(
        body,
        &["csrf_token", "idempotency_key", "edge_event_id", "reason"],
    )?;
    let mut take = |key: &'static str| {
        values
            .remove(key)
            .ok_or_else(|| HttpError::bad_request("bad_form", format!("missing form field: {key}")))
    };
    Ok(OpsEdgeDeadLetterForm {
        csrf_token: take("csrf_token")?,
        idempotency_key: take("idempotency_key")?,
        edge_event_id: take("edge_event_id")?,
        reason: take("reason")?,
    })
}

pub(crate) fn parse_ops_x_bookmarks_schedule_form(
    body: &[u8],
) -> std::result::Result<OpsXBookmarksScheduleForm, HttpError> {
    let mut values = parse_ops_form_fields(
        body,
        &[
            "csrf_token",
            "idempotency_key",
            "bookmark_days",
            "max_bookmarks",
            "cadence",
            "status",
        ],
    )?;
    Ok(OpsXBookmarksScheduleForm {
        csrf_token: take_required_form_string(&mut values, "csrf_token")?,
        idempotency_key: take_required_form_string(&mut values, "idempotency_key")?,
        bookmark_days: take_required_form_i64(&mut values, "bookmark_days", 1, 36_500)?,
        max_bookmarks: take_required_form_usize(&mut values, "max_bookmarks", 1, 100_000)?,
        cadence: take_required_form_string(&mut values, "cadence")?,
        status: take_required_form_string(&mut values, "status")?,
    })
}

pub(crate) fn parse_ops_x_bookmarks_enqueue_form(
    body: &[u8],
) -> std::result::Result<OpsXBookmarksEnqueueForm, HttpError> {
    let mut values = parse_ops_form_fields(
        body,
        &[
            "csrf_token",
            "idempotency_key",
            "bookmark_days",
            "max_bookmarks",
        ],
    )?;
    Ok(OpsXBookmarksEnqueueForm {
        csrf_token: take_required_form_string(&mut values, "csrf_token")?,
        idempotency_key: take_required_form_string(&mut values, "idempotency_key")?,
        bookmark_days: take_required_form_i64(&mut values, "bookmark_days", 1, 36_500)?,
        max_bookmarks: take_required_form_usize(&mut values, "max_bookmarks", 1, 100_000)?,
    })
}

pub(crate) fn parse_ops_knowledge_backlog_schedule_form(
    body: &[u8],
) -> std::result::Result<OpsKnowledgeBacklogScheduleForm, HttpError> {
    let mut values = parse_ops_form_fields(
        body,
        &[
            "csrf_token",
            "idempotency_key",
            "max_source_cards",
            "min_group_size",
            "max_clusters",
            "cadence",
            "status",
        ],
    )?;
    Ok(OpsKnowledgeBacklogScheduleForm {
        csrf_token: take_required_form_string(&mut values, "csrf_token")?,
        idempotency_key: take_required_form_string(&mut values, "idempotency_key")?,
        max_source_cards: take_required_form_usize(&mut values, "max_source_cards", 1, 500)?,
        min_group_size: take_required_form_usize(&mut values, "min_group_size", 1, 20)?,
        max_clusters: take_required_form_usize(&mut values, "max_clusters", 1, 50)?,
        cadence: take_required_form_string(&mut values, "cadence")?,
        status: take_required_form_string(&mut values, "status")?,
    })
}

pub(crate) fn parse_ops_knowledge_backlog_enqueue_form(
    body: &[u8],
) -> std::result::Result<OpsKnowledgeBacklogEnqueueForm, HttpError> {
    let mut values = parse_ops_form_fields(
        body,
        &[
            "csrf_token",
            "idempotency_key",
            "max_source_cards",
            "min_group_size",
            "max_clusters",
        ],
    )?;
    Ok(OpsKnowledgeBacklogEnqueueForm {
        csrf_token: take_required_form_string(&mut values, "csrf_token")?,
        idempotency_key: take_required_form_string(&mut values, "idempotency_key")?,
        max_source_cards: take_required_form_usize(&mut values, "max_source_cards", 1, 500)?,
        min_group_size: take_required_form_usize(&mut values, "min_group_size", 1, 20)?,
        max_clusters: take_required_form_usize(&mut values, "max_clusters", 1, 50)?,
    })
}

pub(crate) fn parse_ops_knowledge_model_clusters_schedule_form(
    body: &[u8],
) -> std::result::Result<OpsKnowledgeModelClustersScheduleForm, HttpError> {
    let mut values = parse_ops_form_fields(
        body,
        &[
            "csrf_token",
            "idempotency_key",
            "query",
            "model_provider",
            "model_name",
            "endpoint",
            "timeout_seconds",
            "max_source_cards",
            "max_clusters",
            "cadence",
            "status",
        ],
    )?;
    Ok(OpsKnowledgeModelClustersScheduleForm {
        csrf_token: take_required_form_string(&mut values, "csrf_token")?,
        idempotency_key: take_required_form_string(&mut values, "idempotency_key")?,
        query: take_required_form_string(&mut values, "query")?,
        model_provider: take_required_form_string(&mut values, "model_provider")?,
        model_name: take_optional_form_string(&mut values, "model_name"),
        endpoint: take_optional_form_string(&mut values, "endpoint"),
        timeout_seconds: take_optional_form_u64(&mut values, "timeout_seconds", 1, 600)?,
        max_source_cards: take_required_form_usize(&mut values, "max_source_cards", 1, 80)?,
        max_clusters: take_required_form_usize(&mut values, "max_clusters", 1, 12)?,
        cadence: take_required_form_string(&mut values, "cadence")?,
        status: take_required_form_string(&mut values, "status")?,
    })
}

pub(crate) fn parse_ops_knowledge_model_clusters_enqueue_form(
    body: &[u8],
) -> std::result::Result<OpsKnowledgeModelClustersEnqueueForm, HttpError> {
    let mut values = parse_ops_form_fields(
        body,
        &[
            "csrf_token",
            "idempotency_key",
            "query",
            "model_provider",
            "model_name",
            "endpoint",
            "timeout_seconds",
            "max_source_cards",
            "max_clusters",
        ],
    )?;
    Ok(OpsKnowledgeModelClustersEnqueueForm {
        csrf_token: take_required_form_string(&mut values, "csrf_token")?,
        idempotency_key: take_required_form_string(&mut values, "idempotency_key")?,
        query: take_required_form_string(&mut values, "query")?,
        model_provider: take_required_form_string(&mut values, "model_provider")?,
        model_name: take_optional_form_string(&mut values, "model_name"),
        endpoint: take_optional_form_string(&mut values, "endpoint"),
        timeout_seconds: take_optional_form_u64(&mut values, "timeout_seconds", 1, 600)?,
        max_source_cards: take_required_form_usize(&mut values, "max_source_cards", 1, 80)?,
        max_clusters: take_required_form_usize(&mut values, "max_clusters", 1, 12)?,
    })
}

pub(crate) fn parse_ops_knowledge_model_write_schedule_form(
    body: &[u8],
) -> std::result::Result<OpsKnowledgeModelWriteScheduleForm, HttpError> {
    let mut values = parse_ops_form_fields(
        body,
        &[
            "csrf_token",
            "idempotency_key",
            "cluster_id",
            "model_provider",
            "model_name",
            "endpoint",
            "timeout_seconds",
            "create_digest",
            "cadence",
            "status",
        ],
    )?;
    Ok(OpsKnowledgeModelWriteScheduleForm {
        csrf_token: take_required_form_string(&mut values, "csrf_token")?,
        idempotency_key: take_required_form_string(&mut values, "idempotency_key")?,
        cluster_id: take_required_form_string(&mut values, "cluster_id")?,
        model_provider: take_required_form_string(&mut values, "model_provider")?,
        model_name: take_optional_form_string(&mut values, "model_name"),
        endpoint: take_optional_form_string(&mut values, "endpoint"),
        timeout_seconds: take_optional_form_u64(&mut values, "timeout_seconds", 1, 600)?,
        create_digest: take_required_form_bool(&mut values, "create_digest")?,
        cadence: take_required_form_string(&mut values, "cadence")?,
        status: take_required_form_string(&mut values, "status")?,
    })
}

pub(crate) fn parse_ops_knowledge_model_write_enqueue_form(
    body: &[u8],
) -> std::result::Result<OpsKnowledgeModelWriteEnqueueForm, HttpError> {
    let mut values = parse_ops_form_fields(
        body,
        &[
            "csrf_token",
            "idempotency_key",
            "cluster_id",
            "model_provider",
            "model_name",
            "endpoint",
            "timeout_seconds",
            "create_digest",
        ],
    )?;
    Ok(OpsKnowledgeModelWriteEnqueueForm {
        csrf_token: take_required_form_string(&mut values, "csrf_token")?,
        idempotency_key: take_required_form_string(&mut values, "idempotency_key")?,
        cluster_id: take_required_form_string(&mut values, "cluster_id")?,
        model_provider: take_required_form_string(&mut values, "model_provider")?,
        model_name: take_optional_form_string(&mut values, "model_name"),
        endpoint: take_optional_form_string(&mut values, "endpoint"),
        timeout_seconds: take_optional_form_u64(&mut values, "timeout_seconds", 1, 600)?,
        create_digest: take_required_form_bool(&mut values, "create_digest")?,
    })
}

pub(crate) fn parse_ops_knowledge_due_model_writes_form(
    body: &[u8],
) -> std::result::Result<OpsKnowledgeDueModelWritesForm, HttpError> {
    let mut values = parse_ops_form_fields(
        body,
        &[
            "csrf_token",
            "idempotency_key",
            "max_clusters",
            "model_provider",
            "model_name",
            "endpoint",
            "timeout_seconds",
            "create_digest",
        ],
    )?;
    Ok(OpsKnowledgeDueModelWritesForm {
        csrf_token: take_required_form_string(&mut values, "csrf_token")?,
        idempotency_key: take_required_form_string(&mut values, "idempotency_key")?,
        max_clusters: take_required_form_usize(&mut values, "max_clusters", 1, 100)?,
        model_provider: take_required_form_string(&mut values, "model_provider")?,
        model_name: take_optional_form_string(&mut values, "model_name"),
        endpoint: take_optional_form_string(&mut values, "endpoint"),
        timeout_seconds: take_optional_form_u64(&mut values, "timeout_seconds", 1, 600)?,
        create_digest: take_required_form_bool(&mut values, "create_digest")?,
    })
}

pub(crate) fn parse_ops_knowledge_entity_resolution_schedule_form(
    body: &[u8],
) -> std::result::Result<OpsKnowledgeEntityResolutionScheduleForm, HttpError> {
    let mut values = parse_ops_form_fields(
        body,
        &[
            "csrf_token",
            "idempotency_key",
            "model_provider",
            "model_name",
            "endpoint",
            "timeout_seconds",
            "max_pairs",
            "cadence",
            "status",
        ],
    )?;
    Ok(OpsKnowledgeEntityResolutionScheduleForm {
        csrf_token: take_required_form_string(&mut values, "csrf_token")?,
        idempotency_key: take_required_form_string(&mut values, "idempotency_key")?,
        model_provider: take_required_form_string(&mut values, "model_provider")?,
        model_name: take_optional_form_string(&mut values, "model_name"),
        endpoint: take_optional_form_string(&mut values, "endpoint"),
        timeout_seconds: take_optional_form_u64(&mut values, "timeout_seconds", 1, 600)?,
        max_pairs: take_required_form_usize(&mut values, "max_pairs", 1, 100)?,
        cadence: take_required_form_string(&mut values, "cadence")?,
        status: take_required_form_string(&mut values, "status")?,
    })
}

pub(crate) fn parse_ops_knowledge_entity_resolution_enqueue_form(
    body: &[u8],
) -> std::result::Result<OpsKnowledgeEntityResolutionEnqueueForm, HttpError> {
    let mut values = parse_ops_form_fields(
        body,
        &[
            "csrf_token",
            "idempotency_key",
            "model_provider",
            "model_name",
            "endpoint",
            "timeout_seconds",
            "max_pairs",
        ],
    )?;
    Ok(OpsKnowledgeEntityResolutionEnqueueForm {
        csrf_token: take_required_form_string(&mut values, "csrf_token")?,
        idempotency_key: take_required_form_string(&mut values, "idempotency_key")?,
        model_provider: take_required_form_string(&mut values, "model_provider")?,
        model_name: take_optional_form_string(&mut values, "model_name"),
        endpoint: take_optional_form_string(&mut values, "endpoint"),
        timeout_seconds: take_optional_form_u64(&mut values, "timeout_seconds", 1, 600)?,
        max_pairs: take_required_form_usize(&mut values, "max_pairs", 1, 100)?,
    })
}

pub(crate) fn parse_ops_knowledge_due_clusters_form(
    body: &[u8],
) -> std::result::Result<OpsKnowledgeDueClustersForm, HttpError> {
    let mut values =
        parse_ops_form_fields(body, &["csrf_token", "idempotency_key", "max_clusters"])?;
    Ok(OpsKnowledgeDueClustersForm {
        csrf_token: take_required_form_string(&mut values, "csrf_token")?,
        idempotency_key: take_required_form_string(&mut values, "idempotency_key")?,
        max_clusters: take_required_form_usize(&mut values, "max_clusters", 1, 100)?,
    })
}

pub(crate) fn parse_ops_knowledge_cluster_promote_form(
    body: &[u8],
) -> std::result::Result<OpsKnowledgeClusterPromoteForm, HttpError> {
    let mut values = parse_ops_form_fields(
        body,
        &[
            "csrf_token",
            "idempotency_key",
            "cluster_id",
            "reviewer",
            "reason",
        ],
    )?;
    Ok(OpsKnowledgeClusterPromoteForm {
        csrf_token: take_required_form_string(&mut values, "csrf_token")?,
        idempotency_key: take_required_form_string(&mut values, "idempotency_key")?,
        cluster_id: take_required_form_string(&mut values, "cluster_id")?,
        reviewer: take_required_form_string(&mut values, "reviewer")?,
        reason: take_required_form_string(&mut values, "reason")?,
    })
}

pub(crate) fn parse_ops_worker_run_once_form(
    body: &[u8],
) -> std::result::Result<OpsWorkerRunOnceForm, HttpError> {
    let mut values = parse_ops_form_fields(body, &["csrf_token", "idempotency_key", "max_jobs"])?;
    Ok(OpsWorkerRunOnceForm {
        csrf_token: take_required_form_string(&mut values, "csrf_token")?,
        idempotency_key: take_required_form_string(&mut values, "idempotency_key")?,
        max_jobs: take_required_form_usize(&mut values, "max_jobs", 1, 25)?,
    })
}

pub(crate) fn parse_ops_form_fields(
    body: &[u8],
    allowed_fields: &[&str],
) -> std::result::Result<BTreeMap<String, String>, HttpError> {
    let text = std::str::from_utf8(body).map_err(|_| {
        HttpError::bad_request("bad_form", "form body must be valid UTF-8 urlencoding")
    })?;
    let mut values = BTreeMap::<String, String>::new();
    for pair in text.split('&').filter(|pair| !pair.is_empty()) {
        let Some((raw_key, raw_value)) = pair.split_once('=') else {
            return Err(HttpError::bad_request(
                "bad_form",
                "form fields must use key=value encoding",
            ));
        };
        let key = percent_decode_form_component(raw_key)?;
        let value = percent_decode_form_component(raw_value)?;
        if !allowed_fields.contains(&key.as_str()) {
            return Err(HttpError::bad_request(
                "bad_form",
                format!("unsupported form field: {key}"),
            ));
        }
        if values.insert(key.clone(), value).is_some() {
            return Err(HttpError::bad_request(
                "bad_form",
                format!("duplicate form field: {key}"),
            ));
        }
    }
    Ok(values)
}

pub(crate) fn take_required_form_string(
    values: &mut BTreeMap<String, String>,
    key: &'static str,
) -> std::result::Result<String, HttpError> {
    values
        .remove(key)
        .ok_or_else(|| HttpError::bad_request("bad_form", format!("missing form field: {key}")))
}

pub(crate) fn take_required_form_i64(
    values: &mut BTreeMap<String, String>,
    key: &'static str,
    min: i64,
    max: i64,
) -> std::result::Result<i64, HttpError> {
    let value = take_required_form_string(values, key)?;
    let parsed = value.parse::<i64>().map_err(|_| {
        HttpError::bad_request("bad_form", format!("form field {key} must be an integer"))
    })?;
    if parsed < min || parsed > max {
        return Err(HttpError::bad_request(
            "bad_form",
            format!("form field {key} must be between {min} and {max}"),
        ));
    }
    Ok(parsed)
}

pub(crate) fn take_required_form_usize(
    values: &mut BTreeMap<String, String>,
    key: &'static str,
    min: usize,
    max: usize,
) -> std::result::Result<usize, HttpError> {
    let value = take_required_form_string(values, key)?;
    let parsed = value.parse::<usize>().map_err(|_| {
        HttpError::bad_request("bad_form", format!("form field {key} must be an integer"))
    })?;
    if parsed < min || parsed > max {
        return Err(HttpError::bad_request(
            "bad_form",
            format!("form field {key} must be between {min} and {max}"),
        ));
    }
    Ok(parsed)
}

pub(crate) fn take_optional_form_string(
    values: &mut BTreeMap<String, String>,
    key: &'static str,
) -> Option<String> {
    values
        .remove(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn take_optional_form_u64(
    values: &mut BTreeMap<String, String>,
    key: &'static str,
    min: u64,
    max: u64,
) -> std::result::Result<Option<u64>, HttpError> {
    let Some(value) = values.remove(key) else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let parsed = value.parse::<u64>().map_err(|_| {
        HttpError::bad_request("bad_form", format!("form field {key} must be an integer"))
    })?;
    if parsed < min || parsed > max {
        return Err(HttpError::bad_request(
            "bad_form",
            format!("form field {key} must be between {min} and {max}"),
        ));
    }
    Ok(Some(parsed))
}

pub(crate) fn take_required_form_bool(
    values: &mut BTreeMap<String, String>,
    key: &'static str,
) -> std::result::Result<bool, HttpError> {
    match take_required_form_string(values, key)?.as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(HttpError::bad_request(
            "bad_form",
            format!("form field {key} must be true or false"),
        )),
    }
}

pub(crate) fn validate_ops_x_schedule_word(value: &str, label: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 40 {
        bail!("{label} must be non-empty and at most 40 bytes");
    }
    if trimmed != value
        || !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
    {
        bail!("{label} may only contain ASCII letters, numbers, underscore, or hyphen");
    }
    Ok(trimmed.to_string())
}

pub(crate) fn percent_decode_form_component(value: &str) -> std::result::Result<String, HttpError> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' => {
                if index + 2 >= bytes.len() {
                    return Err(HttpError::bad_request(
                        "bad_form",
                        "form field contains truncated percent encoding",
                    ));
                }
                let high = hex_value(bytes[index + 1])?;
                let low = hex_value(bytes[index + 2])?;
                decoded.push((high << 4) | low);
                index += 3;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded)
        .map_err(|_| HttpError::bad_request("bad_form", "form field is not valid UTF-8"))
}

pub(crate) fn hex_value(byte: u8) -> std::result::Result<u8, HttpError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(HttpError::bad_request(
            "bad_form",
            "form field contains invalid percent encoding",
        )),
    }
}

pub(crate) fn is_dead_letterable_edge_status(status: &str) -> bool {
    matches!(status, "pending" | "failed" | "leased")
}

pub(crate) fn http_error_response(error: HttpError) -> Response {
    let message = redact_secret_like_text(&error.message);
    let mut response = (
        error.status,
        Json(json!({
            "ok": false,
            "error": {
                "type": error.kind,
                "message": message,
            }
        })),
    )
        .into_response();
    if error.status == StatusCode::UNAUTHORIZED {
        response.headers_mut().insert(
            header::WWW_AUTHENTICATE,
            HeaderValue::from_static(r#"Bearer realm="arcwell-local""#),
        );
    }
    with_http_security_headers(response)
}

pub(crate) fn redirect_to_ops_ui(location: &str) -> Response {
    let location =
        HeaderValue::from_str(location).unwrap_or_else(|_| HeaderValue::from_static("/ops/ui"));
    let mut response = (StatusCode::SEE_OTHER, "").into_response();
    response.headers_mut().insert(header::LOCATION, location);
    with_http_security_headers(response)
}

pub(crate) fn http_html_error_response(error: HttpError) -> Response {
    with_http_security_headers((error.status, Html(render_error_page(&error))).into_response())
}

pub(crate) fn with_http_security_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'"),
    );
    response
}

pub(crate) fn render_error_page(error: &HttpError) -> String {
    let message = redact_secret_like_text(&error.message);
    format!(
        r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><title>Arcwell Ops Error</title></head>
<body><h1>Arcwell Ops Error</h1><p>{}</p><pre>{}</pre></body>
</html>"#,
        html_escape(error.kind),
        html_escape(&message)
    )
}

pub(crate) fn redact_secret_like_text(value: &str) -> String {
    let mut redacted = value.to_string();
    for key in [
        "authorization",
        "api_key",
        "apikey",
        "access_token",
        "refresh_token",
        "token",
        "secret",
        "password",
    ] {
        redacted = redact_after_sensitive_key(&redacted, key);
    }
    for marker in ["Bearer ", "bearer "] {
        redacted = redact_after_marker(&redacted, marker);
    }
    for prefix in ["sk-", "ghp_", "github_pat_", "xoxb-", "xoxp-"] {
        redacted = redact_prefixed_token(&redacted, prefix);
    }
    redacted = redacted
        .split_whitespace()
        .map(redact_high_entropy_token)
        .collect::<Vec<_>>()
        .join(" ");
    redacted
}

pub(crate) fn redact_after_sensitive_key(value: &str, key: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let mut result = String::with_capacity(value.len());
    let mut cursor = 0;
    while let Some(relative_start) = lower[cursor..].find(key) {
        let key_start = cursor + relative_start;
        let key_end = key_start + key.len();
        result.push_str(&value[cursor..key_end]);

        let mut scan = key_end;
        while let Some(next) = value[scan..].chars().next()
            && next.is_ascii_whitespace()
        {
            result.push(next);
            scan += next.len_utf8();
        }
        let Some(separator) = value[scan..].chars().next() else {
            cursor = scan;
            break;
        };
        if !matches!(separator, ':' | '=') {
            cursor = scan;
            continue;
        }
        result.push(separator);
        scan += separator.len_utf8();
        while let Some(next) = value[scan..].chars().next()
            && next.is_ascii_whitespace()
        {
            result.push(next);
            scan += next.len_utf8();
        }

        let quote = value[scan..]
            .chars()
            .next()
            .filter(|next| matches!(next, '"' | '\''));
        if let Some(quote) = quote {
            result.push(quote);
            scan += quote.len_utf8();
        }
        result.push_str("[REDACTED]");
        while let Some(next) = value[scan..].chars().next() {
            let stop = if let Some(quote) = quote {
                next == quote
            } else {
                next.is_ascii_whitespace() || matches!(next, ',' | '&' | '<' | '>' | ';')
            };
            if stop {
                if quote.is_some() {
                    result.push(next);
                    scan += next.len_utf8();
                }
                break;
            }
            scan += next.len_utf8();
        }
        cursor = scan;
    }
    result.push_str(&value[cursor..]);
    result
}

pub(crate) fn redact_after_marker(value: &str, marker: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let marker_lower = marker.to_ascii_lowercase();
    let mut result = String::with_capacity(value.len());
    let mut cursor = 0;
    while let Some(relative_start) = lower[cursor..].find(&marker_lower) {
        let start = cursor + relative_start;
        let mut scan = start + marker.len();
        result.push_str(&value[cursor..scan]);
        result.push_str("[REDACTED]");
        while let Some(next) = value[scan..].chars().next() {
            if next.is_ascii_whitespace() || matches!(next, ',' | '&' | '<' | '>' | ';') {
                break;
            }
            scan += next.len_utf8();
        }
        cursor = scan;
    }
    result.push_str(&value[cursor..]);
    result
}

pub(crate) fn redact_prefixed_token(value: &str, prefix: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut cursor = 0;
    while let Some(relative_start) = value[cursor..].find(prefix) {
        let start = cursor + relative_start;
        let mut scan = start + prefix.len();
        result.push_str(&value[cursor..start]);
        result.push_str("[REDACTED]");
        while let Some(next) = value[scan..].chars().next() {
            if next.is_ascii_whitespace()
                || matches!(next, ',' | '&' | '<' | '>' | ';' | '"' | '\'')
            {
                break;
            }
            scan += next.len_utf8();
        }
        cursor = scan;
    }
    result.push_str(&value[cursor..]);
    result
}

pub(crate) fn redact_high_entropy_token(token: &str) -> String {
    let trimmed = token.trim_matches(|ch: char| {
        matches!(
            ch,
            '"' | '\'' | ',' | ';' | '<' | '>' | '(' | ')' | '[' | ']'
        )
    });
    if trimmed.len() < 32 {
        return token.to_string();
    }
    if trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '='))
    {
        return token.replace(trimmed, "[REDACTED]");
    }
    token.to_string()
}
