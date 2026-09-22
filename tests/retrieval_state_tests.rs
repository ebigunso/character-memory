use character_memory::{
    CharacterMemory, CommitOptions, DerivedMemoryDraft, DerivedType, EntityDraft, EpisodeDraft,
    ForgetMemoryDraft, LifecycleFilterReason, LifecycleTargetRef, MemoryId, ObjectType,
    RelationType, RememberInput, RememberPlanDefaults, RetrievalContext, Scene, SceneParticipant,
    DEFAULT_SCHEMA_VERSION,
};
use chrono::{DateTime, Utc};

#[path = "support/mod.rs"]
pub mod test_support;

async fn commit_input(memory: &CharacterMemory, input: RememberInput) {
    let defaults = RememberPlanDefaults::fixed(&input.content, time(0));
    memory
        .commit(
            input.prepare_write_plan(&defaults),
            CommitOptions::default(),
        )
        .await
        .unwrap();
}

fn time(offset: i64) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-09-21T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
        + chrono::Duration::minutes(offset)
}

fn entity(id: u128) -> EntityDraft {
    let mut draft = EntityDraft::new();
    draft.id = Some(MemoryId::from_u128(id));
    draft.created_at = Some(time(0));
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    draft
}

fn scene(ids: &[u128], at: i64) -> Scene {
    let mut scene = Scene::at((time(at)).fixed_offset());
    scene.participants = ids
        .iter()
        .copied()
        .map(|id| SceneParticipant {
            key: Some(MemoryId::from_u128(id)),
            ..Default::default()
        })
        .collect();
    scene
}

fn episode(id: u128, participants: &[u128], at: i64) -> EpisodeDraft {
    let mut draft = EpisodeDraft::new(format!("Episode {id}"));
    draft.id = Some(MemoryId::from_u128(id));
    draft.scene = Some(scene(participants, at));
    draft.created_at = Some(time(at));
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    draft
}

fn belief(
    id: u128,
    episode_id: u128,
    subject: Option<u128>,
    text: impl Into<String>,
    at: i64,
) -> DerivedMemoryDraft {
    let mut draft = DerivedMemoryDraft::new(DerivedType::Claim, text)
        .with_source_episode(MemoryId::from_u128(episode_id));
    draft.id = Some(MemoryId::from_u128(id));
    draft.entity_ids = subject.into_iter().map(MemoryId::from_u128).collect();
    draft.created_at = Some(time(at));
    draft.updated_at = Some(time(at));
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    draft
}

async fn write(
    memory: &CharacterMemory,
    episode_id: u128,
    participants: &[u128],
    derived: Option<DerivedMemoryDraft>,
    at: i64,
) {
    let mut input = RememberInput::new(format!("record {episode_id}")).with_episode(episode(
        episode_id,
        participants,
        at,
    ));
    if let Some(draft) = derived {
        input = input.with_derived_memory(draft);
    }
    commit_input(memory, input).await;
}

async fn write_many(
    memory: &CharacterMemory,
    episode_id: u128,
    participants: &[u128],
    derived: Vec<DerivedMemoryDraft>,
    at: i64,
) {
    let mut input = RememberInput::new(format!("record {episode_id}")).with_episode(episode(
        episode_id,
        participants,
        at,
    ));
    for draft in derived {
        input = input.with_derived_memory(draft);
    }
    commit_input(memory, input).await;
}

fn ids(result: &character_memory::RetrieveOutcome) -> Vec<MemoryId> {
    result
        .pack
        .derived_memories
        .iter()
        .map(|x| x.memory.id)
        .collect()
}

#[tokio::test]
async fn named_people_share_section_room_in_scope_rounds() {
    let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
    let mut seed = RememberInput::new("six notions");
    for id in 50..=55 {
        seed = seed.with_entity(entity(id));
    }
    commit_input(&memory, seed).await;
    let hub = (0..200u128)
        .map(|offset| {
            belief(
                1000 + offset,
                10000,
                Some(50),
                format!("hub belief {offset}"),
                offset as i64,
            )
        })
        .collect();
    write_many(&memory, 10000, &[50], hub, 0).await;
    for (index, person) in (51..=55).enumerate() {
        let mut beliefs = vec![belief(
            9000 + index as u128,
            11000 + index as u128,
            Some(person),
            format!("other person {person} state"),
            10 + index as i64,
        )];
        if person == 51 {
            beliefs.push(belief(9100, 11000, Some(person), "second other state", 20));
        }
        write_many(
            &memory,
            11000 + index as u128,
            &[person],
            beliefs,
            10 + index as i64,
        )
        .await;
    }
    write(
        &memory,
        12000,
        &[],
        Some(belief(9500, 12000, None, "loud topic anchor", 50)),
        50,
    )
    .await;
    for topic in [None, Some("loud topic")] {
        let mut context = RetrievalContext::default().with_trace();
        context.scene = scene(&[50, 51, 52, 53, 54, 55], 1000);
        context.topic = topic.map(str::to_owned);
        let result = memory.retrieve(context).await.unwrap();
        let states = ids(&result)
            .into_iter()
            .filter(|id| id.as_u128() != 9500)
            .map(|id| id.as_u128())
            .collect::<Vec<_>>();
        assert_eq!(&states[..6], &[1184, 9000, 9001, 9002, 9003, 9004]);
        if topic.is_some() {
            assert_eq!(
                ids(&result)
                    .iter()
                    .map(|id| id.as_u128())
                    .collect::<Vec<_>>(),
                [9500, 1184, 9000, 9001, 9002, 9003, 9004, 1185, 9100, 1186, 1187, 1188]
            );
        } else {
            assert_eq!(&states[6..8], &[1185, 9100]);
        }
        assert_eq!(result.pack.derived_memories.len(), 12);
        let telemetry = &result.rationale.telemetry.selectivity;
        assert_eq!(
            telemetry.decision_count,
            telemetry.high_selectivity_count
                + telemetry.low_selectivity_supported_count
                + telemetry.low_selectivity_rejected_count
                + telemetry.fallback_count
        );
        let trace = result.trace.unwrap();
        let scope_states = if topic.is_some() {
            &states[..6]
        } else {
            &states[..]
        };
        assert!(trace
            .section_assignments
            .iter()
            .filter(|row| row.rank.is_some() && scope_states.contains(&row.object.id.as_u128()))
            .all(|row| row
                .cue_kinds
                .contains(&character_memory::CueKind::Participant)));
    }
    // One slot credits both subjects; scene order still governs a tight section.
    let mut shared = belief(9700, 15000, Some(50), "shared state", 100);
    shared.entity_ids.push(MemoryId::from_u128(51));
    shared.salience_score = 1.0;
    write(&memory, 15000, &[], Some(shared), 100).await;
    let mut context =
        RetrievalContext::default().with_scene(scene(&[55, 54, 53, 52, 51, 50], 1000));
    context.section_limits.derived_memories = 5;
    let result = memory.retrieve(context).await.unwrap();
    assert_eq!(
        ids(&result)
            .iter()
            .map(|id| id.as_u128())
            .collect::<Vec<_>>(),
        vec![9004, 9003, 9002, 9001, 9700]
    );
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn named_subject_fanout_selects_current_salient_then_recent_state() {
    let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
    commit_input(
        &memory,
        RememberInput::new("one notion").with_entity(entity(60)),
    )
    .await;
    // Stale high-salience neighbours cannot use the fanout before current state is chosen.
    let stale = (1980..1996)
        .map(|id| {
            let mut old = belief(id, 12999, Some(60), "superseded state", 100);
            old.salience_score = 1.0;
            old
        })
        .collect();
    write_many(&memory, 12999, &[60], stale, 100).await;
    let beliefs = (0..25u128)
        .map(|offset| {
            let mut draft = belief(
                2000 + offset,
                13000,
                Some(60),
                format!("scoped belief {offset}"),
                200 + offset.min(23) as i64,
            );
            draft.salience_score = match offset {
                3 => 0.99,
                17 => 0.98,
                22 => 0.97,
                _ => 0.1,
            };
            if offset == 3 {
                draft.supersedes = (1980..1996).map(MemoryId::from_u128).collect();
            }
            draft
        })
        .collect::<Vec<_>>();
    write_many(&memory, 13000, &[60], beliefs, 200).await;
    write(
        &memory,
        12000,
        &[],
        Some(belief(9500, 12000, None, "loud topic anchor", 50)),
        50,
    )
    .await;
    for topic in [None, Some("loud topic")] {
        let mut context = RetrievalContext::default().with_scene(scene(&[60], 1000));
        context.topic = topic.map(str::to_owned);
        context.section_limits.derived_memories = 30;
        let result = memory.retrieve(context).await.unwrap();
        let states = ids(&result)
            .into_iter()
            .filter(|id| id.as_u128() != 9500)
            .map(|id| id.as_u128())
            .collect::<Vec<_>>();
        assert_eq!(
            &states[..16],
            &[
                2003, 2017, 2022, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2018, 2019, 2020, 2021,
                2023, 2024
            ]
        );
        assert!(states.iter().all(|id| !(1980..1996).contains(id)));
    }
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn named_person_topic_match_keeps_first_place_in_their_scope() {
    let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
    let mut seed = RememberInput::new("six topic-test notions");
    for id in 70..76 {
        seed = seed.with_entity(entity(id));
    }
    commit_input(&memory, seed).await;
    let mut salient = belief(8000, 16000, Some(70), "unrelated habitual preference", 100);
    salient.salience_score = 1.0;
    let topic = "telescope optics astronomy observatory";
    let mut on_topic = belief(8001, 16000, Some(70), topic, 10);
    on_topic.salience_score = 0.5;
    let mut beliefs = vec![salient, on_topic];
    for person in 71..76 {
        beliefs.push(belief(
            8000 + person,
            16000,
            Some(person),
            "other person state",
            0,
        ));
    }
    write_many(&memory, 16000, &[], beliefs, 0).await;
    let mut context =
        RetrievalContext::new(topic).with_scene(scene(&[70, 71, 72, 73, 74, 75], 1000));
    context.section_limits.derived_memories = 6;
    context.include_trace = true;
    let result = memory.retrieve(context).await.unwrap();
    assert_eq!(
        ids(&result)
            .iter()
            .map(|id| id.as_u128())
            .collect::<Vec<_>>(),
        vec![8001, 8071, 8072, 8073, 8074, 8075]
    );
    assert!(!result.trace.unwrap().floor_admissions.iter().any(|row| {
        row.object.id == MemoryId::from_u128(8000)
            && row.cue_kind == character_memory::CueKind::Topic
            && row.stage
                == character_memory::api::types::CueFloorStage::Section {
                    section: character_memory::ContextPackSection::DerivedMemories,
                }
    }));
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn named_subject_preserves_lifecycle_evidence_and_independent_opt_ins() {
    let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
    commit_input(
        &memory,
        RememberInput::new("lifecycle notion").with_entity(entity(70)),
    )
    .await;
    let mut old = belief(8000, 16000, Some(70), "old state", 0);
    old.salience_score = 1.0;
    let mut suppressed = belief(8001, 16000, Some(70), "suppressed state", 0);
    suppressed.salience_score = 0.9;
    write_many(&memory, 16000, &[], vec![old, suppressed], 0).await;
    let mut current = belief(8002, 16001, Some(70), "current state", 10);
    current.salience_score = 0.1;
    current.supersedes.push(MemoryId::from_u128(8000));
    // A second successor is outside the named subject's graph frontier.
    let mut outside = belief(8003, 16001, None, "another correction", 10);
    outside.supersedes.push(MemoryId::from_u128(8000));
    write_many(&memory, 16001, &[], vec![current, outside], 10).await;
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::derived_memory(MemoryId::from_u128(8001)),
            "test suppression",
        ))
        .await
        .unwrap();
    for (include_superseded, include_suppressed) in
        [(false, false), (true, false), (false, true), (true, true)]
    {
        let mut context = RetrievalContext::default()
            .with_scene(scene(&[70], 1000))
            .with_trace();
        context.graph_limits.max_depth = 1;
        context.graph_limits.allowed_relation_types = vec![RelationType::About];
        context.graph_limits.allowed_object_types = vec![ObjectType::DerivedMemory];
        context.lifecycle_policy.include_superseded = include_superseded;
        context.lifecycle_policy.include_suppressed = include_suppressed;
        let result = memory.retrieve(context.clone()).await.unwrap();
        let returned = ids(&result);
        assert!(returned.contains(&MemoryId::from_u128(8002)));
        assert_eq!(
            returned.contains(&MemoryId::from_u128(8000)),
            include_superseded
        );
        assert_eq!(
            returned.contains(&MemoryId::from_u128(8001)),
            include_suppressed
        );
        let trace = result.trace.unwrap();
        if !include_superseded {
            let decision = trace
                .lifecycle_filter_decisions
                .iter()
                .find(|row| row.object.id == MemoryId::from_u128(8000))
                .unwrap();
            assert_eq!(decision.reason, LifecycleFilterReason::SupersededOmitted);
            assert_eq!(
                decision.superseded_by,
                vec![MemoryId::from_u128(8002), MemoryId::from_u128(8003)]
            );
        }
        if !include_suppressed {
            assert!(trace
                .lifecycle_filter_decisions
                .iter()
                .any(|row| row.object.id == MemoryId::from_u128(8001)
                    && row.reason == LifecycleFilterReason::SuppressedOmitted));
            context.graph_limits.max_fanout_per_node = 1;
            assert_eq!(ids(&memory.retrieve(context).await.unwrap()), vec![MemoryId::from_u128(8002)],
                "superseded high-salience state must not displace current state at the neighbour cap");
        }
    }
    memory.close().await.unwrap();
    root.close().unwrap();
}
