# Knowledge Report Quality Gate Production-Corpus Proof

## Feature Name And Status

Feature: source-card-backed knowledge report quality gate requiring human-readable analysis, uncertainty, citations, and explicit next investigation steps.

Status: Production Data Proof for copied-home foreground report generation over existing source cards. Not Operational.

## User-Visible Claim

Arcwell rejects knowledge reports that are link dumps, uncited prose, too-thin commentary, or nice prose with no follow-up investigation path. Generated projection reports now include a `Next Investigation` section that tells the system and reader what should be verified, corroborated, compared against the wiki, or expanded before stronger claims or outbound alerts are promoted.

This proof does not claim model-backed synthesis, autonomous primary-source investigation, live X provider freshness, scheduled recurrence, or external delivery recurrence.

## Inputs And Outputs

Input:

- Copied real Arcwell home from `/Users/chabotc/.arcwell` into `.arcwell-dev/proofs/knowledge-report-quality-production-proof-20260626T052350Z/home`.
- Existing durable source-card corpus in the copied home.
- Command cap: `--max-source-cards 500 --min-group-size 2 --max-clusters 12`.

Output:

- `.arcwell-dev/proofs/knowledge-report-quality-production-proof-20260626T052350Z/cluster-backlog.json`
- 12 projected knowledge reports in the copied home.
- 157 accepted source cards, 343 skipped source cards, 22 groups considered, and 0 projection-time warnings.
- All 12 projected reports contained `## Next Investigation`, `## Evidence`, and `Confidence and uncertainty`.

## Durable State Written

The foreground clustering command ran against the copied SQLite home and wrote knowledge clusters, editorial decisions, reports, and source-card-backed projections in that disposable home.

The real home was not mutated by this proof.

## Report Quality Gate

The report audit now rejects:

- bodies under the human-readable analysis threshold
- reports missing uncertainty or confidence language
- reports missing exact source-card citations
- reports that look like raw link dumps
- reports with too little explanatory prose
- reports missing a next-investigation section
- reports missing concrete follow-up actions such as primary-source verification, corroboration, comparison, wiki lookup, or follow-up research

The deterministic projection renderer now emits:

- `## What happened`
- `## Why it matters`
- `## Evidence`
- `## Next Investigation`
- `## Confidence and uncertainty`
- `## Warnings`

## Tests Added Or Tightened

- `severe_unified_knowledge_cluster_editorial_and_report_gate_rejects_link_dump` now includes a report body that has prose, uncertainty, and source-card citations but no next-investigation section; it fails with `report_missing_next_investigation_section`.
- Accepted manual report fixtures now include concrete next-investigation steps.
- Existing projection and wiki expansion tests prove generated reports/pages still pass after the stricter gate.

## Commands Run

```sh
cargo fmt -- --check
cargo test -p arcwell-core severe_unified_knowledge_cluster_editorial_and_report_gate_rejects_link_dump -- --nocapture
cargo test -p arcwell-core severe_knowledge_projection_from_source_card_query_creates_human_report -- --nocapture
cargo test -p arcwell-core severe_knowledge_cluster_wiki_audit_rejects_empty_uncited_link_dump -- --nocapture
cargo test -p arcwell-core knowledge_cluster -- --nocapture
cargo test -p arcwell-core knowledge_projection -- --nocapture
cargo test -p arcwell-core unified_knowledge -- --nocapture
scripts/arcwell-dev sync
ARCWELL_HOME=.arcwell-dev/proofs/knowledge-report-quality-production-proof-20260626T052350Z/home target/debug/arcwell knowledge cluster-backlog --max-source-cards 500 --min-group-size 2 --max-clusters 12
```

`scripts/arcwell-dev sync` rebuilt `target/debug/arcwell` before the trusted production-corpus proof.

## False Start Caught

An earlier copied-home proof directory, `.arcwell-dev/proofs/knowledge-report-quality-production-proof-20260626T052253Z`, was generated before rebuilding `target/debug/arcwell`. It produced reports without `## Next Investigation`. That stale-binary artifact is intentionally not the promoted proof.

## Adversarial Review Judgment

Judgment: promote narrowly, hold broadly.

Promote:

- The report gate now rejects one more mirage class: readable-but-terminal prose that cites evidence but does not drive investigation, wiki expansion, or stronger verification.
- The deterministic renderer produces reports with explicit next-investigation work over a copied production corpus.

Hold:

- The reports are still deterministic and source-card summarizing, not model-backed analyst synthesis.
- The system still needs autonomous investigation jobs that fetch primary sources, compare against existing wiki/entity history, and revise pages over time.
- Live X credential freshness, wall-clock recurrence, and external delivery recurrence remain open.
