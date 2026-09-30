# Codex run: 42 — Carmageddon handling comparison pass

Read `AGENTS.md` completely before making changes.

Use `.reference/dethrace` as the canonical Dethrace/BRender behavioral and reverse-engineering reference. Do not use personal forks as the specification.

Preserve Milestones 0 and 1: the original menu/FLIC tooling, Maim Street Damage Gallery, and static Maim Street viewer must keep working. Original Carmageddon assets and `.reference/` must never be committed.

Milestone 2 is the first vehicle-simulation milestone. Do not implement opponents, pedestrians, race progression, damage/crushing, powerups, Action Replay, or career flow.

Stay strictly within this run's scope. Do not implement later prompt files early.

At the end, run relevant fmt/tests/clippy, inspect diff/status, create one focused commit if source/docs changed, and report the commit hash plus validation performed.

## Goal

Tune and correct M2 vehicle behavior against canonical Dethrace/original Carmageddon. This is not a generic "make it feel nice" pass.

## Comparison method

Use the same starting car, Maim Street, original mechanics data, and comparable maneuvers.

Compare:
- standing acceleration;
- braking response/distance;
- reverse;
- low/high-speed steering;
- handbrake turn;
- body pitch/roll;
- suspension response;
- grip/sliding transition;
- ordinary-speed collision response;
- recovery semantics.

Where practical collect simple numeric traces: speed over time, steering vs yaw response, stopping time/distance, suspension compression, fixed-step stability.

## Rules

Do not hardcode Maim-Street-specific magic values to make one scene look right.

Prefer original mechanics data and justified global simulation constants.

Do not chase bit-exact equivalence when legacy numerical behavior is unstable or architecture-specific. Document intentional approximations.

Fix clear coordinate/unit/sign/force/grip/timestep mistakes found during comparison.

## Documentation

Create/update `docs/m2-vehicle-simulation.md` describing timestep, mechanics conversion, contact/suspension model, tyre/propulsion model, collision model, and known differences from Dethrace.

## Commit

Suggested: `fix(game): tune player vehicle handling`
