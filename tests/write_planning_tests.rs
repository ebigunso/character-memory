//! Write-plan contracts observed through the real graph and embedded vector stores.
//! Persistent-mode coverage closes and reopens the stores before reading canonical state.

use character_memory::{
    CandidateValidationIssue, CandidateValidationStatus, CharacterMemory, CommitOptions,
    DerivedMemoryDraft, DerivedType, EntityDraft, EpisodeDraft, IncludedDerivedMemory,
    MemoryCandidate, MemoryId, MemoryLinkDraft, ObjectType, PrepareOptions, RelationType,
    RememberDiagnosticCode, RememberInput, RememberOptions, RememberOutcome, RememberWritePlan,
    RetrievalContext, StatsUpdateCause, StatsUpdateStatus,
};
use tempfile::TempDir;
use uuid::Uuid;

#[path = "support/mod.rs"]
pub mod test_support;
use test_support as base;

#[tokio::test]
async fn core_commit_flow_works_in_in_memory_graph_mode() {
    let (memory, root) = setup_basic().await;

    let plan = memory
        .prepare(core_input("in-memory-core"), PrepareOptions::default())
        .await
        .expect("prepare should produce a core plan");
    memory
        .validate_plan(&plan)
        .await
        .expect("core plan should validate");
    let outcome = memory
        .commit(plan, graph_only_commit_options())
        .await
        .expect("core plan should commit");

    ensure_graph_only_outcome(&outcome);
    base::close_and_remove_root(memory, root).await;
}

#[tokio::test]
async fn core_commit_flow_works_in_persistent_graph_mode() {
    let root = TempDir::new().expect("store root should be created");
    let collection = base::unique_collection_name();
    let memory = setup_persistent(&collection, &root).await;

    let plan = memory
        .prepare(core_input("persistent-core"), PrepareOptions::default())
        .await
        .expect("prepare should produce a persistent plan");
    memory
        .validate_plan(&plan)
        .await
        .expect("persistent plan should validate");
    memory
        .commit(plan, CommitOptions::default())
        .await
        .expect("persistent plan should commit");

    memory
        .close()
        .await
        .expect("persistent stores should close");
    let memory = setup_persistent(&collection, &root).await;
    let retrieved = memory
        .retrieve(RetrievalContext::new("persistent-core"))
        .await
        .expect("committed graph state should survive reopening");

    assert!(retrieved
        .pack
        .relevant_episodes
        .iter()
        .any(|episode| episode.summary == "persistent-core source observation"));
    assert!(retrieved
        .pack
        .salient_observations
        .iter()
        .any(|observation| observation.text == "persistent-core source observation"));
    let derived = all_derived(&retrieved)
        .into_iter()
        .find(|included| included.memory.id == stable_id("persistent-core", 2))
        .expect("committed derived memory should survive reopening");
    assert_eq!(derived.memory.text, "persistent-core derived memory");
    assert_eq!(
        derived.memory.entity_ids,
        vec![stable_id("persistent-core", 1)]
    );
    base::close_and_remove_root(memory, root).await;
}

#[tokio::test]
async fn plan_without_vector_candidates_writes_no_vectors_with_default_commit_options() {
    let (memory, root) = setup_basic().await;
    let prepare_options = PrepareOptions {
        include_vector_index_candidates: false,
        ..PrepareOptions::default()
    };
    let plan = memory
        .prepare(core_input("no-vector-candidates"), prepare_options)
        .await
        .expect("prepare should succeed without vector candidates");

    assert_eq!(
        count_candidates(&plan, |candidate| matches!(
            candidate,
            MemoryCandidate::VectorIndex(_)
        )),
        0
    );
    let outcome = memory
        .commit(plan, CommitOptions::default())
        .await
        .expect("default commit should honor empty plan vector targets");

    assert!(!outcome.persisted_object_ids.is_empty());
    assert!(outcome.vector_indexed_object_ids.is_empty());
    assert_eq!(outcome.vector_indexing_failure, None);
    base::close_and_remove_root(memory, root).await;
}

#[tokio::test]
async fn remember_wrapper_commits_equivalent_graph_state() {
    let (wrapper_memory, wrapper_root) = setup_basic().await;
    let (manual_memory, manual_root) = setup_basic().await;
    let input = remember_equivalence_input();
    let options = RememberOptions::default();

    let wrapper_outcome = wrapper_memory
        .remember(input.clone(), options.clone())
        .await
        .expect("remember wrapper should compose the public write-plan path");

    let plan = manual_memory
        .prepare(input, options.prepare)
        .await
        .expect("manual prepare should create the equivalent plan");
    let validations = manual_memory
        .validate_plan(&plan)
        .await
        .expect("manual validation should succeed");
    assert!(validations
        .iter()
        .all(|validation| validation.status == CandidateValidationStatus::Valid));
    let manual_outcome = manual_memory
        .commit(plan, options.commit)
        .await
        .expect("plan path should commit equivalent graph state");

    let episode_id = id("550e8400-e29b-41d4-a716-446655613401");
    let observation_id = id("550e8400-e29b-41d4-a716-446655613402");
    let entity_id = id("550e8400-e29b-41d4-a716-446655613403");
    let derived_id = id("550e8400-e29b-41d4-a716-446655613404");
    let link_id = id("550e8400-e29b-41d4-a716-446655613405");
    assert_eq!(wrapper_outcome, manual_outcome);
    assert_eq!(
        wrapper_outcome.diagnostics.validations,
        manual_outcome.diagnostics.validations
    );
    assert_eq!(
        wrapper_outcome.persisted_object_ids,
        vec![episode_id, observation_id, entity_id, derived_id]
    );
    assert_eq!(wrapper_outcome.persisted_link_ids.len(), 2);
    assert!(wrapper_outcome.persisted_link_ids.contains(&link_id));
    assert_eq!(
        wrapper_outcome.vector_indexed_object_ids,
        vec![episode_id, observation_id, derived_id]
    );
    let validation = wrapper_outcome
        .diagnostics
        .validations
        .iter()
        .find(|validation| {
            validation.candidate_index == 1
                && validation.candidate_kind == character_memory::MemoryCandidateKind::Observation
        })
        .expect("equivalent outcomes should include the warning-bearing validation");
    assert_eq!(validation.status, CandidateValidationStatus::Valid);
    assert!(validation.errors.is_empty());
    assert_eq!(
        validation.warnings,
        vec![CandidateValidationIssue::DuplicateObservationEcho {
            echo_surface: "Equivalent graph state observation".to_owned(),
            matching_episode_ids: vec![episode_id],
        }]
    );

    let validation_warning = wrapper_outcome
        .diagnostics
        .messages
        .iter()
        .find(|diagnostic| diagnostic.code == RememberDiagnosticCode::WritePlanValidationWarning)
        .expect("equivalent outcomes should include the warning projection");
    assert_eq!(
        validation_warning.severity,
        character_memory::DiagnosticSeverity::Warning
    );

    let query = RetrievalContext::new("equivalent graph state").with_trace();
    let wrapper_retrieval = wrapper_memory
        .retrieve(query.clone())
        .await
        .expect("wrapper graph state should be retrievable");
    let manual_retrieval = manual_memory
        .retrieve(query)
        .await
        .expect("manual graph state should be retrievable");
    assert_eq!(wrapper_retrieval, manual_retrieval);

    let episode = wrapper_retrieval
        .pack
        .relevant_episodes
        .iter()
        .find(|episode| episode.id == episode_id)
        .expect("caller-supplied episode should be in canonical graph state");
    assert_eq!(episode.summary, "Equivalent graph state observation");
    assert_eq!(
        episode.scene,
        character_memory::Scene::at(fixed_timestamp())
    );
    let observation = wrapper_retrieval
        .pack
        .salient_observations
        .iter()
        .find(|observation| observation.id == observation_id)
        .expect("caller-supplied observation should be in canonical graph state");
    assert_eq!(observation.text, "Equivalent graph state observation");
    let derived = all_derived(&wrapper_retrieval)
        .into_iter()
        .find(|included| included.memory.id == derived_id)
        .expect("caller-supplied derived memory should be in canonical graph state");
    assert_eq!(derived.memory.text, "Equivalent graph state claim");
    assert_eq!(
        wrapper_retrieval
            .trace
            .as_ref()
            .expect("trace requested")
            .graph_relations
            .iter()
            .filter(|relation| {
                relation.relation == RelationType::About
                    && ((relation.from.id == entity_id && relation.to.id == derived_id)
                        || (relation.from.id == derived_id && relation.to.id == entity_id))
            })
            .count(),
        1
    );

    base::close_and_remove_root(wrapper_memory, wrapper_root).await;
    base::close_and_remove_root(manual_memory, manual_root).await;
}

#[tokio::test]
async fn approval_flow_can_filter_candidates_before_commit() {
    let (memory, root) = setup_basic().await;
    let mut input = RememberInput::new("approval-flow base observation");
    let mut approved = DerivedMemoryDraft::new(DerivedType::Claim, "approval-flow approved memory");
    approved.id = Some(id("550e8400-e29b-41d4-a716-446655613201"));
    let mut dropped = DerivedMemoryDraft::new(DerivedType::Claim, "approval-flow dropped memory");
    dropped.id = Some(id("550e8400-e29b-41d4-a716-446655613202"));
    input = input
        .with_derived_memory(approved)
        .with_derived_memory(dropped);

    let mut plan = memory
        .prepare(input, PrepareOptions::default())
        .await
        .expect("prepare should expose candidates for approval");
    plan.candidates.retain(|candidate| match candidate {
        MemoryCandidate::DerivedMemory(candidate) => {
            candidate.draft.text != "approval-flow dropped memory"
        }
        MemoryCandidate::VectorIndex(candidate) => {
            candidate.target.id != id("550e8400-e29b-41d4-a716-446655613202")
        }
        MemoryCandidate::StatsUpdate(candidate) => {
            candidate.subject.id != id("550e8400-e29b-41d4-a716-446655613202")
        }
        _ => true,
    });

    let outcome = memory
        .commit(plan, graph_only_commit_options())
        .await
        .expect("reduced approved plan should commit");
    let approved_id = id("550e8400-e29b-41d4-a716-446655613201");
    let dropped_id = id("550e8400-e29b-41d4-a716-446655613202");
    assert!(outcome.persisted_object_ids.contains(&approved_id));
    assert!(outcome
        .persisted_object_ids
        .iter()
        .all(|id| *id != dropped_id));
    base::close_and_remove_root(memory, root).await;
}

#[tokio::test]
async fn authority_split_outcome_fields_are_coherent_on_healthy_commit() {
    let (memory, root) = setup_basic().await;
    let plan = memory
        .prepare(core_input("authority-split"), PrepareOptions::default())
        .await
        .expect("prepare should produce authority-split plan");
    let outcome = memory
        .commit(plan, CommitOptions::default())
        .await
        .expect("healthy authority-split commit should succeed");

    assert!(!outcome.persisted_object_ids.is_empty());
    if let Some(failure) = &outcome.vector_indexing_failure {
        assert!(!failure.unindexed_objects.is_empty());
        assert_eq!(
            failure.unindexed_object_ids(),
            failure
                .unindexed_objects
                .iter()
                .map(|object| object.id)
                .collect::<Vec<_>>()
        );
    }
    if let Some(failure) = &outcome.stats_update_status.failure {
        assert!(!failure.failed_object_ids.is_empty());
        assert!(matches!(
            failure.causes.as_slice(),
            [
                StatsUpdateCause::EndpointHydration { .. }
                    | StatsUpdateCause::EdgeWrite { .. }
                    | StatsUpdateCause::ObjectStateWrite { .. }
                    | StatsUpdateCause::HealthCheck { .. }
                    | StatsUpdateCause::HealthMark { .. }
                    | StatsUpdateCause::StoreUnhealthy { .. },
                ..
            ]
        ));
    } else {
        assert!(!outcome.stats_update_status.updated_object_ids.is_empty());
    }
    base::close_and_remove_root(memory, root).await;
}

async fn setup_basic() -> (CharacterMemory, TempDir) {
    base::try_setup_character_memory()
        .await
        .expect("basic setup should succeed")
}

async fn setup_persistent(collection_name: &str, root: &TempDir) -> CharacterMemory {
    base::try_setup_persistent_character_memory(collection_name.to_owned(), root.path(), None)
        .await
        .expect("persistent setup should succeed")
}

fn core_input(label: &str) -> RememberInput {
    let entity_id = stable_id(label, 1);
    let timestamp = fixed_timestamp();
    let mut entity = EntityDraft::new();
    entity.id = Some(entity_id);
    entity.created_at = Some(timestamp);

    let mut derived =
        DerivedMemoryDraft::new(DerivedType::Claim, format!("{label} derived memory"));
    derived.id = Some(stable_id(label, 2));
    derived.entity_ids.push(entity_id);
    derived.created_at = Some(timestamp);
    derived.updated_at = Some(timestamp);

    let mut episode = EpisodeDraft::new(format!("{label} source observation"));
    episode.created_at = Some(timestamp);
    let mut observation =
        character_memory::ObservationDraft::new(Uuid::nil(), format!("{label} source observation"));
    observation.created_at = Some(timestamp);

    RememberInput::new(format!("{label} source observation"))
        .with_episode(episode)
        .with_observation(observation)
        .with_entity(entity)
        .with_derived_memory(derived)
}

fn remember_equivalence_input() -> RememberInput {
    let timestamp = fixed_timestamp();
    let episode_id = id("550e8400-e29b-41d4-a716-446655613401");
    let observation_id = id("550e8400-e29b-41d4-a716-446655613402");
    let entity_id = id("550e8400-e29b-41d4-a716-446655613403");
    let derived_id = id("550e8400-e29b-41d4-a716-446655613404");

    let mut episode = EpisodeDraft::new("Equivalent graph state observation");
    episode.id = Some(episode_id);
    episode.scene = Some(character_memory::Scene::at(timestamp));
    episode.created_at = Some(timestamp);

    let mut observation =
        character_memory::ObservationDraft::new(episode_id, "Equivalent graph state observation");
    observation.id = Some(observation_id);
    observation.created_at = Some(timestamp);

    let mut entity = EntityDraft::new();
    entity.id = Some(entity_id);
    entity.created_at = Some(timestamp);

    let mut derived = DerivedMemoryDraft::new(DerivedType::Claim, "Equivalent graph state claim")
        .with_source_episode(episode_id)
        .with_source_observation(observation_id);
    derived.id = Some(derived_id);
    derived.entity_ids.push(entity_id);
    derived.created_at = Some(timestamp);
    derived.updated_at = Some(timestamp);

    let mut link = MemoryLinkDraft::new(
        ObjectType::Entity,
        entity_id,
        RelationType::AssociatedWith,
        ObjectType::DerivedMemory,
        derived_id,
    );
    link.id = Some(id("550e8400-e29b-41d4-a716-446655613405"));
    link.created_at = Some(timestamp);

    RememberInput::new("Equivalent graph state observation")
        .with_episode(episode)
        .with_observation(observation)
        .with_entity(entity)
        .with_derived_memory(derived)
        .with_memory_link(link)
}

fn all_derived(outcome: &character_memory::RetrieveOutcome) -> Vec<&IncludedDerivedMemory> {
    outcome
        .pack
        .derived_memories
        .iter()
        .chain(outcome.pack.preferences.iter())
        .chain(outcome.pack.relationship_notes.iter())
        .chain(outcome.pack.open_loops.iter())
        .chain(outcome.pack.commitments.iter())
        .chain(outcome.pack.character_signals.iter())
        .collect()
}

fn ensure_graph_only_outcome(outcome: &RememberOutcome) {
    assert!(!outcome.persisted_object_ids.is_empty());
    assert_eq!(outcome.vector_indexed_object_ids, Vec::<MemoryId>::new());
    assert_eq!(outcome.vector_indexing_failure, None);
    assert_eq!(outcome.stats_update_status, StatsUpdateStatus::default());
}

fn graph_only_commit_options() -> CommitOptions {
    CommitOptions {
        update_vectors: false,
        update_stats: false,
    }
}

fn count_candidates(
    plan: &RememberWritePlan,
    predicate: impl Fn(&MemoryCandidate) -> bool,
) -> usize {
    plan.candidates
        .iter()
        .filter(|candidate| predicate(candidate))
        .count()
}

fn stable_id(label: &str, index: u8) -> MemoryId {
    Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("character-memory:write-planning-test:{label}:{index}").as_bytes(),
    )
}

fn id(value: &str) -> MemoryId {
    Uuid::parse_str(value).expect("fixture UUID should parse")
}

fn fixed_timestamp() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-07-03T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc)
}
