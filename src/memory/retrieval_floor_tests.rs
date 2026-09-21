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
                    "topic" => 0,
                    "person" => 1,
                    "place" => 2,
                    text => panic!("unexpected query {text}"),
                },
                1.0_f32,
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

async fn floor_memory() -> CharacterMemory {
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
        if id != 4000 {
            plan = plan.with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                MemoryObjectRef::new(ObjectType::Observation, MemoryId::from_u128(id)),
                provenance(),
            )));
        }
    }
    memory.commit(plan, CommitOptions::default()).await.unwrap();
    link_activity(&memory, 4000).await;
    memory
}

async fn link_activity(memory: &CharacterMemory, observation: u128) {
    memory
        .link(MemoryLinkDraft::new(
            ObjectType::MemoryThread,
            MemoryId::from_u128(5000),
            RelationType::AssociatedWith,
            ObjectType::Observation,
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

#[tokio::test]
async fn a_large_activity_shares_roots_with_the_topic() {
    let memory = floor_memory().await;
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
    let memory = floor_memory().await;
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
    let memory = floor_memory().await;
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
        .salient_observations
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
        section: ContextPackSection::SalientObservations,
    };
    let expected = [
        (2000, CueFloorStage::CandidateMerge, CueKind::Participant),
        (2000, CueFloorStage::GraphRoots, CueKind::Participant),
        (3000, CueFloorStage::GraphRoots, CueKind::Place),
        (2000, section, CueKind::Participant),
        (3000, section, CueKind::Place),
        (4000, section, CueKind::Activity),
    ];
    assert_eq!(trace.floor_admissions.len(), expected.len());
    for (id, stage, cue_kind) in expected {
        assert!(trace.floor_admissions.contains(&CueFloorAdmission {
            object: MemoryObjectRef::new(ObjectType::Observation, MemoryId::from_u128(id)),
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
    no_section.section_limits.salient_observations = 0;
    let no_section = memory.retrieve(no_section).await.unwrap();
    assert!(no_section.pack.salient_observations.is_empty());
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
    let memory = floor_memory().await;
    let mut context = mixed_context();
    context.topic = None;
    context.activity = None;
    context.candidate_limits.max_vector_candidates = 96;
    context.candidate_limits.max_graph_roots = 96;
    context.section_limits.salient_observations = 2;
    let result = memory.retrieve(context).await.unwrap();
    let selected = result
        .pack
        .salient_observations
        .iter()
        .map(|object| object.id.as_u128())
        .collect::<Vec<_>>();
    assert_eq!(selected, [3000, 2000]);
    assert_eq!(
        result.trace.unwrap().floor_admissions,
        [CueFloorAdmission {
            object: MemoryObjectRef::new(ObjectType::Observation, MemoryId::from_u128(2000)),
            stage: CueFloorStage::Section {
                section: ContextPackSection::SalientObservations
            },
            cue_kind: CueKind::Participant,
        }]
    );
    memory.close().await.unwrap();
}

#[tokio::test]
async fn single_kind_keeps_section_ids_and_order() {
    let memory = floor_memory().await;
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
            context.section_limits.salient_observations = section;
            let result = memory.retrieve(context.clone()).await.unwrap();
            // Exact section/id/order projections captured at 61fbb29.
            let first = match kind {
                CueKind::Topic => 1000,
                CueKind::Participant => 2000,
                CueKind::Place => 3000,
                CueKind::Activity => 4000,
            };
            let count = if kind == CueKind::Activity {
                1
            } else {
                section
            };
            let pack = result.pack;
            assert_eq!(
                pack.salient_observations
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
            assert!(pack.relevant_episodes.is_empty());
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
    let memory = floor_memory().await;
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
        context.section_limits.salient_observations = cap;
        let first = memory.retrieve(context).await.unwrap();
        let observations = first
            .pack
            .salient_observations
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
            .salient_observations
            .iter()
            .map(|object| object.id.as_u128())
            .collect::<Vec<_>>(),
        [4000]
    );
    memory.close().await.unwrap();
}

#[tokio::test]
async fn overlapping_kinds_share_one_slot_and_return_unused_room() {
    let memory = floor_memory().await;
    link_activity(&memory, 1000).await;
    let mut context = mixed_context();
    context.scene.participants[0].description = Some("topic".to_owned());
    context.scene.setting.words = Some("topic".to_owned());
    context.candidate_limits.max_vector_candidates = 2;
    context.candidate_limits.max_graph_roots = 3;
    context.section_limits.salient_observations = 2;
    let result = memory.retrieve(context).await.unwrap();
    assert_eq!(
        result
            .pack
            .salient_observations
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
    let memory = floor_memory().await;
    for (kind, witness) in [
        (CueKind::Participant, 2000),
        (CueKind::Place, 3000),
        (CueKind::Activity, 4000),
    ] {
        let mut context = mixed_context();
        let floors = &mut context.cue_floors;
        match kind {
            CueKind::Participant => floors.participant = 0,
            CueKind::Place => floors.place = 0,
            CueKind::Activity => floors.activity = 0,
            CueKind::Topic => unreachable!(),
        }
        let result = memory.retrieve(context).await.unwrap();
        assert!(
            !result
                .pack
                .salient_observations
                .iter()
                .any(|object| object.id == MemoryId::from_u128(witness)),
            "{kind:?}"
        );
        assert!(result
            .trace
            .unwrap()
            .floor_admissions
            .iter()
            .all(|row| row.cue_kind != kind));
    }
    // A long activity supplies enough explicit roots to crowd out the topic.
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
    let protected = memory.retrieve(mixed_context()).await.unwrap();
    assert!(protected
        .trace
        .unwrap()
        .floor_admissions
        .iter()
        .any(|row| row.cue_kind == CueKind::Topic && row.stage == CueFloorStage::GraphRoots));
    let mut context = mixed_context();
    context.cue_floors.topic = 0;
    let unprotected = memory.retrieve(context).await.unwrap();
    let trace = unprotected.trace.unwrap();
    assert!(trace
        .floor_admissions
        .iter()
        .all(|row| row.cue_kind != CueKind::Topic));
    assert!(trace
        .graph_expansions
        .iter()
        .filter(|row| row.outcome == GraphExpansionOutcome::Expanded)
        .all(|row| !(1000..1048).contains(&row.root.id.as_u128())));
    memory.close().await.unwrap();
}
