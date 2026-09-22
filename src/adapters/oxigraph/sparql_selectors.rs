// Internal SPARQL selectors for the embedded Oxigraph authority. These helpers
// return backend-neutral IDs/refs; canonical object hydration reads RDF state.
use std::collections::HashSet;

use oxigraph::model::Term;
use oxigraph::sparql::{QueryResults, QuerySolution, SparqlEvaluator};
use oxigraph::store::Store;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::domain::{
    graph_uri, MemoryId, MemoryObjectRef, ObjectType, RelationType, RetentionState,
};
use crate::errors::CustomError;
use crate::policy::graph_expansion::{ParticipantOccasion, ParticipantOccasions};
use crate::ports::graph_authority::{
    GraphDerivedMemoryProvenanceQuery, GraphDerivedMemoryThreadQuery, GraphExpansionFilteredNode,
    GraphExpansionFilteredReason, GraphExpansionLifecyclePolicy, GraphObjectQuery,
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
        if query.is_empty() {
            return Ok(Vec::new());
        }

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

    #[allow(
        dead_code,
        reason = "the scene slice will consume exact-name notion cues"
    )]
    pub(crate) fn select_notions_known_as(&self, name: &str) -> Result<Vec<MemoryId>, CustomError> {
        let normalized = crate::domain::belief::normalize_name(name);
        if normalized.is_empty() {
            return Ok(Vec::new());
        }
        let name_value = sparql_literal_values("name", [normalized].into_iter());
        let query = format!(
            r#"
            SELECT DISTINCT ?id WHERE {{
              {name_value}
              GRAPH ?beliefGraph {{
                ?belief a <{derived_class}> ; <{retention}> "active" ; <{assertion}> ?assertion .
                ?assertion <{predicate}> "known_as" ; <{normalized_name}> ?name ; <{subject}> ?notion .
              }}
              GRAPH ?notionGraph {{ ?notion a <{entity_class}> ; <{object_id}> ?id . }}
              FILTER NOT EXISTS {{
                GRAPH ?linkGraph {{
                  ?link a <{link_class}> ; <{from_type}> "derived_memory" ;
                    <{to_type}> "derived_memory" ; <{relation}> "supersedes" ; <{to}> ?belief .
                }}
              }}
            }}
        "#,
            derived_class = vocab::CLASS_DERIVED_MEMORY,
            retention = vocab::RETENTION_STATE,
            assertion = vocab::ASSERTION,
            predicate = vocab::ASSERTION_PREDICATE,
            normalized_name = vocab::NORMALIZED_NAME,
            subject = vocab::ASSERTION_SUBJECT,
            entity_class = vocab::CLASS_ENTITY,
            object_id = vocab::OBJECT_ID,
            link_class = vocab::CLASS_MEMORY_LINK,
            from_type = vocab::FROM_TYPE,
            to_type = vocab::TO_TYPE,
            relation = vocab::RELATION,
            to = vocab::TO,
        );
        self.select_memory_ids(&query, None)
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

    // Do not LIMIT here: own-subject metadata can exist without a traversable About
    // link. Limiting before that intersection can leave the state bucket underfilled.
    pub(crate) fn select_subject_state(
        &self,
        subject_id: MemoryId,
        policy: GraphExpansionLifecyclePolicy,
    ) -> Result<(Vec<MemoryId>, Vec<GraphExpansionFilteredNode>), CustomError> {
        let subject = graph_uri(ObjectType::Entity, subject_id);
        self.select_state(&format!("<{}> <{subject}>", vocab::ABOUT_ENTITY), policy)
    }

    pub(crate) fn select_scope_state(
        &self,
        key: &crate::domain::ScopeKey,
        policy: GraphExpansionLifecyclePolicy,
    ) -> Result<(Vec<MemoryId>, Vec<GraphExpansionFilteredNode>), CustomError> {
        let value = serde_json::to_string(key).expect("scope keys contain strings only");
        let predicate = format!(
            "<{}> {}",
            vocab::SCOPE_KEY,
            oxigraph::model::Literal::new_simple_literal(value)
        );
        self.select_state(&predicate, policy)
    }

    fn select_state(
        &self,
        scope_predicate: &str,
        policy: GraphExpansionLifecyclePolicy,
    ) -> Result<(Vec<MemoryId>, Vec<GraphExpansionFilteredNode>), CustomError> {
        let query = format!(
            r#"
            PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
            SELECT ?id ?retention ?successor WHERE {{
              GRAPH ?g {{
                ?memory a <{derived_class}> ; <{object_id}> ?id ;
                  {scope_predicate} ; <{salience}> ?salience ;
                  <{created}> ?created ; <{retention}> ?retention .
              }}
              OPTIONAL {{
                GRAPH ?linkGraph {{
                  ?link a <{link_class}> ; <{from_type}> "derived_memory" ;
                    <{to_type}> "derived_memory" ; <{relation}> "supersedes" ;
                    <{from}> ?successor ; <{to}> ?memory .
                }}
              }}
            }}
            ORDER BY BOUND(?successor) DESC(xsd:double(?salience)) DESC(xsd:dateTime(?created)) ?id ?successor
            "#,
            derived_class = vocab::CLASS_DERIVED_MEMORY,
            object_id = vocab::OBJECT_ID,
            salience = vocab::SALIENCE_SCORE,
            created = vocab::CREATED_AT,
            retention = vocab::RETENTION_STATE,
            link_class = vocab::CLASS_MEMORY_LINK,
            from_type = vocab::FROM_TYPE,
            to_type = vocab::TO_TYPE,
            relation = vocab::RELATION,
            from = vocab::FROM,
            to = vocab::TO,
        );
        let mut ranked_ids = Vec::new();
        let mut filtered = Vec::<GraphExpansionFilteredNode>::new();
        for solution in self.query_solutions(&query)? {
            let id = memory_id_binding(&solution, "id")?;
            let successor = match solution.get("successor") {
                Some(Term::NamedNode(node)) => {
                    Some(super::shared::memory_id_from_resource(node.as_str())?)
                }
                Some(value) => {
                    return Err(oxigraph_sparql_error(format!(
                        "expected successor IRI, got {value}"
                    )))
                }
                None => None,
            };
            let reason = if enum_binding::<RetentionState>(&solution, "retention")?
                == RetentionState::Suppressed
                && !policy.include_suppressed
            {
                Some(GraphExpansionFilteredReason::Suppressed)
            } else if successor.is_some() && !policy.include_superseded {
                Some(GraphExpansionFilteredReason::Superseded)
            } else {
                None
            };
            if let Some(reason) = reason {
                if filtered
                    .last()
                    .is_none_or(|entry| entry.object_ref.id != id)
                {
                    filtered.push(GraphExpansionFilteredNode {
                        object_ref: MemoryObjectRef::new(ObjectType::DerivedMemory, id),
                        reason,
                        superseded_by: Vec::new(),
                    });
                }
                if let Some(successor) = successor {
                    let successors = &mut filtered.last_mut().unwrap().superseded_by;
                    if successors.last() != Some(&successor) {
                        successors.push(successor);
                    }
                }
            } else if ranked_ids.last() != Some(&id) {
                ranked_ids.push(id);
            }
        }
        Ok((ranked_ids, filtered))
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
                link_ref.to.object_type.stable_rank(),
                link_ref.from.object_type.stable_rank(),
                link_ref.relation.stable_rank(),
            )
        });
        Ok(refs)
    }

    pub(crate) fn select_participant_occasions(
        &self,
        neighbors: &[MemoryObjectRef],
    ) -> Result<ParticipantOccasions, CustomError> {
        if neighbors.is_empty() {
            return Ok(ParticipantOccasions::new());
        }
        let node_values = sparql_node_iri_values("node", neighbors);
        let query_text = format!(
            r#"
            SELECT DISTINCT ?id ?objectType ?episodeId ?sceneTime ?retention ?episodeRetention WHERE {{
              {node_values}
              GRAPH ?node {{ ?node <{object_id}> ?id ; <{object_type}> ?objectType ; <{retention}> ?retention . }}
              {{
                GRAPH ?node {{ ?node <{object_id}> ?episodeId ; <{scene_time}> ?sceneTime ; <{retention}> ?episodeRetention . }}
              }} UNION {{
                GRAPH ?node {{ ?node <{episode}> ?episode . }}
                GRAPH ?episode {{ ?episode <{object_id}> ?episodeId ; <{scene_time}> ?sceneTime ; <{retention}> ?episodeRetention . }}
              }}
            }}
        "#,
            object_id = vocab::OBJECT_ID,
            object_type = vocab::OBJECT_TYPE,
            scene_time = vocab::SCENE_TIME,
            episode = vocab::EPISODE,
            retention = vocab::RETENTION_STATE,
        );
        let mut occasions = ParticipantOccasions::new();
        for solution in self.query_solutions(&query_text)? {
            let neighbor = MemoryObjectRef::from_id_type(
                memory_id_binding(&solution, "id")?,
                enum_binding(&solution, "objectType")?,
            );
            let time = literal_binding(&solution, "sceneTime")?
                .parse()
                .map_err(|error| {
                    CustomError::DatabaseError(format!(
                        "Oxigraph SPARQL invalid Scene.time: {error}"
                    ))
                })?;
            occasions.insert(
                neighbor,
                ParticipantOccasion {
                    episode_id: memory_id_binding(&solution, "episodeId")?,
                    time,
                    retention_state: enum_binding(&solution, "retention")?,
                    episode_retention_state: enum_binding(&solution, "episodeRetention")?,
                },
            );
        }
        Ok(occasions)
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

fn oxigraph_sparql_error(error: impl std::fmt::Display) -> CustomError {
    CustomError::DatabaseError(format!("Oxigraph SPARQL selector error: {error}"))
}
