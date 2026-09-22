// Correction/forget lifecycle pipeline used by the public facade and internal
// tests. The module keeps helper APIs for focused lifecycle controls and
// fixture-level validation.
use chrono::Utc;
use serde::Serialize;

use crate::api::types::lifecycle::{
    LifecycleMutationDiagnostics, LifecycleMutationOutcome, LifecycleMutationTrace,
    LifecycleMutationWarning, LifecycleMutationWarningReason,
};
use crate::api::types::{
    CorrectMemoryDraft, CorrectionTarget, ForgetMemoryDraft, LifecycleTargetRef,
    ReplacementDerivedMemoryDraft, SourceObjectCorrectionTarget, SourceProvenanceReference,
    SupersededByEvidence, VectorMaintenanceFailure, VectorMaintenanceFailureItem,
    VectorMaintenanceOperation,
};
use crate::domain::{
    DerivedMemory, DerivedType, Episode, MemoryId, MemoryLink, MemoryObject, MemoryObjectRef,
    MemoryThread, ObjectType, Observation, RetentionState, SourceReferenceKind,
    DEFAULT_SCHEMA_VERSION,
};
use crate::errors::{CustomError, ReplacementIdentityConflict, ReplacementIdentityConflictError};
use crate::models::vector::EmbeddingInput;
use crate::policy::memory_object_vector_record;
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::graph_authority::{
    GraphAuthorityStore, GraphDerivedMemoryProvenanceQuery, GraphDerivedMemoryThreadQuery,
    GraphExpansionLifecyclePolicy, GraphObjectQuery,
};
use crate::ports::retrieval_stats::RetrievalStatsStore;
use crate::ports::vector_candidate::VectorCandidateStore;
use crate::usecases::write_planning::{derived_memory_links, deterministic_uuid};
use crate::usecases::{StatsProjectionService, VectorIndexingService};

pub(crate) struct CorrectionForgetPipeline<'a, G, V, E>
where
    G: GraphAuthorityStore + ?Sized,
    V: VectorCandidateStore + ?Sized,
    E: MemoryEmbedder + ?Sized,
{
    graph_store: &'a G,
    vector_store: &'a V,
    embedder: &'a E,
    stats_store: &'a dyn RetrievalStatsStore,
}

impl<'a, G, V, E> CorrectionForgetPipeline<'a, G, V, E>
where
    G: GraphAuthorityStore + ?Sized,
    V: VectorCandidateStore + ?Sized,
    E: MemoryEmbedder + ?Sized,
{
    #[cfg(test)]
    pub(crate) fn new(graph_store: &'a G, vector_store: &'a V, embedder: &'a E) -> Self {
        Self {
            graph_store,
            vector_store,
            embedder,
            stats_store: crate::adapters::stats::noop_retrieval_stats_store(),
        }
    }

    pub(crate) fn new_with_stats(
        graph_store: &'a G,
        vector_store: &'a V,
        embedder: &'a E,
        stats_store: &'a dyn RetrievalStatsStore,
    ) -> Self {
        Self {
            graph_store,
            vector_store,
            embedder,
            stats_store,
        }
    }

    pub(crate) async fn correct(
        &self,
        draft: CorrectMemoryDraft,
        write_turn: &tokio::sync::Mutex<()>,
    ) -> Result<LifecycleMutationOutcome, CustomError> {
        draft.validate()?;
        let replacements = correction_replacements(&draft)?;
        let inputs = replacements
            .iter()
            .map(|replacement| {
                EmbeddingInput::new(
                    replacement.id,
                    Some(ObjectType::DerivedMemory),
                    crate::domain::VectorSurface::DerivedText,
                    crate::policy::embedding_surface::derived_embedding_text(
                        replacement.derived_type,
                        &replacement.text,
                    ),
                )
            })
            .collect::<Vec<_>>();
        let embeddings = self.embedder.embed_batch(&inputs).await;
        let _turn = write_turn.lock().await;
        let mut plan = self.correction_plan(draft, replacements).await?;
        super::scope::derive_scope_keys(self.graph_store, &mut plan.graph_objects).await?;
        let idempotent_ids = self.idempotent_replacement_ids(&plan).await?;
        let graph_objects = plan
            .graph_objects
            .iter()
            .filter(|object| !idempotent_ids.contains(&object.id()))
            .cloned()
            .collect::<Vec<_>>();
        let graph_links = plan
            .graph_links
            .iter()
            .filter(|link| !idempotent_ids.contains(&link.from_id))
            .cloned()
            .collect::<Vec<_>>();
        if !graph_objects.is_empty() || !graph_links.is_empty() {
            let existing_links = self
                .graph_store
                .query_links_by_ids(&graph_links.iter().map(|link| link.id).collect::<Vec<_>>())
                .await?;
            crate::usecases::link::reject_divergent_links(&graph_links, &existing_links)?;
            self.graph_store
                .upsert_objects_and_links(&graph_objects, &graph_links)
                .await?;
        }

        let mut outcome = plan.outcome_after_graph_success();
        outcome
            .graph_mutated_object_ids
            .retain(|object| !idempotent_ids.contains(&object.id));
        outcome.graph_mutated_link_ids = graph_links.iter().map(|link| link.id).collect();
        if let Some(trace) = &mut outcome.trace {
            trace
                .superseded_by
                .retain(|evidence| !idempotent_ids.contains(&evidence.superseded_by_memory_id));
        }
        let vector_result = self
            .maintain_vectors(
                &plan.vector_delete_refs,
                &plan.graph_objects,
                &inputs,
                embeddings,
            )
            .await?;
        apply_vector_result(&mut outcome, vector_result);
        if !plan.graph_objects.is_empty() || !plan.graph_links.is_empty() {
            let projection = StatsProjectionService::new(self.graph_store, self.stats_store)
                .project(&plan.graph_objects, &plan.graph_links)
                .await;
            outcome.stats_update_status = projection.into_status();
        }
        Ok(outcome)
    }

    pub(crate) async fn forget(
        &self,
        draft: ForgetMemoryDraft,
        write_turn: &tokio::sync::Mutex<()>,
    ) -> Result<LifecycleMutationOutcome, CustomError> {
        draft.validate()?;
        let _turn = write_turn.lock().await;
        let plan = self.forget_plan(draft).await?;

        self.graph_store.upsert_objects(&plan.graph_objects).await?;

        let mut outcome = plan.outcome_after_graph_success();
        let vector_result = self
            .maintain_vectors(&plan.vector_delete_refs, &[], &[], Ok(Vec::new()))
            .await?;
        apply_vector_result(&mut outcome, vector_result);
        let projection = StatsProjectionService::new(self.graph_store, self.stats_store)
            .project(&plan.graph_objects, &plan.graph_links)
            .await;
        outcome.stats_update_status = projection.into_status();
        Ok(outcome)
    }

    async fn correction_plan(
        &self,
        draft: CorrectMemoryDraft,
        replacements: Vec<ReplacementDerivedMemoryDraft>,
    ) -> Result<MutationPlan, CustomError> {
        let mut superseded = Vec::new();
        let mut source_episode_ids = Vec::new();
        let mut source_observation_ids = Vec::new();
        let mut requested_targets = Vec::new();

        for target in &draft.targets {
            match target {
                CorrectionTarget::DerivedMemory { id } => {
                    requested_targets.push(LifecycleTargetRef::DerivedMemory(*id));
                    let memory = self.fetch_derived_memory(*id).await?;
                    absorb_sources(
                        &mut source_episode_ids,
                        &mut source_observation_ids,
                        &memory,
                    );
                    superseded.push(memory);
                }
                CorrectionTarget::SourceObject { target } => {
                    requested_targets.push(source_correction_lifecycle_ref(target));
                    self.ensure_source_object_matches_original_refs(target)
                        .await?;
                    match target {
                        SourceObjectCorrectionTarget::Episode { id, .. } => {
                            push_unique(&mut source_episode_ids, *id)
                        }
                        SourceObjectCorrectionTarget::Observation { id, .. } => {
                            push_unique(&mut source_observation_ids, *id)
                        }
                    }
                    if draft.cascade_policy.apply_to_provenanced_derived_memories {
                        let affected = self
                            .query_current_derived_by_provenance(
                                source_episode_ids.clone(),
                                source_observation_ids.clone(),
                            )
                            .await?;
                        for memory in affected {
                            absorb_sources(
                                &mut source_episode_ids,
                                &mut source_observation_ids,
                                &memory,
                            );
                            push_memory_unique(&mut superseded, memory);
                        }
                    }
                }
            }
        }

        for id in &draft.superseded_derived_memory_ids {
            let memory = self.fetch_derived_memory(*id).await?;
            absorb_sources(
                &mut source_episode_ids,
                &mut source_observation_ids,
                &memory,
            );
            push_memory_unique(&mut superseded, memory);
        }
        sort_derived_memories(&mut superseded);

        let mut replacement_drafts = ground_replacement_drafts(
            &draft,
            replacements,
            &superseded,
            &source_episode_ids,
            &source_observation_ids,
        )?;
        let replacement_ids = replacement_drafts
            .iter()
            .map(|replacement| {
                replacement
                    .id
                    .expect("replacement id prepared before embedding")
            })
            .collect::<Vec<_>>();
        ensure_unique_replacement_ids(&replacement_ids)?;
        preserve_retried_replacement_lineage(
            &mut replacement_drafts,
            &superseded,
            &replacement_ids,
        )?;
        superseded.retain(|memory| !replacement_ids.contains(&memory.id));

        let replacement_memories = replacement_drafts
            .into_iter()
            .zip(replacement_ids)
            .map(|(replacement, replacement_id)| {
                replacement_memory(replacement, replacement_id, &superseded)
            })
            .collect::<Result<Vec<_>, _>>()?;

        for memory in &replacement_memories {
            for subject in &memory.entity_ids {
                self.fetch_one(MemoryObjectRef::new(ObjectType::Entity, *subject))
                    .await?;
            }
        }

        let graph_objects = replacement_memories
            .iter()
            .cloned()
            .map(MemoryObject::DerivedMemory)
            .collect::<Vec<_>>();
        let graph_links = replacement_memories
            .iter()
            .flat_map(derived_memory_links)
            .collect::<Vec<_>>();
        let mut superseded_ids = graph_links
            .iter()
            .filter(|link| link.relation == crate::domain::RelationType::Supersedes)
            .map(|link| link.to_id)
            .collect::<Vec<_>>();
        sort_dedup(&mut superseded_ids);
        for predecessor_id in &superseded_ids {
            if !replacement_memories
                .iter()
                .any(|memory| memory.id == *predecessor_id)
            {
                self.fetch_derived_memory(*predecessor_id).await?;
            }
        }
        let vector_delete_refs = superseded_ids
            .iter()
            .copied()
            .map(|id| MemoryObjectRef::new(ObjectType::DerivedMemory, id))
            .collect::<Vec<_>>();
        let trace = draft.include_trace.then(|| LifecycleMutationTrace {
            requested_targets,
            superseded_by: graph_links
                .iter()
                .filter(|link| link.relation == crate::domain::RelationType::Supersedes)
                .map(|link| SupersededByEvidence {
                    superseded_memory_id: link.to_id,
                    superseded_by_memory_id: link.from_id,
                })
                .collect(),
        });

        Ok(MutationPlan::new(
            graph_objects,
            graph_links,
            vector_delete_refs,
            trace,
            LifecycleMutationDiagnostics::default(),
        ))
    }

    async fn idempotent_replacement_ids(
        &self,
        plan: &MutationPlan,
    ) -> Result<Vec<MemoryId>, CustomError> {
        let replacement_refs = plan
            .graph_objects
            .iter()
            .map(MemoryObject::object_ref)
            .collect::<Vec<_>>();
        if replacement_refs.is_empty() {
            return Ok(Vec::new());
        }

        let existing = self
            .graph_store
            .query_objects(&GraphObjectQuery::by_refs(replacement_refs))
            .await?;
        let mut idempotent_ids = Vec::new();
        for existing_object in existing {
            let Some(planned_object) = plan
                .graph_objects
                .iter()
                .find(|planned| planned.object_ref() == existing_object.object_ref())
            else {
                continue;
            };
            let (
                MemoryObject::DerivedMemory(planned_memory),
                MemoryObject::DerivedMemory(existing_memory),
            ) = (planned_object, &existing_object)
            else {
                continue;
            };

            if !replacement_content_matches(planned_memory, existing_memory) {
                return Err(ReplacementIdentityConflictError {
                    replacement_id: planned_memory.id,
                    conflict: ReplacementIdentityConflict::DivergentExisting,
                }
                .into());
            }
            idempotent_ids.push(planned_memory.id);
        }

        Ok(idempotent_ids)
    }

    async fn forget_plan(&self, draft: ForgetMemoryDraft) -> Result<MutationPlan, CustomError> {
        let mut graph_objects = Vec::new();
        let mut vector_delete_refs = Vec::new();
        let requested_targets = draft.targets.clone();
        let mut cascade_warning_ids = Vec::new();

        for target in &draft.targets {
            match target {
                LifecycleTargetRef::DerivedMemory(id) => {
                    let memory = self.fetch_derived_memory(*id).await?;
                    if draft.lifecycle_policy.suppression.suppress_target {
                        let suppressed = suppress_derived_memory(memory);
                        push_object_unique(
                            &mut graph_objects,
                            MemoryObject::DerivedMemory(suppressed),
                        );
                        push_ref_unique(&mut vector_delete_refs, target.as_memory_object_ref());
                    }
                }
                LifecycleTargetRef::Episode(id) => {
                    let episode = self.fetch_episode(*id).await?;
                    if draft.lifecycle_policy.suppression.suppress_target {
                        let mut suppressed = episode;
                        suppressed.retention_state = RetentionState::Suppressed;
                        push_object_unique(&mut graph_objects, MemoryObject::Episode(suppressed));
                        push_ref_unique(&mut vector_delete_refs, target.as_memory_object_ref());
                    }
                    if draft.cascade_policy.apply_to_derived_from_target
                        && draft
                            .lifecycle_policy
                            .suppression
                            .suppress_derived_from_target
                    {
                        self.add_forget_cascade(
                            &mut graph_objects,
                            &mut vector_delete_refs,
                            vec![*id],
                            Vec::new(),
                            &mut cascade_warning_ids,
                        )
                        .await?;
                    }
                }
                LifecycleTargetRef::Observation(id) => {
                    let observation = self.fetch_observation(*id).await?;
                    if draft.lifecycle_policy.suppression.suppress_target {
                        let mut suppressed = observation;
                        suppressed.retention_state = RetentionState::Suppressed;
                        push_object_unique(
                            &mut graph_objects,
                            MemoryObject::Observation(suppressed),
                        );
                        push_ref_unique(&mut vector_delete_refs, target.as_memory_object_ref());
                    }
                    if draft.cascade_policy.apply_to_derived_from_target
                        && draft
                            .lifecycle_policy
                            .suppression
                            .suppress_derived_from_target
                    {
                        self.add_forget_cascade(
                            &mut graph_objects,
                            &mut vector_delete_refs,
                            Vec::new(),
                            vec![*id],
                            &mut cascade_warning_ids,
                        )
                        .await?;
                    }
                }
                LifecycleTargetRef::MemoryThread(id) => {
                    self.fetch_thread(*id).await?;
                    if draft.cascade_policy.apply_to_thread_members {
                        self.add_thread_forget_cascade(
                            &mut graph_objects,
                            &mut vector_delete_refs,
                            *id,
                            &mut cascade_warning_ids,
                        )
                        .await?;
                    }
                }
            }
        }

        let trace = draft.include_trace.then(|| LifecycleMutationTrace {
            requested_targets,
            superseded_by: Vec::new(),
        });
        Ok(MutationPlan::new(
            graph_objects,
            Vec::new(),
            vector_delete_refs,
            trace,
            cascade_diagnostics(cascade_warning_ids),
        ))
    }

    async fn add_forget_cascade(
        &self,
        graph_objects: &mut Vec<MemoryObject>,
        vector_delete_refs: &mut Vec<MemoryObjectRef>,
        episode_ids: Vec<MemoryId>,
        observation_ids: Vec<MemoryId>,
        cascade_warning_ids: &mut Vec<MemoryId>,
    ) -> Result<(), CustomError> {
        let affected = self
            .query_current_derived_by_provenance(episode_ids, observation_ids)
            .await?;
        for memory in affected {
            record_current_replacement_warning(&memory, cascade_warning_ids);
            let id = memory.id;
            push_object_unique(
                graph_objects,
                MemoryObject::DerivedMemory(suppress_derived_memory(memory)),
            );
            push_ref_unique(
                vector_delete_refs,
                MemoryObjectRef::new(ObjectType::DerivedMemory, id),
            );
        }
        Ok(())
    }

    async fn add_thread_forget_cascade(
        &self,
        graph_objects: &mut Vec<MemoryObject>,
        vector_delete_refs: &mut Vec<MemoryObjectRef>,
        thread_id: MemoryId,
        cascade_warning_ids: &mut Vec<MemoryId>,
    ) -> Result<(), CustomError> {
        let (matches, _) = self
            .graph_store
            .query_derived_memories_by_thread(
                &GraphDerivedMemoryThreadQuery::by_threads(vec![thread_id])
                    .with_lifecycle_policy(GraphExpansionLifecyclePolicy::default()),
            )
            .await?;
        for memory in matches {
            record_current_replacement_warning(&memory, cascade_warning_ids);
            let id = memory.id;
            push_object_unique(
                graph_objects,
                MemoryObject::DerivedMemory(suppress_derived_memory(memory)),
            );
            push_ref_unique(
                vector_delete_refs,
                MemoryObjectRef::new(ObjectType::DerivedMemory, id),
            );
        }
        Ok(())
    }

    async fn fetch_derived_memory(&self, id: MemoryId) -> Result<DerivedMemory, CustomError> {
        let object = self
            .fetch_one(MemoryObjectRef::from_id_type(id, ObjectType::DerivedMemory))
            .await?;
        match object {
            MemoryObject::DerivedMemory(memory) => Ok(memory),
            _ => Err(missing_object_error(ObjectType::DerivedMemory, id)),
        }
    }

    async fn fetch_episode(&self, id: MemoryId) -> Result<Episode, CustomError> {
        let object = self
            .fetch_one(MemoryObjectRef::from_id_type(id, ObjectType::Episode))
            .await?;
        match object {
            MemoryObject::Episode(episode) => Ok(episode),
            _ => Err(missing_object_error(ObjectType::Episode, id)),
        }
    }

    async fn fetch_observation(&self, id: MemoryId) -> Result<Observation, CustomError> {
        let object = self
            .fetch_one(MemoryObjectRef::from_id_type(id, ObjectType::Observation))
            .await?;
        match object {
            MemoryObject::Observation(observation) => Ok(observation),
            _ => Err(missing_object_error(ObjectType::Observation, id)),
        }
    }

    async fn fetch_thread(&self, id: MemoryId) -> Result<MemoryThread, CustomError> {
        let object = self
            .fetch_one(MemoryObjectRef::from_id_type(id, ObjectType::MemoryThread))
            .await?;
        match object {
            MemoryObject::MemoryThread(thread) => Ok(thread),
            _ => Err(missing_object_error(ObjectType::MemoryThread, id)),
        }
    }

    async fn fetch_one(&self, object_ref: MemoryObjectRef) -> Result<MemoryObject, CustomError> {
        let mut objects = self
            .graph_store
            .query_objects(&GraphObjectQuery::by_refs(vec![object_ref]))
            .await?;
        objects
            .pop()
            .ok_or_else(|| missing_object_error(object_ref.object_type, object_ref.id))
    }

    async fn ensure_source_object_matches_original_refs(
        &self,
        target: &SourceObjectCorrectionTarget,
    ) -> Result<(), CustomError> {
        let object_ref = source_correction_lifecycle_ref(target).as_memory_object_ref();
        let has_raw_ref =
            source_target_original_raw_ref(target).is_some_and(|value| !value.trim().is_empty());
        let has_setting_key = source_target_original_setting_key(target)
            .is_some_and(|value| !value.trim().is_empty());
        if !has_raw_ref && !has_setting_key {
            return Err(CustomError::MissingOriginalSourceReference { target: object_ref });
        }

        match target {
            SourceObjectCorrectionTarget::Episode {
                id,
                original_raw_ref,
                original_setting_key,
            } => {
                let episode = self.fetch_episode(*id).await?;
                validate_optional_original_ref(
                    object_ref,
                    SourceReferenceKind::Raw,
                    original_raw_ref.as_deref(),
                    episode.raw_ref.as_deref(),
                )?;
                validate_optional_original_ref(
                    object_ref,
                    SourceReferenceKind::SettingKey,
                    original_setting_key.as_deref(),
                    episode.scene.setting.key.as_deref(),
                )
            }
            SourceObjectCorrectionTarget::Observation {
                id,
                original_raw_ref,
                original_setting_key,
            } => {
                let observation = self.fetch_observation(*id).await?;
                validate_optional_original_ref(
                    object_ref,
                    SourceReferenceKind::Raw,
                    original_raw_ref.as_deref(),
                    observation.raw_ref.as_deref(),
                )?;
                if original_setting_key
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    let episode = self.fetch_episode(observation.episode_id).await?;
                    validate_optional_original_ref(
                        object_ref,
                        SourceReferenceKind::SettingKey,
                        original_setting_key.as_deref(),
                        episode.scene.setting.key.as_deref(),
                    )?;
                }
                Ok(())
            }
        }
    }

    async fn query_current_derived_by_provenance(
        &self,
        episode_ids: Vec<MemoryId>,
        mut observation_ids: Vec<MemoryId>,
    ) -> Result<Vec<DerivedMemory>, CustomError> {
        observation_ids.extend(self.observation_ids_for_episodes(&episode_ids).await?);
        sort_dedup(&mut observation_ids);
        let mut matches = self
            .graph_store
            .query_derived_memories_by_provenance(
                &GraphDerivedMemoryProvenanceQuery::by_sources(episode_ids, observation_ids)
                    .with_lifecycle_policy(GraphExpansionLifecyclePolicy::default()),
            )
            .await?;
        sort_derived_memories(&mut matches);
        Ok(matches)
    }

    async fn observation_ids_for_episodes(
        &self,
        episode_ids: &[MemoryId],
    ) -> Result<Vec<MemoryId>, CustomError> {
        if episode_ids.is_empty() {
            return Ok(Vec::new());
        }
        let objects = self
            .graph_store
            .query_objects(&GraphObjectQuery::by_types(
                vec![ObjectType::Observation],
                None,
            ))
            .await?;
        let mut observation_ids = objects
            .into_iter()
            .filter_map(|object| match object {
                MemoryObject::Observation(observation)
                    if episode_ids.contains(&observation.episode_id) =>
                {
                    Some(observation.id)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        sort_dedup(&mut observation_ids);
        Ok(observation_ids)
    }

    async fn maintain_vectors(
        &self,
        delete_refs: &[MemoryObjectRef],
        upsert_objects: &[MemoryObject],
        inputs: &[EmbeddingInput],
        embeddings: Result<Vec<Vec<f32>>, CustomError>,
    ) -> Result<VectorMaintenanceResult, CustomError> {
        let mut maintained = Vec::new();
        let mut failures = Vec::new();

        match crate::usecases::vector_indexing::delete_vectors(self.vector_store, delete_refs)
            .await?
        {
            Some(failure) => failures.push(failure),
            None => maintained.extend_from_slice(delete_refs),
        }

        let vector_records = upsert_objects
            .iter()
            .filter_map(memory_object_vector_record)
            .collect::<Vec<_>>();
        if !vector_records.is_empty() {
            let indexing = VectorIndexingService::new(self.vector_store)
                .index(self.graph_store, vector_records, inputs, embeddings)
                .await?;
            maintained.extend(indexing.indexed_objects);
            if let Some(failure) = indexing.failure {
                failures.push(VectorMaintenanceFailureItem {
                    operation: VectorMaintenanceOperation::Upsert,
                    objects: failure.unindexed_objects,
                    cause: failure.cause,
                });
            }
        }

        sort_refs(&mut maintained);
        maintained.dedup();
        Ok(VectorMaintenanceResult {
            maintained,
            failure: (!failures.is_empty()).then_some(VectorMaintenanceFailure { failures }),
        })
    }
}

#[derive(Debug, Clone)]
struct MutationPlan {
    graph_objects: Vec<MemoryObject>,
    graph_links: Vec<MemoryLink>,
    vector_delete_refs: Vec<MemoryObjectRef>,
    trace: Option<LifecycleMutationTrace>,
    diagnostics: LifecycleMutationDiagnostics,
}

impl MutationPlan {
    fn new(
        mut graph_objects: Vec<MemoryObject>,
        mut graph_links: Vec<MemoryLink>,
        mut vector_delete_refs: Vec<MemoryObjectRef>,
        trace: Option<LifecycleMutationTrace>,
        diagnostics: LifecycleMutationDiagnostics,
    ) -> Self {
        sort_objects(&mut graph_objects);
        graph_links.sort_by_key(|link| link.id);
        sort_refs(&mut vector_delete_refs);
        vector_delete_refs.dedup();
        Self {
            graph_objects,
            graph_links,
            vector_delete_refs,
            trace,
            diagnostics,
        }
    }

    fn outcome_after_graph_success(&self) -> LifecycleMutationOutcome {
        let mut graph_mutated_object_ids = self
            .graph_objects
            .iter()
            .map(MemoryObject::object_ref)
            .collect::<Vec<_>>();
        sort_refs(&mut graph_mutated_object_ids);
        LifecycleMutationOutcome {
            graph_mutated_object_ids,
            graph_mutated_link_ids: self.graph_links.iter().map(|link| link.id).collect(),
            vector_maintained_object_ids: Vec::new(),
            vector_maintenance_failure: None,
            stats_update_status: crate::api::types::StatsUpdateStatus::default(),
            trace: self.trace.clone(),
            diagnostics: self.diagnostics.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct VectorMaintenanceResult {
    maintained: Vec<MemoryObjectRef>,
    failure: Option<VectorMaintenanceFailure>,
}

fn correction_replacements(
    draft: &CorrectMemoryDraft,
) -> Result<Vec<ReplacementDerivedMemoryDraft>, CustomError> {
    let seed = correction_seed(draft)?;
    let mut replacements = if draft.replacement_derived_memories.is_empty() {
        vec![ReplacementDerivedMemoryDraft::new(
            DerivedType::Correction,
            &draft.rationale,
        )]
    } else {
        draft.replacement_derived_memories.clone()
    };
    for (index, replacement) in replacements.iter_mut().enumerate() {
        replacement
            .id
            .get_or_insert_with(|| replacement_memory_id(seed, index));
    }
    Ok(replacements)
}

fn ground_replacement_drafts(
    draft: &CorrectMemoryDraft,
    mut replacements: Vec<ReplacementDerivedMemoryDraft>,
    superseded: &[DerivedMemory],
    source_episode_ids: &[MemoryId],
    source_observation_ids: &[MemoryId],
) -> Result<Vec<ReplacementDerivedMemoryDraft>, CustomError> {
    if draft.replacement_derived_memories.is_empty() {
        let replacement = &mut replacements[0];
        replacement.derived_from_episode_ids = source_episode_ids.to_vec();
        replacement.derived_from_observation_ids = source_observation_ids.to_vec();
        replacement.thread_ids = stable_union(
            superseded
                .iter()
                .flat_map(|memory| memory.thread_ids.clone()),
        );
        replacement.entity_ids = stable_union(
            superseded
                .iter()
                .flat_map(|memory| memory.entity_ids.clone()),
        );
        replacement.salience_score = superseded
            .iter()
            .map(|memory| memory.salience_score)
            .max_by(f32::total_cmp)
            .unwrap_or(0.5);
        replacement.supersedes = superseded.iter().map(|memory| memory.id).collect();
        replacement.original_source_provenance = SourceProvenanceReference {
            episode_ids: source_episode_ids.to_vec(),
            observation_ids: source_observation_ids.to_vec(),
            external_refs: Vec::new(),
        };
        replacement.correction_origin_provenance = draft.correction_origin.clone();
    }

    for replacement in &mut replacements {
        merge_sources(
            &mut replacement.derived_from_episode_ids,
            &mut replacement.derived_from_observation_ids,
            &replacement.original_source_provenance,
        );
        merge_sources(
            &mut replacement.derived_from_episode_ids,
            &mut replacement.derived_from_observation_ids,
            &replacement.correction_origin_provenance,
        );
        merge_sources(
            &mut replacement.derived_from_episode_ids,
            &mut replacement.derived_from_observation_ids,
            &draft.correction_origin,
        );
        if !replacement.given_by_application
            && replacement.derived_from_episode_ids.is_empty()
            && replacement.derived_from_observation_ids.is_empty()
        {
            replacement
                .derived_from_episode_ids
                .extend_from_slice(source_episode_ids);
            replacement
                .derived_from_observation_ids
                .extend_from_slice(source_observation_ids);
        }
        for memory in superseded {
            if replacement.thread_ids.is_empty() {
                replacement.thread_ids.extend_from_slice(&memory.thread_ids);
            }
            if replacement.entity_ids.is_empty() {
                replacement.entity_ids.extend_from_slice(&memory.entity_ids);
            }
            push_unique(&mut replacement.supersedes, memory.id);
        }
        sort_dedup(&mut replacement.derived_from_episode_ids);
        sort_dedup(&mut replacement.derived_from_observation_ids);
        sort_dedup(&mut replacement.thread_ids);
        sort_dedup(&mut replacement.entity_ids);
        sort_dedup(&mut replacement.supersedes);
        if draft.replacement_derived_memories.is_empty()
            && !superseded.is_empty()
            && superseded.iter().all(|memory| memory.given_by_application)
            && replacement.derived_from_episode_ids.is_empty()
            && replacement.derived_from_observation_ids.is_empty()
        {
            return Err(crate::domain::LifecycleDtoValidationError::MissingGivenReplacement.into());
        }
        replacement.validate()?;
    }

    Ok(replacements)
}

fn replacement_memory(
    draft: ReplacementDerivedMemoryDraft,
    replacement_id: MemoryId,
    superseded: &[DerivedMemory],
) -> Result<DerivedMemory, CustomError> {
    let now = Utc::now();
    let memory = DerivedMemory {
        id: replacement_id,
        object_type: ObjectType::DerivedMemory,
        derived_type: draft.derived_type,
        text: draft.text,
        derived_from_episode_ids: draft.derived_from_episode_ids,
        derived_from_observation_ids: draft.derived_from_observation_ids,
        thread_ids: draft.thread_ids,
        entity_ids: draft.entity_ids,
        scope_keys: Vec::new(),
        assertions: draft.assertions,
        given_by_application: draft.given_by_application,
        salience_score: draft.salience_score,
        supersedes: draft.supersedes,
        retention_state: RetentionState::Active,
        created_at: now,
        updated_at: now,
        schema_version: superseded
            .first()
            .map(|memory| memory.schema_version.clone())
            .unwrap_or_else(|| DEFAULT_SCHEMA_VERSION.to_owned()),
    };
    memory.validate()?;
    Ok(memory)
}

#[derive(Serialize)]
struct CorrectionIdentityView<'a> {
    targets: &'a [CorrectionTarget],
    superseded_derived_memory_ids: &'a [MemoryId],
    correction_origin: &'a SourceProvenanceReference,
    rationale: &'a str,
    replacement_derived_memories: Vec<ReplacementDerivedMemoryDraft>,
}

fn correction_seed(request: &CorrectMemoryDraft) -> Result<MemoryId, CustomError> {
    let mut replacement_derived_memories = request.replacement_derived_memories.clone();
    for replacement in &mut replacement_derived_memories {
        replacement.id = None;
    }
    let canonical = serde_json::to_string(&CorrectionIdentityView {
        targets: &request.targets,
        superseded_derived_memory_ids: &request.superseded_derived_memory_ids,
        correction_origin: &request.correction_origin,
        rationale: &request.rationale,
        replacement_derived_memories,
    })?;
    Ok(deterministic_uuid(&[
        b"character_memory.lifecycle.correction",
        canonical.as_bytes(),
    ]))
}

fn replacement_memory_id(correction_seed: MemoryId, index: usize) -> MemoryId {
    let position = format!("replacement:{index}");
    deterministic_uuid(&[
        b"character_memory.lifecycle.replacement_memory",
        correction_seed.as_bytes(),
        position.as_bytes(),
    ])
}

fn ensure_unique_replacement_ids(replacement_ids: &[MemoryId]) -> Result<(), CustomError> {
    let mut seen = Vec::new();
    for replacement_id in replacement_ids {
        if seen.contains(replacement_id) {
            return Err(ReplacementIdentityConflictError {
                replacement_id: *replacement_id,
                conflict: ReplacementIdentityConflict::DuplicateInPlan,
            }
            .into());
        }
        seen.push(*replacement_id);
    }
    Ok(())
}

fn preserve_retried_replacement_lineage(
    replacements: &mut [ReplacementDerivedMemoryDraft],
    superseded: &[DerivedMemory],
    replacement_ids: &[MemoryId],
) -> Result<(), CustomError> {
    for (replacement, replacement_id) in replacements.iter_mut().zip(replacement_ids) {
        replacement
            .supersedes
            .retain(|id| !replacement_ids.contains(id));
        if let Some(previous) = superseded
            .iter()
            .find(|memory| memory.id == *replacement_id)
        {
            for ancestor_id in &previous.supersedes {
                push_unique(&mut replacement.supersedes, *ancestor_id);
            }
        }
        sort_dedup(&mut replacement.supersedes);
        replacement.validate()?;
    }
    Ok(())
}

fn replacement_content_matches(planned: &DerivedMemory, existing: &DerivedMemory) -> bool {
    let mut planned = planned.clone();
    planned.created_at = existing.created_at;
    planned.updated_at = existing.updated_at;
    planned == *existing
}

fn suppress_derived_memory(mut memory: DerivedMemory) -> DerivedMemory {
    memory.retention_state = RetentionState::Suppressed;
    memory.updated_at = Utc::now();
    memory
}

fn source_correction_lifecycle_ref(target: &SourceObjectCorrectionTarget) -> LifecycleTargetRef {
    match target {
        SourceObjectCorrectionTarget::Episode { id, .. } => LifecycleTargetRef::Episode(*id),
        SourceObjectCorrectionTarget::Observation { id, .. } => {
            LifecycleTargetRef::Observation(*id)
        }
    }
}

fn source_target_original_raw_ref(target: &SourceObjectCorrectionTarget) -> Option<&str> {
    match target {
        SourceObjectCorrectionTarget::Episode {
            original_raw_ref, ..
        }
        | SourceObjectCorrectionTarget::Observation {
            original_raw_ref, ..
        } => original_raw_ref.as_deref(),
    }
}

fn source_target_original_setting_key(target: &SourceObjectCorrectionTarget) -> Option<&str> {
    match target {
        SourceObjectCorrectionTarget::Episode {
            original_setting_key,
            ..
        }
        | SourceObjectCorrectionTarget::Observation {
            original_setting_key,
            ..
        } => original_setting_key.as_deref(),
    }
}

fn validate_optional_original_ref(
    target: MemoryObjectRef,
    kind: SourceReferenceKind,
    provided: Option<&str>,
    stored: Option<&str>,
) -> Result<(), CustomError> {
    let Some(provided) = provided.filter(|value| !value.trim().is_empty()) else {
        return Ok(());
    };

    if stored == Some(provided) {
        Ok(())
    } else {
        Err(CustomError::OriginalSourceReferenceMismatch {
            target,
            kind,
            provided: provided.to_owned(),
            stored: stored.map(str::to_owned),
        })
    }
}

fn absorb_sources(
    episode_ids: &mut Vec<MemoryId>,
    observation_ids: &mut Vec<MemoryId>,
    memory: &DerivedMemory,
) {
    for id in &memory.derived_from_episode_ids {
        push_unique(episode_ids, *id);
    }
    for id in &memory.derived_from_observation_ids {
        push_unique(observation_ids, *id);
    }
}

fn merge_sources(
    episode_ids: &mut Vec<MemoryId>,
    observation_ids: &mut Vec<MemoryId>,
    provenance: &SourceProvenanceReference,
) {
    for id in &provenance.episode_ids {
        push_unique(episode_ids, *id);
    }
    for id in &provenance.observation_ids {
        push_unique(observation_ids, *id);
    }
}

fn apply_vector_result(outcome: &mut LifecycleMutationOutcome, result: VectorMaintenanceResult) {
    outcome.vector_maintained_object_ids = result.maintained;
    outcome.vector_maintenance_failure = result.failure;
}

fn sort_objects(objects: &mut [MemoryObject]) {
    objects.sort_by_key(MemoryObject::stable_order_key);
}

fn sort_refs(refs: &mut [MemoryObjectRef]) {
    refs.sort_by_key(|object_ref| object_ref.stable_order_key());
}

fn sort_derived_memories(memories: &mut [DerivedMemory]) {
    memories.sort_by_key(|memory| memory.id);
}

fn push_memory_unique(memories: &mut Vec<DerivedMemory>, memory: DerivedMemory) {
    if !memories.iter().any(|existing| existing.id == memory.id) {
        memories.push(memory);
    }
}

fn push_object_unique(objects: &mut Vec<MemoryObject>, object: MemoryObject) {
    let object_ref = object.object_ref();
    objects.retain(|existing| existing.object_ref() != object_ref);
    objects.push(object);
}

fn push_ref_unique(refs: &mut Vec<MemoryObjectRef>, object_ref: MemoryObjectRef) {
    if !refs.contains(&object_ref) {
        refs.push(object_ref);
    }
}

fn push_unique(ids: &mut Vec<MemoryId>, id: MemoryId) {
    if !ids.contains(&id) {
        ids.push(id);
    }
}

fn record_current_replacement_warning(
    memory: &DerivedMemory,
    cascade_warning_ids: &mut Vec<MemoryId>,
) {
    if !memory.supersedes.is_empty() {
        push_unique(cascade_warning_ids, memory.id);
    }
}

fn cascade_diagnostics(mut affected_memory_ids: Vec<MemoryId>) -> LifecycleMutationDiagnostics {
    sort_dedup(&mut affected_memory_ids);
    if affected_memory_ids.is_empty() {
        return LifecycleMutationDiagnostics::default();
    }

    LifecycleMutationDiagnostics {
        warnings: vec![LifecycleMutationWarning {
            reason: LifecycleMutationWarningReason::CascadeSuppressesCurrentReplacement,
            affected_memory_ids,
        }],
    }
}

fn sort_dedup(ids: &mut Vec<MemoryId>) {
    ids.sort();
    ids.dedup();
}

fn stable_union(ids: impl IntoIterator<Item = MemoryId>) -> Vec<MemoryId> {
    let mut values = ids.into_iter().collect::<Vec<_>>();
    sort_dedup(&mut values);
    values
}

fn missing_object_error(object_type: ObjectType, id: MemoryId) -> CustomError {
    CustomError::GraphExpansionRootNotFound {
        object_type,
        object_id: id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ScopeKey;
    use crate::ports::graph_authority::GraphExpansionFilteredNode;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex, MutexGuard};
    use uuid::Uuid;

    use crate::adapters::oxigraph::OxigraphGraphAuthorityStore;
    use crate::adapters::stats::InMemoryRetrievalStatsStore;
    use crate::api::types::{ExternalSourceReference, RetrievalContext, StaleCandidateReason};
    use crate::domain::{Episode, Modality, Observation, RelationType};
    use crate::errors::VectorIndexingCause;
    use crate::errors::{
        RetrievalStatsHealthCause, RetrievalStatsStoreError, StatsUpdateCause, VectorDatabaseError,
        VectorDatabaseErrorKind,
    };
    use crate::models::vector::{
        CanonicalCandidates, EmbeddingInput, VectorCandidateSearch, VectorRecordEmbedding,
    };
    use crate::ports::graph_authority::{GraphExpansion, GraphExpansionQuery};
    use crate::ports::retrieval_stats::{
        RetrievalStatsCounter, RetrievalStatsCounterKey, RetrievalStatsEdge, RetrievalStatsHealth,
        RetrievalStatsObjectState,
    };
    use crate::ports::vector_candidate::VectorCandidateRecall;
    use crate::test_support::{
        in_memory_graph_store, representative_fixtures, DeterministicMemoryEmbedder,
        TemporaryVectorCandidateStore,
    };
    use crate::usecases::RetrievePipeline;

    #[tokio::test]
    async fn ordinary_successor_repairs_vectors_and_derives_currency_for_both_stats_stores() {
        use crate::adapters::stats::SqliteRetrievalStatsStore;
        use crate::api::types::{
            CandidateProvenance, CommitOptions, DerivedMemoryCandidate, DerivedMemoryDraft,
            LifecycleFilterReason, MemoryCandidate, RememberWritePlan, RepairMarker,
            VectorIndexCandidate,
        };
        use crate::ports::graph_authority::GraphExpansionFilteredReason;
        use crate::usecases::RememberPipeline;

        let directory = tempfile::tempdir().unwrap();
        let stores: Vec<Box<dyn RetrievalStatsStore>> = vec![
            Box::new(InMemoryRetrievalStatsStore::new()),
            Box::new(
                SqliteRetrievalStatsStore::open(directory.path().join("currency.sqlite")).unwrap(),
            ),
        ];
        for stats in stores {
            let fixtures = representative_fixtures();
            let graph = in_memory_graph_store();
            graph
                .upsert_objects(&[
                    MemoryObject::Episode(fixtures.episode.clone()),
                    MemoryObject::Entity(fixtures.user_entity.clone()),
                    MemoryObject::Entity(fixtures.assistant_entity.clone()),
                ])
                .await
                .unwrap();
            let vector = OneShotDeleteFailingVectorStore::new().await;
            let embedder = DeterministicMemoryEmbedder::new(4);
            let pipeline =
                RememberPipeline::new_with_stats(&graph, &vector, &embedder, stats.as_ref());
            let predecessor_id = fixtures.user_preference.id;
            let successor_id = fixtures.correction.id;
            let plan_for = |id, entity_id, supersedes| {
                let mut draft =
                    DerivedMemoryDraft::new(DerivedType::UserPreference, "Prefer concise answers.")
                        .with_source_episode(fixtures.episode.id);
                draft.id = Some(id);
                draft.created_at = Some(fixtures.episode.created_at);
                draft.updated_at = draft.created_at;
                draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
                draft.entity_ids = vec![entity_id];
                draft.supersedes = supersedes;
                RememberWritePlan::new()
                    .with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
                        draft,
                        CandidateProvenance::caller("ordinary memory"),
                    )))
                    .with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                        MemoryObjectRef::new(ObjectType::DerivedMemory, id),
                        CandidateProvenance::caller("index authored memory"),
                    )))
            };
            let predecessor_plan = plan_for(predecessor_id, fixtures.user_entity.id, vec![]);
            let first = pipeline
                .commit(
                    predecessor_plan.clone(),
                    CommitOptions::default(),
                    &tokio::sync::Mutex::new(()),
                )
                .await
                .unwrap();
            assert!(first.repair_needed.is_empty());
            let predecessor = graph
                .query_objects(&GraphObjectQuery::by_ids(vec![predecessor_id]))
                .await
                .unwrap();
            let search = VectorCandidateSearch::new(
                vec![1.0, 0.0, 0.0, 0.0],
                10,
                vec![ObjectType::DerivedMemory],
            );
            assert!(vector
                .search_candidates(&search)
                .await
                .unwrap()
                .candidates
                .iter()
                .any(|candidate| candidate.object_id == predecessor_id));
            let predecessor_counter = RetrievalStatsCounterKey {
                entity_id: fixtures.user_entity.id,
                relation_kind: RelationType::About,
                object_type: ObjectType::DerivedMemory,
            };
            assert_eq!(
                stats
                    .counter(&predecessor_counter)
                    .await
                    .unwrap()
                    .unwrap()
                    .current_count,
                1
            );

            // No caller-authored link and no predecessor object in this plan.
            let successor_plan = plan_for(
                successor_id,
                fixtures.assistant_entity.id,
                vec![predecessor_id],
            );
            let outcome = pipeline
                .commit(
                    successor_plan.clone(),
                    CommitOptions::default(),
                    &tokio::sync::Mutex::new(()),
                )
                .await
                .unwrap();
            assert_eq!(outcome.persisted_object_ids, vec![successor_id]);
            assert_eq!(outcome.persisted_link_ids.len(), 2);
            assert!(outcome.vector_indexing_failure.is_none());
            assert!(
                matches!(outcome.repair_needed.as_slice(), [RepairMarker::VectorMaintenance { failure }]
                if matches!(failure.failures.as_slice(), [item]
                    if item.operation == VectorMaintenanceOperation::Delete
                        && item.objects == vec![MemoryObjectRef::new(ObjectType::DerivedMemory, predecessor_id)]
                        && matches!(item.cause, VectorIndexingCause::VectorDatabase(_))))
            );
            assert_eq!(outcome.diagnostics.repair_needed, outcome.repair_needed);
            assert_eq!(
                graph
                    .query_objects(&GraphObjectQuery::by_ids(vec![predecessor_id]))
                    .await
                    .unwrap(),
                predecessor
            );
            assert_eq!(
                graph
                    .query_superseded_derived_memory_ids(&[predecessor_id, successor_id])
                    .await
                    .unwrap(),
                vec![predecessor_id]
            );
            assert_eq!(
                stats.counter(&predecessor_counter).await.unwrap().unwrap(),
                RetrievalStatsCounter {
                    total_count: 1,
                    active_count: 1,
                    current_count: 0
                }
            );
            assert_eq!(
                stats
                    .global_counter(RelationType::About, ObjectType::DerivedMemory)
                    .await
                    .unwrap()
                    .unwrap(),
                RetrievalStatsCounter {
                    total_count: 2,
                    active_count: 2,
                    current_count: 1
                }
            );
            let retrieve = RetrievePipeline::new(&graph, &vector, &embedder);
            let normal = retrieve
                .retrieve(RetrievalContext::new("concise answers").with_trace())
                .await
                .unwrap();
            assert!(!pack_contains_derived_memory(&normal.pack, predecessor_id));
            assert!(normal
                .trace
                .unwrap()
                .lifecycle_filter_decisions
                .iter()
                .any(|decision| decision.object.id == predecessor_id
                    && decision.reason == LifecycleFilterReason::SupersededOmitted));

            let retry = pipeline
                .commit(
                    successor_plan.clone(),
                    CommitOptions::default(),
                    &tokio::sync::Mutex::new(()),
                )
                .await
                .unwrap();
            assert!(retry.repair_needed.is_empty());
            let mut combined_plan = predecessor_plan;
            combined_plan.candidates.extend(successor_plan.candidates);
            let combined = pipeline
                .commit(
                    combined_plan,
                    CommitOptions::default(),
                    &tokio::sync::Mutex::new(()),
                )
                .await
                .unwrap();
            assert_eq!(combined.vector_indexed_object_ids, vec![successor_id]);
            let recall = vector.search_candidates(&search).await.unwrap();
            assert_eq!(
                recall
                    .candidates
                    .iter()
                    .map(|candidate| candidate.object_id)
                    .collect::<Vec<_>>(),
                vec![successor_id]
            );
            let mut history = RetrievalContext::new("concise answers").with_trace();
            history.lifecycle_policy.include_superseded = true;
            let historical = retrieve.retrieve(history).await.unwrap();
            assert!(pack_contains_derived_memory(
                &historical.pack,
                predecessor_id
            ));
            assert!(historical
                .trace
                .unwrap()
                .graph_relations
                .iter()
                .any(|relation| relation.from.id == successor_id
                    && relation.to.id == predecessor_id
                    && relation.relation == RelationType::Supersedes));

            let lifecycle = CorrectionForgetPipeline::new_with_stats(
                &graph,
                &vector,
                &embedder,
                stats.as_ref(),
            );
            lifecycle
                .forget(
                    ForgetMemoryDraft::suppress(
                        LifecycleTargetRef::DerivedMemory(successor_id),
                        "Forget correction.",
                    ),
                    &tokio::sync::Mutex::new(()),
                )
                .await
                .unwrap();
            assert_eq!(
                graph
                    .query_superseded_derived_memory_ids(&[predecessor_id])
                    .await
                    .unwrap(),
                vec![predecessor_id]
            );
            assert_eq!(
                graph
                    .query_objects(&GraphObjectQuery::by_ids(vec![predecessor_id]))
                    .await
                    .unwrap(),
                predecessor
            );
            assert_eq!(
                stats
                    .counter(&predecessor_counter)
                    .await
                    .unwrap()
                    .unwrap()
                    .current_count,
                0
            );
            assert_eq!(
                stats
                    .global_counter(RelationType::About, ObjectType::DerivedMemory)
                    .await
                    .unwrap()
                    .unwrap(),
                RetrievalStatsCounter {
                    total_count: 2,
                    active_count: 1,
                    current_count: 0
                }
            );
            let expansion = graph
                .expand_bounded(&GraphExpansionQuery::new(
                    predecessor_id,
                    ObjectType::DerivedMemory,
                    1,
                    10,
                ))
                .await
                .unwrap();
            assert!(expansion
                .filtered_nodes
                .iter()
                .any(|node| node.object_ref.id == predecessor_id
                    && node.reason == GraphExpansionFilteredReason::Superseded));

            lifecycle
                .forget(
                    ForgetMemoryDraft::suppress(
                        LifecycleTargetRef::DerivedMemory(predecessor_id),
                        "Also suppress history.",
                    ),
                    &tokio::sync::Mutex::new(()),
                )
                .await
                .unwrap();
            let expansion = graph
                .expand_bounded(&GraphExpansionQuery::new(
                    predecessor_id,
                    ObjectType::DerivedMemory,
                    1,
                    10,
                ))
                .await
                .unwrap();
            assert!(expansion
                .filtered_nodes
                .iter()
                .any(|node| node.object_ref.id == predecessor_id
                    && node.reason == GraphExpansionFilteredReason::Suppressed));
        }
    }

    #[tokio::test]
    async fn correction_rejects_absent_target_before_any_write() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::new(Vec::new()).await;
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let stats = RecordingStatsStore::default();
        let pipeline = CorrectionForgetPipeline::new_with_stats(&graph, &vector, &embedder, &stats);

        let error = pipeline
            .correct(correction_draft(&ids), &tokio::sync::Mutex::new(()))
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            CustomError::GraphExpansionRootNotFound {
                object_type: ObjectType::DerivedMemory,
                object_id,
            } if object_id == ids.old
        ));
        assert!(!graph
            .calls()
            .iter()
            .any(|call| matches!(call, StoreCall::GraphObjects(_) | StoreCall::GraphLinks(_))));
        assert!(graph
            .query_objects(&GraphObjectQuery::by_types(
                vec![ObjectType::DerivedMemory],
                None,
            ))
            .await
            .unwrap()
            .is_empty());
        assert!(vector.calls().is_empty());
        assert!(lock(&stats.calls).is_empty());
    }

    #[tokio::test]
    async fn correction_writes_graph_before_vector_maintenance_in_stable_order() {
        let ids = fixed_ids();
        let graph =
            RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))]).await;
        let vector = RecordingVectorStore {
            calls: graph.calls.clone(),
            ..RecordingVectorStore::default()
        };
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .correct(correction_draft(&ids), &tokio::sync::Mutex::new(()))
            .await
            .expect("correction should succeed");

        let calls = graph.calls();
        let last_graph_write = calls
            .iter()
            .rposition(|call| matches!(call, StoreCall::GraphObjects(_) | StoreCall::GraphLinks(_)))
            .unwrap();
        let first_vector_write = calls
            .iter()
            .position(|call| {
                matches!(
                    call,
                    StoreCall::VectorDelete(_) | StoreCall::VectorUpsert(_)
                )
            })
            .unwrap();
        assert!(last_graph_write < first_vector_write);
        assert_eq!(
            outcome.graph_mutated_object_ids,
            vec![MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                ids.replacement
            )]
        );
        assert_eq!(
            outcome.vector_maintained_object_ids,
            vec![
                MemoryObjectRef::new(ObjectType::DerivedMemory, ids.old),
                MemoryObjectRef::new(ObjectType::DerivedMemory, ids.replacement),
            ]
        );
        assert!(outcome.vector_maintenance_failure.is_none());
        let objects = graph
            .query_objects(&GraphObjectQuery::by_refs(vec![
                MemoryObjectRef::new(ObjectType::DerivedMemory, ids.old),
                MemoryObjectRef::new(ObjectType::DerivedMemory, ids.replacement),
            ]))
            .await
            .unwrap();
        assert!(objects.iter().any(|object| matches!(object,
            MemoryObject::DerivedMemory(memory) if memory.id == ids.old
                && memory.retention_state == RetentionState::Active
        )));
        assert!(objects.iter().any(|object| matches!(object,
            MemoryObject::DerivedMemory(memory) if memory.id == ids.replacement
                && memory.supersedes.contains(&ids.old)
        )));
        let links = graph
            .query_links_by_ids(&outcome.graph_mutated_link_ids)
            .await
            .unwrap();
        assert!(
            matches!(links.as_slice(), [link] if link.from_id == ids.replacement
            && link.to_id == ids.old && link.relation == RelationType::Supersedes)
        );
    }

    #[test]
    fn replacement_identity_uses_caller_content_and_excludes_execution_controls() {
        let ids = fixed_ids();
        let request = correction_draft(&ids);
        let seed = correction_seed(&request).unwrap();
        let mut execution_variant = request.clone();
        execution_variant.include_trace = true;
        execution_variant
            .cascade_policy
            .apply_to_provenanced_derived_memories = false;
        execution_variant.replacement_derived_memories[0].id = Some(Uuid::new_v4());

        assert_eq!(seed, correction_seed(&execution_variant).unwrap());
        let mut different_payload = request;
        different_payload.replacement_derived_memories[0]
            .text
            .push_str(" Updated.");
        assert_ne!(seed, correction_seed(&different_payload).unwrap());
        assert_ne!(
            replacement_memory_id(seed, 0),
            replacement_memory_id(seed, 1)
        );
    }

    #[tokio::test]
    async fn distinct_replacement_payloads_do_not_collide_in_same_store() {
        let ids = fixed_ids();
        let graph = in_memory_graph_store();
        graph
            .upsert_objects(&[
                MemoryObject::Episode(source_episode(&ids)),
                MemoryObject::DerivedMemory(old_memory(&ids)),
            ])
            .await
            .unwrap();
        let vector = TemporaryVectorCandidateStore::open(8).await;
        let embedder = DeterministicMemoryEmbedder::new(8);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let first = pipeline
            .correct(
                stateful_correction_draft(&ids, "First corrected payload."),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .expect("first correction should succeed");
        let second = pipeline
            .correct(
                stateful_correction_draft(&ids, "Second corrected payload."),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .expect("distinct correction should append a replacement");
        let first_link = graph
            .query_links_by_ids(&first.graph_mutated_link_ids)
            .await
            .unwrap();
        let second_link = graph
            .query_links_by_ids(&second.graph_mutated_link_ids)
            .await
            .unwrap();

        assert_eq!(first_link.len(), 1);
        assert_eq!(second_link.len(), 1);
        assert_ne!(first_link[0].from_id, second_link[0].from_id);
        let replacements = graph
            .query_objects(&GraphObjectQuery::by_refs(vec![
                MemoryObjectRef::new(ObjectType::DerivedMemory, first_link[0].from_id),
                MemoryObjectRef::new(ObjectType::DerivedMemory, second_link[0].from_id),
            ]))
            .await
            .unwrap();
        assert_eq!(replacements.len(), 2);
    }

    #[tokio::test]
    async fn identical_same_store_retry_repairs_without_graph_mutation() {
        let ids = fixed_ids();
        let graph = in_memory_graph_store();
        graph
            .upsert_objects(&[
                MemoryObject::Episode(source_episode(&ids)),
                MemoryObject::DerivedMemory(old_memory(&ids)),
            ])
            .await
            .unwrap();
        let vector = TemporaryVectorCandidateStore::open(8).await;
        let embedder = DeterministicMemoryEmbedder::new(8);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let draft = stateful_correction_draft(&ids, "Stable corrected payload.");
        let replacement_id = replacement_memory_id(correction_seed(&draft).unwrap(), 0);

        let first = pipeline
            .correct(draft.clone(), &tokio::sync::Mutex::new(()))
            .await
            .expect("first correction should succeed");
        let second = pipeline
            .correct(draft, &tokio::sync::Mutex::new(()))
            .await
            .expect("identical retry should converge");

        assert_eq!(first.graph_mutated_link_ids.len(), 1);
        assert_converged_repair_outcome(
            &second,
            &[ids.old, replacement_id],
            &[ids.old, replacement_id],
        );
    }

    #[tokio::test]
    async fn identical_direct_target_retry_replays_repairs_without_graph_mutation() {
        let ids = fixed_ids();
        let graph = in_memory_graph_store();
        graph
            .upsert_objects(&[MemoryObject::DerivedMemory(old_memory(&ids))])
            .await
            .unwrap();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let stats = RecordingStatsStore::default();
        let pipeline = CorrectionForgetPipeline::new_with_stats(&graph, &vector, &embedder, &stats);
        let mut draft = correction_draft(&ids).with_trace();
        draft.replacement_derived_memories[0].id = None;

        let first = pipeline
            .correct(draft.clone(), &tokio::sync::Mutex::new(()))
            .await
            .expect("first direct-target correction should succeed");
        let replacement_id = graph
            .query_links_by_ids(&first.graph_mutated_link_ids)
            .await
            .unwrap()[0]
            .from_id;
        let refs = vec![
            MemoryObjectRef::new(ObjectType::DerivedMemory, ids.old),
            MemoryObjectRef::new(ObjectType::DerivedMemory, replacement_id),
        ];
        let objects_after_first = graph
            .query_objects(&GraphObjectQuery::by_refs(refs.clone()))
            .await
            .unwrap();
        let links_after_first = graph
            .query_links_by_ids(&first.graph_mutated_link_ids)
            .await
            .unwrap();

        let second = pipeline
            .correct(draft, &tokio::sync::Mutex::new(()))
            .await
            .expect("identical direct-target retry should converge");

        assert_converged_repair_outcome(
            &second,
            &[ids.old, replacement_id],
            &[ids.old, replacement_id],
        );
        assert_eq!(
            graph
                .query_objects(&GraphObjectQuery::by_refs(refs))
                .await
                .unwrap(),
            objects_after_first
        );
        assert_eq!(
            graph
                .query_links_by_ids(&first.graph_mutated_link_ids)
                .await
                .unwrap(),
            links_after_first
        );
    }

    #[tokio::test]
    async fn identical_retry_repairs_one_shot_vector_failure_without_graph_writes() {
        let ids = fixed_ids();
        let old = old_memory(&ids);
        let graph = RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old.clone())]).await;
        let vector = OneShotDeleteFailingVectorStore::new().await;
        let old_record = memory_object_vector_record(&MemoryObject::DerivedMemory(old)).unwrap();
        vector
            .inner
            .upsert_vector_records(&[VectorRecordEmbedding::new(
                &old_record,
                &[1.0, 0.0, 0.0, 0.0],
            )])
            .await
            .unwrap();
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft = correction_draft(&ids);
        draft.replacement_derived_memories[0].id = None;
        let replacement_id = replacement_memory_id(correction_seed(&draft).unwrap(), 0);

        let first = pipeline
            .correct(draft.clone(), &tokio::sync::Mutex::new(()))
            .await
            .expect("first correction should preserve vector failure in its outcome");
        assert!(first.vector_maintenance_failure.is_some());
        let graph_writes_after_first = graph_write_count(&graph.calls());
        let candidates_after_first = vector
            .search_candidates(&VectorCandidateSearch::new(
                vec![1.0, 0.0, 0.0, 0.0],
                10,
                vec![ObjectType::DerivedMemory],
            ))
            .await
            .unwrap();
        assert!(candidates_after_first
            .candidates
            .iter()
            .any(|candidate| candidate.object_id == ids.old));

        let retry = pipeline
            .correct(draft, &tokio::sync::Mutex::new(()))
            .await
            .expect("identical retry should repair vector state");

        assert_eq!(graph_write_count(&graph.calls()), graph_writes_after_first);
        assert!(retry.graph_mutated_object_ids.is_empty());
        assert!(retry.graph_mutated_link_ids.is_empty());
        assert!(retry.vector_maintenance_failure.is_none());
        let maintained_ids = retry
            .vector_maintained_object_ids
            .iter()
            .map(|object_ref| object_ref.id)
            .collect::<Vec<_>>();
        assert!(maintained_ids.contains(&ids.old));
        assert!(maintained_ids.contains(&replacement_id));
        let candidates_after_retry = vector
            .search_candidates(&VectorCandidateSearch::new(
                vec![1.0, 0.0, 0.0, 0.0],
                10,
                vec![ObjectType::DerivedMemory],
            ))
            .await
            .unwrap();
        assert!(!candidates_after_retry
            .candidates
            .iter()
            .any(|candidate| candidate.object_id == ids.old));
        assert!(candidates_after_retry
            .candidates
            .iter()
            .any(|candidate| candidate.object_id == replacement_id));
    }

    #[tokio::test]
    async fn identical_retry_repairs_one_shot_stats_failure_without_graph_writes() {
        let ids = fixed_ids();
        let entity_id = MemoryId::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_8301);
        let mut old = old_memory(&ids);
        old.entity_ids = vec![entity_id];
        let mut notion = representative_fixtures().user_entity;
        notion.id = entity_id;
        let graph = RecordingGraphStore::new(vec![
            MemoryObject::DerivedMemory(old),
            MemoryObject::Entity(notion),
        ])
        .await;
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let stats = OneShotEdgeFailingStatsStore::new();
        let pipeline = CorrectionForgetPipeline::new_with_stats(&graph, &vector, &embedder, &stats);
        let mut draft = correction_draft(&ids);
        draft.replacement_derived_memories[0].id = None;

        let first = pipeline
            .correct(draft.clone(), &tokio::sync::Mutex::new(()))
            .await
            .expect("first correction should preserve stats failure in its outcome");
        assert!(first.stats_update_status.failure.is_some());
        let counter_key = RetrievalStatsCounterKey {
            entity_id,
            relation_kind: RelationType::About,
            object_type: ObjectType::DerivedMemory,
        };
        assert!(stats.counter(&counter_key).await.unwrap().is_none());
        let graph_writes_after_first = graph_write_count(&graph.calls());

        let retry = pipeline
            .correct(draft, &tokio::sync::Mutex::new(()))
            .await
            .expect("identical retry should repair stats state");

        assert_eq!(graph_write_count(&graph.calls()), graph_writes_after_first);
        assert!(retry.graph_mutated_object_ids.is_empty());
        assert!(retry.graph_mutated_link_ids.is_empty());
        let retry_failure = retry
            .stats_update_status
            .failure
            .expect("persisted unhealthy state remains caller-visible after repair");
        assert!(!retry_failure
            .causes
            .iter()
            .any(|cause| matches!(cause, StatsUpdateCause::EdgeWrite { .. })));
        assert!(retry_failure
            .causes
            .iter()
            .any(|cause| matches!(cause, StatsUpdateCause::StoreUnhealthy { .. })));
        let counter = stats
            .counter(&counter_key)
            .await
            .unwrap()
            .expect("retry should rebuild the dropped stats edges");
        assert_eq!(counter.total_count, 2);
        assert_eq!(counter.active_count, 2);
        assert_eq!(counter.current_count, 1);
    }

    #[tokio::test]
    async fn multi_replacement_retry_preserves_each_replacements_lineage() {
        let ids = fixed_ids();
        let graph = in_memory_graph_store();
        graph
            .upsert_objects(&[
                MemoryObject::Episode(source_episode(&ids)),
                MemoryObject::DerivedMemory(old_memory(&ids)),
            ])
            .await
            .unwrap();
        let vector = TemporaryVectorCandidateStore::open(8).await;
        let embedder = DeterministicMemoryEmbedder::new(8);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let first_ancestor = MemoryId::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_8201);
        let second_ancestor = MemoryId::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_8202);
        let ancestors = [first_ancestor, second_ancestor].map(|id| {
            let mut ancestor = old_memory(&ids);
            ancestor.id = id;
            ancestor.derived_from_episode_ids = vec![MemoryId::from_u128(999)];
            ancestor.derived_from_observation_ids.clear();
            MemoryObject::DerivedMemory(ancestor)
        });
        graph.upsert_objects(&ancestors).await.unwrap();
        let mut draft = stateful_correction_draft(&ids, "First replacement payload.");
        draft.replacement_derived_memories[0].supersedes = vec![first_ancestor];
        let mut second_replacement = draft.replacement_derived_memories[0].clone();
        second_replacement.text = "Second replacement payload.".to_owned();
        second_replacement.supersedes = vec![second_ancestor];
        draft.replacement_derived_memories.push(second_replacement);
        let seed = correction_seed(&draft).unwrap();
        let replacement_ids = [
            replacement_memory_id(seed, 0),
            replacement_memory_id(seed, 1),
        ];

        pipeline
            .correct(draft.clone(), &tokio::sync::Mutex::new(()))
            .await
            .expect("first multi-replacement correction should succeed");
        let stored_after_first = graph
            .query_objects(&GraphObjectQuery::by_refs(
                replacement_ids
                    .iter()
                    .map(|id| MemoryObjectRef::new(ObjectType::DerivedMemory, *id))
                    .collect(),
            ))
            .await
            .unwrap();
        assert_replacement_lineage(
            &stored_after_first,
            replacement_ids[0],
            first_ancestor,
            second_ancestor,
        );
        assert_replacement_lineage(
            &stored_after_first,
            replacement_ids[1],
            second_ancestor,
            first_ancestor,
        );

        let retry = pipeline
            .correct(draft, &tokio::sync::Mutex::new(()))
            .await
            .expect("multi-replacement retry should converge independently");

        let expected_ids = [
            replacement_ids[0],
            replacement_ids[1],
            ids.old,
            first_ancestor,
            second_ancestor,
        ];
        assert_converged_repair_outcome(&retry, &expected_ids, &expected_ids);
        assert_eq!(
            graph
                .query_objects(&GraphObjectQuery::by_refs(
                    replacement_ids
                        .iter()
                        .map(|id| MemoryObjectRef::new(ObjectType::DerivedMemory, *id))
                        .collect(),
                ))
                .await
                .unwrap(),
            stored_after_first
        );
    }

    #[tokio::test]
    async fn duplicate_replacement_ids_reject_before_oxigraph_side_effects() {
        let ids = fixed_ids();
        let oxigraph = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let oxigraph_original = old_memory(&ids);
        oxigraph
            .upsert_objects(&[MemoryObject::DerivedMemory(oxigraph_original.clone())])
            .await
            .unwrap();
        assert_duplicate_replacement_rejection(&oxigraph, &ids).await;
    }

    #[tokio::test]
    async fn existing_divergent_replacement_content_is_rejected_before_writes() {
        let ids = fixed_ids();
        let mut draft = correction_draft(&ids);
        draft.replacement_derived_memories[0].id = None;
        let replacement_id = replacement_memory_id(correction_seed(&draft).unwrap(), 0);
        let mut divergent = old_memory(&ids);
        divergent.id = replacement_id;
        divergent.text = "Divergent stored replacement.".to_owned();
        let graph = RecordingGraphStore::new(vec![
            MemoryObject::DerivedMemory(old_memory(&ids)),
            MemoryObject::DerivedMemory(divergent),
        ])
        .await;
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();

        let error = CorrectionForgetPipeline::new(&graph, &vector, &embedder)
            .correct(draft, &tokio::sync::Mutex::new(()))
            .await
            .expect_err("divergent content under a deterministic replacement ID must reject");

        let CustomError::ReplacementIdentityConflict(conflict) = error else {
            panic!("expected structured replacement identity conflict");
        };
        assert_eq!(conflict.replacement_id, replacement_id);
        assert_eq!(
            conflict.conflict,
            ReplacementIdentityConflict::DivergentExisting
        );
        assert!(!graph
            .calls()
            .iter()
            .any(|call| matches!(call, StoreCall::GraphObjects(_))));
    }

    async fn assert_duplicate_replacement_rejection<G>(graph: &G, ids: &FixedIds)
    where
        G: GraphAuthorityStore + ?Sized,
    {
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let mut draft = correction_draft(ids);
        let mut duplicate = draft.replacement_derived_memories[0].clone();
        duplicate.text = "Divergent duplicate payload.".to_owned();
        draft.replacement_derived_memories.push(duplicate);
        let object_refs = vec![
            MemoryObjectRef::new(ObjectType::DerivedMemory, ids.old),
            MemoryObjectRef::new(ObjectType::DerivedMemory, ids.replacement),
        ];
        let link_id =
            derived_memory_links(&current_replacement_from(&old_memory(ids), ids.replacement))[0]
                .id;
        let objects_before = graph
            .query_objects(&GraphObjectQuery::by_refs(object_refs.clone()))
            .await
            .unwrap();
        let links_before = graph.query_links_by_ids(&[link_id]).await.unwrap();

        let error = CorrectionForgetPipeline::new(graph, &vector, &embedder)
            .correct(draft, &tokio::sync::Mutex::new(()))
            .await
            .expect_err("duplicate replacement IDs must reject before writes");

        let CustomError::ReplacementIdentityConflict(conflict) = error else {
            panic!("expected structured replacement identity conflict");
        };
        assert_eq!(conflict.replacement_id, ids.replacement);
        assert_eq!(
            conflict.conflict,
            ReplacementIdentityConflict::DuplicateInPlan
        );
        assert_eq!(
            graph
                .query_objects(&GraphObjectQuery::by_refs(object_refs))
                .await
                .unwrap(),
            objects_before
        );
        assert_eq!(
            graph.query_links_by_ids(&[link_id]).await.unwrap(),
            links_before
        );
        assert!(vector.calls().is_empty());
    }

    fn assert_converged_repair_outcome(
        outcome: &LifecycleMutationOutcome,
        expected_vector_ids: &[MemoryId],
        expected_stats_ids: &[MemoryId],
    ) {
        assert!(outcome.graph_mutated_object_ids.is_empty());
        assert!(outcome.graph_mutated_link_ids.is_empty());
        assert!(outcome.vector_maintenance_failure.is_none());
        assert!(outcome.stats_update_status.failure.is_none());
        let mut actual_vector_ids = outcome
            .vector_maintained_object_ids
            .iter()
            .map(|object_ref| object_ref.id)
            .collect::<Vec<_>>();
        actual_vector_ids.sort();
        let mut expected_vector_ids = expected_vector_ids.to_vec();
        expected_vector_ids.sort();
        assert_eq!(actual_vector_ids, expected_vector_ids);
        let mut actual_stats_ids = outcome.stats_update_status.updated_object_ids.clone();
        actual_stats_ids.sort();
        let mut expected_stats_ids = expected_stats_ids.to_vec();
        expected_stats_ids.sort();
        assert_eq!(actual_stats_ids, expected_stats_ids);
        if let Some(trace) = &outcome.trace {
            assert!(trace.superseded_by.is_empty());
        }
    }

    fn assert_replacement_lineage(
        objects: &[MemoryObject],
        replacement_id: MemoryId,
        expected_ancestor: MemoryId,
        excluded_ancestor: MemoryId,
    ) {
        let replacement = objects
            .iter()
            .find_map(|object| match object {
                MemoryObject::DerivedMemory(memory) if memory.id == replacement_id => Some(memory),
                _ => None,
            })
            .expect("replacement should be stored");
        assert!(replacement.supersedes.contains(&expected_ancestor));
        assert!(!replacement.supersedes.contains(&excluded_ancestor));
    }

    fn graph_write_count(calls: &[StoreCall]) -> usize {
        calls
            .iter()
            .filter(|call| matches!(call, StoreCall::GraphObjects(_) | StoreCall::GraphLinks(_)))
            .count()
    }

    #[tokio::test]
    async fn correction_outcome_preserves_all_stats_failures() {
        let ids = fixed_ids();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let graph = RecordingGraphStore {
            calls: calls.clone(),
            ..RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))]).await
        };
        let vector = RecordingVectorStore {
            calls: calls.clone(),
            fail_delete: false,
        };
        let embedder = RecordingEmbedder {
            calls: calls.clone(),
        };
        let stats = RecordingStatsStore {
            calls: calls.clone(),
            fail_edges: true,
            fail_object_states: true,
        };
        let pipeline = CorrectionForgetPipeline::new_with_stats(&graph, &vector, &embedder, &stats);

        let outcome = pipeline
            .correct(correction_draft(&ids), &tokio::sync::Mutex::new(()))
            .await
            .expect("stats failure should not change lifecycle outcome");

        assert!(outcome.vector_maintenance_failure.is_none());
        assert_stats_failures(&outcome.stats_update_status, &[ids.replacement, ids.old]);
        let calls = lock(&calls);
        let last_vector_write = calls
            .iter()
            .rposition(|call| {
                matches!(
                    call,
                    StoreCall::VectorDelete(_) | StoreCall::VectorUpsert(_)
                )
            })
            .unwrap();
        let first_stats_write = calls
            .iter()
            .position(|call| {
                matches!(
                    call,
                    StoreCall::StatsEdges(_) | StoreCall::StatsObjectStates(_)
                )
            })
            .unwrap();
        assert!(last_vector_write < first_stats_write);
    }

    #[tokio::test]
    async fn forget_records_stats_after_vector_maintenance() {
        let ids = fixed_ids();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let graph = RecordingGraphStore {
            calls: calls.clone(),
            ..RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))]).await
        };
        let vector = RecordingVectorStore {
            calls: calls.clone(),
            fail_delete: false,
        };
        let embedder = RecordingEmbedder {
            calls: calls.clone(),
        };
        let stats = RecordingStatsStore {
            calls: calls.clone(),
            fail_edges: false,
            fail_object_states: false,
        };
        let pipeline = CorrectionForgetPipeline::new_with_stats(&graph, &vector, &embedder, &stats);

        let outcome = pipeline
            .forget(
                ForgetMemoryDraft::suppress(
                    LifecycleTargetRef::DerivedMemory(ids.old),
                    "Suppress stale derived memory.",
                ),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .expect("forget should record stats after vector maintenance");

        assert!(outcome.vector_maintenance_failure.is_none());
        assert_eq!(
            outcome.stats_update_status.updated_object_ids,
            vec![ids.old]
        );
        assert!(outcome.stats_update_status.failure.is_none());
        let calls = lock(&calls);
        let last_vector_write = calls
            .iter()
            .rposition(|call| {
                matches!(
                    call,
                    StoreCall::VectorDelete(_) | StoreCall::VectorUpsert(_)
                )
            })
            .unwrap();
        let first_stats_write = calls
            .iter()
            .position(|call| {
                matches!(
                    call,
                    StoreCall::StatsEdges(_) | StoreCall::StatsObjectStates(_)
                )
            })
            .unwrap();
        assert!(last_vector_write < first_stats_write);
    }

    #[tokio::test]
    async fn forget_outcome_preserves_all_stats_failures() {
        let ids = fixed_ids();
        let graph =
            RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))]).await;
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let stats = RecordingStatsStore {
            fail_edges: true,
            fail_object_states: true,
            ..RecordingStatsStore::default()
        };
        let pipeline = CorrectionForgetPipeline::new_with_stats(&graph, &vector, &embedder, &stats);

        let outcome = pipeline
            .forget(
                ForgetMemoryDraft::suppress(
                    LifecycleTargetRef::DerivedMemory(ids.old),
                    "Suppress stale derived memory.",
                ),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .expect("stats degradation should remain a repairable lifecycle outcome");

        assert_stats_failures(&outcome.stats_update_status, &[ids.old]);
    }

    fn assert_stats_failures(
        status: &crate::api::types::StatsUpdateStatus,
        expected_object_ids: &[MemoryId],
    ) {
        assert!(status.updated_object_ids.is_empty());
        let failure = status
            .failure
            .as_ref()
            .expect("stats failure must be visible");
        assert_eq!(failure.failed_object_ids, expected_object_ids);
        assert!(matches!(
            failure.causes.as_slice(),
            [
                StatsUpdateCause::EdgeWrite { .. },
                StatsUpdateCause::ObjectStateWrite { .. }
            ]
        ));
    }

    #[tokio::test]
    async fn validation_failure_prevents_graph_and_vector_writes() {
        let ids = fixed_ids();
        let graph =
            RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))]).await;
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft = correction_draft(&ids);
        draft.rationale = " ".to_owned();

        let error = pipeline
            .correct(draft, &tokio::sync::Mutex::new(()))
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            CustomError::LifecycleDraftInvalid(
                crate::domain::LifecycleDtoValidationError::EmptyRationale
            )
        ));
        assert!(graph.calls().is_empty());
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn graph_failure_prevents_vector_maintenance() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))])
            .await
            .fail_objects();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let error = pipeline
            .correct(correction_draft(&ids), &tokio::sync::Mutex::new(()))
            .await
            .unwrap_err();

        assert!(matches!(error, CustomError::DatabaseError(_)));
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn graph_link_failure_prevents_partial_object_mutation_and_vector_maintenance() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))])
            .await
            .fail_links();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let error = pipeline
            .correct(correction_draft(&ids), &tokio::sync::Mutex::new(()))
            .await
            .unwrap_err();

        assert!(matches!(error, CustomError::DatabaseError(_)));
        assert_eq!(
            graph
                .store
                .query_objects(&GraphObjectQuery::by_ids(vec![ids.old, ids.replacement]))
                .await
                .unwrap()
                .iter()
                .map(MemoryObject::object_ref)
                .collect::<Vec<_>>(),
            vec![MemoryObjectRef::new(ObjectType::DerivedMemory, ids.old)]
        );
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn vector_failure_after_graph_success_returns_partial_outcome() {
        let ids = fixed_ids();
        let graph =
            RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))]).await;
        let vector = RecordingVectorStore::default().fail_delete();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .correct(correction_draft(&ids), &tokio::sync::Mutex::new(()))
            .await
            .expect("graph success should return partial vector failure");

        assert_eq!(
            outcome.graph_mutated_object_ids,
            vec![MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                ids.replacement
            )]
        );
        assert_eq!(
            outcome.vector_maintained_object_ids,
            vec![MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                ids.replacement
            )]
        );
        let failure = outcome
            .vector_maintenance_failure
            .expect("delete failure should be explicit");
        assert_eq!(
            failure.unmaintained_objects(),
            vec![MemoryObjectRef::new(ObjectType::DerivedMemory, ids.old)]
        );
        assert_eq!(failure.failures.len(), 1);
        assert_eq!(
            failure.failures[0].operation,
            VectorMaintenanceOperation::Delete
        );
        assert!(matches!(
            &failure.failures[0].cause,
            VectorIndexingCause::VectorDatabase(VectorDatabaseError {
                backend,
                kind: VectorDatabaseErrorKind::Response,
                ..
            }) if backend == "test"
        ));
    }

    #[tokio::test]
    async fn source_object_correction_supersedes_provenanced_memories_without_rewriting_source() {
        let fixtures = representative_fixtures();
        let graph = in_memory_graph_store();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = TemporaryVectorCandidateStore::open(4).await;
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let replacement_id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_9100);
        let mut replacement = ReplacementDerivedMemoryDraft::new(
            DerivedType::Correction,
            "The corrected source changes the behavioral note.",
        )
        .with_source_observation(fixtures.salient_observation.id);
        replacement.id = Some(replacement_id);
        replacement.original_source_provenance =
            SourceProvenanceReference::episode(fixtures.episode.id);
        replacement.correction_origin_provenance =
            SourceProvenanceReference::observation(fixtures.salient_observation.id)
                .with_external_ref(ExternalSourceReference::raw("raw://correction/1"));
        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::source_object(SourceObjectCorrectionTarget::Episode {
                id: fixtures.episode.id,
                original_raw_ref: fixtures.episode.raw_ref.clone(),
                original_setting_key: fixtures.episode.scene.setting.key.clone(),
            }),
            "Correct episode-derived behavior.",
        )
        .with_replacement(replacement)
        .with_trace();
        draft.correction_origin =
            SourceProvenanceReference::observation(fixtures.salient_observation.id);

        let outcome = pipeline
            .correct(draft, &tokio::sync::Mutex::new(()))
            .await
            .unwrap();

        assert!(outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                replacement_id
            )));
        assert!(!outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::Episode,
                fixtures.episode.id
            )));
        let objects = graph
            .query_objects(&GraphObjectQuery::by_refs(vec![
                MemoryObjectRef::from_id_type(fixtures.episode.id, ObjectType::Episode),
                MemoryObjectRef::from_id_type(
                    fixtures.user_preference.id,
                    ObjectType::DerivedMemory,
                ),
                MemoryObjectRef::from_id_type(replacement_id, ObjectType::DerivedMemory),
            ]))
            .await
            .unwrap();
        assert!(objects.contains(&MemoryObject::Episode(fixtures.episode.clone())));
        let old = objects
            .iter()
            .find_map(|object| match object {
                MemoryObject::DerivedMemory(memory) if memory.id == fixtures.user_preference.id => {
                    Some(memory)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(old, &fixtures.user_preference);
        let replacement = objects
            .iter()
            .find_map(|object| match object {
                MemoryObject::DerivedMemory(memory) if memory.id == replacement_id => Some(memory),
                _ => None,
            })
            .unwrap();
        assert!(replacement
            .derived_from_episode_ids
            .contains(&fixtures.episode.id));
        assert!(replacement
            .derived_from_observation_ids
            .contains(&fixtures.salient_observation.id));
        assert!(outcome.trace.unwrap().superseded_by.iter().any(|evidence| {
            evidence.superseded_memory_id == fixtures.user_preference.id
                && evidence.superseded_by_memory_id == replacement_id
        }));
    }

    #[tokio::test]
    async fn episode_correction_supersedes_observation_only_derived_memories() {
        let fixtures = representative_fixtures();
        let mut observation_only = fixtures.user_preference.clone();
        observation_only.id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_9101);
        observation_only.derived_from_episode_ids.clear();
        observation_only.derived_from_observation_ids = vec![fixtures.salient_observation.id];
        let graph = in_memory_graph_store();
        let mut objects = fixtures.objects();
        objects.push(MemoryObject::DerivedMemory(observation_only.clone()));
        graph.upsert_objects(&objects).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = TemporaryVectorCandidateStore::open(4).await;
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let replacement_id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_9102);
        let mut replacement = ReplacementDerivedMemoryDraft::new(
            DerivedType::Correction,
            "The corrected episode supersedes observation-only behavior.",
        )
        .with_source_episode(fixtures.episode.id);
        replacement.id = Some(replacement_id);
        replacement.original_source_provenance =
            SourceProvenanceReference::episode(fixtures.episode.id);
        replacement.correction_origin_provenance =
            SourceProvenanceReference::observation(fixtures.salient_observation.id);
        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::source_object(SourceObjectCorrectionTarget::Episode {
                id: fixtures.episode.id,
                original_raw_ref: fixtures.episode.raw_ref.clone(),
                original_setting_key: fixtures.episode.scene.setting.key.clone(),
            }),
            "Correct episode-derived observation-only behavior.",
        )
        .with_replacement(replacement);
        draft.correction_origin =
            SourceProvenanceReference::observation(fixtures.salient_observation.id);

        let outcome = pipeline
            .correct(draft, &tokio::sync::Mutex::new(()))
            .await
            .unwrap();

        assert!(!outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                observation_only.id,
            )));
        let objects = graph
            .query_objects(&GraphObjectQuery::by_refs(vec![
                MemoryObjectRef::from_id_type(observation_only.id, ObjectType::DerivedMemory),
                MemoryObjectRef::from_id_type(replacement_id, ObjectType::DerivedMemory),
            ]))
            .await
            .unwrap();
        let old = objects
            .iter()
            .find_map(|object| match object {
                MemoryObject::DerivedMemory(memory) if memory.id == observation_only.id => {
                    Some(memory)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(old, &observation_only);
        let replacement = objects
            .iter()
            .find_map(|object| match object {
                MemoryObject::DerivedMemory(memory) if memory.id == replacement_id => Some(memory),
                _ => None,
            })
            .unwrap();
        assert!(replacement.supersedes.contains(&observation_only.id));
    }

    #[tokio::test]
    async fn source_object_correction_requires_original_refs_before_writes() {
        let ids = fixed_ids();
        let graph =
            RecordingGraphStore::new(vec![MemoryObject::Episode(source_episode(&ids))]).await;
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::source_object(SourceObjectCorrectionTarget::Episode {
                id: ids.episode,
                original_raw_ref: None,
                original_setting_key: None,
            }),
            "Correct source episode.",
        );
        draft.correction_origin = SourceProvenanceReference::episode(ids.episode);

        let error = pipeline
            .correct(draft, &tokio::sync::Mutex::new(()))
            .await
            .unwrap_err();

        assert!(
            matches!(error, CustomError::MissingOriginalSourceReference { target }
            if target == MemoryObjectRef::new(ObjectType::Episode, ids.episode))
        );
        assert!(graph.calls().is_empty());
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn source_object_correction_rejects_mismatched_episode_refs_before_writes() {
        let ids = fixed_ids();
        let graph =
            RecordingGraphStore::new(vec![MemoryObject::Episode(source_episode(&ids))]).await;
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::source_object(SourceObjectCorrectionTarget::Episode {
                id: ids.episode,
                original_raw_ref: Some("raw://wrong".to_owned()),
                original_setting_key: Some("conversation://original".to_owned()),
            }),
            "Correct source episode.",
        );
        draft.correction_origin = SourceProvenanceReference::episode(ids.episode);

        let error = pipeline
            .correct(draft, &tokio::sync::Mutex::new(()))
            .await
            .unwrap_err();

        assert!(
            matches!(error, CustomError::OriginalSourceReferenceMismatch {
            target, kind: SourceReferenceKind::Raw, provided, stored,
        } if target == MemoryObjectRef::new(ObjectType::Episode, ids.episode)
            && provided == "raw://wrong" && stored.as_deref() == Some("raw://original/episode"))
        );
        assert_eq!(graph_write_count(&graph.calls()), 0);
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn source_object_correction_rejects_mismatched_observation_source_ref_before_writes() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::new(vec![
            MemoryObject::Episode(source_episode(&ids)),
            MemoryObject::Observation(source_observation(&ids)),
        ])
        .await;
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::source_object(SourceObjectCorrectionTarget::Observation {
                id: ids.observation,
                original_raw_ref: Some("raw://original/observation".to_owned()),
                original_setting_key: Some("conversation://wrong".to_owned()),
            }),
            "Correct source observation.",
        );
        draft.correction_origin = SourceProvenanceReference::observation(ids.observation);

        let error = pipeline
            .correct(draft, &tokio::sync::Mutex::new(()))
            .await
            .unwrap_err();

        assert!(
            matches!(error, CustomError::OriginalSourceReferenceMismatch {
            target, kind: SourceReferenceKind::SettingKey, provided, stored,
        } if target == MemoryObjectRef::new(ObjectType::Observation, ids.observation)
            && provided == "conversation://wrong" && stored.as_deref() == Some("conversation://original"))
        );
        assert_eq!(graph_write_count(&graph.calls()), 0);
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn source_correction_supersedes_current_replacement_without_suppression() {
        let fixtures = representative_fixtures();
        let current_replacement_id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_9201);
        let next_replacement_id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_9202);
        let current_replacement =
            current_replacement_from(&fixtures.user_preference, current_replacement_id);
        let graph = in_memory_graph_store();
        let mut objects = fixtures.objects();
        objects.push(MemoryObject::DerivedMemory(current_replacement.clone()));
        graph.upsert_objects(&objects).await.unwrap();
        let mut links = fixtures.links();
        links.extend(derived_memory_links(&current_replacement));
        graph.upsert_links(&links).await.unwrap();
        let vector = TemporaryVectorCandidateStore::open(4).await;
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut replacement = ReplacementDerivedMemoryDraft::new(
            DerivedType::Correction,
            "The next correction remains explicit.",
        )
        .with_source_episode(fixtures.episode.id);
        replacement.id = Some(next_replacement_id);
        replacement.original_source_provenance =
            SourceProvenanceReference::episode(fixtures.episode.id);
        replacement.correction_origin_provenance =
            SourceProvenanceReference::observation(fixtures.salient_observation.id);
        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::source_object(SourceObjectCorrectionTarget::Episode {
                id: fixtures.episode.id,
                original_raw_ref: fixtures.episode.raw_ref.clone(),
                original_setting_key: fixtures.episode.scene.setting.key.clone(),
            }),
            "Correct the source again.",
        )
        .with_replacement(replacement);
        draft.correction_origin =
            SourceProvenanceReference::observation(fixtures.salient_observation.id);

        let outcome = pipeline
            .correct(draft, &tokio::sync::Mutex::new(()))
            .await
            .unwrap();

        assert!(outcome.diagnostics.warnings.is_empty());
        assert!(!outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                current_replacement_id,
            )));
        assert!(outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                next_replacement_id,
            )));
    }

    #[tokio::test]
    async fn forget_cascade_warns_without_changing_existing_outcome_fields() {
        let fixtures = representative_fixtures();
        let current_replacement_id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_9203);
        let current_replacement =
            current_replacement_from(&fixtures.user_preference, current_replacement_id);
        let graph = in_memory_graph_store();
        let mut objects = fixtures.objects();
        objects.push(MemoryObject::DerivedMemory(current_replacement.clone()));
        graph.upsert_objects(&objects).await.unwrap();
        let mut links = fixtures.links();
        links.extend(derived_memory_links(&current_replacement));
        graph.upsert_links(&links).await.unwrap();
        let vector = TemporaryVectorCandidateStore::open(4).await;
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .forget(
                ForgetMemoryDraft::suppress(
                    LifecycleTargetRef::Episode(fixtures.episode.id),
                    "Forget the source with its current replacement.",
                ),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .unwrap();

        assert_eq!(
            outcome.diagnostics.warnings,
            vec![LifecycleMutationWarning {
                reason: LifecycleMutationWarningReason::CascadeSuppressesCurrentReplacement,
                affected_memory_ids: vec![fixtures.correction.id, current_replacement_id],
            }]
        );
        assert_eq!(
            outcome
                .graph_mutated_object_ids
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>(),
            [
                MemoryObjectRef::new(ObjectType::Episode, fixtures.episode.id),
                MemoryObjectRef::new(ObjectType::DerivedMemory, fixtures.derived_reflection.id),
                MemoryObjectRef::new(ObjectType::DerivedMemory, fixtures.open_loop.id),
                MemoryObjectRef::new(ObjectType::DerivedMemory, fixtures.commitment.id),
                MemoryObjectRef::new(ObjectType::DerivedMemory, fixtures.correction.id),
                MemoryObjectRef::new(ObjectType::DerivedMemory, current_replacement_id),
            ]
            .into_iter()
            .collect()
        );
        assert!(outcome.graph_mutated_link_ids.is_empty());
        assert_eq!(
            outcome
                .vector_maintained_object_ids
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>(),
            outcome.graph_mutated_object_ids.iter().copied().collect()
        );
        assert!(outcome.vector_maintenance_failure.is_none());
        assert!(outcome.trace.is_none());
    }

    #[tokio::test]
    async fn forget_suppresses_source_and_dependent_derived_memories_and_deletes_vectors() {
        let fixtures = representative_fixtures();
        let graph = in_memory_graph_store();
        let mut objects = fixtures.objects();
        for object in &mut objects {
            if let MemoryObject::DerivedMemory(memory) = object {
                memory.supersedes.clear();
            }
        }
        graph.upsert_objects(&objects).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = TemporaryVectorCandidateStore::open(4).await;
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .forget(
                ForgetMemoryDraft::suppress(
                    LifecycleTargetRef::Observation(fixtures.salient_observation.id),
                    "Forget source observation.",
                ),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .unwrap();

        assert!(outcome.diagnostics.warnings.is_empty());
        assert!(outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::Observation,
                fixtures.salient_observation.id,
            )));
        assert!(outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                fixtures.user_preference.id,
            )));
        assert_eq!(
            outcome.vector_maintained_object_ids,
            outcome.graph_mutated_object_ids
        );
        let objects = graph
            .query_objects(&GraphObjectQuery::by_refs(vec![
                MemoryObjectRef::from_id_type(
                    fixtures.salient_observation.id,
                    ObjectType::Observation,
                ),
                MemoryObjectRef::from_id_type(
                    fixtures.user_preference.id,
                    ObjectType::DerivedMemory,
                ),
            ]))
            .await
            .unwrap();
        assert!(objects.iter().all(|object| match object {
            MemoryObject::Observation(observation) => {
                observation.retention_state == RetentionState::Suppressed
            }
            MemoryObject::DerivedMemory(memory) => {
                memory.retention_state == RetentionState::Suppressed
            }
            _ => false,
        }));
    }

    #[tokio::test]
    async fn episode_forget_suppresses_observation_only_derived_memories() {
        let fixtures = representative_fixtures();
        let mut observation_only = fixtures.user_preference.clone();
        observation_only.id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_9103);
        observation_only.derived_from_episode_ids.clear();
        observation_only.derived_from_observation_ids = vec![fixtures.salient_observation.id];
        let graph = in_memory_graph_store();
        let mut objects = fixtures.objects();
        objects.push(MemoryObject::DerivedMemory(observation_only.clone()));
        graph.upsert_objects(&objects).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = TemporaryVectorCandidateStore::open(4).await;
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .forget(
                ForgetMemoryDraft::suppress(
                    LifecycleTargetRef::Episode(fixtures.episode.id),
                    "Forget source episode.",
                ),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .unwrap();

        assert!(outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                observation_only.id,
            )));
        let objects = graph
            .query_objects(&GraphObjectQuery::by_refs(vec![
                MemoryObjectRef::from_id_type(observation_only.id, ObjectType::DerivedMemory),
            ]))
            .await
            .unwrap();
        let MemoryObject::DerivedMemory(memory) = &objects[0] else {
            panic!("expected observation-only derived memory");
        };
        assert_eq!(memory.retention_state, RetentionState::Suppressed);
    }

    #[tokio::test]
    async fn forget_policy_can_suppress_source_without_derived_cascade() {
        let fixtures = representative_fixtures();
        let graph = in_memory_graph_store();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = TemporaryVectorCandidateStore::open(4).await;
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft = ForgetMemoryDraft::suppress(
            LifecycleTargetRef::Episode(fixtures.episode.id),
            "Hide source only.",
        );
        draft
            .lifecycle_policy
            .suppression
            .suppress_derived_from_target = false;

        let outcome = pipeline
            .forget(draft, &tokio::sync::Mutex::new(()))
            .await
            .unwrap();

        assert_eq!(
            outcome.graph_mutated_object_ids,
            vec![MemoryObjectRef::new(
                ObjectType::Episode,
                fixtures.episode.id
            )]
        );
        let objects = graph
            .query_objects(&GraphObjectQuery::by_refs(vec![
                MemoryObjectRef::from_id_type(fixtures.episode.id, ObjectType::Episode),
                MemoryObjectRef::from_id_type(
                    fixtures.user_preference.id,
                    ObjectType::DerivedMemory,
                ),
            ]))
            .await
            .unwrap();
        assert!(objects.iter().any(|object| matches!(
            object,
            MemoryObject::Episode(episode)
                if episode.id == fixtures.episode.id
                    && episode.retention_state == RetentionState::Suppressed
        )));
        assert!(objects.iter().any(|object| matches!(
            object,
            MemoryObject::DerivedMemory(memory)
                if memory.id == fixtures.user_preference.id
                    && memory.retention_state == RetentionState::Active
        )));
    }

    #[tokio::test]
    async fn forget_policy_can_skip_source_suppression_without_vector_deleting_source() {
        let fixtures = representative_fixtures();
        let graph = in_memory_graph_store();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = TemporaryVectorCandidateStore::open(4).await;
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft = ForgetMemoryDraft::suppress(
            LifecycleTargetRef::Episode(fixtures.episode.id),
            "Do not hide source object.",
        );
        draft.lifecycle_policy.suppression.suppress_target = false;
        draft
            .lifecycle_policy
            .suppression
            .suppress_derived_from_target = false;

        let outcome = pipeline
            .forget(draft, &tokio::sync::Mutex::new(()))
            .await
            .unwrap();

        assert!(outcome.graph_mutated_object_ids.is_empty());
        assert!(outcome.vector_maintained_object_ids.is_empty());
        let objects = graph
            .query_objects(&GraphObjectQuery::by_refs(vec![
                MemoryObjectRef::from_id_type(fixtures.episode.id, ObjectType::Episode),
            ]))
            .await
            .unwrap();
        assert!(matches!(
            &objects[0],
            MemoryObject::Episode(episode)
                if episode.retention_state == RetentionState::Active
        ));
    }

    #[tokio::test]
    async fn forget_thread_keeps_its_vector_and_optionally_suppresses_members() {
        for cascade in [false, true] {
            let mut fixtures = representative_fixtures();
            fixtures.derived_reflection.thread_ids = vec![fixtures.soft_thread.id];
            let thread = MemoryObject::MemoryThread(fixtures.soft_thread.clone());
            let member = MemoryObject::DerivedMemory(fixtures.derived_reflection.clone());
            let graph = in_memory_graph_store();
            graph.upsert_objects(&fixtures.objects()).await.unwrap();
            graph.upsert_links(&fixtures.links()).await.unwrap();
            let vector = TemporaryVectorCandidateStore::open(2).await;
            for object in [&thread, &member] {
                let record = memory_object_vector_record(object).unwrap();
                vector
                    .upsert_vector_records(&[VectorRecordEmbedding::new(&record, &[1.0, 0.0])])
                    .await
                    .unwrap();
            }
            let query = VectorCandidateSearch::new(
                vec![1.0, 0.0],
                10,
                vec![ObjectType::MemoryThread, ObjectType::DerivedMemory],
            );
            assert_eq!(
                vector
                    .search_candidates(&query)
                    .await
                    .unwrap()
                    .candidates
                    .len(),
                2
            );
            let embedder = DeterministicMemoryEmbedder::new(2);
            let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
            let mut draft = ForgetMemoryDraft::suppress(
                LifecycleTargetRef::MemoryThread(fixtures.soft_thread.id),
                "Forget thread members while preserving the thread.",
            );
            draft.cascade_policy.apply_to_thread_members = cascade;

            let outcome = pipeline
                .forget(draft, &tokio::sync::Mutex::new(()))
                .await
                .unwrap();

            assert!(!outcome
                .graph_mutated_object_ids
                .contains(&thread.object_ref()));
            assert!(!outcome
                .vector_maintained_object_ids
                .contains(&thread.object_ref()));
            let recall = vector.search_candidates(&query).await.unwrap();
            let actual = recall
                .candidates
                .iter()
                .map(|candidate| candidate.object_id)
                .collect::<std::collections::HashSet<_>>();
            let expected = if cascade {
                vec![thread.id()]
            } else {
                vec![thread.id(), member.id()]
            };
            assert_eq!(actual, expected.into_iter().collect());
            let stored = graph
                .query_objects(&GraphObjectQuery::by_refs(vec![
                    thread.object_ref(),
                    member.object_ref(),
                ]))
                .await
                .unwrap();
            assert!(stored.contains(&thread));
            assert!(stored.iter().any(|object| matches!(object,
                MemoryObject::DerivedMemory(memory) if memory.id == member.id()
                    && memory.retention_state == if cascade { RetentionState::Suppressed } else { RetentionState::Active }
            )));
        }
    }

    #[tokio::test]
    async fn forget_thread_cascade_suppresses_members_and_warns_for_current_replacements() {
        let ids = fixed_ids();
        let mut thread = representative_fixtures().soft_thread;
        thread.id = ids.thread;
        let mut current_replacement = old_memory(&ids);
        current_replacement.supersedes.push(ids.replacement);
        let graph = RecordingGraphStore::new(vec![
            MemoryObject::MemoryThread(thread),
            MemoryObject::DerivedMemory(current_replacement),
        ])
        .await;
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft = ForgetMemoryDraft::suppress(
            LifecycleTargetRef::MemoryThread(ids.thread),
            "Suppress thread members.",
        );
        draft.cascade_policy.apply_to_thread_members = true;

        let outcome = pipeline
            .forget(draft, &tokio::sync::Mutex::new(()))
            .await
            .unwrap();

        assert_eq!(
            outcome.diagnostics.warnings,
            vec![LifecycleMutationWarning {
                reason: LifecycleMutationWarningReason::CascadeSuppressesCurrentReplacement,
                affected_memory_ids: vec![ids.old],
            }]
        );
        let objects = graph
            .query_objects(&GraphObjectQuery::by_refs(
                outcome.graph_mutated_object_ids.clone(),
            ))
            .await
            .unwrap();
        assert!(objects.iter().any(|object| matches!(object,
            MemoryObject::DerivedMemory(memory) if memory.id == ids.old
                && memory.retention_state == RetentionState::Suppressed
        )));
        assert_eq!(
            outcome.graph_mutated_object_ids,
            vec![MemoryObjectRef::new(ObjectType::DerivedMemory, ids.old)]
        );
        assert_eq!(
            outcome.vector_maintained_object_ids,
            vec![MemoryObjectRef::new(ObjectType::DerivedMemory, ids.old)]
        );
    }

    #[tokio::test]
    async fn retrieval_excludes_stale_superseded_candidate_when_vector_cleanup_fails() {
        let ids = fixed_ids();
        let graph = in_memory_graph_store();
        graph
            .upsert_objects(&[MemoryObject::DerivedMemory(old_memory(&ids))])
            .await
            .unwrap();
        let vector = DeleteFailingVectorStore::new().await;
        vector
            .inner
            .upsert_vector_records(&[VectorRecordEmbedding::new(
                &memory_object_vector_record(&MemoryObject::DerivedMemory(old_memory(&ids)))
                    .unwrap(),
                &[1.0, 0.0, 0.0, 0.0],
            )])
            .await
            .unwrap();
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let outcome = pipeline
            .correct(correction_draft(&ids), &tokio::sync::Mutex::new(()))
            .await
            .unwrap();
        assert!(outcome.vector_maintenance_failure.is_some());

        let retrieval = RetrievePipeline::new(&graph, &vector, &embedder)
            .retrieve(RetrievalContext::new("stable correction behavior").with_trace())
            .await
            .unwrap();

        let trace = retrieval.trace.as_ref().unwrap();
        assert!(trace.stale_candidate_omissions.iter().any(|omission| {
            omission.candidate.id == ids.old
                && matches!(omission.reason, StaleCandidateReason::Superseded)
        }));
        assert!(!pack_contains_derived_memory(&retrieval.pack, ids.old));
    }

    #[tokio::test]
    async fn retrieval_excludes_stale_source_and_dependent_candidates_when_forget_cleanup_fails() {
        let fixtures = representative_fixtures();
        let graph = in_memory_graph_store();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = DeleteFailingVectorStore::new().await;
        let embedder = DeterministicMemoryEmbedder::new(4);
        for object in [
            MemoryObject::Episode(fixtures.episode.clone()),
            MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
        ] {
            let record = memory_object_vector_record(&object).unwrap();
            vector
                .inner
                .upsert_vector_records(&[VectorRecordEmbedding::new(
                    &record,
                    &[1.0, 0.0, 0.0, 0.0],
                )])
                .await
                .unwrap();
        }
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let outcome = pipeline
            .forget(
                ForgetMemoryDraft::suppress(
                    LifecycleTargetRef::Episode(fixtures.episode.id),
                    "Forget source episode.",
                ),
                &tokio::sync::Mutex::new(()),
            )
            .await
            .unwrap();
        assert!(outcome.vector_maintenance_failure.is_some());

        let retrieval = RetrievePipeline::new(&graph, &vector, &embedder)
            .retrieve(RetrievalContext::new("deterministic store contracts").with_trace())
            .await
            .unwrap();

        let trace = retrieval.trace.as_ref().unwrap();
        assert!(trace.stale_candidate_omissions.iter().any(|omission| {
            omission.candidate.id == fixtures.episode.id
                && matches!(omission.reason, StaleCandidateReason::LifecycleMismatch)
        }));
        assert!(trace.stale_candidate_omissions.iter().any(|omission| {
            omission.candidate.id == fixtures.user_preference.id
                && matches!(omission.reason, StaleCandidateReason::LifecycleMismatch)
        }));
        assert!(!retrieval
            .pack
            .relevant_episodes
            .iter()
            .any(|episode| episode.id == fixtures.episode.id));
        assert!(!pack_contains_derived_memory(
            &retrieval.pack,
            fixtures.user_preference.id,
        ));
    }

    fn correction_draft(ids: &FixedIds) -> CorrectMemoryDraft {
        let mut replacement = ReplacementDerivedMemoryDraft::new(
            DerivedType::Correction,
            "Use graph-authoritative lifecycle correction.",
        )
        .with_source_episode(ids.episode)
        .with_source_observation(ids.observation)
        .with_superseded_memory(ids.old);
        replacement.id = Some(ids.replacement);
        replacement.original_source_provenance = SourceProvenanceReference::episode(ids.episode);
        replacement.correction_origin_provenance =
            SourceProvenanceReference::observation(ids.observation);
        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::derived_memory(ids.old),
            "Replace stale derived memory.",
        )
        .with_replacement(replacement);
        draft.correction_origin = SourceProvenanceReference::observation(ids.observation);
        draft
    }

    fn stateful_correction_draft(ids: &FixedIds, text: &str) -> CorrectMemoryDraft {
        let mut replacement = ReplacementDerivedMemoryDraft::new(DerivedType::Correction, text)
            .with_source_episode(ids.episode)
            .with_source_observation(ids.observation);
        replacement.original_source_provenance = SourceProvenanceReference::episode(ids.episode);
        replacement.correction_origin_provenance =
            SourceProvenanceReference::observation(ids.observation);
        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::source_object(SourceObjectCorrectionTarget::Episode {
                id: ids.episode,
                original_raw_ref: Some("raw://original/episode".to_owned()),
                original_setting_key: None,
            }),
            "Replace stale derived memory.",
        )
        .with_replacement(replacement);
        draft.correction_origin = SourceProvenanceReference::observation(ids.observation);
        draft
    }

    fn old_memory(ids: &FixedIds) -> DerivedMemory {
        DerivedMemory {
            scope_keys: Vec::new(),
            assertions: Vec::new(),
            given_by_application: false,
            id: ids.old,
            object_type: ObjectType::DerivedMemory,
            derived_type: DerivedType::UserPreference,
            text: "Prefer stale lifecycle behavior.".to_owned(),
            derived_from_episode_ids: vec![ids.episode],
            derived_from_observation_ids: vec![ids.observation],
            thread_ids: vec![ids.thread],
            entity_ids: Vec::new(),
            salience_score: 0.7,
            supersedes: Vec::new(),
            retention_state: RetentionState::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        }
    }

    fn current_replacement_from(
        replaced: &DerivedMemory,
        current_replacement_id: MemoryId,
    ) -> DerivedMemory {
        let mut replacement = replaced.clone();
        replacement.id = current_replacement_id;
        replacement.text = "Current correction replacement.".to_owned();
        replacement.supersedes = vec![replaced.id];
        replacement.retention_state = RetentionState::Active;
        replacement
    }

    fn source_episode(ids: &FixedIds) -> Episode {
        Episode {
            scene_local_date: None,
            id: ids.episode,
            object_type: ObjectType::Episode,
            modality: Modality::Chat,
            scene: crate::domain::Scene {
                setting: crate::domain::SceneSetting {
                    key: Some("conversation://original".to_owned()),
                    words: None,
                },

                ..crate::domain::Scene::at((Utc::now()).fixed_offset())
            },
            ended_at: None,
            summary: "Original source episode.".to_owned(),
            raw_ref: Some("raw://original/episode".to_owned()),
            salience_score: 0.8,
            retention_state: RetentionState::Active,
            created_at: Utc::now(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        }
    }

    fn source_observation(ids: &FixedIds) -> Observation {
        Observation {
            id: ids.observation,
            object_type: ObjectType::Observation,
            episode_id: ids.episode,
            speaker_entity_id: None,
            observed_at: None,
            modality: Modality::Chat,
            text: "Original source observation.".to_owned(),
            raw_ref: Some("raw://original/observation".to_owned()),
            salience_score: 0.8,
            retention_state: RetentionState::Active,
            created_at: Utc::now(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct FixedIds {
        old: MemoryId,
        replacement: MemoryId,
        episode: MemoryId,
        observation: MemoryId,
        thread: MemoryId,
    }

    fn fixed_ids() -> FixedIds {
        FixedIds {
            old: id("550e8400-e29b-41d4-a716-446655448001"),
            replacement: id("550e8400-e29b-41d4-a716-446655448002"),
            episode: id("550e8400-e29b-41d4-a716-446655448003"),
            observation: id("550e8400-e29b-41d4-a716-446655448004"),
            thread: id("550e8400-e29b-41d4-a716-446655448005"),
        }
    }

    fn id(value: &str) -> MemoryId {
        Uuid::parse_str(value).unwrap()
    }

    fn pack_contains_derived_memory(
        pack: &crate::api::types::ContinuityContextPack,
        object_id: MemoryId,
    ) -> bool {
        pack.derived_memories
            .iter()
            .chain(pack.preferences.iter())
            .chain(pack.relationship_notes.iter())
            .chain(pack.open_loops.iter())
            .chain(pack.commitments.iter())
            .chain(pack.character_signals.iter())
            .any(|memory| memory.memory.id == object_id)
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum StoreCall {
        GraphQuery(Vec<MemoryId>),
        GraphThreadQuery(Vec<MemoryId>),
        GraphObjects(Vec<MemoryId>),
        GraphLinks(Vec<(MemoryId, MemoryId)>),
        EmbedBatch(Vec<MemoryId>),
        VectorUpsert(Vec<MemoryId>),
        VectorDelete(Vec<MemoryId>),
        StatsEdges(usize),
        StatsObjectStates(usize),
        StatsUnhealthy,
    }

    struct RecordingGraphStore {
        store: OxigraphGraphAuthorityStore,
        calls: Arc<Mutex<Vec<StoreCall>>>,
        fail_objects: bool,
        fail_links: bool,
    }

    impl RecordingGraphStore {
        async fn new(objects: Vec<MemoryObject>) -> Self {
            let store = in_memory_graph_store();
            store.upsert_objects(&objects).await.unwrap();
            Self {
                store,
                calls: Arc::default(),
                fail_objects: false,
                fail_links: false,
            }
        }

        fn fail_objects(mut self) -> Self {
            self.fail_objects = true;
            self
        }

        fn fail_links(mut self) -> Self {
            self.fail_links = true;
            self
        }

        fn calls(&self) -> Vec<StoreCall> {
            lock(&self.calls).clone()
        }
    }

    #[async_trait]
    impl GraphAuthorityStore for RecordingGraphStore {
        async fn query_anniversaries(
            &self,
            date: chrono::NaiveDate,
            participants: &[crate::domain::MemoryId],
            limit: usize,
            policy: crate::ports::graph_authority::GraphExpansionLifecyclePolicy,
        ) -> Result<Vec<(crate::domain::MemoryId, bool)>, CustomError> {
            let _ = (date, participants, limit, policy);
            Ok(Vec::new())
        }

        async fn query_episodes_by_time(
            &self,
            start: Option<chrono::DateTime<chrono::Utc>>,
            end: chrono::DateTime<chrono::Utc>,
            limit: usize,
            policy: crate::ports::graph_authority::GraphExpansionLifecyclePolicy,
        ) -> Result<Vec<crate::domain::MemoryId>, CustomError> {
            self.store
                .query_episodes_by_time(start, end, limit, policy)
                .await
        }

        async fn query_episode_occasions(
            &self,
            episodes: &[crate::domain::MemoryObjectRef],
        ) -> Result<crate::policy::graph_expansion::ParticipantOccasions, CustomError> {
            self.store.query_episode_occasions(episodes).await
        }

        async fn query_last_interaction(
            &self,
            participant: MemoryId,
            reference_time: chrono::DateTime<chrono::Utc>,
            policy: crate::ports::graph_authority::GraphExpansionLifecyclePolicy,
        ) -> Result<Option<(MemoryId, chrono::DateTime<chrono::Utc>)>, CustomError> {
            self.store
                .query_last_interaction(participant, reference_time, policy)
                .await
        }

        async fn query_notions_known_as(
            &self,
            name: &str,
        ) -> Result<Vec<MemoryId>, crate::errors::GraphQueryError> {
            self.store.query_notions_known_as(name).await
        }

        async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError> {
            lock(&self.calls).push(StoreCall::GraphObjects(
                objects.iter().map(MemoryObject::id).collect(),
            ));
            if self.fail_objects {
                return Err(CustomError::DatabaseError("object write failed".to_owned()));
            }
            self.store.upsert_objects(objects).await
        }

        async fn upsert_links(&self, links: &[MemoryLink]) -> Result<(), CustomError> {
            lock(&self.calls).push(StoreCall::GraphLinks(
                links
                    .iter()
                    .map(|link| (link.from_id, link.to_id))
                    .collect(),
            ));
            if self.fail_links {
                return Err(CustomError::DatabaseError("link write failed".to_owned()));
            }
            self.store.upsert_links(links).await
        }

        async fn upsert_objects_and_links(
            &self,
            objects: &[MemoryObject],
            links: &[MemoryLink],
        ) -> Result<(), CustomError> {
            lock(&self.calls).push(StoreCall::GraphObjects(
                objects.iter().map(MemoryObject::id).collect(),
            ));
            if self.fail_objects {
                return Err(CustomError::DatabaseError("object write failed".to_owned()));
            }
            lock(&self.calls).push(StoreCall::GraphLinks(
                links
                    .iter()
                    .map(|link| (link.from_id, link.to_id))
                    .collect(),
            ));
            if self.fail_links {
                return Err(CustomError::DatabaseError("link write failed".to_owned()));
            }
            self.store.upsert_objects_and_links(objects, links).await
        }

        async fn query_objects(
            &self,
            query: &GraphObjectQuery,
        ) -> Result<Vec<MemoryObject>, crate::errors::GraphQueryError> {
            let queried_ids = match query {
                GraphObjectQuery::ByRefs(object_refs) => {
                    object_refs.iter().map(|object_ref| object_ref.id).collect()
                }
                GraphObjectQuery::ByIds(object_ids) => object_ids.clone(),
                GraphObjectQuery::ByTypes { .. } => Vec::new(),
            };
            lock(&self.calls).push(StoreCall::GraphQuery(queried_ids));
            self.store.query_objects(query).await
        }

        async fn query_superseded_derived_memory_ids(
            &self,
            memory_ids: &[crate::domain::MemoryId],
        ) -> Result<Vec<crate::domain::MemoryId>, crate::errors::GraphQueryError> {
            self.store
                .query_superseded_derived_memory_ids(memory_ids)
                .await
        }

        async fn query_links_by_ids(
            &self,
            link_ids: &[MemoryId],
        ) -> Result<Vec<MemoryLink>, CustomError> {
            self.store.query_links_by_ids(link_ids).await
        }

        async fn query_derived_memories_by_provenance(
            &self,
            query: &GraphDerivedMemoryProvenanceQuery,
        ) -> Result<Vec<DerivedMemory>, CustomError> {
            self.store.query_derived_memories_by_provenance(query).await
        }

        async fn query_derived_memories_by_thread(
            &self,
            query: &GraphDerivedMemoryThreadQuery,
        ) -> Result<(Vec<DerivedMemory>, Vec<GraphExpansionFilteredNode>), CustomError> {
            lock(&self.calls).push(StoreCall::GraphThreadQuery(query.thread_ids.clone()));
            self.store.query_derived_memories_by_thread(query).await
        }

        async fn query_scope_state(
            &self,
            key: &ScopeKey,
            policy: GraphExpansionLifecyclePolicy,
        ) -> Result<(Vec<MemoryId>, Vec<GraphExpansionFilteredNode>), CustomError> {
            self.store.query_scope_state(key, policy).await
        }

        async fn expand_bounded(
            &self,
            query: &GraphExpansionQuery,
        ) -> Result<GraphExpansion, CustomError> {
            self.store.expand_bounded(query).await
        }
    }

    #[derive(Debug, Default)]
    struct RecordingVectorStore {
        calls: Arc<Mutex<Vec<StoreCall>>>,
        fail_delete: bool,
    }

    impl RecordingVectorStore {
        fn fail_delete(mut self) -> Self {
            self.fail_delete = true;
            self
        }

        fn calls(&self) -> Vec<StoreCall> {
            lock(&self.calls).clone()
        }
    }

    #[async_trait]
    impl VectorCandidateStore for RecordingVectorStore {
        async fn upsert_vector_records(
            &self,
            records: &[VectorRecordEmbedding<'_>],
        ) -> Result<(), CustomError> {
            lock(&self.calls).push(StoreCall::VectorUpsert(
                records
                    .iter()
                    .map(|record| record.record.object_id)
                    .collect(),
            ));
            Ok(())
        }

        async fn search_candidates(
            &self,
            query: &VectorCandidateSearch,
        ) -> Result<VectorCandidateRecall, CustomError> {
            Ok(VectorCandidateRecall {
                scene_pool: crate::models::vector::CanonicalCandidates::new([]),
                candidates: CanonicalCandidates::new([]),
                completeness: if query.limit == 0 || query.object_types.is_empty() {
                    crate::api::types::retrieval::VectorRecallCompleteness::NotRequested
                } else {
                    crate::api::types::retrieval::VectorRecallCompleteness::Exhaustive {
                        scanned: 0,
                    }
                },
            })
        }

        async fn delete_candidates(&self, object_ids: &[MemoryId]) -> Result<(), CustomError> {
            lock(&self.calls).push(StoreCall::VectorDelete(object_ids.to_vec()));
            if self.fail_delete {
                return Err(CustomError::VectorDatabaseError(VectorDatabaseError::new(
                    "test",
                    VectorDatabaseErrorKind::Response,
                    None,
                    "vector delete failed",
                )));
            }
            Ok(())
        }
    }

    #[derive(Debug, Default)]
    struct RecordingEmbedder {
        calls: Arc<Mutex<Vec<StoreCall>>>,
    }

    #[async_trait]
    impl MemoryEmbedder for RecordingEmbedder {
        async fn embed(&self, _input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
            Ok(vec![1.0, 0.0, 0.0, 0.0])
        }

        async fn embed_batch(
            &self,
            inputs: &[EmbeddingInput],
        ) -> Result<Vec<Vec<f32>>, CustomError> {
            lock(&self.calls).push(StoreCall::EmbedBatch(
                inputs.iter().filter_map(|input| input.object_id).collect(),
            ));
            Ok(inputs.iter().map(|_| vec![1.0, 0.0, 0.0, 0.0]).collect())
        }
    }

    #[derive(Debug, Default)]
    struct RecordingStatsStore {
        calls: Arc<Mutex<Vec<StoreCall>>>,
        fail_edges: bool,
        fail_object_states: bool,
    }

    #[async_trait]
    impl RetrievalStatsStore for RecordingStatsStore {
        async fn record_edges(
            &self,
            edges: &[RetrievalStatsEdge],
        ) -> Result<(), RetrievalStatsStoreError> {
            lock(&self.calls).push(StoreCall::StatsEdges(edges.len()));
            if self.fail_edges {
                return Err(RetrievalStatsStoreError::Sqlite {
                    detail: "stats edge write failed".to_owned(),
                });
            }
            Ok(())
        }

        async fn record_object_states(
            &self,
            states: &[RetrievalStatsObjectState],
        ) -> Result<(), RetrievalStatsStoreError> {
            lock(&self.calls).push(StoreCall::StatsObjectStates(states.len()));
            if self.fail_object_states {
                return Err(RetrievalStatsStoreError::Sqlite {
                    detail: "stats object-state write failed".to_owned(),
                });
            }
            Ok(())
        }

        async fn counter(
            &self,
            _key: &RetrievalStatsCounterKey,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            Ok(None)
        }

        async fn global_counter(
            &self,
            _relation_kind: RelationType,
            _object_type: ObjectType,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            Ok(None)
        }

        async fn health(&self) -> Result<RetrievalStatsHealth, RetrievalStatsStoreError> {
            Ok(RetrievalStatsHealth::default())
        }
        async fn global_episode_counter(
            &self,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            Ok(None)
        }

        async fn mark_unhealthy(
            &self,
            _cause: RetrievalStatsHealthCause,
        ) -> Result<(), RetrievalStatsStoreError> {
            lock(&self.calls).push(StoreCall::StatsUnhealthy);
            Ok(())
        }
    }

    #[derive(Debug)]
    struct OneShotDeleteFailingVectorStore {
        inner: TemporaryVectorCandidateStore,
        fail_next_delete: Mutex<bool>,
    }

    impl OneShotDeleteFailingVectorStore {
        async fn new() -> Self {
            Self {
                inner: TemporaryVectorCandidateStore::open(4).await,
                fail_next_delete: Mutex::new(true),
            }
        }
    }

    #[async_trait]
    impl VectorCandidateStore for OneShotDeleteFailingVectorStore {
        async fn upsert_vector_records(
            &self,
            records: &[VectorRecordEmbedding<'_>],
        ) -> Result<(), CustomError> {
            self.inner.upsert_vector_records(records).await
        }

        async fn search_candidates(
            &self,
            query: &VectorCandidateSearch,
        ) -> Result<VectorCandidateRecall, CustomError> {
            self.inner.search_candidates(query).await
        }

        async fn delete_candidates(&self, object_ids: &[MemoryId]) -> Result<(), CustomError> {
            if std::mem::take(&mut *lock(&self.fail_next_delete)) {
                return Err(CustomError::VectorDatabaseError(VectorDatabaseError::new(
                    "test",
                    VectorDatabaseErrorKind::Response,
                    None,
                    "one-shot vector delete failed",
                )));
            }
            self.inner.delete_candidates(object_ids).await
        }
    }

    #[derive(Debug)]
    struct OneShotEdgeFailingStatsStore {
        inner: InMemoryRetrievalStatsStore,
        fail_next_edges: Mutex<bool>,
    }

    impl OneShotEdgeFailingStatsStore {
        fn new() -> Self {
            Self {
                inner: InMemoryRetrievalStatsStore::new(),
                fail_next_edges: Mutex::new(true),
            }
        }
    }

    #[async_trait]
    impl RetrievalStatsStore for OneShotEdgeFailingStatsStore {
        async fn record_edges(
            &self,
            edges: &[RetrievalStatsEdge],
        ) -> Result<(), RetrievalStatsStoreError> {
            if std::mem::take(&mut *lock(&self.fail_next_edges)) {
                return Err(RetrievalStatsStoreError::Sqlite {
                    detail: "one-shot stats edge write failed".to_owned(),
                });
            }
            self.inner.record_edges(edges).await
        }

        async fn record_object_states(
            &self,
            states: &[RetrievalStatsObjectState],
        ) -> Result<(), RetrievalStatsStoreError> {
            self.inner.record_object_states(states).await
        }

        async fn counter(
            &self,
            key: &RetrievalStatsCounterKey,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            self.inner.counter(key).await
        }

        async fn global_counter(
            &self,
            relation_kind: RelationType,
            object_type: ObjectType,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            self.inner.global_counter(relation_kind, object_type).await
        }

        async fn health(&self) -> Result<RetrievalStatsHealth, RetrievalStatsStoreError> {
            self.inner.health().await
        }
        async fn global_episode_counter(
            &self,
        ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
            self.inner.global_episode_counter().await
        }

        async fn mark_unhealthy(
            &self,
            cause: RetrievalStatsHealthCause,
        ) -> Result<(), RetrievalStatsStoreError> {
            self.inner.mark_unhealthy(cause).await
        }
    }

    #[derive(Debug)]
    struct DeleteFailingVectorStore {
        inner: TemporaryVectorCandidateStore,
    }

    impl DeleteFailingVectorStore {
        async fn new() -> Self {
            Self {
                inner: TemporaryVectorCandidateStore::open(4).await,
            }
        }
    }

    #[async_trait]
    impl VectorCandidateStore for DeleteFailingVectorStore {
        async fn upsert_vector_records(
            &self,
            records: &[VectorRecordEmbedding<'_>],
        ) -> Result<(), CustomError> {
            self.inner.upsert_vector_records(records).await
        }

        async fn search_candidates(
            &self,
            query: &VectorCandidateSearch,
        ) -> Result<VectorCandidateRecall, CustomError> {
            self.inner.search_candidates(query).await
        }

        async fn delete_candidates(&self, _object_ids: &[MemoryId]) -> Result<(), CustomError> {
            Err(CustomError::VectorDatabaseError(VectorDatabaseError::new(
                "test",
                VectorDatabaseErrorKind::Response,
                None,
                "vector delete failed",
            )))
        }
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex.lock().expect("test mutex should not be poisoned")
    }
}
