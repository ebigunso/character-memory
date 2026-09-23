use character_memory::{
    BeliefAssertion, BeliefPredicate, CorrectMemoryDraft, CorrectionTarget, CustomError,
    DerivedMemoryDraft, DerivedType, EntityDraft, EpisodeDraft, ForgetMemoryDraft,
    GraphFailureMode, LifecycleTargetRef, MemoryId, MemoryLinkDraft, ObjectType, RelationType,
    RememberInput, RememberOptions, ReplacementDerivedMemoryDraft, RetrievalCandidateLimits,
    RetrievalContext, RetrievalGraphLimits, SourceProvenanceReference,
};
use chrono::{DateTime, Utc};
use tempfile::TempDir;
use test_support::{ensure, parse_id as id};
use uuid::Uuid;

#[path = "support/mod.rs"]
pub mod test_support;

#[tokio::test]
async fn name_belief_and_derived_about_link_survive_facade_reopen() {
    let root = TempDir::new().expect("store root should be created");
    let collection_name = test_support::unique_collection_name();
    let entity_id = id("550e8400-e29b-41d4-a716-446655461001");
    let episode_id = id("550e8400-e29b-41d4-a716-446655461002");
    let memory_id = id("550e8400-e29b-41d4-a716-446655461003");

    let memory = setup(&collection_name, &root, None)
        .await
        .expect("unexpected stats persistence setup failure");

    let test_result = async {
        let remember_outcome = memory
            .remember(
                RememberInput::new("A neutral field note records archival calibration.")
                    .with_entity(entity(entity_id))
                    .with_episode(episode(
                        episode_id,
                        "A neutral field note records archival calibration.",
                        &[entity_id],
                    ))
                    .with_derived_memory(named_belief(
                        memory_id,
                        entity_id,
                        "Aster Archive",
                        episode_id,
                    ))
                    .with_memory_link(link(
                        id("550e8400-e29b-41d4-a716-446655461005"),
                        ObjectType::Entity,
                        entity_id,
                        RelationType::Involves,
                        ObjectType::Episode,
                        episode_id,
                    )),
                RememberOptions::default(),
            )
            .await
            .map_err(|error| {
                format!("initial remember should populate graph/vector/stats stores: {error}")
            })?;
        ensure_no_vector_indexing_failure(
            &remember_outcome,
            "initial stats persistence remember should index vectors",
        )?;

        memory
            .close()
            .await
            .map_err(|error| format!("facade should close before reopen: {error}"))?;

        let reopened = setup(&collection_name, &root, None)
            .await
            .map_err(|error| {
                format!("reopened facade should use same persistent stores: {error}")
            })?;
        let retrieved = reopened
            .retrieve(belief_root_context("Aster Archive"))
            .await
            .map_err(|error| format!("retrieve after reopen should succeed: {error}"))?;

        ensure(
            returned_derived_ids(&retrieved).contains(&memory_id),
            "name belief should survive reopen",
        )?;
        let name_belief = &retrieved
            .pack
            .derived_memories
            .iter()
            .find(|entry| entry.memory.id == memory_id)
            .unwrap()
            .memory;
        assert_eq!(
            name_belief.assertions,
            vec![BeliefAssertion {
                subject: entity_id,
                predicate: BeliefPredicate::KnownAs {
                    name: "Aster Archive".to_owned()
                },
            }]
        );
        assert!(!name_belief.given_by_application);
        let trace = retrieved.trace.as_ref().unwrap();
        ensure(
            trace
                .vector_candidates
                .iter()
                .all(|candidate| candidate.object.object_type != ObjectType::Entity),
            "notions have no vector candidates",
        )?;
        ensure(
            trace.selectivity_decisions.is_empty(),
            "content roots do not use entity selectivity",
        )?;
        ensure(
            trace.fanout_utilization.iter().any(|entry| {
                entry.root.id == memory_id
                    && entry.relation == RelationType::About
                    && entry.object_type == ObjectType::Entity
                    && entry.retained_count == 1
            }),
            "content belief should reach its notion through the persisted About link",
        )?;

        Ok::<_, String>(reopened)
    }
    .await;

    let reopened = test_result.expect("stats persistence test should pass");
    test_support::close_and_remove_root(reopened, root).await;
}

#[tokio::test]
async fn restart_safe_retrieval_excludes_suppressed_and_superseded_memories() {
    let root = TempDir::new().expect("store root should be created");
    let collection_name = test_support::unique_collection_name();
    let entity_id = id("550e8400-e29b-41d4-a716-446655462001");
    let episode_id = id("550e8400-e29b-41d4-a716-446655462002");
    let old_id = id("550e8400-e29b-41d4-a716-446655462003");
    let suppressed_id = id("550e8400-e29b-41d4-a716-446655462004");
    let replacement_id = id("550e8400-e29b-41d4-a716-446655462005");

    let memory = setup(&collection_name, &root, None)
        .await
        .expect("unexpected restart-safe setup failure");

    let test_result = async {
        let remember_outcome = memory
            .remember(
                RememberInput::new("Ledger Meridian captured a restart-safe correction fixture.")
                    .with_entity(entity(entity_id))
                    .with_episode(episode(
                        episode_id,
                        "Ledger Meridian captured a restart-safe correction fixture.",
                        &[entity_id],
                    ))
                    .with_derived_memory(derived(
                        old_id,
                        DerivedType::Claim,
                        "Ledger Meridian should keep the stale restart-safe statement.",
                        episode_id,
                        &[entity_id],
                    ))
                    .with_derived_memory(derived(
                        suppressed_id,
                        DerivedType::ProjectNote,
                        "Ledger Meridian contains a suppressed restart-safe note.",
                        episode_id,
                        &[entity_id],
                    )),
                RememberOptions::default(),
            )
            .await
            .map_err(|error| format!("initial lifecycle remember should succeed: {error}"))?;
        ensure_no_vector_indexing_failure(
            &remember_outcome,
            "initial lifecycle remember should index vectors",
        )?;

        let mut replacement = ReplacementDerivedMemoryDraft::new(
            DerivedType::Correction,
            "Ledger Meridian should retrieve only the corrected restart-safe statement.",
        )
        .with_source_episode(episode_id)
        .with_superseded_memory(old_id);
        replacement.id = Some(replacement_id);
        replacement.entity_ids.push(entity_id);
        replacement.original_source_provenance = SourceProvenanceReference::episode(episode_id);
        replacement.correction_origin_provenance = SourceProvenanceReference::episode(episode_id);

        let mut correction = CorrectMemoryDraft::new(
            CorrectionTarget::derived_memory(old_id),
            "Replace stale restart-safe statement.",
        )
        .with_replacement(replacement)
        .with_superseded_derived_memory(old_id);
        correction.correction_origin = SourceProvenanceReference::episode(episode_id);

        memory
            .correct(correction)
            .await
            .map_err(|error| format!("correction should supersede old memory: {error}"))?;
        memory
            .forget(ForgetMemoryDraft::suppress(
                LifecycleTargetRef::derived_memory(suppressed_id),
                "Suppress stale restart-safe note.",
            ))
            .await
            .map_err(|error| format!("suppression should persist lifecycle state: {error}"))?;

        memory
            .close()
            .await
            .map_err(|error| format!("facade should close before reopen: {error}"))?;

        let reopened = setup(&collection_name, &root, None)
            .await
            .map_err(|error| {
                format!("reopened lifecycle facade should use same stores: {error}")
            })?;
        let retrieved = reopened
            .retrieve(RetrievalContext::new(
                "Ledger Meridian corrected restart-safe statement",
            ))
            .await
            .map_err(|error| format!("retrieve after lifecycle reopen should succeed: {error}"))?;
        let returned_ids = returned_derived_ids(&retrieved);

        ensure(
            returned_ids.contains(&replacement_id),
            "retrieval should include the superseding replacement after reopen",
        )?;
        ensure(
            !returned_ids.contains(&old_id),
            "retrieval should exclude superseded memory after reopen",
        )?;
        ensure(
            !returned_ids.contains(&suppressed_id),
            "retrieval should exclude suppressed memory after reopen",
        )?;

        Ok::<_, String>(reopened)
    }
    .await;

    let reopened = test_result.expect("restart-safe retrieval test should pass");
    test_support::close_and_remove_root(reopened, root).await;
}

#[tokio::test]
async fn belief_content_reaches_notion_and_static_caps_bound_expansion() {
    let root = TempDir::new().unwrap();
    let collection_name = test_support::unique_collection_name();
    let ids = HighDegreeIds::new();
    let memory = setup(&collection_name, &root, None).await.unwrap();
    let written = memory
        .remember(high_degree_fixture(&ids), RememberOptions::default())
        .await
        .unwrap();
    assert!(written.vector_indexing_failure.is_none());
    let broad = memory
        .retrieve(belief_root_context("Vector Orchard"))
        .await
        .unwrap();
    let trace = broad.trace.as_ref().unwrap();
    assert_eq!(trace.vector_candidates[0].object.id, ids.name_belief);
    assert!(trace
        .vector_candidates
        .iter()
        .all(|candidate| candidate.object.object_type != ObjectType::Entity));
    assert!(trace.selectivity_decisions.is_empty());
    assert_eq!(broad.rationale.telemetry.selectivity.decision_count, 0);
    assert!(trace
        .fanout_utilization
        .iter()
        .any(|entry| entry.root.id == ids.hub_entity
            && entry.relation == RelationType::About
            && entry.retained_count > 2));
    let broad_count = returned_derived_ids(&broad)
        .iter()
        .filter(|id| ids.hub_derived_ids.contains(id))
        .count();
    assert!(broad_count > 2);
    // Candidate scope and graph scope are independent: a belief-only vector search
    // can still cross a notion, while excluding notions from traversal blocks that hop.
    let mut belief_candidates = belief_root_context("Vector Orchard");
    belief_candidates.object_type_defaults = vec![ObjectType::DerivedMemory];
    let belief_only = memory.retrieve(belief_candidates).await.unwrap();
    assert_eq!(
        returned_derived_ids(&belief_only),
        returned_derived_ids(&broad)
    );
    assert!(belief_only
        .trace
        .as_ref()
        .unwrap()
        .vector_candidates
        .iter()
        .all(|candidate| candidate.object.object_type == ObjectType::DerivedMemory));
    let mut belief_graph = belief_root_context("Vector Orchard");
    belief_graph.graph_limits.allowed_object_types = vec![ObjectType::DerivedMemory];
    let no_notion_hop = memory.retrieve(belief_graph).await.unwrap();
    assert_eq!(returned_derived_ids(&no_notion_hop), vec![ids.name_belief]);
    assert_eq!(
        no_notion_hop.trace.as_ref().unwrap().vector_candidates,
        trace.vector_candidates
    );
    let mut limited = belief_root_context("Vector Orchard");
    limited.graph_limits.max_fanout_per_node = 2;
    let bounded = memory.retrieve(limited).await.unwrap();
    let count = returned_derived_ids(&bounded)
        .iter()
        .filter(|id| ids.hub_derived_ids.contains(id))
        .count();
    assert!(count <= 2 && count < broad_count);
    assert!(bounded
        .trace
        .as_ref()
        .unwrap()
        .fanout_utilization
        .iter()
        .any(|entry| entry.root.id == ids.hub_entity
            && entry.relation == RelationType::About
            && entry.selected_cap == 2
            && entry.retained_count <= 2
            && entry.omitted_by_fanout_count > 0));
    test_support::close_and_remove_root(memory, root).await;
}

async fn setup(
    collection_name: &str,
    root: &TempDir,
    fanout: Option<(usize, usize)>,
) -> Result<character_memory::CharacterMemory, CustomError> {
    test_support::try_setup_persistent_character_memory(
        collection_name.to_owned(),
        root.path(),
        fanout,
    )
    .await
}

fn belief_root_context(query: &str) -> RetrievalContext {
    let mut context = RetrievalContext::new(query).with_trace();
    context.candidate_limits = RetrievalCandidateLimits {
        max_vector_candidates: 32,
        max_graph_roots: 1,
    };
    context.graph_limits = RetrievalGraphLimits {
        max_depth: 2,
        max_nodes: 64,
        max_fanout_per_node: 32,
        max_hub_edges: 64,
        timeout_ms: Some(500),
        failure_mode: GraphFailureMode::AllowPartialResults,
        allowed_relation_types: Vec::new(),
        ..RetrievalGraphLimits::default()
    };
    context
}

fn entity(id: MemoryId) -> EntityDraft {
    let mut draft = EntityDraft::new();
    draft.id = Some(id);
    draft.created_at = Some(timestamp());
    draft
}

fn episode(id: MemoryId, summary: &str, participants: &[MemoryId]) -> EpisodeDraft {
    let mut draft = EpisodeDraft::new(summary);
    draft.id = Some(id);
    let mut scene = character_memory::Scene::at((timestamp()).fixed_offset());
    scene.participants = participants
        .iter()
        .copied()
        .map(|key| character_memory::SceneParticipant {
            key: Some(key),
            ..Default::default()
        })
        .collect();
    draft.scene = Some(scene);
    draft.ended_at = Some(timestamp());
    draft.created_at = Some(timestamp());
    draft.raw_ref = Some(format!("raw://integration/v0-1-2/{id}"));
    draft
}

fn derived(
    id: MemoryId,
    derived_type: DerivedType,
    text: &str,
    episode_id: MemoryId,
    entity_ids: &[MemoryId],
) -> DerivedMemoryDraft {
    let mut draft = DerivedMemoryDraft::new(derived_type, text).with_source_episode(episode_id);
    draft.id = Some(id);
    draft.entity_ids = entity_ids.to_vec();
    draft.created_at = Some(timestamp());
    draft.updated_at = Some(timestamp());
    draft
}

fn link(
    id: MemoryId,
    from_type: ObjectType,
    from_id: MemoryId,
    relation: RelationType,
    to_type: ObjectType,
    to_id: MemoryId,
) -> MemoryLinkDraft {
    let mut draft = MemoryLinkDraft::new(from_type, from_id, relation, to_type, to_id);
    draft.id = Some(id);
    draft.created_at = Some(timestamp());
    draft
}

struct HighDegreeIds {
    name_belief: MemoryId,
    hub_entity: MemoryId,
    other_entities: [MemoryId; 4],
    hub_derived_ids: Vec<MemoryId>,
    other_derived_ids: Vec<MemoryId>,
}

impl HighDegreeIds {
    fn new() -> Self {
        Self {
            name_belief: MemoryId::from_u128(6501),
            hub_entity: id("550e8400-e29b-41d4-a716-446655463001"),
            other_entities: [
                id("550e8400-e29b-41d4-a716-446655463002"),
                id("550e8400-e29b-41d4-a716-446655463003"),
                id("550e8400-e29b-41d4-a716-446655463004"),
                id("550e8400-e29b-41d4-a716-446655463005"),
            ],
            hub_derived_ids: (0..8)
                .map(|offset| Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5546_3100 + offset))
                .collect(),
            other_derived_ids: (0..16)
                .map(|offset| Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5546_3200 + offset))
                .collect(),
        }
    }
}

fn high_degree_fixture(ids: &HighDegreeIds) -> RememberInput {
    let mut input = RememberInput::new(
        "A high-degree project fixture records neutral retrieval expansion pressure.",
    )
    .with_entity(entity(ids.hub_entity));
    for entity_id in ids.other_entities {
        input = input.with_entity(entity(entity_id));
    }

    let hub_episode_id = id("550e8400-e29b-41d4-a716-446655463010");
    input = input.with_episode(episode(
        hub_episode_id,
        "A high-degree project fixture records neutral retrieval expansion pressure.",
        &[ids.hub_entity],
    ));
    input = input.with_derived_memory(named_belief(
        ids.name_belief,
        ids.hub_entity,
        "Vector Orchard",
        hub_episode_id,
    ));
    for (index, memory_id) in ids.hub_derived_ids.iter().copied().enumerate() {
        input = input.with_derived_memory(derived(
            memory_id,
            DerivedType::ProjectNote,
            &format!("Bounded recall note {index} for orchard calibration pressure."),
            hub_episode_id,
            &[ids.hub_entity],
        ));
    }

    for (index, memory_id) in ids.other_derived_ids.iter().copied().enumerate() {
        let entity_id = ids.other_entities[index % ids.other_entities.len()];
        input = input.with_derived_memory(derived(
            memory_id,
            DerivedType::Claim,
            &format!("Auxiliary neutral memory {index} expands global selectivity counts."),
            hub_episode_id,
            &[entity_id],
        ));
    }

    input
}

fn returned_derived_ids(outcome: &character_memory::RetrieveOutcome) -> Vec<MemoryId> {
    outcome
        .pack
        .derived_memories
        .iter()
        .chain(outcome.pack.preferences.iter())
        .chain(outcome.pack.relationship_notes.iter())
        .chain(outcome.pack.open_loops.iter())
        .chain(outcome.pack.commitments.iter())
        .chain(outcome.pack.character_signals.iter())
        .map(|included| included.memory.id)
        .collect()
}

fn timestamp() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-06-12T10:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn ensure_no_vector_indexing_failure(
    outcome: &character_memory::RememberOutcome,
    context: &'static str,
) -> Result<(), String> {
    if let Some(failure) = &outcome.vector_indexing_failure {
        return Err(format!(
            "{context}: vector indexing failed for object ids {:?}; persisted object ids {:?}; indexed object ids {:?}; error: {}",
            failure.unindexed_object_ids(),
            outcome.persisted_object_ids,
            outcome.vector_indexed_object_ids,
            failure.cause
        ));
    }

    Ok(())
}

fn named_belief(
    id: MemoryId,
    subject: MemoryId,
    name: &str,
    source: MemoryId,
) -> DerivedMemoryDraft {
    let mut draft = derived(id, DerivedType::Claim, name, source, &[subject]);
    draft.assertions.push(BeliefAssertion {
        subject,
        predicate: BeliefPredicate::KnownAs {
            name: name.to_owned(),
        },
    });
    draft
}
