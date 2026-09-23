use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::api::types::lifecycle::ExternalSourceReference;
use crate::api::types::{
    CandidateProducerKind, CandidateProvenance, DerivedMemoryCandidate, EntityCandidate,
    EpisodeCandidate, MemoryCandidate, MemoryLinkCandidate, MemoryThreadCandidate,
    ObservationCandidate, RememberDiagnostics, RememberInput, RememberWritePlan, SourceSpan,
    SourceSpanValidationError, StatsUpdateCandidate, VectorIndexCandidate,
};
use crate::api::types::{
    DerivedMemoryDraft, EntityDraft, EpisodeDraft, MemoryLinkDraft, MemoryThreadDraft,
    ObservationDraft,
};
use crate::domain::MemoryObjectRef;
use crate::domain::{graph_uri, MemoryId, ObjectType, RelationType, Scene, DEFAULT_SCHEMA_VERSION};

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
        let scene = self
            .episode_drafts
            .first()
            .and_then(|draft| draft.scene.as_ref())
            .or(self.scene.as_ref())
            .cloned()
            .unwrap_or_else(|| Scene::at((defaults.created_at).fixed_offset()));
        let mut plan = RememberWritePlan::new();

        if let Some(source_input_ref) = self.source_reference() {
            plan = plan.with_source_input_ref(source_input_ref);
        }

        let episode_provenance = self.helper_provenance();
        let episode = EpisodeCandidate::new(
            self.episode_candidate_draft(defaults, &scene),
            episode_provenance,
        );
        plan = plan.with_candidate(MemoryCandidate::Episode(episode));

        let observation_provenance = self
            .helper_provenance()
            .with_source_episode(refs.episode_id);
        let observation = ObservationCandidate::new(
            self.observation_candidate_draft(defaults, refs.episode_id, &scene),
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
                plan = plan.with_candidate(MemoryCandidate::VectorIndex(
                    VectorIndexCandidate::new(object_ref, self.helper_provenance()),
                ));
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
            episode_id,
            observation_id,
            candidate_refs,
        }
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

    fn episode_candidate_draft(
        &self,
        defaults: &RememberPlanDefaults,
        scene: &Scene,
    ) -> EpisodeDraft {
        let mut draft = self
            .episode_drafts
            .first()
            .cloned()
            .unwrap_or_else(|| EpisodeDraft::new(self.content.clone()));
        draft
            .id
            .get_or_insert_with(|| defaults.stable_id("episode:0"));
        draft.scene = Some(scene.clone());
        draft.ended_at = draft.ended_at.or(self.ended_at);
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
        scene: &Scene,
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
        draft.observed_at = draft.observed_at.or(Some(scene.time.to_utc()));
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
    if !draft.given_by_application
        && draft.derived_from_episode_ids.is_empty()
        && draft.derived_from_observation_ids.is_empty()
    {
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

pub(crate) fn deterministic_uuid(parts: &[&[u8]]) -> MemoryId {
    let mut label = Vec::new();
    for part in parts {
        label.extend_from_slice(&(part.len() as u64).to_be_bytes());
        label.extend_from_slice(part);
    }

    uuid::Uuid::new_v5(&WRITE_PLAN_NAMESPACE, &label)
}

/// Derives traversal and currency links from the authored memory lists.
pub(crate) fn derived_memory_links(memory: &crate::domain::DerivedMemory) -> Vec<MemoryLink> {
    let mut links = memory
        .supersedes
        .iter()
        .map(|predecessor| MemoryLink {
            id: deterministic_uuid(&[
                b"character_memory.lifecycle.supersedes_link",
                memory.id.as_bytes(),
                predecessor.as_bytes(),
            ]),
            object_type: ObjectType::MemoryLink,
            from_id: memory.id,
            from_type: ObjectType::DerivedMemory,
            to_id: *predecessor,
            to_type: ObjectType::DerivedMemory,
            relation: RelationType::Supersedes,
            rationale: None,
            created_at: memory.created_at,
            schema_version: memory.schema_version.clone(),
        })
        .collect::<Vec<_>>();
    links.extend(memory.entity_ids.iter().map(|subject| MemoryLink {
        id: deterministic_uuid(&[
            b"character_memory.belief.about_link",
            memory.id.as_bytes(),
            subject.as_bytes(),
        ]),
        object_type: ObjectType::MemoryLink,
        from_id: memory.id,
        from_type: ObjectType::DerivedMemory,
        to_id: *subject,
        to_type: ObjectType::Entity,
        relation: RelationType::About,
        rationale: None,
        created_at: memory.created_at,
        schema_version: memory.schema_version.clone(),
    }));
    links.sort_by_key(|link| link.id);
    links.dedup_by_key(|link| link.id);
    links
}

#[cfg(test)]
mod construction_tests {
    use super::*;
    use crate::domain::DerivedType;

    use crate::test_support::{parse_id as memory_id, timestamp};

    #[test]
    fn same_input_and_fixed_defaults_prepare_identical_plan() {
        let defaults =
            RememberPlanDefaults::fixed("fixed-operation", timestamp("2026-07-03T10:00:00Z"));
        let mut scene = Scene::at((defaults.created_at).fixed_offset());
        scene.participants.push(crate::SceneParticipant {
            key: Some(memory_id("550e8400-e29b-41d4-a716-446655443003")),
            ..Default::default()
        });
        let input = RememberInput::new("Caller said they prefer terse planning notes.")
            .with_raw_ref("raw://conversation/7#turn=2")
            .with_source_span(SourceSpan::raw("raw://conversation/7#turn=2").with_turn_range(2, 2))
            .with_entity_id(memory_id("550e8400-e29b-41d4-a716-446655443001"))
            .with_thread_id(memory_id("550e8400-e29b-41d4-a716-446655443002"))
            .with_scene(scene)
            .with_derived_memory(DerivedMemoryDraft::new(
                DerivedType::Reflection,
                "Caller-provided reflection text.",
            ))
            .with_entity(EntityDraft::new());

        let first = input.prepare_write_plan(&defaults);
        let second = input.prepare_write_plan(&defaults);

        assert_eq!(first, second);
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
    fn source_refs_remain_opaque_and_verbatim() {
        let span = SourceSpan::raw("opaque://system/raw/123")
            .with_message_id("message-7")
            .with_char_range(3, 31);
        let input = RememberInput::new("Caller content")
            .with_raw_ref("opaque://system/raw/123")
            .with_source_span(span.clone());

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

        let defaults =
            RememberPlanDefaults::fixed("source-refs", timestamp("2026-07-03T10:05:00Z"));
        let plan = input.prepare_write_plan(&defaults);
        assert_eq!(
            plan.source_input_ref,
            Some(ExternalSourceReference::raw("opaque://system/raw/123"))
        );
        assert!(plan.candidates.iter().any(|candidate| matches!(
            candidate,
            MemoryCandidate::Episode(candidate)
                if candidate.provenance.source.source_spans.contains(&span)
        )));
    }

    #[test]
    fn candidate_ids_follow_defaults_when_content_changes() {
        let defaults =
            RememberPlanDefaults::fixed("same-defaults", timestamp("2026-07-03T10:15:00Z"));
        let first = RememberInput::new("first content");
        let second = RememberInput::new("second content");

        assert_eq!(
            first.prepared_candidate_refs(&defaults).episode_id,
            second.prepared_candidate_refs(&defaults).episode_id
        );
    }
}

use std::collections::{HashMap, HashSet};

use crate::api::types::{CandidateRationale, DraftDefaults};
use crate::domain::{
    CandidateProvenanceIssue, CandidateReferenceRole, CandidateScoreField,
    CandidateSourceSpanIssue, CandidateTimestampField, CandidateValidation,
    CandidateValidationIssue, CandidateValidationStatus, DomainValidationError, MemoryLink,
    MemoryLinkEndpoint, MemoryObject,
};
use crate::errors::CustomError;
use crate::ports::graph_authority::{GraphAuthorityStore, GraphObjectQuery};

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

        let validations = plan
            .candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| context.validate_candidate(index, candidate))
            .collect::<Vec<_>>();
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
    plan_refs: HashSet<MemoryObjectRef>,
    refs_requiring_graph: HashSet<MemoryObjectRef>,
    existing_refs: HashSet<MemoryObjectRef>,
    episode_content_by_id: HashMap<MemoryId, String>,
    derived_subjects_and_threads: HashMap<MemoryId, (Vec<MemoryId>, Vec<MemoryId>)>,
}

impl PlanValidationContext {
    fn new(plan: &RememberWritePlan) -> Result<Self, CustomError> {
        let mut context = Self {
            plan_refs: HashSet::new(),
            refs_requiring_graph: HashSet::new(),
            existing_refs: HashSet::new(),
            episode_content_by_id: HashMap::new(),
            derived_subjects_and_threads: HashMap::new(),
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
                        .insert(MemoryObjectRef::from_id_type(id, ObjectType::Episode));
                }
            }
            MemoryCandidate::Observation(candidate) => {
                if let Some(id) = candidate.draft.id {
                    self.plan_refs
                        .insert(MemoryObjectRef::from_id_type(id, ObjectType::Observation));
                }
            }
            MemoryCandidate::Entity(candidate) => {
                if let Some(id) = candidate.draft.id {
                    self.plan_refs
                        .insert(MemoryObjectRef::from_id_type(id, ObjectType::Entity));
                }
            }
            MemoryCandidate::MemoryThread(candidate) => {
                if let Some(id) = candidate.draft.id {
                    self.plan_refs
                        .insert(MemoryObjectRef::from_id_type(id, ObjectType::MemoryThread));
                }
            }
            MemoryCandidate::DerivedMemory(candidate) => {
                if let Some(id) = candidate.draft.id {
                    self.plan_refs
                        .insert(MemoryObjectRef::from_id_type(id, ObjectType::DerivedMemory));
                    self.derived_subjects_and_threads
                        .entry(id)
                        .or_insert_with(|| {
                            (
                                candidate.draft.entity_ids.clone(),
                                candidate.draft.thread_ids.clone(),
                            )
                        });
                }
            }
            MemoryCandidate::MemoryLink(candidate) => {
                if let Some(id) = candidate.draft.id {
                    self.plan_refs
                        .insert(MemoryObjectRef::from_id_type(id, ObjectType::MemoryLink));
                }
            }
            MemoryCandidate::VectorIndex(_) | MemoryCandidate::StatsUpdate(_) => {}
        }
    }

    fn collect_referenced_refs(&mut self, candidate: &MemoryCandidate) {
        match candidate {
            MemoryCandidate::Observation(candidate) => {
                self.add_ref_to_check(MemoryObjectRef::new(
                    ObjectType::Episode,
                    candidate.draft.episode_id,
                ));
            }
            MemoryCandidate::Episode(candidate) => {
                if let Some(scene) = &candidate.draft.scene {
                    for id in scene.participant_keys() {
                        self.add_ref_to_check(MemoryObjectRef::new(ObjectType::Entity, id));
                    }
                }
            }
            MemoryCandidate::DerivedMemory(candidate) => {
                for entity_id in &candidate.draft.entity_ids {
                    self.add_ref_to_check(MemoryObjectRef::new(ObjectType::Entity, *entity_id));
                }
                for predecessor_id in &candidate.draft.supersedes {
                    self.refs_requiring_graph
                        .insert(MemoryObjectRef::from_id_type(
                            *predecessor_id,
                            ObjectType::DerivedMemory,
                        ));
                }
                for episode_id in &candidate.draft.derived_from_episode_ids {
                    self.add_ref_to_check(MemoryObjectRef::from_id_type(
                        *episode_id,
                        ObjectType::Episode,
                    ));
                }
                for observation_id in &candidate.draft.derived_from_observation_ids {
                    self.add_ref_to_check(MemoryObjectRef::from_id_type(
                        *observation_id,
                        ObjectType::Observation,
                    ));
                }
            }
            MemoryCandidate::MemoryLink(candidate) => {
                self.add_ref_to_check(MemoryObjectRef::from_id_type(
                    candidate.draft.from_id,
                    candidate.draft.from_type,
                ));
                self.add_ref_to_check(MemoryObjectRef::from_id_type(
                    candidate.draft.to_id,
                    candidate.draft.to_type,
                ));
            }
            MemoryCandidate::VectorIndex(candidate) => {
                self.add_ref_to_check(candidate.target);
            }
            MemoryCandidate::StatsUpdate(candidate) => {
                self.add_ref_to_check(candidate.subject);
                if let Some(object) = candidate.object {
                    self.add_ref_to_check(object);
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

    fn add_ref_to_check(&mut self, object_ref: MemoryObjectRef) {
        if !self.plan_refs.contains(&object_ref) {
            self.refs_requiring_graph.insert(object_ref);
        }
    }

    fn graph_refs_to_check(&self) -> Vec<MemoryObjectRef> {
        self.refs_requiring_graph.iter().copied().collect()
    }

    fn add_existing_object(&mut self, object: &MemoryObject) {
        self.existing_refs.insert(object.object_ref());
        if let MemoryObject::DerivedMemory(memory) = object {
            self.derived_subjects_and_threads
                .entry(memory.id)
                .or_insert_with(|| (memory.entity_ids.clone(), memory.thread_ids.clone()));
        }
    }

    fn validate_candidate(&self, index: usize, candidate: &MemoryCandidate) -> CandidateValidation {
        let mut errors = Vec::new();
        match candidate {
            MemoryCandidate::Episode(candidate) => {
                errors.extend(validate_provenance(&candidate.provenance));
                errors.extend(validate_required_candidate_identity(
                    "episode candidate",
                    candidate.draft.id,
                    candidate.draft.schema_version.as_deref(),
                ));
                errors.extend(validate_episode_timestamps(&candidate.draft));
                if let Some(scene) = &candidate.draft.scene {
                    for id in scene.participant_keys() {
                        errors.extend(self.validate_graph_authoritative_ref(
                            MemoryObjectRef::new(ObjectType::Entity, id),
                            CandidateReferenceRole::SceneParticipant,
                        ));
                    }
                }
                match candidate
                    .draft
                    .clone()
                    .into_domain_with_defaults(&mut DraftDefaults::generated())
                {
                    Ok(object) => errors.extend(validate_object(&MemoryObject::Episode(object))),
                    Err(error) => errors.push(candidate_issue_from_domain_error(error)),
                }
            }
            MemoryCandidate::Observation(candidate) => {
                errors.extend(validate_provenance(&candidate.provenance));
                errors.extend(self.validate_graph_authoritative_ref(
                    MemoryObjectRef::new(ObjectType::Episode, candidate.draft.episode_id),
                    CandidateReferenceRole::ObservationEpisode,
                ));
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
                    Err(error) => errors.push(candidate_issue_from_domain_error(error)),
                }
            }
            MemoryCandidate::Entity(candidate) => {
                errors.extend(validate_provenance(&candidate.provenance));
                errors.extend(validate_required_candidate_identity(
                    "entity candidate",
                    candidate.draft.id,
                    candidate.draft.schema_version.as_deref(),
                ));
                errors.extend(validate_required_created_at(
                    "entity candidate",
                    candidate.draft.created_at,
                ));
                match candidate
                    .draft
                    .clone()
                    .into_domain_with_defaults(&mut DraftDefaults::generated())
                {
                    Ok(object) => errors.extend(validate_object(&MemoryObject::Entity(object))),
                    Err(error) => errors.push(candidate_issue_from_domain_error(error)),
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
                    Err(error) => errors.push(candidate_issue_from_domain_error(error)),
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
                        errors.extend(self.validate_derived_sources(&object));
                    }
                    Err(error) => errors.push(candidate_issue_from_domain_error(error)),
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
                    Err(error) => errors.push(candidate_issue_from_domain_error(error)),
                }
            }
            MemoryCandidate::VectorIndex(candidate) => {
                errors.extend(validate_provenance(&candidate.provenance));
                errors.extend(self.validate_graph_authoritative_ref(
                    candidate.target,
                    CandidateReferenceRole::VectorIndexTarget,
                ));
                errors.extend(self.validate_in_plan_ref(
                    candidate.target,
                    CandidateReferenceRole::VectorIndexTarget,
                ));
            }
            MemoryCandidate::StatsUpdate(candidate) => {
                errors.extend(validate_provenance(&candidate.provenance));
                if candidate.relation.is_some() != candidate.object.is_some() {
                    errors.push(CandidateValidationIssue::IncompleteStatsRelationObjectPair);
                }
                errors.extend(self.validate_graph_authoritative_ref(
                    candidate.subject,
                    CandidateReferenceRole::StatsUpdateSubject,
                ));
                errors.extend(self.validate_in_plan_ref(
                    candidate.subject,
                    CandidateReferenceRole::StatsUpdateSubject,
                ));
                if let Some(object) = candidate.object {
                    errors.extend(self.validate_graph_authoritative_ref(
                        object,
                        CandidateReferenceRole::StatsUpdateObject,
                    ));
                    errors.extend(
                        self.validate_in_plan_ref(
                            object,
                            CandidateReferenceRole::StatsUpdateObject,
                        ),
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
        match candidate {
            MemoryCandidate::MemoryLink(candidate) => {
                if let Some(warning) = self.resolver_warning(&candidate.draft) {
                    validation.warnings.push(warning);
                }
            }
            MemoryCandidate::Episode(candidate) => {
                if let Some(scene) = &candidate.draft.scene {
                    let mut seen = HashSet::new();
                    let mut warned = HashSet::new();
                    for participant_id in scene.participant_keys() {
                        if !seen.insert(participant_id) && warned.insert(participant_id) {
                            validation.warnings.push(
                                CandidateValidationIssue::RepeatedSceneParticipant {
                                    participant_id,
                                },
                            );
                        }
                    }
                }
            }
            _ => {}
        }
        validation
    }

    fn resolver_warning(&self, link: &MemoryLinkDraft) -> Option<CandidateValidationIssue> {
        if !matches!(
            link.relation,
            RelationType::Resolves | RelationType::FulfillsCommitment
        ) || link.from_type != ObjectType::DerivedMemory
            || link.to_type != ObjectType::DerivedMemory
            || link.from_id == link.to_id
        {
            return None;
        }
        let (resolver_subjects, resolver_threads) =
            self.derived_subjects_and_threads.get(&link.from_id)?;
        let (target_subjects, target_threads) =
            self.derived_subjects_and_threads.get(&link.to_id)?;
        if resolver_subjects
            .iter()
            .any(|id| target_subjects.contains(id))
            || resolver_threads
                .iter()
                .any(|id| target_threads.contains(id))
        {
            return None;
        }
        Some(
            CandidateValidationIssue::ResolverWithoutSharedSubjectOrThread {
                resolver_id: link.from_id,
                target_id: link.to_id,
            },
        )
    }

    fn echo_surface_warning(
        &self,
        candidate: &MemoryCandidate,
    ) -> Option<CandidateValidationIssue> {
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

        Some(CandidateValidationIssue::DuplicateObservationEcho {
            echo_surface: candidate_content.to_owned(),
            matching_episode_ids,
        })
    }

    fn validate_derived_sources(
        &self,
        object: &crate::domain::DerivedMemory,
    ) -> Vec<CandidateValidationIssue> {
        let mut errors = Vec::new();
        for entity_id in &object.entity_ids {
            errors.extend(self.validate_graph_authoritative_ref(
                MemoryObjectRef::new(ObjectType::Entity, *entity_id),
                CandidateReferenceRole::BeliefSubject,
            ));
        }
        for episode_id in &object.derived_from_episode_ids {
            errors.extend(self.validate_graph_authoritative_ref(
                MemoryObjectRef::from_id_type(*episode_id, ObjectType::Episode),
                CandidateReferenceRole::DerivedSourceEpisode,
            ));
        }
        for observation_id in &object.derived_from_observation_ids {
            errors.extend(self.validate_graph_authoritative_ref(
                MemoryObjectRef::from_id_type(*observation_id, ObjectType::Observation),
                CandidateReferenceRole::DerivedSourceObservation,
            ));
        }
        for predecessor_id in &object.supersedes {
            errors.extend(self.validate_graph_authoritative_ref(
                MemoryObjectRef::from_id_type(*predecessor_id, ObjectType::DerivedMemory),
                CandidateReferenceRole::SupersededMemory,
            ));
        }
        errors
    }

    fn validate_link_targets(&self, link: &MemoryLink) -> Vec<CandidateValidationIssue> {
        let mut errors = Vec::new();
        errors.extend(self.validate_graph_authoritative_ref(
            MemoryObjectRef::from_id_type(link.from_id, link.from_type),
            CandidateReferenceRole::MemoryLinkFrom,
        ));
        errors.extend(self.validate_graph_authoritative_ref(
            MemoryObjectRef::from_id_type(link.to_id, link.to_type),
            CandidateReferenceRole::MemoryLinkTo,
        ));
        errors
    }

    fn validate_graph_authoritative_ref(
        &self,
        object_ref: MemoryObjectRef,
        role: CandidateReferenceRole,
    ) -> Vec<CandidateValidationIssue> {
        if self.existing_refs.contains(&object_ref)
            || (role != CandidateReferenceRole::SupersededMemory
                && self.plan_refs.contains(&object_ref))
        {
            return Vec::new();
        }

        vec![CandidateValidationIssue::UnknownObjectRef {
            role,
            referenced: object_ref,
        }]
    }

    fn validate_in_plan_ref(
        &self,
        object_ref: MemoryObjectRef,
        role: CandidateReferenceRole,
    ) -> Vec<CandidateValidationIssue> {
        if self.plan_refs.contains(&object_ref) {
            return Vec::new();
        }

        vec![CandidateValidationIssue::ReferenceNotInPlan {
            role,
            referenced: object_ref,
        }]
    }
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

        for (index, candidate) in plan.candidates.into_iter().enumerate() {
            let kind = candidate.kind();
            let timestamp_error = |field| CustomError::WritePlanValidationRejected {
                validations: vec![CandidateValidation::invalid(
                    index,
                    kind,
                    CandidateValidationIssue::MissingTimestamp { field },
                )],
            };
            match candidate {
                MemoryCandidate::Episode(candidate) => objects.push(MemoryObject::Episode(
                    require_created_at(candidate.draft)
                        .map_err(timestamp_error)?
                        .into_domain_with_defaults(&mut defaults)?,
                )),
                MemoryCandidate::Observation(candidate) => objects.push(MemoryObject::Observation(
                    require_created_at(candidate.draft)
                        .map_err(timestamp_error)?
                        .into_domain_with_defaults(&mut defaults)?,
                )),
                MemoryCandidate::Entity(candidate) => objects.push(MemoryObject::Entity(
                    require_created_at(candidate.draft)
                        .map_err(timestamp_error)?
                        .into_domain_with_defaults(&mut defaults)?,
                )),
                MemoryCandidate::MemoryThread(candidate) => {
                    objects.push(MemoryObject::MemoryThread(
                        stable_memory_thread_draft(candidate.draft)
                            .map_err(timestamp_error)?
                            .into_domain_with_defaults(&mut defaults)?,
                    ));
                }
                MemoryCandidate::DerivedMemory(candidate) => {
                    objects.push(MemoryObject::DerivedMemory(
                        require_created_and_updated_at(candidate.draft)
                            .map_err(timestamp_error)?
                            .into_domain_with_defaults(&mut defaults)?,
                    ));
                }
                MemoryCandidate::MemoryLink(candidate) => {
                    links.push(
                        stable_memory_link_draft(candidate.draft)
                            .map_err(timestamp_error)?
                            .into_domain_with_defaults(&mut defaults)?,
                    );
                }
                MemoryCandidate::VectorIndex(candidate) => vector_targets.push(candidate.target),
                MemoryCandidate::StatsUpdate(_) => {}
            }
        }

        for object in &objects {
            match object {
                MemoryObject::DerivedMemory(memory) => links.extend(derived_memory_links(memory)),
                MemoryObject::Observation(observation) => {
                    if !links.iter().any(|link| {
                        let from = MemoryObjectRef::new(link.from_type, link.from_id);
                        let to = MemoryObjectRef::new(link.to_type, link.to_id);
                        let observation_ref = object.object_ref();
                        let episode_ref =
                            MemoryObjectRef::new(ObjectType::Episode, observation.episode_id);
                        link.relation == RelationType::ObservedIn
                            && ((from == observation_ref && to == episode_ref)
                                || (to == observation_ref && from == episode_ref))
                    }) {
                        links.push(MemoryLink {
                            id: deterministic_uuid(&[
                                b"character_memory.observation.episode_link",
                                observation.id.as_bytes(),
                                observation.episode_id.as_bytes(),
                            ]),
                            object_type: ObjectType::MemoryLink,
                            from_id: observation.id,
                            from_type: ObjectType::Observation,
                            to_id: observation.episode_id,
                            to_type: ObjectType::Episode,
                            relation: RelationType::ObservedIn,
                            rationale: None,
                            created_at: observation.created_at,
                            schema_version: observation.schema_version.clone(),
                        });
                    }
                }
                MemoryObject::Episode(episode) => {
                    let mut linked = links
                        .iter()
                        .filter(|link| link.relation == RelationType::Involves)
                        .filter_map(|link| {
                            let from = MemoryObjectRef::new(link.from_type, link.from_id);
                            let to = MemoryObjectRef::new(link.to_type, link.to_id);
                            if from == object.object_ref() && to.object_type == ObjectType::Entity {
                                Some(to.id)
                            } else if to == object.object_ref()
                                && from.object_type == ObjectType::Entity
                            {
                                Some(from.id)
                            } else {
                                None
                            }
                        })
                        .collect::<HashSet<_>>();
                    for participant in episode
                        .scene
                        .participant_keys()
                        .filter(|id| linked.insert(*id))
                    {
                        links.push(MemoryLink {
                            id: deterministic_uuid(&[
                                b"character_memory.scene.participant_link",
                                episode.id.as_bytes(),
                                participant.as_bytes(),
                            ]),
                            object_type: ObjectType::MemoryLink,
                            from_id: episode.id,
                            from_type: ObjectType::Episode,
                            to_id: participant,
                            to_type: ObjectType::Entity,
                            relation: RelationType::Involves,
                            rationale: None,
                            created_at: episode.created_at,
                            schema_version: episode.schema_version.clone(),
                        });
                    }
                }
                _ => {}
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

impl CandidateCreatedAt for EpisodeDraft {
    fn created_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.created_at
    }
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

fn require_created_at<T>(draft: T) -> Result<T, CandidateTimestampField>
where
    T: CandidateCreatedAt,
{
    if draft.created_at().is_none() {
        return Err(CandidateTimestampField::CreatedAt);
    }
    Ok(draft)
}

fn require_created_and_updated_at<T>(draft: T) -> Result<T, CandidateTimestampField>
where
    T: CandidateUpdatedAt,
{
    require_created_at(draft).and_then(|draft| {
        if draft.updated_at().is_none() {
            return Err(CandidateTimestampField::UpdatedAt);
        }
        Ok(draft)
    })
}

fn stable_memory_thread_draft(
    draft: MemoryThreadDraft,
) -> Result<MemoryThreadDraft, CandidateTimestampField> {
    if draft.created_at.is_none() {
        return Err(CandidateTimestampField::CreatedAt);
    }
    if draft.updated_at.is_none() {
        return Err(CandidateTimestampField::UpdatedAt);
    }
    if draft.last_touched_at.is_none() {
        return Err(CandidateTimestampField::LastTouchedAt);
    }
    Ok(draft)
}

fn stable_memory_link_draft(
    draft: MemoryLinkDraft,
) -> Result<MemoryLinkDraft, CandidateTimestampField> {
    if draft.created_at.is_none() {
        return Err(CandidateTimestampField::CreatedAt);
    }
    Ok(draft)
}

fn validate_episode_timestamps(draft: &EpisodeDraft) -> Vec<CandidateValidationIssue> {
    validate_required_created_at("episode candidate", draft.created_at)
}

fn validate_memory_thread_timestamps(draft: &MemoryThreadDraft) -> Vec<CandidateValidationIssue> {
    let mut errors = validate_required_created_and_updated_at(
        "memory thread candidate",
        draft.created_at,
        draft.updated_at,
    );
    if draft.last_touched_at.is_none() {
        errors.push(CandidateValidationIssue::MissingTimestamp {
            field: CandidateTimestampField::LastTouchedAt,
        });
    }
    errors
}

fn validate_memory_link_timestamps(draft: &MemoryLinkDraft) -> Vec<CandidateValidationIssue> {
    validate_required_created_at("memory link candidate", draft.created_at)
}

fn validate_required_created_at(
    _label: &str,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Vec<CandidateValidationIssue> {
    if created_at.is_none() {
        return vec![CandidateValidationIssue::MissingTimestamp {
            field: CandidateTimestampField::CreatedAt,
        }];
    }
    Vec::new()
}

fn validate_required_created_and_updated_at(
    label: &str,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Vec<CandidateValidationIssue> {
    let mut errors = validate_required_created_at(label, created_at);
    if updated_at.is_none() {
        errors.push(CandidateValidationIssue::MissingTimestamp {
            field: CandidateTimestampField::UpdatedAt,
        });
    }
    errors
}

fn validate_object(object: &MemoryObject) -> Vec<CandidateValidationIssue> {
    let mut errors = Vec::new();
    if let Err(error) = object.validate() {
        errors.push(candidate_issue_from_domain_error(error));
    }
    if schema_version(object).trim().is_empty() {
        errors.push(CandidateValidationIssue::MissingObjectSchemaVersion);
    }
    errors
}

fn validate_required_candidate_identity(
    _label: &str,
    id: Option<MemoryId>,
    schema_version: Option<&str>,
) -> Vec<CandidateValidationIssue> {
    let mut errors = Vec::new();
    if id.is_none_or(|id| id.is_nil()) {
        errors.push(CandidateValidationIssue::MissingCandidateId);
    }
    if schema_version.is_none_or(|value| value.trim().is_empty()) {
        errors.push(CandidateValidationIssue::MissingCandidateSchemaVersion);
    }
    errors
}

fn validate_link(link: &MemoryLink) -> Vec<CandidateValidationIssue> {
    let mut errors = Vec::new();
    if let Err(error) = link.validate() {
        errors.push(candidate_issue_from_domain_error(error));
    }
    if link.schema_version.trim().is_empty() {
        errors.push(CandidateValidationIssue::MissingObjectSchemaVersion);
    }
    errors
}

fn validate_provenance(
    provenance: &crate::api::types::CandidateProvenance,
) -> Vec<CandidateValidationIssue> {
    let mut errors = Vec::new();
    if provenance.producer_kind != CandidateProducerKind::Caller
        && matches!(
            provenance.rationale,
            CandidateRationale::ProvidedByCaller(_)
        )
    {
        errors.push(CandidateValidationIssue::InvalidProvenance {
            reason: CandidateProvenanceIssue::NonCallerClaimedCallerRationale,
        });
    }
    if provenance
        .rationale
        .text()
        .is_some_and(|text| text.trim().is_empty())
    {
        errors.push(CandidateValidationIssue::InvalidProvenance {
            reason: CandidateProvenanceIssue::EmptyRationaleText,
        });
    }
    for source_span in &provenance.source.source_spans {
        if let Err(error) = source_span.validate() {
            errors.push(CandidateValidationIssue::InvalidSourceSpan {
                reason: candidate_source_span_issue(error),
            });
        }
    }
    for external_ref in &provenance.source.external_refs {
        if !external_ref.has_reference() {
            errors.push(CandidateValidationIssue::InvalidProvenance {
                reason: CandidateProvenanceIssue::EmptyExternalReference,
            });
        }
    }
    errors
}

fn candidate_issue_from_domain_error(error: DomainValidationError) -> CandidateValidationIssue {
    match error {
        DomainValidationError::ObjectTypeMismatch {
            expected, actual, ..
        } => CandidateValidationIssue::ObjectTypeMismatch { expected, actual },
        DomainValidationError::AuthoredSupersedesLink => {
            CandidateValidationIssue::AuthoredSupersedesLink
        }
        DomainValidationError::AuthoredBeliefAboutLink => {
            CandidateValidationIssue::AuthoredBeliefAboutLink
        }
        DomainValidationError::EmptyEpisodeSummary => CandidateValidationIssue::EmptyEpisodeSummary,
        DomainValidationError::MissingScene => CandidateValidationIssue::MissingScene,
        DomainValidationError::InvalidSceneTimeOffset { offset_seconds } => {
            CandidateValidationIssue::InvalidSceneTimeOffset { offset_seconds }
        }
        DomainValidationError::MissingEpisodeReference => {
            CandidateValidationIssue::MissingEpisodeReference
        }
        DomainValidationError::InvalidBelief(reason) => {
            CandidateValidationIssue::InvalidBelief { reason }
        }
        DomainValidationError::MissingDerivedSource => {
            CandidateValidationIssue::MissingDerivedSource
        }
        DomainValidationError::InvalidScore { field, value } => {
            CandidateValidationIssue::InvalidScore {
                field: candidate_score_field(field),
                actual: value.to_string(),
            }
        }
        DomainValidationError::UnsupportedMemoryLinkEndpoint { field } => {
            CandidateValidationIssue::UnsupportedMemoryLinkEndpoint {
                endpoint: match field {
                    "MemoryLink.from_type" => MemoryLinkEndpoint::From,
                    "MemoryLink.to_type" => MemoryLinkEndpoint::To,
                    _ => unreachable!("unrecognized memory-link endpoint field: {field}"),
                },
            }
        }
        DomainValidationError::SelfLink { object_type, id } => CandidateValidationIssue::SelfLink {
            referenced: MemoryObjectRef::new(object_type, id),
        },
    }
}

fn candidate_score_field(field: &'static str) -> CandidateScoreField {
    match field {
        "Episode.salience_score" => CandidateScoreField::EpisodeSalience,
        "Observation.salience_score" => CandidateScoreField::ObservationSalience,
        "MemoryThread.salience_score" => CandidateScoreField::MemoryThreadSalience,
        "DerivedMemory.salience_score" => CandidateScoreField::DerivedMemorySalience,
        _ => unreachable!("unrecognized candidate score field: {field}"),
    }
}

fn candidate_source_span_issue(error: SourceSpanValidationError) -> CandidateSourceSpanIssue {
    match error {
        SourceSpanValidationError::EmptySourceRef => CandidateSourceSpanIssue::EmptySourceRef,
        SourceSpanValidationError::EmptyRawRef => CandidateSourceSpanIssue::EmptyRawRef,
        SourceSpanValidationError::EmptyMessageId => CandidateSourceSpanIssue::EmptyMessageId,
        SourceSpanValidationError::EmptyTranscriptSegmentId => {
            CandidateSourceSpanIssue::EmptyTranscriptSegmentId
        }
        SourceSpanValidationError::InvalidTurnRange => CandidateSourceSpanIssue::InvalidTurnRange,
        SourceSpanValidationError::InvalidCharRange => CandidateSourceSpanIssue::InvalidCharRange,
        SourceSpanValidationError::InvalidByteRange => CandidateSourceSpanIssue::InvalidByteRange,
        SourceSpanValidationError::InvalidTimestampRange => {
            CandidateSourceSpanIssue::InvalidTimestampRange
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::parse_id as id;
    use chrono::{DateTime, Utc};

    use super::RememberPlanDefaults;
    use crate::api::types::{
        CandidateProvenance, CandidateRationale, CommitOptions, DerivedMemoryDraft, EntityDraft,
        EpisodeDraft, MemoryLinkDraft, RememberInput, RememberOutcome, SourceSpan,
        StatsUpdateCandidate, StatsUpdateStatus, VectorIndexCandidate,
    };
    use crate::domain::{
        DerivedType, MemoryCandidateKind, MemoryObject, RelationType, RetentionState,
        DEFAULT_SCHEMA_VERSION,
    };
    use crate::test_support::{
        deterministic_embedder, in_memory_graph_store, representative_fixtures,
        TemporaryVectorCandidateStore,
    };
    use crate::usecases::RememberPipeline;

    #[tokio::test]
    async fn accepts_derived_source_references_to_candidates_declared_later() {
        let graph = in_memory_graph_store();
        let mut plan = RememberInput::new("later source episode")
            .with_derived_memory(DerivedMemoryDraft::new(
                DerivedType::Reflection,
                "reflection on the later source",
            ))
            .prepare_write_plan_with_options(&defaults(), false, false);
        plan.candidates.reverse();

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_eq!(verdict.decision, WritePlanValidationDecision::Accepted);
        assert_eq!(
            verdict.validations[0].candidate_kind,
            MemoryCandidateKind::DerivedMemory
        );
        assert_eq!(
            verdict.validations.last().unwrap().candidate_kind,
            MemoryCandidateKind::Episode
        );
        assert!(verdict.validations.iter().all(|validation| {
            validation.status == CandidateValidationStatus::Valid && validation.errors.is_empty()
        }));
    }

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

        let query = GraphObjectQuery::by_types(
            vec![
                ObjectType::Episode,
                ObjectType::Observation,
                ObjectType::Entity,
                ObjectType::MemoryThread,
                ObjectType::DerivedMemory,
            ],
            None,
        );
        let link_ids = fixtures
            .links()
            .iter()
            .map(|link| link.id)
            .chain(
                plan.candidates
                    .iter()
                    .filter_map(|candidate| match candidate {
                        MemoryCandidate::MemoryLink(candidate) => candidate.draft.id,
                        _ => None,
                    }),
            )
            .collect::<Vec<_>>();
        let objects_before = graph.query_objects(&query).await.unwrap();
        let links_before = graph.query_links_by_ids(&link_ids).await.unwrap();

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert!(verdict.is_valid());
        assert_eq!(graph.query_objects(&query).await.unwrap(), objects_before);
        assert_eq!(
            graph.query_links_by_ids(&link_ids).await.unwrap(),
            links_before
        );
    }

    #[tokio::test]
    async fn warns_when_observation_content_echoes_source_episode_candidate() {
        let graph = in_memory_graph_store();
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
        assert_eq!(
            validation.warnings,
            vec![CandidateValidationIssue::DuplicateObservationEcho {
                echo_surface: "valid minimal plan".to_owned(),
                matching_episode_ids: vec![source_episode_id],
            }]
        );
    }

    #[tokio::test]
    async fn warns_when_derived_memory_content_echoes_source_episode_candidate() {
        let graph = in_memory_graph_store();
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
        assert_eq!(
            validation.warnings,
            vec![CandidateValidationIssue::DuplicateObservationEcho {
                echo_surface: "source episode content".to_owned(),
                matching_episode_ids: vec![source_episode_id],
            }]
        );
    }

    #[tokio::test]
    async fn does_not_warn_for_distinct_observation_and_derived_surfaces() {
        let graph = in_memory_graph_store();
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
    async fn rejects_missing_schema_version() {
        let graph = in_memory_graph_store();
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

        assert_rejected_with(
            &verdict,
            CandidateValidationIssue::MissingCandidateSchemaVersion,
        );
    }

    #[tokio::test]
    async fn rejects_missing_candidate_id() {
        let graph = in_memory_graph_store();
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

        assert_rejected_with(&verdict, CandidateValidationIssue::MissingCandidateId);
    }

    #[tokio::test]
    async fn rejects_ungrounded_derived_memory_provenance() {
        let graph = in_memory_graph_store();
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

        assert_rejected_with(
            &verdict,
            CandidateValidationIssue::UnknownObjectRef {
                role: CandidateReferenceRole::DerivedSourceEpisode,
                referenced: MemoryObjectRef::new(
                    ObjectType::Episode,
                    id("550e8400-e29b-41d4-a716-446655445010"),
                ),
            },
        );
    }

    #[tokio::test]
    async fn missing_derived_source_is_rejected_at_validate_and_commit() {
        let graph = in_memory_graph_store();
        let derived = DerivedMemoryDraft::new(DerivedType::UserPreference, "ungrounded preference");
        let plan = RememberWritePlan::new().with_candidate(MemoryCandidate::DerivedMemory(
            crate::api::types::DerivedMemoryCandidate::new(
                complete_derived(derived),
                CandidateProvenance::caller("caller omitted source provenance"),
            ),
        ));
        let expected = vec![CandidateValidation::invalid(
            0,
            MemoryCandidateKind::DerivedMemory,
            CandidateValidationIssue::MissingDerivedSource,
        )];

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();
        assert_eq!(verdict.decision, WritePlanValidationDecision::Rejected);
        assert_eq!(verdict.validations, expected);

        let vector = TemporaryVectorCandidateStore::open(8).await;
        let embedder = deterministic_embedder(8);
        let error = RememberPipeline::new(&graph, &vector, &embedder)
            .commit(plan, CommitOptions::default(), &tokio::sync::Mutex::new(()))
            .await
            .expect_err("commit must reject an ungrounded preference");
        let CustomError::WritePlanValidationRejected { validations } = error else {
            panic!("expected structured validation rejection, got {error:?}");
        };
        assert_eq!(validations, expected);
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
        let graph = in_memory_graph_store();
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

        assert_rejected_with(
            &verdict,
            CandidateValidationIssue::UnknownObjectRef {
                role: CandidateReferenceRole::MemoryLinkFrom,
                referenced: MemoryObjectRef::new(
                    ObjectType::Entity,
                    id("550e8400-e29b-41d4-a716-446655445020"),
                ),
            },
        );
        assert_rejected_with(
            &verdict,
            CandidateValidationIssue::UnknownObjectRef {
                role: CandidateReferenceRole::MemoryLinkTo,
                referenced: MemoryObjectRef::new(
                    ObjectType::Episode,
                    id("550e8400-e29b-41d4-a716-446655445021"),
                ),
            },
        );
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

        assert_rejected_with(
            &verdict,
            CandidateValidationIssue::SelfLink {
                referenced: MemoryObjectRef::new(ObjectType::Episode, fixtures.episode.id),
            },
        );
    }

    #[tokio::test]
    async fn rejects_memory_link_endpoint_types() {
        let graph = in_memory_graph_store();
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

        assert_rejected_with(
            &verdict,
            CandidateValidationIssue::UnsupportedMemoryLinkEndpoint {
                endpoint: MemoryLinkEndpoint::From,
            },
        );
    }

    #[tokio::test]
    async fn supersession_requires_derived_predecessors_and_rejects_authored_links() {
        let graph = graph_with_fixtures().await;
        let fixtures = representative_fixtures();
        for predecessor in [MemoryId::from_u128(991), fixtures.episode.id] {
            let mut draft = complete_derived(
                DerivedMemoryDraft::new(DerivedType::Correction, "corrected belief")
                    .with_source_episode(fixtures.episode.id),
            );
            draft.supersedes = vec![predecessor];
            let plan = valid_plan().with_candidate(MemoryCandidate::DerivedMemory(
                crate::api::types::DerivedMemoryCandidate::new(
                    draft,
                    CandidateProvenance::caller("correction"),
                ),
            ));
            let verdict = WritePlanValidator::new(&graph)
                .validate(&plan)
                .await
                .unwrap();
            assert_rejected_with(
                &verdict,
                CandidateValidationIssue::UnknownObjectRef {
                    role: CandidateReferenceRole::SupersededMemory,
                    referenced: MemoryObjectRef::new(ObjectType::DerivedMemory, predecessor),
                },
            );
        }
        let mut self_replacing = complete_derived(
            DerivedMemoryDraft::new(DerivedType::Correction, "self replacement")
                .with_source_episode(fixtures.episode.id),
        );
        let self_id = self_replacing.id.unwrap();
        self_replacing.supersedes = vec![self_id];
        let plan = valid_plan().with_candidate(MemoryCandidate::DerivedMemory(
            crate::api::types::DerivedMemoryCandidate::new(
                self_replacing,
                CandidateProvenance::caller("invalid self replacement"),
            ),
        ));
        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();
        assert_rejected_with(
            &verdict,
            CandidateValidationIssue::SelfLink {
                referenced: MemoryObjectRef::new(ObjectType::DerivedMemory, self_id),
            },
        );
        let mut draft = MemoryLinkDraft::new(
            ObjectType::DerivedMemory,
            fixtures.correction.id,
            RelationType::Supersedes,
            ObjectType::DerivedMemory,
            fixtures.user_preference.id,
        );
        draft.id = Some(MemoryId::from_u128(992));
        draft.created_at = Some(timestamp());
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        let plan = valid_plan().with_candidate(MemoryCandidate::MemoryLink(
            crate::api::types::MemoryLinkCandidate::new(
                draft,
                CandidateProvenance::caller("authored link"),
            ),
        ));
        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();
        assert_rejected_with(&verdict, CandidateValidationIssue::AuthoredSupersedesLink);
    }

    #[tokio::test]
    async fn accepts_suppressed_derived_memory() {
        let graph = graph_with_fixtures().await;
        let fixtures = representative_fixtures();
        let mut derived = DerivedMemoryDraft::new(DerivedType::Reflection, "suppressed current")
            .with_source_episode(fixtures.episode.id);
        derived.retention_state = RetentionState::Suppressed;
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

        assert!(verdict.is_valid());
    }

    #[tokio::test]
    async fn accepts_active_successor_with_existing_predecessor() {
        let graph = graph_with_fixtures().await;
        let fixtures = representative_fixtures();
        let mut derived = DerivedMemoryDraft::new(DerivedType::Correction, "new correction")
            .with_source_episode(fixtures.episode.id);
        derived.supersedes.push(fixtures.user_preference.id);
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

        assert!(verdict.is_valid());
    }

    #[tokio::test]
    async fn rejects_vector_index_for_missing_graph_object() {
        let graph = in_memory_graph_store();
        let plan =
            valid_plan().with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                MemoryObjectRef::new(
                    ObjectType::Episode,
                    id("550e8400-e29b-41d4-a716-446655445050"),
                ),
                CandidateProvenance::caller("caller supplied vector candidate"),
            )));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(
            &verdict,
            CandidateValidationIssue::UnknownObjectRef {
                role: CandidateReferenceRole::VectorIndexTarget,
                referenced: MemoryObjectRef::new(
                    ObjectType::Episode,
                    id("550e8400-e29b-41d4-a716-446655445050"),
                ),
            },
        );
    }

    #[tokio::test]
    async fn rejects_vector_index_for_graph_only_object() {
        let graph = graph_with_fixtures().await;
        let fixtures = representative_fixtures();
        let plan =
            valid_plan().with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                MemoryObjectRef::new(ObjectType::Episode, fixtures.episode.id),
                CandidateProvenance::caller("caller supplied vector candidate"),
            )));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert_rejected_with(
            &verdict,
            CandidateValidationIssue::ReferenceNotInPlan {
                role: CandidateReferenceRole::VectorIndexTarget,
                referenced: MemoryObjectRef::new(ObjectType::Episode, fixtures.episode.id),
            },
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
            CandidateValidationIssue::ReferenceNotInPlan {
                role: CandidateReferenceRole::StatsUpdateSubject,
                referenced: MemoryObjectRef::new(ObjectType::Episode, fixtures.episode.id),
            },
        );
    }

    #[tokio::test]
    async fn rejects_missing_candidate_timestamps() {
        let graph = in_memory_graph_store();
        let mut draft = EntityDraft::new();
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

        assert_rejected_with(
            &verdict,
            CandidateValidationIssue::MissingTimestamp {
                field: CandidateTimestampField::CreatedAt,
            },
        );
    }

    #[test]
    fn commit_values_reject_missing_timestamps_before_defaults() {
        let mut draft = EpisodeDraft::new("missing timestamp defense");
        draft.id = Some(id("550e8400-e29b-41d4-a716-446655445056"));
        draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        let plan = RememberWritePlan::new().with_candidate(MemoryCandidate::Episode(
            crate::api::types::EpisodeCandidate::new(
                draft,
                CandidateProvenance::caller("caller supplied episode"),
            ),
        ));

        let error = match WritePlanCommitValues::from_plan(plan) {
            Ok(_) => panic!("missing timestamp plan should reject before defaults"),
            Err(error) => error,
        };

        let CustomError::WritePlanValidationRejected { validations } = error else {
            panic!("expected typed timestamp rejection");
        };
        assert_eq!(
            validations,
            vec![CandidateValidation::invalid(
                0,
                MemoryCandidateKind::Episode,
                CandidateValidationIssue::MissingTimestamp {
                    field: CandidateTimestampField::CreatedAt
                },
            )]
        );
    }

    #[tokio::test]
    async fn rejects_stats_update_for_missing_graph_object() {
        let graph = in_memory_graph_store();
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

        assert_rejected_with(
            &verdict,
            CandidateValidationIssue::UnknownObjectRef {
                role: CandidateReferenceRole::StatsUpdateSubject,
                referenced: MemoryObjectRef::new(
                    ObjectType::Episode,
                    id("550e8400-e29b-41d4-a716-446655445060"),
                ),
            },
        );
    }

    #[tokio::test]
    async fn rejects_invalid_source_span() {
        let graph = in_memory_graph_store();
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
            CandidateValidationIssue::InvalidSourceSpan {
                reason: CandidateSourceSpanIssue::InvalidCharRange,
            },
        );
    }

    #[tokio::test]
    async fn rejects_producer_rationale_origin_conflation() {
        let graph = in_memory_graph_store();
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

        assert_rejected_with(
            &verdict,
            CandidateValidationIssue::InvalidProvenance {
                reason: CandidateProvenanceIssue::NonCallerClaimedCallerRationale,
            },
        );
    }

    #[tokio::test]
    async fn raw_ref_is_validated_only_as_opaque_structure() {
        let graph = in_memory_graph_store();
        let plan = RememberInput::new("opaque raw ref")
            .with_raw_ref("raw://does/not/need/to/exist")
            .prepare_write_plan_with_options(&defaults(), false, false);

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();

        assert!(verdict.is_valid());
    }

    #[tokio::test]
    async fn processor_origin_plan_validates_and_commits_through_pipeline() {
        let graph = in_memory_graph_store();
        let entity_id = id("550e8400-e29b-41d4-a716-446655613301");
        let episode_id = id("550e8400-e29b-41d4-a716-446655613302");
        let derived_id = id("550e8400-e29b-41d4-a716-446655613303");
        let link_id = id("550e8400-e29b-41d4-a716-446655613305");
        let mut entity = EntityDraft::new();
        entity.id = Some(entity_id);
        entity.created_at = Some(timestamp());
        entity.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        let mut episode = complete_episode(EpisodeDraft::new("generated-style source episode"));
        episode.id = Some(episode_id);
        let mut derived = complete_derived(DerivedMemoryDraft::new(
            DerivedType::Claim,
            "generated-style derived memory",
        ));
        derived.id = Some(derived_id);
        derived.derived_from_episode_ids.push(episode_id);
        derived.entity_ids.push(entity_id);
        let mut link = link_draft(entity_id, episode_id);
        link.id = Some(link_id);
        let processor = |rationale: &str| {
            CandidateProvenance::inferred_by_processor(
                CandidateProducerKind::ModelProcessor,
                rationale,
            )
        };
        let plan = RememberWritePlan::new()
            .with_candidate(MemoryCandidate::Entity(
                crate::api::types::EntityCandidate::new(entity, processor("generated entity")),
            ))
            .with_candidate(MemoryCandidate::Episode(
                crate::api::types::EpisodeCandidate::new(episode, processor("generated episode")),
            ))
            .with_candidate(MemoryCandidate::DerivedMemory(
                crate::api::types::DerivedMemoryCandidate::new(
                    derived,
                    processor("generated derived memory").with_source_episode(episode_id),
                ),
            ))
            .with_candidate(MemoryCandidate::MemoryLink(
                crate::api::types::MemoryLinkCandidate::new(link, processor("generated link")),
            ));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();
        assert_eq!(verdict.decision, WritePlanValidationDecision::Accepted);
        assert_eq!(
            verdict.validations,
            vec![
                CandidateValidation::valid(0, MemoryCandidateKind::Entity),
                CandidateValidation::valid(1, MemoryCandidateKind::Episode),
                CandidateValidation::valid(2, MemoryCandidateKind::DerivedMemory),
                CandidateValidation::valid(3, MemoryCandidateKind::MemoryLink),
            ]
        );

        let vector = TemporaryVectorCandidateStore::open(8).await;
        let embedder = deterministic_embedder(8);
        let outcome = RememberPipeline::new(&graph, &vector, &embedder)
            .commit(
                plan,
                graph_only_commit_options(),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .unwrap();
        assert_eq!(
            outcome.persisted_object_ids,
            vec![entity_id, episode_id, derived_id]
        );
        assert_eq!(outcome.persisted_link_ids.len(), 2);
        assert!(outcome.persisted_link_ids.contains(&link_id));
        let persisted_links = graph
            .query_links_by_ids(&outcome.persisted_link_ids)
            .await
            .unwrap();
        assert!(persisted_links.iter().any(|link| link.from_id == derived_id
            && link.to_id == entity_id
            && link.relation == RelationType::About));
        assert_graph_only_outcome(&outcome);

        let objects = graph
            .query_objects(&GraphObjectQuery::by_ids(vec![
                entity_id, episode_id, derived_id,
            ]))
            .await
            .unwrap();
        let mut object_refs = objects
            .iter()
            .map(MemoryObject::object_ref)
            .collect::<Vec<_>>();
        object_refs.sort_by_key(|object_ref| object_ref.id);
        let mut expected_refs = vec![
            MemoryObjectRef::new(ObjectType::Entity, entity_id),
            MemoryObjectRef::new(ObjectType::Episode, episode_id),
            MemoryObjectRef::new(ObjectType::DerivedMemory, derived_id),
        ];
        expected_refs.sort_by_key(|object_ref| object_ref.id);
        assert_eq!(object_refs, expected_refs);
        let links = graph.query_links_by_ids(&[link_id]).await.unwrap();
        assert_eq!(
            links
                .iter()
                .map(|link| (link.id, link.from_id, link.relation, link.to_id))
                .collect::<Vec<_>>(),
            vec![(link_id, entity_id, RelationType::Involves, episode_id)]
        );
    }

    #[tokio::test]
    async fn valid_source_span_validates_and_commits_with_raw_ref_preserved() {
        let graph = in_memory_graph_store();
        let raw_ref = "raw://opaque/source-refs";
        let span = SourceSpan::raw(raw_ref)
            .with_message_id("message-7")
            .with_char_range(3, 31);
        let plan = RememberInput::new("source-preserved episode")
            .with_observation(ObservationDraft::new(
                MemoryId::nil(),
                "distinct observation content",
            ))
            .with_raw_ref(raw_ref)
            .with_source_span(span.clone())
            .prepare_write_plan_with_options(&defaults(), false, false);
        let episode_id = defaults().stable_id("episode:0");
        let observation_id = defaults().stable_id("observation:0");
        assert_eq!(
            plan.source_input_ref,
            Some(ExternalSourceReference::raw(raw_ref))
        );
        assert!(plan.candidates.iter().any(|candidate| matches!(
            candidate,
            MemoryCandidate::Episode(candidate)
                if candidate.provenance.source.source_spans == vec![span.clone()]
        )));

        let verdict = WritePlanValidator::new(&graph)
            .validate(&plan)
            .await
            .unwrap();
        assert_eq!(verdict.decision, WritePlanValidationDecision::Accepted);
        assert_eq!(
            verdict.validations,
            vec![
                CandidateValidation::valid(0, MemoryCandidateKind::Episode),
                CandidateValidation::valid(1, MemoryCandidateKind::Observation),
            ]
        );

        let vector = TemporaryVectorCandidateStore::open(8).await;
        let embedder = deterministic_embedder(8);
        let outcome = RememberPipeline::new(&graph, &vector, &embedder)
            .commit(
                plan,
                graph_only_commit_options(),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .unwrap();
        assert_eq!(
            outcome.persisted_object_ids,
            vec![episode_id, observation_id]
        );
        assert_graph_only_outcome(&outcome);

        let objects = graph
            .query_objects(&GraphObjectQuery::by_ids(vec![episode_id, observation_id]))
            .await
            .unwrap();
        let mut persisted_raw_refs = objects
            .iter()
            .map(|object| match object {
                MemoryObject::Episode(episode) => (episode.id, episode.raw_ref.as_deref()),
                MemoryObject::Observation(observation) => {
                    (observation.id, observation.raw_ref.as_deref())
                }
                other => panic!("unexpected committed object {other:?}"),
            })
            .collect::<Vec<_>>();
        persisted_raw_refs.sort();
        let mut expected_raw_refs =
            vec![(episode_id, Some(raw_ref)), (observation_id, Some(raw_ref))];
        expected_raw_refs.sort();
        assert_eq!(persisted_raw_refs, expected_raw_refs);
    }

    fn graph_only_commit_options() -> CommitOptions {
        CommitOptions {
            update_vectors: false,
            update_stats: false,
        }
    }

    fn assert_graph_only_outcome(outcome: &RememberOutcome) {
        assert!(!outcome.persisted_object_ids.is_empty());
        assert_eq!(outcome.vector_indexed_object_ids, Vec::<MemoryId>::new());
        assert_eq!(outcome.vector_indexing_failure, None);
        assert_eq!(outcome.stats_update_status, StatsUpdateStatus::default());
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
            .scene
            .get_or_insert_with(|| Scene::at((timestamp()).fixed_offset()));
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

    async fn graph_with_fixtures() -> crate::adapters::oxigraph::OxigraphGraphAuthorityStore {
        let graph = in_memory_graph_store();
        let fixtures = representative_fixtures();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        graph
    }

    fn assert_rejected_with(
        verdict: &WritePlanValidationVerdict,
        expected: CandidateValidationIssue,
    ) {
        assert_eq!(verdict.decision, WritePlanValidationDecision::Rejected);
        assert!(
            verdict
                .validations
                .iter()
                .flat_map(|validation| validation.errors.iter())
                .any(|error| error == &expected),
            "expected issue {expected:?}, got {:?}",
            verdict.validations
        );
    }

    fn timestamp() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-07-03T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }
}
