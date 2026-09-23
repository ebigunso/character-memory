mod in_memory;
mod sqlite;

pub(crate) use in_memory::InMemoryRetrievalStatsStore;
pub(crate) use sqlite::SqliteRetrievalStatsStore;
