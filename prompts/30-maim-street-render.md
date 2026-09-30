# Codex run: 30 maim street render

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
Render the static Maim Street environment in Bevy. This is a scene viewer, not a race.

Add a deterministic direct route equivalent to:
`cargo run -p dethrace-app -- --game-dir "$CARMAGEDDON_DIR" --debug-scene maim-street`

Requirements:
- spawn the track actor hierarchy;
- render correct geometry/material/texture linkage;
- use centralized source transforms;
- provide a deterministic useful starting camera;
- provide keyboard + mouse free-fly inspection;
- use a neutral fallback background if original sky systems are deferred.

Fidelity target: recognizably correct static Maim Street geometry and textures.

Do not block M1 on exact lighting, palette shade tables, animated materials, peds, destructibles, or race logic. But fix major geometry/material errors.

Optional wireframe/identifier/camera diagnostics are fine if simple.

Suggested commit:
`feat(game): render Maim Street scene`
