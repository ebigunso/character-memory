use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};

use crate::*;

struct TimeProvider;

#[async_trait]
impl EmbeddingProvider for TimeProvider {
    fn vector_size(&self) -> usize {
        2
    }
    async fn generate_embedding<'a>(&self, text: &'a str) -> Result<Vec<f32>, EmbeddingError> {
        let score: f32 = if matches!(
            text,
            "topic" | "what happened last Tuesday" | "what happened today"
        ) {
            1.0
        } else if text.contains("Strong 900") {
            0.85
        } else if text.contains("Strong ") {
            0.9 - text.rsplit(' ').next().unwrap().parse::<f32>().unwrap() * 0.001
        } else if text.contains("Weak") {
            0.3
        } else {
            0.0
        };
        Ok(vec![score, (1.0 - score * score).sqrt()])
    }
    async fn bulk_generate_embeddings<'a>(
        &self,
        texts: &'a [&'a str],
    ) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let mut vectors = Vec::new();
        for text in texts {
            vectors.push(self.generate_embedding(text).await?);
        }
        Ok(vectors)
    }
}

fn id(n: u128) -> MemoryId {
    MemoryId::from_u128(n)
}
fn time() -> DateTime<Utc> {
    "2026-09-21T18:00:00Z".parse().unwrap()
}
fn keyed() -> SceneParticipant {
    SceneParticipant {
        key: Some(id(7)),
        ..Default::default()
    }
}
fn provenance() -> CandidateProvenance {
    CandidateProvenance::caller("recency witness")
}

async fn open() -> (CharacterMemory, tempfile::TempDir) {
    let root = tempfile::tempdir().unwrap();
    let settings = ::config::Config::builder()
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
        format!("time_{}", MemoryId::new_v4()),
        Box::new(TimeProvider),
    )
    .await
    .unwrap();
    (memory, root)
}

fn episode(
    mut plan: RememberWritePlan,
    n: u128,
    days: i64,
    salience: f32,
    person: bool,
    topic: bool,
) -> RememberWritePlan {
    let mut draft = EpisodeDraft::new(if topic {
        format!("Strong {}", n % 1000)
    } else {
        format!("Ordinary {n}")
    });
    draft.id = Some(id(n));
    // Deliberately unrelated to scene chronology.
    draft.created_at = Some(time() + Duration::days(n as i64));
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut scene = Scene::at(time() - Duration::days(days));
    if n == 100 {
        scene.time += Duration::milliseconds(125);
    }
    if person {
        scene.participants.push(keyed());
    }
    draft.scene = Some(scene);
    draft.salience_score = salience;
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

fn indexed(plan: RememberWritePlan, kind: ObjectType, n: u128) -> RememberWritePlan {
    plan.with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
        MemoryObjectRef::new(kind, id(n)),
        provenance(),
    )))
}

fn link(
    plan: RememberWritePlan,
    from: ObjectType,
    from_id: u128,
    to: ObjectType,
    to_id: u128,
    relation: RelationType,
) -> RememberWritePlan {
    let mut draft = MemoryLinkDraft::new(from, id(from_id), relation, to, id(to_id));
    draft.id = Some(id(10_000_000 + from_id * 10_000 + to_id));
    draft.created_at = Some(time());
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    plan.with_candidate(MemoryCandidate::MemoryLink(MemoryLinkCandidate::new(
        draft,
        provenance(),
    )))
}

fn belief(plan: RememberWritePlan, n: u128, sources: &[u128], topic: bool) -> RememberWritePlan {
    let mut draft = DerivedMemoryDraft::new(
        DerivedType::Claim,
        if topic {
            "Weak interpretation"
        } else {
            "What the occasions meant"
        },
    );
    draft.id = Some(id(n));
    draft.created_at = Some(time());
    draft.updated_at = Some(time());
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    draft.salience_score = 0.0;
    draft.derived_from_episode_ids = sources.iter().copied().map(id).collect();
    let mut plan = plan.with_candidate(MemoryCandidate::DerivedMemory(
        DerivedMemoryCandidate::new(draft, provenance()),
    ));
    for source in sources {
        plan = link(
            plan,
            ObjectType::DerivedMemory,
            n,
            ObjectType::Episode,
            *source,
            RelationType::DerivedFrom,
        );
    }
    if topic {
        indexed(plan, ObjectType::DerivedMemory, n)
    } else {
        plan
    }
}

fn base() -> RememberWritePlan {
    let mut entity = EntityDraft::new();
    entity.id = Some(id(7));
    entity.created_at = Some(time());
    entity.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut plan = RememberWritePlan::new().with_candidate(MemoryCandidate::Entity(
        EntityCandidate::new(entity, provenance()),
    ));
    for (n, days, salience) in [
        (900, 0, 0.0),
        (100, 1, 1.0),
        (700, 31, 0.3),
        (600, 32, 0.0),
        (500, 33, 0.0),
        (50, -1, 0.0),
    ] {
        plan = episode(plan, n, days, salience, true, false);
    }
    // The named participant has several occasions but is not ubiquitous.
    for n in 800..820 {
        plan = episode(plan, n, 60, 0.0, false, false);
    }
    for index in 0..48 {
        plan = episode(plan, 2000 + index, 90 + index as i64, 0.0, false, true);
    }
    plan = belief(plan, 300, &[900, 700], false);
    let mut observation =
        ObservationDraft::new(id(800), "A separate unlinked observation: violet steam");
    observation.id = Some(id(400));
    observation.created_at = Some(time());
    observation.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    plan.with_candidate(MemoryCandidate::Observation(ObservationCandidate::new(
        observation,
        provenance(),
    )))
}

async fn commit(memory: &CharacterMemory, plan: RememberWritePlan) {
    let outcome = memory.commit(plan, CommitOptions::default()).await.unwrap();
    assert!(outcome.vector_indexing_failure.is_none(), "{outcome:?}");
}

fn query(topic: bool, person: bool) -> RetrievalContext {
    let mut scene = Scene::at(time());
    if person {
        scene.participants.push(keyed());
    }
    let mut context = RetrievalContext::default().with_scene(scene).with_trace();
    context.topic = topic.then(|| "topic".to_owned());
    context.graph_limits.timeout_ms = None;
    context
}

async fn record(
    rows: &mut Vec<Value>,
    case: &str,
    memory: &CharacterMemory,
    context: RetrievalContext,
) -> RetrieveOutcome {
    let result = memory.retrieve(context.clone()).await.unwrap();
    assert_eq!(result.scene, context.scene);
    assert!(result.activity.is_none());
    let trace = result.trace.as_ref().unwrap();
    rows.push(json!({"case": case, "input": context, "outcome": result,
        "episodes": result.pack.relevant_episodes.iter().map(|e| e.id.as_u128()).collect::<Vec<_>>(),
        "work": result.pack.derived_memories.iter().map(|e| e.memory.id.as_u128()).collect::<Vec<_>>(),
        "roots": trace.graph_expansions.iter().filter(|r| r.outcome == GraphExpansionOutcome::Expanded).map(|r| r.root.id.as_u128()).collect::<Vec<_>>() }));
    result
}

fn episodes(result: &RetrieveOutcome) -> Vec<u128> {
    result
        .pack
        .relevant_episodes
        .iter()
        .map(|episode| episode.id.as_u128())
        .collect()
}

fn roots(result: &RetrieveOutcome) -> Vec<u128> {
    result
        .trace
        .as_ref()
        .unwrap()
        .graph_expansions
        .iter()
        .filter(|row| row.outcome == GraphExpansionOutcome::Expanded)
        .map(|row| row.root.id.as_u128())
        .collect()
}

fn assignment(result: &RetrieveOutcome, n: u128) -> &SectionAssignment {
    result
        .trace
        .as_ref()
        .unwrap()
        .section_assignments
        .iter()
        .find(|row| row.object.id == id(n))
        .unwrap()
}

fn scores(result: &RetrieveOutcome, n: u128) -> SectionScoreComponents {
    match assignment(result, n).reason {
        SectionAssignmentReason::Selected { scores }
        | SectionAssignmentReason::OmittedByLimit { scores, .. } => scores,
        ref reason => panic!("expected ranked object {n}: {reason:?}"),
    }
}

fn recency_episodes(result: &RetrieveOutcome) -> Vec<u128> {
    result
        .trace
        .as_ref()
        .unwrap()
        .section_assignments
        .iter()
        .filter(|row| {
            row.object.object_type == ObjectType::Episode
                && row.cue_kinds.contains(&CueKind::Recency)
        })
        .map(|row| row.object.id.as_u128())
        .collect()
}

fn room(cap: usize) -> ContinuitySectionLimits {
    ContinuitySectionLimits {
        active_threads: cap,
        relevant_episodes: cap,
        salient_observations: cap,
        derived_memories: cap,
        preferences: cap,
        relationship_notes: cap,
        open_loops: cap,
        commitments: cap,
        character_signals: cap,
    }
}

#[tokio::test]
async fn recency_scene_only_room_and_floor_witnesses() {
    let mut rows = Vec::new();
    let (memory, root) = open().await;
    commit(&memory, base()).await;
    let default = record(&mut rows, "1-default-room", &memory, query(false, false)).await;
    assert_eq!(episodes(&default), [100, 700, 900, 600, 500, 800, 801, 802]);
    assert_eq!(
        roots(&default),
        [900, 100, 700, 600, 500, 800, 801, 802, 803, 804, 805, 806]
    );
    assert_eq!(
        default
            .rationale
            .telemetry
            .unique_graph_root_candidate_count,
        16
    );
    assert!(default.pack.salient_observations.is_empty());
    let mut single = query(false, false);
    single.section_limits = room(1);
    let first = record(
        &mut rows,
        "1-room1-4-unlinked-5-reminder-leaf",
        &memory,
        single.clone(),
    )
    .await;
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
    let multiple = record(&mut rows, "revised-room3", &memory, three.clone()).await;
    assert_eq!(episodes(&multiple), [100, 700, 900]);
    assert_eq!(recency_episodes(&multiple), [100, 700, 900]);
    let mut high_floor = three.clone();
    high_floor.cue_floors.recency = 7;
    assert_eq!(
        roots(&memory.retrieve(high_floor).await.unwrap()),
        roots(&multiple)
    );
    let mut zero = query(false, false);
    zero.section_limits = room(0);
    zero.cue_floors.recency = 7;
    let zero = record(&mut rows, "revised-room0", &memory, zero).await;
    assert!(roots(&zero).is_empty());
    assert!(zero.pack.relevant_episodes.is_empty());
    let mut tight = three;
    tight.section_limits.relevant_episodes = 1;
    tight.cue_floors.recency = 3;
    let tight = record(&mut rows, "revised-room3-tight", &memory, tight).await;
    assert_eq!(
        episodes(&tight),
        [900],
        "the raised floor reserves the latest contribution"
    );
    memory.close().await.unwrap();
    root.close().unwrap();
    println!("TIME_WITNESSES={}", serde_json::to_string(&rows).unwrap());
}

#[tokio::test]
async fn recency_keyed_participant_witnesses() {
    let mut rows = Vec::new();
    let (memory, root) = open().await;
    commit(&memory, base()).await;
    let known = record(
        &mut rows,
        "5-explicit-key-6-own-character-8-neighbor-score",
        &memory,
        query(false, true),
    )
    .await;
    assert_eq!(episodes(&known), [100, 700, 900, 600, 500, 800, 801, 802]);
    assert_eq!(
        scores(&known, 900),
        SectionScoreComponents {
            final_score: 0.737_499_95,
            cue_score: Some(0.75),
            graph_score: Some(1.0),
            salience_score: None,
        }
    );
    assert_eq!(scores(&known, 300).graph_score, Some(1.0 / 3.0));
    assert_eq!(scores(&known, 300).final_score, 0.570_833_3);
    let mut known_single = query(false, true);
    known_single.section_limits = room(1);
    let known_single = memory.retrieve(known_single).await.unwrap();
    assert_eq!(recency_episodes(&known_single), [900]);
    assert!(assignment(&known_single, 700)
        .cue_kinds
        .contains(&CueKind::Participant));
    memory.close().await.unwrap();
    root.close().unwrap();
    println!("TIME_WITNESSES={}", serde_json::to_string(&rows).unwrap());
}

#[tokio::test]
async fn recency_preserves_original_saturated_topic() {
    let mut rows = Vec::new();
    let (memory, root) = open().await;
    commit(&memory, base()).await;
    let saturated = record(
        &mut rows,
        "7-saturated-original-store",
        &memory,
        query(true, false),
    )
    .await;
    assert_eq!(episodes(&saturated), (2000..2008).collect::<Vec<_>>());
    assert_eq!(roots(&saturated), (2000..2012).collect::<Vec<_>>());
    assert!(recency_episodes(&saturated).is_empty());
    memory.close().await.unwrap();
    root.close().unwrap();
    println!("TIME_WITNESSES={}", serde_json::to_string(&rows).unwrap());
}

#[tokio::test]
async fn recency_reference_time_and_lifecycle_witnesses() {
    let mut rows = Vec::new();
    let (memory, root) = open().await;
    commit(&memory, base()).await;
    let mut single = query(false, false);
    single.section_limits = room(1);
    let first = memory.retrieve(single.clone()).await.unwrap();
    let repeated = record(&mut rows, "11-identical-input", &memory, single.clone()).await;
    assert_eq!(first, repeated);
    let mut untraced = single.clone();
    untraced.include_trace = false;
    let untraced = memory.retrieve(untraced).await.unwrap();
    assert_eq!(first.pack, untraced.pack);
    assert!(untraced.trace.is_none());
    let mut past = single.clone();
    past.scene.time -= Duration::hours(12);
    let past = record(&mut rows, "11-past-reference", &memory, past).await;
    assert_eq!(episodes(&past), [100]);
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::Episode(id(900)),
            "recency advances",
        ))
        .await
        .unwrap();
    let forgotten = record(&mut rows, "11-forgotten-latest", &memory, single.clone()).await;
    assert_eq!(episodes(&forgotten), [100]);
    let mut inclusive = single;
    inclusive.lifecycle_policy.include_suppressed = true;
    let inclusive = record(&mut rows, "11-include-forgotten", &memory, inclusive).await;
    assert_eq!(episodes(&inclusive), [900]);
    memory.close().await.unwrap();
    root.close().unwrap();

    println!("TIME_WITNESSES={}", serde_json::to_string(&rows).unwrap());
}

async fn pressure_witnesses(case: &str, strong: u128, weak: bool, overlap: bool) {
    let mut rows = Vec::new();
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
    let result = record(&mut rows, case, &memory, context.clone()).await;
    if strong == 2 && !overlap {
        assert_eq!(episodes(&result), [2000, 2001]);
        assert_eq!(scores(&result, 2000).final_score, 0.835);
        assert_eq!(scores(&result, 2001).final_score, 0.834_35);
        assert!(scores(&result, 2000).cue_score.unwrap() > 0.16);
        assert!(scores(&result, 2001).cue_score.unwrap() > 0.16);
        assert_eq!(roots(&result), [2000, 2001, 900]);
    }
    if weak {
        assert_eq!(episodes(&result), [2000, 2003]);
        assert_eq!(scores(&result, 2003).final_score, 0.396_25);
        assert_eq!(scores(&result, 2003).graph_score, Some(1.0));
        assert_eq!(scores(&result, 900).final_score, 0.35);
        let mut bounded = context.clone();
        bounded.section_limits = room(2);
        let bounded = record(
            &mut rows,
            "3-weak-descendant-without-root",
            &memory,
            bounded,
        )
        .await;
        assert_eq!(episodes(&bounded), [2000, 900]);
        assert_eq!(scores(&bounded, 2003).final_score, 0.271_25);
        assert!(!roots(&bounded).contains(&2003));
    }
    if overlap {
        assert_eq!(episodes(&result), [900, 2000]);
        assert_eq!(scores(&result, 900).final_score, 0.902_500_03);
        assert_eq!(scores(&result, 900).graph_score, Some(1.0));
        assert!(assignment(&result, 900).cue_kinds.contains(&CueKind::Topic));
        assert_eq!(recency_episodes(&result), [900, 2000, 2001, 750]);
        assert!(assignment(&result, 3200)
            .cue_kinds
            .contains(&CueKind::Recency));
        assert!(assignment(&result, 750)
            .cue_kinds
            .contains(&CueKind::Recency));
        let mut bounded = context.clone();
        bounded.section_limits = room(3);
        let bounded = record(&mut rows, "7-overlap-bounded-provenance", &memory, bounded).await;
        assert_eq!(
            assignment(&bounded, 750).cue_kinds,
            std::collections::BTreeSet::from([CueKind::Topic])
        );
        assert_eq!(roots(&bounded), [2000, 2001, 900]);
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
        assert_eq!(episodes(&result), (2000..2008).collect::<Vec<_>>());
        assert_eq!(roots(&result), (2000..2012).collect::<Vec<_>>());
        assert_eq!(recency_episodes(&result), (2000..2012).collect::<Vec<_>>());
        context.candidate_limits.max_graph_roots = 3;
        let capped = record(&mut rows, "7-rootcap3", &memory, context.clone()).await;
        assert_eq!(roots(&capped), [2000, 2001, 2002]);
        assert_eq!(episodes(&capped), [2000, 2001, 2002]);
        context.candidate_limits.max_graph_roots = 12;
        context.cue_floors.recency = 1;
        let shared_floor = record(&mut rows, "9-shared-floor1", &memory, context.clone()).await;
        assert_eq!(
            episodes(&shared_floor),
            (2000..2007).chain([900]).collect::<Vec<_>>()
        );
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
        let reserved = record(&mut rows, "9-distinct-floor1", &memory, context).await;
        assert_eq!(
            episodes(&reserved),
            (2000..2007).chain([900]).collect::<Vec<_>>()
        );
        for stage in [
            crate::api::types::CueFloorStage::GraphRoots,
            crate::api::types::CueFloorStage::Section {
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
        let additions = record(
            &mut rows,
            "10-topic-spare-additions",
            &memory,
            query(true, false),
        )
        .await;
        assert_eq!(episodes(&additions), [2000, 2001, 900, 800]);
        assert_eq!(recency_episodes(&additions), [2000, 2001, 900, 800]);
        let mut bounded = query(true, false);
        bounded.section_limits = room(3);
        let bounded = record(&mut rows, "10-bounded-additions", &memory, bounded).await;
        assert_eq!(episodes(&bounded), [2000, 2001, 900]);
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
    println!("TIME_WITNESSES={}", serde_json::to_string(&rows).unwrap());
}

#[tokio::test]
async fn recency_strong_spare_witnesses() {
    pressure_witnesses("3-strong-spare", 2, false, false).await;
}

#[tokio::test]
async fn recency_weak_descendant_witnesses() {
    pressure_witnesses("3-weak-descendant", 1, true, false).await;
}

#[tokio::test]
async fn recency_saturated_witnesses() {
    pressure_witnesses("7-saturated", 48, false, false).await;
}

#[tokio::test]
async fn recency_topic_overlap_witnesses() {
    pressure_witnesses("7-topic-overlap", 2, false, true).await;
}

#[tokio::test]
async fn recency_fractional_recorded_time_witnesses() {
    let mut rows = Vec::new();
    let (memory, root) = open().await;
    let plan = episode(
        episode(RememberWritePlan::new(), 900, 0, 0.0, false, false),
        100,
        0,
        0.0,
        false,
        false,
    );
    commit(&memory, plan).await;
    let mut fractional = query(false, false);
    fractional.scene.time += Duration::seconds(1);
    fractional.section_limits = room(1);
    let later = record(
        &mut rows,
        "fractional-scene-time-order",
        &memory,
        fractional,
    )
    .await;
    assert_eq!(episodes(&later), [100]);
    let mut exact = query(false, false);
    exact.section_limits = room(1);
    let boundary = record(&mut rows, "fractional-future-cut", &memory, exact).await;
    assert_eq!(episodes(&boundary), [900]);
    memory.close().await.unwrap();
    root.close().unwrap();
    println!("TIME_WITNESSES={}", serde_json::to_string(&rows).unwrap());
}
#[tokio::test]
async fn recency_stops_at_interpreted_memory_sources() {
    let mut rows = Vec::new();
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
        let result = record(
            &mut rows,
            if topic { "topic-overlap" } else { "time-only" },
            &memory,
            context.clone(),
        )
        .await;
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
        let own = record(
            &mut rows,
            if topic {
                "topic-own-sources"
            } else {
                "time-own-sources"
            },
            &memory,
            context.clone(),
        )
        .await;
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
    println!("RECENCY_COSOURCE={}", serde_json::to_string(&rows).unwrap());
    assert_eq!(
        exclusions,
        [true, true],
        "time-only exclusion and Topic-only provenance"
    );
}

fn description_fixture(person: bool) -> RememberWritePlan {
    let mut plan = RememberWritePlan::new();
    if person {
        let mut entity = EntityDraft::new();
        entity.id = Some(id(7));
        entity.created_at = Some(time());
        entity.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        plan = plan.with_candidate(MemoryCandidate::Entity(EntityCandidate::new(
            entity,
            provenance(),
        )));
    }
    let mut latest = EpisodeDraft::new("The described studio this evening");
    latest.id = Some(id(900));
    latest.created_at = Some(time());
    latest.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut scene = Scene::at(time());
    scene.setting.words = Some("studio".to_owned());
    if person {
        scene.participants.push(keyed());
    }
    latest.scene = Some(scene);
    plan = plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
        latest,
        provenance(),
    )));
    plan = indexed(plan, ObjectType::Episode, 900);
    if person {
        plan = link(
            plan,
            ObjectType::Episode,
            900,
            ObjectType::Entity,
            7,
            RelationType::Involves,
        );
    }
    belief(
        episode(plan, 700, 40, 1.0, false, false),
        300,
        &[900, 700],
        false,
    )
}

#[tokio::test]
async fn tier_a_description_stops_at_shared_interpretation() {
    let (memory, temp) = open().await;
    commit(&memory, description_fixture(false)).await;
    let mut context = query(false, false);
    context.scene.setting.words = Some("studio".to_owned());
    context.section_limits = room(2);
    context.candidate_limits.max_graph_roots = 1;
    let mut rows = Vec::new();
    let result = record(&mut rows, "description-leaf", &memory, context).await;
    println!(
        "TIER_A_DESCRIPTION={}",
        serde_json::to_string(&rows).unwrap()
    );
    memory.close().await.unwrap();
    temp.close().unwrap();
    assert_eq!(roots(&result), [900]);
    assert_eq!(episodes(&result), [900]);
    assert!(!result
        .trace
        .as_ref()
        .unwrap()
        .section_assignments
        .iter()
        .any(|row| row.object.id == id(700)));
    assert!(assignment(&result, 300).cue_kinds.contains(&CueKind::Place));
}

#[tokio::test]
async fn tier_a_description_root_keeps_best_proximity() {
    let (memory, temp) = open().await;
    commit(&memory, description_fixture(true)).await;
    let mut context = query(false, true);
    context.scene.setting.words = Some("studio".to_owned());
    context.section_limits = room(2);
    context.candidate_limits.max_graph_roots = 2;
    let mut rows = Vec::new();
    let result = record(&mut rows, "description-root-proximity", &memory, context).await;
    println!("TIER_A_PROXIMITY={}", serde_json::to_string(&rows).unwrap());
    memory.close().await.unwrap();
    temp.close().unwrap();
    assert_eq!(roots(&result), [7, 900]);
    assert_eq!(scores(&result, 300).graph_score, Some(1.0 / 3.0));
    assert_eq!(scores(&result, 900).graph_score, Some(1.0));
}

#[tokio::test]
async fn tier_a_recency_floor_reserves_latest_before_score_fill() {
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
    let mut rows = Vec::new();
    let zero = record(&mut rows, "floor-zero-score", &memory, context.clone()).await;
    context.cue_floors.recency = 1;
    let reserved = record(&mut rows, "floor-one-latest", &memory, context).await;
    println!("TIER_A_FLOOR={}", serde_json::to_string(&rows).unwrap());
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
            && matches!(row.stage, crate::api::types::CueFloorStage::Section { .. })));
}

#[tokio::test]
async fn tier_a_explicit_recency_floor_reserves_under_topic_pressure() {
    let (memory, temp) = open().await;
    let mut plan = episode(RememberWritePlan::new(), 900, 0, 0.0, false, false);
    for index in 0..48 {
        plan = episode(plan, 2000 + index, 2 + index as i64, 0.0, false, true);
    }
    commit(&memory, plan).await;
    let mut context = query(true, false);
    context.cue_floors.recency = 1;
    let mut rows = Vec::new();
    let result = record(&mut rows, "explicit-floor-topic-pressure", &memory, context).await;
    println!(
        "TIER_A_ROOT_FLOOR={}",
        serde_json::to_string(&rows).unwrap()
    );
    memory.close().await.unwrap();
    temp.close().unwrap();
    assert!(roots(&result).contains(&900));
    assert!(episodes(&result).contains(&900));
    for stage in [
        crate::api::types::CueFloorStage::GraphRoots,
        crate::api::types::CueFloorStage::Section {
            section: ContextPackSection::RelevantEpisodes,
        },
    ] {
        assert!(result
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

#[tokio::test]
async fn tier_a_equal_score_recent_occasions_ignore_id_order() {
    let mut rows = Vec::new();
    let mut recalled_times = Vec::new();
    for reverse in [false, true] {
        let (memory, temp) = open().await;
        let mut plan = RememberWritePlan::new();
        for index in 0..20 {
            let n = if reverse { 2019 - index } else { 1000 + index };
            plan = episode(plan, n, 20 - index as i64, 0.0, false, false);
        }
        commit(&memory, plan).await;
        let result = record(
            &mut rows,
            if reverse {
                "reverse-ids"
            } else {
                "forward-ids"
            },
            &memory,
            query(false, false),
        )
        .await;
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
    println!("TIER_A_TIES={}", serde_json::to_string(&rows).unwrap());
    let latest = (1..=8)
        .map(|days| time() - Duration::days(days))
        .collect::<Vec<_>>();
    assert_eq!(recalled_times, [latest.clone(), latest]);
}

fn range_episode(plan: RememberWritePlan, n: u128, at: DateTime<Utc>) -> RememberWritePlan {
    let mut plan = episode(plan, n, 0, 0.0, false, false);
    for candidate in &mut plan.candidates {
        if let MemoryCandidate::Episode(candidate) = candidate {
            if candidate.draft.id == Some(id(n)) {
                candidate.draft.scene.as_mut().unwrap().time = at;
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

#[tokio::test]
async fn time_range_without_input_baseline() {
    let mut rows = Vec::new();
    for reverse in [false, true] {
        let (memory, temp) = open().await;
        commit(&memory, range_fixture(reverse)).await;
        for topic in ["what happened last Tuesday", "what happened today"] {
            let mut context = query(true, false);
            context.topic = Some(topic.to_owned());
            let result = record(
                &mut rows,
                &format!("{topic}-reverse={reverse}"),
                &memory,
                context,
            )
            .await;
            assert_eq!(episodes(&result), (2000..2008).collect::<Vec<_>>());
            assert_eq!(roots(&result), (2000..2012).collect::<Vec<_>>());
        }
        let mut quiet = query(true, false);
        quiet.section_limits = room(24);
        quiet.candidate_limits.max_vector_candidates = 2;
        quiet.candidate_limits.max_graph_roots = 64;
        record(
            &mut rows,
            &format!("quiet-room-reverse={reverse}"),
            &memory,
            quiet,
        )
        .await;
        memory.close().await.unwrap();
        temp.close().unwrap();
    }
    println!(
        "TIME_RANGE_BASELINE={}",
        serde_json::to_string(&rows).unwrap()
    );
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
async fn time_range_reserves_the_requested_day_under_topic_pressure() {
    let mut rows = Vec::new();
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
            let zero = record(
                &mut rows,
                &format!("zero-floor-{topic}-reverse={reverse}"),
                &memory,
                context.clone(),
            )
            .await;
            assert_eq!(episodes(&zero), (2000..2008).collect::<Vec<_>>());
            assert!(date_match_episodes(&zero).is_empty());
            context.cue_floors.date_match = 2;
            let result = record(
                &mut rows,
                &format!("{topic}-reverse={reverse}"),
                &memory,
                context.clone(),
            )
            .await;
            assert_eq!(result.time_range, context.time_range);
            assert_eq!(date_match_episodes(&result), expected);
            assert_eq!(
                episodes(&result),
                (2000..2006).chain(expected).collect::<Vec<_>>()
            );
            assert_eq!(
                result.trace.as_ref().unwrap().time_range_has_more,
                Some(true)
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
    println!(
        "TIME_RANGE_PRESSURE={}",
        serde_json::to_string(&rows).unwrap()
    );
}

#[tokio::test]
async fn time_range_preserves_other_cues_when_there_is_room() {
    let mut rows = Vec::new();
    for reverse in [false, true] {
        let (memory, temp) = open().await;
        commit(&memory, range_fixture(reverse)).await;
        let mut context = query(true, false);
        context.section_limits = room(24);
        context.candidate_limits.max_vector_candidates = 2;
        context.candidate_limits.max_graph_roots = 64;
        let without = record(
            &mut rows,
            &format!("quiet-without-reverse={reverse}"),
            &memory,
            context.clone(),
        )
        .await;
        let start = time() - Duration::days(6) - Duration::hours(18);
        context = context.with_time_range(start, start + Duration::hours(23));
        context.cue_floors.date_match = 2;
        let with = record(
            &mut rows,
            &format!("quiet-with-reverse={reverse}"),
            &memory,
            context,
        )
        .await;
        assert_eq!(with.pack, without.pack);
        for n in episodes(&without) {
            assert_eq!(scores(&with, n), scores(&without, n));
        }
        assert_eq!(
            date_match_episodes(&with),
            if reverse { [200, 201] } else { [104, 103] }
        );
        memory.close().await.unwrap();
        temp.close().unwrap();
    }
    println!("TIME_RANGE_QUIET={}", serde_json::to_string(&rows).unwrap());
}

#[tokio::test]
async fn time_range_overlap_keeps_standing_and_reminders_stay_on_the_occasion() {
    let mut rows = Vec::new();
    for topic in [false, true] {
        let (memory, temp) = open().await;
        commit(&memory, description_fixture(true)).await;
        let mut context = query(topic, false);
        context.section_limits = room(2);
        context.candidate_limits.max_graph_roots = 1;
        let without = record(
            &mut rows,
            &format!("overlap-without-topic={topic}"),
            &memory,
            context.clone(),
        )
        .await;
        context = context.with_time_range(time(), time());
        let with = record(
            &mut rows,
            &format!("overlap-with-topic={topic}"),
            &memory,
            context,
        )
        .await;
        assert_eq!(with.pack, without.pack);
        assert_eq!(roots(&with), [900]);
        assert_eq!(date_match_episodes(&with), [900]);
        assert_eq!(scores(&with, 900), scores(&without, 900));
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
    println!(
        "TIME_RANGE_OVERLAP={}",
        serde_json::to_string(&rows).unwrap()
    );
}

#[tokio::test]
async fn time_range_uses_both_ends_without_the_scene_reference_cut() {
    let (memory, temp) = open().await;
    let start = time() + Duration::days(1);
    let end = start + Duration::milliseconds(125);
    let mut plan = RememberWritePlan::new();
    for (n, at) in [
        (99, start - Duration::milliseconds(1)),
        (900, start),
        (100, end),
        (98, end + Duration::milliseconds(1)),
    ] {
        plan = range_episode(plan, n, at);
    }
    commit(&memory, plan).await;
    let mut rows = Vec::new();
    for (case, range_start, range_end, expected) in [
        ("future-inclusive", start, end, vec![100, 900]),
        ("point-inclusive", start, start, vec![900]),
        ("inverted", end, start, vec![]),
        ("empty", time(), time(), vec![]),
    ] {
        let mut context = query(false, false).with_time_range(range_start, range_end);
        context.cue_floors.date_match = 2;
        let with = record(&mut rows, case, &memory, context.clone()).await;
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
    println!(
        "TIME_RANGE_BOUNDARIES={}",
        serde_json::to_string(&rows).unwrap()
    );
}

#[tokio::test]
async fn time_range_floor_bounds_contribution_after_lifecycle_filtering() {
    let (memory, temp) = open().await;
    let start = time() + Duration::days(1);
    let end = start + Duration::hours(2);
    let mut plan = RememberWritePlan::new();
    for (n, hours) in [(300, 0), (200, 1), (100, 2)] {
        plan = range_episode(plan, n, start + Duration::hours(hours));
    }
    commit(&memory, plan).await;
    let mut rows = Vec::new();
    for floor in [0, 1, 2, 3, 7] {
        let mut context = query(false, false).with_time_range(start, end);
        context.cue_floors.date_match = floor;
        let result = record(
            &mut rows,
            &format!("contribution-floor-{floor}"),
            &memory,
            context,
        )
        .await;
        let count = floor.clamp(1, 3);
        assert_eq!(episodes(&result), [100, 200, 300][..count]);
        assert_eq!(date_match_episodes(&result), [100, 200, 300][..count]);
        assert_eq!(
            result.trace.as_ref().unwrap().time_range_has_more,
            Some(floor < 3)
        );
    }
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::Episode(id(100)),
            "range eligibility",
        ))
        .await
        .unwrap();
    for include_suppressed in [false, true] {
        let mut context = query(false, false).with_time_range(start, end);
        context.cue_floors.date_match = 2;
        context.lifecycle_policy.include_suppressed = include_suppressed;
        let result = record(
            &mut rows,
            &format!("range-include-suppressed-{include_suppressed}"),
            &memory,
            context,
        )
        .await;
        assert_eq!(
            episodes(&result),
            if include_suppressed {
                [100, 200]
            } else {
                [200, 300]
            }
        );
        assert_eq!(
            result.trace.as_ref().unwrap().time_range_has_more,
            Some(include_suppressed)
        );
    }
    memory.close().await.unwrap();
    temp.close().unwrap();
    println!(
        "TIME_RANGE_FLOORS={}",
        serde_json::to_string(&rows).unwrap()
    );
}
