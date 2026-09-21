use std::collections::BTreeSet;

use async_trait::async_trait;

use crate::api::types::*;
use crate::domain::*;
use crate::models::vector::EmbeddingInput;
use crate::ports::embedder::MemoryEmbedder;
use crate::test_support::{in_memory_graph_store, TemporaryVectorCandidateStore};
use crate::{CharacterMemory, CustomError};

struct FloorEmbedder;

#[async_trait]
impl MemoryEmbedder for FloorEmbedder {
    async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
        let (axis, score) = if input.surface == VectorSurface::Query {
            (
                match input.text.as_str() {
                    "topic" | "orchids" => 0,
                    "person" => 1,
                    "place" => 2,
                    "work" => 3,
                    text => panic!("unexpected query {text}"),
                },
                1.0_f32,
            )
        } else if input.text.contains("orchids") {
            (
                0,
                if input.text.contains("work") {
                    0.2
                } else {
                    0.89
                },
            )
        } else {
            match input.object_id.map(|id| id.as_u128()) {
                Some(1000..=1046) => (0, 0.99),
                Some(1047) => (0, 0.96),
                Some(2000) => (1, 0.98),
                Some(2001..=2047) => (1, 0.8),
                Some(3000..=3047) => (2, 0.985),
                _ => (3, 1.0),
            }
        };
        let mut vector = vec![0.0; 4];
        vector[axis] = score;
        if axis != 3 {
            vector[3] = (1.0 - score * score).sqrt();
        }
        Ok(vector)
    }

    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        let mut vectors = Vec::new();
        for input in inputs {
            vectors.push(self.embed(input).await?);
        }
        Ok(vectors)
    }
}

fn occasion() -> Scene {
    Scene::at("2026-09-21T00:00:00Z".parse().unwrap())
}

async fn floor_memory(scene_surfaces: bool, overlap: bool) -> CharacterMemory {
    let memory = CharacterMemory::from_parts(
        Box::new(in_memory_graph_store()),
        Box::new(TemporaryVectorCandidateStore::open(4).await),
        Box::new(FloorEmbedder),
    );
    let provenance = || CandidateProvenance::caller("floor pressure");
    let mut episode = EpisodeDraft::new("An occasion with several recollections.");
    episode.id = Some(MemoryId::from_u128(1));
    episode.scene = Some(occasion());
    episode.created_at = Some(occasion().time);
    episode.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut thread = MemoryThreadDraft::new("Work in progress", "The current activity.");
    thread.id = Some(MemoryId::from_u128(5000));
    thread.created_at = Some(occasion().time);
    thread.updated_at = thread.created_at;
    thread.last_touched_at = thread.created_at;
    thread.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut plan = RememberWritePlan::new()
        .with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
            episode,
            provenance(),
        )))
        .with_candidate(MemoryCandidate::MemoryThread(MemoryThreadCandidate::new(
            thread,
            provenance(),
        )));
    for id in (1000..1048)
        .chain(2000..2048)
        .chain(3000..3048)
        .chain([4000])
    {
        let object_type = if scene_surfaces {
            ObjectType::Episode
        } else {
            ObjectType::Observation
        };
        if scene_surfaces {
            let mut scene = occasion();
            scene.time -= chrono::Duration::days((id % 1000) as i64);
            if (3000..3048).contains(&id) || (overlap && id == 1000) {
                scene.setting.words = Some("place".to_owned());
            }
            if (2000..2048).contains(&id) || (overlap && id == 1000) {
                scene.participants.push(SceneParticipant {
                    description: Some("person".to_owned()),
                    ..Default::default()
                });
            }
            let mut episode = EpisodeDraft::new(format!("Recollection {id}"));
            episode.id = Some(MemoryId::from_u128(id));
            episode.created_at = Some(scene.time);
            episode.scene = Some(scene);
            episode.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
            episode.salience_score = 0.0;
            plan = plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
                episode,
                provenance(),
            )));
        } else {
            let mut observation =
                ObservationDraft::new(MemoryId::from_u128(1), format!("Recollection {id}"));
            observation.id = Some(MemoryId::from_u128(id));
            observation.observed_at = Some(occasion().time);
            observation.created_at = observation.observed_at;
            observation.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
            observation.salience_score = 0.0;
            plan = plan.with_candidate(MemoryCandidate::Observation(ObservationCandidate::new(
                observation,
                provenance(),
            )));
        }
        if id != 4000 {
            plan = plan.with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                MemoryObjectRef::new(object_type, MemoryId::from_u128(id)),
                provenance(),
            )));
        }
    }
    memory.commit(plan, CommitOptions::default()).await.unwrap();
    link_activity(
        &memory,
        4000,
        if scene_surfaces {
            ObjectType::Episode
        } else {
            ObjectType::Observation
        },
    )
    .await;
    memory
}

async fn link_activity(memory: &CharacterMemory, observation: u128, object_type: ObjectType) {
    memory
        .link(MemoryLinkDraft::new(
            ObjectType::MemoryThread,
            MemoryId::from_u128(5000),
            RelationType::AssociatedWith,
            object_type,
            MemoryId::from_u128(observation),
        ))
        .await
        .unwrap();
}

fn mixed_context() -> RetrievalContext {
    let mut context = RetrievalContext::new("topic")
        .with_activity(ActivityRef::Thread(MemoryId::from_u128(5000)))
        .with_trace();
    context.scene = occasion();
    context.scene.participants.push(SceneParticipant {
        description: Some("person".to_owned()),
        ..Default::default()
    });
    context.scene.setting.words = Some("place".to_owned());
    context.graph_limits.max_depth = 1;
    context.section_limits.salient_observations = 8;
    context
}

async fn overlapping_cue_memory() -> (CharacterMemory, MemoryId) {
    let memory = CharacterMemory::from_parts(
        Box::new(in_memory_graph_store()),
        Box::new(TemporaryVectorCandidateStore::open(4).await),
        Box::new(FloorEmbedder),
    );
    let mut thread = MemoryThreadDraft::new("Work in progress", "The current activity.");
    thread.id = Some(MemoryId::from_u128(5000));
    let mut input = RememberInput::new("Progress on the work.")
        .with_scene(occasion())
        .with_memory_thread(thread);
    for id in 6000..6012 {
        let mut member = DerivedMemoryDraft::new(
            DerivedType::Claim,
            "The work included a passing mention of orchids.",
        );
        member.id = Some(MemoryId::from_u128(id));
        member.thread_ids = vec![MemoryId::from_u128(5000)];
        member.created_at = Some(occasion().time);
        input = input.with_derived_memory(member);
    }
    let strong_id = MemoryId::from_u128(7000);
    let mut strong = DerivedMemoryDraft::new(DerivedType::Claim, "orchids need careful watering.");
    strong.id = Some(strong_id);
    input = input.with_derived_memory(strong);
    memory
        .remember(input, RememberOptions::default())
        .await
        .unwrap();
    (memory, strong_id)
}

async fn overlapping_scene_memory() -> (CharacterMemory, MemoryId) {
    let memory = CharacterMemory::from_parts(
        Box::new(in_memory_graph_store()),
        Box::new(TemporaryVectorCandidateStore::open(4).await),
        Box::new(FloorEmbedder),
    );
    for id in (6000..6012).chain([7000]) {
        let text = if id == 7000 {
            "orchids need careful watering."
        } else {
            "The work included a passing mention of orchids."
        };
        let mut scene = occasion();
        scene.time -= chrono::Duration::days((id - 6000) as i64);
        scene.setting.words = Some(text.to_owned());
        let mut episode = EpisodeDraft::new(text);
        episode.id = Some(MemoryId::from_u128(id));
        episode.created_at = Some(scene.time);
        episode.scene = Some(scene);
        episode.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        let provenance = CandidateProvenance::caller("overlapping surfaces");
        let plan = RememberWritePlan::new()
            .with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
                episode,
                provenance.clone(),
            )))
            .with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                MemoryObjectRef::new(ObjectType::Episode, MemoryId::from_u128(id)),
                provenance,
            )));
        memory.commit(plan, CommitOptions::default()).await.unwrap();
    }
    (memory, MemoryId::from_u128(7000))
}
#[tokio::test]
async fn weak_topic_membership_does_not_spend_the_strong_topic_roots_turn() {
    let (memory, strong_id) = overlapping_cue_memory().await;
    let mut outcomes = Vec::new();
    let mut included = Vec::new();
    for place_cue in [false, true] {
        let mut context = RetrievalContext::new("orchids")
            .with_scene(occasion())
            .with_trace();
        if place_cue {
            context.scene.setting.words = Some("work".to_owned());
        } else {
            context.activity = Some(ActivityRef::Thread(MemoryId::from_u128(5000)));
        }
        context.graph_limits.max_depth = 0;
        context.candidate_limits.max_graph_roots = 12;
        context.section_limits.derived_memories = 12;
        let result = memory.retrieve(context).await.unwrap();
        let trace = result.trace.unwrap();
        if !place_cue {
            assert_eq!(trace.vector_candidates[0].object.id, strong_id);
            assert!((trace.vector_candidates[0].score - 0.89).abs() < 0.0001);
            for id in 6000..6012 {
                let row = trace
                    .vector_candidates
                    .iter()
                    .find(|row| row.object.id == MemoryId::from_u128(id))
                    .unwrap();
                assert!((row.score - 0.2).abs() < 0.0001);
            }
        }
        let strong_root = trace
            .graph_expansions
            .iter()
            .find(|row| row.root.id == strong_id)
            .unwrap();
        outcomes.push(strong_root.outcome);
        included.push(
            result
                .pack
                .derived_memories
                .iter()
                .any(|entry| entry.memory.id == strong_id),
        );
    }
    assert_eq!(outcomes, [GraphExpansionOutcome::Expanded; 2]);
    assert_eq!(included, [true; 2]);
    memory.close().await.unwrap();
}

#[tokio::test]
async fn overlapping_cue_orders_protect_the_topic_at_candidate_merge() {
    let (memory, strong_id) = overlapping_scene_memory().await;
    let mut context = RetrievalContext::new("orchids").with_trace();
    context.object_type_defaults = vec![ObjectType::Episode];
    context.scene.setting.words = Some("work".to_owned());
    context.candidate_limits.max_vector_candidates = 12;
    context.candidate_limits.max_graph_roots = 48;
    context.graph_limits.max_depth = 0;
    context.section_limits.relevant_episodes = 48;
    let mut topic_only = context.clone();
    topic_only.scene.setting.words = None;
    let topic_only = memory.retrieve(topic_only).await.unwrap();
    let candidates = topic_only.trace.unwrap().vector_candidates;
    assert_eq!(candidates[0].object.id, strong_id);
    assert!((candidates[0].score - 0.89).abs() < 0.0001);
    assert!(candidates[1..]
        .iter()
        .all(|candidate| (candidate.score - 0.2).abs() < 0.0001));
    let result = memory.retrieve(context).await.unwrap();
    let trace = result.trace.unwrap();
    assert_eq!(trace.vector_candidates.len(), 12);
    assert!(
        trace
            .vector_candidates
            .iter()
            .any(|row| row.object.id == strong_id),
        "the strongest Topic hit was lost at candidate merge"
    );
    assert!(!trace.floor_admissions.contains(&CueFloorAdmission {
        object: MemoryObjectRef::new(ObjectType::Episode, strong_id),
        stage: CueFloorStage::CandidateMerge,
        cue_kind: CueKind::Topic,
    }));
    assert!(trace
        .graph_expansions
        .iter()
        .any(|row| row.root.id == strong_id && row.outcome == GraphExpansionOutcome::Expanded));
    assert!(result
        .pack
        .relevant_episodes
        .iter()
        .any(|row| row.id == strong_id));
    memory.close().await.unwrap();
}

#[tokio::test]
async fn overlapping_cue_orders_protect_the_topic_at_section_cap() {
    let (memory, strong_id) = overlapping_scene_memory().await;
    let mut context = RetrievalContext::new("orchids").with_trace();
    context.object_type_defaults = vec![ObjectType::Episode];
    context.scene.setting.words = Some("work".to_owned());
    context.candidate_limits.max_vector_candidates = 48;
    context.candidate_limits.max_graph_roots = 48;
    context.graph_limits.max_depth = 0;
    context.section_limits.relevant_episodes = 2;
    let result = memory.retrieve(context).await.unwrap();
    let trace = result.trace.unwrap();
    assert!(trace
        .vector_candidates
        .iter()
        .any(|row| row.object.id == strong_id));
    assert!(trace
        .graph_expansions
        .iter()
        .any(|row| row.root.id == strong_id && row.outcome == GraphExpansionOutcome::Expanded));
    let assignment = trace
        .section_assignments
        .iter()
        .find(|row| row.object.id == strong_id)
        .unwrap();
    assert_eq!(result.pack.relevant_episodes.len(), 2);
    assert!(!trace.floor_admissions.contains(&CueFloorAdmission {
        object: MemoryObjectRef::new(ObjectType::Episode, strong_id),
        stage: CueFloorStage::Section {
            section: ContextPackSection::RelevantEpisodes,
        },
        cue_kind: CueKind::Topic,
    }));
    assert!(
        result
            .pack
            .relevant_episodes
            .iter()
            .any(|row| row.id == strong_id),
        "the strongest Topic hit was lost only at the section cap: {assignment:?}"
    );
    memory.close().await.unwrap();
}

#[tokio::test]
async fn a_large_activity_shares_roots_with_the_topic() {
    let memory = floor_memory(false, false).await;
    let mut input = RememberInput::new("Progress on the work.").with_scene(occasion());
    for id in 6000..6016 {
        let mut member = DerivedMemoryDraft::new(DerivedType::Claim, "A detail of the work.");
        member.id = Some(MemoryId::from_u128(id));
        member.thread_ids = vec![MemoryId::from_u128(5000)];
        member.created_at = Some(occasion().time);
        input = input.with_derived_memory(member);
    }
    memory
        .remember(input, RememberOptions::default())
        .await
        .unwrap();
    let mut context = RetrievalContext::new("topic")
        .with_activity(ActivityRef::Thread(MemoryId::from_u128(5000)))
        .with_scene(occasion())
        .with_trace();
    context.graph_limits.max_depth = 1;
    let first = memory.retrieve(context.clone()).await.unwrap();
    let second = memory.retrieve(context).await.unwrap();
    assert_eq!(first.pack, second.pack);
    assert_eq!(first.trace, second.trace);
    let topic_memories = first
        .pack
        .salient_observations
        .iter()
        .filter(|memory| (1000..1048).contains(&memory.id.as_u128()))
        .count();
    assert!(
        topic_memories >= 3,
        "topic brought only {topic_memories} memories"
    );
    assert!(first.pack.derived_memories.len() >= 3);
    memory.close().await.unwrap();
}

#[tokio::test]
async fn configured_root_floors_are_reserved_before_spare_slots_are_shared() {
    let memory = floor_memory(false, false).await;
    let mut input = RememberInput::new("Progress on the work.").with_scene(occasion());
    for id in 6000..6016 {
        let mut member = DerivedMemoryDraft::new(DerivedType::Claim, "A detail of the work.");
        member.id = Some(MemoryId::from_u128(id));
        member.thread_ids = vec![MemoryId::from_u128(5000)];
        member.created_at = Some(occasion().time);
        input = input.with_derived_memory(member);
    }
    memory
        .remember(input, RememberOptions::default())
        .await
        .unwrap();
    for (cap, activity_count, topic_count) in [(6, 5, 1), (8, 6, 2), (4, 3, 1)] {
        let mut context = RetrievalContext::new("topic")
            .with_activity(ActivityRef::Thread(MemoryId::from_u128(5000)))
            .with_scene(occasion())
            .with_trace();
        context.candidate_limits.max_graph_roots = cap;
        context.graph_limits.max_depth = 1;
        context.cue_floors = RetrievalCueFloors {
            participant: 0,
            place: 0,
            activity: 5,
            topic: 1,
        };
        let result = memory.retrieve(context.clone()).await.unwrap();
        let repeat = memory.retrieve(context).await.unwrap();
        assert_eq!(result.pack, repeat.pack);
        assert_eq!(result.trace, repeat.trace);
        let expanded = result
            .trace
            .unwrap()
            .graph_expansions
            .into_iter()
            .filter(|row| row.outcome == GraphExpansionOutcome::Expanded)
            .map(|row| row.root.id.as_u128())
            .collect::<Vec<_>>();
        let topics = expanded
            .iter()
            .filter(|id| (1000..1048).contains(*id))
            .count();
        assert_eq!(
            (expanded.len() - topics, topics),
            (activity_count, topic_count),
            "cap={cap}"
        );
    }
    memory.close().await.unwrap();
}

#[tokio::test]
async fn floors_preserve_witnesses_lost_at_three_different_caps() {
    let memory = floor_memory(true, false).await;
    let result = memory.retrieve(mixed_context()).await.unwrap();
    let trace = result.trace.as_ref().unwrap();
    let stages = [2000, 3000, 4000].map(|id| {
        let id = MemoryId::from_u128(id);
        (
            trace
                .vector_candidates
                .iter()
                .any(|row| row.object.id == id),
            trace
                .graph_expansions
                .iter()
                .find(|row| row.root.id == id)
                .map(|row| row.outcome),
            trace
                .section_assignments
                .iter()
                .find(|row| row.object.id == id)
                .map(|row| row.section),
        )
    });
    let selected = result
        .pack
        .relevant_episodes
        .iter()
        .map(|object| object.id.as_u128())
        .collect::<Vec<_>>();
    assert!(
        [2000, 3000, 4000].iter().all(|id| selected.contains(id)),
        "{stages:?}"
    );
    // At 61fbb29 these witnesses first disappeared at candidate merge, roots,
    // and the section respectively. Later caps can also need to protect them.
    let section = CueFloorStage::Section {
        section: ContextPackSection::RelevantEpisodes,
    };
    let expected = [
        (2000, CueFloorStage::CandidateMerge, CueKind::Participant),
        (2000, CueFloorStage::GraphRoots, CueKind::Participant),
        (3000, CueFloorStage::GraphRoots, CueKind::Place),
        (2000, section, CueKind::Participant),
        (3000, section, CueKind::Place),
        (4000, section, CueKind::Activity),
    ];
    assert_eq!(trace.floor_admissions.len(), 6);
    for (stage, count) in [
        (CueFloorStage::CandidateMerge, 1),
        (CueFloorStage::GraphRoots, 2),
        (section, 3),
    ] {
        assert_eq!(
            trace
                .floor_admissions
                .iter()
                .filter(|row| row.stage == stage)
                .count(),
            count
        );
    }
    for (id, stage, cue_kind) in expected {
        assert!(trace.floor_admissions.contains(&CueFloorAdmission {
            object: MemoryObjectRef::new(ObjectType::Episode, MemoryId::from_u128(id)),
            stage,
            cue_kind,
        }));
    }
    for (id, kind) in [
        (2000, CueKind::Participant),
        (3000, CueKind::Place),
        (4000, CueKind::Activity),
    ] {
        let row = trace
            .section_assignments
            .iter()
            .find(|row| row.object.id == MemoryId::from_u128(id))
            .unwrap();
        assert_eq!(row.cue_kinds, BTreeSet::from([kind]));
    }
    let encoded = serde_json::to_value(trace).unwrap();
    let decoded: RetrievalTrace = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded.floor_admissions, trace.floor_admissions);
    let mut untraced = mixed_context();
    untraced.include_trace = false;
    let untraced = memory.retrieve(untraced).await.unwrap();
    assert_eq!(untraced.pack, result.pack);
    assert!(untraced.trace.is_none());
    let mut no_section = mixed_context();
    no_section.section_limits.relevant_episodes = 0;
    let no_section = memory.retrieve(no_section).await.unwrap();
    assert!(no_section.pack.relevant_episodes.is_empty());
    let admissions = no_section.trace.unwrap().floor_admissions;
    assert_eq!(admissions.len(), 3);
    assert!(admissions
        .iter()
        .all(|row| !matches!(row.stage, CueFloorStage::Section { .. })));
    memory.close().await.unwrap();
}

#[tokio::test]
async fn default_depth_credits_participant_inherited_through_the_episode() {
    let memory = CharacterMemory::from_parts(
        Box::new(in_memory_graph_store()),
        Box::new(TemporaryVectorCandidateStore::open(4).await),
        Box::new(FloorEmbedder),
    );
    let provenance = || CandidateProvenance::caller("shared occasion");
    let mut person = EntityDraft::new();
    person.id = Some(MemoryId::from_u128(7));
    person.created_at = Some(occasion().time);
    person.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut plan = RememberWritePlan::new().with_candidate(MemoryCandidate::Entity(
        EntityCandidate::new(person, provenance()),
    ));
    let mut shared_scene = occasion();
    shared_scene.participants.push(SceneParticipant {
        key: Some(MemoryId::from_u128(7)),
        ..Default::default()
    });
    for (id, scene) in [(1, occasion()), (2, shared_scene.clone())] {
        let mut episode = EpisodeDraft::new("An occasion.");
        episode.id = Some(MemoryId::from_u128(id));
        episode.created_at = Some(scene.time);
        episode.scene = Some(scene);
        episode.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        plan = plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
            episode,
            provenance(),
        )));
    }
    // The topic finds 1000 and 1001 directly. Participant 7 reaches 1001 and
    // its sibling 4000 only through episode 2; neither observation mentions 7.
    for (id, episode) in [(1000, 1), (1001, 2), (4000, 2)] {
        let mut observation =
            ObservationDraft::new(MemoryId::from_u128(episode), format!("Recollection {id}"));
        observation.id = Some(MemoryId::from_u128(id));
        observation.observed_at = Some(occasion().time);
        observation.created_at = observation.observed_at;
        observation.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        observation.salience_score = 0.0;
        plan = plan.with_candidate(MemoryCandidate::Observation(ObservationCandidate::new(
            observation,
            provenance(),
        )));
        if id != 4000 {
            plan = plan.with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                MemoryObjectRef::new(ObjectType::Observation, MemoryId::from_u128(id)),
                provenance(),
            )));
        }
    }
    memory.commit(plan, CommitOptions::default()).await.unwrap();
    for (id, episode) in [(1000, 1), (1001, 2), (4000, 2)] {
        memory
            .link(MemoryLinkDraft::new(
                ObjectType::Observation,
                MemoryId::from_u128(id),
                RelationType::ObservedIn,
                ObjectType::Episode,
                MemoryId::from_u128(episode),
            ))
            .await
            .unwrap();
    }
    let mut context = RetrievalContext::new("topic")
        .with_scene(shared_scene)
        .with_trace();
    context.section_limits.salient_observations = 1;
    let result = memory.retrieve(context.clone()).await.unwrap();
    let trace = result.trace.unwrap();
    assert_eq!(result.pack.salient_observations.len(), 1);
    assert_eq!(
        result.pack.salient_observations[0].id,
        MemoryId::from_u128(1001)
    );
    assert_eq!(
        trace
            .vector_candidates
            .iter()
            .map(|row| row.object.id.as_u128())
            .collect::<Vec<_>>(),
        [1000, 1001]
    );
    assert_eq!(
        trace.floor_admissions,
        [CueFloorAdmission {
            object: MemoryObjectRef::new(ObjectType::Observation, MemoryId::from_u128(1001)),
            stage: CueFloorStage::Section {
                section: ContextPackSection::SalientObservations
            },
            cue_kind: CueKind::Participant,
        }]
    );
    for id in [1001, 4000] {
        let row = trace
            .section_assignments
            .iter()
            .find(|row| row.object.id == MemoryId::from_u128(id))
            .unwrap();
        assert_eq!(
            row.cue_kinds,
            BTreeSet::from([CueKind::Topic, CueKind::Participant])
        );
    }
    assert!(trace.graph_relations.iter().any(|row| {
        row.from.id == MemoryId::from_u128(2)
            && row.to.id == MemoryId::from_u128(7)
            && row.relation == RelationType::Involves
    }));
    // The floor row identifies the credited kind, not its direct/inherited
    // origin. Here only Topic searched content, so Participant was inherited.
    context.cue_floors.participant = 0;
    let unprotected = memory.retrieve(context).await.unwrap();
    assert_eq!(unprotected.pack.salient_observations.len(), 1);
    assert_eq!(
        unprotected.pack.salient_observations[0].id,
        MemoryId::from_u128(1000)
    );
    assert!(unprotected.trace.unwrap().floor_admissions.is_empty());
    memory.close().await.unwrap();
}

#[tokio::test]
async fn participant_and_place_keep_room_without_a_topic() {
    let memory = floor_memory(true, false).await;
    let mut context = mixed_context();
    context.topic = None;
    context.activity = None;
    context.candidate_limits.max_vector_candidates = 96;
    context.candidate_limits.max_graph_roots = 96;
    context.section_limits.relevant_episodes = 6;
    let result = memory.retrieve(context.clone()).await.unwrap();
    let repeated = memory.retrieve(context).await.unwrap();
    assert_eq!(result.pack, repeated.pack);
    assert_eq!(result.trace, repeated.trace);
    let selected = result
        .pack
        .relevant_episodes
        .iter()
        .map(|object| object.id.as_u128())
        .collect::<Vec<_>>();
    assert_eq!(selected, [3000, 2000]);
    assert!(result.trace.unwrap().floor_admissions.is_empty());
    memory.close().await.unwrap();
}

#[tokio::test]
async fn single_kind_keeps_section_ids_and_order() {
    let memory = floor_memory(true, false).await;
    for kind in [
        CueKind::Topic,
        CueKind::Participant,
        CueKind::Place,
        CueKind::Activity,
    ] {
        let mut context = mixed_context();
        if kind != CueKind::Topic {
            context.topic = None;
        }
        if kind != CueKind::Participant {
            context.scene.participants.clear();
        }
        if kind != CueKind::Place {
            context.scene.setting.words = None;
        }
        if kind != CueKind::Activity {
            context.activity = None;
        }
        for (candidates, roots, section) in [(48, 12, 8), (3, 2, 1)] {
            context.candidate_limits.max_vector_candidates = candidates;
            context.candidate_limits.max_graph_roots = roots;
            context.section_limits.relevant_episodes = section;
            let result = memory.retrieve(context.clone()).await.unwrap();
            let mut prefix_context = context.clone();
            prefix_context.cue_floors = RetrievalCueFloors {
                participant: 0,
                place: 0,
                activity: 0,
                topic: 0,
            };
            let prefix = memory.retrieve(prefix_context).await.unwrap();
            assert_eq!(
                serde_json::to_vec(&result).unwrap(),
                serde_json::to_vec(&prefix).unwrap(),
                "single-kind retrieval must preserve the original ranked prefix"
            );
            // Exact section/id/order projections captured at 61fbb29.
            let first = match kind {
                CueKind::Topic => 1000,
                CueKind::Participant => 2000,
                CueKind::Place => 3000,
                CueKind::Activity => 4000,
            };
            let count = if kind != CueKind::Topic { 1 } else { section };
            let pack = result.pack;
            assert_eq!(
                pack.relevant_episodes
                    .iter()
                    .map(|object| object.id.as_u128())
                    .collect::<Vec<_>>(),
                (first..first + count as u128).collect::<Vec<_>>(),
                "{kind:?} {candidates}/{roots}/{section}"
            );
            assert_eq!(
                pack.active_threads
                    .iter()
                    .map(|object| object.id.as_u128())
                    .collect::<Vec<_>>(),
                if kind == CueKind::Activity {
                    vec![5000]
                } else {
                    vec![]
                }
            );
            assert!(pack.salient_observations.is_empty());
            assert!(pack.derived_memories.is_empty());
            assert!(pack.preferences.is_empty());
            assert!(pack.relationship_notes.is_empty());
            assert!(pack.open_loops.is_empty());
            assert!(pack.commitments.is_empty());
            assert!(pack.character_signals.is_empty());
            assert!(result.trace.unwrap().floor_admissions.is_empty());
        }
    }
    memory.close().await.unwrap();
}

#[tokio::test]
async fn short_caps_serve_successive_rounds_in_scene_order() {
    let memory = floor_memory(true, false).await;
    // The floor-two case alone observes a second round after all four kinds
    // have received their first slot.
    for (floor, cap) in (0..=4).map(|cap| (1, cap)).chain([(2, 5)]) {
        let mut context = mixed_context();
        context.cue_floors = RetrievalCueFloors {
            participant: floor,
            place: floor,
            activity: floor,
            topic: floor,
        };
        context.candidate_limits.max_graph_roots = cap;
        let first = memory.retrieve(context).await.unwrap();
        let roots = first
            .trace
            .unwrap()
            .graph_expansions
            .into_iter()
            .filter(|row| row.outcome == GraphExpansionOutcome::Expanded)
            .map(|row| row.root.id.as_u128())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            roots,
            [2000, 3000, 5000, 1000, 2001][..cap]
                .iter()
                .copied()
                .collect()
        );

        let mut context = mixed_context();
        context.cue_floors = RetrievalCueFloors {
            participant: floor,
            place: floor,
            activity: floor,
            topic: floor,
        };
        context.section_limits.relevant_episodes = cap;
        let first = memory.retrieve(context).await.unwrap();
        let observations = first
            .pack
            .relevant_episodes
            .into_iter()
            .map(|object| object.id.as_u128())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            observations,
            [2000, 3000, 4000, 1000, 2001][..cap]
                .iter()
                .copied()
                .collect()
        );
    }
    let first = memory.retrieve(mixed_context()).await.unwrap();
    let second = memory.retrieve(mixed_context()).await.unwrap();
    assert_eq!(first.trace, second.trace);
    let mut context = mixed_context();
    context.candidate_limits.max_vector_candidates = 0;
    let result = memory.retrieve(context).await.unwrap();
    assert!(result.trace.unwrap().vector_candidates.is_empty());
    assert_eq!(
        result
            .pack
            .relevant_episodes
            .iter()
            .map(|object| object.id.as_u128())
            .collect::<Vec<_>>(),
        [4000]
    );
    memory.close().await.unwrap();
}

#[tokio::test]
async fn overlapping_kinds_share_one_slot_and_return_unused_room() {
    let memory = floor_memory(true, true).await;
    link_activity(&memory, 1000, ObjectType::Episode).await;
    let mut context = mixed_context();
    context.scene.participants[0].description = Some("topic".to_owned());
    context.scene.setting.words = Some("topic".to_owned());
    context.candidate_limits.max_vector_candidates = 2;
    context.candidate_limits.max_graph_roots = 3;
    context.section_limits.relevant_episodes = 2;
    let result = memory.retrieve(context).await.unwrap();
    assert_eq!(
        result
            .pack
            .relevant_episodes
            .iter()
            .map(|object| object.id.as_u128())
            .collect::<Vec<_>>(),
        [1000, 1001]
    );
    let trace = result.trace.unwrap();
    assert!(trace.floor_admissions.is_empty());
    let shared = trace
        .section_assignments
        .iter()
        .find(|row| row.object.id == MemoryId::from_u128(1000))
        .unwrap();
    assert_eq!(
        shared.cue_kinds,
        BTreeSet::from([
            CueKind::Topic,
            CueKind::Participant,
            CueKind::Place,
            CueKind::Activity
        ])
    );
    memory.close().await.unwrap();
}

#[tokio::test]
async fn each_zero_floor_removes_only_its_reservation() {
    let memory = floor_memory(true, false).await;
    for (kind, witness) in [
        (CueKind::Participant, 2000),
        (CueKind::Place, 3000),
        (CueKind::Activity, 4000),
        (CueKind::Topic, 1000),
    ] {
        let mut context = mixed_context();
        // Isolate root reservations from the independent candidate score cap.
        context.candidate_limits.max_vector_candidates = 96;
        let floors = &mut context.cue_floors;
        match kind {
            CueKind::Participant => floors.participant = 0,
            CueKind::Place => floors.place = 0,
            CueKind::Activity => floors.activity = 0,
            CueKind::Topic => floors.topic = 0,
        }
        // The other three reservations consume all room, so this kind waits.
        context.candidate_limits.max_graph_roots = 3;
        let result = memory.retrieve(context.clone()).await.unwrap();
        assert!(
            !result
                .pack
                .relevant_episodes
                .iter()
                .any(|object| object.id == MemoryId::from_u128(witness)),
            "{kind:?}"
        );
        assert!(result
            .trace
            .unwrap()
            .floor_admissions
            .iter()
            .filter(|row| row.stage == CueFloorStage::GraphRoots)
            .all(|row| row.cue_kind != kind));
        // A zero reservation still participates when spare turns are available.
        context.candidate_limits.max_graph_roots = 8;
        let spare = memory.retrieve(context).await.unwrap();
        assert!(
            spare
                .pack
                .relevant_episodes
                .iter()
                .any(|object| object.id == MemoryId::from_u128(witness)),
            "{kind:?}"
        );
    }
    memory.close().await.unwrap();
}
