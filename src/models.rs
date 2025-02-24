// Module declarations
pub mod public {
    pub mod memory;
    pub mod memory_filters;
    pub mod memory_input;
    pub mod memory_type;

    pub use memory::Memory;
    pub use memory_filters::MemoryFilters;
    pub use memory_input::MemoryInput;
    pub use memory_type::MemoryType;
}

pub(crate) mod internal {
    pub(crate) mod memory_entry;
    pub(crate) mod vector_metadata;
    pub(crate) mod enums {
        pub(crate) mod embedding_model;
        pub(crate) use embedding_model::EmbeddingModel;
    }

    pub(crate) use memory_entry::MemoryEntry;
    pub(crate) use vector_metadata::VectorMetadata;
    pub(crate) use enums::EmbeddingModel;
}
