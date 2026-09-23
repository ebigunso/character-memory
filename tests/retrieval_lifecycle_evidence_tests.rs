use character_memory::{
    CharacterMemory, CommitOptions, DerivedMemoryDraft, DerivedType, EpisodeDraft, MemoryLinkDraft,
    ObjectType, RelationType, RememberInput, RememberPlanDefaults, RetrievalContext, Scene,
};
use chrono::{DateTime, Utc};
use test_support::id;
#[path = "support/mod.rs"]
pub mod test_support;

fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-09-21T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}
async fn commit(memory: &CharacterMemory, input: RememberInput) {
    let defaults = RememberPlanDefaults::fixed(&input.content, at());
    memory
        .commit(
            input.prepare_write_plan(&defaults),
            CommitOptions::default(),
        )
        .await
        .unwrap();
}
async fn lifecycle_evidence_fixture(relation: RelationType) {
    let (memory, root) = test_support::try_setup_character_memory().await.unwrap();
    let mut episode = EpisodeDraft::new("source");
    episode.id = Some(id(101));
    episode.scene = Some(Scene::at((at()).fixed_offset()));
    episode.created_at = Some(at());
    let mut input = RememberInput::new("fixture").with_episode(episode);
    let mut successor = None;
    for (n, text) in [
        (200, "obscure north"),
        (201, "obscure south"),
        (300, "quasar hatch calibration"),
        (301, "resolution report"),
        (302, "old task"),
    ] {
        let mut draft = DerivedMemoryDraft::new(
            if n == 302 {
                if relation == RelationType::FulfillsCommitment {
                    DerivedType::Commitment
                } else {
                    DerivedType::OpenLoop
                }
            } else {
                DerivedType::Claim
            },
            text,
        )
        .with_source_episode(id(101));
        draft.id = Some(id(n));
        draft.created_at = Some(at());
        draft.updated_at = Some(at());
        if n == 301 && relation == RelationType::Supersedes {
            draft.supersedes.push(id(302));
            successor = Some(draft);
            continue;
        }
        input = input.with_derived_memory(draft);
    }
    commit(&memory, input).await;
    if let Some(successor) = successor {
        commit(
            &memory,
            RememberInput::new("superseding result").with_derived_memory(successor),
        )
        .await;
    }
    for (n, from, to, relation) in [
        (701, 300, 301, RelationType::AssociatedWith),
        (702, 300, 302, RelationType::AssociatedWith),
        (703, 301, 200, RelationType::AssociatedWith),
        (704, 301, 201, RelationType::AssociatedWith),
        (705, 302, 200, RelationType::AssociatedWith),
        (706, 302, 201, RelationType::AssociatedWith),
        (707, 301, 302, relation),
    ] {
        if relation == RelationType::Supersedes {
            continue;
        }
        let mut link = MemoryLinkDraft::new(
            ObjectType::DerivedMemory,
            id(from),
            relation,
            ObjectType::DerivedMemory,
            id(to),
        );
        link.id = Some(id(n));
        link.created_at = Some(at());
        memory.link(link).await.unwrap();
    }
    let mut outcomes = Vec::new();
    for trace in [false, true] {
        let mut request = RetrievalContext::new("quasar hatch calibration")
            .with_scene(Scene::at((at()).fixed_offset()));
        request.object_type_defaults = vec![ObjectType::DerivedMemory];
        request.candidate_limits.max_vector_candidates = 1;
        request.candidate_limits.max_graph_roots = 1;
        request.graph_limits.allowed_object_types = vec![ObjectType::DerivedMemory];
        request.graph_limits.max_depth = 2;
        request.graph_limits.max_nodes = 3;
        request.graph_limits.max_fanout_per_node = 2;
        request.include_trace = trace;
        request.lifecycle_policy.include_superseded = relation == RelationType::Supersedes;
        outcomes.push(memory.retrieve(request).await.unwrap());
    }
    let trace = outcomes[1].trace.take().unwrap();
    memory.close().await.unwrap();
    root.close().unwrap();
    assert_eq!(
        trace
            .graph_expansions
            .iter()
            .map(|entry| entry.relation_count)
            .sum::<usize>(),
        2,
        "{relation:?} evidence must not become traversal"
    );
    assert_eq!(outcomes[0], outcomes[1]);
}

#[tokio::test]
async fn resolution_evidence_respects_selected_traversal_links() {
    lifecycle_evidence_fixture(RelationType::Resolves).await;
}
#[tokio::test]
async fn fulfillment_evidence_respects_selected_traversal_links() {
    lifecycle_evidence_fixture(RelationType::FulfillsCommitment).await;
}
#[tokio::test]
async fn supersession_evidence_respects_selected_traversal_links() {
    lifecycle_evidence_fixture(RelationType::Supersedes).await;
}
