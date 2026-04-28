use super::*;

use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

fn serialized_value<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}

fn memory_id(value: &str) -> MemoryId {
    Uuid::parse_str(value).unwrap()
}

fn timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

fn round_trip<T>(value: &T) -> T
where
    T: Serialize + DeserializeOwned,
{
    let serialized = serde_json::to_string(value).unwrap();
    serde_json::from_str(&serialized).unwrap()
}

fn representative_episode() -> Episode {
    Episode {
        id: memory_id("550e8400-e29b-41d4-a716-446655440000"),
        object_type: ObjectType::Episode,
        modality: Modality::Chat,
        source_conversation_id: Some("conversation-2026-04-27".to_owned()),
        started_at: Some(timestamp("2026-04-27T10:00:00Z")),
        ended_at: Some(timestamp("2026-04-27T10:05:00Z")),
        participant_entity_ids: vec![memory_id("550e8400-e29b-41d4-a716-446655440001")],
        summary: "Discussed the episodic memory domain model.".to_owned(),
        raw_ref: Some("raw://conversation/2026-04-27#episode-1".to_owned()),
        salience_score: 0.8,
        retention_state: RetentionState::Active,
        created_at: timestamp("2026-04-27T10:06:00Z"),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    }
}

fn representative_observation() -> Observation {
    Observation {
        id: memory_id("550e8400-e29b-41d4-a716-446655440010"),
        object_type: ObjectType::Observation,
        episode_id: memory_id("550e8400-e29b-41d4-a716-446655440000"),
        speaker_entity_id: Some(memory_id("550e8400-e29b-41d4-a716-446655440001")),
        observed_at: Some(timestamp("2026-04-27T10:01:00Z")),
        modality: Modality::Chat,
        text: "Use raw references without storing raw input in production.".to_owned(),
        raw_ref: Some("raw://conversation/2026-04-27#message-2".to_owned()),
        salience_score: 0.7,
        retention_state: RetentionState::Active,
        created_at: timestamp("2026-04-27T10:06:01Z"),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    }
}

fn valid_episode() -> Episode {
    Episode {
        summary: "Discussed deterministic model validation.".to_owned(),
        raw_ref: Some("raw-fixture:episode-1".to_owned()),
        ..representative_episode()
    }
}

fn valid_observation() -> Observation {
    Observation {
        text: "The model keeps raw text outside domain objects.".to_owned(),
        raw_ref: Some("raw-fixture:message-1".to_owned()),
        ..representative_observation()
    }
}

fn valid_derived_memory() -> DerivedMemory {
    DerivedMemory {
        id: memory_id("550e8400-e29b-41d4-a716-446655440030"),
        object_type: ObjectType::DerivedMemory,
        derived_type: DerivedType::ProjectNote,
        text: "Raw text is externally referenced by fixture ID.".to_owned(),
        derived_from_episode_ids: vec![memory_id("550e8400-e29b-41d4-a716-446655440000")],
        derived_from_observation_ids: vec![memory_id("550e8400-e29b-41d4-a716-446655440010")],
        thread_ids: vec![],
        entity_ids: vec![],
        confidence: 0.9,
        salience_score: 0.85,
        stability: Stability::Medium,
        is_current: true,
        supersedes: vec![],
        retention_state: RetentionState::Active,
        created_at: timestamp("2026-04-27T10:08:00Z"),
        updated_at: timestamp("2026-04-27T10:08:30Z"),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    }
}

fn valid_memory_link() -> MemoryLink {
    MemoryLink {
        id: memory_id("550e8400-e29b-41d4-a716-446655440040"),
        object_type: ObjectType::MemoryLink,
        from_id: memory_id("550e8400-e29b-41d4-a716-446655440030"),
        from_type: ObjectType::DerivedMemory,
        to_id: memory_id("550e8400-e29b-41d4-a716-446655440000"),
        to_type: ObjectType::Episode,
        relation: RelationType::DerivedFrom,
        confidence: 0.95,
        rationale: Some("Derived memory cites its source episode.".to_owned()),
        created_at: timestamp("2026-04-27T10:09:00Z"),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    }
}

struct FileBackedRawRefFixture {
    path: PathBuf,
    raw_text: String,
}

impl FileBackedRawRefFixture {
    fn new(raw_text: &str) -> Self {
        let path = std::env::temp_dir().join(format!("cmem-raw-ref-{}.txt", Uuid::new_v4()));
        fs::write(&path, raw_text).unwrap();

        Self {
            path,
            raw_text: raw_text.to_owned(),
        }
    }

    fn raw_ref(&self) -> String {
        format!("file:{}", self.path.display())
    }
}

impl Drop for FileBackedRawRefFixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[test]
fn domain_enums_serialize_as_snake_case() {
    let cases = [
        serialized_value(ObjectType::MemoryThread),
        serialized_value(ObjectType::DerivedMemory),
        serialized_value(ObjectType::MemoryLink),
        serialized_value(Modality::VoiceTranscript),
        serialized_value(EntityType::Assistant),
        serialized_value(EntityType::Organization),
        serialized_value(DerivedType::AssistantPreference),
        serialized_value(DerivedType::RelationshipNote),
        serialized_value(RelationType::HasObservation),
        serialized_value(RelationType::CreatesOpenLoop),
        serialized_value(RelationType::FulfillsCommitment),
        serialized_value(RetentionState::Suppressed),
        serialized_value(Stability::Medium),
        serialized_value(ThreadStatus::Dormant),
    ];

    assert_eq!(
        cases,
        [
            "memory_thread",
            "derived_memory",
            "memory_link",
            "voice_transcript",
            "assistant",
            "organization",
            "assistant_preference",
            "relationship_note",
            "has_observation",
            "creates_open_loop",
            "fulfills_commitment",
            "suppressed",
            "medium",
            "dormant",
        ]
    );
}

#[test]
fn graph_uri_maps_object_types_to_stable_urns() {
    let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let memory_id: MemoryId = id;

    let cases = [
        (ObjectType::Episode, "urn:cmem:episode"),
        (ObjectType::Observation, "urn:cmem:observation"),
        (ObjectType::Entity, "urn:cmem:entity"),
        (ObjectType::MemoryThread, "urn:cmem:thread"),
        (ObjectType::DerivedMemory, "urn:cmem:derived-memory"),
        (ObjectType::MemoryLink, "urn:cmem:link"),
    ];

    for (object_type, prefix) in cases {
        assert_eq!(graph_uri(object_type, memory_id), format!("{prefix}:{id}"));
    }

    assert_eq!(
        graph_uri(ObjectType::Episode, memory_id),
        graph_uri(ObjectType::Episode, memory_id)
    );
}

#[test]
fn schema_version_constants_are_pinned_to_the_initial_episodic_memory_schema() {
    assert_eq!(EPISODIC_MEMORY_SCHEMA_VERSION, "episodic_memory_initial");
    assert_eq!(CURRENT_SCHEMA_VERSION, EPISODIC_MEMORY_SCHEMA_VERSION);
    assert_eq!(DEFAULT_SCHEMA_VERSION, EPISODIC_MEMORY_SCHEMA_VERSION);
}

#[test]
fn representative_domain_objects_round_trip_through_serde() {
    let episode = representative_episode();
    let observation = representative_observation();
    let entity = Entity {
        id: memory_id("550e8400-e29b-41d4-a716-446655440001"),
        object_type: ObjectType::Entity,
        entity_type: EntityType::User,
        name: "Kohta".to_owned(),
        aliases: vec!["workspace user".to_owned()],
        canonical_key: Some("person:kohta".to_owned()),
        summary: Some("Primary workspace user.".to_owned()),
        created_at: timestamp("2026-04-27T10:06:02Z"),
        updated_at: timestamp("2026-04-27T10:06:03Z"),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    };
    let thread = MemoryThread {
        id: memory_id("550e8400-e29b-41d4-a716-446655440020"),
        object_type: ObjectType::MemoryThread,
        title: "Episodic memory domain foundation".to_owned(),
        summary: "Model foundation planning and implementation.".to_owned(),
        status: ThreadStatus::Active,
        last_touched_at: timestamp("2026-04-27T10:07:00Z"),
        salience_score: 0.75,
        canonical_key: Some("thread:episodic-memory-domain-foundation".to_owned()),
        created_at: timestamp("2026-04-27T10:06:04Z"),
        updated_at: timestamp("2026-04-27T10:07:00Z"),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    };
    let derived = DerivedMemory {
        text: "The domain model uses external raw references.".to_owned(),
        derived_from_episode_ids: vec![episode.id],
        derived_from_observation_ids: vec![observation.id],
        thread_ids: vec![thread.id],
        entity_ids: vec![entity.id],
        ..valid_derived_memory()
    };
    let link = MemoryLink {
        from_id: derived.id,
        to_id: episode.id,
        rationale: Some("Derived project note cites the source episode.".to_owned()),
        ..valid_memory_link()
    };

    assert_eq!(round_trip(&episode), episode);
    assert_eq!(round_trip(&observation), observation);
    assert_eq!(round_trip(&entity), entity);
    assert_eq!(round_trip(&thread), thread);
    assert_eq!(round_trip(&derived), derived);
    assert_eq!(round_trip(&link), link);
}

#[test]
fn raw_references_are_preserved_as_external_reference_strings() {
    let episode = representative_episode();
    let observation = representative_observation();

    let round_tripped_episode: Episode = round_trip(&episode);
    let round_tripped_observation: Observation = round_trip(&observation);

    assert_eq!(round_tripped_episode.raw_ref, episode.raw_ref);
    assert_eq!(round_tripped_observation.raw_ref, observation.raw_ref);
}

#[test]
fn memory_object_round_trips_with_tagged_object_type() {
    let object = MemoryObject::Episode(representative_episode());
    let serialized = serde_json::to_value(&object).unwrap();

    assert_eq!(serialized["object_type"], "episode");

    let round_tripped: MemoryObject = serde_json::from_value(serialized).unwrap();
    assert_eq!(round_tripped, object);
}

#[test]
fn validation_accepts_representative_valid_objects() {
    let episode = valid_episode();
    let observation = valid_observation();
    let derived = valid_derived_memory();
    let link = valid_memory_link();

    assert_eq!(episode.validate(), Ok(()));
    assert_eq!(observation.validate(), Ok(()));
    assert_eq!(derived.validate(), Ok(()));
    assert_eq!(link.validate(), Ok(()));
    assert_eq!(MemoryObject::Episode(episode).validate(), Ok(()));
}

#[test]
fn episode_validation_rejects_empty_or_whitespace_summary() {
    for summary in ["", "   \n\t"] {
        let mut episode = valid_episode();
        episode.summary = summary.to_owned();

        assert_eq!(
            episode.validate(),
            Err(DomainValidationError::EmptyEpisodeSummary)
        );
    }
}

#[test]
fn observation_validation_rejects_nil_episode_reference() {
    let mut observation = valid_observation();
    observation.episode_id = Uuid::nil();

    assert_eq!(
        observation.validate(),
        Err(DomainValidationError::MissingEpisodeReference)
    );
}

#[test]
fn derived_memory_validation_requires_episode_or_observation_source() {
    let mut derived = valid_derived_memory();
    derived.derived_from_episode_ids.clear();
    derived.derived_from_observation_ids.clear();

    assert_eq!(
        derived.validate(),
        Err(DomainValidationError::MissingDerivedSource)
    );
}

#[test]
fn score_validation_rejects_out_of_range_and_nan_values() {
    let mut episode = valid_episode();
    episode.salience_score = -0.01;
    assert!(matches!(
        episode.validate(),
        Err(DomainValidationError::InvalidScore {
            field: "Episode.salience_score",
            ..
        })
    ));

    let mut observation = valid_observation();
    observation.salience_score = 1.01;
    assert!(matches!(
        observation.validate(),
        Err(DomainValidationError::InvalidScore {
            field: "Observation.salience_score",
            ..
        })
    ));

    let mut derived = valid_derived_memory();
    derived.confidence = f32::NAN;
    assert!(matches!(
        derived.validate(),
        Err(DomainValidationError::InvalidScore {
            field: "DerivedMemory.confidence",
            ..
        })
    ));

    let mut link = valid_memory_link();
    link.confidence = f32::INFINITY;
    assert!(matches!(
        link.validate(),
        Err(DomainValidationError::InvalidScore {
            field: "MemoryLink.confidence",
            ..
        })
    ));
}

#[test]
fn object_type_validation_rejects_mismatched_containing_type() {
    let mut episode = valid_episode();
    episode.object_type = ObjectType::Observation;

    assert_eq!(
        episode.validate(),
        Err(DomainValidationError::ObjectTypeMismatch {
            field: "Episode.object_type",
            expected: ObjectType::Episode,
            actual: ObjectType::Observation,
        })
    );
}

#[test]
fn file_backed_raw_ref_fixture_preserves_reference_without_embedding_payload() {
    let fixture = FileBackedRawRefFixture::new(
        "verbatim raw transcript text that should stay outside the memory object",
    );
    let raw_ref = fixture.raw_ref();
    let mut episode = valid_episode();
    episode.raw_ref = Some(raw_ref.clone());
    episode.summary = "Summarized external transcript fixture.".to_owned();

    let serialized = serde_json::to_string(&episode).unwrap();
    let serialized_value = serde_json::to_value(&episode).unwrap();

    assert_eq!(fs::read_to_string(&fixture.path).unwrap(), fixture.raw_text);
    assert_eq!(episode.raw_ref.as_deref(), Some(raw_ref.as_str()));
    assert_eq!(serialized_value["raw_ref"], raw_ref);
    assert!(!serialized.contains(&fixture.raw_text));
    assert_eq!(episode.validate(), Ok(()));
}
