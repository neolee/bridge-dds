# Project Standards & Constraints

## Documentation

- All documentation must be concise, clear, and to the point.
- No emoji anywhere in project documentation or code comments.
- Tables are permitted only when they are the clearest way to present structured information; prefer prose otherwise.
- All technical terms, code identifiers, file paths, type names, function names, and command-line arguments must be wrapped in Markdown inline code markers (`` ` ``). This applies to `AGENTS.md`, `INIT.md`, `PLAN.md`, `README.md`, and any future design docs.
- Headings must use ATX style (`#`, `##`, ...), Setext not.

## Code

- [to be extended as the project evolves]

## Phase Workflow

Every phase begins with a detailed plan document under `phases/`, named after the phase identifier (e.g. `1a-full-deal-dds.md`). The document must include:

- **Goal**: a concise summary of what this phase delivers.
- **Tasks**: broken down to the level of concrete files, types, and function signatures. Include key design decisions and code sketches where they reduce ambiguity.
- **Verification**: how to confirm the phase is complete and correct. Distinguish between automated checks (unit tests, integration tests, compiler lints) and manual checks (CLI interaction, visual inspection) that the developer performs.
- **Reference**: links to relevant external documentation, DDS API docs, PBN spec sections, etc.

Work begins only after the plan document is reviewed and confirmed. After completion, the developer verifies and confirms before the next phase starts.
