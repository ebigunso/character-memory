use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::custom::CustomError;
use crate::models::public::memory_input::MemoryInput;
use crate::models::public::memory::Memory;
use crate::models::internal::memory_type::MemoryType;

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
    pub(crate) fn new(input: MemoryInput, embedding: Vec<f32>) -> Result<Self, CustomError> {
        let memory_type = match input.memory_type.to_lowercase().as_str() {
            "episodic" => MemoryType::Episodic,
            "semantic" => MemoryType::Semantic,
            _ => {
                return Err(CustomError::MemoryValidation(
                    "memory_type must be either 'episodic' or 'semantic'".to_string(),
                ))
            }
        };

        let entry = Self {
            id: Uuid::new_v4(),
            memory_type,
            content: input.content,
            embedding,
            timestamp: input.timestamp,
            location_text: input.location_text,
            participants: input.participants,
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
            memory_type: match self.memory_type {
                MemoryType::Episodic => "episodic".to_string(),
                MemoryType::Semantic => "semantic".to_string(),
            },
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
    fn test_create_valid_episodic_memory() {
        let input = MemoryInput {
            content: "Discussed plans".to_string(),
            memory_type: "episodic".to_string(),
            timestamp: Some(Utc::now()),
            location_text: Some("Café Central".to_string()),
            participants: Some(vec!["Alice".to_string(), "Bob".to_string()]),
        };

        let result = MemoryEntry::new(input, vec![0.1, 0.2]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_valid_semantic_memory() {
        let input = MemoryInput {
            content: "Alice is a software engineer".to_string(),
            memory_type: "semantic".to_string(),
            timestamp: None,
            location_text: None,
            participants: None,
        };

        let result = MemoryEntry::new(input, vec![0.1, 0.2]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_invalid_episodic_memory() {
        let input = MemoryInput {
            content: "Discussed plans".to_string(),
            memory_type: "episodic".to_string(),
            timestamp: None,
            location_text: Some("Café Central".to_string()),
            participants: Some(vec!["Alice".to_string()]),
        };

        let result = MemoryEntry::new(input, vec![0.1, 0.2]);
        assert!(matches!(result, Err(CustomError::MissingEpisodicField("timestamp"))));
    }

    #[test]
    fn test_create_invalid_semantic_memory() {
        let input = MemoryInput {
            content: "Alice is a software engineer".to_string(),
            memory_type: "semantic".to_string(),
            timestamp: Some(Utc::now()),
            location_text: None,
            participants: None,
        };

        let result = MemoryEntry::new(input, vec![0.1, 0.2]);
        assert!(matches!(result, Err(CustomError::InvalidSemanticMemory)));
    }

    #[test]
    fn test_invalid_memory_type() {
        let input = MemoryInput {
            content: "Test".to_string(),
            memory_type: "invalid".to_string(),
            timestamp: None,
            location_text: None,
            participants: None,
        };

        let result = MemoryEntry::new(input, vec![0.1, 0.2]);
        assert!(matches!(result, Err(CustomError::MemoryValidation(_))));
    }
}
