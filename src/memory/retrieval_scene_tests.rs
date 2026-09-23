use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::api::types::*;
use crate::domain::*;
use crate::models::vector::{EmbeddingInput, VectorRecordEmbedding};
use crate::policy::embedding_surface::memory_object_vector_record;
use crate::ports::embedder::MemoryEmbedder;
use crate::ports::graph_authority::GraphAuthorityStore;
use crate::ports::vector_candidate::VectorCandidateStore;
use crate::test_support::{
    deterministic_embedder, in_memory_graph_store, representative_fixtures,
    TemporaryVectorCandidateStore,
};
use crate::{CharacterMemory, CustomError};

struct RecordingEmbedder(Arc<Mutex<Vec<String>>>);

#[async_trait]
impl MemoryEmbedder for RecordingEmbedder {
    async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
        assert!(
            !input.text.trim().is_empty(),
            "an absent topic must not be embedded"
        );
        self.0.lock().unwrap().push(input.text.clone());
        deterministic_embedder(8).embed(input).await
    }
    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        let mut vectors = Vec::new();
        for input in inputs {
            vectors.push(self.embed(input).await?);
        }
        Ok(vectors)
    }
}

struct CueEmbedder(Arc<Mutex<Vec<String>>>);

#[async_trait]
impl MemoryEmbedder for CueEmbedder {
    async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
        assert!(!input.text.trim().is_empty(), "absent topic was embedded");
        if input.surface == VectorSurface::Query {
            self.0.lock().unwrap().push(input.text.clone());
        }
        let text = input.text.to_lowercase();
        Ok(vec![
            if text.contains("astronomer") {
                1.0
            } else {
                0.0
            },
            if text.contains("observatory") {
                1.0
            } else {
                0.0
            },
            0.1,
        ])
    }
    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        let mut vectors = Vec::new();
        for input in inputs {
            vectors.push(self.embed(input).await?);
        }
        Ok(vectors)
    }
}

fn scene() -> Scene {
    Scene::at(
        (chrono::DateTime::parse_from_rfc3339("2026-09-20T10:00:00.123Z")
            .unwrap()
            .with_timezone(&chrono::Utc))
        .fixed_offset(),
    )
}

async fn scene_memory() -> (CharacterMemory, Arc<Mutex<Vec<String>>>) {
    let queries = Arc::new(Mutex::new(Vec::new()));
    let memory = crate::test_support::memory_with_embedder(3, CueEmbedder(queries.clone())).await;
    (memory, queries)
}

async fn create_notion(memory: &CharacterMemory, id: u128, name: Option<&str>) {
    let notion_id = MemoryId::from_u128(id);
    let mut notion = EntityDraft::new();
    notion.id = Some(notion_id);
    notion.created_at = Some(scene().time.to_utc());
    notion.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut plan = RememberWritePlan::new().with_candidate(MemoryCandidate::Entity(
        EntityCandidate::new(notion, CandidateProvenance::caller("notion")),
    ));
    if let Some(name) = name {
        let mut belief =
            DerivedMemoryDraft::new(DerivedType::Claim, "The astronomer is known by this name.");
        belief.id = Some(MemoryId::from_u128(id + 1000));
        belief.created_at = Some(scene().time.to_utc());
        belief.updated_at = belief.created_at;
        belief.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        belief.entity_ids = vec![notion_id];
        belief.given_by_application = true;
        belief.assertions.push(BeliefAssertion {
            subject: notion_id,
            predicate: BeliefPredicate::KnownAs {
                name: name.to_owned(),
            },
        });
        plan = plan
            .with_candidate(MemoryCandidate::DerivedMemory(DerivedMemoryCandidate::new(
                belief,
                CandidateProvenance::caller("name"),
            )))
            .with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
                MemoryObjectRef::new(ObjectType::DerivedMemory, MemoryId::from_u128(id + 1000)),
                CandidateProvenance::caller("content"),
            )));
    }
    memory.commit(plan, CommitOptions::default()).await.unwrap();
}

async fn write_episode(memory: &CharacterMemory, id: u128, scene: Scene) -> MemoryId {
    let mut episode = EpisodeDraft::new("A quiet meeting.");
    episode.id = Some(MemoryId::from_u128(id));
    let mut observation = ObservationDraft::new(episode.id.unwrap(), "An ordinary remark.");
    observation.id = Some(MemoryId::from_u128(id + 1));
    memory
        .remember(
            RememberInput::new("A quiet meeting.")
                .with_scene(scene)
                .with_episode(episode)
                .with_observation(observation),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    MemoryId::from_u128(id)
}

fn recorded(result: &RetrieveOutcome, object_type: ObjectType, id: MemoryId) -> &[SourceScene] {
    &result
        .memory_scenes
        .iter()
        .find(|entry| entry.memory == MemoryObjectRef::new(object_type, id))
        .unwrap_or_else(|| panic!("missing {object_type:?} {id}: {result:?}"))
        .sources
}

fn keyed(key: u128) -> SceneParticipant {
    SceneParticipant {
        key: Some(MemoryId::from_u128(key)),
        ..Default::default()
    }
}

fn selected_cues(result: &RetrieveOutcome, object: MemoryObjectRef) -> &BTreeSet<CueKind> {
    &result
        .trace
        .as_ref()
        .unwrap()
        .section_assignments
        .iter()
        .find(|row| {
            row.object == object && matches!(row.reason, SectionAssignmentReason::Selected { .. })
        })
        .unwrap()
        .cue_kinds
}

fn participant_observations(result: &RetrieveOutcome) -> Vec<MemoryId> {
    result
        .pack
        .salient_observations
        .iter()
        .filter(|observation| {
            selected_cues(
                result,
                MemoryObjectRef::new(ObjectType::Observation, observation.id),
            )
            .contains(&CueKind::Participant)
        })
        .map(|observation| observation.id)
        .collect()
}

mod activity;
mod descriptions;
mod evidence;
mod presence;
