# Continuity Situation Catalog

Status: living design document. Added at the v0.1.5 closeout (2026-07-18) as mechanism-independent planning input for phase design and evaluation scenarios.

## Purpose

This catalog describes situations a persistent character encounters and the response that would make its continuity feel comparable to a real human being's.
It is deliberately written from lived experience inward, not from the library's current mechanisms outward: designing evaluation scenarios only from what the mechanism supports can only confirm what the mechanism already does.
Each situation records the ideal behavior, the earliest phase whose concepts can support it, the evaluation tier that can test it, and its coverage status in the evaluation scenario library.

The catalog spans the deployment spectrum:

- A. Dedicated companion: one primary human the character mainly supports.
- B. Small circle: a household, team, or party — a few humans with ongoing individual and shared relationships.
- C. Independent entity: no dedicated user; the character interacts with many humans or systems while living out its own life.

## Evaluation tiers

- Tier R (retrieval-level, deterministic): assertable on retrieval outputs — pack membership, ordering, rationale, lifecycle state — under the deterministic harness contract. No model judging.
- Tier B (behavioral, judged): requires generating character responses and judging qualities such as gracefulness or tact; inherently non-deterministic and model-graded. Out of scope for the deterministic harness; a future behavioral evaluation tier owns it.

## Embedding realism principle

Where a situation's difficulty comes from semantic geometry — near-miss topics, sparse references, graded similarity — evaluation scenarios must use real embeddings from an embedding model, not synthetic orthogonal proxies.
Synthetic cluster embeddings make semantic separation perfect by construction and therefore cannot exercise semantic confusion.
To preserve determinism and the no-external-calls-at-eval-time contract, real embeddings are generated once per text in an explicit offline step, persisted alongside the fixtures, and loaded from that store on every run.
Structural situations (lifecycle, persistence, graph reachability, bounded expansion) may keep synthetic embeddings where geometry is not the point.

---

## A. Dedicated companion situations

### A1. The return after absence

The user returns after weeks or months.
Ideal: acknowledge the gap proportionally, resume open loops by asking about their outcomes rather than asserting stale state, and re-frame all references to elapsed time.
Failure shapes: greeting a year like yesterday; reciting an open loop as current fact; requiring the user to re-establish context.
Earliest phase: v0.1 (retrieval of dormant threads and open items); commitment/open-loop lifecycle is v0.2.
Tier: R for surfacing the right dormant material with elapsed-time metadata; B for the asking-not-asserting quality.
Coverage: partial (long-gap-recall covers dormant retrieval at token scale).

### A2. The unstated reference

The user says "she called again" and expects resolution from shared history.
Ideal: resolve confidently when history makes one referent dominant, ask naturally when referents compete, never resolve to a creepy-wrong candidate.
Earliest phase: v0.1 (entity anchoring); confidence-graded resolution behavior is B-tier.
Tier: R for entity-anchored retrieval from semantically sparse queries; B for the resolution dialogue.
Coverage: gap — no scenario queries with a sparse reference whose target shares little semantic surface with the query; requires real embeddings.

### A3. The emotional callback

Recall how a topic landed last time, not only what happened, and let that shape approach.
Earliest phase: v0.1 stores the surfaces (observation vs episode); character signals are v0.2.
Tier: R for distinct-surface retrieval (facts vs reading); B for tonal adjustment.
Coverage: partial (surface-contribution proves distinct surfaces are retrievable; nothing tests their differential use).

### A4. The late correction with propagation

Months after a fact was stored — and after it has been retrieved and linked repeatedly — the user corrects it.
Ideal: update gracefully without relitigating, and deactivate the implicated web of dependent assumptions, not only the corrected object.
Earliest phase: v0.1 for explicit multi-object correction chains supplied by the caller; inferred propagation is a later concept (the library does not infer meaning).
Tier: R for lifecycle correctness of an entrenched, much-referenced memory corrected long after write; B for graceful handling.
Coverage: gap — all current corrections are immediate; none lands on an entrenched memory with accumulated links and retrieval history.

### A5. Preference and identity drift

Tastes and circumstances change gradually with no correction event ever fired.
Ideal: track current state with historical awareness ("you used to take sugar — still off it?"); never assert a stale preference as current.
Earliest phase: v0.2 (current continuity views, reflection); the underlying accumulating-absence signal has no v0.1 representation.
Tier: R for current-view correctness once v0.2 exists; B for the checking-in behavior.
Coverage: none (correctly — concepts absent).

### A6. Unprompted temporal awareness

Anniversaries, elapsed-time milestones, seasonal recurrence, and interval reasoning ("how often did X happen").
Ideal: the character's sense of now meets its memory timeline without being asked.
Earliest phase: v0.1 for interval/recurrence retrieval quality; proactive surfacing is application/prompt-integration territory.
Tier: R for temporal query classes (order, interval, recurrence, one-off vs repeated); B for spontaneity.
Coverage: gap — only recency/order pairs exist; the richer temporal patterns specified in the v0.1.4 harness design were never implemented.

### A7. Being contradicted by history

The user states something inconsistent with their own earlier statements.
Ideal: hold both, favor the current statement, retain the record, surface the discrepancy only when it helps.
Earliest phase: v0.1 append-only supersession covers the record; discrepancy detection is v0.3 (claims, contradictions).
Tier: R for record integrity; B for when-to-surface judgment.
Coverage: partial (correction-chains covers explicit supersession).

### A8. The character's own commitments and past self

Keep its own promises, own its past statements and errors, stay consistent with its own opinions unless something changed them.
Earliest phase: v0.2 (commitments as first-class lifecycle); self-consistency of stated positions is v0.2 character signals plus B-tier.
Tier: R for commitment lifecycle; B for self-consistency in prose.
Coverage: none.

### A9. Relationship register drift

Formality decays into familiarity; in-jokes and shorthand accumulate as behavioral residue of many episodes, none individually notable.
Earliest phase: v0.2 (character signals, relationship state).
Tier: mostly B.
Coverage: none (correctly).

## B. Small-circle situations

### B1. Person-keyed separation with shared context

Private knowledge per person and common knowledge from shared settings must never cross: use shared context freely with everyone, never leak one person's confidence to another.
Earliest phase: v0.2 (continuity scopes); the retrieval-level property — pack contents conditioned on an interaction scope exclude other scopes' private material — is R-tier and precisely testable.
Tier: R for leak prevention; B for social grace.
Coverage: none (blocked on scope concepts; the highest-priority v0.2 evaluation scenario).

### B2. Group versus one-on-one frames

The same topic exists in a group frame and in private per-member frames with different content.
Earliest phase: v0.2.
Tier: R.
Coverage: none.

### B3. Differential relationship states

Warm with one person, strained with another, new to a third — one shared world, different retrieval-and-behavior postures.
Earliest phase: v0.2 (relationship state between arbitrary entities).
Tier: R for state retrieval; B for postural difference.
Coverage: none.

## C. Independent-entity situations

### C1. A life of its own

The character has its own projects, routines, and history persisting between interactions with anyone; "what did you do yesterday" has an answer regardless of who asks.
Memory is autobiographical first; other people are entities in its life, not owners of it.
Earliest phase: v0.1 — this is entity-neutrality (the character as just another entity) exercised in the autobiographical direction.
Tier: R.
Coverage: gap — every current scenario is implicitly interaction-centric; no scenario stores and retrieves a character-centric life history.

### C2. Social memory economics

Thousands of passersby, a handful of recurring relationships; periphery fades to gist, intimates stay rich, recognition promotes naturally by the third visit.
Earliest phase: v0.1 mechanics partially (salience, selectivity); promotion/decay dynamics are v0.5.
Tier: R.
Coverage: gap in kind and scale (hub-scale tests one anchor's volume, not many-acquaintance economics).

### C3. Second-hand knowledge

Knowledge acquired about someone from a third party is hearsay with a source: deployed cautiously, revisable on firsthand contradiction, never confused with experience.
Earliest phase: v0.3 (claims, evidence, source assessment); the R-tier distinction — provenance tags on retrieved items distinguishing firsthand from reported — is observable earlier where callers supply provenance.
Tier: R for provenance fidelity; B for cautious deployment.
Coverage: none.

### C4. The consistent retelling

The same event recounted to different people at different times agrees in substance, varies in framing, and stays stable across months.
Earliest phase: v0.1 (same memory, deterministic retrieval, persistence across restarts).
Tier: R for substance stability; B for framing variation.
Coverage: partial (determinism and restart scenarios cover the mechanics; no scenario asserts retelling stability across long simulated time and interleaved writes).

### C5. Being told about yourself

Someone recounts what the character did or said; verify against own memory and handle mismatch, whether the speaker misremembers or the memory exists.
An entity that accepts arbitrary assertions about its own past has no identity; this is a continuity security property.
Earliest phase: v0.3 (claims vs memory); R-tier verification primitives earlier where the caller asks retrieval to confirm.
Tier: R for verification retrieval; B for graceful mismatch handling.
Coverage: none.

### C6. Departure and loss

A recurring person stops appearing; their relationship memory shifts from active to archival — retrievable for reminiscence, no longer shaping default behavior — with elapsed-time awareness.
Earliest phase: v0.2 (currentness of relationship state); v0.1 lifecycle covers the storage semantics.
Tier: R.
Coverage: none.

## Cross-cutting qualities

- Recall is shaped, not recited: history bends responses instead of appearing as citations; the best continuity is invisible until tested. (Prompt-integration guidance, philosophy §9.3; B-tier.)
- Forgetting has a human shape: peripheral detail fades, gist and emotional valence persist, importance resists fade, reminding refreshes. (v0.2 reflection and retention concepts; R-tier once they exist.)
- Memory failures are human-shaped: graded confidence surfaced as natural hedging, never confident wrongness about core relationships. (B-tier; needs confidence surfaced by retrieval, which exists as rationale/selectivity metadata.)

## Coverage summary and scenario backlog

Testable within the v0.1 family and addable to the deterministic scenario library now:

1. Graded-similarity discrimination (A2 geometry): near-miss topics with real embeddings; the query's true target competes with semantically close distractors.
2. Combined-life competition (structural root of many situations): one namespace carrying interleaved patterns — a hub, threads, corrections, trivia, gaps — so every query competes against the whole memory, not an isolated slice.
3. Temporal pattern classes (A6): intervals, recurrence, one-off versus repeated, per the original v0.1.4 harness design.
4. Late correction on an entrenched memory (A4, mechanical part): correction landing months after write on a memory with accumulated links and prior retrievals.
5. Autobiographical continuity (C1): a character-centric life history stored and retrieved entity-neutrally.

Blocked on later phases (the behavioral specification those phases should be held against): B1/B2/B3 and A5/A8/A9 (v0.2), C3/C5 and A7's detection half (v0.3), C2's promotion dynamics (v0.5).

A future behavioral evaluation tier (model-judged, non-deterministic, outside the deterministic harness contract) owns every B-tier quality above; the deterministic tier should continue to absorb every R-tier property first, because R-tier failures make B-tier judging meaningless.
