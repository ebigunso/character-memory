use async_trait::async_trait;
use character_memory::*;
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};

#[path = "support/mod.rs"]
pub mod test_support;

struct Provider;

fn embedding(text: &str) -> Vec<f32> {
    let (topic, scene): (f32, f32) = match text {
        "orchids" | "unrelated" => (1.0, 0.0),
        "studio" | "botanist" | "stranger" => (0.0, 1.0),
        text if text.starts_with("Episode summary: Orchid lesson ") => {
            let rank: f32 = text.rsplit(' ').next().unwrap().parse().unwrap();
            (0.9 - rank * 0.3 / 7.0, 0.0)
        }
        text if text.contains("Weak topic ") => (0.2, 0.0),
        text if text.contains("Another ordinary day") => (0.1, 0.0),
        _ => (0.0, 0.0),
    };
    vec![
        topic,
        scene,
        (1.0 - topic * topic - scene * scene).max(0.0).sqrt(),
    ]
}

#[async_trait]
impl EmbeddingProvider for Provider {
    fn vector_size(&self) -> usize {
        3
    }
    async fn generate_embedding<'a>(&self, text: &'a str) -> Result<Vec<f32>, EmbeddingError> {
        Ok(embedding(text))
    }
    async fn bulk_generate_embeddings<'a>(
        &self,
        texts: &'a [&'a str],
    ) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        Ok(texts.iter().map(|text| embedding(text)).collect())
    }
}

fn id(n: u128) -> MemoryId {
    MemoryId::from_u128(n)
}
fn time() -> DateTime<Utc> {
    "2026-09-21T12:00:00Z".parse().unwrap()
}
fn keyed(n: u128) -> SceneParticipant {
    SceneParticipant {
        key: Some(id(n)),
        ..Default::default()
    }
}
fn described(words: &str) -> SceneParticipant {
    SceneParticipant {
        description: Some(words.to_owned()),
        ..Default::default()
    }
}

async fn open() -> (CharacterMemory, tempfile::TempDir) {
    let root = tempfile::tempdir().unwrap();
    let settings = config::Config::builder()
        .set_override(
            "vector_store_path",
            root.path().join("vectors").to_string_lossy().into_owned(),
        )
        .unwrap()
        .set_override("graph_store_mode", "persistent")
        .unwrap()
        .set_override(
            "oxigraph_path",
            root.path().join("graph").to_string_lossy().into_owned(),
        )
        .unwrap()
        .set_override("retrieval_stats_store_mode", "sqlite")
        .unwrap()
        .set_override(
            "retrieval_stats_path",
            root.path()
                .join("stats.sqlite3")
                .to_string_lossy()
                .into_owned(),
        )
        .unwrap()
        .build()
        .unwrap();
    let memory = CharacterMemory::new_with_embedding_provider(
        Settings::new(settings).unwrap(),
        test_support::unique_collection_name(),
        Box::new(Provider),
    )
    .await
    .unwrap();
    (memory, root)
}

fn provenance() -> CandidateProvenance {
    CandidateProvenance::caller("scene reminder baseline")
}
fn add_entity(plan: RememberWritePlan, n: u128) -> RememberWritePlan {
    let mut entity = EntityDraft::new();
    entity.id = Some(id(n));
    entity.created_at = Some(time());
    entity.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    plan.with_candidate(MemoryCandidate::Entity(EntityCandidate::new(
        entity,
        provenance(),
    )))
}
fn add_episode(plan: RememberWritePlan, n: u128, text: &str, scene: Scene) -> RememberWritePlan {
    let mut episode = EpisodeDraft::new(text);
    episode.id = Some(id(n));
    episode.created_at = Some(time());
    episode.scene = Some(scene);
    episode.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
        episode,
        provenance(),
    )))
    .with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
        MemoryObjectRef::new(ObjectType::Episode, id(n)),
        provenance(),
    )))
}
fn add_belief(
    plan: RememberWritePlan,
    n: u128,
    entity: Option<u128>,
    indexed: bool,
) -> RememberWritePlan {
    let mut memory = DerivedMemoryDraft::new(
        DerivedType::Claim,
        format!(
            "{} {n}",
            if entity.is_some() {
                "Fitting belief"
            } else {
                "Weak topic"
            }
        ),
    );
    memory.id = Some(id(n));
    memory.created_at = Some(time());
    memory.updated_at = Some(time());
    memory.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    memory.given_by_application = true;
    memory.salience_score = if entity.is_some() { 1.0 } else { 0.0 };
    memory.entity_ids.push(id(entity.unwrap_or(8)));
    let plan = plan.with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
        memory,
        provenance(),
    )));
    if indexed {
        plan.with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
            MemoryObjectRef::new(ObjectType::DerivedMemory, id(n)),
            provenance(),
        )))
    } else {
        plan
    }
}
async fn commit(memory: &CharacterMemory, plan: RememberWritePlan) {
    let outcome = memory.commit(plan, CommitOptions::default()).await.unwrap();
    assert!(outcome.vector_indexing_failure.is_none(), "{outcome:?}");
}
fn scene() -> Scene {
    let mut scene = Scene::at(time());
    scene.setting.words = Some("studio".to_owned());
    scene.participants.push(described("botanist"));
    scene
}
fn query(topic: Option<&str>, current: Scene) -> RetrievalContext {
    let mut context = RetrievalContext::default().with_scene(current).with_trace();
    context.topic = topic.map(str::to_owned);
    context
}
fn snapshot(case: &str, outcome: &RetrieveOutcome) -> Value {
    let trace = outcome.trace.as_ref().unwrap();
    let topic_count = |ids: Vec<MemoryId>| {
        ids.iter()
            .filter(|id| (2000..2008).contains(&id.as_u128()))
            .count()
    };
    let episode_ids = outcome
        .pack
        .relevant_episodes
        .iter()
        .map(|episode| episode.id)
        .collect::<Vec<_>>();
    let belief_ids = outcome
        .pack
        .derived_memories
        .iter()
        .map(|entry| entry.memory.id)
        .collect::<Vec<_>>();
    json!({
        "case": case,
        "topic_counts": [topic_count(trace.vector_candidates.iter().map(|row| row.object.id).collect()), topic_count(trace.graph_expansions.iter().filter(|row| row.outcome == GraphExpansionOutcome::Expanded).map(|row| row.root.id).collect()), topic_count(episode_ids.clone())],
        "episode_ids": episode_ids,
        "belief_ids": belief_ids,
        "fitting_beliefs": belief_ids.iter().filter(|id| (3000..3300).contains(&id.as_u128())).count(),
        "vector_candidates": trace.vector_candidates,
        "graph_roots": trace.graph_expansions,
        "assignments": trace.section_assignments,
        "floor_admissions": trace.floor_admissions,
        "scene_references": outcome.scene_references,
        "completeness": outcome.rationale.telemetry.vector_recall_completeness,
        "scene_cue_omitted_counts": trace.scene_cue_omitted_counts,
    })
}

#[tokio::test]
async fn scene_reminders_and_state_score_fill_preserve_their_witnesses() {
    let mut rows = Vec::new();
    let (memory, root) = open().await;
    let mut plan = add_entity(RememberWritePlan::new(), 7);
    for index in 0..48 {
        let mut past = scene();
        past.time -= Duration::days(48 - index as i64);
        past.participants[0].key = Some(id(7));
        plan = add_episode(plan, 1047 - index, "Another ordinary day", past);
    }
    for index in 0..8 {
        plan = add_episode(
            plan,
            2000 + index,
            &format!("Orchid lesson {index}"),
            Scene::at(time() - Duration::days(1)),
        );
    }
    commit(&memory, plan).await;
    for (case, current, topic) in [
        ("topic-alone", Scene::at(time()), Some("orchids")),
        ("keyless-topic-and-descriptions", scene(), Some("orchids")),
        ("descriptions-only", scene(), None),
    ] {
        rows.push(snapshot(
            case,
            &memory.retrieve(query(topic, current)).await.unwrap(),
        ));
    }
    let mut knowing = scene();
    knowing.participants[0].key = Some(id(7));
    rows.push(snapshot(
        "keyed-overlap",
        &memory
            .retrieve(query(Some("orchids"), knowing))
            .await
            .unwrap(),
    ));
    for floor in [0, 3] {
        let mut context = query(None, scene());
        context.cue_floors.participant = floor;
        context.cue_floors.place = floor;
        rows.push(snapshot(
            &format!("descriptions-only-floor-{floor}"),
            &memory.retrieve(context).await.unwrap(),
        ));
    }
    memory.close().await.unwrap();
    root.close().unwrap();

    let (memory, root) = open().await;
    let mut plan = add_entity(add_entity(RememberWritePlan::new(), 7), 8);
    for n in 3000..3300 {
        plan = add_belief(plan, n, Some(7), false);
    }
    for n in 4000..4012 {
        plan = add_belief(plan, n, None, true);
    }
    commit(&memory, plan).await;
    let mut current = Scene::at(time());
    current.participants.push(keyed(7));
    for topic in [None, Some("unrelated")] {
        for cap in [2, 6, 12] {
            let mut context = query(topic, current.clone());
            context.section_limits.derived_memories = cap;
            rows.push(snapshot(
                &format!("keyed-state-topic={topic:?}-cap={cap}"),
                &memory.retrieve(context).await.unwrap(),
            ));
        }
    }
    memory.close().await.unwrap();
    root.close().unwrap();

    let (memory, root) = open().await;
    let mut plan = add_entity(add_entity(RememberWritePlan::new(), 7), 8);
    plan = add_belief(plan, 3000, Some(7), false);
    plan = add_belief(plan, 3001, Some(8), false);
    let mut past = Scene::at(time() - Duration::days(1));
    past.participants.push(described("stranger"));
    plan = add_episode(plan, 9000, "Encounter", past);
    commit(&memory, plan).await;
    for description in [false, true] {
        let mut current = Scene::at(time());
        current.participants = vec![keyed(7), keyed(8)];
        if description {
            current.participants.push(described("stranger"));
        }
        let mut context = query(None, current);
        context.candidate_limits.max_graph_roots = 2;
        rows.push(snapshot(
            &format!("same-kind-stranger={description}"),
            &memory.retrieve(context).await.unwrap(),
        ));
    }
    memory.close().await.unwrap();
    root.close().unwrap();
    println!(
        "REMINDER_EVIDENCE={}",
        serde_json::to_string(&rows).unwrap()
    );
    for row in &rows {
        let case = row["case"].as_str().unwrap();
        if case == "topic-alone" {
            assert_eq!(row["topic_counts"], json!([8, 8, 8]));
        } else if case == "keyless-topic-and-descriptions" || case == "keyed-overlap" {
            assert_eq!(row["topic_counts"], json!([8, 8, 7]), "{case}");
            assert_eq!(
                row["episode_ids"],
                json!([1000, 2000, 2001, 2002, 2003, 2004, 2005, 2006].map(id)),
                "{case}"
            );
        } else if case.starts_with("descriptions-only") {
            let expected = [1000, 1001, 1002, 1003, 2000, 2001, 2002, 2003].map(id);
            assert_eq!(row["episode_ids"], json!(expected), "{case}");
            for episode in expected {
                let assignment = row["assignments"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|entry| entry["object"]["id"] == json!(episode))
                    .unwrap();
                assert!(assignment["cue_kinds"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("recency")));
            }
            let omitted = if case.ends_with("floor-3") { 45 } else { 47 };
            assert_eq!(
                row["scene_cue_omitted_counts"],
                json!({"participant": omitted, "place": omitted})
            );
        } else if case.starts_with("keyed-state") {
            let cap: usize = case.rsplit('=').next().unwrap().parse().unwrap();
            let topic_floor = usize::from(case.contains("Some"));
            assert_eq!(row["fitting_beliefs"], json!(cap - topic_floor), "{case}");
        } else {
            assert_eq!(row["belief_ids"], json!([id(3000), id(3001)]), "{case}");
        }
    }
}

#[tokio::test]
async fn recent_occasions_use_scene_time_across_the_full_pool() {
    let (memory, root) = open().await;
    let mut plan = RememberWritePlan::new();
    for (n, days) in [(30, -3), (20, -2), (90, -1), (10, 1)] {
        let mut past = Scene::at(time() + Duration::days(days));
        past.setting.words = Some("studio".to_owned());
        plan = add_episode(plan, n, "Encounter", past);
    }
    for candidate in &mut plan.candidates {
        if let MemoryCandidate::Episode(candidate) = candidate {
            candidate.draft.salience_score = if candidate.draft.id == Some(id(90)) {
                0.0
            } else {
                1.0
            };
        }
    }
    commit(&memory, plan).await;
    let mut current = Scene::at(time());
    current.setting.words = Some("studio".to_owned());
    for floor in [0, 1, 3] {
        let mut context = query(None, current.clone());
        context.cue_floors.place = floor;
        context.candidate_limits.max_vector_candidates = if floor < 2 { 1 } else { 8 };
        let result = memory.retrieve(context).await.unwrap();
        let trace = result.trace.unwrap();
        let expected = if floor < 2 {
            vec![id(90), id(20), id(30)]
        } else {
            vec![id(20), id(30), id(90)]
        };
        assert_eq!(
            result
                .pack
                .relevant_episodes
                .iter()
                .map(|e| e.id)
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            trace.scene_cue_omitted_counts[&CueKind::Place],
            3 - floor.max(1)
        );
        assert!(trace
            .vector_candidates
            .iter()
            .all(|row| row.object.id != id(10)));
        assert_eq!(
            result.rationale.telemetry.vector_recall_completeness,
            VectorRecallCompleteness::Exhaustive { scanned: 4 }
        );
    }
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::episode(id(90)),
            "forget latest",
        ))
        .await
        .unwrap();
    let result = memory.retrieve(query(None, current)).await.unwrap();
    assert_eq!(result.pack.relevant_episodes[0].id, id(20));
    assert_eq!(
        result.trace.unwrap().scene_cue_omitted_counts[&CueKind::Place],
        1
    );
    let topic = memory
        .retrieve(query(Some("Encounter"), Scene::at(time())))
        .await
        .unwrap();
    assert!(
        topic
            .pack
            .relevant_episodes
            .iter()
            .any(|episode| episode.id == id(10)),
        "scene cutoff is not a retrieval-wide as-of filter"
    );
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn reminder_stops_at_the_thread_but_a_topic_route_keeps_full_standing() {
    let (memory, root) = open().await;
    let mut thread = MemoryThreadDraft::new("project", "project");
    thread.id = Some(id(7));
    thread.created_at = Some(time());
    thread.updated_at = Some(time());
    thread.last_touched_at = Some(time());
    thread.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut plan = RememberWritePlan::new().with_candidate(MemoryCandidate::MemoryThread(
        MemoryThreadCandidate::new(thread, provenance()),
    ));
    for n in [10, 11] {
        let mut past = Scene::at(time() - Duration::days(n as i64 - 9));
        past.setting.words = Some("studio".to_owned());
        plan = add_episode(plan, n, "Another ordinary day", past);
    }
    let mut interpretation =
        DerivedMemoryDraft::new(DerivedType::Claim, "The work from this occasion.")
            .with_source_episode(id(10));
    interpretation.id = Some(id(90));
    interpretation.created_at = Some(time());
    interpretation.updated_at = Some(time());
    interpretation.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    plan = plan.with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
        interpretation,
        provenance(),
    )));
    commit(&memory, plan).await;
    memory
        .link(MemoryLinkDraft::new(
            ObjectType::DerivedMemory,
            id(90),
            RelationType::DerivedFrom,
            ObjectType::Episode,
            id(10),
        ))
        .await
        .unwrap();
    for n in [10, 11] {
        memory
            .link(MemoryLinkDraft::new(
                ObjectType::Episode,
                id(n),
                RelationType::PartOfThread,
                ObjectType::MemoryThread,
                id(7),
            ))
            .await
            .unwrap();
    }
    let mut current = Scene::at(time());
    current.setting.words = Some("studio".to_owned());
    let result = memory.retrieve(query(None, current.clone())).await.unwrap();
    assert_eq!(
        result
            .pack
            .relevant_episodes
            .iter()
            .map(|e| e.id)
            .collect::<Vec<_>>(),
        [id(10), id(11)]
    );
    assert_eq!(
        result
            .pack
            .active_threads
            .iter()
            .map(|t| t.id)
            .collect::<Vec<_>>(),
        [id(7)]
    );
    let history = result
        .trace
        .as_ref()
        .unwrap()
        .section_assignments
        .iter()
        .find(|row| row.object == MemoryObjectRef::new(ObjectType::Episode, id(11)))
        .unwrap();
    assert_eq!(history.cue_kinds, [CueKind::Recency].into_iter().collect());
    let mut bounded = query(None, current.clone());
    bounded.candidate_limits.max_graph_roots = 1;
    let bounded = memory.retrieve(bounded).await.unwrap();
    assert_eq!(
        bounded
            .pack
            .relevant_episodes
            .iter()
            .map(|episode| episode.id)
            .collect::<Vec<_>>(),
        [id(10)]
    );
    // In this small store every topic returns both episodes; membership alone
    // must not let the high reminder score travel through the thread.
    for (topic, history_cue) in [("orchids", 0.075_f32), ("Another ordinary day", 0.75)] {
        let mut context = query(Some(topic), current.clone());
        context.candidate_limits.max_graph_roots = 1;
        let full = memory.retrieve(context).await.unwrap();
        assert_eq!(full.pack.relevant_episodes.len(), 2);
        let trace = full.trace.unwrap();
        let row = |kind, n| {
            trace
                .section_assignments
                .iter()
                .find(|row| row.object == MemoryObjectRef::new(kind, id(n)))
                .unwrap()
        };
        let cue = |kind, n| match &row(kind, n).reason {
            SectionAssignmentReason::Selected { scores } => scores.cue_score.unwrap(),
            reason => panic!("expected selected object, got {reason:?}"),
        };
        assert!((cue(ObjectType::Episode, 10) - 1.0).abs() < 0.000001);
        assert!((cue(ObjectType::MemoryThread, 7) - 0.75).abs() < 0.000001);
        assert!((cue(ObjectType::DerivedMemory, 90) - 0.75).abs() < 0.000001);
        assert!((cue(ObjectType::Episode, 11) - history_cue).abs() < 0.000001);
        assert_eq!(
            row(ObjectType::Episode, 11).cue_kinds,
            [CueKind::Topic].into_iter().collect()
        );
        assert!(row(ObjectType::DerivedMemory, 90)
            .cue_kinds
            .contains(&CueKind::Place));
        assert_eq!(
            trace
                .graph_expansions
                .iter()
                .filter(|row| row.outcome == GraphExpansionOutcome::Expanded)
                .count(),
            1
        );
    }
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn descriptions_support_write_recall_and_interpretation_without_application_ids() {
    let (memory, root) = open().await;
    let current = scene();
    let written = memory
        .remember(
            RememberInput::new("The visitor showed how to care for the orchids.")
                .with_scene(current.clone()),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    let recalled = memory.retrieve(query(None, current.clone())).await.unwrap();
    assert_eq!(recalled.pack.relevant_episodes.len(), 1);
    let episode = &recalled.pack.relevant_episodes[0];
    assert!(written.persisted_object_ids.contains(&episode.id));
    let interpretation =
        DerivedMemoryDraft::new(DerivedType::Claim, "The visitor knows orchid care.")
            .with_source_episode(episode.id);
    assert!(interpretation.id.is_none());
    // The caller-built plan stands in for consolidation; source ids come only from recall.
    let input = RememberInput::new("Reflection on the visit.")
        .with_scene(Scene::at(time() - Duration::hours(1)))
        .with_derived_memory(interpretation);
    let defaults = RememberPlanDefaults::generated();
    let interpreted_id = input
        .prepared_candidate_refs(&defaults)
        .candidate_refs
        .into_iter()
        .find(|object| object.object_type == ObjectType::DerivedMemory)
        .unwrap()
        .id;
    let input = input.with_memory_link(MemoryLinkDraft::new(
        ObjectType::DerivedMemory,
        interpreted_id,
        RelationType::DerivedFrom,
        ObjectType::Episode,
        episode.id,
    ));
    commit(&memory, input.prepare_write_plan(&defaults)).await;
    let recalled = memory.retrieve(query(None, current)).await.unwrap();
    let interpreted = recalled
        .pack
        .derived_memories
        .iter()
        .find(|row| row.memory.text == "The visitor knows orchid care.")
        .unwrap();
    assert_eq!(interpreted.memory.derived_from_episode_ids, [episode.id]);
    assert!(interpreted.memory.derived_from_observation_ids.is_empty());
    memory.close().await.unwrap();
    root.close().unwrap();
}
