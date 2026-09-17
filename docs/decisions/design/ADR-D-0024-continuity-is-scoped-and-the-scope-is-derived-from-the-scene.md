---
status: proposed
adr_type: design
date: 2026-09-17
deciders: ["ebigunso"]
consulted: ["Claude Fable 5.1"]
informed: []
supersedes: [../superseded/ADR-D-0011-scope-continuity-around-arbitrary-entities-and-contexts--superseded-by-ADR-D-0024.md]
superseded_by: null
depends_on: [ADR-D-0009-entity-neutral-retrieval-policy.md, ADR-D-0019-discretion-is-disclosure-not-recall.md, ADR-D-0022-recall-is-activation-by-scene-cues.md]
---

# ADR-D-0024: Continuity is scoped to arbitrary entities and contexts, and the scope is derived from the scene rather than named by the application

## Context and Problem Statement

Relationship state, character signals, open loops, and commitments are only meaningful within a context: a relationship, a project, a place, a character, a conversation. ADR-D-0011 established that such state is scoped and that the scope may be any of those rather than a single user-and-assistant relationship, and it named a caller-facing `ContinuityScope` object with application-supplied IDs and a `CurrentContinuityView` generated per scope. A scope the application must name by ID requires it to discover the library's identifiers, which is a burden on consumers and a lookup surface the library does not want. The fork is whether scope stays a caller-facing identifier or becomes something the library derives from the circumstances it already knows.

## Decision

Continuity state is scoped, and the scope may be any continuing entity, pair of entities, thread, setting, or application-defined context. Continuity is never assumed to center on one user-and-assistant relationship.

The scope of a memory is derived from the scene it was formed in: the participants, the setting, the activity in progress, and any custom value the application already owns. Scope keys are internal. The library mints no scope identifier a caller must discover, and the retrieval input carries the scene, not a scope. Current continuity context for a scope is what retrieval returns for a scene that implies that scope; there is no separate view type. Reflection and any other scoped operation select their input by scope keys derived the same way, never by walking the whole graph through a broad entity.

## Why

Scoping keeps a signal valid in one context from becoming a brittle global persona patch, which is the reason ADR-D-0011 gave and which still holds. Deriving the scope from the scene keeps that benefit without asking the application to learn and supply identifiers, because everything a scope is made of is information the application already passes when it remembers.

## Rejected Alternatives

- A caller-facing scope object with application-supplied IDs, as ADR-D-0011 named it: rejected because it needs a lookup surface and burdens every consumer; reopen only if a consumer is found whose scope cannot be derived from any scene it can describe.
- Global continuity with no scope: rejected outright; it produces the persona overwrites scoping exists to prevent.
- Continuity centered on a user-and-assistant relationship: rejected outright under entity neutrality (ADR-D-0009).
- A separate current-continuity view type: rejected because it is the same retrieval with an empty content route (ADR-D-0022); reopen only under that record's condition.

## Decision Boundary

Invariant: scoped continuity state attaches to a scope derived from the scene; no core type centers continuity on a user or an assistant; the retrieval input carries a scene and never a scope identifier the library minted.

Not covered: the derivation rules from scene to scope key, the namespacing of custom values, whether a stored scope object with a label ever earns its place through a named consumer, and the reflection trigger vocabulary.

## Validation

- Tests show relationship state between arbitrary entities and character signals attached to non-user scopes, retrieved for a scene that implies the scope and absent for one that does not.
- Schema and API review reject any retrieval input that requires a library-minted scope identifier.
- Scoped input selection for reflection is bounded by scope keys and never scans all history through a broad entity.

## Revisit When

A consumer is found whose scope cannot be derived from any scene it can describe, or scope derivation proves ambiguous in a way custom values cannot resolve.

## More Information

- ADR-D-0011 in `superseded/` carries the original reasoning of 2026-05-08; its scoping decision is kept here in full.
- ADR-D-0019 defines the scene; ADR-D-0022 defines recall as activation by scene cues.
- The v0.2 design draft carries the derivation questions for planning.
