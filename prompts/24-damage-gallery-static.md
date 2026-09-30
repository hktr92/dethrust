# Codex run: 24 damage gallery static

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
Render the original end-race Damage/Wrecks Gallery layout populated with the Maim Street visual car set. Cars may be static in this run.

Use `BuildWrecks`, `DamageScrnDraw`, and `DoEndRaceSummary2` as the oracle, including the interface FLICs.

Add a deterministic direct route, preferably equivalent to:
`cargo run -p dethrace-app -- --game-dir "$CARMAGEDDON_DIR" --debug-scene damage-gallery-maim-street`

Render:
- original gallery UI/background;
- correct logical 3D render rectangle;
- player + Maim Street opponent cars resolved from data;
- upstream 3-column positioning;
- upstream bounding-radius normalization;
- equivalent camera framing;
- initial selected-car label;
- Back/Done visuals.

No spin, zoom, damage deformation, or race flow yet.

Compare against canonical Dethrace using the same assets.

Suggested commit:
`feat(ui): render Maim Street damage gallery`
