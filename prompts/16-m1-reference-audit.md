# Codex run: 16 m1 reference audit

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
Map the exact upstream path for the end-race Damage/Wrecks Gallery, the Maim Street car set, visual-only car loading, and visual-only Maim Street loading. Do not implement parsers yet.

Inspect at minimum `racesumm.c/.h`, `loading.c/.h`, `world.c/.h`, `structur.c`, `opponent.c`, `grafdata.c/.h`, `dr_types.h`, and the BRender submodule.

Trace:
- `BuildWrecks`, `SpinWrecks`, `DamageScrnDraw`, `DoEndRaceSummary2`;
- `LoadCar`, `LoadOpponents`, `LoadRaces`, opponent-car selection/loading;
- world/track loading;
- BRender loaders for `.PIX`, `.MAT`, `.DAT`, `.ACT`.

Create `docs/m1-reference-audit.md` documenting:
- gallery FLIC/UI assets, render rectangle, camera, grid placement, bounding-radius scale, spin rate, selection/zoom/rolling-ball behavior, labels;
- how Maim Street is identified and how its opponents/car files are resolved;
- the minimal visual dependency chain from car TXT -> PIX/MAT/DAT/ACT -> principal actor;
- the minimal Maim Street track visual dependency chain;
- a source-format table listing canonical loader, M1 subset, and deferred features.

Do not hardcode a guessed Maim Street opponent list.

Suggested commit:
`docs: map milestone one asset pipeline`
