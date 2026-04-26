# v0.3 Design Draft: Factual Rigor and Belief Tracking

## Version intent

v0.3 adds stronger factual memory without replacing the episode-backed continuity model.

It answers:

```text
Who said this?
What exactly was claimed?
Does the assistant currently believe it?
Was it contradicted or superseded?
Is it still temporally valid?
```

This is a supporting subsystem, not the root of Character Memory.

---

# 1. Why this comes after v0.1/v0.2

For Character Memory, the first priority is continuity of experience and relationship. But a long-running assistant also needs to handle:

```text
changing facts
wrong information
contradictions
source credibility
user corrections
stale knowledge
```

v0.3 adds this rigor when the starter system is already useful.

---

# 2. New concepts

## 2.1 Assertion

A source's statement extracted from an observation.

```json
{
  "id": "assert_...",
  "object_type": "assertion",
  "asserted_by_entity_id": "ent_user_primary",
  "asserted_claim_id": "claim_...",
  "polarity": "affirms",
  "quote": "I prefer natural-language embedding surfaces over structured templates.",
  "derived_from_observation_id": "obs_...",
  "extraction_confidence": 0.94
}
```

## 2.2 Claim

A proposition under consideration.

```json
{
  "id": "claim_embedding_surfaces",
  "object_type": "claim",
  "canonical_text": "Natural-language embedding surfaces are preferable to structured metadata templates for this memory system.",
  "about_entity_ids": ["ent_project_character_memory"],
  "about_concept_ids": ["ent_concept_vector_retrieval"],
  "valid_from": "2026-04-26",
  "valid_until": null,
  "volatility": "medium"
}
```

## 2.3 EvidenceLink

How an assertion, observation, or episode relates to a claim.

```json
{
  "id": "evidence_...",
  "object_type": "evidence_link",
  "evidence_for_claim_id": "claim_embedding_surfaces",
  "evidence_item_id": "assert_...",
  "role": "supports",
  "strength": 0.78,
  "rationale": "The user explicitly argued that structured templates may skew embedding vectors."
}
```

## 2.4 BeliefAssessment

The assistant's time-stamped stance toward a claim.

```json
{
  "id": "belief_...",
  "object_type": "belief_assessment",
  "assesses_claim_id": "claim_embedding_surfaces",
  "stance": "accepted",
  "confidence": 0.84,
  "assessed_at": "2026-04-26T11:00:00+09:00",
  "based_on_evidence_ids": ["evidence_..."],
  "supersedes_belief_id": null,
  "is_current": true
}
```

## 2.5 SourceAssessment

Domain-scoped and time-scoped source reliability.

```json
{
  "id": "srcassess_...",
  "object_type": "source_assessment",
  "source_entity_id": "ent_person_bob",
  "domain_entity_id": "ent_concept_software_security",
  "reliability_score": 0.42,
  "valid_from": "2026-04-25",
  "valid_until": null,
  "based_on_evidence_ids": ["evidence_..."],
  "rationale": "Bob's prior security claim was contradicted by an official advisory."
}
```

---

# 3. Temporal validity

A single timestamp is not enough.

v0.3 should support:

```text
asserted_at
observed_at
ingested_at
valid_from
valid_until
assessed_at
review_after
volatility
```

Volatility categories:

```text
stable
medium
high
ephemeral
```

Rules:

```text
High-volatility claims should get review_after dates.
Expired or overdue beliefs should be downranked or excluded from current-belief view.
Historical assertions remain even when beliefs change.
```

---

# 4. Current beliefs view

Create a derived view:

```text
current-beliefs
```

A belief appears only if:

```text
latest non-superseded assessment
stance is accepted or tentatively_accepted
confidence above configured threshold
not past valid_until
not review-overdue when freshness is required
retention_state active
```

Do not directly assert factual triples into the raw memory graph unless they are in the derived current-beliefs view and traceable back to belief assessments.

---

# 5. Integration with v0.1 DerivedMemory

v0.1 uses `DerivedMemory(derived_type="claim")` as a lightweight placeholder.

Migration path:

```text
DerivedMemory claim → Claim
DerivedMemory provenance → Assertion/EvidenceLink, where extractable
DerivedMemory currentness → BeliefAssessment
```

Do not force every reflection or character signal into claim form.

---

# 6. Public API additions

```rust
fn extract_claims(&self, episode_id: &str) -> Result<Vec<Claim>, MemoryError>;
fn assess_claim(&self, claim_id: &str) -> Result<BeliefAssessment, MemoryError>;
fn get_current_beliefs(&self, scope: Option<&MemoryScope>) -> Result<Vec<BeliefAssessment>, MemoryError>;
fn get_evidence(&self, claim_id: &str) -> Result<Vec<EvidenceLink>, MemoryError>;
fn review_stale_beliefs(&self, scope: Option<&MemoryScope>) -> Result<Vec<BeliefReview>, MemoryError>;
fn record_contradiction(&self, claim_a: &str, claim_b: &str, evidence: Option<&EvidenceInput>) -> Result<ContradictionRecord, MemoryError>;
```

---

# 7. Acceptance criteria

```text
A claim can be traced to assertion → observation → episode.
Contradictory evidence can supersede a belief assessment.
Current-belief view excludes rejected/superseded/stale beliefs.
Source reliability is domain-scoped, not global.
Temporal validity affects retrieval ranking.
Factual memory does not overwrite raw episodes.
```

---

# 8. Relation to old roadmap

The old roadmap's `semantic` memory type is replaced by a richer factual subsystem.

Old:

```text
semantic memory = content + id
```

Revised:

```text
source assertion
claim under consideration
evidence relationship
current or historical belief assessment
```

This better supports source verification, credibility updates, and changing facts.
