// Correction/forget lifecycle pipeline used by the public facade and internal
// tests. The module keeps helper APIs for focused lifecycle controls and
// fixture-level validation.
use chrono::Utc;
use uuid::Uuid;

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
    DerivedMemory, DerivedType, Episode, LifecyclePolicyKnob, MemoryId, MemoryLink, MemoryObject,
    MemoryObjectRef, MemoryThread, ObjectType, Observation, RelationType, RetentionState,
    Stability, ThreadStatus, DEFAULT_SCHEMA_VERSION,
};
use crate::errors::{CustomError, VectorIndexingCause};
use crate::policy::memory_object_vector_record;
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::graph_authority::{
    GraphAuthorityStore, GraphDerivedMemoryProvenanceQuery, GraphDerivedMemoryThreadQuery,
    GraphExpansionLifecyclePolicy, GraphObjectQuery,
};
use crate::ports::retrieval_stats::RetrievalStatsStore;
use crate::ports::vector_candidate::VectorCandidateStore;
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
    ) -> Result<LifecycleMutationOutcome, CustomError> {
        draft.validate()?;
        validate_correction_policy(&draft)?;
        let plan = self.correction_plan(draft).await?;

        self.graph_store
            .upsert_objects_and_links(&plan.graph_objects, &plan.graph_links)
            .await?;

        let mut outcome = plan.outcome_after_graph_success();
        let vector_result = self
            .maintain_vectors(&plan.vector_delete_refs, &plan.vector_upsert_objects)
            .await?;
        apply_vector_result(&mut outcome, vector_result);
        StatsProjectionService::new(self.graph_store, self.stats_store)
            .project(&plan.graph_objects, &plan.graph_links)
            .await;
        Ok(outcome)
    }

    pub(crate) async fn forget(
        &self,
        draft: ForgetMemoryDraft,
    ) -> Result<LifecycleMutationOutcome, CustomError> {
        draft.validate()?;
        validate_forget_policy(&draft)?;
        let plan = self.forget_plan(draft).await?;

        self.graph_store.upsert_objects(&plan.graph_objects).await?;

        let mut outcome = plan.outcome_after_graph_success();
        let vector_result = self.maintain_vectors(&plan.vector_delete_refs, &[]).await?;
        apply_vector_result(&mut outcome, vector_result);
        StatsProjectionService::new(self.graph_store, self.stats_store)
            .project(&plan.graph_objects, &plan.graph_links)
            .await;
        Ok(outcome)
    }

    async fn correction_plan(
        &self,
        draft: CorrectMemoryDraft,
    ) -> Result<MutationPlan, CustomError> {
        let mut superseded = Vec::new();
        let mut source_episode_ids = Vec::new();
        let mut source_observation_ids = Vec::new();
        let mut requested_targets = Vec::new();
        let mut cascade_warning_ids = Vec::new();

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
                            if draft.lifecycle_policy.suppress_superseded_derived_memories {
                                record_current_replacement_warning(
                                    &memory,
                                    RetentionState::Suppressed,
                                    &mut cascade_warning_ids,
                                );
                            }
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

        let replacement_drafts = replacement_drafts_or_default(
            &draft,
            &superseded,
            &source_episode_ids,
            &source_observation_ids,
        )?;
        let replacement_memories = replacement_drafts
            .into_iter()
            .map(|replacement| replacement_memory(replacement, &superseded, &draft))
            .collect::<Result<Vec<_>, _>>()?;

        let mut graph_objects = Vec::new();
        for memory in superseded.iter().cloned() {
            graph_objects.push(MemoryObject::DerivedMemory(non_current_superseded_memory(
                memory,
                draft.lifecycle_policy.suppress_superseded_derived_memories,
            )));
        }
        graph_objects.extend(
            replacement_memories
                .iter()
                .cloned()
                .map(MemoryObject::DerivedMemory),
        );

        let superseded_ids = superseded
            .iter()
            .map(|memory| memory.id)
            .collect::<Vec<_>>();
        let mut graph_links = Vec::new();
        if draft.lifecycle_policy.supersede_replaced_derived_memories {
            for replacement in &replacement_memories {
                for superseded_id in &superseded_ids {
                    graph_links.push(supersedes_link(
                        replacement.id,
                        *superseded_id,
                        &draft.rationale,
                    ));
                }
            }
        }
        graph_links.sort_by_key(|link| (link.from_id, link.to_id, link.id));

        let vector_upsert_objects = replacement_memories
            .iter()
            .cloned()
            .map(MemoryObject::DerivedMemory)
            .collect::<Vec<_>>();
        let vector_delete_refs = superseded_ids
            .iter()
            .copied()
            .map(|id| MemoryObjectRef::new(ObjectType::DerivedMemory, id))
            .collect::<Vec<_>>();
        let trace = draft.include_trace.then(|| LifecycleMutationTrace {
            requested_targets,
            superseded_by: if draft.lifecycle_policy.supersede_replaced_derived_memories {
                superseded_ids
                    .iter()
                    .flat_map(|superseded_id| {
                        replacement_memories
                            .iter()
                            .map(move |replacement| SupersededByEvidence {
                                superseded_memory_id: *superseded_id,
                                superseded_by_memory_id: replacement.id,
                            })
                    })
                    .collect()
            } else {
                Vec::new()
            },
        });

        Ok(MutationPlan::new(
            graph_objects,
            graph_links,
            vector_delete_refs,
            vector_upsert_objects,
            trace,
            cascade_diagnostics(cascade_warning_ids),
        ))
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
                        let suppressed =
                            suppress_derived_memory(memory, draft.target_retention_state);
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
                        suppressed.retention_state = draft.target_retention_state;
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
                            draft.target_retention_state,
                            &mut cascade_warning_ids,
                        )
                        .await?;
                    }
                }
                LifecycleTargetRef::Observation(id) => {
                    let observation = self.fetch_observation(*id).await?;
                    if draft.lifecycle_policy.suppression.suppress_target {
                        let mut suppressed = observation;
                        suppressed.retention_state = draft.target_retention_state;
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
                            draft.target_retention_state,
                            &mut cascade_warning_ids,
                        )
                        .await?;
                    }
                }
                LifecycleTargetRef::MemoryThread(id) => {
                    let thread = self.fetch_thread(*id).await?;
                    if draft.lifecycle_policy.archive.archive_thread {
                        let mut archived = thread;
                        archived.status =
                            draft.target_thread_status.unwrap_or(ThreadStatus::Archived);
                        push_object_unique(
                            &mut graph_objects,
                            MemoryObject::MemoryThread(archived),
                        );
                        push_ref_unique(&mut vector_delete_refs, target.as_memory_object_ref());
                    }
                    if draft
                        .lifecycle_policy
                        .archive
                        .archive_thread_derived_memories
                        || draft.cascade_policy.apply_to_thread_members
                    {
                        self.add_thread_forget_cascade(
                            &mut graph_objects,
                            &mut vector_delete_refs,
                            *id,
                            draft.target_retention_state,
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
            Vec::new(),
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
        target_retention_state: RetentionState,
        cascade_warning_ids: &mut Vec<MemoryId>,
    ) -> Result<(), CustomError> {
        let affected = self
            .query_current_derived_by_provenance(episode_ids, observation_ids)
            .await?;
        for memory in affected {
            record_current_replacement_warning(
                &memory,
                target_retention_state,
                cascade_warning_ids,
            );
            let id = memory.id;
            push_object_unique(
                graph_objects,
                MemoryObject::DerivedMemory(suppress_derived_memory(
                    memory,
                    target_retention_state,
                )),
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
        target_retention_state: RetentionState,
        cascade_warning_ids: &mut Vec<MemoryId>,
    ) -> Result<(), CustomError> {
        let matches = self
            .graph_store
            .query_derived_memories_by_thread(
                &GraphDerivedMemoryThreadQuery::by_threads(vec![thread_id])
                    .with_lifecycle_policy(GraphExpansionLifecyclePolicy::default()),
            )
            .await?;
        for memory in matches {
            record_current_replacement_warning(
                &memory,
                target_retention_state,
                cascade_warning_ids,
            );
            let id = memory.id;
            push_object_unique(
                graph_objects,
                MemoryObject::DerivedMemory(suppress_derived_memory(
                    memory,
                    target_retention_state,
                )),
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
        let has_raw_ref =
            source_target_original_raw_ref(target).is_some_and(|value| !value.trim().is_empty());
        let has_source_ref =
            source_target_original_source_ref(target).is_some_and(|value| !value.trim().is_empty());
        if !has_raw_ref && !has_source_ref {
            return Err(validation_error(
                "source-object correction requires an original raw or source reference",
            ));
        }

        match target {
            SourceObjectCorrectionTarget::Episode {
                id,
                original_raw_ref,
                original_source_ref,
            } => {
                let episode = self.fetch_episode(*id).await?;
                validate_optional_original_ref(
                    "episode original raw reference",
                    original_raw_ref.as_deref(),
                    episode.raw_ref.as_deref(),
                )?;
                validate_optional_original_ref(
                    "episode original source reference",
                    original_source_ref.as_deref(),
                    episode.source_conversation_id.as_deref(),
                )
            }
            SourceObjectCorrectionTarget::Observation {
                id,
                original_raw_ref,
                original_source_ref,
            } => {
                let observation = self.fetch_observation(*id).await?;
                validate_optional_original_ref(
                    "observation original raw reference",
                    original_raw_ref.as_deref(),
                    observation.raw_ref.as_deref(),
                )?;
                if original_source_ref
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    let episode = self.fetch_episode(observation.episode_id).await?;
                    validate_optional_original_ref(
                        "observation original source reference",
                        original_source_ref.as_deref(),
                        episode.source_conversation_id.as_deref(),
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
    ) -> Result<VectorMaintenanceResult, CustomError> {
        let mut maintained = Vec::new();
        let mut failures = Vec::new();

        if !delete_refs.is_empty() {
            let delete_ids = delete_refs
                .iter()
                .map(|object_ref| object_ref.id)
                .collect::<Vec<_>>();
            match self.vector_store.delete_candidates(&delete_ids).await {
                Ok(()) => maintained.extend_from_slice(delete_refs),
                Err(CustomError::VectorDatabaseError(error)) => {
                    failures.push(VectorMaintenanceFailureItem {
                        operation: VectorMaintenanceOperation::Delete,
                        objects: delete_refs.to_vec(),
                        cause: VectorIndexingCause::VectorDatabase(error),
                    })
                }
                Err(error) => return Err(error),
            }
        }

        let vector_records = upsert_objects
            .iter()
            .filter_map(memory_object_vector_record)
            .collect::<Vec<_>>();
        if !vector_records.is_empty() {
            let indexing = VectorIndexingService::new(self.vector_store, self.embedder)
                .index(vector_records)
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
    vector_upsert_objects: Vec<MemoryObject>,
    trace: Option<LifecycleMutationTrace>,
    diagnostics: LifecycleMutationDiagnostics,
}

impl MutationPlan {
    fn new(
        mut graph_objects: Vec<MemoryObject>,
        mut graph_links: Vec<MemoryLink>,
        mut vector_delete_refs: Vec<MemoryObjectRef>,
        vector_upsert_objects: Vec<MemoryObject>,
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
            vector_upsert_objects,
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

fn validate_correction_policy(draft: &CorrectMemoryDraft) -> Result<(), CustomError> {
    if !draft.lifecycle_policy.retain_original_source_objects {
        return Err(CustomError::LifecyclePolicyUnsupported {
            knob: LifecyclePolicyKnob::CorrectionRetainOriginalSourceObjects,
        });
    }
    if !draft.cascade_policy.require_original_source_match {
        return Err(CustomError::LifecyclePolicyUnsupported {
            knob: LifecyclePolicyKnob::CorrectionRequireOriginalSourceMatch,
        });
    }
    if draft.cascade_policy.cascade_to_threads {
        return Err(CustomError::LifecyclePolicyUnsupported {
            knob: LifecyclePolicyKnob::CorrectionCascadeToThreads,
        });
    }
    Ok(())
}

fn validate_forget_policy(draft: &ForgetMemoryDraft) -> Result<(), CustomError> {
    if !draft
        .lifecycle_policy
        .suppression
        .preserve_original_raw_refs
    {
        return Err(CustomError::LifecyclePolicyUnsupported {
            knob: LifecyclePolicyKnob::ForgetPreserveOriginalRawRefs,
        });
    }
    if !draft.lifecycle_policy.archive.preserve_original_raw_refs {
        return Err(CustomError::LifecyclePolicyUnsupported {
            knob: LifecyclePolicyKnob::ForgetArchivePreserveOriginalRawRefs,
        });
    }
    Ok(())
}

fn replacement_drafts_or_default(
    draft: &CorrectMemoryDraft,
    superseded: &[DerivedMemory],
    source_episode_ids: &[MemoryId],
    source_observation_ids: &[MemoryId],
) -> Result<Vec<ReplacementDerivedMemoryDraft>, CustomError> {
    let mut replacements = if draft.replacement_derived_memories.is_empty() {
        vec![ReplacementDerivedMemoryDraft {
            id: None,
            derived_type: DerivedType::Correction,
            text: draft.rationale.clone(),
            derived_from_episode_ids: source_episode_ids.to_vec(),
            derived_from_observation_ids: source_observation_ids.to_vec(),
            thread_ids: stable_union(
                superseded
                    .iter()
                    .flat_map(|memory| memory.thread_ids.clone()),
            ),
            entity_ids: stable_union(
                superseded
                    .iter()
                    .flat_map(|memory| memory.entity_ids.clone()),
            ),
            confidence: 1.0,
            salience_score: superseded
                .iter()
                .map(|memory| memory.salience_score)
                .max_by(f32::total_cmp)
                .unwrap_or(0.5),
            stability: Stability::Medium,
            supersedes: superseded.iter().map(|memory| memory.id).collect(),
            original_source_provenance: SourceProvenanceReference {
                episode_ids: source_episode_ids.to_vec(),
                observation_ids: source_observation_ids.to_vec(),
                external_refs: Vec::new(),
            },
            correction_origin_provenance: draft.correction_origin.clone(),
        }]
    } else {
        draft.replacement_derived_memories.clone()
    };

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
        if replacement.derived_from_episode_ids.is_empty()
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
        replacement.validate().map_err(validation_error)?;
    }

    Ok(replacements)
}

fn replacement_memory(
    draft: ReplacementDerivedMemoryDraft,
    superseded: &[DerivedMemory],
    request: &CorrectMemoryDraft,
) -> Result<DerivedMemory, CustomError> {
    let now = Utc::now();
    let memory = DerivedMemory {
        id: draft.id.unwrap_or_else(Uuid::new_v4),
        object_type: ObjectType::DerivedMemory,
        derived_type: draft.derived_type,
        text: draft.text,
        derived_from_episode_ids: draft.derived_from_episode_ids,
        derived_from_observation_ids: draft.derived_from_observation_ids,
        thread_ids: draft.thread_ids,
        entity_ids: draft.entity_ids,
        confidence: draft.confidence,
        salience_score: draft.salience_score,
        stability: draft.stability,
        is_current: true,
        supersedes: if request.lifecycle_policy.supersede_replaced_derived_memories {
            draft.supersedes
        } else {
            Vec::new()
        },
        retention_state: RetentionState::Active,
        created_at: now,
        updated_at: now,
        schema_version: superseded
            .first()
            .map(|memory| memory.schema_version.clone())
            .unwrap_or_else(|| DEFAULT_SCHEMA_VERSION.to_owned()),
    };
    memory.validate().map_err(validation_error)?;
    Ok(memory)
}

fn non_current_superseded_memory(mut memory: DerivedMemory, suppress: bool) -> DerivedMemory {
    memory.is_current = false;
    if suppress {
        memory.retention_state = RetentionState::Suppressed;
    }
    memory.updated_at = Utc::now();
    memory
}

fn suppress_derived_memory(
    mut memory: DerivedMemory,
    retention_state: RetentionState,
) -> DerivedMemory {
    memory.is_current = false;
    memory.retention_state = retention_state;
    memory.updated_at = Utc::now();
    memory
}

fn supersedes_link(from_id: MemoryId, to_id: MemoryId, rationale: &str) -> MemoryLink {
    MemoryLink {
        id: Uuid::new_v4(),
        object_type: ObjectType::MemoryLink,
        from_id,
        from_type: ObjectType::DerivedMemory,
        to_id,
        to_type: ObjectType::DerivedMemory,
        relation: RelationType::Supersedes,
        confidence: 1.0,
        rationale: Some(rationale.to_owned()),
        created_at: Utc::now(),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    }
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

fn source_target_original_source_ref(target: &SourceObjectCorrectionTarget) -> Option<&str> {
    match target {
        SourceObjectCorrectionTarget::Episode {
            original_source_ref,
            ..
        }
        | SourceObjectCorrectionTarget::Observation {
            original_source_ref,
            ..
        } => original_source_ref.as_deref(),
    }
}

fn validate_optional_original_ref(
    label: &str,
    provided: Option<&str>,
    stored: Option<&str>,
) -> Result<(), CustomError> {
    let Some(provided) = provided.filter(|value| !value.trim().is_empty()) else {
        return Ok(());
    };

    if stored == Some(provided) {
        Ok(())
    } else {
        Err(validation_error(format!(
            "{label} does not match current graph object"
        )))
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
    target_retention_state: RetentionState,
    cascade_warning_ids: &mut Vec<MemoryId>,
) {
    if target_retention_state == RetentionState::Suppressed
        && memory.is_current
        && !memory.supersedes.is_empty()
    {
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

fn validation_error(error: impl ToString) -> CustomError {
    CustomError::MemoryValidation(error.to_string())
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
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex, MutexGuard};

    use crate::api::types::{
        ExternalSourceReference, RetrievalContext, StaleCandidateReason, VectorCandidateTrace,
    };
    use crate::domain::{Episode, Modality, Observation};
    use crate::errors::{
        RetrievalStatsHealthCause, RetrievalStatsStoreError, VectorDatabaseError,
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
    use crate::test_support::{
        representative_fixtures, DeterministicMemoryEmbedder, FakeGraphAuthorityStore,
        FakeVectorCandidateStore,
    };
    use crate::usecases::RetrievePipeline;

    #[tokio::test]
    async fn correction_writes_graph_before_vector_maintenance_in_stable_order() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))]);
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .correct(correction_draft(&ids))
            .await
            .expect("correction should succeed");

        assert_eq!(
            graph.calls(),
            vec![
                StoreCall::GraphQuery(vec![ids.old]),
                StoreCall::GraphObjects(vec![ids.old, ids.replacement]),
                StoreCall::GraphLinks(vec![(ids.replacement, ids.old)]),
            ]
        );
        assert_eq!(
            vector.calls(),
            vec![
                StoreCall::VectorDelete(vec![ids.old]),
                StoreCall::VectorUpsert(vec![ids.replacement]),
            ]
        );
        assert_eq!(
            embedder.calls(),
            vec![StoreCall::EmbedBatch(vec![ids.replacement])]
        );
        assert_eq!(
            outcome.graph_mutated_object_ids,
            vec![
                MemoryObjectRef::new(ObjectType::DerivedMemory, ids.old),
                MemoryObjectRef::new(ObjectType::DerivedMemory, ids.replacement),
            ]
        );
        assert_eq!(
            outcome.vector_maintained_object_ids,
            vec![
                MemoryObjectRef::new(ObjectType::DerivedMemory, ids.old),
                MemoryObjectRef::new(ObjectType::DerivedMemory, ids.replacement),
            ]
        );
        assert!(outcome.vector_maintenance_failure.is_none());
    }

    #[tokio::test]
    async fn correction_stats_failure_runs_after_vector_and_preserves_outcome() {
        let ids = fixed_ids();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let graph = RecordingGraphStore {
            objects: Mutex::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))]),
            calls: calls.clone(),
            ..RecordingGraphStore::default()
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
        };
        let pipeline = CorrectionForgetPipeline::new_with_stats(&graph, &vector, &embedder, &stats);

        let outcome = pipeline
            .correct(correction_draft(&ids))
            .await
            .expect("stats failure should not change lifecycle outcome");

        assert!(outcome.vector_maintenance_failure.is_none());
        assert_eq!(
            lock(&calls).clone(),
            vec![
                StoreCall::GraphQuery(vec![ids.old]),
                StoreCall::GraphObjects(vec![ids.old, ids.replacement]),
                StoreCall::GraphLinks(vec![(ids.replacement, ids.old)]),
                StoreCall::VectorDelete(vec![ids.old]),
                StoreCall::EmbedBatch(vec![ids.replacement]),
                StoreCall::VectorUpsert(vec![ids.replacement]),
                StoreCall::StatsEdges(0),
                StoreCall::StatsUnhealthy,
            ]
        );
    }

    #[tokio::test]
    async fn forget_records_stats_after_vector_maintenance() {
        let ids = fixed_ids();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let graph = RecordingGraphStore {
            objects: Mutex::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))]),
            calls: calls.clone(),
            ..RecordingGraphStore::default()
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
        };
        let pipeline = CorrectionForgetPipeline::new_with_stats(&graph, &vector, &embedder, &stats);

        let outcome = pipeline
            .forget(ForgetMemoryDraft::suppress(
                LifecycleTargetRef::DerivedMemory(ids.old),
                "Suppress stale derived memory.",
            ))
            .await
            .expect("forget should record stats after vector maintenance");

        assert!(outcome.vector_maintenance_failure.is_none());
        assert_eq!(
            lock(&calls).clone(),
            vec![
                StoreCall::GraphQuery(vec![ids.old]),
                StoreCall::GraphObjects(vec![ids.old]),
                StoreCall::VectorDelete(vec![ids.old]),
                StoreCall::StatsEdges(0),
                StoreCall::StatsObjectStates(1),
            ]
        );
    }

    #[tokio::test]
    async fn validation_failure_prevents_graph_and_vector_writes() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))]);
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft = correction_draft(&ids);
        draft.rationale = " ".to_owned();

        let error = pipeline.correct(draft).await.unwrap_err();

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
    async fn unsupported_correction_policy_knobs_fail_before_writes() {
        let ids = fixed_ids();
        for (mut draft, expected_knob) in [
            (
                {
                    let mut draft = correction_draft(&ids);
                    draft.lifecycle_policy.retain_original_source_objects = false;
                    draft
                },
                LifecyclePolicyKnob::CorrectionRetainOriginalSourceObjects,
            ),
            (
                {
                    let mut draft = correction_draft(&ids);
                    draft.cascade_policy.require_original_source_match = false;
                    draft
                },
                LifecyclePolicyKnob::CorrectionRequireOriginalSourceMatch,
            ),
            (
                {
                    let mut draft = correction_draft(&ids);
                    draft.cascade_policy.cascade_to_threads = true;
                    draft
                },
                LifecyclePolicyKnob::CorrectionCascadeToThreads,
            ),
        ] {
            draft.rationale = format!("{} {}", draft.rationale, Uuid::new_v4());
            let graph =
                RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))]);
            let vector = RecordingVectorStore::default();
            let embedder = RecordingEmbedder::default();
            let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

            let error = pipeline.correct(draft).await.unwrap_err();

            assert!(matches!(
                error,
                CustomError::LifecyclePolicyUnsupported { knob } if knob == expected_knob
            ));
            assert!(graph.calls().is_empty());
            assert!(vector.calls().is_empty());
        }
    }

    #[tokio::test]
    async fn unsupported_forget_policy_knobs_fail_before_writes() {
        let ids = fixed_ids();
        for (mut draft, expected_knob) in [
            (
                {
                    let mut draft = ForgetMemoryDraft::suppress(
                        LifecycleTargetRef::DerivedMemory(ids.old),
                        "Suppress without dropping refs.",
                    );
                    draft
                        .lifecycle_policy
                        .suppression
                        .preserve_original_raw_refs = false;
                    draft
                },
                LifecyclePolicyKnob::ForgetPreserveOriginalRawRefs,
            ),
            (
                {
                    let mut draft = ForgetMemoryDraft::archive_thread(
                        ids.thread,
                        "Archive without dropping refs.",
                    );
                    draft.lifecycle_policy.archive.preserve_original_raw_refs = false;
                    draft
                },
                LifecyclePolicyKnob::ForgetArchivePreserveOriginalRawRefs,
            ),
        ] {
            draft.rationale = format!("{} {}", draft.rationale, Uuid::new_v4());
            let graph =
                RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))]);
            let vector = RecordingVectorStore::default();
            let embedder = RecordingEmbedder::default();
            let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

            let error = pipeline.forget(draft).await.unwrap_err();

            assert!(matches!(
                error,
                CustomError::LifecyclePolicyUnsupported { knob } if knob == expected_knob
            ));
            assert!(graph.calls().is_empty());
            assert!(vector.calls().is_empty());
        }
    }

    #[tokio::test]
    async fn graph_failure_prevents_vector_maintenance() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))])
            .fail_objects();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let error = pipeline.correct(correction_draft(&ids)).await.unwrap_err();

        assert!(error.to_string().contains("object write failed"));
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn graph_link_failure_prevents_partial_object_mutation_and_vector_maintenance() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))])
            .fail_links();
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let error = pipeline.correct(correction_draft(&ids)).await.unwrap_err();

        assert!(error.to_string().contains("link write failed"));
        assert_eq!(
            graph.calls(),
            vec![
                StoreCall::GraphQuery(vec![ids.old]),
                StoreCall::GraphObjects(vec![ids.old, ids.replacement]),
                StoreCall::GraphLinks(vec![(ids.replacement, ids.old)]),
            ]
        );
        assert_eq!(
            graph.object_refs(),
            vec![MemoryObjectRef::new(ObjectType::DerivedMemory, ids.old)]
        );
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn vector_failure_after_graph_success_returns_partial_outcome() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))]);
        let vector = RecordingVectorStore::default().fail_delete();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .correct(correction_draft(&ids))
            .await
            .expect("graph success should return partial vector failure");

        assert_eq!(
            outcome.graph_mutated_object_ids,
            vec![
                MemoryObjectRef::new(ObjectType::DerivedMemory, ids.old),
                MemoryObjectRef::new(ObjectType::DerivedMemory, ids.replacement),
            ]
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
            failure.unmaintained_object_ids(),
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
                message,
                ..
            }) if backend == "test" && message == "vector delete failed"
        ));
    }

    #[tokio::test]
    async fn correction_policy_can_mark_non_current_without_supersession_evidence() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::new(vec![MemoryObject::DerivedMemory(old_memory(&ids))]);
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft = correction_draft(&ids).with_trace();
        draft.lifecycle_policy.supersede_replaced_derived_memories = false;

        let outcome = pipeline.correct(draft).await.unwrap();

        assert_eq!(graph.calls()[2], StoreCall::GraphLinks(Vec::new()));
        let objects = graph
            .query_objects(&GraphObjectQuery::by_refs(vec![
                MemoryObjectRef::from_id_type(ids.replacement, ObjectType::DerivedMemory),
            ]))
            .await
            .unwrap();
        let MemoryObject::DerivedMemory(replacement) = &objects[0] else {
            panic!("expected replacement derived memory");
        };
        assert!(replacement.supersedes.is_empty());
        assert!(outcome.trace.unwrap().superseded_by.is_empty());
    }

    #[tokio::test]
    async fn source_object_correction_supersedes_provenanced_memories_without_rewriting_source() {
        let fixtures = representative_fixtures();
        let graph = FakeGraphAuthorityStore::new();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = FakeVectorCandidateStore::new();
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
                original_source_ref: fixtures.episode.source_conversation_id.clone(),
            }),
            "Correct episode-derived behavior.",
        )
        .with_replacement(replacement)
        .with_trace();
        draft.correction_origin =
            SourceProvenanceReference::observation(fixtures.salient_observation.id);

        let outcome = pipeline.correct(draft).await.unwrap();

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
        assert!(!old.is_current);
        assert_eq!(old.retention_state, RetentionState::Suppressed);
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
        let graph = FakeGraphAuthorityStore::new();
        let mut objects = fixtures.objects();
        objects.push(MemoryObject::DerivedMemory(observation_only.clone()));
        graph.upsert_objects(&objects).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = FakeVectorCandidateStore::new();
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
                original_source_ref: fixtures.episode.source_conversation_id.clone(),
            }),
            "Correct episode-derived observation-only behavior.",
        )
        .with_replacement(replacement);
        draft.correction_origin =
            SourceProvenanceReference::observation(fixtures.salient_observation.id);

        let outcome = pipeline.correct(draft).await.unwrap();

        assert!(outcome
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
        assert!(!old.is_current);
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
        let graph = RecordingGraphStore::new(vec![MemoryObject::Episode(source_episode(&ids))]);
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::source_object(SourceObjectCorrectionTarget::Episode {
                id: ids.episode,
                original_raw_ref: None,
                original_source_ref: None,
            }),
            "Correct source episode.",
        );
        draft.correction_origin = SourceProvenanceReference::episode(ids.episode);

        let error = pipeline.correct(draft).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("original raw or source reference"));
        assert!(graph.calls().is_empty());
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn source_object_correction_rejects_mismatched_episode_refs_before_writes() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::new(vec![MemoryObject::Episode(source_episode(&ids))]);
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::source_object(SourceObjectCorrectionTarget::Episode {
                id: ids.episode,
                original_raw_ref: Some("raw://wrong".to_owned()),
                original_source_ref: Some("conversation://original".to_owned()),
            }),
            "Correct source episode.",
        );
        draft.correction_origin = SourceProvenanceReference::episode(ids.episode);

        let error = pipeline.correct(draft).await.unwrap_err();

        assert!(error.to_string().contains("episode original raw reference"));
        assert_eq!(
            graph.calls(),
            vec![StoreCall::GraphQuery(vec![ids.episode])]
        );
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn source_object_correction_rejects_mismatched_observation_source_ref_before_writes() {
        let ids = fixed_ids();
        let graph = RecordingGraphStore::new(vec![
            MemoryObject::Episode(source_episode(&ids)),
            MemoryObject::Observation(source_observation(&ids)),
        ]);
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft = CorrectMemoryDraft::new(
            CorrectionTarget::source_object(SourceObjectCorrectionTarget::Observation {
                id: ids.observation,
                original_raw_ref: Some("raw://original/observation".to_owned()),
                original_source_ref: Some("conversation://wrong".to_owned()),
            }),
            "Correct source observation.",
        );
        draft.correction_origin = SourceProvenanceReference::observation(ids.observation);

        let error = pipeline.correct(draft).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("observation original source reference"));
        assert_eq!(
            graph.calls(),
            vec![
                StoreCall::GraphQuery(vec![ids.observation]),
                StoreCall::GraphQuery(vec![ids.episode]),
            ]
        );
        assert!(vector.calls().is_empty());
    }

    #[tokio::test]
    async fn source_correction_cascade_warns_when_suppressing_current_replacement() {
        let fixtures = representative_fixtures();
        let current_replacement_id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_9201);
        let next_replacement_id = Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_9202);
        let current_replacement =
            current_replacement_from(&fixtures.user_preference, current_replacement_id);
        let graph = FakeGraphAuthorityStore::new();
        let mut objects = fixtures.objects();
        objects.push(MemoryObject::DerivedMemory(current_replacement.clone()));
        graph.upsert_objects(&objects).await.unwrap();
        let mut links = fixtures.links();
        links.push(supersedes_link(
            current_replacement.id,
            fixtures.user_preference.id,
            "Establish current replacement.",
        ));
        graph.upsert_links(&links).await.unwrap();
        let vector = FakeVectorCandidateStore::new();
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
                original_source_ref: fixtures.episode.source_conversation_id.clone(),
            }),
            "Correct the source again.",
        )
        .with_replacement(replacement);
        draft.correction_origin =
            SourceProvenanceReference::observation(fixtures.salient_observation.id);

        let outcome = pipeline.correct(draft).await.unwrap();

        assert_eq!(
            outcome.diagnostics.warnings,
            vec![LifecycleMutationWarning {
                reason: LifecycleMutationWarningReason::CascadeSuppressesCurrentReplacement,
                affected_memory_ids: vec![fixtures.correction.id, current_replacement_id],
            }]
        );
        assert!(outcome
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
        let graph = FakeGraphAuthorityStore::new();
        let mut objects = fixtures.objects();
        objects.push(MemoryObject::DerivedMemory(current_replacement.clone()));
        graph.upsert_objects(&objects).await.unwrap();
        let mut links = fixtures.links();
        links.push(supersedes_link(
            current_replacement.id,
            fixtures.user_preference.id,
            "Establish current replacement.",
        ));
        graph.upsert_links(&links).await.unwrap();
        let vector = FakeVectorCandidateStore::new();
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .forget(ForgetMemoryDraft::suppress(
                LifecycleTargetRef::Episode(fixtures.episode.id),
                "Forget the source with its current replacement.",
            ))
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
            outcome.graph_mutated_object_ids,
            vec![
                MemoryObjectRef::new(ObjectType::Episode, fixtures.episode.id),
                MemoryObjectRef::new(ObjectType::DerivedMemory, fixtures.derived_reflection.id),
                MemoryObjectRef::new(ObjectType::DerivedMemory, fixtures.open_loop.id),
                MemoryObjectRef::new(ObjectType::DerivedMemory, fixtures.commitment.id),
                MemoryObjectRef::new(ObjectType::DerivedMemory, fixtures.correction.id),
                MemoryObjectRef::new(ObjectType::DerivedMemory, current_replacement_id),
            ]
        );
        assert!(outcome.graph_mutated_link_ids.is_empty());
        assert_eq!(
            outcome.vector_maintained_object_ids,
            outcome.graph_mutated_object_ids
        );
        assert!(outcome.vector_maintenance_failure.is_none());
        assert!(outcome.trace.is_none());
    }

    #[tokio::test]
    async fn forget_cascade_does_not_warn_when_draft_retention_archives_replacements() {
        let fixtures = representative_fixtures();
        let graph = FakeGraphAuthorityStore::new();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = FakeVectorCandidateStore::new();
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft = ForgetMemoryDraft::suppress(
            LifecycleTargetRef::Episode(fixtures.episode.id),
            "Archive the source and its derived memories.",
        );
        draft.target_retention_state = RetentionState::Archived;

        let outcome = pipeline.forget(draft).await.unwrap();

        assert!(outcome.diagnostics.warnings.is_empty());
        assert!(outcome
            .graph_mutated_object_ids
            .contains(&MemoryObjectRef::new(
                ObjectType::DerivedMemory,
                fixtures.correction.id,
            )));
    }

    #[tokio::test]
    async fn forget_suppresses_source_and_dependent_derived_memories_and_deletes_vectors() {
        let fixtures = representative_fixtures();
        let graph = FakeGraphAuthorityStore::new();
        let mut objects = fixtures.objects();
        for object in &mut objects {
            if let MemoryObject::DerivedMemory(memory) = object {
                memory.supersedes.clear();
            }
        }
        graph.upsert_objects(&objects).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = FakeVectorCandidateStore::new();
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .forget(ForgetMemoryDraft::suppress(
                LifecycleTargetRef::Observation(fixtures.salient_observation.id),
                "Forget source observation.",
            ))
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
                memory.retention_state == RetentionState::Suppressed && !memory.is_current
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
        let graph = FakeGraphAuthorityStore::new();
        let mut objects = fixtures.objects();
        objects.push(MemoryObject::DerivedMemory(observation_only.clone()));
        graph.upsert_objects(&objects).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = FakeVectorCandidateStore::new();
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .forget(ForgetMemoryDraft::suppress(
                LifecycleTargetRef::Episode(fixtures.episode.id),
                "Forget source episode.",
            ))
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
        assert!(!memory.is_current);
    }

    #[tokio::test]
    async fn forget_policy_can_suppress_source_without_derived_cascade() {
        let fixtures = representative_fixtures();
        let graph = FakeGraphAuthorityStore::new();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = FakeVectorCandidateStore::new();
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

        let outcome = pipeline.forget(draft).await.unwrap();

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
                    && memory.is_current
        )));
    }

    #[tokio::test]
    async fn forget_policy_can_skip_source_suppression_without_vector_deleting_source() {
        let fixtures = representative_fixtures();
        let graph = FakeGraphAuthorityStore::new();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = FakeVectorCandidateStore::new();
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

        let outcome = pipeline.forget(draft).await.unwrap();

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
    async fn forget_archives_memory_thread() {
        let fixtures = representative_fixtures();
        let graph = FakeGraphAuthorityStore::new();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        let vector = FakeVectorCandidateStore::new();
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);

        let outcome = pipeline
            .forget(ForgetMemoryDraft::archive_thread(
                fixtures.soft_thread.id,
                "Archive thread.",
            ))
            .await
            .unwrap();

        assert_eq!(
            outcome.graph_mutated_object_ids,
            vec![MemoryObjectRef::new(
                ObjectType::MemoryThread,
                fixtures.soft_thread.id,
            )]
        );
        let objects = graph
            .query_objects(&GraphObjectQuery::by_refs(vec![
                MemoryObjectRef::from_id_type(fixtures.soft_thread.id, ObjectType::MemoryThread),
            ]))
            .await
            .unwrap();
        let MemoryObject::MemoryThread(thread) = &objects[0] else {
            panic!("expected memory thread");
        };
        assert_eq!(thread.status, ThreadStatus::Archived);
    }

    #[tokio::test]
    async fn forget_policy_can_skip_thread_archive_without_vector_deleting_thread() {
        let fixtures = representative_fixtures();
        let graph = FakeGraphAuthorityStore::new();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        let vector = FakeVectorCandidateStore::new();
        let embedder = DeterministicMemoryEmbedder::new(4);
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft =
            ForgetMemoryDraft::archive_thread(fixtures.soft_thread.id, "Do not archive thread.");
        draft.lifecycle_policy.archive.archive_thread = false;

        let outcome = pipeline.forget(draft).await.unwrap();

        assert!(outcome.graph_mutated_object_ids.is_empty());
        assert!(outcome.vector_maintained_object_ids.is_empty());
        let objects = graph
            .query_objects(&GraphObjectQuery::by_refs(vec![
                MemoryObjectRef::from_id_type(fixtures.soft_thread.id, ObjectType::MemoryThread),
            ]))
            .await
            .unwrap();
        assert!(matches!(
            &objects[0],
            MemoryObject::MemoryThread(thread) if thread.status == ThreadStatus::Active
        ));
    }

    #[tokio::test]
    async fn forget_thread_cascade_uses_thread_membership_query() {
        let ids = fixed_ids();
        let mut thread = representative_fixtures().soft_thread;
        thread.id = ids.thread;
        let mut current_replacement = old_memory(&ids);
        current_replacement.supersedes.push(ids.replacement);
        let graph = RecordingGraphStore::new(vec![
            MemoryObject::MemoryThread(thread),
            MemoryObject::DerivedMemory(current_replacement),
        ]);
        let vector = RecordingVectorStore::default();
        let embedder = RecordingEmbedder::default();
        let pipeline = CorrectionForgetPipeline::new(&graph, &vector, &embedder);
        let mut draft = ForgetMemoryDraft::archive_thread(ids.thread, "Archive thread members.");
        draft
            .lifecycle_policy
            .archive
            .archive_thread_derived_memories = true;

        let outcome = pipeline.forget(draft).await.unwrap();

        assert!(outcome.diagnostics.warnings.is_empty());
        assert_eq!(
            graph.calls(),
            vec![
                StoreCall::GraphQuery(vec![ids.thread]),
                StoreCall::GraphThreadQuery(vec![ids.thread]),
                StoreCall::GraphObjects(vec![ids.old, ids.thread]),
            ]
        );
        assert_eq!(
            outcome.graph_mutated_object_ids,
            vec![
                MemoryObjectRef::new(ObjectType::DerivedMemory, ids.old),
                MemoryObjectRef::new(ObjectType::MemoryThread, ids.thread),
            ]
        );
        assert_eq!(
            outcome.vector_maintained_object_ids,
            vec![
                MemoryObjectRef::new(ObjectType::DerivedMemory, ids.old),
                MemoryObjectRef::new(ObjectType::MemoryThread, ids.thread),
            ]
        );
    }

    #[tokio::test]
    async fn retrieval_excludes_stale_superseded_candidate_when_vector_cleanup_fails() {
        let ids = fixed_ids();
        let graph = FakeGraphAuthorityStore::new();
        graph
            .upsert_objects(&[MemoryObject::DerivedMemory(old_memory(&ids))])
            .await
            .unwrap();
        let vector = DeleteFailingVectorStore::new();
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
        let outcome = pipeline.correct(correction_draft(&ids)).await.unwrap();
        assert!(outcome.vector_maintenance_failure.is_some());

        let retrieval = RetrievePipeline::new(&graph, &vector, &embedder)
            .retrieve(RetrievalContext::new("stable correction behavior").with_trace())
            .await
            .unwrap();

        assert!(
            !trace_contains_candidate(
                retrieval
                    .trace
                    .as_ref()
                    .unwrap()
                    .vector_candidates
                    .as_slice(),
                ids.old,
            ) || retrieval
                .trace
                .as_ref()
                .unwrap()
                .stale_candidate_omissions
                .iter()
                .any(|omission| {
                    omission.candidate.id == ids.old
                        && matches!(omission.reason, StaleCandidateReason::LifecycleMismatch)
                })
        );
        assert!(!pack_contains_derived_memory(&retrieval.pack, ids.old));
    }

    #[tokio::test]
    async fn retrieval_excludes_stale_source_and_dependent_candidates_when_forget_cleanup_fails() {
        let fixtures = representative_fixtures();
        let graph = FakeGraphAuthorityStore::new();
        graph.upsert_objects(&fixtures.objects()).await.unwrap();
        graph.upsert_links(&fixtures.links()).await.unwrap();
        let vector = DeleteFailingVectorStore::new();
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
            .forget(ForgetMemoryDraft::suppress(
                LifecycleTargetRef::Episode(fixtures.episode.id),
                "Forget source episode.",
            ))
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

    fn old_memory(ids: &FixedIds) -> DerivedMemory {
        DerivedMemory {
            id: ids.old,
            object_type: ObjectType::DerivedMemory,
            derived_type: DerivedType::UserPreference,
            text: "Prefer stale lifecycle behavior.".to_owned(),
            derived_from_episode_ids: vec![ids.episode],
            derived_from_observation_ids: vec![ids.observation],
            thread_ids: vec![ids.thread],
            entity_ids: Vec::new(),
            confidence: 0.8,
            salience_score: 0.7,
            stability: Stability::Medium,
            is_current: true,
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
        replacement.is_current = true;
        replacement.retention_state = RetentionState::Active;
        replacement
    }

    fn source_episode(ids: &FixedIds) -> Episode {
        Episode {
            id: ids.episode,
            object_type: ObjectType::Episode,
            modality: Modality::Chat,
            source_conversation_id: Some("conversation://original".to_owned()),
            started_at: None,
            ended_at: None,
            participant_entity_ids: Vec::new(),
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

    fn trace_contains_candidate(trace: &[VectorCandidateTrace], object_id: MemoryId) -> bool {
        trace
            .iter()
            .any(|candidate| candidate.object.id == object_id)
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

    #[derive(Debug, Default)]
    struct RecordingGraphStore {
        objects: Mutex<Vec<MemoryObject>>,
        links: Mutex<Vec<MemoryLink>>,
        calls: Arc<Mutex<Vec<StoreCall>>>,
        fail_objects: bool,
        fail_links: bool,
    }

    impl RecordingGraphStore {
        fn new(objects: Vec<MemoryObject>) -> Self {
            Self {
                objects: Mutex::new(objects),
                ..Self::default()
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

        fn object_refs(&self) -> Vec<MemoryObjectRef> {
            let mut refs = lock(&self.objects)
                .iter()
                .map(MemoryObject::object_ref)
                .collect::<Vec<_>>();
            sort_refs(&mut refs);
            refs
        }
    }

    #[async_trait]
    impl GraphAuthorityStore for RecordingGraphStore {
        async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError> {
            lock(&self.calls).push(StoreCall::GraphObjects(
                objects.iter().map(MemoryObject::id).collect(),
            ));
            if self.fail_objects {
                return Err(CustomError::DatabaseError("object write failed".to_owned()));
            }
            let mut stored = lock(&self.objects);
            for object in objects {
                let object_ref = object.object_ref();
                stored.retain(|existing| existing.object_ref() != object_ref);
                stored.push(object.clone());
            }
            Ok(())
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
            lock(&self.links).extend_from_slice(links);
            Ok(())
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
            let mut stored = lock(&self.objects);
            for object in objects {
                let object_ref = object.object_ref();
                stored.retain(|existing| existing.object_ref() != object_ref);
                stored.push(object.clone());
            }
            lock(&self.links).extend_from_slice(links);
            Ok(())
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
            Ok(lock(&self.objects)
                .iter()
                .filter(|object| {
                    let object_ref = object.object_ref();
                    match query {
                        GraphObjectQuery::ByRefs(object_refs) => {
                            object_refs.iter().any(|query_ref| {
                                query_ref.id == object_ref.id
                                    && query_ref.object_type == object_ref.object_type
                            })
                        }
                        GraphObjectQuery::ByIds(object_ids) => object_ids.contains(&object_ref.id),
                        GraphObjectQuery::ByTypes { object_types, .. } => {
                            object_types.contains(&object_ref.object_type)
                        }
                    }
                })
                .cloned()
                .collect())
        }

        async fn query_links_by_ids(
            &self,
            link_ids: &[MemoryId],
        ) -> Result<Vec<MemoryLink>, CustomError> {
            Ok(lock(&self.links)
                .iter()
                .filter(|link| link_ids.contains(&link.id))
                .cloned()
                .collect())
        }

        async fn query_derived_memories_by_provenance(
            &self,
            _query: &GraphDerivedMemoryProvenanceQuery,
        ) -> Result<Vec<DerivedMemory>, CustomError> {
            Ok(Vec::new())
        }

        async fn query_derived_memories_by_thread(
            &self,
            query: &GraphDerivedMemoryThreadQuery,
        ) -> Result<Vec<DerivedMemory>, CustomError> {
            lock(&self.calls).push(StoreCall::GraphThreadQuery(query.thread_ids.clone()));
            Ok(lock(&self.objects)
                .iter()
                .filter_map(|object| match object {
                    MemoryObject::DerivedMemory(memory) => Some(memory.clone()),
                    _ => None,
                })
                .filter(|memory| {
                    memory
                        .thread_ids
                        .iter()
                        .any(|thread_id| query.thread_ids.contains(thread_id))
                })
                .collect())
        }

        async fn expand_bounded(
            &self,
            _query: &GraphExpansionQuery,
        ) -> Result<GraphExpansion, CustomError> {
            Ok(GraphExpansion::new(Vec::new(), Vec::new()))
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
            _query: &VectorCandidateSearch,
        ) -> Result<CanonicalCandidates, CustomError> {
            Ok(CanonicalCandidates::new([]))
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

    impl RecordingEmbedder {
        fn calls(&self) -> Vec<StoreCall> {
            lock(&self.calls).clone()
        }
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

        async fn mark_unhealthy(
            &self,
            _cause: RetrievalStatsHealthCause,
        ) -> Result<(), RetrievalStatsStoreError> {
            lock(&self.calls).push(StoreCall::StatsUnhealthy);
            Ok(())
        }
    }

    #[derive(Debug)]
    struct DeleteFailingVectorStore {
        inner: FakeVectorCandidateStore,
    }

    impl DeleteFailingVectorStore {
        fn new() -> Self {
            Self {
                inner: FakeVectorCandidateStore::new(),
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
        ) -> Result<CanonicalCandidates, CustomError> {
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
