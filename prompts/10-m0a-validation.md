# Codex run: 10 — M0a formats validation and cleanup

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Treat this as a review/hardening run, not a feature expansion.

M0a should now independently parse, decode, inspect, and extract the menu FLIC assets without Bevy.

## Review

Audit all M0a code for:
- unchecked indexing;
- arithmetic overflow;
- accidental panics on malformed bytes;
- format/parser code depending on Bevy;
- duplicated parsing between tools and formats;
- unclear error context;
- incorrect chunk-boundary handling;
- tests that rely on original assets in the default test path;
- unnecessary abstractions/dependencies.

Compare the decoder behavior again with current upstream `flicplay.c`.

Fix any issues found.

## Required proof

With `CARMAGEDDON_DIR` set, successfully run the documented original-asset compatibility tests.

Successfully run commands equivalent to:

```bash
cargo run -p dethrace-tools -- \
  flic inspect "$CARMAGEDDON_DIR/ANIM/MAINSTIL.FLI"

cargo run -p dethrace-tools -- \
  flic extract "$CARMAGEDDON_DIR/ANIM/MAINSTIL.FLI" \
  --frame last \
  --output out/main-menu.png
```

If `MAINSTIL.FLI` is not the fresh-boot still in this upstream/game variant, also extract the correct fresh-boot FLIC discovered from `mainmenu.c`.

Do not commit generated output.

## Acceptance

All normal workspace checks pass.

M0a is considered complete only when the FLIC path is reliable enough that M0b does not need to reach into parser internals.

## Commit

Use a focused message describing actual cleanup, for example:

```text
fix(formats): harden FLIC decoding
```

If no source changes are required after validation, do not create an empty commit. Report that the run produced no code changes.
