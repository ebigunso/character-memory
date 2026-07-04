// Reconciliation is an admin/governance seam that is intentionally dormant in the
// public facade; remove this module-level allow once a governance surface calls it.
#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use crate::api::types::{
    graph_uri, MemoryId, MemoryObject, ObjectType, RelationType, RetentionState,
    CURRENT_SCHEMA_VERSION,
};
use crate::errors::CustomError;
use crate::models::vector::VectorCandidateDiagnosticRecord;

use crate::ports::graph_authority::{GraphAuthorityStore, GraphObjectRef};
use crate::ports::vector_candidate::VectorCandidateStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReconciliationReport {
    pub(crate) diagnostics: Vec<ReconciliationDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReconciliationDiagnostic {
    pub(crate) object_ref: GraphObjectRef,
    pub(crate) kind: ReconciliationDriftKind,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReconciliationDriftKind {
    VectorOnlyCandidate,
    GraphOnlyObject,
    GraphUriMismatch,
    StaleLifecycleHint,
    StaleCurrentnessHint,
    UnsupportedVectorSchema,
    MissingProvenance,
}

pub(crate) async fn reconcile_graph_vector_stores<G, V>(
    graph_store: &G,
    vector_store: &V,
) -> Result<ReconciliationReport, CustomError>
where
    G: GraphAuthorityStore + ?Sized,
    V: VectorCandidateStore + ?Sized,
{
    let graph_objects = graph_store.list_diagnostic_objects().await?;
    let graph_links = graph_store.list_diagnostic_links().await?;
    let vector_records = vector_store.list_candidate_diagnostics().await?;

    Ok(reconcile_records(
        graph_objects,
        graph_links,
        vector_records,
    ))
}

fn reconcile_records(
    graph_objects: Vec<MemoryObject>,
    graph_links: Vec<crate::api::types::MemoryLink>,
    vector_records: Vec<VectorCandidateDiagnosticRecord>,
) -> ReconciliationReport {
    let graph_by_ref = graph_objects
        .iter()
        .map(|object| (graph_object_ref(object), object))
        .collect::<HashMap<_, _>>();
    let vector_refs = vector_records
        .iter()
        .map(|record| GraphObjectRef::new(record.object_id, record.object_type))
        .collect::<HashSet<_>>();
    let superseded = graph_links
        .iter()
        .filter(|link| {
            link.relation == RelationType::Supersedes
                && link.from_type == ObjectType::DerivedMemory
                && link.to_type == ObjectType::DerivedMemory
        })
        .map(|link| link.to_id)
        .collect::<HashSet<_>>();
    let mut diagnostics = Vec::new();

    for record in &vector_records {
        let object_ref = GraphObjectRef::new(record.object_id, record.object_type);
        let expected_graph_uri = graph_uri(record.object_type, record.object_id);
        if record.graph_uri != expected_graph_uri {
            push_diagnostic(
                &mut diagnostics,
                object_ref,
                ReconciliationDriftKind::GraphUriMismatch,
                format!(
                    "vector graph_uri '{}' does not match canonical '{}'",
                    record.graph_uri, expected_graph_uri
                ),
            );
        }

        if record.schema_version != CURRENT_SCHEMA_VERSION {
            push_diagnostic(
                &mut diagnostics,
                object_ref,
                ReconciliationDriftKind::UnsupportedVectorSchema,
                format!(
                    "vector schema_version '{}' is not supported",
                    record.schema_version
                ),
            );
        }

        let Some(graph_object) = graph_by_ref.get(&object_ref) else {
            push_diagnostic(
                &mut diagnostics,
                object_ref,
                ReconciliationDriftKind::VectorOnlyCandidate,
                "vector candidate has no matching graph object",
            );
            continue;
        };

        push_lifecycle_diagnostics(
            &mut diagnostics,
            object_ref,
            record,
            graph_object,
            &superseded,
        );
    }

    for object in &graph_objects {
        let object_ref = graph_object_ref(object);
        if object_ref.object_type != ObjectType::MemoryLink && !vector_refs.contains(&object_ref) {
            push_diagnostic(
                &mut diagnostics,
                object_ref,
                ReconciliationDriftKind::GraphOnlyObject,
                "graph object has no matching vector candidate",
            );
        }
        if missing_required_provenance(object, &graph_links) {
            push_diagnostic(
                &mut diagnostics,
                object_ref,
                ReconciliationDriftKind::MissingProvenance,
                "derived memory has no source episode or observation provenance",
            );
        }
    }

    diagnostics.sort_by_key(|diagnostic| {
        (
            diagnostic.object_ref.object_id,
            object_type_rank(diagnostic.object_ref.object_type),
            drift_kind_rank(diagnostic.kind),
        )
    });
    ReconciliationReport { diagnostics }
}

fn push_lifecycle_diagnostics(
    diagnostics: &mut Vec<ReconciliationDiagnostic>,
    object_ref: GraphObjectRef,
    record: &VectorCandidateDiagnosticRecord,
    graph_object: &MemoryObject,
    superseded: &HashSet<MemoryId>,
) {
    if let Some(graph_retention) = graph_retention_state(graph_object) {
        if record
            .retention_state
            .is_some_and(|vector_retention| vector_retention != graph_retention)
        {
            push_diagnostic(
                diagnostics,
                object_ref,
                ReconciliationDriftKind::StaleLifecycleHint,
                format!(
                    "vector retention hint does not match graph retention state {:?}",
                    graph_retention
                ),
            );
        }
    }

    if let Some(graph_is_current) = graph_is_current(graph_object) {
        if record
            .is_current
            .is_some_and(|vector_is_current| vector_is_current != graph_is_current)
        {
            push_diagnostic(
                diagnostics,
                object_ref,
                ReconciliationDriftKind::StaleCurrentnessHint,
                format!(
                    "vector currentness hint does not match graph is_current={graph_is_current}"
                ),
            );
        }

        let graph_is_superseded = superseded.contains(&object_ref.object_id);
        if record
            .is_superseded
            .is_some_and(|vector_is_superseded| vector_is_superseded != graph_is_superseded)
        {
            push_diagnostic(
                diagnostics,
                object_ref,
                ReconciliationDriftKind::StaleCurrentnessHint,
                format!(
                    "vector supersession hint does not match graph superseded={graph_is_superseded}"
                ),
            );
        }
    }
}

fn missing_required_provenance(
    object: &MemoryObject,
    links: &[crate::api::types::MemoryLink],
) -> bool {
    let MemoryObject::DerivedMemory(memory) = object else {
        return false;
    };
    if !memory.derived_from_episode_ids.is_empty()
        || !memory.derived_from_observation_ids.is_empty()
    {
        return false;
    }
    !links.iter().any(|link| {
        link.relation == RelationType::DerivedFrom
            && ((link.from_id == memory.id
                && link.from_type == ObjectType::DerivedMemory
                && matches!(link.to_type, ObjectType::Episode | ObjectType::Observation))
                || (link.to_id == memory.id
                    && link.to_type == ObjectType::DerivedMemory
                    && matches!(
                        link.from_type,
                        ObjectType::Episode | ObjectType::Observation
                    )))
    })
}

fn push_diagnostic(
    diagnostics: &mut Vec<ReconciliationDiagnostic>,
    object_ref: GraphObjectRef,
    kind: ReconciliationDriftKind,
    detail: impl Into<String>,
) {
    diagnostics.push(ReconciliationDiagnostic {
        object_ref,
        kind,
        detail: detail.into(),
    });
}

fn graph_object_ref(object: &MemoryObject) -> GraphObjectRef {
    let (object_id, object_type) = match object {
        MemoryObject::Episode(object) => (object.id, object.object_type),
        MemoryObject::Observation(object) => (object.id, object.object_type),
        MemoryObject::Entity(object) => (object.id, object.object_type),
        MemoryObject::MemoryThread(object) => (object.id, object.object_type),
        MemoryObject::DerivedMemory(object) => (object.id, object.object_type),
        MemoryObject::MemoryLink(object) => (object.id, object.object_type),
    };
    GraphObjectRef::new(object_id, object_type)
}

fn graph_retention_state(object: &MemoryObject) -> Option<RetentionState> {
    match object {
        MemoryObject::Episode(object) => Some(object.retention_state),
        MemoryObject::Observation(object) => Some(object.retention_state),
        MemoryObject::DerivedMemory(object) => Some(object.retention_state),
        MemoryObject::Entity(_) | MemoryObject::MemoryThread(_) | MemoryObject::MemoryLink(_) => {
            None
        }
    }
}

fn graph_is_current(object: &MemoryObject) -> Option<bool> {
    match object {
        MemoryObject::DerivedMemory(object) => Some(object.is_current),
        _ => None,
    }
}

fn object_type_rank(object_type: ObjectType) -> u8 {
    match object_type {
        ObjectType::Episode => 0,
        ObjectType::Observation => 1,
        ObjectType::Entity => 2,
        ObjectType::MemoryThread => 3,
        ObjectType::DerivedMemory => 4,
        ObjectType::MemoryLink => 5,
    }
}

fn drift_kind_rank(kind: ReconciliationDriftKind) -> u8 {
    match kind {
        ReconciliationDriftKind::VectorOnlyCandidate => 0,
        ReconciliationDriftKind::GraphOnlyObject => 1,
        ReconciliationDriftKind::GraphUriMismatch => 2,
        ReconciliationDriftKind::StaleLifecycleHint => 3,
        ReconciliationDriftKind::StaleCurrentnessHint => 4,
        ReconciliationDriftKind::UnsupportedVectorSchema => 5,
        ReconciliationDriftKind::MissingProvenance => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::{
        DerivedMemory, DerivedType, MemoryObject, MemoryThread, Stability, ThreadStatus,
        DEFAULT_SCHEMA_VERSION,
    };
    use crate::models::vector::{VectorCandidateDiagnosticRecord, VectorSurface};
    use crate::ports::graph_authority::GraphAuthorityStore;
    use crate::test_support::{
        representative_fixtures, FakeGraphAuthorityStore, FakeVectorCandidateStore,
    };
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    #[tokio::test]
    async fn reconciliation_reports_cross_store_drift_classes() {
        let fixtures = representative_fixtures();
        let graph = FakeGraphAuthorityStore::new();
        let mut suppressed_observation = fixtures.salient_observation.clone();
        suppressed_observation.retention_state = RetentionState::Suppressed;
        let mut non_current_memory = fixtures.user_preference.clone();
        non_current_memory.is_current = false;
        let mut superseded_memory = fixtures.derived_reflection.clone();
        superseded_memory.id = id(501);
        let mut replacement_memory = fixtures.correction.clone();
        replacement_memory.id = id(502);
        let mut missing_provenance = invalid_derived_memory(id(503));
        missing_provenance.text = "Corrupt derived memory without provenance.".to_owned();
        let graph_only_thread = graph_only_thread(id(504));
        let supersedes_link = crate::api::types::MemoryLink {
            id: id(601),
            object_type: ObjectType::MemoryLink,
            from_id: replacement_memory.id,
            from_type: ObjectType::DerivedMemory,
            to_id: superseded_memory.id,
            to_type: ObjectType::DerivedMemory,
            relation: RelationType::Supersedes,
            confidence: 0.9,
            rationale: Some("replacement".to_owned()),
            created_at: timestamp("2026-04-28T12:00:00Z"),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        };
        graph
            .upsert_objects(&[
                MemoryObject::Episode(fixtures.episode.clone()),
                MemoryObject::Observation(suppressed_observation.clone()),
                MemoryObject::DerivedMemory(non_current_memory.clone()),
                MemoryObject::DerivedMemory(superseded_memory.clone()),
                MemoryObject::DerivedMemory(replacement_memory.clone()),
                MemoryObject::DerivedMemory(missing_provenance.clone()),
                MemoryObject::MemoryThread(graph_only_thread),
            ])
            .await
            .unwrap();
        graph.upsert_links(&[supersedes_link]).await.unwrap();

        let vector = FakeVectorCandidateStore::new();
        vector
            .upsert_diagnostic_records(&[
                diagnostic(
                    fixtures.episode.id,
                    ObjectType::Episode,
                    "urn:cmem:episode:wrong",
                    DEFAULT_SCHEMA_VERSION,
                    Some(RetentionState::Active),
                    None,
                    None,
                ),
                diagnostic(
                    suppressed_observation.id,
                    ObjectType::Observation,
                    graph_uri(ObjectType::Observation, suppressed_observation.id),
                    DEFAULT_SCHEMA_VERSION,
                    Some(RetentionState::Active),
                    None,
                    None,
                ),
                diagnostic(
                    non_current_memory.id,
                    ObjectType::DerivedMemory,
                    graph_uri(ObjectType::DerivedMemory, non_current_memory.id),
                    DEFAULT_SCHEMA_VERSION,
                    Some(RetentionState::Active),
                    Some(true),
                    Some(false),
                ),
                diagnostic(
                    superseded_memory.id,
                    ObjectType::DerivedMemory,
                    graph_uri(ObjectType::DerivedMemory, superseded_memory.id),
                    DEFAULT_SCHEMA_VERSION,
                    Some(RetentionState::Active),
                    Some(true),
                    Some(false),
                ),
                diagnostic(
                    id(999),
                    ObjectType::Entity,
                    graph_uri(ObjectType::Entity, id(999)),
                    "<missing schema_version>",
                    None,
                    None,
                    None,
                ),
                diagnostic(
                    replacement_memory.id,
                    ObjectType::DerivedMemory,
                    graph_uri(ObjectType::DerivedMemory, replacement_memory.id),
                    "future_schema",
                    Some(RetentionState::Active),
                    Some(true),
                    Some(false),
                ),
            ])
            .await
            .unwrap();

        let report = reconcile_graph_vector_stores(&graph, &vector)
            .await
            .unwrap();
        let kinds = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind)
            .collect::<Vec<_>>();

        for expected in [
            ReconciliationDriftKind::VectorOnlyCandidate,
            ReconciliationDriftKind::GraphOnlyObject,
            ReconciliationDriftKind::GraphUriMismatch,
            ReconciliationDriftKind::StaleLifecycleHint,
            ReconciliationDriftKind::StaleCurrentnessHint,
            ReconciliationDriftKind::UnsupportedVectorSchema,
            ReconciliationDriftKind::MissingProvenance,
        ] {
            assert!(kinds.contains(&expected), "missing {expected:?}");
        }
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == ReconciliationDriftKind::MissingProvenance
                && diagnostic.object_ref.object_id == missing_provenance.id
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == ReconciliationDriftKind::UnsupportedVectorSchema
                && diagnostic.object_ref.object_id == id(999)
        }));
    }

    fn diagnostic(
        object_id: MemoryId,
        object_type: ObjectType,
        graph_uri: impl Into<String>,
        schema_version: impl Into<String>,
        retention_state: Option<RetentionState>,
        is_current: Option<bool>,
        is_superseded: Option<bool>,
    ) -> VectorCandidateDiagnosticRecord {
        VectorCandidateDiagnosticRecord {
            object_id,
            object_type,
            graph_uri: graph_uri.into(),
            surface: VectorSurface::Summary,
            schema_version: schema_version.into(),
            retention_state,
            is_current,
            is_superseded,
        }
    }

    fn invalid_derived_memory(id: MemoryId) -> DerivedMemory {
        DerivedMemory {
            id,
            object_type: ObjectType::DerivedMemory,
            derived_type: DerivedType::Reflection,
            text: "Invalid memory".to_owned(),
            derived_from_episode_ids: Vec::new(),
            derived_from_observation_ids: Vec::new(),
            thread_ids: Vec::new(),
            entity_ids: Vec::new(),
            confidence: 0.8,
            salience_score: 0.7,
            stability: Stability::Medium,
            is_current: true,
            supersedes: Vec::new(),
            retention_state: RetentionState::Active,
            created_at: timestamp("2026-04-28T12:00:00Z"),
            updated_at: timestamp("2026-04-28T12:00:00Z"),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        }
    }

    fn graph_only_thread(id: MemoryId) -> MemoryThread {
        MemoryThread {
            id,
            object_type: ObjectType::MemoryThread,
            title: "Graph only thread".to_owned(),
            summary: "Thread missing vector candidate.".to_owned(),
            status: ThreadStatus::Active,
            last_touched_at: timestamp("2026-04-28T12:00:00Z"),
            salience_score: 0.5,
            canonical_key: Some("thread:graph-only".to_owned()),
            created_at: timestamp("2026-04-28T12:00:00Z"),
            updated_at: timestamp("2026-04-28T12:00:00Z"),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        }
    }

    fn id(value: u128) -> MemoryId {
        Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_7000_0000 + value)
    }

    fn timestamp(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }
}
