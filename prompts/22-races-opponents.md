# Codex run: 22 races opponents

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
Parse enough original race/opponent metadata to resolve the exact visual car set for Maim Street. Do not hardcode opponent car filenames.

Trace `LoadOpponents`, `LoadRaces`, race selection, and opponent-car loading.

Expose an API equivalent in intent to:
`find Maim Street -> resolve its opponent references -> resolve visual car definitions`.

Gallery composition should match upstream `BuildWrecks` semantics as far as possible: current/player car plus relevant loaded opponents.

If real player progression does not exist yet, use the canonical initial/basic player car from upstream/original data and make that choice explicit and replaceable.

With `$CARMAGEDDON_DIR`, validate Maim Street lookup, opponent references, and every resulting car path. A `dethrace-tools race inspect "Maim Street"` command is encouraged.

Suggested commit:
`feat(formats): resolve Maim Street visual cars`
