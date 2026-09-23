use super::*;

fn assert_episode_order(result: &RetrieveOutcome) {
    let selected = episodes(result);
    assert_eq!(
        selected
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        selected.len()
    );
    for pair in result.pack.relevant_episodes.windows(2) {
        let first = scores(result, pair[0].id.as_u128()).final_score;
        let second = scores(result, pair[1].id.as_u128()).final_score;
        assert!(first >= second, "higher scores lead the pack");
        if first == second {
            assert!(pair[0].scene.time >= pair[1].scene.time, "newer ties lead");
        }
    }
}

fn assert_topic_prefix(result: &RetrieveOutcome, selected: &[u128]) {
    assert_eq!(
        selected
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        selected.len()
    );
    let candidates = &result.trace.as_ref().unwrap().vector_candidates;
    let selected_scores = selected
        .iter()
        .filter_map(
            |n| match candidates.iter().find(|row| row.object.id == id(*n)) {
                Some(row) => {
                    assert!(scores(result, *n).cue_score.unwrap() > 0.16);
                    Some(row.score)
                }
                None => {
                    assert_eq!(*n, 900, "only the quiet latest occasion has no topic score");
                    None
                }
            },
        )
        .collect::<Vec<_>>();
    assert!(!selected_scores.is_empty());
    assert!(selected_scores.windows(2).all(|pair| pair[0] > pair[1]));
    let cutoff = *selected_scores.last().unwrap();
    assert!(
        candidates
            .iter()
            .filter(|row| !selected.contains(&row.object.id.as_u128()))
            .all(|row| row.score < cutoff),
        "the strongest topic candidates fit first"
    );
}

fn assert_scene_only_membership(result: &RetrieveOutcome) {
    let selected = episodes(result);
    assert_eq!(selected.len(), 8);
    for n in [100, 700, 900, 600, 500] {
        assert!(selected.contains(&n));
    }
    assert!(selected[5..].iter().all(|n| (800..820).contains(n)));
    assert!(scores(result, 100).final_score > scores(result, 700).final_score);
    assert!(scores(result, 700).final_score > scores(result, 900).final_score);
    for n in [600, 500] {
        assert_eq!(
            scores(result, 900).final_score,
            scores(result, n).final_score
        );
    }
    assert_episode_order(result);
}

#[tokio::test]
async fn as_of_omissions_report_time_for_roots_and_expanded_memories() {
    for direct_roots in [true, false] {
        let (memory, temp) = open().await;
        let mut plan = episode(RememberWritePlan::new(), 900, 1, 0.5, false, true);
        plan = episode(plan, 100, -1, 0.5, false, direct_roots);
        for (n, parent, observed_at) in [
            (200, 900, Some(time() + Duration::days(1))),
            (300, 100, None),
            (500, 900, None),
        ] {
            let mut draft = ObservationDraft::new(id(parent), format!("Strong {n}"));
            draft.id = Some(id(n));
            draft.created_at = Some(time());
            draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
            draft.observed_at = observed_at;
            plan = plan.with_candidate(MemoryCandidate::Observation(ObservationCandidate::new(
                draft,
                provenance(),
            )));
            if direct_roots && n != 500 {
                plan = indexed(plan, ObjectType::Observation, n);
            }
        }
        for (kind, n) in [(ObjectType::Episode, 100), (ObjectType::Observation, 300)] {
            plan = link(
                plan,
                ObjectType::Episode,
                900,
                kind,
                n,
                RelationType::AssociatedWith,
            );
        }
        commit(&memory, plan).await;
        let context = query(true, false);
        let result = memory.retrieve(context.clone()).await.unwrap();
        assert_eq!(episodes(&result), [900]);
        assert_eq!(
            result
                .pack
                .salient_observations
                .iter()
                .map(|o| o.id)
                .collect::<Vec<_>>(),
            [id(500)]
        );
        let mut untraced_context = context;
        untraced_context.include_trace = false;
        let untraced = memory.retrieve(untraced_context).await.unwrap();
        assert_eq!(untraced.pack, result.pack);
        assert_eq!(untraced.memory_scenes, result.memory_scenes);
        let trace = result.trace.as_ref().unwrap();
        for (kind, n) in [
            (ObjectType::Episode, 100),
            (ObjectType::Observation, 200),
            (ObjectType::Observation, 300),
        ] {
            let object = MemoryObjectRef::new(kind, id(n));
            let reasons = trace
                .lifecycle_filter_decisions
                .iter()
                .filter(|decision| decision.object == object)
                .map(|decision| serde_json::to_value(decision.reason).unwrap())
                .collect::<Vec<_>>();
            assert!(
                !reasons.is_empty()
                    && reasons
                        .iter()
                        .all(|reason| reason == &json!("later_than_reference_time")),
                "{direct_roots}: {object:?}: {reasons:?}"
            );
            let omissions = trace
                .stale_candidate_omissions
                .iter()
                .filter(|omission| omission.candidate == object)
                .map(|omission| serde_json::to_value(omission.reason).unwrap())
                .collect::<Vec<_>>();
            let expected = if direct_roots {
                vec![json!("later_than_reference_time")]
            } else {
                vec![]
            };
            assert_eq!(omissions, expected, "{direct_roots}: {object:?}");
        }
        memory.close().await.unwrap();
        temp.close().unwrap();
    }
}

#[tokio::test]
async fn recency_scene_only_room_and_floor_witnesses() {
    let (memory, root) = open().await;
    commit(&memory, base()).await;
    let default = retrieve(&memory, query(false, false)).await;
    assert_scene_only_membership(&default);
    let default_roots = roots(&default);
    assert_eq!(default_roots.len(), 12);
    assert_eq!(
        default_roots
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        12
    );
    assert_eq!(&default_roots[..8], episodes(&default));
    assert!(default_roots[8..].iter().all(|n| (800..820).contains(n)));
    for n in &episodes(&default)[5..] {
        assert_eq!(
            scores(&default, 900).final_score,
            scores(&default, *n).final_score
        );
    }
    let saturated = retrieve(&memory, query(true, false)).await;
    assert!(recency_episodes(&saturated).is_empty());
    assert_eq!(
        default
            .rationale
            .telemetry
            .unique_graph_root_candidate_count,
        16
    );
    // Rulings 46 and 69: an occasion brings its own observation through ObservedIn.
    assert_eq!(
        default
            .pack
            .salient_observations
            .iter()
            .map(|o| o.id)
            .collect::<Vec<_>>(),
        [id(400)]
    );
    let mut single = query(false, false);
    single.section_limits = room(1);
    let first = retrieve(&memory, single.clone()).await;
    assert!(first.pack.salient_observations.is_empty());
    assert_eq!(episodes(&first), [900]);
    assert_eq!(recency_episodes(&first), [900]);
    assert!(assignment(&first, 300)
        .cue_kinds
        .contains(&CueKind::Recency));
    assert_eq!(scores(&first, 900).cue_score, Some(0.0));
    assert!(first.trace.as_ref().unwrap().vector_candidates.is_empty());
    let mut three = query(false, false);
    three.section_limits = room(3);
    let multiple = retrieve(&memory, three.clone()).await;
    assert_eq!(episodes(&multiple).len(), 3);
    for n in [100, 700, 900] {
        assert!(episodes(&multiple).contains(&n));
    }
    assert_episode_order(&multiple);
    assert_eq!(recency_episodes(&multiple), episodes(&multiple));
    let mut high_floor = three.clone();
    high_floor.cue_floors.recency = 7;
    assert_eq!(
        roots(&memory.retrieve(high_floor).await.unwrap()),
        roots(&multiple)
    );
    let mut zero = query(false, false);
    zero.section_limits = room(0);
    zero.cue_floors.recency = 7;
    let zero = retrieve(&memory, zero).await;
    assert!(roots(&zero).is_empty());
    assert!(zero.pack.relevant_episodes.is_empty());
    let mut tight = three;
    tight.section_limits.relevant_episodes = 1;
    tight.cue_floors.recency = 3;
    let tight = retrieve(&memory, tight).await;
    assert_eq!(
        episodes(&tight),
        [900],
        "the raised floor reserves the latest contribution"
    );
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn recency_keyed_participant_witnesses() {
    let (memory, root) = open().await;
    commit(&memory, base()).await;
    let known = retrieve(&memory, query(false, true)).await;
    assert_scene_only_membership(&known);
    let direct = scores(&known, 900);
    let reminder = scores(&known, 300);
    assert!(direct.final_score > reminder.final_score);
    assert!(direct.cue_score.unwrap() > 0.16);
    assert_eq!(direct.cue_score, reminder.cue_score);
    assert!(direct.graph_score.unwrap() > reminder.graph_score.unwrap());
    assert!(reminder.graph_score.unwrap() > 0.0);
    assert!(direct.salience_score.is_none());
    assert!(episodes(&known)[5..]
        .iter()
        .all(|n| direct.final_score > scores(&known, *n).final_score));
    let mut known_single = query(false, true);
    known_single.section_limits = room(1);
    let known_single = memory.retrieve(known_single).await.unwrap();
    assert_eq!(recency_episodes(&known_single), [900]);
    assert!(assignment(&known_single, 700)
        .cue_kinds
        .contains(&CueKind::Participant));
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn recency_reference_time_and_lifecycle_witnesses() {
    let (memory, root) = open().await;
    commit(&memory, base()).await;
    let mut single = query(false, false);
    single.section_limits = room(1);
    let first = memory.retrieve(single.clone()).await.unwrap();
    let repeated = retrieve(&memory, single.clone()).await;
    assert_eq!(first, repeated);
    let mut untraced = single.clone();
    untraced.include_trace = false;
    let untraced = memory.retrieve(untraced).await.unwrap();
    assert_eq!(first.pack, untraced.pack);
    assert!(untraced.trace.is_none());
    let mut past = single.clone();
    past.scene.time -= Duration::hours(12);
    let past = retrieve(&memory, past).await;
    assert_eq!(episodes(&past), [100]);
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::Episode(id(900)),
            "recency advances",
        ))
        .await
        .unwrap();
    let forgotten = retrieve(&memory, single.clone()).await;
    assert_eq!(episodes(&forgotten), [100]);
    let mut inclusive = single;
    inclusive.lifecycle_policy.include_suppressed = true;
    let inclusive = retrieve(&memory, inclusive).await;
    assert_eq!(episodes(&inclusive), [900]);
    memory.close().await.unwrap();
    root.close().unwrap();
}

async fn pressure_witnesses(strong: u128, weak: bool, overlap: bool) {
    let (memory, root) = open().await;
    let mut plan = episode(RememberWritePlan::new(), 900, 0, 1.0, false, overlap);
    for index in 0..strong {
        plan = episode(plan, 2000 + index, 2 + index as i64, 0.0, false, true);
    }
    if weak {
        plan = episode(plan, 2003, 6, 0.0, false, false);
        plan = belief(plan, 2100, &[2003], true);
    }
    if overlap {
        plan = episode(plan, 750, 40, 0.0, false, false);
        plan = belief(plan, 3200, &[900, 750], false);
    }
    commit(&memory, plan).await;
    let mut context = query(true, false);
    context.section_limits.relevant_episodes = if strong == 48 { 8 } else { 2 };
    let result = retrieve(&memory, context.clone()).await;
    if strong == 2 && !overlap {
        assert_eq!(episodes(&result).len(), 2);
        assert!(!episodes(&result).contains(&900));
        assert_episode_order(&result);
        assert_topic_prefix(&result, &episodes(&result));
        assert!(scores(&result, 2000).final_score > scores(&result, 2001).final_score);
        assert!(scores(&result, 2001).final_score > scores(&result, 900).final_score);
        assert_eq!(roots(&result).len(), 3);
        assert_eq!(&roots(&result)[..2], episodes(&result));
        assert_eq!(roots(&result).last(), Some(&900));
    }
    if weak {
        assert_eq!(episodes(&result).len(), 2);
        assert!(episodes(&result).contains(&2003));
        assert!(!episodes(&result).contains(&900));
        assert_episode_order(&result);
        assert!(scores(&result, 2000).final_score > scores(&result, 2003).final_score);
        assert!(scores(&result, 2003).final_score > scores(&result, 900).final_score);
        assert!(scores(&result, 2003).cue_score > Some(0.0));
        assert!(scores(&result, 2003).cue_score < scores(&result, 2000).cue_score);
        let mut bounded = context.clone();
        bounded.section_limits = room(2);
        let bounded = retrieve(&memory, bounded).await;
        assert_eq!(episodes(&bounded).len(), 2);
        assert!(episodes(&bounded).contains(&900));
        assert!(!episodes(&bounded).contains(&2003));
        assert_episode_order(&bounded);
        assert!(scores(&bounded, 2000).final_score > scores(&bounded, 900).final_score);
        assert!(scores(&bounded, 900).final_score > scores(&bounded, 2003).final_score);
        assert!(
            scores(&result, 2003).graph_score.unwrap()
                > scores(&bounded, 2003).graph_score.unwrap()
        );
        assert!(scores(&bounded, 2003).graph_score.unwrap() > 0.0);
        assert!(!roots(&bounded).contains(&2003));
    }
    if overlap {
        assert_eq!(episodes(&result).len(), 2);
        assert!(episodes(&result).contains(&900));
        assert!(episodes(&result).contains(&2000));
        assert_episode_order(&result);
        assert!(scores(&result, 900).final_score > scores(&result, 2000).final_score);
        assert!(scores(&result, 900).cue_score < scores(&result, 2000).cue_score);
        assert_eq!(
            scores(&result, 900).graph_score.unwrap(),
            scores(&result, 2000).graph_score.unwrap()
        );
        assert!(assignment(&result, 900).cue_kinds.contains(&CueKind::Topic));
        let recent = recency_episodes(&result);
        assert_eq!(recent.len(), 4);
        for n in [900, 2000, 2001, 750] {
            assert!(recent.contains(&n));
        }
        assert!(recent.windows(2).all(
            |pair| scores(&result, pair[0]).final_score > scores(&result, pair[1]).final_score
        ));
        assert!(assignment(&result, 3200)
            .cue_kinds
            .contains(&CueKind::Recency));
        assert!(assignment(&result, 750)
            .cue_kinds
            .contains(&CueKind::Recency));
        let mut bounded = context.clone();
        bounded.section_limits = room(3);
        let bounded = retrieve(&memory, bounded).await;
        assert_eq!(
            assignment(&bounded, 750).cue_kinds,
            std::collections::BTreeSet::from([CueKind::Topic])
        );
        assert_eq!(roots(&bounded).len(), 3);
        assert!(roots(&bounded).contains(&900));
        assert_topic_prefix(&bounded, &roots(&bounded));
        let root = result
            .trace
            .as_ref()
            .unwrap()
            .graph_expansions
            .iter()
            .find(|row| row.root.id == id(900))
            .unwrap();
        assert_eq!((root.object_count, root.relation_count), (3, 2));
    }
    if strong == 48 {
        assert_eq!(episodes(&result).len(), 8);
        assert_eq!(roots(&result).len(), 12);
        assert!(!roots(&result).contains(&900));
        assert!(!episodes(&result).contains(&900));
        assert_episode_order(&result);
        assert_topic_prefix(&result, &episodes(&result));
        assert_topic_prefix(&result, &roots(&result));
        assert_eq!(recency_episodes(&result), roots(&result));
        context.candidate_limits.max_graph_roots = 3;
        let capped = retrieve(&memory, context.clone()).await;
        assert_eq!(roots(&capped).len(), 3);
        assert_eq!(episodes(&capped), roots(&capped));
        assert_episode_order(&capped);
        assert_topic_prefix(&capped, &roots(&capped));
        context.candidate_limits.max_graph_roots = 12;
        context.cue_floors.recency = 1;
        let shared_floor = retrieve(&memory, context.clone()).await;
        assert_eq!(episodes(&shared_floor).len(), 8);
        assert!(episodes(&shared_floor).contains(&900));
        assert_episode_order(&shared_floor);
        assert_topic_prefix(&shared_floor, &episodes(&shared_floor));
        assert!(roots(&shared_floor).contains(&900));
        assert_eq!(
            shared_floor
                .trace
                .as_ref()
                .unwrap()
                .floor_admissions
                .iter()
                .filter(|row| row.object.id == id(900) && row.cue_kind == CueKind::Recency)
                .count(),
            2
        );
        let mut recent_only = RememberWritePlan::new();
        for n in 901..916 {
            recent_only = episode(recent_only, n, 0, 0.0, false, false);
        }
        commit(&memory, recent_only).await;
        let reserved = retrieve(&memory, context).await;
        assert_eq!(episodes(&reserved), episodes(&shared_floor));
        for stage in [
            character_memory::api::types::CueFloorStage::GraphRoots,
            character_memory::api::types::CueFloorStage::Section {
                section: ContextPackSection::RelevantEpisodes,
            },
        ] {
            assert!(reserved
                .trace
                .as_ref()
                .unwrap()
                .floor_admissions
                .iter()
                .any(|row| row.object.id == id(900)
                    && row.cue_kind == CueKind::Recency
                    && row.stage == stage));
        }
    }
    if strong == 2 && !overlap {
        let mut plan = episode(RememberWritePlan::new(), 800, 45, 0.0, false, false);
        plan = belief(plan, 3100, &[900, 800], false);
        commit(&memory, plan).await;
        let additions = retrieve(&memory, query(true, false)).await;
        assert_eq!(episodes(&additions).len(), 4);
        assert!(episodes(&additions).contains(&800));
        assert!(episodes(&additions).contains(&900));
        assert_episode_order(&additions);
        assert!(scores(&additions, 2001).final_score > scores(&additions, 900).final_score);
        assert!(scores(&additions, 900).final_score > scores(&additions, 800).final_score);
        assert_eq!(recency_episodes(&additions), episodes(&additions));
        let mut bounded = query(true, false);
        bounded.section_limits = room(3);
        let bounded = retrieve(&memory, bounded).await;
        assert_eq!(episodes(&bounded).len(), 3);
        assert!(episodes(&bounded).contains(&900));
        assert!(!episodes(&bounded).contains(&800));
        assert_episode_order(&bounded);
        assert!(!recency_episodes(&bounded).contains(&800));
        assert_eq!(
            additions
                .pack
                .derived_memories
                .iter()
                .map(|row| row.memory.id.as_u128())
                .collect::<Vec<_>>(),
            [3100]
        );
        assert!(assignment(&additions, 3100)
            .cue_kinds
            .contains(&CueKind::Recency));
    }
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn recency_strong_spare_witnesses() {
    pressure_witnesses(2, false, false).await;
}

#[tokio::test]
async fn recency_weak_descendant_witnesses() {
    pressure_witnesses(1, true, false).await;
}

#[tokio::test]
async fn recency_saturated_witnesses() {
    pressure_witnesses(48, false, false).await;
}

#[tokio::test]
async fn recency_topic_overlap_witnesses() {
    pressure_witnesses(2, false, true).await;
}

#[tokio::test]
async fn recency_fractional_recorded_time_witnesses() {
    let (memory, root) = open().await;
    let mut plan = episode(
        episode(RememberWritePlan::new(), 900, 0, 0.0, false, false),
        100,
        0,
        0.0,
        false,
        false,
    );
    // The fractional newer occasion loses an ascending-ID comparison.
    for candidate in &mut plan.candidates {
        if let MemoryCandidate::Episode(candidate) = candidate {
            if candidate.draft.id == Some(id(100)) {
                candidate.draft.id = Some(id(1000));
            }
        }
    }
    commit(&memory, plan).await;
    let mut fractional = query(false, false);
    fractional.scene.time += Duration::seconds(1);
    fractional.section_limits = room(1);
    let later = retrieve(&memory, fractional).await;
    assert_eq!(episodes(&later), [1000]);
    let mut exact = query(false, false);
    exact.section_limits = room(1);
    let boundary = retrieve(&memory, exact).await;
    assert_eq!(episodes(&boundary), [900]);
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn recency_stops_at_interpreted_memory_sources() {
    let mut exclusions = Vec::new();
    for topic in [false, true] {
        let (memory, temp) = open().await;
        let mut plan = episode(
            episode(RememberWritePlan::new(), 900, 0, 1.0, false, topic),
            700,
            40,
            0.0,
            false,
            false,
        );
        let mut old = ObservationDraft::new(
            id(700),
            "old violet steam, unrelated to the latest occasion",
        );
        old.id = Some(id(400));
        old.created_at = Some(time() - Duration::days(40));
        old.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        plan = plan.with_candidate(MemoryCandidate::Observation(ObservationCandidate::new(
            old,
            provenance(),
        )));
        plan = link(
            plan,
            ObjectType::Observation,
            400,
            ObjectType::Episode,
            700,
            RelationType::ObservedIn,
        );
        let mut claim = DerivedMemoryDraft::new(
            DerivedType::Claim,
            "An interpretation with old and new sources",
        );
        claim.id = Some(id(300));
        claim.created_at = Some(time());
        claim.updated_at = Some(time());
        claim.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        claim.derived_from_episode_ids = vec![id(900)];
        claim.derived_from_observation_ids = vec![id(400)];
        plan = plan.with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
            claim,
            provenance(),
        )));
        plan = link(
            plan,
            ObjectType::DerivedMemory,
            300,
            ObjectType::Episode,
            900,
            RelationType::DerivedFrom,
        );
        plan = link(
            plan,
            ObjectType::DerivedMemory,
            300,
            ObjectType::Observation,
            400,
            RelationType::DerivedFrom,
        );
        commit(&memory, plan).await;
        let mut context = query(topic, false);
        context.section_limits = room(1);
        context.candidate_limits.max_graph_roots = 1;
        let result = retrieve(&memory, context.clone()).await;
        assert_eq!(episodes(&result), [900]);
        assert_eq!(roots(&result), [900]);
        assert_eq!(result.pack.derived_memories[0].memory.id, id(300));
        exclusions.push(if topic {
            result
                .pack
                .salient_observations
                .iter()
                .any(|row| row.id == id(400))
                && assignment(&result, 400).cue_kinds
                    == std::collections::BTreeSet::from([CueKind::Topic])
        } else {
            result.pack.salient_observations.is_empty()
                && !result
                    .trace
                    .as_ref()
                    .unwrap()
                    .section_assignments
                    .iter()
                    .any(|row| row.object.id == id(400))
        });
        context.include_trace = false;
        assert_eq!(result.pack, memory.retrieve(context).await.unwrap().pack);

        // The same boundary still admits this occasion's own observation and
        // the interpretation resting on that observation.
        let mut own = ObservationDraft::new(id(900), "what happened this evening");
        own.id = Some(id(410));
        own.created_at = Some(time());
        own.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        let mut own_claim = DerivedMemoryDraft::new(DerivedType::Claim, "this evening's meaning");
        own_claim.id = Some(id(310));
        own_claim.created_at = Some(time());
        own_claim.updated_at = Some(time());
        own_claim.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        own_claim.derived_from_observation_ids = vec![id(410)];
        let plan = RememberWritePlan::new()
            .with_candidate(MemoryCandidate::Observation(ObservationCandidate::new(
                own,
                provenance(),
            )))
            .with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
                own_claim,
                provenance(),
            )));
        let plan = link(
            link(
                plan,
                ObjectType::Observation,
                410,
                ObjectType::Episode,
                900,
                RelationType::ObservedIn,
            ),
            ObjectType::DerivedMemory,
            310,
            ObjectType::Observation,
            410,
            RelationType::DerivedFrom,
        );
        commit(&memory, plan).await;
        let mut context = query(topic, false);
        context.section_limits = room(2);
        context.candidate_limits.max_graph_roots = 1;
        let own = retrieve(&memory, context.clone()).await;
        assert_eq!(roots(&own), [900]);
        assert!(own
            .pack
            .salient_observations
            .iter()
            .any(|row| row.id == id(410)));
        assert!(own
            .pack
            .derived_memories
            .iter()
            .any(|row| row.memory.id == id(310)));
        for n in [300, 310, 410] {
            assert!(assignment(&own, n).cue_kinds.contains(&CueKind::Recency));
        }
        if topic {
            assert_eq!(
                assignment(&own, 400).cue_kinds,
                std::collections::BTreeSet::from([CueKind::Topic])
            );
        } else {
            assert_eq!(own.pack.salient_observations.len(), 1);
        }
        context.include_trace = false;
        assert_eq!(own.pack, memory.retrieve(context).await.unwrap().pack);
        memory.close().await.unwrap();
        temp.close().unwrap();
    }

    assert_eq!(
        exclusions,
        [true, true],
        "time-only exclusion and Topic-only provenance"
    );
}

#[tokio::test]
async fn recency_floor_reserves_latest_before_score_fill() {
    let (memory, temp) = open().await;
    commit(
        &memory,
        episode(
            episode(RememberWritePlan::new(), 900, 0, 0.0, false, false),
            100,
            1,
            1.0,
            false,
            false,
        ),
    )
    .await;
    let mut context = query(false, false);
    context.section_limits = room(2);
    context.section_limits.relevant_episodes = 1;
    let zero = retrieve(&memory, context.clone()).await;
    context.cue_floors.recency = 1;
    let reserved = retrieve(&memory, context).await;

    memory.close().await.unwrap();
    temp.close().unwrap();
    assert_eq!(episodes(&zero), [100]);
    assert_eq!(episodes(&reserved), [900]);
    assert!(reserved
        .trace
        .as_ref()
        .unwrap()
        .floor_admissions
        .iter()
        .any(|row| row.object.id == id(900)
            && row.cue_kind == CueKind::Recency
            && matches!(
                row.stage,
                character_memory::api::types::CueFloorStage::Section { .. }
            )));
}

#[tokio::test]
async fn equal_score_recent_occasions_ignore_id_order() {
    let mut recalled_times = Vec::new();
    for reverse in [false, true] {
        let (memory, temp) = open().await;
        let mut plan = RememberWritePlan::new();
        for index in 0..20 {
            let n = if reverse { 2019 - index } else { 1000 + index };
            plan = episode(plan, n, 20 - index as i64, 0.0, false, false);
        }
        commit(&memory, plan).await;
        let result = retrieve(&memory, query(false, false)).await;
        recalled_times.push(
            result
                .pack
                .relevant_episodes
                .iter()
                .map(|episode| episode.scene.time)
                .collect::<Vec<_>>(),
        );
        memory.close().await.unwrap();
        temp.close().unwrap();
    }

    let latest = (1..=8)
        .map(|days| time() - Duration::days(days))
        .collect::<Vec<_>>();
    assert_eq!(recalled_times, [latest.clone(), latest]);
}
