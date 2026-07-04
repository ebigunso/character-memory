// Retrieval/expansion policies: provider-neutral algorithms that implement
// core guarantees (e.g. ADR-I-0006 bounded graph expansion) independently of
// any storage backend.
pub(crate) mod embedding_surface;
pub(crate) mod graph_expansion;
pub(crate) mod retrieval_selectivity;

// Surface builders are intentionally available as a policy family; remove when callers import concrete modules.
#[allow(unused_imports)]
pub(crate) use embedding_surface::{
    derived_memory_vector_record, entity_vector_record, episode_vector_record,
    memory_object_vector_record, memory_thread_vector_record, observation_vector_record,
};
pub(crate) use retrieval_selectivity::{
    selectivity_plan_for_candidate, RetrievalSelectivityPolicy, SelectivityPlan,
    SelectivityStatsContext,
};
