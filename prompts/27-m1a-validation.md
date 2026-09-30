# Codex run: 27 m1a validation

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
Review and harden M1a before track loading. Do not expand scope.

Required proof with original assets:
- direct Maim Street gallery boot;
- intended player + data-resolved Maim Street opponents;
- correct textures/material assignments;
- correct actor/model transforms and normalized scale;
- spin;
- selection;
- zoom/manual rotation;
- clean exit.

Audit for:
- Bevy dependencies leaking into formats;
- hardcoded car/opponent lists;
- scattered coordinate hacks;
- duplicated linking;
- unchecked parser operations;
- original assets accidentally tracked;
- Bevy code reparsing formats;
- scene re-entry leaks/duplicate entities.

Run full fmt/test/clippy, original-asset tests, useful tools, and manual gallery boot. Update README with the direct gallery command and note cars are not physically damaged yet.

If changes are needed:
`fix: harden Maim Street damage gallery`
Do not create an empty commit.
