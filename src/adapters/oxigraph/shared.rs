use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, MutexGuard};

use chrono::{DateTime, FixedOffset, Utc};
use oxigraph::model::{GraphName, GraphNameRef, Literal, NamedNode, NamedOrBlankNode, Quad, Term};
use oxigraph::store::Store;
use serde::de::DeserializeOwned;

use crate::domain::{
    graph_uri, DerivedMemory, Entity, Episode, MemoryId, MemoryLink, MemoryObject, MemoryObjectRef,
    MemoryThread, ObjectType, Observation, RelationType,
};
use crate::errors::CustomError;
use crate::policy::graph_expansion::{
    bounded_incident_link_refs, is_participant_pair, order_current_subject_links,
    BoundedExpansionLinkRef, ParticipantOccasions, SUBJECT_ABOUTNESS_ROUTES,
};
use crate::ports::graph_authority::{
    GraphExpansion, GraphExpansionBoundedFailure, GraphExpansionBoundedFailureReason,
    GraphExpansionFanoutUtilization, GraphExpansionFilteredNode, GraphExpansionFilteredReason,
    GraphExpansionQuery,
};

use super::rdf_mapping::{RdfObject, RdfTriple};
use super::sparql_selectors::{SparqlGraphSelectors, SparqlLinkRef};
impl BoundedExpansionLinkRef for SparqlLinkRef {
    fn link_id(self) -> MemoryId {
        self.link_id
    }

    fn from(self) -> MemoryObjectRef {
        self.from
    }

    fn to(self) -> MemoryObjectRef {
        self.to
    }

    fn relation(self) -> RelationType {
        self.relation
    }
}

pub(super) fn link_refs_by_endpoint<T: BoundedExpansionLinkRef>(
    link_refs: &[T],
) -> HashMap<MemoryObjectRef, Vec<T>> {
    let mut refs_by_endpoint = HashMap::<MemoryObjectRef, Vec<T>>::new();
    for link_ref in link_refs.iter().copied() {
        refs_by_endpoint
            .entry(link_ref.from())
            .or_default()
            .push(link_ref);
        if link_ref.to() != link_ref.from() {
            refs_by_endpoint
                .entry(link_ref.to())
                .or_default()
                .push(link_ref);
        }
    }
    refs_by_endpoint
}

pub(super) fn insert_visible_ref(
    query: &GraphExpansionQuery,
    graph_refs: &mut HashSet<MemoryObjectRef>,
    next_frontier: &mut Vec<MemoryObjectRef>,
    object_ref: MemoryObjectRef,
    bounded_failure: &mut Option<GraphExpansionBoundedFailure>,
) {
    if graph_refs.contains(&object_ref) {
        return;
    }
    if graph_refs.len() >= query.max_nodes {
        let failure = GraphExpansionBoundedFailure {
            reason: GraphExpansionBoundedFailureReason::NodeLimit,
            at: Some(object_ref),
        };
        bounded_failure.get_or_insert(failure);
        return;
    }
    graph_refs.insert(object_ref);
    next_frontier.push(object_ref);
}

pub(super) fn quads_for_triples(
    owner_graph_uri: &str,
    triples: &[RdfTriple],
) -> Result<Vec<Quad>, CustomError> {
    triples
        .iter()
        .map(|triple| quad_for_triple(owner_graph_uri, triple))
        .collect()
}

#[derive(Default)]
pub(super) struct RdfSubjectValues {
    literals: HashMap<String, Vec<String>>,
    resources: HashMap<String, Vec<String>>,
}

impl RdfSubjectValues {
    fn push_literal(&mut self, predicate: String, value: String) {
        self.literals.entry(predicate).or_default().push(value);
    }

    fn push_resource(&mut self, predicate: String, value: String) {
        self.resources.entry(predicate).or_default().push(value);
    }

    fn literal(&self, subject: &str, predicate: &'static str) -> Result<String, CustomError> {
        self.literals
            .get(predicate)
            .and_then(|values| values.first())
            .cloned()
            .ok_or_else(|| missing_rdf_value(subject, predicate))
    }

    fn optional_literal(&self, predicate: &'static str) -> Option<String> {
        self.literals
            .get(predicate)
            .and_then(|values| values.first())
            .cloned()
    }

    fn resource(&self, subject: &str, predicate: &'static str) -> Result<String, CustomError> {
        self.resources
            .get(predicate)
            .and_then(|values| values.first())
            .cloned()
            .ok_or_else(|| missing_rdf_value(subject, predicate))
    }

    fn resource_values(&self, predicate: &'static str) -> Vec<String> {
        self.resources.get(predicate).cloned().unwrap_or_default()
    }
}

pub(super) fn hydrate_objects_by_refs_from_store(
    store: &Store,
    refs: &[MemoryObjectRef],
) -> Result<Vec<MemoryObject>, CustomError> {
    let graphs = refs
        .iter()
        .filter(|object| object.object_type != ObjectType::MemoryLink)
        .map(|object| NamedNode::new(graph_uri(object.object_type, object.id)))
        .collect::<Result<Vec<_>, _>>()?;
    // Include assertion subjects owned by each graph, not just its root subject.
    let subjects = rdf_subject_values_from_quads(graphs.iter().flat_map(|graph| {
        store.quads_for_pattern(
            None,
            None,
            None,
            Some(GraphNameRef::NamedNode(graph.as_ref())),
        )
    }))?;
    let mut objects = Vec::new();
    for object_ref in refs {
        if object_ref.object_type == ObjectType::MemoryLink {
            continue;
        }
        let subject = graph_uri(object_ref.object_type, object_ref.id);
        if let Some(values) = subjects.get(&subject) {
            objects.push(memory_object_from_rdf(
                &subject,
                values,
                object_ref.object_type,
                &subjects,
            )?);
        }
    }
    objects.sort_by_key(MemoryObject::stable_order_key);
    Ok(objects)
}

pub(super) fn hydrate_all_links_from_store(store: &Store) -> Result<Vec<MemoryLink>, CustomError> {
    let subjects = rdf_subject_values(store)?;
    let mut links = subjects
        .iter()
        .filter_map(|(subject, values)| {
            let object_type = values
                .optional_literal(super::vocabulary::OBJECT_TYPE)
                .and_then(|value| enum_value_from_literal::<ObjectType>(&value).ok());
            match object_type {
                Some(ObjectType::MemoryLink) => Some(memory_link_from_rdf(subject, values)),
                _ => None,
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    links.sort_by_key(|link| link.id);
    Ok(links)
}

pub(super) fn hydrate_links_by_ids_from_store(
    store: &Store,
    link_ids: &[MemoryId],
) -> Result<Vec<MemoryLink>, CustomError> {
    let mut ids = link_ids.to_vec();
    ids.sort_unstable();
    ids.dedup();

    let mut links = Vec::new();
    for link_id in ids {
        let subject = graph_uri(ObjectType::MemoryLink, link_id);
        if let Some(values) = rdf_subject_values_for_named_graph(store, &subject)? {
            links.push(memory_link_from_rdf(&subject, &values)?);
        }
    }
    Ok(links)
}

pub(super) fn hydrate_links_by_id_sets_from_store(
    store: &Store,
    graph_link_ids: &HashSet<MemoryId>,
    lifecycle_link_ids: &HashSet<MemoryId>,
    graph_ref_set: &HashSet<MemoryObjectRef>,
) -> Result<Vec<MemoryLink>, CustomError> {
    let links = hydrate_all_links_from_store(store)?;
    Ok(links
        .into_iter()
        .filter(|link| graph_link_ids.contains(&link.id) || lifecycle_link_ids.contains(&link.id))
        .filter(|link| {
            let endpoints_in_graph = graph_ref_set
                .contains(&MemoryObjectRef::from_id_type(link.from_id, link.from_type))
                && graph_ref_set.contains(&MemoryObjectRef::from_id_type(link.to_id, link.to_type));
            (graph_link_ids.contains(&link.id) && endpoints_in_graph)
                || lifecycle_link_ids.contains(&link.id)
        })
        .collect())
}

pub(super) fn rdf_subject_values(
    store: &Store,
) -> Result<HashMap<String, RdfSubjectValues>, CustomError> {
    rdf_subject_values_from_quads(store.iter())
}

fn rdf_subject_values_from_quads(
    quads: impl IntoIterator<Item = Result<Quad, oxigraph::store::StorageError>>,
) -> Result<HashMap<String, RdfSubjectValues>, CustomError> {
    let mut subjects = HashMap::<String, RdfSubjectValues>::new();
    for quad in quads {
        #[cfg(test)]
        RDF_QUADS_READ.with(|count| count.set(count.get() + 1));
        let quad = quad.map_err(oxigraph_error)?;
        if !matches!(quad.graph_name, GraphName::NamedNode(_)) {
            continue;
        }
        let NamedOrBlankNode::NamedNode(subject) = quad.subject else {
            continue;
        };
        let values = subjects.entry(subject.as_str().to_owned()).or_default();
        match quad.object {
            Term::NamedNode(value) => values.push_resource(
                quad.predicate.as_str().to_owned(),
                value.as_str().to_owned(),
            ),
            Term::Literal(value) => {
                values.push_literal(quad.predicate.as_str().to_owned(), value.value().to_owned())
            }
            Term::BlankNode(_) => {}
        }
    }
    Ok(subjects)
}

#[cfg(test)]
thread_local! {
    pub(super) static RDF_QUADS_READ: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn rdf_subject_values_for_named_graph(
    store: &Store,
    graph_uri: &str,
) -> Result<Option<RdfSubjectValues>, CustomError> {
    let graph_name = NamedNode::new(graph_uri)?;
    let mut values = RdfSubjectValues::default();
    let mut found = false;
    for quad in store.quads_for_pattern(
        None,
        None,
        None,
        Some(GraphNameRef::NamedNode(graph_name.as_ref())),
    ) {
        #[cfg(test)]
        RDF_QUADS_READ.with(|count| count.set(count.get() + 1));
        let quad = quad.map_err(oxigraph_error)?;
        let NamedOrBlankNode::NamedNode(subject) = quad.subject else {
            continue;
        };
        if subject != graph_name {
            continue;
        }
        found = true;
        match quad.object {
            Term::NamedNode(value) => values.push_resource(
                quad.predicate.as_str().to_owned(),
                value.as_str().to_owned(),
            ),
            Term::Literal(value) => {
                values.push_literal(quad.predicate.as_str().to_owned(), value.value().to_owned())
            }
            Term::BlankNode(_) => {}
        }
    }
    Ok(found.then_some(values))
}

pub(super) fn memory_object_from_rdf(
    subject: &str,
    values: &RdfSubjectValues,
    object_type: ObjectType,
    subjects: &HashMap<String, RdfSubjectValues>,
) -> Result<MemoryObject, CustomError> {
    match object_type {
        ObjectType::Episode => Ok(MemoryObject::Episode(Episode {
            id: memory_id_literal(subject, values, super::vocabulary::OBJECT_ID)?,
            object_type,
            modality: enum_literal(subject, values, super::vocabulary::MODALITY)?,
            scene: crate::domain::Scene {
                time: timestamp_literal(subject, values, super::vocabulary::SCENE_TIME)?
                    .with_timezone(&scene_offset_literal(subject, values)?),
                participants: serde_json::from_str(
                    &values.literal(subject, super::vocabulary::SCENE_PARTICIPANTS)?,
                )
                .map_err(|error| {
                    rdf_parse_error(subject, super::vocabulary::SCENE_PARTICIPANTS, error)
                })?,
                setting: crate::domain::SceneSetting {
                    key: values.optional_literal(super::vocabulary::SETTING_KEY),
                    words: values.optional_literal(super::vocabulary::SETTING_WORDS),
                },
                custom_values: serde_json::from_str(
                    &values.literal(subject, super::vocabulary::SCENE_CUSTOM_VALUES)?,
                )
                .map_err(|error| {
                    rdf_parse_error(subject, super::vocabulary::SCENE_CUSTOM_VALUES, error)
                })?,
            },
            ended_at: optional_timestamp_literal(values, super::vocabulary::ENDED_AT)?,
            summary: values.literal(subject, super::vocabulary::SUMMARY)?,
            raw_ref: values.optional_literal(super::vocabulary::RAW_REF),
            salience_score: f32_literal(subject, values, super::vocabulary::SALIENCE_SCORE)?,
            retention_state: enum_literal(subject, values, super::vocabulary::RETENTION_STATE)?,
            created_at: timestamp_literal(subject, values, super::vocabulary::CREATED_AT)?,
            schema_version: values.literal(subject, super::vocabulary::SCHEMA_VERSION)?,
        })),
        ObjectType::Observation => Ok(MemoryObject::Observation(Observation {
            id: memory_id_literal(subject, values, super::vocabulary::OBJECT_ID)?,
            object_type,
            episode_id: memory_id_from_resource(
                &values.resource(subject, super::vocabulary::EPISODE)?,
            )?,
            speaker_entity_id: values
                .resource_values(super::vocabulary::SPEAKER_ENTITY)
                .first()
                .map(|value| memory_id_from_resource(value))
                .transpose()?,
            observed_at: optional_timestamp_literal(values, super::vocabulary::OBSERVED_AT)?,
            modality: enum_literal(subject, values, super::vocabulary::MODALITY)?,
            text: values.literal(subject, super::vocabulary::TEXT)?,
            raw_ref: values.optional_literal(super::vocabulary::RAW_REF),
            salience_score: f32_literal(subject, values, super::vocabulary::SALIENCE_SCORE)?,
            retention_state: enum_literal(subject, values, super::vocabulary::RETENTION_STATE)?,
            created_at: timestamp_literal(subject, values, super::vocabulary::CREATED_AT)?,
            schema_version: values.literal(subject, super::vocabulary::SCHEMA_VERSION)?,
        })),
        ObjectType::Entity => Ok(MemoryObject::Entity(Entity {
            id: memory_id_literal(subject, values, super::vocabulary::OBJECT_ID)?,
            object_type,
            created_at: timestamp_literal(subject, values, super::vocabulary::CREATED_AT)?,
            schema_version: values.literal(subject, super::vocabulary::SCHEMA_VERSION)?,
        })),
        ObjectType::MemoryThread => Ok(MemoryObject::MemoryThread(MemoryThread {
            id: memory_id_literal(subject, values, super::vocabulary::OBJECT_ID)?,
            object_type,
            title: values.literal(subject, super::vocabulary::TITLE)?,
            summary: values.literal(subject, super::vocabulary::SUMMARY)?,
            status: enum_literal(subject, values, super::vocabulary::THREAD_STATUS)?,
            last_touched_at: timestamp_literal(
                subject,
                values,
                super::vocabulary::LAST_TOUCHED_AT,
            )?,
            salience_score: f32_literal(subject, values, super::vocabulary::SALIENCE_SCORE)?,
            canonical_key: values.optional_literal(super::vocabulary::CANONICAL_KEY),
            created_at: timestamp_literal(subject, values, super::vocabulary::CREATED_AT)?,
            updated_at: timestamp_literal(subject, values, super::vocabulary::UPDATED_AT)?,
            schema_version: values.literal(subject, super::vocabulary::SCHEMA_VERSION)?,
        })),
        ObjectType::DerivedMemory => Ok(MemoryObject::DerivedMemory(DerivedMemory {
            scope_keys: values
                .literals
                .get(super::vocabulary::SCOPE_KEY)
                .into_iter()
                .flatten()
                .map(|value| {
                    serde_json::from_str(value).map_err(|error| {
                        rdf_parse_error(subject, super::vocabulary::SCOPE_KEY, error)
                    })
                })
                .collect::<Result<std::collections::BTreeSet<_>, _>>()?
                .into_iter()
                .collect(),
            id: memory_id_literal(subject, values, super::vocabulary::OBJECT_ID)?,
            object_type,
            derived_type: enum_literal(subject, values, super::vocabulary::DERIVED_TYPE)?,
            assertions: belief_assertions_from_rdf(values, subjects)?,
            given_by_application: values
                .literal(subject, super::vocabulary::GIVEN_BY_APPLICATION)?
                .parse::<bool>()
                .map_err(|error| {
                    rdf_parse_error(subject, super::vocabulary::GIVEN_BY_APPLICATION, error)
                })?,
            text: values.literal(subject, super::vocabulary::TEXT)?,
            derived_from_episode_ids: memory_ids_from_resources(
                values.resource_values(super::vocabulary::DERIVED_FROM_EPISODE),
            )?,
            derived_from_observation_ids: memory_ids_from_resources(
                values.resource_values(super::vocabulary::DERIVED_FROM_OBSERVATION),
            )?,
            thread_ids: memory_ids_from_resources(
                values.resource_values(super::vocabulary::PART_OF_THREAD),
            )?,
            entity_ids: memory_ids_from_resources(
                values.resource_values(super::vocabulary::ABOUT_ENTITY),
            )?,
            salience_score: f32_literal(subject, values, super::vocabulary::SALIENCE_SCORE)?,
            supersedes: memory_ids_from_resources(
                values.resource_values(super::vocabulary::SUPERSEDES),
            )?,
            retention_state: enum_literal(subject, values, super::vocabulary::RETENTION_STATE)?,
            created_at: timestamp_literal(subject, values, super::vocabulary::CREATED_AT)?,
            updated_at: timestamp_literal(subject, values, super::vocabulary::UPDATED_AT)?,
            schema_version: values.literal(subject, super::vocabulary::SCHEMA_VERSION)?,
        })),
        ObjectType::MemoryLink => Ok(MemoryObject::MemoryLink(memory_link_from_rdf(
            subject, values,
        )?)),
    }
}

fn belief_assertions_from_rdf(
    values: &RdfSubjectValues,
    subjects: &HashMap<String, RdfSubjectValues>,
) -> Result<Vec<crate::domain::BeliefAssertion>, CustomError> {
    use super::vocabulary as vocab;
    let mut nodes = values.resource_values(vocab::ASSERTION);
    nodes.sort();
    nodes
        .into_iter()
        .map(|node| {
            let values = subjects
                .get(&node)
                .ok_or_else(|| missing_rdf_value(&node, vocab::ASSERTION))?;
            let predicate = serde_json::from_value(serde_json::json!({
                "predicate": values.literal(&node, vocab::ASSERTION_PREDICATE)?,
                "name": values.literal(&node, vocab::ASSERTION_NAME)?,
            }))
            .map_err(|error| rdf_parse_error(&node, vocab::ASSERTION_PREDICATE, error))?;
            Ok(crate::domain::BeliefAssertion {
                subject: memory_id_from_resource(
                    &values.resource(&node, vocab::ASSERTION_SUBJECT)?,
                )?,
                predicate,
            })
        })
        .collect()
}

pub(super) fn memory_link_from_rdf(
    subject: &str,
    values: &RdfSubjectValues,
) -> Result<MemoryLink, CustomError> {
    Ok(MemoryLink {
        id: memory_id_literal(subject, values, super::vocabulary::OBJECT_ID)?,
        object_type: ObjectType::MemoryLink,
        from_id: memory_id_from_resource(&values.resource(subject, super::vocabulary::FROM)?)?,
        from_type: enum_literal(subject, values, super::vocabulary::FROM_TYPE)?,
        to_id: memory_id_from_resource(&values.resource(subject, super::vocabulary::TO)?)?,
        to_type: enum_literal(subject, values, super::vocabulary::TO_TYPE)?,
        relation: enum_literal(subject, values, super::vocabulary::RELATION)?,
        rationale: values.optional_literal(super::vocabulary::RATIONALE),
        created_at: timestamp_literal(subject, values, super::vocabulary::CREATED_AT)?,
        schema_version: values.literal(subject, super::vocabulary::SCHEMA_VERSION)?,
    })
}

pub(super) fn memory_id_literal(
    subject: &str,
    values: &RdfSubjectValues,
    predicate: &'static str,
) -> Result<MemoryId, CustomError> {
    values
        .literal(subject, predicate)?
        .parse()
        .map_err(|error| rdf_parse_error(subject, predicate, error))
}

pub(super) fn memory_id_from_resource(value: &str) -> Result<MemoryId, CustomError> {
    value
        .rsplit(':')
        .next()
        .ok_or_else(|| CustomError::DatabaseError(format!("Invalid graph URI resource: {value}")))?
        .parse()
        .map_err(|error| CustomError::DatabaseError(format!("Invalid graph URI MemoryId: {error}")))
}

pub(super) fn memory_ids_from_resources(values: Vec<String>) -> Result<Vec<MemoryId>, CustomError> {
    let mut ids = values
        .iter()
        .map(|value| memory_id_from_resource(value))
        .collect::<Result<Vec<_>, _>>()?;
    ids.sort();
    Ok(ids)
}

pub(super) fn enum_literal<T: DeserializeOwned>(
    subject: &str,
    values: &RdfSubjectValues,
    predicate: &'static str,
) -> Result<T, CustomError> {
    enum_value_from_literal(&values.literal(subject, predicate)?)
        .map_err(|error| rdf_parse_error(subject, predicate, error))
}

pub(super) fn enum_value_from_literal<T: DeserializeOwned>(
    value: &str,
) -> Result<T, serde_json::Error> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
}

pub(super) fn f32_literal(
    subject: &str,
    values: &RdfSubjectValues,
    predicate: &'static str,
) -> Result<f32, CustomError> {
    values
        .literal(subject, predicate)?
        .parse()
        .map_err(|error| rdf_parse_error(subject, predicate, error))
}

fn scene_offset_literal(
    subject: &str,
    values: &RdfSubjectValues,
) -> Result<FixedOffset, CustomError> {
    let predicate = super::vocabulary::SCENE_OFFSET_SECONDS;
    let seconds = values
        .literal(subject, predicate)?
        .parse()
        .map_err(|error| rdf_parse_error(subject, predicate, error))?;
    FixedOffset::east_opt(seconds)
        .ok_or_else(|| rdf_parse_error(subject, predicate, "offset must be less than 24 hours"))
}

pub(super) fn timestamp_literal(
    subject: &str,
    values: &RdfSubjectValues,
    predicate: &'static str,
) -> Result<DateTime<Utc>, CustomError> {
    parse_timestamp(subject, predicate, &values.literal(subject, predicate)?)
}

pub(super) fn optional_timestamp_literal(
    values: &RdfSubjectValues,
    predicate: &'static str,
) -> Result<Option<DateTime<Utc>>, CustomError> {
    values
        .optional_literal(predicate)
        .map(|value| parse_timestamp("<optional>", predicate, &value))
        .transpose()
}

pub(super) fn parse_timestamp(
    subject: &str,
    predicate: &'static str,
    value: &str,
) -> Result<DateTime<Utc>, CustomError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| rdf_parse_error(subject, predicate, error))
}

pub(super) fn missing_rdf_value(subject: &str, predicate: &'static str) -> CustomError {
    CustomError::DatabaseError(format!(
        "Oxigraph RDF object {subject} is missing required predicate {predicate}"
    ))
}

pub(super) fn rdf_parse_error(
    subject: &str,
    predicate: &'static str,
    error: impl std::fmt::Display,
) -> CustomError {
    CustomError::DatabaseError(format!(
        "Oxigraph RDF object {subject} has invalid predicate {predicate}: {error}"
    ))
}

#[derive(Debug, Default)]
pub(super) struct BoundedGraphVisibility {
    pub(super) participant_occasions: ParticipantOccasions,
    pub(super) object_refs: HashSet<MemoryObjectRef>,
    pub(super) traversal_link_ids: HashSet<MemoryId>,
    pub(super) lifecycle_link_ids: HashSet<MemoryId>,
    pub(super) fanout_utilization: Vec<GraphExpansionFanoutUtilization>,
    pub(super) filtered_nodes: Vec<GraphExpansionFilteredNode>,
    pub(super) bounded_failure: Option<GraphExpansionBoundedFailure>,
}

pub(super) fn assign_expanded_fanout_utilization(
    expansion: &mut GraphExpansion,
    fanout_utilization: Vec<GraphExpansionFanoutUtilization>,
) {
    expansion.fanout_utilization = fanout_utilization
        .into_iter()
        .filter(|entry| expansion.expanded_nodes.contains(&entry.root))
        .collect();
}

pub(super) fn bounded_graph_visible_refs(
    selectors: &SparqlGraphSelectors<'_>,
    root_ref: MemoryObjectRef,
    query: &GraphExpansionQuery,
) -> Result<BoundedGraphVisibility, CustomError> {
    let mut graph_refs = HashSet::from([root_ref]);
    let mut graph_link_ids = HashSet::new();
    let mut fanout_utilization = Vec::new();
    let mut filtered_nodes = Vec::new();
    let mut bounded_failure = None;
    let mut frontier = vec![root_ref];
    let (state_refs, state_filtered) = if query.current_subject_state {
        selectors.select_subject_state(query)?
    } else {
        (Vec::new(), Vec::new())
    };
    let state_ranks = state_refs
        .into_iter()
        .enumerate()
        .map(|(rank, object)| (object, rank))
        .collect();
    let mut participant_occasions = if matches!(
        root_ref.object_type,
        ObjectType::Episode | ObjectType::Observation
    ) {
        selectors.select_participant_occasions(&[root_ref])?
    } else {
        ParticipantOccasions::new()
    };

    for depth in 0..query.max_depth {
        frontier.retain(|object| query.may_continue_from(object.object_type));
        if frontier.is_empty() {
            break;
        }
        let mut link_refs =
            selectors.select_links_touching(&frontier, depth == 0 && query.current_thread_state)?;
        if depth == 0
            && query
                .fanout_overrides
                .iter()
                .any(|entry| is_participant_pair(entry.relation, entry.object_type))
        {
            let mut seen = HashSet::new();
            let neighbors = link_refs
                .iter()
                .filter_map(|link| {
                    let neighbor = link.other_endpoint(root_ref);
                    (is_participant_pair(link.relation, neighbor.object_type)
                        && query.allows_object(neighbor)
                        && (query.allowed_relation_types.is_empty()
                            || query.allowed_relation_types.contains(&link.relation))
                        && crate::policy::graph_expansion::fanout_limit_for_pair(
                            query,
                            link.relation,
                            neighbor.object_type,
                        ) > 0
                        && seen.insert(neighbor))
                    .then_some(neighbor)
                })
                .collect::<Vec<_>>();
            participant_occasions =
                selectors.select_bounded_participant_occasions(&neighbors, query)?;
            link_refs.retain(|link| {
                let neighbor = link.other_endpoint(root_ref);
                !is_participant_pair(link.relation, neighbor.object_type)
                    || participant_occasions.contains_key(&neighbor)
            });
        }
        // Every road is as-of the scene, including an observation's parent hop.
        if query.participant_reference_time != DateTime::<Utc>::MAX_UTC
            || query.current_subject_state
        {
            let memories = link_refs
                .iter()
                .flat_map(|link| [link.from, link.to])
                .chain(frontier.iter().copied())
                .filter(|object| {
                    matches!(
                        object.object_type,
                        ObjectType::Episode | ObjectType::Observation
                    )
                })
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let occasions = selectors.select_participant_occasions(&memories)?;
            let future = occasions
                .iter()
                .filter(|(object, occasion)| {
                    occasion.memory_time(object.object_type) > query.participant_reference_time
                        && !(query.allow_future_root && **object == root_ref)
                })
                .map(|(object, _)| *object)
                .collect::<HashSet<_>>();
            participant_occasions.extend(occasions);
            for link in &link_refs {
                if !query.allowed_relation_types.is_empty()
                    && !query.allowed_relation_types.contains(&link.relation)
                {
                    continue;
                }
                for (from, to) in [(link.from, link.to), (link.to, link.from)] {
                    if frontier.contains(&from)
                        && !future.contains(&from)
                        && future.contains(&to)
                        && query.allows_incident_link(from, link.relation, to)
                    {
                        filtered_nodes.push(GraphExpansionFilteredNode {
                            object_ref: to,
                            reason: GraphExpansionFilteredReason::LaterThanReferenceTime,
                            superseded_by: Vec::new(),
                        });
                    }
                }
            }
            frontier.retain(|object| !future.contains(object));
            link_refs.retain(|link| !future.contains(&link.from) && !future.contains(&link.to));
        }
        let link_refs_by_endpoint = link_refs_by_endpoint(&link_refs);
        let mut next_frontier = Vec::new();
        for object_ref in &frontier {
            let incident_link_refs = link_refs_by_endpoint
                .get(object_ref)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let ordered;
            let incident_link_refs = if depth == 0 && query.current_subject_state {
                let about_refs = incident_link_refs
                    .iter()
                    .filter_map(|link| {
                        let neighbor = link.other_endpoint(*object_ref);
                        (SUBJECT_ABOUTNESS_ROUTES.contains(&(link.relation, neighbor.object_type))
                            && query.allows_object(neighbor)
                            && (query.allowed_relation_types.is_empty()
                                || query.allowed_relation_types.contains(&link.relation)))
                        .then_some(neighbor)
                    })
                    .collect::<HashSet<_>>();
                filtered_nodes.extend(
                    state_filtered
                        .iter()
                        .filter(|entry| about_refs.contains(&entry.object_ref))
                        .cloned(),
                );
                let cap = crate::policy::graph_expansion::fanout_limit_for_pair(
                    query,
                    RelationType::About,
                    ObjectType::DerivedMemory,
                );
                ordered = order_current_subject_links(
                    incident_link_refs.to_vec(),
                    &state_ranks,
                    cap.saturating_add(usize::from(query.trace_mode.is_enabled())),
                    |link| {
                        let neighbor = link.other_endpoint(*object_ref);
                        (link.relation, neighbor)
                    },
                );
                &ordered
            } else {
                incident_link_refs
            };
            let selection = bounded_incident_link_refs(
                query,
                root_ref,
                *object_ref,
                depth,
                incident_link_refs,
                &participant_occasions,
                &mut bounded_failure,
            );
            fanout_utilization.extend(selection.utilization);
            filtered_nodes.extend(selection.filtered_nodes);
            for link_ref in selection.links {
                let neighbor = link_ref.other_endpoint(*object_ref);
                insert_visible_ref(
                    query,
                    &mut graph_refs,
                    &mut next_frontier,
                    neighbor,
                    &mut bounded_failure,
                );
                if graph_refs.contains(&neighbor) {
                    graph_link_ids.insert(link_ref.link_id());
                }
            }
        }

        if next_frontier.is_empty() {
            break;
        }
        frontier = next_frontier;
    }

    let candidate_refs = graph_refs.iter().copied().collect::<Vec<_>>();
    let lifecycle_link_ids = selectors
        .select_links_touching(&candidate_refs, false)?
        .into_iter()
        .filter(|link_ref| {
            matches!(
                link_ref.relation,
                RelationType::Supersedes
                    | RelationType::Resolves
                    | RelationType::FulfillsCommitment
            ) && link_ref.to.object_type == ObjectType::DerivedMemory
                && graph_refs.contains(&link_ref.to)
        })
        .map(|link_ref| link_ref.link_id)
        .collect::<HashSet<_>>();

    Ok(BoundedGraphVisibility {
        participant_occasions,
        object_refs: graph_refs,
        traversal_link_ids: graph_link_ids,
        lifecycle_link_ids,
        fanout_utilization,
        filtered_nodes,
        bounded_failure,
    })
}

pub(super) fn quad_for_triple(
    owner_graph_uri: &str,
    triple: &RdfTriple,
) -> Result<Quad, CustomError> {
    let subject = NamedNode::new(triple.subject.as_str())?;
    let predicate = NamedNode::new(triple.predicate.as_str())?;
    let graph_name = NamedNode::new(owner_graph_uri)?;
    let object = match &triple.object {
        RdfObject::Resource(value) => Term::NamedNode(NamedNode::new(value.as_str())?),
        RdfObject::Literal(value) => Term::Literal(Literal::new_simple_literal(value.as_str())),
    };

    Ok(Quad::new(
        NamedOrBlankNode::NamedNode(subject),
        predicate,
        object,
        GraphName::NamedNode(graph_name),
    ))
}

pub(super) fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, CustomError> {
    mutex.lock().map_err(|error| {
        CustomError::DatabaseError(format!("Oxigraph graph store lock poisoned: {error}"))
    })
}

pub(super) fn oxigraph_error(error: impl std::fmt::Display) -> CustomError {
    CustomError::DatabaseError(format!("Oxigraph graph store error: {error}"))
}

impl From<oxigraph::model::IriParseError> for CustomError {
    fn from(error: oxigraph::model::IriParseError) -> Self {
        CustomError::DatabaseError(format!("Invalid RDF IRI: {error}"))
    }
}
