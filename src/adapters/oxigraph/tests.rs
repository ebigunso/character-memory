#[allow(clippy::module_inception)]
mod tests {
    use super::super::embedded::*;
    use crate::domain::{
        MemoryId, MemoryObject, MemoryObjectRef, ObjectType, RelationType, RetentionState,
        ThreadStatus,
    };
    use crate::ports::graph_authority::{
        GraphAuthorityStore, GraphDerivedMemoryProvenanceQuery, GraphDerivedMemoryThreadQuery,
        GraphExpansionBoundedFailureReason, GraphExpansionFailurePolicy,
        GraphExpansionFanoutOverride, GraphExpansionFilteredReason, GraphExpansionLifecyclePolicy,
        GraphExpansionQuery, GraphObjectQuery,
    };
    use crate::test_support::{high_fanout_graph_fixture, representative_fixtures};
    use crate::CustomError;
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
        for object in fixtures.objects() {
            assert_eq!(
                store
                    .query_objects(&GraphObjectQuery::by_refs(vec![object.object_ref()]))
                    .await
                    .unwrap(),
                vec![object],
            );
        }
    }

    #[tokio::test]
    async fn oxigraph_round_trips_subsecond_object_and_link_timestamps() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let mut fixtures = representative_fixtures();
        let timestamp = chrono::DateTime::parse_from_rfc3339("2026-09-14T07:00:00.123456789Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        fixtures.episode.created_at = timestamp;
        fixtures.episode.started_at = Some(timestamp);
        fixtures.episode.ended_at = Some(timestamp);
        fixtures.salient_observation.created_at = timestamp;
        fixtures.salient_observation.observed_at = Some(timestamp);
        let mut link = fixtures.soft_thread_link.clone();
        link.created_at = timestamp;
        store
            .upsert_objects_and_links(&fixtures.objects(), std::slice::from_ref(&link))
            .await
            .unwrap();

        let objects = store
            .query_objects(&GraphObjectQuery::by_ids(vec![
                fixtures.episode.id,
                fixtures.salient_observation.id,
            ]))
            .await
            .unwrap();
        let links = store.query_links_by_ids(&[link.id]).await.unwrap();
        assert_eq!(
            (objects, links),
            (
                vec![
                    MemoryObject::Episode(fixtures.episode),
                    MemoryObject::Observation(fixtures.salient_observation),
                ],
                vec![link],
            )
        );
    }

    #[tokio::test]
    async fn oxigraph_object_selection_obeys_predicates_ordering_and_limits() {
        let oxigraph = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let episode = MemoryObject::Episode(fixtures.episode.clone());
        let observation = MemoryObject::Observation(fixtures.salient_observation.clone());
        let entity = MemoryObject::Entity(fixtures.user_entity.clone());
        let mut colliding_entity = fixtures.project_entity.clone();
        colliding_entity.id = episode.id();
        let colliding_entity = MemoryObject::Entity(colliding_entity);
        // Input order differs from the contract's ID-then-type order, including an ID tie.
        let objects = vec![
            observation.clone(),
            colliding_entity.clone(),
            episode.clone(),
            entity.clone(),
        ];
        let ordered_objects = vec![
            entity.clone(),
            episode.clone(),
            colliding_entity.clone(),
            observation.clone(),
        ];
        let unknown_id = MemoryId::from_u128(u128::MAX);

        oxigraph.upsert_objects(&objects).await.unwrap();

        let cases = vec![
            (
                "empty refs",
                GraphObjectQuery::by_refs(Vec::new()),
                Vec::new(),
            ),
            (
                "refs match both identifier and type",
                GraphObjectQuery::by_refs(vec![episode.object_ref()]),
                vec![episode.clone()],
            ),
            (
                "mixed-type refs preserve identifier/type pairs",
                GraphObjectQuery::by_refs(vec![episode.object_ref(), entity.object_ref()]),
                vec![entity.clone(), episode.clone()],
            ),
            (
                "refs order by identifier then type rank",
                GraphObjectQuery::by_refs(objects.iter().map(MemoryObject::object_ref).collect()),
                ordered_objects.clone(),
            ),
            (
                "unknown refs",
                GraphObjectQuery::by_refs(vec![MemoryObjectRef::new(
                    ObjectType::Episode,
                    unknown_id,
                )]),
                Vec::new(),
            ),
            (
                "known identifier with wrong reference type",
                GraphObjectQuery::by_refs(vec![MemoryObjectRef::new(
                    ObjectType::Entity,
                    observation.id(),
                )]),
                Vec::new(),
            ),
            (
                "empty ids",
                GraphObjectQuery::by_ids(Vec::new()),
                Vec::new(),
            ),
            (
                "ids select every matching type",
                GraphObjectQuery::by_ids(vec![episode.id()]),
                vec![episode.clone(), colliding_entity.clone()],
            ),
            (
                "ids order by identifier then type rank",
                GraphObjectQuery::by_ids(vec![observation.id(), episode.id(), entity.id()]),
                ordered_objects,
            ),
            (
                "unknown ids",
                GraphObjectQuery::by_ids(vec![unknown_id]),
                Vec::new(),
            ),
            (
                "empty types",
                GraphObjectQuery::by_types(Vec::new(), Some(1)),
                Vec::new(),
            ),
            (
                "types filter and order by identifier then type rank",
                GraphObjectQuery::by_types(vec![ObjectType::Episode, ObjectType::Entity], None),
                vec![entity.clone(), episode.clone(), colliding_entity],
            ),
            (
                "limit follows identifier and type-rank ordering",
                GraphObjectQuery::by_types(vec![ObjectType::Episode, ObjectType::Entity], Some(2)),
                vec![entity, episode.clone()],
            ),
            (
                "type filtering precedes limit",
                GraphObjectQuery::by_types(
                    vec![ObjectType::Episode, ObjectType::Observation],
                    Some(2),
                ),
                vec![episode, observation],
            ),
            (
                "zero limit",
                GraphObjectQuery::by_types(vec![ObjectType::Episode, ObjectType::Entity], Some(0)),
                Vec::new(),
            ),
        ];

        for (label, query, expected) in cases {
            let oxigraph_objects = oxigraph.query_objects(&query).await.unwrap();

            assert_eq!(oxigraph_objects, expected, "Oxigraph {label}");
        }
    }

    #[tokio::test]
    async fn oxigraph_store_queries_only_requested_link_ids_in_canonical_order() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let links = fixtures.links();
        store.upsert_links(&links).await.unwrap();

        let mut expected = vec![links[0].clone(), links[2].clone()];
        expected.sort_by_key(|link| link.id);
        let queried = store
            .query_links_by_ids(&[
                links[2].id,
                MemoryId::from_u128(u128::MAX),
                links[0].id,
                links[2].id,
            ])
            .await
            .unwrap();

        assert_eq!(queried, expected);
        for link in links {
            assert_eq!(
                store.query_links_by_ids(&[link.id]).await.unwrap(),
                vec![link]
            );
        }
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
        assert!(store
            .query_objects(&GraphObjectQuery::by_ids(vec![
                fixtures.episode.id,
                fixtures.salient_observation.id
            ]))
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn oxigraph_upsert_links_rejects_unsupported_schema_before_mutation() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        store.upsert_objects(&fixtures.objects()).await.unwrap();
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
        assert!(store
            .query_objects(&GraphObjectQuery::by_ids(vec![fixtures.episode.id]))
            .await
            .unwrap()
            .is_empty());
        assert!(store
            .query_links_by_ids(&[fixtures.soft_thread_link.id])
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn oxigraph_store_replaces_object_content_on_repeated_upsert() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let mut updated_episode = fixtures.episode.clone();
        updated_episode.summary = "Updated canonical summary.".to_owned();
        store
            .upsert_objects(&[MemoryObject::Episode(fixtures.episode.clone())])
            .await
            .unwrap();
        assert_eq!(
            store
                .query_objects(&GraphObjectQuery::by_ids(vec![fixtures.episode.id]))
                .await
                .unwrap(),
            vec![MemoryObject::Episode(fixtures.episode.clone())],
        );

        store
            .upsert_objects(&[MemoryObject::Episode(updated_episode.clone())])
            .await
            .unwrap();

        assert_eq!(
            store
                .query_objects(&GraphObjectQuery::by_ids(vec![updated_episode.id]))
                .await
                .unwrap(),
            vec![MemoryObject::Episode(updated_episode)]
        );
    }

    #[tokio::test]
    async fn oxigraph_upsert_objects_and_links_replaces_canonical_values() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let mut updated_memory = fixtures.derived_reflection.clone();
        updated_memory.text = "Updated reflection text.".to_owned();
        let mut updated_link = fixtures.soft_thread_link.clone();
        updated_link.to_id = fixtures.episode.id;
        updated_link.to_type = ObjectType::Episode;
        updated_link.relation = RelationType::DerivedFrom;

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store
            .upsert_links(std::slice::from_ref(&fixtures.soft_thread_link))
            .await
            .unwrap();

        store
            .upsert_objects_and_links(
                &[MemoryObject::DerivedMemory(updated_memory.clone())],
                &[updated_link.clone()],
            )
            .await
            .unwrap();

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
    async fn oxigraph_expansion_reports_fanout_omissions() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixture = high_fanout_graph_fixture();

        store.upsert_objects(&fixture.objects()).await.unwrap();
        store.upsert_links(&fixture.links).await.unwrap();

        let query = GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 20)
            .with_allowed_object_types(vec![ObjectType::DerivedMemory])
            .with_max_hub_edges(8)
            .with_max_fanout_per_node(2);
        let without_utilization = store.expand_bounded(&query).await.unwrap();
        let expansion = store
            .expand_bounded(&query.with_fanout_utilization_recording(
                crate::ports::graph_authority::TraceMode::Enabled,
            ))
            .await
            .unwrap();

        assert_eq!(expansion.links.len(), 2);
        assert_eq!(without_utilization.objects, expansion.objects);
        assert_eq!(without_utilization.links, expansion.links);
        assert_eq!(without_utilization.relations, expansion.relations);
        assert_eq!(without_utilization.filtered_nodes, expansion.filtered_nodes);
        assert_eq!(
            without_utilization.bounded_failure,
            expansion.bounded_failure
        );
        assert!(expansion.fanout_utilization.iter().any(|entry| {
            entry.root.id == fixture.hub_entity.id
                && entry.relation == RelationType::About
                && entry.object_type == ObjectType::DerivedMemory
                && entry.selected_cap == 2
                && entry.retained_count == 2
                && entry.omitted_by_fanout_count == 10
        }));
    }

    #[tokio::test]
    async fn oxigraph_utilization_excludes_suppressed_intermediate() {
        let embedded = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let objects = vec![
            MemoryObject::Entity(fixtures.hub_entity.clone()),
            MemoryObject::DerivedMemory(fixtures.suppressed_seed.clone()),
            MemoryObject::DerivedMemory(fixtures.derived_reflection.clone()),
            MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
        ];
        let make_link = |id, from_id, from_type, to_id, to_type, relation| {
            let mut link = fixtures.hub_links[0].clone();
            link.id = MemoryId::from_u128(id);
            link.from_id = from_id;
            link.from_type = from_type;
            link.to_id = to_id;
            link.to_type = to_type;
            link.relation = relation;
            link
        };
        let links = vec![
            make_link(
                0x550e_8400_e29b_41d4_a716_4466_5544_0700,
                fixtures.hub_entity.id,
                ObjectType::Entity,
                fixtures.suppressed_seed.id,
                ObjectType::DerivedMemory,
                RelationType::About,
            ),
            make_link(
                0x550e_8400_e29b_41d4_a716_4466_5544_0701,
                fixtures.suppressed_seed.id,
                ObjectType::DerivedMemory,
                fixtures.derived_reflection.id,
                ObjectType::DerivedMemory,
                RelationType::DerivedFrom,
            ),
            make_link(
                0x550e_8400_e29b_41d4_a716_4466_5544_0702,
                fixtures.suppressed_seed.id,
                ObjectType::DerivedMemory,
                fixtures.user_preference.id,
                ObjectType::DerivedMemory,
                RelationType::DerivedFrom,
            ),
        ];
        embedded.upsert_objects(&objects).await.unwrap();
        embedded.upsert_links(&links).await.unwrap();

        let query = GraphExpansionQuery::new(fixtures.hub_entity.id, ObjectType::Entity, 2, 10)
            .with_allowed_object_types(vec![ObjectType::DerivedMemory])
            .with_max_fanout_per_node(1)
            .with_fanout_utilization_recording(crate::ports::graph_authority::TraceMode::Enabled);
        let embedded_expansion = embedded.expand_bounded(&query).await.unwrap();

        assert!(embedded_expansion.filtered_nodes.iter().any(|filtered| {
            filtered.object_ref.id == fixtures.suppressed_seed.id
                && filtered.reason == GraphExpansionFilteredReason::Suppressed
        }));
        assert!(!embedded_expansion
            .fanout_utilization
            .iter()
            .any(|entry| { entry.root.id == fixtures.suppressed_seed.id }));
        assert!(embedded_expansion
            .fanout_utilization
            .iter()
            .all(|entry| entry.root.id == fixtures.hub_entity.id));
        assert!(!embedded_expansion.fanout_utilization.is_empty());
    }

    #[tokio::test]
    async fn oxigraph_utilization_excludes_nodes_returned_only_at_max_depth() {
        let embedded = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let objects = vec![
            MemoryObject::Entity(fixtures.hub_entity.clone()),
            MemoryObject::DerivedMemory(fixtures.suppressed_seed.clone()),
            MemoryObject::Episode(fixtures.episode.clone()),
            MemoryObject::Observation(fixtures.salient_observation.clone()),
            MemoryObject::DerivedMemory(fixtures.user_preference.clone()),
            MemoryObject::DerivedMemory(fixtures.derived_reflection.clone()),
        ];
        let make_link = |id, from_id, from_type, to_id, to_type, relation| {
            let mut link = fixtures.hub_links[0].clone();
            link.id = MemoryId::from_u128(id);
            link.from_id = from_id;
            link.from_type = from_type;
            link.to_id = to_id;
            link.to_type = to_type;
            link.relation = relation;
            link
        };
        let links = vec![
            make_link(
                0x550e_8400_e29b_41d4_a716_4466_5544_0710,
                fixtures.hub_entity.id,
                ObjectType::Entity,
                fixtures.suppressed_seed.id,
                ObjectType::DerivedMemory,
                RelationType::About,
            ),
            make_link(
                0x550e_8400_e29b_41d4_a716_4466_5544_0711,
                fixtures.suppressed_seed.id,
                ObjectType::DerivedMemory,
                fixtures.user_preference.id,
                ObjectType::DerivedMemory,
                RelationType::DerivedFrom,
            ),
            make_link(
                0x550e_8400_e29b_41d4_a716_4466_5544_0712,
                fixtures.hub_entity.id,
                ObjectType::Entity,
                fixtures.episode.id,
                ObjectType::Episode,
                RelationType::About,
            ),
            make_link(
                0x550e_8400_e29b_41d4_a716_4466_5544_0713,
                fixtures.episode.id,
                ObjectType::Episode,
                fixtures.salient_observation.id,
                ObjectType::Observation,
                RelationType::DerivedFrom,
            ),
            make_link(
                0x550e_8400_e29b_41d4_a716_4466_5544_0714,
                fixtures.salient_observation.id,
                ObjectType::Observation,
                fixtures.user_preference.id,
                ObjectType::DerivedMemory,
                RelationType::DerivedFrom,
            ),
            make_link(
                0x550e_8400_e29b_41d4_a716_4466_5544_0715,
                fixtures.user_preference.id,
                ObjectType::DerivedMemory,
                fixtures.derived_reflection.id,
                ObjectType::DerivedMemory,
                RelationType::DerivedFrom,
            ),
        ];
        embedded.upsert_objects(&objects).await.unwrap();
        embedded.upsert_links(&links).await.unwrap();

        let query = GraphExpansionQuery::new(fixtures.hub_entity.id, ObjectType::Entity, 3, 20)
            .with_fanout_utilization_recording(crate::ports::graph_authority::TraceMode::Enabled);
        let embedded_expansion = embedded.expand_bounded(&query).await.unwrap();

        assert!(embedded_expansion.objects.iter().any(|object| {
            matches!(object, MemoryObject::DerivedMemory(memory) if memory.id == fixtures.user_preference.id)
        }));
        assert!(!embedded_expansion
            .fanout_utilization
            .iter()
            .any(|entry| entry.root.id == fixtures.user_preference.id));
        assert!(embedded_expansion
            .fanout_utilization
            .iter()
            .any(|entry| entry.root.id == fixtures.salient_observation.id));
    }

    #[tokio::test]
    async fn oxigraph_expansion_preserves_depth_node_cap_and_allowlists() {
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

        let relation_filtered = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 20)
                    .with_allowed_relation_types(vec![RelationType::About])
                    .with_max_fanout_per_node(3),
            )
            .await
            .unwrap();
        assert_eq!(relation_filtered.objects.len(), 4);
        assert_eq!(relation_filtered.links.len(), 3);
        assert_eq!(relation_filtered.relations.len(), 3);
        assert!(relation_filtered
            .objects
            .contains(&MemoryObject::Entity(fixture.hub_entity)));
        assert!(relation_filtered.objects.iter().all(|object| matches!(
            object,
            MemoryObject::Entity(_) | MemoryObject::DerivedMemory(_)
        )));
        assert!(relation_filtered.relations.iter().all(|relation| {
            relation.relation == RelationType::About && relation.proximity == 1
        }));
    }

    #[tokio::test]
    async fn oxigraph_expansion_returns_canonical_objects_and_links() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixture = high_fanout_graph_fixture();
        let mut unrelated_link = fixture.links[0].clone();
        unrelated_link.id = MemoryId::from_u128(u128::MAX);
        unrelated_link.from_id = fixture.episode.id;
        unrelated_link.from_type = ObjectType::Episode;
        unrelated_link.to_id = fixture.observation.id;
        unrelated_link.to_type = ObjectType::Observation;

        store.upsert_objects(&fixture.objects()).await.unwrap();
        store.upsert_links(&fixture.links).await.unwrap();
        store.upsert_links(&[unrelated_link]).await.unwrap();

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
    async fn oxigraph_expansion_applies_selectivity_fanout() {
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
        let expansion = store.expand_bounded(&query).await.unwrap();

        assert_eq!(expansion.links.len(), 1);
        assert_eq!(
            expansion
                .objects
                .iter()
                .filter(|object_ref| object_ref.object_type() == ObjectType::DerivedMemory)
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn oxigraph_expansion_applies_node_cap() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixture = high_fanout_graph_fixture();

        store.upsert_objects(&fixture.objects()).await.unwrap();
        store.upsert_links(&fixture.links).await.unwrap();

        let query = GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 5)
            .with_allowed_object_types(vec![ObjectType::DerivedMemory])
            .with_max_fanout_per_node(20);
        let expansion = store.expand_bounded(&query).await.unwrap();

        assert_eq!(expansion.objects.len(), 5);
        assert_eq!(expansion.links.len(), 4);
        assert_eq!(
            expansion
                .objects
                .iter()
                .filter(|object_ref| object_ref.object_type() == ObjectType::DerivedMemory)
                .count(),
            4
        );
        assert_eq!(
            expansion.bounded_failure.unwrap().reason,
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
    async fn oxigraph_expansion_reports_or_fails_closed_on_hub_limit() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixture = high_fanout_graph_fixture();

        store.upsert_objects(&fixture.objects()).await.unwrap();
        store.upsert_links(&fixture.links).await.unwrap();

        let partial = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 20)
                    .with_max_hub_edges(2)
                    .with_max_fanout_per_node(2),
            )
            .await
            .unwrap();
        assert_eq!(
            partial.bounded_failure.unwrap().reason,
            GraphExpansionBoundedFailureReason::HubLimit
        );
        assert_eq!(partial.links.len(), 2);

        let error = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixture.hub_entity.id, ObjectType::Entity, 1, 20)
                    .with_max_hub_edges(1)
                    .with_failure_policy(GraphExpansionFailurePolicy {
                        timeout_ms: Some(250),
                        mode: crate::domain::GraphFailureMode::FailClosed,
                    }),
            )
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            CustomError::GraphExpansionBounded(trace)
                if trace.reason == crate::domain::GraphExpansionBoundedReason::HubLimit
        ));
    }

    #[tokio::test]
    async fn oxigraph_expansion_uses_targeted_supersession_evidence_outside_frontier() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let superseded_memory = fixtures.user_preference.clone();
        let replacement = fixtures.correction.clone();
        let hub_link = crate::domain::MemoryLink {
            id: MemoryId::from_u128(0x550e_8400_e29b_41d4_a716_4466_5500_0002),
            object_type: ObjectType::MemoryLink,
            from_id: fixtures.hub_entity.id,
            from_type: ObjectType::Entity,
            to_id: superseded_memory.id,
            to_type: ObjectType::DerivedMemory,
            relation: RelationType::About,
            rationale: Some("Hub reaches a superseded memory.".to_owned()),
            created_at: superseded_memory.created_at,
            schema_version: superseded_memory.schema_version.clone(),
        };
        let supersedes_link = crate::domain::MemoryLink {
            id: MemoryId::from_u128(0x550e_8400_e29b_41d4_a716_4466_5500_0003),
            object_type: ObjectType::MemoryLink,
            from_id: replacement.id,
            from_type: ObjectType::DerivedMemory,
            to_id: superseded_memory.id,
            to_type: ObjectType::DerivedMemory,
            relation: RelationType::Supersedes,
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
                == MemoryObjectRef::from_id_type(superseded_memory.id, ObjectType::DerivedMemory)
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
                == MemoryObjectRef::from_id_type(superseded_memory.id, ObjectType::DerivedMemory)
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
    async fn oxigraph_expansion_honors_lifecycle_filters() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let mut fixtures = representative_fixtures();
        fixtures.episode.retention_state = RetentionState::Suppressed;
        fixtures.salient_observation.retention_state = RetentionState::Suppressed;

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store.upsert_links(&fixtures.links()).await.unwrap();

        let historical_objects = vec![
            MemoryObject::Episode(fixtures.episode.clone()),
            MemoryObject::Observation(fixtures.salient_observation.clone()),
            MemoryObject::MemoryThread(fixtures.soft_thread.clone()),
            MemoryObject::DerivedMemory(fixtures.suppressed_seed.clone()),
            MemoryObject::DerivedMemory(fixtures.correction.clone()),
        ];
        let explicit_history = store
            .query_objects(&GraphObjectQuery::by_refs(
                historical_objects
                    .iter()
                    .map(MemoryObject::object_ref)
                    .collect(),
            ))
            .await
            .unwrap();
        assert_eq!(explicit_history.len(), 5);
        for object in historical_objects {
            assert!(explicit_history.contains(&object));
        }

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
        assert!(default_expansion.links.is_empty());
        assert_eq!(default_expansion.filtered_nodes.len(), 1);
        assert_eq!(
            default_expansion.filtered_nodes[0].object_ref,
            MemoryObjectRef::from_id_type(fixtures.suppressed_seed.id, ObjectType::DerivedMemory)
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
                    }),
            )
            .await
            .unwrap();
        assert_eq!(
            historical_expansion.links,
            vec![fixtures.hub_links[2].clone()]
        );
        assert_eq!(historical_expansion.relations.len(), 1);
        assert!(historical_expansion
            .objects
            .contains(&MemoryObject::DerivedMemory(
                fixtures.suppressed_seed.clone()
            )));
        assert!(historical_expansion.filtered_nodes.is_empty());
    }

    #[tokio::test]
    async fn oxigraph_lifecycle_upserts_support_supersession_and_provenance_discovery() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let superseded_memory = fixtures.user_preference.clone();
        let mut non_current_memory = fixtures.open_loop.clone();
        let mut replacement = fixtures.correction.clone();
        let mut dormant_thread = fixtures.soft_thread.clone();
        non_current_memory.is_current = false;
        replacement.supersedes = vec![superseded_memory.id];
        dormant_thread.status = ThreadStatus::Dormant;
        let mut link_only = fixtures.derived_reflection.clone();
        link_only.id = MemoryId::from_u128(260);
        link_only.derived_from_episode_ids = vec![MemoryId::from_u128(998)];
        link_only.derived_from_observation_ids = vec![MemoryId::from_u128(999)];
        link_only.thread_ids.clear();
        link_only.entity_ids.clear();
        // The queried source is absent from the object, so discovery must use the link.
        let provenance_link = crate::domain::MemoryLink {
            id: MemoryId::from_u128(261),
            from_id: link_only.id,
            from_type: ObjectType::DerivedMemory,
            to_id: fixtures.episode.id,
            to_type: ObjectType::Episode,
            relation: RelationType::DerivedFrom,
            ..fixtures.soft_thread_link.clone()
        };
        let supersedes_link = crate::domain::MemoryLink {
            id: MemoryId::from_u128(0x550e_8400_e29b_41d4_a716_4466_5600_0001),
            object_type: ObjectType::MemoryLink,
            from_id: replacement.id,
            from_type: ObjectType::DerivedMemory,
            to_id: superseded_memory.id,
            to_type: ObjectType::DerivedMemory,
            relation: RelationType::Supersedes,
            rationale: Some("Replacement supersedes historical derived memory.".to_owned()),
            created_at: replacement.created_at,
            schema_version: replacement.schema_version.clone(),
        };

        store
            .upsert_objects(&[
                MemoryObject::Episode(fixtures.episode.clone()),
                MemoryObject::Observation(fixtures.salient_observation.clone()),
                MemoryObject::MemoryThread(dormant_thread.clone()),
                MemoryObject::DerivedMemory(fixtures.derived_reflection.clone()),
                MemoryObject::DerivedMemory(superseded_memory.clone()),
                MemoryObject::DerivedMemory(non_current_memory.clone()),
                MemoryObject::DerivedMemory(replacement.clone()),
                MemoryObject::DerivedMemory(link_only.clone()),
            ])
            .await
            .unwrap();
        store
            .upsert_links(&[
                fixtures.soft_thread_link.clone(),
                supersedes_link.clone(),
                provenance_link,
            ])
            .await
            .unwrap();

        let default_matches = store
            .query_derived_memories_by_provenance(&GraphDerivedMemoryProvenanceQuery::by_sources(
                vec![fixtures.episode.id],
                vec![fixtures.salient_observation.id],
            ))
            .await
            .unwrap();
        assert!(default_matches
            .iter()
            .any(|memory| memory.id == fixtures.derived_reflection.id));
        assert!(default_matches
            .iter()
            .any(|memory| memory.id == replacement.id));
        assert!(default_matches.contains(&link_only));
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
                == MemoryObjectRef::from_id_type(superseded_memory.id, ObjectType::DerivedMemory)
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
        assert!(historical_expansion
            .objects
            .contains(&MemoryObject::DerivedMemory(replacement.clone())));

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
        assert!(thread_expansion
            .objects
            .contains(&MemoryObject::MemoryThread(dormant_thread.clone())));

        let default_thread_matches = store
            .query_derived_memories_by_thread(&GraphDerivedMemoryThreadQuery::by_threads(vec![
                fixtures.soft_thread.id,
            ]))
            .await
            .unwrap();
        assert!(default_thread_matches.contains(&replacement));
        assert!(!default_thread_matches
            .iter()
            .any(|memory| memory.id == superseded_memory.id));
        assert!(!default_thread_matches
            .iter()
            .any(|memory| memory.id == non_current_memory.id));
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
        assert!(matches!(
            unsupported,
            CustomError::UnsupportedExpansionRoot { object }
                if object == MemoryObjectRef::from_id_type(fixture.links[0].id, ObjectType::MemoryLink)
        ));

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
    }

    #[tokio::test]
    async fn oxigraph_store_replaces_link_endpoints_and_relation() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let mut updated_link = fixtures.soft_thread_link.clone();
        updated_link.from_id = fixtures.derived_reflection.id;
        updated_link.from_type = ObjectType::DerivedMemory;
        updated_link.to_id = fixtures.episode.id;
        updated_link.to_type = ObjectType::Episode;
        updated_link.relation = RelationType::DerivedFrom;
        updated_link.rationale = Some("Updated link endpoints and relation.".to_owned());

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store
            .upsert_links(std::slice::from_ref(&fixtures.soft_thread_link))
            .await
            .unwrap();

        store.upsert_links(&[updated_link.clone()]).await.unwrap();

        let expansion = store
            .expand_bounded(&GraphExpansionQuery::new(
                fixtures.derived_reflection.id,
                ObjectType::DerivedMemory,
                1,
                4,
            ))
            .await
            .unwrap();
        assert_eq!(expansion.links, vec![updated_link.clone()]);
        assert_eq!(
            store.query_links_by_ids(&[updated_link.id]).await.unwrap(),
            vec![updated_link]
        );
        assert!(store
            .expand_bounded(&GraphExpansionQuery::new(
                fixtures.soft_thread_link.from_id,
                fixtures.soft_thread_link.from_type,
                1,
                4,
            ))
            .await
            .unwrap()
            .links
            .is_empty());
        assert!(!expansion.links.contains(&fixtures.soft_thread_link));
    }

    #[tokio::test]
    async fn oxigraph_updating_one_link_preserves_another_with_the_same_endpoints() {
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

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store
            .upsert_links(&[fixtures.soft_thread_link.clone(), duplicate_link.clone()])
            .await
            .unwrap();

        store.upsert_links(&[updated_link]).await.unwrap();

        let expansion = store
            .expand_bounded(&GraphExpansionQuery::new(
                duplicate_link.from_id,
                duplicate_link.from_type,
                1,
                4,
            ))
            .await
            .unwrap();
        assert_eq!(expansion.links, vec![duplicate_link]);
    }

    #[tokio::test]
    async fn persistent_oxigraph_reopens_and_hydrates_objects_links_and_lifecycle_from_rdf() {
        let graph_dir = TempGraphDir::new();
        let fixtures = representative_fixtures();
        let mut dormant_thread = fixtures.soft_thread.clone();
        dormant_thread.status = ThreadStatus::Dormant;

        {
            let store = OxigraphGraphAuthorityStore::new_persistent(graph_dir.path()).unwrap();
            store
                .upsert_objects(&[
                    MemoryObject::Episode(fixtures.episode.clone()),
                    MemoryObject::Observation(fixtures.salient_observation.clone()),
                    MemoryObject::Entity(fixtures.user_entity.clone()),
                    MemoryObject::MemoryThread(dormant_thread.clone()),
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
                    MemoryObjectRef::from_id_type(fixtures.episode.id, ObjectType::Episode),
                    MemoryObjectRef::from_id_type(
                        fixtures.salient_observation.id,
                        ObjectType::Observation,
                    ),
                    MemoryObjectRef::from_id_type(fixtures.user_entity.id, ObjectType::Entity),
                    MemoryObjectRef::from_id_type(dormant_thread.id, ObjectType::MemoryThread),
                    MemoryObjectRef::from_id_type(
                        fixtures.correction.id,
                        ObjectType::DerivedMemory,
                    ),
                    MemoryObjectRef::from_id_type(
                        fixtures.suppressed_seed.id,
                        ObjectType::DerivedMemory,
                    ),
                ]))
                .await
                .unwrap();

            assert!(queried.contains(&MemoryObject::Episode(fixtures.episode.clone())));
            assert!(queried.contains(&MemoryObject::Observation(
                fixtures.salient_observation.clone()
            )));
            assert!(queried.contains(&MemoryObject::Entity(fixtures.user_entity.clone())));
            assert!(queried.contains(&MemoryObject::MemoryThread(dormant_thread.clone())));
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
                    == MemoryObjectRef::from_id_type(
                        fixtures.suppressed_seed.id,
                        ObjectType::DerivedMemory,
                    )
            }));
        }
    }

    #[tokio::test]
    async fn oxigraph_queries_links_separately_from_memory_objects() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();
        let link = fixtures.soft_thread_link.clone();

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store
            .upsert_links(std::slice::from_ref(&link))
            .await
            .unwrap();

        let graph_refs = store
            .query_objects(&GraphObjectQuery::by_refs(vec![
                MemoryObjectRef::from_id_type(link.id, ObjectType::MemoryLink),
            ]))
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

    #[tokio::test]
    async fn oxigraph_uses_deterministic_timeout_substitute() {
        let store = OxigraphGraphAuthorityStore::new_in_memory().unwrap();
        let fixtures = representative_fixtures();

        store.upsert_objects(&fixtures.objects()).await.unwrap();
        store.upsert_links(&fixtures.links()).await.unwrap();

        let expansion = store
            .expand_bounded(
                &GraphExpansionQuery::new(fixtures.hub_entity.id, ObjectType::Entity, 1, 5)
                    .with_failure_policy(GraphExpansionFailurePolicy {
                        timeout_ms: Some(0),
                        mode: crate::domain::GraphFailureMode::AllowPartialResults,
                    }),
            )
            .await
            .unwrap();

        assert!(expansion.objects.is_empty());
        assert_eq!(
            expansion.bounded_failure.unwrap().reason,
            GraphExpansionBoundedFailureReason::Timeout
        );
    }
}
