use super::*;

#[tokio::test]
async fn observation_forget_preserves_episode_presence() {
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
            for forgotten in [None, Some(30_001), Some(30_002)] {
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
                let decisions = result.trace.unwrap().selectivity_decisions;
                // Present-subject aboutness bypasses selectivity.
                assert!(decisions
                    .iter()
                    .all(|row| row.relation != RelationType::Mentions));
                // Episode presence is independent of the remarks retention.
                let presence = decisions
                    .iter()
                    .find(|row| row.relation == RelationType::Involves)
                    .unwrap();
                assert_eq!(
                    presence.entity_count,
                    (in_scene || direct_link).then_some(1)
                );
            }
            memory.close().await.unwrap();
        }
    }
}

#[tokio::test]
async fn mentions_recall_observations_independently_of_parent_lifecycle() {
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
    let result = memory.retrieve(context.clone()).await.unwrap();

    // One subject aboutness budget admits both remarks despite their ubiquity.
    assert_eq!(participant_observations(&result).len(), 2);
    assert!(result
        .trace
        .as_ref()
        .unwrap()
        .selectivity_decisions
        .iter()
        .all(|row| row.relation != RelationType::Mentions));
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::episode(episode),
            "The occasion is forgotten",
        ))
        .await
        .unwrap();
    for include_suppressed in [false, true] {
        context.lifecycle_policy.include_suppressed = include_suppressed;
        let result = memory.retrieve(context.clone()).await.unwrap();

        assert_eq!(participant_observations(&result).len(), 2);
    }
    memory.close().await.unwrap();
}

#[tokio::test]
async fn suppressed_parent_evidence_keeps_its_active_observation_admissible_by_aboutness() {
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
        assert_eq!(result.pack.salient_observations.len(), 1);
        assert_eq!(result.pack.salient_observations[0].id, observation);
        assert_eq!(
            result.pack.salient_observations[0].retention_state,
            RetentionState::Active
        );
        let trace = result.trace.unwrap();
        let omissions = trace
            .lifecycle_filter_decisions
            .iter()
            .filter(|decision| decision.reason == LifecycleFilterReason::SuppressedOmitted)
            .collect::<Vec<_>>();
        // Only the topic traversal encounters the suppressed parent; Mentions is not presence.
        assert_eq!(!omissions.is_empty(), has_topic);
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
        // The remark enters the subject aboutness prefix on its own retention.
        assert_eq!(utilization.retained_count, 1);
        assert_eq!(utilization.omitted_by_fanout_count, 0);
    }
    memory.close().await.unwrap();
}

#[tokio::test]
async fn suppressed_mention_leaves_other_aboutness_eligible() {
    assert_mentions_after_suppression(false).await;
}

#[tokio::test]
async fn suppressed_mention_leaves_its_active_sibling_eligible() {
    assert_mentions_after_suppression(true).await;
}

async fn assert_mentions_after_suppression(active_sibling: bool) {
    let (memory, _) = scene_memory().await;
    create_notion(&memory, 100, None).await;
    create_notion(&memory, 101, None).await;
    for (n, participant) in [(50_000, 100), (50_010, 100)]
        .into_iter()
        .chain((60_000..60_010).map(|n| (n * 10, 101)))
    {
        let episode = write_episode(&memory, n, scene()).await;
        memory
            .link(MemoryLinkDraft::new(
                ObjectType::Observation,
                MemoryId::from_u128(n + 1),
                RelationType::Mentions,
                ObjectType::Entity,
                MemoryId::from_u128(participant),
            ))
            .await
            .unwrap();
        if active_sibling && n == 50_010 {
            let mut observation = ObservationDraft::new(episode, "Another active remark.");
            observation.id = Some(MemoryId::from_u128(50_012));
            observation.created_at = Some(scene().time.to_utc());
            observation.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
            memory
                .commit(
                    RememberWritePlan::new().with_candidate(MemoryCandidate::Observation(
                        ObservationCandidate::new(
                            observation,
                            CandidateProvenance::caller("remark"),
                        ),
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
                    MemoryId::from_u128(50_012),
                ))
                .await
                .unwrap();
        }
    }
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::observation(MemoryId::from_u128(50_011)),
            "Forget the latest remark",
        ))
        .await
        .unwrap();
    let mut context = RetrievalContext::default().with_trace();
    context.scene.participants.push(keyed(100));
    context.graph_limits.max_depth = 1;
    context.candidate_limits.max_graph_roots = 1;
    context.cue_floors.participant = 1;
    let ids = |result: &RetrieveOutcome| {
        participant_observations(result)
            .into_iter()
            .collect::<BTreeSet<_>>()
    };
    let expected = [
        Some(MemoryId::from_u128(50_001)),
        active_sibling.then_some(MemoryId::from_u128(50_012)),
    ]
    .into_iter()
    .flatten()
    .collect::<BTreeSet<_>>();
    let result = memory.retrieve(context.clone()).await.unwrap();
    assert_eq!(ids(&result), expected);
    // Observation suppression still hides the remark, but not its active sibling.
    assert!(result
        .trace
        .as_ref()
        .unwrap()
        .lifecycle_filter_decisions
        .iter()
        .any(|row| row.object.id == MemoryId::from_u128(50_011)
            && row.reason == LifecycleFilterReason::SuppressedOmitted));
    let mut untraced = context.clone();
    untraced.include_trace = false;
    assert_eq!(memory.retrieve(untraced).await.unwrap().pack, result.pack);
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::episode(MemoryId::from_u128(50_010)),
            "Forget the parent occasion",
        ))
        .await
        .unwrap();
    // Aboutness follows the observation's own lifecycle, not its parent's.
    assert_eq!(
        ids(&memory.retrieve(context.clone()).await.unwrap()),
        expected
    );
    context.lifecycle_policy.include_suppressed = true;
    let mut all = expected;
    all.insert(MemoryId::from_u128(50_011));
    assert_eq!(ids(&memory.retrieve(context).await.unwrap()), all);
    memory.close().await.unwrap();
}

#[tokio::test]
async fn ubiquitous_presence_keeps_the_latest_occasion_across_store_sizes_and_directions() {
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
                let mut occasion = scene();
                occasion.time += chrono::Duration::hours(count as i64);
                if route == 0 {
                    occasion.participants.push(keyed(100));
                }
                let mut episode = EpisodeDraft::new("The participant visited.");
                episode.id = Some(episode_id);
                episode.created_at =
                    Some(scene().time.to_utc() - chrono::Duration::hours(count as i64));
                episode.scene = Some(occasion);
                episode.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
                memory
                    .commit(
                        RememberWritePlan::new().with_candidate(MemoryCandidate::Episode(
                            EpisodeCandidate::new(episode, CandidateProvenance::caller("occasion")),
                        )),
                        CommitOptions::default(),
                    )
                    .await
                    .unwrap();
                if route != 0 {
                    let link = if route == 1 {
                        MemoryLinkDraft::new(
                            ObjectType::Episode,
                            episode_id,
                            RelationType::Involves,
                            ObjectType::Entity,
                            participant,
                        )
                    } else {
                        MemoryLinkDraft::new(
                            ObjectType::Entity,
                            participant,
                            RelationType::Involves,
                            ObjectType::Episode,
                            episode_id,
                        )
                    };
                    memory.link(link).await.unwrap();
                }
                let healthy = memory.retrieve(context.clone()).await.unwrap();
                let healthy_stats = std::mem::replace(
                    &mut memory.memory_composition.stats_store,
                    Box::new(InMemoryRetrievalStatsStore::new()),
                );
                let missing = memory.retrieve(context.clone()).await.unwrap();
                memory.memory_composition.stats_store = healthy_stats;
                for (result, fallback) in [(&healthy, false), (&missing, true)] {
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
                        .collect::<BTreeSet<_>>();
                    assert_eq!(
                        occasions,
                        BTreeSet::from([episode_id]),
                        "route={route}, reverse_ids={reverse_ids}, N={count}"
                    );
                    assert_eq!(result.pack.relevant_episodes.len(), count.min(8) as usize);
                    let row = result
                        .trace
                        .as_ref()
                        .unwrap()
                        .selectivity_decisions
                        .iter()
                        .find(|row| {
                            row.root.id == participant && row.relation == RelationType::Involves
                        })
                        .unwrap();
                    assert_eq!((row.chosen_fanout, row.fallback), (1, fallback));
                }
            }
            // Excluded occasions cannot consume the one remaining presence slot.
            let episodes = memory
                .retrieve(context.clone())
                .await
                .unwrap()
                .pack
                .relevant_episodes;
            for episode in episodes {
                memory
                    .forget(ForgetMemoryDraft::suppress(
                        LifecycleTargetRef::episode(episode.id),
                        "Forget recent occasions",
                    ))
                    .await
                    .unwrap();
            }
            let result = memory.retrieve(context.clone()).await.unwrap();
            assert_eq!(result.pack.relevant_episodes.len(), 2);
            assert!(result.rationale.lifecycle_omission_count <= 1);
            for episode in result.pack.relevant_episodes {
                memory
                    .forget(ForgetMemoryDraft::suppress(
                        LifecycleTargetRef::episode(episode.id),
                        "Forget the remaining occasions",
                    ))
                    .await
                    .unwrap();
            }
            let empty = memory.retrieve(context).await.unwrap();
            assert!(empty.pack.relevant_episodes.is_empty());
            assert!(empty.rationale.lifecycle_omission_count <= 1);
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
            // Ruling 69: both write paths record presence on episodes; assert the recalled beliefs.
            let relation = RelationType::Involves;
            assert_eq!(
                result
                    .pack
                    .derived_memories
                    .iter()
                    .map(|entry| entry.memory.id)
                    .collect::<BTreeSet<_>>(),
                BTreeSet::from([
                    MemoryId::from_u128(ubiquitous + 1000),
                    MemoryId::from_u128(rare + 1000)
                ]),
            );
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
                counts,
            ));
            assert!(queries.lock().unwrap().is_empty());
            memory.close().await.unwrap();
        }
    }
    let expected = [false, true]
        .into_iter()
        .flat_map(|caller_built| {
            [100, 101]
                .into_iter()
                .map(move |ubiquitous| (caller_built, ubiquitous, 1, 3, Some((24, 24))))
        })
        .collect::<Vec<_>>();
    assert_eq!(observed, expected);
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
    assert!(!trace.selectivity_decisions.is_empty());
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
    // Rulings 46 and 69: the two occasions also bring their observations.
    assert_eq!(empty.memory_scenes.len(), 4);
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
