use character_memory::{
    ActivityRef, CharacterMemory, CommitOptions, CorrectMemoryDraft, CorrectionTarget,
    DerivedMemoryDraft, DerivedType, EntityDraft, EpisodeDraft, ForgetMemoryDraft,
    GraphExpansionOutcome, GraphRootSource, LifecycleFilterReason, LifecycleTargetRef,
    MemoryThreadDraft, ObjectType, ObservationDraft, RememberInput, RememberPlanDefaults,
    ReplacementDerivedMemoryDraft, RetrievalContext, Scene, SceneParticipant,
    SourceProvenanceReference, DEFAULT_SCHEMA_VERSION,
};
use test_support::id;
use test_support::time_at_minute as at;

#[path = "support/mod.rs"]
pub mod test_support;
use test_support::derived_ids as ids;

fn scene(setting: Option<&str>, custom: &[(&str, &str)]) -> Scene {
    let mut scene = Scene::at((at(10)).fixed_offset());
    scene.setting.key = setting.map(str::to_owned);
    scene.custom_values = custom
        .iter()
        .map(|(k, v)| ((*k).into(), (*v).into()))
        .collect();
    scene
}
fn episode(n: u128, scene: Scene) -> EpisodeDraft {
    let mut draft = EpisodeDraft::new(format!("source {n}"));
    draft.id = Some(id(n));
    draft.scene = Some(scene);
    draft.created_at = Some(at(10));
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
    draft
}
fn belief(n: u128, source: u128) -> DerivedMemoryDraft {
    let mut draft = DerivedMemoryDraft::new(DerivedType::Claim, format!("belief {n}"));
    draft.id = Some(id(n));
    if source != 0 {
        draft.derived_from_episode_ids.push(id(source));
    }
    draft.created_at = Some(at(20));
    draft.updated_at = Some(at(20));
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
    draft
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
fn context(scene: Scene) -> RetrievalContext {
    let mut result = RetrievalContext::default().with_scene(scene).with_trace();
    result.graph_limits.max_depth = 0;
    result
}

#[tokio::test]
async fn source_union_namespaces_and_restart_determine_scope() {
    let root = tempfile::tempdir().unwrap();
    let collection = test_support::unique_collection_name();
    let memory =
        test_support::try_setup_persistent_character_memory(collection.clone(), root.path())
            .await
            .unwrap();
    let common = "quote \" slash \\ newline\n";
    let first = scene(Some("cafe"), &[("project", common), ("zone", "42")]);
    let second = scene(Some("library"), &[("project", common), ("zone", "43")]);
    let mut observation = ObservationDraft::new(id(102), "observed elsewhere");
    observation.id = Some(id(201));
    commit(
        &memory,
        RememberInput::new("second source")
            .with_episode(episode(102, second))
            .with_observation(observation),
    )
    .await;
    let mut combined = belief(301, 101).with_source_observation(id(201));
    combined.salience_score = 0.9;
    let mut notion = EntityDraft::new();
    notion.id = Some(id(500));
    let mut given = belief(303, 0);
    given.given_by_application = true;
    given.entity_ids.push(id(500));
    let mut words_only = scene(None, &[]);
    words_only.setting.words = Some("cafe".into());
    commit(
        &memory,
        RememberInput::new("words source").with_episode(episode(103, words_only)),
    )
    .await;
    let mut local_observation = ObservationDraft::new(id(101), "local source");
    local_observation.id = Some(id(202));
    let input = RememberInput::new("first source")
        .with_episode(episode(101, first))
        .with_observation(local_observation)
        .with_derived_memory(belief(305, 0).with_source_observation(id(202)))
        .with_entity(notion)
        .with_derived_memory(combined)
        .with_derived_memory(belief(302, 101))
        .with_derived_memory(given)
        .with_derived_memory(belief(304, 103));
    let defaults = RememberPlanDefaults::fixed(&input.content, at(0));
    let plan = input.prepare_write_plan(&defaults);
    memory
        .commit(plan.clone(), CommitOptions::default())
        .await
        .unwrap();
    memory.close().await.unwrap();
    let memory = test_support::try_setup_persistent_character_memory(collection, root.path())
        .await
        .unwrap();
    // Rehydrated internal keys must preserve equality for exact plan replay.
    memory.commit(plan, CommitOptions::default()).await.unwrap();
    for (scene, expected) in [
        (scene(Some("cafe"), &[]), vec![id(301), id(302), id(305)]),
        (
            scene(None, &[("project", common)]),
            vec![id(301), id(302), id(305)],
        ),
        (
            scene(None, &[("zone", "42")]),
            vec![id(301), id(302), id(305)],
        ),
        (scene(None, &[("project", "42")]), vec![]),
        (scene(Some("library"), &[]), vec![id(301)]),
        (scene(None, &[("zone", "43")]), vec![id(301)]),
    ] {
        let result = memory.retrieve(context(scene)).await.unwrap();
        assert_eq!(ids(&result), expected);
        assert!(result
            .trace
            .as_ref()
            .unwrap()
            .graph_expansions
            .iter()
            .all(|row| matches!(
                row.source,
                GraphRootSource::Place | GraphRootSource::Recency
            )));
        assert!(!serde_json::to_string(&result)
            .unwrap()
            .contains("scope_keys"));
    }
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn corrections_derive_their_own_scope_and_lifecycle_precedes_root_cap() {
    let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
    commit(
        &memory,
        RememberInput::new("new source").with_episode(episode(102, scene(Some("new"), &[]))),
    )
    .await;
    commit(
        &memory,
        RememberInput::new("old and new sources")
            .with_episode(episode(101, scene(Some("old"), &[])))
            .with_derived_memory(belief(301, 101)),
    )
    .await;
    let origin = SourceProvenanceReference::episode(id(102));
    let mut replacement = ReplacementDerivedMemoryDraft::new(DerivedType::Claim, "new state")
        .with_source_episode(id(101));
    replacement.id = Some(id(302));
    replacement.salience_score = 1.0;
    replacement.correction_origin_provenance = origin.clone();
    let mut correction =
        CorrectMemoryDraft::new(CorrectionTarget::derived_memory(id(301)), "new place")
            .with_replacement(replacement);
    correction.correction_origin = origin;
    memory.correct(correction).await.unwrap();
    let old = memory
        .retrieve(context(scene(Some("old"), &[])))
        .await
        .unwrap();
    assert_eq!(ids(&old), vec![id(302)]);
    assert!(old
        .trace
        .unwrap()
        .lifecycle_filter_decisions
        .iter()
        .any(|row| row.object.id == id(301)
            && row.reason == LifecycleFilterReason::SupersededOmitted
            && row.superseded_by == vec![id(302)]));
    assert_eq!(
        ids(&memory
            .retrieve(context(scene(Some("new"), &[])))
            .await
            .unwrap()),
        vec![id(302)]
    );
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::derived_memory(id(302)),
            "suppress",
        ))
        .await
        .unwrap();
    commit(
        &memory,
        RememberInput::new("eligible lower score").with_derived_memory(belief(303, 102)),
    )
    .await;
    let mut limited = context(scene(Some("new"), &[]));
    limited.candidate_limits.max_graph_roots = 1;
    let result = memory.retrieve(limited.clone()).await.unwrap();
    assert_eq!(ids(&result), vec![id(303)]);
    assert!(result
        .trace
        .unwrap()
        .lifecycle_filter_decisions
        .iter()
        .any(|row| row.object.id == id(302)
            && row.reason == LifecycleFilterReason::SuppressedOmitted));
    limited.lifecycle_policy.include_suppressed = true;
    assert_eq!(
        ids(&memory.retrieve(limited.clone()).await.unwrap()),
        vec![id(302)]
    );
    limited.graph_limits.allowed_object_types = vec![ObjectType::Episode];
    assert!(ids(&memory.retrieve(limited).await.unwrap()).is_empty());
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn place_reminders_share_caps_with_participant_state_and_activity() {
    let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
    let mut thread = MemoryThreadDraft::new("work", "work");
    thread.id = Some(id(600));
    let mut notion = EntityDraft::new();
    notion.id = Some(id(500));
    commit(
        &memory,
        RememberInput::new("anchors")
            .with_entity(notion)
            .with_memory_thread(thread),
    )
    .await;
    for (source, scope, members) in [
        (101, scene(Some("place"), &[]), vec![301, 302, 303, 304]),
        (
            102,
            scene(None, &[("a", "42"), ("z", "42")]),
            vec![311, 312],
        ),
        (103, scene(None, &[]), vec![321]),
    ] {
        let mut input =
            RememberInput::new(format!("scope {source}")).with_episode(episode(source, scope));
        for n in members {
            let mut draft = belief(n, source);
            draft.salience_score = if n == 301 { 1.0 } else { 0.5 };
            if n == 311 {
                draft.entity_ids.push(id(500));
            }
            if n == 321 {
                draft.thread_ids.push(id(600));
            }
            input = input.with_derived_memory(draft);
        }
        commit(&memory, input).await;
    }
    let mut scoped = context(scene(Some("place"), &[("z", "42"), ("a", "42")]));
    scoped.activity = Some(ActivityRef::Thread(id(600)));
    // Principle 1 serves activity's own head (the thread) before its members.
    scoped.candidate_limits.max_graph_roots = 3;
    let result = memory.retrieve(scoped.clone()).await.unwrap();
    // Principles 4 and 5: activity keeps its strength; place reserves only its own head.
    assert_eq!(ids(&result), vec![id(321), id(301)]);
    assert_eq!(result.pack.active_threads[0].id, id(600));
    scoped.candidate_limits.max_graph_roots = 12;
    scoped.section_limits.derived_memories = 3;
    let result = memory.retrieve(scoped.clone()).await.unwrap();
    assert_eq!(ids(&result), vec![id(321), id(301), id(302)]);
    // A participant/place overlap keeps its participant strength in one slot.
    scoped.scene.participants.push(SceneParticipant {
        key: Some(id(500)),
        ..Default::default()
    });
    scoped.graph_limits.max_depth = 1;
    let result = memory.retrieve(scoped.clone()).await.unwrap();
    assert_eq!(ids(&result), vec![id(321), id(311), id(301)]);
    assert_eq!(
        ids(&result).iter().filter(|&&key| key == id(311)).count(),
        1
    );
    scoped.scene.setting.key = None;
    scoped.scene.custom_values.clear();
    assert_eq!(
        ids(&memory.retrieve(scoped).await.unwrap()),
        vec![id(321), id(311)],
        "principle 4 keeps activity outside the round among people"
    );
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn scope_priority_survives_both_caps_and_map_order() {
    let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
    let old = belief(301, 101);
    let mut newer = belief(302, 101);
    newer.created_at = Some(at(30));
    let mut same_time_later_id = newer.clone();
    same_time_later_id.id = Some(id(303));
    commit(
        &memory,
        RememberInput::new("recency")
            .with_episode(episode(101, scene(Some("place"), &[])))
            .with_derived_memory(old)
            .with_derived_memory(newer)
            .with_derived_memory(same_time_later_id),
    )
    .await;
    // Principle 4 breaks equal section scores by memory time, then id.
    for (root_cap, expected) in [(1, 302), (12, 302)] {
        let mut limited = context(scene(Some("place"), &[]));
        limited.candidate_limits.max_graph_roots = root_cap;
        limited.section_limits.derived_memories = 1;
        assert_eq!(
            ids(&memory.retrieve(limited).await.unwrap()),
            vec![id(expected)],
            "root cap {root_cap}"
        );
    }
    for (source, key, memory_id) in [(102, "z", 310), (103, "a", 320)] {
        commit(
            &memory,
            RememberInput::new(key)
                .with_episode(episode(source, scene(None, &[(key, "42")])))
                .with_derived_memory(belief(memory_id, source)),
        )
        .await;
    }
    for root_cap in [1, 12] {
        let mut limited = context(scene(None, &[("z", "42"), ("a", "42")]));
        limited.candidate_limits.max_graph_roots = root_cap;
        limited.section_limits.derived_memories = 1;
        // One Place road breaks equal-time ties by id, not custom-key spelling.
        assert_eq!(ids(&memory.retrieve(limited).await.unwrap()), vec![id(310)]);
    }
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn overlapping_place_keys_share_candidates_without_outscoring_the_topic() {
    let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
    for (source, scope, memory_id, salience) in [
        (101, scene(Some("place"), &[]), 301, 1.0),
        (102, scene(Some("place"), &[("project", "42")]), 302, 0.1),
        (103, scene(None, &[("project", "42")]), 303, 0.9),
    ] {
        let mut memory_draft = belief(memory_id, source);
        memory_draft.salience_score = salience;
        commit(
            &memory,
            RememberInput::new(format!("overlap {source}"))
                .with_episode(episode(source, scope))
                .with_derived_memory(memory_draft),
        )
        .await;
    }
    let mut limited = context(scene(Some("place"), &[("project", "42")]));
    limited.candidate_limits.max_graph_roots = 2;
    // One Place cap contributes 301 and 302; recency outranks 302 in spare room.
    assert_eq!(
        ids(&memory.retrieve(limited.clone()).await.unwrap()),
        vec![id(301)]
    );
    let topic = "quasar telescope astronomy spectroscopy";
    let mut topical = belief(901, 104);
    topical.text = topic.into();
    commit(
        &memory,
        RememberInput::new("separate topic")
            .with_episode(episode(104, scene(None, &[])))
            .with_derived_memory(topical),
    )
    .await;
    limited.topic = Some(topic.into());
    limited.candidate_limits.max_graph_roots = 3;
    let result = memory.retrieve(limited).await.unwrap();
    // Principle 5: a place reminder cannot outscore a positive topic match.
    assert_eq!(ids(&result), vec![id(901), id(301), id(303)]);
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn shared_explicit_state_keeps_expansion_order_for_each_kind() {
    let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
    let mut thread = MemoryThreadDraft::new("work", "work");
    thread.id = Some(id(600));
    commit(
        &memory,
        RememberInput::new("thread").with_memory_thread(thread),
    )
    .await;
    for (source, key, memory_id) in [(103, Some("place"), 321), (104, None, 322)] {
        let mut draft = belief(memory_id, source);
        draft.thread_ids.push(id(600));
        commit(
            &memory,
            RememberInput::new(format!("scope {source}"))
                .with_episode(episode(source, scene(key, &[])))
                .with_derived_memory(draft)
                .with_memory_link(character_memory::MemoryLinkDraft::new(
                    ObjectType::DerivedMemory,
                    id(memory_id),
                    character_memory::RelationType::DerivedFrom,
                    ObjectType::Episode,
                    id(source),
                )),
        )
        .await;
    }
    let mut request = context(scene(Some("place"), &[]));
    request.activity = Some(ActivityRef::Thread(id(600)));
    request.graph_limits.max_depth = 1;
    request.section_limits.relevant_episodes = 1;
    request.cue_floors.place = 0;
    let result = memory.retrieve(request).await.unwrap();
    assert_eq!(
        result
            .pack
            .relevant_episodes
            .iter()
            .map(|episode| episode.id)
            .collect::<Vec<_>>(),
        vec![id(103)]
    );
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn dense_place_records_topic_admission_at_root_cap() {
    let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
    let mut input =
        RememberInput::new("place state").with_episode(episode(101, scene(Some("place"), &[])));
    for n in 301..314 {
        input = input.with_derived_memory(belief(n, 101));
    }
    commit(&memory, input).await;
    let topic = "quasar telescope astronomy spectroscopy";
    let mut topical = belief(901, 102);
    topical.text = topic.into();
    commit(
        &memory,
        RememberInput::new("separate note")
            .with_episode(episode(102, scene(None, &[])))
            .with_derived_memory(topical),
    )
    .await;
    let mut request = context(scene(Some("place"), &[]));
    request.topic = Some(topic.into());
    assert_eq!(request.candidate_limits.max_graph_roots, 12);
    let result = memory.retrieve(request.clone()).await.unwrap();
    let trace = result.trace.as_ref().unwrap();
    let recalled = trace
        .vector_candidates
        .iter()
        .find(|row| row.object.id == id(901))
        .unwrap();
    assert_eq!(
        recalled.rank, 1,
        "topic should be the strongest vector match: {recalled:?}"
    );
    assert_eq!(
        trace
            .graph_expansions
            .iter()
            .filter(|entry| entry.outcome != GraphExpansionOutcome::RootLimit)
            .count(),
        12
    );
    let root_trace = trace
        .graph_expansions
        .iter()
        .find(|row| row.root.id == id(901))
        .unwrap();
    assert_eq!(root_trace.outcome, GraphExpansionOutcome::Expanded);
    assert!(ids(&result).contains(&id(901)));
    let mut additional = RememberInput::new("more topical notes");
    for n in 902..908 {
        let mut topical = belief(n, 102);
        topical.text = topic.into();
        additional = additional.with_derived_memory(topical);
    }
    commit(&memory, additional).await;
    let shared = memory.retrieve(request).await.unwrap();
    let selected = ids(&shared);
    assert_eq!(
        selected
            .iter()
            .filter(|key| (901..908).contains(&key.as_u128()))
            .count(),
        7,
        "principle 4 gives place reminders no spare root turn"
    );
    assert_eq!(
        selected
            .iter()
            .filter(|key| (301..314).contains(&key.as_u128()))
            .count(),
        5
    );
    memory.close().await.unwrap();
    root.close().unwrap();
}
