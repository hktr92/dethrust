# Codex run: 28 track visual spec

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
Parse the minimum original track/race visual metadata needed to load Maim Street. No gameplay/collision/peds/checkpoints/AI.

Trace the world/track load path in `world.c`, `loading.c`, race structures, and BRender resource loading.

Create a neutral `TrackVisualSpec`-style output containing only what actual Maim Street rendering needs, such as:
- track/race identifier;
- main actor file;
- PIX/MAT/DAT resource groups;
- visual transforms/scale;
- sky/fog/background metadata only if essential for recognizable output.

Do not parse unrelated gameplay sections. Use named semantic readers/skippers, not line numbers.

With original data, produce a complete Maim Street visual dependency list and verify all paths exist. A `dethrace-tools track inspect "Maim Street"` command is encouraged.

Suggested commit:
`feat(formats): parse track visual definitions`
