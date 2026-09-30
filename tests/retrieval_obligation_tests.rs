use character_memory::*;
use serde_json::{json, Value};
use test_support::{id, keyed};

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
async fn direction_survives_reopen_without_changing_recall() {
    let mut without_roles = Vec::new();
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
            assert_eq!(snapshots, without_roles);
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
