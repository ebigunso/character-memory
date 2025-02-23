use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::errors::CustomError;
use crate::models::public::{MemoryInput, MemoryType};

/// Represents the metadata structure of a vector memory entry in the vector database.
///
/// # Description
///
/// This struct encapsulates the metadata associated with memory vectors stored in the database, supporting both episodic (event-based) and semantic (general knowledge) memories.
/// It maintains a consistent structure while allowing episodic memories to include additional temporal and contextual information.
#[derive(Debug, Clone)]
pub(crate) struct VectorMetadata {
    /// Unique identifier linking to graph database
    pub(crate) id: Uuid,

    /// Type of memory (episodic or semantic)
    pub(crate) memory_type: MemoryType,

    /// Raw textual content used for vector generation
    pub(crate) content: String,

    /// Event occurrence time (RFC 3339) - Required only for episodic memories
    pub(crate) timestamp: Option<DateTime<Utc>>,

    /// Textual location description - Required only for episodic memories
    pub(crate) location_text: Option<String>,

    /// Array of involved entities - Required only for episodic memories
    pub(crate) participants: Option<Vec<String>>,
}

impl VectorMetadata {
    /// Creates a VectorMetadata instance from a MemoryInput.
    ///
    /// # Parameters
    ///
    /// - `input`: The MemoryInput to convert from
    ///
    /// # Returns
    ///
    /// A Result containing either:
    /// - The converted VectorMetadata instance
    /// - A CustomError if the conversion fails
    pub(crate) fn from_memory_input(input: MemoryInput) -> Result<Self, CustomError> {
        if input.memory_type == MemoryType::Semantic {
            Ok(Self::new_semantic(
                input.id.unwrap_or_else(Uuid::new_v4),
                input.content,
            ))
        } else {
            Ok(Self::new_episodic(
                input.id.unwrap_or_else(Uuid::new_v4),
                input.content,
                input.timestamp.ok_or_else(|| CustomError::MissingEpisodicField("timestamp"))?,
                input.location_text.ok_or_else(|| CustomError::MissingEpisodicField("location_text"))?,
                input.participants.ok_or_else(|| CustomError::MissingEpisodicField("participants"))?,
            ))
        }
    }

    /// Creates a new semantic memory metadata instance.
    ///
    /// # Parameters
    ///
    /// - `id`: Unique identifier for the memory
    /// - `content`: Raw textual content used for vector generation
    ///
    /// # Returns
    ///
    /// A new `VectorMetadata` instance configured for semantic memory
    pub(crate) fn new_semantic(id: Uuid, content: String) -> Self {
        Self {
            id,
            memory_type: MemoryType::Semantic,
            content,
            timestamp: None,
            location_text: None,
            participants: None,
        }
    }

    /// Creates a new episodic memory metadata instance.
    ///
    /// # Parameters
    ///
    /// - `id`: Unique identifier for the memory
    /// - `content`: Raw textual content used for vector generation
    /// - `timestamp`: When the event occurred
    /// - `location_text`: Description of where the event took place
    /// - `participants`: List of entities involved in the event
    ///
    /// # Returns
    ///
    /// A new `VectorMetadata` instance configured for episodic memory
    pub(crate) fn new_episodic(
        id: Uuid,
        content: String,
        timestamp: DateTime<Utc>,
        location_text: String,
        participants: Vec<String>,
    ) -> Self {
        Self {
            id,
            memory_type: MemoryType::Episodic,
            content,
            timestamp: Some(timestamp),
            location_text: Some(location_text),
            participants: Some(participants),
        }
    }

    /// Checks if this metadata represents an episodic memory.
    ///
    /// # Returns
    ///
    /// `true` if this is an episodic memory, `false` otherwise
    #[allow(dead_code)]
    pub(crate) fn is_episodic(&self) -> bool {
        matches!(self.memory_type, MemoryType::Episodic)
    }

    /// Checks if this metadata represents a semantic memory.
    ///
    /// # Returns
    ///
    /// `true` if this is a semantic memory, `false` otherwise
    #[allow(dead_code)]
    pub(crate) fn is_semantic(&self) -> bool {
        matches!(self.memory_type, MemoryType::Semantic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_from_memory_input_semantic() {
        let id = Uuid::new_v4();
        let content = "Alice is a software engineer".to_string();
        let input = MemoryInput {
            id: Some(id),
            content: content.clone(),
            memory_type: MemoryType::Semantic,
            timestamp: None,
            location_text: None,
            participants: None,
        };

        let result = VectorMetadata::from_memory_input(input);
        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.id, id);
        assert!(metadata.is_semantic());
        assert_eq!(metadata.content, content);
        assert!(metadata.timestamp.is_none());
        assert!(metadata.location_text.is_none());
        assert!(metadata.participants.is_none());
    }

    #[test]
    fn test_from_memory_input_episodic() {
        let id = Uuid::new_v4();
        let content = "Discussed plans".to_string();
        let timestamp = Utc::now();
        let location = "Café Central".to_string();
        let participants = vec!["Alice".to_string(), "Bob".to_string()];

        let input = MemoryInput {
            id: Some(id),
            content: content.clone(),
            memory_type: MemoryType::Episodic,
            timestamp: Some(timestamp),
            location_text: Some(location.clone()),
            participants: Some(participants.clone()),
        };

        let result = VectorMetadata::from_memory_input(input);
        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.id, id);
        assert!(metadata.is_episodic());
        assert_eq!(metadata.content, content);
        assert_eq!(metadata.timestamp, Some(timestamp));
        assert_eq!(metadata.location_text, Some(location));
        assert_eq!(metadata.participants, Some(participants));
    }

    #[test]
    fn test_from_memory_input_generates_id() {
        let input = MemoryInput {
            id: None,
            content: "Test content".to_string(),
            memory_type: MemoryType::Semantic,
            timestamp: None,
            location_text: None,
            participants: None,
        };

        let result = VectorMetadata::from_memory_input(input);
        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert!(!metadata.id.to_string().is_empty());
    }

    #[test]
    fn test_from_memory_input_missing_episodic_fields() {
        let input = MemoryInput {
            id: None,
            content: "Test content".to_string(),
            memory_type: MemoryType::Episodic,
            timestamp: None,
            location_text: Some("Location".to_string()),
            participants: Some(vec!["Alice".to_string()]),
        };

        let result = VectorMetadata::from_memory_input(input);
        assert!(result.is_err());
        assert!(matches!(result, Err(CustomError::MissingEpisodicField("timestamp"))));
    }


    #[test]
    fn test_new_semantic() {
        let id = Uuid::new_v4();
        let content = "Alice is a software engineer living in New York.".to_string();

        let metadata = VectorMetadata::new_semantic(id, content.clone());

        assert_eq!(metadata.id, id);
        assert!(metadata.is_semantic());
        assert_eq!(metadata.content, content);
        assert!(metadata.timestamp.is_none());
        assert!(metadata.location_text.is_none());
        assert!(metadata.participants.is_none());
    }

    #[test]
    fn test_new_episodic() {
        let id = Uuid::new_v4();
        let content = "Discussed plans for the weekend at Café Central.".to_string();
        let timestamp = Utc.with_ymd_and_hms(2025, 2, 2, 14, 0, 0).unwrap();
        let location = "Café Central".to_string();
        let participants = vec!["Alice".to_string(), "Bob".to_string()];

        let metadata = VectorMetadata::new_episodic(
            id,
            content.clone(),
            timestamp,
            location.clone(),
            participants.clone(),
        );

        assert_eq!(metadata.id, id);
        assert!(metadata.is_episodic());
        assert_eq!(metadata.content, content);
        assert_eq!(metadata.timestamp, Some(timestamp));
        assert_eq!(metadata.location_text, Some(location));
        assert_eq!(metadata.participants, Some(participants));
    }
}
