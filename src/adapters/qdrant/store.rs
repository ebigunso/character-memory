// Qdrant candidate-store adapter. Qdrant provides vector recall and
// payload prefiltering; Oxigraph remains authoritative for graph/lifecycle
// truth.
use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use qdrant_client::qdrant::{
    points_selector::PointsSelectorOneOf, value::Kind, vectors_config, Condition,
    CreateCollectionBuilder, CreateFieldIndexCollectionBuilder, DeletePointsBuilder, Distance,
    Filter, PointStruct, ScoredPoint, SearchPointsBuilder, UpsertPointsBuilder, VectorParams,
    VectorsConfig,
};
use qdrant_client::{config::QdrantConfig, Qdrant, QdrantError};

use crate::domain::{MemoryId, ObjectType};
use crate::errors::{
    CollectionCompatibilityError, CollectionMismatch, CustomError, IoErrorKind, TransportStatus,
    VectorDatabaseError, VectorDatabaseErrorKind,
};
use crate::models::vector::{
    CanonicalCandidates, VectorCandidateMatch, VectorCandidateSearch, VectorRecordEmbedding,
    VectorSurface,
};
use crate::ports::vector_candidate::VectorCandidateStore;

use super::payload::{
    qdrant_payload_map, QdrantPayloadSchema, OBJECT_ID_FIELD, OBJECT_TYPE_FIELD, SURFACE_FIELD,
};

const QDRANT_CANDIDATE_TIMEOUT_SECS: u64 = 30;
const QDRANT_TIE_COHORT_MIN_EXTRA_CANDIDATES: usize = 4_096;
const QDRANT_TIE_COHORT_LIMIT_MULTIPLIER: usize = 16;
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

        if let Some(filter) = qdrant_candidate_filter(query) {
            builder = builder.filter(filter);
        }

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TieCohortFetchDecision {
    Return,
    ReturnAtBound,
    Grow(usize),
}

fn tie_cohort_fetch_bound(limit: usize) -> usize {
    limit
        .saturating_mul(QDRANT_TIE_COHORT_LIMIT_MULTIPLIER)
        .max(limit.saturating_add(QDRANT_TIE_COHORT_MIN_EXTRA_CANDIDATES))
}

fn tie_cohort_fetch_decision(
    admitted_limit: usize,
    fetch_limit: usize,
    fetch_bound: usize,
    fetched_count: usize,
    candidates: &[VectorCandidateMatch],
) -> TieCohortFetchDecision {
    if fetched_count < fetch_limit || tie_cohort_is_closed(candidates, admitted_limit) {
        return TieCohortFetchDecision::Return;
    }
    if fetch_limit >= fetch_bound {
        return TieCohortFetchDecision::ReturnAtBound;
    }

    TieCohortFetchDecision::Grow(fetch_limit.saturating_mul(2).min(fetch_bound))
}

fn tie_cohort_is_closed(candidates: &[VectorCandidateMatch], admitted_limit: usize) -> bool {
    if admitted_limit == 0 || candidates.len() <= admitted_limit {
        return false;
    }

    candidates.last().is_some_and(|tail| {
        tail.score
            .total_cmp(&candidates[admitted_limit - 1].score)
            .is_lt()
    })
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
    ) -> Result<CanonicalCandidates, CustomError> {
        if query.limit == 0 {
            return Ok(CanonicalCandidates::new([]));
        }

        // Fetch past K until the boundary tie is closed. Growth is bounded by
        // max(K * 16, K + 4096), which avoids unbounded reads when an entire
        // collection ties. At the bound, results are canonical and deterministic
        // for the fetched set, but membership can still vary if the equal-score
        // cohort itself exceeds the bound. A future adapter-observability channel
        // in the RetrievalTrace family should surface that degradation.
        let fetch_bound = tie_cohort_fetch_bound(query.limit);
        let mut fetch_limit = query.limit.saturating_add(1).min(fetch_bound);
        loop {
            let fetched = self.search_candidate_batch(query, fetch_limit).await?;
            let fetched_count = fetched.len();
            let candidates = CanonicalCandidates::new(fetched);

            match tie_cohort_fetch_decision(
                query.limit,
                fetch_limit,
                fetch_bound,
                fetched_count,
                &candidates,
            ) {
                TieCohortFetchDecision::Grow(next_limit) => fetch_limit = next_limit,
                TieCohortFetchDecision::Return | TieCohortFetchDecision::ReturnAtBound => {
                    return Ok(candidates.truncated(query.limit));
                }
            }
        }
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
            let kind = if error.is_timeout() {
                VectorDatabaseErrorKind::HttpTimeout
            } else if error.is_connect() {
                VectorDatabaseErrorKind::HttpConnect
            } else if error.is_status() {
                VectorDatabaseErrorKind::HttpStatus
            } else {
                VectorDatabaseErrorKind::Http
            };
            VectorDatabaseError::new("qdrant", kind, None, error.to_string())
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
    // Ruled external-contract exception: qdrant-client 1.17.0 erases the tonic transport
    // source in src/channel_pool.rs with
    // `Status::internal(format!("Failed to connect to {}: {:?}", self.uri, e))`.
    // Recheck this on every qdrant-client bump; retire the prefix coupling once upstream
    // preserves a downcastable source.
    *status == TransportStatus::Internal && message.starts_with(QDRANT_CONNECT_FAILURE_PREFIX)
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

fn qdrant_candidate_filter(query: &VectorCandidateSearch) -> Option<Filter> {
    (!query.object_types.is_empty()).then(|| {
        Filter::must([any_field_matches(
            OBJECT_TYPE_FIELD,
            query.object_types.iter().copied().map(object_type_name),
        )])
    })
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
    let object_id = payload_string(&point.payload, OBJECT_ID_FIELD)?;
    let object_id = uuid::Uuid::parse_str(&object_id).map_err(|error| {
        CustomError::DatabaseError(format!("Invalid Qdrant object_id payload UUID: {error}"))
    })?;

    let object_type = parse_object_type(payload_string(&point.payload, OBJECT_TYPE_FIELD)?)?;
    let surface = parse_vector_surface(payload_string(&point.payload, SURFACE_FIELD)?)?;

    Ok(VectorCandidateMatch::new(
        object_id,
        object_type,
        surface,
        point.score,
    ))
}

fn qdrant_point_id(record: &crate::models::vector::VectorRecord) -> uuid::Uuid {
    let mut first = 0xcbf29ce484222325_u64;
    let mut second = 0x9e3779b97f4a7c15_u64;

    for byte in record
        .object_id
        .as_bytes()
        .iter()
        .copied()
        .chain(surface_name(record.surface).as_bytes().iter().copied())
    {
        first ^= u64::from(byte);
        first = first.wrapping_mul(0x100000001b3);
        second ^= u64::from(byte).wrapping_add(0x9e3779b97f4a7c15);
        second = second.rotate_left(5).wrapping_mul(0x517cc1b727220a95);
    }

    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&first.to_be_bytes());
    bytes[8..].copy_from_slice(&second.to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes)
}

fn payload_string(
    payload: &HashMap<String, qdrant_client::qdrant::Value>,
    field: &str,
) -> Result<String, CustomError> {
    match payload.get(field).and_then(|value| value.kind.as_ref()) {
        Some(Kind::StringValue(value)) => Ok(value.clone()),
        _ => Err(CustomError::DatabaseError(format!(
            "Missing or invalid string field in Qdrant payload: {field}"
        ))),
    }
}

fn object_type_name(object_type: ObjectType) -> &'static str {
    match object_type {
        ObjectType::Episode => "episode",
        ObjectType::Observation => "observation",
        ObjectType::Entity => "entity",
        ObjectType::MemoryThread => "memory_thread",
        ObjectType::DerivedMemory => "derived_memory",
        ObjectType::MemoryLink => "memory_link",
    }
}

fn surface_name(surface: VectorSurface) -> &'static str {
    match surface {
        VectorSurface::Summary => "summary",
        VectorSurface::Text => "text",
        VectorSurface::Name => "name",
        VectorSurface::DerivedText => "derived_text",
        VectorSurface::Query => "query",
    }
}

fn parse_object_type(value: String) -> Result<ObjectType, CustomError> {
    match value.as_str() {
        "episode" => Ok(ObjectType::Episode),
        "observation" => Ok(ObjectType::Observation),
        "entity" => Ok(ObjectType::Entity),
        "memory_thread" => Ok(ObjectType::MemoryThread),
        "derived_memory" => Ok(ObjectType::DerivedMemory),
        "memory_link" => Ok(ObjectType::MemoryLink),
        _ => Err(CustomError::DatabaseError(format!(
            "Unknown Qdrant object_type payload value: {value}"
        ))),
    }
}

fn parse_vector_surface(value: String) -> Result<VectorSurface, CustomError> {
    match value.as_str() {
        "summary" => Ok(VectorSurface::Summary),
        "text" => Ok(VectorSurface::Text),
        "name" => Ok(VectorSurface::Name),
        "derived_text" => Ok(VectorSurface::DerivedText),
        "query" => Ok(VectorSurface::Query),
        _ => Err(CustomError::DatabaseError(format!(
            "Unknown Qdrant surface payload value: {value}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::super::payload::{
        CONTENT_TEXT_FIELD, GRAPH_URI_FIELD, IS_CURRENT_FIELD, RETENTION_STATE_FIELD,
    };
    use super::*;
    use crate::domain::{graph_uri, RetentionState, DEFAULT_SCHEMA_VERSION};
    use crate::models::vector::{
        VectorRecord, VectorRecordEmbedding, VectorRelationshipHints, VectorSurface,
    };
    use qdrant_client::qdrant::condition::ConditionOneOf;
    use qdrant_client::qdrant::{
        point_id::PointIdOptions, value::Kind, vector, vectors, DeleteCollectionBuilder, PointId,
        Value, VectorParamsMap,
    };
    use std::env;
    use std::time::Instant;
    use uuid::Uuid;

    #[test]
    fn candidate_client_config_extends_default_request_timeout() {
        let config = qdrant_candidate_config("http://localhost:6334");

        assert_eq!(
            config.timeout,
            Duration::from_secs(QDRANT_CANDIDATE_TIMEOUT_SECS)
        );
        assert_eq!(config.uri, "http://localhost:6334");
        assert!(config.keep_alive_while_idle);
    }

    #[test]
    fn candidate_filter_maps_live_object_type_scope() {
        assert!(qdrant_candidate_filter(&VectorCandidateSearch::new(vec![1.0, 0.0], 10)).is_none());

        let query = VectorCandidateSearch::new(vec![1.0, 0.0], 10)
            .with_object_types(vec![ObjectType::Episode]);
        let filter = qdrant_candidate_filter(&query).expect("object-type scope should build");
        let Some(ConditionOneOf::Field(field)) = &filter.must[0].condition_one_of else {
            panic!("single object type should map to a field condition");
        };

        assert_eq!(field.key, OBJECT_TYPE_FIELD);
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
                message,
                retry_after_seconds: None,
            }) if backend == "qdrant" && message == "offline"
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
    fn qdrant_connect_prefix_parser_fixture() {
        // This isolates our sanctioned parser behavior; the dependency-bound canary below
        // verifies that qdrant-client still emits the parsed shape.
        let error = qdrant_error(QdrantError::ResponseError {
            status: tonic::Status::internal(
                "Failed to connect to http://127.0.0.1:65534/: tonic transport failure",
            ),
        });

        assert!(matches!(
            error,
            CustomError::VectorDatabaseError(VectorDatabaseError {
                kind: VectorDatabaseErrorKind::HttpConnect,
                ..
            })
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn qdrant_client_1_17_erased_connect_contract_canary() {
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
            graph_uri(object_type, object_id),
            VectorSurface::Summary,
            "Idle-gap regression record",
            "Idle-gap regression record",
            DEFAULT_SCHEMA_VERSION,
            Some(RetentionState::Active),
            Some(true),
            VectorRelationshipHints::default(),
            None,
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
                (SURFACE_FIELD.to_owned(), string_value("derived_text")),
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
            graph_uri(ObjectType::Episode, object_id),
            VectorSurface::Summary,
            "Episode summary.",
            "Episode summary.",
            DEFAULT_SCHEMA_VERSION,
            None,
            None,
            VectorRelationshipHints::default(),
            None,
        );
        let text = VectorRecord::new(
            object_id,
            ObjectType::Episode,
            graph_uri(ObjectType::Episode, object_id),
            VectorSurface::Text,
            "Episode text.",
            "Episode text.",
            DEFAULT_SCHEMA_VERSION,
            None,
            None,
            VectorRelationshipHints::default(),
            None,
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
    fn upsert_points_use_full_vector_record_payloads() {
        let object_id = Uuid::new_v4();
        let related_episode_id = Uuid::new_v4();
        let record = VectorRecord::new(
            object_id,
            ObjectType::DerivedMemory,
            graph_uri(ObjectType::DerivedMemory, object_id),
            VectorSurface::DerivedText,
            "Reflection: Qdrant keeps payload details.",
            "Qdrant keeps payload details.",
            DEFAULT_SCHEMA_VERSION,
            None,
            Some(true),
            VectorRelationshipHints {
                episode_ids: vec![related_episode_id],
                ..VectorRelationshipHints::default()
            },
            Some("raw://conversation/chat_123#turn_42".to_owned()),
        );

        let points = qdrant_point_structs(&[VectorRecordEmbedding::new(&record, &[0.25, 0.75])])
            .expect("points build");

        assert_eq!(points.len(), 1);
        assert_eq!(
            payload_string(&points[0].payload, OBJECT_TYPE_FIELD).unwrap(),
            "derived_memory"
        );
        assert_eq!(
            payload_string(&points[0].payload, SURFACE_FIELD).unwrap(),
            "derived_text"
        );
        assert_eq!(
            payload_string(&points[0].payload, GRAPH_URI_FIELD).unwrap(),
            record.graph_uri
        );
        assert_eq!(
            payload_string(&points[0].payload, CONTENT_TEXT_FIELD).unwrap(),
            "Qdrant keeps payload details."
        );
        assert!(points[0].payload.contains_key("episode_ids"));
        assert!(points[0].payload.contains_key("raw_ref"));

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

    #[test]
    fn candidate_mapping_can_be_canonicalized_independently_of_qdrant_order() {
        let higher_score_id = Uuid::from_u128(3);
        let first_tied_id = Uuid::from_u128(1);
        let second_tied_id = Uuid::from_u128(2);
        let points = vec![
            scored_point(second_tied_id, ObjectType::DerivedMemory, 0.42),
            scored_point(higher_score_id, ObjectType::DerivedMemory, 0.91),
            scored_point(first_tied_id, ObjectType::DerivedMemory, 0.42),
        ];

        let matches = CanonicalCandidates::new(
            points
                .into_iter()
                .map(scored_point_to_match)
                .collect::<Result<Vec<_>, _>>()
                .expect("points map"),
        );

        assert_eq!(matches[0].object_id, higher_score_id);
        assert_eq!(matches[0].score, 0.91);
        assert_eq!(matches[1].object_id, first_tied_id);
        assert_eq!(matches[1].score, 0.42);
        assert_eq!(matches[2].object_id, second_tied_id);
    }

    #[test]
    fn all_tied_cohort_at_fetch_bound_degrades_to_canonical_fetched_membership() {
        let admitted_limit = 2;
        let fetched = (1..=6)
            .rev()
            .map(|value| {
                VectorCandidateMatch::new(
                    Uuid::from_u128(value),
                    ObjectType::Episode,
                    VectorSurface::Summary,
                    1.0,
                )
            })
            .collect::<Vec<_>>();
        let candidates = CanonicalCandidates::new(fetched);
        let fetch_bound = candidates.len();

        assert_eq!(
            tie_cohort_fetch_decision(
                admitted_limit,
                fetch_bound,
                fetch_bound,
                fetch_bound,
                &candidates,
            ),
            TieCohortFetchDecision::ReturnAtBound
        );

        let candidates = candidates.truncated(admitted_limit);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.object_id)
                .collect::<Vec<_>>(),
            vec![Uuid::from_u128(1), Uuid::from_u128(2)]
        );
    }

    #[test]
    fn candidate_mapping_does_not_return_lifecycle_hints_as_authority() {
        let object_id = Uuid::new_v4();
        let mut point = scored_point(object_id, ObjectType::DerivedMemory, 0.77);
        point
            .payload
            .insert(RETENTION_STATE_FIELD.to_owned(), string_value("active"));
        point
            .payload
            .insert(IS_CURRENT_FIELD.to_owned(), bool_value(true));

        let matched = scored_point_to_match(point).expect("point maps");

        assert_eq!(matched.object_id, object_id);
        assert_eq!(matched.object_type, ObjectType::DerivedMemory);
        assert_eq!(matched.surface, VectorSurface::DerivedText);
    }

    #[tokio::test]
    #[ignore = "requires local Qdrant: docker compose -f docker-compose.qdrant.yml up -d and QDRANT_CONNECTION_STRING"]
    async fn qdrant_candidate_store_live_smoke_upserts_searches_and_deletes() {
        let url = env::var("QDRANT_CONNECTION_STRING")
            .expect("QDRANT_CONNECTION_STRING is required for live Qdrant smoke test");
        let collection_name = format!("cmem_candidate_smoke_{}", Uuid::new_v4());
        let store =
            QdrantVectorCandidateStore::new(url, &collection_name, 2).expect("store builds");

        let object_id = Uuid::new_v4();
        let record = VectorRecord::new(
            object_id,
            ObjectType::DerivedMemory,
            graph_uri(ObjectType::DerivedMemory, object_id),
            VectorSurface::DerivedText,
            "Reflection: Qdrant keeps filter hints.",
            "Qdrant keeps filter hints.",
            DEFAULT_SCHEMA_VERSION,
            Some(RetentionState::Active),
            Some(true),
            VectorRelationshipHints::default(),
            None,
        );

        store.init_collection().await.expect("collection init");
        store
            .upsert_vector_records(&[VectorRecordEmbedding::new(&record, &[1.0, 0.0])])
            .await
            .expect("upsert succeeds");

        let matches = store
            .search_candidates(
                &VectorCandidateSearch::new(vec![1.0, 0.0], 1)
                    .with_object_types(vec![ObjectType::DerivedMemory]),
            )
            .await
            .expect("search succeeds");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].object_id, object_id);

        store
            .delete_candidates(&[object_id])
            .await
            .expect("delete succeeds");
        let _ = store.client.delete_collection(&collection_name).await;
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
                    graph_uri(ObjectType::Episode, *object_id),
                    VectorSurface::Summary,
                    format!("Equal-score episode {object_id}"),
                    format!("Equal-score episode {object_id}"),
                    DEFAULT_SCHEMA_VERSION,
                    Some(RetentionState::Active),
                    Some(true),
                    VectorRelationshipHints::default(),
                    None,
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

        let query = VectorCandidateSearch::new(vec![1.0, 0.0], 5)
            .with_object_types(vec![ObjectType::Episode]);
        let expected = object_ids[..5].to_vec();
        for _ in 0..8 {
            let matches = store
                .search_candidates(&query)
                .await
                .expect("equal-score search succeeds");
            assert_eq!(
                matches
                    .iter()
                    .map(|candidate| candidate.object_id)
                    .collect::<Vec<_>>(),
                expected
            );
        }

        let _ = store.client.delete_collection(&collection_name).await;
    }

    fn scored_point(object_id: Uuid, object_type: ObjectType, score: f32) -> ScoredPoint {
        ScoredPoint {
            id: Some(PointId {
                point_id_options: Some(PointIdOptions::Uuid(Uuid::new_v4().to_string())),
            }),
            payload: HashMap::from([
                (
                    OBJECT_ID_FIELD.to_owned(),
                    string_value(&object_id.to_string()),
                ),
                (
                    OBJECT_TYPE_FIELD.to_owned(),
                    string_value(object_type_name(object_type)),
                ),
                (SURFACE_FIELD.to_owned(), string_value("derived_text")),
            ]),
            score,
            ..Default::default()
        }
    }

    fn string_value(value: &str) -> Value {
        Value {
            kind: Some(Kind::StringValue(value.to_owned())),
        }
    }

    fn bool_value(value: bool) -> Value {
        Value {
            kind: Some(Kind::BoolValue(value)),
        }
    }
}
