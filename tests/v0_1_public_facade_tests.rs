use character_memory::{
    CorrectMemoryDraft, CorrectionTarget, CustomError, DerivedMemoryDraft, DerivedType,
    EpisodeDraft, ForgetMemoryDraft, LifecycleTargetRef, MemoryId, MemoryObjectDraft,
    ObservationDraft, RememberDraft, ReplacementDerivedMemoryDraft, RetrievalContext,
    SourceProvenanceReference,
};
use uuid::Uuid;

mod test_utils;

#[tokio::test]
async fn public_remember_and_retrieve_use_graph_authoritative_path() {
    let (memory, collection_name) = match test_utils::try_setup_character_memory().await {
        Ok(setup) => setup,
        Err(CustomError::QdrantError(error)) => {
            println!("skipping live public facade test because Qdrant is unavailable: {error}");
            return;
        }
        Err(error) => panic!("unexpected live public facade setup failure: {error}"),
    };
    let episode_id = id("550e8400-e29b-41d4-a716-446655440101");
    let observation_id = id("550e8400-e29b-41d4-a716-446655440102");
    let derived_id = id("550e8400-e29b-41d4-a716-446655440103");

    let mut episode = EpisodeDraft::new("The user prefers deterministic public facade tests.");
    episode.id = Some(episode_id);
    episode.raw_ref = Some("raw://integration/public-facade#episode".to_owned());

    let mut observation = ObservationDraft::new(
        episode_id,
        "Please keep public facade tests deterministic and graph-authoritative.",
    );
    observation.id = Some(observation_id);
    observation.raw_ref = Some("raw://integration/public-facade#turn-1".to_owned());

    let mut preference = DerivedMemoryDraft::new(
        DerivedType::UserPreference,
        "The user prefers deterministic public facade tests.",
    )
    .with_source_episode(episode_id)
    .with_source_observation(observation_id);
    preference.id = Some(derived_id);

    let outcome = memory
        .remember(RememberDraft::new([
            MemoryObjectDraft::Episode(episode),
            MemoryObjectDraft::Observation(observation),
            MemoryObjectDraft::DerivedMemory(preference),
        ]))
        .await
        .expect("remember should use public graph/vector facade");

    assert!(outcome.persisted_object_ids.contains(&episode_id));
    assert!(outcome.persisted_object_ids.contains(&observation_id));
    assert!(outcome.persisted_object_ids.contains(&derived_id));
    assert!(outcome.vector_indexing_failure.is_none());

    let retrieved = memory
        .retrieve(RetrievalContext::new("deterministic public facade tests").with_trace())
        .await
        .expect("retrieve should use public graph/vector facade");

    assert!(retrieved
        .pack
        .preferences
        .iter()
        .any(|included| included.memory.id == derived_id));
    assert!(retrieved
        .trace
        .as_ref()
        .is_some_and(|trace| !trace.vector_candidates.is_empty()));

    test_utils::cleanup_collection(&collection_name).await;
}

#[tokio::test]
async fn public_correct_and_forget_hide_stale_memories_from_normal_retrieval() {
    let (memory, collection_name) = match test_utils::try_setup_character_memory().await {
        Ok(setup) => setup,
        Err(CustomError::QdrantError(error)) => {
            println!(
                "skipping live public lifecycle facade test because Qdrant is unavailable: {error}"
            );
            return;
        }
        Err(error) => panic!("unexpected live public lifecycle setup failure: {error}"),
    };
    let episode_id = id("550e8400-e29b-41d4-a716-446655440201");
    let old_id = id("550e8400-e29b-41d4-a716-446655440202");
    let replacement_id = id("550e8400-e29b-41d4-a716-446655440203");

    let mut episode = EpisodeDraft::new("The user corrected a public facade preference.");
    episode.id = Some(episode_id);

    let mut old_preference = DerivedMemoryDraft::new(
        DerivedType::UserPreference,
        "The user prefers stale public facade behavior.",
    )
    .with_source_episode(episode_id);
    old_preference.id = Some(old_id);

    memory
        .remember(RememberDraft::new([
            MemoryObjectDraft::Episode(episode),
            MemoryObjectDraft::DerivedMemory(old_preference),
        ]))
        .await
        .expect("initial remember should succeed");

    let mut replacement = ReplacementDerivedMemoryDraft::new(
        DerivedType::Correction,
        "The user prefers graph-authoritative public facade behavior.",
    )
    .with_source_episode(episode_id)
    .with_superseded_memory(old_id);
    replacement.id = Some(replacement_id);
    replacement.original_source_provenance = SourceProvenanceReference::episode(episode_id);
    replacement.correction_origin_provenance = SourceProvenanceReference::episode(episode_id);

    let mut correction = CorrectMemoryDraft::new(
        CorrectionTarget::derived_memory(old_id),
        "Correct stale public facade behavior.",
    )
    .with_replacement(replacement)
    .with_superseded_derived_memory(old_id);
    correction.correction_origin = SourceProvenanceReference::episode(episode_id);

    memory
        .correct(correction)
        .await
        .expect("public correct should supersede old memory");

    let retrieved = memory
        .retrieve(RetrievalContext::new(
            "graph-authoritative public facade behavior",
        ))
        .await
        .expect("retrieve after correction should succeed");
    assert!(retrieved
        .pack
        .derived_memories
        .iter()
        .chain(retrieved.pack.preferences.iter())
        .any(|included| included.memory.id == replacement_id));
    assert!(!retrieved
        .pack
        .derived_memories
        .iter()
        .chain(retrieved.pack.preferences.iter())
        .any(|included| included.memory.id == old_id));

    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::derived_memory(replacement_id),
            "Suppress corrected public facade memory.",
        ))
        .await
        .expect("public forget should suppress replacement");

    let after_forget = memory
        .retrieve(RetrievalContext::new(
            "graph-authoritative public facade behavior",
        ))
        .await
        .expect("retrieve after forget should succeed");
    assert!(!after_forget
        .pack
        .derived_memories
        .iter()
        .chain(after_forget.pack.preferences.iter())
        .any(|included| included.memory.id == replacement_id));

    test_utils::cleanup_collection(&collection_name).await;
}

fn id(value: &str) -> MemoryId {
    Uuid::parse_str(value).unwrap()
}
