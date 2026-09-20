# Plan: v0.2 schema groundwork

- status: draft
- generated: 2026-09-20
- last_updated: 2026-09-20
- work_type: code

## Goal
- Before any v0.2 route work, the stored shapes say what the accepted decisions say: nothing the library never reads is stored or accepted, currency is read from the supersession chain, and an entity is a notion whose name is a belief. The routes, the scene and the scope keys are then built once, on shapes that will not move under them.

## Definition of Done
- The archived and deleted retention states, the archive machinery, the five lifecycle options whose only legal value is their default, the deferred-destructive placeholder, the stability measure, the scope hint, the operation id and idempotency key, and the confidence on interpreted memory and on links are gone from the domain, the drafts, the graph mapping, the stats store and the public surface, and nothing behaves differently except that they can no longer be supplied and that forgetting a thread no longer deletes the thread's own vector, which made the thread unreachable against ADR-D-0018.
- There is no stored current flag. A memory is current when its retention allows it and no interpreted memory supersedes it; a superseding memory can be written through the validated write path, and `correct` no longer rewrites the memory it supersedes.
- An entity carries its identity and nothing else. What the character holds about it is ordinary interpreted memory about it; a belief may carry a known-as assertion, and the library can find every notion currently known by an exact name. An application can give a name it already owns before any experience exists, and that grounding is recorded as given.
- The README describes what ships; every task's PR is green in CI; the companion evaluation repository's Task_4 has the library commit it needs.

## Planner-added requirements
- A crate-internal query "current known-as beliefs by normalized name". Needed because: with no name on the entity there is otherwise no way to honor the v0.2 draft's "an exact name cues every entity that bears it", and the scene slice builds on it; it is not a facade lookup, so ADR-I-0020 stands.
- Commit derives, in the same graph batch as the memory, an About link from a belief to each notion it is about and a Supersedes link for each memory it supersedes. Needed because: bounded expansion and the currency readers follow explicit links, not the ids a memory lists. The memory's own lists stay the one thing an author writes; the links are an index derived from them at commit, never authored, so a caller-built plan cannot omit them and the two cannot disagree.
- A marker on a belief about a notion that its grounding was given by the application, accepted in place of source experiences only on a memory that has at least one notion among its subjects and no sources. Needed because: the source floor otherwise rejects a name given before any experience exists, and a source-less memory with no marker is indistinguishable from a defect (ADR-D-0028).

## Scope / Non-goals
- Scope: `src/**`, `tests/**`, `Cargo.toml`, `Cargo.lock`, `README.md`, up to two decision records, this plan.
- Non-goals: the scene, scope keys, routes, floors, prospective memory fields, the renderer (later v0.2 slices, each with its own plan); sameness and containment assertions and their read-through (they arrive with the scenario that needs them; until then they are held as ordinary beliefs); a fork diagnostic on supersession; an un-suppress operation (known gap); the user and assistant preference subtypes (decided with the scene slice); the stats-projection clones in the correction path beyond what Task_2 touches; whether `api` and `domain` stay public modules (decided once after these deletions); any concurrency guard (the census's risk families are recorded for the write-path slice); migration of stored data (no consumers).

## Design
- Chosen: a belief about a notion is an ordinary interpreted memory whose subjects include the notion, so whatever the character holds, a kind, a doubt, a relation nobody anticipated, is representable as text with evidence and supersession. Such a memory may carry a short list of machine-readable assertions, each a subject notion, a predicate and, today, a name; the predicate vocabulary is closed and has one member, known as (an assertion whose object is another notion arrives with sameness), because it is the only one a v0.2 scenario reads through. Structure: no new object type and no new subtype; the name lives in one place, the belief; the graph stores the assertion as predicates on the belief and a normalized literal for lookup. Evolution: a new predicate is one variant plus its read-through, added with the scenario that needs it; widening to open predicates later is a validation change, not a shape change. Verification: service-free; the name query and currency are graph reads testable on the embedded store. Operation: reading a notion through a belief is one hop more than reading a field; its cost is measured when the entity route exists. Human: a tentative belief is text with no assertion, and the assertion is the commitment, which is ADR-D-0034's "doubt first" in the data. An assertion is the character's own commitment: a memory held as someone else's claim ("he says he is Bob") carries none, so it never cues a notion by name. Safety: no lookup surface is added.
- Alternative: one typed subtype with an aspect enum (known as, kind, standing, same as, within). Structure: a rigid payload that must anticipate every belief. Evolution: every unanticipated belief needs a schema change or falls outside the type. Rejected by the decider on 2026-09-20.
- Alternative: a free-text claim with no structure. Structure: simplest. Verification: an exact name cannot be found without a model reading text at recall, so the v0.2 draft's exact-name cue cannot be honored.
- Alternative: an entity-to-entity link for sameness. Links carry no evidence and cannot be superseded, which contradicts ADR-D-0034.
- Currency, chosen: delete the stored flag and read the chain (retention allows it, and no incoming Supersedes link from an interpreted memory). The author's statement of supersession is the memory's own `supersedes` list; the Supersedes link is derived from it at commit, in one graph batch with the memory, and is what readers follow. Nothing else may write one: `correct` goes through the same derivation, and the `link` operation rejects a caller-authored Supersedes link, since it would end a memory's currency outside the validated write path (ADR-D-0025). Alternative: keep the flag as a cache. That leaves two sources of truth and forces the validated write path to mutate predecessors, which is the stale-overwrite family the census found; the stats projection keeps its own derived counters, which is the right home for a cache.
- Why chosen: it is the smallest shape that honors ADR-D-0034, ADR-D-0030, ADR-D-0018 and ADR-D-0032 and the one read-through v0.2 has a scenario for. Fit: `docs/roadmap/roadmap-phases/v0_2_scoped_continuity_reflection.md` (groundwork ruling, section 1, section 2.1); ADR-D-0028 for declared grounding; ADR-I-0020 for no lookup surface.

## Compatibility stance (required if a contract/interface/persisted format is touched)
- surface: the public domain and draft types, the lifecycle policy types, the graph vocabulary, the stats tables, the vector surface for entities.
- stance: break
- justification: no external consumers (the no-backcompat ruling); the one locatable consumer is the companion evaluation repository, whose Task_4 follows this plan's last merge (library first, harness immediately after). No stored-data migration.

## Context (workspace)
- Related files/areas: `.agent-work/researcher/v0-2-groundwork-census-report.md` has every site with file and line for each item below, and the concurrency families; read it before each task. `src/domain.rs`, `src/domain/lifecycle.rs`, `src/api/types/{draft,lifecycle,retrieval,write_plan}.rs`, `src/usecases/{write_planning,remember,correct_forget,retrieve,link,stats_projection}.rs`, `src/policy/{embedding_surface,graph_expansion,retrieval_selectivity}.rs`, `src/adapters/oxigraph/*`, `src/adapters/stats/*`, `src/ports/retrieval_stats.rs`, `src/test_support.rs`, `src/lib.rs`, `README.md`.
- Existing patterns or references: expansion already computes the superseded set from explicit Supersedes links (`src/policy/graph_expansion.rs`); the write planner already generates hint links with stable ids (`src/usecases/write_planning.rs`); the correction retry tests (`src/usecases/correct_forget.rs`) pin stats repair on an idempotent retry.
- Design record consulted and deviations from its acceptance: ADR-D-0018, D-0020, D-0028, D-0029, D-0030, D-0032, D-0034, ADR-I-0015, I-0020, I-0022. No deviation.

## Open Questions (max 3)
- None. The six planning decisions were ruled by the decider on 2026-09-20 and are recorded in the v0.2 draft.

## Assumptions
- A1: No production path reads the confidence, stability, scope hint, operation id or idempotency key for behavior beyond validation and whole-value equality. Source: census questions 2 and 3, value audit 2026-09-20; each task re-verifies before deleting.
- A2: The ADR-I-0022 continuity baselines do not move under Task_1 and Task_2, and move under Task_3 only through the entity surface leaving the vector index. Source: unverified; the companion repository's Task_4 and Task_6 measure it, and this plan changes no ranking constant.
- A3: Removing the entity from the default candidate object types leaves no retrieval path that needs an entity as a vector root. Source: census question 1 (entities are traversal intermediates, never pack content); checked by Task_3's tests.

## Tasks

### Task_1: Nothing the library never reads is stored or accepted
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
- depends_on: []
- description: |
  Delete, per the census site lists: `RememberInput.scope_ids`; the five lifecycle options whose non-default value is rejected, their knob enum and error, and the deferred-destructive policy; `Stability`; the two aliased schema-version constants (keep one); `operation_id`, `idempotency_key`, the `PrepareOptions` override and the remember input hash whose only consumer is that key, with the doc comment that promises retry checks; `DerivedMemory.confidence`, `MemoryLink.confidence`, the replacement draft's confidence, the shared predicate and score validation where nothing else uses it; the archived and deleted retention states, `ThreadStatus::Archived`, `ArchivePolicy`, `ForgetMemoryDraft::archive_thread`, the settable target retention state and thread status, the include-archived and include-deleted flags and their omission reasons, the stats parser arms and the selectivity arms that named them. Keep the thread-member cascade of forget. The supported behavior of each deleted option becomes the invariant.
- acceptance:
  - None of the deleted names appears in `src`, `tests` or `README.md`, and no deleted field is accepted by a draft or emitted to the graph or the stats store.
  - Forgetting a thread's members still suppresses them and removes their vectors; nothing deletes a thread's own vector any more, so a thread stays reachable (ADR-D-0018).
  - Retry identity is what it really is: prepared candidate ids are deterministic from supplied ids and stable defaults, a replay of the same plan is accepted by content equality, and correction identity is still deterministic; the existing retry tests pass unchanged in intent.
  - The three-tier selectivity count still means something with two retention states, or is reduced to what it means, with the reason in the report.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review against acceptance and the census site lists; confirm no behavior change beyond the deletions"

### Task_2: Currency is read from the chain
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
- depends_on: [Task_1]
- description: |
  Remove the stored current flag from the domain, the drafts, the graph mapping and the API lifecycle filter. A memory is current when its retention allows it and no interpreted memory supersedes it; a suppressed successor still supersedes, so forgetting a correction does not resurrect what it corrected. Delete the two staged-validation rules that reject a current memory with predecessors (they contradict `correct`). `correct` stops rewriting the memory it supersedes, and so a correction is a supersession and nothing else: the two remaining correction options go, `supersede_replaced_derived_memories` because a correction that did not supersede could no longer end currency at all, and `suppress_superseded_derived_memories` because suppression is a separate decision about a memory (ADR-D-0018), made through forget when a caller wants the corrected version out of history too. A corrected predecessor is therefore Active and superseded, where today it is Suppressed. Commit derives a Supersedes link for each predecessor a memory names and writes a commit's objects and links as one graph batch (the combined write `correct` already uses), so no reader sees a successor without its link. The `link` operation rejects a Supersedes relation. The stats projection keeps its current counters as a cache it derives itself. Shrink the correction path's projection clones to an immutable plan if this task's edits reach them; otherwise leave them.
- acceptance:
  - A superseding interpreted memory written through prepare, validate and commit is current and its predecessor is not, with no in-place mutation of the predecessor.
  - A caller-built plan that names a predecessor and carries no link still ends the predecessor's currency, and `link` with a Supersedes relation is rejected.
  - Intended outcomes, read from links and retention alone: a corrected or otherwise superseded predecessor is omitted by default with the reason superseded and returned under the include-superseded policy; an independently forgotten memory is omitted with the reason suppressed and returned only under include-suppressed; a memory both superseded and suppressed reports suppressed, since retention is checked first. The README and the tests that expected a corrected predecessor to be suppressed are changed to say this.
  - The correction retry tests still show stats repair on an idempotent retry.
  - One focused scenario on both the in-memory and the SQLite stats stores: a predecessor is stored and indexed, an ordinary successor is committed without rewriting it, and the graph's currency and the predecessor's cached current counters both show it no longer current; the successor is then suppressed and the predecessor does not become current again.
  - Selectivity's current counts are unchanged for the existing test fixtures.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review; trace that every reader of currency now reads the chain, and that the census's stale-overwrite family for correct is gone rather than moved"

### Task_3: An entity is a notion, and its name is a belief
- type: impl
- owns:
  - src/**
  - tests/**
  - README.md
  - Cargo.toml
  - Cargo.lock
  - docs/decisions/**
- depends_on: [Task_2]
- description: |
  The entity keeps its id, type tag, creation time and schema version; name, aliases, entity type (the enum goes), canonical key, summary and the updated time go. The entity has no embedding surface and leaves the default candidate object types. An interpreted memory may carry assertions (subject notion, predicate, name) with a closed predicate vocabulary of one, known as; the graph stores, beside the name as given, a normalized literal (Unicode NFKC through the `unicode-normalization` crate as a direct dependency, then case-folded, then whitespace-collapsed), and a crate-internal query on the graph authority port returns the notions currently known by an exact name, several when the name is shared. The query's first production reader is the scene slice; until then it is exercised by tests, and a scoped allow that cites the scene slice is acceptable if the lint demands it. A belief about a notion may be marked as given by the application, which satisfies the source floor in place of source experiences: only with at least one notion subject and no sources. Commit derives the About links the Planner-added requirements name. Two decisions here may deserve records, each held to invariants with mechanism left in this plan: the form of a belief about a notion (an ordinary interpreted memory; assertions with a closed vocabulary; the assertion is the character's own commitment), and how a belief the application gives enters (which closes an item ADR-D-0034 and ADR-D-0028 leave uncovered). Propose each if its admission test passes and let the test choose the record type (`durable-docs-authoring`); the decider accepts or returns them.
- acceptance:
  - An application creates a notion and gives its name before any experience exists, in one write plan; the name is found by the exact-name query; a second notion given the same name makes the query return both.
  - Renaming is an ordinary supersession: the new name is found, the old name is not, and the old belief is still there as history.
  - A belief reached by content leads to its notion through a generated link, and expansion from that notion works as it does today.
  - A memory with neither sources nor the given marker is still rejected; the marker is rejected on a memory with sources and on a memory with no notion subject; an assertion whose subject is not among the memory's subjects is rejected; an unknown predicate is rejected.
  - No code path reads a name, kind or summary from an entity, and the README's restart recipe still works with caller-supplied ids.
- validation:
  - kind: command
    required: true
    owner: worker
    detail: "cargo fmt --check; cargo check; cargo clippy --all-targets -- -D warnings; cargo test"
  - kind: review
    required: true
    owner: reviewer
    detail: "Tier D diff review; Tier A review of the public shape and the proposed record against ADR-D-0034, D-0028, D-0029 and ADR-I-0020"
  - kind: manual
    required: true
    owner: user
    detail: "The decider accepts or returns each proposed record"

## Task Waves (explicit parallel dispatch sets)

- Wave 1: [Task_1]
- Wave 2: [Task_2]
- Wave 3: [Task_3]

One crate, shared files: the tasks are sequential, one worker at a time, each PR stacked on the previous one. The second library worker takes the Tier D fix rounds. When Wave 3 merges, the Orchestrator hands the merge commit to the companion repository's Task_4.

## Rollback / Safety
- Each task is one PR and reverts cleanly on its own in reverse order. Once the companion repository's Task_4 has merged against this work, a revert here is coordinated with it (its plan says how).

## Progress Log (append-only)

- (none yet)

## Decision Log (append-only; re-plans and major discoveries)

- 2026-09-20 Decision: the planning decisions, ruled by the decider before drafting.
  - Trigger / new insight: the groundwork census, a value audit, a design consult on the belief form, and a second value test the decider asked for.
  - Plan delta (what changed): link confidence goes with memory confidence; no idempotency ledger; a first belief enters with a given-by-the-application marker (a marker, not a free-text source string, since nothing would read the string); un-suppression is a known gap; the preference subtypes wait for the scene slice; a belief is an ordinary interpreted memory with optional assertions, not a typed subtype; only known as is built, sameness and containment wait for their scenario; no fork diagnostic; no `known_as` sugar on the entity draft until the example loop shows the friction.
  - Tradeoffs considered: in the Design section.
  - A consequence the plan review surfaced, for the decider's eye at approval: with no predecessor rewrite, a correction no longer suppresses what it corrects; the corrected version is superseded history, reachable under the include-superseded policy, and the two correction options that said otherwise are deleted.
  - User approval: yes, 2026-09-20, for the decisions above; the consequence is presented with the plan.
  - Record proposed: up to two in Task_3, one decision each (the form of a belief about a notion; how an application-given belief enters), type chosen by the admission test; the name query and the entity surface are mechanism and stay in this plan. Acceptance pending.

## Notes
- Risks: Task_1 is wide (public surface, graph vocabulary, stats tables) and mechanical; the census site lists are the checklist. Task_3 changes what the vector index holds, which can move retrieval results for fixtures that matched on an entity's name; that is expected and is measured in the companion repository.
- Edge cases: forgetting a rename leaves the notion with no current name, and the repair is a new belief, not un-suppression; two beliefs naming one notion at once (a nickname and a given name) are both current, since neither supersedes the other; normalization must not merge names that differ only by script in ways a person would not (keep it to case, whitespace and Unicode normalization).
