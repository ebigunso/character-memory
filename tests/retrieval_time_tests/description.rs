use super::*;

#[tokio::test]
async fn description_stops_at_shared_interpretation() {
    let (memory, temp) = open().await;
    commit(&memory, description_fixture(false)).await;
    let mut context = query(false, false);
    context.scene.setting.words = Some("studio".to_owned());
    context.section_limits = room(2);
    context.candidate_limits.max_graph_roots = 1;
    let result = retrieve(&memory, context).await;

    memory.close().await.unwrap();
    temp.close().unwrap();
    assert_eq!(roots(&result), [900]);
    assert_eq!(episodes(&result), [900]);
    assert!(!result
        .trace
        .as_ref()
        .unwrap()
        .section_assignments
        .iter()
        .any(|row| row.object.id == id(700)));
    assert!(assignment(&result, 300).cue_kinds.contains(&CueKind::Place));
}

#[tokio::test]
async fn description_root_keeps_best_proximity() {
    let (memory, temp) = open().await;
    commit(&memory, description_fixture(true)).await;
    let mut context = query(false, true);
    context.scene.setting.words = Some("studio".to_owned());
    context.section_limits = room(2);
    context.candidate_limits.max_graph_roots = 2;
    let result = retrieve(&memory, context).await;

    memory.close().await.unwrap();
    temp.close().unwrap();
    assert_eq!(roots(&result), [7, 900]);
    assert_eq!(scores(&result, 300).graph_score, Some(1.0 / 3.0));
    assert_eq!(scores(&result, 900).graph_score, Some(1.0));
}
