# v0.4 Design Draft: What the Character Holds About the World Over Time

## Status of this draft

Rewritten on 2026-10-16 under the v0.2 completion brief, which corrected this phase where it contradicted the product philosophy. The file keeps its earlier name because records cite it. The roadmap's section 15 is the summary; this draft is the design, kept to the altitude the owner set: future shapes are fleshed out only to the point where findings from prior phase measurements would not pose a significant uncertainty. The behavioral standard it serves is the continuity situation catalog's section C, people and relationships. Decisions cited by record number are in `docs/decisions/`.

## Version intent

v0.3 lets the character form memory from experience. This phase lets what it holds about the world carry time: a belief has a span in which it held; a person or place has had other names and roles; the character knows who else knows a thing; and it holds its own belief about how far a source can be relied on. Everything here is the character's own belief built from experience, held as ordinary interpreted memory with its evidence (philosophy 8.2, ADR-D-0028, ADR-D-0035); none of it is a verdict on what is true, because memory is subjective and the library passes no verdict on truth.

The first-class fact-acceptance objects the earlier draft described (Assertion, Claim, EvidenceLink, BeliefAssessment, SourceAssessment, with stance and confidence) are no longer on the roadmap; section 6 says why.

---

# 1. Time counted in experience, if it is not v0.3's

v0.3's planning decides from a cost reading over the trace store whether time counted in experience is cheap enough to build there. If it is not, it is the first named item here: how long ago a thing seems, and how far an old feeling still presses, follow what the character has lived since, not the calendar (philosophy 7.5 and 8.3). Nothing stored decays, and nothing is worn down except by what came after it (ADR-D-0018). Until it is built, time stays the clock.

---

# 2. A belief has a span

A belief the character holds about the world can carry the span in which it held: from when, and until when, recorded when the belief was written or restated. The span is a fact about the belief recorded by a write with provenance, so expiry is a known event and never a decay. A belief past its span leaves current views under the currency invariant (roadmap 2.15) with that reason and stays recallable; a belief with no span is current until superseded, and its staleness is reported as age, never enforced.

Recall at a reference time reads the belief current at that time, so what the character held about a matter last spring is reachable as what it held then.

---

# 3. Aliases and roles over time

A notion has been called other things and has held other roles and relationships. These are beliefs about the notion with spans (ADR-D-0034, ADR-D-0035): that it was called one name during one interval and another later, that a person held one role and then another, that a relationship changed. An earlier name still cues the notion, as history; recall reads the one current at the reference time. Nothing is overwritten and nothing merged, split or deleted: a change is a new belief, and the old one stays as history (ADR-D-0018, ADR-D-0021).

---

# 4. Who else knows

The character knows who else knows a thing, from the scenes it holds: who was present when something was said, and who it was later told to. This is built from the scene every memory carries (ADR-D-0038) and from the character's own sayings, which v0.3 keeps as their own kind of evidence. Recall reports it with the memory and passes no verdict on who may hear what; that judgement is made where the uncertainty can be weighed, in the application (philosophy 8.5).

---

# 5. Source reliability as the character's own belief about a source

How far a source can be relied on is a belief about that source, built from the character's experience of it: what it said that was later borne out, what was later contradicted. It concerns the matters the experience concerned, since a friend who is right about software may be wrong about people; it is never a global score and never a number (ADR-D-0030). It is held like any belief, with the experiences that formed it reachable, and it changes only by supersession. It shapes how a told thing is held and said; it decides nothing about whether the told thing is true.

---

# 6. Not in v0.4, and no longer on the roadmap

```text
the first-class fact-acceptance objects, Assertion, Claim, EvidenceLink, BeliefAssessment, SourceAssessment, with stance and confidence, and the belief ontology and contradiction machinery around them: removed from the roadmap rather than kept conditional, because the question they answer, whether a claim is true and whether the system accepts it, is not this library's; the character's experience builds toward believing something, the record says which experiences, and the library passes no verdict on truth (philosophy 8.2); ADR-D-0005 deferred these objects, and the outcome of its revisit is proposed as ADR-D-0040
volatility categories and review-after dates: a belief overdue for review that leaves current views would be forgetting by the clock, which ADR-D-0018 rejects; a span recorded at write time is the whole of validity here
a current-beliefs view with stance and confidence thresholds: currency is read from the supersession chain and the recorded span, and interpreted memory carries no confidence score (ADR-D-0030)
```

The assertions of ADR-D-0035, the mechanical commitments an ordinary belief carries from a closed predicate vocabulary, are not these objects and stay; a span, a name, a role are the kind of thing such an assertion carries.

Attribution's core, whether the character witnessed a thing or was told it, is v0.3's (ADR-D-0028 and the v0.3 draft); this phase builds on it and adds nothing to it.

---

# 7. Acceptance criteria

```text
A belief can carry the span in which it held, and one past its span is omitted from current views with that reason and remains recallable.
Recall at a reference time reads the belief current at that time.
The character can hold that a notion had one name, role, or relationship during one interval and another later; an earlier name still cues the notion as history, and nothing is overwritten.
Asked who else knows a thing, the character answers from the scenes it holds and its own sayings, and the answer names what it rests on.
The character's belief about a source is held with the experiences that formed it, concerns stated matters, carries no number, and changes only by supersession.
Corrections and supersession remain provenance-preserving.
```

---

# 8. Questions for planning

```text
the shape of a span on a belief, and whether it is an assertion of ADR-D-0035's vocabulary
how recall at a past reference time selects among a notion's beliefs
how who-else-knows is read from scenes and sayings without a stored register of who knows what
the measurement, in the public companion `CharacterMemoryEvals` evaluation repository (a development aid, not core library functionality), of the catalog's section C situations this phase serves
```
