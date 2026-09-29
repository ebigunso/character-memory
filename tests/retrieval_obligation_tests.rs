use character_memory::*;
use serde_json::{json, Value};
use test_support::{id, keyed};

mod trigger_road {
    use super::*;

    fn belief(
        n: u128,
        kind: DerivedType,
        subjects: &[u128],
        salience: f32,
        text: &str,
    ) -> DerivedMemoryDraft {
        let mut draft = DerivedMemoryDraft::new(kind, text);
        draft.id = Some(id(n));
        draft.entity_ids = subjects.iter().copied().map(id).collect();
        draft.given_by_application = true;
        draft.salience_score = salience;
        draft.created_at = Some(now() - chrono::Duration::minutes(n as i64));
        draft.updated_at = draft.created_at;
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
        draft
    }

    fn open(n: u128, party: u128, salience: f32) -> DerivedMemoryDraft {
        let mut draft = belief(
            n,
            DerivedType::OpenLoop,
            &[1, party],
            salience,
            "open obligation",
        );
        draft.assertions = vec![
            role(1, BeliefPredicate::Actor),
            role(party, BeliefPredicate::Counterpart),
        ];
        draft
    }

    async fn fixture(drafts: Vec<DerivedMemoryDraft>) -> (CharacterMemory, tempfile::TempDir) {
        let root = tempfile::tempdir().unwrap();
        let memory = test_support::open_with_provider(
            test_support::persistent_settings(root.path()),
            test_support::unique_collection_name(),
            test_support::TestEmbeddingProvider::new(2, |text: &str| {
                if text.contains("loud") {
                    vec![1.0, 0.0]
                } else if text.contains("open") {
                    vec![0.0, 1.0]
                } else {
                    vec![-1.0, 0.0]
                }
            }),
            id(1),
        )
        .await
        .unwrap();
        let mut plan = RememberWritePlan::new();
        for n in 1..=4 {
            let mut entity = EntityDraft::new();
            entity.id = Some(id(n));
            entity.created_at = Some(now());
            entity.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
            plan.candidates
                .push(MemoryCandidate::Entity(EntityCandidate::new(
                    entity,
                    CandidateProvenance::caller("notion"),
                )));
        }
        let mut name = belief(9, DerivedType::Claim, &[2], 1.0, "Known as Bob");
        name.assertions = vec![role(2, BeliefPredicate::KnownAs { name: "Bob".into() })];
        for draft in std::iter::once(name).chain(drafts) {
            plan.candidates
                .push(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                    MemoryObjectRef::new(ObjectType::DerivedMemory, draft.id.unwrap()),
                    CandidateProvenance::caller("content"),
                )));
            plan.candidates
                .push(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
                    draft,
                    CandidateProvenance::caller("belief"),
                )));
        }
        let written = memory.commit(plan, CommitOptions::default()).await.unwrap();
        assert!(written.vector_indexing_failure.is_none());
        (memory, root)
    }

    fn request(
        participants: Vec<SceneParticipant>,
        topic: Option<&str>,
        cap: usize,
    ) -> RetrievalContext {
        let mut scene = Scene::at(now().fixed_offset());
        scene.participants = participants;
        let mut query = RetrievalContext::default().with_scene(scene).with_trace();
        query.topic = topic.map(str::to_owned);
        query.candidate_limits.max_graph_roots = cap;
        query.candidate_limits.max_vector_candidates = cap;
        query.section_limits = ContinuitySectionLimits {
            active_threads: cap,
            relevant_episodes: cap,
            salient_observations: cap,
            derived_memories: cap,
            preferences: cap,
            relationship_notes: cap,
            open_loops: cap,
            commitments: cap,
            character_signals: cap,
        };
        query
    }

    fn record(case: &str, result: &RetrieveOutcome) {
        println!(
            "TRIGGER_WITNESS={}",
            json!({
                "case": case, "selection": selection(result),
                "expansions": result.trace.as_ref().unwrap().graph_expansions,
                "relations": result.trace.as_ref().unwrap().graph_relations,
                "lifecycle": result.trace.as_ref().unwrap().lifecycle_filter_decisions,
                "floors": result.trace.as_ref().unwrap().floor_admissions,
            })
        );
    }

    fn assignment(result: &RetrieveOutcome, n: u128) -> &SectionAssignment {
        result
            .trace
            .as_ref()
            .unwrap()
            .section_assignments
            .iter()
            .find(|entry| entry.object.id == id(n))
            .unwrap()
    }

    fn scores(result: &RetrieveOutcome, n: u128) -> SectionScoreComponents {
        match assignment(result, n).reason {
            SectionAssignmentReason::Selected { scores }
            | SectionAssignmentReason::OmittedByLimit { scores, .. } => scores,
            _ => panic!("memory has no score"),
        }
    }

    fn admitted(result: &RetrieveOutcome, n: u128) -> Option<&MemoryScenes> {
        result
            .memory_scenes
            .iter()
            .find(|entry| entry.memory.id == id(n))
    }

    fn expanded(result: &RetrieveOutcome) -> Vec<MemoryId> {
        result
            .trace
            .as_ref()
            .unwrap()
            .graph_expansions
            .iter()
            .filter(|entry| entry.outcome == GraphExpansionOutcome::Expanded)
            .map(|entry| entry.root.id)
            .collect()
    }

    fn close_score(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.000001,
            "{actual} != {expected}"
        );
    }

    #[tokio::test]
    async fn crowded_party_parent_witness() {
        let mut drafts = vec![open(20, 2, 0.1)];
        drafts.extend(
            (100..132)
                .map(|n| belief(n, DerivedType::Claim, &[2], 0.9, "current belief about Bob")),
        );
        drafts.extend((200..232).map(|n| belief(n, DerivedType::Claim, &[4], 1.0, "loud topic")));
        let (memory, root) = fixture(drafts).await;
        let cases = [
            ("crowded_key", vec![keyed(2)], None),
            (
                "crowded_name",
                vec![SceneParticipant {
                    name: Some("Bob".into()),
                    ..Default::default()
                }],
                None,
            ),
            ("crowded_loud", vec![keyed(2)], Some("loud")),
            ("crowded_absent", vec![], None),
            (
                "crowded_description",
                vec![SceneParticipant {
                    description: Some("Bob".into()),
                    ..Default::default()
                }],
                None,
            ),
            ("crowded_self", vec![keyed(1), keyed(2)], None),
        ];
        for (case, participants, topic) in cases {
            let result = memory
                .retrieve(request(participants, topic, 3))
                .await
                .unwrap();
            record(case, &result);
            if matches!(case, "crowded_key" | "crowded_name" | "crowded_loud") {
                assert!(
                    result
                        .pack
                        .open_loops
                        .iter()
                        .any(|item| item.memory.id == id(20)),
                    "{case}"
                );
                assert!(admitted(&result, 20)
                    .unwrap()
                    .admitted_by
                    .contains(&AdmissionRoad::Participant));
                assert!(assignment(&result, 20)
                    .cue_kinds
                    .contains(&CueKind::Trigger));
                assert!(result
                    .trace
                    .as_ref()
                    .unwrap()
                    .graph_expansions
                    .iter()
                    .any(|entry| entry.root.id == id(20)
                        && entry.source == GraphRootSource::Trigger
                        && entry.outcome == GraphExpansionOutcome::Expanded));
                if case == "crowded_loud" {
                    let mut query = request(vec![keyed(2)], Some("loud"), 3);
                    query.cue_floors.trigger = 0;
                    assert!(admitted(&memory.retrieve(query).await.unwrap(), 20).is_none());
                }
            } else if case == "crowded_self" {
                assert!(assignment(&result, 20)
                    .cue_kinds
                    .contains(&CueKind::Trigger));
            } else {
                assert!(admitted(&result, 20).is_none());
                assert!(result
                    .trace
                    .as_ref()
                    .unwrap()
                    .graph_expansions
                    .iter()
                    .all(|entry| entry.source != GraphRootSource::Trigger));
            }
        }
        test_support::close_and_remove_root(memory, root).await;
    }

    #[tokio::test]
    async fn direct_root_score_and_role_free_parent_witness() {
        for roles in [false, true] {
            let mut draft = open(20, 2, 0.5);
            if !roles {
                draft.assertions.clear();
            }
            let (memory, root) = fixture(vec![draft]).await;
            for (label, topic) in [
                ("participant_descendant", None),
                ("also_topic_root", Some("open")),
            ] {
                let result = memory
                    .retrieve(request(vec![keyed(2)], topic, 8))
                    .await
                    .unwrap();
                record(&format!("{label}_roles_{roles}"), &result);
                assert!(result
                    .pack
                    .open_loops
                    .iter()
                    .any(|item| item.memory.id == id(20)));
                let score = scores(&result, 20);
                close_score(
                    score.final_score,
                    if topic.is_some() {
                        0.95
                    } else if roles {
                        0.7875
                    } else {
                        0.6625
                    },
                );
                assert_eq!(
                    score.cue_score,
                    Some(if topic.is_some() { 1.0 } else { 0.75 })
                );
                assert_eq!(
                    assignment(&result, 20)
                        .cue_kinds
                        .contains(&CueKind::Trigger),
                    roles
                );
            }
            test_support::close_and_remove_root(memory, root).await;
        }
    }

    async fn link(memory: &CharacterMemory, n: u128, from: u128, relation: RelationType, to: u128) {
        let mut draft = MemoryLinkDraft::new(
            ObjectType::DerivedMemory,
            id(from),
            relation,
            ObjectType::DerivedMemory,
            id(to),
        );
        draft.id = Some(id(n));
        draft.created_at = Some(now());
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
        memory.link(draft).await.unwrap();
    }

    async fn write_belief(memory: &CharacterMemory, draft: DerivedMemoryDraft) {
        let plan = RememberWritePlan::new()
            .with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                MemoryObjectRef::new(ObjectType::DerivedMemory, draft.id.unwrap()),
                CandidateProvenance::caller("content"),
            )))
            .with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
                draft,
                CandidateProvenance::caller("belief"),
            )));
        memory.commit(plan, CommitOptions::default()).await.unwrap();
    }

    async fn occasion(memory: &CharacterMemory, n: u128, days: i64, observation: bool) {
        let mut episode = EpisodeDraft::new("a recorded source");
        episode.id = Some(id(n));
        let mut scene = Scene::at((now() - chrono::Duration::days(days)).fixed_offset());
        scene.setting.key = Some("kitchen".into());
        episode.scene = Some(scene);
        episode.created_at = Some(now());
        episode.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
        let mut plan = RememberWritePlan::new().with_candidate(MemoryCandidate::Episode(
            EpisodeCandidate::new(episode, CandidateProvenance::caller("occasion")),
        ));
        if observation {
            let mut draft = ObservationDraft::new(id(n), "a remembered detail");
            draft.id = Some(id(n + 1));
            draft.created_at = Some(now());
            draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
            plan.candidates
                .push(MemoryCandidate::Observation(ObservationCandidate::new(
                    draft,
                    CandidateProvenance::caller("detail"),
                )));
        }
        memory.commit(plan, CommitOptions::default()).await.unwrap();
    }

    #[tokio::test]
    async fn hop_and_mixed_provenance_parent_witness() {
        let mut drafts = (100..132)
            .map(|n| belief(n, DerivedType::Claim, &[2], 0.9, "current belief about Bob"))
            .collect::<Vec<_>>();
        drafts.extend((200..208).map(|n| belief(n, DerivedType::Claim, &[4], 1.0, "loud topic")));
        drafts.extend([
            belief(30, DerivedType::Claim, &[3], 0.5, "direct objection"),
            belief(31, DerivedType::Claim, &[3], 0.5, "distant detail"),
        ]);
        let (memory, root) = fixture(drafts).await;
        occasion(&memory, 300, 30, false).await;
        let mut obligation = open(20, 2, 0.1);
        obligation.given_by_application = false;
        obligation.derived_from_episode_ids = vec![id(300)];
        write_belief(&memory, obligation).await;
        let mut source = MemoryLinkDraft::new(
            ObjectType::DerivedMemory,
            id(20),
            RelationType::DerivedFrom,
            ObjectType::Episode,
            id(300),
        );
        source.id = Some(id(802));
        source.created_at = Some(now());
        source.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
        memory.link(source).await.unwrap();
        link(&memory, 800, 20, RelationType::Supports, 30).await;
        link(&memory, 801, 30, RelationType::Supports, 31).await;
        for depth in [0, 1, 3] {
            for room in [3, 128] {
                let mut query = request(vec![keyed(2)], Some("loud"), 3);
                query.section_limits.derived_memories = room;
                query.graph_limits.max_depth = depth;
                let result = memory.retrieve(query).await.unwrap();
                record(&format!("hop_depth_{depth}_room_{room}"), &result);
                assert!(expanded(&result).contains(&id(20)));
                assert!(admitted(&result, 31).is_none());
                assert!(!result
                    .trace
                    .as_ref()
                    .unwrap()
                    .graph_relations
                    .iter()
                    .any(|entry| entry.link_id == id(801)));
                if depth == 0 {
                    assert!(admitted(&result, 300).is_none());
                    assert!(admitted(&result, 30).is_none());
                } else {
                    let source = admitted(&result, 300).unwrap();
                    assert_eq!(source.admitted_by, [AdmissionRoad::Participant].into());
                    for n in [30, 300] {
                        assert_eq!(scores(&result, n).cue_score, Some(0.0));
                        assert!(assignment(&result, n).cue_kinds.is_empty());
                        assert!(!result
                            .trace
                            .as_ref()
                            .unwrap()
                            .floor_admissions
                            .iter()
                            .any(|entry| entry.object.id == id(n)));
                    }
                    assert_eq!(admitted(&result, 30).is_some(), room == 128);
                    if room == 128 {
                        assert_eq!(
                            admitted(&result, 30).unwrap().admitted_by,
                            [AdmissionRoad::Participant].into()
                        );
                    }
                    assert!(result
                        .trace
                        .as_ref()
                        .unwrap()
                        .graph_relations
                        .iter()
                        .any(|entry| entry.link_id == id(802) && entry.proximity == 1));
                }
            }
        }
        for place in [false, true] {
            let mut query = request(vec![keyed(2)], Some("open"), 4);
            query.graph_limits.max_depth = 3;
            if place {
                query.scene.setting.key = Some("kitchen".into());
            }
            let result = memory.retrieve(query).await.unwrap();
            record(&format!("mixed_topic_place_{place}"), &result);
            for n in [30, 300] {
                assert_eq!(
                    admitted(&result, n).unwrap().admitted_by,
                    [AdmissionRoad::Participant, AdmissionRoad::Topic].into()
                );
                assert!(!assignment(&result, n).cue_kinds.contains(&CueKind::Trigger));
            }
            assert_eq!(
                assignment(&result, 20).cue_kinds.contains(&CueKind::Place),
                place
            );
        }
        test_support::close_and_remove_root(memory, root).await;
    }

    #[tokio::test]
    async fn reservation_and_displacement_parent_witness() {
        let (memory, root) = fixture(vec![open(20, 2, 0.5)]).await;
        occasion(&memory, 300, 2, true).await;
        occasion(&memory, 400, 1, true).await;
        let result = memory
            .retrieve(request(vec![keyed(2)], None, 3))
            .await
            .unwrap();
        record("root_seat_displacement", &result);
        assert_eq!(expanded(&result), vec![id(2), id(20), id(400)]);
        assert!(admitted(&result, 300).is_none());
        assert!(admitted(&result, 301).is_none());
        close_score(scores(&result, 400).final_score, 0.3);
        close_score(scores(&result, 401).final_score, 0.175);
        test_support::close_and_remove_root(memory, root).await;

        let mut drafts = vec![
            open(20, 2, 0.9),
            open(21, 2, 0.8),
            open(22, 3, 0.7),
            open(23, 3, 0.6),
            open(24, 2, 0.9),
        ];
        drafts.extend((100..132).map(|n| {
            belief(
                n,
                DerivedType::Claim,
                &[2, 3],
                1.0,
                "current belief about both",
            )
        }));
        drafts.extend((200..208).map(|n| belief(n, DerivedType::Claim, &[4], 1.0, "loud topic")));
        let (memory, root) = fixture(drafts).await;
        for participants in [vec![keyed(2), keyed(3)], vec![keyed(3), keyed(2)]] {
            let first = participants[0].key.unwrap();
            let mut query = request(participants, Some("loud"), 4);
            query.cue_floors.trigger = 2;
            query.section_limits.open_loops = 2;
            let result = memory.retrieve(query).await.unwrap();
            record(&format!("two_parties_first_{first}"), &result);
            assert_eq!(
                result
                    .pack
                    .open_loops
                    .iter()
                    .map(|item| item.memory.id)
                    .collect::<Vec<_>>(),
                vec![id(20), id(22)]
            );
            for n in [20, 22] {
                assert!(expanded(&result).contains(&id(n)));
                assert!(assignment(&result, n).cue_kinds.contains(&CueKind::Trigger));
            }
            assert!(
                !expanded(&result).contains(&id(24)),
                "the equally salient older obligation waits"
            );
            let mut ample = request(vec![keyed(2), keyed(3)], None, 8);
            ample.cue_floors.trigger = 2;
            ample.section_limits.open_loops = 2;
            let result = memory.retrieve(ample).await.unwrap();
            record("two_parties_tight_section", &result);
            assert_eq!(
                result
                    .pack
                    .open_loops
                    .iter()
                    .map(|item| item.memory.id)
                    .collect::<Vec<_>>(),
                vec![id(20), id(22)]
            );
        }
        test_support::close_and_remove_root(memory, root).await;
    }

    #[tokio::test]
    async fn lifecycle_parties_and_ambiguous_name_parent_witness() {
        let mut old = open(20, 2, 0.9);
        old.text = "open superseded obligation".into();
        let mut current = open(21, 2, 0.1);
        current.supersedes = vec![id(20)];
        let resolved = open(22, 2, 0.8);
        let mut other = open(23, 2, 0.5);
        other.entity_ids = vec![id(2), id(3)];
        other.assertions = vec![
            role(2, BeliefPredicate::Actor),
            role(3, BeliefPredicate::Counterpart),
        ];
        let mut future = open(24, 3, 0.5);
        future.text = "open matter due tomorrow".into();
        let mut alice_name = belief(25, DerivedType::Claim, &[3], 0.5, "Known as Bob too");
        alice_name.assertions = vec![role(3, BeliefPredicate::KnownAs { name: "Bob".into() })];
        let mut promise = open(27, 2, 0.4);
        promise.derived_type = DerivedType::Commitment;
        promise.text = "open promise due tomorrow".into();
        promise.assertions = vec![
            role(2, BeliefPredicate::Actor),
            role(1, BeliefPredicate::Counterpart),
        ];
        promise.created_at = Some(now() + chrono::Duration::days(1));
        promise.updated_at = promise.created_at;
        let (memory, root) = fixture(vec![
            old,
            resolved,
            other,
            future,
            alice_name,
            promise,
            belief(26, DerivedType::Claim, &[4], 0.5, "the matter is settled"),
        ])
        .await;
        write_belief(&memory, current).await;
        link(&memory, 800, 26, RelationType::Resolves, 22).await;
        for (case, participants, topic, superseded, cap) in [
            ("lifecycle_excluded", vec![keyed(2)], None, false, 8),
            ("lifecycle_ample", vec![keyed(2)], None, true, 8),
            ("lifecycle_tight", vec![keyed(2)], None, true, 2),
            ("resolved_topic", vec![keyed(2)], Some("open"), false, 12),
            (
                "ambiguous_name",
                vec![SceneParticipant {
                    name: Some("Bob".into()),
                    ..Default::default()
                }],
                None,
                false,
                8,
            ),
            ("self_ordinary", vec![keyed(1), keyed(2)], None, false, 8),
            ("future_due_party", vec![keyed(3)], None, false, 8),
        ] {
            let mut query = request(participants, topic, cap);
            query.lifecycle_policy.include_superseded = superseded;
            let result = memory.retrieve(query).await.unwrap();
            record(case, &result);
            assert!(!result
                .trace
                .as_ref()
                .unwrap()
                .graph_expansions
                .iter()
                .any(|entry| entry.root.id == id(22) && entry.source == GraphRootSource::Trigger));
            if case == "resolved_topic" {
                assert_eq!(
                    result
                        .pack
                        .derived_memories
                        .iter()
                        .find(|item| item.memory.id == id(22))
                        .unwrap()
                        .resolved_by,
                    vec![id(26)]
                );
            } else if case != "future_due_party" {
                assert!(result
                    .trace
                    .as_ref()
                    .unwrap()
                    .lifecycle_filter_decisions
                    .iter()
                    .any(|entry| entry.object.id == id(22)
                        && entry.reason == LifecycleFilterReason::ResolvedOmitted));
            }
            if matches!(
                case,
                "lifecycle_excluded" | "ambiguous_name" | "self_ordinary"
            ) {
                assert_eq!(
                    result
                        .pack
                        .open_loops
                        .iter()
                        .find(|item| item.memory.id == id(23))
                        .unwrap()
                        .direction,
                    None
                );
                assert!(assignment(&result, 23)
                    .cue_kinds
                    .contains(&CueKind::Trigger));
                let promise = result
                    .pack
                    .commitments
                    .iter()
                    .find(|item| item.memory.id == id(27))
                    .unwrap();
                assert_eq!(
                    promise.direction,
                    Some(ObligationDirection::OwedToCharacter)
                );
                assert!(assignment(&result, 27)
                    .cue_kinds
                    .contains(&CueKind::Trigger));
                assert!(admitted(&result, 20).is_none());
            }
            if matches!(
                case,
                "ambiguous_name" | "self_ordinary" | "future_due_party"
            ) {
                assert!(assignment(&result, 24)
                    .cue_kinds
                    .contains(&CueKind::Trigger));
            }
        }
        test_support::close_and_remove_root(memory, root).await;
    }

    #[tokio::test]
    async fn current_floor_then_score_and_no_spare_turn() {
        let mut drafts = vec![open(20, 2, 0.9)];
        drafts
            .extend((100..132).map(|n| belief(n, DerivedType::Claim, &[2], 1.0, "current belief")));
        let (memory, root) = fixture(drafts).await;
        let mut current = open(21, 2, 0.1);
        current.supersedes = vec![id(20)];
        write_belief(&memory, current).await;
        for cap in [2, 8] {
            for include_superseded in [false, true] {
                let mut query = request(vec![keyed(2)], None, cap);
                query.lifecycle_policy.include_superseded = include_superseded;
                let result = memory.retrieve(query).await.unwrap();
                record(
                    &format!("current_floor_cap_{cap}_superseded_{include_superseded}"),
                    &result,
                );
                assert!(expanded(&result).contains(&id(21)));
                assert_eq!(
                    expanded(&result).contains(&id(20)),
                    cap == 8 && include_superseded
                );
                if cap == 8 && include_superseded {
                    assert_eq!(
                        result
                            .pack
                            .open_loops
                            .iter()
                            .map(|item| item.memory.id)
                            .collect::<Vec<_>>(),
                        vec![id(20), id(21)]
                    );
                    assert!(scores(&result, 20).final_score > scores(&result, 21).final_score);
                }
                if !include_superseded {
                    assert!(admitted(&result, 20).is_none());
                }
            }
        }
        test_support::close_and_remove_root(memory, root).await;

        let mut drafts = vec![open(20, 2, 0.9), open(21, 2, 0.8)];
        drafts.extend((200..208).map(|n| belief(n, DerivedType::Claim, &[4], 1.0, "loud topic")));
        let (memory, root) = fixture(drafts).await;
        let result = memory
            .retrieve(request(vec![keyed(2)], Some("loud"), 4))
            .await
            .unwrap();
        record("trigger_takes_no_spare_turn", &result);
        assert_eq!(expanded(&result), vec![id(2), id(200), id(201), id(20)]);
        test_support::close_and_remove_root(memory, root).await;
    }
}

#[path = "support/mod.rs"]
pub mod test_support;

fn now() -> chrono::DateTime<chrono::Utc> {
    "2026-09-21T12:00:00.123Z".parse().unwrap()
}

fn obligation(n: u128, kind: DerivedType, text: &str) -> DerivedMemoryDraft {
    let mut draft = DerivedMemoryDraft::new(kind, text);
    draft.id = Some(id(n));
    draft.entity_ids = vec![id(1), id(2)];
    draft.given_by_application = true;
    draft
}

fn role(subject: u128, predicate: BeliefPredicate) -> BeliefAssertion {
    BeliefAssertion {
        subject: id(subject),
        predicate,
    }
}

fn plan(with_roles: bool) -> RememberWritePlan {
    let mut input = RememberInput::new("Obligations");
    for n in [1, 2] {
        let mut entity = EntityDraft::new();
        entity.id = Some(id(n));
        input = input.with_entity(entity);
    }
    let mut plan = input
        .with_derived_memory(obligation(
            10,
            DerivedType::Commitment,
            "I will bring Bob the book",
        ))
        .with_derived_memory(obligation(
            11,
            DerivedType::OpenLoop,
            "Bob said he would send the draft",
        ))
        .prepare_write_plan(&RememberPlanDefaults::fixed("obligations", now()));
    if with_roles {
        for candidate in &mut plan.candidates {
            if let MemoryCandidate::DerivedMemory(candidate) = candidate {
                let (actor, counterpart) = if candidate.draft.id == Some(id(10)) {
                    (1, 2)
                } else {
                    (2, 1)
                };
                candidate.draft.assertions = vec![
                    role(actor, BeliefPredicate::Actor),
                    role(counterpart, BeliefPredicate::Counterpart),
                ];
            }
        }
    }
    plan
}

fn query(self_present: bool, trace: bool) -> RetrievalContext {
    let mut scene = Scene::at(now().fixed_offset());
    scene.participants = vec![keyed(2)];
    if self_present {
        scene.participants.push(keyed(1));
    }
    RetrievalContext {
        include_trace: trace,
        ..RetrievalContext::default().with_scene(scene)
    }
}

fn selection(result: &RetrieveOutcome) -> Value {
    let pack = serde_json::to_value(&result.pack).unwrap();
    let ids = pack
        .as_object()
        .unwrap()
        .iter()
        .map(|(section, entries)| {
            let ids = entries
                .as_array()
                .unwrap()
                .iter()
                .map(|entry| entry.get("memory").unwrap_or(entry)["id"].clone())
                .collect::<Vec<_>>();
            (section.clone(), json!(ids))
        })
        .collect::<serde_json::Map<_, _>>();
    json!({
        "ids_in_section_order": ids,
        "assignments": result.trace.as_ref().unwrap().section_assignments,
        "admitted_by": result.memory_scenes.iter().map(|entry| json!({"memory": entry.memory, "roads": entry.admitted_by})).collect::<Vec<_>>(),
    })
}

#[tokio::test]
async fn direction_survives_reopen_with_the_same_pack_membership() {
    let mut without_roles: Vec<Value> = Vec::new();
    for with_roles in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let collection = test_support::unique_collection_name();
        let memory = test_support::try_setup_persistent_character_memory(
            collection.clone(),
            root.path(),
            id(1),
        )
        .await
        .unwrap();
        let plan = plan(with_roles);
        let written = memory
            .commit(plan.clone(), CommitOptions::default())
            .await
            .unwrap();
        let mut snapshots = Vec::new();
        for self_present in [false, true] {
            let result = memory.retrieve(query(self_present, true)).await.unwrap();
            assert_eq!(
                result
                    .pack
                    .commitments
                    .iter()
                    .map(|item| item.memory.id)
                    .collect::<Vec<_>>(),
                vec![id(10)]
            );
            assert_eq!(
                result
                    .pack
                    .open_loops
                    .iter()
                    .map(|item| item.memory.id)
                    .collect::<Vec<_>>(),
                vec![id(11)]
            );
            snapshots.push(json!({"self_present": self_present, "selection": selection(&result)}));
        }
        if with_roles {
            // Roles now add Trigger provenance and can raise graph proximity.
            for (after, before) in snapshots.iter().zip(&without_roles) {
                assert_eq!(
                    after["selection"]["ids_in_section_order"],
                    before["selection"]["ids_in_section_order"]
                );
            }
            println!(
                "AFTER_SELECTION={}",
                serde_json::to_string(&snapshots).unwrap()
            );
        } else {
            without_roles = snapshots;
        }
        let before = memory.retrieve(query(false, false)).await.unwrap();
        assert!(before.trace.is_none());
        assert_eq!(
            before.pack.commitments[0].direction,
            with_roles.then_some(ObligationDirection::OwedByCharacter)
        );
        assert_eq!(
            before.pack.open_loops[0].direction,
            with_roles.then_some(ObligationDirection::OwedToCharacter)
        );
        memory.close().await.unwrap();
        let memory =
            test_support::try_setup_persistent_character_memory(collection, root.path(), id(1))
                .await
                .unwrap();
        assert_eq!(
            memory.commit(plan, CommitOptions::default()).await.unwrap(),
            written
        );
        let after = memory.retrieve(query(false, false)).await.unwrap();
        assert_eq!(before.pack, after.pack);
        assert_eq!(
            serde_json::to_vec(&before.pack).unwrap(),
            serde_json::to_vec(&after.pack).unwrap()
        );
        test_support::close_and_remove_root(memory, root).await;
    }
    let error = serde_json::from_value::<BeliefAssertion>(
        json!({"subject": id(1), "predicate": "outside_vocabulary"}),
    )
    .unwrap_err();
    assert!(error.to_string().contains("unknown variant"));
}

#[tokio::test]
async fn direction_uses_roles_with_multiple_parties_and_preserves_assertion_order() {
    use BeliefPredicate::{Actor, Counterpart, KnownAs};
    let root = tempfile::tempdir().unwrap();
    let collection = test_support::unique_collection_name();
    let memory =
        test_support::try_setup_persistent_character_memory(collection.clone(), root.path(), id(1))
            .await
            .unwrap();
    let mut plan = plan(true);
    let mut alice = EntityDraft::new();
    alice.id = Some(id(3));
    alice.created_at = Some(now());
    alice.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
    plan.candidates
        .push(MemoryCandidate::Entity(EntityCandidate::new(
            alice,
            CandidateProvenance::caller("Alice"),
        )));
    let cases = [
        (
            12,
            vec![
                role(2, Counterpart),
                role(1, KnownAs { name: "Me".into() }),
                role(1, Actor),
                role(3, Counterpart),
                role(2, Counterpart),
            ],
            Some(ObligationDirection::OwedByCharacter),
        ),
        (13, vec![role(2, Actor), role(3, Counterpart)], None),
        (14, vec![], None),
        (
            15,
            vec![role(2, Actor), role(1, Actor), role(3, Counterpart)],
            Some(ObligationDirection::OwedByCharacter),
        ),
    ];
    for (n, assertions, _) in &cases {
        let mut draft = obligation(
            *n,
            DerivedType::Commitment,
            "A promise involving Bob and Alice",
        );
        draft.entity_ids.push(id(3));
        draft.assertions = assertions.clone();
        draft.created_at = Some(now());
        draft.updated_at = Some(now());
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
        plan.candidates
            .push(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
                draft,
                CandidateProvenance::caller("promise"),
            )));
    }
    let written = memory
        .commit(plan.clone(), CommitOptions::default())
        .await
        .unwrap();
    memory.close().await.unwrap();
    let memory =
        test_support::try_setup_persistent_character_memory(collection, root.path(), id(1))
            .await
            .unwrap();
    assert_eq!(
        memory.commit(plan, CommitOptions::default()).await.unwrap(),
        written
    );
    let result = memory.retrieve(query(false, false)).await.unwrap();
    assert!(result.trace.is_none());
    for (n, assertions, direction) in cases {
        let included = result
            .pack
            .commitments
            .iter()
            .find(|item| item.memory.id == id(n))
            .unwrap();
        assert_eq!(included.direction, direction, "obligation {n}");
        assert_eq!(included.memory.assertions, assertions);
    }
    test_support::close_and_remove_root(memory, root).await;
}

#[tokio::test]
async fn writes_and_corrections_reject_invalid_obligation_roles() {
    use BeliefPredicate::{Actor, Counterpart};
    let (memory, root) = test_support::try_setup_character_memory(id(1))
        .await
        .unwrap();
    memory
        .commit(plan(true), CommitOptions::default())
        .await
        .unwrap();
    let cases = [
        (
            DerivedType::Commitment,
            vec![role(3, Actor)],
            BeliefValidationError::AssertionSubjectNotInMemory { subject: id(3) },
        ),
        (
            DerivedType::OpenLoop,
            vec![role(3, Counterpart)],
            BeliefValidationError::AssertionSubjectNotInMemory { subject: id(3) },
        ),
        (
            DerivedType::Commitment,
            vec![role(1, Actor), role(1, Counterpart)],
            BeliefValidationError::ConflictingObligationRoles { subject: id(1) },
        ),
        (
            DerivedType::OpenLoop,
            vec![role(1, Counterpart), role(1, Actor)],
            BeliefValidationError::ConflictingObligationRoles { subject: id(1) },
        ),
        (
            DerivedType::Claim,
            vec![role(1, Actor)],
            BeliefValidationError::ObligationFieldOnOtherKind {
                derived_type: DerivedType::Claim,
            },
        ),
        (
            DerivedType::Claim,
            vec![role(1, Counterpart)],
            BeliefValidationError::ObligationFieldOnOtherKind {
                derived_type: DerivedType::Claim,
            },
        ),
    ];
    for (kind, assertions, expected) in cases {
        let mut draft = obligation(20, kind, "Invalid obligation");
        draft.assertions = assertions.clone();
        draft.created_at = Some(now());
        draft.updated_at = Some(now());
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
        let plan = RememberWritePlan::new().with_candidate(MemoryCandidate::DerivedMemory(
            DerivedMemoryCandidate::new(draft, CandidateProvenance::caller("invalid")),
        ));
        let has_error = |entries: &[CandidateValidation]| {
            entries.iter().any(|entry| {
                entry
                    .errors
                    .contains(&CandidateValidationIssue::InvalidBelief { reason: expected })
            })
        };
        assert!(has_error(&memory.validate_plan(&plan).await.unwrap()));
        assert!(
            matches!(memory.commit(plan, CommitOptions::default()).await,
            Err(CustomError::WritePlanValidationRejected { validations }) if has_error(&validations))
        );
        let mut replacement = ReplacementDerivedMemoryDraft::new(kind, "Invalid replacement");
        replacement.entity_ids = vec![id(1), id(2)];
        replacement.given_by_application = true;
        replacement.assertions = assertions;
        replacement
            .correction_origin_provenance
            .external_refs
            .push(ExternalSourceReference::source("application:correction"));
        assert_eq!(
            replacement.validate(),
            Err(LifecycleDtoValidationError::InvalidBelief(expected))
        );
        let mut correction = CorrectMemoryDraft::new(
            CorrectionTarget::derived_memory(id(10)),
            "correct obligation",
        );
        correction.correction_origin = replacement.correction_origin_provenance.clone();
        correction.replacement_derived_memories.push(replacement);
        assert!(matches!(memory.correct(correction).await,
            Err(CustomError::LifecycleDraftInvalid(LifecycleDtoValidationError::InvalidBelief(reason))) if reason == expected));
    }
    let result = memory.retrieve(query(false, false)).await.unwrap();
    assert_eq!(result.pack.commitments.len(), 1);
    assert_eq!(result.pack.commitments[0].memory.id, id(10));
    test_support::close_and_remove_root(memory, root).await;
}

fn graph_quads(root: &std::path::Path) -> std::collections::BTreeSet<String> {
    oxigraph::store::Store::open(root.join("graph"))
        .unwrap()
        .iter()
        .map(|quad| quad.unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn store_identity_is_literal_and_rejects_another_self_before_opening_vectors() {
    let root = tempfile::tempdir().unwrap();
    let collection = test_support::unique_collection_name();
    let memory =
        test_support::try_setup_persistent_character_memory(collection.clone(), root.path(), id(1))
            .await
            .unwrap();
    let result = memory.retrieve(query(true, false)).await.unwrap();
    assert_eq!(result.pack, ContinuityContextPack::empty());
    assert!(result
        .scene_references
        .iter()
        .all(|reference| reference.resolution == SceneReferenceResolution::Unknown));
    memory.close().await.unwrap();
    let before = graph_quads(root.path());
    assert_eq!(
        before,
        [format!(
            "<urn:cmem:store> <urn:cmem:vocab:characterIdentity> \"{}\" <urn:cmem:store>",
            id(1)
        )]
        .into_iter()
        .collect()
    );
    for injected in [false, true] {
        for service in [false, true] {
            let untouched = root.path().join(format!("untouched-{injected}-{service}"));
            let builder = test_support::persistent_settings(root.path())
                .set_override("vector_store_path", untouched.to_str().unwrap())
                .unwrap()
                .set_override("openai_api_key", "test-key")
                .unwrap()
                .set_override(
                    "vector_store_mode",
                    if service { "service" } else { "embedded" },
                )
                .unwrap()
                .set_override("qdrant_connection_string", "http://127.0.0.1:1")
                .unwrap();
            let settings = Settings::new(builder.build().unwrap()).unwrap();
            let opened = if injected {
                CharacterMemory::new_with_embedding_provider(
                    settings,
                    collection.clone(),
                    Box::new(test_support::deterministic_provider(1536)),
                    id(2),
                )
                .await
            } else {
                CharacterMemory::new(settings, collection.clone(), id(2)).await
            };
            let error = match opened {
                Err(error) => error,
                Ok(_) => panic!("another character opened the store"),
            };
            assert!(
                error.to_string().contains(&id(1).to_string())
                    && error.to_string().contains(&id(2).to_string())
            );
            assert!(
                matches!(error, CustomError::CharacterIdentityMismatch { stored, requested } if stored == id(1) && requested == id(2))
            );
            assert!(!untouched.exists());
            assert_eq!(graph_quads(root.path()), before);
        }
    }
    let memory =
        test_support::try_setup_persistent_character_memory(collection, root.path(), id(1))
            .await
            .unwrap();
    assert_eq!(
        memory.retrieve(query(false, false)).await.unwrap().pack,
        ContinuityContextPack::empty()
    );
    test_support::close_and_remove_root(memory, root).await;
}
