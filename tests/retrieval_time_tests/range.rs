use super::*;

fn range_episode(plan: RememberWritePlan, n: u128, at: DateTime<Utc>) -> RememberWritePlan {
    let mut plan = episode(plan, n, 0, 0.0, false, false);
    for candidate in &mut plan.candidates {
        if let MemoryCandidate::Episode(candidate) = candidate {
            if candidate.draft.id == Some(id(n)) {
                candidate.draft.scene.as_mut().unwrap().time = at.fixed_offset();
            }
        }
    }
    indexed(plan, ObjectType::Episode, n)
}

fn range_fixture(reverse: bool) -> RememberWritePlan {
    let mut plan = RememberWritePlan::new();
    let tuesday = time() - Duration::days(6) - Duration::hours(18);
    let today = time() - Duration::hours(18);
    for (index, hours) in [0, 6, 12, 18, 23].into_iter().enumerate() {
        let n = if reverse { 204 - index } else { 100 + index };
        plan = range_episode(plan, n as u128, tuesday + Duration::hours(hours));
    }
    for (index, hours) in [0, 6, 12, 17, 18].into_iter().enumerate() {
        let n = if reverse { 604 - index } else { 500 + index };
        plan = range_episode(plan, n as u128, today + Duration::hours(hours));
    }
    for index in 0..48 {
        plan = episode(plan, 2000 + index, 30 + index as i64, 0.0, false, true);
    }
    plan
}

fn date_match_episodes(result: &RetrieveOutcome) -> Vec<u128> {
    result
        .trace
        .as_ref()
        .unwrap()
        .section_assignments
        .iter()
        .filter(|row| {
            row.object.object_type == ObjectType::Episode
                && row.cue_kinds.contains(&CueKind::DateMatch)
        })
        .map(|row| row.object.id.as_u128())
        .collect()
}

#[tokio::test]
async fn time_range_replaces_recency_window() {
    let mut selected = Vec::new();
    for reverse in [false, true] {
        let (memory, temp) = open().await;
        commit(&memory, range_fixture(reverse)).await;
        let start = time() - Duration::days(6) - Duration::hours(18);
        let context = query(false, false).with_time_range(start, start + Duration::hours(23));
        let result = retrieve(&memory, context).await;
        selected.push(episodes(&result));
        memory.close().await.unwrap();
        temp.close().unwrap();
    }

    assert_eq!(
        selected,
        [vec![104, 103, 102, 101, 100], vec![200, 201, 202, 203, 204],]
    );
}

#[tokio::test]
async fn time_range_reserves_the_requested_day_under_topic_pressure() {
    for reverse in [false, true] {
        let (memory, temp) = open().await;
        commit(&memory, range_fixture(reverse)).await;
        let tuesday = time() - Duration::days(6) - Duration::hours(18);
        for (topic, start, end, expected) in [
            (
                "what happened last Tuesday",
                tuesday,
                tuesday + Duration::hours(23),
                if reverse { [200, 201] } else { [104, 103] },
            ),
            (
                "what happened today",
                time() - Duration::hours(18),
                time(),
                if reverse { [600, 601] } else { [504, 503] },
            ),
        ] {
            let mut context = query(true, false).with_time_range(start, end);
            context.topic = Some(topic.to_owned());
            context.cue_floors.date_match = 0;
            let zero = retrieve(&memory, context.clone()).await;
            assert_eq!(episodes(&zero), (2000..2008).collect::<Vec<_>>());
            assert!(date_match_episodes(&zero).is_empty());
            context.cue_floors.date_match = 2;
            let result = retrieve(&memory, context.clone()).await;
            assert_eq!(result.time_range, context.time_range);
            assert_eq!(date_match_episodes(&result), expected);
            assert_eq!(
                episodes(&result),
                (2000..2006).chain(expected).collect::<Vec<_>>()
            );
            assert_eq!(
                result.trace.as_ref().unwrap().time_range_has_more,
                Some(false)
            );
            for n in expected {
                assert!(roots(&result).contains(&n));
                assert_eq!(scores(&result, n).cue_score, Some(0.0));
            }
            context.include_trace = false;
            let untraced = memory.retrieve(context).await.unwrap();
            assert_eq!(untraced.pack, result.pack);
            assert_eq!(untraced.time_range, result.time_range);
        }
        memory.close().await.unwrap();
        temp.close().unwrap();
    }
}

#[tokio::test]
async fn time_range_preserves_other_cues_when_there_is_room() {
    for reverse in [false, true] {
        let (memory, temp) = open().await;
        commit(&memory, range_fixture(reverse)).await;
        let mut context = query(true, false);
        context.section_limits = room(24);
        context.candidate_limits.max_vector_candidates = 2;
        context.candidate_limits.max_graph_roots = 64;
        let without = retrieve(&memory, context.clone()).await;
        let start = time() - Duration::days(6) - Duration::hours(18);
        context = context.with_time_range(start, start + Duration::hours(23));
        context.cue_floors.date_match = 2;
        let with = retrieve(&memory, context).await;
        let dates = if reverse {
            [200, 201, 202, 203, 204]
        } else {
            [104, 103, 102, 101, 100]
        };
        assert_eq!(
            episodes(&with),
            (2000..2002).chain(dates).collect::<Vec<_>>()
        );
        let mut reordered = without.pack.clone();
        reordered
            .relevant_episodes
            .retain(|episode| episodes(&with).contains(&episode.id.as_u128()));
        reordered.relevant_episodes.sort_by_key(|episode| {
            episodes(&with)
                .iter()
                .position(|&n| n == episode.id.as_u128())
                .unwrap()
        });
        assert_eq!(with.pack, reordered);
        for n in episodes(&with) {
            assert_eq!(scores(&with, n), scores(&without, n));
        }
        assert_eq!(date_match_episodes(&with), dates);
        memory.close().await.unwrap();
        temp.close().unwrap();
    }
}

#[tokio::test]
async fn time_range_overlap_keeps_standing_and_reminders_stay_on_the_occasion() {
    for topic in [false, true] {
        let (memory, temp) = open().await;
        commit(&memory, description_fixture(true)).await;
        let mut context = query(topic, false);
        context.section_limits = room(2);
        context.candidate_limits.max_graph_roots = 1;
        let without = retrieve(&memory, context.clone()).await;
        context = context.with_time_range(time(), time());
        let with = retrieve(&memory, context).await;
        if topic {
            assert_eq!(with.pack, without.pack);
        } else {
            assert_eq!(roots(&without), [700]);
        }
        assert_eq!(roots(&with), [900]);
        assert_eq!(date_match_episodes(&with), [900]);
        if topic {
            assert_eq!(scores(&with, 900), scores(&without, 900));
        } else {
            assert_eq!(scores(&with, 900).cue_score, Some(0.0));
        }
        assert_eq!(
            with.trace.as_ref().unwrap().time_range_has_more,
            Some(false)
        );
        assert!(assignment(&with, 300)
            .cue_kinds
            .contains(&CueKind::DateMatch));
        if topic {
            assert_eq!(episodes(&with).len(), 2);
            assert_eq!(
                assignment(&with, 700).cue_kinds,
                std::collections::BTreeSet::from([CueKind::Topic])
            );
            assert!(assignment(&with, 900).cue_kinds.contains(&CueKind::Topic));
            assert_eq!(scores(&with, 700), scores(&without, 700));
        } else {
            assert_eq!(episodes(&with), [900]);
        }
        memory.close().await.unwrap();
        temp.close().unwrap();
    }
}

#[tokio::test]
async fn time_range_uses_both_ends_without_the_scene_reference_cut() {
    let (memory, temp) = open().await;
    let start = time() + Duration::days(1);
    let end = start + Duration::milliseconds(125);
    let mut plan = RememberWritePlan::new();
    for (n, at) in [
        (99, start - Duration::milliseconds(1)),
        (100, start),
        (900, end),
        (98, end + Duration::milliseconds(1)),
    ] {
        plan = range_episode(plan, n, at);
    }
    commit(&memory, plan).await;
    for (range_start, range_end, expected) in [
        (start, end, vec![900, 100]),
        (start, start, vec![100]),
        (end, start, vec![]),
        (time(), time(), vec![]),
    ] {
        let mut context = query(false, false).with_time_range(range_start, range_end);
        context.cue_floors.date_match = 2;
        let with = retrieve(&memory, context.clone()).await;
        assert_eq!(with.time_range, context.time_range);
        assert_eq!(date_match_episodes(&with), expected);
        assert_eq!(episodes(&with), expected);
        assert_eq!(
            with.trace.as_ref().unwrap().time_range_has_more,
            Some(false)
        );
        context.include_trace = false;
        let untraced = memory.retrieve(context).await.unwrap();
        assert_eq!(untraced.pack, with.pack);
        assert_eq!(untraced.time_range, with.time_range);
    }
    memory.close().await.unwrap();
    temp.close().unwrap();
}

#[tokio::test]
async fn time_range_section_cap_bounds_contribution_after_lifecycle_filtering() {
    let (memory, temp) = open().await;
    let start = time() + Duration::days(1);
    let end = start + Duration::hours(2);
    let mut plan = RememberWritePlan::new();
    for (n, hours) in [(100, 0), (200, 1), (300, 2)] {
        plan = range_episode(plan, n, start + Duration::hours(hours));
    }
    commit(&memory, plan).await;
    for floor in [0, 1, 2, 3, 7] {
        let mut context = query(false, false).with_time_range(start, end);
        context.cue_floors.date_match = floor;
        let result = retrieve(&memory, context).await;
        assert_eq!(episodes(&result), [300, 200, 100]);
        assert_eq!(date_match_episodes(&result), [300, 200, 100]);
        assert_eq!(
            result.trace.as_ref().unwrap().time_range_has_more,
            Some(false)
        );
    }
    for floor in [0, 1, 7] {
        let mut context = query(false, false).with_time_range(start, end);
        context.section_limits = room(2);
        context.cue_floors.date_match = floor;
        let result = retrieve(&memory, context).await;
        assert_eq!(roots(&result), [300, 200]);
        assert_eq!(episodes(&result), [300, 200]);
        assert_eq!(date_match_episodes(&result), [300, 200]);
        assert_eq!(
            result.trace.as_ref().unwrap().time_range_has_more,
            Some(true)
        );
    }
    for (limits, expected_roots, expected_pack, more) in [
        (room(0), vec![], vec![], true),
        (
            ContinuitySectionLimits {
                relevant_episodes: 1,
                ..room(3)
            },
            vec![300, 200, 100],
            vec![300],
            false,
        ),
    ] {
        let mut context = query(false, false).with_time_range(start, end);
        context.section_limits = limits;
        let result = retrieve(&memory, context).await;
        assert_eq!(roots(&result), expected_roots);
        assert_eq!(episodes(&result), expected_pack);
        assert_eq!(
            result.trace.as_ref().unwrap().time_range_has_more,
            Some(more)
        );
    }
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::Episode(id(300)),
            "range eligibility",
        ))
        .await
        .unwrap();
    for (cap, include_suppressed) in [(16, false), (16, true), (2, false), (2, true)] {
        let mut context = query(false, false).with_time_range(start, end);
        if cap == 2 {
            context.section_limits = room(cap);
        }
        context.cue_floors.date_match = 2;
        context.lifecycle_policy.include_suppressed = include_suppressed;
        let result = retrieve(&memory, context).await;
        assert_eq!(
            episodes(&result),
            if include_suppressed {
                vec![300, 200, 100][..cap.min(3)].to_vec()
            } else {
                vec![200, 100]
            }
        );
        assert_eq!(
            result.trace.as_ref().unwrap().time_range_has_more,
            Some(include_suppressed && cap == 2)
        );
    }
    memory.close().await.unwrap();
    temp.close().unwrap();
}
