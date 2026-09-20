use super::*;

use chrono::{DateTime, Utc};
use serde::Serialize;
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

#[test]
fn canonical_identity_and_order_ranks_are_stable() {
    let episode = MemoryObject::Episode(representative_episode());

    assert_eq!(
        episode.id(),
        memory_id("550e8400-e29b-41d4-a716-446655440000")
    );
    assert_eq!(episode.object_type(), ObjectType::Episode);
    assert_eq!(
        episode.object_ref(),
        MemoryObjectRef::new(ObjectType::Episode, episode.id())
    );
    assert_eq!(
        episode.stable_order_key(),
        (episode.id(), ObjectType::Episode.stable_rank())
    );

    let object_ranks = [
        ObjectType::Episode,
        ObjectType::Observation,
        ObjectType::Entity,
        ObjectType::MemoryThread,
        ObjectType::DerivedMemory,
        ObjectType::MemoryLink,
    ]
    .map(ObjectType::stable_rank);
    let relation_ranks = [
        RelationType::HasObservation,
        RelationType::ObservedIn,
        RelationType::Mentions,
        RelationType::Involves,
        RelationType::About,
        RelationType::DerivedFrom,
        RelationType::PartOfThread,
        RelationType::Supports,
        RelationType::Contradicts,
        RelationType::Supersedes,
        RelationType::Resolves,
        RelationType::CreatesOpenLoop,
        RelationType::FulfillsCommitment,
        RelationType::AssociatedWith,
    ]
    .map(RelationType::stable_rank);
    for ranks in [object_ranks.as_slice(), relation_ranks.as_slice()] {
        assert!(ranks.windows(2).all(|pair| pair[0] < pair[1]));
    }
    let retention_ranks = [RetentionState::Active, RetentionState::Suppressed]
        .map(RetentionState::restrictiveness_rank);
    assert!(retention_ranks.windows(2).all(|pair| pair[0] < pair[1]));
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
        salience_score: 0.85,
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
        rationale: Some("Derived memory cites its source episode.".to_owned()),
        created_at: timestamp("2026-04-27T10:09:00Z"),
        schema_version: DEFAULT_SCHEMA_VERSION.to_owned(),
    }
}

#[test]
fn domain_enums_serialize_as_snake_case() {
    let cases = [
        serialized_value(ObjectType::Episode),
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
        serialized_value(ThreadStatus::Dormant),
    ];

    assert_eq!(
        cases,
        [
            "episode",
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
}

#[test]
fn schema_version_constants_are_pinned_to_the_initial_episodic_memory_schema() {
    assert_eq!(EPISODIC_MEMORY_SCHEMA_VERSION, "episodic_memory_initial");
    assert_eq!(CURRENT_SCHEMA_VERSION, EPISODIC_MEMORY_SCHEMA_VERSION);
    assert_eq!(DEFAULT_SCHEMA_VERSION, EPISODIC_MEMORY_SCHEMA_VERSION);
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
    derived.salience_score = f32::NAN;
    assert!(matches!(
        derived.validate(),
        Err(DomainValidationError::InvalidScore {
            field: "DerivedMemory.salience_score",
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
fn raw_references_serialize_without_embedding_transcript_payload() {
    let raw_text = "verbatim raw transcript text that should stay outside the memory object";
    let mut episode = valid_episode();
    episode.raw_ref = Some("file:fixtures/raw/episode.txt".to_owned());
    episode.summary = "Summarized external transcript fixture.".to_owned();
    let observation = valid_observation();

    for (serialized, raw_ref) in [
        (
            serde_json::to_value(&episode).unwrap(),
            episode.raw_ref.as_deref().unwrap(),
        ),
        (
            serde_json::to_value(&observation).unwrap(),
            observation.raw_ref.as_deref().unwrap(),
        ),
    ] {
        assert_eq!(serialized["raw_ref"], raw_ref);
        assert!(serialized.get("raw_transcript").is_none());
        assert!(!serialized.to_string().contains(raw_text));
    }
    assert_eq!(episode.validate(), Ok(()));
}
