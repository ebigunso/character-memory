use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use async_trait::async_trait;
use oxigraph::model::{GraphName, NamedNode, Quad};
use oxigraph::store::Store;

use crate::domain::{
    graph_uri, DerivedMemory, MemoryId, MemoryLink, MemoryObject, MemoryObjectRef, ObjectType,
    ScopeKey,
};
use crate::errors::{CustomError, GraphQueryError};
use crate::policy::graph_expansion::{
    bounded_expansion, derived_memories_by_provenance, derived_memories_by_thread,
};
use crate::ports::graph_authority::{
    GraphAuthorityStore, GraphDerivedMemoryProvenanceQuery, GraphDerivedMemoryThreadQuery,
    GraphExpansion, GraphExpansionFilteredNode, GraphExpansionLifecyclePolicy, GraphExpansionQuery,
    GraphObjectQuery,
};

use super::rdf_mapping::{rdf_triples_for_link, rdf_triples_for_object};
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
}

#[async_trait]
impl GraphAuthorityStore for OxigraphGraphAuthorityStore {
    async fn query_anniversaries(
        &self,
        date: chrono::NaiveDate,
        participants: &[MemoryId],
        limit: usize,
        policy: GraphExpansionLifecyclePolicy,
    ) -> Result<Vec<(crate::ports::graph_authority::GraphMemoryRank, bool)>, CustomError> {
        SparqlGraphSelectors::new(&self.store).select_anniversaries(
            date,
            participants,
            limit,
            policy,
        )
    }

    async fn query_episodes_by_time(
        &self,
        start: Option<chrono::DateTime<chrono::Utc>>,
        end: chrono::DateTime<chrono::Utc>,
        limit: usize,
        policy: GraphExpansionLifecyclePolicy,
    ) -> Result<Vec<crate::ports::graph_authority::GraphMemoryRank>, CustomError> {
        SparqlGraphSelectors::new(&self.store).select_episodes_by_time(start, end, limit, policy)
    }

    async fn query_episode_occasions(
        &self,
        episodes: &[MemoryObjectRef],
    ) -> Result<crate::policy::graph_expansion::ParticipantOccasions, CustomError> {
        SparqlGraphSelectors::new(&self.store).select_participant_occasions(episodes)
    }

    async fn query_last_interaction(
        &self,
        participant: MemoryId,
        reference_time: chrono::DateTime<chrono::Utc>,
        policy: GraphExpansionLifecyclePolicy,
    ) -> Result<Option<(MemoryId, chrono::DateTime<chrono::Utc>)>, CustomError> {
        SparqlGraphSelectors::new(&self.store).select_last_interaction(
            participant,
            reference_time,
            policy,
        )
    }

    async fn query_notions_known_as(&self, name: &str) -> Result<Vec<MemoryId>, GraphQueryError> {
        SparqlGraphSelectors::new(&self.store)
            .select_notions_known_as(name)
            .map_err(|error| GraphQueryError::Selection {
                detail: error.to_string(),
            })
    }

    async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError> {
        let mut replacements = Vec::new();
        for object in objects {
            object.validate()?;
            let owner_graph_uri = graph_uri(object.object_type(), object.id());
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
            link.validate()?;
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
            object.validate()?;
            let owner_graph_uri = graph_uri(object.object_type(), object.id());
            replacements.push((
                owner_graph_uri.clone(),
                quads_for_triples(&owner_graph_uri, &rdf_triples_for_object(object)?)?,
            ));
        }

        for link in links {
            link.validate()?;
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
    ) -> Result<Vec<MemoryObject>, GraphQueryError> {
        let selected_refs = SparqlGraphSelectors::new(&self.store)
            .select_objects(query)
            .map_err(|error| GraphQueryError::Selection {
                detail: error.to_string(),
            })?;
        hydrate_objects_by_refs_from_store(&self.store, &selected_refs).map_err(|error| {
            GraphQueryError::Hydration {
                detail: error.to_string(),
            }
        })
    }

    async fn query_superseded_derived_memory_ids(
        &self,
        memory_ids: &[MemoryId],
    ) -> Result<Vec<MemoryId>, GraphQueryError> {
        let refs = memory_ids
            .iter()
            .map(|id| MemoryObjectRef::new(ObjectType::DerivedMemory, *id))
            .collect::<Vec<_>>();
        let mut ids = SparqlGraphSelectors::new(&self.store)
            .select_links_touching(&refs, false)
            .map_err(|error| GraphQueryError::Selection {
                detail: error.to_string(),
            })?
            .into_iter()
            .filter(|link| {
                link.relation == crate::domain::RelationType::Supersedes
                    && link.from.object_type == ObjectType::DerivedMemory
                    && link.to.object_type == ObjectType::DerivedMemory
                    && memory_ids.contains(&link.to.id)
            })
            .map(|link| link.to.id)
            .collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        Ok(ids)
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
        let selected_ids = SparqlGraphSelectors::new(&self.store)
            .select_derived_memories_by_provenance(query)?
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
        let link_ids = SparqlGraphSelectors::new(&self.store).select_link_ids_touching(
            &objects
                .iter()
                .map(MemoryObject::object_ref)
                .collect::<Vec<_>>(),
        )?;
        let links = hydrate_links_by_ids_from_store(&self.store, &link_ids)?;
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
    ) -> Result<(Vec<DerivedMemory>, Vec<GraphExpansionFilteredNode>), CustomError> {
        let selected_ids = SparqlGraphSelectors::new(&self.store)
            .select_derived_memories_by_thread(query)?
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
        let link_ids = SparqlGraphSelectors::new(&self.store).select_link_ids_touching(
            &objects
                .iter()
                .map(MemoryObject::object_ref)
                .collect::<Vec<_>>(),
        )?;
        let links = hydrate_links_by_ids_from_store(&self.store, &link_ids)?;
        Ok(derived_memories_by_thread(
            query,
            objects.into_iter().filter(
                |object| matches!(object, MemoryObject::DerivedMemory(memory) if selected_ids.contains(&memory.id)),
            ),
            links,
        ))
    }

    async fn query_thread_state(
        &self,
        query: &GraphDerivedMemoryThreadQuery,
        limit: usize,
    ) -> Result<
        (
            Vec<crate::ports::graph_authority::GraphMemoryRank>,
            Vec<GraphExpansionFilteredNode>,
        ),
        CustomError,
    > {
        SparqlGraphSelectors::new(&self.store).select_thread_state(query, limit)
    }

    async fn query_scope_state(
        &self,
        key: &ScopeKey,
        policy: GraphExpansionLifecyclePolicy,
        limit: usize,
    ) -> Result<
        (
            Vec<crate::ports::graph_authority::GraphMemoryRank>,
            Vec<GraphExpansionFilteredNode>,
        ),
        CustomError,
    > {
        SparqlGraphSelectors::new(&self.store).select_scope_state(key, policy, limit)
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
        hydrated_query.traversal_link_ids = Some(visibility.traversal_link_ids);
        let mut expansion = bounded_expansion(
            &hydrated_query,
            objects,
            links,
            &visibility.participant_occasions,
        )?;
        assign_expanded_fanout_utilization(&mut expansion, visibility.fanout_utilization);
        if expansion.expanded_nodes.contains(&root_ref) {
            expansion.filtered_nodes.extend(visibility.filtered_nodes);
            expansion
                .filtered_nodes
                .sort_by_key(|filtered| filtered.object_ref.stable_order_key());
            expansion
                .filtered_nodes
                .dedup_by_key(|filtered| filtered.object_ref);
        }
        if expansion.bounded_failure.is_none() {
            expansion.bounded_failure = visibility.bounded_failure;
        }
        Ok(expansion)
    }
}
