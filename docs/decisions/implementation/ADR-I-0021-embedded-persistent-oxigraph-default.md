---
status: accepted
adr_type: implementation
date: 2026-07-18
deciders: ["ebigunso"]
consulted: ["Claude Fable 5"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-I-0021: Embedded persistent Oxigraph is the validated default graph store; the HTTP service mode is removed

## Context and Problem Statement

v0.1.1 introduced three graph store modes (`service`, `persistent`, `in_memory`) and documented the Docker-backed Oxigraph HTTP service mode as the application default.
That default claim was never recorded in an ADR: ADR-I-0003 describes embedded graph-authoritative storage, and the service-default guidance lived only in the v0.1.1 phase document, the roadmap, and the README.

The v0.1.5 eval-driven closeout exposed a validation asymmetry.
Every piece of live evidence across the v0.1 family — the integration test suite, the continuity evaluation harness including restart and reattach scenarios, and all tuning measurements — exercises the embedded persistent path.
The HTTP service adapter has no automated live coverage: its only live test is manual and excluded from CI, and the remaining service tests assert SPARQL query text shape only.
Shipping a default that the evidence does not cover misrepresents what the library validates.

A deployment-shape analysis of the intended use cases resolved the question of whether the service mode earns a validation investment instead.

## Decision Drivers

- Defaults must match validation evidence.
- Every intended use case that can run today embeds the library in a single host process; desktop and game deployments cannot assume a container runtime on end-user machines.
- The one deployment shape embedded storage cannot serve — multiple replicas sharing one character's memory — is not served by the current HTTP adapter either: retrieval stats are a process-local store, restart identity is a caller-owned local mapping (ADR-I-0020), the write path has no cross-request concurrency contract, and the adapter has no authentication or tenancy story.
- Minimizing unvalidated code surface: an unvalidated adapter must track every graph-authority port change without evidence that it still works.
- Keeping a consumer-facing README truthful: retaining the service mode preserves the appearance of a cloud path while delivering none of its requirements.

## Decision

Remove the Oxigraph HTTP service mode.
`GraphStoreMode::Persistent` (embedded persistent storage at a caller-configured local path) becomes the default; `in_memory` remains for tests and explicit fixture runs.
Configuration parsing rejects the removed `service` value with an explicit migration hint directing endpoint-URL configurations to a local store path.
Historical phase documents are left unchanged as append-only records; this ADR records the supersession of their service-default guidance.

The `GraphAuthorityStore` port is unchanged.
The retired adapter's targeted SPARQL query design remains documented in the completed service-remote-SPARQL plan as reference material for any future remote adapter; it is not code to resurrect verbatim.

## Considered Options

1. Drop service mode: remove the HTTP adapter and Docker artifacts; default to embedded persistent.
2. Pivot to service mode: make it the validated default by extending the evaluation harness with container lifecycle control, service-aware restart semantics, and namespace isolation.
3. Keep both, demoting service mode to documented experimental/unvalidated status.
4. Keep service mode as the designated cloud path.

## Decision Outcome

Chosen option: **Option 1**.

Option 2 requires container orchestration in the harness, a namespace-safe graph naming scheme for a shared server dataset, redefined reattach and durability checks, and re-running the evaluation family against the service — cost disproportionate to demonstrated need, and it would invalidate the tuning evidence produced on the embedded path.

Option 3 leaves a standing parity liability: an unvalidated adapter that must track every graph-authority port change without evidence it still works, while continuing to imply a supported deployment mode.

Option 4 fails on the facts: a shared graph service alone does not enable multi-replica deployments while retrieval stats and identity mapping remain process-local, and the adapter has no concurrency contract and no authentication or tenancy model.
A real multi-replica capability requires a designed remote graph-authority phase covering shared stats, shared identity, write concurrency, authentication, and tenancy.

## Character Memory Relevance

The library's use cases center on persistent characters embedded in a host application: desktop companions, games with persistent inhabitants, research systems, and per-character or per-shard cloud processes.
For all of these, memory belongs in application-owned local storage — copyable, backup-friendly, and free of infrastructure the application cannot guarantee.
The authority split is unaffected: Qdrant suggests vector candidates, stats guide fanout, and the embedded Oxigraph store decides graph truth and final inclusion.

## Consequences

### Positive

- The shipped default matches the validation evidence.
- The validated code surface shrinks; no container dependency remains on the graph-authority path.
- Documentation and decision records become coherent: the local story is that graph memory lives in the application's data directory.

### Negative / Tradeoffs

- Adopters who followed the prior service-mode instructions must migrate an endpoint URL configuration to a local store path; the configuration error message carries the hint.
- Multi-process graph access is explicitly out of scope until a future phase designs a remote graph authority with its own validation budget.
- Qdrant remains the one required external service; a fully self-contained local deployment additionally needs an embedded vector-recall option, which is recorded as a separate roadmap concern rather than solved here.

## Validation

- The full test suite passes with the service surface removed; the default-mode construction test asserts persistent mode.
- Configuration with `GRAPH_STORE_MODE=service` fails with the migration hint.
- Live integration and continuity evaluation evidence is unchanged, since it already exercises the embedded path.
- A documentation sweep confirms no remaining service-default claims outside historical phase documents and this ADR.

## Revisit When

- A concrete multi-replica deployment need materializes — multiple application processes serving the same character's memory concurrently. Revisit as a designed remote graph-authority phase covering shared retrieval stats, shared identity mapping, write concurrency, authentication, and tenancy; the retired adapter's targeted SPARQL query design (see the completed service-remote-SPARQL plan) is the starting reference.
- An embedded vector-recall option is pursued to make fully self-contained local deployment possible; that work is independent of this decision but shares its motivation.

## More Information

- ADR-I-0003 (backend defaults; embedded graph-authoritative storage — unchanged by this decision).
- ADR-I-0020 (restart identity via caller-supplied ids; one of the process-local components that bounds deployment shape).
- Roadmap phase document `docs/design/roadmap-phases/v0_1_1_persistent_graph_authority.md` (historical record of the superseded service-default guidance).
- The completed service-remote-SPARQL graph-authority plan (targeted SPARQL query design reference).
