use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::api::types::*;
use crate::domain::*;
use crate::models::vector::{EmbeddingInput, VectorRecordEmbedding};
use crate::policy::embedding_surface::memory_object_vector_record;
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::graph_authority::GraphAuthorityStore;
use crate::ports::vector_candidate::VectorCandidateStore;
use crate::test_support::{
    in_memory_graph_store, representative_fixtures, DeterministicMemoryEmbedder,
    TemporaryVectorCandidateStore,
};
use crate::{CharacterMemory, CustomError};

struct RecordingEmbedder(Arc<Mutex<Vec<String>>>);

#[async_trait]
impl MemoryEmbedder for RecordingEmbedder {
    async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
        assert!(
            !input.text.trim().is_empty(),
            "an absent topic must not be embedded"
        );
        self.0.lock().unwrap().push(input.text.clone());
        DeterministicMemoryEmbedder::new(8).embed(input).await
    }
    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        let mut vectors = Vec::new();
        for input in inputs {
            vectors.push(self.embed(input).await?);
        }
        Ok(vectors)
    }
}

#[tokio::test]
async fn topic_only_preserves_task2_selection_sections_order_and_query_text() {
    let fixtures = representative_fixtures();
    let graph = in_memory_graph_store();
    graph.upsert_objects(&fixtures.objects()).await.unwrap();
    graph.upsert_links(&fixtures.links()).await.unwrap();
    let vector = TemporaryVectorCandidateStore::open(8).await;
    let embedder = DeterministicMemoryEmbedder::new(8);
    for object in fixtures.objects() {
        if let Some(record) = memory_object_vector_record(&object) {
            let embedding = embedder.embed(&record.embedding_input()).await.unwrap();
            vector
                .upsert_vector_records(&[VectorRecordEmbedding::new(&record, &embedding)])
                .await
                .unwrap();
        }
    }
    let inputs = Arc::new(Mutex::new(Vec::new()));
    let memory = CharacterMemory::from_parts(
        Box::new(graph),
        Box::new(vector),
        Box::new(RecordingEmbedder(inputs.clone())),
    );
    // Captured through the unchanged retrieval pipeline at Task_2 tip 77cbd98.
    let expected = [
        r#"{"active_threads":["550e8400-e29b-41d4-a716-446655440019"],"character_signals":[],"commitments":[],"derived_memories":[],"open_loops":[],"preferences":["550e8400-e29b-41d4-a716-44665544001f"],"relationship_notes":[],"relevant_episodes":[],"salient_observations":["550e8400-e29b-41d4-a716-446655440014"]}"#,
        r#"{"active_threads":["550e8400-e29b-41d4-a716-446655440019"],"character_signals":[],"commitments":["550e8400-e29b-41d4-a716-446655440021"],"derived_memories":["550e8400-e29b-41d4-a716-446655440022","550e8400-e29b-41d4-a716-44665544001e"],"open_loops":["550e8400-e29b-41d4-a716-446655440020"],"preferences":["550e8400-e29b-41d4-a716-44665544001f"],"relationship_notes":[],"relevant_episodes":["550e8400-e29b-41d4-a716-44665544000a"],"salient_observations":["550e8400-e29b-41d4-a716-446655440014"]}"#,
    ];
    for (case, (candidates, roots, derived_limit)) in
        [(6, 3, 1), (48, 12, 2)].into_iter().enumerate()
    {
        let mut context = RetrievalContext::new("  deterministic fixtures\nservice-free  ");
        context.candidate_limits.max_vector_candidates = candidates;
        context.candidate_limits.max_graph_roots = roots;
        context.section_limits.derived_memories = derived_limit;
        let result = memory.retrieve(context).await.unwrap();
        let projection = serde_json::to_value(&result.pack)
            .unwrap()
            .as_object()
            .unwrap()
            .iter()
            .map(|(section, objects)| {
                let ids = objects
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|object| {
                        object
                            .get("id")
                            .or_else(|| object.get("memory").and_then(|memory| memory.get("id")))
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .to_owned()
                    })
                    .collect::<Vec<_>>();
                (section.clone(), ids)
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(
            serde_json::to_value(&projection).unwrap(),
            serde_json::from_str::<serde_json::Value>(expected[case]).unwrap()
        );
    }
    assert_eq!(
        *inputs.lock().unwrap(),
        [
            "deterministic fixtures\nservice-free",
            "deterministic fixtures\nservice-free"
        ]
    );
}

struct CueEmbedder(Arc<Mutex<Vec<String>>>);

#[async_trait]
impl MemoryEmbedder for CueEmbedder {
    async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
        assert!(!input.text.trim().is_empty(), "absent topic was embedded");
        if input.surface == VectorSurface::Query {
            self.0.lock().unwrap().push(input.text.clone());
        }
        let text = input.text.to_lowercase();
        Ok(vec![
            if text.contains("astronomer") {
                1.0
            } else {
                0.0
            },
            if text.contains("observatory") {
                1.0
            } else {
                0.0
            },
            0.1,
        ])
    }
    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        let mut vectors = Vec::new();
        for input in inputs {
            vectors.push(self.embed(input).await?);
        }
        Ok(vectors)
    }
}

fn scene() -> Scene {
    Scene::at(
        chrono::DateTime::parse_from_rfc3339("2026-09-20T10:00:00.123Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    )
}

async fn scene_memory() -> (CharacterMemory, Arc<Mutex<Vec<String>>>) {
    let queries = Arc::new(Mutex::new(Vec::new()));
    let memory = CharacterMemory::from_parts(
        Box::new(in_memory_graph_store()),
        Box::new(TemporaryVectorCandidateStore::open(3).await),
        Box::new(CueEmbedder(queries.clone())),
    );
    (memory, queries)
}

async fn create_notion(memory: &CharacterMemory, id: u128, name: Option<&str>) {
    let notion_id = MemoryId::from_u128(id);
    let mut notion = EntityDraft::new();
    notion.id = Some(notion_id);
    notion.created_at = Some(scene().time);
    notion.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut plan = RememberWritePlan::new().with_candidate(MemoryCandidate::Entity(
        EntityCandidate::new(notion, CandidateProvenance::caller("notion")),
    ));
    if let Some(name) = name {
        let mut belief =
            DerivedMemoryDraft::new(DerivedType::Claim, "The astronomer is known by this name.");
        belief.id = Some(MemoryId::from_u128(id + 1000));
        belief.created_at = Some(scene().time);
        belief.updated_at = belief.created_at;
        belief.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        belief.entity_ids = vec![notion_id];
        belief.given_by_application = true;
        belief.assertions.push(BeliefAssertion {
            subject: notion_id,
            predicate: BeliefPredicate::KnownAs {
                name: name.to_owned(),
            },
        });
        plan = plan
            .with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
                belief,
                CandidateProvenance::caller("name"),
            )))
            .with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                MemoryObjectRef::new(ObjectType::DerivedMemory, MemoryId::from_u128(id + 1000)),
                CandidateProvenance::caller("content"),
            )));
    }
    memory.commit(plan, CommitOptions::default()).await.unwrap();
}

async fn write_episode(memory: &CharacterMemory, id: u128, scene: Scene) -> MemoryId {
    let mut episode = EpisodeDraft::new("A quiet meeting.");
    episode.id = Some(MemoryId::from_u128(id));
    let mut observation = ObservationDraft::new(episode.id.unwrap(), "An ordinary remark.");
    observation.id = Some(MemoryId::from_u128(id + 1));
    memory
        .remember(
            RememberInput::new("A quiet meeting.")
                .with_scene(scene)
                .with_episode(episode)
                .with_observation(observation),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    MemoryId::from_u128(id)
}

fn recorded(result: &RetrieveOutcome, object_type: ObjectType, id: MemoryId) -> &[SourceScene] {
    &result
        .memory_scenes
        .iter()
        .find(|entry| entry.memory == MemoryObjectRef::new(object_type, id))
        .unwrap_or_else(|| panic!("missing {object_type:?} {id}: {result:?}"))
        .sources
}

fn keyed(key: u128) -> SceneParticipant {
    SceneParticipant {
        key: Some(MemoryId::from_u128(key)),
        ..Default::default()
    }
}

#[tokio::test]
async fn authored_episode_without_links_is_recalled_by_its_participant() {
    let (memory, queries) = scene_memory().await;
    create_notion(&memory, 100, Some("Mira")).await;
    create_notion(&memory, 200, None).await;
    let mut elsewhere = scene();
    elsewhere.participants.push(keyed(200));
    write_episode(&memory, 6000, elsewhere).await;
    let mut past = scene();
    past.participants = vec![keyed(100), keyed(100)];
    let episode_id = MemoryId::from_u128(5000);
    let mut episode = EpisodeDraft::new("An authored experience.");
    episode.id = Some(episode_id);
    episode.created_at = Some(past.time);
    episode.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    episode.scene = Some(past.clone());
    let plan = RememberWritePlan::new().with_candidate(MemoryCandidate::Episode(
        EpisodeCandidate::new(episode, CandidateProvenance::caller("experience")),
    ));
    let first = memory
        .commit(plan.clone(), CommitOptions::default())
        .await
        .unwrap();
    let replay = memory.commit(plan, CommitOptions::default()).await.unwrap();
    assert_eq!(first.persisted_link_ids.len(), 1);
    assert_eq!(first.persisted_link_ids, replay.persisted_link_ids);
    queries.lock().unwrap().clear();
    let mut present = scene();
    present.participants.push(keyed(100));
    let result = memory
        .retrieve(RetrievalContext::default().with_scene(present).with_trace())
        .await
        .unwrap();
    assert!(queries.lock().unwrap().is_empty());
    assert_eq!(result.pack.relevant_episodes.len(), 1);
    assert_eq!(result.pack.relevant_episodes[0].id, episode_id);
    assert_eq!(
        recorded(&result, ObjectType::Episode, episode_id),
        [SourceScene::Recorded {
            episode_id,
            scene: past
        }]
    );
    assert!(result.trace.unwrap().graph_relations.iter().any(|link| {
        link.from.id == episode_id
            && link.to.id == MemoryId::from_u128(100)
            && link.relation == RelationType::Involves
    }));
}

#[tokio::test]
async fn participant_references_resolve_and_expand_without_a_topic_under_root_budget() {
    let (memory, queries) = scene_memory().await;
    create_notion(&memory, 100, Some("Mira")).await;
    create_notion(&memory, 200, Some("Mira")).await;
    create_notion(&memory, 300, None).await;
    let mut past = scene();
    past.participants.push(keyed(100));
    past.setting.key = Some("private room".to_owned());
    let episode_id = write_episode(&memory, 5000, past.clone()).await;
    let mut present = scene();
    present.participants.push(keyed(100));
    present.setting.key = Some("public room".to_owned());
    present
        .custom_values
        .insert("session".to_owned(), "different".to_owned());
    let key_result = memory
        .retrieve(
            RetrievalContext::default()
                .with_scene(present.clone())
                .with_trace(),
        )
        .await
        .unwrap();
    assert!(queries.lock().unwrap().is_empty());
    assert_eq!(key_result.scene, present);
    assert_eq!(
        recorded(
            &key_result,
            ObjectType::Observation,
            MemoryId::from_u128(5001)
        ),
        [SourceScene::Recorded {
            episode_id,
            scene: past
        }]
    );
    let trace = key_result.trace.as_ref().unwrap();
    assert!(trace.vector_candidates.is_empty());
    assert!(trace
        .graph_expansions
        .iter()
        .all(|entry| entry.source == GraphRootSource::Participant));
    assert!(key_result.rationale.telemetry.selectivity.decision_count > 0);
    assert!(trace.section_assignments.iter().any(|row| matches!(
        row.reason,
        SectionAssignmentReason::Selected {
            scores: SectionScoreComponents {
                cue_score_source: Some(SectionCueScoreSource::Participant { .. }),
                ..
            }
        }
    )));

    present.participants = vec![
        SceneParticipant {
            name: Some("Mira".to_owned()),
            ..Default::default()
        },
        SceneParticipant {
            name: Some("  MIRA  ".to_owned()),
            ..Default::default()
        },
        SceneParticipant {
            name: Some("Nobody".to_owned()),
            ..Default::default()
        },
    ];
    let named = memory
        .retrieve(RetrievalContext::default().with_scene(present.clone()))
        .await
        .unwrap();
    assert!(named
        .pack
        .salient_observations
        .iter()
        .any(|observation| observation.episode_id == episode_id));
    assert!(named.trace.is_none());
    let references = &named.scene_references;
    assert_eq!(
        references[0].resolution,
        SceneReferenceResolution::Ambiguous {
            notion_ids: vec![MemoryId::from_u128(100), MemoryId::from_u128(200)]
        }
    );
    assert_eq!(references[1].resolution, references[0].resolution);
    assert_eq!(references[2].resolution, SceneReferenceResolution::Unknown);
    // The same lookup becomes resolved after one of the name beliefs is forgotten.
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::derived_memory(MemoryId::from_u128(1200)),
            "This name is no longer believed.",
        ))
        .await
        .unwrap();
    let resolved_name = memory
        .retrieve(RetrievalContext::default().with_scene(present.clone()))
        .await
        .unwrap();
    assert_eq!(
        resolved_name.scene_references[0].resolution,
        SceneReferenceResolution::Resolved {
            notion_id: MemoryId::from_u128(100)
        }
    );

    assert_eq!(
        named.rationale.telemetry.unique_graph_root_candidate_count,
        2
    );

    present.participants = vec![keyed(999), keyed(300)];
    let empty = memory
        .retrieve(RetrievalContext::default().with_scene(present.clone()))
        .await
        .unwrap();
    assert!(empty.memory_scenes.is_empty());
    assert!(empty.trace.is_none());
    let references = &empty.scene_references;
    assert_eq!(references[0].resolution, SceneReferenceResolution::Unknown);
    assert_eq!(
        references[1].resolution,
        SceneReferenceResolution::Resolved {
            notion_id: MemoryId::from_u128(300)
        }
    );

    present.participants = vec![
        keyed(200),
        SceneParticipant {
            name: Some("Mira".to_owned()),
            ..Default::default()
        },
        keyed(300),
    ];
    let mut limited = RetrievalContext::new("astronomer")
        .with_scene(present)
        .with_trace();
    limited.candidate_limits.max_graph_roots = 1;
    let first = memory.retrieve(limited.clone()).await.unwrap();
    let second = memory.retrieve(limited).await.unwrap();
    assert_eq!(
        first.trace.as_ref().unwrap().graph_expansions,
        second.trace.unwrap().graph_expansions
    );
    let roots = &first.trace.unwrap().graph_expansions;
    assert!(roots
        .iter()
        .any(|root| root.root.id == MemoryId::from_u128(200)
            && root.outcome == GraphExpansionOutcome::Expanded));
    for id in [100, 300] {
        assert!(roots
            .iter()
            .any(|root| root.root.id == MemoryId::from_u128(id)
                && root.outcome == GraphExpansionOutcome::RootLimit));
    }
    assert!(roots
        .iter()
        .filter(|root| root.source == GraphRootSource::Vector)
        .all(|root| root.outcome == GraphExpansionOutcome::RootLimit));
}

#[tokio::test]
async fn descriptions_and_setting_words_recall_content_once_and_merge_with_topic() {
    let (memory, queries) = scene_memory().await;
    create_notion(&memory, 100, Some("Mira")).await;
    let mut past = scene();
    past.setting.words = Some("observatory".to_owned());
    let episode_id = write_episode(&memory, 5000, past).await;
    let mut present = scene();
    present.participants = vec![
        SceneParticipant {
            description: Some("astronomer".to_owned()),
            ..Default::default()
        };
        2
    ];
    let mut context = RetrievalContext::default().with_scene(present).with_trace();
    context.candidate_limits.max_vector_candidates = 1;
    let description_only = memory.retrieve(context.clone()).await.unwrap();
    assert_eq!(*queries.lock().unwrap(), ["astronomer"]);
    assert_eq!(
        description_only.pack.derived_memories[0].memory.id,
        MemoryId::from_u128(1100)
    );
    let trace = description_only.trace.as_ref().unwrap();
    assert!(trace
        .graph_relations
        .iter()
        .any(|link| link.to.id == MemoryId::from_u128(100)
            || link.from.id == MemoryId::from_u128(100)));
    assert!(description_only
        .scene_references
        .iter()
        .all(|reference| reference.resolution == SceneReferenceResolution::ContentCue));
    context.topic = Some("astronomer".to_owned());
    queries.lock().unwrap().clear();
    let merged = memory.retrieve(context.clone()).await.unwrap();
    assert_eq!(*queries.lock().unwrap(), ["astronomer"]);
    assert_eq!(merged.pack, description_only.pack);
    assert_eq!(merged.rationale.vector_candidate_count, 1);
    context.scene.setting.words = Some("observatory".to_owned());
    let budgeted = memory.retrieve(context).await.unwrap();
    assert_eq!(budgeted.trace.unwrap().vector_candidates.len(), 1);
    let mut place = RetrievalContext::default();
    place.scene.setting.words = Some("observatory".to_owned());
    place.candidate_limits.max_vector_candidates = 1;
    let place_result = memory.retrieve(place).await.unwrap();
    assert!(place_result.trace.is_none());
    assert_eq!(place_result.pack.relevant_episodes[0].id, episode_id);
    assert_eq!(
        recorded(&place_result, ObjectType::Episode, episode_id),
        [SourceScene::Recorded {
            episode_id,
            scene: place_result.pack.relevant_episodes[0].scene.clone()
        }]
    );
    assert!(matches!(
        &place_result.scene_references[0],
        SceneReferenceResult {
            reference: SceneReference::SettingWords,
            resolution: SceneReferenceResolution::ContentCue
        }
    ));
}

#[tokio::test]
async fn time_only_scene_is_echoed_without_embedding_or_completeness_claim() {
    let (memory, queries) = scene_memory().await;
    let before = chrono::Utc::now();
    let context = RetrievalContext::default();
    assert!(context.scene.time >= before && context.scene.time <= chrono::Utc::now());
    let present = context.scene.clone();
    let outcome = memory.retrieve(context).await.unwrap();
    assert_eq!(outcome.scene, present);
    assert!(outcome.scene_references.is_empty());
    assert!(outcome.trace.is_none());
    assert!(outcome.memory_scenes.is_empty());
    assert_eq!(
        outcome.rationale.telemetry.vector_recall_completeness,
        VectorRecallCompleteness::NotRequested
    );
    assert_eq!(outcome.rationale.telemetry.query_embedding_dimension, 0);
    assert_eq!(outcome.rationale.vector_candidate_count, 0);
    assert!(queries.lock().unwrap().is_empty());
    let mut blank = RetrievalContext::new(" \n ");
    blank.scene.participants.push(SceneParticipant::default());
    blank.scene.setting.words = Some("\t".to_owned());
    let result = memory.retrieve(blank).await.unwrap();
    assert!(result.scene.participants.is_empty());
    assert!(queries.lock().unwrap().is_empty());
}

async fn write_belief(
    memory: &CharacterMemory,
    id: u128,
    episodes: &[u128],
    observations: &[u128],
) {
    let mut belief = DerivedMemoryDraft::new(
        DerivedType::Claim,
        "The astronomer remembers these experiences.",
    );
    belief.id = Some(MemoryId::from_u128(id));
    belief.created_at = Some(scene().time);
    belief.updated_at = belief.created_at;
    belief.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    belief.derived_from_episode_ids = episodes.iter().map(|id| MemoryId::from_u128(*id)).collect();
    belief.derived_from_observation_ids = observations
        .iter()
        .map(|id| MemoryId::from_u128(*id))
        .collect();
    let plan = RememberWritePlan::new()
        .with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
            belief,
            CandidateProvenance::caller("interpretation"),
        )))
        .with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
            MemoryObjectRef::new(ObjectType::DerivedMemory, MemoryId::from_u128(id)),
            CandidateProvenance::caller("content"),
        )));
    memory.commit(plan, CommitOptions::default()).await.unwrap();
}

fn beliefs_context() -> RetrievalContext {
    let mut context = RetrievalContext::new("astronomer");
    context.object_type_defaults = vec![ObjectType::DerivedMemory, ObjectType::MemoryThread];
    context.graph_limits.max_depth = 0;
    context
}

#[tokio::test]
async fn result_reports_all_source_scenes_without_admitting_sources_or_requiring_trace() {
    let (memory, _) = scene_memory().await;
    let first_scene = scene();
    let mut second_scene = scene();
    second_scene.setting.words = Some("a different room".to_owned());
    write_episode(&memory, 5000, first_scene.clone()).await;
    write_episode(&memory, 6000, second_scene.clone()).await;
    write_belief(&memory, 8000, &[5000, 6000], &[5001]).await;
    write_belief(&memory, 8001, &[], &[5001, 6001]).await;
    create_notion(&memory, 100, Some("Mira")).await;
    let mut thread = MemoryThreadDraft::new("Astronomer", "Plans for tomorrow.");
    thread.id = Some(MemoryId::from_u128(9000));
    thread.created_at = Some(scene().time);
    thread.updated_at = thread.created_at;
    thread.last_touched_at = thread.created_at;
    thread.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    memory
        .commit(
            RememberWritePlan::new()
                .with_candidate(MemoryCandidate::MemoryThread(MemoryThreadCandidate::new(
                    thread,
                    CandidateProvenance::caller("thread"),
                )))
                .with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                    MemoryObjectRef::new(ObjectType::MemoryThread, MemoryId::from_u128(9000)),
                    CandidateProvenance::caller("content"),
                ))),
            CommitOptions::default(),
        )
        .await
        .unwrap();
    let expected = vec![
        SourceScene::Recorded {
            episode_id: MemoryId::from_u128(5000),
            scene: first_scene,
        },
        SourceScene::Recorded {
            episode_id: MemoryId::from_u128(6000),
            scene: second_scene,
        },
    ];
    let result = memory.retrieve(beliefs_context()).await.unwrap();
    assert!(result.trace.is_none());
    assert!(
        result.pack.relevant_episodes.is_empty() && result.pack.salient_observations.is_empty()
    );
    for id in [8000, 8001] {
        assert_eq!(
            recorded(&result, ObjectType::DerivedMemory, MemoryId::from_u128(id)),
            expected
        );
    }
    assert!(recorded(
        &result,
        ObjectType::DerivedMemory,
        MemoryId::from_u128(1100)
    )
    .is_empty());
    assert!(recorded(&result, ObjectType::MemoryThread, MemoryId::from_u128(9000)).is_empty());
    let origin = SourceProvenanceReference {
        episode_ids: Vec::new(),
        observation_ids: Vec::new(),
        external_refs: vec![ExternalSourceReference::raw("application:correction")],
    };
    for (old, new, given) in [(8000, 8100, false), (1100, 1200, true)] {
        let mut replacement = ReplacementDerivedMemoryDraft::new(
            DerivedType::Correction,
            "The astronomer corrected this belief.",
        );
        replacement.id = Some(MemoryId::from_u128(new));
        replacement.given_by_application = given;
        if given {
            replacement.entity_ids.push(MemoryId::from_u128(100));
        } else {
            replacement.derived_from_episode_ids =
                vec![MemoryId::from_u128(5000), MemoryId::from_u128(6000)];
        }
        replacement.correction_origin_provenance = origin.clone();
        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::derived_memory(MemoryId::from_u128(old)),
            "Correct this interpretation.",
        )
        .with_replacement(replacement);
        draft.correction_origin = origin.clone();
        memory.correct(draft).await.unwrap();
    }
    let corrected = memory.retrieve(beliefs_context()).await.unwrap();
    assert_eq!(
        recorded(
            &corrected,
            ObjectType::DerivedMemory,
            MemoryId::from_u128(8100)
        ),
        expected
    );
    assert!(recorded(
        &corrected,
        ObjectType::DerivedMemory,
        MemoryId::from_u128(1200)
    )
    .is_empty());
}

#[tokio::test]
async fn forgotten_source_scenes_follow_the_existing_suppression_switch() {
    let (memory, _) = scene_memory().await;
    let past = scene();
    for id in [5000, 6000, 7000] {
        write_episode(&memory, id, past.clone()).await;
    }
    write_belief(&memory, 8000, &[5000], &[]).await;
    write_belief(&memory, 8001, &[], &[6001]).await;
    write_belief(&memory, 8002, &[], &[7001]).await;
    // A forgotten episode with an active direct interpretation, and a forgotten
    // observation with an active observation-only interpretation, are both public paths.
    for target in [
        LifecycleTargetRef::episode(MemoryId::from_u128(5000)),
        LifecycleTargetRef::observation(MemoryId::from_u128(6001)),
        LifecycleTargetRef::episode(MemoryId::from_u128(7000)),
    ] {
        let mut draft = ForgetMemoryDraft::suppress(
            target,
            "Forget the experience, retain its interpretation.",
        );
        draft.cascade_policy.apply_to_derived_from_target = false;
        memory.forget(draft).await.unwrap();
    }
    for include_suppressed in [false, true] {
        let mut context = beliefs_context();
        context.lifecycle_policy.include_suppressed = include_suppressed;
        let result = memory.retrieve(context).await.unwrap();
        for (belief, episode, source) in [
            (
                8000,
                5000,
                MemoryObjectRef::new(ObjectType::Episode, MemoryId::from_u128(5000)),
            ),
            (
                8001,
                6000,
                MemoryObjectRef::new(ObjectType::Observation, MemoryId::from_u128(6001)),
            ),
            (
                8002,
                7000,
                MemoryObjectRef::new(ObjectType::Episode, MemoryId::from_u128(7000)),
            ),
        ] {
            let expected = if include_suppressed {
                SourceScene::Recorded {
                    episode_id: MemoryId::from_u128(episode),
                    scene: past.clone(),
                }
            } else {
                SourceScene::Unavailable {
                    source,
                    reason: SourceSceneUnavailableReason::Forgotten,
                }
            };
            assert_eq!(
                recorded(
                    &result,
                    ObjectType::DerivedMemory,
                    MemoryId::from_u128(belief)
                ),
                [expected]
            );
        }
    }
}
