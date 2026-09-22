use std::collections::BTreeSet;
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
async fn topic_only_applies_section_limits_and_preserves_query_text() {
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
    // The fulfilled commitment shares the ordinary derived-memory section cap.
    let expected = [
        r#"{"active_threads":["550e8400-e29b-41d4-a716-446655440019"],"character_signals":[],"commitments":[],"derived_memories":[],"open_loops":[],"preferences":["550e8400-e29b-41d4-a716-44665544001f"],"relationship_notes":[],"relevant_episodes":[],"salient_observations":["550e8400-e29b-41d4-a716-446655440014"]}"#,
        r#"{"active_threads":["550e8400-e29b-41d4-a716-446655440019"],"character_signals":[],"commitments":[],"derived_memories":["550e8400-e29b-41d4-a716-446655440022","550e8400-e29b-41d4-a716-446655440021"],"open_loops":["550e8400-e29b-41d4-a716-446655440020"],"preferences":["550e8400-e29b-41d4-a716-44665544001f"],"relationship_notes":[],"relevant_episodes":["550e8400-e29b-41d4-a716-44665544000a"],"salient_observations":["550e8400-e29b-41d4-a716-446655440014"]}"#,
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
        (chrono::DateTime::parse_from_rfc3339("2026-09-20T10:00:00.123Z")
            .unwrap()
            .with_timezone(&chrono::Utc))
        .fixed_offset(),
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
    notion.created_at = Some(scene().time.to_utc());
    notion.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut plan = RememberWritePlan::new().with_candidate(MemoryCandidate::Entity(
        EntityCandidate::new(notion, CandidateProvenance::caller("notion")),
    ));
    if let Some(name) = name {
        let mut belief =
            DerivedMemoryDraft::new(DerivedType::Claim, "The astronomer is known by this name.");
        belief.id = Some(MemoryId::from_u128(id + 1000));
        belief.created_at = Some(scene().time.to_utc());
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

fn selected_cues(result: &RetrieveOutcome, object: MemoryObjectRef) -> &BTreeSet<CueKind> {
    &result
        .trace
        .as_ref()
        .unwrap()
        .section_assignments
        .iter()
        .find(|row| {
            row.object == object && matches!(row.reason, SectionAssignmentReason::Selected { .. })
        })
        .unwrap()
        .cue_kinds
}

#[tokio::test]
async fn cue_union_survives_winning_scores_but_excludes_a_root_cut_by_the_budget() {
    let (memory, _) = scene_memory().await;
    create_notion(&memory, 100, None).await;
    // Keep this participant selective so the test observes cue union, not ubiquity.
    write_episode(&memory, 6000, scene()).await;
    let id = MemoryId::from_u128(7000);
    let mut episode = EpisodeDraft::new("An astronomer arrived.");
    episode.id = Some(id);
    let observation_id = MemoryId::from_u128(7001);
    let mut observation = ObservationDraft::new(id, "An astronomer arrived.");
    observation.id = Some(observation_id);
    let mut present = scene();
    present.participants.push(keyed(100));
    memory
        .remember(
            RememberInput::new("An astronomer arrived.")
                .with_scene(present.clone())
                .with_episode(episode)
                .with_observation(observation),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    let mut context = RetrievalContext::new("astronomer")
        .with_scene(present)
        .with_trace();
    context.candidate_limits.max_vector_candidates = 1;
    context.object_type_defaults = vec![ObjectType::Observation];
    let object = MemoryObjectRef::new(ObjectType::Observation, observation_id);
    let combined = memory.retrieve(context.clone()).await.unwrap();
    assert_eq!(
        selected_cues(&combined, object),
        &BTreeSet::from([CueKind::Topic, CueKind::Participant]),
        "{:#?}",
        combined.trace
    );
    context.candidate_limits.max_graph_roots = 1;
    let limited = memory.retrieve(context).await.unwrap();
    assert_eq!(
        selected_cues(&limited, object),
        &BTreeSet::from([CueKind::Participant])
    );
    assert!(limited
        .trace
        .as_ref()
        .unwrap()
        .graph_expansions
        .iter()
        .any(|root| { root.root == object && root.outcome == GraphExpansionOutcome::RootLimit }));
    memory.close().await.unwrap();
}

#[tokio::test]
async fn observation_forget_recounts_notion_presence_without_removing_scene_participants() {
    for sqlite in [false, true] {
        for (in_scene, direct_link) in [(false, false), (true, false), (false, true)] {
            let directory = tempfile::tempdir().unwrap();
            let (mut memory, _) = scene_memory().await;
            if sqlite {
                memory.memory_composition.stats_store = Box::new(
                    crate::adapters::stats::SqliteRetrievalStatsStore::open(
                        directory.path().join("stats.sqlite"),
                    )
                    .unwrap(),
                );
            }
            create_notion(&memory, 100, None).await;
            let mut occasion = scene();
            if in_scene {
                occasion.participants.push(keyed(100));
            }
            let episode = write_episode(&memory, 30_000, occasion).await;
            if direct_link {
                memory
                    .link(MemoryLinkDraft::new(
                        ObjectType::Episode,
                        episode,
                        RelationType::Involves,
                        ObjectType::Entity,
                        MemoryId::from_u128(100),
                    ))
                    .await
                    .unwrap();
            }
            write_episode(&memory, 31_000, scene()).await;
            let mut extra = ObservationDraft::new(episode, "Another remark.");
            extra.id = Some(MemoryId::from_u128(30_002));
            extra.created_at = Some(scene().time.to_utc());
            extra.observed_at = Some(scene().time.to_utc());
            extra.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
            memory
                .commit(
                    RememberWritePlan::new().with_candidate(MemoryCandidate::Observation(
                        ObservationCandidate::new(extra, CandidateProvenance::caller("remark")),
                    )),
                    CommitOptions::default(),
                )
                .await
                .unwrap();
            for observation in [30_001, 30_002] {
                memory
                    .link(MemoryLinkDraft::new(
                        ObjectType::Observation,
                        MemoryId::from_u128(observation),
                        RelationType::Mentions,
                        ObjectType::Entity,
                        MemoryId::from_u128(100),
                    ))
                    .await
                    .unwrap();
            }
            let mut context = RetrievalContext::default().with_trace();
            context.scene.participants.push(keyed(100));
            for (forgotten, expected) in [
                (None, 1),
                (Some(30_001), 1),
                (Some(30_002), u64::from(in_scene || direct_link)),
            ] {
                if let Some(id) = forgotten {
                    memory
                        .forget(ForgetMemoryDraft::suppress(
                            LifecycleTargetRef::observation(MemoryId::from_u128(id)),
                            "Forget only this remark.",
                        ))
                        .await
                        .unwrap();
                }
                let result = memory.retrieve(context.clone()).await.unwrap();
                let decision = result
                    .trace
                    .unwrap()
                    .selectivity_decisions
                    .into_iter()
                    .find(|row| {
                        row.root.id == MemoryId::from_u128(100)
                            && row.relation == RelationType::Mentions
                    })
                    .unwrap();
                assert_eq!(
                    (decision.entity_count, decision.global_count),
                    (Some(expected), Some(2)),
                    "sqlite={sqlite}, in_scene={in_scene}, direct_link={direct_link}, forgotten={forgotten:?}"
                );
            }
            memory.close().await.unwrap();
        }
    }
}

#[tokio::test]
async fn mentions_count_the_parent_episode_once_and_follow_its_lifecycle() {
    let (memory, _) = scene_memory().await;
    create_notion(&memory, 100, None).await;
    let participant = MemoryId::from_u128(100);
    let episode = write_episode(&memory, 30_000, scene()).await;
    write_episode(&memory, 31_000, scene()).await;
    let mut extra = ObservationDraft::new(episode, "Another remark on the same occasion.");
    extra.id = Some(MemoryId::from_u128(30_002));
    extra.created_at = Some(scene().time.to_utc());
    extra.observed_at = Some(scene().time.to_utc());
    extra.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    memory
        .commit(
            RememberWritePlan::new().with_candidate(MemoryCandidate::Observation(
                ObservationCandidate::new(extra, CandidateProvenance::caller("remark")),
            )),
            CommitOptions::default(),
        )
        .await
        .unwrap();
    for (index, observation) in [30_001, 30_002].into_iter().enumerate() {
        let observation = MemoryId::from_u128(observation);
        let mut link = if index == 0 {
            MemoryLinkDraft::new(
                ObjectType::Observation,
                observation,
                RelationType::Mentions,
                ObjectType::Entity,
                participant,
            )
        } else {
            MemoryLinkDraft::new(
                ObjectType::Entity,
                participant,
                RelationType::Mentions,
                ObjectType::Observation,
                observation,
            )
        };
        link.id = Some(MemoryId::from_u128(32_000 + index as u128));
        for _ in 0..2 {
            memory.link(link.clone()).await.unwrap();
        }
    }
    let mut context = RetrievalContext::default().with_trace();
    context.scene.participants = vec![SceneParticipant {
        key: Some(participant),
        ..Default::default()
    }];
    let counts = |result: &RetrieveOutcome| {
        let decision = result
            .trace
            .as_ref()
            .unwrap()
            .selectivity_decisions
            .iter()
            .find(|decision| {
                decision.root.id == participant && decision.relation == RelationType::Mentions
            })
            .unwrap();
        (decision.entity_count, decision.global_count)
    };
    let result = memory.retrieve(context.clone()).await.unwrap();
    assert_eq!(counts(&result), (Some(1), Some(2)));
    assert_eq!(result.pack.salient_observations.len(), 1);
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::episode(episode),
            "The occasion is forgotten",
        ))
        .await
        .unwrap();
    assert_eq!(
        counts(&memory.retrieve(context.clone()).await.unwrap()),
        (Some(0), Some(1))
    );
    context.lifecycle_policy.include_suppressed = true;
    assert_eq!(
        counts(&memory.retrieve(context).await.unwrap()),
        (Some(1), Some(2))
    );
    memory.close().await.unwrap();
}

#[tokio::test]
async fn suppressed_parent_evidence_keeps_its_active_observation_admissible_by_topic() {
    let (memory, _) = scene_memory().await;
    create_notion(&memory, 100, None).await;
    let episode = write_episode(&memory, 50_100, scene()).await;
    let observation = MemoryId::from_u128(50_101);
    memory
        .link(MemoryLinkDraft::new(
            ObjectType::Observation,
            observation,
            RelationType::Mentions,
            ObjectType::Entity,
            MemoryId::from_u128(100),
        ))
        .await
        .unwrap();
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::episode(episode),
            "Forget only the parent occasion",
        ))
        .await
        .unwrap();
    for topic in [Some("ordinary".to_owned()), None] {
        let has_topic = topic.is_some();
        let mut context = RetrievalContext::default().with_trace();
        context.topic = topic;
        context.scene.participants.push(keyed(100));
        context.graph_limits.max_depth = 1;
        let result = memory.retrieve(context).await.unwrap();
        assert_eq!(
            result.pack.salient_observations.len(),
            usize::from(has_topic)
        );
        if has_topic {
            assert_eq!(result.pack.salient_observations[0].id, observation);
            assert_eq!(
                result.pack.salient_observations[0].retention_state,
                RetentionState::Active
            );
        }
        let trace = result.trace.unwrap();
        let omissions = trace
            .lifecycle_filter_decisions
            .iter()
            .filter(|decision| decision.reason == LifecycleFilterReason::SuppressedOmitted)
            .collect::<Vec<_>>();
        assert!(!omissions.is_empty());
        assert!(omissions.iter().all(|decision| {
            decision.object == MemoryObjectRef::new(ObjectType::Episode, episode)
        }));
        let utilization = trace
            .fanout_utilization
            .iter()
            .find(|row| {
                row.root.id == MemoryId::from_u128(100)
                    && row.relation == RelationType::Mentions
                    && row.object_type == ObjectType::Observation
            })
            .unwrap();
        assert_eq!(utilization.retained_count, 0);
        assert_eq!(utilization.omitted_by_fanout_count, 0);
    }
    memory.close().await.unwrap();
}

#[tokio::test]
async fn suppressed_latest_observation_recalls_the_previous_participant_occasion() {
    assert_participant_recall_after_suppression(false).await;
}

#[tokio::test]
async fn suppressed_latest_observation_leaves_its_active_sibling_eligible() {
    assert_participant_recall_after_suppression(true).await;
}

async fn assert_participant_recall_after_suppression(active_sibling: bool) {
    let (memory, _) = scene_memory().await;
    create_notion(&memory, 100, None).await;
    for index in 0..10 {
        let mut occasion = scene();
        occasion.time += chrono::Duration::hours(index as i64);
        let id = 50_000 + index * 10;
        write_episode(&memory, id, occasion).await;
        memory
            .link(MemoryLinkDraft::new(
                ObjectType::Observation,
                MemoryId::from_u128(id + 1),
                RelationType::Mentions,
                ObjectType::Entity,
                MemoryId::from_u128(100),
            ))
            .await
            .unwrap();
    }
    let latest_episode = MemoryId::from_u128(50_090);
    let latest_observation = MemoryId::from_u128(50_091);
    let sibling = MemoryId::from_u128(50_092);
    if active_sibling {
        let mut observation = ObservationDraft::new(latest_episode, "Another active remark.");
        observation.id = Some(sibling);
        observation.created_at = Some(scene().time.to_utc());
        observation.observed_at = Some(scene().time.to_utc());
        observation.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        memory
            .commit(
                RememberWritePlan::new().with_candidate(MemoryCandidate::Observation(
                    ObservationCandidate::new(observation, CandidateProvenance::caller("remark")),
                )),
                CommitOptions::default(),
            )
            .await
            .unwrap();
        memory
            .link(MemoryLinkDraft::new(
                ObjectType::Entity,
                MemoryId::from_u128(100),
                RelationType::Mentions,
                ObjectType::Observation,
                sibling,
            ))
            .await
            .unwrap();
    }
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::observation(latest_observation),
            "Forget the latest remark",
        ))
        .await
        .unwrap();
    let mut context = RetrievalContext::default().with_trace();
    context.scene.participants.push(keyed(100));
    context.graph_limits.max_depth = 1;
    let first = memory.retrieve(context.clone()).await.unwrap();
    let assert_lifecycle_evidence = |result: &RetrieveOutcome| {
        assert_eq!(result.rationale.lifecycle_omission_count, 1);
        let trace = result.trace.as_ref().unwrap();
        assert_eq!(
            trace
                .lifecycle_filter_decisions
                .iter()
                .filter(|decision| decision.reason == LifecycleFilterReason::SuppressedOmitted)
                .map(|decision| decision.object.id)
                .collect::<Vec<_>>(),
            vec![latest_observation]
        );
        let utilization = trace
            .fanout_utilization
            .iter()
            .find(|row| {
                row.root.id == MemoryId::from_u128(100) && row.relation == RelationType::Mentions
            })
            .unwrap();
        assert_eq!(utilization.retained_count, 1);
        assert_eq!(
            utilization.omitted_by_fanout_count,
            8 + usize::from(active_sibling)
        );
    };
    assert_lifecycle_evidence(&first);
    // Suppression after the occasion budget fills must not inflate lifecycle evidence.
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::observation(MemoryId::from_u128(50_001)),
            "Forget an old remark",
        ))
        .await
        .unwrap();
    let after_older = memory.retrieve(context.clone()).await.unwrap();
    assert_lifecycle_evidence(&after_older);
    assert_eq!(first.pack, after_older.pack);
    let mut untraced = context.clone();
    untraced.include_trace = false;
    let untraced = memory.retrieve(untraced).await.unwrap();
    assert_eq!(untraced.pack, first.pack);
    assert_eq!(untraced.rationale.lifecycle_omission_count, 1);
    let observation_ids = |result: RetrieveOutcome| {
        result
            .pack
            .salient_observations
            .into_iter()
            .map(|observation| observation.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        observation_ids(memory.retrieve(context.clone()).await.unwrap()),
        vec![if active_sibling {
            sibling
        } else {
            MemoryId::from_u128(50_081)
        }]
    );
    context.lifecycle_policy.include_suppressed = true;
    let included = memory.retrieve(context.clone()).await.unwrap();
    assert_eq!(included.rationale.lifecycle_omission_count, 0);
    assert_eq!(observation_ids(included), vec![latest_observation]);
    if active_sibling {
        memory.close().await.unwrap();
        return;
    }

    // Parent retention also controls occasion eligibility.
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::episode(latest_episode),
            "Forget the latest occasion",
        ))
        .await
        .unwrap();
    context.lifecycle_policy.include_suppressed = false;
    assert_eq!(
        observation_ids(memory.retrieve(context.clone()).await.unwrap()),
        vec![MemoryId::from_u128(50_081)]
    );
    context.lifecycle_policy.include_suppressed = true;
    assert_eq!(
        observation_ids(memory.retrieve(context).await.unwrap()),
        vec![latest_observation]
    );
    // Direct and observation routes must also agree on the same eligible occasion.
    for index in 0..10 {
        memory
            .link(MemoryLinkDraft::new(
                ObjectType::Episode,
                MemoryId::from_u128(50_000 + index * 10),
                RelationType::Involves,
                ObjectType::Entity,
                MemoryId::from_u128(100),
            ))
            .await
            .unwrap();
    }
    for include_suppressed in [false, true] {
        let mut context = RetrievalContext::default();
        context.scene.participants.push(keyed(100));
        context.graph_limits.max_depth = 1;
        context.lifecycle_policy.include_suppressed = include_suppressed;
        let result = memory.retrieve(context).await.unwrap();
        assert_eq!(
            result
                .pack
                .relevant_episodes
                .iter()
                .map(|episode| episode.id)
                .collect::<Vec<_>>(),
            (0..8)
                .map(|index| MemoryId::from_u128(
                    (if include_suppressed {
                        latest_episode.as_u128()
                    } else {
                        50_080
                    }) - index * 10
                ))
                .collect::<Vec<_>>()
        );
    }
    // Even when many excluded occasions precede the survivor (or none survives),
    // evidence stays within each route's existing participant budget.
    for index in 1..9 {
        memory
            .forget(ForgetMemoryDraft::suppress(
                LifecycleTargetRef::episode(MemoryId::from_u128(50_000 + index * 10)),
                "Forget another occasion",
            ))
            .await
            .unwrap();
    }
    for forget_last in [false, true] {
        if forget_last {
            memory
                .forget(ForgetMemoryDraft::suppress(
                    LifecycleTargetRef::episode(MemoryId::from_u128(50_000)),
                    "Forget the last occasion",
                ))
                .await
                .unwrap();
        }
        let mut context = RetrievalContext::default().with_trace();
        context.scene.participants.push(keyed(100));
        context.graph_limits.max_depth = 1;
        let result = memory.retrieve(context).await.unwrap();
        assert_eq!(
            result.pack.relevant_episodes.len(),
            usize::from(!forget_last)
        );
        assert!(result.pack.salient_observations.is_empty());
        assert_eq!(result.rationale.lifecycle_omission_count, 2);
        let trace = result.trace.unwrap();
        assert_eq!(
            trace
                .lifecycle_filter_decisions
                .iter()
                .filter(|decision| decision.reason == LifecycleFilterReason::SuppressedOmitted)
                .map(|decision| decision.object.id)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([latest_episode, latest_observation])
        );
        for row in trace.fanout_utilization {
            if row.root.id != MemoryId::from_u128(100) {
                continue;
            }
            assert_eq!(row.selected_cap, 1);
            assert_eq!(
                row.omitted_by_fanout_count,
                if row.relation == RelationType::Involves {
                    8 + usize::from(forget_last)
                } else {
                    9
                }
            );
        }
    }
    memory.close().await.unwrap();
}

#[tokio::test]
async fn ubiquitous_participant_keeps_the_latest_occasion_across_store_sizes_and_paths() {
    use crate::adapters::stats::InMemoryRetrievalStatsStore;

    for route in 0..3 {
        for reverse_ids in [false, true] {
            let (mut memory, _) = scene_memory().await;
            create_notion(&memory, 100, None).await;
            let participant = MemoryId::from_u128(100);
            let mut context = RetrievalContext::default().with_trace();
            context.scene.participants.push(keyed(100));
            context.graph_limits.max_depth = 1;
            for count in 1..=10 {
                let id = 40_000 + if reverse_ids { 11 - count } else { count } * 10;
                let episode_id = MemoryId::from_u128(id);
                let direct = route == 0 || (route == 2 && count % 3 != 1);
                let mentions = route == 1 || (route == 2 && count % 3 != 0);
                let mut occasion = scene();
                occasion.time += chrono::Duration::hours(count as i64);
                if direct {
                    occasion.participants.push(keyed(100));
                }
                let mut episode = EpisodeDraft::new("The participant visited.");
                episode.id = Some(episode_id);
                // Authorship order deliberately disagrees with scene recency.
                episode.created_at =
                    Some(scene().time.to_utc() - chrono::Duration::hours(count as i64));
                episode.scene = Some(occasion);
                episode.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
                let mut plan = RememberWritePlan::new().with_candidate(MemoryCandidate::Episode(
                    EpisodeCandidate::new(episode, CandidateProvenance::caller("occasion")),
                ));
                if mentions {
                    for offset in 1..=3 {
                        let mut observation = ObservationDraft::new(episode_id, "A remark.");
                        observation.id = Some(MemoryId::from_u128(id + offset));
                        observation.created_at = Some(scene().time.to_utc());
                        observation.observed_at = Some(scene().time.to_utc());
                        observation.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
                        plan = plan.with_candidate(MemoryCandidate::Observation(
                            ObservationCandidate::new(
                                observation,
                                CandidateProvenance::caller("remark"),
                            ),
                        ));
                    }
                }
                memory.commit(plan, CommitOptions::default()).await.unwrap();
                if mentions {
                    for offset in 1..=3 {
                        let observation = MemoryId::from_u128(id + offset);
                        let link = if offset % 2 == 0 {
                            MemoryLinkDraft::new(
                                ObjectType::Entity,
                                participant,
                                RelationType::Mentions,
                                ObjectType::Observation,
                                observation,
                            )
                        } else {
                            MemoryLinkDraft::new(
                                ObjectType::Observation,
                                observation,
                                RelationType::Mentions,
                                ObjectType::Entity,
                                participant,
                            )
                        };
                        memory.link(link).await.unwrap();
                    }
                }
                let healthy = memory.retrieve(context.clone()).await.unwrap();
                let healthy_stats = std::mem::replace(
                    &mut memory.memory_composition.stats_store,
                    Box::new(InMemoryRetrievalStatsStore::new()),
                );
                let missing = memory.retrieve(context.clone()).await.unwrap();
                memory.memory_composition.stats_store = healthy_stats;
                for result in [&healthy, &missing] {
                    let occasions = result
                        .pack
                        .relevant_episodes
                        .iter()
                        .filter(|episode| {
                            selected_cues(
                                result,
                                MemoryObjectRef::new(ObjectType::Episode, episode.id),
                            )
                            .contains(&CueKind::Participant)
                        })
                        .map(|episode| episode.id)
                        .chain(
                            result
                                .pack
                                .salient_observations
                                .iter()
                                .map(|observation| observation.episode_id),
                        )
                        .collect::<BTreeSet<_>>();
                    assert_eq!(
                        occasions,
                        BTreeSet::from([episode_id]),
                        "route={route}, reverse_ids={reverse_ids}, N={count}"
                    );
                    assert_eq!(result.pack.relevant_episodes.len(), count.min(8) as usize);
                    assert_eq!(
                        result.pack.salient_observations.len(),
                        usize::from(mentions)
                    );
                }
                for relation in [RelationType::Involves, RelationType::Mentions] {
                    let decision = |result: &RetrieveOutcome| {
                        let row = result
                            .trace
                            .as_ref()
                            .unwrap()
                            .selectivity_decisions
                            .iter()
                            .find(|row| row.root.id == participant && row.relation == relation)
                            .unwrap();
                        (row.chosen_fanout, row.fallback)
                    };
                    assert_eq!(decision(&healthy), (1, false));
                    assert_eq!(decision(&missing), (1, true));
                }
            }
            memory.close().await.unwrap();
        }
    }
}

#[tokio::test]
async fn ubiquitous_participants_limit_occasions_without_losing_beliefs() {
    let mut observed = Vec::new();
    for caller_built in [false, true] {
        // The application can regard either ordinary notion as its own identity.
        for ubiquitous in [100, 101] {
            let (memory, queries) = scene_memory().await;
            for id in 100..106 {
                create_notion(&memory, id, Some(&format!("Participant {id}"))).await;
            }
            let rare = 105;
            for index in 0..24 {
                let mut occasion = scene();
                occasion.participants = [
                    ubiquitous,
                    102,
                    103,
                    104,
                    if index >= 21 { rare } else { 201 - ubiquitous },
                ]
                .into_iter()
                .map(|id| SceneParticipant {
                    key: Some(MemoryId::from_u128(id)),
                    ..Default::default()
                })
                .collect();
                let id = 10_000 + index * 10;
                if caller_built {
                    let mut episode = EpisodeDraft::new("A group worked together.");
                    episode.id = Some(MemoryId::from_u128(id));
                    episode.created_at = Some(scene().time.to_utc());
                    episode.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
                    episode.scene = Some(occasion);
                    memory
                        .commit(
                            RememberWritePlan::new().with_candidate(MemoryCandidate::Episode(
                                EpisodeCandidate::new(
                                    episode,
                                    CandidateProvenance::caller("occasion"),
                                ),
                            )),
                            CommitOptions::default(),
                        )
                        .await
                        .unwrap();
                } else {
                    write_episode(&memory, id, occasion).await;
                }
            }
            let mut present = scene();
            present.participants = [ubiquitous, rare]
                .into_iter()
                .map(|id| SceneParticipant {
                    key: Some(MemoryId::from_u128(id)),
                    ..Default::default()
                })
                .collect();
            let result = memory
                .retrieve(RetrievalContext::default().with_scene(present).with_trace())
                .await
                .unwrap();
            let trace = result.trace.as_ref().unwrap();
            let relation = if caller_built {
                RelationType::Involves
            } else {
                RelationType::Mentions
            };
            let retained = |id, relation| {
                trace
                    .fanout_utilization
                    .iter()
                    .find(|row| row.root.id == MemoryId::from_u128(id) && row.relation == relation)
                    .unwrap()
                    .retained_count
            };
            let counts = trace
                .selectivity_decisions
                .iter()
                .find(|row| {
                    row.root.id == MemoryId::from_u128(ubiquitous) && row.relation == relation
                })
                .and_then(|row| row.entity_count.zip(row.global_count));
            observed.push((
                caller_built,
                ubiquitous,
                retained(ubiquitous, relation),
                retained(rare, relation),
                retained(ubiquitous, RelationType::About),
                result.pack.relevant_episodes.len() + result.pack.salient_observations.len(),
                counts,
            ));
            assert!(queries.lock().unwrap().is_empty());
            memory.close().await.unwrap();
        }
    }
    let expected = [false, true]
        .into_iter()
        .flat_map(|caller_built| {
            [100, 101].into_iter().map(move |ubiquitous| {
                (
                    caller_built,
                    ubiquitous,
                    1,
                    3,
                    1,
                    if caller_built { 8 } else { 12 },
                    Some((24, 24)),
                )
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(observed, expected);
}

#[tokio::test]
async fn thread_activity_reads_native_members_and_reports_found_after_filtering() {
    let (memory, queries) = scene_memory().await;
    let thread_id = MemoryId::from_u128(9000);
    let member_id = MemoryId::from_u128(9001);
    let older_member_id = MemoryId::from_u128(8999);
    let mut thread = MemoryThreadDraft::new("The telescope repair", "A completed piece of work");
    thread.id = Some(thread_id);
    thread.status = ThreadStatus::Resolved;
    let mut member = DerivedMemoryDraft::new(DerivedType::Claim, "The lens needs cleaning.");
    member.id = Some(member_id);
    member.created_at = Some(scene().time.to_utc());
    member.thread_ids = vec![thread_id];
    let mut older_member = DerivedMemoryDraft::new(DerivedType::Claim, "The lens was installed.");
    older_member.id = Some(older_member_id);
    older_member.created_at = Some(scene().time.to_utc() - chrono::Duration::days(1));
    older_member.thread_ids = vec![thread_id];
    memory
        .remember(
            RememberInput::new("We inspected the lens.")
                .with_scene(scene())
                .with_memory_thread(thread)
                .with_derived_memory(member)
                .with_derived_memory(older_member),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    let activity = ActivityRef::Thread(thread_id);
    let mut context = RetrievalContext::default()
        .with_activity(activity)
        .with_trace();
    context.candidate_limits.max_graph_roots = 2;
    let found = memory.retrieve(context.clone()).await.unwrap();
    let wrong_kind = ActivityRef::OpenLoop(member_id);
    let unknown = memory
        .retrieve(RetrievalContext::default().with_activity(wrong_kind))
        .await
        .unwrap();
    assert_eq!(
        unknown.activity,
        Some(ActivityResult {
            activity: wrong_kind,
            resolution: ActivityResolution::Unknown
        })
    );
    assert_eq!(unknown.pack.relevant_episodes.len(), 1);
    assert_eq!(unknown.memory_scenes.len(), 1);
    assert_eq!(
        found.activity,
        Some(ActivityResult {
            activity,
            resolution: ActivityResolution::Found
        })
    );
    assert_eq!(found.pack.derived_memories[0].memory.id, member_id);
    assert_eq!(
        selected_cues(
            &found,
            MemoryObjectRef::new(ObjectType::DerivedMemory, member_id)
        ),
        &BTreeSet::from([CueKind::Activity])
    );
    assert!(found.pack.active_threads.is_empty());
    assert!(
        found.trace.as_ref().unwrap().graph_relations.is_empty(),
        "native affiliation requires no link"
    );
    assert!(queries.lock().unwrap().is_empty());
    let mut mixed = context.clone();
    mixed.topic = Some("lens".to_owned());
    // Leave room for the topic head as well as the thread and its newest member.
    mixed.candidate_limits.max_graph_roots = 3;
    mixed.section_limits.derived_memories = 0;
    let mixed = memory.retrieve(mixed).await.unwrap();
    let trace = mixed.trace.unwrap();
    let member_ref = MemoryObjectRef::new(ObjectType::DerivedMemory, member_id);
    let vector_score = trace
        .vector_candidates
        .iter()
        .find(|candidate| candidate.object == member_ref)
        .unwrap()
        .score;
    assert!(trace.section_assignments.iter().any(|row| {
        row.object == member_ref
            && row.cue_kinds == BTreeSet::from([CueKind::Topic, CueKind::Activity])
            && matches!(row.reason, SectionAssignmentReason::OmittedByLimit { .. })
    }));
    assert!(trace.stale_candidate_omissions.iter().any(|omission| {
        omission.candidate == member_ref && omission.vector_score == Some(vector_score)
    }));
    assert_eq!(*queries.lock().unwrap(), ["lens"]);
    queries.lock().unwrap().clear();
    let mut limited = context.clone();
    limited.candidate_limits.max_graph_roots = 1;
    let bounded = memory.retrieve(limited).await.unwrap();
    assert_eq!(bounded.activity, found.activity);
    assert!(bounded.memory_scenes.is_empty());
    assert!(bounded.trace.unwrap().graph_expansions.iter().any(|root| {
        root.root.id == member_id && root.outcome == GraphExpansionOutcome::RootLimit
    }));
    for id in [member_id, older_member_id] {
        memory
            .forget(ForgetMemoryDraft::suppress(
                LifecycleTargetRef::derived_memory(id),
                "No longer useful",
            ))
            .await
            .unwrap();
    }
    let mut untraced = context;
    untraced.include_trace = false;
    let empty = memory.retrieve(untraced.clone()).await.unwrap();
    assert_eq!(empty.activity, found.activity);
    assert_eq!(empty.pack.relevant_episodes.len(), 1);
    assert_eq!(empty.memory_scenes.len(), 1);
    assert!(empty.trace.is_none());
    untraced.lifecycle_policy.include_suppressed = true;
    assert_eq!(
        memory
            .retrieve(untraced)
            .await
            .unwrap()
            .pack
            .derived_memories[0]
            .memory
            .id,
        member_id
    );
    assert!(
        queries.lock().unwrap().is_empty(),
        "an activity needs no content search"
    );
    memory.close().await.unwrap();
}

#[tokio::test]
async fn open_loop_activity_reads_sources_and_threads_and_respects_its_own_lifecycle() {
    let (memory, queries) = scene_memory().await;
    let episode_id = write_episode(&memory, 5000, scene()).await;
    write_episode(&memory, 6000, scene()).await;
    let observation_id = MemoryId::from_u128(6001);
    // Identity includes the kind: these two valid activities deliberately share a UUID.
    let id = MemoryId::from_u128(9200);
    let mut thread = MemoryThreadDraft::new("The unfinished repair", "Work in progress");
    thread.id = Some(id);
    let mut open_loop = DerivedMemoryDraft::new(DerivedType::OpenLoop, "Finish the repair.");
    open_loop.id = Some(id);
    open_loop.thread_ids = vec![id];
    open_loop.derived_from_episode_ids = vec![episode_id];
    open_loop.derived_from_observation_ids = vec![observation_id];
    memory
        .remember(
            RememberInput::new("The repair remains open.")
                .with_scene(scene())
                .with_memory_thread(thread)
                .with_derived_memory(open_loop.clone()),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    let activity = ActivityRef::OpenLoop(id);
    let context = RetrievalContext::default()
        .with_activity(activity)
        .with_trace();
    let encoded = serde_json::to_value(&context).unwrap();
    assert_eq!(
        encoded["activity"],
        serde_json::json!({"kind": "open_loop", "id": id})
    );
    let context: RetrievalContext = serde_json::from_value(encoded).unwrap();
    let found = memory.retrieve(context.clone()).await.unwrap();
    assert_eq!(
        serde_json::to_value(&found).unwrap()["activity"],
        serde_json::json!({
            "activity": {"kind": "open_loop", "id": id}, "resolution": "found"
        })
    );
    assert_eq!(
        found.activity,
        Some(ActivityResult {
            activity,
            resolution: ActivityResolution::Found
        })
    );
    for object in [
        MemoryObjectRef::new(ObjectType::Episode, episode_id),
        MemoryObjectRef::new(ObjectType::Observation, observation_id),
        MemoryObjectRef::new(ObjectType::MemoryThread, id),
        MemoryObjectRef::new(ObjectType::DerivedMemory, id),
    ] {
        assert_eq!(
            selected_cues(&found, object),
            &if object.object_type == ObjectType::Episode {
                BTreeSet::from([CueKind::Activity, CueKind::Recency])
            } else {
                BTreeSet::from([CueKind::Activity])
            }
        );
    }
    let mut thread_context = context.clone();
    assert_eq!(
        found
            .trace
            .as_ref()
            .unwrap()
            .graph_expansions
            .iter()
            .filter(|root| root.source != GraphRootSource::Recency)
            .map(|root| root.root)
            .collect::<Vec<_>>(),
        vec![
            MemoryObjectRef::new(ObjectType::DerivedMemory, id),
            MemoryObjectRef::new(ObjectType::Episode, episode_id),
            MemoryObjectRef::new(ObjectType::Observation, observation_id),
            MemoryObjectRef::new(ObjectType::MemoryThread, id),
        ]
    );
    thread_context.activity = Some(ActivityRef::Thread(id));
    assert_eq!(
        memory.retrieve(thread_context).await.unwrap().activity,
        Some(ActivityResult {
            activity: ActivityRef::Thread(id),
            resolution: ActivityResolution::Found
        })
    );
    for activity in [
        ActivityRef::Thread(episode_id),
        ActivityRef::OpenLoop(episode_id),
        ActivityRef::OpenLoop(MemoryId::from_u128(9999)),
    ] {
        let unknown = memory
            .retrieve(
                RetrievalContext::default()
                    .with_activity(activity)
                    .with_trace(),
            )
            .await
            .unwrap();
        assert_eq!(
            unknown.activity,
            Some(ActivityResult {
                activity,
                resolution: ActivityResolution::Unknown
            })
        );
        assert_eq!(unknown.pack.relevant_episodes.len(), 3);
        assert_eq!(unknown.memory_scenes.len(), 3);
        let expansions = unknown.trace.unwrap().graph_expansions;
        assert_eq!(expansions.len(), 3);
        assert!(expansions
            .iter()
            .all(|root| root.source == GraphRootSource::Recency));
        assert_eq!(
            expansions
                .iter()
                .map(|root| root.root.id)
                .collect::<BTreeSet<_>>(),
            unknown
                .pack
                .relevant_episodes
                .iter()
                .map(|episode| episode.id)
                .collect()
        );
    }
    let mut no_room = context.clone();
    no_room.graph_limits.max_nodes = 0;
    let bounded = memory.retrieve(no_room).await.unwrap();
    assert_eq!(bounded.activity, found.activity);
    assert!(bounded.memory_scenes.is_empty());
    let mut replacement = open_loop;
    replacement.id = Some(MemoryId::from_u128(9201));
    replacement.supersedes = vec![id];
    memory
        .remember(
            RememberInput::new("A revised plan for the repair.")
                .with_scene(scene())
                .with_derived_memory(replacement),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    let superseded = memory.retrieve(context.clone()).await.unwrap();
    assert_eq!(superseded.activity, found.activity);
    assert_eq!(superseded.pack.relevant_episodes[0].id, episode_id);
    assert_eq!(superseded.memory_scenes.len(), 4);
    assert_eq!(
        selected_cues(
            &superseded,
            MemoryObjectRef::new(ObjectType::Episode, episode_id)
        ),
        &BTreeSet::from([CueKind::Recency])
    );
    let mut historical = context.clone();
    historical.lifecycle_policy.include_superseded = true;
    assert!(memory
        .retrieve(historical)
        .await
        .unwrap()
        .pack
        .relevant_episodes
        .iter()
        .any(|episode| episode.id == episode_id));
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::derived_memory(id),
            "The original plan was forgotten.",
        ))
        .await
        .unwrap();
    for (include_suppressed, include_superseded) in
        [(false, false), (true, false), (false, true), (true, true)]
    {
        let mut context = context.clone();
        context.lifecycle_policy.include_suppressed = include_suppressed;
        context.lifecycle_policy.include_superseded = include_superseded;
        let result = memory.retrieve(context).await.unwrap();
        assert_eq!(result.activity, found.activity);
        assert_eq!(
            selected_cues(
                &result,
                MemoryObjectRef::new(ObjectType::Episode, episode_id)
            )
            .contains(&CueKind::Activity),
            include_suppressed && include_superseded
        );
    }
    assert!(queries.lock().unwrap().is_empty());
    memory.close().await.unwrap();
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
    episode.created_at = Some(past.time.to_utc());
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
    assert_eq!(result.pack.relevant_episodes.len(), 2);
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
    // Reference resolution should cue an occasional participant.
    write_episode(&memory, 6000, scene()).await;
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
    assert!(trace.graph_expansions.iter().all(|entry| matches!(
        entry.source,
        GraphRootSource::Participant | GraphRootSource::Recency
    )));
    assert!(key_result.rationale.telemetry.selectivity.decision_count > 0);
    assert!(trace.section_assignments.iter().any(|row| {
        matches!(row.reason, SectionAssignmentReason::Selected { .. })
            && row.cue_kinds == std::collections::BTreeSet::from([CueKind::Participant])
    }));

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
        4
    );

    present.participants = vec![keyed(999), keyed(300)];
    let empty = memory
        .retrieve(RetrievalContext::default().with_scene(present.clone()))
        .await
        .unwrap();
    assert_eq!(empty.pack.relevant_episodes.len(), 2);
    assert_eq!(empty.memory_scenes.len(), 2);
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
    past.participants = vec![
        SceneParticipant {
            description: Some("astronomer".to_owned()),
            ..Default::default()
        };
        2
    ];
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
    let described = MemoryObjectRef::new(ObjectType::Episode, episode_id);
    assert_eq!(
        selected_cues(&description_only, described),
        &BTreeSet::from([CueKind::Participant, CueKind::Recency])
    );
    assert_eq!(*queries.lock().unwrap(), ["astronomer\nastronomer"]);
    assert_eq!(description_only.pack.relevant_episodes[0].id, episode_id);
    assert!(description_only.pack.derived_memories.is_empty());
    assert_eq!(
        description_only.trace.as_ref().unwrap().scene_cue_searches[0].references,
        [
            SceneReference::ParticipantDescription { index: 0 },
            SceneReference::ParticipantDescription { index: 1 },
        ]
    );
    assert_eq!(
        description_only.trace.as_ref().unwrap().vector_candidates[0].surface,
        VectorSurface::SceneParticipants
    );
    assert!(description_only
        .scene_references
        .iter()
        .all(|reference| reference.resolution == SceneReferenceResolution::ContentCue));

    let named_belief = MemoryObjectRef::new(ObjectType::DerivedMemory, MemoryId::from_u128(1100));
    let topic_only = memory
        .retrieve(RetrievalContext::new("astronomer").with_trace())
        .await
        .unwrap();
    assert_eq!(
        selected_cues(&topic_only, named_belief),
        &BTreeSet::from([CueKind::Topic])
    );
    context.topic = Some("astronomer".to_owned());
    context.scene.participants.truncate(1);
    context.scene.setting.words = Some("astronomer".to_owned());
    context.candidate_limits.max_vector_candidates = 48;
    queries.lock().unwrap().clear();
    let shared_text = memory.retrieve(context.clone()).await.unwrap();
    assert_eq!(*queries.lock().unwrap(), ["astronomer"]);
    assert_eq!(
        selected_cues(&shared_text, named_belief),
        &BTreeSet::from([CueKind::Topic])
    );
    assert!(selected_cues(&shared_text, described).contains(&CueKind::Participant));
    assert!(selected_cues(&shared_text, described).contains(&CueKind::Place));
    context.candidate_limits.max_vector_candidates = 1;
    context.scene.setting.words = Some("observatory".to_owned());
    let budgeted = memory.retrieve(context).await.unwrap();
    assert_eq!(budgeted.trace.unwrap().vector_candidates.len(), 1);
    let mut place = RetrievalContext::default();
    place.scene.setting.words = Some("observatory".to_owned());
    place.candidate_limits.max_vector_candidates = 1;
    let place_result = memory.retrieve(place.clone()).await.unwrap();
    assert!(place_result.trace.is_none());
    let traced_place = memory.retrieve(place.with_trace()).await.unwrap();
    assert_eq!(traced_place.pack, place_result.pack);
    assert_eq!(
        selected_cues(
            &traced_place,
            MemoryObjectRef::new(ObjectType::Episode, episode_id)
        ),
        &BTreeSet::from([CueKind::Place, CueKind::Recency])
    );
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
            resolution: SceneReferenceResolution::ContentCue,
            ..
        }
    ));
}

#[tokio::test]
async fn description_search_scores_are_shared_and_precede_occasion_selection() {
    let (memory, _) = scene_memory().await;
    let mut present = scene();
    present.setting.words = Some("observatory".to_owned());
    present.participants = vec![
        SceneParticipant {
            name: Some("astronomer".to_owned()),
            ..Default::default()
        },
        SceneParticipant {
            description: Some("navigator".to_owned()),
            ..Default::default()
        },
    ];
    let mut past = present.clone();
    past.time -= chrono::Duration::days(1);
    write_episode(&memory, 9000, past).await;
    let context = RetrievalContext::default()
        .with_scene(present.clone())
        .with_trace();
    let identical = memory.retrieve(context.clone()).await.unwrap();
    let searches = &identical.trace.as_ref().unwrap().scene_cue_searches;
    assert_eq!(searches.len(), 2);
    assert_eq!(
        (searches[0].cue_kind, searches[1].cue_kind),
        (crate::CueKind::Place, crate::CueKind::Participant)
    );
    assert!(searches.iter().all(|search| search.omitted_count == 0));
    assert_eq!(searches[0].references, [SceneReference::SettingWords]);
    assert_eq!(
        searches[1].references,
        [
            SceneReference::ParticipantName { index: 0 },
            SceneReference::ParticipantDescription { index: 1 },
        ]
    );
    assert!(searches
        .iter()
        .all(|search| (search.best_score.unwrap() - 1.0).abs() < 1e-6));

    let mut reworded = context.clone();
    reworded.scene.setting.words = Some("astronomer observatory".to_owned());
    reworded.scene.participants[0].name = Some("stargazer".to_owned());
    let reworded = memory.retrieve(reworded).await.unwrap();
    let searches = &reworded.trace.unwrap().scene_cue_searches;
    assert!((searches[0].best_score.unwrap() - 0.70886356).abs() < 1e-6);
    assert!((searches[1].best_score.unwrap() - 0.09950372).abs() < 1e-6);

    // Oppose IDs to time; the selected latest occasion is not the best match.
    let mut latest = present;
    latest.setting.words = Some("planetarium".to_owned());
    latest.participants[0].name = Some("stargazer".to_owned());
    write_episode(&memory, 8000, latest).await;
    let recent = memory.retrieve(context.clone()).await.unwrap();
    let trace = recent.trace.as_ref().unwrap();
    assert_eq!(trace.vector_candidates.len(), 1);
    assert_eq!(
        trace.vector_candidates[0].object.id,
        MemoryId::from_u128(8000)
    );
    assert!((trace.vector_candidates[0].score - 0.09950372).abs() < 1e-6);
    assert!(trace
        .scene_cue_searches
        .iter()
        .all(|search| (search.best_score.unwrap() - 1.0).abs() < 1e-6));
    assert!(trace
        .scene_cue_searches
        .iter()
        .all(|search| search.omitted_count == 1));
    let mut untraced_context = context.clone();
    untraced_context.include_trace = false;
    let untraced = memory.retrieve(untraced_context).await.unwrap();
    let mut expected = recent;
    expected.trace = None;
    assert_eq!(untraced, expected);

    let (without_surfaces, _) = scene_memory().await;
    write_episode(&without_surfaces, 7000, scene()).await;
    let empty = without_surfaces.retrieve(context).await.unwrap();
    let searches = empty.trace.unwrap().scene_cue_searches;
    assert_eq!(searches.len(), 2);
    assert!(searches.iter().all(|search| search.best_score.is_none()));
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
    belief.created_at = Some(scene().time.to_utc());
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
async fn result_reports_all_source_scenes_beside_recent_episodes_without_trace() {
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
    thread.created_at = Some(scene().time.to_utc());
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
    assert_eq!(
        result
            .pack
            .relevant_episodes
            .iter()
            .map(|episode| episode.id.as_u128())
            .collect::<Vec<_>>(),
        [5000, 6000]
    );
    assert!(result.pack.salient_observations.is_empty());
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
