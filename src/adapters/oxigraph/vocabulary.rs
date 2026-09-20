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
pub(crate) const SCENE_TIME: &str = "urn:cmem:vocab:sceneTime";
pub(crate) const SCENE_PARTICIPANTS: &str = "urn:cmem:vocab:sceneParticipants";
pub(crate) const SETTING_KEY: &str = "urn:cmem:vocab:settingKey";
pub(crate) const SETTING_WORDS: &str = "urn:cmem:vocab:settingWords";
pub(crate) const SCENE_CUSTOM_VALUES: &str = "urn:cmem:vocab:sceneCustomValues";
pub(crate) const ENDED_AT: &str = "urn:cmem:vocab:endedAt";
pub(crate) const SUMMARY: &str = "urn:cmem:vocab:summary";
pub(crate) const RAW_REF: &str = "urn:cmem:vocab:rawRef";
pub(crate) const SALIENCE_SCORE: &str = "urn:cmem:vocab:salienceScore";
pub(crate) const RETENTION_STATE: &str = "urn:cmem:vocab:retentionState";

pub(crate) const EPISODE: &str = "urn:cmem:vocab:episode";
pub(crate) const SPEAKER_ENTITY: &str = "urn:cmem:vocab:speakerEntity";
pub(crate) const OBSERVED_AT: &str = "urn:cmem:vocab:observedAt";
pub(crate) const TEXT: &str = "urn:cmem:vocab:text";

pub(crate) const CANONICAL_KEY: &str = "urn:cmem:vocab:canonicalKey";

pub(crate) const TITLE: &str = "urn:cmem:vocab:title";
pub(crate) const THREAD_STATUS: &str = "urn:cmem:vocab:threadStatus";
pub(crate) const LAST_TOUCHED_AT: &str = "urn:cmem:vocab:lastTouchedAt";

pub(crate) const DERIVED_TYPE: &str = "urn:cmem:vocab:derivedType";
pub(crate) const DERIVED_FROM_EPISODE: &str = "urn:cmem:vocab:derivedFromEpisode";
pub(crate) const DERIVED_FROM_OBSERVATION: &str = "urn:cmem:vocab:derivedFromObservation";
pub(crate) const PART_OF_THREAD: &str = "urn:cmem:vocab:partOfThread";
pub(crate) const ABOUT_ENTITY: &str = "urn:cmem:vocab:aboutEntity";
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

pub(crate) const ASSERTION: &str = "urn:cmem:vocab:assertion";
pub(crate) const ASSERTION_SUBJECT: &str = "urn:cmem:vocab:assertionSubject";
pub(crate) const ASSERTION_PREDICATE: &str = "urn:cmem:vocab:assertionPredicate";
pub(crate) const ASSERTION_NAME: &str = "urn:cmem:vocab:assertionName";
pub(crate) const NORMALIZED_NAME: &str = "urn:cmem:vocab:normalizedName";
pub(crate) const GIVEN_BY_APPLICATION: &str = "urn:cmem:vocab:givenByApplication";
