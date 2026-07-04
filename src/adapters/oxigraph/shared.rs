use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, MutexGuard};

use chrono::{DateTime, Utc};
use oxigraph::model::{GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term};
use oxigraph::store::Store;
use serde::{de::DeserializeOwned, Deserialize};

use crate::api::types::{
    graph_uri, DerivedMemory, Entity, Episode, MemoryId, MemoryLink, MemoryObject, MemoryThread,
    ObjectType, Observation, RelationType,
};
use crate::errors::CustomError;
use crate::policy::graph_expansion::{
    bounded_incident_link_refs, graph_expansion_bounded_error, BoundedExpansionLinkRef,
};
use crate::ports::graph_authority::{
    GraphExpansionBoundedFailure, GraphExpansionBoundedFailureReason, GraphExpansionQuery,
    GraphObjectQuery, GraphObjectRef,
};

use super::rdf_mapping::{RdfObject, RdfTriple};
use super::sparql_selectors::{SparqlGraphSelectors, SparqlLinkRef};
use super::vocabulary as vocab;
impl BoundedExpansionLinkRef for SparqlLinkRef {
    fn link_id(self) -> MemoryId {
        self.link_id
    }

    fn from(self) -> GraphObjectRef {
        self.from
    }

    fn to(self) -> GraphObjectRef {
        self.to
    }

    fn relation(self) -> RelationType {
        self.relation
    }
}

impl BoundedExpansionLinkRef for SparqlHttpLinkRef {
    fn link_id(self) -> MemoryId {
        self.link_id
    }

    fn from(self) -> GraphObjectRef {
        self.from
    }

    fn to(self) -> GraphObjectRef {
        self.to
    }

    fn relation(self) -> RelationType {
        self.relation
    }
}

pub(super) fn link_refs_by_endpoint<T: BoundedExpansionLinkRef>(
    link_refs: &[T],
) -> HashMap<GraphObjectRef, Vec<T>> {
    let mut refs_by_endpoint = HashMap::<GraphObjectRef, Vec<T>>::new();
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
    graph_refs: &mut HashSet<GraphObjectRef>,
    next_frontier: &mut Vec<GraphObjectRef>,
    object_ref: GraphObjectRef,
    bounded_failure: &mut Option<GraphExpansionBoundedFailure>,
) -> Result<(), CustomError> {
    if graph_refs.contains(&object_ref) {
        return Ok(());
    }
    if graph_refs.len() >= query.max_nodes {
        let failure = GraphExpansionBoundedFailure {
            reason: GraphExpansionBoundedFailureReason::NodeLimit,
            at: Some(object_ref),
        };
        if !query.failure_policy.allow_partial_results {
            return Err(graph_expansion_bounded_error(failure));
        }
        bounded_failure.get_or_insert(failure);
        return Ok(());
    }
    graph_refs.insert(object_ref);
    next_frontier.push(object_ref);
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct SparqlHttpLinkRef {
    pub(super) link_id: MemoryId,
    pub(super) from: GraphObjectRef,
    pub(super) to: GraphObjectRef,
    pub(super) relation: RelationType,
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

pub(super) fn sparql_quad(quad: &Quad) -> Result<String, CustomError> {
    let GraphName::NamedNode(graph_name) = &quad.graph_name else {
        return Err(CustomError::DatabaseError(
            "Oxigraph service writes require named graphs".to_owned(),
        ));
    };
    let NamedOrBlankNode::NamedNode(subject) = &quad.subject else {
        return Err(CustomError::DatabaseError(
            "Oxigraph service writes require named-node subjects".to_owned(),
        ));
    };

    Ok(format!(
        "GRAPH <{}> {{ <{}> <{}> {} . }}",
        graph_name.as_str(),
        subject.as_str(),
        quad.predicate.as_str(),
        sparql_term(&quad.object)?
    ))
}

pub(super) fn sparql_term(term: &Term) -> Result<String, CustomError> {
    match term {
        Term::NamedNode(value) => Ok(format!("<{}>", value.as_str())),
        Term::Literal(value) => Ok(format!("\"{}\"", sparql_escape_literal(value.value()))),
        Term::BlankNode(_) => Err(CustomError::DatabaseError(
            "Oxigraph service writes do not support blank-node objects".to_owned(),
        )),
    }
}

pub(super) fn sparql_escape_literal(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(character),
        }
    }
    escaped
}

pub(super) fn named_graph_quads_query(graph_uris: &[String]) -> String {
    let graph_values = sparql_iri_values("g", graph_uris.iter().map(String::as_str));
    format!(
        r#"
        SELECT ?g ?s ?p ?o WHERE {{
          {graph_values}
          GRAPH ?g {{ ?s ?p ?o }}
        }}
        "#
    )
}

pub(super) fn object_refs_query(query: &GraphObjectQuery) -> String {
    let id_values = sparql_literal_values("id", query.object_ids.iter().map(|id| id.to_string()));
    let type_values = sparql_literal_values(
        "objectType",
        query
            .object_types
            .iter()
            .map(|object_type| enum_value(*object_type)),
    );
    let ref_values = sparql_object_ref_values(&query.object_refs);

    format!(
        r#"
        SELECT DISTINCT ?id ?objectType WHERE {{
          {id_values}
          {type_values}
          {ref_values}
          GRAPH ?g {{
            ?subject <{object_id}> ?id ;
                     <{object_type}> ?objectType .
          }}
        }}
        ORDER BY ?id ?objectType
        "#,
        object_id = vocab::OBJECT_ID,
        object_type = vocab::OBJECT_TYPE,
    )
}

pub(super) fn derived_memory_ids_by_provenance_query(sources: &[String]) -> String {
    let values = sparql_iri_values("source", sources.iter().map(String::as_str));
    format!(
        r#"
        SELECT DISTINCT ?id WHERE {{
          {values}
          GRAPH ?memoryGraph {{
            ?memory a <{derived_class}> ;
                    <{object_id}> ?id .
          }}
          GRAPH ?provenanceGraph {{
            {{
              ?memory <{derived_from_episode}> ?source .
            }} UNION {{
              ?memory <{derived_from_observation}> ?source .
            }} UNION {{
              ?memory <{derived_from_relation}> ?source .
            }} UNION {{
              ?source <{derived_from_relation}> ?memory .
            }}
          }}
        }}
        "#,
        derived_class = vocab::CLASS_DERIVED_MEMORY,
        object_id = vocab::OBJECT_ID,
        derived_from_episode = vocab::DERIVED_FROM_EPISODE,
        derived_from_observation = vocab::DERIVED_FROM_OBSERVATION,
        derived_from_relation = vocab::relation_predicate("derived_from"),
    )
}

pub(super) fn derived_memory_ids_by_resource_predicate_query(
    predicate: &str,
    resources: &[String],
) -> String {
    let values = sparql_iri_values("resource", resources.iter().map(String::as_str));
    format!(
        r#"
        SELECT DISTINCT ?id WHERE {{
          {values}
          GRAPH ?g {{
            ?memory a <{derived_class}> ;
                    <{object_id}> ?id ;
                    <{predicate}> ?resource .
          }}
        }}
        "#,
        derived_class = vocab::CLASS_DERIVED_MEMORY,
        object_id = vocab::OBJECT_ID,
    )
}

pub(super) fn link_ids_query() -> String {
    format!(
        r#"
        SELECT DISTINCT ?id WHERE {{
          GRAPH ?g {{
            ?link a <{link_class}> ;
                  <{object_id}> ?id .
          }}
        }}
        "#,
        link_class = vocab::CLASS_MEMORY_LINK,
        object_id = vocab::OBJECT_ID,
    )
}

pub(super) fn links_touching_query(object_refs: &[GraphObjectRef]) -> String {
    let node_values = sparql_node_iri_values("node", object_refs);
    format!(
        r#"
        SELECT DISTINCT ?linkId ?fromId ?fromType ?toId ?toType ?relation WHERE {{
          {node_values}
          GRAPH ?linkGraph {{
            ?link a <{link_class}> ;
                  <{object_id}> ?linkId ;
                  <{from}> ?from ;
                  <{to}> ?to ;
                  <{relation}> ?relation .
            {{
              ?link <{from}> ?node .
            }} UNION {{
              ?link <{to}> ?node .
            }}
          }}
          GRAPH ?fromGraph {{
            ?from <{object_id}> ?fromId ;
                  <{object_type}> ?fromType .
          }}
          GRAPH ?toGraph {{
            ?to <{object_id}> ?toId ;
                <{object_type}> ?toType .
          }}
        }}
        "#,
        link_class = vocab::CLASS_MEMORY_LINK,
        object_id = vocab::OBJECT_ID,
        object_type = vocab::OBJECT_TYPE,
        from = vocab::FROM,
        to = vocab::TO,
        relation = vocab::RELATION,
    )
}

pub(super) fn sparql_iri_values<'a>(
    variable: &str,
    values: impl Iterator<Item = &'a str>,
) -> String {
    let values = values
        .map(|value| format!("<{}>", sparql_iri(value)))
        .collect::<Vec<_>>()
        .join(" ");
    if values.is_empty() {
        return String::new();
    }
    format!("VALUES ?{variable} {{ {values} }}")
}

pub(super) fn sparql_node_iri_values(variable: &str, object_refs: &[GraphObjectRef]) -> String {
    let values = object_refs
        .iter()
        .map(|object_ref| {
            let graph_uri = graph_uri(object_ref.object_type, object_ref.object_id);
            format!("<{}>", sparql_iri(&graph_uri))
        })
        .collect::<Vec<_>>()
        .join(" ");
    if values.is_empty() {
        return String::new();
    }
    format!("VALUES ?{variable} {{ {values} }}")
}

pub(super) fn sparql_object_ref_values(object_refs: &[GraphObjectRef]) -> String {
    let values = object_refs
        .iter()
        .map(|object_ref| {
            format!(
                "({} {})",
                sparql_string_literal(&object_ref.object_id.to_string()),
                sparql_string_literal(&enum_value(object_ref.object_type)),
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    if values.is_empty() {
        return String::new();
    }
    format!("VALUES (?id ?objectType) {{ {values} }}")
}

pub(super) fn sparql_literal_values(
    variable: &str,
    values: impl Iterator<Item = String>,
) -> String {
    let values = values
        .map(|value| sparql_string_literal(&value))
        .collect::<Vec<_>>()
        .join(" ");
    if values.is_empty() {
        return String::new();
    }
    format!("VALUES ?{variable} {{ {values} }}")
}

pub(super) fn sparql_string_literal(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a SPARQL string literal cannot fail")
}

pub(super) fn sparql_iri(value: &str) -> String {
    value.replace('>', "%3E")
}

pub(super) fn enum_value(value: impl serde::Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_default()
}

#[derive(Debug, Deserialize)]
pub(super) struct SparqlSelectResponse {
    pub(super) results: SparqlResults,
}

#[derive(Debug, Deserialize)]
pub(super) struct SparqlResults {
    pub(super) bindings: Vec<HashMap<String, SparqlBinding>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SparqlBinding {
    #[serde(rename = "type")]
    kind: String,
    pub(super) value: String,
    // SPARQL JSON includes datatype metadata for literals; remove if parsing no longer preserves literal metadata.
    #[allow(dead_code)]
    datatype: Option<String>,
}

pub(super) fn quad_from_sparql_binding(
    binding: &HashMap<String, SparqlBinding>,
) -> Result<Quad, CustomError> {
    let graph = named_node_binding(binding, "g")?;
    let subject = named_node_binding(binding, "s")?;
    let predicate = named_node_binding(binding, "p")?;
    let object = term_binding(binding, "o")?;

    Ok(Quad::new(
        NamedOrBlankNode::NamedNode(subject),
        predicate,
        object,
        GraphName::NamedNode(graph),
    ))
}

pub(super) fn named_node_binding(
    binding: &HashMap<String, SparqlBinding>,
    name: &'static str,
) -> Result<NamedNode, CustomError> {
    let value = binding.get(name).ok_or_else(|| {
        CustomError::DatabaseError(format!("Oxigraph SPARQL result is missing binding {name}"))
    })?;
    if value.kind != "uri" {
        return Err(CustomError::DatabaseError(format!(
            "Oxigraph SPARQL binding {name} must be a uri, got {}",
            value.kind
        )));
    }
    NamedNode::new(value.value.as_str()).map_err(CustomError::from)
}

pub(super) fn term_binding(
    binding: &HashMap<String, SparqlBinding>,
    name: &'static str,
) -> Result<Term, CustomError> {
    let value = binding.get(name).ok_or_else(|| {
        CustomError::DatabaseError(format!("Oxigraph SPARQL result is missing binding {name}"))
    })?;
    match value.kind.as_str() {
        "uri" => Ok(Term::NamedNode(NamedNode::new(value.value.as_str())?)),
        "literal" | "typed-literal" => Ok(Term::Literal(Literal::new_simple_literal(
            value.value.as_str(),
        ))),
        other => Err(CustomError::DatabaseError(format!(
            "Oxigraph SPARQL binding {name} has unsupported term type {other}"
        ))),
    }
}

pub(super) fn memory_id_select_binding(
    binding: &HashMap<String, SparqlBinding>,
    name: &'static str,
) -> Result<MemoryId, CustomError> {
    literal_select_binding(binding, name)?
        .parse::<MemoryId>()
        .map_err(|error| {
            CustomError::DatabaseError(format!(
                "Oxigraph SPARQL invalid MemoryId binding {name}: {error}"
            ))
        })
}

pub(super) fn enum_select_binding<T: DeserializeOwned>(
    binding: &HashMap<String, SparqlBinding>,
    name: &'static str,
) -> Result<T, CustomError> {
    serde_json::from_value(serde_json::Value::String(
        literal_select_binding(binding, name)?.to_owned(),
    ))
    .map_err(|error| {
        CustomError::DatabaseError(format!(
            "Oxigraph SPARQL invalid enum binding {name}: {error}"
        ))
    })
}

pub(super) fn literal_select_binding<'a>(
    binding: &'a HashMap<String, SparqlBinding>,
    name: &'static str,
) -> Result<&'a str, CustomError> {
    let value = binding.get(name).ok_or_else(|| {
        CustomError::DatabaseError(format!("Oxigraph SPARQL result is missing binding {name}"))
    })?;
    if value.kind != "literal" && value.kind != "typed-literal" {
        return Err(CustomError::DatabaseError(format!(
            "Oxigraph SPARQL binding {name} must be a literal, got {}",
            value.kind
        )));
    }
    Ok(value.value.as_str())
}

pub(super) fn object_matches_query(object_ref: GraphObjectRef, query: &GraphObjectQuery) -> bool {
    (query.object_refs.is_empty() || query.object_refs.contains(&object_ref))
        && (query.object_ids.is_empty() || query.object_ids.contains(&object_ref.object_id))
        && (query.object_types.is_empty() || query.object_types.contains(&object_ref.object_type))
}

pub(super) fn sort_graph_object_refs(refs: &mut [GraphObjectRef]) {
    refs.sort_by_key(|object_ref| stable_node_key((object_ref.object_id, object_ref.object_type)));
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

    fn literal_values(&self, predicate: &'static str) -> Vec<String> {
        self.literals.get(predicate).cloned().unwrap_or_default()
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
    refs: &[GraphObjectRef],
) -> Result<Vec<MemoryObject>, CustomError> {
    let subjects = rdf_subject_values(store)?;
    let mut objects = Vec::new();
    for object_ref in refs {
        if object_ref.object_type == ObjectType::MemoryLink {
            continue;
        }
        let subject = graph_uri(object_ref.object_type, object_ref.object_id);
        if let Some(values) = subjects.get(&subject) {
            objects.push(memory_object_from_rdf(
                &subject,
                values,
                object_ref.object_type,
            )?);
        }
    }
    sort_objects(&mut objects);
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

pub(super) fn hydrate_links_by_id_sets_from_store(
    store: &Store,
    graph_link_ids: &HashSet<MemoryId>,
    lifecycle_link_ids: &HashSet<MemoryId>,
    graph_ref_set: &HashSet<GraphObjectRef>,
) -> Result<Vec<MemoryLink>, CustomError> {
    let links = hydrate_all_links_from_store(store)?;
    Ok(links
        .into_iter()
        .filter(|link| graph_link_ids.contains(&link.id) || lifecycle_link_ids.contains(&link.id))
        .filter(|link| {
            let endpoints_in_graph = graph_ref_set
                .contains(&GraphObjectRef::new(link.from_id, link.from_type))
                && graph_ref_set.contains(&GraphObjectRef::new(link.to_id, link.to_type));
            (graph_link_ids.contains(&link.id) && endpoints_in_graph)
                || lifecycle_link_ids.contains(&link.id)
        })
        .collect())
}

pub(super) fn rdf_subject_values(
    store: &Store,
) -> Result<HashMap<String, RdfSubjectValues>, CustomError> {
    let mut subjects = HashMap::<String, RdfSubjectValues>::new();
    for quad in store.iter() {
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

pub(super) fn memory_object_from_rdf(
    subject: &str,
    values: &RdfSubjectValues,
    object_type: ObjectType,
) -> Result<MemoryObject, CustomError> {
    match object_type {
        ObjectType::Episode => Ok(MemoryObject::Episode(Episode {
            id: memory_id_literal(subject, values, super::vocabulary::OBJECT_ID)?,
            object_type,
            modality: enum_literal(subject, values, super::vocabulary::MODALITY)?,
            source_conversation_id: values
                .optional_literal(super::vocabulary::SOURCE_CONVERSATION_ID),
            started_at: optional_timestamp_literal(values, super::vocabulary::STARTED_AT)?,
            ended_at: optional_timestamp_literal(values, super::vocabulary::ENDED_AT)?,
            participant_entity_ids: memory_ids_from_resources(
                values.resource_values(super::vocabulary::PARTICIPANT_ENTITY),
            )?,
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
            entity_type: enum_literal(subject, values, super::vocabulary::ENTITY_TYPE)?,
            name: values.literal(subject, super::vocabulary::NAME)?,
            aliases: values.literal_values(super::vocabulary::ALIAS),
            canonical_key: values.optional_literal(super::vocabulary::CANONICAL_KEY),
            summary: values.optional_literal(super::vocabulary::SUMMARY),
            created_at: timestamp_literal(subject, values, super::vocabulary::CREATED_AT)?,
            updated_at: timestamp_literal(subject, values, super::vocabulary::UPDATED_AT)?,
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
            id: memory_id_literal(subject, values, super::vocabulary::OBJECT_ID)?,
            object_type,
            derived_type: enum_literal(subject, values, super::vocabulary::DERIVED_TYPE)?,
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
            confidence: f32_literal(subject, values, super::vocabulary::CONFIDENCE)?,
            salience_score: f32_literal(subject, values, super::vocabulary::SALIENCE_SCORE)?,
            stability: enum_literal(subject, values, super::vocabulary::STABILITY)?,
            is_current: bool_literal(subject, values, super::vocabulary::IS_CURRENT)?,
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
        confidence: f32_literal(subject, values, super::vocabulary::CONFIDENCE)?,
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

pub(super) fn bool_literal(
    subject: &str,
    values: &RdfSubjectValues,
    predicate: &'static str,
) -> Result<bool, CustomError> {
    values
        .literal(subject, predicate)?
        .parse()
        .map_err(|error| rdf_parse_error(subject, predicate, error))
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
    pub(super) object_refs: HashSet<GraphObjectRef>,
    pub(super) traversal_link_ids: HashSet<MemoryId>,
    pub(super) lifecycle_link_ids: HashSet<MemoryId>,
    pub(super) bounded_failure: Option<GraphExpansionBoundedFailure>,
}

pub(super) fn bounded_graph_visible_refs(
    selectors: &SparqlGraphSelectors<'_>,
    root_ref: GraphObjectRef,
    query: &GraphExpansionQuery,
) -> Result<BoundedGraphVisibility, CustomError> {
    let mut graph_refs = HashSet::from([root_ref]);
    let mut graph_link_ids = HashSet::new();
    let mut bounded_failure = None;
    let mut frontier = vec![root_ref];

    for depth in 0..query.max_depth {
        let link_refs = selectors.select_links_touching(&frontier)?;
        let link_refs_by_endpoint = link_refs_by_endpoint(&link_refs);
        let mut next_frontier = Vec::new();
        for object_ref in &frontier {
            let incident_link_refs = link_refs_by_endpoint
                .get(object_ref)
                .map(Vec::as_slice)
                .unwrap_or_default();
            for link_ref in bounded_incident_link_refs(
                query,
                root_ref,
                *object_ref,
                depth,
                incident_link_refs,
                &mut bounded_failure,
            )? {
                let neighbor = link_ref.other_endpoint(*object_ref);
                insert_visible_ref(
                    query,
                    &mut graph_refs,
                    &mut next_frontier,
                    neighbor,
                    &mut bounded_failure,
                )?;
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
        .select_links_touching(&candidate_refs)?
        .into_iter()
        .filter(|link_ref| {
            link_ref.relation == crate::api::types::RelationType::Supersedes
                && link_ref.to.object_type == ObjectType::DerivedMemory
                && graph_refs.contains(&link_ref.to)
        })
        .map(|link_ref| link_ref.link_id)
        .collect::<HashSet<_>>();

    Ok(BoundedGraphVisibility {
        object_refs: graph_refs,
        traversal_link_ids: graph_link_ids,
        lifecycle_link_ids,
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

pub(super) fn oxigraph_http_error(error: impl std::fmt::Display) -> CustomError {
    CustomError::DatabaseError(format!("Oxigraph service error: {error}"))
}

pub(super) fn object_identity(object: &MemoryObject) -> (MemoryId, ObjectType) {
    match object {
        MemoryObject::Episode(object) => (object.id, object.object_type),
        MemoryObject::Observation(object) => (object.id, object.object_type),
        MemoryObject::Entity(object) => (object.id, object.object_type),
        MemoryObject::MemoryThread(object) => (object.id, object.object_type),
        MemoryObject::DerivedMemory(object) => (object.id, object.object_type),
        MemoryObject::MemoryLink(object) => (object.id, object.object_type),
    }
}

pub(super) fn sort_objects(objects: &mut [MemoryObject]) {
    objects.sort_by(|left, right| {
        stable_node_key(object_identity(left)).cmp(&stable_node_key(object_identity(right)))
    });
}

pub(super) fn stable_node_key(node: (MemoryId, ObjectType)) -> (MemoryId, u8) {
    (node.0, object_type_rank(node.1))
}

pub(super) fn object_type_rank(object_type: ObjectType) -> u8 {
    match object_type {
        ObjectType::Episode => 0,
        ObjectType::Observation => 1,
        ObjectType::Entity => 2,
        ObjectType::MemoryThread => 3,
        ObjectType::DerivedMemory => 4,
        ObjectType::MemoryLink => 5,
    }
}

pub(super) fn relation_type_rank(relation_type: RelationType) -> u8 {
    match relation_type {
        RelationType::HasObservation => 0,
        RelationType::ObservedIn => 1,
        RelationType::Mentions => 2,
        RelationType::Involves => 3,
        RelationType::About => 4,
        RelationType::DerivedFrom => 5,
        RelationType::PartOfThread => 6,
        RelationType::Supports => 7,
        RelationType::Contradicts => 8,
        RelationType::Supersedes => 9,
        RelationType::Resolves => 10,
        RelationType::CreatesOpenLoop => 11,
        RelationType::FulfillsCommitment => 12,
        RelationType::AssociatedWith => 13,
    }
}

impl From<oxigraph::model::IriParseError> for CustomError {
    fn from(error: oxigraph::model::IriParseError) -> Self {
        CustomError::DatabaseError(format!("Invalid RDF IRI: {error}"))
    }
}
