use super::*;
use std::collections::BTreeMap;

// Membership is local to a scene-ordered state scope, not inherited cue provenance.
pub(super) type StateScopes = HashMap<MemoryObjectRef, Vec<usize>>;

pub(super) fn scopes_for_kind(
    scopes: &StateScopes,
    kinds: &[CueKind],
    kind: CueKind,
) -> StateScopes {
    scopes
        .iter()
        .filter_map(|(&object, memberships)| {
            let memberships = memberships
                .iter()
                .copied()
                .filter(|&scope| kinds[scope] == kind)
                .collect::<Vec<_>>();
            (!memberships.is_empty()).then_some((object, memberships))
        })
        .collect()
}

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
    let memories = expansion.objects.iter().filter_map(|object| match object {
        MemoryObject::DerivedMemory(memory)
            if memory.entity_ids.contains(&subject) && direct.contains(&object.object_ref()) =>
        {
            Some(memory)
        }
        _ => None,
    });
    for memory in memories {
        scopes
            .entry(MemoryObjectRef::new(ObjectType::DerivedMemory, memory.id))
            .or_default()
            .push(scope);
    }
}

pub(super) fn order_section_state(
    objects: &mut [RankedObject],
    section: ContextPackSection,
    scopes: &StateScopes,
) {
    order_state(
        objects,
        scopes,
        |object| (section_for_object(object) == Some(section)).then(|| object.object.object_ref()),
        |_, _| 0,
    );
}

pub(super) fn order_state<T: Clone>(
    objects: &mut [T],
    scopes: &StateScopes,
    reference: impl Fn(&T) -> Option<MemoryObjectRef>,
    priority: impl Fn(usize, &T) -> usize,
) {
    let mut positions = Vec::new();
    let mut queues = BTreeMap::<usize, Vec<usize>>::new();
    for (index, object) in objects.iter().enumerate() {
        if let Some(memberships) = reference(object).and_then(|key| scopes.get(&key)) {
            positions.push(index);
            for &scope in memberships {
                queues.entry(scope).or_default().push(index);
            }
        }
    }
    for (&scope, queue) in &mut queues {
        queue.sort_by_key(|&index| priority(scope, &objects[index]));
    }
    if positions.len() < 2 {
        return;
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
            while *cursor < queue.len() && chosen.contains(&queue[*cursor]) {
                *cursor += 1;
            }
            if *cursor == queue.len() {
                continue;
            }
            let index = queue[*cursor];
            chosen.insert(index);
            ordered.push(objects[index].clone());
            for &credited_scope in
                &scopes[&reference(&objects[index]).expect("selected objects have a scope")]
            {
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
