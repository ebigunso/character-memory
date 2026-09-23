use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::{ObjectType, RelationType, RetentionState};
use crate::errors::{IoErrorKind, RetrievalStatsHealthCause, RetrievalStatsStoreError};
use crate::ports::retrieval_stats::{
    object_type_key, relation_type_key, retention_state_key, RetrievalStatsCounter,
    RetrievalStatsCounterKey, RetrievalStatsEdge, RetrievalStatsHealth, RetrievalStatsHealthState,
    RetrievalStatsObjectState, RetrievalStatsStore,
};

#[derive(Debug)]
pub(crate) struct SqliteRetrievalStatsStore {
    connection: Mutex<Connection>,
}

impl SqliteRetrievalStatsStore {
    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, RetrievalStatsStoreError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|error| {
                    RetrievalStatsStoreError::Filesystem {
                        io_kind: IoErrorKind::from(error.kind()),
                        detail: format!(
                            "failed to create retrieval stats directory {}: {error}",
                            parent.display()
                        ),
                    }
                })?;
            }
        }
        let mut connection = Connection::open(path).map_err(sqlite_error)?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        initialize_schema(&transaction)?;
        transaction.commit().map_err(sqlite_error)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}

#[async_trait]
impl RetrievalStatsStore for SqliteRetrievalStatsStore {
    async fn record_edges(
        &self,
        edges: &[RetrievalStatsEdge],
    ) -> Result<(), RetrievalStatsStoreError> {
        let mut connection = lock(&self.connection)?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        for edge in edges {
            if is_episode_presence(edge.relation_kind, edge.object_type) {
                upsert_episode_presence(&transaction, edge)?;
            } else {
                upsert_edge(&transaction, edge)?;
            }
        }
        transaction.commit().map_err(sqlite_error)
    }

    async fn record_object_states(
        &self,
        states: &[RetrievalStatsObjectState],
    ) -> Result<(), RetrievalStatsStoreError> {
        let mut connection = lock(&self.connection)?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        for state in states {
            if state.object_type == ObjectType::Episode {
                transaction.execute(
                    "UPDATE episode_presence_index SET retention_state = ?2, is_current = ?3 WHERE episode_id = ?1",
                    params![state.object_id.to_string(), retention_state_key(state.retention_state), bool_int(state.is_current)],
                ).map_err(sqlite_error)?;
                let previous = transaction
                    .query_row(
                        "SELECT retention_state FROM episode_state_index WHERE episode_id = ?1",
                        [state.object_id.to_string()],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(sqlite_error)?
                    .map(|value| retention_from_key(&value))
                    .transpose()?;
                transaction.execute(
                    "INSERT INTO episode_state_index (episode_id, retention_state) VALUES (?1, ?2)
                     ON CONFLICT(episode_id) DO UPDATE SET retention_state = excluded.retention_state",
                    params![state.object_id.to_string(), retention_state_key(state.retention_state)],
                ).map_err(sqlite_error)?;
                transaction
                    .execute(
                        "UPDATE episode_counts SET total_count = total_count + ?1,
                        active_count = active_count + ?2 WHERE singleton = 1",
                        params![
                            i64::from(previous.is_none()),
                            bool_delta(
                                previous == Some(RetentionState::Active),
                                state.retention_state == RetentionState::Active
                            ),
                        ],
                    )
                    .map_err(sqlite_error)?;
            }
            update_object_state(&transaction, state)?;
        }
        transaction.commit().map_err(sqlite_error)
    }

    async fn counter(
        &self,
        key: &RetrievalStatsCounterKey,
    ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
        let connection = lock(&self.connection)?;
        if is_episode_presence(key.relation_kind, key.object_type) {
            return episode_presence_counter(&connection, key.entity_id.to_string());
        }
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
                raw_counter_row,
            )
            .optional()
            .map_err(sqlite_error)?
            .map(counter_from_raw)
            .transpose()
    }

    async fn global_counter(
        &self,
        relation_kind: RelationType,
        object_type: ObjectType,
    ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
        let connection = lock(&self.connection)?;
        connection
            .query_row(
                "SELECT total_count, active_count, current_count
                 FROM global_relation_counts
                 WHERE relation_kind = ?1 AND object_type = ?2",
                params![
                    relation_type_key(relation_kind),
                    object_type_key(object_type)
                ],
                raw_counter_row,
            )
            .optional()
            .map_err(sqlite_error)?
            .map(counter_from_raw)
            .transpose()
    }

    async fn global_episode_counter(
        &self,
    ) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
        let connection = lock(&self.connection)?;
        connection
            .query_row(
                "SELECT total_count, active_count, active_count FROM episode_counts WHERE singleton = 1",
                [],
                raw_counter_row,
            )
            .map_err(sqlite_error)
            .and_then(counter_from_raw)
            .map(Some)
    }

    async fn health(&self) -> Result<RetrievalStatsHealth, RetrievalStatsStoreError> {
        let connection = lock(&self.connection)?;
        let state =
            meta_value(&connection, "health_state")?.unwrap_or_else(|| "healthy".to_owned());
        let last_error_cause = meta_value(&connection, "last_error_cause")?
            .map(|value| {
                serde_json::from_str(&value).map_err(|error| {
                    RetrievalStatsStoreError::HealthDeserialization {
                        detail: error.to_string(),
                    }
                })
            })
            .transpose()?;
        Ok(RetrievalStatsHealth {
            state: if state == "unhealthy" {
                RetrievalStatsHealthState::Unhealthy
            } else {
                RetrievalStatsHealthState::Healthy
            },
            last_error_cause,
        })
    }

    async fn mark_unhealthy(
        &self,
        cause: RetrievalStatsHealthCause,
    ) -> Result<(), RetrievalStatsStoreError> {
        let mut connection = lock(&self.connection)?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        set_health(
            &transaction,
            RetrievalStatsHealth {
                state: RetrievalStatsHealthState::Unhealthy,
                last_error_cause: Some(cause),
            },
        )?;
        transaction.commit().map_err(sqlite_error)
    }
}

fn is_episode_presence(relation: RelationType, object_type: ObjectType) -> bool {
    relation == RelationType::Involves && object_type == ObjectType::Episode
}

fn upsert_episode_presence(
    connection: &Connection,
    edge: &RetrievalStatsEdge,
) -> Result<(), RetrievalStatsStoreError> {
    connection.execute(
        "INSERT INTO episode_presence_index
         (edge_key, entity_id, episode_id, retention_state, is_current)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(edge_key) DO UPDATE SET
             retention_state = CASE WHEN episode_presence_index.retention_state = 'suppressed' THEN 'suppressed' ELSE excluded.retention_state END,
             is_current = episode_presence_index.is_current AND excluded.is_current",
        params![edge.edge_key, edge.entity_id.to_string(), edge.object_id.to_string(),
            retention_state_key(edge.retention_state), bool_int(edge.is_current)],
    ).map_err(sqlite_error)?;
    Ok(())
}

fn episode_presence_counter(
    connection: &Connection,
    entity_id: String,
) -> Result<Option<RetrievalStatsCounter>, RetrievalStatsStoreError> {
    let counter = connection
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(active), 0), COALESCE(SUM(current), 0) FROM (
            SELECT MAX(retention_state = 'active') AS active,
                   MAX(retention_state = 'active' AND is_current) AS current
            FROM episode_presence_index WHERE entity_id = ?1
            GROUP BY entity_id, episode_id
        )",
            [entity_id],
            raw_counter_row,
        )
        .map_err(sqlite_error)
        .and_then(counter_from_raw)?;
    Ok((counter.total_count > 0).then_some(counter))
}

fn initialize_schema(connection: &Connection) -> Result<(), RetrievalStatsStoreError> {
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

            CREATE TABLE IF NOT EXISTS episode_state_index (
                    episode_id TEXT PRIMARY KEY,
                    retention_state TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS episode_counts (
                    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                    total_count INTEGER NOT NULL,
                    active_count INTEGER NOT NULL
                );
                INSERT OR IGNORE INTO episode_counts (singleton, total_count, active_count) VALUES (1, 0, 0);
                CREATE TABLE IF NOT EXISTS episode_presence_index (
                    edge_key TEXT PRIMARY KEY,
                    entity_id TEXT NOT NULL,
                    episode_id TEXT NOT NULL,
                    retention_state TEXT NOT NULL,
                    is_current INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS episode_presence_entity ON episode_presence_index(entity_id, episode_id);
                CREATE INDEX IF NOT EXISTS episode_presence_episode ON episode_presence_index(episode_id);
            ",
        )
        .map_err(sqlite_error)?;
    initialize_health_metadata(connection)
}

fn initialize_health_metadata(connection: &Connection) -> Result<(), RetrievalStatsStoreError> {
    if meta_value(connection, "health_state")?.is_none() {
        set_health(connection, RetrievalStatsHealth::default())?;
    }
    Ok(())
}

fn upsert_edge(
    connection: &Connection,
    edge: &RetrievalStatsEdge,
) -> Result<(), RetrievalStatsStoreError> {
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
            let merged_retention =
                more_restrictive_retention_key(&old_retention, edge.retention_state)?;
            let merged_is_current = old_is_current && edge.is_current;
            let new_active = merged_retention == RetentionState::Active;
            let active_delta = bool_delta(old_active, new_active);
            let current_delta = bool_delta(
                old_active && old_is_current,
                new_active && merged_is_current,
            );
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
                        retention_state_key(merged_retention),
                        bool_int(merged_is_current),
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
) -> Result<(), RetrievalStatsStoreError> {
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
        let old_active = retention_from_key(&old_retention)? == RetentionState::Active;
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
) -> Result<(), RetrievalStatsStoreError> {
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
) -> Result<(), RetrievalStatsStoreError> {
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

fn set_health(
    connection: &Connection,
    health: RetrievalStatsHealth,
) -> Result<(), RetrievalStatsStoreError> {
    let state = match health.state {
        RetrievalStatsHealthState::Healthy => "healthy",
        RetrievalStatsHealthState::Unhealthy => "unhealthy",
    };
    set_meta_value(connection, "health_state", Some(state))?;
    let serialized_cause = health
        .last_error_cause
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| RetrievalStatsStoreError::HealthSerialization {
            detail: error.to_string(),
        })?;
    set_meta_value(connection, "last_error_cause", serialized_cause.as_deref())
}

fn set_meta_value(
    connection: &Connection,
    key: &str,
    value: Option<&str>,
) -> Result<(), RetrievalStatsStoreError> {
    connection
        .execute(
            "INSERT INTO stats_meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(sqlite_error)?;
    Ok(())
}

fn meta_value(
    connection: &Connection,
    key: &str,
) -> Result<Option<String>, RetrievalStatsStoreError> {
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

fn more_restrictive_retention_key(
    existing: &str,
    incoming: RetentionState,
) -> Result<RetentionState, RetrievalStatsStoreError> {
    let existing = retention_from_key(existing)?;
    Ok(
        if incoming.restrictiveness_rank() > existing.restrictiveness_rank() {
            incoming
        } else {
            existing
        },
    )
}

fn retention_from_key(value: &str) -> Result<RetentionState, RetrievalStatsStoreError> {
    match value {
        "active" => Ok(RetentionState::Active),
        "suppressed" => Ok(RetentionState::Suppressed),
        _ => Err(RetrievalStatsStoreError::Sqlite {
            detail: format!("unknown retention key: {value}"),
        }),
    }
}

fn bool_int(value: bool) -> i64 {
    i64::from(value)
}

fn sqlite_error(error: rusqlite::Error) -> RetrievalStatsStoreError {
    RetrievalStatsStoreError::Sqlite {
        detail: error.to_string(),
    }
}

fn raw_counter_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(i64, i64, i64)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
}

fn counter_from_raw(
    (total_count, active_count, current_count): (i64, i64, i64),
) -> Result<RetrievalStatsCounter, RetrievalStatsStoreError> {
    Ok(RetrievalStatsCounter {
        total_count: non_negative_count(total_count)?,
        active_count: non_negative_count(active_count)?,
        current_count: non_negative_count(current_count)?,
    })
}

fn non_negative_count(value: i64) -> Result<u64, RetrievalStatsStoreError> {
    u64::try_from(value).map_err(|_| RetrievalStatsStoreError::NegativeCounter { value })
}

fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, RetrievalStatsStoreError> {
    mutex
        .lock()
        .map_err(|_| RetrievalStatsStoreError::LockPoisoned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use tempfile::tempdir;

    use crate::domain::{MemoryId, ObjectType, RelationType};

    #[tokio::test]
    async fn failed_schema_creation_rolls_back_all_tables() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("stats.sqlite");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("CREATE TABLE episode_counts (conflict INTEGER);")
            .unwrap();
        assert!(SqliteRetrievalStatsStore::open(&path).is_err());
        assert!(!connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'entity_edge_index')",
            [], |row| row.get::<_, bool>(0),
        ).unwrap());
        connection
            .execute_batch("DROP TABLE episode_counts;")
            .unwrap();
        let store = SqliteRetrievalStatsStore::open(&path).unwrap();
        assert_eq!(
            store.global_episode_counter().await.unwrap(),
            Some(RetrievalStatsCounter::default())
        );
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
            store
                .record_object_states(&[RetrievalStatsObjectState {
                    object_id: episode_id,
                    object_type: ObjectType::Episode,
                    retention_state: RetentionState::Active,
                    is_current: true,
                    observed_at: timestamp(),
                }])
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
            reopened.global_episode_counter().await.unwrap(),
            Some(counter)
        );
        assert_eq!(
            reopened.health().await.unwrap(),
            RetrievalStatsHealth::default()
        );
        drop(reopened);
        dir.close().unwrap();
    }

    #[test]
    fn retention_keys_parse_without_guessing_unknown_values() {
        assert_eq!(retention_from_key("active"), Ok(RetentionState::Active));
        assert_eq!(
            retention_from_key("suppressed"),
            Ok(RetentionState::Suppressed)
        );
        assert!(matches!(
            retention_from_key("unknown"),
            Err(RetrievalStatsStoreError::Sqlite { .. })
        ));
    }

    #[tokio::test]
    async fn sqlite_global_counter_rejects_negative_counts() {
        let dir = tempdir().unwrap();
        let store = SqliteRetrievalStatsStore::open(dir.path().join("stats.sqlite3")).unwrap();
        {
            let connection = lock(&store.connection).unwrap();
            connection
                .execute(
                    "INSERT INTO global_relation_counts
                     (relation_kind, object_type, total_count, active_count, current_count)
                     VALUES ('about', 'derived_memory', -1, 0, 0)",
                    [],
                )
                .unwrap();
        }

        let error = store
            .global_counter(RelationType::About, ObjectType::DerivedMemory)
            .await
            .unwrap_err();

        assert_eq!(
            error,
            RetrievalStatsStoreError::NegativeCounter { value: -1 }
        );
        drop(store);
        dir.close().unwrap();
    }

    #[tokio::test]
    async fn sqlite_store_preserves_unhealthy_marker_across_reopen() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("stats.sqlite3");
        let failure = RetrievalStatsHealthCause::EdgeWrite {
            error: RetrievalStatsStoreError::Sqlite {
                detail: "transient stats failure".to_owned(),
            },
        };

        {
            let store = SqliteRetrievalStatsStore::open(&path).unwrap();
            store.mark_unhealthy(failure.clone()).await.unwrap();
            store
                .record_edges(&[test_edge(
                    id("550e8400-e29b-41d4-a716-446655461051"),
                    id("550e8400-e29b-41d4-a716-446655461052"),
                    RetentionState::Active,
                    true,
                )])
                .await
                .unwrap();
        }

        let reopened = SqliteRetrievalStatsStore::open(&path).unwrap();
        let health = reopened.health().await.unwrap();
        assert_eq!(health.state, RetrievalStatsHealthState::Unhealthy);
        assert_eq!(health.last_error_cause, Some(failure));
        drop(reopened);
        dir.close().unwrap();
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
        DateTime::parse_from_rfc3339("2026-04-28T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }
}
