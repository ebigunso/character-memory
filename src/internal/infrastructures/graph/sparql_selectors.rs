// Internal SPARQL selectors for the embedded Oxigraph authority. These helpers
// return backend-neutral IDs/refs; canonical object hydration stays in the cache.
#![allow(dead_code)]

use std::collections::HashSet;

use oxigraph::model::Term;
use oxigraph::sparql::{QueryResults, QuerySolution, SparqlEvaluator};
use oxigraph::store::Store;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::api::types::{graph_uri, MemoryId, ObjectType};
use crate::errors::CustomError;
use crate::internal::repositories::{
    GraphDerivedMemoryProvenanceQuery, GraphDerivedMemoryThreadQuery, GraphObjectQuery,
    GraphObjectRef,
};

use super::vocabulary as vocab;

pub(crate) struct SparqlGraphSelectors<'a> {
    store: &'a Store,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LifecycleCurrentnessPredicates {
    pub(crate) retention_state: &'static str,
    pub(crate) is_current: &'static str,
    pub(crate) thread_status: &'static str,
    pub(crate) supersedes: &'static str,
    pub(crate) supersedes_relation: &'static str,
}

impl<'a> SparqlGraphSelectors<'a> {
    pub(crate) fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub(crate) fn select_objects(
        &self,
        query: &GraphObjectQuery,
    ) -> Result<Vec<GraphObjectRef>, CustomError> {
        let mut refs = self.select_object_refs(
            r#"
            SELECT DISTINCT ?id ?objectType WHERE {
              GRAPH ?g {
                ?subject <urn:cmem:vocab:objectId> ?id ;
                         <urn:cmem:vocab:objectType> ?objectType .
              }
            }
            "#,
        )?;

        refs.retain(|object_ref| object_matches_query(*object_ref, query));
        sort_object_refs(&mut refs);
        refs.dedup();
        if let Some(limit) = query.limit {
            refs.truncate(limit);
        }
        Ok(refs)
    }

    pub(crate) fn select_derived_memories_by_provenance(
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

        let values = sparql_iri_values("source", sources.iter().map(String::as_str));
        let query_text = format!(
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
        );

        self.select_memory_ids(&query_text, query.limit)
    }

    pub(crate) fn select_derived_memories_by_thread(
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

        self.select_derived_memories_by_resource_predicate(
            vocab::PART_OF_THREAD,
            threads.iter().map(String::as_str),
            query.limit,
        )
    }

    pub(crate) fn select_derived_memories_by_entity(
        &self,
        entity_ids: &[MemoryId],
        limit: Option<usize>,
    ) -> Result<Vec<MemoryId>, CustomError> {
        let entities = entity_ids
            .iter()
            .map(|id| graph_uri(ObjectType::Entity, *id))
            .collect::<Vec<_>>();

        if entities.is_empty() {
            return Ok(Vec::new());
        }

        self.select_derived_memories_by_resource_predicate(
            vocab::ABOUT_ENTITY,
            entities.iter().map(String::as_str),
            limit,
        )
    }

    pub(crate) const fn lifecycle_currentness_predicates() -> LifecycleCurrentnessPredicates {
        LifecycleCurrentnessPredicates {
            retention_state: vocab::RETENTION_STATE,
            is_current: vocab::IS_CURRENT,
            thread_status: vocab::THREAD_STATUS,
            supersedes: vocab::SUPERSEDES,
            supersedes_relation: "urn:cmem:relation:supersedes",
        }
    }

    pub(crate) fn select_superseded_derived_memory_ids(
        &self,
    ) -> Result<Vec<MemoryId>, CustomError> {
        let query_text = format!(
            r#"
            SELECT DISTINCT ?id WHERE {{
              GRAPH ?memoryGraph {{
                ?memory a <{derived_class}> ;
                        <{object_id}> ?id .
              }}
              GRAPH ?supersessionGraph {{
                {{
                  ?replacement <{supersedes}> ?memory .
                }} UNION {{
                  ?replacement <{supersedes_relation}> ?memory .
                }}
              }}
            }}
            "#,
            derived_class = vocab::CLASS_DERIVED_MEMORY,
            object_id = vocab::OBJECT_ID,
            supersedes = vocab::SUPERSEDES,
            supersedes_relation = vocab::relation_predicate("supersedes"),
        );

        self.select_memory_ids(&query_text, None)
    }

    fn select_derived_memories_by_resource_predicate<'b>(
        &self,
        predicate: &str,
        resources: impl Iterator<Item = &'b str>,
        limit: Option<usize>,
    ) -> Result<Vec<MemoryId>, CustomError> {
        let values = sparql_iri_values("resource", resources);
        let query_text = format!(
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
        );

        self.select_memory_ids(&query_text, limit)
    }

    fn select_object_refs(&self, query_text: &str) -> Result<Vec<GraphObjectRef>, CustomError> {
        let mut refs = Vec::new();
        for solution in self.query_solutions(query_text)? {
            let id = memory_id_binding(&solution, "id")?;
            let object_type = enum_binding(&solution, "objectType")?;
            refs.push(GraphObjectRef::new(id, object_type));
        }
        Ok(refs)
    }

    fn select_memory_ids(
        &self,
        query_text: &str,
        limit: Option<usize>,
    ) -> Result<Vec<MemoryId>, CustomError> {
        let mut ids = Vec::new();
        let mut seen = HashSet::new();
        for solution in self.query_solutions(query_text)? {
            let id = memory_id_binding(&solution, "id")?;
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

    fn query_solutions(&self, query_text: &str) -> Result<Vec<QuerySolution>, CustomError> {
        let results = SparqlEvaluator::new()
            .parse_query(query_text)
            .map_err(oxigraph_sparql_error)?
            .on_store(self.store)
            .execute()
            .map_err(oxigraph_sparql_error)?;

        let QueryResults::Solutions(solutions) = results else {
            return Err(CustomError::DatabaseError(
                "Oxigraph SPARQL selector expected SELECT solutions".to_owned(),
            ));
        };

        solutions
            .collect::<Result<Vec<_>, _>>()
            .map_err(oxigraph_sparql_error)
    }
}

fn object_matches_query(object_ref: GraphObjectRef, query: &GraphObjectQuery) -> bool {
    (query.object_refs.is_empty() || query.object_refs.contains(&object_ref))
        && (query.object_ids.is_empty() || query.object_ids.contains(&object_ref.object_id))
        && (query.object_types.is_empty() || query.object_types.contains(&object_ref.object_type))
}

fn memory_id_binding(solution: &QuerySolution, name: &str) -> Result<MemoryId, CustomError> {
    let value = literal_binding(solution, name)?;
    value.parse::<MemoryId>().map_err(|error| {
        CustomError::DatabaseError(format!(
            "Oxigraph SPARQL invalid MemoryId binding {name}: {error}"
        ))
    })
}

fn enum_binding<T: DeserializeOwned>(
    solution: &QuerySolution,
    name: &str,
) -> Result<T, CustomError> {
    let value = literal_binding(solution, name)?;
    serde_json::from_value(Value::String(value.to_owned())).map_err(|error| {
        CustomError::DatabaseError(format!(
            "Oxigraph SPARQL invalid enum binding {name}: {error}"
        ))
    })
}

fn literal_binding<'a>(solution: &'a QuerySolution, name: &str) -> Result<&'a str, CustomError> {
    match solution.get(name) {
        Some(Term::Literal(literal)) => Ok(literal.value()),
        Some(value) => Err(CustomError::DatabaseError(format!(
            "Oxigraph SPARQL binding {name} expected literal, got {value}"
        ))),
        None => Err(CustomError::DatabaseError(format!(
            "Oxigraph SPARQL missing binding {name}"
        ))),
    }
}

fn sparql_iri_values<'a>(variable: &str, values: impl Iterator<Item = &'a str>) -> String {
    let values = values
        .map(|value| format!("<{}>", sparql_iri(value)))
        .collect::<Vec<_>>()
        .join(" ");
    format!("VALUES ?{variable} {{ {values} }}")
}

fn sparql_iri(value: &str) -> String {
    value.replace('>', "%3E")
}

fn sort_object_refs(refs: &mut [GraphObjectRef]) {
    refs.sort_by_key(|object_ref| stable_node_key(object_ref.object_id, object_ref.object_type));
}

fn stable_node_key(object_id: MemoryId, object_type: ObjectType) -> (MemoryId, u8) {
    (object_id, object_type_rank(object_type))
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

fn oxigraph_sparql_error(error: impl std::fmt::Display) -> CustomError {
    CustomError::DatabaseError(format!("Oxigraph SPARQL selector error: {error}"))
}

#[cfg(test)]
mod tests {
    use oxigraph::model::{GraphName, Literal, NamedNode, NamedOrBlankNode, Quad};

    use super::*;
    use crate::api::types::{graph_uri, MemoryObject};
    use crate::internal::infrastructures::graph::rdf_mapping::{
        rdf_triples_for_link, rdf_triples_for_object, RdfObject, RdfTriple,
    };
    use crate::internal::repositories::test_support::representative_fixtures;

    #[test]
    fn sparql_selectors_find_objects_by_id_and_type_across_named_graphs() {
        let store = store_with_representative_fixture();
        let fixtures = representative_fixtures();
        let selectors = SparqlGraphSelectors::new(&store);

        let selected = selectors
            .select_objects(&GraphObjectQuery {
                object_refs: Vec::new(),
                object_ids: vec![fixtures.episode.id, fixtures.correction.id],
                object_types: vec![ObjectType::Episode, ObjectType::DerivedMemory],
                limit: None,
            })
            .unwrap();

        assert_eq!(
            selected,
            vec![
                GraphObjectRef::new(fixtures.episode.id, ObjectType::Episode),
                GraphObjectRef::new(fixtures.correction.id, ObjectType::DerivedMemory),
            ]
        );
    }

    #[test]
    fn sparql_selectors_ignore_default_graph_object_identity_triples() {
        let store = Store::new().unwrap();
        let fixtures = representative_fixtures();
        let episode = MemoryObject::Episode(fixtures.episode.clone());
        for triple in rdf_triples_for_object(&episode).unwrap() {
            store
                .insert(&default_graph_quad_for_triple(&triple).unwrap())
                .unwrap();
        }

        let selected = SparqlGraphSelectors::new(&store)
            .select_objects(&GraphObjectQuery::by_ids(vec![fixtures.episode.id]))
            .unwrap();

        assert_eq!(selected, Vec::<GraphObjectRef>::new());
    }

    #[test]
    fn sparql_selectors_find_derived_memories_by_provenance_thread_and_entity() {
        let store = store_with_representative_fixture();
        let fixtures = representative_fixtures();
        let selectors = SparqlGraphSelectors::new(&store);

        let by_provenance = selectors
            .select_derived_memories_by_provenance(&GraphDerivedMemoryProvenanceQuery::by_sources(
                vec![fixtures.episode.id],
                vec![fixtures.salient_observation.id],
            ))
            .unwrap();
        let by_thread = selectors
            .select_derived_memories_by_thread(&GraphDerivedMemoryThreadQuery::by_threads(vec![
                fixtures.soft_thread.id,
            ]))
            .unwrap();
        let by_entity = selectors
            .select_derived_memories_by_entity(&[fixtures.user_entity.id], None)
            .unwrap();

        assert!(by_provenance.contains(&fixtures.correction.id));
        assert!(by_provenance.contains(&fixtures.derived_reflection.id));
        assert!(by_thread.contains(&fixtures.correction.id));
        assert!(by_entity.contains(&fixtures.correction.id));
    }

    #[test]
    fn sparql_selectors_expose_lifecycle_predicates_and_superseded_ids() {
        let store = store_with_representative_fixture();
        let fixtures = representative_fixtures();
        let selectors = SparqlGraphSelectors::new(&store);
        let predicates = SparqlGraphSelectors::lifecycle_currentness_predicates();

        assert_eq!(predicates.retention_state, vocab::RETENTION_STATE);
        assert_eq!(predicates.is_current, vocab::IS_CURRENT);
        assert_eq!(predicates.thread_status, vocab::THREAD_STATUS);
        assert_eq!(predicates.supersedes, vocab::SUPERSEDES);
        assert_eq!(
            predicates.supersedes_relation,
            "urn:cmem:relation:supersedes"
        );
        assert!(selectors
            .select_superseded_derived_memory_ids()
            .unwrap()
            .contains(&fixtures.suppressed_seed.id));
    }

    fn store_with_representative_fixture() -> Store {
        let store = Store::new().unwrap();
        let fixtures = representative_fixtures();
        for object in fixtures.objects() {
            let (object_id, object_type) = object_identity(&object);
            insert_triples(
                &store,
                &graph_uri(object_type, object_id),
                &rdf_triples_for_object(&object).unwrap(),
            );
        }
        for link in fixtures.links() {
            insert_triples(
                &store,
                &graph_uri(ObjectType::MemoryLink, link.id),
                &rdf_triples_for_link(&link).unwrap(),
            );
        }
        store
    }

    fn insert_triples(store: &Store, owner_graph_uri: &str, triples: &[RdfTriple]) {
        for triple in triples {
            store
                .insert(&quad_for_triple(owner_graph_uri, triple).unwrap())
                .unwrap();
        }
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

    fn default_graph_quad_for_triple(triple: &RdfTriple) -> Result<Quad, CustomError> {
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
}
