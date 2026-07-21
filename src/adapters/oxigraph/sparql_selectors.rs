// Internal SPARQL selectors for the embedded Oxigraph authority. These helpers
// return backend-neutral IDs/refs; canonical object hydration reads RDF state.
use std::collections::HashSet;

use oxigraph::model::Term;
use oxigraph::sparql::{QueryResults, QuerySolution, SparqlEvaluator};
use oxigraph::store::Store;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::domain::{graph_uri, MemoryId, MemoryObjectRef, ObjectType, RelationType};
use crate::errors::CustomError;
use crate::ports::graph_authority::{
    GraphDerivedMemoryProvenanceQuery, GraphDerivedMemoryThreadQuery, GraphObjectQuery,
};

use super::vocabulary as vocab;

pub(crate) struct SparqlGraphSelectors<'a> {
    store: &'a Store,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SparqlLinkRef {
    pub(crate) link_id: MemoryId,
    pub(crate) from: MemoryObjectRef,
    pub(crate) to: MemoryObjectRef,
    pub(crate) relation: RelationType,
}

impl<'a> SparqlGraphSelectors<'a> {
    pub(crate) fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub(crate) fn select_objects(
        &self,
        query: &GraphObjectQuery,
    ) -> Result<Vec<MemoryObjectRef>, CustomError> {
        let (id_values, type_values, ref_values, limit_clause) = match query {
            GraphObjectQuery::ByRefs(object_refs) => (
                String::new(),
                String::new(),
                sparql_object_ref_values(object_refs),
                String::new(),
            ),
            GraphObjectQuery::ByIds(object_ids) => (
                sparql_literal_values("id", object_ids.iter().map(|id| id.to_string())),
                String::new(),
                String::new(),
                String::new(),
            ),
            GraphObjectQuery::ByTypes {
                object_types,
                limit,
            } => (
                String::new(),
                sparql_literal_values(
                    "objectType",
                    object_types
                        .iter()
                        .map(|object_type| enum_value(*object_type)),
                ),
                String::new(),
                limit
                    .map(|limit| format!("LIMIT {limit}"))
                    .unwrap_or_default(),
            ),
        };
        let select_query = format!(
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
            ORDER BY ?id
              (IF(?objectType = "episode", 0,
                IF(?objectType = "observation", 1,
                  IF(?objectType = "entity", 2,
                    IF(?objectType = "memory_thread", 3,
                      IF(?objectType = "derived_memory", 4, 5))))))
            {limit_clause}
            "#,
            object_id = vocab::OBJECT_ID,
            object_type = vocab::OBJECT_TYPE,
        );

        self.select_object_refs(&select_query)
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

        self.select_memory_ids(&query_text, None)
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
            None,
        )
    }

    pub(crate) fn select_links_touching(
        &self,
        object_refs: &[MemoryObjectRef],
    ) -> Result<Vec<SparqlLinkRef>, CustomError> {
        if object_refs.is_empty() {
            return Ok(Vec::new());
        }

        let node_values = sparql_node_iri_values("node", object_refs);
        let query_text = format!(
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
        );

        let mut refs = Vec::new();
        let mut seen = HashSet::new();
        for solution in self.query_solutions(&query_text)? {
            let link_ref = SparqlLinkRef {
                link_id: memory_id_binding(&solution, "linkId")?,
                from: MemoryObjectRef::from_id_type(
                    memory_id_binding(&solution, "fromId")?,
                    enum_binding(&solution, "fromType")?,
                ),
                to: MemoryObjectRef::from_id_type(
                    memory_id_binding(&solution, "toId")?,
                    enum_binding(&solution, "toType")?,
                ),
                relation: enum_binding(&solution, "relation")?,
            };
            if seen.insert(link_ref) {
                refs.push(link_ref);
            }
        }
        refs.sort_by_key(|link_ref| {
            (
                link_ref.to.id,
                link_ref.from.id,
                link_ref.link_id,
                object_type_rank(link_ref.to.object_type),
                object_type_rank(link_ref.from.object_type),
                relation_type_rank(link_ref.relation),
            )
        });
        Ok(refs)
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

    fn select_object_refs(&self, query_text: &str) -> Result<Vec<MemoryObjectRef>, CustomError> {
        let mut refs = Vec::new();
        for solution in self.query_solutions(query_text)? {
            let id = memory_id_binding(&solution, "id")?;
            let object_type = enum_binding(&solution, "objectType")?;
            refs.push(MemoryObjectRef::from_id_type(id, object_type));
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
    if values.is_empty() {
        return String::new();
    }
    format!("VALUES ?{variable} {{ {values} }}")
}

fn sparql_node_iri_values(variable: &str, object_refs: &[MemoryObjectRef]) -> String {
    let values = object_refs
        .iter()
        .map(|object_ref| {
            let graph_uri = graph_uri(object_ref.object_type, object_ref.id);
            format!("<{}>", sparql_iri(&graph_uri))
        })
        .collect::<Vec<_>>()
        .join(" ");
    if values.is_empty() {
        return String::new();
    }
    format!("VALUES ?{variable} {{ {values} }}")
}

fn sparql_iri(value: &str) -> String {
    value.replace('>', "%3E")
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

fn sparql_object_ref_values(object_refs: &[MemoryObjectRef]) -> String {
    let values = object_refs
        .iter()
        .map(|object_ref| {
            format!(
                "({} {})",
                sparql_string_literal(&object_ref.id.to_string()),
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

fn sparql_string_literal(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a SPARQL string literal cannot fail")
}

fn enum_value(value: impl serde::Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_default()
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

fn oxigraph_sparql_error(error: impl std::fmt::Display) -> CustomError {
    CustomError::DatabaseError(format!("Oxigraph SPARQL selector error: {error}"))
}

#[cfg(test)]
mod tests {
    use oxigraph::model::{GraphName, Literal, NamedNode, NamedOrBlankNode, Quad};

    use super::*;
    use crate::adapters::oxigraph::rdf_mapping::{
        rdf_triples_for_link, rdf_triples_for_object, RdfObject, RdfTriple,
    };
    use crate::domain::{graph_uri, MemoryObject};
    use crate::test_support::representative_fixtures;

    #[test]
    fn sparql_selectors_find_objects_by_id_and_type_across_named_graphs() {
        let store = store_with_representative_fixture();
        let fixtures = representative_fixtures();
        let selectors = SparqlGraphSelectors::new(&store);

        let selected = selectors
            .select_objects(&GraphObjectQuery::by_refs(vec![
                MemoryObjectRef::from_id_type(fixtures.episode.id, ObjectType::Episode),
                MemoryObjectRef::from_id_type(fixtures.correction.id, ObjectType::DerivedMemory),
            ]))
            .unwrap();

        assert_eq!(
            selected,
            vec![
                MemoryObjectRef::from_id_type(fixtures.episode.id, ObjectType::Episode),
                MemoryObjectRef::from_id_type(fixtures.correction.id, ObjectType::DerivedMemory),
            ]
        );
    }

    #[test]
    fn sparql_selectors_apply_limit_after_stable_query_ordering() {
        let store = Store::new().unwrap();
        let fixtures = representative_fixtures();
        let mut entity_with_episode_id = fixtures.project_entity.clone();
        entity_with_episode_id.id = fixtures.episode.id;

        insert_triples(
            &store,
            &graph_uri(ObjectType::Episode, fixtures.episode.id),
            &rdf_triples_for_object(&MemoryObject::Episode(fixtures.episode.clone())).unwrap(),
        );
        insert_triples(
            &store,
            &graph_uri(ObjectType::Entity, entity_with_episode_id.id),
            &rdf_triples_for_object(&MemoryObject::Entity(entity_with_episode_id)).unwrap(),
        );

        let selected = SparqlGraphSelectors::new(&store)
            .select_objects(&GraphObjectQuery::by_types(
                vec![ObjectType::Episode, ObjectType::Entity],
                Some(1),
            ))
            .unwrap();

        assert_eq!(
            selected,
            vec![MemoryObjectRef::from_id_type(
                fixtures.episode.id,
                ObjectType::Episode
            )]
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

        assert_eq!(selected, Vec::<MemoryObjectRef>::new());
    }

    #[test]
    fn sparql_selectors_find_derived_memories_by_provenance_and_thread() {
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
        assert!(by_provenance.contains(&fixtures.correction.id));
        assert!(by_provenance.contains(&fixtures.derived_reflection.id));
        assert!(by_thread.contains(&fixtures.correction.id));
    }

    #[test]
    fn sparql_selectors_find_only_links_touching_frontier_refs() {
        let store = store_with_representative_fixture();
        let fixtures = representative_fixtures();
        let selected = SparqlGraphSelectors::new(&store)
            .select_links_touching(&[MemoryObjectRef::from_id_type(
                fixtures.hub_entity.id,
                ObjectType::Entity,
            )])
            .unwrap();

        let selected_ids = selected
            .iter()
            .map(|link_ref| link_ref.link_id)
            .collect::<Vec<_>>();

        assert!(selected_ids.contains(&fixtures.hub_links[0].id));
        assert!(selected_ids.contains(&fixtures.hub_links[1].id));
        assert!(!selected_ids.contains(&fixtures.soft_thread_link.id));
        assert!(selected.iter().all(|link_ref| {
            link_ref.from
                == MemoryObjectRef::from_id_type(fixtures.hub_entity.id, ObjectType::Entity)
                || link_ref.to
                    == MemoryObjectRef::from_id_type(fixtures.hub_entity.id, ObjectType::Entity)
        }));
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
