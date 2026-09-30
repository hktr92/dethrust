# Codex run: 43 — Milestone 2 validation

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical Dethrace/BRender behavioral and reverse-engineering reference. Do not use personal forks as the specification.

Preserve Milestones 0 and 1: the original menu/FLIC tooling, Maim Street Damage Gallery, and static Maim Street viewer must keep working. Original Carmageddon assets and `.reference/` must never be committed.

Milestone 2 is the first vehicle-simulation milestone. Do not implement opponents, pedestrians, race progression, damage/crushing, powerups, Action Replay, or career flow.

Stay strictly within this run's scope. Do not implement later prompt files early.

At the end, run relevant fmt/tests/clippy, inspect diff/status, create one focused commit if source/docs changed, and report the commit hash plus validation performed.

## Goal

Validate Milestone 2 end-to-end. Do not add M3+ features.

## Required scene

A clean run with original assets must support an equivalent of:

```bash
cargo run -p dethrace-app -- \
  --game-dir "$CARMAGEDDON_DIR" \
  --debug-scene maim-street-drive
```

It must:
- load Maim Street;
- load the original/canonical player car from original data;
- spawn at correct start transform;
- settle through wheel contact/suspension;
- accept keyboard input;
- accept physical gamepad input when available;
- accelerate, steer, brake, reverse, handbrake;
- collide with static environment;
- remain stable through an extended Maim Street drive;
- recover/reset and continue;
- provide a functional chase camera.

## Architecture audit

Confirm:
- player control only produces `DriverInput` plus discrete commands;
- input systems do not move transforms directly;
- simulation runs on a fixed timestep;
- simulation core is not coupled to rendering;
- track collision reuses M1 parsed world data;
- no opponent-specific physics exists;
- coordinate conversion remains centralized;
- no original assets are tracked;
- `dethrace-formats` remains Bevy-free;
- no damage/race/replay scope creep.

## Regression

Verify M0/M1: menu boots, FLIC tooling/tests pass, Damage Gallery works, static Maim Street viewer works.

## Checks

Run:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
CARMAGEDDON_DIR="$CARMAGEDDON_DIR" cargo test --workspace -- --ignored
```

Perform a manual drive long enough to expose obvious drift/instability. If a physical gamepad is available, test it and report explicitly.

## README

Update README with M2 status, direct drive command, controls, current simulation limitations, and explicit next milestone: **track semantics**, not opponents yet.

## Completion

M2 is complete when the player can genuinely drive one original car around Maim Street with stable suspension and static-world collision.

Recommend but do not automatically create/push tag `m2-player-vehicle`.

If final fixes are needed, suggested commit: `fix: complete milestone two validation`.

Do not create an empty commit.
