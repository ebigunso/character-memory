mod config;
mod errors;
// NOTE: internal implementation code lives under `crate::internal`.

pub mod api;
mod internal;

use uuid::Uuid;

use crate::config::settings::{EmbeddingProviderSettings, VectorMemoryRepositorySettings};
use crate::internal::infrastructures::external_services::{
    OpenAIEmbeddingProvider, QdrantVectorMemoryRepository,
};
use crate::internal::models::vector::VectorMetadata;
use crate::internal::repositories::{
    GraphAuthorityStore, LinkPipeline, MemoryEmbedder, MemoryRepository, RememberPipeline,
    RememberPipelineDraft, VectorCandidateStore,
};

// Re-export types for public use
pub use crate::api::embedding::EmbeddingProvider;
pub use crate::api::types::{
    graph_uri, DerivedMemory, DerivedMemoryDraft, DerivedType, DomainValidationError,
    DraftDefaults, Entity, EntityDraft, EntityType, Episode, EpisodeDraft, Memory, MemoryFilters,
    MemoryId, MemoryInput, MemoryLink, MemoryLinkDraft, MemoryObject, MemoryObjectDraft,
    MemoryThread, MemoryThreadDraft, MemoryType, Modality, ObjectType, Observation,
    ObservationDraft, RelationType, RememberDraft, RememberOutcome, RetentionState, ScoredMemory,
    Stability, ThreadStatus, VectorIndexingFailure, CURRENT_SCHEMA_VERSION, DEFAULT_SCHEMA_VERSION,
    SCHEMA_VERSION_V0_1,
};
pub use crate::config::settings::Settings;
pub use crate::errors::CustomError;

// Re-export for integration tests
pub mod test_utils {
    use crate::config::settings::Settings;
    use crate::errors::CustomError;

    /// Loads settings from environment variables for integration tests.
    ///
    /// # Important
    ///
    /// This function is intended ONLY for use in integration tests and should not be used in production code.
    /// A `.env` file in the project root directory will be loaded if present,
    /// otherwise existing environment variables are used.
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A new `Settings` instance with configuration loaded from environment
    /// - `Err`: A `CustomError` if loading fails
    pub fn load_test_settings() -> Result<Settings, CustomError> {
        Settings::load()
    }
}

/// CharacterMemory provides a high-level API for memory operations.
///
/// # Description
///
/// This struct serves as the main entry point for memory operations,
/// providing a high-level interface for storing, retrieving, and
/// searching memory entries.
pub struct CharacterMemory {
    legacy_memory_repo: Option<MemoryRepository>,
    v0_1_parts: Option<V0_1Composition>,
}

struct V0_1Composition {
    graph_store: Box<dyn GraphAuthorityStore>,
    vector_store: Box<dyn VectorCandidateStore>,
    embedder: Box<dyn MemoryEmbedder>,
}

impl CharacterMemory {
    /// Builds the v0.1 write facade from provider-neutral graph, vector, and embedder parts.
    ///
    /// This is the durable composition boundary for deterministic tests and application-owned
    /// backend wiring. It remains crate-visible until the public graph/vector/embedder trait
    /// surface is selected.
    #[allow(dead_code)]
    pub(crate) fn from_parts(
        graph_store: Box<dyn GraphAuthorityStore>,
        vector_store: Box<dyn VectorCandidateStore>,
        embedder: Box<dyn MemoryEmbedder>,
    ) -> Self {
        Self {
            legacy_memory_repo: None,
            v0_1_parts: Some(V0_1Composition {
                graph_store,
                vector_store,
                embedder,
            }),
        }
    }

    /// Constructs a new CharacterMemory instance using a caller-provided embedding provider.
    ///
    /// # Description
    ///
    /// This constructor allows callers to inject custom embedding generation, while keeping
    /// vector storage on the default Qdrant backend.
    ///
    /// # Parameters
    ///
    /// - `settings`: Global configuration used to derive the Qdrant connection and embedding
    ///   model settings required to initialize the underlying vector memory repository.
    /// - `collection_name`: The name of the Qdrant collection where memory vectors will be
    ///   stored and queried.
    /// - `embed_provider`: A boxed implementation of [`EmbeddingProvider`] that is responsible
    ///   for generating embeddings from input data.
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok(Self)`: A new [`CharacterMemory`] instance backed by the provided embedding provider
    ///   and a Qdrant-based vector memory repository.
    /// - `Err(CustomError)`: Returned if any error occurs while creating the vector memory
    ///   repository or when resolving configuration from `settings`.
    pub async fn new_with_embedding_provider(
        settings: Settings,
        collection_name: String,
        embed_provider: Box<dyn EmbeddingProvider>,
    ) -> Result<Self, CustomError> {
        let vector_memory_settings = VectorMemoryRepositorySettings::new(
            settings.get_qdrant_connection().to_string(),
            collection_name,
            settings.get_embedding_model()?,
        );
        let vector_repo = Box::new(QdrantVectorMemoryRepository::new(vector_memory_settings)?);
        let memory_repo = MemoryRepository::new(embed_provider, vector_repo);
        Ok(Self {
            legacy_memory_repo: Some(memory_repo),
            v0_1_parts: None,
        })
    }

    /// Constructs a new CharacterMemory instance.
    ///
    /// # Parameters
    ///
    /// - `settings`: Configuration settings for the memory system
    /// - `collection_name`: Name of the vector collection to use
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A new `CharacterMemory` instance
    /// - `Err`: A `CustomError` if initialization fails
    pub async fn new(settings: Settings, collection_name: String) -> Result<Self, CustomError> {
        // Configure and create the embedding provider
        let embedding_settings = EmbeddingProviderSettings::new(
            settings.get_openai_api_key().to_string(),
            settings.get_embedding_model()?,
        );
        let embed_provider = Box::new(OpenAIEmbeddingProvider::new(embedding_settings)?);

        // Configure and create the vector memory repository
        let vector_memory_settings = VectorMemoryRepositorySettings::new(
            settings.get_qdrant_connection().to_string(),
            collection_name.clone(),
            settings.get_embedding_model()?,
        );
        let vector_repo = Box::new(QdrantVectorMemoryRepository::new(vector_memory_settings)?);
        // Assemble the high-level MemoryRepository.
        let memory_repo = MemoryRepository::new(embed_provider, vector_repo);

        Ok(Self {
            legacy_memory_repo: Some(memory_repo),
            v0_1_parts: None,
        })
    }

    /// Initializes the storage systems.
    ///
    /// # Description
    ///
    /// Ensures all required storage systems are properly initialized before any operations are performed.
    /// This should be called during application startup.
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: Empty unit type if initialization succeeds
    /// - `Err`: A `CustomError` if initialization fails
    pub async fn init_storage(&self) -> Result<(), CustomError> {
        self.legacy_repo()?.init_storage().await
    }

    /// Persists a v0.1 remember draft through the graph-authoritative write pipeline.
    pub async fn remember(&self, draft: RememberDraft) -> Result<RememberOutcome, CustomError> {
        let parts = self.v0_1_parts()?;
        let pipeline = RememberPipeline::new(
            parts.graph_store.as_ref(),
            parts.vector_store.as_ref(),
            parts.embedder.as_ref(),
        );
        let outcome = pipeline
            .remember(RememberPipelineDraft::new(
                draft.object_drafts,
                draft.link_drafts,
            ))
            .await?;
        Ok(outcome.into())
    }

    /// Persists a canonical typed relationship through the graph-authoritative link pipeline.
    pub async fn link(&self, draft: MemoryLinkDraft) -> Result<MemoryLink, CustomError> {
        let parts = self.v0_1_parts()?;
        LinkPipeline::new(parts.graph_store.as_ref())
            .link(draft)
            .await
    }

    /// Legacy flat vector-only create path retained for existing integration coverage until the
    /// default production constructor is rewired to v0.1 graph/vector composition.
    #[deprecated(
        note = "use CharacterMemory::remember; remove this once the default constructor is rewired to the v0.1 facade"
    )]
    pub async fn create_memory(&self, input: MemoryInput) -> Result<Memory, CustomError> {
        let legacy_repo = self.legacy_repo()?;
        let metadata = VectorMetadata::from_memory_input(input)?;
        let mem_entry = legacy_repo.create_memory(metadata).await?;
        Ok(mem_entry.into_public())
    }

    /// Creates multiple memory entries in a batch.
    ///
    /// Legacy flat vector-only batch create path retained for existing integration coverage until
    /// the default production constructor is rewired to v0.1 graph/vector composition.
    #[deprecated(
        note = "use CharacterMemory::remember; remove this once the default constructor is rewired to the v0.1 facade"
    )]
    pub async fn bulk_create_memories(
        &self,
        inputs: &[MemoryInput],
    ) -> Result<Vec<Memory>, CustomError> {
        let legacy_repo = self.legacy_repo()?;
        let metadata_list: Result<Vec<_>, _> = inputs
            .iter()
            .map(|input| VectorMetadata::from_memory_input(input.clone()))
            .collect();
        let metadata_list = metadata_list?;

        let entries = legacy_repo.bulk_create_memories(&metadata_list).await?;
        Ok(entries
            .into_iter()
            .map(|entry| entry.into_public())
            .collect())
    }

    /// Retrieves a memory entry by its unique identifier.
    ///
    /// # Parameters
    ///
    /// - `id`: The UUID of the memory entry to retrieve
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A `Memory` containing the requested entry
    /// - `Err`: A `CustomError` if the operation fails
    pub async fn get_memory_by_id(&self, id: Uuid) -> Result<Memory, CustomError> {
        let mem_entry = self.legacy_repo()?.get_memory_by_id(id).await?;
        Ok(mem_entry.into_public())
    }

    /// Retrieves multiple memory entries by their unique identifiers.
    ///
    /// # Parameters
    ///
    /// - `ids`: A slice of UUIDs of the memory entries to retrieve
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A vector of `Memory` containing the requested entries
    /// - `Err`: A `CustomError` if the operation fails
    pub async fn get_memories_by_ids(&self, ids: &[Uuid]) -> Result<Vec<Memory>, CustomError> {
        let mem_entries = self.legacy_repo()?.get_memories_by_ids(ids).await?;
        Ok(mem_entries
            .into_iter()
            .map(|entry| entry.into_public())
            .collect())
    }

    /// Searches for memory entries that are semantically similar to the query.
    ///
    /// # Parameters
    ///
    /// - `query`: The search query string to find similar memories
    /// - `top_k`: The maximum number of results to return
    /// - `filters`: Optional filters to apply to the search results
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A vector of `Memory` containing the search results
    /// - `Err`: A `CustomError` if the operation fails
    #[deprecated(
        note = "this flat vector retrieval path will be replaced by a dedicated v0.1 retrieve facade"
    )]
    pub async fn search_memories(
        &self,
        query: &str,
        top_k: usize,
        filters: Option<MemoryFilters>,
    ) -> Result<Vec<ScoredMemory>, CustomError> {
        let entries = self
            .legacy_repo()?
            .search_memories(query, top_k, filters)
            .await?;

        Ok(entries
            .into_iter()
            .map(|entry| entry.into_public())
            .collect())
    }

    /// Updates an existing memory entry.
    ///
    /// # Parameters
    ///
    /// - `input`: A `MemoryInput` containing the updated data and ID of the entry to update
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: A `Memory` containing the updated entry
    /// - `Err`: A `CustomError` if:
    ///     - The input does not contain an ID
    ///     - The update operation fails
    #[deprecated(
        note = "this flat update path will be replaced deliberately after the v0.1 write facade lands"
    )]
    pub async fn update_memory(&self, input: MemoryInput) -> Result<Memory, CustomError> {
        let legacy_repo = self.legacy_repo()?;
        let metadata = VectorMetadata::from_memory_input(input)?;
        let mem_entry = legacy_repo.update_memory(metadata).await?;
        Ok(mem_entry.into_public())
    }

    /// Deletes a memory entry by its unique identifier.
    ///
    /// # Parameters
    ///
    /// - `id`: The UUID of the memory entry to delete
    ///
    /// # Returns
    ///
    /// A `Result` which is:
    ///
    /// - `Ok`: Empty unit type if deletion succeeds
    /// - `Err`: A `CustomError` if the operation fails
    #[deprecated(
        note = "this flat delete path will be replaced deliberately after the v0.1 write facade lands"
    )]
    pub async fn delete_memory(&self, id: Uuid) -> Result<(), CustomError> {
        self.legacy_repo()?.delete_memory(id).await
    }

    fn legacy_repo(&self) -> Result<&MemoryRepository, CustomError> {
        self.legacy_memory_repo.as_ref().ok_or_else(|| {
            CustomError::DatabaseError(
                "legacy flat memory API is not available on v0.1 injected construction".to_owned(),
            )
        })
    }

    fn v0_1_parts(&self) -> Result<&V0_1Composition, CustomError> {
        self.v0_1_parts.as_ref().ok_or_else(|| {
            CustomError::DatabaseError(
                "v0.1 remember/link API requires injected graph, vector, and embedder parts"
                    .to_owned(),
            )
        })
    }
}

impl From<crate::internal::repositories::RememberPipelineOutcome> for RememberOutcome {
    fn from(value: crate::internal::repositories::RememberPipelineOutcome) -> Self {
        Self {
            persisted_object_ids: value.persisted_object_ids,
            persisted_link_ids: value.persisted_link_ids,
            vector_indexed_object_ids: value.vector_indexed_object_ids,
            vector_indexing_failure: value.vector_indexing_failure.map(Into::into),
        }
    }
}

impl From<crate::internal::repositories::InternalVectorIndexingFailure> for VectorIndexingFailure {
    fn from(value: crate::internal::repositories::InternalVectorIndexingFailure) -> Self {
        Self {
            unindexed_object_ids: value.unindexed_object_ids,
            error_message: value.error_message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    use crate::api::types::{EntityDraft, EntityType, ObjectType, RelationType};
    use crate::internal::repositories::test_support::{
        DeterministicMemoryEmbedder, FakeGraphAuthorityStore, FakeVectorCandidateStore,
    };

    #[tokio::test]
    async fn injected_facade_remembers_backend_free_drafts() {
        let memory = injected_memory();
        let entity_id = id("550e8400-e29b-41d4-a716-446655445001");
        let mut entity = EntityDraft::new(EntityType::User, "Kohta");
        entity.id = Some(entity_id);

        let outcome = memory
            .remember(RememberDraft::new([MemoryObjectDraft::Entity(entity)]))
            .await
            .expect("remember facade should persist through injected parts");

        assert_eq!(outcome.persisted_object_ids, vec![entity_id]);
        assert_eq!(outcome.persisted_link_ids, Vec::<MemoryId>::new());
        assert_eq!(outcome.vector_indexed_object_ids, vec![entity_id]);
        assert_eq!(outcome.vector_indexing_failure, None);
    }

    #[tokio::test]
    async fn injected_facade_links_canonical_relationships() {
        let memory = injected_memory();
        let from_id = id("550e8400-e29b-41d4-a716-446655445010");
        let to_id = id("550e8400-e29b-41d4-a716-446655445011");
        let mut draft = MemoryLinkDraft::new(
            ObjectType::Entity,
            from_id,
            RelationType::Mentions,
            ObjectType::Episode,
            to_id,
        );
        draft.id = Some(id("550e8400-e29b-41d4-a716-446655445012"));

        let link = memory
            .link(draft)
            .await
            .expect("link facade should persist through injected graph store");

        assert_eq!(link.from_id, from_id);
        assert_eq!(link.to_id, to_id);
        assert_eq!(link.relation, RelationType::Mentions);
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn injected_facade_does_not_expose_legacy_flat_create_path() {
        let memory = injected_memory();
        let input = MemoryInput {
            id: None,
            content: "legacy flat input".to_owned(),
            memory_type: MemoryType::Episodic,
            timestamp: None,
            location_text: None,
            participants: None,
        };

        let error = memory.create_memory(input).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("legacy flat memory API is not available"));
    }

    fn injected_memory() -> CharacterMemory {
        CharacterMemory::from_parts(
            Box::new(FakeGraphAuthorityStore::new()),
            Box::new(FakeVectorCandidateStore::new()),
            Box::new(DeterministicMemoryEmbedder::new(8)),
        )
    }

    fn id(value: &str) -> MemoryId {
        Uuid::parse_str(value).unwrap()
    }
}
