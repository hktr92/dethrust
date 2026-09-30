# Codex run: 21 car visual spec

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
Parse only the `CARS/*.TXT` subset required to construct a visually correct Damage Gallery car.

Use upstream `LoadCar` as the data-layout reference but do NOT port it wholesale.

Create a neutral `CarVisualSpec`-style output containing enough information to resolve:
- visual pixelmaps/textures;
- materials;
- models;
- actor variants;
- principal car actor/model.

Trace resolution-independent vs resolution-dependent car files. Skip cockpit, mechanics, HUD, damage simulation, powerups, etc. unless a field is required to locate visual assets.

Do not use brittle absolute line numbers. Use explicit semantic section readers/skippers.

Add synthetic tests and opt-in tests for the canonical starting player car plus several Maim Street opponents. Verify resolved visual files exist.

Suggested commit:
`feat(formats): parse car visual definitions`
