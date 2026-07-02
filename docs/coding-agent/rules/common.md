# Common Repository Rules

last_updated: 2026-07-02

## Repository Reference Documents

- None yet.

## Repository-Specific Validation Commands

- `cargo fmt --check` validates Rust formatting.
- `cargo check` validates the crate compiles.
- `cargo clippy --all-targets -- -D warnings` validates lints with warnings denied, matching CI.
- `cargo test --no-run` validates test targets compile without requiring services to execute tests.

## Repo Safety / Boundaries

- None yet.

## Repo Naming / Structure

- The Rust package/crate name is `character_memory`.
- The primary public memory type is `CharacterMemory`.
- Prefer direct Rust module filenames such as `foo.rs` over `foo/mod.rs` for source modules.
- Reserve `tests/` for integration tests; place unit tests in the same source module tree as the production code they test.
- Keep roadmap version labels out of long-lived production code comments, identifiers, and user-facing errors. Use stable domain/schema language instead; roadmap version labels belong in roadmap/planning docs or clearly temporary migration artifacts with cleanup conditions.

## Global Migration Candidates (Placeholder)

- None yet.
