# Codex run: 15 — Milestone 0 validation

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Review and harden the complete Milestone 0 implementation.

Do not expand into race loading.

## Success criterion

From a clean checkout with a user-owned untouched Carmageddon installation:

```bash
./scripts/bootstrap-reference.sh
cargo run -p dethrace-app -- --game-dir /path/to/CARMA
```

must reach the original fresh-boot main menu and allow interaction with the top-level choices.

`dethrace-tools` FLIC inspection/extraction must still work independently of Bevy.

## Audit

Check:
- workspace dependency direction;
- no Bevy dependency in `dethrace-formats`;
- no original game assets tracked;
- no dependency on `.reference` at runtime;
- clean errors for bad/missing game directory;
- no unchecked binary parsing;
- no accidental `unsafe`;
- deterministic menu asset lookup;
- correct logical scaling;
- no zero-period animation crashes;
- keyboard/mouse/gamepad input paths;
- clean app exit;
- no race/physics scope creep.

Compare the menu visually and behaviorally against upstream Dethrace using the same original assets.

Fix discrepancies that are inside M0 scope.

## Documentation

Update the root README with:
- setup;
- reference bootstrap;
- game-dir invocation;
- original-asset test command;
- FLIC inspect/extract examples;
- current M0 completion status.

## Checks

Run full formatting, tests, clippy, plus a manual main-menu boot with original assets.

## Commit

Suggested message if changes are required:

```text
fix: harden milestone zero
```

If validation requires no source changes, do not create an empty commit.
