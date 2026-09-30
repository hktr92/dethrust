# Codex run: 19 brender models

Read `AGENTS.md` first. Use `.reference/dethrace` as the canonical upstream reference.

Stay strictly inside this prompt. Do not implement later prompts early. Preserve M0 behavior and these invariants:
- `dethrace-formats` stays Bevy-free;
- parsing is bounds-checked and safe;
- original Carmageddon assets and `.reference/` are never committed;
- source formats parse into neutral Rust types;
- Bevy conversion stays outside format parsing;
- do not mechanically port Dethrace C architecture.

At the end run relevant fmt/tests/clippy, inspect diff/status, create one focused commit if source/docs changed, and report the commit hash plus checks.

## Goal
Implement the BRender DAT/model geometry subset required by gallery cars and Maim Street.

Inspect canonical BRender model loading and Dethrace usage in `LoadCar`, `LinkModelsToActor`, and gallery bounding-radius calculation.

Expose neutral data for:
- identifier;
- vertex positions;
- topology/faces;
- material assignment/grouping;
- texture coordinates;
- bounds/radius metadata where present;
- multiple models per file when used.

Do not mirror pointer-heavy C runtime structs. Do not assume topology semantics without verifying them from BRender.

Validate all indexes/references and arithmetic. Add synthetic geometry/error tests and opt-in tests for representative Maim Street car models plus at least one track model.

Suggested commit:
`feat(formats): parse BRender models`
