use super::*;

#[test]
fn x_oauth_exchange_and_refresh_store_tokens_without_echoing_values() {
    let store = test_store("x-oauth");
    let long_access_token = format!("access-{}", "a".repeat(240));
    let long_refresh_token = format!("refresh-{}", "r".repeat(240));
    let exchange_body = Box::leak(
        json!({
            "token_type": "bearer",
            "expires_in": 7200,
            "scope": "tweet.read users.read offline.access",
            "access_token": long_access_token,
            "refresh_token": long_refresh_token
        })
        .to_string()
        .into_boxed_str(),
    );
    let exchange_base = mock_base_server(exchange_body, "application/json");

    let exchange = store
        .x_oauth_exchange_code_with_base(
            "client-id",
            "http://127.0.0.1/callback",
            &format!("code-{}", "c".repeat(240)),
            &format!("verifier-{}", "v".repeat(240)),
            Some("client-secret"),
            false,
            &exchange_base,
        )
        .unwrap();
    let exchange_json = serde_json::to_string(&exchange).unwrap();
    assert_eq!(
        exchange.stored,
        vec!["X_BEARER_TOKEN".to_string(), "X_REFRESH_TOKEN".to_string()]
    );
    assert!(!exchange_json.contains("access-"));
    assert!(!exchange_json.contains("refresh-"));
    assert!(
        store
            .get_secret_value("X_BEARER_TOKEN")
            .unwrap()
            .unwrap()
            .starts_with("access-")
    );

    let refresh_body = Box::leak(
        json!({
            "token_type": "bearer",
            "expires_in": 7200,
            "access_token": "fresh-access-token",
            "refresh_token": "fresh-refresh-token"
        })
        .to_string()
        .into_boxed_str(),
    );
    let refresh_base = mock_base_server(refresh_body, "application/json");
    let refresh = store
        .x_oauth_refresh_with_base("client-id", None, true, &refresh_base)
        .unwrap();
    let refresh_json = serde_json::to_string(&refresh).unwrap();
    assert!(!refresh_json.contains("fresh-access-token"));
    assert!(!refresh_json.contains("fresh-refresh-token"));
    assert_eq!(
        store.get_secret_value("X_BEARER_TOKEN").unwrap().as_deref(),
        Some("fresh-access-token")
    );
}

#[test]
fn gmail_oauth_exchange_and_refresh_store_tokens_without_echoing_values() {
    // CLAIM: Gmail OAuth can establish and refresh the local verifier
    // credential without printing or returning raw token values.
    let store = test_store("gmail-oauth");
    write_policy(
        &store,
        r#"
[[rules]]
id = "allow-gmail-oauth"
effect = "allow"
action = "provider.oauth"
package = "arcwell-email"
provider = "gmail"
source = "gmail_oauth"
reason = "allow Gmail OAuth test"
priority = 10
"#,
    );
    let long_access_token = format!("gmail-access-{}", "a".repeat(240));
    let long_refresh_token = format!("gmail-refresh-{}", "r".repeat(240));
    let exchange_body = Box::leak(
        json!({
            "token_type": "Bearer",
            "expires_in": 3600,
            "scope": "https://www.googleapis.com/auth/gmail.readonly",
            "access_token": long_access_token,
            "refresh_token": long_refresh_token
        })
        .to_string()
        .into_boxed_str(),
    );
    let exchange_base = mock_base_server(exchange_body, "application/json");

    let exchange = store
        .gmail_oauth_exchange_code_with_base(
            "client-id.apps.googleusercontent.com",
            "http://127.0.0.1:8766/callback",
            &format!("code-{}", "c".repeat(240)),
            &format!("verifier-{}", "v".repeat(240)),
            Some("client-secret"),
            &exchange_base,
        )
        .unwrap();
    let exchange_json = serde_json::to_string(&exchange).unwrap();
    assert_eq!(
        exchange.stored,
        vec![
            "GMAIL_ACCESS_TOKEN".to_string(),
            "GMAIL_REFRESH_TOKEN".to_string()
        ]
    );
    assert!(!exchange_json.contains("gmail-access-"));
    assert!(!exchange_json.contains("gmail-refresh-"));
    assert!(
        store
            .get_secret_value("GMAIL_ACCESS_TOKEN")
            .unwrap()
            .unwrap()
            .starts_with("gmail-access-")
    );

    let refresh_body = Box::leak(
        json!({
            "token_type": "Bearer",
            "expires_in": 3600,
            "access_token": "fresh-gmail-access-token",
            "refresh_token": "fresh-gmail-refresh-token"
        })
        .to_string()
        .into_boxed_str(),
    );
    let refresh_base = mock_base_server(refresh_body, "application/json");
    let refresh = store
        .gmail_oauth_refresh_with_base("client-id.apps.googleusercontent.com", None, &refresh_base)
        .unwrap();
    let refresh_json = serde_json::to_string(&refresh).unwrap();
    assert!(!refresh_json.contains("fresh-gmail-access-token"));
    assert!(!refresh_json.contains("fresh-gmail-refresh-token"));
    assert_eq!(
        store
            .get_secret_value("GMAIL_ACCESS_TOKEN")
            .unwrap()
            .as_deref(),
        Some("fresh-gmail-access-token")
    );
}

#[test]
fn severe_gmail_oauth_authorize_url_uses_pkce_and_mailbox_scopes() {
    // CLAIM: the Gmail verifier/repair credential asks for the mailbox API
    // surface it needs and uses PKCE/offline access so the worker can later
    // refresh without storing browser session material.
    let store = test_store("gmail-oauth-url");
    let start = store
        .gmail_oauth_authorize_url(
            "client-id.apps.googleusercontent.com",
            "http://127.0.0.1:8766/callback",
            &[],
        )
        .unwrap();
    assert!(
        start
            .authorization_url
            .contains("code_challenge_method=S256")
    );
    assert!(start.authorization_url.contains("access_type=offline"));
    assert!(start.authorization_url.contains("prompt=consent"));
    assert!(
        start
            .scopes
            .iter()
            .any(|scope| scope == "https://www.googleapis.com/auth/gmail.readonly")
    );
    assert!(
        start
            .scopes
            .iter()
            .any(|scope| scope == "https://www.googleapis.com/auth/gmail.modify")
    );
    assert_eq!(start.code_verifier.len(), 64);
    assert!(!start.code_challenge.is_empty());
}

#[test]
fn severe_gmail_access_token_auto_refreshes_from_stored_refresh_material() {
    // CLAIM: daemon-owned Gmail mailbox verification/repair does not require
    // repeated manual access-token pasting when stored refresh/client material
    // can refresh and write back a new access token.
    // ORACLE: an expired GMAIL_ACCESS_TOKEN is replaced through the configured
    // OAuth endpoint before use, and returned/stored reports omit token values.
    // SEVERITY: Severe because scheduled mailbox verification is hollow if it
    // works only until the first short-lived Google access token expires.
    let store = test_store("gmail-access-auto-refresh");
    let old_access = format!("old-gmail-access-{}", "a".repeat(48));
    let refresh = format!("refresh-gmail-{}", "r".repeat(48));
    let fresh_access = "fresh-gmail-access-ffffffffffffffffffffffffffffffffffffffffffffffff";
    let expired = (Utc::now() - ChronoDuration::minutes(5)).to_rfc3339();
    let (oauth_base, requests) = mock_recording_sequence_server(vec![(
        "200 OK",
        "",
        r#"{
            "access_token": "fresh-gmail-access-ffffffffffffffffffffffffffffffffffffffffffffffff",
            "expires_in": 3600,
            "scope": "https://www.googleapis.com/auth/gmail.readonly https://www.googleapis.com/auth/gmail.modify",
            "token_type": "Bearer"
        }"#,
        "application/json",
    )]);
    write_policy(
        &store,
        r#"
[[rules]]
id = "allow-gmail-oauth-refresh"
effect = "allow"
action = "provider.oauth"
package = "arcwell-email"
provider = "gmail"
source = "gmail_oauth"
reason = "allow managed Gmail OAuth refresh"
priority = 20
"#,
    );
    store
        .set_secret_value_with_metadata(
            "GMAIL_ACCESS_TOKEN",
            &old_access,
            "gmail",
            Some("gmail"),
            Some(&expired),
        )
        .unwrap();
    store
        .set_secret_value_with_metadata(
            "GMAIL_REFRESH_TOKEN",
            &refresh,
            "gmail",
            Some("gmail"),
            None,
        )
        .unwrap();
    store
        .set_secret_value_with_metadata(
            "GMAIL_CLIENT_ID",
            "client-id.apps.googleusercontent.com",
            "gmail",
            Some("gmail"),
            None,
        )
        .unwrap();

    let token = store
        .configured_gmail_access_token_with_oauth_base(Some(&oauth_base))
        .unwrap()
        .expect("refreshed token");
    assert_eq!(token, fresh_access);
    assert_eq!(
        store
            .get_secret_value("GMAIL_ACCESS_TOKEN")
            .unwrap()
            .as_deref(),
        Some(fresh_access)
    );
    let stored = store.list_secret_values().unwrap();
    let access_health = stored
        .iter()
        .find(|item| item.name == "GMAIL_ACCESS_TOKEN")
        .expect("stored access token metadata");
    assert_eq!(access_health.provider.as_deref(), Some("gmail"));
    assert!(access_health.expires_at.is_some());

    let captured = requests.lock().unwrap();
    assert_eq!(captured.len(), 1, "{captured:#?}");
    assert!(captured[0].contains("grant_type=refresh_token"));
    assert!(captured[0].contains("client_id=client-id.apps.googleusercontent.com"));
    assert!(captured[0].contains("refresh_token="));

    let serialized = serde_json::to_string(&json!({
        "stored": stored,
        "secret_health": store.secret_health().unwrap(),
    }))
    .unwrap();
    assert!(!serialized.contains(&old_access), "{serialized}");
    assert!(!serialized.contains(&refresh), "{serialized}");
    assert!(!serialized.contains(fresh_access), "{serialized}");
}

#[test]
fn severe_policy_denied_gmail_oauth_blocks_before_secret_or_cost_mutation() {
    // CLAIM: Gmail OAuth exchange requires provider.oauth policy before token
    // storage, credential lookup, network exchange, or cost reservation.
    let store = test_store("policy-gmail-oauth-deny");
    write_policy(
        &store,
        r#"
[[rules]]
id = "deny-gmail-oauth"
effect = "deny"
action = "provider.oauth"
provider = "gmail"
source = "gmail_oauth"
reason = "Gmail OAuth disabled during policy test"
"#,
    );

    let error = store
        .gmail_oauth_exchange_code_with_base(
            "client-id.apps.googleusercontent.com",
            "http://127.0.0.1:8766/callback",
            "authorization-code",
            "code-verifier",
            Some("explicit-client-secret"),
            "https://oauth2.googleapis.com",
        )
        .unwrap_err()
        .to_string();
    assert!(error.contains("policy denied provider.oauth"), "{error}");
    assert!(store.list_secret_values().unwrap().is_empty());
    assert_eq!(store.cost_summary().unwrap().2, 0);

    let decisions = store.list_policy_decisions(10).unwrap();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].action, "provider.oauth");
    assert_eq!(decisions[0].effect, "deny");
}

#[test]
fn severe_x_oauth_probe_proves_each_scope_endpoint_and_writes_ledgers() {
    // CLAIM: X OAuth scope probing proves each required user-context scope
    // by reaching a matching provider endpoint, not by trusting stored scope
    // metadata or a single /users/me response.
    // PRECONDITIONS: A bearer token is available and the provider accepts
    // users/me, bookmarks, following, and recent-search probes.
    // POSTCONDITIONS: the report passes all required scopes, writes healthy
    // source health, records a completed x_sync_run, and does not leak token
    // bytes in serialized output.
    // ORACLE: loopback provider request paths plus source_health/x_sync_runs.
    // SEVERITY: Severe because provider-scope claims are otherwise easy to
    // fake with stale token-response metadata.
    clear_x_bearer_env();
    let store = test_store("x-oauth-probe-pass");
    let token = format!("probe-token-{}", "p".repeat(64));
    store
        .set_secret_value_with_metadata("X_BEARER_TOKEN", &token, "x", Some("x"), None)
        .unwrap();
    write_policy(
        &store,
        r#"
[[rules]]
id = "allow-x-oauth-probe-network"
effect = "allow"
action = "provider.network"
package = "arcwell-x"
provider = "x"
source = "x_oauth_probe"
reason = "allow local X OAuth probe fixture"
priority = 20
"#,
    );
    let (base, requests) = mock_recording_sequence_server(vec![
        (
            "200 OK",
            "",
            r#"{"data":{"id":"u1","username":"arcwell_probe","name":"Arcwell Probe"}}"#,
            "application/json",
        ),
        (
            "200 OK",
            "",
            r#"{"data":[],"meta":{"result_count":0}}"#,
            "application/json",
        ),
        (
            "200 OK",
            "",
            r#"{"data":[],"meta":{"result_count":0}}"#,
            "application/json",
        ),
        (
            "200 OK",
            "",
            r#"{"data":[],"meta":{"result_count":0}}"#,
            "application/json",
        ),
    ]);

    let report = store.x_oauth_probe_with_base("from:openai", &base).unwrap();
    assert_eq!(report.status, "passed");
    assert_eq!(report.account_id.as_deref(), Some("u1"));
    assert!(report.missing_or_unproven_scopes.is_empty(), "{report:?}");
    assert_eq!(
        report
            .endpoints
            .iter()
            .map(|endpoint| endpoint.required_scope.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["bookmark.read", "follows.read", "tweet.read", "users.read"])
    );
    assert!(
        report
            .endpoints
            .iter()
            .all(|endpoint| endpoint.status == "passed"),
        "{report:?}"
    );
    let captured = requests.lock().unwrap().join("\n");
    assert!(captured.contains("GET /2/users/me?"), "{captured}");
    assert!(
        captured.contains("GET /2/users/u1/bookmarks?"),
        "{captured}"
    );
    assert!(
        captured.contains("GET /2/users/u1/following?"),
        "{captured}"
    );
    assert!(
        captured.contains("GET /2/tweets/search/recent?"),
        "{captured}"
    );
    assert!(
        captured.contains("authorization: Bearer probe-token-"),
        "{captured}"
    );

    let health = store
        .get_source_health("x:oauth-scope-probe")
        .unwrap()
        .expect("probe health");
    assert_eq!(health.status, "healthy");
    assert_eq!(health.last_item_id.as_deref(), Some("u1"));
    let sync: (String, String) = store
        .conn
        .query_row(
            "SELECT status, metadata_json FROM x_sync_runs WHERE id = ?1",
            params![report.sync_run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(sync.0, "completed");
    assert!(sync.1.contains("bookmark.read"), "{}", sync.1);
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains(&token));
    assert!(!sync.1.contains(&token));
}

#[test]
fn severe_x_oauth_probe_keeps_partial_scope_failures_visible_and_redacted() {
    // CLAIM: X OAuth scope probing must not call credentials healthy when
    // only /users/me succeeds; every unproven endpoint must stay visible in
    // report, source health, and x_sync_runs.
    // PRECONDITIONS: /users/me succeeds but bookmarks/following/recent
    // search fail with scope/tier/revocation-style provider errors.
    // POSTCONDITIONS: the report is partial, missing scopes are explicit,
    // source health is failed, sync-run status is failed, and raw bearer
    // text is not serialized.
    // ORACLE: report fields plus durable source_health/x_sync_runs rows.
    // SEVERITY: Severe because single-endpoint provider probes are a
    // classic false-green for scheduled bookmark/following ingestion.
    clear_x_bearer_env();
    let store = test_store("x-oauth-probe-partial");
    let token = format!("probe-token-{}", "s".repeat(64));
    store
        .set_secret_value_with_metadata("X_BEARER_TOKEN", &token, "x", Some("x"), None)
        .unwrap();
    write_policy(
        &store,
        r#"
[[rules]]
id = "allow-x-oauth-probe-network"
effect = "allow"
action = "provider.network"
package = "arcwell-x"
provider = "x"
source = "x_oauth_probe"
reason = "allow local X OAuth probe fixture"
priority = 20
"#,
    );
    let (base, _requests) = mock_recording_sequence_server(vec![
        (
            "200 OK",
            "",
            r#"{"data":{"id":"u1","username":"arcwell_probe","name":"Arcwell Probe"}}"#,
            "application/json",
        ),
        (
            "403 Forbidden",
            "",
            r#"{"title":"Unsupported Authentication","detail":"bookmark.read scope required"}"#,
            "application/json",
        ),
        (
            "403 Forbidden",
            "",
            r#"{"title":"Forbidden","detail":"follows.read scope required"}"#,
            "application/json",
        ),
        (
            "401 Unauthorized",
            "",
            r#"{"error":"invalid_token","error_description":"revoked access token probe-token-ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss"}"#,
            "application/json",
        ),
    ]);

    let report = store.x_oauth_probe_with_base("from:openai", &base).unwrap();
    assert_eq!(report.status, "partial");
    assert_eq!(
        report.missing_or_unproven_scopes,
        vec![
            "bookmark.read".to_string(),
            "follows.read".to_string(),
            "tweet.read".to_string()
        ]
    );
    let by_name = report
        .endpoints
        .iter()
        .map(|endpoint| (endpoint.name.as_str(), endpoint))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(by_name["users_me"].status, "passed");
    assert_eq!(by_name["bookmarks"].classification, "scope_mismatch");
    assert_eq!(by_name["following"].classification, "scope_mismatch");
    assert_eq!(
        by_name["recent_search"].classification,
        "provider_revocation_or_expiry"
    );

    let health = store
        .get_source_health("x:oauth-scope-probe")
        .unwrap()
        .expect("probe health");
    assert_eq!(health.status, "failed");
    assert!(
        health
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("bookmarks:scope_mismatch"),
        "{health:?}"
    );
    let sync: (String, String, String) = store
        .conn
        .query_row(
            "SELECT status, COALESCE(error, ''), metadata_json FROM x_sync_runs WHERE id = ?1",
            params![report.sync_run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(sync.0, "failed");
    assert!(sync.1.contains("bookmark.read"), "{}", sync.1);
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains(&token));
    assert!(!sync.2.contains(&token));
    assert!(!health.last_error.unwrap_or_default().contains(&token));
}

#[test]
fn severe_provider_credential_probe_checks_each_provider_and_writes_health() {
    // CLAIM: provider credential probing proves each selected provider by
    // reaching that provider's cheap credential endpoint, not by trusting
    // local secret presence.
    // PRECONDITIONS: four provider secrets are present and policy allows
    // each probe against a loopback provider fixture.
    // POSTCONDITIONS: every provider passes, source_health has one healthy
    // row per provider, request headers use the provider's expected auth
    // shape, and serialized output does not leak token values.
    // ORACLE: recorded HTTP request paths/headers plus source_health rows.
    // SEVERITY: Severe because a fake probe that only lists secrets would
    // miss revoked tokens and broken provider auth shapes.
    let store = test_store("provider-credential-probe-pass");
    let github_token = format!("ghp_{}", "g".repeat(80));
    let openai_token = format!("sk-{}", "o".repeat(80));
    let brave_token = format!("brave-{}", "b".repeat(80));
    let cloudflare_token = format!("cf-{}", "c".repeat(80));
    for (name, value, provider) in [
        ("GITHUB_TOKEN", github_token.as_str(), "github"),
        ("OPENAI_API_KEY", openai_token.as_str(), "openai"),
        ("BRAVE_SEARCH_API_KEY", brave_token.as_str(), "brave"),
        (
            "CLOUDFLARE_API_TOKEN",
            cloudflare_token.as_str(),
            "cloudflare",
        ),
    ] {
        store
            .set_secret_value_with_metadata(name, value, provider, Some(provider), None)
            .unwrap();
    }
    write_policy(
        &store,
        r#"
[[rules]]
id = "allow-provider-probe-github"
effect = "allow"
action = "provider.network"
package = "arcwell-provider-probe"
provider = "github"
source = "provider_credential_probe"
reason = "allow github credential probe fixture"
priority = 20

[[rules]]
id = "allow-provider-probe-openai"
effect = "allow"
action = "provider.network"
package = "arcwell-provider-probe"
provider = "openai"
source = "provider_credential_probe"
reason = "allow openai credential probe fixture"
priority = 20

[[rules]]
id = "allow-provider-probe-brave"
effect = "allow"
action = "provider.network"
package = "arcwell-provider-probe"
provider = "brave"
source = "provider_credential_probe"
reason = "allow brave credential probe fixture"
priority = 20

[[rules]]
id = "allow-provider-probe-cloudflare"
effect = "allow"
action = "provider.network"
package = "arcwell-provider-probe"
provider = "cloudflare"
source = "provider_credential_probe"
reason = "allow cloudflare credential probe fixture"
priority = 20
"#,
    );
    let (base, requests) = mock_recording_sequence_server(vec![
        (
            "200 OK",
            "",
            r#"{"login":"arcwell","id":42}"#,
            "application/json",
        ),
        (
            "200 OK",
            "",
            r#"{"object":"list","data":[]}"#,
            "application/json",
        ),
        (
            "200 OK",
            "",
            r#"{"query":{"original":"arcwell"},"web":{"results":[]}}"#,
            "application/json",
        ),
        (
            "200 OK",
            "",
            r#"{"success":true,"result":{"status":"active"}}"#,
            "application/json",
        ),
    ]);
    let specs = vec![
        test_provider_probe_spec("github", "GITHUB_TOKEN", &format!("{base}/github")),
        test_provider_probe_spec("openai", "OPENAI_API_KEY", &format!("{base}/openai")),
        test_provider_probe_spec("brave", "BRAVE_SEARCH_API_KEY", &format!("{base}/brave")),
        test_provider_probe_spec(
            "cloudflare",
            "CLOUDFLARE_API_TOKEN",
            &format!("{base}/cloudflare"),
        ),
    ];

    let report = store.provider_credential_probe_with_specs(specs).unwrap();
    assert_eq!(report.status, "passed");
    assert!(report.missing_or_failed_providers.is_empty(), "{report:?}");
    assert_eq!(report.endpoints.len(), 4);
    let captured = requests.lock().unwrap().join("\n");
    assert!(captured.contains("GET /github"), "{captured}");
    assert!(
        captured.contains("authorization: Bearer ghp_"),
        "{captured}"
    );
    assert!(captured.contains("GET /openai"), "{captured}");
    assert!(captured.contains("authorization: Bearer sk-"), "{captured}");
    assert!(captured.contains("GET /brave"), "{captured}");
    assert!(
        captured.contains("x-subscription-token: brave-"),
        "{captured}"
    );
    assert!(captured.contains("GET /cloudflare"), "{captured}");
    for provider in ["github", "openai", "brave", "cloudflare"] {
        let health = store
            .get_source_health(&format!("provider:{provider}:credential-probe"))
            .unwrap()
            .expect("provider health");
        assert_eq!(health.status, "healthy", "{health:?}");
        assert_eq!(health.source_kind, "provider_credential_probe");
    }
    let serialized = serde_json::to_string(&report).unwrap();
    for token in [
        &github_token,
        &openai_token,
        &brave_token,
        &cloudflare_token,
    ] {
        assert!(!serialized.contains(token), "{serialized}");
    }
}

#[test]
fn severe_cloudflare_provider_probe_uses_account_endpoint_for_account_tokens() {
    // CLAIM: Cloudflare provider probes validate the token against the
    // configured account when an account id is available, instead of assuming
    // every useful Cloudflare token passes /user/tokens/verify.
    // ORACLE: the default Cloudflare probe spec is rewritten to the account
    // endpoint and expects account response evidence; injected loopback URLs
    // are left alone for deterministic tests.
    // SEVERITY: Severe because Wrangler/account-scoped tokens can be valid
    // for the actual email/worker operations while failing the user-token
    // verify endpoint, creating false credential outages.
    let store = test_store("provider-credential-probe-cloudflare-account");
    store
        .set_secret_value_with_metadata(
            "CLOUDFLARE_ACCOUNT_ID",
            "0123456789abcdef0123456789abcdef",
            "cloudflare",
            Some("cloudflare"),
            None,
        )
        .unwrap();
    let mut spec = provider_credential_probe_specs(&["cloudflare".to_string()])
        .unwrap()
        .remove(0);
    store
        .prepare_provider_credential_probe_spec(&mut spec)
        .unwrap();
    assert_eq!(
        spec.url,
        "https://api.cloudflare.com/client/v4/accounts/0123456789abcdef0123456789abcdef"
    );
    assert!(matches!(
        spec.evidence,
        ProviderProbeEvidence::CloudflareAccount
    ));

    let mut injected = test_provider_probe_spec(
        "cloudflare",
        "CLOUDFLARE_API_TOKEN",
        "http://127.0.0.1:9999/cloudflare",
    );
    store
        .prepare_provider_credential_probe_spec(&mut injected)
        .unwrap();
    assert_eq!(injected.url, "http://127.0.0.1:9999/cloudflare");
    assert!(matches!(
        injected.evidence,
        ProviderProbeEvidence::CloudflareTokenVerify
    ));
}

#[test]
fn severe_provider_credential_probe_keeps_policy_missing_quota_failures_visible_and_redacted() {
    // CLAIM: provider credential probing keeps partial failures explicit and
    // fails closed before secret reads/network when policy denies a provider.
    // PRECONDITIONS: GitHub is policy-denied, OpenAI succeeds, Brave is
    // missing a secret, and Cloudflare returns a token-echoing 429.
    // POSTCONDITIONS: the report is partial, every failed provider has a
    // durable source_health row with a distinct classification, only two
    // provider requests occur, and raw token text is redacted everywhere.
    // ORACLE: report classifications, recorded request paths, source_health
    // rows, and serialized output token scan.
    // SEVERITY: Severe because this catches secret-leaking errors, false
    // global success, policy-bypass network calls, and missing-secret
    // silence in one fixture.
    let store = test_store("provider-credential-probe-partial");
    let github_token = format!("ghp_{}", "d".repeat(80));
    let openai_token = format!("sk-{}", "p".repeat(80));
    let cloudflare_token = format!("cf-{}", "q".repeat(80));
    for (name, value, provider) in [
        ("GITHUB_TOKEN", github_token.as_str(), "github"),
        ("OPENAI_API_KEY", openai_token.as_str(), "openai"),
        (
            "CLOUDFLARE_API_TOKEN",
            cloudflare_token.as_str(),
            "cloudflare",
        ),
    ] {
        store
            .set_secret_value_with_metadata(name, value, provider, Some(provider), None)
            .unwrap();
    }
    write_policy(
        &store,
        r#"
[[rules]]
id = "deny-provider-probe-github"
effect = "deny"
action = "provider.network"
package = "arcwell-provider-probe"
provider = "github"
source = "provider_credential_probe"
reason = "deny github credential probe fixture"
priority = 50

[[rules]]
id = "allow-provider-probe-openai"
effect = "allow"
action = "provider.network"
package = "arcwell-provider-probe"
provider = "openai"
source = "provider_credential_probe"
reason = "allow openai credential probe fixture"
priority = 20

[[rules]]
id = "allow-provider-probe-brave"
effect = "allow"
action = "provider.network"
package = "arcwell-provider-probe"
provider = "brave"
source = "provider_credential_probe"
reason = "allow brave credential probe fixture"
priority = 20

[[rules]]
id = "allow-provider-probe-cloudflare"
effect = "allow"
action = "provider.network"
package = "arcwell-provider-probe"
provider = "cloudflare"
source = "provider_credential_probe"
reason = "allow cloudflare credential probe fixture"
priority = 20
"#,
    );
    let (base, requests) = mock_recording_sequence_server(vec![
        (
            "200 OK",
            "",
            r#"{"object":"list","data":[]}"#,
            "application/json",
        ),
        (
            "429 Too Many Requests",
            "retry-after: 60\r\n",
            r#"{"error":"quota","detail":"retry later with cf-qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq"}"#,
            "application/json",
        ),
    ]);
    let specs = vec![
        test_provider_probe_spec("github", "GITHUB_TOKEN", &format!("{base}/github")),
        test_provider_probe_spec("openai", "OPENAI_API_KEY", &format!("{base}/openai")),
        test_provider_probe_spec("brave", "BRAVE_SEARCH_API_KEY", &format!("{base}/brave")),
        test_provider_probe_spec(
            "cloudflare",
            "CLOUDFLARE_API_TOKEN",
            &format!("{base}/cloudflare"),
        ),
    ];

    let report = store.provider_credential_probe_with_specs(specs).unwrap();
    assert_eq!(report.status, "partial");
    assert_eq!(
        report.missing_or_failed_providers,
        vec![
            "brave".to_string(),
            "cloudflare".to_string(),
            "github".to_string()
        ]
    );
    let by_provider = report
        .endpoints
        .iter()
        .map(|endpoint| (endpoint.provider.as_str(), endpoint))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(by_provider["github"].classification, "policy_denied");
    assert_eq!(by_provider["openai"].status, "passed");
    assert_eq!(by_provider["brave"].classification, "missing_secret");
    assert_eq!(
        by_provider["cloudflare"].classification,
        "quota_or_rate_limit"
    );
    let captured = requests.lock().unwrap().join("\n");
    assert!(!captured.contains("/github"), "{captured}");
    assert!(captured.contains("GET /openai"), "{captured}");
    assert!(!captured.contains("/brave"), "{captured}");
    assert!(captured.contains("GET /cloudflare"), "{captured}");
    for (provider, expected_status) in [
        ("github", "failed"),
        ("openai", "healthy"),
        ("brave", "failed"),
        ("cloudflare", "rate_limited"),
    ] {
        let health = store
            .get_source_health(&format!("provider:{provider}:credential-probe"))
            .unwrap()
            .expect("provider health");
        assert_eq!(health.status, expected_status, "{health:?}");
        assert!(
            !health
                .last_error
                .as_deref()
                .unwrap_or_default()
                .contains(&cloudflare_token),
            "{health:?}"
        );
    }
    let serialized = serde_json::to_string(&report).unwrap();
    for token in [&github_token, &openai_token, &cloudflare_token] {
        assert!(!serialized.contains(token), "{serialized}");
    }
    assert!(serialized.contains("[REDACTED]"), "{serialized}");
}

#[test]
fn severe_x_oauth_public_exchange_includes_client_id_without_basic_auth() {
    // CLAIM: X public-client OAuth authorization-code exchange identifies
    // the client in the form body and does not send Basic auth.
    // PRECONDITIONS: The caller does not supply a client secret and no
    // usable client secret is resolved from the environment/store.
    // POSTCONDITIONS: the token request has no Authorization: Basic header
    // and includes client_id with the authorization-code form fields.
    // ORACLE: request bytes captured by a local token endpoint fixture.
    // SEVERITY: Severe because repairing confidential-client request shape
    // must not regress the documented public-client PKCE path.
    let store = test_store("x-oauth-public-exchange-shape");
    let base = mock_oauth_request_assertion_server(|request| {
        assert!(request.contains("POST /2/oauth2/token "), "{request}");
        assert!(
            !request
                .to_ascii_lowercase()
                .contains("authorization: basic "),
            "{request}"
        );
        assert!(
            request.contains("grant_type=authorization_code"),
            "{request}"
        );
        assert!(request.contains("code=auth-code"), "{request}");
        assert!(
            request.contains("redirect_uri=http%3A%2F%2F127.0.0.1%2Fcallback"),
            "{request}"
        );
        assert!(request.contains("code_verifier=pkce-verifier"), "{request}");
        assert!(request.contains("client_id=client-id"), "{request}");
    });
    let report = store
        .x_oauth_exchange_code_with_base(
            "client-id",
            "http://127.0.0.1/callback",
            "auth-code",
            "pkce-verifier",
            None,
            true,
            &base,
        )
        .unwrap();
    assert_eq!(
        report.stored,
        vec!["X_BEARER_TOKEN".to_string(), "X_REFRESH_TOKEN".to_string()]
    );
}

#[test]
fn severe_x_oauth_public_refresh_includes_client_id_without_basic_auth() {
    // CLAIM: X public-client OAuth refresh identifies the client in the
    // form body and does not send Basic auth.
    // PRECONDITIONS: A refresh token is stored and the caller does not
    // supply a client secret.
    // POSTCONDITIONS: the token request has no Authorization: Basic header
    // and includes grant_type, refresh_token, and client_id in the body.
    // ORACLE: request bytes captured by a local token endpoint fixture.
    // SEVERITY: Severe because this is the fallback path documented by X
    // for public PKCE clients.
    let store = test_store("x-oauth-public-refresh-shape");
    store
        .set_secret_value("X_REFRESH_TOKEN", "refresh-token", "x")
        .unwrap();
    let base = mock_oauth_request_assertion_server(|request| {
        assert!(request.contains("POST /2/oauth2/token "), "{request}");
        assert!(
            !request
                .to_ascii_lowercase()
                .contains("authorization: basic "),
            "{request}"
        );
        assert!(request.contains("grant_type=refresh_token"), "{request}");
        assert!(request.contains("refresh_token=refresh-token"), "{request}");
        assert!(request.contains("client_id=client-id"), "{request}");
    });
    let report = store
        .x_oauth_refresh_with_base("client-id", None, true, &base)
        .unwrap();
    assert_eq!(
        report.stored,
        vec!["X_BEARER_TOKEN".to_string(), "X_REFRESH_TOKEN".to_string()]
    );
}

#[test]
fn severe_x_oauth_public_refresh_ignores_stored_client_secret_aliases() {
    // CLAIM: explicit public-client mode remains public even when older
    // confidential-client secret aliases are present in local secret storage.
    // PRECONDITIONS: A refresh token and stored X_CLIENT_SECRET alias exist.
    // POSTCONDITIONS: the token request omits Authorization: Basic and sends
    // client_id in the form body.
    // ORACLE: request bytes captured by a local token endpoint fixture.
    // SEVERITY: Severe because stale stored client-secret aliases can otherwise
    // shadow PKCE public-client recovery and make live refresh look healthy
    // while producing provider-rejected tokens.
    let store = test_store("x-oauth-public-refresh-ignores-stored-secret");
    store
        .set_secret_value("X_REFRESH_TOKEN", "refresh-token", "x")
        .unwrap();
    store
        .set_secret_value("X_CLIENT_SECRET", "stored-client-secret", "x")
        .unwrap();
    store
        .set_secret_value(
            "TWITTER_OAUTH2_CLIENT_SECRET",
            "legacy-stored-client-secret",
            "x",
        )
        .unwrap();
    let base = mock_oauth_request_assertion_server(|request| {
        assert!(request.contains("POST /2/oauth2/token "), "{request}");
        assert!(
            !request
                .to_ascii_lowercase()
                .contains("authorization: basic "),
            "{request}"
        );
        assert!(request.contains("grant_type=refresh_token"), "{request}");
        assert!(request.contains("refresh_token=refresh-token"), "{request}");
        assert!(request.contains("client_id=client-id"), "{request}");
    });
    let report = store
        .x_oauth_refresh_with_base("client-id", None, true, &base)
        .unwrap();
    assert_eq!(
        report.stored,
        vec!["X_BEARER_TOKEN".to_string(), "X_REFRESH_TOKEN".to_string()]
    );
}

#[test]
fn severe_x_oauth_confidential_exchange_uses_basic_auth_without_client_id_body() {
    // CLAIM: X confidential-client OAuth authorization-code exchange uses
    // Basic auth for client identity and omits client_id from the form body.
    // PRECONDITIONS: The caller supplies an explicit client secret.
    // POSTCONDITIONS: the token request carries Authorization: Basic, the
    // form includes code/redirect/code_verifier, and the body does not also
    // include client_id.
    // ORACLE: request bytes captured by a local token endpoint fixture.
    // SEVERITY: Severe because mixed public/confidential OAuth request shape
    // breaks real X token exchange before any radar proof can run.
    let store = test_store("x-oauth-confidential-exchange-shape");
    let base = mock_oauth_request_assertion_server(|request| {
        assert!(request.contains("POST /2/oauth2/token "), "{request}");
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: basic "),
            "{request}"
        );
        assert!(
            request.contains("grant_type=authorization_code"),
            "{request}"
        );
        assert!(request.contains("code=auth-code"), "{request}");
        assert!(
            request.contains("redirect_uri=http%3A%2F%2F127.0.0.1%2Fcallback"),
            "{request}"
        );
        assert!(request.contains("code_verifier=pkce-verifier"), "{request}");
        assert!(!request.contains("client_id=client-id"), "{request}");
    });
    let report = store
        .x_oauth_exchange_code_with_base(
            "client-id",
            "http://127.0.0.1/callback",
            "auth-code",
            "pkce-verifier",
            Some("client-secret"),
            false,
            &base,
        )
        .unwrap();
    assert_eq!(
        report.stored,
        vec!["X_BEARER_TOKEN".to_string(), "X_REFRESH_TOKEN".to_string()]
    );
}

#[test]
fn severe_x_oauth_confidential_refresh_uses_basic_auth_without_client_id_body() {
    // CLAIM: X confidential-client OAuth refresh uses Basic auth as client
    // identity and omits client_id from the form body.
    // PRECONDITIONS: A refresh token is stored and an explicit client secret
    // is supplied.
    // POSTCONDITIONS: the token request carries Authorization: Basic, the
    // form includes grant_type/refresh_token, and it does not also include
    // client_id in the body.
    // ORACLE: request bytes captured by a local token endpoint fixture.
    // SEVERITY: Severe because X rejects the mixed confidential/public form
    // with unauthorized_client, making live proofs look credential-broken.
    let store = test_store("x-oauth-confidential-refresh-shape");
    store
        .set_secret_value("X_REFRESH_TOKEN", "refresh-token", "x")
        .unwrap();
    let base = mock_oauth_request_assertion_server(|request| {
        assert!(request.contains("POST /2/oauth2/token "), "{request}");
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: basic "),
            "{request}"
        );
        assert!(request.contains("grant_type=refresh_token"), "{request}");
        assert!(request.contains("refresh_token=refresh-token"), "{request}");
        assert!(!request.contains("client_id=client-id"), "{request}");
    });
    let report = store
        .x_oauth_refresh_with_base("client-id", Some("client-secret"), false, &base)
        .unwrap();
    assert_eq!(
        report.stored,
        vec!["X_BEARER_TOKEN".to_string(), "X_REFRESH_TOKEN".to_string()]
    );
}

#[test]
fn severe_x_worker_auto_refresh_uses_stored_confidential_client_secret() {
    // CLAIM: worker-side automatic X bearer refresh uses the configured
    // confidential-client credentials when they exist.
    // PRECONDITIONS: Stored X client id, client secret, and refresh token exist
    // but no usable bearer token is available.
    // POSTCONDITIONS: the automatic refresh request carries Authorization:
    // Basic and does not force the public-client client_id form shape.
    // ORACLE: request bytes captured by a local token endpoint fixture and the
    // refreshed bearer token stored in local secret state.
    // SEVERITY: Severe because the resident worker otherwise keeps choosing a
    // provider-rejected public-client refresh path while operators see valid
    // confidential-client credentials in secret health.
    clear_x_bearer_env();
    let store = test_store("x-worker-auto-refresh-confidential");
    store
        .set_secret_value("X_CLIENT_ID", "client-id", "x")
        .unwrap();
    store
        .set_secret_value("X_CLIENT_SECRET", "stored-client-secret", "x")
        .unwrap();
    store
        .set_secret_value("X_REFRESH_TOKEN", "refresh-token", "x")
        .unwrap();
    let base = mock_oauth_request_assertion_server(|request| {
        assert!(request.contains("POST /2/oauth2/token "), "{request}");
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: basic "),
            "{request}"
        );
        assert!(request.contains("grant_type=refresh_token"), "{request}");
        assert!(request.contains("refresh_token=refresh-token"), "{request}");
        assert!(!request.contains("client_id=client-id"), "{request}");
    });

    let token = store.refresh_x_bearer_token_for_endpoint(&base).unwrap();
    assert_eq!(token, "fresh-access-token");
    assert_eq!(
        store.get_secret_value("X_BEARER_TOKEN").unwrap().as_deref(),
        Some("fresh-access-token")
    );
}

#[test]
fn severe_x_oauth_public_revoke_includes_client_id_without_basic_auth() {
    // CLAIM: X public-client OAuth revocation identifies the client in the
    // form body and never uses Basic auth.
    // PRECONDITIONS: a stored X bearer token exists and no client secret is supplied.
    // POSTCONDITIONS: the revoke request hits /2/oauth2/revoke, carries the
    // token/token_type_hint/client_id form fields, and leaves local state
    // untouched unless explicitly requested.
    // ORACLE: request bytes captured by a local revoke endpoint fixture and
    // post-call local secret state.
    // SEVERITY: Severe because request-shape drift would make live credential
    // revocation look completed while the provider never accepted it.
    let store = test_store("x-oauth-public-revoke-shape");
    store
        .set_secret_value("X_BEARER_TOKEN", "access-token-to-revoke", "x")
        .unwrap();
    let base = mock_oauth_request_assertion_server(|request| {
        assert!(request.contains("POST /2/oauth2/revoke "), "{request}");
        assert!(
            !request
                .to_ascii_lowercase()
                .contains("authorization: basic "),
            "{request}"
        );
        assert!(
            request.contains("token=access-token-to-revoke"),
            "{request}"
        );
        assert!(
            request.contains("token_type_hint=access_token"),
            "{request}"
        );
        assert!(request.contains("client_id=client-id"), "{request}");
    });
    let report = store
        .x_oauth_revoke_with_base(
            "X_BEARER_TOKEN",
            "client-id",
            None,
            true,
            Some("access_token"),
            false,
            &base,
        )
        .unwrap();
    assert_eq!(report.provider_status, 200);
    assert!(report.revoked_provider_side);
    assert!(!report.deleted_local_secret);
    assert_eq!(
        store.get_secret_value("X_BEARER_TOKEN").unwrap().as_deref(),
        Some("access-token-to-revoke")
    );
}

#[test]
fn severe_x_oauth_confidential_revoke_uses_basic_auth_without_client_id_body() {
    // CLAIM: X confidential-client OAuth revocation uses Basic auth as
    // client identity and does not also send client_id in the body.
    // PRECONDITIONS: a stored X refresh token exists and a client secret is supplied.
    // POSTCONDITIONS: provider success can delete only the selected local
    // secret after the revoke call succeeds.
    // ORACLE: captured request bytes and local secret inventory after success.
    // SEVERITY: Severe because revoking the wrong token or deleting before a
    // provider success is a credential recovery failure.
    let store = test_store("x-oauth-confidential-revoke-shape");
    store
        .set_secret_value("X_REFRESH_TOKEN", "refresh-token-to-revoke", "x")
        .unwrap();
    store
        .set_secret_value("X_BEARER_TOKEN", "access-token-to-keep", "x")
        .unwrap();
    let base = mock_oauth_request_assertion_server(|request| {
        assert!(request.contains("POST /2/oauth2/revoke "), "{request}");
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: basic "),
            "{request}"
        );
        assert!(
            request.contains("token=refresh-token-to-revoke"),
            "{request}"
        );
        assert!(
            request.contains("token_type_hint=refresh_token"),
            "{request}"
        );
        assert!(!request.contains("client_id=client-id"), "{request}");
    });
    let report = store
        .x_oauth_revoke_with_base(
            "X_REFRESH_TOKEN",
            "client-id",
            Some("client-secret"),
            false,
            Some("refresh_token"),
            true,
            &base,
        )
        .unwrap();
    assert_eq!(report.secret_name, "X_REFRESH_TOKEN");
    assert!(report.revoked_provider_side);
    assert!(report.deleted_local_secret);
    assert!(store.get_secret_value("X_REFRESH_TOKEN").unwrap().is_none());
    assert_eq!(
        store.get_secret_value("X_BEARER_TOKEN").unwrap().as_deref(),
        Some("access-token-to-keep")
    );
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("refresh-token-to-revoke"));
    assert!(!serialized.contains("client-secret"));
}

#[test]
fn severe_x_oauth_rejects_token_response_without_tokens() {
    let store = test_store("x-oauth-empty");
    let base = mock_base_server(
        r#"{ "token_type": "bearer", "expires_in": 7200 }"#,
        "application/json",
    );
    let error = store
        .x_oauth_exchange_code_with_base(
            "client-id",
            "http://127.0.0.1/callback",
            "code",
            "verifier",
            None,
            true,
            &base,
        )
        .expect_err("token endpoint responses without tokens must not be accepted");
    assert!(
        error
            .to_string()
            .contains("did not include an access_token or refresh_token")
    );
    assert!(store.list_secret_values().unwrap().is_empty());
}

#[test]
fn severe_x_oauth_reauthorize_preflight_blocks_policy_before_browser() {
    // CLAIM: browser-assisted X reauthorization is policy-gated before
    // opening a browser or reaching X.
    // PRECONDITIONS: local policy denies provider.oauth for arcwell-x/x_oauth.
    // POSTCONDITIONS: preflight fails with policy denial and no source or
    // secret state is changed.
    // ORACLE: error text, cost summary, and empty local secret inventory.
    // SEVERITY: Severe because browser auth recovery is high-authority
    // credential work and must not bypass Arcwell policy gates.
    let store = test_store("x-oauth-reauthorize-policy-deny");
    write_policy(
        &store,
        r#"
[[rules]]
id = "deny-x-oauth-reauthorize"
effect = "deny"
action = "provider.oauth"
package = "arcwell-x"
provider = "x"
source = "x_oauth"
reason = "deny browser reauthorization fixture"
priority = 50
"#,
    );

    let error = store
        .x_oauth_reauthorize_preflight(
            "http://127.0.0.1:8765/callback",
            &["tweet.read".to_string(), "offline.access".to_string()],
        )
        .expect_err("policy denial must happen before browser launch")
        .to_string();
    assert!(error.contains("policy denied provider.oauth"), "{error}");
    assert_eq!(store.cost_summary().unwrap().2, 0);
    assert!(store.list_secret_values().unwrap().is_empty());
}

#[test]
fn severe_x_oauth_reauthorize_resolves_stored_client_aliases_without_user_token_material() {
    // CLAIM: reauthorization can use stored Arcwell client metadata and
    // does not require the operator to know token/client strings.
    // PRECONDITIONS: only legacy TWITTER_OAUTH2_* client aliases and a
    // stored redirect URI are present.
    // POSTCONDITIONS: client id, client secret, and redirect URI resolve;
    // preflight returns the full default user-context scope set.
    // ORACLE: resolved non-secret client id/redirect and preflight scopes.
    // SEVERITY: Severe because requiring the user to retype client metadata
    // recreates the credential-management failure mode this path fixes.
    let store = test_store("x-oauth-reauthorize-aliases");
    store
        .set_secret_value("TWITTER_OAUTH2_CLIENT_ID", "stored-client-id", "x")
        .unwrap();
    store
        .set_secret_value("TWITTER_OAUTH2_CLIENT_SECRET", "stored-client-secret", "x")
        .unwrap();
    store
        .set_secret_value("X_REDIRECT_URI", "http://127.0.0.1:8765/callback", "x")
        .unwrap();
    write_policy(
        &store,
        r#"
[[rules]]
id = "allow-x-oauth-reauthorize"
effect = "allow"
action = "provider.oauth"
package = "arcwell-x"
provider = "x"
source = "x_oauth"
reason = "allow browser reauthorization fixture"
priority = 20
"#,
    );

    assert_eq!(
        store.resolve_x_oauth_client_id(None).unwrap(),
        "stored-client-id"
    );
    assert_eq!(
        store.resolve_x_oauth_redirect_uri(None).unwrap(),
        "http://127.0.0.1:8765/callback"
    );
    assert_eq!(
        store
            .resolve_x_client_secret(None, false)
            .unwrap()
            .as_deref(),
        Some("stored-client-secret")
    );
    let report = store
        .x_oauth_reauthorize_preflight("http://127.0.0.1:8765/callback", &[])
        .unwrap();
    assert_eq!(report.status, "ready");
    for scope in [
        "tweet.read",
        "users.read",
        "bookmark.read",
        "follows.read",
        "offline.access",
    ] {
        assert!(report.scopes.contains(&scope.to_string()), "{report:?}");
    }
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("stored-client-secret"));
}

#[test]
fn severe_x_oauth_refresh_failure_is_classified_and_redacted() {
    // CLAIM: X OAuth refresh failures are visible by class and never echo token values.
    // PRECONDITIONS: A stored refresh token exists and the token endpoint rejects refresh.
    // POSTCONDITIONS: Error names token rejection/refresh failure, stored secrets
    // are unchanged, source health records auth_failed, and raw tokens are absent.
    // ORACLE: Error string, source health, and secret list/value surfaces.
    // SEVERITY: Severe because refresh failures are a realistic production credential lifecycle break.
    clear_x_bearer_env();
    let store = test_store("x-oauth-refresh-failure");
    let refresh_token = format!("refresh-{}", "q".repeat(48));
    store
        .set_secret_value("X_REFRESH_TOKEN", &refresh_token, "x")
        .unwrap();
    let body = Box::leak(
        format!(
            r#"{{"error":"invalid_request","error_description":"Value passed for the token was invalid. refresh_token={refresh_token}"}}"#
        )
        .into_boxed_str(),
    );
    let base = mock_status_server("400 Bad Request", "", body, "application/json");

    let error = store
        .x_oauth_refresh_with_base("client-id", None, true, &base)
        .expect_err("refresh rejection must be surfaced")
        .to_string();
    assert!(error.contains("X OAuth token endpoint failed"), "{error}");
    assert!(
        error.contains("token rejected") || error.contains("expired") || error.contains("invalid"),
        "{error}"
    );
    assert!(!error.contains(&refresh_token));
    let health = store
        .get_source_health("x:oauth-refresh")
        .unwrap()
        .expect("refresh failure must write source health");
    assert_eq!(health.provider, "x");
    assert_eq!(health.source_kind, "x_oauth");
    assert_eq!(health.locator, "oauth_refresh");
    assert_eq!(health.status, "auth_failed");
    let health_json = serde_json::to_string(&health).unwrap();
    assert!(health_json.contains("X OAuth token endpoint failed"));
    assert!(!health_json.contains(&refresh_token));
    let listed = serde_json::to_string(&store.list_secret_values().unwrap()).unwrap();
    assert!(listed.contains("X_REFRESH_TOKEN"));
    assert!(!listed.contains(&refresh_token));
    assert_eq!(
        store
            .get_secret_value("X_REFRESH_TOKEN")
            .unwrap()
            .as_deref(),
        Some(refresh_token.as_str())
    );
}

#[test]
fn severe_policy_denied_x_oauth_revoke_blocks_before_secret_or_cost_mutation() {
    // CLAIM: X OAuth revocation requires provider.oauth policy before token
    // lookup, network IO, local deletion, or cost reservation.
    // PRECONDITIONS: a stored token exists but policy denies x_oauth.
    // POSTCONDITIONS: the token remains present and no cost is recorded.
    // ORACLE: denial reason, secret value state, and cost summary.
    // SEVERITY: Severe because revocation is a destructive credential-control path.
    let store = test_store("policy-x-oauth-revoke-deny");
    let token = "access-token-denied-revoke";
    store
        .set_secret_value("X_BEARER_TOKEN", token, "x")
        .unwrap();
    write_policy(
        &store,
        r#"
[[rules]]
id = "deny-x-oauth-revoke"
effect = "deny"
action = "provider.oauth"
provider = "x"
source = "x_oauth"
reason = "X OAuth disabled during revoke policy test"
"#,
    );

    let error = store
        .x_oauth_revoke_with_base(
            "X_BEARER_TOKEN",
            "client-id",
            None,
            true,
            Some("access_token"),
            true,
            "https://api.x.com",
        )
        .unwrap_err()
        .to_string();
    assert!(error.contains("policy denied provider.oauth"), "{error}");
    assert_eq!(
        store.get_secret_value("X_BEARER_TOKEN").unwrap().as_deref(),
        Some(token)
    );
    assert_eq!(store.cost_summary().unwrap().2, 0);
}

#[test]
fn severe_x_oauth_revoke_failure_is_redacted_and_preserves_local_secret() {
    // CLAIM: provider-side revoke failures surface a classified error, never
    // leak the token, and do not delete the local secret even when
    // delete_local was requested.
    // PRECONDITIONS: the revoke endpoint rejects a stored token.
    // POSTCONDITIONS: raw token value is absent from errors/list output and
    // the local token remains retryable.
    // ORACLE: error text and local secret state.
    // SEVERITY: Severe because failure-time deletion would turn transient
    // provider errors into permanent credential loss.
    let store = test_store("x-oauth-revoke-failure");
    let token = format!("access-revoke-{}", "z".repeat(48));
    store
        .set_secret_value("X_BEARER_TOKEN", &token, "x")
        .unwrap();
    let body = Box::leak(
        format!(r#"{{"error":"invalid_request","detail":"token={token} rejected"}}"#)
            .into_boxed_str(),
    );
    let base = mock_status_server("400 Bad Request", "", body, "application/json");

    let error = store
        .x_oauth_revoke_with_base(
            "X_BEARER_TOKEN",
            "client-id",
            None,
            true,
            Some("access_token"),
            true,
            &base,
        )
        .expect_err("revoke rejection must be surfaced")
        .to_string();
    assert!(error.contains("X OAuth revoke endpoint failed"), "{error}");
    assert!(!error.contains(&token), "{error}");
    let listed = serde_json::to_string(&store.list_secret_values().unwrap()).unwrap();
    assert!(listed.contains("X_BEARER_TOKEN"));
    assert!(!listed.contains(&token));
    assert_eq!(
        store.get_secret_value("X_BEARER_TOKEN").unwrap().as_deref(),
        Some(token.as_str())
    );
}

#[test]
fn severe_x_oauth_revoke_rejects_unsupported_secret_names_and_hints() {
    // CLAIM: OAuth revocation cannot be abused to post arbitrary local
    // secrets or unsupported token hints to the provider.
    // ORACLE: invalid names/hints fail before network IO and leave secrets intact.
    // SEVERITY: Severe because token-control surfaces sit next to local secret storage.
    let store = test_store("x-oauth-revoke-invalid-input");
    store
        .set_secret_value("OPENAI_API_KEY", "not-an-x-token", "openai")
        .unwrap();
    store
        .set_secret_value("X_BEARER_TOKEN", "access-token", "x")
        .unwrap();

    let name_error = store
        .x_oauth_revoke_with_base(
            "OPENAI_API_KEY",
            "client-id",
            None,
            false,
            Some("access_token"),
            false,
            "https://api.x.com",
        )
        .unwrap_err()
        .to_string();
    assert!(name_error.contains("only supports X_BEARER_TOKEN or X_REFRESH_TOKEN"));

    let hint_error = store
        .x_oauth_revoke_with_base(
            "X_BEARER_TOKEN",
            "client-id",
            None,
            false,
            Some("id_token"),
            false,
            "https://api.x.com",
        )
        .unwrap_err()
        .to_string();
    assert!(hint_error.contains("token_type_hint must be access_token or refresh_token"));
    assert_eq!(
        store.get_secret_value("OPENAI_API_KEY").unwrap().as_deref(),
        Some("not-an-x-token")
    );
    assert_eq!(
        store.get_secret_value("X_BEARER_TOKEN").unwrap().as_deref(),
        Some("access-token")
    );
}

#[test]
fn severe_x_provider_auto_refresh_failure_is_redacted_and_preserves_cursor() {
    // CLAIM: Automatic pre-fetch X OAuth refresh failures fail closed without leaking secrets or advancing provider cursors.
    // PRECONDITIONS: X recent search has an expired bearer and a refresh token, but the token endpoint rejects refresh.
    // POSTCONDITIONS: no search request is made, the old cursor/items remain absent, and the failed sync is operator-visible.
    // ORACLE: captured request count, durable secret/cursor/item/sync-run state, and redacted error text.
    // SEVERITY: Severe because a broken refresh token is a normal production outage and must not corrupt ingestion state.
    clear_x_bearer_env();
    let store = test_store("x-auto-refresh-failure-redacted");
    let expired_token = format!("expired-auto-{}", "c".repeat(48));
    let refresh_token = format!("refresh-auto-{}", "d".repeat(48));
    let expired_at = (Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();
    store
        .set_secret_value_with_metadata(
            "X_BEARER_TOKEN",
            &expired_token,
            "x",
            Some("x"),
            Some(&expired_at),
        )
        .unwrap();
    store
        .set_secret_value("X_REFRESH_TOKEN", &refresh_token, "x")
        .unwrap();
    store
        .set_secret_value("X_CLIENT_ID", "client-id", "x")
        .unwrap();
    let body = Box::leak(
        format!(r#"{{"error":"invalid_grant","detail":"refresh_token={refresh_token} expired"}}"#)
            .into_boxed_str(),
    );
    let (base, requests) =
        mock_recording_sequence_server(vec![("401 Unauthorized", "", body, "application/json")]);

    let error = store
        .x_recent_search_with_base("agents", 10, &base)
        .expect_err("failed OAuth refresh must stop the provider fetch")
        .to_string();
    assert!(
        error.contains("refreshing expired X_BEARER_TOKEN failed"),
        "{error}"
    );
    assert!(error.contains("X OAuth token endpoint failed"), "{error}");
    assert!(!error.contains(&expired_token), "{error}");
    assert!(!error.contains(&refresh_token), "{error}");
    assert_eq!(requests.lock().unwrap().len(), 1);
    assert!(
        store
            .get_cursor("x:recent-search:agents")
            .unwrap()
            .is_none()
    );
    assert!(store.list_x_items(None).unwrap().is_empty());
    assert_eq!(
        store.get_secret_value("X_BEARER_TOKEN").unwrap().as_deref(),
        Some(expired_token.as_str())
    );
    assert_eq!(
        store
            .get_secret_value("X_REFRESH_TOKEN")
            .unwrap()
            .as_deref(),
        Some(refresh_token.as_str())
    );
    let health = store
        .get_source_health("x:recent-search:agents")
        .unwrap()
        .expect("failed auto-refresh must be visible in source health");
    let health_json = serde_json::to_string(&health).unwrap();
    assert_eq!(health.status, "auth_failed");
    assert!(health_json.contains("X_BEARER_TOKEN"), "{health_json}");
    assert!(!health_json.contains(&expired_token), "{health_json}");
    assert!(!health_json.contains(&refresh_token), "{health_json}");
    let stats = store.x_stats().unwrap();
    assert_eq!(stats.latest_sync_runs[0].stream, "recent_search");
    assert_eq!(stats.latest_sync_runs[0].status, "failed");
    assert!(
        !stats.latest_sync_runs[0]
            .error
            .as_deref()
            .unwrap_or("")
            .contains(&refresh_token)
    );
    assert_eq!(
        store.cost_summary().unwrap().2,
        1,
        "the OAuth refresh provider call is budgeted, but the original search reservation is released"
    );
}
