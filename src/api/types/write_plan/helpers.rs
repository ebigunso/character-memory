//! Deterministic construction helpers for remember write plans.
//!
//! These helpers only carry caller-provided structure and content. They do not parse raw natural
//! language to infer preferences, commitments, corrections, character signals, thread membership,
//! entity identity, or scope membership. Raw references remain opaque strings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{
    CandidateProducerKind, CandidateProvenance, DerivedMemoryCandidate, EntityCandidate,
    EpisodeCandidate, MemoryCandidate, MemoryLinkCandidate, MemoryThreadCandidate,
    ObservationCandidate, RememberDiagnostics, RememberInput, RememberWritePlan, SourceSpan,
    StatsUpdateCandidate, VectorIndexCandidate,
};
use crate::api::types::domain::{
    graph_uri, MemoryId, ObjectType, RelationType, RetentionState, DEFAULT_SCHEMA_VERSION,
};
use crate::api::types::draft::{
    DerivedMemoryDraft, EntityDraft, EpisodeDraft, MemoryLinkDraft, MemoryThreadDraft,
    ObservationDraft,
};
use crate::api::types::lifecycle::ExternalSourceReference;
use crate::api::types::retrieval::MemoryObjectRef;

/// Stable UUIDv5 namespace for write-plan IDs. IDs remain stable across releases as long as this
/// namespace and `deterministic_uuid` label framing stay fixed.
const WRITE_PLAN_NAMESPACE: uuid::Uuid = uuid::uuid!("5f18dc72-f839-58f8-8ff3-c841298cc789");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RememberPlanDefaults {
    pub operation_seed: String,
    pub created_at: DateTime<Utc>,
    pub schema_version: String,
}

impl RememberPlanDefaults {
    pub fn generated() -> Self {
        Self {
            operation_seed: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        }
    }

    pub fn fixed(operation_seed: impl Into<String>, created_at: DateTime<Utc>) -> Self {
        Self {
            operation_seed: operation_seed.into(),
            created_at,
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        }
    }

    pub fn with_schema_version(mut self, schema_version: impl Into<String>) -> Self {
        self.schema_version = schema_version.into();
        self
    }

    pub fn stable_id(&self, label: impl AsRef<str>) -> MemoryId {
        deterministic_uuid(&[
            "character_memory.remember_plan".as_bytes(),
            self.operation_seed.as_bytes(),
            label.as_ref().as_bytes(),
        ])
    }

    pub fn graph_iri(&self, object_type: ObjectType, id: MemoryId) -> String {
        graph_uri(object_type, id)
    }
}

impl Default for RememberPlanDefaults {
    fn default() -> Self {
        Self::generated()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedCandidateRefs {
    pub operation_id: MemoryId,
    pub episode_id: MemoryId,
    pub observation_id: MemoryId,
    pub candidate_refs: Vec<MemoryObjectRef>,
}

impl RememberInput {
    pub fn prepare_write_plan(&self, defaults: &RememberPlanDefaults) -> RememberWritePlan {
        self.prepare_write_plan_with_options(defaults, true, true)
    }

    pub fn prepare_write_plan_with_options(
        &self,
        defaults: &RememberPlanDefaults,
        include_vector_index_candidates: bool,
        include_stats_update_candidates: bool,
    ) -> RememberWritePlan {
        let refs = self.prepared_candidate_refs(defaults);
        let idempotency_key = self.idempotency_key(defaults);
        let mut plan = RememberWritePlan::new(refs.operation_id, idempotency_key);

        if let Some(source_input_ref) = self.source_reference() {
            plan = plan.with_source_input_ref(source_input_ref);
        }

        let episode_provenance = self.helper_provenance();
        let episode =
            EpisodeCandidate::new(self.episode_candidate_draft(defaults), episode_provenance);
        plan = plan.with_candidate(MemoryCandidate::Episode(episode));

        let observation_provenance = self
            .helper_provenance()
            .with_source_episode(refs.episode_id);
        let observation = ObservationCandidate::new(
            self.observation_candidate_draft(defaults, refs.episode_id),
            observation_provenance,
        );
        plan = plan.with_candidate(MemoryCandidate::Observation(observation));

        for draft in self.entity_drafts.iter().cloned().enumerate() {
            plan = plan.with_candidate(MemoryCandidate::Entity(EntityCandidate::new(
                complete_entity_draft(draft.1, defaults, refs.candidate_refs[2 + draft.0].id),
                self.caller_provenance(),
            )));
        }

        let thread_offset = 2 + self.entity_drafts.len();
        for draft in self.memory_thread_drafts.iter().cloned().enumerate() {
            plan = plan.with_candidate(MemoryCandidate::MemoryThread(MemoryThreadCandidate::new(
                complete_thread_draft(
                    draft.1,
                    defaults,
                    refs.candidate_refs[thread_offset + draft.0].id,
                ),
                self.caller_provenance(),
            )));
        }

        let derived_offset = thread_offset + self.memory_thread_drafts.len();
        for draft in self.derived_memory_drafts.iter().cloned().enumerate() {
            let draft = complete_derived_draft(
                draft.1,
                defaults,
                refs.candidate_refs[derived_offset + draft.0].id,
                refs.episode_id,
                refs.observation_id,
            );
            let provenance = self
                .caller_provenance()
                .with_source_episode(refs.episode_id)
                .with_source_observation(refs.observation_id);
            plan = plan.with_candidate(MemoryCandidate::DerivedMemory(
                DerivedMemoryCandidate::new(draft, provenance),
            ));
        }

        let link_offset = derived_offset + self.derived_memory_drafts.len();
        for draft in self.memory_link_drafts.iter().cloned().enumerate() {
            plan = plan.with_candidate(MemoryCandidate::MemoryLink(MemoryLinkCandidate::new(
                complete_link_draft(
                    draft.1,
                    defaults,
                    refs.candidate_refs[link_offset + draft.0].id,
                ),
                self.caller_provenance(),
            )));
        }

        for link in self.caller_hint_links(defaults, refs.episode_id, refs.observation_id) {
            plan = plan.with_candidate(MemoryCandidate::MemoryLink(MemoryLinkCandidate::new(
                link,
                self.helper_provenance()
                    .with_source_episode(refs.episode_id)
                    .with_source_observation(refs.observation_id),
            )));
        }

        if include_vector_index_candidates {
            for object_ref in refs.candidate_refs.iter().copied() {
                plan =
                    plan.with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                        object_ref,
                        self.embedding_text(),
                        self.helper_provenance(),
                    )));
            }
        }

        if include_stats_update_candidates {
            for object_ref in refs.candidate_refs.iter().copied() {
                plan = plan.with_candidate(MemoryCandidate::StatsUpdate(
                    StatsUpdateCandidate::new(object_ref, self.helper_provenance()),
                ));
            }
        }

        plan.with_diagnostics(RememberDiagnostics::default())
    }

    pub fn prepared_candidate_refs(
        &self,
        defaults: &RememberPlanDefaults,
    ) -> PreparedCandidateRefs {
        let operation_id = defaults.stable_id("operation");
        let episode_id = self
            .episode_drafts
            .first()
            .and_then(|draft| draft.id)
            .unwrap_or_else(|| defaults.stable_id("episode:0"));
        let observation_id = self
            .observation_drafts
            .first()
            .and_then(|draft| draft.id)
            .unwrap_or_else(|| defaults.stable_id("observation:0"));

        let mut candidate_refs = vec![
            MemoryObjectRef::new(ObjectType::Episode, episode_id),
            MemoryObjectRef::new(ObjectType::Observation, observation_id),
        ];

        for (index, draft) in self.entity_drafts.iter().enumerate() {
            candidate_refs.push(MemoryObjectRef::new(
                ObjectType::Entity,
                draft
                    .id
                    .unwrap_or_else(|| defaults.stable_id(format!("entity:{index}"))),
            ));
        }
        for (index, draft) in self.memory_thread_drafts.iter().enumerate() {
            candidate_refs.push(MemoryObjectRef::new(
                ObjectType::MemoryThread,
                draft
                    .id
                    .unwrap_or_else(|| defaults.stable_id(format!("thread:{index}"))),
            ));
        }
        for (index, draft) in self.derived_memory_drafts.iter().enumerate() {
            candidate_refs.push(MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                draft
                    .id
                    .unwrap_or_else(|| defaults.stable_id(format!("derived:{index}"))),
            ));
        }
        for (index, draft) in self.memory_link_drafts.iter().enumerate() {
            candidate_refs.push(MemoryObjectRef::new(
                ObjectType::MemoryLink,
                draft
                    .id
                    .unwrap_or_else(|| defaults.stable_id(format!("caller-link:{index}"))),
            ));
        }

        PreparedCandidateRefs {
            operation_id,
            episode_id,
            observation_id,
            candidate_refs,
        }
    }

    pub fn idempotency_key(&self, defaults: &RememberPlanDefaults) -> String {
        let encoded =
            serde_json::to_string(&(self, defaults)).expect("remember input is serializable");
        let id = deterministic_uuid(&[
            "character_memory.idempotency".as_bytes(),
            encoded.as_bytes(),
        ]);
        format!("remember:{id}")
    }

    pub fn source_reference(&self) -> Option<ExternalSourceReference> {
        self.raw_refs
            .first()
            .cloned()
            .map(ExternalSourceReference::raw)
            .or_else(|| {
                self.source_spans.iter().find_map(|span| {
                    span.source_ref
                        .clone()
                        .map(ExternalSourceReference::source)
                        .or_else(|| span.raw_ref.clone().map(ExternalSourceReference::raw))
                })
            })
    }

    pub fn raw_source_span(raw_ref: impl Into<String>) -> SourceSpan {
        SourceSpan::raw(raw_ref)
    }

    pub fn source_span(source_ref: impl Into<String>) -> SourceSpan {
        SourceSpan::source(source_ref)
    }

    pub fn embedding_text(&self) -> String {
        self.content.clone()
    }

    fn episode_candidate_draft(&self, defaults: &RememberPlanDefaults) -> EpisodeDraft {
        let mut draft = self
            .episode_drafts
            .first()
            .cloned()
            .unwrap_or_else(|| EpisodeDraft::new(self.content.clone()));
        draft
            .id
            .get_or_insert_with(|| defaults.stable_id("episode:0"));
        draft.started_at = draft.started_at.or(self.started_at);
        draft.ended_at = draft.ended_at.or(self.ended_at);
        if draft.participant_entity_ids.is_empty() {
            draft.participant_entity_ids = self.participant_entity_ids.clone();
        }
        if draft.raw_ref.is_none() {
            draft.raw_ref = self.raw_refs.first().cloned();
        }
        draft.created_at.get_or_insert(defaults.created_at);
        draft
            .schema_version
            .get_or_insert_with(|| defaults.schema_version.clone());
        draft.retention_state = retention_default(draft.retention_state);
        draft
    }

    fn observation_candidate_draft(
        &self,
        defaults: &RememberPlanDefaults,
        episode_id: MemoryId,
    ) -> ObservationDraft {
        let mut draft = self
            .observation_drafts
            .first()
            .cloned()
            .unwrap_or_else(|| ObservationDraft::new(episode_id, self.content.clone()));
        draft
            .id
            .get_or_insert_with(|| defaults.stable_id("observation:0"));
        draft.episode_id = if draft.episode_id.is_nil() {
            episode_id
        } else {
            draft.episode_id
        };
        draft.observed_at = draft.observed_at.or(self.started_at);
        if draft.raw_ref.is_none() {
            draft.raw_ref = self.raw_refs.first().cloned();
        }
        draft.created_at.get_or_insert(defaults.created_at);
        draft
            .schema_version
            .get_or_insert_with(|| defaults.schema_version.clone());
        draft.retention_state = retention_default(draft.retention_state);
        draft
    }

    fn caller_hint_links(
        &self,
        defaults: &RememberPlanDefaults,
        episode_id: MemoryId,
        observation_id: MemoryId,
    ) -> Vec<MemoryLinkDraft> {
        let mut links = Vec::new();
        for (index, entity_id) in self.entity_ids.iter().copied().enumerate() {
            links.push(complete_link_draft(
                MemoryLinkDraft::new(
                    ObjectType::Episode,
                    episode_id,
                    RelationType::Involves,
                    ObjectType::Entity,
                    entity_id,
                ),
                defaults,
                defaults.stable_id(format!("hint-link:entity:{index}")),
            ));
        }
        for (index, participant_id) in self.participant_entity_ids.iter().copied().enumerate() {
            links.push(complete_link_draft(
                MemoryLinkDraft::new(
                    ObjectType::Observation,
                    observation_id,
                    RelationType::Mentions,
                    ObjectType::Entity,
                    participant_id,
                ),
                defaults,
                defaults.stable_id(format!("hint-link:participant:{index}")),
            ));
        }
        for (index, thread_id) in self.thread_ids.iter().copied().enumerate() {
            links.push(complete_link_draft(
                MemoryLinkDraft::new(
                    ObjectType::Observation,
                    observation_id,
                    RelationType::PartOfThread,
                    ObjectType::MemoryThread,
                    thread_id,
                ),
                defaults,
                defaults.stable_id(format!("hint-link:thread:{index}")),
            ));
        }
        links
    }

    fn helper_provenance(&self) -> CandidateProvenance {
        self.provenance(CandidateProducerKind::DeterministicHelper)
    }

    fn caller_provenance(&self) -> CandidateProvenance {
        self.provenance(CandidateProducerKind::Caller)
    }

    fn provenance(&self, producer_kind: CandidateProducerKind) -> CandidateProvenance {
        let mut provenance = CandidateProvenance::unavailable(producer_kind);
        for span in self.source_spans.iter().cloned() {
            provenance = provenance.with_source_span(span);
        }
        for raw_ref in &self.raw_refs {
            provenance =
                provenance.with_external_ref(ExternalSourceReference::raw(raw_ref.clone()));
        }
        provenance
    }
}

fn complete_entity_draft(
    mut draft: EntityDraft,
    defaults: &RememberPlanDefaults,
    id: MemoryId,
) -> EntityDraft {
    draft.id.get_or_insert(id);
    draft.created_at.get_or_insert(defaults.created_at);
    draft.updated_at.get_or_insert(defaults.created_at);
    draft
        .schema_version
        .get_or_insert_with(|| defaults.schema_version.clone());
    draft
}

fn complete_thread_draft(
    mut draft: MemoryThreadDraft,
    defaults: &RememberPlanDefaults,
    id: MemoryId,
) -> MemoryThreadDraft {
    draft.id.get_or_insert(id);
    draft.last_touched_at.get_or_insert(defaults.created_at);
    draft.created_at.get_or_insert(defaults.created_at);
    draft.updated_at.get_or_insert(defaults.created_at);
    draft
        .schema_version
        .get_or_insert_with(|| defaults.schema_version.clone());
    draft
}

fn complete_derived_draft(
    mut draft: DerivedMemoryDraft,
    defaults: &RememberPlanDefaults,
    id: MemoryId,
    episode_id: MemoryId,
    observation_id: MemoryId,
) -> DerivedMemoryDraft {
    draft.id.get_or_insert(id);
    if draft.derived_from_episode_ids.is_empty() && draft.derived_from_observation_ids.is_empty() {
        draft.derived_from_episode_ids.push(episode_id);
        draft.derived_from_observation_ids.push(observation_id);
    }
    draft.created_at.get_or_insert(defaults.created_at);
    draft.updated_at.get_or_insert(defaults.created_at);
    draft
        .schema_version
        .get_or_insert_with(|| defaults.schema_version.clone());
    draft.retention_state = retention_default(draft.retention_state);
    draft
}

fn complete_link_draft(
    mut draft: MemoryLinkDraft,
    defaults: &RememberPlanDefaults,
    id: MemoryId,
) -> MemoryLinkDraft {
    draft.id.get_or_insert(id);
    draft.created_at.get_or_insert(defaults.created_at);
    draft
        .schema_version
        .get_or_insert_with(|| defaults.schema_version.clone());
    draft
}

fn retention_default(retention_state: RetentionState) -> RetentionState {
    retention_state
}

fn deterministic_uuid(parts: &[&[u8]]) -> MemoryId {
    let mut label = Vec::new();
    for part in parts {
        label.extend_from_slice(&(part.len() as u64).to_be_bytes());
        label.extend_from_slice(part);
    }

    uuid::Uuid::new_v5(&WRITE_PLAN_NAMESPACE, &label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::domain::{DerivedType, EntityType};

    fn timestamp(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn memory_id(value: &str) -> MemoryId {
        uuid::Uuid::parse_str(value).unwrap()
    }

    #[test]
    fn same_input_and_fixed_defaults_prepare_identical_plan() {
        let defaults =
            RememberPlanDefaults::fixed("fixed-operation", timestamp("2026-07-03T10:00:00Z"));
        let input = RememberInput::new("Caller said they prefer terse planning notes.")
            .with_raw_ref("raw://conversation/7#turn=2")
            .with_source_span(SourceSpan::raw("raw://conversation/7#turn=2").with_turn_range(2, 2))
            .with_entity_id(memory_id("550e8400-e29b-41d4-a716-446655443001"))
            .with_thread_id(memory_id("550e8400-e29b-41d4-a716-446655443002"))
            .with_participant_entity_id(memory_id("550e8400-e29b-41d4-a716-446655443003"))
            .with_derived_memory(DerivedMemoryDraft::new(
                DerivedType::Reflection,
                "Caller-provided reflection text.",
            ))
            .with_entity(EntityDraft::new(EntityType::Person, "Caller Named Entity"));

        let first = input.prepare_write_plan(&defaults);
        let second = input.prepare_write_plan(&defaults);

        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
    }

    #[test]
    fn helper_carries_content_verbatim_without_inferred_links() {
        let defaults =
            RememberPlanDefaults::fixed("no-inference", timestamp("2026-07-03T10:05:00Z"));
        let input = RememberInput::new(
            "Alice promised Bob a fix, but no structured entity or thread hints were supplied.",
        );

        let plan = input.prepare_write_plan_with_options(&defaults, false, false);

        assert_eq!(plan.candidates.len(), 2);
        assert!(plan.candidates.iter().all(|candidate| !matches!(
            candidate,
            MemoryCandidate::Entity(_)
                | MemoryCandidate::DerivedMemory(_)
                | MemoryCandidate::MemoryLink(_)
        )));
        match &plan.candidates[0] {
            MemoryCandidate::Episode(candidate) => assert_eq!(
                candidate.draft.summary,
                "Alice promised Bob a fix, but no structured entity or thread hints were supplied."
            ),
            other => panic!("expected episode candidate, got {other:?}"),
        }
        match &plan.candidates[1] {
            MemoryCandidate::Observation(candidate) => assert_eq!(
                candidate.draft.text,
                "Alice promised Bob a fix, but no structured entity or thread hints were supplied."
            ),
            other => panic!("expected observation candidate, got {other:?}"),
        }
    }

    #[test]
    fn source_refs_and_embedding_text_remain_opaque_and_verbatim() {
        let input = RememberInput::new("  keep caller spacing exactly  ")
            .with_raw_ref("opaque://system/raw/123")
            .with_source_span(SourceSpan::source("conversation-123"));

        assert_eq!(input.embedding_text(), "  keep caller spacing exactly  ");
        assert_eq!(
            input.source_reference(),
            Some(ExternalSourceReference::raw("opaque://system/raw/123"))
        );
        assert_eq!(
            RememberInput::raw_source_span("opaque://system/raw/123")
                .raw_ref
                .as_deref(),
            Some("opaque://system/raw/123")
        );
    }

    #[test]
    fn graph_iri_reuses_domain_graph_uri() {
        let defaults = RememberPlanDefaults::fixed("graph-iri", timestamp("2026-07-03T10:10:00Z"));
        let id = defaults.stable_id("episode:0");

        assert_eq!(
            defaults.graph_iri(ObjectType::Episode, id),
            graph_uri(ObjectType::Episode, id)
        );
    }

    #[test]
    fn candidate_ids_and_idempotency_key_change_with_content() {
        let defaults =
            RememberPlanDefaults::fixed("same-defaults", timestamp("2026-07-03T10:15:00Z"));
        let first = RememberInput::new("first content");
        let second = RememberInput::new("second content");

        assert_ne!(
            first.idempotency_key(&defaults),
            second.idempotency_key(&defaults)
        );
        assert_eq!(
            first.prepared_candidate_refs(&defaults).episode_id,
            second.prepared_candidate_refs(&defaults).episode_id
        );
    }

    #[test]
    fn diagnostics_candidate_counts_can_be_computed_by_callers_deterministically() {
        let defaults = RememberPlanDefaults::fixed("counts", timestamp("2026-07-03T10:20:00Z"));
        let plan = RememberInput::new("count candidates").prepare_write_plan(&defaults);
        let episode_count = plan
            .candidates
            .iter()
            .filter(|candidate| candidate.kind() == super::super::MemoryCandidateKind::Episode)
            .count();
        let observation_count = plan
            .candidates
            .iter()
            .filter(|candidate| candidate.kind() == super::super::MemoryCandidateKind::Observation)
            .count();

        assert_eq!(episode_count, 1);
        assert_eq!(observation_count, 1);
    }
}
