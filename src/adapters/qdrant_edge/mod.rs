use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use async_trait::async_trait;
use qdrant_edge::{
    Condition, CountRequest, CreateIndex, Distance, EdgeConfig, EdgeOptimizersConfig, EdgeShard,
    EdgeVectorParams, FieldCondition, FieldIndexOperations, Filter, Match, MatchValue, NamedQuery,
    PayloadFieldSchema, PayloadSchemaType, PointInsertOperations, PointOperations, PointStruct,
    QueryEnum, ScoredPoint, ScrollRequest, SearchParams, SearchRequestBuilder, UpdateOperation,
    ValueVariants, WithPayloadInterface, DEFAULT_VECTOR_NAME,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{oneshot, Mutex};

use crate::adapters::qdrant::payload::{
    qdrant_payload_map, QdrantPayloadKind, QdrantPayloadSchema, OBJECT_ID_FIELD, OBJECT_TYPE_FIELD,
    SURFACE_FIELD,
};
use crate::adapters::qdrant::tie_closure::close_tie_cohort;
use crate::domain::{MemoryId, DEFAULT_SCHEMA_VERSION};
use crate::errors::{
    CollectionCompatibilityError, CollectionMismatch, ConfigValidationError,
    ConfigValidationReason, CustomError, IoErrorKind, VectorDatabaseError, VectorDatabaseErrorKind,
};
use crate::models::vector::{
    CanonicalCandidates, VectorCandidateMatch, VectorCandidateSearch, VectorRecordEmbedding,
};
use crate::ports::vector_candidate::{VectorCandidateRecall, VectorCandidateStore};

const MARKER_FILE: &str = "character-memory.json";
const EDGE_CONFIG_FILE: &str = "edge_config.json";
const INDEXING_THRESHOLD_KB: usize = 0;
const OPEN_ATTEMPTS: usize = 21;
const OPEN_RETRY_DELAY: Duration = Duration::from_millis(50);

#[derive(Debug, Deserialize, Serialize)]
struct ShardMarker {
    collection: String,
    record_schema_version: String,
}

#[derive(Debug)]
pub(crate) struct QdrantEdgeVectorCandidateStore {
    collection_name: String,
    vector_size: usize,
    exact_scan: bool,
    commands: Sender<Command>,
    operation: Mutex<()>,
}

enum Command {
    Upsert {
        points: Vec<qdrant_edge::PointStructPersisted>,
        reply: Reply<()>,
    },
    Delete {
        object_ids: Vec<String>,
        reply: Reply<()>,
    },
    Search {
        query_embedding: Vec<f32>,
        object_types: Vec<String>,
        limit: usize,
        exact: bool,
        zero_norm: bool,
        reply: Reply<Vec<VectorCandidateMatch>>,
    },
    Count {
        object_types: Vec<String>,
        reply: Reply<usize>,
    },
    #[cfg(test)]
    Optimize {
        reply: Reply<bool>,
    },
    Shutdown {
        reply: Option<Reply<()>>,
    },
}

type Reply<T> = oneshot::Sender<Result<T, CustomError>>;

impl QdrantEdgeVectorCandidateStore {
    pub(crate) async fn open(
        root: impl AsRef<Path>,
        collection_name: impl Into<String>,
        vector_size: usize,
    ) -> Result<Self, CustomError> {
        Self::open_with_threshold(root, collection_name, vector_size, INDEXING_THRESHOLD_KB).await
    }

    async fn open_with_threshold(
        root: impl AsRef<Path>,
        collection_name: impl Into<String>,
        vector_size: usize,
        indexing_threshold_kb: usize,
    ) -> Result<Self, CustomError> {
        let collection_name = collection_name.into();
        validate_collection_name(&collection_name)?;

        let root = root.as_ref().to_path_buf();
        let owner_collection = collection_name.clone();
        let (commands, receiver) = mpsc::channel();
        let (ready_tx, ready_rx) = oneshot::channel();
        thread::Builder::new()
            .name(format!("qdrant-edge-{collection_name}"))
            .spawn(move || {
                let shard =
                    open_shard(&root, &owner_collection, vector_size, indexing_threshold_kb);
                match shard {
                    Ok(shard) => {
                        let _ = ready_tx.send(Ok(()));
                        owner_loop(shard, receiver);
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                    }
                }
            })
            .map_err(io_error)?;

        receive(ready_rx).await?;
        Ok(Self {
            collection_name,
            vector_size,
            exact_scan: indexing_threshold_kb == 0,
            commands,
            operation: Mutex::new(()),
        })
    }

    #[cfg(test)]
    async fn optimize(&self) -> Result<bool, CustomError> {
        let _operation = self.operation.lock().await;
        let (reply, receiver) = oneshot::channel();
        self.send(Command::Optimize { reply })?;
        receive(receiver).await
    }

    #[cfg(test)]
    async fn close(&self) -> Result<(), CustomError> {
        let _operation = self.operation.lock().await;
        let (reply, receiver) = oneshot::channel();
        self.send(Command::Shutdown { reply: Some(reply) })?;
        receive(receiver).await
    }

    fn send(&self, command: Command) -> Result<(), CustomError> {
        self.commands.send(command).map_err(|_| owner_unavailable())
    }

    async fn search_batch(
        &self,
        query: &VectorCandidateSearch,
        fetch_limit: usize,
    ) -> Result<Vec<VectorCandidateMatch>, CustomError> {
        let (reply, receiver) = oneshot::channel();
        self.send(Command::Search {
            query_embedding: query.query_embedding.clone(),
            object_types: object_type_tokens(query),
            limit: fetch_limit,
            exact: self.exact_scan,
            zero_norm: query.is_zero_norm(),
            reply,
        })?;
        receive(receiver).await
    }

    async fn scoped_count(&self, query: &VectorCandidateSearch) -> Result<usize, CustomError> {
        let (reply, receiver) = oneshot::channel();
        self.send(Command::Count {
            object_types: object_type_tokens(query),
            reply,
        })?;
        receive(receiver).await
    }
}

#[async_trait]
impl VectorCandidateStore for QdrantEdgeVectorCandidateStore {
    async fn upsert_vector_records(
        &self,
        records: &[VectorRecordEmbedding<'_>],
    ) -> Result<(), CustomError> {
        if records.is_empty() {
            return Ok(());
        }
        let points = records
            .iter()
            .map(|record| self.point(record))
            .collect::<Result<Vec<_>, _>>()?;
        let _operation = self.operation.lock().await;
        let (reply, receiver) = oneshot::channel();
        self.send(Command::Upsert { points, reply })?;
        receive(receiver).await
    }

    async fn search_candidates(
        &self,
        query: &VectorCandidateSearch,
    ) -> Result<VectorCandidateRecall, CustomError> {
        if query.limit == 0 || query.object_types.is_empty() {
            return Ok(VectorCandidateRecall {
                candidates: CanonicalCandidates::new([]),
                completeness: crate::api::types::retrieval::VectorRecallCompleteness::NotRequested,
            });
        }
        if query.query_embedding.len() != self.vector_size {
            return Err(CollectionCompatibilityError {
                collection: self.collection_name.clone(),
                mismatch: CollectionMismatch::VectorSize {
                    expected: self.vector_size as u64,
                    actual: query.query_embedding.len() as u64,
                },
            }
            .into());
        }

        let _operation = self.operation.lock().await;
        let closed = close_tie_cohort(query.limit, usize::MAX, |fetch_limit| {
            self.search_batch(query, fetch_limit)
        })
        .await?;
        let scanned = if self.exact_scan {
            Some(self.scoped_count(query).await?)
        } else {
            None
        };
        Ok(VectorCandidateRecall {
            completeness: closed.completeness(scanned),
            candidates: closed.candidates,
        })
    }

    async fn delete_candidates(&self, object_ids: &[MemoryId]) -> Result<(), CustomError> {
        if object_ids.is_empty() {
            return Ok(());
        }
        let _operation = self.operation.lock().await;
        let (reply, receiver) = oneshot::channel();
        self.send(Command::Delete {
            object_ids: object_ids.iter().map(ToString::to_string).collect(),
            reply,
        })?;
        receive(receiver).await
    }
}

impl QdrantEdgeVectorCandidateStore {
    fn point(
        &self,
        record: &VectorRecordEmbedding<'_>,
    ) -> Result<qdrant_edge::PointStructPersisted, CustomError> {
        if record.embedding.len() != self.vector_size {
            return Err(CollectionCompatibilityError {
                collection: self.collection_name.clone(),
                mismatch: CollectionMismatch::VectorSize {
                    expected: self.vector_size as u64,
                    actual: record.embedding.len() as u64,
                },
            }
            .into());
        }
        let point_id = MemoryId::new_v5(
            &record.record.object_id,
            record.record.surface.to_string().as_bytes(),
        )
        .to_string()
        .parse::<qdrant_edge::PointId>()
        .expect("UUID text is a valid Qdrant Edge point ID");
        let payload = serde_json::Value::Object(qdrant_payload_map(record.record)?);
        Ok(PointStruct::new(point_id, record.embedding.to_vec(), payload).into())
    }
}

impl Drop for QdrantEdgeVectorCandidateStore {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Shutdown { reply: None });
    }
}

fn owner_loop(shard: EdgeShard, commands: mpsc::Receiver<Command>) {
    while let Ok(command) = commands.recv() {
        match command {
            Command::Upsert { points, reply } => {
                let result = shard
                    .update(UpdateOperation::PointOperation(
                        PointOperations::UpsertPoints(PointInsertOperations::PointsList(points)),
                    ))
                    .map_err(edge_error)
                    .and_then(|_| shard.flush().map_err(edge_error));
                let _ = reply.send(result);
            }
            Command::Delete { object_ids, reply } => {
                let filter = string_filter(OBJECT_ID_FIELD, object_ids);
                let result = shard
                    .update(UpdateOperation::PointOperation(
                        PointOperations::DeletePointsByFilter(filter),
                    ))
                    .map_err(edge_error)
                    .and_then(|_| shard.flush().map_err(edge_error));
                let _ = reply.send(result);
            }
            Command::Search {
                query_embedding,
                object_types,
                limit,
                exact,
                zero_norm,
                reply,
            } => {
                let result = search_shard(
                    &shard,
                    query_embedding,
                    object_types,
                    limit,
                    exact,
                    zero_norm,
                );
                let _ = reply.send(result);
            }
            Command::Count {
                object_types,
                reply,
            } => {
                let result = shard
                    .count(CountRequest {
                        filter: Some(string_filter(OBJECT_TYPE_FIELD, object_types)),
                        exact: true,
                    })
                    .map_err(edge_error);
                let _ = reply.send(result);
            }
            #[cfg(test)]
            Command::Optimize { reply } => {
                let result = shard.optimize().map_err(edge_error);
                let _ = reply.send(result);
            }
            Command::Shutdown { reply } => {
                let result = shard.flush().map_err(edge_error);
                if let Some(reply) = reply {
                    let _ = reply.send(result);
                }
                break;
            }
        }
    }
}

fn open_shard(
    root: &Path,
    collection_name: &str,
    vector_size: usize,
    indexing_threshold_kb: usize,
) -> Result<EdgeShard, CustomError> {
    fs::create_dir_all(root).map_err(io_error)?;
    let path = root.join(collection_name);
    fs::create_dir_all(&path).map_err(io_error)?;
    let existing = path.join(EDGE_CONFIG_FILE).is_file();
    if existing {
        validate_marker(&path, collection_name)?;
        let config = EdgeConfig::load(&path)
            .expect("existing config path must produce a load result")
            .map_err(edge_error)?;
        validate_persisted_vector_config(&config, collection_name, vector_size)?;
    }

    let mut last_error = None;
    for attempt in 0..OPEN_ATTEMPTS {
        let result = if path.join(EDGE_CONFIG_FILE).is_file() {
            EdgeShard::load(&path, None)
        } else {
            EdgeShard::new(&path, edge_config(vector_size, indexing_threshold_kb))
        };
        match result {
            Ok(shard) => {
                validate_vector_config(&shard, collection_name, vector_size)?;
                ensure_payload_indexes(&shard)?;
                shard.flush().map_err(edge_error)?;
                if !existing {
                    write_marker(&path, collection_name)?;
                }
                return Ok(shard);
            }
            Err(error) => {
                last_error = Some(error);
                if attempt + 1 < OPEN_ATTEMPTS {
                    thread::sleep(OPEN_RETRY_DELAY);
                }
            }
        }
    }
    Err(edge_error(last_error.expect("open attempts are non-zero")))
}

fn edge_config(vector_size: usize, indexing_threshold_kb: usize) -> EdgeConfig {
    EdgeConfig::builder()
        .vector(
            DEFAULT_VECTOR_NAME,
            EdgeVectorParams::builder(vector_size, Distance::Cosine).build(),
        )
        .optimizers(EdgeOptimizersConfig {
            indexing_threshold: Some(indexing_threshold_kb),
            ..Default::default()
        })
        .build()
}

fn validate_vector_config(
    shard: &EdgeShard,
    collection_name: &str,
    expected_size: usize,
) -> Result<(), CustomError> {
    let config = shard.config();
    validate_persisted_vector_config(&config, collection_name, expected_size)
}

fn validate_persisted_vector_config(
    config: &EdgeConfig,
    collection_name: &str,
    expected_size: usize,
) -> Result<(), CustomError> {
    let Some(vector) = config.vectors.get(DEFAULT_VECTOR_NAME) else {
        return Err(CollectionCompatibilityError {
            collection: collection_name.to_owned(),
            mismatch: CollectionMismatch::MissingVectorConfiguration,
        }
        .into());
    };
    if vector.size != expected_size {
        return Err(CollectionCompatibilityError {
            collection: collection_name.to_owned(),
            mismatch: CollectionMismatch::VectorSize {
                expected: expected_size as u64,
                actual: vector.size as u64,
            },
        }
        .into());
    }
    if vector.distance != Distance::Cosine {
        return Err(CollectionCompatibilityError {
            collection: collection_name.to_owned(),
            mismatch: CollectionMismatch::Distance {
                expected: "Cosine",
                actual: format!("{:?}", vector.distance),
            },
        }
        .into());
    }
    Ok(())
}

fn ensure_payload_indexes(shard: &EdgeShard) -> Result<(), CustomError> {
    let info = shard.info().map_err(edge_error)?;
    for field in QdrantPayloadSchema::indexed_fields() {
        if info
            .payload_schema
            .keys()
            .any(|indexed| indexed.to_string() == field.field.name())
        {
            continue;
        }
        let field_schema = match field.kind {
            QdrantPayloadKind::Keyword => PayloadSchemaType::Keyword,
            QdrantPayloadKind::Text => PayloadSchemaType::Text,
        };
        shard
            .update(UpdateOperation::FieldIndexOperation(
                FieldIndexOperations::CreateIndex(CreateIndex {
                    field_name: field
                        .field
                        .name()
                        .try_into()
                        .expect("manifest fields are valid JSON paths"),
                    field_schema: Some(PayloadFieldSchema::FieldType(field_schema)),
                }),
            ))
            .map_err(edge_error)?;
    }
    Ok(())
}

fn search_shard(
    shard: &EdgeShard,
    query_embedding: Vec<f32>,
    object_types: Vec<String>,
    limit: usize,
    exact: bool,
    zero_norm: bool,
) -> Result<Vec<VectorCandidateMatch>, CustomError> {
    let filter = string_filter(OBJECT_TYPE_FIELD, object_types);
    if zero_norm {
        let (records, _) = shard
            .scroll(ScrollRequest {
                limit: Some(limit),
                filter: Some(filter),
                ..Default::default()
            })
            .map_err(edge_error)?;
        return records
            .into_iter()
            .map(|record| payload_to_match(record.payload.as_ref(), 0.0))
            .collect();
    }

    let request = SearchRequestBuilder::new(
        QueryEnum::Nearest(NamedQuery {
            query: query_embedding.into(),
            using: None,
        }),
        limit,
    )
    .filter(filter)
    .with_payload(WithPayloadInterface::Bool(true))
    .params(SearchParams {
        exact,
        ..Default::default()
    })
    .build();
    shard
        .search(request)
        .map_err(edge_error)?
        .into_iter()
        .map(scored_point_to_match)
        .collect()
}

fn scored_point_to_match(point: ScoredPoint) -> Result<VectorCandidateMatch, CustomError> {
    payload_to_match(point.payload.as_ref(), point.score)
}

fn payload_to_match(
    payload: Option<&qdrant_edge::Payload>,
    score: f32,
) -> Result<VectorCandidateMatch, CustomError> {
    let payload = payload.ok_or_else(|| payload_error("payload is missing"))?;
    let object_id = payload_string(payload, OBJECT_ID_FIELD)?
        .parse()
        .map_err(|error| payload_error(format!("invalid object_id UUID: {error}")))?;
    let object_type = payload_string(payload, OBJECT_TYPE_FIELD)?
        .parse()
        .map_err(|error| payload_error(format!("invalid object_type: {error}")))?;
    let surface = payload_string(payload, SURFACE_FIELD)?
        .parse()
        .map_err(|error| payload_error(format!("invalid surface: {error}")))?;
    Ok(VectorCandidateMatch::new(
        object_id,
        object_type,
        surface,
        score,
    ))
}

fn payload_string<'a>(
    payload: &'a qdrant_edge::Payload,
    field: &str,
) -> Result<&'a str, CustomError> {
    payload
        .0
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| payload_error(format!("missing or invalid string field {field}")))
}

fn string_filter(field: &str, values: Vec<String>) -> Filter {
    let conditions = values
        .into_iter()
        .map(|value| {
            Condition::Field(FieldCondition::new_match(
                field
                    .try_into()
                    .expect("adapter fields are valid JSON paths"),
                Match::Value(MatchValue {
                    value: ValueVariants::String(value),
                }),
            ))
        })
        .collect();
    Filter {
        should: Some(conditions),
        ..Filter::new()
    }
}

fn object_type_tokens(query: &VectorCandidateSearch) -> Vec<String> {
    query.object_types.iter().map(ToString::to_string).collect()
}

fn validate_collection_name(name: &str) -> Result<(), CustomError> {
    let valid_chars = name
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(&byte));
    let first_is_alphanumeric = name
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric);
    let reserved = matches!(
        name,
        "con"
            | "prn"
            | "aux"
            | "nul"
            | "com1"
            | "com2"
            | "com3"
            | "com4"
            | "com5"
            | "com6"
            | "com7"
            | "com8"
            | "com9"
            | "lpt1"
            | "lpt2"
            | "lpt3"
            | "lpt4"
            | "lpt5"
            | "lpt6"
            | "lpt7"
            | "lpt8"
            | "lpt9"
    );
    if name.len() > 128 || !valid_chars || !first_is_alphanumeric || reserved {
        return Err(ConfigValidationError {
            keys: vec!["collection_name"],
            reason: ConfigValidationReason::OutOfDomain {
                expected: "1-128 lowercase ASCII letters, digits, underscores, or hyphens; first character alphanumeric; not a Windows reserved name",
                actual: name.to_owned(),
            },
        }
        .into());
    }
    Ok(())
}

fn validate_marker(path: &Path, collection_name: &str) -> Result<(), CustomError> {
    let marker_path = path.join(MARKER_FILE);
    let mut bytes = Vec::new();
    File::open(&marker_path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(io_error)?;
    let marker: ShardMarker = serde_json::from_slice(&bytes)?;
    if marker.collection != collection_name {
        return Err(CollectionCompatibilityError {
            collection: collection_name.to_owned(),
            mismatch: CollectionMismatch::CollectionName {
                expected: collection_name.to_owned(),
                actual: marker.collection,
            },
        }
        .into());
    }
    if marker.record_schema_version != DEFAULT_SCHEMA_VERSION {
        return Err(CustomError::UnsupportedSchemaVersion {
            context: "Qdrant Edge shard marker",
            expected: DEFAULT_SCHEMA_VERSION,
            actual: marker.record_schema_version,
        });
    }
    Ok(())
}

fn write_marker(path: &Path, collection_name: &str) -> Result<(), CustomError> {
    let marker = ShardMarker {
        collection: collection_name.to_owned(),
        record_schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    };
    let bytes = serde_json::to_vec_pretty(&marker)?;
    let marker_path = path.join(MARKER_FILE);
    let temporary = path.join(format!("{MARKER_FILE}.tmp"));
    let mut file = File::create(&temporary).map_err(io_error)?;
    file.write_all(&bytes).map_err(io_error)?;
    file.sync_all().map_err(io_error)?;
    fs::rename(temporary, marker_path).map_err(io_error)
}

async fn receive<T>(receiver: oneshot::Receiver<Result<T, CustomError>>) -> Result<T, CustomError> {
    receiver.await.map_err(|_| owner_unavailable())?
}

fn owner_unavailable() -> CustomError {
    CustomError::VectorDatabaseError(VectorDatabaseError::new(
        "qdrant_edge",
        VectorDatabaseErrorKind::Engine,
        None,
        "blocking shard owner is unavailable",
    ))
}

fn payload_error(message: impl Into<String>) -> CustomError {
    CustomError::VectorDatabaseError(VectorDatabaseError::new(
        "qdrant_edge",
        VectorDatabaseErrorKind::PayloadDeserialization,
        None,
        message,
    ))
}

fn edge_error(error: impl std::fmt::Display) -> CustomError {
    CustomError::VectorDatabaseError(VectorDatabaseError::new(
        "qdrant_edge",
        VectorDatabaseErrorKind::Engine,
        None,
        error.to_string(),
    ))
}

fn io_error(error: std::io::Error) -> CustomError {
    CustomError::VectorDatabaseError(VectorDatabaseError::new(
        "qdrant_edge",
        VectorDatabaseErrorKind::Io {
            io_kind: IoErrorKind::from(error.kind()),
        },
        None,
        error.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::future::Future;
    use std::path::PathBuf;
    use std::process::Command as ProcessCommand;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    use tempfile::TempDir;

    use super::*;
    use crate::api::types::retrieval::VectorRecallCompleteness;
    use crate::domain::{ObjectType, VectorSurface};
    use crate::models::vector::VectorRecord;

    const CHILD_MODE: &str = "CM_QDRANT_EDGE_CHILD_MODE";
    const CHILD_PATH: &str = "CM_QDRANT_EDGE_CHILD_PATH";

    fn records(count: usize, embedding: &[f32]) -> (Vec<VectorRecord>, Vec<Vec<f32>>) {
        let records = (1..=count)
            .map(|id| {
                VectorRecord::new(
                    MemoryId::from_u128(id as u128),
                    if id % 2 == 0 {
                        ObjectType::Episode
                    } else {
                        ObjectType::Observation
                    },
                    VectorSurface::Summary,
                    DEFAULT_SCHEMA_VERSION,
                    format!("record {id}"),
                )
            })
            .collect::<Vec<_>>();
        let embeddings = (0..count).map(|_| embedding.to_vec()).collect();
        (records, embeddings)
    }

    async fn upsert(
        store: &QdrantEdgeVectorCandidateStore,
        records: &[VectorRecord],
        embeddings: &[Vec<f32>],
    ) {
        let records = records
            .iter()
            .zip(embeddings)
            .map(|(record, embedding)| VectorRecordEmbedding::new(record, embedding))
            .collect::<Vec<_>>();
        store.upsert_vector_records(&records).await.unwrap();
    }

    fn query(limit: usize) -> VectorCandidateSearch {
        VectorCandidateSearch::new(
            vec![1.0, 0.0],
            limit,
            vec![ObjectType::Episode, ObjectType::Observation],
        )
    }

    #[tokio::test]
    async fn zero_limit_or_empty_scope_does_not_request_recall() {
        let temp = TempDir::new().unwrap();
        let store = QdrantEdgeVectorCandidateStore::open(temp.path(), "not_requested", 2)
            .await
            .unwrap();

        for request in [
            query(0),
            VectorCandidateSearch::new(vec![1.0, 0.0], 3, Vec::new()),
        ] {
            let result = store.search_candidates(&request).await.unwrap();
            assert!(result.candidates.is_empty());
            assert_eq!(result.completeness, VectorRecallCompleteness::NotRequested);
        }

        store.close().await.unwrap();
    }

    #[tokio::test]
    async fn collection_names_are_validated_before_the_root_is_touched() {
        for invalid in [
            "",
            ".",
            "../escape",
            "with/slash",
            "with\\slash",
            "Upper",
            "-first",
            "con",
            "com9",
        ] {
            let temp = TempDir::new().unwrap();
            let root = temp.path().join("untouched");
            let error = QdrantEdgeVectorCandidateStore::open(&root, invalid, 2)
                .await
                .expect_err("invalid collection name must fail");
            assert!(matches!(
                error,
                CustomError::ConfigValidation(ConfigValidationError { keys, .. })
                    if keys == vec!["collection_name"]
            ));
            assert!(!root.exists(), "{invalid:?} touched the configured root");
        }

        let too_long = "a".repeat(129);
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("untouched");
        assert!(QdrantEdgeVectorCandidateStore::open(&root, too_long, 2)
            .await
            .is_err());
        assert!(!root.exists());
    }

    #[tokio::test]
    async fn exact_store_restarts_deletes_all_surfaces_and_reports_scoped_count() {
        let temp = TempDir::new().unwrap();
        let (records, embeddings) = records(6, &[1.0, 0.0]);
        let store = QdrantEdgeVectorCandidateStore::open(temp.path(), "restart", 2)
            .await
            .unwrap();
        upsert(&store, &records, &embeddings).await;

        let first = store.search_candidates(&query(3)).await.unwrap();
        assert_eq!(first.candidates.len(), 3);
        assert_eq!(
            first.completeness,
            VectorRecallCompleteness::Exhaustive { scanned: 6 }
        );
        let first_bytes = format!("{first:?}").into_bytes();
        assert_eq!(
            format!("{:?}", store.search_candidates(&query(3)).await.unwrap()).into_bytes(),
            first_bytes
        );

        store
            .delete_candidates(&[records[0].object_id])
            .await
            .unwrap();
        store.close().await.unwrap();

        let reopened = QdrantEdgeVectorCandidateStore::open(temp.path(), "restart", 2)
            .await
            .unwrap();
        let result = reopened.search_candidates(&query(10)).await.unwrap();
        assert_eq!(result.candidates.len(), 5);
        assert!(result
            .candidates
            .iter()
            .all(|candidate| candidate.object_id != records[0].object_id));
        assert_eq!(
            result.completeness,
            VectorRecallCompleteness::Exhaustive { scanned: 5 }
        );
        reopened.close().await.unwrap();
    }

    #[tokio::test]
    async fn zero_norm_query_returns_canonical_zero_scores_exhaustively() {
        let temp = TempDir::new().unwrap();
        let (records, embeddings) = records(4, &[1.0, 0.0]);
        let store = QdrantEdgeVectorCandidateStore::open(temp.path(), "zero_norm", 2)
            .await
            .unwrap();
        upsert(&store, &records, &embeddings).await;

        let result = store
            .search_candidates(&VectorCandidateSearch::new(
                vec![0.0, 0.0],
                4,
                vec![ObjectType::Episode, ObjectType::Observation],
            ))
            .await
            .unwrap();
        assert!(result
            .candidates
            .iter()
            .all(|candidate| candidate.score == 0.0));
        assert_eq!(
            result.completeness,
            VectorRecallCompleteness::Exhaustive { scanned: 4 }
        );
    }

    #[tokio::test]
    async fn reopen_classifies_size_distance_and_marker_mismatches() {
        let temp = TempDir::new().unwrap();
        let store = QdrantEdgeVectorCandidateStore::open(temp.path(), "compatibility", 2)
            .await
            .unwrap();
        store.close().await.unwrap();

        let size_error = QdrantEdgeVectorCandidateStore::open(temp.path(), "compatibility", 3)
            .await
            .expect_err("vector size mismatch must fail");
        assert!(matches!(
            size_error,
            CustomError::CollectionIncompatible(CollectionCompatibilityError {
                mismatch: CollectionMismatch::VectorSize {
                    expected: 3,
                    actual: 2
                },
                ..
            })
        ));

        let shard_path = temp.path().join("compatibility");
        let mut config = EdgeConfig::load(&shard_path).unwrap().unwrap();
        config
            .vectors
            .get_mut(DEFAULT_VECTOR_NAME)
            .unwrap()
            .distance = Distance::Dot;
        config.save(&shard_path).unwrap();
        let distance_error = QdrantEdgeVectorCandidateStore::open(temp.path(), "compatibility", 2)
            .await
            .expect_err("distance mismatch must fail");
        assert!(matches!(
            distance_error,
            CustomError::CollectionIncompatible(CollectionCompatibilityError {
                mismatch: CollectionMismatch::Distance { .. },
                ..
            })
        ));

        config
            .vectors
            .get_mut(DEFAULT_VECTOR_NAME)
            .unwrap()
            .distance = Distance::Cosine;
        config.save(&shard_path).unwrap();
        let marker_path = shard_path.join(MARKER_FILE);
        let mut marker: ShardMarker =
            serde_json::from_slice(&fs::read(&marker_path).unwrap()).unwrap();
        marker.record_schema_version = "future".to_owned();
        fs::write(&marker_path, serde_json::to_vec(&marker).unwrap()).unwrap();
        let marker_error = QdrantEdgeVectorCandidateStore::open(temp.path(), "compatibility", 2)
            .await
            .expect_err("unsupported marker must fail");
        assert!(matches!(
            marker_error,
            CustomError::UnsupportedSchemaVersion {
                context: "Qdrant Edge shard marker",
                expected: DEFAULT_SCHEMA_VERSION,
                actual,
            } if actual == "future"
        ));
    }

    #[tokio::test]
    async fn locked_directory_uses_bounded_backoff_then_reopens_after_close() {
        let temp = TempDir::new().unwrap();
        let first = QdrantEdgeVectorCandidateStore::open(temp.path(), "locked", 2)
            .await
            .unwrap();
        let started = Instant::now();
        let error = QdrantEdgeVectorCandidateStore::open(temp.path(), "locked", 2)
            .await
            .expect_err("second owner must not open a locked shard");
        assert!(matches!(error, CustomError::VectorDatabaseError(_)));
        assert!(started.elapsed() >= Duration::from_millis(900));
        assert!(started.elapsed() < Duration::from_secs(10));

        first.close().await.unwrap();
        let reopened = QdrantEdgeVectorCandidateStore::open(temp.path(), "locked", 2)
            .await
            .unwrap();
        reopened.close().await.unwrap();
    }

    #[tokio::test]
    async fn acknowledged_writes_survive_hard_exit() {
        if env::var(CHILD_MODE).as_deref() == Ok("adapter_flush") {
            let path = PathBuf::from(env::var(CHILD_PATH).unwrap());
            let store = QdrantEdgeVectorCandidateStore::open(&path, "hard_exit", 2)
                .await
                .unwrap();
            let (records, embeddings) = records(3, &[1.0, 0.0]);
            upsert(&store, &records, &embeddings).await;
            std::process::exit(0);
        }

        let temp = TempDir::new().unwrap();
        run_child(
            "acknowledged_writes_survive_hard_exit",
            "adapter_flush",
            temp.path(),
        );
        let reopened = QdrantEdgeVectorCandidateStore::open(temp.path(), "hard_exit", 2)
            .await
            .unwrap();
        let result = reopened.search_candidates(&query(10)).await.unwrap();
        assert_eq!(result.candidates.len(), 3);
        assert_eq!(
            result.completeness,
            VectorRecallCompleteness::Exhaustive { scanned: 3 }
        );
        reopened.close().await.unwrap();
    }

    #[test]
    fn qdrant_edge_0_8_0_contract_canary() {
        if env::var(CHILD_MODE).as_deref() == Ok("raw_no_flush") {
            let path = PathBuf::from(env::var(CHILD_PATH).unwrap()).join("raw");
            fs::create_dir_all(&path).unwrap();
            let shard = EdgeShard::new(&path, edge_config(2, 0)).unwrap();
            let point: qdrant_edge::PointStructPersisted = PointStruct::new(
                1_u64,
                vec![1.0, 0.0],
                serde_json::json!({"object_id": MemoryId::from_u128(1).to_string()}),
            )
            .into();
            shard
                .update(UpdateOperation::PointOperation(
                    PointOperations::UpsertPoints(PointInsertOperations::PointsList(vec![point])),
                ))
                .unwrap();
            std::process::exit(0);
        }

        let temp = TempDir::new().unwrap();
        run_child(
            "qdrant_edge_0_8_0_contract_canary",
            "raw_no_flush",
            temp.path(),
        );
        let raw_path = temp.path().join("raw");
        let shard = EdgeShard::load(&raw_path, None).unwrap();
        assert_eq!(shard.count(CountRequest::new()).unwrap(), 0);
        assert_eq!(shard.info().unwrap().indexed_vectors_count, 0);
        assert!(EdgeShard::load(&raw_path, None).is_err());
        drop(shard);

        let missing = temp.path().join("missing");
        assert!(EdgeShard::new(&missing, edge_config(2, 0)).is_err());
        let non_object_payload = std::panic::catch_unwind(|| {
            PointStruct::new(1_u64, vec![1.0, 0.0], serde_json::json!("not an object"))
        });
        assert!(non_object_payload.is_err());
        assert_ne!(
            std::any::type_name::<qdrant_edge::PointStruct>(),
            std::any::type_name::<qdrant_client::qdrant::PointStruct>()
        );
    }

    #[tokio::test]
    async fn indexed_test_configuration_reports_boundary_and_matches_exact_recall() {
        let temp = TempDir::new().unwrap();
        let (records, embeddings) = records(200, &[1.0, 0.0]);
        let exact = QdrantEdgeVectorCandidateStore::open(temp.path(), "exact", 2)
            .await
            .unwrap();
        let indexed =
            QdrantEdgeVectorCandidateStore::open_with_threshold(temp.path(), "indexed", 2, 1)
                .await
                .unwrap();
        upsert(&exact, &records, &embeddings).await;
        upsert(&indexed, &records, &embeddings).await;
        assert!(indexed.optimize().await.unwrap());

        let exact_result = exact.search_candidates(&query(20)).await.unwrap();
        let indexed_result = indexed.search_candidates(&query(20)).await.unwrap();
        assert_eq!(exact_result.candidates, indexed_result.candidates);
        assert_eq!(
            exact_result.completeness,
            VectorRecallCompleteness::Exhaustive { scanned: 200 }
        );
        assert!(matches!(
            indexed_result.completeness,
            VectorRecallCompleteness::BoundaryTieClosed { .. }
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "manual latency and executor-responsiveness benchmark"]
    async fn benchmark_configured_dimension_and_owner_responsiveness() {
        const DIMENSION: usize = 1_536;
        for count in [100, 1_000, 5_000] {
            let temp = TempDir::new().unwrap();
            let (store, open, open_ticks) = timed_with_heartbeat(
                QdrantEdgeVectorCandidateStore::open(temp.path(), "benchmark", DIMENSION),
            )
            .await;
            let store = store.unwrap();
            let (records, embeddings) = benchmark_records(count, DIMENSION);
            let (_, write, write_ticks) =
                timed_with_heartbeat(upsert(&store, &records, &embeddings)).await;
            let request = VectorCandidateSearch::new(
                benchmark_embedding(0, DIMENSION),
                48,
                vec![ObjectType::Episode, ObjectType::Observation],
            );
            let (recall, scan, scan_ticks) =
                timed_with_heartbeat(store.search_candidates(&request)).await;
            let recall = recall.unwrap();
            let (_, close, close_ticks) = timed_with_heartbeat(store.close()).await;

            println!(
                "qdrant-edge benchmark corpus={count} dimension={DIMENSION} open_ms={} write_ms={} scan_ms={} returned={} heartbeats=open:{open_ticks},write:{write_ticks},scan:{scan_ticks},close:{close_ticks} close_ms={}",
                open.as_millis(),
                write.as_millis(),
                scan.as_millis(),
                recall.candidates.len(),
                close.as_millis(),
            );
        }

        let temp = TempDir::new().unwrap();
        let exact = QdrantEdgeVectorCandidateStore::open(temp.path(), "recall_exact", DIMENSION)
            .await
            .unwrap();
        let indexed = QdrantEdgeVectorCandidateStore::open_with_threshold(
            temp.path(),
            "recall_indexed",
            DIMENSION,
            1,
        )
        .await
        .unwrap();
        let (records, embeddings) = benchmark_records(1_000, DIMENSION);
        upsert(&exact, &records, &embeddings).await;
        upsert(&indexed, &records, &embeddings).await;
        let (_, build, build_ticks) = timed_with_heartbeat(indexed.optimize()).await;
        let request = VectorCandidateSearch::new(
            benchmark_embedding(0, DIMENSION),
            48,
            vec![ObjectType::Episode, ObjectType::Observation],
        );
        let exact_recall = exact.search_candidates(&request).await.unwrap();
        let indexed_recall = indexed.search_candidates(&request).await.unwrap();
        let overlap = indexed_recall
            .candidates
            .iter()
            .filter(|candidate| exact_recall.candidates.contains(candidate))
            .count();
        println!(
            "qdrant-edge indexed recall corpus=1000 overlap={overlap}/{} build_ms={} build_heartbeats={build_ticks}",
            exact_recall.candidates.len(),
            build.as_millis(),
        );
        exact.close().await.unwrap();
        indexed.close().await.unwrap();

        let temp = TempDir::new().unwrap();
        let first = QdrantEdgeVectorCandidateStore::open(temp.path(), "lock_benchmark", 2)
            .await
            .unwrap();
        let (locked, wait, wait_ticks) = timed_with_heartbeat(
            QdrantEdgeVectorCandidateStore::open(temp.path(), "lock_benchmark", 2),
        )
        .await;
        assert!(locked.is_err());
        let (_, close, close_ticks) = timed_with_heartbeat(first.close()).await;
        let (reopened, reopen, reopen_ticks) = timed_with_heartbeat(
            QdrantEdgeVectorCandidateStore::open(temp.path(), "lock_benchmark", 2),
        )
        .await;
        reopened.unwrap().close().await.unwrap();
        println!(
            "qdrant-edge lock wait_ms={} close_ms={} reopen_ms={} heartbeats=wait:{wait_ticks},close:{close_ticks},reopen:{reopen_ticks}",
            wait.as_millis(),
            close.as_millis(),
            reopen.as_millis(),
        );
    }

    fn benchmark_records(count: usize, dimension: usize) -> (Vec<VectorRecord>, Vec<Vec<f32>>) {
        let records = (1..=count)
            .map(|value| {
                VectorRecord::new(
                    MemoryId::from_u128(value as u128),
                    if value % 2 == 0 {
                        ObjectType::Episode
                    } else {
                        ObjectType::Observation
                    },
                    VectorSurface::Summary,
                    DEFAULT_SCHEMA_VERSION,
                    format!("benchmark record {value}"),
                )
            })
            .collect();
        let embeddings = (1..=count)
            .map(|value| benchmark_embedding(value, dimension))
            .collect();
        (records, embeddings)
    }

    fn benchmark_embedding(value: usize, dimension: usize) -> Vec<f32> {
        let mut embedding = vec![0.0; dimension];
        embedding[0] = 1.0;
        embedding[1] = value as f32 / 10_000.0;
        embedding
    }

    async fn timed_with_heartbeat<T>(future: impl Future<Output = T>) -> (T, Duration, usize) {
        let running = Arc::new(AtomicBool::new(true));
        let ticks = Arc::new(AtomicUsize::new(0));
        let heartbeat = tokio::spawn({
            let running = Arc::clone(&running);
            let ticks = Arc::clone(&ticks);
            async move {
                while running.load(Ordering::Relaxed) {
                    ticks.fetch_add(1, Ordering::Relaxed);
                    tokio::task::yield_now().await;
                }
            }
        });
        let started = Instant::now();
        let output = future.await;
        let elapsed = started.elapsed();
        running.store(false, Ordering::Relaxed);
        heartbeat.await.unwrap();
        let ticks = ticks.load(Ordering::Relaxed);
        assert!(ticks > 0, "the async executor made no concurrent progress");
        (output, elapsed, ticks)
    }

    fn run_child(test_name: &str, mode: &str, path: &Path) {
        let full_name = format!("adapters::qdrant_edge::tests::{test_name}");
        let status = ProcessCommand::new(env::current_exe().unwrap())
            .args(["--exact", &full_name, "--nocapture"])
            .env(CHILD_MODE, mode)
            .env(CHILD_PATH, path)
            .status()
            .unwrap();
        assert!(status.success(), "child mode {mode} failed: {status}");
    }
}
