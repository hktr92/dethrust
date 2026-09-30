# Codex run: 18 brender materials

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
Implement the BRender MAT subset needed to display Maim Street cars and track.

Inspect canonical BRender material loading plus Dethrace material helpers.

Parse the target-asset fields required for visible rendering, such as:
- identifier;
- colour/base properties;
- texture/pixelmap reference;
- render flags;
- two-sidedness;
- transparency/blending state;
- UV/mapping state if used;
- prelit/smooth/light flags useful to the later adapter.

Keep source semantics neutral. Do not create Bevy materials here and do not reproduce BRender global registries.

Shade-table lighting and legacy quirks may be deferred if unnecessary for recognizable M1 rendering, but retain/document metadata needed later.

Add synthetic malformed/truncation tests and opt-in tests for representative Maim Street car/track MAT assets.

Suggested commit:
`feat(formats): parse BRender materials`
