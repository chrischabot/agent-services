# Knowledge Investigation Execution Recurrence Proof

Status: Production Data Proof for copied-home resident worker recurrence with explicit local enqueue policy; not Operational wall-clock proof.

Input home: copied from `.arcwell-dev/proofs/knowledge-cluster-investigation-production-proof-20260626T053631Z/home`.

## Policy-Denied First Attempt

Processed jobs: 0
Investigation execution enqueue: {"enqueued": 0, "errors": ["kcl-1f6f4730aae00342:Anthropic: source-backed updates: policy deferred worker.enqueue: no matching policy rule; defer to explicit user or higher-level policy"], "inspected": 1, "jobs": [], "skipped": 1}

The first attempt is preserved because the copied home had no matching `worker.enqueue` policy for `arcwell-knowledge`, so the worker correctly refused to persist the execution job.

## Allowed Pass 1

Disposable policy file: `.arcwell-dev/proofs/knowledge-investigation-execution-recurrence-proof-20260626T064500Z/home-allowed/arcwell-policy.toml`

Processed jobs: 1
Job kind: `knowledge_cluster_expand`
Job status: `completed`
Expanded cluster: `kcl-c6fcba8254780cb3`
Investigation run created: `211d0274-1d34-443a-9b81-3c88bc23d581`
Investigation execution enqueue: {"enqueued": 1, "errors": [], "inspected": 1, "jobs": ["e2c34756-f41c-4385-9b23-7026b705cc07"], "skipped": 0}

This pass proves worker ordering: the worker may first drain an already-due expansion job while also enqueueing investigation execution work.

## Allowed Pass 2

Processed jobs: 1
Job kind: `knowledge_cluster_investigation_execute`
Job status: `completed`
Cluster: `kcl-1f6f4730aae00342`
Research run: `3c1c8a14-d595-447a-a3e4-c354f20b4fd7`
Research run status: `investigation_evidence_ready`
Executed task count: 4
Already completed task count: 0
Role run count: 4
Artifact count: 4
Quality findings: []

This pass is the main proof target: the resident worker executed the pre-existing 65-source cluster investigation without a manual `execute-cluster-investigation` command.

## Allowed Pass 3

Processed jobs: 1
Job kind: `knowledge_cluster_investigation_execute`
Job status: `completed`
Cluster: `kcl-c6fcba8254780cb3`
Research run status: `investigation_evidence_ready`
Executed task count: 4
Artifact count: 4

This pass proves the same recurrence path executed the investigation created by pass 1's expansion job.

## Allowed Pass 4 Replay/Suppression

Processed jobs: 0
Investigation execution enqueue: {"enqueued": 0, "errors": [], "inspected": 1, "jobs": [], "skipped": 1}

No execution job was processed after both eligible investigations had completed.

## Commands

```sh
ARCWELL_HOME=$PWD/.arcwell-dev/proofs/knowledge-investigation-execution-recurrence-proof-20260626T064500Z/home target/debug/arcwell worker run-once --max-jobs 1 > .arcwell-dev/proofs/knowledge-investigation-execution-recurrence-proof-20260626T064500Z/worker-run-once.json
ARCWELL_HOME=$PWD/.arcwell-dev/proofs/knowledge-investigation-execution-recurrence-proof-20260626T064500Z/home-allowed target/debug/arcwell worker run-once --max-jobs 1 > .arcwell-dev/proofs/knowledge-investigation-execution-recurrence-proof-20260626T064500Z/worker-run-once-allowed.json
ARCWELL_HOME=$PWD/.arcwell-dev/proofs/knowledge-investigation-execution-recurrence-proof-20260626T064500Z/home-allowed target/debug/arcwell worker run-once --max-jobs 1 > .arcwell-dev/proofs/knowledge-investigation-execution-recurrence-proof-20260626T064500Z/worker-run-once-allowed-replay.json
ARCWELL_HOME=$PWD/.arcwell-dev/proofs/knowledge-investigation-execution-recurrence-proof-20260626T064500Z/home-allowed target/debug/arcwell worker run-once --max-jobs 1 > .arcwell-dev/proofs/knowledge-investigation-execution-recurrence-proof-20260626T064500Z/worker-run-once-allowed-pass3.json
ARCWELL_HOME=$PWD/.arcwell-dev/proofs/knowledge-investigation-execution-recurrence-proof-20260626T064500Z/home-allowed target/debug/arcwell worker run-once --max-jobs 1 > .arcwell-dev/proofs/knowledge-investigation-execution-recurrence-proof-20260626T064500Z/worker-run-once-allowed-pass4.json
```

## Anti-Mirage Boundary

This proves that the resident worker can discover already-planned shared-cluster investigations and execute pending source-card-linked tasks without a manual `execute-cluster-investigation` command, once local policy authorizes `worker.enqueue`. It also proves fail-closed policy behavior and replay suppression in a copied production-data home. It is still not a wall-clock resident-service proof, not fresh external primary-source acquisition, not accepted model-backed synthesis, and not external digest recurrence.
