use crate::api::types::*;
use crate::domain::*;
use crate::ports::graph_authority::{GraphExpansionQuery, GraphObjectQuery, TraceMode};
use crate::ports::retrieval_stats::RetrievalStatsCounterKey;
use crate::test_support::{
    in_memory_graph_store, DeterministicMemoryEmbedder, TemporaryVectorCandidateStore,
};
use crate::{CharacterMemory, CustomError};

fn assertion(subject: MemoryId, name: &str) -> BeliefAssertion {
    BeliefAssertion {
        subject,
        predicate: BeliefPredicate::KnownAs {
            name: name.to_owned(),
        },
    }
}

fn given_belief(id: MemoryId, subject: MemoryId, name: &str) -> DerivedMemoryDraft {
    let mut draft =
        DerivedMemoryDraft::new(DerivedType::Claim, format!("I know this notion as {name}."));
    draft.id = Some(id);
    draft.entity_ids = vec![subject];
    draft.assertions = vec![assertion(subject, name)];
    draft.given_by_application = true;
    draft.created_at = Some(chrono::Utc::now());
    draft.updated_at = draft.created_at;
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    draft
}

fn plan_with_belief(draft: DerivedMemoryDraft, create_notion: bool) -> RememberWritePlan {
    let mut plan = RememberWritePlan::new();
    if create_notion {
        let mut entity = EntityDraft::new();
        entity.id = draft.entity_ids.first().copied();
        entity.created_at = draft.created_at;
        entity.schema_version = draft.schema_version.clone();
        plan = plan.with_candidate(MemoryCandidate::Entity(EntityCandidate::new(
            entity,
            CandidateProvenance::caller("application notion"),
        )));
    }
    plan.with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
        MemoryObjectRef::new(ObjectType::DerivedMemory, draft.id.unwrap()),
        CandidateProvenance::caller("belief content"),
    )))
    .with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
        draft,
        CandidateProvenance::caller("application belief"),
    )))
}

async fn memory() -> CharacterMemory {
    CharacterMemory::from_parts(
        Box::new(in_memory_graph_store()),
        Box::new(TemporaryVectorCandidateStore::open(8).await),
        Box::new(DeterministicMemoryEmbedder::new(8)),
    )
}

#[tokio::test]
async fn notion_names_are_current_beliefs_with_history_and_lossless_replay() {
    let memory = memory().await;
    let notion = MemoryId::from_u128(6001);
    let old_id = MemoryId::from_u128(6002);
    let second_notion = MemoryId::from_u128(6003);
    let other_belief = MemoryId::from_u128(6004);
    let new_id = MemoryId::from_u128(6005);
    let mut belief = given_belief(old_id, notion, "  Ａｌｉｃｅ\u{a0}  Smith  ");
    // Preserve order and duplicates, not just a set of assertion values.
    belief
        .assertions
        .extend([assertion(notion, "Al"), assertion(notion, "Al")]);
    let original = belief.clone().into_domain().unwrap();
    let plan = plan_with_belief(belief, true);
    let first = memory
        .commit(plan.clone(), CommitOptions::default())
        .await
        .unwrap();
    assert_eq!(first.persisted_link_ids.len(), 1);
    assert_eq!(first.vector_indexed_object_ids, vec![old_id]);
    memory
        .commit(plan.clone(), CommitOptions::default())
        .await
        .unwrap();
    let graph = &memory.memory_composition.graph_store;
    assert!(graph
        .query_objects(&GraphObjectQuery::by_types(
            vec![ObjectType::Episode, ObjectType::Observation],
            None
        ))
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        graph.query_notions_known_as("alice smith").await.unwrap(),
        vec![notion]
    );
    assert_eq!(
        graph.query_notions_known_as("AL").await.unwrap(),
        vec![notion]
    );
    assert_eq!(
        graph
            .query_objects(&GraphObjectQuery::by_ids(vec![old_id]))
            .await
            .unwrap(),
        vec![MemoryObject::DerivedMemory(original.clone())]
    );
    memory
        .commit(
            plan_with_belief(
                given_belief(other_belief, second_notion, "Alice Smith"),
                true,
            ),
            CommitOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(
        graph.query_notions_known_as("alice smith").await.unwrap(),
        vec![notion, second_notion]
    );

    let mut renamed = given_belief(new_id, notion, "Bob");
    renamed.supersedes = vec![old_id];
    let prepared = memory
        .prepare(
            RememberInput::new("The application changes the name.").with_derived_memory(renamed),
            PrepareOptions::default(),
        )
        .await
        .unwrap();
    assert!(memory
        .validate_plan(&prepared)
        .await
        .unwrap()
        .iter()
        .all(|item| item.status == CandidateValidationStatus::Valid));
    memory
        .commit(prepared.clone(), CommitOptions::default())
        .await
        .unwrap();
    memory
        .commit(prepared, CommitOptions::default())
        .await
        .unwrap();
    memory.commit(plan, CommitOptions::default()).await.unwrap();
    assert_eq!(
        graph.query_notions_known_as("bob").await.unwrap(),
        vec![notion]
    );
    assert_eq!(
        graph.query_notions_known_as("alice smith").await.unwrap(),
        vec![second_notion]
    );
    assert!(graph.query_notions_known_as("Al").await.unwrap().is_empty());
    assert_eq!(
        graph
            .query_objects(&GraphObjectQuery::by_ids(vec![old_id]))
            .await
            .unwrap(),
        vec![MemoryObject::DerivedMemory(original)]
    );
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::derived_memory(new_id),
            "Suppress the given name.",
        ))
        .await
        .unwrap();
    assert!(graph
        .query_notions_known_as("bob")
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        graph.query_notions_known_as("alice smith").await.unwrap(),
        vec![second_notion]
    );
    memory.close().await.unwrap();
}

#[tokio::test]
async fn notion_name_normalization_preserves_scripts_punctuation_and_lowercase_ceiling() {
    let memory = memory().await;
    let notion = MemoryId::from_u128(6101);
    let mut belief = given_belief(MemoryId::from_u128(6102), notion, "Straße");
    belief.assertions.extend([
        assertion(notion, "Éva"),
        assertion(notion, "アリス"),
        assertion(notion, "A-B"),
        assertion(notion, "A\"B\\C"),
    ]);
    memory
        .commit(plan_with_belief(belief, true), CommitOptions::default())
        .await
        .unwrap();
    let graph = &memory.memory_composition.graph_store;
    for name in ["STRAẞE", "E\u{301}VA", "ｱﾘｽ", "a-b", "a\"b\\c"] {
        assert_eq!(
            graph.query_notions_known_as(name).await.unwrap(),
            vec![notion],
            "{name}"
        );
    }
    for name in ["STRASSE", "Eva", "ありす", "AB", " ", "unseen"] {
        assert!(
            graph.query_notions_known_as(name).await.unwrap().is_empty(),
            "{name}"
        );
    }
    memory.close().await.unwrap();
}

#[tokio::test]
async fn notion_belief_admission_is_enforced_at_validate_and_commit() {
    let memory = memory().await;
    let subject = MemoryId::from_u128(6201);
    let base = given_belief(MemoryId::from_u128(6202), subject, "Alice");
    let invalid = [
        (
            base.clone(),
            CandidateValidationIssue::UnknownObjectRef {
                role: CandidateReferenceRole::BeliefSubject,
                referenced: MemoryObjectRef::new(ObjectType::Entity, subject),
            },
        ),
        (
            DerivedMemoryDraft {
                given_by_application: false,
                ..base.clone()
            },
            CandidateValidationIssue::MissingDerivedSource,
        ),
        (
            DerivedMemoryDraft {
                derived_from_episode_ids: vec![MemoryId::from_u128(6203)],
                ..base.clone()
            },
            CandidateValidationIssue::InvalidBelief {
                reason: BeliefValidationError::GivenWithSources,
            },
        ),
        (
            DerivedMemoryDraft {
                entity_ids: vec![],
                ..base.clone()
            },
            CandidateValidationIssue::InvalidBelief {
                reason: BeliefValidationError::GivenWithoutSubject,
            },
        ),
        (
            DerivedMemoryDraft {
                assertions: vec![assertion(MemoryId::from_u128(6204), "Alice")],
                ..base.clone()
            },
            CandidateValidationIssue::InvalidBelief {
                reason: BeliefValidationError::AssertionSubjectNotInMemory {
                    subject: MemoryId::from_u128(6204),
                },
            },
        ),
        (
            DerivedMemoryDraft {
                assertions: vec![assertion(subject, "\u{a0}\t\n")],
                ..base.clone()
            },
            CandidateValidationIssue::InvalidBelief {
                reason: BeliefValidationError::EmptyAssertionName { subject },
            },
        ),
    ];
    for (draft, expected) in invalid {
        let plan = plan_with_belief(draft, false);
        assert!(
            memory
                .validate_plan(&plan)
                .await
                .unwrap()
                .iter()
                .any(|item| item.errors.contains(&expected)),
            "{expected:?}"
        );
        assert!(matches!(
            memory.commit(plan, CommitOptions::default()).await,
            Err(CustomError::WritePlanValidationRejected { .. })
        ));
    }
    let mut plain = DerivedMemoryDraft::new(DerivedType::Claim, "Someone calls this notion Alice.");
    plain.entity_ids = vec![subject];
    let plain_plan = memory
        .prepare(
            RememberInput::new("A reported name.").with_derived_memory(plain),
            PrepareOptions::default(),
        )
        .await
        .unwrap();
    let missing = CandidateValidationIssue::UnknownObjectRef {
        role: CandidateReferenceRole::BeliefSubject,
        referenced: MemoryObjectRef::new(ObjectType::Entity, subject),
    };
    assert!(memory
        .validate_plan(&plain_plan)
        .await
        .unwrap()
        .iter()
        .any(|item| item.errors.contains(&missing)));
    assert!(
        matches!(memory.commit(plain_plan, CommitOptions::default()).await,
        Err(CustomError::WritePlanValidationRejected { validations }) if validations.iter().any(|item| item.errors.contains(&missing)))
    );
    assert!(serde_json::from_value::<BeliefAssertion>(
        serde_json::json!({"subject": subject, "predicate": "same_as", "name": "Alice"})
    )
    .is_err());
    assert!(memory
        .memory_composition
        .graph_store
        .query_objects(&GraphObjectQuery::by_ids(vec![base.id.unwrap()]))
        .await
        .unwrap()
        .is_empty());
    memory.close().await.unwrap();
}

#[tokio::test]
async fn every_belief_subject_is_linked_and_counted_once() {
    for sqlite in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut memory = memory().await;
        if sqlite {
            memory.memory_composition.stats_store = Box::new(
                crate::adapters::stats::SqliteRetrievalStatsStore::open(
                    root.path().join("stats.sqlite"),
                )
                .unwrap(),
            );
        }
        let subject = MemoryId::from_u128(6301);
        let belief = given_belief(MemoryId::from_u128(6302), subject, "Alice");
        let plan = plan_with_belief(belief.clone(), true);
        let outcome = memory
            .commit(plan.clone(), CommitOptions::default())
            .await
            .unwrap();
        memory.commit(plan, CommitOptions::default()).await.unwrap();
        let graph = &memory.memory_composition.graph_store;
        let stored_links = graph
            .query_links_by_ids(&outcome.persisted_link_ids)
            .await
            .unwrap();
        assert_eq!(stored_links.len(), 1);
        assert!(stored_links
            .iter()
            .all(|link| link.relation == RelationType::About));
        let objects = graph
            .query_objects(&GraphObjectQuery::by_ids(vec![subject, belief.id.unwrap()]))
            .await
            .unwrap();
        for (root_id, root_type) in [
            (subject, ObjectType::Entity),
            (belief.id.unwrap(), ObjectType::DerivedMemory),
        ] {
            let query = GraphExpansionQuery::new(root_id, root_type, 2, 2)
                .with_max_hub_edges(1)
                .with_max_fanout_per_node(1)
                .with_fanout_utilization_recording(TraceMode::Enabled);
            for expanded in [
                graph.expand_bounded(&query).await.unwrap(),
                crate::policy::graph_expansion::bounded_expansion(
                    &query,
                    objects.clone(),
                    stored_links.clone(),
                )
                .unwrap(),
            ] {
                assert_eq!(expanded.objects.len(), 2);
                assert_eq!(expanded.links.len(), 1);
                assert_eq!(expanded.relations.len(), 1);
                assert!(expanded.bounded_failure.is_none());
                assert!(expanded
                    .fanout_utilization
                    .iter()
                    .all(|item| item.retained_count == 1 && item.omitted_by_fanout_count == 0));
            }
        }
        let counter = memory
            .memory_composition
            .stats_store
            .counter(&RetrievalStatsCounterKey {
                entity_id: subject,
                relation_kind: RelationType::About,
                object_type: ObjectType::DerivedMemory,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(counter.total_count, 1);
        assert_eq!(counter.current_count, 1);
        let mut given_text = given_belief(MemoryId::from_u128(6303), subject, "unasserted");
        given_text.assertions.clear();
        assert_eq!(
            memory
                .commit(
                    plan_with_belief(given_text, false),
                    CommitOptions::default()
                )
                .await
                .unwrap()
                .persisted_link_ids
                .len(),
            1
        );
        assert!(memory
            .memory_composition
            .graph_store
            .query_notions_known_as("unasserted")
            .await
            .unwrap()
            .is_empty());
        let mut ordinary =
            DerivedMemoryDraft::new(DerivedType::Claim, "I am unsure what to call this notion.");
        ordinary.entity_ids = vec![subject];
        let ordinary_outcome = memory
            .remember(
                RememberInput::new("An experience about a notion.").with_derived_memory(ordinary),
                RememberOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(ordinary_outcome.persisted_link_ids.len(), 1);
        let plain_link = graph
            .query_links_by_ids(&ordinary_outcome.persisted_link_ids)
            .await
            .unwrap();
        assert_eq!(plain_link[0].to_id, subject);
        assert_eq!(plain_link[0].relation, RelationType::About);
        let counter = memory
            .memory_composition
            .stats_store
            .counter(&RetrievalStatsCounterKey {
                entity_id: subject,
                relation_kind: RelationType::About,
                object_type: ObjectType::DerivedMemory,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(counter.total_count, 3);
        assert_eq!(counter.current_count, 3);
        memory.close().await.unwrap();
    }
}

#[tokio::test]
async fn given_belief_correction_requires_declared_grounding_and_replays() {
    let memory = memory().await;
    let subject = MemoryId::from_u128(6401);
    let original_id = MemoryId::from_u128(6402);
    let replacement_id = MemoryId::from_u128(6403);
    memory
        .commit(
            plan_with_belief(given_belief(original_id, subject, "Alice"), true),
            CommitOptions::default(),
        )
        .await
        .unwrap();
    let origin = SourceProvenanceReference {
        episode_ids: vec![],
        observation_ids: vec![],
        external_refs: vec![ExternalSourceReference::source("application:rename")],
    };
    let mut correction = CorrectMemoryDraft::new(
        CorrectionTarget::derived_memory(original_id),
        "The given name is Bob.",
    );
    correction.correction_origin = origin.clone();
    assert!(matches!(
        memory.correct(correction.clone()).await,
        Err(CustomError::LifecycleDraftInvalid(
            LifecycleDtoValidationError::MissingGivenReplacement
        ))
    ));
    let mut replacement =
        ReplacementDerivedMemoryDraft::new(DerivedType::Correction, "The given name is Bob.");
    replacement.id = Some(replacement_id);
    replacement.entity_ids = vec![subject];
    replacement.assertions = vec![assertion(subject, "Bob")];
    replacement.given_by_application = true;
    replacement.correction_origin_provenance = origin;
    correction.replacement_derived_memories.push(replacement);
    correction.include_trace = true;
    let written = memory.correct(correction.clone()).await.unwrap();
    assert_eq!(written.trace.unwrap().superseded_by.len(), 1);
    assert!(!written
        .vector_maintained_object_ids
        .contains(&MemoryObjectRef::new(ObjectType::Entity, subject)));
    let replay = memory.correct(correction).await.unwrap();
    assert!(replay.graph_mutated_object_ids.is_empty());
    assert!(replay.graph_mutated_link_ids.is_empty());
    assert_eq!(
        memory
            .memory_composition
            .graph_store
            .query_notions_known_as("Bob")
            .await
            .unwrap(),
        vec![subject]
    );
    assert!(memory
        .memory_composition
        .graph_store
        .query_notions_known_as("Alice")
        .await
        .unwrap()
        .is_empty());
    memory.close().await.unwrap();
}

#[tokio::test]
async fn given_replacement_does_not_inherit_predecessor_sources() {
    let memory = memory().await;
    let subject = MemoryId::from_u128(6501);
    let original_id = MemoryId::from_u128(6502);
    let replacement_id = MemoryId::from_u128(6503);
    let mut notion = EntityDraft::new();
    notion.id = Some(subject);
    let mut observed = given_belief(original_id, subject, "Alice");
    observed.given_by_application = false;
    memory
        .remember(
            RememberInput::new("I heard the name Alice.")
                .with_entity(notion)
                .with_derived_memory(observed),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    let graph = &memory.memory_composition.graph_store;
    let sources = graph
        .query_objects(&GraphObjectQuery::by_types(vec![ObjectType::Episode], None))
        .await
        .unwrap();
    let origin = SourceProvenanceReference {
        episode_ids: vec![],
        observation_ids: vec![],
        external_refs: vec![ExternalSourceReference::source("application:new-name")],
    };
    let mut replacement =
        ReplacementDerivedMemoryDraft::new(DerivedType::Correction, "The given name is Bob.");
    replacement.id = Some(replacement_id);
    replacement.entity_ids = vec![subject];
    replacement.given_by_application = true;
    replacement.assertions = vec![assertion(subject, "Bob")];
    replacement.correction_origin_provenance = origin.clone();
    let mut correction = CorrectMemoryDraft::new(
        CorrectionTarget::derived_memory(original_id),
        "Give a new name.",
    )
    .with_replacement(replacement);
    correction.correction_origin = origin;
    let mut mixed = correction.clone();
    mixed.replacement_derived_memories[0].derived_from_episode_ids = vec![sources[0].id()];
    assert!(matches!(
        memory.correct(mixed).await,
        Err(CustomError::LifecycleDraftInvalid(
            LifecycleDtoValidationError::InvalidBelief(BeliefValidationError::GivenWithSources)
        ))
    ));
    let mut missing_subject = correction.clone();
    let plain = &mut missing_subject.replacement_derived_memories[0];
    plain.given_by_application = false;
    plain.assertions.clear();
    plain.entity_ids = vec![sources[0].id()]; // An episode is not a notion.
    plain.derived_from_episode_ids = vec![sources[0].id()];
    assert!(matches!(memory.correct(missing_subject).await,
        Err(CustomError::GraphExpansionRootNotFound { object_type: ObjectType::Entity, object_id })
        if object_id == sources[0].id()));
    memory.correct(correction).await.unwrap();
    let stored = graph
        .query_objects(&GraphObjectQuery::by_ids(vec![replacement_id]))
        .await
        .unwrap();
    let MemoryObject::DerivedMemory(stored) = &stored[0] else {
        panic!("expected replacement");
    };
    assert!(stored.given_by_application);
    assert!(stored.derived_from_episode_ids.is_empty());
    assert!(stored.derived_from_observation_ids.is_empty());
    assert_eq!(
        graph.query_notions_known_as("Bob").await.unwrap(),
        vec![subject]
    );
    assert!(graph
        .query_notions_known_as("Alice")
        .await
        .unwrap()
        .is_empty());
    memory.close().await.unwrap();
}

#[tokio::test]
async fn repeated_episode_and_belief_id_sets_commit_and_replay_canonically() {
    let memory = memory().await;
    let subjects = [MemoryId::from_u128(6901), MemoryId::from_u128(6902)];
    let predecessors = [MemoryId::from_u128(6905), MemoryId::from_u128(6906)];
    let mut stored = RememberWritePlan::new();
    let belief_id = MemoryId::from_u128(6903);
    let episode_id = MemoryId::from_u128(6904);
    let mut belief = given_belief(belief_id, subjects[1], "Alice");
    belief.entity_ids = vec![subjects[1], subjects[0], subjects[0]];
    belief.supersedes = vec![predecessors[1], predecessors[0], predecessors[0]];
    let mut plan = plan_with_belief(belief.clone(), false);
    for subject in subjects {
        let mut notion = EntityDraft::new();
        notion.id = Some(subject);
        notion.created_at = belief.created_at;
        notion.schema_version = belief.schema_version.clone();
        let candidate = MemoryCandidate::Entity(EntityCandidate::new(
            notion,
            CandidateProvenance::caller("notion"),
        ));
        plan = plan.with_candidate(candidate.clone());
        stored = stored.with_candidate(candidate);
    }
    for predecessor in predecessors {
        stored =
            stored.with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
                given_belief(predecessor, subjects[0], "An earlier name"),
                CandidateProvenance::caller("earlier belief"),
            )));
    }
    memory
        .commit(stored, CommitOptions::default())
        .await
        .unwrap();
    let mut episode = EpisodeDraft::new("An experience with two notions.");
    episode.id = Some(episode_id);
    let mut scene = crate::Scene::at(belief.created_at.unwrap());
    scene.participants = [subjects[1], subjects[0], subjects[0]]
        .into_iter()
        .map(crate::SceneParticipant::Key)
        .collect();
    episode.scene = Some(scene.clone());
    episode.created_at = belief.created_at;
    episode.schema_version = belief.schema_version.clone();
    plan = plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
        episode,
        CandidateProvenance::caller("experience"),
    )));
    let first = memory
        .commit(plan.clone(), CommitOptions::default())
        .await
        .unwrap();
    let replay = memory.commit(plan, CommitOptions::default()).await.unwrap();
    assert_eq!(first.persisted_object_ids, replay.persisted_object_ids);
    assert_eq!(first.persisted_link_ids, replay.persisted_link_ids);
    let objects = memory
        .memory_composition
        .graph_store
        .query_objects(&GraphObjectQuery::by_ids(vec![episode_id, belief_id]))
        .await
        .unwrap();
    assert_eq!(objects.len(), 2);
    for object in objects {
        match object {
            MemoryObject::Episode(episode) => assert_eq!(episode.scene, scene),
            MemoryObject::DerivedMemory(memory) => {
                assert_eq!(memory.entity_ids, subjects);
                assert_eq!(memory.supersedes, predecessors);
                assert_eq!(memory.assertions, belief.assertions);
            }
            _ => panic!("expected the authored episode or belief"),
        }
    }
    memory.close().await.unwrap();
}

#[tokio::test]
async fn authored_belief_about_links_are_rejected_in_both_directions() {
    let memory = memory().await;
    let subject = MemoryId::from_u128(6801);
    let other_subject = MemoryId::from_u128(6802);
    let belief_id = MemoryId::from_u128(6803);
    let episode_id = MemoryId::from_u128(6804);
    let observation_id = MemoryId::from_u128(6805);
    let notion = |id| {
        let mut draft = EntityDraft::new();
        draft.id = Some(id);
        draft
    };
    let mut belief = DerivedMemoryDraft::new(DerivedType::Reflection, "A belief about one notion.");
    belief.id = Some(belief_id);
    belief.entity_ids = vec![subject];
    let mut episode = EpisodeDraft::new("An experience about the notion.");
    episode.id = Some(episode_id);
    let mut observation = ObservationDraft::new(episode_id, "I observed the notion.");
    observation.id = Some(observation_id);
    let valid = memory
        .prepare(
            RememberInput::new("An experience about the notion.")
                .with_episode(episode)
                .with_observation(observation)
                .with_entity(notion(subject))
                .with_entity(notion(other_subject))
                .with_derived_memory(belief),
            PrepareOptions::default(),
        )
        .await
        .unwrap();
    let mut rejected = Vec::new();
    for target in [subject, other_subject] {
        for reverse in [false, true] {
            let mut draft = MemoryLinkDraft::new(
                ObjectType::DerivedMemory,
                belief_id,
                RelationType::About,
                ObjectType::Entity,
                target,
            );
            if reverse {
                std::mem::swap(&mut draft.from_id, &mut draft.to_id);
                std::mem::swap(&mut draft.from_type, &mut draft.to_type);
            }
            draft.id = Some(MemoryId::from_u128(6810 + rejected.len() as u128));
            draft.created_at = Some(chrono::Utc::now());
            draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
            let invalid = valid.clone().with_candidate(MemoryCandidate::MemoryLink(
                MemoryLinkCandidate::new(
                    draft.clone(),
                    CandidateProvenance::caller("subject link"),
                ),
            ));
            let validations = memory.validate_plan(&invalid).await.unwrap();
            assert_eq!(
                validations.last().unwrap().errors,
                vec![CandidateValidationIssue::AuthoredBeliefAboutLink]
            );
            assert!(
                matches!(memory.commit(invalid, CommitOptions::default()).await,
                Err(CustomError::WritePlanValidationRejected { validations })
                if validations.last().unwrap().errors == vec![CandidateValidationIssue::AuthoredBeliefAboutLink])
            );
            rejected.push(draft);
        }
    }
    let graph = &memory.memory_composition.graph_store;
    assert!(graph
        .query_objects(&GraphObjectQuery::by_ids(vec![
            subject,
            other_subject,
            belief_id,
            episode_id,
            observation_id
        ]))
        .await
        .unwrap()
        .is_empty());
    let first = memory
        .commit(valid.clone(), CommitOptions::default())
        .await
        .unwrap();
    let replay = memory
        .commit(valid, CommitOptions::default())
        .await
        .unwrap();
    assert_eq!(first.persisted_link_ids, replay.persisted_link_ids);
    let derived = graph
        .query_links_by_ids(&first.persisted_link_ids)
        .await
        .unwrap();
    assert!(matches!(derived.as_slice(), [link]
        if link.from_id == belief_id && link.from_type == ObjectType::DerivedMemory
        && link.to_id == subject && link.to_type == ObjectType::Entity && link.relation == RelationType::About));
    for draft in &rejected {
        assert!(matches!(
            memory.link(draft.clone()).await,
            Err(CustomError::DomainValidation(
                DomainValidationError::AuthoredBeliefAboutLink
            ))
        ));
    }
    assert!(graph
        .query_links_by_ids(
            &rejected
                .iter()
                .map(|draft| draft.id.unwrap())
                .collect::<Vec<_>>()
        )
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        graph
            .query_links_by_ids(&first.persisted_link_ids)
            .await
            .unwrap(),
        derived
    );

    // About remains available for other endpoint pairs, through both writers.
    for (from_type, from_id, to_type, to_id) in [
        (
            ObjectType::DerivedMemory,
            belief_id,
            ObjectType::Observation,
            observation_id,
        ),
        (
            ObjectType::Observation,
            observation_id,
            ObjectType::DerivedMemory,
            belief_id,
        ),
        (ObjectType::Episode, episode_id, ObjectType::Entity, subject),
        (ObjectType::Entity, subject, ObjectType::Episode, episode_id),
    ] {
        let linked = memory
            .link(MemoryLinkDraft::new(
                from_type,
                from_id,
                RelationType::About,
                to_type,
                to_id,
            ))
            .await
            .unwrap()
            .link;
        let draft = MemoryLinkDraft {
            id: Some(linked.id),
            created_at: Some(linked.created_at),
            schema_version: Some(linked.schema_version),
            ..MemoryLinkDraft::new(from_type, from_id, RelationType::About, to_type, to_id)
        };
        let plan = RememberWritePlan::new().with_candidate(MemoryCandidate::MemoryLink(
            MemoryLinkCandidate::new(draft, CandidateProvenance::caller("non-subject link")),
        ));
        assert!(memory
            .validate_plan(&plan)
            .await
            .unwrap()
            .iter()
            .all(|item| item.status == CandidateValidationStatus::Valid));
        memory.commit(plan, CommitOptions::default()).await.unwrap();
    }
    memory.close().await.unwrap();
}

#[tokio::test]
async fn commit_rejects_authored_link_ids_colliding_with_generated_or_authored_links() {
    for reuse_generated_id in [true, false] {
        let memory = memory().await;
        let subject = MemoryId::from_u128(1991);
        let belief_id = MemoryId::from_u128(1992);
        let belief = given_belief(belief_id, subject, "Alice");
        let plan = plan_with_belief(belief.clone(), true);
        let outcome = memory
            .commit(plan.clone(), CommitOptions::default())
            .await
            .unwrap();
        assert_eq!(outcome.persisted_link_ids.len(), 1);
        let link_id = if reuse_generated_id {
            outcome.persisted_link_ids[0]
        } else {
            MemoryId::from_u128(1993)
        };
        let mut authored = MemoryLinkDraft::new(
            ObjectType::Entity,
            subject,
            RelationType::AssociatedWith,
            ObjectType::DerivedMemory,
            belief_id,
        );
        authored.id = Some(link_id);
        authored.created_at = belief.created_at;
        authored.schema_version = belief.schema_version;
        let authored = MemoryCandidate::MemoryLink(MemoryLinkCandidate::new(
            authored,
            CandidateProvenance::caller("authored collision"),
        ));
        let plan = if reuse_generated_id {
            plan.with_candidate(authored)
        } else {
            RememberWritePlan::new()
                .with_candidate(authored.clone())
                .with_candidate(authored)
        };
        assert!(
            matches!(memory.commit(plan, CommitOptions::default()).await,
            Err(CustomError::DeterministicIdCollision { object })
                if object == MemoryObjectRef::new(ObjectType::MemoryLink, link_id))
        );
        memory.close().await.unwrap();
    }
}
