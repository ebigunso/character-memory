use std::collections::BTreeSet;

use async_trait::async_trait;

use crate::api::types::*;
use crate::domain::*;
use crate::models::vector::EmbeddingInput;
use crate::ports::embedder::MemoryEmbedder;
use crate::test_support::{in_memory_graph_store, TemporaryVectorCandidateStore};
use crate::{CharacterMemory, CustomError};

struct CohortEmbedder;

#[async_trait]
impl MemoryEmbedder for CohortEmbedder {
    async fn embed(&self, input: &EmbeddingInput) -> Result<Vec<f32>, CustomError> {
        let (topic, scene, unknown): (f32, f32, f32) = if input.surface == VectorSurface::Query {
            match input.text.as_str() {
                "orchids" => (1.0, 0.0, 0.0),
                "unlived" => (0.0, 0.0, 1.0),
                "studio" => (0.0, 1.0, 0.0),
                text if text.starts_with("botanist") => (0.0, 1.0, 0.0),
                text => panic!("unexpected query {text}"),
            }
        } else if input.object_id == Some(MemoryId::from_u128(900)) {
            (0.8, 0.6, 0.0)
        } else if matches!(
            input.surface,
            VectorSurface::SceneSetting | VectorSurface::SceneParticipants
        ) || input.text == "Episode summary: Another ordinary day."
        {
            (0.1, 0.99, 0.0)
        } else if input.text.contains("Orchid shared") {
            (0.8, 0.6, 0.0)
        } else {
            let rank: f32 = input
                .text
                .strip_prefix("Episode summary: Orchid lesson ")
                .unwrap()
                .parse()
                .unwrap();
            (0.9 - rank * 0.3 / 7.0, 0.0, 0.0)
        };
        Ok(vec![
            topic,
            scene,
            (1.0 - topic * topic - scene * scene - unknown * unknown)
                .max(0.0)
                .sqrt(),
            unknown,
        ])
    }

    async fn embed_batch(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>, CustomError> {
        let mut result = Vec::new();
        for input in inputs {
            result.push(self.embed(input).await?);
        }
        Ok(result)
    }
}

fn context() -> RetrievalContext {
    let mut context = RetrievalContext::new("orchids").with_trace();
    context.scene = Scene::at("2026-09-21T00:00:00Z".parse().unwrap());
    context.scene.setting.words = Some("studio".to_owned());
    context.scene.participants.push(SceneParticipant {
        description: Some("botanist".to_owned()),
        ..Default::default()
    });
    context.object_type_defaults = vec![ObjectType::Episode];
    context.graph_limits.max_depth = 0;
    context
}

fn episode(id: u128, summary: String, scene: Scene) -> [MemoryCandidate; 2] {
    let mut draft = EpisodeDraft::new(summary);
    draft.id = Some(MemoryId::from_u128(id));
    draft.created_at = Some(scene.time);
    draft.scene = Some(scene);
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    [
        MemoryCandidate::Episode(EpisodeCandidate::new(
            draft,
            CandidateProvenance::caller("turn pressure"),
        )),
        MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
            MemoryObjectRef::new(ObjectType::Episode, MemoryId::from_u128(id)),
            CandidateProvenance::caller("turn pressure"),
        )),
    ]
}

async fn cohort_memory() -> CharacterMemory {
    let memory = CharacterMemory::from_parts(
        Box::new(in_memory_graph_store()),
        Box::new(TemporaryVectorCandidateStore::open(4).await),
        Box::new(CohortEmbedder),
    );
    let mut person = EntityDraft::new();
    person.id = Some(MemoryId::from_u128(7));
    person.created_at = Some(context().scene.time);
    person.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    memory
        .commit(
            RememberWritePlan::new().with_candidate(MemoryCandidate::Entity(EntityCandidate::new(
                person,
                CandidateProvenance::caller("participant"),
            ))),
            CommitOptions::default(),
        )
        .await
        .unwrap();
    let mut plan = RememberWritePlan::new();
    for index in 0..48 {
        let mut scene = context().scene;
        scene.time += chrono::Duration::days(index as i64);
        scene.participants[0].key = Some(MemoryId::from_u128(7));
        for candidate in episode(1047 - index, "Another ordinary day.".to_owned(), scene) {
            plan = plan.with_candidate(candidate);
        }
    }
    for index in 0..8 {
        for candidate in episode(
            2000 + index,
            format!("Orchid lesson {index}"),
            Scene::at(context().scene.time),
        ) {
            plan = plan.with_candidate(candidate);
        }
    }
    memory.commit(plan, CommitOptions::default()).await.unwrap();
    memory
}

fn topic_count(ids: impl Iterator<Item = MemoryId>) -> usize {
    ids.filter(|id| (2000..2008).contains(&id.as_u128()))
        .count()
}

#[tokio::test]
async fn shared_scene_cohort_keeps_the_latest_occasion_and_score_fills_the_pack() {
    let memory = cohort_memory().await;
    for descriptions in [1, 5] {
        let mut query = context();
        query.scene.time += chrono::Duration::days(48);
        query.scene.participants = (0..descriptions)
            .map(|index| SceneParticipant {
                description: Some(format!("botanist {index}")),
                ..Default::default()
            })
            .collect();
        let result = memory.retrieve(query).await.unwrap();
        let trace = result.trace.as_ref().unwrap();
        let counts = (
            topic_count(trace.vector_candidates.iter().map(|row| row.object.id)),
            topic_count(
                trace
                    .graph_expansions
                    .iter()
                    .filter(|row| row.outcome == GraphExpansionOutcome::Expanded)
                    .map(|row| row.root.id),
            ),
            topic_count(
                result
                    .pack
                    .relevant_episodes
                    .iter()
                    .map(|episode| episode.id),
            ),
        );
        assert_eq!(counts, (8, 8, 7), "descriptions={descriptions}");
        // Admission changes, but output still follows the original final scores.
        assert_eq!(
            result
                .pack
                .relevant_episodes
                .iter()
                .map(|episode| episode.id.as_u128())
                .collect::<Vec<_>>(),
            [1000, 2000, 2001, 2002, 2003, 2004, 2005, 2006]
        );
        assert!(trace.floor_admissions.is_empty());
        assert_eq!(trace.scene_cue_omitted_counts[&CueKind::Participant], 47);
        assert_eq!(trace.scene_cue_omitted_counts[&CueKind::Place], 47);
    }
    memory.close().await.unwrap();
}

#[tokio::test]
async fn shared_scene_keeps_latest_keyed_occasion_with_lived_and_unlived_topics() {
    let mut memory = cohort_memory().await;
    for topic in ["orchids", "unlived"] {
        let mut query = context();
        query.scene.time += chrono::Duration::days(48);
        query.topic = Some(topic.to_owned());
        query.scene.participants[0].key = Some(MemoryId::from_u128(7));
        query.graph_limits.max_depth = 1;
        let result = memory.retrieve(query).await.unwrap();
        let trace = result.trace.unwrap();
        assert!(trace
            .graph_expansions
            .iter()
            .any(|row| row.root.id == MemoryId::from_u128(7)
                && row.outcome == GraphExpansionOutcome::Expanded));
        assert_eq!(result.pack.relevant_episodes.len(), 8);
        if topic == "unlived" {
            assert!(result
                .pack
                .relevant_episodes
                .iter()
                .all(|episode| (1000..1048).contains(&episode.id.as_u128())));
        }
        assert!(
            result
                .pack
                .relevant_episodes
                .iter()
                .any(|episode| episode.id == MemoryId::from_u128(1000)),
            "latest participant occasion must reach the pack: {:?}",
            trace
                .section_assignments
                .iter()
                .find(|row| row.object.id == MemoryId::from_u128(1000))
        );
    }
    // More than one explicit occasion must retain selector order, not ID order.
    memory.memory_composition.selectivity_policy =
        crate::policy::RetrievalSelectivityPolicy::try_new_with_fanout_budgets(
            1.0,
            1.0,
            [(RelationType::Involves, ObjectType::Episode, 2, 2)],
        )
        .unwrap();
    let mut query = context();
    query.scene.time += chrono::Duration::days(48);
    query.scene.participants[0].key = Some(MemoryId::from_u128(7));
    query.graph_limits.max_depth = 1;
    query.section_limits.relevant_episodes = 1;
    let result = memory.retrieve(query).await.unwrap();
    assert_eq!(
        result.pack.relevant_episodes[0].id,
        MemoryId::from_u128(1000)
    );
    assert!(result
        .trace
        .unwrap()
        .section_assignments
        .iter()
        .any(|row| row.object.id == MemoryId::from_u128(1001)));
    memory.close().await.unwrap();
}

#[tokio::test]
async fn shared_scene_topic_only_keeps_original_bytes() {
    let memory = cohort_memory().await;
    let mut query = context();
    query.scene.participants.clear();
    query.scene.setting.words = None;
    let result = memory.retrieve(query.clone()).await.unwrap();
    query.cue_floors = RetrievalCueFloors {
        participant: 0,
        place: 0,
        activity: 0,
        topic: 0,
    };
    let zero_floors = memory.retrieve(query).await.unwrap();
    assert_eq!(
        serde_json::to_vec(&result).unwrap(),
        serde_json::to_vec(&zero_floors).unwrap()
    );
    assert_eq!(
        topic_count(
            result
                .pack
                .relevant_episodes
                .iter()
                .map(|episode| episode.id)
        ),
        8
    );
    assert!(result.trace.unwrap().floor_admissions.is_empty());
    memory.close().await.unwrap();
}

#[tokio::test]
async fn shared_scene_overlap_uses_one_slot_and_uncapped_turns_emit_no_admissions() {
    let memory = cohort_memory().await;
    let mut plan = RememberWritePlan::new();
    let mut latest = context().scene;
    latest.time += chrono::Duration::days(49);
    for candidate in episode(900, "Orchid shared".to_owned(), latest.clone()) {
        plan = plan.with_candidate(candidate);
    }
    memory.commit(plan, CommitOptions::default()).await.unwrap();
    let mut query = context();
    query.scene.time = latest.time;
    query.candidate_limits.max_vector_candidates = 64;
    let capped = memory.retrieve(query.clone()).await.unwrap();
    let trace = capped.trace.unwrap();
    assert_eq!(
        trace
            .vector_candidates
            .iter()
            .filter(|row| row.object.id == MemoryId::from_u128(900))
            .count(),
        1
    );
    assert_eq!(
        trace
            .graph_expansions
            .iter()
            .filter(|row| row.root.id == MemoryId::from_u128(900)
                && row.outcome == GraphExpansionOutcome::Expanded)
            .count(),
        1
    );
    assert_eq!(
        capped
            .pack
            .relevant_episodes
            .iter()
            .filter(|episode| episode.id == MemoryId::from_u128(900))
            .count(),
        1
    );
    let shared = trace
        .section_assignments
        .iter()
        .find(|row| row.object.id == MemoryId::from_u128(900))
        .unwrap();
    assert_eq!(
        shared.cue_kinds,
        BTreeSet::from([CueKind::Participant, CueKind::Place, CueKind::Topic])
    );
    query.candidate_limits.max_graph_roots = 64;
    query.section_limits.relevant_episodes = 64;
    let uncapped = memory.retrieve(query).await.unwrap();
    assert_eq!(uncapped.pack.relevant_episodes.len(), 57);
    assert!(uncapped.trace.unwrap().floor_admissions.is_empty());
    memory.close().await.unwrap();
}
