# Codex run: 41 — chase camera and recovery

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical Dethrace/BRender behavioral and reverse-engineering reference. Do not use personal forks as the specification.

Preserve Milestones 0 and 1: the original menu/FLIC tooling, Maim Street Damage Gallery, and static Maim Street viewer must keep working. Original Carmageddon assets and `.reference/` must never be committed.

Milestone 2 is the first vehicle-simulation milestone. Do not implement opponents, pedestrians, race progression, damage/crushing, powerups, Action Replay, or career flow.

Stay strictly within this run's scope. Do not implement later prompt files early.

At the end, run relevant fmt/tests/clippy, inspect diff/status, create one focused commit if source/docs changed, and report the commit hash plus validation performed.

## Goal

Make the driving scene practical for extended testing by adding a chase camera and safe recovery/reset behavior.

## Chase camera

Implement a simple M2 camera:
- behind/above player;
- smooth but responsive follow;
- sensible orientation through turns;
- presentation-only ownership;
- no attempt at the final Carmageddon camera system.

A trivial look-back is optional only if it fits existing input architecture cleanly.

## Recovery

Trace upstream safe-position/recovery behavior and implement an M2-safe equivalent:
- remember recent valid/upright positions;
- recovery restores a sensible pose;
- reset velocities appropriately;
- avoid respawning inside geometry;
- recover from flip/stuck/fall-invalid state.

Bind keyboard/gamepad recovery as a discrete game command rather than continuous `DriverInput` if that is the cleaner architecture.

## Debug telemetry

Provide toggleable compact diagnostics for speed, pose, linear/angular velocity, grounded wheels, steering/throttle/brake, fixed-step rate, and recovery/safe-position state.

Do not make debug UI mandatory in normal rendering.

## Validation

Drive through turns, spin/flip or force an invalid pose, recover, continue driving, and verify the chase camera remains usable.

## Commit

Suggested: `feat(game): add chase camera and recovery`
