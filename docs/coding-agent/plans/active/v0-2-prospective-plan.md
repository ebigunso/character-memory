# Plan: the character carries what it owes, what it is owed, and what fell due

- status: in-progress
- generated: 2026-09-22
- last_updated: 2026-09-23
- work_type: code

## Goal
- A character that meets someone arrives knowing what stands open between them. The most salient unsettled obligation naming that person, in whichever direction it runs, has room reserved for it under any topic. More obligations come, in either direction, as room allows or as the application raises the trigger floor. A promise that fell due today, or is overdue, comes to mind on that day and every day after until it is settled, whatever the conversation is about. It brings what bears on it directly, not the whole history of the person it concerns. The character knows which side of each obligation it is on, because the application told it at construction which notion is itself. Nothing about recall changes for knowing that. Both ways an obligation comes to mind are rows of the road table the consolidation slice built, so the five principles and ADR-D-0023's one bounded hop explain the room they take, and the result says how each obligation came.

## Definition of Done
- An open loop or a commitment can say who owes whom. Its actors and its counterparts are each one of the memory's own notion subjects, given by the application through the closed assertion vocabulary (ADR-D-0035) and never inferred. Several actors or several counterparts are true obligations ("I promised Bob and Alice"). The write is rejected in three cases: a role on a subject the memory is not about, both roles on one subject, or a role on a memory that is not an open loop or a commitment. The stored shape is the existing assertion node; no new field carries a party.
- The facade is told at construction which notion is the character itself (ADR-D-0020). The identity has exactly two effects:
  - the direction report on each admitted obligation, which is the self among its actors or among its counterparts;
  - the check that a store is never opened as someone else.

  No retrieval path treats the self as a special role. It is never a cue, a root, a floor, a filter, a scene participant added by the library, a notion required to exist, or a score term. The README says the character perceives the scene and is not listed as a participant in it. An application that lists it anyway gets it treated as any notion, so its obligations then come by trigger.
- The identity is kept as one literal in the graph store, written on first open. Opening the store with a different identity is refused with a plain error, before the vector store is opened. This prevents a silently inverted direction: a store opened as someone else would report every I-owe-you as you-owe-me. The literal is not a notion, and nothing in recall reads it.
- An open loop or a commitment can carry a due instant (`due_at`), given by the application as a UTC instant and never parsed from text. It is stored in the UTC instant literal form that every time query already casts. No offset is stored with it: an instant is absolute, and ruling 64 concerns the offset a scene was given. A due instant on any other kind of interpreted memory is rejected at the write. The instant is part of what a write says, so a replay with another instant is a content difference.
- The road table (`RecallRoad::rule`) gains two rows. Both come under principle 1, room reserved for "what a stored intention was waiting for" (rulings 50 and 75). Under principle 2 both are exact roads. Both come after the activity row, so in the root key's road row they follow the given roads and precede the search and time roads. The table's expands column gains a third value, one hop, used by these two rows only:

  | road | floor kind | admitted by (reported) | expands | reserves | contributes |
  | --- | --- | --- | --- | --- | --- |
  | trigger: an unsettled open loop or commitment that names, as actor or counterpart, a notion the scene resolved | trigger | participant | one hop | yes | up to the root cap for each party, in the state selector's order |
  | due: an unsettled open loop or commitment whose due instant falls before the end of the scene's local day | due | due | one hop | yes | up to the room, in the state selector's order |

- One hop satisfies ADR-D-0023's accepted decision that surfaced state re-cues one bounded hop (ruling 76):
  - The obligation root expands exactly one hop (depth 1) within the existing per-node fanout and node bounds. The expansion is recorded in the trace's graph expansion entry for that root.
  - What the hop reaches holds reminder standing and opens nothing further (rulings 46 and 54). It carries no cue score and inherits none. It sets a graph component only where no expanding road reached it (ruling 49). It reports the row's admission road, as every reminder's reach does.
  - The root takes no spare turn at root selection; only rows that open history take turns (principle 4).
  - A root that another road also reached, and which that road opens, expands as that road does.
  - An overdue promise to an absent Bob therefore brings its source occasion and what is linked to it directly, not Bob's history.
- The trigger road runs once for each notion the scene resolves by key or by name. An ambiguous name triggers for each notion it could mean. A party is named by the obligation's own assertion, never by who was present when it was formed (items 26 and 70). A description is never an identity and triggers nothing. Each party is one state scope of the trigger kind in scene order, so the rounds among the scopes of one kind (principle 4) serve each person's reserved room before any person gets a second.
- The due road runs once per retrieval. "Due" is judged by the scene's local day: the calendar date of the scene time at the offset the application gave. A scene time the library fixes itself is at UTC (ruling 73). Overdue obligations keep coming until they are resolved or superseded. The due road contributes nothing due after the scene's local day; the trigger road has no due condition, so an obligation due tomorrow still comes by trigger when its party is present. The as-of cut does not apply to interpreted memories (ruling 71). It applies, as inherited, to any occasion the hop reaches.
- Both roads read through the state selector, so a resolved or fulfilled obligation is contributed by neither. It stays reachable by every other road and is marked resolved by, as today (item 27). A superseded obligation is excluded by default and included, marked as elsewhere, when the caller includes superseded.
- Order:
  - Current before superseded, then salience, then newest, is the selector's order. It is guaranteed only for which obligations each road contributes and which obligation holds each floor.
  - Every other stage uses the ordinary order. At the root stage that is the root key: score, where these roots have zero; then salience; then memory time; then road row, position and id (ruling 65). The key carries no supersession status. At sections it is the final score.
  - With superseded included, a high-salience superseded obligation can therefore come before a low-salience current one once both are admitted.
- Measured effects, stated rather than claimed away (Tier D REV-R1):
  - An obligation that another road already reached as a descendant becomes a direct root, so its own graph component rises to one. At salience 0.5, a participant descendant at cue 0.75 and proximity 1 goes from 0.6625 to 0.7875. Its cue component is unchanged.
  - A direct topic root already has graph component one, and its score does not change.
  - A reserving root takes a seat under the root cap, and an expanding root it displaces can take its paths with it.
  - Floors reserve room; they do not bound these effects. The measurement's falsifiers do.
  - No scoring rule changes.
- Every admitted open loop or commitment reports two values on its `IncludedDerivedMemory` entry, in the result and with the trace off:
  - `direction: Option<ObligationDirection>`: `OwedByCharacter` when the self is among the actors, `OwedToCharacter` when among the counterparts, `None` when the character is not a party or no roles were given.
  - `due_state: Option<DueState>`: `Overdue` when the due instant is before the scene's local day, `DueToday` within it, `NotYetDue` after it, `None` when no instant was given.

  A memory the due road contributed, or reached in its hop, reports `AdmissionRoad::Due` in `memory_scenes[].admitted_by`. A memory the trigger road contributed, or reached in its hop, reports `AdmissionRoad::Participant`, because it is connected to someone present, which is what that value already means. Nothing is removed, omitted or reordered because of these values (ADR-D-0018).
- A retrieval on a store holding no roles and no due instants selects what it selects at the base: the same ids, sections, order, scores and `admitted_by`. No trigger or due root exists there. The one difference is that the constructor now names the self. Where obligations are contributed, the differences are the measured effects above.
- Retrieval reads no clock. The reference scene is captured once, and the same store, scene and input run twice, on any two days, give the same result. This holds for every task and is not restated per task.
- The slice-end measurement numbers exist in this plan's Decision Log, each with a paragraph on what they mean for the character and a verdict. The plan is not complete until they do.

## Planner-added requirements
- Direction lives in the assertion vocabulary, not in new fields. Needed because:
  - An assertion must already name one of the memory's subjects (ADR-D-0035's invariant, enforced by `validate_belief`). That is exactly the rule that a party can never be authored inconsistently with what the memory is about.
  - The assertion node already has graph storage, hydration, replay equality and a selector pattern by predicate and subject (`select_notions_known_as`).
  - ADR-D-0035 admits a new predicate only with a concrete reading behavior, which the trigger road and the direction report are.

  There are two unit predicates, `BeliefPredicate::Actor` and `BeliefPredicate::Counterpart`. Each may appear on any number of subjects, but never both on one subject.
- The self is nobody special in recall (ADR-D-0020, ruling 76). Needed because: the record's Decision text says no retrieval path treats the self as a special role. The character perceives the scene rather than being present in it, so the README tells applications not to list it. Listed anyway, it is treated as any notion, and its own obligations then come by trigger on every scene that lists it; that is the application's choice.
- The identity is checked against the store. Needed because: ADR-D-0020 says a scope may name the same self again and never a different one. Without a check, a store opened under another identity reports every direction inverted with nothing to show it. The smallest form is one literal, written on first open and compared at every open, with a mismatch refused by one plain error.
- Due is judged by the scene's local day, not by the instant alone. Needed because: a person wakes carrying what is due today (D1), and a promise due this evening is carried from the morning. The scene keeps the offset it was given (consolidation Task_6), so the boundary is one instant computed in Rust from `scene.time` and bound into the query.
- Both rows reach one hop at score zero and take no spare turn. Needed because ADR-D-0023 (accepted) says surfaced state re-cues one bounded hop, and an accepted record wins over the leaf reading of principle 5 (ruling 76). The row must not open history at full strength either: the knowing in "Bob is here" is the participant row's, and an obligation opening it again would get a turn equal to a person's, the defect ruling 59 removed (ruling 75). One hop with reminder standing is exactly the record's bound. The coordinator's ruling that a due intention takes a floor and no spare turn follows, because only rows that open history take turns.
- A triggered obligation reports `Participant`, and a due obligation reports a new `AdmissionRoad::Due`. Needed because ruling 72 reports the road so that a resemblance never reads as knowing. `Participant` already means connected to someone present, and nothing reached by these rows is anything else. No existing value can say "fell due" without lying: `Range` reads as dates asked about and `Recency` as lately.
- Current before superseded is a selector and floor guarantee only. Needed because `GraphMemoryRank` carries no supersession status, and the root key sorts salience and time before position (Tier D REV-R3). Carrying it further would need a new rank field and a new key position for one stage's benefit; the floor already serves the current obligation first.
- Each new read is checked at a scaled store before review (ruling 67). Needed because the anniversary read with people present took 20.6 s at 2000 daily episodes before it was reshaped, and a read joined through assertion nodes or filtered by a cast literal can be planned just as badly.

## Scope / Non-goals
- Scope: the production modules, named test files and documents each task lists under owns, and this plan. No decision record: the two predicates are admitted by ADR-D-0035's rule for new predicates; the identity at construction is ADR-D-0020's own Decision text; the one hop is ADR-D-0023's; and the rows follow the principles the consolidation slice recorded.
- Non-goals:
  - A single Preference subtype replacing the user and assistant preference subtypes (a rename that changes no behavior; it goes to the v0.2 closeout value audit).
  - A pair object or stored pair scope. In a first-person store every obligation with one other party is between the character and that party by construction; the assertions name the party and the direction says which way (Open Questions).
  - Silence relative to a pair's cadence (deferred with rhythms).
  - The memo's phrasing of direction, due state and admission roads (the renderer plan).
  - A topic trigger beyond the topic road.
  - A resolve method on the facade.
  - A lookahead, a snooze, or recurring due instants.
  - A local date field on the obligation.
  - Any road that treats the self specially, including one that brings the character's own promises with nothing said.
  - A trigger or due road that opens more than one hop.
  - A guarantee of current-before-superseded beyond the selector and the floors.
  - A due instant on episodes, observations or threads.
  - Migrating stored data.
  - Parsing dates or names out of text anywhere.

## Design
- Chosen, the stored shape: two unit predicates in the closed assertion vocabulary, actor and counterpart, each carried by an assertion on one of the memory's notion subjects, and one optional due instant on the interpreted memory. All three are valid only on an open loop or a commitment.
  - Structure:
    - `BeliefPredicate` gains two variants.
    - `validate_belief` gains the derived type, the subtype rule and the no-both-roles rule.
    - The graph mapping stores the predicate literal on the existing assertion node exactly as `known_as` is stored.
    - Hydration reads the name literal only for `known_as`; today it reads one for every node.
    - The due instant is one UTC instant literal.
    - The drafts and the replacement draft carry the due instant and already carry assertions.
    - Replay equality holds because both are fields of the persisted object.
  - Evolution: v0.3 writes these from reflection. v0.4's validity intervals may generalize the due instant.
  - Verification: service-free on the embedded stores through the public facade, each task showing its case failing at its parent commit.
  - Operation: one bounded read for each distinct resolved notion, plus one read for due. Each obligation root adds one depth-1 expansion. There is no model call and no clock read.
  - Human: the application says who owes whom and by when, as the character's own commitment about the matter (ADR-D-0035).
  - Safety: nothing is withheld; both roads only add candidates.
- Alternative: two fields on the interpreted memory. Rejected: this duplicates the subject-consistency rule and needs a new graph property.
- Alternative: one predicate with a payload. Rejected: the party would sometimes be the payload.
- Alternative: parties as memory links. Rejected by ADR-D-0035: a link carries no evidence and cannot be superseded.
- Chosen, the self: a required constructor argument on both public constructors. The composition holds it and hands it to the retrieval pipeline.
  - At open, `construct` opens the graph store and reads its identity literal before opening the vector store; today the vector store is opened first. An absent literal is written; an equal one passes; a different one fails construction with one plain error naming both ids.
  - In retrieval the identity is read exactly once, in `push_derived`, for the direction report. No selector, expansion rule, floor, score or section rule consults it.
- Alternative: the trigger road skips the self (this plan's earlier design). Rejected by ADR-D-0020's Decision text ("no retrieval path treats the self as a special role", ruling 76). The worry behind it, a listed self bringing its own obligations on every scene, is answered by not listing the self, which the README says.
- Alternative: the self from configuration, optional, unchecked, or stored as a notion the store must hold. Rejected as before: it is an identity; the record says it is supplied; the failure is silent; and an empty store must open.
- Chosen, the trigger road: one read for each scene-resolved notion.
  - The read goes through `select_state` with a scope predicate matching an open loop or commitment that carries an assertion whose predicate is actor or counterpart and whose subject is that notion. It uses salience-first order and no as-of cut.
  - The selector's lifecycle and resolution filtering, ranking in Rust and omission reporting are reused unchanged.
  - Results become explicit roots through `CandidateRoot::from_rank` at score zero, on the road `RecallRoad::Trigger`. Its row: kind `CueKind::Trigger`, reported `AdmissionRoad::Participant`, source `GraphRootSource::Trigger`, expands one hop, reserves, contribution `RootCap`, time rank.
  - Each party is one scope of the trigger kind in `scope_kinds`.
  - `CueKind::Trigger`, `RetrievalCueFloors.trigger` (default 1, provisional), `cue_floor` and the helper's fixed kind list (after activity) gain the kind.
- Chosen, one hop:
  - `RoadRule.expands` becomes a three-valued `Expansion { Opens, OneHop, Leaf }`. Every current row keeps its meaning: `true` becomes `Opens` and `false` becomes `Leaf`.
  - Spare turns and the propagation of score and kind to descendants read `Opens`.
  - `graph_query_for_candidate` gives a root whose best row is `OneHop` a `max_depth` of 1, without the leaf's occasion restriction, and ranks what it reaches as reminder-only, with `max_depth` set to the smaller of the caller's limit and 1, so a caller's `max_depth` of zero still means no expansion.
  - Reach is decided per expansion over the already-admitted graph, not by one shared reminder set: only `Opens` carries score and opens history; `OneHop` carries its own admission road for its first hop only; `Leaf` keeps exactly its reach today. A root reached by several rows keeps each row's reach, so a Topic and Due root's direct source episode reports both, and a Place row on the same root does not stamp Place onto what the Due hop reaches. This lives in `absorb_expansion` (owned `retrieve.rs`), with no new store read.
  - A root that also has an `Opens` row expands as that row does.
- Alternative: leave obligations to the participant road, which already brings the present person's aboutness list within one bounded budget (ruling 70). Rejected unless the baseline shows otherwise: a low-salience obligation behind many salient beliefs is cut by that budget, and nothing reserves it under a loud topic. Task_2 records the baseline first.
- Alternative: the trigger and due rows as pure leaves (ruling 75 as first written). Rejected by ADR-D-0023 (ruling 76): the record's one bounded hop is accepted.
- Alternative: the trigger and due rows open history at score one with spare turns (this plan's first design). Rejected by rulings 59 and 75: the obligation would get a turn equal to a person's and lift what it reached.
- Alternative: the trigger fires only for obligations between the self and the present notion. Rejected: a person seeing Bob carries what concerns Bob; the direction then reports `None`.
- Alternative: a new `AdmissionRoad::Trigger`. Rejected: connected to someone present is `Participant`'s meaning.
- Chosen, the due road: one read per retrieval.
  - The read goes through `select_state` with a scope predicate matching an open loop or commitment whose due literal, cast as `xsd:dateTime`, is before a bound instant. The bound instant is the first instant of the day after the scene's local day, computed in Rust from `scene.time` and converted to UTC. Order is salience first.
  - Results become explicit roots at score zero on `RecallRoad::Due`. Its row: kind `CueKind::Due`, reported `AdmissionRoad::Due`, source `GraphRootSource::Due`, expands one hop, reserves, contribution `Room`, time rank.
  - `CueKind::Due`, `RetrievalCueFloors.due` (default 1, provisional), `cue_floor` and the helper's fixed list (after trigger) gain the kind.
  - No state scope is needed: it is one road with one order.
- Alternative: due by the instant alone, due as a zero-floor time kind, or a window after which overdue stops. Rejected as before: a promise due at five is carried all day; D8 needs a floor under a loud topic; nobody measured such a window, and resolution is the exit (ADR-D-0018).
- Chosen, the report: two optional values added to `IncludedDerivedMemory` beside `resolved_by`, computed in `push_derived` with no second read. `AdmissionRoad::Due` comes from the table's reported column.
- Measured effects (REV-R1): a direct root's own graph component becomes one, so an obligation also reached as a descendant rises; a reserving root can displace an expanding root and its paths. Both are recorded as before and after cases in Task_2 and measured by F7 to F9. No scoring change is made.
- Measurement. It runs once, at slice completion (orchestrator rule, Measurement Of Behavior Changes). If a family regresses, the cause is found by re-measuring at the Task_1 and Task_2 tips, which exist as commits.
  - Instrument: the companion evaluation repository's generated runner. It uses the existing families (scene overlap, keyless, familiar person, time, shared interpretation, keyed setting, activity pressure) plus one new family, **obligations**, built before Task_1 is dispatched so before-numbers exist at the base. The base cannot write roles or due instants, so its before-run records the same stories without them. The family holds:
    - one keyed person with obligations in both directions, some resolved by a later memory;
    - due instants before, on and after the reference day at a non-UTC offset;
    - an unresolved obligation naming the person present that is due tomorrow;
    - the character's own undated promise with the self as actor and no other party, whose topic some retrievals ask;
    - a person with more high-salience beliefs than the aboutness budget, beside one low-salience open loop naming them;
    - two people present with several obligations each;
    - with superseded included, a high-salience superseded obligation beside a low-salience current one naming the same person;
    - unrelated loud-topic pressure that fills every cap;
    - a year-deep keyless daily store that holds several overdue obligations, retrieved with nothing said;
    - identifiers opposed to time.

    The actual caps, the enabled floors and every admission path are recorded with the numbers. A floor is a reservation, not unlimited capacity.
  - Each run compares before (the base) with after (the slice tip) on the same inputs, in both identifier orders, run twice. Any one of the following falsifies the design:
    - F1: in any loud-topic retrieval, the first-listed present person's most salient unresolved obligation is absent. That is what one trigger floor guarantees. For every other person present, the count of their obligations is reported per retrieval, as a reading and not a falsifier.
    - F2: with the trigger floor set to two and one person present, whose two most salient unresolved obligations run one in each direction, either of the two is absent under the loud topic.
    - F3: the most salient unresolved due or overdue obligation is absent from any retrieval, with nothing said or under the loud topic.
    - F4: an obligation due after the scene's local day is contributed by Due; a resolved or fulfilled obligation is contributed by Trigger or Due; a resolved obligation admitted by any road lacks `resolved_by`; or the control fails. The control: with Bob present, an unresolved obligation naming him that is due tomorrow must come by Trigger and report `NotYetDue`.
    - F5: the character's own undated promise is admitted on every retrieval of the family.
    - F6: the character's own undated promise is absent from a retrieval that asks its topic.
    - F7: on-topic memories kept under the loud topic fall more than two below the topic-alone count.
    - F8: within the obligations family, the present person's non-obligation state memories fall by more than one.
    - F9: with nothing said in the family's keyless daily store holding overdue obligations, the latest occasions of equal salience fall by more than one.
    - F10: any existing family, which holds no roles and no due instants, differs from its base numbers in ids, sections, order, scores or `admitted_by`.

    F7 to F9 are empirical thresholds, not bounds the floors prove.
  - The validator has no measurement kind, so the item is `kind: command`, `owner: orchestrator`, naming the evals worker as the runner.
- Why chosen: this is the smallest shape that lets an obligation say who owes whom and by when. It reuses the state selector, its resolution filter, the road table, the root key and rounds per kind. It keeps ADR-D-0023's one bounded hop and ADR-D-0020's self with no special role, and adds the two new ways of coming to mind as table rows, not exceptions. Fit: the phase draft sections 2 and 2.2; philosophy "what is owed in either direction, what fell due today" and "purpose surfaces from memory"; ADR-D-0018, D-0020, D-0022, D-0023, D-0029, D-0035, D-0038; rulings 12, 26, 27, 32, 39, 42, 46, 49, 50, 54, 59, 61, 65, 67, 70 to 76.

## Compatibility stance (required if a contract/interface/persisted format is touched)
- surface:
  - Both public constructors take the self's notion id, and there is one error at open for an identity mismatch.
  - The graph store holds one identity literal.
  - `BeliefPredicate` gains `Actor` and `Counterpart`.
  - `DerivedMemory`, its draft and the replacement draft gain `due_at`.
  - `BeliefValidationError` gains two variants.
  - `CueKind` gains `Trigger` and `Due`.
  - `RetrievalCueFloors` gains `trigger` and `due`.
  - `GraphRootSource` gains `Trigger` and `Due`.
  - `AdmissionRoad` gains `Due`.
  - `IncludedDerivedMemory` gains `direction` and `due_state`, with the new enums `ObligationDirection` and `DueState`.
  - The graph port gains two selector entry points and the identity read-or-write.
  - The stored shape gains two predicate literals and one due literal.
  - The crate-internal road table's expands column becomes three-valued.
- stance: break
- justification: no external consumers. The one locatable consumer is `CharacterMemoryEvals`, the public companion evaluation repository, which follows each library slice. No stored-data migration is needed: memories written before this plan carry no roles and no due instant, report `None` for both, and are never triggered or due.

## Context (workspace)
- Related files and areas, at the consolidation's final tip ef8dbfb (source identical to 97dd7c2, pinned at `C:/w/cm-rev8`):
  - `src/domain/belief.rs`: `BeliefPredicate` at 19, `validate_belief` at 36.
  - `src/domain.rs`: `DerivedType::Commitment` and `OpenLoop` at 151; `DerivedMemory` at 395, with `entity_ids` at 404 and `assertions` at 408.
  - `src/api/types/draft.rs`: `DerivedMemoryDraft` at 330.
  - `src/api/types/lifecycle.rs`: `ReplacementDerivedMemoryDraft` at 198; the second `validate_belief` caller at 266.
  - `src/composition.rs`: `new_with_embedding_provider` at 132; `construct` opens the vector store before the graph store; `new` at 212.
  - `src/memory.rs`: `retrieve` at 125.
  - `src/usecases/retrieve.rs`:
    - explicit roots from 96, including the place read at 122-156, the pattern for a `select_state` road through `from_rank`;
    - `absorb_expansion`, with candidate roots at proximity zero at 582 and the root merge taking the larger graph component at 629-635 (REV-R1);
    - `final_score` at 794;
    - `select_with_cue_floors` at 854, with its fixed kind list at 877;
    - `push_derived` at 1180;
    - `RecallRoad` at 1201 and `RecallRoad::rule` at 1233;
    - `cue_floor` at 1355;
    - `CandidateRoot::from_rank` at 1400;
    - `select_candidate_roots` at 1515;
    - `graph_query_for_candidate` at 1605.
  - `src/usecases/retrieve/state.rs`: `scopes_for_kind` at 7, `order_state_per_kind` at 61.
  - `src/usecases/retrieve/scene.rs`: resolved participants at 45-96.
  - `src/ports/graph_authority.rs`: `GraphMemoryRank` at 16 (id, time and salience only); `GraphExpansionQuery` with `max_depth` and `reminder_only` at 152-173; the leaf restrictions at 213-240; `query_scope_state` at 448.
  - `src/adapters/oxigraph/sparql_selectors.rs`: `State::rank` at 35, which drops supersession; `select_notions_known_as` at 129; `select_scope_state` at 319; `select_state` at 368, with its resolution filter at 466 and its current-first ranking in Rust at 489-499.
  - `src/adapters/oxigraph/rdf_mapping.rs`: assertion nodes at 373.
  - `src/adapters/oxigraph/shared.rs`: `belief_assertions_from_rdf` at 443, which reads a name for every node at 458.
  - `src/api/types/retrieval.rs`: `RetrievalCueFloors` at 128, `MemoryScenes` at 260, `AdmissionRoad` at 277, `GraphRootSource` at 379, `IncludedDerivedMemory` at 418, `CueKind` at 815.
  - `README.md` at ef8dbfb: the `AdmissionRoad` table at 57-67, the `CueKind` list at 75, the road table and principles at 77-96.
- Rulings: the rulings log `.agent-work/orchestrator/v0-2-load-bearing-decisions.md`, items 12, 26, 27, 32, 39, 42, 46, 49, 50, 54, and 58 to 76.
- Tier D report: `.agent-work/reviewer/v0-2-prospective-plan-review.md` (REV-R1 to REV-R3, 2026-09-23).
- Existing patterns and references:
  - A `known_as` assertion is the pattern for a predicate with a mechanical reader.
  - The place read is the pattern for a `select_state` road entering at score zero with a time rank.
  - Participant scopes are the pattern for rounds among the scopes of one kind.
  - `resolved_by` is the pattern for a computed value on the included entry.
- Design records consulted, and deviations from their acceptance: ADR-D-0018, D-0020, D-0022, D-0023, D-0024, D-0029, D-0035, D-0038. There is no deviation. ADR-D-0020 is satisfied: the self is supplied at construction, never a different one opens the store, and no retrieval path treats it as a special role. ADR-D-0023 is satisfied: surfaced state re-cues one bounded hop. ADR-D-0035's rule for new predicates is followed.

## Integration
- The library stack: the cues, fixes, state, scene-words and time slices, the phase correctness fixes, the write-path warnings, the consolidation slice (code at f6926ef, records at ef8dbfb), then this plan, then the renderer plan, which reads this plan's fields.
- Base: the consolidation's final tip, `feature/2026-09-23/consolidation-records` at ef8dbfb, pinned in the dispatch brief. Task_1 is cut from it after the consolidation's slice-end measurement is recorded.
- Inherited and not redone here: the road table and the five principles; the root key; rounds per kind; `select_state` ranking in Rust; the resolution filter and `resolved_by`; `AdmissionRoad` and `admitted_by`; the leaf and reminder merge rules (rulings 46, 47, 49, 54); the as-of cut; the UTC instant literal with the given offset stored beside it; the write-path warning for a resolver that shares no subject with what it resolves. This plan adds:
  - two predicates and one field;
  - two table rows, one expands value, two kinds, two floors, two root sources and one admission value;
  - two selector entry points and two reported values.
- Controls that must not change under this plan in a store with no roles and no due instants: the consolidation's parity fixtures and every existing retrieval test. Every test that constructs the facade gets the mechanical constructor update and no change of expectation. Every other changed expectation is listed with before and after from runs.

## Open Questions (max 3)
- For the decider at the slice boundary, not blocking: ADR-D-0024 names a pair scope. This plan delivers "what is between us" as the direction and the parties on each admitted obligation, with the trigger firing for the other party present, and adds no pair object or key. Since the consolidation ruled that ADR-D-0024 is not replaced, the proposal is to record this reading here and draft no record unless you ask for one.
- For the decider, stated plainly and not blocking: with nothing said and the self not listed, the character's own undated promise does not come by any obligation road. It comes when its topic is asked (F6 holds this), when its place key is given, when it falls due if it has a due instant, or when a recent occasion it rests on comes by recency. That is ADR-D-0020's consequence (ruling 76); a self trigger was ruled out.

- For the decider, stated plainly and not blocking: ADR-D-0023's example says an open loop pulls the counterpart's objections. The one hop brings what is linked to the obligation itself; a counterpart's view that is not linked to it comes only if another road brings it. The record leaves which state kinds re-cue, and how, open, so this is within it; say so if the example should be taken literally.

## Assumptions
- A1: `select_state` accepts, as its scope predicate, both a pattern joining the assertion node by predicate literal and subject and one filtering the due literal by a typed cast against a bound instant, without a change to its filtering or ordering. Source: the selector at ef8dbfb interpolates the predicate inside the memory's graph pattern, as Tier D confirmed. Checked by Task_2 and Task_3, each carrying a stop condition.
- A2: A root query with `max_depth` 1 and without the leaf's occasion restriction reaches the obligation's direct links (its source occasions, its subjects, and memories linked to it) within the existing fanout bounds. What it reaches can be ranked reminder-only by the existing merge. Source: `GraphExpansionQuery` and `absorb_expansion` at ef8dbfb. Checked by Task_2.
- A3: The helper's fixed kind list, `cue_floor` and `RecallRoad::rule` take two more entries, and `expands` takes a third value, with no change of selection for the rows they serve today. Source: consolidation Task_1. Checked by Task_2 and Task_3 end to end.
- A4: Every admitted open loop and commitment passes through `push_derived`. The self and the scene time must be plumbed into it; the task owns that file. Source: `src/usecases/retrieve.rs:1180`. Checked by Task_1.
- A5: The companion repository constructs the facade through one helper, so the constructor change is mechanical there. A note for the handoff, not checked here.
- A6: The obligations family exists in the evaluation repository, with before-numbers at the base, before Task_1 is dispatched. A precondition of dispatch.

## Tasks

### Task_1: An obligation says who owes whom, and the character knows which side it is on
- type: impl
- effort: 8 worker-hours
- owns:
  - src/domain/belief.rs
  - src/domain.rs
  - src/domain/tests.rs
  - src/api/types/lifecycle.rs
  - src/api/types/retrieval.rs
  - src/api/types.rs
  - src/lib.rs
  - src/composition.rs
  - src/memory.rs
  - src/errors.rs
  - src/usecases/retrieve.rs
  - src/ports/graph_authority.rs
  - src/adapters/oxigraph/vocabulary.rs
  - src/adapters/oxigraph/rdf_mapping.rs
  - src/adapters/oxigraph/shared.rs
  - src/adapters/oxigraph/embedded.rs
  - src/adapters/oxigraph/tests.rs
  - src/test_support.rs
  - tests/support/
  - tests/retrieval_obligation_tests.rs
  - tests/public_facade_tests.rs
  - README.md
  - docs/design/database/graph_schema_design.md
  - docs/design/database/schema_cheat_sheet.md
  - any other file only for the mechanical update where the facade or the retrieval pipeline is constructed, or where a test `GraphAuthorityStore` impl gains the identity method, with no change of behavior or expectation
- depends_on: []
- description: |
  Base: the consolidation's final tip (see Integration).

  First, at the parent commit, through the public facade on the real embedded stores, record:
  - Given two notions, the character and Bob; a commitment "I will bring Bob the book" with both as subjects; and an open loop "Bob said he would send the draft" with both as subjects, a retrieval with Bob present and no topic. Nothing in the result can say who owes whom, and nothing can tell the library which notion is the character.
  - That a memory with an assertion predicate outside the vocabulary is rejected today.

  Then:
  - `BeliefPredicate` gains `Actor` and `Counterpart`. A memory may carry several of either.
  - `validate_belief` takes the derived type, and both callers pass it. Each assertion names a subject of its memory (the existing rule), no subject carries both roles, and roles appear only on an open loop or a commitment. There is one error variant for both roles on one subject, and one for a role on another kind of memory, which Task_3 also uses for a due instant.
  - The graph mapping stores the predicate literal on the assertion node. Hydration reads the name literal only for `known_as`. A replay is byte-identical.
  - Both public constructors take the character's notion id. The composition holds it and hands it to the retrieval pipeline. The test constructors and every helper that builds the facade take it too.
  - In `construct`, the graph store is opened first. Its identity literal is written if absent, accepted if equal, and refused with one plain error naming both ids if different. Only then is the vector store opened. The literal is not a notion, and no selector reads it.
  - In retrieval, the identity is read in exactly one place: `push_derived` sets `direction: Option<ObligationDirection>` on every admitted open loop or commitment (`OwedByCharacter` when the self is among the actors, `OwedToCharacter` when among the counterparts, `None` otherwise).
  - No selector, expansion rule, floor, score or section rule changes.
  - The README states the rule: an obligation names its parties among its subjects, and several are allowed. The character is told which notion is itself at construction, and a store refuses another self. The character perceives the scene and is not listed as a participant; listed anyway, it is treated as any notion. The identity's effects are only the direction report and the store check. The schema documents describe the two predicate literals and the identity literal.
- acceptance:
  - With the trace off:
    - the commitment reports `OwedByCharacter`;
    - the open loop reports `OwedToCharacter`;
    - a promise to Bob and Alice (two counterparts) reports `OwedByCharacter`;
    - a matter between Bob and Alice with the character as neither reports `None`;
    - an obligation with no roles reports `None`.
  - Each of the following is rejected at the write with the named error, through the facade and through the replacement draft of a correction: a role on a subject the memory is not about, both roles on one subject, and a role on a claim. The identical write replayed after reopening the store is accepted.
  - Opening a fresh store writes the identity literal, and reopening with the same id succeeds. Reopening with a different id fails with the plain error naming both ids, writes nothing, and touches no vector collection. The store still opens with the original id afterwards.
  - A retrieval on these fixtures selects what it selects at the parent (this task adds no road), `admitted_by` included, with and without the self listed as a participant. The report shows both runs.
  - Every existing test passes with only the constructor update. The report lists the files touched for it and confirms no expectation changed.
  - The report holds the baseline run and lists every reader of the identity.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test; the baseline run at the parent commit, recorded in the report"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review, tracing every reader of the self's identity (the direction report and the store check only), the open order in construct, and the assertion validation, hydration and replay; Tier A altitude review against the philosophy, ADR-D-0020 (no special role for the self; never a different self) and ADR-D-0035 (a predicate enters with its reader)"

### Task_2: Seeing someone brings what stands open between you, however much else you know about them
- type: impl
- effort: 7 worker-hours
- owns:
  - src/usecases/retrieve.rs
  - src/usecases/retrieve/state.rs
  - src/api/types/retrieval.rs
  - src/ports/graph_authority.rs
  - src/adapters/oxigraph/sparql_selectors.rs
  - src/adapters/oxigraph/embedded.rs
  - src/adapters/oxigraph/tests.rs
  - src/memory/retrieval_floor_tests.rs
  - tests/retrieval_obligation_tests.rs
  - tests/public_facade_tests.rs
  - README.md
  - the test GraphAuthorityStore impls, updated mechanically for the new selector only
- depends_on: [Task_1]
- description: |
  First, at the parent commit: in a store where Bob has more current high-salience beliefs than the aboutness budget admits, plus one low-salience open loop with Bob as its counterpart, retrieve with Bob present by key and no topic, by a name Bob is known by, and under a topic that fills every cap. Record whether the open loop comes in each case and with which `admitted_by`.

  The task builds only what fails. If the open loop comes in every case, the task delivers no code. The report says so, this plan's Decision Log records that the trigger row is not added, and Task_3 adds the one-hop expands value itself.

  Otherwise, as the Design states:
  - `RoadRule.expands` becomes `Expansion { Opens, OneHop, Leaf }`, with every existing row unchanged in meaning. Spare turns and descendant propagation read `Opens`. `graph_query_for_candidate` gives a `OneHop` root depth 1 without the leaf's occasion restriction and ranks what it reaches reminder-only, unless an `Opens` row also reached the root.
  - Add `RecallRoad::Trigger` after `Activity`. Its `rule`: kind `CueKind::Trigger`, reported `AdmissionRoad::Participant`, source `GraphRootSource::Trigger`, `OneHop`, reserves, `Contribution::RootCap`, time rank.
  - Add `CueKind::Trigger` and `RetrievalCueFloors.trigger`, default 1, documented as provisional. Add the kind to `cue_floor` and to the helper's fixed kind list after activity.
  - Add one graph-port selector per party, through `select_state`, with the assertion pattern, salience first and no as-of cut. If the selector cannot take the pattern without a change to its rule for the scopes it already serves, stop and report.
  - Make one call per scene-resolved notion, in scene order, entering through `from_rank` at score zero, with each notion one scope of the trigger kind.
  - A description triggers nothing. An ambiguous name triggers for each notion it could mean. The self is not skipped.
  - Measure the new read at a store of 2000 obligations beside the existing subject-state read at the same scale. If it is slower, reshape it per ruling 67 before review.
  - The README road table gains the row and the one-hop value, principle 5's line gains one hop (ADR-D-0023), the `Participant` admission row says it also covers an unsettled obligation naming someone present and what that obligation's hop reaches, and the `CueKind` list gains `Trigger`.
- acceptance:
  - In the baseline store, the low-salience open loop comes with Bob present by key and by name, and under the topic that fills every cap, and reports `admitted_by` containing `Participant`. The trace names the trigger kind for it and shows its depth-1 expansion. With Bob absent it does not come by trigger. With Bob given only by description, nothing is triggered.
  - One hop: a triggered obligation brings its source occasion and a memory linked to it directly, each reporting `Participant` and carrying no cue score. It brings no memory two hops away that the participant road did not also reach. It takes no spare turn at root selection. A caller's `max_depth` of zero, one and deeper each behave as stated (zero expands nothing). A root reached by Topic and Trigger reports both on its direct source episode; with Place also on it, Place is not reported on what only the Trigger hop reached.
  - Measured effects, each shown before and after with scores:
    - An obligation reached at the parent as Bob's participant descendant (cue 0.75, proximity 1) becomes a direct root. With salience 0.5, its final score goes from 0.6625 to 0.7875 and its cue component is unchanged.
    - An obligation that is also a direct topic root keeps its parent score.
    - At a root cap that holds every root at the parent, the trigger root takes a seat. The report names the displaced root and every memory only its paths brought, with its before and after scores.
  - Two people are present, each a party to several obligations. Under a trigger floor of two and a tight cap, each person's most salient obligation holds a reserved slot before either brings a second. The trigger road's order within one person's obligations is salience, then newest. That is the order the floor serves; admitted obligations are then ordered by score.
  - An obligation Bob is the actor of and one he is the counterpart of both come. An obligation between Bob and Alice, with the character as neither, comes with Bob present and reports `direction` `None`.
  - Bob is present, and an unresolved obligation naming him is due tomorrow. It comes by trigger (`admitted_by` containing `Participant`). Its due state is added by Task_3.
  - An obligation resolved by a later memory is not contributed by trigger and is reported as left out by resolution. A matching topic still admits it, marked `resolved_by`, as at the parent.
  - With superseded included, a high-salience superseded obligation H and a low-salience current one C both name Bob. With a trigger floor of one and room for one trigger root, C holds the floor. With ample room both come, and H orders before C by score. The report states that current-before-superseded is a floor guarantee only. With superseded excluded, H is not contributed.
  - With the self listed as present beside Bob (which the README advises against), obligations naming the self come by trigger exactly as for any notion.
  - A retrieval on a store with no roles selects what it selects at the parent, `admitted_by` included.
  - The report holds the baseline run for each case, saying which failed there, plus the scaled read timing. A case that already passed is not claimed as delivered.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test; the acceptance cases run at the parent commit, with what each brought recorded in the report; the trigger read timed at 2000 obligations beside the subject-state read"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review, reproducing the baseline evidence at the parent commit and tracing the table row, the Expansion value's readers (turns, propagation, depth), the selector reuse, the scope registration, the helper's kind list and the measured-effect cases; Tier A altitude review against the philosophy, principles 1, 2, 4 and 5, ADR-D-0020 (no self skip), ADR-D-0023 (one bounded hop), and rulings 26, 50, 59, 70, 75 and 76"

### Task_3: A promise falls due whatever the conversation is about
- type: impl
- effort: 8 worker-hours
- owns:
  - src/domain.rs
  - src/domain/belief.rs
  - src/domain/tests.rs
  - src/api/types/draft.rs
  - src/api/types/lifecycle.rs
  - src/api/types/retrieval.rs
  - src/api/types.rs
  - src/lib.rs
  - src/usecases/correct_forget.rs
  - src/usecases/retrieve.rs
  - src/ports/graph_authority.rs
  - src/adapters/oxigraph/vocabulary.rs
  - src/adapters/oxigraph/rdf_mapping.rs
  - src/adapters/oxigraph/shared.rs
  - src/adapters/oxigraph/sparql_selectors.rs
  - src/adapters/oxigraph/embedded.rs
  - src/adapters/oxigraph/tests.rs
  - src/memory/retrieval_floor_tests.rs
  - tests/retrieval_obligation_tests.rs
  - tests/public_facade_tests.rs
  - tests/write_planning_tests.rs
  - README.md
  - docs/design/database/graph_schema_design.md
  - docs/design/database/schema_cheat_sheet.md
  - the fixtures that construct a DerivedMemory directly, and the test GraphAuthorityStore impls, for the mechanical `due_at: None` and new selector only (test code only)
- depends_on: [Task_2]
- description: |
  First, at the parent commit: take a commitment the character made a month ago, due yesterday, resolved by nothing, about a matter whose words share nothing with the conversation. Retrieve with a scene giving only the time, then under a topic that fills every cap. Record that nothing brings it; since no due instant exists at the parent, the baseline is the same fixture without one.

  Then:
  - `DerivedMemory`, its draft and the replacement draft of a correction gain `due_at: Option<DateTime<Utc>>`, given as an instant and never parsed. The domain validation rejects it on any memory that is not an open loop or a commitment, with Task_1's variant. A correction's replacement carries the instant it is given, not its predecessor's.
  - The graph mapping stores it as one UTC instant literal, and hydration reads it back. A same-id replay with another instant is a content difference, rejected as any collision is.
  - If Task_2 delivered no code, add the one-hop expands value as Task_2 describes it.
  - Add `RecallRoad::Due` after `Trigger`. Its `rule`: kind `CueKind::Due`, reported `AdmissionRoad::Due`, source `GraphRootSource::Due`, `OneHop`, reserves, `Contribution::Room`, time rank.
  - Add `CueKind::Due`, `RetrievalCueFloors.due` (default 1, provisional), the kind in `cue_floor` and in the helper's fixed list after trigger, and `AdmissionRoad::Due`.
  - Add one graph-port selector through `select_state`, with a scope predicate on open loops and commitments whose due literal, cast as `xsd:dateTime`, is before a bound instant. The bound instant is the first instant of the day after `scene.time`'s local date at its own offset, converted to UTC in Rust. Order is salience first, with no as-of cut. Nothing reads a clock. If the selector cannot take the pattern without a change to its rule, stop and report.
  - Make one call per retrieval, entering through `from_rank` at score zero, with no state scope.
  - `push_derived` sets `due_state: Option<DueState>` beside the direction: `Overdue` before the scene's local day, `DueToday` within it, `NotYetDue` after it, `None` without an instant. A not-yet-due obligation is never contributed by the due road, and it reports `NotYetDue` when another road admits it.
  - Measure the new read at 2000 obligations beside the subject-state read, and reshape it per ruling 67 if it is slower.
  - The README road table gains the row. The admission table gains `Due`: an unsettled obligation's due instant fell before the end of the scene's local day, or the memory was reached in one hop from such an obligation. The `CueKind` list gains `Due`. The README states the rule: due is judged by the scene's local day at the offset the application gave; a scene time the library fixes itself is at UTC; an all-day due is given as the start of that local day, as an instant; an overdue promise keeps coming until it is resolved or superseded; and nothing is removed by time.
  - The schema documents describe the stored literal.
- acceptance:
  - Mixed roots with Due: a root reached by Topic and Due reports both on its direct source episode; with Place also on it, Place is not reported on what only the Due hop reached. If Task_2 delivered no code, this task introduces the one-hop value with the caller-limit controls (max_depth zero, one and deeper).
  - The commitment due yesterday comes with a scene giving only the time, and under the topic that fills every cap. It reports `admitted_by` containing `Due` and `due_state` `Overdue`. The trace names the due kind and shows its depth-1 expansion, and it takes no spare turn at root selection. Its source occasion comes through the hop, reporting `Due`. No other memory about its counterpart comes through it.
  - Retrieved with a reference time two days before it fell due, it does not come by due. When its topic admits it, it reports `NotYetDue`.
  - Bob is present, and an unresolved obligation naming him is due tomorrow. It comes by trigger, reports `NotYetDue`, and is not contributed by due.
  - A promise due at five in the evening comes at eight in the morning of that local day and reports `DueToday`. At eight the evening before, at the same offset, it does not come by due.
  - An instant at half past midnight at +14:00, on the same calendar date as a scene at -10:00, falls on the previous UTC day, before the scene's local day begins, and reports `Overdue`.
  - A scene at the library's own UTC time judges by the UTC day. The report shows the parent's reading of the same inputs.
  - After a later memory resolves the commitment, it is not contributed by due and is reported as left out by resolution. A matching topic still admits it, marked `resolved_by` and reporting its due state. Suppressing the resolver does not bring it back by due.
  - Three overdue obligations, room for one, under a due floor of one: the most salient comes by its floor, and the trace says so. With spare root room and nothing said, all three come.
  - With superseded included, a high-salience superseded overdue commitment H and a low-salience current one C, with equal creation times: a due floor of one with room for one reserves C. With ample room both come, H before C by score. The report states that current-before-superseded is a floor guarantee only.
  - A due instant on a claim is rejected at the write. A same-id replay with a different due instant is rejected as a collision. The identical replay after reopening is accepted. A correction's replacement carries the instant it is given.
  - A retrieval on a store with no due instants selects what it selects at the parent, `admitted_by` included. The consolidation's parity fixtures pass unchanged.
  - The values are in the result with the trace off.
  - The report holds the baseline runs, says which failed, and holds the scaled read timing.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test; the baseline cases run at the parent commit and recorded in the report; the due read timed at 2000 obligations beside the subject-state read"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review, tracing every reader and writer of the due instant, the local-day boundary computed in Rust from scene.time, the collision check, the table row, the one-hop depth and the helper's kind list; Tier A altitude review against the philosophy, ADR-D-0018 (time never removes recall; resolution is the only exit), ADR-D-0023 (a perceived fact, never a purpose; one bounded hop), principles 1, 2 and 5, and rulings 39, 50, 71, 73 and 76"
  - kind: command
    required: true
    owner: orchestrator
    detail: "Slice-end measurement, run by the evals worker at this task's tip, on the new obligations family plus the scene overlap, keyless, familiar person, time, shared interpretation, keyed setting and activity pressure families; before at the base and after, identifiers opposed to time, both orders, twice; recording caps, enabled floors and admission paths; only on a regression, re-measure at the Task_1 and Task_2 tips to find the cause. Falsified if: F1 in any loud-topic retrieval the first-listed present person's most salient unresolved obligation is absent (other persons' counts reported per retrieval); F2 with the trigger floor at two and one person present whose two most salient obligations run one each way, either is absent under the loud topic; F3 the most salient unresolved due or overdue obligation is absent with nothing said or under the loud topic; F4 an obligation due after the scene's local day is contributed by Due, a resolved or fulfilled one by Trigger or Due, a resolved one admitted by any road lacks resolved_by, or the control (Bob present, an unresolved obligation naming him due tomorrow) does not come by Trigger reporting NotYetDue; F5 the character's own undated promise is admitted on every retrieval; F6 it is absent when its topic is asked; F7 on-topic memories kept under the loud topic fall more than two below topic-alone; F8 within the obligations family the present person's non-obligation state memories fall by more than one; F9 with nothing said in the family's keyless daily store holding overdue obligations the latest occasions of equal salience fall by more than one; F10 any existing family differs from its base numbers in ids, sections, order, scores or admitted_by. F7 to F9 are empirical thresholds, not bounds the floors prove. The numbers, their reading against the philosophy and a verdict go into this plan's Decision Log"

## Task Waves (explicit parallel dispatch sets)

- Wave 1: [Task_1]
- Wave 2: [Task_2]
- Wave 3: [Task_3]

The tasks share one crate and its files, so they run in sequence, one worker at a time, each pull request stacked on the previous one; the first is stacked on the consolidation's final tip, as Integration states. The evaluation repository builds the obligations family before Task_1 is dispatched. After the slice, the library commit is handed to the companion repository, whose own plan schedules the following:
- D4: obligations in both directions on meeting a person;
- D7: the intention surfacing on its counterpart;
- D1 and D8: the due and overdue promise with no topic and under a loud topic;
- the trigger and due floors in the calibration;
- the constructor update in its test support.

## Rollback / Safety
- Each task is one pull request and reverts on its own, in reverse order. Once the companion repository has followed a slice, a revert is coordinated with it. Task_1 adds two predicate literals and one identity literal, and Task_3 adds one due literal. Reverting leaves values nothing reads. A store holding roles hydrates only while the predicates are known, so a revert of Task_1 is coordinated with any store that holds roles. The companion regenerates its stores on every run.

## Progress Log (append-only)

- (none yet)

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-22 Decision: prospective memory is direction as two assertion predicates, the self at construction with two readers, a due instant, and two given-cue routes through the state selector.
  - Trigger / new insight: the rulings deferred the self's identity and the pair counterpart to this plan. The routes census fixed a closed cue vocabulary that already names trigger and due. The state slice landed a selector that filters resolution and reports omissions, and the time slice gives the scene a local day. The phase draft asks only that an intention surface on its counterpart or its topic, and a promise on its due date.
  - Plan delta (what changed): new plan.
    - Actor and counterpart enter the closed assertion vocabulary, so subject consistency is the existing rule.
    - The self is a required constructor argument, read by the direction report and by the trigger route to skip itself, and nowhere else.
    - The due instant is a field on the interpreted memory.
    - Trigger and due are ruled cue kinds with provisional floors, full standing and spare turns, contributed through the state selector with an assertion pattern and a due comparison.
    - Due is judged by the scene's local day.
    - The report is two optional values on the included entry.
    - There is no pair object, no Preference subtype, no cadence, no topic trigger beyond the content route, no resolve method and no window on overdue.
  - Tradeoffs considered:
    - Fields for the parties were rejected as a second copy of the subject rule.
    - A payload predicate was rejected because the party would sometimes be the payload.
    - The pair reading of the trigger was rejected because a person carries what concerns the one in front of them.
    - Due as a zero-floor time kind was rejected because a due obligation is not on every scene and D8 needs it under a loud topic.
    - Due by the instant alone was rejected because a promise due at five is carried all day.
    - A most-overdue-first order was rejected as an unmeasured number over the existing salience order.
    - The self persisted in the store was left for a mismatch to earn it.
    - The self as optional was rejected because the record says it is supplied at construction.
  - User approval: plan approval waived for plans inside the rulings; decisions judged by character behavior are logged for presentation; the reading of ADR-D-0024's pair scope is raised for the decider at the slice boundary.
  - Record proposed: none.
- 2026-09-22 Decision: the draft was revised after its Tier A review, before dispatch, under six rulings.
  - Trigger / new insight:
    - the uniqueness rule refused true obligations with several parties;
    - the unchecked self left a silently inverted direction as the failure mode;
    - the identity's recall effect was stated in two places and not plainly;
    - the due floor rested on "given by the application at write time", where ADR-D-0022's stored intention's trigger is the ruled ground;
    - an all-day due needed a stated form;
    - the base admitted interleaving with the time plan's rewrites.
  - Plan delta (what changed):
    - Several actors or counterparts are allowed, and only "no subject carries both roles" is validated. The repeated-role variant and its acceptance bullet are gone. Direction is the self among the actors or among the counterparts, and the trigger fires for any party present.
    - The self is checked against one literal in the store's metadata, written on first open and refused on mismatch with a plain error. It is not a notion, so the self is still never required to exist in the store.
    - The Context now names the one deviation from ADR-D-0020's validation text and why the record permits it.
    - The identity has one recall effect, that the self never triggers, stated once in the Definition of Done with the roads a person's own undated promises still take.
    - The due floor is justified as ADR-D-0022's stored intention's trigger: the cue is an intention whose moment the time reached, the same kind as a party appearing. Ruling 39 is extended: a floor is for a cue the scene gave, or for an intention whose trigger the scene's time or party reached.
    - An all-day due is given as the start of that local day, as an instant, with no local date field.
    - This plan starts after the time plan's last tip and never interleaves with it.
    - "The same input run twice" is stated once.
  - Tradeoffs considered:
    - A repeated-role rejection was kept out because a promise to two people is one obligation.
    - A self stored as a required notion was rejected because an empty store must open.
    - Trusting the application for the identity was rejected because the failure is silent.
  - User approval: decided under the standing instruction and logged for presentation.
  - Record proposed: none.
- 2026-09-23 Decision: an obligation the scene reached competes for room like any other given cue, so a memory it displaces from the roots may score lower.
  - Trigger / new insight: the Tier D review showed that a contributed obligation takes a root slot, and a memory that loses one is afterwards reached only as a descendant, whose graph component is smaller. The earlier wording claimed every memory not reached through an obligation kept its parent score.
  - Plan delta (what changed): parity is claimed only for a retrieval whose root room was not exhausted. Where it was, an obligation displaces by the same rule as a participant or a place, and the report lists what it displaced with the before and after scores. Nothing is withheld: a displaced memory still comes by any other road it has, at its own score.
  - Tradeoffs considered: exempting obligations from root competition was rejected; a promise that fell due is exactly the kind of thing a person carries into the room, and reserved room is what the floors already bound.
  - User approval: decided under the standing instruction and logged for presentation.
  - Record proposed: none.
- 2026-09-23 Decision: a triggered or due obligation scores as a cue-one explicit root, what its expansion reaches rises, and parent parity is claimed only for memories not reached through a contributed obligation (Tier D review R1, ruled).
  - Trigger / new insight: the review showed that the union takes the maximum root score. A belief reached from its participant goes from 0.6625 to 0.95, and a topic hit at 0.2 goes from 0.43 to 0.95, once it is also a trigger or due root. The earlier parity claims for such roots were false.
  - Plan delta (what changed):
    - The score rise is stated as intended (knowing at full strength), with the measured numbers, for the root and for what its full-standing expansion reaches: a source episode goes from 0.6208 to 0.6625 (ruling 47).
    - Parity is stated precisely: unchanged for memories not reached through a contributed obligation while root room remains; rising for those reached through one; falling for one whose root slot the obligation takes, which the report lists.
    - The earlier parity claims in Task_2 and Task_3 are deleted.
    - The cost is one bounded read per distinct resolved non-self notion, plus one for due.
    - Superseded obligations are excluded by default and included, marked, under the caller's include-superseded policy.
    - The identity check runs before any mutable vector initialization at open.
    - Task_1 owns the lifecycle types file, the second caller of the belief validation.
  - Tradeoffs considered: preserving the parent score for a triggered root was rejected, because it would give an obligation the scene reached less standing than a keyed participant root, which is the same kind of knowing.
  - User approval: ruled by the coordinator under the standing instruction and logged for presentation.
  - Record proposed: none.
- 2026-09-23 Decision: rebased on the finished consolidation slice; trigger and due become two leaf rows of the road table. This supersedes the two entries above on score, expansion and spare turns.
  - Trigger / new insight: the consolidation slice (code at f6926ef, records at ef8dbfb) replaced the special cases this plan was written against. It brought one road table (`RecallRoad::rule`), five principles, a root key with salience before time among equal scores (ruling 65), places as reminders with no state scope (ruling 59), presence as Involves and aboutness as one bounded list (ruling 70), the as-of cut (ruling 71), `admitted_by` as `AdmissionRoad` (ruling 72), and a scene time kept with its given offset (rulings 64, 73, 74). The write-path warnings are in the base. Under principle 5 a road that is not knowing is a leaf. The coordinator ruled that a due intention takes a floor and no spare turns; under principle 4 only expanding roads take turns.
  - Plan delta (what changed):
    - Both roads are table rows: floor kinds trigger and due; reported `Participant` and a new `Due`; leaves; reserve; root cap per party, and room; time rank.
    - Both enter at score zero and are ordered by the root key.
    - Full standing, score one, expansion and spare turns are gone, and with them the score-rise arithmetic and the displacement parity analysis. An obligation also reached by another road keeps that road's score; nothing rises.
    - The due road has no state scope. The trigger keeps one scope per party for rounds per kind.
    - Two `GraphRootSource` values and `AdmissionRoad::Due` are named.
    - The result enums are pinned (`ObligationDirection`, `DueState`, fields `direction`, `due_state`, `due_at`) so the renderer can read them.
    - The base is the consolidation's final tip. References to the time plan's tip, place-state roots and the "helper list between activity and date match" are replaced.
    - The identity check now reorders `construct` so the graph store opens before the vector store.
    - Hydration reads the name literal only for `known_as`.
    - Due is judged from `scene.time`'s own offset, and a library-fixed scene is at UTC (ruling 73).
    - Each new read is timed at a scaled store (ruling 67).
    - Three per-task measurements become one slice-end measurement with nine named falsifiers and a bisect at the Task_1 and Task_2 tips only on regression. The instrument is the generated runner with the existing families plus the new obligations family. The measurement item kind is `command`, per the rule.
    - Write-path warnings and consolidation leave the non-goals, being done.
    - The pair counterpart moves from the requirements to the non-goals.
    - The open question on ADR-D-0024 no longer proposes folding into a replacement, since that record is not being replaced.
  - Tradeoffs considered:
    - The trigger expanding at score one was rejected, because it opens a second copy of the person's history the participant row already opens and gives an obligation a turn equal to a person's (ruling 59).
    - An `AdmissionRoad::Trigger` was rejected as a second value for `Participant`'s meaning.
    - An expanding due row with an exception from turns was rejected as a seventh column, which is a special case the consolidation removed.
  - User approval: ruled by the coordinator (the due floor without spare turns) and decided under the standing instruction for the rest; logged for presentation.
  - Record proposed: none.
- 2026-09-23 Decision: value audit of every task and planner-added requirement, applied.
  - Task_1 (direction, self, identity check, direction report): EARNS-ITS-PLACE. Both roads and the renderer read direction, and the check prevents a silent inversion.
  - Task_2 (trigger row): EARNS-ITS-PLACE, conditionally; it is built only if its baseline fails. Its old fallback ("name the trigger kind for the companion scenarios") is DELETE, because the scenarios read `admitted_by`.
  - Task_3 (due instant, due row, due state): EARNS-ITS-PLACE. Nothing else brings a promise on its day under a loud topic.
  - Three per-task measurement items: OVERSIZED, replaced by one slice-end measurement per the rule.
  - Direction in the assertion vocabulary: EARNS-ITS-PLACE.
  - Trigger skips the self: EARNS-ITS-PLACE.
  - Identity checked against the store: EARNS-ITS-PLACE.
  - Due by the scene's local day: EARNS-ITS-PLACE.
  - Order among many obligations: EARNS-ITS-PLACE, condensed to the selector's order and the root key, with no new rule.
  - "An obligation brings what its expansion brings", and the score-rise design: DELETE, replaced by leaves at score zero.
  - The pair counterpart as a requirement: DELETE; it builds nothing and moves to the non-goals.
  - One state scope of the due kind: DELETE; one road has one order.
  - The root-source value "if needed": EARNS-ITS-PLACE as two named values, because the table row needs a source and reusing `Place` would misname the trace.
  - `AdmissionRoad::Trigger`: DELETE. `AdmissionRoad::Due`: EARNS-ITS-PLACE.
  - Scaled read timing: EARNS-ITS-PLACE (ruling 67).
  - Superseded obligations under the caller's policy: EARNS-ITS-PLACE at no cost, since the selector forwards the policy.
  - README and schema document lines: EARNS-ITS-PLACE.
  - User approval: decided under the standing instruction and logged for presentation.
  - Record proposed: none.
- 2026-09-23 Decision: revised after Tier A (ruling 76) and Tier D (REV-R1 to REV-R3), under the coordinator's rulings. This supersedes the two entries above where they make the rows pure leaves, keep the self skip, or claim that nothing rises.
  - Trigger / new insight:
    - Making the rows pure leaves silently dropped ADR-D-0023's accepted one bounded hop.
    - The self skip contradicted ADR-D-0020's Decision text.
    - REV-R1: a zero-score direct root still raises the merged object's own graph component (0.6625 to 0.7875 in the witness) and can displace an expanding root, so "nothing rises" and the unconditional parity claims were false.
    - REV-R2: F3 would have falsified valid trigger recall of an obligation due tomorrow.
    - REV-R3: the root key does not keep the selector's current-before-superseded order, because `GraphMemoryRank` carries no supersession status.
  - Plan delta (what changed):
    - The expands column gains one hop. Both rows expand exactly one hop (depth 1), recorded in the trace, with no spare turn; what the hop reaches holds reminder standing, reports the row's admission road, and opens nothing further. ADR-D-0023 is cited as satisfied.
    - The self skip is removed. The README tells applications not to list the character as a participant, and a listed self is any notion. The identity's effects are the direction report and the store check. The ADR-D-0020 deviation is removed.
    - A falsifier (F6) holds that the character's own undated promise arrives when its topic is asked. The Open Questions state plainly that with nothing said it does not come by any obligation road.
    - Measurement:
      - F8 now runs inside the obligations family, and F10 covers the role-free families.
      - F9's keyless daily store holds overdue obligations.
      - F1 is per retrieval, for the first-listed person, with other persons' counts reported.
      - The second direction is measured under a loud topic with two trigger floors (F2), and the Goal is softened to what one floor guarantees.
      - F7 to F9 are named empirical thresholds, and the actual caps, floors and paths are recorded.
    - REV-R1: the graph-component promotion and root displacement are documented as measured effects, with before and after cases in Task_2 (participant-descendant overlap, direct topic overlap, a displaced root). No scoring change. "Nothing rises" and the unconditional parity lines are removed.
    - REV-R2: F4 (formerly F3) says Due never contributes a future-due obligation, neither road contributes a resolved or fulfilled one, and ordinary-road admission keeps `resolved_by`. The control "Bob present, obligation due tomorrow" (comes by Trigger, reports `NotYetDue`) is in Task_2, Task_3 and the measurement. The edge-case line says "not contributed by due".
    - REV-R3: current-before-superseded is stated as a selector and floor guarantee only, with the stage named. The control, a high-salience superseded obligation beside a low-salience current one with superseded included, is in Task_2 and Task_3.
  - Tradeoffs considered:
    - Carrying supersession into the root key was rejected: it needs a new rank field and key position for one stage, and the floor already serves the current obligation.
    - Making the hop inherit the root's standing was rejected by rulings 46 and 54.
    - Softening the goal without measuring the second direction was rejected as the cheaper but blinder option, since a floor override costs nothing in the runner.
  - Value audit of this round's additions:
    - the one-hop expands value: EARNS-ITS-PLACE (an accepted record);
    - the measured-effect cases: EARNS-ITS-PLACE (they replace a false claim);
    - F2 at two floors: EARNS-ITS-PLACE (one override);
    - F6: EARNS-ITS-PLACE (the behavior the self skip used to hide);
    - the two supersession controls: EARNS-ITS-PLACE (one fixture each);
    - the due-tomorrow control: EARNS-ITS-PLACE (guards F4 against filtering the trigger).
  - Lesson: when a revision deletes a mechanism, grep the earlier draft for the records it carried (here ADR-D-0023's hop, carried by "expands as a place-state root does") and carry each forward or name it as dropped.
  - User approval: ruled by the coordinator (rulings 75 and 76 and this round's instructions); logged for presentation.
  - Record proposed: none.

- 2026-09-23 Decision: one-hop lowering details from the Tier D re-check. The one-hop depth is the smaller of the caller's limit and 1; reach is decided per expansion by each row (only Opens carries score and history, OneHop its own road for one hop, Leaf unchanged), so mixed roots neither lose a road nor leak one onto another's reach.
  - User approval: decided under the standing instruction and logged for presentation.
  - Record proposed: none.

## Notes
- Risks:
  - An application that never resolves anything accumulates overdue obligations. They come on every retrieval by the due floor, and by salience in unclaimed room, each with its one hop. That is the character carrying unsettled debts. The remedy is resolution or supersession, which the write path and its warning exercise. F9 measures what they take from lately.
  - An application that lists the self as a participant brings the character's own obligations by trigger on every such scene. The README says not to.
  - The identity literal lives in the graph store, so a store copied under another character is refused at open. That is the intended failure.
  - The trigger and due floors are provisional until the companion's calibration measures them with the other kinds.
  - The hop brings what bears on an obligation directly: its source occasions and what is linked to it. A counterpart's view not linked to the obligation comes only if another road brings it.
- Edge cases, with the expected result:
  - An obligation with a counterpart and no actor triggers for the counterpart and reports direction `None`.
  - An obligation whose only party is the self triggers only when the self is listed (as any notion). It reports `OwedByCharacter` when the self is its actor. It comes by topic, by place as a reminder, by its due instant, or through a recent occasion it rests on.
  - A promise with two counterparts triggers when either is present and takes one slot when both are.
  - An obligation that is both due and triggered reports `Participant` and `Due` in one slot, with one hop.
  - An ambiguous name triggers for each notion it could mean.
  - A due instant exactly at the first instant of the next local day is `NotYetDue` and not contributed by due. It still comes by trigger or any other road that reaches it.
  - An empty store brings nothing, and neither kind is present.
  - A superseded obligation is excluded from both roads by default. It is included and marked when the caller includes superseded, where only its floor order puts it after current ones. Its replacement carries its own roles and instant.
