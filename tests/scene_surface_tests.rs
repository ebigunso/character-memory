use std::sync::{Arc, Mutex};
use test_support::id;
use test_support::scene_time as time;

use async_trait::async_trait;
use character_memory::{
    CharacterMemory, CommitOptions, EmbeddingError, EmbeddingProvider, EpisodeDraft,
    ForgetMemoryDraft, LifecycleTargetRef, ObjectType, RememberInput, RememberOptions,
    RememberPlanDefaults, RetrievalContext, Scene, SceneParticipant, Settings,
};
use character_memory::{CueKind, VectorRecallCompleteness, VectorSurface};
use std::collections::BTreeSet;

#[path = "support/mod.rs"]
pub mod test_support;

#[derive(Default)]
struct Calls {
    batches: Mutex<Vec<Vec<String>>>,
    queries: Mutex<Vec<String>>,
}
struct Provider(Arc<Calls>);

fn embedding(text: &str) -> Vec<f32> {
    if text == "zero" {
        vec![0.0; 3]
    } else if text == "studio" {
        vec![1.0, 0.0, 0.0]
    } else if text == "visitor" {
        vec![0.0, 1.0, 0.0]
    } else if text.ends_with("studio") {
        vec![0.8, 0.0, 0.6]
    } else {
        vec![0.0, 0.0, 1.0]
    }
}

#[async_trait]
impl EmbeddingProvider for Provider {
    fn vector_size(&self) -> usize {
        3
    }
    async fn generate_embedding<'a>(&self, text: &'a str) -> Result<Vec<f32>, EmbeddingError> {
        self.0.queries.lock().unwrap().push(text.to_owned());
        Ok(embedding(text))
    }
    async fn bulk_generate_embeddings<'a>(
        &self,
        texts: &'a [&'a str],
    ) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        self.0
            .batches
            .lock()
            .unwrap()
            .push(texts.iter().map(|s| (*s).to_owned()).collect());
        if texts.contains(&"failed batch") {
            return Ok(Vec::new());
        }
        Ok(texts.iter().map(|text| embedding(text)).collect())
    }
}

fn scene(setting: Option<&str>, participants: &[&str]) -> Scene {
    let mut scene = Scene::at((time()).fixed_offset());
    scene.setting.words = setting.map(str::to_owned);
    scene.participants = participants
        .iter()
        .map(|words| SceneParticipant {
            description: Some((*words).to_owned()),
            ..Default::default()
        })
        .collect();
    scene
}

async fn open() -> (CharacterMemory, Arc<Calls>, tempfile::TempDir) {
    let root = tempfile::tempdir().unwrap();
    let settings = test_support::persistent_settings(root.path())
        .build()
        .unwrap();
    let collection = test_support::unique_collection_name();
    let calls = Arc::new(Calls::default());
    let memory = CharacterMemory::new_with_embedding_provider(
        Settings::new(settings).unwrap(),
        collection,
        Box::new(Provider(calls.clone())),
    )
    .await
    .unwrap();
    (memory, calls, root)
}

async fn write(
    memory: &CharacterMemory,
    n: u128,
    summary: &str,
    scene: Scene,
    caller: bool,
) -> character_memory::RememberOutcome {
    let mut episode = EpisodeDraft::new(summary);
    episode.id = Some(id(n));
    episode.scene = Some(scene);
    let input = RememberInput::new(summary).with_episode(episode);
    if caller {
        let defaults = RememberPlanDefaults::fixed(summary, time());
        memory
            .commit(
                input.prepare_write_plan(&defaults),
                CommitOptions::default(),
            )
            .await
            .unwrap()
    } else {
        memory
            .remember(input, RememberOptions::default())
            .await
            .unwrap()
    }
}

fn query(topic: Option<&str>, scene: Scene, limit: usize) -> RetrievalContext {
    let mut context = RetrievalContext::default().with_scene(scene).with_trace();
    context.topic = topic.map(str::to_owned);
    context.object_type_defaults = vec![ObjectType::Episode];
    context.candidate_limits.max_vector_candidates = limit;
    context.graph_limits.max_depth = 0;
    context
}

#[tokio::test]
async fn scene_surfaces_preserve_content_batching_and_object_outcomes() {
    for caller in [false, true] {
        for words in [false, true] {
            let (memory, calls, root) = open().await;
            let recorded = if words {
                scene(Some("studio"), &["visitor"])
            } else {
                scene(None, &[])
            };
            let outcome = write(&memory, 1, "unrelated", recorded, caller).await;
            let batches = calls.batches.lock().unwrap().clone();
            assert_eq!(batches.len(), 1);
            assert_eq!(batches[0].len(), if words { 4 } else { 2 });
            assert_eq!(outcome.vector_indexed_object_ids.len(), 2);
            assert_eq!(
                outcome
                    .vector_indexed_object_ids
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>()
                    .len(),
                2
            );
            memory.close().await.unwrap();
            root.close().unwrap();
        }
    }
    let (memory, _, root) = open().await;
    let outcome = write(
        &memory,
        1,
        "unrelated",
        scene(Some("failed batch"), &["visitor"]),
        true,
    )
    .await;
    assert!(outcome.vector_indexed_object_ids.is_empty());
    let failure = outcome.vector_indexing_failure.unwrap();
    assert_eq!(failure.unindexed_objects.len(), 2);
    assert_eq!(
        failure
            .unindexed_objects
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        2
    );
    assert_eq!(
        failure.cause,
        character_memory::VectorIndexingCause::CardinalityMismatch {
            expected: 4,
            actual: 0
        }
    );
    memory.close().await.unwrap();
    root.close().unwrap();
}

#[tokio::test]
async fn embedded_surface_scopes_cover_nearest_zero_norm_counts_and_deletion() {
    let (memory, calls, root) = open().await;
    write(
        &memory,
        1,
        "unrelated",
        scene(Some("studio"), &["visitor"]),
        false,
    )
    .await;
    write(&memory, 2, "studio", scene(None, &[]), false).await;
    write(
        &memory,
        3,
        "another",
        scene(Some("visitor"), &["studio"]),
        true,
    )
    .await;
    for (topic, current, winner, surface, count) in [
        (
            Some("studio"),
            scene(None, &[]),
            2,
            VectorSurface::Summary,
            3,
        ),
        (
            None,
            scene(Some("studio"), &[]),
            1,
            VectorSurface::SceneSetting,
            2,
        ),
        (
            None,
            scene(None, &["studio"]),
            1,
            VectorSurface::SceneParticipants,
            2,
        ),
    ] {
        let result = memory.retrieve(query(topic, current, 1)).await.unwrap();
        assert_eq!(result.pack.relevant_episodes[0].id, id(winner));
        let trace = result.trace.unwrap();
        assert_eq!(trace.vector_candidates.len(), 1);
        assert_eq!(trace.vector_candidates[0].surface, surface);
        assert_eq!(
            result.rationale.telemetry.vector_recall_completeness,
            VectorRecallCompleteness::Exhaustive { scanned: count }
        );
    }
    calls.queries.lock().unwrap().clear();
    let mut both = query(Some("studio"), scene(Some("studio"), &["studio"]), 3);
    both.cue_floors.participant = 2;
    both.cue_floors.place = 2;
    let result = memory.retrieve(both).await.unwrap();
    assert_eq!(*calls.queries.lock().unwrap(), ["studio"]);
    let trace = result.trace.unwrap();
    assert_eq!(
        trace
            .vector_candidates
            .iter()
            .map(|c| (c.object.id, c.surface))
            .collect::<Vec<_>>(),
        [
            (id(1), VectorSurface::SceneSetting),
            (id(3), VectorSurface::SceneParticipants),
            (id(2), VectorSurface::Summary)
        ]
    );
    assert_eq!(
        trace
            .section_assignments
            .iter()
            .find(|row| row.object.id == id(2))
            .unwrap()
            .cue_kinds,
        BTreeSet::from([CueKind::Topic, CueKind::Recency])
    );
    calls.queries.lock().unwrap().clear();
    let result = memory
        .retrieve(query(
            None,
            scene(None, &["one", "two", "three", "four", "five", "six"]),
            3,
        ))
        .await
        .unwrap();
    let queries = calls.queries.lock().unwrap().clone();
    assert_eq!(queries.len(), 1);
    for description in ["one", "two", "three", "four", "five", "six"] {
        assert!(queries[0].contains(description));
    }
    assert_eq!(result.scene_references.len(), 6);
    assert!(result
        .trace
        .unwrap()
        .vector_candidates
        .iter()
        .all(|row| row.surface == VectorSurface::SceneParticipants));

    for forgotten in [false, true] {
        if forgotten {
            memory
                .forget(ForgetMemoryDraft::suppress(
                    LifecycleTargetRef::episode(id(1)),
                    "forget every episode surface",
                ))
                .await
                .unwrap();
        }
        for (topic, current, surface, count) in [
            (Some("zero"), scene(None, &[]), VectorSurface::Summary, 3),
            (
                None,
                scene(Some("zero"), &[]),
                VectorSurface::SceneSetting,
                2,
            ),
            (
                None,
                scene(None, &["zero"]),
                VectorSurface::SceneParticipants,
                2,
            ),
        ] {
            let mut context = query(topic, current, 20);
            context.lifecycle_policy.include_suppressed = true;
            context.cue_floors.participant = 2;
            context.cue_floors.place = 2;
            let result = memory.retrieve(context).await.unwrap();
            let trace = result.trace.unwrap();
            assert_eq!(
                trace.vector_candidates.len(),
                count - usize::from(forgotten)
            );
            assert!(trace
                .vector_candidates
                .iter()
                .all(|row| row.surface == surface
                    && row.score == 0.0
                    && (!forgotten || row.object.id != id(1))));
            assert_eq!(
                result.rationale.telemetry.vector_recall_completeness,
                VectorRecallCompleteness::Exhaustive {
                    scanned: count - usize::from(forgotten)
                }
            );
        }
    }
    memory.close().await.unwrap();
    root.close().unwrap();
}
