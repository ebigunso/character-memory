---
status: accepted
adr_type: implementation
date: 2026-07-13
deciders: ["ebigunso"]
consulted: ["Claude Fable 5 (orchestrator)"]
informed: []
supersedes: []
superseded_by: null
---

# ADR-I-0020: Restart identity via caller-supplied MemoryIds, not a public lookup surface

## Context and Problem Statement

The continuity evaluation harness (ADR-I-0019) must measure restart behavior: drop a `CharacterMemory` instance mid-scenario, reconstruct it over the same persistent stores, and prove that previously written memories survived with their identities intact.
Proving that requires the caller to re-find the `MemoryId`s of its own objects after the restart.

An audit of the public facade found that store-side re-association is not possible through the public API.
The facade exposes exactly eight methods (`remember`, `retrieve`, `correct`, `forget`, `link`, `prepare`, `validate_plan`, `commit`).
There is no lookup by external id, no enumeration, and no query by source reference; diagnostic listing exists only behind crate-internal ports.
Retrieval can opportunistically resurface some identities, but it is relevance-dependent by design and cannot guarantee complete re-association.

The same audit found the enabler that resolves the problem without any API change: public draft types accept caller-supplied `MemoryId`s, `RememberOutcome` returns the persisted object and link ids, `link` returns the created link, and `correct` reports generated replacement ids through `LifecycleMutationOutcome`.
A caller can therefore know every identity it owns at write time, either by supplying deterministic ids or by persisting the ids the library returns.

This ADR records the resulting contract, because the absence of a lookup surface is otherwise invisible to readers of the code and could be mistaken for an oversight.

## Decision Drivers

- Every lifecycle mutation (`correct`, `forget`, `link`) takes `MemoryId`s directly, so callers must hold ids to use the API at all.
- The roadmap constraint that evaluation work adds no new public facade methods, and the general principle that the facade stays minimal and consumer-driven.
- Identity re-association must be complete to be useful; relevance-dependent rediscovery through retrieval cannot provide completeness guarantees.
- The primary consumer profile is an application doing day-to-day memory storage and retrieval, which has its own persistent storage and obtains ids naturally from write outcomes and retrieval packs.
- Evaluation tooling must not grow library surface that no product usecase has yet demanded.

## Decision

Identity across restart is the caller's responsibility, discharged at write time rather than by post-restart discovery.

- Callers that need to reference memories across process or instance restarts either supply deterministic `MemoryId`s in drafts (including replacement drafts in corrections), or durably persist every id the API returns — persisted object and link ids in `RememberOutcome`, the created link id from `link`, and generated replacement ids reported through `LifecycleMutationOutcome` from `correct` — keyed by their own external identifiers.
- The public facade deliberately provides no lookup-by-external-id, enumeration, or query-by-source-reference surface; this absence is intentional, not a gap.
- Retrieval is the verification mechanism for persistence, not the identity-recovery mechanism: after reconstruction over the same stores, a caller confirms survival by retrieving scripted or known content and checking the returned ids against its own mapping.

## Character Memory Relevance

Append-only permanence and provenance discipline make identity a first-class, caller-visible concept: corrections supersede and forgetting suppresses, both by explicit `MemoryId`.
Keeping identity bookkeeping with the caller preserves the library's stance that it does not infer meaning or ownership; the caller decides what an external identity maps to, and the library guarantees the mapped objects persist.

## Implementation Impact

- No library change: the contract is satisfied by existing draft fields and `RememberOutcome`.
- Integration documentation must state the caller obligation explicitly, so consumers do not discover it only when a restart loses their mapping.
- The private evaluation harness implements this contract as a durable external-id registry persisted alongside its stores, with an open/reattach lifecycle that requires every configured durable store to be present before reporting restored identities.
- Caller-supplied deterministic ids additionally give consumers stable identity under replay: a repeated write reuses the same ids rather than minting new ones. This is identity stability, not full ingest idempotency — direct draft replay can regenerate defaulted timestamps and repeat derived-store side effects; exact retry semantics belong to replaying the same prepared plan through the staged write path.

## Considered Options

1. Caller-supplied deterministic `MemoryId`s plus caller-persisted id mappings, with retrieval as verification only.
2. Add a public lookup/enumeration method (by external id, raw ref, or object listing) to the facade.
3. Rely on retrieval to rediscover identities after restart.

## Decision Outcome

Chosen option: **Option 1**.

Option 2 was rejected because it violates the no-new-facade-methods constraint, grows the public surface solely to serve evaluation tooling, and pre-builds an API that no product usecase has demanded.
Option 3 was rejected because relevance-dependent rediscovery is incomplete by design and cannot back a completeness claim.
Option 1 requires no library change, keeps the facade minimal, and turns identity management into an explicit, testable caller contract.

## Consequences

### Positive

- The facade stays at eight methods; evaluation needs added zero public surface.
- The contract is deterministic and auditable: the harness proved exact-mapping restoration across restart for every identity category using only the public API.
- Identity stability under replay falls out of deterministic caller-supplied ids.

### Negative / Tradeoffs

- Every caller that mutates memories across restarts carries a bookkeeping obligation, and a caller that loses its mapping has no complete recovery path through the public API.
- Compliance-style complete deletion keyed by external identity cannot be built on retrieval and currently depends on the caller's mapping being lossless.

## Validation

- The continuity harness restart scenarios assert exact forward and reverse mappings for all identity categories after adapter drop and reconstruction over persistent stores, plus post-restart retrieval verification, using only the public API.
- Documentation review: library integration docs state the caller obligation (supply deterministic ids or persist `RememberOutcome` ids).

## Revisit When

- A caller-facing requirement emerges for complete deletion or suppression keyed by external source identity (for example user-data removal or retention enforcement across everything derived from one conversation).
  Revisit as a scoped forget-by-provenance operation, not as a general lookup or enumeration surface; completeness there cannot depend on caller bookkeeping.
- A memory-inspection or browsing product surface is planned, which would need true enumeration and is a deliberate product decision rather than an extension of this contract.

## More Information

- ADR-I-0019 (harness placement and the public-API-only consumption boundary).
- The v0.1.4 continuity evaluation harness plan (completed), Task_2 findings appendix: facade audit, the restart-identity infeasibility finding, and the caller-supplied-id enabler.
- Related permanence semantics: corrections supersede and forgetting suppresses by explicit id; nothing is destructively deleted.
