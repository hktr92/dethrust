# Codex run: 29 maim street assets

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
Load the complete Maim Street visual asset graph into Bevy-ready assets using the same PIX/MAT/DAT/ACT infrastructure used for cars.

Resolve and convert:
- pixelmaps;
- materials;
- models;
- actor hierarchy;
- local transforms and links.

Do not create a second track-specific BRender parser.

Cache/reuse resources by stable source identity/path rather than pointer identity.

Defer gameplay systems and exact legacy lighting unless they block recognizable static rendering: collision, peds, opponents, checkpoints, groove/funk animation, destructibles, HUD, audio, physics, shade-table fidelity.

Add opt-in tests that fail with precise context for missing model/material/texture references.

Suggested commit:
`feat(assets): load Maim Street visual assets`
