use character_memory::{
    ActivityRef, CharacterMemory, CommitOptions, DerivedMemoryDraft, DerivedType, EntityDraft,
    EpisodeDraft, ForgetMemoryDraft, LifecycleFilterReason, LifecycleTargetRef, MemoryId,
    MemoryLinkDraft, MemoryThreadDraft, ObjectType, RelationType, RememberInput,
    RememberPlanDefaults, RetentionState, RetrievalContext, RetrieveOutcome, Scene,
    SceneParticipant,
};
use chrono::{DateTime, Utc};

#[path = "support/mod.rs"]
pub mod test_support;

fn id(n: u128) -> MemoryId {
    MemoryId::from_u128(n)
}
fn at(n: i64) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-09-21T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
        + chrono::Duration::minutes(n)
}
async fn commit(memory: &CharacterMemory, input: RememberInput) {
    let defaults = RememberPlanDefaults::fixed(&input.content, at(0));
    memory
        .commit(
            input.prepare_write_plan(&defaults),
            CommitOptions::default(),
        )
        .await
        .unwrap();
}
fn episode(n: u128, scene: Scene) -> EpisodeDraft {
    let mut draft = EpisodeDraft::new(format!("experience {n}"));
    draft.id = Some(id(n));
    draft.scene = Some(scene);
    draft.created_at = Some(at(n as i64));
    draft
}
fn derived(
    n: u128,
    kind: DerivedType,
    text: &str,
    source: u128,
    minute: i64,
) -> DerivedMemoryDraft {
    let mut draft = DerivedMemoryDraft::new(kind, text).with_source_episode(id(source));
    draft.id = Some(id(n));
    draft.created_at = Some(at(minute));
    draft.updated_at = Some(at(minute));
    draft
}

async fn fixture(resolvers_share_state: bool) -> (CharacterMemory, tempfile::TempDir) {
    let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
    let mut person = EntityDraft::new();
    person.id = Some(id(501));
    let mut thread = MemoryThreadDraft::new("shared work", "shared work");
    thread.id = Some(id(601));
    let mut original_scene = Scene::at(at(10));
    original_scene.setting.key = Some("workshop".into());
    original_scene
        .custom_values
        .insert("project".into(), "42".into());
    let originals = [
        (
            301,
            DerivedType::OpenLoop,
            "quasar hatch calibration pending",
        ),
        (302, DerivedType::Commitment, "meteor delivery promise"),
    ];
    let mut input = RememberInput::new("prior work")
        .with_entity(person)
        .with_memory_thread(thread)
        .with_episode(episode(101, original_scene));
    for (n, kind, text) in originals {
        let mut draft = derived(n, kind, text, 101, 10);
        draft.entity_ids.push(id(501));
        draft.thread_ids.push(id(601));
        draft.salience_score = 1.0;
        input = input.with_derived_memory(draft);
    }
    commit(&memory, input).await;
    let mut input =
        RememberInput::new("later results").with_episode(episode(102, Scene::at(at(20))));
    for (n, text) in [
        (401, "calibration completed successfully"),
        (402, "delivered all supplies"),
    ] {
        let mut resolver = derived(n, DerivedType::Claim, text, 102, 20);
        if resolvers_share_state {
            resolver.entity_ids.push(id(501));
            resolver.thread_ids.push(id(601));
        }
        input = input.with_derived_memory(resolver);
    }
    commit(&memory, input).await;
    for (n, source, target, relation) in [
        (701, 401, 301, RelationType::Resolves),
        (702, 402, 302, RelationType::FulfillsCommitment),
        (703, 402, 301, RelationType::Resolves),
        (704, 401, 301, RelationType::Resolves),
    ] {
        let mut link = MemoryLinkDraft::new(
            ObjectType::DerivedMemory,
            id(source),
            relation,
            ObjectType::DerivedMemory,
            id(target),
        );
        link.id = Some(id(n));
        link.created_at = Some(at(30));
        memory.link(link).await.unwrap();
    }
    (memory, root)
}

fn request(route: &str) -> RetrievalContext {
    let mut scene = Scene::at(at(40));
    if route == "named" || route == "mixed" {
        scene.participants.push(SceneParticipant {
            key: Some(id(501)),
            ..Default::default()
        });
    }
    if route == "setting" {
        scene.setting.key = Some("workshop".into());
    }
    if route == "custom" {
        scene.custom_values.insert("project".into(), "42".into());
    }
    let mut request = RetrievalContext::default().with_scene(scene);
    request.graph_limits.max_depth =
        u8::from(matches!(route, "named" | "mixed" | "thread_expansion"));
    if matches!(route, "thread" | "thread_expansion") {
        request.activity = Some(ActivityRef::Thread(id(601)));
    }
    if matches!(route, "topic_loop" | "mixed") {
        request.topic = Some("quasar hatch calibration pending".into());
    }
    if route == "topic_commitment" {
        request.topic = Some("meteor delivery promise".into());
    }
    if route == "explicit_loop" {
        request.activity = Some(ActivityRef::OpenLoop(id(301)));
    }
    // Resolution evidence must not depend on traversing resolving links.
    request.graph_limits.allowed_relation_types =
        vec![RelationType::About, RelationType::PartOfThread];
    request
}

fn omitted(result: &RetrieveOutcome, target: u128, reason: LifecycleFilterReason) {
    assert!(
        result
            .trace
            .as_ref()
            .unwrap()
            .lifecycle_filter_decisions
            .iter()
            .any(|entry| entry.object.id == id(target) && entry.reason == reason),
        "missing {reason:?} for {target}: {:?}",
        result.trace.as_ref().unwrap().lifecycle_filter_decisions
    );
}

#[tokio::test]
async fn resolution_leaves_all_state_routes_before_their_caps() {
    let (memory, root) = fixture(false).await;
    let mut input = RememberInput::new("remaining work");
    for (n, kind) in [(303, DerivedType::OpenLoop), (304, DerivedType::Commitment)] {
        let mut live = derived(n, kind, "still pending", 101, 5);
        live.entity_ids.push(id(501));
        live.thread_ids.push(id(601));
        live.salience_score = 0.1;
        input = input.with_derived_memory(live);
    }
    commit(&memory, input).await;
    for route in ["named", "thread", "thread_expansion", "setting", "custom"] {
        let mut request = request(route).with_trace();
        request.candidate_limits.max_graph_roots = if route.starts_with("thread") { 3 } else { 2 };
        request.graph_limits.max_fanout_per_node = 2;
        let result = memory.retrieve(request).await.unwrap();
        assert_eq!(
            result
                .pack
                .open_loops
                .iter()
                .map(|item| item.memory.id)
                .collect::<Vec<_>>(),
            vec![id(303)],
            "{route}"
        );
        assert_eq!(
            result
                .pack
                .commitments
                .iter()
                .map(|item| item.memory.id)
                .collect::<Vec<_>>(),
            vec![id(304)],
            "{route}"
        );
        assert!(result.pack.open_loops[0].resolved_by.is_empty());
        for target in [301, 302] {
            omitted(&result, target, LifecycleFilterReason::ResolvedOmitted);
        }
    }
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn thread_resolution_preserves_other_object_types_with_the_same_id() {
    let (memory, root) = fixture(false).await;
    let mut observation =
        character_memory::ObservationDraft::new(id(101), "independent observed fact");
    observation.id = Some(id(301));
    observation.created_at = Some(at(11));
    commit(
        &memory,
        RememberInput::new("observation thread membership")
            .with_thread_id(id(601))
            .with_observation(observation),
    )
    .await;
    for include_trace in [false, true] {
        let mut context = request("thread_expansion");
        context.include_trace = include_trace;
        context.graph_limits.allowed_relation_types = vec![RelationType::PartOfThread];
        let result = memory.retrieve(context).await.unwrap();
        assert!(
            result
                .pack
                .salient_observations
                .iter()
                .any(|item| item.id == id(301)),
            "resolution of DerivedMemory 301 must not suppress Observation 301"
        );
        assert!(result.pack.open_loops.is_empty());
        if let Some(trace) = result.trace {
            assert!(trace.lifecycle_filter_decisions.iter().any(|entry| {
                entry.object.object_type == ObjectType::DerivedMemory
                    && entry.object.id == id(301)
                    && entry.reason == LifecycleFilterReason::ResolvedOmitted
            }));
        }
    }
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn recall_names_resolvers_without_trace_even_after_resolvers_are_suppressed() {
    let (memory, root) = fixture(false).await;
    for suppress_resolvers in [false, true] {
        if suppress_resolvers {
            for resolver in [401, 402] {
                memory
                    .forget(ForgetMemoryDraft::suppress(
                        LifecycleTargetRef::derived_memory(id(resolver)),
                        "forget result",
                    ))
                    .await
                    .unwrap();
            }
            for route in ["named", "thread", "setting", "custom"] {
                let result = memory.retrieve(request(route).with_trace()).await.unwrap();
                assert!(result.pack.open_loops.is_empty());
                assert!(result.pack.commitments.is_empty());
                for target in [301, 302] {
                    omitted(&result, target, LifecycleFilterReason::ResolvedOmitted);
                }
            }
        }
        for route in ["topic_loop", "topic_commitment", "mixed", "explicit_loop"] {
            let result = memory.retrieve(request(route)).await.unwrap();
            assert!(result.trace.is_none());
            let item = if route == "topic_commitment" {
                result
                    .pack
                    .commitments
                    .iter()
                    .find(|item| item.memory.id == id(302))
                    .unwrap()
            } else {
                result
                    .pack
                    .open_loops
                    .iter()
                    .find(|item| item.memory.id == id(301))
                    .unwrap()
            };
            let expected = if route == "topic_commitment" {
                vec![id(402)]
            } else {
                vec![id(401), id(402)]
            };
            assert_eq!(
                item.resolved_by, expected,
                "{route}, suppressed={suppress_resolvers}"
            );
            assert_eq!(item.memory.retention_state, RetentionState::Active);
            assert!(item.memory.supersedes.is_empty());
            assert_eq!(item.source_episode_ids, vec![id(101)]);
            let json = serde_json::to_value(item).unwrap();
            assert_eq!(json["resolved_by"], serde_json::json!(expected));
        }
    }
    // A nonstate edge from the same named participant can still recall it.
    memory
        .link(MemoryLinkDraft::new(
            ObjectType::Entity,
            id(501),
            RelationType::AssociatedWith,
            ObjectType::DerivedMemory,
            id(301),
        ))
        .await
        .unwrap();
    let mut request = request("named");
    request
        .graph_limits
        .allowed_relation_types
        .push(RelationType::AssociatedWith);
    let result = memory.retrieve(request).await.unwrap();
    assert!(result.trace.is_none());
    assert_eq!(result.pack.open_loops[0].memory.id, id(301));
    assert_eq!(
        result.pack.open_loops[0].resolved_by,
        vec![id(401), id(402)]
    );
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn resolution_does_not_override_existing_lifecycle_omission_reasons() {
    let (memory, root) = fixture(false).await;
    let mut replacement = derived(
        405,
        DerivedType::Correction,
        "revised understanding",
        102,
        35,
    );
    replacement.supersedes.push(id(301));
    commit(
        &memory,
        RememberInput::new("correction").with_derived_memory(replacement),
    )
    .await;
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::derived_memory(id(301)),
            "forget original",
        ))
        .await
        .unwrap();
    for include_suppressed in [false, true] {
        for include_superseded in [false, true] {
            let expected = if !include_suppressed {
                LifecycleFilterReason::SuppressedOmitted
            } else if !include_superseded {
                LifecycleFilterReason::SupersededOmitted
            } else {
                LifecycleFilterReason::ResolvedOmitted
            };
            for route in ["named", "setting", "custom"] {
                let mut request = request(route).with_trace();
                request.lifecycle_policy.include_suppressed = include_suppressed;
                request.lifecycle_policy.include_superseded = include_superseded;
                let result = memory.retrieve(request).await.unwrap();
                assert!(result.pack.open_loops.is_empty());
                omitted(&result, 301, expected);
            }
        }
    }
    // Flags still allow explicit recall; resolution never becomes a global filter.
    let mut request = request("explicit_loop");
    request.lifecycle_policy.include_suppressed = true;
    request.lifecycle_policy.include_superseded = true;
    let result = memory.retrieve(request).await.unwrap();
    assert_eq!(
        result.pack.open_loops[0].resolved_by,
        vec![id(401), id(402)]
    );
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn thread_forget_still_cascades_to_resolved_members() {
    let (memory, root) = fixture(false).await;
    let mut forget =
        ForgetMemoryDraft::suppress(LifecycleTargetRef::memory_thread(id(601)), "forget work");
    forget.cascade_policy.apply_to_thread_members = true;
    let outcome = memory.forget(forget).await.unwrap();
    for target in [301, 302] {
        assert!(outcome
            .graph_mutated_object_ids
            .iter()
            .any(
                |reference| reference.object_type == ObjectType::DerivedMemory
                    && reference.id == id(target)
            ));
    }
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn settled_results_arrive_as_named_and_thread_current_state() {
    let (memory, root) = fixture(true).await;
    for route in ["named", "thread", "thread_expansion"] {
        let result = memory.retrieve(request(route)).await.unwrap();
        assert_eq!(
            result
                .pack
                .derived_memories
                .iter()
                .map(|item| item.memory.id)
                .collect::<Vec<_>>(),
            vec![id(401), id(402)],
            "{route}"
        );
        assert!(result
            .pack
            .derived_memories
            .iter()
            .all(|item| item.resolved_by.is_empty()));
        assert!(result.pack.open_loops.is_empty());
        assert!(result.pack.commitments.is_empty());
    }
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn recalled_resolution_has_no_omission_but_section_capped_resolution_does() {
    let (memory, root) = fixture(true).await;
    for include_trace in [true, false] {
        let mut context = request("named");
        context.include_trace = include_trace;
        context.graph_limits.allowed_relation_types.clear();
        context.graph_limits.max_depth = 2;
        let result = memory.retrieve(context.clone()).await.unwrap();
        assert_eq!(result.pack.open_loops[0].memory.id, id(301));
        assert_eq!(
            result.pack.open_loops[0].resolved_by,
            vec![id(401), id(402)]
        );
        assert_eq!(result.pack.commitments[0].memory.id, id(302));
        assert_eq!(result.pack.commitments[0].resolved_by, vec![id(402)]);
        if let Some(trace) = result.trace {
            assert!(trace
                .lifecycle_filter_decisions
                .iter()
                .all(|entry| entry.object.id != id(301) && entry.object.id != id(302)));
        }
        assert!(result
            .rationale
            .lifecycle_omission_reasons
            .iter()
            .all(|entry| entry.reason != LifecycleFilterReason::ResolvedOmitted));
        assert_eq!(result.rationale.lifecycle_omission_count, 0);

        // Reaching an object during expansion is not enough: only pack admission clears the omission.
        context.include_trace = true;
        context.section_limits.open_loops = 0;
        let result = memory.retrieve(context).await.unwrap();
        assert!(result.pack.open_loops.is_empty());
        omitted(&result, 301, LifecycleFilterReason::ResolvedOmitted);
        assert!(result
            .trace
            .as_ref()
            .unwrap()
            .lifecycle_filter_decisions
            .iter()
            .all(|entry| entry.object.id != id(302)));
        assert!(result
            .rationale
            .lifecycle_omission_reasons
            .iter()
            .any(|entry| entry.reason == LifecycleFilterReason::ResolvedOmitted));
    }
    memory.close().await.unwrap();
    root.close().unwrap();
}
