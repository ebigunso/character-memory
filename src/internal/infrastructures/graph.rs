mod oxigraph_authority_store;
mod rdf_mapping;
mod vocabulary;

// Transitional v0.1 graph adapter surface: downstream pipeline chunks will
// consume the concrete store and mapping helpers after bounded expansion lands.
#[allow(unused_imports)]
pub(crate) use oxigraph_authority_store::OxigraphGraphAuthorityStore;
#[allow(unused_imports)]
pub(crate) use rdf_mapping::{rdf_triples_for_link, rdf_triples_for_object, RdfObject, RdfTriple};
