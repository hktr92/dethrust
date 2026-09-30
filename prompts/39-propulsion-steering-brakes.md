# Codex run: 39 — propulsion steering and brakes

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical Dethrace/BRender behavioral and reverse-engineering reference. Do not use personal forks as the specification.

Preserve Milestones 0 and 1: the original menu/FLIC tooling, Maim Street Damage Gallery, and static Maim Street viewer must keep working. Original Carmageddon assets and `.reference/` must never be committed.

Milestone 2 is the first vehicle-simulation milestone. Do not implement opponents, pedestrians, race progression, damage/crushing, powerups, Action Replay, or career flow.

Stay strictly within this run's scope. Do not implement later prompt files early.

At the end, run relevant fmt/tests/clippy, inspect diff/status, create one focused commit if source/docs changed, and report the commit hash plus validation performed.

## Goal

Make the player car drive by implementing the M2 subset of drivetrain and tyre-force behavior required for recognizable Carmageddon handling.

Trace upstream mechanics including `ControlCar*`, `CalcForce`, `DoRevs`, steering/curvature, driven vs non-driven wheels, braking/handbrake/reverse, tyre grip/slip, engine/revs/gearing values.

Preserve behavior, not C function boundaries.

## Requirements

Using neutral mechanics config and wheel contacts, implement:
- throttle propulsion;
- reverse;
- steering;
- braking;
- handbrake;
- driven-wheel behavior;
- basic longitudinal/lateral tyre forces;
- drivetrain/revs only to the extent required by M2;
- grip/slip behavior sufficient to turn, slide, brake, and recover plausibly.

This must not be a generic `move transform forward` controller.

All force/integration work runs in the fixed simulation step. Render/update systems must not add vehicle motion.

## Acceptance

On Maim Street the player can pull away, steer both directions, coast, brake, reverse, handbrake, and transition between grip/slide without numerical failure. Keyboard and gamepad feed the same `DriverInput` path.

## Tests

Cover zero-throttle, forward/reverse signs, steering symmetry, braking opposing motion, handbrake effect, and finite state under aggressive input.

## Commit

Suggested: `feat(game): add vehicle propulsion and steering`
