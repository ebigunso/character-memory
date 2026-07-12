use std::collections::HashSet;

use async_trait::async_trait;
use oxigraph::model::Quad;
use oxigraph::store::Store;

use crate::api::types::{
    graph_uri, DerivedMemory, MemoryId, MemoryLink, MemoryObject, ObjectType, RelationType,
};
use crate::errors::CustomError;
use crate::policy::graph_expansion::{
    bounded_expansion, derived_memories_by_provenance, derived_memories_by_thread,
};
use crate::policy::graph_expansion::{bounded_incident_link_refs, BoundedExpansionLinkRef};
use crate::ports::graph_authority::{
    GraphAuthorityStore, GraphDerivedMemoryProvenanceQuery, GraphDerivedMemoryThreadQuery,
    GraphExpansion, GraphExpansionQuery, GraphObjectQuery, GraphObjectRef,
};

use super::rdf_mapping::{rdf_triples_for_link, rdf_triples_for_object};
use super::shared::*;
use super::vocabulary as vocab;

pub(crate) struct OxigraphHttpGraphAuthorityStore {
    endpoint: String,
    client: reqwest::Client,
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
    pub(crate) async fn delete_named_graphs(
        &self,
        graph_uris: &[String],
    ) -> Result<(), CustomError> {
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
    pub(crate) async fn named_graph_quad_count(
        &self,
        graph_uris: &[String],
    ) -> Result<usize, CustomError> {
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
        let mut fanout_utilization = Vec::new();
        let mut bounded_failure = None;
        let mut frontier = vec![root_ref];

        for depth in 0..query.max_depth {
            let link_refs = self.query_link_refs_touching(&frontier).await?;
            let link_refs_by_endpoint = link_refs_by_endpoint(&link_refs);
            let mut next_frontier = Vec::new();
            for object_ref in &frontier {
                let incident_link_refs = link_refs_by_endpoint
                    .get(object_ref)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                let (bounded_link_refs, utilization) = bounded_incident_link_refs(
                    query,
                    root_ref,
                    *object_ref,
                    depth,
                    incident_link_refs,
                    &mut bounded_failure,
                )?;
                fanout_utilization.extend(utilization);
                for link_ref in bounded_link_refs {
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
            fanout_utilization,
            bounded_failure,
        })
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

        let mut hydrated_query = query.clone();
        hydrated_query.record_fanout_utilization = false;
        let mut expansion = bounded_expansion(&hydrated_query, objects, links)?;
        assign_expanded_fanout_utilization(&mut expansion, visibility.fanout_utilization);
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
