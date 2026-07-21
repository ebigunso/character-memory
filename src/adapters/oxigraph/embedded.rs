use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use async_trait::async_trait;
use oxigraph::model::{GraphName, NamedNode, Quad};
#[cfg(test)]
use oxigraph::model::{Literal, NamedOrBlankNode, Term};
use oxigraph::store::Store;

use crate::domain::{
    graph_uri, DerivedMemory, MemoryId, MemoryLink, MemoryObject, MemoryObjectRef, ObjectType,
};
use crate::errors::CustomError;
use crate::policy::graph_expansion::{
    bounded_expansion, derived_memories_by_provenance, derived_memories_by_thread,
};
use crate::ports::graph_authority::{
    GraphAuthorityStore, GraphDerivedMemoryProvenanceQuery, GraphDerivedMemoryThreadQuery,
    GraphExpansion, GraphExpansionQuery, GraphObjectQuery,
};

#[cfg(test)]
use super::rdf_mapping::RdfObject;
use super::rdf_mapping::{rdf_triples_for_link, rdf_triples_for_object, RdfTriple};
use super::shared::*;
use super::sparql_selectors::SparqlGraphSelectors;

pub(crate) struct OxigraphGraphAuthorityStore {
    pub(crate) store: Store,
    inserted_quads: Mutex<HashMap<String, Vec<Quad>>>,
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

    // Embedded adapter tests assert graph mutation counts; remove when tests assert via graph queries only.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn triple_count(&self) -> Result<usize, CustomError> {
        Ok(self.store.iter().count())
    }

    // Embedded adapter tests simulate graph corruption directly; remove when those tests use public writes only.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn replace_triples(
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
    pub(crate) fn contains_triple(&self, triple: &RdfTriple) -> Result<bool, CustomError> {
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
    pub(crate) fn contains_triple_in_graph(
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
    pub(crate) fn matching_triple_count(&self, triple: &RdfTriple) -> Result<usize, CustomError> {
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

    async fn query_links_by_ids(
        &self,
        link_ids: &[MemoryId],
    ) -> Result<Vec<MemoryLink>, CustomError> {
        hydrate_links_by_ids_from_store(&self.store, link_ids)
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
                .map(|id| MemoryObjectRef::from_id_type(id, ObjectType::DerivedMemory))
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
                .map(|id| MemoryObjectRef::from_id_type(id, ObjectType::DerivedMemory))
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
        let root_ref = MemoryObjectRef::from_id_type(query.root_id, query.root_type);
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

        let mut hydrated_query = query.clone();
        hydrated_query.trace_mode = crate::ports::graph_authority::TraceMode::Disabled;
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
        let object_refs = SparqlGraphSelectors::new(&self.store)
            .select_objects(&GraphObjectQuery::by_types(object_types.to_vec(), None))?;
        hydrate_objects_by_refs_from_store(&self.store, &object_refs)
    }

    async fn list_diagnostic_links(&self) -> Result<Vec<MemoryLink>, CustomError> {
        hydrate_all_links_from_store(&self.store)
    }
}
