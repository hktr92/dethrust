# Codex run: 26 damage gallery interaction

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
Implement the useful original gallery interaction to validate transforms thoroughly.

Trace `DamageScrnLeft/Right/Up/Down`, `DamageScrnGoHead`, `ZoomInTo`, `ZoomOutTo`, `ClickDamage`, rolling-ball mouse behavior, and exit handling.

Implement:
- grid selection;
- selected car label;
- keyboard + gamepad navigation;
- mouse selection if clean with existing infrastructure;
- zoom in/out with an approximately one-second stable interpolation;
- manual rotation while zoomed in using an idiomatic modern equivalent;
- Back/Done clean exit.

Do not port BRender scene-picking APIs. Use Bevy picking/raycasting or explicit modern logic.

No damage deformation or race state yet.

Verify re-entry is clean and manual rotation does not cause permanent transform/scale corruption.

Suggested commit:
`feat(ui): add damage gallery interaction`
