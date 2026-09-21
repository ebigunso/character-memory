use super::*;
use crate::api::types::{
    MemoryScenes, SceneReference, SceneReferenceResolution, SceneReferenceResult, SourceScene,
    SourceSceneUnavailableReason, VectorRecallCompleteness,
};
use crate::domain::RetentionState;
use crate::models::vector::CanonicalCandidates;
use crate::ports::graph_authority::GraphObjectQuery;

pub(super) struct RecallCues {
    pub candidates: CanonicalCandidates,
    pub kinds: HashMap<MemoryObjectRef, BTreeSet<CueKind>>,
    pub orders: BTreeMap<CueKind, Vec<MemoryObjectRef>>,
    pub participants: Vec<MemoryId>,
    pub references: Vec<SceneReferenceResult>,
    pub dimension: usize,
    pub completeness: VectorRecallCompleteness,
    pub floor_admissions: Vec<CueFloorAdmission>,
}

impl<G, V, E> RetrievePipeline<'_, G, V, E>
where
    G: GraphAuthorityStore + ?Sized,
    V: VectorCandidateStore + ?Sized,
    E: MemoryEmbedder + ?Sized,
{
    pub(super) async fn recall_cues(
        &self,
        context: &RetrievalContext,
    ) -> Result<RecallCues, CustomError> {
        let keys = context.scene.participant_keys().collect::<HashSet<_>>();
        let known_keys = self
            .graph_store
            .query_objects(&GraphObjectQuery::by_refs(
                keys.into_iter()
                    .map(|id| MemoryObjectRef::new(ObjectType::Entity, id))
                    .collect(),
            ))
            .await?
            .into_iter()
            .map(|object| object.id())
            .collect::<HashSet<_>>();
        let mut participants = Vec::new();
        let mut names = HashMap::new();
        let mut references = Vec::new();
        let mut descriptions = Vec::new();
        for (index, participant) in context.scene.participants.iter().enumerate() {
            if let Some(key) = participant.key {
                let resolution = if known_keys.contains(&key) {
                    participants.push(key);
                    SceneReferenceResolution::Resolved { notion_id: key }
                } else {
                    SceneReferenceResolution::Unknown
                };
                references.push(SceneReferenceResult {
                    reference: SceneReference::ParticipantKey { index },
                    resolution,
                });
            }
            if let Some(name) = nonblank(participant.name.as_deref()) {
                let normalized = crate::domain::belief::normalize_name(name);
                if !names.contains_key(&normalized) {
                    let mut ids = self.graph_store.query_notions_known_as(name).await?;
                    ids.sort();
                    ids.dedup();
                    names.insert(normalized.clone(), ids);
                }
                let ids = &names[&normalized];
                participants.extend(ids);
                let resolution = match ids.as_slice() {
                    [] => SceneReferenceResolution::Unknown,
                    [notion_id] => SceneReferenceResolution::Resolved {
                        notion_id: *notion_id,
                    },
                    _ => SceneReferenceResolution::Ambiguous {
                        notion_ids: ids.clone(),
                    },
                };
                references.push(SceneReferenceResult {
                    reference: SceneReference::ParticipantName { index },
                    resolution,
                });
            }
            if let Some(description) = nonblank(participant.description.as_deref()) {
                descriptions.push((
                    SceneReference::ParticipantDescription { index },
                    description,
                ));
            }
        }
        if let Some(words) = nonblank(context.scene.setting.words.as_deref()) {
            descriptions.push((SceneReference::SettingWords, words));
        }
        let mut seen = HashSet::new();
        participants.retain(|id| seen.insert(*id));
        let mut searches = HashMap::new();
        let mut kinds: HashMap<MemoryObjectRef, BTreeSet<CueKind>> = HashMap::new();
        let mut all_candidates = Vec::new();
        let mut candidates_by_kind: BTreeMap<CueKind, Vec<VectorCandidateMatch>> = BTreeMap::new();
        let mut dimension = 0;
        let mut completeness = VectorRecallCompleteness::NotRequested;
        let topic = nonblank(context.topic.as_deref()).map(|text| (None, text));
        for (reference, text) in topic.into_iter().chain(
            descriptions
                .into_iter()
                .map(|(reference, text)| (Some(reference), text)),
        ) {
            if !searches.contains_key(text) {
                let input = EmbeddingInput::new(None, None, VectorSurface::Query, text);
                let embedding = self.embedder.embed(&input).await?;
                dimension = embedding.len();
                let query = VectorCandidateSearch::new(
                    embedding,
                    context.candidate_limits.max_vector_candidates,
                    context.object_type_defaults.clone(),
                );
                let recall = self.vector_store.search_candidates(&query).await?;
                completeness = merge_completeness(completeness, recall.completeness);
                all_candidates.extend(recall.candidates.iter().cloned());
                searches.insert(text, recall.candidates);
            }
            let kind = match &reference {
                None => CueKind::Topic,
                Some(SceneReference::SettingWords) => CueKind::Place,
                Some(_) => CueKind::Participant,
            };
            let search = &searches[text];
            candidates_by_kind
                .entry(kind)
                .or_default()
                .extend(search.iter().cloned());
            for candidate in search.iter() {
                let object = MemoryObjectRef::new(candidate.object_type, candidate.object_id);
                kinds.entry(object).or_default().insert(kind);
            }
            if let Some(reference) = reference {
                references.push(SceneReferenceResult {
                    reference,
                    resolution: SceneReferenceResolution::ContentCue,
                });
            }
        }
        let canonical = CanonicalCandidates::new(all_candidates);
        let mut seen = HashSet::new();
        let candidates = canonical
            .iter()
            .filter(|candidate| {
                seen.insert(MemoryObjectRef::new(
                    candidate.object_type,
                    candidate.object_id,
                ))
            })
            .cloned()
            .collect::<Vec<_>>();
        let orders = candidates_by_kind
            .into_iter()
            .map(|(kind, candidates)| {
                let ordered = CanonicalCandidates::new(candidates);
                let mut seen = HashSet::new();
                let order = ordered
                    .iter()
                    .map(|candidate| {
                        MemoryObjectRef::new(candidate.object_type, candidate.object_id)
                    })
                    .filter(|object| seen.insert(*object))
                    .collect();
                (kind, order)
            })
            .collect();
        let selection = select_with_cue_floors(
            candidates.iter().map(|candidate| {
                let object = MemoryObjectRef::new(candidate.object_type, candidate.object_id);
                (object, &kinds[&object])
            }),
            &orders,
            context.candidate_limits.max_vector_candidates,
            context.cue_floors,
            false,
        );
        let mut floor_admissions = Vec::new();
        let selected = selection
            .into_iter()
            .map(|(index, cause)| {
                let candidate = &candidates[index];
                if let Some(cue_kind) = cause {
                    floor_admissions.push(CueFloorAdmission {
                        object: MemoryObjectRef::new(candidate.object_type, candidate.object_id),
                        stage: CueFloorStage::CandidateMerge,
                        cue_kind,
                    });
                }
                candidate.clone()
            })
            .collect::<Vec<_>>();
        Ok(RecallCues {
            candidates: CanonicalCandidates::new(selected),
            kinds,
            orders,
            participants,
            references,
            dimension,
            completeness,
            floor_admissions,
        })
    }

    pub(super) async fn memory_scenes(
        &self,
        pack: &ContinuityContextPack,
        include_suppressed: bool,
    ) -> Result<Vec<MemoryScenes>, CustomError> {
        let mut memories = Vec::new();
        let mut objects = HashMap::new();
        for episode in &pack.relevant_episodes {
            let source = MemoryObjectRef::new(ObjectType::Episode, episode.id);
            memories.push((source, vec![source]));
            objects.insert(source, MemoryObject::Episode(episode.clone()));
        }
        for observation in &pack.salient_observations {
            let source = MemoryObjectRef::new(ObjectType::Observation, observation.id);
            memories.push((source, vec![source]));
            objects.insert(source, MemoryObject::Observation(observation.clone()));
        }
        for thread in &pack.active_threads {
            memories.push((
                MemoryObjectRef::new(ObjectType::MemoryThread, thread.id),
                Vec::new(),
            ));
        }
        for included in pack
            .derived_memories
            .iter()
            .chain(&pack.preferences)
            .chain(&pack.relationship_notes)
            .chain(&pack.open_loops)
            .chain(&pack.commitments)
            .chain(&pack.character_signals)
        {
            let memory = &included.memory;
            let sources = memory
                .derived_from_episode_ids
                .iter()
                .map(|id| MemoryObjectRef::new(ObjectType::Episode, *id))
                .chain(
                    memory
                        .derived_from_observation_ids
                        .iter()
                        .map(|id| MemoryObjectRef::new(ObjectType::Observation, *id)),
                )
                .collect::<Vec<_>>();
            memories.push((
                MemoryObjectRef::new(ObjectType::DerivedMemory, memory.id),
                sources,
            ));
        }
        let missing = memories
            .iter()
            .flat_map(|(_, sources)| sources)
            .filter(|source| !objects.contains_key(source))
            .copied()
            .collect::<HashSet<_>>();
        for object in self
            .graph_store
            .query_objects(&GraphObjectQuery::by_refs(missing.into_iter().collect()))
            .await?
        {
            objects.insert(object.object_ref(), object);
        }
        let parents = objects
            .values()
            .filter_map(|object| match object {
                MemoryObject::Observation(observation)
                    if include_suppressed
                        || observation.retention_state != RetentionState::Suppressed =>
                {
                    Some(MemoryObjectRef::new(
                        ObjectType::Episode,
                        observation.episode_id,
                    ))
                }
                _ => None,
            })
            .filter(|source| !objects.contains_key(source))
            .collect::<HashSet<_>>();
        for object in self
            .graph_store
            .query_objects(&GraphObjectQuery::by_refs(parents.into_iter().collect()))
            .await?
        {
            objects.insert(object.object_ref(), object);
        }
        memories.sort_by_key(|(memory, _)| (memory.object_type.stable_rank(), memory.id));
        Ok(memories
            .into_iter()
            .map(|(memory, sources)| {
                let mut seen = HashSet::new();
                let mut sources = sources
                    .into_iter()
                    .map(|source| source_scene(source, &objects, include_suppressed))
                    .filter(|scene| seen.insert(source_scene_ref(scene)))
                    .collect::<Vec<_>>();
                sources.sort_by_key(|scene| {
                    let source = source_scene_ref(scene);
                    (source.object_type.stable_rank(), source.id)
                });
                MemoryScenes { memory, sources }
            })
            .collect())
    }
}

fn source_scene(
    source: MemoryObjectRef,
    objects: &HashMap<MemoryObjectRef, MemoryObject>,
    include_suppressed: bool,
) -> SourceScene {
    let unavailable = |source, reason| SourceScene::Unavailable { source, reason };
    match objects.get(&source) {
        Some(MemoryObject::Episode(episode)) => {
            if !include_suppressed && episode.retention_state == RetentionState::Suppressed {
                unavailable(source, SourceSceneUnavailableReason::Forgotten)
            } else {
                SourceScene::Recorded {
                    episode_id: episode.id,
                    scene: episode.scene.clone(),
                }
            }
        }
        Some(MemoryObject::Observation(observation)) => {
            if !include_suppressed && observation.retention_state == RetentionState::Suppressed {
                unavailable(source, SourceSceneUnavailableReason::Forgotten)
            } else {
                source_scene(
                    MemoryObjectRef::new(ObjectType::Episode, observation.episode_id),
                    objects,
                    include_suppressed,
                )
            }
        }
        _ => unavailable(source, SourceSceneUnavailableReason::Missing),
    }
}

fn source_scene_ref(scene: &SourceScene) -> MemoryObjectRef {
    match scene {
        SourceScene::Recorded { episode_id, .. } => {
            MemoryObjectRef::new(ObjectType::Episode, *episode_id)
        }
        SourceScene::Unavailable { source, .. } => *source,
    }
}

fn nonblank(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

fn merge_completeness(
    left: VectorRecallCompleteness,
    right: VectorRecallCompleteness,
) -> VectorRecallCompleteness {
    use VectorRecallCompleteness::*;
    match (left, right) {
        (NotRequested, other) | (other, NotRequested) => other,
        (BoundaryTieOpen { .. }, _) => left,
        (_, BoundaryTieOpen { .. }) => right,
        (BoundaryTieClosed { .. }, _) => left,
        (_, BoundaryTieClosed { .. }) => right,
        _ => left,
    }
}
