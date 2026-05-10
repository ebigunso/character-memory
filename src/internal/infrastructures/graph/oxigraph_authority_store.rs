// Oxigraph graph authority. Canonical domain objects and links are written to
// RDF and hydrated from Oxigraph.
#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use oxigraph::model::{GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term};
use oxigraph::store::Store;
use serde::{de::DeserializeOwned, Deserialize};

use crate::api::types::{
    graph_uri, DerivedMemory, Entity, Episode, MemoryId, MemoryLink, MemoryObject, MemoryThread,
    ObjectType, Observation, RelationType,
};
use crate::errors::CustomError;
use crate::internal::repositories::{
    bounded_expansion, derived_memories_by_provenance, derived_memories_by_thread,
    GraphAuthorityStore, GraphDerivedMemoryProvenanceQuery, GraphDerivedMemoryThreadQuery,
    GraphExpansion, GraphExpansionBoundedFailure, GraphExpansionBoundedFailureReason,
    GraphExpansionQuery, GraphObjectQuery, GraphObjectRef,
};

use super::rdf_mapping::{rdf_triples_for_link, rdf_triples_for_object, RdfObject, RdfTriple};
use super::sparql_selectors::{SparqlGraphSelectors, SparqlLinkRef};
use super::vocabulary as vocab;

pub(crate) struct OxigraphGraphAuthorityStore {
    store: Store,
    inserted_quads: Mutex<HashMap<String, Vec<Quad>>>,
}

pub(crate) struct OxigraphHttpGraphAuthorityStore {
    endpoint: String,
    client: reqwest::Client,
}

impl OxigraphGraphAuthorityStore {
    pub(crate) fn new_in_memory() -> Result<Self, CustomError> {
        let store = Store::new().map_err(oxigraph_error)?;
        Ok(Self {
            store,
            inserted_quads: Mutex::new(HashMap::new()),
        })
    }

    pub(crate) fn new_persistent(path: impl AsRef<Path>) -> Result<Self, CustomError> {
        let path = path.as_ref();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|error| {
                CustomError::DatabaseError(format!(
                    "Failed to create Oxigraph graph store parent directory {}: {error}",
                    parent.display()
                ))
            })?;
        }
        let store = Store::open(path).map_err(oxigraph_error)?;
        Ok(Self {
            store,
            inserted_quads: Mutex::new(HashMap::new()),
        })
    }

    pub(crate) fn triple_count(&self) -> Result<usize, CustomError> {
        Ok(self.store.iter().count())
    }

    fn replace_triples(
        &self,
        owner_graph_uri: String,
        triples: &[RdfTriple],
    ) -> Result<(), CustomError> {
        let quads = quads_for_triples(&owner_graph_uri, triples)?;
        self.replace_triples_batch(vec![(owner_graph_uri, quads)])
    }

    fn replace_triples_batch(
        &self,
        replacements: Vec<(String, Vec<Quad>)>,
    ) -> Result<(), CustomError> {
        let mut inserted_quads = lock(&self.inserted_quads)?;
        let mut transaction = self.store.start_transaction().map_err(oxigraph_error)?;

        for (owner_graph_uri, _) in &replacements {
            if let Some(previous_quads) = inserted_quads.get(owner_graph_uri) {
                for quad in previous_quads {
                    transaction.remove(quad.as_ref());
                }
            } else {
                for quad in self.quads_in_graph(owner_graph_uri)? {
                    transaction.remove(quad.as_ref());
                }
            }
        }

        for (_, quads) in &replacements {
            for quad in quads {
                transaction.insert(quad.as_ref());
            }
        }

        transaction.commit().map_err(oxigraph_error)?;
        for (owner_graph_uri, quads) in replacements {
            inserted_quads.insert(owner_graph_uri, quads);
        }
        Ok(())
    }

    fn insert_quads(&self, quads: &[Quad]) -> Result<(), CustomError> {
        for quad in quads {
            self.store.insert(quad).map_err(oxigraph_error)?;
        }
        Ok(())
    }

    fn remove_quads(&self, quads: &[Quad]) -> Result<(), CustomError> {
        for quad in quads {
            self.store.remove(quad).map_err(oxigraph_error)?;
        }
        Ok(())
    }

    fn quads_in_graph(&self, owner_graph_uri: &str) -> Result<Vec<Quad>, CustomError> {
        let graph_name = GraphName::NamedNode(NamedNode::new(owner_graph_uri)?);
        self.store
            .iter()
            .filter_map(|quad| match quad {
                Ok(quad) if quad.graph_name == graph_name => Some(Ok(quad)),
                Ok(_) => None,
                Err(error) => Some(Err(oxigraph_error(error))),
            })
            .collect()
    }

    #[cfg(test)]
    fn contains_triple(&self, triple: &RdfTriple) -> Result<bool, CustomError> {
        let subject = NamedOrBlankNode::NamedNode(NamedNode::new(triple.subject.as_str())?);
        let predicate = NamedNode::new(triple.predicate.as_str())?;
        let object = match &triple.object {
            RdfObject::Resource(value) => Term::NamedNode(NamedNode::new(value.as_str())?),
            RdfObject::Literal(value) => Term::Literal(Literal::new_simple_literal(value.as_str())),
        };

        for quad in self.store.iter() {
            let quad = quad.map_err(oxigraph_error)?;
            if quad.subject == subject && quad.predicate == predicate && quad.object == object {
                return Ok(true);
            }
        }

        Ok(false)
    }

    #[cfg(test)]
    fn contains_triple_in_graph(
        &self,
        triple: &RdfTriple,
        owner_graph_uri: &str,
    ) -> Result<bool, CustomError> {
        let subject = NamedOrBlankNode::NamedNode(NamedNode::new(triple.subject.as_str())?);
        let predicate = NamedNode::new(triple.predicate.as_str())?;
        let graph_name = GraphName::NamedNode(NamedNode::new(owner_graph_uri)?);
        let object = match &triple.object {
            RdfObject::Resource(value) => Term::NamedNode(NamedNode::new(value.as_str())?),
            RdfObject::Literal(value) => Term::Literal(Literal::new_simple_literal(value.as_str())),
        };

        for quad in self.store.iter() {
            let quad = quad.map_err(oxigraph_error)?;
            if quad.subject == subject
                && quad.predicate == predicate
                && quad.object == object
                && quad.graph_name == graph_name
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    #[cfg(test)]
    fn matching_triple_count(&self, triple: &RdfTriple) -> Result<usize, CustomError> {
        let subject = NamedOrBlankNode::NamedNode(NamedNode::new(triple.subject.as_str())?);
        let predicate = NamedNode::new(triple.predicate.as_str())?;
        let object = match &triple.object {
            RdfObject::Resource(value) => Term::NamedNode(NamedNode::new(value.as_str())?),
            RdfObject::Literal(value) => Term::Literal(Literal::new_simple_literal(value.as_str())),
        };

        let mut count = 0;
        for quad in self.store.iter() {
            let quad = quad.map_err(oxigraph_error)?;
            if quad.subject == subject && quad.predicate == predicate && quad.object == object {
                count += 1;
            }
        }

        Ok(count)
    }
}

impl OxigraphHttpGraphAuthorityStore {
    pub(crate) fn new(endpoint: impl AsRef<str>) -> Result<Self, CustomError> {
        let endpoint = endpoint.as_ref().trim().trim_end_matches('/');
        if !(endpoint.starts_with("http://") || endpoint.starts_with("https://")) {
            return Err(CustomError::ConfigParseError(
                "Oxigraph service endpoint must start with http:// or https://".to_owned(),
            ));
        }
        Ok(Self {
            endpoint: endpoint.to_owned(),
            client: reqwest::Client::new(),
        })
    }

    async fn replace_triples_batch(
        &self,
        replacements: Vec<(String, Vec<Quad>)>,
    ) -> Result<(), CustomError> {
        if replacements.is_empty() {
            return Ok(());
        }

        let mut update = String::new();
        for (owner_graph_uri, _) in &replacements {
            update.push_str("DELETE { GRAPH <");
            update.push_str(owner_graph_uri);
            update.push_str("> { ?s ?p ?o } } WHERE { GRAPH <");
            update.push_str(owner_graph_uri);
            update.push_str("> { ?s ?p ?o } };\n");
        }
        update.push_str("INSERT DATA {\n");
        for (_, quads) in &replacements {
            for quad in quads {
                update.push_str(&sparql_quad(quad)?);
                update.push('\n');
            }
        }
        update.push_str("}\n");

        self.post_update(update).await
    }

    async fn post_update(&self, update: String) -> Result<(), CustomError> {
        let response = self
            .client
            .post(format!("{}/update", self.endpoint))
            .header("content-type", "application/sparql-update")
            .body(update)
            .send()
            .await
            .map_err(oxigraph_http_error)?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(CustomError::DatabaseError(format!(
                "Oxigraph update failed with {status}: {body}"
            )))
        }
    }

    #[cfg(test)]
    async fn delete_named_graphs(&self, graph_uris: &[String]) -> Result<(), CustomError> {
        if graph_uris.is_empty() {
            return Ok(());
        }

        let mut update = String::new();
        for graph_uri in graph_uris {
            update.push_str("DROP SILENT GRAPH <");
            update.push_str(graph_uri);
            update.push_str(">;\n");
        }
        self.post_update(update).await
    }

    #[cfg(test)]
    async fn named_graph_quad_count(&self, graph_uris: &[String]) -> Result<usize, CustomError> {
        if graph_uris.is_empty() {
            return Ok(0);
        }

        let mut query = String::from("SELECT (COUNT(*) AS ?count) WHERE { VALUES ?g {");
        for graph_uri in graph_uris {
            query.push_str(" <");
            query.push_str(graph_uri);
            query.push('>');
        }
        query.push_str(" } GRAPH ?g { ?s ?p ?o } }");

        let response = self
            .client
            .post(format!("{}/query", self.endpoint))
            .header("accept", "application/sparql-results+json")
            .header("content-type", "application/sparql-query")
            .body(query)
            .send()
            .await
            .map_err(oxigraph_http_error)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(CustomError::DatabaseError(format!(
                "Oxigraph query failed with {status}: {body}"
            )));
        }

        let result = response
            .json::<SparqlSelectResponse>()
            .await
            .map_err(oxigraph_http_error)?;
        let count = result
            .results
            .bindings
            .first()
            .and_then(|binding| binding.get("count"))
            .ok_or_else(|| {
                CustomError::DatabaseError(
                    "Oxigraph SPARQL count result is missing count binding".to_owned(),
                )
            })?;
        count.value.parse().map_err(|error| {
            CustomError::DatabaseError(format!("Invalid Oxigraph SPARQL count literal: {error}"))
        })
    }

    async fn post_select(&self, query: String) -> Result<SparqlSelectResponse, CustomError> {
        let response = self
            .client
            .post(format!("{}/query", self.endpoint))
            .header("accept", "application/sparql-results+json")
            .header("content-type", "application/sparql-query")
            .body(query)
            .send()
            .await
            .map_err(oxigraph_http_error)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(CustomError::DatabaseError(format!(
                "Oxigraph query failed with {status}: {body}"
            )));
        }

        response
            .json::<SparqlSelectResponse>()
            .await
            .map_err(oxigraph_http_error)
    }

    async fn store_for_named_graphs(&self, graph_uris: &[String]) -> Result<Store, CustomError> {
        let store = Store::new().map_err(oxigraph_error)?;
        if graph_uris.is_empty() {
            return Ok(store);
        }

        let result = self
            .post_select(named_graph_quads_query(graph_uris))
            .await?;
        for binding in result.results.bindings {
            store
                .insert(&quad_from_sparql_binding(&binding)?)
                .map_err(oxigraph_error)?;
        }
        Ok(store)
    }

    async fn query_object_refs(
        &self,
        query: &GraphObjectQuery,
    ) -> Result<Vec<GraphObjectRef>, CustomError> {
        let result = self.post_select(object_refs_query(query)).await?;
        let mut refs = Vec::new();
        let mut seen = HashSet::new();
        for binding in result.results.bindings {
            let object_ref = GraphObjectRef::new(
                memory_id_select_binding(&binding, "id")?,
                enum_select_binding(&binding, "objectType")?,
            );
            if object_matches_query(object_ref, query) && seen.insert(object_ref) {
                refs.push(object_ref);
            }
        }
        sort_graph_object_refs(&mut refs);
        if let Some(limit) = query.limit {
            refs.truncate(limit);
        }
        Ok(refs)
    }

    async fn query_derived_memory_ids_by_provenance(
        &self,
        query: &GraphDerivedMemoryProvenanceQuery,
    ) -> Result<Vec<MemoryId>, CustomError> {
        let sources = query
            .episode_ids
            .iter()
            .map(|id| graph_uri(ObjectType::Episode, *id))
            .chain(
                query
                    .observation_ids
                    .iter()
                    .map(|id| graph_uri(ObjectType::Observation, *id)),
            )
            .collect::<Vec<_>>();
        if sources.is_empty() {
            return Ok(Vec::new());
        }

        self.query_memory_ids(
            derived_memory_ids_by_provenance_query(&sources),
            query.limit,
        )
        .await
    }

    async fn query_derived_memory_ids_by_thread(
        &self,
        query: &GraphDerivedMemoryThreadQuery,
    ) -> Result<Vec<MemoryId>, CustomError> {
        let threads = query
            .thread_ids
            .iter()
            .map(|id| graph_uri(ObjectType::MemoryThread, *id))
            .collect::<Vec<_>>();
        if threads.is_empty() {
            return Ok(Vec::new());
        }

        self.query_memory_ids(
            derived_memory_ids_by_resource_predicate_query(vocab::PART_OF_THREAD, &threads),
            query.limit,
        )
        .await
    }

    async fn query_memory_ids(
        &self,
        query: String,
        limit: Option<usize>,
    ) -> Result<Vec<MemoryId>, CustomError> {
        let result = self.post_select(query).await?;
        let mut ids = Vec::new();
        let mut seen = HashSet::new();
        for binding in result.results.bindings {
            let id = memory_id_select_binding(&binding, "id")?;
            if seen.insert(id) {
                ids.push(id);
            }
        }
        ids.sort();
        if let Some(limit) = limit {
            ids.truncate(limit);
        }
        Ok(ids)
    }

    async fn query_link_refs_touching(
        &self,
        object_refs: &[GraphObjectRef],
    ) -> Result<Vec<SparqlHttpLinkRef>, CustomError> {
        if object_refs.is_empty() {
            return Ok(Vec::new());
        }

        let result = self.post_select(links_touching_query(object_refs)).await?;
        let mut refs = Vec::new();
        let mut seen = HashSet::new();
        for binding in result.results.bindings {
            let link_ref = SparqlHttpLinkRef {
                link_id: memory_id_select_binding(&binding, "linkId")?,
                from: GraphObjectRef::new(
                    memory_id_select_binding(&binding, "fromId")?,
                    enum_select_binding(&binding, "fromType")?,
                ),
                to: GraphObjectRef::new(
                    memory_id_select_binding(&binding, "toId")?,
                    enum_select_binding(&binding, "toType")?,
                ),
                relation: enum_select_binding(&binding, "relation")?,
            };
            if seen.insert(link_ref) {
                refs.push(link_ref);
            }
        }
        refs.sort_by_key(|link_ref| {
            (
                link_ref.to.object_id,
                link_ref.from.object_id,
                link_ref.link_id,
                object_type_rank(link_ref.to.object_type),
                object_type_rank(link_ref.from.object_type),
                relation_type_rank(link_ref.relation),
            )
        });
        Ok(refs)
    }

    async fn hydrate_objects_by_refs(
        &self,
        refs: &[GraphObjectRef],
    ) -> Result<Vec<MemoryObject>, CustomError> {
        let graph_uris = refs
            .iter()
            .filter(|object_ref| object_ref.object_type != ObjectType::MemoryLink)
            .map(|object_ref| graph_uri(object_ref.object_type, object_ref.object_id))
            .collect::<Vec<_>>();
        let store = self.store_for_named_graphs(&graph_uris).await?;
        hydrate_objects_by_refs_from_store(&store, refs)
    }

    async fn hydrate_links_by_ids(
        &self,
        link_ids: &HashSet<MemoryId>,
    ) -> Result<Vec<MemoryLink>, CustomError> {
        let graph_uris = link_ids
            .iter()
            .map(|id| graph_uri(ObjectType::MemoryLink, *id))
            .collect::<Vec<_>>();
        let store = self.store_for_named_graphs(&graph_uris).await?;
        hydrate_all_links_from_store(&store)
    }

    async fn hydrate_links_touching(
        &self,
        refs: &[GraphObjectRef],
    ) -> Result<Vec<MemoryLink>, CustomError> {
        let link_ids = self
            .query_link_refs_touching(refs)
            .await?
            .into_iter()
            .map(|link_ref| link_ref.link_id)
            .collect::<HashSet<_>>();
        self.hydrate_links_by_ids(&link_ids).await
    }

    async fn bounded_graph_visible_refs(
        &self,
        root_ref: GraphObjectRef,
        query: &GraphExpansionQuery,
    ) -> Result<BoundedGraphVisibility, CustomError> {
        let mut graph_refs = HashSet::from([root_ref]);
        let mut graph_link_ids = HashSet::new();
        let mut bounded_failure = None;
        let mut frontier = vec![root_ref];

        for depth in 0..query.max_depth {
            let link_refs = self.query_link_refs_touching(&frontier).await?;
            let mut next_frontier = Vec::new();
            for object_ref in &frontier {
                for link_ref in bounded_incident_link_refs(
                    query,
                    root_ref,
                    *object_ref,
                    depth,
                    &link_refs,
                    &mut bounded_failure,
                )? {
                    graph_link_ids.insert(link_ref.link_id());
                    let neighbor = link_ref.other_endpoint(*object_ref);
                    if graph_refs.insert(neighbor) {
                        next_frontier.push(neighbor);
                    }
                }
            }

            if next_frontier.is_empty() {
                break;
            }
            frontier = next_frontier;
        }

        let candidate_refs = graph_refs.iter().copied().collect::<Vec<_>>();
        let lifecycle_link_ids = self
            .query_link_refs_touching(&candidate_refs)
            .await?
            .into_iter()
            .filter(|link_ref| {
                link_ref.relation == RelationType::Supersedes
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
}

trait BoundedExpansionLinkRef: Copy {
    fn link_id(self) -> MemoryId;
    fn from(self) -> GraphObjectRef;
    fn to(self) -> GraphObjectRef;
    fn relation(self) -> RelationType;

    fn touches(self, object_ref: GraphObjectRef) -> bool {
        self.from() == object_ref || self.to() == object_ref
    }

    fn other_endpoint(self, object_ref: GraphObjectRef) -> GraphObjectRef {
        if self.from() == object_ref {
            self.to()
        } else {
            self.from()
        }
    }
}

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

fn bounded_incident_link_refs<T: BoundedExpansionLinkRef>(
    query: &GraphExpansionQuery,
    root_ref: GraphObjectRef,
    object_ref: GraphObjectRef,
    depth: u8,
    link_refs: &[T],
    bounded_failure: &mut Option<GraphExpansionBoundedFailure>,
) -> Result<Vec<T>, CustomError> {
    let mut incident_links = link_refs
        .iter()
        .copied()
        .filter(|link_ref| relation_allowed(query, link_ref.relation()))
        .filter(|link_ref| link_ref.touches(object_ref))
        .filter(|link_ref| {
            object_type_allowed(query, link_ref.other_endpoint(object_ref).object_type)
        })
        .collect::<Vec<_>>();
    incident_links.sort_by_key(|link_ref| stable_link_ref_key(*link_ref));

    if incident_links.len() > query.max_hub_edges {
        let failure = GraphExpansionBoundedFailure {
            reason: GraphExpansionBoundedFailureReason::HubLimit,
            at: Some(object_ref),
        };
        if !query.failure_policy.allow_partial_results {
            return Err(graph_expansion_bounded_error(failure));
        }
        bounded_failure.get_or_insert(failure);
        incident_links.truncate(query.max_hub_edges);
    }
    let apply_selectivity_overrides = depth == 0 && object_ref == root_ref;
    Ok(apply_link_ref_fanout_limits(
        query,
        object_ref,
        incident_links,
        apply_selectivity_overrides,
    ))
}

fn apply_link_ref_fanout_limits<T: BoundedExpansionLinkRef>(
    query: &GraphExpansionQuery,
    object_ref: GraphObjectRef,
    mut incident_links: Vec<T>,
    apply_selectivity_overrides: bool,
) -> Vec<T> {
    if query.fanout_overrides.is_empty() || !apply_selectivity_overrides {
        incident_links.truncate(query.max_fanout_per_node);
        return incident_links;
    }

    let mut retained = Vec::new();
    let mut per_pair_counts = HashMap::<(RelationType, ObjectType), usize>::new();
    for link_ref in incident_links {
        if retained.len() >= query.max_fanout_per_node {
            break;
        }
        let neighbor = link_ref.other_endpoint(object_ref);
        let max_for_pair = fanout_limit_for_pair(query, link_ref.relation(), neighbor.object_type);
        let count = per_pair_counts
            .entry((link_ref.relation(), neighbor.object_type))
            .or_default();
        if *count >= max_for_pair {
            continue;
        }
        *count += 1;
        retained.push(link_ref);
    }
    retained
}

fn fanout_limit_for_pair(
    query: &GraphExpansionQuery,
    relation: RelationType,
    object_type: ObjectType,
) -> usize {
    query
        .fanout_overrides
        .iter()
        .find(|override_| override_.relation == relation && override_.object_type == object_type)
        .map(|override_| override_.max_fanout)
        .unwrap_or(query.max_fanout_per_node)
        .min(query.max_fanout_per_node)
}

fn relation_allowed(query: &GraphExpansionQuery, relation: RelationType) -> bool {
    query.allowed_relation_types.is_empty() || query.allowed_relation_types.contains(&relation)
}

fn object_type_allowed(query: &GraphExpansionQuery, object_type: ObjectType) -> bool {
    query.allowed_object_types.is_empty() || query.allowed_object_types.contains(&object_type)
}

fn stable_link_ref_key<T: BoundedExpansionLinkRef>(
    link_ref: T,
) -> (MemoryId, MemoryId, MemoryId, u8, u8, u8) {
    (
        link_ref.to().object_id,
        link_ref.from().object_id,
        link_ref.link_id(),
        object_type_rank(link_ref.to().object_type),
        object_type_rank(link_ref.from().object_type),
        relation_type_rank(link_ref.relation()),
    )
}

fn graph_expansion_bounded_error(failure: GraphExpansionBoundedFailure) -> CustomError {
    let location = failure
        .at
        .map(|object_ref| {
            format!(
                " at object_type={} object_id={}",
                graph_object_type_name(object_ref.object_type),
                object_ref.object_id
            )
        })
        .unwrap_or_default();

    CustomError::GraphExpansionBounded {
        reason: bounded_failure_reason_name(failure.reason).to_owned(),
        location,
    }
}

fn bounded_failure_reason_name(reason: GraphExpansionBoundedFailureReason) -> &'static str {
    match reason {
        GraphExpansionBoundedFailureReason::NodeLimit => "node_limit",
        GraphExpansionBoundedFailureReason::Timeout => "timeout",
        GraphExpansionBoundedFailureReason::HubLimit => "hub_limit",
    }
}

fn graph_object_type_name(object_type: ObjectType) -> &'static str {
    match object_type {
        ObjectType::Episode => "episode",
        ObjectType::Observation => "observation",
        ObjectType::Entity => "entity",
        ObjectType::MemoryThread => "memory_thread",
        ObjectType::DerivedMemory => "derived_memory",
        ObjectType::MemoryLink => "memory_link",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SparqlHttpLinkRef {
    link_id: MemoryId,
    from: GraphObjectRef,
    to: GraphObjectRef,
    relation: RelationType,
}

fn quads_for_triples(
    owner_graph_uri: &str,
    triples: &[RdfTriple],
) -> Result<Vec<Quad>, CustomError> {
    triples
        .iter()
        .map(|triple| quad_for_triple(owner_graph_uri, triple))
        .collect()
}

fn sparql_quad(quad: &Quad) -> Result<String, CustomError> {
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

fn sparql_term(term: &Term) -> Result<String, CustomError> {
    match term {
        Term::NamedNode(value) => Ok(format!("<{}>", value.as_str())),
        Term::Literal(value) => Ok(format!("\"{}\"", sparql_escape_literal(value.value()))),
        Term::BlankNode(_) => Err(CustomError::DatabaseError(
            "Oxigraph service writes do not support blank-node objects".to_owned(),
        )),
    }
}

fn sparql_escape_literal(value: &str) -> String {
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

fn named_graph_quads_query(graph_uris: &[String]) -> String {
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

fn object_refs_query(query: &GraphObjectQuery) -> String {
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

fn derived_memory_ids_by_provenance_query(sources: &[String]) -> String {
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

fn derived_memory_ids_by_resource_predicate_query(predicate: &str, resources: &[String]) -> String {
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

fn link_ids_query() -> String {
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

fn links_touching_query(object_refs: &[GraphObjectRef]) -> String {
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

fn sparql_iri_values<'a>(variable: &str, values: impl Iterator<Item = &'a str>) -> String {
    let values = values
        .map(|value| format!("<{}>", sparql_iri(value)))
        .collect::<Vec<_>>()
        .join(" ");
    if values.is_empty() {
        return String::new();
    }
    format!("VALUES ?{variable} {{ {values} }}")
}

fn sparql_node_iri_values(variable: &str, object_refs: &[GraphObjectRef]) -> String {
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

fn sparql_object_ref_values(object_refs: &[GraphObjectRef]) -> String {
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

fn sparql_literal_values(variable: &str, values: impl Iterator<Item = String>) -> String {
    let values = values
        .map(|value| sparql_string_literal(&value))
        .collect::<Vec<_>>()
        .join(" ");
    if values.is_empty() {
        return String::new();
    }
    format!("VALUES ?{variable} {{ {values} }}")
}

fn sparql_string_literal(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a SPARQL string literal cannot fail")
}

fn sparql_iri(value: &str) -> String {
    value.replace('>', "%3E")
}

fn enum_value(value: impl serde::Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_default()
}

#[derive(Debug, Deserialize)]
struct SparqlSelectResponse {
    results: SparqlResults,
}

#[derive(Debug, Deserialize)]
struct SparqlResults {
    bindings: Vec<HashMap<String, SparqlBinding>>,
}

#[derive(Debug, Deserialize)]
struct SparqlBinding {
    #[serde(rename = "type")]
    kind: String,
    value: String,
}

fn quad_from_sparql_binding(binding: &HashMap<String, SparqlBinding>) -> Result<Quad, CustomError> {
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

fn named_node_binding(
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

fn term_binding(
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

fn memory_id_select_binding(
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

fn enum_select_binding<T: DeserializeOwned>(
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

fn literal_select_binding<'a>(
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

fn object_matches_query(object_ref: GraphObjectRef, query: &GraphObjectQuery) -> bool {
    (query.object_refs.is_empty() || query.object_refs.contains(&object_ref))
        && (query.object_ids.is_empty() || query.object_ids.contains(&object_ref.object_id))
        && (query.object_types.is_empty() || query.object_types.contains(&object_ref.object_type))
}

fn sort_graph_object_refs(refs: &mut [GraphObjectRef]) {
    refs.sort_by_key(|object_ref| stable_node_key((object_ref.object_id, object_ref.object_type)));
}

#[async_trait]
impl GraphAuthorityStore for OxigraphGraphAuthorityStore {
    async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError> {
        let mut replacements = Vec::new();
        for object in objects {
            object
                .validate()
                .map_err(|error| CustomError::MemoryValidation(error.to_string()))?;
            let (object_id, object_type) = object_identity(object);
            let owner_graph_uri = graph_uri(object_type, object_id);
            replacements.push((
                owner_graph_uri.clone(),
                quads_for_triples(&owner_graph_uri, &rdf_triples_for_object(object)?)?,
            ));
        }

        self.replace_triples_batch(replacements)?;

        Ok(())
    }

    async fn upsert_links(&self, links: &[MemoryLink]) -> Result<(), CustomError> {
        let mut replacements = Vec::new();
        for link in links {
            link.validate()
                .map_err(|error| CustomError::MemoryValidation(error.to_string()))?;
            let owner_graph_uri = graph_uri(ObjectType::MemoryLink, link.id);
            replacements.push((
                owner_graph_uri.clone(),
                quads_for_triples(&owner_graph_uri, &rdf_triples_for_link(link)?)?,
            ));
        }

        self.replace_triples_batch(replacements)?;

        Ok(())
    }

    async fn upsert_objects_and_links(
        &self,
        objects: &[MemoryObject],
        links: &[MemoryLink],
    ) -> Result<(), CustomError> {
        let mut replacements = Vec::new();
        for object in objects {
            object
                .validate()
                .map_err(|error| CustomError::MemoryValidation(error.to_string()))?;
            let (object_id, object_type) = object_identity(object);
            let owner_graph_uri = graph_uri(object_type, object_id);
            replacements.push((
                owner_graph_uri.clone(),
                quads_for_triples(&owner_graph_uri, &rdf_triples_for_object(object)?)?,
            ));
        }

        for link in links {
            link.validate()
                .map_err(|error| CustomError::MemoryValidation(error.to_string()))?;
            let owner_graph_uri = graph_uri(ObjectType::MemoryLink, link.id);
            replacements.push((
                owner_graph_uri.clone(),
                quads_for_triples(&owner_graph_uri, &rdf_triples_for_link(link)?)?,
            ));
        }

        self.replace_triples_batch(replacements)?;

        Ok(())
    }

    async fn query_objects(
        &self,
        query: &GraphObjectQuery,
    ) -> Result<Vec<MemoryObject>, CustomError> {
        let selected_refs = SparqlGraphSelectors::new(&self.store).select_objects(query)?;
        hydrate_objects_by_refs_from_store(&self.store, &selected_refs)
    }

    async fn query_derived_memories_by_provenance(
        &self,
        query: &GraphDerivedMemoryProvenanceQuery,
    ) -> Result<Vec<DerivedMemory>, CustomError> {
        let mut selector_query = query.clone();
        selector_query.limit = None;
        let selected_ids = SparqlGraphSelectors::new(&self.store)
            .select_derived_memories_by_provenance(&selector_query)?
            .into_iter()
            .collect::<HashSet<_>>();

        let objects = hydrate_objects_by_refs_from_store(
            &self.store,
            &selected_ids
                .iter()
                .copied()
                .map(|id| GraphObjectRef::new(id, ObjectType::DerivedMemory))
                .collect::<Vec<_>>(),
        )?;
        let links = hydrate_all_links_from_store(&self.store)?;
        Ok(derived_memories_by_provenance(
            query,
            objects.into_iter().filter(
                |object| matches!(object, MemoryObject::DerivedMemory(memory) if selected_ids.contains(&memory.id)),
            ),
            links,
        ))
    }

    async fn query_derived_memories_by_thread(
        &self,
        query: &GraphDerivedMemoryThreadQuery,
    ) -> Result<Vec<DerivedMemory>, CustomError> {
        let mut selector_query = query.clone();
        selector_query.limit = None;
        let selected_ids = SparqlGraphSelectors::new(&self.store)
            .select_derived_memories_by_thread(&selector_query)?
            .into_iter()
            .collect::<HashSet<_>>();

        let objects = hydrate_objects_by_refs_from_store(
            &self.store,
            &selected_ids
                .iter()
                .copied()
                .map(|id| GraphObjectRef::new(id, ObjectType::DerivedMemory))
                .collect::<Vec<_>>(),
        )?;
        let links = hydrate_all_links_from_store(&self.store)?;
        Ok(derived_memories_by_thread(
            query,
            objects.into_iter().filter(
                |object| matches!(object, MemoryObject::DerivedMemory(memory) if selected_ids.contains(&memory.id)),
            ),
            links,
        ))
    }

    async fn expand_bounded(
        &self,
        query: &GraphExpansionQuery,
    ) -> Result<GraphExpansion, CustomError> {
        let selectors = SparqlGraphSelectors::new(&self.store);
        let root_ref = GraphObjectRef::new(query.root_id, query.root_type);
        let root_refs = selectors.select_objects(&GraphObjectQuery::by_refs(vec![root_ref]))?;
        if root_refs.is_empty() {
            return Err(CustomError::GraphExpansionRootNotFound {
                object_type: query.root_type,
                object_id: query.root_id,
            });
        }

        let visibility = bounded_graph_visible_refs(&selectors, root_ref, query)?;
        let objects = hydrate_objects_by_refs_from_store(
            &self.store,
            &visibility.object_refs.iter().copied().collect::<Vec<_>>(),
        )?;
        let links = hydrate_links_by_id_sets_from_store(
            &self.store,
            &visibility.traversal_link_ids,
            &visibility.lifecycle_link_ids,
            &visibility.object_refs,
        )?;

        let mut expansion = bounded_expansion(query, objects, links)?;
        if expansion.bounded_failure.is_none() {
            expansion.bounded_failure = visibility.bounded_failure;
        }
        Ok(expansion)
    }

    async fn list_diagnostic_objects(&self) -> Result<Vec<MemoryObject>, CustomError> {
        let object_types = [
            ObjectType::Episode,
            ObjectType::Observation,
            ObjectType::Entity,
            ObjectType::MemoryThread,
            ObjectType::DerivedMemory,
        ];
        let object_refs = SparqlGraphSelectors::new(&self.store)
            .select_objects(&GraphObjectQuery::by_types(object_types.to_vec(), None))?;
        hydrate_objects_by_refs_from_store(&self.store, &object_refs)
    }

    async fn list_diagnostic_links(&self) -> Result<Vec<MemoryLink>, CustomError> {
        hydrate_all_links_from_store(&self.store)
    }
}

#[async_trait]
impl GraphAuthorityStore for OxigraphHttpGraphAuthorityStore {
    async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError> {
        let mut replacements = Vec::new();
        for object in objects {
            object
                .validate()
                .map_err(|error| CustomError::MemoryValidation(error.to_string()))?;
            let (object_id, object_type) = object_identity(object);
            let owner_graph_uri = graph_uri(object_type, object_id);
            replacements.push((
                owner_graph_uri.clone(),
                quads_for_triples(&owner_graph_uri, &rdf_triples_for_object(object)?)?,
            ));
        }

        self.replace_triples_batch(replacements).await
    }

    async fn upsert_links(&self, links: &[MemoryLink]) -> Result<(), CustomError> {
        let mut replacements = Vec::new();
        for link in links {
            link.validate()
                .map_err(|error| CustomError::MemoryValidation(error.to_string()))?;
            let owner_graph_uri = graph_uri(ObjectType::MemoryLink, link.id);
            replacements.push((
                owner_graph_uri.clone(),
                quads_for_triples(&owner_graph_uri, &rdf_triples_for_link(link)?)?,
            ));
        }

        self.replace_triples_batch(replacements).await
    }

    async fn upsert_objects_and_links(
        &self,
        objects: &[MemoryObject],
        links: &[MemoryLink],
    ) -> Result<(), CustomError> {
        let mut replacements = Vec::new();
        for object in objects {
            object
                .validate()
                .map_err(|error| CustomError::MemoryValidation(error.to_string()))?;
            let (object_id, object_type) = object_identity(object);
            let owner_graph_uri = graph_uri(object_type, object_id);
            replacements.push((
                owner_graph_uri.clone(),
                quads_for_triples(&owner_graph_uri, &rdf_triples_for_object(object)?)?,
            ));
        }

        for link in links {
            link.validate()
                .map_err(|error| CustomError::MemoryValidation(error.to_string()))?;
            let owner_graph_uri = graph_uri(ObjectType::MemoryLink, link.id);
            replacements.push((
                owner_graph_uri.clone(),
                quads_for_triples(&owner_graph_uri, &rdf_triples_for_link(link)?)?,
            ));
        }

        self.replace_triples_batch(replacements).await
    }

    async fn query_objects(
        &self,
        query: &GraphObjectQuery,
    ) -> Result<Vec<MemoryObject>, CustomError> {
        let selected_refs = self.query_object_refs(query).await?;
        self.hydrate_objects_by_refs(&selected_refs).await
    }

    async fn query_derived_memories_by_provenance(
        &self,
        query: &GraphDerivedMemoryProvenanceQuery,
    ) -> Result<Vec<DerivedMemory>, CustomError> {
        let mut selector_query = query.clone();
        selector_query.limit = None;
        let selected_ids = self
            .query_derived_memory_ids_by_provenance(&selector_query)
            .await?
            .into_iter()
            .collect::<HashSet<_>>();

        let selected_refs = selected_ids
            .iter()
            .copied()
            .map(|id| GraphObjectRef::new(id, ObjectType::DerivedMemory))
            .collect::<Vec<_>>();
        let source_refs = query
            .episode_ids
            .iter()
            .copied()
            .map(|id| GraphObjectRef::new(id, ObjectType::Episode))
            .chain(
                query
                    .observation_ids
                    .iter()
                    .copied()
                    .map(|id| GraphObjectRef::new(id, ObjectType::Observation)),
            );
        let link_refs = selected_refs
            .iter()
            .copied()
            .chain(source_refs)
            .collect::<Vec<_>>();
        let objects = self.hydrate_objects_by_refs(&selected_refs).await?;
        let links = self.hydrate_links_touching(&link_refs).await?;
        Ok(derived_memories_by_provenance(
            query,
            objects.into_iter().filter(
                |object| matches!(object, MemoryObject::DerivedMemory(memory) if selected_ids.contains(&memory.id)),
            ),
            links,
        ))
    }

    async fn query_derived_memories_by_thread(
        &self,
        query: &GraphDerivedMemoryThreadQuery,
    ) -> Result<Vec<DerivedMemory>, CustomError> {
        let mut selector_query = query.clone();
        selector_query.limit = None;
        let selected_ids = self
            .query_derived_memory_ids_by_thread(&selector_query)
            .await?
            .into_iter()
            .collect::<HashSet<_>>();

        let selected_refs = selected_ids
            .iter()
            .copied()
            .map(|id| GraphObjectRef::new(id, ObjectType::DerivedMemory))
            .collect::<Vec<_>>();
        let thread_refs = query
            .thread_ids
            .iter()
            .copied()
            .map(|id| GraphObjectRef::new(id, ObjectType::MemoryThread));
        let link_refs = selected_refs
            .iter()
            .copied()
            .chain(thread_refs)
            .collect::<Vec<_>>();
        let objects = self.hydrate_objects_by_refs(&selected_refs).await?;
        let links = self.hydrate_links_touching(&link_refs).await?;
        Ok(derived_memories_by_thread(
            query,
            objects.into_iter().filter(
                |object| matches!(object, MemoryObject::DerivedMemory(memory) if selected_ids.contains(&memory.id)),
            ),
            links,
        ))
    }

    async fn expand_bounded(
        &self,
        query: &GraphExpansionQuery,
    ) -> Result<GraphExpansion, CustomError> {
        let root_ref = GraphObjectRef::new(query.root_id, query.root_type);
        let root_refs = self
            .query_object_refs(&GraphObjectQuery::by_refs(vec![root_ref]))
            .await?;
        if root_refs.is_empty() {
            return Err(CustomError::GraphExpansionRootNotFound {
                object_type: query.root_type,
                object_id: query.root_id,
            });
        }

        let visibility = self.bounded_graph_visible_refs(root_ref, query).await?;
        let object_refs = visibility.object_refs.iter().copied().collect::<Vec<_>>();
        let objects = self.hydrate_objects_by_refs(&object_refs).await?;
        let link_ids = visibility
            .traversal_link_ids
            .union(&visibility.lifecycle_link_ids)
            .copied()
            .collect::<HashSet<_>>();
        let links = self
            .hydrate_links_by_ids(&link_ids)
            .await?
            .into_iter()
            .filter(|link| {
                let endpoints_in_graph = visibility
                    .object_refs
                    .contains(&GraphObjectRef::new(link.from_id, link.from_type))
                    && visibility
                        .object_refs
                        .contains(&GraphObjectRef::new(link.to_id, link.to_type));
                (visibility.traversal_link_ids.contains(&link.id) && endpoints_in_graph)
                    || visibility.lifecycle_link_ids.contains(&link.id)
            })
            .collect::<Vec<_>>();

        let mut expansion = bounded_expansion(query, objects, links)?;
        if expansion.bounded_failure.is_none() {
            expansion.bounded_failure = visibility.bounded_failure;
        }
        Ok(expansion)
    }

    async fn list_diagnostic_objects(&self) -> Result<Vec<MemoryObject>, CustomError> {
        let object_types = [
            ObjectType::Episode,
            ObjectType::Observation,
            ObjectType::Entity,
            ObjectType::MemoryThread,
            ObjectType::DerivedMemory,
        ];
        let object_refs = self
            .query_object_refs(&GraphObjectQuery::by_types(object_types.to_vec(), None))
            .await?;
        self.hydrate_objects_by_refs(&object_refs).await
    }

    async fn list_diagnostic_links(&self) -> Result<Vec<MemoryLink>, CustomError> {
        let link_ids = self
            .query_memory_ids(link_ids_query(), None)
            .await?
            .into_iter()
            .collect::<HashSet<_>>();
        self.hydrate_links_by_ids(&link_ids).await
    }
}

#[derive(Debug, Default)]
struct RdfSubjectValues {
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

fn hydrate_objects_by_refs_from_store(
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

fn hydrate_all_links_from_store(store: &Store) -> Result<Vec<MemoryLink>, CustomError> {
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

fn hydrate_links_by_id_sets_from_store(
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

fn rdf_subject_values(store: &Store) -> Result<HashMap<String, RdfSubjectValues>, CustomError> {
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

fn memory_object_from_rdf(
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

fn memory_link_from_rdf(
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

fn memory_id_literal(
    subject: &str,
    values: &RdfSubjectValues,
    predicate: &'static str,
) -> Result<MemoryId, CustomError> {
    values
        .literal(subject, predicate)?
        .parse()
        .map_err(|error| rdf_parse_error(subject, predicate, error))
}

fn memory_id_from_resource(value: &str) -> Result<MemoryId, CustomError> {
    value
        .rsplit(':')
        .next()
        .ok_or_else(|| CustomError::DatabaseError(format!("Invalid graph URI resource: {value}")))?
        .parse()
        .map_err(|error| CustomError::DatabaseError(format!("Invalid graph URI MemoryId: {error}")))
}

fn memory_ids_from_resources(values: Vec<String>) -> Result<Vec<MemoryId>, CustomError> {
    let mut ids = values
        .iter()
        .map(|value| memory_id_from_resource(value))
        .collect::<Result<Vec<_>, _>>()?;
    ids.sort();
    Ok(ids)
}

fn enum_literal<T: DeserializeOwned>(
    subject: &str,
    values: &RdfSubjectValues,
    predicate: &'static str,
) -> Result<T, CustomError> {
    enum_value_from_literal(&values.literal(subject, predicate)?)
        .map_err(|error| rdf_parse_error(subject, predicate, error))
}

fn enum_value_from_literal<T: DeserializeOwned>(value: &str) -> Result<T, serde_json::Error> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
}

fn f32_literal(
    subject: &str,
    values: &RdfSubjectValues,
    predicate: &'static str,
) -> Result<f32, CustomError> {
    values
        .literal(subject, predicate)?
        .parse()
        .map_err(|error| rdf_parse_error(subject, predicate, error))
}

fn bool_literal(
    subject: &str,
    values: &RdfSubjectValues,
    predicate: &'static str,
) -> Result<bool, CustomError> {
    values
        .literal(subject, predicate)?
        .parse()
        .map_err(|error| rdf_parse_error(subject, predicate, error))
}

fn timestamp_literal(
    subject: &str,
    values: &RdfSubjectValues,
    predicate: &'static str,
) -> Result<DateTime<Utc>, CustomError> {
    parse_timestamp(subject, predicate, &values.literal(subject, predicate)?)
}

fn optional_timestamp_literal(
    values: &RdfSubjectValues,
    predicate: &'static str,
) -> Result<Option<DateTime<Utc>>, CustomError> {
    values
        .optional_literal(predicate)
        .map(|value| parse_timestamp("<optional>", predicate, &value))
        .transpose()
}

fn parse_timestamp(
    subject: &str,
    predicate: &'static str,
    value: &str,
) -> Result<DateTime<Utc>, CustomError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| rdf_parse_error(subject, predicate, error))
}

fn missing_rdf_value(subject: &str, predicate: &'static str) -> CustomError {
    CustomError::DatabaseError(format!(
        "Oxigraph RDF object {subject} is missing required predicate {predicate}"
    ))
}

fn rdf_parse_error(
    subject: &str,
    predicate: &'static str,
    error: impl std::fmt::Display,
) -> CustomError {
    CustomError::DatabaseError(format!(
        "Oxigraph RDF object {subject} has invalid predicate {predicate}: {error}"
    ))
}

#[derive(Debug, Default)]
struct BoundedGraphVisibility {
    object_refs: HashSet<GraphObjectRef>,
    traversal_link_ids: HashSet<MemoryId>,
    lifecycle_link_ids: HashSet<MemoryId>,
    bounded_failure: Option<GraphExpansionBoundedFailure>,
}

fn bounded_graph_visible_refs(
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
        let mut next_frontier = Vec::new();
        for object_ref in &frontier {
            for link_ref in bounded_incident_link_refs(
                query,
                root_ref,
                *object_ref,
                depth,
                &link_refs,
                &mut bounded_failure,
            )? {
                graph_link_ids.insert(link_ref.link_id());
                let neighbor = link_ref.other_endpoint(*object_ref);
                if graph_refs.insert(neighbor) {
                    next_frontier.push(neighbor);
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

fn graph_object_ref(object: &MemoryObject) -> GraphObjectRef {
    let (object_id, object_type) = object_identity(object);
    GraphObjectRef::new(object_id, object_type)
}

fn quad_for_triple(owner_graph_uri: &str, triple: &RdfTriple) -> Result<Quad, CustomError> {
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

fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, CustomError> {
    mutex.lock().map_err(|error| {
        CustomError::DatabaseError(format!("Oxigraph graph store lock poisoned: {error}"))
    })
}

fn oxigraph_error(error: impl std::fmt::Display) -> CustomError {
    CustomError::DatabaseError(format!("Oxigraph graph store error: {error}"))
}

fn oxigraph_http_error(error: impl std::fmt::Display) -> CustomError {
    CustomError::DatabaseError(format!("Oxigraph service error: {error}"))
}

fn object_identity(object: &MemoryObject) -> (MemoryId, ObjectType) {
    match object {
        MemoryObject::Episode(object) => (object.id, object.object_type),
        MemoryObject::Observation(object) => (object.id, object.object_type),
        MemoryObject::Entity(object) => (object.id, object.object_type),
        MemoryObject::MemoryThread(object) => (object.id, object.object_type),
        MemoryObject::DerivedMemory(object) => (object.id, object.object_type),
        MemoryObject::MemoryLink(object) => (object.id, object.object_type),
    }
}

fn sort_objects(objects: &mut [MemoryObject]) {
    objects.sort_by(|left, right| {
        stable_node_key(object_identity(left)).cmp(&stable_node_key(object_identity(right)))
    });
}

fn stable_node_key(node: (MemoryId, ObjectType)) -> (MemoryId, u8) {
    (node.0, object_type_rank(node.1))
}

fn object_type_rank(object_type: ObjectType) -> u8 {
    match object_type {
        ObjectType::Episode => 0,
        ObjectType::Observation => 1,
        ObjectType::Entity => 2,
        ObjectType::MemoryThread => 3,
        ObjectType::DerivedMemory => 4,
        ObjectType::MemoryLink => 5,
    }
}

fn relation_type_rank(relation_type: RelationType) -> u8 {
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

#[cfg(test)]
mod tests {
    use super::super::vocabulary as vocab;
    use super::*;
    use crate::api::types::{
        graph_uri, ContextPackSection, LifecycleFilterAction, LifecycleFilterReason, RelationType,
        RetentionState, RetrievalContext, ThreadStatus,
    };
    use crate::internal::models::vector::{
        memory_object_vector_record, EmbeddingInput, VectorCandidateMatch, VectorCandidateSearch,
        VectorRecordEmbedding, VectorSurface,
    };
    use crate::internal::repositories::test_support::{
        high_fanout_graph_fixture, representative_fixtures,
    };
    use crate::internal::repositories::{
        GraphDerivedMemoryProvenanceQuery, GraphExpansionBoundedFailureReason,
        GraphExpansionFailurePolicy, GraphExpansionFanoutOverride, GraphExpansionFilteredReason,
        GraphExpansionLifecyclePolicy, GraphObjectRef,
    };
    use crate::internal::repositories::{MemoryEmbedder, RetrievePipeline, VectorCandidateStore};
    use std::path::{Path, PathBuf};

    struct TempGraphDir {
        path: PathBuf,
    }

    impl TempGraphDir {
        fn new() -> Self {
            Self {
                path: std::env::temp_dir().join(format!("cmem-oxigraph-{}", MemoryId::new_v4())),
            }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempGraphDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[tokio::test]
    async fn oxigraph_store_upserts_and_queries_canonical_objects() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();

        store.upsert_objects(&fixtures.objects()).await.unwrap();

        let queried = store
            .query_objects(&GraphObjectQuery::by_ids(vec![
                fixtures.episode.id,
                fixtures.correction.id,
            ]))
            .await
            .unwrap();

        assert_eq!(queried.len(), 2);
        assert!(queried.contains(&MemoryObject::Episode(fixtures.episode.clone())));
        assert!(queried.contains(&MemoryObject::DerivedMemory(fixtures.correction.clone())));
        assert!(store.triple_count().unwrap() > 0);
    }

    #[tokio::test]
    async fn oxigraph_upsert_objects_rejects_unsupported_schema_before_mutation() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let mut unsupported = fixtures.salient_observation.clone();
        unsupported.schema_version = "future_schema".to_owned();

        let error = store
            .upsert_objects(&[
                MemoryObject::Episode(fixtures.episode.clone()),
                MemoryObject::Observation(unsupported),
            ])
            .await
            .expect_err("unsupported schema fails");

        assert!(matches!(
            error,
            CustomError::UnsupportedSchemaVersion { .. }
        ));
        assert_eq!(store.triple_count().unwrap(), 0);
        assert!(store
            .query_objects(&GraphObjectQuery::by_ids(vec![fixtures.episode.id]))
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn oxigraph_upsert_links_rejects_unsupported_schema_before_mutation() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        store.upsert_objects(&fixtures.objects()).await.unwrap();
        let baseline_triples = store.triple_count().unwrap();
        let mut unsupported = fixtures.soft_thread_link.clone();
        unsupported.id = MemoryId::new_v4();
        unsupported.schema_version = "future_schema".to_owned();

        let error = store
            .upsert_links(&[fixtures.soft_thread_link.clone(), unsupported])
            .await
            .expect_err("unsupported schema fails");

        assert!(matches!(
            error,
            CustomError::UnsupportedSchemaVersion { .. }
        ));
        assert_eq!(store.triple_count().unwrap(), baseline_triples);
        assert!(store
            .expand_bounded(&GraphExpansionQuery::new(
                fixtures.salient_observation.id,
                ObjectType::Observation,
                1,
                4,
            ))
            .await
            .unwrap()
            .links
            .is_empty());
    }

    #[tokio::test]
    async fn oxigraph_combined_upsert_rejects_unsupported_schema_before_mutation() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let mut unsupported = fixtures.soft_thread_link.clone();
        unsupported.schema_version = "future_schema".to_owned();

        let error = store
            .upsert_objects_and_links(
                &[MemoryObject::Episode(fixtures.episode.clone())],
                &[unsupported],
            )
            .await
            .expect_err("unsupported schema fails");

        assert!(matches!(
            error,
            CustomError::UnsupportedSchemaVersion { .. }
        ));
        assert_eq!(store.triple_count().unwrap(), 0);
        assert!(store
            .query_objects(&GraphObjectQuery::by_ids(vec![fixtures.episode.id]))
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn oxigraph_queries_hydrate_from_rdf() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let episode_graph = graph_uri(ObjectType::Episode, fixtures.episode.id);

        store
            .upsert_objects(&[MemoryObject::Episode(fixtures.episode.clone())])
            .await
            .unwrap();
        assert_eq!(
            store
                .query_objects(&GraphObjectQuery::by_ids(vec![fixtures.episode.id]))
                .await
                .unwrap(),
            vec![MemoryObject::Episode(fixtures.episode.clone())]
        );

        store.replace_triples(episode_graph, &[]).unwrap();

        assert_eq!(
            store
                .query_objects(&GraphObjectQuery::by_ids(vec![fixtures.episode.id]))
                .await
                .unwrap(),
            Vec::<MemoryObject>::new()
        );
    }

    #[tokio::test]
    async fn oxigraph_store_replaces_stale_object_quads_on_repeated_upsert() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let mut updated_episode = fixtures.episode.clone();
        updated_episode.summary =
            "Updated canonical summary replaces RDF materialization.".to_owned();
        let subject = graph_uri(ObjectType::Episode, fixtures.episode.id);
        let stale_summary = RdfTriple {
            subject: subject.clone(),
            predicate: vocab::SUMMARY.to_owned(),
            object: RdfObject::Literal(fixtures.episode.summary.clone()),
        };
        let replacement_summary = RdfTriple {
            subject,
            predicate: vocab::SUMMARY.to_owned(),
            object: RdfObject::Literal(updated_episode.summary.clone()),
        };

        store
            .upsert_objects(&[MemoryObject::Episode(fixtures.episode.clone())])
            .await
            .unwrap();
        assert!(store.contains_triple(&stale_summary).unwrap());

        store
            .upsert_objects(&[MemoryObject::Episode(updated_episode.clone())])
            .await
            .unwrap();

        assert!(!store.contains_triple(&stale_summary).unwrap());
        assert!(store.contains_triple(&replacement_summary).unwrap());
        assert_eq!(
            store.triple_count().unwrap(),
            rdf_triples_for_object(&MemoryObject::Episode(updated_episode.clone()))
                .unwrap()
                .len()
        );
        assert_eq!(
            store
                .query_objects(&GraphObjectQuery::by_ids(vec![updated_episode.id]))
                .await
                .unwrap(),
            vec![MemoryObject::Episode(updated_episode)]
        );
    }

    #[tokio::test]
    async fn oxigraph_upsert_objects_and_links_replaces_rdf_atomically() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let mut updated_memory = fixtures.derived_reflection.clone();
        updated_memory.text = "Updated reflection text replaces stale RDF.".to_owned();
        let mut updated_link = fixtures.soft_thread_link.clone();
        updated_link.to_id = fixtures.episode.id;
        updated_link.to_type = ObjectType::Episode;
        updated_link.relation = RelationType::DerivedFrom;

        let memory_subject = graph_uri(ObjectType::DerivedMemory, fixtures.derived_reflection.id);
        let stale_text = RdfTriple {
            subject: memory_subject.clone(),
            predicate: vocab::TEXT.to_owned(),
            object: RdfObject::Literal(fixtures.derived_reflection.text.clone()),
        };
        let replacement_text = RdfTriple {
            subject: memory_subject,
            predicate: vocab::TEXT.to_owned(),
            object: RdfObject::Literal(updated_memory.text.clone()),
        };
        let stale_relation = RdfTriple {
            subject: graph_uri(
                fixtures.soft_thread_link.from_type,
                fixtures.soft_thread_link.from_id,
            ),
            predicate: vocab::relation_predicate("part_of_thread"),
            object: RdfObject::Resource(graph_uri(
                fixtures.soft_thread_link.to_type,
                fixtures.soft_thread_link.to_id,
            )),
        };
        let replacement_relation = RdfTriple {
            subject: graph_uri(updated_link.from_type, updated_link.from_id),
            predicate: vocab::relation_predicate("derived_from"),
            object: RdfObject::Resource(graph_uri(updated_link.to_type, updated_link.to_id)),
        };

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store
            .upsert_links(std::slice::from_ref(&fixtures.soft_thread_link))
            .await
            .unwrap();
        assert!(store.contains_triple(&stale_text).unwrap());
        assert!(store.contains_triple(&stale_relation).unwrap());

        store
            .upsert_objects_and_links(
                &[MemoryObject::DerivedMemory(updated_memory.clone())],
                &[updated_link.clone()],
            )
            .await
            .unwrap();

        assert!(!store.contains_triple(&stale_text).unwrap());
        assert!(!store.contains_triple(&stale_relation).unwrap());
        assert!(store.contains_triple(&replacement_text).unwrap());
        assert!(store.contains_triple(&replacement_relation).unwrap());
        assert_eq!(
            store
                .query_objects(&GraphObjectQuery::by_ids(vec![updated_memory.id]))
                .await
                .unwrap(),
            vec![MemoryObject::DerivedMemory(updated_memory)]
        );
        assert_eq!(
            store
                .expand_bounded(&GraphExpansionQuery::new(
                    updated_link.from_id,
                    updated_link.from_type,
                    1,
                    4,
                ))
                .await
                .unwrap()
                .links,
            vec![updated_link]
        );
    }

    #[tokio::test]
    async fn oxigraph_link_quads_are_owned_by_memory_link_named_graphs() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let link = fixtures.soft_thread_link.clone();
        let relation_triple = RdfTriple {
            subject: graph_uri(link.from_type, link.from_id),
            predicate: vocab::relation_predicate("part_of_thread"),
            object: RdfObject::Resource(graph_uri(link.to_type, link.to_id)),
        };

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store
            .upsert_links(std::slice::from_ref(&link))
            .await
            .unwrap();

        assert!(store
            .contains_triple_in_graph(
                &relation_triple,
                &graph_uri(ObjectType::MemoryLink, link.id)
            )
            .unwrap());
        assert!(!store
            .contains_triple_in_graph(
                &relation_triple,
                &graph_uri(ObjectType::Observation, fixtures.salient_observation.id)
            )
            .unwrap());
    }

    #[tokio::test]
    async fn oxigraph_store_upserts_links_and_expands_bounded_graph() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store.upsert_links(&fixtures.links()).await.unwrap();

        let expansion = store
            .expand_bounded(&GraphExpansionQuery::new(
                fixtures.hub_entity.id,
                ObjectType::Entity,
                1,
                3,
            ))
            .await
            .unwrap();

        assert!(expansion
            .objects
            .contains(&MemoryObject::Entity(fixtures.hub_entity.clone())));
        assert!(expansion.objects.len() <= 3);
        assert!(expansion
            .links
            .iter()
            .all(|link| link.from_id == fixtures.hub_entity.id
                || link.to_id == fixtures.hub_entity.id));
    }

    #[tokio::test]
    async fn oxigraph_expansion_preserves_depth_node_cap_and_allowed_types() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixture = high_fanout_graph_fixture();

        store.upsert_objects(&fixture.objects()).await.unwrap();
        store.upsert_links(&fixture.links).await.unwrap();

        let depth_zero = store
            .expand_bounded(&GraphExpansionQuery::new(
                fixture.hub_entity.id,
                ObjectType::Entity,
                0,
                20,
            ))
            .await
            .unwrap();
        assert_eq!(
            depth_zero.objects,
            vec![MemoryObject::Entity(fixture.hub_entity.clone())]
        );
        assert!(depth_zero.links.is_empty());

        let query = GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 5)
            .with_allowed_object_types(vec![ObjectType::DerivedMemory]);
        let first = store.expand_bounded(&query).await.unwrap();
        let second = store.expand_bounded(&query).await.unwrap();

        assert_eq!(first, second);
        assert_eq!(first.objects.len(), 5);
        assert_eq!(first.links.len(), 4);
        assert!(first
            .objects
            .contains(&MemoryObject::Entity(fixture.hub_entity.clone())));
        assert!(first.objects.iter().all(|object| matches!(
            object,
            MemoryObject::Entity(_) | MemoryObject::DerivedMemory(_)
        )));

        let expanded_derived_ids = first
            .objects
            .iter()
            .filter_map(|object| match object {
                MemoryObject::DerivedMemory(memory) => Some(memory.id),
                _ => None,
            })
            .collect::<Vec<_>>();
        let expected_derived_ids = fixture
            .derived_memories
            .iter()
            .take(4)
            .map(|memory| memory.id)
            .collect::<Vec<_>>();
        assert_eq!(expanded_derived_ids, expected_derived_ids);
        assert!(first.links.iter().all(|link| {
            link.from_id == fixture.hub_entity.id && link.to_type == ObjectType::DerivedMemory
        }));
    }

    #[tokio::test]
    async fn oxigraph_expansion_returns_canonical_objects_and_links() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixture = high_fanout_graph_fixture();

        store.upsert_objects(&fixture.objects()).await.unwrap();
        store.upsert_links(&fixture.links).await.unwrap();

        let expansion = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 3)
                    .with_allowed_object_types(vec![ObjectType::Episode, ObjectType::Observation]),
            )
            .await
            .unwrap();

        assert_eq!(
            expansion.objects,
            vec![
                MemoryObject::Episode(fixture.episode.clone()),
                MemoryObject::Observation(fixture.observation.clone()),
                MemoryObject::Entity(fixture.hub_entity.clone()),
            ]
        );
        assert_eq!(
            expansion.links,
            vec![fixture.links[0].clone(), fixture.links[1].clone()]
        );
    }

    #[tokio::test]
    async fn oxigraph_expansion_returns_only_traversed_links_after_fanout_pruning() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixture = high_fanout_graph_fixture();
        let traversed_link = fixture.links[0].clone();
        let mut pruned_duplicate = traversed_link.clone();
        pruned_duplicate.id = MemoryId::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0195);
        pruned_duplicate.rationale = Some("Fanout-pruned duplicate endpoint link.".to_owned());
        let mut links = fixture.links.clone();
        links.push(pruned_duplicate.clone());

        store.upsert_objects(&fixture.objects()).await.unwrap();
        store.upsert_links(&links).await.unwrap();

        let expansion = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 20)
                    .with_max_fanout_per_node(1),
            )
            .await
            .unwrap();

        assert_eq!(expansion.links, vec![traversed_link.clone()]);
        assert!(!expansion.links.contains(&pruned_duplicate));
        assert_eq!(
            expansion
                .relations
                .iter()
                .map(|relation| relation.link_id)
                .collect::<Vec<_>>(),
            vec![traversed_link.id]
        );
    }

    #[tokio::test]
    async fn oxigraph_visibility_applies_selectivity_fanout_before_hydration() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixture = high_fanout_graph_fixture();

        store.upsert_objects(&fixture.objects()).await.unwrap();
        store.upsert_links(&fixture.links).await.unwrap();

        let query = GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 20)
            .with_allowed_object_types(vec![ObjectType::DerivedMemory])
            .with_max_fanout_per_node(20)
            .with_fanout_overrides(vec![GraphExpansionFanoutOverride {
                relation: RelationType::About,
                object_type: ObjectType::DerivedMemory,
                max_fanout: 1,
            }]);
        let selectors = SparqlGraphSelectors::new(&store.store);

        let visibility = bounded_graph_visible_refs(
            &selectors,
            GraphObjectRef::new(fixture.hub_entity.id, ObjectType::Entity),
            &query,
        )
        .unwrap();

        assert_eq!(visibility.traversal_link_ids.len(), 1);
        assert_eq!(
            visibility
                .object_refs
                .iter()
                .filter(|object_ref| object_ref.object_type == ObjectType::DerivedMemory)
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn oxigraph_expansion_applies_bounds_after_graph_visibility_filtering() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let graph_visible_link = fixtures.hub_links[1].clone();
        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store
            .upsert_links(std::slice::from_ref(&graph_visible_link))
            .await
            .unwrap();

        let expansion = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixtures.hub_entity.id, ObjectType::Entity, 1, 3)
                    .with_max_fanout_per_node(1),
            )
            .await
            .unwrap();

        assert_eq!(expansion.links, vec![graph_visible_link.clone()]);
        assert!(expansion
            .objects
            .contains(&MemoryObject::Entity(fixtures.hub_entity.clone())));
        assert!(expansion.objects.contains(&MemoryObject::DerivedMemory(
            fixtures.derived_reflection.clone()
        )));
    }

    #[tokio::test]
    async fn oxigraph_expansion_fails_closed_when_visibility_hits_hub_limit() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixture = high_fanout_graph_fixture();

        store.upsert_objects(&fixture.objects()).await.unwrap();
        store.upsert_links(&fixture.links).await.unwrap();

        let error = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 20)
                    .with_max_hub_edges(1)
                    .with_failure_policy(GraphExpansionFailurePolicy {
                        timeout_ms: Some(250),
                        allow_partial_results: false,
                    }),
            )
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            CustomError::GraphExpansionBounded { reason, .. } if reason == "hub_limit"
        ));
    }

    #[tokio::test]
    async fn oxigraph_expansion_uses_targeted_supersession_evidence_outside_frontier() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let superseded_memory = fixtures.user_preference.clone();
        let replacement = fixtures.correction.clone();
        let hub_link = crate::api::types::MemoryLink {
            id: MemoryId::from_u128(0x550e_8400_e29b_41d4_a716_4466_5500_0002),
            object_type: ObjectType::MemoryLink,
            from_id: fixtures.hub_entity.id,
            from_type: ObjectType::Entity,
            to_id: superseded_memory.id,
            to_type: ObjectType::DerivedMemory,
            relation: RelationType::About,
            confidence: 1.0,
            rationale: Some("Hub reaches a superseded memory.".to_owned()),
            created_at: superseded_memory.created_at,
            schema_version: superseded_memory.schema_version.clone(),
        };
        let supersedes_link = crate::api::types::MemoryLink {
            id: MemoryId::from_u128(0x550e_8400_e29b_41d4_a716_4466_5500_0003),
            object_type: ObjectType::MemoryLink,
            from_id: replacement.id,
            from_type: ObjectType::DerivedMemory,
            to_id: superseded_memory.id,
            to_type: ObjectType::DerivedMemory,
            relation: RelationType::Supersedes,
            confidence: 1.0,
            rationale: Some("Replacement supersedes candidate memory.".to_owned()),
            created_at: replacement.created_at,
            schema_version: replacement.schema_version.clone(),
        };

        store
            .upsert_objects(&[
                MemoryObject::Entity(fixtures.hub_entity.clone()),
                MemoryObject::DerivedMemory(superseded_memory.clone()),
                MemoryObject::DerivedMemory(replacement.clone()),
            ])
            .await
            .unwrap();
        store
            .upsert_links(&[hub_link.clone(), supersedes_link])
            .await
            .unwrap();

        let depth_zero_root = store
            .expand_bounded(&GraphExpansionQuery::new(
                superseded_memory.id,
                ObjectType::DerivedMemory,
                0,
                3,
            ))
            .await
            .unwrap();
        assert!(depth_zero_root.objects.is_empty());
        assert!(depth_zero_root.filtered_nodes.iter().any(|filtered| {
            filtered.object_ref
                == GraphObjectRef::new(superseded_memory.id, ObjectType::DerivedMemory)
                && filtered.reason == GraphExpansionFilteredReason::Superseded
        }));

        let depth_one_neighbor = store
            .expand_bounded(&GraphExpansionQuery::new(
                fixtures.hub_entity.id,
                ObjectType::Entity,
                1,
                3,
            ))
            .await
            .unwrap();
        assert_eq!(
            depth_one_neighbor.objects,
            vec![MemoryObject::Entity(fixtures.hub_entity.clone())]
        );
        assert!(depth_one_neighbor.links.is_empty());
        assert!(depth_one_neighbor.filtered_nodes.iter().any(|filtered| {
            filtered.object_ref
                == GraphObjectRef::new(superseded_memory.id, ObjectType::DerivedMemory)
                && filtered.reason == GraphExpansionFilteredReason::Superseded
        }));

        let historical_neighbor = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixtures.hub_entity.id, ObjectType::Entity, 1, 3)
                    .with_lifecycle_policy(GraphExpansionLifecyclePolicy {
                        include_superseded: true,
                        ..GraphExpansionLifecyclePolicy::default()
                    }),
            )
            .await
            .unwrap();
        assert!(historical_neighbor
            .objects
            .contains(&MemoryObject::DerivedMemory(superseded_memory)));
        assert_eq!(historical_neighbor.links, vec![hub_link]);
    }

    #[tokio::test]
    async fn oxigraph_expansion_honors_policy_bounds_and_lifecycle_filters() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store.upsert_links(&fixtures.links()).await.unwrap();

        let default_expansion = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixtures.correction.id, ObjectType::DerivedMemory, 1, 5)
                    .with_allowed_relation_types(vec![RelationType::Supersedes]),
            )
            .await
            .unwrap();
        assert_eq!(
            default_expansion.objects,
            vec![MemoryObject::DerivedMemory(fixtures.correction.clone())]
        );
        assert_eq!(default_expansion.filtered_nodes.len(), 1);
        assert_eq!(
            default_expansion.filtered_nodes[0].object_ref,
            GraphObjectRef::new(fixtures.suppressed_seed.id, ObjectType::DerivedMemory)
        );
        assert_eq!(
            default_expansion.filtered_nodes[0].reason,
            GraphExpansionFilteredReason::Suppressed
        );

        let historical_expansion = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixtures.correction.id, ObjectType::DerivedMemory, 1, 5)
                    .with_allowed_relation_types(vec![RelationType::Supersedes])
                    .with_lifecycle_policy(GraphExpansionLifecyclePolicy {
                        include_suppressed: true,
                        include_non_current: true,
                        include_superseded: true,
                        ..GraphExpansionLifecyclePolicy::default()
                    }),
            )
            .await
            .unwrap();
        assert_eq!(
            historical_expansion.links,
            vec![fixtures.hub_links[2].clone()]
        );
        assert_eq!(historical_expansion.relations.len(), 1);

        let timed_out = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixtures.hub_entity.id, ObjectType::Entity, 1, 5)
                    .with_failure_policy(GraphExpansionFailurePolicy {
                        timeout_ms: Some(0),
                        allow_partial_results: true,
                    }),
            )
            .await
            .unwrap();
        assert_eq!(
            timed_out.bounded_failure.unwrap().reason,
            GraphExpansionBoundedFailureReason::Timeout
        );
    }

    #[tokio::test]
    async fn oxigraph_lifecycle_upserts_support_supersession_and_provenance_discovery() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let superseded_memory = fixtures.user_preference.clone();
        let mut non_current_memory = fixtures.open_loop.clone();
        let mut replacement = fixtures.correction.clone();
        let mut archived_thread = fixtures.soft_thread.clone();
        non_current_memory.is_current = false;
        replacement.supersedes = vec![superseded_memory.id];
        archived_thread.status = ThreadStatus::Archived;
        let supersedes_link = crate::api::types::MemoryLink {
            id: MemoryId::from_u128(0x550e_8400_e29b_41d4_a716_4466_5600_0001),
            object_type: ObjectType::MemoryLink,
            from_id: replacement.id,
            from_type: ObjectType::DerivedMemory,
            to_id: superseded_memory.id,
            to_type: ObjectType::DerivedMemory,
            relation: RelationType::Supersedes,
            confidence: 1.0,
            rationale: Some("Replacement supersedes historical derived memory.".to_owned()),
            created_at: replacement.created_at,
            schema_version: replacement.schema_version.clone(),
        };

        store
            .upsert_objects(&[
                MemoryObject::Episode(fixtures.episode.clone()),
                MemoryObject::Observation(fixtures.salient_observation.clone()),
                MemoryObject::MemoryThread(archived_thread.clone()),
                MemoryObject::DerivedMemory(fixtures.derived_reflection.clone()),
                MemoryObject::DerivedMemory(superseded_memory.clone()),
                MemoryObject::DerivedMemory(non_current_memory.clone()),
                MemoryObject::DerivedMemory(replacement.clone()),
            ])
            .await
            .unwrap();
        store
            .upsert_links(&[fixtures.soft_thread_link.clone(), supersedes_link.clone()])
            .await
            .unwrap();

        let default_matches = store
            .query_derived_memories_by_provenance(
                &GraphDerivedMemoryProvenanceQuery::by_sources(
                    vec![fixtures.episode.id],
                    vec![fixtures.salient_observation.id],
                )
                .with_limit(10),
            )
            .await
            .unwrap();
        assert!(default_matches
            .iter()
            .any(|memory| memory.id == fixtures.derived_reflection.id));
        assert!(default_matches
            .iter()
            .any(|memory| memory.id == replacement.id));
        assert!(!default_matches
            .iter()
            .any(|memory| memory.id == superseded_memory.id));
        assert!(!default_matches
            .iter()
            .any(|memory| memory.id == non_current_memory.id));

        let historical_matches = store
            .query_derived_memories_by_provenance(
                &GraphDerivedMemoryProvenanceQuery::by_sources(
                    vec![fixtures.episode.id],
                    vec![fixtures.salient_observation.id],
                )
                .with_lifecycle_policy(GraphExpansionLifecyclePolicy {
                    include_non_current: true,
                    include_superseded: true,
                    ..GraphExpansionLifecyclePolicy::default()
                }),
            )
            .await
            .unwrap();
        assert!(historical_matches
            .iter()
            .any(|memory| memory.id == superseded_memory.id));
        assert!(historical_matches
            .iter()
            .any(|memory| memory.id == non_current_memory.id));

        let default_expansion = store
            .expand_bounded(
                &GraphExpansionQuery::new(replacement.id, ObjectType::DerivedMemory, 1, 5)
                    .with_allowed_relation_types(vec![RelationType::Supersedes]),
            )
            .await
            .unwrap();
        assert_eq!(
            default_expansion.objects,
            vec![MemoryObject::DerivedMemory(replacement.clone())]
        );
        assert!(default_expansion.links.is_empty());
        assert!(default_expansion.filtered_nodes.iter().any(|filtered| {
            filtered.object_ref
                == GraphObjectRef::new(superseded_memory.id, ObjectType::DerivedMemory)
                && filtered.reason == GraphExpansionFilteredReason::Superseded
        }));

        let historical_expansion = store
            .expand_bounded(
                &GraphExpansionQuery::new(replacement.id, ObjectType::DerivedMemory, 1, 5)
                    .with_allowed_relation_types(vec![RelationType::Supersedes])
                    .with_lifecycle_policy(GraphExpansionLifecyclePolicy {
                        include_superseded: true,
                        ..GraphExpansionLifecyclePolicy::default()
                    }),
            )
            .await
            .unwrap();
        assert!(historical_expansion
            .objects
            .contains(&MemoryObject::DerivedMemory(superseded_memory.clone())));
        assert_eq!(historical_expansion.links, vec![supersedes_link]);
        assert_eq!(replacement.supersedes, vec![superseded_memory.id]);

        let thread_expansion = store
            .expand_bounded(
                &GraphExpansionQuery::new(
                    fixtures.salient_observation.id,
                    ObjectType::Observation,
                    1,
                    5,
                )
                .with_allowed_relation_types(vec![RelationType::PartOfThread]),
            )
            .await
            .unwrap();
        assert!(thread_expansion.filtered_nodes.iter().any(|filtered| {
            filtered.object_ref == GraphObjectRef::new(archived_thread.id, ObjectType::MemoryThread)
                && filtered.reason == GraphExpansionFilteredReason::Archived
        }));

        let default_thread_matches = store
            .query_derived_memories_by_thread(
                &GraphDerivedMemoryThreadQuery::by_threads(vec![fixtures.soft_thread.id])
                    .with_limit(10),
            )
            .await
            .unwrap();
        assert!(!default_thread_matches
            .iter()
            .any(|memory| memory.id == superseded_memory.id));
        assert!(!default_thread_matches
            .iter()
            .any(|memory| memory.id == non_current_memory.id));
    }

    #[tokio::test]
    async fn oxigraph_retrieval_after_lifecycle_mutation_excludes_stale_records_by_default() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let mut superseded_memory = fixtures.user_preference.clone();
        let mut suppressed_memory = fixtures.suppressed_seed.clone();
        let mut non_current_memory = fixtures.open_loop.clone();
        let mut replacement = fixtures.correction.clone();
        let mut archived_thread = fixtures.soft_thread.clone();
        superseded_memory.retention_state = RetentionState::Active;
        superseded_memory.is_current = true;
        suppressed_memory.retention_state = RetentionState::Suppressed;
        non_current_memory.is_current = false;
        replacement.supersedes = vec![superseded_memory.id];
        archived_thread.status = ThreadStatus::Archived;
        let supersedes_link = crate::api::types::MemoryLink {
            id: MemoryId::from_u128(0x550e_8400_e29b_41d4_a716_4466_5600_0002),
            object_type: ObjectType::MemoryLink,
            from_id: replacement.id,
            from_type: ObjectType::DerivedMemory,
            to_id: superseded_memory.id,
            to_type: ObjectType::DerivedMemory,
            relation: RelationType::Supersedes,
            confidence: 1.0,
            rationale: Some("Replacement supersedes stale retrieval candidate.".to_owned()),
            created_at: replacement.created_at,
            schema_version: replacement.schema_version.clone(),
        };
        store
            .upsert_objects(&[
                MemoryObject::Episode(fixtures.episode.clone()),
                MemoryObject::Observation(fixtures.salient_observation.clone()),
                MemoryObject::MemoryThread(archived_thread.clone()),
                MemoryObject::DerivedMemory(superseded_memory.clone()),
                MemoryObject::DerivedMemory(suppressed_memory.clone()),
                MemoryObject::DerivedMemory(non_current_memory.clone()),
                MemoryObject::DerivedMemory(replacement.clone()),
            ])
            .await
            .unwrap();
        store
            .upsert_links(&[fixtures.soft_thread_link.clone(), supersedes_link])
            .await
            .unwrap();

        let vector = FixedVectorStore::new(vec![
            VectorCandidateMatch::new(
                replacement.id,
                ObjectType::DerivedMemory,
                VectorSurface::DerivedText,
                0.99,
            ),
            VectorCandidateMatch::new(
                superseded_memory.id,
                ObjectType::DerivedMemory,
                VectorSurface::DerivedText,
                0.98,
            ),
            VectorCandidateMatch::new(
                suppressed_memory.id,
                ObjectType::DerivedMemory,
                VectorSurface::DerivedText,
                0.97,
            ),
            VectorCandidateMatch::new(
                non_current_memory.id,
                ObjectType::DerivedMemory,
                VectorSurface::DerivedText,
                0.96,
            ),
            VectorCandidateMatch::new(
                archived_thread.id,
                ObjectType::MemoryThread,
                VectorSurface::Summary,
                0.95,
            ),
        ]);
        let embedder = FixedEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&store, &vector, &embedder);
        let outcome = pipeline
            .retrieve(RetrievalContext::new("lifecycle graph truth").with_trace())
            .await
            .unwrap();
        let trace = outcome.trace.as_ref().unwrap();

        assert!(outcome
            .pack
            .derived_memories
            .iter()
            .any(|included| included.memory.id == replacement.id));
        assert!(!outcome
            .pack
            .preferences
            .iter()
            .any(|included| included.memory.id == superseded_memory.id));
        assert!(!outcome
            .pack
            .preferences
            .iter()
            .any(|included| included.memory.id == suppressed_memory.id));
        assert!(!outcome
            .pack
            .open_loops
            .iter()
            .any(|included| included.memory.id == non_current_memory.id));
        assert!(outcome.pack.active_threads.is_empty());
        assert!(trace.lifecycle_filter_decisions.iter().any(|decision| {
            decision.object.id == superseded_memory.id
                && decision.reason == LifecycleFilterReason::SupersededOmitted
        }));
        assert!(trace.lifecycle_filter_decisions.iter().any(|decision| {
            decision.object.id == suppressed_memory.id
                && decision.reason == LifecycleFilterReason::SuppressedOmitted
        }));
        assert!(trace.lifecycle_filter_decisions.iter().any(|decision| {
            decision.object.id == non_current_memory.id
                && decision.reason == LifecycleFilterReason::NonCurrentOmitted
        }));
        assert!(trace.lifecycle_filter_decisions.iter().any(|decision| {
            decision.object.id == archived_thread.id
                && decision.reason == LifecycleFilterReason::ArchivedOmitted
        }));
    }

    #[tokio::test]
    async fn oxigraph_expansion_maps_unsupported_or_missing_roots_to_custom_error() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixture = high_fanout_graph_fixture();

        store.upsert_objects(&fixture.objects()).await.unwrap();
        store.upsert_links(&fixture.links).await.unwrap();

        let unsupported = store
            .expand_bounded(&GraphExpansionQuery::new(
                fixture.links[0].id,
                ObjectType::MemoryLink,
                1,
                2,
            ))
            .await
            .unwrap_err();
        assert!(matches!(unsupported, CustomError::MemoryValidation(_)));
        assert!(unsupported
            .to_string()
            .contains("does not support MemoryLink roots"));

        let missing_root = store
            .expand_bounded(&GraphExpansionQuery::new(
                MemoryId::new_v4(),
                ObjectType::Entity,
                1,
                2,
            ))
            .await
            .unwrap_err();
        assert!(matches!(
            missing_root,
            CustomError::GraphExpansionRootNotFound { .. }
        ));
        assert!(missing_root.to_string().contains("root not found"));
    }

    #[tokio::test]
    async fn oxigraph_store_replaces_stale_link_quads_and_owned_relation_triples() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let mut updated_link = fixtures.soft_thread_link.clone();
        updated_link.from_id = fixtures.derived_reflection.id;
        updated_link.from_type = ObjectType::DerivedMemory;
        updated_link.to_id = fixtures.episode.id;
        updated_link.to_type = ObjectType::Episode;
        updated_link.relation = RelationType::DerivedFrom;
        updated_link.rationale = Some("Updated link replaces stale RDF relation.".to_owned());

        let link_subject = graph_uri(ObjectType::MemoryLink, fixtures.soft_thread_link.id);
        let stale_from = graph_uri(
            fixtures.soft_thread_link.from_type,
            fixtures.soft_thread_link.from_id,
        );
        let stale_to = graph_uri(
            fixtures.soft_thread_link.to_type,
            fixtures.soft_thread_link.to_id,
        );
        let replacement_from = graph_uri(updated_link.from_type, updated_link.from_id);
        let replacement_to = graph_uri(updated_link.to_type, updated_link.to_id);
        let stale_relation_literal = RdfTriple {
            subject: link_subject.clone(),
            predicate: vocab::RELATION.to_owned(),
            object: RdfObject::Literal("part_of_thread".to_owned()),
        };
        let replacement_relation_literal = RdfTriple {
            subject: link_subject,
            predicate: vocab::RELATION.to_owned(),
            object: RdfObject::Literal("derived_from".to_owned()),
        };
        let stale_relation = RdfTriple {
            subject: stale_from,
            predicate: vocab::relation_predicate("part_of_thread"),
            object: RdfObject::Resource(stale_to),
        };
        let replacement_relation = RdfTriple {
            subject: replacement_from,
            predicate: vocab::relation_predicate("derived_from"),
            object: RdfObject::Resource(replacement_to),
        };

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store
            .upsert_links(std::slice::from_ref(&fixtures.soft_thread_link))
            .await
            .unwrap();
        assert!(store.contains_triple(&stale_relation_literal).unwrap());
        assert!(store.contains_triple(&stale_relation).unwrap());

        store.upsert_links(&[updated_link.clone()]).await.unwrap();

        assert!(!store.contains_triple(&stale_relation_literal).unwrap());
        assert!(!store.contains_triple(&stale_relation).unwrap());
        assert!(store
            .contains_triple(&replacement_relation_literal)
            .unwrap());
        assert!(store.contains_triple(&replacement_relation).unwrap());

        let expansion = store
            .expand_bounded(&GraphExpansionQuery::new(
                fixtures.derived_reflection.id,
                ObjectType::DerivedMemory,
                1,
                4,
            ))
            .await
            .unwrap();
        assert!(expansion.links.contains(&updated_link));
        assert!(!expansion.links.contains(&fixtures.soft_thread_link));
    }

    #[tokio::test]
    async fn oxigraph_store_keeps_duplicate_direct_relation_quads_owned_by_other_links() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let mut duplicate_link = fixtures.soft_thread_link.clone();
        duplicate_link.id = MemoryId::new_v4();
        let mut updated_link = fixtures.soft_thread_link.clone();
        updated_link.from_id = fixtures.derived_reflection.id;
        updated_link.from_type = ObjectType::DerivedMemory;
        updated_link.to_id = fixtures.episode.id;
        updated_link.to_type = ObjectType::Episode;
        updated_link.relation = RelationType::DerivedFrom;

        let stale_relation = RdfTriple {
            subject: graph_uri(
                fixtures.soft_thread_link.from_type,
                fixtures.soft_thread_link.from_id,
            ),
            predicate: vocab::relation_predicate("part_of_thread"),
            object: RdfObject::Resource(graph_uri(
                fixtures.soft_thread_link.to_type,
                fixtures.soft_thread_link.to_id,
            )),
        };

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store
            .upsert_links(&[fixtures.soft_thread_link.clone(), duplicate_link.clone()])
            .await
            .unwrap();
        assert_eq!(store.matching_triple_count(&stale_relation).unwrap(), 2);

        store.upsert_links(&[updated_link]).await.unwrap();

        assert_eq!(store.matching_triple_count(&stale_relation).unwrap(), 1);
        let expansion = store
            .expand_bounded(&GraphExpansionQuery::new(
                duplicate_link.from_id,
                duplicate_link.from_type,
                1,
                4,
            ))
            .await
            .unwrap();
        assert!(expansion.links.contains(&duplicate_link));
    }

    #[tokio::test]
    async fn embedded_in_memory_oxigraph_smoke_has_no_external_prerequisite() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();

        store
            .upsert_objects(&[
                MemoryObject::Episode(fixtures.episode.clone()),
                MemoryObject::Observation(fixtures.salient_observation.clone()),
            ])
            .await
            .unwrap();
        store
            .upsert_links(std::slice::from_ref(&fixtures.soft_thread_link))
            .await
            .unwrap();

        let expansion = store
            .expand_bounded(&GraphExpansionQuery::new(
                fixtures.salient_observation.id,
                ObjectType::Observation,
                1,
                2,
            ))
            .await
            .unwrap();

        assert!(store.triple_count().unwrap() > 0);
        assert!(expansion
            .objects
            .contains(&MemoryObject::Observation(fixtures.salient_observation)));
    }

    #[test]
    fn http_service_queries_do_not_use_whole_dataset_snapshot_shape() {
        let fixtures = representative_fixtures();
        let object_ref = GraphObjectRef::new(fixtures.episode.id, ObjectType::Episode);
        let source = graph_uri(ObjectType::Episode, fixtures.episode.id);
        let graph = graph_uri(ObjectType::Episode, fixtures.episode.id);
        let queries = vec![
            object_refs_query(&GraphObjectQuery::by_refs(vec![object_ref])),
            derived_memory_ids_by_provenance_query(std::slice::from_ref(&source)),
            derived_memory_ids_by_resource_predicate_query(
                vocab::PART_OF_THREAD,
                &[graph_uri(ObjectType::MemoryThread, fixtures.soft_thread.id)],
            ),
            links_touching_query(&[object_ref]),
            link_ids_query(),
            named_graph_quads_query(&[graph]),
        ];

        let forbidden_snapshot_query =
            ["SELECT ?g ?s ?p ?o WHERE", "{ GRAPH ?g", "{ ?s ?p ?o } }"].join(" ");
        for query in queries {
            assert!(
                !query.contains(&forbidden_snapshot_query),
                "query unexpectedly used the full snapshot shape: {query}"
            );
        }
    }

    #[test]
    fn http_service_named_graph_hydration_is_scoped_by_values() {
        let fixtures = representative_fixtures();
        let graph = graph_uri(ObjectType::DerivedMemory, fixtures.derived_reflection.id);
        let query = named_graph_quads_query(std::slice::from_ref(&graph));

        assert!(query.contains("VALUES ?g"));
        assert!(query.contains(&format!("<{graph}>")));
        assert!(query.contains("GRAPH ?g"));
    }

    #[test]
    fn http_service_object_ref_query_is_scoped_by_requested_refs() {
        let fixtures = representative_fixtures();
        let query = object_refs_query(&GraphObjectQuery::by_refs(vec![GraphObjectRef::new(
            fixtures.episode.id,
            ObjectType::Episode,
        )]));

        assert!(query.contains("VALUES (?id ?objectType)"));
        assert!(query.contains(&fixtures.episode.id.to_string()));
        assert!(query.contains("episode"));
    }

    #[tokio::test]
    #[ignore = "requires local test Oxigraph: docker compose -f docker-compose.oxigraph.test.yml up -d and OXIGRAPH_TEST_CONNECTION_STRING"]
    async fn oxigraph_http_service_live_smoke_upserts_queries_and_filters(
    ) -> Result<(), CustomError> {
        let endpoint = std::env::var("OXIGRAPH_TEST_CONNECTION_STRING")
            .unwrap_or_else(|_| "http://localhost:7879".to_owned());
        ensure_live_smoke_test_endpoint(&endpoint)?;
        let store = OxigraphHttpGraphAuthorityStore::new(endpoint)?;
        let fixtures = representative_fixtures();
        let smoke_graph_uris = fixtures
            .objects()
            .into_iter()
            .map(|object| {
                let (id, object_type) = object_identity(&object);
                graph_uri(object_type, id)
            })
            .chain(
                fixtures
                    .links()
                    .into_iter()
                    .map(|link| graph_uri(ObjectType::MemoryLink, link.id)),
            )
            .collect::<Vec<_>>();

        let test_result: Result<(), CustomError> = async {
            store.upsert_objects(&fixtures.objects()).await?;
            store.upsert_links(&fixtures.links()).await?;

            let queried = store
                .query_objects(&GraphObjectQuery::by_refs(vec![
                    GraphObjectRef::new(fixtures.episode.id, ObjectType::Episode),
                    GraphObjectRef::new(fixtures.derived_reflection.id, ObjectType::DerivedMemory),
                ]))
                .await?;
            if !queried.contains(&MemoryObject::Episode(fixtures.episode.clone())) {
                return Err(CustomError::DatabaseError(
                    "Oxigraph smoke did not return the stored episode".to_owned(),
                ));
            }
            if !queried.contains(&MemoryObject::DerivedMemory(
                fixtures.derived_reflection.clone(),
            )) {
                return Err(CustomError::DatabaseError(
                    "Oxigraph smoke did not return the stored derived memory".to_owned(),
                ));
            }

            let vector = FixedVectorStore::new(vec![
                VectorCandidateMatch::new(
                    fixtures.derived_reflection.id,
                    ObjectType::DerivedMemory,
                    VectorSurface::Summary,
                    0.99,
                ),
                VectorCandidateMatch::new(
                    fixtures.suppressed_seed.id,
                    ObjectType::DerivedMemory,
                    VectorSurface::Summary,
                    0.98,
                ),
            ]);
            let embedder = FixedEmbedder::new(vec![1.0, 0.0]);
            let pipeline = RetrievePipeline::new(&store, &vector, &embedder);

            let outcome = pipeline
                .retrieve(RetrievalContext::new("service graph authority"))
                .await?;
            if !outcome
                .pack
                .derived_memories
                .iter()
                .any(|included| included.memory.id == fixtures.derived_reflection.id)
            {
                return Err(CustomError::DatabaseError(
                    "Oxigraph smoke retrieval did not include the current derived memory"
                        .to_owned(),
                ));
            }
            if outcome
                .pack
                .derived_memories
                .iter()
                .chain(outcome.pack.preferences.iter())
                .any(|included| included.memory.id == fixtures.suppressed_seed.id)
            {
                return Err(CustomError::DatabaseError(
                    "Oxigraph smoke retrieval included a suppressed derived memory".to_owned(),
                ));
            }
            Ok(())
        }
        .await;

        let cleanup_result = store.delete_named_graphs(&smoke_graph_uris).await;
        let remaining_result = store.named_graph_quad_count(&smoke_graph_uris).await;
        cleanup_result?;
        let remaining = remaining_result?;
        if remaining != 0 {
            return Err(CustomError::DatabaseError(format!(
                "Oxigraph smoke cleanup left {remaining} quads in smoke graphs"
            )));
        }
        test_result
    }

    fn ensure_live_smoke_test_endpoint(endpoint: &str) -> Result<(), CustomError> {
        let parsed = reqwest::Url::parse(endpoint).map_err(|error| {
            CustomError::ConfigParseError(format!(
                "OXIGRAPH_TEST_CONNECTION_STRING must be a valid test endpoint URL: {error}"
            ))
        })?;
        let host = parsed.host_str().unwrap_or_default();
        let is_local_test_endpoint = parsed.scheme() == "http"
            && matches!(host, "localhost" | "127.0.0.1" | "::1")
            && parsed.port_or_known_default() == Some(7879);
        if is_local_test_endpoint {
            return Ok(());
        }

        Err(CustomError::ConfigParseError(
            "Refusing live Oxigraph smoke cleanup outside the local test endpoint http://localhost:7879"
            .to_owned(),
        ))
    }

    #[test]
    fn live_smoke_endpoint_guard_allows_only_local_test_service() {
        assert!(ensure_live_smoke_test_endpoint("http://localhost:7879").is_ok());
        assert!(ensure_live_smoke_test_endpoint("http://127.0.0.1:7879").is_ok());
        assert!(ensure_live_smoke_test_endpoint("http://localhost:7878").is_err());
        assert!(ensure_live_smoke_test_endpoint("https://localhost:7879").is_err());
        assert!(ensure_live_smoke_test_endpoint("http://example.com:7879").is_err());
    }

    #[tokio::test]
    async fn persistent_oxigraph_reopens_and_hydrates_objects_links_and_lifecycle_from_rdf() {
        let graph_dir = TempGraphDir::new();
        let fixtures = representative_fixtures();
        let mut archived_thread = fixtures.soft_thread.clone();
        archived_thread.status = ThreadStatus::Archived;

        {
            let store = OxigraphGraphAuthorityStore::new_persistent(graph_dir.path()).unwrap();
            store
                .upsert_objects(&[
                    MemoryObject::Episode(fixtures.episode.clone()),
                    MemoryObject::Observation(fixtures.salient_observation.clone()),
                    MemoryObject::Entity(fixtures.user_entity.clone()),
                    MemoryObject::MemoryThread(archived_thread.clone()),
                    MemoryObject::DerivedMemory(fixtures.correction.clone()),
                    MemoryObject::DerivedMemory(fixtures.suppressed_seed.clone()),
                ])
                .await
                .unwrap();
            store.upsert_links(&fixtures.links()).await.unwrap();
        }

        {
            let reopened = OxigraphGraphAuthorityStore::new_persistent(graph_dir.path()).unwrap();
            let queried = reopened
                .query_objects(&GraphObjectQuery::by_refs(vec![
                    GraphObjectRef::new(fixtures.episode.id, ObjectType::Episode),
                    GraphObjectRef::new(fixtures.salient_observation.id, ObjectType::Observation),
                    GraphObjectRef::new(fixtures.user_entity.id, ObjectType::Entity),
                    GraphObjectRef::new(archived_thread.id, ObjectType::MemoryThread),
                    GraphObjectRef::new(fixtures.correction.id, ObjectType::DerivedMemory),
                    GraphObjectRef::new(fixtures.suppressed_seed.id, ObjectType::DerivedMemory),
                ]))
                .await
                .unwrap();

            assert!(queried.contains(&MemoryObject::Episode(fixtures.episode.clone())));
            assert!(queried.contains(&MemoryObject::Observation(
                fixtures.salient_observation.clone()
            )));
            assert!(queried.contains(&MemoryObject::Entity(fixtures.user_entity.clone())));
            assert!(queried.contains(&MemoryObject::MemoryThread(archived_thread.clone())));
            assert!(queried.contains(&MemoryObject::DerivedMemory(fixtures.correction.clone())));
            assert!(queried.contains(&MemoryObject::DerivedMemory(
                fixtures.suppressed_seed.clone()
            )));

            let by_provenance = reopened
                .query_derived_memories_by_provenance(
                    &GraphDerivedMemoryProvenanceQuery::by_sources(
                        vec![fixtures.episode.id],
                        vec![fixtures.salient_observation.id],
                    ),
                )
                .await
                .unwrap();
            assert!(by_provenance
                .iter()
                .any(|memory| memory.id == fixtures.correction.id));
            assert!(!by_provenance
                .iter()
                .any(|memory| memory.id == fixtures.suppressed_seed.id));

            let default_expansion = reopened
                .expand_bounded(&GraphExpansionQuery::new(
                    fixtures.correction.id,
                    ObjectType::DerivedMemory,
                    1,
                    5,
                ))
                .await
                .unwrap();
            assert!(default_expansion
                .objects
                .contains(&MemoryObject::DerivedMemory(fixtures.correction.clone())));
            assert!(!default_expansion
                .objects
                .contains(&MemoryObject::DerivedMemory(
                    fixtures.suppressed_seed.clone()
                )));
            assert!(default_expansion.filtered_nodes.iter().any(|filtered| {
                filtered.object_ref
                    == GraphObjectRef::new(fixtures.suppressed_seed.id, ObjectType::DerivedMemory)
            }));
        }
    }

    #[tokio::test]
    async fn retrieve_pipeline_expands_fixed_vector_candidate_with_embedded_oxigraph() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store.upsert_links(&fixtures.links()).await.unwrap();

        let vector = FixedVectorStore::new(vec![VectorCandidateMatch::new(
            fixtures.hub_entity.id,
            ObjectType::Entity,
            VectorSurface::Summary,
            0.99,
        )]);
        let embedder = FixedEmbedder::new(vec![1.0, 0.0]);
        let pipeline = RetrievePipeline::new(&store, &vector, &embedder);

        let outcome = pipeline
            .retrieve(RetrievalContext::new("store contract continuity").with_trace())
            .await
            .unwrap();
        let repeated = pipeline
            .retrieve(RetrievalContext::new("store contract continuity").with_trace())
            .await
            .unwrap();
        let trace = outcome.trace.as_ref().unwrap();
        let repeated_trace = repeated.trace.as_ref().unwrap();
        let included_assignments = trace
            .section_assignments
            .iter()
            .filter(|assignment| assignment.section != ContextPackSection::Omitted)
            .count();

        assert_eq!(outcome.pack.relevant_episodes[0].id, fixtures.episode.id);
        assert_eq!(
            outcome.pack.derived_memories[0].memory.id,
            fixtures.derived_reflection.id
        );
        assert_eq!(outcome.rationale.vector_candidate_count, 1);
        assert_eq!(outcome.rationale.graph_verified_count, included_assignments);
        assert!(outcome
            .rationale
            .summary
            .contains("final context-pack objects"));
        assert!(trace
            .lifecycle_filter_decisions
            .iter()
            .any(|decision| decision.object.id == fixtures.hub_entity.id
                && decision.action == LifecycleFilterAction::Included));
        assert!(trace.section_assignments.iter().any(|assignment| {
            assignment.object.id == fixtures.episode.id
                && assignment.section == ContextPackSection::RelevantEpisodes
                && assignment.reason.is_some()
        }));
        assert_eq!(trace.vector_candidates.len(), 1);
        assert_eq!(trace.vector_candidates[0].object.id, fixtures.hub_entity.id);
        assert_eq!(
            trace
                .section_assignments
                .iter()
                .map(|assignment| (assignment.object.id, assignment.section, assignment.rank))
                .collect::<Vec<_>>(),
            repeated_trace
                .section_assignments
                .iter()
                .map(|assignment| (assignment.object.id, assignment.section, assignment.rank))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            trace
                .graph_relations
                .iter()
                .map(|relation| (relation.from.id, relation.to.id, relation.relation))
                .collect::<Vec<_>>(),
            repeated_trace
                .graph_relations
                .iter()
                .map(|relation| (relation.from.id, relation.to.id, relation.relation))
                .collect::<Vec<_>>()
        );
        assert!(trace.graph_relations.iter().any(|relation| {
            relation.from.id == fixtures.hub_entity.id
                && relation.to.id == fixtures.episode.id
                && relation.relation == RelationType::Involves
        }));
        assert!(trace.section_assignments.iter().any(|assignment| {
            assignment.object.id == fixtures.derived_reflection.id
                && assignment.section == ContextPackSection::DerivedMemories
        }));
    }

    #[tokio::test]
    async fn retrieve_pipeline_after_persistent_reopen_uses_graph_authority_filters() {
        let graph_dir = TempGraphDir::new();
        let fixtures = representative_fixtures();
        let missing_vector_only_id = MemoryId::new_v4();

        {
            let store = OxigraphGraphAuthorityStore::new_persistent(graph_dir.path()).unwrap();
            store.upsert_objects(&fixtures.objects()).await.unwrap();
            store.upsert_links(&fixtures.links()).await.unwrap();
        }

        {
            let reopened = OxigraphGraphAuthorityStore::new_persistent(graph_dir.path()).unwrap();
            let vector = FixedVectorStore::new(vec![
                VectorCandidateMatch::new(
                    fixtures.derived_reflection.id,
                    ObjectType::DerivedMemory,
                    VectorSurface::Summary,
                    0.99,
                ),
                VectorCandidateMatch::new(
                    fixtures.suppressed_seed.id,
                    ObjectType::DerivedMemory,
                    VectorSurface::Summary,
                    0.98,
                ),
                VectorCandidateMatch::new(
                    missing_vector_only_id,
                    ObjectType::DerivedMemory,
                    VectorSurface::Summary,
                    0.97,
                ),
            ]);
            let embedder = FixedEmbedder::new(vec![1.0, 0.0]);
            let pipeline = RetrievePipeline::new(&reopened, &vector, &embedder);

            let outcome = pipeline
                .retrieve(RetrievalContext::new("restart graph authority").with_trace())
                .await
                .unwrap();
            let retrieved_ids = outcome
                .pack
                .derived_memories
                .iter()
                .chain(outcome.pack.preferences.iter())
                .map(|included| included.memory.id)
                .collect::<HashSet<_>>();

            assert!(retrieved_ids.contains(&fixtures.derived_reflection.id));
            assert!(!retrieved_ids.contains(&fixtures.suppressed_seed.id));
            assert!(!retrieved_ids.contains(&missing_vector_only_id));
            let trace = outcome.trace.as_ref().unwrap();
            assert!(trace.vector_candidates.iter().any(|candidate| {
                candidate.object.id == missing_vector_only_id
                    && candidate.object.object_type == ObjectType::DerivedMemory
            }));
            assert!(trace.lifecycle_filter_decisions.iter().any(|decision| {
                decision.object.id == fixtures.suppressed_seed.id
                    && decision.action == LifecycleFilterAction::Omitted
            }));
        }
    }

    #[tokio::test]
    async fn oxigraph_memory_links_are_graph_only_not_vector_indexed_records() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let link = fixtures.soft_thread_link.clone();

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store
            .upsert_links(std::slice::from_ref(&link))
            .await
            .unwrap();

        assert_eq!(
            memory_object_vector_record(&MemoryObject::MemoryLink(link.clone())),
            None
        );
        let graph_refs = store
            .query_objects(&GraphObjectQuery::by_refs(vec![GraphObjectRef::new(
                link.id,
                ObjectType::MemoryLink,
            )]))
            .await
            .unwrap();
        assert_eq!(graph_refs, Vec::<MemoryObject>::new());

        let expansion = store
            .expand_bounded(&GraphExpansionQuery::new(
                fixtures.salient_observation.id,
                ObjectType::Observation,
                1,
                4,
            ))
            .await
            .unwrap();
        assert!(expansion.links.contains(&link));
    }

    #[derive(Debug)]
    struct FixedEmbedder {
        embedding: Vec<f32>,
    }

    impl FixedEmbedder {
        fn new(embedding: Vec<f32>) -> Self {
            Self { embedding }
        }
    }

    #[async_trait]
    impl MemoryEmbedder for FixedEmbedder {
        async fn embed(&self, _input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
            Ok(self.embedding.clone())
        }

        async fn embed_batch(
            &self,
            inputs: &[EmbeddingInput],
        ) -> Result<Vec<Vec<f32>>, CustomError> {
            Ok(vec![self.embedding.clone(); inputs.len()])
        }
    }

    #[derive(Debug)]
    struct FixedVectorStore {
        candidates: Vec<VectorCandidateMatch>,
    }

    impl FixedVectorStore {
        fn new(candidates: Vec<VectorCandidateMatch>) -> Self {
            Self { candidates }
        }
    }

    #[async_trait]
    impl VectorCandidateStore for FixedVectorStore {
        async fn upsert_vector_records(
            &self,
            _records: &[VectorRecordEmbedding<'_>],
        ) -> Result<(), CustomError> {
            Ok(())
        }

        async fn search_candidates(
            &self,
            query: &VectorCandidateSearch,
        ) -> Result<Vec<VectorCandidateMatch>, CustomError> {
            let mut candidates = self.candidates.clone();
            candidates.truncate(query.limit);
            Ok(candidates)
        }

        async fn list_candidate_diagnostics(
            &self,
        ) -> Result<
            Vec<crate::internal::models::vector::VectorCandidateDiagnosticRecord>,
            CustomError,
        > {
            Ok(Vec::new())
        }

        async fn delete_candidates(&self, _object_ids: &[MemoryId]) -> Result<(), CustomError> {
            Ok(())
        }
    }
}
