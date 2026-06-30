use super::*;

pub(crate) fn fetch_text(url: &str, bearer_token: Option<&str>) -> Result<String> {
    fetch_text_with_user_agent(url, bearer_token, "arcwell/0.1")
}

pub(crate) fn fetch_text_with_user_agent(
    url: &str,
    bearer_token: Option<&str>,
    user_agent: &str,
) -> Result<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(Policy::none())
        .build()?;
    let mut request = client
        .get(url)
        .header(
            ACCEPT,
            "application/rss+xml, application/atom+xml, application/xml, text/xml, text/plain, */*",
        )
        .header("user-agent", user_agent);
    if let Some(token) = bearer_token {
        request = request.header(AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = request
        .send()
        .with_context(|| format!("fetch request failed: {url}"))?;
    let status = response.status();
    let retry_after = response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    if !status.is_success() {
        let text = response.text().unwrap_or_default();
        bail!(
            "{}",
            classify_provider_http_error("fetch", status, retry_after.as_deref(), &text)
        );
    }
    if let Some(length) = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        && length > FETCH_TEXT_MAX_BYTES
    {
        bail!("fetched body is too large");
    }
    let mut bytes = Vec::new();
    let mut limited = response.take(FETCH_TEXT_MAX_BYTES + 1);
    limited
        .read_to_end(&mut bytes)
        .with_context(|| format!("reading fetch response: {url}"))?;
    if bytes.len() > FETCH_TEXT_MAX_BYTES as usize {
        bail!("fetched body is too large");
    }
    String::from_utf8(bytes).with_context(|| format!("fetch returned invalid text: {url}"))
}

pub(crate) fn provider_user_agent(provider: &str) -> String {
    match provider {
        "reddit" => std::env::var("ARCWELL_REDDIT_USER_AGENT")
            .unwrap_or_else(|_| "macos:arcwell-local:v0.1 (by /u/arcwell-local)".to_string()),
        _ => "arcwell/0.1".to_string(),
    }
}

pub(crate) fn default_x_oauth_scopes() -> Vec<String> {
    [
        "tweet.read",
        "users.read",
        "bookmark.read",
        "follows.read",
        "offline.access",
    ]
    .iter()
    .map(|scope| (*scope).to_string())
    .collect()
}

pub(crate) fn provider_credential_probe_specs(
    providers: &[String],
) -> Result<Vec<ProviderCredentialProbeSpec>> {
    let mut selected = providers
        .iter()
        .flat_map(|provider| provider.split(','))
        .map(|provider| provider.trim().to_ascii_lowercase())
        .filter(|provider| !provider.is_empty())
        .collect::<Vec<_>>();
    if selected.is_empty() || selected.iter().any(|provider| provider == "all") {
        selected = vec![
            "github".to_string(),
            "openai".to_string(),
            "brave".to_string(),
            "cloudflare".to_string(),
        ];
    }
    let mut deduped = BTreeSet::new();
    let mut specs = Vec::new();
    for provider in selected {
        if !deduped.insert(provider.clone()) {
            continue;
        }
        let spec = match provider.as_str() {
            "github" => ProviderCredentialProbeSpec {
                provider,
                secret_names: vec!["GITHUB_TOKEN".to_string()],
                url: "https://api.github.com/user".to_string(),
                auth: ProviderProbeAuth::Bearer,
                evidence: ProviderProbeEvidence::GithubUser,
            },
            "openai" => ProviderCredentialProbeSpec {
                provider,
                secret_names: vec!["OPENAI_API_KEY".to_string()],
                url: "https://api.openai.com/v1/models".to_string(),
                auth: ProviderProbeAuth::Bearer,
                evidence: ProviderProbeEvidence::OpenAiModels,
            },
            "brave" => ProviderCredentialProbeSpec {
                provider,
                secret_names: vec![
                    "BRAVE_SEARCH_API_KEY".to_string(),
                    "BRAVE_API_KEY".to_string(),
                ],
                url: "https://api.search.brave.com/res/v1/web/search?q=arcwell&count=1".to_string(),
                auth: ProviderProbeAuth::BraveSearchToken,
                evidence: ProviderProbeEvidence::BraveSearch,
            },
            "cloudflare" => ProviderCredentialProbeSpec {
                provider,
                secret_names: vec!["CLOUDFLARE_API_TOKEN".to_string()],
                url: "https://api.cloudflare.com/client/v4/user/tokens/verify".to_string(),
                auth: ProviderProbeAuth::Bearer,
                evidence: ProviderProbeEvidence::CloudflareTokenVerify,
            },
            _ => bail!("unsupported provider credential probe: {provider}"),
        };
        specs.push(spec);
    }
    Ok(specs)
}

pub(crate) fn provider_probe_endpoint_label(url: &str) -> String {
    Url::parse(url)
        .ok()
        .and_then(|parsed| {
            let host = parsed.host_str()?;
            Some(format!("{host}{}", parsed.path()))
        })
        .unwrap_or_else(|| excerpt(url, 240))
}

pub(crate) fn fetch_provider_probe_json(
    spec: &ProviderCredentialProbeSpec,
    token: &str,
) -> Result<Value> {
    let client = Client::builder().timeout(Duration::from_secs(20)).build()?;
    let mut request = client
        .get(&spec.url)
        .header(ACCEPT, "application/json")
        .header("user-agent", provider_user_agent(&spec.provider));
    request = match spec.auth {
        ProviderProbeAuth::Bearer => request.header(AUTHORIZATION, format!("Bearer {token}")),
        ProviderProbeAuth::BraveSearchToken => request.header("X-Subscription-Token", token),
    };
    let response = request
        .send()
        .with_context(|| format!("{} credential probe request failed", spec.provider))?;
    let status = response.status();
    let retry_after = response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let text = response
        .text()
        .with_context(|| format!("{} returned unreadable probe response body", spec.provider))?;
    if !status.is_success() {
        bail!(
            "{}",
            classify_provider_http_error(&spec.provider, status, retry_after.as_deref(), &text)
        );
    }
    serde_json::from_str(&text)
        .with_context(|| format!("{} credential probe returned invalid JSON", spec.provider))
}

pub(crate) fn provider_probe_evidence_passes(
    spec: &ProviderCredentialProbeSpec,
    value: &Value,
) -> bool {
    match spec.evidence {
        ProviderProbeEvidence::GithubUser => {
            value.get("login").and_then(Value::as_str).is_some()
                || value.get("id").and_then(Value::as_i64).is_some()
        }
        ProviderProbeEvidence::OpenAiModels => {
            value.get("data").and_then(Value::as_array).is_some()
        }
        ProviderProbeEvidence::BraveSearch => {
            value.get("web").is_some() || value.get("query").is_some()
        }
        ProviderProbeEvidence::CloudflareTokenVerify => value
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        ProviderProbeEvidence::CloudflareAccount => {
            value
                .get("success")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && value
                    .pointer("/result/id")
                    .and_then(Value::as_str)
                    .is_some()
        }
    }
}

pub(crate) fn provider_probe_success_evidence(
    spec: &ProviderCredentialProbeSpec,
    value: &Value,
) -> String {
    match spec.evidence {
        ProviderProbeEvidence::GithubUser => {
            let login = value
                .get("login")
                .and_then(Value::as_str)
                .map(|value| excerpt(value, 80))
                .unwrap_or_else(|| "authenticated user".to_string());
            format!("provider accepted credential and returned GitHub user {login}")
        }
        ProviderProbeEvidence::OpenAiModels => {
            let count = value
                .get("data")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            format!(
                "provider accepted credential and returned OpenAI models list with {count} item(s)"
            )
        }
        ProviderProbeEvidence::BraveSearch => {
            "provider accepted credential and returned Brave Search response shape".to_string()
        }
        ProviderProbeEvidence::CloudflareTokenVerify => {
            "provider accepted credential and verified Cloudflare API token".to_string()
        }
        ProviderProbeEvidence::CloudflareAccount => {
            "provider accepted credential and returned Cloudflare account details".to_string()
        }
    }
}

pub(crate) fn classify_provider_probe_error(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if lower.contains("policy") || lower.contains("denied") {
        "policy_denied".to_string()
    } else if lower.contains("cost") || lower.contains("budget") {
        "cost_denied".to_string()
    } else if lower.contains("missing") || lower.contains("no usable") {
        "missing_secret".to_string()
    } else if lower.contains("token rejected")
        || lower.contains("expired")
        || lower.contains("unauthorized")
        || lower.contains("http 401")
        || lower.contains("forbidden")
        || lower.contains("http 403")
    {
        "provider_revocation_or_expiry".to_string()
    } else if lower.contains("rate limit")
        || lower.contains("quota")
        || lower.contains("too many requests")
        || lower.contains("http 429")
    {
        "quota_or_rate_limit".to_string()
    } else {
        "provider_network_failure".to_string()
    }
}

pub(crate) fn fetch_json(url: &str, bearer_token: Option<&str>, provider: &str) -> Result<Value> {
    let client = Client::builder().timeout(Duration::from_secs(20)).build()?;
    let mut request = client
        .get(url)
        .header(ACCEPT, "application/json")
        .header("user-agent", provider_user_agent(provider));
    if let Some(token) = bearer_token {
        request = request.header(AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = request
        .send()
        .with_context(|| format!("{provider} request failed"))?;
    let status = response.status();
    let retry_after = response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let text = response
        .text()
        .with_context(|| format!("{provider} returned unreadable response body"))?;
    if !status.is_success() {
        bail!(
            "{}",
            classify_provider_http_error(provider, status, retry_after.as_deref(), &text)
        );
    }
    serde_json::from_str(&text).with_context(|| format!("{provider} returned invalid JSON"))
}

pub(crate) fn classify_provider_http_error(
    provider: &str,
    status: StatusCode,
    retry_after: Option<&str>,
    body: &str,
) -> String {
    let body = redact_secret_like_text(body);
    let body_excerpt = excerpt(&body, 500);
    let mut reason = match status {
        StatusCode::TOO_MANY_REQUESTS => {
            format!("{provider} rate limit or quota exceeded; HTTP 429")
        }
        StatusCode::UNAUTHORIZED => format!("{provider} token rejected or expired; HTTP 401"),
        StatusCode::FORBIDDEN => format!("{provider} request forbidden; HTTP 403"),
        _ => format!("{provider} returned HTTP {}", status.as_u16()),
    };
    if let Some(retry_after) = retry_after
        && !retry_after.trim().is_empty()
    {
        reason.push_str(&format!("; retry_after={}", excerpt(retry_after, 120)));
    }
    if !body_excerpt.trim().is_empty() {
        reason.push_str(&format!("; provider_error={body_excerpt}"));
    }
    reason
}

pub(crate) fn fetch_x_json(url: &str, bearer_token: Option<&str>) -> Result<Value> {
    let client = Client::builder().timeout(Duration::from_secs(20)).build()?;
    let mut request = client
        .get(url)
        .header(ACCEPT, "application/json")
        .header("user-agent", "arcwell/0.1");
    if let Some(token) = bearer_token {
        request = request.header(AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = request.send().context("x request failed")?;
    let status = response.status();
    let retry_after = response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let text = response.text().unwrap_or_default();
    if !status.is_success() {
        bail!(
            "{}",
            classify_x_http_error(status, retry_after.as_deref(), &text)
        );
    }
    serde_json::from_str(&text).context("x returned invalid JSON")
}

#[derive(Debug, Clone)]
pub(crate) struct XApiMcpTool {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) input_schema: Value,
}

pub(crate) fn default_x_mcp_server_url(endpoint: &str) -> Result<String> {
    if let Ok(url) = std::env::var("ARCWELL_X_MCP_SERVER_URL")
        && !url.trim().is_empty()
    {
        validate_public_http_url(url.trim())?;
        return Ok(url.trim().to_string());
    }
    let base = validated_x_api_base(endpoint)?;
    Ok(base.join("/mcp")?.to_string())
}

pub(crate) fn fetch_x_mcp_tools(server_url: &str, bearer_token: &str) -> Result<Vec<XApiMcpTool>> {
    let result = x_mcp_json_rpc(
        server_url,
        bearer_token,
        "tools/list",
        json!({}),
        Some("x_mcp_tools_list"),
    )?;
    let tools = result
        .get("tools")
        .and_then(Value::as_array)
        .context("X MCP tools/list response missing tools array")?;
    let mut out = Vec::new();
    for tool in tools {
        let name = tool
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .context("X MCP tool missing name")?;
        out.push(XApiMcpTool {
            name: name.to_string(),
            description: tool
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            input_schema: tool
                .get("inputSchema")
                .or_else(|| tool.get("input_schema"))
                .cloned()
                .unwrap_or_else(|| json!({})),
        });
    }
    Ok(out)
}

pub(crate) fn call_x_mcp_tool(
    server_url: &str,
    bearer_token: &str,
    tool_name: &str,
    arguments: Value,
) -> Result<Value> {
    validate_key(tool_name)?;
    x_mcp_json_rpc(
        server_url,
        bearer_token,
        "tools/call",
        json!({
            "name": tool_name,
            "arguments": arguments,
        }),
        Some("x_mcp_tools_call"),
    )
}

pub(crate) fn select_x_mcp_tool_excluding(
    tools: &[XApiMcpTool],
    explicit_env: &str,
    label: &str,
    required_name_fragments: &[&str],
    excluded_name_fragments: &[&str],
) -> Result<XApiMcpTool> {
    if let Ok(name) = std::env::var(explicit_env)
        && !name.trim().is_empty()
    {
        let name = name.trim();
        return tools
            .iter()
            .find(|tool| tool.name == name)
            .cloned()
            .with_context(|| {
                format!("configured {explicit_env} tool {name:?} was not advertised by X MCP")
            });
    }
    tools
        .iter()
        .find(|tool| {
            let haystack = format!(
                "{} {}",
                tool.name.to_ascii_lowercase(),
                tool.description.to_ascii_lowercase()
            );
            required_name_fragments
                .iter()
                .all(|fragment| haystack.contains(fragment))
                && excluded_name_fragments
                    .iter()
                    .all(|fragment| !haystack.contains(fragment))
        })
        .cloned()
        .with_context(|| {
            format!(
                "X MCP tools/list did not advertise a usable {label} tool; set {explicit_env} after inspecting tools"
            )
        })
}

pub(crate) fn x_mcp_tool_accepts(tool: &XApiMcpTool, key: &str) -> bool {
    let Some(properties) = tool
        .input_schema
        .get("properties")
        .and_then(Value::as_object)
    else {
        return true;
    };
    properties.contains_key(key)
}

pub(crate) fn x_mcp_extract_x_api_response(value: &Value) -> Result<Value> {
    if value.get("status").and_then(Value::as_i64).is_some()
        && (value.get("detail").is_some() || value.get("title").is_some())
    {
        let status = value
            .get("status")
            .and_then(Value::as_i64)
            .map(|status| status.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let title = value
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("provider error");
        let detail = value.get("detail").and_then(Value::as_str).unwrap_or("");
        bail!(
            "X MCP tool returned provider error HTTP {status}: {}",
            excerpt(
                &redact_secret_like_text(&format!("{title}: {detail}")),
                1000
            )
        );
    }
    if value.get("data").and_then(Value::as_array).is_some() {
        return Ok(value.clone());
    }
    for key in [
        "structuredContent",
        "structured_content",
        "result",
        "response",
        "data",
        "json",
    ] {
        if let Some(nested) = value.get(key) {
            match x_mcp_extract_x_api_response(nested) {
                Ok(found) => return Ok(found),
                Err(error) if x_mcp_terminal_extract_error(&error) => return Err(error),
                Err(_) => {}
            }
        }
    }
    if let Some(content) = value.get("content").and_then(Value::as_array) {
        for item in content {
            match x_mcp_extract_x_api_response(item) {
                Ok(found) => return Ok(found),
                Err(error) if x_mcp_terminal_extract_error(&error) => return Err(error),
                Err(_) => {}
            }
            if let Some(text) = item.get("text").and_then(Value::as_str)
                && let Ok(parsed) = parse_x_mcp_text_json(text)
            {
                match x_mcp_extract_x_api_response(&parsed) {
                    Ok(found) => return Ok(found),
                    Err(error) if x_mcp_terminal_extract_error(&error) => return Err(error),
                    Err(_) => {}
                }
            }
        }
    }
    for array_key in ["tweets", "posts", "items", "results"] {
        if let Some(items) = value.get(array_key).and_then(Value::as_array) {
            return Ok(json!({
                "data": items,
                "includes": value.get("includes").cloned().unwrap_or_else(|| json!({})),
                "meta": value.get("meta").cloned().unwrap_or_else(|| json!({})),
            }));
        }
    }
    bail!("X MCP tool result did not contain an X API-shaped JSON response")
}

fn x_mcp_terminal_extract_error(error: &anyhow::Error) -> bool {
    let text = error.to_string();
    text.contains("X MCP tool returned provider error")
        || text.contains("x response contained provider error")
        || text.contains("X MCP JSON-RPC error")
}

pub(crate) fn x_mcp_extract_json_response(value: &Value) -> Result<Value> {
    for key in [
        "structuredContent",
        "structured_content",
        "result",
        "response",
        "json",
    ] {
        if let Some(nested) = value.get(key)
            && let Ok(found) = x_mcp_extract_json_response(nested)
        {
            return Ok(found);
        }
    }
    if let Some(content) = value.get("content").and_then(Value::as_array) {
        for item in content {
            if let Some(text) = item.get("text").and_then(Value::as_str)
                && let Ok(parsed) = parse_x_mcp_text_json(text)
            {
                return Ok(parsed);
            }
            if let Ok(found) = x_mcp_extract_json_response(item) {
                return Ok(found);
            }
        }
    }
    if value.is_object() {
        return Ok(value.clone());
    }
    bail!("X MCP tool result did not contain JSON object content")
}

fn x_mcp_json_rpc(
    server_url: &str,
    bearer_token: &str,
    method: &str,
    params: Value,
    source_label: Option<&str>,
) -> Result<Value> {
    validate_public_http_url(server_url)?;
    validate_oauth_param(bearer_token, "X MCP bearer token")?;
    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
    let mut session_id = None;
    let initialize = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "arcwell",
                "version": "0.1"
            }
        }
    });
    let _ = post_x_mcp_json(
        &client,
        server_url,
        bearer_token,
        &initialize,
        Some(1),
        &mut session_id,
        source_label,
    )?;
    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    });
    let _ = post_x_mcp_json(
        &client,
        server_url,
        bearer_token,
        &initialized,
        None,
        &mut session_id,
        source_label,
    )?;
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": method,
        "params": params,
    });
    post_x_mcp_json(
        &client,
        server_url,
        bearer_token,
        &request,
        Some(2),
        &mut session_id,
        source_label,
    )
}

fn post_x_mcp_json(
    client: &Client,
    server_url: &str,
    bearer_token: &str,
    payload: &Value,
    expected_id: Option<i64>,
    session_id: &mut Option<String>,
    source_label: Option<&str>,
) -> Result<Value> {
    let mut request = client
        .post(server_url)
        .header(ACCEPT, "application/json, text/event-stream")
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, format!("Bearer {bearer_token}"))
        .header("mcp-protocol-version", "2025-06-18")
        .header("user-agent", "arcwell/0.1")
        .json(payload);
    if let Some(session_id) = session_id.as_deref() {
        request = request.header("mcp-session-id", session_id);
    }
    let response = request
        .send()
        .with_context(|| format!("X MCP request failed: {server_url}"))?;
    if session_id.is_none()
        && let Some(value) = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
    {
        *session_id = Some(value.to_string());
    }
    let status = response.status();
    let retry_after = response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let text = response.text().unwrap_or_default();
    if !status.is_success() {
        let source = source_label.unwrap_or("x_mcp");
        bail!(
            "{}",
            classify_provider_http_error(source, status, retry_after.as_deref(), &text)
        );
    }
    if expected_id.is_none() || text.trim().is_empty() {
        return Ok(json!({}));
    }
    let message = if content_type.contains("text/event-stream")
        || text
            .lines()
            .any(|line| line.trim_start().starts_with("data:"))
    {
        parse_x_mcp_sse_json(&text, expected_id)?
    } else {
        serde_json::from_str(&text).context("X MCP returned invalid JSON-RPC response")?
    };
    x_mcp_json_rpc_result(&message, expected_id)
}

fn x_mcp_json_rpc_result(message: &Value, expected_id: Option<i64>) -> Result<Value> {
    if let Some(error) = message.get("error") {
        bail!(
            "X MCP JSON-RPC error: {}",
            excerpt(&redact_secret_like_text(&error.to_string()), 1000)
        );
    }
    if let Some(expected_id) = expected_id {
        let id_matches = message
            .get("id")
            .and_then(Value::as_i64)
            .is_some_and(|id| id == expected_id);
        if !id_matches {
            bail!("X MCP JSON-RPC response id mismatch");
        }
    }
    message
        .get("result")
        .cloned()
        .context("X MCP JSON-RPC response missing result")
}

fn parse_x_mcp_sse_json(text: &str, expected_id: Option<i64>) -> Result<Value> {
    let mut parse_errors = Vec::new();
    for line in text.lines() {
        let line = line.trim_start();
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        match serde_json::from_str::<Value>(data) {
            Ok(message) => {
                if expected_id.is_none()
                    || message
                        .get("id")
                        .and_then(Value::as_i64)
                        .is_some_and(|id| Some(id) == expected_id)
                {
                    return Ok(message);
                }
            }
            Err(error) => parse_errors.push(error.to_string()),
        }
    }
    bail!(
        "X MCP SSE response did not contain the expected JSON-RPC message{}",
        if parse_errors.is_empty() {
            String::new()
        } else {
            format!("; parse_errors={}", excerpt(&parse_errors.join("; "), 500))
        }
    )
}

fn parse_x_mcp_text_json(text: &str) -> Result<Value> {
    let mut text = text.trim();
    if let Some(stripped) = text.strip_prefix("```json") {
        text = stripped.trim();
        if let Some(stripped) = text.strip_suffix("```") {
            text = stripped.trim();
        }
    } else if let Some(stripped) = text.strip_prefix("```") {
        text = stripped.trim();
        if let Some(stripped) = text.strip_suffix("```") {
            text = stripped.trim();
        }
    }
    serde_json::from_str(text).context("X MCP text content was not JSON")
}

#[derive(Debug, Clone)]
pub(crate) struct XurlCommandSpec {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) source: String,
}

pub(crate) fn xurl_command_spec() -> XurlCommandSpec {
    if let Ok(bin) = std::env::var("ARCWELL_XURL_BIN")
        && !bin.trim().is_empty()
    {
        return XurlCommandSpec {
            program: bin.trim().to_string(),
            args: Vec::new(),
            source: "ARCWELL_XURL_BIN".to_string(),
        };
    }
    if command_appears_available("xurl") {
        return XurlCommandSpec {
            program: "xurl".to_string(),
            args: Vec::new(),
            source: "PATH:xurl".to_string(),
        };
    }
    let package = std::env::var("ARCWELL_XURL_NPX_PACKAGE")
        .unwrap_or_else(|_| "@xdevplatform/xurl".to_string());
    let mut args = vec!["-y".to_string()];
    if let Ok(registry) = std::env::var("ARCWELL_XURL_NPX_REGISTRY")
        && !registry.trim().is_empty()
    {
        args.push(format!("--registry={}", registry.trim()));
    }
    args.push(package.clone());
    XurlCommandSpec {
        program: "npx".to_string(),
        args,
        source: format!("npx:{package}"),
    }
}

pub(crate) fn command_appears_available(program: &str) -> bool {
    Command::new(program)
        .arg("version")
        .output()
        .is_ok_and(|output| output.status.success())
        || Command::new(program)
            .arg("--help")
            .output()
            .is_ok_and(|output| output.status.success())
}

pub(crate) fn xurl_token_command_args() -> Vec<String> {
    let mut args = vec!["token".to_string()];
    if let Ok(app) = std::env::var("ARCWELL_XURL_APP")
        && !app.trim().is_empty()
    {
        args.push("--app".to_string());
        args.push(app.trim().to_string());
    }
    if let Ok(username) = std::env::var("ARCWELL_XURL_USERNAME")
        && !username.trim().is_empty()
    {
        args.push("-u".to_string());
        args.push(username.trim().to_string());
    }
    args
}

pub(crate) fn run_xurl_token_command() -> Result<(String, XurlCommandSpec)> {
    let spec = xurl_command_spec();
    let mut args = spec.args.clone();
    args.extend(xurl_token_command_args());
    let output = Command::new(&spec.program)
        .args(&args)
        .output()
        .with_context(|| format!("starting xurl token command via {}", spec.source))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{} {}", stderr.trim(), stdout.trim());
        bail!(
            "xurl token command failed via {}: {}",
            spec.source,
            excerpt(&redact_secret_like_text(&combined), 1000)
        );
    }
    let token = String::from_utf8(output.stdout)
        .context("xurl token command returned non-UTF-8 output")?
        .trim()
        .to_string();
    validate_oauth_param(&token, "xurl OAuth2 access token")?;
    Ok((token, spec))
}

pub(crate) fn classify_x_http_error(
    status: StatusCode,
    retry_after: Option<&str>,
    body: &str,
) -> String {
    let body = redact_secret_like_text(body);
    let body_excerpt = excerpt(&body, 500);
    let mut reason = match status {
        StatusCode::UNAUTHORIZED => {
            "x token rejected or expired; refresh OAuth token before retry".to_string()
        }
        StatusCode::FORBIDDEN => {
            let lower = body_excerpt.to_ascii_lowercase();
            if lower.contains("client-not-enrolled")
                || lower.contains("unsupported")
                || lower.contains("tier")
                || lower.contains("access")
            {
                "x API access tier does not allow this endpoint".to_string()
            } else {
                "x request forbidden; source may be protected, blocked, deleted, or out of scope"
                    .to_string()
            }
        }
        StatusCode::TOO_MANY_REQUESTS => "x rate limit or quota exceeded".to_string(),
        _ => format!("x returned HTTP {}", status.as_u16()),
    };
    if let Some(retry_after) = retry_after
        && !retry_after.trim().is_empty()
    {
        reason.push_str(&format!("; retry_after={}", excerpt(retry_after, 120)));
    }
    if !body_excerpt.trim().is_empty() {
        reason.push_str(&format!("; provider_error={body_excerpt}"));
    }
    reason
}

pub(crate) fn x_probe_collection_response_is_valid(value: &Value) -> bool {
    value.get("data").and_then(Value::as_array).is_some()
        || value.get("meta").and_then(Value::as_object).is_some()
}

pub(crate) fn classify_x_probe_error(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if lower.contains("token rejected")
        || lower.contains("expired")
        || lower.contains("invalid_grant")
        || lower.contains("revok")
        || lower.contains("unauthorized")
        || lower.contains("http 401")
    {
        "provider_revocation_or_expiry".to_string()
    } else if lower.contains("scope")
        || lower.contains("bookmark.read")
        || lower.contains("follows.read")
        || lower.contains("tweet.read")
        || lower.contains("users.read")
    {
        "scope_mismatch".to_string()
    } else if lower.contains("tier")
        || lower.contains("access tier")
        || lower.contains("client-not-enrolled")
        || lower.contains("unsupported authentication")
        || lower.contains("does not allow this endpoint")
        || lower.contains("http 403")
    {
        "provider_tier_or_endpoint_denial".to_string()
    } else if lower.contains("rate limit")
        || lower.contains("quota")
        || lower.contains("too many requests")
        || lower.contains("http 429")
    {
        "quota_tier_denial".to_string()
    } else if lower.contains("missing") || lower.contains("required") || lower.contains("not found")
    {
        "missing_refresh_material".to_string()
    } else {
        "provider_network_failure".to_string()
    }
}

pub(crate) fn x_oauth_probe_failed_endpoint(
    name: &str,
    required_scope: &str,
    path: &str,
    error: anyhow::Error,
) -> XOAuthScopeProbeEndpoint {
    let error = redact_secret_like_text(&error.to_string());
    XOAuthScopeProbeEndpoint {
        name: name.to_string(),
        required_scope: required_scope.to_string(),
        path: path.to_string(),
        status: "failed".to_string(),
        classification: classify_x_probe_error(&error),
        evidence: "provider did not accept this endpoint with the current bearer token".to_string(),
        error: Some(excerpt(&error, 1000)),
    }
}

pub(crate) fn x_fail_on_response_errors(value: &Value) -> Result<()> {
    if value.get("status").and_then(Value::as_i64).is_some()
        && (value.get("detail").is_some() || value.get("title").is_some())
    {
        let status = value
            .get("status")
            .and_then(Value::as_i64)
            .map(|status| status.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let title = value
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("provider error");
        let detail = value.get("detail").and_then(Value::as_str).unwrap_or("");
        bail!(
            "x response contained provider error HTTP {status}; cursor was not advanced: {}",
            excerpt(
                &redact_secret_like_text(&format!("{title}: {detail}")),
                1000
            )
        );
    }
    let Some(errors) = value.get("errors").and_then(Value::as_array) else {
        return Ok(());
    };
    if errors.is_empty() {
        return Ok(());
    }
    let error_text = errors
        .iter()
        .take(5)
        .map(|error| {
            let title = error
                .get("title")
                .and_then(Value::as_str)
                .or_else(|| error.get("type").and_then(Value::as_str))
                .unwrap_or("x partial error");
            let detail = error
                .get("detail")
                .and_then(Value::as_str)
                .or_else(|| error.get("message").and_then(Value::as_str))
                .unwrap_or("");
            format!("{title}: {detail}")
        })
        .collect::<Vec<_>>()
        .join("; ");
    bail!(
        "x response contained blocked/protected/deleted or partial-error items; cursor was not advanced: {}",
        excerpt(&redact_secret_like_text(&error_text), 1000)
    )
}

pub(crate) fn x_effective_cursor(previous: Option<&str>, newest: Option<&str>) -> Option<String> {
    match (previous, newest) {
        (None, None) => None,
        (Some(previous), None) => Some(previous.to_string()),
        (None, Some(newest)) => Some(newest.to_string()),
        (Some(previous), Some(newest)) => {
            if x_id_is_newer(newest, previous) {
                Some(newest.to_string())
            } else {
                Some(previous.to_string())
            }
        }
    }
}

pub(crate) fn x_id_is_newer(candidate: &str, previous: &str) -> bool {
    match (candidate.parse::<u128>(), previous.parse::<u128>()) {
        (Ok(candidate), Ok(previous)) => candidate > previous,
        _ => candidate > previous,
    }
}

pub(crate) fn x_failure_should_release_budget(error: &anyhow::Error) -> bool {
    let text = error.to_string().to_ascii_lowercase();
    text.contains("x_bearer_token is required")
        || text.contains("x_refresh_token is required")
        || text.contains("x_client_id is required")
        || text.contains("refreshing expired x_bearer_token failed")
        || text.contains("budget blocked x oauth refresh")
        || text.contains("policy denied provider.oauth")
        || text.contains("expired")
        || text.contains("token rejected")
        || text.contains("rate limit")
        || text.contains("quota exceeded")
        || text.contains("access tier")
        || text.contains("does not allow this endpoint")
}

pub(crate) fn post_x_oauth_form(
    endpoint: &str,
    client_id: &str,
    client_secret: Option<&str>,
    form: &[(&str, &str)],
) -> Result<Value> {
    let base = validated_x_api_base(endpoint)?;
    let url = base.join("/2/oauth2/token")?;
    post_x_oauth_json_form(url, client_id, client_secret, form)
}

pub(crate) fn post_x_oauth_json_form(
    url: Url,
    client_id: &str,
    client_secret: Option<&str>,
    form: &[(&str, &str)],
) -> Result<Value> {
    let client = Client::builder().timeout(Duration::from_secs(20)).build()?;
    let mut request = client
        .post(url)
        .header(ACCEPT, "application/json")
        .header("user-agent", "arcwell/0.1")
        .form(form);
    if let Some(client_secret) = client_secret {
        request = request.basic_auth(client_id, Some(client_secret));
    }
    let response = request.send().context("X OAuth token request failed")?;
    let status = response.status();
    let text = response.text().unwrap_or_default();
    if !status.is_success() {
        bail!(
            "X OAuth token endpoint failed: {}",
            classify_x_http_error(status, None, &text)
        );
    }
    serde_json::from_str(&text).context("X OAuth token endpoint returned invalid JSON")
}

pub(crate) fn post_x_oauth_revoke_form(
    endpoint: &str,
    client_id: &str,
    client_secret: Option<&str>,
    form: &[(&str, &str)],
) -> Result<u16> {
    let base = validated_x_api_base(endpoint)?;
    let url = base.join("/2/oauth2/revoke")?;
    let client = Client::builder().timeout(Duration::from_secs(20)).build()?;
    let mut request = client
        .post(url)
        .header(ACCEPT, "application/json")
        .header("user-agent", "arcwell/0.1")
        .form(form);
    if let Some(client_secret) = client_secret {
        request = request.basic_auth(client_id, Some(client_secret));
    }
    let response = request.send().context("X OAuth revoke request failed")?;
    let status = response.status();
    let text = response.text().unwrap_or_default();
    if !status.is_success() {
        bail!(
            "X OAuth revoke endpoint failed: {}",
            classify_x_http_error(status, None, &text)
        );
    }
    Ok(status.as_u16())
}
