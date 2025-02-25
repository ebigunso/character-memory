use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::CustomError;
use crate::models::vector::VectorMetadata;
use crate::models::memory::dto::Memory;
use crate::models::memory::MemoryType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MemoryEntry {
    pub(crate) id: Uuid,
    pub(crate) memory_type: MemoryType,
    pub(crate) content: String,
    pub(crate) embedding: Vec<f32>,
    pub(crate) timestamp: Option<DateTime<Utc>>,
    pub(crate) location_text: Option<String>,
    pub(crate) participants: Option<Vec<String>>,
}

impl MemoryEntry {
    pub(crate) fn new(metadata: VectorMetadata, embedding: Vec<f32>) -> Result<Self, CustomError> {
        let entry = Self {
            id: metadata.id,
            memory_type: metadata.memory_type,
            content: metadata.content,
            embedding,
            timestamp: metadata.timestamp,
            location_text: metadata.location_text,
            participants: metadata.participants,
        };
        entry.validate()?;
        Ok(entry)
    }

    pub(crate) fn validate(&self) -> Result<(), CustomError> {
        match self.memory_type {
            MemoryType::Episodic => {
                if self.timestamp.is_none() {
                    return Err(CustomError::MissingEpisodicField("timestamp"));
                }
                if self.location_text.is_none() {
                    return Err(CustomError::MissingEpisodicField("location_text"));
                }
                if self.participants.is_none() {
                    return Err(CustomError::MissingEpisodicField("participants"));
                }
            }
            MemoryType::Semantic => {
                if self.timestamp.is_some() || self.location_text.is_some() || self.participants.is_some() {
                    return Err(CustomError::InvalidSemanticMemory);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn into_public(self) -> Memory {
        Memory {
            id: self.id,
            content: self.content,
            memory_type: self.memory_type,
            timestamp: self.timestamp,
            location_text: self.location_text,
            participants: self.participants,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_create_episodic_memory() {
        let id = Uuid::new_v4();
        let timestamp = Utc::now();
        let metadata = VectorMetadata::new_episodic(
            id,
            "Discussed plans".to_string(),
            timestamp,
            "Café Central".to_string(),
            vec!["Alice".to_string(), "Bob".to_string()],
        );

        let result = MemoryEntry::new(metadata.clone(), vec![0.1, 0.2]);
        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.id, id);
        assert_eq!(entry.content, metadata.content);
        assert_eq!(entry.timestamp, Some(timestamp));
        assert_eq!(entry.location_text, Some("Café Central".to_string()));
        assert_eq!(entry.participants, Some(vec!["Alice".to_string(), "Bob".to_string()]));
    }

    #[test]
    fn test_create_semantic_memory() {
        let id = Uuid::new_v4();
        let metadata = VectorMetadata::new_semantic(
            id,
            "Alice is a software engineer".to_string(),
        );

        let result = MemoryEntry::new(metadata.clone(), vec![0.1, 0.2]);
        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.id, id);
        assert_eq!(entry.content, metadata.content);
        assert!(entry.timestamp.is_none());
        assert!(entry.location_text.is_none());
        assert!(entry.participants.is_none());
    }

    #[test]
    fn test_create_invalid_semantic_memory() {
        let id = Uuid::new_v4();
        let timestamp = Utc::now();
        let metadata = VectorMetadata::new_semantic(
            id,
            "Alice is a software engineer".to_string(),
        );

        // Manually create an invalid semantic memory with timestamp
        let entry = MemoryEntry {
            id,
            memory_type: MemoryType::Semantic,
            content: metadata.content,
            embedding: vec![0.1, 0.2],
            timestamp: Some(timestamp),
            location_text: None,
            participants: None,
        };

        assert!(matches!(entry.validate(), Err(CustomError::InvalidSemanticMemory)));
    }
}
