// Internal SPARQL selectors for the embedded Oxigraph authority. These helpers
// return backend-neutral IDs/refs; canonical object hydration reads RDF state.
use std::collections::HashSet;

use chrono::{DateTime, Utc};
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

    pub(crate) fn select_subject_state(
        &self,
        subject_id: MemoryId,
        policy: GraphExpansionLifecyclePolicy,
        limit: usize,
        read_more: bool,
    ) -> Result<(Vec<MemoryId>, Vec<GraphExpansionFilteredNode>), CustomError> {
        let subject = graph_uri(ObjectType::Entity, subject_id);
        let scope = format!(
            r#"
            GRAPH ?memory {{ ?memory <{about}> <{subject}> . }}
            GRAPH ?aboutLink {{
                ?aboutLink a <{link_class}> ; <{relation}> "about" .
                {{ ?aboutLink <{from}> ?memory ; <{to}> <{subject}> ; <{from_type}> "derived_memory" ; <{to_type}> "entity" . }}
                UNION {{ ?aboutLink <{to}> ?memory ; <{from}> <{subject}> ; <{to_type}> "derived_memory" ; <{from_type}> "entity" . }}
            }}
        "#,
            about = vocab::ABOUT_ENTITY,
            link_class = vocab::CLASS_MEMORY_LINK,
            relation = vocab::RELATION,
            from = vocab::FROM,
            to = vocab::TO,
            from_type = vocab::FROM_TYPE,
            to_type = vocab::TO_TYPE
        );
        self.select_state(
            &scope,
            policy,
            limit,
            read_more,
            "?superseded DESC(xsd:double(?salience)) DESC(xsd:dateTime(?created)) ?id",
        )
    }

    pub(crate) fn select_scope_state(
        &self,
        key: &crate::domain::ScopeKey,
        policy: GraphExpansionLifecyclePolicy,
        limit: usize,
    ) -> Result<(Vec<MemoryId>, Vec<GraphExpansionFilteredNode>), CustomError> {
        let value = serde_json::to_string(key).expect("scope keys contain strings only");
        let scope = format!(
            "GRAPH ?memory {{ ?memory <{}> {} . }}",
            vocab::SCOPE_KEY,
            oxigraph::model::Literal::new_simple_literal(value)
        );
        self.select_state(
            &scope,
            policy,
            limit,
            false,
            "?superseded DESC(xsd:double(?salience)) DESC(xsd:dateTime(?created)) ?id",
        )
    }

    pub(crate) fn select_thread_state(
        &self,
        query: &GraphDerivedMemoryThreadQuery,
        limit: usize,
    ) -> Result<(Vec<MemoryId>, Vec<GraphExpansionFilteredNode>), CustomError> {
        if query.thread_ids.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        let threads = query
            .thread_ids
            .iter()
            .map(|id| graph_uri(ObjectType::MemoryThread, *id))
            .collect::<Vec<_>>();
        let scope = format!(
            "{} GRAPH ?memory {{ ?memory <{}> ?thread . }}",
            sparql_iri_values("thread", threads.iter().map(String::as_str)),
            vocab::PART_OF_THREAD
        );
        self.select_state(
            &scope,
            query.lifecycle_policy,
            limit,
            false,
            "DESC(xsd:dateTime(?created)) ?id",
        )
    }

    fn select_state(
        &self,
        scope: &str,
        policy: GraphExpansionLifecyclePolicy,
        limit: usize,
        read_more: bool,
        order: &str,
    ) -> Result<(Vec<MemoryId>, Vec<GraphExpansionFilteredNode>), CustomError> {
        let eligible_limit = limit.saturating_add(usize::from(read_more));
        if eligible_limit == 0 {
            return Ok((Vec::new(), Vec::new()));
        }
        // Join lifecycle links once; per-memory EXISTS repeats the same link scan.
        let body = format!(
            r#"
              {scope}
              GRAPH ?memory {{
                ?memory a <{derived_class}> ; <{object_id}> ?id ;
                  <{salience}> ?salience ;
                  <{created}> ?created ; <{retention}> ?retention .
              }}
              OPTIONAL {{
                GRAPH ?supersessionLink {{
                  ?supersessionLink a <{link_class}> ; <{from_type}> "derived_memory" ;
                    <{to_type}> "derived_memory" ; <{relation}> "supersedes" ; <{to}> ?memory .
                }}
              }}
              BIND(BOUND(?supersessionLink) AS ?superseded)
              OPTIONAL {{
                GRAPH ?resolution {{
                  ?resolution a <{link_class}> ; <{from_type}> "derived_memory" ;
                    <{to_type}> "derived_memory" ; <{relation}> ?resolutionKind ; <{to}> ?memory .
                  VALUES ?resolutionKind {{ "resolves" "fulfills_commitment" }}
                }}
              }}
              BIND(BOUND(?resolution) AS ?resolved)
              BIND(({include_suppressed} || ?retention != "suppressed") &&
                   ({include_superseded} || !?superseded) && !?resolved AS ?eligible)
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
            to = vocab::TO,
            include_suppressed = policy.include_suppressed,
            include_superseded = policy.include_superseded,
        );
        let prefix = "PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>";
        let ranked_ids = self
            .query_solutions(&format!(
                r#"{prefix}
            SELECT DISTINCT ?id ?superseded ?salience ?created WHERE {{ {body} FILTER(?eligible) }}
            ORDER BY {order} LIMIT {eligible_limit}
        "#
            ))?
            .iter()
            .map(|row| memory_id_binding(row, "id"))
            .collect::<Result<Vec<_>, _>>()?;
        if limit == 0 {
            return Ok((ranked_ids, Vec::new()));
        }
        // Exclusion evidence has its own latest-first budget; it cannot crowd out eligible state.
        let excluded = format!(
            r#"{prefix}
            SELECT ?id ?retention ?created (GROUP_CONCAT(DISTINCT COALESCE(STR(?source), ""); SEPARATOR=" ") AS ?sources) WHERE {{
                {{ SELECT DISTINCT ?memory ?id ?retention ?created WHERE {{ {body} FILTER(!?eligible) }}
                   ORDER BY DESC(xsd:dateTime(?created)) ?id LIMIT {limit} }}
                OPTIONAL {{ GRAPH ?supersession {{
                    ?supersession a <{link_class}> ; <{from_type}> "derived_memory" ;
                        <{to_type}> "derived_memory" ; <{relation}> "supersedes" ; <{from}> ?source ; <{to}> ?memory .
                }} }}
            }} GROUP BY ?id ?retention ?created
            ORDER BY DESC(xsd:dateTime(?created)) ?id LIMIT {limit}
        "#,
            link_class = vocab::CLASS_MEMORY_LINK,
            from_type = vocab::FROM_TYPE,
            to_type = vocab::TO_TYPE,
            relation = vocab::RELATION,
            from = vocab::FROM,
            to = vocab::TO
        );
        let mut filtered = Vec::new();
        for row in self.query_solutions(&excluded)? {
            let id = memory_id_binding(&row, "id")?;
            let retention = enum_binding::<RetentionState>(&row, "retention")?;
            let mut superseded_by = literal_binding(&row, "sources")?
                .split_whitespace()
                .map(super::shared::memory_id_from_resource)
                .collect::<Result<Vec<_>, _>>()?;
            superseded_by.sort_unstable();
            let reason = if retention == RetentionState::Suppressed && !policy.include_suppressed {
                GraphExpansionFilteredReason::Suppressed
            } else if !superseded_by.is_empty() && !policy.include_superseded {
                GraphExpansionFilteredReason::Superseded
            } else {
                GraphExpansionFilteredReason::Resolved
            };
            filtered.push(GraphExpansionFilteredNode {
                object_ref: MemoryObjectRef::new(ObjectType::DerivedMemory, id),
                reason,
                superseded_by,
            });
        }
        Ok((ranked_ids, filtered))
    }

    pub(crate) fn select_links_touching(
        &self,
        object_refs: &[MemoryObjectRef],
        current_thread_state: bool,
    ) -> Result<Vec<SparqlLinkRef>, CustomError> {
        if object_refs.is_empty() {
            return Ok(Vec::new());
        }

        let node_values = sparql_node_iri_values("node", object_refs);
        let thread_filter = if current_thread_state {
            format!(
                r#"
                BIND(IF(?fromType = "derived_memory", ?from, ?to) AS ?member)
                FILTER(?relation != "part_of_thread" || NOT EXISTS {{ GRAPH ?resolution {{
                    ?resolution a <{link_class}> ; <{from_type}> "derived_memory" ;
                        <{to_type}> "derived_memory" ; <{relation}> ?resolutionKind ; <{to}> ?member .
                    VALUES ?resolutionKind {{ "resolves" "fulfills_commitment" }}
                }} }})
            "#,
                link_class = vocab::CLASS_MEMORY_LINK,
                from_type = vocab::FROM_TYPE,
                to_type = vocab::TO_TYPE,
                relation = vocab::RELATION,
                to = vocab::TO
            )
        } else {
            String::new()
        };
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
              {thread_filter}
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

    pub(crate) fn select_last_interaction(
        &self,
        participant: MemoryId,
        reference_time: DateTime<Utc>,
        policy: GraphExpansionLifecyclePolicy,
    ) -> Result<Option<(MemoryId, DateTime<Utc>)>, CustomError> {
        let query_text = format!(
            r#"
            PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
            SELECT DISTINCT ?episodeId ?sceneTime ?time WHERE {{
              GRAPH ?linkGraph {{
                ?link a <{link_class}> .
                {{ ?link <{from}> <{participant}> ; <{to}> ?neighbor . }}
                UNION {{ ?link <{to}> <{participant}> ; <{from}> ?neighbor . }}
              }}
              {{
                GRAPH ?linkGraph {{ ?link <{relation}> "involves" . }}
                GRAPH ?neighbor {{ ?neighbor <{object_type}> "episode" ; <{retention}> ?retention . }}
                BIND(?neighbor AS ?episode)
              }} UNION {{
                GRAPH ?linkGraph {{ ?link <{relation}> "mentions" . }}
                GRAPH ?neighbor {{ ?neighbor <{object_type}> "observation" ; <{episode}> ?episode ; <{retention}> ?retention . }}
              }}
              GRAPH ?episode {{ ?episode <{object_type}> "episode" ; <{object_id}> ?episodeId ; <{scene_time}> ?sceneTime ; <{retention}> ?episodeRetention . }}
              BIND(xsd:dateTime(?sceneTime) AS ?time)
              FILTER(?time <= {reference_time}^^xsd:dateTime)
              {retention_filter}
            }}
            ORDER BY DESC(?time) ?episodeId
            LIMIT 1
            "#,
            participant = graph_uri(ObjectType::Entity, participant),
            link_class = vocab::CLASS_MEMORY_LINK,
            from = vocab::FROM,
            to = vocab::TO,
            relation = vocab::RELATION,
            object_type = vocab::OBJECT_TYPE,
            object_id = vocab::OBJECT_ID,
            scene_time = vocab::SCENE_TIME,
            episode = vocab::EPISODE,
            retention = vocab::RETENTION_STATE,
            reference_time = sparql_string_literal(&reference_time.to_rfc3339()),
            retention_filter = occasion_retention_filter(policy),
        );
        self.query_solutions(&query_text)?
            .into_iter()
            .next()
            .map(|solution| {
                let time = literal_binding(&solution, "sceneTime")?
                    .parse()
                    .map_err(|error| {
                        CustomError::DatabaseError(format!(
                            "Oxigraph SPARQL invalid Scene.time: {error}"
                        ))
                    })?;
                Ok((memory_id_binding(&solution, "episodeId")?, time))
            })
            .transpose()
    }

    pub(crate) fn select_participant_occasions(
        &self,
        neighbors: &[MemoryObjectRef],
    ) -> Result<ParticipantOccasions, CustomError> {
        if neighbors.is_empty() {
            return Ok(ParticipantOccasions::new());
        }
        let query_text = format!("SELECT DISTINCT ?id ?objectType ?episodeId ?sceneTime ?retention ?episodeRetention WHERE {{ {} }}", participant_occasion_pattern(neighbors));
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

    pub(crate) fn select_bounded_participant_occasions(
        &self,
        neighbors: &[MemoryObjectRef],
        query: &crate::ports::graph_authority::GraphExpansionQuery,
    ) -> Result<ParticipantOccasions, CustomError> {
        let budget = query
            .fanout_overrides
            .iter()
            .filter(|entry| {
                crate::policy::graph_expansion::is_participant_pair(
                    entry.relation,
                    entry.object_type,
                )
            })
            .map(|entry| entry.max_fanout)
            .max()
            .unwrap_or(0)
            .min(query.max_fanout_per_node);
        if neighbors.is_empty() || budget == 0 {
            return Ok(ParticipantOccasions::new());
        }
        let prefix = "PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>";
        let cutoff = if query.current_subject_state {
            format!(
                "FILTER(?time <= {}^^xsd:dateTime)",
                sparql_string_literal(&query.participant_reference_time.to_rfc3339())
            )
        } else {
            String::new()
        };
        let body = format!(
            r#"{}
            BIND(xsd:dateTime(?sceneTime) AS ?time) {cutoff}
            BIND(({} || (?retention = "active" && ?episodeRetention = "active")) AS ?eligible)
        "#,
            participant_occasion_pattern(neighbors),
            query.lifecycle_policy.include_suppressed
        );
        let mut selected = Vec::new();
        for eligible in [true, false] {
            let limit =
                budget.saturating_add(usize::from(eligible && query.trace_mode.is_enabled()));
            // Reserve distinct occasions before choosing one stable route of each type.
            let episodes = self
                .query_solutions(&format!(
                    r#"{prefix}
                SELECT DISTINCT ?episodeId ?time WHERE {{ {body} FILTER(?eligible = {eligible}) }}
                ORDER BY DESC(?time) ?episodeId LIMIT {limit}
            "#
                ))?
                .iter()
                .map(|row| memory_id_binding(row, "episodeId"))
                .collect::<Result<Vec<_>, _>>()?;
            if episodes.is_empty() {
                continue;
            }
            let values =
                sparql_literal_values("episodeId", episodes.iter().map(ToString::to_string));
            for row in self.query_solutions(&format!(
                r#"{prefix}
                SELECT ?episodeId ?objectType (MIN(?rank) AS ?selectedRank) WHERE {{
                    {body} {values} FILTER(?eligible = {eligible})
                }} GROUP BY ?episodeId ?objectType
            "#
            ))? {
                let rank = literal_binding(&row, "selectedRank")?
                    .parse::<usize>()
                    .map_err(oxigraph_sparql_error)?;
                selected.push(neighbors[rank]);
            }
        }
        self.select_participant_occasions(&selected)
    }

    pub(crate) fn select_anniversaries(
        &self,
        date: chrono::NaiveDate,
        participants: &[MemoryId],
        limit: usize,
        policy: GraphExpansionLifecyclePolicy,
    ) -> Result<Vec<(MemoryId, bool)>, CustomError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let shared = if participants.is_empty() {
            "false".to_owned()
        } else {
            let values = participants
                .iter()
                .map(|id| format!("<{}>", graph_uri(ObjectType::Entity, *id)))
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                r#"EXISTS {{
                VALUES ?participant {{ {values} }}
                {{
                    GRAPH ?linkGraph {{
                        ?link a <{link_class}> ; <{relation}> "involves" .
                        {{ ?link <{from}> ?participant ; <{to}> ?episode . }}
                        UNION {{ ?link <{to}> ?participant ; <{from}> ?episode . }}
                    }}
                }} UNION {{
                    GRAPH ?linkGraph {{
                        ?link a <{link_class}> ; <{relation}> "mentions" .
                        {{ ?link <{from}> ?participant ; <{to}> ?neighbor . }}
                        UNION {{ ?link <{to}> ?participant ; <{from}> ?neighbor . }}
                    }}
                    GRAPH ?neighbor {{ ?neighbor <{object_type}> "observation" ; <{episode_pred}> ?episode ; <{retention}> ?neighborRetention . }}
                    {retention_filter}
                }}
            }}"#,
                link_class = vocab::CLASS_MEMORY_LINK,
                from = vocab::FROM,
                to = vocab::TO,
                relation = vocab::RELATION,
                object_type = vocab::OBJECT_TYPE,
                episode_pred = vocab::EPISODE,
                retention = vocab::RETENTION_STATE,
                retention_filter =
                    occasion_retention_filter(policy).replace("?retention", "?neighborRetention")
            )
        };
        let query = format!(
            r#"
            PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
            SELECT DISTINCT ?episodeId ?time ?shared WHERE {{
                GRAPH ?episode {{
                    ?episode <{object_type}> "episode" ; <{object_id}> ?episodeId ;
                        <{month_day}> {month_day_value} ; <{local_year}> ?localYear ;
                        <{scene_time}> ?sceneTime ; <{retention}> ?episodeRetention .
                }}
                FILTER(xsd:integer(?localYear) < {year})
                BIND(?episodeRetention AS ?retention)
                {retention_filter}
                BIND(xsd:dateTime(?sceneTime) AS ?time)
                BIND({shared} AS ?shared)
            }}
            ORDER BY DESC(?shared) DESC(?time) ?episodeId
            LIMIT {limit}
        "#,
            object_type = vocab::OBJECT_TYPE,
            object_id = vocab::OBJECT_ID,
            month_day = vocab::SCENE_MONTH_DAY,
            local_year = vocab::SCENE_LOCAL_YEAR,
            month_day_value = sparql_string_literal(&date.format("%m-%d").to_string()),
            year = chrono::Datelike::year(&date),
            scene_time = vocab::SCENE_TIME,
            retention = vocab::RETENTION_STATE,
            retention_filter = occasion_retention_filter(policy)
        );
        self.query_solutions(&query)?
            .iter()
            .map(|row| {
                Ok((
                    memory_id_binding(row, "episodeId")?,
                    literal_binding(row, "shared")? == "true",
                ))
            })
            .collect()
    }

    pub(crate) fn select_episodes_by_time(
        &self,
        start: Option<DateTime<Utc>>,
        end: DateTime<Utc>,
        limit: usize,
        policy: GraphExpansionLifecyclePolicy,
    ) -> Result<Vec<MemoryId>, CustomError> {
        if start.is_some_and(|start| start > end) || limit == 0 {
            return Ok(Vec::new());
        }
        let pattern = format!(
            r#"
            GRAPH ?episode {{ ?episode <{object_type}> "episode" ; <{object_id}> ?episodeId ; <{scene_time}> ?sceneTime ; <{retention}> ?episodeRetention . }}
            BIND(?episodeRetention AS ?retention)
            BIND(xsd:dateTime(?sceneTime) AS ?time)
            FILTER(?time <= {end}^^xsd:dateTime)
            {start_filter}
            {retention_filter}
        "#,
            object_type = vocab::OBJECT_TYPE,
            object_id = vocab::OBJECT_ID,
            scene_time = vocab::SCENE_TIME,
            retention = vocab::RETENTION_STATE,
            end = sparql_string_literal(&end.to_rfc3339()),
            start_filter = start.map_or_else(String::new, |start| format!(
                "FILTER(?time >= {}^^xsd:dateTime)",
                sparql_string_literal(&start.to_rfc3339())
            )),
            retention_filter = occasion_retention_filter(policy),
        );
        // ponytail: the typed date needs a store-side scan/sort; add a chronological
        // index only if measured recall latency warrants it. Payload reads stay bounded.
        let episodes = self
            .query_solutions(&format!(
                r#"
            PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
            SELECT DISTINCT ?episodeId ?time WHERE {{ {pattern} }}
            ORDER BY DESC(?time) ?episodeId
            LIMIT {limit}
        "#,
            ))?
            .iter()
            .map(|solution| memory_id_binding(solution, "episodeId"))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(episodes)
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

        let rows = solutions
            .collect::<Result<Vec<_>, _>>()
            .map_err(oxigraph_sparql_error)?;
        #[cfg(test)]
        MAX_SELECT_ROWS.with(|count| count.set(count.get().max(rows.len())));
        Ok(rows)
    }
}

#[cfg(test)]
thread_local! {
    pub(super) static MAX_SELECT_ROWS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn participant_occasion_pattern(neighbors: &[MemoryObjectRef]) -> String {
    let values = neighbors
        .iter()
        .enumerate()
        .map(|(rank, object)| format!("(<{}> {rank})", graph_uri(object.object_type, object.id)))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        r#"
        VALUES (?node ?rank) {{ {values} }}
        GRAPH ?node {{ ?node <{object_id}> ?id ; <{object_type}> ?objectType ; <{retention}> ?retention . }}
        {{
            GRAPH ?node {{ ?node <{object_id}> ?episodeId ; <{scene_time}> ?sceneTime ; <{retention}> ?episodeRetention . }}
        }} UNION {{
            GRAPH ?node {{ ?node <{episode}> ?episode . }}
            GRAPH ?episode {{ ?episode <{object_id}> ?episodeId ; <{scene_time}> ?sceneTime ; <{retention}> ?episodeRetention . }}
        }}
    "#,
        object_id = vocab::OBJECT_ID,
        object_type = vocab::OBJECT_TYPE,
        retention = vocab::RETENTION_STATE,
        scene_time = vocab::SCENE_TIME,
        episode = vocab::EPISODE
    )
}

fn occasion_retention_filter(policy: GraphExpansionLifecyclePolicy) -> &'static str {
    if policy.include_suppressed {
        ""
    } else {
        "FILTER(?retention = \"active\" && ?episodeRetention = \"active\")"
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
