# Codex run: 8 — dethrace-tools flic inspect

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Implement the first useful CLI in `dethrace-tools`:

```bash
cargo run -p dethrace-tools -- flic inspect PATH
```

This tool must use `dethrace-formats`; do not duplicate parsing logic.

## Output

Produce concise human-readable information including:
- path;
- FLI/FLC variant;
- dimensions;
- frame count;
- timing information;
- declared/actual size where useful;
- chunk type histogram;
- per-frame summary under an optional verbose flag if that fits cleanly.

Raw chunk codes should remain visible even if friendly names are provided.

Use a lightweight CLI dependency if useful. Do not add Bevy.

Errors should be readable and retain the detailed parser context.

## Validation

Run it against multiple menu FLICs under `$CARMAGEDDON_DIR/ANIM`.

Also test CLI argument handling without requiring original assets.

## Commit

Suggested message:

```text
feat(tools): add FLIC inspector
```
