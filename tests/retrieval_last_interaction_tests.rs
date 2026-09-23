use std::collections::{BTreeMap, BTreeSet};

use character_memory::{
    BeliefAssertion, BeliefPredicate, CandidateProvenance, CharacterMemory, CommitOptions,
    DerivedMemoryDraft, DerivedType, EntityDraft, EpisodeCandidate, EpisodeDraft,
    ForgetMemoryDraft, LastInteraction, LifecycleTargetRef, MemoryCandidate, MemoryId,
    MemoryLinkDraft, ObjectType, ObservationCandidate, ObservationDraft, RelationType,
    RememberInput, RememberOptions, RememberPlanDefaults, RememberWritePlan, RetrievalContext,
    RetrieveOutcome, Scene, SceneParticipant, SceneReference, SceneReferenceResolution,
    DEFAULT_SCHEMA_VERSION,
};
use chrono::{DateTime, Duration, Utc};

#[path = "support/mod.rs"]
pub mod test_support;

fn id(n: u128) -> MemoryId {
    MemoryId::from_u128(n)
}
fn reference_time() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-09-21T12:00:00.750Z")
        .unwrap()
        .with_timezone(&Utc)
}
fn keyed(n: u128) -> SceneParticipant {
    SceneParticipant {
        key: Some(id(n)),
        ..Default::default()
    }
}
fn request(participants: Vec<SceneParticipant>) -> RetrievalContext {
    let mut scene = Scene::at((reference_time()).fixed_offset());
    scene.participants = participants;
    RetrievalContext::default().with_scene(scene)
}
async fn open() -> (CharacterMemory, tempfile::TempDir) {
    let root = tempfile::tempdir().unwrap();
    let memory = test_support::try_setup_persistent_character_memory(
        test_support::unique_collection_name(),
        root.path(),
        None,
    )
    .await
    .unwrap();
    (memory, root)
}
async fn commit(memory: &CharacterMemory, input: RememberInput) {
    let defaults = RememberPlanDefaults::fixed(&input.content, reference_time());
    memory
        .commit(
            input.prepare_write_plan(&defaults),
            CommitOptions::default(),
        )
        .await
        .unwrap();
}
async fn notion(memory: &CharacterMemory, n: u128, name: Option<&str>) {
    let mut entity = EntityDraft::new();
    entity.id = Some(id(n));
    let mut input = RememberInput::new(format!("notion {n}")).with_entity(entity);
    if let Some(name) = name {
        let mut belief = DerivedMemoryDraft::new(DerivedType::Claim, format!("Known as {name}"));
        belief.entity_ids.push(id(n));
        belief.given_by_application = true;
        belief.assertions.push(BeliefAssertion {
            subject: id(n),
            predicate: BeliefPredicate::KnownAs { name: name.into() },
        });
        input = input.with_derived_memory(belief);
    }
    commit(memory, input).await;
}
fn episode(n: u128, time: DateTime<Utc>) -> EpisodeDraft {
    let mut episode = EpisodeDraft::new(format!("Meeting {n}"));
    episode.id = Some(id(n));
    episode.scene = Some(Scene::at((time).fixed_offset()));
    episode.created_at = Some(reference_time() + (reference_time() - time));
    episode.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    episode
}
async fn experience(memory: &CharacterMemory, n: u128, time: DateTime<Utc>, caller: bool) {
    let mut episode = episode(n, time);
    episode
        .scene
        .as_mut()
        .unwrap()
        .participants
        .push(keyed(100));
    if caller {
        memory
            .commit(
                RememberWritePlan::new().with_candidate(MemoryCandidate::Episode(
                    EpisodeCandidate::new(episode, CandidateProvenance::caller("meeting")),
                )),
                CommitOptions::default(),
            )
            .await
            .unwrap();
    } else {
        memory
            .remember(
                RememberInput::new("We met").with_episode(episode),
                RememberOptions::default(),
            )
            .await
            .unwrap();
    }
}
fn fact(n: u128, time: DateTime<Utc>) -> LastInteraction {
    LastInteraction {
        episode_id: id(n),
        scene_time: time,
        seconds_since: (reference_time() - time).num_seconds(),
    }
}
fn occasions(result: &RetrieveOutcome) -> BTreeSet<MemoryId> {
    result
        .pack
        .relevant_episodes
        .iter()
        .map(|e| e.id)
        .chain(
            result
                .pack
                .salient_observations
                .iter()
                .map(|o| o.episode_id),
        )
        .collect()
}
fn last(result: &RetrieveOutcome) -> Option<&LastInteraction> {
    result.scene_references[0].last_interactions[&id(100)].as_ref()
}

#[tokio::test]
async fn identity_reports_last_interaction_without_trace_or_pack_room() {
    for caller in [false, true] {
        let (memory, root) = open().await;
        notion(&memory, 100, Some("Mira")).await;
        notion(&memory, 200, None).await;
        for days in [365, 30, 1] {
            experience(
                &memory,
                (10000 + days) as u128,
                reference_time() - Duration::days(days),
                caller,
            )
            .await;
        }
        let expected = fact(10001, reference_time() - Duration::days(1));
        for participant in [
            keyed(100),
            SceneParticipant {
                name: Some("Mira".into()),
                ..Default::default()
            },
        ] {
            let result = memory.retrieve(request(vec![participant])).await.unwrap();
            assert!(result.trace.is_none());
            assert_eq!(last(&result), Some(&expected));
            assert!(occasions(&result).contains(&id(10001)));
        }
        experience(&memory, 9999, reference_time() + Duration::days(1), caller).await;
        let result = memory.retrieve(request(vec![keyed(100)])).await.unwrap();
        assert_eq!(last(&result), Some(&expected));
        assert!(occasions(&result).contains(&id(10001)));
        assert!(!occasions(&result).contains(&id(9999)));

        let mut no_room = request(vec![keyed(100), keyed(200)]);
        no_room.candidate_limits.max_graph_roots = 0;
        no_room.graph_limits.max_depth = 0;
        let result = memory.retrieve(no_room).await.unwrap();
        assert!(occasions(&result).is_empty());
        assert_eq!(last(&result), Some(&expected));
        assert_eq!(
            result.scene_references[1].last_interactions,
            BTreeMap::from([(id(200), None)])
        );
        let reference = serde_json::to_value(&result.scene_references[1]).unwrap();
        assert!(reference["last_interactions"][id(200).to_string()].is_null());

        // An ambiguous known name reports each possible identity, without guessing.
        let mut other_name = DerivedMemoryDraft::new(DerivedType::Claim, "Also known as Mira");
        other_name.entity_ids.push(id(200));
        other_name.given_by_application = true;
        other_name.assertions.push(BeliefAssertion {
            subject: id(200),
            predicate: BeliefPredicate::KnownAs {
                name: "Mira".into(),
            },
        });
        commit(
            &memory,
            RememberInput::new("shared name").with_derived_memory(other_name),
        )
        .await;
        let result = memory
            .retrieve(request(vec![
                SceneParticipant {
                    name: Some("Mira".into()),
                    ..Default::default()
                },
                keyed(999),
                SceneParticipant {
                    name: Some("Unknown".into()),
                    ..Default::default()
                },
                SceneParticipant {
                    description: Some("Mira".into()),
                    ..Default::default()
                },
            ]))
            .await
            .unwrap();
        assert_eq!(
            result.scene_references[0].resolution,
            SceneReferenceResolution::Ambiguous {
                notion_ids: vec![id(100), id(200)]
            }
        );
        assert_eq!(
            result.scene_references[0].last_interactions,
            BTreeMap::from([(id(100), Some(expected)), (id(200), None)])
        );
        assert!(result.scene_references[1..]
            .iter()
            .all(|reference| reference.last_interactions.is_empty()));
        assert!(matches!(
            result.scene_references.last().unwrap().reference,
            SceneReference::ParticipantDescription { index: 3 }
        ));
        memory.close().await.unwrap();
        root.close().unwrap();
    }
}

#[tokio::test]
async fn scene_boundary_and_whole_seconds_distinguish_zero_from_never_met() {
    let (memory, root) = open().await;
    notion(&memory, 100, None).await;
    let earlier = reference_time() - Duration::milliseconds(1500);
    experience(&memory, 400, earlier, true).await;
    experience(
        &memory,
        401,
        reference_time() + Duration::milliseconds(1),
        true,
    )
    .await;
    let result = memory.retrieve(request(vec![keyed(100)])).await.unwrap();
    assert_eq!(last(&result), Some(&fact(400, earlier)));
    assert_eq!(last(&result).unwrap().seconds_since, 1);
    let notion_episode =
        RememberPlanDefaults::fixed("notion 100", reference_time()).stable_id("episode:0");
    assert_eq!(
        occasions(&result),
        BTreeSet::from([id(400), notion_episode])
    );
    for n in [403, 402] {
        experience(&memory, n, reference_time(), true).await;
    }
    let result = memory.retrieve(request(vec![keyed(100)])).await.unwrap();
    assert_eq!(last(&result), Some(&fact(402, reference_time())));
    assert_eq!(last(&result).unwrap().seconds_since, 0);
    assert_eq!(
        occasions(&result),
        BTreeSet::from([id(400), id(402), id(403), notion_episode])
    );
    let mut before = request(vec![keyed(100)]);
    before.scene.time = (earlier - Duration::milliseconds(1)).fixed_offset();
    let result = memory.retrieve(before).await.unwrap();
    assert_eq!(last(&result), None);
    assert!(occasions(&result).is_empty());
    memory.close().await.unwrap();
    root.close().unwrap();
}

async fn linked_experience(
    memory: &CharacterMemory,
    n: u128,
    time: DateTime<Utc>,
    mentions: bool,
    reverse: bool,
) {
    let mut plan =
        RememberWritePlan::new().with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
            episode(n, time),
            CandidateProvenance::caller("unkeyed experience"),
        )));
    let (kind, neighbor, relation) = if mentions {
        let mut observation = ObservationDraft::new(id(n), "A remark");
        observation.id = Some(id(n + 1000));
        observation.created_at = Some(reference_time());
        observation.schema_version = Some(DEFAULT_SCHEMA_VERSION.into());
        plan = plan.with_candidate(MemoryCandidate::Observation(ObservationCandidate::new(
            observation,
            CandidateProvenance::caller("remark"),
        )));
        (
            ObjectType::Observation,
            id(n + 1000),
            RelationType::Mentions,
        )
    } else {
        (ObjectType::Episode, id(n), RelationType::Involves)
    };
    memory.commit(plan, CommitOptions::default()).await.unwrap();
    let link = if reverse {
        MemoryLinkDraft::new(kind, neighbor, relation, ObjectType::Entity, id(100))
    } else {
        MemoryLinkDraft::new(ObjectType::Entity, id(100), relation, kind, neighbor)
    };
    memory.link(link).await.unwrap();
}

#[tokio::test]
async fn links_and_suppression_determine_last_interaction_in_both_orientations() {
    let mut tight_occasions = Vec::new();
    for (mentions, reverse) in [(false, false), (false, true), (true, false), (true, true)] {
        let (memory, root) = open().await;
        notion(&memory, 100, None).await;
        let old = reference_time() - Duration::days(1);
        let recent = reference_time() - Duration::hours(1);
        linked_experience(&memory, 400, old, mentions, reverse).await;
        linked_experience(&memory, 401, recent, mentions, reverse).await;
        linked_experience(
            &memory,
            499,
            reference_time() + Duration::days(1),
            mentions,
            reverse,
        )
        .await;
        let result = memory.retrieve(request(vec![keyed(100)])).await.unwrap();
        // Talking about someone does not establish an interaction with them.
        assert_eq!(last(&result), (!mentions).then_some(&fact(401, recent)));
        assert!(occasions(&result).contains(&id(401)));
        assert!(!occasions(&result).contains(&id(499)));
        let mut ranged = request(vec![keyed(100)]);
        ranged.time_range = Some(character_memory::api::types::TimeRange {
            start: old,
            end: recent,
        });
        let ranged = memory.retrieve(ranged).await.unwrap();
        assert!(
            !occasions(&ranged).contains(&id(499)),
            "a range does not exempt ordinary expansion"
        );
        let mut tight = request(vec![keyed(100)]);
        tight.section_limits.relevant_episodes = 1;
        tight.section_limits.salient_observations = 1;
        let tight_result = memory.retrieve(tight).await.unwrap();
        assert_eq!(
            last(&tight_result),
            (!mentions).then_some(&fact(401, recent))
        );
        tight_occasions.push(occasions(&tight_result));
        let target = if mentions {
            LifecycleTargetRef::observation(id(1401))
        } else {
            LifecycleTargetRef::episode(id(401))
        };
        memory
            .forget(ForgetMemoryDraft::suppress(target, "forgotten interaction"))
            .await
            .unwrap();
        for (suppressed, superseded) in [(false, false), (false, true), (true, false)] {
            let mut query = request(vec![keyed(100)]);
            query.lifecycle_policy.include_suppressed = suppressed;
            query.lifecycle_policy.include_superseded = superseded;
            let result = memory.retrieve(query).await.unwrap();
            let expected = if suppressed {
                fact(401, recent)
            } else {
                fact(400, old)
            };
            assert_eq!(last(&result), (!mentions).then_some(&expected));
            assert!(occasions(&result).contains(&expected.episode_id));
        }
        if mentions {
            let latest = reference_time() - Duration::minutes(30);
            linked_experience(&memory, 402, latest, true, reverse).await;
            memory
                .forget(ForgetMemoryDraft::suppress(
                    LifecycleTargetRef::episode(id(402)),
                    "forgotten parent",
                ))
                .await
                .unwrap();
            for suppressed in [false, true] {
                let mut query = request(vec![keyed(100)]);
                query.lifecycle_policy.include_suppressed = suppressed;
                let result = memory.retrieve(query).await.unwrap();
                let expected = if suppressed {
                    fact(402, latest)
                } else {
                    fact(400, old)
                };
                assert_eq!(last(&result), (!mentions).then_some(&expected));
                assert!(occasions(&result).contains(&expected.episode_id));
            }
        }
        memory.close().await.unwrap();
        root.close().unwrap();
    }
    let notion_episode =
        RememberPlanDefaults::fixed("notion 100", reference_time()).stable_id("episode:0");
    assert_eq!(
        tight_occasions,
        vec![
            // Rulings 46 and 69: recency brings the notion-creation observation.
            BTreeSet::from([id(401), notion_episode]),
            BTreeSet::from([id(401), notion_episode]),
            // Bounded aboutness now brings the remark and its occasion before recency.
            BTreeSet::from([id(401)]),
            BTreeSet::from([id(401)]),
        ]
    );
}
