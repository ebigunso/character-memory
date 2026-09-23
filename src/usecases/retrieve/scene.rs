use super::*;
use crate::api::types::{
    LastInteraction, MemoryScenes, SceneCueSearchTrace, SceneReference, SceneReferenceResolution,
    SceneReferenceResult, SourceScene, SourceSceneUnavailableReason, VectorRecallCompleteness,
};
use crate::domain::RetentionState;
use crate::models::vector::CanonicalCandidates;
use crate::ports::graph_authority::GraphObjectQuery;
use chrono::{DateTime, Utc};

pub(super) struct RecallCues {
    pub candidates: CanonicalCandidates,
    pub roots: Vec<CandidateRoot>,
    pub orders: BTreeMap<RecallRoad, Vec<MemoryObjectRef>>,
    pub participants: Vec<MemoryId>,
    pub references: Vec<SceneReferenceResult>,
    pub dimension: usize,
    pub completeness: VectorRecallCompleteness,
    pub floor_admissions: Vec<CueFloorAdmission>,
    pub scene_cue_searches: Vec<SceneCueSearchTrace>,
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
                    last_interactions: BTreeMap::new(),
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
                    last_interactions: BTreeMap::new(),
                });
            }
            if nonblank(participant.description.as_deref()).is_some() {
                descriptions.push(SceneReference::ParticipantDescription { index });
            }
        }
        if nonblank(context.scene.setting.words.as_deref()).is_some() {
            descriptions.push(SceneReference::SettingWords);
        }
        let mut seen = HashSet::new();
        participants.retain(|id| seen.insert(*id));
        let policy = GraphExpansionLifecyclePolicy::from(context.lifecycle_policy);
        let mut last_interactions = HashMap::new();
        for &participant in &participants {
            let last = self
                .graph_store
                .query_last_interaction(participant, context.scene.time.to_utc(), policy)
                .await?;
            last_interactions.insert(
                participant,
                last.map(|(episode_id, scene_time)| LastInteraction {
                    episode_id,
                    scene_time,
                    seconds_since: (context.scene.time.to_utc() - scene_time).num_seconds(),
                }),
            );
        }
        for reference in &mut references {
            let ids = match &reference.resolution {
                SceneReferenceResolution::Resolved { notion_id } => std::slice::from_ref(notion_id),
                SceneReferenceResolution::Ambiguous { notion_ids } => notion_ids.as_slice(),
                _ => &[],
            };
            reference.last_interactions = ids
                .iter()
                .map(|id| (*id, last_interactions[id].clone()))
                .collect();
        }
        let mut searches = HashMap::new();
        let mut embeddings = HashMap::new();
        let mut reaches: HashMap<MemoryObjectRef, BTreeMap<RecallRoad, RoadReach>> = HashMap::new();
        let mut all_candidates = Vec::new();
        let mut candidates_by_kind: BTreeMap<RecallRoad, Vec<VectorCandidateMatch>> =
            BTreeMap::new();
        let mut scene_cue_searches = Vec::new();
        let mut dimension = 0;
        let mut completeness = VectorRecallCompleteness::NotRequested;
        let topic = nonblank(context.topic.as_deref()).map(|text| {
            (
                RecallRoad::Topic,
                vec![
                    VectorSurface::Summary,
                    VectorSurface::Text,
                    VectorSurface::DerivedText,
                ],
                text.to_owned(),
            )
        });
        let scene_cues = crate::policy::embedding_surface::scene_surface_texts(&context.scene)
            .into_iter()
            .filter(|(_, text)| !text.is_empty())
            .map(|(surface, text)| {
                (
                    if surface == VectorSurface::SceneSetting {
                        RecallRoad::SettingWords
                    } else {
                        RecallRoad::ParticipantDescription
                    },
                    vec![surface],
                    text,
                )
            });
        for (road, surfaces, text) in topic.into_iter().chain(scene_cues) {
            let kind = road.rule().kind;
            let key = (text.clone(), surfaces.clone());
            if !searches.contains_key(&key) {
                if !embeddings.contains_key(&text) {
                    let input = EmbeddingInput::new(None, None, VectorSurface::Query, &text);
                    embeddings.insert(text.clone(), self.embedder.embed(&input).await?);
                }
                let embedding = embeddings[&text].clone();
                dimension = embedding.len();
                let mut query = VectorCandidateSearch::new(
                    embedding,
                    RecallRoad::Topic.contribution(context),
                    context.object_type_defaults.clone(),
                );
                query.surfaces = surfaces;
                let recall = self.vector_store.search_candidates(&query).await?;
                completeness = merge_completeness(completeness, recall.completeness);
                let mut candidates = if kind == CueKind::Topic {
                    recall.candidates.iter().cloned().collect::<Vec<_>>()
                } else {
                    let pool = &recall.scene_pool;
                    let objects = pool
                        .iter()
                        .map(|candidate| {
                            MemoryObjectRef::new(candidate.object_type, candidate.object_id)
                        })
                        .collect::<Vec<_>>();
                    let occasions = self.graph_store.query_episode_occasions(&objects).await?;
                    let mut eligible = pool
                        .iter()
                        .filter(|candidate| {
                            let object =
                                MemoryObjectRef::new(candidate.object_type, candidate.object_id);
                            occasions.get(&object).is_some_and(|occasion| {
                                occasion.time <= context.scene.time
                                    && occasion.filtered_reason(object, policy).is_none()
                            })
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    eligible.sort_by_key(|candidate| {
                        let object =
                            MemoryObjectRef::new(candidate.object_type, candidate.object_id);
                        (
                            std::cmp::Reverse(occasions[&object].time),
                            candidate.object_id,
                        )
                    });
                    let count = eligible.len();
                    let limit = road.contribution(context);
                    eligible.truncate(limit);
                    if context.include_trace {
                        scene_cue_searches.push(SceneCueSearchTrace {
                            cue_kind: kind,
                            references: references
                                .iter()
                                .map(|result| &result.reference)
                                .chain(&descriptions)
                                .filter(|reference| {
                                    matches!(
                                        (kind, reference),
                                        (CueKind::Place, SceneReference::SettingWords)
                                            | (
                                                CueKind::Participant,
                                                SceneReference::ParticipantName { .. }
                                                    | SceneReference::ParticipantDescription { .. }
                                            )
                                    )
                                })
                                .cloned()
                                .collect(),
                            best_score: pool.first().map(|candidate| candidate.score),
                            omitted_count: count - eligible.len(),
                        });
                    }
                    eligible
                };
                // Clamping changes strength, not this road's raw search order.
                for candidate in &mut candidates {
                    candidate.score = if candidate.score > 0.0 {
                        candidate.score
                    } else {
                        0.0
                    };
                }
                all_candidates.extend(candidates.iter().cloned());
                searches.insert(key.clone(), candidates);
            }
            let search = &searches[&key];
            candidates_by_kind
                .entry(road)
                .or_default()
                .extend(search.iter().cloned());
            for (position, candidate) in search.iter().enumerate() {
                let object = MemoryObjectRef::new(candidate.object_type, candidate.object_id);
                reaches
                    .entry(object)
                    .or_default()
                    .entry(road)
                    .and_modify(|reach| {
                        reach.score = reach.score.max(candidate.score);
                        reach.position = reach.position.min(position);
                    })
                    .or_insert(RoadReach {
                        score: candidate.score,
                        position,
                    });
            }
        }
        references.extend(
            descriptions
                .into_iter()
                .map(|reference| SceneReferenceResult {
                    reference,
                    resolution: SceneReferenceResolution::Reminder,
                    last_interactions: BTreeMap::new(),
                }),
        );
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
                let mut seen = HashSet::new();
                let order = candidates
                    .iter()
                    .map(|candidate| {
                        MemoryObjectRef::new(candidate.object_type, candidate.object_id)
                    })
                    .filter(|object| seen.insert(*object))
                    .collect();
                (kind, order)
            })
            .collect();
        let roots = candidates
            .iter()
            .map(|candidate| {
                let object = MemoryObjectRef::new(candidate.object_type, candidate.object_id);
                CandidateRoot {
                    object,
                    roads: reaches.remove(&object).unwrap(),
                    vector_score: Some(candidate.score),
                    memory_rank: None,
                }
            })
            .collect::<Vec<_>>();
        let selection = select_with_cue_floors(
            roots.iter().map(|root| (root.object, root.road_set())),
            &orders,
            RecallRoad::Topic.contribution(context),
            context.cue_floors,
            CueFloorStage::CandidateMerge,
        );
        let mut floor_admissions = Vec::new();
        let mut selection = selection.into_iter().peekable();
        let mut selected = Vec::new();
        let mut selected_roots = Vec::new();
        for (index, (candidate, root)) in candidates.into_iter().zip(roots).enumerate() {
            if let Some((_, cause)) = selection.next_if(|(chosen, _)| *chosen == index) {
                if let Some(cue_kind) = cause {
                    floor_admissions.push(CueFloorAdmission {
                        object: MemoryObjectRef::new(candidate.object_type, candidate.object_id),
                        stage: CueFloorStage::CandidateMerge,
                        cue_kind,
                    });
                }
                selected.push(candidate);
                selected_roots.push(root);
            }
        }
        Ok(RecallCues {
            candidates: CanonicalCandidates::new(selected),
            roots: selected_roots,
            orders,
            participants,
            references,
            dimension,
            completeness,
            floor_admissions,
            scene_cue_searches,
        })
    }

    pub(super) async fn memory_scenes(
        &self,
        pack: &ContinuityContextPack,
        mut admitted_by: HashMap<MemoryObjectRef, BTreeSet<AdmissionRoad>>,
        include_suppressed: bool,
        reference_time: DateTime<Utc>,
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
                let seconds_since_support = if memory.object_type == ObjectType::DerivedMemory {
                    sources
                        .iter()
                        .filter_map(|source| support_time(*source, &objects, include_suppressed))
                        .filter(|time| *time <= reference_time)
                        .max()
                        .map(|time| (reference_time - time).num_seconds())
                } else {
                    None
                };
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
                MemoryScenes {
                    memory,
                    admitted_by: admitted_by
                        .remove(&memory)
                        .expect("every packed memory has its admission roads"),
                    sources,
                    seconds_since_support,
                }
            })
            .collect())
    }
}

fn support_time(
    source: MemoryObjectRef,
    objects: &HashMap<MemoryObjectRef, MemoryObject>,
    include_suppressed: bool,
) -> Option<DateTime<Utc>> {
    match objects.get(&source)? {
        MemoryObject::Episode(episode)
            if include_suppressed || episode.retention_state != RetentionState::Suppressed =>
        {
            Some(episode.scene.time.to_utc())
        }
        MemoryObject::Observation(observation)
            if include_suppressed || observation.retention_state != RetentionState::Suppressed =>
        {
            observation.observed_at.or_else(|| {
                match objects.get(&MemoryObjectRef::new(
                    ObjectType::Episode,
                    observation.episode_id,
                ))? {
                    MemoryObject::Episode(episode) => Some(episode.scene.time.to_utc()),
                    _ => None,
                }
            })
        }
        _ => None,
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
