use super::*;

#[tokio::test]
async fn thread_activity_reads_native_members_and_reports_found_after_filtering() {
    let (memory, queries) = scene_memory().await;
    let thread_id = MemoryId::from_u128(9000);
    let member_id = MemoryId::from_u128(9001);
    let older_member_id = MemoryId::from_u128(8999);
    let mut thread = MemoryThreadDraft::new("The telescope repair", "A completed piece of work");
    thread.id = Some(thread_id);
    thread.status = ThreadStatus::Resolved;
    let mut member = DerivedMemoryDraft::new(DerivedType::Claim, "The lens needs cleaning.");
    member.id = Some(member_id);
    member.created_at = Some(scene().time.to_utc());
    member.thread_ids = vec![thread_id];
    let mut older_member = DerivedMemoryDraft::new(DerivedType::Claim, "The lens was installed.");
    older_member.id = Some(older_member_id);
    older_member.created_at = Some(scene().time.to_utc() - chrono::Duration::days(1));
    older_member.thread_ids = vec![thread_id];
    memory
        .remember(
            RememberInput::new("We inspected the lens.")
                .with_scene(scene())
                .with_memory_thread(thread)
                .with_derived_memory(member)
                .with_derived_memory(older_member),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    let activity = ActivityRef::Thread(thread_id);
    let mut context = RetrievalContext::default()
        .with_activity(activity)
        .with_trace();
    context.candidate_limits.max_graph_roots = 2;
    let found = memory.retrieve(context.clone()).await.unwrap();
    let wrong_kind = ActivityRef::OpenLoop(member_id);
    let unknown = memory
        .retrieve(RetrievalContext::default().with_activity(wrong_kind))
        .await
        .unwrap();
    assert_eq!(
        unknown.activity,
        Some(ActivityResult {
            activity: wrong_kind,
            resolution: ActivityResolution::Unknown
        })
    );
    assert_eq!(unknown.pack.relevant_episodes.len(), 1);
    // Rulings 46 and 69: recency reports both the episode and its observation's scene.
    assert_eq!(unknown.memory_scenes.len(), 2);
    assert_eq!(
        found.activity,
        Some(ActivityResult {
            activity,
            resolution: ActivityResolution::Found
        })
    );
    assert_eq!(found.pack.derived_memories[0].memory.id, member_id);
    assert_eq!(
        selected_cues(
            &found,
            MemoryObjectRef::new(ObjectType::DerivedMemory, member_id)
        ),
        &BTreeSet::from([CueKind::Activity])
    );
    assert!(found.pack.active_threads.is_empty());
    assert!(
        found
            .trace
            .as_ref()
            .unwrap()
            .graph_relations
            .iter()
            .all(|link| link.relation == RelationType::ObservedIn),
        "native affiliation requires no link"
    );
    assert!(queries.lock().unwrap().is_empty());
    let mut mixed = context.clone();
    mixed.topic = Some("lens".to_owned());
    // Leave room for the topic head as well as the thread and its newest member.
    mixed.candidate_limits.max_graph_roots = 3;
    mixed.section_limits.derived_memories = 0;
    let mixed = memory.retrieve(mixed).await.unwrap();
    let trace = mixed.trace.unwrap();
    let member_ref = MemoryObjectRef::new(ObjectType::DerivedMemory, member_id);
    let vector_score = trace
        .vector_candidates
        .iter()
        .find(|candidate| candidate.object == member_ref)
        .unwrap()
        .score;
    assert!(trace.section_assignments.iter().any(|row| {
        row.object == member_ref
            && row.cue_kinds == BTreeSet::from([CueKind::Topic, CueKind::Activity])
            && matches!(row.reason, SectionAssignmentReason::OmittedByLimit { .. })
    }));
    assert!(trace.stale_candidate_omissions.iter().any(|omission| {
        omission.candidate == member_ref && omission.vector_score == Some(vector_score)
    }));
    assert_eq!(*queries.lock().unwrap(), ["lens"]);
    queries.lock().unwrap().clear();
    let mut limited = context.clone();
    limited.candidate_limits.max_graph_roots = 1;
    let bounded = memory.retrieve(limited).await.unwrap();
    assert_eq!(bounded.activity, found.activity);
    assert!(bounded.memory_scenes.is_empty());
    assert!(bounded.trace.unwrap().graph_expansions.iter().any(|root| {
        root.root.id == member_id && root.outcome == GraphExpansionOutcome::RootLimit
    }));
    for id in [member_id, older_member_id] {
        memory
            .forget(ForgetMemoryDraft::suppress(
                LifecycleTargetRef::derived_memory(id),
                "No longer useful",
            ))
            .await
            .unwrap();
    }
    let mut untraced = context;
    untraced.include_trace = false;
    let empty = memory.retrieve(untraced.clone()).await.unwrap();
    assert_eq!(empty.activity, found.activity);
    assert_eq!(empty.pack.relevant_episodes.len(), 1);
    // Rulings 46 and 69: the observation's scene remains after the beliefs are suppressed.
    assert_eq!(empty.memory_scenes.len(), 2);
    assert!(empty.trace.is_none());
    untraced.lifecycle_policy.include_suppressed = true;
    assert_eq!(
        memory
            .retrieve(untraced)
            .await
            .unwrap()
            .pack
            .derived_memories[0]
            .memory
            .id,
        member_id
    );
    assert!(
        queries.lock().unwrap().is_empty(),
        "an activity needs no content search"
    );
    memory.close().await.unwrap();
}

#[tokio::test]
async fn open_loop_activity_reads_sources_and_threads_and_respects_its_own_lifecycle() {
    let (memory, queries) = scene_memory().await;
    let episode_id = write_episode(&memory, 5000, scene()).await;
    write_episode(&memory, 6000, scene()).await;
    let observation_id = MemoryId::from_u128(6001);
    // Identity includes the kind: these two valid activities deliberately share a UUID.
    let id = MemoryId::from_u128(9200);
    let mut thread = MemoryThreadDraft::new("The unfinished repair", "Work in progress");
    thread.id = Some(id);
    let mut open_loop = DerivedMemoryDraft::new(DerivedType::OpenLoop, "Finish the repair.");
    open_loop.id = Some(id);
    open_loop.thread_ids = vec![id];
    open_loop.derived_from_episode_ids = vec![episode_id];
    open_loop.derived_from_observation_ids = vec![observation_id];
    memory
        .remember(
            RememberInput::new("The repair remains open.")
                .with_scene(scene())
                .with_memory_thread(thread)
                .with_derived_memory(open_loop.clone()),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    let activity = ActivityRef::OpenLoop(id);
    let context = RetrievalContext::default()
        .with_activity(activity)
        .with_trace();
    let encoded = serde_json::to_value(&context).unwrap();
    assert_eq!(
        encoded["activity"],
        serde_json::json!({"kind": "open_loop", "id": id})
    );
    let context: RetrievalContext = serde_json::from_value(encoded).unwrap();
    let found = memory.retrieve(context.clone()).await.unwrap();
    assert_eq!(
        serde_json::to_value(&found).unwrap()["activity"],
        serde_json::json!({
            "activity": {"kind": "open_loop", "id": id}, "resolution": "found"
        })
    );
    assert_eq!(
        found.activity,
        Some(ActivityResult {
            activity,
            resolution: ActivityResolution::Found
        })
    );
    for object in [
        MemoryObjectRef::new(ObjectType::Episode, episode_id),
        MemoryObjectRef::new(ObjectType::Observation, observation_id),
        MemoryObjectRef::new(ObjectType::MemoryThread, id),
        MemoryObjectRef::new(ObjectType::DerivedMemory, id),
    ] {
        assert_eq!(
            selected_cues(&found, object),
            // Rulings 46 and 69: recency also reaches the observation on this occasion.
            &if matches!(
                object.object_type,
                ObjectType::Episode | ObjectType::Observation
            ) {
                BTreeSet::from([CueKind::Activity, CueKind::Recency])
            } else {
                BTreeSet::from([CueKind::Activity])
            }
        );
    }
    let mut thread_context = context.clone();
    assert_eq!(
        found
            .trace
            .as_ref()
            .unwrap()
            .graph_expansions
            .iter()
            .filter(|root| root.source != GraphRootSource::Recency)
            .map(|root| root.root)
            .collect::<Vec<_>>(),
        vec![
            MemoryObjectRef::new(ObjectType::DerivedMemory, id),
            MemoryObjectRef::new(ObjectType::Episode, episode_id),
            MemoryObjectRef::new(ObjectType::Observation, observation_id),
            MemoryObjectRef::new(ObjectType::MemoryThread, id),
        ]
    );
    thread_context.activity = Some(ActivityRef::Thread(id));
    assert_eq!(
        memory.retrieve(thread_context).await.unwrap().activity,
        Some(ActivityResult {
            activity: ActivityRef::Thread(id),
            resolution: ActivityResolution::Found
        })
    );
    for activity in [
        ActivityRef::Thread(episode_id),
        ActivityRef::OpenLoop(episode_id),
        ActivityRef::OpenLoop(MemoryId::from_u128(9999)),
    ] {
        let unknown = memory
            .retrieve(
                RetrievalContext::default()
                    .with_activity(activity)
                    .with_trace(),
            )
            .await
            .unwrap();
        assert_eq!(
            unknown.activity,
            Some(ActivityResult {
                activity,
                resolution: ActivityResolution::Unknown
            })
        );
        assert_eq!(unknown.pack.relevant_episodes.len(), 3);
        // Rulings 46 and 69: each recency occasion also brings its observation's scene.
        assert_eq!(unknown.memory_scenes.len(), 6);
        let expansions = unknown.trace.unwrap().graph_expansions;
        assert_eq!(expansions.len(), 3);
        assert!(expansions
            .iter()
            .all(|root| root.source == GraphRootSource::Recency));
        assert_eq!(
            expansions
                .iter()
                .map(|root| root.root.id)
                .collect::<BTreeSet<_>>(),
            unknown
                .pack
                .relevant_episodes
                .iter()
                .map(|episode| episode.id)
                .collect()
        );
    }
    let mut no_room = context.clone();
    no_room.graph_limits.max_nodes = 0;
    let bounded = memory.retrieve(no_room).await.unwrap();
    assert_eq!(bounded.activity, found.activity);
    assert!(bounded.memory_scenes.is_empty());
    let mut replacement = open_loop;
    replacement.id = Some(MemoryId::from_u128(9201));
    replacement.supersedes = vec![id];
    memory
        .remember(
            RememberInput::new("A revised plan for the repair.")
                .with_scene(scene())
                .with_derived_memory(replacement),
            RememberOptions::default(),
        )
        .await
        .unwrap();
    let superseded = memory.retrieve(context.clone()).await.unwrap();
    assert_eq!(superseded.activity, found.activity);
    assert_eq!(superseded.pack.relevant_episodes[0].id, episode_id);
    // Rulings 46 and 69: the four recency occasions also bring their observations.
    assert_eq!(superseded.memory_scenes.len(), 8);
    assert_eq!(
        selected_cues(
            &superseded,
            MemoryObjectRef::new(ObjectType::Episode, episode_id)
        ),
        &BTreeSet::from([CueKind::Recency])
    );
    let mut historical = context.clone();
    historical.lifecycle_policy.include_superseded = true;
    assert!(memory
        .retrieve(historical)
        .await
        .unwrap()
        .pack
        .relevant_episodes
        .iter()
        .any(|episode| episode.id == episode_id));
    memory
        .forget(ForgetMemoryDraft::suppress(
            LifecycleTargetRef::derived_memory(id),
            "The original plan was forgotten.",
        ))
        .await
        .unwrap();
    for (include_suppressed, include_superseded) in
        [(false, false), (true, false), (false, true), (true, true)]
    {
        let mut context = context.clone();
        context.lifecycle_policy.include_suppressed = include_suppressed;
        context.lifecycle_policy.include_superseded = include_superseded;
        let result = memory.retrieve(context).await.unwrap();
        assert_eq!(result.activity, found.activity);
        assert_eq!(
            selected_cues(
                &result,
                MemoryObjectRef::new(ObjectType::Episode, episode_id)
            )
            .contains(&CueKind::Activity),
            include_suppressed && include_superseded
        );
    }
    assert!(queries.lock().unwrap().is_empty());
    memory.close().await.unwrap();
}
