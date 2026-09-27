# Codex run: 13 — main menu FLIC animation and composition

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Reproduce the fresh-boot main-menu opening/still animation behavior using the original FLICs.

## Reference

Trace:
- fresh-boot opening FLIC selection;
- frame timing;
- overlay composition behavior;
- palette changes;
- any transition from opening animation to stable menu state.

Use `mainmenu.c`, `flicplay.c`, and `intrface.c`.

## Requirements

Implement:
- time-based FLIC playback;
- correct frame progression;
- palette updates;
- transition to the stable menu state;
- overlay composition support needed by menu elements;
- scaling through the logical menu canvas.

Do not tie format parsing to wall-clock timing. Timing/playback belongs in the UI/Bevy layer.

Avoid reproducing upstream's divide-by-frame-period style timing hazards. Zero/invalid periods must already be rejected or safely handled before scheduling playback.

Do not implement full button input yet except what is strictly required to visually validate the animation.

## Validation

Compare visually against the current Dethrace reference using the same original asset directory.

## Commit

Suggested message:

```text
feat(ui): animate original main menu
```
