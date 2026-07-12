# Common Repository Rules

last_updated: 2026-07-11

## Repository Reference Documents

- Evaluation harnesses live in a separate private companion repository, `CharacterMemoryEvals` (checked out as a sibling directory of this repo; note the directory name is `CharacterMemoryEvals`, not `character-memory-evals`). It is a Rust workspace consuming this crate via a path dependency. Evaluation tooling exists there — do not assume it is unimplemented — but it is not part of this library's core functionality and the repo is not publicly accessible.

## Repo Documentation Wording

- Committed artifacts in this repository must not contain machine-local absolute paths (for example user-profile paths); refer to sibling repositories by name and relative relationship instead.
- When mentioning the private `CharacterMemoryEvals` repository in committed docs, word it so public readers are not confused by being unable to access it: state that it is private and that evaluation tooling is a development aid, not core library functionality.
- Do not hard-wrap prose in committed documents: never insert line breaks mid-sentence to fit a column width. Write each sentence/paragraph/list item as one line and let editors soft-wrap. Structural line breaks (list items, headings, YAML keys, code) are fine.

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
