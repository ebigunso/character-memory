# Character Memory

## Product Philosophy

**Memory as the substrate of persistent AI character**

### One sentence thesis

Character Memory is not a generic memory store. It is an episodic memory substrate that lets an LLM-based assistant, companion, simulation character, or other persistent AI interface preserve continuity, form stable character through remembered experience, and behave less like a new instance every session.

---

## 1. Executive Summary

Character Memory is a library for giving LLM-based assistants and companions long-term episodic memory. Its purpose is not just to store facts or retrieve similar past messages. Its purpose is to support **character continuity**: the sense that the assistant or persistent AI character is the same continuing entity across interactions.

The central idea is that memory and character should reinforce each other. What an assistant remembers influences how it behaves. How it behaves influences what becomes salient and worth remembering. Over time, this feedback loop produces a more stable and recognizable character.

This library should therefore be designed as a memory layer for persistent AI systems, not as a task-agent framework, a vector database wrapper, or a prompt-based persona manager.

---

## 2. Core Philosophy

### Core distinction

**A persona is assigned. A character is accumulated.**

A prompt can assign a persona, but it cannot by itself create durable character. Character emerges from continuity: repeated interactions, remembered events, reinforced preferences, relationship history, unresolved tensions, fulfilled promises, mistakes, corrections, and reflections over time.

Character Memory exists to provide the memory substrate for that continuity. It should help an AI system remember past experience in ways that influence future interpretation and behavior.

### 2.1 Memory should be lived, not merely logged

The system should treat memories as episodes from a continuing relationship, character arc, project, or interaction history, not as inert records. A chat log records that something happened. Character Memory should help the assistant remember why it matters, who or what was involved, what it relates to, and how it should affect future interactions.

### 2.2 Character continuity is the product goal

The implementation should optimize for continuity of behavior, not just recall accuracy. A good system does not merely surface old facts. It helps the assistant or character act as though past interactions have shaped it.

### 2.3 Entity-neutrality is part of the core design

Character Memory should not assume that memory is always centered on a single user-assistant relationship. The core library should be usable for personal assistants, companions, game/simulation characters, research systems, and developer tools for temporal, entity-based, and relational retrieval.

Therefore the schema should treat entities equally. A user, assistant, player, protagonist, NPC, place, project, object, faction, organization, or topic can all be continuity anchors. Application-specific layers may interpret domain roles, but the core memory system should not hard-code them.

Memory is first-person. The remembering character is itself an entity in its own graph. Its actions are episodes it participated in, its promises are its commitments, and its history persists between interactions with anyone. What the character did yesterday has an answer regardless of who asks. Other people are entities in the character's life, not owners of its memory.

### 2.4 Human-like means the behaviour, not the mechanism

The standard for recall is what a person would have in mind in that moment, and how a person would then behave. It is not a model of how a brain stores or retrieves. Where this library departs from human memory on purpose, the departure is stated and kept as an advantage. The departures are few and deliberate: recall is complete, where a person's thins; how something felt is kept as it was recorded, where a person's remembered feeling drifts toward the present; no feeling fades faster for being unpleasant; nothing is forgotten to keep the record tidy, and recalling one thing never pushes its neighbours out of reach; and the character is never more sure of a detail than its record allows, where a person's confidence outlives the accuracy. Each is kept because an honest record is worth more than the human failure it replaces, and each is stated where it applies.

---

## 3. Problem Being Solved

Modern LLM agents are usually optimized for task execution. They can plan, call tools, and complete workflows, but they often lack continuity of experience. This makes them useful but psychologically shallow: each session can feel like a new instance with access to notes.

Common memory approaches are insufficient for persistent companions and assistants:

- **Flat chat history** preserves text but not meaning, salience, or relation structure.
- **Simple vector search** retrieves semantic similarity but often misses temporal order, recurring entities, causality, and relationship history.
- **User profile stores** capture facts about the user but not shared episodes or the assistant's evolving relationship to them.
- **Prompted personas** define surface behavior but do not accumulate history.
- **Task-agent memory** helps complete workflows but does not necessarily support persistent character.

Character Memory should solve the gap between useful task execution and persistent character continuity.

---

## 4. Intended Use Cases

The primary users are builders of AI systems where past interaction should shape future behavior. Examples include:

- LLM-based personal assistants that remember long-running projects, preferences, and relationship history.
- AI companions that should feel continuous across days, months, or years.
- Persistent game or simulation characters whose behavior should be shaped by prior encounters.
- Research systems studying memory, reflection, and long-term behavioral continuity in LLM applications.
- Developer tools for experimenting with temporal, entity-based, and relational retrieval.

The library should still be usable in ordinary assistant products, but its philosophy is strongest when the assistant or character is expected to be more than a stateless task executor.

---

## 5. System Mental Model

Character Memory should sit around the LLM interaction loop. It participates both before generation and after interaction.

1. Observe the current interaction.
2. Recall what the present moment calls for, from the situation itself and from what is being said.
3. Provide memory context to the LLM in a concise, grounded form.
4. Generate the assistant response.
5. Keep a trace of what happened, as it happened.
6. Know at once how things now stand, before there has been time to reflect.
7. Reflect afterward to decide what lasts: what the experience meant, what changed, what repeats, and what to carry forward.
8. Use what lasts to reinforce character continuity over future interactions.

**Key implication:** memory retrieval should not be treated as a final answer. It is context for situated behavior.

---

## 6. Core Concepts

| Concept | What it means for the character |
|---|---|
| Episode | A remembered event or interaction, kept with enough of what happened, when, with whom and from where that it can later be corrected, inspected and reflected on. |
| Trace | What happened, kept as it happened, before it has been reflected on. Recent experience is held literally and briefly; consolidation turns it into lasting memory and lets the transient trace go, and lasting memory itself is never let go. |
| Entity | A person, project, place, object, topic, character, organization, or recurring concept: an anchor through which memories are reached by relationship, not only by similar wording. What role an entity plays is the application's reading, never the library's. |
| Temporal context | When something happened and how events relate over time: recency, sequence, duration, intervals, anniversaries, change over time, and how long it has been. |
| Relation | A connection between memories and entities, typed or inferred: involved in, caused by, follows from, contradicts, resolved by, resembles. |
| Salience | Why a memory matters. Importance is judged when the experience is consolidated, from its behavioral, emotional, practical or relational weight and from surprise: what broke with expectation is what a person keeps, and the expected is what they let go. Importance stays revisable afterward, since a later event can make an earlier ordinary one matter. At recall it is weighed against evidence and elapsed time, never read from a stored score that drifts. |
| Reflection | A higher-level interpretation derived from several memories: a summary, a pattern, a stable observation, each linked back to the episodes it rests on. |
| Character signal | A stable tendency or preference inferred from memory, held with the situations it was seen in. Personality is never overwritten from outside; it is derived from remembered evidence. |
| Continuity | The character behaves as the same persistent entity over time. Coherent behavior across sessions is the goal, not recall of isolated facts. |
| Scene | The circumstances a memory was formed in, and the circumstances the character is in now: who is present, where and when, what is going on. A memory is remembered together with its scene, and the present scene is what brings memories to mind. The character knows whether it was there when something happened or was told about it later. |
| Continuity scope | The scope in which continuity state is meaningful. It follows from the scene a memory was formed in; the application never has to name it. |

---

## 7. Retrieval Philosophy

Retrieval should model recollection, not search alone. Human-like recall is associative: a person, character, place, time, recurring theme, object, project, or recent emotional tone can bring back relevant memories even when the wording is different.

Retrieved memory is material the character reconstructs a response from, not a record it recites. History should bend the response rather than appear in it as citations. The best continuity is invisible until it is tested.

This section describes what a person has in mind at a moment, and so what recall should give the character: first what the moment itself brings (7.1), then the shape of what is in mind (7.2), what singles a thing out for notice and for keeping (7.3), the things felt rather than thought (7.4 and 7.5), the case where nothing is there (7.6), and only then the signals and bounds that produce it (7.7 to 7.9).

### 7.1 Recall is situated before it is cued

A person walks into a room already carrying what the room calls for: who is here, what was last said with them, what is owed in either direction, what is in progress, what fell due today, what happened this morning. None of that is brought up by the topic, because there is no topic yet. The character should arrive the same way, and what the conversation then turns to should add to that, never replace it.

Purpose is not something the moment hands to the character. What it is trying to do comes up from what it remembers: an unfinished matter, a promise, a project in progress, a settled tendency. A purpose supplied from outside is a persona assigned for one turn. A purpose that surfaces from memory is accumulated character. An intention is remembered with where it stands, pending, changed, kept or dropped, and one that is done or withdrawn never fires again; remembering it is not the same as acting on it.

### 7.2 What comes to mind is situations, not records

A person has in mind a few situations, each gathered from everything that bears on it, each with a weight and a felt tone. An unsettled matter comes whole: what was promised, how long ago, what the other party thinks of it, and the character's own conduct on it, including what was said about the matter that nobody tied to the promise. A memory is in mind through a connection that can be named: it is about the thing, or shares a person, a thread, a place or a time with it. Two memories that merely sound alike are not connected; when the only link between a memory and the present question is that their words resemble each other, recall says so, so that the character does not mistake a similar-sounding memory for a relevant one.

Remembering and acting are different things, and recall is the first. A person greeted on a quiet morning has their unfinished story in mind whether or not they mention it; a person who would not raise something still carries it. Recall returns what a person would have in mind at that moment, acted on or not, and what comes to mind is never cut down to what should be said.

### 7.3 The unexpected is what comes to mind and what lasts

A person carries expectations into every moment, built from everything remembered: how this room usually is, what this friend usually says, how a Tuesday usually goes. The moment is read against them. What matches is barely noticed and is inhabited rather than remembered; what breaks with them is noticed, comes to mind, and is what the person still has a year later. The surprise need not be large or emotional. A quiet remark that did not fit, a habit that changed, a door that was open when it is always shut: these are kept, while a hundred ordinary Tuesdays merge into "Tuesdays".

Three things the rest of this section describes rest on this one. What comes to mind about familiar ground is what is new or unresolved there, which is to say what broke with expectation (7.4). What lasts when experience is consolidated is decided largely by how far it departed from what was expected, beside how much it mattered (section 6, salience). And a situation begins and ends where expectation breaks: a new person arrives, the topic turns, something happens that the current picture did not predict, and a person's sense of "that was one thing, this is another" follows those breaks rather than the clock (7.2).

For the character this is what keeps recall from being flat. A memory weighed by emotional and practical importance alone keeps the routine as readily as the remarkable, and a character built that way recites its ordinary days with the same weight as the day something changed. A person does not, and the difference reads at once.

### 7.4 Ground, and the feeling that comes before the reason

Familiar ground is not recalled, it is inhabited. With a place lived in or a person always present, the character has in mind what is new or unresolved there, not its history; the history is available the moment anything touches it. A place visited once, or a person met once, is a memory. After an absence the familiar becomes a memory again. Ground still colours the moment: familiarity carries a felt tone, comfort or unease, and that tone shapes the character even when nothing about the ground is in mind. The same familiarity with a different tone is a different moment.

The feeling comes before the reason. In a person the subconscious runs ahead of conscious thought: walking into a sensitive subject, they feel the alert first, without yet knowing why, and only then remember why and act on it. Much of what makes behaviour read as human comes from this order, the flinch before the explanation, the warmth before the name is placed, the hesitation nobody can account for until later. Not every reaction is like this; people also think before they respond, and a character should have both. But a character whose every reaction is deliberate reads as artificial. What the character feels about a thing is therefore handed to it as it is, wary here, warm there, heavy today, and it can act on that without ever putting the reason into words, as a person steers around a subject without consulting a list. In this library the reason is always there to be found, because every disposition rests on the memories that formed it and the link is kept. That is a choice, not a fact about people, whose feelings can outlive every memory of their cause; the library keeps the link because a feeling with no reachable reason is one the character cannot explain and the developer cannot correct.

### 7.5 The state the character carries into every moment

The character carries a current state into every moment, about nothing in particular in the room and made of memory all the same: the residue of what just happened, a bad morning or a good conversation an hour ago; the weight of anything impactful still close, a loss, a success, a fright, which colours everything until it fades; and how things have been going lately, a quiet stretch or a run of trouble. A person in a heavy week is shorter with a friend, slower to be curious, quicker to hear criticism, and notices different things; the character should be the same, whoever is in front of it and whatever is asked.

That weight fades, but not by the clock. Some events, good or bad, are never worn down and stay reinforced far beyond the rest. For the others, what lessens a feeling is a new experience: something that changes how the matter stands, or simply life going on, so a full stretch moves old feelings back quickly and an empty one, a long absence or a bored week, barely moves them at all. The character's sense of how long ago something was is counted in what it has lived since, not in hours. This is also why, returning after a long absence, a person finds the place strange and the people exactly as they left them: the place was not inhabited and has become a memory, while nothing has happened to displace how they feel about the people.

### 7.6 Not knowing is obvious to the one who does not know

A person asked about something they never lived, or something they cannot place, usually notices the gap at once and acts on it: they say they do not know, they wonder why it came up, they ask. For the character this has to be just as noticeable. When recall has nothing on a matter, or nothing it can place, that absence reaches the character as plainly as a memory would, so that the character behaves like someone who does not remember, instead of passing over the gap and answering from whatever else is at hand.

Where people fail here is instructive. A person who knows a name but nothing behind it, or who has lived something that merely resembles what was asked, tends to fill the gap with confident invention; the feeling of knowing comes from the familiar cue, not from any memory. So the character distinguishes three states and is handed each as itself: nothing found; something found but thin or uncertain, a name heard once, a resemblance and no more; and verified absence, where the thing asked about is one the record would have named and does not. Only the last is a flat "no". A miss by resemblance never shows that nothing happened. Having nothing on the topic does not leave the character blank: it still has the moment in mind, who is asking and what is going on with them, and its reaction comes from there, a question, a guess, a redirection, varying with the situation and the character.

### 7.7 Retrieval signals

Character Memory should therefore combine multiple retrieval signals:

- **Semantic retrieval** for meaning similarity.
- **Temporal retrieval** for recency, order, duration, and time-based relevance.
- **Entity-based retrieval** for recurring people, projects, places, characters, objects, and concepts.
- **Relational retrieval** for connected memories and graph traversal.
- **Salience-aware retrieval** for memories that matter beyond textual similarity.
- **Scope-aware retrieval** for continuity contexts that should not be treated as global.
- **Scene-aware recall** for what the present moment brings up before anything is said: the people here, the place, the time and date, what is in progress, and anything the character was waiting for exactly this moment to do.

A retrieved memory should ideally include why it was retrieved: similar meaning, same thread, same entity, recent event, unresolved thread, repeated pattern, contradiction, relationship relevance, high selectivity, or explicit scope. This makes the system debuggable and reduces arbitrary memory injection.

### 7.8 Recurring entities are continuity anchors, not traversal invitations

Entity continuity is central to recall, but the system should not treat every memory connected to a recurring entity as relevant. Any entity can become high-degree over years of memory accumulation.

A broad entity may still be important. However, retrieval should adapt to entity specificity, relation type, temporal context, salience, lifecycle/currentness, and explicit scope. The system should require additional narrowing evidence before expanding broadly through low-selectivity entity links.

This protects continuity without turning recurring entities into context pollution.

### 7.9 Serendipitous recall should be supported without false continuity

Human-like recall includes weak associative moments: one memory can remind the assistant of another through partial cues, repeated coactivation, or a shared context that is not strong enough to be a formal relationship.

Character Memory should eventually support this kind of serendipitous recall.

However, weak broad co-occurrence should not be treated as durable relationship truth. A recurring entity is a continuity anchor, but not every memory involving that entity should become directly associated with every other memory involving it.

The system should distinguish:

```text
entity incidence
query-time associative activation
association candidate evidence
active AssociativeUnit
strong durable relation
```

This preserves human-like recall while reducing false continuity and graph pollution over long time horizons.

---

## 8. Design Principles for Implementation

The principles are grouped by what they protect: continuity and character (8.1), memory held as experience (8.2), time (8.3), entities (8.4), what is never lost or hidden (8.5), and what keeps the system inspectable and portable (8.6).

### 8.1 Continuity and character

- **Design for character continuity first.** Backend choices, vector indexes, and graph storage are implementation details. The user-visible outcome is an assistant or character that remembers and behaves consistently over time.
- **Let memory influence behavior, not dictate it.** The LLM should receive memory as grounded context. It should not be forced into brittle rules from stale memories.
- **The library knows the scene and not the moment.** It knows who is here, where, when and what is in progress; it does not know how much room the conversation has, the tone the person arrived in, or what is pressing on them. So it reports the ingredients of a memory's weight, how much it matters, how long it has been, whether it is owed and to whom, whether its party is here, whether it was ever said, how it felt at the time, and passes no verdict on what to raise. Any finite recall expresses a judgement about what is in mind; the verdict the library withholds is the one about what to do with it. Which memories are in mind and how much each weighs are two different questions. Deciding what fits into a fixed number of slots, with slots reserved for kinds of cue, is a decision about what matters right now, made without knowing the moment; the library does not make that decision in the character's place.
- **A situation comes to mind as this character holds it.** Its facts, how its parts felt when they happened, the character's relevant tendencies, and its own conduct on the matter. The same overdue promise makes a conscientious character apologetic and a proud one curt, and that difference is the library's to carry: a tendency supplied by the application is a persona assigned, not a character accumulated. The library reports the facts, overdue by this long, to someone this close, this important, and never an urgency or a mood; pressure, guilt or irritation are what the character makes of them.
- **What a thing is to the character is one concept, whatever the thing.** A person, a place, a project, an object, a topic, a time of year, a group, the character itself: toward each the character holds a few situations explicitly in mind and a disposition formed over time, with the disposition's reasons reachable. The design should not take the shape of one kind of target.

### 8.2 Memory held as experience

- **Prefer episodes over isolated facts.** Facts are useful, but character is shaped by remembered events. Preserve the context in which facts emerged.
- **Treat raw source material as evidence, not the memory substrate.** Character Memory should model remembered experience through episodes, observations, entities, links, reflections, and current continuity views. Source material should ground those memories through provenance, but the core memory model should not be shaped around raw archival logs.
- **Do not collapse memory into unsupported summaries only.** Summaries are useful for compression, but behavior-influencing memory should remain grounded in episodes, observations, provenance, and correction paths.
- **Everything experienced leaves a trace; what lasts is decided afterward.** A person cannot know at the moment which remark will matter in a year, and the character should not have to either. What happened is kept as it happened, cheaply and without judgment. Deciding what it meant, what changed, what repeats, and what to carry forward happens later, when there is time to reflect and more is known. Nothing is lost for having seemed unimportant at the time.
- **What lasts must have earned it.** One remark is an observation, not a trait. A tendency is believed only after it repeats, and is held as a tendency, together with the situations it was seen in: a person is blunt with an old friend and careful with a stranger, and both are true of them. A tendency that is held without its situations becomes a label, and the counterexamples that would have kept it honest are lost, so they are kept.
- **A belief is built from experience, and the record says which.** Memory is subjective: it can never say what is objectively true, only that the character's experience has built toward believing something. Every kind of experience builds that way, including the character's own sayings and thoughts; a person who has told a story five times believes it more firmly, and so may the character. What the library refuses is to make the kinds indistinguishable. Witnessing something, being told it, and saying or thinking it oneself are different experiences, each counts as what it is, and the record keeps them apart, so that a belief resting on one sighting and four retellings is held as exactly that, and the character and the developer can both see what it rests on. The library passes no verdict on truth. What someone states plainly about themselves can be taken at their word; what is inferred about them cannot, from one instance. What was said in jest, in play, or as a supposition is remembered as that and never as fact. What others say about the character's own past is a claim, weighed against what it remembers.
- **A memory keeps how it felt.** What happened can be reconsidered for the rest of a life. In this library, how it felt at the time is kept as recorded and is only joined by how it looks now. People do not work this way: a person's memory of a feeling drifts toward what they feel about the matter today, and the two merge. The library keeps both on purpose, because the original feeling is evidence the character can be corrected against. What a person voices is how it looks now, so that is what the character leads with; the original feeling is there when the moment or the developer needs it.
- **Feeling is kept by keeping the experience.** A gist that has lost how something felt is not a substitute for the episode it came from. Where memory is compressed, the feeling is kept with it, and in the shape a person keeps it: what an exchange felt like is remembered by its strongest moment and by how it ended, not by its average, and what caused the feeling and who was central stay in full while the surroundings go to gist. A tense talk that ended warmly is remembered as warm.
- **A memory's meaning is other memories.** How a memory should be treated comes from what is linked to it: a request not to raise it, a correction, a resolution, a framing, a note about the source. Each of these is itself a remembered event with provenance. The library does not sort memories into treatments or annotate them with instructions, because an experienced fact can mean many things and the boundary between an instruction and an experience is not one the library can draw. When how a memory should be treated changes, the character's memory of it changes with it, and what it remembers afterward reads as one memory, not as a history of edits.

### 8.3 Time

- **Make time a first-class dimension.** A timestamp is not enough. The system should understand before and after, recent and old, repeated and one-off, ongoing and resolved, and how long it has been. Recall happens at a moment, and the gap since a memory is part of its meaning: a promise from last week and one from last year are not the same memory, and a person who stopped appearing months ago is remembered differently from one seen yesterday.
- **Time is counted in experience.** How much an old feeling still presses, and how long ago something seems, follow what the character has lived since, not the calendar; nothing stored decays on its own, and nothing is worn down except by what came after it.
- **The character knows how things stand before it has reflected.** In the afternoon it still remembers the morning: what it just did, what it was told an hour ago, what changed since yesterday. Recent experience is as reachable by what it was about as old experience is, and more literal.
- **The character knows when it was there.** A quiet afternoon is part of its life and is remembered as quiet. A stretch it was absent for is known as absence. It never confuses nothing happening with not having been there.

### 8.4 Entities

- **Make entities first-class.** People, projects, places, characters, objects, and recurring concepts are anchors for recall. Entity continuity is central to relationship and scope continuity.
- **Keep entity policy use-case agnostic.** The core library should not special-case user, assistant, player, NPC, protagonist, or other application roles. Application-specific layers can provide scope and role hints.
- **Bound entity expansion.** Recurring entities should support recall without causing unbounded graph traversal or accidental pairwise link growth.

### 8.5 Nothing is lost, and nothing is hidden

- **Recall is complete.** The character can find any memory it holds. No memory becomes less reachable because time passed, and no stored measure of importance drifts on its own. Completeness is a promise about memories reached by what they are: a person, a place, a thread, a time, an identifier. A memory reached only by resemblance of meaning is found less reliably as the store grows, through no fault of time, so reach by resemblance is bounded and measured, never promised. What changes with time is rank and expression: recent and important material is offered in detail, old and minor material as gist, and the whole is available when it matters. A person's memory of an old event thins with the years, and this character's does not: whatever consolidation kept stays recoverable in full, and that is an advantage to keep, not a trait to imitate away. What consolidation let go, the literal wording of an ordinary exchange, is recoverable only while its trace is still held, or from the source if the application kept it.
- **Forgetting is always someone's decision.** The record is append-only, and forgetting changes influence, not history. It takes exactly two forms, each intentional, reversible, and inspectable: suppression removes a memory's influence, and supersession replaces it with a corrected memory while keeping the old one as history. A memory that is no longer current, such as a finished project or a relationship that ended, is not forgotten: it leaves current views through a change of currency and stays fully recallable. Destructive deletion is not a memory operation; erasure exists only as an out-of-band operational action for compliance, security remediation, or explicit operator-directed alteration. A request to leave something alone is usually itself a memory to keep, not a suppression to perform.
- **Discretion is disclosure, not recall.** A person told something in confidence still knows it in the next room. They choose not to say it, and knowing it still shapes how they act. Recall is therefore never gated by privacy, sensitivity, or who is in the room: a character from whom a confidence is withheld is not discreet, only ignorant of what it should protect. Every memory carries its scene: who was present, who said it, whether the character was there when it happened, and in which setting. Recall reports that scene with the memory so the character can be consistent and careful at once. The character does not know everything about the room it is in, any more than a person does, so the library reports what was recorded and what it was told of the present, and passes no verdict on who may hear what; that judgment is made where the uncertainty can be weighed. Where an application has an obligation that discretion alone cannot meet, it meets it outside recall, by filtering what it passes on or by keeping what must never mix in separate memory, never by a property stored on a memory or a filter on what the character remembers.

### 8.6 Inspectable and portable

- **Preserve provenance.** Derived reflections and character signals should link back to source memories. This keeps the system auditable and correctable.
- **Expose retrieval rationale.** Implementation designers and application developers need to understand why a memory was retrieved.
- **Stay backend-agnostic where practical.** A default embedding provider and a default store exist for convenience; nothing in this philosophy depends on either.

---

## 9. Handing Memory to the Model

The library should feel like a memory system, not a database client: an application remembers, retrieves, reflects, corrects and forgets, and the names and workflows say so.

The library should make it easy for an application to insert memory into the LLM context in a concise, grounded form. Memory context should distinguish between raw remembered episodes, derived reflections, and stable character signals, so the application can decide what the character says and what it is merely shaped by. Any rendering help the library offers follows that distinction and favors gist and stance over quotation.

A model acts on what it is handed where a person merely carries it. So where the library hands memories to a model, it says in words how each is held: what is in front of the character, what is pressing, what is only shaping it, what it does not know. That rendering is the library's judgement offered as a default, readable and replaceable. The application owns the moment, how much room the conversation has and the tone the person arrived in, and the final words the model reads. It should never have to supply the character's feelings or stance itself; if it does, the memories were not doing their job.

Example memory context categories:

- Relevant episodes from past interactions.
- Active relationship, project, or continuity scopes.
- Preferences with provenance.
- Character-relevant reflections derived from repeated memories.
- Open loops, promises, unresolved questions, and recent commitments.
- Relevant current beliefs once the later belief layer exists.

---

## 10. What the System Should Avoid

- **Avoid looking like a generic RAG wrapper.** RAG retrieves information. Character Memory should preserve continuity of experience.
- **Avoid turning every remark into a lasting fact.** Everything leaves a trace, but few things deserve to become a belief, and the difference is decided with evidence and time.
- **Avoid personality overwrites.** Character should be reinforced through memory, not replaced by arbitrary labels.
- **Avoid false intimacy.** The system should only use memories it actually has and should make corrections possible.
- **Avoid unexplained recall.** When a memory influences behavior, developers should be able to inspect why it was selected.
- **Avoid silent forgetting.** If a memory stops influencing behavior, someone decided that, and the decision is visible. A memory that quietly faded is a failure the developer cannot diagnose and the character cannot explain.
- **Avoid third-person archive framing.** This is not a world chronicle. It is memory for a continuing assistant, character, or relationship context, held from that character's own point of view.
- **Avoid hard-coded role assumptions in the core library.** A system that only understands user/assistant roles will not serve companions, simulations, games, research systems, or developer tools well.
- **Avoid unbounded traversal through recurring entities.** Entity continuity is valuable, but broad entities need scope, selectivity, and supporting evidence.
- **Avoid always having something to say.** A search always returns its nearest matches; a person often has nothing. A character that fills the silence with the least unrelated memory is inventing continuity.
- **Avoid becoming a mirror.** A character that remembers what the person believes tends to agree with it, and one that remembers the person's life tends to bring it up whether or not it belongs. Both are measured failures of memory-equipped agents, and both are the opposite of character. An accumulated character holds its own view against a remembered one, and continuity is not compulsive reminiscence.

---

## 11. Non-Goals

- Not a complete agent framework.
- Not a replacement for the LLM.
- Not merely a vector database abstraction.
- Not only a chat history store.
- Not only a user profile or preference database.
- Not a roleplay-only system, even though it can support persistent AI characters.
- Not an application-specific entity-role engine in the core library.

---

## 12. Success Criteria

These outcomes summarize a fuller standard. The continuity situation catalog describes, from lived experience inward, how a character with human-comparable memory behaves: what it recalls unprompted and what it lets fade, how it treats the people it knows well and the ones it barely knows, how it handles being corrected, being told about itself, and the passage of time. The implementation is successful to the degree that a character behaves that way across those situations, whatever it perceives through and whatever mechanism stands behind it.

One measure stands above the list: someone who has spent a month with the character would be surprised to learn it had been replaced by a fresh instance halfway through. And one failure outweighs every success: invented continuity, the character saying warmly and with confidence something it never experienced, or filling a silence with the nearest unrelated memory. A companion that forgets disappoints; one that fabricates a shared past poisons trust in everything else it remembers. Lost continuity, blankness toward something the character plainly lived, is the second failure, close behind.

The implementation should be considered successful if it enables these outcomes:

- After a long gap, the assistant or character can recall relevant past events without requiring exact wording.
- The assistant can connect a current topic to earlier people, characters, projects, commitments, objects, places, or emotional context.
- The assistant can retrieve memories based on temporal relation, not only semantic similarity.
- The assistant can show stable behavior shaped by prior interactions while still accepting correction.
- The application developer can inspect why a memory was retrieved and where derived claims came from.
- The system can distinguish raw episodes, inferred reflections, and stable character signals.
- Broad recurring entities do not flood context merely because they are connected to many memories.
- The assistant feels less like a new instance every session and more like a continuing participant in the user's life or application world.
- Before anything is said, the character already carries what the moment calls for: who is here, what was last said with them, what is owed, what is in progress, and what fell due.
- The character can say what it did earlier today, and knows what changed an hour ago, without having had time to reflect.
- It can tell a quiet stretch from an absence.
- It does not mistake a joke for a fact, a single remark for a trait, or someone's claim about its past for its own memory.
- Surfaced detail matches relevance and importance. The character does not volunteer distant trivia, and it recalls the detail fully when asked.
- Memory failures have a human shape. Uncertainty surfaces as natural hedging with a stated basis, never as confident wrongness about the people and commitments the character knows best, and never as blankness toward something it plainly experienced.
- Asked about something it never lived, the character says so plainly and turns to why it was asked, instead of producing the nearest unrelated memory.
- The character carries what is on its mind without saying all of it. The same unsettled situation makes two differently shaped characters feel and behave differently, and the difference comes from what each remembers, not from its prompt.
- In a place it has long kept, or with a person always present, what comes to mind is what is new or unresolved, not the history, and the place or person still colours how the character is.

---

## 13. Final Guidance

### Design north star

Build Character Memory as the layer that lets an assistant or character carry experience forward.

The implementation should always be evaluated against this question:

> Does this design help the assistant or character remain behaviorally continuous across time?

When in doubt, favor designs that preserve temporal context, entity continuity, provenance, complete recall, correction, bounded retrieval, and reflection. These are the ingredients that let memory shape character instead of merely filling context.
