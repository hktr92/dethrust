# Codex run: 20 brender actors

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
Implement the BRender ACT hierarchy subset required by car and track visuals.

Inspect canonical `BrActorLoad` behavior, Dethrace `LoadCar`/`LinkModelsToActor`, and track actor loading.

Represent:
- actor identifier/type;
- local transform;
- parent/child hierarchy;
- model reference;
- actor material override if used;
- visibility/render style needed by M1.

Provide explicit linking from actor references to parsed model/material resources. Missing required references must produce useful errors.

Do not use global mutable registries.

Add synthetic hierarchy/transform/linking tests plus opt-in tests for representative car ACT files and the Maim Street track actor file.

Suggested commit:
`feat(formats): parse BRender actor hierarchies`
