use character_memory::{
    CorrectMemoryDraft, CorrectionTarget, CustomError, DerivedMemoryDraft, DerivedType,
    EpisodeDraft, ForgetMemoryDraft, LifecycleTargetRef, MemoryId, MemoryObjectDraft,
    ObservationDraft, RememberDraft, ReplacementDerivedMemoryDraft, RetrievalContext,
    SourceProvenanceReference,
};
use uuid::Uuid;

#[path = "support/basic.rs"]
mod test_utils;

#[tokio::test]
async fn public_remember_and_retrieve_use_graph_authoritative_path() {
    let (memory, collection_name) = match test_utils::try_setup_character_memory().await {
        Ok(setup) => setup,
        Err(CustomError::QdrantError(error)) if test_utils::is_qdrant_unavailable_error(&error) => {
            println!("skipping live public facade test because Qdrant is unavailable: {error}");
            return;
        }
        Err(error) => panic!("unexpected live public facade setup failure: {error}"),
    };

    let test_result = async {
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
            .map_err(|error| format!("remember should use public graph/vector facade: {error}"))?;

        ensure(
            outcome.persisted_object_ids.contains(&episode_id),
            "remember should persist episode id",
        )?;
        ensure(
            outcome.persisted_object_ids.contains(&observation_id),
            "remember should persist observation id",
        )?;
        ensure(
            outcome.persisted_object_ids.contains(&derived_id),
            "remember should persist derived memory id",
        )?;
        ensure(
            outcome.vector_indexing_failure.is_none(),
            "remember should index vectors without partial failure",
        )?;

        let retrieved = memory
            .retrieve(RetrievalContext::new("deterministic public facade tests").with_trace())
            .await
            .map_err(|error| format!("retrieve should use public graph/vector facade: {error}"))?;

        ensure(
            retrieved
                .pack
                .preferences
                .iter()
                .any(|included| included.memory.id == derived_id),
            "retrieval should include the derived preference",
        )?;
        ensure(
            retrieved
                .trace
                .as_ref()
                .is_some_and(|trace| !trace.vector_candidates.is_empty()),
            "retrieval trace should include vector candidates",
        )?;

        Ok::<(), String>(())
    }
    .await;
    test_utils::cleanup_collection(&collection_name).await;
    test_result.expect("live public facade test should pass");
}

#[tokio::test]
async fn public_correct_and_forget_hide_stale_memories_from_normal_retrieval() {
    let (memory, collection_name) = match test_utils::try_setup_character_memory().await {
        Ok(setup) => setup,
        Err(CustomError::QdrantError(error)) if test_utils::is_qdrant_unavailable_error(&error) => {
            println!(
                "skipping live public lifecycle facade test because Qdrant is unavailable: {error}"
            );
            return;
        }
        Err(error) => panic!("unexpected live public lifecycle setup failure: {error}"),
    };

    let test_result = async {
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
            .map_err(|error| format!("initial remember should succeed: {error}"))?;

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
            .map_err(|error| format!("public correct should supersede old memory: {error}"))?;

        let retrieved = memory
            .retrieve(RetrievalContext::new(
                "graph-authoritative public facade behavior",
            ))
            .await
            .map_err(|error| format!("retrieve after correction should succeed: {error}"))?;
        ensure(
            retrieved
                .pack
                .derived_memories
                .iter()
                .chain(retrieved.pack.preferences.iter())
                .any(|included| included.memory.id == replacement_id),
            "retrieval after correction should include replacement memory",
        )?;
        ensure(
            !retrieved
                .pack
                .derived_memories
                .iter()
                .chain(retrieved.pack.preferences.iter())
                .any(|included| included.memory.id == old_id),
            "retrieval after correction should hide old memory",
        )?;

        memory
            .forget(ForgetMemoryDraft::suppress(
                LifecycleTargetRef::derived_memory(replacement_id),
                "Suppress corrected public facade memory.",
            ))
            .await
            .map_err(|error| format!("public forget should suppress replacement: {error}"))?;

        let after_forget = memory
            .retrieve(RetrievalContext::new(
                "graph-authoritative public facade behavior",
            ))
            .await
            .map_err(|error| format!("retrieve after forget should succeed: {error}"))?;
        ensure(
            !after_forget
                .pack
                .derived_memories
                .iter()
                .chain(after_forget.pack.preferences.iter())
                .any(|included| included.memory.id == replacement_id),
            "retrieval after forget should hide suppressed replacement",
        )?;

        Ok::<(), String>(())
    }
    .await;
    test_utils::cleanup_collection(&collection_name).await;
    test_result.expect("live public lifecycle facade test should pass");
}

fn id(value: &str) -> MemoryId {
    Uuid::parse_str(value).unwrap()
}

fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}
