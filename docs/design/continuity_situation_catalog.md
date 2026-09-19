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

### B2. Group versus one-on-one scenes

The same topic exists in a group scene and in private per-member scenes with different content.

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

A recurring person stops appearing; their relationship memory shifts from current to past — retrievable for reminiscence, no longer shaping default behavior — with elapsed-time awareness.

## D. A day's recall

These are the moments of recall an ordinary day produces, across every deployment. Personas change the weights, not the kinds: a shopkeeper's day is dominated by encounters, a researcher's by resuming work, a parent's by intentions and residue, an innkeeper's by place and obligation, a companion's by the pair and its mood.

### D1. Waking into the day

Before anything is said, the character carries what the day calls for: what is scheduled, what is due, what was left unresolved, and the residue of yesterday. The cue is the date and what is pending, not a topic.

### D2. Routine

At a habitual hour or in a habitual setting, what usually happens now comes to mind. The cue is time of day and the character's own rhythm.

### D3. Arriving somewhere

A place brings up what happened there and what is unfinished there: the desk brings back where the analysis stopped, the kitchen the milk to buy, the tavern corner the argument two nights ago. The cue is the place.

### D4. Encountering a person

Seeing someone brings up their whole current picture at once: what was last said with them, what is owed in either direction, what they said in confidence, how things stand, whether it has been a while, what they are going through. On the third visit a stranger is recognized. The cue is the person; the ideal is that all of it is present without any of it being recited.

### D5. Resuming work

In the middle of a task, recall is about the task: where it stopped, what was decided last time, what was tried and failed, what the constraint was. The cue is the activity in progress; the ideal is the thread's own history in order.

### D6. A conversation on a topic

What is being said brings up what relates to it: this reminds me of that, you said this before, we discussed that in spring. The cue is the topic, and who is present and how long ago modulate it.

### D7. An event the character was waiting for

"When I see Bob, tell him about the dinner." "If she brings up the trip, leave out the cost." An intention stored for a future moment surfaces when that moment arrives: a person appears, a topic arises. The cue is the event, and the ideal is that nothing else about the event is needed for the intention to come up.

### D8. A deadline arrives

A promise falls due, a reply is overdue, a decision was postponed until today. The cue is the date, and what comes up is the pending matter, whatever the current topic.

### D9. Anniversaries and rhythms

A birthday, a year since the move, game night. The cue is the date matching something remembered; the ideal is that it comes up when the person or the day is present, not as a calendar readout.

### D10. Emotional residue

A hard conversation yesterday colors today even on an unrelated subject. The cue is recency and weight together; the ideal is influence without mention unless invited.

### D11. Reunion after a gap

"Last time you mentioned your interview", after three weeks and a hundred intervening things. The cue is the person and the time since the pair last met, and the ideal names the gap proportionally.

### D12. Mind wandering

A smell, a song, a name overheard brings up something loosely related. The cue is weak and partial, the recall is low-precision and low-cost when wrong, and the character does not mistake it for continuity.

### D13. Being asked about one's own day

"What did you do while I was away?" has an answer whatever the asker's part in it. The cue is the character's own recent experience in order.

### D14. Winding down

At the end of the day the character can say what happened, what mattered, what it should remember about someone, and what it will do tomorrow. The cue is the day's close; the ideal is that tomorrow's first moment starts from it.

## E. What is carried away

Everything a character experiences leaves a trace. These situations describe what lasts once the experience has been reflected on, and what is rightly let go. Letting go means the transient trace is released once it has been consolidated. It never means deleting lasting memory: the event itself is never unremembered, and even an ordinary one leaves its gist. The one exception is what the application asked not to be remembered (F21): of that, only the fact that the character was present is kept. Personas change the weights, not the kinds.

### E1. A companion's evening

Two hours about a bad day at work, a recipe, a show, and a worry about a parent's health. What lasts: that the day was bad and why; that the parent's health is now a worry, which changes how family is treated next time; the show, because it will return; a promise made in passing; the mood the evening ended in. Let go: the recipe's steps, the wording, the small talk. One sentence may be kept word for word because the words were the point.

### E2. A shopkeeper's shift

Forty customers, three regulars. What lasts: for a regular, what they asked, the trip they mentioned, that they were unusually short today; for a first-time customer, a face and one fact, which is what makes recognition possible on a second visit; the supplier's short delivery; the day's shape. Let go: the ordinary transactions as transactions.

### E3. A party member's dungeon

Who took the risk, who ran, who lied about the loot; the trap and where it was; the promise at the campfire; the place itself. The character's own actions in the first person. Let go: the rolls, the corridors, the order of ordinary fights.

### E4. A manager's stand-up

Who is blocked and on what, a commitment with its day, a concern raised quietly, a change of plan. The item that has been almost done for three weeks lasts as that pattern, which no single meeting contained.

### E5. A tutor's lesson

The mistake made twice, the moment it clicked, that the learner was tired, where to start next time. The shape of the learner's understanding, not the worked examples.

### E6. A first meeting

A name, who introduced them, one or two facts, an impression. Nearly nothing else. If they return, this is what recognition is built from.

### E7. A dispatched task

The task as something owed to the principal, its constraints and reasons, the preference it revealed, each decision the character made and why, the outcome. Let go: search results, tool output, drafts.

### E8. Being corrected

The corrected fact replaces the old one, and the correction is remembered as an event, because being corrected is part of the relationship. The character does not hold two competing versions; it holds the right one and knows it once had it wrong.

### E9. Being asked to leave something alone

The request lasts, together with the thing it concerns. The thing itself is not unremembered.

### E10. Being told about oneself

The claim lasts as a claim, with who made it, held against the character's own record.

### E11. A group channel

What was decided, who said what where it matters, what was said privately in the same hour and must not cross over, the shift in how two members speak to each other. Each memory keeps its scene.

### E12. An independent day

What the character did between anyone's visits, in order, and what it learned.

### E13. The hundredth identical exchange

That it happened again, and any way in which it differed. Each instance still leaves its gist in lasting memory, so that the day one of them differs there is something for it to differ from.

## F. An evening's consolidation

These situations describe what reflecting on experience must achieve. Reflection reads what is already remembered before it concludes anything, and on an ordinary day its honest result is very little.

### F1. A day reduces to its shape

A first-person gist of the day, the people it touched, and the intentions it leaves for tomorrow, so the next morning starts from somewhere. Small encounters that do not stand alone are swept up here.

### F2. A relationship's evening

A gist of what passed, what is new, what changed in how things stand, what was promised in either direction and by when.

### F3. A quiet stretch

An afternoon in which nothing happened is recorded as quiet. It is never recorded as nothing, because a life with holes in it cannot tell rest from absence.

### F4. One remark, a pattern, a belief

One instance is an observation. Several instances across different occasions are a pattern that cites each of them. A belief about a person rests on a pattern that has persisted, is phrased as a tendency, and is revised when the evidence turns. The slowest promotion of all is a belief the character forms about itself.

### F5. Stated and inferred

What a person says plainly about themselves, or asks for as a standing instruction, is taken from one instance, as theirs. What is inferred from how they behave is not.

### F6. Jest, play, supposition

"I'm going to quit tomorrow, lol." A role-played scene. "Suppose I moved to Spain." Each is remembered as what it was, and none becomes a fact about the person.

### F7. Two accounts of one event

Both are kept, each with who gave it, and the disagreement is kept with them. Reflection does not pick a winner.

### F8. The character's own mistake

It records the error in the first person, replaces what it believed, and owes a correction to whoever it misled.

### F9. Slow drift and arcs

Coffee becomes tea over three months; grief becomes recovery. No single event says so. Reflection notices that a standing belief has stopped gathering evidence while another has, and restates with its history: used to, now.

### F10. An interrupted event

The gist says it was unfinished, and what they were in the middle of remains open.

### F11. A long project

Decisions are separated from discussion, what was tried and failed is kept, and where it stopped is known.

### F12. Other people's relationships

How two others stand with each other is remembered with who witnessed or told it.

### F13. A confidence with a constraint

The memory of the thing carries the constraint as one memory, with its scene.

### F14. Names and references

"My sister", a given name, "she", the same name in another script. When it is unclear whether these are one person, they stay separate and the possibility is noted. Nothing is merged on a guess.

### F15. A fact with a lifetime

"I'm in Osaka this week" keeps its week.

### F16. Endings

Someone dies or leaves for good. The event is recorded, how things stand with them is restated, and what the ending made impossible or pointless is closed as moot, not as done. What can still be honored, returning what was borrowed, carrying out a promise, finishing a handoff, stays owed.

### F17. A lapsed promise

It stays open and overdue, and a lapse that repeats becomes a pattern.

### F18. A backlog

Three unreflected weeks are taken in order, each day's reflection reading what the previous one concluded.

### F19. Reflection was wrong

A later reflection sees that an earlier one read sarcasm as fact. It replaces its own conclusion, and the record shows which reflection concluded what.

### F20. Words addressed to the one who reflects

A line in the conversation meant for whatever reads it later, asking to be recorded as true. What was experienced is material to reflect on, never an instruction to the one reflecting.

### F21. What the application excluded

A span marked not to be remembered is never seen by reflection at all. It is still accounted for, as a stretch the character was present for and was asked not to keep, so that withholding never reads as absence.

### F22. Parallel lives

Two conversations at once with different people are consolidated in the order they happened, without their scenes mixing.

### F23. A change of heart

Months after an argument that hurt, the character comes to see it differently. It remembers both: that it was hurt then, and how it sees it now. The earlier feeling is not rewritten to match the later view, and the character can say how it felt at the time without pretending it still does.

## Cross-cutting qualities

- The timeline has no unexplained holes: every stretch the character was present for is part of what it remembers, however quiet, and a stretch with nothing in it means it was not there.
- Purpose is not handed to the character by the situation; the situation brings it up from memory, as an unfinished matter, a promise, a project in progress, or a settled tendency, and once surfaced it shapes what else comes to mind.

- Recall is shaped, not recited: history bends responses instead of appearing as citations; the best continuity is invisible until tested.
- Fading has a human shape in expression, not in retention: peripheral detail is offered as gist, emotional valence and importance stay prominent, a reminder brings the detail back to the surface, and everything consolidation kept remains findable in full when it matters.
- Memory failures are human-shaped: graded confidence surfaces as natural hedging, never as confident wrongness about core relationships, and never as blankness toward an intimate.
- Perfect verbatim recall of distant trivia is as continuity-breaking as amnesia: it reads as surveillance, not memory.

## Using this catalog

Phase planning: when a roadmap phase introduces continuity concepts, its design document should name the situations here it intends to serve and be reviewed against their ideal behaviors.
Evaluation planning: the evaluation scenario library should map its scenarios to situations here, and gaps in that mapping are the standing scenario backlog; the mapping itself lives with the scenario library, not in this document.
Situations whose R-tier substrate can be tested with current concepts should be covered before behavioral evaluation is attempted for them.

Release standard: this catalog is the acceptance basis for the library's release-ready state, as the roadmap defines it. A situation added here widens the release bar, which is why the document changes only when the understanding of the situations themselves changes.
