use super::*;

#[tokio::test]
async fn topic_only_applies_section_limits_and_preserves_query_text() {
    let fixtures = representative_fixtures();
    let graph = in_memory_graph_store();
    graph.upsert_objects(&fixtures.objects()).await.unwrap();
    graph.upsert_links(&fixtures.links()).await.unwrap();
    let vector = TemporaryVectorCandidateStore::open(8).await;
    let embedder = deterministic_embedder(8);
    for object in fixtures.objects() {
        if let Some(record) = memory_object_vector_record(&object) {
            let embedding = embedder.embed(&record.embedding_input()).await.unwrap();
            vector
                .upsert_vector_records(&[VectorRecordEmbedding::new(&record, &embedding)])
                .await
                .unwrap();
        }
    }
    let inputs = Arc::new(Mutex::new(Vec::new()));
    let memory = CharacterMemory::from_parts(
        Box::new(graph),
        Box::new(vector),
        Box::new(RecordingEmbedder(inputs.clone())),
    );
    for (candidates, roots, derived_limit) in [(6, 3, 1), (48, 12, 2)] {
        let mut context = RetrievalContext::new("  deterministic fixtures\nservice-free  ");
        context.candidate_limits.max_vector_candidates = candidates;
        context.candidate_limits.max_graph_roots = roots;
        context.section_limits.derived_memories = derived_limit;
        let limits = context.section_limits;
        let result = memory.retrieve(context).await.unwrap();
        let pack = &result.pack;
        for (count, limit) in [
            (pack.active_threads.len(), limits.active_threads),
            (pack.relevant_episodes.len(), limits.relevant_episodes),
            (pack.salient_observations.len(), limits.salient_observations),
            (pack.derived_memories.len(), limits.derived_memories),
            (pack.preferences.len(), limits.preferences),
            (pack.relationship_notes.len(), limits.relationship_notes),
            (pack.open_loops.len(), limits.open_loops),
            (pack.commitments.len(), limits.commitments),
            (pack.character_signals.len(), limits.character_signals),
        ] {
            assert!(count <= limit);
        }
    }
    assert_eq!(
        *inputs.lock().unwrap(),
        [
            "deterministic fixtures\nservice-free",
            "deterministic fixtures\nservice-free"
        ]
    );
}

#[tokio::test]
async fn cue_union_survives_winning_scores_but_excludes_a_root_cut_by_the_budget() {
    let (memory, _) = scene_memory().await;
    create_notion(&memory, 100, None).await;
    // Keep this participant selective so the test observes cue union, not ubiquity.
    write_episode(&memory, 6000, scene()).await;
    let id = MemoryId::from_u128(7000);
    let mut episode = EpisodeDraft::new("An astronomer arrived.");
    episode.id = Some(id);
    let observation_id = MemoryId::from_u128(7001);
    let mut observation = ObservationDraft::new(id, "An astronomer arrived.");
    observation.id = Some(observation_id);
    let mut present = scene();
    present.participants.push(keyed(100));
    memory
        .remember(
            RememberInput::new("An astronomer arrived.")
                .with_scene(present.clone())
                .with_episode(episode)
                .with_observation(observation),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    let mut context = RetrievalContext::new("astronomer")
        .with_scene(present)
        .with_trace();
    context.candidate_limits.max_vector_candidates = 1;
    context.object_type_defaults = vec![ObjectType::Observation];
    let object = MemoryObjectRef::new(ObjectType::Observation, observation_id);
    let combined = memory.retrieve(context.clone()).await.unwrap();
    assert_eq!(
        selected_cues(&combined, object),
        // Rulings 46 and 69: recency also brings the occasion's observation.
        &BTreeSet::from([CueKind::Topic, CueKind::Participant, CueKind::Recency]),
        "{:#?}",
        combined.trace
    );
    context.candidate_limits.max_graph_roots = 1;
    let limited = memory.retrieve(context).await.unwrap();
    assert_eq!(
        selected_cues(&limited, object),
        &BTreeSet::from([CueKind::Participant])
    );
    assert!(limited
        .trace
        .as_ref()
        .unwrap()
        .graph_expansions
        .iter()
        .any(|root| { root.root == object && root.outcome == GraphExpansionOutcome::RootLimit }));
    memory.close().await.unwrap();
}

#[tokio::test]
async fn descriptions_and_setting_words_recall_content_once_and_merge_with_topic() {
    let (memory, queries) = scene_memory().await;
    create_notion(&memory, 100, Some("Mira")).await;
    let mut past = scene();
    past.setting.words = Some("observatory".to_owned());
    past.participants = vec![
        SceneParticipant {
            description: Some("astronomer".to_owned()),
            ..Default::default()
        };
        2
    ];
    let episode_id = write_episode(&memory, 5000, past).await;
    let mut present = scene();
    present.participants = vec![
        SceneParticipant {
            description: Some("astronomer".to_owned()),
            ..Default::default()
        };
        2
    ];
    let mut context = RetrievalContext::default().with_scene(present).with_trace();
    context.candidate_limits.max_vector_candidates = 1;
    let description_only = memory.retrieve(context.clone()).await.unwrap();
    let described = MemoryObjectRef::new(ObjectType::Episode, episode_id);
    assert_eq!(
        selected_cues(&description_only, described),
        &BTreeSet::from([CueKind::Participant, CueKind::Recency])
    );
    assert_eq!(*queries.lock().unwrap(), ["astronomer\nastronomer"]);
    assert_eq!(description_only.pack.relevant_episodes[0].id, episode_id);
    assert!(description_only.pack.derived_memories.is_empty());
    assert_eq!(
        description_only.trace.as_ref().unwrap().scene_cue_searches[0].references,
        [
            SceneReference::ParticipantDescription { index: 0 },
            SceneReference::ParticipantDescription { index: 1 },
        ]
    );
    assert_eq!(
        description_only.trace.as_ref().unwrap().vector_candidates[0].surface,
        VectorSurface::SceneParticipants
    );
    assert!(description_only
        .scene_references
        .iter()
        .all(|reference| reference.resolution == SceneReferenceResolution::Reminder));

    let named_belief = MemoryObjectRef::new(ObjectType::DerivedMemory, MemoryId::from_u128(1100));
    let topic_only = memory
        .retrieve(RetrievalContext::new("astronomer").with_trace())
        .await
        .unwrap();
    assert_eq!(
        selected_cues(&topic_only, named_belief),
        &BTreeSet::from([CueKind::Topic])
    );
    context.topic = Some("astronomer".to_owned());
    context.scene.participants.truncate(1);
    context.scene.setting.words = Some("astronomer".to_owned());
    context.candidate_limits.max_vector_candidates = 48;
    queries.lock().unwrap().clear();
    let shared_text = memory.retrieve(context.clone()).await.unwrap();
    assert_eq!(*queries.lock().unwrap(), ["astronomer"]);
    assert_eq!(
        selected_cues(&shared_text, named_belief),
        &BTreeSet::from([CueKind::Topic])
    );
    assert!(selected_cues(&shared_text, described).contains(&CueKind::Participant));
    assert!(selected_cues(&shared_text, described).contains(&CueKind::Place));
    context.candidate_limits.max_vector_candidates = 1;
    context.scene.setting.words = Some("observatory".to_owned());
    let budgeted = memory.retrieve(context).await.unwrap();
    assert_eq!(budgeted.trace.unwrap().vector_candidates.len(), 1);
    let mut place = RetrievalContext::default();
    place.scene.setting.words = Some("observatory".to_owned());
    place.candidate_limits.max_vector_candidates = 1;
    let place_result = memory.retrieve(place.clone()).await.unwrap();
    assert!(place_result.trace.is_none());
    let traced_place = memory.retrieve(place.with_trace()).await.unwrap();
    assert_eq!(traced_place.pack, place_result.pack);
    assert_eq!(
        selected_cues(
            &traced_place,
            MemoryObjectRef::new(ObjectType::Episode, episode_id)
        ),
        &BTreeSet::from([CueKind::Place, CueKind::Recency])
    );
    assert_eq!(place_result.pack.relevant_episodes[0].id, episode_id);
    assert_eq!(
        recorded(&place_result, ObjectType::Episode, episode_id),
        [SourceScene::Recorded {
            episode_id,
            scene: place_result.pack.relevant_episodes[0].scene.clone()
        }]
    );
    assert!(matches!(
        &place_result.scene_references[0],
        SceneReferenceResult {
            reference: SceneReference::SettingWords,
            resolution: SceneReferenceResolution::Reminder,
            ..
        }
    ));
}

#[tokio::test]
async fn description_search_scores_are_shared_and_precede_occasion_selection() {
    let (memory, _) = scene_memory().await;
    let mut present = scene();
    present.setting.words = Some("observatory".to_owned());
    present.participants = vec![
        SceneParticipant {
            name: Some("astronomer".to_owned()),
            ..Default::default()
        },
        SceneParticipant {
            description: Some("navigator".to_owned()),
            ..Default::default()
        },
    ];
    let mut past = present.clone();
    past.time -= chrono::Duration::days(1);
    write_episode(&memory, 8000, past).await;
    let context = RetrievalContext::default()
        .with_scene(present.clone())
        .with_trace();
    let identical = memory.retrieve(context.clone()).await.unwrap();
    let searches = &identical.trace.as_ref().unwrap().scene_cue_searches;
    assert_eq!(searches.len(), 2);
    assert_eq!(
        (searches[0].cue_kind, searches[1].cue_kind),
        (crate::CueKind::Place, crate::CueKind::Participant)
    );
    assert!(searches.iter().all(|search| search.omitted_count == 0));
    assert_eq!(searches[0].references, [SceneReference::SettingWords]);
    assert_eq!(
        searches[1].references,
        [
            SceneReference::ParticipantName { index: 0 },
            SceneReference::ParticipantDescription { index: 1 },
        ]
    );
    assert!(searches
        .iter()
        .all(|search| (search.best_score.unwrap() - 1.0).abs() < 1e-6));

    let mut reworded = context.clone();
    reworded.scene.setting.words = Some("astronomer observatory".to_owned());
    reworded.scene.participants[0].name = Some("stargazer".to_owned());
    let reworded = memory.retrieve(reworded).await.unwrap();
    let searches = &reworded.trace.unwrap().scene_cue_searches;
    assert!((searches[0].best_score.unwrap() - 0.70886356).abs() < 1e-6);
    assert!((searches[1].best_score.unwrap() - 0.09950372).abs() < 1e-6);

    // Oppose IDs to time; the selected latest occasion is not the best match.
    let mut latest = present;
    latest.setting.words = Some("planetarium".to_owned());
    latest.participants[0].name = Some("stargazer".to_owned());
    write_episode(&memory, 9000, latest).await;
    let recent = memory.retrieve(context.clone()).await.unwrap();
    let trace = recent.trace.as_ref().unwrap();
    assert_eq!(trace.vector_candidates.len(), 1);
    assert_eq!(
        trace.vector_candidates[0].object.id,
        MemoryId::from_u128(9000)
    );
    assert!((trace.vector_candidates[0].score - 0.09950372).abs() < 1e-6);
    assert!(trace
        .scene_cue_searches
        .iter()
        .all(|search| (search.best_score.unwrap() - 1.0).abs() < 1e-6));
    assert!(trace
        .scene_cue_searches
        .iter()
        .all(|search| search.omitted_count == 1));
    let mut untraced_context = context.clone();
    untraced_context.include_trace = false;
    let untraced = memory.retrieve(untraced_context).await.unwrap();
    let mut expected = recent;
    expected.trace = None;
    assert_eq!(untraced, expected);

    let (without_surfaces, _) = scene_memory().await;
    write_episode(&without_surfaces, 7000, scene()).await;
    let empty = without_surfaces.retrieve(context).await.unwrap();
    let searches = empty.trace.unwrap().scene_cue_searches;
    assert_eq!(searches.len(), 2);
    assert!(searches.iter().all(|search| search.best_score.is_none()));
}

#[tokio::test]
async fn time_only_scene_is_echoed_without_embedding_or_completeness_claim() {
    let (memory, queries) = scene_memory().await;
    let before = chrono::Utc::now();
    let context = RetrievalContext::default();
    assert!(context.scene.time >= before && context.scene.time <= chrono::Utc::now());
    let present = context.scene.clone();
    let outcome = memory.retrieve(context).await.unwrap();
    assert_eq!(outcome.scene, present);
    assert!(outcome.scene_references.is_empty());
    assert!(outcome.trace.is_none());
    assert!(outcome.memory_scenes.is_empty());
    assert_eq!(
        outcome.rationale.telemetry.vector_recall_completeness,
        VectorRecallCompleteness::NotRequested
    );
    assert_eq!(
        outcome.rationale.telemetry.returned_vector_candidate_count,
        0
    );
    assert!(queries.lock().unwrap().is_empty());
    let mut blank = RetrievalContext::new(" \n ");
    blank.scene.participants.push(SceneParticipant::default());
    blank.scene.setting.words = Some("\t".to_owned());
    let result = memory.retrieve(blank).await.unwrap();
    assert!(result.scene.participants.is_empty());
    assert!(queries.lock().unwrap().is_empty());
}
