// Embedding-surface builders for graph objects that participate in vector
// candidate recall.
use crate::domain::{
    graph_uri, DerivedMemory, Entity, Episode, MemoryObject, MemoryThread, ObjectType, Observation,
};

use crate::models::vector::{
    VectorPayloadHints, VectorRecord, VectorRelationshipHints, VectorSurface,
};

pub(crate) fn episode_vector_record(episode: &Episode) -> VectorRecord {
    VectorRecord::new(
        episode.id,
        ObjectType::Episode,
        graph_uri(ObjectType::Episode, episode.id),
        VectorSurface::Summary,
        prefixed_text("Episode summary", &episode.summary),
        clean_text(&episode.summary),
        episode.schema_version.clone(),
        Some(episode.retention_state),
        None,
        VectorRelationshipHints {
            participant_entity_ids: episode.participant_entity_ids.clone(),
            ..VectorRelationshipHints::default()
        },
        episode.raw_ref.clone(),
    )
    .with_payload_hints(VectorPayloadHints {
        modality: Some(episode.modality),
        source_conversation_id: episode.source_conversation_id.clone(),
        created_at: Some(episode.created_at),
        started_at: episode.started_at,
        ended_at: episode.ended_at,
        salience_score: Some(episode.salience_score),
        ..VectorPayloadHints::default()
    })
}

pub(crate) fn observation_vector_record(observation: &Observation) -> VectorRecord {
    VectorRecord::new(
        observation.id,
        ObjectType::Observation,
        graph_uri(ObjectType::Observation, observation.id),
        VectorSurface::Text,
        prefixed_text("Observation excerpt", &observation.text),
        clean_text(&observation.text),
        observation.schema_version.clone(),
        Some(observation.retention_state),
        None,
        VectorRelationshipHints {
            episode_ids: vec![observation.episode_id],
            speaker_entity_id: observation.speaker_entity_id,
            ..VectorRelationshipHints::default()
        },
        observation.raw_ref.clone(),
    )
    .with_payload_hints(VectorPayloadHints {
        modality: Some(observation.modality),
        created_at: Some(observation.created_at),
        observed_at: observation.observed_at,
        salience_score: Some(observation.salience_score),
        ..VectorPayloadHints::default()
    })
}

pub(crate) fn derived_memory_vector_record(memory: &DerivedMemory) -> VectorRecord {
    VectorRecord::new(
        memory.id,
        ObjectType::DerivedMemory,
        graph_uri(ObjectType::DerivedMemory, memory.id),
        VectorSurface::DerivedText,
        prefixed_text(derived_label(memory), &memory.text),
        clean_text(&memory.text),
        memory.schema_version.clone(),
        Some(memory.retention_state),
        Some(memory.is_current),
        VectorRelationshipHints {
            episode_ids: memory.derived_from_episode_ids.clone(),
            observation_ids: memory.derived_from_observation_ids.clone(),
            thread_ids: memory.thread_ids.clone(),
            entity_ids: memory.entity_ids.clone(),
            supersedes: memory.supersedes.clone(),
            ..VectorRelationshipHints::default()
        },
        None,
    )
    .with_payload_hints(VectorPayloadHints {
        derived_type: Some(memory.derived_type),
        created_at: Some(memory.created_at),
        updated_at: Some(memory.updated_at),
        salience_score: Some(memory.salience_score),
        confidence: Some(memory.confidence),
        stability: Some(memory.stability),
        is_superseded: Some(!memory.is_current),
        ..VectorPayloadHints::default()
    })
}

pub(crate) fn memory_thread_vector_record(thread: &MemoryThread) -> VectorRecord {
    let content_text = join_clean([thread.title.as_str(), thread.summary.as_str()]);

    VectorRecord::new(
        thread.id,
        ObjectType::MemoryThread,
        graph_uri(ObjectType::MemoryThread, thread.id),
        VectorSurface::Summary,
        prefixed_text("Thread summary", &content_text),
        content_text,
        thread.schema_version.clone(),
        None,
        None,
        VectorRelationshipHints::default(),
        None,
    )
    .with_payload_hints(VectorPayloadHints {
        thread_status: Some(thread.status),
        canonical_key: thread.canonical_key.clone(),
        created_at: Some(thread.created_at),
        updated_at: Some(thread.updated_at),
        last_touched_at: Some(thread.last_touched_at),
        salience_score: Some(thread.salience_score),
        ..VectorPayloadHints::default()
    })
}

pub(crate) fn entity_vector_record(entity: &Entity) -> VectorRecord {
    let alias_text = if entity.aliases.is_empty() {
        String::new()
    } else {
        format!("Aliases: {}", entity.aliases.join(", "))
    };
    let summary = entity.summary.as_deref().unwrap_or_default();
    let content_text = join_clean([entity.name.as_str(), alias_text.as_str(), summary]);

    VectorRecord::new(
        entity.id,
        ObjectType::Entity,
        graph_uri(ObjectType::Entity, entity.id),
        VectorSurface::Name,
        prefixed_text("Entity", &content_text),
        content_text,
        entity.schema_version.clone(),
        None,
        None,
        VectorRelationshipHints::default(),
        None,
    )
    .with_payload_hints(VectorPayloadHints {
        entity_type: Some(entity.entity_type),
        canonical_key: entity.canonical_key.clone(),
        created_at: Some(entity.created_at),
        updated_at: Some(entity.updated_at),
        ..VectorPayloadHints::default()
    })
}

pub(crate) fn memory_object_vector_record(object: &MemoryObject) -> Option<VectorRecord> {
    match object {
        MemoryObject::Episode(object) => Some(episode_vector_record(object)),
        MemoryObject::Observation(object) => Some(observation_vector_record(object)),
        MemoryObject::Entity(object) => Some(entity_vector_record(object)),
        MemoryObject::MemoryThread(object) => Some(memory_thread_vector_record(object)),
        MemoryObject::DerivedMemory(object) => Some(derived_memory_vector_record(object)),
        MemoryObject::MemoryLink(_) => None,
    }
}

fn derived_label(memory: &DerivedMemory) -> &'static str {
    match memory.derived_type {
        crate::domain::DerivedType::Reflection => "Reflection",
        crate::domain::DerivedType::UserPreference => "User preference",
        crate::domain::DerivedType::AssistantPreference => "Assistant preference",
        crate::domain::DerivedType::Commitment => "Commitment",
        crate::domain::DerivedType::OpenLoop => "Open loop",
        crate::domain::DerivedType::CharacterSignal => "Character signal",
        crate::domain::DerivedType::RelationshipNote => "Relationship note",
        crate::domain::DerivedType::ProjectNote => "Project note",
        crate::domain::DerivedType::Claim => "Claim",
        crate::domain::DerivedType::Correction => "Correction",
    }
}

fn prefixed_text(label: &str, text: &str) -> String {
    let text = clean_text(text);
    if text.is_empty() {
        label.to_owned()
    } else {
        format!("{label}: {text}")
    }
}

fn join_clean<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    parts
        .into_iter()
        .map(clean_text)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn clean_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        DerivedType, EntityType, MemoryLink, Modality, RelationType, RetentionState, Stability,
        ThreadStatus, DEFAULT_SCHEMA_VERSION,
    };
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    #[test]
    fn episode_builder_uses_summary_surface_and_preserves_filter_hints() {
        let episode = episode_fixture();
        let record = episode_vector_record(&episode);

        assert_eq!(record.object_id, episode.id);
        assert_eq!(record.object_type, ObjectType::Episode);
        assert_eq!(record.graph_uri, graph_uri(ObjectType::Episode, episode.id));
        assert_eq!(record.surface, VectorSurface::Summary);
        assert_eq!(record.embedding_text, "Episode summary: Short summary.");
        assert_eq!(record.content_text, "Short summary.");
        assert_eq!(record.schema_version, DEFAULT_SCHEMA_VERSION);
        assert_eq!(record.retention_state, Some(RetentionState::Active));
        assert_eq!(
            record.relationship_hints.participant_entity_ids,
            vec![id(1)]
        );
        assert_eq!(record.raw_ref.as_deref(), Some("raw://episode"));
        assert_embedding_text_excludes_metadata(&record);
    }

    #[test]
    fn observation_builder_uses_excerpt_without_raw_reference_text() {
        let observation = observation_fixture();
        let record = observation_vector_record(&observation);

        assert_eq!(record.surface, VectorSurface::Text);
        assert_eq!(
            record.embedding_text,
            "Observation excerpt: Important excerpt."
        );
        assert_eq!(record.content_text, "Important excerpt.");
        assert_eq!(record.relationship_hints.episode_ids, vec![id(10)]);
        assert_eq!(record.relationship_hints.speaker_entity_id, Some(id(1)));
        assert_eq!(record.raw_ref.as_deref(), Some("raw://observation"));
        assert_embedding_text_excludes_metadata(&record);
    }

    #[test]
    fn derived_memory_builder_keeps_currentness_and_relationship_hints_out_of_embedding_text() {
        let derived = derived_memory_fixture();
        let record = derived_memory_vector_record(&derived);

        assert_eq!(record.surface, VectorSurface::DerivedText);
        assert_eq!(record.embedding_text, "Reflection: Derived insight.");
        assert_eq!(record.is_current, Some(false));
        assert_eq!(record.relationship_hints.episode_ids, vec![id(10)]);
        assert_eq!(record.relationship_hints.observation_ids, vec![id(20)]);
        assert_eq!(record.relationship_hints.thread_ids, vec![id(30)]);
        assert_eq!(record.relationship_hints.entity_ids, vec![id(1)]);
        assert_eq!(record.relationship_hints.supersedes, vec![id(99)]);
        assert_embedding_text_excludes_metadata(&record);
    }

    #[test]
    fn thread_and_entity_builders_use_names_summaries_and_exclude_state_metadata() {
        let thread = thread_fixture();
        let entity = entity_fixture();

        let thread_record = memory_thread_vector_record(&thread);
        let entity_record = entity_vector_record(&entity);

        assert_eq!(thread_record.surface, VectorSurface::Summary);
        assert_eq!(
            thread_record.embedding_text,
            "Thread summary: Useful thread Thread summary."
        );
        assert_embedding_text_excludes_metadata(&thread_record);

        assert_eq!(entity_record.surface, VectorSurface::Name);
        assert_eq!(
            entity_record.embedding_text,
            "Entity: Kohta Aliases: K. User summary."
        );
        assert_embedding_text_excludes_metadata(&entity_record);
    }

    #[test]
    fn memory_object_builder_covers_vector_indexed_domain_objects_and_skips_links() {
        let objects = [
            MemoryObject::Episode(episode_fixture()),
            MemoryObject::Observation(observation_fixture()),
            MemoryObject::DerivedMemory(derived_memory_fixture()),
            MemoryObject::MemoryThread(thread_fixture()),
            MemoryObject::Entity(entity_fixture()),
            MemoryObject::MemoryLink(link_fixture()),
        ];

        let records: Vec<_> = objects
            .iter()
            .filter_map(memory_object_vector_record)
            .collect();

        assert_eq!(records[0].object_type, ObjectType::Episode);
        assert_eq!(records[1].object_type, ObjectType::Observation);
        assert_eq!(records[2].object_type, ObjectType::DerivedMemory);
        assert_eq!(records[3].object_type, ObjectType::MemoryThread);
        assert_eq!(records[4].object_type, ObjectType::Entity);
        assert_eq!(records.len(), 5);
        assert_eq!(
            memory_object_vector_record(&MemoryObject::MemoryLink(link_fixture())),
            None
        );
    }

    fn assert_embedding_text_excludes_metadata(record: &VectorRecord) {
        assert!(!record
            .embedding_text
            .contains(&record.object_id.to_string()));
        assert!(!record.embedding_text.contains(&record.graph_uri));
        assert!(!record.embedding_text.contains(&record.schema_version));
        assert!(!record.embedding_text.contains("raw://"));
        assert!(!record.embedding_text.contains("Retention"));
        assert!(!record.embedding_text.contains("Active"));
        assert!(!record.embedding_text.contains("false"));
        assert!(!record.embedding_text.contains("0.42"));
    }

    fn episode_fixture() -> Episode {
        Episode {
            id: id(10),
            object_type: ObjectType::Episode,
            modality: Modality::Chat,
            source_conversation_id: Some("conversation-1".to_owned()),
            started_at: Some(timestamp()),
            ended_at: Some(timestamp()),
            participant_entity_ids: vec![id(1)],
            summary: " Short   summary. ".to_owned(),
            raw_ref: Some("raw://episode".to_owned()),
            salience_score: 0.42,
            retention_state: RetentionState::Active,
            created_at: timestamp(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        }
    }

    fn observation_fixture() -> Observation {
        Observation {
            id: id(20),
            object_type: ObjectType::Observation,
            episode_id: id(10),
            speaker_entity_id: Some(id(1)),
            observed_at: Some(timestamp()),
            modality: Modality::Chat,
            text: "Important   excerpt.".to_owned(),
            raw_ref: Some("raw://observation".to_owned()),
            salience_score: 0.42,
            retention_state: RetentionState::Active,
            created_at: timestamp(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        }
    }

    fn derived_memory_fixture() -> DerivedMemory {
        DerivedMemory {
            id: id(40),
            object_type: ObjectType::DerivedMemory,
            derived_type: DerivedType::Reflection,
            text: "Derived   insight.".to_owned(),
            derived_from_episode_ids: vec![id(10)],
            derived_from_observation_ids: vec![id(20)],
            thread_ids: vec![id(30)],
            entity_ids: vec![id(1)],
            confidence: 0.42,
            salience_score: 0.42,
            stability: Stability::High,
            is_current: false,
            supersedes: vec![id(99)],
            retention_state: RetentionState::Active,
            created_at: timestamp(),
            updated_at: timestamp(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        }
    }

    fn thread_fixture() -> MemoryThread {
        MemoryThread {
            id: id(30),
            object_type: ObjectType::MemoryThread,
            title: "Useful thread".to_owned(),
            summary: "Thread summary.".to_owned(),
            status: ThreadStatus::Dormant,
            last_touched_at: timestamp(),
            salience_score: 0.42,
            canonical_key: Some("thread-key".to_owned()),
            created_at: timestamp(),
            updated_at: timestamp(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        }
    }

    fn entity_fixture() -> Entity {
        Entity {
            id: id(1),
            object_type: ObjectType::Entity,
            entity_type: EntityType::User,
            name: "Kohta".to_owned(),
            aliases: vec!["K.".to_owned()],
            canonical_key: Some("person:kohta".to_owned()),
            summary: Some("User summary.".to_owned()),
            created_at: timestamp(),
            updated_at: timestamp(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        }
    }

    fn link_fixture() -> MemoryLink {
        MemoryLink {
            id: id(50),
            object_type: ObjectType::MemoryLink,
            from_id: id(40),
            from_type: ObjectType::DerivedMemory,
            to_id: id(10),
            to_type: ObjectType::Episode,
            relation: RelationType::DerivedFrom,
            confidence: 1.0,
            rationale: Some("Derived from episode".to_owned()),
            created_at: timestamp(),
            schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
        }
    }

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(0x550e_8400_e29b_41d4_a716_4466_5544_0000 + value)
    }

    fn timestamp() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 28, 12, 0, 0).unwrap()
    }
}
