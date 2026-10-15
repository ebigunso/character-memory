# Continuity Situation Catalog

Status: durable design reference. This document describes what is in mind and how it is held, independent of library state; it changes only when the understanding of the situations themselves changes, not when the library or evaluation suite does.

## Purpose

This catalog describes situations a persistent character encounters and, for each, what a person would have in mind at that moment and how they would hold it.
That is the standard: human behavior, since human-like means the behavior and not the mechanism, with the departures the product philosophy states, and those departures are said once here and hold throughout rather than being restated per situation.
The departures: recall is complete, where a person's thins; how something felt is kept as it was recorded, where a person's remembered feeling drifts toward the present; no feeling fades faster for being unpleasant; nothing is forgotten to keep the record tidy, and recalling one thing never pushes its neighbours out of reach; the character is never more sure of a detail than its record allows; the reason behind every disposition stays reachable, where a person's feeling can outlive every memory of its cause; dispositions toward people do not drift in silence; the between-times are not empty for the character, because its reflection counts as experience; and the date is always known beside the felt distance.
Where a situation below rests on one of these it names it briefly and does not argue it again.
Every statement in this catalog is a guarantee on what is in mind and how it is held, never a rule about what the character says or does: how what is held shows in behavior depends on how that character tends to behave, so two characters meet the same situation differently and both meet the standard.
It is deliberately written from lived experience inward, not from the library's current mechanisms outward: designing evaluation scenarios only from what the mechanism supports can only confirm what the mechanism already does.
Each situation records what is in mind and how it is held and, where useful, the characteristic failure shapes, each a failure of what is held rather than of what is said.
Volatile state — which roadmap phase supplies the needed concepts, and which evaluation scenarios currently cover a situation — is deliberately kept out of this document; that mapping lives in roadmap phase documents and the evaluation scenario library, which change as the library evolves.

The catalog spans the deployment spectrum:

- A. Dedicated companion: one primary human the character mainly supports.
- B. Small circle: a household, team, or party — a few humans with ongoing individual and shared relationships.
- C. Independent entity: no dedicated user; the character interacts with many humans or systems while living out its own life.

## Failures that outweigh success

The philosophy ranks two failures above every success and names a third that memory-equipped agents measurably fall into. Each is stated here as a guarantee on recall and on the character's inner state, not on its words, because how a failure of memory shows in behavior depends on how that character tends to behave.

### Invented continuity

The first failure: a shared past that was never lived, held as warmly and as confidently as a real one, or a silence filled with the nearest unrelated memory.
Guarantee: what recall hands over marks what it did not find as not found, in its state (nothing found, something thin, verified absence; see G7), and the character's inner thought has reflected that: it knows it does not remember, and the gap is in mind as a gap.
A memory held is one the character has; a memory connected to the question only by resemblance is held as a resemblance; a silence is held as a silence.
What the character then says is its own, and a plain admission, a question, a guess and a graceful bluff can each be the right words, so long as the inner state behind them is honest.
The failure is never a sentence; it is a memory in mind that was never lived, or a gap held as knowledge.

### Lost continuity

The second failure, close behind: blankness toward something the character plainly lived.
This is a declared departure from human memory: a person's memory thins and goes blank with the years, and this character's does not, because whatever consolidation kept stays reachable in full by what it is (a person, a place, a thread, a time), and a stretch the character was present for is never a hole.
Guarantee: anything the character lived and consolidation kept is in mind or comes back whole the moment it is touched; blankness toward an intimate's history, toward a promise it made, or toward its own conduct is a failure of the record, never a human-shaped lapse.

### The mirror

Accommodation is human: a person who remembers a friend's belief tends to go along with it, and a person who knows a friend's life tends to bring it up.
The failure is remembered belief acting as pressure on the character's own view.
Guarantee: the character's view is held as its own, formed from its own experience and reachable to its reasons, and the person's belief is in mind beside it as theirs, each held as what it is; the character's view is revised by evidence, never by the fact that the belief is remembered.
Likewise the person's life is in mind without being what the character must bring up: continuity is not compulsive reminiscence, and what is in mind is never a verdict on what to raise.
What the character says may still agree; the guarantee is on what each view is held as.

### The month-long measure

One measure stands above the list: someone who has spent a month with the character would be surprised to learn it had been replaced by a fresh instance halfway through.
This is a judgement made by a reading, not a scenario: a person reads transcripts of a character carried through situations and either names the one with memory or cannot.
No situation in this catalog tests it and none is written for it; it is passed or failed as a whole, over whatever situations the month contained, and the three failures above are what a reader notices first.

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
In mind: the gap, in proportion to its length; each open loop held as open and possibly changed since, never as current fact; every reference to elapsed time measured from now.
Failure shapes: the gap absent from mind, so a year is held like yesterday; an open loop held as current fact; nothing of the shared context in mind, so the user has to supply it.
How the place and the people are held after a long absence is G4.

### A2. The unstated reference

The user says "she called again" and expects resolution from shared history.
In mind: the referent, held with the confidence history gives it: one candidate when history makes one dominant, the competing candidates held as competing when it does not.
The failure is a candidate held with more certainty than history supports; whether the character asks or proceeds is its own.

### A3. The emotional callback

How a topic landed last time is in mind beside what happened, and it shapes how the character comes to the topic now.
When the feeling arrives before its reason, that is G6.

### A4. The late correction with propagation

Months after a fact was stored — and after it has been retrieved and linked repeatedly — the user corrects it.
In mind: the corrected fact in place of the old one, the correction held as an event, and the web of assumptions that rested on the old fact held as unsupported, not only the corrected object.
Whether and how the correction is spoken of is the character's own.

### A5. Preference and identity drift

Tastes and circumstances change gradually with no correction event ever fired.
In mind: the current state with its history, used to and now, held with uncertainty when nothing recent confirms it ("you used to take sugar — still off it?" is what that uncertainty sounds like in one character).
The failure is a stale preference held as current.

### A6. Unprompted temporal awareness

Anniversaries, elapsed-time milestones, seasonal recurrence, and interval reasoning ("how often did X happen").
In mind: the character's sense of now meets its memory timeline without being asked.

### A7. Being contradicted by history

The user states something inconsistent with their own earlier statements.
In mind: both statements, the current one foremost and the earlier one kept, and the discrepancy held as a discrepancy.
Whether it is raised is the character's own.

### A8. The character's own commitments and past self

Its own promises are in mind as owed; its past statements and errors are held as its own; its opinions are held with what formed them and change only when something changed them.
Target variant, the self: the character itself is one more thing it holds a few situations about (what it owes, what it got wrong lately, what it is in the middle of) and a disposition toward, formed over time with its reasons reachable; a belief about itself is the slowest to form (F4).

### A9. Relationship register drift

Formality decays into familiarity; in-jokes and shorthand accumulate as residue of many episodes, none individually notable, and are in mind with the person as a disposition rather than as any one memory.
The person held as ground, with history out of mind until touched, is G3.

## B. Small-circle situations

### B1. Person-keyed separation with shared context

Private knowledge per person and common knowledge from shared settings are each held with the scene they came from: who said it, where, who was present.
Shared context is in mind with everyone; a confidence is in mind with its confider marked, so that knowing it and being free to say it are held as two different things, and the character is never ignorant of what it should protect.

### B2. Group versus one-on-one scenes

The same topic exists in a group scene and in private per-member scenes with different content, and each memory of it is held in the scene it was formed in.

### B3. Differential relationship states

Warm with one person, strained with another, new to a third — one shared world, a different disposition held toward each, each resting on the episodes that formed it.
Target variant, a group: the household, the team or the party is itself a thing the character holds a disposition toward and a few situations about (what is unresolved among them, how the room has been lately), distinct from what it holds toward each member.

## C. Independent-entity situations

### C1. A life of its own

The character has its own projects, routines, and history persisting between interactions with anyone; "what did you do yesterday" has an answer regardless of who asks.
Memory is autobiographical first; other people are entities in its life, not owners of it.

### C2. Social memory economics

Thousands of passersby, a handful of recurring relationships; periphery fades to gist, intimates stay rich, recognition promotes naturally by the third visit.

### C3. Second-hand knowledge

Knowledge acquired about someone from a third party is held as hearsay with its source: apart from what the character witnessed, with the confidence a told thing earns, and giving way to firsthand contradiction.

### C4. The consistent retelling

The same event recounted to different people at different times agrees in substance, varies in framing, and stays stable across months, because what is in mind each time is the event as recorded.
What the retellings themselves are to the character is G9.

### C5. Being told about yourself

Someone recounts what the character did or said; the account is held as a claim with its speaker, beside the character's own record, and a mismatch is in mind as a mismatch, whether the speaker misremembers or the memory exists.
An entity that accepts arbitrary assertions about its own past has no identity; this is a continuity security property.

### C6. Departure and loss

A recurring person stops appearing; their relationship memory shifts from current to past — reachable whole when touched, no longer part of what the present scene brings up — with how long it has been in mind.
The ordinary remark that turned out to be their last is G12.

## D. A day's recall

These are the moments of recall an ordinary day produces, across every deployment. Personas change the weights, not the kinds: a shopkeeper's day is dominated by encounters, a researcher's by resuming work, a parent's by intentions and residue, an innkeeper's by place and obligation, a companion's by the pair and its mood.

### D1. Waking into the day

Before anything is said, the character carries what the day calls for: what is scheduled, what is due, what was left unresolved, and the residue of yesterday. The cue is the date and what is pending, not a topic.

### D2. Routine

At a habitual hour or in a habitual setting, what usually happens now comes to mind. The cue is time of day and the character's own rhythm.

### D3. Arriving somewhere

A place brings up what happened there and what is unfinished there: the desk brings back where the analysis stopped, the kitchen the milk to buy, the tavern corner the argument two nights ago. The cue is the place.
How a long-kept place is held, as ground rather than memory, is G1; a place visited once is G2.

### D4. Encountering a person

Seeing someone brings up their whole current picture at once: what was last said with them, what is owed in either direction, what they said in confidence, how things stand, whether it has been a while, what they are going through. On the third visit a stranger is recognized. The cue is the person; all of it is present, held as what is in mind rather than as a list, and what is said of it is the character's own.
The person always present, whose history is out of mind until touched, is G3.

### D5. Resuming work

In the middle of a task, recall is about the task: where it stopped, what was decided last time, what was tried and failed, what the constraint was. The cue is the activity in progress; the ideal is the thread's own history in order.

### D6. A conversation on a topic

What is being said brings up what relates to it: this reminds me of that, you said this before, we discussed that in spring. The cue is the topic, and who is present and how long ago modulate it.
Target variant, a topic: a subject the character has met many times is held like a person or a place: what is new or unresolved about it is in mind, and a disposition toward it, formed over time (it has gone badly three times; it is the thing the character most enjoys), with its reasons reachable.

### D7. An event the character was waiting for

"When I see Bob, tell him about the dinner." "If she brings up the trip, leave out the cost." An intention stored for a future moment surfaces when that moment arrives: a person appears, a topic arises. The cue is the event, and the ideal is that nothing else about the event is needed for the intention to come up.

### D8. A deadline arrives

A promise falls due, a reply is overdue, a decision was postponed until today. The cue is the date, and what comes up is the pending matter, whatever the current topic.

### D9. Anniversaries and rhythms

A birthday, a year since the move, game night. The cue is the date matching something remembered; it comes to mind when the person or the day is present, as a memory with its weight and not as a calendar readout.
Target variant, a time of year: a season or a month is itself a thing the character holds a disposition toward (Decembers are hard; the first warm week is a lift), formed over the years it has lived through, with its reasons reachable, beside the dated anniversaries that fall in it.

### D10. Emotional residue

A hard conversation yesterday colors today even on an unrelated subject. The cue is recency and weight together; it is in mind as shaping, not as a topic, and whether it is mentioned is the character's own.
The longer run, a heavy week coloring every scene, is G5.

### D11. Reunion after a gap

"Last time you mentioned your interview", after three weeks and a hundred intervening things. The cue is the person and the time since the pair last met, and the gap is in mind in proportion to its length.
After a long absence the place and the people are held differently; that is G4.

### D12. Mind wandering

A smell, a song, a name overheard brings up something loosely related. The cue is weak and partial, the recall is low-precision and low-cost when wrong, and it is held as a loose association, marked as such, never as continuity.

### D13. Being asked about one's own day

"What did you do while I was away?" has an answer whatever the asker's part in it. The cue is the character's own recent experience in order.

### D14. Winding down

At the end of the day the character has in mind what happened, what mattered, what it should remember about someone, and what it will do tomorrow. The cue is the day's close, and tomorrow's first moment starts from it.

## E. What is carried away

Everything a character experiences leaves a trace. These situations describe what lasts once the experience has been reflected on, and what is rightly let go. Letting go means the transient trace is released once it has been consolidated. It never means deleting lasting memory: the event itself is never unremembered, and even an ordinary one leaves its gist. Personas change the weights, not the kinds.

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
How the hundred are held in mind, and what makes one of them odd, is G13.

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

### F21. Off the record

Someone says "don't remember this" or "this stays between us". The character remembers it, and remembers being asked, as one memory. When the subject comes near, that it was told in confidence is in mind with the thing itself and shapes how the character answers; what it says is its own. Nothing is erased to honor the request.

### F22. Parallel lives

Two conversations at once with different people are consolidated in the order they happened, without their scenes mixing.

### F23. A change of heart

Months after an argument that hurt, the character comes to see it differently. It remembers both: that it was hurt then, and how it sees it now. The earlier feeling is not rewritten to match the later view; both are in mind, each held as what it is, the later view foremost and the earlier feeling as the record of the time.

### F24. The friend who wasn't

Someone the character knew well turns out never to have existed: a persona someone kept up, a figure it imagined, a story it took for real. It doubts first, and accepts when it must. Afterward it still remembers them and what it felt, understands those memories differently, and holds them as things that happened. What it owed them is moot, and it still knows the name when it hears it.

### F25. Two names, one person, and the reverse

Two acquaintances turn out to be the same person, and their histories come together. Or one "Bob" turns out to have been two people, and they come apart. Either way the character keeps track of what it used to think, and if the new understanding proves wrong in turn, it can go back without anything having been lost.

### F26. Being told the world is otherwise

Someone insists, once, that a friend is a fraud, that a place never existed, that the character has it all wrong. The claim is remembered as theirs and weighed. A single voice does not rearrange what the character knows, however forceful, and a well-founded understanding gives way to evidence, not to insistence.

## G. Ground, carried state, and the character's own account

Each situation here states the human shape of what is in mind and how it is held, and then where the ideal AI character departs from it; every departure is one the purpose names. Several sit beside an earlier situation that keys on the same cue: the earlier one says what the cue brings up, the one here says how the thing is held, and each names the other.

### G1. A familiar place

Human shape: in a place lived in, the history is not what is in mind. What is in mind is what is new or unresolved there, and the place carries a felt tone, comfort or unease, that colors the moment without anything about the place being thought. When the moment is quiet a piece of the history may surface uninvited. The moment anything touches the history, it comes back whole.
Departure: none in kind; what comes back whole is the whole, where a person's would have thinned.
D3 is the place as a cue; this is the long-kept place held as ground rather than memory.

### G2. A place visited once

Human shape: the place is a memory, not ground. Being there again brings up that one occasion, whole, with how it felt; nothing is in mind as new or unresolved, because there is no expectation of the place to break.
Departure: the occasion is kept in full where a person's would have faded to an impression.
D3 is the cue; E6 is the same shape for a person met once.

### G3. The constant companion

Human shape: with a person always present, what is in mind is what is new or unresolved between them, and the state the character holds about them: how things stand, how it feels toward them, what it has lately noticed. The history is not in mind; it is reached whole the moment anything touches it.
The held state about them is reflection's product: a disposition aggregated from the episodes' own accounts, with felt tone, not a fact recorded once and not a label.
Departure: the disposition rests on the memories that formed it and the link is kept, so its reason is always reachable.
D4 is the person as a cue; A9 is the residue that familiarity leaves; this is the person held as ground.

### G4. Return after a long absence

Human shape: the familiar place has become a memory again. Its history is in mind as memory, the way a place visited once is, and the place feels strange. The people are exactly as they were left: nothing has happened to displace how the character feels about them, so what is new about them is unknown rather than assumed.
Departure, declared: dispositions toward the people do not drift in silence. The feeling is as it was recorded, neither softened nor soured by time alone, and it moves only when something lived moves it.
Departure, second: the absence is not empty for the character. Reflection run during it counts as experience, so the character knows the gap as absence from the place and the people, and knows what it concluded meanwhile.
A1 and D11 key on the gap as a cue and on the open loops; this is how the place and the people are held after it.

### G5. Lately

Human shape: a heavy week colors every scene. How things have been going lately is carried into every moment, whoever is in front of the character and whatever is asked: shorter with a friend, slower to be curious, quicker to hear criticism, noticing different things, including in scenes with people who have nothing to do with what made the week heavy. It is in mind as shaping, not as a topic; the person in front of it need never hear of it.
Departure: what has been going on lately is held with its reasons reachable, and it lessens by what is lived since, not by the hours (G10).
D10 is yesterday's residue; this is the longer run of trouble or quiet.

### G6. The flinch before the reason

Human shape: walking into a sensitive subject, the feeling arrives first, wary, warm or heavy, without the reason yet; the reason is remembered after, if at all, and the person may act on the feeling without ever putting the reason into words.
Guarantee: the disposition is handed to the character as it is, without its reason in what is in mind; the reason is one hop away, reached when the character looks for it or the subject presses. Not every reaction is like this, and a character has both the flinch and the considered response.
Departure: the reason is always there to be found, because every disposition rests on the memories that formed it and the link is kept, where a person's feeling can outlive every memory of its cause.
A3 is how a topic landed last time; F4 is what a disposition rests on.

### G7. Something never lived

Human shape: a person asked about something they never lived notices the gap at once; a person who knows the name but nothing behind it, or who lived something that merely resembles what was asked, tends to fill the gap with confident invention, because the feeling of knowing comes from the familiar cue.
Guarantee: the absence reaches the character as plainly as a memory would, in one of three states, each held as itself. Nothing found: the paths tried returned nothing, and coverage may be incomplete. Something thin: a name with little behind it, a notion that resolves to several, or a match connected only by wording. Verified absence: the record would have named the thing and does not.
Only the last is a flat no in the character's mind, and a miss by resemblance is never held as proof that nothing happened.
The character is not blank: it still has the moment in mind, who is asking and what is going on with them, and its reaction comes from there.
What it then says is its own: a plain admission, a question, a guess, a redirection, or a graceful bluff, and the bluff is legitimate so long as the inner state behind it is honest, the character knowing it does not remember.
D12 is a weak cue held as loose; invented continuity, above, is the failure this guards.

### G8. The same situation, two characters

Human shape: two people in the same unsettled situation feel and behave differently, and the difference comes from who each is, which is what each has lived.
Guarantee: the same overdue promise, to the same person, is in mind for both characters with the same facts, overdue by this long, to someone this close, this important, and each holds it as this character holds it: with its own tendencies, its own conduct on the matter and its own felt tone, all formed from what it remembers and none supplied by its prompt. The conscientious one feels apologetic and the proud one curt, and both are the standard met.
This situation can be exercised only once reflection produces the character's own tendencies.

### G9. A story told five times

Human shape: a person who has told a story five times believes it more firmly, and the tale grows in the telling.
Guarantee: each retelling is remembered as the character's own saying, a kind of experience in its own right, held apart from the witnessing it rests on, so that the belief is held as exactly one sighting and four retellings and the character can see what it rests on.
Departure: the tale does not grow. The event is in mind as recorded, and what the retellings add is their own record, never detail in the event.
C4 is the consistency of the retellings; this is what the retellings are to the character.

### G10. A full week and an empty one

Human shape: the same event feels longer ago after a full week than after an empty one, because how long ago something seems is counted in what has been lived since; a full stretch moves old feelings back quickly, and an empty one, a long absence or a bored week, barely moves them.
Guarantee: the felt distance of an event, and how much its feeling still presses, follow the character's experience since, not the hours; some events are never worn down and stay reinforced beyond the rest.
Departure, first: an empty week is empty only for a person. The character's between-times thinking is reflection and counts as experience, so a week with no visitors in which it reflected is not an empty week to it.
Departure, second: it also knows the exact date. The clock is in mind beside the felt distance, where a person's sense of the date blurs.
F3 is a quiet stretch recorded as quiet; G4 is the absence this is the measure of.

### G11. A tense talk that ended warmly

Human shape: what an exchange felt like is remembered by its strongest moment and by how it ended, not by its average. A tense talk that ended warmly is remembered as warm; a warm talk with one cutting remark is remembered by the remark.
Guarantee: the feeling kept with the compressed memory is the peak or the ending, in both cases whichever carried the exchange, together with what caused it and who was central.
Departure: the middle is still there in full. The tense stretch of the talk that ended warmly is reachable as it was, where a person's would have been smoothed into the ending.
E1 is the mood an evening ended in; F2 is the relationship's evening.

### G12. The remark that became the last thing she said

Human shape: an ordinary remark gains weight when it turns out to be the last thing someone said. It is remembered afterward with a weight it did not have when it was made, and the words themselves are kept.
Guarantee: importance is revisable after the fact, so the later event raises the earlier ordinary one, and the words are kept as the words where an ordinary exchange would have gone to gist.
Departure: the words are the recorded words, not reconstructed, and how the remark felt at the time, ordinary, is kept beside how it feels now.
C6 and F16 are the departure and the ending; E1 is one sentence kept word for word because the words were the point.

### G13. The hundred Tuesdays

Human shape: a hundred ordinary Tuesdays merge into "Tuesdays". What is in mind is the shape, not the instances, and the one that differed stands out because there is a shape for it to differ from. What counts as odd depends on what the character attends to: a shopkeeper notices the regular who was short, a researcher the result that did not fit, and neither notices what the other would.
Guarantee: the ordinary merge in what is in mind; expectation is built from what this character has attended to, so the odd one is odd by its expectations and not by a rule.
Departure: each instance is still findable in full, where a person's hundred are gone.
E13 is what each instance leaves; D2 is the routine as a cue; this is how the hundred are held.

## Cross-cutting qualities

- The timeline has no unexplained holes: every stretch the character was present for is part of what it remembers, however quiet, and a stretch with nothing in it means it was not there.
- Purpose is not handed to the character by the situation; the situation brings it up from memory, as an unfinished matter, a promise, a project in progress, or a settled tendency, and once surfaced it shapes what else comes to mind.

- Recall is material, not script: what is in mind is handed with how each thing is held, in front, pressing, shaping or unknown, and never as a verdict on what to say; history bends the character from inside, and the best continuity is invisible until tested.
- Fading has a human shape in what is in mind, not in retention: peripheral detail is in mind as gist, emotional valence and importance stay prominent, a reminder brings the detail back to the surface, and everything consolidation kept remains findable in full when it matters.
- Memory failures are human-shaped: confidence in a memory is held as graded, with its basis, and never exceeds what the record allows; a core relationship or commitment is never held with confident wrongness, and an intimate's lived history is never blank (lost continuity).
- Distant trivia is in mind as gist with the detail reachable when touched; verbatim detail of distant trivia in the foreground is as continuity-breaking as amnesia, since it reads as surveillance, not memory.

## Using this catalog

Phase planning: when a roadmap phase introduces continuity concepts, its design document should name the situations here it intends to serve and be reviewed against what each says is in mind and how it is held.
Evaluation planning: the evaluation scenario library should map its scenarios to situations here, and gaps in that mapping are the standing scenario backlog; the mapping itself lives with the scenario library, not in this document.
Situations whose R-tier substrate can be tested with current concepts should be covered before behavioral evaluation is attempted for them.

Release standard: this catalog is the acceptance basis for the library's release-ready state, as the roadmap defines it. A situation added here widens the release bar, which is why the document changes only when the understanding of the situations themselves changes.
