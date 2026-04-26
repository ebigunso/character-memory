# Common Repository Rules

last_updated: 2026-04-27

## Repository Reference Documents

- None yet.

## Repository-Specific Validation Commands

- `cargo fmt --check` validates Rust formatting.
- `cargo check` validates the crate compiles.
- `cargo test --no-run` validates test targets compile without requiring services to execute tests.

## Repo Safety / Boundaries

- None yet.

## Repo Naming / Structure

- The Rust package/crate name is `character_memory`.
- The primary public memory type is `CharacterMemory`.
- Prefer direct Rust module filenames such as `foo.rs` over `foo/mod.rs` for source modules.
- Reserve `tests/` for integration tests; place unit tests in the same source module tree as the production code they test.

## Global Migration Candidates (Placeholder)

- None yet.
