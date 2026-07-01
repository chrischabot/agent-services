use crate::*;

pub(crate) enum SlashAliasResolution {
    Cli(Vec<OsString>),
    Mcp {
        home: Option<PathBuf>,
        tool: &'static str,
        arguments: Value,
    },
    HostOnly {
        alias: String,
        reason: &'static str,
    },
}

#[derive(Clone, Copy)]
pub(crate) enum SlashAliasTarget {
    Cli(&'static [&'static str]),
    Mcp(&'static str),
    HostOnly(&'static str),
}

pub(crate) fn resolve_slash_alias(args: Vec<OsString>) -> Result<SlashAliasResolution> {
    let Some(command_index) = slash_alias_command_index(&args) else {
        return Ok(SlashAliasResolution::Cli(args));
    };
    let Some(alias) = slash_alias_name(&args[command_index]) else {
        return Ok(SlashAliasResolution::Cli(args));
    };
    if let Some(resolution) = resolve_dynamic_slash_alias(&args, command_index, alias)? {
        return Ok(resolution);
    }
    let Some(target) = slash_alias_target(alias) else {
        return Ok(SlashAliasResolution::Cli(args));
    };
    match target {
        SlashAliasTarget::Cli(parts) => Ok(SlashAliasResolution::Cli(rewrite_slash_alias_args(
            &args,
            command_index,
            parts,
            &args[command_index + 1..],
        ))),
        SlashAliasTarget::Mcp(tool) => Ok(SlashAliasResolution::Mcp {
            home: home_arg_from_raw_args(&args, command_index),
            tool,
            arguments: parse_slash_alias_mcp_arguments(alias, tool, &args[command_index + 1..])?,
        }),
        SlashAliasTarget::HostOnly(reason) => Ok(SlashAliasResolution::HostOnly {
            alias: alias.to_string(),
            reason,
        }),
    }
}

pub(crate) fn slash_alias_command_index(args: &[OsString]) -> Option<usize> {
    let mut index = 1;
    while index < args.len() {
        let text = args[index].to_string_lossy();
        if text == "--home" {
            index += 2;
            continue;
        }
        if text.starts_with("--home=") {
            index += 1;
            continue;
        }
        if text.starts_with('-') {
            return None;
        }
        return Some(index);
    }
    None
}

pub(crate) fn slash_alias_name(value: &OsString) -> Option<&str> {
    let text = value.to_str()?.trim_start_matches('/');
    (slash_alias_target(text).is_some() || slash_alias_is_dynamic(text)).then_some(text)
}

pub(crate) fn rewrite_slash_alias_args(
    args: &[OsString],
    command_index: usize,
    parts: &[&str],
    rest: &[OsString],
) -> Vec<OsString> {
    let mut rewritten = args[..command_index].to_vec();
    rewritten.extend(parts.iter().map(OsString::from));
    rewritten.extend(rest.iter().cloned());
    rewritten
}

pub(crate) fn home_arg_from_raw_args(args: &[OsString], command_index: usize) -> Option<PathBuf> {
    let mut index = 1;
    while index < command_index {
        let text = args[index].to_string_lossy();
        if text == "--home" {
            return args.get(index + 1).map(PathBuf::from);
        }
        if let Some(home) = text.strip_prefix("--home=") {
            return Some(PathBuf::from(home));
        }
        index += 1;
    }
    None
}

pub(crate) fn parse_slash_alias_mcp_arguments(
    alias: &str,
    tool: &str,
    rest: &[OsString],
) -> Result<Value> {
    if rest.is_empty() {
        return Ok(json!({}));
    }
    let first = rest[0].to_string_lossy();
    let raw = if first == "--json" {
        rest.get(1)
            .with_context(|| format!("arcwell {alias} --json requires a JSON object"))?
            .to_string_lossy()
            .to_string()
    } else if let Some(raw) = first.strip_prefix("--json=") {
        raw.to_string()
    } else if first == "-" {
        read_stdin_lossy()?
    } else {
        bail!(
            "arcwell {alias} maps to MCP tool {tool}; pass structured arguments as --json '{{...}}' or '-' for stdin"
        );
    };
    serde_json::from_str(&raw).with_context(|| format!("parsing JSON arguments for {alias}"))
}

pub(crate) fn resolve_dynamic_slash_alias(
    args: &[OsString],
    command_index: usize,
    alias: &str,
) -> Result<Option<SlashAliasResolution>> {
    let rest = &args[command_index + 1..];
    match alias {
        "memory-candidates" => {
            let apply = rest.first().is_some_and(|arg| arg == "apply");
            let parts = if apply {
                &["candidate", "apply"][..]
            } else {
                &["candidate", "list"][..]
            };
            let rest = if apply { &rest[1..] } else { rest };
            Ok(Some(SlashAliasResolution::Cli(rewrite_slash_alias_args(
                args,
                command_index,
                parts,
                rest,
            ))))
        }
        "watch-github" => {
            let Some(first) = rest.first().and_then(|arg| arg.to_str()) else {
                return Ok(Some(SlashAliasResolution::Cli(rewrite_slash_alias_args(
                    args,
                    command_index,
                    &["wiki", "enqueue-github-owner"],
                    rest,
                ))));
            };
            if let Some((owner, repo)) = first.split_once('/') {
                let mut rewritten_rest = vec![OsString::from(owner), OsString::from(repo)];
                rewritten_rest.extend(rest[1..].iter().cloned());
                return Ok(Some(SlashAliasResolution::Cli(rewrite_slash_alias_args(
                    args,
                    command_index,
                    &["wiki", "enqueue-github"],
                    &rewritten_rest,
                ))));
            }
            Ok(Some(SlashAliasResolution::Cli(rewrite_slash_alias_args(
                args,
                command_index,
                &["wiki", "enqueue-github-owner"],
                rest,
            ))))
        }
        "wiki-run-github" => {
            let Some(first) = rest.first().and_then(|arg| arg.to_str()) else {
                return Ok(Some(SlashAliasResolution::Cli(rewrite_slash_alias_args(
                    args,
                    command_index,
                    &["wiki", "run-github-owner"],
                    rest,
                ))));
            };
            if let Some((owner, repo)) = first.split_once('/') {
                let mut rewritten_rest = vec![OsString::from(owner), OsString::from(repo)];
                rewritten_rest.extend(rest[1..].iter().cloned());
                return Ok(Some(SlashAliasResolution::Cli(rewrite_slash_alias_args(
                    args,
                    command_index,
                    &["wiki", "run-github"],
                    &rewritten_rest,
                ))));
            }
            Ok(Some(SlashAliasResolution::Cli(rewrite_slash_alias_args(
                args,
                command_index,
                &["wiki", "run-github-owner"],
                rest,
            ))))
        }
        "wiki-ingest" => {
            let parts = rest
                .first()
                .and_then(|arg| arg.to_str())
                .map(|target| {
                    if target.starts_with("http://") || target.starts_with("https://") {
                        &["wiki", "ingest-url"][..]
                    } else if PathBuf::from(target).is_dir() {
                        &["wiki", "ingest-dir"][..]
                    } else {
                        &["wiki", "ingest-file"][..]
                    }
                })
                .unwrap_or(&["wiki", "ingest-file"][..]);
            Ok(Some(SlashAliasResolution::Cli(rewrite_slash_alias_args(
                args,
                command_index,
                parts,
                rest,
            ))))
        }
        "x-oauth" => {
            let Some(step) = rest.first().and_then(|arg| arg.to_str()) else {
                return Ok(Some(SlashAliasResolution::Cli(rewrite_slash_alias_args(
                    args,
                    command_index,
                    &["x"],
                    rest,
                ))));
            };
            let parts = match step {
                "url" | "authorize-url" | "oauth-url" => &["x", "oauth-url"][..],
                "exchange" | "exchange-code" | "oauth-exchange" => &["x", "oauth-exchange"][..],
                "reauthorize" | "reauth" | "login" | "oauth-reauthorize" => {
                    &["x", "oauth-reauthorize"][..]
                }
                "refresh" | "oauth-refresh" => &["x", "oauth-refresh"][..],
                "revoke" | "oauth-revoke" => &["x", "oauth-revoke"][..],
                "probe" | "oauth-probe" => &["x", "oauth-probe"][..],
                _ => &["x"][..],
            };
            let rest = if parts == ["x"] { rest } else { &rest[1..] };
            Ok(Some(SlashAliasResolution::Cli(rewrite_slash_alias_args(
                args,
                command_index,
                parts,
                rest,
            ))))
        }
        _ => Ok(None),
    }
}

pub(crate) fn slash_alias_is_dynamic(alias: &str) -> bool {
    matches!(
        alias,
        "memory-candidates" | "watch-github" | "wiki-run-github" | "wiki-ingest" | "x-oauth"
    )
}

pub(crate) fn slash_alias_target(alias: &str) -> Option<SlashAliasTarget> {
    SLASH_COMMAND_ALIASES
        .iter()
        .find_map(|(name, target)| (*name == alias).then_some(*target))
}

const SLASH_COMMAND_ALIASES: &[(&str, SlashAliasTarget)] = &[
    ("arcwell-health", SlashAliasTarget::Cli(&["health"])),
    (
        "backup-create",
        SlashAliasTarget::Cli(&["backup", "create"]),
    ),
    (
        "backup-restore",
        SlashAliasTarget::Cli(&["backup", "restore"]),
    ),
    (
        "backup-status",
        SlashAliasTarget::Cli(&["backup", "status"]),
    ),
    (
        "backup-verify",
        SlashAliasTarget::Cli(&["backup", "verify"]),
    ),
    (
        "channel-authorizations",
        SlashAliasTarget::Mcp("channel_authorizations"),
    ),
    (
        "channel-authorize",
        SlashAliasTarget::Mcp("channel_authorize"),
    ),
    (
        "channel-deliveries",
        SlashAliasTarget::Mcp("channel_delivery_list"),
    ),
    ("channel-list", SlashAliasTarget::Mcp("channel_list")),
    ("channel-record", SlashAliasTarget::Mcp("channel_record")),
    (
        "codex-host-adapter",
        SlashAliasTarget::HostOnly(
            "it needs the resident Codex app thread tools to list/read/create/send/stop threads",
        ),
    ),
    ("cost-add", SlashAliasTarget::Cli(&["cost", "add"])),
    ("cost-check", SlashAliasTarget::Cli(&["cost", "check"])),
    (
        "cost-policy-list",
        SlashAliasTarget::Cli(&["cost", "policies"]),
    ),
    (
        "cost-policy-set",
        SlashAliasTarget::Cli(&["cost", "set-policy"]),
    ),
    ("cost-summary", SlashAliasTarget::Cli(&["cost", "summary"])),
    ("cursor-get", SlashAliasTarget::Cli(&["cursors", "get"])),
    ("cursor-list", SlashAliasTarget::Cli(&["cursors", "list"])),
    (
        "digest-candidate-create",
        SlashAliasTarget::Mcp("digest_candidate_create"),
    ),
    (
        "digest-candidates",
        SlashAliasTarget::Mcp("digest_candidate_list"),
    ),
    (
        "digest-candidate-approve",
        SlashAliasTarget::Mcp("digest_candidate_approve"),
    ),
    (
        "digest-candidate-reject",
        SlashAliasTarget::Mcp("digest_candidate_reject"),
    ),
    (
        "digest-candidate-delivery-check",
        SlashAliasTarget::Mcp("digest_candidate_delivery_check"),
    ),
    (
        "digest-candidate-deliveries",
        SlashAliasTarget::Mcp("digest_candidate_deliveries"),
    ),
    (
        "digest-candidate-deliver-telegram",
        SlashAliasTarget::Mcp("digest_candidate_deliver_telegram"),
    ),
    (
        "digest-candidate-deliver-email",
        SlashAliasTarget::Mcp("digest_candidate_deliver_email"),
    ),
    (
        "digest-alert-schedule-create",
        SlashAliasTarget::Mcp("digest_alert_schedule_create"),
    ),
    (
        "digest-alert-schedules",
        SlashAliasTarget::Mcp("digest_alert_schedules"),
    ),
    (
        "digest-alert-ticks",
        SlashAliasTarget::Mcp("digest_alert_ticks"),
    ),
    (
        "radar-profile-create",
        SlashAliasTarget::Mcp("radar_profile_create"),
    ),
    (
        "radar-profile-read",
        SlashAliasTarget::Mcp("radar_profile_read"),
    ),
    (
        "radar-profiles",
        SlashAliasTarget::Mcp("radar_profile_list"),
    ),
    ("radar-enqueue", SlashAliasTarget::Mcp("radar_enqueue")),
    ("radar-run", SlashAliasTarget::Mcp("radar_run")),
    ("radar-runs", SlashAliasTarget::Mcp("radar_runs")),
    ("radar-stage", SlashAliasTarget::Mcp("radar_stage_read")),
    ("radar-summarize", SlashAliasTarget::Mcp("radar_summarize")),
    ("radar-summary", SlashAliasTarget::Mcp("radar_summary_read")),
    (
        "radar-deliver",
        SlashAliasTarget::Mcp("radar_deliver_summary"),
    ),
    (
        "radar-deliveries",
        SlashAliasTarget::Mcp("radar_delivery_list"),
    ),
    ("radar-audit", SlashAliasTarget::Mcp("radar_audit_run")),
    (
        "radar-source-quality",
        SlashAliasTarget::Mcp("radar_source_quality"),
    ),
    (
        "radar-source-quality-trends",
        SlashAliasTarget::Mcp("radar_source_quality_trends"),
    ),
    (
        "radar-repair-fts",
        SlashAliasTarget::Mcp("radar_rebuild_fts"),
    ),
    ("edge-ack", SlashAliasTarget::Mcp("edge_event_ack")),
    (
        "edge-dead-letter",
        SlashAliasTarget::Mcp("edge_event_dead_letter"),
    ),
    ("edge-enqueue", SlashAliasTarget::Mcp("edge_event_enqueue")),
    ("edge-events", SlashAliasTarget::Mcp("edge_event_list")),
    ("edge-lease", SlashAliasTarget::Mcp("edge_event_lease")),
    ("edge-nack", SlashAliasTarget::Mcp("edge_event_nack")),
    (
        "email-drain",
        SlashAliasTarget::Mcp("email_drain_edge_events"),
    ),
    ("email-poll", SlashAliasTarget::Mcp("email_poll_edge")),
    ("email-reply", SlashAliasTarget::Mcp("email_reply_message")),
    ("email-send", SlashAliasTarget::Mcp("email_send_message")),
    (
        "import-claude",
        SlashAliasTarget::Cli(&["import", "claude"]),
    ),
    (
        "librarian-expand",
        SlashAliasTarget::Mcp("librarian_expand_topic"),
    ),
    ("mem0-add", SlashAliasTarget::Cli(&["memory", "mem0-add"])),
    (
        "mem0-delete",
        SlashAliasTarget::Cli(&["memory", "mem0-delete"]),
    ),
    (
        "mem0-forget-user",
        SlashAliasTarget::Cli(&["memory", "mem0-forget-user"]),
    ),
    (
        "mem0-history",
        SlashAliasTarget::Cli(&["memory", "mem0-history"]),
    ),
    (
        "mem0-search",
        SlashAliasTarget::Cli(&["memory", "mem0-search"]),
    ),
    (
        "mem0-update",
        SlashAliasTarget::Cli(&["memory", "mem0-update"]),
    ),
    (
        "memory-capture",
        SlashAliasTarget::Cli(&["memory", "capture"]),
    ),
    (
        "memory-delete",
        SlashAliasTarget::Cli(&["memory", "delete"]),
    ),
    ("memory-dream", SlashAliasTarget::Cli(&["memory", "dream"])),
    (
        "memory-events",
        SlashAliasTarget::Cli(&["memory", "events"]),
    ),
    (
        "memory-extract",
        SlashAliasTarget::Mcp("memory_extract_candidates"),
    ),
    ("memory-list", SlashAliasTarget::Cli(&["memory", "list"])),
    (
        "memory-recall",
        SlashAliasTarget::Cli(&["memory", "recall"]),
    ),
    (
        "memory-reject",
        SlashAliasTarget::Cli(&["candidate", "reject"]),
    ),
    (
        "memory-search",
        SlashAliasTarget::Cli(&["memory", "mem0-search"]),
    ),
    ("ops", SlashAliasTarget::Cli(&["ops"])),
    (
        "profile-delete",
        SlashAliasTarget::Cli(&["profile", "delete"]),
    ),
    ("profile-get", SlashAliasTarget::Cli(&["profile", "get"])),
    ("profile-list", SlashAliasTarget::Cli(&["profile", "list"])),
    (
        "profile-search",
        SlashAliasTarget::Cli(&["profile", "search"]),
    ),
    ("profile-set", SlashAliasTarget::Cli(&["profile", "set"])),
    (
        "project-create",
        SlashAliasTarget::Cli(&["project", "create"]),
    ),
    ("project-list", SlashAliasTarget::Cli(&["project", "list"])),
    (
        "project-status",
        SlashAliasTarget::Cli(&["project", "status-get"]),
    ),
    (
        "project-status-record",
        SlashAliasTarget::Cli(&["project", "status-record"]),
    ),
    (
        "project-sync-codex",
        SlashAliasTarget::HostOnly(
            "it needs the current Codex host thread inventory before writing a project snapshot",
        ),
    ),
    ("remember", SlashAliasTarget::Cli(&["memory", "mem0-add"])),
    (
        "research-brief",
        SlashAliasTarget::Cli(&["research", "brief"]),
    ),
    (
        "research-plan",
        SlashAliasTarget::Cli(&["research", "plan"]),
    ),
    (
        "research-runs",
        SlashAliasTarget::Cli(&["research", "runs"]),
    ),
    (
        "research-search",
        SlashAliasTarget::Cli(&["research", "search"]),
    ),
    (
        "research-task-complete",
        SlashAliasTarget::Cli(&["research", "complete-task"]),
    ),
    (
        "research-tasks",
        SlashAliasTarget::Cli(&["research", "tasks"]),
    ),
    (
        "research-workflow",
        SlashAliasTarget::Cli(&["research", "workflow"]),
    ),
    (
        "secret-delete",
        SlashAliasTarget::Cli(&["secrets", "delete-value"]),
    ),
    (
        "secret-list",
        SlashAliasTarget::Cli(&["secrets", "list-values"]),
    ),
    (
        "secret-ref-list",
        SlashAliasTarget::Cli(&["secrets", "list"]),
    ),
    (
        "secret-ref-set",
        SlashAliasTarget::Cli(&["secrets", "set-ref"]),
    ),
    (
        "secret-set",
        SlashAliasTarget::Cli(&["secrets", "set-value"]),
    ),
    (
        "source-card-add",
        SlashAliasTarget::Cli(&["source-card", "add"]),
    ),
    (
        "source-card-read",
        SlashAliasTarget::Cli(&["source-card", "read"]),
    ),
    (
        "source-card-search",
        SlashAliasTarget::Cli(&["source-card", "search"]),
    ),
    (
        "telegram-drain",
        SlashAliasTarget::Cli(&["telegram", "drain"]),
    ),
    ("telegram-inbox", SlashAliasTarget::Mcp("channel_list")),
    (
        "telegram-send",
        SlashAliasTarget::Cli(&["telegram", "send"]),
    ),
    (
        "watch-arxiv",
        SlashAliasTarget::Cli(&["wiki", "enqueue-arxiv"]),
    ),
    ("watch-rss", SlashAliasTarget::Cli(&["wiki", "enqueue-rss"])),
    ("wiki-add", SlashAliasTarget::Cli(&["wiki", "add"])),
    ("wiki-compile", SlashAliasTarget::Cli(&["wiki", "compile"])),
    ("wiki-expand", SlashAliasTarget::Cli(&["wiki", "expand"])),
    (
        "wiki-import-codex-swift-sources",
        SlashAliasTarget::Cli(&["wiki", "import-codex-swift-sources"]),
    ),
    ("wiki-job", SlashAliasTarget::Cli(&["wiki", "job"])),
    ("wiki-jobs", SlashAliasTarget::Cli(&["wiki", "jobs"])),
    ("wiki-list", SlashAliasTarget::Cli(&["wiki", "list"])),
    ("wiki-read", SlashAliasTarget::Cli(&["wiki", "read"])),
    (
        "wiki-run-arxiv",
        SlashAliasTarget::Cli(&["wiki", "run-arxiv"]),
    ),
    ("wiki-run-rss", SlashAliasTarget::Cli(&["wiki", "run-rss"])),
    ("wiki-search", SlashAliasTarget::Cli(&["wiki", "search"])),
    ("wiki-sources", SlashAliasTarget::Cli(&["wiki", "sources"])),
    (
        "worker-run-once",
        SlashAliasTarget::Cli(&["worker", "run-once"]),
    ),
    (
        "x-enqueue-search",
        SlashAliasTarget::Cli(&["x", "enqueue-recent-search"]),
    ),
    ("x-bookmarks", SlashAliasTarget::Cli(&["x", "bookmarks"])),
    (
        "x-import-bookmarks",
        SlashAliasTarget::Cli(&["x", "import-bookmarks"]),
    ),
    (
        "x-import-following-watch-sources",
        SlashAliasTarget::Cli(&["x", "import-following-watch-sources"]),
    ),
    (
        "x-import-json",
        SlashAliasTarget::Cli(&["x", "import-json"]),
    ),
    (
        "x-import-archive",
        SlashAliasTarget::Cli(&["x", "import-archive"]),
    ),
    (
        "x-discover-archives",
        SlashAliasTarget::Cli(&["x", "discover-archives"]),
    ),
    (
        "x-export-portable",
        SlashAliasTarget::Cli(&["x", "export-portable"]),
    ),
    (
        "x-validate-portable",
        SlashAliasTarget::Cli(&["x", "validate-portable"]),
    ),
    (
        "x-import-portable",
        SlashAliasTarget::Cli(&["x", "import-portable"]),
    ),
    (
        "x-extract-links",
        SlashAliasTarget::Cli(&["x", "extract-links"]),
    ),
    (
        "x-expand-links",
        SlashAliasTarget::Cli(&["x", "expand-links"]),
    ),
    ("x-links", SlashAliasTarget::Cli(&["x", "links"])),
    ("x-list", SlashAliasTarget::Cli(&["x", "list"])),
    ("x-report", SlashAliasTarget::Cli(&["x", "report"])),
    ("x-search", SlashAliasTarget::Cli(&["x", "recent-search"])),
    (
        "x-search-tweets",
        SlashAliasTarget::Cli(&["x", "search-tweets"]),
    ),
    ("x-research", SlashAliasTarget::Cli(&["x", "research"])),
    ("x-thread", SlashAliasTarget::Cli(&["x", "thread"])),
    (
        "x-repair-projections",
        SlashAliasTarget::Cli(&["x", "repair-projections"]),
    ),
    ("x-stats", SlashAliasTarget::Cli(&["x", "stats"])),
    (
        "x-watch-rebuild",
        SlashAliasTarget::Cli(&["x", "rebuild-definitive-watch-sources"]),
    ),
    (
        "x-watch-curate",
        SlashAliasTarget::Mcp("x_curate_watch_sources"),
    ),
    (
        "x-watch-curation-report",
        SlashAliasTarget::Mcp("x_watch_curation_report"),
    ),
    (
        "x-watch-curation-restore",
        SlashAliasTarget::Mcp("x_restore_watch_curation"),
    ),
    (
        "x-watch-manual-rules-import",
        SlashAliasTarget::Mcp("x_import_watch_manual_rules"),
    ),
    (
        "x-watch-profiles-enrich",
        SlashAliasTarget::Mcp("x_enrich_watch_profiles"),
    ),
];
