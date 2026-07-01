use crate::*;

pub(crate) fn proof(store: Store, args: ProofCommand) -> Result<()> {
    match args.command {
        ProofSubcommand::Record {
            scope,
            title,
            proof_level,
            status,
            summary,
            artifact_root,
            reviewer,
            claims_json,
            artifacts_json,
            checks_json,
            metadata_json,
        } => {
            let claims = parse_typed_json::<Vec<ProofClaimInput>>(&claims_json, "--claims-json")?;
            let artifacts =
                parse_typed_json::<Vec<ProofArtifactInput>>(&artifacts_json, "--artifacts-json")?;
            let checks = parse_typed_json::<Vec<ProofCheckInput>>(&checks_json, "--checks-json")?;
            let metadata = parse_json_arg(&metadata_json, "--metadata-json")?;
            print_json(&store.record_proof_packet(ProofPacketInput {
                scope,
                title,
                proof_level,
                status,
                summary,
                artifact_root,
                reviewer,
                claims,
                artifacts,
                checks,
                metadata,
            })?)
        }
        ProofSubcommand::Read { packet_id } => print_json(&store.read_proof_packet(&packet_id)?),
        ProofSubcommand::List { scope, limit } => {
            print_json(&store.list_proof_packets(scope.as_deref(), limit)?)
        }
        ProofSubcommand::Latest { capability } => {
            print_json(&store.latest_proof_packet(&capability)?)
        }
        ProofSubcommand::VerifyPacket { path } => {
            print_json(&store.verify_proof_packet_file(path)?)
        }
        ProofSubcommand::Promote {
            packet_id,
            reviewer,
        } => print_json(&store.promote_proof_packet(&packet_id, &reviewer)?),
        ProofSubcommand::ReviewRecord {
            scope,
            title,
            reviewer,
            requested_proof_level,
            judgment,
            summary,
            strongest_fake_done_path,
            packet_id,
            refutations_json,
            skipped_categories_json,
            findings_json,
            metadata_json,
        } => {
            let refutations = parse_json_arg(&refutations_json, "--refutations-json")?;
            let skipped_categories =
                parse_json_arg(&skipped_categories_json, "--skipped-categories-json")?;
            let findings = parse_typed_json::<Vec<AdversarialReviewFindingInput>>(
                &findings_json,
                "--findings-json",
            )?;
            let metadata = parse_json_arg(&metadata_json, "--metadata-json")?;
            print_json(&store.record_adversarial_review(AdversarialReviewRunInput {
                packet_id,
                scope,
                title,
                reviewer,
                requested_proof_level,
                judgment,
                summary,
                strongest_fake_done_path,
                refutations,
                skipped_categories,
                findings,
                metadata,
            })?)
        }
        ProofSubcommand::ReviewRead { review_id } => {
            print_json(&store.read_adversarial_review(&review_id)?)
        }
        ProofSubcommand::ReviewList {
            scope,
            packet_id,
            limit,
        } => print_json(&store.list_adversarial_reviews(
            scope.as_deref(),
            packet_id.as_deref(),
            limit,
        )?),
    }
}

fn parse_typed_json<T: for<'de> Deserialize<'de>>(raw: &str, label: &str) -> Result<T> {
    let value = parse_json_arg(raw, label)?;
    serde_json::from_value(value).with_context(|| format!("parsing {label} shape"))
}
