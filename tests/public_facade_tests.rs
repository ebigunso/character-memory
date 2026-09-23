use character_memory::{
    CorrectMemoryDraft, CorrectionTarget, DerivedMemoryDraft, DerivedType, EpisodeDraft,
    ForgetMemoryDraft, LifecycleTargetRef, MemoryId, ObservationDraft, RememberInput,
    RememberOptions, ReplacementDerivedMemoryDraft, RetrievalContext, SourceProvenanceReference,
};
use uuid::Uuid;

mod road_order {
    use async_trait::async_trait;
    use character_memory::api::types::{RetrievalCueFloors, TimeRange};
    use character_memory::*;
    use chrono::{DateTime, Duration, Utc};
    use serde_json::{json, Value};

    struct Provider;
    fn embedding(text: &str) -> Vec<f32> {
        let (axis, score) = if text == "topic" {
            (0, 1.0)
        } else if text == "perception" {
            (1, 1.0)
        } else {
            text.split_whitespace()
                .find_map(|word| {
                    word.strip_prefix("topic=")
                        .map(|score| (0, score.parse::<f32>().unwrap()))
                        .or_else(|| {
                            word.strip_prefix("scene=")
                                .map(|score| (1, score.parse::<f32>().unwrap()))
                        })
                })
                .unwrap_or((0, 0.0))
        };
        let mut vector = vec![0.0, 0.0, (1.0_f32 - score * score).max(0.0).sqrt()];
        vector[axis] = score;
        vector
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
    fn time() -> DateTime<Utc> {
        "2026-09-23T12:00:00Z".parse().unwrap()
    }
    fn id(n: u128, reverse: bool) -> MemoryId {
        MemoryId::from_u128(if reverse { 100_000 - n } else { n })
    }
    fn provenance() -> CandidateProvenance {
        CandidateProvenance::caller("road order fixture")
    }
    async fn open() -> (CharacterMemory, tempfile::TempDir) {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".agent-work/worker/task1/stores");
        std::fs::create_dir_all(&directory).unwrap();
        let root = tempfile::tempdir_in(directory).unwrap();
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
            format!("road_{}", MemoryId::new_v4()),
            Box::new(Provider),
        )
        .await
        .unwrap();
        (memory, root)
    }
    fn episode(
        plan: RememberWritePlan,
        n: u128,
        days: i64,
        salience: f32,
        score: Option<f32>,
        reverse: bool,
    ) -> RememberWritePlan {
        let mut draft = EpisodeDraft::new(score.map_or_else(
            || format!("occasion {n}"),
            |s| format!("occasion {n} topic={s}"),
        ));
        draft.id = Some(id(n, reverse));
        draft.scene = Some(Scene::at((time() - Duration::days(days)).fixed_offset()));
        draft.created_at = Some(time() + Duration::seconds(n as i64));
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
        draft.salience_score = salience;
        let plan = plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
            draft,
            provenance(),
        )));
        if score.is_some() {
            indexed(plan, ObjectType::Episode, n, reverse)
        } else {
            plan
        }
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
    fn entity(plan: RememberWritePlan, n: u128, reverse: bool) -> RememberWritePlan {
        let mut draft = EntityDraft::new();
        draft.id = Some(id(n, reverse));
        draft.created_at = Some(time());
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
        plan.with_candidate(MemoryCandidate::Entity(EntityCandidate::new(
            draft,
            provenance(),
        )))
    }
    async fn commit(memory: &CharacterMemory, plan: RememberWritePlan) {
        let result = memory.commit(plan, CommitOptions::default()).await.unwrap();
        assert!(result.vector_indexing_failure.is_none(), "{result:?}");
        assert!(result.diagnostics.messages.is_empty(), "{result:?}");
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
    fn query(topic: bool, roots: usize, cap: usize) -> RetrievalContext {
        let mut context = RetrievalContext::default()
            .with_scene(Scene::at(time().fixed_offset()))
            .with_trace();
        context.topic = topic.then(|| "topic".into());
        context.candidate_limits.max_graph_roots = roots;
        context.candidate_limits.max_vector_candidates = 32;
        context.section_limits = room(cap);
        context.graph_limits.max_depth = 0;
        context.graph_limits.timeout_ms = None;
        context.cue_floors = RetrievalCueFloors {
            participant: 0,
            place: 0,
            activity: 0,
            topic: 0,
            recency: 0,
            date_match: 0,
        };
        context
    }
    fn root_ids(result: &RetrieveOutcome) -> Vec<MemoryId> {
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
    fn snapshot(
        case: &str,
        reverse: bool,
        context: &RetrievalContext,
        result: &RetrieveOutcome,
    ) -> Value {
        assert_eq!(result.scene, context.scene);
        assert_eq!(result.time_range, context.time_range);
        let trace = result.trace.as_ref().unwrap();
        json!({"case":case,"reverse":reverse,"input":context,"roots":root_ids(result),"pack":result.pack,
            "echo":{"scene":result.scene,"activity":result.activity,"time_range":result.time_range,"scene_references":result.scene_references,"memory_scenes":result.memory_scenes},
            "assignments":trace.section_assignments,"floors":trace.floor_admissions,"candidates":trace.vector_candidates,
            "expansions":trace.graph_expansions,"telemetry":result.rationale.telemetry})
    }
    async fn capture(
        rows: &mut Vec<Value>,
        case: &str,
        reverse: bool,
        memory: &CharacterMemory,
        context: RetrievalContext,
    ) -> RetrieveOutcome {
        let result = memory.retrieve(context.clone()).await.unwrap();
        rows.push(snapshot(case, reverse, &context, &result));
        result
    }
    fn emit(rows: &[Value]) {
        println!("ROAD_BASELINE={}", serde_json::to_string(rows).unwrap());
        let mut failures = Vec::new();
        for row in rows {
            let reverse = row["reverse"].as_bool().unwrap();
            let case = row["case"].as_str().unwrap();
            let tied = if reverse {
                vec![101, 100]
            } else {
                vec![100, 101]
            };
            let day = if reverse { vec![21, 20] } else { vec![20, 21] };
            let expected = match case {
                "range-last-tuesday" => day,
                "no-range" => vec![10, 11],
                "saturated-topic-parity" | "description-spare" | "setting-description-spare" => {
                    vec![100, 101]
                }
                "recency-salience-cap" => vec![200, 300],
                "memory-clock" => {
                    if reverse {
                        vec![101, 100, 201, 200, 301, 300]
                    } else {
                        vec![100, 101, 200, 201, 300, 301]
                    }
                }
                "scope-own-head" => vec![301, 311, 600],
                "time-plan-topic-parity" => (2000..2012).collect(),
                "latest-recency-floor" => vec![100, 10],
                "recency-floor-overlap" => vec![900],
                "zero-cap" | "negative-cap" => vec![900, tied[0]],
                "positive-overlap" => tied,
                "clamped-overlap" => vec![101, 100],
                "section-margin" => vec![100, 101, 900],
                "ordinary-anniversary" => (1000..1012).collect(),
                "salient-anniversary" => std::iter::once(1365).chain(1000..1011).collect(),
                "shared-anniversary" => std::iter::once(10)
                    .chain(1000..1010)
                    .chain([1365])
                    .collect(),
                "anniversary-separate-lists" => vec![10, 200, 201, 202, 300, 301, 302],
                "cross-kind-one" => vec![10, 30, 40],
                _ => panic!("unasserted case {case}"),
            };
            let actual = row["roots"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| {
                    let id: MemoryId = serde_json::from_value(value.clone()).unwrap();
                    if reverse {
                        100_000 - id.as_u128()
                    } else {
                        id.as_u128()
                    }
                })
                .collect::<Vec<_>>();
            if actual != expected {
                failures.push(format!(
                    "{case} reverse={reverse}: roots {actual:?}, expected {expected:?}"
                ));
            }
            let episodes = match case {
                "cross-kind-one" | "scope-own-head" => Vec::new(),
                "time-plan-topic-parity" => (2000..2008).collect(),
                "zero-cap" | "negative-cap" | "positive-overlap" | "clamped-overlap" => {
                    vec![if reverse { 101 } else { 100 }]
                }
                "section-margin" => vec![100, 900],
                "shared-anniversary" => (1000..1010).chain([1365]).collect(),
                "anniversary-separate-lists" => vec![200, 201, 300],
                "memory-clock" => vec![101, 100],
                _ => expected,
            };
            for (section, expected) in [
                ("relevant_episodes", episodes),
                (
                    "salient_observations",
                    if case == "memory-clock" {
                        vec![201, 200]
                    } else {
                        Vec::new()
                    },
                ),
                (
                    "derived_memories",
                    match case {
                        "memory-clock" => vec![301, 300],
                        "cross-kind-one" => vec![30],
                        "scope-own-head" => vec![301, 311],
                        _ => Vec::new(),
                    },
                ),
                (
                    "active_threads",
                    match case {
                        "cross-kind-one" => vec![40],
                        "scope-own-head" => vec![600],
                        _ => Vec::new(),
                    },
                ),
            ] {
                let actual = row["pack"][section]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|entry| {
                        let value = if section == "derived_memories" {
                            &entry["memory"]["id"]
                        } else {
                            &entry["id"]
                        };
                        let id: MemoryId = serde_json::from_value(value.clone()).unwrap();
                        if reverse {
                            100_000 - id.as_u128()
                        } else {
                            id.as_u128()
                        }
                    })
                    .collect::<Vec<_>>();
                if actual != expected {
                    failures.push(format!(
                        "{case} reverse={reverse}: {section} {actual:?}, expected {expected:?}"
                    ));
                }
            }
            if case == "range-last-tuesday"
                && row["assignments"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|assignment| {
                        assignment["cue_kinds"]
                            .as_array()
                            .unwrap()
                            .contains(&json!("recency"))
                    })
            {
                failures.push(format!(
                    "{case} reverse={reverse}: recency was reported with a range"
                ));
            }
            if case == "negative-cap"
                && row["candidates"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|candidate| candidate["score"].as_f64().is_some_and(|score| score < 0.0))
            {
                failures.push(format!(
                    "{case} reverse={reverse}: a negative search score was not clamped"
                ));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[tokio::test]
    async fn time_plan_saturated_topic_parity() {
        let mut rows = Vec::new();
        for reverse in [false, true] {
            let (memory, root) = open().await;
            let mut plan = RememberWritePlan::new();
            for n in 0..20 {
                plan = episode(
                    plan,
                    2000 + n,
                    30 + n as i64,
                    0.0,
                    Some(0.99 - n as f32 * 0.01),
                    reverse,
                );
            }
            for n in 0..12 {
                plan = episode(plan, 1000 + n, n as i64, 0.5, None, reverse);
            }
            commit(&memory, plan).await;
            let mut context = query(true, 12, 8);
            context.cue_floors.topic = 1;
            capture(
                &mut rows,
                "time-plan-topic-parity",
                reverse,
                &memory,
                context,
            )
            .await;
            memory.close().await.unwrap();
            root.close().unwrap();
        }
        emit(&rows);
    }

    #[tokio::test]
    async fn window_floor_and_saturated_topic() {
        let mut rows = Vec::new();
        for reverse in [false, true] {
            let (memory, root) = open().await;
            let mut plan = RememberWritePlan::new();
            for (n, days, score) in [
                (10, 0, None),
                (11, 1, None),
                (20, 8, None),
                (21, 8, None),
                (100, 30, Some(0.9)),
                (101, 31, Some(0.8)),
            ] {
                plan = episode(plan, n, days, 0.0, score, reverse);
            }
            commit(&memory, plan).await;
            let mut requested = query(false, 4, 2);
            requested.time_range = Some(TimeRange {
                start: time() - Duration::days(8) - Duration::hours(1),
                end: time() - Duration::days(8) + Duration::hours(1),
            });
            capture(&mut rows, "range-last-tuesday", reverse, &memory, requested).await;
            capture(&mut rows, "no-range", reverse, &memory, query(false, 4, 2)).await;
            let mut saturated = query(true, 2, 2);
            saturated.cue_floors.topic = 1;
            capture(
                &mut rows,
                "saturated-topic-parity",
                reverse,
                &memory,
                saturated.clone(),
            )
            .await;
            saturated.cue_floors.recency = 1;
            capture(
                &mut rows,
                "latest-recency-floor",
                reverse,
                &memory,
                saturated,
            )
            .await;
            memory.close().await.unwrap();
            root.close().unwrap();
        }
        emit(&rows);
    }

    #[tokio::test]
    async fn zero_negative_positive_and_overlapping_search_roads() {
        let mut rows = Vec::new();
        for reverse in [false, true] {
            for (case, score, overlap) in [
                ("zero-cap", 0.0, false),
                ("negative-cap", -0.5, false),
                ("positive-overlap", 0.8, true),
                ("clamped-overlap", -0.5, true),
            ] {
                let (memory, root) = open().await;
                let mut plan =
                    episode(RememberWritePlan::new(), 100, 30, 0.0, Some(score), reverse);
                plan = episode(
                    plan,
                    101,
                    if overlap { 0 } else { 31 },
                    0.0,
                    Some(score),
                    reverse,
                );
                if !overlap {
                    plan = episode(plan, 900, 0, 0.0, None, reverse);
                }
                commit(&memory, plan).await;
                let mut context = query(true, 2, 1);
                context.candidate_limits.max_vector_candidates = 2;
                context.cue_floors.topic = 1;
                capture(&mut rows, case, reverse, &memory, context).await;
                memory.close().await.unwrap();
                root.close().unwrap();
            }
        }
        emit(&rows);
    }

    #[tokio::test]
    async fn description_spare_turn_and_section_margin() {
        let mut rows = Vec::new();
        for reverse in [false, true] {
            let (memory, root) = open().await;
            let mut plan = episode(RememberWritePlan::new(), 100, 30, 0.0, Some(0.99), reverse);
            plan = episode(plan, 101, 31, 0.0, Some(0.95), reverse);
            let mut described = EpisodeDraft::new("description occasion");
            described.id = Some(id(200, reverse));
            described.created_at = Some(time());
            described.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
            described.salience_score = 0.0;
            let mut scene = Scene::at((time() - Duration::days(2)).fixed_offset());
            scene.participants.push(SceneParticipant {
                description: Some("scene=0.8".into()),
                ..Default::default()
            });
            scene.setting.words = Some("scene=0.8".into());
            described.scene = Some(scene);
            plan = plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
                described,
                provenance(),
            )));
            plan = indexed(plan, ObjectType::Episode, 200, reverse);
            commit(&memory, plan).await;
            let mut context = query(true, 2, 2);
            context.cue_floors.topic = 1;
            context.scene.participants.push(SceneParticipant {
                description: Some("perception".into()),
                ..Default::default()
            });
            capture(&mut rows, "description-spare", reverse, &memory, context).await;
            let mut context = query(true, 2, 2);
            context.cue_floors.topic = 1;
            context.scene.setting.words = Some("perception".into());
            capture(
                &mut rows,
                "setting-description-spare",
                reverse,
                &memory,
                context,
            )
            .await;
            memory.close().await.unwrap();
            root.close().unwrap();

            let (memory, root) = open().await;
            let mut plan = episode(RememberWritePlan::new(), 100, 30, 0.0, Some(0.9), reverse);
            plan = episode(plan, 101, 31, 0.0, Some(0.1), reverse);
            plan = episode(plan, 900, 0, 1.0, None, reverse);
            commit(&memory, plan).await;
            let mut context = query(true, 3, 2);
            context.cue_floors.topic = 1;
            capture(&mut rows, "section-margin", reverse, &memory, context).await;
            memory.close().await.unwrap();
            root.close().unwrap();
        }
        emit(&rows);
    }

    #[tokio::test]
    async fn anniversary_year_deep_root_cap() {
        let mut rows = Vec::new();
        let ordinary = EpisodeDraft::new("ordinary").salience_score;
        for reverse in [false, true] {
            for (case, salience) in [
                ("ordinary-anniversary", ordinary),
                ("salient-anniversary", 1.0),
                ("shared-anniversary", ordinary),
            ] {
                let (memory, root) = open().await;
                let mut plan = RememberWritePlan::new();
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
                let mut context = query(false, 12, 12);
                if case == "shared-anniversary" {
                    plan = entity(plan, 10, reverse);
                    for candidate in &mut plan.candidates {
                        if let MemoryCandidate::Episode(candidate) = candidate {
                            if candidate.draft.id == Some(id(1365, reverse)) {
                                candidate.draft.scene.as_mut().unwrap().participants.push(
                                    SceneParticipant {
                                        key: Some(id(10, reverse)),
                                        ..Default::default()
                                    },
                                );
                            }
                        }
                    }
                    context.scene.participants.push(SceneParticipant {
                        key: Some(id(10, reverse)),
                        ..Default::default()
                    });
                    context.cue_floors.date_match = 1;
                }
                commit(&memory, plan).await;
                capture(&mut rows, case, reverse, &memory, context).await;
                memory.close().await.unwrap();
                root.close().unwrap();
            }
        }
        emit(&rows);
    }

    #[tokio::test]
    async fn given_roads_precede_an_equal_topic_hit() {
        let mut rows = Vec::new();
        for reverse in [false, true] {
            let (memory, root) = open().await;
            let mut source = EpisodeDraft::new("where the belief formed");
            source.id = Some(id(20, reverse));
            source.created_at = Some(time() - Duration::days(20));
            let mut scene = Scene::at((time() - Duration::days(20)).fixed_offset());
            scene.setting.key = Some("office".into());
            source.scene = Some(scene);
            let mut belief = DerivedMemoryDraft::new(DerivedType::Claim, "place belief")
                .with_source_episode(id(20, reverse));
            belief.id = Some(id(30, reverse));
            belief.created_at = Some(time() - Duration::days(19));
            belief.updated_at = belief.created_at;
            let mut thread = MemoryThreadDraft::new("activity", "activity");
            thread.id = Some(id(40, reverse));
            thread.created_at = Some(time());
            thread.updated_at = thread.created_at;
            thread.last_touched_at = thread.created_at;
            let input = RememberInput::new("given roads")
                .with_episode(source)
                .with_derived_memory(belief)
                .with_memory_thread(thread);
            let mut plan =
                input.prepare_write_plan(&RememberPlanDefaults::fixed("given roads", time()));
            plan = entity(plan, 10, reverse);
            plan = episode(plan, 100, 1, 0.0, Some(1.0), reverse);
            commit(&memory, plan).await;
            let mut context = query(true, 3, 3);
            context.scene.participants.push(SceneParticipant {
                key: Some(id(10, reverse)),
                ..Default::default()
            });
            context.scene.setting.key = Some("office".into());
            context.activity = Some(ActivityRef::Thread(id(40, reverse)));
            capture(&mut rows, "cross-kind-one", reverse, &memory, context).await;
            memory.close().await.unwrap();
            root.close().unwrap();
        }
        emit(&rows);
    }

    #[tokio::test]
    async fn recency_overlap_and_separate_anniversary_lists() {
        let mut rows = Vec::new();
        for reverse in [false, true] {
            let (memory, root) = open().await;
            let plan = episode(
                episode(RememberWritePlan::new(), 100, 1, 0.0, Some(0.9), reverse),
                900,
                0,
                0.0,
                None,
                reverse,
            );
            commit(&memory, plan).await;
            let mut context = query(true, 1, 2);
            context.cue_floors.recency = 1;
            capture(
                &mut rows,
                "recency-floor-overlap",
                reverse,
                &memory,
                context,
            )
            .await;
            memory.close().await.unwrap();
            root.close().unwrap();

            let (memory, root) = open().await;
            let mut plan = entity(RememberWritePlan::new(), 10, reverse);
            for (n, days) in [(100, 0), (101, 1), (102, 2)] {
                plan = episode(plan, n, days, 0.0, None, reverse);
            }
            for (n, year, shared) in [
                (200, 2025, false),
                (201, 2024, false),
                (202, 2023, false),
                (300, 2022, true),
                (301, 2021, true),
                (302, 2020, true),
            ] {
                let mut draft = EpisodeDraft::new(format!("anniversary {n}"));
                draft.id = Some(id(n, reverse));
                draft.created_at = Some(time());
                draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
                let mut scene = Scene::at(format!("{year}-09-23T12:00:00Z").parse().unwrap());
                if shared {
                    scene.participants.push(SceneParticipant {
                        key: Some(id(10, reverse)),
                        ..Default::default()
                    });
                }
                draft.scene = Some(scene);
                draft.salience_score = 1.0;
                plan = plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
                    draft,
                    provenance(),
                )));
            }
            commit(&memory, plan).await;
            let mut context = query(false, 7, 3);
            context.scene.participants.push(SceneParticipant {
                key: Some(id(10, reverse)),
                ..Default::default()
            });
            context.cue_floors.date_match = 1;
            capture(
                &mut rows,
                "anniversary-separate-lists",
                reverse,
                &memory,
                context,
            )
            .await;
            memory.close().await.unwrap();
            root.close().unwrap();
        }
        emit(&rows);
    }
    #[tokio::test]
    async fn salience_memory_clock_and_scope_head_ignore_identifier_direction() {
        let mut rows = Vec::new();
        for reverse in [false, true] {
            let (memory, root) = open().await;
            let mut plan = RememberWritePlan::new();
            for (n, days, salience) in [(100, 0, 0.0), (200, 1, 1.0), (300, 2, 1.0)] {
                plan = episode(plan, n, days, salience, None, reverse);
            }
            commit(&memory, plan).await;
            capture(
                &mut rows,
                "recency-salience-cap",
                reverse,
                &memory,
                query(false, 2, 3),
            )
            .await;
            memory.close().await.unwrap();
            root.close().unwrap();

            let (memory, root) = open().await;
            let mut plan = episode(RememberWritePlan::new(), 100, 10, 0.0, Some(0.8), reverse);
            plan = episode(plan, 101, 3, 0.0, Some(0.8), reverse);
            for n in [200, 201] {
                let mut draft = ObservationDraft::new(id(100, reverse), "topic=0.8");
                draft.id = Some(id(n, reverse));
                draft.observed_at = (n == 201).then_some(time() - Duration::days(1));
                // Oppose creation time to the observation/parent clock.
                draft.created_at = Some(time() - Duration::days(if n == 200 { 0 } else { 20 }));
                draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
                draft.salience_score = 0.0;
                plan = plan.with_candidate(MemoryCandidate::Observation(
                    ObservationCandidate::new(draft, provenance()),
                ));
                plan = indexed(plan, ObjectType::Observation, n, reverse);
            }
            for (n, days) in [(300, 5), (301, 1)] {
                let mut draft = DerivedMemoryDraft::new(DerivedType::Claim, "topic=0.8")
                    .with_source_episode(id(100, reverse));
                draft.id = Some(id(n, reverse));
                draft.created_at = Some(time() - Duration::days(days));
                draft.updated_at = draft.created_at;
                draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
                draft.salience_score = 0.0;
                plan = plan.with_candidate(MemoryCandidate::DerivedMemory(
                    DerivedMemoryCandidate::new(draft, provenance()),
                ));
                plan = indexed(plan, ObjectType::DerivedMemory, n, reverse);
            }
            commit(&memory, plan).await;
            capture(
                &mut rows,
                "memory-clock",
                reverse,
                &memory,
                query(true, 6, 6),
            )
            .await;
            memory.close().await.unwrap();
            root.close().unwrap();

            let (memory, root) = open().await;
            let mut thread = MemoryThreadDraft::new("work", "work");
            thread.id = Some(id(600, reverse));
            thread.created_at = Some(time());
            thread.updated_at = Some(time());
            thread.last_touched_at = Some(time());
            thread.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
            commit(
                &memory,
                RememberWritePlan::new().with_candidate(MemoryCandidate::MemoryThread(
                    MemoryThreadCandidate::new(thread, provenance()),
                )),
            )
            .await;
            for (source, member, place, custom) in [
                (101, 301, true, false),
                (102, 311, false, true),
                (103, 321, false, false),
            ] {
                let mut scene = Scene::at((time() - Duration::days(10)).fixed_offset());
                if place {
                    scene.setting.key = Some("office".into());
                }
                if custom {
                    scene.custom_values.insert("a".into(), "42".into());
                    scene.custom_values.insert("z".into(), "42".into());
                }
                let mut source_draft = EpisodeDraft::new("scope source");
                source_draft.id = Some(id(source, reverse));
                source_draft.scene = Some(scene);
                let mut draft = DerivedMemoryDraft::new(DerivedType::Claim, "scope belief")
                    .with_source_episode(id(source, reverse));
                draft.id = Some(id(member, reverse));
                draft.salience_score = if place { 1.0 } else { 0.5 };
                if member == 321 {
                    draft.thread_ids.push(id(600, reverse));
                }
                let mut input = RememberInput::new(format!("scope cue {source}"))
                    .with_episode(source_draft)
                    .with_derived_memory(draft);
                for n in if place {
                    vec![302, 303, 304]
                } else if custom {
                    vec![312]
                } else {
                    Vec::new()
                } {
                    let mut extra =
                        DerivedMemoryDraft::new(DerivedType::Claim, "additional scope belief")
                            .with_source_episode(id(source, reverse));
                    extra.id = Some(id(n, reverse));
                    extra.created_at = Some(time() - Duration::seconds(1));
                    extra.updated_at = extra.created_at;
                    extra.salience_score = 0.5;
                    input = input.with_derived_memory(extra);
                }
                commit(
                    &memory,
                    input.prepare_write_plan(&RememberPlanDefaults::fixed(
                        format!("scope cue {source}"),
                        time(),
                    )),
                )
                .await;
            }
            let mut context = query(false, 3, 8);
            context.cue_floors = RetrievalCueFloors::default();
            context.scene.setting.key = Some("office".into());
            context.scene.custom_values.insert("a".into(), "42".into());
            context.scene.custom_values.insert("z".into(), "42".into());
            context.activity = Some(ActivityRef::Thread(id(600, reverse)));
            capture(&mut rows, "scope-own-head", reverse, &memory, context).await;
            memory.close().await.unwrap();
            root.close().unwrap();
        }
        emit(&rows);
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
