// Retrieval/expansion policies: provider-neutral algorithms that implement
// core guarantees (e.g. ADR-I-0006 bounded graph expansion) independently of
// any storage backend.
pub(crate) mod embedding_surface;
pub(crate) mod graph_expansion;
pub(crate) mod retrieval_selectivity;

#[cfg(test)]
pub(crate) use embedding_surface::episode_vector_record;
pub(crate) use embedding_surface::memory_object_vector_record;
pub(crate) use retrieval_selectivity::{
    selectivity_plan_for_candidate, RetrievalSelectivityPolicy, SelectivityPlan,
    SelectivityStatsContext,
};
