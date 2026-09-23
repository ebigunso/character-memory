use super::*;

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
async fn support_age_reports_experience_without_changing_recall() {
    use chrono::{DateTime, Duration, Utc};

    let at = |text: &str| text.parse::<DateTime<Utc>>().unwrap();
    let reference = at("2026-09-20T10:00:00Z");
    let later = at("2026-10-20T10:00:00Z");
    let old = at("2025-07-20T10:00:00Z");
    let recent = reference - Duration::days(1);
    let fallback = reference - Duration::days(3);
    let current = reference - Duration::milliseconds(900);
    let id = MemoryId::from_u128;
    let (memory, queries) = scene_memory().await;
    create_notion(&memory, 100, None).await;
    let mut plan = RememberWritePlan::new();
    for (n, time) in [
        (5000, at("2025-06-20T10:00:00Z")),
        (6000, reference - Duration::days(2)),
        (7000, fallback),
        (8000, reference - Duration::days(2)),
        (9000, at("2026-12-01T10:00:00Z")),
        (10000, reference - Duration::days(1)),
    ] {
        let mut episode = EpisodeDraft::new("An experience behind a belief.");
        episode.id = Some(id(n));
        episode.scene = Some(Scene::at(time.fixed_offset()));
        episode.created_at = Some(later + Duration::days(n as i64));
        episode.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        plan = plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
            episode,
            CandidateProvenance::caller("support age"),
        )));
    }
    for (n, parent, observed_at) in [
        (5001, 5000, Some(old)),
        (6001, 6000, Some(recent)),
        (6002, 6000, Some(recent - Duration::hours(12))),
        (7001, 7000, None),
        (8001, 8000, Some(at("2026-12-02T10:00:00Z"))),
        (10001, 10000, Some(current)),
    ] {
        let mut observation = ObservationDraft::new(id(parent), "Support for a belief.");
        observation.id = Some(id(n));
        observation.observed_at = observed_at;
        observation.created_at = Some(later);
        observation.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        plan = plan.with_candidate(MemoryCandidate::Observation(ObservationCandidate::new(
            observation,
            CandidateProvenance::caller("support age"),
        )));
    }
    for (n, episodes, observations) in [
        (20000, vec![], vec![5001]),
        (20001, vec![], vec![6001]),
        (20002, vec![], vec![7001]),
        (20003, vec![], vec![5001, 6001]),
        (20004, vec![5000, 6000], vec![]),
        (20005, vec![], vec![8001]),
        (20006, vec![9000], vec![]),
        (20007, vec![], vec![]),
        (20008, vec![], vec![6001, 6002]),
        (20009, vec![], vec![10001]),
    ] {
        let mut belief = DerivedMemoryDraft::new(DerivedType::Claim, format!("Belief {n}"));
        belief.id = Some(id(n));
        belief.entity_ids = vec![id(100)];
        belief.given_by_application = episodes.is_empty() && observations.is_empty();
        belief.derived_from_episode_ids = episodes.into_iter().map(id).collect();
        belief.derived_from_observation_ids = observations.into_iter().map(id).collect();
        belief.created_at = Some(later + Duration::days(n as i64));
        belief.updated_at = belief.created_at;
        belief.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        plan = plan.with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
            belief,
            CandidateProvenance::caller("support age"),
        )));
    }
    memory.commit(plan, CommitOptions::default()).await.unwrap();
    let mut mismatches = Vec::new();
    for phase in 0..5 {
        let targets = match phase {
            1 => vec![LifecycleTargetRef::episode(id(6000))],
            2 => vec![LifecycleTargetRef::observation(id(6001))],
            3 => vec![
                LifecycleTargetRef::episode(id(5000)),
                LifecycleTargetRef::episode(id(7000)),
            ],
            _ => vec![],
        };
        for target in targets {
            let mut draft = ForgetMemoryDraft::suppress(target, "Retain the supported belief.");
            draft.cascade_policy.apply_to_derived_from_target = false;
            memory.forget(draft).await.unwrap();
        }
        if phase == 4 {
            let origin = SourceProvenanceReference {
                episode_ids: vec![],
                observation_ids: vec![id(10001)],
                external_refs: vec![],
            };
            let mut replacement = ReplacementDerivedMemoryDraft::new(
                DerivedType::Correction,
                "This belief now rests on a different experience.",
            );
            replacement.id = Some(id(21000));
            replacement.entity_ids = vec![id(100)];
            replacement.derived_from_observation_ids = vec![id(10001)];
            replacement.correction_origin_provenance = origin.clone();
            let mut draft = CorrectMemoryDraft::new(
                CorrectionTarget::derived_memory(id(20000)),
                "Replace the old interpretation.",
            )
            .with_replacement(replacement);
            draft.correction_origin = origin;
            memory.correct(draft).await.unwrap();
        }
        for time in [reference, later] {
            for include_suppressed in [false, true] {
                let mut present = Scene::at(time.fixed_offset());
                present.participants.push(keyed(100));
                let mut context = RetrievalContext::default().with_scene(present);
                context.candidate_limits.max_graph_roots = 1;
                context.graph_limits.max_depth = 1;
                context.graph_limits.max_fanout_per_node = 32;
                context.graph_limits.timeout_ms = None;
                context.graph_limits.allowed_relation_types = vec![RelationType::About];
                context.graph_limits.allowed_object_types =
                    vec![ObjectType::Entity, ObjectType::DerivedMemory];
                context.section_limits.derived_memories = 32;
                context.lifecycle_policy.include_suppressed = include_suppressed;
                let result = memory.retrieve(context.clone()).await.unwrap();
                assert!(result.trace.is_none());
                assert!(result.pack.relevant_episodes.is_empty());
                assert!(result.pack.salient_observations.is_empty());
                assert_eq!(result.pack.derived_memories.len(), 10);
                let traced = memory.retrieve(context.clone().with_trace()).await.unwrap();
                assert_eq!(traced.pack, result.pack);
                assert_eq!(traced.memory_scenes, result.memory_scenes);
                let trace = traced.trace.as_ref().unwrap();
                assert_eq!(
                    trace
                        .graph_expansions
                        .iter()
                        .filter(|root| root.outcome == GraphExpansionOutcome::Expanded)
                        .map(|root| root.root.id)
                        .collect::<Vec<_>>(),
                    [id(100)]
                );
                let expected = [
                    (
                        if phase == 4 { 21000 } else { 20000 },
                        Some(if phase == 4 { current } else { old }),
                    ),
                    (20001, (phase < 2 || include_suppressed).then_some(recent)),
                    (20002, Some(fallback)),
                    (
                        20003,
                        Some(if phase < 2 || include_suppressed {
                            recent
                        } else {
                            old
                        }),
                    ),
                    (
                        20004,
                        if phase == 0 || include_suppressed {
                            Some(reference - Duration::days(2))
                        } else if phase < 3 {
                            Some(at("2025-06-20T10:00:00Z"))
                        } else {
                            None
                        },
                    ),
                    (20005, None),
                    (20006, None),
                    (20007, None),
                    (
                        20008,
                        Some(if phase < 2 || include_suppressed {
                            recent
                        } else {
                            recent - Duration::hours(12)
                        }),
                    ),
                    (20009, Some(current)),
                ]
                .into_iter()
                .map(|(n, support)| {
                    (
                        n.to_string(),
                        support.map(|support| (time - support).num_seconds()),
                    )
                })
                .collect::<std::collections::BTreeMap<_, _>>();
                for entry in &result.memory_scenes {
                    let expected_age = expected[&entry.memory.id.as_u128().to_string()];
                    if entry.seconds_since_support != expected_age {
                        mismatches.push(format!(
                            "phase {phase}, reference {time}, include_suppressed {include_suppressed}, memory {:?}: {:?} != {expected_age:?}",
                            entry.memory, entry.seconds_since_support
                        ));
                    }
                }
            }
        }
    }
    assert!(queries.lock().unwrap().is_empty());
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
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
