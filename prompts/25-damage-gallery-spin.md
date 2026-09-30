# Codex run: 25 damage gallery spin

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
Implement gallery auto-rotation equivalent to upstream `SpinWrecks`.

Requirements:
- frame-rate-independent upstream-equivalent angular speed;
- preserve translation and normalized scale;
- no matrix-normalization hacks or transform drift;
- explicit component/state for auto-spin;
- leave room for a car to become manually/custom rotated later.

Unit-test angular integration where practical. Manually verify no scale/translation drift across frame rates.

Suggested commit:
`feat(ui): spin damage gallery cars`
