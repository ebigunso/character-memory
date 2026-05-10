use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};

use crate::api::types::RetentionState;
use crate::errors::CustomError;
use crate::internal::repositories::{
    object_type_key, relation_type_key, retention_state_key, RetrievalStatsCounter,
    RetrievalStatsCounterKey, RetrievalStatsEdge, RetrievalStatsHealth, RetrievalStatsHealthState,
    RetrievalStatsObjectState, RetrievalStatsStore,
};

#[derive(Debug)]
pub(crate) struct SqliteRetrievalStatsStore {
    connection: Mutex<Connection>,
}

impl SqliteRetrievalStatsStore {
    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, CustomError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|error| {
                    CustomError::DatabaseError(format!(
                        "failed to create retrieval stats directory {}: {error}",
                        parent.display()
                    ))
                })?;
            }
        }
        let connection = Connection::open(path).map_err(sqlite_error)?;
        initialize_schema(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}

#[async_trait]
impl RetrievalStatsStore for SqliteRetrievalStatsStore {
    async fn record_edges(&self, edges: &[RetrievalStatsEdge]) -> Result<(), CustomError> {
        let mut connection = lock(&self.connection)?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        for edge in edges {
            upsert_edge(&transaction, edge)?;
        }
        set_health(&transaction, RetrievalStatsHealth::default())?;
        transaction.commit().map_err(sqlite_error)
    }

    async fn record_object_states(
        &self,
        states: &[RetrievalStatsObjectState],
    ) -> Result<(), CustomError> {
        let mut connection = lock(&self.connection)?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        for state in states {
            update_object_state(&transaction, state)?;
        }
        set_health(&transaction, RetrievalStatsHealth::default())?;
        transaction.commit().map_err(sqlite_error)
    }

    async fn counter(
        &self,
        key: &RetrievalStatsCounterKey,
    ) -> Result<Option<RetrievalStatsCounter>, CustomError> {
        let connection = lock(&self.connection)?;
        connection
            .query_row(
                "SELECT total_count, active_count, current_count
                 FROM entity_relation_counts
                 WHERE entity_id = ?1 AND relation_kind = ?2 AND object_type = ?3",
                params![
                    key.entity_id.to_string(),
                    relation_type_key(key.relation_kind),
                    object_type_key(key.object_type)
                ],
                |row| {
                    Ok(RetrievalStatsCounter {
                        total_count: non_negative_count(row.get(0)?)?,
                        active_count: non_negative_count(row.get(1)?)?,
                        current_count: non_negative_count(row.get(2)?)?,
                    })
                },
            )
            .optional()
            .map_err(sqlite_error)
    }

    async fn health(&self) -> Result<RetrievalStatsHealth, CustomError> {
        let connection = lock(&self.connection)?;
        let state =
            meta_value(&connection, "health_state")?.unwrap_or_else(|| "healthy".to_owned());
        let last_error_message = meta_value(&connection, "last_error_message")?;
        Ok(RetrievalStatsHealth {
            state: if state == "unhealthy" {
                RetrievalStatsHealthState::Unhealthy
            } else {
                RetrievalStatsHealthState::Healthy
            },
            last_error_message,
        })
    }

    async fn mark_unhealthy(&self, message: String) -> Result<(), CustomError> {
        let mut connection = lock(&self.connection)?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        set_health(
            &transaction,
            RetrievalStatsHealth {
                state: RetrievalStatsHealthState::Unhealthy,
                last_error_message: Some(message),
            },
        )?;
        transaction.commit().map_err(sqlite_error)
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), CustomError> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS entity_edge_index (
                edge_key TEXT PRIMARY KEY,
                entity_id TEXT NOT NULL,
                relation_kind TEXT NOT NULL,
                object_id TEXT NOT NULL,
                object_type TEXT NOT NULL,
                retention_state TEXT NOT NULL,
                is_current INTEGER NOT NULL,
                first_seen_at TEXT NOT NULL,
                last_seen_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS entity_edge_index_object
            ON entity_edge_index(object_id, object_type);

            CREATE TABLE IF NOT EXISTS entity_relation_counts (
                entity_id TEXT NOT NULL,
                relation_kind TEXT NOT NULL,
                object_type TEXT NOT NULL,
                total_count INTEGER NOT NULL DEFAULT 0,
                active_count INTEGER NOT NULL DEFAULT 0,
                current_count INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_id, relation_kind, object_type)
            );

            CREATE TABLE IF NOT EXISTS global_relation_counts (
                relation_kind TEXT NOT NULL,
                object_type TEXT NOT NULL,
                total_count INTEGER NOT NULL DEFAULT 0,
                active_count INTEGER NOT NULL DEFAULT 0,
                current_count INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (relation_kind, object_type)
            );

            CREATE TABLE IF NOT EXISTS stats_meta (
                key TEXT PRIMARY KEY,
                value TEXT
            );
            ",
        )
        .map_err(sqlite_error)?;
    initialize_health_metadata(connection)
}

fn initialize_health_metadata(connection: &Connection) -> Result<(), CustomError> {
    if meta_value(connection, "health_state")?.is_none() {
        set_health(connection, RetrievalStatsHealth::default())?;
    }
    Ok(())
}

fn upsert_edge(connection: &Connection, edge: &RetrievalStatsEdge) -> Result<(), CustomError> {
    let existing = connection
        .query_row(
            "SELECT retention_state, is_current FROM entity_edge_index WHERE edge_key = ?1",
            params![edge.edge_key],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0)),
        )
        .optional()
        .map_err(sqlite_error)?;

    match existing {
        Some((old_retention, old_is_current)) => {
            let old_active = old_retention == "active";
            let new_active = edge.is_active();
            let active_delta = bool_delta(old_active, new_active);
            let current_delta =
                bool_delta(old_active && old_is_current, new_active && edge.is_current);
            connection
                .execute(
                    "UPDATE entity_edge_index
                     SET retention_state = ?2,
                         is_current = ?3,
                         first_seen_at = MIN(first_seen_at, ?4),
                         last_seen_at = MAX(last_seen_at, ?5)
                     WHERE edge_key = ?1",
                    params![
                        edge.edge_key,
                        retention_state_key(edge.retention_state),
                        bool_int(edge.is_current),
                        edge.first_seen_at.to_rfc3339(),
                        edge.last_seen_at.to_rfc3339()
                    ],
                )
                .map_err(sqlite_error)?;
            apply_count_delta(connection, edge, 0, active_delta, current_delta)
        }
        None => {
            connection
                .execute(
                    "INSERT INTO entity_edge_index
                     (edge_key, entity_id, relation_kind, object_id, object_type,
                      retention_state, is_current, first_seen_at, last_seen_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        edge.edge_key,
                        edge.entity_id.to_string(),
                        relation_type_key(edge.relation_kind),
                        edge.object_id.to_string(),
                        object_type_key(edge.object_type),
                        retention_state_key(edge.retention_state),
                        bool_int(edge.is_current),
                        edge.first_seen_at.to_rfc3339(),
                        edge.last_seen_at.to_rfc3339()
                    ],
                )
                .map_err(sqlite_error)?;
            apply_count_delta(
                connection,
                edge,
                1,
                i64::from(edge.is_active()),
                i64::from(edge.is_active() && edge.is_current),
            )
        }
    }
}

fn update_object_state(
    connection: &Connection,
    state: &RetrievalStatsObjectState,
) -> Result<(), CustomError> {
    let mut statement = connection
        .prepare(
            "SELECT edge_key, entity_id, relation_kind, object_type, retention_state, is_current
             FROM entity_edge_index
             WHERE object_id = ?1 AND object_type = ?2",
        )
        .map_err(sqlite_error)?;
    let rows = statement
        .query_map(
            params![
                state.object_id.to_string(),
                object_type_key(state.object_type)
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)? != 0,
                ))
            },
        )
        .map_err(sqlite_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sqlite_error)?;
    drop(statement);

    for (edge_key, entity_id, relation_kind, object_type, old_retention, old_is_current) in rows {
        let old_active = old_retention == "active";
        let new_active = state.retention_state == RetentionState::Active;
        let active_delta = bool_delta(old_active, new_active);
        let current_delta =
            bool_delta(old_active && old_is_current, new_active && state.is_current);
        connection
            .execute(
                "UPDATE entity_edge_index
                 SET retention_state = ?2,
                     is_current = ?3,
                     last_seen_at = MAX(last_seen_at, ?4)
                 WHERE edge_key = ?1",
                params![
                    edge_key,
                    retention_state_key(state.retention_state),
                    bool_int(state.is_current),
                    state.observed_at.to_rfc3339()
                ],
            )
            .map_err(sqlite_error)?;
        apply_count_delta_by_names(
            connection,
            &entity_id,
            &relation_kind,
            &object_type,
            0,
            active_delta,
            current_delta,
        )?;
    }

    Ok(())
}

fn apply_count_delta(
    connection: &Connection,
    edge: &RetrievalStatsEdge,
    total_delta: i64,
    active_delta: i64,
    current_delta: i64,
) -> Result<(), CustomError> {
    apply_count_delta_by_names(
        connection,
        &edge.entity_id.to_string(),
        relation_type_key(edge.relation_kind),
        object_type_key(edge.object_type),
        total_delta,
        active_delta,
        current_delta,
    )
}

fn apply_count_delta_by_names(
    connection: &Connection,
    entity_id: &str,
    relation_kind: &str,
    object_type: &str,
    total_delta: i64,
    active_delta: i64,
    current_delta: i64,
) -> Result<(), CustomError> {
    connection
        .execute(
            "INSERT INTO entity_relation_counts
             (entity_id, relation_kind, object_type, total_count, active_count, current_count)
             VALUES (?1, ?2, ?3, 0, 0, 0)
             ON CONFLICT(entity_id, relation_kind, object_type) DO NOTHING",
            params![entity_id, relation_kind, object_type],
        )
        .map_err(sqlite_error)?;
    connection
        .execute(
            "UPDATE entity_relation_counts
             SET total_count = total_count + ?4,
                 active_count = active_count + ?5,
                 current_count = current_count + ?6
             WHERE entity_id = ?1 AND relation_kind = ?2 AND object_type = ?3",
            params![
                entity_id,
                relation_kind,
                object_type,
                total_delta,
                active_delta,
                current_delta
            ],
        )
        .map_err(sqlite_error)?;

    connection
        .execute(
            "INSERT INTO global_relation_counts
             (relation_kind, object_type, total_count, active_count, current_count)
             VALUES (?1, ?2, 0, 0, 0)
             ON CONFLICT(relation_kind, object_type) DO NOTHING",
            params![relation_kind, object_type],
        )
        .map_err(sqlite_error)?;
    connection
        .execute(
            "UPDATE global_relation_counts
             SET total_count = total_count + ?3,
                 active_count = active_count + ?4,
                 current_count = current_count + ?5
             WHERE relation_kind = ?1 AND object_type = ?2",
            params![
                relation_kind,
                object_type,
                total_delta,
                active_delta,
                current_delta
            ],
        )
        .map_err(sqlite_error)?;
    Ok(())
}

fn set_health(connection: &Connection, health: RetrievalStatsHealth) -> Result<(), CustomError> {
    let state = match health.state {
        RetrievalStatsHealthState::Healthy => "healthy",
        RetrievalStatsHealthState::Unhealthy => "unhealthy",
    };
    set_meta_value(connection, "health_state", Some(state))?;
    set_meta_value(
        connection,
        "last_error_message",
        health.last_error_message.as_deref(),
    )
}

fn set_meta_value(
    connection: &Connection,
    key: &str,
    value: Option<&str>,
) -> Result<(), CustomError> {
    connection
        .execute(
            "INSERT INTO stats_meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(sqlite_error)?;
    Ok(())
}

#[allow(dead_code)]
fn meta_value(connection: &Connection, key: &str) -> Result<Option<String>, CustomError> {
    connection
        .query_row(
            "SELECT value FROM stats_meta WHERE key = ?1",
            params![key],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map(|value| value.flatten())
        .map_err(sqlite_error)
}

fn bool_delta(old: bool, new: bool) -> i64 {
    match (old, new) {
        (false, true) => 1,
        (true, false) => -1,
        _ => 0,
    }
}

fn bool_int(value: bool) -> i64 {
    i64::from(value)
}

fn sqlite_error(error: rusqlite::Error) -> CustomError {
    CustomError::DatabaseError(format!("retrieval stats sqlite error: {error}"))
}

fn non_negative_count(value: i64) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Integer,
            Box::new(CustomError::DatabaseError(format!(
                "retrieval stats counter was negative: {value}"
            ))),
        )
    })
}

fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, CustomError> {
    mutex.lock().map_err(|_| {
        CustomError::DatabaseError("retrieval stats sqlite mutex was poisoned".to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use tempfile::tempdir;

    use crate::api::types::{MemoryId, ObjectType, RelationType};

    #[tokio::test]
    async fn sqlite_store_persists_idempotent_counters() {
        let dir = tempdir().unwrap();
        let store = SqliteRetrievalStatsStore::open(dir.path().join("stats.sqlite3")).unwrap();
        let entity_id = id("550e8400-e29b-41d4-a716-446655461001");
        let episode_id = id("550e8400-e29b-41d4-a716-446655461002");
        let edge = test_edge(entity_id, episode_id, RetentionState::Active, true);

        store
            .record_edges(std::slice::from_ref(&edge))
            .await
            .unwrap();
        store
            .record_edges(std::slice::from_ref(&edge))
            .await
            .unwrap();

        let counter = store
            .counter(&RetrievalStatsCounterKey {
                entity_id,
                relation_kind: RelationType::Involves,
                object_type: ObjectType::Episode,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(counter.total_count, 1);
        assert_eq!(counter.active_count, 1);
        assert_eq!(counter.current_count, 1);
    }

    #[tokio::test]
    async fn sqlite_store_merges_duplicate_edge_timestamps_monotonically() {
        let dir = tempdir().unwrap();
        let store = SqliteRetrievalStatsStore::open(dir.path().join("stats.sqlite3")).unwrap();
        let entity_id = id("550e8400-e29b-41d4-a716-446655461031");
        let episode_id = id("550e8400-e29b-41d4-a716-446655461032");
        let mut later_edge = test_edge(entity_id, episode_id, RetentionState::Active, true);
        later_edge.first_seen_at = timestamp_at("2026-04-28T12:00:00Z");
        later_edge.last_seen_at = timestamp_at("2026-04-28T13:00:00Z");
        let mut earlier_edge = later_edge.clone();
        earlier_edge.first_seen_at = timestamp_at("2026-04-28T11:00:00Z");
        earlier_edge.last_seen_at = timestamp_at("2026-04-28T12:30:00Z");

        store.record_edges(&[later_edge]).await.unwrap();
        store.record_edges(&[earlier_edge]).await.unwrap();

        let connection = lock(&store.connection).unwrap();
        let (first_seen_at, last_seen_at): (String, String) = connection
            .query_row(
                "SELECT first_seen_at, last_seen_at
                 FROM entity_edge_index
                 WHERE edge_key = ?1",
                params![format!("{}:involves:episode:{}", entity_id, episode_id)],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(first_seen_at, "2026-04-28T11:00:00+00:00");
        assert_eq!(last_seen_at, "2026-04-28T13:00:00+00:00");
    }

    #[tokio::test]
    async fn sqlite_store_persists_counters_across_reopen() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("stats.sqlite3");
        let entity_id = id("550e8400-e29b-41d4-a716-446655461021");
        let episode_id = id("550e8400-e29b-41d4-a716-446655461022");
        let edge = test_edge(entity_id, episode_id, RetentionState::Active, true);

        {
            let store = SqliteRetrievalStatsStore::open(&path).unwrap();
            store
                .record_edges(std::slice::from_ref(&edge))
                .await
                .unwrap();
        }

        let reopened = SqliteRetrievalStatsStore::open(&path).unwrap();
        let counter = reopened
            .counter(&RetrievalStatsCounterKey {
                entity_id,
                relation_kind: RelationType::Involves,
                object_type: ObjectType::Episode,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(counter.total_count, 1);
        assert_eq!(counter.active_count, 1);
        assert_eq!(counter.current_count, 1);
        assert_eq!(
            reopened.health().await.unwrap(),
            RetrievalStatsHealth::default()
        );
    }

    #[tokio::test]
    async fn sqlite_store_preserves_unhealthy_marker_across_reopen() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("stats.sqlite3");

        {
            let store = SqliteRetrievalStatsStore::open(&path).unwrap();
            store
                .mark_unhealthy("transient stats failure".to_owned())
                .await
                .unwrap();
        }

        let reopened = SqliteRetrievalStatsStore::open(&path).unwrap();
        let health = reopened.health().await.unwrap();
        assert_eq!(health.state, RetrievalStatsHealthState::Unhealthy);
        assert_eq!(
            health.last_error_message.as_deref(),
            Some("transient stats failure")
        );
    }

    #[tokio::test]
    async fn sqlite_store_updates_lifecycle_counts() {
        let dir = tempdir().unwrap();
        let store = SqliteRetrievalStatsStore::open(dir.path().join("stats.sqlite3")).unwrap();
        let entity_id = id("550e8400-e29b-41d4-a716-446655461011");
        let episode_id = id("550e8400-e29b-41d4-a716-446655461012");
        let edge = test_edge(entity_id, episode_id, RetentionState::Active, true);
        store.record_edges(&[edge]).await.unwrap();

        store
            .record_object_states(&[RetrievalStatsObjectState {
                object_id: episode_id,
                object_type: ObjectType::Episode,
                retention_state: RetentionState::Suppressed,
                is_current: true,
                observed_at: timestamp(),
            }])
            .await
            .unwrap();

        let counter = store
            .counter(&RetrievalStatsCounterKey {
                entity_id,
                relation_kind: RelationType::Involves,
                object_type: ObjectType::Episode,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(counter.total_count, 1);
        assert_eq!(counter.active_count, 0);
        assert_eq!(counter.current_count, 0);
    }

    #[tokio::test]
    async fn sqlite_object_state_updates_do_not_regress_last_seen_at() {
        let dir = tempdir().unwrap();
        let store = SqliteRetrievalStatsStore::open(dir.path().join("stats.sqlite3")).unwrap();
        let entity_id = id("550e8400-e29b-41d4-a716-446655461041");
        let episode_id = id("550e8400-e29b-41d4-a716-446655461042");
        let mut edge = test_edge(entity_id, episode_id, RetentionState::Active, true);
        edge.first_seen_at = timestamp_at("2026-04-28T13:00:00Z");
        edge.last_seen_at = timestamp_at("2026-04-28T13:00:00Z");
        store.record_edges(&[edge]).await.unwrap();

        store
            .record_object_states(&[RetrievalStatsObjectState {
                object_id: episode_id,
                object_type: ObjectType::Episode,
                retention_state: RetentionState::Suppressed,
                is_current: false,
                observed_at: timestamp_at("2026-04-28T11:00:00Z"),
            }])
            .await
            .unwrap();

        let connection = lock(&store.connection).unwrap();
        let last_seen_at: String = connection
            .query_row(
                "SELECT last_seen_at
                 FROM entity_edge_index
                 WHERE edge_key = ?1",
                params![format!("{}:involves:episode:{}", entity_id, episode_id)],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(last_seen_at, "2026-04-28T13:00:00+00:00");
    }

    fn test_edge(
        entity_id: MemoryId,
        object_id: MemoryId,
        retention_state: RetentionState,
        is_current: bool,
    ) -> RetrievalStatsEdge {
        RetrievalStatsEdge {
            edge_key: format!("{}:involves:episode:{}", entity_id, object_id),
            entity_id,
            relation_kind: RelationType::Involves,
            object_id,
            object_type: ObjectType::Episode,
            retention_state,
            is_current,
            first_seen_at: timestamp(),
            last_seen_at: timestamp(),
        }
    }

    fn id(value: &str) -> MemoryId {
        uuid::Uuid::parse_str(value).unwrap()
    }

    fn timestamp() -> DateTime<Utc> {
        timestamp_at("2026-04-28T12:00:00Z")
    }

    fn timestamp_at(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }
}
