# Codex run: 37 — player input to DriverInput

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical Dethrace/BRender behavioral and reverse-engineering reference. Do not use personal forks as the specification.

Preserve Milestones 0 and 1: the original menu/FLIC tooling, Maim Street Damage Gallery, and static Maim Street viewer must keep working. Original Carmageddon assets and `.reference/` must never be committed.

Milestone 2 is the first vehicle-simulation milestone. Do not implement opponents, pedestrians, race progression, damage/crushing, powerups, Action Replay, or career flow.

Stay strictly within this run's scope. Do not implement later prompt files early.

At the end, run relevant fmt/tests/clippy, inspect diff/status, create one focused commit if source/docs changed, and report the commit hash plus validation performed.

## Goal

Map keyboard and physical gamepad controls into neutral `DriverInput`. Input systems must not directly move/rotate entities.

## Keyboard

Support steering, throttle, brake/reverse semantics appropriate to the chosen vehicle model, and handbrake. Recovery/reset can remain a discrete command for prompt 41.

## Gamepad

Using Bevy input abstractions and existing M0 gamepad conventions where useful, support a normal Xbox-style controller with analog steering, appropriate analog/digital throttle and brake, handbrake, and sensible deadzones.

Normalize/clamp input before simulation, e.g. steering `[-1,1]`, throttle/brake `[0,1]` where appropriate.

Do not encode force/torque values in the input layer.

Add optional quiet telemetry for current `DriverInput`, not noisy per-frame logs by default.

## Tests

Test opposing digital inputs, analog deadzone, clamping, release-to-neutral, and handbrake mapping. Manually validate a physical gamepad if available.

## Commit

Suggested: `feat(game): map player controls to driver input`
