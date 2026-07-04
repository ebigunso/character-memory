#[allow(clippy::module_inception)]
mod tests {
    use std::collections::HashSet;

    use super::super::embedded::*;
    use super::super::http::*;
    use super::super::rdf_mapping::{
        rdf_triples_for_link, rdf_triples_for_object, RdfObject, RdfTriple,
    };
    use super::super::shared::*;
    use super::super::sparql_selectors::SparqlGraphSelectors;
    use super::super::vocabulary as vocab;
    use super::super::*;
    use crate::api::types::{
        graph_uri, ContextPackSection, LifecycleFilterAction, LifecycleFilterReason, MemoryId,
        MemoryLink, MemoryObject, ObjectType, RelationType, RetentionState, RetrievalContext,
        ThreadStatus,
    };
    use crate::models::vector::{
        EmbeddingInput, VectorCandidateMatch, VectorCandidateSearch, VectorRecordEmbedding,
        VectorSurface,
    };
    use crate::policy::memory_object_vector_record;
    use crate::ports::embedder::MemoryEmbedder;
    use crate::ports::graph_authority::{
        GraphAuthorityStore, GraphDerivedMemoryProvenanceQuery, GraphDerivedMemoryThreadQuery,
        GraphExpansionBoundedFailureReason, GraphExpansionFailurePolicy,
        GraphExpansionFanoutOverride, GraphExpansionFilteredReason, GraphExpansionLifecyclePolicy,
        GraphExpansionQuery, GraphObjectQuery, GraphObjectRef,
    };
    use crate::ports::vector_candidate::VectorCandidateStore;
    use crate::test_support::{high_fanout_graph_fixture, representative_fixtures};
    use crate::usecases::RetrievePipeline;
    use crate::CustomError;
    use async_trait::async_trait;
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
    async fn oxigraph_visibility_applies_node_cap_before_hydration() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixture = high_fanout_graph_fixture();

        store.upsert_objects(&fixture.objects()).await.unwrap();
        store.upsert_links(&fixture.links).await.unwrap();

        let query = GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 5)
            .with_allowed_object_types(vec![ObjectType::DerivedMemory])
            .with_max_fanout_per_node(20);
        let selectors = SparqlGraphSelectors::new(&store.store);

        let visibility = bounded_graph_visible_refs(
            &selectors,
            GraphObjectRef::new(fixture.hub_entity.id, ObjectType::Entity),
            &query,
        )
        .unwrap();

        assert_eq!(visibility.object_refs.len(), 5);
        assert_eq!(visibility.traversal_link_ids.len(), 4);
        assert_eq!(
            visibility
                .object_refs
                .iter()
                .filter(|object_ref| object_ref.object_type == ObjectType::DerivedMemory)
                .count(),
            4
        );
        assert_eq!(
            visibility.bounded_failure.unwrap().reason,
            GraphExpansionBoundedFailureReason::NodeLimit
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
        ) -> Result<Vec<crate::models::vector::VectorCandidateDiagnosticRecord>, CustomError>
        {
            Ok(Vec::new())
        }

        async fn delete_candidates(&self, _object_ids: &[MemoryId]) -> Result<(), CustomError> {
            Ok(())
        }
    }
}
