use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::api::types::*;
use crate::domain::*;
use crate::models::vector::EmbeddingInput;
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::graph_authority::GraphObjectQuery;
use crate::ports::retrieval_stats::RetrievalStatsCounterKey;
use crate::test_support::{
    in_memory_graph_store, DeterministicMemoryEmbedder, TemporaryVectorCandidateStore,
};
use crate::{CharacterMemory, CustomError};

fn time() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-09-20T10:00:00.123456789Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn words_scene() -> Scene {
    let mut scene = Scene::at(time());
    scene.setting.words = Some("  窓のそば\nquiet café  ".to_owned());
    scene.participants = vec![
        SceneParticipant {
            name: Some("  Alice  ".to_owned()),
            ..Default::default()
        },
        SceneParticipant {
            description: Some("a visitor in blue".to_owned()),
            ..Default::default()
        },
        SceneParticipant {
            name: Some("  Alice  ".to_owned()),
            ..Default::default()
        },
    ];
    scene
        .custom_values
        .insert("session".to_owned(), "  session/42  ".to_owned());
    scene
        .custom_values
        .insert("empty".to_owned(), String::new());
    scene
}

// Observe provider input while using the same deterministic embedder and real stores.
struct RecordingEmbedder(Arc<Mutex<Vec<EmbeddingInput>>>);

#[async_trait]
impl MemoryEmbedder for RecordingEmbedder {
    async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
        self.0.lock().unwrap().push(input.clone());
        DeterministicMemoryEmbedder::new(8).embed(input).await
    }
    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        self.0.lock().unwrap().extend_from_slice(inputs);
        DeterministicMemoryEmbedder::new(8)
            .embed_batch(inputs)
            .await
    }
}

async fn memory() -> (CharacterMemory, Arc<Mutex<Vec<EmbeddingInput>>>) {
    let inputs = Arc::new(Mutex::new(Vec::new()));
    let memory = CharacterMemory::from_parts(
        Box::new(in_memory_graph_store()),
        Box::new(TemporaryVectorCandidateStore::open(8).await),
        Box::new(RecordingEmbedder(inputs.clone())),
    );
    (memory, inputs)
}

fn episode_draft(id: u128, scene: Option<Scene>) -> EpisodeDraft {
    let mut draft = EpisodeDraft::new("An experience");
    draft.id = Some(MemoryId::from_u128(id));
    draft.created_at = Some(time());
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    draft.scene = scene;
    draft
}

fn episode_plan(draft: EpisodeDraft) -> RememberWritePlan {
    RememberWritePlan::new().with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
        draft,
        CandidateProvenance::caller("experience"),
    )))
}

async fn objects(memory: &CharacterMemory, types: Vec<ObjectType>) -> Vec<MemoryObject> {
    memory
        .memory_composition
        .graph_store
        .query_objects(&GraphObjectQuery::by_types(types, None))
        .await
        .unwrap()
}

fn assert_issue(error: CustomError, expected: CandidateValidationIssue) {
    let CustomError::WritePlanValidationRejected { validations } = error else {
        panic!("unexpected error: {error:?}")
    };
    assert!(
        validations
            .iter()
            .flat_map(|v| &v.errors)
            .any(|issue| issue == &expected),
        "missing {expected:?}: {validations:?}"
    );
}

#[tokio::test]
async fn scene_words_round_trip_through_remember_and_authored_plan_without_inference() {
    for direct in [false, true] {
        let (memory, inputs) = memory().await;
        let mut scene = words_scene();
        scene.participants[0].description = Some(" \t ".to_owned());
        scene.participants[1].name = Some(String::new());
        let mut supplied = scene.clone();
        supplied.participants.insert(0, SceneParticipant::default());
        supplied.participants.push(SceneParticipant {
            name: Some(" \n ".to_owned()),
            description: Some("\t".to_owned()),
            key: None,
        });
        let draft = episode_draft(8101, Some(supplied.clone()));
        let outcome = if direct {
            let plan = episode_plan(draft).with_candidate(MemoryCandidate::VectorIndex(
                VectorIndexCandidate::new(
                    MemoryObjectRef::new(ObjectType::Episode, MemoryId::from_u128(8101)),
                    CandidateProvenance::caller("summary"),
                ),
            ));
            let plan: RememberWritePlan =
                serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
            let first = memory
                .commit(plan.clone(), CommitOptions::default())
                .await
                .unwrap();
            let replay = memory.commit(plan, CommitOptions::default()).await.unwrap();
            assert_eq!(first.persisted_object_ids, replay.persisted_object_ids);
            first
        } else {
            memory
                .remember(
                    RememberInput::new("An experience")
                        .with_scene(supplied.clone())
                        .with_episode(episode_draft(8101, None)),
                    RememberOptions::default(),
                )
                .await
                .unwrap()
        };
        let saved = objects(&memory, vec![ObjectType::Episode]).await;
        let [MemoryObject::Episode(episode)] = saved.as_slice() else {
            panic!("one episode expected")
        };
        assert_eq!(episode.scene, scene);
        assert!(
            objects(&memory, vec![ObjectType::Entity, ObjectType::DerivedMemory])
                .await
                .is_empty()
        );
        assert!(outcome.persisted_link_ids.is_empty());
        assert_eq!(
            outcome.vector_indexed_object_ids.len(),
            if direct { 1 } else { 2 }
        );
        let recorded = inputs.lock().unwrap().clone();
        assert_eq!(recorded.len(), if direct { 6 } else { 4 });
        for input in recorded {
            assert_eq!(
                input.text,
                match input.surface {
                    VectorSurface::Summary => "Episode summary: An experience",
                    VectorSurface::SceneSetting => "窓のそば quiet café",
                    VectorSurface::SceneParticipants => "Alice\na visitor in blue\nAlice",
                    VectorSurface::Text => "Observation excerpt: An experience",
                    other => panic!("unexpected embedding: {other:?}"),
                }
            );
        }
        let result = memory
            .retrieve(RetrievalContext::default().with_scene(supplied))
            .await
            .unwrap();
        assert_eq!(result.scene, scene);
        memory.close().await.unwrap();
    }
}

#[tokio::test]
async fn scene_without_words_keeps_the_exact_legacy_embedding_text() {
    for blank_words in [false, true] {
        let (memory, inputs) = memory().await;
        let participant = MemoryId::from_u128(8700);
        let mut entity = EntityDraft::new();
        entity.id = Some(participant);
        let mut scene = Scene::at(time());
        scene.setting.key = Some("never embed this setting key".to_owned());
        scene.custom_values.insert(
            "session".to_owned(),
            "never embed this custom value".to_owned(),
        );
        scene.participants.push(SceneParticipant {
            key: Some(participant),
            ..Default::default()
        });
        if blank_words {
            scene.setting.words = Some(" \n ".to_owned());
            scene.participants[0].name = Some(" \t ".to_owned());
            scene.participants[0].description = Some(String::new());
        }
        memory
            .remember(
                RememberInput::new("  An\n experience  ")
                    .with_scene(scene.clone())
                    .with_entity(entity),
                RememberOptions::default(),
            )
            .await
            .unwrap();
        let saved = objects(&memory, vec![ObjectType::Episode]).await;
        let [MemoryObject::Episode(episode)] = saved.as_slice() else {
            panic!("one episode expected")
        };
        assert_eq!(episode.scene, scene);
        let recorded = inputs.lock().unwrap().clone();
        assert_eq!(recorded.len(), 2);
        assert_eq!(
            recorded
                .iter()
                .find(|input| input.object_type == Some(ObjectType::Episode))
                .unwrap()
                .text,
            "Episode summary: An experience"
        );
        assert_eq!(
            recorded
                .iter()
                .find(|input| input.object_type == Some(ObjectType::Observation))
                .unwrap()
                .text,
            "Observation excerpt: An experience"
        );
        memory.close().await.unwrap();
    }
}

struct TextEmbedder([&'static str; 2]);
#[async_trait]
impl MemoryEmbedder for TextEmbedder {
    async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
        Ok(vec![
            f32::from(input.text.contains(self.0[0])),
            f32::from(input.text.contains(self.0[1])),
            1.0,
        ])
    }

    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        let mut embeddings = Vec::new();
        for input in inputs {
            embeddings.push(self.embed(input).await?);
        }
        Ok(embeddings)
    }
}

#[tokio::test]
async fn remember_keeps_distinct_vectors_for_episode_and_observation_with_the_same_uuid() {
    let memory = CharacterMemory::from_parts(
        Box::new(in_memory_graph_store()),
        Box::new(TemporaryVectorCandidateStore::open(3).await),
        Box::new(TextEmbedder(["volcano", "harbor"])),
    );
    let id = MemoryId::from_u128(8801);
    let mut episode = episode_draft(id.as_u128(), Some(Scene::at(time())));
    episode.summary = "A volcano erupted".to_owned();
    let mut observation = ObservationDraft::new(id, "Ships arrived at the harbor");
    observation.id = Some(id);
    let outcome = memory
        .remember(
            RememberInput::new("A volcano erupted")
                .with_episode(episode)
                .with_observation(observation),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    assert!(outcome.vector_indexing_failure.is_none());
    for (topic, object_type, surface) in [
        ("volcano", ObjectType::Episode, VectorSurface::Summary),
        ("harbor", ObjectType::Observation, VectorSurface::Text),
    ] {
        let mut context = RetrievalContext::new(topic).with_trace();
        context.object_type_defaults = vec![object_type];
        context.graph_limits.max_depth = 0;
        let result = memory.retrieve(context).await.unwrap();
        let trace = result.trace.unwrap();
        let [candidate] = trace.vector_candidates.as_slice() else {
            panic!(
                "one {object_type:?} vector expected: {:?}",
                trace.vector_candidates
            );
        };
        assert_eq!(candidate.object, MemoryObjectRef::new(object_type, id));
        assert_eq!(candidate.surface, surface);
        assert!(candidate.score > 0.9999, "{topic}: {candidate:?}");
    }
    memory.close().await.unwrap();
}

#[tokio::test]
async fn setting_words_recall_the_episode_when_the_summary_does_not_name_the_place() {
    // A lexical provider makes the recall test depend on words, not hash collisions.
    struct PlaceEmbedder;
    #[async_trait]
    impl MemoryEmbedder for PlaceEmbedder {
        async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
            Ok(vec![
                1.0,
                input
                    .text
                    .split_whitespace()
                    .filter(|word| *word == "observatory")
                    .count() as f32,
            ])
        }
        async fn embed_batch(
            &self,
            inputs: &[EmbeddingInput],
        ) -> Result<Vec<Vec<f32>>, CustomError> {
            let mut embeddings = Vec::new();
            for input in inputs {
                embeddings.push(self.embed(input).await?);
            }
            Ok(embeddings)
        }
    }
    let memory = CharacterMemory::from_parts(
        Box::new(in_memory_graph_store()),
        Box::new(TemporaryVectorCandidateStore::open(2).await),
        Box::new(PlaceEmbedder),
    );
    let control = MemoryId::from_u128(8701);
    let situated = MemoryId::from_u128(8702);
    for (id, words) in [(control, None), (situated, Some("observatory"))] {
        let mut scene = Scene::at(time());
        scene.setting.words = words.map(str::to_owned);
        let mut episode = episode_draft(id.as_u128(), Some(scene));
        episode.summary = "We made a decision".to_owned();
        let outcome = memory
            .remember(
                RememberInput::new("We made a decision").with_episode(episode),
                RememberOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(outcome.vector_indexed_object_ids.len(), 2);
    }
    let mut query = RetrievalContext::default();
    query.scene.setting.words = Some("observatory".to_owned());
    query.object_type_defaults = vec![ObjectType::Episode];
    query.candidate_limits.max_vector_candidates = 1;
    query.candidate_limits.max_graph_roots = 1;
    query.graph_limits.max_depth = 0;
    let recalled = memory.retrieve(query).await.unwrap();
    assert_eq!(recalled.pack.relevant_episodes.len(), 1);
    let episode = &recalled.pack.relevant_episodes[0];
    assert_eq!(episode.id, situated);
    assert!(!episode.summary.contains("observatory"));
    assert_eq!(episode.scene.setting.words.as_deref(), Some("observatory"));
    memory.close().await.unwrap();
}

#[tokio::test]
async fn scene_override_preserves_participants_involvement_threads_interval_and_observation() {
    let (memory, inputs) = memory().await;
    let episode_id = MemoryId::from_u128(8220);
    let observation_id = MemoryId::from_u128(8221);
    let present = MemoryId::from_u128(8201);
    let involved = MemoryId::from_u128(8202);
    let existing = MemoryId::from_u128(8203);
    let thread_ids = [MemoryId::from_u128(8211), MemoryId::from_u128(8212)];
    let entity = |id| {
        let mut draft = EntityDraft::new();
        draft.id = Some(id);
        draft
    };
    memory
        .remember(
            RememberInput::new("Earlier experience").with_entity(entity(existing)),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    let mut scene = words_scene();
    scene.setting.key = Some("room/42".to_owned());
    scene.participants.extend([
        SceneParticipant {
            key: Some(present),
            name: Some("Mira".to_owned()),
            description: Some("the host".to_owned()),
        },
        SceneParticipant {
            key: Some(existing),
            ..Default::default()
        },
        SceneParticipant {
            key: Some(present),
            ..Default::default()
        },
    ]);
    let episode = episode_draft(8220, Some(scene.clone()));
    let mut observation = ObservationDraft::new(episode_id, "Explicitly timed statement");
    observation.id = Some(observation_id);
    observation.observed_at = Some(time() + chrono::Duration::seconds(10));
    observation.speaker_entity_id = Some(involved);
    let ended_at = time() + chrono::Duration::minutes(5);
    let mut overridden = Scene::at(time() + chrono::Duration::days(1));
    overridden
        .custom_values
        .insert("input-only".to_owned(), "must not merge".to_owned());
    overridden.participants.push(SceneParticipant {
        key: Some(MemoryId::from_u128(9999)),
        ..Default::default()
    });
    let mut input = RememberInput::new("An experience")
        .with_scene(overridden)
        .with_episode(episode)
        .with_observation(observation.clone())
        .with_ended_at(ended_at)
        .with_entity(entity(present))
        .with_entity(entity(involved))
        .with_entity_id(involved);
    for id in thread_ids {
        let mut thread = MemoryThreadDraft::new("A thread", "An ongoing topic");
        thread.id = Some(id);
        input = input.with_memory_thread(thread).with_thread_id(id);
    }
    let outcome = memory
        .remember(input, RememberOptions::default())
        .await
        .unwrap();
    assert!(inputs
        .lock()
        .unwrap()
        .iter()
        .any(|input| input.object_id == Some(episode_id)
            && input.surface == VectorSurface::SceneParticipants
            && input.text.ends_with("\nMira, the host")));

    let graph = &memory.memory_composition.graph_store;
    let saved = graph
        .query_objects(&GraphObjectQuery::by_ids(vec![episode_id, observation_id]))
        .await
        .unwrap();
    let [MemoryObject::Episode(saved_episode), MemoryObject::Observation(saved_observation)] =
        saved.as_slice()
    else {
        panic!("expected the source episode and observation")
    };
    assert_eq!(saved_episode.scene, scene);
    assert_eq!(saved_episode.ended_at, Some(ended_at));
    assert_eq!(saved_observation.observed_at, observation.observed_at);
    assert_eq!(saved_observation.speaker_entity_id, Some(involved));
    let links = graph
        .query_links_by_ids(&outcome.persisted_link_ids)
        .await
        .unwrap();
    for (from, relation, to) in [
        (episode_id, RelationType::Involves, involved),
        (observation_id, RelationType::Mentions, present),
        (observation_id, RelationType::Mentions, existing),
        (observation_id, RelationType::PartOfThread, thread_ids[0]),
        (observation_id, RelationType::PartOfThread, thread_ids[1]),
    ] {
        assert!(links
            .iter()
            .any(|link| (link.from_id, link.relation, link.to_id) == (from, relation, to)));
    }
    for participant in [present, existing] {
        let counter = memory
            .memory_composition
            .stats_store
            .counter(&RetrievalStatsCounterKey {
                entity_id: participant,
                relation_kind: RelationType::Involves,
                object_type: ObjectType::Episode,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(counter.total_count, 1, "repeated keys must not count twice");
    }
    memory.close().await.unwrap();
}

#[tokio::test]
async fn omitted_scene_time_uses_preparation_instant_instead_of_episode_creation_time() {
    let (memory, _) = memory().await;
    let before = Utc::now();
    let plan = memory
        .prepare(
            RememberInput::new("An experience").with_episode(episode_draft(8301, None)),
            PrepareOptions::default(),
        )
        .await
        .unwrap();
    let after = Utc::now();
    let scene = plan
        .candidates
        .iter()
        .find_map(|candidate| match candidate {
            MemoryCandidate::Episode(candidate) => candidate.draft.scene.clone(),
            _ => None,
        })
        .unwrap();
    assert!(scene.time >= before && scene.time <= after);
    assert_ne!(scene.time, time());
    assert_eq!(scene, Scene::at(scene.time));
    let serialized = serde_json::to_string(&plan).unwrap();
    for _ in 0..2 {
        memory
            .commit(
                serde_json::from_str(&serialized).unwrap(),
                CommitOptions::default(),
            )
            .await
            .unwrap();
    }
    for object in objects(&memory, vec![ObjectType::Episode, ObjectType::Observation]).await {
        match object {
            MemoryObject::Episode(episode) => {
                assert_eq!(episode.scene, scene);
                assert_eq!(episode.created_at, time());
            }
            MemoryObject::Observation(observation) => {
                assert_eq!(observation.observed_at, Some(scene.time))
            }
            _ => unreachable!(),
        }
    }
    memory.close().await.unwrap();
}

#[tokio::test]
async fn writes_reject_missing_scene_and_unknown_keys() {
    let (memory, inputs) = memory().await;
    let with_vector = |draft| {
        episode_plan(draft).with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
            MemoryObjectRef::new(ObjectType::Episode, MemoryId::from_u128(8401)),
            CandidateProvenance::caller("summary"),
        )))
    };
    assert_eq!(
        episode_draft(8401, None).into_domain(),
        Err(DomainValidationError::MissingScene)
    );
    let error = memory
        .commit(
            with_vector(episode_draft(8401, None)),
            CommitOptions::default(),
        )
        .await
        .unwrap_err();
    assert_issue(error, CandidateValidationIssue::MissingScene);
    // Missing scene is rejected while materializing request-owned values.
    assert!(inputs.lock().unwrap().is_empty());
    let unknown = MemoryId::from_u128(8499);
    let mut scene = Scene::at(time());
    scene.participants.push(SceneParticipant {
        key: Some(unknown),
        ..Default::default()
    });
    let error = memory
        .commit(
            with_vector(episode_draft(8401, Some(scene))),
            CommitOptions::default(),
        )
        .await
        .unwrap_err();
    assert_issue(
        error,
        CandidateValidationIssue::UnknownObjectRef {
            role: CandidateReferenceRole::SceneParticipant,
            referenced: MemoryObjectRef::new(ObjectType::Entity, unknown),
        },
    );
    // Key validation needs the graph inside the write turn, after embedding.
    assert_eq!(inputs.lock().unwrap().len(), 1);
    let recalled = memory
        .retrieve(RetrievalContext::new("An experience").with_trace())
        .await
        .unwrap();
    assert!(recalled.trace.unwrap().vector_candidates.is_empty());
    assert!(objects(
        &memory,
        vec![
            ObjectType::Episode,
            ObjectType::Observation,
            ObjectType::Entity
        ]
    )
    .await
    .is_empty());
    memory.close().await.unwrap();
}

#[tokio::test]
async fn default_correction_embeds_its_rationale_and_is_recalled_by_content() {
    let memory = CharacterMemory::from_parts(
        Box::new(in_memory_graph_store()),
        Box::new(TemporaryVectorCandidateStore::open(3).await),
        Box::new(TextEmbedder(["Monday", "Tuesday"])),
    );
    let old_id = MemoryId::from_u128(8902);
    let mut old = DerivedMemoryDraft::new(DerivedType::Claim, "The meeting is Monday.");
    old.id = Some(old_id);
    memory
        .remember(
            RememberInput::new("We discussed the meeting.")
                .with_episode(episode_draft(8901, Some(Scene::at(time()))))
                .with_derived_memory(old),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    let mut correction = CorrectMemoryDraft::new(
        CorrectionTarget::derived_memory(old_id),
        "The meeting is Tuesday.",
    );
    correction.correction_origin = SourceProvenanceReference {
        episode_ids: vec![],
        observation_ids: vec![],
        external_refs: vec![ExternalSourceReference::source("calendar:update")],
    };
    let outcome = memory.correct(correction).await.unwrap();
    assert!(outcome.vector_maintenance_failure.is_none());
    let [replacement] = outcome.graph_mutated_object_ids.as_slice() else {
        panic!("one default replacement expected")
    };
    assert!(outcome.vector_maintained_object_ids.contains(replacement));
    let mut query = RetrievalContext::new("Tuesday").with_trace();
    query.object_type_defaults = vec![ObjectType::DerivedMemory];
    query.graph_limits.max_depth = 0;
    let result = memory.retrieve(query).await.unwrap();
    assert!(result
        .trace
        .unwrap()
        .vector_candidates
        .iter()
        .any(|candidate| candidate.object == *replacement && candidate.score > 0.9999));
    assert!(result
        .pack
        .derived_memories
        .iter()
        .any(|entry| entry.memory.id == replacement.id
            && entry.memory.text == "The meeting is Tuesday."));
    memory.close().await.unwrap();
}

#[tokio::test]
async fn source_correction_uses_setting_key_and_preserves_every_source_scene() {
    for observation_target in [false, true] {
        let (memory, _) = memory().await;
        let mut original_scene = words_scene();
        original_scene.setting.key = Some("channel/original".to_owned());
        let correction_scene = Scene::at(time() + chrono::Duration::days(1));
        let original = MemoryId::from_u128(8501);
        let observation = MemoryId::from_u128(8502);
        let correction = MemoryId::from_u128(8503);
        let old = MemoryId::from_u128(8504);
        let new = MemoryId::from_u128(8505);
        let mut source_observation = ObservationDraft::new(original, "Original statement");
        source_observation.id = Some(observation);
        let mut belief = DerivedMemoryDraft::new(DerivedType::Claim, "Original interpretation")
            .with_source_episode(original)
            .with_source_observation(observation);
        belief.id = Some(old);
        memory
            .remember(
                RememberInput::new("An experience")
                    .with_episode(episode_draft(8501, Some(original_scene.clone())))
                    .with_observation(source_observation)
                    .with_derived_memory(belief),
                RememberOptions::default(),
            )
            .await
            .unwrap();
        memory
            .remember(
                RememberInput::new("Correction experience")
                    .with_episode(episode_draft(8503, Some(correction_scene))),
                RememberOptions::default(),
            )
            .await
            .unwrap();
        let sources_before =
            objects(&memory, vec![ObjectType::Episode, ObjectType::Observation]).await;
        let target = |key| {
            if observation_target {
                SourceObjectCorrectionTarget::Observation {
                    id: observation,
                    original_raw_ref: None,
                    original_setting_key: Some(key),
                }
            } else {
                SourceObjectCorrectionTarget::Episode {
                    id: original,
                    original_raw_ref: None,
                    original_setting_key: Some(key),
                }
            }
        };
        let mut replacement =
            ReplacementDerivedMemoryDraft::new(DerivedType::Correction, "Revised interpretation")
                .with_source_episode(original)
                .with_source_episode(correction)
                .with_source_observation(observation);
        replacement.id = Some(new);
        replacement.original_source_provenance = SourceProvenanceReference::episode(original);
        replacement.correction_origin_provenance = SourceProvenanceReference::episode(correction);
        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::source_object(target(original_scene.setting.words.clone().unwrap())),
            "Correct the interpretation",
        )
        .with_replacement(replacement);
        draft.correction_origin = SourceProvenanceReference::episode(correction);
        let error = memory.correct(draft.clone()).await.unwrap_err();
        assert!(
            matches!(error, CustomError::OriginalSourceReferenceMismatch { kind: SourceReferenceKind::SettingKey, provided, stored, .. } if Some(provided.as_str()) == original_scene.setting.words.as_deref() && stored == original_scene.setting.key)
        );
        draft.targets = vec![CorrectionTarget::source_object(target(
            original_scene.setting.key.clone().unwrap(),
        ))];
        memory.correct(draft).await.unwrap();
        assert_eq!(
            objects(&memory, vec![ObjectType::Episode, ObjectType::Observation]).await,
            sources_before
        );
        let beliefs = objects(&memory, vec![ObjectType::DerivedMemory]).await;
        let updated = beliefs
            .iter()
            .find_map(|object| match object {
                MemoryObject::DerivedMemory(memory) if memory.id == new => Some(memory),
                _ => None,
            })
            .unwrap();
        assert_eq!(updated.derived_from_episode_ids, vec![original, correction]);
        assert_eq!(updated.derived_from_observation_ids, vec![observation]);
        assert!(updated.supersedes.contains(&old));
        memory.close().await.unwrap();
    }
}
