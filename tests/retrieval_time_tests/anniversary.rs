use super::*;

fn ann_scene(text: &str, person: bool) -> Scene {
    let mut value = serde_json::to_value(Scene::at(time().fixed_offset())).unwrap();
    value["time"] = json!(text);
    let mut scene: Scene = serde_json::from_value(value).unwrap();
    if person {
        scene.participants.push(test_support::keyed(7));
    }
    scene
}

fn ann_episode(
    mut plan: RememberWritePlan,
    n: u128,
    text: &str,
    person: bool,
    topic: bool,
) -> RememberWritePlan {
    let mut draft = EpisodeDraft::new(if topic {
        format!("Strong {}", n % 1000)
    } else {
        format!("Anniversary {n}")
    });
    draft.id = Some(id(n));
    draft.created_at = Some(time());
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    draft.scene = Some(ann_scene(text, person));
    draft.salience_score = if n == 900 { 1.0 } else { 0.0 };
    plan = plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
        draft,
        provenance(),
    )));
    if person {
        plan = link(
            plan,
            ObjectType::Episode,
            n,
            ObjectType::Entity,
            7,
            RelationType::Involves,
        );
    }
    if topic {
        plan = indexed(plan, ObjectType::Episode, n);
    }
    plan
}

fn ann_query(text: &str, person: bool, topic: bool, cap: usize) -> RetrievalContext {
    let mut context = query(topic, false).with_scene(ann_scene(text, person));
    context.section_limits = room(cap);
    context
}

fn ann_entity() -> RememberWritePlan {
    let mut draft = EntityDraft::new();
    draft.id = Some(id(7));
    draft.created_at = Some(time());
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    RememberWritePlan::new().with_candidate(MemoryCandidate::Entity(EntityCandidate::new(
        draft,
        provenance(),
    )))
}

async fn ann_reopen(root: &std::path::Path) -> CharacterMemory {
    test_support::open_with_provider(
        test_support::persistent_settings(root),
        "anniversary_roundtrip".to_owned(),
        test_support::TestEmbeddingProvider::new(2, time_embedding),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn anniversary_sources_reserve_only_shared_occasions() {
    let current = "2026-09-21T20:00:00+09:00";
    let (memory, temp) = open().await;
    let mut plan = ann_episode(ann_entity(), 900, "2025-09-21T08:00:00+09:00", true, false);
    plan = ann_episode(plan, 700, "2025-09-21T19:00:00+09:00", false, false);
    plan = ann_episode(plan, 600, "2024-09-21T08:00:00+09:00", false, true);
    plan = ann_episode(plan, 500, "2023-03-02T08:00:00+09:00", true, false);
    plan = belief(plan, 300, &[900, 500], false);
    // These shield the anniversary from the ordinary latest-occasion/recency roads.
    for n in 100..120 {
        plan = ann_episode(
            plan,
            n,
            &format!("2026-09-20T10:{:02}:00+09:00", n - 100),
            true,
            false,
        );
    }
    for n in 2000..2048 {
        plan = ann_episode(plan, n, "2026-08-10T10:00:00+09:00", false, true);
    }
    // Both directions and observation-to-episode semantics point to the same occasion.
    plan = link(
        plan,
        ObjectType::Entity,
        7,
        ObjectType::Episode,
        900,
        RelationType::Involves,
    );
    let mut observed = ObservationDraft::new(id(900), "Anniversary witness");
    observed.id = Some(id(400));
    observed.created_at = Some(time());
    observed.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    plan = plan.with_candidate(MemoryCandidate::Observation(ObservationCandidate::new(
        observed,
        provenance(),
    )));
    plan = link(
        plan,
        ObjectType::Observation,
        400,
        ObjectType::Entity,
        7,
        RelationType::Mentions,
    );
    plan = link(
        plan,
        ObjectType::Observation,
        400,
        ObjectType::Episode,
        900,
        RelationType::ObservedIn,
    );
    commit(&memory, plan).await;
    for (name, person, topic, day, cap) in [
        ("shared-quiet", true, false, current, 8),
        ("shared-pressure", true, true, current, 8),
        ("next-day", true, true, "2026-09-22T20:00:00+09:00", 8),
        ("unshared-quiet", false, false, current, 2),
        ("unshared-pressure", false, true, current, 8),
        (
            "unshared-pressure-control",
            false,
            true,
            "2026-09-22T20:00:00+09:00",
            8,
        ),
    ] {
        let result = retrieve(&memory, ann_query(day, person, topic, cap)).await;
        match name {
            "shared-quiet" => assert!(episodes(&result).contains(&900)),
            "shared-pressure" => {
                assert!(episodes(&result).contains(&900));
                assert_eq!(
                    result
                        .trace
                        .as_ref()
                        .unwrap()
                        .floor_admissions
                        .iter()
                        .filter(|a| a.object.id == id(900) && a.cue_kind == CueKind::DateMatch)
                        .count(),
                    // Participant aboutness already brings the occasion into the section.
                    1
                );
            }
            "next-day" => assert!(ann_dates(&result).is_empty()),
            "unshared-quiet" => {
                assert!(roots(&result).contains(&700));
                assert_eq!(ann_dates(&result), [900, 700]);
                assert!(!result
                    .trace
                    .as_ref()
                    .unwrap()
                    .floor_admissions
                    .iter()
                    .any(|a| a.cue_kind == CueKind::DateMatch));
            }
            _ => assert_eq!(episodes(&result), (2000..2008).collect::<Vec<_>>()),
        }
        assert!(!ann_dates(&result).contains(&500));
    }
    for person in [false, true] {
        let mut context = ann_query(current, person, true, 1);
        context.candidate_limits.max_graph_roots = 1;
        context.cue_floors.participant = 0;
        let result = retrieve(&memory, context).await;
        assert_eq!(roots(&result), [if person { 900 } else { 2000 }]);
        if person {
            assert!(result
                .pack
                .salient_observations
                .iter()
                .any(|o| o.id == id(400)));
            assert!(result
                .pack
                .derived_memories
                .iter()
                .any(|m| m.memory.id == id(300)));
            assert!(!episodes(&result).contains(&500));
        } else {
            assert!(result
                .trace
                .as_ref()
                .unwrap()
                .floor_admissions
                .iter()
                .all(|a| a.cue_kind != CueKind::DateMatch));
        }
    }
    for floor in [0, 1, 2, 3] {
        let mut context = ann_query(current, true, false, 3);
        context.cue_floors.date_match = floor;
        let result = retrieve(&memory, context).await;
        assert_eq!(
            ann_dates(&result).len(),
            3,
            "contribution does not shrink with the reservation"
        );
    }
    let mut context = ann_query(current, true, false, 2);
    context.cue_floors.date_match = 2;
    context.time_range = Some(TimeRange {
        start: "2026-08-10T00:00:00Z".parse().unwrap(),
        end: "2026-08-10T23:59:59Z".parse().unwrap(),
    });
    retrieve(&memory, context.clone()).await;
    context.cue_floors.participant = 0;
    context.cue_floors.date_match = 1;
    // Isolate the two direct time contributions from participant descendants.
    context.graph_limits.max_depth = 0;
    let together = memory.retrieve(context).await.unwrap();
    assert_eq!(episodes(&together), [900, 2000]);
    let mut overlapping = ann_query(current, false, true, 1);
    overlapping.candidate_limits.max_graph_roots = 1;
    overlapping.time_range = Some(TimeRange {
        start: "2025-09-20T00:00:00Z".parse().unwrap(),
        end: "2025-09-21T23:00:00Z".parse().unwrap(),
    });
    let overlapping = memory.retrieve(overlapping).await.unwrap();
    assert_eq!(roots(&overlapping), [700]);
    assert!(overlapping
        .trace
        .as_ref()
        .unwrap()
        .floor_admissions
        .iter()
        .any(|a| a.object.id == id(700) && a.cue_kind == CueKind::DateMatch));
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::Episode(id(900)),
            "anniversary eligibility before cut",
        ))
        .await
        .unwrap();
    let forgotten = retrieve(&memory, ann_query(current, true, false, 2)).await;
    assert_eq!(ann_dates(&forgotten), [700, 600]);
    assert!(!roots(&forgotten).contains(&900));
    let mut by_topic = ann_query(current, false, false, 1);
    by_topic.topic = Some("anniversary topic".to_owned());
    assert_eq!(episodes(&memory.retrieve(by_topic).await.unwrap()), [600]);
    memory.close().await.unwrap();
    temp.close().unwrap();

    let (memory, temp) = open().await;
    let mut plan = ann_episode(
        RememberWritePlan::new(),
        900,
        "2025-09-21T08:00:00+09:00",
        false,
        false,
    );
    plan = ann_episode(plan, 800, "2026-09-21T08:00:00+09:00", false, false);
    plan = ann_episode(plan, 700, "2026-09-21T00:30:00+14:00", false, false);
    plan = ann_episode(plan, 600, "2024-02-29T08:00:00+09:00", false, false);
    commit(&memory, plan).await;
    for (name, day) in [
        ("local-anniversary", current),
        ("local-next-day", "2026-09-22T20:00:00+09:00"),
        ("leap", "2028-02-29T20:00:00+09:00"),
        ("nonleap-feb28", "2027-02-28T20:00:00+09:00"),
        ("nonleap-mar1", "2027-03-01T20:00:00+09:00"),
    ] {
        let result = retrieve(&memory, ann_query(day, false, false, 8)).await;
        assert_eq!(
            ann_dates(&result),
            match name {
                "local-anniversary" => vec![900],
                "leap" => vec![600],
                _ => vec![],
            }
        );
    }
    memory.close().await.unwrap();
    temp.close().unwrap();
}

fn ann_dates(result: &RetrieveOutcome) -> Vec<u128> {
    result
        .trace
        .as_ref()
        .unwrap()
        .section_assignments
        .iter()
        .filter(|a| {
            a.object.object_type == ObjectType::Episode && a.cue_kinds.contains(&CueKind::DateMatch)
        })
        .map(|a| a.object.id.as_u128())
        .collect()
}

#[tokio::test]
async fn scene_offset_is_persisted_content() {
    let root = tempfile::tempdir().unwrap();
    let memory = ann_reopen(root.path()).await;
    let plan = ann_episode(
        RememberWritePlan::new(),
        900,
        "2025-09-21T08:00:00.123456789+09:00",
        false,
        false,
    );
    commit(&memory, plan.clone()).await;
    memory.close().await.unwrap();
    let memory = ann_reopen(root.path()).await;
    commit(&memory, plan).await;
    let collision = memory
        .commit(
            ann_episode(
                RememberWritePlan::new(),
                900,
                "2025-09-20T23:00:00.123456789Z",
                false,
                false,
            ),
            CommitOptions::default(),
        )
        .await;
    assert!(
        matches!(collision, Err(CustomError::DeterministicIdCollision { object }) if object.id == id(900))
    );
    let context = ann_query("2026-09-21T20:00:00+09:00", false, false, 8);
    let result = memory.retrieve(context.clone()).await.unwrap();
    let saved = &result.pack.relevant_episodes[0];
    assert_eq!(
        saved
            .scene
            .time
            .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
        "2025-09-21T08:00:00.123456789+09:00"
    );
    assert_eq!(
        saved.scene.time.date_naive(),
        "2025-09-21".parse::<chrono::NaiveDate>().unwrap()
    );
    assert_eq!(ann_dates(&result), [900]);
    let mut no_trace = context;
    no_trace.include_trace = false;
    assert_eq!(memory.retrieve(no_trace).await.unwrap().pack, result.pack);
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn anniversary_reservation_needs_a_resolved_notion() {
    let (memory, temp) = open().await;
    let mut plan = ann_episode(ann_entity(), 900, "2025-09-21T08:00:00+09:00", true, false);
    plan = ann_episode(plan, 2000, "2026-08-10T10:00:00Z", false, true);
    let mut other = EntityDraft::new();
    other.id = Some(id(8));
    other.created_at = Some(time());
    other.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    plan = plan.with_candidate(MemoryCandidate::Entity(EntityCandidate::new(
        other,
        provenance(),
    )));
    for n in [7, 8] {
        let mut draft = DerivedMemoryDraft::new(DerivedType::Claim, "A supplied name");
        draft.id = Some(id(100 + n));
        draft.created_at = Some(time());
        draft.updated_at = Some(time());
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        draft.given_by_application = true;
        draft.entity_ids = vec![id(n)];
        draft.assertions.push(BeliefAssertion {
            subject: id(n),
            predicate: BeliefPredicate::KnownAs {
                name: "Twin".to_owned(),
            },
        });
        if n == 7 {
            draft.assertions.push(BeliefAssertion {
                subject: id(n),
                predicate: BeliefPredicate::KnownAs {
                    name: "Only".to_owned(),
                },
            });
        }
        plan = plan.with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
            draft,
            provenance(),
        )));
    }
    commit(&memory, plan).await;
    for name in ["Twin", "Only", "Unknown"] {
        let mut context = ann_query("2026-09-21T20:00:00+09:00", false, true, 1);
        context.scene.participants.push(SceneParticipant {
            name: Some(name.to_owned()),
            ..Default::default()
        });
        context.candidate_limits.max_graph_roots = 1;
        context.cue_floors.participant = 0;
        let result = memory.retrieve(context).await.unwrap();
        if name == "Only" {
            assert!(
                matches!(result.scene_references[0].resolution, SceneReferenceResolution::Resolved { notion_id } if notion_id == id(7))
            );
        }
        assert_eq!(
            roots(&result),
            [if name == "Only" { 900 } else { 2000 }],
            "{name}"
        );
        if name == "Twin" {
            assert!(
                matches!(&result.scene_references[0].resolution,SceneReferenceResolution::Ambiguous { notion_ids } if notion_ids==&vec![id(7),id(8)])
            );
            assert!(!result
                .trace
                .as_ref()
                .unwrap()
                .floor_admissions
                .iter()
                .any(|a| a.cue_kind == CueKind::DateMatch));
        }
    }
    memory.close().await.unwrap();
    temp.close().unwrap();
}

#[tokio::test]
async fn anniversary_sharing_requires_presence_in_either_direction() {
    for (mentions, reverse) in [(false, false), (false, true), (true, false), (true, true)] {
        let (memory, temp) = open().await;
        // No participant literal on either occasion: sharing comes from the links.
        let mut plan = ann_episode(ann_entity(), 900, "2025-09-21T08:00:00+09:00", false, false);
        plan = ann_episode(plan, 700, "2025-09-21T19:00:00+09:00", false, false);
        plan = ann_episode(plan, 2000, "2026-08-10T10:00:00Z", false, true);
        let (kind, n, relation) = if mentions {
            let mut draft = ObservationDraft::new(id(900), "Shared occasion");
            draft.id = Some(id(400));
            draft.created_at = Some(time());
            draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
            plan = plan.with_candidate(MemoryCandidate::Observation(ObservationCandidate::new(
                draft,
                provenance(),
            )));
            (ObjectType::Observation, 400, RelationType::Mentions)
        } else {
            (ObjectType::Episode, 900, RelationType::Involves)
        };
        plan = if reverse {
            link(plan, ObjectType::Entity, 7, kind, n, relation)
        } else {
            link(plan, kind, n, ObjectType::Entity, 7, relation)
        };
        commit(&memory, plan).await;
        let mut context = ann_query("2026-09-21T20:00:00+09:00", true, true, 1);
        context.cue_floors.participant = 0;
        context.candidate_limits.max_graph_roots = 1;
        let shared = memory.retrieve(context.clone()).await.unwrap();
        assert_eq!(
            roots(&shared),
            [if mentions { 2000 } else { 900 }],
            "mentions={mentions} reverse={reverse}"
        );
        if mentions {
            memory
                .forget(ForgetMemoryDraft::suppress(
                    LifecycleTargetRef::Observation(id(400)),
                    "Forget the remark, which is not presence",
                ))
                .await
                .unwrap();
            assert_eq!(
                roots(&memory.retrieve(context.clone()).await.unwrap()),
                [2000]
            );
            context.lifecycle_policy.include_suppressed = true;
            // Restoring a mention does not turn it into a shared occasion.
            assert_eq!(roots(&memory.retrieve(context).await.unwrap()), [2000]);
        }
        memory.close().await.unwrap();
        temp.close().unwrap();
    }
}

#[tokio::test]
async fn unshared_anniversary_can_fill_spare_room_with_its_reminder_leaf() {
    let (memory, temp) = open().await;
    let mut plan = ann_episode(ann_entity(), 900, "2025-09-21T08:00:00+09:00", true, false);
    plan = ann_episode(plan, 500, "2023-03-02T08:00:00+09:00", true, false);
    plan = belief(plan, 300, &[900, 500], false);
    for n in 100..120 {
        plan = ann_episode(plan, n, "2026-09-20T10:00:00+09:00", false, false);
    }
    for n in 2000..2048 {
        plan = ann_episode(plan, n, "2026-08-10T10:00:00Z", false, true);
    }
    commit(&memory, plan).await;
    let current = "2026-09-21T20:00:00+09:00";
    let quiet = memory
        .retrieve(ann_query(current, false, false, 8))
        .await
        .unwrap();
    assert!(episodes(&quiet).contains(&900));
    assert!(quiet
        .pack
        .derived_memories
        .iter()
        .any(|m| m.memory.id == id(300)));
    assert!(!episodes(&quiet).contains(&500));
    assert_eq!(scores(&quiet, 900).cue_score, Some(0.0));
    assert!(!quiet
        .trace
        .as_ref()
        .unwrap()
        .floor_admissions
        .iter()
        .any(|a| a.cue_kind == CueKind::DateMatch));
    let pressure = memory
        .retrieve(ann_query(current, false, true, 8))
        .await
        .unwrap();
    let control = memory
        .retrieve(ann_query("2026-09-22T20:00:00+09:00", false, true, 8))
        .await
        .unwrap();
    assert_eq!(pressure.pack, control.pack);
    assert!(!episodes(&pressure).contains(&900));
    memory.close().await.unwrap();
    temp.close().unwrap();
}
