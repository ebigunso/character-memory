use character_memory::{
    CorrectMemoryDraft, CorrectionTarget, DerivedMemoryDraft, DerivedType, EpisodeDraft,
    ForgetMemoryDraft, LifecycleTargetRef, MemoryId, ObservationDraft, RememberInput,
    RememberOptions, ReplacementDerivedMemoryDraft, RetrievalContext, SourceProvenanceReference,
};
use uuid::Uuid;

mod road_behavior {
    use super::test_support;
    use character_memory::api::types::{CueFloorStage, RetrievalCueFloors, TimeRange};
    use character_memory::*;
    use chrono::{DateTime, Duration, Utc};

    fn time() -> DateTime<Utc> {
        "2026-09-23T12:00:00Z".parse().unwrap()
    }
    fn id(n: u128, reverse: bool) -> MemoryId {
        MemoryId::from_u128(if reverse { 100_000 - n } else { n })
    }
    fn provenance() -> CandidateProvenance {
        CandidateProvenance::caller("recall behavior")
    }
    fn indexed(
        plan: RememberWritePlan,
        kind: ObjectType,
        n: u128,
        reverse: bool,
    ) -> RememberWritePlan {
        plan.with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
            MemoryObjectRef::new(kind, id(n, reverse)),
            provenance(),
        )))
    }
    fn episode(
        plan: RememberWritePlan,
        n: u128,
        days: i64,
        salience: f32,
        text: Option<&str>,
        reverse: bool,
    ) -> RememberWritePlan {
        let mut draft = EpisodeDraft::new(text.unwrap_or("ordinary occasion"));
        draft.id = Some(id(n, reverse));
        draft.scene = Some(Scene::at((time() - Duration::days(days)).fixed_offset()));
        draft.created_at = Some(time());
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
        draft.salience_score = salience;
        let plan = plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
            draft,
            provenance(),
        )));
        if text.is_some() {
            indexed(plan, ObjectType::Episode, n, reverse)
        } else {
            plan
        }
    }
    async fn commit(memory: &CharacterMemory, plan: RememberWritePlan) {
        let result = memory.commit(plan, CommitOptions::default()).await.unwrap();
        assert!(result.vector_indexing_failure.is_none());
        assert!(result.diagnostics.messages.is_empty(), "{result:?}");
    }
    fn query(topic: Option<&str>, roots: usize, room: usize) -> RetrievalContext {
        let mut context = RetrievalContext::default()
            .with_scene(Scene::at(time().fixed_offset()))
            .with_trace();
        context.topic = topic.map(str::to_owned);
        context.graph_limits.max_depth = 0;
        context.graph_limits.timeout_ms = None;
        context.candidate_limits.max_graph_roots = roots;
        context.candidate_limits.max_vector_candidates = 32;
        context.section_limits = ContinuitySectionLimits {
            active_threads: room,
            relevant_episodes: room,
            salient_observations: room,
            derived_memories: room,
            preferences: room,
            relationship_notes: room,
            open_loops: room,
            commitments: room,
            character_signals: room,
        };
        context.cue_floors = RetrievalCueFloors {
            participant: 0,
            place: 0,
            activity: 0,
            date_match: 0,
            topic: 0,
            recency: 0,
        };
        context
    }
    fn roots(result: &RetrieveOutcome) -> Vec<MemoryId> {
        result
            .trace
            .as_ref()
            .unwrap()
            .graph_expansions
            .iter()
            .filter(|row| {
                matches!(
                    row.outcome,
                    GraphExpansionOutcome::Expanded | GraphExpansionOutcome::Bounded
                )
            })
            .map(|row| row.root.id)
            .collect()
    }

    #[tokio::test]
    async fn talking_about_someone_is_independent_of_being_with_them() {
        let mut results = Vec::new();
        for reverse in [false, true] {
            let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
            let mut plan = RememberWritePlan::new();
            for n in [10, 11] {
                let mut person = EntityDraft::new();
                person.id = Some(id(n, reverse));
                person.created_at = Some(time());
                person.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
                plan = plan.with_candidate(MemoryCandidate::Entity(EntityCandidate::new(
                    person,
                    provenance(),
                )));
            }
            plan = episode(plan, 100, 1, 0.5, None, reverse);
            if let MemoryCandidate::Episode(candidate) = plan.candidates.last_mut().unwrap() {
                candidate
                    .draft
                    .scene
                    .as_mut()
                    .unwrap()
                    .participants
                    .push(SceneParticipant {
                        key: Some(id(10, reverse)),
                        ..Default::default()
                    });
            }
            for n in 101..112 {
                plan = episode(plan, n, if n == 101 { 365 } else { 2 }, 0.5, None, reverse);
                for offset in 0..if n == 101 { 3 } else { 1 } {
                    let mut observation =
                        ObservationDraft::new(id(n, reverse), "A remark about someone");
                    let oid = id(n * 10 + offset, reverse);
                    observation.id = Some(oid);
                    observation.created_at = Some(time());
                    observation.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
                    plan = plan.with_candidate(MemoryCandidate::Observation(
                        ObservationCandidate::new(observation, provenance()),
                    ));
                    let mut link = MemoryLinkDraft::new(
                        ObjectType::Observation,
                        oid,
                        RelationType::Mentions,
                        ObjectType::Entity,
                        id(if n == 101 { 10 } else { 11 }, reverse),
                    );
                    link.id = Some(id(5000 + n * 10 + offset, reverse));
                    link.created_at = Some(time());
                    link.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
                    plan = plan.with_candidate(MemoryCandidate::MemoryLink(
                        MemoryLinkCandidate::new(link, provenance()),
                    ));
                }
            }
            commit(&memory, plan).await;
            let mut context = query(None, 1, 16);
            context.scene.participants.push(SceneParticipant {
                key: Some(id(10, reverse)),
                ..Default::default()
            });
            context.cue_floors.participant = 1;
            context.graph_limits.max_depth = 1;
            let result = memory.retrieve(context.clone()).await.unwrap();
            context.candidate_limits.max_graph_roots = 2;
            context.cue_floors.date_match = 1;
            context.graph_limits.max_depth = 0;
            let anniversary = memory.retrieve(context).await.unwrap();
            test_support::close_and_remove_root(memory, root).await;
            results.push((reverse, result, anniversary));
        }
        for (reverse, result, anniversary) in results {
            let trace = result.trace.as_ref().unwrap();
            let presence = trace
                .selectivity_decisions
                .iter()
                .find(|row| row.relation == RelationType::Involves)
                .unwrap();
            let mentions = trace
                .selectivity_decisions
                .iter()
                .find(|row| row.relation == RelationType::Mentions)
                .unwrap();
            assert_eq!(presence.entity_count, Some(1));
            assert_eq!(
                (mentions.entity_count, mentions.global_count),
                (Some(3), Some(13))
            );
            assert_eq!(result.pack.salient_observations.len(), 3);
            assert!(
                !roots(&anniversary).contains(&id(101, reverse)),
                "a mention is not a shared anniversary"
            );
        }
    }

    #[tokio::test]
    async fn only_episode_involves_deduplicates_present_participants() {
        let mut outcomes = Vec::new();
        for reverse in [false, true] {
            for backward in [false, true] {
                for relation in [
                    RelationType::Mentions,
                    RelationType::About,
                    RelationType::Involves,
                ] {
                    let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
                    let mut person = EntityDraft::new();
                    person.id = Some(id(10, reverse));
                    let mut source = EpisodeDraft::new("Alex was here");
                    source.id = Some(id(100, reverse));
                    source.scene = Some(Scene {
                        participants: vec![SceneParticipant {
                            key: person.id,
                            ..Default::default()
                        }],
                        ..Scene::at(time().fixed_offset())
                    });
                    let mut observation =
                        ObservationDraft::new(id(100, reverse), "Alex discussed the ferry");
                    observation.id = Some(id(200, reverse));
                    let (kind, source_id) = if relation == RelationType::Involves {
                        (ObjectType::Episode, id(100, reverse))
                    } else {
                        (ObjectType::Observation, id(200, reverse))
                    };
                    let link = if backward {
                        MemoryLinkDraft::new(
                            ObjectType::Entity,
                            id(10, reverse),
                            relation,
                            kind,
                            source_id,
                        )
                    } else {
                        MemoryLinkDraft::new(
                            kind,
                            source_id,
                            relation,
                            ObjectType::Entity,
                            id(10, reverse),
                        )
                    };
                    let plan = RememberInput::new("presence and aboutness")
                        .with_entity(person)
                        .with_episode(source)
                        .with_observation(observation)
                        .with_memory_link(link)
                        .prepare_write_plan(&RememberPlanDefaults::fixed(
                            "presence and aboutness",
                            time(),
                        ));
                    let outcome = memory.commit(plan, CommitOptions::default()).await.unwrap();
                    test_support::close_and_remove_root(memory, root).await;
                    outcomes.push((
                        reverse,
                        backward,
                        relation,
                        outcome.persisted_link_ids.len(),
                    ));
                }
            }
        }
        for (reverse, backward, relation, count) in outcomes {
            // ObservedIn plus presence; caller aboutness is independently preserved.
            assert_eq!(
                count,
                if relation == RelationType::Involves {
                    2
                } else {
                    3
                },
                "reverse={reverse} backward={backward} relation={relation:?}"
            );
        }
    }

    #[tokio::test]
    async fn a_present_person_reaches_the_observation_through_its_episode() {
        let mut results = Vec::new();
        for reverse in [false, true] {
            let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
            let mut person = EntityDraft::new();
            person.id = Some(id(10, reverse));
            let mut scene = Scene::at((time() - Duration::days(1)).fixed_offset());
            scene.participants.push(SceneParticipant {
                key: person.id,
                ..Default::default()
            });
            let mut source = EpisodeDraft::new("Alex described the lighthouse repair");
            source.id = Some(id(100, reverse));
            source.scene = Some(scene);
            let mut observation =
                ObservationDraft::new(id(100, reverse), "Alex plans to repair the lighthouse");
            observation.id = Some(id(200, reverse));
            let plan = RememberInput::new("a conversation with Alex")
                .with_entity(person)
                .with_episode(source)
                .with_observation(observation)
                .prepare_write_plan(&RememberPlanDefaults::fixed("presence", time()));
            commit(&memory, plan).await;
            // Other occasions make this participant selective enough to traverse.
            let mut background = RememberWritePlan::new();
            for n in 300..310 {
                background = episode(background, n, 2, 0.5, None, reverse);
            }
            commit(&memory, background).await;
            let mut context = query(None, 1, 8);
            context.scene.participants.push(SceneParticipant {
                key: Some(id(10, reverse)),
                ..Default::default()
            });
            context.cue_floors.participant = 1;
            context.graph_limits.max_depth = 2;
            let result = memory.retrieve(context).await.unwrap();
            test_support::close_and_remove_root(memory, root).await;
            results.push((reverse, result));
        }
        for (reverse, result) in results {
            let links = &result.trace.as_ref().unwrap().graph_relations;
            assert!(!links
                .iter()
                .any(|link| link.relation == RelationType::Mentions));
            assert!(links.iter().any(|link| {
                link.relation == RelationType::Involves
                    && link.from == MemoryObjectRef::new(ObjectType::Episode, id(100, reverse))
                    && link.to == MemoryObjectRef::new(ObjectType::Entity, id(10, reverse))
            }));
            assert!(links.iter().any(|link| {
                link.relation == RelationType::ObservedIn
                    && link.from == MemoryObjectRef::new(ObjectType::Observation, id(200, reverse))
                    && link.to == MemoryObjectRef::new(ObjectType::Episode, id(100, reverse))
                    && link.proximity == 2
            }));
            assert!(result
                .pack
                .salient_observations
                .iter()
                .any(|o| o.id == id(200, reverse)));
        }
    }

    #[tokio::test]
    async fn a_reminder_keeps_its_observations_without_opening_other_occasions() {
        let mut results = Vec::new();
        for reverse in [false, true] {
            let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
            let mut person = EntityDraft::new();
            person.id = Some(id(10, reverse));
            let mut thread = MemoryThreadDraft::new("repairs", "ongoing repairs");
            thread.id = Some(id(900, reverse));
            let mut source = EpisodeDraft::new("lighthouse repair");
            source.id = Some(id(100, reverse));
            source.scene = Some(Scene::at(time().fixed_offset()));
            let mut observation = ObservationDraft::new(id(100, reverse), "Alex will repair it");
            observation.id = Some(id(200, reverse));
            commit(
                &memory,
                RememberInput::new("a conversation")
                    .with_entity(person)
                    .with_memory_thread(thread)
                    .with_thread_id(id(900, reverse))
                    .with_episode(source)
                    .with_observation(observation)
                    .with_memory_link(MemoryLinkDraft::new(
                        ObjectType::Observation,
                        id(200, reverse),
                        RelationType::Mentions,
                        ObjectType::Entity,
                        id(10, reverse),
                    ))
                    .prepare_write_plan(&RememberPlanDefaults::fixed("recent observation", time())),
            )
            .await;
            let mut older = EpisodeDraft::new("ferry crossing");
            older.id = Some(id(101, reverse));
            older.scene = Some(Scene::at((time() - Duration::days(1)).fixed_offset()));
            let mut older_observation = ObservationDraft::new(id(101, reverse), "a ferry crossing");
            older_observation.id = Some(id(201, reverse));
            commit(
                &memory,
                RememberInput::new("a separate occasion")
                    .with_episode(older)
                    .with_observation(older_observation)
                    .with_entity_id(id(10, reverse))
                    .with_thread_id(id(900, reverse))
                    .prepare_write_plan(&RememberPlanDefaults::fixed("older observation", time())),
            )
            .await;
            for (from, relation, to) in [
                (200, RelationType::AssociatedWith, 201),
                (201, RelationType::Supports, 200),
            ] {
                memory
                    .link(MemoryLinkDraft::new(
                        ObjectType::Observation,
                        id(from, reverse),
                        relation,
                        ObjectType::Observation,
                        id(to, reverse),
                    ))
                    .await
                    .unwrap();
            }
            for topic in [None, Some("lighthouse repair")] {
                let mut context = query(topic, 1, 8);
                context.graph_limits.max_depth = 4;
                context.object_type_defaults = vec![ObjectType::Episode];
                results.push((
                    reverse,
                    topic.is_some(),
                    memory.retrieve(context).await.unwrap(),
                ));
            }
            test_support::close_and_remove_root(memory, root).await;
        }
        for (reverse, topic, result) in results {
            assert_eq!(result.pack.relevant_episodes[0].id, id(100, reverse));
            assert!(result
                .pack
                .salient_observations
                .iter()
                .any(|o| o.id == id(200, reverse)));
            assert_eq!(
                result
                    .pack
                    .salient_observations
                    .iter()
                    .any(|o| o.id == id(201, reverse)),
                topic
            );
            assert_eq!(
                result
                    .pack
                    .relevant_episodes
                    .iter()
                    .any(|e| e.id == id(101, reverse)),
                topic
            );
        }
    }

    #[tokio::test]
    async fn a_real_mention_reaches_its_occasion_but_not_a_sibling_at_depth_two() {
        let mut results = Vec::new();
        for reverse in [false, true] {
            let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
            let mut person = EntityDraft::new();
            person.id = Some(id(10, reverse));
            let mut other_person = EntityDraft::new();
            other_person.id = Some(id(11, reverse));
            let mut source = EpisodeDraft::new("a shared occasion");
            source.id = Some(id(100, reverse));
            source.scene = Some(Scene::at((time() - Duration::days(1)).fixed_offset()));
            let mut observation = ObservationDraft::new(id(100, reverse), "Alex will repair it");
            observation.id = Some(id(200, reverse));
            let mut plan = RememberInput::new("a conversation")
                .with_entity(person)
                .with_entity(other_person)
                .with_episode(source)
                .with_observation(observation)
                .with_memory_link(MemoryLinkDraft::new(
                    ObjectType::Observation,
                    id(200, reverse),
                    RelationType::Mentions,
                    ObjectType::Entity,
                    id(10, reverse),
                ))
                .prepare_write_plan(&RememberPlanDefaults::fixed("real mention", time()));
            let mut sibling = ObservationDraft::new(id(100, reverse), "the tide is coming in");
            sibling.id = Some(id(201, reverse));
            sibling.created_at = Some(time());
            sibling.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
            plan = plan.with_candidate(MemoryCandidate::Observation(ObservationCandidate::new(
                sibling,
                provenance(),
            )));
            for n in 300..310 {
                // Aboutness selectivity counts other remarks, not unrelated episodes.
                let mut source = EpisodeDraft::new("another conversation");
                source.id = Some(id(n, reverse));
                source.scene = Some(Scene::at((time() - Duration::days(2)).fixed_offset()));
                let mut remark =
                    ObservationDraft::new(id(n, reverse), "Someone else was discussed");
                remark.id = Some(id(n + 1000, reverse));
                let background = RememberInput::new(format!("background {n}"))
                    .with_episode(source)
                    .with_observation(remark)
                    .with_memory_link(MemoryLinkDraft::new(
                        ObjectType::Observation,
                        id(n + 1000, reverse),
                        RelationType::Mentions,
                        ObjectType::Entity,
                        id(11, reverse),
                    ))
                    .prepare_write_plan(&RememberPlanDefaults::fixed(
                        &format!("background {n}"),
                        time(),
                    ));
                plan.candidates.extend(background.candidates);
            }
            commit(&memory, plan).await;
            let mut context = query(None, 1, 8);
            context.scene.participants.push(SceneParticipant {
                key: Some(id(10, reverse)),
                ..Default::default()
            });
            context.cue_floors.participant = 1;
            context.graph_limits.max_depth = 2;
            results.push((reverse, memory.retrieve(context).await.unwrap()));
            test_support::close_and_remove_root(memory, root).await;
        }
        for (reverse, result) in results {
            assert_eq!(
                result
                    .pack
                    .relevant_episodes
                    .iter()
                    .map(|e| e.id)
                    .collect::<Vec<_>>(),
                [id(100, reverse)]
            );
            assert_eq!(
                result
                    .pack
                    .salient_observations
                    .iter()
                    .map(|o| o.id)
                    .collect::<Vec<_>>(),
                [id(200, reverse)]
            );
        }
    }

    mod place_behavior {
        use super::*;

        fn source(n: u128, days: i64, setting: Option<&str>, reverse: bool) -> EpisodeDraft {
            let mut draft = EpisodeDraft::new("ordinary source");
            draft.id = Some(id(n, reverse));
            let mut scene = Scene::at((time() - Duration::days(days)).fixed_offset());
            scene.setting.key = setting.map(str::to_owned);
            draft.scene = Some(scene);
            draft.created_at = Some(time());
            draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
            draft
        }

        fn belief(
            n: u128,
            source: u128,
            days: i64,
            text: &str,
            reverse: bool,
        ) -> DerivedMemoryDraft {
            let mut draft = DerivedMemoryDraft::new(DerivedType::Claim, text)
                .with_source_episode(id(source, reverse));
            draft.id = Some(id(n, reverse));
            draft.created_at = Some(time() - Duration::days(days));
            draft.updated_at = draft.created_at;
            draft.salience_score = 0.0;
            draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
            draft
        }

        async fn write(memory: &CharacterMemory, input: RememberInput) {
            let defaults = RememberPlanDefaults::fixed(&input.content, time());
            commit(memory, input.prepare_write_plan(&defaults)).await;
        }

        fn claims(result: &RetrieveOutcome) -> Vec<MemoryId> {
            result
                .pack
                .derived_memories
                .iter()
                .map(|entry| entry.memory.id)
                .collect()
        }

        fn cue_score(result: &RetrieveOutcome, id: MemoryId) -> f32 {
            result
                .trace
                .as_ref()
                .unwrap()
                .section_assignments
                .iter()
                .find_map(|row| {
                    if row.object.id != id {
                        return None;
                    }
                    match row.reason {
                        SectionAssignmentReason::Selected { scores } => scores.cue_score,
                        _ => None,
                    }
                })
                .unwrap()
        }

        #[tokio::test]
        async fn an_office_key_leaves_room_for_every_person_present() {
            for reverse in [false, true] {
                let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
                write(
                    &memory,
                    RememberInput::new("office formation").with_episode(source(
                        101,
                        20,
                        Some("office"),
                        reverse,
                    )),
                )
                .await;
                let mut input = RememberInput::new("six people and office memories")
                    .with_episode(source(100, 20, None, reverse));
                for person in 0..6 {
                    let mut entity = EntityDraft::new();
                    entity.id = Some(id(10 + person, reverse));
                    input = input.with_entity(entity);
                    for offset in 0..2 {
                        let mut state = belief(
                            200 + person * 10 + offset,
                            100,
                            10 - offset as i64,
                            "personal state",
                            reverse,
                        );
                        state.entity_ids.push(id(10 + person, reverse));
                        input = input.with_derived_memory(state);
                    }
                }
                for n in 300..312 {
                    input = input.with_derived_memory(belief(
                        n,
                        101,
                        1 + (311 - n) as i64,
                        "office memory",
                        reverse,
                    ));
                }
                for n in 400..408 {
                    input = input.with_derived_memory(belief(
                        n,
                        100,
                        2 + (407 - n) as i64,
                        "quasar telescope astronomy spectroscopy",
                        reverse,
                    ));
                }
                write(&memory, input).await;
                let mut context = query(Some("quasar telescope astronomy spectroscopy"), 12, 12);
                context.candidate_limits.max_vector_candidates = 8;
                context.graph_limits.max_depth = 1;
                context.cue_floors.participant = 1;
                context.cue_floors.topic = 1;
                context.cue_floors.place = 1;
                context.scene.participants = (0..6)
                    .map(|person| SceneParticipant {
                        key: Some(id(10 + person, reverse)),
                        ..Default::default()
                    })
                    .collect();
                let without = memory.retrieve(context.clone()).await.unwrap();
                context.scene.setting.key = Some("office".into());
                let with = memory.retrieve(context).await.unwrap();
                test_support::close_and_remove_root(memory, root).await;
                for person in 0..6 {
                    let count = |result: &RetrieveOutcome| {
                        claims(result)
                            .iter()
                            .filter(|&&member| {
                                [
                                    id(200 + person * 10, reverse),
                                    id(201 + person * 10, reverse),
                                ]
                                .contains(&member)
                            })
                            .count()
                    };
                    assert!(count(&with) > 0, "person {person}");
                    assert_eq!(count(&with), count(&without));
                }
                assert_eq!(
                    claims(&with)
                        .iter()
                        .filter(|&&member| (300..312).any(|n| member == id(n, reverse)))
                        .count(),
                    1
                );
            }
        }

        #[tokio::test]
        async fn activity_does_not_join_the_round_among_people() {
            for reverse in [false, true] {
                let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
                let mut thread = MemoryThreadDraft::new("work", "work");
                thread.id = Some(id(500, reverse));
                let mut input = RememberInput::new("people and activity")
                    .with_memory_thread(thread)
                    .with_episode(source(100, 20, None, reverse));
                for person in 0..6 {
                    let mut entity = EntityDraft::new();
                    entity.id = Some(id(10 + person, reverse));
                    input = input.with_entity(entity);
                    for offset in 0..2 {
                        let mut state = belief(
                            200 + person * 10 + offset,
                            100,
                            10 - offset as i64,
                            "personal state",
                            reverse,
                        );
                        state.entity_ids.push(id(10 + person, reverse));
                        input = input.with_derived_memory(state);
                    }
                }
                for n in 600..608 {
                    let mut member =
                        belief(n, 100, 1 + (607 - n) as i64, "activity member", reverse);
                    member.thread_ids.push(id(500, reverse));
                    input = input.with_derived_memory(member);
                }
                write(&memory, input).await;
                let mut context = query(None, 24, 24);
                context.graph_limits.max_depth = 1;
                context.activity = Some(ActivityRef::Thread(id(500, reverse)));
                context.scene.participants = (0..6)
                    .map(|person| SceneParticipant {
                        key: Some(id(10 + person, reverse)),
                        ..Default::default()
                    })
                    .collect();
                let result = memory.retrieve(context).await.unwrap();
                test_support::close_and_remove_root(memory, root).await;
                let ordered = claims(&result);
                assert_eq!(
                    &ordered[..8],
                    &(600..608).rev().map(|n| id(n, reverse)).collect::<Vec<_>>()
                );
                assert_eq!(
                    &ordered[8..14],
                    &(0..6)
                        .map(|person| id(201 + person * 10, reverse))
                        .collect::<Vec<_>>()
                );
            }
        }

        #[tokio::test]
        async fn home_reserves_its_newest_memory_across_distinct_topics() {
            for reverse in [false, true] {
                let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
                let mut salient = belief(500, 100, 100, "old important home belief", reverse);
                salient.salience_score = 1.0;
                let mut input = RememberInput::new("a constant home key")
                    .with_episode(source(100, 100, Some("home"), reverse))
                    .with_derived_memory(salient)
                    .with_derived_memory(belief(501, 100, 0, "newest home memory", reverse));
                let topics = [
                    "quasar telescope astronomy spectroscopy",
                    "orchid botany greenhouse fertilizer",
                    "violin sonata orchestra rehearsal",
                ];
                for (offset, topic) in topics.iter().enumerate() {
                    input = input.with_derived_memory(belief(
                        600 + offset as u128,
                        100,
                        5,
                        topic,
                        reverse,
                    ));
                }
                write(&memory, input).await;
                for topic in topics {
                    let mut context = query(Some(topic), 8, 2);
                    context.candidate_limits.max_vector_candidates = 1;
                    context.scene.setting.key = Some("home".into());
                    context.cue_floors.place = 1;
                    context.cue_floors.topic = 1;
                    let result = memory.retrieve(context).await.unwrap();
                    assert!(claims(&result).contains(&id(501, reverse)));
                    assert!(!claims(&result).contains(&id(500, reverse)));
                    assert!(result
                        .trace
                        .as_ref()
                        .unwrap()
                        .floor_admissions
                        .iter()
                        .any(|row| row.cue_kind == CueKind::Place
                            && row.object.id == id(501, reverse)));
                }
                test_support::close_and_remove_root(memory, root).await;
            }
        }

        #[tokio::test]
        async fn place_keys_share_one_newest_first_road_and_budget() {
            let mut outcomes = Vec::new();
            for reverse in [false, true] {
                let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
                let mut home = RememberInput::new("home memories").with_episode(source(
                    100,
                    20,
                    Some("home"),
                    reverse,
                ));
                for n in 500..503 {
                    home = home.with_derived_memory(belief(
                        n,
                        100,
                        (503 - n) as i64,
                        "home memory",
                        reverse,
                    ));
                }
                write(&memory, home).await;
                let values = [
                    ("weather".into(), "rain".into()),
                    ("mood".into(), "calm".into()),
                ]
                .into_iter()
                .collect();
                let mut latest = source(200, 20, None, reverse);
                latest.scene.as_mut().unwrap().custom_values = values;
                let values = latest.scene.as_ref().unwrap().custom_values.clone();
                write(
                    &memory,
                    RememberInput::new("newest context memory")
                        .with_episode(latest)
                        .with_derived_memory(belief(600, 200, 0, "newest memory", reverse)),
                )
                .await;
                for cap in [1, 3] {
                    let mut context = query(None, cap, 3);
                    context.scene.setting.key = Some("home".into());
                    context.scene.custom_values = values.clone();
                    context.cue_floors.place = 1;
                    let result = memory.retrieve(context).await.unwrap();
                    let trace = result.trace.unwrap();
                    let number = |id: MemoryId| {
                        if reverse {
                            100_000 - id.as_u128()
                        } else {
                            id.as_u128()
                        }
                    };
                    let floor = trace
                        .floor_admissions
                        .iter()
                        .filter(|row| {
                            row.cue_kind == CueKind::Place && row.stage == CueFloorStage::GraphRoots
                        })
                        .map(|row| number(row.object.id))
                        .collect::<Vec<_>>();
                    let mut contributed = trace
                        .graph_expansions
                        .iter()
                        .filter(|row| row.source == GraphRootSource::Place)
                        .map(|row| number(row.root.id))
                        .collect::<Vec<_>>();
                    contributed.sort_unstable();
                    outcomes.push((reverse, cap, floor, contributed));
                }
                test_support::close_and_remove_root(memory, root).await;
            }
            // At cap one the floor must promote the newest Place memory past recency.
            assert!(
                outcomes
                    .iter()
                    .filter(|(_, cap, _, _)| *cap == 1)
                    .all(|(_, _, floor, _)| floor == &[600]),
                "{outcomes:?}"
            );
            for (_, cap, _, contributed) in outcomes {
                assert_eq!(
                    contributed,
                    if cap == 1 {
                        vec![600]
                    } else {
                        vec![501, 502, 600]
                    }
                );
            }
        }

        #[tokio::test]
        async fn a_place_reminder_only_opens_history_when_the_topic_also_matches() {
            for reverse in [false, true] {
                for custom in [false, true] {
                    let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
                    let mut entity = EntityDraft::new();
                    entity.id = Some(id(7, reverse));
                    let mut formation = source(100, 2, (!custom).then_some("library"), reverse);
                    if custom {
                        formation
                            .scene
                            .as_mut()
                            .unwrap()
                            .custom_values
                            .insert("weather".into(), "rain".into());
                    }
                    let mut older = source(200, 20, None, reverse);
                    older
                        .scene
                        .as_mut()
                        .unwrap()
                        .participants
                        .push(SceneParticipant {
                            key: Some(id(7, reverse)),
                            ..Default::default()
                        });
                    let mut learned = belief(
                        400,
                        100,
                        1,
                        "quasar telescope astronomy spectroscopy",
                        reverse,
                    );
                    learned.entity_ids.push(id(7, reverse));
                    write(
                        &memory,
                        RememberInput::new("older encounter")
                            .with_entity(entity)
                            .with_entity_id(id(7, reverse))
                            .with_episode(older),
                    )
                    .await;
                    write(
                        &memory,
                        RememberInput::new("a belief formed here about someone")
                            .with_episode(formation)
                            .with_derived_memory(learned),
                    )
                    .await;
                    let mut context = query(None, 8, 8);
                    context.graph_limits.max_depth = 3;
                    context.candidate_limits.max_vector_candidates = 1;
                    context.cue_floors.place = 1;
                    context.time_range = Some(TimeRange {
                        start: time() - Duration::days(400),
                        end: time() - Duration::days(399),
                    });
                    if custom {
                        context
                            .scene
                            .custom_values
                            .insert("weather".into(), "rain".into());
                    } else {
                        context.scene.setting.key = Some("library".into());
                    }
                    let reminder = memory.retrieve(context.clone()).await.unwrap();
                    context.topic = Some("quasar telescope astronomy spectroscopy".into());
                    let matched = memory.retrieve(context).await.unwrap();
                    test_support::close_and_remove_root(memory, root).await;
                    assert_eq!(cue_score(&reminder, id(400, reverse)), 0.0);
                    assert!(!reminder
                        .pack
                        .relevant_episodes
                        .iter()
                        .any(|episode| episode.id == id(200, reverse)));
                    let topic_score = matched
                        .trace
                        .as_ref()
                        .unwrap()
                        .vector_candidates
                        .iter()
                        .find(|row| row.object.id == id(400, reverse))
                        .unwrap()
                        .score;
                    assert!(topic_score > 0.0);
                    assert_eq!(cue_score(&matched, id(400, reverse)), topic_score);
                    assert!(matched
                        .pack
                        .relevant_episodes
                        .iter()
                        .any(|episode| episode.id == id(200, reverse)));
                }
            }
        }
    }

    #[tokio::test]
    async fn a_range_replaces_the_recency_window() {
        for reverse in [false, true] {
            let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
            let plan = episode(
                episode(RememberWritePlan::new(), 10, 0, 0.0, None, reverse),
                20,
                8,
                0.0,
                None,
                reverse,
            );
            commit(&memory, plan).await;
            let mut context = query(None, 4, 2);
            context.time_range = Some(TimeRange {
                start: time() - Duration::days(9),
                end: time() - Duration::days(7),
            });
            let ranged = memory.retrieve(context).await.unwrap();
            assert!(ranged
                .pack
                .relevant_episodes
                .iter()
                .any(|episode| episode.id == id(20, reverse)));
            assert!(!ranged
                .pack
                .relevant_episodes
                .iter()
                .any(|episode| episode.id == id(10, reverse)));
            assert!(ranged
                .trace
                .unwrap()
                .section_assignments
                .iter()
                .all(|row| !row.cue_kinds.contains(&CueKind::Recency)));
            let recent = memory.retrieve(query(None, 1, 1)).await.unwrap();
            assert_eq!(recent.pack.relevant_episodes[0].id, id(10, reverse));
            test_support::close_and_remove_root(memory, root).await;
        }
    }

    #[tokio::test]
    async fn a_salient_or_shared_anniversary_can_survive_a_year_of_daily_occasions() {
        let ordinary = EpisodeDraft::new("ordinary").salience_score;
        for reverse in [false, true] {
            for (salience, shared) in [(ordinary, false), (1.0, false), (ordinary, true)] {
                let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
                let mut plan = RememberWritePlan::new();
                if shared {
                    let mut entity = EntityDraft::new();
                    entity.id = Some(id(77, reverse));
                    entity.created_at = Some(time());
                    entity.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
                    plan = plan.with_candidate(MemoryCandidate::Entity(EntityCandidate::new(
                        entity,
                        provenance(),
                    )));
                }
                for days in 0..367 {
                    plan = episode(
                        plan,
                        1000 + days as u128,
                        days,
                        if days == 365 { salience } else { ordinary },
                        None,
                        reverse,
                    );
                }
                if shared {
                    for candidate in &mut plan.candidates {
                        if let MemoryCandidate::Episode(candidate) = candidate {
                            if candidate.draft.id == Some(id(1365, reverse)) {
                                candidate.draft.scene.as_mut().unwrap().participants.push(
                                    SceneParticipant {
                                        key: Some(id(77, reverse)),
                                        ..Default::default()
                                    },
                                );
                            }
                        }
                    }
                }
                commit(&memory, plan).await;
                let mut context = query(None, 12, 12);
                if shared {
                    context.scene.participants.push(SceneParticipant {
                        key: Some(id(77, reverse)),
                        ..Default::default()
                    });
                    context.cue_floors.date_match = 1;
                }
                let result = memory.retrieve(context).await.unwrap();
                assert_eq!(
                    result
                        .pack
                        .relevant_episodes
                        .iter()
                        .any(|episode| episode.id == id(1365, reverse)),
                    shared || salience > ordinary
                );
                if !shared && salience == ordinary {
                    assert!(result
                        .pack
                        .relevant_episodes
                        .iter()
                        .all(|episode| episode.scene.time >= time() - Duration::days(11)));
                }
                test_support::close_and_remove_root(memory, root).await;
            }
        }
    }

    #[tokio::test]
    async fn shared_and_unshared_anniversaries_have_independent_room() {
        for reverse in [false, true] {
            let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
            let mut entity = EntityDraft::new();
            entity.id = Some(id(77, reverse));
            entity.created_at = Some(time());
            entity.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
            let mut plan = RememberWritePlan::new().with_candidate(MemoryCandidate::Entity(
                EntityCandidate::new(entity, provenance()),
            ));
            for (n, year, shared) in [
                (200, 2025, false),
                (201, 2024, false),
                (202, 2023, false),
                (300, 2022, true),
                (301, 2021, true),
                (302, 2020, true),
            ] {
                let mut draft = EpisodeDraft::new("anniversary occasion");
                draft.id = Some(id(n, reverse));
                draft.created_at = Some(time());
                draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
                draft.salience_score = 1.0;
                let mut scene = Scene::at(format!("{year}-09-23T12:00:00Z").parse().unwrap());
                if shared {
                    scene.participants.push(SceneParticipant {
                        key: Some(id(77, reverse)),
                        ..Default::default()
                    });
                }
                draft.scene = Some(scene);
                plan = plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
                    draft,
                    provenance(),
                )));
            }
            commit(&memory, plan).await;
            let mut context = query(None, 7, 3);
            context.scene.participants.push(SceneParticipant {
                key: Some(id(77, reverse)),
                ..Default::default()
            });
            context.cue_floors.date_match = 1;
            let result = memory.retrieve(context).await.unwrap();
            let selected = roots(&result);
            assert_eq!(
                (200..203)
                    .filter(|&n| selected.contains(&id(n, reverse)))
                    .count(),
                3
            );
            assert_eq!(
                (300..303)
                    .filter(|&n| selected.contains(&id(n, reverse)))
                    .count(),
                3
            );
            assert!(result
                .pack
                .relevant_episodes
                .iter()
                .any(|episode| episode.id == id(300, reverse)));
            test_support::close_and_remove_root(memory, root).await;
        }
    }

    #[tokio::test]
    async fn descriptions_do_not_take_spare_root_turns() {
        for reverse in [false, true] {
            for setting in [false, true] {
                let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
                let mut plan = episode(
                    episode(
                        RememberWritePlan::new(),
                        100,
                        30,
                        0.0,
                        Some("orchid"),
                        reverse,
                    ),
                    101,
                    31,
                    0.0,
                    Some("orchid"),
                    reverse,
                );
                plan = episode(plan, 200, 2, 0.0, Some("quiet afternoon"), reverse);
                for candidate in &mut plan.candidates {
                    if let MemoryCandidate::Episode(candidate) = candidate {
                        if candidate.draft.id == Some(id(200, reverse)) {
                            let scene = candidate.draft.scene.as_mut().unwrap();
                            scene.setting.words = Some("botanist astronomer".into());
                            scene.participants.push(SceneParticipant {
                                description: Some("botanist astronomer".into()),
                                ..Default::default()
                            });
                        }
                    }
                }
                commit(&memory, plan).await;
                let mut context = query(Some("Episode summary: orchid"), 2, 2);
                context.cue_floors.topic = 1;
                if setting {
                    context.scene.setting.words = Some("botanist".into());
                } else {
                    context.scene.participants.push(SceneParticipant {
                        description: Some("botanist".into()),
                        ..Default::default()
                    });
                }
                let result = memory.retrieve(context).await.unwrap();
                assert!(result
                    .trace
                    .as_ref()
                    .unwrap()
                    .scene_cue_searches
                    .iter()
                    .any(|search| search.best_score.is_some_and(|score| score > 0.0)));
                assert!(!roots(&result).contains(&id(200, reverse)));
                test_support::close_and_remove_root(memory, root).await;
            }
        }
    }

    #[tokio::test]
    async fn a_recency_reservation_uses_the_latest_occasion() {
        for reverse in [false, true] {
            let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
            let plan = episode(
                episode(
                    RememberWritePlan::new(),
                    100,
                    30,
                    0.0,
                    Some("orchid"),
                    reverse,
                ),
                900,
                0,
                0.0,
                None,
                reverse,
            );
            commit(&memory, plan).await;
            let mut context = query(Some("orchid"), 1, 2);
            let unreserved = memory.retrieve(context.clone()).await.unwrap();
            assert!(!roots(&unreserved).contains(&id(900, reverse)));
            context.cue_floors.recency = 1;
            let reserved = memory.retrieve(context).await.unwrap();
            assert!(roots(&reserved).contains(&id(900, reverse)));
            test_support::close_and_remove_root(memory, root).await;
        }
    }

    #[tokio::test]
    async fn a_timestamped_occasion_beats_an_unmatched_topic_tail() {
        for reverse in [false, true] {
            let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
            let mut plan = episode(
                episode(
                    RememberWritePlan::new(),
                    100,
                    30,
                    0.0,
                    Some("orchid"),
                    reverse,
                ),
                101,
                31,
                0.0,
                Some("orchid"),
                reverse,
            );
            plan = episode(plan, 900, 0, 0.0, None, reverse);
            commit(&memory, plan).await;
            let mut context = query(Some("zebra"), 2, 1);
            context.cue_floors.topic = 1;
            let result = memory.retrieve(context).await.unwrap();
            assert!(result
                .trace
                .as_ref()
                .unwrap()
                .vector_candidates
                .iter()
                .all(|candidate| candidate.score.to_bits() == 0));
            assert!(roots(&result).contains(&id(900, reverse)));
            test_support::close_and_remove_root(memory, root).await;
        }
    }

    #[tokio::test]
    async fn only_a_zero_score_overlap_uses_root_time() {
        for reverse in [false, true] {
            let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
            let plan = episode(
                episode(
                    RememberWritePlan::new(),
                    100,
                    30,
                    0.0,
                    Some("orchid"),
                    reverse,
                ),
                101,
                0,
                0.0,
                Some("orchid"),
                reverse,
            );
            commit(&memory, plan).await;
            let positive = memory.retrieve(query(Some("orchid"), 1, 2)).await.unwrap();
            let first_match = &positive.trace.as_ref().unwrap().vector_candidates[0];
            assert!(first_match.score > 0.0);
            assert_eq!(roots(&positive)[0], first_match.object.id);
            let unmatched = memory.retrieve(query(Some("zebra"), 1, 2)).await.unwrap();
            assert_eq!(roots(&unmatched)[0], id(101, reverse));
            test_support::close_and_remove_root(memory, root).await;
        }
    }

    #[tokio::test]
    async fn quiet_recency_preserves_a_saturated_strong_topic_pack() {
        for reverse in [false, true] {
            let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
            let mut plan = RememberWritePlan::new();
            for n in 100..104 {
                plan = episode(plan, n, n as i64, 0.0, Some("orchid"), reverse);
            }
            commit(&memory, plan).await;
            let mut context = query(Some("Episode summary: orchid"), 2, 2);
            context.cue_floors.topic = 1;
            let before = memory.retrieve(context.clone()).await.unwrap();
            commit(
                &memory,
                episode(RememberWritePlan::new(), 900, 0, 1.0, None, reverse),
            )
            .await;
            let after = memory.retrieve(context).await.unwrap();
            assert_eq!(roots(&before), roots(&after));
            assert_eq!(before.pack, after.pack);
            assert_eq!(before.scene, after.scene);
            assert_eq!(before.activity, after.activity);
            assert_eq!(before.time_range, after.time_range);
            assert_eq!(before.scene_references, after.scene_references);
            assert_eq!(before.memory_scenes, after.memory_scenes);
            let scores = |result: &RetrieveOutcome| {
                result
                    .trace
                    .as_ref()
                    .unwrap()
                    .section_assignments
                    .iter()
                    .filter_map(|row| match &row.reason {
                        SectionAssignmentReason::Selected { scores } => Some((row.object, *scores)),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(scores(&before), scores(&after));
            assert!(scores(&after)
                .iter()
                .all(|(_, scores)| scores.final_score > 0.35));
            test_support::close_and_remove_root(memory, root).await;
        }
    }

    #[tokio::test]
    async fn section_ties_use_observed_parent_and_creation_times() {
        for reverse in [false, true] {
            let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
            let mut plan = episode(
                episode(RememberWritePlan::new(), 100, 10, 0.0, None, reverse),
                101,
                3,
                0.0,
                None,
                reverse,
            );
            for n in [200, 201] {
                let mut draft = ObservationDraft::new(id(100, reverse), "orchid");
                draft.id = Some(id(n, reverse));
                draft.observed_at = (n == 201).then_some(time() - Duration::days(1));
                draft.created_at = Some(time() - Duration::days(if n == 200 { 0 } else { 20 }));
                draft.salience_score = 0.0;
                draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
                plan = indexed(
                    plan.with_candidate(MemoryCandidate::Observation(ObservationCandidate::new(
                        draft,
                        provenance(),
                    ))),
                    ObjectType::Observation,
                    n,
                    reverse,
                );
            }
            for (n, days) in [(300, 5), (301, 1)] {
                let mut draft = DerivedMemoryDraft::new(DerivedType::Claim, "orchid")
                    .with_source_episode(id(100, reverse));
                draft.id = Some(id(n, reverse));
                draft.created_at = Some(time() - Duration::days(days));
                draft.updated_at = draft.created_at;
                draft.salience_score = 0.0;
                draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
                plan = indexed(
                    plan.with_candidate(MemoryCandidate::DerivedMemory(
                        DerivedMemoryCandidate::new(draft, provenance()),
                    )),
                    ObjectType::DerivedMemory,
                    n,
                    reverse,
                );
            }
            commit(&memory, plan).await;
            let result = memory.retrieve(query(Some("zebra"), 6, 6)).await.unwrap();
            assert_eq!(result.pack.relevant_episodes[0].id, id(101, reverse));
            assert_eq!(
                result
                    .pack
                    .salient_observations
                    .iter()
                    .map(|object| object.id)
                    .collect::<Vec<_>>(),
                [id(201, reverse), id(200, reverse)]
            );
            assert_eq!(
                result
                    .pack
                    .derived_memories
                    .iter()
                    .map(|object| object.memory.id)
                    .collect::<Vec<_>>(),
                [id(301, reverse), id(300, reverse)]
            );
            test_support::close_and_remove_root(memory, root).await;
        }
    }

    #[tokio::test]
    async fn an_activity_reservation_takes_its_thread_before_a_member() {
        for reverse in [false, true] {
            let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
            let mut thread = MemoryThreadDraft::new("work", "work");
            thread.id = Some(id(600, reverse));
            thread.created_at = Some(time());
            thread.updated_at = Some(time());
            thread.last_touched_at = Some(time());
            thread.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
            let mut plan = RememberWritePlan::new().with_candidate(MemoryCandidate::MemoryThread(
                MemoryThreadCandidate::new(thread, provenance()),
            ));
            for (source, members) in [
                (101, vec![301, 302, 303]),
                (102, vec![311, 312]),
                (103, vec![321]),
            ] {
                let mut scene = Scene::at((time() - Duration::days(10)).fixed_offset());
                if source == 101 {
                    scene.setting.key = Some("office".into());
                }
                if source == 102 {
                    scene.custom_values.insert("project".into(), "42".into());
                }
                let mut draft = EpisodeDraft::new("source experience");
                draft.id = Some(id(source, reverse));
                draft.scene = Some(scene);
                draft.created_at = Some(time());
                draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
                plan = plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
                    draft,
                    provenance(),
                )));
                for n in members {
                    let mut draft = DerivedMemoryDraft::new(DerivedType::Claim, "current state")
                        .with_source_episode(id(source, reverse));
                    draft.id = Some(id(n, reverse));
                    draft.created_at = Some(time() - Duration::seconds(i64::from(n == 312)));
                    draft.updated_at = draft.created_at;
                    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
                    draft.salience_score = if n == 301 { 1.0 } else { 0.5 };
                    if n == 321 {
                        draft.thread_ids.push(id(600, reverse));
                    }
                    plan = plan.with_candidate(MemoryCandidate::DerivedMemory(
                        DerivedMemoryCandidate::new(draft, provenance()),
                    ));
                }
            }
            commit(&memory, plan).await;
            // Two reserved seats keep this about the thread head; Place takes no spare turn.
            let mut context = query(None, 2, 8);
            context.scene.setting.key = Some("office".into());
            context
                .scene
                .custom_values
                .insert("project".into(), "42".into());
            context.activity = Some(ActivityRef::Thread(id(600, reverse)));
            context.cue_floors = RetrievalCueFloors::default();
            let result = memory.retrieve(context).await.unwrap();
            assert_eq!(result.pack.active_threads[0].id, id(600, reverse));
            assert!(!roots(&result).contains(&id(321, reverse)));
            test_support::close_and_remove_root(memory, root).await;
        }
    }
}

#[path = "support/mod.rs"]
pub mod test_support;

#[tokio::test]
async fn public_remember_and_retrieve_use_graph_authoritative_path() {
    let (memory, root) = test_support::try_setup_character_memory()
        .await
        .expect("unexpected live public facade setup failure");

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
            .remember(
                RememberInput::new("The user prefers deterministic public facade tests.")
                    .with_episode(episode)
                    .with_observation(observation)
                    .with_derived_memory(preference),
                RememberOptions::default(),
            )
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
    test_support::close_and_remove_root(memory, root).await;
    test_result.expect("live public facade test should pass");
}

#[tokio::test]
async fn public_correct_and_forget_hide_stale_memories_from_normal_retrieval() {
    let (memory, root) = test_support::try_setup_character_memory()
        .await
        .expect("unexpected live public lifecycle setup failure");

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
            .remember(
                RememberInput::new("The user corrected a public facade preference.")
                    .with_episode(episode)
                    .with_derived_memory(old_preference),
                RememberOptions::default(),
            )
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
    test_support::close_and_remove_root(memory, root).await;
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
