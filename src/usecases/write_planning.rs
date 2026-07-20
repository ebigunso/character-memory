use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::api::types::lifecycle::ExternalSourceReference;
use crate::api::types::retrieval::MemoryObjectRef;
use crate::api::types::{
    CandidateProducerKind, CandidateProvenance, DerivedMemoryCandidate, EntityCandidate,
    EpisodeCandidate, MemoryCandidate, MemoryLinkCandidate, MemoryThreadCandidate,
    ObservationCandidate, RememberDiagnostics, RememberInput, RememberWritePlan, SourceSpan,
    StatsUpdateCandidate, VectorIndexCandidate,
};
use crate::api::types::{
    DerivedMemoryDraft, EntityDraft, EpisodeDraft, MemoryLinkDraft, MemoryThreadDraft,
    ObservationDraft,
};
use crate::domain::{graph_uri, MemoryId, ObjectType, RelationType, DEFAULT_SCHEMA_VERSION};

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

fn deterministic_uuid(parts: &[&[u8]]) -> MemoryId {
    let mut label = Vec::new();
    for part in parts {
        label.extend_from_slice(&(part.len() as u64).to_be_bytes());
        label.extend_from_slice(part);
    }

    uuid::Uuid::new_v5(&WRITE_PLAN_NAMESPACE, &label)
}

#[cfg(test)]
mod construction_tests {
    use super::*;
    use crate::domain::{DerivedType, EntityType};

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
            .filter(|candidate| candidate.kind() == MemoryCandidateKind::Episode)
            .count();
        let observation_count = plan
            .candidates
            .iter()
            .filter(|candidate| candidate.kind() == MemoryCandidateKind::Observation)
            .count();

        assert_eq!(episode_count, 1);
        assert_eq!(observation_count, 1);
    }
}

use std::collections::{HashMap, HashSet};

use crate::api::types::{CandidateRationale, DraftDefaults};
use crate::domain::{
    CandidateValidation, CandidateValidationStatus, MemoryCandidateKind, MemoryLink, MemoryObject,
    RetentionState,
};
use crate::errors::CustomError;
use crate::ports::graph_authority::{GraphAuthorityStore, GraphObjectQuery, GraphObjectRef};
use crate::usecases::{admit_link, LinkAdmissionDecision, LinkAdmissionEvidence};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WritePlanValidationVerdict {
    pub(crate) validations: Vec<CandidateValidation>,
    pub(crate) decision: WritePlanValidationDecision,
}

impl WritePlanValidationVerdict {
    pub(crate) fn is_valid(&self) -> bool {
        self.decision == WritePlanValidationDecision::Accepted
    }

    pub(crate) fn into_result(self) -> Result<Self, CustomError> {
        if self.is_valid() {
            Ok(self)
        } else {
            Err(CustomError::WritePlanValidationRejected {
                validations: self.validations,
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WritePlanValidationDecision {
    Accepted,
    Rejected,
}

pub(crate) struct WritePlanValidator<'a, G>
where
    G: GraphAuthorityStore + ?Sized,
{
    graph_store: &'a G,
}

impl<'a, G> WritePlanValidator<'a, G>
where
    G: GraphAuthorityStore + ?Sized,
{
    pub(crate) fn new(graph_store: &'a G) -> Self {
        Self { graph_store }
    }

    pub(crate) async fn validate(
        &self,
        plan: &RememberWritePlan,
    ) -> Result<WritePlanValidationVerdict, CustomError> {
        let mut context = PlanValidationContext::new(plan)?;
        let graph_refs = context.graph_refs_to_check();
        if !graph_refs.is_empty() {
            for object in self
                .graph_store
                .query_objects(&GraphObjectQuery::by_refs(graph_refs))
                .await?
            {
                context.add_existing_object(&object);
            }
        }

        let mut validations = plan
            .candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| context.validate_candidate(index, candidate))
            .collect::<Vec<_>>();
        if validations.is_empty() && !context.plan_errors.is_empty() {
            validations.push(CandidateValidation::invalid(
                0,
                MemoryCandidateKind::Episode,
                format!("write plan is invalid: {}", context.plan_errors.join("; ")),
            ));
        }
        let decision = if validations
            .iter()
            .all(|validation| validation.status == CandidateValidationStatus::Valid)
        {
            WritePlanValidationDecision::Accepted
        } else {
            WritePlanValidationDecision::Rejected
        };

        Ok(WritePlanValidationVerdict {
            validations,
            decision,
        })
    }
}

#[derive(Debug)]
struct PlanValidationContext {
    plan_refs: HashSet<GraphObjectRef>,
    refs_requiring_graph: HashSet<GraphObjectRef>,
    existing_refs: HashSet<GraphObjectRef>,
    episode_content_by_id: HashMap<MemoryId, String>,
    plan_errors: Vec<String>,
}

impl PlanValidationContext {
    fn new(plan: &RememberWritePlan) -> Result<Self, CustomError> {
        let mut context = Self {
            plan_refs: HashSet::new(),
            refs_requiring_graph: HashSet::new(),
            existing_refs: HashSet::new(),
            episode_content_by_id: HashMap::new(),
            plan_errors: validate_plan_identity(plan),
        };

        for candidate in &plan.candidates {
            context.collect_plan_ref(candidate);
            context.collect_referenced_refs(candidate);
            context.collect_echo_surface_data(candidate);
        }

        Ok(context)
    }

    fn collect_plan_ref(&mut self, candidate: &MemoryCandidate) {
        match candidate {
            MemoryCandidate::Episode(candidate) => {
                if let Some(id) = candidate.draft.id {
                    self.plan_refs
                        .insert(GraphObjectRef::new(id, ObjectType::Episode));
                }
            }
            MemoryCandidate::Observation(candidate) => {
                if let Some(id) = candidate.draft.id {
                    self.plan_refs
                        .insert(GraphObjectRef::new(id, ObjectType::Observation));
                }
            }
            MemoryCandidate::Entity(candidate) => {
                if let Some(id) = candidate.draft.id {
                    self.plan_refs
                        .insert(GraphObjectRef::new(id, ObjectType::Entity));
                }
            }
            MemoryCandidate::MemoryThread(candidate) => {
                if let Some(id) = candidate.draft.id {
                    self.plan_refs
                        .insert(GraphObjectRef::new(id, ObjectType::MemoryThread));
                }
            }
            MemoryCandidate::DerivedMemory(candidate) => {
                if let Some(id) = candidate.draft.id {
                    self.plan_refs
                        .insert(GraphObjectRef::new(id, ObjectType::DerivedMemory));
                }
            }
            MemoryCandidate::MemoryLink(candidate) => {
                if let Some(id) = candidate.draft.id {
                    self.plan_refs
                        .insert(GraphObjectRef::new(id, ObjectType::MemoryLink));
                }
            }
            MemoryCandidate::VectorIndex(_) | MemoryCandidate::StatsUpdate(_) => {}
        }
    }

    fn collect_referenced_refs(&mut self, candidate: &MemoryCandidate) {
        match candidate {
            MemoryCandidate::DerivedMemory(candidate) => {
                for episode_id in &candidate.draft.derived_from_episode_ids {
                    self.add_ref_to_check(GraphObjectRef::new(*episode_id, ObjectType::Episode));
                }
                for observation_id in &candidate.draft.derived_from_observation_ids {
                    self.add_ref_to_check(GraphObjectRef::new(
                        *observation_id,
                        ObjectType::Observation,
                    ));
                }
            }
            MemoryCandidate::MemoryLink(candidate) => {
                self.add_ref_to_check(GraphObjectRef::new(
                    candidate.draft.from_id,
                    candidate.draft.from_type,
                ));
                self.add_ref_to_check(GraphObjectRef::new(
                    candidate.draft.to_id,
                    candidate.draft.to_type,
                ));
            }
            MemoryCandidate::VectorIndex(candidate) => {
                self.add_ref_to_check(candidate.target.into());
            }
            MemoryCandidate::StatsUpdate(candidate) => {
                self.add_ref_to_check(candidate.subject.into());
                if let Some(object) = candidate.object {
                    self.add_ref_to_check(object.into());
                }
            }
            _ => {}
        }
    }

    fn collect_echo_surface_data(&mut self, candidate: &MemoryCandidate) {
        if let MemoryCandidate::Episode(candidate) = candidate {
            if let Some(id) = candidate.draft.id {
                self.episode_content_by_id
                    .entry(id)
                    .or_insert_with(|| candidate.draft.summary.clone());
            }
        }
    }

    fn add_ref_to_check(&mut self, object_ref: GraphObjectRef) {
        if !self.plan_refs.contains(&object_ref) {
            self.refs_requiring_graph.insert(object_ref);
        }
    }

    fn graph_refs_to_check(&self) -> Vec<GraphObjectRef> {
        self.refs_requiring_graph.iter().copied().collect()
    }

    fn add_existing_object(&mut self, object: &MemoryObject) {
        self.existing_refs.insert(memory_object_ref(object));
    }

    fn validate_candidate(&self, index: usize, candidate: &MemoryCandidate) -> CandidateValidation {
        let mut errors = self.plan_errors.clone();
        match candidate {
            MemoryCandidate::Episode(candidate) => {
                errors.extend(validate_provenance(&candidate.provenance));
                errors.extend(validate_required_candidate_identity(
                    "episode candidate",
                    candidate.draft.id,
                    candidate.draft.schema_version.as_deref(),
                ));
                errors.extend(validate_episode_timestamps(&candidate.draft));
                match candidate
                    .draft
                    .clone()
                    .into_domain_with_defaults(&mut DraftDefaults::generated())
                {
                    Ok(object) => errors.extend(validate_object(&MemoryObject::Episode(object))),
                    Err(error) => errors.push(error.to_string()),
                }
            }
            MemoryCandidate::Observation(candidate) => {
                errors.extend(validate_provenance(&candidate.provenance));
                errors.extend(validate_required_candidate_identity(
                    "observation candidate",
                    candidate.draft.id,
                    candidate.draft.schema_version.as_deref(),
                ));
                errors.extend(validate_required_created_at(
                    "observation candidate",
                    candidate.draft.created_at,
                ));
                match candidate
                    .draft
                    .clone()
                    .into_domain_with_defaults(&mut DraftDefaults::generated())
                {
                    Ok(object) => {
                        errors.extend(validate_object(&MemoryObject::Observation(object)))
                    }
                    Err(error) => errors.push(error.to_string()),
                }
            }
            MemoryCandidate::Entity(candidate) => {
                errors.extend(validate_provenance(&candidate.provenance));
                errors.extend(validate_required_candidate_identity(
                    "entity candidate",
                    candidate.draft.id,
                    candidate.draft.schema_version.as_deref(),
                ));
                errors.extend(validate_required_created_and_updated_at(
                    "entity candidate",
                    candidate.draft.created_at,
                    candidate.draft.updated_at,
                ));
                match candidate
                    .draft
                    .clone()
                    .into_domain_with_defaults(&mut DraftDefaults::generated())
                {
                    Ok(object) => errors.extend(validate_object(&MemoryObject::Entity(object))),
                    Err(error) => errors.push(error.to_string()),
                }
            }
            MemoryCandidate::MemoryThread(candidate) => {
                errors.extend(validate_provenance(&candidate.provenance));
                errors.extend(validate_required_candidate_identity(
                    "memory thread candidate",
                    candidate.draft.id,
                    candidate.draft.schema_version.as_deref(),
                ));
                errors.extend(validate_memory_thread_timestamps(&candidate.draft));
                match candidate
                    .draft
                    .clone()
                    .into_domain_with_defaults(&mut DraftDefaults::generated())
                {
                    Ok(object) => {
                        errors.extend(validate_object(&MemoryObject::MemoryThread(object)))
                    }
                    Err(error) => errors.push(error.to_string()),
                }
            }
            MemoryCandidate::DerivedMemory(candidate) => {
                errors.extend(validate_provenance(&candidate.provenance));
                errors.extend(validate_required_candidate_identity(
                    "derived memory candidate",
                    candidate.draft.id,
                    candidate.draft.schema_version.as_deref(),
                ));
                errors.extend(validate_required_created_and_updated_at(
                    "derived memory candidate",
                    candidate.draft.created_at,
                    candidate.draft.updated_at,
                ));
                match candidate
                    .draft
                    .clone()
                    .into_domain_with_defaults(&mut DraftDefaults::generated())
                {
                    Ok(object) => {
                        errors.extend(validate_object(&MemoryObject::DerivedMemory(
                            object.clone(),
                        )));
                        errors.extend(validate_derived_memory_lifecycle(&object));
                        errors.extend(self.validate_derived_sources(&object));
                    }
                    Err(error) => errors.push(error.to_string()),
                }
            }
            MemoryCandidate::MemoryLink(candidate) => {
                errors.extend(validate_provenance(&candidate.provenance));
                errors.extend(validate_required_candidate_identity(
                    "memory link candidate",
                    candidate.draft.id,
                    candidate.draft.schema_version.as_deref(),
                ));
                errors.extend(validate_memory_link_timestamps(&candidate.draft));
                match candidate
                    .draft
                    .clone()
                    .into_domain_with_defaults(&mut DraftDefaults::generated())
                {
                    Ok(link) => {
                        errors.extend(validate_link(&link));
                        errors.extend(self.validate_link_targets(&link));
                    }
                    Err(error) => errors.push(error.to_string()),
                }
            }
            MemoryCandidate::VectorIndex(candidate) => {
                errors.extend(validate_provenance(&candidate.provenance));
                if candidate.embedding_text.trim().is_empty() {
                    errors
                        .push("vector index candidate embedding_text must not be empty".to_owned());
                }
                errors.extend(self.validate_graph_authoritative_ref(
                    candidate.target.into(),
                    "vector index candidate target",
                ));
                errors.extend(self.validate_in_plan_ref(
                    candidate.target.into(),
                    "vector index candidate target",
                ));
            }
            MemoryCandidate::StatsUpdate(candidate) => {
                errors.extend(validate_provenance(&candidate.provenance));
                if candidate.relation.is_some() != candidate.object.is_some() {
                    errors.push(
                        "stats update candidate relation and object must be supplied together"
                            .to_owned(),
                    );
                }
                errors.extend(self.validate_graph_authoritative_ref(
                    candidate.subject.into(),
                    "stats update candidate subject",
                ));
                errors.extend(self.validate_in_plan_ref(
                    candidate.subject.into(),
                    "stats update candidate subject",
                ));
                if let Some(object) = candidate.object {
                    errors.extend(self.validate_graph_authoritative_ref(
                        object.into(),
                        "stats update candidate object",
                    ));
                    errors.extend(
                        self.validate_in_plan_ref(object.into(), "stats update candidate object"),
                    );
                }
            }
        }

        let mut validation = if errors.is_empty() {
            CandidateValidation::valid(index, candidate.kind())
        } else {
            let mut validation =
                CandidateValidation::invalid(index, candidate.kind(), errors[0].clone());
            validation.errors.extend(errors.into_iter().skip(1));
            validation
        };
        if let Some(warning) = self.echo_surface_warning(candidate) {
            validation.warnings.push(warning);
        }
        validation
    }

    fn echo_surface_warning(&self, candidate: &MemoryCandidate) -> Option<String> {
        let (candidate_content, source_episode_ids) = match candidate {
            MemoryCandidate::Observation(candidate) => (
                candidate.draft.text.as_str(),
                std::slice::from_ref(&candidate.draft.episode_id),
            ),
            MemoryCandidate::DerivedMemory(candidate) => (
                candidate.draft.text.as_str(),
                candidate.draft.derived_from_episode_ids.as_slice(),
            ),
            _ => return None,
        };

        let mut matching_episode_ids = source_episode_ids
            .iter()
            .filter(|episode_id| {
                let Some(episode_content) = self.episode_content_by_id.get(episode_id) else {
                    return false;
                };
                candidate_content == episode_content
            })
            .copied()
            .collect::<Vec<_>>();
        matching_episode_ids.sort();
        matching_episode_ids.dedup();
        if matching_episode_ids.is_empty() {
            return None;
        }

        Some(format!(
            "echo-surface: candidate content is byte-identical to source episode candidate(s): {}",
            matching_episode_ids
                .iter()
                .map(MemoryId::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ))
    }

    fn validate_derived_sources(&self, object: &crate::domain::DerivedMemory) -> Vec<String> {
        let mut errors = Vec::new();
        for episode_id in &object.derived_from_episode_ids {
            errors.extend(self.validate_graph_authoritative_ref(
                GraphObjectRef::new(*episode_id, ObjectType::Episode),
                "derived memory source episode",
            ));
        }
        for observation_id in &object.derived_from_observation_ids {
            errors.extend(self.validate_graph_authoritative_ref(
                GraphObjectRef::new(*observation_id, ObjectType::Observation),
                "derived memory source observation",
            ));
        }
        errors
    }

    fn validate_link_targets(&self, link: &MemoryLink) -> Vec<String> {
        let mut errors = Vec::new();
        errors.extend(self.validate_graph_authoritative_ref(
            GraphObjectRef::new(link.from_id, link.from_type),
            "memory link from target",
        ));
        errors.extend(self.validate_graph_authoritative_ref(
            GraphObjectRef::new(link.to_id, link.to_type),
            "memory link to target",
        ));
        errors
    }

    fn validate_graph_authoritative_ref(
        &self,
        object_ref: GraphObjectRef,
        label: &str,
    ) -> Vec<String> {
        if self.plan_refs.contains(&object_ref) || self.existing_refs.contains(&object_ref) {
            return Vec::new();
        }

        vec![format!(
            "{label} does not exist in write plan or graph: {:?} {}",
            object_ref.object_type, object_ref.object_id
        )]
    }

    fn validate_in_plan_ref(&self, object_ref: GraphObjectRef, label: &str) -> Vec<String> {
        if self.plan_refs.contains(&object_ref) {
            return Vec::new();
        }

        vec![format!(
            "{label} must reference an object candidate in the write plan: {:?} {}",
            object_ref.object_type, object_ref.object_id
        )]
    }
}

fn validate_plan_identity(plan: &RememberWritePlan) -> Vec<String> {
    let mut errors = Vec::new();
    if plan.operation_id.is_nil() {
        errors.push("write plan operation_id must be present".to_owned());
    }
    if plan.idempotency_key.trim().is_empty() {
        errors.push("write plan idempotency_key must be present".to_owned());
    }

    errors
}

pub(crate) struct WritePlanCommitValues {
    pub(crate) objects: Vec<MemoryObject>,
    pub(crate) links: Vec<MemoryLink>,
    pub(crate) vector_targets: Vec<MemoryObjectRef>,
}

impl WritePlanCommitValues {
    pub(crate) fn from_plan(plan: RememberWritePlan) -> Result<Self, CustomError> {
        let mut objects = Vec::new();
        let mut links = Vec::new();
        let mut vector_targets = Vec::new();
        let mut defaults = DraftDefaults::generated();

        for candidate in plan.candidates {
            match candidate {
                MemoryCandidate::Episode(candidate) => objects.push(MemoryObject::Episode(
                    stable_episode_draft(candidate.draft)?
                        .into_domain_with_defaults(&mut defaults)
                        .map_err(validation_error)?,
                )),
                MemoryCandidate::Observation(candidate) => objects.push(MemoryObject::Observation(
                    require_created_at(candidate.draft, "observation candidate")?
                        .into_domain_with_defaults(&mut defaults)
                        .map_err(validation_error)?,
                )),
                MemoryCandidate::Entity(candidate) => objects.push(MemoryObject::Entity(
                    require_created_and_updated_at(candidate.draft, "entity candidate")?
                        .into_domain_with_defaults(&mut defaults)
                        .map_err(validation_error)?,
                )),
                MemoryCandidate::MemoryThread(candidate) => {
                    objects.push(MemoryObject::MemoryThread(
                        stable_memory_thread_draft(candidate.draft)?
                            .into_domain_with_defaults(&mut defaults)
                            .map_err(validation_error)?,
                    ));
                }
                MemoryCandidate::DerivedMemory(candidate) => {
                    objects.push(MemoryObject::DerivedMemory(
                        require_created_and_updated_at(
                            candidate.draft,
                            "derived memory candidate",
                        )?
                        .into_domain_with_defaults(&mut defaults)
                        .map_err(validation_error)?,
                    ));
                }
                MemoryCandidate::MemoryLink(candidate) => {
                    links.push(
                        stable_memory_link_draft(candidate.draft)?
                            .into_domain_with_defaults(&mut defaults)
                            .map_err(validation_error)?,
                    );
                }
                MemoryCandidate::VectorIndex(candidate) => vector_targets.push(candidate.target),
                MemoryCandidate::StatsUpdate(_) => {}
            }
        }

        Ok(Self {
            objects,
            links,
            vector_targets,
        })
    }
}

trait CandidateCreatedAt {
    fn created_at(&self) -> Option<chrono::DateTime<chrono::Utc>>;
}

trait CandidateUpdatedAt: CandidateCreatedAt {
    fn updated_at(&self) -> Option<chrono::DateTime<chrono::Utc>>;
}

impl CandidateCreatedAt for crate::api::types::ObservationDraft {
    fn created_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.created_at
    }
}

impl CandidateCreatedAt for crate::api::types::EntityDraft {
    fn created_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.created_at
    }
}

impl CandidateUpdatedAt for crate::api::types::EntityDraft {
    fn updated_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.updated_at
    }
}

impl CandidateCreatedAt for crate::api::types::DerivedMemoryDraft {
    fn created_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.created_at
    }
}

impl CandidateUpdatedAt for crate::api::types::DerivedMemoryDraft {
    fn updated_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.updated_at
    }
}

fn require_created_at<T>(draft: T, label: &str) -> Result<T, CustomError>
where
    T: CandidateCreatedAt,
{
    if draft.created_at().is_none() {
        return Err(validation_error(format!(
            "{label} created_at must be present for deterministic commit"
        )));
    }
    Ok(draft)
}

fn require_created_and_updated_at<T>(draft: T, label: &str) -> Result<T, CustomError>
where
    T: CandidateUpdatedAt,
{
    require_created_at(draft, label).and_then(|draft| {
        if draft.updated_at().is_none() {
            return Err(validation_error(format!(
                "{label} updated_at must be present for deterministic commit"
            )));
        }
        Ok(draft)
    })
}

fn stable_episode_draft(draft: EpisodeDraft) -> Result<EpisodeDraft, CustomError> {
    if draft.created_at.is_none() {
        return Err(validation_error(
            "episode candidate created_at must be present for deterministic commit",
        ));
    }
    Ok(draft)
}

fn stable_memory_thread_draft(draft: MemoryThreadDraft) -> Result<MemoryThreadDraft, CustomError> {
    if draft.created_at.is_none() {
        return Err(validation_error(
            "memory thread candidate created_at must be present for deterministic commit",
        ));
    }
    if draft.updated_at.is_none() {
        return Err(validation_error(
            "memory thread candidate updated_at must be present for deterministic commit",
        ));
    }
    if draft.last_touched_at.is_none() {
        return Err(validation_error(
            "memory thread candidate last_touched_at must be present for deterministic commit",
        ));
    }
    Ok(draft)
}

fn stable_memory_link_draft(draft: MemoryLinkDraft) -> Result<MemoryLinkDraft, CustomError> {
    if draft.created_at.is_none() {
        return Err(validation_error(
            "memory link candidate created_at must be present for deterministic commit",
        ));
    }
    Ok(draft)
}

fn validate_episode_timestamps(draft: &EpisodeDraft) -> Vec<String> {
    validate_required_created_at("episode candidate", draft.created_at)
}

fn validate_memory_thread_timestamps(draft: &MemoryThreadDraft) -> Vec<String> {
    let mut errors = validate_required_created_and_updated_at(
        "memory thread candidate",
        draft.created_at,
        draft.updated_at,
    );
    if draft.last_touched_at.is_none() {
        errors.push(
            "memory thread candidate last_touched_at must be present for deterministic commit"
                .to_owned(),
        );
    }
    errors
}

fn validate_memory_link_timestamps(draft: &MemoryLinkDraft) -> Vec<String> {
    validate_required_created_at("memory link candidate", draft.created_at)
}

fn validate_required_created_at(
    label: &str,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Vec<String> {
    if created_at.is_none() {
        return vec![format!(
            "{label} created_at must be present for deterministic commit"
        )];
    }
    Vec::new()
}

fn validate_required_created_and_updated_at(
    label: &str,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Vec<String> {
    let mut errors = validate_required_created_at(label, created_at);
    if updated_at.is_none() {
        errors.push(format!(
            "{label} updated_at must be present for deterministic commit"
        ));
    }
    errors
}

fn validate_object(object: &MemoryObject) -> Vec<String> {
    let mut errors = Vec::new();
    if let Err(error) = object.validate() {
        errors.push(error.to_string());
    }
    if schema_version(object).trim().is_empty() {
        errors.push("memory object schema_version must be present".to_owned());
    }
    errors
}

fn validate_required_candidate_identity(
    label: &str,
    id: Option<MemoryId>,
    schema_version: Option<&str>,
) -> Vec<String> {
    let mut errors = Vec::new();
    if id.is_none_or(|id| id.is_nil()) {
        errors.push(format!("{label} id must be present"));
    }
    if schema_version.is_none_or(|value| value.trim().is_empty()) {
        errors.push(format!("{label} schema_version must be present"));
    }
    errors
}

fn validate_link(link: &MemoryLink) -> Vec<String> {
    let mut errors = Vec::new();
    if let Err(error) = link.validate() {
        errors.push(error.to_string());
    }
    if link.schema_version.trim().is_empty() {
        errors.push("memory link schema_version must be present".to_owned());
    }
    if admit_link(link, LinkAdmissionEvidence::ExplicitCallerIntent)
        == LinkAdmissionDecision::RejectedLowInformationCoOccurrence
    {
        errors.push("memory link rejected by link admission policy".to_owned());
    }
    errors
}

fn validate_derived_memory_lifecycle(object: &crate::domain::DerivedMemory) -> Vec<String> {
    let mut errors = Vec::new();
    if object.retention_state == RetentionState::Suppressed && object.is_current {
        errors.push("suppressed memories are not current".to_owned());
    }
    if !object.supersedes.is_empty()
        && object.is_current
        && object.retention_state != RetentionState::Archived
    {
        errors.push("superseded memories are not current unless explicitly historical".to_owned());
    }
    errors
}

fn validate_provenance(provenance: &crate::api::types::CandidateProvenance) -> Vec<String> {
    let mut errors = Vec::new();
    if provenance.producer_kind != CandidateProducerKind::Caller
        && matches!(
            provenance.rationale,
            CandidateRationale::ProvidedByCaller(_)
        )
    {
        errors.push("non-caller candidate cannot claim caller-provided rationale".to_owned());
    }
    if provenance
        .rationale
        .text()
        .is_some_and(|text| text.trim().is_empty())
    {
        errors.push("candidate rationale text must not be empty".to_owned());
    }
    for source_span in &provenance.source.source_spans {
        if let Err(error) = source_span.validate() {
            errors.push(error.to_string());
        }
    }
    for external_ref in &provenance.source.external_refs {
        if !external_ref.has_reference() {
            errors.push("candidate external source reference must not be empty".to_owned());
        }
    }
    errors
}

fn memory_object_ref(object: &MemoryObject) -> GraphObjectRef {
    match object {
        MemoryObject::Episode(object) => GraphObjectRef::new(object.id, ObjectType::Episode),
        MemoryObject::Observation(object) => {
            GraphObjectRef::new(object.id, ObjectType::Observation)
        }
        MemoryObject::Entity(object) => GraphObjectRef::new(object.id, ObjectType::Entity),
        MemoryObject::MemoryThread(object) => {
            GraphObjectRef::new(object.id, ObjectType::MemoryThread)
        }
        MemoryObject::DerivedMemory(object) => {
            GraphObjectRef::new(object.id, ObjectType::DerivedMemory)
        }
        MemoryObject::MemoryLink(object) => GraphObjectRef::new(object.id, ObjectType::MemoryLink),
    }
}

fn schema_version(object: &MemoryObject) -> &str {
    match object {
        MemoryObject::Episode(object) => &object.schema_version,
        MemoryObject::Observation(object) => &object.schema_version,
        MemoryObject::Entity(object) => &object.schema_version,
        MemoryObject::MemoryThread(object) => &object.schema_version,
        MemoryObject::DerivedMemory(object) => &object.schema_version,
        MemoryObject::MemoryLink(object) => &object.schema_version,
    }
}

impl From<MemoryObjectRef> for GraphObjectRef {
    fn from(value: MemoryObjectRef) -> Self {
        Self::new(value.id, value.object_type)
    }
}

fn validation_error(error: impl ToString) -> CustomError {
    CustomError::MemoryValidation(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    use super::RememberPlanDefaults;
    use crate::api::types::{
        CandidateProvenance, CandidateRationale, DerivedMemoryDraft, EntityDraft, EpisodeDraft,
        MemoryLinkDraft, RememberInput, SourceSpan, StatsUpdateCandidate, VectorIndexCandidate,
    };
    use crate::domain::{DerivedType, RelationType, Stability, DEFAULT_SCHEMA_VERSION};
    use crate::test_support::{representative_fixtures, FakeGraphAuthorityStore};

    #[tokio::test]
    async fn accepts_valid_plan_without_writes() {
        let graph = graph_with_fixtures().await;
        let fixtures = representative_fixtures();
        let plan = valid_plan().with_candidate(MemoryCandidate::MemoryLink(
            crate::api::types::MemoryLinkCandidate::new(
                link_draft(fixtures.user_entity.id, fixtures.episode.id),
                CandidateProvenance::caller("caller asked to link entity to episode"),
            ),
        ));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert!(verdict.is_valid());
        assert_eq!(graph.list_diagnostic_objects().await.unwrap().len(), 13);
        assert_eq!(graph.list_diagnostic_links().await.unwrap().len(), 5);
    }

    #[tokio::test]
    async fn warns_when_observation_content_echoes_source_episode_candidate() {
        let graph = FakeGraphAuthorityStore::new();
        let plan = valid_plan();
        let source_episode_id = plan
            .candidates
            .iter()
            .find_map(|candidate| match candidate {
                MemoryCandidate::Observation(candidate) => Some(candidate.draft.episode_id),
                _ => None,
            })
            .unwrap();

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert!(verdict.is_valid());
        let validation = verdict
            .validations
            .iter()
            .find(|validation| validation.candidate_kind == MemoryCandidateKind::Observation)
            .unwrap();
        assert_eq!(validation.status, CandidateValidationStatus::Valid);
        assert_eq!(validation.warnings.len(), 1);
        assert!(validation.warnings[0].contains("echo-surface"));
        assert!(validation.warnings[0].contains(&source_episode_id.to_string()));
    }

    #[tokio::test]
    async fn warns_when_derived_memory_content_echoes_source_episode_candidate() {
        let graph = FakeGraphAuthorityStore::new();
        let plan = RememberInput::new("source episode content")
            .with_observation(ObservationDraft::new(
                MemoryId::nil(),
                "distinct observation content",
            ))
            .with_derived_memory(DerivedMemoryDraft::new(
                DerivedType::Reflection,
                "source episode content",
            ))
            .prepare_write_plan_with_options(&defaults(), false, false);
        let source_episode_id = plan
            .candidates
            .iter()
            .find_map(|candidate| match candidate {
                MemoryCandidate::DerivedMemory(candidate) => {
                    candidate.draft.derived_from_episode_ids.first().copied()
                }
                _ => None,
            })
            .unwrap();

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert!(verdict.is_valid());
        let validation = verdict
            .validations
            .iter()
            .find(|validation| validation.candidate_kind == MemoryCandidateKind::DerivedMemory)
            .unwrap();
        assert_eq!(validation.status, CandidateValidationStatus::Valid);
        assert_eq!(validation.warnings.len(), 1);
        assert!(validation.warnings[0].contains("echo-surface"));
        assert!(validation.warnings[0].contains(&source_episode_id.to_string()));
    }

    #[tokio::test]
    async fn does_not_warn_for_distinct_surfaces_with_vector_candidates_enabled() {
        let graph = FakeGraphAuthorityStore::new();
        let plan = RememberInput::new("source episode content")
            .with_observation(ObservationDraft::new(
                MemoryId::nil(),
                "distinct observation content",
            ))
            .with_derived_memory(DerivedMemoryDraft::new(
                DerivedType::Reflection,
                "distinct derived content",
            ))
            .prepare_write_plan_with_options(&defaults(), true, false);

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert!(verdict.is_valid());
        assert!(verdict
            .validations
            .iter()
            .filter(|validation| {
                matches!(
                    validation.candidate_kind,
                    MemoryCandidateKind::Observation | MemoryCandidateKind::DerivedMemory
                )
            })
            .all(|validation| validation.warnings.is_empty()));
    }

    #[tokio::test]
    async fn does_not_warn_for_distinct_observation_and_derived_surfaces() {
        let graph = FakeGraphAuthorityStore::new();
        let plan = RememberInput::new("source episode content")
            .with_observation(ObservationDraft::new(
                MemoryId::nil(),
                "distinct observation content",
            ))
            .with_derived_memory(DerivedMemoryDraft::new(
                DerivedType::Reflection,
                "distinct derived content",
            ))
            .prepare_write_plan_with_options(&defaults(), false, false);

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert!(verdict.is_valid());
        assert!(verdict
            .validations
            .iter()
            .filter(|validation| {
                matches!(
                    validation.candidate_kind,
                    MemoryCandidateKind::Observation | MemoryCandidateKind::DerivedMemory
                )
            })
            .all(|validation| validation.warnings.is_empty()));
    }

    #[tokio::test]
    async fn rejects_missing_idempotency_key() {
        let graph = FakeGraphAuthorityStore::new();
        let mut plan = valid_plan();
        plan.idempotency_key.clear();

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(&verdict, "idempotency_key must be present");
    }

    #[tokio::test]
    async fn rejects_empty_plan_with_plan_identity_errors() {
        let graph = FakeGraphAuthorityStore::new();
        let mut plan = RememberWritePlan::new(id("00000000-0000-0000-0000-000000000000"), "");
        plan.idempotency_key.clear();

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert!(!verdict.validations.is_empty());
        assert_rejected_with(&verdict, "write plan operation_id must be present");
        assert_rejected_with(&verdict, "idempotency_key must be present");
        let error = verdict
            .into_result()
            .expect_err("invalid plan should return structured rejection rows");
        let CustomError::WritePlanValidationRejected { validations } = error else {
            panic!("expected structured write-plan validation rejection, got {error:?}");
        };
        assert!(validations.iter().any(|validation| {
            validation.candidate_kind == MemoryCandidateKind::Episode
                && validation.status == CandidateValidationStatus::Invalid
                && validation
                    .errors
                    .iter()
                    .any(|error| error.contains("write plan operation_id"))
                && validation
                    .errors
                    .iter()
                    .any(|error| error.contains("idempotency_key"))
        }));
    }

    #[tokio::test]
    async fn rejects_missing_schema_version() {
        let graph = FakeGraphAuthorityStore::new();
        let mut draft = EpisodeDraft::new("episode without schema");
        draft.id = Some(id("550e8400-e29b-41d4-a716-446655445001"));
        draft.schema_version = Some(String::new());
        let plan = valid_plan().with_candidate(MemoryCandidate::Episode(
            crate::api::types::EpisodeCandidate::new(
                draft,
                CandidateProvenance::caller("caller supplied episode"),
            ),
        ));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(&verdict, "schema_version must be present");
    }

    #[tokio::test]
    async fn rejects_missing_candidate_id() {
        let graph = FakeGraphAuthorityStore::new();
        let mut draft = EpisodeDraft::new("episode without id");
        draft.created_at = Some(timestamp());
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        let plan = valid_plan().with_candidate(MemoryCandidate::Episode(
            crate::api::types::EpisodeCandidate::new(
                draft,
                CandidateProvenance::caller("caller supplied episode"),
            ),
        ));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(&verdict, "episode candidate id must be present");
    }

    #[tokio::test]
    async fn rejects_ungrounded_derived_memory_provenance() {
        let graph = FakeGraphAuthorityStore::new();
        let derived = DerivedMemoryDraft::new(DerivedType::Reflection, "ungrounded reflection")
            .with_source_episode(id("550e8400-e29b-41d4-a716-446655445010"));
        let plan = valid_plan().with_candidate(MemoryCandidate::DerivedMemory(
            crate::api::types::DerivedMemoryCandidate::new(
                complete_derived(derived),
                CandidateProvenance::caller("caller supplied derived memory"),
            ),
        ));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(&verdict, "derived memory source episode does not exist");
    }

    #[tokio::test]
    async fn accepts_derived_memory_sources_existing_in_graph() {
        let graph = graph_with_fixtures().await;
        let fixtures = representative_fixtures();
        let derived = DerivedMemoryDraft::new(DerivedType::Reflection, "grounded reflection")
            .with_source_episode(fixtures.episode.id)
            .with_source_observation(fixtures.salient_observation.id);
        let plan = valid_plan().with_candidate(MemoryCandidate::DerivedMemory(
            crate::api::types::DerivedMemoryCandidate::new(
                complete_derived(derived),
                CandidateProvenance::caller("caller supplied derived memory"),
            ),
        ));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert!(verdict.is_valid());
    }

    #[tokio::test]
    async fn rejects_missing_link_targets() {
        let graph = FakeGraphAuthorityStore::new();
        let plan = valid_plan().with_candidate(MemoryCandidate::MemoryLink(
            crate::api::types::MemoryLinkCandidate::new(
                link_draft(
                    id("550e8400-e29b-41d4-a716-446655445020"),
                    id("550e8400-e29b-41d4-a716-446655445021"),
                ),
                CandidateProvenance::caller("caller asked to link missing targets"),
            ),
        ));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(&verdict, "memory link from target does not exist");
        assert_rejected_with(&verdict, "memory link to target does not exist");
    }

    #[tokio::test]
    async fn rejects_self_links_with_existing_targets() {
        let graph = graph_with_fixtures().await;
        let fixtures = representative_fixtures();
        let mut draft = MemoryLinkDraft::new(
            ObjectType::Episode,
            fixtures.episode.id,
            RelationType::AssociatedWith,
            ObjectType::Episode,
            fixtures.episode.id,
        );
        draft.id = Some(id("550e8400-e29b-41d4-a716-446655445030"));
        draft.created_at = Some(timestamp());
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        let plan = valid_plan().with_candidate(MemoryCandidate::MemoryLink(
            crate::api::types::MemoryLinkCandidate::new(
                draft,
                CandidateProvenance::caller("caller asked for self link"),
            ),
        ));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(&verdict, "cannot point from an object to itself");
    }

    #[tokio::test]
    async fn rejects_memory_link_endpoint_types() {
        let graph = FakeGraphAuthorityStore::new();
        let mut draft = MemoryLinkDraft::new(
            ObjectType::MemoryLink,
            id("550e8400-e29b-41d4-a716-446655445040"),
            RelationType::AssociatedWith,
            ObjectType::Episode,
            id("550e8400-e29b-41d4-a716-446655445041"),
        );
        draft.id = Some(id("550e8400-e29b-41d4-a716-446655445042"));
        draft.created_at = Some(timestamp());
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        let plan = valid_plan().with_candidate(MemoryCandidate::MemoryLink(
            crate::api::types::MemoryLinkCandidate::new(
                draft,
                CandidateProvenance::caller("caller supplied invalid link"),
            ),
        ));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(&verdict, "cannot point at MemoryLink endpoints");
    }

    #[tokio::test]
    async fn rejects_suppressed_current_derived_memory() {
        let graph = graph_with_fixtures().await;
        let fixtures = representative_fixtures();
        let mut derived = DerivedMemoryDraft::new(DerivedType::Reflection, "suppressed current")
            .with_source_episode(fixtures.episode.id);
        derived.retention_state = RetentionState::Suppressed;
        derived.is_current = true;
        let plan = valid_plan().with_candidate(MemoryCandidate::DerivedMemory(
            crate::api::types::DerivedMemoryCandidate::new(
                complete_derived(derived),
                CandidateProvenance::caller("caller supplied lifecycle state"),
            ),
        ));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(&verdict, "suppressed memories are not current");
    }

    #[tokio::test]
    async fn rejects_current_superseding_derived_memory_unless_historical() {
        let graph = graph_with_fixtures().await;
        let fixtures = representative_fixtures();
        let mut derived = DerivedMemoryDraft::new(DerivedType::Correction, "new correction")
            .with_source_episode(fixtures.episode.id);
        derived.supersedes.push(fixtures.user_preference.id);
        derived.is_current = true;
        derived.retention_state = RetentionState::Active;
        let plan = valid_plan().with_candidate(MemoryCandidate::DerivedMemory(
            crate::api::types::DerivedMemoryCandidate::new(
                complete_derived(derived),
                CandidateProvenance::caller("caller supplied correction"),
            ),
        ));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(
            &verdict,
            "superseded memories are not current unless explicitly historical",
        );
    }

    #[tokio::test]
    async fn rejects_vector_index_for_missing_graph_object() {
        let graph = FakeGraphAuthorityStore::new();
        let plan =
            valid_plan().with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                MemoryObjectRef::new(
                    ObjectType::Episode,
                    id("550e8400-e29b-41d4-a716-446655445050"),
                ),
                "embedding text",
                CandidateProvenance::caller("caller supplied vector candidate"),
            )));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(&verdict, "vector index candidate target does not exist");
    }

    #[tokio::test]
    async fn rejects_vector_index_for_graph_only_object() {
        let graph = graph_with_fixtures().await;
        let fixtures = representative_fixtures();
        let plan =
            valid_plan().with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                MemoryObjectRef::new(ObjectType::Episode, fixtures.episode.id),
                "embedding text",
                CandidateProvenance::caller("caller supplied vector candidate"),
            )));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(
            &verdict,
            "must reference an object candidate in the write plan",
        );
    }

    #[tokio::test]
    async fn rejects_stats_update_for_graph_only_object() {
        let graph = graph_with_fixtures().await;
        let fixtures = representative_fixtures();
        let plan =
            valid_plan().with_candidate(MemoryCandidate::StatsUpdate(StatsUpdateCandidate::new(
                MemoryObjectRef::new(ObjectType::Episode, fixtures.episode.id),
                CandidateProvenance::caller("caller supplied stats candidate"),
            )));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(
            &verdict,
            "must reference an object candidate in the write plan",
        );
    }

    #[tokio::test]
    async fn rejects_missing_candidate_timestamps() {
        let graph = FakeGraphAuthorityStore::new();
        let mut draft = EntityDraft::new(crate::domain::EntityType::Project, "no timestamps");
        draft.id = Some(id("550e8400-e29b-41d4-a716-446655445055"));
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        let plan = valid_plan().with_candidate(MemoryCandidate::Entity(
            crate::api::types::EntityCandidate::new(
                draft,
                CandidateProvenance::caller("caller supplied entity"),
            ),
        ));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(&verdict, "entity candidate created_at must be present");
        assert_rejected_with(&verdict, "entity candidate updated_at must be present");
    }

    #[test]
    fn commit_values_reject_missing_timestamps_before_defaults() {
        let mut draft = EpisodeDraft::new("missing timestamp defense");
        draft.id = Some(id("550e8400-e29b-41d4-a716-446655445056"));
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        let plan = RememberWritePlan::new(
            id("550e8400-e29b-41d4-a716-446655445057"),
            "missing-timestamp-defense",
        )
        .with_candidate(MemoryCandidate::Episode(
            crate::api::types::EpisodeCandidate::new(
                draft,
                CandidateProvenance::caller("caller supplied episode"),
            ),
        ));

        let error = match WritePlanCommitValues::from_plan(plan) {
            Ok(_) => panic!("missing timestamp plan should reject before defaults"),
            Err(error) => error,
        };

        assert!(error
            .to_string()
            .contains("episode candidate created_at must be present"));
    }

    #[tokio::test]
    async fn rejects_stats_update_for_missing_graph_object() {
        let graph = FakeGraphAuthorityStore::new();
        let plan =
            valid_plan().with_candidate(MemoryCandidate::StatsUpdate(StatsUpdateCandidate::new(
                MemoryObjectRef::new(
                    ObjectType::Episode,
                    id("550e8400-e29b-41d4-a716-446655445060"),
                ),
                CandidateProvenance::caller("caller supplied stats candidate"),
            )));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(&verdict, "stats update candidate subject does not exist");
    }

    #[tokio::test]
    async fn rejects_invalid_source_span() {
        let graph = FakeGraphAuthorityStore::new();
        let plan = valid_plan().with_candidate(MemoryCandidate::Episode(
            crate::api::types::EpisodeCandidate::new(
                complete_episode(EpisodeDraft::new("bad span")),
                CandidateProvenance::caller("caller supplied episode")
                    .with_source_span(SourceSpan::source("source://1").with_char_range(9, 3)),
            ),
        ));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(
            &verdict,
            "character range start must be less than or equal to end",
        );
    }

    #[tokio::test]
    async fn rejects_producer_rationale_origin_conflation() {
        let graph = FakeGraphAuthorityStore::new();
        let plan = valid_plan().with_candidate(MemoryCandidate::Episode(
            crate::api::types::EpisodeCandidate::new(
                complete_episode(EpisodeDraft::new("bad provenance")),
                CandidateProvenance::new(CandidateProducerKind::ModelProcessor)
                    .with_rationale(CandidateRationale::provided_by_caller("not caller")),
            ),
        ));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(&verdict, "cannot claim caller-provided rationale");
    }

    #[tokio::test]
    async fn raw_ref_is_validated_only_as_opaque_structure() {
        let graph = FakeGraphAuthorityStore::new();
        let plan = RememberInput::new("opaque raw ref")
            .with_raw_ref("raw://does/not/need/to/exist")
            .prepare_write_plan_with_options(&defaults(), false, false);

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert!(verdict.is_valid());
    }

    fn valid_plan() -> RememberWritePlan {
        RememberInput::new("valid minimal plan").prepare_write_plan_with_options(
            &defaults(),
            false,
            false,
        )
    }

    fn defaults() -> RememberPlanDefaults {
        RememberPlanDefaults::fixed("validator-tests", timestamp())
    }

    fn complete_episode(mut draft: EpisodeDraft) -> EpisodeDraft {
        draft
            .id
            .get_or_insert(id("550e8400-e29b-41d4-a716-446655444100"));
        draft.created_at.get_or_insert(timestamp());
        draft
            .schema_version
            .get_or_insert_with(|| DEFAULT_SCHEMA_VERSION.to_owned());
        draft
    }

    fn complete_derived(mut draft: DerivedMemoryDraft) -> DerivedMemoryDraft {
        draft
            .id
            .get_or_insert(id("550e8400-e29b-41d4-a716-446655444101"));
        draft.created_at.get_or_insert(timestamp());
        draft.updated_at.get_or_insert(timestamp());
        draft
            .schema_version
            .get_or_insert_with(|| DEFAULT_SCHEMA_VERSION.to_owned());
        draft.stability = Stability::Medium;
        draft
    }

    fn link_draft(from_id: MemoryId, to_id: MemoryId) -> MemoryLinkDraft {
        let mut draft = MemoryLinkDraft::new(
            ObjectType::Entity,
            from_id,
            RelationType::Involves,
            ObjectType::Episode,
            to_id,
        );
        draft.id = Some(id("550e8400-e29b-41d4-a716-446655444102"));
        draft.created_at = Some(timestamp());
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        draft
    }

    async fn graph_with_fixtures() -> FakeGraphAuthorityStore {
        let graph = FakeGraphAuthorityStore::new();
        let fixtures = representative_fixtures();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        graph
    }

    fn assert_rejected_with(verdict: &WritePlanValidationVerdict, expected: &str) {
        assert_eq!(verdict.decision, WritePlanValidationDecision::Rejected);
        assert!(
            verdict
                .validations
                .iter()
                .flat_map(|validation| validation.errors.iter())
                .any(|error| error.contains(expected)),
            "expected error containing {expected:?}, got {:?}",
            verdict.validations
        );
    }

    fn timestamp() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-07-03T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn id(value: &str) -> MemoryId {
        Uuid::parse_str(value).unwrap()
    }
}
