pub(crate) const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

pub(crate) const CLASS_EPISODE: &str = "urn:cmem:vocab:Episode";
pub(crate) const CLASS_OBSERVATION: &str = "urn:cmem:vocab:Observation";
pub(crate) const CLASS_ENTITY: &str = "urn:cmem:vocab:Entity";
pub(crate) const CLASS_MEMORY_THREAD: &str = "urn:cmem:vocab:MemoryThread";
pub(crate) const CLASS_DERIVED_MEMORY: &str = "urn:cmem:vocab:DerivedMemory";
pub(crate) const CLASS_MEMORY_LINK: &str = "urn:cmem:vocab:MemoryLink";

pub(crate) const OBJECT_ID: &str = "urn:cmem:vocab:objectId";
pub(crate) const OBJECT_TYPE: &str = "urn:cmem:vocab:objectType";
pub(crate) const GRAPH_URI: &str = "urn:cmem:vocab:graphUri";
pub(crate) const SCHEMA_VERSION: &str = "urn:cmem:vocab:schemaVersion";
pub(crate) const CREATED_AT: &str = "urn:cmem:vocab:createdAt";
pub(crate) const UPDATED_AT: &str = "urn:cmem:vocab:updatedAt";

pub(crate) const MODALITY: &str = "urn:cmem:vocab:modality";
pub(crate) const SOURCE_CONVERSATION_ID: &str = "urn:cmem:vocab:sourceConversationId";
pub(crate) const STARTED_AT: &str = "urn:cmem:vocab:startedAt";
pub(crate) const ENDED_AT: &str = "urn:cmem:vocab:endedAt";
pub(crate) const PARTICIPANT_ENTITY: &str = "urn:cmem:vocab:participantEntity";
pub(crate) const SUMMARY: &str = "urn:cmem:vocab:summary";
pub(crate) const RAW_REF: &str = "urn:cmem:vocab:rawRef";
pub(crate) const SALIENCE_SCORE: &str = "urn:cmem:vocab:salienceScore";
pub(crate) const RETENTION_STATE: &str = "urn:cmem:vocab:retentionState";

pub(crate) const EPISODE: &str = "urn:cmem:vocab:episode";
pub(crate) const SPEAKER_ENTITY: &str = "urn:cmem:vocab:speakerEntity";
pub(crate) const OBSERVED_AT: &str = "urn:cmem:vocab:observedAt";
pub(crate) const TEXT: &str = "urn:cmem:vocab:text";

pub(crate) const ENTITY_TYPE: &str = "urn:cmem:vocab:entityType";
pub(crate) const NAME: &str = "urn:cmem:vocab:name";
pub(crate) const ALIAS: &str = "urn:cmem:vocab:alias";
pub(crate) const CANONICAL_KEY: &str = "urn:cmem:vocab:canonicalKey";

pub(crate) const TITLE: &str = "urn:cmem:vocab:title";
pub(crate) const THREAD_STATUS: &str = "urn:cmem:vocab:threadStatus";
pub(crate) const LAST_TOUCHED_AT: &str = "urn:cmem:vocab:lastTouchedAt";

pub(crate) const DERIVED_TYPE: &str = "urn:cmem:vocab:derivedType";
pub(crate) const DERIVED_FROM_EPISODE: &str = "urn:cmem:vocab:derivedFromEpisode";
pub(crate) const DERIVED_FROM_OBSERVATION: &str = "urn:cmem:vocab:derivedFromObservation";
pub(crate) const PART_OF_THREAD: &str = "urn:cmem:vocab:partOfThread";
pub(crate) const ABOUT_ENTITY: &str = "urn:cmem:vocab:aboutEntity";
pub(crate) const CONFIDENCE: &str = "urn:cmem:vocab:confidence";
pub(crate) const STABILITY: &str = "urn:cmem:vocab:stability";
pub(crate) const IS_CURRENT: &str = "urn:cmem:vocab:isCurrent";
pub(crate) const SUPERSEDES: &str = "urn:cmem:vocab:supersedes";

pub(crate) const FROM: &str = "urn:cmem:vocab:from";
pub(crate) const FROM_TYPE: &str = "urn:cmem:vocab:fromType";
pub(crate) const TO: &str = "urn:cmem:vocab:to";
pub(crate) const TO_TYPE: &str = "urn:cmem:vocab:toType";
pub(crate) const RELATION: &str = "urn:cmem:vocab:relation";
pub(crate) const RATIONALE: &str = "urn:cmem:vocab:rationale";
pub(crate) fn relation_predicate(name: &str) -> String {
    format!("urn:cmem:relation:{name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocabulary_uris_pin_graph_selection_and_lifecycle_contract() {
        assert_eq!(RDF_TYPE, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        assert_eq!(CLASS_DERIVED_MEMORY, "urn:cmem:vocab:DerivedMemory");
        assert_eq!(CLASS_MEMORY_LINK, "urn:cmem:vocab:MemoryLink");
        assert_eq!(OBJECT_ID, "urn:cmem:vocab:objectId");
        assert_eq!(OBJECT_TYPE, "urn:cmem:vocab:objectType");
        assert_eq!(GRAPH_URI, "urn:cmem:vocab:graphUri");
        assert_eq!(RETENTION_STATE, "urn:cmem:vocab:retentionState");
        assert_eq!(THREAD_STATUS, "urn:cmem:vocab:threadStatus");
        assert_eq!(DERIVED_FROM_EPISODE, "urn:cmem:vocab:derivedFromEpisode");
        assert_eq!(
            DERIVED_FROM_OBSERVATION,
            "urn:cmem:vocab:derivedFromObservation"
        );
        assert_eq!(PART_OF_THREAD, "urn:cmem:vocab:partOfThread");
        assert_eq!(ABOUT_ENTITY, "urn:cmem:vocab:aboutEntity");
        assert_eq!(IS_CURRENT, "urn:cmem:vocab:isCurrent");
        assert_eq!(SUPERSEDES, "urn:cmem:vocab:supersedes");
        assert_eq!(
            relation_predicate("part_of_thread"),
            "urn:cmem:relation:part_of_thread"
        );
    }
}
