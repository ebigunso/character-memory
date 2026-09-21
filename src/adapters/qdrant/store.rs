// Qdrant candidate-store adapter. Qdrant provides vector recall and
// payload prefiltering; Oxigraph remains authoritative for graph/lifecycle
// truth.
use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use qdrant_client::qdrant::{
    points_selector::PointsSelectorOneOf, vectors_config, Condition, CreateCollectionBuilder,
    CreateFieldIndexCollectionBuilder, DeletePointsBuilder, Distance, Filter, PointStruct,
    ScoredPoint, ScrollPointsBuilder, SearchPointsBuilder, UpsertPointsBuilder, VectorParams,
    VectorsConfig,
};
use qdrant_client::{config::QdrantConfig, Qdrant, QdrantError};

use crate::domain::MemoryId;
use crate::errors::{
    CollectionCompatibilityError, CollectionMismatch, CustomError, IoErrorKind, TransportStatus,
    VectorDatabaseError, VectorDatabaseErrorKind,
};
use crate::models::vector::{VectorCandidateMatch, VectorCandidateSearch, VectorRecordEmbedding};
use crate::ports::vector_candidate::{VectorCandidateRecall, VectorCandidateStore};

use super::payload::{
    qdrant_payload_map, qdrant_point_id, read_candidate_match, QdrantPayloadSchema,
    OBJECT_ID_FIELD, OBJECT_TYPE_FIELD, SURFACE_FIELD,
};
use super::tie_closure::close_tie_cohort;

const QDRANT_CANDIDATE_TIMEOUT_SECS: u64 = 30;
const QDRANT_CONNECT_FAILURE_PREFIX: &str = "Failed to connect to ";

pub(crate) struct QdrantVectorCandidateStore {
    client: Qdrant,
    collection_name: String,
    vector_size: u64,
}

impl QdrantVectorCandidateStore {
    pub(crate) fn new(
        url: impl AsRef<str>,
        collection_name: impl Into<String>,
        vector_size: u64,
    ) -> Result<Self, CustomError> {
        let client = Qdrant::new(qdrant_candidate_config(url.as_ref())).map_err(qdrant_error)?;
        Ok(Self {
            client,
            collection_name: collection_name.into(),
            vector_size,
        })
    }

    pub(crate) async fn init_collection(&self) -> Result<(), CustomError> {
        let collections = self.client.list_collections().await.map_err(qdrant_error)?;
        if !collections
            .collections
            .iter()
            .any(|collection| collection.name == self.collection_name)
        {
            let vectors_config = VectorsConfig {
                config: Some(vectors_config::Config::Params(VectorParams {
                    size: self.vector_size,
                    distance: Distance::Cosine.into(),
                    ..Default::default()
                })),
            };

            let create_req = CreateCollectionBuilder::new(&self.collection_name)
                .vectors_config(vectors_config)
                .build();
            self.client
                .create_collection(create_req)
                .await
                .map_err(qdrant_error)?;
        }

        self.ensure_payload_indexes().await
    }

    pub(crate) async fn ensure_payload_indexes(&self) -> Result<(), CustomError> {
        let info = self
            .client
            .collection_info(&self.collection_name)
            .await
            .map_err(qdrant_error)?;
        let collection_info = info.result.as_ref().ok_or_else(|| {
            CustomError::DatabaseError(format!(
                "Qdrant collection info response was missing result for collection '{}'",
                self.collection_name
            ))
        })?;
        validate_collection_vector_config(
            &self.collection_name,
            self.vector_size,
            collection_info
                .config
                .as_ref()
                .and_then(|config| config.params.as_ref())
                .and_then(|params| params.vectors_config.as_ref()),
        )?;

        let empty_payload_schema: HashMap<String, qdrant_client::qdrant::PayloadSchemaInfo> =
            HashMap::new();
        let payload_schema = if collection_info.payload_schema.is_empty() {
            &empty_payload_schema
        } else {
            &collection_info.payload_schema
        };

        for field in QdrantPayloadSchema::indexed_fields() {
            if payload_schema.contains_key(field.field.name()) {
                continue;
            }

            self.client
                .create_field_index(CreateFieldIndexCollectionBuilder::new(
                    &self.collection_name,
                    field.field.name(),
                    field.kind.field_type(),
                ))
                .await
                .map_err(qdrant_error)?;
        }

        Ok(())
    }

    async fn upsert_points(
        &self,
        records: &[VectorRecordEmbedding<'_>],
    ) -> Result<(), CustomError> {
        for record in records {
            let actual_vector_size = u64::try_from(record.embedding.len()).unwrap_or(u64::MAX);
            if actual_vector_size != self.vector_size {
                return Err(CollectionCompatibilityError {
                    collection: self.collection_name.clone(),
                    mismatch: CollectionMismatch::VectorSize {
                        expected: self.vector_size,
                        actual: actual_vector_size,
                    },
                }
                .into());
            }
        }
        let points = qdrant_point_structs(records)?;
        let request = UpsertPointsBuilder::new(&self.collection_name, points)
            .wait(true)
            .build();
        self.client
            .upsert_points(request)
            .await
            .map_err(qdrant_error)?;
        Ok(())
    }

    async fn search_candidate_batch(
        &self,
        query: &VectorCandidateSearch,
        fetch_limit: usize,
    ) -> Result<Vec<VectorCandidateMatch>, CustomError> {
        let mut builder = SearchPointsBuilder::new(
            &self.collection_name,
            query.query_embedding.clone(),
            fetch_limit as u64,
        )
        .with_payload(true)
        .with_vectors(false);

        builder = builder.filter(qdrant_candidate_filter(query));

        let response = self
            .client
            .search_points(builder.build())
            .await
            .map_err(qdrant_error)?;
        response
            .result
            .into_iter()
            .map(scored_point_to_match)
            .collect()
    }

    async fn scroll_zero_norm_candidate_batch(
        &self,
        query: &VectorCandidateSearch,
        fetch_limit: usize,
    ) -> Result<Vec<VectorCandidateMatch>, CustomError> {
        let backend_limit = qdrant_scroll_fetch_limit(fetch_limit)?;
        let request = ScrollPointsBuilder::new(&self.collection_name)
            .filter(qdrant_candidate_filter(query))
            .limit(backend_limit)
            .with_payload(true)
            .with_vectors(false)
            .build();
        self.client
            .scroll(request)
            .await
            .map_err(qdrant_error)?
            .result
            .into_iter()
            .map(|point| qdrant_payload_to_match(&point.payload, 0.0))
            .collect()
    }
}

fn validate_collection_vector_config(
    collection_name: &str,
    expected_vector_size: u64,
    vectors_config: Option<&VectorsConfig>,
) -> Result<(), CustomError> {
    let Some(vectors_config) = vectors_config else {
        return Err(CollectionCompatibilityError {
            collection: collection_name.to_owned(),
            mismatch: CollectionMismatch::MissingVectorConfiguration,
        }
        .into());
    };

    match vectors_config.config.as_ref() {
        Some(vectors_config::Config::Params(params))
            if params.size == expected_vector_size
                && params.distance == Distance::Cosine as i32 =>
        {
            Ok(())
        }
        Some(vectors_config::Config::Params(params)) if params.size == expected_vector_size => {
            Err(CollectionCompatibilityError {
                collection: collection_name.to_owned(),
                mismatch: CollectionMismatch::Distance {
                    expected: "Cosine",
                    actual: Distance::try_from(params.distance)
                        .map(|distance| distance.as_str_name().to_owned())
                        .unwrap_or_else(|_| params.distance.to_string()),
                },
            }
            .into())
        }
        Some(vectors_config::Config::Params(params)) => Err(CollectionCompatibilityError {
            collection: collection_name.to_owned(),
            mismatch: CollectionMismatch::VectorSize {
                expected: expected_vector_size,
                actual: params.size,
            },
        }
        .into()),
        Some(vectors_config::Config::ParamsMap(params_map)) => {
            let mut vector_names = params_map.map.keys().cloned().collect::<Vec<_>>();
            vector_names.sort();
            Err(CollectionCompatibilityError {
                collection: collection_name.to_owned(),
                mismatch: CollectionMismatch::NamedVectors {
                    names: vector_names,
                },
            }
            .into())
        }
        None => Err(CollectionCompatibilityError {
            collection: collection_name.to_owned(),
            mismatch: CollectionMismatch::EmptyVectorConfiguration,
        }
        .into()),
    }
}

#[async_trait]
impl VectorCandidateStore for QdrantVectorCandidateStore {
    async fn upsert_vector_records(
        &self,
        records: &[VectorRecordEmbedding<'_>],
    ) -> Result<(), CustomError> {
        self.upsert_points(records).await
    }

    async fn search_candidates(
        &self,
        query: &VectorCandidateSearch,
    ) -> Result<VectorCandidateRecall, CustomError> {
        if query.limit == 0 || query.object_types.is_empty() || query.surfaces.is_empty() {
            return Ok(VectorCandidateRecall {
                candidates: crate::models::vector::CanonicalCandidates::new([]),
                scene_pool: None,
                completeness: crate::api::types::retrieval::VectorRecallCompleteness::NotRequested,
            });
        }

        let actual_vector_size = u64::try_from(query.query_embedding.len()).unwrap_or(u64::MAX);
        if actual_vector_size != self.vector_size {
            return Err(CollectionCompatibilityError {
                collection: self.collection_name.clone(),
                mismatch: CollectionMismatch::VectorSize {
                    expected: self.vector_size,
                    actual: actual_vector_size,
                },
            }
            .into());
        }

        let zero_norm = query.is_zero_norm();
        let fetch_limit_cap = if zero_norm {
            usize::try_from(u32::MAX).unwrap_or(usize::MAX)
        } else {
            usize::MAX
        };
        let closed = close_tie_cohort(query.limit, fetch_limit_cap, |fetch_limit| async move {
            if zero_norm {
                self.scroll_zero_norm_candidate_batch(query, fetch_limit)
                    .await
            } else {
                self.search_candidate_batch(query, fetch_limit).await
            }
        })
        .await?;
        let scanned = zero_norm.then_some(closed.fetched);
        let completeness = closed.completeness(scanned);
        Ok(VectorCandidateRecall {
            candidates: closed.candidates,
            scene_pool: query
                .surfaces
                .iter()
                .any(|surface| {
                    matches!(
                        surface,
                        crate::domain::VectorSurface::SceneSetting
                            | crate::domain::VectorSurface::SceneParticipants
                    )
                })
                .then_some(closed.pool),
            completeness,
        })
    }

    async fn delete_candidates(&self, object_ids: &[MemoryId]) -> Result<(), CustomError> {
        if object_ids.is_empty() {
            return Ok(());
        }

        let conditions: Vec<_> = object_ids
            .iter()
            .map(|id| Condition::matches(OBJECT_ID_FIELD, id.to_string()))
            .collect();
        let selector = PointsSelectorOneOf::Filter(Filter::should(conditions));
        let request = DeletePointsBuilder::new(&self.collection_name)
            .points(selector)
            .wait(true)
            .build();
        self.client
            .delete_points(request)
            .await
            .map_err(qdrant_error)?;
        Ok(())
    }
}

fn qdrant_error(error: QdrantError) -> CustomError {
    let vector_error = match error {
        QdrantError::ResponseError { status } => {
            let status_kind = transport_status(status.code() as i32);
            let erased_connect_source =
                is_erased_qdrant_connect_failure(&status_kind, status.message());
            let kind = if let Some(io_kind) = find_io_error_kind(&status) {
                VectorDatabaseErrorKind::Io { io_kind }
            } else if erased_connect_source {
                VectorDatabaseErrorKind::HttpConnect
            } else {
                VectorDatabaseErrorKind::Response
            };
            VectorDatabaseError::new(
                "qdrant",
                kind,
                Some(status_kind),
                status.message().to_owned(),
            )
        }
        QdrantError::ResourceExhaustedError {
            status,
            retry_after_seconds,
        } => VectorDatabaseError::new(
            "qdrant",
            VectorDatabaseErrorKind::ResourceExhausted,
            Some(transport_status(status.code() as i32)),
            status.message().to_owned(),
        )
        .with_retry_after_seconds(retry_after_seconds),
        QdrantError::ConversionError(message) => {
            VectorDatabaseError::new("qdrant", VectorDatabaseErrorKind::Conversion, None, message)
        }
        QdrantError::InvalidUri(error) => VectorDatabaseError::new(
            "qdrant",
            VectorDatabaseErrorKind::InvalidUri,
            None,
            error.to_string(),
        ),
        QdrantError::NoSnapshotFound(collection) => VectorDatabaseError::new(
            "qdrant",
            VectorDatabaseErrorKind::NoSnapshotFound,
            None,
            collection,
        ),
        QdrantError::Io(error) => VectorDatabaseError::new(
            "qdrant",
            VectorDatabaseErrorKind::Io {
                io_kind: IoErrorKind::from(error.kind()),
            },
            None,
            error.to_string(),
        ),
        QdrantError::Reqwest(error) => {
            let status = error
                .status()
                .map(|status| http_transport_status(status.as_u16()));
            let kind = if error.is_timeout() {
                VectorDatabaseErrorKind::HttpTimeout
            } else if error.is_connect() {
                VectorDatabaseErrorKind::HttpConnect
            } else if error.is_status() {
                VectorDatabaseErrorKind::HttpStatus
            } else {
                VectorDatabaseErrorKind::Http
            };
            VectorDatabaseError::new("qdrant", kind, status, error.to_string())
        }
        QdrantError::JsonToPayload(value) => VectorDatabaseError::new(
            "qdrant",
            VectorDatabaseErrorKind::JsonToPayload,
            None,
            value.to_string(),
        ),
        QdrantError::PayloadDeserialization(error) => VectorDatabaseError::new(
            "qdrant",
            VectorDatabaseErrorKind::PayloadDeserialization,
            None,
            error.to_string(),
        ),
    };

    CustomError::VectorDatabaseError(vector_error)
}

fn is_erased_qdrant_connect_failure(status: &TransportStatus, message: &str) -> bool {
    // Ruled external-contract exception: qdrant-client 1.19.0 erases the tonic transport
    // source in src/channel_pool.rs with
    // `Status::internal(format!("Failed to connect to {}: {:?}", self.uri, e))`.
    // Recheck this on every qdrant-client bump; retire the prefix coupling once upstream
    // preserves a downcastable source.
    *status == TransportStatus::Internal && message.starts_with(QDRANT_CONNECT_FAILURE_PREFIX)
}

fn http_transport_status(status: u16) -> TransportStatus {
    match status {
        200 => TransportStatus::Ok,
        400 => TransportStatus::InvalidArgument,
        401 => TransportStatus::Unauthenticated,
        403 => TransportStatus::PermissionDenied,
        404 => TransportStatus::NotFound,
        408 | 504 => TransportStatus::DeadlineExceeded,
        409 => TransportStatus::Aborted,
        412 => TransportStatus::FailedPrecondition,
        416 => TransportStatus::OutOfRange,
        429 => TransportStatus::ResourceExhausted,
        499 => TransportStatus::Cancelled,
        500 => TransportStatus::Internal,
        501 => TransportStatus::Unimplemented,
        503 => TransportStatus::Unavailable,
        other => TransportStatus::Unrecognized(other.to_string()),
    }
}

fn find_io_error_kind(error: &(dyn std::error::Error + 'static)) -> Option<IoErrorKind> {
    let mut current = Some(error);
    while let Some(source) = current {
        if let Some(io_error) = source.downcast_ref::<std::io::Error>() {
            return Some(IoErrorKind::from(io_error.kind()));
        }
        current = source.source();
    }
    None
}

fn transport_status(code: i32) -> TransportStatus {
    match code {
        0 => TransportStatus::Ok,
        1 => TransportStatus::Cancelled,
        2 => TransportStatus::Unknown,
        3 => TransportStatus::InvalidArgument,
        4 => TransportStatus::DeadlineExceeded,
        5 => TransportStatus::NotFound,
        6 => TransportStatus::AlreadyExists,
        7 => TransportStatus::PermissionDenied,
        8 => TransportStatus::ResourceExhausted,
        9 => TransportStatus::FailedPrecondition,
        10 => TransportStatus::Aborted,
        11 => TransportStatus::OutOfRange,
        12 => TransportStatus::Unimplemented,
        13 => TransportStatus::Internal,
        14 => TransportStatus::Unavailable,
        15 => TransportStatus::DataLoss,
        16 => TransportStatus::Unauthenticated,
        other => TransportStatus::Unrecognized(other.to_string()),
    }
}

fn qdrant_candidate_config(url: &str) -> QdrantConfig {
    // `keep_alive_while_idle` codifies the crate default rather than changing
    // behavior: without a transport-level ping interval (not exposed by
    // qdrant-client) tonic sends no idle keepalive pings either way. Kept
    // explicit so the intended channel behavior survives crate-default changes.
    QdrantConfig::from_url(url)
        .timeout(Duration::from_secs(QDRANT_CANDIDATE_TIMEOUT_SECS))
        .keep_alive_while_idle()
}

fn qdrant_scroll_fetch_limit(fetch_limit: usize) -> Result<u32, CustomError> {
    u32::try_from(fetch_limit).map_err(|_| {
        CustomError::VectorDatabaseError(VectorDatabaseError::new(
            "qdrant",
            VectorDatabaseErrorKind::Conversion,
            None,
            format!(
                "Qdrant scroll limit {fetch_limit} exceeds the backend maximum {}",
                u32::MAX
            ),
        ))
    })
}

fn qdrant_candidate_filter(query: &VectorCandidateSearch) -> Filter {
    Filter::must([
        any_field_matches(
            OBJECT_TYPE_FIELD,
            query.object_types.iter().map(ToString::to_string),
        ),
        any_field_matches(
            SURFACE_FIELD,
            query.surfaces.iter().map(ToString::to_string),
        ),
    ])
}

fn any_field_matches(
    field: &str,
    values: impl IntoIterator<Item = impl Into<String>>,
) -> Condition {
    let conditions = values
        .into_iter()
        .map(|value| Condition::matches(field, value.into()))
        .collect::<Vec<_>>();

    if conditions.len() == 1 {
        conditions.into_iter().next().unwrap()
    } else {
        Condition::from(Filter::min_should(1, conditions))
    }
}

fn qdrant_point_structs(
    records: &[VectorRecordEmbedding<'_>],
) -> Result<Vec<PointStruct>, CustomError> {
    records
        .iter()
        .map(|record| {
            let payload = qdrant_payload_map(record.record)?;
            Ok(PointStruct::new(
                qdrant_point_id(record.record).to_string(),
                record.embedding.to_vec(),
                payload,
            ))
        })
        .collect()
}

fn scored_point_to_match(point: ScoredPoint) -> Result<VectorCandidateMatch, CustomError> {
    qdrant_payload_to_match(&point.payload, point.score)
}

fn qdrant_payload_to_match(
    payload: &HashMap<String, qdrant_client::qdrant::Value>,
    score: f32,
) -> Result<VectorCandidateMatch, CustomError> {
    read_candidate_match("qdrant", score, |field| {
        payload
            .get(field.name())
            .and_then(|value| value.kind.as_ref())
            .and_then(|kind| match kind {
                qdrant_client::qdrant::value::Kind::StringValue(value) => Some(value.as_str()),
                _ => None,
            })
    })
    .map_err(CustomError::VectorDatabaseError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::qdrant::payload::QdrantPayloadField;
    use crate::api::types::retrieval::VectorRecallCompleteness;
    use crate::domain::{ObjectType, VectorSurface, DEFAULT_SCHEMA_VERSION};
    use crate::models::vector::{VectorRecord, VectorRecordEmbedding};
    use qdrant_client::qdrant::{
        point_id::PointIdOptions, value::Kind, vector, vectors, DeleteCollectionBuilder, PointId,
        Value, VectorParamsMap,
    };

    fn payload_string<'a>(
        payload: &'a HashMap<String, qdrant_client::qdrant::Value>,
        field: &str,
    ) -> Option<&'a str> {
        payload
            .get(field)
            .and_then(|value| value.kind.as_ref())
            .and_then(|kind| match kind {
                Kind::StringValue(value) => Some(value.as_str()),
                _ => None,
            })
    }
    use std::env;
    use std::time::Instant;
    use uuid::Uuid;

    mod port_contract {
        use super::*;
        use crate::test_support::TemporaryVectorCandidateStore;
        use std::collections::HashSet;

        #[tokio::test]
        async fn embedded_wrong_width_upsert_rejects_with_collection_mismatch() {
            let store = TemporaryVectorCandidateStore::open(2).await;
            let result = wrong_width_upsert(&store).await;
            drop(store);

            assert_wrong_width_rejected(result);
        }

        #[tokio::test]
        async fn service_wrong_width_upsert_fails_before_contacting_qdrant() {
            let store =
                QdrantVectorCandidateStore::new("http://127.0.0.1:1", "not_contacted", 2).unwrap();
            let result = wrong_width_upsert(&store).await;

            assert_wrong_width_rejected(result);
        }

        #[tokio::test]
        async fn service_wrong_width_upsert_rejects_with_collection_mismatch() {
            if env::var_os("REQUIRE_QDRANT_TESTS").is_none() {
                return;
            }
            let store = service_store().await;
            let result = wrong_width_upsert(&store).await;
            store
                .client
                .delete_collection(&store.collection_name)
                .await
                .unwrap();

            assert_wrong_width_rejected(result);
        }

        #[tokio::test]
        async fn embedded_delete_removes_every_surface_of_only_the_requested_object() {
            let store = TemporaryVectorCandidateStore::open(2).await;
            let result = delete_surfaces(&store).await;
            drop(store);

            assert_deleted_surfaces(result.unwrap());
        }

        #[tokio::test]
        async fn service_delete_removes_every_surface_of_only_the_requested_object() {
            if env::var_os("REQUIRE_QDRANT_TESTS").is_none() {
                return;
            }
            let store = service_store().await;
            let result = delete_surfaces(&store).await;
            store
                .client
                .delete_collection(&store.collection_name)
                .await
                .unwrap();

            assert_deleted_surfaces(result.unwrap());
        }

        async fn service_store() -> QdrantVectorCandidateStore {
            let url = env::var("QDRANT_CONNECTION_STRING")
                .expect("QDRANT_CONNECTION_STRING is required for service port contracts");
            let collection = format!("cmem_port_contract_{}", Uuid::new_v4());
            let store = QdrantVectorCandidateStore::new(url, collection, 2).unwrap();
            store.init_collection().await.unwrap();
            store
        }

        async fn wrong_width_upsert(store: &dyn VectorCandidateStore) -> Result<(), CustomError> {
            let record = VectorRecord::new(
                MemoryId::from_u128(1),
                ObjectType::Episode,
                VectorSurface::Summary,
                DEFAULT_SCHEMA_VERSION,
                "wrong-width record",
            );
            store
                .upsert_vector_records(&[VectorRecordEmbedding::new(&record, &[1.0, 0.0, 0.0])])
                .await
        }

        fn assert_wrong_width_rejected(result: Result<(), CustomError>) {
            assert!(
                matches!(
                    &result,
                    Err(CustomError::CollectionIncompatible(
                        CollectionCompatibilityError {
                            mismatch: CollectionMismatch::VectorSize {
                                expected: 2,
                                actual: 3
                            },
                            ..
                        }
                    ))
                ),
                "wrong-width upsert result: {result:?}"
            );
        }

        async fn delete_surfaces(
            store: &dyn VectorCandidateStore,
        ) -> Result<(VectorCandidateRecall, VectorCandidateRecall), CustomError> {
            let records = [
                (MemoryId::from_u128(1), VectorSurface::Summary),
                (MemoryId::from_u128(1), VectorSurface::Text),
                (MemoryId::from_u128(2), VectorSurface::Summary),
            ]
            .map(|(id, surface)| {
                VectorRecord::new(
                    id,
                    ObjectType::Episode,
                    surface,
                    DEFAULT_SCHEMA_VERSION,
                    "surface",
                )
            });
            let embeddings = records
                .iter()
                .map(|record| VectorRecordEmbedding::new(record, &[1.0, 0.0]))
                .collect::<Vec<_>>();
            store.upsert_vector_records(&embeddings).await?;
            let query = VectorCandidateSearch::new(vec![1.0, 0.0], 10, vec![ObjectType::Episode]);
            let before = store.search_candidates(&query).await?;
            store.delete_candidates(&[MemoryId::from_u128(1)]).await?;
            let after = store.search_candidates(&query).await?;
            Ok((before, after))
        }

        fn assert_deleted_surfaces(
            (before, after): (VectorCandidateRecall, VectorCandidateRecall),
        ) {
            let surfaces = |recall: &VectorCandidateRecall| {
                recall
                    .candidates
                    .iter()
                    .map(|candidate| (candidate.object_id, candidate.surface))
                    .collect::<HashSet<_>>()
            };
            assert_eq!(
                surfaces(&before),
                HashSet::from([
                    (MemoryId::from_u128(1), VectorSurface::Summary),
                    (MemoryId::from_u128(1), VectorSurface::Text),
                    (MemoryId::from_u128(2), VectorSurface::Summary),
                ])
            );
            assert_eq!(
                surfaces(&after),
                HashSet::from([(MemoryId::from_u128(2), VectorSurface::Summary)])
            );
        }
    }

    #[tokio::test]
    async fn empty_scope_and_zero_limit_return_without_contacting_qdrant() {
        let store =
            QdrantVectorCandidateStore::new("http://127.0.0.1:1", "not_contacted", 2).unwrap();
        let queries = [
            VectorCandidateSearch::new(vec![1.0, 0.0], 10, Vec::new()),
            VectorCandidateSearch::new(vec![1.0, 0.0], 0, vec![ObjectType::Episode]),
        ];

        for query in queries {
            let recall = store.search_candidates(&query).await.unwrap();
            assert!(recall.candidates.is_empty());
            assert_eq!(recall.completeness, VectorRecallCompleteness::NotRequested);
        }
    }

    #[tokio::test]
    async fn wrong_dimension_zero_norm_query_fails_before_contacting_qdrant() {
        let store =
            QdrantVectorCandidateStore::new("http://127.0.0.1:1", "not_contacted", 2).unwrap();
        let query = VectorCandidateSearch::new(vec![0.0], 10, vec![ObjectType::Episode]);

        let error = store.search_candidates(&query).await.unwrap_err();

        assert!(matches!(
            error,
            CustomError::CollectionIncompatible(CollectionCompatibilityError {
                collection,
                mismatch: CollectionMismatch::VectorSize {
                    expected: 2,
                    actual: 1,
                },
            }) if collection == "not_contacted"
        ));
    }

    #[test]
    fn qdrant_scroll_limit_checks_backend_width_without_narrowing() {
        let backend_max = usize::try_from(u32::MAX).unwrap();
        assert_eq!(qdrant_scroll_fetch_limit(backend_max).unwrap(), u32::MAX);

        if let Some(too_large) = backend_max.checked_add(1) {
            assert!(matches!(
                qdrant_scroll_fetch_limit(too_large),
                Err(CustomError::VectorDatabaseError(VectorDatabaseError {
                    kind: VectorDatabaseErrorKind::Conversion,
                    ..
                }))
            ));
        }
    }

    #[test]
    fn qdrant_response_error_preserves_typed_transport_status() {
        let error = qdrant_error(QdrantError::ResponseError {
            status: tonic::Status::unavailable("offline"),
        });

        assert!(matches!(
            error,
            CustomError::VectorDatabaseError(VectorDatabaseError {
                backend,
                kind: VectorDatabaseErrorKind::Response,
                status: Some(TransportStatus::Unavailable),
                retry_after_seconds: None,
                ..
            }) if backend == "qdrant"
        ));
    }

    #[test]
    fn qdrant_response_error_promotes_nested_io_classification() {
        let status = tonic::Status::from_error(Box::new(std::io::Error::from(
            std::io::ErrorKind::ConnectionRefused,
        )));
        let error = qdrant_error(QdrantError::ResponseError { status });

        assert!(matches!(
            error,
            CustomError::VectorDatabaseError(VectorDatabaseError {
                kind: VectorDatabaseErrorKind::Io {
                    io_kind: IoErrorKind::ConnectionRefused,
                },
                ..
            })
        ));
    }

    #[test]
    fn qdrant_http_status_maps_to_typed_transport_status() {
        assert_eq!(
            http_transport_status(429),
            TransportStatus::ResourceExhausted
        );
        assert_eq!(http_transport_status(503), TransportStatus::Unavailable);
        assert_eq!(
            http_transport_status(418),
            TransportStatus::Unrecognized("418".to_owned())
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn qdrant_client_erased_connect_contract_canary() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let unreachable_address = listener.local_addr().unwrap();
        drop(listener);

        let client = Qdrant::new(
            QdrantConfig::from_url(&format!("http://{unreachable_address}"))
                .connect_timeout(Duration::from_millis(250))
                .timeout(Duration::from_millis(500)),
        )
        .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(2), client.list_collections())
            .await
            .expect("qdrant-client unreachable-endpoint request must remain bounded");
        let Err(upstream_error) = result else {
            panic!("closed loopback endpoint unexpectedly accepted a Qdrant request");
        };

        let classified = qdrant_error(upstream_error);
        assert!(
            matches!(
                classified,
                CustomError::VectorDatabaseError(VectorDatabaseError {
                    kind: VectorDatabaseErrorKind::HttpConnect,
                    ..
                })
            ),
            "qdrant-client connection-error contract drifted; inspect channel_pool.rs and retire or update the ruled adapter exception"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    #[ignore = "requires live Qdrant at QDRANT_CONNECTION_STRING or 127.0.0.1:6334"]
    async fn qdrant_channel_survives_idle_gap_before_mutating_upsert() {
        let url = env::var("QDRANT_CONNECTION_STRING")
            .unwrap_or_else(|_| "http://127.0.0.1:6334".to_owned());
        let collection_name = format!("character_memory_idle_gap_{}", Uuid::new_v4().simple());
        let store = QdrantVectorCandidateStore::new(&url, &collection_name, 4)
            .expect("live Qdrant client should build");

        store
            .init_collection()
            .await
            .expect("live Qdrant collection should initialize");

        // Idle gap without blocking the runtime; the stall signature this
        // canary encodes reproduces identically with async and blocking gaps.
        tokio::time::sleep(Duration::from_secs(10)).await;

        let records = [
            idle_gap_vector_record(ObjectType::Episode),
            idle_gap_vector_record(ObjectType::Observation),
            idle_gap_vector_record(ObjectType::Entity),
        ];
        let embeddings = [
            vec![0.1, 0.2, 0.3, 0.4],
            vec![0.2, 0.3, 0.4, 0.5],
            vec![0.3, 0.4, 0.5, 0.6],
        ];
        let record_embeddings = records
            .iter()
            .zip(embeddings.iter())
            .map(|(record, embedding)| VectorRecordEmbedding::new(record, embedding))
            .collect::<Vec<_>>();

        let started_at = Instant::now();
        let upsert_result = store.upsert_points(&record_embeddings).await;
        let elapsed = started_at.elapsed();

        // Best-effort cleanup: on environments where mutations stall after idle
        // gaps, cleanup can fail for the same reason as the upsert under test.
        // Never let cleanup mask the primary upsert diagnosis.
        let cleanup_result = store
            .client
            .delete_collection(DeleteCollectionBuilder::new(&collection_name))
            .await;

        upsert_result.unwrap_or_else(|error| {
            panic!("upsert after idle gap failed after {elapsed:?}: {error} (cleanup result: {cleanup_result:?})")
        });
        assert!(
            elapsed < Duration::from_secs(1),
            "upsert after idle gap took {elapsed:?}"
        );
        // Cleanup is a mutation on the same channel and can fail for the same
        // environmental reason this canary detects; report without failing so
        // the test outcome stays focused on the upsert timing/signature.
        if let Err(error) = cleanup_result {
            eprintln!("warning: idle-gap canary cleanup failed for {collection_name}: {error}");
        }
    }

    fn idle_gap_vector_record(object_type: ObjectType) -> VectorRecord {
        let object_id = MemoryId::new_v4();
        VectorRecord::new(
            object_id,
            object_type,
            VectorSurface::Summary,
            DEFAULT_SCHEMA_VERSION,
            "Idle-gap regression record",
        )
    }

    #[test]
    fn validates_existing_collection_vector_size() {
        let config = VectorsConfig {
            config: Some(vectors_config::Config::Params(VectorParams {
                size: 1536,
                distance: Distance::Cosine.into(),
                ..Default::default()
            })),
        };

        assert!(validate_collection_vector_config("memories", 1536, Some(&config)).is_ok());

        let error = validate_collection_vector_config("memories", 3072, Some(&config))
            .expect_err("mismatched existing collection should fail");
        assert!(matches!(
            error,
            CustomError::CollectionIncompatible(CollectionCompatibilityError {
                collection,
                mismatch: CollectionMismatch::VectorSize {
                    expected: 3072,
                    actual: 1536,
                },
            }) if collection == "memories"
        ));
    }

    #[test]
    fn rejects_existing_collection_with_wrong_distance_metric() {
        let config = VectorsConfig {
            config: Some(vectors_config::Config::Params(VectorParams {
                size: 1536,
                distance: Distance::Euclid.into(),
                ..Default::default()
            })),
        };

        let error = validate_collection_vector_config("memories", 1536, Some(&config))
            .expect_err("same-size collection with wrong distance should fail");
        assert!(matches!(
            error,
            CustomError::CollectionIncompatible(CollectionCompatibilityError {
                collection,
                mismatch: CollectionMismatch::Distance {
                    expected: "Cosine",
                    actual,
                },
            }) if collection == "memories" && actual == "Euclid"
        ));
    }

    #[test]
    fn rejects_named_vector_collection_config() {
        let config = VectorsConfig {
            config: Some(vectors_config::Config::ParamsMap(VectorParamsMap {
                map: HashMap::from([(
                    "content".to_owned(),
                    VectorParams {
                        size: 1536,
                        distance: Distance::Cosine.into(),
                        ..Default::default()
                    },
                )]),
            })),
        };

        let error = validate_collection_vector_config("memories", 1536, Some(&config))
            .expect_err("named vectors should not be accepted for unnamed vector store");
        assert!(matches!(
            error,
            CustomError::CollectionIncompatible(CollectionCompatibilityError {
                collection,
                mismatch: CollectionMismatch::NamedVectors { names },
            }) if collection == "memories" && names == vec!["content"]
        ));
    }

    #[test]
    fn search_result_mapping_reads_payload_identity_and_surface() {
        let object_id = Uuid::new_v4();
        let point_id = Uuid::new_v4();
        let point = ScoredPoint {
            id: Some(PointId {
                point_id_options: Some(PointIdOptions::Uuid(point_id.to_string())),
            }),
            payload: HashMap::from([
                (
                    OBJECT_ID_FIELD.to_owned(),
                    string_value(&object_id.to_string()),
                ),
                (OBJECT_TYPE_FIELD.to_owned(), string_value("derived_memory")),
                (
                    QdrantPayloadField::Surface.name().to_owned(),
                    string_value("derived_text"),
                ),
            ]),
            score: 0.75,
            ..Default::default()
        };

        let matched = scored_point_to_match(point).expect("point maps");

        assert_eq!(matched.object_id, object_id);
        assert_eq!(matched.object_type, ObjectType::DerivedMemory);
        assert_eq!(matched.surface, VectorSurface::DerivedText);
        assert_eq!(matched.score, 0.75);
    }

    #[test]
    fn point_ids_are_unique_per_object_surface_and_identity_stays_in_payload() {
        let object_id = Uuid::new_v4();
        let summary = VectorRecord::new(
            object_id,
            ObjectType::Episode,
            VectorSurface::Summary,
            DEFAULT_SCHEMA_VERSION,
            "Episode summary.",
        );
        let text = VectorRecord::new(
            object_id,
            ObjectType::Episode,
            VectorSurface::Text,
            DEFAULT_SCHEMA_VERSION,
            "Episode text.",
        );

        let points = qdrant_point_structs(&[
            VectorRecordEmbedding::new(&summary, &[1.0, 0.0]),
            VectorRecordEmbedding::new(&text, &[0.0, 1.0]),
        ])
        .expect("points build");

        assert_ne!(points[0].id, points[1].id);
        assert_eq!(
            payload_string(&points[0].payload, OBJECT_ID_FIELD).unwrap(),
            object_id.to_string()
        );
        assert_eq!(
            payload_string(&points[1].payload, OBJECT_ID_FIELD).unwrap(),
            object_id.to_string()
        );
    }

    #[test]
    fn upsert_points_encode_dense_unnamed_vectors() {
        let record = VectorRecord::new(
            Uuid::new_v4(),
            ObjectType::DerivedMemory,
            VectorSurface::DerivedText,
            DEFAULT_SCHEMA_VERSION,
            "Reflection: Qdrant keeps embedding provenance.",
        );

        let points = qdrant_point_structs(&[VectorRecordEmbedding::new(&record, &[0.25, 0.75])])
            .expect("points build");

        assert_eq!(points.len(), 1);
        let vector = points[0]
            .vectors
            .as_ref()
            .and_then(|vectors| vectors.vectors_options.as_ref())
            .expect("vectors present");
        match vector {
            vectors::VectorsOptions::Vector(vector) => match vector.vector.as_ref() {
                Some(vector::Vector::Dense(dense)) => assert_eq!(dense.data, vec![0.25, 0.75]),
                _ => panic!("expected dense vector"),
            },
            _ => panic!("expected unnamed vector"),
        }
    }

    #[tokio::test]
    #[ignore = "requires local Qdrant: docker compose -f docker-compose.qdrant.yml up -d and QDRANT_CONNECTION_STRING"]
    async fn qdrant_candidate_store_live_closes_equal_score_boundary_deterministically() {
        let url = env::var("QDRANT_CONNECTION_STRING")
            .expect("QDRANT_CONNECTION_STRING is required for live Qdrant regression");
        let collection_name = format!("cmem_candidate_ties_{}", Uuid::new_v4());
        let store =
            QdrantVectorCandidateStore::new(url, &collection_name, 2).expect("store builds");
        let object_ids = (1..=12).map(Uuid::from_u128).collect::<Vec<_>>();
        let records = object_ids
            .iter()
            .rev()
            .map(|object_id| {
                VectorRecord::new(
                    *object_id,
                    ObjectType::Episode,
                    VectorSurface::Summary,
                    DEFAULT_SCHEMA_VERSION,
                    format!("Equal-score episode {object_id}"),
                )
            })
            .collect::<Vec<_>>();
        let embeddings = vec![vec![1.0, 0.0]; records.len()];
        let record_embeddings = records
            .iter()
            .zip(&embeddings)
            .map(|(record, embedding)| VectorRecordEmbedding::new(record, embedding))
            .collect::<Vec<_>>();

        store.init_collection().await.expect("collection init");
        store
            .upsert_vector_records(&record_embeddings)
            .await
            .expect("upsert succeeds");

        let query = VectorCandidateSearch::new(vec![1.0, 0.0], 5, vec![ObjectType::Episode]);
        let expected = object_ids[..5].to_vec();
        for _ in 0..8 {
            let matches = store
                .search_candidates(&query)
                .await
                .expect("equal-score search succeeds");
            assert_eq!(
                matches
                    .candidates
                    .iter()
                    .map(|candidate| candidate.object_id)
                    .collect::<Vec<_>>(),
                expected
            );
            assert_eq!(
                matches.completeness,
                VectorRecallCompleteness::BoundaryTieClosed { fetched: 12 }
            );
        }

        let _ = store.client.delete_collection(&collection_name).await;
    }

    fn string_value(value: &str) -> Value {
        Value {
            kind: Some(Kind::StringValue(value.to_owned())),
        }
    }
}
