use crate::*;

pub(crate) fn import(store: Store, args: ImportCommand) -> Result<()> {
    match args.command {
        ImportSubcommand::Claude {
            path,
            dry_run,
            limit,
            user_id,
            write_candidates,
        } => {
            if dry_run && write_candidates {
                bail!("--dry-run and --write-candidates cannot be used together");
            }
            let mode = if write_candidates {
                "write_candidates"
            } else if dry_run {
                "dry_run"
            } else {
                "analyze"
            };
            let import_run_id = store.start_import_run(
                "claude",
                &path.display().to_string(),
                mode,
                json!({
                    "limit": limit,
                    "user_id_configured": user_id.is_some()
                }),
            )?;
            let result = (|| -> Result<ClaudeImportReport> {
                let mut report = analyze_claude_export(&path, limit, user_id.as_deref())?;
                if write_candidates {
                    let mut existing = HashSet::new();
                    for status in ["pending", "applied", "rejected"] {
                        for candidate in store.list_candidates(status)? {
                            existing.insert(candidate_dedupe_key(
                                &candidate.target,
                                &candidate.kind,
                                &candidate.content,
                                &candidate.source_ref,
                                candidate.user_id.as_deref(),
                            ));
                        }
                    }
                    for candidate in &report.candidates {
                        let key = candidate_dedupe_key(
                            &candidate.target,
                            &candidate.kind,
                            &candidate.content,
                            &candidate.source_ref,
                            candidate.user_id.as_deref(),
                        );
                        if !existing.insert(key) {
                            report.duplicates_suppressed += 1;
                            continue;
                        }
                        store.add_candidate_with_operation(
                            &candidate.target,
                            &candidate.kind,
                            &candidate.content,
                            &candidate.sensitivity,
                            &candidate.source_ref,
                            &candidate.operation,
                            candidate.memory_id.as_deref(),
                            candidate.user_id.as_deref(),
                            candidate.metadata.clone(),
                        )?;
                        report.candidates_written += 1;
                    }
                }
                Ok(report)
            })();
            match result {
                Ok(mut report) => {
                    let record = store.finish_import_run(
                        &import_run_id,
                        ImportRunFinish {
                            status: "completed".to_string(),
                            conversations_seen: report.conversations_seen,
                            conversations_sampled: report.conversations_sampled,
                            candidates_seen: report.candidates_seen,
                            candidates_sampled: report.candidates_sampled,
                            candidates_written: report.candidates_written,
                            duplicates_suppressed: report.duplicates_suppressed,
                            error: None,
                            metadata: json!({
                                "resolved_source_kind": report.source_kind.clone(),
                                "resolved_source_path": report.source_path.clone(),
                                "dry_run": dry_run,
                                "write_candidates": write_candidates
                            }),
                        },
                    )?;
                    report.import_run_id = Some(record.id);
                    print_json(&report)
                }
                Err(error) => {
                    let _ = store.finish_import_run(
                        &import_run_id,
                        ImportRunFinish {
                            status: "failed".to_string(),
                            conversations_seen: 0,
                            conversations_sampled: 0,
                            candidates_seen: 0,
                            candidates_sampled: 0,
                            candidates_written: 0,
                            duplicates_suppressed: 0,
                            error: Some(error.to_string()),
                            metadata: json!({
                                "dry_run": dry_run,
                                "write_candidates": write_candidates
                            }),
                        },
                    );
                    Err(error)
                }
            }
        }
        ImportSubcommand::Runs { limit } => print_json(&store.list_import_runs(limit)?),
    }
}

pub(crate) fn candidate(store: Store, args: CandidateCommand) -> Result<()> {
    match args.command {
        CandidateSubcommand::List { status } => print_json(&store.list_candidates(&status)?),
        CandidateSubcommand::Apply { id } => print_json(&store.apply_candidate(&id)?),
        CandidateSubcommand::Reject { id } => print_json(
            &json!({ "ok": store.reject_candidate(&id, None)?, "id": id, "status": "rejected" }),
        ),
    }
}

pub(crate) fn backup(store: Store, args: BackupCommand) -> Result<()> {
    match args.command {
        BackupSubcommand::Create => {
            let path = store.create_backup()?;
            print_json(&json!({ "ok": true, "path": path }))
        }
        BackupSubcommand::Status => print_json(&store.latest_backup()?),
        BackupSubcommand::Verify => print_json(&store.verify_latest_backup()?),
        BackupSubcommand::Restore { .. } => unreachable!("restore is handled before store open"),
    }
}

pub(crate) fn cost(store: Store, args: CostCommand) -> Result<()> {
    match args.command {
        CostSubcommand::Add {
            package,
            job_id,
            provider,
            model,
            estimated_usd,
            actual_usd,
        } => {
            let id = store.add_cost(
                &package,
                &job_id,
                &provider,
                &model,
                estimated_usd,
                actual_usd,
            )?;
            print_json(&json!({ "ok": true, "id": id }))
        }
        CostSubcommand::SetPolicy {
            scope,
            key,
            limit_usd,
            kill_switch,
            override_until,
        } => print_json(&store.set_cost_policy(
            &scope,
            &key,
            limit_usd,
            kill_switch,
            override_until.as_deref(),
        )?),
        CostSubcommand::Policies => print_json(&store.list_cost_policies()?),
        CostSubcommand::Check {
            package,
            provider,
            source,
            projected_usd,
        } => print_json(&store.cost_decision(
            &package,
            &provider,
            source.as_deref(),
            projected_usd,
        )?),
        CostSubcommand::Summary => {
            let (estimated_usd, actual_usd, entries) = store.cost_summary()?;
            let recent_decisions = store.list_cost_decisions(25)?;
            print_json(&json!({
                "estimated_usd": estimated_usd,
                "actual_usd": actual_usd,
                "entries": entries,
                "recent_decisions": recent_decisions
            }))
        }
    }
}

pub(crate) fn policy(store: Store, args: PolicyCommand) -> Result<()> {
    match args.command {
        PolicySubcommand::Check(request) => {
            print_json(&store.policy_check(policy_request_from_args(request)?)?)
        }
        PolicySubcommand::Explain(request) => {
            print_json(&store.policy_explain(policy_request_from_args(request)?)?)
        }
        PolicySubcommand::List { limit } => print_json(&store.list_policy_decisions(limit)?),
        PolicySubcommand::Rules => print_json(&store.list_policy_rules()?),
        PolicySubcommand::Override {
            request,
            reason,
            expires_at,
        } => print_json(&store.create_policy_allow_override(
            policy_request_from_args(request)?,
            &reason,
            &expires_at,
        )?),
        PolicySubcommand::Approvals { status } => {
            print_json(&store.list_policy_approvals(status.as_deref())?)
        }
        PolicySubcommand::Approve { id, reason } => {
            print_json(&store.approve_policy_approval(&id, reason.as_deref())?)
        }
        PolicySubcommand::Reject { id, reason } => {
            print_json(&store.reject_policy_approval(&id, reason.as_deref())?)
        }
    }
}

pub(crate) fn policy_request_from_args(args: PolicyRequestArgs) -> Result<PolicyRequest> {
    let metadata = args
        .metadata_json
        .map(|raw| serde_json::from_str::<Value>(&raw).context("parsing --metadata-json"))
        .transpose()?
        .unwrap_or_else(|| json!({}));
    Ok(PolicyRequest {
        action: args.action,
        package: args.package,
        provider: args.provider,
        source: args.source,
        channel: args.channel,
        subject: args.subject,
        target: args.target,
        projected_usd: args.projected_usd,
        metadata,
        untrusted_excerpt: args.untrusted_excerpt,
    })
}

pub(crate) fn secrets(store: Store, args: SecretsCommand) -> Result<()> {
    match args.command {
        SecretsSubcommand::SetRef {
            name,
            location,
            scope,
            expires_at,
        } => {
            store.set_secret_ref_with_policy(
                &name,
                &location,
                &scope,
                expires_at.as_deref(),
                "cli",
            )?;
            print_json(&json!({ "ok": true, "name": name }))
        }
        SecretsSubcommand::List => print_json(&store.list_secret_refs()?),
        SecretsSubcommand::SetValue {
            name,
            value,
            scope,
            provider,
            expires_at,
        } => {
            store.set_secret_value_with_policy(
                &name,
                &value,
                &scope,
                provider.as_deref(),
                expires_at.as_deref(),
                "cli",
            )?;
            print_json(&json!({ "ok": true, "name": name }))
        }
        SecretsSubcommand::GetValue { name } => {
            print_json(&store.get_secret_value_with_policy(&name, "cli")?)
        }
        SecretsSubcommand::ListValues => print_json(&store.list_secret_values()?),
        SecretsSubcommand::Health => print_json(&store.secret_health()?),
        SecretsSubcommand::DeleteValue { name } => print_json(
            &json!({ "ok": store.delete_secret_value_with_policy(&name, "cli")?, "name": name }),
        ),
    }
}

pub(crate) fn cursors(store: Store, args: CursorCommand) -> Result<()> {
    match args.command {
        CursorSubcommand::List => print_json(&store.list_cursors()?),
        CursorSubcommand::Get { key } => print_json(&store.get_cursor(&key)?),
    }
}

pub(crate) fn print_json(value: &impl Serialize) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    write_json_pretty(&mut stdout, value)
}

pub(crate) fn write_json_pretty(writer: &mut impl Write, value: &impl Serialize) -> Result<()> {
    let mut output = serde_json::to_string_pretty(value)?;
    output.push('\n');

    if let Err(err) = writer.write_all(output.as_bytes()) {
        if err.kind() == std::io::ErrorKind::BrokenPipe {
            return Ok(());
        }
        return Err(err.into());
    }
    if let Err(err) = writer.flush() {
        if err.kind() == std::io::ErrorKind::BrokenPipe {
            return Ok(());
        }
        return Err(err.into());
    }
    Ok(())
}
