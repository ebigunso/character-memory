// Module declarations
pub mod memory {
    pub mod dto {
        mod memory;
        mod memory_filters;
        mod memory_input;
        mod scored_memory;

        pub use memory::Memory;
        pub use memory_filters::MemoryFilters;
        pub use memory_input::MemoryInput;
        pub use scored_memory::ScoredMemory;
    }

    mod enums {
        pub(super) mod memory_type;
    }

    mod memory_entry;
    mod scored_memory_entry;
    pub use enums::memory_type::MemoryType;
    pub use memory_entry::MemoryEntry;
    pub use scored_memory_entry::ScoredMemoryEntry;
}

pub(crate) mod vector {
    mod vector_metadata;
    mod enums {
        pub(super) mod embedding_model;
    }

    pub(crate) use enums::embedding_model::EmbeddingModel;
    pub(crate) use vector_metadata::VectorMetadata;
}
