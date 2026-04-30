# Database Design Notes

These documents explain the storage-schema rationale behind the graph-authoritative architecture.

- [Database Schema Cheat Sheet](schema_cheat_sheet.md): compact reference for Qdrant payload fields, Oxigraph classes, graph predicates, and cross-store authority.
- [Vector Database Payload Design](vector_payload_design.md): why Qdrant stores candidate-recall payload hints rather than authoritative memory state.
- [Graph Database Schema Design](graph_schema_design.md): why Oxigraph/RDF stores canonical objects, typed links, provenance, lifecycle state, and bounded expansion context.
