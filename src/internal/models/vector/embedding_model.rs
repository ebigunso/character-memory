use crate::errors::CustomError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
pub(crate) enum EmbeddingModel {
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
    pub(crate) fn vector_size(&self) -> u64 {
        match self {
            Self::TextEmbedding3Small => 1536,
            Self::TextEmbedding3Large => 3072,
            Self::TextEmbeddingAda002 => 1536,
        }
    }

    /// Returns the string representation of this embedding model.
    ///
    /// # Returns
    ///
    /// The canonical string identifier used by OpenAI's API for this model.
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::TextEmbedding3Small => "text-embedding-3-small",
            Self::TextEmbedding3Large => "text-embedding-3-large",
            Self::TextEmbeddingAda002 => "text-embedding-ada-002",
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
            _ => Err(CustomError::ConfigParseError(format!(
                "Invalid embedding model: {s}",
            ))),
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

    #[test]
    fn test_as_str() {
        assert_eq!(
            EmbeddingModel::TextEmbedding3Small.as_str(),
            "text-embedding-3-small"
        );
        assert_eq!(
            EmbeddingModel::TextEmbedding3Large.as_str(),
            "text-embedding-3-large"
        );
        assert_eq!(
            EmbeddingModel::TextEmbeddingAda002.as_str(),
            "text-embedding-ada-002"
        );
    }
}
