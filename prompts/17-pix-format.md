# Codex run: 17 pix format

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
Implement the BRender PIX/pixelmap subset required by Maim Street cars and track in `dethrace-formats`.

Find and use the canonical BRender pixelmap loader/chunk definitions. Cross-check Dethrace `LoadPixelmap` / `LoadSomePixelmaps`.

Support whatever the real target assets require, including as applicable:
- identifiers, dimensions, pixel type, row metadata, pixel payload;
- indexed/palette semantics;
- multiple pixelmaps per container.

Preserve indexed source data; do not eagerly convert to RGBA in formats.

Validate chunk bounds, dimensions, stride/buffer arithmetic, payload lengths, and supported pixel formats. Unknown skippable chunks may be skipped intentionally; unknown required chunks are errors.

Add synthetic tests plus opt-in `$CARMAGEDDON_DIR` tests against representative Maim Street car/track PIX assets.

A small `dethrace-tools pix inspect PATH` command is allowed if useful.

Suggested commit:
`feat(formats): parse BRender PIX assets`
