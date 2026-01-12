// Module declarations
pub mod memory {
    mod memory_entry;
    mod scored_memory_entry;
    pub(crate) use memory_entry::MemoryEntry;
    pub(crate) use scored_memory_entry::ScoredMemoryEntry;
}

pub(crate) mod vector {
    mod vector_metadata;
    mod enums {
        pub(super) mod embedding_model;
    }

    pub(crate) use enums::embedding_model::EmbeddingModel;
    pub(crate) use vector_metadata::VectorMetadata;
}
