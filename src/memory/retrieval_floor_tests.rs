use std::collections::BTreeSet;

use crate::api::types::*;
use crate::domain::*;
use crate::models::vector::EmbeddingInput;
use crate::test_support::TestEmbedder;
use crate::CharacterMemory;

#[tokio::test]
async fn open_loop_activity_reserves_the_latest_recorded_source() {
    let memory = crate::test_support::memory_with_embedder(4, TestEmbedder(floor_embedding)).await;
    let provenance = || CandidateProvenance::caller("open-loop source order");
    let mut plan = RememberWritePlan::new();
    // Neither IDs nor creation times give the scene order: 2, 3, 1.
    for (id, days) in [(1, 30), (2, 0), (3, 1)] {
        let mut draft = EpisodeDraft::new(format!("Source {id}"));
        draft.id = Some(MemoryId::from_u128(id));
        draft.created_at = Some(occasion().time.to_utc() + chrono::Duration::days(id as i64));
        draft.scene = Some(Scene::at(
            (occasion().time - chrono::Duration::days(days)).fixed_offset(),
        ));
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        draft.salience_score = if id == 1 { 1.0 } else { 0.0 };
        plan = plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
            draft,
            provenance(),
        )));
    }
    let mut draft = DerivedMemoryDraft::new(DerivedType::OpenLoop, "Finish the conversation");
    draft.id = Some(MemoryId::from_u128(4));
    draft.created_at = Some(occasion().time.to_utc());
    draft.updated_at = draft.created_at;
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    draft.derived_from_episode_ids = [1, 2, 3].map(MemoryId::from_u128).to_vec();
    plan = plan.with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
        draft,
        provenance(),
    )));
    memory.commit(plan, CommitOptions::default()).await.unwrap();
    for roots in [2, 4] {
        let mut context = RetrievalContext::default()
            .with_scene(occasion())
            .with_trace()
            .with_activity(ActivityRef::OpenLoop(MemoryId::from_u128(4)));
        context.candidate_limits.max_graph_roots = roots;
        context.section_limits.relevant_episodes = 1;
        context.graph_limits.max_depth = 0;
        let outcome = memory.retrieve(context).await.unwrap();
        let episodes = outcome
            .pack
            .relevant_episodes
            .iter()
            .map(|episode| episode.id.as_u128())
            .collect::<Vec<_>>();
        assert_eq!(episodes, [2], "root cap {roots}");
    }
    let mut forget = ForgetMemoryDraft::suppress(
        LifecycleTargetRef::episode(MemoryId::from_u128(2)),
        "Forget the latest source only",
    );
    forget.cascade_policy.apply_to_derived_from_target = false;
    memory.forget(forget).await.unwrap();
    let mut observed = Vec::new();
    for include_suppressed in [false, true] {
        let mut context = RetrievalContext::default()
            .with_scene(occasion())
            .with_trace()
            .with_activity(ActivityRef::OpenLoop(MemoryId::from_u128(4)));
        context.candidate_limits.max_graph_roots = 2;
        context.section_limits.relevant_episodes = 1;
        context.graph_limits.max_depth = 0;
        context.lifecycle_policy.include_suppressed = include_suppressed;
        let outcome = memory.retrieve(context).await.unwrap();
        let episodes = outcome
            .pack
            .relevant_episodes
            .iter()
            .map(|episode| episode.id.as_u128())
            .collect::<Vec<_>>();
        assert_eq!(
            outcome.activity.unwrap().resolution,
            ActivityResolution::Found
        );
        if !include_suppressed {
            assert!(outcome
                .trace
                .unwrap()
                .lifecycle_filter_decisions
                .iter()
                .any(|row| {
                    row.object.id == MemoryId::from_u128(2)
                        && row.reason == LifecycleFilterReason::SuppressedOmitted
                }));
        }
        observed.push(episodes);
    }
    assert_eq!(observed, [vec![3], vec![2]]);
    let mut plan = RememberWritePlan::new();
    for (id, parent) in [(9, 2), (10, 3), (11, 1)] {
        let mut draft = ObservationDraft::new(MemoryId::from_u128(parent), "Source observation");
        draft.id = Some(MemoryId::from_u128(id));
        draft.created_at = Some(occasion().time.to_utc());
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        plan = plan.with_candidate(MemoryCandidate::Observation(ObservationCandidate::new(
            draft,
            provenance(),
        )));
    }
    for (id, source) in [(20, 9), (21, 10)] {
        let mut draft = DerivedMemoryDraft::new(DerivedType::OpenLoop, "Use the observation");
        draft.id = Some(MemoryId::from_u128(id));
        draft.created_at = Some(occasion().time.to_utc());
        draft.updated_at = draft.created_at;
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        draft.derived_from_observation_ids = [source, 11].map(MemoryId::from_u128).to_vec();
        plan = plan.with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
            draft,
            provenance(),
        )));
    }
    memory.commit(plan, CommitOptions::default()).await.unwrap();
    let mut forget = ForgetMemoryDraft::suppress(
        LifecycleTargetRef::observation(MemoryId::from_u128(10)),
        "Forget observation only",
    );
    forget.cascade_policy.apply_to_derived_from_target = false;
    memory.forget(forget).await.unwrap();
    let mut observed = Vec::new();
    for activity in [20, 21] {
        for include_suppressed in [false, true] {
            let mut context = RetrievalContext::default()
                .with_scene(occasion())
                .with_trace()
                .with_activity(ActivityRef::OpenLoop(MemoryId::from_u128(activity)));
            context.candidate_limits.max_graph_roots = 2;
            context.section_limits.salient_observations = 1;
            context.graph_limits.max_depth = 0;
            context.lifecycle_policy.include_suppressed = include_suppressed;
            let outcome = memory.retrieve(context).await.unwrap();
            let observations = outcome
                .pack
                .salient_observations
                .iter()
                .map(|observation| observation.id.as_u128())
                .collect::<Vec<_>>();
            if !include_suppressed {
                let excluded = if activity == 20 {
                    MemoryObjectRef::new(ObjectType::Episode, MemoryId::from_u128(2))
                } else {
                    MemoryObjectRef::new(ObjectType::Observation, MemoryId::from_u128(10))
                };
                assert!(outcome
                    .trace
                    .unwrap()
                    .lifecycle_filter_decisions
                    .iter()
                    .any(|row| {
                        row.object == excluded
                            && row.reason == LifecycleFilterReason::SuppressedOmitted
                    }));
            }
            observed.push(observations);
        }
    }
    assert_eq!(observed, [vec![11], vec![9], vec![11], vec![10]]);
    memory.close().await.unwrap();
}

fn floor_embedding(input: &EmbeddingInput) -> Vec<f32> {
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
    vector
}

fn occasion() -> Scene {
    Scene::at("2026-09-21T00:00:00Z".parse().unwrap())
}

async fn floor_memory(scene_surfaces: bool, overlap: bool) -> CharacterMemory {
    let memory = crate::test_support::memory_with_embedder(4, TestEmbedder(floor_embedding)).await;
    let provenance = || CandidateProvenance::caller("floor pressure");
    let mut episode = EpisodeDraft::new("An occasion with several recollections.");
    episode.id = Some(MemoryId::from_u128(1));
    episode.scene = Some(Scene::at(
        (occasion().time - chrono::Duration::days(60)).fixed_offset(),
    ));
    episode.created_at = Some(occasion().time.to_utc());
    episode.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut thread = MemoryThreadDraft::new("Work in progress", "The current activity.");
    thread.id = Some(MemoryId::from_u128(5000));
    thread.created_at = Some(occasion().time.to_utc());
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
            episode.created_at = Some(scene.time.to_utc());
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
            observation.observed_at = Some(occasion().time.to_utc());
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
    let memory = crate::test_support::memory_with_embedder(4, TestEmbedder(floor_embedding)).await;
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
        member.created_at = Some(occasion().time.to_utc());
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
    let memory = crate::test_support::memory_with_embedder(4, TestEmbedder(floor_embedding)).await;
    for id in (6000..6012).chain([7000]) {
        let text = if id == 7000 {
            "orchids need careful watering."
        } else {
            "The work included a passing mention of orchids."
        };
        let mut scene = occasion();
        scene.time -= chrono::Duration::days(if id == 7000 { 1000 } else { (6011 - id) as i64 });
        scene.setting.words = Some(text.to_owned());
        let mut episode = EpisodeDraft::new(text);
        episode.id = Some(MemoryId::from_u128(id));
        episode.created_at = Some(scene.time.to_utc());
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
        member.created_at = Some(occasion().time.to_utc());
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
        member.created_at = Some(occasion().time.to_utc());
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
            date_match: 1,
            participant: 0,
            place: 0,
            activity: 5,
            topic: 1,
            recency: 0,
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
        let mut expected = BTreeSet::from([kind]);
        if id != 4000 {
            expected.insert(CueKind::Recency);
        }
        assert_eq!(row.cue_kinds, expected);
    }
    assert!(trace.graph_expansions.iter().any(|root| {
        root.root == MemoryObjectRef::new(ObjectType::Episode, MemoryId::from_u128(4000))
            && root.source == GraphRootSource::Recency
            && root.outcome == GraphExpansionOutcome::RootLimit
    }));
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
    let memory = crate::test_support::memory_with_embedder(4, TestEmbedder(floor_embedding)).await;
    let provenance = || CandidateProvenance::caller("shared occasion");
    let mut person = EntityDraft::new();
    person.id = Some(MemoryId::from_u128(7));
    person.created_at = Some(occasion().time.to_utc());
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
        episode.created_at = Some(scene.time.to_utc());
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
        observation.observed_at = Some(occasion().time.to_utc());
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
            BTreeSet::from([CueKind::Topic, CueKind::Participant, CueKind::Recency])
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
    // The smaller-ID parent is older and cannot take a recent occasion's slot.
    assert_eq!(selected, [3000, 2000, 1000, 4000, 1001, 2001]);
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
                date_match: 1,
                participant: 0,
                place: 0,
                activity: 0,
                topic: 0,
                recency: 0,
            };
            let prefix = memory.retrieve(prefix_context).await.unwrap();
            assert_eq!(
                serde_json::to_vec(&result).unwrap(),
                serde_json::to_vec(&prefix).unwrap(),
                "single-kind retrieval must preserve the original ranked prefix"
            );
            // The original given-cue prefix remains; recency uses spare root and section room.
            let first = match kind {
                CueKind::Topic => 1000,
                CueKind::Participant => 2000,
                CueKind::Place => 3000,
                CueKind::Activity => 4000,
                CueKind::Recency | CueKind::DateMatch => {
                    unreachable!("fixture uses only given cues")
                }
            };
            let expected = if kind == CueKind::Topic {
                (first..first + section as u128).collect::<Vec<_>>()
            } else {
                [first]
                    .into_iter()
                    .chain(
                        [1000, 2000, 3000, 4000, 1001, 2001, 3001, 1002]
                            .into_iter()
                            .filter(|&id| id != first),
                    )
                    .take(section)
                    .collect()
            };
            let pack = result.pack;
            assert_eq!(
                pack.relevant_episodes
                    .iter()
                    .map(|object| object.id.as_u128())
                    .collect::<Vec<_>>(),
                expected,
                "{kind:?} {candidates}/{roots}/{section}"
            );
            assert_eq!(
                pack.active_threads
                    .iter()
                    .map(|object| object.id.as_u128())
                    .collect::<Vec<_>>(),
                if kind == CueKind::Activity || (kind != CueKind::Topic && section > 1) {
                    vec![5000]
                } else {
                    vec![]
                },
                "{kind:?} {candidates}/{roots}/{section}"
            );
            if !matches!(kind, CueKind::Topic | CueKind::Activity) && section > 1 {
                let thread = result
                    .trace
                    .as_ref()
                    .unwrap()
                    .section_assignments
                    .iter()
                    .find(|row| {
                        row.object
                            == MemoryObjectRef::new(
                                ObjectType::MemoryThread,
                                MemoryId::from_u128(5000),
                            )
                    })
                    .unwrap();
                assert_eq!(thread.cue_kinds, BTreeSet::from([CueKind::Recency]));
            }
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
            date_match: 1,
            participant: floor,
            place: floor,
            activity: floor,
            topic: floor,
            recency: 0,
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
            date_match: 1,
            participant: floor,
            place: floor,
            activity: floor,
            topic: floor,
            recency: 0,
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
        [4000, 1000, 2000, 3000, 1001, 2001, 3001, 1002]
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
            CueKind::Activity,
            CueKind::Recency
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
            CueKind::Recency | CueKind::DateMatch => unreachable!("fixture uses only given cues"),
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
        // Expanding roads still share spare turns; descriptions do not.
        context.candidate_limits.max_graph_roots = 8;
        let spare = memory.retrieve(context).await.unwrap();
        assert_eq!(
            spare
                .pack
                .relevant_episodes
                .iter()
                .any(|object| object.id == MemoryId::from_u128(witness)),
            matches!(kind, CueKind::Activity | CueKind::Topic),
            "{kind:?}"
        );
    }
    memory.close().await.unwrap();
}

mod scene_cohorts {
    use std::collections::BTreeSet;

    use crate::api::types::*;
    use crate::domain::*;
    use crate::models::vector::EmbeddingInput;
    use crate::test_support::TestEmbedder;
    use crate::CharacterMemory;

    fn cohort_id(day: u128, ascending_ids: bool) -> u128 {
        if ascending_ids {
            1000 + day
        } else {
            1047 - day
        }
    }

    fn cohort_embedding(input: &EmbeddingInput, overlap_id: u128) -> Vec<f32> {
        let (topic, scene, unknown): (f32, f32, f32) = if input.surface == VectorSurface::Query {
            match input.text.as_str() {
                "orchids" => (1.0, 0.0, 0.0),
                "unlived" => (0.0, 0.0, 1.0),
                "studio" => (0.0, 1.0, 0.0),
                text if text.starts_with("botanist") => (0.0, 1.0, 0.0),
                text => panic!("unexpected query {text}"),
            }
        } else if input.object_id == Some(MemoryId::from_u128(overlap_id)) {
            (0.8, 0.6, 0.0)
        } else if matches!(
            input.surface,
            VectorSurface::SceneSetting | VectorSurface::SceneParticipants
        ) || input.text == "Episode summary: Another ordinary day."
        {
            (0.1, 0.99, 0.0)
        } else if input.text.contains("Orchid shared") {
            (0.8, 0.6, 0.0)
        } else {
            let rank: f32 = input
                .text
                .strip_prefix("Episode summary: Orchid lesson ")
                .unwrap()
                .parse()
                .unwrap();
            (0.9 - rank * 0.3 / 7.0, 0.0, 0.0)
        };
        vec![
            topic,
            scene,
            (1.0 - topic * topic - scene * scene - unknown * unknown)
                .max(0.0)
                .sqrt(),
            unknown,
        ]
    }

    fn context() -> RetrievalContext {
        let mut context = RetrievalContext::new("orchids").with_trace();
        context.scene = Scene::at("2026-09-21T00:00:00Z".parse().unwrap());
        context.scene.setting.words = Some("studio".to_owned());
        context.scene.participants.push(SceneParticipant {
            description: Some("botanist".to_owned()),
            ..Default::default()
        });
        context.object_type_defaults = vec![ObjectType::Episode];
        context.graph_limits.max_depth = 0;
        context
    }

    fn episode(id: u128, summary: String, scene: Scene) -> [MemoryCandidate; 2] {
        let mut draft = EpisodeDraft::new(summary);
        draft.id = Some(MemoryId::from_u128(id));
        draft.created_at = Some(scene.time.to_utc());
        draft.scene = Some(scene);
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        [
            MemoryCandidate::Episode(EpisodeCandidate::new(
                draft,
                CandidateProvenance::caller("turn pressure"),
            )),
            MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                MemoryObjectRef::new(ObjectType::Episode, MemoryId::from_u128(id)),
                CandidateProvenance::caller("turn pressure"),
            )),
        ]
    }

    async fn cohort_memory(ascending_ids: bool) -> CharacterMemory {
        let memory = crate::test_support::memory_with_embedder(
            4,
            TestEmbedder(move |input: &EmbeddingInput| {
                cohort_embedding(input, cohort_id(49, ascending_ids))
            }),
        )
        .await;
        let mut person = EntityDraft::new();
        person.id = Some(MemoryId::from_u128(7));
        person.created_at = Some(context().scene.time.to_utc());
        person.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        memory
            .commit(
                RememberWritePlan::new().with_candidate(MemoryCandidate::Entity(
                    EntityCandidate::new(person, CandidateProvenance::caller("participant")),
                )),
                CommitOptions::default(),
            )
            .await
            .unwrap();
        let mut plan = RememberWritePlan::new();
        for index in 0..48 {
            let mut scene = context().scene;
            scene.time += chrono::Duration::days(index as i64);
            scene.participants[0].key = Some(MemoryId::from_u128(7));
            for candidate in episode(
                cohort_id(index, ascending_ids),
                "Another ordinary day.".to_owned(),
                scene,
            ) {
                plan = plan.with_candidate(candidate);
            }
        }
        for index in 0..8 {
            for candidate in episode(
                2000 + index,
                format!("Orchid lesson {index}"),
                Scene::at((context().scene.time).fixed_offset()),
            ) {
                plan = plan.with_candidate(candidate);
            }
        }
        memory.commit(plan, CommitOptions::default()).await.unwrap();
        memory
    }

    fn topic_count(ids: impl Iterator<Item = MemoryId>) -> usize {
        ids.filter(|id| (2000..2008).contains(&id.as_u128()))
            .count()
    }

    #[tokio::test]
    async fn shared_scene_cohort_keeps_the_latest_occasion_and_score_fills_the_pack() {
        for ascending_ids in [false, true] {
            let memory = cohort_memory(ascending_ids).await;
            for descriptions in [1, 5] {
                let mut query = context();
                query.scene.time += chrono::Duration::days(48);
                query.scene.participants = (0..descriptions)
                    .map(|index| SceneParticipant {
                        description: Some(format!("botanist {index}")),
                        ..Default::default()
                    })
                    .collect();
                let result = memory.retrieve(query).await.unwrap();
                let trace = result.trace.as_ref().unwrap();
                let counts = (
                    topic_count(trace.vector_candidates.iter().map(|row| row.object.id)),
                    topic_count(
                        trace
                            .graph_expansions
                            .iter()
                            .filter(|row| row.outcome == GraphExpansionOutcome::Expanded)
                            .map(|row| row.root.id),
                    ),
                    topic_count(
                        result
                            .pack
                            .relevant_episodes
                            .iter()
                            .map(|episode| episode.id),
                    ),
                );
                assert_eq!(counts, (8, 8, 7), "descriptions={descriptions}");
                // Admission changes, but output still follows the original final scores.
                assert_eq!(
                    result
                        .pack
                        .relevant_episodes
                        .iter()
                        .map(|episode| episode.id.as_u128())
                        .collect::<Vec<_>>(),
                    [
                        cohort_id(47, ascending_ids),
                        2000,
                        2001,
                        2002,
                        2003,
                        2004,
                        2005,
                        2006
                    ]
                );
                assert!(trace.floor_admissions.is_empty());
                assert_eq!(
                    trace
                        .scene_cue_searches
                        .iter()
                        .map(|search| (search.cue_kind, search.omitted_count))
                        .collect::<Vec<_>>(),
                    [(CueKind::Place, 47), (CueKind::Participant, 47)]
                );
            }
            memory.close().await.unwrap();
        }
    }

    #[tokio::test]
    async fn shared_scene_keeps_latest_keyed_occasion_with_lived_and_unlived_topics() {
        for ascending_ids in [false, true] {
            let mut memory = cohort_memory(ascending_ids).await;
            for topic in ["orchids", "unlived"] {
                let mut query = context();
                query.scene.time += chrono::Duration::days(48);
                query.topic = Some(topic.to_owned());
                query.scene.participants[0].key = Some(MemoryId::from_u128(7));
                query.graph_limits.max_depth = 1;
                let result = memory.retrieve(query).await.unwrap();
                let trace = result.trace.unwrap();
                assert!(trace
                    .graph_expansions
                    .iter()
                    .any(|row| row.root.id == MemoryId::from_u128(7)
                        && row.outcome == GraphExpansionOutcome::Expanded));
                assert_eq!(result.pack.relevant_episodes.len(), 8);
                if topic == "unlived" {
                    assert!(result
                        .pack
                        .relevant_episodes
                        .iter()
                        .all(|episode| (1000..1048).contains(&episode.id.as_u128())));
                }
                assert!(
                    result
                        .pack
                        .relevant_episodes
                        .iter()
                        .any(|episode| episode.id
                            == MemoryId::from_u128(cohort_id(47, ascending_ids))),
                    "latest participant occasion must reach the pack: {:?}",
                    trace
                        .section_assignments
                        .iter()
                        .find(|row| row.object.id
                            == MemoryId::from_u128(cohort_id(47, ascending_ids)))
                );
            }
            // More than one explicit occasion must retain selector order, not ID order.
            memory.memory_composition.selectivity_policy =
                crate::policy::RetrievalSelectivityPolicy::with_fanout_budgets(
                    1.0,
                    1.0,
                    crate::config::Settings::new(Default::default())
                        .unwrap()
                        .get_retrieval_fanout_budgets()
                        .map(|(relation, object_type, budget)| {
                            if relation == RelationType::Involves {
                                (relation, object_type, 2, 2)
                            } else {
                                (relation, object_type, budget.min(), budget.max())
                            }
                        }),
                );
            let mut query = context();
            query.scene.time += chrono::Duration::days(48);
            query.scene.participants[0].key = Some(MemoryId::from_u128(7));
            query.graph_limits.max_depth = 1;
            query.section_limits.relevant_episodes = 1;
            let result = memory.retrieve(query).await.unwrap();
            assert_eq!(
                result.pack.relevant_episodes[0].id,
                MemoryId::from_u128(cohort_id(47, ascending_ids))
            );
            assert!(result
                .trace
                .unwrap()
                .section_assignments
                .iter()
                .any(|row| row.object.id == MemoryId::from_u128(cohort_id(46, ascending_ids))));
            memory.close().await.unwrap();
        }
    }

    #[tokio::test]
    async fn shared_scene_topic_only_keeps_original_bytes() {
        for ascending_ids in [false, true] {
            let memory = cohort_memory(ascending_ids).await;
            let mut query = context();
            query.scene.participants.clear();
            query.scene.setting.words = None;
            let result = memory.retrieve(query.clone()).await.unwrap();
            query.cue_floors = RetrievalCueFloors {
                date_match: 1,
                participant: 0,
                place: 0,
                activity: 0,
                topic: 0,
                recency: 0,
            };
            let zero_floors = memory.retrieve(query).await.unwrap();
            assert_eq!(
                serde_json::to_vec(&result).unwrap(),
                serde_json::to_vec(&zero_floors).unwrap()
            );
            assert_eq!(
                topic_count(
                    result
                        .pack
                        .relevant_episodes
                        .iter()
                        .map(|episode| episode.id)
                ),
                8
            );
            assert!(result.trace.unwrap().floor_admissions.is_empty());
            memory.close().await.unwrap();
        }
    }

    #[tokio::test]
    async fn shared_scene_overlap_uses_one_slot_and_uncapped_turns_emit_no_admissions() {
        for ascending_ids in [false, true] {
            let memory = cohort_memory(ascending_ids).await;
            let mut plan = RememberWritePlan::new();
            let mut latest = context().scene;
            latest.time += chrono::Duration::days(49);
            for candidate in episode(
                cohort_id(49, ascending_ids),
                "Orchid shared".to_owned(),
                latest.clone(),
            ) {
                plan = plan.with_candidate(candidate);
            }
            memory.commit(plan, CommitOptions::default()).await.unwrap();
            let mut query = context();
            query.scene.time = latest.time;
            query.candidate_limits.max_vector_candidates = 64;
            let capped = memory.retrieve(query.clone()).await.unwrap();
            let trace = capped.trace.unwrap();
            assert_eq!(
            trace
                .vector_candidates
                .iter()
                .filter(|row| row.object.id == MemoryId::from_u128(cohort_id(49, ascending_ids)))
                .count(),
            1
        );
            assert_eq!(
                trace
                    .graph_expansions
                    .iter()
                    .filter(
                        |row| row.root.id == MemoryId::from_u128(cohort_id(49, ascending_ids))
                            && row.outcome == GraphExpansionOutcome::Expanded
                    )
                    .count(),
                1
            );
            assert_eq!(
                capped
                    .pack
                    .relevant_episodes
                    .iter()
                    .filter(
                        |episode| episode.id == MemoryId::from_u128(cohort_id(49, ascending_ids))
                    )
                    .count(),
                1
            );
            let shared = trace
                .section_assignments
                .iter()
                .find(|row| row.object.id == MemoryId::from_u128(cohort_id(49, ascending_ids)))
                .unwrap();
            assert_eq!(
                shared.cue_kinds,
                BTreeSet::from([
                    CueKind::Participant,
                    CueKind::Place,
                    CueKind::Topic,
                    CueKind::Recency
                ])
            );
            query.candidate_limits.max_graph_roots = 64;
            query.section_limits.relevant_episodes = 64;
            let uncapped = memory.retrieve(query).await.unwrap();
            assert_eq!(uncapped.pack.relevant_episodes.len(), 57);
            assert!(uncapped.trace.unwrap().floor_admissions.is_empty());
            memory.close().await.unwrap();
        }
    }
}
