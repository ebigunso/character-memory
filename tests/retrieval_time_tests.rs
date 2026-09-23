use chrono::{DateTime, Duration, Utc};
use serde_json::json;

use character_memory::*;
use test_support::id;

#[path = "support/mod.rs"]
pub mod test_support;

fn time_embedding(text: &str) -> Vec<f32> {
    let score: f32 = if matches!(
        text,
        "topic" | "what happened last Tuesday" | "what happened today"
    ) {
        1.0
    } else if text.contains("Strong 900") {
        0.85
    } else if text.contains("Strong ") {
        0.9 - text.rsplit(' ').next().unwrap().parse::<f32>().unwrap() * 0.001
    } else if text.contains("Weak") {
        0.3
    } else {
        0.0
    };
    vec![score, (1.0 - score * score).sqrt()]
}

fn time() -> DateTime<Utc> {
    "2026-09-21T18:00:00Z".parse().unwrap()
}

fn provenance() -> CandidateProvenance {
    CandidateProvenance::caller("recency witness")
}

async fn open() -> (CharacterMemory, tempfile::TempDir) {
    let root = tempfile::tempdir().unwrap();
    let memory = test_support::open_with_provider(
        test_support::persistent_settings(root.path()),
        test_support::unique_collection_name(),
        test_support::TestEmbeddingProvider::new(2, time_embedding),
    )
    .await
    .unwrap();
    (memory, root)
}

fn episode(
    mut plan: RememberWritePlan,
    n: u128,
    days: i64,
    salience: f32,
    person: bool,
    topic: bool,
) -> RememberWritePlan {
    let mut draft = EpisodeDraft::new(if topic {
        format!("Strong {}", n % 1000)
    } else {
        format!("Ordinary {n}")
    });
    draft.id = Some(id(n));
    // Deliberately unrelated to scene chronology.
    draft.created_at = Some(time() + Duration::days(n as i64));
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut scene = Scene::at((time() - Duration::days(days)).fixed_offset());
    if n == 100 {
        scene.time += Duration::milliseconds(125);
    }
    if person {
        scene.participants.push(test_support::keyed(7));
    }
    draft.scene = Some(scene);
    draft.salience_score = salience;
    plan = plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
        draft,
        provenance(),
    )));
    if person {
        plan = link(
            plan,
            ObjectType::Episode,
            n,
            ObjectType::Entity,
            7,
            RelationType::Involves,
        );
    }
    if topic {
        plan = indexed(plan, ObjectType::Episode, n);
    }
    plan
}

fn indexed(plan: RememberWritePlan, kind: ObjectType, n: u128) -> RememberWritePlan {
    plan.with_candidate(MemoryCandidate::VectorIndex(VectorIndexCandidate::new(
        MemoryObjectRef::new(kind, id(n)),
        provenance(),
    )))
}

fn link(
    plan: RememberWritePlan,
    from: ObjectType,
    from_id: u128,
    to: ObjectType,
    to_id: u128,
    relation: RelationType,
) -> RememberWritePlan {
    let mut draft = MemoryLinkDraft::new(from, id(from_id), relation, to, id(to_id));
    draft.id = Some(id(10_000_000 + from_id * 10_000 + to_id));
    draft.created_at = Some(time());
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    plan.with_candidate(MemoryCandidate::MemoryLink(MemoryLinkCandidate::new(
        draft,
        provenance(),
    )))
}

fn belief(plan: RememberWritePlan, n: u128, sources: &[u128], topic: bool) -> RememberWritePlan {
    let mut draft = DerivedMemoryDraft::new(
        DerivedType::Claim,
        if topic {
            "Weak interpretation"
        } else {
            "What the occasions meant"
        },
    );
    draft.id = Some(id(n));
    draft.created_at = Some(time());
    draft.updated_at = Some(time());
    draft.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    draft.salience_score = 0.0;
    draft.derived_from_episode_ids = sources.iter().copied().map(id).collect();
    let mut plan = plan.with_candidate(MemoryCandidate::DerivedMemory(
        DerivedMemoryCandidate::new(draft, provenance()),
    ));
    for source in sources {
        plan = link(
            plan,
            ObjectType::DerivedMemory,
            n,
            ObjectType::Episode,
            *source,
            RelationType::DerivedFrom,
        );
    }
    if topic {
        indexed(plan, ObjectType::DerivedMemory, n)
    } else {
        plan
    }
}

fn base() -> RememberWritePlan {
    let mut entity = EntityDraft::new();
    entity.id = Some(id(7));
    entity.created_at = Some(time());
    entity.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut plan = RememberWritePlan::new().with_candidate(MemoryCandidate::Entity(
        EntityCandidate::new(entity, provenance()),
    ));
    for (n, days, salience) in [
        (900, 0, 0.0),
        (100, 1, 1.0),
        (700, 31, 0.3),
        (600, 32, 0.0),
        (500, 33, 0.0),
        (50, -1, 0.0),
    ] {
        plan = episode(plan, n, days, salience, true, false);
    }
    // The named participant has several occasions but is not ubiquitous.
    for n in 800..820 {
        plan = episode(plan, n, 60, 0.0, false, false);
    }
    for index in 0..48 {
        plan = episode(plan, 2000 + index, 90 + index as i64, 0.0, false, true);
    }
    plan = belief(plan, 300, &[900, 700], false);
    let mut observation =
        ObservationDraft::new(id(800), "A separate unlinked observation: violet steam");
    observation.id = Some(id(400));
    observation.created_at = Some(time());
    observation.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    plan.with_candidate(MemoryCandidate::Observation(ObservationCandidate::new(
        observation,
        provenance(),
    )))
}

async fn commit(memory: &CharacterMemory, plan: RememberWritePlan) {
    let outcome = memory.commit(plan, CommitOptions::default()).await.unwrap();
    assert!(outcome.vector_indexing_failure.is_none(), "{outcome:?}");
}

fn query(topic: bool, person: bool) -> RetrievalContext {
    let mut scene = Scene::at((time()).fixed_offset());
    if person {
        scene.participants.push(test_support::keyed(7));
    }
    let mut context = RetrievalContext::default().with_scene(scene).with_trace();
    context.topic = topic.then(|| "topic".to_owned());
    context.graph_limits.timeout_ms = None;
    context
}

async fn retrieve(memory: &CharacterMemory, context: RetrievalContext) -> RetrieveOutcome {
    let result = memory.retrieve(context.clone()).await.unwrap();
    assert_eq!(result.scene, context.scene);
    assert!(result.activity.is_none());
    assert!(result.trace.is_some());
    result
}

fn episodes(result: &RetrieveOutcome) -> Vec<u128> {
    result
        .pack
        .relevant_episodes
        .iter()
        .map(|episode| episode.id.as_u128())
        .collect()
}

fn roots(result: &RetrieveOutcome) -> Vec<u128> {
    result
        .trace
        .as_ref()
        .unwrap()
        .graph_expansions
        .iter()
        .filter(|row| row.outcome == GraphExpansionOutcome::Expanded)
        .map(|row| row.root.id.as_u128())
        .collect()
}

fn assignment(result: &RetrieveOutcome, n: u128) -> &SectionAssignment {
    result
        .trace
        .as_ref()
        .unwrap()
        .section_assignments
        .iter()
        .find(|row| row.object.id == id(n))
        .unwrap()
}

fn scores(result: &RetrieveOutcome, n: u128) -> SectionScoreComponents {
    match assignment(result, n).reason {
        SectionAssignmentReason::Selected { scores }
        | SectionAssignmentReason::OmittedByLimit { scores, .. } => scores,
        ref reason => panic!("expected ranked object {n}: {reason:?}"),
    }
}

fn recency_episodes(result: &RetrieveOutcome) -> Vec<u128> {
    result
        .trace
        .as_ref()
        .unwrap()
        .section_assignments
        .iter()
        .filter(|row| {
            row.object.object_type == ObjectType::Episode
                && row.cue_kinds.contains(&CueKind::Recency)
        })
        .map(|row| row.object.id.as_u128())
        .collect()
}

fn room(cap: usize) -> ContinuitySectionLimits {
    ContinuitySectionLimits {
        active_threads: cap,
        relevant_episodes: cap,
        salient_observations: cap,
        derived_memories: cap,
        preferences: cap,
        relationship_notes: cap,
        open_loops: cap,
        commitments: cap,
        character_signals: cap,
    }
}

#[path = "retrieval_time_tests/anniversary.rs"]
mod anniversary;
#[path = "retrieval_time_tests/description.rs"]
mod description;
#[path = "retrieval_time_tests/range.rs"]
mod range;
#[path = "retrieval_time_tests/recency.rs"]
mod recency;

fn description_fixture(person: bool) -> RememberWritePlan {
    let mut plan = RememberWritePlan::new();
    if person {
        let mut entity = EntityDraft::new();
        entity.id = Some(id(7));
        entity.created_at = Some(time());
        entity.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
        plan = plan.with_candidate(MemoryCandidate::Entity(EntityCandidate::new(
            entity,
            provenance(),
        )));
    }
    let mut latest = EpisodeDraft::new("The described studio this evening");
    latest.id = Some(id(900));
    latest.created_at = Some(time());
    latest.schema_version = Some(DEFAULT_SCHEMA_VERSION.to_owned());
    let mut scene = Scene::at((time()).fixed_offset());
    scene.setting.words = Some("studio".to_owned());
    if person {
        scene.participants.push(test_support::keyed(7));
    }
    latest.scene = Some(scene);
    plan = plan.with_candidate(MemoryCandidate::Episode(EpisodeCandidate::new(
        latest,
        provenance(),
    )));
    plan = indexed(plan, ObjectType::Episode, 900);
    if person {
        plan = link(
            plan,
            ObjectType::Episode,
            900,
            ObjectType::Entity,
            7,
            RelationType::Involves,
        );
    }
    belief(
        episode(plan, 700, 40, 1.0, false, false),
        300,
        &[900, 700],
        false,
    )
}
