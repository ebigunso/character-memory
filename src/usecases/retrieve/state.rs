use super::*;
use std::collections::BTreeMap;

// The rank is local to the scene-ordered notion scope, not inherited cue provenance.
pub(super) type StateScopes = HashMap<MemoryObjectRef, Vec<(usize, usize)>>;

pub(super) fn record_subject_state(
    scopes: &mut StateScopes,
    scope: usize,
    subject: MemoryId,
    expansion: &GraphExpansion,
) {
    let direct = expansion
        .relations
        .iter()
        .filter(|relation| relation.relation == RelationType::About && relation.proximity == 1)
        .filter_map(|relation| {
            if relation.from == MemoryObjectRef::new(ObjectType::Entity, subject) {
                Some(relation.to)
            } else if relation.to == MemoryObjectRef::new(ObjectType::Entity, subject) {
                Some(relation.from)
            } else {
                None
            }
        })
        .collect::<HashSet<_>>();
    let mut memories = expansion
        .objects
        .iter()
        .filter_map(|object| match object {
            MemoryObject::DerivedMemory(memory)
                if memory.entity_ids.contains(&subject)
                    && direct.contains(&object.object_ref()) =>
            {
                Some(memory)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    memories.sort_by(|left, right| {
        right
            .salience_score
            .total_cmp(&left.salience_score)
            .then_with(|| right.created_at.cmp(&left.created_at))
            .then_with(|| left.id.cmp(&right.id))
    });
    for (rank, memory) in memories.into_iter().enumerate() {
        scopes
            .entry(MemoryObjectRef::new(ObjectType::DerivedMemory, memory.id))
            .or_default()
            .push((scope, rank));
    }
}

pub(super) fn order_section_state(
    objects: &mut [RankedObject],
    section: ContextPackSection,
    scopes: &StateScopes,
) {
    let mut positions = Vec::new();
    let mut queues = BTreeMap::<usize, Vec<(usize, usize)>>::new();
    for (index, object) in objects.iter().enumerate() {
        if section_for_object(&object.object) != Some(section) {
            continue;
        }
        if let Some(ranks) = scopes.get(&object.object.object_ref()) {
            positions.push(index);
            for &(scope, rank) in ranks {
                queues.entry(scope).or_default().push((rank, index));
            }
        }
    }
    if positions.len() < 2 {
        return;
    }
    for queue in queues.values_mut() {
        queue.sort_unstable();
    }
    let mut counts = HashMap::<usize, usize>::new();
    let mut cursors = HashMap::<usize, usize>::new();
    let mut chosen = HashSet::new();
    let mut ordered = Vec::new();
    for round in 1..=positions.len() {
        for (&scope, queue) in &queues {
            if counts.get(&scope).copied().unwrap_or_default() >= round {
                continue;
            }
            let cursor = cursors.entry(scope).or_default();
            while *cursor < queue.len() && chosen.contains(&queue[*cursor].1) {
                *cursor += 1;
            }
            if *cursor == queue.len() {
                continue;
            }
            let index = queue[*cursor].1;
            chosen.insert(index);
            ordered.push(objects[index].clone());
            for &(credited_scope, _) in &scopes[&objects[index].object.object_ref()] {
                *counts.entry(credited_scope).or_default() += 1;
            }
        }
        if ordered.len() == positions.len() {
            break;
        }
    }
    for (index, object) in positions.into_iter().zip(ordered) {
        objects[index] = object;
    }
}
