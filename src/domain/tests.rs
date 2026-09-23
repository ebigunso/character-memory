use super::*;

use crate::test_support::representative_fixtures;
use serde::Serialize;
use uuid::Uuid;

fn serialized_value<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn canonical_identity_and_order_ranks_are_stable() {
    let fixtures = representative_fixtures();
    let episode = MemoryObject::Episode(fixtures.episode.clone());

    assert_eq!(episode.id(), fixtures.episode.id);
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

#[test]
fn domain_enums_serialize_as_snake_case() {
    let cases = [
        serialized_value(ObjectType::Episode),
        serialized_value(ObjectType::MemoryThread),
        serialized_value(ObjectType::DerivedMemory),
        serialized_value(ObjectType::MemoryLink),
        serialized_value(Modality::VoiceTranscript),
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
fn validation_accepts_representative_valid_objects() {
    let fixtures = representative_fixtures();
    let episode = fixtures.episode;
    let observation = fixtures.salient_observation;
    let derived = fixtures.derived_reflection;
    let link = fixtures.soft_thread_link;

    assert_eq!(episode.validate(), Ok(()));
    assert_eq!(observation.validate(), Ok(()));
    assert_eq!(derived.validate(), Ok(()));
    assert_eq!(link.validate(), Ok(()));
    assert_eq!(MemoryObject::Episode(episode).validate(), Ok(()));
}

#[test]
fn episode_validation_rejects_empty_or_whitespace_summary() {
    for summary in ["", "   \n\t"] {
        let mut episode = representative_fixtures().episode;
        episode.summary = summary.to_owned();

        assert_eq!(
            episode.validate(),
            Err(DomainValidationError::EmptyEpisodeSummary)
        );
    }
}

#[test]
fn observation_validation_rejects_nil_episode_reference() {
    let mut observation = representative_fixtures().salient_observation;
    observation.episode_id = Uuid::nil();

    assert_eq!(
        observation.validate(),
        Err(DomainValidationError::MissingEpisodeReference)
    );
}

#[test]
fn derived_memory_validation_requires_episode_or_observation_source() {
    let mut derived = representative_fixtures().derived_reflection;
    derived.derived_from_episode_ids.clear();
    derived.derived_from_observation_ids.clear();

    assert_eq!(
        derived.validate(),
        Err(DomainValidationError::MissingDerivedSource)
    );
}

#[test]
fn score_validation_rejects_out_of_range_and_nan_values() {
    let mut episode = representative_fixtures().episode;
    episode.salience_score = -0.01;
    assert!(matches!(
        episode.validate(),
        Err(DomainValidationError::InvalidScore {
            field: "Episode.salience_score",
            ..
        })
    ));

    let mut observation = representative_fixtures().salient_observation;
    observation.salience_score = 1.01;
    assert!(matches!(
        observation.validate(),
        Err(DomainValidationError::InvalidScore {
            field: "Observation.salience_score",
            ..
        })
    ));

    let mut derived = representative_fixtures().derived_reflection;
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
    let mut episode = representative_fixtures().episode;
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
    let mut episode = representative_fixtures().episode;
    episode.raw_ref = Some("file:fixtures/raw/episode.txt".to_owned());
    episode.summary = "Summarized external transcript fixture.".to_owned();
    let observation = representative_fixtures().salient_observation;

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
