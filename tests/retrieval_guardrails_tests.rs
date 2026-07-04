use character_memory::{
    CorrectMemoryDraft, CorrectionTarget, CustomError, DerivedMemoryDraft, DerivedType,
    EntityDraft, EntityType, EpisodeDraft, ForgetMemoryDraft, LifecycleTargetRef, MemoryId,
    MemoryLinkDraft, MemoryObjectDraft, ObjectType, RelationType, RememberDraft,
    ReplacementDerivedMemoryDraft, RetrievalCandidateLimits, RetrievalContext,
    RetrievalGraphLimits, SourceProvenanceReference,
};
use chrono::{DateTime, Utc};
use tempfile::TempDir;
use uuid::Uuid;

#[path = "support/persistent.rs"]
mod test_utils;

#[tokio::test]
async fn stats_persist_across_facade_reopen() {
    let fixture = StoreFixture::new();
    let collection_name = test_utils::unique_collection_name();
    let entity_id = id("550e8400-e29b-41d4-a716-446655461001");
    let episode_id = id("550e8400-e29b-41d4-a716-446655461002");
    let memory_id = id("550e8400-e29b-41d4-a716-446655461003");

    let memory = match setup(&collection_name, &fixture, None).await {
        Ok(memory) => memory,
        Err(CustomError::VectorDatabaseError(error))
            if test_utils::is_qdrant_unavailable_error(&error) =>
        {
            println!(
                "skipping v0.1.2 stats persistence test because Qdrant is unavailable: {error}"
            );
            return;
        }
        Err(error) => panic!("unexpected v0.1.2 stats persistence setup failure: {error}"),
    };

    let test_result = async {
        let remember_outcome = memory
            .remember(
                RememberDraft::new([
                    MemoryObjectDraft::Entity(entity(entity_id, EntityType::Person, "Aster Archive")),
                    MemoryObjectDraft::Episode(episode(
                        episode_id,
                        "A neutral field note records archival calibration.",
                        &[entity_id],
                    )),
                    MemoryObjectDraft::DerivedMemory(derived(
                        memory_id,
                        DerivedType::Claim,
                        "Archival calibration should prefer bounded recall over broad expansion.",
                        episode_id,
                        &[entity_id],
                    )),
                ])
                .with_links([
                    link(
                        id("550e8400-e29b-41d4-a716-446655461004"),
                        ObjectType::Entity,
                        entity_id,
                        RelationType::About,
                        ObjectType::DerivedMemory,
                        memory_id,
                    ),
                    link(
                        id("550e8400-e29b-41d4-a716-446655461005"),
                        ObjectType::Entity,
                        entity_id,
                        RelationType::Involves,
                        ObjectType::Episode,
                        episode_id,
                    ),
                ]),
            )
            .await
            .map_err(|error| format!("initial remember should populate graph/vector/stats stores: {error}"))?;
        ensure_no_vector_indexing_failure(
            &remember_outcome,
            "initial stats persistence remember should index vectors",
        )?;

        drop(memory);

        let reopened = setup(&collection_name, &fixture, None)
            .await
            .map_err(|error| format!("reopened facade should use same persistent stores: {error}"))?;
        let retrieved = reopened
            .retrieve(entity_root_context("Aster Archive"))
            .await
            .map_err(|error| format!("retrieve after reopen should succeed: {error}"))?;

        ensure(
            retrieved.rationale.telemetry.selectivity.decision_count > 0,
            "selectivity telemetry should be populated after reopen",
        )?;
        let about_trace = retrieved
            .trace
            .as_ref()
            .and_then(|trace| {
                trace.selectivity_decisions.iter().find(|decision| {
                    decision.root.id == entity_id
                        && decision.relation == RelationType::About
                        && decision.object_type == ObjectType::DerivedMemory
                })
            })
            .ok_or_else(|| "retrieve trace should include About->DerivedMemory selectivity for reopened entity root".to_owned())?;
        ensure(
            about_trace.entity_count.is_some_and(|count| count >= 1)
                && about_trace.global_count.is_some_and(|count| count >= 1)
                && !about_trace.fallback,
            "selectivity counters should survive SQLite stats reopen",
        )?;

        Ok::<(), String>(())
    }
    .await;

    test_utils::cleanup_collection(&collection_name).await;
    test_result.expect("v0.1.2 stats persistence test should pass");
}

#[tokio::test]
async fn restart_safe_retrieval_excludes_suppressed_and_superseded_memories() {
    let fixture = StoreFixture::new();
    let collection_name = test_utils::unique_collection_name();
    let entity_id = id("550e8400-e29b-41d4-a716-446655462001");
    let episode_id = id("550e8400-e29b-41d4-a716-446655462002");
    let old_id = id("550e8400-e29b-41d4-a716-446655462003");
    let suppressed_id = id("550e8400-e29b-41d4-a716-446655462004");
    let replacement_id = id("550e8400-e29b-41d4-a716-446655462005");

    let memory = match setup(&collection_name, &fixture, None).await {
        Ok(memory) => memory,
        Err(CustomError::VectorDatabaseError(error))
            if test_utils::is_qdrant_unavailable_error(&error) =>
        {
            println!("skipping v0.1.2 restart-safe retrieval test because Qdrant is unavailable: {error}");
            return;
        }
        Err(error) => panic!("unexpected v0.1.2 restart-safe setup failure: {error}"),
    };

    let test_result = async {
        let remember_outcome = memory
            .remember(
                RememberDraft::new([
                    MemoryObjectDraft::Entity(entity(
                        entity_id,
                        EntityType::Project,
                        "Ledger Meridian",
                    )),
                    MemoryObjectDraft::Episode(episode(
                        episode_id,
                        "Ledger Meridian captured a restart-safe correction fixture.",
                        &[entity_id],
                    )),
                    MemoryObjectDraft::DerivedMemory(derived(
                        old_id,
                        DerivedType::Claim,
                        "Ledger Meridian should keep the stale restart-safe statement.",
                        episode_id,
                        &[entity_id],
                    )),
                    MemoryObjectDraft::DerivedMemory(derived(
                        suppressed_id,
                        DerivedType::ProjectNote,
                        "Ledger Meridian contains a suppressed restart-safe note.",
                        episode_id,
                        &[entity_id],
                    )),
                ])
                .with_links([
                    link(
                        id("550e8400-e29b-41d4-a716-446655462006"),
                        ObjectType::Entity,
                        entity_id,
                        RelationType::About,
                        ObjectType::DerivedMemory,
                        old_id,
                    ),
                    link(
                        id("550e8400-e29b-41d4-a716-446655462007"),
                        ObjectType::Entity,
                        entity_id,
                        RelationType::About,
                        ObjectType::DerivedMemory,
                        suppressed_id,
                    ),
                ]),
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

        drop(memory);

        let reopened = setup(&collection_name, &fixture, None)
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

        Ok::<(), String>(())
    }
    .await;

    test_utils::cleanup_collection(&collection_name).await;
    test_result.expect("v0.1.2 restart-safe retrieval test should pass");
}

#[tokio::test]
async fn selectivity_telemetry_and_fanout_override_bound_entity_root_expansion() {
    let fixture = StoreFixture::new();
    let collection_name = test_utils::unique_collection_name();
    let ids = HighDegreeIds::new();

    let memory = match setup(&collection_name, &fixture, None).await {
        Ok(memory) => memory,
        Err(CustomError::VectorDatabaseError(error))
            if test_utils::is_qdrant_unavailable_error(&error) =>
        {
            println!(
                "skipping v0.1.2 selectivity fanout test because Qdrant is unavailable: {error}"
            );
            return;
        }
        Err(error) => panic!("unexpected v0.1.2 selectivity setup failure: {error}"),
    };

    let test_result = async {
        let remember_outcome = memory
            .remember(high_degree_fixture(&ids))
            .await
            .map_err(|error| format!("high-degree fixture remember should succeed: {error}"))?;
        ensure_no_vector_indexing_failure(
            &remember_outcome,
            "high-degree fixture remember should index vectors",
        )?;

        let default = memory
            .retrieve(entity_root_context("Vector Orchard"))
            .await
            .map_err(|error| format!("default selectivity retrieve should succeed: {error}"))?;
        let default_trace = about_trace(&default, ids.hub_entity)
            .ok_or_else(|| "default retrieve should trace hub About->DerivedMemory selectivity".to_owned())?;
        let default_derived_count = returned_derived_ids(&default)
            .into_iter()
            .filter(|memory_id| ids.hub_derived_ids.contains(memory_id))
            .count();

        ensure(
            default.rationale.telemetry.selectivity.decision_count > 0
                && default_trace.entity_count.is_some_and(|count| count >= 8)
                && default_trace.global_count.is_some_and(|count| count > default_trace.entity_count.unwrap_or_default()),
            "default retrieve should expose non-fallback selectivity telemetry for a high-degree entity",
        )?;
        ensure(
            default_derived_count > 2,
            "default fanout should include more hub derived memories than the small override",
        )?;

        drop(memory);

        let constrained = setup(&collection_name, &fixture, Some((0, 2)))
            .await
            .map_err(|error| format!("constrained facade should reopen with fanout override: {error}"))?;
        let constrained_result = constrained
            .retrieve(entity_root_context("Vector Orchard"))
            .await
            .map_err(|error| format!("constrained selectivity retrieve should succeed: {error}"))?;
        let constrained_trace = about_trace(&constrained_result, ids.hub_entity)
            .ok_or_else(|| "constrained retrieve should trace hub About->DerivedMemory selectivity".to_owned())?;
        let constrained_derived_count = returned_derived_ids(&constrained_result)
            .into_iter()
            .filter(|memory_id| ids.hub_derived_ids.contains(memory_id))
            .count();

        ensure(
            constrained_trace.max_fanout == 2
                && constrained_trace.chosen_fanout <= 2
                && constrained_trace.chosen_fanout < default_trace.chosen_fanout,
            "configured fanout override should reduce traced About->DerivedMemory budget",
        )?;
        ensure(
            constrained_derived_count <= 2 && constrained_derived_count < default_derived_count,
            "configured fanout override should observably constrain returned hub expansion",
        )?;

        Ok::<(), String>(())
    }
    .await;

    test_utils::cleanup_collection(&collection_name).await;
    test_result.expect("v0.1.2 selectivity fanout test should pass");
}

struct StoreFixture {
    _root: TempDir,
    graph_path: std::path::PathBuf,
    stats_path: std::path::PathBuf,
}

impl StoreFixture {
    fn new() -> Self {
        let root = tempfile::tempdir().expect("tempdir should be created");
        Self {
            graph_path: root.path().join("graph"),
            stats_path: root.path().join("stats.sqlite3"),
            _root: root,
        }
    }
}

async fn setup(
    collection_name: &str,
    fixture: &StoreFixture,
    fanout: Option<(usize, usize)>,
) -> Result<character_memory::CharacterMemory, CustomError> {
    test_utils::try_setup_persistent_character_memory(
        collection_name.to_owned(),
        &fixture.graph_path,
        &fixture.stats_path,
        fanout,
    )
    .await
}

fn entity_root_context(query: &str) -> RetrievalContext {
    let mut context = RetrievalContext::new(query).with_trace();
    context.candidate_limits = RetrievalCandidateLimits {
        max_vector_candidates: 32,
        max_graph_roots: 1,
    };
    context.graph_limits = RetrievalGraphLimits {
        max_depth: 1,
        max_nodes: 64,
        max_fanout_per_node: 32,
        max_hub_edges: 64,
        timeout_ms: Some(500),
        allow_degraded_results: true,
        allowed_relation_types: Vec::new(),
    };
    context
}

fn entity(id: MemoryId, entity_type: EntityType, name: &str) -> EntityDraft {
    let mut draft = EntityDraft::new(entity_type, name);
    draft.id = Some(id);
    draft.canonical_key = Some(format!(
        "test:{}",
        name.to_ascii_lowercase().replace(' ', "-")
    ));
    draft.created_at = Some(timestamp());
    draft.updated_at = Some(timestamp());
    draft
}

fn episode(id: MemoryId, summary: &str, participant_entity_ids: &[MemoryId]) -> EpisodeDraft {
    let mut draft = EpisodeDraft::new(summary);
    draft.id = Some(id);
    draft.participant_entity_ids = participant_entity_ids.to_vec();
    draft.started_at = Some(timestamp());
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
    hub_entity: MemoryId,
    other_entities: [MemoryId; 4],
    hub_derived_ids: Vec<MemoryId>,
    other_derived_ids: Vec<MemoryId>,
}

impl HighDegreeIds {
    fn new() -> Self {
        Self {
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

fn high_degree_fixture(ids: &HighDegreeIds) -> RememberDraft {
    let mut objects = vec![MemoryObjectDraft::Entity(entity(
        ids.hub_entity,
        EntityType::Project,
        "Vector Orchard",
    ))];
    for (entity_id, entity_type, name) in [
        (ids.other_entities[0], EntityType::Person, "Mara Quill"),
        (ids.other_entities[1], EntityType::Place, "North Atrium"),
        (ids.other_entities[2], EntityType::Concept, "Signal Weaving"),
        (ids.other_entities[3], EntityType::Tool, "Copper Loom"),
    ] {
        objects.push(MemoryObjectDraft::Entity(entity(
            entity_id,
            entity_type,
            name,
        )));
    }

    let hub_episode_id = id("550e8400-e29b-41d4-a716-446655463010");
    objects.push(MemoryObjectDraft::Episode(episode(
        hub_episode_id,
        "A high-degree project fixture records neutral retrieval expansion pressure.",
        &[ids.hub_entity],
    )));
    for (index, memory_id) in ids.hub_derived_ids.iter().copied().enumerate() {
        objects.push(MemoryObjectDraft::DerivedMemory(derived(
            memory_id,
            DerivedType::ProjectNote,
            &format!("Bounded recall note {index} for orchard calibration pressure."),
            hub_episode_id,
            &[ids.hub_entity],
        )));
    }

    for (index, memory_id) in ids.other_derived_ids.iter().copied().enumerate() {
        let entity_id = ids.other_entities[index % ids.other_entities.len()];
        let episode_id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5546_3300 + index as u128);
        objects.push(MemoryObjectDraft::Episode(episode(
            episode_id,
            &format!("Auxiliary heterogeneous fixture episode {index}."),
            &[entity_id],
        )));
        objects.push(MemoryObjectDraft::DerivedMemory(derived(
            memory_id,
            DerivedType::Claim,
            &format!("Auxiliary neutral memory {index} expands global selectivity counts."),
            episode_id,
            &[entity_id],
        )));
    }

    let mut links = Vec::new();
    for (index, memory_id) in ids.hub_derived_ids.iter().copied().enumerate() {
        links.push(link(
            Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5546_3400 + index as u128),
            ObjectType::Entity,
            ids.hub_entity,
            RelationType::About,
            ObjectType::DerivedMemory,
            memory_id,
        ));
    }
    for (index, memory_id) in ids.other_derived_ids.iter().copied().enumerate() {
        links.push(link(
            Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5546_3500 + index as u128),
            ObjectType::Entity,
            ids.other_entities[index % ids.other_entities.len()],
            RelationType::About,
            ObjectType::DerivedMemory,
            memory_id,
        ));
    }

    RememberDraft::new(objects).with_links(links)
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

fn about_trace(
    outcome: &character_memory::RetrieveOutcome,
    entity_id: MemoryId,
) -> Option<&character_memory::SelectivityTrace> {
    outcome
        .trace
        .as_ref()?
        .selectivity_decisions
        .iter()
        .find(|decision| {
            decision.root.id == entity_id
                && decision.relation == RelationType::About
                && decision.object_type == ObjectType::DerivedMemory
        })
}

fn timestamp() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-06-12T10:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
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

fn ensure_no_vector_indexing_failure(
    outcome: &character_memory::RememberOutcome,
    context: &'static str,
) -> Result<(), String> {
    if let Some(failure) = &outcome.vector_indexing_failure {
        return Err(format!(
            "{context}: vector indexing failed for object ids {:?}; persisted object ids {:?}; indexed object ids {:?}; error: {}",
            failure.unindexed_object_ids,
            outcome.persisted_object_ids,
            outcome.vector_indexed_object_ids,
            failure.error_message
        ));
    }

    Ok(())
}
