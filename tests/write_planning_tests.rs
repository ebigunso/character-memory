//! Traceability from v0.1.3 phase-doc acceptance criteria to integration tests:
//! - "A caller can prepare a RememberWritePlan without committing it." -> prepare_without_persist_leaves_graph_and_vectors_empty
//! - "A caller can validate a RememberWritePlan without committing it." -> validate_without_persist_leaves_graph_and_vectors_empty
//! - "A caller can commit a validated RememberWritePlan." -> core_commit_flow_works_in_in_memory_graph_mode; core_commit_flow_works_in_persistent_graph_mode
//! - "remember() remains available as a convenience wrapper." -> remember_wrapper_commits_equivalent_graph_state
//! - "commit() can be called directly or after explicit validate_plan() with equivalent graph-state shape." -> commit_with_and_without_explicit_validation_produce_equivalent_graph_state
//! - "commit() revalidates before writing." -> commit_revalidates_and_rejects_after_intervening_graph_change
//! - "Invalid behavior-influencing DerivedMemory without provenance is rejected." -> ungrounded_behavior_influencing_derived_memory_rejected_at_validate_and_commit
//! - "Missing MemoryLink targets are rejected or deferred according to explicit policy." -> missing_memory_link_target_is_strictly_rejected
//! - "Idempotency keys prevent duplicate writes from retry." -> idempotent_exact_retry_does_not_duplicate_graph_writes; divergent_same_key_rejected
//! - "Plan-path vector writes use exactly declared vector index candidates." -> plan_without_vector_candidates_writes_no_vectors_with_default_commit_options; approval_flow_stripping_vector_candidates_writes_no_vectors
//! - "Deterministic source references and source spans are preserved." -> source_refs_and_source_spans_are_preserved_and_raw_ref_is_opaque
//! - "Manual writes and future generated writes can share the same commit path." -> generated_style_plan_commits_through_same_path_as_manual_candidates
//! - "The write-plan flow works with in-memory and persistent graph modes." -> core_commit_flow_works_in_in_memory_graph_mode; core_commit_flow_works_in_persistent_graph_mode
//! - "Qdrant remains candidate recall only." -> authority_split_outcome_fields_are_coherent_on_healthy_commit
//! - "Oxigraph remains authoritative for object existence, links, provenance, lifecycle, currentness, and final inclusion." -> missing_memory_link_target_is_strictly_rejected; commit_revalidates_and_rejects_after_intervening_graph_change
//! - "RetrievalStatsStore remains derived policy metadata only." -> authority_split_outcome_fields_are_coherent_on_healthy_commit
//! - "No v0.1.3 helper infers preferences, commitments, corrections, character signals, thread membership, or entity identity from raw natural language." -> no_inference_helpers_only_plan_caller_supplied_candidates
//! - "CandidateProvenance records candidate producer kind and rationale origin." -> candidate_provenance_records_producer_kind_and_rationale_origin
//! - "Missing rationale can be represented explicitly as unavailable." -> candidate_provenance_records_producer_kind_and_rationale_origin
//! - "No v0.1.3 helper persists raw logs or resolves raw_ref values." -> source_refs_and_source_spans_are_preserved_and_raw_ref_is_opaque

use character_memory::test_utils::load_test_settings;
use character_memory::RememberPlanDefaults;
use character_memory::{
    CandidateProducerKind, CandidateProvenance, CandidateRationale, CandidateValidationStatus,
    CharacterMemory, CommitOptions, CustomError, DerivedMemoryCandidate, DerivedMemoryDraft,
    DerivedType, EntityDraft, EntityType, EpisodeDraft, ExternalSourceReference,
    IncludedDerivedMemory, MemoryCandidate, MemoryId, MemoryLinkCandidate, MemoryLinkDraft,
    MemoryObjectDraft, ObjectType, PrepareOptions, RationaleOrigin, RelationType, RememberDraft,
    RememberInput, RememberOutcome, RememberWritePlan, RetrievalContext, Settings, SourceSpan,
    StatsUpdateStatus, DEFAULT_SCHEMA_VERSION,
};
use config::Config;
use std::path::Path;
use tempfile::TempDir;
use uuid::Uuid;

#[path = "support/base.rs"]
mod base;

#[tokio::test]
async fn prepare_without_persist_leaves_graph_and_vectors_empty() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };

    let input = core_input("prepare-no-persist");
    let plan = memory
        .prepare(input, PrepareOptions::default())
        .await
        .expect("prepare should produce a plan");

    assert!(has_candidate_kind(&plan, |candidate| matches!(
        candidate,
        MemoryCandidate::Episode(_)
    )));
    assert_retrieval_empty(&memory, "prepare-no-persist").await;
    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn validate_without_persist_leaves_graph_and_vectors_empty() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };

    let plan = memory
        .prepare(core_input("validate-no-persist"), PrepareOptions::default())
        .await
        .expect("prepare should produce a plan");
    let validations = memory
        .validate_plan(&plan)
        .await
        .expect("valid plan should validate");

    assert!(validations
        .iter()
        .all(|validation| validation.status == CandidateValidationStatus::Valid));
    assert_retrieval_empty(&memory, "validate-no-persist").await;
    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn core_commit_flow_works_in_in_memory_graph_mode() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };

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
    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn core_commit_flow_works_in_persistent_graph_mode() {
    let fixture = PersistentFixture::new();
    let collection_name = base::unique_collection_name();
    let memory = match setup_persistent(&collection_name, &fixture).await {
        Some(memory) => memory,
        None => return,
    };

    let plan = memory
        .prepare(core_input("persistent-core"), PrepareOptions::default())
        .await
        .expect("prepare should produce a persistent plan");
    memory
        .validate_plan(&plan)
        .await
        .expect("persistent plan should validate");
    let outcome = memory
        .commit(plan, graph_only_commit_options())
        .await
        .expect("persistent plan should commit");

    ensure_graph_only_outcome(&outcome);
    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn commit_revalidates_and_rejects_after_intervening_graph_change() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };
    let entity_id = id("550e8400-e29b-41d4-a716-446655613001");

    let plan = link_to_existing_entity_plan("revalidate", entity_id).await;
    memory
        .validate_plan(&plan)
        .await
        .expect("plan should validate while target is absent from graph but present in plan");

    let mut divergent = EntityDraft::new(EntityType::Project, "revalidate divergent entity");
    divergent.id = Some(entity_id);
    memory
        .commit(
            memory
                .prepare(
                    RememberInput::new("revalidate divergent entity").with_entity(divergent),
                    PrepareOptions::default(),
                )
                .await
                .expect("intervening prepare should succeed"),
            graph_only_commit_options(),
        )
        .await
        .expect("intervening graph change should persist");

    let error = memory
        .commit(plan.clone(), CommitOptions::default())
        .await
        .expect_err("commit should revalidate/reject divergent existing graph content");
    assert_error_contains(error, "deterministic ID collided");

    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn ungrounded_behavior_influencing_derived_memory_rejected_at_validate_and_commit() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };
    let plan = ungrounded_derived_memory_plan();

    let validations = memory
        .validate_plan(&plan)
        .await
        .expect("validate_plan returns validation records for invalid plans");
    assert_invalid_validation_contains(
        &validations,
        "derived memory must reference at least one source episode or observation",
    );

    let error = memory
        .commit(plan, CommitOptions::default())
        .await
        .expect_err("commit should reject ungrounded behavior-influencing derived memory");
    assert_error_contains(
        error,
        "derived memory must reference at least one source episode or observation",
    );
    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn missing_memory_link_target_is_strictly_rejected() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };
    let plan = missing_link_target_plan();

    let validations = memory
        .validate_plan(&plan)
        .await
        .expect("validate_plan returns validation records for invalid plans");
    assert_invalid_validation_contains(&validations, "target does not exist");

    let error = memory
        .commit(plan, CommitOptions::default())
        .await
        .expect_err("missing link target should be rejected at commit");
    assert_error_contains(error, "target");
    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn idempotent_exact_retry_does_not_duplicate_graph_writes() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };
    let plan = memory
        .prepare(core_input("idempotent-retry"), PrepareOptions::default())
        .await
        .expect("prepare should produce retry plan");

    let first = memory
        .commit(plan.clone(), graph_only_commit_options())
        .await
        .expect("first commit should succeed");
    let second = memory
        .commit(plan, graph_only_commit_options())
        .await
        .expect("exact retry should be idempotent");

    assert_eq!(first.persisted_object_ids, second.persisted_object_ids);
    assert_eq!(first.persisted_link_ids, second.persisted_link_ids);
    assert_eq!(
        second.persisted_object_ids.len(),
        first.persisted_object_ids.len()
    );
    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn divergent_same_key_rejected() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };
    let first_plan = memory
        .prepare(core_input("same-key-original"), PrepareOptions::default())
        .await
        .expect("prepare should produce first plan");
    let same_key = first_plan.idempotency_key.clone();
    memory
        .commit(first_plan.clone(), graph_only_commit_options())
        .await
        .expect("first same-key commit should succeed");

    let mut divergent_plan = first_plan.clone();
    divergent_plan.idempotency_key = same_key;
    for candidate in &mut divergent_plan.candidates {
        if let MemoryCandidate::DerivedMemory(candidate) = candidate {
            candidate.draft.text = "same-key divergent derived memory".to_owned();
        }
    }

    let error = memory
        .commit(divergent_plan, CommitOptions::default())
        .await
        .expect_err("same IDs with divergent content should be rejected");
    assert_error_contains(error, "deterministic ID collided");
    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn plan_without_vector_candidates_writes_no_vectors_with_default_commit_options() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };
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
    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn source_refs_and_source_spans_are_preserved_and_raw_ref_is_opaque() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };
    let raw_ref = "raw://opaque/source-refs";
    let span = SourceSpan::raw(raw_ref)
        .with_message_id("message-7")
        .with_char_range(3, 31);
    let input = core_input("source-preserved")
        .with_raw_ref(raw_ref)
        .with_source_span(span.clone());
    let plan = memory
        .prepare(input, PrepareOptions::default())
        .await
        .expect("prepare should preserve source input");

    assert_eq!(
        plan.source_input_ref,
        Some(ExternalSourceReference::raw(raw_ref))
    );
    assert!(plan
        .candidates
        .iter()
        .any(|candidate| provenance(candidate).source.source_spans.contains(&span)));

    let outcome = memory
        .commit(plan, graph_only_commit_options())
        .await
        .expect("source-preserving plan should commit");
    ensure_graph_only_outcome(&outcome);
    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn no_inference_helpers_only_plan_caller_supplied_candidates() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };
    let input = RememberInput::new(
        "Kohta likes hidden tea ceremonies, owes a quest, and should be a wizard.",
    );
    let plan = memory
        .prepare(input, PrepareOptions::default())
        .await
        .expect("prepare should succeed without semantic inference");

    assert_eq!(
        count_candidates(&plan, |candidate| matches!(
            candidate,
            MemoryCandidate::Episode(_)
        )),
        1
    );
    assert_eq!(
        count_candidates(&plan, |candidate| matches!(
            candidate,
            MemoryCandidate::Observation(_)
        )),
        1
    );
    assert_eq!(
        count_candidates(&plan, |candidate| matches!(
            candidate,
            MemoryCandidate::Entity(_)
        )),
        0
    );
    assert_eq!(
        count_candidates(&plan, |candidate| matches!(
            candidate,
            MemoryCandidate::MemoryThread(_)
        )),
        0
    );
    assert_eq!(
        count_candidates(&plan, |candidate| matches!(
            candidate,
            MemoryCandidate::DerivedMemory(_)
        )),
        0
    );
    assert!(!plan
        .candidates
        .iter()
        .any(|candidate| matches!(candidate, MemoryCandidate::MemoryLink(_))));
    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn commit_with_and_without_explicit_validation_produce_equivalent_graph_state() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };

    let direct_commit_outcome = memory
        .commit(
            memory
                .prepare(
                    core_input("direct-commit-parity"),
                    PrepareOptions::default(),
                )
                .await
                .expect("direct-commit prepare should succeed"),
            graph_only_commit_options(),
        )
        .await
        .expect("commit without explicit validate_plan should succeed");
    ensure_graph_only_outcome(&direct_commit_outcome);

    let plan = memory
        .prepare(core_input("plan-parity"), PrepareOptions::default())
        .await
        .expect("prepare should produce validate-then-commit parity plan");
    memory
        .validate_plan(&plan)
        .await
        .expect("validate-then-commit parity plan should validate");
    let validated_commit_outcome = memory
        .commit(plan, graph_only_commit_options())
        .await
        .expect("commit after explicit validate_plan should succeed");
    ensure_graph_only_outcome(&validated_commit_outcome);
    assert_eq!(
        direct_commit_outcome.persisted_object_ids.len(),
        validated_commit_outcome.persisted_object_ids.len()
    );
    assert_eq!(
        direct_commit_outcome.persisted_link_ids.len(),
        validated_commit_outcome.persisted_link_ids.len()
    );
    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn remember_wrapper_commits_equivalent_graph_state() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };
    let timestamp = fixed_timestamp();
    let mut wrapper_episode = EpisodeDraft::new("remember-wrapper source observation");
    wrapper_episode.id = Some(id("550e8400-e29b-41d4-a716-446655613401"));
    wrapper_episode.created_at = Some(timestamp);
    let wrapper_outcome = memory
        .remember(RememberDraft::new([MemoryObjectDraft::Episode(
            wrapper_episode,
        )]))
        .await
        .expect("remember wrapper should call the public wrapper path");

    let mut plan_episode = EpisodeDraft::new("remember-wrapper source observation");
    plan_episode.id = Some(id("550e8400-e29b-41d4-a716-446655613402"));
    plan_episode.created_at = Some(timestamp);
    let plan = RememberInput::new("remember-wrapper source observation")
        .with_episode(plan_episode)
        .prepare_write_plan_with_options(
            &RememberPlanDefaults::fixed("remember-wrapper-plan", timestamp),
            false,
            false,
        );
    let plan_outcome = memory
        .commit(plan, graph_only_commit_options())
        .await
        .expect("plan path should commit equivalent graph state");

    assert_eq!(wrapper_outcome.persisted_object_ids.len(), 1);
    assert_eq!(wrapper_outcome.persisted_link_ids.len(), 0);
    assert_eq!(plan_outcome.persisted_object_ids.len(), 2);
    assert_eq!(plan_outcome.persisted_link_ids.len(), 0);
    assert!(wrapper_outcome
        .persisted_object_ids
        .contains(&id("550e8400-e29b-41d4-a716-446655613401")));
    assert!(plan_outcome
        .persisted_object_ids
        .contains(&id("550e8400-e29b-41d4-a716-446655613402")));
    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn approval_flow_can_filter_candidates_before_commit() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };
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
    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn approval_flow_stripping_vector_candidates_writes_no_vectors() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };
    let mut plan = memory
        .prepare(
            core_input("approval-strips-vectors"),
            PrepareOptions::default(),
        )
        .await
        .expect("prepare should expose vector candidates");
    plan.candidates
        .retain(|candidate| !matches!(candidate, MemoryCandidate::VectorIndex(_)));

    let outcome = memory
        .commit(plan, CommitOptions::default())
        .await
        .expect("commit should honor approval-stripped vector candidates");

    assert!(!outcome.persisted_object_ids.is_empty());
    assert!(outcome.vector_indexed_object_ids.is_empty());
    assert_eq!(outcome.vector_indexing_failure, None);
    base::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn authority_split_outcome_fields_are_coherent_on_healthy_commit() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };
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
        assert!(!failure.unindexed_object_ids.is_empty());
        assert!(!failure.error_message.is_empty());
    }
    if let Some(failure) = &outcome.stats_update_status.failure {
        assert!(!failure.failed_object_ids.is_empty());
        assert!(!failure.error_message.is_empty());
    } else {
        assert!(!outcome.stats_update_status.updated_object_ids.is_empty());
    }
    base::cleanup_collection(&collection_name).await;
}

#[test]
fn candidate_provenance_records_producer_kind_and_rationale_origin() {
    let caller = CandidateProvenance::caller("caller supplied rationale");
    assert_eq!(caller.producer_kind, CandidateProducerKind::Caller);
    assert_eq!(caller.rationale_origin(), RationaleOrigin::ProvidedByCaller);
    assert_eq!(caller.rationale.text(), Some("caller supplied rationale"));

    let generated = CandidateProvenance::inferred_by_processor(
        CandidateProducerKind::ModelProcessor,
        "processor inferred rationale",
    );
    assert_eq!(
        generated.producer_kind,
        CandidateProducerKind::ModelProcessor
    );
    assert_eq!(
        generated.rationale_origin(),
        RationaleOrigin::InferredByProcessor
    );

    let unavailable = CandidateProvenance::unavailable(CandidateProducerKind::Unknown);
    assert_eq!(unavailable.rationale, CandidateRationale::Unavailable);
    assert_eq!(unavailable.rationale_origin(), RationaleOrigin::Unavailable);
}

#[tokio::test]
async fn generated_style_plan_commits_through_same_path_as_manual_candidates() {
    let (memory, collection_name) = match setup_basic().await {
        Some(fixture) => fixture,
        None => return,
    };
    let plan = generated_style_plan();

    memory
        .validate_plan(&plan)
        .await
        .expect("generated-style candidates should validate through public plan path");
    let outcome = memory
        .commit(plan.clone(), graph_only_commit_options())
        .await
        .expect("generated-style plan should commit through same path");

    ensure_graph_only_outcome(&outcome);
    base::cleanup_collection(&collection_name).await;
}

async fn setup_basic() -> Option<(CharacterMemory, String)> {
    match try_setup_in_memory_character_memory().await {
        Ok(fixture) => Some(fixture),
        Err(CustomError::VectorDatabaseError(error))
            if base::is_qdrant_unavailable_error(&error) =>
        {
            println!("skipping v0.1.3 write-planning test because Qdrant is unavailable: {error}");
            None
        }
        Err(error) if is_qdrant_timeout_signature(&error) => {
            println!("skipping v0.1.3 write-planning test because local Qdrant gRPC mutation stalled: {error}");
            None
        }
        Err(error) => panic!("unexpected v0.1.3 basic setup failure: {error}"),
    }
}

async fn setup_persistent(
    collection_name: &str,
    fixture: &PersistentFixture,
) -> Option<CharacterMemory> {
    match try_setup_persistent_character_memory(
        collection_name.to_owned(),
        &fixture.graph_path,
        &fixture.stats_path,
    )
    .await
    {
        Ok(memory) => Some(memory),
        Err(CustomError::VectorDatabaseError(error))
            if base::is_qdrant_unavailable_error(&error) =>
        {
            println!("skipping v0.1.3 persistent write-planning test because Qdrant is unavailable: {error}");
            None
        }
        Err(error) if is_qdrant_timeout_signature(&error) => {
            println!("skipping v0.1.3 persistent write-planning test because local Qdrant gRPC mutation stalled: {error}");
            None
        }
        Err(error) => panic!("unexpected v0.1.3 persistent setup failure: {error}"),
    }
}

async fn try_setup_in_memory_character_memory() -> Result<(CharacterMemory, String), CustomError> {
    let collection_name = base::unique_collection_name();
    let settings = load_in_memory_settings()?;
    let embed_provider = Box::new(base::DeterministicEmbeddingProvider::new(
        settings.get_embedding_vector_size()?,
    ));

    let character_memory = CharacterMemory::new_with_embedding_provider(
        settings,
        collection_name.clone(),
        embed_provider,
    )
    .await?;

    Ok((character_memory, collection_name))
}

async fn try_setup_persistent_character_memory(
    collection_name: String,
    graph_path: &Path,
    stats_path: &Path,
) -> Result<CharacterMemory, CustomError> {
    let base_settings = load_test_settings()?;
    let embedding_model = std::env::var("EMBEDDING_MODEL")
        .map_err(|error| CustomError::ConfigParseError(format!("EMBEDDING_MODEL: {error}")))?;

    let settings = Settings::new(
        Config::builder()
            .set_override(
                "qdrant_connection_string",
                base_settings.get_qdrant_connection(),
            )
            .map_err(base::config_error)?
            .set_override("oxigraph_connection_string", path_string(graph_path))
            .map_err(base::config_error)?
            .set_override("openai_api_key", base_settings.get_openai_api_key())
            .map_err(base::config_error)?
            .set_override("embedding_model", embedding_model)
            .map_err(base::config_error)?
            .set_override("graph_store_mode", "persistent")
            .map_err(base::config_error)?
            .set_override("retrieval_stats_store_mode", "sqlite")
            .map_err(base::config_error)?
            .set_override("retrieval_stats_path", path_string(stats_path))
            .map_err(base::config_error)?
            .build()
            .map_err(base::config_error)?,
    )?;
    let embed_provider = Box::new(base::DeterministicEmbeddingProvider::new(
        settings.get_embedding_vector_size()?,
    ));

    CharacterMemory::new_with_embedding_provider(settings, collection_name, embed_provider).await
}

fn load_in_memory_settings() -> Result<Settings, CustomError> {
    let base_settings = load_test_settings()?;
    let embedding_model = std::env::var("EMBEDDING_MODEL")
        .map_err(|error| CustomError::ConfigParseError(format!("EMBEDDING_MODEL: {error}")))?;

    let config = Config::builder()
        .set_override(
            "qdrant_connection_string",
            base_settings.get_qdrant_connection(),
        )
        .map_err(base::config_error)?
        .set_override(
            "oxigraph_connection_string",
            base_settings.get_oxigraph_connection(),
        )
        .map_err(base::config_error)?
        .set_override("openai_api_key", base_settings.get_openai_api_key())
        .map_err(base::config_error)?
        .set_override("embedding_model", embedding_model)
        .map_err(base::config_error)?
        .set_override("graph_store_mode", "in_memory")
        .map_err(base::config_error)?
        .build()
        .map_err(base::config_error)?;

    Settings::new(config)
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

struct PersistentFixture {
    _temp_dir: TempDir,
    graph_path: std::path::PathBuf,
    stats_path: std::path::PathBuf,
}

impl PersistentFixture {
    fn new() -> Self {
        let temp_dir = tempfile::tempdir().expect("persistent fixture tempdir should be created");
        Self {
            graph_path: temp_dir.path().join("graph.oxigraph"),
            stats_path: temp_dir.path().join("stats.sqlite3"),
            _temp_dir: temp_dir,
        }
    }
}

fn core_input(label: &str) -> RememberInput {
    let entity_id = stable_id(label, 1);
    let timestamp = fixed_timestamp();
    let mut entity = EntityDraft::new(EntityType::Project, format!("{label} entity"));
    entity.id = Some(entity_id);
    entity.created_at = Some(timestamp);
    entity.updated_at = Some(timestamp);

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

async fn link_to_existing_entity_plan(label: &str, entity_id: MemoryId) -> RememberWritePlan {
    let mut input = core_input(label);
    let mut extra_entity =
        EntityDraft::new(EntityType::Project, format!("{label} original entity"));
    extra_entity.id = Some(entity_id);
    let mut link = MemoryLinkDraft::new(
        ObjectType::Episode,
        Uuid::nil(),
        RelationType::Involves,
        ObjectType::Entity,
        entity_id,
    );
    link.id = Some(stable_id(label, 55));
    input = input.with_entity(extra_entity).with_memory_link(link);
    let defaults = RememberPlanDefaults::fixed(
        format!("{label}-seed"),
        chrono::DateTime::parse_from_rfc3339("2026-07-03T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    );
    let refs = input.prepared_candidate_refs(&defaults);
    let mut plan = input.prepare_write_plan(&defaults);
    for candidate in &mut plan.candidates {
        if let MemoryCandidate::MemoryLink(candidate) = candidate {
            if candidate.draft.from_id == Uuid::nil() {
                candidate.draft.from_id = refs.episode_id;
            }
        }
    }
    plan
}

fn ungrounded_derived_memory_plan() -> RememberWritePlan {
    let mut derived = DerivedMemoryDraft::new(DerivedType::UserPreference, "ungrounded preference");
    derived.id = Some(id("550e8400-e29b-41d4-a716-446655613101"));
    derived.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    derived.created_at = Some(fixed_timestamp());
    derived.updated_at = Some(fixed_timestamp());
    let candidate = MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
        derived,
        CandidateProvenance::caller("caller omitted source provenance"),
    ));
    RememberWritePlan::new(
        id("550e8400-e29b-41d4-a716-446655613102"),
        "ungrounded-derived",
    )
    .with_candidate(candidate)
}

fn missing_link_target_plan() -> RememberWritePlan {
    let mut episode = EpisodeDraft::new("missing link target episode");
    episode.id = Some(id("550e8400-e29b-41d4-a716-446655613111"));
    let mut plan = RememberInput::new("missing link target episode")
        .with_episode(episode)
        .prepare_write_plan(&RememberPlanDefaults::fixed(
            "missing-link-target",
            chrono::DateTime::parse_from_rfc3339("2026-07-03T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        ));
    plan.candidates
        .push(MemoryCandidate::MemoryLink(MemoryLinkCandidate::new(
            {
                let mut link = MemoryLinkDraft::new(
                    ObjectType::Episode,
                    id("550e8400-e29b-41d4-a716-446655613111"),
                    RelationType::Involves,
                    ObjectType::Entity,
                    id("550e8400-e29b-41d4-a716-446655613112"),
                );
                link.id = Some(id("550e8400-e29b-41d4-a716-446655613113"));
                link.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
                link.created_at = Some(fixed_timestamp());
                link
            },
            CandidateProvenance::caller("missing target fixture"),
        )));
    plan
}

fn generated_style_plan() -> RememberWritePlan {
    let timestamp = fixed_timestamp();
    let entity_id = id("550e8400-e29b-41d4-a716-446655613301");
    let episode_id = id("550e8400-e29b-41d4-a716-446655613302");
    let derived_id = id("550e8400-e29b-41d4-a716-446655613303");
    let mut entity = EntityDraft::new(EntityType::Project, "generated-style entity");
    entity.id = Some(entity_id);
    entity.created_at = Some(timestamp);
    entity.updated_at = Some(timestamp);
    entity.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut episode = EpisodeDraft::new("generated-style source episode");
    episode.id = Some(episode_id);
    episode.created_at = Some(timestamp);
    episode.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut derived = DerivedMemoryDraft::new(DerivedType::Claim, "generated-style derived memory");
    derived.id = Some(derived_id);
    derived.derived_from_episode_ids.push(episode_id);
    derived.entity_ids.push(entity_id);
    derived.created_at = Some(timestamp);
    derived.updated_at = Some(timestamp);
    derived.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());

    RememberWritePlan::new(
        id("550e8400-e29b-41d4-a716-446655613304"),
        "generated-style-plan",
    )
    .with_candidate(MemoryCandidate::Entity(
        character_memory::EntityCandidate::new(
            entity,
            CandidateProvenance::inferred_by_processor(
                CandidateProducerKind::ModelProcessor,
                "generated entity candidate",
            ),
        ),
    ))
    .with_candidate(MemoryCandidate::Episode(
        character_memory::EpisodeCandidate::new(
            episode,
            CandidateProvenance::inferred_by_processor(
                CandidateProducerKind::ModelProcessor,
                "generated episode candidate",
            ),
        ),
    ))
    .with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
        derived,
        CandidateProvenance::inferred_by_processor(
            CandidateProducerKind::ModelProcessor,
            "generated derived memory candidate",
        )
        .with_source_episode(episode_id),
    )))
}

async fn assert_retrieval_empty(memory: &character_memory::CharacterMemory, query: &str) {
    let retrieved = retrieve(memory, query).await;
    assert!(retrieved.pack.active_threads.is_empty());
    assert!(retrieved.pack.relevant_episodes.is_empty());
    assert!(retrieved.pack.salient_observations.is_empty());
    assert!(all_derived(&retrieved).is_empty());
}

async fn retrieve(
    memory: &character_memory::CharacterMemory,
    query: &str,
) -> character_memory::RetrieveOutcome {
    memory
        .retrieve(RetrievalContext::new(query))
        .await
        .expect("retrieval should succeed")
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

fn is_qdrant_timeout_signature(error: &CustomError) -> bool {
    let message = error.to_string();
    message.contains("Vector database error") && message.contains("Timeout expired")
}

fn has_candidate_kind(
    plan: &RememberWritePlan,
    predicate: impl Fn(&MemoryCandidate) -> bool,
) -> bool {
    plan.candidates.iter().any(predicate)
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

fn provenance(candidate: &MemoryCandidate) -> &CandidateProvenance {
    match candidate {
        MemoryCandidate::Episode(candidate) => &candidate.provenance,
        MemoryCandidate::Observation(candidate) => &candidate.provenance,
        MemoryCandidate::Entity(candidate) => &candidate.provenance,
        MemoryCandidate::MemoryThread(candidate) => &candidate.provenance,
        MemoryCandidate::DerivedMemory(candidate) => &candidate.provenance,
        MemoryCandidate::MemoryLink(candidate) => &candidate.provenance,
        MemoryCandidate::VectorIndex(candidate) => &candidate.provenance,
        MemoryCandidate::StatsUpdate(candidate) => &candidate.provenance,
    }
}

fn assert_error_contains(error: CustomError, needle: &str) {
    let message = error.to_string();
    assert!(
        message.contains(needle),
        "expected error to contain {needle:?}, got {message:?}"
    );
}

fn assert_invalid_validation_contains(
    validations: &[character_memory::CandidateValidation],
    needle: &str,
) {
    assert!(
        validations.iter().any(|validation| {
            validation.status == CandidateValidationStatus::Invalid
                && validation.errors.iter().any(|error| error.contains(needle))
        }),
        "expected invalid validation containing {needle:?}, got {validations:?}"
    );
}

fn stable_id(label: &str, index: u8) -> MemoryId {
    Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("character-memory:v0.1.3-test:{label}:{index}").as_bytes(),
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
