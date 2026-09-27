# Codex run: 14 — main menu navigation and button states

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical C reference. Do not use a personal fork as the specification.

Stay inside this run's scope. Do not implement later prompt files early.

At the end:
1. run the relevant formatter/tests/lints;
2. inspect `git diff` and `git status`;
3. commit all intended changes in one focused commit;
4. report the commit hash, checks run, and any remaining limitation.


## Goal

Make the fresh-boot main menu interactive.

## Reference

Trace the continue-not-allowed `tInterface_spec` in upstream `mainmenu.c`.

Determine from the reference:
- number of choices;
- initial choice;
- up/down wrapping/clamping semantics;
- mouse hit regions;
- hover/flicker-on FLICs;
- flicker-off FLICs;
- pushed/confirm FLICs;
- escape behavior;
- returned semantic menu choices.

Do not guess or redesign the visible behavior in M0.

## Input

Support:
- keyboard;
- mouse;
- gamepad navigation/confirm/back using Bevy input abstractions.

It is acceptable for exact controller bindings to be modernized, but visible menu selection semantics should match the original.

## Scope boundary

Only the top-level fresh-boot menu must work.

For selections whose destination screen is not implemented yet, transition to a typed placeholder state or display a clear temporary stub rather than implementing future screens.

Exit/Quit should actually terminate cleanly if that is part of the chosen top-level flow.

## Architecture

Keep menu definitions/data separate from raw input polling where practical.

Do not encode menu state as a collection of unrelated global Resources.

## Commit

Suggested message:

```text
feat(ui): add main menu navigation
```
