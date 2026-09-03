// Embedding-surface builders for graph objects that participate in vector
// candidate recall.
use crate::domain::{
    DerivedMemory, Entity, Episode, MemoryObject, MemoryThread, ObjectType, Observation,
    VectorSurface,
};

use crate::models::vector::VectorRecord;

pub const fn max_embedding_surfaces(object_type: ObjectType) -> usize {
    match object_type {
        ObjectType::Episode
        | ObjectType::Observation
        | ObjectType::Entity
        | ObjectType::MemoryThread
        | ObjectType::DerivedMemory => 1,
        ObjectType::MemoryLink => 0,
    }
}

pub(crate) fn episode_vector_record(episode: &Episode) -> VectorRecord {
    VectorRecord::new(
        episode.id,
        ObjectType::Episode,
        VectorSurface::Summary,
        episode.schema_version.clone(),
        prefixed_text("Episode summary", &episode.summary),
    )
}

pub(crate) fn observation_vector_record(observation: &Observation) -> VectorRecord {
    VectorRecord::new(
        observation.id,
        ObjectType::Observation,
        VectorSurface::Text,
        observation.schema_version.clone(),
        prefixed_text("Observation excerpt", &observation.text),
    )
}

pub(crate) fn derived_memory_vector_record(memory: &DerivedMemory) -> VectorRecord {
    VectorRecord::new(
        memory.id,
        ObjectType::DerivedMemory,
        VectorSurface::DerivedText,
        memory.schema_version.clone(),
        prefixed_text(derived_label(memory), &memory.text),
    )
}

pub(crate) fn memory_thread_vector_record(thread: &MemoryThread) -> VectorRecord {
    let surface_text = join_clean([thread.title.as_str(), thread.summary.as_str()]);

    VectorRecord::new(
        thread.id,
        ObjectType::MemoryThread,
        VectorSurface::Summary,
        thread.schema_version.clone(),
        prefixed_text("Thread summary", &surface_text),
    )
}

pub(crate) fn entity_vector_record(entity: &Entity) -> VectorRecord {
    let alias_text = if entity.aliases.is_empty() {
        String::new()
    } else {
        format!("Aliases: {}", entity.aliases.join(", "))
    };
    let summary = entity.summary.as_deref().unwrap_or_default();
    let surface_text = join_clean([entity.name.as_str(), alias_text.as_str(), summary]);

    VectorRecord::new(
        entity.id,
        ObjectType::Entity,
        VectorSurface::Name,
        entity.schema_version.clone(),
        prefixed_text("Entity", &surface_text),
    )
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
    fn episode_builder_uses_summary_surface() {
        let episode = episode_fixture();
        let record = episode_vector_record(&episode);

        assert_eq!(record.object_id, episode.id);
        assert_eq!(record.object_type, ObjectType::Episode);
        assert_eq!(record.surface, VectorSurface::Summary);
        assert_eq!(record.embedding_text, "Episode summary: Short summary.");
        assert_eq!(record.schema_version, DEFAULT_SCHEMA_VERSION);
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
        assert_embedding_text_excludes_metadata(&record);
    }

    #[test]
    fn derived_memory_builder_keeps_graph_state_out_of_embedding_text() {
        let derived = derived_memory_fixture();
        let record = derived_memory_vector_record(&derived);

        assert_eq!(record.surface, VectorSurface::DerivedText);
        assert_eq!(record.embedding_text, "Reflection: Derived insight.");
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

    #[test]
    fn published_surface_limits_match_current_builders() {
        let objects = [
            MemoryObject::Episode(episode_fixture()),
            MemoryObject::Observation(observation_fixture()),
            MemoryObject::DerivedMemory(derived_memory_fixture()),
            MemoryObject::MemoryThread(thread_fixture()),
            MemoryObject::Entity(entity_fixture()),
            MemoryObject::MemoryLink(link_fixture()),
        ];

        for object in &objects {
            let produced = usize::from(memory_object_vector_record(object).is_some());
            assert_eq!(produced, max_embedding_surfaces(object.object_type()));
        }
    }

    fn assert_embedding_text_excludes_metadata(record: &VectorRecord) {
        assert!(!record
            .embedding_text
            .contains(&record.object_id.to_string()));
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
