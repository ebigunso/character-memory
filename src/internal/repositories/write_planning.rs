#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use crate::api::types::{
    CandidateProducerKind, CandidateRationale, CandidateValidation, CandidateValidationStatus,
    DraftDefaults, MemoryCandidate, MemoryId, MemoryLink, MemoryObject, MemoryObjectRef,
    ObjectType, RememberWritePlan, RetentionState,
};
use crate::errors::CustomError;
use crate::internal::repositories::{
    admit_link, GraphAuthorityStore, GraphObjectQuery, GraphObjectRef, LinkAdmissionDecision,
    LinkAdmissionEvidence,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WritePlanValidationVerdict {
    pub(crate) validations: Vec<CandidateValidation>,
    pub(crate) decision: WritePlanValidationDecision,
}

impl WritePlanValidationVerdict {
    pub(crate) fn is_valid(&self) -> bool {
        self.decision == WritePlanValidationDecision::Accepted
    }

    #[allow(dead_code)]
    pub(crate) fn into_result(self) -> Result<Self, CustomError> {
        if self.is_valid() {
            Ok(self)
        } else {
            let errors = self
                .validations
                .iter()
                .flat_map(|validation| validation.errors.iter())
                .cloned()
                .collect::<Vec<_>>()
                .join("; ");
            Err(validation_error(format!("write plan rejected: {errors}")))
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
    prior_plan_fingerprints: HashMap<String, String>,
}

impl<'a, G> WritePlanValidator<'a, G>
where
    G: GraphAuthorityStore + ?Sized,
{
    pub(crate) fn new(graph_store: &'a G) -> Self {
        Self {
            graph_store,
            prior_plan_fingerprints: HashMap::new(),
        }
    }

    pub(crate) fn with_prior_plan_fingerprint(
        mut self,
        idempotency_key: impl Into<String>,
        fingerprint: impl Into<String>,
    ) -> Self {
        self.prior_plan_fingerprints
            .insert(idempotency_key.into(), fingerprint.into());
        self
    }

    pub(crate) async fn validate(
        &self,
        plan: &RememberWritePlan,
    ) -> Result<WritePlanValidationVerdict, CustomError> {
        let mut context = PlanValidationContext::new(plan, &self.prior_plan_fingerprints)?;
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
    plan_refs: HashSet<GraphObjectRef>,
    refs_requiring_graph: HashSet<GraphObjectRef>,
    existing_refs: HashSet<GraphObjectRef>,
    plan_errors: Vec<String>,
}

impl PlanValidationContext {
    fn new(
        plan: &RememberWritePlan,
        prior_plan_fingerprints: &HashMap<String, String>,
    ) -> Result<Self, CustomError> {
        let mut context = Self {
            plan_refs: HashSet::new(),
            refs_requiring_graph: HashSet::new(),
            existing_refs: HashSet::new(),
            plan_errors: validate_plan_identity(plan, prior_plan_fingerprints),
        };

        for candidate in &plan.candidates {
            context.collect_plan_ref(candidate);
            context.collect_referenced_refs(candidate);
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
                if let Some(object) = candidate.object {
                    errors.extend(self.validate_graph_authoritative_ref(
                        object.into(),
                        "stats update candidate object",
                    ));
                }
            }
        }

        if errors.is_empty() {
            CandidateValidation::valid(index, candidate.kind())
        } else {
            let mut validation =
                CandidateValidation::invalid(index, candidate.kind(), errors[0].clone());
            validation.errors.extend(errors.into_iter().skip(1));
            validation
        }
    }

    fn validate_derived_sources(&self, object: &crate::api::types::DerivedMemory) -> Vec<String> {
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
}

fn validate_plan_identity(
    plan: &RememberWritePlan,
    prior_plan_fingerprints: &HashMap<String, String>,
) -> Vec<String> {
    let mut errors = Vec::new();
    if plan.operation_id.is_nil() {
        errors.push("write plan operation_id must be present".to_owned());
    }
    if plan.idempotency_key.trim().is_empty() {
        errors.push("write plan idempotency_key must be present".to_owned());
    }
    if let Some(prior_fingerprint) = prior_plan_fingerprints.get(&plan.idempotency_key) {
        match plan_fingerprint(plan) {
            Ok(current_fingerprint) if &current_fingerprint != prior_fingerprint => errors
                .push("write plan idempotency_key was reused with divergent content".to_owned()),
            Ok(_) => {}
            Err(error) => errors.push(error.to_string()),
        }
    }

    errors
}

pub(crate) fn plan_fingerprint(plan: &RememberWritePlan) -> Result<String, CustomError> {
    serde_json::to_string(plan).map_err(|error| validation_error(error.to_string()))
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

fn validate_derived_memory_lifecycle(object: &crate::api::types::DerivedMemory) -> Vec<String> {
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

    use crate::api::types::write_plan::RememberPlanDefaults;
    use crate::api::types::{
        CandidateProvenance, CandidateRationale, DerivedMemoryDraft, DerivedType, EntityDraft,
        EntityType, EpisodeDraft, MemoryLinkDraft, RelationType, RememberInput, SourceSpan,
        Stability, StatsUpdateCandidate, VectorIndexCandidate, DEFAULT_SCHEMA_VERSION,
    };
    use crate::internal::repositories::test_support::{
        representative_fixtures, FakeGraphAuthorityStore,
    };

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
    async fn rejects_same_key_divergent_content_against_prior_fingerprint() {
        let graph = FakeGraphAuthorityStore::new();
        let prior = valid_plan();
        let mut divergent = valid_plan();
        divergent.candidates.push(MemoryCandidate::Entity(
            crate::api::types::EntityCandidate::new(
                EntityDraft::new(EntityType::Concept, "new divergent candidate"),
                CandidateProvenance::caller("caller supplied entity"),
            ),
        ));

        let verdict = WritePlanValidator::new(&graph)
            .with_prior_plan_fingerprint(
                divergent.idempotency_key.clone(),
                plan_fingerprint(&prior).unwrap(),
            )
            .validate(&divergent)
            .await
            .unwrap();

        assert_rejected_with(&verdict, "divergent content");
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
