use std::str::FromStr;
use serde::{Serialize, Deserialize};
use crate::errors::CustomError;

/// Available embedding models and their corresponding vector sizes.
///
/// # Description
///
/// This enum represents the supported OpenAI embedding models,
/// each with its specific vector dimension size.
///
/// # See also
///
/// - [OpenAI Embeddings Guide](https://platform.openai.com/docs/guides/embeddings)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EmbeddingModel {
    /// OpenAI text-embedding-3-small model.
    ///
    /// # Description
    ///
    /// A smaller, more efficient model with 1536-dimensional embeddings
    TextEmbedding3Small,

    /// OpenAI text-embedding-3-large model.
    ///
    /// # Description
    ///
    /// A larger model with 3072-dimensional embeddings for higher accuracy
    TextEmbedding3Large,

    /// OpenAI text-embedding-ada-002 model.
    ///
    /// # Description
    ///
    /// Legacy model with 1536-dimensional embeddings
    TextEmbeddingAda002,
}

impl EmbeddingModel {
    /// Returns the vector size (dimensions) for this embedding model.
    ///
    /// # Returns
    ///
    /// The number of dimensions in the embedding vectors produced by this model.
    pub fn vector_size(&self) -> u64 {
        match self {
            Self::TextEmbedding3Small => 1536,
            Self::TextEmbedding3Large => 3072,
            Self::TextEmbeddingAda002 => 1536,
        }
    }
}

impl FromStr for EmbeddingModel {
    type Err = CustomError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "text-embedding-3-small" => Ok(Self::TextEmbedding3Small),
            "text-embedding-3-large" => Ok(Self::TextEmbedding3Large),
            "text-embedding-ada-002" => Ok(Self::TextEmbeddingAda002),
            _ => Err(CustomError::ConfigParseError(
                format!("Invalid embedding model: {}", s)
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_valid_models() {
        assert!(matches!(
            "text-embedding-3-small".parse::<EmbeddingModel>(),
            Ok(EmbeddingModel::TextEmbedding3Small)
        ));
        assert!(matches!(
            "text-embedding-3-large".parse::<EmbeddingModel>(),
            Ok(EmbeddingModel::TextEmbedding3Large)
        ));
        assert!(matches!(
            "text-embedding-ada-002".parse::<EmbeddingModel>(),
            Ok(EmbeddingModel::TextEmbeddingAda002)
        ));
    }

    #[test]
    fn test_from_str_invalid_model() {
        assert!(matches!(
            "invalid-model".parse::<EmbeddingModel>(),
            Err(CustomError::ConfigParseError(_))
        ));
    }

    #[test]
    fn test_vector_size() {
        assert_eq!(EmbeddingModel::TextEmbedding3Small.vector_size(), 1536);
        assert_eq!(EmbeddingModel::TextEmbedding3Large.vector_size(), 3072);
        assert_eq!(EmbeddingModel::TextEmbeddingAda002.vector_size(), 1536);
    }
}
