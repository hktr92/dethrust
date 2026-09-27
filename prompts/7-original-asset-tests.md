# Codex run: 7 — opt-in original asset compatibility tests

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Validate our FLIC parser/decoder against the user's untouched Carmageddon installation without committing copyrighted data.

## Requirements

Add an opt-in integration test suite for original assets.

Use `CARMAGEDDON_DIR`.

The normal workspace test suite must still pass when that environment variable is missing.

A good shape is either:
- ignored tests that are explicitly invoked; or
- tests that cleanly skip when the environment variable is not present.

Document the exact command.

At minimum, validate representative main-menu files discovered from the current upstream reference, including the fresh-boot main menu assets and several button/highlight FLICs.

Do not hardcode assumptions that contradict the local upstream checkout. Trace `gMain_flic_list` and `mainmenu.c`.

For every selected FLIC:
- parse the full container;
- decode every frame;
- ensure no decoder error;
- ensure dimensions/frame counts are sensible;
- ensure decoding ends exactly as expected.

Where useful, assert known metadata discovered from the actual files, but avoid brittle checks that add no compatibility value.

## Diagnostics

When a file fails, diagnostics must include:
- source path;
- frame index;
- chunk index/type when known;
- offset/error.

## Documentation

Add a short section to the root README explaining how to run original-asset tests.

## Commit

Suggested message:

```text
test(formats): validate original menu FLICs
```
