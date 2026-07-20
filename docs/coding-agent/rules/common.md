---
rule_schema_version: 2
suite_id: "rules-cm-20260719"
rule_file: "common"
last_updated: "2026-07-21"
---

# Common Repository Rules
## Repository Reference Documents

- Evaluation harnesses live in a separate public companion repository, `CharacterMemoryEvals` (checked out as a sibling directory of this repo; note the directory name is `CharacterMemoryEvals`, not `character-memory-evals`; made public 2026-07-19). It is a Rust workspace consuming this crate via a path dependency. Evaluation tooling exists there — do not assume it is unimplemented — but it is not part of this library's core functionality.

## Repo Documentation Wording

- Committed artifacts in this repository must not contain machine-local absolute paths (for example user-profile paths); refer to sibling repositories by name and relative relationship instead.
- When mentioning the `CharacterMemoryEvals` repository in committed docs, describe it as the public companion evaluation repository and state that evaluation tooling is a development aid, not core library functionality. Do not describe it as private or inaccessible (it was made public 2026-07-19); historical records (completed plans, dated ADR bodies) that reflect the earlier private status stay unchanged.
- Do not hard-wrap prose in committed documents: never insert line breaks mid-sentence to fit a column width. Write each sentence/paragraph/list item as one line and let editors soft-wrap. Structural line breaks (list items, headings, YAML keys, code) are fine.
- ADR frontmatter `consulted` entries record model names only (for example "Claude Fable 5", "GPT-5.5 Pro") — no role, platform, or product designations such as "(orchestrator)" or "Codex" (user-directed 2026-07-18).

## Repository-Specific Validation Commands

- `cargo fmt --check` validates Rust formatting.
- `cargo check` validates the crate compiles.
- `cargo clippy --all-targets -- -D warnings` validates lints with warnings denied, matching CI.
- `cargo test --no-run` validates test targets compile without requiring services to execute tests.

## Repo Safety / Boundaries

- None yet.

## Compatibility Policy

- Until the library has external consumers, backwards compatibility is not a goal: changes replace old surfaces outright and only the latest supported surfaces remain (user-directed 2026-07-21).
- Do not introduce compat shims, legacy aliases or re-exports kept so old paths resolve, serde tolerance for old field names or schema shapes, deprecated-but-retained APIs, or migration code for formats that never shipped; remove a superseded surface in the same change that replaces it.

## Repo Naming / Structure

- The Rust package/crate name is `character_memory`.
- The primary public memory type is `CharacterMemory`.
- Prefer direct Rust module filenames such as `foo.rs` over `foo/mod.rs` for source modules.
- Reserve `tests/` for integration tests; place unit tests in the same source module tree as the production code they test.
- Keep roadmap version labels out of long-lived production code comments, identifiers, and user-facing errors. Use stable domain/schema language instead; roadmap version labels belong in roadmap/planning docs or clearly temporary migration artifacts with cleanup conditions.

## Global Migration Candidates (Placeholder)

- None yet.
