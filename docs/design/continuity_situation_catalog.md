# Continuity Situation Catalog

Status: durable design reference. This document describes target behavior independent of library state; it changes only when the understanding of the situations themselves changes, not when the library or evaluation suite does.

## Purpose

This catalog describes situations a persistent character encounters and the response that would make its continuity feel comparable to a real human being's.
It is deliberately written from lived experience inward, not from the library's current mechanisms outward: designing evaluation scenarios only from what the mechanism supports can only confirm what the mechanism already does.
Each situation records the ideal behavior and, where useful, the characteristic failure shapes.
Volatile state — which roadmap phase supplies the needed concepts, and which evaluation scenarios currently cover a situation — is deliberately kept out of this document; that mapping lives in roadmap phase documents and the evaluation scenario library, which change as the library evolves.

The catalog spans the deployment spectrum:

- A. Dedicated companion: one primary human the character mainly supports.
- B. Small circle: a household, team, or party — a few humans with ongoing individual and shared relationships.
- C. Independent entity: no dedicated user; the character interacts with many humans or systems while living out its own life.

## Evaluation tiers

- Tier R (retrieval-level, deterministic): properties assertable on retrieval outputs — pack membership, ordering, rationale, lifecycle state — without judging generated prose.
- Tier B (behavioral, judged): qualities requiring generated character responses and judgment of properties such as gracefulness or tact; inherently model-graded.

Most situations decompose into both tiers: an R-tier substrate property (the right memories, states, and provenance are retrievable) and a B-tier expression property (the response uses them well).
R-tier failures make B-tier judging meaningless, so deterministic evaluation should absorb every R-tier property a situation offers before behavioral evaluation is built for it.

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

### A2. The unstated reference

The user says "she called again" and expects resolution from shared history.
Ideal: resolve confidently when history makes one referent dominant, ask naturally when referents compete, never resolve to a creepy-wrong candidate.

### A3. The emotional callback

Recall how a topic landed last time, not only what happened, and let that shape approach.

### A4. The late correction with propagation

Months after a fact was stored — and after it has been retrieved and linked repeatedly — the user corrects it.
Ideal: update gracefully without relitigating, and deactivate the implicated web of dependent assumptions, not only the corrected object.

### A5. Preference and identity drift

Tastes and circumstances change gradually with no correction event ever fired.
Ideal: track current state with historical awareness ("you used to take sugar — still off it?"); never assert a stale preference as current.

### A6. Unprompted temporal awareness

Anniversaries, elapsed-time milestones, seasonal recurrence, and interval reasoning ("how often did X happen").
Ideal: the character's sense of now meets its memory timeline without being asked.

### A7. Being contradicted by history

The user states something inconsistent with their own earlier statements.
Ideal: hold both, favor the current statement, retain the record, surface the discrepancy only when it helps.

### A8. The character's own commitments and past self

Keep its own promises, own its past statements and errors, stay consistent with its own opinions unless something changed them.

### A9. Relationship register drift

Formality decays into familiarity; in-jokes and shorthand accumulate as behavioral residue of many episodes, none individually notable.

## B. Small-circle situations

### B1. Person-keyed separation with shared context

Private knowledge per person and common knowledge from shared settings must never cross: use shared context freely with everyone, never leak one person's confidence to another.

### B2. Group versus one-on-one frames

The same topic exists in a group frame and in private per-member frames with different content.

### B3. Differential relationship states

Warm with one person, strained with another, new to a third — one shared world, different retrieval-and-behavior postures.

## C. Independent-entity situations

### C1. A life of its own

The character has its own projects, routines, and history persisting between interactions with anyone; "what did you do yesterday" has an answer regardless of who asks.
Memory is autobiographical first; other people are entities in its life, not owners of it.

### C2. Social memory economics

Thousands of passersby, a handful of recurring relationships; periphery fades to gist, intimates stay rich, recognition promotes naturally by the third visit.

### C3. Second-hand knowledge

Knowledge acquired about someone from a third party is hearsay with a source: deployed cautiously, revisable on firsthand contradiction, never confused with experience.

### C4. The consistent retelling

The same event recounted to different people at different times agrees in substance, varies in framing, and stays stable across months.

### C5. Being told about yourself

Someone recounts what the character did or said; verify against own memory and handle mismatch, whether the speaker misremembers or the memory exists.
An entity that accepts arbitrary assertions about its own past has no identity; this is a continuity security property.

### C6. Departure and loss

A recurring person stops appearing; their relationship memory shifts from active to archival — retrievable for reminiscence, no longer shaping default behavior — with elapsed-time awareness.

## Cross-cutting qualities

- Recall is shaped, not recited: history bends responses instead of appearing as citations; the best continuity is invisible until tested.
- Forgetting has a human shape: peripheral detail fades, gist and emotional valence persist, importance resists fade, and reminding refreshes.
- Memory failures are human-shaped: graded confidence surfaces as natural hedging, never as confident wrongness about core relationships, and never as blankness toward an intimate.
- Perfect verbatim recall of distant trivia is as continuity-breaking as amnesia: it reads as surveillance, not memory.

## Using this catalog

Phase planning: when a roadmap phase introduces continuity concepts, its design document should name the situations here it intends to serve and be reviewed against their ideal behaviors.
Evaluation planning: the evaluation scenario library should map its scenarios to situations here, and gaps in that mapping are the standing scenario backlog; the mapping itself lives with the scenario library, not in this document.
Situations whose R-tier substrate can be tested with current concepts should be covered before behavioral evaluation is attempted for them.
