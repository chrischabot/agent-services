use super::*;

pub(crate) async fn serve(paths: AppPaths, args: ServeArgs) -> Result<()> {
    Store::open(paths.clone())?;
    let state = HttpState::new(
        paths,
        args.auth_token,
        args.max_uri_bytes,
        args.max_body_bytes,
    )?;
    if !args.addr.ip().is_loopback() && state.auth_token.is_none() {
        bail!("HTTP auth token is required when binding to a non-loopback address");
    }
    let app = Router::new()
        .route("/health", get(http_health).post(http_mutation_rejected))
        .route("/profile", get(http_profile).post(http_mutation_rejected))
        .route("/memory", get(http_memories).post(http_mutation_rejected))
        .route("/wiki", get(http_wiki).post(http_mutation_rejected))
        .route("/ops", get(http_ops).post(http_mutation_rejected))
        .route("/ops/ui", get(http_ops_ui).post(http_mutation_rejected))
        .route(
            "/ops/actions/edge-events/dead-letter",
            post(http_ops_edge_event_dead_letter),
        )
        .route(
            "/ops/actions/x/bookmarks/schedule",
            post(http_ops_x_bookmarks_schedule),
        )
        .route(
            "/ops/actions/x/bookmarks/enqueue",
            post(http_ops_x_bookmarks_enqueue),
        )
        .route(
            "/ops/actions/knowledge/backlog/schedule",
            post(http_ops_knowledge_backlog_schedule),
        )
        .route(
            "/ops/actions/knowledge/backlog/enqueue",
            post(http_ops_knowledge_backlog_enqueue),
        )
        .route(
            "/ops/actions/knowledge/model-clusters/schedule",
            post(http_ops_knowledge_model_clusters_schedule),
        )
        .route(
            "/ops/actions/knowledge/model-clusters/enqueue",
            post(http_ops_knowledge_model_clusters_enqueue),
        )
        .route(
            "/ops/actions/knowledge/model-writes/schedule",
            post(http_ops_knowledge_model_write_schedule),
        )
        .route(
            "/ops/actions/knowledge/model-writes/enqueue",
            post(http_ops_knowledge_model_write_enqueue),
        )
        .route(
            "/ops/actions/knowledge/clusters/enqueue-expansions",
            post(http_ops_knowledge_cluster_expansions_enqueue),
        )
        .route(
            "/ops/actions/knowledge/clusters/enqueue-editorial-decisions",
            post(http_ops_knowledge_cluster_editorial_decisions_enqueue),
        )
        .route(
            "/ops/actions/knowledge/model-writes/enqueue-due",
            post(http_ops_knowledge_model_writes_enqueue_due),
        )
        .route(
            "/ops/actions/knowledge/entity-resolution/schedule",
            post(http_ops_knowledge_entity_resolution_schedule),
        )
        .route(
            "/ops/actions/knowledge/entity-resolution/enqueue-due",
            post(http_ops_knowledge_entity_resolution_enqueue_due),
        )
        .route(
            "/ops/actions/knowledge/clusters/promote",
            post(http_ops_knowledge_cluster_promote),
        )
        .route(
            "/ops/actions/knowledge/investigations/enqueue-execution",
            post(http_ops_knowledge_investigation_execution_enqueue),
        )
        .route(
            "/ops/actions/worker/run-once",
            post(http_ops_worker_run_once),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(args.addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Clone)]
pub(crate) struct HttpState {
    pub(crate) paths: AppPaths,
    pub(crate) auth_token: Option<String>,
    pub(crate) max_uri_bytes: usize,
    pub(crate) max_body_bytes: u64,
    pub(crate) csrf_token: String,
    pub(crate) idempotency_keys: Arc<Mutex<BTreeSet<String>>>,
}

impl HttpState {
    pub(crate) fn new(
        paths: AppPaths,
        auth_token: Option<String>,
        max_uri_bytes: usize,
        max_body_bytes: u64,
    ) -> Result<Self> {
        if let Some(token) = &auth_token {
            let token = token.trim();
            if token.len() < 16 {
                bail!("HTTP auth token must be at least 16 characters");
            }
            if token.len() > 4096 {
                bail!("HTTP auth token is too long");
            }
            if token.chars().any(char::is_control) {
                bail!("HTTP auth token cannot contain control characters");
            }
        }
        Ok(Self {
            paths,
            auth_token,
            max_uri_bytes,
            max_body_bytes,
            csrf_token: Uuid::new_v4().to_string(),
            idempotency_keys: Arc::new(Mutex::new(BTreeSet::new())),
        })
    }
}

pub(crate) async fn http_health(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    json_response(&state, &headers, &uri, || {
        Ok(json!(Store::open(state.paths.clone())?.health()?))
    })
}

pub(crate) async fn http_profile(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    json_response(&state, &headers, &uri, || {
        Ok(json!(Store::open(state.paths.clone())?.list_profile()?))
    })
}

pub(crate) async fn http_memories(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    json_response(&state, &headers, &uri, || {
        Ok(json!(Store::open(state.paths.clone())?.list_memories(100)?))
    })
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct WikiQuery {
    pub(crate) q: Option<String>,
}

pub(crate) async fn http_wiki(
    State(state): State<HttpState>,
    headers: HeaderMap,
    uri: Uri,
    query: Result<Query<WikiQuery>, QueryRejection>,
) -> Response {
    let Query(query) = match query {
        Ok(query) => query,
        Err(error) => {
            return http_error_response(HttpError::bad_request("bad_query", error.to_string()));
        }
    };
    if let Some(q) = &query.q
        && q.len() > 4096
    {
        return http_error_response(HttpError::new(
            StatusCode::URI_TOO_LONG,
            "query_too_large",
            "query parameter q is too large",
        ));
    }
    json_response(&state, &headers, &uri, || {
        let store = Store::open(state.paths.clone())?;
        let pages = match query.q {
            Some(q) => store.search_wiki_pages(&q),
            None => store.list_wiki_pages(),
        }?;
        Ok(json!(pages))
    })
}
