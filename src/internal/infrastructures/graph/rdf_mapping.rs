// RDF mapping surface. Public domain objects stay independent from
// Oxigraph types.
#![allow(dead_code)]

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;

use crate::api::types::{
    graph_uri, DerivedMemory, Entity, Episode, MemoryId, MemoryLink, MemoryObject, MemoryThread,
    ObjectType, Observation,
};
use crate::errors::CustomError;
use crate::internal::schema::require_current_schema_version;

use super::vocabulary as vocab;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RdfObject {
    Resource(String),
    Literal(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RdfTriple {
    pub(crate) subject: String,
    pub(crate) predicate: String,
    pub(crate) object: RdfObject,
}

impl RdfTriple {
    fn resource(
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
    ) -> Self {
        Self {
            subject: subject.into(),
            predicate: predicate.into(),
            object: RdfObject::Resource(object.into()),
        }
    }

    fn literal(
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
    ) -> Self {
        Self {
            subject: subject.into(),
            predicate: predicate.into(),
            object: RdfObject::Literal(object.into()),
        }
    }
}

pub(crate) fn rdf_triples_for_object(object: &MemoryObject) -> Result<Vec<RdfTriple>, CustomError> {
    match object {
        MemoryObject::Episode(object) => {
            require_current_schema_version(&object.schema_version, "RDF episode mapping")?;
            Ok(episode_triples(object))
        }
        MemoryObject::Observation(object) => {
            require_current_schema_version(&object.schema_version, "RDF observation mapping")?;
            Ok(observation_triples(object))
        }
        MemoryObject::Entity(object) => {
            require_current_schema_version(&object.schema_version, "RDF entity mapping")?;
            Ok(entity_triples(object))
        }
        MemoryObject::MemoryThread(object) => {
            require_current_schema_version(&object.schema_version, "RDF memory thread mapping")?;
            Ok(memory_thread_triples(object))
        }
        MemoryObject::DerivedMemory(object) => {
            require_current_schema_version(&object.schema_version, "RDF derived memory mapping")?;
            Ok(derived_memory_triples(object))
        }
        MemoryObject::MemoryLink(object) => rdf_triples_for_link(object),
    }
}

pub(crate) fn rdf_triples_for_link(link: &MemoryLink) -> Result<Vec<RdfTriple>, CustomError> {
    require_current_schema_version(&link.schema_version, "RDF memory link mapping")?;

    let subject = graph_uri(ObjectType::MemoryLink, link.id);
    let from = graph_uri(link.from_type, link.from_id);
    let to = graph_uri(link.to_type, link.to_id);
    let relation = enum_value(link.relation);
    let relation_predicate = vocab::relation_predicate(&relation);
    let mut triples = base_triples(
        &subject,
        link.id,
        ObjectType::MemoryLink,
        vocab::CLASS_MEMORY_LINK,
        &link.schema_version,
    );

    triples.extend([
        RdfTriple::resource(&subject, vocab::FROM, from.clone()),
        RdfTriple::literal(&subject, vocab::FROM_TYPE, enum_value(link.from_type)),
        RdfTriple::resource(&subject, vocab::TO, to.clone()),
        RdfTriple::literal(&subject, vocab::TO_TYPE, enum_value(link.to_type)),
        RdfTriple::literal(&subject, vocab::RELATION, relation),
        RdfTriple::literal(&subject, vocab::CONFIDENCE, score(link.confidence)),
        RdfTriple::literal(&subject, vocab::CREATED_AT, timestamp(link.created_at)),
        RdfTriple::resource(from, relation_predicate, to),
    ]);
    push_optional_literal(
        &mut triples,
        &subject,
        vocab::RATIONALE,
        link.rationale.as_deref(),
    );
    Ok(triples)
}

fn episode_triples(episode: &Episode) -> Vec<RdfTriple> {
    let subject = graph_uri(ObjectType::Episode, episode.id);
    let mut triples = base_triples(
        &subject,
        episode.id,
        episode.object_type,
        vocab::CLASS_EPISODE,
        &episode.schema_version,
    );
    triples.extend([
        RdfTriple::literal(&subject, vocab::MODALITY, enum_value(episode.modality)),
        RdfTriple::literal(&subject, vocab::SUMMARY, episode.summary.clone()),
        RdfTriple::literal(
            &subject,
            vocab::SALIENCE_SCORE,
            score(episode.salience_score),
        ),
        RdfTriple::literal(
            &subject,
            vocab::RETENTION_STATE,
            enum_value(episode.retention_state),
        ),
        RdfTriple::literal(&subject, vocab::CREATED_AT, timestamp(episode.created_at)),
    ]);
    push_optional_literal(
        &mut triples,
        &subject,
        vocab::SOURCE_CONVERSATION_ID,
        episode.source_conversation_id.as_deref(),
    );
    push_optional_literal(
        &mut triples,
        &subject,
        vocab::STARTED_AT,
        episode.started_at.map(timestamp),
    );
    push_optional_literal(
        &mut triples,
        &subject,
        vocab::ENDED_AT,
        episode.ended_at.map(timestamp),
    );
    push_optional_literal(
        &mut triples,
        &subject,
        vocab::RAW_REF,
        episode.raw_ref.as_deref(),
    );
    for entity_id in &episode.participant_entity_ids {
        triples.push(RdfTriple::resource(
            &subject,
            vocab::PARTICIPANT_ENTITY,
            graph_uri(ObjectType::Entity, *entity_id),
        ));
    }
    triples
}

fn observation_triples(observation: &Observation) -> Vec<RdfTriple> {
    let subject = graph_uri(ObjectType::Observation, observation.id);
    let mut triples = base_triples(
        &subject,
        observation.id,
        observation.object_type,
        vocab::CLASS_OBSERVATION,
        &observation.schema_version,
    );
    triples.extend([
        RdfTriple::resource(
            &subject,
            vocab::EPISODE,
            graph_uri(ObjectType::Episode, observation.episode_id),
        ),
        RdfTriple::literal(&subject, vocab::MODALITY, enum_value(observation.modality)),
        RdfTriple::literal(&subject, vocab::TEXT, observation.text.clone()),
        RdfTriple::literal(
            &subject,
            vocab::SALIENCE_SCORE,
            score(observation.salience_score),
        ),
        RdfTriple::literal(
            &subject,
            vocab::RETENTION_STATE,
            enum_value(observation.retention_state),
        ),
        RdfTriple::literal(
            &subject,
            vocab::CREATED_AT,
            timestamp(observation.created_at),
        ),
    ]);
    if let Some(speaker_id) = observation.speaker_entity_id {
        triples.push(RdfTriple::resource(
            &subject,
            vocab::SPEAKER_ENTITY,
            graph_uri(ObjectType::Entity, speaker_id),
        ));
    }
    push_optional_literal(
        &mut triples,
        &subject,
        vocab::OBSERVED_AT,
        observation.observed_at.map(timestamp),
    );
    push_optional_literal(
        &mut triples,
        &subject,
        vocab::RAW_REF,
        observation.raw_ref.as_deref(),
    );
    triples
}

fn entity_triples(entity: &Entity) -> Vec<RdfTriple> {
    let subject = graph_uri(ObjectType::Entity, entity.id);
    let mut triples = base_triples(
        &subject,
        entity.id,
        entity.object_type,
        vocab::CLASS_ENTITY,
        &entity.schema_version,
    );
    triples.extend([
        RdfTriple::literal(&subject, vocab::ENTITY_TYPE, enum_value(entity.entity_type)),
        RdfTriple::literal(&subject, vocab::NAME, entity.name.clone()),
        RdfTriple::literal(&subject, vocab::CREATED_AT, timestamp(entity.created_at)),
        RdfTriple::literal(&subject, vocab::UPDATED_AT, timestamp(entity.updated_at)),
    ]);
    for alias in &entity.aliases {
        triples.push(RdfTriple::literal(&subject, vocab::ALIAS, alias.clone()));
    }
    push_optional_literal(
        &mut triples,
        &subject,
        vocab::CANONICAL_KEY,
        entity.canonical_key.as_deref(),
    );
    push_optional_literal(
        &mut triples,
        &subject,
        vocab::SUMMARY,
        entity.summary.as_deref(),
    );
    triples
}

fn memory_thread_triples(thread: &MemoryThread) -> Vec<RdfTriple> {
    let subject = graph_uri(ObjectType::MemoryThread, thread.id);
    let mut triples = base_triples(
        &subject,
        thread.id,
        thread.object_type,
        vocab::CLASS_MEMORY_THREAD,
        &thread.schema_version,
    );
    triples.extend([
        RdfTriple::literal(&subject, vocab::TITLE, thread.title.clone()),
        RdfTriple::literal(&subject, vocab::SUMMARY, thread.summary.clone()),
        RdfTriple::literal(&subject, vocab::THREAD_STATUS, enum_value(thread.status)),
        RdfTriple::literal(
            &subject,
            vocab::LAST_TOUCHED_AT,
            timestamp(thread.last_touched_at),
        ),
        RdfTriple::literal(
            &subject,
            vocab::SALIENCE_SCORE,
            score(thread.salience_score),
        ),
        RdfTriple::literal(&subject, vocab::CREATED_AT, timestamp(thread.created_at)),
        RdfTriple::literal(&subject, vocab::UPDATED_AT, timestamp(thread.updated_at)),
    ]);
    push_optional_literal(
        &mut triples,
        &subject,
        vocab::CANONICAL_KEY,
        thread.canonical_key.as_deref(),
    );
    triples
}

fn derived_memory_triples(memory: &DerivedMemory) -> Vec<RdfTriple> {
    let subject = graph_uri(ObjectType::DerivedMemory, memory.id);
    let mut triples = base_triples(
        &subject,
        memory.id,
        memory.object_type,
        vocab::CLASS_DERIVED_MEMORY,
        &memory.schema_version,
    );
    triples.extend([
        RdfTriple::literal(
            &subject,
            vocab::DERIVED_TYPE,
            enum_value(memory.derived_type),
        ),
        RdfTriple::literal(&subject, vocab::TEXT, memory.text.clone()),
        RdfTriple::literal(&subject, vocab::CONFIDENCE, score(memory.confidence)),
        RdfTriple::literal(
            &subject,
            vocab::SALIENCE_SCORE,
            score(memory.salience_score),
        ),
        RdfTriple::literal(&subject, vocab::STABILITY, enum_value(memory.stability)),
        RdfTriple::literal(&subject, vocab::IS_CURRENT, memory.is_current.to_string()),
        RdfTriple::literal(
            &subject,
            vocab::RETENTION_STATE,
            enum_value(memory.retention_state),
        ),
        RdfTriple::literal(&subject, vocab::CREATED_AT, timestamp(memory.created_at)),
        RdfTriple::literal(&subject, vocab::UPDATED_AT, timestamp(memory.updated_at)),
    ]);
    for id in &memory.derived_from_episode_ids {
        triples.push(RdfTriple::resource(
            &subject,
            vocab::DERIVED_FROM_EPISODE,
            graph_uri(ObjectType::Episode, *id),
        ));
    }
    for id in &memory.derived_from_observation_ids {
        triples.push(RdfTriple::resource(
            &subject,
            vocab::DERIVED_FROM_OBSERVATION,
            graph_uri(ObjectType::Observation, *id),
        ));
    }
    for id in &memory.thread_ids {
        triples.push(RdfTriple::resource(
            &subject,
            vocab::PART_OF_THREAD,
            graph_uri(ObjectType::MemoryThread, *id),
        ));
    }
    for id in &memory.entity_ids {
        triples.push(RdfTriple::resource(
            &subject,
            vocab::ABOUT_ENTITY,
            graph_uri(ObjectType::Entity, *id),
        ));
    }
    for id in &memory.supersedes {
        triples.push(RdfTriple::resource(
            &subject,
            vocab::SUPERSEDES,
            graph_uri(ObjectType::DerivedMemory, *id),
        ));
    }
    triples
}

fn base_triples(
    subject: &str,
    id: MemoryId,
    object_type: ObjectType,
    class: &'static str,
    schema_version: &str,
) -> Vec<RdfTriple> {
    vec![
        RdfTriple::resource(subject, vocab::RDF_TYPE, class),
        RdfTriple::literal(subject, vocab::OBJECT_ID, id.to_string()),
        RdfTriple::literal(subject, vocab::OBJECT_TYPE, enum_value(object_type)),
        RdfTriple::literal(subject, vocab::GRAPH_URI, subject),
        RdfTriple::literal(subject, vocab::SCHEMA_VERSION, schema_version),
    ]
}

fn push_optional_literal<T: Into<String>>(
    triples: &mut Vec<RdfTriple>,
    subject: &str,
    predicate: &'static str,
    value: Option<T>,
) {
    if let Some(value) = value {
        triples.push(RdfTriple::literal(subject, predicate, value));
    }
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn score(value: f32) -> String {
    value.to_string()
}

fn enum_value(value: impl Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::{graph_uri, ObjectType, DEFAULT_SCHEMA_VERSION};
    use crate::internal::repositories::test_support::representative_fixtures;

    #[test]
    fn rdf_mapping_uses_canonical_graph_uris_for_all_object_subjects() {
        let fixtures = representative_fixtures();

        for object in fixtures.objects() {
            let triples = rdf_triples_for_object(&object).expect("current schema maps");
            let (id, object_type) = object_identity(&object);

            assert!(triples
                .iter()
                .all(|triple| triple.subject == graph_uri(object_type, id)));
            assert!(triples.iter().any(|triple| {
                triple.predicate == vocab::GRAPH_URI
                    && triple.object == RdfObject::Literal(graph_uri(object_type, id))
            }));
        }
    }

    #[test]
    fn rdf_mapping_covers_lifecycle_currentness_provenance_and_supersession() {
        let fixtures = representative_fixtures();
        let triples =
            rdf_triples_for_object(&MemoryObject::DerivedMemory(fixtures.correction.clone()))
                .expect("current schema maps");

        assert_contains_literal(&triples, vocab::SCHEMA_VERSION, DEFAULT_SCHEMA_VERSION);
        assert_contains_literal(&triples, vocab::IS_CURRENT, "true");
        assert_contains_literal(&triples, vocab::RETENTION_STATE, "active");
        assert_contains_resource(
            &triples,
            vocab::DERIVED_FROM_EPISODE,
            &graph_uri(ObjectType::Episode, fixtures.episode.id),
        );
        assert_contains_resource(
            &triples,
            vocab::DERIVED_FROM_OBSERVATION,
            &graph_uri(ObjectType::Observation, fixtures.salient_observation.id),
        );
        assert_contains_resource(
            &triples,
            vocab::PART_OF_THREAD,
            &graph_uri(ObjectType::MemoryThread, fixtures.soft_thread.id),
        );
        assert_contains_resource(
            &triples,
            vocab::ABOUT_ENTITY,
            &graph_uri(ObjectType::Entity, fixtures.user_entity.id),
        );
        assert_contains_resource(
            &triples,
            vocab::SUPERSEDES,
            &graph_uri(ObjectType::DerivedMemory, fixtures.suppressed_seed.id),
        );
    }

    #[test]
    fn rdf_mapping_preserves_schema_version_literals_for_objects_and_links() {
        let fixtures = representative_fixtures();

        for object in fixtures.objects() {
            let triples = rdf_triples_for_object(&object).expect("current schema maps");

            assert_contains_literal(&triples, vocab::SCHEMA_VERSION, DEFAULT_SCHEMA_VERSION);
        }

        for link in fixtures.links() {
            let triples = rdf_triples_for_link(&link).expect("current schema maps");

            assert_contains_literal(&triples, vocab::SCHEMA_VERSION, DEFAULT_SCHEMA_VERSION);
        }
    }

    #[test]
    fn rdf_mapping_preserves_source_pointers_without_raw_transcript_literals() {
        let fixtures = representative_fixtures();
        let raw_transcript = "verbatim raw transcript text must remain external";
        let mut episode = fixtures.episode.clone();
        episode.raw_ref = Some("file:fixtures/raw/source-episode.txt".to_owned());
        episode.summary = "Summarized source episode.".to_owned();
        let mut observation = fixtures.salient_observation.clone();
        observation.raw_ref = Some("file:fixtures/raw/source-observation.txt".to_owned());
        observation.text = "A concise observation excerpt.".to_owned();

        let episode_triples =
            rdf_triples_for_object(&MemoryObject::Episode(episode)).expect("current schema maps");
        let observation_triples = rdf_triples_for_object(&MemoryObject::Observation(observation))
            .expect("current schema maps");

        assert_contains_literal(
            &episode_triples,
            vocab::RAW_REF,
            "file:fixtures/raw/source-episode.txt",
        );
        assert_contains_literal(
            &observation_triples,
            vocab::RAW_REF,
            "file:fixtures/raw/source-observation.txt",
        );
        assert_no_literal_value_contains(&episode_triples, raw_transcript);
        assert_no_literal_value_contains(&observation_triples, raw_transcript);
        assert!(!episode_triples
            .iter()
            .any(|triple| triple.predicate.contains("rawTranscript")));
        assert!(!observation_triples
            .iter()
            .any(|triple| triple.predicate.contains("rawTranscript")));
    }

    #[test]
    fn rdf_mapping_reifies_memory_links_and_adds_typed_relation_triples() {
        let fixtures = representative_fixtures();
        let link = fixtures.soft_thread_link;
        let triples = rdf_triples_for_link(&link).expect("current schema maps");
        let from = graph_uri(link.from_type, link.from_id);
        let to = graph_uri(link.to_type, link.to_id);

        assert_contains_resource(&triples, vocab::FROM, &from);
        assert_contains_resource(&triples, vocab::TO, &to);
        assert_contains_literal(&triples, vocab::RELATION, "part_of_thread");
        assert!(triples.iter().any(|triple| {
            triple.subject == from
                && triple.predicate == "urn:cmem:relation:part_of_thread"
                && triple.object == RdfObject::Resource(to.clone())
        }));
    }

    #[test]
    fn rdf_mapping_rejects_unsupported_schema_versions_for_all_object_variants() {
        let fixtures = representative_fixtures();
        let mut objects = fixtures.objects();
        objects.push(MemoryObject::MemoryLink(fixtures.soft_thread_link));

        for mut object in objects {
            set_schema_version(&mut object, "future_schema");

            let error = rdf_triples_for_object(&object).expect_err("unsupported schema fails");

            assert!(
                matches!(
                    error,
                    CustomError::UnsupportedSchemaVersion {
                        expected: DEFAULT_SCHEMA_VERSION,
                        ref actual,
                        ..
                    } if actual == "future_schema"
                ),
                "expected unsupported schema error for {object:?}, got {error:?}"
            );
        }
    }

    #[test]
    fn rdf_link_mapping_rejects_unsupported_schema_versions() {
        let fixtures = representative_fixtures();
        let mut link = fixtures.soft_thread_link;
        link.schema_version = "future_schema".to_owned();

        let error = rdf_triples_for_link(&link).expect_err("unsupported schema fails");

        assert!(matches!(
            error,
            CustomError::UnsupportedSchemaVersion {
                context: "RDF memory link mapping",
                ..
            }
        ));
    }

    fn assert_contains_literal(triples: &[RdfTriple], predicate: &'static str, value: &str) {
        assert!(triples.iter().any(|triple| {
            triple.predicate == predicate && triple.object == RdfObject::Literal(value.to_owned())
        }));
    }

    fn assert_contains_resource(triples: &[RdfTriple], predicate: &'static str, value: &str) {
        assert!(triples.iter().any(|triple| {
            triple.predicate == predicate && triple.object == RdfObject::Resource(value.to_owned())
        }));
    }

    fn assert_no_literal_value_contains(triples: &[RdfTriple], forbidden: &str) {
        assert!(triples.iter().all(|triple| match &triple.object {
            RdfObject::Literal(value) => !value.contains(forbidden),
            RdfObject::Resource(_) => true,
        }));
    }

    fn object_identity(object: &MemoryObject) -> (MemoryId, ObjectType) {
        match object {
            MemoryObject::Episode(object) => (object.id, object.object_type),
            MemoryObject::Observation(object) => (object.id, object.object_type),
            MemoryObject::Entity(object) => (object.id, object.object_type),
            MemoryObject::MemoryThread(object) => (object.id, object.object_type),
            MemoryObject::DerivedMemory(object) => (object.id, object.object_type),
            MemoryObject::MemoryLink(object) => (object.id, object.object_type),
        }
    }

    fn set_schema_version(object: &mut MemoryObject, schema_version: &str) {
        match object {
            MemoryObject::Episode(object) => object.schema_version = schema_version.to_owned(),
            MemoryObject::Observation(object) => object.schema_version = schema_version.to_owned(),
            MemoryObject::Entity(object) => object.schema_version = schema_version.to_owned(),
            MemoryObject::MemoryThread(object) => object.schema_version = schema_version.to_owned(),
            MemoryObject::DerivedMemory(object) => {
                object.schema_version = schema_version.to_owned()
            }
            MemoryObject::MemoryLink(object) => object.schema_version = schema_version.to_owned(),
        }
    }
}
