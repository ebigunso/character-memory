# v0.3 Design Draft: Factual Rigor, Temporal Validity, and Entity Evolution

## Version intent

v0.3 adds stronger factual memory without replacing the episode-backed continuity model. It answers:

```text
Who said this?
What exactly was claimed?
Does the system currently accept it?
Was it contradicted or superseded?
Is it still temporally valid?
Did the entity's name, role, location, identity, or relationship change over time?
```

This is a supporting subsystem, not the root of Character Memory.

---

# 1. Why this comes after v0.1/v0.2

For Character Memory, the first priority is continuity of experience and relationship. But a long-running memory system also needs to handle:

```text
changing facts
wrong information
contradictions
source credibility
corrections
stale knowledge
entity drift
historical aliases
historical roles and relationships
```

v0.3 adds this rigor when the starter system is already useful and scoped continuity exists.

---

# 2. New concepts

## 2.1 Assertion

A source's statement extracted from an observation.

```json
{
  "id": "assert_...",
  "object_type": "assertion",
  "asserted_by_entity_id": "ent_person_or_source",
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

How an assertion, observation, episode, or other evidence item relates to a claim.

```json
{
  "id": "evidence_...",
  "object_type": "evidence_link",
  "evidence_for_claim_id": "claim_embedding_surfaces",
  "evidence_item_id": "assert_...",
  "role": "supports",
  "strength": 0.78,
  "rationale": "The source explicitly argued that structured templates may skew embedding vectors."
}
```

## 2.4 BeliefAssessment

The system's time-stamped stance toward a claim.

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
  "rationale": "A prior security claim was contradicted by a stronger source."
}
```

## 2.6 EntityStateHistory

A temporally scoped representation of entity state.

Examples:

```text
entity had one name during interval A and another name during interval B
entity had one role during interval A and another role during interval B
entity was in one location during interval A and another later
relationship between entities changed over time
alias was valid historically but should not be treated as current
```

This may be implemented through claims at first rather than a separate class if that keeps the model simpler.

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
Old claims about an entity are historical evidence, not automatically current truth.
```

---

# 4. Entity evolution

Long-lived memory creates entity drift:

```text
entities may rename
entities may split or merge
roles may change
locations may change
relationships may change
aliases may be historically scoped
old claims may be historical but not current
```

The factual-rigor layer should represent temporally qualified entity state without destructive overwrite.

Acceptance implications:

```text
The system can represent that an entity had one role/name/location/relationship during one interval and another later.
TemporalValidity applies to arbitrary claims about arbitrary entities.
Contradictions about entity identity or state can be represented without destructive overwrite.
Corrections and supersession remain provenance-preserving.
```

Do not try to solve full entity resolution in v0.1.2. v0.1.2 only needs selectivity and fanout safety.

---

# 5. Current beliefs view

Create a derived view:

```text
current-beliefs(scope)
```

A belief appears only if:

```text
latest non-superseded assessment
stance is accepted or tentatively_accepted
confidence above configured threshold
not past valid_until
not review-overdue when freshness is required
retention_state active
within requested ContinuityScope or belief scope
```

Do not directly assert factual triples into the raw memory graph unless they are in the derived current-beliefs view and traceable back to belief assessments.

---

# 6. Integration with v0.1 DerivedMemory

v0.1 uses `DerivedMemory(derived_type="claim")` as a lightweight placeholder.

Migration path:

```text
DerivedMemory claim → Claim
DerivedMemory provenance → Assertion/EvidenceLink, where extractable
DerivedMemory currentness → BeliefAssessment
```

Do not force every reflection or character signal into claim form.

---

# 7. Public API additions

Illustrative shape:

```rust
fn extract_claims(&self, episode_id: &str) -> Result<Vec<Claim>, MemoryError>;

fn assess_claim(&self, claim_id: &str) -> Result<BeliefAssessment, MemoryError>;

fn get_current_beliefs(
    &self,
    scope: Option<&ContinuityScope>,
) -> Result<Vec<BeliefAssessment>, MemoryError>;

fn get_evidence(&self, claim_id: &str) -> Result<Vec<EvidenceLink>, MemoryError>;

fn review_stale_beliefs(
    &self,
    scope: Option<&ContinuityScope>,
) -> Result<ReviewResult, MemoryError>;

fn record_contradiction(
    &self,
    claim_a: &str,
    claim_b: &str,
    evidence: Option<&EvidenceInput>,
) -> Result<(), MemoryError>;
```

---

# 8. Acceptance criteria

```text
A claim can be traced to assertion → observation → episode.
Contradictory evidence can supersede a belief assessment.
Current-belief view excludes rejected/superseded/stale beliefs.
Source reliability is domain-scoped, not global.
Temporal validity affects retrieval ranking.
TemporalValidity applies to arbitrary claims about arbitrary entities.
CurrentBeliefView is scope-based.
The system can represent that an entity had one role/name/location/relationship during one interval and another later.
Contradictions about entity identity or state can be represented without destructive overwrite.
Factual memory does not overwrite raw episodes.
Corrections and supersession remain provenance-preserving.
```

---

# 9. Factual subsystem shape

```text
source assertion
claim under consideration
evidence relationship
current or historical belief assessment
temporal validity
entity state over time
```

This better supports source verification, credibility updates, changing facts, and long-lived entity continuity.
