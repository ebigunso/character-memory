mod in_memory;
#[cfg(test)]
mod noop;
mod sqlite;

pub(crate) use in_memory::InMemoryRetrievalStatsStore;
#[cfg(test)]
pub(crate) use noop::noop_retrieval_stats_store;
pub(crate) use sqlite::SqliteRetrievalStatsStore;
