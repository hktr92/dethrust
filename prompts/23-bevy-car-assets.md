# Codex run: 23 bevy car assets

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
Bridge PIX/MAT/DAT/ACT + `CarVisualSpec` into Bevy-renderable assets/entities.

Implement in `dethrace-assets` / appropriate runtime layer:
- source/indexed texture -> Bevy image conversion;
- material conversion sufficient for target cars;
- model -> Bevy Mesh conversion while preserving material grouping;
- actor hierarchy -> Bevy entity hierarchy;
- centralized BRender -> Bevy coordinate/transform conversion;
- target visibility/render-style handling.

Do not reparse source files in Bevy-facing code.

Derive coordinate handedness/orientation from upstream + known assets, not guesses. Document and test the conversion.

Modern unlit/minimally lit material mapping is acceptable for M1 if texture/material assignment, transparency and two-sidedness are correct; document deferred shade-table fidelity.

Add a small dev path that spawns one original car in a simple scene as proof.

Suggested commit:
`feat(assets): render original car assets`
