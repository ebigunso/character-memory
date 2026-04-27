// Transitional v0.1 Oxigraph authority store: preserves canonical domain
// objects for contract reads while materializing RDF triples into Oxigraph.
// Remove once remember/link production wiring or tests consume the store, or
// prune any remaining unused surface then.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use async_trait::async_trait;
use oxigraph::model::{GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term};
use oxigraph::store::Store;

use crate::api::types::{graph_uri, MemoryId, MemoryLink, MemoryObject, ObjectType};
use crate::errors::CustomError;
use crate::internal::repositories::{
    bounded_expansion_node_set, GraphAuthorityStore, GraphExpansion, GraphExpansionQuery,
    GraphObjectQuery,
};

use super::rdf_mapping::{rdf_triples_for_link, rdf_triples_for_object, RdfObject, RdfTriple};

pub(crate) struct OxigraphGraphAuthorityStore {
    store: Store,
    objects: Mutex<HashMap<(MemoryId, ObjectType), MemoryObject>>,
    links: Mutex<HashMap<MemoryId, MemoryLink>>,
    inserted_quads: Mutex<HashMap<String, Vec<Quad>>>,
}

impl OxigraphGraphAuthorityStore {
    pub(crate) fn new_in_memory() -> Result<Self, CustomError> {
        let store = Store::new().map_err(oxigraph_error)?;
        Ok(Self {
            store,
            objects: Mutex::new(HashMap::new()),
            links: Mutex::new(HashMap::new()),
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
        let quads = quads_for_triples(triples)?;
        let mut inserted_quads = lock(&self.inserted_quads)?;

        if let Some(previous_quads) = inserted_quads.remove(&owner_graph_uri) {
            self.remove_quads(&previous_quads)?;
        }

        self.insert_quads(&quads)?;
        inserted_quads.insert(owner_graph_uri, quads);
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

    #[cfg(test)]
    fn contains_triple(&self, triple: &RdfTriple) -> Result<bool, CustomError> {
        let quad = quad_for_triple(triple)?;
        self.store.contains(&quad).map_err(oxigraph_error)
    }
}

fn quads_for_triples(triples: &[RdfTriple]) -> Result<Vec<Quad>, CustomError> {
    triples.iter().map(quad_for_triple).collect()
}

#[async_trait]
impl GraphAuthorityStore for OxigraphGraphAuthorityStore {
    async fn upsert_objects(&self, objects: &[MemoryObject]) -> Result<(), CustomError> {
        for object in objects {
            object
                .validate()
                .map_err(|error| CustomError::MemoryValidation(error.to_string()))?;
            let (object_id, object_type) = object_identity(object);
            self.replace_triples(
                graph_uri(object_type, object_id),
                &rdf_triples_for_object(object),
            )?;

            let mut stored = lock(&self.objects)?;
            stored.insert(object_identity(object), object.clone());
        }
        Ok(())
    }

    async fn upsert_links(&self, links: &[MemoryLink]) -> Result<(), CustomError> {
        for link in links {
            link.validate()
                .map_err(|error| CustomError::MemoryValidation(error.to_string()))?;
            self.replace_triples(
                graph_uri(ObjectType::MemoryLink, link.id),
                &rdf_triples_for_link(link),
            )?;

            let mut stored = lock(&self.links)?;
            stored.insert(link.id, link.clone());
        }
        Ok(())
    }

    async fn query_objects(
        &self,
        query: &GraphObjectQuery,
    ) -> Result<Vec<MemoryObject>, CustomError> {
        let mut objects: Vec<_> = lock(&self.objects)?
            .values()
            .filter(|object| {
                let (object_id, object_type) = object_identity(object);
                (query.object_ids.is_empty() || query.object_ids.contains(&object_id))
                    && (query.object_types.is_empty() || query.object_types.contains(&object_type))
            })
            .cloned()
            .collect();

        sort_objects(&mut objects);
        if let Some(limit) = query.limit {
            objects.truncate(limit);
        }
        Ok(objects)
    }

    async fn expand_bounded(
        &self,
        query: &GraphExpansionQuery,
    ) -> Result<GraphExpansion, CustomError> {
        let objects = lock(&self.objects)?.clone();
        let links = lock(&self.links)?.clone();
        let root_exists = objects.contains_key(&(query.root_id, query.root_type));
        let visited = bounded_expansion_node_set(query, root_exists, links.values().cloned())?;

        let mut expanded_objects: Vec<_> = objects
            .into_values()
            .filter(|object| visited.contains(&object_identity(object)))
            .collect();
        sort_objects(&mut expanded_objects);

        let mut expanded_links: Vec<_> = links
            .into_values()
            .filter(|link| {
                visited.contains(&(link.from_id, link.from_type))
                    && visited.contains(&(link.to_id, link.to_type))
            })
            .collect();
        expanded_links.sort_by_key(|link| link.id);

        Ok(GraphExpansion::new(expanded_objects, expanded_links))
    }
}

fn quad_for_triple(triple: &RdfTriple) -> Result<Quad, CustomError> {
    let subject = NamedNode::new(triple.subject.as_str())?;
    let predicate = NamedNode::new(triple.predicate.as_str())?;
    let object = match &triple.object {
        RdfObject::Resource(value) => Term::NamedNode(NamedNode::new(value.as_str())?),
        RdfObject::Literal(value) => Term::Literal(Literal::new_simple_literal(value.as_str())),
    };

    Ok(Quad::new(
        NamedOrBlankNode::NamedNode(subject),
        predicate,
        object,
        GraphName::DefaultGraph,
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

impl From<oxigraph::model::IriParseError> for CustomError {
    fn from(error: oxigraph::model::IriParseError) -> Self {
        CustomError::DatabaseError(format!("Invalid RDF IRI: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::vocabulary as vocab;
    use super::*;
    use crate::api::types::{graph_uri, RelationType};
    use crate::internal::repositories::test_support::{
        high_fanout_graph_fixture, representative_fixtures,
    };

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
            rdf_triples_for_object(&MemoryObject::Episode(updated_episode.clone())).len()
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
        assert!(matches!(missing_root, CustomError::DatabaseError(_)));
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
            .upsert_links(&[fixtures.soft_thread_link.clone()])
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
            .upsert_links(&[fixtures.soft_thread_link.clone()])
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
}
