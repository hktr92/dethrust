# Codex run: 12 — render the fresh-boot main menu

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Render the original **fresh-boot** Carmageddon main menu in Bevy using original assets.

No interaction or animated transitions yet.

## Reference behavior

Trace the continue-not-allowed path in:

```text
.reference/dethrace/src/DETHRACE/common/mainmenu.c
.reference/dethrace/src/DETHRACE/common/flicplay.c
.reference/dethrace/src/DETHRACE/common/intrface.c
```

Do not assume `MAINSTIL.FLI` is the exact fresh-boot menu for every path. Determine which opening/still FLICs upstream uses when Continue is unavailable.

Use original coordinates/aspect assumptions from upstream where they define layout.

## App behavior

Provide an executable invocation along the lines of:

```bash
cargo run -p dethrace-app -- --game-dir "$CARMAGEDDON_DIR"
```

For this run it should:
- validate the game directory;
- open a Bevy window;
- reach a `MainMenu` app/game state;
- show the correct static fresh-boot menu appearance using original FLIC data.

A final decoded still frame is acceptable in this run.

Do not implement race loading, physics, options screens, save/load, or menu navigation.

## Presentation

Preserve the original logical menu coordinate system and scale it cleanly to the window rather than hardcoding everything in physical pixels.

Avoid recreating the old 8-bit framebuffer architecture. Source assets may remain indexed internally, but Bevy presentation should use the modern renderer.

## Commit

Suggested message:

```text
feat(ui): render original main menu
```
